//! ============================================================================
//! Milestone 1 Verification Suite:
//! Seed Unlock & Pure-Pilot Architectural Gate Runner (`pilot gate`)
//! ============================================================================

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 1,
    test_file_rel: "crates/pilot-analysis/tests/milestone01_seed_unlock_and_pilot_gate.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/gate/manifest.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "seed_token_registry_new",
                "seed_registry_count",
                "seed_registry_contains",
                "build_crate_ceiling_registry",
                "temp_guard_new",
                "temp_guard_cleanup",
            ],
            required_code_markers: &["SeedTokenRegistry", "CrateCeilingRule", "GateViolation", "GateReport", "IGateCheck", "TempDirGuard"],
            forbidden_code_markers: &["return 14;"],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/gate/seed_audit.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "harvest_core_vocabulary",
                "verify_rust_source_name_blind",
                "execute_adaptability_gate",
                "execute_hygiene_and_ratchet_gate",
            ],
            required_code_markers: &["GATE001", "GATE002", "GATE003", "GATE004", "GATE005", "GATE006", "GATE007"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/gate/rename_litmus.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "rl_rename_pilot_identifiers",
                "execute_universal_rename_litmus",
                "execute_dual_pass_rename_gate",
            ],
            required_code_markers: &["GATE008", "temp_guard_new", "temp_guard_cleanup"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/gate/bootstrap_parity.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "verify_stage3_fixed_point_digests",
                "execute_bootstrap_parity_gate",
            ],
            required_code_markers: &["GATE009", "sha256_hex", "pilotc_bootstrap_stage_c11", "pilotc_bootstrap_stage_pilc_hex"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/gate/runner.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "parse_gate_cli_args",
                "run_gate_by_id",
                "run_all_gates",
                "format_gate_reports_json",
                "run_pilot_gate_cli",
            ],
            required_code_markers: &["build_gate_registry"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn workspace_root() -> PathBuf {
    anticheat::workspace_root()
}

fn load_gate_bundle(root: &Path) -> String {
    let deps = [
        "stdlib/crypto/hash.pilot",
        "selfhost/lexer.pilot",
        "selfhost/ast.pilot",
        "selfhost/parser.pilot",
        "selfhost/pilotc.pilot",
        "selfhost/gate/manifest.pilot",
        "selfhost/gate/seed_audit.pilot",
        "selfhost/gate/rename_litmus.pilot",
        "selfhost/gate/bootstrap_parity.pilot",
        "selfhost/gate/runner.pilot",
    ];
    let mut combined = String::with_capacity(96 * 1024);
    for rel in deps {
        let s = std::fs::read_to_string(root.join(rel))
            .unwrap_or_else(|e| panic!("Failed reading {}: {}", rel, e));
        combined.push_str(&s);
        combined.push('\n');
    }
    combined
}

fn escape_pilot_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
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

fn eval_gate_script(bundle: &str, tail: &str) -> String {
    let full = format!("{bundle}\n{tail}\n");
    let arena = parse(&full);
    assert!(
        arena.diagnostics.is_empty(),
        "Gate bundle parse diagnostics: {:?}",
        arena.diagnostics
    );
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    vm.run().to_string_repr()
}

fn ensure_pilot_cli_bin(root: &Path) -> PathBuf {
    let bin = root.join("target/debug/pilot");
    if !bin.exists() {
        let st = Command::new("cargo")
            .args(["build", "-q", "-p", "pilot-cli"])
            .current_dir(root)
            .status()
            .expect("cargo build -p pilot-cli");
        assert!(st.success(), "Failed building pilot-cli binary");
    }
    bin
}

#[test]
pub fn test_m1_1_dynamic_vocabulary_harvester_and_fast_gate001_slice_injection() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m1_1_dynamic_vocabulary_harvester_and_fast_gate001_slice_injection",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let bundle = load_gate_bundle(&root);
    let root_esc = escape_pilot_str(root.to_str().unwrap());

    // Pre-compile the gate VM once with a deliberate `"slice"` injection into a copy of `infer.rs`
    let real_infer = std::fs::read_to_string(root.join("crates/pilot-analysis/src/infer.rs")).unwrap();
    let mut lines: Vec<&str> = real_infer.lines().collect();
    let inject_line_1based = 12usize;
    let injected_stmt = "    let _forbidden_probe = \"slice\";";
    let expected_col_1based = injected_stmt.find("\"slice\"").unwrap() + 1;
    lines.insert(inject_line_1based - 1, injected_stmt);
    let mutated_infer = lines.join("\n");

    let host_rust_clean = "pub fn clean_host(arr: &[u8], idx: usize) -> bool {\n    // \"slice\" inside comment must be ignored\n    let ch = '\"';\n    idx < arr.len() && ch == '\"'\n}\n";
    let dead_and_todo_rust = "#[allow(dead_code)]\npub fn bad_seed() -> usize {\n    let leaked = \"RangeWindow\";\n    todo!()\n}\n";

    let tail = format!(
        "let sreg = seed_token_registry_new();\n\
         let scnt = seed_registry_count(sreg);\n\
         let vocab = harvest_core_vocabulary(\"{root_esc}\", sreg);\n\
         let t_start = gate_time_now_us();\n\
         let v_inj = verify_rust_source_name_blind(\"crates/pilot-analysis/src/infer.rs\", \"{inj_esc}\", sreg, vocab);\n\
         let dt_us = gate_time_now_us() - t_start;\n\
         let v_clean = verify_rust_source_name_blind(\"crates/pilot-rt/src/clean_probe.rs\", \"{clean_esc}\", sreg, vocab);\n\
         let v_bad = verify_rust_source_name_blind(\"crates/pilot-codegen/src/lib.rs\", \"{bad_esc}\", sreg, vocab);\n\
         let v0 = glist_get(v_inj, 0);\n\
         let b0 = glist_get(v_bad, 0);\n\
         let b1 = glist_get(v_bad, 1);\n\
         let b2 = glist_get(v_bad, 2);\n\
         let __res = \"\" + scnt + \"|\" + dt_us + \"|\" + glist_len(v_inj) + \"|\" + v0.code + \":\" + v0.line + \":\" + v0.column + \":\" + v0.offending_snippet\n\
             + \"|\" + glist_len(v_clean) + \"|\" + glist_len(v_bad) + \":\" + b0.code + \",\" + b1.code + \",\" + b2.code;\n\
         __res;",
        inj_esc = escape_pilot_str(&mutated_infer),
        clean_esc = escape_pilot_str(host_rust_clean),
        bad_esc = escape_pilot_str(dead_and_todo_rust),
    );

    let wall_t0 = Instant::now();
    let out = eval_gate_script(&bundle, &tail);
    let _wall_ms = wall_t0.elapsed().as_millis();

    let parts: Vec<&str> = out.split('|').collect();
    assert_eq!(parts.len(), 6, "Unexpected audit output: {}", out);
    assert_eq!(parts[0], "14", "SeedTokenRegistry must dynamically count 14 hardware seed tokens");

    let scan_us: u64 = parts[1].parse().expect("scan elapsed_us");
    assert!(
        scan_us < 200_000,
        "verify_rust_source_name_blind on infer.rs took {} us (must be < 200,000 us / 200 ms)",
        scan_us
    );
    assert_eq!(parts[2], "1", "Expected 1 GATE001 violation on injected infer.rs, got {}", out);
    assert_eq!(
        parts[3],
        format!("GATE001:{}:{}:slice", inject_line_1based, expected_col_1based),
        "Expected exact line:col GATE001 detection for injected \"slice\""
    );
    assert_eq!(
        parts[4], "0",
        "Unquoted Rust host types (`usize`, `bool`, `u8`) and comments/char-literals must produce 0 false positives"
    );
    assert!(
        parts[5].starts_with("3:")
            && parts[5].contains("GATE004")
            && parts[5].contains("GATE001")
            && parts[5].contains("GATE005"),
        "Expected GATE004 (allow(dead_code)), GATE001 (RangeWindow), and GATE005 (todo!()) in bad probe, got: {}",
        parts[5]
    );
    guard.finish();
}

