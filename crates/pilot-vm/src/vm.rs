//! Zero-Allocation Register-Based Virtual Machine Engine (`PilVirtualMachine`).

use std::collections::HashMap;
use pilot_rt::PilotVal;
use pilot_span::ByteSpan;
use crate::instruction::{Instruction, Opcode};
use crate::pilc::PilcBinary;

pub type VmHook = Box<dyn FnMut(usize, Instruction, &[PilotVal; 256])>;
pub type EngineIntrinsicHandler = Box<dyn FnMut(&str, &[PilotVal]) -> Result<PilotVal, String>>;

pub struct PilVirtualMachine {
    pub registers: [PilotVal; 256],
    pub constants: Vec<PilotVal>,
    pub instructions: Vec<Instruction>,
    pub pc_spans: Vec<ByteSpan>,
    pub reg_symbols: Vec<(u8, u32, u32)>,
    pub pc: usize,
    pub call_stack: Vec<(usize, [PilotVal; 256], u8)>,
    pub heap_maps: Vec<HashMap<String, PilotVal>>,
    pub hook: Option<VmHook>,
    pub engine_handler: Option<EngineIntrinsicHandler>,
    pub shared_symbols: Option<std::sync::Arc<HashMap<u32, String>>>,
}

impl Default for PilVirtualMachine { fn default() -> Self { Self::new() } }
#[inline] fn is_str(v: PilotVal) -> bool { v.is_sso_str() || v.tag() == 9 }

impl PilVirtualMachine {
    pub fn new() -> Self {
        Self {
            registers: [PilotVal::NULL; 256], constants: Vec::new(), instructions: Vec::new(),
            pc_spans: Vec::new(), reg_symbols: Vec::new(), pc: 0,
            call_stack: Vec::with_capacity(64), heap_maps: Vec::new(), hook: None, engine_handler: None,
            shared_symbols: None,
        }
    }

    pub fn load_binary(&mut self, b: PilcBinary) {
        self.constants = b.constants; self.instructions = b.instructions;
        self.pc_spans = b.pc_spans; self.reg_symbols = b.reg_symbols;
        self.pc = 0; self.call_stack.clear(); self.registers = [PilotVal::NULL; 256];
    }

    pub fn set_hook(&mut self, hook: Option<VmHook>) { self.hook = hook; }
    #[inline] pub fn current_span(&self) -> Option<ByteSpan> { self.pc_spans.get(self.pc).copied().filter(|s| s.start > 0 || s.end > 0) }

