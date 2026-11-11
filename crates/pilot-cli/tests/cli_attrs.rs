//! Acceptance tests for the `[Debug]`, `[Live]` and `[Generate]` attributes.
//!
//! - `[Debug(level, changed_only)]` on a function traces its `let`, `var` and
//!   assignment statements to stderr when the program runs, on the seed
//!   (`pilot run`), on PilIR (`pilot test --ir`) and natively through C
//!   (`pilot test --emit c`). The line formats are stated in
//!   `stdlib/core/debug.pilot`; the expected lines below are worked out by
//!   hand from them.
//! - `[Live(args)]` runs the function with `args` when the program is checked
//!   or built, and prints `<file>:<line>: live: f(args) = <value>`.
//! - `[Generate("path.ext")]` (or `[Generate(ext: "ext")]`, which names the
//!   file after the function) runs the function when the program is checked
//!   or built and writes the text it returns to that file, beside the source.
//! - `[Live(args, when: flags)]` picks the moments it runs at (`LiveWhen` in
//!   `stdlib/core/live.pilot`, joined with `|`): `::Compile` (the default) on
//!   check and build, `::Run` on `pilot run` before `main`. `::Debug` and
//!   `::OnSave` belong to `pilot debug` and the language server, which have no
//!   handler yet: those commands stop at the `cli.command_handler` seam.
//! - `pilot run` runs no `[Generate]`, and no `[Live]` without `::Run`.
//! - The step attributes of a function (`[Live]`, `[Generate]` and user
//!   `...Attr` structs implementing `IStepAttribute`, stdlib/core/attr_flow.pilot)
//!   run as one chain: highest `weight` first, equal weights as written; a
//!   step answers `::Continue`, `::SkipNext` or `::Stop`, and an `always()`
//!   step runs anyway.
//! - An attribute on a `pkg`, `struct`, `interface` or `ext` block reaches
//!   every function and method inside when it is inherited
//!   (`IInheritedAttribute`; `[Debug]` and `[Generate(ext: ..)]` among the
//!   built-ins). Its `policy()` says what an attribute of the same name
//!   written closer does: `::Override` (the default) replaces it, `::Append`
//!   adds a step, `::Coalesce` fills over its fields. `[NoInherit]` /
//!   `[NoInherit(A, B)]` drops inherited attributes, on a struct for all its
//!   methods. One that cannot apply is ATTR005.
//! - Misuse is a checker error: ATTR003 for a bad argument, ATTR004 for a
//!   `[Generate]` function that takes parameters or does not return `str`.

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

/// A fresh directory for this test's files.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pilot_attrs_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Writes `src` as `<dir>/<file>` and returns its path as text.
fn program(dir: &Path, file: &str, src: &str) -> String {
    let p = dir.join(file);
    std::fs::write(&p, src).expect("write program");
    p.to_str().expect("utf-8 path").to_string()
}

const DEBUG_CASE: &str = "tests/conformance/attr_debug.pilot";

/// Every trace line of `attr_debug.pilot`, in the order the program makes
/// them (the calls in `main`'s f-string run left to right). `quiet` has no
/// attribute, so nothing of it appears.
const DEBUG_TRACE: &str = "\
[debug] x = 1
[debug] x = 3
[debug] y = 10
[debug] y = 30 (was 10)
[debug] verbose:26: var z = a => z = 4
[debug] verbose:27: z = z - 1 => z = 3 (was 4)
[debug] w = 5
[debug] w = 6 (was 5)
[debug] s = a
[debug] s = ab
[debug] xs = [1, 2]
[debug] xs[0] = 7 (was 1)
";

#[test]
fn debug_traces_on_the_seed() {
    let out = pilot(&["run", DEBUG_CASE]);
    assert_eq!(text(&out.stdout), "3 30 3 6 ab 9 2\n");
    assert_eq!(text(&out.stderr), DEBUG_TRACE);
    assert_eq!(out.status.code(), Some(0));
}

