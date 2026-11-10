//! Pipes: `x |> f` and `f <| x` pass `x` to `f` as its first argument.
//!
//! The side that is called may be:
//! - a function or closure value, `x |> f`, which calls `f(x)`;
//! - a call, `x |> f(a, b)`, which calls `f(x, a, b)`;
//! - a prefix-dot method, `x |> .m(a)` or `x |> .m`, which calls `x.m(a)`.
//!
//! Each step is lowered as the call it stands for, so it dispatches the same
//! way a written call does.

use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_parser::MEMBER_IMPLIED_RECEIVER;

impl BytecodeCompiler {
    /// Lowers `value |> target` (and `target <| value`).
    pub(crate) fn compile_pipe(&mut self, arena: &AstArena, value: NodeId, target: NodeId) -> u8 {
        let tn = *arena.node(target);
        let (callee, args): (NodeId, Vec<NodeId>) = match tn.node_kind() {
            NodeKind::CallExpr if tn.rhs > 0 => {
                let ids = arena.children_slice(tn.lhs, tn.rhs);
                (NodeId(ids[0]), ids[1..].iter().map(|&c| NodeId(c)).collect())
            }
            _ => (target, Vec::new()),
        };
        let cn = *arena.node(callee);
        if cn.node_kind() == NodeKind::MemberExpr && cn.flags == MEMBER_IMPLIED_RECEIVER {
            let method = Self::name(arena, cn.rhs).to_string();
            let hint = self.static_type(arena, value).unwrap_or_default();
            let dst = self.alloc_reg();
            let mark = self.next_reg;
            let r_recv = self.compile_node(arena, value);
            let r = self.emit_method_call(arena, r_recv, &hint, &method, &args);
            self.emit(crate::instruction::Opcode::Move, dst, r, 0);
            self.release_to(mark);
            return dst;
        }
        let mut ids = Vec::with_capacity(args.len() + 2);
        ids.push(callee);
        ids.push(value);
        ids.extend(args);
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r = self.compile_call_ids(arena, &ids);
        self.emit(crate::instruction::Opcode::Move, dst, r, 0);
        self.release_to(mark);
        dst
    }
}
