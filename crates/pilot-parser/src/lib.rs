//! Homoiconic 14-Token Seed Parser & `$param` AST-Splicing Macro Engine (`pilot-parser`).
//! Hardcodes strictly the 14 seed `TokenKind`s; zero Pilot scalar/composite type names exist here.
use std::borrow::Cow; use std::collections::{HashMap, HashSet}; use std::sync::OnceLock; use pilot_ast::*; use pilot_intern::SymbolId;
use pilot_lexer::{tokenize, TokenKind, TokenTape}; use pilot_span::ByteSpan;
mod accessors;
pub use accessors::{INDEX_GET_OP, INDEX_SET_OP};
mod attr_hooks;
mod generic_receiver;
mod op_decl;
mod modules;
mod pilot_impls;
pub use modules::{parse_header, parse_unit, ModuleDecl};
use modules::EXCLUDE_RECEPTOR;
pub use pilot_impls::{EnumeratorProtocol, GeneratorProtocol, NativeKind, OperatorFn, TryProtocol, UnitLiteralHook};
use pilot_impls::*;
pub use attr_hooks::RegLayout;
#[derive(Clone, Debug)]
pub enum TemplateNode { Splice(String), TypeNamed(String, Vec<TemplateNode>), TypeTuple(Vec<TemplateNode>),
    Block(Vec<TemplateNode>), If(Box<TemplateNode>, Box<TemplateNode>, Option<Box<TemplateNode>>),
    Loop(Box<TemplateNode>), Let(String, bool, Option<Box<TemplateNode>>),
    Return(Option<Box<TemplateNode>>), Break(Option<Box<TemplateNode>>), Continue(Option<Box<TemplateNode>>),
    Unary(u16, Box<TemplateNode>), Binary(u16, Box<TemplateNode>, Box<TemplateNode>),
    Call(u16, Vec<TemplateNode>), Member(Box<TemplateNode>, String), Index(Box<TemplateNode>, Box<TemplateNode>),
    Tuple(Vec<TemplateNode>), List(Vec<TemplateNode>), Ident(String), Int(i64), Float(f64), Bool(bool), Str(String, u16), NoneLit, }
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
            NodeKind::StrLit => Self::Str(arena.interner.resolve(SymbolId(n.lhs)).to_string(), n.flags), _ => Self::NoneLit, } }
    pub fn instantiate(&self, arena: &mut AstArena, bindings: &[(String, NodeId)], span: ByteSpan) -> NodeId { match self {
            Self::Splice(nm) => { if let Some(&(_, id)) = bindings.iter().find(|(k, _)| k == nm) { clone_ast(arena, id) }
                else { arena.alloc_ident(nm, span) } }
            Self::TypeNamed(base, args) => {
                if args.is_empty() { if let Some(&(_, id)) = bindings.iter().find(|(k, _)| k == base) { return clone_ast(arena, id); } }
                let a: Vec<NodeId> = args.iter().map(|x| x.instantiate(arena, bindings, span)).collect();
                let sym = arena.interner.intern(base); arena.alloc_type_named(sym, &a, span) }
            Self::TypeTuple(v) => { let ch: Vec<NodeId> = v.iter().map(|x| x.instantiate(arena, bindings, span)).collect(); arena.alloc_list_node(NodeKind::TypeTuple, 0, span, &ch) }
            Self::Block(v) => { let ch: Vec<NodeId> = v.iter().map(|x| x.instantiate(arena, bindings, span)).filter(|i| i.is_some()).collect(); arena.alloc_list_node(NodeKind::BlockExpr, 0, span, &ch) }
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
            Self::NoneLit => arena.alloc_node(NodeKind::NoneLit, 0, span, 0, 0), } } }
fn clone_ast(arena: &mut AstArena, id: NodeId) -> NodeId { if id.is_none() || (id.0 as usize) >= arena.nodes.len() { return NodeId::NONE; }
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
#[derive(Clone, Debug)]
pub enum TypePostfixKind { BracketEmpty, BracketDotDot, BracketColon, BracketArgs(usize), Sym(String, bool) }
#[derive(Clone, Debug)]
pub enum TypeConstructRule { Prefix { op: String, param: String, template: TemplateNode },
    Postfix { kind: TypePostfixKind, params: Vec<String>, template: TemplateNode },
    Infix { op: String, bp: u8, right_assoc: bool, lhs_param: String, rhs_param: String, template: TemplateNode },
    NamedGeneric { name: String, params: Vec<String>, template: TemplateNode }, }
#[derive(Clone, Debug)]
pub struct SyntaxMacroRule { pub params: Vec<String>, pub template: TemplateNode }
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ModAct { Vis(u8), Partial, Static, Pure, Mut, Const, Iso, Ext, Async, Ring }
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PrattEntry { pub bp: u8, pub right_assoc: bool, pub bin_op: u16 }
#[derive(Clone, Debug)]
enum CVal { Str(String), Int(i64), Map(HashMap<String, CVal>) }
#[derive(Clone, Debug, Default)]
pub struct ParserTables { modifiers: HashMap<String, ModAct>, param_modifiers: HashSet<String>, module_kws: HashMap<String, bool>,
    pub receptors: HashMap<String, (u8, u8)>, pub receiver_ident: String,
    /// `field` and `value` inside accessor bodies (`bind_receiver_ident`).
    pub backing_ident: String, pub setter_value_ident: String,
    pub prefix_ops: HashMap<String, u8>, pub infix_ops: HashMap<String, PrattEntry>, pub postfix_ops: HashMap<String, u8>,
    pub type_rules: Vec<TypeConstructRule>, pub syntax_macros: HashMap<String, SyntaxMacroRule>, const_maps: HashMap<String, CVal>,
    /// Attribute functions by name (see `attr_hooks`).
    attr_hooks: HashMap<String, attr_hooks::AttrHook>,
    /// Register layouts declared with `[Bits(..)]`, by type name.
    reg_layouts: HashMap<String, RegLayout>,
    /// The Pilot functions that value dimensional literals (see `pilot_impls`).
    unit_literal: Option<UnitLiteralHook>,
    /// The Pilot function each infix operator calls, by symbol (see `pilot_impls`).
    operator_fns: HashMap<String, OperatorFn>,
    /// Names bound to engine intrinsics, by kind (see `pilot_impls`).
    natives: HashMap<(NativeKind, String), String>,
    enumerator: Option<EnumeratorProtocol>,
    generator: Option<GeneratorProtocol>,
    try_protocol: Option<TryProtocol>, }
impl ParserTables {
    pub fn new() -> Self { Self::default() }
    fn eval_cval(&self, arena: &AstArena, id: NodeId) -> Option<CVal> { if id.is_none() { return None; }
        let n = arena.node(id); match n.node_kind() { NodeKind::StrLit => Some(CVal::Str(arena.interner.resolve(SymbolId(n.lhs)).to_string())),
            NodeKind::IntLit => Some(CVal::Int(arena.i64_pool.get(n.lhs as usize).copied().unwrap_or(0))),
            NodeKind::IdentExpr => { let s = arena.interner.resolve(SymbolId(n.lhs)); self.const_maps.get(s).cloned().or_else(|| Some(CVal::Str(s.to_string()))) }
            NodeKind::MemberExpr => if let Some(CVal::Map(m)) = self.eval_cval(arena, NodeId(n.lhs)) { m.get(arena.interner.resolve(SymbolId(n.rhs))).cloned() } else { None },
            NodeKind::CallExpr => { let ch = arena.children_slice(n.lhs, n.rhs); ch.get(1).and_then(|&x| self.eval_cval(arena, NodeId(x))) },
            NodeKind::MapExpr => { let (ch, mut m, mut i) = (arena.children_slice(n.lhs, n.rhs), HashMap::new(), 0);
                while i + 1 < ch.len() { let kn = arena.node(NodeId(ch[i])); if matches!(kn.node_kind(), NodeKind::IdentExpr | NodeKind::StrLit) {
                        if let Some(v) = self.eval_cval(arena, NodeId(ch[i + 1])) { m.insert(arena.interner.resolve(SymbolId(kn.lhs)).to_string(), v); }
                    }
                    i += 2; }
                Some(CVal::Map(m)) }
            _ => None, } }
    fn ingest_node(&mut self, arena: &AstArena, id: NodeId) { if id.is_none() { return; }
        let n = *arena.node(id); match n.node_kind() {
            NodeKind::Program | NodeKind::BlockExpr => for &c in arena.children_slice(n.lhs, n.rhs).to_vec().iter() { self.ingest_node(arena, NodeId(c)); },
            NodeKind::ComptimeExpr | NodeKind::ExprStmt => self.ingest_node(arena, NodeId(n.lhs)),
            NodeKind::FnDecl => self.ingest_attr_fn(arena, id),
            NodeKind::StructDecl => self.ingest_decl_attrs(arena, id),
            NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
                if let Some(v) = self.eval_cval(arena, NodeId(n.rhs)) { self.const_maps.insert(arena.interner.resolve(SymbolId(n.lhs)).to_string(), v); }
            }
            NodeKind::CallExpr => { let ch = arena.children_slice(n.lhs, n.rhs);
                if ch.len() < 2 || arena.node(NodeId(ch[0])).node_kind() != NodeKind::IdentExpr || arena.interner.resolve(SymbolId(arena.node(NodeId(ch[0])).lhs)) != "__engine" { return; }
                let s_arg = |i: usize| ch.get(i).and_then(|&x| match self.eval_cval(arena, NodeId(x))? { CVal::Str(s) => Some(s), _ => None }).unwrap_or_default();
                let i_arg = |i: usize| ch.get(i).and_then(|&x| match self.eval_cval(arena, NodeId(x))? { CVal::Int(v) => Some(v), _ => None });
                match s_arg(1).as_str() { "register_modifier" => {
                        let (w, act) = (s_arg(2), if let Some(v) = i_arg(3) { ModAct::Vis(v as u8) } else { match s_arg(3).as_str() {
                                "merge_type_members" => ModAct::Partial, "storage_static" => ModAct::Static, "effect_pure" => ModAct::Pure,
                                "binding_mutable" => ModAct::Mut, "binding_const" => ModAct::Const, "perceus_isolated" => ModAct::Iso,
                                "extension_member" => ModAct::Ext, "effect_async" => ModAct::Async, _ => ModAct::Ring, }
                        }); if !w.is_empty() { self.modifiers.insert(w, act); } }
                    "register_parametric_modifier" => { let w = s_arg(2); if !w.is_empty() { self.param_modifiers.insert(w); } }
                    "register_module_kw" => { let w = s_arg(2); if !w.is_empty() { self.module_kws.insert(w, s_arg(3) == "declare_package_scope"); } }
                    HOOK_UNIT_LITERAL => self.bind_unit_literal(s_arg(2), s_arg(3)),
                    HOOK_OPERATOR_FN => self.bind_operator_fn(s_arg(2), s_arg(3)),
                    HOOK_NATIVE_FN => self.bind_native(NativeKind::Function, s_arg(2), s_arg(3)),
                    HOOK_NATIVE_METHOD => self.bind_native(NativeKind::Method, s_arg(2), s_arg(3)),
                    HOOK_NATIVE_PROPERTY => self.bind_native(NativeKind::Property, s_arg(2), s_arg(3)),
                    HOOK_NATIVE_VALUE => self.bind_native(NativeKind::Value, s_arg(2), s_arg(3)),
                    HOOK_TYPE_QUERY => self.bind_native(NativeKind::TypeQuery, s_arg(2), s_arg(3)),
                    HOOK_ENUMERATOR => self.bind_enumerator(s_arg(2), s_arg(3), s_arg(4), s_arg(5)),
                    HOOK_GENERATOR => self.bind_generator(s_arg(2), s_arg(3), s_arg(4), s_arg(5)),
                    HOOK_TRY_PROTOCOL => self.bind_try_protocol(s_arg(2), s_arg(3), s_arg(4)),
                    "bind_receiver_ident" => { let (w, b, v) = (s_arg(2), s_arg(3), s_arg(4)); if !w.is_empty() { self.receiver_ident = w; }
                        self.backing_ident = b; self.setter_value_ident = v; }
                    "bind_receptor" => { let (w, k, p) = (s_arg(2), i_arg(3).unwrap_or(0) as u8, i_arg(4).unwrap_or(0) as u8); if !w.is_empty() && k > 0 { self.receptors.insert(w, (k, p)); } }
                    "register_pratt_op" => { let (sym, fix, bp, assoc) = (s_arg(2), s_arg(3), i_arg(4).unwrap_or(20) as u8, s_arg(5));
                        if !sym.is_empty() { match fix.as_str() { "prefix" => { self.prefix_ops.insert(sym, bp); }
                                "postfix" => { self.postfix_ops.insert(sym, bp); }
                                _ => { let code = map_binop_sym(&sym); self.infix_ops.insert(sym, PrattEntry { bp, right_assoc: assoc == "right", bin_op: code }); }
                            } } }
                    _ => {} } }
            _ => {} } } }
/// `MemberExpr.flags` of a prefix-dot member (`.name`), whose receiver the
/// parser filled in. `0` is `a.b` and `1` is `a?.b`.
pub const MEMBER_IMPLIED_RECEIVER: u16 = 2;
fn map_binop_sym(sym: &str) -> u16 { use BinaryOp::*; (match sym {
        "+" => Add, "-" => Sub, "*" => Mul, "/" => Div, "//" => FloorDiv, "%" => Rem, "**" => Pow,
        "==" => Eq, "!=" => Ne, "<" => Lt, "<=" => Le, ">" => Gt, ">=" => Ge, "&&" | "and" => And,
        "||" | "or" => Or, "&" => BitAnd, "|" => BitOr, "^" => BitXor, "<<" => Shl, ">>" => Shr,
        "|>" => Pipe, "<|" => PipeBackward, "??" => Coalesce, "..=" => RangeInclusive,
        ".." | "..<" => RangeHalfOpen, "is" => Is, ":=" => Walrus, "<-" => Send, "union" => Union,
        "intersect" => Intersect, "except" => Except, "<=>" => Spaceship, "+|" => SatAdd,
        "-|" => SatSub, "*|" => SatMul, "+%" => WrapAdd, "-%" => WrapSub, "*%" => WrapMul,
        "+?" => CheckedAdd, "-?" => CheckedSub, "*?" => CheckedMul, "%%" => EucMod, "~>" => Lens,
        ">..<" => OpenRange, ">.." => AfterRange, ":=:" => Swap, _ => return 0, }) as u16 }
/// The stdlib files whose `syntax`, keyword, native-binding and protocol
/// declarations fill the bootstrap tables, relative to the source root, in
/// ingest order. Every `stdlib/core/primitives/*.pilot` file follows them.
const BOOTSTRAP_SOURCES: &[&str] = &[
    "stdlib/core/core.pilot",
    "stdlib/core/syntax/type_construct.pilot",
    "stdlib/core/syntax/visibility.pilot",
    "stdlib/core/syntax/lifecycle.pilot",
    "stdlib/core/syntax/properties.pilot",
    "stdlib/core/syntax/module_ext.pilot",
    "stdlib/core/syntax/attribute.pilot",
    "stdlib/core/syntax/for.pilot",
    "stdlib/core/generator.pilot",
    "stdlib/core/syntax/while.pilot",
    "stdlib/core/syntax/until.pilot",
    "stdlib/core/syntax/switch.pilot",
    "stdlib/core/syntax/guard.pilot",
    "stdlib/core/syntax/try_catch.pilot",
    "stdlib/core/syntax/expr_sugar.pilot",
    "stdlib/core/syntax/fstring.pilot",
    "stdlib/core/syntax/call_quote.pilot",
    "stdlib/core/syntax/unit_decl.pilot",
    "stdlib/core/builtins/native_calls.pilot",
    "stdlib/db/sql_syntax.pilot",
    "stdlib/db/set_ops.pilot",
    "stdlib/polyglot/polyglot_syntax.pilot",
    "stdlib/realtime/rt_syntax.pilot",
    "stdlib/asyncio/select_syntax.pilot",
    "stdlib/contracts/contracts_syntax.pilot",
];

/// Where the primitive declarations live, relative to the source root.
const PRIMITIVES_DIR: &str = "stdlib/core/primitives";

static SOURCE_ROOT: OnceLock<std::path::PathBuf> = OnceLock::new();

/// Sets the directory holding the `stdlib/` the bootstrap tables are read
/// from. The toolchain calls it once, before anything is parsed, with the
/// root it runs with; later calls have no effect. Without it the tables come
/// from the workspace this crate was built in.
pub fn set_source_root(root: &std::path::Path) {
    let _ = SOURCE_ROOT.set(root.to_path_buf());
}

