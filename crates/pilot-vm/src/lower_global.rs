//! Module-level bindings and `comptime` code.
//!
//! A top-level `let`, `var` or `const` whose value is not a compile-time
//! constant is a global: every function reads and writes it by name, and it
//! holds its value from the moment its declaration runs.
//!
//! `comptime { .. }` and `comptime expr` run during compilation in the full
//! compiler. The seed compiles each one out of line and runs them all once,
//! in source order, before any other code: before the top-level statements
//! and `main`. A `comptime` expression's value is kept and read wherever the
//! expression appears. Comptime code cannot see the local variables around it,
//! since those do not exist until the program runs.

use crate::instruction::{Instruction, Opcode};
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;

/// One `comptime` block or expression, compiled out of line.
pub(crate) struct ComptimeRun {
    /// The unit it belongs to, in link order: it runs when that unit starts.
    unit: usize,
    /// Where it starts in the source, which orders the runs of a unit.
    pos: u32,
    entry: u32,
    /// For an expression, the hidden global its value is kept in.
    result: Option<String>,
}

impl BytecodeCompiler {
    /// Records the top-level bindings that are globals.
    pub(crate) fn register_global(&mut self, arena: &AstArena, decl: NodeId) {
        let n = arena.node(decl);
        if Self::is_destructuring_let(arena, decl) {
            let mut names = Vec::new();
            Self::pattern_names(arena, NodeId(n.lhs), &mut names);
            self.globals.extend(names);
            return;
        }
        let name = Self::name(arena, n.lhs);
        if !self.const_maps.contains_key(name) {
            self.globals.insert(name.to_string());
        }
    }

    /// Whether `decl`, a top-level item, declares a global.
    pub(crate) fn is_global_decl(&self, arena: &AstArena, decl: NodeId) -> bool {
        let n = arena.node(decl);
        if !matches!(n.node_kind(), NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl) {
            return false;
        }
        Self::is_destructuring_let(arena, decl) || self.globals.contains(Self::name(arena, n.lhs))
    }

    /// Runs a top-level binding of a global.
    pub(crate) fn compile_global_let(&mut self, arena: &AstArena, decl: NodeId) -> u8 {
        if Self::is_destructuring_let(arena, decl) {
            return self.compile_global_destructuring_let(arena, decl);
        }
        let n = *arena.node(decl);
        let name = Self::name(arena, n.lhs).to_string();
        let mark = self.next_reg;
        let r_v = if NodeId(n.rhs).is_some() {
            self.compile_node(arena, NodeId(n.rhs))
        } else {
            self.emit_null()
        };
        self.emit_global_set(&name, r_v);
        self.release_to(mark);
        self.emit_null()
    }

    pub(crate) fn emit_global_get(&mut self, name: &str) -> u8 {
        let mark = self.next_reg;
        let r_k = self.emit_const(PilotVal::from_str(name));
        let r = self.emit_intrinsic("global_get", &[r_k]);
        self.emit(Opcode::Move, mark, r, 0);
        self.release_to(mark + 1);
        mark
    }

    pub(crate) fn emit_global_set(&mut self, name: &str, r_v: u8) {
        let mark = self.next_reg;
        let r_k = self.emit_const(PilotVal::from_str(name));
        self.emit_intrinsic("global_set", &[r_k, r_v]);
        self.release_to(mark);
    }

    /// Compiles a `comptime` block or expression out of line and schedules it.
    /// Where it appears, a block is `none` and an expression reads its value.
    pub(crate) fn compile_comptime(&mut self, arena: &AstArena, id: NodeId) -> u8 {
        let n = *arena.node(id);
        let saved = self.save_fn_state();
        let skip = self.emit_jump(0);
        let entry = self.here();
        self.reset_fn_state();
        let r = self.compile_node(arena, NodeId(n.lhs));
        self.emit(Opcode::Move, 0, r, 0);
        self.emit(Opcode::Ret, 0, 0, 0);
        self.restore_fn_state(saved);
        self.patch_to_here(skip);
        let result = (n.flags == 1).then(|| format!("comptime#{entry}"));
        // `arena.units` is in link order and by first node, so the unit a
        // node belongs to is the last one starting at or before it.
        let unit = arena.units.partition_point(|u| u.first_node <= id.0).saturating_sub(1);
        self.comptime_runs.push(ComptimeRun {
            unit,
            pos: arena.span(id).start,
            entry,
            result: result.clone(),
        });
        match result {
            Some(g) => self.emit_global_get(&g),
            None => self.emit_null(),
        }
    }

    /// Emits the scheduled `comptime` runs after the program, one block per
    /// unit. `unit_starts` holds, per unit in link order, the jump that starts
    /// its top-level statements and the address the block returns to. A unit
    /// without runs gets a no-op instead of its jump. When the program falls
    /// off its end (no `main`), a jump past the blocks ends it.
    pub(crate) fn place_comptime_runs(&mut self, unit_starts: &[(usize, u32)], falls_through: bool) {
        let mut runs = std::mem::take(&mut self.comptime_runs);
        runs.sort_by_key(|r| r.pos);
        let last_unit = unit_starts.len().saturating_sub(1);
        let over = (falls_through && !runs.is_empty()).then(|| self.emit_jump(0));
        for (k, &(to_runs, back)) in unit_starts.iter().enumerate() {
            let mine: Vec<&ComptimeRun> = runs.iter().filter(|r| r.unit.min(last_unit) == k).collect();
            if mine.is_empty() {
                self.instructions[to_runs] = Instruction::encode(Opcode::Nop, 0, 0, 0);
                continue;
            }
            self.patch_to_here(to_runs);
            for run in mine {
                self.emit_comptime_run(run);
            }
            self.emit_jump(back);
        }
        if let Some(j) = over {
            self.patch_to_here(j);
        }
    }

    /// Emits a call to `comptime` run `run`, keeping an expression's value.
    fn emit_comptime_run(&mut self, run: &ComptimeRun) {
        let mark = self.next_reg;
        let dst = self.alloc_reg();
        self.push_ins(Instruction::encode_wide(Opcode::Call, dst, run.entry));
        if let Some(g) = &run.result {
            self.emit_global_set(g, dst);
        }
        self.release_to(mark);
    }
}
