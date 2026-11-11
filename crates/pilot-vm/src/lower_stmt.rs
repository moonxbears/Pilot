//! Statement, declaration and control-flow lowering.

use crate::{BytecodeCompiler, LoopCtx};
use crate::instruction::Opcode;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;


/// The register class `[Bits]` descriptors give unsigned integers.
const UNSIGNED_REGISTER: &str = "gpr_unsigned";
impl BytecodeCompiler {
    /// Lowers a `{ ... }` block. Scratch registers used by one statement are
    /// released before the next; the block's value is its last statement's.
    pub(crate) fn compile_block(&mut self, arena: &AstArena, lhs: u32, rhs: u32) -> u8 {
        let children: Vec<u32> = arena.children_slice(lhs, rhs).to_vec();
        self.block_scopes.push(crate::BlockScope {
            slots: self.var_slots.clone(),
            types: self.var_types.clone(),
        });
        self.defers.push(Vec::new());
        let mut last = 0;
        for (i, &c) in children.iter().enumerate() {
            let mark = self.next_reg;
            // A `for` that is a block item collects nothing (`lower_yield`).
            self.stmt_for = Self::item_for(arena, NodeId(c));
            last = self.compile_node(arena, NodeId(c));
            if i + 1 < children.len() {
                self.next_reg = mark;
            }
        }
        let value = match children.last() {
            Some(&c) if Self::item_yields_value(arena, NodeId(c)) => last,
            _ => self.emit_const(PilotVal::NULL),
        };
        let r = self.close_defer_scope(arena, value);
        if let Some(scope) = self.block_scopes.pop() {
            self.var_slots = scope.slots;
            self.var_types = scope.types;
        }
        r
    }

    /// Lowers a function. `owner` is the struct/extend name for methods.
    pub(crate) fn compile_fn_decl(
        &mut self,
        arena: &AstArena,
        fn_id: NodeId,
        owner: Option<&str>,
        entry: &mut Option<u32>,
    ) {
        self.compile_fn_decl_as(arena, fn_id, owner, None, entry);
    }

