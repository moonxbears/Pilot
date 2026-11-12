//! `defer` and `errdefer`: statements that run when their enclosing block
//! exits, and `become`, which leaves through a tail call.
//!
//! Each block opens a defer scope. `defer stmt` records `stmt` in the
//! innermost scope and emits nothing. Every way out of a block runs the
//! scopes it leaves, innermost first and each scope last-in first-out:
//! falling off the end, `return` (after the return value is computed),
//! and `break` / `continue` out of the enclosing loop or switch. The
//! deferred code is emitted inline at each exit.
//!
//! `errdefer stmt` is recorded the same way but runs only on the exits a
//! failure takes: a `?` that fails, and a `return` whose value `?` would
//! propagate (absent, or a struct record whose success predicate answers
//! `false`). Every other exit skips it.
//!
//! `become f(args);` lowers the call like any other and turns it into a tail
//! call: the callee takes over the frame and returns to the caller's caller.
//! Nothing of the caller may still have to run afterwards; the checker
//! reports a `defer` or `errdefer` above it (TAIL002).

use crate::instruction::Opcode;
use crate::lower_drop::Deferred;
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId};

/// How a block is being left.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeferExit {
    /// Falling off the end, `break`, `continue`, a `return` without a value:
    /// `errdefer`s are skipped.
    Normal,
    /// A failing `?`: `errdefer`s run.
    Failure,
    /// A `return` of the value in this register: `errdefer`s run when it is
    /// a failure.
    Returning(u8),
}

impl BytecodeCompiler {
    pub(crate) fn compile_defer(&mut self, body: u32, flags: u16) -> u8 {
        let item = if flags == pilot_parser::DEFER_ON_FAILURE {
            Deferred::OnFailure(NodeId(body))
        } else {
            Deferred::Stmt(NodeId(body))
        };
        match self.defers.last_mut() {
            Some(scope) => {
                scope.push(item);
                self.emit_null()
            }
            None => self.error(crate::codes::SYNTAX, "`defer` must be inside a block".to_string()),
        }
    }

    /// Emits the deferred statements of every scope above `depth` for a
    /// normal exit.
    pub(crate) fn emit_defers_above(&mut self, arena: &AstArena, depth: usize) {
        self.emit_defers_on(arena, depth, DeferExit::Normal);
    }

    /// Emits the deferred statements of every scope above `depth`. While a
    /// scope's statements are emitted it is closed, so a deferred statement
    /// never re-runs its own scope.
    pub(crate) fn emit_defers_on(&mut self, arena: &AstArena, depth: usize, exit: DeferExit) {
        if self.defers.iter().skip(depth).all(|s| s.is_empty()) {
            return;
        }
        let on_failure = self
            .defers
            .iter()
            .skip(depth)
            .flatten()
            .any(|d| matches!(d, Deferred::OnFailure(_)));
        let failed = match exit {
            DeferExit::Returning(r) if on_failure => Some(self.emit_failure_flag(arena, r)),
            _ => None,
        };
        let open = self.defers.clone();
        while self.defers.len() > depth {
            let scope = self.defers.pop().unwrap_or_default();
            for item in scope.iter().rev() {
                let mark = self.next_reg;
                match item {
                    Deferred::Stmt(stmt) => {
                        self.compile_node(arena, *stmt);
                    }
                    Deferred::OnFailure(stmt) => match (exit, failed) {
                        (DeferExit::Failure, _) => {
                            self.compile_node(arena, *stmt);
                        }
                        (DeferExit::Returning(_), Some(flag)) => {
                            let skip = self.emit_branch(Opcode::JumpIfFalse, flag, 0);
                            self.compile_node(arena, *stmt);
                            self.patch_to_here(skip);
                        }
                        _ => {}
                    },
                    Deferred::Drop(d) => self.emit_drops(arena, std::slice::from_ref(d)),
                    Deferred::Probe(p) => self.emit_probe_call(arena, NodeId::NONE, p, None),
                }
                self.next_reg = mark;
            }
        }
        self.defers = open;
    }

    /// A register holding whether the value in `r` is a failure, the test
    /// postfix `?` makes: absent, or a struct record whose success
    /// predicate answers `false`.
    fn emit_failure_flag(&mut self, arena: &AstArena, r: u8) -> u8 {
        let flag = self.alloc_reg();
        let yes = self.emit_const(crate::PilotVal::from_bool(true));
        self.emit(Opcode::Move, flag, yes, 0);
        let absent = self.emit_branch(Opcode::JumpIfNull, r, 0);
        let no = self.emit_const(crate::PilotVal::from_bool(false));
        self.emit(Opcode::Move, flag, no, 0);
        let protocol = pilot_parser::bootstrap_tables().try_protocol().cloned();
        if let Some(p) = protocol.filter(|p| self.method_owners.contains_key(&p.is_success)) {
            let r_type = self.emit_intrinsic("record_type", &[r]);
            let plain = self.emit_branch(Opcode::JumpIfNull, r_type, 0);
            let r_ok = self.emit_method_call(arena, r, "", &p.is_success, &[]);
            self.emit(Opcode::Not, flag, r_ok, 0);
            self.patch_to_here(plain);
        }
        self.patch_to_here(absent);
        flag
    }

    /// Closes the innermost scope at the end of a block whose value is in
    /// `r_value`; returns the register that holds the value afterwards.
    pub(crate) fn close_defer_scope(&mut self, arena: &AstArena, r_value: u8) -> u8 {
        let has_deferred = self.defers.last().is_some_and(|s| !s.is_empty());
        let r = if has_deferred {
            // Keep the block's value out of reach of the deferred code.
            let keep = self.alloc_reg();
            self.emit(Opcode::Move, keep, r_value, 0);
            let depth = self.defers.len() - 1;
            self.emit_defers_above(arena, depth);
            keep
        } else {
            r_value
        };
        self.defers.pop();
        r
    }

    /// `become call;`: the call, made a tail call.
    pub(crate) fn compile_become(&mut self, arena: &AstArena, call: u32) -> u8 {
        if self.gen_body.is_some() {
            return self.error(crate::codes::SEED, "`become` cannot leave a generator".to_string());
        }
        // Not a call, or deferred statements still to run: the checker reports
        // it (TAIL001, TAIL002); lowered as the plain `return` it reads as.
        let deferred = self.defers.iter().flatten().any(|d| matches!(d, Deferred::Stmt(_) | Deferred::OnFailure(_)))
            || !self.contract.ensures.is_empty();
        if deferred || arena.node(NodeId(call)).node_kind() != pilot_ast::NodeKind::CallExpr {
            return self.compile_return(arena, call);
        }
        let start = self.instructions.len();
        let r = self.compile_node(arena, NodeId(call));
        // The call is the last instruction that does anything: only register
        // moves of its value may follow it, and a tail call skips them.
        let call_at = (start..self.instructions.len())
            .rev()
            .find(|&i| self.instructions[i].opcode() != Opcode::Move)
            .filter(|&i| self.instructions[i].opcode() == Opcode::Call);
        let pending = !self.pending_exit_drops.is_empty() || self.loops.iter().any(|l| l.release.is_some());
        let Some(i) = call_at.filter(|_| !pending) else {
            return self.error(
                crate::codes::SEED,
                "`become` here needs a direct call to a named function with nothing left to drop or release after it"
                    .to_string(),
            );
        };
        let ins = self.instructions[i];
        self.instructions[i] = crate::Instruction::encode_wide(Opcode::TailCall, ins.dst(), ins.wide());
        r
    }
}
