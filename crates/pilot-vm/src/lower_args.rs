//! Named arguments and memberwise construction.
//!
//! `f(b: 2, a: 1)` passes each value to the parameter it names, whatever the
//! order. Positional arguments come first and fill the parameters in order;
//! named ones fill the rest. The callee must be a function, method or
//! constructor whose declaration the compiler can see, so it knows the
//! parameter names; anywhere else a named argument is a compile error.
//!
//! A struct without a `self(..)` constructor is built memberwise:
//! `Point(1, 2)` or `Point(y: 2, x: 1)` sets its fields in declaration order
//! or by name. A field left out takes its default, and leaving out a field
//! that has none is a compile error.

use crate::BytecodeCompiler;
use crate::instruction::Opcode;
use pilot_ast::{AstArena, NodeId, NodeKind, PARAM_FLAG_MUT};
use pilot_rt::PilotVal;


/// A call whose arguments do not fit its callee: the diagnostic code and message.
type ArgError = (&'static str, String);

impl BytecodeCompiler {
    /// Records the declaration whose parameters `key` (`f`, `Owner.m` or
    /// `Owner.self`) takes.
    pub(crate) fn register_params(&mut self, key: String, decl: NodeId) {
        self.param_decls.entry(key).or_insert(decl);
    }

    /// Parameter declarations of a function or constructor declaration,
    /// without the receiver.
    fn decl_params(arena: &AstArena, decl: NodeId) -> Vec<NodeId> {
        let dn = arena.node(decl);
        let children = match dn.node_kind() {
            NodeKind::FnDecl => arena.children_slice(dn.rhs, dn.flags as u32),
            NodeKind::ConstructorDecl => arena.children_slice(dn.lhs, dn.rhs),
            _ => return Vec::new(),
        };
        let receiver = crate::RECEIVER;
        children
            .iter()
            .skip(1)
            .map(|&p| NodeId(p))
            .filter(|&p| {
                let name = Self::name(arena, arena.node(p).lhs);
                name != receiver && name != &receiver[1..]
            })
            .collect()
    }

    /// Whether parameter declaration `p` lends the caller's storage for
    /// changing: written with a word registered `binding_mutable` (`var ref n`).
    fn lends_for_change(arena: &AstArena, p: NodeId) -> bool {
        arena.node(p).flags & PARAM_FLAG_MUT != 0
    }

    /// Records the slots of the `var ref` parameters of `decl`, whose
    /// parameters are bound: each `Ret` then hands their final values back
    /// to the caller (`SetOut`).
    pub(crate) fn bind_ret_outs(&mut self, arena: &AstArena, decl: NodeId) {
        self.ret_outs.clear();
        for (i, p) in Self::decl_params(arena, decl).into_iter().enumerate() {
            if !Self::lends_for_change(arena, p) {
                continue;
            }
            let name = Self::name(arena, arena.node(p).lhs);
            if let Some(&slot) = self.var_slots.get(name) {
                self.ret_outs.push((i as u8, slot));
            }
        }
    }

    /// After a call to `key`, writes the final value of each `var ref`
    /// parameter back to the variable passed for it (`GetOut`). An argument
    /// that is not a variable (a field, an element) shares the caller's
    /// value already when it is a heap value, and has no storage to write
    /// to when it is a temporary.
    pub(crate) fn emit_out_writebacks(&mut self, arena: &AstArena, key: &str, args: &[NodeId]) {
        let Some(&decl) = self.param_decls.get(key) else { return };
        for (i, p) in Self::decl_params(arena, decl).into_iter().enumerate() {
            if !Self::lends_for_change(arena, p) {
                continue;
            }
            let Some(&a) = args.get(i) else { continue };
            let an = arena.node(a);
            if an.node_kind() != NodeKind::IdentExpr {
                continue;
            }
            let name = Self::name(arena, an.lhs).to_string();
            if let Some(&slot) = self.var_slots.get(&name) {
                self.emit(Opcode::GetOut, slot, i as u8, 0);
            } else if self.globals.contains(&name) && !self.is_captured_name(&name) {
                let mark = self.next_reg;
                let r = self.alloc_reg();
                self.emit(Opcode::GetOut, r, i as u8, 0);
                self.emit_global_set(&name, r);
                self.release_to(mark);
            }
        }
    }

    /// Parameter names of a function or constructor declaration, without the
    /// receiver.
    fn decl_param_names(arena: &AstArena, decl: NodeId) -> Vec<String> {
        Self::decl_params(arena, decl)
            .into_iter()
            .map(|p| Self::name(arena, arena.node(p).lhs).to_string())
            .collect()
    }

    /// Compiles positional `args` of a call to `key` (`f` or `Owner.m`), each
    /// with its parameter's declared type as the expected type, so a bare
    /// `ok(x)` or `::Variant` argument resolves against it.
    pub(crate) fn compile_args_for(&mut self, arena: &AstArena, key: &str, args: &[NodeId]) -> Vec<u8> {
        let hints: Vec<Option<String>> = match self.param_decls.get(key) {
            Some(&decl) => Self::decl_params(arena, decl)
                .into_iter()
                .map(|p| Self::annot_name(arena, p))
                .collect(),
            None => Vec::new(),
        };
        // `f(a, *xs)`: the one spread argument fills the parameters the
        // others leave, one item each.
        let is_spread = |a: NodeId| {
            let n = arena.node(a);
            n.node_kind() == NodeKind::UnaryExpr && n.flags == pilot_ast::UNARY_OP_SPLAT
        };
        let spreads = args.iter().filter(|&&a| is_spread(a)).count();
        let known = self.param_decls.contains_key(key);
        let take = (hints.len() + spreads).saturating_sub(args.len());
        let mut regs = Vec::new();
        for &a in args {
            if is_spread(a) && spreads == 1 && known {
                let r_seq = self.compile_node(arena, NodeId(arena.node(a).lhs));
                self.emit_spread(r_seq, take, key, &mut regs);
                continue;
            }
            let hint = hints.get(regs.len()).cloned().flatten();
            regs.push(self.compile_with_hint(arena, a, hint));
        }
        regs
    }

    /// The `take` items of the sequence in `r_seq`, each in a register of
    /// its own appended to `regs`; the program stops when the sequence
    /// holds another number of items.
    fn emit_spread(&mut self, r_seq: u8, take: usize, callee: &str, regs: &mut Vec<u8>) {
        let mark = self.next_reg;
        let r_len = self.emit_intrinsic("list_len", &[r_seq]);
        let r_want = self.emit_const(PilotVal::from_int(take as i64));
        let r_ok = self.alloc_reg();
        self.emit(Opcode::CmpEq, r_ok, r_len, r_want);
        let ok = self.emit_branch(Opcode::JumpIfTrue, r_ok, 0);
        self.emit_trap(format!(
            "CALL001: `{callee}` takes {take} argument(s) from the sequence spread with `*`, which holds another number"
        ));
        self.patch_to_here(ok);
        self.release_to(mark);
        for k in 0..take {
            let r_item = self.alloc_reg();
            let inner = self.next_reg;
            let r_k = self.emit_const(PilotVal::from_int(k as i64));
            let r_v = self.emit_intrinsic("map_get", &[r_seq, r_k]);
            self.emit(Opcode::Move, r_item, r_v, 0);
            self.release_to(inner);
            regs.push(r_item);
        }
    }

    /// Whether any of `args` is named.
    pub(crate) fn has_named_args(arena: &AstArena, args: &[NodeId]) -> bool {
        args.iter().any(|&a| arena.node(a).node_kind() == NodeKind::CallArg)
    }

    /// Matches `args` to `params`: the argument each parameter receives, or
    /// `None` where none was given.
    pub(crate) fn bind_args(
        arena: &AstArena,
        callee: &str,
        params: &[String],
        args: &[NodeId],
    ) -> Result<Vec<Option<NodeId>>, ArgError> {
        let mut slots: Vec<Option<NodeId>> = vec![None; params.len()];
        let mut next = 0;
        let mut named = false;
        for &a in args {
            let an = arena.node(a);
            if an.node_kind() == NodeKind::CallArg {
                named = true;
                let name = Self::name(arena, an.lhs);
                let Some(i) = params.iter().position(|p| p == name) else {
                    return Err((
                        crate::codes::NAMED_ARG,
                        format!("`{callee}` has no parameter `{name}`"),
                    ));
                };
                if slots[i].is_some() {
                    return Err((
                        crate::codes::NAMED_ARG,
                        format!("`{callee}` is given `{name}` twice"),
                    ));
                }
                slots[i] = Some(NodeId(an.rhs));
                continue;
            }
            if named {
                return Err((
                    crate::codes::NAMED_ARG,
                    format!("in a call to `{callee}`, a positional argument follows a named one"),
                ));
            }
            if next >= params.len() {
                return Err((
                    crate::codes::ARG_COUNT,
                    format!("`{callee}` takes {} argument(s), but more were given", params.len()),
                ));
            }
            slots[next] = Some(a);
            next += 1;
        }
        Ok(slots)
    }

    /// The declaration a call through `callee` reaches, when it is known.
    fn named_call_key(&self, arena: &AstArena, callee: NodeId) -> Option<String> {
        let cn = arena.node(callee);
        match cn.node_kind() {
            NodeKind::IdentExpr | NodeKind::TypeNamed => {
                let name = Self::name(arena, cn.lhs);
                if self.structs.contains_key(name) {
                    return Some(format!("{name}.{}", &crate::RECEIVER[1..]));
                }
                Some(name.to_string())
            }
            NodeKind::MemberExpr => {
                let ln = arena.node(NodeId(cn.lhs));
                let owner = if matches!(ln.node_kind(), NodeKind::IdentExpr | NodeKind::TypeNamed) {
                    let raw = Self::name(arena, ln.lhs);
                    self.var_types.get(raw).cloned().unwrap_or_else(|| raw.to_string())
                } else {
                    self.static_type(arena, NodeId(cn.lhs))?
                };
                Some(format!("{owner}.{}", Self::name(arena, cn.rhs)))
            }
            _ => None,
        }
    }

    /// Puts the named arguments of the call `ids` (callee, then arguments)
    /// in parameter order. A struct without a constructor keeps its
    /// arguments, which memberwise construction binds to fields.
    pub(crate) fn order_named_args(&self, arena: &AstArena, ids: &[NodeId]) -> Result<Vec<NodeId>, ArgError> {
        let callee = ids[0];
        let cn = arena.node(callee);
        if matches!(cn.node_kind(), NodeKind::IdentExpr | NodeKind::TypeNamed) {
            let name = Self::name(arena, cn.lhs);
            if self.structs.get(name).is_some_and(|s| s.ctor.is_none()) {
                return Ok(ids.to_vec());
            }
        }
        let key = self.named_call_key(arena, callee);
        let Some(decl) = key.as_ref().and_then(|k| self.param_decls.get(k)) else {
            return Err((
                crate::codes::SEED,
                format!(
                    "named arguments need a callee whose parameters are known; `{}` is not one",
                    key.unwrap_or_else(|| "this call".to_string())
                ),
            ));
        };
        let callee_name = key.clone().unwrap_or_default();
        let params = Self::decl_param_names(arena, *decl);
        let slots = Self::bind_args(arena, &callee_name, &params, &ids[1..])?;
        let mut out = vec![callee];
        let decl_params = Self::decl_params(arena, *decl);
        for ((p, slot), pd) in params.iter().zip(slots).zip(decl_params) {
            match slot {
                Some(a) => out.push(a),
                None if arena.field_default(pd).is_some() => out.push(arena.field_default(pd)),
                None => {
                    return Err((
                        crate::codes::ARG_COUNT,
                        format!("`{callee_name}` is missing argument `{p}`"),
                    ))
                }
            }
        }
        Ok(out)
    }

    /// The call `ids` (callee, then positional arguments) with the defaults
    /// of the parameters it leaves out at the end appended, when the callee's
    /// parameters are known. A call with a spread argument is left as it is.
    /// A method call whose receiver's type is unknown, to a method that some
    /// type declares with a default the call would need, is an error.
    pub(crate) fn fill_defaults(&self, arena: &AstArena, ids: &[NodeId]) -> Result<Vec<NodeId>, ArgError> {
        let spread = ids[1..].iter().any(|&a| {
            let n = arena.node(a);
            n.node_kind() == NodeKind::UnaryExpr && n.flags == pilot_ast::UNARY_OP_SPLAT
        });
        if spread {
            return Ok(ids.to_vec());
        }
        let given = ids.len() - 1;
        let decl = self
            .named_call_key(arena, ids[0])
            .and_then(|k| self.param_decls.get(&k).copied())
            .or_else(|| self.planned_call_decl(arena, ids[0]));
        let Some(decl) = decl else {
            return self.unknown_callee_defaults(arena, ids[0], given).map(|()| ids.to_vec());
        };
        let mut out = ids.to_vec();
        for &p in Self::decl_params(arena, decl).iter().skip(given) {
            let value = arena.field_default(p);
            if value.is_none() {
                break;
            }
            out.push(value);
        }
        Ok(out)
    }

    /// The declaration a method call reaches through the receiver type the
    /// checker inferred (the run plan), when the seed does not know it.
    fn planned_call_decl(&self, arena: &AstArena, callee: NodeId) -> Option<NodeId> {
        let cn = arena.node(callee);
        if cn.node_kind() != NodeKind::MemberExpr {
            return None;
        }
        let owner = self.planned_receiver(arena, NodeId(cn.lhs))?;
        let key = format!("{owner}.{}", Self::name(arena, cn.rhs));
        self.param_decls.get(&key).copied()
    }

    /// An error when `callee` is a method call the seed cannot place and a
    /// type declares that method with a default the `given` arguments
    /// would leave to it.
    fn unknown_callee_defaults(&self, arena: &AstArena, callee: NodeId, given: usize) -> Result<(), ArgError> {
        let cn = arena.node(callee);
        if cn.node_kind() != NodeKind::MemberExpr {
            return Ok(());
        }
        let method = Self::name(arena, cn.rhs);
        let owners = self.method_owners.get(method).map(Vec::as_slice).unwrap_or(&[]);
        let needs_default = owners.iter().any(|(owner, _)| {
            self.param_decls.get(&format!("{owner}.{method}")).is_some_and(|&d| {
                Self::decl_params(arena, d)
                    .get(given)
                    .is_some_and(|&p| arena.field_default(p).is_some())
            })
        });
        if !needs_default {
            return Ok(());
        }
        Err((
            crate::codes::SEED,
            format!("the seed cannot tell which `{method}` this call reaches, so it cannot fill its default arguments; pass every argument"),
        ))
    }

    /// `Name(args)` for a struct without a constructor: sets each field from
    /// the argument bound to it and the rest from their defaults.
    pub(crate) fn compile_memberwise(&mut self, arena: &AstArena, name: &str, args: &[NodeId]) -> u8 {
        let fields = self.struct_fields(name);
        let names: Vec<String> = fields.iter().map(|(f, _)| f.clone()).collect();
        let slots = match Self::bind_args(arena, name, &names, args) {
            Ok(s) => s,
            Err((code, message)) => return self.error(code, message),
        };
        if let Some((f, _)) = fields.iter().zip(&slots).find(|((_, d), s)| s.is_none() && d.is_none()).map(|(f, _)| f) {
            return self.error(
                crate::codes::MISSING_FIELD,
                format!("`{name}(..)` is missing field `{f}`, which has no default"),
            );
        }
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r_obj = self.emit_new_record(name);
        let given: Vec<String> = names
            .iter()
            .zip(&slots)
            .filter(|(_, s)| s.is_some())
            .map(|(n, _)| n.clone())
            .collect();
        self.emit_field_defaults(arena, name, &given, r_obj);
        for (field, slot) in names.iter().zip(&slots) {
            let Some(a) = *slot else { continue };
            let m = self.next_reg;
            let r_v = self.compile_node(arena, a);
            let ty = self.field_type(name, field);
            let r_v = self.convert_for_binding(r_v, ty.as_deref());
            let r_k = self.emit_const(PilotVal::from_str(field));
            self.emit_intrinsic("map_set", &[r_obj, r_k, r_v]);
            self.release_to(m);
        }
        self.emit(crate::instruction::Opcode::Move, dst, r_obj, 0);
        self.release_to(mark);
        dst
    }

    /// A named argument outside a call that can bind it.
    pub(crate) fn compile_stray_named_arg(&mut self, arena: &AstArena, sym: u32) -> u8 {
        let name = Self::name(arena, sym).to_string();
        self.error(
            crate::codes::SEED,
            format!(
                "named argument `{name}` can only be passed to a function, method or struct whose parameters are known"
            ),
        )
    }
}
