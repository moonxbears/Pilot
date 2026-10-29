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

pub struct BytecodeCompiler {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<PilotVal>,
    var_slots: HashMap<String, u8>,
    fn_entries: HashMap<String, (u16, Vec<String>)>,
    call_patches: Vec<(usize, String)>,
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
            self.next_reg = 1;
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

    pub fn compile_node(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        if node_id.is_none() {
            return 0;
        }
        let node = *arena.node(node_id);

        match node.node_kind() {
            NodeKind::Program => {
                let children = arena.children_slice(node.lhs, node.rhs);

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
                        self.fn_entries.insert(fn_name, (entry_pc, params.clone()));

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

            NodeKind::LetStmt | NodeKind::VarStmt => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym).to_string();
                let rhs_node = NodeId(node.rhs);
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

                let then_reg = self.compile_node(arena, then_node);
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
                    self.compile_node(arena, else_node);
                }

                // Patch JumpEnd target
                let end_pos = self.instructions.len() as u16;
                self.instructions[jump_end_idx] = Instruction::encode(
                    Opcode::Jump,
                    0,
                    (end_pos >> 8) as u8,
                    (end_pos & 0xFF) as u8,
                );

                then_reg
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

            NodeKind::FnDecl => 0,

            NodeKind::CallExpr => {
                let arg_start = node.lhs as usize;
                let arg_len = node.rhs as usize;
                if arg_len > 0 && arg_start < arena.extra_children.len() {
                    let callee_id = NodeId(arena.extra_children[arg_start]);
                    let callee_node = *arena.node(callee_id);
                    if callee_node.node_kind() == NodeKind::IdentExpr {
                        let name = arena.interner.resolve(SymbolId(callee_node.lhs));
                        if name == "assert" && arg_len > 1 {
                            let r_cond = self.compile_node(arena, NodeId(arena.extra_children[arg_start + 1]));
                            self.emit(Opcode::Assert, 0, r_cond, 0);
                            return 0;
                        } else if (name == "print" || name == "println") && arg_len > 1 {
                            let r_arg = self.compile_node(arena, NodeId(arena.extra_children[arg_start + 1]));
                            self.emit(Opcode::Print, 0, r_arg, 0);
                            return 0;
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
