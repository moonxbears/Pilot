//! `pilot emit <lang>` over the conformance corpus (`pilot test --emit <lang>`).
//!
//! - Each eligible conformance case is emitted from its PilIR, built and run
//!   with the host toolchain, and must give the stdout, exit code and runtime
//!   error of `pilot run`, or stop at a registered `emit.*` / `ir.*` seam.
//! - The runner reports a case whose emitted program prints the wrong output.

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

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `pilot test --emit <lang>` passes, and every case that did not run names
/// the seam it stopped at.
fn corpus_passes(lang: &str) {
    let out = pilot(&["test", "--emit", lang]);
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}{}", text(&out.stderr));
    let head = format!("RESULT (emit {lang}):");
    let result = stdout.lines().find(|l| l.starts_with(&head)).expect("a result line");
    assert!(result.contains(" 0 failed"), "{result}");
    for line in stdout.lines().filter(|l| l.starts_with("SEAM ")) {
        assert!(line.contains("SEAM001") && (line.contains("`emit.") || line.contains("`ir.")), "{line}");
    }
}

#[test]
#[ignore = "slow: runs the whole conformance corpus; `cargo test-full` runs it"]
fn the_corpus_runs_as_emitted_python() {
    corpus_passes("python");
}

#[test]
#[ignore = "slow: runs the whole conformance corpus; `cargo test-full` runs it"]
fn the_corpus_runs_as_emitted_javascript() {
    corpus_passes("js");
}

#[test]
#[ignore = "slow: runs the whole conformance corpus; `cargo test-full` runs it"]
fn the_corpus_runs_as_emitted_c_or_stops_at_a_seam() {
    corpus_passes("c");
}

#[test]
fn a_wrong_emitted_output_is_reported() {
    let dir = std::env::temp_dir().join(format!("pilot_cli_emit_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // 2 + 3 is 5: the first expectation is right, the second wrong.
    std::fs::write(dir.join("right.pilot"), "fn main() {\n    print(2 + 3);\n}\n").unwrap();
    std::fs::write(dir.join("right.stdout"), "5\n").unwrap();
    std::fs::write(dir.join("wrong.pilot"), "fn main() {\n    print(2 + 3);\n}\n").unwrap();
    std::fs::write(dir.join("wrong.stdout"), "6\n").unwrap();
    let out = pilot(&["test", "--emit", "python", dir.to_str().unwrap()]);
    let stdout = text(&out.stdout);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(out.status.code(), Some(1), "{stdout}{}", text(&out.stderr));
    assert!(stdout.contains("FAIL ") && stdout.contains("wrong.pilot"), "{stdout}");
    assert!(!stdout.contains("right.pilot"), "{stdout}");
    assert!(stdout.contains("RESULT (emit python): 1 passed, 1 failed"), "{stdout}");
}

#[test]
fn an_unknown_target_is_refused() {
    let out = pilot(&["test", "--emit", "cobol"]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out.stdout));
    assert!(text(&out.stderr).contains("no target `cobol`"), "{}", text(&out.stderr));
}
