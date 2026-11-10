//! Fluid cascades: `recv..method(args)..field = value`.
//!
//! The receiver is evaluated once. Each step is a method call or a field
//! assignment applied to it, and the cascade's value is the receiver itself.
//! `recv?..step` skips every step when the receiver is `none`, and the
//! cascade's value is then `none`.

use crate::instruction::Opcode;
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;

impl BytecodeCompiler {
    pub(crate) fn compile_cascade(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        let node = *arena.node(node_id);
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r = self.compile_node(arena, NodeId(node.lhs));
        self.emit(Opcode::Move, dst, r, 0);
        self.release_to(mark);
        let skip = (node.flags != 0).then(|| self.emit_branch(Opcode::JumpIfNull, dst, 0));
        let owner = self.static_type(arena, NodeId(node.lhs)).unwrap_or_default();
        self.compile_cascade_step(arena, NodeId(node.rhs), dst, &owner);
        self.release_to(mark);
        if let Some(j) = skip {
            self.patch_to_here(j);
        }
        dst
    }

    fn compile_cascade_step(&mut self, arena: &AstArena, step: NodeId, r_recv: u8, owner: &str) {
        if step.is_none() {
            self.error("a cascade `..` needs a step".to_string());
            return;
        }
        self.cur_span = arena.span(step);
        let sn = *arena.node(step);
        match sn.node_kind() {
            NodeKind::CallExpr if Self::is_quoted_call(&sn) => {
                self.compile_quoted_call();
            }
            NodeKind::CallExpr => {
                let children = arena.children_slice(sn.lhs, sn.rhs).to_vec();
                let Some((&callee, args)) = children.split_first() else {
                    self.error("malformed call".to_string());
                    return;
                };
                let cn = *arena.node(NodeId(callee));
                if cn.node_kind() != NodeKind::IdentExpr {
                    self.error("a cascade step calls a method by name: `..name(args)`".to_string());
                    return;
                }
                let args: Vec<NodeId> = args.iter().map(|&a| NodeId(a)).collect();
                let method = Self::name(arena, cn.lhs).to_string();
                self.emit_method_call(arena, r_recv, owner, &method, &args);
            }
            NodeKind::AssignStmt => {
                let tn = *arena.node(NodeId(sn.lhs));
                if tn.node_kind() != NodeKind::IdentExpr {
                    self.error("a cascade step assigns a field by name: `..name = value`".to_string());
                    return;
                }
                let r_v = self.compile_node(arena, NodeId(sn.rhs));
                let r_k = self.emit_const(PilotVal::from_str(Self::name(arena, tn.lhs)));
                self.emit_intrinsic("map_set", &[r_recv, r_k, r_v]);
            }
            _ => {
                self.error("a cascade step must be a method call or a field assignment".to_string());
            }
        }
    }
}
