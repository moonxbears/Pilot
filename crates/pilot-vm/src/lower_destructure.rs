//! Record and slice patterns, and destructuring `let`.
//!
//! - `{ "key": p, field: p, name }` matches a record or map that has every
//!   listed key and matches each value against its sub-pattern; a bare `name`
//!   binds the field of that name. `Name { .. }` also requires the record to
//!   be built as struct (or payload variant) `Name`.
//! - `[a, b]` matches a list of exactly that length; `[a, ..rest, z]` (or
//!   `*rest`) matches a list at least as long as the other elements and binds
//!   the middle to `rest` as a new list.
//! - `let (a, b) = v;`, `let [h, ..t] = v;` and `let { "k": x } = v;` bind
//!   through a pattern; a value the pattern does not match is a runtime error.

use crate::instruction::Opcode;
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;

/// `PatRecord.flags` when the first child names the type.
const RECORD_PATTERN_TYPED: u16 = 1;
/// `PatBind.flags` of a rest element (`..rest`, `*rest`) in a slice pattern.
const PATTERN_REST: u16 = 2;
/// `LetStmt.flags` when `lhs` is a pattern rather than a name.
const LET_PATTERN: u16 = 1;

impl BytecodeCompiler {
    /// Whether `let`/`var` `decl` binds through a pattern.
    pub(crate) fn is_destructuring_let(arena: &AstArena, decl: NodeId) -> bool {
        arena.node(decl).flags == LET_PATTERN
    }

    /// `let <pattern> = value;`.
    pub(crate) fn compile_destructuring_let(&mut self, arena: &AstArena, decl: NodeId) -> u8 {
        let dn = *arena.node(decl);
        if NodeId(dn.rhs).is_none() {
            return self.error("a destructuring `let` needs a value".to_string());
        }
        let r = self.compile_node(arena, NodeId(dn.rhs));
        let subject = self.alloc_local_reg();
        self.emit(Opcode::Move, subject, r, 0);
        let mut fails = Vec::new();
        self.compile_pattern(arena, NodeId(dn.lhs), subject, &None, &mut fails);
        if !fails.is_empty() {
            let done = self.emit_jump(0);
            for f in fails {
                self.patch_to_here(f);
            }
            self.emit_trap("the value does not match the destructuring pattern".to_string());
            self.patch_to_here(done);
        }
        self.emit_null()
    }

    /// The names pattern `pat` binds, in source order.
    pub(crate) fn pattern_names(arena: &AstArena, pat: NodeId, out: &mut Vec<String>) {
        if pat.is_none() {
            return;
        }
        let pn = *arena.node(pat);
        match pn.node_kind() {
            NodeKind::PatBind => out.push(Self::name(arena, pn.lhs).to_string()),
            NodeKind::PatOr => Self::pattern_names(arena, NodeId(pn.lhs), out),
            NodeKind::CallArg => Self::pattern_names(arena, NodeId(pn.rhs), out),
            NodeKind::PatTuple | NodeKind::PatSlice | NodeKind::PatRecord => {
                for &c in arena.children_slice(pn.lhs, pn.rhs) {
                    Self::pattern_names(arena, NodeId(c), out);
                }
            }
            _ => {}
        }
    }

    /// A top-level destructuring `let`: binds through the pattern, then
    /// stores every bound name in its global.
    pub(crate) fn compile_global_destructuring_let(
        &mut self,
        arena: &AstArena,
        decl: NodeId,
    ) -> u8 {
        let mark = self.next_reg;
        self.compile_destructuring_let(arena, decl);
        let mut names = Vec::new();
        Self::pattern_names(arena, NodeId(arena.node(decl).lhs), &mut names);
        for name in names {
            if let Some(slot) = self.var_slots.remove(&name) {
                self.emit_global_set(&name, slot);
            }
        }
        self.release_to(mark);
        self.emit_null()
    }

