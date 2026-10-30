//! Milestone 43.2: Native Pilot Debugger Core Engine.

use std::collections::HashMap;
use pilot_ast::{AstArena, NodeId, NodeKind, AstNode};
use pilot_intern::SymbolId;
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
    pub local_types: HashMap<String, String>,
    pub stmts: Vec<(NodeId, u32, u32)>,
    pub stmt_idx: usize,
    pub dest_var: Option<String>,
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
    next_frame_id: u32,
    pub breakpoints: Vec<PilotBreakpoint>,
    pub call_stack: Vec<PilotStackFrame>,
    pub state: DebuggerState,
    pub step_mode: StepMode,
    pub step_target_depth: usize,

    pub program_path: Option<String>,
    pub source: String,
    pub arena: Option<AstArena>,
    pub globals: HashMap<String, PilotVal>,
    pub global_types: HashMap<String, String>,
    pub functions: HashMap<String, (NodeId, Vec<String>, NodeId, u32)>,
    pub pending_outputs: Vec<String>,
    pub skip_bp: Option<(String, u32)>,
}

impl Default for PilotDebugger {
    fn default() -> Self {
        Self::new()
    }
}

fn unpack_call(arena: &AstArena, node: AstNode) -> Option<(NodeId, Vec<NodeId>)> {
    let children = arena.children_slice(node.lhs, node.rhs);
    if children.is_empty() {
        return None;
    }
    let callee = NodeId(children[0]);
    let args = children[1..].iter().map(|&id| NodeId(id)).collect();
    Some((callee, args))
}

impl PilotDebugger {
    pub fn new() -> Self {
        Self {
            next_breakpoint_id: 1,
            next_frame_id: 1,
            breakpoints: Vec::new(),
            call_stack: Vec::new(),
            state: DebuggerState::Running,
            step_mode: StepMode::Continue,
            step_target_depth: 0,

            program_path: None,
            source: String::new(),
            arena: None,
            globals: HashMap::new(),
            global_types: HashMap::new(),
            functions: HashMap::new(),
            pending_outputs: Vec::new(),
            skip_bp: None,
        }
    }

    pub fn load_program(&mut self, file_path: &str) -> Result<(), String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;
        let arena = pilot_parser::parse(&content);
        let line_index = pilot_span::LineIndex::new(content.as_bytes());

        let mut stmts = Vec::new();
        let mut functions = HashMap::new();

        if arena.root.is_some() {
            let root_id = arena.root;
            let root_node = arena.node(root_id);
            if root_node.node_kind() == NodeKind::Program {
                let children = arena.children_slice(root_node.lhs, root_node.rhs);
                for &child_id in children {
                    let node = arena.node(NodeId(child_id));
                    let span = arena.spans.get(child_id as usize).copied().unwrap_or(pilot_span::ByteSpan::new(0, 0));
                    let (line_1, col_1) = line_index.offset_to_line_col(span.start);

                    if node.node_kind() == NodeKind::FnDecl {
                        let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                        let fn_start = node.rhs as usize;
                        let fn_len = node.flags as usize;
                        let body_node = NodeId(arena.extra_children.get(fn_start).copied().unwrap_or(0));
                        let mut params = Vec::new();
                        if fn_len > 1 && fn_start + fn_len <= arena.extra_children.len() {
                            for &p_id in &arena.extra_children[fn_start + 1..fn_start + fn_len] {
                                let p_node = arena.node(NodeId(p_id));
                                let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                                params.push(p_name);
                            }
                        }
                        functions.insert(name, (NodeId(child_id), params, body_node, line_1 as u32));
                    } else {
                        stmts.push((NodeId(child_id), line_1 as u32, col_1 as u32));
                    }
                }
            }
        }

        let initial_line = if let Some(&(_, l, _)) = stmts.first() {
            l
        } else {
            1
        };

        self.program_path = Some(file_path.to_string());
        self.source = content;
        self.arena = Some(arena);
        self.functions = functions;
        self.globals.clear();
        self.global_types.clear();
        self.call_stack.clear();
        self.pending_outputs.clear();
        self.next_frame_id = 1;
        self.skip_bp = None;

        self.push_frame_with_stmts("main", file_path, initial_line, 1, stmts, None);
        self.state = DebuggerState::Running;