    pub fn run(&mut self) -> PilotVal {
        let code_len = self.instructions.len();
        while self.pc < code_len {
            let ins = unsafe { *self.instructions.get_unchecked(self.pc) };
            if let Some(ref mut hook) = self.hook { hook(self.pc, ins, &self.registers); }
            self.pc += 1;
            let (dst, src1, src2) = (ins.dst() as usize, ins.src1() as usize, ins.src2() as usize);
            let (r1, r2) = unsafe { (*self.registers.get_unchecked(src1), *self.registers.get_unchecked(src2)) };
            match ins.opcode() {
                Opcode::Nop => {}
                Opcode::AddI64 => {
                    let v = if is_str(r1) || is_str(r2) { PilotVal::from_str(&format!("{}{}", r1.to_string_repr(), r2.to_string_repr())) }
                    else { PilotVal::from_int(r1.as_int().unwrap_or(0).wrapping_add(r2.as_int().unwrap_or(0))) };
                    unsafe { *self.registers.get_unchecked_mut(dst) = v };
                }
                Opcode::SubI64 => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(r1.as_int().unwrap_or(0).wrapping_sub(r2.as_int().unwrap_or(0))) },
                Opcode::MulI64 => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(r1.as_int().unwrap_or(0).wrapping_mul(r2.as_int().unwrap_or(0))) },
                Opcode::DivI64 => { let b = r2.as_int().unwrap_or(1); unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(if b != 0 { r1.as_int().unwrap_or(0) / b } else { 0 }) }; }
                Opcode::RemI64 => { let b = r2.as_int().unwrap_or(1); unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(if b != 0 { r1.as_int().unwrap_or(0) % b } else { 0 }) }; }
                Opcode::AddF64 => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_float(r1.as_float().unwrap_or(0.0) + r2.as_float().unwrap_or(0.0)) },
                Opcode::SubF64 => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_float(r1.as_float().unwrap_or(0.0) - r2.as_float().unwrap_or(0.0)) },
                Opcode::MulF64 => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_float(r1.as_float().unwrap_or(0.0) * r2.as_float().unwrap_or(0.0)) },
                Opcode::DivF64 => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_float(r1.as_float().unwrap_or(0.0) / r2.as_float().unwrap_or(1.0)) },
                Opcode::LoadConst => if let Some(&v) = self.constants.get((src1 << 8) | src2) { unsafe { *self.registers.get_unchecked_mut(dst) = v }; },
                Opcode::Move => unsafe { *self.registers.get_unchecked_mut(dst) = r1 },
                Opcode::CmpEq => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(if is_str(r1) && is_str(r2) { r1.with_str(|a| r2.with_str(|b| a == b).unwrap_or(false)).unwrap_or(false) } else if is_str(r1) || is_str(r2) { r1.to_string_repr() == r2.to_string_repr() } else { r1 == r2 }) },
                Opcode::CmpNe => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(if is_str(r1) && is_str(r2) { r1.with_str(|a| r2.with_str(|b| a != b).unwrap_or(true)).unwrap_or(true) } else if is_str(r1) || is_str(r2) { r1.to_string_repr() != r2.to_string_repr() } else { r1 != r2 }) },
                Opcode::CmpLt => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(if is_str(r1) || is_str(r2) { r1.to_string_repr() < r2.to_string_repr() } else { r1.as_int().unwrap_or(0) < r2.as_int().unwrap_or(0) }) },
                Opcode::CmpGt => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(if is_str(r1) || is_str(r2) { r1.to_string_repr() > r2.to_string_repr() } else { r1.as_int().unwrap_or(0) > r2.as_int().unwrap_or(0) }) },
                Opcode::CmpLe => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(if is_str(r1) || is_str(r2) { r1.to_string_repr() <= r2.to_string_repr() } else { r1.as_int().unwrap_or(0) <= r2.as_int().unwrap_or(0) }) },
                Opcode::CmpGe => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(if is_str(r1) || is_str(r2) { r1.to_string_repr() >= r2.to_string_repr() } else { r1.as_int().unwrap_or(0) >= r2.as_int().unwrap_or(0) }) },
                Opcode::NegI64 => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(-r1.as_int().unwrap_or(0)) },
                Opcode::NegF64 => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_float(-r1.as_float().unwrap_or(0.0)) },
                Opcode::Not => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_bool(!r1.as_bool().unwrap_or(false)) },
                Opcode::And => unsafe { *self.registers.get_unchecked_mut(dst) = if let (Some(a), Some(b)) = (r1.as_int(), r2.as_int()) { PilotVal::from_int(a & b) } else { PilotVal::from_bool(r1.as_bool().unwrap_or(false) && r2.as_bool().unwrap_or(false)) } },
                Opcode::Or => unsafe { *self.registers.get_unchecked_mut(dst) = if let (Some(a), Some(b)) = (r1.as_int(), r2.as_int()) { PilotVal::from_int(a | b) } else { PilotVal::from_bool(r1.as_bool().unwrap_or(false) || r2.as_bool().unwrap_or(false)) } },
                Opcode::Xor => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(r1.as_int().unwrap_or(0) ^ r2.as_int().unwrap_or(0)) },
                Opcode::Shl => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(r1.as_int().unwrap_or(0) << r2.as_int().unwrap_or(0)) },
                Opcode::Shr => unsafe { *self.registers.get_unchecked_mut(dst) = PilotVal::from_int(r1.as_int().unwrap_or(0) >> r2.as_int().unwrap_or(0)) },
                Opcode::Jump => self.pc = (src1 << 8) | src2,
                Opcode::JumpIfTrue => if r1.as_bool().unwrap_or_else(|| r1.as_int().map_or(false, |i| i != 0)) { self.pc = (dst << 8) | src2; },
                Opcode::JumpIfFalse => if !r1.as_bool().unwrap_or_else(|| r1.as_int().map_or(false, |i| i != 0)) { self.pc = (dst << 8) | src2; },
                Opcode::Call => { self.call_stack.push((self.pc, self.registers, ins.dst())); self.pc = (src1 << 8) | src2; }
                Opcode::Ret => {
                    let ret = if src1 != 0 { r1 } else { unsafe { *self.registers.get_unchecked(0) } };
                    if let Some((ret_pc, saved, d)) = self.call_stack.pop() { self.registers = saved; unsafe { *self.registers.get_unchecked_mut(d as usize) = ret }; self.pc = ret_pc; } else { return ret; }
                }
                Opcode::EngineIntrinsic => {
                    let mut sbuf = [PilotVal::NULL; 16];
                    let overflow: Vec<PilotVal>;
                    let args: &[PilotVal] = if src2 <= 16 { sbuf[..src2].copy_from_slice(&self.registers[1..=src2]); &sbuf[..src2] } else { overflow = self.registers[1..=src2.min(255)].to_vec(); &overflow };
                    let mut run_op = |hook_name: &str| -> PilotVal {
                        match hook_name {
                            "rt_print" => {
                                if let Some(a) = args.first() {
                                    if let Some(m) = a.as_int().and_then(|i| self.heap_maps.get(i.max(0) as usize)) {
                                        let mut items: Vec<String> = m.iter().map(|(k, v)| v.as_int().and_then(|i| self.heap_maps.get(i.max(0) as usize)).map_or_else(|| format!("{k}: {v:?}"), |s| { let mut p: Vec<String> = s.iter().map(|(sk, sv)| format!("{sk}: {sv:?}")).collect(); p.sort(); format!("{k}: {{ {} }}", p.join(", ")) })).collect();
                                        items.sort(); println!("{{ {} }}", items.join(", "));
                                    } else { println!("{}", a.to_string_repr()); }
                                }
                                PilotVal::NULL
                            }
                            "rt_assert" => { if !args.first().map_or(false, |v| v.as_bool().unwrap_or(false) || v.as_int().unwrap_or(0) != 0) { panic!("Assertion failed in PilVM execution"); } PilotVal::NULL }
                            "str_len" if !args.is_empty() => PilotVal::from_int(if args[0].is_null() { 0 } else { args[0].with_str(|s| s.len() as i64).unwrap_or_else(|| args[0].to_string_repr().len() as i64) }),
                            "str_byte_at" if args.len() >= 2 => { let idx = args[1].as_int().unwrap_or(0).max(0) as usize; PilotVal::from_int(if args[0].is_null() { 0 } else { args[0].with_str(|s| s.as_bytes().get(idx).copied().unwrap_or(0) as i64).unwrap_or_else(|| args[0].to_string_repr().as_bytes().get(idx).copied().unwrap_or(0) as i64) }) }
                            "str_slice" if args.len() >= 3 => {
                                let slice_fn = |s: &str| { let (mut a, mut b) = ((args[1].as_int().unwrap_or(0).max(0) as usize).min(s.len()), (args[2].as_int().unwrap_or(0).max(0) as usize).min(s.len())); while !s.is_char_boundary(a) && a < s.len() { a += 1; } while !s.is_char_boundary(b) && b < s.len() { b += 1; } PilotVal::from_str(&s[a..b.max(a)]) };
                                if args[0].is_null() { PilotVal::from_str("") } else { args[0].with_str(slice_fn).unwrap_or_else(|| slice_fn(&args[0].to_string_repr())) }
                            }
                            "str_find" if args.len() >= 3 => {
                                let st = args[2].as_int().unwrap_or(0);
                                if st < 0 || args[0].is_null() || args[1].is_null() { PilotVal::from_int(-1) } else { let find_fn = |h: &str, nd: &str| { let mut u = (st as usize).min(h.len()); while !h.is_char_boundary(u) && u < h.len() { u += 1; } h[u..].find(nd).map_or(-1, |o| (u + o) as i64) }; PilotVal::from_int(args[0].with_str(|h| args[1].with_str(|nd| find_fn(h, nd)).unwrap_or_else(|| find_fn(h, &args[1].to_string_repr()))).unwrap_or(-1)) }
                            }
                            "map_new" => { let idx = self.heap_maps.len(); self.heap_maps.push(HashMap::new()); PilotVal::from_int(idx as i64) }
                            "map_set" if args.len() >= 3 => { let idx = args[0].as_int().unwrap_or(-1); if idx >= 0 && (idx as usize) < self.heap_maps.len() { self.heap_maps[idx as usize].insert(args[1].to_string_repr(), args[2]); } args[0] }
                            "map_get" if args.len() >= 2 => {
                                let idx = args[0].as_int().unwrap_or(-1);
                                if idx >= 0 && (idx as usize) < self.heap_maps.len() { let m = &self.heap_maps[idx as usize]; args[1].with_str(|k| m.get(k).copied()).unwrap_or_else(|| m.get(&args[1].to_string_repr()).copied()).unwrap_or(PilotVal::NULL) } else { PilotVal::NULL }
                            }
                            "map_has" if args.len() >= 2 => { let idx = args[0].as_int().unwrap_or(-1); if idx >= 0 && (idx as usize) < self.heap_maps.len() { let m = &self.heap_maps[idx as usize]; PilotVal::from_bool(args[1].with_str(|k| m.contains_key(k)).unwrap_or_else(|| m.contains_key(&args[1].to_string_repr()))) } else { PilotVal::FALSE } }
                            "is_null" if !args.is_empty() => PilotVal::from_bool(args[0].is_null()),
                            "sym_str" if !args.is_empty() => { let sid = args[0].as_int().unwrap_or(0) as u32; self.shared_symbols.as_ref().and_then(|s| s.get(&sid)).map_or_else(|| self.engine_handler.as_mut().and_then(|h| h(hook_name, args).ok()).unwrap_or(PilotVal::NULL), |s| PilotVal::from_str(s)) }
                            "get_env" => PilotVal::from_int(0),
                            _ => match self.engine_handler.as_mut().map(|h| h(hook_name, args)) { Some(Ok(v)) if !v.is_null() => v, Some(Err(_)) => PilotVal::NULL, _ => pilot_rt::PilotOsSyscall::vm_intrinsic(hook_name, args) },
                        }
                    };
                    let res = r1.with_str(&mut run_op).unwrap_or_else(|| run_op(&r1.to_string_repr()));
                    unsafe { *self.registers.get_unchecked_mut(dst) = res };
                }
                Opcode::Halt => break,
            }
        }
        unsafe { *self.registers.get_unchecked(0) }
    }
}
