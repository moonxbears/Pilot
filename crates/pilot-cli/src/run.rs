//! Compiling and running user programs for the host operations of the CLI.
//!
//! `compile` links a program through the Pilot module loader, parses it and
//! lowers it to bytecode, collecting every diagnostic instead of printing it:
//! the Pilot CLI renders them (`selfhost/cli/report.pilot`). `execute` runs a
//! compiled program and maps its outcome to a process exit code.
//!
//! Exit codes: the integer returned by `main` (0 when it returns nothing);
//! `1` for runtime faults.

use crate::link::LinkedSource;
use crate::tool;
use pilot_analysis::PhysicsRegistry;
use pilot_rt::PilotVal;
use pilot_span::ByteSpan;
use pilot_vm::{codes, BytecodeCompiler, HostValue, PilVirtualMachine, PilcBinary};
use std::fs;

pub const EXIT_FAILURE: i32 = 1;

/// The loader's code for a file that cannot be read (`selfhost/loader/module_graph.pilot`).
const UNREADABLE: &str = "MOD001";

/// A diagnostic found while linking, parsing or compiling a program. Offsets
/// are byte offsets into `file`.
pub struct Diag {
    pub code: String,
    pub message: String,
    pub file: String,
    pub start: u32,
    pub end: u32,
}

impl Diag {
    /// `[code, message, file, start, end]`, the shape the Pilot CLI reads.
    pub fn to_host(&self) -> HostValue {
        HostValue::List(vec![
            HostValue::Str(self.code.clone()),
            HostValue::Str(self.message.clone()),
            HostValue::Str(self.file.clone()),
            HostValue::Int(i64::from(self.start)),
            HostValue::Int(i64::from(self.end)),
        ])
    }

    fn whole_file(file: &str, code: &str, message: String) -> Self {
        Diag {
            code: code.to_string(),
            message,
            file: file.to_string(),
            start: 0,
            end: 0,
        }
    }
}

/// A program ready to run.
pub struct Program {
    source: Option<LinkedSource>,
    bin: PilcBinary,
}

