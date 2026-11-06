//! Milestone 39.1: Two-Phase AstArena -> .pilc v3 Register Bytecode Compiler.
//!
//! Lowers Pilot declarations, statements, and expressions from AstArena into
//! 32-bit fixed-width 3-address PilVM v3 instructions and constant pools.

use std::collections::HashMap;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_rt::PilotVal;
use crate::instruction::{Instruction, Opcode};
use crate::pilc::PilcBinary;

#[derive(Clone, Debug)]
enum ConstMapVal {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Map(HashMap<String, ConstMapVal>),
}

pub struct BytecodeCompiler {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<PilotVal>,
    var_slots: HashMap<String, u8>,
    fn_entries: HashMap<String, (u16, Vec<String>)>,
    call_patches: Vec<(usize, String)>,
    const_maps: HashMap<String, ConstMapVal>,
    next_reg: u8,
}

impl Default for BytecodeCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl BytecodeCompiler {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            constants: Vec::new(),
            var_slots: HashMap::new(),
            fn_entries: HashMap::new(),
            call_patches: Vec::new(),
            const_maps: HashMap::new(),
            next_reg: 1, // r0 is reserved for function return value
        }
    }

    pub fn compile(arena: &AstArena, root: NodeId) -> PilcBinary {
        let mut compiler = Self::new();
        let last_reg = if root.is_some() {
            compiler.compile_node(arena, root)
        } else {
            0
        };
        compiler.emit(Opcode::Move, 0, last_reg, 0);
        compiler.emit(Opcode::Ret, 0, 0, 0);
        compiler.emit(Opcode::Halt, 0, 0, 0);

        // Resolve call patches
        for &(call_idx, ref fn_name) in &compiler.call_patches {
            let inst = compiler.instructions[call_idx];
            let dst = inst.dst();
            if let Some(&(entry, _)) = compiler.fn_entries.get(fn_name) {
                compiler.instructions[call_idx] = Instruction::encode(
                    Opcode::Call,
                    dst,
                    (entry >> 8) as u8,
                    (entry & 0xFF) as u8,
                );
            } else {
                compiler.instructions[call_idx] = Instruction::encode(Opcode::Nop, dst, 0, 0);
            }
        }

        PilcBinary::new(compiler.constants, compiler.instructions)
    }

    fn alloc_reg(&mut self) -> u8 {
        let r = self.next_reg;
        self.next_reg = self.next_reg.wrapping_add(1);
        if self.next_reg == 0 {
            self.next_reg = 17;
        }
        r
    }

    fn add_constant(&mut self, val: PilotVal) -> u16 {
        for (i, &c) in self.constants.iter().enumerate() {
            if c == val {
                return i as u16;
            }
        }
        let idx = self.constants.len() as u16;
        self.constants.push(val);
        idx
    }

    fn emit(&mut self, op: Opcode, dst: u8, src1: u8, src2: u8) -> usize {
        let idx = self.instructions.len();
        self.instructions.push(Instruction::encode(op, dst, src1, src2));
        idx
    }

    fn eval_const_map(&self, arena: &AstArena, node_id: NodeId) -> Option<ConstMapVal> {
        if node_id.is_none() {
            return None;
        }
        let node = arena.node(node_id);
        match node.node_kind() {
            NodeKind::StrLit => {
                let sym = SymbolId(node.lhs);
                Some(ConstMapVal::Str(arena.interner.resolve(sym).to_string()))
            }
            NodeKind::IntLit => {
                let val = arena.i64_pool.get(node.lhs as usize).copied().unwrap_or(0);
                Some(ConstMapVal::Int(val))
            }
            NodeKind::FloatLit => {
                let val = arena.f64_pool.get(node.lhs as usize).copied().unwrap_or(0.0);
                Some(ConstMapVal::Float(val))
            }
            NodeKind::BoolLit => {
                Some(ConstMapVal::Bool(node.lhs != 0))
            }
            NodeKind::MapExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut map = HashMap::new();
                let mut i = 0;
                while i + 1 < children.len() {
                    let k_id = NodeId(children[i]);
                    let v_id = NodeId(children[i + 1]);
                    let k_node = arena.node(k_id);
                    let k_name = match k_node.node_kind() {
                        NodeKind::IdentExpr | NodeKind::StrLit => {
                            arena.interner.resolve(SymbolId(k_node.lhs)).to_string()
                        }
                        _ => String::new(),
                    };
                    if !k_name.is_empty() {
                        if let Some(v_val) = self.eval_const_map(arena, v_id) {
                            map.insert(k_name, v_val);
                        }
                    }
                    i += 2;
                }
                Some(ConstMapVal::Map(map))
            }
            _ => None,
        }
    }

    fn resolve_const_member(&self, arena: &AstArena, node_id: NodeId) -> Option<ConstMapVal> {
        if node_id.is_none() {
            return None;
        }
        let node = arena.node(node_id);
        match node.node_kind() {
            NodeKind::IdentExpr => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym);
                self.const_maps.get(name).cloned()
            }
            NodeKind::MemberExpr => {
                let parent_val = self.resolve_const_member(arena, NodeId(node.lhs))?;
                if let ConstMapVal::Map(map) = parent_val {
                    let member_sym = SymbolId(node.rhs);
                    let member_name = arena.interner.resolve(member_sym);
                    map.get(member_name).cloned()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn compile_node(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        if node_id.is_none() {
            return 0;
        }
        let node = *arena.node(node_id);

        match node.node_kind() {
            NodeKind::Program => {
                let children = arena.children_slice(node.lhs, node.rhs);

                for &child_id in children {
                    let child_node = arena.node(NodeId(child_id));
                    if matches!(child_node.node_kind(), NodeKind::LetStmt | NodeKind::VarStmt) {
                        let sym = SymbolId(child_node.lhs);
                        let name = arena.interner.resolve(sym).to_string();
                        if let Some(map_val) = self.eval_const_map(arena, NodeId(child_node.rhs)) {
                            self.const_maps.insert(name, map_val);
                        }
                    }
                }

                // Jump over function bodies to top-level code
                let skip_fns_jump = self.emit(Opcode::Jump, 0, 0, 0);

                for &child_id in children {
                    let child_node = arena.node(NodeId(child_id));
                    if child_node.node_kind() == NodeKind::FnDecl {
                        let fn_name = arena.interner.resolve(SymbolId(child_node.lhs)).to_string();
                        let fn_start = child_node.rhs as usize;
                        let fn_len = child_node.flags as usize;
                        let body_node = NodeId(arena.extra_children[fn_start]);
                        let mut params = Vec::new();
                        if fn_len > 1 {
                            for &p_id in &arena.extra_children[fn_start + 1..fn_start + fn_len] {
                                let p_node = arena.node(NodeId(p_id));
                                let p_name = arena.interner.resolve(SymbolId(p_node.lhs));
                                params.push(p_name.to_string());
                            }
                        }

                        let entry_pc = self.instructions.len() as u16;
                        self.fn_entries.insert(fn_name.clone(), (entry_pc, params.clone()));

                        self.var_slots.clear();
                        self.next_reg = 17; // Local variables start at r17; r1..=16 are reserved for call arguments

                        // Save incoming arguments from r1..=params.len() into dedicated local slots
                        for (i, p) in params.iter().enumerate() {
                            let slot = self.alloc_reg();
                            self.emit(Opcode::Move, slot, (i + 1) as u8, 0);
                            self.var_slots.insert(p.clone(), slot);
                        }

                        let body_reg = self.compile_node(arena, body_node);
                        self.emit(Opcode::Move, 0, body_reg, 0);
                        self.emit(Opcode::Ret, 0, 0, 0);
                    }
                }

                let main_start = self.instructions.len() as u16;
                self.instructions[skip_fns_jump] = Instruction::encode(
                    Opcode::Jump,
                    0,
                    (main_start >> 8) as u8,
                    (main_start & 0xFF) as u8,
                );

                self.var_slots.clear();
                self.next_reg = 17; // Local variables start at r17; r1..=16 are reserved for call arguments

                if let Some(&(main_entry, _)) = self.fn_entries.get("main") {
                    let dst = self.alloc_reg();
                    self.emit(Opcode::Call, dst, (main_entry >> 8) as u8, (main_entry & 0xFF) as u8);
                    dst
                } else {
                    let mut last_reg = 0;
                    for &child_id in children {
                        let child_node = arena.node(NodeId(child_id));
                        if child_node.node_kind() != NodeKind::FnDecl {
                            last_reg = self.compile_node(arena, NodeId(child_id));
                        }
                    }
                    last_reg
                }
            }

            NodeKind::BlockExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut last_reg = 0;
                for &child_id in children {
                    last_reg = self.compile_node(arena, NodeId(child_id));
                }
                last_reg
            }

            NodeKind::TupleExpr => {
                let items = arena.children_slice(node.lhs, node.rhs);
                let mut last_reg = 0;
                for &item_id in items {
                    last_reg = self.compile_node(arena, NodeId(item_id));
                }
                last_reg
            }

            NodeKind::IntLit => {
                let val = arena.i64_pool.get(node.lhs as usize).copied().unwrap_or(0);
                let const_idx = self.add_constant(PilotVal::from_int(val));
                let dst = self.alloc_reg();
                self.emit(Opcode::LoadConst, dst, (const_idx >> 8) as u8, (const_idx & 0xFF) as u8);
                dst
            }

            NodeKind::FloatLit => {
                let val = arena.f64_pool.get(node.lhs as usize).copied().unwrap_or(0.0);
                let const_idx = self.add_constant(PilotVal::from_float(val));
                let dst = self.alloc_reg();
                self.emit(Opcode::LoadConst, dst, (const_idx >> 8) as u8, (const_idx & 0xFF) as u8);
                dst
            }

            NodeKind::BoolLit => {
                let val = node.lhs != 0;
                let const_idx = self.add_constant(PilotVal::from_bool(val));
                let dst = self.alloc_reg();
                self.emit(Opcode::LoadConst, dst, (const_idx >> 8) as u8, (const_idx & 0xFF) as u8);
                dst
            }

            NodeKind::StrLit => {
                let s = arena.interner.resolve(SymbolId(node.lhs));
                let const_idx = self.add_constant(PilotVal::from_str(s));
                let dst = self.alloc_reg();
                self.emit(Opcode::LoadConst, dst, (const_idx >> 8) as u8, (const_idx & 0xFF) as u8);
                dst
            }

            NodeKind::IdentExpr => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym);
                if let Some(&reg) = self.var_slots.get(name) {
                    reg
                } else {
                    let r = self.alloc_reg();
                    self.var_slots.insert(name.to_string(), r);
                    r
                }
            }

            NodeKind::MemberExpr => {
                if let Some(const_val) = self.resolve_const_member(arena, node_id) {
                    match const_val {
                        ConstMapVal::Str(s) => {
                            let const_idx = self.add_constant(PilotVal::from_str(&s));
                            let dst = self.alloc_reg();
                            self.emit(Opcode::LoadConst, dst, (const_idx >> 8) as u8, (const_idx & 0xFF) as u8);
                            return dst;
                        }
                        ConstMapVal::Int(i) => {
                            let const_idx = self.add_constant(PilotVal::from_int(i));
                            let dst = self.alloc_reg();
                            self.emit(Opcode::LoadConst, dst, (const_idx >> 8) as u8, (const_idx & 0xFF) as u8);
                            return dst;
                        }
                        ConstMapVal::Float(f) => {
                            let const_idx = self.add_constant(PilotVal::from_float(f));
                            let dst = self.alloc_reg();
                            self.emit(Opcode::LoadConst, dst, (const_idx >> 8) as u8, (const_idx & 0xFF) as u8);
                            return dst;
                        }
                        ConstMapVal::Bool(b) => {
                            let const_idx = self.add_constant(PilotVal::from_bool(b));
                            let dst = self.alloc_reg();
                            self.emit(Opcode::LoadConst, dst, (const_idx >> 8) as u8, (const_idx & 0xFF) as u8);
                            return dst;
                        }
                        _ => {}
                    }
                }
                0
            }

            NodeKind::LetStmt | NodeKind::VarStmt => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym).to_string();
                let rhs_node = NodeId(node.rhs);
                if rhs_node.is_some() {
                    if let Some(map_val) = self.eval_const_map(arena, rhs_node) {
                        self.const_maps.insert(name.clone(), map_val);
                    }
                }
                let r_val = if rhs_node.is_some() {
                    self.compile_node(arena, rhs_node)
                } else {
                    let r = self.alloc_reg();
                    let const_idx = self.add_constant(PilotVal::NULL);
                    self.emit(Opcode::LoadConst, r, (const_idx >> 8) as u8, (const_idx & 0xFF) as u8);
                    r
                };
                let slot = self.alloc_reg();
                self.emit(Opcode::Move, slot, r_val, 0);
                self.var_slots.insert(name, slot);
                slot
            }

            NodeKind::AssignStmt => {
                let lhs_node = *arena.node(NodeId(node.lhs));
                let rhs_reg = self.compile_node(arena, NodeId(node.rhs));
                if lhs_node.node_kind() == NodeKind::IdentExpr {
                    let sym = SymbolId(lhs_node.lhs);
                    let name = arena.interner.resolve(sym);
                    if let Some(&slot) = self.var_slots.get(name) {
                        self.emit(Opcode::Move, slot, rhs_reg, 0);
                        slot
                    } else {
                        let slot = self.alloc_reg();
                        self.emit(Opcode::Move, slot, rhs_reg, 0);
                        self.var_slots.insert(name.to_string(), slot);
                        slot
                    }
                } else {
                    rhs_reg
                }
            }

            NodeKind::BinaryExpr => {
                let r_lhs = self.compile_node(arena, NodeId(node.lhs));
                let r_rhs = self.compile_node(arena, NodeId(node.rhs));
                if r_lhs == 0 || r_rhs == 0 {
                    return 0;
                }
                let dst = self.alloc_reg();

                let op = node.flags;
                match op {
                    1 => self.emit(Opcode::AddI64, dst, r_lhs, r_rhs),
                    2 => self.emit(Opcode::SubI64, dst, r_lhs, r_rhs),
                    3 => self.emit(Opcode::MulI64, dst, r_lhs, r_rhs),
                    4 => self.emit(Opcode::DivI64, dst, r_lhs, r_rhs),
                    6 => self.emit(Opcode::RemI64, dst, r_lhs, r_rhs),
                    8 => self.emit(Opcode::CmpEq, dst, r_lhs, r_rhs),
                    9 => self.emit(Opcode::CmpNe, dst, r_lhs, r_rhs),
                    10 => self.emit(Opcode::CmpLt, dst, r_lhs, r_rhs),
                    11 => self.emit(Opcode::CmpLe, dst, r_lhs, r_rhs),
                    12 => self.emit(Opcode::CmpGt, dst, r_lhs, r_rhs),
                    13 => self.emit(Opcode::CmpGe, dst, r_lhs, r_rhs),
                    14 | 16 => self.emit(Opcode::And, dst, r_lhs, r_rhs),
                    15 | 17 => self.emit(Opcode::Or, dst, r_lhs, r_rhs),
                    18 => self.emit(Opcode::Xor, dst, r_lhs, r_rhs),
                    19 => self.emit(Opcode::Shl, dst, r_lhs, r_rhs),
                    20 => self.emit(Opcode::Shr, dst, r_lhs, r_rhs),
                    _ => self.emit(Opcode::AddI64, dst, r_lhs, r_rhs),
                };
                dst
            }

            NodeKind::UnaryExpr => {
                let r_op = self.compile_node(arena, NodeId(node.lhs));
                let dst = self.alloc_reg();
                if node.flags == 1 {
                    self.emit(Opcode::Move, dst, r_op, 0);
                } else if node.flags == 5 {
                    self.emit(Opcode::Not, dst, r_op, 0);
                } else {
                    self.emit(Opcode::NegI64, dst, r_op, 0);
                }
                dst
            }

            NodeKind::IfExpr => {
                let r_cond = self.compile_node(arena, NodeId(node.lhs));
                let jump_false_idx = self.emit(Opcode::JumpIfFalse, 0, r_cond, 0);

                let (then_node, else_node) = if node.flags == 2 {
                    let children = arena.children_slice(node.rhs, 2);
                    (NodeId(children[0]), NodeId(children[1]))
                } else {
                    (NodeId(node.rhs), NodeId::NONE)
                };

                let dst = self.alloc_reg();
                let then_reg = self.compile_node(arena, then_node);
                self.emit(Opcode::Move, dst, then_reg, 0);
                let jump_end_idx = self.emit(Opcode::Jump, 0, 0, 0);

                let else_pos = self.instructions.len() as u16;
                // Patch JumpIfFalse target
                self.instructions[jump_false_idx] = Instruction::encode(
                    Opcode::JumpIfFalse,
                    (else_pos >> 8) as u8,
                    r_cond,
                    (else_pos & 0xFF) as u8,
                );

                if else_node.is_some() {
                    let else_reg = self.compile_node(arena, else_node);
                    self.emit(Opcode::Move, dst, else_reg, 0);
                }

                // Patch JumpEnd target
                let end_pos = self.instructions.len() as u16;
                self.instructions[jump_end_idx] = Instruction::encode(
                    Opcode::Jump,
                    0,
                    (end_pos >> 8) as u8,
                    (end_pos & 0xFF) as u8,
                );

                dst
            }

            NodeKind::WhileStmt => {
                let loop_start = self.instructions.len() as u16;
                let r_cond = self.compile_node(arena, NodeId(node.lhs));
                let jump_false_idx = self.emit(Opcode::JumpIfFalse, 0, r_cond, 0);

                self.compile_node(arena, NodeId(node.rhs));
                self.emit(
                    Opcode::Jump,
                    0,
                    (loop_start >> 8) as u8,
                    (loop_start & 0xFF) as u8,
                );

                let loop_end = self.instructions.len() as u16;
                self.instructions[jump_false_idx] = Instruction::encode(
                    Opcode::JumpIfFalse,
                    (loop_end >> 8) as u8,
                    r_cond,
                    (loop_end & 0xFF) as u8,
                );

                0
            }

            NodeKind::MatchExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                if children.is_empty() {
                    return 0;
                }
                let scrutinee_id = NodeId(children[0]);
                let r_scrutinee = if scrutinee_id.is_some() {
                    self.compile_node(arena, scrutinee_id)
                } else {
                    0
                };
                let arms = &children[1..];
                let dst = self.alloc_reg();
                let mut end_jumps = Vec::new();

                for &arm_id in arms {
                    let arm_node = arena.node(NodeId(arm_id));
                    if arm_node.node_kind() != NodeKind::MatchArm {
                        continue;
                    }
                    let pat_id = NodeId(arm_node.lhs);
                    let body_id = NodeId(arm_node.rhs);
                    let pat_node = arena.node(pat_id);

                    let is_wild = pat_id.is_none() || pat_node.node_kind() == NodeKind::PatWildcard;
                    let mut next_arm_jump = None;

                    if !is_wild {
                        let pat_eval_id = if pat_node.node_kind() == NodeKind::PatLiteral {
                            NodeId(pat_node.lhs)
                        } else {
                            pat_id
                        };
                        let r_pat = self.compile_node(arena, pat_eval_id);
                        let r_eq = self.alloc_reg();
                        self.emit(Opcode::CmpEq, r_eq, r_scrutinee, r_pat);
                        let j_false = self.emit(Opcode::JumpIfFalse, 0, r_eq, 0);
                        next_arm_jump = Some(j_false);
                    }

                    let r_body = self.compile_node(arena, body_id);
                    self.emit(Opcode::Move, dst, r_body, 0);
                    let j_end = self.emit(Opcode::Jump, 0, 0, 0);
                    end_jumps.push(j_end);

                    if let Some(j_false) = next_arm_jump {
                        let pos = self.instructions.len() as u16;
                        let cur = self.instructions[j_false];
                        self.instructions[j_false] = Instruction::encode(
                            Opcode::JumpIfFalse,
                            (pos >> 8) as u8,
                            cur.src1(),
                            (pos & 0xFF) as u8,
                        );
                    }

                    if is_wild {
                        break;
                    }
                }

                let end_pos = self.instructions.len() as u16;
                for j_end in end_jumps {
                    self.instructions[j_end] = Instruction::encode(
                        Opcode::Jump,
                        0,
                        (end_pos >> 8) as u8,
                        (end_pos & 0xFF) as u8,
                    );
                }
                dst
            }

            NodeKind::ReturnStmt => {
                let expr_id = NodeId(node.lhs);
                let r_ret = if expr_id.is_some() {
                    self.compile_node(arena, expr_id)
                } else {
                    0
                };
                self.emit(Opcode::Move, 0, r_ret, 0); // r0 is return register
                self.emit(Opcode::Ret, 0, 0, 0);
                0
            }

            NodeKind::ExprStmt => {
                let expr_id = NodeId(node.lhs);
                if expr_id.is_some() {
                    let expr_node = *arena.node(expr_id);
                    if expr_node.node_kind() == NodeKind::CallExpr {
                        let arg_start = expr_node.lhs as usize;
                        let arg_len = expr_node.rhs as usize;
                        if arg_len > 0 && arg_start < arena.extra_children.len() {
                            let callee_id = NodeId(arena.extra_children[arg_start]);
                            let callee_node = *arena.node(callee_id);
                            if callee_node.node_kind() == NodeKind::IdentExpr {
                                let name = arena.interner.resolve(SymbolId(callee_node.lhs));
                                if name == "assert" {
                                    return 0;
                                }
                            }
                        }
                    }
                    self.compile_node(arena, expr_id)
                } else {
                    0
                }
            }

            NodeKind::FnDecl => 0,
            NodeKind::PolyglotBlock => {
                let const_idx = self.add_constant(PilotVal::from_int(0));
                let dst = self.alloc_reg();
                self.emit(Opcode::LoadConst, dst, (const_idx >> 8) as u8, (const_idx & 0xFF) as u8);
                dst
            }

            NodeKind::CallExpr => {
                let arg_start = node.lhs as usize;
                let arg_len = node.rhs as usize;
                if arg_len > 0 && arg_start < arena.extra_children.len() {
                    let callee_id = NodeId(arena.extra_children[arg_start]);
                    let callee_node = *arena.node(callee_id);
                    if callee_node.node_kind() == NodeKind::IdentExpr {
                        let name = arena.interner.resolve(SymbolId(callee_node.lhs));
                        if name == "assert" && arg_len > 1 {
                            let cond_arg = NodeId(arena.extra_children[arg_start + 1]);
                            let cond_node = *arena.node(cond_arg);
                            let r_cond = if cond_node.node_kind() == NodeKind::TupleExpr {
                                let items = arena.children_slice(cond_node.lhs, cond_node.rhs);
                                if !items.is_empty() {
                                    self.compile_node(arena, NodeId(items[0]))
                                } else {
                                    0
                                }
                            } else {
                                self.compile_node(arena, cond_arg)
                            };
                            if r_cond != 0 {
                                self.emit(Opcode::Assert, 0, r_cond, 0);
                            }
                            return 0;
                        } else if (name == "print" || name == "println") && arg_len > 1 {
                            let r_arg = self.compile_node(arena, NodeId(arena.extra_children[arg_start + 1]));
                            self.emit(Opcode::Print, 0, r_arg, 0);
                            return 0;
                        } else if name == "nameof" && arg_len > 1 {
                            let arg_id = NodeId(arena.extra_children[arg_start + 1]);
                            return self.compile_node(arena, arg_id);
                        } else if name == "__engine" && arg_len > 1 {
                            let hook_arg = NodeId(arena.extra_children[arg_start + 1]);
                            let r_hook = self.compile_node(arena, hook_arg);
                            let num_args = (arg_len - 2) as u8;
                            let mut arg_regs = Vec::with_capacity(num_args as usize);
                            for i in 0..num_args {
                                let arg_id = NodeId(arena.extra_children[arg_start + 2 + (i as usize)]);
                                let r = self.compile_node(arena, arg_id);
                                arg_regs.push(r);
                            }
                            let mut final_args = Vec::with_capacity(num_args as usize);
                            for &r in &arg_regs {
                                if r >= 1 && r <= num_args {
                                    let tmp = self.alloc_reg();
                                    self.emit(Opcode::Move, tmp, r, 0);
                                    final_args.push(tmp);
                                } else {
                                    final_args.push(r);
                                }
                            }
                            for (i, &r) in final_args.iter().enumerate() {
                                self.emit(Opcode::Move, (i + 1) as u8, r, 0);
                            }
                            let dst = self.alloc_reg();
                            self.emit(Opcode::EngineIntrinsic, dst, r_hook, num_args);
                            return dst;
                        } else {
                            let mut arg_regs = Vec::new();
                            for i in 1..arg_len {
                                let arg_id = NodeId(arena.extra_children[arg_start + i]);
                                let r = self.compile_node(arena, arg_id);
                                arg_regs.push(r);
                            }
                            let num_args = arg_regs.len() as u8;
                            let mut final_args = Vec::new();
                            for &r in &arg_regs {
                                if r >= 1 && r <= num_args {
                                    let tmp = self.alloc_reg();
                                    self.emit(Opcode::Move, tmp, r, 0);
                                    final_args.push(tmp);
                                } else {
                                    final_args.push(r);
                                }
                            }
                            for (i, &r) in final_args.iter().enumerate() {
                                self.emit(Opcode::Move, (i + 1) as u8, r, 0);
                            }
                            let dst = self.alloc_reg();
                            if let Some(&(entry, _)) = self.fn_entries.get(name) {
                                self.emit(Opcode::Call, dst, (entry >> 8) as u8, (entry & 0xFF) as u8);
                            } else {
                                let idx = self.emit(Opcode::Call, dst, 0, 0);
                                self.call_patches.push((idx, name.to_string()));
                            }
                            return dst;
                        }
                    }
                }
                0
            }

            _ => 0,
        }
    }
}
