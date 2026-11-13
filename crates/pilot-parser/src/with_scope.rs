//! `with open name { .. }` and `with open { .. }`: a block run inside a
//! scope that `open()` starts and its value's destructor ends.
//!
//! The word bound to receptor slot 30, sub-slot 1 (`with`,
//! `stdlib/core/syntax/module_ext.pilot`) followed by a function name, an
//! optional binding name and a block is the block
//! `{ let name = open(); { .. } }`: the value `open()` returns (a `0b11`
//! guard such as `arena()`'s `ArenaScope`) lives exactly as long as the
//! block and is dropped, running its `~self()`, when the block is left.
//! Without a name the binding is hidden (`_` then the function's name). The
//! same word inside a `*Bundle` splice keeps its collision meaning
//! (`with { flags as net_flags }`), where a `{` follows it at once.

use crate::{is_word_tok, Parser};
use pilot_ast::{NodeId, NodeKind};
use pilot_lexer::TokenKind;
use pilot_span::ByteSpan;

/// The receptor slot and sub-slot of the scope word.
pub(crate) const WITH_RECEPTOR: (u8, u8) = (30, 1);

impl<'a> Parser<'a> {
    /// At `with open {` or `with open name {`.
    pub(crate) fn at_with_scope(&self) -> bool {
        if self.receptor() != Some(WITH_RECEPTOR) {
            return false;
        }
        let f = self.next_non_nl(self.cur() + 1);
        if f >= self.tape.len() || !is_word_tok(self.tape.kind(f)) {
            return false;
        }
        let n = self.next_non_nl(f + 1);
        if n < self.tape.len() && self.tape.kind(n) == TokenKind::LBrace {
            return true;
        }
        if n >= self.tape.len() || !is_word_tok(self.tape.kind(n)) {
            return false;
        }
        let b = self.next_non_nl(n + 1);
        b < self.tape.len() && self.tape.kind(b) == TokenKind::LBrace
    }

    /// `with open [name] { .. }` as `{ let name = open(); { .. } }`.
    pub(crate) fn parse_with_scope(&mut self) -> NodeId {
        let sp = self.span();
        self.advance();
        let fsp = self.span();
        let open = self.text().to_string();
        self.advance();
        let name = if self.at(TokenKind::LBrace) {
            format!("_{open}")
        } else {
            let n = self.text().to_string();
            self.advance();
            n
        };
        let body = self.parse_block();
        let callee = self.arena.alloc_ident(&open, fsp);
        let call = self.arena.alloc_list_node(NodeKind::CallExpr, 0, fsp, &[callee]);
        let sym = self.arena.interner.intern(&name);
        let span = ByteSpan::new(sp.start, self.prev_span().end);
        let binding = self.arena.alloc_node(NodeKind::LetStmt, 0, ByteSpan::new(sp.start, fsp.end), sym.0, call.0);
        self.arena.alloc_list_node(NodeKind::BlockExpr, 0, span, &[binding, body])
    }
}