/// The directory holding the `stdlib/` the toolchain runs with.
pub fn source_root() -> std::path::PathBuf {
    SOURCE_ROOT
        .get()
        .cloned()
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

/// The sources of the bootstrap tables: `BOOTSTRAP_SOURCES`, then every
/// primitive file in file-name order, so adding a primitive file needs no
/// change here. A missing file is reported and skipped.
fn bootstrap_sources() -> Vec<String> {
    let root = source_root();
    let mut out = Vec::new();
    for rel in BOOTSTRAP_SOURCES {
        match std::fs::read_to_string(root.join(rel)) {
            Ok(src) => out.push(src),
            Err(e) => eprintln!("internal error: bootstrap source `{rel}` unreadable: {e}"),
        }
    }
    let mut files: Vec<_> = std::fs::read_dir(root.join(PRIMITIVES_DIR))
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "pilot"))
        .collect();
    files.sort();
    out.extend(files.iter().filter_map(|p| std::fs::read_to_string(p).ok()));
    out
}

static BOOTSTRAP_TABLES: OnceLock<ParserTables> = OnceLock::new();

/// The tables built from the stdlib at the source root (`set_source_root`).
pub fn bootstrap_tables() -> &'static ParserTables {
    BOOTSTRAP_TABLES.get_or_init(|| {
        let mut t = ParserTables::new();
        for src in bootstrap_sources() {
            let _ = parse_and_ingest(&src, &mut t);
        }
        t
    })
}
pub fn parse(src: &str) -> AstArena { parse_with_tables(src, bootstrap_tables()) }
pub fn parse_with_tables(src: &str, tables: &ParserTables) -> AstArena { let mut p = Parser { src, tape: tokenize(src), pos: 0, arena: AstArena::with_capacity((src.len() / 12).max(16)), tables: Cow::Borrowed(tables), active_hdr: None, hdr_stack: Vec::new(), in_quote_type: false, no_struct_lit: false, in_cascade_step: false, ingest: false, }; p.parse_program(); p.arena }
pub fn parse_and_ingest(src: &str, tables: &mut ParserTables) -> AstArena { let mut p = Parser { src, tape: tokenize(src), pos: 0, arena: AstArena::with_capacity((src.len() / 12).max(16)), tables: Cow::Borrowed(tables), active_hdr: None, hdr_stack: Vec::new(), in_quote_type: false, no_struct_lit: false, in_cascade_step: false, ingest: true, }; p.parse_program(); let (arena, owned) = (p.arena, match p.tables { Cow::Owned(o) => Some(o), _ => None }); if let Some(o) = owned { *tables = o; } tables.finish_ingest(src); arena }
#[inline]
fn is_seed_kw(k: TokenKind) -> bool { matches!(k, TokenKind::KwEngine | TokenKind::KwStruct | TokenKind::KwInterface | TokenKind::KwFn
        | TokenKind::KwLet | TokenKind::KwRef | TokenKind::KwIf | TokenKind::KwLoop
        | TokenKind::KwReturn | TokenKind::KwYield | TokenKind::KwDefer | TokenKind::KwComptime
        | TokenKind::KwQuote | TokenKind::KwAsm) }
#[inline]
fn is_word_tok(k: TokenKind) -> bool { k == TokenKind::Ident }
#[derive(Clone, Debug, Default)]
struct PendingAttr { name: SymbolId, args: Vec<NodeId>, is_mut: bool, bind_sym: SymbolId }
#[derive(Clone, Debug, Default)]
struct DeclMods { vis: Option<u8>, flags: u16, is_const: bool, is_ext: bool, attrs: Vec<PendingAttr>, ret_ty: Option<TemplateNode> }
impl DeclMods {
    fn merge_under(&self, hdr: Option<&ScopeHeader>) -> Self { let Some(h) = hdr else { return self.clone(); };
        let mut attrs: Vec<PendingAttr> = h.mods.attrs.iter().filter(|ha| !self.attrs.iter().any(|la| la.name == ha.name)).cloned().collect();
        attrs.extend(self.attrs.iter().cloned()); Self {
            vis: self.vis.or(h.mods.vis), flags: h.mods.flags | self.flags, is_const: self.is_const || h.mods.is_const,
            is_ext: self.is_ext || h.mods.is_ext, attrs, ret_ty: self.ret_ty.clone().or_else(|| h.mods.ret_ty.clone()), } }
    fn is_non_empty(&self) -> bool {
        self.vis.is_some() || self.flags != 0 || self.is_const || self.is_ext || !self.attrs.is_empty() || self.ret_ty.is_some() } }
#[derive(Clone, Debug)]
struct ScopeHeader { mods: DeclMods, target: u8 }
struct Parser<'a> { src: &'a str, tape: TokenTape, pos: usize, arena: AstArena, tables: Cow<'a, ParserTables>,
    active_hdr: Option<ScopeHeader>, hdr_stack: Vec<Option<ScopeHeader>>, in_quote_type: bool, no_struct_lit: bool,
    /// Parsing a cascade step (`..f(x)`, `..field = value`), which ends at the next step.
    in_cascade_step: bool,
    /// Feeding the bootstrap tables: every item is ingested, not only engine calls.
    ingest: bool, }
