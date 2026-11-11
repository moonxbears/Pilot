//! ==============================================================================
//! crates/pilot-analysis/tests/milestone55_selfhost_vm_compiler_and_debug_table.rs
//! Milestone 55 — Uncheatable Self-Hosted PilVM v3 Bytecode Compiler & `.pilc` Gate
//!
//! Required `.pilot` Contracts (all tested simultaneously on randomized ASTs):
//!   1. `selfhost/vm/compiler.pilot` & `selfhost/vm/pilc_writer.pilot`:
//!      - `pub fn compile_arena_to_pilc_hex() -> str`
//!        Walks the lossless `RawAstArena` in `PhysicsRegistry` via `__engine("ast_*", ...)`
//!        and `__engine("sym_str", ...)`, compiles all `FnDecl`s (`kind_tag == 3`,
//!        `orig_kind == 3`), parameters (`kind_tag == 21`), local `let`/`var` bindings
//!        (`kind_tag == 4`), assignments (`kind_tag == 20`), `if`/`else` (`kind_tag == 6`),
//!        `while` loops (`kind_tag == 7`, `orig_kind == 106`) with `break` (`orig_kind == 107`)
//!        and `continue` (`orig_kind == 108`), `return` (`kind_tag == 8`), unary & binary
//!        operators (`orig_kind == 206` & `kind_tag == 19`), string `.len()`/`.byte_at()`/
//!        `.slice()` member calls, function calls, and `__engine(...)` calls into valid
//!        `PIL3` v3 binary bytes encoded as lowercase hexadecimal!
//!      - Exact `PIL3` v3 Wire Format consumed by `PilcBinary::load_from_bytes`:
//!        * Bytes `0..4`: `"PIL3"` (`50494c33`)
//!        * Bytes `4..8`: `5u32` little-endian (`05000000`)
//!        * Bytes `8..12`: `const_count: u32` LE
//!        * `const_count * 16` bytes: `(tag_meta: u64 LE, payload: u64 LE)`:
//!          - `TAG_NULL = 0`, `TAG_BOOL = 2` (`payload = 0|1`), `TAG_INT = 3` (`payload = i64 as u64`),
//!            `TAG_FLOAT = 5` (`payload = f64_bits`),
//!          - `TAG_SSO_STR = 8` (for UTF-8 len `<= 14`: `meta_bytes = [8, len, b0..b5]`, `payload_bytes = [b6..b13]`),
//!          - `TAG_HEAP_STR = 9` (for UTF-8 len `> 14`: `tag_meta = 9 | (len << 8)`, `payload = byte_offset_in_heap_blob`,
//!            where `heap_blob` stores `u32_len_le` + raw UTF-8 bytes for each heap string!).
//!        * `heap_len: u32` LE + `heap_len` bytes of `heap_blob`
//!        * `ins_count: u32` LE + `ins_count` entries of `u64` LE (`(ext << 32) | (op << 24) | (dst << 16) | (src1 << 8) | src2`,
//!          where `ext` is the 32-bit jump target, call entry or constant index).
//!      - `pub fn compile_arena_debug_manifest() -> str`
//!        Returns debug metadata formatted as:
//!        `"SPANS:<pc>:<start>:<end>,...|REGS:<reg>:<sym_id>:<name>,..."`
//!        mapping emitted instruction PCs to source `ByteSpan`s (`ast_span_start`/`ast_span_end`)
//!        and allocated local registers (`17..95`) to symbol IDs and names!
//!      - Zero hardcoded `"main"` special-casing: top-level statements execute in order
//!        (returning the final top-level expression in `r0`, or calling the last 0-arg
//!        `FnDecl` if no top-level value expression exists).
//!   2. Contraction & Opcode Purification Gates:
//!      - `crates/pilot-vm/src/compiler.rs` deleted; `crates/pilot-vm <= 650` LOC;
//!        `VmHook` traces live execution cleanly with zero `Print`/`Assert` opcodes.
//! ==============================================================================

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_analysis::{flatten_ast, handle_engine_intrinsic, PhysicsRegistry};
use pilot_parser::parse;
use pilot_vm::pilc::PilcBinary;
use pilot_vm::{BytecodeCompiler, Opcode, PilVirtualMachine, PILC_V3_MAGIC, PILC_V3_VERSION};

