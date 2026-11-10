//! Phase F conformance runner (`MILESTONES_100.md` F1).
//!
//! Every `tests/conformance/<name>.pilot` program is executed with the built
//! `pilot run` binary from the workspace root. The runner compares:
//!
//! - stdout against `<name>.stdout` (exact match, required),
//! - the process exit code against `<name>.exit` (optional, defaults to `0`),
//! - stderr against `<name>.stderr` (optional, every non-empty line must
//!   appear somewhere in stderr).
//!
//! `<name>.args` (optional) holds the script arguments, one per line, passed
//! after `--`. Only the top-level `.pilot` files are programs; packages they
//! import live in subdirectories (`tests/conformance/lib/`).
//!
//! All mismatches are collected and reported together so one run shows the
//! full state of the corpus.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn read_optional(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

struct CaseResult {
    name: String,
    problems: Vec<String>,
}

fn run_case(root: &Path, program: &Path) -> CaseResult {
    let name = program
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("?")
        .to_string();
    let mut problems = Vec::new();

    let stdout_path = program.with_extension("stdout");
    let expected_stdout = match read_optional(&stdout_path) {
        Some(s) => s,
        None => {
            problems.push(format!("missing expectation file {}", stdout_path.display()));
            return CaseResult { name, problems };
        }
    };
    let expected_exit: i32 = read_optional(&program.with_extension("exit"))
        .map(|s| s.trim().parse().expect("exit file must hold an integer"))
        .unwrap_or(0);
    let expected_stderr = read_optional(&program.with_extension("stderr"));

    let args: Vec<String> = read_optional(&program.with_extension("args"))
        .map(|s| s.lines().map(str::to_string).collect())
        .unwrap_or_default();
    let output = Command::new(env!("CARGO_BIN_EXE_pilot"))
        .arg("run")
        .arg(program)
        .arg("--")
        .args(&args)
        .current_dir(root)
        .output()
        .expect("spawn pilot binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit = output.status.code().unwrap_or(-1);

    if stdout != expected_stdout {
        problems.push(format!(
            "stdout mismatch\n--- expected ---\n{expected_stdout}--- actual ---\n{stdout}--- stderr ---\n{stderr}"
        ));
    }
    if exit != expected_exit {
        problems.push(format!(
            "exit code {exit}, expected {expected_exit}\n--- stderr ---\n{stderr}"
        ));
    }
    if let Some(expected) = expected_stderr {
        for line in expected.lines().map(str::trim).filter(|l| !l.is_empty()) {
            if !stderr.contains(line) {
                problems.push(format!(
                    "stderr does not contain `{line}`\n--- stderr ---\n{stderr}"
                ));
            }
        }
    }
    CaseResult { name, problems }
}

#[test]
fn conformance_corpus() {
    let root = workspace_root();
    let dir = root.join("tests/conformance");
    let mut programs: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("tests/conformance directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("pilot"))
        .collect();
    programs.sort();
    assert!(!programs.is_empty(), "conformance corpus is empty");

    let results: Vec<CaseResult> = programs.iter().map(|p| run_case(&root, p)).collect();
    let failed: Vec<&CaseResult> = results.iter().filter(|r| !r.problems.is_empty()).collect();
    if !failed.is_empty() {
        let mut report = format!(
            "{} of {} conformance programs failed\n",
            failed.len(),
            results.len()
        );
        for case in failed {
            report.push_str(&format!("\n==== {} ====\n", case.name));
            for p in &case.problems {
                report.push_str(p);
                report.push('\n');
            }
        }
        panic!("{report}");
    }
}
