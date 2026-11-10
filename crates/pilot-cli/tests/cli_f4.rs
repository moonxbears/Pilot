//! F4 acceptance that `pilot test` cannot express: well-typed programs get
//! no diagnostic at all from the type checker (`selfhost/check/`).
//!
//! The negative side, one program per code, is in `tests/conformance/err_*`
//! and runs under `pilot test` (crates/pilot-cli/tests/conformance.rs).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn check(path: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pilot"))
        .args(["check", path])
        .current_dir(root())
        .output()
        .expect("spawn pilot binary")
}

fn assert_clean(path: &str) {
    let out = check(path);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{path}: {stderr}");
    assert_eq!(stderr, "", "{path}: a well-typed program gets no diagnostic");
}

#[test]
fn well_typed_programs_get_no_diagnostic() {
    // Generics with constraints, optional and fallible values with `?`, `??`,
    // `if some` and `catch`, closures, exhaustive enum switches, numeric
    // literals of several widths, lifting into `T?` and `T ! E`.
    assert_clean("tests/conformance/types_clean.pilot");
    // Calls through local closures are uses of the local (the old analyzer
    // reported `f` as unused here).
    assert_clean("tests/conformance/closures.pilot");
    // Switches written as a function's value, over enums and tuples.
    assert_clean("tests/conformance/switch.pilot");
    // A program of several files: nothing is reported in the imported stdlib.
    assert_clean("tests/conformance/modules.pilot");
}

#[test]
fn a_program_that_does_not_link_is_not_type_checked() {
    let out = check("tests/conformance/no_such_program.pilot");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("error[MOD001]"), "{stderr}");
    assert!(!stderr.contains("TYPE"), "{stderr}");
}

/// Writes `src` as a program in a fresh directory of its own (so nothing
/// else is linked with it) and returns the `(code, line:col)` of every
/// diagnostic `pilot check` reports for it.
fn reported(name: &str, src: &str) -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli_f4").join(name);
    fs::create_dir_all(&dir).expect("create program directory");
    let file = dir.join("program.pilot");
    fs::write(&file, src).expect("write program");
    let out = check(&file.to_string_lossy());
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let mut found = Vec::new();
    let mut code = None;
    for line in stderr.lines() {
        let head = line.split(':').next().unwrap_or("");
        if let (Some(open), Some(close)) = (head.find('['), head.find(']')) {
            code = Some(head[open + 1..close].to_string());
        } else if let Some(at) = line.trim().strip_prefix("--> ") {
            let mut parts = at.rsplitn(3, ':');
            let col = parts.next().unwrap_or("");
            let row = parts.next().unwrap_or("");
            if let Some(c) = code.take() {
                found.push((c, format!("{row}:{col}")));
            }
        }
    }
    found
}

fn at(code: &str, place: &str) -> (String, String) {
    (code.to_string(), place.to_string())
}

/// The null-safety and dropped-result cases the old analyzer tests held
/// (crates/pilot-analysis/tests/phase1_ironclad_killswitch.rs), now derived
/// from types. Each line holds one case; the comment gives what is expected.
#[test]
fn none_and_fallible_values_are_checked_against_types() {
    let src = "\
fn store() { let x: i64 = none; }
fn store_opt() { let _x: i64? = none; }
fn give() -> i64 { return none; }
fn give_opt() -> i64? { return none; }
fn helper(x: i64) -> i64 { return x; }
fn helper_opt(_x: i64?) -> i64 { return 0; }
fn pass() { helper(none); helper_opt(none); }
fn try_op() -> i64 ! i64 { return 1; }
fn drop_it() { try_op(); }
fn keep_it() -> i64 { let x = try_op() catch _e => 0; return x; }
fn recover() { try_op() catch _e => 0; }
";
    let got = reported("none_and_fallible", src);
    let want = vec![
        // line 1: `none` into `i64` (the unread `x` is also reported).
        at("UNUSED001", "1:14"),
        at("NULL001", "1:14"),
        // line 3: `none` returned as `i64` (reported at the `return`).
        at("NULL003", "3:20"),
        // line 7: `none` passed for an `i64`; the optional parameter accepts it.
        at("NULL004", "7:20"),
        // line 9: the fallible result is dropped; lines 10-11 handle it.
        at("ERR001", "9:16"),
    ];
    assert_eq!(got, want);
}

/// The unused-binding and shadowing cases the old analyzer tests held, now
/// from the scoped resolver (selfhost/check/resolve.pilot).
#[test]
fn unused_and_shadowed_bindings_come_from_scoped_resolution() {
    // `scratch` and `spare` are never read; a name that merely starts with
    // "unused" but is read is not reported. A parameter read only inside a
    // closure is used; the closure parameter `w` is not. Bodiless interface
    // methods and `_` names are not reported. The inner `let x` hides the
    // parameter `x`.
    let src = "\
fn tally(limit: i64, spare: i64) -> i64 {
    let total = limit * 2;
    let scratch = 7;
    var count = 0;
    count = count + total;
    let unused_name = 1;
    return count + unused_name;
}
fn lift(base: i64) -> i64 {
    let add = (v, w) => v + base;
    return add(1, 2);
}
interface IShape { fn area(scale: i64) -> i64; }
fn keep(_ignored: i64) -> i64 { let _tmp = 1; return 1; }
fn scale(x: i64) -> i64 {
    if x > 0 {
        let x = 2;
        return x;
    }
    return x;
}
";
    let got = reported("unused_and_shadowed", src);
    let want = vec![
        at("UNUSED004", "1:22"),
        at("UNUSED001", "3:5"),
        at("UNUSED004", "10:19"),
        at("SHADOW001", "17:9"),
    ];
    assert_eq!(got, want);

    // tests/conformance/shadowing.pilot: the three inner `let x` hide the
    // outer `x`; `let y = y + 1;` re-declares in the same block, which is not
    // shadowing. Every binding is read.
    let out = check("tests/conformance/shadowing.pilot");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let shadows: Vec<&str> = stderr
        .lines()
        .zip(stderr.lines().skip(1))
        .filter(|(l, _)| l.contains("[SHADOW001]"))
        .map(|(_, at)| at.trim())
        .collect();
    assert_eq!(
        shadows,
        [
            "--> tests/conformance/shadowing.pilot:7:9",
            "--> tests/conformance/shadowing.pilot:13:9",
            "--> tests/conformance/shadowing.pilot:19:9",
        ]
    );
    assert!(!stderr.contains("UNUSED"), "{stderr}");
}

