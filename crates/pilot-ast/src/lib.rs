//! Milestone 28.3: 16-Byte Cache-Packed Flat `AstArena` & Grammar Invariant Freeze (`pilot-ast`)
//!
//! Enforces Cross-Milestone Architectural Invariants #1–#9:
//! - Every `AstNode` is strictly **16 bytes** (`4` nodes per 64-byte L1 cache line), `Copy`, and pointer-free.
//! - Variable-arity child lists (`Program`, `BlockExpr`, `CallExpr`, `MatchExpr`, `TupleExpr`, `ListExpr`)
//!   are stored contiguously in `extra_children: Vec<u32>` referenced by `(start, len)`.
//! - Typed zero-cost view wrappers (`ExprView`, `StmtView`, `TypeSpecView`, `PatternView`) carry only
//!   `(&AstArena, NodeId)` with zero heap allocations.

use pilot_intern::{Interner, KeywordId, SymbolId};
use pilot_span::ByteSpan;

/// 32-bit `Copy` index into `AstArena::nodes`.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    pub const NONE: Self = Self(u32::MAX);

    #[inline]
    pub const fn is_none(self) -> bool {
        self.0 == u32::MAX
    }

    #[inline]
    pub const fn is_some(self) -> bool {
        self.0 != u32::MAX
    }
}

impl From<u32> for NodeId {
    #[inline]
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl From<NodeId> for u32 {
    #[inline]
    fn from(id: NodeId) -> Self {
        id.0
    }
}

/// Canonical 16-bit syntax/AST node discriminator (`0..65535`).
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
    // --- Root & Module Declarations (0..99) ---
    Program = 0,
    ImportDecl = 1,
    FromImportDecl = 2,
    FnDecl = 3,
    ParamDecl = 4,
    TypeAdtDecl = 5,
    AdtVariant = 6,
    RecordDecl = 7,
    StructDecl = 8,
    ClassDecl = 9,
    InterfaceDecl = 10,
    ExtendDecl = 11,
    ConstDecl = 12,

    // --- Statements & Control Flow (100..199) ---
    LetStmt = 100,
    VarStmt = 101,
    AssignStmt = 102,
    ReturnStmt = 103,
    GuardStmt = 104,
    DeferStmt = 105,
    WhileStmt = 106,
    BreakStmt = 107,
    ContinueStmt = 108,
    YieldStmt = 109,
    ExprStmt = 110,
    TryStmt = 111,
    CatchClause = 112,

    // --- Expressions (200..299) ---
    BlockExpr = 200,
    IfExpr = 201,
    MatchExpr = 202,
    MatchArm = 203,
    ForExpr = 204,
    BinaryExpr = 205,
    UnaryExpr = 206,
    CallExpr = 207,
    CallArg = 208,
    MemberExpr = 209,
    IndexExpr = 210,
    CascadeExpr = 211,
    PipeExpr = 212,
    TryPropagateExpr = 213,
    CatchRecoveryExpr = 214,
    CoalesceExpr = 215,
    LambdaExpr = 216,
    TupleExpr = 217,
    ListExpr = 218,
    MapExpr = 219,
    RangeExpr = 220,
    ComptimeExpr = 221,
    PolyglotBlock = 222,

    // --- Atomic Literals & Identifiers (300..349) ---
    IdentExpr = 300,
    IntLit = 301,
    FloatLit = 302,
    BoolLit = 303,
    StrLit = 304,
    NoneLit = 305,
    KeywordLit = 306,
    UnitLit = 307,

    // --- Type Specifications (400..449, Invariant #6) ---
    TypeNamed = 400,
    TypeOptional = 401,     // T? (bp = 30)
    TypeIntersection = 402, // A & B (bp = 20)
    TypeUnion = 403,        // A | B (bp = 15)
    TypeFn = 404,           // (A) -> B (bp = 10)
    TypeFallible = 405,     // T ! E (bp = 5)
    TypeSlice = 406,        // SliceBuffer[T]
    TypeList = 407,         // [T]
    TypeMap = 408,          // {K: V}
    TypeTuple = 409,        // (T1, T2)

