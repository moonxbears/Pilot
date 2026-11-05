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
    ConstructorDecl = 13,
    DestructorDecl = 14,
    ScopeDecl = 15,
    IndexerDecl = 16,
    DynamicMemberDecl = 17,
    OperatorDecl = 18,

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
            13 => Self::ConstructorDecl,
            14 => Self::DestructorDecl,
            15 => Self::ScopeDecl,
            16 => Self::IndexerDecl,
            17 => Self::DynamicMemberDecl,
            18 => Self::OperatorDecl,
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
            Self::ConstructorDecl => "ConstructorDecl",
            Self::DestructorDecl => "DestructorDecl",
            Self::ScopeDecl => "ScopeDecl",
            Self::IndexerDecl => "IndexerDecl",
            Self::DynamicMemberDecl => "DynamicMemberDecl",
            Self::OperatorDecl => "OperatorDecl",
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

    #[inline]
    pub const fn is_type_spec(self) -> bool {
        let v = self as u16;
        v >= 400 && v <= 409
    }

    #[inline]
    pub const fn is_expr(self) -> bool {
        let v = self as u16;
        (v >= 200 && v <= 349) && v != Self::MatchArm as u16
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
    Send = 27,           // ch <- val
    Union = 28,          // a union b / a union all b
    Intersect = 29,      // a intersect b
    Except = 30,         // a except b
    PipeBackward = 31,   // a <| b
    As = 32,             // a as Target (Tier A bitcast / cast)
    AsView = 33,         // a as view Target (Tier B compile-time offset swizzling)
    Spaceship = 34,      // a <=> b (3-way comparison)
    SatAdd = 35,         // a +| b (saturating add)
    SatSub = 36,         // a -| b (saturating sub)
    SatMul = 37,         // a *| b (saturating mul)
    WrapAdd = 38,        // a +% b (wrapping add)
    WrapSub = 39,        // a -% b (wrapping sub)
    WrapMul = 40,        // a *% b (wrapping mul)
    CheckedAdd = 41,     // a +? b (checked add -> T?)
    CheckedSub = 42,     // a -? b (checked sub -> T?)
    CheckedMul = 43,     // a *? b (checked mul -> T?)
    EucMod = 44,         // a %% b (Euclidean modulo)
    Lens = 45,           // a ~> .field / a ~> (.k, .v) / a ~> (.2, .1, .0)
    OpenRange = 46,      // a >..< b (exclusive-both range)
    AfterRange = 47,     // a >.. (exclusive-start range)
    Swap = 48,           // a :=: b (in-place Perceus swap)
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
            Self::Send => "<-",
            Self::Union => "union",
            Self::Intersect => "intersect",
            Self::Except => "except",
            Self::PipeBackward => "<|",
            Self::As => "as",
            Self::AsView => "as view",
            Self::Spaceship => "<=>",
            Self::SatAdd => "+|",
            Self::SatSub => "-|",
            Self::SatMul => "*|",
            Self::WrapAdd => "+%",
            Self::WrapSub => "-%",
            Self::WrapMul => "*%",
            Self::CheckedAdd => "+?",
            Self::CheckedSub => "-?",
            Self::CheckedMul => "*?",
            Self::EucMod => "%%",
            Self::Lens => "~>",
            Self::OpenRange => ">..<",
            Self::AfterRange => ">..",
            Self::Swap => ":=:",
        }
    }
}

/// Milestone 30 & 31: Canonical Operator and Protocol Kind for `OperatorDecl`.
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum OperatorKind {
    BinaryLeft = 1,
    BinaryRight = 2,
    CompoundMut = 3,
    UnaryPrefix = 4,
    Equality = 5,
    FunctorCall = 6,
    Membership = 7,
    TypeGuard = 8,
    ImplicitCast = 9,
    ExplicitCast = 10,
    FormattedStr = 11,
    Deconstruct = 12,
    Clone = 13,
    Hash = 14,
    Pack = 15,
    Unpack = 16,
    SyncIterator = 17,
    AsyncFuture = 18,
    AsyncStream = 19,
    DeepFlatten = 20,
}

