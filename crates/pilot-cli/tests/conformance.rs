//! Phase F conformance corpus (`MILESTONES_100.md` F1, F3).
//!
//! The corpus is run by `pilot test` (`selfhost/cli/test_cmd.pilot`), which
//! compares each `tests/conformance/<area>/<name>.pilot` program's stdout, exit code
//! and stderr with its expectation files and requires every
//! `tests/conformance/known_bugs/` program to keep failing. This test only
//! runs it from the workspace root; `cli_f3.rs` checks that the runner really
//! reports failures.

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

#[test]
#[ignore = "slow: runs the whole conformance corpus; `cargo test-full` runs it"]
fn conformance_corpus_passes_under_pilot_test() {
    let out = Command::new(env!("CARGO_BIN_EXE_pilot"))
        .arg("test")
        .current_dir(workspace_root())
        .output()
        .expect("spawn pilot binary");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "pilot test failed\n--- stdout ---\n{stdout}--- stderr ---\n{stderr}"
    );
    assert!(stdout.contains("RESULT: "), "no summary line:\n{stdout}");
    assert!(stdout.contains(" 0 failed, "), "{stdout}");
}