impl<'a> Parser<'a> {
    #[inline] fn cur(&self) -> usize { self.next_non_nl(self.pos) }
    #[inline] fn skip_seps(&mut self) { while self.eat(TokenKind::Semicolon) {} }
    #[inline] fn peek(&self) -> TokenKind { let p = self.cur(); if p < self.tape.len() { self.tape.kind(p) } else { TokenKind::Eof } }
    #[inline] fn at(&self, k: TokenKind) -> bool { self.peek() == k }
    #[inline] fn at_end(&self) -> bool { self.peek() == TokenKind::Eof }
    #[inline] fn is_word(&self) -> bool { is_word_tok(self.peek()) }
    #[inline] fn span(&self) -> ByteSpan { let p = self.cur(); if p < self.tape.len() { self.tape.span(p) } else { ByteSpan::new(self.src.len() as u32, self.src.len() as u32) } }
    #[inline] fn prev_span(&self) -> ByteSpan { if self.pos > 0 && self.pos - 1 < self.tape.len() { self.tape.span(self.pos - 1) } else { ByteSpan::new(0, 0) } }
    #[inline] fn text(&self) -> &'a str { let p = self.cur(); if p < self.tape.len() { self.tape.text(p, self.src) } else { "" } }
    #[inline] fn advance(&mut self) { let p = self.cur(); if p < self.tape.len() { self.pos = p + 1; } }
    #[inline] fn eat(&mut self, k: TokenKind) -> bool { let p = self.cur(); if p < self.tape.len() && self.tape.kind(p) == k { self.pos = p + 1; true } else { false } }
    #[inline] fn intern_cur(&mut self) -> SymbolId { let s = self.arena.interner.intern(self.text()); self.advance(); s }
    fn expect(&mut self, k: TokenKind) {
        if !self.eat(k) { let (sp, got) = (self.span(), self.peek()); self.arena.diagnostics.push((sp, format!("Expected {:?}, found {:?}", k, got))); }
    }
    fn next_non_nl(&self, mut p: usize) -> usize {
        while p < self.tape.len() && matches!(self.tape.kind(p), TokenKind::Newline | TokenKind::Comment | TokenKind::BlockComment | TokenKind::Shebang) { p += 1; }
        p }
    #[inline] fn receptor(&self) -> Option<(u8, u8)> { if self.is_word() { self.tables.receptors.get(self.text()).copied() } else { None } }
    #[inline] fn is_rec(&self, kind: u8) -> bool { self.receptor().is_some_and(|(k, _)| k == kind) }
    #[inline] fn is_receiver(&self) -> bool { self.is_word() && !self.tables.receiver_ident.is_empty() && self.text() == self.tables.receiver_ident }
    fn apply_mods(&mut self, id: NodeId, mods: &DeclMods, def_vis: u8) { if id.is_none() || (id.0 as usize) >= self.arena.nodes.len() { return; }
        let vis = mods.vis.unwrap_or(def_vis) as u16;
        let written = if mods.vis.is_some() { DECL_FLAG_VIS_WRITTEN } else { 0 };
        self.arena.decl_flags[id.0 as usize] = (self.arena.decl_flags[id.0 as usize] & !DECL_VIS_MASK) | (vis & DECL_VIS_MASK) | mods.flags | written;
        for a in &mods.attrs { let args: Vec<NodeId> = a.args.iter().map(|&x| clone_ast(&mut self.arena, x)).collect();
            let (st, ln) = self.arena.alloc_child_slice(&args);
            self.arena.attrs.push(AstAttribute { target: id, name: a.name, args_start: st, args_len: ln as u16, is_mut_binding: a.is_mut, binding_sym: a.bind_sym });
        }
        if let Some(rt) = &mods.ret_ty { if self.arena.type_annot(id).is_none() {
                let sp = self.arena.span(id); let tid = rt.instantiate(&mut self.arena, &[], sp); self.arena.set_type_annot(id, tid); }
        } }
    fn parse_program(&mut self) { let mut items = Vec::new(); while !self.at_end() { self.skip_seps(); if self.at_end() { break; }
            let mark = self.pos; self.parse_item(&mut items, false); if self.pos == mark { self.advance(); }
            if let Some(&last) = items.last() { if self.ingest || self.src.contains("__engine") { self.tables.to_mut().ingest_node(&self.arena, last); } } }
        self.arena.root = self.arena.alloc_list_node(NodeKind::Program, 0, ByteSpan::new(0, self.src.len() as u32), &items); }
    fn is_attr_prefix(&self, _in_type_body: bool) -> bool { if self.peek() != TokenKind::LBracket { return false; }
        let mut p = self.cur(); while p < self.tape.len() && self.tape.kind(p) == TokenKind::LBracket {
            let Some(close) = self.tape.matching_bracket(p) else { return false; }; p = self.next_non_nl(close as usize + 1); }
        if p >= self.tape.len() { return false; }
        let tk = self.tape.kind(p); is_seed_kw(tk) || is_word_tok(tk) || matches!(tk, TokenKind::LBrace | TokenKind::Tilde) }
    fn parse_attrs(&mut self) -> Vec<PendingAttr> { let mut out = Vec::new(); while self.at(TokenKind::LBracket) { self.advance();
            while !self.at_end() && !self.at(TokenKind::RBracket) { let (mut is_mut, mut bind_sym) = (false, SymbolId(0));
                if self.at(TokenKind::KwLet) || (self.is_word() && self.tables.modifiers.get(self.text()) == Some(&ModAct::Mut)) {
                    is_mut = !self.at(TokenKind::KwLet); self.advance(); if self.is_word() { bind_sym = self.intern_cur(); }
                    let _ = self.eat(TokenKind::Equal); }
                let (mark, mut nm) = (self.pos, String::new()); while self.is_word() || is_seed_kw(self.peek()) {
                    nm.push_str(self.text()); self.advance();
                    if self.eat(TokenKind::InfixDot) || self.eat(TokenKind::ColonColon) { nm.push('.'); } else { break; } }
                let (name, mut args) = (self.arena.interner.intern(&nm), Vec::new()); if self.eat(TokenKind::LParen) {
                    while !self.at_end() && !self.at(TokenKind::RParen) { let nx = self.next_non_nl(self.cur() + 1);
                        if self.is_word() && nx < self.tape.len() && matches!(self.tape.kind(nx), TokenKind::Equal | TokenKind::Colon) { self.advance(); self.advance(); }
                        let a = self.parse_expr(); if a.is_some() { args.push(a); }
                        if self.eat(TokenKind::Comma) {
                                                        if self.at(TokenKind::RParen) { self.arena.diagnostics.push((self.prev_span(), "Trailing comma not permitted in argument list".into())); break; }
                        } else { break; } }
                    self.expect(TokenKind::RParen); }
                if !nm.is_empty() { out.push(PendingAttr { name, args, is_mut, bind_sym }); }
                if self.eat(TokenKind::Comma) {
                                        if self.at(TokenKind::RBracket) { self.arena.diagnostics.push((self.prev_span(), "Trailing comma not permitted in attribute brackets".into())); break; }
                } else if self.pos == mark { self.advance(); } }
            self.expect(TokenKind::RBracket); }
        out }
    fn parse_mods(&mut self, in_type_body: bool) -> DeclMods { let mut m = DeclMods::default();
        if self.is_attr_prefix(in_type_body) { m.attrs = self.parse_attrs(); }
        loop { let nx = self.next_non_nl(self.cur() + 1); if nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LBrace
                && self.next_non_nl(nx + 1) < self.tape.len() && self.tape.kind(self.next_non_nl(nx + 1)) == TokenKind::RBrace
            { break; }
            // `comptime { .. }` is a comptime block, not a modifier on a block.
            if self.at(TokenKind::KwComptime) && !(nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LBrace) { self.advance(); m.flags |= DECL_FLAG_COMPTIME; continue; }
            if !self.is_word() { break; }
            let w = self.text(); if self.tables.param_modifiers.contains(w) && nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LParen {
                self.advance(); self.eat(TokenKind::LParen); let mut d = 1;
                while !self.at_end() && d > 0 { if self.at(TokenKind::LParen) { d += 1; } else if self.at(TokenKind::RParen) { d -= 1; } self.advance(); }
                continue; }
            if self.tables.module_kws.get(w) == Some(&true) && nx < self.tape.len() && (is_word_tok(self.tape.kind(nx)) || self.tape.kind(nx) == TokenKind::LBrace) && !self.tables.modifiers.contains_key(self.tape.text(nx, self.src)) {
                break; }
            let Some(&act) = self.tables.modifiers.get(w) else { break; };
            // After `var`/`const`, a modifier word followed by `=`, `:`, `;` or `,`
            // is the binding's name (`var pkg = 1;`), not another modifier.
            let binding_started = (m.flags & DECL_FLAG_MUT) != 0 || m.is_const;
            if binding_started && nx < self.tape.len()
                && matches!(self.tape.kind(nx), TokenKind::Equal | TokenKind::Colon | TokenKind::Semicolon | TokenKind::Comma)
            { break; }
            self.advance(); match act {
                ModAct::Vis(v) => m.vis = Some(v), ModAct::Partial => m.flags |= DECL_FLAG_PARTIAL,
                ModAct::Static => m.flags |= DECL_FLAG_STATIC, ModAct::Pure => m.flags |= DECL_FLAG_PURE,
                ModAct::Mut => m.flags |= DECL_FLAG_MUT, ModAct::Const => m.is_const = true,
                ModAct::Ext => m.is_ext = true, ModAct::Iso | ModAct::Async | ModAct::Ring => {} } }
        m }
    fn parse_item(&mut self, out: &mut Vec<NodeId>, in_type_body: bool) { let start_pos = self.pos;
        let raw_mods = self.parse_mods(in_type_body); if raw_mods.is_non_empty() { let (mut target, saved) = (0u8, self.pos);
            if self.eat(TokenKind::KwFn) { target = 1; }
            else if self.eat(TokenKind::KwStruct) { target = 2; }
            else if self.eat(TokenKind::KwInterface) { target = 3; }
            else if self.is_rec(27) { self.advance(); target = 4; }
                        let mut hdr_mods = raw_mods.clone(); if self.eat(TokenKind::Arrow) {
                let rt = self.parse_type(); hdr_mods.ret_ty = Some(TemplateNode::from_arena(&self.arena, rt));             }
            if self.eat(TokenKind::Colon) { let merged = hdr_mods.merge_under(self.hdr_stack.last().and_then(|x| x.as_ref()));
                self.active_hdr = Some(ScopeHeader { mods: merged, target }); return; }
            if self.at(TokenKind::LBrace) {
                let (merged, blk_sp) = (hdr_mods.merge_under(self.active_hdr.as_ref()), self.span()); self.advance();
                self.hdr_stack.push(self.active_hdr.take()); self.active_hdr = Some(ScopeHeader { mods: merged, target });
                let mut inner = Vec::new(); while !self.at_end() && !self.at(TokenKind::RBrace) {
                    self.skip_seps(); if self.at_end() || self.at(TokenKind::RBrace) { break; }
                    let mp = self.pos; self.parse_headless_or_item(&mut inner, target, in_type_body); if self.pos == mp { self.advance(); } }
                self.expect(TokenKind::RBrace); self.active_hdr = self.hdr_stack.pop().unwrap_or(None);
                out.push(self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(blk_sp.start, self.prev_span().end), &inner)); return; }
            self.pos = saved;
            if (raw_mods.flags & DECL_FLAG_COMPTIME) != 0 && self.at(TokenKind::LBrace) && raw_mods.vis.is_none() && raw_mods.attrs.is_empty() {
                let b = self.parse_block(); out.push(self.arena.alloc_node(NodeKind::ComptimeExpr, 0, self.arena.span(b), b.0, 0)); return; } }
        let hdr_snap = self.active_hdr.clone(); let (mods, def_vis) = (raw_mods.merge_under(hdr_snap.as_ref()), if in_type_body { 0 } else { 15 });
        if !raw_mods.is_non_empty() && hdr_snap.as_ref().is_some_and(|h| h.target != 0) && self.is_word() && !self.is_stmt_receptor_start() {
            self.parse_headless_or_item(out, hdr_snap.unwrap().target, in_type_body); return; }
        match self.peek() { TokenKind::KwStruct => out.push(self.parse_type_decl(NodeKind::StructDecl, &mods, def_vis, false)),
            TokenKind::KwInterface => out.push(self.parse_type_decl(NodeKind::InterfaceDecl, &mods, def_vis, false)),
            TokenKind::KwFn => out.push(self.parse_fn(&mods, def_vis, false)),
            TokenKind::KwLet => self.parse_bindings(out, &mods, def_vis, false), TokenKind::KwComptime => { self.advance();
                let b = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_stmt() };
                out.push(self.arena.alloc_node(NodeKind::ComptimeExpr, 0, self.arena.span(b), b.0, 0)); }
            TokenKind::Tilde if in_type_body => out.push(self.parse_dtor(&mods)),
            TokenKind::Star if in_type_body => out.push(self.parse_bundle_splice()),
            // `ref name: T;` is a field that borrows what it holds (flags 1, as a `ref` parameter).
            TokenKind::KwRef if in_type_body => { self.advance(); let f = self.parse_field_or_prop(&mods, def_vis); self.arena.nodes[f.0 as usize].flags = 1; out.push(f); }
            _ if self.is_word() => {
                let nx = self.next_non_nl(self.cur() + 1);
                let decl_follows = nx < self.tape.len() && (is_word_tok(self.tape.kind(nx)) || self.tape.kind(nx) == TokenKind::LBrace);
                if let Some(&is_pkg) = self.tables.module_kws.get(self.text()).filter(|_| decl_follows) {
                    self.parse_module_decl(is_pkg, &mods, out); return; }
                if mods.is_ext && !self.at(TokenKind::KwFn) { out.push(self.parse_extend_decl(&mods)); return; }
                if raw_mods.flags & DECL_FLAG_MUT != 0 || raw_mods.is_const { self.parse_bindings(out, &mods, def_vis, true); return; }
                // `syntax { .. }` / `syntax <kind> { .. }`; `syntax: T` is a field named after the keyword.
                if self.is_rec(4) && self.tape.kind(self.next_non_nl(self.cur() + 1)) != TokenKind::Colon { out.push(self.parse_syntax_decl(&mods)); return; }
                if self.is_rec(7) { out.push(self.parse_extern_decl(&mods)); return; }
                if self.is_rec(26) { out.push(self.parse_type_adt(&mods, def_vis)); return; }
                if self.is_rec(27) { out.push(self.parse_enum_decl(&mods, def_vis, false)); return; }
                if self.is_rec(25) && self.op_decl_follows(in_type_body) { out.push(self.parse_op_decl(&mods, def_vis)); return; }
                if self.is_rec(20) && in_type_body { out.push(self.parse_scope_decl()); return; }
                if self.is_receiver() && in_type_body { let nx = self.next_non_nl(self.cur() + 1);
                    if nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LParen { out.push(self.parse_ctor(&mods)); return; }
                    if nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LBracket { out.push(self.parse_indexer(&mods, def_vis)); return; } }
                if self.receptor().is_some_and(|(rk, pl)| rk == 33 && matches!(pl, 1 | 21 | 22)) { out.push(self.parse_domain_decl(&mods, def_vis)); return; }
                if in_type_body || raw_mods.is_non_empty() { let nx = self.next_non_nl(self.cur() + 1);
                    if nx < self.tape.len() && matches!(self.tape.kind(nx), TokenKind::Colon | TokenKind::Question) { out.push(self.parse_field_or_prop(&mods, def_vis)); return; }
                    if let Some(h) = &hdr_snap { if h.target != 0 { self.parse_headless_or_item(out, h.target, in_type_body); return; } } }
                self.pos = start_pos; let st = self.parse_stmt(); if st.is_some() { out.push(st); } }
            _ => { self.pos = start_pos; let st = self.parse_stmt(); if st.is_some() { out.push(st); } } } }
    fn parse_headless_or_item(&mut self, out: &mut Vec<NodeId>, target: u8, in_type_body: bool) {
        let local = self.parse_mods(true);         let mods = local.merge_under(self.active_hdr.as_ref());
        let nx = self.next_non_nl(self.cur() + 1); let next_is_brace = nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LBrace;
        if !next_is_brace && matches!(self.peek(), TokenKind::KwFn | TokenKind::KwStruct | TokenKind::KwInterface | TokenKind::KwLet) {
            match self.peek() { TokenKind::KwFn => out.push(self.parse_fn(&mods, 15, false)),
                TokenKind::KwStruct => out.push(self.parse_type_decl(NodeKind::StructDecl, &mods, 15, false)),
                TokenKind::KwInterface => out.push(self.parse_type_decl(NodeKind::InterfaceDecl, &mods, 15, false)),
                _ => self.parse_bindings(out, &mods, 15, false), }
            return; }
        match target { 1 => out.push(self.parse_fn(&mods, if in_type_body { 0 } else { 15 }, true)),
            2 => out.push(self.parse_type_decl(NodeKind::StructDecl, &mods, 15, true)),
            3 => out.push(self.parse_type_decl(NodeKind::InterfaceDecl, &mods, 15, true)),
            4 => out.push(self.parse_enum_decl(&mods, 15, true)), _ => { let saved = self.active_hdr.clone();
                self.active_hdr = Some(ScopeHeader { mods, target: 0 }); self.parse_item(out, in_type_body); self.active_hdr = saved; }
        } }
    /// `<T : IA + IB, U, const N = 4>`: each parameter's name and the bound
    /// types after its `:`.
    fn parse_generic_params(&mut self) -> Vec<(SymbolId, Vec<NodeId>, NodeId)> { let mut out = Vec::new(); if !self.eat(TokenKind::Less) { return out; }
        while !self.at_end() && !self.at(TokenKind::Greater) && !self.at(TokenKind::ShiftRight) {
                        if self.is_word() && self.tables.modifiers.get(self.text()) == Some(&ModAct::Const) { self.advance(); }
            if self.is_word() || is_seed_kw(self.peek()) { out.push((self.intern_cur(), Vec::new(), NodeId::NONE)); } else { self.advance(); }
            if self.eat(TokenKind::Colon) {
                let mut bounds = vec![self.parse_type()];
                while self.eat(TokenKind::Plus) { bounds.push(self.parse_type()); }
                if let Some(last) = out.last_mut() { last.1.extend(bounds.into_iter().filter(|b| b.is_some())); }
            }
            // `<TSent = void>`: the type a use that leaves the parameter out gets.
            if self.eat(TokenKind::Equal) {
                let default = self.parse_type();
                if let Some(last) = out.last_mut() {
                    last.2 = default;
                }
            }
            if self.eat(TokenKind::Comma) {
                                if self.at(TokenKind::Greater) { self.arena.diagnostics.push((self.prev_span(), "Trailing comma not permitted in generic parameters".into())); break; }
            } else { break; } }
        if !self.eat(TokenKind::Greater) && self.at(TokenKind::ShiftRight) { let p = self.cur(); self.tape.kinds[p] = TokenKind::Greater; }
        out }
    /// Records the generic parameters and their bounds on `owner`.
    fn add_type_params(&mut self, owner: NodeId, tps: Vec<(SymbolId, Vec<NodeId>, NodeId)>) {
        for (tp, bounds, default) in tps {
            self.arena.type_params.push((owner, tp));
            if default.is_some() {
                self.arena.type_param_defaults.push((owner, tp, default));
            }
            for b in bounds { self.arena.type_param_bounds.push((owner, tp, b)); }
        }
    }
    fn parse_type_decl(&mut self, kind: NodeKind, mods: &DeclMods, def_vis: u8, headless: bool) -> NodeId {
        let sp = self.span(); if !headless { self.advance(); }
        let (name_sym, tparams, mut bases) = (self.intern_cur(), self.parse_generic_params(), Vec::new());
        if self.eat(TokenKind::Colon) { loop { let b = self.parse_type(); if b.is_some() { bases.push(b); }
                if self.eat(TokenKind::Comma) { if self.at(TokenKind::LBrace) || self.at(TokenKind::Semicolon) {
                        self.arena.diagnostics.push((self.prev_span(), "Trailing comma not permitted in inheritance list".into())); break; }
                } else { break; } } }
        while self.is_rec(28) { self.advance(); while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Semicolon) { self.advance(); } }
        let mut has_dtor = false; let body = if self.at(TokenKind::LBrace) { let bsp = self.span(); self.advance();
            self.hdr_stack.push(self.active_hdr.take()); let mut members = Vec::new(); while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.skip_seps(); if self.at_end() || self.at(TokenKind::RBrace) { break; }
                let m = self.pos; self.parse_item(&mut members, true);
                if members.last().is_some_and(|&id| self.arena.node(id).node_kind() == NodeKind::DestructorDecl) { has_dtor = true; }
                if self.pos == m { self.advance(); } }
            self.expect(TokenKind::RBrace); self.active_hdr = self.hdr_stack.pop().unwrap_or(None);
            self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(bsp.start, self.prev_span().end), &members)
        } else { let _ = self.eat(TokenKind::Semicolon); NodeId::NONE };
        let id = self.arena.alloc_node(kind, 0, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), name_sym.0, body.0);
        let mut fm = mods.clone(); if has_dtor { fm.flags |= DECL_FLAG_DTOR; }
        self.apply_mods(id, &fm, def_vis); self.add_type_params(id, tparams);
        for b in bases { self.arena.base_types.push((id, b)); }
        id }
    fn parse_extend_decl(&mut self, mods: &DeclMods) -> NodeId { let (sp, name_sym) = (self.span(), self.intern_cur());
        let _ = self.parse_generic_params();
        if self.eat(TokenKind::Colon) { while !self.at_end() && !self.at(TokenKind::LBrace) { let _ = self.parse_type(); if !self.eat(TokenKind::Comma) { break; } } }
        let body = if self.at(TokenKind::LBrace) { let bsp = self.span(); self.advance(); self.hdr_stack.push(self.active_hdr.take());
            let mut members = Vec::new(); while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
                let m = self.pos; self.parse_item(&mut members, true); if self.pos == m { self.advance(); } }
            self.expect(TokenKind::RBrace); self.active_hdr = self.hdr_stack.pop().unwrap_or(None);
            self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(bsp.start, self.prev_span().end), &members)
        } else { NodeId::NONE };
        let id = self.arena.alloc_node(NodeKind::ExtendDecl, 0, ByteSpan::new(sp.start, self.prev_span().end), name_sym.0, body.0);
        self.apply_mods(id, mods, 7); id }
    fn parse_param_list(&mut self) -> Vec<NodeId> { let mut params = Vec::new(); if !self.eat(TokenKind::LParen) { return params; }
        while !self.at_end() && !self.at(TokenKind::RParen) {
            if matches!(self.peek(), TokenKind::Ellipsis | TokenKind::DotDot | TokenKind::PrefixDot | TokenKind::InfixDot) {
                while matches!(self.peek(), TokenKind::Ellipsis | TokenKind::DotDot | TokenKind::PrefixDot | TokenKind::InfixDot) { self.advance(); }
                if self.at(TokenKind::RParen) || self.eat(TokenKind::Comma) { continue; }
            }
            let (psp, mut pflags) = (self.span(), 0u16);
            while self.eat(TokenKind::KwRef) || self.eat(TokenKind::KwComptime) || (self.is_word() && matches!(self.tables.modifiers.get(self.text()), Some(ModAct::Mut | ModAct::Iso))) {
                pflags = 1; if !matches!(self.tape.kind(self.pos.saturating_sub(1)), TokenKind::KwRef | TokenKind::KwComptime) { self.advance(); } }
            let _ = self.eat(TokenKind::Star) || self.eat(TokenKind::DoubleStar); let psym = self.intern_cur();
            let ty = if self.eat(TokenKind::Colon) { self.parse_type() } else { NodeId::NONE };
            if self.eat(TokenKind::Equal) { let _ = self.parse_expr(); }
            let pid = self.arena.alloc_node(NodeKind::ParamDecl, pflags, ByteSpan::new(psp.start, self.prev_span().end.max(psp.end)), psym.0, ty.0);
            if ty.is_some() { self.arena.set_type_annot(pid, ty); }
            params.push(pid); if self.eat(TokenKind::Comma) {
                                if self.at(TokenKind::RParen) { self.arena.diagnostics.push((self.prev_span(), "Trailing comma not permitted in parameter list".into())); break; }
            } else { break; } }
        self.expect(TokenKind::RParen); params }
    fn parse_fn(&mut self, mods: &DeclMods, def_vis: u8, headless: bool) -> NodeId { let sp = self.span(); if !headless { self.advance(); }
        let mut full_name = self.text().to_string(); self.advance(); let mut tparams = self.parse_generic_params();
        if self.eat(TokenKind::InfixDot) { full_name.push('.'); full_name.push_str(self.text()); self.advance(); tparams.extend(self.parse_generic_params()); }
        let (name_sym, params) = (self.arena.interner.intern(&full_name), self.parse_param_list());
        let ret_ty = if self.eat(TokenKind::Arrow) { self.parse_type() } else { NodeId::NONE };
        while self.is_rec(28) || self.receptor().is_some_and(|(rk, pl)| rk == 33 && matches!(pl, 18 | 19)) {
            self.advance(); while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::FatArrow) && !self.at(TokenKind::Semicolon) { self.advance(); }
        }
        let body = if self.eat(TokenKind::FatArrow) { let e = self.parse_expr(); let _ = self.eat(TokenKind::Semicolon); e }
        else if self.at(TokenKind::LBrace) { self.parse_block() } else { let _ = self.eat(TokenKind::Semicolon); NodeId::NONE };
        let mut ch = vec![body]; ch.extend(params); let (st, ln) = self.arena.alloc_child_slice(&ch);
        let id = self.arena.alloc_node(NodeKind::FnDecl, ln as u16, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), name_sym.0, st);
        if ret_ty.is_some() { self.arena.set_type_annot(id, ret_ty); }
        self.apply_mods(id, mods, def_vis); self.add_type_params(id, tparams);
        id }
    fn parse_ctor(&mut self, mods: &DeclMods) -> NodeId { let sp = self.span(); self.advance(); let params = self.parse_param_list();
        let body = if self.eat(TokenKind::FatArrow) { let e = self.parse_expr(); let _ = self.eat(TokenKind::Semicolon); e }
        else if self.at(TokenKind::LBrace) { self.parse_block() } else { let _ = self.eat(TokenKind::Semicolon); NodeId::NONE };
        let mut ch = vec![body]; ch.extend(params);
        let id = self.arena.alloc_list_node(NodeKind::ConstructorDecl, 0, ByteSpan::new(sp.start, self.prev_span().end), &ch);
        self.apply_mods(id, mods, 7); id }
    fn parse_dtor(&mut self, mods: &DeclMods) -> NodeId { let sp = self.span(); self.advance(); self.advance(); let _ = self.parse_param_list();
        let body = if self.at(TokenKind::LBrace) { self.parse_block() } else { let _ = self.eat(TokenKind::Semicolon); NodeId::NONE };
        let id = self.arena.alloc_node(NodeKind::DestructorDecl, 0, ByteSpan::new(sp.start, self.prev_span().end), body.0, 0);
        let mut fm = mods.clone(); fm.flags |= DECL_FLAG_DTOR; self.apply_mods(id, &fm, 7); id }
    fn parse_scope_decl(&mut self) -> NodeId { let sp = self.span(); self.advance(); if self.is_receiver() { self.advance(); }
        let (mut en, mut ex) = (NodeId::NONE, NodeId::NONE); if self.eat(TokenKind::LBrace) {
            while !self.at_end() && !self.at(TokenKind::RBrace) { self.skip_seps();
                if self.is_rec(21) { self.advance(); let _ = self.eat(TokenKind::Colon); en = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_stmt() }; }
                else if self.is_rec(22) { self.advance(); let _ = self.eat(TokenKind::Colon); ex = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_stmt() }; }
                else { self.advance(); } }
            self.expect(TokenKind::RBrace); }
        self.arena.alloc_node(NodeKind::ScopeDecl, 0, ByteSpan::new(sp.start, self.prev_span().end), en.0, ex.0) }
    fn parse_indexer(&mut self, mods: &DeclMods, def_vis: u8) -> NodeId {
        let sp = self.span(); self.advance(); self.expect(TokenKind::LBracket); let mut params = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBracket) { let (psp, psym) = (self.span(), self.intern_cur());
            let ty = if self.eat(TokenKind::Colon) { self.parse_type() } else { NodeId::NONE };
            params.push(self.arena.alloc_node(NodeKind::ParamDecl, 0, psp, psym.0, ty.0)); if !self.eat(TokenKind::Comma) { break; } }
        self.expect(TokenKind::RBracket); let ret = if self.eat(TokenKind::Arrow) { self.parse_type() } else { NodeId::NONE };
        let body = if self.eat(TokenKind::FatArrow) { let e = self.parse_expr(); let _ = self.eat(TokenKind::Semicolon); e }
        else if self.at(TokenKind::LBrace) { self.skip_accessor_block() } else { NodeId::NONE }; let mut ch = vec![body]; ch.extend(params);
        let id = self.arena.alloc_list_node(NodeKind::IndexerDecl, 0, ByteSpan::new(sp.start, self.prev_span().end), &ch);
        if ret.is_some() { self.arena.set_type_annot(id, ret); }
        self.apply_mods(id, mods, def_vis); id }
    fn parse_op_decl(&mut self, mods: &DeclMods, def_vis: u8) -> NodeId { let sp = self.span(); self.advance(); let mut op_txt = String::new();
        if self.eat(TokenKind::LBracket) { let _ = self.eat(TokenKind::RBracket); op_txt.push_str(if self.eat(TokenKind::Equal) { INDEX_SET_OP } else { INDEX_GET_OP }); }
        else { while !self.at_end() && !self.at(TokenKind::LParen) { op_txt.push_str(self.text()); self.advance(); } }
        let (params, ret) = (self.parse_param_list(), if self.eat(TokenKind::Arrow) { self.parse_type() } else { NodeId::NONE });
        let body = if self.eat(TokenKind::FatArrow) { let e = self.parse_expr(); let _ = self.eat(TokenKind::Semicolon); e }
        else if self.at(TokenKind::LBrace) { self.parse_block() } else { let _ = self.eat(TokenKind::Semicolon); NodeId::NONE };
        // Laid out like a function, named by the operator's text: `op ?` is
        // the method `?` of its type.
        let mut ch = vec![body]; ch.extend(params); let (st, ln) = self.arena.alloc_child_slice(&ch);
        let name = self.arena.interner.intern(&op_txt);
        let id = self.arena.alloc_node(NodeKind::OperatorDecl, ln as u16, ByteSpan::new(sp.start, self.prev_span().end), name.0, st);
        if ret.is_some() { self.arena.set_type_annot(id, ret); }
        self.apply_mods(id, mods, def_vis); id }
    fn parse_field_or_prop(&mut self, mods: &DeclMods, def_vis: u8) -> NodeId { let (sp, sym) = (self.span(), self.intern_cur());
        let _ = self.eat(TokenKind::Question); let ty = if self.eat(TokenKind::Colon) { self.parse_type() } else { NodeId::NONE };
        let (mut default, mut accessors) = (NodeId::NONE, NodeId::NONE);
        if self.at(TokenKind::LBrace) { accessors = self.skip_accessor_block(); }
        else if self.eat(TokenKind::Equal) { default = self.parse_expr(); }
        let _ = self.eat(TokenKind::Semicolon) || self.eat(TokenKind::Comma);
        let id = self.arena.alloc_node(NodeKind::ParamDecl, 0, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), sym.0, ty.0);
        if default.is_some() { self.arena.field_defaults.push((id, default)); }
        if accessors.is_some() { self.arena.property_accessors.push((id, accessors)); }
        if ty.is_some() { self.arena.set_type_annot(id, ty); }
        self.apply_mods(id, mods, def_vis); id }
    /// `*Bundle<T> coalesce { id } with { flags as net_flags } without { x };`
    /// Each listed member is kept in `splice_mods` under the receptor
    /// sub-slot of the modifier that lists it.
    fn parse_bundle_splice(&mut self) -> NodeId { let sp = self.span(); self.advance(); let ty = self.parse_type();
        let mut mods: Vec<(u8, SymbolId, SymbolId)> = Vec::new();
        while self.is_rec(30) {
            let sub = self.receptor().map_or(0, |(_, s)| s);
            self.advance();
            if !self.eat(TokenKind::LBrace) { continue; }
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                if self.is_word() {
                    let member = self.intern_cur();
                    let mut alias = SymbolId(0);
                    if self.is_rec(29) { self.advance(); if self.is_word() { alias = self.intern_cur(); } }
                    mods.push((sub, member, alias));
                } else { self.advance(); }
                let _ = self.eat(TokenKind::Comma) || self.eat(TokenKind::Semicolon);
            }
            self.expect(TokenKind::RBrace);
        }
        let _ = self.eat(TokenKind::Semicolon);
        let u = self.arena.alloc_node(NodeKind::UnaryExpr, UNARY_OP_SPLAT, ByteSpan::new(sp.start, self.prev_span().end), ty.0, 0);
        for (sub, member, alias) in mods { self.arena.splice_mods.push((u, sub, member, alias)); }
        self.arena.alloc_node(NodeKind::ExprStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), u.0, 0) }
    fn parse_type_adt(&mut self, mods: &DeclMods, def_vis: u8) -> NodeId { let sp = self.span(); self.advance();
        let (sym, tps) = (self.intern_cur(), self.parse_generic_params());
        self.expect(TokenKind::Equal); let _ = self.eat(TokenKind::Vbar); let nx = self.next_non_nl(self.cur() + 1);
        if self.is_word() && nx < self.tape.len() && matches!(self.tape.kind(nx), TokenKind::LParen | TokenKind::Vbar) {
            let mut variants = Vec::new(); loop { let (vsp, vsym) = (self.span(), self.intern_cur());
                let params = if self.at(TokenKind::LParen) { self.parse_param_list() } else { Vec::new() };
                let (st, ln) = self.arena.alloc_child_slice(&params);
                variants.push(self.arena.alloc_node(NodeKind::AdtVariant, ln as u16, ByteSpan::new(vsp.start, self.prev_span().end.max(vsp.end)), vsym.0, st));
                if !self.eat(TokenKind::Vbar) { break; } }
            let _ = self.eat(TokenKind::Semicolon); let (st, ln) = self.arena.alloc_child_slice(&variants);
            let id = self.arena.alloc_node(NodeKind::TypeAdtDecl, ln as u16, ByteSpan::new(sp.start, self.prev_span().end), sym.0, st);
            self.apply_mods(id, mods, def_vis); self.add_type_params(id, tps);
            id
        } else { let rhs = self.parse_type(); let _ = self.eat(TokenKind::Semicolon);
            let id = self.arena.alloc_node(NodeKind::TypeAdtDecl, 0, ByteSpan::new(sp.start, self.prev_span().end), sym.0, rhs.0);
            self.apply_mods(id, mods, def_vis); id } }
    fn parse_enum_decl(&mut self, mods: &DeclMods, def_vis: u8, headless: bool) -> NodeId { let sp = self.span(); if !headless { self.advance(); }
        let sym = self.intern_cur(); let _ = self.parse_generic_params(); let base = if self.eat(TokenKind::Colon) { self.parse_type() } else { NodeId::NONE };
        let mut variants = Vec::new(); if self.eat(TokenKind::LBrace) { while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
                let _ = self.parse_mods(true); let (vsp, vsym) = (self.span(), self.intern_cur());
                let params = if self.at(TokenKind::LParen) { self.parse_param_list() } else { Vec::new() };
                // `Low = 5`: the variant's value is kept as its initializer.
                let value = if self.eat(TokenKind::Equal) { self.parse_expr() } else { NodeId::NONE };
                let (st, ln) = self.arena.alloc_child_slice(&params);
                let v = self.arena.alloc_node(NodeKind::AdtVariant, ln as u16, vsp, vsym.0, st);
                if value.is_some() { self.arena.field_defaults.push((v, value)); }
                variants.push(v); let _ = self.eat(TokenKind::Comma); }
            self.expect(TokenKind::RBrace); }
        let (st, ln) = self.arena.alloc_child_slice(&variants);
        let id = self.arena.alloc_node(NodeKind::TypeAdtDecl, ln as u16, ByteSpan::new(sp.start, self.prev_span().end), sym.0, st);
        // `enum E : u8`: the type that holds the variants' values.
        if base.is_some() { self.arena.base_types.push((id, base)); }
        self.apply_mods(id, mods, def_vis); id }
    fn parse_extern_decl(&mut self, mods: &DeclMods) -> NodeId { let sp = self.span(); self.advance();
        if self.at(TokenKind::StrLit) || self.is_word() { self.advance(); }
        if self.is_rec(8) { self.advance(); if self.at(TokenKind::StrLit) || self.is_word() { self.advance(); } }
        if self.at(TokenKind::KwFn) { return self.parse_fn(mods, 7, false); }
        let mut fns = Vec::new(); if self.eat(TokenKind::LBrace) { while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
                let m = self.pos; self.parse_item(&mut fns, false); if self.pos == m { self.advance(); } }
            self.expect(TokenKind::RBrace); }
        self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), &fns) }
    fn parse_domain_decl(&mut self, mods: &DeclMods, def_vis: u8) -> NodeId {
        let (sp, (_, pl)) = (self.span(), self.receptor().unwrap_or((33, 0))); self.advance(); let sym = if pl == 21 {
            let from_t = self.parse_type(); let _ = self.eat(TokenKind::Arrow); let _ = self.parse_type();
            if from_t.is_some() { SymbolId(self.arena.node(from_t).lhs) } else { self.arena.interner.intern("_") }
        } else { let s = if self.is_word() { self.intern_cur() } else { self.arena.interner.intern("_") };
            if self.eat(TokenKind::LParen) { while !self.at_end() && !self.at(TokenKind::RParen) { self.advance(); } let _ = self.eat(TokenKind::RParen); }
            s }; let b = if self.at(TokenKind::LBrace) { self.parse_block() } else { NodeId::NONE };
        let id = self.arena.alloc_node(NodeKind::StructDecl, 0, ByteSpan::new(sp.start, self.prev_span().end), sym.0, b.0);
        self.apply_mods(id, mods, def_vis); id }
    fn parse_syntax_decl(&mut self, mods: &DeclMods) -> NodeId { let sp = self.span(); self.advance();
        let (is_tc, is_ac) = (self.is_rec(5), self.is_rec(6)); if is_tc || is_ac { self.advance(); }
        self.expect(TokenKind::LBrace); let mut rules = Vec::new(); while !self.at_end() && !self.at(TokenKind::RBrace) {
            self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
            let rsp = self.span(); if is_tc || is_ac { let mut pat_toks = Vec::new(); if is_tc && self.eat(TokenKind::LParen) {
                    let mut d = 1; while !self.at_end() && d > 0 { let k = self.peek();
                        if k == TokenKind::LParen { d += 1; } else if k == TokenKind::RParen { d -= 1; if d == 0 { self.advance(); break; } }
                        pat_toks.push((k, self.text().to_string())); self.advance(); }
                } else { let (mut pd, mut bd) = (0i32, 0i32); while !self.at_end() && !(pd == 0 && bd == 0 && self.at(TokenKind::FatArrow)) {
                        match self.peek() { TokenKind::LParen => pd += 1, TokenKind::RParen => pd -= 1, TokenKind::LBrace => bd += 1, TokenKind::RBrace => bd -= 1, _ => {} }
                        self.advance(); } }
                self.expect(TokenKind::FatArrow); let body = if self.eat(TokenKind::KwQuote) && self.eat(TokenKind::LBrace) {
                    self.in_quote_type = is_tc; let t = if is_tc { self.parse_type() } else { self.parse_expr() };
                    self.in_quote_type = false; self.expect(TokenKind::RBrace); t
                } else { self.parse_expr() }; let _ = self.eat(TokenKind::Semicolon); if is_tc && body.is_some() {
                    if let Some(rule) = compile_type_rule(&pat_toks, TemplateNode::from_arena(&self.arena, body)) {
                        let t = self.tables.to_mut(); t.type_rules.retain(|e| !same_type_trigger(e, &rule)); t.type_rules.push(rule); } }
                rules.push(body); } else { let kw = self.text().to_string(); self.advance(); let params = self.parse_param_list();
                let pnames: Vec<String> = params.iter().map(|&pid| self.arena.interner.resolve(SymbolId(self.arena.node(pid).lhs)).to_string()).collect();
                self.expect(TokenKind::FatArrow); let has_quote = self.at(TokenKind::KwQuote);
                let body = self.parse_expr(); let _ = self.eat(TokenKind::Semicolon); if has_quote && body.is_some() {
                    let tpl = TemplateNode::from_arena(&self.arena, body);
                    self.tables.to_mut().syntax_macros.insert(kw.clone(), SyntaxMacroRule { params: pnames, template: tpl }); }
                let sym = self.arena.interner.intern(&kw); let mut ch = vec![body]; ch.extend(params);
                let (st, ln) = self.arena.alloc_child_slice(&ch);
                let f = self.arena.alloc_node(NodeKind::FnDecl, ln as u16, ByteSpan::new(rsp.start, self.prev_span().end), sym.0, st);
                let mut fm = mods.clone(); fm.flags |= DECL_FLAG_COMPTIME; self.apply_mods(f, &fm, 7); rules.push(f); } }
        self.expect(TokenKind::RBrace); self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), &rules) }
    fn parse_bindings(&mut self, out: &mut Vec<NodeId>, mods: &DeclMods, def_vis: u8, kw_consumed: bool) {
        let sp = self.span(); if !kw_consumed { self.advance(); }
        let kind = if mods.is_const { NodeKind::ConstDecl } else if (mods.flags & DECL_FLAG_MUT) != 0 { NodeKind::VarStmt } else { NodeKind::LetStmt };
        let nx = self.next_non_nl(self.cur() + 1);
        if self.is_word() && nx < self.tape.len() && !matches!(self.tape.kind(nx), TokenKind::Colon | TokenKind::Equal | TokenKind::Semicolon | TokenKind::Comma) {
            let saved = self.pos; let dist_ty = self.parse_type(); if dist_ty.is_some() && self.eat(TokenKind::LBrace) {
                while !self.at_end() && !self.at(TokenKind::RBrace) { self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
                    if !self.is_word() { self.advance(); continue; }
                    let (bsp, sym) = (self.span(), self.intern_cur());
                    let init = if self.eat(TokenKind::Equal) { self.parse_expr() } else { NodeId::NONE };
                    let _ = self.eat(TokenKind::Comma) || self.eat(TokenKind::Semicolon);
                    let id = self.arena.alloc_node(kind, 0, ByteSpan::new(bsp.start, self.prev_span().end.max(bsp.end)), sym.0, init.0);
                    let cty = clone_ast(&mut self.arena, dist_ty);
                    self.arena.set_type_annot(id, cty); self.apply_mods(id, mods, def_vis); out.push(id); }
                self.expect(TokenKind::RBrace); let _ = self.eat(TokenKind::Semicolon); return; }
            self.pos = saved; if self.is_word() && nx < self.tape.len() && is_word_tok(self.tape.kind(nx)) {
                let (esp, bad) = (self.span(), self.text().to_string());
                self.arena.diagnostics.push((esp, format!("Prohibited or unregistered binding modifier '{}'", bad))); self.advance(); } }
        if matches!(self.peek(), TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace) { let pat = self.parse_pattern();
            let ty = if self.eat(TokenKind::Colon) { self.parse_type() } else { NodeId::NONE };
            let init = self.parse_binding_init();
            let _ = self.eat(TokenKind::Semicolon);
            let id = self.arena.alloc_node(kind, 1, ByteSpan::new(sp.start, self.prev_span().end), pat.0, init.0);
            if ty.is_some() { self.arena.set_type_annot(id, ty); }
            self.apply_mods(id, mods, def_vis); out.push(id); return; }
        // `let $name = ..` inside `quote { .. }` splices the name.
        if !self.is_word() && !self.at(TokenKind::Dollar) {
            let found = self.text().to_string();
            self.arena.diagnostics.push((self.span(), format!("expected a name to bind, found `{found}`")));
        }
        let sym = self.intern_cur(); let _ = self.parse_generic_params();
        let ty = if self.eat(TokenKind::Colon) { self.parse_type() } else { NodeId::NONE };
        let init = self.parse_binding_init();
        let _ = self.eat(TokenKind::Semicolon);
        let id = self.arena.alloc_node(kind, 0, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), sym.0, init.0);
        if ty.is_some() { self.arena.set_type_annot(id, ty); }
        self.apply_mods(id, mods, def_vis); out.push(id); }
    /// The `= value` of a binding, or `NONE` without one. An `=` followed
    /// by no value is an error.
    fn parse_binding_init(&mut self) -> NodeId {
        if !self.eat(TokenKind::Equal) {
            return NodeId::NONE;
        }
        if matches!(self.peek(), TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Comma) {
            let sp = self.prev_span();
            self.arena.diagnostics.push((sp, "`=` in a binding needs a value".to_string()));
            return NodeId::NONE;
        }
        self.parse_expr()
    }
    pub fn parse_type(&mut self) -> NodeId { self.parse_type_bp(0) }
    fn parse_type_bp(&mut self, min_bp: u8) -> NodeId { let sp = self.span(); let mut lhs = if self.in_quote_type && self.eat(TokenKind::Dollar) {
            let sym = self.intern_cur(); self.arena.alloc_node(NodeKind::TypeNamed, 0, sp, sym.0, u32::MAX)
        } else if matches!(self.peek(), TokenKind::Star | TokenKind::At) { let op = self.text().to_string(); self.advance();
            let inner = self.parse_type_bp(TypePrecedence::PostfixOptional as u8);
            if let Some(TypeConstructRule::Prefix { param, template, .. }) = self.tables.type_rules.iter().find(|r| matches!(r, TypeConstructRule::Prefix { op: o, .. } if o == &op)).cloned() {
                let end = self.prev_span().end; template.instantiate(&mut self.arena, &[(param, inner)], ByteSpan::new(sp.start, end))
            } else { inner }
        } else if self.eat(TokenKind::LParen) { let mut items = Vec::new(); while !self.at_end() && !self.at(TokenKind::RParen) {
                let nx = self.next_non_nl(self.cur() + 1);
                if self.is_word() && nx < self.tape.len() && self.tape.kind(nx) == TokenKind::Colon { self.advance(); self.advance(); }
                items.push(self.parse_type()); if !self.eat(TokenKind::Comma) { break; } }
            self.expect(TokenKind::RParen); if items.len() == 1 && !matches!(self.peek(), TokenKind::FatArrow | TokenKind::Arrow) { items[0] }
            else { self.arena.alloc_list_node(NodeKind::TypeTuple, 0, ByteSpan::new(sp.start, self.prev_span().end), &items) }
        } else if self.is_word() || is_seed_kw(self.peek()) { let nx = self.next_non_nl(self.cur() + 1);
            if self.is_word() && matches!(self.tables.modifiers.get(self.text()), Some(ModAct::Iso | ModAct::Mut)) && nx < self.tape.len() && (is_word_tok(self.tape.kind(nx)) || is_seed_kw(self.tape.kind(nx))) { self.advance(); }
            let mut nm = self.text().to_string(); self.advance();
            while self.eat(TokenKind::InfixDot) || self.eat(TokenKind::ColonColon) { nm.push('.'); nm.push_str(self.text()); self.advance(); }
            let (sym, mut args) = (self.arena.interner.intern(&nm), Vec::new()); if self.eat(TokenKind::Less) {
                while !self.at_end() && !self.at(TokenKind::Greater) && !self.at(TokenKind::ShiftRight) {
                    args.push(if matches!(self.peek(), TokenKind::IntLit | TokenKind::Minus) { self.parse_expr_bp(30) } else { self.parse_type() });
                    if self.eat(TokenKind::Comma) { if self.at(TokenKind::Greater) { self.arena.diagnostics.push((self.prev_span(), "Trailing comma not permitted in generic parameters".into())); break; } } else { break; } }
                if !self.eat(TokenKind::Greater) && self.at(TokenKind::ShiftRight) { let p = self.cur(); self.tape.kinds[p] = TokenKind::Greater; } }
            if let Some(TypeConstructRule::NamedGeneric { params, template, .. }) = self.tables.type_rules.iter().find(|r| matches!(r, TypeConstructRule::NamedGeneric { name, params, .. } if name == &nm && params.len() == args.len())).cloned() {
                let (b, end) = (params.into_iter().zip(args).collect::<Vec<_>>(), self.prev_span().end.max(sp.end));
                template.instantiate(&mut self.arena, &b, ByteSpan::new(sp.start, end))
            } else { self.arena.alloc_type_named(sym, &args, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end))) }
        } else { return NodeId::NONE; }; loop { if TypePrecedence::PostfixOptional as u8 >= min_bp { if self.eat(TokenKind::LBracket) {
                    let (kind, extra) = if self.eat(TokenKind::RBracket) { (TypePostfixKind::BracketEmpty, Vec::new()) }
                    else if self.eat(TokenKind::DotDot) { self.expect(TokenKind::RBracket); (TypePostfixKind::BracketDotDot, Vec::new()) }
                    else if self.eat(TokenKind::Colon) { self.expect(TokenKind::RBracket); (TypePostfixKind::BracketColon, Vec::new()) }
                    else { let mut dims = Vec::new(); while !self.at_end() && !self.at(TokenKind::RBracket) {
                            dims.push(if self.in_quote_type && self.at(TokenKind::Dollar) { self.parse_type() } else { self.parse_expr() });
                            if !self.eat(TokenKind::Comma) { break; } }
                        self.expect(TokenKind::RBracket); (TypePostfixKind::BracketArgs(dims.len()), dims) };
                    if let Some(TypeConstructRule::Postfix { params, template, .. }) = self.tables.type_rules.iter().find(|r| matches!(r, TypeConstructRule::Postfix { kind: k, .. } if match_postfix(k, &kind))).cloned() {
                        let mut b = vec![(params[0].clone(), lhs)];
                        for (i, d) in extra.into_iter().enumerate() { if let Some(pn) = params.get(i + 1) { b.push((pn.clone(), d)); } }
                        let end = self.prev_span().end; lhs = template.instantiate(&mut self.arena, &b, ByteSpan::new(sp.start, end)); }
                    continue; }
                if matches!(self.peek(), TokenKind::Question | TokenKind::Caret | TokenKind::At) { let sym = self.text().to_string();
                    if let Some(TypeConstructRule::Postfix { kind: TypePostfixKind::Sym(_, has_arg), params, template }) =
                        self.tables.type_rules.iter().find(|r| matches!(r, TypeConstructRule::Postfix { kind: TypePostfixKind::Sym(s, _), .. } if s == &sym)).cloned()
                    { self.advance(); let mut b = vec![(params[0].clone(), lhs)]; if has_arg {
                            let arg = if self.at(TokenKind::IntLit) { self.parse_expr_bp(40) } else { self.parse_type_bp(31) };
                            if let Some(p1) = params.get(1) { b.push((p1.clone(), arg)); } }
                        let end = self.prev_span().end; lhs = template.instantiate(&mut self.arena, &b, ByteSpan::new(sp.start, end));
                        continue; } } }
            let op_str = match self.peek() {
                TokenKind::Amper => "&", TokenKind::Vbar => "|", TokenKind::Arrow => "->", TokenKind::Exclamation => "!",
                TokenKind::FatArrow if self.arena.node(lhs).node_kind() == NodeKind::TypeTuple => "=>", _ => break, };
            let Some(TypeConstructRule::Infix { bp, right_assoc, lhs_param, rhs_param, template, .. }) =
                self.tables.type_rules.iter().find(|r| matches!(r, TypeConstructRule::Infix { op, .. } if op == op_str)).cloned()
            else { break; }; if bp < min_bp { break; }
            self.advance(); let rhs = self.parse_type_bp(if right_assoc { bp } else { bp + 1 });
            let lhs_clean = if self.arena.node(lhs).node_kind() == NodeKind::TypeTuple {
                let ch = self.arena.children_slice(self.arena.node(lhs).lhs, self.arena.node(lhs).rhs);
                if ch.len() == 1 { NodeId(ch[0]) } else { lhs }
            } else { lhs }; let end = self.prev_span().end;
            lhs = template.instantiate(&mut self.arena, &[(lhs_param, lhs_clean), (rhs_param, rhs)], ByteSpan::new(sp.start, end)); }
        lhs }
    fn is_stmt_receptor_start(&self) -> bool { self.receptor().is_some_and(|(rk, _)| matches!(rk, 9 | 13 | 14 | 17 | 18 | 32 | 33))
            || (self.is_word() && self.tables.syntax_macros.contains_key(self.text())) }
    fn parse_block(&mut self) -> NodeId { let sp = self.span(); self.expect(TokenKind::LBrace);
        self.hdr_stack.push(self.active_hdr.take()); let mut items = Vec::new(); while !self.at_end() && !self.at(TokenKind::RBrace) {
            self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
            let m = self.pos; self.parse_item(&mut items, false); if self.pos == m { self.advance(); } }
        self.expect(TokenKind::RBrace); self.active_hdr = self.hdr_stack.pop().unwrap_or(None);
        self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), &items) }
    fn parse_stmt(&mut self) -> NodeId { let sp = self.span(); match self.peek() {
            TokenKind::KwReturn => { self.advance(); let v = if self.at(TokenKind::Semicolon) || self.at(TokenKind::RBrace) || self.at_end() { NodeId::NONE } else { self.parse_expr() }; let _ = self.eat(TokenKind::Semicolon); self.arena.alloc_node(NodeKind::ReturnStmt, 0, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), v.0, 0) }
            TokenKind::KwYield => { self.advance(); let v = self.parse_expr(); let _ = self.eat(TokenKind::Semicolon); self.arena.alloc_node(NodeKind::YieldStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), v.0, 0) }
            TokenKind::KwDefer => { self.advance(); let b = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_stmt() }; self.arena.alloc_node(NodeKind::DeferStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), b.0, 0) }
            TokenKind::KwLoop => { self.advance(); let (tr, b) = (self.arena.alloc_node(NodeKind::BoolLit, 0, sp, 1, 0), self.parse_block()); self.arena.alloc_node(NodeKind::WhileStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), tr.0, b.0) }
            TokenKind::KwIf => self.parse_if(), TokenKind::KwAsm => { self.advance(); let bsp = self.span(); if self.at(TokenKind::LBrace) {
                    let close = self.tape.matching_bracket(self.cur()).unwrap_or(self.cur() as u32) as usize; let code = &self.src[bsp.end as usize..self.tape.span(close).start as usize]; self.pos = close + 1;
                    self.arena.alloc_polyglot_block("asm", code.trim(), 0, ByteSpan::new(sp.start, self.prev_span().end)) } else { NodeId::NONE } }
            TokenKind::RawDslBlock => { let raw = self.text(); self.advance(); let inner = raw.trim_start_matches('`').trim_end_matches('`'); let (lang, code) = inner.split_once('\n').unwrap_or(("dsl", inner)); self.arena.alloc_polyglot_block(lang.trim(), code.trim(), 0, sp) }
            _ if self.is_word() => { if let Some((rk, pl)) = self.receptor() { match rk { 9 => return self.parse_for(), 13 => { self.advance();
                            let v = if self.at(TokenKind::Semicolon) || self.at(TokenKind::RBrace) || self.at_end() { NodeId::NONE }
                            else { let first = self.parse_expr(); if self.at(TokenKind::Comma) { let mut items = vec![first]; while self.eat(TokenKind::Comma) { items.push(self.parse_expr()); } self.arena.alloc_list_node(NodeKind::TupleExpr, 0, sp, &items) } else { first } }; let _ = self.eat(TokenKind::Semicolon);
                            return self.arena.alloc_node(if pl == 0 { NodeKind::BreakStmt } else { NodeKind::ContinueStmt }, 0, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), v.0, 0); }
                        14 => return self.parse_switch(), 17 => { self.advance();
                            // `guard some x [= v] else`: flags = 1, and a binding keeps its name as `AssignStmt(x, v)`.
                            let is_some = self.receptor() == Some((31, 2)) || self.at(TokenKind::KwLet);
                            if is_some { self.advance(); }
                            self.no_struct_lit = true; let mut c = self.parse_expr();
                            if self.eat(TokenKind::Equal) { let v = self.parse_expr();
                                c = self.arena.alloc_node(NodeKind::AssignStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), c.0, v.0); }
                            self.no_struct_lit = false; if self.is_rec(3) { self.advance(); } else { self.arena.diagnostics.push((sp, "NULL002: 'guard' statement requires an 'else' block with early exit".into())); }
                            let eb = self.parse_block(); return self.arena.alloc_node(NodeKind::GuardStmt, u16::from(is_some), ByteSpan::new(sp.start, self.prev_span().end), c.0, eb.0); }
                        18 => { self.advance(); let tb = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_expr() };
                            let mut ch = vec![tb]; while self.is_rec(19) { let csp = self.span(); self.advance();
                                let psym = if self.is_word() && !self.at(TokenKind::LBrace) { let s = self.intern_cur(); if self.eat(TokenKind::Colon) { let _ = self.parse_type(); } s } else { SymbolId(0) };
                                let cb = if self.eat(TokenKind::FatArrow) { self.parse_expr() } else { self.parse_block() }; ch.push(self.arena.alloc_node(NodeKind::CatchClause, 0, csp, psym.0, cb.0)); }
                            let _ = self.eat(TokenKind::Semicolon); return self.arena.alloc_list_node(NodeKind::TryStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), &ch); }
                        32 if matches!(self.tape.kind(self.next_non_nl(self.cur() + 1)), TokenKind::LBrace | TokenKind::LParen) => return self.parse_polyglot_inline(),
                        _ => {} } }
                if self.receptor() == Some(EXCLUDE_RECEPTOR) && self.word_follows() {
                    return self.parse_exclude_decl();
                }
                let w = self.text().to_string(); let nx = self.next_non_nl(self.cur() + 1);
                if let Some(mac) = self.tables.syntax_macros.get(&w).cloned() {
                    if nx < self.tape.len() && !matches!(self.tape.kind(nx), TokenKind::Equal | TokenKind::InfixDot | TokenKind::RBrace | TokenKind::Semicolon | TokenKind::Comma | TokenKind::RParen | TokenKind::RBracket) {
                        return self.expand_syntax_macro(&mac); } }
                if nx < self.tape.len() && (is_word_tok(self.tape.kind(nx)) || matches!(self.tape.kind(nx), TokenKind::IntLit | TokenKind::StrLit | TokenKind::KwFn))
                    && !self.tables.infix_ops.contains_key(self.tape.text(nx, self.src))
                    && self.tables.receptors.get(self.tape.text(nx, self.src)).map_or(true, |&(rk, _)| !matches!(rk, 19 | 29 | 31))
                { self.arena.diagnostics.push((sp, format!("Unregistered keyword or unexpected identifier '{}'", w)));
                    while !self.at_end() && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::RBrace) {
                        if self.src[self.prev_span().end as usize..self.span().start as usize].contains('\n') { break; }
                        self.advance(); }
                    let _ = self.eat(TokenKind::Semicolon); return self.arena.alloc_node(NodeKind::ErrorNode, 0, sp, 0, 0); }
                self.parse_expr_or_assign(sp) }
            _ => self.parse_expr_or_assign(sp), } }
    fn parse_expr_or_assign(&mut self, sp: ByteSpan) -> NodeId { let e = self.parse_expr(); if e.is_none() { self.advance(); return NodeId::NONE; }
        if self.peek_assign_op().is_some() { let a = self.parse_assign_tail(e, sp, 0); let _ = self.eat(TokenKind::Semicolon); return a; }
        let _ = self.eat(TokenKind::Semicolon);
        if matches!(self.arena.node(e).node_kind(), NodeKind::IfExpr | NodeKind::ForExpr | NodeKind::MatchExpr | NodeKind::BlockExpr) { e }
        else { self.arena.alloc_node(NodeKind::ExprStmt, 0, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), e.0, 0) } }
    fn peek_assign_op(&self) -> Option<u16> { assign_op_of(self.peek()) }
    /// `target <op>= value` with the cursor on the operator; the value is parsed at `value_bp`.
    fn parse_assign_tail(&mut self, target: NodeId, sp: ByteSpan, value_bp: u8) -> NodeId { let op = self.peek_assign_op().unwrap_or(0); self.advance();
        let rhs = self.parse_expr_bp(value_bp);
        let val = if op == 0 { rhs } else { let lc = clone_ast(&mut self.arena, target); self.arena.alloc_node(NodeKind::BinaryExpr, op, sp, lc.0, rhs.0) };
        self.arena.alloc_node(NodeKind::AssignStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), target.0, val.0) }
    fn expand_syntax_macro(&mut self, mac: &SyntaxMacroRule) -> NodeId { let sp = self.span(); self.advance();
        let mut bindings = Vec::new(); let call_form = self.at(TokenKind::LParen) && {
            let close = self.tape.matching_bracket(self.cur()).unwrap_or(0) as usize; let after = self.next_non_nl(close + 1);
            !(mac.params.len() >= 2 && after < self.tape.len() && self.tape.kind(after) == TokenKind::LBrace) };
        if mac.params.is_empty() { if self.eat(TokenKind::LParen) { self.expect(TokenKind::RParen); }
        } else if call_form { self.eat(TokenKind::LParen); for p in &mac.params {
                let arg = if matches!(self.peek(), TokenKind::KwFn | TokenKind::KwStruct) { let mut v = Vec::new(); self.parse_item(&mut v, false); v.first().copied().unwrap_or(NodeId::NONE) } else { self.parse_expr() };
                bindings.push((p.clone(), arg)); let _ = self.eat(TokenKind::Comma); }
            self.expect(TokenKind::RParen); } else { for (i, p) in mac.params.iter().enumerate() {
                let arg = if i + 1 == mac.params.len() && self.at(TokenKind::LBrace) { self.parse_block() }
                else { self.no_struct_lit = true; let a = self.parse_expr(); self.no_struct_lit = false; a }; bindings.push((p.clone(), arg)); } }
        let _ = self.eat(TokenKind::Semicolon); let end = self.prev_span().end.max(sp.end);
        mac.template.instantiate(&mut self.arena, &bindings, ByteSpan::new(sp.start, end)) }
    fn parse_if(&mut self) -> NodeId { let sp = self.span(); self.advance();
        self.no_struct_lit = true; let cond = self.parse_some_condition().unwrap_or_else(|| self.parse_expr()); self.no_struct_lit = false;
        let then_b = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_expr() }; if self.is_rec(3) { self.advance();
            let else_b = if self.at(TokenKind::KwIf) { self.parse_if() } else if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_expr() };
            let (st, _) = self.arena.alloc_child_slice(&[then_b, else_b]);
            self.arena.alloc_node(NodeKind::IfExpr, 2, ByteSpan::new(sp.start, self.prev_span().end), cond.0, st)
        } else { self.arena.alloc_node(NodeKind::IfExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), cond.0, then_b.0) } }
    /// `some x = v` or `some v` as a condition (`if some mail = acct.email`).
    /// `some(..)` is a call, not a condition.
    fn parse_some_condition(&mut self) -> Option<NodeId> {
        let nx = self.next_non_nl(self.cur() + 1);
        if self.receptor() != Some((31, 2)) || nx >= self.tape.len() || self.tape.kind(nx) == TokenKind::LParen { return None; }
        let sp = self.span(); self.advance(); let mut c = self.parse_expr();
        if self.eat(TokenKind::Equal) { let v = self.parse_expr();
            c = self.arena.alloc_node(NodeKind::AssignStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), c.0, v.0); }
        Some(self.arena.alloc_node(NodeKind::UnaryExpr, pilot_ast::UNARY_OP_SOME, ByteSpan::new(sp.start, self.prev_span().end), c.0, 0)) }
    fn parse_for(&mut self) -> NodeId { let sp = self.span(); self.advance();
        let vsym = if matches!(self.peek(), TokenKind::LParen | TokenKind::LBracket) { let _ = self.parse_pattern(); self.arena.interner.intern("_item") }
        else { let s = self.intern_cur(); if self.eat(TokenKind::Comma) { self.advance(); } s }; if self.is_rec(10) { self.advance(); }
        self.no_struct_lit = true; let iter = self.parse_expr(); self.no_struct_lit = false;
        let body = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_stmt() };
        let then_n = if self.is_rec(11) { self.advance(); if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_expr() } } else { NodeId::NONE };
        self.arena.alloc_for(vsym, iter, body, then_n, ByteSpan::new(sp.start, self.prev_span().end)) }
    fn parse_switch(&mut self) -> NodeId { let sp = self.span(); self.advance();
        self.no_struct_lit = true; let scrut = self.parse_expr(); self.no_struct_lit = false; self.expect(TokenKind::LBrace);
        let mut ch = vec![scrut]; while !self.at_end() && !self.at(TokenKind::RBrace) {
            self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
            let asp = self.span(); let pat = if self.is_rec(16) || (self.is_word() && self.text() == "_") { self.advance();
                self.arena.alloc_node(NodeKind::PatWildcard, 0, asp, 0, 0)
            } else { if self.is_rec(15) { self.advance(); } self.parse_pattern() };
            let mut guard = NodeId::NONE;
            if self.eat(TokenKind::KwIf) || self.is_rec(28) {
                if self.is_rec(28) { self.advance(); }
                self.no_struct_lit = true; guard = self.parse_expr(); self.no_struct_lit = false; }
            let is_colon = self.eat(TokenKind::Colon); if !is_colon { let _ = self.eat(TokenKind::FatArrow); }
            let body = if self.at(TokenKind::LBrace) { self.parse_block() }
            else { let mut stmts = Vec::new(); while !self.at_end() && !self.at(TokenKind::RBrace) && !self.is_rec(15) && !self.is_rec(16) {
                    let m = self.pos; stmts.push(self.parse_stmt()); if self.eat(TokenKind::Comma) || self.pos == m || (!is_colon && stmts.len() == 1) { break; } }
                if stmts.len() == 1 { let s = stmts[0]; if self.arena.node(s).node_kind() == NodeKind::ExprStmt { NodeId(self.arena.node(s).lhs) } else { s } }
                else { self.arena.alloc_list_node(NodeKind::BlockExpr, 0, asp, &stmts) } };
            let _ = self.eat(TokenKind::Comma) || self.eat(TokenKind::Semicolon);
            ch.push(self.arena.alloc_match_arm(pat, guard, body, ByteSpan::new(asp.start, self.prev_span().end))); }
        self.expect(TokenKind::RBrace); self.arena.alloc_list_node(NodeKind::MatchExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), &ch) }
    /// Binding power for a literal inside a pattern: tighter than the range
    /// operators (from the Pilot operator table), so `case 4 ..= 9:` keeps the
    /// range as a pattern instead of folding it into the literal.
    fn pattern_literal_bp(&self) -> u8 {
        let is_range = |op: u16| matches!(pilot_ast::BinaryOp::from_u16(op), Some(pilot_ast::BinaryOp::RangeInclusive | pilot_ast::BinaryOp::RangeHalfOpen));
        self.tables.infix_ops.values().filter(|e| is_range(e.bin_op)).map(|e| e.bp.saturating_add(1)).max().unwrap_or(36).max(36)
    }
    fn parse_pattern(&mut self) -> NodeId { let mut lhs = self.parse_pat_atom();
        if matches!(self.peek(), TokenKind::DotDot | TokenKind::DotDotEqual | TokenKind::DotDotLess) {
            // PatRange flags: 1 = inclusive `..=`, 0 = half-open `..` / `..<`.
            let inclusive = u16::from(self.at(TokenKind::DotDotEqual));
            let sp = self.arena.span(lhs); self.advance(); let rhs = self.parse_pat_atom();
            lhs = self.arena.alloc_node(NodeKind::PatRange, inclusive, sp, lhs.0, rhs.0); }
        while self.eat(TokenKind::Vbar) { let (sp, rhs) = (self.arena.span(lhs), self.parse_pat_atom()); lhs = self.arena.alloc_node(NodeKind::PatOr, 0, sp, lhs.0, rhs.0); }
        lhs }
    fn parse_pat_atom(&mut self) -> NodeId { let sp = self.span();
        if self.is_word() && self.text() == "_" { self.advance(); return self.arena.alloc_node(NodeKind::PatWildcard, 0, sp, 0, 0); }
        if self.eat(TokenKind::LParen) { let mut ps = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RParen) { ps.push(self.parse_pattern()); if !self.eat(TokenKind::Comma) { break; } }
            self.expect(TokenKind::RParen);
            return self.arena.alloc_list_node(NodeKind::PatTuple, 0, ByteSpan::new(sp.start, self.prev_span().end), &ps); }
        // `{ "key": p, field: p, name }`: a record or map of any type.
        if self.eat(TokenKind::LBrace) { let fields = self.parse_record_pattern_fields();
            return self.arena.alloc_list_node(NodeKind::PatRecord, 0, ByteSpan::new(sp.start, self.prev_span().end), &fields); }
        if self.eat(TokenKind::LBracket) { let mut ps = Vec::new(); while !self.at_end() && !self.at(TokenKind::RBracket) {
                let is_splat = self.eat(TokenKind::Star) || self.eat(TokenKind::DotDot); let p = self.parse_pattern();
                if is_splat && self.arena.node(p).node_kind() == NodeKind::PatBind { self.arena.nodes[p.0 as usize].flags = 2; }
                ps.push(p); if !self.eat(TokenKind::Comma) { break; } }
            self.expect(TokenKind::RBracket);
            return self.arena.alloc_list_node(NodeKind::PatSlice, 0, ByteSpan::new(sp.start, self.prev_span().end), &ps); }
        if self.eat(TokenKind::ColonColon) || (self.is_word() && (self.text().starts_with(char::is_uppercase) || self.receptor() == Some((31, 2)))) {
            let mut nm = self.text().to_string(); self.advance();
            while self.eat(TokenKind::InfixDot) || self.eat(TokenKind::ColonColon) { nm.push('.'); nm.push_str(self.text()); self.advance(); }
            let (sym, mut sub) = (self.arena.interner.intern(&nm), Vec::new()); if self.eat(TokenKind::LParen) {
                while !self.at_end() && !self.at(TokenKind::RParen) { sub.push(self.parse_pattern()); if !self.eat(TokenKind::Comma) { break; } }
                self.expect(TokenKind::RParen); } else if self.eat(TokenKind::LBrace) {
                // `Name { .. }`: flags = 1 and the first child names the type.
                let mut fields = vec![self.arena.alloc_node(NodeKind::IdentExpr, 0, sp, sym.0, 0)];
                fields.extend(self.parse_record_pattern_fields());
                return self.arena.alloc_list_node(NodeKind::PatRecord, 1, ByteSpan::new(sp.start, self.prev_span().end), &fields); }
            let (st, ln) = self.arena.alloc_child_slice(&sub);
            return self.arena.alloc_node(NodeKind::PatVariant, ln as u16, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), sym.0, st); }
        if matches!(self.peek(), TokenKind::IntLit | TokenKind::HexIntLit | TokenKind::FloatLit | TokenKind::StrLit | TokenKind::Minus) || self.is_rec(1) || self.is_rec(2) {
            let lit = self.parse_expr_bp(self.pattern_literal_bp()); return self.arena.alloc_node(NodeKind::PatLiteral, 0, sp, lit.0, 0); }
        if self.is_word() { let sym = self.intern_cur(); return self.arena.alloc_node(NodeKind::PatBind, 0, sp, sym.0, 0); }
        NodeId::NONE }
    /// The fields of a record pattern after its `{`, through the `}`. A field
    /// with a sub-pattern (`key: p`, `"key": p`) is `CallArg(key, p)`; a bare
    /// `name` binds the field of that name.
    fn parse_record_pattern_fields(&mut self) -> Vec<NodeId> { let mut fields = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) { self.skip_seps(); if self.at(TokenKind::RBrace) { break; }
            let fsp = self.span(); let quoted = self.at(TokenKind::StrLit);
            let key = if quoted { let raw = self.text(); let k = raw[1..raw.len().saturating_sub(1)].to_string(); self.advance(); self.arena.interner.intern(&k) } else { self.intern_cur() };
            if self.eat(TokenKind::Colon) { let p = self.parse_pattern();
                fields.push(self.arena.alloc_node(NodeKind::CallArg, 0, ByteSpan::new(fsp.start, self.prev_span().end), key.0, p.0));
            } else { fields.push(self.arena.alloc_node(NodeKind::PatBind, 0, fsp, key.0, 0)); }
            if !self.eat(TokenKind::Comma) { self.skip_seps(); if !self.at(TokenKind::RBrace) { break; } } }
        self.expect(TokenKind::RBrace); fields }
    fn parse_polyglot_inline(&mut self) -> NodeId { let (sp, lang) = (self.span(), self.text().to_string()); self.advance();
        if self.at(TokenKind::LBrace) { let bsp = self.span();
            let close = self.tape.matching_bracket(self.cur()).unwrap_or(self.cur() as u32) as usize;
            let code = &self.src[bsp.end as usize..self.tape.span(close).start as usize]; self.pos = close + 1;
            self.arena.alloc_polyglot_block(&lang, code.trim(), 0, ByteSpan::new(sp.start, self.prev_span().end))
        } else if self.eat(TokenKind::LParen) { let e = self.parse_expr(); self.expect(TokenKind::RParen);
            let txt = if self.arena.node(e).node_kind() == NodeKind::StrLit { self.arena.interner.resolve(SymbolId(self.arena.node(e).lhs)).to_string() } else { String::new() };
            self.arena.alloc_polyglot_block(&lang, &txt, 1, ByteSpan::new(sp.start, self.prev_span().end))
        } else { NodeId::NONE } }
    pub fn parse_expr(&mut self) -> NodeId { self.parse_expr_bp(0) }
    fn parse_expr_bp(&mut self, min_bp: u8) -> NodeId { let mut lhs = self.parse_prefix(); if lhs.is_none() { return NodeId::NONE; }
        loop { let raw_p = self.cur(); if raw_p < self.tape.len() && self.tape.kind(raw_p) == TokenKind::Newline { let nx = self.next_non_nl(raw_p);
                if nx < self.tape.len() && matches!(self.tape.kind(nx), TokenKind::InfixDot | TokenKind::PrefixDot | TokenKind::SafeNav | TokenKind::PipeForward | TokenKind::SafeCascade | TokenKind::DotDot) {
                    self.pos = nx; } }
            if self.peek() == TokenKind::Less && matches!(self.arena.node(lhs).node_kind(), NodeKind::IdentExpr | NodeKind::MemberExpr) {
                if let Some(after) = self.try_skip_generic_expr_args() { self.rewrite_generic_receiver(lhs); self.pos = after; let cp = self.cur();
                    if cp < self.tape.len() && self.tape.kind(cp) == TokenKind::PrefixDot { self.tape.kinds[cp] = TokenKind::InfixDot; }
                } }
            // Read after any generic arguments: `Name<T>(..)` continues with the call.
            let k = self.peek();
            if !self.no_struct_lit && self.at(TokenKind::LBrace) && matches!(self.arena.node(lhs).node_kind(), NodeKind::IdentExpr | NodeKind::MemberExpr | NodeKind::TypeNamed) && self.looks_like_struct_lit() {
                let m = self.parse_brace_map();
                lhs = self.arena.alloc_list_node(NodeKind::CallExpr, 0, ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end), &[lhs, m]);
                continue; }
            if matches!(k, TokenKind::InfixDot | TokenKind::SafeNav | TokenKind::ColonColon) {
                let safe = (k == TokenKind::SafeNav) as u16; self.advance();
                let sym = if self.eat(TokenKind::Tilde) { let dsym = self.intern_cur(); let dnm = format!("~{}", self.arena.interner.resolve(dsym)); self.arena.interner.intern(&dnm) } else { self.intern_cur() };
                lhs = self.arena.alloc_node(NodeKind::MemberExpr, safe, ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end), lhs.0, sym.0);
                continue; }
            if k == TokenKind::LParen || matches!(k, TokenKind::CallQuoteShallow | TokenKind::CallQuoteDeep | TokenKind::CallQuoteAbi | TokenKind::CallQuoteReact | TokenKind::CallQuoteAdjoint | TokenKind::CallQuoteEffect) {
                let (pend, cstart) = (self.arena.span(lhs).end as usize, self.span().start as usize);
                if pend <= cstart && cstart <= self.src.len() && self.src[pend..cstart].contains('\n') { break; }
                lhs = self.parse_call(lhs); continue; }
            if k == TokenKind::LBracket { let (pend, cstart) = (self.arena.span(lhs).end as usize, self.span().start as usize);
                if pend <= cstart && cstart <= self.src.len() && self.src[pend..cstart].contains('\n') { break; }
                self.advance(); let idx = if self.at(TokenKind::RBracket) { NodeId::NONE } else { self.parse_expr() };
                while self.eat(TokenKind::Comma) { if self.at(TokenKind::RBracket) { break; } let _ = self.parse_expr(); }
                self.expect(TokenKind::RBracket);
                lhs = self.arena.alloc_node(NodeKind::IndexExpr, 0, ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end), lhs.0, idx.0);
                continue; }
            if k == TokenKind::Question && min_bp <= 95 {
                self.advance(); lhs = self.arena.alloc_node(NodeKind::TryPropagateExpr, 0, ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end), lhs.0, 0); continue;
            }
            if k == TokenKind::Exclamation && self.tables.receptors.contains_key("!") { let bsp = self.span(); self.advance();
                self.arena.diagnostics.push((bsp, "NULL001: Unchecked postfix `!` is prohibited; use `?`, `??`, or `guard`".into())); continue; }
            if (k == TokenKind::SafeCascade || (k == TokenKind::DotDot && self.is_cascade_dotdot(lhs))) && min_bp <= 1 {
                // A step, including the value of `..field = value`, ends at the next step.
                let safe = (k == TokenKind::SafeCascade) as u16; self.advance(); let ssp = self.span();
                let outer = std::mem::replace(&mut self.in_cascade_step, true); let mut step = self.parse_expr_bp(2);
                if step.is_some() && self.peek_assign_op().is_some() { step = self.parse_assign_tail(step, ssp, 2); }
                self.in_cascade_step = outer;
                lhs = self.arena.alloc_node(NodeKind::CascadeExpr, safe, ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end), lhs.0, step.0);
                continue; }
            // `expr catch err => fallback`: the fallback is a `CatchClause` binding `err`.
            if self.is_rec(19) && min_bp <= 6 { let csp = self.span(); self.advance(); let nx = self.next_non_nl(self.cur() + 1);
                let psym = if self.is_word() && nx < self.tape.len() && self.tape.kind(nx) == TokenKind::FatArrow { self.intern_cur() } else { SymbolId(0) };
                let _ = self.eat(TokenKind::FatArrow); let fb = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_expr_bp(6) };
                let clause = self.arena.alloc_node(NodeKind::CatchClause, 0, ByteSpan::new(csp.start, self.prev_span().end), psym.0, fb.0);
                lhs = self.arena.alloc_node(NodeKind::CatchRecoveryExpr, 0, ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end), lhs.0, clause.0);
                continue; }
            // `x as T`: its binding power is the one Pilot registers for the word.
            if self.receptor() == Some((29, 0)) { let Some(bp) = self.tables.infix_ops.get(self.text()).map(|e| e.bp) else { break; }; if bp < min_bp { break; } self.advance();
                let is_view = self.receptor() == Some((29, 1)); if is_view { self.advance(); }
                let (ty, op) = (self.parse_type_bp(30), if is_view { BinaryOp::AsView } else { BinaryOp::As } as u16);
                lhs = self.arena.alloc_node(NodeKind::BinaryExpr, op, ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end), lhs.0, ty.0);
                continue; }
            if k == TokenKind::DotDot && self.in_cascade_step && self.cascade_step_ahead() { break; }
            let op_entry = if let Some((31, code)) = self.receptor() {
                (code != 1 && code != 2).then_some(PrattEntry { bp: if code == 15 { 12 } else if code == 14 { 14 } else { 45 }, right_assoc: false, bin_op: code as u16 })
            } else { self.tables.infix_ops.get(self.text()).copied() }; let Some(pe) = op_entry else { break; }; if pe.bp < min_bp { break; }
            self.advance(); let rhs = self.parse_expr_bp(if pe.right_assoc { pe.bp } else { pe.bp + 1 });
            if matches!(pe.bin_op, 23 | 24) && self.eat(TokenKind::Colon) { let _ = self.parse_expr_bp(pe.bp + 1); }
            lhs = self.arena.alloc_node(NodeKind::BinaryExpr, pe.bin_op, ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end), lhs.0, rhs.0);
        }
        lhs }
    fn parse_prefix(&mut self) -> NodeId { let sp = self.span(); match self.peek() {
            // `yield e` in an expression: its value is what the next
            // `send(v)` passes in.
            TokenKind::KwYield => {
                self.advance();
                let v = self.parse_expr();
                let span = ByteSpan::new(sp.start, self.prev_span().end);
                self.arena.alloc_node(NodeKind::YieldStmt, 0, span, v.0, 0)
            }
            TokenKind::IntLit | TokenKind::FloatLit if self.tape.flag(self.cur()).contains(pilot_lexer::TokenFlags::IS_DIMENSIONAL) => {
                let sym = self.intern_cur(); self.arena.alloc_node(NodeKind::UnitLit, 1, sp, sym.0, 0) }
            TokenKind::IntLit => { let t = self.text().replace('_', ""); let v = t.parse::<i64>().or_else(|_| t.parse::<u64>().map(|u| u as i64)).unwrap_or(0); self.advance(); self.arena.alloc_int(v, sp) }
            TokenKind::HexIntLit => { let v = u64::from_str_radix(&self.text()[2..].replace('_', ""), 16).map_or(0, |v| v as i64); self.advance(); self.arena.alloc_int(v, sp) }
            TokenKind::BinIntLit => { let v = u64::from_str_radix(&self.text()[2..].replace('_', ""), 2).map_or(0, |v| v as i64); self.advance(); self.arena.alloc_int(v, sp) }
            TokenKind::OctIntLit => { let v = u64::from_str_radix(&self.text()[2..].replace('_', ""), 8).map_or(0, |v| v as i64); self.advance(); self.arena.alloc_int(v, sp) }
            TokenKind::FloatLit => { let v = self.text().replace('_', "").parse().unwrap_or(0.0); self.advance(); self.arena.alloc_float(v, sp) }
            TokenKind::StrLit | TokenKind::TripleStrLit => { let raw = self.text(); let is_f = raw.starts_with('f') || raw.starts_with('F');
                let s = if is_f { &raw[1..] } else { raw };
                let inner = if (s.starts_with("\"\"\"") || s.starts_with("'''")) && s.len() >= 6 { &s[3..s.len() - 3] }
                else if s.len() >= 2 { &s[1..s.len() - 1] } else { s }; self.advance();
                if is_f { return self.lower_fstring(inner, sp); }
                let unesc = unescape_str(inner);
                let sym = self.arena.interner.intern(&unesc); self.arena.alloc_node(NodeKind::StrLit, 0, sp, sym.0, 0) }
            TokenKind::KeywordLit => { let t = self.text(); self.advance(); self.arena.alloc_keyword(t, sp) }
            TokenKind::KwIf => self.parse_if(),
            TokenKind::KwQuote => { self.advance(); if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_expr() } }
            // `comptime expr` / `comptime { .. }` as a value: flags = 1.
            TokenKind::KwComptime => { self.advance(); let e = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_expr() };
                self.arena.alloc_node(NodeKind::ComptimeExpr, 1, ByteSpan::new(sp.start, self.prev_span().end), e.0, 0) }
            TokenKind::Dollar => { self.advance(); if self.eat(TokenKind::LParen) { let e = self.parse_expr(); self.expect(TokenKind::RParen); e }
                else { let sym = self.intern_cur(); self.arena.alloc_node(NodeKind::IdentExpr, 1, sp, sym.0, 0) } }
            TokenKind::PrefixDot => { self.advance();
                let rcv_nm = if self.tables.receiver_ident.is_empty() { "_self".to_string() } else { self.tables.receiver_ident.clone() };
                let (rcv, fld) = (self.arena.alloc_ident(&rcv_nm, sp), self.intern_cur());
                self.arena.alloc_node(NodeKind::MemberExpr, MEMBER_IMPLIED_RECEIVER, ByteSpan::new(sp.start, self.prev_span().end), rcv.0, fld.0) }
            TokenKind::ColonColon => { self.advance(); let sym = self.intern_cur(); self.arena.alloc_node(NodeKind::IdentExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), sym.0, 0) }
            TokenKind::Minus | TokenKind::Plus | TokenKind::Exclamation | TokenKind::CallQuoteShallow | TokenKind::Tilde | TokenKind::Amper | TokenKind::KwRef | TokenKind::ArrowLeft
            | TokenKind::Star | TokenKind::DoubleStar | TokenKind::StarDotDot | TokenKind::GreaterDotDot => { let (fl, bp) = match self.peek() {
                    TokenKind::Amper | TokenKind::KwRef | TokenKind::Plus => (1u16, 80u8), TokenKind::ArrowLeft => (3, 80), TokenKind::Tilde => (4, 80),
                    TokenKind::Exclamation | TokenKind::CallQuoteShallow => (5, 80), TokenKind::Star => (UNARY_OP_SPLAT, 90), TokenKind::DoubleStar => (UNARY_OP_DOUBLE_SPLAT, 90),
                    TokenKind::StarDotDot => (UNARY_OP_DEEP_FLATTEN, 90), _ => (0, 80), }; self.advance(); let v = self.parse_expr_bp(bp);
                self.arena.alloc_node(NodeKind::UnaryExpr, fl, ByteSpan::new(sp.start, self.prev_span().end), v.0, 0) }
            TokenKind::LParen => self.parse_paren_or_lambda(), TokenKind::LBracket => self.parse_bracket_lit(),
            TokenKind::LBrace => if self.looks_like_brace_map() { self.parse_brace_map() } else { self.parse_block() },
            _ if self.is_word() => { let nx = self.next_non_nl(self.cur() + 1);
                if self.tables.modifiers.get(self.text()) == Some(&ModAct::Mut) && nx < self.tape.len() && (is_word_tok(self.tape.kind(nx)) || matches!(self.tape.kind(nx), TokenKind::KwRef | TokenKind::Amper)) {
                    self.advance(); return self.parse_prefix(); }
                if let Some((rk, pl)) = self.receptor() { match rk {
                        1 => { self.advance(); return self.arena.alloc_node(NodeKind::BoolLit, 0, sp, pl as u32, 0); }
                        2 => { self.advance(); return self.arena.alloc_node(NodeKind::NoneLit, 0, sp, 0, 0); }
                        9 => return self.parse_for(), 14 => return self.parse_switch(), 18 => { self.advance(); return self.parse_expr_bp(6); }
                        31 if pl == 1 => { self.advance(); let v = self.parse_expr_bp(16); return self.arena.alloc_node(NodeKind::UnaryExpr, 5, sp, v.0, 0); }
                        31 if pl == 2 => { self.advance(); return self.parse_expr_bp(80); }
                        32 if nx < self.tape.len() && (matches!(self.tape.kind(nx), TokenKind::LParen) || (!self.no_struct_lit && matches!(self.tape.kind(nx), TokenKind::LBrace))) => return self.parse_polyglot_inline(),
                        33 if pl == 6 => { let p2 = self.next_non_nl(nx + 1);
                            if p2 < self.tape.len() && is_word_tok(self.tape.kind(nx)) && self.tables.receptors.get(self.tape.text(p2, self.src)) == Some(&(10, 0)) {
                                return self.parse_linq_query(); } }
                        33 if matches!(pl, 17 | 23) && nx < self.tape.len() && !matches!(self.tape.kind(nx), TokenKind::RParen | TokenKind::Comma | TokenKind::Semicolon | TokenKind::InfixDot) => {
                            self.advance(); return self.parse_expr_bp(80); }
                        33 if pl == 7 && nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LBrace => { self.advance(); return self.parse_block(); }
                        35 if nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LParen => {
                            self.advance(); self.expect(TokenKind::LParen); let arg = self.parse_expr(); self.expect(TokenKind::RParen);
                            let fl = match pl { 1 => UNARY_OP_SPLAT, 2 => UNARY_OP_DOUBLE_SPLAT, 3 => UNARY_OP_DEEP_FLATTEN, _ => UNARY_OP_SPLAT };
                            return self.arena.alloc_node(NodeKind::UnaryExpr, fl, ByteSpan::new(sp.start, self.prev_span().end), arg.0, 0); }
                        37 if pl >= 1 && pl <= 6 => {
                            if nx < self.tape.len() && (is_word_tok(self.tape.kind(nx)) || is_seed_kw(self.tape.kind(nx))) {
                                let mut p_check = nx; while p_check < self.tape.len() && (is_word_tok(self.tape.kind(p_check)) || is_seed_kw(self.tape.kind(p_check)) || self.tape.kind(p_check) == TokenKind::InfixDot) { p_check += 1; }
                                if p_check < self.tape.len() && self.tape.kind(p_check) == TokenKind::LParen && p_check > nx {
                                    self.advance(); let mut callee = self.parse_prefix();
                                    if callee.is_some() {
                                        while matches!(self.peek(), TokenKind::InfixDot | TokenKind::SafeNav) {
                                            let safe = (self.peek() == TokenKind::SafeNav) as u16; self.advance(); let sym = self.intern_cur();
                                            callee = self.arena.alloc_node(NodeKind::MemberExpr, safe, ByteSpan::new(self.arena.span(callee).start, self.prev_span().end), callee.0, sym.0); }
                                        return self.parse_call_args(callee, pl as u16); } } } }
                        _ => {} } }
                let w = self.text().to_string(); if let Some(mac) = self.tables.syntax_macros.get(&w).cloned() {
                    // In the head of `for`, `if` or `switch` a `{` opens the body, so
                    // a binding named like a macro (`for x in set { .. }`) stays a name.
                    if nx < self.tape.len() && (self.tape.kind(nx) == TokenKind::LParen || (!self.no_struct_lit && self.tape.kind(nx) == TokenKind::LBrace)) { return self.expand_syntax_macro(&mac); }
                }
                self.advance(); self.arena.alloc_ident(&w, sp) }
            _ if is_seed_kw(self.peek()) => { let w = self.text(); self.advance(); self.arena.alloc_ident(w, sp) }
            _ => NodeId::NONE, } }
    fn parse_call(&mut self, callee: NodeId) -> NodeId { let qk = match self.peek() {
            TokenKind::CallQuoteShallow => CallQuoteKind::ShallowQuote, TokenKind::CallQuoteDeep => CallQuoteKind::DeepSplice,
            TokenKind::CallQuoteAbi => CallQuoteKind::AbiFrame, TokenKind::CallQuoteReact => CallQuoteKind::ReactiveSignal,
            TokenKind::CallQuoteAdjoint => CallQuoteKind::SymbolicAdjoint, TokenKind::CallQuoteEffect => CallQuoteKind::EffectDryRun,
            _ => CallQuoteKind::None, }; if qk != CallQuoteKind::None { self.advance(); }
        self.parse_call_args(callee, qk as u16) }
    fn parse_call_args(&mut self, callee: NodeId, qk: u16) -> NodeId {
        let is_sym_quote = self.arena.node(callee).node_kind() == NodeKind::IdentExpr && self.tables.quotes_symbol(self.arena.interner.resolve(SymbolId(self.arena.node(callee).lhs)));
        self.expect(TokenKind::LParen); let mut ch = vec![callee]; while !self.at_end() && !self.at(TokenKind::RParen) {
            if is_sym_quote { let (tsp, mut txt) = (self.span(), String::new());
                while !self.at_end() && !self.at(TokenKind::RParen) && !self.at(TokenKind::Comma) { txt.push_str(self.text()); self.advance(); }
                let sym = self.arena.interner.intern(&txt); ch.push(self.arena.alloc_node(NodeKind::StrLit, 0, tsp, sym.0, 0));
            } else { let nx = self.next_non_nl(self.cur() + 1);
                // `name: value` keeps its name in a `CallArg` (lhs = name, rhs = value).
                if self.is_word() && nx < self.tape.len() && matches!(self.tape.kind(nx), TokenKind::Equal | TokenKind::Colon) {
                    let nsp = self.span(); let name = self.intern_cur(); self.advance(); let a = self.parse_expr();
                    if a.is_some() { ch.push(self.arena.alloc_node(NodeKind::CallArg, 0, ByteSpan::new(nsp.start, self.prev_span().end), name.0, a.0)); }
                } else { let a = self.parse_expr(); if a.is_some() { ch.push(a); } } }
            if self.eat(TokenKind::Comma) { if self.at(TokenKind::RParen) { self.arena.diagnostics.push((self.prev_span(), "Trailing comma not permitted in argument list".into())); break; } } else { break; } }
        self.expect(TokenKind::RParen); self.arena.alloc_list_node(NodeKind::CallExpr, qk, ByteSpan::new(self.arena.span(callee).start, self.prev_span().end), &ch) }
    /// Lowers the body of `f"..."` into string concatenation: literal
    /// segments become string literals and every `{expr}` hole is parsed as an
    /// expression. `{{` and `}}` stand for literal braces. The chain starts
    /// with a string so `+` concatenates instead of adding numbers.
    fn lower_fstring(&mut self, body: &'a str, sp: ByteSpan) -> NodeId {
        let bytes = body.as_bytes();
        let mut parts: Vec<NodeId> = Vec::new();
        let mut lit = String::new();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i];
            if (c == b'{' || c == b'}') && bytes.get(i + 1) == Some(&c) {
                lit.push(c as char);
                i += 2;
                continue;
            }
            if c == b'}' {
                self.arena.diagnostics.push((sp, "SyntaxError: unmatched `}` in f-string; write `}}` for a literal brace".into()));
                i += 1;
                continue;
            }
            if c != b'{' {
                let ch = body[i..].chars().next().unwrap_or('?');
                lit.push(ch);
                i += ch.len_utf8();
                continue;
            }
            let Some(end) = fstring_hole_end(bytes, i + 1) else {
                self.arena.diagnostics.push((sp, "SyntaxError: unterminated `{` in f-string".into()));
                break;
            };
            if !lit.is_empty() {
                let sym = self.arena.interner.intern(&unescape_str(&lit));
                parts.push(self.arena.alloc_node(NodeKind::StrLit, 0, sp, sym.0, 0));
                lit.clear();
            }
            let hole = &body[i + 1..end];
            let offset = (hole.as_ptr() as usize).saturating_sub(self.src.as_ptr() as usize) as u32;
            parts.push(self.parse_fstring_hole(unescape_hole_quotes(hole), offset, sp));
            i = end + 1;
        }
        if !lit.is_empty() || parts.is_empty() {
            let sym = self.arena.interner.intern(&unescape_str(&lit));
            parts.push(self.arena.alloc_node(NodeKind::StrLit, 0, sp, sym.0, 0));
        }
        let first_is_str = self.arena.node(parts[0]).node_kind() == NodeKind::StrLit;
        let mut acc = if first_is_str { parts[0] } else {
            let empty = self.arena.interner.intern("");
            let e = self.arena.alloc_node(NodeKind::StrLit, 0, sp, empty.0, 0);
            self.arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::Add as u16, sp, e.0, parts[0].0)
        };
        for &p in &parts[1..] {
            acc = self.arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::Add as u16, sp, acc.0, p.0);
        }
        acc
    }

    /// Parses one `{expr}` hole of an f-string with a temporary token tape.
    /// Spans of the hole's nodes are shifted back into the enclosing source.
    /// `offset` is where the hole starts in the enclosing source.
    fn parse_fstring_hole(&mut self, hole: &'a str, offset: u32, sp: ByteSpan) -> NodeId {
        let first_node = self.arena.nodes.len();
        let first_diag = self.arena.diagnostics.len();
        let saved_tape = std::mem::replace(&mut self.tape, tokenize(hole));
        let saved_src = std::mem::replace(&mut self.src, hole);
        let saved_pos = std::mem::replace(&mut self.pos, 0);
        let e = if self.at_end() { NodeId::NONE } else { self.parse_expr() };
        let trailing = !self.at_end();
        self.tape = saved_tape;
        self.src = saved_src;
        self.pos = saved_pos;
        for span in &mut self.arena.spans[first_node..] {
            *span = ByteSpan::new(span.start + offset, span.end + offset);
        }
        for d in &mut self.arena.diagnostics[first_diag..] {
            d.0 = ByteSpan::new(d.0.start + offset, d.0.end + offset);
        }
        if e.is_none() {
            self.arena.diagnostics.push((sp, "SyntaxError: empty `{}` in f-string".into()));
        } else if trailing {
            self.arena.diagnostics.push((sp, "SyntaxError: f-string hole must be a single expression (format specifiers are not supported yet)".into()));
        }
        e
    }

    fn parse_paren_or_lambda(&mut self) -> NodeId { let sp = self.span(); if let Some(close) = self.tape.matching_bracket(self.cur()) {
            let nx = self.next_non_nl(close as usize + 1); if nx < self.tape.len() && self.tape.kind(nx) == TokenKind::FatArrow {
                let params = self.parse_param_list(); self.expect(TokenKind::FatArrow);
                let body = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_expr() };
                let (st, ln) = self.arena.alloc_child_slice(&params);
                return self.arena.alloc_node(NodeKind::LambdaExpr, ln as u16, ByteSpan::new(sp.start, self.prev_span().end), st, body.0); } }
        self.advance(); let (mut items, mut had_comma) = (Vec::new(), false); while !self.at_end() && !self.at(TokenKind::RParen) {
            let e = self.parse_expr(); if e.is_some() { items.push(e); }
            if self.eat(TokenKind::Comma) { had_comma = true; } else { break; } }
        self.expect(TokenKind::RParen);
        if items.len() == 1 && !had_comma { items[0] } else { self.arena.alloc_list_node(NodeKind::TupleExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), &items) } }
    fn parse_bracket_lit(&mut self) -> NodeId {
        let sp = self.span(); self.advance(); if self.eat(TokenKind::Colon) { self.expect(TokenKind::RBracket); return self.arena.alloc_list_node(NodeKind::MapExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), &[]); }
        let (mut items, mut is_map) = (Vec::new(), false); while !self.at_end() && !self.at(TokenKind::RBracket) {
            if self.at(TokenKind::RBracket) { break; }
            let mp = self.pos; let k = self.parse_expr();
            if self.eat(TokenKind::Colon) { is_map = true; let v = self.parse_expr(); if k.is_some() { items.push(k); } if v.is_some() { items.push(v); } }
            else if k.is_some() { items.push(k); }
            if self.eat(TokenKind::Comma) { } else if self.pos == mp { self.advance(); } else { break; } }
        self.expect(TokenKind::RBracket);
        self.arena.alloc_list_node(if is_map { NodeKind::MapExpr } else { NodeKind::ListExpr }, 0, ByteSpan::new(sp.start, self.prev_span().end), &items) }
    fn looks_like_brace_map(&self) -> bool { let p1 = self.next_non_nl(self.cur() + 1); if p1 >= self.tape.len() { return false; }
        if self.tape.kind(p1) == TokenKind::RBrace { return true; }
        let p2 = self.next_non_nl(p1 + 1);
        if p2 < self.tape.len() && (is_word_tok(self.tape.kind(p1)) || matches!(self.tape.kind(p1), TokenKind::StrLit | TokenKind::IntLit))
            && matches!(self.tape.kind(p2), TokenKind::Colon | TokenKind::Equal)
        { let w = self.tape.text(p1, self.src); !matches!(self.tables.receptors.get(w), Some((15 | 16 | 21 | 22, _)))
        } else { false } }
    fn looks_like_struct_lit(&self) -> bool { let p1 = self.next_non_nl(self.cur() + 1);
        if p1 < self.tape.len() && self.tape.kind(p1) == TokenKind::RBrace { return true; }
        if !self.looks_like_brace_map() { return false; }
        let Some(close) = self.tape.matching_bracket(self.cur()) else { return false; }; let mut d = 0;
        for i in (self.cur() + 1)..(close as usize) { match self.tape.kind(i) {
                TokenKind::LBrace | TokenKind::LParen | TokenKind::LBracket => d += 1,
                TokenKind::RBrace | TokenKind::RParen | TokenKind::RBracket => d -= 1, TokenKind::Semicolon if d == 0 => return false, _ => {} } }
        true }
    fn parse_brace_map(&mut self) -> NodeId { let sp = self.span(); self.advance(); let mut pairs = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) { self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
            let mp = self.pos; let nx = self.next_non_nl(self.cur() + 1);
            let k = if self.is_word() && nx < self.tape.len() && matches!(self.tape.kind(nx), TokenKind::Colon | TokenKind::Equal) {
                let (ksp, nm) = (self.span(), self.text().to_string()); self.advance(); self.arena.alloc_ident(&nm, ksp)
            } else { self.parse_expr() }; if self.eat(TokenKind::Colon) || self.eat(TokenKind::Equal) {
                let v = self.parse_expr(); if k.is_some() { pairs.push(k); } if v.is_some() { pairs.push(v); }
            } else if k.is_some() { pairs.push(k); }
            let _ = self.eat(TokenKind::Comma) || self.eat(TokenKind::Semicolon); if self.pos == mp { self.advance(); } }
        self.expect(TokenKind::RBrace); self.arena.alloc_list_node(NodeKind::MapExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), &pairs) }
    fn parse_linq_query(&mut self) -> NodeId { let sp = self.span(); self.advance(); let vsym = self.intern_cur();
        if self.is_rec(10) { self.advance(); }
        let src_e = self.parse_expr_bp(20); while !self.at_end() && self.receptor() != Some((33, 7)) {
            if self.is_rec(28) { self.advance(); let _ = self.parse_expr_bp(20); } else { self.advance(); } }
        let proj = if self.receptor() == Some((33, 7)) { self.advance(); self.parse_expr_bp(15) } else { NodeId::NONE };
        self.arena.alloc_for(vsym, src_e, proj, NodeId::NONE, ByteSpan::new(sp.start, self.prev_span().end)) }
    fn try_skip_generic_expr_args(&self) -> Option<usize> { let (mut p, mut d) = (self.cur(), 0i32); while p < self.tape.len() {
            match self.tape.kind(p) { TokenKind::Less => d += 1, TokenKind::Greater => { d -= 1; if d == 0 { break; } }
                TokenKind::ShiftRight => { d -= 2; if d <= 0 { break; } }
                TokenKind::Comma | TokenKind::LBracket | TokenKind::RBracket | TokenKind::LParen | TokenKind::RParen | TokenKind::Star
                | TokenKind::Question | TokenKind::Exclamation | TokenKind::Vbar | TokenKind::Amper | TokenKind::Arrow | TokenKind::FatArrow | TokenKind::DotDot | TokenKind::IntLit => {}
                tk if is_word_tok(tk) || is_seed_kw(tk) => {}
                _ => return None, }
            p += 1; }
        let after = self.next_non_nl(p + 1);
        (d == 0 && after < self.tape.len() && matches!(self.tape.kind(after), TokenKind::LBrace | TokenKind::InfixDot | TokenKind::PrefixDot | TokenKind::LParen)).then_some(after)
    }
    /// Whether the `..` at the cursor starts a cascade step rather than a range.
    /// A numeric lower bound is always a range. `..name(` and `..name =` are
    /// cascade steps; `..a.b` is one only when the member chain is assigned
    /// (`..a.b = v`), so `0..xs.len()` stays a range. A range bounded by a
    /// plain function call needs parentheses: `i..(f(x))`.
    fn is_cascade_dotdot(&self, lhs: NodeId) -> bool {
        !matches!(self.arena.node(lhs).node_kind(), NodeKind::IntLit | NodeKind::FloatLit) && self.cascade_step_ahead() }
    /// Whether the tokens after the `..` at the cursor have the shape of a cascade step.
    fn cascade_step_ahead(&self) -> bool {
        let mut nx = self.next_non_nl(self.cur() + 1);
        if nx >= self.tape.len() || !is_word_tok(self.tape.kind(nx)) { return false; }
        let mut after = self.next_non_nl(nx + 1); let mut chained = false;
        while after < self.tape.len() && self.tape.kind(after) == TokenKind::InfixDot {
            nx = self.next_non_nl(after + 1);
            if nx >= self.tape.len() || !is_word_tok(self.tape.kind(nx)) { return false; }
            after = self.next_non_nl(nx + 1); chained = true; }
        if after >= self.tape.len() { return false; }
        let k = self.tape.kind(after);
        (k == TokenKind::LParen && !chained) || assign_op_of(k).is_some() } }
