//! 16-Byte Cache-Packed Flat `AstArena` & Grammar Invariants (`pilot-ast`).
//! Every `AstNode` is strictly **16 bytes**, `Copy`, and pointer-free; zero `HashMap`s exist on `AstArena`.

use pilot_intern::{Interner, KeywordId, SymbolId};
use pilot_span::ByteSpan;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    pub const NONE: Self = Self(u32::MAX);
    #[inline] pub const fn is_none(self) -> bool { self.0 == u32::MAX }
    #[inline] pub const fn is_some(self) -> bool { self.0 != u32::MAX }
}
impl From<u32> for NodeId { #[inline] fn from(v: u32) -> Self { Self(v) } }
impl From<NodeId> for u32 { #[inline] fn from(id: NodeId) -> Self { id.0 } }

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Program = 0, ImportDecl = 1, FnDecl = 3, ParamDecl = 4, TypeAdtDecl = 5, AdtVariant = 6,
    StructDecl = 8, InterfaceDecl = 10, ExtendDecl = 11, ConstDecl = 12, ConstructorDecl = 13,
    DestructorDecl = 14, ScopeDecl = 15, IndexerDecl = 16, DynamicMemberDecl = 17, OperatorDecl = 18,
    LetStmt = 100, VarStmt = 101, AssignStmt = 102, ReturnStmt = 103, GuardStmt = 104,
    DeferStmt = 105, WhileStmt = 106, BreakStmt = 107, ContinueStmt = 108, YieldStmt = 109,
    ExprStmt = 110, TryStmt = 111, CatchClause = 112,
    BlockExpr = 200, IfExpr = 201, MatchExpr = 202, MatchArm = 203, ForExpr = 204,
    BinaryExpr = 205, UnaryExpr = 206, CallExpr = 207, CallArg = 208, MemberExpr = 209,
    IndexExpr = 210, CascadeExpr = 211, PipeExpr = 212, TryPropagateExpr = 213, CatchRecoveryExpr = 214,
    CoalesceExpr = 215, LambdaExpr = 216, TupleExpr = 217, ListExpr = 218, MapExpr = 219,
    RangeExpr = 220, ComptimeExpr = 221, PolyglotBlock = 222,
    IdentExpr = 300, IntLit = 301, FloatLit = 302, BoolLit = 303, StrLit = 304,
    NoneLit = 305, KeywordLit = 306, UnitLit = 307,
    TypeNamed = 400, TypeTuple = 409,
    PatWildcard = 500, PatBind = 501, PatLiteral = 502, PatVariant = 503, PatRecord = 504,
    PatTuple = 505, PatSlice = 506, PatOr = 507, PatRange = 508,
    ErrorNode = 65535,
}

impl NodeKind {
    #[inline]
    pub const fn from_u16(raw: u16) -> Self {
        match raw {
            0 => Self::Program, 1 => Self::ImportDecl, 3 => Self::FnDecl, 4 => Self::ParamDecl,
            5 => Self::TypeAdtDecl, 6 => Self::AdtVariant, 8 => Self::StructDecl,
            10 => Self::InterfaceDecl, 11 => Self::ExtendDecl, 12 => Self::ConstDecl,
            13 => Self::ConstructorDecl, 14 => Self::DestructorDecl, 15 => Self::ScopeDecl,
            16 => Self::IndexerDecl, 17 => Self::DynamicMemberDecl, 18 => Self::OperatorDecl,
            100 => Self::LetStmt, 101 => Self::VarStmt, 102 => Self::AssignStmt, 103 => Self::ReturnStmt,
            104 => Self::GuardStmt, 105 => Self::DeferStmt, 106 => Self::WhileStmt, 107 => Self::BreakStmt,
            108 => Self::ContinueStmt, 109 => Self::YieldStmt, 110 => Self::ExprStmt, 111 => Self::TryStmt,
            112 => Self::CatchClause, 200 => Self::BlockExpr, 201 => Self::IfExpr, 202 => Self::MatchExpr,
            203 => Self::MatchArm, 204 => Self::ForExpr, 205 => Self::BinaryExpr, 206 => Self::UnaryExpr,
            207 => Self::CallExpr, 208 => Self::CallArg, 209 => Self::MemberExpr, 210 => Self::IndexExpr,
            211 => Self::CascadeExpr, 212 => Self::PipeExpr, 213 => Self::TryPropagateExpr,
            214 => Self::CatchRecoveryExpr, 215 => Self::CoalesceExpr, 216 => Self::LambdaExpr,
            217 => Self::TupleExpr, 218 => Self::ListExpr, 219 => Self::MapExpr, 220 => Self::RangeExpr,
            221 => Self::ComptimeExpr, 222 => Self::PolyglotBlock, 300 => Self::IdentExpr,
            301 => Self::IntLit, 302 => Self::FloatLit, 303 => Self::BoolLit, 304 => Self::StrLit,
            305 => Self::NoneLit, 306 => Self::KeywordLit, 307 => Self::UnitLit, 400 => Self::TypeNamed,
            409 => Self::TypeTuple, 500 => Self::PatWildcard, 501 => Self::PatBind, 502 => Self::PatLiteral,
            503 => Self::PatVariant, 504 => Self::PatRecord, 505 => Self::PatTuple, 506 => Self::PatSlice,
            507 => Self::PatOr, 508 => Self::PatRange, _ => Self::ErrorNode,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Program => "Program", Self::ImportDecl => "ImportDecl", Self::FnDecl => "FnDecl",
            Self::ParamDecl => "ParamDecl", Self::TypeAdtDecl => "TypeAdtDecl", Self::AdtVariant => "AdtVariant",
            Self::StructDecl => "StructDecl", Self::InterfaceDecl => "InterfaceDecl", Self::ExtendDecl => "ExtendDecl",
            Self::ConstDecl => "ConstDecl", Self::ConstructorDecl => "ConstructorDecl",
            Self::DestructorDecl => "DestructorDecl", Self::ScopeDecl => "ScopeDecl",
            Self::IndexerDecl => "IndexerDecl", Self::DynamicMemberDecl => "DynamicMemberDecl",
            Self::OperatorDecl => "OperatorDecl", Self::LetStmt => "LetStmt", Self::VarStmt => "VarStmt",
            Self::AssignStmt => "AssignStmt", Self::ReturnStmt => "ReturnStmt", Self::GuardStmt => "GuardStmt",
            Self::DeferStmt => "DeferStmt", Self::WhileStmt => "WhileStmt", Self::BreakStmt => "BreakStmt",
            Self::ContinueStmt => "ContinueStmt", Self::YieldStmt => "YieldStmt", Self::ExprStmt => "ExprStmt",
            Self::TryStmt => "TryStmt", Self::CatchClause => "CatchClause", Self::BlockExpr => "BlockExpr",
            Self::IfExpr => "IfExpr", Self::MatchExpr => "MatchExpr", Self::MatchArm => "MatchArm",
            Self::ForExpr => "ForExpr", Self::BinaryExpr => "BinaryExpr", Self::UnaryExpr => "UnaryExpr",
            Self::CallExpr => "CallExpr", Self::CallArg => "CallArg", Self::MemberExpr => "MemberExpr",
            Self::IndexExpr => "IndexExpr", Self::CascadeExpr => "CascadeExpr", Self::PipeExpr => "PipeExpr",
            Self::TryPropagateExpr => "TryPropagateExpr", Self::CatchRecoveryExpr => "CatchRecoveryExpr",
            Self::CoalesceExpr => "CoalesceExpr", Self::LambdaExpr => "LambdaExpr", Self::TupleExpr => "TupleExpr",
            Self::ListExpr => "ListExpr", Self::MapExpr => "MapExpr", Self::RangeExpr => "RangeExpr",
            Self::ComptimeExpr => "ComptimeExpr", Self::PolyglotBlock => "PolyglotBlock",
            Self::IdentExpr => "IdentExpr", Self::IntLit => "IntLit", Self::FloatLit => "FloatLit",
            Self::BoolLit => "BoolLit", Self::StrLit => "StrLit", Self::NoneLit => "NoneLit",
            Self::KeywordLit => "KeywordLit", Self::UnitLit => "UnitLit", Self::TypeNamed => "TypeNamed",
            Self::TypeTuple => "TypeTuple", Self::PatWildcard => "PatWildcard", Self::PatBind => "PatBind",
            Self::PatLiteral => "PatLiteral", Self::PatVariant => "PatVariant", Self::PatRecord => "PatRecord",
            Self::PatTuple => "PatTuple", Self::PatSlice => "PatSlice", Self::PatOr => "PatOr",
            Self::PatRange => "PatRange", Self::ErrorNode => "ErrorNode",
        }
    }

