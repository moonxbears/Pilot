//! Evaluation of the self-hosted Pilot tool scripts (`selfhost/**`) that
//! implement the CLI, the gate and the emitters.
//!
//! A tool script is several source files concatenated with an entry call.
//! Compile errors and runtime faults inside a tool are internal toolchain
//! failures: they are reported with the originating file and line, and the
//! process exits with `EXIT_INTERNAL`.

use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_span::ByteSpan;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};
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

/// Concatenated tool source with the byte range each file occupies.
struct ToolSource {
    text: String,
    files: Vec<(String, usize)>,
}

impl ToolSource {
    fn assemble(paths: &[&str], tail: &str) -> Self {
        let (mut text, mut files) = (String::new(), Vec::new());
        for &p in paths {
            match read_src(p) {
                Some(s) => {
                    files.push((p.to_string(), text.len()));
                    text.push_str(&s);
                    text.push('\n');
                }
                None => {
                    eprintln!("internal error: toolchain source `{p}` not found");
                    std::process::exit(EXIT_INTERNAL);
                }
            }
        }
        files.push(("<entry>".to_string(), text.len()));
        text.push('\n');
        text.push_str(tail);
        Self { text, files }
    }

    /// `file:line:col` for a byte offset in the concatenated text.
    fn locate(&self, span: ByteSpan) -> String {
        let off = (span.start as usize).min(self.text.len());
        let (name, start) = self
            .files
            .iter()
            .rev()
            .find(|(_, s)| *s <= off)
            .cloned()
            .unwrap_or_else(|| ("<tool>".to_string(), 0));
        let upto = &self.text[start..off];
        let line = upto.matches('\n').count() + 1;
        let col = upto.rfind('\n').map_or(upto.len(), |nl| upto.len() - nl - 1) + 1;
        format!("{name}:{line}:{col}")
    }
}

/// Compiles and runs a tool script, returning its result rendered as text.
/// `phy` is the physics registry the script's `__engine` calls operate on.
fn run_script(src: &ToolSource, mut phy: PhysicsRegistry) -> String {
    let arena = parse(&src.text);
    let mut failed = false;
    for (span, msg) in &arena.diagnostics {
        eprintln!("{}: internal error: {msg}", src.locate(*span));
        failed = true;
    }
    let bin = BytecodeCompiler::compile(&arena, arena.root);
    for d in &bin.diagnostics {
        eprintln!("{}: internal error: {}", src.locate(d.span), d.message);
        failed = true;
    }
    if failed {
        std::process::exit(EXIT_INTERNAL);
    }
    let mut vm = PilVirtualMachine::new();
    let reg_ptr = &mut phy as *mut PhysicsRegistry;
    vm.engine_handler = Some(Box::new(move |op, a| {
        pilot_analysis::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, a)
    }));
    vm.load_binary(bin);
    let out = vm.run();
    if let Some(fault) = vm.fault.take() {
        let at = fault.span.map_or_else(|| "<tool>".to_string(), |s| src.locate(s));
        eprintln!("{at}: internal error: {}", fault.message);
        std::process::exit(EXIT_INTERNAL);
    }
    vm.display(out)
}

/// Runs a tool whose engine calls see the tool's own AST.
pub fn eval(paths: &[&str], tail: &str) -> String {
    let src = ToolSource::assemble(paths, tail);
    let arena = parse(&src.text);
    let (flat, syms) = pilot_analysis::flatten_ast(&arena, arena.root);
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
    run_script(&ToolSource::assemble(paths, tail), phy)
}
