//! F3 acceptance: errors reach the user, the CLI dispatches through its
//! registry, and `pilot test` really judges programs (`MILESTONES_100.md` F3).
//!
//! Expected outputs here are written by hand from the fixture sources in
//! `tests/pilot_test_fixtures/` and `tests/conformance/`, not taken from a run.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn pilot(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pilot"))
        .args(args)
        .current_dir(root())
        .output()
        .expect("spawn pilot binary")
}

fn text(out: &Output) -> (String, String, i32) {
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn check_reports_coded_diagnostics_without_running() {
    let (stdout, stderr, code) = text(&pilot(&["check", "tests/conformance/err_undefined_name.pilot"]));
    assert_eq!(code, 1, "{stderr}");
    assert_eq!(stdout, "", "check must not run the program");
    assert!(stderr.contains("error[DECL001]: undefined name `missing_value`"), "{stderr}");
    assert!(stderr.contains("--> tests/conformance/err_undefined_name.pilot:4:11"), "{stderr}");
    assert!(stderr.contains("print(missing_value);"), "the source line is shown: {stderr}");
}

#[test]
fn check_passes_a_valid_program_without_running_it() {
    // `stdin_lines.pilot` prints as soon as it runs; check prints only its summary.
    let (stdout, stderr, code) = text(&pilot(&["check", "tests/conformance/stdin_lines.pilot"]));
    assert_eq!(
        (code, stdout.as_str(), stderr.as_str()),
        (0, "check passed: 1 file(s), no errors\n", "")
    );
}

#[test]
fn check_reports_every_file() {
    let (_, stderr, code) = text(&pilot(&[
        "check",
        "tests/conformance/err_break_outside.pilot",
        "tests/conformance/err_continue_outside.pilot",
    ]));
    assert_eq!(code, 1);
    assert!(stderr.contains("error[FLOW003]"), "{stderr}");
    assert!(stderr.contains("error[FLOW004]"), "{stderr}");
    assert!(stderr.contains("2 error(s)"), "{stderr}");
}

#[test]
fn check_and_run_reject_missing_files_and_usage() {
    let (_, stderr, code) = text(&pilot(&["check", "tests/conformance/no_such_program.pilot"]));
    assert_eq!(code, 1);
    assert!(stderr.contains("error[MOD001]: cannot read file"), "{stderr}");
    assert_eq!(text(&pilot(&["check"])).2, 2);
    assert_eq!(text(&pilot(&["run"])).2, 2);
}

#[test]
fn unknown_command_is_rejected() {
    let (_, stderr, code) = text(&pilot(&["frobnicate"]));
    assert_ne!(code, 0);
    assert!(!stderr.is_empty());
}

/// The names of the registered commands, read from the CLI's JSON schema.
fn registered_commands() -> Vec<String> {
    let (schema, _, code) = text(&pilot(&["--schema-json"]));
    assert_eq!(code, 0);
    // A command entry is a `"name"` line followed by its `"category"`; flag
    // entries have no category.
    let lines: Vec<&str> = schema.lines().map(str::trim).collect();
    lines
        .windows(2)
        .filter(|w| w[1].starts_with("\"category\""))
        .filter_map(|w| w[0].strip_prefix("\"name\": \""))
        .filter_map(|l| l.split('"').next())
        .map(str::to_string)
        .collect()
}

#[test]
fn every_listed_command_works_or_says_it_is_not_implemented() {
    let commands = registered_commands();
    assert!(commands.len() >= 20, "schema lists {commands:?}");
    // Commands with a handler, each invoked so it has its job to do.
    let working: &[(&str, &[&str])] = &[
        ("run", &["run", "tests/pilot_test_fixtures/fixed_bug/still_fine.pilot"]),
        ("check", &["check", "tests/pilot_test_fixtures/fixed_bug/still_fine.pilot"]),
        ("test", &["test", "tests/pilot_test_fixtures/passing"]),
        ("gate", &["gate", "--only", "hygiene"]),
        ("bench", &["bench", "--filter", "no_such_test"]),
        ("emit", &["emit", "python", "tests/pilot_test_fixtures/fixed_bug/still_fine.pilot"]),
        ("ir", &["ir", "print", "tests/pilot_test_fixtures/fixed_bug/still_fine.pilot"]),
        ("completions", &["completions", "bash"]),
    ];
    for name in &commands {
        let out = match working.iter().find(|(n, _)| n == name) {
            Some((_, args)) => pilot(args),
            None => pilot(&[name.as_str()]),
        };
        let (stdout, stderr, code) = text(&out);
        if working.iter().any(|(n, _)| n == name) {
            assert_eq!(code, 0, "`pilot {name}` has a handler and must succeed\n{stdout}{stderr}");
            assert!(!stdout.is_empty(), "`pilot {name}` printed nothing");
        } else {
            assert_ne!(code, 0, "`pilot {name}` has no handler and must fail");
            assert!(
                stderr.contains(&format!("`pilot {name}` is not implemented yet")),
                "`pilot {name}` must say it is not implemented: {stderr}"
            );
            assert!(stderr.contains("SEAM001"), "{stderr}");
        }
    }
}

#[test]
fn check_fix_stops_at_its_seam() {
    let (_, stderr, code) = text(&pilot(&["check", "--fix", "tests/conformance/stdin_lines.pilot"]));
    assert_ne!(code, 0);
    assert!(stderr.contains("SEAM001: unconnected seam `cli.check_fix`"), "{stderr}");
}

#[test]
fn version_and_help_print() {
    let (stdout, _, code) = text(&pilot(&["version"]));
    assert_eq!(code, 0);
    assert!(stdout.starts_with("Pilot v"), "{stdout}");
    let (stdout, _, code) = text(&pilot(&[]));
    assert_eq!(code, 0);
    assert!(stdout.contains("Usage: pilot <command>"), "{stdout}");
}

const SUMMARY_CLEAN: &str = "RESULT: 1 passed, 0 failed, 0 known bugs reproduced, 0 known bugs fixed";

#[test]
fn test_runner_passes_args_and_stdin() {
    let (stdout, stderr, code) = text(&pilot(&["test", "tests/pilot_test_fixtures/passing"]));
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert!(stdout.contains(SUMMARY_CLEAN), "{stdout}");
}

#[test]
fn test_runner_reports_each_kind_of_failure() {
    let (stdout, _, code) = text(&pilot(&["test", "tests/pilot_test_fixtures/failing"]));
    assert_eq!(code, 1, "{stdout}");
    let dir = "tests/pilot_test_fixtures/failing";
    for expected in [
        format!("FAIL {dir}/wrong_stdout.pilot"),
        "stdout differs".to_string(),
        format!("FAIL {dir}/wrong_exit.pilot"),
        "exit code 3, expected 0".to_string(),
        format!("FAIL {dir}/missing_stderr.pilot"),
        "stderr lacks `something that never appears`".to_string(),
        format!("FAIL {dir}/no_expectation.pilot"),
        "RESULT: 0 passed, 4 failed, 0 known bugs reproduced, 0 known bugs fixed".to_string(),
    ] {
        assert!(stdout.contains(&expected), "missing `{expected}` in:\n{stdout}");
    }
}

#[test]
fn test_runner_runs_marked_functions_only() {
    let (stdout, _, code) = text(&pilot(&["test", "tests/pilot_test_fixtures/units.pilot"]));
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.contains("FAIL tests/pilot_test_fixtures/units.pilot::breaks"), "{stdout}");
    assert!(!stdout.contains("helper_is_not_a_test"), "{stdout}");
    assert!(stdout.contains("RESULT: 1 passed, 1 failed,"), "{stdout}");

    let (stdout, _, code) = text(&pilot(&["test", "tests/pilot_test_fixtures/units.pilot", "--filter", "adds"]));
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains(SUMMARY_CLEAN), "{stdout}");
}

#[test]
fn test_runner_flags_a_fixed_known_bug() {
    let (stdout, _, code) = text(&pilot(&["test", "tests/pilot_test_fixtures/fixed_bug"]));
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.contains("FIXED tests/pilot_test_fixtures/fixed_bug/known_bugs/now_works.pilot"), "{stdout}");
    assert!(stdout.contains("1 known bugs fixed"), "{stdout}");
}

#[test]
fn test_runner_rejects_missing_paths() {
    assert_eq!(text(&pilot(&["test", "tests/pilot_test_fixtures/nowhere.pilot"])).2, 2);
}