    #[inline] pub const fn is_type_spec(self) -> bool { matches!(self, Self::TypeNamed | Self::TypeTuple) }
    #[inline] pub const fn is_expr(self) -> bool {
        let v = self as u16;
        (v >= 200 && v <= 349) && v != Self::MatchArm as u16
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CallQuoteKind {
    None = 0, ShallowQuote = 1, DeepSplice = 2, AbiFrame = 3,
    ReactiveSignal = 4, SymbolicAdjoint = 5, EffectDryRun = 6,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExprPrecedence {
    Lowest = 0, Cascade = 1, Assign = 2, LambdaTernary = 4, CatchRecovery = 6,
    Pipe = 8, Coalesce = 10, LogicalOr = 12, LogicalAnd = 14, LogicalNot = 16,
    Comparison = 18, ArrowDto = 20, Range = 22, BitOr = 24, BitXor = 26,
    BitAnd = 28, Shift = 30, AddSub = 32, MulDivRem = 34, UnaryPrefix = 36,
    Power = 38, PostfixLed = 40,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TypePrecedence {
    Lowest = 0, Fallible = 5, FnArrow = 10, Union = 15, Intersection = 20, PostfixOptional = 30,
}

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add = 1, Sub = 2, Mul = 3, Div = 4, FloorDiv = 5, Rem = 6, Pow = 7,
    Eq = 8, Ne = 9, Lt = 10, Le = 11, Gt = 12, Ge = 13, And = 14, Or = 15,
    BitAnd = 16, BitOr = 17, BitXor = 18, Shl = 19, Shr = 20, Pipe = 21,
    Coalesce = 22, RangeInclusive = 23, RangeHalfOpen = 24, Is = 25, Walrus = 26,
    Send = 27, Union = 28, Intersect = 29, Except = 30, PipeBackward = 31,
    As = 32, AsView = 33, Spaceship = 34, SatAdd = 35, SatSub = 36, SatMul = 37,
    WrapAdd = 38, WrapSub = 39, WrapMul = 40, CheckedAdd = 41, CheckedSub = 42,
    CheckedMul = 43, EucMod = 44, Lens = 45, OpenRange = 46, AfterRange = 47, Swap = 48,
}

impl BinaryOp {
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+", Self::Sub => "-", Self::Mul => "*", Self::Div => "/",
            Self::FloorDiv => "//", Self::Rem => "%", Self::Pow => "**", Self::Eq => "==",
            Self::Ne => "!=", Self::Lt => "<", Self::Le => "<=", Self::Gt => ">",
            Self::Ge => ">=", Self::And => "and", Self::Or => "or", Self::BitAnd => "&",
            Self::BitOr => "|", Self::BitXor => "^", Self::Shl => "<<", Self::Shr => ">>",
            Self::Pipe => "|>", Self::Coalesce => "??", Self::RangeInclusive => "..",
            Self::RangeHalfOpen => "..<", Self::Is => "is", Self::Walrus => ":=",
            Self::Send => "<-", Self::Union => "union", Self::Intersect => "intersect",
            Self::Except => "except", Self::PipeBackward => "<|", Self::As => "as",
            Self::AsView => "as view", Self::Spaceship => "<=>", Self::SatAdd => "+|",
            Self::SatSub => "-|", Self::SatMul => "*|", Self::WrapAdd => "+%",
            Self::WrapSub => "-%", Self::WrapMul => "*%", Self::CheckedAdd => "+?",
            Self::CheckedSub => "-?", Self::CheckedMul => "*?", Self::EucMod => "%%",
            Self::Lens => "~>", Self::OpenRange => ">..<", Self::AfterRange => ">..", Self::Swap => ":=:",
        }
    }
}

pub const UNARY_OP_SPLAT: u16 = 6;
pub const UNARY_OP_DOUBLE_SPLAT: u16 = 7;
pub const UNARY_OP_DEEP_FLATTEN: u16 = 8;

pub const DECL_VIS_MASK: u16 = 0x000F;
pub const DECL_FLAG_PARTIAL: u16 = 16;
pub const DECL_FLAG_STATIC: u16 = 32;
pub const DECL_FLAG_PURE: u16 = 64;
pub const DECL_FLAG_MUT: u16 = 128;
pub const DECL_FLAG_COMPTIME: u16 = 256;
pub const DECL_FLAG_PKG: u16 = 512;
pub const DECL_FLAG_DTOR: u16 = 1024;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AstAttribute {
    pub target: NodeId,
    pub name: SymbolId,
    pub args_start: u32,
    pub args_len: u16,
    pub is_mut_binding: bool,
    pub binding_sym: SymbolId,
}

#[repr(C, align(8))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AstNode {
    pub kind: u16,
    pub flags: u16,
    pub span_start: u32,
    pub lhs: u32,
    pub rhs: u32,
}

