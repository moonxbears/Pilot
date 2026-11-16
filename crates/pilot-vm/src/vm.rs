//! Register-based virtual machine (`PilVirtualMachine`).
//!
//! Execution stops at the first fault. A fault records the message and the
//! source span of the faulting instruction in `PilVirtualMachine::fault`;
//! `run` then returns `PilotVal::NULL` and callers decide how to surface it.

use crate::instruction::{Instruction, Opcode};
use crate::ops;
use crate::pilc::PilcBinary;
use crate::value::PilotVal;
use pilot_span::ByteSpan;
use std::collections::HashMap;

pub type VmHook = Box<dyn FnMut(usize, Instruction, &[PilotVal; 256])>;
pub type EngineIntrinsicHandler = Box<dyn FnMut(&str, &[PilotVal]) -> Result<PilotVal, String>>;

/// Host operations the embedding program provides (`crate::host::HostValue`).
/// Returns `None` for an operation the host does not provide.
pub type HostOpHandler = Box<
    dyn FnMut(&str, &[crate::host::HostValue]) -> Option<Result<crate::host::HostValue, String>>,
>;

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
    /// The registers of the running frame that may hold a value, as two
    /// ranges, because a function's locals sit low in its window and its
    /// scratch registers start at `SCRATCH_BASE`: `..reg_tops[0]` and
    /// `SCRATCH_BASE..reg_tops[1]`. Every other register holds `NULL`, so a
    /// return clears only these.
    pub(crate) reg_tops: [usize; 2],
    /// The strings this VM made while running (`strings`).
    pub strings: crate::strings::RuntimeStrings,
    /// Dead slots and collection bookkeeping (`collect`).
    pub heap: crate::collect::HeapState,
    /// The registers of every active frame, one window after another. The
    /// running frame's window starts at `base` and is `FRAME_REGS` wide; a
    /// call opens the next window just past the caller's live registers.
    pub registers: Vec<PilotVal>,
    /// Where the running frame's window starts in `registers`.
    pub(crate) base: usize,
    pub constants: Vec<PilotVal>,
    pub instructions: Vec<Instruction>,
    pub pc_spans: Vec<ByteSpan>,
    pub reg_symbols: Vec<(u8, u32, u32)>,
    pub pc: usize,
    /// One entry per active call: the return address, the caller's window
    /// start and live register count, and its destination register.
    pub call_stack: Vec<(usize, usize, [u16; 2], u8)>,
    pub heap_maps: Vec<crate::collect::Record>,
    pub heap_lists: Vec<Vec<PilotVal>>,
    /// The struct each record was built as, by record slot. Kept beside the
    /// record so it never shows up as a field.
    pub record_types: crate::collect::KeyMap<usize, PilotVal>,
    /// Module-level bindings, by name.
    pub globals: crate::collect::Record,
    pub hook: Option<VmHook>,
    pub engine_handler: Option<EngineIntrinsicHandler>,
    /// Host operations, tried before the engine handler.
    pub host_ops: Option<HostOpHandler>,
    pub shared_symbols: Option<std::sync::Arc<HashMap<u32, String>>>,
    pub fuel_limit: Option<u64>,
    pub fuel_used: u64,
    pub fuel_exhausted: bool,
    pub fault: Option<VmFault>,
    /// The arguments after `--` on the command line (`EnvCtl` sub-op `2`).
    pub script_args: Vec<String>,
    /// Unified host descriptor, process and dynamic-library state (`bridge`).
    pub bridge: crate::bridge::BridgeState,
    /// Generator frames, by handle (`vm_gen`).
    pub generators: Vec<crate::vm_gen::GenFrame>,
    /// The generator bodies running, innermost last: the call depth each
    /// was entered at, and its frame.
    pub gen_stack: Vec<(usize, usize)>,
    /// The final values of the `var ref` parameters of the call that just
    /// returned, by parameter position (`SetOut` / `GetOut`).
    pub outs: Vec<PilotVal>,
    /// The names in `heap::READS` as string values, in the same order.
    reads: [PilotVal; 2],
}

