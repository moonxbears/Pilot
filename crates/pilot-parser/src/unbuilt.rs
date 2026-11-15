//! Keywords that are registered but not built yet.
//!
//! A modifier the library registers with an action the parser does not carry
//! out (the library writes `"keyword_unbuilt"`), or the `asm { .. }` block,
//! is a keyword whose meaning is promised (`stdlib/core/keywords.pilot`) but
//! not implemented. Any use of it stops the build at the seam
//! `keyword.unbuilt`, so the word can never be read as something else, or
//! skipped, and appear to work (AGENTS.md section 11, rule 2).
//! `docs/keyword_roadmap.md` lists the phase that builds each one. Unbuilt
//! words that start a construct are `syntax` rules whose template is
//! `form(CoreForm::Reject, ..)` (PROP-009 section 13), not seed code.

use crate::Parser;
use pilot_span::ByteSpan;

/// The seam every use of an unbuilt keyword stops at.
const UNBUILT_SEAM: &str = "SEAM001: unconnected seam `keyword.unbuilt`";

impl<'a> Parser<'a> {
    /// Reports `word` (at `span`) as a keyword that is not built yet.
    pub(crate) fn report_unbuilt(&mut self, span: ByteSpan, word: &str) {
        let message = format!(
            "{UNBUILT_SEAM}: `{word}` is registered but not built yet \
             (docs/keyword_roadmap.md lists the phase that builds it)"
        );
        self.arena.diagnostics.push((span, message));
    }
}