/// The `BinaryOp` an assignment token stores: 0 for `=`, the combined
/// operator for `+=` and the other compound forms.
fn assign_op_of(k: TokenKind) -> Option<u16> { match k { TokenKind::Equal => Some(0u16), TokenKind::PlusEqual => Some(1), TokenKind::MinusEqual => Some(2),
        TokenKind::StarEqual => Some(3), TokenKind::SlashEqual => Some(4), TokenKind::PercentEqual => Some(6),
        TokenKind::AmperEqual => Some(16), TokenKind::VbarEqual => Some(17), TokenKind::CaretEqual => Some(18),
        TokenKind::ShiftLeftEqual => Some(19), TokenKind::ShiftRightEqual => Some(20), TokenKind::CoalesceEqual => Some(22),
        _ => None, } }
fn match_postfix(a: &TypePostfixKind, b: &TypePostfixKind) -> bool { matches!((a, b), (TypePostfixKind::BracketEmpty, TypePostfixKind::BracketEmpty)
        | (TypePostfixKind::BracketDotDot, TypePostfixKind::BracketDotDot)
        | (TypePostfixKind::BracketColon, TypePostfixKind::BracketColon))
        || matches!((a, b), (TypePostfixKind::BracketArgs(x), TypePostfixKind::BracketArgs(y)) if x == y) }