impl Default for PilVirtualMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// Upper bound on nested calls before the VM faults instead of exhausting memory.
const MAX_CALL_DEPTH: usize = 10_000;

/// Where a function's scratch registers start in its window.
const SCRATCH_BASE: usize = crate::FIRST_TEMP_REG as usize;

/// The registers one frame can address.
const FRAME_REGS: usize = 256;

/// The registers a call hands to the callee: the result register, the most
/// arguments a call stages (`MAX_CALL_ARGS`) and a closure's environment.
const CARRIED_REGS: usize = crate::MAX_CALL_ARGS + 2;

/// Intrinsic calls with at most this many arguments pass them on the stack.
const INLINE_ARGS: usize = 16;

impl PilVirtualMachine {
    pub fn new() -> Self {
        Self {
            reads: crate::heap::READS.map(PilotVal::from_str),
            reg_tops: [0, SCRATCH_BASE],
            strings: Default::default(),
            heap: crate::collect::HeapState::new(),
            registers: vec![PilotVal::NULL; FRAME_REGS],
            base: 0,
            constants: Vec::new(),
            instructions: Vec::new(),
            pc_spans: Vec::new(),
            reg_symbols: Vec::new(),
            pc: 0,
            call_stack: Vec::with_capacity(64),
            heap_maps: Vec::new(),
            heap_lists: Vec::new(),
            record_types: Default::default(),
            globals: Default::default(),
            hook: None,
            engine_handler: None,
            host_ops: None,
            script_args: Vec::new(),
            bridge: Default::default(),
            generators: Vec::new(),
            gen_stack: Vec::new(),
            outs: Vec::new(),
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
        self.heap_lists.clear();
        self.globals.clear();
        self.bridge = Default::default();
        self.generators.clear();
        self.gen_stack.clear();
        self.registers.clear();
        self.registers.resize(FRAME_REGS, PilotVal::NULL);
        self.base = 0;
        self.reg_tops = [0, SCRATCH_BASE];
        self.fuel_used = 0;
        self.fuel_exhausted = false;
        self.fault = None;
        // A loaded program starts with an empty heap: records left by the
        // program before would leak and could hold handles to its lists.
        // Heap record slot 0 is the program environment record (`ENV`).
        self.heap_maps.clear();
        self.heap_maps.push(Default::default());
        self.record_types.clear();
        self.heap.reset();
        // The host may still hold a value the last program returned; its
        // strings are kept for good rather than collected under it.
        drop(std::mem::take(&mut self.strings));
    }

    pub fn set_hook(&mut self, hook: Option<VmHook>) {
        self.hook = hook;
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
        self.fault = Some(VmFault {
            message,
            pc,
            span,
            trace,
            depth,
        });
    }

    fn span_at(&self, pc: usize) -> Option<ByteSpan> {
        self.pc_spans
            .get(pc)
            .copied()
            .filter(|s| s.start > 0 || s.end > 0)
    }

    #[inline]
    pub(crate) fn set(&mut self, reg: usize, v: PilotVal) {
        self.registers[self.base + reg] = v;
        let top = &mut self.reg_tops[(reg >= SCRATCH_BASE) as usize];
        if reg >= *top {
            *top = reg + 1;
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

    /// Every register of the running frame from this one up holds `NULL`.
    #[inline]
    pub(crate) fn reg_top(&self) -> usize {
        let [low, scratch] = self.reg_tops;
        if scratch > SCRATCH_BASE {
            scratch
        } else {
            low
        }
    }

    /// Records that the registers below `top` may hold a value.
    pub(crate) fn set_reg_top(&mut self, top: usize) {
        self.reg_tops = [top.min(SCRATCH_BASE), top.max(SCRATCH_BASE)];
    }

    /// The running frame's registers.
    #[inline]
    pub(crate) fn frame(&self) -> &[PilotVal] {
        &self.registers[self.base..self.base + FRAME_REGS]
    }

    /// Opens the callee's window past the caller's live registers, hands it
    /// the staged arguments and jumps to `entry`; `Ret` writes the result to
    /// `dst`. The caller's registers stay where they are, out of the
    /// callee's reach.
    pub(crate) fn enter_call(&mut self, entry: usize, dst: u8) -> Result<(), String> {
        if self.call_stack.len() >= MAX_CALL_DEPTH {
            return Err(format!("call depth exceeded {MAX_CALL_DEPTH} frames"));
        }
        let (caller, tops) = (self.base, self.reg_tops);
        let callee = caller + self.reg_top();
        if self.registers.len() < callee + FRAME_REGS {
            self.registers.resize(callee + FRAME_REGS, PilotVal::NULL);
        }
        let carried = tops[0].min(CARRIED_REGS);
        self.registers.copy_within(caller..caller + carried, callee);
        self.call_stack
            .push((self.pc, caller, [tops[0] as u16, tops[1] as u16], dst));
        self.base = callee;
        self.reg_tops = [carried, SCRATCH_BASE];
        self.pc = entry;
        Ok(())
    }

    /// Returns from the current frame with `ret`: the caller's registers come
    /// back and `ret` lands in its destination register.
    pub(crate) fn leave_frame(&mut self, ret: PilotVal) {
        let Some((ret_pc, caller, tops, d)) = self.call_stack.pop() else {
            return;
        };
        // The window is left empty for the next call that opens here.
        let (base, [low, scratch]) = (self.base, self.reg_tops);
        self.registers[base..base + low].fill(PilotVal::NULL);
        self.registers[base + SCRATCH_BASE..base + scratch].fill(PilotVal::NULL);
        self.base = caller;
        self.reg_tops = [tops[0] as usize, tops[1] as usize];
        self.set(d as usize, ret);
        self.pc = ret_pc;
    }

    /// `==`: lists (and tuples) compare element by element; everything else
    /// as `ops::equals`.
    pub(crate) fn values_equal(&self, a: PilotVal, b: PilotVal) -> bool {
        self.values_equal_depth(a, b, 64)
    }

    fn values_equal_depth(&self, a: PilotVal, b: PilotVal, depth: usize) -> bool {
        if a == b && a.as_float().is_none() {
            return true;
        }
        if depth == 0 {
            return false;
        }
        let list = |v: PilotVal| {
            v.as_list_handle()
                .and_then(|slot| self.heap_lists.get(slot))
        };
        if let (Some(x), Some(y)) = (list(a), list(b)) {
            return x.len() == y.len()
                && x.iter()
                    .zip(y)
                    .all(|(&p, &q)| self.values_equal_depth(p, q, depth - 1));
        }
        let map = |v: PilotVal| v.as_map_handle().and_then(|slot| self.heap_maps.get(slot));
        if let (Some(x), Some(y)) = (map(a), map(b)) {
            return x.len() == y.len()
                && x.iter().all(|(k, &v)| {
                    y.get(k)
                        .is_some_and(|&w| self.values_equal_depth(v, w, depth - 1))
                });
        }
        ops::equals(a, b)
    }

    /// Entry point and arity of a function value.
    fn closure_target(&self, f: PilotVal) -> Result<(usize, usize), String> {
        let env = f
            .as_fn_handle()
            .filter(|&slot| !self.list_is_dead(slot))
            .and_then(|slot| self.heap_lists.get(slot))
            .ok_or_else(|| format!("{} is not callable", f.kind_label()))?;
        let field = |i: usize| {
            env.get(i)
                .and_then(|v| v.as_int())
                .and_then(|n| usize::try_from(n).ok())
        };
        match (field(0), field(1)) {
            (Some(entry), Some(arity)) => Ok((entry, arity)),
            _ => Err("malformed function value".to_string()),
        }
    }

    fn constant(&self, i: u32) -> Result<PilotVal, String> {
        self.constants
            .get(i as usize)
            .copied()
            .ok_or_else(|| format!("constant #{i} out of range"))
    }

    /// Runs intrinsic `name` on `args` for the instruction `ins` and puts
    /// the result in its destination register.
    fn call_intrinsic(
        &mut self,
        ins: Instruction,
        name: PilotVal,
        args: &[PilotVal],
    ) -> Result<(), String> {
        let dst = ins.dst() as usize;
        // A read is found by comparing the name's two words; it allocates
        // nothing.
        if let Some(i) = self
            .reads
            .iter()
            .position(|r| *r == name)
            .filter(|_| args.len() == 2)
        {
            let v = self.get(args[0], args[1], crate::heap::READS[i])?;
            self.set(dst, v);
            self.collect_if_due();
            return Ok(());
        }
        let mut name_buf = [0u8; 14];
        let hook = name.str_in(&mut name_buf).ok_or_else(|| {
            format!(
                "intrinsic name must be a string, found {}",
                name.kind_label()
            )
        })?;
        if self.gen_intrinsic(hook, ins, args)? {
            return Ok(());
        }
        let v = self.intrinsic(hook, args)?;
        self.set(dst, v);
        self.collect_if_due();
        Ok(())
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
                let frame = &self.registers[self.base..self.base + FRAME_REGS];
                hook(
                    self.pc,
                    ins,
                    frame
                        .try_into()
                        .map_err(|_| "register window out of range")?,
                );
            }
            self.pc += 1;
            let (dst, src1, src2) = (ins.dst() as usize, ins.src1() as usize, ins.src2() as usize);
            let base = self.base;
            let (r1, r2) = (self.registers[base + src1], self.registers[base + src2]);
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
                    let r2 = match src2 as u8 {
                        crate::instruction::CONST_REG => self.constant(ins.wide())?,
                        _ => r2,
                    };
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
                // The callee takes over this frame: its `Ret` returns to
                // this frame's caller, so the call stack does not grow.
                Opcode::TailCall => self.pc = ins.wide() as usize,
                Opcode::SetOut => {
                    if self.outs.len() <= dst {
                        self.outs.resize(dst + 1, PilotVal::NULL);
                    }
                    self.outs[dst] = r1;
                }
                Opcode::GetOut => {
                    let v = self.outs.get(src1).copied().unwrap_or(PilotVal::NULL);
                    self.set(dst, v);
                }
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
                    let ret = match src1 as u8 {
                        crate::instruction::CONST_REG => self.constant(ins.wide())?,
                        0 => self.registers[base],
                        _ => r1,
                    };
                    let ret = self.gen_return(ret);
                    if self.call_stack.is_empty() {
                        return Ok(ret);
                    }
                    self.leave_frame(ret);
                }
                Opcode::EngineIntrinsic => {
                    let name = match src1 as u8 {
                        crate::instruction::CONST_REG => self.constant(ins.wide())?,
                        _ => r1,
                    };
                    // Neither the name nor the arguments are copied to the
                    // heap: intrinsics are the hottest calls the tools make.
                    let n = src2.min(255);
                    let mut small = [PilotVal::NULL; INLINE_ARGS];
                    let spilled: Vec<PilotVal>;
                    let args: &[PilotVal] = if n <= INLINE_ARGS {
                        small[..n].copy_from_slice(&self.registers[base + 1..=base + n]);
                        &small[..n]
                    } else {
                        spilled = self.registers[base + 1..=base + n].to_vec();
                        &spilled
                    };
                    self.call_intrinsic(ins, name, args)?;
                }
                Opcode::JumpUnless => {
                    let v = self.binary_op(Opcode::from_u8(dst as u8), r1, r2)?;
                    if !ops::truth(v)? {
                        self.pc = ins.branch_target() as usize;
                    }
                }
                Opcode::Clear => {
                    let end = (dst + src1).min(FRAME_REGS);
                    self.registers[base + dst..base + end].fill(PilotVal::NULL);
                }
                Opcode::IntrinsicKey => {
                    let w = ins.wide();
                    let (name, key) = (self.constant(w & 0xFFFF)?, self.constant(w >> 16)?);
                    self.call_intrinsic(ins, name, &[r1, key])?;
                }
                Opcode::IntrinsicArgs => {
                    let w = ins.wide();
                    let name = self.constant(w & crate::instruction::ARGS_NAME_MASK)?;
                    let args = [r1, r2];
                    let n = (w >> crate::instruction::ARGS_COUNT_SHIFT) as usize;
                    self.call_intrinsic(ins, name, &args[..n.min(2)])?;
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
        Ok(self.registers[self.base])
    }
}
