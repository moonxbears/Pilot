//! Declaration attributes that feed the bootstrap tables.
//!
//! An attribute such as `[Bits(Cfg.Reg.b64, Cfg.Reg.simd_f)]` is a call to the
//! Pilot comptime function of that name, declared in `stdlib/core/core.pilot`
//! as a single engine call:
//!
//! ```text
//! comptime fn Bits(target_type, width, reg_class) =>
//!     __engine(Cfg.Hooks.reg_layout, target_type, width, reg_class);
//! ```
//!
//! Ingesting such a function records which engine hook it reaches and where
//! each hook argument comes from. Ingesting a declaration then runs every
//! attribute on it through its function, with the declaration's name as the
//! first parameter. The seed acts on the hooks it implements; the others are
//! ignored here.

use crate::{CVal, ParserTables};
use pilot_ast::{AstArena, NodeId, NodeKind, DECL_FLAG_COMPTIME};
use pilot_intern::SymbolId;

/// Where an engine-hook argument comes from.
#[derive(Clone, Debug)]
pub(crate) enum HookArg {
    /// The attribute function's parameter at this index.
    Param(usize),
    /// A value fixed by the function body.
    Const(CVal),
}

/// An attribute function: the engine hook it calls and its arguments.
#[derive(Clone, Debug)]
pub(crate) struct AttrHook {
    hook: String,
    args: Vec<HookArg>,
}

/// The register layout a type declares through `set_register_layout`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegLayout {
    /// Width in bits.
    pub width: i64,
    /// Register class, as the descriptor names it (`gpr_signed`, `simd_float`, ...).
    pub class: String,
}

/// Engine hook that sets a type's register layout.
const HOOK_REG_LAYOUT: &str = "set_register_layout";

impl ParserTables {
    /// The register layout declared for type `name`, if any.
    pub fn reg_layout(&self, name: &str) -> Option<&RegLayout> {
        self.reg_layouts.get(name)
    }

    /// Records a comptime function whose body is one engine call.
    pub(crate) fn ingest_attr_fn(&mut self, arena: &AstArena, id: NodeId) {
        let n = *arena.node(id);
        let flags = arena.decl_flags.get(id.0 as usize).copied().unwrap_or(0);
        if flags & DECL_FLAG_COMPTIME == 0 {
            return;
        }
        let ch = arena.children_slice(n.rhs, n.flags as u32);
        let Some((&body, params)) = ch.split_first() else {
            return;
        };
        let params: Vec<&str> = params
            .iter()
            .map(|&p| arena.interner.resolve(SymbolId(arena.node(NodeId(p)).lhs)))
            .collect();
        let mut body = NodeId(body);
        while body.is_some() && arena.node(body).node_kind() == NodeKind::ExprStmt {
            body = NodeId(arena.node(body).lhs);
        }
        if body.is_none() || arena.node(body).node_kind() != NodeKind::CallExpr {
            return;
        }
        let bn = *arena.node(body);
        let call = arena.children_slice(bn.lhs, bn.rhs);
        let is_engine = call.first().is_some_and(|&c| {
            let cn = arena.node(NodeId(c));
            cn.node_kind() == NodeKind::IdentExpr && arena.interner.resolve(SymbolId(cn.lhs)) == "__engine"
        });
        if !is_engine || call.len() < 2 {
            return;
        }
        let Some(CVal::Str(hook)) = self.eval_cval(arena, NodeId(call[1])) else {
            return;
        };
        let args = call[2..]
            .iter()
            .map(|&a| {
                let an = arena.node(NodeId(a));
                let param = (an.node_kind() == NodeKind::IdentExpr)
                    .then(|| params.iter().position(|&p| p == arena.interner.resolve(SymbolId(an.lhs))))
                    .flatten();
                match param {
                    Some(i) => HookArg::Param(i),
                    None => HookArg::Const(self.eval_cval(arena, NodeId(a)).unwrap_or(CVal::Int(0))),
                }
            })
            .collect();
        let name = arena.interner.resolve(SymbolId(n.lhs)).to_string();
        self.attr_hooks.insert(name, AttrHook { hook, args });
    }

    /// Runs each attribute on declaration `id` through its attribute function.
    pub(crate) fn ingest_decl_attrs(&mut self, arena: &AstArena, id: NodeId) {
        let decl_name = CVal::Str(arena.interner.resolve(SymbolId(arena.node(id).lhs)).to_string());
        for attr in arena.attrs.iter().filter(|a| a.target == id) {
            let Some(f) = self.attr_hooks.get(arena.interner.resolve(attr.name)).cloned() else {
                continue;
            };
            let attr_args = arena.children_slice(attr.args_start, attr.args_len as u32);
            let param = |i: usize| match i {
                0 => Some(decl_name.clone()),
                _ => attr_args.get(i - 1).and_then(|&a| self.eval_cval(arena, NodeId(a))),
            };
            let values: Vec<Option<CVal>> = f
                .args
                .iter()
                .map(|a| match a {
                    HookArg::Param(i) => param(*i),
                    HookArg::Const(v) => Some(v.clone()),
                })
                .collect();
            self.apply_hook(&f.hook, &values);
        }
    }

    fn apply_hook(&mut self, hook: &str, values: &[Option<CVal>]) {
        if hook == HOOK_REG_LAYOUT {
            if let [Some(CVal::Str(ty)), Some(CVal::Int(width)), Some(CVal::Str(class))] = values {
                let layout = RegLayout {
                    width: *width,
                    class: class.clone(),
                };
                self.reg_layouts.insert(ty.clone(), layout);
            }
        }
    }
}
