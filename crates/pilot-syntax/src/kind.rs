//! Milestone 31: Unified SyntaxKind covering tokens, trivia, missing tokens, and AST composite nodes.

use pilot_ast::NodeKind;
use pilot_lexer::TokenKind;

/// 16-bit representation of every lexical token, trivia element, synthesized missing token,
/// and composite syntax node in Pilot's Concrete Syntax Tree (CST).
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SyntaxKind {
    // --- Special & Trivia Tokens (0..19) ---
    Eof = 0,
    Whitespace = 1,
    Newline = 2,
    LineComment = 3,
    BlockComment = 4,
    Shebang = 5,
    RegionStart = 6,
    RegionEnd = 7,
    ErrorToken = 8,
    VirtualLBrace = 9,
    VirtualRBrace = 10,

    // --- Synthesized Missing Tokens for IDE Completion (20..29) ---
    MissingIdent = 20,
    MissingRParen = 21,
    MissingRBrace = 22,
    MissingRBracket = 23,
    MissingSemicolon = 24,
    MissingExpr = 25,
    MissingType = 26,

    // --- Identifiers & Atomic Literals (30..69) ---
    Ident = 30,
    KeywordLit = 31,
    IntLit = 32,
    HexIntLit = 33,
    BinIntLit = 34,
    OctIntLit = 35,
    FloatLit = 36,
    StrLit = 37,
    TripleStrLit = 38,
    FStrStart = 39,
    FStrMiddle = 40,
    FStrEnd = 41,
    DurationLit = 42,
    FrequencyLit = 43,
    MemoryLit = 44,
    RawDslBlock = 45,

    // --- Strict Keywords (70..139) ---
    KwEngine = 67,
    KwAsm = 68,
    KwLoop = 69,
    KwLet = 70,
    KwVar = 71,
    KwConst = 72,
    KwDef = 73,
    KwFn = 74,
    KwClass = 75,
    KwStruct = 76,
    KwRecord = 77,
    KwInterface = 78,
    KwProtocol = 79,
    KwEnum = 80,
    KwActor = 81,
    KwType = 82,
    KwRef = 83,
    KwDynamic = 84,
    KwSymbol = 85,
    KwPure = 86,
    KwComptime = 87,
    KwMut = 88,
    KwImmut = 89,
    KwProperty = 90,
    KwGet = 91,
    KwOperator = 92,
    KwExtension = 93,
    KwExtend = 94,
    KwState = 95,
    KwIf = 96,
    KwElif = 97,
    KwElse = 98,
    KwThen = 99,
    KwWhile = 100,
    KwFor = 101,
    KwIn = 102,
    KwMatch = 103,
    KwCase = 104,
    KwWhen = 105,
    KwReturn = 106,
    KwYield = 107,
    KwBreak = 108,
    KwContinue = 109,
    KwPass = 110,
    KwRaise = 111,
    KwTry = 112,
    KwCatch = 113,
    KwFinally = 114,
    KwWith = 115,
    KwAs = 116,
    KwAssert = 117,
    KwImport = 118,
    KwFrom = 119,
    KwGlobal = 120,
    KwLambda = 121,
    KwAnd = 122,
    KwOr = 123,
    KwNot = 124,
    KwIs = 125,
    KwGuard = 126,
    KwDefer = 127,
    KwContext = 128,
    KwRequires = 129,
    KwEnsures = 130,

    // --- Contextual Keywords (140..259) ---
    KwAsync = 140,
    KwAwait = 141,
    KwSpawn = 142,
    KwParallel = 143,
    KwChan = 144,
    KwChannel = 145,
    KwAtomic = 146,
    KwSync = 147,
    KwLock = 148,
    KwIsolate = 149,
    KwRealtime = 150,
    KwDeadline = 151,
    KwOnMissed = 152,
    KwWithin = 153,
    KwFallback = 154,
    KwPeriod = 155,
    KwPeriodic = 156,
    KwPriority = 157,
    KwJitter = 158,
    KwBounded = 159,
    KwNoalloc = 160,
    KwStackalloc = 161,
    KwRegion = 162,
    KwWcet = 163,
    KwHardRt = 164,
    KwSoftRt = 165,
    KwPublic = 166,
    KwPrivate = 167,
    KwProtected = 168,
    KwInternal = 169,
    KwStatic = 170,
    KwPartial = 171,
    KwSteal = 172,
    KwContracts = 173,
    KwDelegate = 174,
    KwParams = 175,
    KwArgs = 176,
    KwKwargs = 177,
    KwNameof = 178,
    KwEach = 179,
    KwRepeat = 180,
    KwAdapter = 181,
    KwConverter = 182,
    KwDefault = 183,
    KwTimeout = 184,
    KwSelect = 185,
    KwWhere = 186,
    KwJoin = 187,
    KwInner = 188,
    KwLeft = 189,
    KwRight = 190,
    KwFull = 191,
    KwCross = 192,
    KwOuter = 193,
    KwOn = 194,
    KwGroup = 195,
    KwBy = 196,
    KwHaving = 197,
    KwOrder = 198,
    KwAsc = 199,
    KwDesc = 200,
    KwAscending = 201,
    KwDescending = 202,
    KwLimit = 203,
    KwOffset = 204,
    KwInto = 205,
    KwDistinct = 206,
    KwUnion = 207,
    KwAll = 208,
    KwIntersect = 209,
    KwExcept = 210,
    KwDifference = 211,
    KwIndex = 212,
    KwIndexed = 213,
    KwLookup = 214,
    KwTable = 215,
    KwSchema = 216,
    KwInsert = 217,
    KwUpdate = 218,
    KwDelete = 219,
    KwValues = 220,
    KwSet = 221,
    KwReturning = 222,
    KwConflict = 223,
    KwDo = 224,
    KwNothing = 225,
    KwOver = 226,
    KwPartition = 227,
    KwWindow = 228,
    KwTransaction = 229,
    KwCommit = 230,
    KwRollback = 231,
    KwSavepoint = 232,
    KwTo = 233,
    KwConnect = 234,
    KwDatabase = 235,
    KwModel = 236,
    KwDsl = 237,
    KwDomain = 238,
    KwUnit = 239,
    KwMacro = 240,
    KwDefmacro = 241,
    KwDefkeyword = 242,
    KwQuote = 243,
    KwQuasiquote = 244,
    KwSplice = 245,
    KwUnquote = 246,
    KwUnquoteSplicing = 247,
    KwExtern = 248,
    KwModule = 249,
    KwLib = 250,
    KwTrue = 251,
    KwFalse = 252,
    KwSome = 253,
    KwNone = 254,
    KwNull = 255,
    KwUndefined = 256,
    KwSelf = 257,
    KwOp = 258,
    KwScope = 259,
    KwEnter = 260,
    KwExit = 261,
    KwClone = 262,
    KwPack = 263,
    KwUnpack = 264,
    KwFlat = 265,

    // --- Delimiters & Single Character Punctuation (270..289) ---
    LParen = 270,
    RParen = 271,
    LBracket = 272,
    RBracket = 273,
    LBrace = 274,
    RBrace = 275,
    Colon = 276,
    Semicolon = 277,
    Comma = 278,
    InfixDot = 279,
    PrefixDot = 289,
    At = 280,
    Question = 281,
    Exclamation = 282,
    Tilde = 283,
    Dollar = 284,
    Backquote = 285,
    Underscore = 286,
    Walrus = 287,
    ArrowLeft = 288,

    // --- Operators & Arrows (290..349) ---
    Plus = 290,
    Minus = 291,
    Star = 292,
    Slash = 293,
    Percent = 294,
    DoubleStar = 295,
    DoubleSlash = 296,
    Equal = 297,
    EqEqual = 298,
    NotEqual = 299,
    Less = 300,
    LessEqual = 301,
    Greater = 302,
    GreaterEqual = 303,
    Ampersand = 304,
    Vbar = 305,
    Caret = 306,
    ShiftLeft = 307,
    ShiftRight = 308,
    PlusEqual = 309,
    MinusEqual = 310,
    StarEqual = 311,
    SlashEqual = 312,
    PercentEqual = 313,
    DoubleStarEqual = 314,
    DoubleSlashEqual = 315,
    AmperEqual = 316,
    VbarEqual = 317,
    CaretEqual = 318,
    ShiftLeftEqual = 319,
    ShiftRightEqual = 320,
    CoalesceEqual = 321,
    PipeForward = 322,
    PipeBackward = 323,
    Cascade = 324,
    SafeNav = 325,
    Coalesce = 326,
    Arrow = 327,
    FatArrow = 328,
    JsonGetText = 329,
    JsonContains = 330,
    JsonContained = 331,
    DotDot = 332,
    DotDotEqual = 333,
    DotDotLess = 334,
    Ellipsis = 335,
    ColonColon = 336,
    StarDotDot = 337,
    Spaceship = 338,
    PlusPipe = 339,
    MinusPipe = 340,
    StarPipe = 341,
    PlusPercent = 342,
    MinusPercent = 343,
    StarPercent = 344,
    PlusQuestion = 345,
    MinusQuestion = 346,
    StarQuestion = 347,
    DoublePercent = 348,
    TildeGreater = 349,
    GreaterDotDotLess = 356,
    GreaterDotDot = 357,
    SwapEqual = 358,

    // --- Call Quotes (350..355) ---
    CallQuoteShallow = 350,
    CallQuoteDeep = 351,
    CallQuoteAbi = 352,
    CallQuoteReact = 353,
    CallQuoteAdjoint = 354,
    CallQuoteEffect = 355,

    // --- Composite Syntax Nodes (400..499) ---
    Program = 400,
    FnDecl = 401,
    ClassDecl = 402,
    StructDecl = 403,
    RecordDecl = 404,
    InterfaceDecl = 405,
    ExtendDecl = 406,
    TypeAdtDecl = 407,
    ImportDecl = 408,
    FromImportDecl = 409,
    ConstDecl = 410,
    Param = 411,
    ParamList = 412,
    TypeParam = 413,
    TypeParamList = 414,
    ConstructorDecl = 415,
    DestructorDecl = 416,
    ScopeDecl = 417,
    IndexerDecl = 418,
    DynamicMemberDecl = 419,
    OperatorDecl = 420,

    // --- Statements (430..459) ---
    LetStmt = 430,
    VarStmt = 431,
    ReturnStmt = 432,
    YieldStmt = 433,
    BreakStmt = 434,
    ContinueStmt = 435,
    GuardStmt = 436,
    DeferStmt = 437,
    WhileStmt = 438,
    ForStmt = 439,
    ExprStmt = 440,

    // --- Expressions (460..499) ---
    BinaryExpr = 460,
    UnaryExpr = 461,
    CallExpr = 462,
    ArgList = 463,
    MemberExpr = 464,
    IndexExpr = 465,
    TryPropagateExpr = 466,
    BlockExpr = 467,
    IfExpr = 468,
    MatchExpr = 469,
    MatchArm = 470,
    ListExpr = 471,
    MapExpr = 472,
    TupleExpr = 473,
    ParenExpr = 474,
    LambdaExpr = 475,
    SwitchStmt = 476,
    SwitchArm = 477,
    AttrPipeline = 478,
    DistributiveBlock = 479,
    DistributiveHeader = 480,

    // --- Patterns & Types (500..529) ---
    PatWildcard = 500,
    PatBind = 501,
    PatLiteral = 502,
    PatTuple = 503,
    PatList = 504,
    PatOr = 505,

    TypeNamed = 510,
    TypeOptional = 511,
    TypeFallible = 512,
    TypeUnion = 513,
    TypeIntersection = 514,
    TypeFn = 515,
    TypeList = 516,
    TypeMap = 517,
    TypeTuple = 518,

    // --- Error Node (600) ---
    ErrorNode = 600,
}

