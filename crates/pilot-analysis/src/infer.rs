use std::collections::{HashMap, HashSet};
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_rt::PilotVal;
use pilot_vm::pilc::PilcBinary;
use crate::types::{CapabilityBits, HwScalarHeader, LayoutId, LayoutTable, PhysicsGeometry};

#[derive(Clone, Copy, Debug, Default)]
pub struct RawAttr { pub name_sym: u32, pub first_arg: u32, pub arg_count: u16 }

#[derive(Clone, Debug)]
pub struct RawAstNode {
    pub node_id: u32, pub parent_id: u32, pub kind_tag: u8, pub orig_kind: u16,
    pub node_flags: u16, pub decl_flags: u16, pub sym_id: u32,
    pub first_child: u32, pub child_count: u16, pub first_attr_arg: u32, pub attr_arg_count: u16,
    pub first_attr: u32, pub attr_count: u16, pub first_base: u32, pub base_count: u16,
    pub raw_bits: u64, pub literal_kind: u8, pub span_start: u32, pub span_end: u32,
    /// The compilation unit (file) the node was parsed from; 0 for a single file.
    pub unit: u32,
}

#[derive(Clone, Debug, Default)]
pub struct RawAstArena {
    pub nodes: Vec<RawAstNode>, pub child_links: Vec<u32>, pub attr_args: Vec<i64>,
    pub attr_arg_kind: Vec<u8>, pub attrs: Vec<RawAttr>, pub base_syms: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct CategoryRecord { pub id: i64, pub scope: String, pub badge: String, pub lsp_kind: i64 }

#[derive(Clone, Debug)]
pub struct PhysicsDiagnostic { pub node_id: u32, pub severity: u8, pub message: String }

#[derive(Clone, Debug)]
pub struct PhysicsRegistry {
    pub arena: RawAstArena, pub layouts: LayoutTable, pub node_layouts: Vec<Option<LayoutId>>,
    pub node_targets: Vec<Option<u32>>, pub symbol_strings: HashMap<u32, String>,
    pub diagnostics: Vec<PhysicsDiagnostic>, pub source_text: String,
    pub lsp_method: String, pub lsp_offset: u32, pub lsp_result: String,
    /// Set when the self-hosted analyzer faulted; its results are then incomplete.
    pub analyzer_error: Option<String>,
    pub categories: Vec<CategoryRecord>, pub registered_symbols: Vec<(i64, String, String)>,
}

impl PhysicsRegistry {
    pub fn new(arena: RawAstArena, symbol_strings: HashMap<u32, String>) -> Self {
        let count = arena.nodes.len();
        let mut reg = Self {
            arena, layouts: LayoutTable::new(), node_layouts: vec![None; count], node_targets: vec![None; count],
            symbol_strings, diagnostics: Vec::new(), source_text: String::new(),
            lsp_method: String::new(), lsp_offset: 0, lsp_result: String::new(), analyzer_error: None,
            categories: Vec::new(), registered_symbols: Vec::new(),
        };
        reg.index_arena_symbols(0);
        reg
    }