pub const UNARY_OP_SPLAT: u16 = 6;
pub const UNARY_OP_DOUBLE_SPLAT: u16 = 7;
pub const UNARY_OP_DEEP_FLATTEN: u16 = 8;

/// 10-Tier Spatial Visibility Lattice (Axis 1).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum VisibilityTier {
    Private = 0,           // priv / private (Default for functions/methods)
    FilePrivate = 1,       // file / fileprivate
    PrivateProtected = 2,  // priv prot / private protected
    Protected = 3,         // prot / protected
    Package = 4,           // pkg / package
    ProtectedInternal = 5, // prot int / protected internal
    Internal = 6,          // int / internal (Default for types/traits)
    Public = 7,            // pub / public
    Open = 8,              // open
    Export = 9,            // export
}

impl VisibilityTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::FilePrivate => "fileprivate",
            Self::PrivateProtected => "private protected",
            Self::Protected => "protected",
            Self::Package => "package",
            Self::ProtectedInternal => "protected internal",
            Self::Internal => "internal",
            Self::Public => "public",
            Self::Open => "open",
            Self::Export => "export",
        }
    }
}

/// Parsed prefix bracket attribute `[Name(arg1, arg2)]` or `[var contents = Name(arg1, arg2)]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AstAttribute {
    pub name: SymbolId,
    pub args: Vec<NodeId>,
    pub binding_sym: Option<SymbolId>, // Some("contents") for `[var contents = Patch(...)]`
    pub is_mutable_binding: bool,      // true for `var`, false for `let`
    pub span: ByteSpan,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CaptureMode {
    ByImmRef,  // Read-only capture of outer local/parameter (`ref<T>`)
    ByVarRef,  // Mutating capture of outer `var` local/parameter (`var ref<T>`)
    ByIsoMove, // Explicit move of an `iso` `0b10` `IObject`
}

#[derive(Clone, Debug)]
pub struct CapturedVar {
    pub sym: SymbolId,
    pub ty: u32,                       // TypeId inner
    pub mode: CaptureMode,
    pub outer_slot_coord: (u16, u16),  // `(frame_depth, slot_index)`
    pub span: ByteSpan,
}

#[derive(Clone, Debug)]
pub struct LocalFnCaptureSpec {
    pub fn_node: NodeId,
    pub enclosing_fn_node: NodeId,
    pub lifted_symbol: SymbolId,       // `__local_<outer>_<inner>_<id>`
    pub captures: Vec<CapturedVar>,
    pub lattice_bits: u8,              // `0b00` if `captures.is_empty()`, else `0b01` (`IReference`)
    pub escapes_as_first_class_value: bool,
}

/// Compile-Time Clearance Level (Axis 2 ? `lvl(N)` / `lvlN`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ClearanceLevel {
    pub level: u32,
    pub domain: Option<SymbolId>,
}

/// Flat Sibling Setter Clause (Variation B).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetterClause {
    pub param_name: SymbolId,
    pub param_type: NodeId,
    pub guard_expr: NodeId,
    pub is_iso: bool,
    pub generic_param: Option<SymbolId>,
    pub generic_bound: NodeId,
    pub body: NodeId,
    pub span: ByteSpan,
    pub visibility: Option<VisibilityTier>,
    pub clearance: Option<ClearanceLevel>,
}

