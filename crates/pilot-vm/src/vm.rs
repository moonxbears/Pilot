//! Register-based virtual machine (`PilVirtualMachine`).
//!
//! Execution stops at the first fault. A fault records the message and the
//! source span of the faulting instruction in `PilVirtualMachine::fault`;
//! `run` then returns `PilotVal::NULL` and callers decide how to surface it.

use crate::instruction::{Instruction, Opcode};
use crate::ops;
use crate::pilc::PilcBinary;
use pilot_rt::PilotVal;
use pilot_span::ByteSpan;
use std::collections::HashMap;

pub type VmHook = Box<dyn FnMut(usize, Instruction, &[PilotVal; 256])>;
pub type EngineIntrinsicHandler = Box<dyn FnMut(&str, &[PilotVal]) -> Result<PilotVal, String>>;

/// A runtime failure: what went wrong and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VmFault {
    pub message: String,
    pub pc: usize,
    pub span: Option<ByteSpan>,
}

impl std::fmt::Display for VmFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.span {
            Some(span) => write!(f, "{} (bytes {}..{})", self.message, span.start, span.end),
            None => write!(f, "{} (pc {})", self.message, self.pc),
        }
    }
}

pub struct PilVirtualMachine {
    pub registers: [PilotVal; 256],
    pub constants: Vec<PilotVal>,
    pub instructions: Vec<Instruction>,
    pub pc_spans: Vec<ByteSpan>,
    pub reg_symbols: Vec<(u8, u32, u32)>,
    pub pc: usize,
    pub call_stack: Vec<(usize, usize, u16, u8)>,
    pub reg_stack: Vec<PilotVal>,
    pub heap_maps: Vec<HashMap<String, PilotVal>>,
    pub heap_lists: Vec<Vec<PilotVal>>,
    pub hook: Option<VmHook>,
    pub engine_handler: Option<EngineIntrinsicHandler>,
    pub shared_symbols: Option<std::sync::Arc<HashMap<u32, String>>>,
    pub fuel_limit: Option<u64>,
    pub fuel_used: u64,
    pub fuel_exhausted: bool,
    pub fault: Option<VmFault>,
}

impl Default for PilVirtualMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// Upper bound on nested calls before the VM faults instead of exhausting memory.
const MAX_CALL_DEPTH: usize = 10_000;

impl PilVirtualMachine {
    pub fn new() -> Self {
        Self {
            registers: [PilotVal::NULL; 256],
            constants: Vec::new(),
            instructions: Vec::new(),
            pc_spans: Vec::new(),
            reg_symbols: Vec::new(),
            pc: 0,
            call_stack: Vec::with_capacity(64),
            reg_stack: Vec::with_capacity(512),
            heap_maps: Vec::new(),
            heap_lists: Vec::new(),
            hook: None,
            engine_handler: None,
            shared_symbols: None,
            fuel_limit: None,
            fuel_used: 0,
            fuel_exhausted: false,
            fault: None,
        }
    }

    pub fn load_binary(&mut self, b: PilcBinary) {
        self.constants = b.constants;
        self.instructions = b.instructions;
        self.pc_spans = b.pc_spans;
        self.reg_symbols = b.reg_symbols;
        self.pc = 0;
        self.call_stack.clear();
        self.reg_stack.clear();
        self.heap_lists.clear();
        self.registers = [PilotVal::NULL; 256];
        self.fuel_used = 0;
        self.fuel_exhausted = false;
        self.fault = None;
        // Heap record slot 0 is the program environment record (`ENV`).
        if self.heap_maps.is_empty() {
            self.heap_maps.push(HashMap::new());
        }
    }

    pub fn set_hook(&mut self, hook: Option<VmHook>) {
        self.hook = hook;
    }

    #[inline]
    pub fn current_span(&self) -> Option<ByteSpan> {
        self.pc_spans
            .get(self.pc)
            .copied()
            .filter(|s| s.start > 0 || s.end > 0)
    }

    /// Records a fault for the instruction at `pc`.
    fn raise(&mut self, pc: usize, message: String) {
        let span = self
            .pc_spans
            .get(pc)
            .copied()
            .filter(|s| s.start > 0 || s.end > 0);
        self.fault = Some(VmFault { message, pc, span });
    }

    #[inline]
    fn set(&mut self, reg: usize, v: PilotVal) {
        self.registers[reg] = v;
    }

    pub fn run(&mut self) -> PilotVal {
        match self.execute() {
            Ok(v) => v,
            Err(message) => {
                let pc = self.pc.saturating_sub(1);
                self.raise(pc, message);
                PilotVal::NULL
            }
        }
    }

    /// Like `run`, but returns the fault instead of leaving it in `self.fault`.
    pub fn run_checked(&mut self) -> Result<PilotVal, VmFault> {
        let v = self.run();
        match self.fault.take() {
            Some(fault) => Err(fault),
            None => Ok(v),
        }
    }