    // --- Exhaustive Patterns (500..549, M11) ---
    PatWildcard = 500,
    PatBind = 501,
    PatLiteral = 502,
    PatVariant = 503,
    PatRecord = 504,
    PatTuple = 505,
    PatSlice = 506,
    PatOr = 507,
    PatRange = 508,

    // --- Resilient Error Recovery Node (M31.2) ---
    ErrorNode = 65535,
}

impl NodeKind {
    #[inline]
    pub const fn from_u16(raw: u16) -> Self {
        match raw {
            0 => Self::Program,
            1 => Self::ImportDecl,
            2 => Self::FromImportDecl,
            3 => Self::FnDecl,
            4 => Self::ParamDecl,
            5 => Self::TypeAdtDecl,
            6 => Self::AdtVariant,
            7 => Self::RecordDecl,
            8 => Self::StructDecl,
            9 => Self::ClassDecl,
            10 => Self::InterfaceDecl,
            11 => Self::ExtendDecl,
            12 => Self::ConstDecl,
            100 => Self::LetStmt,
            101 => Self::VarStmt,
            102 => Self::AssignStmt,
            103 => Self::ReturnStmt,
            104 => Self::GuardStmt,
            105 => Self::DeferStmt,
            106 => Self::WhileStmt,
            107 => Self::BreakStmt,
            108 => Self::ContinueStmt,
            109 => Self::YieldStmt,
            110 => Self::ExprStmt,
            111 => Self::TryStmt,
            112 => Self::CatchClause,
            200 => Self::BlockExpr,
            201 => Self::IfExpr,
            202 => Self::MatchExpr,
            203 => Self::MatchArm,
            204 => Self::ForExpr,
            205 => Self::BinaryExpr,
            206 => Self::UnaryExpr,
            207 => Self::CallExpr,
            208 => Self::CallArg,
            209 => Self::MemberExpr,
            210 => Self::IndexExpr,
            211 => Self::CascadeExpr,
            212 => Self::PipeExpr,
            213 => Self::TryPropagateExpr,
            214 => Self::CatchRecoveryExpr,
            215 => Self::CoalesceExpr,
            216 => Self::LambdaExpr,
            217 => Self::TupleExpr,
            218 => Self::ListExpr,
            219 => Self::MapExpr,
            220 => Self::RangeExpr,
            221 => Self::ComptimeExpr,
            222 => Self::PolyglotBlock,
            300 => Self::IdentExpr,
            301 => Self::IntLit,
            302 => Self::FloatLit,
            303 => Self::BoolLit,
            304 => Self::StrLit,
            305 => Self::NoneLit,
            306 => Self::KeywordLit,
            307 => Self::UnitLit,
            400 => Self::TypeNamed,
            401 => Self::TypeOptional,
            402 => Self::TypeIntersection,
            403 => Self::TypeUnion,
            404 => Self::TypeFn,
            405 => Self::TypeFallible,
            406 => Self::TypeSlice,
            407 => Self::TypeList,
            408 => Self::TypeMap,
            409 => Self::TypeTuple,
            500 => Self::PatWildcard,
            501 => Self::PatBind,
            502 => Self::PatLiteral,
            503 => Self::PatVariant,
            504 => Self::PatRecord,
            505 => Self::PatTuple,
            506 => Self::PatSlice,
            507 => Self::PatOr,
            508 => Self::PatRange,
            _ => Self::ErrorNode,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Program => "Program",
            Self::ImportDecl => "ImportDecl",
            Self::FromImportDecl => "FromImportDecl",
            Self::FnDecl => "FnDecl",
            Self::ParamDecl => "ParamDecl",
            Self::TypeAdtDecl => "TypeAdtDecl",
            Self::AdtVariant => "AdtVariant",
            Self::RecordDecl => "RecordDecl",
            Self::StructDecl => "StructDecl",
            Self::ClassDecl => "ClassDecl",
            Self::InterfaceDecl => "InterfaceDecl",
            Self::ExtendDecl => "ExtendDecl",
            Self::ConstDecl => "ConstDecl",
            Self::LetStmt => "LetStmt",
            Self::VarStmt => "VarStmt",
            Self::AssignStmt => "AssignStmt",
            Self::ReturnStmt => "ReturnStmt",
            Self::GuardStmt => "GuardStmt",
            Self::DeferStmt => "DeferStmt",
            Self::WhileStmt => "WhileStmt",
            Self::BreakStmt => "BreakStmt",
            Self::ContinueStmt => "ContinueStmt",
            Self::YieldStmt => "YieldStmt",
            Self::ExprStmt => "ExprStmt",
            Self::TryStmt => "TryStmt",
            Self::CatchClause => "CatchClause",
            Self::BlockExpr => "BlockExpr",
            Self::IfExpr => "IfExpr",
            Self::MatchExpr => "MatchExpr",
            Self::MatchArm => "MatchArm",
            Self::ForExpr => "ForExpr",
            Self::BinaryExpr => "BinaryExpr",
            Self::UnaryExpr => "UnaryExpr",
            Self::CallExpr => "CallExpr",
            Self::CallArg => "CallArg",
            Self::MemberExpr => "MemberExpr",
            Self::IndexExpr => "IndexExpr",
            Self::CascadeExpr => "CascadeExpr",
            Self::PipeExpr => "PipeExpr",
            Self::TryPropagateExpr => "TryPropagateExpr",
            Self::CatchRecoveryExpr => "CatchRecoveryExpr",
            Self::CoalesceExpr => "CoalesceExpr",
            Self::LambdaExpr => "LambdaExpr",
            Self::TupleExpr => "TupleExpr",
            Self::ListExpr => "ListExpr",
            Self::MapExpr => "MapExpr",
            Self::RangeExpr => "RangeExpr",
            Self::ComptimeExpr => "ComptimeExpr",
            Self::PolyglotBlock => "PolyglotBlock",
            Self::IdentExpr => "IdentExpr",
            Self::IntLit => "IntLit",
            Self::FloatLit => "FloatLit",
            Self::BoolLit => "BoolLit",
            Self::StrLit => "StrLit",
            Self::NoneLit => "NoneLit",
            Self::KeywordLit => "KeywordLit",
            Self::UnitLit => "UnitLit",
            Self::TypeNamed => "TypeNamed",
            Self::TypeOptional => "TypeOptional",
            Self::TypeIntersection => "TypeIntersection",
            Self::TypeUnion => "TypeUnion",
            Self::TypeFn => "TypeFn",
            Self::TypeFallible => "TypeFallible",
            Self::TypeSlice => "TypeSlice",
            Self::TypeList => "TypeList",
            Self::TypeMap => "TypeMap",
            Self::TypeTuple => "TypeTuple",
            Self::PatWildcard => "PatWildcard",
            Self::PatBind => "PatBind",
            Self::PatLiteral => "PatLiteral",
            Self::PatVariant => "PatVariant",
            Self::PatRecord => "PatRecord",
            Self::PatTuple => "PatTuple",
            Self::PatSlice => "PatSlice",
            Self::PatOr => "PatOr",
            Self::PatRange => "PatRange",
            Self::ErrorNode => "ErrorNode",
        }
    }
}

