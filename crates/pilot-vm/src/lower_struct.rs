//! Struct instances: field defaults, struct literals and constructors.
//!
//! An instance is a record. `Name { a: 1 }` builds the record from the
//! literal, then fills every field the literal leaves out that declares a
//! default (`b: i64 = 2`). `Name(args)` builds the defaulted record and runs
//! the struct's constructor `self(params) { .. }` on it, with `self` bound to
//! the record; the call's value is the record.
//!
//! Every record built this way remembers its struct, so a method call on a
//! value whose type the compiler cannot see is dispatched at run time.
//!
//! A constructor is compiled the first time a `Name(args)` call is lowered,
//! like a generic body checked at instantiation: a struct whose constructor
//! needs types from modules that are not linked yet can still be used for its
//! other members.

use crate::instruction::Opcode;
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;

/// What the compiler knows about a declared struct.
#[derive(Clone, Default)]
pub(crate) struct StructInfo {
    /// Fields in declaration order, with their default initializer or `NONE`.
    pub(crate) fields: Vec<(String, NodeId)>,
    /// The `self(..)` declaration, if the struct has one.
    pub(crate) ctor: Option<NodeId>,
}

impl BytecodeCompiler {
    /// Members of a struct body, looking through header-group blocks
    /// (`pub { .. }`).
    pub(crate) fn type_members(arena: &AstArena, body: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        if body.is_none() {
            return out;
        }
        let bn = arena.node(body);
        if bn.node_kind() != NodeKind::BlockExpr {
            return out;
        }
        for &m in arena.children_slice(bn.lhs, bn.rhs) {
            let id = NodeId(m);
            if arena.node(id).node_kind() == NodeKind::BlockExpr {
                out.extend(Self::type_members(arena, id));
            } else {
                out.push(id);
            }
        }
        out
    }

    /// Whether type member `m` is a method: a `fn`, or an `op` named by its
    /// operator.
    pub(crate) fn is_method_decl(arena: &AstArena, m: NodeId) -> bool {
        matches!(arena.node(m).node_kind(), NodeKind::FnDecl | NodeKind::OperatorDecl)
    }

    /// Records the methods a struct or `ext` block declares, and a struct's
    /// fields and constructor.
    pub(crate) fn register_struct(&mut self, arena: &AstArena, decl: NodeId) {
        let dn = *arena.node(decl);
        if dn.rhs == u32::MAX || dn.rhs == 0 {
            return;
        }
        let owner = Self::name(arena, dn.lhs).to_string();
        for m in Self::type_members(arena, NodeId(dn.rhs)) {
            if !Self::is_method_decl(arena, m) {
                continue;
            }
            let is_static =
                arena.decl_flags.get(m.0 as usize).copied().unwrap_or(0) & pilot_ast::DECL_FLAG_STATIC != 0;
            let owners = self
                .method_owners
                .entry(Self::name(arena, arena.node(m).lhs).to_string())
                .or_default();
            if !owners.iter().any(|(o, _)| *o == owner) {
                owners.push((owner.clone(), is_static));
            }
            let key = format!("{owner}.{}", Self::name(arena, arena.node(m).lhs));
            self.register_params(key, m);
        }
        if dn.node_kind() != NodeKind::StructDecl {
            return;
        }
        // A `partial struct` adds to the parts declared before it.
        let mut info = self.structs.remove(Self::name(arena, dn.lhs)).unwrap_or_default();
        for m in Self::type_members(arena, NodeId(dn.rhs)) {
            let mn = arena.node(m);
            match mn.node_kind() {
                NodeKind::ParamDecl => info
                    .fields
                    .push((Self::name(arena, mn.lhs).to_string(), arena.field_default(m))),
                // `var x: T = v;` and `let x: T;` in a struct body are fields too.
                NodeKind::VarStmt | NodeKind::LetStmt => {
                    info.fields.push((Self::name(arena, mn.lhs).to_string(), NodeId(mn.rhs)))
                }
                NodeKind::ConstructorDecl => {
                    info.ctor = Some(m);
                    self.register_params(format!("{owner}.{}", &crate::RECEIVER[1..]), m);
                }
                _ => {}
            }
        }
        self.structs.insert(owner, info);
    }

    /// Fields of struct `name` in declaration order, with their defaults.
    pub(crate) fn struct_fields(&self, name: &str) -> Vec<(String, NodeId)> {
        self.structs.get(name).map(|s| s.fields.clone()).unwrap_or_default()
    }

