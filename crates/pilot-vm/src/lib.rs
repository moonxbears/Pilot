//! Register-Based `PilVM` v3, `.pilc` v3 Zero-Copy Format & Debug-Span Compiler (`pilot-vm`).

pub mod instruction;
pub mod pilc;
pub mod vm;

pub use instruction::{Instruction, Opcode};
pub use pilc::{PilcBinary, PILC_V3_MAGIC, PILC_V3_VERSION};
pub use vm::{EngineIntrinsicHandler, PilVirtualMachine, VmHook};
pub mod compiler { pub use super::BytecodeCompiler; }

use std::collections::HashMap;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_rt::PilotVal;
use pilot_span::ByteSpan;

#[derive(Clone, Debug)]
enum CMap { Val(PilotVal), Map(HashMap<String, CMap>) }

#[derive(Default)]
pub struct BytecodeCompiler {
    pub instructions: Vec<Instruction>, pub constants: Vec<PilotVal>,
    pub pc_spans: Vec<ByteSpan>, pub reg_symbols: Vec<(u8, u32, u32)>,
    var_slots: HashMap<String, u8>, fn_entries: HashMap<String, (u16, Vec<String>)>,
    call_patches: Vec<(usize, String)>, const_maps: HashMap<String, CMap>,
    loop_stack: Vec<(u16, Vec<usize>)>, cur_span: ByteSpan, next_local_reg: u8, next_reg: u8,
}

impl BytecodeCompiler {
    pub fn new() -> Self { Self { next_local_reg: 17, next_reg: 96, ..Self::default() } }

    pub fn compile(arena: &AstArena, root: NodeId) -> PilcBinary {
        let mut c = Self::new();
        let last = if root.is_some() { c.compile_node(arena, root) } else { 0 };
        c.emit(Opcode::Move, 0, last, 0); c.emit(Opcode::Ret, 0, 0, 0); c.emit(Opcode::Halt, 0, 0, 0);
        for &(idx, ref name) in &c.call_patches {
            let dst = c.instructions[idx].dst();
            c.instructions[idx] = if let Some(&(ep, _)) = c.fn_entries.get(name) {
                Instruction::encode(Opcode::Call, dst, (ep >> 8) as u8, (ep & 0xFF) as u8)
            } else { Instruction::encode(Opcode::Nop, dst, 0, 0) };
        }
        PilcBinary::with_debug_tables(c.constants, c.instructions, c.pc_spans, c.reg_symbols)
    }

    fn alloc_local_reg(&mut self) -> u8 { let r = self.next_local_reg; if self.next_local_reg < 95 { self.next_local_reg += 1; } r }
    fn alloc_reg(&mut self) -> u8 { let r = self.next_reg; self.next_reg = self.next_reg.wrapping_add(1).max(96); r }
    fn add_constant(&mut self, val: PilotVal) -> u16 {
        if let Some(i) = self.constants.iter().position(|&c| c == val) { return i as u16; }
        let idx = self.constants.len() as u16; self.constants.push(val); idx
    }
    fn emit(&mut self, op: Opcode, dst: u8, src1: u8, src2: u8) -> usize {
        let idx = self.instructions.len();
        self.instructions.push(Instruction::encode(op, dst, src1, src2)); self.pc_spans.push(self.cur_span); idx
    }
    fn emit_const(&mut self, val: PilotVal) -> u8 {
        let (c, r) = (self.add_constant(val), self.alloc_reg());
        self.emit(Opcode::LoadConst, r, (c >> 8) as u8, (c & 0xFF) as u8); r
    }
    fn stage_args(&mut self, args: &[u8]) {
        let n = args.len() as u8;
        let safe: Vec<u8> = args.iter().map(|&r| if r >= 1 && r <= n { let t = self.alloc_reg(); self.emit(Opcode::Move, t, r, 0); t } else { r }).collect();
        for (i, &r) in safe.iter().enumerate() { self.emit(Opcode::Move, (i + 1) as u8, r, 0); }
    }
    fn emit_intrinsic(&mut self, hook: &str, args: &[u8]) -> u8 {
        let r_hook = self.emit_const(PilotVal::from_str(hook));
        self.stage_args(args);
        let dst = self.alloc_reg();
        self.emit(Opcode::EngineIntrinsic, dst, r_hook, args.len() as u8); dst
    }

