//! ==============================================================================
//! crates/pilot-analysis/tests/milestone56_layout_driven_codegen_and_gpu_backends.rs
//! Milestone 56 — Uncheatable Layout-Driven C11, LLVM, WASM, ASM & 8-GPU Gate
//!
//! Required `.pilot` Contracts (all tested simultaneously on randomized ASTs):
//!   Every emitter in `selfhost/codegen/*.pilot` MUST lower functions, structs,
//!   types, and expressions **directly from `PhysicsRegistry` (`RawAstArena` +
//!   `LayoutTable` + `source_slice`)** — ZERO pre-compiled C/LLVM/WAT/ASM/GPU
//!   strings are passed by the test! All scalar & composite types in tests use
//!   **randomized names** bound via `[Bits(width, reg_bank)]` / `[Bits(width, lanes, reg_bank)]`
//!   (`reg_bank`: `2 = gpr_signed`, `3 = gpr_unsigned`, `4 = simd_float`, `5 = simd_vector`,
//!   `6 = raw_ptr`), so hardcoding `"i64"`, `"f32"`, etc. is impossible!
//!
//!   `RawAstArena` Quick Reference (`crates/pilot-analysis/src/infer.rs`):
//!   - `StructDecl`: `kind_tag == 1`, `sym_id = name`, `[Bits(...)]` in `ast_attr_decl_*`,
//!     fields are children with `kind_tag == 21` and `raw_bits == field_type_sym`.
//!   - `FnDecl`: `kind_tag == 3`, `sym_id = name`, child `0` is body (`BlockExpr`),
//!     children `1..N` are `ParamDecl` (`kind_tag == 21`, `sym_id = param_name`,
//!     `raw_bits == param_type_sym`); return type appears after `->` in
//!     `source_slice(ast_span_start(fn_nid), ast_span_end(fn_nid))` (or `void` if no `->`).
//!   - `LetStmt`/`VarStmt`: `kind_tag == 4`, `orig_kind == 100|101`, `raw_bits == type_sym`, child `0` = init.
//!   - `AssignStmt`: `kind_tag == 20`, children `[lhs, rhs]`; `IndexExpr`: `orig_kind == 210`, children `[base, idx]`.
//!   - `IfExpr`: `kind_tag == 6`, children `[cond, then_blk]` or `[cond, then_blk, else_blk]`.
//!   - `WhileStmt`: `kind_tag == 7`, `orig_kind == 106`, children `[cond, body_blk]`.
//!   - `ReturnStmt`: `kind_tag == 8`, child `0` (if `child_count == 1`) = expr.
//!   - `BinaryExpr`: `kind_tag == 19`, `raw_bits = BinaryOp` (`1=+, 2=-, 3=*, 4=/, 6=%, 8===, 9=!=, 10=<, 11=<=, 12=>, 13=>=, 16=&, 17=|, 18=^, 19=<<, 20=>>`).
//!   - `CallExpr`: `kind_tag == 16`, child `0` = callee `IdentExpr`, children `1..` = args
//!     (or struct literal when child `0` is a `StructDecl` name and child `1` is `MapExpr` `orig_kind == 219`).
//!   - `MemberExpr`: `kind_tag == 17`, `sym_id = field_sym`, child `0` = receiver expr.
//!
//!   Required Entrypoints:
//!   1. `selfhost/codegen/{physics,c_types,c_header,c_emitter}.pilot`:
//!      - `pub fn emit_arena_c11() -> str` (standalone compilable C11 source for all structs & `pub fn`s in `RawAstArena`; compiled & executed via `clang -std=c11 -O2`!)
//!   2. `selfhost/codegen/llvm_emitter.pilot`:
//!      - `pub fn emit_arena_llvm_ir() -> str` (valid typed LLVM IR SSA module for all `pub fn`s in `RawAstArena`; compiled & executed via `clang -x ir -O2`!)
//!   3. `selfhost/codegen/wasm_emitter.pilot`:
//!      - `pub fn emit_arena_wasm_hex() -> str` (valid binary `.wasm` v1 module as lowercase hex exporting all `pub fn`s; instantiated & executed in `node` `WebAssembly.Instance`!)
//!      - `pub fn emit_arena_wat() -> str`
//!      - `pub fn emit_arena_wasm_mjs() -> str`
//!      - `pub fn emit_arena_wasm_dts() -> str`
//!   4. `selfhost/codegen/asm_emitter.pilot`:
//!      - `pub fn emit_arena_x86_64_asm() -> str` (System V AMD64 `.s` assembly for scalar & AVX/SSE SIMD functions; compiled & executed via `clang -x assembler`!)
//!   5. `selfhost/codegen/{gpu_c_dialects,gpu_shaders,gpu_llvm}.pilot`:
//!      - `pub fn emit_arena_gpu_kernel(backend: str, kernel_name: str) -> str`
//!        for `"cuda"`, `"hip"`, `"opencl"`, `"openmp"`, `"wgsl"`, `"spirv"`, `"nvptx"`, `"amdgpu"`.
//!        (`cuda`, `hip`, `opencl`, and `openmp` are compiled & executed with `clang++` against a CPU launch shim on 64 random floats!)
//! ==============================================================================

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_analysis::{flatten_ast, handle_engine_intrinsic, run_pilot_analyzer, PhysicsRegistry};
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const C_CODEGEN_GROUP: &[&str] = &[
    "selfhost/codegen/physics.pilot",
    "selfhost/codegen/c_types.pilot",
    "selfhost/codegen/c_header.pilot",
    "selfhost/codegen/c_emitter.pilot",
];

