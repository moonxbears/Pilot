//! ==============================================================================
//! crates/pilot-analysis/tests/milestone57_bidirectional_transpilers_and_ingest.rs
//! Milestone 57 — Uncheatable 7-Target Transpilers, 6-Lang Ingesters & SPDX Gate
//!
//! Required `.pilot` Contracts (all tested simultaneously on randomized inputs):
//!   1. `pilot emit <lang>` (`selfhost/transpile/source.pilot`, `c_family.pilot` and
//!      `emit_{rust,c,cpp,csharp,python,js,lua}.pilot`): every emitter reads the
//!      program's typed PilIR (`selfhost/ir/*`), not the raw AST, so a program with
//!      **randomized `[Bits(...)]` type names** and randomized function names and
//!      constants is emitted with zero caller-supplied target source strings:
//!      - Rust (compiled with `rustc` and executed), C (`clang`), C++ (`clang++`),
//!        C# (`mcs` + `mono`), Python (`python3`), JavaScript (`node`), each
//!        compared with the value computed by hand in this test;
//!      - Lua (no interpreter on the host: structural `function`..`while..do`..
//!        `if..then..else..end` check).
//!   2. `selfhost/transpile/ingest.pilot`:
//!      - `pub fn ingest_foreign_source(source_lang: str, foreign_src: str) -> str`
//!        Accepts `"python"`, `"rust"`, `"js"`, `"csharp"`, `"c"` (and `"cpp"`), and `"lua"`
//!        containing **raw foreign source code** (functions, parameters, mutable locals,
//!        `while` loops, `if`/`else`, `%`/`*`/`+`/`-`/`==`/`===` operators, `return`)
//!        and emits **strictly canonical Pilot** (`pub fn`, `var`, `{ ... }`, `;` — NEVER
//!        `def`, `class`, `let mut`, `function`, `elseif`, or `===`) that parses with
//!        0 diagnostics in `pilot_parser::parse` AND executes on `PilVirtualMachine`!
//!   3. `stdlib/pkg/licensing.pilot`:
//!      - `pub fn is_viable_spdx_license(spdx_expr: str) -> bool`
//!        Evaluates both single SPDX identifiers (`MIT`, `Apache-2.0`, `BSD-2-Clause`,
//!        `BSD-3-Clause`, `ISC`, `CC0-1.0`, `Unlicense`, `0BSD`, `Zlib`, `BSL-1.0` => `true`;
//!        `GPL-*`, `AGPL-*`, `LGPL-*`, unknown/empty => `false`) AND compound boolean
//!        SPDX expressions with `AND`, `OR`, and parentheses `(...)`!
//! ==============================================================================

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const TRANSPILE_BANNED_CHEATS: &[&str] = &[
    "get_service_data",
    "Shape::Circle",
    "Channel::receive",
    "String.prototype.shout",
    "head_pt",
];

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 57,
    test_file_rel: "crates/pilot-analysis/tests/milestone57_bidirectional_transpilers_and_ingest.rs",
    expected_test_count: 2,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/transpile/source.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["emit_program"],
            required_code_markers: &[],
            forbidden_code_markers: &["__engine"],
            forbidden_string_substrings: TRANSPILE_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/transpile/c_family.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &["__engine"],
            forbidden_string_substrings: TRANSPILE_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/transpile/emit_rust.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["rust_dialect"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: TRANSPILE_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/transpile/emit_c.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["c_dialect"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: TRANSPILE_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/transpile/emit_cpp.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["cpp_dialect"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: TRANSPILE_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/transpile/emit_csharp.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["csharp_dialect"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: TRANSPILE_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/transpile/emit_python.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["python_dialect"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: TRANSPILE_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/transpile/emit_js.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["js_dialect"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: TRANSPILE_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/transpile/emit_lua.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["lua_dialect"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: TRANSPILE_BANNED_CHEATS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/transpile/ingest.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["ingest_foreign_source"],
            required_code_markers: &[],
            forbidden_code_markers: &["host_spawn_eval"],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/pkg/licensing.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["is_viable_spdx_license"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5757_5757_CAFE_BABE);
        let s = nanos ^ ((std::process::id() as u64).rotate_left(11)) ^ salt;
        Self(if s == 0 { 0xCAFE_1234_BABE_5678 } else { s })
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

fn escape_pilot_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `pilot emit <lang> <program>`: the emitted source, after the command
/// succeeded.
fn pilot_emit(lang: &str, program: &Path) -> String {
    let out = Command::new("cargo")
        .args(["run", "-q", "-p", "pilot-cli", "--", "emit", lang, program.to_str().unwrap()])
        .current_dir(ws_root())
        .output()
        .expect("run pilot emit");
    assert!(
        out.status.success(),
        "pilot emit {} failed:\n{}{}",
        lang,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let code = String::from_utf8_lossy(&out.stdout).to_string();
    for banned in TRANSPILE_BANNED_CHEATS {
        assert!(!code.contains(banned), "emitted {} contains banned benchmark cheat `{}`", lang, banned);
    }
    code
}

/// Runs a host compiler or program, which must succeed; its stdout.
fn run_ok(trial: usize, what: &str, cmd: &mut Command, code: &str) -> String {
    let out = cmd.output().unwrap_or_else(|e| panic!("Trial {}: cannot start {}: {}", trial, what, e));
    assert!(
        out.status.success(),
        "Trial {}: {} rejected the emitted code:\n{}{}\nCode:\n{}",
        trial,
        what,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
        code
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// A program's stdout as one integer.
fn integer(trial: usize, what: &str, stdout: &str) -> i64 {
    stdout
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("Trial {}: {} printed no integer: {:?}", trial, what, stdout))
}

/// Compiles with `compiler args.. src -o bin`, runs `bin`; its integer.
fn compile_and_run(trial: usize, what: &str, compiler: &str, args: &[&str], src: &Path, bin: &Path, code: &str) -> i64 {
    run_ok(trial, what, Command::new(compiler).args(args).arg(src).arg("-o").arg(bin), code);
    integer(trial, what, &run_ok(trial, what, &mut Command::new(bin), code))
}

#[test]
pub fn test_57_1_and_57_2_seven_target_transpilers_compiled_and_executed_on_host_toolchains() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_57_1_and_57_2_seven_target_transpilers_compiled_and_executed_on_host_toolchains",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5701);
    let tmp = std::env::temp_dir().join(format!("pilot_m57_{}_{}", std::process::id(), rng.next_u64()));
    std::fs::create_dir_all(&tmp).unwrap();

    for trial in 0..4 {
        let ty_int = rng.ident("num64");
        let st_box = rng.pascal("BoxPair");
        let fn_helper = rng.ident("scale_delta");
        let fn_entry = rng.ident("run_sim");
        let k_mul = rng.range(2, 7);
        let k_add = rng.range(5, 31);
        let k_sub = rng.range(1, 13);
        let arg_a = rng.range(6, 20);
        let arg_b = rng.range(5, 14);

        let pilot_prog = format!(
            "[Bits(Cfg.Reg.b64, Cfg.Reg.gpr_s)]\npub struct {ty_int} {{}}\n\
             pub struct {st_box} {{\n\
                 pub u: {ty_int};\n\
                 pub v: {ty_int};\n\
             }}\n\
             pub fn {fn_helper}(x: {ty_int}, y: {ty_int}) -> {ty_int} {{\n\
                 return (x * {k_mul}) - y + {k_add};\n\
             }}\n\
             pub fn {fn_entry}(a: {ty_int}, b: {ty_int}) -> {ty_int} {{\n\
                 var acc: {ty_int} = {fn_helper}(a, b);\n\
                 var i: {ty_int} = 0;\n\
                 while i < b {{\n\
                     if (i % 2) == 0 {{\n\
                         acc = acc + i * {k_mul};\n\
                     }} else {{\n\
                         acc = acc - {k_sub};\n\
                     }}\n\
                     i = i + 1;\n\
                 }}\n\
                 return acc;\n\
             }}\n"
        );

        let mut expected = (arg_a * k_mul) - arg_b + k_add;
        for i in 0..arg_b {
            if i % 2 == 0 {
                expected += i * k_mul;
            } else {
                expected -= k_sub;
            }
        }

        let src = tmp.join(format!("t_{}.pilot", trial));
        std::fs::write(&src, &pilot_prog).unwrap();
        let emitted = |lang: &str| pilot_emit(lang, &src);
        let file = |ext: &str| tmp.join(format!("t_{}.{}", trial, ext));
        let bin = |tag: &str| tmp.join(format!("t_{}_{}.bin", tag, trial));

        // 1. Rust (`rustc`)
        let rs_code = emitted("rust");
        std::fs::write(
            file("rs"),
            format!("#![allow(warnings)]\n{}\nfn main() {{ println!(\"{{}}\", {}({}, {})); }}\n", rs_code, fn_entry, arg_a, arg_b),
        )
        .unwrap();
        let rs_val = compile_and_run(trial, "rustc", "rustc", &["-O"], &file("rs"), &bin("rs"), &rs_code);
        assert_eq!(rs_val, expected, "Trial {}: Transpiled Rust output mismatch!", trial);

        // 2. C (`clang`)
        let c_code = emitted("c");
        std::fs::write(
            file("c"),
            format!("#include <stdio.h>\n{}\nint main(void) {{ printf(\"%lld\\n\", (long long){}({}, {})); return 0; }}\n", c_code, fn_entry, arg_a, arg_b),
        )
        .unwrap();
        // The emitted C and C++ include the runtime header (F6).
        let rt = format!("-I{}", ws_root().join("runtime/c").display());
        let c_val = compile_and_run(trial, "clang", "clang", &["-std=c11", &rt, "-lm"], &file("c"), &bin("c"), &c_code);
        assert_eq!(c_val, expected, "Trial {}: Transpiled C output mismatch!", trial);

        // 3. C++ (`clang++`)
        let cpp_code = emitted("cpp");
        std::fs::write(
            file("cpp"),
            format!("#include <iostream>\n{}\nint main() {{ std::cout << {}({}, {}) << \"\\n\"; return 0; }}\n", cpp_code, fn_entry, arg_a, arg_b),
        )
        .unwrap();
        let cpp_val = compile_and_run(trial, "clang++", "clang++", &["-std=c++17", &rt], &file("cpp"), &bin("cpp"), &cpp_code);
        assert_eq!(cpp_val, expected, "Trial {}: Transpiled C++ output mismatch!", trial);

        // 4. C# (`mcs` + `mono`)
        let cs_code = emitted("csharp");
        std::fs::write(
            file("cs"),
            format!(
                "{}\npublic static class M57Harness {{ public static void Main() {{ System.Console.WriteLine(PilotProgram.{}({}, {})); }} }}\n",
                cs_code, fn_entry, arg_a, arg_b
            ),
        )
        .unwrap();
        let cs_exe = tmp.join(format!("t_{}.exe", trial));
        run_ok(
            trial,
            "mcs",
            Command::new("mcs").arg(format!("-out:{}", cs_exe.display())).arg(file("cs")),
            &cs_code,
        );
        let cs_val = integer(trial, "mono", &run_ok(trial, "mono", Command::new("mono").arg(&cs_exe), &cs_code));
        assert_eq!(cs_val, expected, "Trial {}: Transpiled C# output mismatch!", trial);

        // 5. Python (`python3`)
        let py_code = emitted("python");
        std::fs::write(file("py"), format!("{}\nprint({}({}, {}))\n", py_code, fn_entry, arg_a, arg_b)).unwrap();
        let py_val = integer(trial, "python3", &run_ok(trial, "python3", Command::new("python3").arg(file("py")), &py_code));
        assert_eq!(py_val, expected, "Trial {}: Transpiled Python output mismatch!", trial);

        // 6. JavaScript (`node`)
        let js_code = emitted("js");
        std::fs::write(file("js"), format!("{}\nconsole.log(String({}({}, {})));\n", js_code, fn_entry, arg_a, arg_b)).unwrap();
        let js_val = integer(trial, "node", &run_ok(trial, "node", Command::new("node").arg(file("js")), &js_code));
        assert_eq!(js_val, expected, "Trial {}: Transpiled JS output mismatch!", trial);

        // 7. Lua: structural check on the randomized symbols and constants
        let lua_code = emitted("lua");
        assert!(
            lua_code.contains(&format!("function {}", fn_entry))
                && lua_code.contains(&fn_helper)
                && lua_code.contains("while ")
                && lua_code.contains(" do")
                && lua_code.contains("then")
                && lua_code.contains("else")
                && lua_code.contains("end")
                && lua_code.contains(&k_add.to_string()),
            "Trial {}: Transpiled Lua missing lowered constructs:\n{}",
            trial,
            lua_code
        );
    }

    let _ = std::fs::remove_dir_all(&tmp);
    guard.finish();
}

#[test]
pub fn test_57_3_and_57_4_six_language_foreign_ingesters_vm_execution_spdx_and_crate_purge() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_57_3_and_57_4_six_language_foreign_ingesters_vm_execution_spdx_and_crate_purge",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5703);
    let ingest_mod = read_ws("selfhost/transpile/ingest.pilot");
    let license_mod = read_ws("stdlib/pkg/licensing.pilot");

    // 1. Compound & atomic SPDX license verification
    let spdx_cases = [
        ("MIT", true),
        ("Apache-2.0", true),
        ("BSD-3-Clause", true),
        ("0BSD", true),
        ("Zlib", true),
        ("GPL-3.0", false),
        ("AGPL-3.0-only", false),
        ("LGPL-2.1", false),
        ("", false),
        ("UNKNOWN-PROPRIETARY-99", false),
        ("MIT AND Apache-2.0", true),
        ("MIT AND GPL-3.0", false),
        ("GPL-3.0 OR MIT", true),
        ("(GPL-2.0-only OR BSD-2-Clause) AND ISC", true),
        ("(GPL-3.0 OR AGPL-3.0-only) AND MIT", false),
    ];
    for (expr, expected) in spdx_cases {
        let script = format!(
            "{}\nlet __ok = is_viable_spdx_license({});\n__ok;\n",
            license_mod,
            escape_pilot_str(expr)
        );
        let arena = parse(&script);
        assert!(arena.diagnostics.is_empty());
        let mut vm = PilVirtualMachine::new();
        vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
        assert_eq!(
            vm.run().as_bool(),
            Some(expected),
            "SPDX check failed for `{}` (expected {})",
            expr,
            expected
        );
    }

    // 2. 6-Language raw foreign source ingestion -> canonical Pilot parse -> PilVM execution!
    for trial in 0..4 {
        let fn_name = rng.ident("foreign_kern");
        let k_mul = rng.range(2, 8);
        let k_add = rng.range(5, 29);
        let arg_a = rng.range(4, 15);
        let arg_b = rng.range(4, 11);

        let mut expected = arg_a * k_mul;
        for i in 0..arg_b {
            if i % 2 == 0 {
                expected = expected + i + k_add;
            } else {
                expected -= 1;
            }
        }

        let py_src = format!(
            "def {f}(a: int, b: int) -> int:\n\
             \x20   acc = a * {k_mul}\n\
             \x20   i = 0\n\
             \x20   while i < b:\n\
             \x20       if i % 2 == 0:\n\
             \x20           acc = acc + i + {k_add}\n\
             \x20       else:\n\
             \x20           acc = acc - 1\n\
             \x20       i = i + 1\n\
             \x20   return acc\n",
            f = fn_name
        );

        let rs_src = format!(
            "pub fn {f}(a: i64, b: i64) -> i64 {{\n\
                 let mut acc: i64 = a * {k_mul};\n\
                 let mut i: i64 = 0;\n\
                 while i < b {{\n\
                     if i % 2 == 0 {{\n\
                         acc = acc + i + {k_add};\n\
                     }} else {{\n\
                         acc = acc - 1;\n\
                     }}\n\
                     i = i + 1;\n\
                 }}\n\
                 return acc;\n\
             }}\n",
            f = fn_name
        );

        let js_src = format!(
            "function {f}(a, b) {{\n\
                 let acc = a * {k_mul};\n\
                 let i = 0;\n\
                 while (i < b) {{\n\
                     if (i % 2 === 0) {{\n\
                         acc = acc + i + {k_add};\n\
                     }} else {{\n\
                         acc = acc - 1;\n\
                     }}\n\
                     i = i + 1;\n\
                 }}\n\
                 return acc;\n\
             }}\n",
            f = fn_name
        );

        let cs_src = format!(
            "public static class ForeignMath {{\n\
                 public static long {f}(long a, long b) {{\n\
                     long acc = a * {k_mul};\n\
                     long i = 0;\n\
                     while (i < b) {{\n\
                         if (i % 2 == 0) {{\n\
                             acc = acc + i + {k_add};\n\
                         }} else {{\n\
                             acc = acc - 1;\n\
                         }}\n\
                         i = i + 1;\n\
                     }}\n\
                     return acc;\n\
                 }}\n\
             }}\n",
            f = fn_name
        );

        let c_src = format!(
            "long long {f}(long long a, long long b) {{\n\
                 long long acc = a * {k_mul};\n\
                 long long i = 0;\n\
                 while (i < b) {{\n\
                     if (i % 2 == 0) {{\n\
                         acc = acc + i + {k_add};\n\
                     }} else {{\n\
                         acc = acc - 1;\n\
                     }}\n\
                     i = i + 1;\n\
                 }}\n\
                 return acc;\n\
             }}\n",
            f = fn_name
        );

        let lua_src = format!(
            "function {f}(a, b)\n\
               local acc = a * {k_mul}\n\
               local i = 0\n\
               while i < b do\n\
                 if i % 2 == 0 then\n\
                   acc = acc + i + {k_add}\n\
                 else\n\
                   acc = acc - 1\n\
                 end\n\
                 i = i + 1\n\
               end\n\
               return acc\n\
             end\n",
            f = fn_name
        );

        for (lang, foreign) in [
            ("python", &py_src),
            ("rust", &rs_src),
            ("js", &js_src),
            ("csharp", &cs_src),
            ("c", &c_src),
            ("lua", &lua_src),
        ] {
            let ing_script = format!(
                "{}\nlet __pil = ingest_foreign_source({}, {});\n__pil;\n",
                ingest_mod,
                escape_pilot_str(lang),
                escape_pilot_str(foreign),
            );
            let ing_arena = parse(&ing_script);
            assert!(
                ing_arena.diagnostics.is_empty(),
                "ingest.pilot failed to parse: {:?}",
                ing_arena.diagnostics
            );
            let mut ivm = PilVirtualMachine::new();
            ivm.load_binary(BytecodeCompiler::compile(&ing_arena, ing_arena.root));
            let pilot_out = ivm.run().to_string_repr();

            // Must emit strictly canonical Pilot with zero foreign/legacy remnants!
            for forbidden in ["def ", "class ", "let mut ", "function ", "===", "local "] {
                assert!(
                    !pilot_out.contains(forbidden),
                    "Trial {} `{}` ingester leaked prohibited non-Pilot syntax `{}`:\n{}",
                    trial,
                    lang,
                    forbidden,
                    pilot_out
                );
            }

            // Parse and execute the ingested Pilot code on PilVirtualMachine!
            let exec_src = format!("{}\nlet __r = {}({}, {});\n__r;\n", pilot_out, fn_name, arg_a, arg_b);
            let exec_arena = parse(&exec_src);
            assert!(
                exec_arena.diagnostics.is_empty(),
                "Trial {} `{}` ingested Pilot failed to parse: {:?}\nIngested Pilot:\n{}",
                trial,
                lang,
                exec_arena.diagnostics,
                pilot_out
            );
            let mut evm = PilVirtualMachine::new();
            evm.load_binary(BytecodeCompiler::compile(&exec_arena, exec_arena.root));
            let val = evm.run().as_int();
            assert_eq!(
                val,
                Some(expected),
                "Trial {} `{}` ingested Pilot executed to wrong value!\nForeign:\n{}\nIngested:\n{}",
                trial,
                lang,
                foreign,
                pilot_out
            );
        }
    }

    // Deliverable 57.4: Legacy `pilot-transpile` completely retired
    let root = ws_root();
    assert!(!root.join("crates/pilot-transpile").exists());
    assert!(!read_ws("Cargo.toml").contains("pilot-transpile"));
    guard.finish();
}
