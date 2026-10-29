//! Milestone 29: Token definitions, bitflags, and SoA `TokenTape` with balanced bracket links.

use pilot_span::ByteSpan;

/// 16-bit representation of all lexical tokens in Pilot.
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // --- Special & Virtual Tokens (0..19) ---
    Eof = 0,
    Newline = 1,
    VirtualLBrace = 2,
    VirtualRBrace = 3,
    Comment = 4,
    BlockComment = 5,
    Shebang = 6,
    ErrorToken = 7,

    // --- Identifiers & Atomic Literals (20..59) ---
    Ident = 20,
    KeywordLit = 21,    // :keyword or :ns/keyword
    IntLit = 22,        // 42
    HexIntLit = 23,     // 0x2A
    BinIntLit = 24,     // 0b101010
    OctIntLit = 25,     // 0o52
    FloatLit = 26,      // 3.1415, 1e-4
    StrLit = 27,        // "hello" or 'hello'
    TripleStrLit = 28,  // """raw""" or '''raw'''
    FStrStart = 29,     // f"hello {
    FStrMiddle = 30,    // } middle {
    FStrEnd = 31,       // } end"
    DurationLit = 32,   // 250ms, 10s, 5min, 1h, 100ns, 50us
    FrequencyLit = 33,  // 60Hz, 44.1kHz, 2.4GHz
    MemoryLit = 34,     // 64B, 512KB, 16MB, 1TB
    RawDslBlock = 35,   // ```sql ... ```

    // --- Strict Keywords (60..119) ---
    KwLet = 60,
    KwVar = 61,
    KwConst = 62,
    KwDef = 63,
    KwFn = 64,
    KwClass = 65,
    KwStruct = 66,
    KwRecord = 67,
    KwInterface = 68,
    KwProtocol = 69,
    KwEnum = 70,
    KwActor = 71,
    KwType = 72,
    KwRef = 73,
    KwDynamic = 74,
    KwSymbol = 75,
    KwPure = 76,
    KwComptime = 77,
    KwMut = 78,
    KwImmut = 79,
    KwProperty = 80,
    KwGet = 81,
    KwOperator = 82,
    KwExtension = 83,
    KwExtend = 84,
    KwState = 85,
    KwIf = 86,
    KwElif = 87,
    KwElse = 88,
    KwThen = 89,
    KwWhile = 90,
    KwFor = 91,
    KwIn = 92,
    KwMatch = 93,
    KwCase = 94,
    KwWhen = 95,
    KwReturn = 96,
    KwYield = 97,
    KwBreak = 98,
    KwContinue = 99,
    KwPass = 100,
    KwRaise = 101,
    KwTry = 102,
    KwCatch = 103,
    KwFinally = 104,
    KwWith = 105,
    KwAs = 106,
    KwAssert = 107,
    KwImport = 108,
    KwFrom = 109,
    KwGlobal = 110,
    KwLambda = 111,
    KwAnd = 112,
    KwOr = 113,
    KwNot = 114,
    KwIs = 115,
    KwGuard = 116,
    KwDefer = 117,
    KwContext = 118,
    KwRequires = 119,

    // --- Contextual Keywords (120..199) ---
    KwEnsures = 120,
    KwAsync = 121,
    KwAwait = 122,
    KwSpawn = 123,
    KwParallel = 124,
    KwChan = 125,
    KwChannel = 126,
    KwAtomic = 127,
    KwSync = 128,
    KwLock = 129,
    KwIsolate = 130,
    KwRealtime = 131,
    KwDeadline = 132,
    KwOnMissed = 133,
    KwWithin = 134,
    KwFallback = 135,
    KwPeriod = 136,
    KwPeriodic = 137,
    KwPriority = 138,
    KwJitter = 139,
    KwBounded = 140,
    KwNoalloc = 141,
    KwStackalloc = 142,
    KwRegion = 143,
    KwWcet = 144,
    KwHardRt = 145,
    KwSoftRt = 146,
    KwPublic = 147,
    KwPrivate = 148,
    KwProtected = 149,
    KwInternal = 150,
    KwStatic = 151,
    KwPartial = 152,
    KwSteal = 153,
    KwContracts = 154,
    KwDelegate = 155,
    KwParams = 156,
    KwArgs = 157,
    KwKwargs = 158,
    KwNameof = 159,
    KwEach = 160,
    KwRepeat = 161,
    KwAdapter = 162,
    KwConverter = 163,
    KwDefault = 164,
    KwTimeout = 165,
    KwSelect = 166,
    KwWhere = 167,
    KwJoin = 168,
    KwInner = 169,
    KwLeft = 170,
    KwRight = 171,
    KwFull = 172,
    KwCross = 173,
    KwOuter = 174,
    KwOn = 175,
    KwGroup = 176,
    KwBy = 177,
    KwHaving = 178,
    KwOrder = 179,
    KwAsc = 180,
    KwDesc = 181,
    KwAscending = 182,
    KwDescending = 183,
    KwLimit = 184,
    KwOffset = 185,
    KwInto = 186,
    KwDistinct = 187,
    KwUnion = 188,
    KwAll = 189,
    KwIntersect = 190,
    KwExcept = 191,
    KwDifference = 192,
    KwIndex = 193,
    KwIndexed = 194,
    KwLookup = 195,
    KwTable = 196,
    KwSchema = 197,
    KwInsert = 198,
    KwUpdate = 199,

    // --- Additional Contextual & Polyglot Keywords (200..239) ---
    KwDelete = 200,
    KwValues = 201,
    KwSet = 202,
    KwReturning = 203,
    KwConflict = 204,
    KwDo = 205,
    KwNothing = 206,
    KwOver = 207,
    KwPartition = 208,
    KwWindow = 209,
    KwTransaction = 210,
    KwCommit = 211,
    KwRollback = 212,
    KwSavepoint = 213,
    KwTo = 214,
    KwConnect = 215,
    KwDatabase = 216,
    KwModel = 217,
    KwDsl = 218,
    KwDomain = 219,
    KwUnit = 220,
    KwMacro = 221,
    KwDefmacro = 222,
    KwDefkeyword = 223,
    KwQuote = 224,
    KwQuasiquote = 225,
    KwSplice = 226,
    KwUnquote = 227,
    KwUnquoteSplicing = 228,
    KwExtern = 229,
    KwModule = 230,
    KwLib = 231,
    KwTrue = 232,
    KwFalse = 233,
    KwSome = 234,
    KwNone = 235,
    KwNull = 236,
    KwUndefined = 237,

    // --- Delimiters & Single Character Punctuation (240..269) ---
    LParen = 240,       // (
    RParen = 241,       // )
    LBracket = 242,     // [
    RBracket = 243,     // ]
    LBrace = 244,       // {
    RBrace = 245,       // }
    Colon = 246,        // :
    Semicolon = 247,    // ;
    Comma = 248,        // ,
    Dot = 249,          // .
    At = 250,           // @
    Question = 251,     // ?
    Exclamation = 252,  // !
    Tilde = 253,        // ~
    Dollar = 254,       // $
    Backquote = 255,    // `
    Underscore = 256,   // _
    Walrus = 257,       // :=
    ArrowLeft = 258,    // <-

    // --- Operators & Arrows (270..319) ---
    Plus = 270,         // +
    Minus = 271,        // -
    Star = 272,         // *
    Slash = 273,        // /
    Percent = 274,      // %
    DoubleStar = 275,   // **
    DoubleSlash = 276,  // //
    Equal = 277,        // =
    EqEqual = 278,      // ==
    NotEqual = 279,     // != or <>
    Less = 280,         // <
    LessEqual = 281,    // <=
    Greater = 282,      // >
    GreaterEqual = 283, // >=
    Amper = 284,        // &
    Vbar = 285,         // |
    Caret = 286,        // ^
    ShiftLeft = 287,    // <<
    ShiftRight = 288,   // >>
    PlusEqual = 289,    // +=
    MinusEqual = 290,   // -=
    StarEqual = 291,    // *=
    SlashEqual = 292,   // /=
    PercentEqual = 293, // %=
    DoubleStarEqual = 294, // **=
    DoubleSlashEqual = 295, // //=
    AmperEqual = 296,   // &=
    VbarEqual = 297,    // |=
    CaretEqual = 298,   // ^=
    ShiftLeftEqual = 299, // <<=
    ShiftRightEqual = 300, // >>=
    CoalesceEqual = 301, // ??=
    Coalesce = 302,     // ??
    SafeNav = 303,      // ?.
    SafeCascade = 304,  // ?..
    PipeForward = 305,  // |>
    PipeBackward = 306, // <|
    Arrow = 307,        // ->
    FatArrow = 308,     // =>
    JsonGetText = 309,  // ->>
    JsonContains = 310, // @>
    JsonContained = 311,// <@
    DotDot = 312,       // ..
    DotDotEqual = 313,  // ..=
    DotDotLess = 314,   // ..<
    Ellipsis = 315,     // ...

    // --- Call-Quoting Projections (320..329, Invariant #1 & M17) ---
    CallQuoteShallow = 320, // fn!(...)
    CallQuoteDeep = 321,    // fn!!(...)
    CallQuoteAbi = 322,     // fn!&(...)
    CallQuoteReact = 323,   // fn!^(...)
    CallQuoteAdjoint = 324, // fn!@(...)
    CallQuoteEffect = 325,  // fn!?(...)
}