/// Milestone 17 & 18 Unified Call-Quote Projection Kind (Invariant #1).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CallQuoteKind {
    None = 0,            // fn(a, b)      -> TReturn
    ShallowQuote = 1,    // fn!(a, b)     -> CallExpr<TReturn>
    DeepSplice = 2,      // fn!!(a, b)    -> AstExpr<TReturn> (supports `~arg` quasiquoting)
    AbiFrame = 3,        // fn!&(a, b)    -> CallFrame_<FnSig>
    ReactiveSignal = 4,  // fn!^(a, b)    -> Signal<TReturn>
    SymbolicAdjoint = 5, // fn!@(a, b)    -> Dual<TReturn, TGradTuple>
    EffectDryRun = 6,    // fn!?(a, b)    -> EffectPlan<TReturn>
}

/// Milestone 28.3 & 30.1 Pratt Expression Binding-Power Hierarchy (Invariant #8).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExprPrecedence {
    Lowest = 0,
    Cascade = 1,        // .. and ?.. on outer receiver (binds looser than =, eliminating post-parse hoisting!)
    Assign = 2,         // =, +=, -=, *=, /=, ??=, := (Right-assoc)
    LambdaTernary = 4,  // =>, expr if cond else alt (Right-assoc)
    CatchRecovery = 6,  // expr catch default / expr catch err => ... (M13)
    Pipe = 8,           // |> (Left), <| (Right), fluent |> .Method()
    Coalesce = 10,      // ?? (Right-assoc)
    LogicalOr = 12,     // or, ||
    LogicalAnd = 14,    // and, &&
    LogicalNot = 16,    // not (Prefix)
    Comparison = 18,    // ==, !=, <, <=, >, >=, is, in, not in (Chained)
    ArrowDto = 20,      // -> DTO projection, ->> JSON extract
    Range = 22,         // .., ..=, ..< (M15.1 Invariant #3)
    BitOr = 24,         // |
    BitXor = 26,        // ^
    BitAnd = 28,        // &
    Shift = 30,         // <<, >>
    AddSub = 32,        // +, -
    MulDivRem = 34,     // *, /, //, %
    UnaryPrefix = 36,   // -, +, ~, !, await, some, ref, *
    Power = 38,         // ** (Right-assoc)
    PostfixLed = 40,    // .prop, ?.prop, [idx], (args), !/!!/!&/!^/!@/!?, postfix ?
}