/// Clauses for properties, indexers (`self[...]`), and dynamic member dispatch (`self.(...)`).
#[derive(Clone, Debug, Default)]
pub struct AccessorClauses {
    pub get_body: NodeId,
    pub get_visibility: Option<VisibilityTier>,
    pub get_clearance: Option<ClearanceLevel>,
    pub set_param: Option<(SymbolId, NodeId)>,
    pub set_body: NodeId,
    pub default_body: NodeId,
    pub setters: Vec<SetterClause>,
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

/// Structural AST context bitflags computed per token / node.
/// Mirrors `stdlib/core/meta.pilot`'s `AstContext`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AstContext(pub u32);

impl AstContext {
    pub const NONE: u32 = 0;
    pub const IS_DECLARATION: u32      = 1 << 0;  // Token is at its declaration site
    pub const IS_CALL_CALLEE: u32      = 1 << 1;  // Followed by `(...)` or `{ field: val }`
    pub const IS_TYPE_ANNOTATION: u32  = 1 << 2;  // Inside `: Type`, `<Type>`, or `let Type { ... }` header
    pub const IS_MEMBER_ACCESS: u32    = 1 << 3;  // RHS of `.`, `?.`, `..`, or `~>`
    pub const IS_PROP_ACCESSOR: u32    = 1 << 4;  // Inside `get` or `set` body of a property/indexer
    pub const IS_ATTR_PREFIX: u32      = 1 << 5;  // Inside `[Attr(...)]` or `[var x = Attr(...)]`
    pub const IS_BUNDLE_SPLICE: u32    = 1 << 6;  // Target of `*Bundle`, `*Facet`, or `*myBlock`
    pub const IS_VARIANT_CTOR: u32     = 1 << 7;  // Preceded by `::` or resolving to ADT/Outcome variant
    pub const IS_QUOTE_SPLICE: u32     = 1 << 8;  // `$ident` inside `quote { ... }` or `fn!!(...)`
    pub const IS_QUOTE_TYPE_PARAM: u32 = 1 << 9;  // `$T` where enclosing `syntax` param is `: type`
    pub const IS_UNQUOTE_ESCAPE: u32   = 1 << 10; // `~expr` escape inside `quote { ... }`
    pub const IS_MODIFIER_POS: u32     = 1 << 11; // Declaration modifier position (`pub`, `var`, `lvl30`)
    pub const IS_MUTATING_TARGET: u32  = 1 << 12; // `var` binding, `var ref`, `var fn`, or LHS of `=`
    pub const IS_CALLABLE_OBJECT: u32  = 1 << 13; // Variable/param/field whose type is `fn<A, R>` or has `op ()`
    pub const IS_METHOD_SYMBOL: u32    = 1 << 14; // Resolves in receiver's 4-tier `MethodTable` or `ext`
    pub const IS_FIELD_OR_PROP: u32    = 1 << 15; // Resolves in receiver's `fields` or `property_clauses`
    pub const IS_TYPE_SYMBOL: u32      = 1 << 16; // Resolves in `TypeArena` (`struct`, `interface`, `[Lang]`)
    pub const IS_LANG_PRIMITIVE: u32   = 1 << 17; // Annotated with `[Lang(...)]` / scalar register physics
    pub const IS_SYNTAX_MACRO: u32     = 1 << 18; // Registered in `ParserTables.syntax_macros_by_name`
    pub const IS_UNIT_DIMENSION: u32   = 1 << 19; // Resolves in `DimensionChecker` (`100[m/s]`)
    pub const IS_CONTEXTUAL_FIELD: u32 = 1 << 20; // Matches `Cfg.Tokens.kw_field` symbol
    pub const IS_CONTEXTUAL_VALUE: u32 = 1 << 21; // Matches `Cfg.Tokens.kw_value` symbol
    pub const IS_RAW_BACKTICK: u32     = 1 << 22; // Escaped `` `ident` `` (never classify as keyword!)
    pub const IS_TYPE_OPERATOR: u32    = 1 << 23; // Symbolic operator inside `TypeSpec` (`T@N`, `T[:]`, `T^`, `@T`, `T!E`)
    pub const IS_LOCAL_CAPTURE: u32    = 1 << 24; // Outer variable captured inside an embedded local `fn`

    #[inline]
    pub const fn empty() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    #[inline]
    pub const fn bits(self) -> u32 {
        self.0
    }

    #[inline]
    pub const fn contains(self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }

    #[inline]
    pub const fn with(self, flag: u32) -> Self {
        Self(self.0 | flag)
    }

    #[inline]
    pub fn insert(&mut self, flag: u32) {
        self.0 |= flag;
    }

    #[inline]
    pub fn remove(&mut self, flag: u32) {
        self.0 &= !flag;
    }
}

/// Provenance tracking across macro expansions and `[Patch]` AST transforms.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExpansionOrigin {
    Source,
    MacroExpansion { macro_id: NodeId, call_site: ByteSpan },
    PatchExpansion { patch_id: NodeId, original_site: ByteSpan },
}

