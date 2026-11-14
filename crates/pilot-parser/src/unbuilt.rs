//! Keywords that are registered but not built yet.
//!
//! A word the library binds to receptor kind [`UNBUILT_RECEPTOR`] (the
//! receptor's second number is the place where it starts its construct), or a
//! modifier it registers with an action the parser does not carry out (the
//! library writes `"keyword_unbuilt"`), is a keyword whose meaning is promised (`stdlib/core/keywords.pilot`) but not
//! implemented. Any use of it stops the build at the seam `keyword.unbuilt`,
//! so the word can never be read as something else, or skipped, and appear to
//! work (AGENTS.md section 11, rule 2). `docs/keyword_roadmap.md` lists the
//! phase that builds each one. The parser knows no keyword by name here: the
//! library decides which words are unbuilt.

use crate::Parser;
use pilot_ast::NodeId;
use pilot_lexer::TokenKind;
use pilot_span::ByteSpan;

/// The receptor kind of a registered keyword that is not built yet.
pub(crate) const UNBUILT_RECEPTOR: u8 = 40;

/// Receptor place: the word starts its construct in expression position
/// too (`each xs`), as well as where a declaration or statement starts.
const PLACE_ANYWHERE: u8 = 0;

/// Receptor place: the word starts its construct only where a declaration
/// or statement starts (`actor Counter { .. }`).
const PLACE_DECLARATION: u8 = 1;

/// Receptor place: the word is a clause of a query (`from x in xs order ..`).
const PLACE_CLAUSE: u8 = 2;

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

    /// The second number of the unbuilt receptor says where the word can
    /// start its construct, so that elsewhere it stays an ordinary name.
    fn unbuilt_place(&self) -> Option<u8> {
        match self.receptor() {
            Some((UNBUILT_RECEPTOR, place)) => Some(place),
            _ => None,
        }
    }

    /// At an unbuilt word in expression position (place 0) that starts its
    /// construct: the next token is a name, a literal, `{` or `[`. Followed
    /// by anything else (`.`, `(`, `)`, `,`, `;`, `=`, `:`, an operator) the
    /// word is an ordinary identifier (`database.len()`, `f(each)`), as it
    /// was before the keyword was registered.
    pub(crate) fn at_unbuilt(&self) -> bool {
        if self.unbuilt_place() != Some(PLACE_ANYWHERE) {
            return false;
        }
        let nx = self.next_non_nl(self.cur() + 1);
        nx < self.tape.len()
            && (matches!(self.tape.kind(nx), TokenKind::LBracket) || self.starts_unbuilt_body(nx))
    }

    /// At an unbuilt word where a declaration or statement starts (places 0
    /// and 1), followed by a name, a literal or `{`. No declaration or
    /// statement starts with an ordinary name followed by one of those, so
    /// `values[i] = 1;`, `state.push(x);` and `update(x);` stay ordinary.
    pub(crate) fn at_unbuilt_decl(&self) -> bool {
        if !matches!(self.unbuilt_place(), Some(PLACE_ANYWHERE | PLACE_DECLARATION)) {
            return false;
        }
        let nx = self.next_non_nl(self.cur() + 1);
        nx < self.tape.len() && self.starts_unbuilt_body(nx)
    }

    /// At an unbuilt word that is a clause of a query (place 2).
    pub(crate) fn at_unbuilt_clause(&self) -> bool {
        self.unbuilt_place() == Some(PLACE_CLAUSE)
    }

    /// Whether the token at `at` can follow an unbuilt word as the start of
    /// its construct: a name, a literal or `{`.
    fn starts_unbuilt_body(&self, at: usize) -> bool {
        matches!(
            self.tape.kind(at),
            TokenKind::Ident
                | TokenKind::LBrace
                | TokenKind::IntLit
                | TokenKind::HexIntLit
                | TokenKind::BinIntLit
                | TokenKind::OctIntLit
                | TokenKind::FloatLit
                | TokenKind::StrLit
                | TokenKind::TripleStrLit
        )
    }

    /// An unbuilt keyword and the construct it starts: reported, and read
    /// past so that the rest of the file still parses. The construct ends at
    /// a `;`, after a `{ .. }` block, or before a closer it did not open.
    pub(crate) fn parse_unbuilt(&mut self) -> NodeId {
        let sp = self.span();
        let word = self.text().to_string();
        self.advance();
        let mut depth = 0usize;
        while !self.at_end() {
            match self.peek() {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    if depth == 0 && self.peek() == TokenKind::RBrace {
                        self.advance();
                        break;
                    }
                }
                TokenKind::Comma if depth == 0 => break,
                TokenKind::Semicolon if depth == 0 => {
                    self.advance();
                    break;
                }
                _ => {}
            }
            self.advance();
        }
        let end = self.prev_span().end.max(sp.end);
        self.report_unbuilt(ByteSpan::new(sp.start, end), &word);
        NodeId::NONE
    }
}
