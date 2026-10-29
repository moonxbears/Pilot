//! Milestone 34.4: Zero-Allocation Register-Based Virtual Machine Engine (`PilVirtualMachine`).

use pilot_rt::PilotVal;
use crate::instruction::{Instruction, Opcode};
use crate::pilc::PilcBinary;

pub type VmHook = Box<dyn FnMut(usize, Instruction, &[PilotVal; 256])>;

pub struct PilVirtualMachine {
    pub registers: [PilotVal; 256],
    pub constants: Vec<PilotVal>,
    pub instructions: Vec<Instruction>,
    pub pc: usize,
    pub call_stack: Vec<(usize, [PilotVal; 256], u8)>,
    pub hook: Option<VmHook>,
}

impl Default for PilVirtualMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl PilVirtualMachine {
    pub fn new() -> Self {
        Self {
            registers: [PilotVal::NULL; 256],
            constants: Vec::new(),
            instructions: Vec::new(),
            pc: 0,
            call_stack: Vec::with_capacity(64),
            hook: None,
        }
    }

    pub fn load_binary(&mut self, binary: PilcBinary) {
        self.constants = binary.constants;
        self.instructions = binary.instructions;
        self.pc = 0;
        self.call_stack.clear();
        self.registers = [PilotVal::NULL; 256];
    }

    pub fn set_hook(&mut self, hook: Option<VmHook>) {
        self.hook = hook;
    }

