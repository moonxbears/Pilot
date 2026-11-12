//! Constants computed while compiling (selfhost/ir/pass_const_eval.pilot),
//! seen in the optimized PilIR (`pilot ir opt`) and the emitted C.
//!
//! What the programs print is checked by the conformance corpus
//! (tests/conformance/const_values, const_effects); these tests check how:
//! - every `const` holding a number, a boolean or text, and every
//!   zero-argument function giving one, is no longer called: its value is a
//!   constant where it was used;
//! - a `const` holding a list or map, and a zero-argument `pure` function
//!   giving one, is built once, at the start, into a global read without a
//!   check that its declaration has run;
//! - a function that prints, or that never finishes, is still called.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn pilot_stdout(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_pilot"))
        .args(args)
        .current_dir(root())
        .output()
        .expect("spawn pilot binary");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert_eq!(out.status.code(), Some(0), "{stdout}{}", String::from_utf8_lossy(&out.stderr));
    stdout
}

/// The index of the function named `name` in printed PilIR.
fn function_index(ir: &str, name: &str) -> usize {
    let quoted = format!(" \"{name}\" ");
    ir.lines()
        .filter(|l| l.starts_with("fn ") && l.contains(&quoted))
        .find_map(|l| l.split(' ').nth(1).and_then(|i| i.parse().ok()))
        .unwrap_or_else(|| panic!("no function `{name}` in\n{ir}"))
}

/// How many instructions call function `index`.
fn calls_of(ir: &str, index: usize) -> usize {
    let call = format!("call_direct.{index}");
    ir.lines()
        .filter(|l| l.split_whitespace().any(|w| w == call))
        .count()
}

/// The `global` line naming `name`.
fn global_line<'a>(ir: &'a str, name: &str) -> &'a str {
    let quoted = format!(" \"{name}\" ");
    ir.lines()
        .find(|l| l.starts_with("global ") && l.contains(&quoted))
        .unwrap_or_else(|| panic!("no global `{name}` in\n{ir}"))
}

#[test]
fn constants_with_scalar_values_are_not_called() {
    let ir = pilot_stdout(&["ir", "opt", "tests/conformance/const_values.pilot"]);
    for name in [
        "const@LIMIT",
        "const@SCALE",
        "const@NAME",
        "const@LABEL",
        "const@HALF",
        "const@BIG",
        "width",
        "title",
    ] {
        assert_eq!(calls_of(&ir, function_index(&ir, name)), 0, "`{name}` is still called");
    }
}

#[test]
fn constant_lists_and_maps_are_built_once_at_the_start() {
    let ir = pilot_stdout(&["ir", "opt", "tests/conformance/const_values.pilot"]);
    for name in ["PRIMES", "GRID", "INDEX", "once@table"] {
        assert!(global_line(&ir, name).ends_with(" ready"), "{}", global_line(&ir, name));
    }
    // Each initializer, and `table`, is called once: where `<start>` builds it.
    for name in ["const@PRIMES", "const@GRID", "const@INDEX", "table"] {
        assert_eq!(calls_of(&ir, function_index(&ir, name)), 1, "`{name}`");
    }
    let c = pilot_stdout(&["emit", "c", "tests/conformance/const_values.pilot"]);
    assert!(!c.contains("_set ?"), "a global read is still checked");
}

#[test]
fn functions_with_effects_or_no_end_are_still_called() {
    let ir = pilot_stdout(&["ir", "opt", "tests/conformance/const_effects.pilot"]);
    assert_eq!(calls_of(&ir, function_index(&ir, "noisy")), 2);
    assert_eq!(calls_of(&ir, function_index(&ir, "forever")), 1);
    assert_eq!(calls_of(&ir, function_index(&ir, "steady")), 0);
}

/// The body of the emitted C function `name`.
fn c_function<'a>(c: &'a str, name: &str) -> &'a str {
    let open = format!(" {name}(void) {{");
    let at = c.find(&open).unwrap_or_else(|| panic!("no C function `{name}` in\n{c}"));
    let rest = &c[at..];
    &rest[..rest.find("\n}").unwrap_or(rest.len())]
}

#[test]
fn constant_lists_of_numbers_are_static_data_in_c() {
    let c = pilot_stdout(&["emit", "c", "tests/conformance/const_values.pilot"]);
    // `const PRIMES = [2, 3, 5, 7, 11];` and the rows of
    // `const GRID = [[1, 2], [3, 4]];` are laid out item by item, and
    // `pure fn table()` (whose body builds [10, 20, 30]) too.
    for items in [
        "INT64_C(2) }, { .tag = PILOT_V_INT, .as.i = INT64_C(3) }, { .tag = PILOT_V_INT, .as.i = INT64_C(5) }, { .tag = PILOT_V_INT, .as.i = INT64_C(7) }, { .tag = PILOT_V_INT, .as.i = INT64_C(11) }",
        "INT64_C(1) }, { .tag = PILOT_V_INT, .as.i = INT64_C(2) }",
        "INT64_C(3) }, { .tag = PILOT_V_INT, .as.i = INT64_C(4) }",
        "INT64_C(10) }, { .tag = PILOT_V_INT, .as.i = INT64_C(20) }, { .tag = PILOT_V_INT, .as.i = INT64_C(30) }",
    ] {
        assert!(c.contains(items), "no static items `{items}` in\n{c}");
    }
    // GRID's outer list holds its two rows, and the globals PRIMES, GRID and
    // table's result each hold one list.
    assert_eq!(c.matches("{ .tag = PILOT_V_LIST, .as.l = &pilot_k").count(), 2 + 3, "{c}");
    // The start no longer builds them; INDEX, a map, is still built there.
    let start = c_function(&c, "pilot_start");
    for name in ["const_PRIMES()", "const_GRID()", "table()"] {
        assert!(!start.contains(name), "the start still calls {name}:\n{start}");
    }
    assert!(start.contains("const_INDEX()"), "{start}");
}
