//! Generic parameter lists of declarations: `<T : IA + IB, U = void>`.
//!
//! Each parameter keeps its name, the bound types after its `:` and the
//! default type after its `=`, which a use that leaves the parameter out gets
//! (`Generator<i64>` is `Generator<i64, void>`).

use crate::{is_seed_kw, ModAct, Parser};
use pilot_ast::NodeId;
use pilot_intern::SymbolId;
use pilot_lexer::TokenKind;

/// A generic parameter: its name, its bound types and its default type
/// (`NodeId::NONE` when it has none).
pub(crate) type GenericParam = (SymbolId, Vec<NodeId>, NodeId);

impl Parser<'_> {
    /// `<T : IA + IB, U, const N = 4>`: each parameter's name, the bound
    /// types after its `:` and the default type after its `=`.
    pub(crate) fn parse_generic_params(&mut self) -> Vec<GenericParam> {
        let mut out: Vec<GenericParam> = Vec::new();
        if !self.eat(TokenKind::Less) {
            return out;
        }
        while !self.at_end() && !self.at(TokenKind::Greater) && !self.at(TokenKind::ShiftRight) {
            if self.is_word() && self.tables.modifiers.get(self.text()) == Some(&ModAct::Const) {
                self.advance();
            }
            if self.is_word() || is_seed_kw(self.peek()) {
                out.push((self.intern_cur(), Vec::new(), NodeId::NONE));
            } else {
                self.advance();
            }
            if self.eat(TokenKind::Colon) {
                let mut bounds = vec![self.parse_type()];
                while self.eat(TokenKind::Plus) {
                    bounds.push(self.parse_type());
                }
                if let Some(last) = out.last_mut() {
                    last.1.extend(bounds.into_iter().filter(|b| b.is_some()));
                }
            }
            if self.eat(TokenKind::Equal) {
                let default = self.parse_type();
                if let Some(last) = out.last_mut() {
                    last.2 = default;
                }
            }
            if self.eat(TokenKind::Comma) {
                if self.at(TokenKind::Greater) {
                    let msg = "Trailing comma not permitted in generic parameters".into();
                    self.arena.diagnostics.push((self.prev_span(), msg));
                    break;
                }
            } else {
                break;
            }
        }
        if !self.eat(TokenKind::Greater) && self.at(TokenKind::ShiftRight) {
            let p = self.cur();
            self.tape.kinds[p] = TokenKind::Greater;
        }
        out
    }

    /// Records the generic parameters, their bounds and defaults on `owner`.
    pub(crate) fn add_type_params(&mut self, owner: NodeId, tps: Vec<GenericParam>) {
        for (tp, bounds, default) in tps {
            self.arena.type_params.push((owner, tp));
            if default.is_some() {
                self.arena.type_param_defaults.push((owner, tp, default));
            }
            for b in bounds {
                self.arena.type_param_bounds.push((owner, tp, b));
            }
        }
    }
}
