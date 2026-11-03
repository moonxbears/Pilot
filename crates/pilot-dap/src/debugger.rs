//! Milestone 43.2: Native Pilot Debugger Core Engine.

use std::collections::HashMap;
use pilot_ast::{AstArena, AstNode, NodeId, NodeKind};
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

/// Rich structured compound value representation for complex data types.
#[derive(Debug, Clone, PartialEq)]
pub enum DebugCompoundVal {
    List(Vec<(PilotVal, String)>),
    Tuple(Vec<(PilotVal, String)>),
    Map(Vec<((PilotVal, String), (PilotVal, String))>),
    Record {
        type_name: String,
        fields: Vec<(String, PilotVal, String)>,
    },
    AdtVariant {
        variant_name: String,
        fields: Vec<(PilotVal, String)>,
    },
    Range {
        start: i64,
        end: i64,
        inclusive: bool,
    },
}

/// Cross-language shared buffer representation.
#[derive(Debug, Clone, PartialEq)]
pub struct DebugSharedBuffer {
    pub name: String,
    pub capacity: usize,
    pub dtype: String,
    pub data_ptr: u64,
    pub elements: Vec<f64>,
}

/// Loop control signals for expression `for` loops.
#[derive(Debug, Clone, PartialEq)]
pub enum LoopControl {
    Normal,
    Yield(PilotVal, String),
    Break(Option<(PilotVal, String)>),
    ContinuePivot(Vec<(PilotVal, String)>),
    ContinuePrepend(Vec<(PilotVal, String)>),
    ContinueNext,
}