/// Converts `main`'s return value into an exit code.
fn exit_code_of(v: PilotVal) -> i32 {
    v.as_int().map_or(0, |i| i.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

/// The Pilot sources of the module loader (`mg_link_plan`).
const LOADER: &[&str] = &[
    "stdlib/core/file.pilot",
    "selfhost/diagnostics/model.pilot",
    "selfhost/loader/module_graph.pilot",
];

/// Escapes `s` for a Pilot string literal.
pub fn pilot_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn val_str(vm: &PilVirtualMachine, v: PilotVal) -> String {
    v.with_str(|s| s.to_string()).unwrap_or_else(|| vm.display(v))
}

/// The files of the program whose entry is `path`, in link order, as the
/// loader resolves its `use` declarations.
fn link_plan(path: &str) -> Result<Vec<String>, Vec<Diag>> {
    let root = tool::toolchain_root();
    let call = format!("mg_link_plan(\"{}\", \"{}\");", pilot_str(path), pilot_str(&root));
    let (vm, plan) = tool::eval_value(LOADER, &call);
    let parts = vm.list_items(plan).unwrap_or_default();
    let paths = parts.first().and_then(|&p| vm.list_items(p)).unwrap_or_default();
    let diags = parts.get(1).and_then(|&d| vm.list_items(d)).unwrap_or_default();
    if diags.is_empty() {
        return Ok(paths.into_iter().map(|p| val_str(&vm, p)).collect());
    }
    Err(diags
        .into_iter()
        .map(|d| {
            let f = vm.list_items(d).unwrap_or_default();
            let field = |i: usize| f.get(i).map_or_else(String::new, |&v| val_str(&vm, v));
            let offset = f.get(3).and_then(|v| v.as_int()).unwrap_or(0) as u32;
            Diag {
                code: field(0),
                message: field(1),
                file: field(2),
                start: offset,
                end: offset,
            }
        })
        .collect())
}

/// Links, parses and compiles the program at `path`. `tail`, when given, is
/// source text linked after the program as its entry unit: it runs instead of
/// the program's `main` (the test runner calls one function this way).
pub fn compile(path: &str, tail: Option<&str>) -> Result<Program, Vec<Diag>> {
    if path.ends_with(".pilc") {
        return fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|b| PilcBinary::load_from_bytes(&b))
            .map(|bin| Program { source: None, bin })
            .map_err(|e| vec![Diag::whole_file(path, codes::SYNTAX, e)]);
    }
    if let Err(e) = fs::metadata(path) {
        return Err(vec![Diag::whole_file(path, UNREADABLE, format!("cannot read file: {e}"))]);
    }
    let paths = link_plan(path)?;
    let mut files = Vec::with_capacity(paths.len() + 1);
    for p in paths {
        match fs::read_to_string(&p) {
            Ok(src) => files.push((p, src)),
            Err(e) => return Err(vec![Diag::whole_file(&p, UNREADABLE, format!("cannot read file: {e}"))]),
        }
    }
    if let Some(t) = tail {
        files.push(("<entry>".to_string(), t.to_string()));
    }
    let source = LinkedSource::parse(files);
    let locate = |span: ByteSpan, code: &str, message: &str| {
        let (file, start, end) = source.resolve(span);
        Diag {
            code: code.to_string(),
            message: message.to_string(),
            file,
            start,
            end,
        }
    };
    let parse_diags: Vec<Diag> = source
        .arena
        .diagnostics
        .iter()
        .map(|(span, msg)| locate(*span, codes::SYNTAX, msg))
        .collect();
    if !parse_diags.is_empty() {
        return Err(parse_diags);
    }
    let bin = BytecodeCompiler::compile_linked(&source.arena);
    if !bin.diagnostics.is_empty() {
        return Err(bin
            .diagnostics
            .iter()
            .map(|d| locate(d.span, d.code, &d.message))
            .collect());
    }
    Ok(Program {
        source: Some(source),
        bin,
    })
}

/// Runs a compiled program with the script arguments `args`. Runtime faults
/// are reported on stderr with their call trace.
pub fn execute(program: Program, args: Vec<String>) -> i32 {
    let Program { source, bin } = program;
    let mut vm = PilVirtualMachine::new();
    vm.set_script_args(args);
    if let Some(src) = &source {
        let (flat, syms) = pilot_analysis::flatten_ast(&src.arena, src.arena.root);
        let mut phy = PhysicsRegistry::new(flat, syms);
        vm.engine_handler = Some(Box::new(move |op, a| {
            pilot_analysis::handle_engine_intrinsic(&mut phy, op, a)
        }));
    }
    vm.load_binary(bin);
    let result = vm.run();
    let Some(fault) = vm.fault.take() else {
        return exit_code_of(result);
    };
    let locate = |span: ByteSpan| match &source {
        Some(s) => s.locate(span),
        None => "<program>".to_string(),
    };
    let at = fault.span.map_or_else(|| "<program>".to_string(), locate);
    eprintln!("{at}: runtime error: {}", fault.message);
    // Consecutive calls from one place (recursion) print once.
    let mut i = 0;
    while i < fault.trace.len() {
        let run = fault.trace[i..].iter().take_while(|&&s| s == fault.trace[i]).count();
        match run {
            1 => eprintln!("  called from {}", locate(fault.trace[i])),
            n => eprintln!("  called from {} ({n} times)", locate(fault.trace[i])),
        }
        i += run;
    }
    if fault.depth > fault.trace.len() {
        eprintln!("  ... and {} more calls", fault.depth - fault.trace.len());
    }
    EXIT_FAILURE
}

/// The top-level functions `path` declares, each as `[name, [attribute
/// names]]`, as the seed parser reads them. The test runner picks its tests
/// from these by attribute.
pub fn declared_functions(path: &str) -> Option<HostValue> {
    let src = fs::read_to_string(path).ok()?;
    let arena = pilot_parser::parse(&src);
    let root = arena.node(arena.root);
    let mut out = Vec::new();
    for &c in arena.children_slice(root.lhs, root.rhs) {
        let n = arena.node(pilot_ast::NodeId(c));
        if n.node_kind() != pilot_ast::NodeKind::FnDecl {
            continue;
        }
        let name = arena.interner.resolve(pilot_intern::SymbolId(n.lhs)).to_string();
        let attrs = arena
            .attrs
            .iter()
            .filter(|a| a.target.0 == c)
            .map(|a| HostValue::Str(arena.interner.resolve(a.name).to_string()))
            .collect();
        out.push(HostValue::List(vec![HostValue::Str(name), HostValue::List(attrs)]));
    }
    Some(HostValue::List(out))
}
