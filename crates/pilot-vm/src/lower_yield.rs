//! `yield`: generator functions and collecting `for` expressions (F5b).
//!
//! A `yield` inside a `for` in value position (`let ys = for ..`, an
//! argument, a returned value) appends its value to the list that `for`
//! evaluates to. Any other `yield` in a function or lambda body makes it a
//! generator. A `for` that is an item of a block is in statement position
//! and collects nothing.
//!
//! A generator function's entry is a `Nop` that becomes a jump to a stub
//! placed after the body once a `yield` is found. The stub asks the engine
//! for a frame holding the call's registers, to resume at the body
//! (`gen_new`, whose 32-bit operand is that address), then builds the
//! record the bound constructor makes of it and returns that. The body runs
//! only when the record is resumed (`vm_gen`).
//!
//! `yield e` is `gen_yield(e)`: the frame suspends and the resumer gets `e`;
//! when resumed, the intrinsic's result is the sent value. Right after it the
//! body asks whether it was resumed to be closed (`gen_closing`), and if so
//! closes the enumerators of the loops it is in, runs its pending `defer`s
//! and returns, which ends the generator.
//!
//! `yield *xs` resumes a generator `xs` (any other value through the bound
//! `each_item(xs)`) until it ends, yielding each of its items and passing
//! each value sent in on to it.

use crate::instruction::{Instruction, Opcode};
use crate::{BytecodeCompiler, YieldOwner};
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;

/// The fault a `yield` that belongs to nothing gives.
const ORPHAN_YIELD: &str = "`yield` outside a generator function or a collecting `for`";

impl BytecodeCompiler {
    /// The `for` that block item `item` is, or wraps as an expression
    /// statement.
    pub(crate) fn item_for(arena: &AstArena, item: NodeId) -> Option<NodeId> {
        let n = arena.node(item);
        match n.node_kind() {
            NodeKind::ForExpr => Some(item),
            NodeKind::ExprStmt if arena.node(NodeId(n.lhs)).node_kind() == NodeKind::ForExpr => Some(NodeId(n.lhs)),
            _ => None,
        }
    }

    /// Whether the `for` at `node_id` is in value position, and so collects
    /// what its body yields.
    pub(crate) fn for_collects(&mut self, node_id: NodeId) -> bool {
        self.stmt_for.take() != Some(node_id)
    }

    /// Opens the collecting `for` whose result list lives in `r_list`.
    pub(crate) fn begin_collect(&mut self, r_list: u8) {
        let r_null = self.emit_null();
        self.emit(Opcode::Move, r_list, r_null, 0);
        self.yield_owners.push(YieldOwner::Collect(r_list, false));
    }

    /// Closes the innermost collecting `for`; returns the register of its
    /// value: the list of what it yielded, or `null` when its body has no
    /// `yield`.
    pub(crate) fn end_collect(&mut self) -> u8 {
        match self.yield_owners.pop() {
            Some(YieldOwner::Collect(r_list, true)) => {
                // A loop that ran no `yield` is the empty list.
                let j = self.emit_branch(Opcode::JumpIfNotNull, r_list, 0);
                let r_new = self.emit_intrinsic("list_new", &[]);
                self.emit(Opcode::Move, r_list, r_new, 0);
                self.patch_to_here(j);
                r_list
            }
            _ => self.emit_null(),
        }
    }

    pub(crate) fn compile_yield(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        let operand = NodeId(arena.node(node_id).lhs);
        if let Some(YieldOwner::Collect(r_list, _)) = self.yield_owners.last().copied() {
            let r_v = self.compile_node(arena, operand);
            let j = self.emit_branch(Opcode::JumpIfNotNull, r_list, 0);
            let r_new = self.emit_intrinsic("list_new", &[]);
            self.emit(Opcode::Move, r_list, r_new, 0);
            self.patch_to_here(j);
            self.emit_intrinsic("list_push", &[r_list, r_v]);
            if let Some(last) = self.yield_owners.last_mut() {
                *last = YieldOwner::Collect(r_list, true);
            }
            return self.emit_null();
        }
        self.gen_body.get_or_insert(self.cur_span);
        let on = arena.node(operand);
        if on.node_kind() == NodeKind::UnaryExpr && on.flags == pilot_ast::UNARY_OP_SPLAT {
            return self.compile_yield_from(arena, NodeId(on.lhs));
        }
        let r_v = self.compile_node(arena, operand);
        let r_sent = self.emit_intrinsic("gen_yield", &[r_v]);
        let keep = self.alloc_reg();
        self.emit(Opcode::Move, keep, r_sent, 0);
        self.emit_close_check(arena, None);
        keep
    }

