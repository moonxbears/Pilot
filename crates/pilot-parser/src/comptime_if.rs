//! `comptime if cond { .. } else { .. }`: the branch is chosen while parsing.
//!
//! The condition is folded here, so the branch not taken is skipped without
//! being parsed into the tree: names and types inside it are never checked,
//! and no engine ever sees it. The taken branch takes the place of the whole
//! `comptime if` (a block, or an empty block when nothing is taken), so the
//! checker and every engine run the program unchanged.
//!
//! A condition folds when it is built from literals, `const`s declared
//! earlier in the same file, `!`, `-`, arithmetic, comparisons and `&&` /
//! `||`. A parameter or local binding is only known when the program runs
//! (CTIME007). Calls, members and names declared in other files are known
//! while compiling but are not evaluated here yet: they stop at the seam
//! `comptime_if.condition`.

use crate::Parser;
use pilot_ast::{BinaryOp, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_span::ByteSpan;
use pilot_lexer::TokenKind;
use std::collections::{HashMap, HashSet};

/// The `else` receptor slot (`stdlib/core/syntax/*.pilot`).
const ELSE_RECEPTOR: u8 = 3;

/// Unary flags the parser writes (`UnaryExpr.flags`).
const UNARY_NEG: u16 = 0;
const UNARY_IDENTITY: u16 = 1;
const UNARY_NOT: u16 = 5;

const NOT_KNOWN: &str = "CTIME007: the condition of `comptime if` is not known while compiling";
const UNCONNECTED: &str = "SEAM001: unconnected seam `comptime_if.condition`: \
    a `comptime if` condition may use only literals and `const`s declared earlier in its file; \
    calls, members and other files' names are not evaluated yet";

/// A folded constant.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Folded {
    Int(i64),
    Bool(bool),
    Text(String),
}

/// What the parser knows while folding: the file's constants so far, and
/// the names that hold run-time values in the function being parsed.
#[derive(Clone, Debug, Default)]
pub(crate) struct FoldScope {
    consts: HashMap<String, Folded>,
    runtime: HashSet<String>,
}

/// Why a condition did not fold.
enum NoFold {
    /// It reads a value the program computes (CTIME007).
    Runtime,
    /// It needs evaluation the parser does not do yet (the seam).
    Unconnected,
}

impl FoldScope {
    pub(crate) fn add_runtime(&mut self, name: &str) {
        self.runtime.insert(name.to_string());
    }

    pub(crate) fn take_runtime(&mut self) -> HashSet<String> {
        std::mem::take(&mut self.runtime)
    }

    pub(crate) fn restore_runtime(&mut self, names: HashSet<String>) {
        self.runtime = names;
    }
}

impl<'a> Parser<'a> {
    /// Records `const name = init;` when `init` folds.
    pub(crate) fn note_const(&mut self, name: SymbolId, init: NodeId) {
        if let Ok(v) = self.fold(init) {
            let n = self.arena.interner.resolve(name).to_string();
            self.folds.runtime.remove(&n);
            self.folds.consts.insert(n, v);
        }
    }

    /// Records a `let` / `var` / parameter name, whose value is run-time.
    pub(crate) fn note_runtime(&mut self, name: SymbolId) {
        let n = self.arena.interner.resolve(name).to_string();
        self.folds.add_runtime(&n);
    }