struct TempDirDropGuard(PathBuf);

impl Drop for TempDirDropGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
pub fn test_m1_2_rust_tamper_unlock_comment_freedom_and_retired_scripts_purged() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m1_2_rust_tamper_unlock_comment_freedom_and_retired_scripts_purged",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();

    // 1. Confirm all 5 superseded Python scripts are permanently deleted
    for retired_py in [
        "scripts/gate1_check.py",
        "scripts/gate2_check.py",
        "scripts/gate3_check.py",
        "scripts/gate4_selfhost_contraction.py",
        "scripts/convert_dunders.py",
    ] {
        assert!(
            !root.join(retired_py).exists(),
            "Legacy Python script '{}' must be permanently deleted in M1!",
            retired_py
        );
    }

    // 2. Confirm static FROZEN_FILE_HASHES lock is gone from `anticheat.rs`
    let anticheat_src = std::fs::read_to_string(root.join("crates/pilot-analysis/tests/common/anticheat.rs")).unwrap();
    assert!(
        !anticheat_src.contains("FROZEN_FILE_HASHES") && !anticheat_src.contains("ILLEGAL_RUST_OR_GATE_MODIFICATION"),
        "Frozen SHA-256 Rust tamper lock must be retired in anticheat.rs"
    );

    // 3. Create an isolated workspace overlay, inject a dynamic comment into `crates/pilot-cli/src/main.rs`,
    //    and prove `pilot gate` succeeds with 0 violations while semantic checks remain active!
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let tmp_ws = std::env::temp_dir().join(format!(
        "pilot_m1_comment_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _cleanup = TempDirDropGuard(tmp_ws.clone());
    std::fs::create_dir_all(tmp_ws.join("crates/pilot-cli/src")).unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("stdlib"), tmp_ws.join("stdlib")).unwrap();
        std::os::unix::fs::symlink(root.join("selfhost"), tmp_ws.join("selfhost")).unwrap();
        for ent in std::fs::read_dir(root.join("crates")).unwrap().flatten() {
            let cname = ent.file_name();
            if cname.to_string_lossy() != "pilot-cli" {
                std::os::unix::fs::symlink(ent.path(), tmp_ws.join("crates").join(cname)).unwrap();
            }
        }
    }
    let orig_cli = std::fs::read_to_string(root.join("crates/pilot-cli/src/main.rs")).unwrap();
    let mutated_cli = format!("// M1 dynamic comment-freedom probe\n{}", orig_cli);
    assert!(mutated_cli.lines().count() <= 100, "Mutated main.rs must stay <= 100 LOC");
    std::fs::write(tmp_ws.join("crates/pilot-cli/src/main.rs"), &mutated_cli).unwrap();

    let bundle = load_gate_bundle(&root);
    let tmp_esc = escape_pilot_str(tmp_ws.to_str().unwrap());
    let res = eval_gate_script(
        &bundle,
        &format!("run_pilot_gate_cli(\"{tmp_esc}\", \"--json\");"),
    );
    let (exit_code, json_body) = res.split_once('\n').expect("exit_code + json");
    assert_eq!(exit_code, "0", "Comment edit in main.rs must pass pilot gate! Got: {}", json_body);
    assert!(json_body.contains("\"passed\":true") && json_body.contains("\"total_violations\":0"));

    guard.finish();
}