const GPU_CODEGEN_GROUP: &[&str] = &[
    "selfhost/codegen/physics.pilot",
    "selfhost/codegen/gpu_c_dialects.pilot",
    "selfhost/codegen/gpu_shaders.pilot",
    "selfhost/codegen/gpu_llvm.pilot",
];

const CODEGEN_BANNED_CHEATS: &[&str] = &[
    "ExprNode",
    "TransformFrame",
    "FlowState",
    "c[idx] = a[idx] + b[idx]",
];

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 56,
    test_file_rel: "crates/pilot-analysis/tests/milestone56_layout_driven_codegen_and_gpu_backends.rs",
    expected_test_count: 3,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/codegen/physics.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &["__engine"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: CODEGEN_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/codegen/c_types.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: CODEGEN_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/codegen/c_header.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: CODEGEN_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/codegen/c_emitter.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: CODEGEN_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/codegen/llvm_emitter.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["emit_arena_llvm_ir"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: CODEGEN_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/codegen/wasm_emitter.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "emit_arena_wasm_hex",
                "emit_arena_wat",
                "emit_arena_wasm_mjs",
                "emit_arena_wasm_dts",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: CODEGEN_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/codegen/asm_emitter.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["emit_arena_x86_64_asm"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: CODEGEN_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/codegen/gpu_c_dialects.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: CODEGEN_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/codegen/gpu_shaders.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: CODEGEN_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/codegen/gpu_llvm.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: CODEGEN_BANNED_CHEATS,
        },
    ],
    required_group_pub_fns: &[
        ("emit_arena_c11", C_CODEGEN_GROUP),
        ("emit_arena_gpu_kernel", GPU_CODEGEN_GROUP),
    ],
};

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5656_5656_CAFE_BABE);
        let s = nanos ^ ((std::process::id() as u64).rotate_left(23)) ^ salt;
        Self(if s == 0 { 0xFEED_FACE_1234_5678 } else { s })
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn range(&mut self, lo: i64, hi_inclusive: i64) -> i64 {
        lo + (self.next_u64() % ((hi_inclusive - lo + 1) as u64)) as i64
    }

    fn ident(&mut self, prefix: &str) -> String {
        let chars = b"abcdefghijklmnopqrstuvwxyz";
        let mut out = format!("{}_", prefix);
        for _ in 0..5 {
            out.push(chars[(self.next_u64() as usize) % chars.len()] as char);
        }
        out
    }

    fn pascal(&mut self, prefix: &str) -> String {
        let chars = b"abcdefghijklmnopqrstuvwxyz";
        let mut out = prefix.to_string();
        for _ in 0..5 {
            out.push(chars[(self.next_u64() as usize) % chars.len()] as char);
        }
        out
    }
}

