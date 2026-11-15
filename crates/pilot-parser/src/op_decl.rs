//! Telling an operator declaration from a name that reads `op`.
//!
//! `op` is the word that starts an operator declaration (`op + (rhs: T)`,
//! `op [] (i: usize)`, `op flat () -> T`), and it is also an ordinary name
//! (`var op = 1; op = op + 2;`). Inside a type body it always declares.
//! Elsewhere it declares only when what follows has the shape of one: the
//! operator spelled as symbols or as one word, then `(`; or `()` for the
//! call operator. `op - x(..)` (symbols, then a word) is an expression.

use crate::Parser;
use pilot_lexer::TokenKind;

impl Parser<'_> {
    /// Whether the `op` word at the cursor starts an operator declaration.
    pub(crate) fn op_decl_follows(&self, in_type_body: bool) -> bool {
        if in_type_body {
            return true;
        }
        let mut p = self.next_non_nl(self.cur() + 1);
        let mut symbols = 0;
        let mut words = 0;
        while p < self.tape.len() {
            match self.tape.kind(p) {
                // `op () -> R` declares the call operator.
                TokenKind::LParen if symbols == 0 && words == 0 => {
                    let q = self.next_non_nl(p + 1);
                    return q < self.tape.len() && self.tape.kind(q) == TokenKind::RParen;
                }
                TokenKind::LParen => return (symbols > 0) != (words == 1),
                TokenKind::Ident if symbols == 0 && words == 0 => words += 1,
                TokenKind::Ident
                | TokenKind::Semicolon
                | TokenKind::Comma
                | TokenKind::RParen
                | TokenKind::RBrace
                | TokenKind::InfixDot
                | TokenKind::Newline => return false,
                k if is_assign(k) => return false,
                _ if words == 0 => symbols += 1,
                _ => return false,
            }
            p += 1;
        }
        false
    }
}

/// `=` and the compound assignments: `op = ..` assigns to a name.
fn is_assign(k: TokenKind) -> bool {
    matches!(
        k,
        TokenKind::Equal
            | TokenKind::PlusEqual
            | TokenKind::MinusEqual
            | TokenKind::StarEqual
            | TokenKind::SlashEqual
            | TokenKind::PercentEqual
            | TokenKind::AmperEqual
            | TokenKind::VbarEqual
            | TokenKind::CaretEqual
            | TokenKind::ShiftLeftEqual
            | TokenKind::ShiftRightEqual
            | TokenKind::CoalesceEqual
    )
}
