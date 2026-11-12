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

/// Host operations the embedding program provides (`crate::host::HostValue`).
/// Returns `None` for an operation the host does not provide.
pub type HostOpHandler =
    Box<dyn FnMut(&str, &[crate::host::HostValue]) -> Option<Result<crate::host::HostValue, String>>>;

/// A runtime failure: what went wrong and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VmFault {
    pub message: String,
    pub pc: usize,
    pub span: Option<ByteSpan>,
    /// The calls that led here, innermost first, at most `MAX_TRACE` of them.
    pub trace: Vec<ByteSpan>,
    /// How many calls with a source position were active; it can exceed
    /// `trace.len()`.
    pub depth: usize,
}

/// The most calls a fault's trace keeps.
pub const MAX_TRACE: usize = 32;

impl std::fmt::Display for VmFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.span {
            Some(span) => write!(f, "{} (bytes {}..{})", self.message, span.start, span.end),
            None => write!(f, "{} (pc {})", self.message, self.pc),
        }
    }
}

pub struct PilVirtualMachine {
    /// Every register from this one up holds `NULL`, so calls save and
    /// returns clear only the registers below it.
    pub(crate) reg_top: usize,
    /// The strings this VM made while running (`strings`).
    pub strings: crate::strings::RuntimeStrings,
    /// Dead slots and collection bookkeeping (`collect`).
    pub heap: crate::collect::HeapState,
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
    /// The struct each record was built as, by record slot. Kept beside the
    /// record so it never shows up as a field.
    pub record_types: HashMap<usize, PilotVal>,
    /// Module-level bindings, by name.
    pub globals: HashMap<String, PilotVal>,
    pub hook: Option<VmHook>,
    pub engine_handler: Option<EngineIntrinsicHandler>,
    /// Host operations, tried before the engine handler.
    pub host_ops: Option<HostOpHandler>,
    pub shared_symbols: Option<std::sync::Arc<HashMap<u32, String>>>,
    pub fuel_limit: Option<u64>,
    pub fuel_used: u64,
    pub fuel_exhausted: bool,
    pub fault: Option<VmFault>,
    /// The arguments after `--` on the command line (`proc_args()`).
    pub script_args: Vec<String>,
    /// Child processes started with `proc_spawn`, by handle; `None` once
    /// waited for.
    pub children: Vec<Option<crate::host::Child>>,
    /// Generator frames, by handle (`vm_gen`).
    pub generators: Vec<crate::vm_gen::GenFrame>,
    /// The generator bodies running, innermost last: the call depth each
    /// was entered at, and its frame.
    pub gen_stack: Vec<(usize, usize)>,
}

impl Default for PilVirtualMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// Upper bound on nested calls before the VM faults instead of exhausting memory.
const MAX_CALL_DEPTH: usize = 10_000;

/// Intrinsic calls with at most this many arguments pass them on the stack.
const INLINE_ARGS: usize = 16;

/// The names of the generator intrinsics (`vm_gen`) start with this.
const GEN_PREFIX: &str = "gen_";