    /// Lowers a function or method; `alias` names a method a splice exposes
    /// under another name (`with { m as n }`).
    pub(crate) fn compile_fn_decl_as(
        &mut self,
        arena: &AstArena,
        fn_id: NodeId,
        owner: Option<&str>,
        alias: Option<&str>,
        entry: &mut Option<u32>,
    ) {
        let cn = *arena.node(fn_id);
        let fn_name = alias.unwrap_or(Self::name(arena, cn.lhs)).to_string();
        let (fn_start, fn_len) = (cn.rhs as usize, cn.flags as usize);
        if fn_start >= arena.extra_children.len() {
            return;
        }
        let body_node = NodeId(arena.extra_children[fn_start]);
        // A declaration without a body states a signature: the engine
        // answers it (a native-bound method) or an implementing type gives
        // the body. Compiling it would shadow the native method with a stub.
        if body_node.is_none() {
            return;
        }
        let receiver = crate::RECEIVER;
        let receiver_short = &receiver[1..];
        let mut params: Vec<String> = if fn_len > 1 {
            arena.extra_children[fn_start + 1..fn_start + fn_len]
                .iter()
                .map(|&p| Self::name(arena, arena.node(NodeId(p)).lhs).to_string())
                .collect()
        } else {
            Vec::new()
        };
        let is_static = (arena.decl_flags.get(fn_id.0 as usize).copied().unwrap_or(0)
            & pilot_ast::DECL_FLAG_STATIC)
            != 0;
        if owner.is_some() && !is_static && !params.iter().any(|p| p == receiver || p == receiver_short) {
            params.insert(0, receiver.to_string());
        }
        let pc = self.here();
        // Becomes a jump to the generator stub when the body yields.
        self.emit(Opcode::Nop, 0, 0, 0);
        if params.is_empty() && fn_name == "main" && owner.is_none() {
            *entry = Some(pc);
        }
        // A method is reached through its type (`Owner.name`), never by its
        // bare name, which belongs to free functions.
        match owner {
            Some(p) => {
                self.fn_entries.insert(format!("{p}.{fn_name}"), (pc, params.clone()));
            }
            None => {
                self.fn_entries.entry(fn_name.clone()).or_insert((pc, params.clone()));
            }
        }
        let ret_name = arena
            .type_annot(fn_id)
            .and_then(|tid| {
                let tn = arena.node(tid);
                matches!(tn.node_kind(), NodeKind::TypeNamed | NodeKind::IdentExpr)
                    .then(|| Self::name(arena, tn.lhs).to_string())
            })
            .unwrap_or_default();
        if !ret_name.is_empty() {
            let key = match owner {
                Some(p) => format!("{p}.{fn_name}"),
                None => fn_name,
            };
            self.fn_ret_types.entry(key).or_insert(ret_name.clone());
        }
        self.reset_fn_state();
        self.cur_owner = owner.map(str::to_string);
        self.cur_ret_type = (!ret_name.is_empty()).then(|| ret_name.clone());
        if fn_len > 1 {
            for &p in &arena.extra_children[fn_start + 1..fn_start + fn_len] {
                if let Some(t) = Self::annot_name(arena, NodeId(p)) {
                    self.var_types.insert(Self::name(arena, arena.node(NodeId(p)).lhs).to_string(), t);
                }
            }
        }
        if let Some(o) = owner.filter(|_| !is_static) {
            self.var_types.insert(receiver.to_string(), o.to_string());
            self.var_types.insert(receiver_short.to_string(), o.to_string());
        }
        for (i, p) in params.iter().enumerate() {
            let s = self.alloc_local_reg();
            self.emit(Opcode::Move, s, (i + 1) as u8, 0);
            self.var_slots.insert(p.clone(), s);
            if p == receiver {
                self.var_slots.insert(receiver_short.to_string(), s);
            }
            if p == receiver_short {
                self.var_slots.insert(receiver.to_string(), s);
            }
        }
        // Arguments are converted once all are in their slots: a conversion
        // stages its own arguments in the argument registers.
        for p in &params {
            let (Some(&slot), ty) = (self.var_slots.get(p), self.var_types.get(p).cloned()) else { continue };
            let mark = self.next_reg;
            let r = self.convert_for_binding(slot, ty.as_deref());
            if r != slot {
                self.emit(Opcode::Move, slot, r, 0);
            }
            self.release_to(mark);
        }
        let b = self.compile_node(arena, body_node);
        // A block-bodied function without a declared return type returns
        // nothing when it falls off the end; `=> expr` bodies return `expr`.
        let returns_value = arena.type_annot(fn_id).is_some()
            || arena.node(body_node).node_kind() != NodeKind::BlockExpr;
        let ret = self.cur_ret_type.clone();
        let r = if returns_value { self.convert_for_binding(b, ret.as_deref()) } else { self.emit_const(PilotVal::NULL) };
        self.emit(Opcode::Move, 0, r, 0);
        self.emit(Opcode::Ret, 0, 0, 0);
        if self.gen_body.is_some() {
            self.emit_generator_stub(pc as usize);
        }
    }

    /// Whether a block item supplies the block's value: an expression or a
    /// nested `if`/`switch`/block does; a declaration, assignment or loop does not.
    fn item_yields_value(arena: &AstArena, item: NodeId) -> bool {
        matches!(
            arena.node(item).node_kind(),
            NodeKind::ExprStmt | NodeKind::IfExpr | NodeKind::MatchExpr | NodeKind::BlockExpr
        )
    }

    /// The function name of `owner`'s destructor, `Owner.~self`.
    pub(crate) fn destructor_name(owner: &str) -> String {
        format!("{owner}.~{}", &crate::RECEIVER[1..])
    }

    /// Lowers destructor `m` of `owner` as a function of its receiver. An
    /// empty `~self() {}` still has a destructor to call.
    pub(crate) fn compile_destructor(&mut self, arena: &AstArena, owner: &str, m: NodeId) {
        let receiver = crate::RECEIVER;
        let pc = self.here();
        self.fn_entries.insert(Self::destructor_name(owner), (pc, vec![receiver.to_string()]));
        self.reset_fn_state();
        let s = self.alloc_local_reg();
        self.emit(Opcode::Move, s, 1, 0);
        self.var_slots.insert(receiver.to_string(), s);
        self.var_slots.insert(receiver[1..].to_string(), s);
        let body = arena.node(m).lhs;
        let b = if body == 0 { self.emit_null() } else { self.compile_node(arena, NodeId(body)) };
        self.emit(Opcode::Move, 0, b, 0);
        self.emit(Opcode::Ret, 0, 0, 0);
    }