fn same_type_trigger(a: &TypeConstructRule, b: &TypeConstructRule) -> bool { match (a, b) {
        (TypeConstructRule::NamedGeneric { name: x, .. }, TypeConstructRule::NamedGeneric { name: y, .. }) | (TypeConstructRule::Prefix { op: x, .. }, TypeConstructRule::Prefix { op: y, .. }) | (TypeConstructRule::Infix { op: x, .. }, TypeConstructRule::Infix { op: y, .. }) => x == y,
        (TypeConstructRule::Postfix { kind: TypePostfixKind::BracketArgs(x), .. }, TypeConstructRule::Postfix { kind: TypePostfixKind::BracketArgs(y), .. }) => x == y,
        (TypeConstructRule::Postfix { kind: TypePostfixKind::Sym(x, _), .. }, TypeConstructRule::Postfix { kind: TypePostfixKind::Sym(y, _), .. }) => x == y,
        (TypeConstructRule::Postfix { kind: x, .. }, TypeConstructRule::Postfix { kind: y, .. }) => core::mem::discriminant(x) == core::mem::discriminant(y),
        _ => false, } }
fn compile_type_rule(pat: &[(TokenKind, String)], template: TemplateNode) -> Option<TypeConstructRule> { if pat.is_empty() { return None; }
    if pat.len() >= 4 && is_word_tok(pat[0].0) && pat[1].0 == TokenKind::Less && pat.last().map(|x| x.0) == Some(TokenKind::Greater) {
        let (name, mut params) = (pat[0].1.clone(), Vec::new());
        for (tk, s) in &pat[2..pat.len() - 1] { if is_word_tok(*tk) { params.push(s.clone()); } }
        return Some(TypeConstructRule::NamedGeneric { name, params, template }); }
    if pat.len() == 2 && matches!(pat[0].0, TokenKind::Star | TokenKind::At) {
        return Some(TypeConstructRule::Prefix { op: pat[0].1.clone(), param: pat[1].1.clone(), template }); }
    if pat.len() >= 3 && pat[1].0 == TokenKind::LBracket { let t = pat[0].1.clone();
        if pat[2].0 == TokenKind::RBracket { return Some(TypeConstructRule::Postfix { kind: TypePostfixKind::BracketEmpty, params: vec![t], template }); }
        if pat[2].0 == TokenKind::DotDot { return Some(TypeConstructRule::Postfix { kind: TypePostfixKind::BracketDotDot, params: vec![t], template }); }
        if pat[2].0 == TokenKind::Colon { return Some(TypeConstructRule::Postfix { kind: TypePostfixKind::BracketColon, params: vec![t], template }); }
        let mut ps = vec![t]; for (tk, s) in &pat[2..] { if is_word_tok(*tk) { ps.push(s.clone()); } }
        return Some(TypeConstructRule::Postfix { kind: TypePostfixKind::BracketArgs(ps.len() - 1), params: ps, template }); }
    if pat.len() == 2 && matches!(pat[1].0, TokenKind::Question | TokenKind::Caret) {
        return Some(TypeConstructRule::Postfix { kind: TypePostfixKind::Sym(pat[1].1.clone(), false), params: vec![pat[0].1.clone()], template }); }
    if pat.len() == 3 && pat[1].0 == TokenKind::At {
        return Some(TypeConstructRule::Postfix { kind: TypePostfixKind::Sym("@".into(), true), params: vec![pat[0].1.clone(), pat[2].1.clone()], template }); }
    let clean: Vec<&(TokenKind, String)> = pat.iter().filter(|(k, _)| !matches!(k, TokenKind::LParen | TokenKind::RParen)).collect();
    if clean.len() == 3 { let (bp, ra) = match clean[1].0 { TokenKind::Amper => (20, false), TokenKind::Vbar => (15, false),
            TokenKind::FatArrow | TokenKind::Arrow => (10, true), TokenKind::Exclamation => (5, false), _ => return None, };
        return Some(TypeConstructRule::Infix { op: clean[1].1.clone(), bp, right_assoc: ra, lhs_param: clean[0].1.clone(), rhs_param: clean[2].1.clone(), template });
    }
    None }
