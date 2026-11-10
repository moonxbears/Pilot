//! Call lowering: free functions, methods, struct literals and seed builtins.

use crate::instruction::{Instruction, Opcode};
use crate::lower_native::native;
use crate::BytecodeCompiler;
use pilot_parser::NativeKind;
use pilot_ast::{AstArena, AstNode, CallQuoteKind, NodeId, NodeKind};
use pilot_rt::PilotVal;

/// Where a call-quote mode stops until its carrier values exist.
const QUOTED_CALL_SEAM: &str = "SEAM001: unconnected seam `vm.call_quote`: \
    call-quote modes build their carrier values once the type checker exists (F4)";

impl BytecodeCompiler {
    /// True for `f!(..)`, `sig f(..)` and the other call-quote modes: the
    /// parser records the mode in the call node's flags.
    pub(crate) fn is_quoted_call(node: &AstNode) -> bool {
        node.flags != CallQuoteKind::None as u16
    }

    /// Lowers a call-quote mode to a seam fault instead of a plain call.
    pub(crate) fn compile_quoted_call(&mut self) -> u8 {
        self.emit_trap(QUOTED_CALL_SEAM.to_string())
    }

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
        self.check_exported(&full);
        if let Some((owner, _)) = full.rsplit_once('.') {
            self.check_exported(owner);
        }
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

    /// `a op b` for an infix operator a Pilot function implements
    /// (`pilot_parser::OperatorFn`), or `None`.
    pub(crate) fn compile_operator_fn(
        &mut self,
        symbol: &str,
        arena: &AstArena,
        lhs: NodeId,
        rhs: NodeId,
    ) -> Option<u8> {
        let function = pilot_parser::bootstrap_tables().operator_fn(symbol)?.function.clone();
        let mark = self.next_reg;
        let r_l = self.compile_node(arena, lhs);
        let r_r = self.compile_node(arena, rhs);
        let r = self.emit_user_call(function.clone(), function, &[r_l, r_r]);
        self.emit(Opcode::Move, mark, r, 0);
        self.release_to(mark + 1);
        Some(mark)
    }

    fn compile_named_call(&mut self, arena: &AstArena, name: &str, args: &[NodeId]) -> u8 {
        // A name the program declares wins over a native function. The seed
        // token and receptors such as `nameof` are syntax and always win.
        let syntax =
            name == crate::ENGINE_TOKEN || pilot_parser::bootstrap_tables().quotes_symbol(name);
        let declared = self.fn_entries.contains_key(name)
            || self.known_fns.contains_key(name)
            || self.lazy_decls.contains_key(name)
            || self.structs.contains_key(name)
            || self.var_slots.contains_key(name);
        if syntax || !declared {
            if let Some(r) = self.compile_native_call(arena, name, args) {
                return r;
            }
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
        // So does calling a module-level binding (`let f = (x) => ..;`).
        if self.globals.contains(name) && !self.fn_entries.contains_key(name) && !self.known_fns.contains_key(name) {
            let r_callee = self.emit_global_get(name);
            let regs = self.compile_args(arena, args);
            return self.emit_closure_call(r_callee, &regs);
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
        // A local or module-level binding is a value, not a type name.
        let is_var = lhs_is_name && (self.var_slots.contains_key(&raw_l) || self.globals.contains(&raw_l));
        let l_nm = if is_var {
            self.var_types.get(&raw_l).cloned().unwrap_or_else(|| raw_l.clone())
        } else {
            raw_l.clone()
        };
        let full = format!("{l_nm}.{r_nm}");
        // `Type.method` names a method the type declares, compiled or not yet
        // (a method of an imported unit is compiled when first called).
        let known = self.fn_entries.contains_key(&full) || (!is_var && self.declares(&l_nm, &r_nm).is_some());
        let receiver_is_value = is_var || (!lhs_is_name && !known);
        if receiver_is_value || (!known && native(NativeKind::Method, &r_nm).is_some()) {
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