/// Milestone 28.3 & Invariant #6 Pratt Type-Spec Binding-Power Hierarchy (`T?` > `&` > `|` > `->` > `!`).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TypePrecedence {
    Lowest = 0,
    Fallible = 5,        // T ! E (binds loosest so `User? ! IoError | ParseError` == `(User?) ! (IoError | ParseError)`)
    FnArrow = 10,        // (A, B) -> R
    Union = 15,          // A | B
    Intersection = 20,   // A & B
    PostfixOptional = 30,// T? (binds tightest)
}

/// Binary operator codes packed into `AstNode::flags`.
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add = 1,
    Sub = 2,
    Mul = 3,
    Div = 4,
    FloorDiv = 5,
    Rem = 6,
    Pow = 7,
    Eq = 8,
    Ne = 9,
    Lt = 10,
    Le = 11,
    Gt = 12,
    Ge = 13,
    And = 14,
    Or = 15,
    BitAnd = 16,
    BitOr = 17,
    BitXor = 18,
    Shl = 19,
    Shr = 20,
    Pipe = 21,
    Coalesce = 22,
    RangeInclusive = 23, // a..b / a..=b (Invariant #3)
    RangeHalfOpen = 24,  // a..<b        (Invariant #3)
    Is = 25,             // a is Pattern / a is Type
    Walrus = 26,         // a := b
}

impl BinaryOp {
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::FloorDiv => "//",
            Self::Rem => "%",
            Self::Pow => "**",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::And => "and",
            Self::Or => "or",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::Pipe => "|>",
            Self::Coalesce => "??",
            Self::RangeInclusive => "..",
            Self::RangeHalfOpen => "..<",
            Self::Is => "is",
            Self::Walrus => ":=",
        }
    }
}

