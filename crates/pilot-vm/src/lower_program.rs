//! Whole programs, single-file or linked from several units.
//!
//! A linked program is the units of `AstArena::units` in link order: every
//! unit after the units it imports, the entry unit last. Each unit's
//! declarations are registered first, so any unit can name what another
//! declares. The entry unit's functions are compiled up front. A function or
//! method of an imported unit is compiled the first time a call to it is
//! resolved, like a constructor or a generic body: code the program never
//! reaches is never lowered, so an imported module can hold constructs the
//! seed does not lower yet without breaking programs that do not use them.
//!
//! At run time the `comptime` code of every unit runs first, then the
//! top-level statements of each unit in link order (an imported module's
//! globals are set before its importers run), then the entry unit's `main`.
//!
//! `without Item;` and `without Item { member, .. };` in a unit hide those
//! names from every other unit; a reference from another unit is an error.

use crate::instruction::Opcode;
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId, NodeKind};

/// A declaration of an imported unit, compiled when first called.
#[derive(Clone)]
pub(crate) struct LazyDecl {
    pub(crate) node: NodeId,
    /// The struct or `ext` type of a method.
    pub(crate) owner: Option<String>,
}

impl BytecodeCompiler {
    /// A single-file program.
    pub(crate) fn compile_program(&mut self, arena: &AstArena, lhs: u32, rhs: u32) -> u8 {
        let items: Vec<NodeId> = arena.children_slice(lhs, rhs).iter().map(|&c| NodeId(c)).collect();
        self.compile_units(arena, &[items])
    }

    /// The top-level items of each unit of a linked program.
    pub(crate) fn unit_items(arena: &AstArena) -> Vec<Vec<NodeId>> {
        arena
            .units
            .iter()
            .map(|u| {
                let root = arena.node(u.root);
                arena.children_slice(root.lhs, root.rhs).iter().map(|&c| NodeId(c)).collect()
            })
            .collect()
    }

    /// Lowers the units of a program; the last is the entry unit. Constants,
    /// enums, types, functions and globals of every unit are registered
    /// first, then the entry unit's functions and methods are compiled, then
    /// the `comptime` code, then the top-level statements of each unit, then
    /// a call to a zero-argument `main` when the entry unit declares one.
    pub(crate) fn compile_units(&mut self, arena: &AstArena, units: &[Vec<NodeId>]) -> u8 {
        let Some((entry_items, imported)) = units.split_last() else {
            return self.emit_null();
        };
        // The entry unit registers first, so its names win over imported ones.
        self.register_items(arena, entry_items);
        for items in imported {
            self.register_items(arena, items);
            self.register_lazy(arena, items);
        }
        let skip = self.emit_jump(0);
        let mut entry = None;
        for &id in entry_items {
            match arena.node(id).node_kind() {
                NodeKind::FnDecl => self.compile_fn_decl(arena, id, None, &mut entry),
                NodeKind::StructDecl | NodeKind::ExtendDecl => self.compile_type_members(arena, id, &mut entry),
                _ => {}
            }
        }
        for items in units {
            for &id in items {
                if arena.node(id).node_kind() == NodeKind::ComptimeExpr {
                    self.compile_comptime(arena, id);
                }
            }
        }
        self.patch_to_here(skip);
        self.reset_fn_state();
        // The comptime runs go after the program, once every one is compiled,
        // and run first: the program starts by jumping to them.
        let to_runs = self.emit_jump(0);
        let body = self.here();
        let mut last = 0;
        for items in units {
            for &id in items {
                if !Self::runs_at_top_level(arena.node(id).node_kind()) {
                    continue;
                }
                let mark = self.next_reg;
                last = if self.is_global_decl(arena, id) {
                    self.compile_global_let(arena, id)
                } else {
                    self.compile_node(arena, id)
                };
                if last >= crate::FIRST_TEMP_REG {
                    self.next_reg = mark;
                }
            }
        }
        let result = match entry {
            Some(ep) => {
                // The program's own call into `main` has no source position.
                self.cur_span = pilot_span::ByteSpan::default();
                self.push_ins(crate::Instruction::encode_wide(Opcode::Call, 0, ep));
                self.emit(Opcode::Halt, 0, 0, 0);
                0
            }
            None => last,
        };
        self.place_comptime_runs(to_runs, body, entry.is_none());
        result
    }

    /// Whether a top-level item of this kind is a statement that runs when
    /// the program starts, rather than a declaration.
    fn runs_at_top_level(kind: NodeKind) -> bool {
        !matches!(
            kind,
            NodeKind::FnDecl
                | NodeKind::StructDecl
                | NodeKind::ExtendDecl
                | NodeKind::ComptimeExpr
                | NodeKind::ImportDecl
                | NodeKind::ExcludeDecl
                | NodeKind::InterfaceDecl
        )
    }

