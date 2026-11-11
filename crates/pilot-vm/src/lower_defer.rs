//! `defer`: statements that run when their enclosing block exits.
//!
//! Each block opens a defer scope. `defer stmt` records `stmt` in the
//! innermost scope and emits nothing. Every way out of a block runs the
//! scopes it leaves, innermost first and each scope last-in first-out:
//! falling off the end, `return` (after the return value is computed),
//! and `break` / `continue` out of the enclosing loop or switch. The
//! deferred code is emitted inline at each exit.

use crate::instruction::Opcode;
use crate::lower_drop::Deferred;
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId};

impl BytecodeCompiler {
    pub(crate) fn compile_defer(&mut self, body: u32) -> u8 {
        match self.defers.last_mut() {
            Some(scope) => {
                scope.push(Deferred::Stmt(NodeId(body)));
                self.emit_null()
            }
            None => self.error(crate::codes::SYNTAX, "`defer` must be inside a block".to_string()),
        }
    }

    /// Emits the deferred statements of every scope above `depth`. While a
    /// scope's statements are emitted it is closed, so a deferred statement
    /// never re-runs its own scope.
    pub(crate) fn emit_defers_above(&mut self, arena: &AstArena, depth: usize) {
        if self.defers.iter().skip(depth).all(|s| s.is_empty()) {
            return;
        }
        let open = self.defers.clone();
        while self.defers.len() > depth {
            let scope = self.defers.pop().unwrap_or_default();
            for item in scope.iter().rev() {
                let mark = self.next_reg;
                match item {
                    Deferred::Stmt(stmt) => {
                        self.compile_node(arena, *stmt);
                    }
                    Deferred::Drop(d) => self.emit_drops(arena, std::slice::from_ref(d)),
                    Deferred::Probe(p) => self.emit_probe_call(arena, NodeId::NONE, p, None),
                }
                self.next_reg = mark;
            }
        }
        self.defers = open;
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
}
