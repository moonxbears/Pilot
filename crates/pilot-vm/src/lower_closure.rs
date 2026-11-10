//! Function values: lambdas with captured variables, named functions used as
//! values, and calls through a function value.
//!
//! A lambda body is compiled out of line as a function whose parameters are
//! the lambda's parameters followed by one hidden parameter, its environment.
//! The environment is the function value itself: entry point, arity, then the
//! captured values, copied when the lambda expression is evaluated. Captures
//! are by value, so assigning to a captured variable inside the body is a
//! compile error rather than a write the enclosing scope would never see.

use crate::instruction::{Instruction, Opcode};
use crate::{BytecodeCompiler, CaptureCtx};
use pilot_ast::{AstArena, NodeId};
use pilot_rt::PilotVal;

/// Closure calls pass the environment in the register after the arguments.
const MAX_CLOSURE_ARGS: usize = crate::MAX_CALL_ARGS - 1;
/// `closure_new` takes the entry point and arity before the captured values.
const MAX_CAPTURES: usize = crate::MAX_CALL_ARGS - 2;

impl BytecodeCompiler {
    pub(crate) fn compile_lambda(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        let node = *arena.node(node_id);
        let params: Vec<String> = arena
            .children_slice(node.lhs, node.flags as u32)
            .iter()
            .map(|&p| Self::name(arena, arena.node(NodeId(p)).lhs).to_string())
            .collect();
        if params.len() > MAX_CLOSURE_ARGS {
            return self.error(format!("a lambda takes at most {MAX_CLOSURE_ARGS} parameters"));
        }
        let dst = self.alloc_reg();
        let skip = self.emit_jump(0);
        let entry = self.here();

        let outer_slots = self.var_slots.clone();
        let saved = self.save_fn_state();
        self.reset_fn_state();
        self.captures = saved.captures.clone();
        // A lambda inside a method still sees the method's type for bare calls.
        self.cur_owner = saved.cur_owner.clone();
        for (i, p) in params.iter().enumerate() {
            let slot = self.alloc_local_reg();
            self.emit(Opcode::Move, slot, (i + 1) as u8, 0);
            self.var_slots.insert(p.clone(), slot);
        }
        let env_slot = self.alloc_local_reg();
        self.emit(Opcode::Move, env_slot, (params.len() + 1) as u8, 0);
        self.captures.push(CaptureCtx {
            outer_slots,
            names: Vec::new(),
            env_slot,
        });
        let r_body = self.compile_node(arena, NodeId(node.rhs));
        self.emit(Opcode::Move, 0, r_body, 0);
        self.emit(Opcode::Ret, 0, 0, 0);
        let captured = self.captures.pop().map(|c| c.names).unwrap_or_default();
        self.restore_fn_state(saved);
        self.patch_to_here(skip);

        if captured.len() > MAX_CAPTURES {
            return self.error(format!("a lambda captures at most {MAX_CAPTURES} variables"));
        }
        let mark = self.next_reg;
        let mut regs = vec![
            self.emit_const(PilotVal::from_int(entry as i64)),
            self.emit_const(PilotVal::from_int(params.len() as i64)),
        ];
        for name in &captured {
            let r = match self.var_slots.get(name).copied() {
                Some(r) => r,
                None => self
                    .capture_read(name)
                    .unwrap_or_else(|| self.error(format!("undefined name `{name}`"))),
            };
            regs.push(r);
        }
        let r = self.emit_intrinsic("closure_new", &regs);
        self.emit(Opcode::Move, dst, r, 0);
        self.release_to(mark);
        dst
    }

    /// Whether `name` is a variable of the scope enclosing lambda `depth`
    /// (directly, or through that scope's own captures).
    fn capturable(&self, depth: usize, name: &str) -> bool {
        self.captures[depth].outer_slots.contains_key(name)
            || (depth > 0 && self.capturable(depth - 1, name))
    }

    pub(crate) fn is_captured_name(&self, name: &str) -> bool {
        self.captures
            .len()
            .checked_sub(1)
            .is_some_and(|d| self.capturable(d, name))
    }

    /// Reads a captured variable inside the innermost lambda, registering the
    /// capture on first use. `None` when `name` is not an enclosing variable.
    pub(crate) fn capture_read(&mut self, name: &str) -> Option<u8> {
        let depth = self.captures.len().checked_sub(1)?;
        if !self.capturable(depth, name) {
            return None;
        }
        let ctx = &mut self.captures[depth];
        let index = match ctx.names.iter().position(|n| n == name) {
            Some(i) => i,
            None => {
                ctx.names.push(name.to_string());
                ctx.names.len() - 1
            }
        };
        let env = ctx.env_slot;
        let r_i = self.emit_const(PilotVal::from_int(index as i64));
        Some(self.emit_intrinsic("closure_env", &[env, r_i]))
    }

    /// A named function used as a value. `None` when `name` is not a function.
    pub(crate) fn function_value(&mut self, name: &str) -> Option<u8> {
        let arity = self
            .known_fns
            .get(name)
            .copied()
            .or_else(|| self.fn_entries.get(name).map(|(_, params)| params.len()))?;
        if arity > MAX_CLOSURE_ARGS {
            return Some(self.error(format!(
                "`{name}` takes {arity} parameters; a function value takes at most {MAX_CLOSURE_ARGS}"
            )));
        }
        let r_entry = self.alloc_reg();
        let idx = self.push_ins(Instruction::encode_wide(Opcode::LoadConst, r_entry, 0));
        self.entry_patches.push((idx, name.to_string(), self.cur_span));
        let r_arity = self.emit_const(PilotVal::from_int(arity as i64));
        Some(self.emit_intrinsic("closure_new", &[r_entry, r_arity]))
    }

    /// Calls the function value in `r_callee` with `args`.
    pub(crate) fn emit_closure_call(&mut self, r_callee: u8, args: &[u8]) -> u8 {
        if args.len() > MAX_CLOSURE_ARGS {
            return self.error(format!(
                "a call through a function value takes at most {MAX_CLOSURE_ARGS} arguments"
            ));
        }
        self.stage_args(args);
        let dst = self.alloc_reg();
        self.emit(Opcode::CallClosure, dst, r_callee, args.len() as u8);
        dst
    }
}
