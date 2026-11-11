//! Members a struct gets beyond the methods it declares, and the reads and
//! writes that reach them.
//!
//! - **Inherited methods.** A struct also has the methods of the `*Facet` /
//!   `*Bundle` types it splices (tier 2), after the splice's `without { m }`
//!   (dropped) and `with { m as n }` (exposed as `n`), then the bodied
//!   methods of the interfaces it implements, nearest interface first
//!   (tiers 3 and 4). A name an earlier tier gives is not replaced by a later
//!   one; a method the struct declares itself always wins. Each inherited
//!   method is compiled as a method of the struct, the first time it is
//!   called, so `self` and `.x` inside it mean the struct's record.
//! - **Properties.** A field written with an accessor block has its getter
//!   compiled as `Owner.field:` and its setter as `Owner.field:=`.
//!   Reading the member of a record built from that struct calls the getter;
//!   assigning it calls the setter. Inside the accessors the bound backing
//!   name (`field`) is the record's own storage for the property and the
//!   setter's value is its parameter, or the bound value name (`value`).
//! - **Index operators.** `x[i]` and `x[i] = v` on a record of a struct with a
//!   bodied `op []` / `op []=` call them; anything else indexes the value as
//!   before.
//!
//! When the compiler knows the receiver's struct, the call is direct.
//! Otherwise it tests the record's struct against every struct with that
//! member at run time; a value that matches none is read or written as a
//! plain record, list or map.

use crate::instruction::Opcode;
use crate::lower_program::{LazyDecl, LazyKind};
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;
use std::collections::{HashSet, VecDeque};

/// Receptor sub-slots of the `*Bundle` collision modifiers (category 30,
/// `stdlib/core/syntax/module_ext.pilot`).
const SPLICE_WITH: u8 = 1;
const SPLICE_WITHOUT: u8 = 2;

/// The function name of a property accessor of `owner`.
pub(crate) fn accessor_key(owner: &str, field: &str, setter: bool) -> String {
    let part = if setter { ":=" } else { ":" };
    format!("{owner}.{field}{part}")
}

impl BytecodeCompiler {
    /// Indexes the struct and interface declarations of every unit by name,
    /// and the accessor rows of the arena, before any unit is registered.
    pub(crate) fn index_declarations(&mut self, arena: &AstArena, units: &[Vec<NodeId>]) {
        for items in units {
            for &id in items {
                let n = arena.node(id);
                if matches!(n.node_kind(), NodeKind::StructDecl | NodeKind::InterfaceDecl) {
                    let name = Self::name(arena, n.lhs).to_string();
                    self.type_decls.entry(name).or_default().push(id);
                }
            }
        }
        for &(field, block) in &arena.property_accessors {
            self.accessor_blocks.insert(field.0, block);
        }
        for &(body, param, guard) in &arena.property_setters {
            self.setter_rows.insert(body.0, (param, guard));
        }
    }