    /// Records the constants, enums, types, functions and globals `items`
    /// declare. A name an earlier unit already declared keeps that meaning.
    fn register_items(&mut self, arena: &AstArena, items: &[NodeId]) {
        for &id in items {
            let cn = *arena.node(id);
            match cn.node_kind() {
                NodeKind::LetStmt | NodeKind::ConstDecl | NodeKind::VarStmt => {
                    let pattern = Self::is_destructuring_let(arena, id);
                    if cn.node_kind() != NodeKind::VarStmt && !pattern {
                        let name = Self::name(arena, cn.lhs).to_string();
                        if !self.const_maps.contains_key(&name) {
                            if let Some(m) = self.eval_cmap(arena, NodeId(cn.rhs)) {
                                self.const_maps.insert(name, m);
                            }
                        }
                    }
                    self.register_global(arena, id);
                }
                NodeKind::TypeAdtDecl => self.register_enum(arena, id),
                NodeKind::StructDecl | NodeKind::ExtendDecl => self.register_struct(arena, id),
                NodeKind::FnDecl => {
                    let name = Self::name(arena, cn.lhs).to_string();
                    if !self.known_fns.contains_key(&name) {
                        let arity = (cn.flags as usize).saturating_sub(1);
                        self.known_fns.insert(name.clone(), arity);
                        self.register_params(name, id);
                    }
                }
                NodeKind::ExcludeDecl => self.register_exclusion(arena, id),
                _ => {}
            }
        }
    }

    /// Records the functions and methods of an imported unit for compiling
    /// on first use.
    fn register_lazy(&mut self, arena: &AstArena, items: &[NodeId]) {
        for &id in items {
            let cn = *arena.node(id);
            match cn.node_kind() {
                NodeKind::FnDecl => {
                    let name = Self::name(arena, cn.lhs).to_string();
                    self.lazy_decls.entry(name).or_insert(LazyDecl { node: id, owner: None });
                }
                NodeKind::StructDecl | NodeKind::ExtendDecl if cn.rhs != 0 && cn.rhs != u32::MAX => {
                    let owner = Self::name(arena, cn.lhs).to_string();
                    for m in Self::type_members(arena, NodeId(cn.rhs)) {
                        if !Self::is_method_decl(arena, m) {
                            continue;
                        }
                        let key = format!("{owner}.{}", Self::name(arena, arena.node(m).lhs));
                        self.lazy_decls.entry(key).or_insert(LazyDecl { node: m, owner: Some(owner.clone()) });
                    }
                }
                _ => {}
            }
        }
    }

    /// Compiles the imported function or method `name` if it is one and is
    /// not compiled yet. Returns whether it compiled something.
    pub(crate) fn compile_lazy(&mut self, arena: &AstArena, name: &str) -> bool {
        if self.fn_entries.contains_key(name) {
            return false;
        }
        let Some(decl) = self.lazy_decls.get(name).cloned() else {
            return false;
        };
        let saved = self.save_fn_state();
        let saved_span = self.cur_span;
        let skip = self.emit_jump(0);
        let mut no_entry = None;
        self.compile_fn_decl(arena, decl.node, decl.owner.as_deref(), &mut no_entry);
        self.patch_to_here(skip);
        self.restore_fn_state(saved);
        self.cur_span = saved_span;
        true
    }

    /// `without Item;` / `without Item { member, .. };`: hides `Item`, or
    /// `Item.member`, from the other units.
    fn register_exclusion(&mut self, arena: &AstArena, id: NodeId) {
        let n = *arena.node(id);
        let names: Vec<String> = arena
            .children_slice(n.lhs, n.rhs)
            .iter()
            .map(|&c| Self::name(arena, arena.node(NodeId(c)).lhs).to_string())
            .collect();
        let Some((item, members)) = names.split_first() else {
            return;
        };
        let unit = arena.unit_of(id).map(|u| u.file);
        if members.is_empty() {
            self.excluded.insert(item.clone(), unit);
        }
        for m in members {
            self.excluded.insert(format!("{item}.{m}"), unit);
        }
    }

    /// Reports a reference to `name` from outside the unit that excluded it.
    pub(crate) fn check_exported(&mut self, name: &str) {
        let Some(&home) = self.excluded.get(name) else {
            return;
        };
        if home != self.cur_unit {
            self.error(format!("`{name}` is not exported by the module that declares it (`without`)"));
        }
    }
}
