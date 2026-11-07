//! Milestone 60 Rigorous Uncheatable Verification Suite:
//! Self-Hosted CLI, Package/Venv Manager, Profiler & AST Optimizer —
//! Retire `pilot-pkg`, `pilot-perf`, `pilot-bench` & Shrink `pilot-cli <= 100` LOC.
//!
//! ============================================================================
//! FROZEN-RUST RUNNER CONTRACT (DO NOT MODIFY THIS TEST FILE OR RUST CRATES)
//! ============================================================================
//! All tests in this file must pass simultaneously using ONLY edits to `.pilot`
//! files (`selfhost/pkg/*.pilot`, `selfhost/perf/*.pilot`,
//! `selfhost/bench/bench.pilot`, `selfhost/cli/*.pilot`, `selfhost/pilotc.pilot`,
//! `stdlib/crypto/hash.pilot`).
//!
//! Required `.pilot` entrypoints & behavioral contracts:
//!
//! 1. `selfhost/pkg/{manifest,venv,archive,interop}.pilot` (`<= 1,500` LOC each):
//!    - `manifest.pilot`:
//!      - `pub fn generate_default_manifest_json(pkg_name: str, version: str) -> str`
//!      - `pub fn parse_manifest_json(json_src: str) -> i64`
//!      - `pub fn manifest_get_name(mf: i64) -> str`
//!      - `pub fn manifest_get_version(mf: i64) -> str`
//!      - `pub fn manifest_dep_count(mf: i64) -> i64`
//!      - `pub fn manifest_dep_version(mf: i64, dep_name: str) -> str`
//!      - `pub fn scaffold_canonical_main_pilot(pkg_name: str) -> str` (canonical `pub fn main() -> i64`, zero `def `)
//!      - `pub fn compute_package_lockfile_json(pkg_name: str, version: str, payload: str) -> str`
//!        (Uses real `sha256_hex(payload)` from `stdlib/crypto/hash.pilot`, emitting valid JSON with `"lockfileVersion": 3` and `"integrity": "sha256-<64hex>"`.)
//!      - `pub fn verify_lockfile_integrity(lock_json: str, payload: str) -> bool`
//!    - `archive.pilot`:
//!      - `pub fn pilpkg_new() -> i64`
//!      - `pub fn pilpkg_add_file(ar: i64, rel_path: str, content: str) -> i64`
//!      - `pub fn pilpkg_serialize(ar: i64) -> str`
//!      - `pub fn pilpkg_file_count(archive_blob: str) -> i64`
//!      - `pub fn pilpkg_extract_file(archive_blob: str, rel_path: str) -> str` (returns `""` if path absent)
//!    - `venv.pilot`:
//!      - `pub fn format_pilotvenv_cfg(venv_dir: str, compiler_ver: str) -> str`
//!      - `pub fn parse_pilotvenv_version(cfg_text: str) -> str`
//!    - `interop.pilot`:
//!      - `pub fn parse_ecosystem_spec(spec: str) -> str`
//!        (Parses `"<eco>:<pkg>@<ver>"` for `cargo`, `pip`, `npm` into `"<eco>|<pkg>|<ver>"`.)
//!
//! 2. `selfhost/perf/{profiler,wcet,optimizer}.pilot` & `selfhost/bench/bench.pilot` (`<= 1,500` LOC each):
//!    - `profiler.pilot`:
//!      - `pub fn profile_folded_stacks_to_svg(folded_stacks: str, total_width_px: i64) -> str`
//!        (Aggregates `\n`-delimited `"frame1;frame2 <count>"` or raw per-tick `"frame1;frame2"` lines from `VmHook` into valid XML `<svg xmlns="http://www.w3.org/2000/svg" ...>` whose `<rect>` widths are proportional to sample counts.)
//!      - `pub fn profile_folded_stacks_to_chrome_trace(folded_stacks: str) -> str`
//!        (Emits valid Chrome Trace JSON `{"traceEvents":[...]}` with `"ph":"X"` and `"dur"` proportional to measured samples.)
//!    - `wcet.pilot`:
//!      - `pub fn verify_arena_wcet() -> str`
//!        Walks mounted `AstArena` functions carrying `[HardRealtime]` and `[Wcet(budget)]`:
//!        - Rejects unbounded `while` / `loop` constructs with `"FAIL:UNBOUNDED_LOOP"`.
//!        - Computes loop bound * body operation cost on bounded `for i in a..b` loops: returns `"FAIL:WCET_EXCEEDED"` if cost > `budget`, else `"PASS:<cycles>/<budget>"`.
//!    - `optimizer.pilot`:
//!      - `pub fn optimize_mounted_pilot_source() -> str`
//!        Walks mounted `AstArena` + `source_slice` to recursively fold nested integer constant expressions (`+`, `-`, `*`, `/`) and eliminate dead `if false { ... }` branches, returning valid canonical Pilot source code.
//!    - `bench.pilot`:
//!      - `pub fn compute_bench_stats_json(samples_csv: str) -> str`
//!        Computes valid JSON `{"count":N,"min":...,"max":...,"mean":...,"median":...}` over comma-separated integer nanosecond samples.
//!
//! 3. `selfhost/cli/{main,build_cmd,test_runner,diff_test,repl}.pilot` & `selfhost/pilotc.pilot` (`<= 1,500` LOC each):
//!    - `cli/main.pilot` & `cli/build_cmd.pilot`:
//!      - `pub fn parse_cli_build_args(argv_line: str) -> str`
//!        (Parses e.g. `"build <input> --target <t> --release -o <out>"` -> `"<cmd>|<input>|<target>|<release_0_or_1>|<out>"`.)
//!    - `cli/test_runner.pilot`:
//!      - `pub fn discover_arena_tests() -> str` (`|`-joined names of `test_*` or `[Test]` `fn`s in mounted AST)
//!      - `pub fn summarize_test_suite_json(results_csv: str) -> str` (e.g. `"t1:1,t2:0,t3:1"` -> JSON with `"passed":2,"failed":1`)
//!    - `cli/diff_test.pilot`:
//!      - `pub fn check_differential_backends(vm_out: str, c_out: str, llvm_out: str, wasm_out: str) -> str`
//!        (Returns `"PASS"` iff all 4 match; otherwise `"MISMATCH:<backend_name>"` naming the first diverging backend.)
//!    - `cli/repl.pilot`:
//!      - Zero hardcoded Pilot scalar type names (`"i64"`, `"f64"`, `"bool"`, `"str"`, `"usize"`).
//!      - `pub fn repl_session_new() -> i64`
//!      - `pub fn repl_eval_line(sess: i64, line: str) -> str`
//!        Persists `let`/`var` bindings and reassignments across lines, evaluates arithmetic expressions over session bindings, and returns the evaluated decimal result string.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_analysis::{AnalysisPipeline, handle_engine_intrinsic, PhysicsRegistry};
use pilot_intern::SymbolId;
use pilot_ast::NodeKind;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const CLI_BUILD_GROUP: &[&str] = &["selfhost/cli/main.pilot", "selfhost/cli/build_cmd.pilot"];

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 60,
    test_file_rel: "crates/pilot-analysis/tests/milestone60_selfhost_cli_pkg_perf_and_bench.rs",
    expected_test_count: 3,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/pkg/manifest.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "generate_default_manifest_json",
                "parse_manifest_json",
                "manifest_get_name",
                "manifest_get_version",
                "manifest_dep_count",
                "manifest_dep_version",
                "scaffold_canonical_main_pilot",
                "compute_package_lockfile_json",
                "verify_lockfile_integrity",
            ],
            required_code_markers: &["sha256_hex"],
            forbidden_code_markers: &["14695981039346656037", "0xcbf29ce484222325"],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/pkg/archive.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "pilpkg_new",
                "pilpkg_add_file",
                "pilpkg_serialize",
                "pilpkg_file_count",
                "pilpkg_extract_file",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/pkg/venv.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["format_pilotvenv_cfg", "parse_pilotvenv_version"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/pkg/interop.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["parse_ecosystem_spec"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/perf/profiler.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "profile_folded_stacks_to_svg",
                "profile_folded_stacks_to_chrome_trace",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/perf/wcet.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["verify_arena_wcet"],
            required_code_markers: &["__engine"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/perf/optimizer.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["optimize_mounted_pilot_source"],
            required_code_markers: &["__engine"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &["1024 * 1024"],
        },
        anticheat::FileAuditRule {
            path: "selfhost/bench/bench.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["compute_bench_stats_json"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/cli/main.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &["system("],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/cli/build_cmd.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &["system("],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/cli/test_runner.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["discover_arena_tests", "summarize_test_suite_json"],
            required_code_markers: &["__engine"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/cli/diff_test.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["check_differential_backends"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/cli/repl.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["repl_session_new", "repl_eval_line"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/pilotc.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &["system("],
            forbidden_string_substrings: &["pilot build"],
        },
    ],
    required_group_pub_fns: &[("parse_cli_build_args", CLI_BUILD_GROUP)],
};

fn eval_with_mounted_physics(driver_src: &str, target_src: &str, physics: &PhysicsRegistry) -> String {
    let drv_arena = parse(driver_src);
    assert!(
        drv_arena.diagnostics.is_empty(),
        "Driver parse diagnostics: {:?}",
        drv_arena.diagnostics
    );
    let mut reg = physics.clone();
    reg.source_text = target_src.to_string();
    let reg_ptr = &mut reg as *mut PhysicsRegistry;
    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(move |op, args| {
        handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, args)
    }));
    vm.load_binary(BytecodeCompiler::compile(&drv_arena, drv_arena.root));
    vm.run().to_string_repr()
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn from_entropy() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x2545_F491_4F6C_DD1D);
        let pid = std::process::id() as u64;
        Self {
            state: nanos ^ pid.rotate_left(17) ^ 0x85EB_CA6B_C2B2_AE35,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn range_i64(&mut self, lo: i64, hi_inclusive: i64) -> i64 {
        let span = (hi_inclusive - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }

    fn ident(&mut self, prefix: &str, len: usize) -> String {
        const CH: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let mut s = String::from(prefix);
        for _ in 0..len {
            s.push(CH[(self.next_u64() as usize) % CH.len()] as char);
        }
        s
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn escape_pilot_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
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
    out
}

fn python_sha256_hex(payload: &str) -> String {
    let mut child = Command::new("python3")
        .args(["-c", "import hashlib, sys; print(hashlib.sha256(sys.stdin.read().encode('utf-8')).hexdigest())"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(payload.as_bytes()).unwrap();
    String::from_utf8(child.wait_with_output().unwrap().stdout)
        .unwrap()
        .trim()
        .to_string()
}

#[test]
pub fn test_60_1_pkg_manifest_parser_real_sha256_lockfile_multifile_pilpkg_and_venv() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_60_1_pkg_manifest_parser_real_sha256_lockfile_multifile_pilpkg_and_venv",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();

    let hash_src = std::fs::read_to_string(root.join("stdlib/crypto/hash.pilot")).unwrap();
    let manifest_src = std::fs::read_to_string(root.join("selfhost/pkg/manifest.pilot")).unwrap();
    let venv_src = std::fs::read_to_string(root.join("selfhost/pkg/venv.pilot")).unwrap();
    let archive_src = std::fs::read_to_string(root.join("selfhost/pkg/archive.pilot")).unwrap();
    let interop_src = std::fs::read_to_string(root.join("selfhost/pkg/interop.pilot")).unwrap();

    for (nm, s) in [
        ("manifest.pilot", &manifest_src),
        ("venv.pilot", &venv_src),
        ("archive.pilot", &archive_src),
        ("interop.pilot", &interop_src),
    ] {
        assert!(s.lines().count() <= 1500, "selfhost/pkg/{} exceeds 1,500 LOC", nm);
    }

    let pkg_name = rng.ident("nav_core_", 6);
    let pkg_ver = format!("{}.{}.{}", rng.range_i64(1, 9), rng.range_i64(0, 20), rng.range_i64(0, 50));
    let dep_a = rng.ident("linalg_", 5);
    let dep_a_ver = format!("^{}.{}", rng.range_i64(1, 4), rng.range_i64(0, 9));
    let dep_b = rng.ident("telemetry_", 5);
    let dep_b_ver = format!("~{}.{}.0", rng.range_i64(2, 7), rng.range_i64(1, 9));

    let manifest_json = format!(
        "{{\n  \"name\": \"{}\",\n  \"version\": \"{}\",\n  \"dependencies\": {{\n    \"{}\": \"{}\",\n    \"{}\": \"{}\"\n  }}\n}}",
        pkg_name, pkg_ver, dep_a, dep_a_ver, dep_b, dep_b_ver
    );

    let file1_content = format!("pub fn f1() -> i64 {{ return {}; }}\n// pipe|colon:test\n", rng.range_i64(10, 999));
    let file2_content = format!("pub struct Data {{ pub v: i64; }}\n// {}\n", rng.ident("blob_", 20));
    let file3_content = format!("{{\"cfg\":\"{}\"}}", rng.ident("val_", 10));

    let script = format!(
        "{hash_src}\n{manifest_src}\n{venv_src}\n{archive_src}\n{interop_src}\n\
         let mf = parse_manifest_json(\"{mf_esc}\");\n\
         let m_nm = manifest_get_name(mf);\n\
         let m_vr = manifest_get_version(mf);\n\
         let m_cnt = manifest_dep_count(mf);\n\
         let m_da = manifest_dep_version(mf, \"{dep_a}\");\n\
         let m_db = manifest_dep_version(mf, \"{dep_b}\");\n\
         let main_code = scaffold_canonical_main_pilot(\"{pkg_name}\");\n\
         \n\
         let ar = pilpkg_new();\n\
         let _a1 = pilpkg_add_file(ar, \"src/lib.pilot\", \"{f1_esc}\");\n\
         let _a2 = pilpkg_add_file(ar, \"src/types.pilot\", \"{f2_esc}\");\n\
         let _a3 = pilpkg_add_file(ar, \"pilot.json\", \"{f3_esc}\");\n\
         let blob = pilpkg_serialize(ar);\n\
         let f_cnt = pilpkg_file_count(blob);\n\
         let ex1 = pilpkg_extract_file(blob, \"src/lib.pilot\");\n\
         let ex2 = pilpkg_extract_file(blob, \"src/types.pilot\");\n\
         let ex3 = pilpkg_extract_file(blob, \"pilot.json\");\n\
         let ex_miss = pilpkg_extract_file(blob, \"src/missing.pilot\");\n\
         \n\
         let lock = compute_package_lockfile_json(\"{pkg_name}\", \"{pkg_ver}\", blob);\n\
         let lock_ok = verify_lockfile_integrity(lock, blob);\n\
         let lock_tampered = !verify_lockfile_integrity(lock, blob + \"X\");\n\
         \n\
         let vcfg = format_pilotvenv_cfg(\".venv_nav\", \"{pkg_ver}\");\n\
         let vver = parse_pilotvenv_version(vcfg);\n\
         let i_cargo = parse_ecosystem_spec(\"cargo:{dep_a}@1.4.2\");\n\
         let i_pip = parse_ecosystem_spec(\"pip:{dep_b}@2.3.1\");\n\
         let i_npm = parse_ecosystem_spec(\"npm:vite@5.4.0\");\n\
         \n\
         let ar_ok = f_cnt == 3 && ex1 == \"{f1_esc}\" && ex2 == \"{f2_esc}\" && ex3 == \"{f3_esc}\" && ex_miss == \"\";\n\
         let mf_ok = m_nm == \"{pkg_name}\" && m_vr == \"{pkg_ver}\" && m_cnt == 2 && m_da == \"{dep_a_ver}\" && m_db == \"{dep_b_ver}\";\n\
         let __out = (if mf_ok {{ \"MF_OK\" }} else {{ \"MF_BAD\" }}) + \"@@PKG@@\"\n\
             + (if ar_ok {{ \"AR_OK\" }} else {{ \"AR_BAD\" }}) + \"@@PKG@@\"\n\
             + (if lock_ok && lock_tampered {{ \"LOCK_OK\" }} else {{ \"LOCK_BAD\" }}) + \"@@PKG@@\"\n\
             + blob + \"@@PKG@@\" + lock + \"@@PKG@@\" + main_code + \"@@PKG@@\"\n\
             + vver + \"@@PKG@@\" + i_cargo + \";\" + i_pip + \";\" + i_npm;\n\
         __out;\n",
        mf_esc = escape_pilot_str(&manifest_json),
        f1_esc = escape_pilot_str(&file1_content),
        f2_esc = escape_pilot_str(&file2_content),
        f3_esc = escape_pilot_str(&file3_content),
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "pkg suite parse diagnostics: {:?}", arena.diagnostics);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    let out = vm.run().to_string_repr();
    let cols: Vec<&str> = out.split("@@PKG@@").collect();
    assert_eq!(cols.len(), 8, "Unexpected pkg output: {}", out);

    assert_eq!(cols[0], "MF_OK", "pilot.json manifest parser failed");
    assert_eq!(cols[1], "AR_OK", "Multi-file .pilpkg pack/extract lossless round-trip failed");
    assert_eq!(cols[2], "LOCK_OK", "Lockfile SHA-256 integrity verification / tamper detection failed");

    let expected_sha = python_sha256_hex(cols[3]);
    assert!(
        cols[4].contains(&format!("sha256-{}", expected_sha)),
        "Lockfile must contain real SHA-256 digest sha256-{} (no FNV-1a cheats!), got:\n{}",
        expected_sha,
        cols[4]
    );

    // Scaffolded `main.pilot` must parse cleanly as canonical Pilot (`pub fn main`, zero `def `)
    assert!(!cols[5].contains("def "), "Scaffolded main.pilot used forbidden legacy `def`");
    let main_arena = parse(cols[5]);
    assert!(main_arena.diagnostics.is_empty(), "Scaffolded main.pilot failed parse: {:?}", main_arena.diagnostics);
    assert_eq!(cols[6], pkg_ver, "pilotvenv.cfg version round-trip failed");
    assert_eq!(
        cols[7],
        format!("cargo|{}|1.4.2;pip|{}|2.3.1;npm|vite|5.4.0", dep_a, dep_b),
        "Foreign ecosystem spec parser failed"
    );
    guard.finish();
}

#[test]
pub fn test_60_2_live_vmhook_flamegraph_svg_ast_wcet_and_real_constant_folding_optimizer() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_60_2_live_vmhook_flamegraph_svg_ast_wcet_and_real_constant_folding_optimizer",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();

    let prof_src = std::fs::read_to_string(root.join("selfhost/perf/profiler.pilot")).unwrap();
    let wcet_src = std::fs::read_to_string(root.join("selfhost/perf/wcet.pilot")).unwrap();
    let opt_src = std::fs::read_to_string(root.join("selfhost/perf/optimizer.pilot")).unwrap();
    let bench_src = std::fs::read_to_string(root.join("selfhost/bench/bench.pilot")).unwrap();

    assert!(
        !opt_src.contains("1024 * 1024"),
        "optimizer.pilot must not contain legacy '1024 * 1024' string replacement cheat"
    );

    // 1. Execute a REAL workload on PilVirtualMachine with VmHook to collect real PC call-stack samples!
    let hot_fn = rng.ident("hot_solver_", 5);
    let cold_fn = rng.ident("cold_init_", 5);
    let k_hot = rng.range_i64(110, 160);
    let k_cold = rng.range_i64(10, 18);

    let workload = format!(
        "pub fn {cold_fn}(n: i64) -> i64 {{\n\
             var s = 0;\n\
             var i = 0;\n\
             while i < n {{ s = s + i; i = i + 1; }}\n\
             return s;\n\
         }}\n\
         pub fn {hot_fn}(n: i64) -> i64 {{\n\
             var s = 1;\n\
             var i = 0;\n\
             while i < n {{ s = (s + i) ^ 3; i = i + 1; }}\n\
             return s;\n\
         }}\n\
         let a = {cold_fn}({k_cold});\n\
         let b = {hot_fn}({k_hot});\n\
         let __w = a + b;\n\
         __w;\n"
    );
    let w_arena = parse(&workload);
    assert!(w_arena.diagnostics.is_empty());
    // Map byte spans of `cold_fn` and `hot_fn` declarations so VmHook attributes every PC to its real function!
    let mut fn_ranges: Vec<(u32, u32, String)> = Vec::new();
    for (idx, node) in w_arena.nodes.iter().enumerate() {
        if node.node_kind() == NodeKind::FnDecl {
            let nm = w_arena.interner.resolve(SymbolId(node.lhs)).to_string();
            let sp = w_arena.spans[idx];
            fn_ranges.push((sp.start, sp.end, nm));
        }
    }
    let w_bin = BytecodeCompiler::compile(&w_arena, w_arena.root);
    let w_spans = w_bin.pc_spans.clone();
    let samples: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let samp_ref = Arc::clone(&samples);
    let mut w_vm = PilVirtualMachine::new();
    w_vm.set_hook(Some(Box::new(move |pc, _ins, _regs| {
        if let Some(sp) = w_spans.get(pc) {
            if sp.start > 0 || sp.end > 0 {
                let mut label = "main";
                for (s, e, nm) in &fn_ranges {
                    if sp.start >= *s && sp.start <= *e {
                        label = nm.as_str();
                        break;
                    }
                }
                if label != "main" {
                    samp_ref.lock().unwrap().push(format!("main;{} 1", label));
                }
            }
        }
    })));
    w_vm.load_binary(w_bin);
    let _ = w_vm.run();
    let folded_trace = samples.lock().unwrap().join("\n");
    assert!(folded_trace.contains(&hot_fn) && folded_trace.contains(&cold_fn));

    // Feed real `VmHook` trace + randomized benchmark samples into `profiler.pilot` & `bench.pilot`
    let b1 = rng.range_i64(110, 150);
    let b2 = rng.range_i64(160, 210);
    let b3 = rng.range_i64(220, 290);
    let b4 = rng.range_i64(310, 420);
    let b5 = rng.range_i64(500, 700);
    let samples_csv = format!("{},{},{},{},{}", b4, b1, b5, b2, b3);

    let prof_script = format!(
        "{prof_src}\n{bench_src}\n\
         let svg = profile_folded_stacks_to_svg(\"{trace_esc}\", 1000);\n\
         let chrome = profile_folded_stacks_to_chrome_trace(\"{trace_esc}\");\n\
         let bstats = compute_bench_stats_json(\"{samples_csv}\");\n\
         let __out = svg + \"@@PERF@@\" + chrome + \"@@PERF@@\" + bstats;\n\
         __out;\n",
        trace_esc = escape_pilot_str(&folded_trace),
    );
    let prof_arena = parse(&prof_script);
    assert!(prof_arena.diagnostics.is_empty(), "profiler/bench diagnostics: {:?}", prof_arena.diagnostics);
    let mut prof_vm = PilVirtualMachine::new();
    prof_vm.load_binary(BytecodeCompiler::compile(&prof_arena, prof_arena.root));
    let prof_out = prof_vm.run().to_string_repr();
    let pcols: Vec<&str> = prof_out.split("@@PERF@@").collect();
    assert_eq!(pcols.len(), 3);

    // Validate SVG XML & proportional flamegraph widths via Python `xml.etree.ElementTree`!
    let py_svg_check = format!(
        r#"
import json, sys, xml.etree.ElementTree as ET
payload = json.loads(sys.stdin.read())
root = ET.fromstring(payload["svg"])
svg_txt = payload["svg"]
assert "{hot}" in svg_txt and "{cold}" in svg_txt
# Extract rect widths associated with hot vs cold frames
rects = []
for elem in root.iter():
    if elem.tag.endswith("rect") and "width" in elem.attrib:
        rects.append(float(elem.attrib["width"]))
assert len(rects) >= 2, f"Expected >=2 rects, got {{rects}}"
events = json.loads(payload["chrome"])
ev_list = events["traceEvents"] if isinstance(events, dict) else events
hot_dur = sum(e.get("dur", 0) for e in ev_list if e.get("name") == "{hot}")
cold_dur = sum(e.get("dur", 0) for e in ev_list if e.get("name") == "{cold}")
assert hot_dur > 3 * cold_dur and cold_dur > 0, f"Chrome trace durations not proportional: hot={{hot_dur}}, cold={{cold_dur}}"
b = json.loads(payload["bench"])
assert b["count"] == 5 and b["min"] == {b1} and b["max"] == {b5} and b["median"] == {b3}
print("OK")
"#,
        hot = hot_fn,
        cold = cold_fn,
        b1 = b1,
        b3 = b3,
        b5 = b5,
    );
    let check_json = format!(
        "{{\"svg\":\"{}\",\"chrome\":\"{}\",\"bench\":\"{}\"}}",
        escape_pilot_str(pcols[0]),
        escape_pilot_str(pcols[1]),
        escape_pilot_str(pcols[2])
    );
    let mut py = Command::new("python3")
        .arg("-c")
        .arg(&py_svg_check)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    py.stdin.as_mut().unwrap().write_all(check_json.as_bytes()).unwrap();
    let py_res = py.wait_with_output().unwrap();
    assert!(
        py_res.status.success(),
        "Profiler SVG / Chrome Trace / Bench stats validation failed:\n{}",
        String::from_utf8_lossy(&py_res.stderr)
    );

    // 2. Test `wcet.pilot` across 3 mounted ASTs: bounded-pass, budget-exceeded, and unbounded `while`
    let wcet_driver = format!("{}\nlet __r = verify_arena_wcet();\n__r;\n", wcet_src);
    let ok_rt = "[HardRealtime]\n[Wcet(500)]\npub fn rt_ok() -> i64 {\n    var acc = 0;\n    for i in 0..10 { acc = acc + i; }\n    return acc;\n}\n";
    let over_rt = "[HardRealtime]\n[Wcet(25)]\npub fn rt_over() -> i64 {\n    var acc = 0;\n    for i in 0..80 { acc = acc + i * 3; }\n    return acc;\n}\n";
    let unb_rt = "[HardRealtime]\n[Wcet(5000)]\npub fn rt_unbounded(n: i64) -> i64 {\n    var i = 0;\n    while i < n { i = i + 1; }\n    return i;\n}\n";

    for (src_case, expected_prefix) in [
        (ok_rt, "PASS:"),
        (over_rt, "FAIL:WCET_EXCEEDED"),
        (unb_rt, "FAIL:UNBOUNDED_LOOP"),
    ] {
        let case_arena = parse(src_case);
        assert!(case_arena.diagnostics.is_empty());
        let analysis = AnalysisPipeline::analyze(&case_arena, case_arena.root);
        let res = eval_with_mounted_physics(&wcet_driver, src_case, &analysis.physics);
        assert!(
            res.starts_with(expected_prefix),
            "WCET verifier expected prefix '{}' on:\n{}\nGot: '{}'",
            expected_prefix,
            src_case,
            res
        );
    }

    // 3. Test `optimizer.pilot`: real recursive AST constant folding + dead `if false` elimination
    let c_a = rng.range_i64(13, 47);
    let c_b = rng.range_i64(11, 39);
    let c_c = rng.range_i64(100, 400);
    let c_d = rng.range_i64(15, 85);
    let expected_const = (c_a * c_b) + (c_c - c_d);
    let opt_fn = rng.ident("compute_folded_", 5);
    let x_arg = rng.range_i64(7, 60);

    let unopt_pilot = format!(
        "pub fn {opt_fn}(x: i64) -> i64 {{\n\
             if false {{\n\
                 return 987654321;\n\
             }}\n\
             let k = ({c_a} * {c_b}) + ({c_c} - {c_d});\n\
             return x + k;\n\
         }}\n"
    );
    let unopt_arena = parse(&unopt_pilot);
    assert!(unopt_arena.diagnostics.is_empty());
    let unopt_analysis = AnalysisPipeline::analyze(&unopt_arena, unopt_arena.root);
    let opt_driver = format!("{}\nlet __opt = optimize_mounted_pilot_source();\n__opt;\n", opt_src);
    let opt_pilot = eval_with_mounted_physics(&opt_driver, &unopt_pilot, &unopt_analysis.physics);

    assert!(
        opt_pilot.contains(&expected_const.to_string()),
        "Optimized source must contain folded integer constant {}, got:\n{}",
        expected_const,
        opt_pilot
    );
    assert!(
        !opt_pilot.contains(&format!("{} * {}", c_a, c_b)) && !opt_pilot.contains("987654321"),
        "Optimized source failed to eliminate constant sub-expression or dead `if false` branch:\n{}",
        opt_pilot
    );

    // Execute optimized source on PilVirtualMachine and verify exact semantic equivalence!
    let exec_opt = format!("{}\nlet __v = {}({});\n__v;\n", opt_pilot, opt_fn, x_arg);
    let exec_arena = parse(&exec_opt);
    assert!(exec_arena.diagnostics.is_empty(), "Optimized Pilot source must parse cleanly: {:?}", exec_arena.diagnostics);
    let mut exec_vm = PilVirtualMachine::new();
    exec_vm.load_binary(BytecodeCompiler::compile(&exec_arena, exec_arena.root));
    assert_eq!(exec_vm.run().as_int(), Some(x_arg + expected_const));
    guard.finish();
}

#[test]
pub fn test_60_3_and_60_4_selfhost_cli_test_runner_diff_test_stateful_repl_and_crate_purge() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_60_3_and_60_4_selfhost_cli_test_runner_diff_test_stateful_repl_and_crate_purge",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();

    // 1. Verify retired crates & files are completely deleted
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    for retired in ["pilot-pkg", "pilot-perf", "pilot-bench"] {
        assert!(!root.join("crates").join(retired).exists(), "crates/{} must be deleted", retired);
        assert!(!cargo_toml.contains(retired), "Cargo.toml must not reference {}", retired);
    }
    assert!(!root.join("crates/pilot-cli/src/commands.rs").exists());

    // 2. Verify `pilot-cli` total Rust LOC <= 100
    let mut cli_loc = 0usize;
    for entry in std::fs::read_dir(root.join("crates/pilot-cli/src")).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().and_then(|e| e.to_str()) == Some("rs") {
            cli_loc += std::fs::read_to_string(&p).unwrap().lines().count();
        }
    }
    assert!(cli_loc <= 100, "crates/pilot-cli/src total LOC ({}) must be <= 100", cli_loc);

    let pilotc_src = std::fs::read_to_string(root.join("selfhost/pilotc.pilot")).unwrap();
    assert!(!pilotc_src.contains("system(") && !pilotc_src.contains("pilot build"));

    let cli_main = std::fs::read_to_string(root.join("selfhost/cli/main.pilot")).unwrap();
    let build_cmd = std::fs::read_to_string(root.join("selfhost/cli/build_cmd.pilot")).unwrap();
    let test_runner = std::fs::read_to_string(root.join("selfhost/cli/test_runner.pilot")).unwrap();
    let diff_test = std::fs::read_to_string(root.join("selfhost/cli/diff_test.pilot")).unwrap();
    let repl_src = std::fs::read_to_string(root.join("selfhost/cli/repl.pilot")).unwrap();

    // Enforce Name-Blind REPL rule: zero hardcoded Pilot scalar type names in `repl.pilot`
    for scalar in ["\"i64\"", "\"f64\"", "\"bool\"", "\"usize\"", "\"u8\""] {
        assert!(
            !repl_src.contains(scalar),
            "selfhost/cli/repl.pilot must be Name-Blind (found hardcoded {})",
            scalar
        );
    }

    // 3. Test AST test discovery + CLI argv parsing + differential backend detector + stateful REPL
    let t_fn_1 = rng.ident("test_math_", 5);
    let t_fn_2 = rng.ident("test_crypto_", 5);
    let helper_fn = rng.ident("helper_ignore_", 5);
    let suite_src = format!(
        "pub fn {helper_fn}() -> i64 {{ return 1; }}\n\
         pub fn {t_fn_1}() -> i64 {{ assert(2 + 2 == 4); return 0; }}\n\
         pub fn {t_fn_2}() -> i64 {{ assert(3 * 3 == 9); return 0; }}\n"
    );
    let suite_arena = parse(&suite_src);
    let suite_analysis = AnalysisPipeline::analyze(&suite_arena, suite_arena.root);

    let r1 = rng.range_i64(12, 60);
    let r2 = rng.range_i64(3, 9);
    let r3 = rng.range_i64(10, 80);
    let r4 = rng.range_i64(5, 25);
    let expected_repl = r1 + (r1 * r2 + r3 - r4);
    let rand_val = format!("{}", rng.range_i64(1000, 9999));
    let bad_val = format!("{}_bad", rand_val);

    let driver = format!(
        "{cli_main}\n{build_cmd}\n{test_runner}\n{diff_test}\n{repl_src}\n\
         let argv_parsed = parse_cli_build_args(\"build app/guidance.pilot --target wasm --release -o out/guidance.wasm\");\n\
         let discovered = discover_arena_tests();\n\
         let summary_json = summarize_test_suite_json(\"{t_fn_1}:1,{t_fn_2}:0,extra:1\");\n\
         let d_ok = check_differential_backends(\"{rand_val}\", \"{rand_val}\", \"{rand_val}\", \"{rand_val}\");\n\
         let d_c = check_differential_backends(\"{rand_val}\", \"{bad_val}\", \"{rand_val}\", \"{rand_val}\");\n\
         let d_llvm = check_differential_backends(\"{rand_val}\", \"{rand_val}\", \"{bad_val}\", \"{rand_val}\");\n\
         let d_wasm = check_differential_backends(\"{rand_val}\", \"{rand_val}\", \"{rand_val}\", \"{bad_val}\");\n\
         let sess = repl_session_new();\n\
         let _l1 = repl_eval_line(sess, \"let alpha = {r1};\");\n\
         let _l2 = repl_eval_line(sess, \"var beta = alpha * {r2} + {r3};\");\n\
         let _l3 = repl_eval_line(sess, \"beta = beta - {r4};\");\n\
         let repl_ans = repl_eval_line(sess, \"alpha + beta\");\n\
         let __out = argv_parsed + \"@@CLI@@\" + discovered + \"@@CLI@@\" + summary_json + \"@@CLI@@\"\n\
             + d_ok + \",\" + d_c + \",\" + d_llvm + \",\" + d_wasm + \"@@CLI@@\" + repl_ans;\n\
         __out;\n"
    );

    let out = eval_with_mounted_physics(&driver, &suite_src, &suite_analysis.physics);
    let cols: Vec<&str> = out.split("@@CLI@@").collect();
    assert_eq!(cols.len(), 5, "Unexpected CLI suite output: {}", out);

    assert_eq!(
        cols[0],
        "build|app/guidance.pilot|wasm|1|out/guidance.wasm",
        "CLI build argv parser mismatch"
    );
    assert!(
        cols[1].contains(&t_fn_1) && cols[1].contains(&t_fn_2) && !cols[1].contains(&helper_fn),
        "AST test runner discovery must include test_* functions and exclude helpers, got: {}",
        cols[1]
    );
    assert!(
        cols[2].contains("\"passed\":2") && cols[2].contains("\"failed\":1"),
        "Test runner JSON summary mismatch: {}",
        cols[2]
    );
    assert_eq!(
        cols[3],
        "PASS,MISMATCH:c,MISMATCH:llvm,MISMATCH:wasm",
        "Differential backend verifier failed to pinpoint diverging backend"
    );
    assert!(
        cols[4].contains(&expected_repl.to_string()),
        "Stateful REPL session expected {} across 4 lines, got: {}",
        expected_repl,
        cols[4]
    );
    guard.finish();
}
