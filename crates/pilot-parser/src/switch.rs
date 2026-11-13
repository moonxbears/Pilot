//! `switch` and its arms.
//!
//! An arm is `case <patterns> [if <guard>]:` (or `=>`) followed by its body,
//! or `default:` / `_`. Several patterns separated by commas match when any of
//! them does, the same as joining them with `|`: `case 1, 2:` is `case 1 | 2:`.

use crate::Parser;
use pilot_ast::{NodeId, NodeKind};
use pilot_lexer::TokenKind;
use pilot_span::ByteSpan;

impl<'a> Parser<'a> {
    pub(crate) fn parse_switch(&mut self) -> NodeId {
        let sp = self.span();
        self.advance();
        self.no_struct_lit = true;
        let scrut = self.parse_expr();
        self.no_struct_lit = false;
        self.expect(TokenKind::LBrace);
        let mut ch = vec![scrut];
        while !self.at_end() && !self.at(TokenKind::RBrace) {
            self.skip_seps();
            if self.at(TokenKind::RBrace) || self.at_end() {
                break;
            }
            let m = self.pos;
            ch.push(self.parse_switch_arm());
            if self.pos == m {
                // Nothing in the arm could be read: report it and stop here
                // rather than loop on the same token.
                let (span, got) = (self.span(), self.peek());
                self.arena.diagnostics.push((span, format!("Expected a switch arm, found {:?}", got)));
                break;
            }
        }
        self.expect(TokenKind::RBrace);
        self.arena.alloc_list_node(NodeKind::MatchExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), &ch)
    }

    fn parse_switch_arm(&mut self) -> NodeId {
        let asp = self.span();
        let pat = if self.is_rec(16) || (self.is_word() && self.text() == "_") {
            self.advance();
            self.arena.alloc_node(NodeKind::PatWildcard, 0, asp, 0, 0)
        } else {
            if self.is_rec(15) {
                self.advance();
            }
            self.parse_arm_patterns()
        };
        let mut guard = NodeId::NONE;
        if self.eat(TokenKind::KwIf) || self.is_rec(28) {
            if self.is_rec(28) {
                self.advance();
            }
            self.no_struct_lit = true;
            guard = self.parse_expr();
            self.no_struct_lit = false;
        }
        let is_colon = self.eat(TokenKind::Colon);
        if !is_colon {
            let _ = self.eat(TokenKind::FatArrow);
        }
        let body = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_arm_body(asp, is_colon) };
        let _ = self.eat(TokenKind::Comma) || self.eat(TokenKind::Semicolon);
        self.arena.alloc_match_arm(pat, guard, body, ByteSpan::new(asp.start, self.prev_span().end))
    }

    /// `p1, p2, ...` folded into `p1 | p2 | ...`.
    fn parse_arm_patterns(&mut self) -> NodeId {
        let mut lhs = self.parse_pattern();
        while self.eat(TokenKind::Comma) {
            let sp = self.arena.span(lhs);
            let rhs = self.parse_pattern();
            lhs = self.arena.alloc_node(NodeKind::PatOr, 0, sp, lhs.0, rhs.0);
        }
        lhs
    }

    /// The statements of an arm without braces: after `:` they run to the
    /// next arm; after `=>` the arm is one statement.
    fn parse_arm_body(&mut self, asp: ByteSpan, is_colon: bool) -> NodeId {
        let mut stmts = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) && !self.is_rec(15) && !self.is_rec(16) {
            let m = self.pos;
            let s = self.parse_stmt();
            if !s.is_none() {
                stmts.push(s);
            }
            if self.eat(TokenKind::Comma) || self.pos == m || (!is_colon && stmts.len() == 1) {
                break;
            }
        }
        if stmts.len() == 1 {
            let s = stmts[0];
            if self.arena.node(s).node_kind() == NodeKind::ExprStmt { NodeId(self.arena.node(s).lhs) } else { s }
        } else {
            self.arena.alloc_list_node(NodeKind::BlockExpr, 0, asp, &stmts)
        }
    }
}