/// Index of the `}` closing an f-string hole that starts at `start`, skipping
/// nested braces and quoted strings inside the hole.
/// Marks a string inside a hole delimited by escaped quotes (`\"`), the only
/// way to write a double-quoted string inside `f"..."`.
const ESCAPED_QUOTE: u8 = 0;

/// A hole with its escaped quotes made plain: `m[\"k\"]` becomes `m[ "k" ]`.
/// Each `\"` becomes two characters, so every offset in the hole is kept.
/// The rewritten text must live as long as the source it stands in for; only
/// holes that contain `\"` are copied, and each copy is kept for the life of
/// the process.
fn unescape_hole_quotes(hole: &str) -> &str {
    if !hole.contains("\\\"") {
        return hole;
    }
    let b = hole.as_bytes();
    let (mut out, mut j, mut open) = (String::with_capacity(hole.len()), 0, false);
    while j < b.len() {
        if b[j] == b'\\' && b.get(j + 1) == Some(&b'"') {
            out.push_str(if open { "\" " } else { " \"" });
            open = !open;
            j += 2;
        } else {
            let ch = hole[j..].chars().next().unwrap_or(' ');
            out.push(ch);
            j += ch.len_utf8();
        }
    }
    Box::leak(out.into_boxed_str())
}

fn fstring_hole_end(b: &[u8], start: usize) -> Option<usize> {
    let (mut depth, mut j, mut quote) = (1usize, start, None::<u8>);
    while j < b.len() {
        let c = b[j];
        let escaped_quote = c == b'\\' && b.get(j + 1) == Some(&b'"');
        if let Some(q) = quote {
            if q == ESCAPED_QUOTE && escaped_quote {
                quote = None;
                j += 1;
            } else if c == b'\\' {
                j += 1;
            } else if c == q {
                quote = None;
            }
        } else if escaped_quote {
            quote = Some(ESCAPED_QUOTE);
            j += 1;
        } else if c == b'"' || c == b'\'' {
            quote = Some(c);
        } else if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(j);
            }
        }
        j += 1;
    }
    None
}

fn unescape_str(s: &str) -> String { if !s.contains('\\') { return s.to_string(); }
    let (mut out, mut ch) = (String::with_capacity(s.len()), s.chars()); while let Some(c) = ch.next() { if c == '\\' {
            match ch.next() { Some('n') => out.push('\n'), Some('t') => out.push('\t'), Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'), Some('"') => out.push('"'), Some('\'') => out.push('\''),
                Some(o) => { out.push('\\'); out.push(o); } None => out.push('\\'), }
        } else { out.push(c); } }
    out }
