//! Contracts: `requires(cond)` and `ensures(r => cond)` clauses, which the
//! parser puts at the start of a function's body (crates/pilot-parser/src/
//! contracts.rs), and `old(e)` inside `ensures`.
//!
//! A `requires` clause is checked where it stands, when the function is
//! entered; when it is false the program stops with DBC006. An `ensures`
//! clause takes, where it stands, the entry value of every `old(e)` in it,
//! and is checked at every `return` and at the end of the body, after the
//! returned value is computed: when it is false the program stops with
//! DBC007. In a function whose return type can fail (it declares the try
//! protocol's success predicate) a failed result skips the `ensures`
//! clauses, and `r` is the success value. A failing `?` leaves without
//! checking them either.

use crate::instruction::Opcode;
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId, NodeKind, UNARY_OP_OLD};
use std::collections::HashMap;

/// The contract state of the function being compiled.
#[derive(Clone, Debug, Default)]
pub(crate) struct ContractCtx {
    /// The function's name, for the messages.
    pub(crate) fn_name: String,
    /// Its `ensures` clauses, in order.
    pub(crate) ensures: Vec<NodeId>,
    /// The register holding the entry value of each `old(e)` node.
    pub(crate) olds: HashMap<u32, u8>,
}

impl BytecodeCompiler {
    /// A contract clause statement (`ExprStmt` with contract flags).
    pub(crate) fn compile_contract_clause(&mut self, arena: &AstArena, flags: u16, clause: u32) -> u8 {
        if flags == pilot_parser::CONTRACT_REQUIRES {
            let mark = self.next_reg;
            let r = self.compile_node(arena, NodeId(clause));
            let ok = self.emit_branch(Opcode::JumpIfTrue, r, 0);
            let name = self.contract.fn_name.clone();
            self.emit_trap(format!("DBC006: a `requires` clause of `{name}` does not hold"));
            self.patch_to_here(ok);
            self.release_to(mark);
        } else {
            self.take_olds(arena, NodeId(clause));
            self.contract.ensures.push(NodeId(clause));
        }
        self.emit_null()
    }

    /// Stores the entry value of every `old(e)` under `id` in a register
    /// of its own.
    fn take_olds(&mut self, arena: &AstArena, id: NodeId) {
        if id.is_none() {
            return;
        }
        let n = *arena.node(id);
        let list = |n: &pilot_ast::AstNode| arena.children_slice(n.lhs, n.rhs).to_vec();
        let kids: Vec<u32> = match n.node_kind() {
            NodeKind::UnaryExpr if n.flags == UNARY_OP_OLD => {
                let mark = self.next_reg;
                let r = self.compile_node(arena, NodeId(n.lhs));
                let slot = self.alloc_local_reg();
                self.emit(Opcode::Move, slot, r, 0);
                self.release_to(mark);
                self.contract.olds.insert(id.0, slot);
                return;
            }
            NodeKind::BinaryExpr | NodeKind::IndexExpr => vec![n.lhs, n.rhs],
            NodeKind::UnaryExpr | NodeKind::ExprStmt | NodeKind::MemberExpr => vec![n.lhs],
            NodeKind::LambdaExpr | NodeKind::CallArg => vec![n.rhs],
            NodeKind::CallExpr | NodeKind::BlockExpr | NodeKind::TupleExpr | NodeKind::ListExpr => list(&n),
            _ => Vec::new(),
        };
        for k in kids {
            self.take_olds(arena, NodeId(k));
        }
    }

    /// `old(e)`: the register its entry value was stored in.
    pub(crate) fn compile_old(&mut self, id: NodeId) -> u8 {
        match self.contract.olds.get(&id.0) {
            Some(&slot) => slot,
            None => self.error(
                crate::codes::SEED,
                "`old(..)` is only read inside an `ensures` clause".to_string(),
            ),
        }
    }

    /// Checks the function's `ensures` clauses against the value in `r`.
    pub(crate) fn emit_ensures(&mut self, arena: &AstArena, r: u8) {
        if self.contract.ensures.is_empty() {
            return;
        }
        let mark = self.next_reg;
        let value = self.alloc_reg();
        self.emit(Opcode::Move, value, r, 0);
        let mut skips = Vec::new();
        if let Some(p) = self.fallible_return() {
            // A failed result skips the clauses; a successful record is
            // replaced by its success value.
            let r_type = self.emit_intrinsic("record_type", &[value]);
            let plain = self.emit_branch(Opcode::JumpIfNull, r_type, 0);
            let r_ok = self.emit_method_call(arena, value, "", &p.is_success, &[]);
            skips.push(self.emit_branch(Opcode::JumpIfFalse, r_ok, 0));
            let r_v = self.emit_method_call(arena, value, "", &p.unwrap, &[]);
            self.emit(Opcode::Move, value, r_v, 0);
            self.patch_to_here(plain);
        }
        let name = self.contract.fn_name.clone();
        for clause in self.contract.ensures.clone() {
            let inner = self.next_reg;
            let c = *arena.node(clause);
            let r_cond = if c.node_kind() == NodeKind::LambdaExpr {
                let params = arena.children_slice(c.lhs, c.flags as u32).to_vec();
                let saved = self.var_slots.clone();
                if let Some(&p) = params.first() {
                    let result_name = Self::name(arena, arena.node(NodeId(p)).lhs).to_string();
                    self.var_slots.insert(result_name, value);
                }
                let r_c = self.compile_node(arena, NodeId(c.rhs));
                self.var_slots = saved;
                r_c
            } else {
                self.compile_node(arena, clause)
            };
            let ok = self.emit_branch(Opcode::JumpIfTrue, r_cond, 0);
            self.emit_trap(format!("DBC007: an `ensures` clause of `{name}` does not hold"));
            self.patch_to_here(ok);
            self.release_to(inner);
        }
        for s in skips {
            self.patch_to_here(s);
        }
        self.release_to(mark);
    }

    /// The try protocol when the function's declared return type declares
    /// its success predicate.
    fn fallible_return(&self) -> Option<pilot_parser::TryProtocol> {
        let ret = self.cur_ret_type.as_deref()?;
        let p = pilot_parser::bootstrap_tables().try_protocol()?.clone();
        let owners = self.method_owners.get(&p.is_success)?;
        owners.iter().any(|(o, _)| o == ret).then_some(p)
    }
}