    /// Lowers the methods of a `struct` or `extend` body.
    pub(crate) fn compile_type_members(&mut self, arena: &AstArena, id: NodeId, entry: &mut Option<u32>) {
        let cn = *arena.node(id);
        if cn.rhs == 0 || cn.rhs == u32::MAX {
            return;
        }
        let owner = Self::name(arena, cn.lhs).to_string();
        for m in Self::type_members(arena, NodeId(cn.rhs)) {
            let mn = *arena.node(m);
            match mn.node_kind() {
                NodeKind::FnDecl | NodeKind::OperatorDecl => {
                    self.compile_fn_decl(arena, m, Some(&owner), entry)
                }
                NodeKind::DestructorDecl => self.compile_destructor(arena, &owner, m),
                _ => {}
            }
        }
    }

    /// Lowers a declaration met in statement position (a local function or a
    /// nested type): its code is placed inline behind a jump.
    pub(crate) fn compile_local_decl(&mut self, arena: &AstArena, id: NodeId) -> u8 {
        let saved = self.save_fn_state();
        let skip = self.emit_jump(0);
        let mut unused_entry = None;
        if arena.node(id).node_kind() == NodeKind::FnDecl {
            self.compile_fn_decl(arena, id, None, &mut unused_entry);
        } else {
            self.register_struct(arena, id);
            self.compile_type_members(arena, id, &mut unused_entry);
        }
        self.patch_to_here(skip);
        self.restore_fn_state(saved);
        0
    }

    pub(crate) fn compile_let(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        if Self::is_destructuring_let(arena, node_id) {
            return self.compile_destructuring_let(arena, node_id);
        }
        let node = *arena.node(node_id);
        let (name, rhs) = (Self::name(arena, node.lhs).to_string(), NodeId(node.rhs));
        // A name bound outside the enclosing block is shadowed, not reused:
        // the new binding gets its own slot and its own type.
        let shadows = self.block_scopes.last().is_some_and(|outer| {
            outer.slots.get(&name).is_some() && outer.slots.get(&name) == self.var_slots.get(&name)
        });
        // The value is compiled before the binding exists, so it still sees
        // any outer binding of the same name.
        let annotated = arena.type_annot(node_id).and_then(|ann_id| {
            let an = arena.node(ann_id);
            matches!(an.node_kind(), NodeKind::TypeNamed | NodeKind::IdentExpr)
                .then(|| Self::name(arena, an.lhs).to_string())
        });
        let kept = if shadows { None } else { self.var_types.get(&name).cloned() };
        let ty = annotated
            .or(kept)
            .or_else(|| rhs.is_some().then(|| self.infer_call_type(arena, rhs)).flatten());
        let mark = self.next_reg;
        let r_val = if rhs.is_some() { self.compile_with_hint(arena, rhs, ty.clone()) } else { self.emit_null() };
        let r_val = self.convert_for_binding(r_val, ty.as_deref());
        match ty {
            Some(t) => {
                self.var_types.insert(name.clone(), t);
            }
            None => {
                self.var_types.remove(&name);
            }
        }
        let slot = match self.var_slots.get(&name) {
            Some(&s) if !shadows => s,
            _ => {
                let s = self.alloc_local_reg();
                self.reg_symbols.push((s, node.lhs, 0));
                s
            }
        };
        self.emit(Opcode::Move, slot, r_val, 0);
        self.release_to(mark);
        self.var_slots.insert(name, slot);
        slot
    }

    /// The value in `r` as a binding of type `ty` holds it. A primitive laid
    /// out in an unsigned integer register of 64 bits or more holds values the
    /// signed integer tag cannot, so the value is converted to that layout;
    /// every other binding keeps the value as it is.
    pub(crate) fn convert_for_binding(&mut self, r: u8, ty: Option<&str>) -> u8 {
        let layout = ty.and_then(|t| pilot_parser::bootstrap_tables().reg_layout(t).cloned());
        match layout {
            Some(l) if l.class == UNSIGNED_REGISTER && l.width >= 64 => self.emit_conversion(r, &l),
            _ => r,
        }
    }