impl SyntaxKind {
    #[allow(non_upper_case_globals)]
    pub const Dot: Self = Self::InfixDot;

    #[inline]
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            SyntaxKind::Whitespace
                | SyntaxKind::Newline
                | SyntaxKind::LineComment
                | SyntaxKind::BlockComment
                | SyntaxKind::Shebang
                | SyntaxKind::RegionStart
                | SyntaxKind::RegionEnd
        )
    }

    #[inline]
    pub fn is_missing(self) -> bool {
        (self as u16) >= (SyntaxKind::MissingIdent as u16)
            && (self as u16) <= (SyntaxKind::MissingType as u16)
    }

    #[inline]
    pub fn is_token(self) -> bool {
        (self as u16) < (SyntaxKind::Program as u16)
    }

    #[inline]
    pub fn is_node(self) -> bool {
        (self as u16) >= (SyntaxKind::Program as u16)
    }

    #[inline]
    pub fn is_keyword(self) -> bool {
        (self as u16) >= (SyntaxKind::KwLet as u16)
            && (self as u16) <= (SyntaxKind::KwUnpack as u16)
    }

    #[inline]
    pub fn is_contextual_keyword(self) -> bool {
        (self as u16) >= (SyntaxKind::KwAsync as u16)
            && (self as u16) <= (SyntaxKind::KwUnpack as u16)
    }

    #[inline]
    pub fn is_modifier_keyword(self) -> bool {
        matches!(
            self,
            SyntaxKind::KwPure
                | SyntaxKind::KwComptime
                | SyntaxKind::KwMut
                | SyntaxKind::KwImmut
                | SyntaxKind::KwRef
                | SyntaxKind::KwDynamic
                | SyntaxKind::KwAsync
                | SyntaxKind::KwPublic
                | SyntaxKind::KwPrivate
                | SyntaxKind::KwProtected
                | SyntaxKind::KwInternal
                | SyntaxKind::KwStatic
                | SyntaxKind::KwPartial
                | SyntaxKind::KwExtern
                | SyntaxKind::KwAtomic
                | SyntaxKind::KwSync
                | SyntaxKind::KwRealtime
                | SyntaxKind::KwNoalloc
                | SyntaxKind::KwHardRt
                | SyntaxKind::KwSoftRt
        )
    }

    #[inline]
    pub fn is_ident_like(self) -> bool {
        self == SyntaxKind::Ident
            || self.is_contextual_keyword()
            || matches!(
                self,
                SyntaxKind::KwGet
                    | SyntaxKind::KwState
                    | SyntaxKind::KwContext
                    | SyntaxKind::KwProperty
                    | SyntaxKind::KwDynamic
                    | SyntaxKind::KwSymbol
                    | SyntaxKind::KwFrom
                    | SyntaxKind::KwType
            )
    }

    #[inline]
    pub fn is_operator(self) -> bool {
        ((self as u16) >= (SyntaxKind::Plus as u16) && (self as u16) <= (SyntaxKind::SwapEqual as u16))
            || matches!(
                self,
                SyntaxKind::Walrus
                    | SyntaxKind::ArrowLeft
                    | SyntaxKind::Question
                    | SyntaxKind::Exclamation
                    | SyntaxKind::Tilde
            )
    }

    pub fn from_token_kind(tk: TokenKind) -> Self {
        match tk {
            TokenKind::Eof => SyntaxKind::Eof,
            TokenKind::Newline => SyntaxKind::Newline,
            TokenKind::Comment => SyntaxKind::LineComment,
            TokenKind::BlockComment => SyntaxKind::BlockComment,
            TokenKind::Shebang => SyntaxKind::Shebang,
            TokenKind::ErrorToken => SyntaxKind::ErrorToken,

            TokenKind::Ident => SyntaxKind::Ident,
            TokenKind::KeywordLit => SyntaxKind::KeywordLit,
            TokenKind::IntLit => SyntaxKind::IntLit,
            TokenKind::HexIntLit => SyntaxKind::HexIntLit,
            TokenKind::BinIntLit => SyntaxKind::BinIntLit,
            TokenKind::OctIntLit => SyntaxKind::OctIntLit,
            TokenKind::FloatLit => SyntaxKind::FloatLit,
            TokenKind::StrLit => SyntaxKind::StrLit,
            TokenKind::TripleStrLit => SyntaxKind::TripleStrLit,
            TokenKind::RawDslBlock => SyntaxKind::RawDslBlock,

            TokenKind::KwEngine => SyntaxKind::KwEngine,
            TokenKind::KwStruct => SyntaxKind::KwStruct,
            TokenKind::KwInterface => SyntaxKind::KwInterface,
            TokenKind::KwFn => SyntaxKind::KwFn,
            TokenKind::KwLet => SyntaxKind::KwLet,
            TokenKind::KwRef => SyntaxKind::KwRef,
            TokenKind::KwIf => SyntaxKind::KwIf,
            TokenKind::KwLoop => SyntaxKind::KwLoop,
            TokenKind::KwReturn => SyntaxKind::KwReturn,
            TokenKind::KwYield => SyntaxKind::KwYield,
            TokenKind::KwDefer => SyntaxKind::KwDefer,
            TokenKind::KwComptime => SyntaxKind::KwComptime,
            TokenKind::KwQuote => SyntaxKind::KwQuote,
            TokenKind::KwAsm => SyntaxKind::KwAsm,

            TokenKind::LParen => SyntaxKind::LParen,
            TokenKind::RParen => SyntaxKind::RParen,
            TokenKind::LBracket => SyntaxKind::LBracket,
            TokenKind::RBracket => SyntaxKind::RBracket,
            TokenKind::LBrace => SyntaxKind::LBrace,
            TokenKind::RBrace => SyntaxKind::RBrace,
            TokenKind::Colon => SyntaxKind::Colon,
            TokenKind::DoubleColon => SyntaxKind::ColonColon,
            TokenKind::Semicolon => SyntaxKind::Semicolon,
            TokenKind::Comma => SyntaxKind::Comma,
            TokenKind::InfixDot => SyntaxKind::InfixDot,
            TokenKind::PrefixDot => SyntaxKind::PrefixDot,
            TokenKind::StarDotDot => SyntaxKind::StarDotDot,
            TokenKind::At => SyntaxKind::At,
            TokenKind::Question => SyntaxKind::Question,
            TokenKind::Exclamation => SyntaxKind::Exclamation,
            TokenKind::Tilde => SyntaxKind::Tilde,
            TokenKind::Dollar => SyntaxKind::Dollar,
            TokenKind::Walrus => SyntaxKind::Walrus,
            TokenKind::ArrowLeft => SyntaxKind::ArrowLeft,

            TokenKind::Plus => SyntaxKind::Plus,
            TokenKind::Minus => SyntaxKind::Minus,
            TokenKind::Star => SyntaxKind::Star,
            TokenKind::Slash => SyntaxKind::Slash,
            TokenKind::Percent => SyntaxKind::Percent,
            TokenKind::DoubleStar => SyntaxKind::DoubleStar,
            TokenKind::DoubleSlash => SyntaxKind::DoubleSlash,
            TokenKind::Equal => SyntaxKind::Equal,
            TokenKind::EqEqual => SyntaxKind::EqEqual,
            TokenKind::NotEqual => SyntaxKind::NotEqual,
            TokenKind::Less => SyntaxKind::Less,
            TokenKind::LessEqual => SyntaxKind::LessEqual,
            TokenKind::Greater => SyntaxKind::Greater,
            TokenKind::GreaterEqual => SyntaxKind::GreaterEqual,
            TokenKind::Amper => SyntaxKind::Ampersand,
            TokenKind::Vbar => SyntaxKind::Vbar,
            TokenKind::Caret => SyntaxKind::Caret,
            TokenKind::ShiftLeft => SyntaxKind::ShiftLeft,
            TokenKind::ShiftRight => SyntaxKind::ShiftRight,
            TokenKind::PlusEqual => SyntaxKind::PlusEqual,
            TokenKind::MinusEqual => SyntaxKind::MinusEqual,
            TokenKind::StarEqual => SyntaxKind::StarEqual,
            TokenKind::SlashEqual => SyntaxKind::SlashEqual,
            TokenKind::PercentEqual => SyntaxKind::PercentEqual,
            TokenKind::DoubleStarEqual => SyntaxKind::DoubleStarEqual,
            TokenKind::DoubleSlashEqual => SyntaxKind::DoubleSlashEqual,
            TokenKind::AmperEqual => SyntaxKind::AmperEqual,
            TokenKind::VbarEqual => SyntaxKind::VbarEqual,
            TokenKind::CaretEqual => SyntaxKind::CaretEqual,
            TokenKind::ShiftLeftEqual => SyntaxKind::ShiftLeftEqual,
            TokenKind::ShiftRightEqual => SyntaxKind::ShiftRightEqual,
            TokenKind::CoalesceEqual => SyntaxKind::CoalesceEqual,
            TokenKind::PipeForward => SyntaxKind::PipeForward,
            TokenKind::PipeBackward => SyntaxKind::PipeBackward,
            TokenKind::SafeCascade => SyntaxKind::Cascade,
            TokenKind::SafeNav => SyntaxKind::SafeNav,
            TokenKind::Coalesce => SyntaxKind::Coalesce,
            TokenKind::Arrow => SyntaxKind::Arrow,
            TokenKind::FatArrow => SyntaxKind::FatArrow,
            TokenKind::JsonGetText => SyntaxKind::JsonGetText,
            TokenKind::JsonContains => SyntaxKind::JsonContains,
            TokenKind::JsonContained => SyntaxKind::JsonContained,
            TokenKind::DotDot => SyntaxKind::DotDot,
            TokenKind::DotDotEqual => SyntaxKind::DotDotEqual,
            TokenKind::DotDotLess => SyntaxKind::DotDotLess,
            TokenKind::Ellipsis => SyntaxKind::Ellipsis,

            TokenKind::CallQuoteShallow => SyntaxKind::CallQuoteShallow,
            TokenKind::CallQuoteDeep => SyntaxKind::CallQuoteDeep,
            TokenKind::CallQuoteAbi => SyntaxKind::CallQuoteAbi,
            TokenKind::CallQuoteReact => SyntaxKind::CallQuoteReact,
            TokenKind::CallQuoteAdjoint => SyntaxKind::CallQuoteAdjoint,
            TokenKind::CallQuoteEffect => SyntaxKind::CallQuoteEffect,
            TokenKind::DoubleAmper => SyntaxKind::KwAnd,
            TokenKind::DoublePipe => SyntaxKind::KwOr,
            TokenKind::Spaceship => SyntaxKind::Spaceship,
            TokenKind::PlusPipe => SyntaxKind::PlusPipe,
            TokenKind::MinusPipe => SyntaxKind::MinusPipe,
            TokenKind::StarPipe => SyntaxKind::StarPipe,
            TokenKind::PlusPercent => SyntaxKind::PlusPercent,
            TokenKind::MinusPercent => SyntaxKind::MinusPercent,
            TokenKind::StarPercent => SyntaxKind::StarPercent,
            TokenKind::PlusQuestion => SyntaxKind::PlusQuestion,
            TokenKind::MinusQuestion => SyntaxKind::MinusQuestion,
            TokenKind::StarQuestion => SyntaxKind::StarQuestion,
            TokenKind::DoublePercent => SyntaxKind::DoublePercent,
            TokenKind::TildeGreater => SyntaxKind::TildeGreater,
            TokenKind::GreaterDotDotLess => SyntaxKind::GreaterDotDotLess,
            TokenKind::GreaterDotDot => SyntaxKind::GreaterDotDot,
            TokenKind::SwapEqual => SyntaxKind::SwapEqual,
        }
    }

    pub fn from_node_kind(nk: NodeKind) -> Self {
        match nk {
            NodeKind::Program => SyntaxKind::Program,
            NodeKind::FnDecl => SyntaxKind::FnDecl,
            NodeKind::StructDecl => SyntaxKind::StructDecl,
            NodeKind::InterfaceDecl => SyntaxKind::InterfaceDecl,
            NodeKind::ExtendDecl => SyntaxKind::ExtendDecl,
            NodeKind::TypeAdtDecl => SyntaxKind::TypeAdtDecl,
            NodeKind::ImportDecl => SyntaxKind::ImportDecl,
            NodeKind::ConstDecl => SyntaxKind::ConstDecl,
            NodeKind::ConstructorDecl => SyntaxKind::ConstructorDecl,
            NodeKind::DestructorDecl => SyntaxKind::DestructorDecl,
            NodeKind::ScopeDecl => SyntaxKind::ScopeDecl,
            NodeKind::IndexerDecl => SyntaxKind::IndexerDecl,
            NodeKind::DynamicMemberDecl => SyntaxKind::DynamicMemberDecl,
            NodeKind::OperatorDecl => SyntaxKind::OperatorDecl,
            NodeKind::LetStmt => SyntaxKind::LetStmt,
            NodeKind::VarStmt => SyntaxKind::VarStmt,
            NodeKind::ReturnStmt => SyntaxKind::ReturnStmt,
            NodeKind::YieldStmt => SyntaxKind::YieldStmt,
            NodeKind::BreakStmt => SyntaxKind::BreakStmt,
            NodeKind::ContinueStmt => SyntaxKind::ContinueStmt,
            NodeKind::GuardStmt => SyntaxKind::GuardStmt,
            NodeKind::DeferStmt => SyntaxKind::DeferStmt,
            NodeKind::WhileStmt => SyntaxKind::WhileStmt,
            NodeKind::ForExpr => SyntaxKind::ForStmt,
            NodeKind::ExprStmt => SyntaxKind::ExprStmt,
            NodeKind::BinaryExpr => SyntaxKind::BinaryExpr,
            NodeKind::UnaryExpr => SyntaxKind::UnaryExpr,
            NodeKind::CallExpr => SyntaxKind::CallExpr,
            NodeKind::MemberExpr => SyntaxKind::MemberExpr,
            NodeKind::IndexExpr => SyntaxKind::IndexExpr,
            NodeKind::TryPropagateExpr => SyntaxKind::TryPropagateExpr,
            NodeKind::BlockExpr => SyntaxKind::BlockExpr,
            NodeKind::IfExpr => SyntaxKind::IfExpr,
            NodeKind::MatchExpr => SyntaxKind::MatchExpr,
            NodeKind::ListExpr => SyntaxKind::ListExpr,
            NodeKind::MapExpr => SyntaxKind::MapExpr,
            NodeKind::TupleExpr => SyntaxKind::TupleExpr,
            NodeKind::TypeNamed => SyntaxKind::TypeNamed,
            NodeKind::TypeTuple => SyntaxKind::TypeTuple,
            NodeKind::PatWildcard => SyntaxKind::PatWildcard,
            NodeKind::PatBind => SyntaxKind::PatBind,
            NodeKind::PatLiteral => SyntaxKind::PatLiteral,
            NodeKind::PolyglotBlock => SyntaxKind::RawDslBlock,
            _ => SyntaxKind::ErrorNode,
        }
    }
}
