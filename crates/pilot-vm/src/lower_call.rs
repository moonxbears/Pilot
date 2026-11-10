//! Call lowering: free functions, methods, struct literals and seed builtins.

use crate::instruction::{Instruction, Opcode};
use crate::{BytecodeCompiler, SeedBuiltin};
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;

impl BytecodeCompiler {
    /// Every call nets one scratch register: argument temporaries are
    /// released once the call has been emitted.
    pub(crate) fn compile_call(&mut self, arena: &AstArena, arg_start: u32, arg_len: u32) -> u8 {
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r = self.compile_call_value(arena, arg_start, arg_len);
        self.emit(Opcode::Move, dst, r, 0);
        self.release_to(mark);
        dst
    }

    fn compile_call_value(&mut self, arena: &AstArena, arg_start: u32, arg_len: u32) -> u8 {
        let (arg_start, arg_len) = (arg_start as usize, arg_len as usize);
        if arg_len == 0 || arg_start >= arena.extra_children.len() {
            return self.error("malformed call".to_string());
        }
        let ids: Vec<NodeId> = arena.extra_children[arg_start..arg_start + arg_len]
            .iter()
            .map(|&c| NodeId(c))
            .collect();
        self.compile_call_ids(arena, &ids)
    }

    /// Calls `ids[0]` with the arguments `ids[1..]`.
    pub(crate) fn compile_call_ids(&mut self, arena: &AstArena, ids: &[NodeId]) -> u8 {
        if Self::has_named_args(arena, &ids[1..]) {
            return match self.order_named_args(arena, ids) {
                Ok(ordered) if Self::has_named_args(arena, &ordered[1..]) => self.compile_call_positional(arena, &ordered),
                Ok(ordered) => self.compile_call_ids(arena, &ordered),
                Err(message) => self.error(message),
            };
        }
        self.compile_call_positional(arena, ids)
    }

    /// Calls `ids[0]` with `ids[1..]`, which are positional except for a
    /// memberwise struct construction.
    fn compile_call_positional(&mut self, arena: &AstArena, ids: &[NodeId]) -> u8 {
        let callee = *arena.node(ids[0]);
        match callee.node_kind() {
            NodeKind::IdentExpr | NodeKind::TypeNamed => {
                let name = Self::name(arena, callee.lhs).to_string();
                self.compile_named_call(arena, &name, &ids[1..])
            }
            NodeKind::MemberExpr => self.compile_method_call(arena, ids[0], &ids[1..]),
            _ => {
                let r_callee = self.compile_node(arena, ids[0]);
                let regs = self.compile_args(arena, &ids[1..]);
                self.emit_closure_call(r_callee, &regs)
            }
        }
    }

    pub(crate) fn compile_args(&mut self, arena: &AstArena, args: &[NodeId]) -> Vec<u8> {
        args.iter().map(|&a| self.compile_node(arena, a)).collect()
    }

    /// Emits a call to `full`, falling back to `bare` when `full` is unknown.
    pub(crate) fn emit_user_call(&mut self, full: String, bare: String, args: &[u8]) -> u8 {
        self.stage_args(args);
        let dst = self.alloc_reg();
        let entry = self
            .fn_entries
            .get(&full)
            .or_else(|| self.fn_entries.get(&bare))
            .map(|&(e, _)| e);
        match entry {
            Some(e) => {
                self.push_ins(Instruction::encode_wide(Opcode::Call, dst, e));
            }
            None => {
                let idx = self.emit(Opcode::Call, dst, 0, 0);
                self.call_patches.push((idx, full, bare, self.cur_span));
            }
        }
        dst
    }