/// A field target, and a target the trace cannot read again without a call
/// (`make().x`, not traced). Seed only: PilIR and C show a struct value as
/// its field list.
#[test]
fn debug_traces_fields_on_the_seed() {
    let dir = scratch("debug_fields");
    let p = program(
        &dir,
        "fields.pilot",
        "struct Point { x: i64; y: i64; }\n\
         fn make() -> Point => Point { x: 1, y: 2 };\n\
         [Debug]\n\
         fn fields() -> i64 {\n\
             var p = make();\n\
             p.x = 7;\n\
             p.y += p.x;\n\
             return p.x + p.y;\n\
         }\n\
         fn main() { print(fields()); }\n",
    );
    let out = pilot(&["run", &p]);
    assert_eq!(text(&out.stdout), "16\n");
    assert_eq!(
        text(&out.stderr),
        "[debug] p = { x: 1, y: 2 }\n[debug] p.x = 7 (was 1)\n[debug] p.y = 9 (was 2)\n"
    );
}

#[test]
fn debug_traces_on_pilir_and_c() {
    for mode in [&["test", "--ir", DEBUG_CASE][..], &["test", "--emit", "c", DEBUG_CASE][..]] {
        let out = pilot(mode);
        assert_eq!(out.status.code(), Some(0), "{mode:?}\n{}{}", text(&out.stdout), text(&out.stderr));
    }
}

#[test]
fn debug_traces_natively() {
    let dir = scratch("debug_native");
    let exe = dir.join("attr_debug");
    let built = pilot(&["build", DEBUG_CASE, "-o", exe.to_str().expect("utf-8 path")]);
    assert_eq!(built.status.code(), Some(0), "{}", text(&built.stderr));
    let out = Command::new(&exe).output().expect("run built binary");
    assert_eq!(text(&out.stdout), "3 30 3 6 ab 9 2\n");
    assert_eq!(text(&out.stderr), DEBUG_TRACE);
}

#[test]
fn debug_rejects_bad_arguments() {
    let out = pilot(&["run", "tests/conformance/err_attr_debug_level.pilot"]);
    let err = text(&out.stderr);
    assert_eq!(out.status.code(), Some(1));
    assert!(err.contains("error[ATTR003]: `Loud` is not a level of `[Debug]`"), "{err}");
    assert!(err.contains("error[ATTR003]: `[Debug]` has no argument `every`"), "{err}");
    assert!(text(&out.stdout).is_empty());
}

const LIVE_SRC: &str = "\
// Each [Live] runs when the program is checked or built.
[Live(3, 4)]
[Live(-2, 10)]
fn add(a: i64, b: i64) -> i64 => a + b;

