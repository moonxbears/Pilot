//! `pilot run` execution path: compile, refuse on errors, run, map the
//! outcome to a process exit code.
//!
//! Exit codes: the integer returned by `main` (0 when it returns nothing);
//! `1` for syntax errors, compile errors and runtime faults.

use pilot_analysis::{AnalysisPipeline, PhysicsRegistry};
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

fn report(path: &str, src: &str, span: Option<ByteSpan>, kind: &str, message: &str) {
    match span {
        Some(s) if !src.is_empty() => {
            let (line, col) = line_col(src, s.start);
            eprintln!("{path}:{line}:{col}: {kind}: {message}");
        }
        _ => eprintln!("{path}: {kind}: {message}"),
    }
}

/// Converts `main`'s return value into an exit code.
fn exit_code_of(v: pilot_rt::PilotVal) -> i32 {
    v.as_int().map_or(0, |i| i.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

fn execute(vm: &mut PilVirtualMachine, path: &str, src: &str) -> i32 {
    let result = vm.run();
    if let Some(fault) = vm.fault.take() {
        report(path, src, fault.span, "runtime error", &fault.message);
        return EXIT_FAILURE;
    }
    exit_code_of(result)
}

/// Runs a `.pilot` source file or a precompiled `.pilc` binary.
pub fn run_file(path: &str) -> i32 {
    if path.ends_with(".pilc") {
        let bin = match fs::read(path).map_err(|e| e.to_string()).and_then(|b| PilcBinary::load_from_bytes(&b)) {
            Ok(bin) => bin,
            Err(e) => {
                eprintln!("{path}: error: {e}");
                return EXIT_FAILURE;
            }
        };
        let mut vm = PilVirtualMachine::new();
        vm.load_binary(bin);
        return execute(&mut vm, path, "");
    }
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{path}: error: cannot read file: {e}");
            return EXIT_FAILURE;
        }
    };
    let (arena, mut phy) = AnalysisPipeline::load_program(path);
    if !arena.diagnostics.is_empty() {
        for (span, msg) in &arena.diagnostics {
            report(path, &src, Some(*span), "error", msg);
        }
        return EXIT_FAILURE;
    }
    let bin = BytecodeCompiler::compile(&arena, arena.root);
    if !bin.diagnostics.is_empty() {
        for d in &bin.diagnostics {
            report(path, &src, Some(d.span), "error", &d.message);
        }
        return EXIT_FAILURE;
    }
    let mut vm = PilVirtualMachine::new();
    let reg_ptr = &mut phy as *mut PhysicsRegistry;
    vm.engine_handler = Some(Box::new(move |op, a| {
        pilot_analysis::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, a)
    }));
    vm.load_binary(bin);
    execute(&mut vm, path, &src)
}
