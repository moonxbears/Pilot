//! Method calls: `recv.method(args)`, and bare `method(args)` inside a
//! method of the same type.
//!
//! A method is reached through its type, never by its bare name. When the
//! receiver's type is known at compile time, the call goes straight to
//! `Type.method`. Otherwise the call tests the struct the receiver record was
//! built as against every struct declaring `method` and calls the match. A
//! receiver that matches none (a list, a string, a record of another struct)
//! goes to the one non-struct type (`ext` block) declaring `method`, else the
//! seed's builtin method of that name, else a free function
//! `method(recv, args)`; with none of these the call stops the program with
//! an error naming the method.

use crate::instruction::Opcode;
use crate::lower_native::native;
use crate::BytecodeCompiler;
use pilot_parser::NativeKind;
use pilot_ast::{AstArena, NodeId};
use pilot_rt::PilotVal;

impl BytecodeCompiler {
    /// Whether `owner` declares `method`; `Some(is_static)` when it does.
    pub(crate) fn declares(&self, owner: &str, method: &str) -> Option<bool> {
        self.method_owners
            .get(method)?
            .iter()
            .find(|(o, _)| o == owner)
            .map(|&(_, is_static)| is_static)
    }

    /// Calls `method` on the value in `r_recv`, whose static type (or, without
    /// one, its source name) is `owner`.
    pub(crate) fn emit_method_call(
        &mut self,
        arena: &AstArena,
        r_recv: u8,
        owner: &str,
        method: &str,
        args: &[NodeId],
    ) -> u8 {
        let full = format!("{owner}.{method}");
        let mut regs = vec![r_recv];
        regs.extend(self.compile_args(arena, args));
        if self.declares(owner, method) == Some(false) || self.fn_entries.contains_key(&full) {
            return self.emit_user_call(full, method.to_string(), &regs);
        }
        let instance_owners: Vec<String> = self
            .method_owners
            .get(method)
            .map(|v| v.iter().filter(|(_, st)| !st).map(|(o, _)| o.clone()).collect())
            .unwrap_or_default();
        if instance_owners.is_empty() {
            if let Some(hook) = native(NativeKind::Method, method) {
                return self.emit_intrinsic(hook, &regs);
            }
            return self.emit_user_call(full, method.to_string(), &regs);
        }
        let (mut records, others): (Vec<String>, Vec<String>) =
            instance_owners.into_iter().partition(|o| self.structs.contains_key(o));
        if others.len() > 1 {
            return self.error(format!(
                "`{method}` is declared by {}; give the receiver a type so the call can choose",
                others.join(", ")
            ));
        }
        records.sort();
        self.emit_dynamic_dispatch(method, &records, others.first(), &regs)
    }

    /// Tests the receiver's struct against each of `records`, in order, then
    /// falls back as the module documentation describes.
    fn emit_dynamic_dispatch(&mut self, method: &str, records: &[String], other: Option<&String>, regs: &[u8]) -> u8 {
        let dst = self.alloc_reg();
        let r_type = self.emit_intrinsic("record_type", &regs[..1]);
        let mark = self.next_reg;
        let mut end_jumps = Vec::new();
        for owner in records {
            let r_name = self.emit_const(PilotVal::from_str(owner));
            let r_is = self.alloc_reg();
            self.emit(Opcode::CmpEq, r_is, r_type, r_name);
            let next = self.emit_branch(Opcode::JumpIfFalse, r_is, 0);
            let full = format!("{owner}.{method}");
            let r = self.emit_user_call(full, method.to_string(), regs);
            self.emit(Opcode::Move, dst, r, 0);
            end_jumps.push(self.emit_jump(0));
            self.patch_to_here(next);
            self.release_to(mark);
        }
        let r = if let Some(o) = other {
            self.emit_user_call(format!("{o}.{method}"), method.to_string(), regs)
        } else if let Some(hook) = native(NativeKind::Method, method) {
            self.emit_intrinsic(hook, regs)
        } else if self.known_fns.contains_key(method) {
            self.emit_user_call(method.to_string(), method.to_string(), regs)
        } else {
            self.emit_trap(format!("no method `{method}` for this value"))
        };
        self.emit(Opcode::Move, dst, r, 0);
        for j in end_jumps {
            self.patch_to_here(j);
        }
        self.release_to(mark);
        dst
    }

    /// A bare call `method(args)` inside a method of a type that declares
    /// `method` calls it on `self` (or, for a static method, on the type).
    /// `None` when the name is not such a method.
    pub(crate) fn compile_implicit_self_call(&mut self, arena: &AstArena, name: &str, args: &[NodeId]) -> Option<u8> {
        let owner = self.cur_owner.clone()?;
        let is_static = self.declares(&owner, name)?;
        let full = format!("{owner}.{name}");
        if is_static {
            let regs = self.compile_args(arena, args);
            return Some(self.emit_user_call(full, name.to_string(), &regs));
        }
        let receiver = crate::RECEIVER;
        let r_recv = match self.var_slots.get(receiver).copied() {
            Some(r) => r,
            None => self.capture_read(receiver)?,
        };
        Some(self.emit_method_call(arena, r_recv, &owner, name, args))
    }
}
