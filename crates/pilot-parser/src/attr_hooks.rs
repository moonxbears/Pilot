//! Declaration attributes and airlock wrapper functions that feed the bootstrap tables.

use crate::{CVal, ParserTables};
use pilot_ast::{AstArena, NodeId, NodeKind, DECL_FLAG_COMPTIME};
use pilot_intern::SymbolId;

#[derive(Clone, Debug)]
pub(crate) enum HookArg {
    Param(usize),
    Const(CVal),
}

#[derive(Clone, Debug)]
pub(crate) struct AttrHook {
    pub(crate) hook: String,
    pub(crate) args: Vec<HookArg>,
    pub(crate) defaults: Vec<Option<CVal>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegLayout {
    pub width: i64,
    pub class: String,
}

const HOOK_REG_LAYOUT: &str = "set_register_layout";

fn cval_scalar(v: &CVal) -> Option<Result<i64, &str>> {
    match v {
        CVal::Int(n) => Some(Ok(*n)),
        CVal::Str(s) => Some(Err(s.as_str())),
        CVal::Map(_) => None,
    }
}

impl ParserTables {
    pub fn reg_layout(&self, name: &str) -> Option<&RegLayout> {
        self.reg_layouts.get(name)
    }

    pub fn call_hook(&self, name: &str) -> Option<(&str, Vec<Option<Result<i64, &str>>>, Vec<Result<usize, Result<i64, &str>>>)> {
        let h = self.attr_hooks.get(name)?;
        let defs = h.defaults.iter().map(|d| d.as_ref().and_then(cval_scalar)).collect();
        let args = h
            .args
            .iter()
            .map(|a| match a {
                HookArg::Param(i) => Some(Ok(*i)),
                HookArg::Const(v) => Some(Err(cval_scalar(v)?)),
            })
            .collect::<Option<Vec<_>>>()?;
        Some((h.hook.as_str(), defs, args))
    }

    pub(crate) fn ingest_attr_fn(&mut self, arena: &AstArena, id: NodeId) {
        let n = *arena.node(id);
        let flags = arena.decl_flags.get(id.0 as usize).copied().unwrap_or(0);
        let Some((&body_raw, param_ids)) = arena.children_slice(n.rhs, n.flags as u32).split_first() else { return };
        let mut body = NodeId(body_raw);
        while body.is_some() && arena.node(body).node_kind() == NodeKind::ExprStmt {
            body = NodeId(arena.node(body).lhs);
        }
        if body.is_none() || arena.node(body).node_kind() != NodeKind::CallExpr {
            return;
        }
        let bn = *arena.node(body);
        let call = arena.children_slice(bn.lhs, bn.rhs);
        let Some(&callee_id) = call.first() else { return };
        let cn = arena.node(NodeId(callee_id));
        if cn.node_kind() != NodeKind::IdentExpr {
            return;
        }
        let callee = arena.interner.resolve(SymbolId(cn.lhs));
        if callee != "__engine" && (flags & DECL_FLAG_COMPTIME) == 0 {
            return;
        }
        let params: Vec<&str> = param_ids.iter().map(|&p| arena.interner.resolve(SymbolId(arena.node(NodeId(p)).lhs))).collect();
        let defaults: Vec<Option<CVal>> = param_ids
            .iter()
            .map(|&p| arena.field_default(NodeId(p)).is_some().then(|| self.eval_cval(arena, arena.field_default(NodeId(p)))).flatten())
            .collect();
        let map_arg = |a: u32| -> HookArg {
            let an = arena.node(NodeId(a));
            (an.node_kind() == NodeKind::IdentExpr)
                .then(|| params.iter().position(|&nm| nm == arena.interner.resolve(SymbolId(an.lhs))))
                .flatten()
                .map_or_else(|| HookArg::Const(self.eval_cval(arena, NodeId(a)).unwrap_or(CVal::Int(0))), HookArg::Param)
        };
        let (hook, args) = if callee == "__engine" {
            let Some(CVal::Str(h)) = call.get(1).and_then(|&a| self.eval_cval(arena, NodeId(a))) else { return };
            (h, call[2..].iter().map(|&a| map_arg(a)).collect())
        } else if let Some(base) = self.attr_hooks.get(callee).cloned() {
            let passed: Vec<HookArg> = call[1..].iter().map(|&a| map_arg(a)).collect();
            let composed = base
                .args
                .into_iter()
                .map(|ba| match ba {
                    HookArg::Param(i) => passed.get(i).cloned().unwrap_or_else(|| HookArg::Const(base.defaults.get(i).and_then(Clone::clone).unwrap_or(CVal::Int(0)))),
                    HookArg::Const(v) => HookArg::Const(v),
                })
                .collect();
            (base.hook, composed)
        } else {
            return;
        };
        self.attr_hooks.insert(arena.interner.resolve(SymbolId(n.lhs)).to_string(), AttrHook { hook, args, defaults });
    }

    pub(crate) fn ingest_decl_attrs(&mut self, arena: &AstArena, id: NodeId) {
        let decl_name = CVal::Str(arena.interner.resolve(SymbolId(arena.node(id).lhs)).to_string());
        for attr in arena.attrs.iter().filter(|a| a.target == id) {
            let Some(f) = self.attr_hooks.get(arena.interner.resolve(attr.name)).cloned() else { continue };
            let attr_args = arena.children_slice(attr.args_start, attr.args_len as u32);
            let values: Vec<Option<CVal>> = f
                .args
                .iter()
                .map(|a| match a {
                    HookArg::Param(0) => Some(decl_name.clone()),
                    HookArg::Param(i) => attr_args.get(i - 1).and_then(|&x| self.eval_cval(arena, NodeId(x))).or_else(|| f.defaults.get(*i).and_then(Clone::clone)),
                    HookArg::Const(v) => Some(v.clone()),
                })
                .collect();
            if f.hook == HOOK_REG_LAYOUT {
                if let [Some(CVal::Str(ty)), Some(CVal::Int(width)), Some(CVal::Str(class))] = values.as_slice() {
                    self.reg_layouts.insert(ty.clone(), RegLayout { width: *width, class: class.clone() });
                }
            }
        }
    }
}
