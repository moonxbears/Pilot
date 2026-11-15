//! Index ranges with a bound left out: `x[..b]`, `x[..=b]` and `x[..]`.
//!
//! A range that starts at the `..` has no left operand; `x[a..]` (parsed as
//! any binary expression whose right operand is missing) has no right one.
//! The engines read a missing bound as the start or the end of the value.

use crate::Parser;
use pilot_ast::{BinaryOp, NodeId, NodeKind};
use pilot_lexer::TokenKind;
use pilot_span::ByteSpan;

impl<'a> Parser<'a> {
    /// At `..` or `..=` right after the `[` of an index.
    pub(crate) fn parse_open_range(&mut self) -> NodeId {
        let sp = self.span();
        let op = if self.at(TokenKind::DotDotEqual) { BinaryOp::RangeInclusive } else { BinaryOp::RangeHalfOpen };
        self.advance();
        let end = if self.at(TokenKind::RBracket) { NodeId::NONE } else { self.parse_expr() };
        let span = ByteSpan::new(sp.start, self.prev_span().end.max(sp.end));
        self.arena.alloc_node(NodeKind::BinaryExpr, op as u16, span, NodeId::NONE.0, end.0)
    }
}
