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
        let callee = *arena.node(ids[0]);
        match callee.node_kind() {
            NodeKind::IdentExpr | NodeKind::TypeNamed => {
                let name = Self::name(arena, callee.lhs).to_string();
                self.compile_named_call(arena, &name, &ids[1..])
            }
            NodeKind::MemberExpr => self.compile_method_call(arena, ids[0], &ids[1..]),
            _ => self.error("calling a computed value is not lowered by the seed yet".to_string()),
        }
    }

    fn compile_args(&mut self, arena: &AstArena, args: &[NodeId]) -> Vec<u8> {
        args.iter().map(|&a| self.compile_node(arena, a)).collect()
    }

    /// Emits a call to `full`, falling back to `bare` when `full` is unknown.
    fn emit_user_call(&mut self, full: String, bare: String, args: &[u8]) -> u8 {
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
            let mn = *arena.node(args[0]);
            return self.compile_map_literal(arena, mn.lhs, mn.rhs);
        }
        if self.var_slots.contains_key(name) {
            return self.error(format!(
                "`{name}` is a variable; calling values (closures) is not lowered by the seed yet"
            ));
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
        if !known {
            if let Some(hook) = SeedBuiltin::method(&r_nm) {
                let mut regs = vec![self.compile_node(arena, NodeId(callee.lhs))];
                regs.extend(self.compile_args(arena, args));
                return self.emit_intrinsic(hook, &regs);
            }
        }
        let mut regs = if receiver_is_value {
            vec![self.compile_node(arena, NodeId(callee.lhs))]
        } else {
            Vec::new()
        };
        regs.extend(self.compile_args(arena, args));
        self.emit_user_call(full, r_nm, &regs)
    }
}