    /// Static receiver type of `let x = Type.make(...)` / `let x = make(...)`,
    /// used to pick `Type.method` for later `x.method()` calls.
    fn infer_call_type(&self, arena: &AstArena, rhs: NodeId) -> Option<String> {
        let rn = arena.node(rhs);
        if rn.node_kind() != NodeKind::CallExpr || rn.rhs == 0 || (rn.lhs as usize) >= arena.extra_children.len() {
            return None;
        }
        let cn = arena.node(NodeId(arena.extra_children[rn.lhs as usize]));
        let ident_of = |n: NodeId| {
            let x = arena.node(n);
            matches!(x.node_kind(), NodeKind::IdentExpr | NodeKind::TypeNamed).then(|| Self::name(arena, x.lhs))
        };
        let (fc, type_name) = if cn.node_kind() == NodeKind::MemberExpr {
            let l_id = ident_of(NodeId(cn.lhs)).unwrap_or("");
            let l_real = self.var_types.get(l_id).map(|s| s.as_str()).unwrap_or(l_id);
            let td = if self.var_slots.contains_key(l_id) { "" } else { l_id };
            (format!("{l_real}.{}", Self::name(arena, cn.rhs)), td)
        } else {
            let nm = ident_of(NodeId(arena.extra_children[rn.lhs as usize]))?;
            // A bare call names a type only when it constructs a struct; a
            // function without a declared return type gives no type.
            (nm.to_string(), if self.structs.contains_key(nm) { nm } else { "" })
        };
        let ty = self.fn_ret_types.get(&fc).map(|s| s.as_str()).unwrap_or(type_name);
        (!ty.is_empty()).then(|| ty.to_string())
    }

    pub(crate) fn compile_assign(&mut self, arena: &AstArena, lhs: u32, rhs: u32) -> u8 {
        let lhs_n = *arena.node(NodeId(lhs));
        let replace = self.take_replace_drops();
        let rhs_r = self.compile_node(arena, NodeId(rhs));
        match lhs_n.node_kind() {
            NodeKind::IdentExpr => {
                let name = Self::name(arena, lhs_n.lhs).to_string();
                if let Some(r) = self.compile_backing_write(&name, rhs_r) {
                    return r;
                }
                let ty = self.var_types.get(&name).cloned();
                let rhs_r = self.convert_for_binding(rhs_r, ty.as_deref());
                match self.var_slots.get(&name) {
                    Some(&slot) => {
                        self.emit_drops(arena, &replace);
                        self.emit(Opcode::Move, slot, rhs_r, 0);
                        slot
                    }
                    None if self.is_captured_name(&name) => self.error(
                        crate::codes::CAPTURED_ASSIGN,
                        format!("cannot assign to `{name}` inside a lambda: closures capture variables by value"),
                    ),
                    None if self.globals.contains(&name) => {
                        self.emit_global_set(&name, rhs_r);
                        rhs_r
                    }
                    None => self.error(
                        crate::codes::UNDECLARED,
                        format!("assignment to undeclared variable `{name}`"),
                    ),
                }
            }
            NodeKind::MemberExpr => {
                let ty = self.static_type(arena, NodeId(lhs_n.lhs));
                let r_obj = self.compile_node(arena, NodeId(lhs_n.lhs));
                let field = Self::name(arena, lhs_n.rhs).to_string();
                let field_ty = ty.as_deref().and_then(|t| self.field_type(t, &field));
                let rhs_r = self.convert_for_binding(rhs_r, field_ty.as_deref());
                self.emit_member_write(r_obj, ty.as_deref(), &field, rhs_r)
            }
            NodeKind::IndexExpr => {
                let ty = self.static_type(arena, NodeId(lhs_n.lhs));
                let r_obj = self.compile_node(arena, NodeId(lhs_n.lhs));
                let r_key = self.compile_node(arena, NodeId(lhs_n.rhs));
                self.emit_index_write(r_obj, ty.as_deref(), r_key, rhs_r)
            }
            other => self.error(crate::codes::SYNTAX, format!("cannot assign to {other:?}")),
        }
    }

    pub(crate) fn compile_if(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        let node = *arena.node(node_id);
        // A binding the condition makes (`if some x = v`) ends with the `if`.
        let (outer_slots, outer_types) = (self.var_slots.clone(), self.var_types.clone());
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r_cond = self.compile_node(arena, NodeId(node.lhs));
        let jf = self.emit_branch(Opcode::JumpIfFalse, r_cond, 0);
        self.release_to(mark);
        let (then_n, else_n) = if node.flags == 2 {
            let ch = arena.children_slice(node.rhs, 2);
            (NodeId(ch[0]), NodeId(ch[1]))
        } else {
            (NodeId(node.rhs), NodeId::NONE)
        };
        let t_reg = self.compile_node(arena, then_n);
        self.emit(Opcode::Move, dst, t_reg, 0);
        self.release_to(mark);
        let j_end = self.emit_jump(0);
        self.patch_to_here(jf);
        if else_n.is_none() {
            let drops = self.else_drops(arena, node_id);
            self.emit_drops(arena, &drops);
        }
        let e_reg = self.compile_node(arena, else_n);
        self.emit(Opcode::Move, dst, e_reg, 0);
        self.release_to(mark);
        self.patch_to_here(j_end);
        self.var_slots = outer_slots;
        self.var_types = outer_types;
        dst
    }

