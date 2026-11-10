//! `pilot run` execution path: compile, refuse on errors, run, map the
//! outcome to a process exit code.
//!
//! Exit codes: the integer returned by `main` (0 when it returns nothing);
//! `1` for syntax errors, compile errors and runtime faults.

use crate::link::LinkedSource;
use crate::tool;
use pilot_analysis::PhysicsRegistry;
use pilot_rt::PilotVal;
use pilot_span::ByteSpan;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine, PilcBinary};
use std::fs;

const EXIT_FAILURE: i32 = 1;

/// 1-based line and column of a byte offset.
fn line_col(src: &str, offset: u32) -> (usize, usize) {
    let upto = &src[..(offset as usize).min(src.len())];
    let line = upto.matches('\n').count() + 1;
    let col = upto.rfind('\n').map_or(upto.len(), |nl| upto.len() - nl - 1) + 1;
    (line, col)
}


/// Converts `main`'s return value into an exit code.
fn exit_code_of(v: pilot_rt::PilotVal) -> i32 {
    v.as_int().map_or(0, |i| i.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

/// Runs the loaded program; `locate` names the source position of a span.
fn execute(vm: &mut PilVirtualMachine, locate: impl Fn(ByteSpan) -> String) -> i32 {
    let result = vm.run();
    if let Some(fault) = vm.fault.take() {
        let at = fault.span.map_or_else(|| "<program>".to_string(), &locate);
        eprintln!("{at}: runtime error: {}", fault.message);
        return EXIT_FAILURE;
    }
    exit_code_of(result)
}

/// The Pilot sources of the module loader (`mg_link_plan`).
const LOADER: &[&str] = &[
    "stdlib/core/file.pilot",
    "selfhost/diagnostics/model.pilot",
    "selfhost/loader/module_graph.pilot",
];

/// Escapes `s` for a Pilot string literal.
fn pilot_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn val_str(vm: &PilVirtualMachine, v: PilotVal) -> String {
    v.with_str(|s| s.to_string()).unwrap_or_else(|| vm.display(v))
}

/// The files of the program whose entry is `path`, in link order, as the
/// loader resolves its `use` declarations. Loader diagnostics are reported
/// and give `Err`.
fn link_plan(path: &str) -> Result<Vec<String>, ()> {
    let root = tool::toolchain_root();
    let call = format!("mg_link_plan(\"{}\", \"{}\");", pilot_str(path), pilot_str(&root));
    let (vm, plan) = tool::eval_value(LOADER, &call);
    let parts = vm.list_items(plan).unwrap_or_default();
    let paths = parts.first().and_then(|&p| vm.list_items(p)).unwrap_or_default();
    let diags = parts.get(1).and_then(|&d| vm.list_items(d)).unwrap_or_default();
    if diags.is_empty() {
        return Ok(paths.into_iter().map(|p| val_str(&vm, p)).collect());
    }
    for d in diags {
        let f = vm.list_items(d).unwrap_or_default();
        let field = |i: usize| f.get(i).map_or_else(String::new, |&v| val_str(&vm, v));
        let offset = f.get(3).and_then(|v| v.as_int()).unwrap_or(0) as u32;
        let file = field(2);
        let at = match fs::read_to_string(&file) {
            Ok(src) => {
                let (line, col) = line_col(&src, offset);
                format!("{file}:{line}:{col}")
            }
            Err(_) => file,
        };
        eprintln!("{at}: error: {}: {}", field(0), field(1));
    }
    Err(())
}

/// Runs a `.pilot` source file or a precompiled `.pilc` binary. `args` are
/// the script arguments (after `--`).
pub fn run_file(path: &str, args: Vec<String>) -> i32 {
    if path.ends_with(".pilc") {
        let bin = match fs::read(path).map_err(|e| e.to_string()).and_then(|b| PilcBinary::load_from_bytes(&b)) {
            Ok(bin) => bin,
            Err(e) => {
                eprintln!("{path}: error: {e}");
                return EXIT_FAILURE;
            }
        };
        let mut vm = PilVirtualMachine::new();
        vm.set_script_args(args);
        vm.load_binary(bin);
        return execute(&mut vm, |_| path.to_string());
    }
    if let Err(e) = fs::metadata(path) {
        eprintln!("{path}: error: cannot read file: {e}");
        return EXIT_FAILURE;
    }
    let Ok(paths) = link_plan(path) else {
        return EXIT_FAILURE;
    };
    let mut files = Vec::with_capacity(paths.len());
    for p in paths {
        match fs::read_to_string(&p) {
            Ok(src) => files.push((p, src)),
            Err(e) => {
                eprintln!("{p}: error: cannot read file: {e}");
                return EXIT_FAILURE;
            }
        }
    }
    let program = LinkedSource::parse(files);
    if !program.arena.diagnostics.is_empty() {
        for (span, msg) in &program.arena.diagnostics {
            eprintln!("{}: error: {msg}", program.locate(*span));
        }
        return EXIT_FAILURE;
    }
    let bin = BytecodeCompiler::compile_linked(&program.arena);
    if !bin.diagnostics.is_empty() {
        for d in &bin.diagnostics {
            eprintln!("{}: error: {}", program.locate(d.span), d.message);
        }
        return EXIT_FAILURE;
    }
    let (flat, syms) = pilot_analysis::flatten_ast(&program.arena, program.arena.root);
    let mut phy = PhysicsRegistry::new(flat, syms);
    let mut vm = PilVirtualMachine::new();
    vm.set_script_args(args);
    vm.engine_handler = Some(Box::new(move |op, a| {
        pilot_analysis::handle_engine_intrinsic(&mut phy, op, a)
    }));
    vm.load_binary(bin);
    execute(&mut vm, |span| program.locate(span))
}
