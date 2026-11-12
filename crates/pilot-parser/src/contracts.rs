//! `requires(cond)` and `ensures(r => cond)` after a function's signature,
//! and `old(expr)` inside `ensures` (`stdlib/contracts/contracts_syntax.pilot`
//! binds the words).
//!
//! The clauses become the first statements of the function's body, so the
//! checker resolves and types them where the parameters are in scope:
//!   - `requires(c)` is `ExprStmt(c)` with flags `CONTRACT_REQUIRES`;
//!   - `ensures(q)` is `ExprStmt(q)` with flags `CONTRACT_ENSURES`; with a
//!     result name, `ensures(r => q)`, `q` is the lambda `(r) => q`;
//!   - `old(e)` is `UnaryExpr(e)` with flags `UNARY_OP_OLD`: `e`'s value when
//!     the function was entered.
//! The body `b` (a block or the expression after `=>`) becomes the block
//! `{ clauses..; b }`.

use crate::Parser;
use pilot_ast::{NodeId, NodeKind, UNARY_OP_OLD};
use pilot_lexer::TokenKind;
use pilot_span::ByteSpan;

/// The receptor slot of the contract words and the sub-slots of each.
pub(crate) const CONTRACT_RECEPTOR: u8 = 33;
pub(crate) const REQUIRES_SUB: u8 = 18;
pub(crate) const ENSURES_SUB: u8 = 19;
pub(crate) const INVARIANT_SUB: u8 = 20;
pub(crate) const OLD_SUB: u8 = 24;

/// Struct invariants are parsed and stopped here: nothing checks them yet.
const INVARIANT_UNCONNECTED: &str = "SEAM001: unconnected seam `contracts.invariant`: \
    a struct `invariant` is not checked yet: it needs a check after each construction \
    and at the exit of each method that changes the struct (DBC005)";

/// `ExprStmt.flags` of a `requires` clause.
pub const CONTRACT_REQUIRES: u16 = 1;
/// `ExprStmt.flags` of an `ensures` clause.
pub const CONTRACT_ENSURES: u16 = 2;

impl<'a> Parser<'a> {
    /// At `requires` or `ensures`.
    pub(crate) fn at_contract_clause(&self) -> bool {
        matches!(self.receptor(), Some((CONTRACT_RECEPTOR, REQUIRES_SUB | ENSURES_SUB)))
    }

    /// At `invariant`.
    pub(crate) fn at_invariant(&self) -> bool {
        self.receptor() == Some((CONTRACT_RECEPTOR, INVARIANT_SUB))
    }

    /// `invariant(cond);` in a struct body: parsed, then stopped at its seam,
    /// so an invariant can never look checked while it is not.
    pub(crate) fn parse_invariant(&mut self) {
        let sp = self.span();
        self.advance();
        self.expect(TokenKind::LParen);
        let _ = self.parse_expr();
        self.expect(TokenKind::RParen);
        let span = ByteSpan::new(sp.start, self.prev_span().end);
        self.eat(TokenKind::Semicolon);
        self.arena.diagnostics.push((span, INVARIANT_UNCONNECTED.into()));
    }

    /// One `requires(..)` / `ensures(..)` clause, as its body statement.
    pub(crate) fn parse_contract_clause(&mut self) -> NodeId {
        let sp = self.span();
        let is_ensures = self.receptor() == Some((CONTRACT_RECEPTOR, ENSURES_SUB));
        self.advance();
        self.expect(TokenKind::LParen);
        let cond = if is_ensures {
            self.in_ensures = true;
            let nx = self.next_non_nl(self.cur() + 1);
            let named = self.is_word() && nx < self.tape.len() && self.tape.kind(nx) == TokenKind::FatArrow;
            let q = if named { self.parse_result_lambda() } else { self.parse_expr() };
            self.in_ensures = false;
            q
        } else {
            self.parse_expr()
        };
        self.expect(TokenKind::RParen);
        let flags = if is_ensures { CONTRACT_ENSURES } else { CONTRACT_REQUIRES };
        let span = ByteSpan::new(sp.start, self.prev_span().end);
        self.arena.alloc_node(NodeKind::ExprStmt, flags, span, cond.0, 0)
    }

    /// `r => q` inside `ensures(..)`: the lambda `(r) => q`.
    fn parse_result_lambda(&mut self) -> NodeId {
        let sp = self.span();
        let name = self.intern_cur();
        self.expect(TokenKind::FatArrow);
        let param = self.arena.alloc_node(NodeKind::ParamDecl, 0, sp, name.0, 0);
        let body = self.parse_expr();
        let (st, ln) = self.arena.alloc_child_slice(&[param]);
        let span = ByteSpan::new(sp.start, self.prev_span().end);
        self.arena.alloc_node(NodeKind::LambdaExpr, ln as u16, span, st, body.0)
    }

    /// `old(e)` inside `ensures`, at `old`.
    pub(crate) fn parse_old(&mut self) -> NodeId {
        let sp = self.span();
        self.advance();
        self.expect(TokenKind::LParen);
        let e = self.parse_expr();
        self.expect(TokenKind::RParen);
        let span = ByteSpan::new(sp.start, self.prev_span().end);
        self.arena.alloc_node(NodeKind::UnaryExpr, UNARY_OP_OLD, span, e.0, 0)
    }

    /// Puts `clauses` in front of `body`: the body becomes
    /// `{ clauses..; body }`. A block body is kept whole as the last item
    /// rather than copied, so every node keeps the one parent it has.
    pub(crate) fn with_contracts(&mut self, body: NodeId, clauses: &[NodeId]) -> NodeId {
        if clauses.is_empty() || body.is_none() {
            return body;
        }
        let span = self.arena.span(body);
        let mut items = clauses.to_vec();
        items.push(self.arena.alloc_node(NodeKind::ExprStmt, 0, span, body.0, 0));
        self.arena.alloc_list_node(NodeKind::BlockExpr, 0, span, &items)
    }
}
