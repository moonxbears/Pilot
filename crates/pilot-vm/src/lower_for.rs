//! `for` loops.
//!
//! `for i in a..b` and `for i in a..=b` count. `for x in xs` walks a list (or
//! anything `list_len`/`map_get` index) by position. Any other value goes
//! through the enumerator protocol that `stdlib/core/syntax/for.pilot` binds:
//! the loop calls its enumerator method once, then the step method before
//! each element (`false` ends the loop) and reads the element member after
//! it.
//!
//! When the iterated value's type is unknown at compile time and some struct
//! implements the protocol, the loop chooses at run time: a struct record is
//! enumerated, anything else is indexed. The body is compiled once either
//! way; only the loop head differs.

use crate::instruction::Opcode;
use crate::lower_expr::{OP_RANGE_HALF_OPEN, OP_RANGE_INCLUSIVE};
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_parser::EnumeratorProtocol;
use pilot_rt::PilotVal;

/// How a loop reaches its elements.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Walk {
    Range,
    Index,
    Enumerate,
    /// Enumerate a struct record, index anything else.
    Either,
}

impl BytecodeCompiler {
    pub(crate) fn compile_for(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        let node = *arena.node(node_id);
        let clause = (node.flags as usize)
            .checked_sub(1)
            .and_then(|i| arena.for_clauses.get(i).copied());
        let Some((iter_id, then_id)) = clause else {
            return self.error(crate::codes::SYNTAX, "malformed `for` expression".to_string());
        };
        if then_id.is_some() {
            let msg = "`for ... then` expressions are not lowered by the seed yet";
            return self.error(crate::codes::SEED, msg.to_string());
        }
        let iter_n = *arena.node(iter_id);
        let is_range = iter_n.node_kind() == NodeKind::BinaryExpr
            && matches!(iter_n.flags, OP_RANGE_INCLUSIVE | OP_RANGE_HALF_OPEN);
        let proto = pilot_parser::bootstrap_tables().enumerator().cloned();
        let static_type = if is_range { None } else { self.static_type(arena, iter_id) };
        let walk = match &proto {
            _ if is_range => Walk::Range,
            Some(p) if self.method_owners.contains_key(&p.get_enumerator) => match &static_type {
                Some(t) if self.declares(t, &p.get_enumerator).is_some() => Walk::Enumerate,
                Some(t) if self.structs.contains_key(t) => Walk::Enumerate,
                Some(_) => Walk::Index,
                None => Walk::Either,
            },
            _ => Walk::Index,
        };

        // Loop state lives in locals so the body cannot clobber it.
        let loop_var = Self::name(arena, node.lhs).to_string();
        let slot = self.alloc_local_reg();
        self.var_slots.insert(loop_var, slot);
        self.reg_symbols.push((slot, node.lhs, 0));
        let r_idx = self.alloc_local_reg();
        let r_end = self.alloc_local_reg();
        let r_seq = self.alloc_local_reg();
        let r_enum = self.alloc_local_reg();

        let mark = self.next_reg;
        if is_range {
            let r_s = self.compile_node(arena, NodeId(iter_n.lhs));
            self.emit(Opcode::Move, r_idx, r_s, 0);
            let r_e = self.compile_node(arena, NodeId(iter_n.rhs));
            self.emit(Opcode::Move, r_end, r_e, 0);
        } else {
            let r_it = self.compile_node(arena, iter_id);
            self.emit(Opcode::Move, r_seq, r_it, 0);
        }
        self.next_reg = mark;
        // `r_enum` is `none` while indexing, the enumerator while enumerating.
        let r_null = self.emit_null();
        self.emit(Opcode::Move, r_enum, r_null, 0);
        if !is_range {
            // The step at the loop's end counts in every mode.
            let r_zero = self.emit_const(PilotVal::from_int(0));
            self.emit(Opcode::Move, r_idx, r_zero, 0);
        }
        let to_index = match walk {
            Walk::Either => {
                let r_type = self.emit_intrinsic("record_type", &[r_seq]);
                Some(self.emit_branch(Opcode::JumpIfNull, r_type, 0))
            }
            _ => None,
        };
        if let (Walk::Enumerate | Walk::Either, Some(p)) = (walk, &proto) {
            let owner = static_type.clone().unwrap_or_default();
            let r = self.emit_method_call(arena, r_seq, &owner, &p.get_enumerator, &[]);
            self.emit(Opcode::Move, r_enum, r, 0);
        }
        let skip_index = (walk == Walk::Either).then(|| self.emit_jump(0));
        if let Some(j) = to_index {
            self.patch_to_here(j);
        }
        if matches!(walk, Walk::Index | Walk::Either) {
            let r_len = self.emit_intrinsic("list_len", &[r_seq]);
            self.emit(Opcode::Move, r_end, r_len, 0);
        }
        if let Some(j) = skip_index {
            self.patch_to_here(j);
        }
        self.next_reg = mark;

        let start = self.here();
        let mut exits = Vec::new();
        let mut to_body = None;
        if let (Walk::Enumerate | Walk::Either, Some(p)) = (walk, &proto) {
            let to_indexed_head =
                (walk == Walk::Either).then(|| self.emit_branch(Opcode::JumpIfNull, r_enum, 0));
            self.emit_enumerator_step(arena, p, r_enum, slot, &mut exits);
            to_body = Some(self.emit_jump(0));
            if let Some(j) = to_indexed_head {
                self.patch_to_here(j);
            }
        }
        if walk != Walk::Enumerate {
            let r_cond = self.alloc_reg();
            let inclusive = is_range && iter_n.flags == OP_RANGE_INCLUSIVE;
            let cmp = if inclusive { Opcode::CmpLe } else { Opcode::CmpLt };
            self.emit(cmp, r_cond, r_idx, r_end);
            exits.push(self.emit_branch(Opcode::JumpIfFalse, r_cond, 0));
            if is_range {
                self.emit(Opcode::Move, slot, r_idx, 0);
            } else {
                let r_item = self.emit_intrinsic("map_get", &[r_seq, r_idx]);
                self.emit(Opcode::Move, slot, r_item, 0);
            }
        }
        if let Some(j) = to_body {
            self.patch_to_here(j);
        }
        self.next_reg = mark;
        let ctx = self.loop_body(arena, NodeId(node.rhs));
        for j in ctx.continue_jumps {
            self.patch_to_here(j);
        }
        let r_one = self.emit_const(PilotVal::from_int(1));
        self.emit(Opcode::Add, r_idx, r_idx, r_one);
        self.emit_jump(start);
        for j in exits.into_iter().chain(ctx.break_jumps) {
            self.patch_to_here(j);
        }
        self.emit_null()
    }

    /// `if !e.move_next() { exit } x = e.current;` for the enumerator in
    /// `r_enum`, with the element stored in `slot`.
    fn emit_enumerator_step(
        &mut self,
        arena: &AstArena,
        p: &EnumeratorProtocol,
        r_enum: u8,
        slot: u8,
        exits: &mut Vec<usize>,
    ) {
        let mark = self.next_reg;
        let r_more = self.emit_method_call(arena, r_enum, "", &p.move_next, &[]);
        exits.push(self.emit_branch(Opcode::JumpIfFalse, r_more, 0));
        let r_cur = self.emit_field_read(r_enum, &p.current);
        self.emit(Opcode::Move, slot, r_cur, 0);
        self.release_to(mark);
    }
}
