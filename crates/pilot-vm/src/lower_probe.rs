//! Probes: calls the type checker plans around statements.
//!
//! A probe plan (built by `selfhost/check/probes.pilot`, carried here by the
//! CLI) names statements by span and kind. Around each one the seed:
//!
//! 1. when the probe captures, reads the text of the statement's target
//!    before the statement runs;
//! 2. calls the probe's function with its arguments, each a fixed string, a
//!    fixed flag, the target's text now, the text captured before, the text
//!    of a named binding, or the value of an engine operation that takes no
//!    arguments. When it calls is the probe's `at`: after the node (the
//!    default), before it, or as if `defer` were written just before it
//!    (on every way out of the enclosing block, after a return value is
//!    computed).
//!
//! The target of a `let` / `var` is the binding it makes; that of an
//! assignment is the place it writes. A target's text is `"" + value`, as an
//! f-string shows a value. Reading a target runs none of the drop plan's
//! drops or copies: a probe only looks. The seed knows nothing of what a
//! probe is for; the function to call and its arguments come from the plan.

use crate::lower_drop::{Deferred, NodeKey};
use crate::{BytecodeCompiler, Opcode};
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;
use std::collections::HashMap;
use std::rc::Rc;

/// One argument of a probe's call. The codes are `ProbeArgKind` in
/// `selfhost/check/probes.pilot`.
#[derive(Clone, Debug, PartialEq)]
pub enum ProbeArg {
    Text(String),
    Flag(bool),
    TargetText,
    Captured,
    /// `"" + value` of the binding of that name in scope.
    BindingText(String),
    /// The value of the engine operation of that name, called with nothing.
    Engine(String),
}

impl ProbeArg {
    /// The argument with code `code` and, for a fixed one, value `text` or
    /// `flag`.
    pub fn from_code(code: i64, text: &str, flag: bool) -> Option<Self> {
        Some(match code {
            0 => ProbeArg::Text(text.to_string()),
            1 => ProbeArg::Flag(flag),
            2 => ProbeArg::TargetText,
            3 => ProbeArg::Captured,
            4 => ProbeArg::BindingText(text.to_string()),
            5 => ProbeArg::Engine(text.to_string()),
            _ => return None,
        })
    }
}

/// When a probe calls, relative to its node. The codes are `ProbeAt` in
/// `selfhost/check/probes.pilot`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ProbeAt {
    #[default]
    After,
    Before,
    Defer,
}

impl ProbeAt {
    pub fn from_code(code: i64) -> Option<Self> {
        Some(match code {
            0 => ProbeAt::After,
            1 => ProbeAt::Before,
            2 => ProbeAt::Defer,
            _ => return None,
        })
    }
}

/// What happens around one statement.
#[derive(Clone, Debug)]
pub struct Probe {
    pub capture: bool,
    pub call: String,
    pub args: Vec<ProbeArg>,
    pub at: ProbeAt,
}

/// The probes of a program, by the statement they surround.
#[derive(Clone, Debug, Default)]
pub struct ProbePlan {
    pub at: HashMap<NodeKey, Vec<Probe>>,
}

fn key_of(arena: &AstArena, id: NodeId) -> NodeKey {
    let span = arena.span(id);
    (span.start, span.end, arena.node(id).kind)
}

impl BytecodeCompiler {
    pub(crate) fn set_probe_plan(&mut self, plan: ProbePlan) {
        if !plan.at.is_empty() {
            self.probe_plan = Some(Rc::new(plan));
        }
    }

    fn probes_at(&self, arena: &AstArena, id: NodeId) -> Vec<Probe> {
        match &self.probe_plan {
            Some(p) if id.is_some() => p.at.get(&key_of(arena, id)).cloned().unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// Runs before statement `id` is lowered: makes the calls of its
    /// `Before` probes, registers its `Defer` probes with the innermost
    /// `defer` scope, and reads the text of its target for each probe that
    /// captures it, in a register held until `probes_after`.
    pub(crate) fn probes_before(&mut self, arena: &AstArena, id: NodeId) -> Vec<Option<u8>> {
        let probes = self.probes_at(arena, id);
        let mut captured = Vec::with_capacity(probes.len());
        for p in &probes {
            match p.at {
                ProbeAt::Before => {
                    let mark = self.next_reg;
                    self.emit_probe_call(arena, id, p, None);
                    self.release_to(mark);
                }
                ProbeAt::Defer => {
                    if let Some(scope) = self.defers.last_mut() {
                        scope.push(Deferred::Probe(p.clone()));
                    }
                }
                ProbeAt::After => {}
            }
            captured.push((p.capture && p.at == ProbeAt::After).then(|| self.target_text(arena, id)));
        }
        captured
    }

    /// Runs after statement `id` is lowered: the calls of its `After` probes.
    pub(crate) fn probes_after(&mut self, arena: &AstArena, id: NodeId, captured: &[Option<u8>]) {
        let probes = self.probes_at(arena, id);
        if probes.is_empty() {
            return;
        }
        let mark = self.next_reg;
        for (i, p) in probes.iter().enumerate() {
            if p.at == ProbeAt::After {
                self.emit_probe_call(arena, id, p, captured.get(i).copied().flatten());
            }
        }
        self.release_to(mark);
    }

    /// The call of probe `p` at node `id` (`NodeId::NONE`-like for a
    /// deferred probe, which reads no target); `captured` is the register
    /// `probes_before` read the target's text into.
    pub(crate) fn emit_probe_call(&mut self, arena: &AstArena, id: NodeId, p: &Probe, captured: Option<u8>) {
        let mut regs = Vec::with_capacity(p.args.len());
        for a in &p.args {
            let r = match a {
                ProbeArg::Text(t) => self.emit_const(PilotVal::from_str(t)),
                ProbeArg::Flag(b) => self.emit_const(PilotVal::from_bool(*b)),
                ProbeArg::TargetText => self.target_text(arena, id),
                ProbeArg::Captured => match captured {
                    Some(r) => r,
                    None => self.emit_const(PilotVal::from_str("")),
                },
                ProbeArg::BindingText(name) => {
                    let value = match self.var_slots.get(name) {
                        Some(&slot) => slot,
                        None => self.emit_null(),
                    };
                    let empty = self.emit_const(PilotVal::from_str(""));
                    let dst = self.alloc_reg();
                    self.emit(Opcode::Add, dst, empty, value);
                    dst
                }
                ProbeArg::Engine(op) => self.emit_intrinsic(op, &[]),
            };
            regs.push(r);
        }
        self.emit_user_call(p.call.clone(), p.call.clone(), &regs);
    }

    /// The text of the target of statement `id` (`"" + value`), in a new
    /// register; the drop plan is set aside while it is read.
    fn target_text(&mut self, arena: &AstArena, id: NodeId) -> u8 {
        let plan = self.drop_plan.take();
        let node = *arena.node(id);
        let value = match node.node_kind() {
            NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
                let name = Self::name(arena, node.lhs).to_string();
                match self.var_slots.get(&name) {
                    Some(&slot) => slot,
                    None => self.emit_null(),
                }
            }
            NodeKind::AssignStmt => self.compile_node(arena, NodeId(node.lhs)),
            _ => self.emit_null(),
        };
        self.drop_plan = plan;
        let empty = self.emit_const(PilotVal::from_str(""));
        let dst = self.alloc_reg();
        self.emit(Opcode::Add, dst, empty, value);
        dst
    }
}
