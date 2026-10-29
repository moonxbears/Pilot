//! Milestone 34.3: Register-Based PilVM v3 Execution Engine.

use pilot_rt::PilotVal;
use crate::instruction::{Instruction, Opcode};
use crate::pilc::PilcBinary;

pub type VmHook = Box<dyn FnMut(usize, Instruction, &[PilotVal; 256])>;

pub struct PilVirtualMachine {
    pub registers: [PilotVal; 256],
    pub constants: Vec<PilotVal>,
    pub instructions: Vec<Instruction>,
    pub pc: usize,
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
            hook: None,
        }
    }

    pub fn load_binary(&mut self, binary: PilcBinary) {
        self.constants = binary.constants;
        self.instructions = binary.instructions;
        self.pc = 0;
        self.registers = [PilotVal::NULL; 256];
    }

    pub fn set_hook(&mut self, hook: Option<VmHook>) {
        self.hook = hook;
    }

    /// Executes instructions until `Ret` or `Halt`, returning the final register value in `r0`.
    pub fn run(&mut self) -> PilotVal {
        let code_len = self.instructions.len();

        while self.pc < code_len {
            let ins = self.instructions[self.pc];
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
                    let a = self.registers[src1].as_int().unwrap_or(0);
                    let b = self.registers[src2].as_int().unwrap_or(0);
                    self.registers[dst] = PilotVal::from_int(a.wrapping_add(b));
                }
                Opcode::SubI64 => {
                    let a = self.registers[src1].as_int().unwrap_or(0);
                    let b = self.registers[src2].as_int().unwrap_or(0);
                    self.registers[dst] = PilotVal::from_int(a.wrapping_sub(b));
                }
                Opcode::MulI64 => {
                    let a = self.registers[src1].as_int().unwrap_or(0);
                    let b = self.registers[src2].as_int().unwrap_or(0);
                    self.registers[dst] = PilotVal::from_int(a.wrapping_mul(b));
                }
                Opcode::DivI64 => {
                    let a = self.registers[src1].as_int().unwrap_or(0);
                    let b = self.registers[src2].as_int().unwrap_or(1);
                    self.registers[dst] = PilotVal::from_int(if b != 0 { a / b } else { 0 });
                }
                Opcode::AddF64 => {
                    let a = self.registers[src1].as_float().unwrap_or(0.0);
                    let b = self.registers[src2].as_float().unwrap_or(0.0);
                    self.registers[dst] = PilotVal::from_float(a + b);
                }
                Opcode::SubF64 => {
                    let a = self.registers[src1].as_float().unwrap_or(0.0);
                    let b = self.registers[src2].as_float().unwrap_or(0.0);
                    self.registers[dst] = PilotVal::from_float(a - b);
                }
                Opcode::MulF64 => {
                    let a = self.registers[src1].as_float().unwrap_or(0.0);
                    let b = self.registers[src2].as_float().unwrap_or(0.0);
                    self.registers[dst] = PilotVal::from_float(a * b);
                }
                Opcode::DivF64 => {
                    let a = self.registers[src1].as_float().unwrap_or(0.0);
                    let b = self.registers[src2].as_float().unwrap_or(1.0);
                    self.registers[dst] = PilotVal::from_float(a / b);
                }
                Opcode::LoadConst => {
                    let const_idx = ((src1 as u16) << 8 | (src2 as u16)) as usize;
                    if const_idx < self.constants.len() {
                        self.registers[dst] = self.constants[const_idx];
                    }
                }
                Opcode::Move => {
                    self.registers[dst] = self.registers[src1];
                }
                Opcode::CmpEq => {
                    let eq = self.registers[src1] == self.registers[src2];
                    self.registers[dst] = PilotVal::from_bool(eq);
                }
                Opcode::CmpLt => {
                    let a = self.registers[src1].as_int().unwrap_or(0);
                    let b = self.registers[src2].as_int().unwrap_or(0);
                    self.registers[dst] = PilotVal::from_bool(a < b);
                }
                Opcode::Jump => {
                    let target = ((src1 as u16) << 8 | (src2 as u16)) as usize;
                    self.pc = target;
                }
                Opcode::JumpIfTrue => {
                    if self.registers[src1].as_bool().unwrap_or(false) {
                        let target = ((dst as u16) << 8 | (src2 as u16)) as usize;
                        self.pc = target;
                    }
                }
                Opcode::JumpIfFalse => {
                    if !self.registers[src1].as_bool().unwrap_or(false) {
                        let target = ((dst as u16) << 8 | (src2 as u16)) as usize;
                        self.pc = target;
                    }
                }
                Opcode::Call => {
                    // Function calls in register VM
                }
                Opcode::Ret => {
                    return self.registers[src1];
                }
                Opcode::Halt => {
                    break;
                }
            }
        }

        self.registers[0]
    }
}