    /// Tests `r_subject` against record pattern `pat`.
    pub(crate) fn compile_record_pattern(
        &mut self,
        arena: &AstArena,
        pat: NodeId,
        r_subject: u8,
        fails: &mut Vec<usize>,
    ) {
        let pn = *arena.node(pat);
        let children = arena.children_slice(pn.lhs, pn.rhs).to_vec();
        let fields = if pn.flags == RECORD_PATTERN_TYPED {
            let ty = Self::name(arena, arena.node(NodeId(children[0])).lhs).to_string();
            let tag = self.payload_variant(&ty, None).unwrap_or(ty);
            let r_type = self.emit_intrinsic("record_type", &[r_subject]);
            let r_tag = self.emit_const(PilotVal::from_str(&tag));
            self.emit_pattern_test(Opcode::CmpEq, r_type, r_tag, fails);
            &children[1..]
        } else {
            &children[..]
        };
        for &f in fields {
            let fnode = *arena.node(NodeId(f));
            let (key, sub) = match fnode.node_kind() {
                NodeKind::CallArg => {
                    (Self::name(arena, fnode.lhs).to_string(), NodeId(fnode.rhs))
                }
                _ => (Self::name(arena, fnode.lhs).to_string(), NodeId(f)),
            };
            let r_k = self.emit_const(PilotVal::from_str(&key));
            let r_has = self.emit_intrinsic("map_has", &[r_subject, r_k]);
            fails.push(self.emit_branch(Opcode::JumpIfFalse, r_has, 0));
            let r_v = self.emit_intrinsic("map_get", &[r_subject, r_k]);
            let slot = self.alloc_local_reg();
            self.emit(Opcode::Move, slot, r_v, 0);
            self.compile_pattern(arena, sub, slot, &None, fails);
        }
    }

    /// Tests `r_subject` against slice pattern `pat`.
    pub(crate) fn compile_slice_pattern(
        &mut self,
        arena: &AstArena,
        pat: NodeId,
        r_subject: u8,
        fails: &mut Vec<usize>,
    ) {
        let pn = *arena.node(pat);
        let elems: Vec<NodeId> = arena
            .children_slice(pn.lhs, pn.rhs)
            .iter()
            .map(|&c| NodeId(c))
            .collect();
        let is_rest = |e: &NodeId| {
            let n = arena.node(*e);
            n.node_kind() == NodeKind::PatBind && n.flags == PATTERN_REST
        };
        let rests: Vec<usize> = elems
            .iter()
            .enumerate()
            .filter(|(_, e)| is_rest(e))
            .map(|(i, _)| i)
            .collect();
        if rests.len() > 1 {
            self.error("a slice pattern can have only one rest element".to_string());
            return;
        }
        let fixed = elems.len() - rests.len();
        let r_len = self.alloc_local_reg();
        let r = self.emit_intrinsic("list_len", &[r_subject]);
        self.emit(Opcode::Move, r_len, r, 0);
        let r_n = self.emit_const(PilotVal::from_int(fixed as i64));
        let op = if rests.is_empty() { Opcode::CmpEq } else { Opcode::CmpGe };
        self.emit_pattern_test(op, r_len, r_n, fails);
        let rest_at = rests.first().copied();
        for (i, &e) in elems.iter().enumerate() {
            let r_e = match rest_at {
                Some(r) if i == r => {
                    // The middle: from `r` up to `len - (elements after it)`.
                    let r_from = self.emit_const(PilotVal::from_int(r as i64));
                    let r_after = self.emit_const(PilotVal::from_int((elems.len() - 1 - r) as i64));
                    let r_to = self.alloc_reg();
                    self.emit(Opcode::Sub, r_to, r_len, r_after);
                    let r_s = self.emit_intrinsic("str_slice", &[r_subject, r_from, r_to]);
                    let name = Self::name(arena, arena.node(e).lhs).to_string();
                    let slot = self.alloc_local_reg();
                    self.emit(Opcode::Move, slot, r_s, 0);
                    self.var_slots.insert(name.clone(), slot);
                    self.var_types.remove(&name);
                    continue;
                }
                Some(r) if i > r => {
                    let r_back = self.emit_const(PilotVal::from_int((elems.len() - i) as i64));
                    let r_i = self.alloc_reg();
                    self.emit(Opcode::Sub, r_i, r_len, r_back);
                    self.emit_intrinsic("map_get", &[r_subject, r_i])
                }
                _ => {
                    let r_i = self.emit_const(PilotVal::from_int(i as i64));
                    self.emit_intrinsic("map_get", &[r_subject, r_i])
                }
            };
            let slot = self.alloc_local_reg();
            self.emit(Opcode::Move, slot, r_e, 0);
            self.compile_pattern(arena, e, slot, &None, fails);
        }
    }

    /// Emits `r_a <op> r_b` and a branch to the failure list when false.
    fn emit_pattern_test(&mut self, op: Opcode, r_a: u8, r_b: u8, fails: &mut Vec<usize>) {
        let r_c = self.alloc_reg();
        self.emit(op, r_c, r_a, r_b);
        fails.push(self.emit_branch(Opcode::JumpIfFalse, r_c, 0));
    }
}