        Ok(())
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
        let before_len = self.breakpoints.len();
        self.breakpoints.retain(|b| b.id != id);
        self.breakpoints.len() < before_len
    }

    pub fn clear_breakpoints_for_file(&mut self, file: &str) {
        self.breakpoints.retain(|b| b.file != file);
    }

    pub fn clear_all_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    pub fn check_breakpoint(&mut self, file: &str, line: u32) -> Option<PilotBreakpoint> {
        let norm_file = file.replace('\\', "/");
        for bp in self.breakpoints.iter_mut() {
            let bp_norm = bp.file.replace('\\', "/");
            let file_match = bp_norm == norm_file
                || norm_file.ends_with(&bp_norm)
                || bp_norm.ends_with(&norm_file);

            if file_match && bp.line == line {
                bp.hit_count += 1;
                if let Some(target_hits) = bp.hit_condition {
                    if bp.hit_count < target_hits {
                        return None;
                    }
                }
                return Some(bp.clone());
            }
        }
        None
    }

    pub fn eval_condition(cond: &str, locals: &HashMap<String, PilotVal>) -> bool {
        let trimmed = cond.trim();
        if trimmed.is_empty() {
            return true;
        }

        if let Some((lhs, rhs)) = trimmed.split_once("==") {
            let l_key = lhs.trim();
            let r_str = rhs.trim().trim_matches('"');
            if let Some(val) = locals.get(l_key) {
                if let Some(i) = val.as_int() {
                    if let Ok(target) = r_str.parse::<i64>() {
                        return i == target;
                    }
                }
                return val.to_string_repr() == r_str;
            }
            return false;
        }

        if let Some(val) = locals.get(trimmed) {
            return val.as_bool().unwrap_or(false) || val.as_int().map_or(false, |i| i != 0);
        }
        true
    }

    pub fn push_frame(&mut self, name: &str, file: &str, line: u32, column: u32) -> u32 {
        let id = self.next_frame_id;
        self.next_frame_id += 1;
        self.call_stack.push(PilotStackFrame {
            id,
            name: name.to_string(),
            file: file.to_string(),
            line,
            column,
            locals: HashMap::new(),
            local_types: HashMap::new(),
            stmts: Vec::new(),
            stmt_idx: 0,
            dest_var: None,
        });
        id
    }

    pub fn push_frame_with_stmts(
        &mut self,
        name: &str,
        file: &str,
        line: u32,
        column: u32,
        stmts: Vec<(NodeId, u32, u32)>,
        dest_var: Option<String>,
    ) -> u32 {
        let id = self.next_frame_id;
        self.next_frame_id += 1;
        self.call_stack.push(PilotStackFrame {
            id,
            name: name.to_string(),
            file: file.to_string(),
            line,
            column,
            locals: HashMap::new(),
            local_types: HashMap::new(),
            stmts,
            stmt_idx: 0,
            dest_var,
        });
        id
    }

    pub fn pop_frame(&mut self) -> Option<PilotStackFrame> {
        self.call_stack.pop()
    }

    pub fn set_local(&mut self, name: &str, val: PilotVal, ty: &str) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.locals.insert(name.to_string(), val);
            frame.local_types.insert(name.to_string(), ty.to_string());
        }
    }

    pub fn current_frame(&self) -> Option<&PilotStackFrame> {
        self.call_stack.last()
    }

    /// Single atomic execution step on the top frame.
    pub fn step_one(&mut self) {
        if self.call_stack.is_empty() {
            self.state = DebuggerState::Terminated;
            return;
        }

        let arena = match self.arena.as_ref() {
            Some(a) => a.clone(),
            None => {
                self.state = DebuggerState::Terminated;
                return;
            }
        };

        // Check if top frame is finished
        let (is_done, return_val, dest_var) = {
            let frame = self.call_stack.last().unwrap();
            (
                frame.stmt_idx >= frame.stmts.len(),
                frame.locals.get("__return__").copied(),
                frame.dest_var.clone(),
            )
        };

        if is_done {
            self.call_stack.pop();
            if self.call_stack.is_empty() {
                self.state = DebuggerState::Terminated;
                return;
            }
            if let Some(dest) = dest_var {
                if let Some(ret) = return_val {
                    let parent = self.call_stack.last_mut().unwrap();
                    parent.locals.insert(dest.clone(), ret);
                    self.globals.insert(dest, ret);
                }
            }
            return;
        }

        // Get statement at current stmt_idx
        let (node_id, line, col) = {
            let frame = self.call_stack.last().unwrap();
            frame.stmts[frame.stmt_idx]
        };

        // Update frame position
        {
            let frame = self.call_stack.last_mut().unwrap();
            frame.line = line;
            frame.column = col;
        }

        // Check breakpoint
        let prog = self.program_path.clone().unwrap_or_default();
        let at_skip = self.skip_bp.as_ref().map_or(false, |(p, l)| p == &prog && *l == line);

        if !at_skip {
            if let Some(bp) = self.check_breakpoint(&prog, line) {
                let cond_ok = if let Some(cond) = &bp.condition {
                    let frame = self.call_stack.last().unwrap();
                    Self::eval_condition(cond, &frame.locals)
                } else {
                    true
                };

                if cond_ok {
                    self.skip_bp = Some((prog, line));
                    self.state = DebuggerState::Paused("breakpoint".to_string());
                    return;
                }
            }
        } else {
            self.skip_bp = None;
        }

        // Advance stmt_idx
        {
            let frame = self.call_stack.last_mut().unwrap();
            frame.stmt_idx += 1;
        }

        // Execute statement node
        self.execute_statement_node(&arena, node_id, line);
    }

    fn execute_statement_node(&mut self, arena: &AstArena, node_id: NodeId, _current_line: u32) {
        if node_id.is_none() {
            return;
        }
        let node = *arena.node(node_id);
        match node.node_kind() {
            NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                let rhs_node_id = NodeId(node.rhs);
                let rhs_node = *arena.node(rhs_node_id);

                if rhs_node.node_kind() == NodeKind::CallExpr {
                    if let Some((callee_id, arg_ids)) = unpack_call(arena, rhs_node) {
                        let callee_node = *arena.node(callee_id);
                        let fn_name = if callee_node.node_kind() == NodeKind::IdentExpr {
                            arena.interner.resolve(SymbolId(callee_node.lhs)).to_string()
                        } else {
                            "anon".to_string()
                        };

                        if let Some((_, params, body_node, start_line)) = self.functions.get(&fn_name).cloned() {
                            let mut evaluated_args = Vec::new();
                            for arg_id in arg_ids {
                                let (val, ty) = self.eval_ast_node(arena, arg_id);
                                evaluated_args.push((val, ty));
                            }

                            let prog = self.program_path.clone().unwrap_or_default();
                            let line_index = pilot_span::LineIndex::new(self.source.as_bytes());
                            let fn_stmts = extract_fn_stmts(arena, &line_index, body_node);
                            let initial_line = fn_stmts.first().map(|s| s.1).unwrap_or(start_line);

                            self.push_frame_with_stmts(&fn_name, &prog, initial_line, 1, fn_stmts, Some(name));

                            for (i, p) in params.iter().enumerate() {
                                if let Some((val, ty)) = evaluated_args.get(i) {
                                    self.set_local(p, *val, ty);
                                }
                            }
                            return;
                        }
                    }
                }

                let (val, ty) = self.eval_ast_node(arena, rhs_node_id);
                self.set_local(&name, val, &ty);
                self.globals.insert(name.clone(), val);
                self.global_types.insert(name, ty);
            }
            NodeKind::AssignStmt => {
                let lhs_node = *arena.node(NodeId(node.lhs));
                let rhs_node_id = NodeId(node.rhs);
                let rhs_node = *arena.node(rhs_node_id);

                if lhs_node.node_kind() == NodeKind::IdentExpr {
                    let name = arena.interner.resolve(SymbolId(lhs_node.lhs)).to_string();

                    if rhs_node.node_kind() == NodeKind::CallExpr {
                        if let Some((callee_id, arg_ids)) = unpack_call(arena, rhs_node) {
                            let callee_node = *arena.node(callee_id);
                            let fn_name = if callee_node.node_kind() == NodeKind::IdentExpr {
                                arena.interner.resolve(SymbolId(callee_node.lhs)).to_string()
                            } else {
                                "anon".to_string()
                            };

                            if let Some((_, params, body_node, start_line)) = self.functions.get(&fn_name).cloned() {
                                let mut evaluated_args = Vec::new();
                                for arg_id in arg_ids {
                                    let (val, ty) = self.eval_ast_node(arena, arg_id);
                                    evaluated_args.push((val, ty));
                                }

                                let prog = self.program_path.clone().unwrap_or_default();
                                let line_index = pilot_span::LineIndex::new(self.source.as_bytes());
                                let fn_stmts = extract_fn_stmts(arena, &line_index, body_node);
                                let initial_line = fn_stmts.first().map(|s| s.1).unwrap_or(start_line);

                                self.push_frame_with_stmts(&fn_name, &prog, initial_line, 1, fn_stmts, Some(name));

                                for (i, p) in params.iter().enumerate() {
                                    if let Some((val, ty)) = evaluated_args.get(i) {
                                        self.set_local(p, *val, ty);
                                    }
                                }
                                return;
                            }
                        }
                    }

                    let (val, ty) = self.eval_ast_node(arena, rhs_node_id);
                    self.set_local(&name, val, &ty);
                    self.globals.insert(name.clone(), val);
                    self.global_types.insert(name, ty);
                }
            }
            NodeKind::ReturnStmt => {
                let ret_node = NodeId(node.lhs);
                let (val, _) = self.eval_ast_node(arena, ret_node);
                if let Some(frame) = self.call_stack.last_mut() {
                    frame.locals.insert("__return__".to_string(), val);
                    frame.stmt_idx = frame.stmts.len(); // finish frame
                }
            }
            NodeKind::ExprStmt => {
                let expr_node = *arena.node(NodeId(node.lhs));
                if expr_node.node_kind() == NodeKind::CallExpr {
                    if let Some((callee_id, arg_ids)) = unpack_call(arena, expr_node) {
                        let callee_node = *arena.node(callee_id);
                        let fn_name = if callee_node.node_kind() == NodeKind::IdentExpr {
                            arena.interner.resolve(SymbolId(callee_node.lhs)).to_string()
                        } else {
                            "anon".to_string()
                        };

                        if let Some((_, params, body_node, start_line)) = self.functions.get(&fn_name).cloned() {
                            let mut evaluated_args = Vec::new();
                            for arg_id in arg_ids {
                                let (val, ty) = self.eval_ast_node(arena, arg_id);
                                evaluated_args.push((val, ty));
                            }

                            let prog = self.program_path.clone().unwrap_or_default();
                            let line_index = pilot_span::LineIndex::new(self.source.as_bytes());
                            let fn_stmts = extract_fn_stmts(arena, &line_index, body_node);
                            let initial_line = fn_stmts.first().map(|s| s.1).unwrap_or(start_line);

                            self.push_frame_with_stmts(&fn_name, &prog, initial_line, 1, fn_stmts, None);

                            for (i, p) in params.iter().enumerate() {
                                if let Some((val, ty)) = evaluated_args.get(i) {
                                    self.set_local(p, *val, ty);
                                }
                            }
                            return;
                        }
                    }
                }
                self.eval_ast_node(arena, NodeId(node.lhs));
            }
            _ => {
                self.eval_ast_node(arena, node_id);
            }
        }
    }

    pub fn eval_ast_node(&mut self, arena: &AstArena, node_id: NodeId) -> (PilotVal, String) {
        if node_id.is_none() {
            return (PilotVal::NULL, "null".to_string());
        }

        let node = *arena.node(node_id);
        match node.node_kind() {
            NodeKind::IntLit => {
                let val = arena.i64_pool.get(node.lhs as usize).copied().unwrap_or(0);
                (PilotVal::from_int(val), "int".to_string())
            }
            NodeKind::FloatLit => {
                let val = arena.f64_pool.get(node.lhs as usize).copied().unwrap_or(0.0);
                (PilotVal::from_float(val), "float".to_string())
            }
            NodeKind::BoolLit => {
                let b = node.lhs != 0;
                (PilotVal::from_bool(b), "bool".to_string())
            }
            NodeKind::StrLit => {
                let s = arena.interner.resolve(SymbolId(node.lhs));
                (PilotVal::from_str(s), "str".to_string())
            }
            NodeKind::NoneLit => {
                (PilotVal::NULL, "none".to_string())
            }
            NodeKind::IdentExpr => {
                let name = arena.interner.resolve(SymbolId(node.lhs));
                if let Some(frame) = self.call_stack.last() {
                    if let Some(&v) = frame.locals.get(name) {
                        let ty = frame.local_types.get(name).cloned().unwrap_or_else(|| "any".to_string());
                        return (v, ty);
                    }
                }
                if let Some(&v) = self.globals.get(name) {
                    let ty = self.global_types.get(name).cloned().unwrap_or_else(|| "any".to_string());
                    return (v, ty);
                }
                match name {
                    "true" | "True" => (PilotVal::TRUE, "bool".to_string()),
                    "false" | "False" => (PilotVal::FALSE, "bool".to_string()),
                    "none" | "None" | "null" => (PilotVal::NULL, "none".to_string()),
                    _ => (PilotVal::NULL, "undefined".to_string()),
                }
            }
            NodeKind::BinaryExpr => {
                let (lhs_val, _) = self.eval_ast_node(arena, NodeId(node.lhs));
                let (rhs_val, _) = self.eval_ast_node(arena, NodeId(node.rhs));
                let op = node.flags;

                match op {
                    1 => { // Add
                        if lhs_val.is_int() && rhs_val.is_int() {
                            (PilotVal::from_int(lhs_val.as_int().unwrap() + rhs_val.as_int().unwrap()), "int".to_string())
                        } else if lhs_val.is_float() || rhs_val.is_float() {
                            let l = lhs_val.as_float().or_else(|| lhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                            let r = rhs_val.as_float().or_else(|| rhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                            (PilotVal::from_float(l + r), "float".to_string())
                        } else {
                            let mut s = lhs_val.to_string_repr();
                            s.push_str(&rhs_val.to_string_repr());
                            (PilotVal::from_str(&s), "str".to_string())
                        }
                    }
                    2 => { // Sub
                        let l = lhs_val.as_int().unwrap_or(0);
                        let r = rhs_val.as_int().unwrap_or(0);
                        (PilotVal::from_int(l - r), "int".to_string())
                    }
                    3 => { // Mul
                        let l = lhs_val.as_int().unwrap_or(0);
                        let r = rhs_val.as_int().unwrap_or(0);
                        (PilotVal::from_int(l * r), "int".to_string())
                    }
                    4 => { // Div
                        let l = lhs_val.as_int().unwrap_or(0);
                        let r = rhs_val.as_int().unwrap_or(1);
                        (PilotVal::from_int(if r != 0 { l / r } else { 0 }), "int".to_string())
                    }
                    6 => { // Rem
                        let l = lhs_val.as_int().unwrap_or(0);
                        let r = rhs_val.as_int().unwrap_or(1);
                        (PilotVal::from_int(if r != 0 { l % r } else { 0 }), "int".to_string())
                    }
                    8 => { // CmpEq
                        let eq = lhs_val == rhs_val;
                        (PilotVal::from_bool(eq), "bool".to_string())
                    }
                    9 => { // CmpNe
                        let ne = lhs_val != rhs_val;
                        (PilotVal::from_bool(ne), "bool".to_string())
                    }
                    10 => { // CmpLt
                        let lt = lhs_val.as_int().unwrap_or(0) < rhs_val.as_int().unwrap_or(0);
                        (PilotVal::from_bool(lt), "bool".to_string())
                    }
                    11 => { // CmpLe
                        let le = lhs_val.as_int().unwrap_or(0) <= rhs_val.as_int().unwrap_or(0);
                        (PilotVal::from_bool(le), "bool".to_string())
                    }
                    12 => { // CmpGt
                        let gt = lhs_val.as_int().unwrap_or(0) > rhs_val.as_int().unwrap_or(0);
                        (PilotVal::from_bool(gt), "bool".to_string())
                    }
                    13 => { // CmpGe
                        let ge = lhs_val.as_int().unwrap_or(0) >= rhs_val.as_int().unwrap_or(0);
                        (PilotVal::from_bool(ge), "bool".to_string())
                    }
                    _ => (PilotVal::from_int(0), "int".to_string()),
                }
            }
            NodeKind::UnaryExpr => {
                let (val, ty) = self.eval_ast_node(arena, NodeId(node.lhs));
                if node.flags == 2 { // Negate
                    (PilotVal::from_int(-val.as_int().unwrap_or(0)), "int".to_string())
                } else if node.flags == 3 { // Not
                    let b = !val.as_bool().unwrap_or(false);
                    (PilotVal::from_bool(b), "bool".to_string())
                } else {
                    (val, ty)
                }
            }
            NodeKind::CallExpr => {
                let (callee_id, arg_ids) = match unpack_call(arena, node) {
                    Some(c) => c,
                    None => return (PilotVal::NULL, "void".to_string()),
                };

                let callee_node = *arena.node(callee_id);
                let fn_name = if callee_node.node_kind() == NodeKind::IdentExpr {
                    arena.interner.resolve(SymbolId(callee_node.lhs)).to_string()
                } else {
                    "anon".to_string()
                };

                let mut evaluated_args = Vec::new();
                for arg_id in arg_ids {
                    let (val, ty) = self.eval_ast_node(arena, arg_id);
                    evaluated_args.push((val, ty));
                }

                match fn_name.as_str() {
                    "println" | "print" => {
                        let out_str = evaluated_args.iter()
                            .map(|(v, _)| v.to_string_repr())
                            .collect::<Vec<_>>()
                            .join(" ");
                        if fn_name == "println" {
                            self.pending_outputs.push(format!("{}\n", out_str));
                        } else {
                            self.pending_outputs.push(out_str);
                        }
                        (PilotVal::NULL, "void".to_string())
                    }
                    "assert" => {
                        let ok = evaluated_args.first().map_or(false, |(v, _)| v.as_bool().unwrap_or(false));
                        if !ok {
                            let msg = evaluated_args.get(1).map_or("Assertion failed".to_string(), |(v, _)| v.to_string_repr());
                            self.pending_outputs.push(format!("Assertion Error: {}\n", msg));
                        }
                        (PilotVal::NULL, "void".to_string())
                    }
                    "len" => {
                        let length = evaluated_args.first().map_or(0, |(v, _)| {
                            v.with_str(|s| s.len() as i64).unwrap_or(0)
                        });
                        (PilotVal::from_int(length), "int".to_string())
                    }
                    _ => {
                        // User function evaluated synchronously when called directly in an expression
                        if let Some((_, params, body_node, _)) = self.functions.get(&fn_name).cloned() {
                            let line_index = pilot_span::LineIndex::new(self.source.as_bytes());
                            let fn_stmts = extract_fn_stmts(arena, &line_index, body_node);
                            let prog = self.program_path.clone().unwrap_or_default();
                            let frame_id = self.push_frame_with_stmts(&fn_name, &prog, 1, 1, fn_stmts, None);

                            for (i, p) in params.iter().enumerate() {
                                if let Some((val, ty)) = evaluated_args.get(i) {
                                    self.set_local(p, *val, ty);
                                }
                            }

                            // Execute statements synchronously for this call
                            while self.call_stack.iter().any(|f| f.id == frame_id) {
                                self.step_one();
                                if let DebuggerState::Paused(_) = self.state {
                                    break;
                                }
                            }

                            let ret = self.call_stack.last()
                                .and_then(|f| f.locals.get("__return__").copied())
                                .unwrap_or(PilotVal::NULL);
                            (ret, "any".to_string())
                        } else {
                            (PilotVal::NULL, "any".to_string())
                        }
                    }
                }
            }
            NodeKind::BlockExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut last = (PilotVal::NULL, "null".to_string());
                for &child_id in children {
                    last = self.eval_ast_node(arena, NodeId(child_id));
                }
                last
            }
            _ => (PilotVal::NULL, "any".to_string()),
        }
    }

    pub fn step_over(&mut self) {
        if self.call_stack.is_empty() {
            self.state = DebuggerState::Terminated;
            return;
        }
        let target_depth = self.call_stack.len();
        self.state = DebuggerState::Running;

        self.step_one();
        while self.state == DebuggerState::Running && self.call_stack.len() > target_depth {
            self.step_one();
        }

        if self.state == DebuggerState::Running {
            self.state = DebuggerState::Paused("step".to_string());
        }
    }

    pub fn step_into(&mut self) {
        if self.call_stack.is_empty() {
            self.state = DebuggerState::Terminated;
            return;
        }
        self.state = DebuggerState::Running;
        self.step_one();
        if self.state == DebuggerState::Running {
            self.state = DebuggerState::Paused("step".to_string());
        }
    }

    pub fn step_out(&mut self) {
        if self.call_stack.len() <= 1 {
            self.continue_exec();
            return;
        }
        let target_depth = self.call_stack.len() - 1;
        self.state = DebuggerState::Running;
        while self.state == DebuggerState::Running && self.call_stack.len() > target_depth {
            self.step_one();
        }
        if self.state == DebuggerState::Running {
            self.state = DebuggerState::Paused("step".to_string());
        }
    }

    pub fn continue_exec(&mut self) {
        self.state = DebuggerState::Running;
        while self.state == DebuggerState::Running {
            self.step_one();
        }
    }

    pub fn pause(&mut self) {
        self.state = DebuggerState::Paused("pause".to_string());
    }

    pub fn evaluate(&self, expr: &str, frame_id: Option<u32>) -> Result<(PilotVal, String), String> {
        let trimmed = expr.trim();

        // 1. Check frame locals if frame_id specified
        if let Some(fid) = frame_id {
            if let Some(frame) = self.call_stack.iter().find(|f| f.id == fid) {
                if let Some(&val) = frame.locals.get(trimmed) {
                    let ty = frame.local_types.get(trimmed).cloned().unwrap_or_else(|| "any".to_string());
                    return Ok((val, ty));
                }
            }
        }

        // 2. Check topmost frame locals
        if let Some(frame) = self.call_stack.last() {
            if let Some(&val) = frame.locals.get(trimmed) {
                let ty = frame.local_types.get(trimmed).cloned().unwrap_or_else(|| "any".to_string());
                return Ok((val, ty));
            }
        }

        // 3. Check globals
        if let Some(&val) = self.globals.get(trimmed) {
            let ty = self.global_types.get(trimmed).cloned().unwrap_or_else(|| "any".to_string());
            return Ok((val, ty));
        }

        // 4. Literal parsing
        if let Ok(i) = trimmed.parse::<i64>() {
            return Ok((PilotVal::from_int(i), "i64".to_string()));
        }
        if let Ok(f) = trimmed.parse::<f64>() {
            return Ok((PilotVal::from_float(f), "f64".to_string()));
        }
        if trimmed == "true" {
            return Ok((PilotVal::TRUE, "bool".to_string()));
        }
        if trimmed == "false" {
            return Ok((PilotVal::FALSE, "bool".to_string()));
        }
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            let content = &trimmed[1..trimmed.len() - 1];
            return Ok((PilotVal::from_str(content), "str".to_string()));
        }

        // 5. Binary expression evaluation (e.g. "a + b" or "x * 2")
        if let Some((l_str, r_str)) = trimmed.split_once('+') {
            let (l_val, _) = self.evaluate(l_str, frame_id)?;
            let (r_val, _) = self.evaluate(r_str, frame_id)?;
            if l_val.is_int() && r_val.is_int() {
                return Ok((PilotVal::from_int(l_val.as_int().unwrap() + r_val.as_int().unwrap()), "i64".to_string()));
            }
        }
        if let Some((l_str, r_str)) = trimmed.split_once('*') {
            let (l_val, _) = self.evaluate(l_str, frame_id)?;
            let (r_val, _) = self.evaluate(r_str, frame_id)?;
            if l_val.is_int() && r_val.is_int() {
                return Ok((PilotVal::from_int(l_val.as_int().unwrap() * r_val.as_int().unwrap()), "i64".to_string()));
            }
        }

        Err(format!("Cannot evaluate '{}'", expr))
    }
}

pub fn extract_fn_stmts(arena: &AstArena, line_index: &pilot_span::LineIndex, body_id: NodeId) -> Vec<(NodeId, u32, u32)> {
    let mut stmts = Vec::new();
    if body_id.is_none() {
        return stmts;
    }
    let body_node = arena.node(body_id);
    if body_node.node_kind() == NodeKind::BlockExpr {
        for &cid in arena.children_slice(body_node.lhs, body_node.rhs) {
            let span = arena.spans.get(cid as usize).copied().unwrap_or(pilot_span::ByteSpan::new(0, 0));
            let (l, c) = line_index.offset_to_line_col(span.start);
            stmts.push((NodeId(cid), l, c));
        }
    } else {
        let span = arena.spans.get(body_id.0 as usize).copied().unwrap_or(pilot_span::ByteSpan::new(0, 0));
        let (l, c) = line_index.offset_to_line_col(span.start);
        stmts.push((body_id, l, c));
    }
    stmts
}