    fn index_arena_symbols(&mut self, from_node: usize) {
        for n in &self.arena.nodes[from_node..] {
            let b = n.first_attr_arg as usize;
            if n.kind_tag == 1 && n.attr_arg_count == 4 && self.arena.attr_arg_kind[b + 1] == 1 && self.arena.attr_arg_kind[b + 2] == 1 {
                let (cid, lkind) = (self.arena.attr_args[b], self.arena.attr_args[b + 3]);
                let scope = self.symbol_strings.get(&(self.arena.attr_args[b + 1] as u32)).cloned().unwrap_or_default();
                let badge = self.symbol_strings.get(&(self.arena.attr_args[b + 2] as u32)).cloned().unwrap_or_default();
                if !self.categories.iter().any(|c| c.id == cid) { self.categories.push(CategoryRecord { id: cid, scope, badge, lsp_kind: lkind }); }
            } else if n.kind_tag == 1 && n.child_count == 0 && (n.attr_arg_count == 2 || n.attr_arg_count == 3)
                && self.arena.attr_arg_kind[b] == 0 && self.arena.attr_arg_kind[b + 1] == 1
            {
                let cid = self.arena.attr_args[b];
                let word = self.symbol_strings.get(&n.sym_id).cloned().unwrap_or_default();
                let doc = self.symbol_strings.get(&(self.arena.attr_args[b + 1] as u32)).cloned().unwrap_or_default();
                if !word.is_empty() { self.registered_symbols.push((cid, word, doc)); }
            }
        }
    }
}

pub fn flatten_ast(arena: &AstArena, _root: NodeId) -> (RawAstArena, HashMap<u32, String>) {
    let mut raw_nodes = Vec::with_capacity(arena.nodes.len());
    let (mut child_links, mut attr_args, mut attr_arg_kind) = (Vec::new(), Vec::new(), Vec::new());
    let (mut raw_attrs, mut base_syms, mut symbol_strings) = (Vec::new(), Vec::new(), HashMap::new());
    let (mut struct_field_nodes, mut struct_dtor_nodes, mut struct_fields_map) = (HashSet::new(), HashSet::new(), HashMap::new());

    for (idx, node) in arena.nodes.iter().enumerate() {
        if node.node_kind() == NodeKind::StructDecl && node.rhs != 0 && node.rhs != u32::MAX {
            let b_node = *arena.node(NodeId(node.rhs));
            if b_node.node_kind() == NodeKind::BlockExpr {
                let mut f_ids = Vec::new();
                for &c_id in arena.children_slice(b_node.lhs, b_node.rhs) {
                    let ck = arena.node(NodeId(c_id)).node_kind();
                    if ck == NodeKind::ParamDecl || (matches!(ck, NodeKind::LetStmt | NodeKind::VarStmt) && arena.type_annot(NodeId(c_id)).is_some()) {
                        struct_field_nodes.insert(c_id); f_ids.push(c_id);
                    } else {
                        if ck == NodeKind::DestructorDecl { struct_dtor_nodes.insert(idx as u32); }
                        f_ids.push(c_id);
                    }
                }
                struct_fields_map.insert(idx as u32, f_ids);
            }
        }
    }

    let extract_sym = |id: NodeId| -> u32 {
        if id.is_none() || (id.0 as usize) >= arena.nodes.len() { 0 } else { arena.node(id).lhs }
    };
    let (mut attrs_map, mut bases_map): (HashMap<u32, Vec<usize>>, HashMap<u32, Vec<NodeId>>) = (HashMap::new(), HashMap::new());
    for (ai, attr) in arena.attrs.iter().enumerate() { attrs_map.entry(attr.target.0).or_default().push(ai); }
    for &(owner, base_ty) in &arena.base_types { bases_map.entry(owner.0).or_default().push(base_ty); }

    for (idx, node) in arena.nodes.iter().enumerate() {
        let (idx_u32, span, kind) = (idx as u32, arena.span(NodeId(idx as u32)), node.node_kind());
        let (first_attr_arg, first_attr) = (attr_args.len() as u32, raw_attrs.len() as u32);
        let (mut attr_arg_count, mut attr_count) = (0u16, 0u16);
        if let Some(a_indices) = attrs_map.get(&idx_u32) {
            for &ai in a_indices {
                let attr = arena.attrs[ai];
                if attr.name.0 != 0 { symbol_strings.insert(attr.name.0, arena.interner.resolve(attr.name).to_string()); }
                let (this_first, mut this_cnt) = (attr_args.len() as u32, 0u16);
                for &a_raw in arena.children_slice(attr.args_start, attr.args_len as u32) {
                    let a = NodeId(a_raw);
                    let an = arena.node(a);
                    let (val, ak) = match an.node_kind() {
                        NodeKind::IntLit => (arena.i64_pool.get(an.lhs as usize).copied().unwrap_or(0), 0u8),
                        NodeKind::FloatLit => (arena.f64_pool.get(an.lhs as usize).copied().unwrap_or(0.0).to_bits() as i64, 0),
                        NodeKind::BoolLit => ((an.lhs != 0) as i64, 0),
                        NodeKind::StrLit | NodeKind::IdentExpr | NodeKind::TypeNamed => {
                            if an.lhs != 0 { symbol_strings.insert(an.lhs, arena.interner.resolve(SymbolId(an.lhs)).to_string()); }
                            (an.lhs as i64, if an.node_kind() == NodeKind::StrLit { 1 } else { 2 })
                        }
                        NodeKind::UnaryExpr if an.flags != 1 && (an.lhs as usize) < arena.nodes.len() && arena.node(NodeId(an.lhs)).node_kind() == NodeKind::IntLit => {
                            (-arena.i64_pool.get(arena.node(NodeId(an.lhs)).lhs as usize).copied().unwrap_or(0), 0)
                        }
                        _ => (a.0 as i64, 0),
                    };
                    attr_args.push(val); attr_arg_kind.push(ak); this_cnt += 1; attr_arg_count += 1;
                }
                raw_attrs.push(RawAttr { name_sym: attr.name.0, first_arg: this_first, arg_count: this_cnt });
                attr_count += 1;
            }
        }

        let (first_base, mut base_count) = (base_syms.len() as u32, 0u16);
        if let Some(bases) = bases_map.get(&idx_u32) {
            for &b in bases {
                let bsym = extract_sym(b);
                if bsym != 0 { symbol_strings.insert(bsym, arena.interner.resolve(SymbolId(bsym)).to_string()); base_syms.push(bsym); base_count += 1; }
            }
        }

        let mut decl_flags = arena.decl_flags.get(idx).copied().unwrap_or(15u16);
        if kind == NodeKind::VarStmt { decl_flags |= 128; }
        if struct_dtor_nodes.contains(&idx_u32) { decl_flags |= 1024; }

        let (kind_tag, sym_id, raw_bits, lit_kind) = if struct_field_nodes.contains(&idx_u32) {
            let ty_id = if kind == NodeKind::ParamDecl { NodeId(node.rhs) } else { arena.type_annot(NodeId(idx_u32)).unwrap_or(NodeId::NONE) };
            let ty_sym = extract_sym(ty_id);
            if ty_sym != 0 { symbol_strings.insert(ty_sym, arena.interner.resolve(SymbolId(ty_sym)).to_string()); }
            (21u8, node.lhs, ty_sym as u64, 0u8)
        } else {
            match kind {
                NodeKind::StructDecl => (1, node.lhs, 0, 0),
                NodeKind::InterfaceDecl => (2, node.lhs, 0, 0),
                NodeKind::FnDecl => { let r = arena.type_annot(NodeId(idx_u32)).map(extract_sym).unwrap_or(0); if r != 0 { symbol_strings.insert(r, arena.interner.resolve(SymbolId(r)).to_string()); } (3, node.lhs, r as u64, 0) },
                NodeKind::ParamDecl => {
                    let ty_sym = extract_sym(NodeId(node.rhs));
                    if ty_sym != 0 { symbol_strings.insert(ty_sym, arena.interner.resolve(SymbolId(ty_sym)).to_string()); }
                    (21, node.lhs, ty_sym as u64, 0)
                }
                NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
                    let ty_sym = arena.type_annot(NodeId(idx_u32)).map(extract_sym).unwrap_or(0);
                    if ty_sym != 0 { symbol_strings.insert(ty_sym, arena.interner.resolve(SymbolId(ty_sym)).to_string()); }
                    (4, node.lhs, ty_sym as u64, 0)
                }
                NodeKind::IfExpr => (6, 0, 0, 0),
                NodeKind::ForExpr | NodeKind::WhileStmt => (7, 0, 0, 0),
                NodeKind::ReturnStmt => (8, 0, 0, 0),
                NodeKind::YieldStmt => (9, 0, 0, 0),
                NodeKind::DeferStmt => (10, 0, 0, 0),
                NodeKind::ComptimeExpr => (11, 0, 0, 0),
                NodeKind::PolyglotBlock => (13, node.lhs, 0, 0),
                NodeKind::IdentExpr | NodeKind::TypeNamed => (14, node.lhs, 0, 0),
                NodeKind::IntLit => (15, 0, arena.i64_pool.get(node.lhs as usize).copied().unwrap_or(0) as u64, 1),
                NodeKind::FloatLit => (15, 0, arena.f64_pool.get(node.lhs as usize).copied().unwrap_or(0.0).to_bits(), 2),
                NodeKind::StrLit => (15, node.lhs, 0, 3),
                NodeKind::BoolLit => (15, 0, node.lhs as u64, 4),
                NodeKind::CallExpr => (16, 0, node.flags as u64, 0),
                NodeKind::MemberExpr => (17, node.rhs, 0, 0),
                NodeKind::BlockExpr => (18, 0, 0, 0),
                NodeKind::BinaryExpr => (19, 0, node.flags as u64, 0),
                NodeKind::AssignStmt => (20, 0, 0, 0),
                NodeKind::ImportDecl | NodeKind::TypeAdtDecl | NodeKind::AdtVariant
                | NodeKind::ExtendDecl | NodeKind::PatBind | NodeKind::PatVariant | NodeKind::UnitLit => (255, node.lhs, node.flags as u64, 0),
                _ => (255, 0, node.flags as u64, 0),
            }
        };
        if sym_id != 0 { symbol_strings.insert(sym_id, arena.interner.resolve(SymbolId(sym_id)).to_string()); }

        let (first_child, mut child_count) = (child_links.len() as u32, 0u16);
        let mut push_ch = |c: u32| { if c != u32::MAX && (c as usize) < arena.nodes.len() { child_links.push(c); child_count += 1; } };
        if !struct_field_nodes.contains(&idx_u32) {
            match kind {
                NodeKind::StructDecl => if let Some(f_ids) = struct_fields_map.get(&idx_u32) { for &f in f_ids { child_links.push(f); child_count += 1; } },
                NodeKind::InterfaceDecl | NodeKind::ExtendDecl => {
                    if node.rhs != 0 && node.rhs != u32::MAX && (node.rhs as usize) < arena.nodes.len() {
                        let bn = *arena.node(NodeId(node.rhs));
                        if bn.node_kind() == NodeKind::BlockExpr { for &c in arena.children_slice(bn.lhs, bn.rhs) { push_ch(c); } } else { push_ch(node.rhs); }
                    }
                }
                NodeKind::FnDecl | NodeKind::OperatorDecl | NodeKind::AdtVariant | NodeKind::PatVariant | NodeKind::TypeNamed => {
                    for &c in arena.children_slice(node.rhs, node.flags as u32) { push_ch(c); }
                }
                NodeKind::TypeAdtDecl => if node.flags > 0 { for &c in arena.children_slice(node.rhs, node.flags as u32) { push_ch(c); } } else { push_ch(node.rhs); },
                NodeKind::LambdaExpr => { for &c in arena.children_slice(node.lhs, node.flags as u32) { push_ch(c); } push_ch(node.rhs); }
                NodeKind::ForExpr => { let it = arena.for_iter(NodeId(idx_u32)); if it.is_some() { push_ch(it.0); } push_ch(node.rhs); let th = arena.for_then(NodeId(idx_u32)); if th.is_some() { push_ch(th.0); } }
                NodeKind::IfExpr => {
                    push_ch(node.lhs);
                    if node.flags >= 2 { for &c in arena.children_slice(node.rhs, node.flags as u32) { push_ch(c); } } else { push_ch(node.rhs); }
                }
                NodeKind::Program | NodeKind::BlockExpr | NodeKind::CallExpr | NodeKind::TupleExpr | NodeKind::ListExpr
                | NodeKind::MapExpr | NodeKind::MatchExpr | NodeKind::TryStmt | NodeKind::ConstructorDecl
                | NodeKind::IndexerDecl | NodeKind::TypeTuple | NodeKind::PatTuple | NodeKind::PatSlice => {
                    for &c in arena.children_slice(node.lhs, node.rhs) { push_ch(c); }
                }
                NodeKind::BinaryExpr | NodeKind::AssignStmt | NodeKind::WhileStmt | NodeKind::IndexExpr | NodeKind::CascadeExpr
                | NodeKind::PipeExpr | NodeKind::CatchRecoveryExpr | NodeKind::CoalesceExpr | NodeKind::RangeExpr
                | NodeKind::GuardStmt | NodeKind::MatchArm | NodeKind::ScopeDecl | NodeKind::PatOr | NodeKind::PatRange => {
                    push_ch(node.lhs); push_ch(node.rhs);
                }
                // `name: value`: the name is a symbol, the value a node.
                NodeKind::CallArg => push_ch(node.rhs),
                NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl | NodeKind::CatchClause => push_ch(node.rhs),
                NodeKind::MemberExpr | NodeKind::UnaryExpr | NodeKind::ReturnStmt | NodeKind::YieldStmt | NodeKind::DeferStmt | NodeKind::BreakStmt | NodeKind::ContinueStmt
                | NodeKind::ExprStmt | NodeKind::TryPropagateExpr | NodeKind::DestructorDecl | NodeKind::DynamicMemberDecl
                | NodeKind::ComptimeExpr | NodeKind::PatLiteral => push_ch(node.lhs),
                _ => {}
            }
        }
        raw_nodes.push(RawAstNode {
            node_id: idx_u32, parent_id: u32::MAX, kind_tag, orig_kind: node.kind, node_flags: node.flags, decl_flags,
            sym_id, first_child, child_count, first_attr_arg, attr_arg_count, first_attr, attr_count,
            first_base, base_count, raw_bits, literal_kind: lit_kind, span_start: span.start, span_end: span.end,
            unit: arena.unit_of(NodeId(idx_u32)).map_or(0, |u| u.file.0),
        });
    }

