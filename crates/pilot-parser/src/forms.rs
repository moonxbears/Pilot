//! The nodes a `syntax` rule's template builds with `form(..)` (PROP-009
//! step 2), by position in the enum `register_core_forms` names
//! (stdlib/core/syntax/forms.pilot; closed in the seed, append-only under
//! B+). Each builds the node the parser built for the word before its rule
//! moved to Pilot, so the checker and the engines read it unchanged.

use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_span::ByteSpan;

const FOR: u8 = 0;
const BREAK: u8 = 1;
const CONTINUE: u8 = 2;
const GUARD: u8 = 3;
const TRY: u8 = 4;
const CATCH: u8 = 5;
const SWITCH: u8 = 6;
const ARM: u8 = 7;
const REJECT: u8 = 8;
const ERR_DEFER: u8 = 9;
const BECOME: u8 = 10;
const REJECT_WORD: u8 = 11;

/// `DeferStmt.flags` of an `errdefer`: it runs only when its block is left
/// by a failure.
pub const DEFER_ON_FAILURE: u16 = 1;
/// `ReturnStmt.flags` of a `become`: a guaranteed tail call.
pub const RETURN_TAIL_CALL: u16 = 1;

/// Whether the seed can build form `form`.
pub(crate) fn has_constructor(form: u8) -> bool {
    form <= REJECT_WORD
}

/// Builds form `form` from `ops` (an absent operand is `NONE`).
pub(crate) fn build_form(arena: &mut AstArena, form: u8, ops: &[NodeId], span: ByteSpan) -> NodeId {
    let op = |i: usize| ops.get(i).copied().unwrap_or(NodeId::NONE);
    let present = |from: usize| {
        ops.iter()
            .skip(from)
            .copied()
            .filter(|id| id.is_some())
            .collect::<Vec<_>>()
    };
    match form {
        FOR => {
            let sym = binder(arena, op(0));
            arena.alloc_for(sym, op(1), op(2), op(3), span)
        }
        BREAK | CONTINUE => {
            let vals = present(0);
            let v = match vals.as_slice() {
                [] => NodeId::NONE,
                [one] => *one,
                _ => arena.alloc_list_node(NodeKind::TupleExpr, 0, span, &vals),
            };
            let kind = if form == BREAK {
                NodeKind::BreakStmt
            } else {
                NodeKind::ContinueStmt
            };
            arena.alloc_node(kind, 0, span, v.0, 0)
        }
        // `guard some x = v else { .. }`: flags 1, and a binding keeps its
        // name as `AssignStmt(x, v)`.
        GUARD => {
            let is_some = op(0).is_some() && arena.node(op(0)).lhs != 0;
            let mut c = op(1);
            if op(2).is_some() {
                c = arena.alloc_node(NodeKind::AssignStmt, 0, span, c.0, op(2).0);
            }
            arena.alloc_node(NodeKind::GuardStmt, u16::from(is_some), span, c.0, op(3).0)
        }
        TRY => arena.alloc_list_node(NodeKind::TryStmt, 0, span, &present(0)),
        CATCH => {
            let sym = if op(0).is_some() {
                arena.node(op(0)).lhs
            } else {
                0
            };
            arena.alloc_node(NodeKind::CatchClause, 0, span, sym, op(1).0)
        }
        SWITCH => {
            let mut ch = vec![op(0)];
            ch.extend(present(1));
            arena.alloc_list_node(NodeKind::MatchExpr, 0, span, &ch)
        }
        // Several patterns match when any does (`case 1, 2:` is `case 1 | 2:`).
        ARM => {
            let mut pat = NodeId::NONE;
            for p in present(2) {
                pat = if pat.is_none() {
                    p
                } else {
                    arena.alloc_node(NodeKind::PatOr, 0, arena.span(pat), pat.0, p.0)
                };
            }
            if pat.is_none() {
                pat = arena.alloc_node(NodeKind::PatWildcard, 0, span, 0, 0);
            }
            let mut body = op(1);
            if body.is_some() && arena.node(body).node_kind() == NodeKind::ExprStmt {
                body = NodeId(arena.node(body).lhs);
            }
            arena.alloc_match_arm(pat, op(0), body, span)
        }
        // `RejectWord` reports at the rule's words, from its first word up
        // to the code, not at the code.
        REJECT | REJECT_WORD => {
            if op(1).is_some() {
                let msg = if op(0).is_some() {
                    arena
                        .interner
                        .resolve(SymbolId(arena.node(op(0)).lhs))
                        .to_string()
                } else {
                    String::new()
                };
                let code = arena.span(op(1));
                let at = if form == REJECT_WORD {
                    ByteSpan::new(span.start, code.start.max(span.start))
                } else {
                    code
                };
                arena.diagnostics.push((at, msg));
            }
            NodeId::NONE
        }
        ERR_DEFER => arena.alloc_node(NodeKind::DeferStmt, DEFER_ON_FAILURE, span, op(0).0, 0),
        BECOME => arena.alloc_node(NodeKind::ReturnStmt, RETURN_TAIL_CALL, span, op(0).0, 0),
        _ => NodeId::NONE,
    }
}

/// The name a binder node declares; in a rule template `$name` declares the
/// name hole `name` holds.
fn binder(arena: &mut AstArena, id: NodeId) -> SymbolId {
    if id.is_none() {
        return arena.interner.intern("_");
    }
    let n = *arena.node(id);
    if n.flags == 1 {
        let hole = format!("${}", arena.interner.resolve(SymbolId(n.lhs)));
        return arena.interner.intern(&hole);
    }
    SymbolId(n.lhs)
}
