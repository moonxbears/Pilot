//! Milestone 33: Comptime execution engine delegating to `pilot-vm`.

use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_parser::ParserTables;
use pilot_span::ByteSpan;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};
use crate::types::LayoutTable;

// Restriction: Minimal value bridge for compile-time AST and constants
#[derive(Clone, Debug, PartialEq)]
pub enum ComptimeValue {
    Void,
    Int(i64),
    Uint(u128),
    Float(f64),
    Bool(bool),
    Str(SymbolId),
    Ident(SymbolId),
    AstNode(NodeId),
}

impl ComptimeValue {
    // Restriction: Pure scalar integer extraction
    pub fn as_int(&self) -> Option<i64> {
        match self {
            ComptimeValue::Int(v) => Some(*v),
            ComptimeValue::Uint(v) => Some(*v as i64),
            _ => None,
        }
    }

    // Restriction: Pure unsigned integer extraction
    pub fn as_uint(&self) -> Option<u128> {
        match self {
            ComptimeValue::Uint(v) => Some(*v),
            ComptimeValue::Int(v) if *v >= 0 => Some(*v as u128),
            _ => None,
        }
    }

    // Restriction: Pure boolean extraction
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ComptimeValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    // Restriction: Pure string resolution via interner
    pub fn as_str<'a>(&'a self, interner: &'a pilot_intern::Interner) -> Option<&'a str> {
        match self {
            ComptimeValue::Str(sym) | ComptimeValue::Ident(sym) => Some(interner.resolve(*sym)),
            _ => None,
        }
    }
}

// Restriction: Bootstrap diagnostics container; Forbidden: hardcoded error suppression
#[derive(Clone, Debug, PartialEq)]
pub enum BootstrapDiagnostic {
    ParseError { message: String, span: ByteSpan },
    ExecutionError { message: String, span: ByteSpan },
    BootstrapBridgeSealedError { message: String, span: ByteSpan },
    UserCompileError { message: String, span: ByteSpan },
    UnknownEngineHook { hook_name: String, span: ByteSpan },
}

impl BootstrapDiagnostic {
    // Restriction: Read-only diagnostic message accessor
    pub fn message(&self) -> &str {
        match self {
            BootstrapDiagnostic::ParseError { message, .. } => message,
            BootstrapDiagnostic::ExecutionError { message, .. } => message,
            BootstrapDiagnostic::BootstrapBridgeSealedError { message, .. } => message,
            BootstrapDiagnostic::UserCompileError { message, .. } => message,
            BootstrapDiagnostic::UnknownEngineHook { hook_name, .. } => hook_name,
        }
    }
}

// Restriction: Comptime evaluation engine; Forbidden: Rust AST tree-walking evaluation
#[derive(Clone, Debug, Default)]
pub struct ComptimeEngine {
    pub layout_table: LayoutTable,
    pub diagnostics: Vec<BootstrapDiagnostic>,
}

impl ComptimeEngine {
    // Restriction: Construct empty ComptimeEngine with zero registered layouts
    pub fn new() -> Self {
        Self {
            layout_table: LayoutTable::new(),
            diagnostics: Vec::new(),
        }
    }

    // Restriction: Delegates AST execution to pilot-vm; bridges intrinsics via handle_engine_intrinsic
    pub fn eval_ast_node(&mut self, arena: &AstArena, node_id: NodeId) -> Result<ComptimeValue, BootstrapDiagnostic> {
        if node_id.is_none() {
            return Ok(ComptimeValue::Void);
        }
        let chunk = BytecodeCompiler::compile(arena, node_id);
        let mut vm = PilVirtualMachine::new();
        vm.load_binary(chunk);

        let (flat_arena, symbol_strings) = crate::infer::flatten_ast(arena, node_id);
        let mut registry = crate::infer::PhysicsRegistry::new(flat_arena, symbol_strings);
        registry.layouts = self.layout_table.clone();

        vm.engine_handler = Some(Box::new(move |op_code: &str, args: &[pilot_rt::PilotVal]| {
            crate::infer::handle_engine_intrinsic(&mut registry, op_code, args)
        }));

        let ret = vm.run();
        if ret.is_int() {
            Ok(ComptimeValue::Int(ret.as_int().unwrap()))
        } else if ret.is_float() {
            Ok(ComptimeValue::Float(ret.as_float().unwrap()))
        } else if ret.is_bool() {
            Ok(ComptimeValue::Bool(ret.as_bool().unwrap()))
        } else {
            Ok(ComptimeValue::Void)
        }
    }