#[test]
pub fn test_m1_3_dual_pass_universal_rename_litmus_and_temp_dir_guard_cleanup() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m1_3_dual_pass_universal_rename_litmus_and_temp_dir_guard_cleanup",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let bundle = load_gate_bundle(&root);
    let root_esc = escape_pilot_str(root.to_str().unwrap());

    let seed = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as i64)
        + 104_729;

    let res = eval_gate_script(
        &bundle,
        &format!(
            "let opts = parse_gate_cli_args(\"--only rename_litmus --seed {seed} --json\");\n\
             let reps = run_all_gates(\"{root_esc}\", opts);\n\
             format_gate_reports_json(reps);"
        ),
    );
    assert!(
        res.contains("\"id\":\"rename_litmus\"")
            && res.contains("\"passed\":true")
            && res.contains("\"total_violations\":0"),
        "Dual-Pass Universal Rename Litmus failed for seed {}: {}",
        seed,
        res
    );

    // Verify TempDirGuard deterministic cleanup on a dedicated probe arena
    let probe_dir = format!("/tmp/pilot_litmus_guard_probe_{}_{}", std::process::id(), seed);
    let probe_esc = escape_pilot_str(&probe_dir);
    let guard_probe = eval_gate_script(
        &bundle,
        &format!(
            "let g = temp_guard_new(\"{probe_esc}\");\n\
             let _ = gate_fs_write_text(\"{probe_esc}/marker.txt\", \"ok\");\n\
             let existed = gate_fs_exists(\"{probe_esc}/marker.txt\");\n\
             let _ = temp_guard_cleanup(g);\n\
             let gone = !gate_fs_exists(\"{probe_esc}\");\n\
             \"\" + existed + \"|\" + gone;"
        ),
    );
    assert_eq!(guard_probe, "true|true", "TempDirGuard failed deterministic cleanup");
    assert!(!Path::new(&probe_dir).exists(), "TempDirGuard leaked {}", probe_dir);
    guard.finish();
}

