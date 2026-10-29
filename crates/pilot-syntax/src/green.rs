//! Milestone 31.1: Immutable Width-Relative Green Tree with Token Interning.

use std::sync::Arc;
use crate::kind::SyntaxKind;

/// An immutable, width-relative token storing its `SyntaxKind`, interned text, and UTF-8 length.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GreenToken {
    kind: SyntaxKind,
    text: Arc<str>,
    text_len: u32,
}

impl GreenToken {
    #[inline]
    pub fn new(kind: SyntaxKind, text: &str) -> Self {
        if let Some(token) = get_interned_token(kind, text) {
            return token;
        }
        let text_len = text.len() as u32;
        let text_arc: Arc<str> = Arc::from(text);
        Self {
            kind,
            text: text_arc,
            text_len,
        }
    }

    #[inline]
    pub fn kind(&self) -> SyntaxKind {
        self.kind
    }

    #[inline]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[inline]
    pub fn text_arc(&self) -> &Arc<str> {
        &self.text
    }

    #[inline]
    pub fn text_len(&self) -> u32 {
        self.text_len
    }
}

/// An element in the immutable Green tree: either a composite GreenNode or a leaf GreenToken.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GreenElement {
    Node(Arc<GreenNode>),
    Token(GreenToken),
}

impl GreenElement {
    #[inline]
    pub fn text_len(&self) -> u32 {
        match self {
            GreenElement::Node(node) => node.text_len,
            GreenElement::Token(token) => token.text_len,
        }
    }

    #[inline]
    pub fn kind(&self) -> SyntaxKind {
        match self {
            GreenElement::Node(node) => node.kind,
            GreenElement::Token(token) => token.kind,
        }
    }

    pub fn write_text(&self, out: &mut String) {
        match self {
            GreenElement::Node(node) => node.write_text(out),
            GreenElement::Token(token) => out.push_str(token.text()),
        }
    }
}

/// An immutable, width-relative composite syntax node in the Green tree.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GreenNode {
    kind: SyntaxKind,
    text_len: u32,
    children: Box<[GreenElement]>,
}

impl GreenNode {
    pub fn new(kind: SyntaxKind, children: Vec<GreenElement>) -> Arc<Self> {
        let text_len = children.iter().map(|c| c.text_len()).sum();
        Arc::new(Self {
            kind,
            text_len,
            children: children.into_boxed_slice(),
        })
    }

    #[inline]
    pub fn kind(&self) -> SyntaxKind {
        self.kind
    }

    #[inline]
    pub fn text_len(&self) -> u32 {
        self.text_len
    }

    #[inline]
    pub fn children(&self) -> &[GreenElement] {
        &self.children
    }

    pub fn write_text(&self, out: &mut String) {
        for child in self.children.iter() {
            child.write_text(out);
        }
    }

    pub fn to_text(&self) -> String {
        let mut buf = String::with_capacity(self.text_len as usize);
        self.write_text(&mut buf);
        buf
    }
}

/// Zero-allocation global cache for common punctuation, whitespace, and keywords.
fn get_interned_token(kind: SyntaxKind, text: &str) -> Option<GreenToken> {
    macro_rules! intern {
        ($expected_kind:expr, $str_lit:expr) => {
            if kind == $expected_kind && text == $str_lit {
                static CACHED: std::sync::OnceLock<Arc<str>> = std::sync::OnceLock::new();
                let arc = CACHED.get_or_init(|| Arc::from($str_lit));
                return Some(GreenToken {
                    kind,
                    text: arc.clone(),
                    text_len: $str_lit.len() as u32,
                });
            }
        };
    }

    // Common whitespace & newlines
    intern!(SyntaxKind::Whitespace, " ");
    intern!(SyntaxKind::Whitespace, "  ");
    intern!(SyntaxKind::Whitespace, "    ");
    intern!(SyntaxKind::Whitespace, "        ");
    intern!(SyntaxKind::Newline, "\n");
    intern!(SyntaxKind::Newline, "\r\n");

    // Common punctuation
    intern!(SyntaxKind::Semicolon, ";");
    intern!(SyntaxKind::Colon, ":");
    intern!(SyntaxKind::Comma, ",");
    intern!(SyntaxKind::Dot, ".");
    intern!(SyntaxKind::LParen, "(");
    intern!(SyntaxKind::RParen, ")");
    intern!(SyntaxKind::LBrace, "{");
    intern!(SyntaxKind::RBrace, "}");
    intern!(SyntaxKind::LBracket, "[");
    intern!(SyntaxKind::RBracket, "]");
    intern!(SyntaxKind::Equal, "=");
    intern!(SyntaxKind::Plus, "+");
    intern!(SyntaxKind::Minus, "-");
    intern!(SyntaxKind::Star, "*");
    intern!(SyntaxKind::Slash, "/");
    intern!(SyntaxKind::Arrow, "->");
    intern!(SyntaxKind::FatArrow, "=>");
    intern!(SyntaxKind::Question, "?");
    intern!(SyntaxKind::Exclamation, "!");
    intern!(SyntaxKind::Underscore, "_");

    // Common strict & contextual keywords
    intern!(SyntaxKind::KwLet, "let");
    intern!(SyntaxKind::KwVar, "var");
    intern!(SyntaxKind::KwConst, "const");
    intern!(SyntaxKind::KwDef, "def");
    intern!(SyntaxKind::KwFn, "fn");
    intern!(SyntaxKind::KwReturn, "return");
    intern!(SyntaxKind::KwIf, "if");
    intern!(SyntaxKind::KwElse, "else");
    intern!(SyntaxKind::KwWhile, "while");
    intern!(SyntaxKind::KwFor, "for");
    intern!(SyntaxKind::KwMatch, "match");
    intern!(SyntaxKind::KwClass, "class");
    intern!(SyntaxKind::KwStruct, "struct");
    intern!(SyntaxKind::KwType, "type");
    intern!(SyntaxKind::KwTrue, "true");
    intern!(SyntaxKind::KwFalse, "false");
    intern!(SyntaxKind::KwNone, "none");
    intern!(SyntaxKind::KwNull, "null");

    None
}
