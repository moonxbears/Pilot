//! Lowering of `switch` (patterns and arm guards) and `guard` statements.
//!
//! A `switch` evaluates its subject once, then tests each arm's pattern in
//! order. A pattern either matches (binding its names for the arm) or jumps
//! to the next arm. When no arm matches, the `switch` yields `null`. `break`
//! inside an arm leaves the `switch`; `continue` reaches the enclosing loop.

use crate::instruction::Opcode;
use crate::{BytecodeCompiler, LoopCtx};
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;

impl BytecodeCompiler {
    pub(crate) fn compile_match(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        let node = *arena.node(node_id);
        let children = arena.children_slice(node.lhs, node.rhs).to_vec();
        let Some((&subject, arms)) = children.split_first() else {
            return self.emit_null();
        };
        let dst = self.alloc_reg();
        let r_subject = self.alloc_reg();
        let mark = self.next_reg;
        let r = self.compile_node(arena, NodeId(subject));
        self.emit(Opcode::Move, r_subject, r, 0);
        let r_null = self.emit_null();
        self.emit(Opcode::Move, dst, r_null, 0);
        self.release_to(mark);
        let subject_type = self.static_type(arena, NodeId(subject));

        self.loops.push(LoopCtx {
            continue_jumps: Vec::new(),
            break_jumps: Vec::new(),
            is_switch: true,
            defer_depth: self.defers.len(),
        });
        let mut end_jumps = Vec::new();
        for &arm in arms {
            let arm_id = NodeId(arm);
            let an = *arena.node(arm_id);
            if an.node_kind() != NodeKind::MatchArm {
                continue;
            }
            self.cur_span = arena.span(arm_id);
            let outer_slots = self.var_slots.clone();
            let mut fails = Vec::new();
            self.compile_pattern(arena, NodeId(an.lhs), r_subject, &subject_type, &mut fails);
            let guard = arena.arm_guard(arm_id);
            if guard.is_some() {
                let r_g = self.compile_node(arena, guard);
                fails.push(self.emit_branch(Opcode::JumpIfFalse, r_g, 0));
                self.release_to(mark);
            }
            let r_body = self.compile_node(arena, NodeId(an.rhs));
            self.emit(Opcode::Move, dst, r_body, 0);
            self.release_to(mark);
            end_jumps.push(self.emit_jump(0));
            for f in fails {
                self.patch_to_here(f);
            }
            self.var_slots = outer_slots;
        }
        let ctx = self.loops.pop();
        for j in end_jumps.into_iter().chain(ctx.into_iter().flat_map(|c| c.break_jumps)) {
            self.patch_to_here(j);
        }
        dst
    }

    /// Emits the test for `pat` against `r_subject`. Each emitted branch that
    /// fires on a mismatch is pushed to `fails`; names the pattern binds are
    /// added to `var_slots`.
    pub(crate) fn compile_pattern(
        &mut self,
        arena: &AstArena,
        pat: NodeId,
        r_subject: u8,
        subject_type: &Option<String>,
        fails: &mut Vec<usize>,
    ) {
        if pat.is_none() {
            self.error("`case` without a pattern".to_string());
            return;
        }
        let pn = *arena.node(pat);
        let mark = self.next_reg;
        match pn.node_kind() {
            NodeKind::PatWildcard => {}
            NodeKind::PatBind => {
                if pn.flags != 0 {
                    self.error("rest patterns are not lowered by the seed yet".to_string());
                    return;
                }
                let slot = self.alloc_local_reg();
                self.emit(Opcode::Move, slot, r_subject, 0);
                self.var_slots.insert(Self::name(arena, pn.lhs).to_string(), slot);
            }
            NodeKind::PatLiteral => {
                let r_v = self.compile_with_hint(arena, NodeId(pn.lhs), subject_type.clone());
                self.emit_test(Opcode::CmpEq, r_subject, r_v, fails);
            }
            NodeKind::PatVariant => {
                let name = Self::name(arena, pn.lhs).to_string();
                if let Some(full) = self.payload_variant(&name, subject_type.as_deref()) {
                    self.compile_payload_pattern(arena, pat, &full, r_subject, fails);
                    self.release_to(mark);
                    return;
                }
                if pn.flags != 0 {
                    self.error(format!("`{name}` is not an enum variant that carries values"));
                    return;
                }
                let Some(v) = self.resolve_variant(&name, subject_type) else {
                    self.error(format!("no enum variant named `{name}`"));
                    return;
                };
                let r_v = self.emit_const(v);
                self.emit_test(Opcode::CmpEq, r_subject, r_v, fails);
            }
            NodeKind::PatRange => {
                let r_lo = self.compile_pattern_bound(arena, NodeId(pn.lhs), subject_type);
                self.emit_test(Opcode::CmpGe, r_subject, r_lo, fails);
                let r_hi = self.compile_pattern_bound(arena, NodeId(pn.rhs), subject_type);
                let upper = if pn.flags == 1 { Opcode::CmpLe } else { Opcode::CmpLt };
                self.emit_test(upper, r_subject, r_hi, fails);
            }
            NodeKind::PatOr => {
                let mut left_fails = Vec::new();
                self.compile_pattern(arena, NodeId(pn.lhs), r_subject, subject_type, &mut left_fails);
                let matched = self.emit_jump(0);
                for f in left_fails {
                    self.patch_to_here(f);
                }
                self.compile_pattern(arena, NodeId(pn.rhs), r_subject, subject_type, fails);
                self.patch_to_here(matched);
            }
            NodeKind::PatTuple => {
                let elems = arena.children_slice(pn.lhs, pn.rhs).to_vec();
                let r_len = self.emit_intrinsic("list_len", &[r_subject]);
                let r_n = self.emit_const(PilotVal::from_int(elems.len() as i64));
                self.emit_test(Opcode::CmpEq, r_len, r_n, fails);
                for (i, &e) in elems.iter().enumerate() {
                    let r_i = self.emit_const(PilotVal::from_int(i as i64));
                    let r_e = self.emit_intrinsic("map_get", &[r_subject, r_i]);
                    self.compile_pattern(arena, NodeId(e), r_e, &None, fails);
                }
            }
            other => {
                self.error(format!("`{}` patterns are not lowered by the seed yet", other.name()));
            }
        }
        self.release_to(mark);
    }

