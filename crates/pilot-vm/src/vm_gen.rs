//! Generator frames (F5b): suspended function bodies the VM resumes.

use crate::instruction::Instruction;
use crate::value::PilotVal;
use crate::vm::PilVirtualMachine;

const RESUMED_WHILE_RUNNING: &str = "GEN003: a generator cannot be resumed while it runs";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GenState {
    Fresh,
    Suspended,
    Running,
    Done,
}

#[derive(Clone, Debug)]
pub struct GenFrame {
    pub(crate) state: GenState,
    resume_pc: usize,
    regs: Vec<PilotVal>,
    sent_dst: Option<u8>,
    closing: bool,
    current: PilotVal,
}

impl PilVirtualMachine {
    /// Executes generator operation `code` (`0..=6` mapped by `stage_a_natives`).
    pub(crate) fn exec_gen_op(
        &mut self,
        code: u8,
        ins: Instruction,
        args: &[PilotVal],
    ) -> Result<(), String> {
        let dst = ins.dst() as usize;
        match code {
            0 => {
                let act = self.active_registers();
                self.generators.push(GenFrame {
                    state: GenState::Fresh,
                    resume_pc: ins.wide() as usize,
                    regs: self.frame()[..act].to_vec(),
                    sent_dst: None,
                    closing: false,
                    current: PilotVal::NULL,
                });
                self.set(dst, PilotVal::from_int(self.generators.len() as i64 - 1));
            }
            1 => {
                let id = self.frame_id(args.first())?;
                let sent = args.get(1).copied().unwrap_or(PilotVal::NULL);
                self.resume(id, sent, ins.dst())?;
            }
            2 => {
                let id = self.frame_id(args.first())?;
                match self.generators[id].state {
                    GenState::Fresh => self.finish(id),
                    GenState::Suspended => {
                        self.generators[id].closing = true;
                        return self.resume(id, PilotVal::NULL, ins.dst());
                    }
                    GenState::Running => return Err(RESUMED_WHILE_RUNNING.to_string()),
                    GenState::Done => {}
                }
                self.set(dst, PilotVal::NULL);
            }
            3 => {
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
                frame.regs = self.registers[self.base..self.base + act].to_vec();
                frame.resume_pc = self.pc;
                frame.sent_dst = Some(ins.dst());
                frame.state = GenState::Suspended;
                frame.current = item;
                self.leave_frame(item);
            }
            4 => {
                let id = self.frame_id(args.first())?;
                self.set(
                    dst,
                    PilotVal::from_bool(self.generators[id].state == GenState::Done),
                );
            }
            5 => {
                let id = self.frame_id(args.first())?;
                self.set(dst, self.generators[id].current);
            }
            _ => {
                let closing = self
                    .gen_stack
                    .last()
                    .is_some_and(|&(_, id)| self.generators[id].closing);
                self.set(dst, PilotVal::from_bool(closing));
            }
        }
        Ok(())
    }

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

    fn resume(&mut self, id: usize, sent: PilotVal, dst: u8) -> Result<(), String> {
        match self.generators[id].state {
            GenState::Done => {
                self.set(dst as usize, PilotVal::NULL);
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
        let base = self.base;
        let top = self.reg_top();
        self.registers[base..base + top].fill(PilotVal::NULL);
        self.registers[base..base + regs.len()].copy_from_slice(&regs);
        self.set_reg_top(regs.len());
        if let Some(d) = sent_dst {
            self.set(d as usize, sent);
        }
        self.gen_stack.push((self.call_stack.len(), id));
        Ok(())
    }

    pub(crate) fn generator_values(&self) -> impl Iterator<Item = &PilotVal> {
        self.generators
            .iter()
            .flat_map(|g| g.regs.iter().chain(std::iter::once(&g.current)))
    }

    pub(crate) fn active_registers(&self) -> usize {
        self.frame()[..self.reg_top()]
            .iter()
            .rposition(|r| !r.is_null())
            .map_or(0, |i| i + 1)
    }
}
