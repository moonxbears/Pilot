//! F7 acceptance: values are freed, destructors run, leaks are measurable.
//!
//! - The conformance programs `drop_last_use`, `drop_branches_loops`,
//!   `guard_scope_order` and `value_copy` (tests/conformance/) give the
//!   destructor order, guard release order and value-type copies their
//!   headers specify, on the seed (`pilot test`), on the PilIR interpreter
//!   (`pilot test --ir`) and natively (`pilot test --emit c`).
//! - `pilot build --live-allocations` writes a binary that reports the heap
//!   objects still allocated when it exits, on standard error, as
//!   `live allocations: N`; `pilot test --emit c --live-allocations` fails a
//!   case whose binary ends with any.
//! - The whole conformance corpus, built natively, ends with none.

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

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pilot_f7_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Each case as its folder and name under `tests/conformance/`.
const OWNERSHIP: [&str; 4] =
    ["memory/drop_last_use", "memory/drop_branches_loops", "control/guard_scope_order", "memory/value_copy"];

fn expected(case: &str) -> String {
    std::fs::read_to_string(root().join(format!("tests/conformance/{case}.stdout"))).expect("expected stdout")
}

#[test]
fn the_seed_runs_destructors_guards_and_copies_as_specified() {
    for case in OWNERSHIP {
        let out = pilot(&["run", &format!("tests/conformance/{case}.pilot")]);
        assert_eq!(out.status.code(), Some(0), "{case}: {}", text(&out.stderr));
        assert_eq!(text(&out.stdout), expected(case), "{case} (seed)");
    }
}

#[test]
fn the_ir_interpreter_runs_destructors_guards_and_copies_as_specified() {
    for case in OWNERSHIP {
        let out = pilot(&["ir", "run", &format!("tests/conformance/{case}.pilot")]);
        assert_eq!(out.status.code(), Some(0), "{case}: {}", text(&out.stderr));
        assert_eq!(text(&out.stdout), expected(case), "{case} (PilIR)");
    }
}

#[test]
fn native_binaries_run_destructors_guards_and_copies_as_specified() {
    let dir = scratch("ownership");
    for case in OWNERSHIP {
        let bin = dir.join(case.replace('/', "_"));
        let out = pilot(&["build", &format!("tests/conformance/{case}.pilot"), "-o", bin.to_str().expect("utf-8 path")]);
        assert_eq!(out.status.code(), Some(0), "{case}: {}", text(&out.stderr));
        let run = Command::new(&bin).output().expect("run binary");
        assert_eq!(text(&run.stdout), expected(case), "{case} (native)");
    }
}

#[test]
fn a_binary_built_to_count_allocations_reports_none_left() {
    let dir = scratch("counted");
    let bin = dir.join("drop_last_use");
    let out = pilot(&["build", "tests/conformance/memory/drop_last_use.pilot", "-o", bin.to_str().expect("utf-8 path"), "--live-allocations"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let run = Command::new(&bin).output().expect("run binary");
    assert_eq!(text(&run.stdout), expected("memory/drop_last_use"));
    assert!(text(&run.stderr).contains("live allocations: 0"), "stderr: {}", text(&run.stderr));

    // Without the flag the binary says nothing about allocations.
    let plain = dir.join("plain");
    let out = pilot(&["build", "tests/conformance/memory/drop_last_use.pilot", "-o", plain.to_str().expect("utf-8 path")]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let run = Command::new(&plain).output().expect("run binary");
    assert!(!text(&run.stderr).contains("live allocations"), "stderr: {}", text(&run.stderr));
}

/// The counter counts: the same C compiled with freeing turned off
/// (`PILOT_NO_FREE`) reports the objects it allocated.
#[test]
fn the_allocation_counter_sees_objects_that_are_not_freed() {
    let dir = scratch("unfreed");
    let c = dir.join("records_native.c");
    let out = pilot(&["build", "tests/conformance/types/records_native.pilot", "--target", "c", "-o", c.to_str().expect("utf-8 path")]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let bin = dir.join("records_native");
    let cc = Command::new("cc")
        .args(["-std=c11", "-O1", "-w", "-DPILOT_LIVE_COUNT", "-DPILOT_NO_FREE", "-I"])
        .arg(root().join("runtime/c"))
        .arg(&c)
        .arg("-o")
        .arg(&bin)
        .arg("-lm")
        .output()
        .expect("run cc");
    assert!(cc.status.success(), "{}", text(&cc.stderr));
    let run = Command::new(&bin).output().expect("run binary");
    let err = text(&run.stderr);
    let n: i64 = err
        .lines()
        .find_map(|l| l.strip_prefix("live allocations: "))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or_else(|| panic!("no count reported: {err}"));
    // records_native builds at least its 2 + 1 + 4 struct values.
    assert!(n >= 7, "only {n} live allocations without freeing");
}

/// The one native run of the whole corpus: every program's output and exit
/// code must match its expectation files (as plain `test --emit c` checks),
/// and no heap object may be left allocated at the end.
#[test]
#[ignore = "slow: runs the whole conformance corpus; `cargo test-full` runs it"]
fn the_corpus_passes_natively_with_no_live_allocations() {
    let out = pilot(&["test", "--emit", "c", "--live-allocations"]);
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}{}", text(&out.stderr));
    let result = stdout
        .lines()
        .find(|l| l.starts_with("RESULT (emit c):"))
        .expect("a result line");
    assert!(result.contains(" 0 failed"), "{result}\n{stdout}");
    assert!(result.contains(" 0 stopped at a seam"), "{result}\n{stdout}");
}
