//! Milestone 29.4 & 52.2: The 14-Token Hardware Seed Keyword Matcher.
//!
//! Only the **14 irreducible hardware seed tokens** (`__engine`, `struct`, `interface`, `fn`,
//! `let`, `ref`, `if`, `loop`, `return`, `yield`, `defer`, `comptime`, `quote`, `asm`) are
//! recognized as `TokenKind::Kw*` in Rust. Every other identifier lexes as `TokenKind::Ident`.

use crate::token::TokenKind;

/// O(1) length-and-byte switch mapping strictly the 14 hardware seed words to `TokenKind::Kw*`.
#[inline]
pub fn lookup_keyword(ident: &str) -> Option<TokenKind> {
    match ident {
        "__engine" => Some(TokenKind::KwEngine),
        "struct" => Some(TokenKind::KwStruct),
        "interface" => Some(TokenKind::KwInterface),
        "fn" => Some(TokenKind::KwFn),
        "let" => Some(TokenKind::KwLet),
        "ref" => Some(TokenKind::KwRef),
        "if" => Some(TokenKind::KwIf),
        "loop" => Some(TokenKind::KwLoop),
        "return" => Some(TokenKind::KwReturn),
        "yield" => Some(TokenKind::KwYield),
        "defer" => Some(TokenKind::KwDefer),
        "comptime" => Some(TokenKind::KwComptime),
        "quote" => Some(TokenKind::KwQuote),
        "asm" => Some(TokenKind::KwAsm),
        _ => None,
    }
}
