//! F6 acceptance: Pilot source compiles to a native executable through C11.
//!
//! - `pilot test --emit c` builds every eligible conformance case with the
//!   host C compiler and runs it: the outputs are the seed's (the cases'
//!   expectation files), with no failures and no case left at a seam.
//! - `pilot build file.pilot -o out` writes a runnable binary: its output is
//!   the case's expected output, script arguments and standard input reach
//!   it, and a run-time error stops it with the seed's message.
//! - A program the compiler rejects is reported as diagnostics, with exit
//!   code 1 and no binary written; bad usage exits 2.
//! - The benchmark set in `benchmarks/native/` builds natively and prints
//!   the values worked out by hand in each program's header, as its C
//!   reference does, and `benchmarks/native/baseline.tsv` records the times.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::io::Write;

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

/// A fresh directory for this test's files.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pilot_f6_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Builds `src` (relative to the workspace root) into `out`.
fn build(src: &str, out: &Path) -> Output {
    pilot(&["build", src, "-o", out.to_str().expect("utf-8 path")])
}

fn run_binary(bin: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn built binary");
    child.stdin.take().expect("stdin").write_all(input.as_bytes()).expect("write stdin");
    child.wait_with_output().expect("wait for built binary")
}

fn expected(stem: &str) -> String {
    std::fs::read_to_string(root().join(format!("{stem}.stdout"))).expect("expected stdout")
}

// The whole corpus is built natively once, in `cli_f7.rs`
// (`the_corpus_passes_natively_with_no_live_allocations`): that run checks
// every program's output and exit code as well as its live allocations.

#[test]
fn pilot_build_writes_a_binary_with_the_seed_output() {
    let dir = scratch("build");
    for case in ["closures", "collections", "generator_local", "structs_methods"] {
        let stem = format!("tests/conformance/{case}");
        let bin = dir.join(case);
        let out = build(&format!("{stem}.pilot"), &bin);
        assert_eq!(out.status.code(), Some(0), "{case}: {}{}", text(&out.stdout), text(&out.stderr));
        let ran = run_binary(&bin, &[], "");
        assert_eq!(ran.status.code(), Some(0), "{case}: {}", text(&ran.stderr));
        assert_eq!(text(&ran.stdout), expected(&stem), "{case}");
    }
}

#[test]
fn a_built_binary_reads_its_arguments_and_standard_input() {
    let dir = scratch("io");
    let args_bin = dir.join("script_args");
    let out = build("tests/conformance/script_args.pilot", &args_bin);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let ran = run_binary(&args_bin, &["--verbose", "two words", ""], "");
    assert_eq!(text(&ran.stdout), "3\n[--verbose]\n[two words]\n[]\n");

    let stdin_bin = dir.join("stdin_lines");
    let out = build("tests/conformance/stdin_lines.pilot", &stdin_bin);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    // 5 + 0 + 10 + 20 bytes.
    let ran = run_binary(&stdin_bin, &[], "first\n\nthird line\nlast without newline");
    assert_eq!(
        text(&ran.stdout),
        "1: [first]\n2: []\n3: [third line]\n4: [last without newline]\n4 lines, 35 bytes\n"
    );
}