    fn eval_cmap(&self, arena: &AstArena, id: NodeId) -> Option<CMap> {
        if id.is_none() { return None; }
        let n = arena.node(id);
        match n.node_kind() {
            NodeKind::StrLit => Some(CMap::Val(PilotVal::from_str(arena.interner.resolve(SymbolId(n.lhs))))),
            NodeKind::IntLit => Some(CMap::Val(PilotVal::from_int(arena.i64_pool.get(n.lhs as usize).copied().unwrap_or(0)))),
            NodeKind::FloatLit => Some(CMap::Val(PilotVal::from_float(arena.f64_pool.get(n.lhs as usize).copied().unwrap_or(0.0)))),
            NodeKind::BoolLit => Some(CMap::Val(PilotVal::from_bool(n.lhs != 0))),
            NodeKind::MapExpr => {
                let (ch, mut map, mut i) = (arena.children_slice(n.lhs, n.rhs), HashMap::new(), 0);
                while i + 1 < ch.len() {
                    let kn = arena.node(NodeId(ch[i]));
                    let k = match kn.node_kind() {
                        NodeKind::IdentExpr | NodeKind::StrLit => arena.interner.resolve(SymbolId(kn.lhs)).to_string(),
                        NodeKind::IntLit => arena.i64_pool.get(kn.lhs as usize).map(|v| v.to_string()).unwrap_or_default(),
                        _ => String::new(),
                    };
                    if !k.is_empty() { if let Some(v) = self.eval_cmap(arena, NodeId(ch[i + 1])) { map.insert(k, v); } }
                    i += 2;
                }
                Some(CMap::Map(map))
            }
            _ => None,
        }
    }

    fn resolve_cmap(&self, arena: &AstArena, id: NodeId) -> Option<CMap> {
        if id.is_none() { return None; }
        let n = arena.node(id);
        match n.node_kind() {
            NodeKind::IdentExpr => self.const_maps.get(arena.interner.resolve(SymbolId(n.lhs))).cloned(),
            NodeKind::MemberExpr => {
                let CMap::Map(m) = self.resolve_cmap(arena, NodeId(n.lhs))? else { return None; };
                let fld = arena.interner.resolve(SymbolId(n.rhs));
                m.get(fld).cloned().or_else(|| (fld == "len").then_some(CMap::Val(PilotVal::from_int(m.len() as i64))))
            }
            _ => None,
        }
    }

