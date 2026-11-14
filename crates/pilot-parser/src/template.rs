//! `syntax` rule templates: the AST shape a rule's `quote { .. }` body is
//! kept in, and how a use of the rule builds nodes from it.
use pilot_ast::*;
use pilot_intern::SymbolId;
use pilot_span::ByteSpan;

#[derive(Clone, Debug)]
pub enum TemplateNode { Splice(String), TypeNamed(String, Vec<TemplateNode>), TypeTuple(Vec<TemplateNode>),
    Block(Vec<TemplateNode>), If(Box<TemplateNode>, Box<TemplateNode>, Option<Box<TemplateNode>>),
    Loop(Box<TemplateNode>), Let(String, bool, Option<Box<TemplateNode>>),
    Return(Option<Box<TemplateNode>>), Break(Option<Box<TemplateNode>>), Continue(Option<Box<TemplateNode>>),
    Unary(u16, Box<TemplateNode>), Binary(u16, Box<TemplateNode>, Box<TemplateNode>),
    Call(u16, Vec<TemplateNode>), Member(Box<TemplateNode>, String), Index(Box<TemplateNode>, Box<TemplateNode>),
    Tuple(Vec<TemplateNode>), List(Vec<TemplateNode>), Ident(String), Int(i64), Float(f64), Bool(bool), Str(String, u16), NoneLit,
    /// `target = value` (`flags` as the parser gives them) and `defer body`.
    Assign(u16, Box<TemplateNode>, Box<TemplateNode>), Defer(u16, Box<TemplateNode>),
    /// A node a template cannot hold yet; `add_rule` reports it.
    Unsupported(&'static str), }
impl TemplateNode {
    pub fn from_arena(arena: &AstArena, id: NodeId) -> Self { if id.is_none() || (id.0 as usize) >= arena.nodes.len() { return Self::NoneLit; }
        let n = *arena.node(id); match n.node_kind() { NodeKind::IdentExpr => { let s = arena.interner.resolve(SymbolId(n.lhs)).to_string();
                if n.flags == 1 { Self::Splice(s) } else { Self::Ident(s) } }
            NodeKind::TypeNamed => { let s = arena.interner.resolve(SymbolId(n.lhs)).to_string();
                if let Some(rest) = s.strip_prefix('$') { return Self::Splice(rest.to_string()); }
                if n.rhs == u32::MAX { return Self::Splice(s); }
                Self::TypeNamed(s, arena.type_args(id).iter().map(|&c| Self::from_arena(arena, NodeId(c))).collect()) }
            NodeKind::TypeTuple => Self::TypeTuple(arena.children_slice(n.lhs, n.rhs).iter().map(|&c| Self::from_arena(arena, NodeId(c))).collect()),
            NodeKind::BlockExpr | NodeKind::Program => {
                let v: Vec<Self> = arena.children_slice(n.lhs, n.rhs).iter().map(|&c| Self::from_arena(arena, NodeId(c))).collect();
                if v.len() == 1 && matches!(&v[0], Self::Loop(_) | Self::If(..) | Self::Splice(_) | Self::Int(_) | Self::Block(_)) {
                    v.into_iter().next().unwrap()
                } else { Self::Block(v) } }
            NodeKind::ExprStmt | NodeKind::ComptimeExpr => Self::from_arena(arena, NodeId(n.lhs)), NodeKind::IfExpr => {
                let c = Box::new(Self::from_arena(arena, NodeId(n.lhs))); if n.flags == 2 { let ch = arena.children_slice(n.rhs, 2);
                    Self::If(c, Box::new(Self::from_arena(arena, NodeId(ch[0]))), Some(Box::new(Self::from_arena(arena, NodeId(ch[1])))))
                } else { Self::If(c, Box::new(Self::from_arena(arena, NodeId(n.rhs))), None) } }
            NodeKind::WhileStmt => Self::Loop(Box::new(Self::from_arena(arena, NodeId(n.rhs)))),
            NodeKind::LetStmt | NodeKind::VarStmt => { let nm = arena.interner.resolve(SymbolId(n.lhs)).to_string();
                Self::Let(nm, n.node_kind() == NodeKind::VarStmt, NodeId(n.rhs).is_some().then(|| Box::new(Self::from_arena(arena, NodeId(n.rhs)))))
            }
            NodeKind::ReturnStmt => Self::Return(NodeId(n.lhs).is_some().then(|| Box::new(Self::from_arena(arena, NodeId(n.lhs))))),
            NodeKind::BreakStmt => Self::Break(NodeId(n.lhs).is_some().then(|| Box::new(Self::from_arena(arena, NodeId(n.lhs))))),
            NodeKind::ContinueStmt => Self::Continue(NodeId(n.lhs).is_some().then(|| Box::new(Self::from_arena(arena, NodeId(n.lhs))))),
            NodeKind::UnaryExpr => Self::Unary(n.flags, Box::new(Self::from_arena(arena, NodeId(n.lhs)))),
            NodeKind::BinaryExpr => Self::Binary(n.flags, Box::new(Self::from_arena(arena, NodeId(n.lhs))), Box::new(Self::from_arena(arena, NodeId(n.rhs)))),
            NodeKind::CallExpr => Self::Call(n.flags, arena.children_slice(n.lhs, n.rhs).iter().map(|&c| Self::from_arena(arena, NodeId(c))).collect()),
            NodeKind::MemberExpr => Self::Member(Box::new(Self::from_arena(arena, NodeId(n.lhs))), arena.interner.resolve(SymbolId(n.rhs)).to_string()),
            NodeKind::IndexExpr => Self::Index(Box::new(Self::from_arena(arena, NodeId(n.lhs))), Box::new(Self::from_arena(arena, NodeId(n.rhs)))),
            NodeKind::TupleExpr => Self::Tuple(arena.children_slice(n.lhs, n.rhs).iter().map(|&c| Self::from_arena(arena, NodeId(c))).collect()),
            NodeKind::ListExpr => Self::List(arena.children_slice(n.lhs, n.rhs).iter().map(|&c| Self::from_arena(arena, NodeId(c))).collect()),
            NodeKind::IntLit => Self::Int(arena.i64_pool.get(n.lhs as usize).copied().unwrap_or(0)),
            NodeKind::FloatLit => Self::Float(arena.f64_pool.get(n.lhs as usize).copied().unwrap_or(0.0)),
            NodeKind::BoolLit => Self::Bool(n.lhs != 0),
            NodeKind::StrLit => Self::Str(arena.interner.resolve(SymbolId(n.lhs)).to_string(), n.flags),
            NodeKind::AssignStmt => Self::Assign(n.flags, Box::new(Self::from_arena(arena, NodeId(n.lhs))), Box::new(Self::from_arena(arena, NodeId(n.rhs)))),
            NodeKind::DeferStmt => Self::Defer(n.flags, Box::new(Self::from_arena(arena, NodeId(n.lhs)))),
            NodeKind::NoneLit => Self::NoneLit, k => Self::Unsupported(k.name()), } }
    pub fn instantiate(&self, arena: &mut AstArena, bindings: &[(String, NodeId)], span: ByteSpan) -> NodeId { match self {
            Self::Splice(nm) => { if let Some(&(_, id)) = bindings.iter().find(|(k, _)| k == nm) { clone_ast(arena, id) }
                else { arena.alloc_ident(nm, span) } }
            Self::TypeNamed(base, args) => {
                if args.is_empty() { if let Some(&(_, id)) = bindings.iter().find(|(k, _)| k == base) { return clone_ast(arena, id); } }
                let a: Vec<NodeId> = args.iter().map(|x| x.instantiate(arena, bindings, span)).collect();
                let sym = arena.interner.intern(base); arena.alloc_type_named(sym, &a, span) }
            Self::TypeTuple(v) => { let ch: Vec<NodeId> = v.iter().map(|x| x.instantiate(arena, bindings, span)).collect(); arena.alloc_list_node(NodeKind::TypeTuple, 0, span, &ch) }
            Self::Block(v) => {
                let mut ch: Vec<NodeId> = Vec::new();
                for x in v {
                    let i = x.instantiate(arena, bindings, span);
                    if i.is_some() {
                        ch.push(as_block_item(arena, i, span));
                    }
                }
                arena.alloc_list_node(NodeKind::BlockExpr, 0, span, &ch)
            }
            Self::If(c, t, e) => { let (cid, tid) = (c.instantiate(arena, bindings, span), t.instantiate(arena, bindings, span));
                if let Some(el) = e { let eid = el.instantiate(arena, bindings, span);
                    let (st, _) = arena.alloc_child_slice(&[tid, eid]); arena.alloc_node(NodeKind::IfExpr, 2, span, cid.0, st)
                } else { arena.alloc_node(NodeKind::IfExpr, 0, span, cid.0, tid.0) } }
            Self::Loop(b) => { if let Self::Block(stmts) = b.as_ref() { if stmts.len() == 2 { if let Self::If(ec, eb, None) = &stmts[0] {
                            let is_brk = matches!(eb.as_ref(), Self::Break(None)) || matches!(eb.as_ref(), Self::Block(x) if x.len() == 1 && matches!(x[0], Self::Break(None)));
                            if is_brk { let wc = if let Self::Unary(5, ic) = ec.as_ref() { ic.instantiate(arena, bindings, span) }
                                else { let raw = ec.instantiate(arena, bindings, span); arena.alloc_node(NodeKind::UnaryExpr, 5, span, raw.0, 0) };
                                let wb = stmts[1].instantiate(arena, bindings, span);
                                return arena.alloc_node(NodeKind::WhileStmt, 0, span, wc.0, wb.0); } } } }
                let (tr, bid) = (arena.alloc_node(NodeKind::BoolLit, 0, span, 1, 0), b.instantiate(arena, bindings, span));
                arena.alloc_node(NodeKind::WhileStmt, 0, span, tr.0, bid.0) }
            Self::Let(nm, m, init) => {
                let (sym, r) = (arena.interner.intern(nm), init.as_ref().map_or(NodeId::NONE, |v| v.instantiate(arena, bindings, span)));
                arena.alloc_node(if *m { NodeKind::VarStmt } else { NodeKind::LetStmt }, 0, span, sym.0, r.0) }
            Self::Return(v) => { let r = v.as_ref().map_or(NodeId::NONE, |x| x.instantiate(arena, bindings, span)); arena.alloc_node(NodeKind::ReturnStmt, 0, span, r.0, 0) }
            Self::Break(v) => { let r = v.as_ref().map_or(NodeId::NONE, |x| x.instantiate(arena, bindings, span)); arena.alloc_node(NodeKind::BreakStmt, 0, span, r.0, 0) }
            Self::Continue(v) => { let r = v.as_ref().map_or(NodeId::NONE, |x| x.instantiate(arena, bindings, span)); arena.alloc_node(NodeKind::ContinueStmt, 0, span, r.0, 0) }
            Self::Unary(f, x) => { let v = x.instantiate(arena, bindings, span); arena.alloc_node(NodeKind::UnaryExpr, *f, span, v.0, 0) }
            Self::Binary(f, l, r) => { let (lid, rid) = (l.instantiate(arena, bindings, span), r.instantiate(arena, bindings, span)); arena.alloc_node(NodeKind::BinaryExpr, *f, span, lid.0, rid.0) }
            Self::Call(f, v) => { let ch: Vec<NodeId> = v.iter().map(|x| x.instantiate(arena, bindings, span)).collect(); arena.alloc_list_node(NodeKind::CallExpr, *f, span, &ch) }
            Self::Member(t, fld) => { let (tid, fsym) = (t.instantiate(arena, bindings, span), arena.interner.intern(fld)); arena.alloc_node(NodeKind::MemberExpr, 0, span, tid.0, fsym.0) }
            Self::Index(t, i) => { let (tid, iid) = (t.instantiate(arena, bindings, span), i.instantiate(arena, bindings, span)); arena.alloc_node(NodeKind::IndexExpr, 0, span, tid.0, iid.0) }
            Self::Tuple(v) => { let ch: Vec<NodeId> = v.iter().map(|x| x.instantiate(arena, bindings, span)).collect(); arena.alloc_list_node(NodeKind::TupleExpr, 0, span, &ch) }
            Self::List(v) => { let ch: Vec<NodeId> = v.iter().map(|x| x.instantiate(arena, bindings, span)).collect(); arena.alloc_list_node(NodeKind::ListExpr, 0, span, &ch) }
            Self::Ident(s) => arena.alloc_ident(s, span), Self::Int(v) => arena.alloc_int(*v, span),
            Self::Float(v) => arena.alloc_float(*v, span), Self::Bool(b) => arena.alloc_node(NodeKind::BoolLit, 0, span, *b as u32, 0),
            Self::Str(s, f) => { let sym = arena.interner.intern(s); arena.alloc_node(NodeKind::StrLit, *f, span, sym.0, 0) }
            Self::Assign(f, t, v) => { let (tid, vid) = (t.instantiate(arena, bindings, span), v.instantiate(arena, bindings, span)); arena.alloc_node(NodeKind::AssignStmt, *f, span, tid.0, vid.0) }
            Self::Defer(f, b) => { let bid = b.instantiate(arena, bindings, span); arena.alloc_node(NodeKind::DeferStmt, *f, span, bid.0, 0) }
            Self::NoneLit | Self::Unsupported(_) => arena.alloc_node(NodeKind::NoneLit, 0, span, 0, 0), } } }
/// A block item made from a template: an expression stands as an
/// expression statement, as the parser makes it, so a block whose last item
/// is an expression has that expression's value (`quote { $x + $x }`).
fn as_block_item(arena: &mut AstArena, id: NodeId, span: ByteSpan) -> NodeId {
    let is_expr = matches!(
        arena.node(id).node_kind(),
        NodeKind::BinaryExpr
            | NodeKind::UnaryExpr
            | NodeKind::CallExpr
            | NodeKind::MemberExpr
            | NodeKind::IndexExpr
            | NodeKind::TupleExpr
            | NodeKind::ListExpr
            | NodeKind::IdentExpr
            | NodeKind::IntLit
            | NodeKind::FloatLit
            | NodeKind::BoolLit
            | NodeKind::StrLit
            | NodeKind::NoneLit
    );
    if is_expr {
        arena.alloc_node(NodeKind::ExprStmt, 0, span, id.0, 0)
    } else {
        id
    }
}
pub(crate) fn clone_ast(arena: &mut AstArena, id: NodeId) -> NodeId { if id.is_none() || (id.0 as usize) >= arena.nodes.len() { return NodeId::NONE; }
    let (n, sp, ann) = (*arena.node(id), arena.span(id), arena.type_annot(id)); let out = match n.node_kind() { NodeKind::TypeNamed => {
            let args: Vec<NodeId> = arena.type_args(id).to_vec().into_iter().map(|c| clone_ast(arena, NodeId(c))).collect();
            arena.alloc_type_named(SymbolId(n.lhs), &args, sp) }
        NodeKind::TypeTuple | NodeKind::BlockExpr | NodeKind::TupleExpr | NodeKind::ListExpr | NodeKind::MapExpr | NodeKind::CallExpr => {
            let ch: Vec<NodeId> = arena.children_slice(n.lhs, n.rhs).to_vec().into_iter().map(|c| clone_ast(arena, NodeId(c))).collect();
            arena.alloc_list_node(n.node_kind(), n.flags, sp, &ch) }
        NodeKind::BinaryExpr | NodeKind::AssignStmt | NodeKind::IndexExpr | NodeKind::WhileStmt | NodeKind::GuardStmt => {
            let (l, r) = (clone_ast(arena, NodeId(n.lhs)), clone_ast(arena, NodeId(n.rhs)));
            arena.alloc_node(n.node_kind(), n.flags, sp, l.0, r.0) }
        NodeKind::IfExpr => { let c = clone_ast(arena, NodeId(n.lhs)); if n.flags == 2 { let raw = arena.children_slice(n.rhs, 2).to_vec();
                let (t, e) = (clone_ast(arena, NodeId(raw[0])), clone_ast(arena, NodeId(raw[1])));
                let (st, _) = arena.alloc_child_slice(&[t, e]); arena.alloc_node(NodeKind::IfExpr, 2, sp, c.0, st)
            } else { let t = clone_ast(arena, NodeId(n.rhs)); arena.alloc_node(NodeKind::IfExpr, 0, sp, c.0, t.0) } }
        NodeKind::UnaryExpr | NodeKind::ReturnStmt | NodeKind::BreakStmt | NodeKind::ContinueStmt | NodeKind::ExprStmt | NodeKind::MemberExpr => {
            let l = clone_ast(arena, NodeId(n.lhs)); arena.alloc_node(n.node_kind(), n.flags, sp, l.0, n.rhs) }
        NodeKind::LetStmt | NodeKind::VarStmt => { let r = clone_ast(arena, NodeId(n.rhs)); arena.alloc_node(n.node_kind(), n.flags, sp, n.lhs, r.0) }
        _ => arena.alloc_node(n.node_kind(), n.flags, sp, n.lhs, n.rhs), };
    if let Some(a) = ann { let ca = clone_ast(arena, a); arena.set_type_annot(out, ca); }
    out }
