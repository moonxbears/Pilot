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
//! 3. `selfhost/cli/{main,build_cmd,test_runner,diff_test}.pilot` & `selfhost/pilotc.pilot` (`<= 1,500` LOC each):
//!    - `cli/main.pilot` & `cli/build_cmd.pilot`:
//!      - `pub fn parse_cli_build_args(argv_line: str) -> str`
//!        (Parses e.g. `"build <input> --target <t> --release -o <out>"` -> `"<cmd>|<input>|<target>|<release_0_or_1>|<out>"`.)
//!    - `cli/test_runner.pilot`:
//!      - `pub fn discover_arena_tests() -> str` (`|`-joined names of `test_*` or `[Test]` `fn`s in mounted AST)
//!      - `pub fn summarize_test_suite_json(results_csv: str) -> str` (e.g. `"t1:1,t2:0,t3:1"` -> JSON with `"passed":2,"failed":1`)
//!    - `cli/diff_test.pilot`:
//!      - `pub fn check_differential_backends(vm_out: str, c_out: str, llvm_out: str, wasm_out: str) -> str`
//!        (Returns `"PASS"` iff all 4 match; otherwise `"MISMATCH:<backend_name>"` naming the first diverging backend.)

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const CLI_BUILD_GROUP: &[&str] = &["selfhost/cli/main.pilot", "selfhost/cli/build_cmd.pilot"];

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 60,
    test_file_rel: "crates/pilot-analysis/tests/milestone60_selfhost_cli_pkg_perf_and_bench.rs",
    expected_test_count: 1,
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
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
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
        assert!(
            s.lines().count() <= 1500,
            "selfhost/pkg/{} exceeds 1,500 LOC",
            nm
        );
    }

    let pkg_name = rng.ident("nav_core_", 6);
    let pkg_ver = format!(
        "{}.{}.{}",
        rng.range_i64(1, 9),
        rng.range_i64(0, 20),
        rng.range_i64(0, 50)
    );
    let dep_a = rng.ident("linalg_", 5);
    let dep_a_ver = format!("^{}.{}", rng.range_i64(1, 4), rng.range_i64(0, 9));
    let dep_b = rng.ident("telemetry_", 5);
    let dep_b_ver = format!("~{}.{}.0", rng.range_i64(2, 7), rng.range_i64(1, 9));

    let manifest_json = format!(
        "{{\n  \"name\": \"{}\",\n  \"version\": \"{}\",\n  \"dependencies\": {{\n    \"{}\": \"{}\",\n    \"{}\": \"{}\"\n  }}\n}}",
        pkg_name, pkg_ver, dep_a, dep_a_ver, dep_b, dep_b_ver
    );

    let file1_content = format!(
        "pub fn f1() -> i64 {{ return {}; }}\n// pipe|colon:test\n",
        rng.range_i64(10, 999)
    );
    let file2_content = format!(
        "pub struct Data {{ pub v: i64; }}\n// {}\n",
        rng.ident("blob_", 20)
    );
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
    assert!(
        arena.diagnostics.is_empty(),
        "pkg suite parse diagnostics: {:?}",
        arena.diagnostics
    );
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    let out = vm.run().to_string_repr();
    let cols: Vec<&str> = out.split("@@PKG@@").collect();
    assert_eq!(cols.len(), 8, "Unexpected pkg output: {}", out);

    assert_eq!(cols[0], "MF_OK", "pilot.json manifest parser failed");
    assert_eq!(
        cols[1], "AR_OK",
        "Multi-file .pilpkg pack/extract lossless round-trip failed"
    );
    assert_eq!(
        cols[2], "LOCK_OK",
        "Lockfile SHA-256 integrity verification / tamper detection failed"
    );

    let expected_sha = python_sha256_hex(cols[3]);
    assert!(
        cols[4].contains(&format!("sha256-{}", expected_sha)),
        "Lockfile must contain real SHA-256 digest sha256-{} (no FNV-1a cheats!), got:\n{}",
        expected_sha,
        cols[4]
    );

    // Scaffolded `main.pilot` must parse cleanly as canonical Pilot (`pub fn main`, zero `def `)
    assert!(
        !cols[5].contains("def "),
        "Scaffolded main.pilot used forbidden legacy `def`"
    );
    let main_arena = parse(cols[5]);
    assert!(
        main_arena.diagnostics.is_empty(),
        "Scaffolded main.pilot failed parse: {:?}",
        main_arena.diagnostics
    );
    assert_eq!(cols[6], pkg_ver, "pilotvenv.cfg version round-trip failed");
    assert_eq!(
        cols[7],
        format!("cargo|{}|1.4.2;pip|{}|2.3.1;npm|vite|5.4.0", dep_a, dep_b),
        "Foreign ecosystem spec parser failed"
    );
    guard.finish();
}