#[test]
fn a_runtime_error_stops_the_binary_with_its_message() {
    let dir = scratch("trap");
    let src = dir.join("div.pilot");
    std::fs::write(
        &src,
        "fn divide(a: i64, b: i64) -> i64 => a / b;\nfn main() {\n    print(divide(7, 2));\n    print(divide(1, 0));\n}\n",
    )
    .expect("write program");
    let bin = dir.join("div");
    let out = pilot(&["build", src.to_str().unwrap(), "-o", bin.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let ran = run_binary(&bin, &[], "");
    assert_eq!(text(&ran.stdout), "3\n");
    assert_eq!(ran.status.code(), Some(1));
    assert!(text(&ran.stderr).contains("runtime error: integer division by zero"), "{}", text(&ran.stderr));
}

#[test]
fn a_rejected_program_is_reported_as_diagnostics_and_writes_no_binary() {
    let dir = scratch("reject");
    let src = dir.join("bad.pilot");
    std::fs::write(&src, "fn main() {\n    let x: i64 = \"text\";\n    print(x);\n}\n").expect("write program");
    let bin = dir.join("bad");
    let out = pilot(&["build", src.to_str().unwrap(), "-o", bin.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    let stderr = text(&out.stderr);
    assert!(stderr.contains("error[") && stderr.contains("bad.pilot"), "{stderr}");
    assert!(!bin.exists(), "a rejected program left a binary");
}

#[test]
fn bad_usage_exits_2() {
    assert_eq!(pilot(&["build"]).status.code(), Some(2));
    assert_eq!(pilot(&["build", "tests/conformance/no_such_case.pilot", "-o", "x"]).status.code(), Some(2));
}

#[test]
fn the_benchmarks_build_natively_and_match_their_c_references() {
    let dir = scratch("bench");
    for b in ["fib", "numeric_loop", "string_build", "map_workload"] {
        let stem = format!("benchmarks/native/{b}");
        let want = expected(&stem);

        let bin = dir.join(b);
        let out = build(&format!("{stem}.pilot"), &bin);
        assert_eq!(out.status.code(), Some(0), "{b}: {}", text(&out.stderr));
        assert_eq!(text(&run_binary(&bin, &[], "").stdout), want, "{b} (Pilot)");

        let c_bin = dir.join(format!("{b}_c"));
        let cc = Command::new("cc")
            .args(["-O2", "-std=c11", "-D_POSIX_C_SOURCE=200809L"])
            .arg(root().join(format!("{stem}.c")))
            .arg("-o")
            .arg(&c_bin)
            .output()
            .expect("run cc");
        assert!(cc.status.success(), "{b}: {}", text(&cc.stderr));
        assert_eq!(text(&run_binary(&c_bin, &[], "").stdout), want, "{b} (C)");
    }
    // The recorded baseline: a header, then one row per benchmark.
    let baseline = std::fs::read_to_string(root().join("benchmarks/native/baseline.tsv")).expect("baseline");
    let rows: Vec<&str> = baseline.lines().skip(1).collect();
    for b in ["fib", "numeric_loop", "string_build", "map_workload"] {
        let row = rows.iter().find(|r| r.split('\t').next() == Some(b)).unwrap_or_else(|| panic!("no row for {b}"));
        assert_eq!(row.split('\t').count(), 4, "{row}");
    }
}

/// A struct value is a C struct of its fields in their own C types (an
/// `i64` field an `int64_t`, an `f64` a `double`, a `bool` a `bool`), not a
/// list of dynamic values; the program's output is unchanged.
#[test]
fn struct_values_are_c_structs_of_their_field_types() {
    let dir = scratch("records");
    let c = dir.join("records_native.c");
    let out = pilot(&["build", "tests/conformance/records_native.pilot", "--target", "c", "-o", c.to_str().expect("utf-8 path")]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let src = std::fs::read_to_string(&c).expect("emitted C");
    let structs: Vec<&str> = src.lines().filter(|l| l.starts_with("typedef struct ")).collect();
    // `Point { x: i64, y: i64 }` and `Label { name: str, at: Point, weight: f64, shown: bool }`.
    assert!(structs.iter().any(|l| l.contains("int64_t f0; int64_t f1; }")), "{structs:?}");
    assert!(structs.iter().any(|l| l.contains("double f2; bool f3; }")), "{structs:?}");

    let bin = dir.join("records_native");
    let out = build("tests/conformance/records_native.pilot", &bin);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(text(&run_binary(&bin, &[], "").stdout), expected("tests/conformance/records_native"));
}

/// The records benchmark (struct values in a list) builds natively and
/// prints what its C reference prints.
#[test]
fn the_records_benchmark_matches_its_c_reference() {
    let dir = scratch("bench_records");
    let stem = "benchmarks/native/records";
    let want = expected(stem);
    let bin = dir.join("records");
    let out = build(&format!("{stem}.pilot"), &bin);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(text(&run_binary(&bin, &[], "").stdout), want, "records (Pilot)");
    let c_bin = dir.join("records_c");
    let cc = Command::new("cc")
        .args(["-O2", "-std=c11"])
        .arg(root().join(format!("{stem}.c")))
        .arg("-o")
        .arg(&c_bin)
        .output()
        .expect("run cc");
    assert!(cc.status.success(), "{}", text(&cc.stderr));
    assert_eq!(text(&run_binary(&c_bin, &[], "").stdout), want, "records (C)");
}

/// A string's length is 64 bits: one longer than the length a value holds
/// itself (`PILOT_V_SHORT_MAX`, about 4 GiB) keeps it before its bytes.
/// Lowering that limit to 4 bytes puts nearly every string of these
/// programs in the long form; their output is unchanged.
#[test]
fn strings_past_the_short_length_limit_keep_their_length() {
    let dir = scratch("long_strings");
    for stem in [
        "tests/conformance/collections",
        "tests/conformance/containers",
        "tests/conformance/json_rfc8259",
        "tests/conformance/records_native",
        "benchmarks/native/string_build",
    ] {
        let name = stem.rsplit('/').next().expect("file name");
        let c = dir.join(format!("{name}.c"));
        let out = pilot(&["build", &format!("{stem}.pilot"), "--target", "c", "-o", c.to_str().expect("utf-8 path")]);
        assert_eq!(out.status.code(), Some(0), "{stem}: {}", text(&out.stderr));
        let bin = dir.join(name);
        let cc = Command::new("cc")
            .args(["-std=c11", "-O1", "-w", "-DPILOT_V_SHORT_MAX=4", "-I"])
            .arg(root().join("runtime/c"))
            .arg(&c)
            .arg("-o")
            .arg(&bin)
            .arg("-lm")
            .output()
            .expect("run cc");
        assert!(cc.status.success(), "{stem}: {}", text(&cc.stderr));
        assert_eq!(text(&run_binary(&bin, &[], "").stdout), expected(stem), "{stem}");
    }
}

/// A struct value read out of a list (`bodies[j]`, a dynamic value in C)
/// has its fields read by its layout's typed readers, not looked up as
/// dynamic fields.
#[test]
fn a_struct_read_from_a_list_uses_its_typed_fields() {
    let dir = scratch("typed_reads");
    let c = dir.join("records.c");
    let out = pilot(&["build", "benchmarks/native/records.pilot", "--target", "c", "-o", c.to_str().expect("utf-8 path")]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let src = std::fs::read_to_string(&c).expect("emitted C");
    // The function bodies, without the per-layout helpers declared first.
    let bodies: Vec<&str> = src.lines().filter(|l| !l.starts_with("static ") && !l.starts_with("typedef ")).collect();
    assert!(bodies.iter().all(|l| !l.contains("pilot_v_field(")), "dynamic field reads remain");
    assert!(bodies.iter().filter(|l| l.contains("pilot_get")).count() >= 4, "no typed field reads");
}
