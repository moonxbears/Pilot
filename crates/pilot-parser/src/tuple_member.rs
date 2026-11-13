//! Nested tuple positions: `t.0.1`.
//!
//! After a member dot the lexer reads `0.1` as one float literal. Where a
//! member name is expected it is two positions, `0` then `1`.

use crate::Parser;
use pilot_ast::{NodeId, NodeKind};
use pilot_lexer::TokenKind;
use pilot_span::ByteSpan;

impl<'a> Parser<'a> {
    /// At the token after a member dot on `lhs`: when it is a float literal
    /// made of two digit runs, the two tuple positions it spells, as nested
    /// member reads.
    pub(crate) fn parse_tuple_path(&mut self, lhs: NodeId) -> Option<NodeId> {
        if self.peek() != TokenKind::FloatLit {
            return None;
        }
        let text = self.text().to_string();
        let (first, second) = text.split_once('.')?;
        let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        if !digits(first) || !digits(second) {
            return None;
        }
        let start = self.arena.span(lhs).start;
        let end = self.span().end;
        self.advance();
        let (a, b) = (self.arena.interner.intern(first), self.arena.interner.intern(second));
        let span = ByteSpan::new(start, end);
        let outer = self.arena.alloc_node(NodeKind::MemberExpr, 0, span, lhs.0, a.0);
        Some(self.arena.alloc_node(NodeKind::MemberExpr, 0, span, outer.0, b.0))
    }
}