    /// `yield *xs`.
    fn compile_yield_from(&mut self, arena: &AstArena, xs: NodeId) -> u8 {
        let Some(proto) = pilot_parser::bootstrap_tables().generator().cloned() else {
            let msg = "`yield *xs` needs the generator protocol (`bind_generator`)";
            return self.error(crate::codes::SEED, msg.to_string());
        };
        let r_gen = self.alloc_local_reg();
        let r_frame = self.alloc_local_reg();
        let r_sent = self.alloc_local_reg();
        let mark = self.next_reg;
        let r_src = self.compile_node(arena, xs);
        // A generator is resumed itself; anything else is walked by one.
        let r_type = self.emit_intrinsic("record_type", &[r_src]);
        let r_name = self.emit_const(PilotVal::from_str(&proto.type_name));
        let r_is = self.alloc_reg();
        self.emit(Opcode::CmpEq, r_is, r_type, r_name);
        self.emit(Opcode::Move, r_gen, r_src, 0);
        let is_gen = self.emit_branch(Opcode::JumpIfTrue, r_is, 0);
        let r_wrapped = self.emit_user_call(proto.each_item.clone(), proto.each_item.clone(), &[r_src]);
        self.emit(Opcode::Move, r_gen, r_wrapped, 0);
        self.patch_to_here(is_gen);
        let r_f = self.emit_member_read(r_gen, Some(&proto.type_name), &proto.frame_field);
        self.emit(Opcode::Move, r_frame, r_f, 0);
        let r_null = self.emit_null();
        self.emit(Opcode::Move, r_sent, r_null, 0);
        self.release_to(mark);

        let head = self.here();
        let r_item = self.emit_intrinsic("gen_resume", &[r_frame, r_sent]);
        let r_done = self.emit_intrinsic("gen_done", &[r_frame]);
        let exit = self.emit_branch(Opcode::JumpIfTrue, r_done, 0);
        let r_in = self.emit_intrinsic("gen_yield", &[r_item]);
        self.emit(Opcode::Move, r_sent, r_in, 0);
        self.release_to(mark);
        self.emit_close_check(arena, Some(r_frame));
        self.emit_jump(head);
        self.patch_to_here(exit);
        self.release_to(mark);
        self.emit_null()
    }

    /// After a `gen_yield`: when the generator was resumed to be closed,
    /// closes `inner` (the frame a `yield *` delegates to), the enumerators
    /// of the loops around, runs the pending `defer`s and returns.
    fn emit_close_check(&mut self, arena: &AstArena, inner: Option<u8>) {
        let mark = self.next_reg;
        let r_closing = self.emit_intrinsic("gen_closing", &[]);
        let go_on = self.emit_branch(Opcode::JumpIfFalse, r_closing, 0);
        if let Some(r_frame) = inner {
            self.emit_intrinsic("gen_close", &[r_frame]);
        }
        self.emit_loop_releases(0);
        self.emit_defers_above(arena, 0);
        let r_null = self.emit_null();
        self.emit(Opcode::Move, 0, r_null, 0);
        self.emit(Opcode::Ret, 0, 0, 0);
        self.patch_to_here(go_on);
        self.release_to(mark);
    }

    /// Closes the enumerators of the loops from `from` (an index into the
    /// open loops) inward, innermost first.
    pub(crate) fn emit_loop_releases(&mut self, from: usize) {
        let held: Vec<u8> = self.loops.iter().skip(from).rev().filter_map(|l| l.release).collect();
        for r_enum in held {
            self.emit_release(r_enum);
        }
    }

    /// Closes the enumerator in `r_enum` when it is a record of a struct
    /// declaring the bound close method; anything else is left alone.
    pub(crate) fn emit_release(&mut self, r_enum: u8) {
        let Some(close) = pilot_parser::bootstrap_tables()
            .enumerator()
            .map(|p| p.close.clone())
            .filter(|c| !c.is_empty())
        else {
            return;
        };
        let mut owners: Vec<String> = self
            .method_owners
            .get(&close)
            .map(|v| v.iter().filter(|(_, is_static)| !is_static).map(|(o, _)| o.clone()).collect())
            .unwrap_or_default();
        owners.retain(|o| self.structs.contains_key(o));
        if owners.is_empty() {
            return;
        }
        owners.sort();
        owners.dedup();
        let mark = self.next_reg;
        let skip = self.emit_branch(Opcode::JumpIfNull, r_enum, 0);
        let r_type = self.emit_intrinsic("record_type", &[r_enum]);
        let mut ends = Vec::new();
        for owner in owners {
            let inner = self.next_reg;
            let r_name = self.emit_const(PilotVal::from_str(&owner));
            let r_is = self.alloc_reg();
            self.emit(Opcode::CmpEq, r_is, r_type, r_name);
            let next = self.emit_branch(Opcode::JumpIfFalse, r_is, 0);
            self.emit_user_call(format!("{owner}.{close}"), close.clone(), &[r_enum]);
            ends.push(self.emit_jump(0));
            self.patch_to_here(next);
            self.release_to(inner);
        }
        for j in ends {
            self.patch_to_here(j);
        }
        self.patch_to_here(skip);
        self.release_to(mark);
    }

    /// The stub of a generator function whose `Nop` entry is at `entry`:
    /// builds the generator around a new frame and returns it.
    pub(crate) fn emit_generator_stub(&mut self, entry: usize) {
        let Some(proto) = pilot_parser::bootstrap_tables().generator().cloned() else {
            let msg = "a function that yields needs the generator protocol (`bind_generator`)";
            self.error(crate::codes::SEED, msg.to_string());
            return;
        };
        let stub = self.here();
        self.instructions[entry] = Instruction::encode_wide(Opcode::Jump, 0, stub);
        // The frame resumes right after the entry, with the call's registers
        // as they are here: nothing above has run.
        let r_hook = self.emit_const(PilotVal::from_str("gen_new"));
        let r_frame = self.alloc_reg();
        let body = (entry + 1) as u64;
        let ins = Instruction::encode(Opcode::EngineIntrinsic, r_frame, r_hook, 0);
        self.push_ins(Instruction(ins.0 | (body << 32)));
        let r = self.emit_user_call(proto.construct.clone(), proto.construct.clone(), &[r_frame]);
        self.emit(Opcode::Move, 0, r, 0);
        self.emit(Opcode::Ret, 0, 0, 0);
    }

    /// A `yield` at a script's top level belongs to nothing.
    pub(crate) fn check_top_level_yield(&mut self) {
        if let Some(span) = self.gen_body.take() {
            self.cur_span = span;
            self.error(crate::codes::GEN_ORPHAN_YIELD, ORPHAN_YIELD.to_string());
        }
    }
}