    /// Registers what struct `decl` (named `owner`) gets beyond the methods
    /// it declares: its properties, its spliced members and its interfaces'
    /// default methods.
    pub(crate) fn register_inherited(&mut self, arena: &AstArena, decl: NodeId, owner: &str) {
        let body = NodeId(arena.node(decl).rhs);
        let members = Self::type_members(arena, body);
        let mut taken: HashSet<String> = members
            .iter()
            .filter(|&&m| Self::is_method_decl(arena, m))
            .map(|&m| Self::name(arena, arena.node(m).lhs).to_string())
            .collect();
        for &m in &members {
            self.register_property(arena, owner, m);
        }
        // Tier 2: spliced facets and bundles.
        for &m in &members {
            let n = arena.node(m);
            if n.node_kind() != NodeKind::ExprStmt || NodeId(n.lhs).is_none() {
                continue;
            }
            let u = NodeId(n.lhs);
            let un = arena.node(u);
            if un.node_kind() != NodeKind::UnaryExpr || un.flags != pilot_ast::UNARY_OP_SPLAT || NodeId(un.lhs).is_none() {
                continue;
            }
            let bundle = Self::name(arena, arena.node(NodeId(un.lhs)).lhs).to_string();
            let mut dropped = HashSet::new();
            let mut aliases = std::collections::HashMap::new();
            for &(splice, sub, member, alias) in &arena.splice_mods {
                if splice != u {
                    continue;
                }
                let member = arena.interner.resolve(member).to_string();
                match sub {
                    SPLICE_WITHOUT => {
                        dropped.insert(member);
                    }
                    SPLICE_WITH if alias.0 != 0 => {
                        aliases.insert(member, arena.interner.resolve(alias).to_string());
                    }
                    _ => {}
                }
            }
            for source in self.decls_of(arena, &bundle, NodeKind::StructDecl) {
                for fm in Self::type_members(arena, NodeId(arena.node(source).rhs)) {
                    if !Self::is_method_decl(arena, fm) {
                        if !dropped.contains(Self::name(arena, arena.node(fm).lhs)) {
                            self.register_property(arena, owner, fm);
                        }
                        continue;
                    }
                    let name = Self::name(arena, arena.node(fm).lhs).to_string();
                    if dropped.contains(&name) {
                        continue;
                    }
                    let exposed = aliases.get(&name).cloned().unwrap_or(name);
                    if taken.insert(exposed.clone()) {
                        self.add_inherited(arena, owner, &exposed, fm);
                    }
                }
            }
        }
        // Tiers 3 and 4: interface defaults, nearest interface first.
        let mut queue: VecDeque<NodeId> = VecDeque::from([decl]);
        let mut seen = HashSet::new();
        while let Some(d) = queue.pop_front() {
            for &(of, base) in &arena.base_types {
                if of != d || base.is_none() {
                    continue;
                }
                let iface = Self::name(arena, arena.node(base).lhs).to_string();
                if !seen.insert(iface.clone()) {
                    continue;
                }
                for source in self.decls_of(arena, &iface, NodeKind::InterfaceDecl) {
                    for fm in Self::type_members(arena, NodeId(arena.node(source).rhs)) {
                        if !Self::is_method_decl(arena, fm) {
                            self.register_property(arena, owner, fm);
                            continue;
                        }
                        if !Self::has_body(arena, fm) {
                            continue;
                        }
                        let name = Self::name(arena, arena.node(fm).lhs).to_string();
                        if taken.insert(name.clone()) {
                            self.add_inherited(arena, owner, &name, fm);
                        }
                    }
                    queue.push_back(source);
                }
            }
        }
    }

    /// The declarations named `name` of node kind `kind`.
    pub(crate) fn decls_of(&self, arena: &AstArena, name: &str, kind: NodeKind) -> Vec<NodeId> {
        let all = self.type_decls.get(name).map(Vec::as_slice).unwrap_or_default();
        all.iter().copied().filter(|&d| arena.node(d).node_kind() == kind).collect()
    }

    /// Gives `owner` the method `node` under the name `exposed`.
    fn add_inherited(&mut self, arena: &AstArena, owner: &str, exposed: &str, node: NodeId) {
        let key = format!("{owner}.{exposed}");
        if self.lazy_decls.contains_key(&key) || self.fn_entries.contains_key(&key) {
            return;
        }
        let is_static =
            arena.decl_flags.get(node.0 as usize).copied().unwrap_or(0) & pilot_ast::DECL_FLAG_STATIC != 0;
        let owners = self.method_owners.entry(exposed.to_string()).or_default();
        if !owners.iter().any(|(o, _)| o == owner) {
            owners.push((owner.to_string(), is_static));
        }
        self.register_params(key.clone(), node);
        let alias = (Self::name(arena, arena.node(node).lhs) != exposed).then(|| exposed.to_string());
        let decl = LazyDecl {
            node,
            owner: Some(owner.to_string()),
            kind: LazyKind::Method { alias },
            inherited: true,
        };
        self.lazy_decls.insert(key, decl);
    }

    /// Whether field `m` is a property whose accessors have bodies.
    pub(crate) fn is_computed_property(&self, arena: &AstArena, m: NodeId) -> bool {
        self.accessor_blocks.get(&m.0).is_some_and(|&b| arena.node(b).rhs > 0)
    }