fn ws_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read_ws(rel: &str) -> String {
    let p = ws_root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("Failed to read {}: {}", p.display(), e))
}

fn make_temp_dir(tag: &str, rng: &mut Rng) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "pilot_m56_{}_{}_{}",
        tag,
        std::process::id(),
        rng.next_u64()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn decode_hex(hex: &str) -> Vec<u8> {
    let t = hex.trim();
    assert!(
        t.len() >= 16 && t.len() % 2 == 0,
        "Expected valid hex binary output, got len={} (`{}`)",
        t.len(),
        &t[..t.len().min(80)]
    );
    let mut out = Vec::with_capacity(t.len() / 2);
    let nib = |b: u8| -> u8 {
        match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => panic!("Invalid hex char `{}`", b as char),
        }
    };
    let bs = t.as_bytes();
    let mut i = 0;
    while i < bs.len() {
        out.push((nib(bs[i]) << 4) | nib(bs[i + 1]));
        i += 2;
    }
    out
}

fn run_codegen_on_arena(target_program: &str, codegen_files: &[&str], call_expr: &str) -> String {
    let target_arena = parse(target_program);
    assert!(
        target_arena.diagnostics.is_empty(),
        "Target Pilot program failed to parse: {:?}\n{}",
        target_arena.diagnostics,
        target_program
    );
    let (flat, syms) = flatten_ast(&target_arena, target_arena.root);
    let mut registry = PhysicsRegistry::new(flat, syms);
    registry.source_text = target_program.to_string();
    run_pilot_analyzer(&mut registry, None).expect("analyzer.pilot must bind layouts");

    let mut script = String::new();
    for rel in codegen_files {
        script.push_str(&read_ws(rel));
        script.push('\n');
    }
    script.push_str(&format!("let __emitted = {};\n__emitted;\n", call_expr));

    let driver_arena = parse(&script);
    assert!(
        driver_arena.diagnostics.is_empty(),
        "Codegen Pilot modules {:?} failed to parse: {:?}",
        codegen_files,
        driver_arena.diagnostics
    );

    let reg_ptr = &mut registry as *mut PhysicsRegistry;
    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(move |op, args| {
        handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, args)
    }));
    vm.load_binary(BytecodeCompiler::compile(&driver_arena, driver_arena.root));
    vm.run().to_string_repr()
}