    pub fn compile_node(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        if node_id.is_none() { return 0; }
        let node = *arena.node(node_id);
        self.cur_span = arena.span(node_id);
        match node.node_kind() {
            NodeKind::Program => {
                let children = arena.children_slice(node.lhs, node.rhs);
                for &c_id in children {
                    let cn = arena.node(NodeId(c_id));
                    if matches!(cn.node_kind(), NodeKind::LetStmt | NodeKind::VarStmt) {
                        if let Some(m) = self.eval_cmap(arena, NodeId(cn.rhs)) {
                            self.const_maps.insert(arena.interner.resolve(SymbolId(cn.lhs)).to_string(), m);
                        }
                    }
                }
                let (skip_jump, mut entry_fn) = (self.emit(Opcode::Jump, 0, 0, 0), None);
                for &c_id in children {
                    let cn = arena.node(NodeId(c_id));
                    if cn.node_kind() == NodeKind::FnDecl {
                        let fn_name = arena.interner.resolve(SymbolId(cn.lhs)).to_string();
                        let (fn_start, fn_len) = (cn.rhs as usize, cn.flags as usize);
                        let body_node = NodeId(arena.extra_children[fn_start]);
                        let params: Vec<String> = if fn_len > 1 {
                            arena.extra_children[fn_start + 1..fn_start + fn_len].iter()
                                .map(|&p| arena.interner.resolve(SymbolId(arena.node(NodeId(p)).lhs)).to_string()).collect()
                        } else { Vec::new() };
                        let pc = self.instructions.len() as u16;
                        if params.is_empty() { entry_fn = Some(pc); }
                        self.fn_entries.insert(fn_name, (pc, params.clone()));
                        self.var_slots.clear(); self.next_local_reg = 17; self.next_reg = 96;
                        for (i, p) in params.iter().enumerate() {
                            let slot = self.alloc_local_reg();
                            self.emit(Opcode::Move, slot, (i + 1) as u8, 0); self.var_slots.insert(p.clone(), slot);
                        }
                        let b_reg = self.compile_node(arena, body_node);
                        self.emit(Opcode::Move, 0, b_reg, 0); self.emit(Opcode::Ret, 0, 0, 0);
                    }
                }
                let main_start = self.instructions.len() as u16;
                self.instructions[skip_jump] = Instruction::encode(Opcode::Jump, 0, (main_start >> 8) as u8, (main_start & 0xFF) as u8);
                self.var_slots.clear(); self.next_local_reg = 17; self.next_reg = 96;
                let mut last_reg = 0;
                for &c_id in children { if arena.node(NodeId(c_id)).node_kind() != NodeKind::FnDecl { last_reg = self.compile_node(arena, NodeId(c_id)); } }
                if last_reg == 0 { if let Some(ep) = entry_fn { let dst = self.alloc_reg(); self.emit(Opcode::Call, dst, (ep >> 8) as u8, (ep & 0xFF) as u8); return dst; } }
                last_reg
            }
            NodeKind::BlockExpr | NodeKind::TupleExpr => {
                let mut last = 0; for &c in arena.children_slice(node.lhs, node.rhs) { last = self.compile_node(arena, NodeId(c)); } last
            }
            NodeKind::IntLit => self.emit_const(PilotVal::from_int(arena.i64_pool.get(node.lhs as usize).copied().unwrap_or(0))),
            NodeKind::FloatLit => self.emit_const(PilotVal::from_float(arena.f64_pool.get(node.lhs as usize).copied().unwrap_or(0.0))),
            NodeKind::BoolLit => self.emit_const(PilotVal::from_bool(node.lhs != 0)),
            NodeKind::StrLit => self.emit_const(PilotVal::from_str(arena.interner.resolve(SymbolId(node.lhs)))),
            NodeKind::IdentExpr => {
                let name = arena.interner.resolve(SymbolId(node.lhs));
                if let Some(&r) = self.var_slots.get(name) { r } else {
                    let r = self.alloc_local_reg(); self.var_slots.insert(name.to_string(), r); self.reg_symbols.push((r, node.lhs, 0)); r
                }
            }
            NodeKind::MemberExpr => {
                if let Some(CMap::Val(v)) = self.resolve_cmap(arena, node_id) { return self.emit_const(v); }
                let lhs_n = arena.node(NodeId(node.lhs));
                if lhs_n.node_kind() == NodeKind::IdentExpr {
                    let lhs_nm = arena.interner.resolve(SymbolId(lhs_n.lhs));
                    if let Some(&r_obj) = self.var_slots.get(lhs_nm) {
                        if !self.const_maps.contains_key(lhs_nm) {
                            let r_key = self.emit_const(PilotVal::from_str(arena.interner.resolve(SymbolId(node.rhs))));
                            return self.emit_intrinsic("map_get", &[r_obj, r_key]);
                        }
                    }
                }
                0
            }
            NodeKind::IndexExpr => {
                if let Some(CMap::Map(map)) = self.resolve_cmap(arena, NodeId(node.lhs)) {
                    let (r_idx, dst) = (self.compile_node(arena, NodeId(node.rhs)), self.emit_const(PilotVal::from_str("")));
                    for (k_str, v_val) in &map {
                        let (r_k, r_eq) = (self.emit_const(k_str.parse::<i64>().map_or_else(|_| PilotVal::from_str(k_str), PilotVal::from_int)), self.alloc_reg());
                        self.emit(Opcode::CmpEq, r_eq, r_idx, r_k);
                        let jf = self.emit(Opcode::JumpIfFalse, 0, r_eq, 0);
                        if let CMap::Val(v) = v_val { let c = self.add_constant(*v); self.emit(Opcode::LoadConst, dst, (c >> 8) as u8, (c & 0xFF) as u8); }
                        let after = self.instructions.len() as u16;
                        self.instructions[jf] = Instruction::encode(Opcode::JumpIfFalse, (after >> 8) as u8, r_eq, (after & 0xFF) as u8);
                    }
                    return dst;
                }
                0
            }
            NodeKind::LetStmt | NodeKind::VarStmt => {
                let (name, rhs) = (arena.interner.resolve(SymbolId(node.lhs)).to_string(), NodeId(node.rhs));
                if rhs.is_some() { if let Some(m) = self.eval_cmap(arena, rhs) { self.const_maps.insert(name.clone(), m); } }
                let r_val = if rhs.is_some() { self.compile_node(arena, rhs) } else { self.emit_const(PilotVal::NULL) };
                let slot = self.var_slots.get(&name).copied().unwrap_or_else(|| { let s = self.alloc_local_reg(); self.reg_symbols.push((s, node.lhs, 0)); s });
                self.emit(Opcode::Move, slot, r_val, 0); self.var_slots.insert(name, slot); slot
            }
            NodeKind::AssignStmt => {
                let (lhs_n, rhs_r) = (*arena.node(NodeId(node.lhs)), self.compile_node(arena, NodeId(node.rhs)));
                if lhs_n.node_kind() == NodeKind::IdentExpr {
                    let name = arena.interner.resolve(SymbolId(lhs_n.lhs)).to_string();
                    let slot = self.var_slots.get(&name).copied().unwrap_or_else(|| self.alloc_local_reg());
                    self.emit(Opcode::Move, slot, rhs_r, 0); self.var_slots.insert(name, slot); slot
                } else { rhs_r }
            }
            NodeKind::BinaryExpr => {
                let (r_l, r_r) = (self.compile_node(arena, NodeId(node.lhs)), self.compile_node(arena, NodeId(node.rhs)));
                if r_l == 0 || r_r == 0 { return 0; }
                let (dst, op) = (self.alloc_reg(), match node.flags {
                    1 => Opcode::AddI64, 2 => Opcode::SubI64, 3 => Opcode::MulI64, 4 => Opcode::DivI64, 6 => Opcode::RemI64,
                    8 => Opcode::CmpEq, 9 => Opcode::CmpNe, 10 => Opcode::CmpLt, 11 => Opcode::CmpLe, 12 => Opcode::CmpGt,
                    13 => Opcode::CmpGe, 14 | 16 => Opcode::And, 15 | 17 => Opcode::Or, 18 => Opcode::Xor, 19 => Opcode::Shl, 20 => Opcode::Shr, _ => Opcode::AddI64,
                });
                self.emit(op, dst, r_l, r_r); dst
            }
            NodeKind::UnaryExpr => {
                let (r_op, dst) = (self.compile_node(arena, NodeId(node.lhs)), self.alloc_reg());
                self.emit(if node.flags == 1 { Opcode::Move } else if node.flags == 5 { Opcode::Not } else { Opcode::NegI64 }, dst, r_op, 0); dst
            }
            NodeKind::IfExpr => {
                let r_cond = self.compile_node(arena, NodeId(node.lhs));
                let jf = self.emit(Opcode::JumpIfFalse, 0, r_cond, 0);
                let (then_n, else_n) = if node.flags == 2 { let ch = arena.children_slice(node.rhs, 2); (NodeId(ch[0]), NodeId(ch[1])) } else { (NodeId(node.rhs), NodeId::NONE) };
                let (dst, t_reg) = (self.alloc_reg(), self.compile_node(arena, then_n));
                self.emit(Opcode::Move, dst, t_reg, 0);
                let (j_end, else_pc) = (self.emit(Opcode::Jump, 0, 0, 0), self.instructions.len() as u16);
                self.instructions[jf] = Instruction::encode(Opcode::JumpIfFalse, (else_pc >> 8) as u8, r_cond, (else_pc & 0xFF) as u8);
                if else_n.is_some() { let e_reg = self.compile_node(arena, else_n); self.emit(Opcode::Move, dst, e_reg, 0); }
                let end_pc = self.instructions.len() as u16;
                self.instructions[j_end] = Instruction::encode(Opcode::Jump, 0, (end_pc >> 8) as u8, (end_pc & 0xFF) as u8); dst
            }
            NodeKind::WhileStmt => {
                let (start_pc, r_cond) = (self.instructions.len() as u16, self.compile_node(arena, NodeId(node.lhs)));
                let jf = self.emit(Opcode::JumpIfFalse, 0, r_cond, 0);
                self.loop_stack.push((start_pc, Vec::new()));
                self.compile_node(arena, NodeId(node.rhs));
                let (_, breaks) = self.loop_stack.pop().unwrap_or((start_pc, Vec::new()));
                self.emit(Opcode::Jump, 0, (start_pc >> 8) as u8, (start_pc & 0xFF) as u8);
                let end_pc = self.instructions.len() as u16;
                self.instructions[jf] = Instruction::encode(Opcode::JumpIfFalse, (end_pc >> 8) as u8, r_cond, (end_pc & 0xFF) as u8);
                for b in breaks { self.instructions[b] = Instruction::encode(Opcode::Jump, 0, (end_pc >> 8) as u8, (end_pc & 0xFF) as u8); }
                0
            }
            NodeKind::BreakStmt => { if !self.loop_stack.is_empty() { let j = self.emit(Opcode::Jump, 0, 0, 0); if let Some((_, b)) = self.loop_stack.last_mut() { b.push(j); } } 0 }
            NodeKind::ContinueStmt => { if let Some(&(sp, _)) = self.loop_stack.last() { self.emit(Opcode::Jump, 0, (sp >> 8) as u8, (sp & 0xFF) as u8); } 0 }
            NodeKind::ForExpr => {
                let (loop_var, ch) = (arena.interner.resolve(SymbolId(node.lhs)).to_string(), arena.children_slice(node.rhs, node.flags as u32));
                if ch.len() >= 2 {
                    let (iter_n, body_n) = (*arena.node(NodeId(ch[0])), NodeId(ch[1]));
                    if iter_n.node_kind() == NodeKind::BinaryExpr && matches!(iter_n.flags, 23 | 24) {
                        let (r_s, r_e, r_end) = (self.compile_node(arena, NodeId(iter_n.lhs)), self.compile_node(arena, NodeId(iter_n.rhs)), self.alloc_local_reg());
                        self.emit(Opcode::Move, r_end, r_e, 0);
                        let slot = self.var_slots.get(&loop_var).copied().unwrap_or_else(|| self.alloc_local_reg());
                        self.emit(Opcode::Move, slot, r_s, 0); self.var_slots.insert(loop_var, slot);
                        let (start_pc, r_cond) = (self.instructions.len() as u16, self.alloc_reg());
                        self.emit(if iter_n.flags == 23 { Opcode::CmpLe } else { Opcode::CmpLt }, r_cond, slot, r_end);
                        let jf = self.emit(Opcode::JumpIfFalse, 0, r_cond, 0);
                        self.loop_stack.push((start_pc, Vec::new()));
                        self.compile_node(arena, body_n);
                        let (_, breaks) = self.loop_stack.pop().unwrap_or((start_pc, Vec::new()));
                        let r_one = self.emit_const(PilotVal::from_int(1));
                        self.emit(Opcode::AddI64, slot, slot, r_one);
                        self.emit(Opcode::Jump, 0, (start_pc >> 8) as u8, (start_pc & 0xFF) as u8);
                        let end_pc = self.instructions.len() as u16;
                        self.instructions[jf] = Instruction::encode(Opcode::JumpIfFalse, (end_pc >> 8) as u8, r_cond, (end_pc & 0xFF) as u8);
                        for b in breaks { self.instructions[b] = Instruction::encode(Opcode::Jump, 0, (end_pc >> 8) as u8, (end_pc & 0xFF) as u8); }
                    }
                }
                0
            }
            NodeKind::ReturnStmt => {
                let r_ret = if NodeId(node.lhs).is_some() { self.compile_node(arena, NodeId(node.lhs)) } else { 0 };
                self.emit(Opcode::Move, 0, r_ret, 0); self.emit(Opcode::Ret, 0, 0, 0); 0
            }
            NodeKind::ExprStmt => if NodeId(node.lhs).is_some() { self.compile_node(arena, NodeId(node.lhs)) } else { 0 },
            NodeKind::CallExpr => {
                let (arg_start, arg_len) = (node.lhs as usize, node.rhs as usize);
                if arg_len > 0 && arg_start < arena.extra_children.len() {
                    let callee_n = *arena.node(NodeId(arena.extra_children[arg_start]));
                    if callee_n.node_kind() == NodeKind::IdentExpr {
                        let name = arena.interner.resolve(SymbolId(callee_n.lhs));
                        if name == "assert" && arg_len > 1 {
                            let cond_id = NodeId(arena.extra_children[arg_start + 1]);
                            let cn = *arena.node(cond_id);
                            let r_c = if cn.node_kind() == NodeKind::TupleExpr { arena.children_slice(cn.lhs, cn.rhs).first().map_or(0, |&id| self.compile_node(arena, NodeId(id))) } else { self.compile_node(arena, cond_id) };
                            if r_c != 0 { let _ = self.emit_intrinsic("rt_assert", &[r_c]); }
                            return 0;
                        } else if (name == "print" || name == "println") && arg_len > 1 {
                            let r_a = self.compile_node(arena, NodeId(arena.extra_children[arg_start + 1]));
                            let _ = self.emit_intrinsic("rt_print", &[r_a]);
                            return 0;
                        } else if name == "nameof" && arg_len > 1 {
                            let arg_id = NodeId(arena.extra_children[arg_start + 1]);
                            let an = arena.node(arg_id);
                            return if an.node_kind() == NodeKind::IdentExpr { self.emit_const(PilotVal::from_str(arena.interner.resolve(SymbolId(an.lhs)))) } else { self.compile_node(arena, arg_id) };
                        } else if arg_len == 2 && arena.node(NodeId(arena.extra_children[arg_start + 1])).node_kind() == NodeKind::MapExpr && !self.fn_entries.contains_key(name) {
                            let mn = *arena.node(NodeId(arena.extra_children[arg_start + 1]));
                            let (r_new, r_map, r_set) = (self.emit_const(PilotVal::from_str("map_new")), self.alloc_local_reg(), self.alloc_local_reg());
                            self.emit(Opcode::EngineIntrinsic, r_map, r_new, 0);
                            let c_set = self.add_constant(PilotVal::from_str("map_set"));
                            self.emit(Opcode::LoadConst, r_set, (c_set >> 8) as u8, (c_set & 0xFF) as u8);
                            let (ch, mut i) = (arena.children_slice(mn.lhs, mn.rhs), 0);
                            while i + 1 < ch.len() {
                                let kn = arena.node(NodeId(ch[i]));
                                let k_nm = if matches!(kn.node_kind(), NodeKind::IdentExpr | NodeKind::StrLit) { arena.interner.resolve(SymbolId(kn.lhs)).to_string() } else { String::new() };
                                let (r_v, r_k) = (self.compile_node(arena, NodeId(ch[i + 1])), self.emit_const(PilotVal::from_str(&k_nm)));
                                self.emit(Opcode::Move, 1, r_map, 0); self.emit(Opcode::Move, 2, r_k, 0); self.emit(Opcode::Move, 3, r_v, 0);
                                self.emit(Opcode::EngineIntrinsic, r_map, r_set, 3);
                                i += 2;
                            }
                            return r_map;
                        } else if name == "__engine" && arg_len > 1 {
                            let r_hook = self.compile_node(arena, NodeId(arena.extra_children[arg_start + 1]));
                            let args: Vec<u8> = (2..arg_len).map(|i| self.compile_node(arena, NodeId(arena.extra_children[arg_start + i]))).collect();
                            self.stage_args(&args);
                            let dst = self.alloc_reg();
                            self.emit(Opcode::EngineIntrinsic, dst, r_hook, args.len() as u8);
                            return dst;
                        } else {
                            let args: Vec<u8> = (1..arg_len).map(|i| self.compile_node(arena, NodeId(arena.extra_children[arg_start + i]))).collect();
                            self.stage_args(&args);
                            let dst = self.alloc_reg();
                            if let Some(&(ep, _)) = self.fn_entries.get(name) { self.emit(Opcode::Call, dst, (ep >> 8) as u8, (ep & 0xFF) as u8); }
                            else { let idx = self.emit(Opcode::Call, dst, 0, 0); self.call_patches.push((idx, name.to_string())); }
                            return dst;
                        }
                    } else if callee_n.node_kind() == NodeKind::MemberExpr {
                        let hook = match arena.interner.resolve(SymbolId(callee_n.rhs)) { "len" => Some("str_len"), "byte_at" => Some("str_byte_at"), "slice" | "substring" => Some("str_slice"), _ => None };
                        if let Some(h) = hook {
                            let mut args = vec![self.compile_node(arena, NodeId(callee_n.lhs))];
                            for i in 1..arg_len { args.push(self.compile_node(arena, NodeId(arena.extra_children[arg_start + i]))); }
                            return self.emit_intrinsic(h, &args);
                        }
                    }
                }
                0
            }
            NodeKind::PolyglotBlock => self.emit_const(PilotVal::from_int(0)),
            _ => 0,
        }
    }
}
