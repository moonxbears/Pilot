//! The raw text of an inline foreign block: `asm { .. }`, `c { .. }`.
//!
//! The text between the braces is kept as written for the foreign tool. A
//! block whose `{` is never closed is reported and runs to the end of the
//! file.

use crate::Parser;
use pilot_lexer::TokenKind;

impl<'a> Parser<'a> {
    /// At the `{` of an inline block: its inner text, with the cursor moved
    /// past the closing `}`.
    pub(crate) fn inline_block_code(&mut self) -> &'a str {
        let open = self.span();
        let Some(close) = self.tape.matching_bracket(self.cur()) else {
            self.arena
                .diagnostics
                .push((open, "SyntaxError: this `{` is never closed".into()));
            let src = self.src;
            self.pos = self.tape.len();
            return src.get(open.end as usize..).unwrap_or("");
        };
        let close = close as usize;
        let src = self.src;
        let code = src
            .get(open.end as usize..self.tape.span(close).start as usize)
            .unwrap_or("");
        self.pos = close + 1;
        code
    }

    /// Reports each character no lexer rule takes: before this, a stray
    /// `└` between two expressions was skipped without a word.
    pub(crate) fn report_stray_characters(&mut self) {
        for i in 0..self.tape.len() {
            if self.tape.kind(i) != TokenKind::ErrorToken {
                continue;
            }
            let sp = self.tape.span(i);
            let ch = self.tape.text(i, self.src);
            self.arena
                .diagnostics
                .push((sp, format!("SyntaxError: unexpected character `{ch}`")));
        }
    }
}