    /// Registers the accessors of member `m` of a type, as accessors of
    /// `owner`, when `m` is a property with accessor bodies. A property
    /// `owner` already has keeps its accessors.
    fn register_property(&mut self, arena: &AstArena, owner: &str, m: NodeId) {
        if arena.node(m).node_kind() != NodeKind::ParamDecl {
            return;
        }
        let Some(&block) = self.accessor_blocks.get(&m.0) else {
            return;
        };
        let bn = *arena.node(block);
        let bodies: Vec<NodeId> = arena.children_slice(bn.lhs, bn.rhs).iter().map(|&b| NodeId(b)).collect();
        let (getters, setters) = bodies.split_at((bn.flags as usize).min(bodies.len()));
        let field = Self::name(arena, arena.node(m).lhs).to_string();
        if let Some(&body) = getters.first() {
            let owners = self.getters.entry(field.clone()).or_default();
            if !owners.iter().any(|o| o == owner) {
                owners.push(owner.to_string());
                let decl = LazyDecl {
                    node: body,
                    owner: Some(owner.to_string()),
                    kind: LazyKind::Getter { field: field.clone() },
                    inherited: true,
                };
                self.lazy_decls.insert(accessor_key(owner, &field, false), decl);
            }
        }
        if let Some(&body) = setters.first() {
            let owners = self.setters.entry(field.clone()).or_default();
            if !owners.iter().any(|o| o == owner) {
                owners.push(owner.to_string());
                let decl = LazyDecl {
                    node: body,
                    owner: Some(owner.to_string()),
                    kind: LazyKind::Setter { field: field.clone(), bodies: setters.to_vec() },
                    inherited: true,
                };
                self.lazy_decls.insert(accessor_key(owner, &field, true), decl);
            }
        }
    }

    /// Lowers a property accessor of `owner` as the function `key`: its
    /// first parameter is the record, a setter's second is the new value.
    pub(crate) fn compile_accessor(&mut self, arena: &AstArena, key: &str, body: NodeId, owner: &str, kind: &LazyKind) {
        let (field, setters) = match kind {
            LazyKind::Getter { field } => (field.clone(), None),
            LazyKind::Setter { field, bodies } => (field.clone(), Some(bodies.clone())),
            LazyKind::Method { .. } => return,
        };
        let receiver = crate::RECEIVER;
        let mut params = vec![receiver.to_string()];
        if setters.is_some() {
            // The value travels in a hidden slot; each setter names it.
            params.push("#value".to_string());
        }
        let pc = self.here();
        self.fn_entries.insert(key.to_string(), (pc, params.clone()));
        self.reset_fn_state();
        self.cur_owner = Some(owner.to_string());
        self.var_types.insert(receiver.to_string(), owner.to_string());
        self.var_types.insert(receiver[1..].to_string(), owner.to_string());
        for (i, p) in params.iter().enumerate() {
            let slot = self.alloc_local_reg();
            self.emit(Opcode::Move, slot, (i + 1) as u8, 0);
            self.var_slots.insert(p.clone(), slot);
        }
        let self_slot = self.var_slots[receiver];
        self.var_slots.insert(receiver[1..].to_string(), self_slot);
        self.backing = Some(field.clone());
        let r = match setters {
            None => self.compile_node(arena, body),
            Some(bodies) => self.compile_setters(arena, owner, &field, &bodies),
        };
        self.backing = None;
        self.emit(Opcode::Move, 0, r, 0);
        self.emit(Opcode::Ret, 0, 0, 0);
    }

