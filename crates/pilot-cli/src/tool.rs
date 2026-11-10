//! Evaluation of the self-hosted Pilot tool scripts (`selfhost/**`) that
//! implement the CLI, the gate and the emitters.
//!
//! A tool script is several source files linked as units of one program,
//! with its entry call as the last unit. Compile errors and runtime faults
//! inside a tool are internal toolchain
//! failures: they are reported with the originating file and line, and the
//! process exits with `EXIT_INTERNAL`.

use crate::link::LinkedSource;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_rt::PilotVal;
use pilot_vm::{BytecodeCompiler, HostOpHandler, PilVirtualMachine};
use std::fs;
use std::path::{Path, PathBuf};

/// Exit code for failures inside the toolchain's own Pilot code (`EX_SOFTWARE`).
pub const EXIT_INTERNAL: i32 = 70;

/// Directories searched for toolchain sources: the working directory and its
/// parents, then the directories above the running executable.
fn search_roots() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("."), PathBuf::from(".."), PathBuf::from("../..")];
    if let Ok(exe) = std::env::current_exe() {
        roots.extend(exe.ancestors().skip(1).map(Path::to_path_buf));
    }
    roots
}

pub fn read_src(rel: &str) -> Option<String> {
    search_roots()
        .into_iter()
        .find_map(|r| fs::read_to_string(r.join(rel)).ok())
}

/// The tool's files and its entry call, each parsed as one unit of a linked
/// program; the entry call is the last unit.
fn link_tool(paths: &[&str], tail: &str) -> LinkedSource {
    let mut files = Vec::with_capacity(paths.len() + 1);
    for &p in paths {
        match read_src(p) {
            Some(s) => files.push((p.to_string(), s)),
            None => {
                eprintln!("internal error: toolchain source `{p}` not found");
                std::process::exit(EXIT_INTERNAL);
            }
        }
    }
    files.push(("<entry>".to_string(), tail.to_string()));
    LinkedSource::parse(files)
}

/// Compiles and runs a tool, returning the VM and the entry call's value.
/// `phy` is the physics registry the script's `__engine` calls operate on.
fn run_tool(src: &LinkedSource, mut phy: PhysicsRegistry, host_ops: Option<HostOpHandler>) -> (PilVirtualMachine, PilotVal) {
    let mut failed = false;
    for (span, msg) in &src.arena.diagnostics {
        eprintln!("{}: internal error: {msg}", src.locate(*span));
        failed = true;
    }
    let bin = BytecodeCompiler::compile_linked(&src.arena);
    for d in &bin.diagnostics {
        eprintln!("{}: internal error: {}", src.locate(d.span), d.message);
        failed = true;
    }
    if failed {
        std::process::exit(EXIT_INTERNAL);
    }
    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(move |op, a| {
        pilot_analysis::handle_engine_intrinsic(&mut phy, op, a)
    }));
    vm.host_ops = host_ops;
    vm.load_binary(bin);
    let out = vm.run();
    if let Some(fault) = vm.fault.take() {
        let at = fault.span.map_or_else(|| "<tool>".to_string(), |s| src.locate(s));
        eprintln!("{at}: internal error: {}", fault.message);
        std::process::exit(EXIT_INTERNAL);
    }
    (vm, out)
}

/// Compiles and runs a tool script, returning its result rendered as text.
fn run_script(src: &LinkedSource, phy: PhysicsRegistry) -> String {
    let (vm, out) = run_tool(src, phy, None);
    vm.display(out)
}

/// The directory holding the toolchain's `stdlib/`, as the search roots
/// find it.
pub fn toolchain_root() -> String {
    search_roots()
        .into_iter()
        .find(|r| r.join("stdlib").is_dir())
        .map_or_else(|| ".".to_string(), |r| r.to_string_lossy().replace('\\', "/"))
}

/// Runs a tool and returns the VM holding its result, and the result.
pub fn eval_value(paths: &[&str], tail: &str) -> (PilVirtualMachine, PilotVal) {
    let src = link_tool(paths, tail);
    let (flat, syms) = pilot_analysis::flatten_ast(&src.arena, src.arena.root);
    run_tool(&src, PhysicsRegistry::new(flat, syms), None)
}

/// Runs the CLI tool with the host operations `ops`; its result is the
/// process exit code.
pub fn eval_cli(paths: &[&str], tail: &str, ops: HostOpHandler) -> i32 {
    let src = link_tool(paths, tail);
    let (flat, syms) = pilot_analysis::flatten_ast(&src.arena, src.arena.root);
    let (_, out) = run_tool(&src, PhysicsRegistry::new(flat, syms), Some(ops));
    out.as_int().map_or(EXIT_INTERNAL, |c| c.clamp(0, 255) as i32)
}

/// Runs the tool made of `tool_files`, calling `entry`, with the text of the
/// file at `path` served through the host operation `tool_input_text`; its
/// result is the tool's exit code.
pub fn run_on_text(path: &str, tool_files: &[String], entry: &str) -> i32 {
    let input = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{path}: error: cannot read file: {e}");
            return crate::run::EXIT_FAILURE;
        }
    };
    let ops: HostOpHandler = Box::new(move |op, _| match op {
        "tool_input_text" => Some(Ok(pilot_vm::HostValue::Str(input.clone()))),
        _ => None,
    });
    let paths: Vec<&str> = tool_files.iter().map(String::as_str).collect();
    eval_cli(&paths, entry, ops)
}

/// Runs a tool with the host operations `ops` and returns its result as a
/// host value.
pub fn eval_to_host(paths: &[&str], tail: &str, ops: HostOpHandler) -> pilot_vm::HostValue {
    let src = link_tool(paths, tail);
    let (flat, syms) = pilot_analysis::flatten_ast(&src.arena, src.arena.root);
    let (vm, out) = run_tool(&src, PhysicsRegistry::new(flat, syms), Some(ops));
    vm.to_host(out)
}

/// Runs a tool whose engine calls see the tool's own AST.
pub fn eval(paths: &[&str], tail: &str) -> String {
    let src = link_tool(paths, tail);
    let (flat, syms) = pilot_analysis::flatten_ast(&src.arena, src.arena.root);
    run_script(&src, PhysicsRegistry::new(flat, syms))
}

/// Runs an emitter tool whose engine calls see the AST of `input_path`.
pub fn eval_on_input(paths: &[&str], tail: &str, input_path: &str) -> String {
    let input = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{input_path}: error: cannot read file: {e}");
            std::process::exit(1);
        }
    };
    let input_arena = parse(&input);
    let (flat, syms) = pilot_analysis::flatten_ast(&input_arena, input_arena.root);
    let mut phy = PhysicsRegistry::new(flat, syms);
    phy.source_text = input;
    run_script(&link_tool(paths, tail), phy)
}
