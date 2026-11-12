//! Values-first failure handling: postfix `?`, `try { .. } catch err { .. }`
//! and `expr catch err => fallback`.
//!
//! A failure is a value, never an unwinding exception. Postfix `expr?` tests
//! its operand: `none` is a failure, and so is a struct record whose success
//! predicate (the try protocol `stdlib/core/syntax/try_catch.pilot` binds)
//! answers `false`. A successful record is unwrapped by its `op ?`; any other
//! value passes through unchanged. A failure leaves. Outside any
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
    /// A failure that leaves the function first drops the owned values the
    /// drop plan lists for it.
    pub(crate) fn compile_try_propagate(&mut self, arena: &AstArena, inner: u32) -> u8 {
        let exit_drops = std::mem::take(&mut self.pending_exit_drops);
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r = self.compile_node(arena, NodeId(inner));
        self.emit(Opcode::Move, dst, r, 0);
        self.release_to(mark);
        let absent = self.emit_branch(Opcode::JumpIfNull, dst, 0);
        let failed = self.emit_unwrap(arena, dst);
        let done = self.emit_jump(0);
        self.patch_to_here(absent);
        if let Some(j) = failed {
            self.patch_to_here(j);
        }
        self.emit_drops(arena, &exit_drops);
        self.emit_failure_exit(arena, dst);
        self.patch_to_here(done);
        self.release_to(mark);
        dst
    }

    /// Leaves with the failure in `r_fail`: to the innermost `try`'s catch
    /// clause, or out of the function.
    fn emit_failure_exit(&mut self, arena: &AstArena, r_fail: u8) {
        let failure = crate::lower_defer::DeferExit::Failure;
        let Some(depth) = self.tries.last().map(|t| t.defer_depth) else {
            self.emit_defers_on(arena, 0, failure);
            self.emit(Opcode::Move, 0, r_fail, 0);
            self.emit(Opcode::Ret, 0, 0, 0);
            return;
        };
        let fail_reg = self.tries.last().map_or(0, |t| t.fail_reg);
        self.emit(Opcode::Move, fail_reg, r_fail, 0);
        self.emit_defers_on(arena, depth, failure);
        let jump = self.emit_jump(0);
        if let Some(t) = self.tries.last_mut() {
            t.fail_jumps.push(jump);
        }
    }

    /// When `r` holds a successful struct record, replaces it with its
    /// success value. Returns the branch taken, with `r` unchanged, when it
    /// holds a failed one. Nothing is emitted while no struct implements the
    /// try protocol.
    fn emit_unwrap(&mut self, arena: &AstArena, r: u8) -> Option<usize> {
        let p = pilot_parser::bootstrap_tables().try_protocol()?.clone();
        if !self.method_owners.contains_key(&p.is_success) {
            return None;
        }
        let mark = self.next_reg;
        let r_type = self.emit_intrinsic("record_type", &[r]);
        let plain = self.emit_branch(Opcode::JumpIfNull, r_type, 0);
        let r_ok = self.emit_method_call(arena, r, "", &p.is_success, &[]);
        let failed = self.emit_branch(Opcode::JumpIfFalse, r_ok, 0);
        let r_v = self.emit_method_call(arena, r, "", &p.unwrap, &[]);
        self.emit(Opcode::Move, r, r_v, 0);
        self.patch_to_here(plain);
        self.release_to(mark);
        Some(failed)
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

    /// Compiles a `catch` clause's body with its name bound to the failure in
    /// `fail_reg`: a failed record's error (the try protocol's `failure`), or
    /// the failure itself when it is not a record.
    fn compile_catch_clause(&mut self, arena: &AstArena, clause: NodeId, fail_reg: u8) -> u8 {
        let cn = *arena.node(clause);
        let failure = pilot_parser::bootstrap_tables()
            .try_protocol()
            .map(|p| p.failure.clone())
            .filter(|f| !f.is_empty() && self.method_owners.contains_key(f));
        if let (true, Some(f)) = (cn.lhs != 0, failure) {
            let mark = self.next_reg;
            let r_type = self.emit_intrinsic("record_type", &[fail_reg]);
            let plain = self.emit_branch(Opcode::JumpIfNull, r_type, 0);
            let r_e = self.emit_method_call(arena, fail_reg, "", &f, &[]);
            self.emit(Opcode::Move, fail_reg, r_e, 0);
            self.patch_to_here(plain);
            self.release_to(mark);
        }
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
            [] => return self.error(
                crate::codes::SYNTAX,
                "`try` needs a `catch` clause".to_string(),
            ),
            _ => {
                return self.error(crate::codes::SEED, 
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
            return self.error(crate::codes::SYNTAX, "`catch` needs a fallback".to_string());
        }
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let fail_reg = self.open_try();
        let r = self.compile_node(arena, NodeId(expr));
        self.emit(Opcode::Move, dst, r, 0);
        self.release_to(mark);
        let fails = self.close_try();
        let absent = self.emit_branch(Opcode::JumpIfNull, dst, 0);
        let failed = self.emit_unwrap(arena, dst);
        let end = self.emit_jump(0);
        self.patch_to_here(absent);
        if let Some(j) = failed {
            self.patch_to_here(j);
        }
        self.emit(Opcode::Move, fail_reg, dst, 0);
        for j in fails {
            self.patch_to_here(j);
        }
        let r_fb = self.compile_catch_clause(arena, clause, fail_reg);
        self.emit(Opcode::Move, dst, r_fb, 0);
        self.release_to(mark);
        self.patch_to_here(end);
        dst
    }
}
