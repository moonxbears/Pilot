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
    Void,
    Int(i64),
    Uint(u128),
    Float(f64),
    Bool(bool),
    Str(SymbolId),
    StringVal(String),
    Ident(SymbolId),
    AstNode(NodeId),
    Map(i64),
}

impl ComptimeValue {
    pub fn as_int(&self) -> Option<i64> { match self { Self::Int(v) => Some(*v), Self::Uint(v) => Some(*v as i64), _ => None } }
    pub fn as_uint(&self) -> Option<u128> { match self { Self::Uint(v) => Some(*v), Self::Int(v) if *v >= 0 => Some(*v as u128), _ => None } }
    pub fn as_bool(&self) -> Option<bool> { match self { Self::Bool(b) => Some(*b), _ => None } }
    pub fn as_str<'a>(&'a self, interner: &'a pilot_intern::Interner) -> Option<&'a str> {
        match self {
            Self::Str(sym) | Self::Ident(sym) => Some(interner.resolve(*sym)),
            Self::StringVal(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BootstrapDiagnostic {
    ParseError { message: String, span: ByteSpan }, ExecutionError { message: String, span: ByteSpan },
    BootstrapBridgeSealedError { message: String, span: ByteSpan }, UserCompileError { message: String, span: ByteSpan }, UnknownEngineHook { hook_name: String, span: ByteSpan },
}

impl BootstrapDiagnostic {
    pub fn message(&self) -> &str { match self { Self::ParseError { message, .. } | Self::ExecutionError { message, .. } | Self::BootstrapBridgeSealedError { message, .. } | Self::UserCompileError { message, .. } => message, Self::UnknownEngineHook { hook_name, .. } => hook_name } }
}

#[derive(Clone, Debug, Default)]
pub struct ComptimeEngine { pub layout_table: LayoutTable, pub node_layouts: Vec<Option<LayoutId>>, pub diagnostics: Vec<BootstrapDiagnostic> }

impl ComptimeEngine {
    pub fn new() -> Self { Self::default() }
    pub fn eval_ast_node(&mut self, arena: &AstArena, node_id: NodeId) -> Result<ComptimeValue, BootstrapDiagnostic> {
        if node_id.is_none() { return Ok(ComptimeValue::Void); }
        let mut vm = PilVirtualMachine::new();
        vm.load_binary(BytecodeCompiler::compile(arena, node_id));
        let (flat_arena, symbol_strings) = crate::infer::flatten_ast(arena, node_id);
        let mut registry = crate::infer::PhysicsRegistry::new(flat_arena, symbol_strings);
        registry.layouts = self.layout_table.clone();
        let (mut env_m, mut tgt_m, mut ast_m, mut sym_m) = (std::collections::HashMap::new(), std::collections::HashMap::new(), std::collections::HashMap::new(), std::collections::HashMap::new());
        for (k, v) in [("architecture", PilotVal::from_str(std::env::consts::ARCH)), ("arch", PilotVal::from_str(std::env::consts::ARCH)), ("os", PilotVal::from_str(std::env::consts::OS)), ("pointer_byte", PilotVal::from_int(std::mem::size_of::<usize>() as i64)), ("pointer_bytes", PilotVal::from_int(std::mem::size_of::<usize>() as i64)), ("endianess", PilotVal::from_str(if cfg!(target_endian = "little") { "little" } else { "big" })), ("endian", PilotVal::from_str(if cfg!(target_endian = "little") { "little" } else { "big" })), ("backend", PilotVal::from_str("vm")), ("simd_width_bits", PilotVal::from_int(128)), ("cache_line", PilotVal::from_int(64)), ("cache_line_bytes", PilotVal::from_int(64)), ("opt_level", PilotVal::from_int(2))] { tgt_m.insert(k.into(), v); }
        ast_m.insert("node_count".into(), PilotVal::from_int(registry.arena.nodes.len() as i64)); ast_m.insert("root_node".into(), PilotVal::from_int(0)); sym_m.insert("count".into(), PilotVal::from_int(registry.symbol_strings.len() as i64));
        for (k, v) in [("is_comptime", 1), ("unit_count", 1), ("symbol_count", registry.symbol_strings.len() as i64), ("target", 1), ("ast_nodes", 2), ("symbols", 3)] { env_m.insert(k.into(), PilotVal::from_int(v)); }
        vm.heap_maps.extend([env_m, tgt_m, ast_m, sym_m]);
        vm.shared_symbols = Some(std::sync::Arc::new(registry.symbol_strings.clone()));
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
        else if ret.is_sso_str() || ret.tag() == 9 { Ok(ComptimeValue::StringVal(ret.to_string_repr())) }
        else { Ok(ComptimeValue::Void) }
    }
}

pub struct ComptimeEvaluator<'a> { pub engine: &'a mut ComptimeEngine }
impl<'a> ComptimeEvaluator<'a> {
    pub fn new(engine: &'a mut ComptimeEngine) -> Self { Self { engine } }
    pub fn eval_comptime_block(&mut self, arena: &AstArena, root: NodeId) -> Result<ComptimeValue, BootstrapDiagnostic> { self.engine.eval_ast_node(arena, root) }
}

pub fn bootstrap_core_file(source: &str, parser_tables: &mut ParserTables, comptime_vm: &mut ComptimeEngine) -> Result<AstArena, BootstrapDiagnostic> {
    let arena = pilot_parser::parse_and_ingest(source, parser_tables);
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
            let harness = format!(
                "{}\n\
                 let sc = bootstrap_stage_count();\n\
                 var out = \"\" + sc;\n\
                 var si = 0;\n\
                 while si < sc {{\n\
                     out = out + \"|\" + bootstrap_stage_path(si);\n\
                     si = si + 1;\n\
                 }}\n\
                 out;\n",
                manifest_src
            );
            let c_arena = pilot_parser::parse(&harness);
            m_vm.load_binary(BytecodeCompiler::compile(&c_arena, c_arena.root));
            let result_str = m_vm.run().to_string_repr();
            let mut parts = result_str.split('|');
            let _stage_count: usize = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let stage_paths: Vec<&str> = parts.collect();
            let ws_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            for (idx, rel_path) in stage_paths.iter().enumerate() {
                let src = std::fs::read_to_string(ws_root.join(rel_path)).or_else(|_| std::fs::read_to_string(rel_path))
                    .unwrap_or_else(|e| panic!("Bootstrap stage {} ('{}') unreadable: {}", idx, rel_path, e));
                let arena = bootstrap_core_file(&src, &mut tables, &mut vm).unwrap_or_else(|e| panic!("Bootstrap stage '{}' failed: {:?}", rel_path, e));
                let pkg = arena.decl_flags.iter().enumerate().find(|(_, &f)| (f & pilot_ast::DECL_FLAG_PKG) != 0)
                    .map(|(n, _)| arena.interner.resolve(SymbolId(arena.node(NodeId(n as u32)).lhs)).to_string()).unwrap_or_else(|| rel_path.to_string());
                arenas.push((pkg, arena));
            }
            BootstrapSnapshot { tables, vm, arenas }
        })
    }
}