    /// A new empty record of struct `name`.
    pub(crate) fn emit_new_record(&mut self, name: &str) -> u8 {
        let r_obj = self.emit_intrinsic("map_new", &[]);
        self.emit_record_type(r_obj, name);
        r_obj
    }

    /// Marks the record in `r_obj` as built from struct `name`, which is what
    /// method calls dispatch on.
    fn emit_record_type(&mut self, r_obj: u8, name: &str) {
        let mark = self.next_reg;
        let r_name = self.emit_const(PilotVal::from_str(name));
        self.emit_intrinsic("record_set_type", &[r_obj, r_name]);
        self.release_to(mark);
    }

    /// Sets each defaulted field of `name` that is not in `given` on `r_obj`.
    pub(crate) fn emit_field_defaults(&mut self, arena: &AstArena, name: &str, given: &[String], r_obj: u8) {
        let Some(info) = self.structs.get(name).cloned() else {
            return;
        };
        for (field, default) in info.fields {
            if default.is_none() || given.contains(&field) {
                continue;
            }
            let mark = self.next_reg;
            let r_v = self.compile_node(arena, default);
            let r_k = self.emit_const(PilotVal::from_str(&field));
            self.emit_intrinsic("map_set", &[r_obj, r_k, r_v]);
            self.release_to(mark);
        }
    }

    /// `Name { field: value, .. }`.
    pub(crate) fn compile_struct_literal(&mut self, arena: &AstArena, name: &str, literal: NodeId) -> u8 {
        self.check_exported(name);
        let ln = *arena.node(literal);
        let r_obj = self.compile_map_literal(arena, ln.lhs, ln.rhs);
        self.emit_record_type(r_obj, name);
        let given: Vec<String> = arena
            .children_slice(ln.lhs, ln.rhs)
            .chunks(2)
            .filter_map(|pair| Self::map_key(arena, NodeId(pair[0])))
            .collect();
        self.emit_field_defaults(arena, name, &given, r_obj);
        r_obj
    }

    /// `Name(args)` for a declared struct; `None` when `name` is not one.
    pub(crate) fn compile_ctor_call(&mut self, arena: &AstArena, name: &str, args: &[NodeId]) -> Option<u8> {
        let ctor = self.structs.get(name)?.ctor;
        self.check_exported(name);
        let Some(ctor) = ctor else {
            if !args.is_empty() {
                return Some(self.compile_memberwise(arena, name, args));
            }
            let r_obj = self.emit_new_record(name);
            self.emit_field_defaults(arena, name, &[], r_obj);
            return Some(r_obj);
        };
        let full = format!("{name}.{}", &crate::RECEIVER[1..]);
        if !self.fn_entries.contains_key(&full) {
            let saved = self.save_fn_state();
            let skip = self.emit_jump(0);
            self.compile_ctor(arena, ctor, name);
            self.patch_to_here(skip);
            self.restore_fn_state(saved);
        }
        let r_obj = self.emit_new_record(name);
        self.emit_field_defaults(arena, name, &[], r_obj);
        let mut regs = vec![r_obj];
        regs.extend(args.iter().map(|&a| self.compile_node(arena, a)));
        Some(self.emit_user_call(full.clone(), full, &regs))
    }

    /// Lowers `self(params) { body }` as the function `Owner.self`, whose
    /// first parameter is the record being constructed. It returns the record.
    fn compile_ctor(&mut self, arena: &AstArena, ctor: NodeId, owner: &str) {
        let cn = *arena.node(ctor);
        let children = arena.children_slice(cn.lhs, cn.rhs).to_vec();
        let Some((&body, params)) = children.split_first() else {
            return;
        };
        let receiver = crate::RECEIVER;
        let mut names = vec![receiver.to_string()];
        names.extend(params.iter().map(|&p| Self::name(arena, arena.node(NodeId(p)).lhs).to_string()));
        let pc = self.here();
        self.fn_entries
            .insert(format!("{owner}.{}", &receiver[1..]), (pc, names.clone()));
        self.reset_fn_state();
        for (i, p) in names.iter().enumerate() {
            let slot = self.alloc_local_reg();
            self.emit(Opcode::Move, slot, (i + 1) as u8, 0);
            self.var_slots.insert(p.clone(), slot);
        }
        let self_slot = self.var_slots[receiver];
        self.var_slots.insert(receiver[1..].to_string(), self_slot);
        let mark = self.next_reg;
        self.compile_node(arena, NodeId(body));
        self.release_to(mark);
        self.emit(Opcode::Move, 0, self_slot, 0);
        self.emit(Opcode::Ret, 0, 0, 0);
    }
}