#[test]
fn test_56_1_layout_driven_c11_backend_compiled_and_executed_by_clang() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_56_1_layout_driven_c11_backend_compiled_and_executed_by_clang",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5601);
    let tmp = make_temp_dir("c11", &mut rng);

    // Static check: ban legacy benchmark cheats across all selfhost/codegen/*.pilot files
    for rel in [
        "selfhost/codegen/physics.pilot",
        "selfhost/codegen/c_types.pilot",
        "selfhost/codegen/c_header.pilot",
        "selfhost/codegen/c_emitter.pilot",
    ] {
        let src = read_ws(rel);
        for banned in ["ExprNode", "TransformFrame", "FlowState"] {
            assert!(!src.contains(banned), "{} must not contain benchmark cheat {}", rel, banned);
        }
    }

    for trial in 0..5 {
        // Randomized type names (`s64_*`, `Pair_*`) — proves layout-driven C11 emission!
        let ty_s64 = rng.ident("s64");
        let st_pair = rng.pascal("Pair");
        let fn_step = rng.ident("step_rec");
        let fn_entry = rng.ident("eval_pair_kernel");
        let k_mul = rng.range(2, 6);
        let k_bias = rng.range(5, 40);
        let arg_a = rng.range(4, 15);
        let arg_b = rng.range(3, 12);

        let pilot_prog = format!(
            "[Bits(64, 2)]\npub struct {ty_s64} {{}}\n\
             pub struct {st_pair} {{\n\
                 pub left: {ty_s64};\n\
                 pub right: {ty_s64};\n\
             }}\n\
             pub fn {fn_step}(n: {ty_s64}, acc: {ty_s64}) -> {ty_s64} {{\n\
                 if n <= 0 {{\n\
                     return acc;\n\
                 }}\n\
                 return {fn_step}(n - 1, acc * {k_mul} + n);\n\
             }}\n\
             pub fn {fn_entry}(a: {ty_s64}, b: {ty_s64}) -> {ty_s64} {{\n\
                 let p: {st_pair} = {st_pair} {{ left: a + {k_bias}, right: b * {k_mul} }};\n\
                 var sum: {ty_s64} = p.left - p.right;\n\
                 var idx: {ty_s64} = 0;\n\
                 while idx < b {{\n\
                     sum = sum + (idx ^ {k_bias});\n\
                     idx = idx + 1;\n\
                 }}\n\
                 return sum + {fn_step}(4, a);\n\
             }}\n"
        );

        let c_code = run_codegen_on_arena(
            &pilot_prog,
            &[
                "selfhost/codegen/physics.pilot",
                "selfhost/codegen/c_types.pilot",
                "selfhost/codegen/c_header.pilot",
                "selfhost/codegen/c_emitter.pilot",
            ],
            "emit_arena_c11()",
        );

        let c_path = tmp.join(format!("mod_{}.c", trial));
        let bin_path = tmp.join(format!("mod_{}.bin", trial));
        let full_c = format!(
            "{}\n#include <stdio.h>\nint main(void) {{ printf(\"%lld\\n\", (long long){}({}, {})); return 0; }}\n",
            c_code, fn_entry, arg_a, arg_b
        );
        std::fs::write(&c_path, &full_c).unwrap();

        let compile = Command::new("clang")
            .args(["-std=c11", "-O2", c_path.to_str().unwrap(), "-o", bin_path.to_str().unwrap()])
            .output()
            .expect("invoke clang on emitted C11");
        assert!(
            compile.status.success(),
            "Trial {}: clang failed on emitted C11:\nStderr: {}\nEmitted C11:\n{}",
            trial,
            String::from_utf8_lossy(&compile.stderr),
            full_c
        );

        let run_out = Command::new(&bin_path).output().expect("execute C11 binary");
        assert!(run_out.status.success());
        let actual: i64 = String::from_utf8_lossy(&run_out.stdout)
            .trim()
            .parse()
            .expect("parse C11 stdout int");

        // Reference oracle
        let mut rec = arg_a;
        for n in (1..=4).rev() {
            rec = rec * k_mul + n;
        }
        let mut expected = (arg_a + k_bias) - (arg_b * k_mul);
        for idx in 0..arg_b {
            expected += idx ^ k_bias;
        }
        expected += rec;

        assert_eq!(actual, expected, "Trial {}: C11 execution mismatch!", trial);
    }
    let _ = std::fs::remove_dir_all(&tmp);
    guard.finish();
}

