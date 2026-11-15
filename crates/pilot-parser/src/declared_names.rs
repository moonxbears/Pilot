//! Names a file declares win over receptor words in call position.
//!
//! A word the stdlib binds to a receptor (a language name such as the one
//! that opens an inline foreign block, or the readable unpacking words) is
//! still an ordinary name when the file declares a function of that name:
//! `fn pairs(..)` and a later `pairs(x)` are a declaration and a call of it.
//! A language word followed by `(` opens an inline block only when the
//! parentheses hold the block's text, a string literal.

use crate::{is_word_tok, Parser};
use pilot_lexer::TokenKind;

impl<'a> Parser<'a> {
    /// Whether this file declares a function named `word` (`fn word(` or
    /// `fn word<`).
    pub(crate) fn file_declares_fn(&self, word: &str) -> bool {
        let mut p = 0;
        while p < self.tape.len() {
            if self.tape.kind(p) == TokenKind::KwFn {
                let n = self.next_non_nl(p + 1);
                if n < self.tape.len() && is_word_tok(self.tape.kind(n)) && self.tape.text(n, self.src) == word {
                    let after = self.next_non_nl(n + 1);
                    if after < self.tape.len() && matches!(self.tape.kind(after), TokenKind::LParen | TokenKind::Less) {
                        return true;
                    }
                }
            }
            p += 1;
        }
        false
    }
}