/// Mirrors `stdlib/core/meta.pilot`'s `LspKind`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LspKind;

impl LspKind {
    pub const NAMESPACE_K: u32 = 0;
    pub const TYPE_K: u32 = 1;
    pub const CLASS_K: u32 = 2;
    pub const ENUM_K: u32 = 3;
    pub const INTERFACE_K: u32 = 4;
    pub const STRUCT_K: u32 = 5;
    pub const TYPE_PARAM_K: u32 = 6;
    pub const PARAM_K: u32 = 7;
    pub const VARIABLE_K: u32 = 8;
    pub const PROPERTY_K: u32 = 9;
    pub const ENUM_MEM_K: u32 = 10;
    pub const EVENT_K: u32 = 11;
    pub const FUNCTION_K: u32 = 12;
    pub const METHOD_K: u32 = 13;
    pub const MACRO_K: u32 = 14;
    pub const KEYWORD_K: u32 = 15;
    pub const MODIFIER_K: u32 = 16;
    pub const COMMENT_K: u32 = 17;
    pub const STRING_K: u32 = 18;
    pub const NUMBER_K: u32 = 19;
    pub const REGEXP_K: u32 = 20;
    pub const OPERATOR_K: u32 = 21;
    pub const DECORATOR_K: u32 = 22;
}

/// Mirrors `stdlib/core/meta.pilot`'s `LspMod`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LspMod;

impl LspMod {
    pub const NONE: u32 = 0;
    pub const DECLARATION: u32 = 1 << 0;
    pub const DEFINITION: u32 = 1 << 1;
    pub const READONLY: u32 = 1 << 2;
    pub const STATIC_M: u32 = 1 << 3;
    pub const DEPRECATED: u32 = 1 << 4;
    pub const ABSTRACT_M: u32 = 1 << 5;
    pub const ASYNC_M: u32 = 1 << 6;
    pub const MODIFICATION: u32 = 1 << 7;
    pub const DOC_M: u32 = 1 << 8;
    pub const DEFAULT_LIB: u32 = 1 << 9;
    pub const CONSTRUCTOR: u32 = 1 << 10;
    pub const CALLABLE: u32 = 1 << 11;
}

/// Mirrors `stdlib/core/meta.pilot`'s `IdeContextPolicy`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IdeContextPolicy;