/// Hardware-packed **16-byte** C-ABI AST node (`4` nodes per 64-byte L1 cache line).
/// Zero raw pointers, zero individual heap allocations, `Copy + Send + Sync`.
#[repr(C, align(8))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AstNode {
    pub kind: u16,       // NodeKind as u16
    pub flags: u16,      // CallQuoteKind, BinaryOp, mutability, or unquote flag (`~arg`)
    pub span_start: u32, // UTF-8 byte offset or token index
    pub lhs: u32,        // Child NodeId, SymbolId, or literal pool index
    pub rhs: u32,        // Child NodeId, or slice index into extra_children
}

// Compile-time hardware layout guarantee (Invariant #9): strictly 16 bytes, 8-byte aligned!
const _: () = assert!(core::mem::size_of::<AstNode>() == 16);
const _: () = assert!(core::mem::align_of::<AstNode>() == 8);

impl AstNode {
    #[inline]
    pub const fn new(kind: NodeKind, flags: u16, span_start: u32, lhs: u32, rhs: u32) -> Self {
        Self {
            kind: kind as u16,
            flags,
            span_start,
            lhs,
            rhs,
        }
    }

    #[inline]
    pub const fn node_kind(self) -> NodeKind {
        NodeKind::from_u16(self.kind)
    }
}

/// Flat, pointer-free AST arena storing 16-byte `AstNode`s, side-table literal pools,
/// and variable-arity child index slices (`extra_children`).
#[derive(Clone, Debug, Default)]
pub struct AstArena {
    pub nodes: Vec<AstNode>,
    pub spans: Vec<ByteSpan>,
    pub extra_children: Vec<u32>,
    pub i64_pool: Vec<i64>,
    pub f64_pool: Vec<f64>,
    pub interner: Interner,
    pub root: NodeId,
    /// Optional explicit type spec `NodeId` for `LetStmt`, `VarStmt`, `ConstDecl`, `FnDecl` (return type), or `ParamDecl`.
    pub type_annotations: std::collections::HashMap<u32, NodeId>,
    /// Iterable expression `NodeId` for `ForExpr` nodes (`for x in <iter>`).
    pub for_iter_nodes: std::collections::HashMap<u32, NodeId>,
    /// Declared `(field_name, field_type_node)` pairs for `RecordDecl`, `StructDecl`, `ClassDecl`, and `InterfaceDecl`.
    pub record_fields: std::collections::HashMap<u32, Vec<(SymbolId, NodeId)>>,
}