impl PilVirtualMachine {
    pub fn new() -> Self {
        Self {
            reg_top: 0,
            strings: Default::default(),
            heap: crate::collect::HeapState::new(),
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
            record_types: HashMap::new(),
            globals: HashMap::new(),
            hook: None,
            engine_handler: None,
            host_ops: None,
            script_args: Vec::new(),
            children: Vec::new(),
            generators: Vec::new(),
            gen_stack: Vec::new(),
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
        self.globals.clear();
        self.generators.clear();
        self.gen_stack.clear();
        self.registers = [PilotVal::NULL; 256];
        self.reg_top = 0;
        self.fuel_used = 0;
        self.fuel_exhausted = false;
        self.fault = None;
        // A loaded program starts with an empty heap: records left by the
        // program before would leak and could hold handles to its lists.
        // Heap record slot 0 is the program environment record (`ENV`).
        self.heap_maps.clear();
        self.heap_maps.push(HashMap::new());
        self.record_types.clear();
        self.heap.reset();
        // The host may still hold a value the last program returned; its
        // strings are kept for good rather than collected under it.
        drop(std::mem::take(&mut self.strings));
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

    /// Records a fault for the instruction at `pc`, with the calls that led
    /// to it.
    fn raise(&mut self, pc: usize, message: String) {
        let span = self.span_at(pc);
        // A frame saves the address after its call instruction.
        let calls = self
            .call_stack
            .iter()
            .rev()
            .filter_map(|&(ret, ..)| self.span_at(ret.saturating_sub(1)));
        let depth = calls.clone().count();
        let trace = calls.take(MAX_TRACE).collect();
        self.fault = Some(VmFault { message, pc, span, trace, depth });
    }

    fn span_at(&self, pc: usize) -> Option<ByteSpan> {
        self.pc_spans
            .get(pc)
            .copied()
            .filter(|s| s.start > 0 || s.end > 0)
    }

    #[inline]
    pub(crate) fn set(&mut self, reg: usize, v: PilotVal) {
        self.registers[reg] = v;
        if reg >= self.reg_top {
            self.reg_top = reg + 1;
        }
    }

    pub fn run(&mut self) -> PilotVal {
        self.pin_host_slots();
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

    /// Saves the live registers and jumps to `entry`; `Ret` writes the result to `dst`.
    pub(crate) fn enter_call(&mut self, entry: usize, dst: u8) -> Result<(), String> {
        if self.call_stack.len() >= MAX_CALL_DEPTH {
            return Err(format!("call depth exceeded {MAX_CALL_DEPTH} frames"));
        }
        let act = self.active_registers();
        let st = self.reg_stack.len();
        self.reg_stack.extend_from_slice(&self.registers[..act]);
        self.call_stack.push((self.pc, st, act as u16, dst));
        self.pc = entry;
        Ok(())
    }

    /// Returns from the current frame with `ret`: the caller's registers come
    /// back and `ret` lands in its destination register.
    pub(crate) fn leave_frame(&mut self, ret: PilotVal) {
        let Some((ret_pc, st, act, d)) = self.call_stack.pop() else {
            return;
        };
        let n = act as usize;
        self.registers[..n].copy_from_slice(&self.reg_stack[st..st + n]);
        if self.reg_top > n {
            self.registers[n..self.reg_top].fill(PilotVal::NULL);
        }
        self.reg_top = n;
        self.reg_stack.truncate(st);
        self.set(d as usize, ret);
        self.pc = ret_pc;
    }

    /// `==`: lists (and tuples) compare element by element; everything else
    /// as `ops::equals`.
    pub(crate) fn values_equal(&self, a: PilotVal, b: PilotVal) -> bool {
        let list = |v: PilotVal| v.as_list_handle().and_then(|slot| self.heap_lists.get(slot));
        match (list(a), list(b)) {
            (Some(x), Some(y)) => {
                a == b || (x.len() == y.len() && x.iter().zip(y).all(|(&p, &q)| self.values_equal(p, q)))
            }
            _ => ops::equals(a, b),
        }
    }

    /// Entry point and arity of a function value.
    fn closure_target(&self, f: PilotVal) -> Result<(usize, usize), String> {
        let env = f
            .as_fn_handle()
            .filter(|&slot| !self.list_is_dead(slot))
            .and_then(|slot| self.heap_lists.get(slot))
            .ok_or_else(|| format!("{} is not callable", f.kind_label()))?;
        let field = |i: usize| env.get(i).and_then(|v| v.as_int()).and_then(|n| usize::try_from(n).ok());
        match (field(0), field(1)) {
            (Some(entry), Some(arity)) => Ok((entry, arity)),
            _ => Err("malformed function value".to_string()),
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
                op @ (Opcode::Add
                | Opcode::Sub
                | Opcode::Mul
                | Opcode::Div
                | Opcode::Rem
                | Opcode::Pow
                | Opcode::FloorDiv
                | Opcode::BitAnd
                | Opcode::BitOr
                | Opcode::Xor
                | Opcode::Shl
                | Opcode::Shr
                | Opcode::CmpEq
                | Opcode::CmpNe
                | Opcode::CmpLt
                | Opcode::CmpLe
                | Opcode::CmpGt
                | Opcode::CmpGe) => {
                    let v = self.binary_op(op, r1, r2)?;
                    self.set(dst, v);
                    if v.is_str() {
                        self.collect_if_due();
                    }
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
                Opcode::Call => self.enter_call(ins.wide() as usize, ins.dst())?,
                Opcode::CallClosure => {
                    let (entry, arity) = self.closure_target(r1)?;
                    if src2 != arity {
                        return Err(format!("function expects {arity} argument(s), got {src2}"));
                    }
                    // The callee finds its environment right after its arguments.
                    self.set(src2 + 1, r1);
                    self.enter_call(entry, ins.dst())?;
                }
                Opcode::Ret => {
                    let ret = if src1 != 0 { r1 } else { self.registers[0] };
                    let ret = self.gen_return(ret);
                    if self.call_stack.is_empty() {
                        return Ok(ret);
                    }
                    self.leave_frame(ret);
                }
                Opcode::EngineIntrinsic => {
                    // Neither the name nor the arguments are copied to the
                    // heap: intrinsics are the hottest calls the tools make.
                    let mut name_buf = [0u8; 14];
                    let hook = r1
                        .str_in(&mut name_buf)
                        .ok_or_else(|| format!("intrinsic name must be a string, found {}", r1.kind_label()))?;
                    let n = src2.min(255);
                    let mut small = [PilotVal::NULL; INLINE_ARGS];
                    let spilled: Vec<PilotVal>;
                    let args: &[PilotVal] = if n <= INLINE_ARGS {
                        small[..n].copy_from_slice(&self.registers[1..=n]);
                        &small[..n]
                    } else {
                        spilled = self.registers[1..=n].to_vec();
                        &spilled
                    };
                    if hook.starts_with(GEN_PREFIX) && self.gen_intrinsic(hook, ins, args)? {
                        continue;
                    }
                    let v = self.intrinsic(hook, args)?;
                    self.set(dst, v);
                    self.collect_if_due();
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
