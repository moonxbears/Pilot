//! Statement, declaration and control-flow lowering.

use crate::{BytecodeCompiler, LoopCtx};
use crate::instruction::Opcode;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;

impl BytecodeCompiler {
    /// Lowers a whole program: constants and enums first, then functions and
    /// methods, then top-level statements, then a call to a zero-argument
    /// `main` when one exists.
    pub(crate) fn compile_program(&mut self, arena: &AstArena, lhs: u32, rhs: u32) -> u8 {
        let children: Vec<u32> = arena.children_slice(lhs, rhs).to_vec();
        for &c_id in &children {
            let cn = arena.node(NodeId(c_id));
            match cn.node_kind() {
                NodeKind::LetStmt | NodeKind::ConstDecl | NodeKind::VarStmt => {
                    if cn.node_kind() != NodeKind::VarStmt {
                        if let Some(m) = self.eval_cmap(arena, NodeId(cn.rhs)) {
                            self.const_maps.insert(Self::name(arena, cn.lhs).to_string(), m);
                        }
                    }
                    self.register_global(arena, NodeId(c_id));
                }
                NodeKind::TypeAdtDecl => self.register_enum(arena, NodeId(c_id)),
                NodeKind::StructDecl | NodeKind::ExtendDecl => self.register_struct(arena, NodeId(c_id)),
                NodeKind::FnDecl => {
                    let arity = (cn.flags as usize).saturating_sub(1);
                    self.known_fns.insert(Self::name(arena, cn.lhs).to_string(), arity);
                }
                _ => {}
            }
        }
        let skip = self.emit_jump(0);
        let mut entry = None;
        for &c_id in &children {
            let id = NodeId(c_id);
            match arena.node(id).node_kind() {
                NodeKind::FnDecl => self.compile_fn_decl(arena, id, None, &mut entry),
                NodeKind::StructDecl | NodeKind::ExtendDecl => self.compile_type_members(arena, id, &mut entry),
                _ => {}
            }
        }
        for &c_id in &children {
            if arena.node(NodeId(c_id)).node_kind() == NodeKind::ComptimeExpr {
                self.compile_comptime(arena, NodeId(c_id));
            }
        }
        self.patch_to_here(skip);
        self.reset_fn_state();
        // The comptime runs go after the program, once every one is compiled,
        // and run first: the program starts by jumping to them.
        let to_runs = self.emit_jump(0);
        let body = self.here();
        let mut last = 0;
        for &c_id in &children {
            let kind = arena.node(NodeId(c_id)).node_kind();
            if matches!(
                kind,
                NodeKind::FnDecl | NodeKind::StructDecl | NodeKind::ExtendDecl | NodeKind::ComptimeExpr
            ) {
                continue;
            }
            let mark = self.next_reg;
            last = if self.is_global_decl(arena, NodeId(c_id)) {
                self.compile_global_let(arena, NodeId(c_id))
            } else {
                self.compile_node(arena, NodeId(c_id))
            };
            if last >= crate::FIRST_TEMP_REG {
                self.next_reg = mark;
            }
        }
        let result = match entry {
            Some(ep) => {
                self.push_ins(crate::Instruction::encode_wide(Opcode::Call, 0, ep));
                self.emit(Opcode::Halt, 0, 0, 0);
                0
            }
            None => last,
        };
        self.place_comptime_runs(to_runs, body, entry.is_none());
        result
    }

    /// Lowers a `{ ... }` block. Scratch registers used by one statement are
    /// released before the next; the block's value is its last statement's.
    pub(crate) fn compile_block(&mut self, arena: &AstArena, lhs: u32, rhs: u32) -> u8 {
        let children: Vec<u32> = arena.children_slice(lhs, rhs).to_vec();
        self.defers.push(Vec::new());
        let mut last = 0;
        for (i, &c) in children.iter().enumerate() {
            let mark = self.next_reg;
            last = self.compile_node(arena, NodeId(c));
            if i + 1 < children.len() {
                self.next_reg = mark;
            }
        }
        let value = match children.last() {
            Some(&c) if Self::item_yields_value(arena, NodeId(c)) => last,
            _ => self.emit_const(PilotVal::NULL),
        };
        self.close_defer_scope(arena, value)
    }