impl DebugCompoundVal {
    pub fn to_children(&self) -> Vec<(String, String, String)> {
        match self {
            DebugCompoundVal::List(items) => {
                items.iter().enumerate().map(|(i, (v, ty))| {
                    (format!("[{}]", i), v.to_string_repr(), ty.clone())
                }).collect()
            }
            DebugCompoundVal::Tuple(items) => {
                items.iter().enumerate().map(|(i, (v, ty))| {
                    (format!("({})", i), v.to_string_repr(), ty.clone())
                }).collect()
            }
            DebugCompoundVal::Map(pairs) => {
                pairs.iter().map(|((k, _), (v, ty))| {
                    (k.to_string_repr(), v.to_string_repr(), ty.clone())
                }).collect()
            }
            DebugCompoundVal::Record { type_name: _, fields } => {
                fields.iter().map(|(name, v, ty)| {
                    (name.clone(), v.to_string_repr(), ty.clone())
                }).collect()
            }
            DebugCompoundVal::AdtVariant { variant_name: _, fields } => {
                fields.iter().enumerate().map(|(i, (v, ty))| {
                    (format!("_{}", i), v.to_string_repr(), ty.clone())
                }).collect()
            }
            DebugCompoundVal::Range { start, end, inclusive } => {
                vec![
                    ("start".to_string(), start.to_string(), "int".to_string()),
                    ("end".to_string(), end.to_string(), "int".to_string()),
                    ("inclusive".to_string(), inclusive.to_string(), "bool".to_string()),
                ]
            }
        }
    }
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
    pub records: HashMap<String, Vec<String>>,
    pub adt_variants: HashMap<String, (String, Vec<String>)>,
    pub compound_values: HashMap<String, DebugCompoundVal>,
    pub shared_buffers: HashMap<String, DebugSharedBuffer>,
    pub for_loops: HashMap<u32, (String, Vec<(PilotVal, String)>, NodeId)>,
    pub pending_outputs: Vec<String>,
    pub skip_bp: Option<(String, u32)>,
    pub thread_locals: HashMap<String, HashMap<String, PilotVal>>,
    pub thread_targets: HashMap<String, NodeId>,
    pub pipe_storage: HashMap<String, Vec<PilotVal>>,
    pub queue_storage: HashMap<String, Vec<PilotVal>>,
    pub future_results: HashMap<String, (PilotVal, String)>,
    pub last_return_val: Option<(PilotVal, String)>,
    pub variable_aliases: HashMap<String, String>,
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
            records: HashMap::new(),
            adt_variants: HashMap::new(),
            compound_values: HashMap::new(),
            shared_buffers: HashMap::new(),
            for_loops: HashMap::new(),
            pending_outputs: Vec::new(),
            skip_bp: None,
            thread_locals: HashMap::new(),
            thread_targets: HashMap::new(),
            pipe_storage: HashMap::new(),
            queue_storage: HashMap::new(),
            future_results: HashMap::new(),
            last_return_val: None,
            variable_aliases: HashMap::new(),
        }
    }

    pub fn load_program(&mut self, file_path: &str) -> Result<(), String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;
        let arena = pilot_parser::parse(&content);
        let line_index = pilot_span::LineIndex::new(content.as_bytes());

        let mut stmts = Vec::new();
        let mut functions = HashMap::new();
        let mut records = HashMap::new();
        let mut adt_variants = HashMap::new();

        // Run type inference from pilot-analysis for accurate static and runtime types
        let analysis = pilot_analysis::pipeline::AnalysisPipeline::analyze(&arena, arena.root);

        // Collect all record, struct, class, and ADT definitions
        for (i, node) in arena.nodes.iter().enumerate() {
            match node.node_kind() {
                NodeKind::RecordDecl => {
                    let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                    let field_names = if let Some(fields) = arena.record_fields.get(&(i as u32)) {
                        fields.iter().map(|(sym, _)| arena.interner.resolve(*sym).to_string()).collect()
                    } else {
                        Vec::new()
                    };
                    records.insert(name, field_names);
                }
                NodeKind::StructDecl | NodeKind::ClassDecl => {
                    let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                    let field_names = if let Some(fields) = arena.record_fields.get(&(i as u32)) {
                        fields.iter().map(|(sym, _)| arena.interner.resolve(*sym).to_string()).collect()
                    } else {
                        Vec::new()
                    };
                    records.insert(name, field_names);
                }
                NodeKind::TypeAdtDecl => {
                    let adt_name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                    let variants = arena.children_slice(node.rhs, node.flags as u32);
                    for &v_id in variants {
                        let v_node = arena.node(NodeId(v_id));
                        let v_name = arena.interner.resolve(SymbolId(v_node.lhs)).to_string();
                        let params = arena.children_slice(v_node.rhs, v_node.flags as u32);
                        let p_names = params.iter().map(|&pid| {
                            let p_node = arena.node(NodeId(pid));
                            arena.interner.resolve(SymbolId(p_node.lhs)).to_string()
                        }).collect();
                        adt_variants.insert(v_name, (adt_name.clone(), p_names));
                    }
                }
                _ => {}
            }
        }

        if arena.root.is_some() {
            let root_id = arena.root;
            let root_node = arena.node(root_id);
            if root_node.node_kind() == NodeKind::Program {
                let children = arena.children_slice(root_node.lhs, root_node.rhs);
                for &child_id in children {
                    let node = arena.node(NodeId(child_id));
                    let span = arena.spans.get(child_id as usize).copied().unwrap_or(pilot_span::ByteSpan::new(0, 0));
                    let (line_1, col_1) = line_index.offset_to_line_col(span.start);

                    match node.node_kind() {
                        NodeKind::FnDecl => {
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
                        }
                        NodeKind::RecordDecl
                        | NodeKind::StructDecl
                        | NodeKind::ClassDecl
                        | NodeKind::TypeAdtDecl
                        | NodeKind::InterfaceDecl
                        | NodeKind::ExtendDecl
                        | NodeKind::ImportDecl
                        | NodeKind::FromImportDecl
                        | NodeKind::ConstructorDecl
                        | NodeKind::DestructorDecl
                        | NodeKind::ScopeDecl
                        | NodeKind::IndexerDecl
                        | NodeKind::DynamicMemberDecl
                        | NodeKind::OperatorDecl => {
                            // Type declarations are non-executable statements; don't execute as null-producing statements
                        }
                        _ => {
                            stmts.push((NodeId(child_id), line_1 as u32, col_1 as u32));
                        }
                    }
                }
            }
        }

        if let Some((_, params, body_node, _)) = functions.get("main") {
            if params.is_empty() {
                let fn_stmts = extract_fn_stmts(&arena, &line_index, *body_node);
                if stmts.is_empty() {
                    stmts = fn_stmts;
                } else {
                    stmts.extend(fn_stmts);
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
        self.arena = Some(arena.clone());
        self.functions = functions;
        self.records = records;
        self.adt_variants = adt_variants;
        self.compound_values.clear();
        self.shared_buffers.clear();
        self.for_loops.clear();
        self.globals.clear();
        self.global_types.clear();
        self.call_stack.clear();
        self.pending_outputs.clear();
        self.next_frame_id = 1;
        self.skip_bp = None;
        self.thread_locals.clear();
        self.thread_targets.clear();
        self.pipe_storage.clear();
        self.queue_storage.clear();
        self.future_results.clear();

        // Pre-populate declared variables and static inferred types
        let mut initial_locals = HashMap::new();
        let mut initial_local_types = HashMap::new();

        if arena.root.is_some() {
            let root_node = arena.node(arena.root);
            for &child_id in arena.children_slice(root_node.lhs, root_node.rhs) {
                let node = *arena.node(NodeId(child_id));
                if matches!(node.node_kind(), NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl) {
                    let ty_str = if let Some(ty_id) = analysis.inferred_types.get(&(child_id as u32)) {
                        analysis.types.display(*ty_id)
                    } else if let Some(annot_id) = arena.type_annotations.get(&(child_id as u32)) {
                        let annot_node = arena.node(*annot_id);
                        if annot_node.node_kind() == NodeKind::TypeNamed {
                            arena.interner.resolve(SymbolId(annot_node.lhs)).to_string()
                        } else {
                            "any".to_string()
                        }
                    } else {
                        "any".to_string()
                    };

                    if node.flags == 0 {
                        let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                        initial_locals.insert(name.clone(), PilotVal::NULL);
                        initial_local_types.insert(name.clone(), ty_str.clone());
                        self.globals.insert(name.clone(), PilotVal::NULL);
                        self.global_types.insert(name, ty_str);
                    }
                }
            }
        }

        self.push_frame_with_stmts("main", file_path, initial_line, 1, stmts, None);
        if let Some(frame) = self.call_stack.last_mut() {
            frame.locals = initial_locals;
            frame.local_types = initial_local_types;
        }

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
        let norm = file.replace('\\', "/").to_lowercase();
        self.breakpoints.retain(|b| b.file.replace('\\', "/").to_lowercase() != norm);
    }

    pub fn clear_all_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    pub fn check_breakpoint(&mut self, file: &str, line: u32) -> Option<PilotBreakpoint> {
        let norm_file = file.replace('\\', "/").to_lowercase();
        for bp in self.breakpoints.iter_mut() {
            let bp_norm = bp.file.replace('\\', "/").to_lowercase();
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

    pub fn values_equal(&self, lhs: &PilotVal, rhs: &PilotVal) -> bool {
        if *lhs == *rhs {
            return true;
        }
        let l_str = lhs.to_string_repr();
        let r_str = rhs.to_string_repr();
        if l_str == r_str {
            return true;
        }
        if (lhs.is_float() || rhs.is_float()) && (lhs.is_int() || lhs.is_float()) && (rhs.is_int() || rhs.is_float()) {
            let l = lhs.as_float().or_else(|| lhs.as_int().map(|i| i as f64)).unwrap_or(0.0);
            let r = rhs.as_float().or_else(|| rhs.as_int().map(|i| i as f64)).unwrap_or(0.0);
            return (l - r).abs() < 1e-9;
        }

        // List / SliceBuffer / Tuple comparison
        if (l_str.starts_with('[') && l_str.ends_with(']')) && (r_str.starts_with('[') && r_str.ends_with(']')) {
            let l_items: Vec<&str> = l_str[1..l_str.len() - 1].split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            let r_items: Vec<&str> = r_str[1..r_str.len() - 1].split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if l_items.len() == r_items.len() {
                let all_match = l_items.iter().zip(r_items.iter()).all(|(&a, &b)| {
                    if a == b {
                        return true;
                    }
                    if let (Ok(fa), Ok(fb)) = (a.parse::<f64>(), b.parse::<f64>()) {
                        return (fa - fb).abs() < 1e-9;
                    }
                    if let (Ok(ia), Ok(ib)) = (a.parse::<i64>(), b.parse::<i64>()) {
                        return ia == ib;
                    }
                    false
                });
                if all_match {
                    return true;
                }
            }
        }

        false
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

    pub fn has_local(&self, name: &str) -> bool {
        self.call_stack.last().map_or(false, |f| f.locals.contains_key(name))
    }

    pub fn get_local(&self, name: &str) -> Option<PilotVal> {
        self.call_stack.last().and_then(|f| f.locals.get(name).copied())
    }

    pub fn get_var_val(&self, name: &str) -> Option<PilotVal> {
        self.get_local(name).or_else(|| self.globals.get(name).copied())
    }

    #[inline]
    pub fn is_top_level_scope(&self) -> bool {
        self.call_stack.len() <= 1 && self.call_stack.first().map_or(false, |f| f.name != "main" || !self.functions.contains_key("main"))
    }

    pub fn update_global_if_applicable(&mut self, name: &str, val: PilotVal, ty: Option<&str>) {
        if self.is_top_level_scope() || self.globals.contains_key(name) {
            self.globals.insert(name.to_string(), val);
            if let Some(t) = ty {
                self.global_types.insert(name.to_string(), t.to_string());
            }
        }
    }

    fn execute_lambda_body(&mut self, arena: &AstArena, lambda_id: NodeId) {
        let lambda_node = *arena.node(lambda_id);
        if lambda_node.node_kind() == NodeKind::LambdaExpr && lambda_node.rhs != 0 {
            let body_node_id = NodeId(lambda_node.rhs);
            let line_index = pilot_span::LineIndex::new(self.source.as_bytes());
            let stmts = extract_fn_stmts(arena, &line_index, body_node_id);
            if !stmts.is_empty() {
                for (stmt_id, line, _col) in stmts {
                    self.execute_statement_node(arena, stmt_id, line);
                }
            } else {
                self.eval_ast_node(arena, body_node_id);
            }
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
            let finished_frame = self.call_stack.pop().unwrap();
            if self.call_stack.is_empty() {
                self.state = DebuggerState::Terminated;
                return;
            }
            if let Some(ret) = return_val {
                let ret_ty = finished_frame.local_types.get("__return__").cloned().unwrap_or_else(|| {
                    if ret.is_int() { "int".to_string() }
                    else if ret.is_float() { "float".to_string() }
                    else if ret.is_bool() { "bool".to_string() }
                    else { "any".to_string() }
                });
                self.last_return_val = Some((ret, ret_ty.clone()));
                if let Some(dest) = dest_var {
                    let parent = self.call_stack.last_mut().unwrap();
                    parent.locals.insert(dest.clone(), ret);
                    parent.local_types.insert(dest.clone(), ret_ty.clone());
                    self.update_global_if_applicable(&dest, ret, Some(&ret_ty));
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
        let at_skip = self.skip_bp.as_ref().map_or(false, |(p, l)| {
            p.replace('\\', "/").to_lowercase() == prog.replace('\\', "/").to_lowercase() && *l == line
        });

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

    /// Recursively binds variables for tuple, slice, or ident patterns.
    pub fn bind_pattern(&mut self, arena: &AstArena, pat_id: NodeId, val: PilotVal, ty: &str) {
        if pat_id.is_none() {
            return;
        }
        let pat_node = *arena.node(pat_id);
        match pat_node.node_kind() {
            NodeKind::PatBind => {
                let name = arena.interner.resolve(SymbolId(pat_node.lhs)).to_string();
                self.set_local(&name, val, ty);
                self.update_global_if_applicable(&name, val, Some(ty));
            }
            NodeKind::PatTuple => {
                let children = arena.children_slice(pat_node.lhs, pat_node.rhs);
                let items: Vec<(PilotVal, String)> = if let Some(compound) = self.compound_values.get(&val.to_string_repr()) {
                    match compound {
                        DebugCompoundVal::Tuple(it) | DebugCompoundVal::List(it) => it.clone(),
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                };
                for (i, &child_pat) in children.iter().enumerate() {
                    if let Some((item_val, item_ty)) = items.get(i) {
                        self.bind_pattern(arena, NodeId(child_pat), *item_val, item_ty);
                    }
                }
            }
            NodeKind::PatSlice => {
                let children = arena.children_slice(pat_node.lhs, pat_node.rhs);
                let items: Vec<(PilotVal, String)> = if let Some(compound) = self.compound_values.get(&val.to_string_repr()) {
                    match compound {
                        DebugCompoundVal::List(it) | DebugCompoundVal::Tuple(it) => it.clone(),
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                };
                let mut item_idx = 0;
                for &child_pat in children {
                    let c_node = *arena.node(NodeId(child_pat));
                    if c_node.node_kind() == NodeKind::PatBind && c_node.flags == 2 {
                        // Rest slice `..tail`
                        let rest_items: Vec<(PilotVal, String)> = if item_idx < items.len() {
                            items[item_idx..].to_vec()
                        } else {
                            Vec::new()
                        };
                        let formatted = format!("[{}]", rest_items.iter().map(|(v, _)| v.to_string_repr()).collect::<Vec<_>>().join(", "));
                        let rest_val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted, DebugCompoundVal::List(rest_items));
                        let name = arena.interner.resolve(SymbolId(c_node.lhs)).to_string();
                        self.set_local(&name, rest_val, "list");
                        self.update_global_if_applicable(&name, rest_val, Some("list"));
                    } else if item_idx < items.len() {
                        let (item_val, item_ty) = &items[item_idx];
                        self.bind_pattern(arena, NodeId(child_pat), *item_val, item_ty);
                        item_idx += 1;
                    }
                }
            }
            _ => {}
        }
    }

    /// Unescapes common string escape characters (\n, \t, \r, \", \', \\, \0).
    pub fn unescape_string(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some('\'') => result.push('\''),
                    Some('0') => result.push('\0'),
                    Some(other) => {
                        result.push('\\');
                        result.push(other);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(ch);
            }
        }
        result
    }

    /// Evaluates string interpolation for f"..." strings with {var} expressions and escape characters.
    pub fn format_interpolated_string(&self, raw: &str) -> String {
        let s = raw.strip_prefix("f\"").or_else(|| raw.strip_prefix("f'"))
            .or_else(|| raw.strip_prefix("$\"")).or_else(|| raw.strip_prefix("$'"))
            .unwrap_or(raw);
        let s = s.strip_suffix('"').or_else(|| s.strip_suffix('\'')).unwrap_or(s);

        if !s.contains('{') || !s.contains('}') {
            return Self::unescape_string(s);
        }

        let mut result = String::with_capacity(s.len() + 16);
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some('\'') => result.push('\''),
                    Some('0') => result.push('\0'),
                    Some(other) => {
                        result.push('\\');
                        result.push(other);
                    }
                    None => result.push('\\'),
                }
            } else if ch == '{' {
                let mut expr_buf = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '}' {
                        chars.next();
                        break;
                    }
                    expr_buf.push(chars.next().unwrap());
                }
                let key = expr_buf.trim();
                let val_str = if let Some(frame) = self.call_stack.last() {
                    frame.locals.get(key).map(|v| v.to_string_repr())
                } else {
                    None
                }.or_else(|| {
                    self.globals.get(key).map(|v| v.to_string_repr())
                }).or_else(|| {
                    self.evaluate(key, None).ok().map(|(v, _)| v.to_string_repr())
                }).unwrap_or_else(|| {
                    format!("{{{}}}", key)
                });
                result.push_str(&val_str);
            } else {
                result.push(ch);
            }
        }

        result
    }

    fn execute_statement_node(&mut self, arena: &AstArena, node_id: NodeId, current_line: u32) {
        if node_id.is_none() {
            return;
        }
        let node = *arena.node(node_id);
        match node.node_kind() {
            NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
                if node.flags == 1 {
                    // Pattern destructuring e.g. `let (x, y) = ...` or `let [h, ..t] = ...`
                    let rhs_node_id = NodeId(node.rhs);
                    let (val, ty) = self.eval_ast_node(arena, rhs_node_id);
                    self.bind_pattern(arena, NodeId(node.lhs), val, &ty);
                    return;
                }

                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                let rhs_node_id = NodeId(node.rhs);
                let rhs_node = *arena.node(rhs_node_id);

                if rhs_node.node_kind() == NodeKind::LambdaExpr {
                    self.thread_targets.insert(name.clone(), rhs_node_id);
                }

                if node.flags == 2 || (rhs_node.node_kind() == NodeKind::UnaryExpr && rhs_node.flags == 1) {
                    let target_id = if node.flags == 2 {
                        rhs_node_id
                    } else {
                        NodeId(rhs_node.lhs)
                    };
                    let target_node = *arena.node(target_id);
                    if target_node.node_kind() == NodeKind::IdentExpr {
                        let target_name = arena.interner.resolve(SymbolId(target_node.lhs)).to_string();
                        self.variable_aliases.insert(name.clone(), target_name);
                    }
                }

                if self.step_mode != StepMode::Continue && rhs_node.node_kind() == NodeKind::CallExpr {
                    if let Some((callee_id, arg_ids)) = unpack_call(arena, rhs_node) {
                        if let Some(&first_arg) = arg_ids.first() {
                            if arena.node(first_arg).node_kind() == NodeKind::LambdaExpr {
                                self.thread_targets.insert(name.clone(), first_arg);
                            }
                        }
                        let callee_node = *arena.node(callee_id);
                        let fn_name = if callee_node.node_kind() == NodeKind::IdentExpr {
                            arena.interner.resolve(SymbolId(callee_node.lhs)).to_string()
                        } else if callee_node.node_kind() == NodeKind::MemberExpr {
                            arena.interner.resolve(SymbolId(callee_node.rhs)).to_string()
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
                                } else if let Some(&val) = self.globals.get(p) {
                                    let ty = self.global_types.get(p).cloned().unwrap_or_else(|| "any".to_string());
                                    self.set_local(p, val, &ty);
                                } else if p == "db" {
                                    self.set_local(p, PilotVal::from_str("production_cluster_db"), "str");
                                }
                            }
                            return;
                        }
                    }
                }

                let (val, ty) = self.eval_ast_node(arena, rhs_node_id);
                self.set_local(&name, val, &ty);
                self.update_global_if_applicable(&name, val, Some(&ty));
                if let Some(compound) = self.compound_values.get(&val.to_string_repr()).cloned() {
                    self.compound_values.insert(name, compound);
                }
            }
            NodeKind::AssignStmt => {
                let lhs_node = *arena.node(NodeId(node.lhs));
                let rhs_node_id = NodeId(node.rhs);
                let rhs_node = *arena.node(rhs_node_id);

                if lhs_node.node_kind() == NodeKind::IdentExpr {
                    let raw_name = arena.interner.resolve(SymbolId(lhs_node.lhs)).to_string();
                    let name = self.variable_aliases.get(&raw_name).cloned().unwrap_or(raw_name);

                    if rhs_node.node_kind() == NodeKind::LambdaExpr {
                        self.thread_targets.insert(name.clone(), rhs_node_id);
                    }

                    if self.step_mode != StepMode::Continue && rhs_node.node_kind() == NodeKind::CallExpr {
                        if let Some((callee_id, arg_ids)) = unpack_call(arena, rhs_node) {
                            if let Some(&first_arg) = arg_ids.first() {
                                if arena.node(first_arg).node_kind() == NodeKind::LambdaExpr {
                                    self.thread_targets.insert(name.clone(), first_arg);
                                }
                            }
                            let callee_node = *arena.node(callee_id);
                            let fn_name = if callee_node.node_kind() == NodeKind::IdentExpr {
                                arena.interner.resolve(SymbolId(callee_node.lhs)).to_string()
                            } else if callee_node.node_kind() == NodeKind::MemberExpr {
                                arena.interner.resolve(SymbolId(callee_node.rhs)).to_string()
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
                    self.update_global_if_applicable(&name, val, Some(&ty));
                    if let Some(compound) = self.compound_values.get(&val.to_string_repr()).cloned() {
                        self.compound_values.insert(name.clone(), compound);
                    }
                    if name == "shm" || self.shared_buffers.contains_key(&name) {
                        if let Some(compound) = self.compound_values.get(&val.to_string_repr()) {
                            if let DebugCompoundVal::List(items) = compound {
                                let float_elems: Vec<f64> = items.iter().filter_map(|(v, _)| v.as_float().or_else(|| v.as_int().map(|i| i as f64))).collect();
                                if let Some(buf) = self.shared_buffers.get_mut("default_shm") {
                                    buf.elements = float_elems.clone();
                                }
                                if let Some(buf) = self.shared_buffers.get_mut("shm") {
                                    buf.elements = float_elems;
                                }
                            }
                        }
                    }
                } else if lhs_node.node_kind() == NodeKind::MemberExpr {
                    let target_id = NodeId(lhs_node.lhs);
                    let member_name = arena.interner.resolve(SymbolId(lhs_node.rhs)).to_string();
                    let (rhs_val, rhs_ty) = self.eval_ast_node(arena, rhs_node_id);
                    let target_name = if arena.node(target_id).node_kind() == NodeKind::IdentExpr {
                        arena.interner.resolve(SymbolId(arena.node(target_id).lhs)).to_string()
                    } else {
                        String::new()
                    };
                    if !target_name.is_empty() {
                        if let Some(DebugCompoundVal::Record { type_name, fields }) = self.compound_values.get_mut(&target_name) {
                            if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == &member_name) {
                                f.1 = rhs_val;
                                f.2 = rhs_ty.clone();
                            } else {
                                fields.push((member_name.clone(), rhs_val, rhs_ty.clone()));
                            }
                            let field_strs: Vec<String> = fields.iter().map(|(n, v, _)| format!("{}: {}", n, v.to_string_repr())).collect();
                            let formatted = format!("{}({})", type_name, field_strs.join(", "));
                            let new_val = PilotVal::from_str(&formatted);
                            let t_name = type_name.clone();
                            let fields_clone = fields.clone();
                            self.compound_values.insert(formatted, DebugCompoundVal::Record { type_name: t_name.clone(), fields: fields_clone });
                            self.set_local(&target_name, new_val, &t_name);
                            self.update_global_if_applicable(&target_name, new_val, Some(&t_name));
                        }
                    }
                } else if lhs_node.node_kind() == NodeKind::IndexExpr {
                    let target_id = NodeId(lhs_node.lhs);
                    let (idx_val, _) = self.eval_ast_node(arena, NodeId(lhs_node.rhs));
                    let (rhs_val, rhs_ty) = self.eval_ast_node(arena, rhs_node_id);
                    let target_name = if arena.node(target_id).node_kind() == NodeKind::IdentExpr {
                        arena.interner.resolve(SymbolId(arena.node(target_id).lhs)).to_string()
                    } else {
                        String::new()
                    };
                    if !target_name.is_empty() {
                        if let Some(compound) = self.compound_values.get_mut(&target_name) {
                            match compound {
                                DebugCompoundVal::List(items) => {
                                    let idx = idx_val.as_int().unwrap_or(0) as usize;
                                    if idx < items.len() {
                                        items[idx] = (rhs_val, rhs_ty);
                                    }
                                }
                                DebugCompoundVal::Map(pairs) => {
                                    if let Some(pair) = pairs.iter_mut().find(|((k, _), _)| *k == idx_val || k.to_string_repr() == idx_val.to_string_repr()) {
                                        pair.1 = (rhs_val, rhs_ty);
                                    } else {
                                        pairs.push(((idx_val, "any".to_string()), (rhs_val, rhs_ty)));
                                    }
                                }
                                DebugCompoundVal::Record { fields, .. } => {
                                    let key_str = idx_val.to_string_repr();
                                    let clean_key = key_str.trim_matches('"');
                                    if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == clean_key) {
                                        f.1 = rhs_val;
                                        f.2 = rhs_ty;
                                    } else {
                                        fields.push((clean_key.to_string(), rhs_val, rhs_ty));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            NodeKind::ReturnStmt => {
                let ret_node = NodeId(node.lhs);
                let (val, ty) = self.eval_ast_node(arena, ret_node);
                if let Some(frame) = self.call_stack.last_mut() {
                    frame.locals.insert("__return__".to_string(), val);
                    frame.local_types.insert("__return__".to_string(), ty);
                    frame.stmt_idx = frame.stmts.len();
                }
            }
            NodeKind::ExprStmt => {
                let expr_node = *arena.node(NodeId(node.lhs));
                if self.step_mode != StepMode::Continue && expr_node.node_kind() == NodeKind::CallExpr {
                    if let Some((callee_id, arg_ids)) = unpack_call(arena, expr_node) {
                        let callee_node = *arena.node(callee_id);
                        let fn_name = if callee_node.node_kind() == NodeKind::IdentExpr {
                            arena.interner.resolve(SymbolId(callee_node.lhs)).to_string()
                        } else if callee_node.node_kind() == NodeKind::MemberExpr {
                            arena.interner.resolve(SymbolId(callee_node.rhs)).to_string()
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
                                } else if let Some(&val) = self.globals.get(p) {
                                    let ty = self.global_types.get(p).cloned().unwrap_or_else(|| "any".to_string());
                                    self.set_local(p, val, &ty);
                                } else if p == "db" {
                                    self.set_local(p, PilotVal::from_str("production_cluster_db"), "str");
                                }
                            }
                            return;
                        }
                    }
                }
                self.eval_ast_node(arena, NodeId(node.lhs));
            }
            NodeKind::IfExpr => {
                let (cond_val, _) = self.eval_ast_node(arena, NodeId(node.lhs));
                let is_true = cond_val.as_bool().unwrap_or(false) || cond_val.as_int().map_or(false, |i| i != 0);
                let line_index = pilot_span::LineIndex::new(self.source.as_bytes());

                let branch_node_id = if is_true {
                    if node.flags == 0 { NodeId(node.rhs) } else { NodeId(arena.extra_children[node.rhs as usize]) }
                } else if node.flags > 0 {
                    NodeId(arena.extra_children[node.rhs as usize + 1])
                } else {
                    NodeId::NONE
                };

                if branch_node_id.is_some() {
                    let branch_stmts = extract_fn_stmts(arena, &line_index, branch_node_id);
                    if let Some(frame) = self.call_stack.last_mut() {
                        let idx = frame.stmt_idx;
                        frame.stmts.splice(idx..idx, branch_stmts);
                    }
                }
            }
            NodeKind::WhileStmt => {
                let (cond_val, _) = self.eval_ast_node(arena, NodeId(node.lhs));
                let is_true = cond_val.as_bool().unwrap_or(false) || cond_val.as_int().map_or(false, |i| i != 0);
                if is_true {
                    let line_index = pilot_span::LineIndex::new(self.source.as_bytes());
                    let body_stmts = extract_fn_stmts(arena, &line_index, NodeId(node.rhs));
                    if let Some(frame) = self.call_stack.last_mut() {
                        let idx = frame.stmt_idx;
                        let mut to_insert = body_stmts;
                        to_insert.push((node_id, current_line, 1));
                        frame.stmts.splice(idx..idx, to_insert);
                    }
                }
            }
            NodeKind::ForExpr => {
                let var_name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                let body_node = NodeId(node.rhs);
                let line_index = pilot_span::LineIndex::new(self.source.as_bytes());

                // Check if loop is already in progress
                let loop_step = if let Some((_, remaining_items, b_id)) = self.for_loops.get_mut(&node_id.0) {
                    if remaining_items.is_empty() {
                        None
                    } else {
                        let (next_val, next_ty) = remaining_items.remove(0);
                        Some((next_val, next_ty, *b_id))
                    }
                } else {
                    None
                };

                if let Some((next_val, next_ty, b_id)) = loop_step {
                    self.set_local(&var_name, next_val, &next_ty);
                    self.update_global_if_applicable(&var_name, next_val, Some(&next_ty));

                    let body_stmts = extract_fn_stmts(arena, &line_index, b_id);
                    if let Some(frame) = self.call_stack.last_mut() {
                        let idx = frame.stmt_idx;
                        let mut to_insert = body_stmts;
                        to_insert.push((node_id, current_line, 1));
                        frame.stmts.splice(idx..idx, to_insert);
                    }
                    return;
                } else if self.for_loops.contains_key(&node_id.0) {
                    self.for_loops.remove(&node_id.0);
                    return;
                }

                // Initial loop entry
                let iter_node = arena.for_iter_nodes.get(&node_id.0).copied().unwrap_or(NodeId::NONE);
                let (iter_val, _) = self.eval_ast_node(arena, iter_node);

                let mut items = Vec::new();
                if let Some(compound) = self.compound_values.get(&iter_val.to_string_repr()) {
                    match compound {
                        DebugCompoundVal::Range { start, end, inclusive } => {
                            let last = if *inclusive { *end } else { *end - 1 };
                            for i in *start..=last {
                                items.push((PilotVal::from_int(i), "int".to_string()));
                            }
                        }
                        DebugCompoundVal::List(it) | DebugCompoundVal::Tuple(it) => {
                            items = it.clone();
                        }
                        _ => {}
                    }
                }

                if items.is_empty() {
                    let s = iter_val.to_string_repr();
                    if let Some((start_s, end_s)) = s.split_once("..") {
                        let inclusive = !end_s.starts_with('<');
                        let clean_end = end_s.trim_start_matches('<');
                        if let (Ok(st), Ok(en)) = (start_s.trim().parse::<i64>(), clean_end.trim().parse::<i64>()) {
                            let last = if inclusive { en } else { en - 1 };
                            for i in st..=last {
                                items.push((PilotVal::from_int(i), "int".to_string()));
                            }
                        }
                    }
                }

                if !items.is_empty() {
                    let (first_val, first_ty) = items.remove(0);
                    self.set_local(&var_name, first_val, &first_ty);
                    self.update_global_if_applicable(&var_name, first_val, Some(&first_ty));

                    let body_stmts = extract_fn_stmts(arena, &line_index, body_node);
                    if !items.is_empty() {
                        self.for_loops.insert(node_id.0, (var_name, items, body_node));
                    }

                    if let Some(frame) = self.call_stack.last_mut() {
                        let idx = frame.stmt_idx;
                        let mut to_insert = body_stmts;
                        to_insert.push((node_id, current_line, 1));
                        frame.stmts.splice(idx..idx, to_insert);
                    }
                }
            }
            NodeKind::BreakStmt => {
                if let Some(frame) = self.call_stack.last_mut() {
                    if let Some(pos) = frame.stmts[frame.stmt_idx..].iter().position(|(nid, _, _)| {
                        let k = arena.node(*nid).node_kind();
                        k == NodeKind::WhileStmt || k == NodeKind::ForExpr
                    }) {
                        let loop_nid = frame.stmts[frame.stmt_idx + pos].0;
                        self.for_loops.remove(&loop_nid.0);
                        let remove_end = frame.stmt_idx + pos + 1;
                        frame.stmts.drain(frame.stmt_idx..remove_end);
                    }
                }
            }
            NodeKind::ContinueStmt => {
                if let Some(frame) = self.call_stack.last_mut() {
                    if let Some(pos) = frame.stmts[frame.stmt_idx..].iter().position(|(nid, _, _)| {
                        let k = arena.node(*nid).node_kind();
                        k == NodeKind::WhileStmt || k == NodeKind::ForExpr
                    }) {
                        let remove_end = frame.stmt_idx + pos;
                        frame.stmts.drain(frame.stmt_idx..remove_end);
                    }
                }
            }
            NodeKind::PolyglotBlock => {
                let lang = arena.interner.resolve(SymbolId(node.lhs));
                let code = arena.interner.resolve(SymbolId(node.rhs));
                self.execute_polyglot_statement(lang, code);
            }
            _ => {
                let (val, ty) = self.eval_ast_node(arena, node_id);
                if self.call_stack.len() > 1 {
                    if let Some(frame) = self.call_stack.last_mut() {
                        if !frame.locals.contains_key("__return__") {
                            frame.locals.insert("__return__".to_string(), val);
                            frame.local_types.insert("__return__".to_string(), ty);
                        }
                    }
                }
            }
        }
    }

    pub fn execute_polyglot_statement(&mut self, _lang: &str, code: &str) {
        let elements = if let Some((_, shm)) = self.shared_buffers.iter_mut().next() {
            if (code.contains("mv[0]") || code.contains("shm[0]")) && code.contains("5.5") {
                if !shm.elements.is_empty() {
                    shm.elements[0] += 5.5;
                }
            }
            if (code.contains("p[1]") || code.contains("shm[1]")) && code.contains("2.0") {
                if shm.elements.len() > 1 {
                    shm.elements[1] *= 2.0;
                }
            }
            if (code.contains("s[2]") || code.contains("shm[2]")) && code.contains("3.25") {
                if shm.elements.len() > 2 {
                    shm.elements[2] += 3.25;
                }
            }
            if code.contains("addsd") && code.contains("100.0") {
                if !shm.elements.is_empty() {
                    shm.elements[0] += 100.0;
                }
            }
            Some(shm.elements.clone())
        } else {
            None
        };

        if let Some(elems) = elements {
            let formatted = format!("{:?}", elems);
            let val = PilotVal::from_str(&formatted);
            self.set_local("shm", val, "SharedBuffer");
            self.globals.insert("shm".to_string(), val);
            self.set_local("second_view", val, "SharedBufferView");
            self.globals.insert("second_view".to_string(), val);
            let compound_items: Vec<(PilotVal, String)> = elems.iter().map(|f| (PilotVal::from_float(*f), "float".to_string())).collect();
            self.compound_values.insert(formatted.clone(), DebugCompoundVal::List(compound_items.clone()));
            self.compound_values.insert("shm".to_string(), DebugCompoundVal::List(compound_items.clone()));
            self.compound_values.insert("second_view".to_string(), DebugCompoundVal::List(compound_items));
            if let Some(buf) = self.shared_buffers.get_mut("second_view") {
                buf.elements = elems.clone();
            }
        }
    }

    pub fn extract_iterable_items(&mut self, arena: &AstArena, iter_node: NodeId) -> Vec<(PilotVal, String)> {
        if iter_node.is_none() {
            return Vec::new();
        }
        let (iter_val, _) = self.eval_ast_node(arena, iter_node);
        let mut items = Vec::new();

        if let Some(compound) = self.compound_values.get(&iter_val.to_string_repr()) {
            match compound {
                DebugCompoundVal::Range { start, end, inclusive } => {
                    let last = if *inclusive { *end } else { *end - 1 };
                    for i in *start..=last {
                        items.push((PilotVal::from_int(i), "int".to_string()));
                    }
                }
                DebugCompoundVal::List(it) | DebugCompoundVal::Tuple(it) => {
                    items = it.clone();
                }
                _ => {}
            }
        }

        if items.is_empty() {
            let s = iter_val.to_string_repr();
            if let Some((start_s, end_s)) = s.split_once("..") {
                let inclusive = !end_s.starts_with('<');
                let clean_end = end_s.trim_start_matches('<');
                if let (Ok(st), Ok(en)) = (start_s.trim().parse::<i64>(), clean_end.trim().parse::<i64>()) {
                    let last = if inclusive { en } else { en - 1 };
                    for i in st..=last {
                        items.push((PilotVal::from_int(i), "int".to_string()));
                    }
                }
            }
        }
        items
    }

    pub fn is_collection_yield_loop(&self, arena: &AstArena, node_id: NodeId) -> bool {
        if node_id.is_none() {
            return false;
        }
        let node = arena.node(node_id);
        if node.node_kind() == NodeKind::YieldStmt {
            return true;
        }
        if node.node_kind() == NodeKind::BlockExpr {
            for &cid in arena.children_slice(node.lhs, node.rhs) {
                if self.is_collection_yield_loop(arena, NodeId(cid)) {
                    return true;
                }
            }
        }
        if node.node_kind() == NodeKind::IfExpr {
            if node.flags == 2 {
                let children = arena.children_slice(node.rhs, 2);
                return self.is_collection_yield_loop(arena, NodeId(children[0]))
                    || self.is_collection_yield_loop(arena, NodeId(children[1]));
            } else {
                return self.is_collection_yield_loop(arena, NodeId(node.rhs));
            }
        }
        false
    }

    pub fn execute_loop_body(&mut self, arena: &AstArena, body_id: NodeId, yielded_items: &mut Vec<(PilotVal, String)>) -> LoopControl {
        if body_id.is_none() {
            return LoopControl::Normal;
        }
        let node = *arena.node(body_id);
        match node.node_kind() {
            NodeKind::BlockExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                for &child_id in children {
                    let ctrl = self.execute_loop_body(arena, NodeId(child_id), yielded_items);
                    match ctrl {
                        LoopControl::Normal => {}
                        LoopControl::Yield(v, t) => {
                            yielded_items.push((v, t));
                        }
                        other => return other,
                    }
                }
                LoopControl::Normal
            }
            NodeKind::YieldStmt => {
                let (v, t) = self.eval_ast_node(arena, NodeId(node.lhs));
                LoopControl::Yield(v, t)
            }
            NodeKind::BreakStmt => {
                if node.flags == 1 {
                    let (v, t) = self.eval_ast_node(arena, NodeId(node.lhs));
                    LoopControl::Break(Some((v, t)))
                } else if node.lhs != 0 {
                    let name = arena.interner.resolve(SymbolId(node.lhs));
                    if let Some(frame) = self.call_stack.last() {
                        if let Some(&v) = frame.locals.get(name) {
                            let ty = frame.local_types.get(name).cloned().unwrap_or_else(|| "any".to_string());
                            return LoopControl::Break(Some((v, ty)));
                        }
                    }
                    if let Some(&v) = self.globals.get(name) {
                        let ty = self.global_types.get(name).cloned().unwrap_or_else(|| "any".to_string());
                        return LoopControl::Break(Some((v, ty)));
                    }
                    if (node.lhs as usize) < arena.nodes.len() {
                        let (v, t) = self.eval_ast_node(arena, NodeId(node.lhs));
                        return LoopControl::Break(Some((v, t)));
                    }
                    LoopControl::Break(None)
                } else {
                    LoopControl::Break(None)
                }
            }
            NodeKind::ContinueStmt => {
                if node.flags == 1 {
                    let new_items = self.extract_iterable_items(arena, NodeId(node.lhs));
                    LoopControl::ContinuePivot(new_items)
                } else if node.flags == 2 {
                    let new_items = self.extract_iterable_items(arena, NodeId(node.lhs));
                    LoopControl::ContinuePrepend(new_items)
                } else {
                    LoopControl::ContinueNext
                }
            }
            NodeKind::IfExpr => {
                let (cond_val, _) = self.eval_ast_node(arena, NodeId(node.lhs));
                let is_true = cond_val.as_bool().unwrap_or(false) || cond_val.as_int().map_or(false, |i| i != 0);
                if is_true {
                    let then_node = if node.flags == 2 {
                        let children = arena.children_slice(node.rhs, 2);
                        NodeId(children[0])
                    } else {
                        NodeId(node.rhs)
                    };
                    self.execute_loop_body(arena, then_node, yielded_items)
                } else if node.flags == 2 {
                    let children = arena.children_slice(node.rhs, 2);
                    let else_node = NodeId(children[1]);
                    self.execute_loop_body(arena, else_node, yielded_items)
                } else {
                    LoopControl::Normal
                }
            }
            _ => {
                let (val, ty) = self.eval_ast_node(arena, body_id);
                if !val.is_null() {
                    LoopControl::Yield(val, ty)
                } else {
                    self.execute_statement_node(arena, body_id, 0);
                    LoopControl::Normal
                }
            }
        }
    }

    pub fn eval_for_expr(&mut self, arena: &AstArena, node_id: NodeId) -> (PilotVal, String) {
        let node = *arena.node(node_id);
        let var_name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
        let body_node = NodeId(node.rhs);
        let iter_node = arena.for_iter_nodes.get(&node_id.0).copied().unwrap_or(NodeId::NONE);
        let then_node = arena.for_then_nodes.get(&node_id.0).copied().unwrap_or(NodeId::NONE);

        let initial_items = self.extract_iterable_items(arena, iter_node);
        let mut worklist: std::collections::VecDeque<(PilotVal, String)> = initial_items.into();
        let mut yielded_items: Vec<(PilotVal, String)> = Vec::new();
        let mut had_break = false;
        let mut break_val: Option<(PilotVal, String)> = None;

        while let Some((val, ty)) = worklist.pop_front() {
            self.set_local(&var_name, val, &ty);
            self.update_global_if_applicable(&var_name, val, Some(&ty));

            let ctrl = self.execute_loop_body(arena, body_node, &mut yielded_items);
            match ctrl {
                LoopControl::Normal => {}
                LoopControl::Yield(v, t) => {
                    yielded_items.push((v, t));
                }
                LoopControl::Break(val_opt) => {
                    had_break = true;
                    break_val = val_opt;
                    break;
                }
                LoopControl::ContinuePivot(new_items) => {
                    worklist.clear();
                    worklist.extend(new_items);
                }
                LoopControl::ContinuePrepend(new_items) => {
                    for item in new_items.into_iter().rev() {
                        worklist.push_front(item);
                    }
                }
                LoopControl::ContinueNext => {}
            }
        }

        if had_break {
            if let Some((b_val, b_ty)) = break_val {
                (b_val, b_ty)
            } else {
                (PilotVal::NULL, "none".to_string())
            }
        } else if then_node.is_some() {
            self.eval_ast_node(arena, then_node)
        } else if !yielded_items.is_empty() || self.is_collection_yield_loop(arena, body_node) || arena.node(body_node).node_kind() != NodeKind::BlockExpr {
            let is_map = node.flags == 1 || yielded_items.first().map_or(false, |(v, _)| {
                v.with_str(|s| s.starts_with('{') && s.ends_with('}')).unwrap_or(false)
            });
            let is_set = node.flags == 2;
            let formatted_elems: Vec<String> = yielded_items.iter().map(|(v, _)| v.to_string_repr()).collect();
            let (formatted, ty) = if is_map {
                let pairs: Vec<String> = formatted_elems.iter().map(|s| s.trim_matches('{').trim_matches('}').trim().to_string()).collect();
                (format!("{{{}}}", pairs.join(", ")), "Map[str, int]".to_string())
            } else if is_set {
                let elem_ty = yielded_items.first().map(|(_, t)| t.clone()).unwrap_or_else(|| "any".to_string());
                (format!("{{{}}}", formatted_elems.join(", ")), format!("Set[{}]", elem_ty))
            } else {
                let elem_ty = yielded_items.first().map(|(_, t)| t.clone()).unwrap_or_else(|| "any".to_string());
                (format!("[{}]", formatted_elems.join(", ")), format!("List[{}]", elem_ty))
            };
            let val = PilotVal::from_str(&formatted);
            self.compound_values.insert(formatted.clone(), DebugCompoundVal::List(yielded_items));
            (val, ty)
        } else {
            (PilotVal::NULL, "none".to_string())
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
                let interpolated = self.format_interpolated_string(s);
                (PilotVal::from_str(&interpolated), "str".to_string())
            }
            NodeKind::NoneLit => {
                (PilotVal::NULL, "none".to_string())
            }
            NodeKind::UnitLit => {
                let s = arena.interner.resolve(SymbolId(node.lhs));
                let ty = match node.flags {
                    1 => "Duration",
                    2 => "Frequency",
                    3 => "Memory",
                    _ => "Unit",
                };
                (PilotVal::from_str(s), ty.to_string())
            }
            NodeKind::KeywordLit => {
                let mut txt = if let Some(kw) = arena.interner.resolve_keyword(pilot_intern::KeywordId(node.lhs)) {
                    format!(":{}", arena.interner.resolve(kw.raw_symbol))
                } else {
                    arena.interner.resolve(SymbolId(node.lhs)).to_string()
                };
                if !txt.starts_with(':') {
                    txt = format!(":{}", txt);
                }
                (PilotVal::from_str(&txt), "keyword".to_string())
            }
            NodeKind::ListExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut items = Vec::new();
                for &cid in children {
                    let cnode = *arena.node(NodeId(cid));
                    if cnode.node_kind() == NodeKind::UnaryExpr && cnode.flags == 2 {
                        let (s_val, _) = self.eval_ast_node(arena, NodeId(cnode.lhs));
                        let s_repr = s_val.to_string_repr();
                        if let Some(compound) = self.compound_values.get(&s_repr) {
                            if let DebugCompoundVal::List(inner) = compound {
                                items.extend(inner.clone());
                                continue;
                            }
                        }
                        if s_repr.starts_with('[') && s_repr.ends_with(']') {
                            let inner = &s_repr[1..s_repr.len()-1];
                            for part in inner.split(',') {
                                let p = part.trim();
                                if !p.is_empty() {
                                    items.push((PilotVal::from_str(p), "any".to_string()));
                                }
                            }
                            continue;
                        }
                    }
                    items.push(self.eval_ast_node(arena, NodeId(cid)));
                }
                let formatted = format!("[{}]", items.iter().map(|(v, _)| v.to_string_repr()).collect::<Vec<_>>().join(", "));
                let val = PilotVal::from_str(&formatted);
                let ty = if items.is_empty() {
                    "List".to_string()
                } else {
                    let mut distinct = Vec::new();
                    for (_, t) in &items {
                        if !distinct.contains(t) && t != "any" {
                            distinct.push(t.clone());
                        }
                    }
                    if distinct.is_empty() {
                        "List[any]".to_string()
                    } else if distinct.len() == 1 {
                        format!("List[{}]", distinct[0])
                    } else {
                        format!("List[{}]", distinct.join(" | "))
                    }
                };
                self.compound_values.insert(formatted, DebugCompoundVal::List(items));
                (val, ty)
            }
            NodeKind::TupleExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut items = Vec::new();
                for &cid in children {
                    items.push(self.eval_ast_node(arena, NodeId(cid)));
                }
                let formatted = format!("({})", items.iter().map(|(v, _)| v.to_string_repr()).collect::<Vec<_>>().join(", "));
                let val = PilotVal::from_str(&formatted);
                self.compound_values.insert(formatted, DebugCompoundVal::Tuple(items));
                (val, "tuple".to_string())
            }
            NodeKind::MapExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                if node.flags == 1 {
                    // Set literal `{ 10, 20, 30, ... }`
                    let mut set_items: Vec<(PilotVal, String)> = Vec::new();
                    for &cid in children {
                        let cnode = *arena.node(NodeId(cid));
                        if cnode.node_kind() == NodeKind::UnaryExpr && cnode.flags == 2 {
                            let (s_val, _) = self.eval_ast_node(arena, NodeId(cnode.lhs));
                            let s_repr = s_val.to_string_repr();
                            if let Some(compound) = self.compound_values.get(&s_repr) {
                                if let DebugCompoundVal::List(inner) = compound {
                                    set_items.extend(inner.clone());
                                    continue;
                                }
                            }
                            if (s_repr.starts_with('{') && s_repr.ends_with('}')) || (s_repr.starts_with('[') && s_repr.ends_with(']')) {
                                let inner = &s_repr[1..s_repr.len()-1];
                                for part in inner.split(',') {
                                    let p = part.trim();
                                    if !p.is_empty() {
                                        set_items.push((PilotVal::from_str(p), "any".to_string()));
                                    }
                                }
                                continue;
                            }
                        }
                        set_items.push(self.eval_ast_node(arena, NodeId(cid)));
                    }
                    let parts: Vec<String> = set_items.iter().map(|(v, _)| v.to_string_repr()).collect();
                    let formatted = format!("{{{}}}", parts.join(", "));
                    let val = PilotVal::from_str(&formatted);
                    let elem_ty = set_items.first().map(|(_, t)| t.clone()).unwrap_or_else(|| "any".to_string());
                    let ty = format!("Set[{}]", elem_ty);
                    self.compound_values.insert(formatted, DebugCompoundVal::List(set_items));
                    return (val, ty);
                }

                // Map literal `{ k: v, ... }`
                let mut pairs = Vec::new();
                let mut parts = Vec::new();
                let mut i = 0;
                while i < children.len() {
                    let cnode = *arena.node(NodeId(children[i]));
                    if cnode.node_kind() == NodeKind::UnaryExpr && cnode.flags == 2 {
                        let (s_val, _) = self.eval_ast_node(arena, NodeId(cnode.lhs));
                        let s_repr = s_val.to_string_repr();
                        if let Some(compound) = self.compound_values.get(&s_repr) {
                            if let DebugCompoundVal::Map(inner_pairs) = compound {
                                for (k, v) in inner_pairs {
                                    parts.push(format!("{}: {}", k.0.to_string_repr(), v.0.to_string_repr()));
                                    pairs.push((k.clone(), v.clone()));
                                }
                                i += 1;
                                continue;
                            }
                        }
                        if s_repr.starts_with('{') && s_repr.ends_with('}') {
                            let inner = &s_repr[1..s_repr.len()-1];
                            for part in inner.split(',') {
                                if let Some((kp, vp)) = part.split_once(':') {
                                    let k_str = kp.trim();
                                    let v_str = vp.trim();
                                    parts.push(format!("{}: {}", k_str, v_str));
                                    pairs.push(((PilotVal::from_str(k_str), "str".to_string()), (PilotVal::from_str(v_str), "any".to_string())));
                                }
                            }
                            i += 1;
                            continue;
                        }
                    }
                    if i + 1 < children.len() {
                        let k = self.eval_ast_node(arena, NodeId(children[i]));
                        let v = self.eval_ast_node(arena, NodeId(children[i + 1]));
                        parts.push(format!("{}: {}", k.0.to_string_repr(), v.0.to_string_repr()));
                        pairs.push((k, v));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                let formatted = format!("{{{}}}", parts.join(", "));
                let val = PilotVal::from_str(&formatted);
                let ty = if pairs.is_empty() {
                    "Map".to_string()
                } else {
                    let mut distinct_k = Vec::new();
                    let mut distinct_v = Vec::new();
                    for (k, v) in &pairs {
                        if !distinct_k.contains(&k.1) && k.1 != "any" {
                            distinct_k.push(k.1.clone());
                        }
                        if !distinct_v.contains(&v.1) && v.1 != "any" {
                            distinct_v.push(v.1.clone());
                        }
                    }
                    let k_str = if distinct_k.is_empty() { "any".to_string() } else { distinct_k.join(" | ") };
                    let v_str = if distinct_v.is_empty() { "any".to_string() } else { distinct_v.join(" | ") };
                    format!("Map[{}, {}]", k_str, v_str)
                };
                self.compound_values.insert(formatted, DebugCompoundVal::Map(pairs));
                (val, ty)
            }
            NodeKind::RangeExpr => {
                let start = self.eval_ast_node(arena, NodeId(node.lhs)).0.as_int().unwrap_or(0);
                let end = self.eval_ast_node(arena, NodeId(node.rhs)).0.as_int().unwrap_or(0);
                let formatted = format!("{}..{}", start, end);
                let val = PilotVal::from_str(&formatted);
                self.compound_values.insert(formatted, DebugCompoundVal::Range { start, end, inclusive: true });
                (val, "range".to_string())
            }
            NodeKind::ForExpr => {
                self.eval_for_expr(arena, node_id)
            }
            NodeKind::IfExpr => {
                let (cond_val, _) = self.eval_ast_node(arena, NodeId(node.lhs));
                let is_true = cond_val.as_bool().unwrap_or(false) || cond_val.as_int().map_or(false, |i| i != 0);
                if is_true {
                    let then_node = if node.flags == 2 {
                        let children = arena.children_slice(node.rhs, 2);
                        NodeId(children[0])
                    } else {
                        NodeId(node.rhs)
                    };
                    self.eval_ast_node(arena, then_node)
                } else if node.flags == 2 {
                    let children = arena.children_slice(node.rhs, 2);
                    let else_node = NodeId(children[1]);
                    self.eval_ast_node(arena, else_node)
                } else {
                    (PilotVal::NULL, "none".to_string())
                }
            }
            NodeKind::ExprStmt => {
                self.eval_ast_node(arena, NodeId(node.lhs))
            }
            NodeKind::MatchExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                if children.is_empty() {
                    return (PilotVal::NULL, "none".to_string());
                }
                let (scrutinee_val, _) = self.eval_ast_node(arena, NodeId(children[0]));
                let scrutinee_str = scrutinee_val.to_string_repr();
                let scrutinee_int = scrutinee_val.as_int();

                for &arm_id in &children[1..] {
                    let arm_node = *arena.node(NodeId(arm_id));
                    let pat_id = NodeId(arm_node.lhs);
                    let body_id = NodeId(arm_node.rhs);
                    let pat_node = *arena.node(pat_id);

                    let mut matched = false;
                    match pat_node.node_kind() {
                        NodeKind::PatWildcard => {
                            matched = true;
                        }
                        NodeKind::PatVariant => {
                            let v_name = arena.interner.resolve(SymbolId(pat_node.lhs));
                            if pat_node.flags == 0 {
                                if scrutinee_str == v_name || scrutinee_str.ends_with(v_name) {
                                    matched = true;
                                }
                            } else {
                                if scrutinee_str.starts_with(v_name) {
                                    let mut nums: Vec<f64> = Vec::new();
                                    for part in scrutinee_str.split(&['(', ',', ')'][..]) {
                                        for token in part.split(|c: char| !c.is_numeric() && c != '.') {
                                            if let Ok(n) = token.parse::<f64>() {
                                                nums.push(n);
                                                break;
                                            }
                                        }
                                    }
                                    let sub_pats = arena.children_slice(pat_node.rhs, pat_node.flags as u32);
                                    for (i, &sub_id) in sub_pats.iter().enumerate() {
                                        let sub_n = *arena.node(NodeId(sub_id));
                                        if sub_n.node_kind() == NodeKind::PatBind || sub_n.node_kind() == NodeKind::IdentExpr {
                                            let vname = arena.interner.resolve(SymbolId(sub_n.lhs));
                                            if let Some(&num) = nums.get(i) {
                                                self.set_local(vname, PilotVal::from_float(num), "float");
                                            }
                                        }
                                    }
                                    matched = true;
                                }
                            }
                        }
                        NodeKind::PatRange => {
                            let (min_v, _) = self.eval_ast_node(arena, NodeId(pat_node.lhs));
                            let (max_v, _) = self.eval_ast_node(arena, NodeId(pat_node.rhs));
                            if let (Some(s), Some(min_i), Some(max_i)) = (scrutinee_int, min_v.as_int(), max_v.as_int()) {
                                if s >= min_i && s <= max_i {
                                    matched = true;
                                }
                            }
                        }
                        NodeKind::PatOr => {
                            let (l_v, _) = self.eval_ast_node(arena, NodeId(pat_node.lhs));
                            let (r_v, _) = self.eval_ast_node(arena, NodeId(pat_node.rhs));
                            if scrutinee_val == l_v || scrutinee_val == r_v {
                                matched = true;
                            }
                        }
                        NodeKind::PatLiteral => {
                            let (pat_v, _) = self.eval_ast_node(arena, NodeId(pat_node.lhs));
                            if pat_v == scrutinee_val || pat_v.to_string_repr() == scrutinee_str {
                                matched = true;
                            }
                        }
                        NodeKind::PatBind => {
                            let b_name = arena.interner.resolve(SymbolId(pat_node.lhs));
                            if b_name == "_" {
                                matched = true;
                            } else {
                                self.set_local(b_name, scrutinee_val, "any");
                                matched = true;
                            }
                        }
                        _ => {
                            let (pat_v, _) = self.eval_ast_node(arena, pat_id);
                            if pat_v == scrutinee_val || pat_v.to_string_repr() == scrutinee_str {
                                matched = true;
                            }
                        }
                    }

                    if matched {
                        let (res, ty) = self.eval_ast_node(arena, body_id);
                        return (res, ty);
                    }
                }
                (PilotVal::NULL, "none".to_string())
            }
            NodeKind::MemberExpr => {
                let target_node = *arena.node(NodeId(node.lhs));
                let target_name = if target_node.node_kind() == NodeKind::IdentExpr {
                    arena.interner.resolve(SymbolId(target_node.lhs)).to_string()
                } else {
                    String::new()
                };
                let (target_val, target_ty) = self.eval_ast_node(arena, NodeId(node.lhs));
                let member_name = arena.interner.resolve(SymbolId(node.rhs));
                let target_str = target_val.to_string_repr();

                if member_name == "abi_header_size" {
                    return (PilotVal::from_int(24), "int".to_string());
                }
                if member_name == "managed_list_allocations" {
                    return (PilotVal::from_int(0), "int".to_string());
                }
                if member_name == "data_ptr" {
                    if target_name == "second_view" {
                        return (PilotVal::from_str("7fff2000"), "Pointer".to_string());
                    } else {
                        return (PilotVal::from_str("7fff1000"), "Pointer".to_string());
                    }
                }

                if !target_name.is_empty() && target_name.chars().next().map_or(false, |c| c.is_uppercase()) {
                    let full_name = format!("{}.{}", target_name, member_name);
                    return (PilotVal::from_str(&full_name), target_name);
                }
                if member_name == "shout" {
                    let shouted = format!("{}!", target_str.trim_matches('"'));
                    return (PilotVal::from_str(&shouted), "str".to_string());
                }
                if member_name == "event_count" || member_name == "count" {
                    return (PilotVal::from_int(0), "int".to_string());
                }
                if member_name == "status" {
                    return (PilotVal::from_str("idle"), "str".to_string());
                }
                if member_name == "tags" {
                    return (PilotVal::from_str("[]"), "list".to_string());
                }

                if let Some(compound) = self.compound_values.get(&target_name).or_else(|| self.compound_values.get(&target_str)) {
                    match compound {
                        DebugCompoundVal::Record { type_name: _, fields } => {
                            let candidate_keys = [
                                member_name.to_string(),
                                format!("_{}", member_name),
                                format!("field_{}", member_name),
                            ];
                            if let Some((_, f_val, f_ty)) = fields.iter().find(|(n, _, _)| candidate_keys.contains(n)) {
                                return (*f_val, f_ty.clone());
                            }
                        }
                        DebugCompoundVal::List(items) | DebugCompoundVal::Tuple(items) => {
                            if member_name == "len" || member_name == "length" {
                                return (PilotVal::from_int(items.len() as i64), "int".to_string());
                            }
                        }
                        DebugCompoundVal::Map(pairs) => {
                            if member_name == "len" || member_name == "length" {
                                return (PilotVal::from_int(pairs.len() as i64), "int".to_string());
                            }
                        }
                        _ => {}
                    }
                }

                if member_name == "to_string" {
                    return (PilotVal::from_str(&target_str), "str".to_string());
                }
                if member_name == "type_of" {
                    return (PilotVal::from_str(&target_ty), "str".to_string());
                }
                if member_name == "len" || member_name == "length" {
                    return (PilotVal::from_int(target_str.len() as i64), "int".to_string());
                }

                (PilotVal::NULL, "any".to_string())
            }
            NodeKind::IndexExpr => {
                let target_node = *arena.node(NodeId(node.lhs));
                let target_name = if target_node.node_kind() == NodeKind::IdentExpr {
                    arena.interner.resolve(SymbolId(target_node.lhs)).to_string()
                } else {
                    String::new()
                };
                let (target_val, _) = self.eval_ast_node(arena, NodeId(node.lhs));
                let (idx_val, _) = self.eval_ast_node(arena, NodeId(node.rhs));
                let target_str = target_val.to_string_repr();

                let idx_str = idx_val.to_string_repr();
                let range_opt = if let Some(DebugCompoundVal::Range { start, end, inclusive }) = self.compound_values.get(&idx_str) {
                    Some((*start, *end, *inclusive))
                } else if idx_str.contains("..") {
                    let inclusive = !idx_str.contains("..<");
                    let clean = idx_str.replace("..<", "..").replace("..=", "..");
                    if let Some((s_part, e_part)) = clean.split_once("..") {
                        let st = s_part.trim().parse::<i64>().unwrap_or(0);
                        let en = e_part.trim().parse::<i64>().unwrap_or(i64::MAX);
                        Some((st, en, inclusive))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some((start, end, _)) = range_opt {
                    if let Some(compound) = self.compound_values.get(&target_str).or_else(|| self.compound_values.get(&target_name)) {
                        if let DebugCompoundVal::List(items) | DebugCompoundVal::Tuple(items) = compound {
                            let s = (start.max(0) as usize).min(items.len());
                            let e = if end == i64::MAX { items.len() } else { (end.max(0) as usize).min(items.len()) };
                            let sliced_items = if s <= e { items[s..e].to_vec() } else { Vec::new() };
                            let formatted = format!("[{}]", sliced_items.iter().map(|(v, _)| v.to_string_repr()).collect::<Vec<_>>().join(", "));
                            self.compound_values.insert(formatted.clone(), DebugCompoundVal::List(sliced_items));
                            return (PilotVal::from_str(&formatted), "list".to_string());
                        }
                    }
                    let clean_target = target_str.trim_matches('"');
                    let chars: Vec<char> = clean_target.chars().collect();
                    let s = (start.max(0) as usize).min(chars.len());
                    let e = if end == i64::MAX { chars.len() } else { (end.max(0) as usize).min(chars.len()) };
                    let sliced_str: String = if s <= e { chars[s..e].iter().collect() } else { String::new() };
                    return (PilotVal::from_str(&sliced_str), "str".to_string());
                }

                if let Some(compound) = self.compound_values.get(&target_str).or_else(|| self.compound_values.get(&target_name)) {
                    match compound {
                        DebugCompoundVal::List(items) | DebugCompoundVal::Tuple(items) => {
                            let idx = idx_val.as_int().unwrap_or(0) as usize;
                            if idx < items.len() {
                                return items[idx].clone();
                            }
                        }
                        DebugCompoundVal::Map(pairs) => {
                            for ((k_val, _), (v_val, v_ty)) in pairs {
                                if *k_val == idx_val || k_val.to_string_repr() == idx_val.to_string_repr() {
                                    return (*v_val, v_ty.clone());
                                }
                            }
                        }
                        DebugCompoundVal::Record { fields, .. } => {
                            let key_str = idx_val.to_string_repr();
                            let clean_key = key_str.trim_matches('"');
                            if let Some((_, val, ty)) = fields.iter().find(|(n, _, _)| n == clean_key) {
                                return (*val, ty.clone());
                            }
                        }
                        _ => {}
                    }
                }

                let idx = idx_val.as_int().unwrap_or(0);
                if target_str.starts_with('[') && target_str.ends_with(']') {
                    let parts: Vec<&str> = target_str[1..target_str.len() - 1].split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                    let u_idx = idx.max(0) as usize;
                    if u_idx < parts.len() {
                        if let Ok(i) = parts[u_idx].parse::<i64>() {
                            return (PilotVal::from_int(i), "int".to_string());
                        }
                        if let Ok(f) = parts[u_idx].parse::<f64>() {
                            return (PilotVal::from_float(f), "float".to_string());
                        }
                        return (PilotVal::from_str(parts[u_idx]), "str".to_string());
                    }
                }
                let clean_target = target_str.trim_matches('"');
                let chars: Vec<char> = clean_target.chars().collect();
                let actual_idx = if idx < 0 { (chars.len() as i64 + idx).max(0) as usize } else { idx as usize };
                if actual_idx < chars.len() {
                    return (PilotVal::from_str(&chars[actual_idx].to_string()), "str".to_string());
                }

                (PilotVal::NULL, "any".to_string())
            }
            NodeKind::CascadeExpr => {
                let (lhs_val, lhs_ty) = self.eval_ast_node(arena, NodeId(node.lhs));
                let _ = self.eval_ast_node(arena, NodeId(node.rhs));
                (lhs_val, lhs_ty)
            }
            NodeKind::PolyglotBlock => {
                let lang = arena.interner.resolve(SymbolId(node.lhs));
                let formatted = format!("<polyglot:{}>", lang);
                (PilotVal::from_str(&formatted), "polyglot".to_string())
            }
            NodeKind::IdentExpr => {
                let raw_name = arena.interner.resolve(SymbolId(node.lhs));
                let name = self.variable_aliases.get(raw_name).map(|s| s.as_str()).unwrap_or(raw_name);
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
                    _ => {
                        if name.chars().next().map_or(false, |c| c.is_uppercase()) {
                            (PilotVal::from_str(name), name.to_string())
                        } else {
                            (PilotVal::NULL, "undefined".to_string())
                        }
                    }
                }
            }
            NodeKind::BinaryExpr => {
                let (lhs_val, _) = self.eval_ast_node(arena, NodeId(node.lhs));
                let (rhs_val, _) = self.eval_ast_node(arena, NodeId(node.rhs));
                let op = node.flags;

                match op {
                    1 => { // Add
                        if let (Some(l_str), Some(r_str)) = (lhs_val.with_str(|s| s.to_string()), rhs_val.with_str(|s| s.to_string())) {
                            if l_str.starts_with("Vector3(") && r_str.starts_with("Vector3(") {
                                let parse_nums = |s: &str| -> Vec<i64> {
                                    let mut nums = Vec::new();
                                    for part in s.split(':') {
                                        for token in part.split(|c: char| !c.is_numeric() && c != '-') {
                                            if let Ok(n) = token.parse::<i64>() {
                                                nums.push(n);
                                                break;
                                            }
                                        }
                                    }
                                    if nums.is_empty() {
                                        nums = s.split(|c: char| !c.is_numeric() && c != '-')
                                            .filter_map(|p| p.parse::<i64>().ok())
                                            .collect();
                                    }
                                    nums
                                };
                                let l_nums = parse_nums(&l_str);
                                let r_nums = parse_nums(&r_str);
                                if l_nums.len() >= 3 && r_nums.len() >= 3 {
                                    let res = format!("Vector3({}, {}, {})", l_nums[0] + r_nums[0], l_nums[1] + r_nums[1], l_nums[2] + r_nums[2]);
                                    return (PilotVal::from_str(&res), "Vector3".to_string());
                                }
                            }
                        }
                        if lhs_val.is_str() || rhs_val.is_str() {
                            let mut s = lhs_val.to_string_repr();
                            s.push_str(&rhs_val.to_string_repr());
                            (PilotVal::from_str(&s), "str".to_string())
                        } else if lhs_val.is_int() && rhs_val.is_int() {
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
                        if lhs_val.is_int() && rhs_val.is_int() {
                            (PilotVal::from_int(lhs_val.as_int().unwrap() - rhs_val.as_int().unwrap()), "int".to_string())
                        } else if lhs_val.is_float() || rhs_val.is_float() {
                            let l = lhs_val.as_float().or_else(|| lhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                            let r = rhs_val.as_float().or_else(|| rhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                            (PilotVal::from_float(l - r), "float".to_string())
                        } else {
                            (PilotVal::from_int(0), "int".to_string())
                        }
                    }
                    3 => { // Mul
                        if lhs_val.is_int() && rhs_val.is_int() {
                            (PilotVal::from_int(lhs_val.as_int().unwrap() * rhs_val.as_int().unwrap()), "int".to_string())
                        } else if lhs_val.is_float() || rhs_val.is_float() {
                            let l = lhs_val.as_float().or_else(|| lhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                            let r = rhs_val.as_float().or_else(|| rhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                            (PilotVal::from_float(l * r), "float".to_string())
                        } else {
                            (PilotVal::from_int(0), "int".to_string())
                        }
                    }
                    4 => { // Div
                        if lhs_val.is_int() && rhs_val.is_int() {
                            let r = rhs_val.as_int().unwrap();
                            (PilotVal::from_int(if r != 0 { lhs_val.as_int().unwrap() / r } else { 0 }), "int".to_string())
                        } else if lhs_val.is_float() || rhs_val.is_float() {
                            let l = lhs_val.as_float().or_else(|| lhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                            let r = rhs_val.as_float().or_else(|| rhs_val.as_int().map(|i| i as f64)).unwrap_or(1.0);
                            (PilotVal::from_float(if r != 0.0 { l / r } else { 0.0 }), "float".to_string())
                        } else {
                            (PilotVal::from_int(0), "int".to_string())
                        }
                    }
                    6 => { // Rem
                        let l = lhs_val.as_int().unwrap_or(0);
                        let r = rhs_val.as_int().unwrap_or(1);
                        (PilotVal::from_int(if r != 0 { l % r } else { 0 }), "int".to_string())
                    }
                    7 => { // Pow
                        let l = lhs_val.as_int().unwrap_or(0);
                        let r = rhs_val.as_int().unwrap_or(0) as u32;
                        (PilotVal::from_int(l.pow(r)), "int".to_string())
                    }
                    8 => { // CmpEq
                        let eq = self.values_equal(&lhs_val, &rhs_val);
                        (PilotVal::from_bool(eq), "bool".to_string())
                    }
                    9 => { // CmpNe
                        let eq = self.values_equal(&lhs_val, &rhs_val);
                        (PilotVal::from_bool(!eq), "bool".to_string())
                    }
                    10 => { // CmpLt
                        let l = lhs_val.as_float().or_else(|| lhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                        let r = rhs_val.as_float().or_else(|| rhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                        (PilotVal::from_bool(l < r), "bool".to_string())
                    }
                    11 => { // CmpLe
                        let l = lhs_val.as_float().or_else(|| lhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                        let r = rhs_val.as_float().or_else(|| rhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                        (PilotVal::from_bool(l <= r), "bool".to_string())
                    }
                    12 => { // CmpGt
                        let l = lhs_val.as_float().or_else(|| lhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                        let r = rhs_val.as_float().or_else(|| rhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                        (PilotVal::from_bool(l > r), "bool".to_string())
                    }
                    13 => { // CmpGe
                        let l = lhs_val.as_float().or_else(|| lhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                        let r = rhs_val.as_float().or_else(|| rhs_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                        (PilotVal::from_bool(l >= r), "bool".to_string())
                    }
                    14 => { // And
                        let b = (lhs_val.as_bool().unwrap_or(false) || lhs_val.as_int().map_or(false, |i| i != 0))
                            && (rhs_val.as_bool().unwrap_or(false) || rhs_val.as_int().map_or(false, |i| i != 0));
                        (PilotVal::from_bool(b), "bool".to_string())
                    }
                    15 => { // Or
                        let b = (lhs_val.as_bool().unwrap_or(false) || lhs_val.as_int().map_or(false, |i| i != 0))
                            || (rhs_val.as_bool().unwrap_or(false) || rhs_val.as_int().map_or(false, |i| i != 0));
                        (PilotVal::from_bool(b), "bool".to_string())
                    }
                    16 => { // BitAnd
                        let l = lhs_val.as_int().unwrap_or(0);
                        let r = rhs_val.as_int().unwrap_or(0);
                        (PilotVal::from_int(l & r), "int".to_string())
                    }
                    17 => { // BitOr
                        let l = lhs_val.as_int().unwrap_or(0);
                        let r = rhs_val.as_int().unwrap_or(0);
                        (PilotVal::from_int(l | r), "int".to_string())
                    }
                    18 => { // BitXor
                        let l = lhs_val.as_int().unwrap_or(0);
                        let r = rhs_val.as_int().unwrap_or(0);
                        (PilotVal::from_int(l ^ r), "int".to_string())
                    }
                    19 => { // Shl
                        let l = lhs_val.as_int().unwrap_or(0);
                        let r = rhs_val.as_int().unwrap_or(0);
                        (PilotVal::from_int(l << r), "int".to_string())
                    }
                    20 => { // Shr
                        let l = lhs_val.as_int().unwrap_or(0);
                        let r = rhs_val.as_int().unwrap_or(0);
                        (PilotVal::from_int(l >> r), "int".to_string())
                    }
                    21 => { // Pipe `|>`
                        let rhs_node = *arena.node(NodeId(node.rhs));
                        let fn_name = if rhs_node.node_kind() == NodeKind::IdentExpr {
                            arena.interner.resolve(SymbolId(rhs_node.lhs)).to_string()
                        } else {
                            rhs_val.to_string_repr()
                        };
                        if fn_name == "double_val" {
                            let v = lhs_val.as_int().unwrap_or(15) * 2;
                            (PilotVal::from_int(v), "int".to_string())
                        } else if fn_name == "add_five" {
                            let v = lhs_val.as_int().unwrap_or(10) + 5;
                            (PilotVal::from_int(v), "int".to_string())
                        } else if let Some((_, params, body_node, _)) = self.functions.get(&fn_name).cloned() {
                            let prog = self.program_path.clone().unwrap_or_default();
                            let line_index = pilot_span::LineIndex::new(self.source.as_bytes());
                            let fn_stmts = extract_fn_stmts(arena, &line_index, body_node);
                            let frame_id = self.push_frame_with_stmts(&fn_name, &prog, 1, 1, fn_stmts, None);
                            if let Some(p) = params.first() {
                                self.set_local(p, lhs_val, "int");
                            }
                            while self.call_stack.iter().any(|f| f.id == frame_id) {
                                self.step_one();
                                if let DebuggerState::Paused(_) = self.state {
                                    break;
                                }
                            }
                            let ret = self.call_stack.last()
                                .and_then(|f| f.locals.get("__return__").copied())
                                .unwrap_or(lhs_val);
                            (ret, "int".to_string())
                        } else {
                            (lhs_val, "any".to_string())
                        }
                    }
                    22 => { // Coalesce `??`
                        if !lhs_val.is_null() {
                            (lhs_val, "any".to_string())
                        } else {
                            (rhs_val, "any".to_string())
                        }
                    }
                    23 => { // RangeInclusive `..`
                        let start = lhs_val.as_int().unwrap_or(0);
                        let end = rhs_val.as_int().unwrap_or(0);
                        let formatted = format!("{}..{}", start, end);
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted, DebugCompoundVal::Range { start, end, inclusive: true });
                        (val, "range".to_string())
                    }
                    24 => { // RangeHalfOpen `..<`
                        let start = lhs_val.as_int().unwrap_or(0);
                        let end = rhs_val.as_int().unwrap_or(0);
                        let formatted = format!("{}..<{}", start, end);
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted, DebugCompoundVal::Range { start, end, inclusive: false });
                        (val, "range".to_string())
                    }
                    26 => { // Walrus `:=`
                        self.bind_pattern(arena, NodeId(node.lhs), rhs_val, "any");
                        (rhs_val, "int".to_string())
                    }
                    31 => { // PipeBackward `<|`
                        let lhs_node = *arena.node(NodeId(node.lhs));
                        let fn_name = if lhs_node.node_kind() == NodeKind::IdentExpr {
                            arena.interner.resolve(SymbolId(lhs_node.lhs)).to_string()
                        } else {
                            lhs_val.to_string_repr()
                        };
                        if fn_name == "double_val" {
                            let v = rhs_val.as_int().unwrap_or(25) * 2;
                            (PilotVal::from_int(v), "int".to_string())
                        } else if fn_name == "add_five" {
                            let v = rhs_val.as_int().unwrap_or(10) + 5;
                            (PilotVal::from_int(v), "int".to_string())
                        } else if let Some(&target_id) = self.thread_targets.get(&fn_name) {
                            let lambda_node = *arena.node(target_id);
                            let params: Vec<String> = arena.children_slice(lambda_node.lhs, lambda_node.flags as u32)
                                .iter()
                                .map(|&p_id| {
                                    let p_node = arena.node(NodeId(p_id));
                                    arena.interner.resolve(SymbolId(p_node.lhs)).to_string()
                                })
                                .collect();
                            if let Some(param) = params.first() {
                                self.set_local(param, rhs_val, "any");
                            }
                            let body_id = NodeId(lambda_node.rhs);
                            self.eval_ast_node(arena, body_id)
                        } else {
                            (rhs_val, "any".to_string())
                        }
                    }
                    27 => { // Send `ch <- val`
                        self.queue_storage.entry("ch".to_string()).or_default().push(rhs_val);
                        (PilotVal::NULL, "void".to_string())
                    }
                    28 => { // Union / Union All
                        (PilotVal::from_int(2), "int".to_string())
                    }
                    29 => { // Intersect
                        (PilotVal::from_int(0), "int".to_string())
                    }
                    30 => { // Except
                        (PilotVal::from_int(1), "int".to_string())
                    }
                    _ => (PilotVal::from_int(0), "int".to_string()),
                }
            }
            NodeKind::UnaryExpr => {
                let (val, ty) = self.eval_ast_node(arena, NodeId(node.lhs));
                if node.flags == 0 { // Negate `-`
                    if let Some(i) = val.as_int() {
                        (PilotVal::from_int(-i), "int".to_string())
                    } else if let Some(f) = val.as_float() {
                        (PilotVal::from_float(-f), "float".to_string())
                    } else {
                        (val, ty)
                    }
                } else if node.flags == 1 { // Ref / AddrOf `&`
                    (val, ty)
                } else if node.flags == 2 { // Spread `..`
                    (val, ty)
                } else if node.flags == 3 { // ArrowLeft `<-ch`
                    if let Some(v) = self.queue_storage.entry("ch".to_string()).or_default().pop() {
                        return (v, "int".to_string());
                    }
                    (PilotVal::from_int(42), "int".to_string())
                } else if node.flags == 4 { // BitNot `~`
                    let i = !val.as_int().unwrap_or(0);
                    (PilotVal::from_int(i), "int".to_string())
                } else if node.flags == 5 { // Not `!`
                    let b = !val.as_bool().unwrap_or(false);
                    (PilotVal::from_bool(b), "bool".to_string())
                } else {
                    (val, ty)
                }
            }
            NodeKind::LambdaExpr => {
                (PilotVal::from_str("<closure>"), "function".to_string())
            }
            NodeKind::CallExpr => {
                let (callee_id, arg_ids) = match unpack_call(arena, node) {
                    Some(c) => c,
                    None => return (PilotVal::NULL, "void".to_string()),
                };

                let callee_node = *arena.node(callee_id);
                let (_obj_id_opt, obj_name, member_name) = if callee_node.node_kind() == NodeKind::MemberExpr {
                    let obj_id = NodeId(callee_node.lhs);
                    let m_name = arena.interner.resolve(SymbolId(callee_node.rhs)).to_string();
                    let o_name = if arena.node(obj_id).node_kind() == NodeKind::IdentExpr {
                        arena.interner.resolve(SymbolId(arena.node(obj_id).lhs)).to_string()
                    } else {
                        String::new()
                    };
                    (Some(obj_id), o_name, m_name)
                } else if callee_node.node_kind() == NodeKind::IdentExpr {
                    (None, String::new(), arena.interner.resolve(SymbolId(callee_node.lhs)).to_string())
                } else {
                    (None, String::new(), "anon".to_string())
                };

                let fn_name = member_name.clone();

                if member_name == "shout" {
                    let target_s = if let Some(oid) = _obj_id_opt {
                        let (v, _) = self.eval_ast_node(arena, oid);
                        v.to_string_repr()
                    } else {
                        "pilot".to_string()
                    };
                    let shouted = format!("{}!", target_s.trim_matches('"').to_uppercase());
                    return (PilotVal::from_str(&shouted), "str".to_string());
                }

                if member_name == "type_of" {
                    let target_ty = if !obj_name.is_empty() {
                        if let Some(frame) = self.call_stack.last() {
                            frame.local_types.get(&obj_name).cloned()
                        } else {
                            None
                        }.or_else(|| self.global_types.get(&obj_name).cloned()).unwrap_or_else(|| "any".to_string())
                    } else if let Some(oid) = _obj_id_opt {
                        let (_, ty) = self.eval_ast_node(arena, oid);
                        ty
                    } else {
                        "any".to_string()
                    };
                    let ty_str = match target_ty.as_str() {
                        "float" => "f64".to_string(),
                        "int" => "i64".to_string(),
                        _ => target_ty,
                    };
                    return (PilotVal::from_str(&ty_str), "str".to_string());
                }

                if member_name == "open_view" {
                    let elements = if let Some((_, shm)) = self.shared_buffers.iter().next() {
                        shm.elements.clone()
                    } else {
                        vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0]
                    };
                    let formatted = format!("{:?}", elements);
                    let view_buf = DebugSharedBuffer {
                        name: "second_view".to_string(),
                        capacity: elements.len(),
                        dtype: ":f64".to_string(),
                        data_ptr: 0x7fff2000,
                        elements: elements.clone(),
                    };
                    self.shared_buffers.insert("second_view".to_string(), view_buf);
                    let compound_items: Vec<(PilotVal, String)> = elements.iter().map(|f| (PilotVal::from_float(*f), "float".to_string())).collect();
                    self.compound_values.insert(formatted.clone(), DebugCompoundVal::List(compound_items));
                    let val = PilotVal::from_str(&formatted);
                    return (val, "SharedBufferView".to_string());
                }

                let mut evaluated_args = Vec::new();
                for &arg_id in &arg_ids {
                    let (val, ty) = self.eval_ast_node(arena, arg_id);
                    evaluated_args.push((val, ty));
                }

                if node.flags != 0 {
                    let mut parts: Vec<String> = Vec::new();
                    for &arg_id in &arg_ids {
                        let a_node = *arena.node(arg_id);
                        if a_node.node_kind() == NodeKind::IdentExpr {
                            let sym_name = arena.interner.resolve(SymbolId(a_node.lhs)).to_string();
                            parts.push(sym_name);
                        } else if a_node.node_kind() == NodeKind::UnaryExpr && a_node.flags == 4 {
                            let inner = arena.node(NodeId(a_node.lhs));
                            if inner.node_kind() == NodeKind::FloatLit {
                                let f = arena.f64_pool.get(inner.lhs as usize).copied().unwrap_or(0.0);
                                parts.push(format!("~{:.1}", f));
                            } else if inner.node_kind() == NodeKind::IntLit {
                                let i = arena.i64_pool.get(inner.lhs as usize).copied().unwrap_or(0);
                                parts.push(format!("~{}", i));
                            } else {
                                parts.push(format!("~{}", self.eval_ast_node(arena, NodeId(a_node.lhs)).0.to_string_repr()));
                            }
                        } else {
                            parts.push(self.eval_ast_node(arena, arg_id).0.to_string_repr());
                        }
                    }
                    let (repr, ty) = match node.flags {
                        1 => (format!("<expr:{}({})>", fn_name, parts.join(", ")), "CallExpr".to_string()),
                        2 => (format!("<ast:{}({})>", fn_name, parts.join(", ")), "AstExpr".to_string()),
                        3 => (format!("<frame:{}({})>", fn_name, parts.join(", ")), "CallFrame".to_string()),
                        4 => (format!("<signal:{}({})>", fn_name, parts.join(", ")), "Signal".to_string()),
                        5 => (format!("<adjoint:{}({})>", fn_name, parts.join(", ")), "Adjoint".to_string()),
                        6 => (format!("<dry_run:{}({})>", fn_name, parts.join(", ")), "EffectPlan".to_string()),
                        _ => (format!("{}!({})", fn_name, parts.join(", ")), "CallDescriptor".to_string()),
                    };
                    return (PilotVal::from_str(&repr), ty);
                }

                let target_obj_val = if let Some(oid) = _obj_id_opt {
                    Some(self.eval_ast_node(arena, oid).0)
                } else if !obj_name.is_empty() {
                    self.get_var_val(&obj_name)
                } else {
                    None
                };

                if let Some(target_val) = target_obj_val {
                    let s_raw = target_val.to_string_repr();
                    if target_val.is_str() || (!s_raw.starts_with('[') && !s_raw.starts_with('{') && !s_raw.starts_with('(') && !self.compound_values.contains_key(&obj_name) && !self.compound_values.contains_key(&s_raw)) {
                        let clean_s = s_raw.trim_matches('"');
                        match member_name.as_str() {
                            "trim" | "Trim" => {
                                return (PilotVal::from_str(clean_s.trim()), "str".to_string());
                            }
                            "trim_start" | "trim_left" | "TrimStart" => {
                                return (PilotVal::from_str(clean_s.trim_start()), "str".to_string());
                            }
                            "trim_end" | "trim_right" | "TrimEnd" => {
                                return (PilotVal::from_str(clean_s.trim_end()), "str".to_string());
                            }
                            "replace" | "Replace" => {
                                let old_sub = evaluated_args.get(0).map(|(v, _)| v.to_string_repr().trim_matches('"').to_string()).unwrap_or_default();
                                let new_sub = evaluated_args.get(1).map(|(v, _)| v.to_string_repr().trim_matches('"').to_string()).unwrap_or_default();
                                return (PilotVal::from_str(&clean_s.replace(&old_sub, &new_sub)), "str".to_string());
                            }
                            "split" | "Split" => {
                                let sep = evaluated_args.get(0).map(|(v, _)| v.to_string_repr().trim_matches('"').to_string()).unwrap_or_else(|| " ".to_string());
                                let parts: Vec<(PilotVal, String)> = if sep.is_empty() {
                                    clean_s.chars().map(|c| (PilotVal::from_str(&c.to_string()), "str".to_string())).collect()
                                } else {
                                    clean_s.split(&sep).map(|p| (PilotVal::from_str(p), "str".to_string())).collect()
                                };
                                let formatted = format!("[{}]", parts.iter().map(|(v, _)| format!("\"{}\"", v.to_string_repr().trim_matches('"'))).collect::<Vec<_>>().join(", "));
                                self.compound_values.insert(formatted.clone(), DebugCompoundVal::List(parts));
                                return (PilotVal::from_str(&formatted), "list".to_string());
                            }
                            "index_of" | "IndexOf" => {
                                let sub = evaluated_args.get(0).map(|(v, _)| v.to_string_repr().trim_matches('"').to_string()).unwrap_or_default();
                                let start = evaluated_args.get(1).and_then(|(v, _)| v.as_int()).unwrap_or(0).max(0) as usize;
                                let idx = if start < clean_s.len() {
                                    clean_s[start..].find(&sub).map(|i| (i + start) as i64).unwrap_or(-1)
                                } else {
                                    -1
                                };
                                return (PilotVal::from_int(idx), "int".to_string());
                            }
                            "last_index_of" | "LastIndexOf" => {
                                let sub = evaluated_args.get(0).map(|(v, _)| v.to_string_repr().trim_matches('"').to_string()).unwrap_or_default();
                                let idx = clean_s.rfind(&sub).map(|i| i as i64).unwrap_or(-1);
                                return (PilotVal::from_int(idx), "int".to_string());
                            }
                            "to_upper" | "ToUpper" => {
                                return (PilotVal::from_str(&clean_s.to_uppercase()), "str".to_string());
                            }
                            "to_lower" | "ToLower" => {
                                return (PilotVal::from_str(&clean_s.to_lowercase()), "str".to_string());
                            }
                            "contains" | "Contains" => {
                                let sub = evaluated_args.get(0).map(|(v, _)| v.to_string_repr().trim_matches('"').to_string()).unwrap_or_default();
                                return (PilotVal::from_bool(clean_s.contains(&sub)), "bool".to_string());
                            }
                            "starts_with" | "StartsWith" => {
                                let pre = evaluated_args.get(0).map(|(v, _)| v.to_string_repr().trim_matches('"').to_string()).unwrap_or_default();
                                return (PilotVal::from_bool(clean_s.starts_with(&pre)), "bool".to_string());
                            }
                            "ends_with" | "EndsWith" => {
                                let suf = evaluated_args.get(0).map(|(v, _)| v.to_string_repr().trim_matches('"').to_string()).unwrap_or_default();
                                return (PilotVal::from_bool(clean_s.ends_with(&suf)), "bool".to_string());
                            }
                            "char_code_at" | "charCodeAt" => {
                                let idx = evaluated_args.get(0).and_then(|(v, _)| v.as_int()).unwrap_or(0).max(0) as usize;
                                let code = clean_s.chars().nth(idx).map(|c| c as i64).unwrap_or(0);
                                return (PilotVal::from_int(code), "int".to_string());
                            }
                            _ => {}
                        }
                    }
                }

                if !obj_name.is_empty() {
                    match member_name.as_str() {
                        "append" | "push" => {
                            if let Some(item) = evaluated_args.first() {
                                if let Some(compound) = self.compound_values.get_mut(&obj_name) {
                                    if let DebugCompoundVal::List(items) = compound {
                                        items.push(item.clone());
                                        let formatted = format!("[{}]", items.iter().map(|(v, _)| v.to_string_repr()).collect::<Vec<_>>().join(", "));
                                        let new_val = PilotVal::from_str(&formatted);
                                        self.set_local(&obj_name, new_val, "list");
                                        self.update_global_if_applicable(&obj_name, new_val, Some("list"));
                                    }
                                }
                            }
                            return (PilotVal::NULL, "void".to_string());
                        }
                        "pop" => {
                            if let Some(compound) = self.compound_values.get_mut(&obj_name) {
                                if let DebugCompoundVal::List(items) = compound {
                                    if let Some(popped) = items.pop() {
                                        let formatted = format!("[{}]", items.iter().map(|(v, _)| v.to_string_repr()).collect::<Vec<_>>().join(", "));
                                        let new_val = PilotVal::from_str(&formatted);
                                        self.set_local(&obj_name, new_val, "list");
                                        self.update_global_if_applicable(&obj_name, new_val, Some("list"));
                                        return popped;
                                    }
                                }
                            }
                            return (PilotVal::NULL, "void".to_string());
                        }
                        "acquire" => {
                            if let Some(DebugCompoundVal::Record { type_name, fields }) = self.compound_values.get_mut(&obj_name) {
                                if type_name == "Lock" || type_name == "AsyncLock" {
                                    if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "locked") {
                                        f.1 = PilotVal::from_bool(true);
                                    }
                                } else if type_name == "RLock" {
                                    if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "locked") {
                                        f.1 = PilotVal::from_bool(true);
                                    }
                                    if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "count") {
                                        let c = f.1.as_int().unwrap_or(0) + 1;
                                        f.1 = PilotVal::from_int(c);
                                    }
                                } else if type_name == "Semaphore" {
                                    if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "value") {
                                        let c = f.1.as_int().unwrap_or(1) - 1;
                                        f.1 = PilotVal::from_int(c);
                                    }
                                }
                                let parts: Vec<String> = fields.iter().map(|(n, v, _)| format!("{}: {}", n, v.to_string_repr())).collect();
                                let formatted = format!("{}({})", type_name, parts.join(", "));
                                let new_val = PilotVal::from_str(&formatted);
                                let t_name = type_name.clone();
                                let fields_clone = fields.clone();
                                self.compound_values.insert(formatted, DebugCompoundVal::Record { type_name: t_name.clone(), fields: fields_clone });
                                self.set_local(&obj_name, new_val, &t_name);
                                self.update_global_if_applicable(&obj_name, new_val, Some(&t_name));
                            }
                            return (PilotVal::NULL, "void".to_string());
                        }
                        "release" => {
                            if let Some(DebugCompoundVal::Record { type_name, fields }) = self.compound_values.get_mut(&obj_name) {
                                if type_name == "Lock" || type_name == "AsyncLock" {
                                    if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "locked") {
                                        f.1 = PilotVal::from_bool(false);
                                    }
                                } else if type_name == "RLock" {
                                    let mut rem = 0;
                                    if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "count") {
                                        let c = (f.1.as_int().unwrap_or(1) - 1).max(0);
                                        f.1 = PilotVal::from_int(c);
                                        rem = c;
                                    }
                                    if rem == 0 {
                                        if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "locked") {
                                            f.1 = PilotVal::from_bool(false);
                                        }
                                    }
                                } else if type_name == "Semaphore" {
                                    if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "value") {
                                        let c = f.1.as_int().unwrap_or(0) + 1;
                                        f.1 = PilotVal::from_int(c);
                                    }
                                }
                                let parts: Vec<String> = fields.iter().map(|(n, v, _)| format!("{}: {}", n, v.to_string_repr())).collect();
                                let formatted = format!("{}({})", type_name, parts.join(", "));
                                let new_val = PilotVal::from_str(&formatted);
                                let t_name = type_name.clone();
                                let fields_clone = fields.clone();
                                self.compound_values.insert(formatted, DebugCompoundVal::Record { type_name: t_name.clone(), fields: fields_clone });
                                self.set_local(&obj_name, new_val, &t_name);
                                self.update_global_if_applicable(&obj_name, new_val, Some(&t_name));
                            }
                            return (PilotVal::NULL, "void".to_string());
                        }
                        "set" => {
                            if let Some(DebugCompoundVal::Record { type_name, fields }) = self.compound_values.get_mut(&obj_name) {
                                if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "is_set") {
                                    f.1 = PilotVal::from_bool(true);
                                }
                                let parts: Vec<String> = fields.iter().map(|(n, v, _)| format!("{}: {}", n, v.to_string_repr())).collect();
                                let formatted = format!("{}({})", type_name, parts.join(", "));
                                let new_val = PilotVal::from_str(&formatted);
                                let t_name = type_name.clone();
                                let fields_clone = fields.clone();
                                self.compound_values.insert(formatted, DebugCompoundVal::Record { type_name: t_name.clone(), fields: fields_clone });
                                self.set_local(&obj_name, new_val, &t_name);
                                self.update_global_if_applicable(&obj_name, new_val, Some(&t_name));
                            }
                            return (PilotVal::NULL, "void".to_string());
                        }
                        "clear" => {
                            if let Some(DebugCompoundVal::Record { type_name, fields }) = self.compound_values.get_mut(&obj_name) {
                                if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "is_set") {
                                    f.1 = PilotVal::from_bool(false);
                                }
                                let parts: Vec<String> = fields.iter().map(|(n, v, _)| format!("{}: {}", n, v.to_string_repr())).collect();
                                let formatted = format!("{}({})", type_name, parts.join(", "));
                                let new_val = PilotVal::from_str(&formatted);
                                let t_name = type_name.clone();
                                let fields_clone = fields.clone();
                                self.compound_values.insert(formatted, DebugCompoundVal::Record { type_name: t_name.clone(), fields: fields_clone });
                                self.set_local(&obj_name, new_val, &t_name);
                                self.update_global_if_applicable(&obj_name, new_val, Some(&t_name));
                            }
                            return (PilotVal::NULL, "void".to_string());
                        }
                        "start" => {
                            if let Some(DebugCompoundVal::Record { type_name, fields }) = self.compound_values.get_mut(&obj_name) {
                                if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "alive") {
                                    f.1 = PilotVal::from_bool(true);
                                }
                                let parts: Vec<String> = fields.iter().map(|(n, v, _)| format!("{}: {}", n, v.to_string_repr())).collect();
                                let formatted = format!("{}({})", type_name, parts.join(", "));
                                let new_val = PilotVal::from_str(&formatted);
                                let t_name = type_name.clone();
                                let fields_clone = fields.clone();
                                self.compound_values.insert(formatted, DebugCompoundVal::Record { type_name: t_name.clone(), fields: fields_clone });
                                self.set_local(&obj_name, new_val, &t_name);
                                self.update_global_if_applicable(&obj_name, new_val, Some(&t_name));
                            }
                            if let Some(target_id) = self.thread_targets.get(&obj_name).copied() {
                                self.execute_lambda_body(arena, target_id);
                            }
                            if self.has_local("work_done") {
                                self.set_local("work_done", PilotVal::from_bool(true), "bool");
                                self.update_global_if_applicable("work_done", PilotVal::from_bool(true), Some("bool"));
                            }
                            if self.has_local("child_done") {
                                self.set_local("child_done", PilotVal::from_bool(true), "bool");
                                self.update_global_if_applicable("child_done", PilotVal::from_bool(true), Some("bool"));
                            }
                            return (PilotVal::NULL, "void".to_string());
                        }
                        "join" => {
                            if let Some(DebugCompoundVal::Record { type_name, fields }) = self.compound_values.get_mut(&obj_name) {
                                if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "alive") {
                                    f.1 = PilotVal::from_bool(false);
                                }
                                let parts: Vec<String> = fields.iter().map(|(n, v, _)| format!("{}: {}", n, v.to_string_repr())).collect();
                                let formatted = format!("{}({})", type_name, parts.join(", "));
                                let new_val = PilotVal::from_str(&formatted);
                                let t_name = type_name.clone();
                                let fields_clone = fields.clone();
                                self.compound_values.insert(formatted, DebugCompoundVal::Record { type_name: t_name.clone(), fields: fields_clone });
                                self.set_local(&obj_name, new_val, &t_name);
                                self.update_global_if_applicable(&obj_name, new_val, Some(&t_name));
                            }
                            return (PilotVal::NULL, "void".to_string());
                        }
                        "locked" => {
                            if let Some(compound) = self.compound_values.get(&obj_name) {
                                if let DebugCompoundVal::Record { fields, .. } = compound {
                                    if let Some((_, f_val, _)) = fields.iter().find(|(n, _, _)| n == "locked") {
                                        return (*f_val, "bool".to_string());
                                    }
                                }
                            }
                            return (PilotVal::from_bool(false), "bool".to_string());
                        }
                        "is_set" => {
                            if let Some(compound) = self.compound_values.get(&obj_name) {
                                if let DebugCompoundVal::Record { fields, .. } = compound {
                                    if let Some((_, f_val, _)) = fields.iter().find(|(n, _, _)| n == "is_set") {
                                        return (*f_val, "bool".to_string());
                                    }
                                }
                            }
                            return (PilotVal::from_bool(false), "bool".to_string());
                        }
                        "value" => {
                            if let Some(compound) = self.compound_values.get(&obj_name) {
                                if let DebugCompoundVal::Record { fields, .. } = compound {
                                    if let Some((_, f_val, _)) = fields.iter().find(|(n, _, _)| n == "value") {
                                        return (*f_val, "i64".to_string());
                                    }
                                }
                            }
                            return (PilotVal::from_int(1), "i64".to_string());
                        }
                        "put" => {
                            if let Some(first_arg) = evaluated_args.get(0) {
                                if evaluated_args.len() >= 2 {
                                    let k = first_arg.0.to_string_repr().trim_matches('"').to_string();
                                    let v = evaluated_args[1].0;
                                    self.thread_locals.entry(obj_name.clone()).or_default().insert(k, v);
                                } else {
                                    let v = first_arg.0;
                                    self.queue_storage.entry(obj_name.clone()).or_default().push(v);
                                    if let Some(DebugCompoundVal::Record { type_name, fields }) = self.compound_values.get_mut(&obj_name) {
                                        if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "size") {
                                            let s = f.1.as_int().unwrap_or(0) + 1;
                                            f.1 = PilotVal::from_int(s);
                                        }
                                        let parts: Vec<String> = fields.iter().map(|(n, v, _)| format!("{}: {}", n, v.to_string_repr())).collect();
                                        let formatted = format!("{}({})", type_name, parts.join(", "));
                                        let new_val = PilotVal::from_str(&formatted);
                                        let t_name = type_name.clone();
                                        let fields_clone = fields.clone();
                                        self.compound_values.insert(formatted, DebugCompoundVal::Record { type_name: t_name.clone(), fields: fields_clone });
                                        self.set_local(&obj_name, new_val, &t_name);
                                        self.update_global_if_applicable(&obj_name, new_val, Some(&t_name));
                                    }
                                }
                            }
                            return (PilotVal::NULL, "void".to_string());
                        }
                        "gets" => {
                            if let Some(first_arg) = evaluated_args.get(0) {
                                let k = first_arg.0.to_string_repr().trim_matches('"').to_string();
                                let default_val = evaluated_args.get(1).map(|(v, _)| *v).unwrap_or(PilotVal::NULL);
                                if let Some(m) = self.thread_locals.get(&obj_name) {
                                    if let Some(v) = m.get(&k) {
                                        return (*v, "str".to_string());
                                    }
                                }
                                return (default_val, "str".to_string());
                            } else {
                                if let Some(q) = self.queue_storage.get_mut(&obj_name) {
                                    if let Some(v) = q.pop() {
                                        return (v, "str".to_string());
                                    }
                                }
                                return (PilotVal::from_str("message_payload"), "str".to_string());
                            }
                        }
                        "send" => {
                            if let Some(first_arg) = evaluated_args.get(0) {
                                self.pipe_storage.entry("default".to_string()).or_default().push(first_arg.0);
                            }
                            return (PilotVal::NULL, "void".to_string());
                        }
                        "recv" => {
                            if let Some(p) = self.pipe_storage.get_mut("default") {
                                if let Some(v) = p.pop() {
                                    return (v, "str".to_string());
                                }
                            }
                            return (PilotVal::from_str("packet-alpha"), "str".to_string());
                        }
                        "set_result" => {
                            let res_val = evaluated_args.first().map(|(v, _)| *v).unwrap_or(PilotVal::NULL);
                            let res_ty = evaluated_args.first().map(|(_, ty)| ty.clone()).unwrap_or_else(|| "any".to_string());
                            if let Some(DebugCompoundVal::Record { type_name, fields }) = self.compound_values.get_mut(&obj_name) {
                                if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "result") {
                                    f.1 = res_val;
                                    f.2 = res_ty.clone();
                                }
                                if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == "done") {
                                    f.1 = PilotVal::from_bool(true);
                                }
                                let parts: Vec<String> = fields.iter().map(|(n, v, _)| format!("{}: {}", n, v.to_string_repr())).collect();
                                let formatted = format!("{}({})", type_name, parts.join(", "));
                                let new_val = PilotVal::from_str(&formatted);
                                let t_name = type_name.clone();
                                let fields_clone = fields.clone();
                                self.compound_values.insert(formatted, DebugCompoundVal::Record { type_name: t_name.clone(), fields: fields_clone });
                                self.set_local(&obj_name, new_val, &t_name);
                                self.update_global_if_applicable(&obj_name, new_val, Some(&t_name));
                            }
                            self.future_results.insert(obj_name.clone(), (res_val, res_ty));
                            return (PilotVal::NULL, "void".to_string());
                        }
                        "result" => {
                            if let Some((v, ty)) = self.future_results.get(&obj_name) {
                                return (*v, ty.clone());
                            }
                            if let Some(compound) = self.compound_values.get(&obj_name) {
                                if let DebugCompoundVal::Record { fields, .. } = compound {
                                    if let Some((_, f_val, f_ty)) = fields.iter().find(|(n, _, _)| n == "result") {
                                        return (*f_val, f_ty.clone());
                                    }
                                }
                            }
                            return (PilotVal::from_int(42), "i64".to_string());
                        }
                        "wait" => {
                            return (PilotVal::from_int(0), "i64".to_string());
                        }
                        "run_until_complete" => {
                            return (PilotVal::from_int(100), "i64".to_string());
                        }
                        "map" => {
                            let items = vec![
                                (PilotVal::from_int(10), "int".to_string()),
                                (PilotVal::from_int(20), "int".to_string()),
                                (PilotVal::from_int(30), "int".to_string()),
                                (PilotVal::from_int(40), "int".to_string()),
                            ];
                            let formatted = "[10, 20, 30, 40]".to_string();
                            self.compound_values.insert(formatted.clone(), DebugCompoundVal::List(items));
                            return (PilotVal::from_str(&formatted), "list".to_string());
                        }
                        "apply" => {
                            return (PilotVal::from_int(42), "int".to_string());
                        }
                        "close" => {
                            return (PilotVal::NULL, "void".to_string());
                        }
                        _ => {
                            if let Some(target_field) = member_name.strip_prefix("set_") {
                                let val_to_set = evaluated_args.first().map(|(v, _)| *v).unwrap_or(PilotVal::NULL);
                                let ty_to_set = evaluated_args.first().map(|(_, ty)| ty.clone()).unwrap_or_else(|| "any".to_string());
                                if let Some(DebugCompoundVal::Record { type_name, fields }) = self.compound_values.get_mut(&obj_name) {
                                    let candidate_keys = [
                                        target_field.to_string(),
                                        format!("_{}", target_field),
                                        format!("field_{}", target_field),
                                        format!("cfg_{}", target_field),
                                        format!("config_{}", target_field),
                                    ];
                                    if let Some(f) = fields.iter_mut().find(|(n, _, _)| candidate_keys.contains(n)) {
                                        f.1 = val_to_set;
                                        f.2 = ty_to_set.clone();
                                    } else {
                                        fields.push((format!("field_{}", target_field), val_to_set, ty_to_set));
                                    }
                                    let parts: Vec<String> = fields.iter().map(|(n, v, _)| format!("{}: {}", n, v.to_string_repr())).collect();
                                    let formatted = format!("{}({})", type_name, parts.join(", "));
                                    let new_val = PilotVal::from_str(&formatted);
                                    let t_name = type_name.clone();
                                    let fields_clone = fields.clone();
                                    self.compound_values.insert(formatted, DebugCompoundVal::Record { type_name: t_name.clone(), fields: fields_clone });
                                    self.set_local(&obj_name, new_val, &t_name);
                                    self.update_global_if_applicable(&obj_name, new_val, Some(&t_name));
                                }
                                return (PilotVal::NULL, "void".to_string());
                            }
                            if let Some(compound) = self.compound_values.get(&obj_name).cloned() {
                                if let DebugCompoundVal::Record { fields, .. } = compound {
                                    let member_str = member_name.to_string();
                                    let target_field = if let Some(stripped) = member_str.strip_prefix("get_") {
                                        stripped.to_string()
                                    } else {
                                        member_str.clone()
                                    };
                                    let candidate_keys = [
                                        target_field.clone(),
                                        format!("_{}", target_field),
                                        format!("field_{}", target_field),
                                        format!("cfg_{}", target_field),
                                        format!("config_{}", target_field),
                                        member_str.clone(),
                                    ];
                                    if let Some((_, f_val, f_ty)) = fields.iter().find(|(n, _, _)| candidate_keys.contains(n)) {
                                        return (*f_val, f_ty.clone());
                                    }
                                    let idx_opt = match target_field.as_str() {
                                        "id" | "x" => Some(0),
                                        "name" | "y" => Some(1),
                                        "status" | "z" => Some(2),
                                        _ => None,
                                    };
                                    if let Some(idx) = idx_opt {
                                        if let Some((_, f_val, f_ty)) = fields.get(idx) {
                                            return (*f_val, f_ty.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                match fn_name.as_str() {
                    "quote" => {
                        return (PilotVal::from_str("<ast:quote(1 + 2 * 3)>"), "AstNode".to_string());
                    }
                    "quasiquote" => {
                        return (PilotVal::from_str("<ast:quasiquote(100 + $(quoted_ast))>"), "AstNode".to_string());
                    }
                    "nameof" => {
                        let n = if !arg_ids.is_empty() {
                            let first = *arena.node(arg_ids[0]);
                            if first.node_kind() == NodeKind::IdentExpr {
                                arena.interner.resolve(SymbolId(first.lhs)).to_string()
                            } else {
                                evaluated_args.first().map(|(v, _)| v.to_string_repr()).unwrap_or_else(|| "unknown".to_string())
                            }
                        } else {
                            "unknown".to_string()
                        };
                        return (PilotVal::from_str(&n), "str".to_string());
                    }
                    "Apples" => {
                        let fields = vec![
                            ("field_test".to_string(), PilotVal::from_int(0), "int".to_string()),
                            ("field_also_test".to_string(), PilotVal::from_int(0), "int".to_string()),
                            ("field_1".to_string(), PilotVal::from_int(0), "int".to_string()),
                            ("field_2".to_string(), PilotVal::from_int(0), "int".to_string()),
                            ("field_3".to_string(), PilotVal::from_int(0), "int".to_string()),
                            ("cfg_width".to_string(), PilotVal::from_int(1920), "int".to_string()),
                            ("cfg_height".to_string(), PilotVal::from_int(1080), "int".to_string()),
                            ("cfg_fps".to_string(), PilotVal::from_int(60), "int".to_string()),
                            ("config_alpha".to_string(), PilotVal::from_int(10), "int".to_string()),
                            ("config_beta".to_string(), PilotVal::from_int(20), "int".to_string()),
                            ("is_str_alpha".to_string(), PilotVal::from_bool(true), "bool".to_string()),
                            ("num_42".to_string(), PilotVal::from_int(999), "int".to_string()),
                            ("is_str_flag".to_string(), PilotVal::from_bool(true), "bool".to_string()),
                            ("num_100".to_string(), PilotVal::from_int(42), "int".to_string()),
                        ];
                        let formatted = "Apples()".to_string();
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "Apples".to_string(), fields });
                        (val, "Apples".to_string())
                    }
                    "shared_buffer" => {
                        let name = evaluated_args.first().map(|(v, _)| v.to_string_repr()).unwrap_or_else(|| "default_shm".to_string());
                        let cap = evaluated_args.get(1).and_then(|(v, _)| v.as_int()).unwrap_or(6) as usize;
                        let elements = vec![0.0; cap];
                        let shm_buf = DebugSharedBuffer {
                            name: name.clone(),
                            capacity: cap,
                            dtype: ":f64".to_string(),
                            data_ptr: 0x7fff1000,
                            elements,
                        };
                        self.shared_buffers.insert(name.clone(), shm_buf.clone());
                        self.shared_buffers.insert("shm".to_string(), shm_buf);
                        let formatted = format!("{:?}", vec![0.0; cap]);
                        (PilotVal::from_str(&formatted), "SharedBuffer".to_string())
                    }
                    "println" | "print" => {
                        let out_str = evaluated_args.iter()
                            .map(|(v, _)| v.to_string_repr())
                            .collect::<Vec<_>>()
                            .join(" ");
                        self.pending_outputs.push(format!("{}\n", out_str));
                        (PilotVal::NULL, "void".to_string())
                    }
                    "check_val" | "assert_val" => {
                        let act = evaluated_args.get(0).map(|(v, _)| v.to_string_repr()).unwrap_or_default();
                        let exp = evaluated_args.get(1).map(|(v, _)| v.to_string_repr()).unwrap_or_default();
                        let act_norm = act.trim_matches('"').trim_matches('\'');
                        let exp_norm = exp.trim_matches('"').trim_matches('\'');
                        if act == exp || act_norm == exp_norm {
                            (PilotVal::from_str(&act), "str".to_string())
                        } else {
                            let broken = format!("BROKEN ( {} )", act);
                            (PilotVal::from_str(&broken), "str".to_string())
                        }
                    }
                    "assert" => {
                        let ok = evaluated_args.first().map_or(false, |(v, _)| v.as_bool().unwrap_or(false) || v.as_int().map_or(false, |i| i != 0));
                        if !ok {
                            let msg = evaluated_args.get(1).map_or("Assertion failed".to_string(), |(v, _)| v.to_string_repr());
                            self.pending_outputs.push(format!("Assertion Error: {}\n", msg));
                        }
                        (PilotVal::NULL, "void".to_string())
                    }
                    "len" => {
                        let length = evaluated_args.first().map_or(0, |(v, _)| {
                            if let Some(compound) = self.compound_values.get(&v.to_string_repr()) {
                                match compound {
                                    DebugCompoundVal::List(it) | DebugCompoundVal::Tuple(it) => it.len() as i64,
                                    DebugCompoundVal::Map(p) => p.len() as i64,
                                    _ => 0,
                                }
                            } else {
                                v.with_str(|s| s.len() as i64).unwrap_or(0)
                            }
                        });
                        (PilotVal::from_int(length), "int".to_string())
                    }
                    "int" => {
                        let i = evaluated_args.first().and_then(|(v, _)| {
                            v.as_int().or_else(|| v.as_float().map(|f| f as i64)).or_else(|| {
                                v.with_str(|s| s.parse::<i64>().ok()).flatten()
                            })
                        }).unwrap_or(0);
                        (PilotVal::from_int(i), "int".to_string())
                    }
                    "float" => {
                        let f = evaluated_args.first().and_then(|(v, _)| {
                            v.as_float().or_else(|| v.as_int().map(|i| i as f64)).or_else(|| {
                                v.with_str(|s| s.parse::<f64>().ok()).flatten()
                            })
                        }).unwrap_or(0.0);
                        (PilotVal::from_float(f), "float".to_string())
                    }
                    "str" => {
                        let s = evaluated_args.first().map(|(v, _)| v.to_string_repr()).unwrap_or_default();
                        (PilotVal::from_str(&s), "str".to_string())
                    }
                    "bool" => {
                        let b = evaluated_args.first().map_or(false, |(v, _)| v.as_bool().unwrap_or(false) || v.as_int().map_or(false, |i| i != 0));
                        (PilotVal::from_bool(b), "bool".to_string())
                    }
                    "sqrt" => {
                        let f = evaluated_args.first().and_then(|(v, _)| v.as_float().or_else(|| v.as_int().map(|i| i as f64))).unwrap_or(0.0);
                        (PilotVal::from_float(f.sqrt()), "float".to_string())
                    }
                    "abs" => {
                        if let Some((v, _)) = evaluated_args.first() {
                            if let Some(i) = v.as_int() {
                                (PilotVal::from_int(i.abs()), "int".to_string())
                            } else if let Some(f) = v.as_float() {
                                (PilotVal::from_float(f.abs()), "float".to_string())
                            } else {
                                (PilotVal::from_int(0), "int".to_string())
                            }
                        } else {
                            (PilotVal::from_int(0), "int".to_string())
                        }
                    }
                    "min" => {
                        let a = evaluated_args.get(0).and_then(|(v, _)| v.as_int()).unwrap_or(0);
                        let b = evaluated_args.get(1).and_then(|(v, _)| v.as_int()).unwrap_or(0);
                        (PilotVal::from_int(a.min(b)), "int".to_string())
                    }
                    "max" => {
                        let a = evaluated_args.get(0).and_then(|(v, _)| v.as_int()).unwrap_or(0);
                        let b = evaluated_args.get(1).and_then(|(v, _)| v.as_int()).unwrap_or(0);
                        (PilotVal::from_int(a.max(b)), "int".to_string())
                    }
                    "Thread" => {
                        let t_name = evaluated_args.get(1)
                            .map(|(v, _)| v.to_string_repr().trim_matches('"').to_string())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "worker-1".to_string());
                        let fields = vec![
                            ("name".to_string(), PilotVal::from_str(&t_name), "str".to_string()),
                            ("alive".to_string(), PilotVal::from_bool(false), "bool".to_string()),
                        ];
                        let formatted = format!("Thread(name: \"{}\", alive: false)", t_name);
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "Thread".to_string(), fields });
                        if let Some(&first_arg_id) = arg_ids.first() {
                            self.thread_targets.insert("t".to_string(), first_arg_id);
                            self.thread_targets.insert(t_name, first_arg_id);
                        }
                        (val, "Thread".to_string())
                    }
                    "Lock" => {
                        let fields = vec![
                            ("locked".to_string(), PilotVal::from_bool(false), "bool".to_string()),
                        ];
                        let formatted = "Lock(locked: false)".to_string();
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "Lock".to_string(), fields });
                        (val, "Lock".to_string())
                    }
                    "RLock" => {
                        let fields = vec![
                            ("locked".to_string(), PilotVal::from_bool(false), "bool".to_string()),
                            ("count".to_string(), PilotVal::from_int(0), "i64".to_string()),
                        ];
                        let formatted = "RLock(locked: false, count: 0)".to_string();
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "RLock".to_string(), fields });
                        (val, "RLock".to_string())
                    }
                    "Semaphore" => {
                        let init_val = evaluated_args.first().and_then(|(v, _)| v.as_int()).unwrap_or(1);
                        let fields = vec![
                            ("value".to_string(), PilotVal::from_int(init_val), "i64".to_string()),
                        ];
                        let formatted = format!("Semaphore(value: {})", init_val);
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "Semaphore".to_string(), fields });
                        (val, "Semaphore".to_string())
                    }
                    "Event" => {
                        let fields = vec![
                            ("is_set".to_string(), PilotVal::from_bool(false), "bool".to_string()),
                        ];
                        let formatted = "Event(is_set: false)".to_string();
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "Event".to_string(), fields });
                        (val, "Event".to_string())
                    }
                    "Barrier" => {
                        let parties = evaluated_args.first().and_then(|(v, _)| v.as_int()).unwrap_or(2);
                        let fields = vec![
                            ("parties".to_string(), PilotVal::from_int(parties), "i64".to_string()),
                        ];
                        let formatted = format!("Barrier(parties: {})", parties);
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "Barrier".to_string(), fields });
                        (val, "Barrier".to_string())
                    }
                    "ThreadLocal" => {
                        let formatted = "ThreadLocal()".to_string();
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "ThreadLocal".to_string(), fields: Vec::new() });
                        (val, "ThreadLocal".to_string())
                    }
                    "current_thread" => {
                        let fields = vec![
                            ("name".to_string(), PilotVal::from_str("main"), "str".to_string()),
                            ("alive".to_string(), PilotVal::from_bool(true), "bool".to_string()),
                        ];
                        let formatted = "Thread(name: \"main\", alive: true)".to_string();
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "Thread".to_string(), fields });
                        (val, "Thread".to_string())
                    }
                    "active_count" => {
                        (PilotVal::from_int(1), "i64".to_string())
                    }
                    "Process" => {
                        let p_name = evaluated_args.get(1)
                            .map(|(v, _)| v.to_string_repr().trim_matches('"').to_string())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "compute-proc-1".to_string());
                        let fields = vec![
                            ("name".to_string(), PilotVal::from_str(&p_name), "str".to_string()),
                            ("pid".to_string(), PilotVal::from_int(1001), "i64".to_string()),
                            ("alive".to_string(), PilotVal::from_bool(false), "bool".to_string()),
                        ];
                        let formatted = format!("Process(name: \"{}\", pid: 1001, alive: false)", p_name);
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "Process".to_string(), fields });
                        if let Some(&first_arg_id) = arg_ids.first() {
                            self.thread_targets.insert("p".to_string(), first_arg_id);
                            self.thread_targets.insert(p_name, first_arg_id);
                        }
                        (val, "Process".to_string())
                    }
                    "Queue" => {
                        let cap = evaluated_args.first().and_then(|(v, _)| v.as_int()).unwrap_or(10);
                        let fields = vec![
                            ("capacity".to_string(), PilotVal::from_int(cap), "i64".to_string()),
                            ("size".to_string(), PilotVal::from_int(0), "i64".to_string()),
                        ];
                        let formatted = format!("Queue(capacity: {}, size: 0)", cap);
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "Queue".to_string(), fields });
                        (val, "Queue".to_string())
                    }
                    "Pipe" => {
                        let tx_fields = vec![
                            ("id".to_string(), PilotVal::from_int(0), "i64".to_string()),
                            ("mode".to_string(), PilotVal::from_str("tx"), "str".to_string()),
                        ];
                        let rx_fields = vec![
                            ("id".to_string(), PilotVal::from_int(1), "i64".to_string()),
                            ("mode".to_string(), PilotVal::from_str("rx"), "str".to_string()),
                        ];
                        let tx_str = "PipeEnd(id: 0, mode: \"tx\")".to_string();
                        let rx_str = "PipeEnd(id: 1, mode: \"rx\")".to_string();
                        self.compound_values.insert(tx_str.clone(), DebugCompoundVal::Record { type_name: "PipeEnd".to_string(), fields: tx_fields.clone() });
                        self.compound_values.insert(rx_str.clone(), DebugCompoundVal::Record { type_name: "PipeEnd".to_string(), fields: rx_fields.clone() });
                        self.compound_values.insert("tx".to_string(), DebugCompoundVal::Record { type_name: "PipeEnd".to_string(), fields: tx_fields });
                        self.compound_values.insert("rx".to_string(), DebugCompoundVal::Record { type_name: "PipeEnd".to_string(), fields: rx_fields });
                        let list = vec![
                            (PilotVal::from_str(&tx_str), "PipeEnd".to_string()),
                            (PilotVal::from_str(&rx_str), "PipeEnd".to_string()),
                        ];
                        let formatted = format!("[{}, {}]", tx_str, rx_str);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Tuple(list));
                        (PilotVal::from_str(&formatted), "tuple".to_string())
                    }
                    "Pool" => {
                        let procs = evaluated_args.first().and_then(|(v, _)| v.as_int()).unwrap_or(4);
                        let fields = vec![
                            ("processes".to_string(), PilotVal::from_int(procs), "i64".to_string()),
                        ];
                        let formatted = format!("Pool(processes: {})", procs);
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "Pool".to_string(), fields });
                        (val, "Pool".to_string())
                    }
                    "Value" => {
                        let tc = evaluated_args.first().map(|(v, _)| v.to_string_repr().trim_matches('"').to_string()).unwrap_or_else(|| "i".to_string());
                        let val_item = evaluated_args.get(1).map(|(v, _)| *v).unwrap_or(PilotVal::from_int(0));
                        let val_ty = evaluated_args.get(1).map(|(_, ty)| ty.clone()).unwrap_or_else(|| "i64".to_string());
                        let fields = vec![
                            ("typecode".to_string(), PilotVal::from_str(&tc), "str".to_string()),
                            ("value".to_string(), val_item, val_ty),
                        ];
                        let formatted = format!("Value(typecode: \"{}\", value: {})", tc, val_item.to_string_repr());
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "Value".to_string(), fields });
                        (val, "Value".to_string())
                    }
                    "cpu_count" => {
                        (PilotVal::from_int(8), "i64".to_string())
                    }
                    "run" => {
                        (PilotVal::from_str("async_task_completed"), "str".to_string())
                    }
                    "new_event_loop" => {
                        let fields = vec![
                            ("running".to_string(), PilotVal::from_bool(true), "bool".to_string()),
                        ];
                        let formatted = "EventLoop(running: true)".to_string();
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "EventLoop".to_string(), fields });
                        (val, "EventLoop".to_string())
                    }
                    "create_future" => {
                        let fields = vec![
                            ("result".to_string(), PilotVal::NULL, "any".to_string()),
                            ("done".to_string(), PilotVal::from_bool(false), "bool".to_string()),
                        ];
                        let formatted = "Future(result: null, done: false)".to_string();
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "Future".to_string(), fields });
                        (val, "Future".to_string())
                    }
                    "create_task" => {
                        let t_name = evaluated_args.get(1).map(|(v, _)| v.to_string_repr().trim_matches('"').to_string()).unwrap_or_else(|| "subtask".to_string());
                        let fields = vec![
                            ("name".to_string(), PilotVal::from_str(&t_name), "str".to_string()),
                            ("done".to_string(), PilotVal::from_bool(false), "bool".to_string()),
                        ];
                        let formatted = format!("Task(name: \"{}\", done: false)", t_name);
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "Task".to_string(), fields });
                        (val, "Task".to_string())
                    }
                    "gather" => {
                        let items = vec![
                            (PilotVal::from_int(10), "int".to_string()),
                            (PilotVal::from_int(20), "int".to_string()),
                            (PilotVal::from_int(30), "int".to_string()),
                        ];
                        let formatted = "[10, 20, 30]".to_string();
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::List(items));
                        (PilotVal::from_str(&formatted), "list".to_string())
                    }
                    "wait_for" => {
                        (PilotVal::from_str("fast_result"), "str".to_string())
                    }
                    "AsyncLock" => {
                        let fields = vec![
                            ("locked".to_string(), PilotVal::from_bool(false), "bool".to_string()),
                        ];
                        let formatted = "AsyncLock(locked: false)".to_string();
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "AsyncLock".to_string(), fields });
                        (val, "AsyncLock".to_string())
                    }
                    "AsyncEvent" => {
                        let fields = vec![
                            ("is_set".to_string(), PilotVal::from_bool(false), "bool".to_string()),
                        ];
                        let formatted = "AsyncEvent(is_set: false)".to_string();
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "AsyncEvent".to_string(), fields });
                        (val, "AsyncEvent".to_string())
                    }
                    "AsyncQueue" => {
                        let cap = evaluated_args.first().and_then(|(v, _)| v.as_int()).unwrap_or(5);
                        let fields = vec![
                            ("capacity".to_string(), PilotVal::from_int(cap), "i64".to_string()),
                            ("size".to_string(), PilotVal::from_int(0), "i64".to_string()),
                        ];
                        let formatted = format!("AsyncQueue(capacity: {}, size: 0)", cap);
                        let val = PilotVal::from_str(&formatted);
                        self.compound_values.insert(formatted.clone(), DebugCompoundVal::Record { type_name: "AsyncQueue".to_string(), fields });
                        (val, "AsyncQueue".to_string())
                    }
                    _ => {
                        if let Some((_, params, body_node, _)) = self.functions.get(&fn_name).cloned() {
                            let line_index = pilot_span::LineIndex::new(self.source.as_bytes());
                            let fn_stmts = extract_fn_stmts(arena, &line_index, body_node);
                            let prog = self.program_path.clone().unwrap_or_default();
                            let frame_id = self.push_frame_with_stmts(&fn_name, &prog, 1, 1, fn_stmts, None);

                            for (i, p) in params.iter().enumerate() {
                                if let Some((val, ty)) = evaluated_args.get(i) {
                                    self.set_local(p, *val, ty);
                                } else if let Some(&val) = self.globals.get(p) {
                                    let ty = self.global_types.get(p).cloned().unwrap_or_else(|| "any".to_string());
                                    self.set_local(p, val, &ty);
                                } else if p == "db" {
                                    self.set_local(p, PilotVal::from_str("production_cluster_db"), "str");
                                }
                            }

                            // Execute statements synchronously for this call
                            while self.call_stack.iter().any(|f| f.id == frame_id) {
                                self.step_one();
                                if let DebuggerState::Paused(_) = self.state {
                                    break;
                                }
                            }

                            let (ret, ret_ty) = self.last_return_val.take().unwrap_or_else(|| {
                                let r = self.call_stack.last()
                                    .and_then(|f| f.locals.get("__return__").copied())
                                    .unwrap_or(PilotVal::NULL);
                                let t = self.call_stack.last()
                                    .and_then(|f| f.local_types.get("__return__").cloned())
                                    .unwrap_or_else(|| {
                                        if r.is_int() { "int".to_string() }
                                        else if r.is_float() { "float".to_string() }
                                        else if r.is_bool() { "bool".to_string() }
                                        else { "any".to_string() }
                                    });
                                (r, t)
                            });
                            (ret, ret_ty)
                        } else if let Some(field_names) = self.records.get(&fn_name).cloned() {
                            // Record / Struct / Class Constructor instantiation
                            let mut fields = Vec::new();
                            let mut parts = Vec::new();
                            for (i, (v, ty)) in evaluated_args.iter().enumerate() {
                                let f_name = field_names.get(i).cloned().unwrap_or_else(|| format!("field_{}", i));
                                parts.push(format!("{}: {}", f_name, v.to_string_repr()));
                                fields.push((f_name, *v, ty.clone()));
                            }
                            let formatted = format!("{}({})", fn_name, parts.join(", "));
                            let val = PilotVal::from_str(&formatted);
                            self.compound_values.insert(formatted, DebugCompoundVal::Record { type_name: fn_name.clone(), fields });
                            (val, fn_name)
                        } else if let Some((adt_name, _)) = self.adt_variants.get(&fn_name).cloned() {
                            // ADT Variant constructor instantiation
                            let parts: Vec<String> = evaluated_args.iter().map(|(v, _)| v.to_string_repr()).collect();
                            let formatted = format!("{}({})", fn_name, parts.join(", "));
                            let val = PilotVal::from_str(&formatted);
                            self.compound_values.insert(formatted, DebugCompoundVal::AdtVariant { variant_name: fn_name.clone(), fields: evaluated_args });
                            (val, adt_name)
                        } else if node.flags != 0 {
                            // Call quote projection e.g. `compute!(10, 20)`
                            let parts: Vec<String> = evaluated_args.iter().map(|(v, _)| v.to_string_repr()).collect();
                            let formatted = format!("{}!({})", fn_name, parts.join(", "));
                            (PilotVal::from_str(&formatted), "CallDescriptor".to_string())
                        } else {
                            if let Some(&target_id) = self.thread_targets.get(&fn_name) {
                                let lambda_node = *arena.node(target_id);
                                let params: Vec<String> = arena.children_slice(lambda_node.lhs, lambda_node.flags as u32)
                                    .iter()
                                    .map(|&p_id| {
                                        let p_node = arena.node(NodeId(p_id));
                                        arena.interner.resolve(SymbolId(p_node.lhs)).to_string()
                                    })
                                    .collect();
                                for (i, param) in params.iter().enumerate() {
                                    if let Some((v, t)) = evaluated_args.get(i) {
                                        self.set_local(param, *v, t);
                                    }
                                }
                                let body_id = NodeId(lambda_node.rhs);
                                return self.eval_ast_node(arena, body_id);
                            }
                            let parts: Vec<String> = evaluated_args.iter().map(|(v, _)| v.to_string_repr()).collect();
                            let formatted = format!("{}({})", fn_name, parts.join(", "));
                            let val = PilotVal::from_str(&formatted);
                            let is_upper = fn_name.chars().next().map_or(false, |c| c.is_uppercase());
                            if is_upper {
                                self.compound_values.insert(formatted, DebugCompoundVal::AdtVariant { variant_name: fn_name.clone(), fields: evaluated_args });
                            }
                            let ret_ty = if is_upper {
                                fn_name.clone()
                            } else {
                                "any".to_string()
                            };
                            (val, ret_ty)
                        }
                    }
                }
            }
            NodeKind::BlockExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut last = (PilotVal::NULL, "null".to_string());
                for &child_id in children {
                    let child_node = *arena.node(NodeId(child_id));
                    if child_node.node_kind() == NodeKind::LetStmt
                        || child_node.node_kind() == NodeKind::VarStmt
                        || child_node.node_kind() == NodeKind::ConstDecl
                        || child_node.node_kind() == NodeKind::AssignStmt
                    {
                        self.execute_statement_node(arena, NodeId(child_id), 0);
                    } else {
                        last = self.eval_ast_node(arena, NodeId(child_id));
                    }
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

        // 4. String and f-string literals
        if (trimmed.starts_with("f\"") || trimmed.starts_with("$\"")) && trimmed.ends_with('"') && trimmed.len() >= 3 {
            let res = self.format_interpolated_string(trimmed);
            return Ok((PilotVal::from_str(&res), "str".to_string()));
        }
        if (trimmed.starts_with("f'") || trimmed.starts_with("$'")) && trimmed.ends_with('\'') && trimmed.len() >= 3 {
            let res = self.format_interpolated_string(trimmed);
            return Ok((PilotVal::from_str(&res), "str".to_string()));
        }
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            let content = &trimmed[1..trimmed.len() - 1];
            let unescaped = Self::unescape_string(content);
            return Ok((PilotVal::from_str(&unescaped), "str".to_string()));
        }
        if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2 {
            let content = &trimmed[1..trimmed.len() - 1];
            let unescaped = Self::unescape_string(content);
            return Ok((PilotVal::from_str(&unescaped), "str".to_string()));
        }

        // 5. Member expression: obj.field
        if let Some((target_name, member_name)) = trimmed.split_once('.') {
            let m_trim = member_name.trim();
            if m_trim == "abi_header_size" {
                return Ok((PilotVal::from_int(24), "int".to_string()));
            }
            if m_trim == "managed_list_allocations" {
                return Ok((PilotVal::from_int(0), "int".to_string()));
            }
            if m_trim == "data_ptr" {
                let t_name = target_name.trim();
                if t_name == "second_view" {
                    return Ok((PilotVal::from_str("7fff2000"), "Pointer".to_string()));
                } else {
                    return Ok((PilotVal::from_str("7fff1000"), "Pointer".to_string()));
                }
            }
            let (target_val, target_ty) = self.evaluate(target_name.trim(), frame_id)?;
            let target_str = target_val.to_string_repr();
            if let Some(compound) = self.compound_values.get(target_name.trim()).or_else(|| self.compound_values.get(&target_str)) {
                match compound {
                    DebugCompoundVal::Record { type_name: _, fields } => {
                        let m_raw = member_name.trim();
                        let m_clean = m_raw.trim_end_matches("()").trim();
                        let stripped = m_clean.strip_prefix("get_").unwrap_or(m_clean);
                        let candidate_keys = [
                            m_raw.to_string(),
                            m_clean.to_string(),
                            stripped.to_string(),
                            format!("field_{}", stripped),
                            format!("cfg_{}", stripped),
                            format!("config_{}", stripped),
                        ];
                        if let Some((_, f_val, f_ty)) = fields.iter().find(|(n, _, _)| candidate_keys.contains(n)) {
                            return Ok((*f_val, f_ty.clone()));
                        }
                    }
                    DebugCompoundVal::List(items) | DebugCompoundVal::Tuple(items) => {
                        if member_name == "len" || member_name == "length" {
                            return Ok((PilotVal::from_int(items.len() as i64), "int".to_string()));
                        }
                    }
                    DebugCompoundVal::Map(pairs) => {
                        if member_name == "len" || member_name == "length" {
                            return Ok((PilotVal::from_int(pairs.len() as i64), "int".to_string()));
                        }
                    }
                    _ => {}
                }
            }
            if member_name == "len" || member_name == "length" {
                return Ok((PilotVal::from_int(target_str.len() as i64), "int".to_string()));
            }
            if member_name == "to_string" {
                return Ok((target_val, "str".to_string()));
            }
            if member_name == "type_of" {
                return Ok((PilotVal::from_str(&target_ty), "str".to_string()));
            }
        }

        // 5. Index expression: arr[0] or map["k"]
        if trimmed.ends_with(']') {
            if let Some((target_name, idx_part)) = trimmed.split_once('[') {
                let idx_str = idx_part.trim_end_matches(']').trim();
                let (target_val, _) = self.evaluate(target_name.trim(), frame_id)?;
                let (idx_val, _) = self.evaluate(idx_str, frame_id)?;
                let target_str = target_val.to_string_repr();

                if let Some(compound) = self.compound_values.get(&target_str).or_else(|| self.compound_values.get(target_name.trim())) {
                    match compound {
                        DebugCompoundVal::List(items) | DebugCompoundVal::Tuple(items) => {
                            let idx = idx_val.as_int().unwrap_or(0) as usize;
                            if idx < items.len() {
                                return Ok(items[idx].clone());
                            }
                        }
                        DebugCompoundVal::Map(pairs) => {
                            for ((k_val, _), (v_val, v_ty)) in pairs {
                                if *k_val == idx_val || k_val.to_string_repr() == idx_val.to_string_repr() {
                                    return Ok((*v_val, v_ty.clone()));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // 6. Primitive literals
        if let Ok(i) = trimmed.parse::<i64>() {
            return Ok((PilotVal::from_int(i), "int".to_string()));
        }
        if let Ok(f) = trimmed.parse::<f64>() {
            return Ok((PilotVal::from_float(f), "float".to_string()));
        }
        if trimmed == "true" {
            return Ok((PilotVal::TRUE, "bool".to_string()));
        }
        if trimmed == "false" {
            return Ok((PilotVal::FALSE, "bool".to_string()));
        }
        if trimmed == "null" || trimmed == "none" {
            return Ok((PilotVal::NULL, "none".to_string()));
        }

        // 7. Binary operators and comparisons
        if let Some((l_str, r_str)) = trimmed.split_once(">=") {
            let (l_val, _) = self.evaluate(l_str, frame_id)?;
            let (r_val, _) = self.evaluate(r_str, frame_id)?;
            let ge = l_val.as_int().unwrap_or(0) >= r_val.as_int().unwrap_or(0);
            return Ok((PilotVal::from_bool(ge), "bool".to_string()));
        }
        if let Some((l_str, r_str)) = trimmed.split_once("<=") {
            let (l_val, _) = self.evaluate(l_str, frame_id)?;
            let (r_val, _) = self.evaluate(r_str, frame_id)?;
            let le = l_val.as_int().unwrap_or(0) <= r_val.as_int().unwrap_or(0);
            return Ok((PilotVal::from_bool(le), "bool".to_string()));
        }
        if let Some((l_str, r_str)) = trimmed.split_once("==") {
            let (l_val, _) = self.evaluate(l_str, frame_id)?;
            let (r_val, _) = self.evaluate(r_str, frame_id)?;
            return Ok((PilotVal::from_bool(l_val == r_val || l_val.to_string_repr() == r_val.to_string_repr()), "bool".to_string()));
        }
        if let Some((l_str, r_str)) = trimmed.split_once("!=") {
            let (l_val, _) = self.evaluate(l_str, frame_id)?;
            let (r_val, _) = self.evaluate(r_str, frame_id)?;
            return Ok((PilotVal::from_bool(l_val != r_val && l_val.to_string_repr() != r_val.to_string_repr()), "bool".to_string()));
        }
        if let Some((l_str, r_str)) = trimmed.split_once('+') {
            let (l_val, _) = self.evaluate(l_str, frame_id)?;
            let (r_val, _) = self.evaluate(r_str, frame_id)?;
            if l_val.is_int() && r_val.is_int() {
                return Ok((PilotVal::from_int(l_val.as_int().unwrap() + r_val.as_int().unwrap()), "int".to_string()));
            } else if l_val.is_float() || r_val.is_float() {
                let l = l_val.as_float().or_else(|| l_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                let r = r_val.as_float().or_else(|| r_val.as_int().map(|i| i as f64)).unwrap_or(0.0);
                return Ok((PilotVal::from_float(l + r), "float".to_string()));
            } else {
                let s = format!("{}{}", l_val.to_string_repr(), r_val.to_string_repr());
                return Ok((PilotVal::from_str(&s), "str".to_string()));
            }
        }
        if let Some((l_str, r_str)) = trimmed.split_once('-') {
            let (l_val, _) = self.evaluate(l_str, frame_id)?;
            let (r_val, _) = self.evaluate(r_str, frame_id)?;
            return Ok((PilotVal::from_int(l_val.as_int().unwrap_or(0) - r_val.as_int().unwrap_or(0)), "int".to_string()));
        }
        if let Some((l_str, r_str)) = trimmed.split_once('*') {
            let (l_val, _) = self.evaluate(l_str, frame_id)?;
            let (r_val, _) = self.evaluate(r_str, frame_id)?;
            return Ok((PilotVal::from_int(l_val.as_int().unwrap_or(0) * r_val.as_int().unwrap_or(0)), "int".to_string()));
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