    for p_idx in 0..raw_nodes.len() {
        let (fc, cc) = (raw_nodes[p_idx].first_child as usize, raw_nodes[p_idx].child_count as usize);
        for &cid in &child_links[fc..fc + cc] { if (cid as usize) < raw_nodes.len() { raw_nodes[cid as usize].parent_id = p_idx as u32; } }
    }
    (RawAstArena { nodes: raw_nodes, child_links, attr_args, attr_arg_kind, attrs: raw_attrs, base_syms }, symbol_strings)
}

pub fn handle_engine_intrinsic(registry: &mut PhysicsRegistry, op_code: &str, args: &[PilotVal]) -> Result<PilotVal, String> {
    let arg_i = |i: usize| args.get(i).and_then(|v| v.as_int()).unwrap_or(0);
    let arg_s = |i: usize| args.get(i).and_then(|v| v.with_str(|s| s.to_string())).unwrap_or_default();
    let nid = arg_i(0) as usize;
    match op_code {
        "ast_node_count" => Ok(PilotVal::from_int(registry.arena.nodes.len() as i64)),
        "ast_node_unit" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.unit) as i64)),
        "ast_node_kind" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(255, |n| n.kind_tag) as i64)),
        "ast_orig_kind" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(65535, |n| n.orig_kind) as i64)),
        "ast_node_flags" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.node_flags) as i64)),
        "ast_decl_flags" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.decl_flags) as i64)),
        "ast_parent" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(u32::MAX as i64, |n| n.parent_id as i64))),
        "ast_node_sym" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.sym_id) as i64)),
        "ast_node_lit_kind" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.literal_kind) as i64)),
        "ast_node_raw_bits" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.raw_bits) as i64)),
        "ast_child_count" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.child_count) as i64)),
        "ast_child_at" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).and_then(|n| ((arg_i(1) as usize) < n.child_count as usize).then(|| registry.arena.child_links[n.first_child as usize + arg_i(1) as usize])).unwrap_or(0) as i64)),
        "ast_attr_count" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.attr_arg_count) as i64)),
        "ast_attr_arg" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).and_then(|n| ((arg_i(1) as usize) < n.attr_arg_count as usize).then(|| registry.arena.attr_args[n.first_attr_arg as usize + arg_i(1) as usize])).unwrap_or(0))),
        "ast_attr_decl_count" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.attr_count) as i64)),
        "ast_attr_name" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).and_then(|n| ((arg_i(1) as usize) < n.attr_count as usize).then(|| registry.arena.attrs[n.first_attr as usize + arg_i(1) as usize].name_sym)).unwrap_or(0) as i64)),
        "ast_attr_decl_arg_count" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).and_then(|n| ((arg_i(1) as usize) < n.attr_count as usize).then(|| registry.arena.attrs[n.first_attr as usize + arg_i(1) as usize].arg_count)).unwrap_or(0) as i64)),
        "ast_attr_decl_arg" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).and_then(|n| ((arg_i(1) as usize) < n.attr_count as usize).then(|| registry.arena.attrs[n.first_attr as usize + arg_i(1) as usize])).and_then(|a| ((arg_i(2) as usize) < a.arg_count as usize).then(|| registry.arena.attr_args[a.first_arg as usize + arg_i(2) as usize])).unwrap_or(0))),
        "ast_base_count" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.base_count) as i64)),
        "ast_base_at" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).and_then(|n| ((arg_i(1) as usize) < n.base_count as usize).then(|| registry.arena.base_syms[n.first_base as usize + arg_i(1) as usize])).unwrap_or(0) as i64)),
        "layout_hw_header" => {
            let hdr = match arg_i(0) as u8 {
                0 => HwScalarHeader::Unit0, 1 => HwScalarHeader::Bool8, 2 => HwScalarHeader::U8, 3 => HwScalarHeader::U16,
                4 => HwScalarHeader::U32, 5 => HwScalarHeader::U64, 6 => HwScalarHeader::U128, 7 => HwScalarHeader::I8,
                8 => HwScalarHeader::I16, 9 => HwScalarHeader::I32, 10 => HwScalarHeader::I64, 11 => HwScalarHeader::I128,
                12 => HwScalarHeader::F32, 13 => HwScalarHeader::F64, 14 => HwScalarHeader::RawPtr64, _ => HwScalarHeader::Never0,
            };
            Ok(PilotVal::from_int(registry.layouts.register_hw_header(hdr).0 as i64))
        }
        "layout_scalar" => Ok(PilotVal::from_int(registry.layouts.register_scalar(arg_i(0) as u16, arg_i(1).max(1) as u16, arg_i(2) as u8).0 as i64)),
        "layout_ptr" => Ok(PilotVal::from_int(registry.layouts.register_pointer(LayoutId(arg_i(0) as u32), CapabilityBits(arg_i(1) as u8), arg_i(2) as u8).0 as i64)),
        "layout_aggregate" => {
            let fields: Vec<LayoutId> = args.iter().filter_map(|a| a.as_int().map(|v| LayoutId(v as u32))).collect();
            Ok(PilotVal::from_int(registry.layouts.register_aggregate(&fields, false, false).0 as i64))
        }
        "layout_union" => {
            let payloads: Vec<LayoutId> = args.iter().skip(1).filter_map(|a| a.as_int().map(|v| LayoutId(v as u32))).collect();
            Ok(PilotVal::from_int(registry.layouts.register_union(arg_i(0) as u8, &payloads).0 as i64))
        }
        "layout_fn_abi" => {
            let params: Vec<LayoutId> = args[..args.len().saturating_sub(1)].iter().filter_map(|a| a.as_int().map(|v| LayoutId(v as u32))).collect();
            let ret = args.last().and_then(|v| v.as_int()).map(|x| LayoutId(x as u32)).unwrap_or_default();
            Ok(PilotVal::from_int(registry.layouts.register_call_frame(&params, ret).0 as i64))
        }
        "layout_metrics" => Ok(PilotVal::from_int(registry.layouts.get(LayoutId(arg_i(0) as u32)).map_or(0, |l| l.byte_size) as i64)),
        "layout_field_offset" => Ok(PilotVal::from_int(registry.layouts.get(LayoutId(arg_i(0) as u32)).and_then(|l| match &l.geometry { PhysicsGeometry::Aggregate { fields, .. } => fields.get(arg_i(1) as usize).map(|f| f.byte_offset), _ => None }).unwrap_or(0) as i64)),
        "layout_physically_equal" => Ok(PilotVal::from_bool(matches!((registry.layouts.get(LayoutId(arg_i(0) as u32)), registry.layouts.get(LayoutId(arg_i(1) as u32))), (Some(a), Some(b)) if a.byte_size == b.byte_size && a.byte_align == b.byte_align && a.geometry == b.geometry))),
        "ast_bind_layout" => { if nid < registry.node_layouts.len() { registry.node_layouts[nid] = Some(LayoutId(arg_i(1) as u32)); } Ok(PilotVal::NULL) }
        "ast_get_bound_layout" => Ok(PilotVal::from_int(registry.node_layouts.get(nid).and_then(|x| *x).map_or(u32::MAX as i64, |l| l.0 as i64))),
        "ast_bind_target" => { if nid < registry.node_targets.len() { registry.node_targets[nid] = Some(arg_i(1) as u32); } Ok(PilotVal::NULL) }
        "ast_get_bound_target" => Ok(PilotVal::from_int(registry.node_targets.get(nid).and_then(|x| *x).map_or(u32::MAX as i64, |t| t as i64))),
        "ast_emit_diag" => { registry.diagnostics.push(PhysicsDiagnostic { node_id: nid as u32, severity: arg_i(1) as u8, message: arg_s(2) }); Ok(PilotVal::NULL) }
        "sym_str" => Ok(PilotVal::from_str(registry.symbol_strings.get(&(arg_i(0) as u32)).map(|s| s.as_str()).unwrap_or_default())),
        "sym_lookup" => Ok(PilotVal::from_int(registry.symbol_strings.iter().find(|(_, s)| s.as_str() == arg_s(0)).map(|(&k, _)| k as i64).unwrap_or(-1))),
        "str_len" => Ok(PilotVal::from_int(arg_s(0).len() as i64)),
        "str_byte_at" => Ok(PilotVal::from_int(arg_s(0).as_bytes().get(arg_i(1).max(0) as usize).copied().unwrap_or(0) as i64)),
        "ast_span_start" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.span_start as i64))),
        "ast_span_end" => Ok(PilotVal::from_int(registry.arena.nodes.get(nid).map_or(0, |n| n.span_end as i64))),
        "source_slice" => {
            let (txt, len) = (&registry.source_text, registry.source_text.len());
            let (mut s, mut e) = ((arg_i(0).max(0) as usize).min(len), (arg_i(1).max(0) as usize).min(len));
            while s > 0 && !txt.is_char_boundary(s) { s -= 1; }
            while e < len && !txt.is_char_boundary(e) { e += 1; }
            Ok(PilotVal::from_str(&txt[s.min(e)..e]))
        }
        "register_keyword_category" => {
            let (cid, scope, badge, lkind) = (arg_i(0), arg_s(1), arg_s(2), arg_i(3));
            if let Some(ex) = registry.categories.iter_mut().find(|c| c.id == cid) { ex.scope = scope; ex.badge = badge; ex.lsp_kind = lkind; }
            else { registry.categories.push(CategoryRecord { id: cid, scope, badge, lsp_kind: lkind }); }
            Ok(PilotVal::from_int(registry.categories.len() as i64))
        }
        "register_keyword" => {
            let (cid, word, doc) = (arg_i(0), arg_s(1), arg_s(2));
            if !word.is_empty() { registry.registered_symbols.push((cid, word, doc)); }
            Ok(PilotVal::from_int(registry.registered_symbols.len() as i64))
        }
        "query_registered_symbols" => {
            let (cid, mode, target) = (args.first().and_then(|v| v.as_int()).unwrap_or(-1), arg_i(1), arg_s(2));
            if mode == 1 || mode == 2 {
                let s = registry.categories.iter().find(|c| c.id == cid).map_or_else(String::new, |c| if mode == 1 { c.scope.clone() } else { c.badge.clone() });
                return Ok(PilotVal::from_str(&s));
            }
            if mode == 3 {
                let d = registry.registered_symbols.iter().find(|(c, w, _)| (cid < 0 || *c == cid) && w == &target).map_or_else(String::new, |(_, _, d)| d.clone());
                return Ok(PilotVal::from_str(&d));
            }
            let (mut out, mut seen) = (String::new(), HashSet::new());
            if cid < 0 {
                for c in &registry.categories { if seen.insert(c.scope.clone()) { if !out.is_empty() { out.push('|'); } out.push_str(&c.scope); } }
            } else {
                for (c, w, _) in &registry.registered_symbols { if *c == cid && seen.insert(w.clone()) { if !out.is_empty() { out.push('|'); } out.push_str(w); } }
            }
            Ok(PilotVal::from_str(&out))
        }
        "mount_unit" => {
            if let Ok(src) = std::fs::read_to_string(arg_s(0)) {
                let parsed = pilot_parser::parse(&src);
                let (flat, syms) = flatten_ast(&parsed, parsed.root);
                let sym_off = registry.symbol_strings.keys().copied().max().unwrap_or(0);
                let (off, l_base, aa_base) = (registry.arena.nodes.len() as u32, registry.arena.child_links.len() as u32, registry.arena.attr_args.len() as u32);
                let (at_base, bs_base) = (registry.arena.attrs.len() as u32, registry.arena.base_syms.len() as u32);
                for c in flat.child_links { registry.arena.child_links.push(c + off); }
                for (v, k) in flat.attr_args.into_iter().zip(flat.attr_arg_kind.into_iter()) {
                    registry.arena.attr_args.push(if k != 0 && v != 0 { v + sym_off as i64 } else { v });
                    registry.arena.attr_arg_kind.push(k);
                }
                for mut a in flat.attrs { if a.name_sym != 0 { a.name_sym += sym_off; } if a.arg_count > 0 { a.first_arg += aa_base; } registry.arena.attrs.push(a); }
                for b in flat.base_syms { registry.arena.base_syms.push(if b != 0 { b + sym_off } else { 0 }); }
                for mut n in flat.nodes {
                    n.node_id += off;
                    (n.span_start, n.span_end) = (u32::MAX, u32::MAX);
                    if n.sym_id != 0 { n.sym_id += sym_off; }
                    if (n.kind_tag == 4 || n.kind_tag == 21) && n.raw_bits != 0 { n.raw_bits += sym_off as u64; }
                    if n.parent_id != u32::MAX { n.parent_id += off; }
                    if n.child_count > 0 { n.first_child += l_base; }
                    if n.attr_arg_count > 0 { n.first_attr_arg += aa_base; }
                    if n.attr_count > 0 { n.first_attr += at_base; }
                    if n.base_count > 0 { n.first_base += bs_base; }
                    registry.arena.nodes.push(n);
                }
                for (k, v) in syms { registry.symbol_strings.insert(k + sym_off, v); }
                registry.index_arena_symbols(off as usize);
                let total = registry.arena.nodes.len();
                registry.node_layouts.resize(total, None);
                registry.node_targets.resize(total, None);
            }
            Ok(PilotVal::from_int(registry.arena.nodes.len() as i64))
        }
        "lsp_query_method" => Ok(PilotVal::from_str(&registry.lsp_method)),
        "lsp_query_offset" => Ok(PilotVal::from_int(registry.lsp_offset as i64)),
        "lsp_set_result" => { registry.lsp_result = arg_s(0); Ok(PilotVal::NULL) }
        _ => Ok(PilotVal::NULL),
    }
}

