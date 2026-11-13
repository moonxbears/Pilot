//! `pilot test --changed`: only the cases a change reaches run
//! (`selfhost/cli/test_changed.pilot`).
//!
//! The expected selections are worked out by hand: from what `git show
//! --name-only` lists for a commit of this repository, and from the `use`
//! declarations of small fixture programs in a scratch git repository.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn pilot_in(dir: &Path, args: &[&str]) -> (String, String, i32) {
    let out: Output = Command::new(env!("CARGO_BIN_EXE_pilot"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn pilot binary");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "user.name=pilot-test", "-c", "user.email=pilot-test@example.invalid"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn git");
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create directory");
    for entry in fs::read_dir(from).expect("read directory") {
        let entry = entry.expect("directory entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy file");
        }
    }
}

/// A scratch toolchain root under git: a copy of `stdlib/` and two cases,
/// `uses_csv` (imports `std.csv`) and `plain` (imports nothing), committed.
fn scratch_repo(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pilot_changed_{name}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    copy_dir(&root().join("stdlib"), &dir.join("stdlib"));
    let cases = dir.join("tests/conformance");
    fs::create_dir_all(&cases).expect("create cases");
    fs::write(cases.join("uses_csv.pilot"), "use std.csv;\n\nfn main() {\n    print(\"csv\");\n}\n").unwrap();
    fs::write(cases.join("uses_csv.stdout"), "csv\n").unwrap();
    fs::write(cases.join("plain.pilot"), "fn main() {\n    print(\"plain\");\n}\n").unwrap();
    fs::write(cases.join("plain.stdout"), "plain\n").unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn result_line(stdout: &str) -> &str {
    stdout.lines().find(|l| l.starts_with("RESULT")).unwrap_or_else(|| panic!("no result line: {stdout}"))
}

/// The cases listed under the `CHANGED` line.
fn picked(stdout: &str) -> Vec<&str> {
    stdout
        .lines()
        .skip_while(|l| !l.starts_with("CHANGED"))
        .skip(1)
        .map_while(|l| l.strip_prefix("  "))
        .collect()
}

#[test]
fn a_stdlib_change_runs_only_the_cases_that_link_it() {
    let dir = scratch_repo("stdlib");
    let csv = dir.join("stdlib/std/csv.pilot");
    let mut src = fs::read_to_string(&csv).unwrap();
    src.push_str("\n// changed\n");
    fs::write(&csv, src).unwrap();
    let (stdout, stderr, code) = pilot_in(&dir, &["test", "--changed"]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert!(stdout.contains("CHANGED 1 files since HEAD: 1 of 2 cases reached"), "{stdout}");
    assert_eq!(picked(&stdout), ["tests/conformance/uses_csv.pilot"], "{stdout}");
    assert_eq!(result_line(&stdout), "RESULT: 1 passed, 0 failed, 0 known bugs reproduced, 0 known bugs fixed");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_changed_expectation_file_runs_its_case_and_judges_it() {
    let dir = scratch_repo("expect");
    // A wrong expectation: the case must run, and fail.
    fs::write(dir.join("tests/conformance/plain.stdout"), "not plain\n").unwrap();
    let (stdout, stderr, code) = pilot_in(&dir, &["test", "--changed"]);
    assert_eq!(code, 1, "{stdout}{stderr}");
    assert_eq!(picked(&stdout), ["tests/conformance/plain.pilot"], "{stdout}");
    assert!(result_line(&stdout).starts_with("RESULT: 0 passed, 1 failed"), "{stdout}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_untracked_case_is_run() {
    let dir = scratch_repo("untracked");
    let cases = dir.join("tests/conformance");
    fs::write(cases.join("fresh.pilot"), "fn main() {\n    print(1 + 2);\n}\n").unwrap();
    fs::write(cases.join("fresh.stdout"), "3\n").unwrap();
    let (stdout, stderr, code) = pilot_in(&dir, &["test", "--changed"]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert_eq!(picked(&stdout), ["tests/conformance/fresh.pilot"], "{stdout}");
    assert!(result_line(&stdout).starts_with("RESULT: 1 passed, 0 failed"), "{stdout}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn nothing_runs_when_no_case_is_reached() {
    let dir = scratch_repo("nothing");
    fs::write(dir.join("NOTES.md"), "notes\n").unwrap();
    let (stdout, stderr, code) = pilot_in(&dir, &["test", "--changed"]);
    assert_eq!((code, stderr.as_str()), (0, ""), "{stdout}");
    assert_eq!(stdout, "CHANGED 1 files since HEAD: 0 of 2 cases reached\n");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_revision_git_does_not_know_is_an_error() {
    let dir = scratch_repo("badrev");
    let (_, stderr, code) = pilot_in(&dir, &["test", "--changed", "--since", "no-such-revision"]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("error: `git diff --name-only no-such-revision` failed"), "{stderr}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn changed_runs_from_the_toolchain_root_only() {
    let (_, stderr, code) = pilot_in(&root().join("tests"), &["test", "--changed"]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("runs from the toolchain root"), "{stderr}");
}

// Commits of this repository. fcd1a7a changes only MILESTONES_100.md;
// 95f85ad changes selfhost/ir/lower.pilot and selfhost/gate/ledger/wiring.pilot.
// Either way the seed runs only the two cases the loader cannot link
// (err_import_cycle, err_import_missing), which always run.

#[test]
fn a_documentation_commit_runs_only_the_cases_that_never_link() {
    let (stdout, stderr, code) = pilot_in(&root(), &["test", "--changed", "--since", "fcd1a7a~1..fcd1a7a"]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert!(stdout.starts_with("CHANGED 1 files since fcd1a7a~1..fcd1a7a: 2 of "), "{stdout}");
    assert_eq!(
        picked(&stdout),
        ["tests/conformance/errors/err_import_missing.pilot", "tests/conformance/modules/err_import_cycle.pilot"],
        "{stdout}"
    );
}

#[test]
fn a_lowerer_change_reaches_every_ir_case_but_no_seed_case() {
    let since = "95f85ad~1..95f85ad";
    let (stdout, _, code) = pilot_in(&root(), &["test", "--changed", "--since", since]);
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(picked(&stdout).len(), 2, "{stdout}");
    // `--filter` keeps the run itself short; the choice is printed first.
    let (stdout, stderr, code) = pilot_in(&root(), &["test", "--ir", "--changed", "--since", since, "--filter", "no case has this name"]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert!(
        stdout.starts_with(&format!(
            "CHANGED 2 files since {since}: `selfhost/ir/lower.pilot` is a source of the tools this mode runs; running every case"
        )),
        "{stdout}"
    );
}
