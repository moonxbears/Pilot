use std::collections::{HashMap, HashSet};
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_rt::PilotVal;
use crate::types::{CapabilityBits, FieldOffset, LayoutId, LayoutTable, PhysicalLayout, PhysicsGeometry};

#[derive(Clone, Debug)]
pub struct RawAstNode {
    pub node_id: u32, pub parent_id: u32, pub kind_tag: u8, pub sym_id: u32,
    pub first_child: u32, pub child_count: u16, pub raw_bits: u64,
    pub literal_kind: u8, pub span_start: u32, pub span_end: u32,
}

#[derive(Clone, Debug, Default)]
pub struct RawAstArena {
    pub nodes: Vec<RawAstNode>,
    pub child_links: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct PhysicsDiagnostic {
    pub node_id: u32,
    pub severity: u8,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct PhysicsRegistry {
    pub arena: RawAstArena,
    pub layouts: LayoutTable,
    pub node_layouts: Vec<Option<LayoutId>>,
    pub node_targets: Vec<Option<u32>>,
    pub symbol_strings: HashMap<u32, String>,
    pub diagnostics: Vec<PhysicsDiagnostic>,
}

impl PhysicsRegistry {
    pub fn new(arena: RawAstArena, symbol_strings: HashMap<u32, String>) -> Self {
        let count = arena.nodes.len();
        Self { arena, layouts: LayoutTable::new(), node_layouts: vec![None; count], node_targets: vec![None; count], symbol_strings, diagnostics: Vec::new() }
    }
}

pub fn flatten_ast(arena: &AstArena, _root: NodeId) -> (RawAstArena, HashMap<u32, String>) {
    let mut raw_nodes = Vec::with_capacity(arena.nodes.len());
    let mut child_links = Vec::new();
    let mut symbol_strings = HashMap::new();
    let mut struct_field_nodes = HashSet::new();
    let mut struct_fields_map: HashMap<u32, Vec<u32>> = HashMap::new();

    for (idx, node) in arena.nodes.iter().enumerate() {
        if node.node_kind() == NodeKind::StructDecl && node.rhs != 0 {
            let b_node = *arena.node(NodeId(node.rhs));
            if b_node.node_kind() == NodeKind::BlockExpr {
                let mut f_ids = Vec::new();
                for &c_id in arena.children_slice(b_node.lhs, b_node.rhs) {
                    let c_node = *arena.node(NodeId(c_id));
                    let is_f = c_node.node_kind() == NodeKind::ParamDecl
                        || (matches!(c_node.node_kind(), NodeKind::LetStmt | NodeKind::VarStmt) && arena.type_annotations.contains_key(&c_id));
                    if is_f { struct_field_nodes.insert(c_id); f_ids.push(c_id); }
                }
                struct_fields_map.insert(idx as u32, f_ids);
            }
        }
    }

    let extract_sym = |id: NodeId| -> u32 { if id.is_none() { 0 } else { arena.node(id).lhs } };

    for (idx, node) in arena.nodes.iter().enumerate() {
        let idx_u32 = idx as u32;
        let span = arena.span(NodeId(idx_u32));
        let kind = node.node_kind();

        let (kind_tag, sym_id, raw_bits, lit_kind) = if struct_field_nodes.contains(&idx_u32) {
            let ty_id = if kind == NodeKind::ParamDecl { NodeId(node.rhs) } else { arena.type_annotations.get(&idx_u32).copied().unwrap_or(NodeId::NONE) };
            let ty_sym = extract_sym(ty_id);
            if ty_sym != 0 { symbol_strings.insert(ty_sym, arena.interner.resolve(SymbolId(ty_sym)).to_string()); }
            (21u8, node.lhs, ty_sym as u64, 0u8)
        } else {
            match kind {
                NodeKind::StructDecl => {
                    let mut bits = 0u64;
                    let mut lit = 0u8;
                    if let Some(attrs) = arena.node_attributes.get(&idx_u32) {
                        for attr in attrs {
                            let args: Vec<i64> = attr.args.iter().filter_map(|&a| {
                                let an = arena.node(a);
                                if an.node_kind() == NodeKind::IntLit { arena.i64_pool.get(an.lhs as usize).copied() } else { None }
                            }).collect();
                            if args.len() == 3 {
                                bits = ((args[0].max(0) as u64 & 0xFFFF) << 16) | ((args[1].max(1) as u64 & 0xFF) << 8) | (args[2].max(0) as u64 & 0xFF);
                            } else if args.len() == 2 {
                                bits = ((args[0].max(0) as u64 & 0xFFFF) << 16) | (1u64 << 8) | (args[1].max(0) as u64 & 0xFF);
                            } else if args.len() == 1 && attr.args.len() == 1 && (1..=4).contains(&args[0]) {
                                lit = args[0] as u8;
                            }
                        }
                    }
                    (1u8, node.lhs, bits, lit)
                }
                NodeKind::InterfaceDecl => (2u8, node.lhs, 0u64, 0u8),
                NodeKind::FnDecl => (3u8, node.lhs, 0u64, 0u8),
                NodeKind::ParamDecl => {
                    let ty_sym = extract_sym(NodeId(node.rhs));
                    if ty_sym != 0 { symbol_strings.insert(ty_sym, arena.interner.resolve(SymbolId(ty_sym)).to_string()); }
                    (21u8, node.lhs, ty_sym as u64, 0u8)
                }
                NodeKind::LetStmt | NodeKind::VarStmt => {
                    let ty_sym = arena.type_annotations.get(&idx_u32).copied().map(extract_sym).unwrap_or(0);
                    if ty_sym != 0 { symbol_strings.insert(ty_sym, arena.interner.resolve(SymbolId(ty_sym)).to_string()); }
                    (4u8, node.lhs, ty_sym as u64, 0u8)
                }
                NodeKind::IfExpr => (6u8, 0, 0u64, 0u8),
                NodeKind::ForExpr | NodeKind::WhileStmt => (7u8, 0, 0u64, 0u8),
                NodeKind::ReturnStmt => (8u8, 0, 0u64, 0u8),
                NodeKind::YieldStmt => (9u8, 0, 0u64, 0u8),
                NodeKind::DeferStmt => (10u8, 0, 0u64, 0u8),
                NodeKind::ComptimeExpr => (11u8, 0, 0u64, 0u8),
                NodeKind::PolyglotBlock => (13u8, 0, 0u64, 0u8),
                NodeKind::IdentExpr | NodeKind::TypeNamed => (14u8, node.lhs, 0u64, 0u8),
                NodeKind::IntLit => (15u8, 0, arena.i64_pool.get(node.lhs as usize).copied().unwrap_or(0) as u64, 1u8),
                NodeKind::FloatLit => (15u8, 0, arena.f64_pool.get(node.lhs as usize).copied().unwrap_or(0.0).to_bits(), 2u8),
                NodeKind::StrLit => (15u8, node.lhs, 0u64, 3u8),
                NodeKind::BoolLit => (15u8, 0, node.lhs as u64, 4u8),
                NodeKind::CallExpr => (16u8, 0, 0u64, 0u8),
                NodeKind::MemberExpr => (17u8, node.rhs, 0u64, 0u8),
                NodeKind::BlockExpr => (18u8, 0, 0u64, 0u8),
                NodeKind::BinaryExpr => (19u8, 0, node.flags as u64, 0u8),
                NodeKind::AssignStmt => (20u8, 0, 0u64, 0u8),
                _ => (255u8, 0, 0u64, 0u8),
            }
        };

        if sym_id != 0 { symbol_strings.insert(sym_id, arena.interner.resolve(SymbolId(sym_id)).to_string()); }

        let first_child = child_links.len() as u32;
        let mut child_count = 0u16;

        if !struct_field_nodes.contains(&idx_u32) {
            match kind {
                NodeKind::StructDecl => {
                    if let Some(f_ids) = struct_fields_map.get(&idx_u32) {
                        for &f_id in f_ids { child_links.push(f_id); child_count += 1; }
                    }
                }
                NodeKind::FnDecl => {
                    let (s, c) = (node.rhs as usize, node.flags as usize);
                    if c > 0 && s + c <= arena.extra_children.len() {
                        for &ch in &arena.extra_children[s..s + c] { child_links.push(ch); child_count += 1; }
                    }
                }
                NodeKind::BinaryExpr | NodeKind::AssignStmt | NodeKind::WhileStmt => {
                    for &ch in &[node.lhs, node.rhs] { if ch != 0 { child_links.push(ch); child_count += 1; } }
                }
                NodeKind::IfExpr => {
                    if node.lhs != 0 { child_links.push(node.lhs); child_count += 1; }
                    if node.flags == 2 {
                        for &ch in arena.children_slice(node.rhs, 2) { if ch != 0 { child_links.push(ch); child_count += 1; } }
                    } else if node.rhs != 0 { child_links.push(node.rhs); child_count += 1; }
                }
                NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::MemberExpr => {
                    let ch = if kind == NodeKind::MemberExpr { node.lhs } else { node.rhs };
                    if ch != 0 { child_links.push(ch); child_count += 1; }
                }
                NodeKind::ReturnStmt | NodeKind::YieldStmt => {
                    if node.lhs != 0 { child_links.push(node.lhs); child_count += 1; }
                }
                NodeKind::CallExpr | NodeKind::BlockExpr | NodeKind::Program => {
                    let (s, c) = (node.lhs as usize, node.rhs as usize);
                    if c > 0 && s + c <= arena.extra_children.len() {
                        for &ch in &arena.extra_children[s..s + c] { child_links.push(ch); child_count += 1; }
                    }
                }
                _ => {}
            }
        }

        raw_nodes.push(RawAstNode {
            node_id: idx_u32, parent_id: 0, kind_tag, sym_id, first_child, child_count,
            raw_bits, literal_kind: lit_kind, span_start: span.start, span_end: span.end,
        });
    }

    (RawAstArena { nodes: raw_nodes, child_links }, symbol_strings)
}

pub fn handle_engine_intrinsic(registry: &mut PhysicsRegistry, op_code: &str, args: &[PilotVal]) -> Result<PilotVal, String> {
    match op_code {
        "ast_node_count" => Ok(PilotVal::from_int(registry.arena.nodes.len() as i64)),
        "ast_node_kind" => {
            let nid = args.first().and_then(|v| v.as_int()).unwrap_or(0) as usize;
            Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(255, |n| n.kind_tag) as i64))
        }
        "ast_node_sym" => {
            let nid = args.first().and_then(|v| v.as_int()).unwrap_or(0) as usize;
            Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.sym_id) as i64))
        }
        "ast_node_lit_kind" => {
            let nid = args.first().and_then(|v| v.as_int()).unwrap_or(0) as usize;
            Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.literal_kind) as i64))
        }
        "ast_node_raw_bits" => {
            let nid = args.first().and_then(|v| v.as_int()).unwrap_or(0) as usize;
            Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.raw_bits) as i64))
        }
        "ast_child_count" => {
            let nid = args.first().and_then(|v| v.as_int()).unwrap_or(0) as usize;
            Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.child_count) as i64))
        }
        "ast_child_at" => {
            let nid = args.first().and_then(|v| v.as_int()).unwrap_or(0) as usize;
            let idx = args.get(1).and_then(|v| v.as_int()).unwrap_or(0) as usize;
            let cid = registry.arena.nodes.get(nid).and_then(|n| {
                if idx < n.child_count as usize { registry.arena.child_links.get(n.first_child as usize + idx).copied() } else { None }
            }).unwrap_or(0);
            Ok(PilotVal::from_int(cid as i64))
        }
        "layout_scalar" => {
            let w = args.first().and_then(|v| v.as_int()).unwrap_or(64) as u16;
            let l = args.get(1).and_then(|v| v.as_int()).unwrap_or(1) as u16;
            let b = args.get(2).and_then(|v| v.as_int()).unwrap_or(0) as u8;
            Ok(PilotVal::from_int(registry.layouts.register_scalar(w, l, b).0 as i64))
        }
        "layout_ptr" => {
            let target = args.first().and_then(|v| v.as_int()).unwrap_or(0) as u32;
            let cap = args.get(1).and_then(|v| v.as_int()).unwrap_or(0) as u8;
            let space = args.get(2).and_then(|v| v.as_int()).unwrap_or(0) as u8;
            Ok(PilotVal::from_int(registry.layouts.register_pointer(LayoutId(target), CapabilityBits(cap), space).0 as i64))
        }
        "layout_aggregate" => {
            let mut fields = Vec::new();
            for a in args { if let Some(v) = a.as_int() { fields.push(LayoutId(v as u32)); } }
            Ok(PilotVal::from_int(registry.layouts.register_aggregate(&fields, false, false).0 as i64))
        }
        "layout_union" => {
            let tag = args.first().and_then(|v| v.as_int()).unwrap_or(8) as u8;
            let mut payloads = Vec::new();
            for a in &args[1..] { if let Some(v) = a.as_int() { payloads.push(LayoutId(v as u32)); } }
            Ok(PilotVal::from_int(registry.layouts.register_union(tag, &payloads).0 as i64))
        }
        "layout_fn_abi" => {
            let mut params = Vec::new();
            for a in &args[..args.len().saturating_sub(1)] { if let Some(v) = a.as_int() { params.push(LayoutId(v as u32)); } }
            let ret = args.last().and_then(|v| v.as_int()).map(|x| LayoutId(x as u32)).unwrap_or_default();
            Ok(PilotVal::from_int(registry.layouts.register_call_frame(&params, ret).0 as i64))
        }
        "layout_metrics" => {
            let lid = args.first().and_then(|v| v.as_int()).unwrap_or(0) as u32;
            Ok(PilotVal::from_int(registry.layouts.get(LayoutId(lid)).map_or(0, |l| l.byte_size) as i64))
        }
        "layout_field_offset" => {
            let lid = args.first().and_then(|v| v.as_int()).unwrap_or(0) as u32;
            let idx = args.get(1).and_then(|v| v.as_int()).unwrap_or(0) as usize;
            let off = registry.layouts.get(LayoutId(lid)).and_then(|l| match &l.geometry {
                PhysicsGeometry::Aggregate { fields, .. } => fields.get(idx).map(|f| f.byte_offset),
                _ => None,
            }).unwrap_or(0);
            Ok(PilotVal::from_int(off as i64))
        }
        "layout_physically_equal" => {
            let a = args.first().and_then(|v| v.as_int()).unwrap_or(0) as u32;
            let b = args.get(1).and_then(|v| v.as_int()).unwrap_or(0) as u32;
            let eq = match (registry.layouts.get(LayoutId(a)), registry.layouts.get(LayoutId(b))) {
                (Some(la), Some(lb)) => la.byte_size == lb.byte_size && la.byte_align == lb.byte_align && la.geometry == lb.geometry,
                _ => false,
            };
            Ok(PilotVal::from_bool(eq))
        }
        "ast_bind_layout" => {
            let nid = args.first().and_then(|v| v.as_int()).unwrap_or(0) as usize;
            let lid = args.get(1).and_then(|v| v.as_int()).unwrap_or(0) as u32;
            if nid < registry.node_layouts.len() { registry.node_layouts[nid] = Some(LayoutId(lid)); }
            Ok(PilotVal::NULL)
        }
        "ast_get_bound_layout" => {
            let nid = args.first().and_then(|v| v.as_int()).unwrap_or(0) as usize;
            let lid = registry.node_layouts.get(nid).and_then(|x| *x).map_or(u32::MAX as i64, |l| l.0 as i64);
            Ok(PilotVal::from_int(lid))
        }
        "ast_bind_target" => {
            let nid = args.first().and_then(|v| v.as_int()).unwrap_or(0) as usize;
            let tid = args.get(1).and_then(|v| v.as_int()).unwrap_or(0) as u32;
            if nid < registry.node_targets.len() { registry.node_targets[nid] = Some(tid); }
            Ok(PilotVal::NULL)
        }
        "ast_emit_diag" => {
            let nid = args.first().and_then(|v| v.as_int()).unwrap_or(0) as u32;
            let sev = args.get(1).and_then(|v| v.as_int()).unwrap_or(1) as u8;
            let msg = args.get(2).and_then(|v| v.with_str(|s| s.to_string())).unwrap_or_default();
            registry.diagnostics.push(PhysicsDiagnostic { node_id: nid, severity: sev, message: msg });
            Ok(PilotVal::NULL)
        }
        _ => Ok(PilotVal::NULL),
    }
}