const _: () = assert!(core::mem::size_of::<AstNode>() == 16 && core::mem::align_of::<AstNode>() == 8);

impl AstNode {
    #[inline] pub const fn new(kind: NodeKind, flags: u16, span_start: u32, lhs: u32, rhs: u32) -> Self {
        Self { kind: kind as u16, flags, span_start, lhs, rhs }
    }
    #[inline] pub const fn node_kind(self) -> NodeKind { NodeKind::from_u16(self.kind) }
}

#[derive(Clone, Debug, Default)]
pub struct AstArena {
    pub nodes: Vec<AstNode>,
    pub spans: Vec<ByteSpan>,
    pub decl_flags: Vec<u16>,
    pub type_annots: Vec<NodeId>,
    pub extra_children: Vec<u32>,
    pub attrs: Vec<AstAttribute>,
    pub base_types: Vec<(NodeId, NodeId)>,
    pub type_params: Vec<(NodeId, SymbolId)>,
    pub for_clauses: Vec<(NodeId, NodeId)>,
    pub diagnostics: Vec<(ByteSpan, String)>,
    pub i64_pool: Vec<i64>,
    pub f64_pool: Vec<f64>,
    pub interner: Interner,
    pub root: NodeId,
}

impl AstArena {
    pub fn new() -> Self { Self::default() }