/// Method tables built from spliced facets (selfhost/check/methods.pilot):
/// a clash settled by `without` or `with .. as`, and struct methods that
/// override facet methods, are accepted, and a spliced method has its
/// facet's type. The seed does not run spliced methods and rejects a call
/// to an aliased name before the checker runs (known bug
/// `seed.bundle_splice_methods`), so no aliased name is called here.
#[test]
fn spliced_facets_settled_by_modifiers_type_check() {
    let src = "\
struct LoudFacet {
    pub fn speak() -> str => \"LOUD\";
    pub fn volume() -> i64 => 9;
}
struct QuietFacet {
    pub fn speak() -> str => \"quiet\";
    pub fn volume() -> i64 => 1;
}
struct Speaker {
    *LoudFacet without { speak } with { volume as loud_volume };
    *QuietFacet without { volume };
    pub name: str;
}
struct Announcer {
    *LoudFacet;
    *QuietFacet;
    pub fn speak() -> str => \"announce\";
    pub fn volume() -> i64 => 5;
}
fn read(s: Speaker, a: Announcer) -> str {
    let v: i64 = a.volume();
    let w: i64 = s.speak();
    return s.speak() + a.speak() + f\"{v}{w}\";
}
";
    let got = reported("spliced_facets", src);
    // Only line 22 is wrong: the `speak` Speaker keeps gives a `str`.
    // Nothing is reported for the clashes the modifiers and overrides settle.
    assert_eq!(got, vec![at("TYPE001", "22:18")]);
}

/// OWN001 follows moves through branches, loops and calls: a value moved
/// on a branch that returns is still usable after it, a `ref` parameter
/// borrows, and a new value makes a moved binding usable again.
#[test]
fn moves_are_followed_through_control_flow() {
    assert_clean("tests/conformance/moves_clean.pilot");
    let src = "\
struct Ticket : IObject { pub id: i64; }
fn consume(t: Ticket) -> i64 => t.id;
fn peek(ref t: Ticket) -> i64 => t.id;
fn named(t: Ticket) -> i64 { let a = consume(t: t); return a + t.id; }
fn both(flag: bool) -> i64 {
    let t = Ticket { id: 1 };
    if flag { consume(t); } else { peek(t); }
    return t.id;
}
fn listed() -> i64 {
    let t = Ticket { id: 2 };
    let all: List<Ticket> = [t];
    return t.id + all.len();
}
";
    let got = reported("moves_flow", src);
    let want = vec![
        // line 4: a named argument for a plain parameter moves too.
        at("OWN001", "4:64"),
        // line 7: moved on the `then` branch only; maybe moved after.
        at("OWN001", "8:12"),
        // line 12: putting it in a list moves it.
        at("OWN001", "13:12"),
    ];
    assert_eq!(got, want);
}

/// The stdlib collections take the collection roles of
/// stdlib/core/operator.pilot and provide what they require (TYPE006).
/// Checked as the entry program, a file's own types are held to their
/// interfaces. `slice`, `stride`, `array`, `map` and `SliceBuffer` sit over
/// raw memory the seed cannot compile yet, so they are not checked here.
#[test]
fn stdlib_collections_provide_their_roles() {
    assert_clean("stdlib/core/primitives/list.pilot");
    assert_clean("stdlib/std/col.pilot");
    assert_clean("stdlib/core/stream.pilot");
    assert_clean("tests/conformance/roles_clean.pilot");
}

/// A role gives a type its lattice: a struct that names `IBuffer` is owned
/// (0b10) without naming `IObject`, so using it after a move is OWN001; one
/// that names `ICursor` is a borrowed view (0b01) and is not moved.
#[test]
fn a_role_gives_its_lattice() {
    let src = "\
struct Bag : IBuffer<i64> {
    pub items: List<i64>;
    pub count: usize;
    pub fn contains(item: i64) -> bool => item == 0;
    pub fn add(_item: i64) { .count = .count + 1; }
    pub fn clear() { .count = 0; }
    pub fn remove_at(_index: usize) -> i64 => 0;
    pub op [] (_index: usize) -> i64 => 0;
    pub op []= (_index: usize, _item: i64) { .count = .count; }
}
struct Walk : ICursor<i64> {
    pub current: i64;
    pub fn move_next() -> bool => false;
}
fn keep(b: Bag) -> usize => b.count;
fn look(w: Walk) -> i64 => w.current;
fn main() {
    let b = Bag { items: [], count: 0 };
    let n = keep(b);
    print(b.count + n);
    let w = Walk { current: 1 };
    let m = look(w);
    print(w.current + m);
}
";
    let got = reported("role_lattice", src);
    // line 20: `b` was moved into `keep`; `w` is a view and is not moved.
    assert_eq!(got, vec![at("OWN001", "20:11")]);
}