#[path = "common/anticheat.rs"]
mod anticheat;

const VM_GROUP: &[&str] = &["selfhost/vm/compiler.pilot", "selfhost/vm/pilc_writer.pilot"];

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 55,
    test_file_rel: "crates/pilot-analysis/tests/milestone55_selfhost_vm_compiler_and_debug_table.rs",
    expected_test_count: 3,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/vm/compiler.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &["__engine"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &["main"],
        },
        anticheat::FileAuditRule {
            path: "selfhost/vm/pilc_writer.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[
        ("compile_arena_to_pilc_hex", VM_GROUP),
        ("compile_arena_debug_manifest", VM_GROUP),
    ],
};

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5555_5555_DEAD_BEEF);
        let s = nanos ^ ((std::process::id() as u64).rotate_left(19)) ^ salt;
        Self(if s == 0 { 0x1357_9BDF_2468_ACE0 } else { s })
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
        for _ in 0..6 {
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

fn decode_hex(hex: &str) -> Vec<u8> {
    let trimmed = hex.trim();
    assert!(
        !trimmed.is_empty() && trimmed.len() % 2 == 0,
        "Expected non-empty even-length hex string for .pilc binary, got len={} (`{}`)",
        trimmed.len(),
        &trimmed[..trimmed.len().min(80)]
    );
    let bytes = trimmed.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let nib = |b: u8| -> u8 {
        match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => panic!("Invalid hex character `{}` in `{}`", b as char, &trimmed[..trimmed.len().min(80)]),
        }
    };
    let mut i = 0;
    while i < bytes.len() {
        out.push((nib(bytes[i]) << 4) | nib(bytes[i + 1]));
        i += 2;
    }
    out
}

/// Compiles `target_program` SOLELY via `selfhost/vm/{compiler,pilc_writer}.pilot`
/// reading `PhysicsRegistry` (`RawAstArena`), returning `(PilcBinary, debug_manifest_str)`.
fn selfhost_compile_program(target_program: &str) -> (PilcBinary, String) {
    let target_arena = parse(target_program);
    assert!(
        target_arena.diagnostics.is_empty(),
        "Target program failed to parse: {:?}\n{}",
        target_arena.diagnostics,
        target_program
    );
    let (flat, syms) = flatten_ast(&target_arena, target_arena.root);
    let mut registry = PhysicsRegistry::new(flat, syms);
    registry.source_text = target_program.to_string();

    let vm_compiler = read_ws("selfhost/vm/compiler.pilot");
    let pilc_writer = read_ws("selfhost/vm/pilc_writer.pilot");

    let driver = format!(
        "{}\n{}\n\
         let __hex = compile_arena_to_pilc_hex();\n\
         let __dbg = compile_arena_debug_manifest();\n\
         __hex + \"<<<DBG>>>\" + __dbg;\n",
        vm_compiler, pilc_writer
    );
    let driver_arena = parse(&driver);
    assert!(
        driver_arena.diagnostics.is_empty(),
        "selfhost/vm/compiler.pilot + pilc_writer.pilot failed to parse: {:?}",
        driver_arena.diagnostics
    );

    let reg_ptr = &mut registry as *mut PhysicsRegistry;
    let mut host_vm = PilVirtualMachine::new();
    host_vm.engine_handler = Some(Box::new(move |op, args| {
        handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, args)
    }));
    host_vm.load_binary(BytecodeCompiler::compile(&driver_arena, driver_arena.root));
    let combined_out = host_vm.run().to_string_repr();

    let parts: Vec<&str> = combined_out.split("<<<DBG>>>").collect();
    assert_eq!(
        parts.len(),
        2,
        "Expected `<pilc_hex><<<DBG>>><debug_manifest>`, got `{}`",
        &combined_out[..combined_out.len().min(120)]
    );

    let raw_bytes = decode_hex(parts[0]);
    assert_eq!(&raw_bytes[0..4], PILC_V3_MAGIC, ".pilc header must start with PIL3");
    assert_eq!(
        u32::from_le_bytes(raw_bytes[4..8].try_into().unwrap()),
        PILC_V3_VERSION
    );
    let pilc = PilcBinary::load_from_bytes(&raw_bytes)
        .unwrap_or_else(|e| panic!("PilcBinary::load_from_bytes rejected selfhost .pilc: {}", e));
    (pilc, parts[1].to_string())
}