impl TokenKind {
    #[inline]
    pub const fn is_keyword(self) -> bool {
        (self as u16) >= (TokenKind::KwLet as u16) && (self as u16) <= (TokenKind::KwUndefined as u16)
    }

    #[inline]
    pub const fn is_delimiter_open(self) -> bool {
        matches!(self, TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace | TokenKind::VirtualLBrace)
    }

    #[inline]
    pub const fn is_delimiter_close(self) -> bool {
        matches!(self, TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace | TokenKind::VirtualRBrace)
    }

    #[inline]
    pub const fn matching_delimiter(self) -> Option<Self> {
        match self {
            Self::LParen => Some(Self::RParen),
            Self::RParen => Some(Self::LParen),
            Self::LBracket => Some(Self::RBracket),
            Self::RBracket => Some(Self::LBracket),
            Self::LBrace => Some(Self::RBrace),
            Self::RBrace => Some(Self::LBrace),
            Self::VirtualLBrace => Some(Self::VirtualRBrace),
            Self::VirtualRBrace => Some(Self::VirtualLBrace),
            _ => None,
        }
    }
}

/// Bitflags associated with each emitted token.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct TokenFlags(pub u16);

impl TokenFlags {
    pub const NONE: Self = Self(0);
    pub const IS_KEYWORD: Self = Self(1 << 0);
    pub const IS_CONTEXTUAL_IDENT: Self = Self(1 << 1);
    pub const IS_VIRTUAL: Self = Self(1 << 2);
    pub const IS_DELIMITER_OPEN: Self = Self(1 << 3);
    pub const IS_DELIMITER_CLOSE: Self = Self(1 << 4);
    pub const PRECEDING_WHITESPACE: Self = Self(1 << 5);
    pub const TRAILING_WHITESPACE: Self = Self(1 << 6);
    pub const IS_DIMENSIONAL: Self = Self(1 << 7);

    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline]
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl core::ops::BitOr for TokenFlags {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for TokenFlags {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Dense SoA Token Tape with balanced delimiter jump table (`bracket_links`).
/// Invariant: `kinds.len() == flags.len() == spans.len() == bracket_links.len()`.
#[derive(Clone, Debug, Default)]
pub struct TokenTape {
    pub kinds: Vec<TokenKind>,
    pub flags: Vec<TokenFlags>,
    pub spans: Vec<ByteSpan>,
    pub bracket_links: Vec<u32>,
}

impl TokenTape {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            kinds: Vec::with_capacity(cap),
            flags: Vec::with_capacity(cap),
            spans: Vec::with_capacity(cap),
            bracket_links: Vec::with_capacity(cap),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    #[inline]
    pub fn push(&mut self, kind: TokenKind, flags: TokenFlags, span: ByteSpan) -> u32 {
        let idx = self.kinds.len() as u32;
        self.kinds.push(kind);
        self.flags.push(flags);
        self.spans.push(span);
        self.bracket_links.push(u32::MAX);
        idx
    }

    #[inline]
    pub fn kind(&self, idx: usize) -> TokenKind {
        self.kinds[idx]
    }

    #[inline]
    pub fn flag(&self, idx: usize) -> TokenFlags {
        self.flags[idx]
    }

    #[inline]
    pub fn span(&self, idx: usize) -> ByteSpan {
        self.spans[idx]
    }

    #[inline]
    pub fn matching_bracket(&self, idx: usize) -> Option<u32> {
        let target = self.bracket_links.get(idx).copied()?;
        if target == u32::MAX {
            None
        } else {
            Some(target)
        }
    }

    #[inline]
    pub fn text<'a>(&self, idx: usize, src: &'a str) -> &'a str {
        let span = self.span(idx);
        let s = span.start as usize;
        let e = (span.end as usize).min(src.len());
        if s <= e && s <= src.len() {
            &src[s..e]
        } else {
            ""
        }
    }
}