    /// Lowers a function. `owner` is the struct/extend name for methods.
    pub(crate) fn compile_fn_decl(
        &mut self,
        arena: &AstArena,
        fn_id: NodeId,
        owner: Option<&str>,
        entry: &mut Option<u16>,
    ) {
        let cn = *arena.node(fn_id);
        let fn_name = Self::name(arena, cn.lhs).to_string();
        let (fn_start, fn_len) = (cn.rhs as usize, cn.flags as usize);
        if fn_start >= arena.extra_children.len() {
            return;
        }
        let body_node = NodeId(arena.extra_children[fn_start]);
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
        let b = self.compile_node(arena, body_node);
        // A block-bodied function without a declared return type returns
        // nothing when it falls off the end; `=> expr` bodies return `expr`.
        let returns_value = arena.type_annot(fn_id).is_some()
            || arena.node(body_node).node_kind() != NodeKind::BlockExpr;
        let r = if returns_value { b } else { self.emit_const(PilotVal::NULL) };
        self.emit(Opcode::Move, 0, r, 0);
        self.emit(Opcode::Ret, 0, 0, 0);
    }

    /// Whether a block item supplies the block's value: an expression or a
    /// nested `if`/`switch`/block does; a declaration, assignment or loop does not.
    fn item_yields_value(arena: &AstArena, item: NodeId) -> bool {
        matches!(
            arena.node(item).node_kind(),
            NodeKind::ExprStmt | NodeKind::IfExpr | NodeKind::MatchExpr | NodeKind::BlockExpr
        )
    }