#[test]
fn test_56_3_llvm_ir_wasm_binary_and_x86_64_asm_executed_by_clang_and_node() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_56_3_llvm_ir_wasm_binary_and_x86_64_asm_executed_by_clang_and_node",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5603);
    let tmp = make_temp_dir("llvm_wasm_asm", &mut rng);

    for trial in 0..4 {
        let ty_s64 = rng.ident("intw");
        let fn_calc = rng.ident("poly_loop");
        let c1 = rng.range(3, 9);
        let c2 = rng.range(11, 37);
        let in_x = rng.range(5, 14);
        let in_y = rng.range(4, 9);

        let pilot_prog = format!(
            "[Bits(64, 2)]\npub struct {ty_s64} {{}}\n\
             pub fn {fn_calc}(x: {ty_s64}, y: {ty_s64}) -> {ty_s64} {{\n\
                 var acc: {ty_s64} = x * {c1};\n\
                 var i: {ty_s64} = 0;\n\
                 while i < y {{\n\
                     if (i & 1) == 0 {{\n\
                         acc = acc + i + {c2};\n\
                     }} else {{\n\
                         acc = acc - i;\n\
                     }}\n\
                     i = i + 1;\n\
                 }}\n\
                 return acc;\n\
             }}\n"
        );

        let mut expected = in_x * c1;
        for i in 0..in_y {
            if (i & 1) == 0 {
                expected += i + c2;
            } else {
                expected -= i;
            }
        }

        // 1. LLVM IR SSA compiled & executed natively via `clang`!
        let llvm_ir = run_codegen_on_arena(
            &pilot_prog,
            &["selfhost/codegen/physics.pilot", "selfhost/codegen/llvm_emitter.pilot"],
            "emit_arena_llvm_ir()",
        );
        let ll_path = tmp.join(format!("t_{}.ll", trial));
        let ll_drv = tmp.join(format!("drv_ll_{}.c", trial));
        let ll_bin = tmp.join(format!("ll_{}.bin", trial));
        std::fs::write(&ll_path, &llvm_ir).unwrap();
        std::fs::write(
            &ll_drv,
            format!(
                "#include <stdio.h>\n#include <stdint.h>\nextern int64_t {}(int64_t, int64_t);\nint main(void) {{ printf(\"%lld\\n\", (long long){}({}, {})); return 0; }}\n",
                fn_calc, fn_calc, in_x, in_y
            ),
        )
        .unwrap();
        let ll_comp = Command::new("clang")
            .args(["-O2", ll_path.to_str().unwrap(), ll_drv.to_str().unwrap(), "-o", ll_bin.to_str().unwrap()])
            .output()
            .expect("clang compile LLVM IR");
        assert!(
            ll_comp.status.success(),
            "Trial {}: clang rejected emitted LLVM IR:\nStderr: {}\nLLVM IR:\n{}",
            trial,
            String::from_utf8_lossy(&ll_comp.stderr),
            llvm_ir
        );
        let ll_run = Command::new(&ll_bin).output().unwrap();
        let ll_val: i64 = String::from_utf8_lossy(&ll_run.stdout).trim().parse().unwrap();
        assert_eq!(ll_val, expected, "Trial {}: LLVM IR execution mismatch!", trial);

        // 2. Raw binary `.wasm` + `.mjs`/`.d.ts` instantiated & executed in `node` V8 WebAssembly!
        let wasm_out = run_codegen_on_arena(
            &pilot_prog,
            &["selfhost/codegen/physics.pilot", "selfhost/codegen/wasm_emitter.pilot"],
            "emit_arena_wasm_hex() + \"<<<W>>>\" + emit_arena_wat() + \"<<<W>>>\" + emit_arena_wasm_mjs() + \"<<<W>>>\" + emit_arena_wasm_dts()",
        );
        let wparts: Vec<&str> = wasm_out.split("<<<W>>>").collect();
        assert_eq!(wparts.len(), 4);
        let wasm_bytes = decode_hex(wparts[0]);
        assert_eq!(&wasm_bytes[0..8], b"\0asm\x01\x00\x00\x00", "Invalid WASM magic/version");
        assert!(wparts[1].contains(&fn_calc), "WAT must export `{}`", fn_calc);
        assert!(wparts[3].contains(&fn_calc), ".d.ts must declare `{}`", fn_calc);

        let wasm_path = tmp.join(format!("t_{}.wasm", trial));
        std::fs::write(&wasm_path, &wasm_bytes).unwrap();
        let node_eval = Command::new("node")
            .args([
                "-e",
                &format!(
                    "const fs = require('fs'); const buf = fs.readFileSync({:?}); \
                     const mod = new WebAssembly.Module(buf); \
                     const inst = new WebAssembly.Instance(mod, {{}}); \
                     console.log(inst.exports.{}(BigInt({}), BigInt({})).toString());",
                    wasm_path.to_str().unwrap(),
                    fn_calc,
                    in_x,
                    in_y
                ),
            ])
            .output()
            .expect("node WebAssembly execution");
        assert!(
            node_eval.status.success(),
            "Trial {}: Node WebAssembly rejected emitted .wasm binary: {}",
            trial,
            String::from_utf8_lossy(&node_eval.stderr)
        );
        let wasm_val: i64 = String::from_utf8_lossy(&node_eval.stdout).trim().parse().unwrap();
        assert_eq!(wasm_val, expected, "Trial {}: WASM binary execution mismatch!", trial);

        // 3. x86_64 Native Assembly compiled & executed via `clang -x assembler`!
        let asm_text = run_codegen_on_arena(
            &pilot_prog,
            &["selfhost/codegen/physics.pilot", "selfhost/codegen/asm_emitter.pilot"],
            "emit_arena_x86_64_asm()",
        );
        let s_path = tmp.join(format!("t_{}.s", trial));
        let s_bin = tmp.join(format!("asm_{}.bin", trial));
        std::fs::write(&s_path, &asm_text).unwrap();
        let asm_comp = Command::new("clang")
            .args([s_path.to_str().unwrap(), ll_drv.to_str().unwrap(), "-o", s_bin.to_str().unwrap()])
            .output()
            .expect("clang assemble x86_64 .s");
        assert!(
            asm_comp.status.success(),
            "Trial {}: clang assembler rejected emitted x86_64 .s:\nStderr: {}\nASM:\n{}",
            trial,
            String::from_utf8_lossy(&asm_comp.stderr),
            asm_text
        );
        let asm_run = Command::new(&s_bin).output().unwrap();
        let asm_val: i64 = String::from_utf8_lossy(&asm_run.stdout).trim().parse().unwrap();
        assert_eq!(asm_val, expected, "Trial {}: x86_64 ASM execution mismatch!", trial);
    }

    // Also verify SIMD vector lanes (`[Bits(32, 8, 5)]`) emit AVX/SSE vector mnemonics
    let simd_prog = "[Bits(32, 8, 5)]\npub struct f32x8_vec {}\npub fn vec_combine(a: f32x8_vec, b: f32x8_vec) -> f32x8_vec { return a + b; }\n";
    let simd_asm = run_codegen_on_arena(
        simd_prog,
        &["selfhost/codegen/physics.pilot", "selfhost/codegen/asm_emitter.pilot"],
        "emit_arena_x86_64_asm()",
    );
    assert!(
        simd_asm.contains("vaddps") || simd_asm.contains("addps"),
        "SIMD vector layout `[Bits(32, 8, 5)]` must emit `vaddps`/`addps` vector instruction:\n{}",
        simd_asm
    );

    let _ = std::fs::remove_dir_all(&tmp);
    guard.finish();
}