impl IdeContextPolicy {
    pub fn classify(
        inherited_kind: u32,
        inherited_mods: u32,
        ctx: u32,
        lattice_bits: u8,
    ) -> (u32, u32) {
        let mut mods = inherited_mods;
        if (ctx & AstContext::IS_DECLARATION) != 0 {
            mods |= LspMod::DECLARATION;
        }
        if (ctx & AstContext::IS_MUTATING_TARGET) != 0 {
            mods = (mods & !LspMod::READONLY) | LspMod::MODIFICATION;
        }
        if (ctx & AstContext::IS_LOCAL_CAPTURE) != 0 {
            mods |= LspMod::STATIC_M;
        }

        // Rule 1: Type-Position Symbolic Operators (`T@N`, `T[:]`, `T^`, `@T`, `T?`, `T!E`, `(A)=>R`)
        if (ctx & AstContext::IS_TYPE_OPERATOR) != 0 {
            return (LspKind::OPERATOR_K, mods | LspMod::DEFINITION);
        }

        // Rule 2: Homoiconic `quote { ... }` Splices (`$T` vs `$x`) & `~unquote` Escapes
        if (ctx & AstContext::IS_QUOTE_SPLICE) != 0 {
            if (ctx & AstContext::IS_QUOTE_TYPE_PARAM) != 0 {
                return (LspKind::TYPE_PARAM_K, mods | LspMod::STATIC_M);
            }
            return (LspKind::PARAM_K, mods | LspMod::STATIC_M);
        }
        if (ctx & AstContext::IS_UNQUOTE_ESCAPE) != 0 {
            return (LspKind::MACRO_K, mods | LspMod::STATIC_M);
        }

        // Rule 3: Variation B Contextual `field` and `value` Inside Property Accessors
        if ((ctx & AstContext::IS_PROP_ACCESSOR) != 0) && ((ctx & AstContext::IS_MEMBER_ACCESS) == 0) {
            if (ctx & AstContext::IS_CONTEXTUAL_FIELD) != 0 {
                return (LspKind::PROPERTY_K, mods | LspMod::DEFAULT_LIB | LspMod::MODIFICATION);
            }
            if (ctx & AstContext::IS_CONTEXTUAL_VALUE) != 0 {
                return (LspKind::PARAM_K, mods | LspMod::DEFAULT_LIB | LspMod::READONLY);
            }
        }

        // Rule 4: `[...]` Attribute Prefix (`[Clamp]`, `[var c = Patch(orig, ...)]`)
        if (ctx & AstContext::IS_ATTR_PREFIX) != 0 {
            if (ctx & AstContext::IS_TYPE_SYMBOL) != 0 {
                return (LspKind::DECORATOR_K, mods);
            }
        }

        // Rule 5: `*Bundle`, `*Facet`, & `*myBlock` Splices
        if (ctx & AstContext::IS_BUNDLE_SPLICE) != 0 {
            return (inherited_kind, mods | LspMod::STATIC_M);
        }

        // Rule 6: Dimensional Unit Suffixes (`100[m/s]`)
        if (ctx & AstContext::IS_UNIT_DIMENSION) != 0 {
            return (LspKind::TYPE_PARAM_K, mods | LspMod::DEFAULT_LIB);
        }

        // Rule 7: Variant Constructors (`::ok(v)`, `::some(x)`, `Shape.Circle(r)`)
        if (ctx & AstContext::IS_VARIANT_CTOR) != 0 {
            return (LspKind::ENUM_MEM_K, mods | LspMod::READONLY);
        }

        // Rule 8: Call Target Disambiguation (`Point(1, 2)` vs `closure_var(1)` vs `syntax_macro(1)` vs `fn(1)`)
        if (ctx & AstContext::IS_CALL_CALLEE) != 0 {
            if ((ctx & AstContext::IS_SYNTAX_MACRO) != 0) && ((ctx & AstContext::IS_RAW_BACKTICK) == 0) {
                return (LspKind::MACRO_K, mods);
            }
            if (ctx & AstContext::IS_TYPE_SYMBOL) != 0 {
                if (ctx & AstContext::IS_LANG_PRIMITIVE) != 0 {
                    return (LspKind::TYPE_K, mods | LspMod::DEFAULT_LIB);
                }
                let ctor_kind = if (lattice_bits & 0b10) != 0 {
                    LspKind::CLASS_K
                } else {
                    LspKind::STRUCT_K
                };
                return (ctor_kind, mods | LspMod::CONSTRUCTOR);
            }
            if (ctx & AstContext::IS_CALLABLE_OBJECT) != 0 {
                let var_kind = if (ctx & AstContext::IS_FIELD_OR_PROP) != 0 {
                    LspKind::PROPERTY_K
                } else {
                    inherited_kind
                };
                return (var_kind, mods | LspMod::CALLABLE);
            }
            if (ctx & AstContext::IS_METHOD_SYMBOL) != 0 {
                return (LspKind::METHOD_K, mods);
            }
            return (LspKind::FUNCTION_K, mods);
        }

        // Rule 9: Member Access (`obj.m`, `.m` PrefixDot, `node..m` Cascade, `xs ~> .m` Lens)
        if (ctx & AstContext::IS_MEMBER_ACCESS) != 0 {
            if (ctx & AstContext::IS_METHOD_SYMBOL) != 0 {
                return (LspKind::METHOD_K, mods);
            }
            if (ctx & AstContext::IS_FIELD_OR_PROP) != 0 {
                return (LspKind::PROPERTY_K, mods);
            }
        }

        // Rule 10: Type Annotations & Distributive `let Type<T> { ... }` Headers
        if ((ctx & AstContext::IS_TYPE_ANNOTATION) != 0) || ((ctx & AstContext::IS_TYPE_SYMBOL) != 0) {
            if (ctx & AstContext::IS_LANG_PRIMITIVE) != 0 {
                return (LspKind::TYPE_K, mods | LspMod::DEFAULT_LIB);
            }
            if inherited_kind != 0 {
                return (inherited_kind, mods);
            }
            let ty_kind = if (lattice_bits & 0b10) != 0 {
                LspKind::CLASS_K
            } else {
                LspKind::STRUCT_K
            };
            return (ty_kind, mods);
        }

        // Rule 11: Declaration Modifiers (`pub`..`export`, `var`, `pure`, `lvl30`) vs Expression `lvl`
        if (ctx & AstContext::IS_MODIFIER_POS) != 0 {
            if (ctx & AstContext::IS_RAW_BACKTICK) == 0 {
                return (LspKind::MODIFIER_K, mods | LspMod::STATIC_M);
            }
        }

        // Rule 12: User or Core `syntax` Macro Invocations Without `()` (`break;`, `continue;`)
        if ((ctx & AstContext::IS_SYNTAX_MACRO) != 0) && ((ctx & AstContext::IS_RAW_BACKTICK) == 0) {
            return (LspKind::MACRO_K, mods);
        }

        (inherited_kind, mods)
    }
}

