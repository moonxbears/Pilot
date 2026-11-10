//! Accessor blocks of properties and indexers: `{ get => ..; set => { .. } }`.
//!
//! The words that open a getter and a setter are the receptors of category
//! 23 and 24 (`stdlib/core/syntax/properties.pilot`). The block keeps the
//! bodies that are written, getters first, with the number of getters in its
//! `flags`. Each setter body gets a row in `AstArena::property_setters` with
//! its parameter (`set(v: T)`) and guard (`set if cond`), when it has them.

use crate::Parser;
use pilot_ast::{NodeId, NodeKind};
use pilot_lexer::TokenKind;
use pilot_span::ByteSpan;

/// The method names of `op []` (read) and `op []=` (write).
pub const INDEX_GET_OP: &str = "[]";
pub const INDEX_SET_OP: &str = "[]=";

/// Receptor category of the word that opens a getter.
const GETTER_RECEPTOR: u8 = 23;
/// Receptor category of the word that opens a setter.
const SETTER_RECEPTOR: u8 = 24;

impl Parser<'_> {
    pub(crate) fn skip_accessor_block(&mut self) -> NodeId {
        let block_span = self.span();
        self.advance();
        let mut getters = Vec::new();
        let mut setters = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) {
            self.skip_seps();
            if self.at(TokenKind::RBrace) || self.at_end() {
                break;
            }
            let _ = self.parse_mods(true);
            let is_getter = self.is_rec(GETTER_RECEPTOR);
            if !is_getter && !self.is_rec(SETTER_RECEPTOR) {
                self.advance();
                continue;
            }
            self.advance();
            let param = if self.at(TokenKind::LParen) {
                self.parse_param_list().first().copied().unwrap_or(NodeId::NONE)
            } else {
                NodeId::NONE
            };
            let guard = if self.eat(TokenKind::KwIf) {
                self.no_struct_lit = true;
                let g = self.parse_expr();
                self.no_struct_lit = false;
                g
            } else {
                NodeId::NONE
            };
            // `set => { field = value; }` is a block, never a map literal.
            let arrow = self.eat(TokenKind::FatArrow);
            let body = if arrow && !self.at(TokenKind::LBrace) {
                let e = self.parse_expr();
                let _ = self.eat(TokenKind::Semicolon);
                e
            } else if self.at(TokenKind::LBrace) {
                self.parse_block()
            } else {
                let _ = self.eat(TokenKind::Semicolon);
                NodeId::NONE
            };
            if body.is_none() {
                continue;
            }
            if is_getter {
                getters.push(body);
            } else {
                setters.push(body);
                self.arena.property_setters.push((body, param, guard));
            }
        }
        self.expect(TokenKind::RBrace);
        let span = ByteSpan::new(block_span.start, self.prev_span().end);
        let count = getters.len() as u16;
        getters.extend(setters);
        self.arena.alloc_list_node(NodeKind::BlockExpr, count, span, &getters)
    }
}