#[test]
fn test_56_2_and_56_4_all_eight_gpu_backends_ast_lowering_cpu_execution_and_legacy_purge() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_56_2_and_56_4_all_eight_gpu_backends_ast_lowering_cpu_execution_and_legacy_purge",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5602);
    let tmp = make_temp_dir("gpu", &mut rng);

    for trial in 0..3 {
        let ty_ptr = rng.ident("buf_f32");
        let ty_idx = rng.ident("idx64");
        let kern_name = rng.ident("affine_blend_kernel");
        let k1 = rng.range(2, 9);
        let k2 = rng.range(3, 11);
        let k3 = rng.range(5, 29);

        let gpu_prog = format!(
            "[Bits(64, 6)]\npub struct {ty_ptr} {{}}\n\
             [Bits(64, 2)]\npub struct {ty_idx} {{}}\n\
             pub fn {kern_name}(out: {ty_ptr}, a: {ty_ptr}, b: {ty_ptr}, n: {ty_idx}) {{\n\
                 let idx = gpu_global_id();\n\
                 if idx < n {{\n\
                     out[idx] = (a[idx] * {k1}) + (b[idx] * {k2}) - {k3};\n\
                 }}\n\
             }}\n"
        );

        let gpu_files = [
            "selfhost/codegen/physics.pilot",
            "selfhost/codegen/gpu_c_dialects.pilot",
            "selfhost/codegen/gpu_shaders.pilot",
            "selfhost/codegen/gpu_llvm.pilot",
        ];

        // 1. Execute CUDA, HIP, OpenCL, and OpenMP kernels natively via `clang++` + CPU shim!
        for backend in ["cuda", "hip", "opencl", "openmp"] {
            let emitted = run_codegen_on_arena(
                &gpu_prog,
                &gpu_files,
                &format!("emit_arena_gpu_kernel(\"{}\", \"{}\")", backend, kern_name),
            );
            // Strip `#include <cuda_runtime.h>` / `<hip/hip_runtime.h>` / `<omp.h>` so our CPU shim supplies them!
            let sanitized: String = emitted
                .lines()
                .filter(|l| {
                    !l.contains("cuda_runtime.h")
                        && !l.contains("hip_runtime.h")
                        && !l.contains("<omp.h>")
                })
                .collect::<Vec<_>>()
                .join("\n");

            let invoke_loop = if backend == "openmp" {
                format!("{}(out, a, b, 64);", kern_name)
            } else {
                format!(
                    "for (int t = 0; t < 64; ++t) {{ __g_tid = t; {}(out, a, b, 64); }}",
                    kern_name
                )
            };

            let cpp_harness = format!(
                "#include <stdint.h>\n#include <stdio.h>\n#include <math.h>\n\
                 static int __g_tid = 0;\n\
                 struct __Dim3 {{ int x; }};\n\
                 static __Dim3 blockIdx = {{0}}, blockDim = {{64}}; \n\
                 #define threadIdx (__Dim3{{__g_tid}})\n\
                 #define hipBlockIdx_x 0\n\
                 #define hipBlockDim_x 64\n\
                 #define hipThreadIdx_x __g_tid\n\
                 #define __global__\n\
                 #define __kernel\n\
                 #define __global\n\
                 static inline int get_global_id(int) {{ return __g_tid; }}\n\
                 {sanitized}\n\
                 int main() {{\n\
                     float out[64] = {{0}}, a[64], b[64];\n\
                     for (int i = 0; i < 64; ++i) {{ a[i] = (float)(i + 1) * 1.5f; b[i] = (float)(70 - i) * 0.5f; }}\n\
                     {invoke_loop}\n\
                     for (int i = 0; i < 64; ++i) {{\n\
                         float exp = (a[i] * {k1}.0f) + (b[i] * {k2}.0f) - {k3}.0f;\n\
                         if (fabsf(out[i] - exp) > 1e-3f) {{ printf(\"MISMATCH at %d: got %f exp %f\\n\", i, out[i], exp); return 1; }}\n\
                     }}\n\
                     return 0;\n\
                 }}\n"
            );

            let cpp_path = tmp.join(format!("gpu_{}_{}.cpp", backend, trial));
            let cpp_bin = tmp.join(format!("gpu_{}_{}.bin", backend, trial));
            std::fs::write(&cpp_path, &cpp_harness).unwrap();
            let comp = Command::new("clang++")
                .args(["-O2", "-Wno-unknown-pragmas", cpp_path.to_str().unwrap(), "-o", cpp_bin.to_str().unwrap()])
                .output()
                .expect("clang++ compile GPU C dialect");
            assert!(
                comp.status.success(),
                "Trial {} backend `{}` failed to compile under CPU shim:\nStderr: {}\nEmitted:\n{}",
                trial,
                backend,
                String::from_utf8_lossy(&comp.stderr),
                emitted
            );
            let run = Command::new(&cpp_bin).output().unwrap();
            assert!(
                run.status.success(),
                "Trial {} backend `{}` numerical mismatch: {}\nEmitted:\n{}",
                trial,
                backend,
                String::from_utf8_lossy(&run.stdout),
                emitted
            );
        }

        // 2. Verify WGSL, SPIR-V, NVPTX, and AMDGPU lower the exact randomized constants & ops!
        let wgsl = run_codegen_on_arena(
            &gpu_prog,
            &gpu_files,
            &format!("emit_arena_gpu_kernel(\"wgsl\", \"{}\")", kern_name),
        );
        assert!(
            wgsl.contains("@compute")
                && wgsl.contains(&kern_name)
                && wgsl.contains(&k1.to_string())
                && wgsl.contains(&k2.to_string())
                && wgsl.contains(&k3.to_string()),
            "WGSL shader did not lower AST body constants/ops:\n{}",
            wgsl
        );

        let spirv = run_codegen_on_arena(
            &gpu_prog,
            &gpu_files,
            &format!("emit_arena_gpu_kernel(\"spirv\", \"{}\")", kern_name),
        );
        assert!(
            spirv.contains("OpEntryPoint GLCompute")
                && spirv.contains(&kern_name)
                && (spirv.contains("OpFMul") || spirv.contains("OpIMul"))
                && (spirv.contains("OpFAdd") || spirv.contains("OpIAdd"))
                && (spirv.contains("OpFSub") || spirv.contains("OpISub"))
                && spirv.contains("OpStore")
                && spirv.contains(&k1.to_string())
                && spirv.contains(&k3.to_string()),
            "SPIR-V module did not genuinely lower AST arithmetic & store ops:\n{}",
            spirv
        );

        for (llvm_gpu, expected_marker) in [
            ("nvptx", "nvptx64-nvidia-cuda"),
            ("amdgpu", "amdgcn-amd-amdhsa"),
        ] {
            let ir = run_codegen_on_arena(
                &gpu_prog,
                &gpu_files,
                &format!("emit_arena_gpu_kernel(\"{}\", \"{}\")", llvm_gpu, kern_name),
            );
            assert!(
                ir.contains(expected_marker)
                    && ir.contains(&kern_name)
                    && (ir.contains("fmul") || ir.contains("mul "))
                    && (ir.contains("fadd") || ir.contains("add "))
                    && (ir.contains("fsub") || ir.contains("sub "))
                    && ir.contains("store ")
                    && ir.contains(&k1.to_string())
                    && ir.contains(&k3.to_string()),
                "{} LLVM IR did not lower AST body into SSA instructions:\n{}",
                llvm_gpu,
                ir
            );
        }
    }

    let _ = std::fs::remove_dir_all(&tmp);

    // Deliverable 56.4: Legacy Rust codegen purge & strict LOC caps
    let root = ws_root();
    for deleted in [
        "crates/pilot-codegen/src/asm.rs",
        "crates/pilot-codegen/src/c_target.rs",
        "crates/pilot-codegen/src/driver.rs",
        "crates/pilot-codegen/src/gpu.rs",
        "crates/pilot-codegen/src/llvm.rs",
        "crates/pilot-codegen/src/physics.rs",
        "crates/pilot-codegen/src/wasm.rs",
        "selfhost/emitter.pilot",
    ] {
        assert!(!root.join(deleted).exists(), "Legacy file {} must remain deleted", deleted);
    }
    assert!(read_ws("crates/pilot-codegen/src/lib.rs").lines().count() <= 120);
    assert!(read_ws("crates/pilot-rt/src/c_abi.rs").lines().count() <= 220);
    guard.finish();
}