    /// The setter bodies of `owner.field`, tried in declaration order: the
    /// first whose parameter type takes the value (checked only when there
    /// are several setters) and whose guard holds runs. When none does, the
    /// program stops with an error naming the property.
    fn compile_setters(&mut self, arena: &AstArena, owner: &str, field: &str, bodies: &[NodeId]) -> u8 {
        let value_slot = self.var_slots["#value"];
        let value_ident = pilot_parser::bootstrap_tables().setter_value_ident.clone();
        let outer = self.var_slots.clone();
        for &body in bodies {
            let (param, guard) = self.setter_rows.get(&body.0).copied().unwrap_or((NodeId::NONE, NodeId::NONE));
            let name = if param.is_some() {
                Self::name(arena, arena.node(param).lhs).to_string()
            } else {
                value_ident.clone()
            };
            self.var_slots.insert(name, value_slot);
            let mark = self.next_reg;
            let mut misses = Vec::new();
            if bodies.len() > 1 && param.is_some() {
                if let Some(t) = Self::annot_name(arena, param) {
                    self.cur_span = arena.span(param);
                    let r_fits = self.emit_value_test(value_slot, &t, owner, field);
                    misses.push(self.emit_branch(Opcode::JumpIfFalse, r_fits, 0));
                }
            }
            if guard.is_some() {
                let r_ok = self.compile_node(arena, guard);
                misses.push(self.emit_branch(Opcode::JumpIfFalse, r_ok, 0));
            }
            self.release_to(mark);
            self.compile_node(arena, body);
            let r = self.emit_null();
            self.emit(Opcode::Move, 0, r, 0);
            self.emit(Opcode::Ret, 0, 0, 0);
            for j in misses {
                self.patch_to_here(j);
            }
            self.release_to(mark);
            self.var_slots = outer.clone();
        }
        if let Some(&first) = bodies.first() {
            self.cur_span = arena.span(first);
        }
        self.emit_trap(format!("no setter of `{owner}.{field}` accepts this value"))
    }

    /// Whether the value in `r_val` is of type `ty`: a type with a register
    /// layout by its register class, a struct by the record's struct. A type
    /// neither tells apart at run time stops the build with SEED001.
    fn emit_value_test(&mut self, r_val: u8, ty: &str, owner: &str, field: &str) -> u8 {
        if let Some(layout) = pilot_parser::bootstrap_tables().reg_layout(ty).cloned() {
            let r_class = self.emit_const(PilotVal::from_str(&layout.class));
            let r_width = self.emit_const(PilotVal::from_int(layout.width));
            return self.emit_intrinsic("num_fits", &[r_val, r_class, r_width]);
        }
        if self.structs.get(ty).is_some_and(|s| !s.fields.is_empty() || s.ctor.is_some()) {
            let r_type = self.emit_intrinsic("record_type", &[r_val]);
            let r_name = self.emit_const(PilotVal::from_str(ty));
            let r_is = self.alloc_reg();
            self.emit(Opcode::CmpEq, r_is, r_type, r_name);
            return r_is;
        }
        let msg = format!("the setters of `{owner}.{field}` overload on `{ty}`, which the seed cannot tell apart at run time yet");
        self.error(crate::codes::SEED, msg)
    }

    /// The record that owns the property whose accessor is being compiled,
    /// and the property's name, when `name` is the backing name.
    fn backing_target(&mut self, name: &str) -> Option<(u8, String)> {
        let field = self.backing.clone()?;
        if name.is_empty() || name != pilot_parser::bootstrap_tables().backing_ident {
            return None;
        }
        let receiver = crate::RECEIVER;
        let r_self = match self.var_slots.get(receiver).copied() {
            Some(r) => r,
            None => self.capture_read(receiver)?,
        };
        Some((r_self, field))
    }

    /// Reads the property's storage when `name` is the backing name inside
    /// an accessor.
    pub(crate) fn compile_backing_read(&mut self, name: &str) -> Option<u8> {
        let (r_self, field) = self.backing_target(name)?;
        let r_key = self.emit_const(PilotVal::from_str(&field));
        Some(self.emit_intrinsic("map_get", &[r_self, r_key]))
    }

    /// Writes the property's storage when `name` is the backing name inside
    /// an accessor.
    pub(crate) fn compile_backing_write(&mut self, name: &str, r_val: u8) -> Option<u8> {
        let (r_self, field) = self.backing_target(name)?;
        let r_key = self.emit_const(PilotVal::from_str(&field));
        self.emit_intrinsic("map_set", &[r_self, r_key, r_val]);
        Some(r_val)
    }

