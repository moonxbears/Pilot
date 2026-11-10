//! Values-first failure handling: postfix `?`, `try { .. } catch err { .. }`
//! and `expr catch err => fallback`.
//!
//! A failure is a value, never an unwinding exception. Postfix `expr?` tests
//! its operand: a success value is unwrapped, a failure leaves. Outside any
//! `try`, it returns the failure from the current function. Inside a `try`
//! block, it jumps to the `catch` clause with the failure bound to the
//! clause's name, running the deferred statements of every block it leaves.
//! `expr catch err => fallback` is the expression form: the fallback is the
//! value when `expr` is itself a failure or a `?` inside it fails.
//!
//! VM faults (division by zero, a failed `assert`) are not failures: they stop
//! the program and are never caught.

use crate::instruction::Opcode;
use crate::{BytecodeCompiler, TryCtx};
use pilot_ast::{AstArena, NodeId, NodeKind};

impl BytecodeCompiler {
    /// Postfix `expr?`.
    pub(crate) fn compile_try_propagate(&mut self, arena: &AstArena, inner: u32) -> u8 {
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r = self.compile_node(arena, NodeId(inner));
        self.emit(Opcode::Move, dst, r, 0);
        self.release_to(mark);
        let present = self.emit_branch(Opcode::JumpIfNotNull, dst, 0);
        self.emit_failure_exit(arena, dst);
        self.patch_to_here(present);
        self.emit_unwrap(dst);
        self.release_to(mark);
        dst
    }

    /// Leaves with the failure in `r_fail`: to the innermost `try`'s catch
    /// clause, or out of the function.
    fn emit_failure_exit(&mut self, arena: &AstArena, r_fail: u8) {
        let Some(depth) = self.tries.last().map(|t| t.defer_depth) else {
            self.emit_defers_above(arena, 0);
            self.emit(Opcode::Move, 0, r_fail, 0);
            self.emit(Opcode::Ret, 0, 0, 0);
            return;
        };
        let fail_reg = self.tries.last().map_or(0, |t| t.fail_reg);
        self.emit(Opcode::Move, fail_reg, r_fail, 0);
        self.emit_defers_above(arena, depth);
        let jump = self.emit_jump(0);
        if let Some(t) = self.tries.last_mut() {
            t.fail_jumps.push(jump);
        }
    }

    /// Replaces the success value in `r` with what it carries.
    fn emit_unwrap(&mut self, r: u8) {
        let r_v = self.emit_intrinsic("try_unwrap", &[r]);
        self.emit(Opcode::Move, r, r_v, 0);
    }

    /// Opens a `try` region; failures inside it go to the returned register.
    fn open_try(&mut self) -> u8 {
        let fail_reg = self.alloc_local_reg();
        self.tries.push(TryCtx {
            fail_reg,
            defer_depth: self.defers.len(),
            fail_jumps: Vec::new(),
        });
        fail_reg
    }

    /// Closes the innermost `try` region; returns its jumps to the catch clause.
    fn close_try(&mut self) -> Vec<usize> {
        self.tries.pop().map(|t| t.fail_jumps).unwrap_or_default()
    }

    /// Compiles a `catch` clause's body with its name bound to `fail_reg`.
    fn compile_catch_clause(&mut self, arena: &AstArena, clause: NodeId, fail_reg: u8) -> u8 {
        let cn = *arena.node(clause);
        let outer_slots = self.var_slots.clone();
        if cn.lhs != 0 {
            self.var_slots.insert(Self::name(arena, cn.lhs).to_string(), fail_reg);
        }
        let r = self.compile_node(arena, NodeId(cn.rhs));
        self.var_slots = outer_slots;
        r
    }

    /// `try { body } catch err { handler }`.
    pub(crate) fn compile_try_stmt(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        let node = *arena.node(node_id);
        let children = arena.children_slice(node.lhs, node.rhs).to_vec();
        let Some((&body, clauses)) = children.split_first() else {
            return self.emit_null();
        };
        let clause = match clauses {
            [one] => NodeId(*one),
            [] => return self.error("`try` needs a `catch` clause".to_string()),
            _ => {
                return self.error(
                    "a `try` takes one `catch` clause; the seed cannot tell failure types apart yet"
                        .to_string(),
                )
            }
        };
        let mark = self.next_reg;
        let fail_reg = self.open_try();
        self.compile_node(arena, NodeId(body));
        self.release_to(mark);
        let fails = self.close_try();
        let end = self.emit_jump(0);
        for j in fails {
            self.patch_to_here(j);
        }
        self.compile_catch_clause(arena, clause, fail_reg);
        self.release_to(mark);
        self.patch_to_here(end);
        self.emit_null()
    }

    /// `expr catch err => fallback`.
    pub(crate) fn compile_catch_recovery(&mut self, arena: &AstArena, expr: u32, clause: u32) -> u8 {
        let clause = NodeId(clause);
        if clause.is_none() || arena.node(clause).node_kind() != NodeKind::CatchClause {
            return self.error("`catch` needs a fallback".to_string());
        }
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let fail_reg = self.open_try();
        let r = self.compile_node(arena, NodeId(expr));
        self.emit(Opcode::Move, dst, r, 0);
        self.release_to(mark);
        let fails = self.close_try();
        let present = self.emit_branch(Opcode::JumpIfNotNull, dst, 0);
        self.emit(Opcode::Move, fail_reg, dst, 0);
        for j in fails {
            self.patch_to_here(j);
        }
        let r_fb = self.compile_catch_clause(arena, clause, fail_reg);
        self.emit(Opcode::Move, dst, r_fb, 0);
        self.release_to(mark);
        let end = self.emit_jump(0);
        self.patch_to_here(present);
        self.emit_unwrap(dst);
        self.release_to(mark);
        self.patch_to_here(end);
        dst
    }
}