    /// Lowers the methods of a `struct` or `extend` body.
    pub(crate) fn compile_type_members(&mut self, arena: &AstArena, id: NodeId, entry: &mut Option<u16>) {
        let cn = *arena.node(id);
        if cn.rhs == 0 || cn.rhs == u32::MAX {
            return;
        }
        let owner = Self::name(arena, cn.lhs).to_string();
        for m in Self::type_members(arena, NodeId(cn.rhs)) {
            let mn = *arena.node(m);
            match mn.node_kind() {
                NodeKind::FnDecl => self.compile_fn_decl(arena, m, Some(&owner), entry),
                NodeKind::DestructorDecl if mn.lhs != 0 => {
                    let receiver = crate::RECEIVER;
                    let dtor = format!("~{}", &receiver[1..]);
                    let pc = self.here();
                    self.fn_entries
                        .insert(format!("{owner}.{dtor}"), (pc, vec![receiver.to_string()]));
                    self.reset_fn_state();
                    let s = self.alloc_local_reg();
                    self.emit(Opcode::Move, s, 1, 0);
                    self.var_slots.insert(receiver.to_string(), s);
                    self.var_slots.insert(receiver[1..].to_string(), s);
                    let b = self.compile_node(arena, NodeId(mn.lhs));
                    self.emit(Opcode::Move, 0, b, 0);
                    self.emit(Opcode::Ret, 0, 0, 0);
                }
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
        let node = *arena.node(node_id);
        let (name, rhs) = (Self::name(arena, node.lhs).to_string(), NodeId(node.rhs));
        if let Some(ann_id) = arena.type_annot(node_id) {
            let an = arena.node(ann_id);
            if matches!(an.node_kind(), NodeKind::TypeNamed | NodeKind::IdentExpr) {
                self.var_types.insert(name.clone(), Self::name(arena, an.lhs).to_string());
            }
        }
        if !self.var_types.contains_key(&name) && rhs.is_some() {
            if let Some(ty) = self.infer_call_type(arena, rhs) {
                self.var_types.insert(name.clone(), ty);
            }
        }
        let mark = self.next_reg;
        let hint = self.var_types.get(&name).cloned();
        let r_val = if rhs.is_some() { self.compile_with_hint(arena, rhs, hint) } else { self.emit_null() };
        let slot = match self.var_slots.get(&name) {
            Some(&s) => s,
            None => {
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
            (nm.to_string(), nm)
        };
        let ty = self.fn_ret_types.get(&fc).map(|s| s.as_str()).unwrap_or(type_name);
        (!ty.is_empty()).then(|| ty.to_string())
    }

    pub(crate) fn compile_assign(&mut self, arena: &AstArena, lhs: u32, rhs: u32) -> u8 {
        let lhs_n = *arena.node(NodeId(lhs));
        let rhs_r = self.compile_node(arena, NodeId(rhs));
        match lhs_n.node_kind() {
            NodeKind::IdentExpr => {
                let name = Self::name(arena, lhs_n.lhs).to_string();
                match self.var_slots.get(&name) {
                    Some(&slot) => {
                        self.emit(Opcode::Move, slot, rhs_r, 0);
                        slot
                    }
                    None if self.is_captured_name(&name) => self.error(format!(
                        "cannot assign to `{name}` inside a lambda: closures capture variables by value"
                    )),
                    None if self.globals.contains(&name) => {
                        self.emit_global_set(&name, rhs_r);
                        rhs_r
                    }
                    None => self.error(format!("assignment to undeclared variable `{name}`")),
                }
            }
            NodeKind::MemberExpr => {
                let r_obj = self.compile_node(arena, NodeId(lhs_n.lhs));
                let r_key = self.emit_const(PilotVal::from_str(Self::name(arena, lhs_n.rhs)));
                self.emit_intrinsic("map_set", &[r_obj, r_key, rhs_r]);
                rhs_r
            }
            NodeKind::IndexExpr => {
                let r_obj = self.compile_node(arena, NodeId(lhs_n.lhs));
                let r_key = self.compile_node(arena, NodeId(lhs_n.rhs));
                self.emit_intrinsic("map_set", &[r_obj, r_key, rhs_r]);
                rhs_r
            }
            other => self.error(format!("cannot assign to {other:?}")),
        }
    }

    pub(crate) fn compile_if(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        let node = *arena.node(node_id);
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
        let e_reg = self.compile_node(arena, else_n);
        self.emit(Opcode::Move, dst, e_reg, 0);
        self.release_to(mark);
        self.patch_to_here(j_end);
        dst
    }

    /// Runs `body` inside a loop context; returns the context's jump lists.
    fn loop_body(&mut self, arena: &AstArena, body: NodeId) -> LoopCtx {
        self.loops.push(LoopCtx {
            continue_jumps: Vec::new(),
            break_jumps: Vec::new(),
            is_switch: false,
            defer_depth: self.defers.len(),
        });
        let mark = self.next_reg;
        self.compile_node(arena, body);
        self.next_reg = mark;
        self.loops.pop().unwrap_or(LoopCtx {
            continue_jumps: Vec::new(),
            break_jumps: Vec::new(),
            is_switch: false,
            defer_depth: self.defers.len(),
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

    pub(crate) fn compile_for(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        let node = *arena.node(node_id);
        let clause = (node.flags as usize).checked_sub(1).and_then(|i| arena.for_clauses.get(i).copied());
        let Some((iter_id, then_id)) = clause else {
            return self.error("malformed `for` expression".to_string());
        };
        if then_id.is_some() {
            return self.error("`for ... then` expressions are not lowered by the seed yet".to_string());
        }
        let loop_var = Self::name(arena, node.lhs).to_string();
        let iter_n = *arena.node(iter_id);
        let is_range = iter_n.node_kind() == NodeKind::BinaryExpr
            && matches!(iter_n.flags, crate::lower_expr::OP_RANGE_INCLUSIVE | crate::lower_expr::OP_RANGE_HALF_OPEN);

        // Counter, bound and loop variable live in locals so the body cannot clobber them.
        let slot = self.alloc_local_reg();
        self.var_slots.insert(loop_var.clone(), slot);
        self.reg_symbols.push((slot, node.lhs, 0));
        let r_idx = self.alloc_local_reg();
        let r_end = self.alloc_local_reg();
        let r_seq = self.alloc_local_reg();

        let mark = self.next_reg;
        if is_range {
            let r_s = self.compile_node(arena, NodeId(iter_n.lhs));
            self.emit(Opcode::Move, r_idx, r_s, 0);
            let r_e = self.compile_node(arena, NodeId(iter_n.rhs));
            self.emit(Opcode::Move, r_end, r_e, 0);
        } else {
            let r_it = self.compile_node(arena, iter_id);
            self.emit(Opcode::Move, r_seq, r_it, 0);
            let r_len = self.emit_intrinsic("list_len", &[r_seq]);
            self.emit(Opcode::Move, r_end, r_len, 0);
            let r_zero = self.emit_const(PilotVal::from_int(0));
            self.emit(Opcode::Move, r_idx, r_zero, 0);
        }
        self.next_reg = mark;

        let start = self.here();
        let r_cond = self.alloc_reg();
        let cmp = if is_range && iter_n.flags == crate::lower_expr::OP_RANGE_INCLUSIVE {
            Opcode::CmpLe
        } else {
            Opcode::CmpLt
        };
        self.emit(cmp, r_cond, r_idx, r_end);
        let jf = self.emit_branch(Opcode::JumpIfFalse, r_cond, 0);
        if is_range {
            self.emit(Opcode::Move, slot, r_idx, 0);
        } else {
            let r_item = self.emit_intrinsic("map_get", &[r_seq, r_idx]);
            self.emit(Opcode::Move, slot, r_item, 0);
        }
        let ctx = self.loop_body(arena, NodeId(node.rhs));
        for j in ctx.continue_jumps {
            self.patch_to_here(j);
        }
        let r_one = self.emit_const(PilotVal::from_int(1));
        self.emit(Opcode::Add, r_idx, r_idx, r_one);
        self.emit_jump(start);
        self.patch_to_here(jf);
        for j in ctx.break_jumps {
            self.patch_to_here(j);
        }
        self.emit_null()
    }

    pub(crate) fn compile_break(&mut self, arena: &AstArena, has_value: bool) -> u8 {
        if has_value {
            return self.error("`break` with a value is not lowered by the seed yet".to_string());
        }
        let Some(depth) = self.loops.last().map(|l| l.defer_depth) else {
            return self.error("`break` outside of a loop or switch".to_string());
        };
        self.emit_defers_above(arena, depth);
        let j = self.emit_jump(0);
        if let Some(l) = self.loops.last_mut() {
            l.break_jumps.push(j);
        }
        0
    }

    pub(crate) fn compile_continue(&mut self, arena: &AstArena, has_value: bool) -> u8 {
        if has_value {
            return self.error("`continue` with new bounds is not lowered by the seed yet".to_string());
        }
        let Some(depth) = self.loops.iter().rev().find(|l| !l.is_switch).map(|l| l.defer_depth) else {
            return self.error("`continue` outside of a loop".to_string());
        };
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
            self.compile_with_hint(arena, NodeId(value), hint)
        } else {
            self.emit_null()
        };
        // The value is computed before deferred statements run.
        let keep = self.alloc_reg();
        self.emit(Opcode::Move, keep, r_ret, 0);
        self.emit_defers_above(arena, 0);
        self.emit(Opcode::Move, 0, keep, 0);
        self.emit(Opcode::Ret, 0, 0, 0);
        0
    }
}
