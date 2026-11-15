//! A method call on an interface-typed receiver, in a program linked flat
//! (`BytecodeCompiler::compile_linked`, as the toolchain links `pilot emit`),
//! runs the method of the struct the receiver record was built as, never a
//! free function that happens to share the method's bare name.
//!
//! Imported functions and methods compile lazily, in the order calls reach
//! them. The bug: a call to `Owner.method` made while that method was still
//! uncompiled, but a free function of the same bare name already was, bound
//! to the free function on the spot.

use pilot_ast::AstArena;
use pilot_parser::parse_unit;
use pilot_span::FileId;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

/// Parses `units` as one linked program (the last is the entry unit), runs
/// it, and returns its integer result or the fault message.
fn run_linked(units: &[&str]) -> Result<i64, String> {
    let mut arena = AstArena::new();
    let mut base = 0u32;
    for (i, src) in units.iter().enumerate() {
        arena = parse_unit(arena, src, FileId(i as u32), base);
        base += src.len() as u32 + 1;
    }
    assert!(arena.diagnostics.is_empty(), "units must parse: {:?}", arena.diagnostics);
    let bin = BytecodeCompiler::compile_linked(&arena);
    let msgs: Vec<&str> = bin.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert!(msgs.is_empty(), "units must compile: {msgs:?}");
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(bin);
    let out = vm.run();
    if let Some(fault) = vm.fault.take() {
        return Err(fault.message);
    }
    out.as_int().ok_or_else(|| format!("result is not an integer: {out}"))
}

/// A private free function named like the interface method, in another unit
/// (as `selfhost/ir/pass_fold.pilot`'s `compare` is to `ISourceDialect`).
/// `warm` makes the free function compile before the caller of the method.
const FREE_FN_UNIT: &str = "\
fn compare(pred: i64, a: i64, b: i64) -> bool => a >= b;
pub fn warm() -> bool => compare(0, 2, 1);
";

/// The interface, two implementing structs, and a caller that only knows
/// the receiver as the interface.
const DIALECT_UNIT: &str = "\
pub interface IDialect {
    fn compare(pred: i64, a: str, b: str, width: i64) -> i64;
}
pub struct Narrow : IDialect {
    pub fn compare(pred: i64, a: str, b: str, width: i64) -> i64 => pred * 10 + width;
}
pub struct Wide : IDialect {
    pub fn compare(pred: i64, a: str, b: str, width: i64) -> i64 => pred * 100 + width;
}
pub fn narrow() -> IDialect => Narrow {};
pub fn wide() -> IDialect => Wide {};
pub fn ask(d: IDialect) -> i64 => d.compare(5, \"x\", \"y\", 3);
pub fn ask_narrow() -> i64 => ask(narrow());
pub fn ask_wide() -> i64 => ask(wide());
";

#[test]
fn interface_receiver_runs_the_records_struct_method() {
    // Hand-worked: Narrow gives 5 * 10 + 3, Wide gives 5 * 100 + 3.
    // `warm()` is called first, so the free `compare` compiles in the same
    // lazy round as `ask`, before it.
    assert_eq!(run_linked(&[FREE_FN_UNIT, DIALECT_UNIT, "warm();\nask_narrow();\n"]), Ok(53));
    assert_eq!(run_linked(&[FREE_FN_UNIT, DIALECT_UNIT, "warm();\nask_wide();\n"]), Ok(503));
    // Without the free function compiled first.
    assert_eq!(run_linked(&[FREE_FN_UNIT, DIALECT_UNIT, "ask_narrow();\n"]), Ok(53));
}

#[test]
fn interface_receiver_with_no_implementing_struct_traps() {
    // No struct implements the method, so the call must stop the program
    // instead of running the free `compare` (which would return a bool).
    let unit = "\
pub interface IDialect {
    fn compare(pred: i64, a: str, b: str, width: i64) -> i64;
}
pub struct Plain { pub n: i64; }
pub fn ask(d: IDialect) -> i64 => d.compare(5, \"x\", \"y\", 3);
pub fn ask_plain() -> i64 => ask(Plain { n: 1 });
";
    let out = run_linked(&[FREE_FN_UNIT, unit, "warm();\nask_plain();\n"]);
    assert!(out.as_ref().is_err_and(|m| m.contains("compare")), "expected a trap naming `compare`, got {out:?}");
}