#[test]
pub fn test_55_1_selfhost_compiler_emits_executable_pilc_for_randomized_recursive_and_loop_programs() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_55_1_selfhost_compiler_emits_executable_pilc_for_randomized_recursive_and_loop_programs",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5501);

    // Static check: forbid hardcoded `"main"` special-casing in `selfhost/vm/compiler.pilot`
    let compiler_src = read_ws("selfhost/vm/compiler.pilot");
    for (ln, line) in compiler_src.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        assert!(
            !code.contains("\"main\""),
            "selfhost/vm/compiler.pilot:{} must NOT hardcode `\"main\"`: {}",
            ln + 1,
            line
        );
    }

    // 8 randomized recursive + loop + break/continue + bitwise + negative literal trials
    for trial in 0..8 {
        let fn_rec = rng.ident("rec_seq");
        let fn_loop = rng.ident("scan_acc");
        let var_seed = rng.ident("top_seed");
        let c_mul = rng.range(2, 7);
        let c_add = rng.range(3, 19);
        let c_sub = rng.range(1, 11);
        let n_rec = rng.range(4, 7);
        let n_lim = rng.range(12, 24);
        let skip_at = rng.range(3, 6);
        let stop_at = rng.range(9, 11);
        let mask_val = rng.range(15, 255);

        let prog = format!(
            "pub fn {fn_rec}(n: i64, k: i64) -> i64 {{\n\
                 if n <= 1 {{\n\
                     return k + {c_add};\n\
                 }} else {{\n\
                     return {fn_rec}(n - 1, k) * {c_mul} - {c_sub};\n\
                 }}\n\
             }}\n\
             pub fn {fn_loop}(limit: i64, base: i64) -> i64 {{\n\
                 var i = 0;\n\
                 var acc = base;\n\
                 while i < limit {{\n\
                     i = i + 1;\n\
                     if i == {skip_at} {{\n\
                         continue;\n\
                     }}\n\
                     if i == {stop_at} {{\n\
                         break;\n\
                     }}\n\
                     if (i & 1) == 0 {{\n\
                         acc = (acc + (i * {c_mul})) ^ {mask_val};\n\
                     }} else {{\n\
                         acc = acc - (-{c_sub}) + (i << 1);\n\
                     }}\n\
                 }}\n\
                 return acc;\n\
             }}\n\
             let {var_seed} = {fn_rec}({n_rec}, {c_sub});\n\
             let final_ans = {fn_loop}({n_lim}, {var_seed});\n\
             final_ans;\n"
        );

        // Compute independent Rust reference answer!
        fn ref_rec(n: i64, k: i64, c_add: i64, c_mul: i64, c_sub: i64) -> i64 {
            if n <= 1 {
                k + c_add
            } else {
                ref_rec(n - 1, k, c_add, c_mul, c_sub) * c_mul - c_sub
            }
        }
        let ref_seed = ref_rec(n_rec, c_sub, c_add, c_mul, c_sub);
        let mut ref_i = 0i64;
        let mut ref_acc = ref_seed;
        while ref_i < n_lim {
            ref_i += 1;
            if ref_i == skip_at {
                continue;
            }
            if ref_i == stop_at {
                break;
            }
            if (ref_i & 1) == 0 {
                ref_acc = (ref_acc + (ref_i * c_mul)) ^ mask_val;
            } else {
                ref_acc = ref_acc - (-c_sub) + (ref_i << 1);
            }
        }

        // Compile via `selfhost/vm/{compiler,pilc_writer}.pilot` and execute on a fresh VM!
        let (pilc, dbg_manifest) = selfhost_compile_program(&prog);
        let executed_pcs = Arc::new(Mutex::new(Vec::new()));
        let pcs_clone = Arc::clone(&executed_pcs);

        let mut fresh_vm = PilVirtualMachine::new();
        fresh_vm.load_binary(pilc);
        fresh_vm.set_hook(Some(Box::new(move |pc, _ins, _regs| {
            pcs_clone.lock().unwrap().push(pc);
        })));
        let actual = fresh_vm.run().as_int();

        assert_eq!(
            actual,
            Some(ref_acc),
            "Trial {}: Self-hosted .pilc binary produced wrong result on fresh PilVM!\nProgram:\n{}",
            trial,
            prog
        );
        assert!(
            executed_pcs.lock().unwrap().len() > 25,
            "Trial {}: VmHook expected > 25 executed instructions",
            trial
        );

        // Verify debug manifest contains valid PC spans inside `prog` and local register symbol names!
        assert!(
            dbg_manifest.contains("SPANS:") && dbg_manifest.contains("REGS:"),
            "Trial {}: Debug manifest missing SPANS:/REGS: sections: `{}`",
            trial,
            dbg_manifest
        );
        assert!(
            dbg_manifest.contains(&var_seed) && dbg_manifest.contains("final_ans"),
            "Trial {}: Debug register table must record local variable symbols `{}` and `final_ans`: `{}`",
            trial,
            var_seed,
            dbg_manifest
        );
    }
    guard.finish();
}

