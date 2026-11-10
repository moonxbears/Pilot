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
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;


/// A call whose arguments do not fit its callee: the diagnostic code and message.
type ArgError = (&'static str, String);

impl BytecodeCompiler {
    /// Records the declaration whose parameters `key` (`f`, `Owner.m` or
    /// `Owner.self`) takes.
    pub(crate) fn register_params(&mut self, key: String, decl: NodeId) {
        self.param_decls.entry(key).or_insert(decl);
    }

    /// Parameter names of a function or constructor declaration, without the
    /// receiver.
    fn decl_param_names(arena: &AstArena, decl: NodeId) -> Vec<String> {
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
            .map(|&p| Self::name(arena, arena.node(NodeId(p)).lhs).to_string())
            .filter(|p| p != receiver && p != &receiver[1..])
            .collect()
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
        for (p, slot) in params.iter().zip(slots) {
            match slot {
                Some(a) => out.push(a),
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
