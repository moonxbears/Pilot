//! Generator frames (F5b): suspended function bodies the VM resumes.
//!
//! A frame holds the registers of a generator body and where it resumes.
//! `gen_new` makes one from the registers of the call that created the
//! generator; its 32-bit operand is the body's address. `gen_resume(id, v)`
//! enters the frame like a call: the resumer's registers are saved, the
//! frame's are loaded, and the `yield` waiting in it receives `v`.
//! `gen_yield(item)` saves the frame and returns `item` to the resumer, as
//! `Ret` would. When the body returns, the frame is done and the resumer
//! gets `null`. `gen_close(id)` resumes a suspended frame with its closing
//! flag set, so the body runs its pending `defer`s and returns
//! (`lower_yield`); a frame that never started just ends. No OS threads are
//! involved.
//!
//! A frame resumed while it runs (from inside itself) stops the program
//! with GEN003.

use crate::instruction::Instruction;
use crate::vm::PilVirtualMachine;
use pilot_rt::PilotVal;

/// The fault of resuming a generator from inside itself.
const RESUMED_WHILE_RUNNING: &str = "GEN003: a generator cannot be resumed while it runs";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GenState {
    /// Created; the body has not run.
    Fresh,
    /// Stopped at a `yield`.
    Suspended,
    /// Running, between a resume and its next `yield` or return.
    Running,
    /// The body returned, or the generator was closed.
    Done,
}

/// One generator body: its registers and where it goes on.
#[derive(Clone, Debug)]
pub struct GenFrame {
    pub(crate) state: GenState,
    resume_pc: usize,
    regs: Vec<PilotVal>,
    /// The register the waiting `yield` gives the sent value in.
    sent_dst: Option<u8>,
    /// Resumed to be closed: the body leaves at its `yield`.
    closing: bool,
    /// The item the last `yield` handed out.
    current: PilotVal,
}

impl PilVirtualMachine {
    /// Runs generator intrinsic `hook` for instruction `ins` with `args`.
    /// `Ok(false)` when `hook` is not one of the generator intrinsics.
    pub(crate) fn gen_intrinsic(&mut self, hook: &str, ins: Instruction, args: &[PilotVal]) -> Result<bool, String> {
        let dst = ins.dst() as usize;
        match hook {
            "gen_new" => {
                let act = self.active_registers();
                self.generators.push(GenFrame {
                    state: GenState::Fresh,
                    resume_pc: ins.wide() as usize,
                    regs: self.registers[..act].to_vec(),
                    sent_dst: None,
                    closing: false,
                    current: PilotVal::NULL,
                });
                self.registers[dst] = PilotVal::from_int(self.generators.len() as i64 - 1);
            }
            "gen_resume" => {
                let id = self.frame_id(args.first())?;
                let sent = args.get(1).copied().unwrap_or(PilotVal::NULL);
                self.resume(id, sent, ins.dst())?;
            }
            "gen_close" => {
                let id = self.frame_id(args.first())?;
                match self.generators[id].state {
                    GenState::Fresh => self.finish(id),
                    GenState::Suspended => {
                        self.generators[id].closing = true;
                        self.resume(id, PilotVal::NULL, ins.dst())?;
                        return Ok(true);
                    }
                    GenState::Running => return Err(RESUMED_WHILE_RUNNING.to_string()),
                    GenState::Done => {}
                }
                self.registers[dst] = PilotVal::NULL;
            }
            "gen_yield" => {
                let Some(&(depth, id)) = self.gen_stack.last() else {
                    return Err("`yield` outside a running generator".to_string());
                };
                if depth != self.call_stack.len() {
                    return Err("`yield` outside a running generator".to_string());
                }
                self.gen_stack.pop();
                let item = args.first().copied().unwrap_or(PilotVal::NULL);
                let act = self.active_registers();
                let frame = &mut self.generators[id];
                frame.regs = self.registers[..act].to_vec();
                frame.resume_pc = self.pc;
                frame.sent_dst = Some(ins.dst());
                frame.state = GenState::Suspended;
                frame.current = item;
                self.leave_frame(item);
            }
            "gen_done" => {
                let id = self.frame_id(args.first())?;
                self.registers[dst] = PilotVal::from_bool(self.generators[id].state == GenState::Done);
            }
            "gen_current" => {
                let id = self.frame_id(args.first())?;
                self.registers[dst] = self.generators[id].current;
            }
            "gen_closing" => {
                let closing = self.gen_stack.last().is_some_and(|&(_, id)| self.generators[id].closing);
                self.registers[dst] = PilotVal::from_bool(closing);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Called by `Ret`: when the returning frame is a generator body, the
    /// generator is done and its resumer gets `null`. Returns the value the
    /// resumer gets.
    pub(crate) fn gen_return(&mut self, value: PilotVal) -> PilotVal {
        match self.gen_stack.last() {
            Some(&(depth, id)) if depth == self.call_stack.len() => {
                self.gen_stack.pop();
                self.finish(id);
                PilotVal::NULL
            }
            _ => value,
        }
    }

    fn frame_id(&self, v: Option<&PilotVal>) -> Result<usize, String> {
        v.and_then(|v| v.as_int())
            .and_then(|n| usize::try_from(n).ok())
            .filter(|&n| n < self.generators.len())
            .ok_or_else(|| "not a generator frame".to_string())
    }

    fn finish(&mut self, id: usize) {
        let frame = &mut self.generators[id];
        frame.state = GenState::Done;
        frame.regs = Vec::new();
        frame.current = PilotVal::NULL;
        frame.sent_dst = None;
    }

    /// Enters frame `id`, its waiting `yield` receiving `sent`; what it
    /// yields or returns lands in the resumer's register `dst`.
    fn resume(&mut self, id: usize, sent: PilotVal, dst: u8) -> Result<(), String> {
        match self.generators[id].state {
            GenState::Done => {
                self.registers[dst as usize] = PilotVal::NULL;
                return Ok(());
            }
            GenState::Running => return Err(RESUMED_WHILE_RUNNING.to_string()),
            GenState::Fresh | GenState::Suspended => {}
        }
        let resume_pc = self.generators[id].resume_pc;
        self.enter_call(resume_pc, dst)?;
        let frame = &mut self.generators[id];
        frame.state = GenState::Running;
        let regs = std::mem::take(&mut frame.regs);
        let sent_dst = frame.sent_dst;
        self.registers.fill(PilotVal::NULL);
        self.registers[..regs.len()].copy_from_slice(&regs);
        if let Some(d) = sent_dst {
            self.registers[d as usize] = sent;
        }
        self.gen_stack.push((self.call_stack.len(), id));
        Ok(())
    }

    /// The number of registers up to the last one holding a value.
    pub(crate) fn active_registers(&self) -> usize {
        self.registers.iter().rposition(|r| !r.is_null()).map_or(0, |i| i + 1)
    }
}
