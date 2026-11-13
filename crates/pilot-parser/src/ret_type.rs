//! A parenthesized return type before an expression body.
//!
//! `fn pair() -> (i64, str) => (1, "b");` has two readings of its first
//! `=>`: the arrow of a function type `(i64, str) => ..`, or the start of the
//! function's expression body after the tuple type `(i64, str)`. It begins
//! the body when no later `=>` and no block follows before the declaration
//! ends: a function type would leave the function without a body.

use crate::Parser;
use pilot_lexer::TokenKind;

/// One above the binding power of the function-type arrow (`compile_type_rule`
/// gives `=>` and `->` 10), so a type parsed at it stops before the arrow.
pub(crate) const ABOVE_FN_ARROW: u8 = 11;

impl<'a> Parser<'a> {
    /// At a function's return type: whether it is a parenthesized type whose
    /// `)` is followed by the `=>` that begins the expression body.
    pub(crate) fn tuple_return_before_body(&self) -> bool {
        if !self.at(TokenKind::LParen) {
            return false;
        }
        let Some(close) = self.tape.matching_bracket(self.cur()) else { return false };
        let after = self.next_non_nl(close as usize + 1);
        if after >= self.tape.len() || self.tape.kind(after) != TokenKind::FatArrow {
            return false;
        }
        let (mut depth, mut arrows, mut p) = (0i32, 0u32, after);
        while p < self.tape.len() {
            match self.tape.kind(p) {
                // A block follows: the arrows before it belong to the type.
                TokenKind::LBrace if depth == 0 => return false,
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => depth -= 1,
                TokenKind::FatArrow if depth == 0 => arrows += 1,
                TokenKind::Semicolon if depth == 0 => break,
                _ => {}
            }
            if depth < 0 {
                break;
            }
            p += 1;
        }
        arrows == 1
    }
}
