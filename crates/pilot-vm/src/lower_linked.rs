//! Pilot functions the seed compiles into a program on demand.
//!
//! Some constructs are implemented in Pilot (see `pilot_parser::OperatorFn`).
//! The first time a program uses one, the seed compiles every function of the
//! module that implements it into the program, once, and calls the function
//! from then on. Real module linking (`use`) replaces this in F2.

use crate::BytecodeCompiler;
use pilot_ast::{NodeId, NodeKind};

impl BytecodeCompiler {
    /// Compiles the functions of the Pilot module `src` into the program,
    /// unless they already are.
    pub(crate) fn link_pilot_module(&mut self, src: &str) {
        if !self.linked_modules.insert(src.to_string()) {
            return;
        }
        let arena = pilot_parser::parse(src);
        let root = arena.node(arena.root);
        let fns: Vec<NodeId> = arena
            .children_slice(root.lhs, root.rhs)
            .iter()
            .map(|&c| NodeId(c))
            .filter(|&c| arena.node(c).node_kind() == NodeKind::FnDecl)
            .collect();
        let saved = self.save_fn_state();
        let skip = self.emit_jump(0);
        let mut no_entry = None;
        for f in fns {
            self.compile_fn_decl(&arena, f, None, &mut no_entry);
        }
        self.patch_to_here(skip);
        self.restore_fn_state(saved);
    }

    /// `a op b` for an operator a Pilot function implements, or `None`.
    pub(crate) fn compile_operator_fn(
        &mut self,
        symbol: &str,
        arena: &pilot_ast::AstArena,
        lhs: NodeId,
        rhs: NodeId,
    ) -> Option<u8> {
        let op = pilot_parser::bootstrap_tables().operator_fn(symbol)?;
        let (function, module) = (op.function.clone(), op.module.clone());
        self.link_pilot_module(&module);
        let mark = self.next_reg;
        let r_l = self.compile_node(arena, lhs);
        let r_r = self.compile_node(arena, rhs);
        let r = self.emit_user_call(function.clone(), function, &[r_l, r_r]);
        self.emit(crate::instruction::Opcode::Move, mark, r, 0);
        self.release_to(mark + 1);
        Some(mark)
    }
}
