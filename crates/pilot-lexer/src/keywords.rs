//! Phase 1: Strict 14-Token Reserved Keyword Table.
//!
//! At startup, the lexer recognizes ONLY these 14 reserved keywords:
//! `__engine`, `struct`, `interface`, `fn`, `let`, `ref`, `if`, `loop`, `return`, `yield`, `defer`, `comptime`, `quote`, `asm`.
//! All other words lex as `TokenKind::Ident` until dynamically registered into `ParserTables` (Phase 2).

use crate::token::TokenKind;
use std::collections::HashMap;
use std::sync::RwLock;

static DYNAMIC_KEYWORDS: RwLock<Option<HashMap<String, (TokenKind, bool)>>> = RwLock::new(None);

/// Register a dynamic keyword or modifier from `stdlib/core/core.pilot` (Phase 2).
pub fn register_dynamic_keyword(ident: &str, kind: TokenKind, is_contextual: bool) {
    if let Ok(mut lock) = DYNAMIC_KEYWORDS.write() {
        if lock.is_none() {
            *lock = Some(HashMap::new());
        }
        if let Some(map) = lock.as_mut() {
            map.insert(ident.to_string(), (kind, is_contextual));
        }
    }
}

/// Clears all dynamically registered keywords.
pub fn clear_dynamic_keywords() {
    if let Ok(mut lock) = DYNAMIC_KEYWORDS.write() {
        *lock = None;
    }
}

/// Checks if an identifier has been dynamically registered.
pub fn is_registered_dynamic_keyword(ident: &str) -> bool {
    if let Ok(lock) = DYNAMIC_KEYWORDS.read() {
        if let Some(map) = lock.as_ref() {
            return map.contains_key(ident);
        }
    }
    false
}

/// Looks up an ASCII identifier slice, returning `Some((TokenKind, false))` if it matches
/// one of the 14 strict reserved keywords or a dynamically registered keyword.
#[inline]
pub fn lookup_keyword(ident: &str) -> Option<(TokenKind, bool)> {
    match ident {
        "__engine" => Some((TokenKind::KwEngine, false)),
        "struct" => Some((TokenKind::KwStruct, false)),
        "interface" => Some((TokenKind::KwInterface, false)),
        "fn" => Some((TokenKind::KwFn, false)),
        "let" => Some((TokenKind::KwLet, false)),
        "ref" => Some((TokenKind::KwRef, false)),
        "if" => Some((TokenKind::KwIf, false)),
        "loop" => Some((TokenKind::KwLoop, false)),
        "return" => Some((TokenKind::KwReturn, false)),
        "yield" => Some((TokenKind::KwYield, false)),
        "defer" => Some((TokenKind::KwDefer, false)),
        "comptime" => Some((TokenKind::KwComptime, false)),
        "quote" => Some((TokenKind::KwQuote, false)),
        "asm" => Some((TokenKind::KwAsm, false)),
        _ => {
            if let Ok(lock) = DYNAMIC_KEYWORDS.read() {
                if let Some(map) = lock.as_ref() {
                    if let Some(res) = map.get(ident) {
                        return Some(*res);
                    }
                }
            }
            None
        }
    }
}
