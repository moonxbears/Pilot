//! ============================================================================
//! Milestone 1 Verification Suite:
//! Seed Unlock & Pure-Pilot Architectural Gate Runner (`pilot gate`)
//! ============================================================================

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/vm_outcome.rs"]
mod vm_outcome;
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
                "gate_seam_open",
                "gate_at_seam_report",
            ],
            required_code_markers: &["SeedTokenRegistry", "CrateCeilingRule", "GateViolation", "GateReport", "IGateCheck", "TempDirGuard", "at_seam"],
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
            required_code_markers: &["GATE008", "temp_guard_new", "temp_guard_cleanup", "gate_seam_open", "rl_run_program"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/gate/bootstrap_parity.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["execute_bootstrap_parity_gate"],
            required_code_markers: &["GATE009", "sha256_hex", "pilotc_bootstrap_stage_c11", "gate_seam_open"],
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
        "selfhost/gate/ledger/seams.pilot",
        "selfhost/gate/ledger/wiring.pilot",
        "selfhost/gate/seam_gate.pilot",
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
    vm_outcome::outcome(&mut vm)
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
        std::os::unix::fs::symlink(root.join("tests"), tmp_ws.join("tests")).unwrap();
        for ent in std::fs::read_dir(root.join("crates")).unwrap().flatten() {
            let cname = ent.file_name();
            if cname.to_string_lossy() != "pilot-cli" {
                std::os::unix::fs::symlink(ent.path(), tmp_ws.join("crates").join(cname)).unwrap();
            }
        }
    }
    let orig_cli = std::fs::read_to_string(root.join("crates/pilot-cli/src/main.rs")).unwrap();
    let mutated_cli = format!("// M1 dynamic comment-freedom probe\n{}", orig_cli);
    let cli_cap = anticheat::seed_rust_file_loc_cap(&root, 100);
    assert!(mutated_cli.lines().count() <= cli_cap, "Mutated main.rs must stay <= {cli_cap} LOC");
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

    // 1. The renamer: whole identifiers only, never inside comments, string
    //    or char literals, never a longer identifier. Expected text is
    //    written by hand.
    let input = "// a slice comment\n\
                 fn f(xs: slice<char>, o: opt<range<i64>>) -> result<str, res> {\n\
                 \x20   let slices = xs.slice(0, 1);\n\
                 \x20   let t = \"slice range\";\n\
                 \x20   let c = 'r';\n\
                 \x20   /* option */ return option.none;\n\
                 }\n";
    let expected = "// a slice comment\n\
                    fn f(xs: window<c32>, o: maybe<interval<i64>>) -> outcome<str, outcome> {\n\
                    \x20   let slices = xs.window(0, 1);\n\
                    \x20   let t = \"slice range\";\n\
                    \x20   let c = 'r';\n\
                    \x20   /* option */ return maybe.none;\n\
                    }\n";
    let renamed = eval_gate_script(
        &bundle,
        &format!(
            "let rmap = rl_pairs_to_map(build_canonical_litmus_pairs());\n\
             rl_rename_pilot_identifiers(\"{}\", rmap);",
            escape_pilot_str(input)
        ),
    );
    assert_eq!(renamed, expected, "Pass-0 identifier renamer");

    // 2. Pass 1 renames to seeded names: the same seed gives the same names,
    //    and no seeded name is a canonical one.
    let seed = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as i64)
        + 104_729;
    let seeded = |s: i64| {
        eval_gate_script(
            &bundle,
            &format!(
                "let rmap = rl_pairs_to_map(build_seeded_litmus_pairs({s}));\n\
                 rl_rename_pilot_identifiers(\"slice char\", rmap);"
            ),
        )
    };
    let first = seeded(seed);
    assert_eq!(first, seeded(seed), "seeded renames must be deterministic");
    let parts: Vec<&str> = first.split(' ').collect();
    assert_eq!(parts.len(), 2, "seeded rename output: {first}");
    assert!(
        parts[0].starts_with("window_") && parts[1].starts_with("c32_") && parts[0].len() == "window_".len() + 6,
        "seeded rename output: {first}"
    );

    // 3. The gate builds the renamed sandbox, then stops at the seam that
    //    runs programs (no process spawn until F6). It must not pass.
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
            && res.contains("\"passed\":false")
            && res.contains("\"at_seam\":\"gate.litmus_run_program\"")
            && res.contains("\"at_seams\":1")
            && res.contains("\"total_violations\":0"),
        "Rename litmus must stop at its seam for seed {}: {}",
        seed,
        res
    );
    let scanned: usize = res
        .split("\"items_scanned\":")
        .nth(1)
        .and_then(|t| t.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    let conformance_programs = std::fs::read_dir(root.join("tests/conformance"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "pilot") && p.with_extension("stdout").exists())
        .count();
    assert!(
        scanned >= 2 * conformance_programs,
        "the sandbox must hold every conformance program in both passes ({scanned} < 2 x {conformance_programs})"
    );
    let leftover = std::fs::read_dir("/tmp")
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| e.file_name().to_string_lossy().starts_with(&format!("pilot_litmus_{}_", std::process::id())));
    assert!(!leftover, "rename litmus leaked its sandbox under /tmp");

    // 4. Running a program stops at the registered seam.
    let run = eval_gate_script(&bundle, "rl_run_program(\"/tmp\", \"progs/x.pilot\", glist_new());");
    assert!(
        run.starts_with("FAULT: SEAM001: unconnected seam `gate.litmus_run_program`"),
        "rl_run_program must stop at its seam: {run}"
    );

    // 5. TempDirGuard deterministic cleanup on a dedicated probe arena
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
pub fn test_m1_4_cli_pilot_gate_full_json_exit_codes_and_seam_status() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m1_4_cli_pilot_gate_full_json_exit_codes_and_seam_status",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let bin = ensure_pilot_cli_bin(&root);

    // 1. `pilot gate` fast default
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
            && fast_json.contains("\"gate_count\":3")
            && fast_json.contains("\"at_seams\":0")
            && fast_json.contains("\"total_violations\":0"),
        "Unexpected fast gate JSON: {}",
        fast_json
    );

    // 2. `pilot gate --full`: the three source gates pass; the rename litmus
    //    and the bootstrap parity gate cannot run their checks yet and report
    //    the seams they stop at. Exit 0: no gate failed.
    let full_out = Command::new(&bin)
        .args(["gate", "--full", "--seed", "77777"])
        .current_dir(&root)
        .output()
        .expect("run pilot gate --full");
    let full_txt = String::from_utf8(full_out.stdout).unwrap();
    assert!(full_out.status.success(), "pilot gate --full failed:\n{}", full_txt);
    for line in [
        "[SEAM] rename_litmus (Adaptability) stops at seam `gate.litmus_run_program`",
        "[SEAM] bootstrap_parity (BootstrapParity) stops at seam `bootstrap.stage_compile`",
        "RESULT: 3 GATE(S) PASSED, 2 AT SEAMS (0 violations)",
    ] {
        assert!(full_txt.contains(line), "Expected `{}` in --full output:\n{}", line, full_txt);
    }
    assert!(!full_txt.contains("[FAIL]"), "--full output:\n{}", full_txt);

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

    // 4. The stage build stops at its seam, and a gate claiming a seam that
    //    is not registered fails.
    let bundle = load_gate_bundle(&root);
    let stage = eval_gate_script(&bundle, "pilotc_bootstrap_stage_c11(::Stage2, \"fn f() {}\");");
    assert!(
        stage.starts_with("FAULT: SEAM001: unconnected seam `bootstrap.stage_compile`"),
        "stage build must stop at its seam: {stage}"
    );
    let fake_seam = eval_gate_script(
        &bundle,
        "let reps = glist_new();\n\
         let _ = glist_push(reps, gate_at_seam_report(\"probe\", GateCategoryId.Hygiene, \"no_such.seam\", gate_time_now_us(), 0));\n\
         format_gate_reports_human(reps);",
    );
    assert!(
        fake_seam.contains("[FAIL] probe") && fake_seam.contains("RESULT: FAILED"),
        "an unregistered seam must fail the gate:\n{fake_seam}"
    );

    guard.finish();
}
