//! F5 acceptance: PilIR through the real toolchain (`pilot ir`, `pilot test --ir`).
//!
//! - The positive conformance corpus runs on the IR interpreter with the
//!   output of `pilot run`, or stops at a registered `ir.*` seam; every case
//!   that runs round-trips through the IR text (`pilot test --ir`).
//! - Hand-written IR text in `tests/ir/` runs on the interpreter without the
//!   lowering, and each malformed file is rejected with its code: IR009 for
//!   text that does not parse, IR001 (a use its definition does not
//!   dominate), IR002 (branch arity) and IR003 (terminators) from the verifier.

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

#[test]
fn the_corpus_runs_on_the_ir_interpreter_or_stops_at_a_seam() {
    let out = pilot(&["test", "--ir"]);
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}{}", text(&out.stderr));
    let result = stdout.lines().find(|l| l.starts_with("RESULT (ir):")).expect("a result line");
    assert!(result.contains(" 0 failed"), "{result}");
    // Every case that did not pass names the seam it stopped at.
    for line in stdout.lines().filter(|l| l.starts_with("SEAM ")) {
        assert!(line.contains("SEAM001") && line.contains("`ir."), "{line}");
    }
}

#[test]
fn a_lowered_program_runs_with_the_seed_output() {
    // 10+11+12+13+14 = 60 > 50, so `big`.
    let out = pilot(&["ir", "run", "tests/conformance/ir_loop_call_join.pilot"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(text(&out.stdout), "60\nbig\n");
    let trip = pilot(&["ir", "roundtrip", "tests/conformance/ir_loop_call_join.pilot"]);
    assert_eq!(trip.status.code(), Some(0), "{}", text(&trip.stderr));
}

#[test]
fn printed_ir_is_text_the_parser_reads_back() {
    let printed = pilot(&["ir", "print", "tests/conformance/structs_methods.pilot"]);
    assert_eq!(printed.status.code(), Some(0), "{}", text(&printed.stderr));
    let dir = std::env::temp_dir().join(format!("pilot_f5_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("printed.pir");
    std::fs::write(&file, &printed.stdout).expect("write printed IR");
    let path = file.to_string_lossy().into_owned();
    let trip = pilot(&["ir", "roundtrip", &path]);
    assert_eq!(trip.status.code(), Some(0), "{}", text(&trip.stderr));
    // The text runs as the program did: `bump(2)` then `bump(3)` from 1.
    let run = pilot(&["ir", "run", &path]);
    assert_eq!(text(&run.stdout), "6\n6\n", "{}", text(&run.stderr));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hand_written_ir_runs_on_the_interpreter() {
    // 6 * 7, then the sum 0 + 1 + .. + 5 carried in block parameters.
    let out = pilot(&["ir", "run", "tests/ir/loop_params.pir"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(text(&out.stdout), "42\n15\n");
    let trip = pilot(&["ir", "roundtrip", "tests/ir/loop_params.pir"]);
    assert_eq!(trip.status.code(), Some(0), "{}", text(&trip.stderr));
}

fn assert_rejected(file: &str, code: &str, detail: &str) {
    let out = pilot(&["ir", "verify", file]);
    let stderr = text(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{file}: {stderr}");
    assert!(stderr.contains(code), "{file}: expected {code}: {stderr}");
    assert!(stderr.contains(detail), "{file}: expected `{detail}`: {stderr}");
}

#[test]
fn malformed_ir_is_rejected_with_its_code() {
    assert_rejected("tests/ir/bad_header.pir", "IR009", "line 1");
    assert_rejected("tests/ir/bad_opcode.pir", "IR009", "frobnicate");
    assert_rejected("tests/ir/use_before_def.pir", "IR001", "v12");
    assert_rejected("tests/ir/not_dominated.pir", "IR001", "does not dominate");
    assert_rejected("tests/ir/branch_arity.pir", "IR002", "b1");
    assert_rejected("tests/ir/missing_terminator.pir", "IR003", "b2");
}

#[test]
fn an_unlowered_construct_stops_at_its_seam() {
    // Closures are not lowered yet.
    let out = pilot(&["ir", "run", "tests/conformance/closures.pilot"]);
    let stderr = text(&out.stderr);
    assert_ne!(out.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("SEAM001") && stderr.contains("ir.lower_construct"), "{stderr}");
    assert_eq!(text(&out.stdout), "");
}

#[test]
fn struct_sizes_follow_the_c_abi() {
    // Expected sizes from gcc and clang (x86_64 SysV) on the same structs
    // written in C: tests/ir/c_abi_layout.c.
    let out = pilot(&["ir", "run", "tests/ir/c_abi_layout.pilot"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let expected = std::fs::read_to_string(root().join("tests/ir/c_abi_layout.stdout")).expect("expected sizes");
    assert_eq!(text(&out.stdout), expected);
}