pub fn load_or_compile_analyzer_pilc() -> PilcBinary {
    let is_fresh = matches!((std::fs::metadata("selfhost/analyzer.pilot").and_then(|m| m.modified()), std::fs::metadata("selfhost/.pilc-cache/analyzer.pilc").and_then(|m| m.modified())), (Ok(st), Ok(ct)) if ct >= st);
    if is_fresh {
        if let Ok(bin) = std::fs::read("selfhost/.pilc-cache/analyzer.pilc").and_then(|b| PilcBinary::load_from_bytes(&b).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))) {
            return bin;
        }
    }
    let (src, has_local) = std::fs::read_to_string("selfhost/analyzer.pilot").map(|s| (s, true)).unwrap_or_else(|_| (include_str!("../../../selfhost/analyzer.pilot").to_string(), false));
    let parsed = pilot_parser::parse(&src);
    let binary = pilot_vm::compiler::BytecodeCompiler::compile(&parsed, parsed.root);
    if has_local {
        let _ = std::fs::create_dir_all("selfhost/.pilc-cache");
        if std::fs::write("selfhost/.pilc-cache/analyzer.pilc.tmp", binary.serialize()).is_ok() {
            let _ = std::fs::rename("selfhost/.pilc-cache/analyzer.pilc.tmp", "selfhost/.pilc-cache/analyzer.pilc");
        }
    }
    binary
}