[Live(\"Pilot\")]
fn greet(name: str) -> str => f\"hello, {name}\";

fn total(xs: List<i64>) -> i64 {
    var t = 0;
    for x in xs { t = t + x; }
    return t;
}

[Live([1, 2, 3])]
fn sum3(xs: List<i64>) -> i64 => total(xs);

fn main() {
    print(add(1, 1));
}
";

#[test]
fn live_runs_on_check() {
    let dir = scratch("live_check");
    let p = program(&dir, "live.pilot", LIVE_SRC);
    let out = pilot(&["check", &p]);
    assert_eq!(
        text(&out.stdout),
        format!(
            "{p}:2: live: add(3, 4) = 7\n\
             {p}:3: live: add(-2, 10) = 8\n\
             {p}:6: live: greet(\"Pilot\") = hello, Pilot\n\
             {p}:15: live: sum3([1, 2, 3]) = 6\n\
             check passed: 1 file(s), no errors\n"
        )
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
}

#[test]
fn live_runs_on_build_not_on_run() {
    let dir = scratch("live_build");
    let p = program(&dir, "live.pilot", LIVE_SRC);
    let exe = dir.join("live");
    let built = pilot(&["build", &p, "-o", exe.to_str().expect("utf-8 path")]);
    assert_eq!(built.status.code(), Some(0), "{}", text(&built.stderr));
    assert!(text(&built.stdout).contains(&format!("{p}:2: live: add(3, 4) = 7\n")));
    let ran = pilot(&["run", &p]);
    assert_eq!(text(&ran.stdout), "2\n");
    let native = Command::new(&exe).output().expect("run built binary");
    assert_eq!(text(&native.stdout), "2\n");
}

#[test]
fn live_fault_fails_the_check() {
    let dir = scratch("live_fault");
    let p = program(&dir, "fault.pilot", "[Live(1, 0)]\nfn div(a: i64, b: i64) -> i64 => a / b;\n\nfn main() {}\n");
    let out = pilot(&["check", &p]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("runtime error"), "{}", text(&out.stderr));
    assert!(!text(&out.stdout).contains("check passed"));
}

#[test]
fn live_argument_count_is_checked() {
    let dir = scratch("live_arity");
    let p = program(&dir, "arity.pilot", "[Live(1)]\nfn add(a: i64, b: i64) -> i64 => a + b;\n\nfn main() {}\n");
    let out = pilot(&["check", &p]);
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(err.contains("error[ATTR003]: `[Live]` gives `add` 1 argument(s); it takes 2"), "{err}");
}

const GEN_SRC: &str = "\
// The text `schema` returns becomes `out/schema.sql`; `config` becomes `config.json`.
[Generate(\"out/schema.sql\")]
fn schema() -> str {
    var sql = \"\";
    for t in [\"users\", \"posts\"] {
        sql = sql + f\"CREATE TABLE {t} (id INTEGER PRIMARY KEY);\\n\";
    }
    return sql;
}

[Generate(ext: \"json\")]
fn config() -> str => \"{\\\"debug\\\": true}\";

fn main() {
    print(\"ran\");
}
";

#[test]
fn generate_writes_files_on_check() {
    let dir = scratch("gen_check");
    let p = program(&dir, "gen.pilot", GEN_SRC);
    let out = pilot(&["check", &p]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let d = dir.to_str().expect("utf-8 path");
    assert_eq!(
        text(&out.stdout),
        format!(
            "{p}:2: generated {d}/out/schema.sql\n\
             {p}:11: generated {d}/config.json\n\
             check passed: 1 file(s), no errors\n"
        )
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("out/schema.sql")).expect("schema.sql written"),
        "CREATE TABLE users (id INTEGER PRIMARY KEY);\nCREATE TABLE posts (id INTEGER PRIMARY KEY);\n"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("config.json")).expect("config.json written"),
        "{\"debug\": true}"
    );
}

#[test]
fn generate_writes_files_on_build_not_on_run() {
    let dir = scratch("gen_build");
    let p = program(&dir, "gen.pilot", GEN_SRC);
    let ran = pilot(&["run", &p]);
    assert_eq!(text(&ran.stdout), "ran\n");
    assert!(!dir.join("config.json").exists(), "`pilot run` must not generate");
    let exe = dir.join("gen");
    let built = pilot(&["build", &p, "-o", exe.to_str().expect("utf-8 path")]);
    assert_eq!(built.status.code(), Some(0), "{}", text(&built.stderr));
    assert_eq!(std::fs::read_to_string(dir.join("config.json")).expect("config.json written"), "{\"debug\": true}");
}

#[test]
fn generate_needs_a_text_function_and_a_target() {
    let dir = scratch("gen_bad");
    let p = program(
        &dir,
        "bad.pilot",
        "[Generate(\"a.txt\")]\nfn takes(n: i64) -> str => \"x\";\n\n\
         [Generate(\"b.txt\")]\nfn number() -> i64 => 1;\n\n\
         [Generate]\nfn nowhere() -> str => \"x\";\n\nfn main() {}\n",
    );
    let out = pilot(&["check", &p]);
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(err.contains("error[ATTR004]: `[Generate]` function `takes` must take no parameters"), "{err}");
    assert!(err.contains("error[ATTR004]: `[Generate]` function `number` must return `str`"), "{err}");
    assert!(err.contains("error[ATTR003]: `[Generate]` needs the file to write: `[Generate(\"name.ext\")]` or `[Generate(ext: \"ext\")]`"), "{err}");
    assert!(!dir.join("a.txt").exists() && !dir.join("b.txt").exists());
}

const WHEN_SRC: &str = "\
[Live(3, 4)]
[Live(1, 1, when: ::Run)]
[Live(5, 5, when: ::Compile | ::Run)]
[Live(2, 2, when: ::Debug)]
[Live(9, 9, when: ::OnSave)]
fn add(a: i64, b: i64) -> i64 => a + b;

fn main() { print(add(0, 1)); }
";

#[test]
fn live_when_picks_the_moment() {
    let dir = scratch("live_when");
    let p = program(&dir, "when.pilot", WHEN_SRC);
    let checked = pilot(&["check", &p]);
    assert_eq!(checked.status.code(), Some(0), "{}", text(&checked.stderr));
    assert_eq!(
        text(&checked.stdout),
        format!(
            "{p}:1: live: add(3, 4) = 7\n\
             {p}:3: live: add(5, 5) = 10\n\
             check passed: 1 file(s), no errors\n"
        )
    );
    let ran = pilot(&["run", &p]);
    assert_eq!(ran.status.code(), Some(0), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        format!("{p}:2: live: add(1, 1) = 2\n{p}:3: live: add(5, 5) = 10\n1\n")
    );
}

#[test]
fn live_run_fault_stops_the_run() {
    let dir = scratch("live_run_fault");
    let p = program(
        &dir,
        "fault.pilot",
        "[Live(1, 0, when: ::Run)]\nfn div(a: i64, b: i64) -> i64 => a / b;\n\nfn main() { print(\"main\"); }\n",
    );
    let ran = pilot(&["run", &p]);
    assert_eq!(ran.status.code(), Some(1));
    assert!(text(&ran.stderr).contains("runtime error"), "{}", text(&ran.stderr));
    assert!(!text(&ran.stdout).contains("main"));
    let checked = pilot(&["check", &p]);
    assert_eq!(checked.status.code(), Some(0), "a `::Run` call does not run on check");
}

#[test]
fn live_debug_and_on_save_wait_for_their_commands() {
    let dir = scratch("live_seams");
    let p = program(&dir, "when.pilot", WHEN_SRC);
    let debugged = pilot(&["debug", &p]);
    assert_eq!(debugged.status.code(), Some(70));
    let err = text(&debugged.stderr);
    assert!(err.contains("SEAM001: unconnected seam `cli.command_handler`: `pilot debug`"), "{err}");
    assert!(!text(&debugged.stdout).contains("add(2, 2)"));
}

#[test]
fn live_when_takes_only_flags() {
    let dir = scratch("live_bad_when");
    let p = program(&dir, "bad.pilot", "[Live(1, when: ::Later)]\nfn one(a: i64) -> i64 => a;\n\nfn main() {}\n");
    let out = pilot(&["check", &p]);
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(
        err.contains("error[ATTR003]: `Later` is not a flag of `[Live]`: use `::OnSave`, `::Debug`, `::Compile` or `::Run`, joined with `|`"),
        "{err}"
    );
    // `when:` is not one of the call's arguments.
    assert!(!err.contains("argument(s)"), "{err}");
}

const CHAIN_SRC: &str = "use Pilot.Core.AttrFlow;

// Says what the chain does next.
struct FlowAttr : IStepAttribute {
    pub say: str;
    pub weight: i32 = 0;
    pub fn step(fn_name: str) -> AttrFlow {
        print(f\"step {self.say} on {fn_name}\");
        if self.say == \"stop\" { return ::Stop; }
        if self.say == \"skip\" { return ::SkipNext; }
        return ::Continue;
    }
}

// Runs whatever the steps before it said.
struct CleanupAttr : IStepAttribute {
    pub weight: i32 = -10;
    pub fn step(fn_name: str) -> AttrFlow {
        print(f\"cleanup {fn_name}\");
        return ::Continue;
    }
    pub pure fn always() -> bool => true;
}

[Flow(\"go\")]
[Live(1)]
[Flow(\"skip\")]
[Live(2)]
[Live(3)]
fn a(x: i64) -> i64 => x;

[Cleanup]
[Flow(\"stop\")]
[Generate(\"b.txt\")]
[Live(weight: 5)]
fn b() -> str => \"b\";

[Flow(\"first\", weight: 1)]
[Flow(\"skip\", weight: 2)]
[Cleanup(weight: 1)]
fn c() -> i64 => 0;

[Flow(\"skip\")]
[Cleanup(weight: 0)]
[Flow(\"after\")]
fn d() -> i64 => 0;

fn main() { print(\"main\"); }
";

/// The chain of each function, worked out by hand from the rules in
/// stdlib/core/attr_flow.pilot:
/// - `a`: all weight 0, so as written: `go`, `Live(1)`, `skip`, then
///   `Live(2)` is skipped and `Live(3)` runs.
/// - `b`: `Live(weight: 5)` first, then `stop` (weight 0, before `Generate`
///   as written), so `Generate` does not run; `Cleanup` (weight -10) is
///   `always` and runs.
/// - `c`: `skip` (2) first; `first` and `Cleanup` (both 1) as written, so the
///   skip lands on `first`; `Cleanup` runs.
/// - `d`: the skip lands on `Cleanup(weight: 0)`, which is `always`: it runs
///   and the skip is used up, so `after` runs too.
#[test]
fn attr_chain_runs_steps_in_order() {
    let dir = scratch("attr_chain");
    let p = program(&dir, "chain.pilot", CHAIN_SRC);
    let checked = pilot(&["check", &p]);
    assert_eq!(checked.status.code(), Some(0), "{}", text(&checked.stderr));
    assert_eq!(
        text(&checked.stdout),
        format!(
            "step go on a\n\
             {p}:26: live: a(1) = 1\n\
             step skip on a\n\
             {p}:29: live: a(3) = 3\n\
             {p}:35: live: b() = b\n\
             step stop on b\n\
             cleanup b\n\
             step skip on c\n\
             cleanup c\n\
             step skip on d\n\
             cleanup d\n\
             step after on d\n\
             check passed: 1 file(s), no errors\n"
        )
    );
    assert!(!dir.join("b.txt").exists(), "a `::Stop` before `[Generate]` keeps it from writing");
    // On `pilot run` the steps run again before `main`; the `::Compile`
    // `[Live]`s keep their places without running.
    let ran = pilot(&["run", &p]);
    assert_eq!(ran.status.code(), Some(0), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "step go on a\nstep skip on a\nstep stop on b\ncleanup b\nstep skip on c\ncleanup c\n\
         step skip on d\ncleanup d\nstep after on d\nmain\n"
    );
}

const CHAIN_BAD_SRC: &str = "use Pilot.Core.AttrFlow;

struct FlowAttr : IStepAttribute {
    pub say: str;
    pub weight: i32 = 0;
    pub fn step(fn_name: str) -> AttrFlow => ::Continue;
}

[Flow]
fn missing() -> i64 => 0;

[Flow(\"a\", \"b\")]
fn extra() -> i64 => 0;

[Flow(tone: \"x\")]
fn unknown() -> i64 => 0;

[Flow(\"a\", say: \"b\")]
fn twice() -> i64 => 0;

[Flow(\"a\", weight: \"heavy\")]
fn heavy() -> i64 => 0;

fn main() {}
";

#[test]
fn attr_chain_checks_user_steps() {
    let dir = scratch("attr_chain_bad");
    let p = program(&dir, "bad.pilot", CHAIN_BAD_SRC);
    let out = pilot(&["check", &p]);
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    for want in [
        "error[ATTR003]: `[Flow]` needs `say`: `FlowAttr` gives it no default",
        "error[ATTR003]: `[Flow]` has more arguments than `FlowAttr` has fields",
        "error[ATTR003]: `[Flow]` has no argument `tone`",
        "error[ATTR003]: `[Flow]` gives `say` twice",
        "error[ATTR003]: the `weight` of `[Flow]` must be an integer",
    ] {
        assert!(err.contains(want), "missing `{want}` in\n{err}");
    }
    assert!(text(&out.stdout).is_empty());
}

/// A step that decides on every call (`IRuntimeStepAttribute`) is not woven
/// into its function yet: the chain stops at seam `attr.runtime_step`.
#[test]
fn attr_runtime_steps_stop_at_their_seam() {
    let dir = scratch("attr_runtime");
    let p = program(&dir, "rt.pilot", "use Pilot.Core.AttrFlow;

struct OncePerCallAttr : IRuntimeStepAttribute {
    pub weight: i32 = 0;
    pub fn on_call(fn_name: str) -> AttrFlow => ::Continue;
}

[OncePerCall]
fn f() -> i64 => 1;

fn main() { print(f()); }
");
    let ran = pilot(&["run", &p]);
    assert_eq!(ran.status.code(), Some(1));
    let err = text(&ran.stderr);
    assert!(err.contains("SEAM001: unconnected seam `attr.runtime_step`"), "{err}");
    assert!(!text(&ran.stdout).contains('1'), "`main` must not run");
}

const INHERIT_CHAIN_SRC: &str = "[Flow(\"pkg\")]
[Tag(\"pkg-tag\", mark: \"p\")]
pub pkg Demo.Chain;

use Pilot.Core.AttrFlow;

// Each [Flow] adds its own step (`::Append`).
[NoInherit]
struct FlowAttr : IStepAttribute, IInheritedAttribute {
    pub say: str;
    pub weight: i32 = 0;
    pub pure fn policy() -> InheritPolicy => ::Append;
    pub fn step(fn_name: str) -> AttrFlow {
        print(f\"flow {self.say} on {fn_name}\");
        return ::Continue;
    }
}

// A [Tag] written closer fills over the fields of the one inherited (`::Coalesce`).
[NoInherit]
struct TagAttr : IStepAttribute, IInheritedAttribute {
    pub label: str;
    pub mark: str = \"-\";
    pub weight: i32 = 0;
    pub pure fn policy() -> InheritPolicy => ::Coalesce;
    pub fn step(fn_name: str) -> AttrFlow {
        print(f\"tag {self.label}/{self.mark} on {fn_name}\");
        return ::Continue;
    }
}

// Only the nearest [Gate] applies (`::Override`, the default).
[NoInherit]
struct GateAttr : IStepAttribute, IInheritedAttribute {
    pub allow: bool;
    pub weight: i32 = 5;
    pub fn step(fn_name: str) -> AttrFlow {
        print(f\"gate {self.allow} on {fn_name}\");
        return if self.allow { ::Continue } else { ::Stop };
    }
}

fn a() -> i64 => 1;

[Flow(\"own\")]
[Tag(mark: \"b\")]
fn b() -> i64 => 2;

[Gate(false)]
struct Door {
    pub id: i64;
    fn knock() -> i64 => 1;
    [Gate(true)]
    fn enter() -> i64 => 2;
    [NoInherit(Gate, Tag)]
    fn wall() -> i64 => 3;
}

[NoInherit]
fn main() { print(\"main\"); }
";

/// Worked out by hand. The package gives every function `[Flow("pkg")]` and
/// `[Tag("pkg-tag", mark: "p")]`; the three `...Attr` structs and `main`
/// opt out with `[NoInherit]`.
/// - `a`: the two inherited steps, as written.
/// - `b`: its `[Flow("own")]` is appended after them; its `[Tag(mark: "b")]`
///   is merged into the inherited one (label kept, mark replaced) in its place.
/// - `Door.knock`: the struct's `[Gate(false)]` (weight 5) runs first and stops.
/// - `Door.enter`: its own `[Gate(true)]` replaces the struct's; it continues.
/// - `Door.wall`: `[NoInherit(Gate, Tag)]` leaves only the package's `[Flow]`.
#[test]
fn inherited_steps_follow_their_policy() {
    let dir = scratch("inherit_chain");
    let p = program(&dir, "chain.pilot", INHERIT_CHAIN_SRC);
    let checked = pilot(&["check", &p]);
    assert_eq!(checked.status.code(), Some(0), "{}", text(&checked.stderr));
    assert_eq!(
        text(&checked.stdout),
        "flow pkg on a\n\
         tag pkg-tag/p on a\n\
         flow pkg on b\n\
         tag pkg-tag/b on b\n\
         flow own on b\n\
         gate false on Door.knock\n\
         gate true on Door.enter\n\
         flow pkg on Door.enter\n\
         tag pkg-tag/p on Door.enter\n\
         flow pkg on Door.wall\n\
         check passed: 1 file(s), no errors\n"
    );
}

/// `[Generate(ext: "txt")]` on the package writes `<function>.txt` for each
/// function that takes nothing and returns `str`, and skips the others.
#[test]
fn generate_on_a_package_writes_each_text_function() {
    let dir = scratch("inherit_gen");
    let p = program(
        &dir,
        "gen.pilot",
        "[Generate(ext: \"txt\")]\npub pkg Demo.Gen;\n\n\
         fn hello() -> str => \"hi\";\n\
         fn number() -> i64 => 1;\n\
         fn takes(x: i64) -> str => \"x\";\n\n\
         fn main() {}\n",
    );
    let checked = pilot(&["check", &p]);
    assert_eq!(checked.status.code(), Some(0), "{}", text(&checked.stderr));
    let d = dir.to_str().expect("utf-8 path");
    assert_eq!(
        text(&checked.stdout),
        format!("{p}:1: generated {d}/hello.txt\ncheck passed: 1 file(s), no errors\n")
    );
    assert_eq!(std::fs::read_to_string(dir.join("hello.txt")).expect("hello.txt written"), "hi");
    assert!(!dir.join("number.txt").exists() && !dir.join("takes.txt").exists() && !dir.join("main.txt").exists());
}

const INHERIT_BAD_SRC: &str = "[Live(1)]
[Generate(\"all.txt\")]
pub pkg Demo.Bad;

use Pilot.Core.AttrFlow;

[NoInherit]
struct PlainAttr : IStepAttribute {
    pub weight: i32 = 0;
    pub fn step(fn_name: str) -> AttrFlow => ::Continue;
}

[Plain]
struct S { pub x: i64; }

[Debug]
enum E { A, B }

[NoInherit(Debug)]
fn f() -> i64 => 1;

fn main() {}
";

#[test]
fn attributes_that_cannot_apply_are_reported() {
    let dir = scratch("inherit_bad");
    let p = program(&dir, "bad.pilot", INHERIT_BAD_SRC);
    let out = pilot(&["check", &p]);
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    for want in [
        "error[ATTR005]: `[Live]` is not inherited, so it cannot apply to the functions of a package",
        "error[ATTR005]: `[Generate]` on a package writes one file per function: name the extension, `[Generate(ext: \"..\")]`",
        "error[ATTR005]: `[Plain]` is not inherited, so it cannot apply to the methods of a struct: `PlainAttr` does not implement `IInheritedAttribute`",
        "error[ATTR005]: `[Debug]` applies to a function, or through a package, struct, interface or ext block",
        "error[ATTR005]: `f` inherits no `[Debug]`",
    ] {
        assert!(err.contains(want), "missing `{want}` in\n{err}");
    }
    assert!(text(&out.stdout).is_empty());
}