impl AstArena {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(expected_nodes: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(expected_nodes),
            spans: Vec::with_capacity(expected_nodes),
            extra_children: Vec::with_capacity(expected_nodes),
            i64_pool: Vec::with_capacity(expected_nodes / 4),
            f64_pool: Vec::with_capacity(expected_nodes / 8),
            interner: Interner::with_capacity(expected_nodes / 2, expected_nodes * 8),
            root: NodeId::NONE,
            type_annotations: std::collections::HashMap::new(),
            for_iter_nodes: std::collections::HashMap::new(),
            record_fields: std::collections::HashMap::new(),
        }
    }

    #[inline]
    pub fn alloc_node(&mut self, kind: NodeKind, flags: u16, span: ByteSpan, lhs: u32, rhs: u32) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(AstNode::new(kind, flags, span.start, lhs, rhs));
        self.spans.push(span);
        id
    }

    /// Appends a slice of child `NodeId`s to `extra_children` and returns `(slice_start, slice_len)`.
    #[inline]
    pub fn alloc_child_slice(&mut self, children: &[NodeId]) -> (u32, u32) {
        let start = self.extra_children.len() as u32;
        let len = children.len() as u32;
        for &c in children {
            self.extra_children.push(c.0);
        }
        (start, len)
    }

    /// Allocates an $N$-ary node (`Program`, `BlockExpr`, `TupleExpr`, `ListExpr`) whose children
    /// are stored at `extra_children[lhs .. lhs + rhs]`.
    #[inline]
    pub fn alloc_list_node(&mut self, kind: NodeKind, flags: u16, span: ByteSpan, children: &[NodeId]) -> NodeId {
        let (start, len) = self.alloc_child_slice(children);
        self.alloc_node(kind, flags, span, start, len)
    }

    #[inline]
    pub fn alloc_int(&mut self, val: i64, span: ByteSpan) -> NodeId {
        let pool_idx = self.i64_pool.len() as u32;
        self.i64_pool.push(val);
        self.alloc_node(NodeKind::IntLit, 0, span, pool_idx, 0)
    }

    #[inline]
    pub fn alloc_float(&mut self, val: f64, span: ByteSpan) -> NodeId {
        let pool_idx = self.f64_pool.len() as u32;
        self.f64_pool.push(val);
        self.alloc_node(NodeKind::FloatLit, 0, span, pool_idx, 0)
    }

    #[inline]
    pub fn alloc_ident(&mut self, name: &str, span: ByteSpan) -> NodeId {
        let sym = self.interner.intern(name);
        self.alloc_node(NodeKind::IdentExpr, 0, span, sym.0, 0)
    }

    #[inline]
    pub fn alloc_str(&mut self, value: &str, span: ByteSpan) -> NodeId {
        let sym = self.interner.intern(value);
        self.alloc_node(NodeKind::StrLit, 0, span, sym.0, 0)
    }

    #[inline]
    pub fn alloc_keyword(&mut self, raw_kw: &str, span: ByteSpan) -> NodeId {
        let kw: KeywordId = self.interner.intern_keyword(raw_kw);
        self.alloc_node(NodeKind::KeywordLit, 0, span, kw.0, 0)
    }

    #[inline]
    pub fn alloc_polyglot_block(&mut self, lang: &str, code: &str, flags: u16, span: ByteSpan) -> NodeId {
        let lang_sym = self.interner.intern(lang);
        let code_sym = self.interner.intern(code);
        self.alloc_node(NodeKind::PolyglotBlock, flags, span, lang_sym.0, code_sym.0)
    }

    #[inline]
    pub fn node(&self, id: NodeId) -> &AstNode {
        &self.nodes[id.0 as usize]
    }

    #[inline]
    pub fn span(&self, id: NodeId) -> ByteSpan {
        if id.is_none() || id.0 as usize >= self.spans.len() {
            ByteSpan::default()
        } else {
            self.spans[id.0 as usize]
        }
    }

    #[inline]
    pub fn children_slice(&self, start: u32, len: u32) -> &[u32] {
        let s = start as usize;
        if s >= self.extra_children.len() || len == 0 {
            return &[];
        }
        let e = (s + len as usize).min(self.extra_children.len());
        &self.extra_children[s..e]
    }

    #[inline]
    pub fn expr_view(&self, id: NodeId) -> ExprView<'_> {
        ExprView { arena: self, id }
    }

    #[inline]
    pub fn stmt_view(&self, id: NodeId) -> StmtView<'_> {
        StmtView { arena: self, id }
    }

    #[inline]
    pub fn type_view(&self, id: NodeId) -> TypeSpecView<'_> {
        TypeSpecView { arena: self, id }
    }

    #[inline]
    pub fn pattern_view(&self, id: NodeId) -> PatternView<'_> {
        PatternView { arena: self, id }
    }

    /// Formats the subtree rooted at `id` as a canonical S-Expression for differential oracle testing.
    pub fn to_sexpr(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.write_sexpr(id, &mut out);
        out
    }

    fn write_sexpr(&self, id: NodeId, out: &mut String) {
        if id.is_none() || (id.0 as usize) >= self.nodes.len() {
            out.push_str("nil");
            return;
        }
        let n = self.node(id);
        let kind = n.node_kind();
        match kind {
            NodeKind::Program | NodeKind::BlockExpr | NodeKind::TupleExpr | NodeKind::ListExpr => {
                out.push('(');
                out.push_str(kind.name());
                for &child_raw in self.children_slice(n.lhs, n.rhs) {
                    out.push(' ');
                    self.write_sexpr(NodeId(child_raw), out);
                }
                out.push(')');
            }
            NodeKind::IdentExpr => {
                out.push_str("(Ident ");
                out.push_str(self.interner.resolve(SymbolId(n.lhs)));
                out.push(')');
            }
            NodeKind::PolyglotBlock => {
                out.push_str("(PolyglotBlock ");
                out.push_str(self.interner.resolve(SymbolId(n.lhs)));
                out.push(')');
            }
            NodeKind::IntLit => {
                let v = self.i64_pool.get(n.lhs as usize).copied().unwrap_or(0);
                out.push_str(&format!("(Int {})", v));
            }
            NodeKind::FloatLit => {
                let v = self.f64_pool.get(n.lhs as usize).copied().unwrap_or(0.0);
                out.push_str(&format!("(Float {:?})", v));
            }
            NodeKind::BoolLit => {
                out.push_str(if n.lhs != 0 { "(Bool true)" } else { "(Bool false)" });
            }
            NodeKind::StrLit => {
                let s = self.interner.resolve(SymbolId(n.lhs));
                out.push_str(&format!("(Str {:?})", s));
            }
            NodeKind::NoneLit => {
                out.push_str("(None)");
            }
            NodeKind::KeywordLit => {
                if let Some(kw) = self.interner.resolve_keyword(KeywordId(n.lhs)) {
                    out.push_str("(Kw :");
                    out.push_str(self.interner.resolve(kw.raw_symbol));
                    out.push(')');
                } else {
                    out.push_str("(Kw :?)");
                }
            }
            NodeKind::ImportDecl
            | NodeKind::FromImportDecl
            | NodeKind::FnDecl
            | NodeKind::TypeAdtDecl
            | NodeKind::RecordDecl
            | NodeKind::StructDecl
            | NodeKind::ClassDecl
            | NodeKind::InterfaceDecl
            | NodeKind::ExtendDecl => {
                out.push('(');
                out.push_str(kind.name());
                out.push(' ');
                out.push_str(self.interner.resolve(SymbolId(n.lhs)));
                out.push(')');
            }
            NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
                out.push('(');
                out.push_str(kind.name());
                out.push(' ');
                out.push_str(self.interner.resolve(SymbolId(n.lhs)));
                out.push(' ');
                self.write_sexpr(NodeId(n.rhs), out);
                out.push(')');
            }
            NodeKind::ReturnStmt => {
                out.push_str("(ReturnStmt ");
                self.write_sexpr(NodeId(n.lhs), out);
                out.push(')');
            }
            NodeKind::BinaryExpr => {
                out.push_str("(BinaryExpr ");
                out.push_str(&format!("op={} ", n.flags));
                self.write_sexpr(NodeId(n.lhs), out);
                out.push(' ');
                self.write_sexpr(NodeId(n.rhs), out);
                out.push(')');
            }
            _ => {
                out.push('(');
                out.push_str(kind.name());
                if n.lhs != u32::MAX {
                    out.push(' ');
                    self.write_sexpr(NodeId(n.lhs), out);
                }
                if n.rhs != u32::MAX && n.rhs != 0 {
                    out.push(' ');
                    self.write_sexpr(NodeId(n.rhs), out);
                }
                out.push(')');
            }
        }
    }
}