    pub fn run(&mut self) -> PilotVal {
        let code_len = self.instructions.len();

        while self.pc < code_len {
            let ins = unsafe { *self.instructions.get_unchecked(self.pc) };
            if let Some(ref mut hook) = self.hook {
                hook(self.pc, ins, &self.registers);
            }

            self.pc += 1;

            let dst = ins.dst() as usize;
            let src1 = ins.src1() as usize;
            let src2 = ins.src2() as usize;

            match ins.opcode() {
                Opcode::Nop => {}
                Opcode::AddI64 => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_int().unwrap_or(0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_int().unwrap_or(0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(a.wrapping_add(b)) };
                }
                Opcode::SubI64 => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_int().unwrap_or(0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_int().unwrap_or(0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(a.wrapping_sub(b)) };
                }
                Opcode::MulI64 => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_int().unwrap_or(0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_int().unwrap_or(0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(a.wrapping_mul(b)) };
                }
                Opcode::DivI64 => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_int().unwrap_or(0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_int().unwrap_or(1) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(if b != 0 { a / b } else { 0 }) };
                }
                Opcode::AddF64 => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_float().unwrap_or(0.0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_float().unwrap_or(0.0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_float(a + b) };
                }
                Opcode::SubF64 => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_float().unwrap_or(0.0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_float().unwrap_or(0.0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_float(a - b) };
                }
                Opcode::MulF64 => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_float().unwrap_or(0.0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_float().unwrap_or(0.0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_float(a * b) };
                }
                Opcode::DivF64 => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_float().unwrap_or(0.0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_float().unwrap_or(1.0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_float(a / b) };
                }
                Opcode::LoadConst => {
                    let const_idx = ((src1 as u16) << 8 | (src2 as u16)) as usize;
                    if const_idx < self.constants.len() {
                        unsafe { *self.registers.get_unchecked_mut(dst) = *self.constants.get_unchecked(const_idx) };
                    }
                }
                Opcode::Move => {
                    unsafe { *self.registers.get_unchecked_mut(dst) = *self.registers.get_unchecked(src1) };
                }
                Opcode::CmpEq => {
                    let eq = unsafe { *self.registers.get_unchecked(src1) == *self.registers.get_unchecked(src2) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(eq) };
                }
                Opcode::CmpLt => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_int().unwrap_or(0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_int().unwrap_or(0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(a < b) };
                }
                Opcode::CmpGt => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_int().unwrap_or(0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_int().unwrap_or(0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(a > b) };
                }
                Opcode::CmpLe => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_int().unwrap_or(0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_int().unwrap_or(0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(a <= b) };
                }
                Opcode::CmpGe => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_int().unwrap_or(0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_int().unwrap_or(0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(a >= b) };
                }
                Opcode::CmpNe => {
                    let ne = unsafe { *self.registers.get_unchecked(src1) != *self.registers.get_unchecked(src2) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(ne) };
                }
                Opcode::RemI64 => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_int().unwrap_or(0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_int().unwrap_or(1) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(if b != 0 { a % b } else { 0 }) };
                }
                Opcode::NegI64 => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_int().unwrap_or(0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(-a) };
                }
                Opcode::NegF64 => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_float().unwrap_or(0.0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_float(-a) };
                }
                Opcode::Not => {
                    let b = unsafe { self.registers.get_unchecked(src1).as_bool().unwrap_or(false) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(!b) };
                }
                Opcode::And => {
                    let r1 = unsafe { *self.registers.get_unchecked(src1) };
                    let r2 = unsafe { *self.registers.get_unchecked(src2) };
                    if let (Some(a), Some(b)) = (r1.as_int(), r2.as_int()) {
                        unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(a & b) };
                    } else {
                        let a = r1.as_bool().unwrap_or(false);
                        let b = r2.as_bool().unwrap_or(false);
                        unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(a && b) };
                    }
                }
                Opcode::Or => {
                    let r1 = unsafe { *self.registers.get_unchecked(src1) };
                    let r2 = unsafe { *self.registers.get_unchecked(src2) };
                    if let (Some(a), Some(b)) = (r1.as_int(), r2.as_int()) {
                        unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(a | b) };
                    } else {
                        let a = r1.as_bool().unwrap_or(false);
                        let b = r2.as_bool().unwrap_or(false);
                        unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(a || b) };
                    }
                }
                Opcode::Print => {
                    let val = unsafe { *self.registers.get_unchecked(src1) };
                    if let Some(i) = val.as_int() {
                        println!("{}", i);
                    } else if let Some(f) = val.as_float() {
                        println!("{}", f);
                    } else if let Some(b) = val.as_bool() {
                        println!("{}", b);
                    } else {
                        println!("{:?}", val);
                    }
                }
                Opcode::Assert => {
                    let val = unsafe { *self.registers.get_unchecked(src1) };
                    let ok = val.as_bool().unwrap_or(false)
                        || val.as_int().unwrap_or(0) != 0;
                    if !ok {
                        panic!("Assertion failed in PilVM execution");
                    }
                }
                Opcode::Xor => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_int().unwrap_or(0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_int().unwrap_or(0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(a ^ b) };
                }
                Opcode::Shl => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_int().unwrap_or(0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_int().unwrap_or(0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(a << b) };
                }
                Opcode::Shr => {
                    let a = unsafe { self.registers.get_unchecked(src1).as_int().unwrap_or(0) };
                    let b = unsafe { self.registers.get_unchecked(src2).as_int().unwrap_or(0) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(a >> b) };
                }
                Opcode::Jump => {
                    let target = ((src1 as u16) << 8 | (src2 as u16)) as usize;
                    self.pc = target;
                }
                Opcode::JumpIfTrue => {
                    let val = unsafe { *self.registers.get_unchecked(src1) };
                    let cond = val.as_bool().unwrap_or_else(|| val.as_int().map_or(false, |i| i != 0));
                    if cond {
                        let target = ((dst as u16) << 8 | (src2 as u16)) as usize;
                        self.pc = target;
                    }
                }
                Opcode::JumpIfFalse => {
                    let val = unsafe { *self.registers.get_unchecked(src1) };
                    let cond = val.as_bool().unwrap_or_else(|| val.as_int().map_or(false, |i| i != 0));
                    if !cond {
                        let target = ((dst as u16) << 8 | (src2 as u16)) as usize;
                        self.pc = target;
                    }
                }
                Opcode::Call => {
                    let target = ((src1 as u16) << 8 | (src2 as u16)) as usize;
                    self.call_stack.push((self.pc, self.registers, ins.dst()));
                    self.pc = target;
                }
                Opcode::Ret => {
                    let ret_val = if src1 != 0 {
                        unsafe { *self.registers.get_unchecked(src1) }
                    } else {
                        unsafe { *self.registers.get_unchecked(0) }
                    };
                    if let Some((ret_pc, saved_regs, dst_reg)) = self.call_stack.pop() {
                        self.registers = saved_regs;
                        unsafe { *self.registers.get_unchecked_mut(dst_reg as usize) = ret_val };
                        self.pc = ret_pc;
                    } else {
                        return ret_val;
                    }
                }
                Opcode::Halt => {
                    break;
                }
            }
        }

        unsafe { *self.registers.get_unchecked(0) }
    }
}
