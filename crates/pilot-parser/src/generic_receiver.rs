//! Generic arguments on a receiver in expression position.
//!
//! `Name<A, B>.member(..)` keeps `Name` as the receiver and drops the
//! arguments, which the seed does not use in expressions. A `syntax
//! type_construct` rule that rewrites `Name<A, B>` to another named type
//! (`res<T, E>` to `result<T, E>`) applies here too, so both spellings reach
//! the same type, as the Dual Readable/Symbolic Pairing Law asks.

use crate::{Parser, TemplateNode, TypeConstructRule};
use pilot_ast::{NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_lexer::TokenKind;

impl Parser<'_> {
    /// `Name<A, B>` in expression position (`res<i64, str>.err(..)`): when a
    /// `syntax type_construct` rule rewrites `Name<A, B>` to another named
    /// type, the receiver takes that type's name, as it would in a type.
    pub(crate) fn rewrite_generic_receiver(&mut self, lhs: NodeId) {
        let ln = *self.arena.node(lhs);
        if ln.node_kind() != NodeKind::IdentExpr { return; }
        let nm = self.arena.interner.resolve(SymbolId(ln.lhs)).to_string();
        let (mut p, mut depth, mut count) = (self.cur(), 0i32, 1usize);
        while p < self.tape.len() {
            match self.tape.kind(p) {
                TokenKind::Less | TokenKind::LParen | TokenKind::LBracket => depth += 1,
                TokenKind::Greater | TokenKind::RParen | TokenKind::RBracket => { depth -= 1; if depth == 0 { break; } }
                TokenKind::ShiftRight => { depth -= 2; if depth <= 0 { break; } }
                TokenKind::Comma if depth == 1 => count += 1,
                _ => {}
            }
            p += 1;
        }
        let head = self.tables.type_rules.iter().find_map(|r| match r {
            TypeConstructRule::NamedGeneric { name, params, template: TemplateNode::TypeNamed(base, _) }
                if name == &nm && params.len() == count && !params.contains(base) => Some(base.clone()),
            _ => None,
        });
        if let Some(base) = head {
            let sym = self.arena.interner.intern(&base);
            self.arena.nodes[lhs.0 as usize].lhs = sym.0;
        }
    }
}