/// Zero-cost typed expression view carrying only `(&AstArena, NodeId)`.
#[derive(Copy, Clone, Debug)]
pub struct ExprView<'a> {
    pub arena: &'a AstArena,
    pub id: NodeId,
}

impl<'a> ExprView<'a> {
    #[inline]
    pub fn kind(self) -> NodeKind {
        self.arena.node(self.id).node_kind()
    }

    #[inline]
    pub fn span(self) -> ByteSpan {
        self.arena.span(self.id)
    }

    #[inline]
    pub fn flags(self) -> u16 {
        self.arena.node(self.id).flags
    }
}

/// Zero-cost typed statement view carrying only `(&AstArena, NodeId)`.
#[derive(Copy, Clone, Debug)]
pub struct StmtView<'a> {
    pub arena: &'a AstArena,
    pub id: NodeId,
}

impl<'a> StmtView<'a> {
    #[inline]
    pub fn kind(self) -> NodeKind {
        self.arena.node(self.id).node_kind()
    }

    #[inline]
    pub fn span(self) -> ByteSpan {
        self.arena.span(self.id)
    }
}

/// Zero-cost typed `type_spec` view carrying only `(&AstArena, NodeId)`.
#[derive(Copy, Clone, Debug)]
pub struct TypeSpecView<'a> {
    pub arena: &'a AstArena,
    pub id: NodeId,
}