    /// The structs that answer member `kind`.
    fn switch_owners(&self, kind: &Switch) -> Vec<String> {
        let mut owners: Vec<String> = match kind {
            Switch::Get(f) => self.getters.get(f).cloned().unwrap_or_default(),
            Switch::Set(f) => self.setters.get(f).cloned().unwrap_or_default(),
            Switch::IndexGet | Switch::IndexSet => self
                .method_owners
                .get(kind.operator())
                .map(|v| {
                    v.iter()
                        .filter(|(o, st)| !st && self.structs.contains_key(o))
                        .map(|(o, _)| o.clone())
                        .collect()
                })
                .unwrap_or_default(),
        };
        owners.sort();
        owners
    }

    /// The plain read or write of `kind`, for a value no struct member
    /// answers; `regs` are the receiver, then the key or value operands.
    fn emit_plain(&mut self, kind: &Switch, regs: &[u8]) -> u8 {
        match kind {
            Switch::Get(f) => self.emit_field_read(regs[0], f),
            Switch::Set(f) => {
                let r_key = self.emit_const(PilotVal::from_str(f));
                self.emit_intrinsic("map_set", &[regs[0], r_key, regs[1]]);
                regs[1]
            }
            Switch::IndexGet => self.emit_intrinsic("map_get", &[regs[0], regs[1]]),
            Switch::IndexSet => {
                self.emit_intrinsic("map_set", regs);
                regs[2]
            }
        }
    }

    /// Lowers member access `kind` with operands `regs` (receiver first),
    /// whose receiver has static type `ty`. A known struct is called
    /// directly. Otherwise a value that is not a struct record takes the plain
    /// path inline and a record goes to the member's dispatcher, which
    /// `emit_dispatchers` builds once the program is linked.
    fn emit_member_access(&mut self, kind: Switch, ty: Option<&str>, regs: &[u8]) -> u8 {
        let owners = self.switch_owners(&kind);
        if owners.is_empty() {
            return self.emit_plain(&kind, regs);
        }
        if let Some(t) = ty.filter(|t| self.structs.contains_key(*t)) {
            if owners.iter().any(|o| o == t) {
                let full = kind.key(t);
                return self.emit_user_call(full.clone(), full, regs);
            }
            return self.emit_plain(&kind, regs);
        }
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r_type = self.emit_intrinsic("record_type", &regs[..1]);
        let to_plain = self.emit_branch(Opcode::JumpIfNull, r_type, 0);
        let name = kind.dispatcher();
        self.dispatchers.entry(name.clone()).or_insert((kind.clone(), None));
        let r = self.emit_user_call(name.clone(), name, regs);
        self.emit(Opcode::Move, dst, r, 0);
        let to_end = self.emit_jump(0);
        self.patch_to_here(to_plain);
        self.release_to(mark);
        let r = self.emit_plain(&kind, regs);
        self.emit(Opcode::Move, dst, r, 0);
        self.patch_to_here(to_end);
        self.release_to(mark);
        dst
    }

    /// Builds the dispatcher of every member access that needed one, testing
    /// the record's struct against each owner the compiled program builds
    /// records of. A dispatcher built before more structs were known is built
    /// again and its calls go to the new one. Returns whether it built any.
    pub(crate) fn emit_dispatchers(&mut self) -> bool {
        let mut built = false;
        let names: Vec<String> = self.dispatchers.keys().cloned().collect();
        for name in names {
            let (kind, done) = self.dispatchers[&name].clone();
            let owners: Vec<String> =
                self.switch_owners(&kind).into_iter().filter(|o| self.constructed.contains(o)).collect();
            if done.as_ref() == Some(&owners) {
                continue;
            }
            self.emit_dispatcher(&name, &kind, &owners);
            self.dispatchers.insert(name, (kind, Some(owners)));
            built = true;
        }
        built
    }