#[test]
pub fn test_55_2_sso_and_heap_string_constants_and_zero_arg_entry_without_main_hardcoding() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_55_2_sso_and_heap_string_constants_and_zero_arg_entry_without_main_hardcoding",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5502);

    for trial in 0..5 {
        let entry_name = rng.ident("custom_boot_entry"); // NOT named `main`!
        let sso_lit = format!("s_{}", rng.range(100, 999)); // <= 14 bytes (TAG_SSO_STR = 8)
        let heap_lit = format!(
            "long_heap_constant_payload_{}_{}",
            rng.ident("seg"),
            rng.range(10000, 99999)
        ); // > 14 bytes (TAG_HEAP_STR = 9)

        let prog = format!(
            "pub fn {entry}() -> str {{\n\
                 let a = \"{sso}\";\n\
                 let b = \"{heap}\";\n\
                 return a + \"|\" + b.slice(0, 18) + \"|\" + b.len();\n\
             }}\n",
            entry = entry_name,
            sso = sso_lit,
            heap = heap_lit,
        );

        let (pilc, _) = selfhost_compile_program(&prog);
        let mut fresh_vm = PilVirtualMachine::new();
        fresh_vm.load_binary(pilc);
        let out = fresh_vm.run().to_string_repr();
        let expected = format!("{}|{}|{}", sso_lit, &heap_lit[0..18], heap_lit.len());
        assert_eq!(
            out, expected,
            "Trial {}: SSO + Heap string pool or 0-arg custom entry `{}` failed in selfhost .pilc!",
            trial, entry_name
        );
    }
    guard.finish();
}

#[test]
pub fn test_55_3_and_55_4_debug_tables_opcode_purification_and_line_caps() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_55_3_and_55_4_debug_tables_opcode_purification_and_line_caps",
        &ANTICHEAT_SPEC,
    );
    let root = ws_root();
    assert!(
        !root.join("crates/pilot-vm/src/compiler.rs").exists(),
        "Legacy crates/pilot-vm/src/compiler.rs must remain deleted"
    );

    let mut total_lines = 0usize;
    for file in ["instruction.rs", "pilc.rs", "vm.rs", "lib.rs"] {
        let content = read_ws(&format!("crates/pilot-vm/src/{}", file));
        total_lines += content.lines().count();
    }
    let vm_cap = anticheat::seed_rust_crate_loc_cap(&root, 650);
    assert!(
        total_lines <= vm_cap,
        "crates/pilot-vm total LOC ({}) must be <= {}",
        total_lines,
        vm_cap
    );

    let inst_rs = read_ws("crates/pilot-vm/src/instruction.rs");
    assert!(
        !inst_rs.contains("Print") && !inst_rs.contains("Assert"),
        "Purified Opcode enum must not contain legacy Print or Assert opcodes"
    );

    let src = "let alpha = 20;\nlet beta = 22;\nassert(alpha + beta == 42);\nalpha + beta;\n";
    let arena = parse(src);
    let binary = BytecodeCompiler::compile(&arena, arena.root);
    assert_eq!(binary.instructions.len(), binary.pc_spans.len());
    assert!(!binary.reg_symbols.is_empty());
    assert!(binary.instructions.iter().any(|ins| ins.opcode() == Opcode::EngineIntrinsic));
    guard.finish();
}