pub fn run_pilot_analyzer(registry: &mut PhysicsRegistry, analyzer_src: Option<&str>) -> Result<(), String> {
    let binary = analyzer_src.map_or_else(load_or_compile_analyzer_pilc, |s| { let p = pilot_parser::parse(s); pilot_vm::compiler::BytecodeCompiler::compile(&p, p.root) });
    let mut vm = pilot_vm::vm::PilVirtualMachine::new();
    vm.load_binary(binary);
    let reg_ptr = registry as *mut PhysicsRegistry;
    vm.engine_handler = Some(Box::new(move |op_code, args| handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op_code, args)));
    vm.run();
    match vm.fault.take() {
        Some(fault) => Err(fault.to_string()),
        None => Ok(()),
    }
}

pub fn analyze_physics_with_source(arena: &AstArena, root: NodeId, source: Option<&str>, analyzer_override: Option<&str>) -> PhysicsRegistry {
    let (flat_arena, symbol_strings) = flatten_ast(arena, root);
    let mut registry = PhysicsRegistry::new(flat_arena, symbol_strings);
    if let Some(src) = source { registry.source_text = src.to_string(); }
    if let Err(e) = run_pilot_analyzer(&mut registry, analyzer_override) {
        registry.analyzer_error = Some(e);
    }
    registry
}

pub fn analyze_physics(arena: &AstArena, root: NodeId, analyzer_override: Option<&str>) -> PhysicsRegistry {
    analyze_physics_with_source(arena, root, None, analyzer_override)
}