    fn emit_dispatcher(&mut self, name: &str, kind: &Switch, owners: &[String]) {
        let saved = self.save_fn_state();
        let skip = self.emit_jump(0);
        self.reset_fn_state();
        let params: Vec<String> = (0..kind.arity()).map(|i| format!("#{i}")).collect();
        let pc = self.here();
        self.fn_entries.insert(name.to_string(), (pc, params.clone()));
        let regs: Vec<u8> = (0..params.len())
            .map(|i| {
                let slot = self.alloc_local_reg();
                self.emit(Opcode::Move, slot, (i + 1) as u8, 0);
                slot
            })
            .collect();
        let r_type = self.emit_intrinsic("record_type", &regs[..1]);
        let mark = self.next_reg;
        for owner in owners {
            let r_name = self.emit_const(PilotVal::from_str(owner));
            let r_is = self.alloc_reg();
            self.emit(Opcode::CmpEq, r_is, r_type, r_name);
            let next = self.emit_branch(Opcode::JumpIfFalse, r_is, 0);
            let full = kind.key(owner);
            let r = self.emit_user_call(full.clone(), full, &regs);
            self.emit(Opcode::Move, 0, r, 0);
            self.emit(Opcode::Ret, 0, 0, 0);
            self.patch_to_here(next);
            self.release_to(mark);
        }
        let r = self.emit_plain(kind, &regs);
        self.emit(Opcode::Move, 0, r, 0);
        self.emit(Opcode::Ret, 0, 0, 0);
        self.patch_to_here(skip);
        self.restore_fn_state(saved);
    }

    /// Reads member `field` of the value in `r_obj` (static type `ty`):
    /// through the getter of a property, else as a field.
    pub(crate) fn emit_member_read(&mut self, r_obj: u8, ty: Option<&str>, field: &str) -> u8 {
        self.emit_member_access(Switch::Get(field.to_string()), ty, &[r_obj])
    }

    /// Writes `r_val` to member `field` of the value in `r_obj`: through the
    /// setter of a property, else as a field.
    pub(crate) fn emit_member_write(&mut self, r_obj: u8, ty: Option<&str>, field: &str, r_val: u8) -> u8 {
        self.emit_member_access(Switch::Set(field.to_string()), ty, &[r_obj, r_val]);
        r_val
    }

    /// `x[i]` for the value in `r_obj` (static type `ty`) and key in `r_key`.
    pub(crate) fn emit_index_read(&mut self, r_obj: u8, ty: Option<&str>, r_key: u8) -> u8 {
        self.emit_member_access(Switch::IndexGet, ty, &[r_obj, r_key])
    }

    /// `x[i] = v`.
    pub(crate) fn emit_index_write(&mut self, r_obj: u8, ty: Option<&str>, r_key: u8, r_val: u8) -> u8 {
        self.emit_member_access(Switch::IndexSet, ty, &[r_obj, r_key, r_val]);
        r_val
    }
}

/// A member access that a struct may answer with its own code.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Switch {
    /// Reading property `field`.
    Get(String),
    /// Writing property `field`.
    Set(String),
    /// `x[i]`.
    IndexGet,
    /// `x[i] = v`.
    IndexSet,
}

impl Switch {
    fn operator(&self) -> &'static str {
        match self {
            Self::IndexSet => pilot_parser::INDEX_SET_OP,
            _ => pilot_parser::INDEX_GET_OP,
        }
    }

    /// The function of `owner` that answers this access.
    fn key(&self, owner: &str) -> String {
        match self {
            Self::Get(f) => accessor_key(owner, f, false),
            Self::Set(f) => accessor_key(owner, f, true),
            Self::IndexGet | Self::IndexSet => format!("{owner}.{}", self.operator()),
        }
    }

    /// The dispatcher's function name.
    fn dispatcher(&self) -> String {
        match self {
            Self::Get(f) => format!("#{f}:"),
            Self::Set(f) => format!("#{f}:="),
            Self::IndexGet | Self::IndexSet => format!("#{}", self.operator()),
        }
    }

    /// Operands: the receiver, then the key and/or value.
    fn arity(&self) -> usize {
        match self {
            Self::Get(_) => 1,
            Self::Set(_) | Self::IndexGet => 2,
            Self::IndexSet => 3,
        }
    }
}