    /// `comptime if ..`, at `comptime`: the taken branch, or an empty block.
    pub(crate) fn parse_comptime_if(&mut self) -> NodeId {
        let sp = self.span();
        self.advance();
        let mut chosen = None;
        loop {
            // At `if`.
            self.advance();
            self.no_struct_lit = true;
            let cond = self.parse_expr();
            self.no_struct_lit = false;
            let take = chosen.is_none() && self.decide(cond);
            if take {
                chosen = Some(self.parse_block());
            } else {
                self.skip_block();
            }
            if !self.is_rec(ELSE_RECEPTOR) {
                break;
            }
            self.advance();
            if self.at(TokenKind::KwIf) {
                continue;
            }
            if chosen.is_none() {
                chosen = Some(self.parse_block());
            } else {
                self.skip_block();
            }
            break;
        }
        chosen.unwrap_or_else(|| {
            let end = self.prev_span().end;
            self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(sp.start, end), &[])
        })
    }

    /// Whether the branch guarded by `cond` is taken; reports a condition
    /// that does not fold and takes nothing.
    fn decide(&mut self, cond: NodeId) -> bool {
        let span = self.arena.span(cond);
        match self.fold(cond) {
            Ok(Folded::Bool(b)) => b,
            Ok(_) => {
                self.arena.diagnostics.push((span, "CTIME007: the condition of `comptime if` must be a `bool`".into()));
                false
            }
            Err(NoFold::Runtime) => {
                self.arena.diagnostics.push((span, NOT_KNOWN.into()));
                false
            }
            Err(NoFold::Unconnected) => {
                self.arena.diagnostics.push((span, UNCONNECTED.into()));
                false
            }
        }
    }

    /// Skips a `{ .. }` block without building it.
    fn skip_block(&mut self) {
        if !self.at(TokenKind::LBrace) {
            self.expect(TokenKind::LBrace);
            return;
        }
        match self.tape.matching_bracket(self.cur()) {
            Some(close) => self.pos = close as usize + 1,
            None => {
                let _ = self.parse_block();
            }
        }
    }

    fn fold(&self, id: NodeId) -> Result<Folded, NoFold> {
        if id.is_none() {
            return Err(NoFold::Unconnected);
        }
        let n = *self.arena.node(id);
        match n.node_kind() {
            NodeKind::BoolLit => Ok(Folded::Bool(n.lhs != 0)),
            NodeKind::IntLit => Ok(Folded::Int(self.arena.i64_pool.get(n.lhs as usize).copied().unwrap_or(0))),
            NodeKind::StrLit => Ok(Folded::Text(self.arena.interner.resolve(SymbolId(n.lhs)).to_string())),
            NodeKind::IdentExpr => {
                let name = self.arena.interner.resolve(SymbolId(n.lhs));
                if self.folds.runtime.contains(name) {
                    return Err(NoFold::Runtime);
                }
                self.folds.consts.get(name).cloned().ok_or(NoFold::Unconnected)
            }
            NodeKind::UnaryExpr => {
                let v = self.fold(NodeId(n.lhs))?;
                match (n.flags, v) {
                    (UNARY_NOT, Folded::Bool(b)) => Ok(Folded::Bool(!b)),
                    (UNARY_NEG, Folded::Int(i)) => i.checked_neg().map(Folded::Int).ok_or(NoFold::Unconnected),
                    (UNARY_IDENTITY, v) => Ok(v),
                    _ => Err(NoFold::Unconnected),
                }
            }
            NodeKind::BinaryExpr => {
                let op = BinaryOp::from_u16(n.flags).ok_or(NoFold::Unconnected)?;
                let l = self.fold(NodeId(n.lhs))?;
                // `&&` and `||` do not read their right side when the left decides.
                match (op, &l) {
                    (BinaryOp::And, Folded::Bool(false)) => return Ok(Folded::Bool(false)),
                    (BinaryOp::Or, Folded::Bool(true)) => return Ok(Folded::Bool(true)),
                    _ => {}
                }
                let r = self.fold(NodeId(n.rhs))?;
                fold_binary(op, l, r).ok_or(NoFold::Unconnected)
            }
            NodeKind::BlockExpr => {
                // `(cond)` keeps its parentheses as a one-item block.
                let ch = self.arena.children_slice(n.lhs, n.rhs);
                match ch {
                    [only] => self.fold(NodeId(*only)),
                    _ => Err(NoFold::Unconnected),
                }
            }
            NodeKind::ExprStmt => self.fold(NodeId(n.lhs)),
            _ => Err(NoFold::Unconnected),
        }
    }
}

fn fold_binary(op: BinaryOp, l: Folded, r: Folded) -> Option<Folded> {
    use Folded::{Bool, Int, Text};
    Some(match (op, l, r) {
        (BinaryOp::And, Bool(a), Bool(b)) => Bool(a && b),
        (BinaryOp::Or, Bool(a), Bool(b)) => Bool(a || b),
        (BinaryOp::Add, Int(a), Int(b)) => Int(a.checked_add(b)?),
        (BinaryOp::Sub, Int(a), Int(b)) => Int(a.checked_sub(b)?),
        (BinaryOp::Mul, Int(a), Int(b)) => Int(a.checked_mul(b)?),
        (BinaryOp::Div, Int(a), Int(b)) => Int(a.checked_div(b)?),
        (BinaryOp::Rem, Int(a), Int(b)) => Int(a.checked_rem(b)?),
        (BinaryOp::BitAnd, Int(a), Int(b)) => Int(a & b),
        (BinaryOp::BitOr, Int(a), Int(b)) => Int(a | b),
        (BinaryOp::Eq, a, b) => Bool(a == b),
        (BinaryOp::Ne, a, b) => Bool(a != b),
        (BinaryOp::Lt, Int(a), Int(b)) => Bool(a < b),
        (BinaryOp::Le, Int(a), Int(b)) => Bool(a <= b),
        (BinaryOp::Gt, Int(a), Int(b)) => Bool(a > b),
        (BinaryOp::Ge, Int(a), Int(b)) => Bool(a >= b),
        (BinaryOp::Add, Text(a), Text(b)) => Text(a + &b),
        _ => return None,
    })
}

impl<'a> Parser<'a> {
    /// At `comptime if`.
    pub(crate) fn at_comptime_if(&self) -> bool {
        if !self.at(TokenKind::KwComptime) {
            return false;
        }
        let nx = self.next_non_nl(self.cur() + 1);
        nx < self.tape.len() && self.tape.kind(nx) == TokenKind::KwIf
    }
}
