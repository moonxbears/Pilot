//! Acceptance tests for `[Coverage(modes, to: outputs)]`
//! (stdlib/core/coverage.pilot; the cross-backend case is
//! tests/conformance/attr_coverage.pilot).
//!
//! - Modes are `CoverageMode` flags joined with `|`: `::Speed` (time and
//!   statements run), `::Calls`, `::Intrinsics` (big-O by code and
//!   measured). Bare `[Coverage]` turns on all three. Statement coverage is
//!   always reported.
//! - `to:` takes `CoverageOut` flags: `::Stdout`, `::Stderr` (the default),
//!   `::File` (`<source stem>.coverage.txt` beside the source, written by
//!   `pilot run`).
//! - On a package, struct, interface or ext block it covers every function
//!   inside, and adds a total line for that declaration.
//! - Time comes from the seed's clock: PilIR and C have none yet.

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
    let dir = std::env::temp_dir().join(format!("pilot_cover_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn program(dir: &Path, file: &str, src: &str) -> String {
    let p = dir.join(file);
    std::fs::write(&p, src).expect("write program");
    p.to_str().expect("utf-8 path").to_string()
}

/// `line` with its `time <digits> us` replaced by `time T us`, after
/// checking the digits are there.
fn untimed(line: &str) -> String {
    let Some(at) = line.find("time ") else { return line.to_string() };
    let rest = &line[at + 5..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    assert!(!digits.is_empty(), "no time in `{line}`");
    assert!(rest[digits.len()..].starts_with(" us"), "no unit in `{line}`");
    format!("{}time T{}", &line[..at], &rest[digits.len()..])
}

const FIB_SRC: &str = "\
[Coverage(::Speed | ::Calls, to: ::Stdout | ::File)]
struct Fib {
    pub base: i64;
    fn at(n: i64) -> i64 {
        if n < 2 { return n; }
        return self.at(n - 1) + self.at(n - 2);
    }
}

fn main() {
    let f = Fib { base: 0 };
    print(f.at(5));
}
";

/// Worked by hand: `at(5)` makes 15 calls (the call tree of fib 5). Every
/// call runs the `if`; the 8 calls with n < 2 run `return n`, the other 7
/// `return self.at(..) + ..`: 15 + 8 + 7 = 30 statements, 2 per call. The
/// time is the outermost call's, the same on the function's and the
/// struct's line.
#[test]
fn coverage_on_a_struct_measures_its_methods() {
    let dir = scratch("fib");
    let p = program(&dir, "fib.pilot", FIB_SRC);
    let out = pilot(&["run", &p]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    let lines: Vec<String> = stdout.lines().map(untimed).collect();
    assert_eq!(
        lines,
        [
            "5",
            "[coverage] Fib.at: statements 3/3 (100%), calls 15, ops 30 (2/call), time T us",
            "[coverage] struct Fib: statements 3/3 (100%) in 1 function(s), calls 15, ops 30, time T us",
        ]
    );
    let file = std::fs::read_to_string(dir.join("fib.coverage.txt")).expect("fib.coverage.txt written");
    let filed: Vec<String> = file.lines().map(untimed).collect();
    assert_eq!(filed, lines[1..]);
    assert!(text(&out.stderr).is_empty(), "{}", text(&out.stderr));
}

/// Bare `[Coverage]` on a function: every mode, to stderr. Worked by hand:
/// `count(4)` runs `var k`, the `while`, 4 x `k = ..`, `return` = 7
/// statements; `count(8)` 11. Off by n^0 1.57, n^1 1.27, n^2 2.5: O(n).
#[test]
fn bare_coverage_turns_on_every_mode() {
    let dir = scratch("bare");
    let p = program(
        &dir,
        "bare.pilot",
        "[Coverage]\nfn count(n: i64) -> i64 {\n    var k = 0;\n    while k < n { k = k + 1; }\n    return k;\n}\n\n\
         fn main() { print(count(4) + count(8)); }\n",
    );
    let out = pilot(&["run", &p]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(text(&out.stdout), "12\n");
    assert_eq!(
        untimed(text(&out.stderr).trim_end()),
        "[coverage] count: statements 4/4 (100%), calls 2, ops 18 (9/call), time T us, O(n) by code, O(n) measured"
    );
}

#[test]
fn coverage_rejects_unknown_flags() {
    let dir = scratch("bad");
    let p = program(
        &dir,
        "bad.pilot",
        "[Coverage(::Fast)]\nfn a() -> i64 => 1;\n\n\
         [Coverage(to: ::Printer)]\nfn b() -> i64 => 2;\n\n\
         [Coverage]\nenum E { X, Y }\n\nfn main() {}\n",
    );
    let out = pilot(&["check", &p]);
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    for want in [
        "error[ATTR003]: `Fast` is not a mode of `[Coverage]`: use `::Speed`, `::Calls` or `::Intrinsics`, joined with `|`",
        "error[ATTR003]: `Printer` is not an output of `[Coverage]`: use `::Stdout`, `::Stderr` or `::File`, joined with `|`",
        "error[ATTR005]: `[Coverage]` applies to a function, or through a package, struct, interface or ext block",
    ] {
        assert!(err.contains(want), "missing `{want}` in\n{err}");
    }
}
