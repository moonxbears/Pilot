//! `errdefer` and `become` (`stdlib/core/syntax/exits.pilot`).
//!
//! `errdefer stmt` is a `DeferStmt` whose flags are 1: it runs only when its
//! block is left by a failure. `become call;` is a `ReturnStmt` whose flags
//! are 1: a guaranteed tail call.

use crate::Parser;
use pilot_ast::{NodeId, NodeKind};
use pilot_lexer::TokenKind;
use pilot_span::ByteSpan;

/// The receptor slots `stdlib/core/syntax/exits.pilot` binds.
pub(crate) const ERRDEFER_RECEPTOR: u8 = 38;
pub(crate) const BECOME_RECEPTOR: u8 = 39;

/// `DeferStmt.flags` of an `errdefer`.
pub const DEFER_ON_FAILURE: u16 = 1;
/// `ReturnStmt.flags` of a `become`.
pub const RETURN_TAIL_CALL: u16 = 1;

impl<'a> Parser<'a> {
    /// `errdefer { .. }` or `errdefer stmt`.
    pub(crate) fn parse_errdefer(&mut self) -> NodeId {
        let sp = self.span();
        self.advance();
        let body = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_stmt() };
        let span = ByteSpan::new(sp.start, self.prev_span().end);
        self.arena.alloc_node(NodeKind::DeferStmt, DEFER_ON_FAILURE, span, body.0, 0)
    }

    /// `become f(args);`
    pub(crate) fn parse_become(&mut self) -> NodeId {
        let sp = self.span();
        self.advance();
        let call = self.parse_expr();
        let _ = self.eat(TokenKind::Semicolon);
        let span = ByteSpan::new(sp.start, self.prev_span().end.max(sp.end));
        self.arena.alloc_node(NodeKind::ReturnStmt, RETURN_TAIL_CALL, span, call.0, 0)
    }
}
