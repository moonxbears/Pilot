//! The raw text of an inline foreign block: `asm { .. }`, `c { .. }`.
//!
//! The text between the braces is kept as written for the foreign tool. A
//! block whose `{` is never closed is reported and runs to the end of the
//! file.

use crate::Parser;
use pilot_ast::NodeKind;
use pilot_lexer::TokenKind;
use pilot_span::ByteSpan;

impl<'a> Parser<'a> {
    /// At the `{` of an inline block: its inner text, with the cursor moved
    /// past the closing `}`.
    pub(crate) fn inline_block_code(&mut self) -> &'a str {
        let open = self.span();
        let Some(close) = self.tape.matching_bracket(self.cur()) else {
            self.arena.diagnostics.push((open, "SyntaxError: this `{` is never closed".into()));
            let src = self.src;
            self.pos = self.tape.len();
            return src.get(open.end as usize..).unwrap_or("");
        };
        let close = close as usize;
        let src = self.src;
        let code = src.get(open.end as usize..self.tape.span(close).start as usize).unwrap_or("");
        self.pos = close + 1;
        code
    }

    /// Reports each character no lexer rule takes, outside inline foreign
    /// blocks (whose text belongs to the foreign tool): before this, a stray
    /// `└` between two expressions was skipped without a word.
    pub(crate) fn report_stray_characters(&mut self) {
        let foreign: Vec<ByteSpan> = (0..self.arena.nodes.len())
            .filter(|&i| self.arena.nodes[i].node_kind() == NodeKind::PolyglotBlock)
            .map(|i| self.arena.spans[i])
            .collect();
        for i in 0..self.tape.len() {
            if self.tape.kind(i) != TokenKind::ErrorToken {
                continue;
            }
            let sp = self.tape.span(i);
            if foreign.iter().any(|f| f.start <= sp.start && sp.end <= f.end) {
                continue;
            }
            let ch = self.tape.text(i, self.src);
            self.arena.diagnostics.push((sp, format!("SyntaxError: unexpected character `{ch}`")));
        }
    }
}
