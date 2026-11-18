//! Milestone 29.3 & 52.2: Struct-of-Arrays (SoA) `TokenTape`, `TokenFlags`, and 14-Seed `TokenKind`.
//!
//! `TokenKind` contains strictly the **14 Hardware Seed Keywords** (`KwEngine`..`KwAsm`),
//! raw atomic literals, and punctuation/operators. All non-seed words lex as `TokenKind::Ident`.

use pilot_span::ByteSpan;

/// Declares `TokenKind` and its checked `from_u16`: the lexer table names
/// kinds by number (PROP-021 §3), and a number the enum lacks is a load
/// error, never a guess.
macro_rules! token_kinds {
    ($($name:ident = $v:literal,)*) => {
        #[repr(u16)]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
        pub enum TokenKind {
            $($name = $v,)*
        }

        impl TokenKind {
            /// The kind numbered `v`, or `None` when no kind has that number.
            pub fn from_u16(v: u16) -> Option<Self> {
                match v {
                    $($v => Some(Self::$name),)*
                    _ => None,
                }
            }
        }
    };
}

token_kinds! {
    // 0. Special / Structural Tokens
    Eof = 0,
    Newline = 1,
    Comment = 2,
    BlockComment = 3,
    Shebang = 4,
    ErrorToken = 5,

    // 1. Identifiers, Keywords & Literals
    Ident = 20,
    KeywordLit = 21, // :ok, :http/ok
    IntLit = 22,
    HexIntLit = 23,
    BinIntLit = 24,
    OctIntLit = 25,
    FloatLit = 26,
    StrLit = 27,
    TripleStrLit = 28,

    // 2. The Strict 14 Hardware Seed Keywords
    KwEngine = 40,    // __engine
    KwStruct = 41,    // struct
    KwInterface = 42, // interface
    KwFn = 43,        // fn
    KwLet = 44,       // let
    KwRef = 45,       // ref
    KwIf = 46,        // if
    KwLoop = 47,      // loop
    KwReturn = 48,    // return
    KwYield = 49,     // yield
    KwDefer = 50,     // defer
    KwComptime = 51,  // comptime
    KwQuote = 52,     // quote
    KwAsm = 53,       // asm

    // 3. Delimiters & Punctuation
    LParen = 250,      // (
    RParen = 251,      // )
    LBracket = 252,    // [
    RBracket = 253,    // ]
    LBrace = 254,      // {
    RBrace = 255,      // }
    Colon = 256,       // :
    DoubleColon = 257, // ::
    Semicolon = 258,   // ;
    Comma = 259,       // ,
    InfixDot = 260,    // . (immediately after expr)
    PrefixDot = 261,   // . (after whitespace/delimiter -> self.field)
    At = 262,          // @
    Dollar = 263,      // $
    Question = 264,    // ?
    Exclamation = 265, // ! (when not followed by '(')
    Tilde = 266,       // ~

    // 4. Operators. `Operator` is every operator spelling the parser does
    // not name: it reads those by their text (PROP-008 step 2).
    Operator = 269,
    Plus = 270,         // +
    Minus = 271,        // -
    Star = 272,         // *
    DoubleStar = 275,   // **
    StarDotDot = 276,   // *..
    Amper = 278,        // &
    Vbar = 279,         // |
    Caret = 280,        // ^
    ShiftRight = 282,   // >>
    Equal = 283,        // =
    Less = 286,         // <
    Greater = 287,      // >
    ArrowLeft = 291,    // <-

    // 5. Compound Assignment Operators
    PlusEqual = 300,       // +=
    MinusEqual = 301,      // -=
    StarEqual = 302,       // *=
    SlashEqual = 303,      // /=
    PercentEqual = 304,    // %=
    AmperEqual = 307,      // &=
    VbarEqual = 308,       // |=
    CaretEqual = 309,      // ^=
    ShiftLeftEqual = 310,  // <<=
    ShiftRightEqual = 311, // >>=
    CoalesceEqual = 312,   // ??=

    // 6. Pipes, Navigation, Ranges & Lenses
    PipeForward = 320,   // |>
    SafeCascade = 322,   // ?..
    SafeNav = 323,       // ?.
    Arrow = 325,         // ->
    FatArrow = 326,      // =>
    DotDot = 330,        // ..
    DotDotEqual = 331,   // ..=
    DotDotLess = 332,    // ..<
    Ellipsis = 333,      // ...
    GreaterDotDot = 365, // >..

    // 7. Homoiconic Call-Quoting Projections (when immediately followed by '(')
    CallQuoteShallow = 340, // fn!(
    CallQuoteDeep = 341,    // fn!!(
    CallQuoteAbi = 342,     // fn!&(
    CallQuoteReact = 343,   // fn!^(
    CallQuoteAdjoint = 344, // fn!@(
    CallQuoteEffect = 345,  // fn!?(
}

#[allow(non_upper_case_globals)]
impl TokenKind {
    pub const ColonColon: Self = Self::DoubleColon;
}

/// 8-bit bitflags attached to each token in the SoA `TokenTape`.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct TokenFlags(pub u8);

impl TokenFlags {
    pub const NONE: Self = Self(0);
    pub const IS_RAW_STRING: Self = Self(1 << 3);
    pub const IS_INTERPOLATED: Self = Self(1 << 4);
    pub const IS_DIMENSIONAL: Self = Self(1 << 5);

    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline(always)]
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// Cache-friendly Struct-of-Arrays (SoA) token stream.
#[derive(Clone, Debug, Default)]
pub struct TokenTape {
    pub kinds: Vec<TokenKind>,
    pub flags: Vec<TokenFlags>,
    pub spans: Vec<ByteSpan>,
    pub bracket_links: Vec<u32>,
}

impl TokenTape {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            kinds: Vec::with_capacity(cap),
            flags: Vec::with_capacity(cap),
            spans: Vec::with_capacity(cap),
            bracket_links: Vec::with_capacity(cap),
        }
    }

    #[inline(always)]
    pub fn push(&mut self, kind: TokenKind, flag: TokenFlags, span: ByteSpan) -> u32 {
        let idx = self.kinds.len() as u32;
        self.kinds.push(kind);
        self.flags.push(flag);
        self.spans.push(span);
        self.bracket_links.push(u32::MAX);
        idx
    }

    #[inline(always)]
    pub fn matching_bracket(&self, idx: usize) -> Option<u32> {
        let target = self.bracket_links.get(idx).copied()?;
        if target == u32::MAX {
            None
        } else {
            Some(target)
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.kinds.len()
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
    #[inline(always)]
    pub fn kind(&self, idx: usize) -> TokenKind {
        self.kinds[idx]
    }
    #[inline(always)]
    pub fn flag(&self, idx: usize) -> TokenFlags {
        self.flags[idx]
    }
    #[inline(always)]
    pub fn span(&self, idx: usize) -> ByteSpan {
        self.spans[idx]
    }
    #[inline(always)]
    pub fn text<'a>(&self, idx: usize, src: &'a str) -> &'a str {
        let sp = self.spans[idx];
        &src[sp.start as usize..(sp.end as usize).min(src.len())]
    }
}
