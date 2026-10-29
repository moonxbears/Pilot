//! Milestone 43.2: Native Pilot Debugger Core Engine.

use std::collections::HashMap;
use pilot_rt::value::PilotVal;

/// Debugger breakpoint representation.
#[derive(Debug, Clone, PartialEq)]
pub struct PilotBreakpoint {
    pub id: u32,
    pub file: String,
    pub line: u32,
    pub condition: Option<String>,
    pub hit_count: u32,
    pub hit_condition: Option<u32>,
    pub log_message: Option<String>,
}

/// Call stack frame representation.
#[derive(Debug, Clone, PartialEq)]
pub struct PilotStackFrame {
    pub id: u32,
    pub name: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub locals: HashMap<String, PilotVal>,
}

/// Execution state of the debugger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebuggerState {
    Running,
    Paused(String), // pause reason: "breakpoint", "step", "pause", "entry"
    Terminated,
}

/// Stepping mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    Continue,
    StepOver,
    StepInto,
    StepOut,
}

/// Core debugger engine managing breakpoints, call frames, and stepped execution.
pub struct PilotDebugger {
    next_breakpoint_id: u32,
    pub breakpoints: Vec<PilotBreakpoint>,
    pub call_stack: Vec<PilotStackFrame>,
    pub state: DebuggerState,
    pub step_mode: StepMode,
    pub step_target_depth: usize,
}

impl Default for PilotDebugger {
    fn default() -> Self {
        Self::new()
    }
}

impl PilotDebugger {
    pub fn new() -> Self {
        Self {
            next_breakpoint_id: 1,
            breakpoints: Vec::new(),
            call_stack: Vec::new(),
            state: DebuggerState::Running,
            step_mode: StepMode::Continue,
            step_target_depth: 0,
        }
    }

    pub fn set_breakpoint(
        &mut self,
        file: &str,
        line: u32,
        condition: Option<String>,
        hit_condition: Option<u32>,
        log_message: Option<String>,
    ) -> PilotBreakpoint {
        let id = self.next_breakpoint_id;
        self.next_breakpoint_id += 1;
        let bp = PilotBreakpoint {
            id,
            file: file.to_string(),
            line,
            condition,
            hit_count: 0,
            hit_condition,
            log_message,
        };
        self.breakpoints.push(bp.clone());
        bp
    }

    pub fn remove_breakpoint(&mut self, id: u32) -> bool {
        if let Some(pos) = self.breakpoints.iter().position(|b| b.id == id) {
            self.breakpoints.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn clear_breakpoints_for_file(&mut self, file: &str) {
        self.breakpoints.retain(|b| b.file != file);
    }

    pub fn clear_all_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    pub fn check_breakpoint(&mut self, file: &str, line: u32) -> Option<PilotBreakpoint> {
        for bp in &mut self.breakpoints {
            if bp.line == line && (bp.file == file || bp.file.ends_with(file) || file.ends_with(&bp.file)) {
                bp.hit_count += 1;

                if let Some(thresh) = bp.hit_condition {
                    if bp.hit_count < thresh {
                        return None;
                    }
                }

                // If conditional breakpoint, evaluate condition in top frame locals
                if let Some(cond) = &bp.condition {
                    if let Some(frame) = self.call_stack.last() {
                        if !Self::eval_condition(cond, &frame.locals) {
                            return None;
                        }
                    }
                }

                return Some(bp.clone());
            }
        }
        None
    }

    pub fn eval_condition(cond: &str, locals: &HashMap<String, PilotVal>) -> bool {
        let trimmed = cond.trim();
        if let Some(eq_idx) = trimmed.find("==") {
            let left = trimmed[..eq_idx].trim();
            let right = trimmed[eq_idx + 2..].trim();
            if let Some(l_val) = locals.get(left) {
                if let Ok(i) = right.parse::<i64>() {
                    return l_val.as_int() == Some(i);
                } else if right == "true" {
                    return l_val.as_bool() == Some(true);
                } else if right == "false" {
                    return l_val.as_bool() == Some(false);
                } else if right.starts_with('"') && right.ends_with('"') {
                    let s_expect = &right[1..right.len() - 1];
                    return l_val.with_str(|s| s == s_expect).unwrap_or(false);
                }
            }
        } else if let Some(val) = locals.get(trimmed) {
            return val.as_bool().unwrap_or(false) || val.as_int().map_or(false, |i| i != 0);
        }
        true
    }

    pub fn push_frame(&mut self, name: &str, file: &str, line: u32, column: u32) -> u32 {
        let id = (self.call_stack.len() + 1) as u32;
        self.call_stack.push(PilotStackFrame {
            id,
            name: name.to_string(),
            file: file.to_string(),
            line,
            column,
            locals: HashMap::new(),
        });
        id
    }

    pub fn pop_frame(&mut self) -> Option<PilotStackFrame> {
        self.call_stack.pop()
    }

    pub fn set_local(&mut self, name: &str, val: PilotVal) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.locals.insert(name.to_string(), val);
        }
    }

    pub fn current_frame(&self) -> Option<&PilotStackFrame> {
        self.call_stack.last()
    }

    pub fn continue_exec(&mut self) {
        self.step_mode = StepMode::Continue;
        self.state = DebuggerState::Running;
    }

    pub fn step_over(&mut self) {
        self.step_mode = StepMode::StepOver;
        self.step_target_depth = self.call_stack.len();
        self.state = DebuggerState::Running;
    }

    pub fn step_into(&mut self) {
        self.step_mode = StepMode::StepInto;
        self.state = DebuggerState::Running;
    }

    pub fn step_out(&mut self) {
        self.step_mode = StepMode::StepOut;
        self.step_target_depth = self.call_stack.len().saturating_sub(1);
        self.state = DebuggerState::Running;
    }

    pub fn pause(&mut self) {
        self.state = DebuggerState::Paused("pause".to_string());
    }

    pub fn evaluate(&self, expr: &str, frame_id: Option<u32>) -> Result<(PilotVal, String), String> {
        let frame = if let Some(fid) = frame_id {
            self.call_stack.iter().find(|f| f.id == fid)
        } else {
            self.call_stack.last()
        };

        if let Some(f) = frame {
            let trimmed = expr.trim();
            if let Some(val) = f.locals.get(trimmed) {
                let ty = match val.tag() {
                    pilot_rt::value::TAG_INT => "i64",
                    pilot_rt::value::TAG_FLOAT => "f64",
                    pilot_rt::value::TAG_BOOL => "bool",
                    pilot_rt::value::TAG_SSO_STR | pilot_rt::value::TAG_HEAP_STR => "str",
                    _ => "any",
                };
                return Ok((*val, ty.to_string()));
            }

            // Simple literal evaluations
            if let Ok(i) = trimmed.parse::<i64>() {
                return Ok((PilotVal::from_int(i), "i64".to_string()));
            } else if let Ok(flt) = trimmed.parse::<f64>() {
                return Ok((PilotVal::from_float(flt), "f64".to_string()));
            } else if trimmed == "true" {
                return Ok((PilotVal::TRUE, "bool".to_string()));
            } else if trimmed == "false" {
                return Ok((PilotVal::FALSE, "bool".to_string()));
            } else if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
                return Ok((PilotVal::from_str(&trimmed[1..trimmed.len() - 1]), "str".to_string()));
            }

            Err(format!("Variable or expression '{}' not found in current frame", expr))
        } else {
            Err("No active stack frame".to_string())
        }
    }
}