impl<'a> TypeSpecView<'a> {
    #[inline]
    pub fn kind(self) -> NodeKind {
        self.arena.node(self.id).node_kind()
    }

    #[inline]
    pub fn span(self) -> ByteSpan {
        self.arena.span(self.id)
    }
}

/// Zero-cost typed `match` pattern view carrying only `(&AstArena, NodeId)`.
#[derive(Copy, Clone, Debug)]
pub struct PatternView<'a> {
    pub arena: &'a AstArena,
    pub id: NodeId,
}

impl<'a> PatternView<'a> {
    #[inline]
    pub fn kind(self) -> NodeKind {
        self.arena.node(self.id).node_kind()
    }

    #[inline]
    pub fn span(self) -> ByteSpan {
        self.arena.span(self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ast_node_is_strictly_16_bytes_and_invariant_precedence_holds() {
        assert_eq!(core::mem::size_of::<AstNode>(), 16);
        assert_eq!(core::mem::align_of::<AstNode>(), 8);

        // Invariant #6: T? (30) > & (20) > | (15) > -> (10) > ! (5)
        assert!(TypePrecedence::PostfixOptional > TypePrecedence::Intersection);
        assert!(TypePrecedence::Intersection > TypePrecedence::Union);
        assert!(TypePrecedence::Union > TypePrecedence::FnArrow);
        assert!(TypePrecedence::FnArrow > TypePrecedence::Fallible);

        // Invariant #8: Cascade (1) < Assign (2) < Pipe (8) < Range (22) < PostfixLed (40)
        assert!(ExprPrecedence::Cascade < ExprPrecedence::Assign);
        assert!(ExprPrecedence::Assign < ExprPrecedence::Pipe);
        assert!(ExprPrecedence::Pipe < ExprPrecedence::Range);
        assert!(ExprPrecedence::Range < ExprPrecedence::PostfixLed);
    }

    #[test]
    fn arena_alloc_and_sexpr_roundtrip() {
        let mut arena = AstArena::with_capacity(16);
        let lhs = arena.alloc_int(40, ByteSpan::new(8, 10));
        let rhs = arena.alloc_int(2, ByteSpan::new(13, 14));
        let add = arena.alloc_node(
            NodeKind::BinaryExpr,
            BinaryOp::Add as u16,
            ByteSpan::new(8, 14),
            lhs.0,
            rhs.0,
        );
        let sym_x = arena.interner.intern("x");
        let let_stmt = arena.alloc_node(NodeKind::LetStmt, 0, ByteSpan::new(0, 15), sym_x.0, add.0);
        let root = arena.alloc_list_node(NodeKind::Program, 0, ByteSpan::new(0, 15), &[let_stmt]);
        arena.root = root;

        assert_eq!(
            arena.to_sexpr(root),
            "(Program (LetStmt x (BinaryExpr op=1 (Int 40) (Int 2))))"
        );
    }
}
