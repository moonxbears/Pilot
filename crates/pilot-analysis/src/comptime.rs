//! Comptime execution engine & Pilot-driven bootstrap snapshot (`pilot-vm`).

use std::path::Path;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_parser::ParserTables;
use pilot_rt::PilotVal;
use pilot_span::ByteSpan;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};
use crate::types::{LayoutId, LayoutTable};

#[derive(Clone, Debug, PartialEq)]
pub enum ComptimeValue {
    Void, Int(i64), Uint(u128), Float(f64), Bool(bool), Str(SymbolId), Ident(SymbolId), AstNode(NodeId),
}

impl ComptimeValue {
    pub fn as_int(&self) -> Option<i64> { match self { Self::Int(v) => Some(*v), Self::Uint(v) => Some(*v as i64), _ => None } }
    pub fn as_uint(&self) -> Option<u128> { match self { Self::Uint(v) => Some(*v), Self::Int(v) if *v >= 0 => Some(*v as u128), _ => None } }
    pub fn as_bool(&self) -> Option<bool> { match self { Self::Bool(b) => Some(*b), _ => None } }
    pub fn as_str<'a>(&'a self, interner: &'a pilot_intern::Interner) -> Option<&'a str> {
        match self { Self::Str(sym) | Self::Ident(sym) => Some(interner.resolve(*sym)), _ => None }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BootstrapDiagnostic {
    ParseError { message: String, span: ByteSpan },
    ExecutionError { message: String, span: ByteSpan },
    BootstrapBridgeSealedError { message: String, span: ByteSpan },
    UserCompileError { message: String, span: ByteSpan },
    UnknownEngineHook { hook_name: String, span: ByteSpan },
}

impl BootstrapDiagnostic {
    pub fn message(&self) -> &str {
        match self {
            Self::ParseError { message, .. } | Self::ExecutionError { message, .. }
            | Self::BootstrapBridgeSealedError { message, .. } | Self::UserCompileError { message, .. } => message,
            Self::UnknownEngineHook { hook_name, .. } => hook_name,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ComptimeEngine {
    pub layout_table: LayoutTable, pub node_layouts: Vec<Option<LayoutId>>, pub diagnostics: Vec<BootstrapDiagnostic>,
}

impl ComptimeEngine {
    pub fn new() -> Self { Self::default() }
    pub fn eval_ast_node(&mut self, arena: &AstArena, node_id: NodeId) -> Result<ComptimeValue, BootstrapDiagnostic> {
        if node_id.is_none() { return Ok(ComptimeValue::Void); }
        let mut vm = PilVirtualMachine::new();
        vm.load_binary(BytecodeCompiler::compile(arena, node_id));
        let (flat_arena, symbol_strings) = crate::infer::flatten_ast(arena, node_id);
        let mut registry = crate::infer::PhysicsRegistry::new(flat_arena, symbol_strings);
        registry.layouts = self.layout_table.clone();
        let reg_ptr = &mut registry as *mut crate::infer::PhysicsRegistry;
        vm.engine_handler = Some(Box::new(move |op: &str, args: &[PilotVal]| crate::infer::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, args)));
        let ret = vm.run();
        self.layout_table = registry.layouts;
        self.node_layouts = registry.node_layouts;
        for d in registry.diagnostics {
            self.diagnostics.push(BootstrapDiagnostic::UserCompileError { message: d.message, span: arena.span(NodeId(d.node_id)) });
        }
        if let Some(v) = ret.as_int() { Ok(ComptimeValue::Int(v)) }
        else if let Some(v) = ret.as_float() { Ok(ComptimeValue::Float(v)) }
        else if let Some(v) = ret.as_bool() { Ok(ComptimeValue::Bool(v)) }
        else { Ok(ComptimeValue::Void) }
    }
}

pub struct ComptimeEvaluator<'a> { pub engine: &'a mut ComptimeEngine }
impl<'a> ComptimeEvaluator<'a> {
    pub fn new(engine: &'a mut ComptimeEngine) -> Self { Self { engine } }
    pub fn eval_comptime_block(&mut self, arena: &AstArena, root: NodeId) -> Result<ComptimeValue, BootstrapDiagnostic> { self.engine.eval_ast_node(arena, root) }
}

pub fn bootstrap_core_file(source: &str, parser_tables: &mut ParserTables, comptime_vm: &mut ComptimeEngine) -> Result<AstArena, BootstrapDiagnostic> {
    let arena = pilot_parser::parse_with_tables(source, parser_tables);
    for (idx, node) in arena.nodes.iter().enumerate() {
        if node.node_kind() == NodeKind::ComptimeExpr { let _ = comptime_vm.eval_ast_node(&arena, NodeId(idx as u32)); }
    }
    Ok(arena)
}

pub fn eval_file_with_tables(source: &str, parser_tables: &mut ParserTables, comptime_vm: &mut ComptimeEngine) -> Result<AstArena, BootstrapDiagnostic> {
    bootstrap_core_file(source, parser_tables, comptime_vm)
}

#[derive(Clone, Debug)]
pub struct BootstrapSnapshot { pub tables: ParserTables, pub vm: ComptimeEngine, pub arenas: Vec<(String, AstArena)> }

static GLOBAL_BOOTSTRAP_SNAPSHOT: std::sync::OnceLock<BootstrapSnapshot> = std::sync::OnceLock::new();

impl BootstrapSnapshot {
    pub fn global() -> &'static Self {
        GLOBAL_BOOTSTRAP_SNAPSHOT.get_or_init(|| {
            let (mut tables, mut vm, mut arenas, mut m_vm) = (ParserTables::new(), ComptimeEngine::new(), Vec::new(), PilVirtualMachine::new());
            let manifest_src = include_str!("../../../stdlib/core/bootstrap.pilot");
            let c_arena = pilot_parser::parse(&format!("{}\nbootstrap_stage_count();\n", manifest_src));
            m_vm.load_binary(BytecodeCompiler::compile(&c_arena, c_arena.root));
            let (stage_count, ws_root) = (m_vm.run().as_int().unwrap_or(0), Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
            for idx in 0..stage_count {
                let p_arena = pilot_parser::parse(&format!("{}\nbootstrap_stage_path({});\n", manifest_src, idx));
                m_vm.load_binary(BytecodeCompiler::compile(&p_arena, p_arena.root));
                let rel_path = m_vm.run().to_string_repr();
                let src = std::fs::read_to_string(ws_root.join(&rel_path)).or_else(|_| std::fs::read_to_string(&rel_path))
                    .unwrap_or_else(|e| panic!("Bootstrap stage {} ('{}') unreadable: {}", idx, rel_path, e));
                let arena = bootstrap_core_file(&src, &mut tables, &mut vm).unwrap_or_else(|e| panic!("Bootstrap stage '{}' failed: {:?}", rel_path, e));
                let pkg = arena.pkg_decls.iter().next().map(|&n| arena.interner.resolve(SymbolId(arena.node(NodeId(n)).lhs)).to_string()).unwrap_or(rel_path);
                arenas.push((pkg, arena));
            }
            BootstrapSnapshot { tables, vm, arenas }
        })
    }
}