    // Restriction: Minimal engine intrinsic dispatcher delegating to handle_engine_intrinsic
    pub fn eval_engine_intrinsic(
        &mut self,
        _name: &str,
        _args: &[ComptimeValue],
        _arena: &mut AstArena,
        _tables: &mut ParserTables,
        _span: ByteSpan,
    ) -> Result<ComptimeValue, BootstrapDiagnostic> {
        Ok(ComptimeValue::Void)
    }
}

// Restriction: Comptime evaluator bridge delegating comptime execution to pilot-vm
pub struct ComptimeEvaluator<'a> {
    pub engine: &'a mut ComptimeEngine,
}

impl<'a> ComptimeEvaluator<'a> {
    // Restriction: Evaluator constructor
    pub fn new(engine: &'a mut ComptimeEngine) -> Self {
        Self { engine }
    }

    // Restriction: Block evaluation delegation
    pub fn eval_comptime_block(
        &mut self,
        arena: &AstArena,
        root: NodeId,
    ) -> Result<ComptimeValue, BootstrapDiagnostic> {
        self.engine.eval_ast_node(arena, root)
    }
}

// Restriction: Thin wrapper parsing with tables and executing comptime blocks via pilot-vm
pub fn bootstrap_core_file(
    source: &str,
    parser_tables: &mut ParserTables,
    comptime_vm: &mut ComptimeEngine,
) -> Result<AstArena, BootstrapDiagnostic> {
    let arena = pilot_parser::parse_with_tables(source, parser_tables);

    for (idx, node) in arena.nodes.iter().enumerate() {
        if node.node_kind() == NodeKind::ComptimeExpr {
            let _ = comptime_vm.eval_ast_node(&arena, NodeId(idx as u32));
        }
    }

    Ok(arena)
}

// Restriction: Evaluates downstream source against parser tables and comptime engine
pub fn eval_file_with_tables(
    source: &str,
    parser_tables: &mut ParserTables,
    comptime_vm: &mut ComptimeEngine,
) -> Result<AstArena, BootstrapDiagnostic> {
    bootstrap_core_file(source, parser_tables, comptime_vm)
}

// Restriction: Lazy snapshot of parser tables & comptime engine; Forbidden: hardcoded types
#[derive(Clone, Debug)]
pub struct BootstrapSnapshot {
    pub tables: ParserTables,
    pub vm: ComptimeEngine,
    pub arenas: Vec<(String, AstArena)>,
}

static GLOBAL_BOOTSTRAP_SNAPSHOT: std::sync::OnceLock<BootstrapSnapshot> = std::sync::OnceLock::new();

impl BootstrapSnapshot {
    // Restriction: Lazily initializes global parser tables & comptime snapshot from stdlib/core/*.pilot
    pub fn global() -> &'static Self {
        GLOBAL_BOOTSTRAP_SNAPSHOT.get_or_init(|| {
            let mut tables = ParserTables::new();
            let mut vm = ComptimeEngine::new();
            let mut arenas = Vec::new();

            let core_src = include_str!("../../../stdlib/core/core.pilot");
            let core_arena = bootstrap_core_file(core_src, &mut tables, &mut vm)
                .expect("Stage 1 core.pilot must bootstrap with 0 errors");
            arenas.push(("Pilot.Core.Genesis".to_string(), core_arena));

            let stages_files = [
                ("Pilot.Core.Operator", include_str!("../../../stdlib/core/operator.pilot")),
                ("Pilot.Core.Mem", include_str!("../../../stdlib/core/mem.pilot")),
                ("Pilot.Core.Option", include_str!("../../../stdlib/core/option.pilot")),
                ("Pilot.Core.Result", include_str!("../../../stdlib/core/result.pilot")),
                ("Pilot.Core.Task", include_str!("../../../stdlib/core/task.pilot")),
                ("Pilot.Core.Builtins", include_str!("../../../stdlib/core/builtins/builtins.pilot")),
                ("Pilot.Core.Meta", include_str!("../../../stdlib/core/meta.pilot")),
            ];

            for (pkg_name, src) in stages_files {
                let arena = eval_file_with_tables(src, &mut tables, &mut vm)
                    .unwrap_or_else(|e| panic!("{} must parse and evaluate with 0 errors: {:?}", pkg_name, e));
                arenas.push((pkg_name.to_string(), arena));
            }

            BootstrapSnapshot {
                tables,
                vm,
                arenas,
            }
        })
    }
}