pub fn run_pilot_analyzer(registry: &mut PhysicsRegistry, analyzer_src: Option<&str>) -> Result<(), String> {
    let src = match analyzer_src {
        Some(s) => s.to_string(),
        None => {
            std::fs::read_to_string("selfhost/analyzer.pilot")
                .or_else(|_| std::fs::read_to_string("e:/Pilot/selfhost/analyzer.pilot"))
                .unwrap_or_else(|_| include_str!("../../../selfhost/analyzer.pilot").to_string())
        }
    };

    let parsed_arena = pilot_parser::parse(&src);
    let binary = pilot_vm::compiler::BytecodeCompiler::compile(&parsed_arena, parsed_arena.root);
    let mut vm = pilot_vm::vm::PilVirtualMachine::new();
    vm.load_binary(binary);

    let reg_ptr = registry as *mut PhysicsRegistry;
    vm.engine_handler = Some(Box::new(move |op_code, args| {
        let reg = unsafe { &mut *reg_ptr };
        handle_engine_intrinsic(reg, op_code, args)
    }));

    vm.run();
    Ok(())
}

pub fn analyze_physics(arena: &AstArena, root: NodeId, analyzer_override: Option<&str>) -> PhysicsRegistry {
    let (flat_arena, symbol_strings) = flatten_ast(arena, root);
    let mut registry = PhysicsRegistry::new(flat_arena, symbol_strings);
    let _ = run_pilot_analyzer(&mut registry, analyzer_override);
    registry
}

