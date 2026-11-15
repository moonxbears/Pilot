//! Acceptance tests for `tools/keyword_matrix`, the Pilot tool that keeps
//! `docs/keyword_matrix.md` current (AGENTS.md section 11, rule 8).
//!
//! The tool runs on a copy of `tests/tool_fixtures/keyword_matrix/` and its
//! output is compared with `expected_keyword_matrix.md` there, which was
//! worked by hand from the fixture files before the tool was run on them.
//! The working, keyword by keyword (sizes are characters of code without
//! comments or white space):
//!
//! * Registry: categories 0, 1 and 35 are in the matrix, 20 is not, so
//!   `quark` gets no row. `orbit`'s doc holds a `|`, escaped as `\|`;
//!   `lull`'s doc is 103 bytes, clipped to 90 and `…`. `tether` has the
//!   symbol pair `~>`.
//! * Hand columns come from the stale table between the markers, found by
//!   column name; the second `orbit` row and the `ghost` row are ignored.
//!   `tether` has no row: `?` columns, NOT AUDITED, "Not checked yet.".
//! * Uses pass / fail count programs whose code (not comments or strings)
//!   names the keyword: `orbit` is in drift_steps and orbit_basic (pass) and
//!   err_orbit_empty (fail): 2 / 1. `drift` 1 / 0 (orbit_basic names it only
//!   in a comment; the known bug is not counted). `tether` 0 / 1 (only a
//!   string in drift_steps). `lull` 0 / 0: control/lib/helper.pilot has no
//!   expected output, so it is no program.
//! * Named tests are programs whose file name holds the keyword, shortest
//!   path first; Codes are the first `error[..]` of a named failing one.
//! * Ledger: backtick spans of row titles: `orbit` rows 1 and 2 (Draft,
//!   Wired, sorted), `drift` row 2, `tether` row 3, `unit` row 4. Known bugs:
//!   the one summary quotes `drift` and `orbit`.
//! * Rust (crates/seed/src only; tests/outside_src.rs is outside):
//!   - `orbit`: receptor (15, 0) found on line `(15, 0) => lower_orbit(),`
//!     (22) of `lower_receptor`, which has 3 callers, so only the line
//!     counts; it calls `lower_orbit` (1 caller, 40): 62.
//!   - `drift`: (15, 1) through `Cfg.Tokens.kw_drift`, line `(15, 1) => 7,`: 10.
//!   - `tether`: (16, 0) line: 10; `is_rec(16)` (kind 16 is tether's alone)
//!     sits in `tether_ready` (0 callers, so all 37 count), which calls
//!     `is_rec` (37): 84.
//!   - `unit`: file units.rs holds the concept word `units`: 30.
//! * Pilot (stdlib except keywords.pilot, then selfhost):
//!   - `orbit`: `nameof(orbit)` on the receptor line (48) and in
//!     `check_names` (dedicated, 69 whole), and the syntax rule line
//!     `orbit(body) => quote ..` (32): 149.
//!   - `drift`: `nameof(drift)` in `check_names`: 69.
//!   - `tether`: no key found: 0.
//!   - `unit`: function `unit_literal_value` holds the concept `unit_literal`: 47.
//! * Shares, rounded half up: 149/211 = 70.6 -> 71%, 69/79 = 87.3 -> 87%,
//!   0/84 = 0%, 47/77 = 61.0 -> 61%. `lull` is bound by `none`, so it is not
//!   measured. Median of 0, 61, 71, 87 is (61 + 71) / 2 = 66.
//! * Gaps: only `unit` has none. Verdicts: REAL 2, then the ones with 1 by name.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn fixture() -> PathBuf {
    root().join("tests/tool_fixtures/keyword_matrix")
}

fn tool(args: &[&str]) -> Output {
    let mut all = vec!["run", "tools/keyword_matrix/main.pilot", "--"];
    all.extend_from_slice(args);
    Command::new(env!("CARGO_BIN_EXE_pilot"))
        .args(&all)
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

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create fixture copy");
    for entry in fs::read_dir(from).expect("read fixture") {
        let entry = entry.expect("fixture entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy fixture file");
        }
    }
}

/// A fresh copy of the fixture, so the tool can rewrite its matrix.
fn fixture_copy(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pilot_keyword_matrix_{name}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    copy_dir(&fixture(), &dir);
    dir
}

fn matrix(dir: &Path) -> String {
    fs::read_to_string(dir.join("docs/keyword_matrix.md")).expect("read matrix")
}

#[test]
fn the_matrix_is_regenerated_from_the_repository() {
    let dir = fixture_copy("regenerate");
    let arg = dir.to_string_lossy().into_owned();
    let (stdout, stderr, code) = text(&tool(&["--root", &arg]));
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert!(stdout.contains("keyword_matrix: wrote"), "{stdout}");
    let expected = fs::read_to_string(fixture().join("expected_keyword_matrix.md")).expect("expected matrix");
    assert_eq!(matrix(&dir), expected);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn check_reports_a_stale_matrix_without_writing_it() {
    let dir = fixture_copy("stale");
    let before = matrix(&dir);
    let arg = dir.to_string_lossy().into_owned();
    let (stdout, stderr, code) = text(&tool(&["--check", "--root", &arg]));
    assert_eq!(code, 1, "{stdout}{stderr}");
    assert!(stdout.contains("is stale"), "{stdout}");
    assert_eq!(matrix(&dir), before, "--check must not write");

    // Once written, the same check passes.
    assert_eq!(text(&tool(&["--root", &arg])).2, 0);
    let (stdout, _, code) = text(&tool(&["--check", "--root", &arg]));
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("is up to date"), "{stdout}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_matrix_without_markers_is_refused() {
    let dir = fixture_copy("markers");
    let path = dir.join("docs/keyword_matrix.md");
    let without: String = matrix(&dir)
        .lines()
        .filter(|l| !l.starts_with("<!-- keyword_matrix:"))
        .map(|l| format!("{l}\n"))
        .collect();
    fs::write(&path, &without).expect("write matrix");
    let arg = dir.to_string_lossy().into_owned();
    let (stdout, stderr, code) = text(&tool(&["--root", &arg]));
    assert_eq!(code, 2, "{stdout}{stderr}");
    assert!(stdout.contains("has no generated part"), "{stdout}");
    assert_eq!(matrix(&dir), without, "nothing is written");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_matrix_and_an_unknown_argument_are_refused() {
    let dir = std::env::temp_dir().join(format!("pilot_keyword_matrix_empty_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create empty root");
    let arg = dir.to_string_lossy().into_owned();
    let (stdout, _, code) = text(&tool(&["--root", &arg]));
    assert_eq!(code, 2, "{stdout}");
    assert!(stdout.contains("does not exist"), "{stdout}");
    let _ = fs::remove_dir_all(&dir);

    let (stdout, _, code) = text(&tool(&["--fix"]));
    assert_eq!(code, 2, "{stdout}");
    assert!(stdout.contains("unknown argument `--fix`"), "{stdout}");
}

#[test]
#[ignore = "slow: measures the whole repository (about 15 s)"]
fn the_repository_matrix_is_up_to_date() {
    let (stdout, stderr, code) = text(&tool(&["--check"]));
    assert_eq!(code, 0, "docs/keyword_matrix.md is stale; run `pilot run tools/keyword_matrix/main.pilot`\n{stdout}{stderr}");
}