#[test]
pub fn test_m1_4_cli_pilot_gate_full_json_exit_codes_and_stage3_sha256_parity() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m1_4_cli_pilot_gate_full_json_exit_codes_and_stage3_sha256_parity",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let bin = ensure_pilot_cli_bin(&root);

    // 1. `pilot gate` fast default (< 500 ms inside pure-Pilot engine)
    let fast_out = Command::new(&bin)
        .args(["gate", "--json"])
        .current_dir(&root)
        .output()
        .expect("run pilot gate --json");
    assert!(
        fast_out.status.success(),
        "pilot gate --json failed:\n{}",
        String::from_utf8_lossy(&fast_out.stdout)
    );
    let fast_json = String::from_utf8(fast_out.stdout).unwrap();
    assert!(
        fast_json.contains("\"passed\":true")
            && fast_json.contains("\"gate_count\":2")
            && fast_json.contains("\"total_violations\":0"),
        "Unexpected fast gate JSON: {}",
        fast_json
    );

    // 2. `pilot gate --full` (runs all 4 gates: adaptability, hygiene, rename_litmus, bootstrap_parity)
    let full_out = Command::new(&bin)
        .args(["gate", "--full", "--seed", "77777"])
        .current_dir(&root)
        .output()
        .expect("run pilot gate --full");
    assert!(
        full_out.status.success(),
        "pilot gate --full failed:\n{}",
        String::from_utf8_lossy(&full_out.stdout)
    );
    let full_txt = String::from_utf8(full_out.stdout).unwrap();
    assert!(
        full_txt.contains("ALL 4 GATE(S) PASSED (0 violations)"),
        "Expected all 4 gates to pass in --full mode, got:\n{}",
        full_txt
    );

    // 3. Invalid CLI flag must return exit code `2`
    let bad_out = Command::new(&bin)
        .args(["gate", "--not-a-valid-flag"])
        .current_dir(&root)
        .output()
        .expect("run pilot gate --not-a-valid-flag");
    assert_eq!(
        bad_out.status.code(),
        Some(2),
        "Invalid CLI flags on `pilot gate` must return exit code 2"
    );

    // 4. Direct Stage-2 == Stage-3 SHA-256 fixed-point digest equality check
    let bundle = load_gate_bundle(&root);
    let digests = eval_gate_script(&bundle, "verify_stage3_fixed_point_digests(4242);");
    let dcols: Vec<&str> = digests.split('|').collect();
    assert_eq!(dcols.len(), 4, "Expected 4 SHA-256 digests: {}", digests);
    assert_eq!(dcols[0].len(), 64);
    assert_eq!(dcols[0], dcols[1], "Stage-2 != Stage-3 C11 SHA-256 digest");
    assert_eq!(dcols[2].len(), 64);
    assert_eq!(dcols[2], dcols[3], "Stage-2 != Stage-3 .pilc SHA-256 digest");

    guard.finish();
}