#[derive(Clone, Debug, Default)]
pub struct NodeLayoutMap {
    pub node_layouts: Vec<Option<LayoutId>>,
}

impl NodeLayoutMap {
    pub fn new() -> Self { Self::default() }
    pub fn get(&self, node_id: u32) -> Option<LayoutId> { self.node_layouts.get(node_id as usize).copied().flatten() }
    pub fn set(&mut self, node_id: u32, layout_id: LayoutId) {
        if node_id as usize >= self.node_layouts.len() { self.node_layouts.resize(node_id as usize + 1, None); }
        self.node_layouts[node_id as usize] = Some(layout_id);
    }
}

pub fn verify_capability_transition(source: CapabilityBits, target: CapabilityBits) -> bool {
    match (source, target) {
        (CapabilityBits::VALUE, _) => true,
        (CapabilityBits::OWNED_OBJ, CapabilityBits::BORROW_REF) => true,
        (CapabilityBits::OWNED_OBJ, CapabilityBits::SCOPED_GUARD) => true,
        (s, t) if s == t => true,
        _ => false,
    }
}

pub fn compute_field_projection(aggregate: &PhysicalLayout, field_idx: usize) -> Option<FieldOffset> {
    match &aggregate.geometry {
        PhysicsGeometry::Aggregate { fields, .. } => fields.get(field_idx).copied(),
        _ => None,
    }
}

pub fn compute_soa_projection(slice_layout: &PhysicalLayout, field_idx: usize) -> Option<PhysicalLayout> {
    match &slice_layout.geometry {
        PhysicsGeometry::Aggregate { fields, .. } => {
            let field = fields.get(field_idx)?;
            Some(PhysicalLayout {
                id: field.layout,
                byte_size: 16,
                byte_align: 8,
                geometry: PhysicsGeometry::Pointer {
                    target: field.layout,
                    capability: CapabilityBits::MUTABLE,
                    addr_space: 0,
                },
            })
        }
        _ => None,
    }
}