/// Mirrors `stdlib/core/meta.pilot`'s `IdePresentationPolicy`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IdePresentationPolicy;

impl IdePresentationPolicy {
    #[inline]
    pub fn lattice_badge(bits: u8) -> &'static str {
        match bits {
            0b00 => "0b00 IValue (Trivial Copy)",
            0b01 => "0b01 IReference (Lexical Borrow)",
            0b10 => "0b10 IObject (Perceus Move + ~self)",
            _ => "0b11 IGuard (Scoped RAII + Borrow)",
        }
    }

    #[inline]
    pub fn format_type_hover(name: &str, byte_size: usize, byte_align: usize, bits: u8) -> String {
        format!(
            "**{}** — `{} bytes` (`align {}`) | `{}`",
            name,
            byte_size,
            byte_align,
            Self::lattice_badge(bits)
        )
    }

    #[inline]
    pub fn format_capture_inlay_hint(captures_csv: &str) -> String {
        format!("/* captures: {} */", captures_csv)
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
    /// Optional completion `then` clause expression `NodeId` for `ForExpr` nodes (`for ... { ... } then <expr>`).
    pub for_then_nodes: std::collections::HashMap<u32, NodeId>,
    /// Declared `(field_name, field_type_node)` pairs for `RecordDecl`, `StructDecl`, `ClassDecl`, and `InterfaceDecl`.
    pub record_fields: std::collections::HashMap<u32, Vec<(SymbolId, NodeId)>>,
    /// Optional connection / scope argument `NodeId` for `PolyglotBlock` (e.g. `postgresql(conn) { ... }` or `sql(conn) { ... }`).
    pub polyglot_conns: std::collections::HashMap<u32, NodeId>,
    /// Constructor chaining target `NodeId` for `ConstructorDecl` (`self(params...) -> self(other...)`).
    pub constructor_chains: std::collections::HashMap<u32, NodeId>,
    /// Scoped context manager `(enter_body, exit_body)` for `ScopeDecl` (`scope self { enter: ... exit: ... }`).
    pub scope_clauses: std::collections::HashMap<u32, (NodeId, NodeId)>,
    /// Accessor clauses for `IndexerDecl` (`self[...]`) and `DynamicMemberDecl` (`self.(...)`).
    pub accessor_clauses: std::collections::HashMap<u32, AccessorClauses>,
    /// Operator symbol or identifier for `OperatorDecl`.
    pub operator_symbols: std::collections::HashMap<u32, SymbolId>,
    pub static_methods: std::collections::HashSet<u32>,
    pub pure_methods: std::collections::HashSet<u32>,
    pub mut_methods: std::collections::HashSet<u32>,
    pub node_visibility: std::collections::HashMap<u32, VisibilityTier>,
    pub node_clearance: std::collections::HashMap<u32, ClearanceLevel>,
    pub node_attributes: std::collections::HashMap<u32, Vec<AstAttribute>>,
    pub comptime_fns: std::collections::HashSet<u32>,
    pub partial_types: std::collections::HashSet<u32>,
    pub pkg_decls: std::collections::HashSet<u32>,
    pub clearance_exprs: std::collections::HashMap<u32, NodeId>,
    pub operator_bodies: std::collections::HashMap<u32, NodeId>,
    pub operator_params: std::collections::HashMap<u32, Vec<NodeId>>,
    /// Generic type parameter names `<T, U>` declared on `StructDecl` or `InterfaceDecl`.
    pub struct_type_params: std::collections::HashMap<u32, Vec<SymbolId>>,
    /// Inherited base type nodes `: IBase1, IBase2` declared on `StructDecl` or `InterfaceDecl`.
    pub struct_base_types: std::collections::HashMap<u32, Vec<NodeId>>,
    /// Generic base symbol for parameterized type nodes (`Name<T, U>`).
    pub type_base_symbols: std::collections::HashMap<u32, SymbolId>,
    /// Statement block splice renames (`*AuditBlock with { old as new }`).
    pub statement_splice_renames: std::collections::HashMap<u32, Vec<(SymbolId, SymbolId)>>,
    /// Extension method declarations (`ext fn Type.method()`).
    pub extension_methods: std::collections::HashMap<u32, (SymbolId, SymbolId)>,
    /// Named extension group declarations (`ext ExtName on Type { ... }`).
    pub named_extension_groups: std::collections::HashMap<u32, SymbolId>,
    /// Lexical `use ext ExtName { ... }` block scopes.
    pub lexical_use_ext_scopes: std::collections::HashMap<u32, SymbolId>,
    /// Typestate variant declarations (`state ConnState { Disconnected, Connected }`).
    pub state_typestates: std::collections::HashMap<u32, Vec<SymbolId>>,
    /// Refinement type declarations (`type Sub = Base where <pred>`).
    pub refinement_predicates: std::collections::HashMap<u32, (NodeId, NodeId)>,
    /// `.pchor` cue declarations.
    pub cue_declarations: std::collections::HashSet<u32>,
    /// Capture specifications for embedded local functions.
    pub local_fn_captures: std::collections::HashMap<u32, LocalFnCaptureSpec>,
    /// Structural AST context bitflags computed per node.
    pub node_contexts: std::collections::HashMap<u32, AstContext>,
    /// Homoiconic quote depth per node.
    pub node_quote_depths: std::collections::HashMap<u32, u32>,
    /// Provenance origin across macro and patch expansions.
    pub node_expansion_origins: std::collections::HashMap<u32, ExpansionOrigin>,
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
            for_then_nodes: std::collections::HashMap::new(),
            record_fields: std::collections::HashMap::new(),
            polyglot_conns: std::collections::HashMap::new(),
            constructor_chains: std::collections::HashMap::new(),
            scope_clauses: std::collections::HashMap::new(),
            accessor_clauses: std::collections::HashMap::new(),
            operator_symbols: std::collections::HashMap::new(),
            static_methods: std::collections::HashSet::new(),
            mut_methods: std::collections::HashSet::new(),
            pure_methods: std::collections::HashSet::new(),
            node_visibility: std::collections::HashMap::new(),
            node_clearance: std::collections::HashMap::new(),
            node_attributes: std::collections::HashMap::new(),
            comptime_fns: std::collections::HashSet::new(),
            partial_types: std::collections::HashSet::new(),
            pkg_decls: std::collections::HashSet::new(),
            clearance_exprs: std::collections::HashMap::new(),
            operator_bodies: std::collections::HashMap::new(),
            operator_params: std::collections::HashMap::new(),
            struct_type_params: std::collections::HashMap::new(),
            struct_base_types: std::collections::HashMap::new(),
            type_base_symbols: std::collections::HashMap::new(),
            statement_splice_renames: std::collections::HashMap::new(),
            extension_methods: std::collections::HashMap::new(),
            named_extension_groups: std::collections::HashMap::new(),
            lexical_use_ext_scopes: std::collections::HashMap::new(),
            state_typestates: std::collections::HashMap::new(),
            refinement_predicates: std::collections::HashMap::new(),
            cue_declarations: std::collections::HashSet::new(),
            local_fn_captures: std::collections::HashMap::new(),
            node_contexts: std::collections::HashMap::new(),
            node_quote_depths: std::collections::HashMap::new(),
            node_expansion_origins: std::collections::HashMap::new(),
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
