//! Milestone 31.2: Resilient Panic-Free Error Recovery, TokenSets, and Fix-It Diagnostics.

use pilot_span::ByteSpan;
use crate::kind::SyntaxKind;

/// High-performance 128-bit bitmask set for fast O(1) syntax recovery token testing.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct TokenSet(pub u128);

impl TokenSet {
    pub const EMPTY: Self = Self(0);

    #[inline]
    pub const fn of(kind: SyntaxKind) -> Self {
        let val = kind as u16;
        if val < 128 {
            Self(1u128 << val)
        } else {
            Self(0)
        }
    }

    #[inline]
    pub const fn contains(&self, kind: SyntaxKind) -> bool {
        let val = kind as u16;
        if val < 128 {
            (self.0 & (1u128 << val)) != 0
        } else {
            false
        }
    }

    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn insert(&mut self, kind: SyntaxKind) {
        let val = kind as u16;
        if val < 128 {
            self.0 |= 1u128 << val;
        }
    }
}

/// A structured syntax diagnostic with source span and optional fix-it recommendation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxDiagnostic {
    pub message: String,
    pub span: ByteSpan,
    pub suggested_fix: Option<String>,
}

impl SyntaxDiagnostic {
    pub fn new(message: impl Into<String>, span: ByteSpan) -> Self {
        Self {
            message: message.into(),
            span,
            suggested_fix: None,
        }
    }

    pub fn with_fix(message: impl Into<String>, span: ByteSpan, fix: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span,
            suggested_fix: Some(fix.into()),
        }
    }
}

/// Standard stacked recovery sets for robust synchronization without block discarding.
pub struct RecoverySets;

impl RecoverySets {
    pub fn stmt_recovery() -> &'static [SyntaxKind] {
        &[
            SyntaxKind::KwLet,
            SyntaxKind::KwVar,
            SyntaxKind::KwConst,
            SyntaxKind::KwDef,
            SyntaxKind::KwFn,
            SyntaxKind::KwClass,
            SyntaxKind::KwStruct,
            SyntaxKind::KwRecord,
            SyntaxKind::KwInterface,
            SyntaxKind::KwType,
            SyntaxKind::KwReturn,
            SyntaxKind::KwYield,
            SyntaxKind::KwBreak,
            SyntaxKind::KwContinue,
            SyntaxKind::KwGuard,
            SyntaxKind::KwDefer,
            SyntaxKind::KwWhile,
            SyntaxKind::KwFor,
            SyntaxKind::KwIf,
            SyntaxKind::KwMatch,
            SyntaxKind::Semicolon,
            SyntaxKind::RBrace,
            SyntaxKind::VirtualRBrace,
        ]
    }

    pub fn decl_recovery() -> &'static [SyntaxKind] {
        &[
            SyntaxKind::KwImport,
            SyntaxKind::KwFrom,
            SyntaxKind::KwClass,
            SyntaxKind::KwStruct,
            SyntaxKind::KwRecord,
            SyntaxKind::KwInterface,
            SyntaxKind::KwExtend,
            SyntaxKind::KwDef,
            SyntaxKind::KwFn,
            SyntaxKind::KwType,
            SyntaxKind::KwLet,
            SyntaxKind::KwVar,
            SyntaxKind::KwConst,
        ]
    }

    pub fn param_recovery() -> &'static [SyntaxKind] {
        &[
            SyntaxKind::Comma,
            SyntaxKind::RParen,
            SyntaxKind::Colon,
            SyntaxKind::Arrow,
            SyntaxKind::LBrace,
        ]
    }

    pub fn arg_recovery() -> &'static [SyntaxKind] {
        &[
            SyntaxKind::Comma,
            SyntaxKind::RParen,
            SyntaxKind::RBracket,
            SyntaxKind::Semicolon,
        ]
    }
}

/// Damerau-Levenshtein distance calculation for keyword typo fix-it hints.
pub fn find_keyword_typo_candidate(ident: &str) -> Option<&'static str> {
    const KEYWORDS: &[&str] = &[
        "let", "var", "const", "def", "fn", "class", "struct", "record",
        "interface", "protocol", "enum", "actor", "type", "extend",
        "return", "yield", "break", "continue", "guard", "defer",
        "while", "for", "in", "match", "case", "when", "if", "elif", "else",
        "import", "from", "public", "private", "async", "await", "spawn",
        "select", "where", "default", "true", "false", "none", "null",
    ];

    let mut best_match = None;
    let mut min_dist = usize::MAX;

    for &kw in KEYWORDS {
        let dist = damerau_levenshtein(ident, kw);
        if dist <= 2 && dist < min_dist {
            min_dist = dist;
            best_match = Some(kw);
        }
    }

    best_match
}

/// Computes the Damerau-Levenshtein distance between two string slices.
pub fn damerau_levenshtein(s1: &str, s2: &str) -> usize {
    let b1 = s1.as_bytes();
    let b2 = s2.as_bytes();
    let l1 = b1.len();
    let l2 = b2.len();

    if l1 == 0 {
        return l2;
    }
    if l2 == 0 {
        return l1;
    }

    let mut d = vec![vec![0usize; l2 + 1]; l1 + 1];

    for i in 0..=l1 {
        d[i][0] = i;
    }
    for j in 0..=l2 {
        d[0][j] = j;
    }

    for i in 1..=l1 {
        for j in 1..=l2 {
            let cost = if b1[i - 1] == b2[j - 1] { 0 } else { 1 };

            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);

            if i > 1 && j > 1 && b1[i - 1] == b2[j - 2] && b1[i - 2] == b2[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }

    d[l1][l2]
}
