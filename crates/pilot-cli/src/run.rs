//! Compiling and running user programs for the host operations of the CLI.
//!
//! `compile` links a program through the Pilot module loader, parses it and
//! lowers it to bytecode, collecting every diagnostic instead of printing it:
//! the Pilot CLI renders them (`selfhost/cli/report.pilot`). `execute` runs a
//! compiled program and maps its outcome to a process exit code.
//!
//! Exit codes: the integer returned by `main` (0 when it returns nothing);
//! `1` for runtime faults.

use crate::check;
use crate::link::LinkedSource;
use crate::tool;
use pilot_analysis::PhysicsRegistry;
use pilot_rt::PilotVal;
use pilot_span::ByteSpan;
use pilot_vm::lower_drop::{DropAction, DropGlue, DropItem, DropPlan};
use pilot_vm::{codes, BytecodeCompiler, HostValue, PilVirtualMachine, PilcBinary};
use std::fs;

pub const EXIT_FAILURE: i32 = 1;

/// The loader's code for a file that cannot be read (`selfhost/loader/module_graph.pilot`).
const UNREADABLE: &str = "MOD001";

/// Code for a fault inside the self-hosted analyzer itself.
const ANALYZER_FAULT: &str = "VM001";

/// A diagnostic found while linking, parsing or compiling a program. Offsets
/// are byte offsets into `file`.
pub struct Diag {
    pub code: String,
    pub message: String,
    pub file: String,
    pub start: u32,
    pub end: u32,
    /// 1 error, 2 warning, 3 info, 4 hint (`diag_severity_*` in
    /// `selfhost/diagnostics/model.pilot`).
    pub severity: u8,
}

/// Severity of the seed's own diagnostics.
const ERROR: u8 = 1;

impl Diag {
    /// `[code, message, file, start, end]`, the shape the Pilot CLI reads.
    pub fn to_host(&self) -> HostValue {
        HostValue::List(vec![
            HostValue::Str(self.code.clone()),
            HostValue::Str(self.message.clone()),
            HostValue::Str(self.file.clone()),
            HostValue::Int(i64::from(self.start)),
            HostValue::Int(i64::from(self.end)),
            HostValue::Int(i64::from(self.severity)),
        ])
    }

    fn whole_file(file: &str, code: &str, message: String) -> Self {
        Diag {
            code: code.to_string(),
            message,
            file: file.to_string(),
            start: 0,
            end: 0,
            severity: ERROR,
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
pub(crate) fn link_plan(path: &str) -> Result<Vec<String>, Vec<Diag>> {
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
                severity: ERROR,
            }
        })
        .collect())
}

/// Links, parses and compiles the program at `path`. `tail`, when given, is
/// source text linked after the program as its entry unit: it runs instead of
/// the program's `main` (the test runner calls one function this way).
pub fn compile(path: &str, tail: Option<&str>) -> Result<Program, Vec<Diag>> {
    compile_planned(path, tail, None)
}

/// The seed's drop plan from the checker's, its nodes moved into the
/// offsets of `source`.
fn seed_drop_plan(source: &LinkedSource, plan: &check::CheckedDropPlan) -> DropPlan {
    let bases: std::collections::HashMap<&str, u32> = source.units().map(|(p, _, b)| (p, b)).collect();
    let mut out = DropPlan::default();
    for (ty, owner, fields) in &plan.glue {
        out.glue.insert(
            ty.clone(),
            DropGlue {
                destructor_owner: (!owner.is_empty()).then(|| owner.clone()),
                fields: fields.clone(),
            },
        );
    }
    for r in &plan.rows {
        let Some(&base) = bases.get(r.file.as_str()) else { continue };
        let key = (base + r.start, base + r.end, r.kind);
        let item = || DropItem { binding: r.a.clone(), ty: r.b.clone() };
        match r.action {
            DropAction::Before => out.before.entry(key).or_default().push(item()),
            DropAction::After => out.after.entry(key).or_default().push(item()),
            DropAction::Exit => out.exit.entry(key).or_default().push(item()),
            DropAction::Else => out.on_else.entry(key).or_default().push(item()),
            DropAction::Guard => out.guard.entry(key).or_default().push(item()),
            DropAction::Seam => out.seams.entry(key).or_default().push((r.a.clone(), r.b.clone())),
            DropAction::Glue => {}
        }
    }
    out
}

/// `compile`, dropping owned values where the checker's `plan` says (F7).
pub fn compile_planned(path: &str, tail: Option<&str>, plan: Option<&check::CheckedDropPlan>) -> Result<Program, Vec<Diag>> {
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
            severity: ERROR,
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
    let drops = plan.map(|p| seed_drop_plan(&source, p)).unwrap_or_default();
    let bin = BytecodeCompiler::compile_linked_with_drops(&source.arena, drops);
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

/// The self-hosted analyzer's diagnostics for the program at `path`
/// (`selfhost/analyzer.pilot`, run by `pilot_analysis`). The analyzer reads
/// the whole linked program, so a declaration in an imported file is seen
/// where it is used; only diagnostics inside the user's own files (not the
/// toolchain's `stdlib/`) are kept. A program that does not link is read on
/// its own: `pilot check` reports why it does not link. A fault inside the
/// analyzer is reported as an error, never dropped.
pub fn analyze(path: &str) -> Vec<Diag> {
    let paths = link_plan(path).unwrap_or_else(|_| vec![path.to_string()]);
    let mut sources = Vec::with_capacity(paths.len());
    let mut user = Vec::new();
    for p in paths {
        let Ok(src) = fs::read_to_string(&p) else {
            return Vec::new();
        };
        if !crate::check::in_stdlib(&p) {
            user.push(p.clone());
        }
        sources.push((p, src));
    }
    let linked = LinkedSource::parse(sources);
    let result = pilot_analysis::AnalysisPipeline::analyze(&linked.arena, linked.arena.root);
    let mut out: Vec<Diag> = result
        .diagnostics
        .iter()
        .filter_map(|d| {
            let (file, start, end) = linked.resolve(d.span);
            if !user.contains(&file) {
                return None;
            }
            Some(Diag {
                code: d.code.clone(),
                message: d
                    .message
                    .strip_prefix(&format!("{}: ", d.code))
                    .unwrap_or(&d.message)
                    .to_string(),
                file,
                start,
                end,
                severity: d.severity.as_lsp_code(),
            })
        })
        .collect();
    if let Some(e) = &result.physics.analyzer_error {
        out.push(Diag::whole_file(path, ANALYZER_FAULT, format!("the analyzer stopped: {e}")));
    }
    out
}