    fn execute(&mut self) -> Result<PilotVal, String> {
        let code_len = self.instructions.len();
        while self.pc < code_len {
            if let Some(limit) = self.fuel_limit {
                self.fuel_used += 1;
                if self.fuel_used > limit {
                    self.fuel_exhausted = true;
                    return Ok(PilotVal::NULL);
                }
            }
            let ins = self.instructions[self.pc];
            if let Some(ref mut hook) = self.hook {
                hook(self.pc, ins, &self.registers);
            }
            self.pc += 1;
            let (dst, src1, src2) = (ins.dst() as usize, ins.src1() as usize, ins.src2() as usize);
            let (r1, r2) = (self.registers[src1], self.registers[src2]);
            match ins.opcode() {
                Opcode::Nop => {}
                Opcode::Add => {
                    let v = if r1.is_str() || r2.is_str() {
                        let mut s = self.display(r1);
                        s.push_str(&self.display(r2));
                        PilotVal::from_str(&s)
                    } else {
                        ops::arith(Opcode::Add, r1, r2)?
                    };
                    self.set(dst, v);
                }
                op @ (Opcode::Sub
                | Opcode::Mul
                | Opcode::Div
                | Opcode::Rem
                | Opcode::Pow
                | Opcode::FloorDiv) => {
                    let v = ops::arith(op, r1, r2)?;
                    self.set(dst, v);
                }
                op @ (Opcode::BitAnd | Opcode::BitOr | Opcode::Xor | Opcode::Shl | Opcode::Shr) => {
                    let v = ops::bitwise(op, r1, r2)?;
                    self.set(dst, v);
                }
                op @ (Opcode::CmpEq
                | Opcode::CmpNe
                | Opcode::CmpLt
                | Opcode::CmpLe
                | Opcode::CmpGt
                | Opcode::CmpGe) => {
                    let v = ops::compare(op, r1, r2)?;
                    self.set(dst, v);
                }
                op @ (Opcode::Neg | Opcode::Not | Opcode::BitNot) => {
                    let v = ops::unary(op, r1)?;
                    self.set(dst, v);
                }
                Opcode::LoadConst => {
                    let v = self
                        .constants
                        .get(ins.wide() as usize)
                        .copied()
                        .ok_or_else(|| format!("constant #{} out of range", ins.wide()))?;
                    self.set(dst, v);
                }
                Opcode::Move => self.set(dst, r1),
                Opcode::Jump => self.pc = ins.wide() as usize,
                Opcode::JumpIfTrue => {
                    if ops::truth(r1)? {
                        self.pc = ins.branch_target() as usize;
                    }
                }
                Opcode::JumpIfFalse => {
                    if !ops::truth(r1)? {
                        self.pc = ins.branch_target() as usize;
                    }
                }
                Opcode::JumpIfNull => {
                    if r1.is_null() {
                        self.pc = ins.branch_target() as usize;
                    }
                }
                Opcode::JumpIfNotNull => {
                    if !r1.is_null() {
                        self.pc = ins.branch_target() as usize;
                    }
                }
                Opcode::Call => {
                    if self.call_stack.len() >= MAX_CALL_DEPTH {
                        return Err(format!("call depth exceeded {MAX_CALL_DEPTH} frames"));
                    }
                    let act = self
                        .registers
                        .iter()
                        .rposition(|r| !r.is_null())
                        .map_or(0, |i| i + 1);
                    let st = self.reg_stack.len();
                    self.reg_stack.extend_from_slice(&self.registers[..act]);
                    self.call_stack.push((self.pc, st, act as u16, ins.dst()));
                    self.pc = ins.wide() as usize;
                }
                Opcode::Ret => {
                    let ret = if src1 != 0 { r1 } else { self.registers[0] };
                    let Some((ret_pc, st, act, d)) = self.call_stack.pop() else {
                        return Ok(ret);
                    };
                    let n = act as usize;
                    self.registers[..n].copy_from_slice(&self.reg_stack[st..st + n]);
                    self.registers[n..].fill(PilotVal::NULL);
                    self.reg_stack.truncate(st);
                    self.set(d as usize, ret);
                    self.pc = ret_pc;
                }
                Opcode::EngineIntrinsic => {
                    let args: Vec<PilotVal> = self.registers[1..=src2.min(255)].to_vec();
                    let hook = r1
                        .with_str(|s| s.to_string())
                        .ok_or_else(|| format!("intrinsic name must be a string, found {}", r1.kind_label()))?;
                    let v = self.intrinsic(&hook, &args[..src2])?;
                    self.set(dst, v);
                }
                Opcode::Trap => {
                    let msg = self
                        .constants
                        .get(ins.wide() as usize)
                        .map(|c| c.to_string_repr())
                        .unwrap_or_else(|| "trap".to_string());
                    return Err(msg);
                }
                Opcode::Invalid => {
                    return Err(format!("invalid opcode byte {:#04x}", ins.0 >> 24));
                }
                Opcode::Halt => break,
            }
        }
        Ok(self.registers[0])
    }
}