    pub fn with_capacity(expected: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(expected), spans: Vec::with_capacity(expected),
            decl_flags: Vec::with_capacity(expected), type_annots: Vec::with_capacity(expected),
            extra_children: Vec::with_capacity(expected),
            attrs: Vec::new(), base_types: Vec::new(), type_params: Vec::new(),
            for_clauses: Vec::new(), diagnostics: Vec::new(),
            i64_pool: Vec::with_capacity(expected / 4), f64_pool: Vec::with_capacity(expected / 8),
            interner: Interner::with_capacity(expected / 2, expected * 8), root: NodeId::NONE,
        }
    }

    #[inline]
    pub fn alloc_node(&mut self, kind: NodeKind, flags: u16, span: ByteSpan, lhs: u32, rhs: u32) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(AstNode::new(kind, flags, span.start, lhs, rhs));
        self.spans.push(span);
        self.decl_flags.push(DECL_VIS_MASK);
        self.type_annots.push(NodeId::NONE);
        id
    }

    #[inline]
    pub fn alloc_child_slice(&mut self, children: &[NodeId]) -> (u32, u32) {
        let start = self.extra_children.len() as u32;
        for &c in children { self.extra_children.push(c.0); }
        (start, children.len() as u32)
    }

    #[inline]
    pub fn alloc_list_node(&mut self, kind: NodeKind, flags: u16, span: ByteSpan, children: &[NodeId]) -> NodeId {
        let (start, len) = self.alloc_child_slice(children);
        self.alloc_node(kind, flags, span, start, len)
    }

    #[inline]
    pub fn alloc_type_named(&mut self, base: SymbolId, args: &[NodeId], span: ByteSpan) -> NodeId {
        if args.is_empty() { self.alloc_node(NodeKind::TypeNamed, 0, span, base.0, 0) }
        else { let (start, len) = self.alloc_child_slice(args); self.alloc_node(NodeKind::TypeNamed, len as u16, span, base.0, start) }
    }

    #[inline]
    pub fn alloc_for(&mut self, var_sym: SymbolId, iter: NodeId, body: NodeId, then_node: NodeId, span: ByteSpan) -> NodeId {
        let clause_idx = self.for_clauses.len();
        self.for_clauses.push((iter, then_node));
        self.alloc_node(NodeKind::ForExpr, (clause_idx + 1) as u16, span, var_sym.0, body.0)
    }

    #[inline] pub fn alloc_int(&mut self, val: i64, span: ByteSpan) -> NodeId {
        let idx = self.i64_pool.len() as u32; self.i64_pool.push(val); self.alloc_node(NodeKind::IntLit, 0, span, idx, 0)
    }
    #[inline] pub fn alloc_float(&mut self, val: f64, span: ByteSpan) -> NodeId {
        let idx = self.f64_pool.len() as u32; self.f64_pool.push(val); self.alloc_node(NodeKind::FloatLit, 0, span, idx, 0)
    }
    #[inline] pub fn alloc_ident(&mut self, name: &str, span: ByteSpan) -> NodeId {
        let sym = self.interner.intern(name); self.alloc_node(NodeKind::IdentExpr, 0, span, sym.0, 0)
    }
    #[inline] pub fn alloc_str(&mut self, value: &str, span: ByteSpan) -> NodeId {
        let sym = self.interner.intern(value); self.alloc_node(NodeKind::StrLit, 0, span, sym.0, 0)
    }
    #[inline] pub fn alloc_keyword(&mut self, raw_kw: &str, span: ByteSpan) -> NodeId {
        let kw: KeywordId = self.interner.intern_keyword(raw_kw); self.alloc_node(NodeKind::KeywordLit, 0, span, kw.0, 0)
    }
    #[inline] pub fn alloc_polyglot_block(&mut self, lang: &str, code: &str, flags: u16, span: ByteSpan) -> NodeId {
        let (l, c) = (self.interner.intern(lang), self.interner.intern(code));
        self.alloc_node(NodeKind::PolyglotBlock, flags, span, l.0, c.0)
    }

    #[inline] pub fn node(&self, id: NodeId) -> &AstNode { &self.nodes[id.0 as usize] }
    #[inline] pub fn span(&self, id: NodeId) -> ByteSpan { self.spans.get(id.0 as usize).copied().unwrap_or_default() }
    #[inline] pub fn type_annot(&self, id: NodeId) -> Option<NodeId> { self.type_annots.get(id.0 as usize).copied().filter(|t| t.is_some()) }
    #[inline] pub fn set_type_annot(&mut self, id: NodeId, ty: NodeId) { if let Some(s) = self.type_annots.get_mut(id.0 as usize) { *s = ty; } }

    #[inline] pub fn type_args(&self, id: NodeId) -> &[u32] {
        let n = self.node(id);
        if n.node_kind() == NodeKind::TypeNamed && n.flags > 0 { self.children_slice(n.rhs, n.flags as u32) } else { &[] }
    }
    #[inline] pub fn for_iter(&self, id: NodeId) -> NodeId {
        let f = self.node(id).flags as usize;
        if f > 0 { self.for_clauses.get(f - 1).map_or(NodeId::NONE, |c| c.0) } else { NodeId::NONE }
    }
    #[inline] pub fn for_then(&self, id: NodeId) -> NodeId {
        let f = self.node(id).flags as usize;
        if f > 0 { self.for_clauses.get(f - 1).map_or(NodeId::NONE, |c| c.1) } else { NodeId::NONE }
    }
    #[inline] pub fn children_slice(&self, start: u32, len: u32) -> &[u32] {
        let s = start as usize;
        if s >= self.extra_children.len() || len == 0 { return &[]; }
        &self.extra_children[s..(s + len as usize).min(self.extra_children.len())]
    }

    pub fn to_sexpr(&self, id: NodeId) -> String {
        let mut out = String::new(); self.write_sexpr(id, &mut out); out
    }

    fn write_sexpr(&self, id: NodeId, out: &mut String) {
        if id.is_none() || (id.0 as usize) >= self.nodes.len() { out.push_str("nil"); return; }
        let (n, kind) = (self.node(id), self.node(id).node_kind());
        match kind {
            NodeKind::Program | NodeKind::BlockExpr | NodeKind::TupleExpr | NodeKind::ListExpr => {
                out.push('('); out.push_str(kind.name());
                for &c in self.children_slice(n.lhs, n.rhs) { out.push(' '); self.write_sexpr(NodeId(c), out); }
                out.push(')');
            }
            NodeKind::TypeNamed => {
                out.push_str("(TypeNamed "); out.push_str(self.interner.resolve(SymbolId(n.lhs)));
                for &c in self.type_args(id) { out.push(' '); self.write_sexpr(NodeId(c), out); }
                out.push(')');
            }
            NodeKind::IdentExpr | NodeKind::PolyglotBlock => {
                out.push('('); out.push_str(if kind == NodeKind::IdentExpr { "Ident " } else { "PolyglotBlock " });
                out.push_str(self.interner.resolve(SymbolId(n.lhs))); out.push(')');
            }
            NodeKind::IntLit => out.push_str(&format!("(Int {})", self.i64_pool.get(n.lhs as usize).copied().unwrap_or(0))),
            NodeKind::FloatLit => out.push_str(&format!("(Float {:?})", self.f64_pool.get(n.lhs as usize).copied().unwrap_or(0.0))),
            NodeKind::BoolLit => out.push_str(if n.lhs != 0 { "(Bool true)" } else { "(Bool false)" }),
            NodeKind::StrLit => out.push_str(&format!("(Str {:?})", self.interner.resolve(SymbolId(n.lhs)))),
            NodeKind::NoneLit => out.push_str("(None)"),
            NodeKind::KeywordLit => {
                if let Some(kw) = self.interner.resolve_keyword(KeywordId(n.lhs)) {
                    out.push_str("(Kw :"); out.push_str(self.interner.resolve(kw.raw_symbol)); out.push(')');
                } else { out.push_str("(Kw :?)"); }
            }
            NodeKind::ImportDecl | NodeKind::FnDecl | NodeKind::TypeAdtDecl
            | NodeKind::StructDecl | NodeKind::InterfaceDecl | NodeKind::ExtendDecl => {
                out.push('('); out.push_str(kind.name()); out.push(' ');
                out.push_str(self.interner.resolve(SymbolId(n.lhs))); out.push(')');
            }
            NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
                out.push('('); out.push_str(kind.name()); out.push(' ');
                out.push_str(self.interner.resolve(SymbolId(n.lhs))); out.push(' ');
                self.write_sexpr(NodeId(n.rhs), out); out.push(')');
            }
            NodeKind::ReturnStmt => { out.push_str("(ReturnStmt "); self.write_sexpr(NodeId(n.lhs), out); out.push(')'); }
            NodeKind::BinaryExpr => {
                out.push_str(&format!("(BinaryExpr op={} ", n.flags));
                self.write_sexpr(NodeId(n.lhs), out); out.push(' '); self.write_sexpr(NodeId(n.rhs), out); out.push(')');
            }
            _ => {
                out.push('('); out.push_str(kind.name());
                if n.lhs != u32::MAX { out.push(' '); self.write_sexpr(NodeId(n.lhs), out); }
                if n.rhs != u32::MAX && n.rhs != 0 { out.push(' '); self.write_sexpr(NodeId(n.rhs), out); }
                out.push(')');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ast_node_is_strictly_16_bytes_and_sexpr_roundtrips() {
        assert_eq!(core::mem::size_of::<AstNode>(), 16);
        assert!(TypePrecedence::PostfixOptional > TypePrecedence::Intersection && TypePrecedence::Union > TypePrecedence::Fallible);
        assert!(ExprPrecedence::Cascade < ExprPrecedence::Assign && ExprPrecedence::Pipe < ExprPrecedence::PostfixLed);
        let mut arena = AstArena::with_capacity(16);
        let (lhs, rhs) = (arena.alloc_int(40, ByteSpan::new(8, 10)), arena.alloc_int(2, ByteSpan::new(13, 14)));
        let add = arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::Add as u16, ByteSpan::new(8, 14), lhs.0, rhs.0);
        let sym_x = arena.interner.intern("x");
        let let_stmt = arena.alloc_node(NodeKind::LetStmt, 0, ByteSpan::new(0, 15), sym_x.0, add.0);
        arena.root = arena.alloc_list_node(NodeKind::Program, 0, ByteSpan::new(0, 15), &[let_stmt]);
        assert_eq!(arena.to_sexpr(arena.root), "(Program (LetStmt x (BinaryExpr op=1 (Int 40) (Int 2))))");
    }
}
