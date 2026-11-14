//! Modifiers that take an argument (`lvl(9) fn f()`).
//!
//! The library registers them with `register_parametric_modifier`. The
//! parser knows none by name: it keeps each one on its declaration as an
//! attribute named by the modifier's word, holding the argument, so the
//! checker sees it (`attr_uses`) the way it sees `[Name(arg)]`. Nothing is
//! skipped (AGENTS.md section 11, rule 2).

use crate::{Parser, PendingAttr};
use pilot_lexer::TokenKind;

impl<'a> Parser<'a> {
    /// At a parametric modifier followed by `(`: reads `word(arg)` and
    /// returns it as an attribute.
    pub(crate) fn parse_param_modifier(&mut self) -> PendingAttr {
        let name = self.intern_cur();
        self.expect(TokenKind::LParen);
        let mut args = Vec::new();
        if !self.at(TokenKind::RParen) {
            let arg = self.parse_expr();
            if arg.is_some() {
                args.push(arg);
            }
        }
        self.expect(TokenKind::RParen);
        PendingAttr { name, args, is_mut: false, bind_sym: Default::default() }
    }
}