    /// Runs `body` inside a loop context; returns the context's jump lists.
    pub(crate) fn loop_body(&mut self, arena: &AstArena, body: NodeId) -> LoopCtx {
        self.loop_body_releasing(arena, body, None)
    }

    /// `loop_body` for a loop whose enumerator, in local `release`, is closed
    /// when the loop is left.
    pub(crate) fn loop_body_releasing(&mut self, arena: &AstArena, body: NodeId, release: Option<u8>) -> LoopCtx {
        self.loops.push(LoopCtx {
            continue_jumps: Vec::new(),
            break_jumps: Vec::new(),
            is_switch: false,
            defer_depth: self.defers.len(),
            release,
        });
        let mark = self.next_reg;
        self.compile_node(arena, body);
        self.next_reg = mark;
        self.loops.pop().unwrap_or(LoopCtx {
            continue_jumps: Vec::new(),
            break_jumps: Vec::new(),
            is_switch: false,
            defer_depth: self.defers.len(),
            release,
        })
    }

    pub(crate) fn compile_while(&mut self, arena: &AstArena, cond: u32, body: u32) -> u8 {
        let start = self.here();
        let mark = self.next_reg;
        let r_cond = self.compile_node(arena, NodeId(cond));
        let jf = self.emit_branch(Opcode::JumpIfFalse, r_cond, 0);
        self.release_to(mark);
        let ctx = self.loop_body(arena, NodeId(body));
        for j in ctx.continue_jumps {
            self.patch_jump(j, start);
        }
        self.emit_jump(start);
        self.patch_to_here(jf);
        for j in ctx.break_jumps {
            self.patch_to_here(j);
        }
        self.emit_null()
    }

    pub(crate) fn compile_break(&mut self, arena: &AstArena, has_value: bool) -> u8 {
        if has_value {
            return self.error(
                crate::codes::SEED,
                "`break` with a value is not lowered by the seed yet".to_string(),
            );
        }
        let Some(depth) = self.loops.last().map(|l| l.defer_depth) else {
            return self.error(
                crate::codes::BREAK_OUTSIDE_LOOP,
                "`break` outside of a loop or switch".to_string(),
            );
        };
        self.emit_exit_drops(arena);
        self.emit_defers_above(arena, depth);
        let j = self.emit_jump(0);
        if let Some(l) = self.loops.last_mut() {
            l.break_jumps.push(j);
        }
        0
    }

    pub(crate) fn compile_continue(&mut self, arena: &AstArena, has_value: bool) -> u8 {
        if has_value {
            return self.error(
                crate::codes::SEED,
                "`continue` with new bounds is not lowered by the seed yet".to_string(),
            );
        }
        let Some(depth) = self.loops.iter().rev().find(|l| !l.is_switch).map(|l| l.defer_depth) else {
            return self.error(
                crate::codes::CONTINUE_OUTSIDE_LOOP,
                "`continue` outside of a loop".to_string(),
            );
        };
        self.emit_exit_drops(arena);
        self.emit_defers_above(arena, depth);
        let j = self.emit_jump(0);
        if let Some(l) = self.loops.iter_mut().rev().find(|l| !l.is_switch) {
            l.continue_jumps.push(j);
        }
        0
    }

    pub(crate) fn compile_return(&mut self, arena: &AstArena, value: u32) -> u8 {
        let r_ret = if NodeId(value).is_some() {
            let hint = self.cur_ret_type.clone();
            let r = self.compile_with_hint(arena, NodeId(value), hint.clone());
            self.convert_for_binding(r, hint.as_deref())
        } else {
            self.emit_null()
        };
        // The value is computed before deferred statements run.
        let keep = self.alloc_reg();
        self.emit(Opcode::Move, keep, r_ret, 0);
        self.emit_exit_drops(arena);
        self.emit_loop_releases(0);
        self.emit_defers_above(arena, 0);
        self.emit(Opcode::Move, 0, keep, 0);
        self.emit(Opcode::Ret, 0, 0, 0);
        0
    }
}