    /// A range bound is a literal pattern; lowers the literal itself.
    fn compile_pattern_bound(&mut self, arena: &AstArena, id: NodeId, hint: &Option<String>) -> u8 {
        if id.is_some() && arena.node(id).node_kind() == NodeKind::PatLiteral {
            return self.compile_with_hint(arena, NodeId(arena.node(id).lhs), hint.clone());
        }
        self.error("range pattern bounds must be literals".to_string())
    }

    /// Emits `r_tmp = r_a <op> r_b` and a branch to the next arm when false.
    fn emit_test(&mut self, op: Opcode, r_a: u8, r_b: u8, fails: &mut Vec<usize>) {
        let r_c = self.alloc_reg();
        self.emit(op, r_c, r_a, r_b);
        fails.push(self.emit_branch(Opcode::JumpIfFalse, r_c, 0));
    }

    /// `Enum.Variant` names its enum; a bare or `::` variant is resolved
    /// against the subject's static type, then against every enum.
    fn resolve_variant(&mut self, name: &str, subject_type: &Option<String>) -> Option<PilotVal> {
        if let Some((path, variant)) = name.rsplit_once('.') {
            let owner = path.rsplit('.').next().unwrap_or(path);
            return match self.const_maps.get(owner) {
                Some(crate::CMap::Map(m)) => match m.get(variant) {
                    Some(crate::CMap::Val(v)) => Some(*v),
                    _ => None,
                },
                _ => None,
            };
        }
        let saved = std::mem::replace(&mut self.type_hint, subject_type.clone());
        let v = self.unique_variant(name);
        self.type_hint = saved;
        v
    }

    /// `guard cond else { .. }`, `guard some x else { .. }` and
    /// `guard some x = value else { .. }`. The else block runs when the
    /// condition is false or the value is `none`; a binding stays in scope.
    pub(crate) fn compile_guard(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        let node = *arena.node(node_id);
        let mark = self.next_reg;
        let cond = NodeId(node.lhs);
        let is_some = node.flags & 1 != 0;
        let pass = if is_some && cond.is_some() && arena.node(cond).node_kind() == NodeKind::AssignStmt {
            let cn = *arena.node(cond);
            let target = arena.node(NodeId(cn.lhs));
            if target.node_kind() != NodeKind::IdentExpr {
                return self.error("`guard some` must bind a plain name".to_string());
            }
            let r_v = self.compile_node(arena, NodeId(cn.rhs));
            let slot = self.alloc_local_reg();
            self.emit(Opcode::Move, slot, r_v, 0);
            self.var_slots.insert(Self::name(arena, target.lhs).to_string(), slot);
            self.emit_branch(Opcode::JumpIfNotNull, slot, 0)
        } else {
            let r_c = self.compile_node(arena, cond);
            let op = if is_some { Opcode::JumpIfNotNull } else { Opcode::JumpIfTrue };
            self.emit_branch(op, r_c, 0)
        };
        self.release_to(mark);
        self.compile_node(arena, NodeId(node.rhs));
        self.release_to(mark);
        self.patch_to_here(pass);
        self.emit_null()
    }
}