    fn compile_named_call(&mut self, arena: &AstArena, name: &str, args: &[NodeId]) -> u8 {
        match SeedBuiltin::function(name) {
            Some(SeedBuiltin::SizeOf) => {
                return self.emit_const(PilotVal::from_int(std::mem::size_of::<usize>() as i64));
            }
            Some(SeedBuiltin::Assert) => {
                let Some(&cond) = args.first() else {
                    return self.error("assert expects a condition".to_string());
                };
                let cn = *arena.node(cond);
                let r_c = if cn.node_kind() == NodeKind::TupleExpr {
                    match arena.children_slice(cn.lhs, cn.rhs).first() {
                        Some(&first) => self.compile_node(arena, NodeId(first)),
                        None => return self.error("assert expects a condition".to_string()),
                    }
                } else {
                    self.compile_node(arena, cond)
                };
                return self.emit_intrinsic("rt_assert", &[r_c]);
            }
            Some(SeedBuiltin::Print) => {
                let regs = self.compile_args(arena, args);
                return self.emit_intrinsic("rt_print", &regs);
            }
            Some(SeedBuiltin::NameOf) => {
                let Some(&arg) = args.first() else {
                    return self.error("nameof expects one argument".to_string());
                };
                let an = arena.node(arg);
                if an.node_kind() == NodeKind::IdentExpr {
                    return self.emit_const(PilotVal::from_str(Self::name(arena, an.lhs)));
                }
                return self.compile_node(arena, arg);
            }
            Some(SeedBuiltin::Engine) => {
                let Some((&hook, rest)) = args.split_first() else {
                    return self.error("__engine expects an intrinsic name".to_string());
                };
                let r_hook = self.compile_node(arena, hook);
                let regs = self.compile_args(arena, rest);
                self.stage_args(&regs);
                let dst = self.alloc_reg();
                self.emit(Opcode::EngineIntrinsic, dst, r_hook, regs.len() as u8);
                return dst;
            }
            Some(SeedBuiltin::Method(_)) | None => {}
        }
        // `Name { field: value, ... }` struct literal.
        if args.len() == 1
            && arena.node(args[0]).node_kind() == NodeKind::MapExpr
            && !self.fn_entries.contains_key(name)
        {
            return self.compile_struct_literal(arena, name, args[0]);
        }
        // Calling a variable or captured variable calls the function value it holds.
        if let Some(r_callee) = self.var_slots.get(name).copied().or_else(|| self.capture_read(name)) {
            let regs = self.compile_args(arena, args);
            return self.emit_closure_call(r_callee, &regs);
        }
        if let Some(r) = self.compile_implicit_self_call(arena, name, args) {
            return r;
        }
        if !self.fn_entries.contains_key(name) && !self.known_fns.contains_key(name) {
            let hint = self.type_hint.clone();
            if let Some(full) = self.payload_variant(name, hint.as_deref()) {
                return self.compile_variant_ctor(arena, &full, args);
            }
        }
        if args.len() == 1 && !self.fn_entries.contains_key(name) && !self.known_fns.contains_key(name) {
            if let Some(layout) = pilot_parser::bootstrap_tables().reg_layout(name).cloned() {
                let r = self.compile_node(arena, args[0]);
                return self.emit_conversion(r, &layout);
            }
        }
        if !self.fn_entries.contains_key(name) && !self.known_fns.contains_key(name) {
            if let Some(r) = self.compile_ctor_call(arena, name, args) {
                return r;
            }
        }
        let regs = self.compile_args(arena, args);
        self.emit_user_call(name.to_string(), name.to_string(), &regs)
    }

    fn compile_method_call(&mut self, arena: &AstArena, callee_id: NodeId, args: &[NodeId]) -> u8 {
        let callee = *arena.node(callee_id);
        let ln = *arena.node(NodeId(callee.lhs));
        let r_nm = Self::name(arena, callee.rhs).to_string();
        let lhs_is_name = matches!(ln.node_kind(), NodeKind::IdentExpr | NodeKind::TypeNamed);
        let raw_l = if lhs_is_name { Self::name(arena, ln.lhs).to_string() } else { String::new() };
        let is_var = lhs_is_name && self.var_slots.contains_key(&raw_l);
        let l_nm = if is_var {
            self.var_types.get(&raw_l).cloned().unwrap_or_else(|| raw_l.clone())
        } else {
            raw_l.clone()
        };
        let full = format!("{l_nm}.{r_nm}");
        let known = self.fn_entries.contains_key(&full);
        let receiver_is_value = is_var || (!lhs_is_name && !known);
        if receiver_is_value || (!known && SeedBuiltin::method(&r_nm).is_some()) {
            let r_recv = self.compile_node(arena, NodeId(callee.lhs));
            return self.emit_method_call(arena, r_recv, &l_nm, &r_nm, args);
        }
        if !known && self.variant_payloads.contains_key(&full) {
            return self.compile_variant_ctor(arena, &full, args);
        }
        let regs = self.compile_args(arena, args);
        self.emit_user_call(full, r_nm, &regs)
    }

    /// Converts the value in `r` to a primitive's register layout: `T(x)` and
    /// `x as T` for a type `T` declared with `[Bits(..)]`.
    pub(crate) fn emit_conversion(&mut self, r: u8, layout: &pilot_parser::RegLayout) -> u8 {
        let mark = self.next_reg;
        let r_class = self.emit_const(PilotVal::from_str(&layout.class));
        let r_width = self.emit_const(PilotVal::from_int(layout.width));
        let dst = self.emit_intrinsic("num_convert", &[r, r_class, r_width]);
        if dst >= mark {
            self.emit(Opcode::Move, mark, dst, 0);
        }
        self.release_to(mark + 1);
        mark
    }
}
