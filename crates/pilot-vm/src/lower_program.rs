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
//! At run time each unit, in link order, runs its `comptime` code and then
//! its top-level statements, so an imported module's globals are set before
//! any module that imports it runs, its `comptime` code included; then the
//! entry unit's `main` runs.
//!
//! `without Item;` and `without Item { member, .. };` in a unit hide those
//! names from every other unit; a reference from another unit is an error.
//!
//! Top-level functions of the same name in two units are two functions. A
//! bare name reaches the first one registered (the entry unit's, else the
//! first imported unit's); a call from inside another unit that declares
//! the name reaches that unit's own (`unit_fn_key`).

use crate::instruction::Opcode;
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId, NodeKind};

/// A declaration of an imported unit, compiled when first called.
#[derive(Clone)]
pub(crate) struct LazyDecl {
    /// The function or method declaration, or an accessor's body.
    pub(crate) node: NodeId,
    /// The struct or `ext` type of a method.
    pub(crate) owner: Option<String>,
    pub(crate) kind: LazyKind,
    /// Given to the owner by a splice or an interface rather than declared
    /// by it; a method the owner declares replaces it.
    pub(crate) inherited: bool,
}

/// What a lazily compiled declaration lowers to.
#[derive(Clone)]
pub(crate) enum LazyKind {
    /// A function or method; `alias` is the name a splice exposes it under.
    Method { alias: Option<String> },
    /// The getter of property `field`.
    Getter { field: String },
    /// The setters of property `field`: its setter bodies in declaration
    /// order, tried in that order.
    Setter { field: String, bodies: Vec<NodeId> },
    /// The type's `~self()`, called when one of its values is dropped.
    Destructor,
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
    /// the `comptime` code, then each unit's top-level statements, each unit
    /// starting with its `comptime` runs, then a call to a zero-argument
    /// `main` when the entry unit declares one.
    pub(crate) fn compile_units(&mut self, arena: &AstArena, units: &[Vec<NodeId>]) -> u8 {
        let Some((entry_items, imported)) = units.split_last() else {
            return self.emit_null();
        };
        self.index_declarations(arena, units);
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
        // Each unit's comptime runs go after the program, once every one is
        // compiled; the unit's top-level statements start by jumping to them.
        let mut unit_starts: Vec<(usize, u32)> = Vec::with_capacity(units.len());
        let mut last = 0;
        for items in units {
            let to_runs = self.emit_jump(0);
            unit_starts.push((to_runs, self.here()));
            for &id in items {
                if !Self::runs_at_top_level(arena.node(id).node_kind()) {
                    continue;
                }
                let mark = self.next_reg;
                self.stmt_for = Self::item_for(arena, id);
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
        self.check_top_level_yield();
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
        self.place_comptime_runs(&unit_starts, entry.is_none());
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
                    let unit = arena.unit_of(id).map(|u| u.file);
                    self.fn_homes.entry(name.clone()).or_insert(unit);
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
                    let decl = LazyDecl { node: id, owner: None, kind: LazyKind::Method { alias: None }, inherited: false };
                    self.lazy_decls.entry(name.clone()).or_insert(decl);
                    // The unit's own copy, for calls from inside the unit.
                    if let Some(u) = arena.unit_of(id).map(|u| u.file) {
                        let key = Self::unit_fn_name(&name, u);
                        let kind = LazyKind::Method { alias: Some(key.clone()) };
                        self.lazy_decls.entry(key).or_insert(LazyDecl { node: id, owner: None, kind, inherited: false });
                    }
                }
                NodeKind::StructDecl | NodeKind::ExtendDecl if cn.rhs != 0 && cn.rhs != u32::MAX => {
                    let owner = Self::name(arena, cn.lhs).to_string();
                    for m in Self::type_members(arena, NodeId(cn.rhs)) {
                        if arena.node(m).node_kind() == NodeKind::DestructorDecl {
                            let decl = LazyDecl {
                                node: m,
                                owner: Some(owner.clone()),
                                kind: LazyKind::Destructor,
                                inherited: false,
                            };
                            self.lazy_decls.entry(Self::destructor_name(&owner)).or_insert(decl);
                            continue;
                        }
                        if !Self::is_method_decl(arena, m) {
                            continue;
                        }
                        let key = format!("{owner}.{}", Self::name(arena, arena.node(m).lhs));
                        let decl = LazyDecl {
                            node: m,
                            owner: Some(owner.clone()),
                            kind: LazyKind::Method { alias: None },
                            inherited: false,
                        };
                        self.lazy_decls.entry(key).or_insert(decl);
                    }
                }
                _ => {}
            }
        }
    }

    /// The key of unit `u`'s own top-level function `name`.
    fn unit_fn_name(name: &str, u: pilot_span::FileId) -> String {
        format!("{name}@{}", u.0)
    }

    /// The function a bare call to `name` from the unit being compiled
    /// reaches: that unit's own `name` when the bare name belongs to another
    /// unit and this one declares it too, else `name`.
    pub(crate) fn unit_fn_key(&self, name: &str) -> String {
        let Some(u) = self.cur_unit else {
            return name.to_string();
        };
        match self.fn_homes.get(name) {
            Some(&home) if home != Some(u) => {
                let key = Self::unit_fn_name(name, u);
                if self.lazy_decls.contains_key(&key) { key } else { name.to_string() }
            }
            _ => name.to_string(),
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
        match (&decl.kind, decl.owner.as_deref()) {
            (LazyKind::Method { alias }, owner) => {
                self.compile_fn_decl_as(arena, decl.node, owner, alias.as_deref(), &mut no_entry)
            }
            (LazyKind::Destructor, Some(owner)) => self.compile_destructor(arena, owner, decl.node),
            (kind, Some(owner)) => self.compile_accessor(arena, name, decl.node, owner, kind),
            (_, None) => {}
        }
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
            self.error(
                crate::codes::NOT_EXPORTED,
                format!("`{name}` is not exported by the module that declares it (`without`)"),
            );
        }
    }
}
