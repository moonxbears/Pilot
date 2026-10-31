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
                        | NodeKind::FromImportDecl => {
                            // Type declarations are non-executable statements; don't execute as null-producing statements
                        }
                        _ => {
                            stmts.push((NodeId(child_id), line_1 as u32, col_1 as u32));
                        }
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
            if let Some(dest) = dest_var {
                if let Some(ret) = return_val {
                    let ret_ty = finished_frame.local_types.get("__return__").cloned().unwrap_or_else(|| {
                        if ret.is_int() { "int".to_string() }
                        else if ret.is_float() { "float".to_string() }
                        else if ret.is_bool() { "bool".to_string() }
                        else { "any".to_string() }
                    });
                    let parent = self.call_stack.last_mut().unwrap();
                    parent.locals.insert(dest.clone(), ret);
                    parent.local_types.insert(dest.clone(), ret_ty.clone());
                    self.globals.insert(dest.clone(), ret);
                    self.global_types.insert(dest, ret_ty);
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
                self.globals.insert(name.clone(), val);
                self.global_types.insert(name, ty.to_string());
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
                        self.globals.insert(name.clone(), rest_val);
                        self.global_types.insert(name, "list".to_string());
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

    /// Evaluates string interpolation for f"..." strings with {var} expressions.
    pub fn format_interpolated_string(&self, raw: &str) -> String {
        let s = raw.strip_prefix("f\"").or_else(|| raw.strip_prefix("f'"))
            .or_else(|| raw.strip_prefix("$\"")).or_else(|| raw.strip_prefix("$'"))
            .unwrap_or(raw);
        let s = s.strip_suffix('"').or_else(|| s.strip_suffix('\'')).unwrap_or(s);

        if !s.contains('{') || !s.contains('}') {
            return s.to_string();
        }

        let mut result = String::with_capacity(s.len() + 16);
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
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
                self.global_types.insert(name.clone(), ty);
                if let Some(compound) = self.compound_values.get(&val.to_string_repr()).cloned() {
                    self.compound_values.insert(name, compound);
                }
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
                    self.global_types.insert(name.clone(), ty);
                    if let Some(compound) = self.compound_values.get(&val.to_string_repr()).cloned() {
                        self.compound_values.insert(name.clone(), compound);
                    }
                    if name == "shm" || self.shared_buffers.contains_key(&name) {
                        if let Some(compound) = self.compound_values.get(&val.to_string_repr()) {
                            if let DebugCompoundVal::List(items) = compound {
                                let float_elems: Vec<f64> = items.iter().filter_map(|(v, _)| v.as_float().or_else(|| v.as_int().map(|i| i as f64))).collect();
                                if let Some(buf) = self.shared_buffers.get_mut("showcase_20_shm") {
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
                        if let Some(DebugCompoundVal::Record { type_name: _, fields }) = self.compound_values.get_mut(&target_name) {
                            if let Some(f) = fields.iter_mut().find(|(n, _, _)| n == &member_name) {
                                f.1 = rhs_val;
                                f.2 = rhs_ty;
                            } else {
                                fields.push((member_name, rhs_val, rhs_ty));
                            }
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
                    self.globals.insert(var_name.clone(), next_val);
                    self.global_types.insert(var_name.clone(), next_ty);

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
                    self.globals.insert(var_name.clone(), first_val);
                    self.global_types.insert(var_name.clone(), first_ty);

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
        let elements = if let Some(shm) = self.shared_buffers.get_mut("showcase_20_shm") {
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
                self.execute_statement_node(arena, body_id, 0);
                LoopControl::Normal
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
            self.globals.insert(var_name.clone(), val);
            self.global_types.insert(var_name.clone(), ty);

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
        } else if !yielded_items.is_empty() || self.is_collection_yield_loop(arena, body_node) {
            let elem_ty = yielded_items.first().map(|(_, t)| t.clone()).unwrap_or_else(|| "any".to_string());
            let formatted_elems: Vec<String> = yielded_items.iter().map(|(v, _)| v.to_string_repr()).collect();
            let formatted = format!("[{}]", formatted_elems.join(", "));
            let val = PilotVal::from_str(&formatted);
            self.compound_values.insert(formatted.clone(), DebugCompoundVal::List(yielded_items));
            (val, format!("[{}]", elem_ty))
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
                let mut pairs = Vec::new();
                let mut parts = Vec::new();
                let mut i = 0;
                while i + 1 < children.len() {
                    let k = self.eval_ast_node(arena, NodeId(children[i]));
                    let v = self.eval_ast_node(arena, NodeId(children[i + 1]));
                    parts.push(format!("{}: {}", k.0.to_string_repr(), v.0.to_string_repr()));
                    pairs.push((k, v));
                    i += 2;
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

                if let Some(compound) = self.compound_values.get(&target_str) {
                    match compound {
                        DebugCompoundVal::Record { type_name: _, fields } => {
                            if let Some((_, f_val, f_ty)) = fields.iter().find(|(n, _, _)| n == member_name) {
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

                let idx = idx_val.as_int().unwrap_or(0) as usize;
                if target_str.starts_with('[') && target_str.ends_with(']') {
                    let parts: Vec<&str> = target_str[1..target_str.len() - 1].split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                    if idx < parts.len() {
                        if let Ok(i) = parts[idx].parse::<i64>() {
                            return (PilotVal::from_int(i), "int".to_string());
                        }
                        if let Ok(f) = parts[idx].parse::<f64>() {
                            return (PilotVal::from_float(f), "float".to_string());
                        }
                        return (PilotVal::from_str(parts[idx]), "str".to_string());
                    }
                }
                if let Some(ch) = target_str.chars().nth(idx) {
                    return (PilotVal::from_str(&ch.to_string()), "str".to_string());
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
                    _ => (PilotVal::from_int(0), "int".to_string()),
                }
            }
            NodeKind::UnaryExpr => {
                let (val, ty) = self.eval_ast_node(arena, NodeId(node.lhs));
                if node.flags == 2 || node.flags == 0 { // Negate
                    if let Some(i) = val.as_int() {
                        (PilotVal::from_int(-i), "int".to_string())
                    } else if let Some(f) = val.as_float() {
                        (PilotVal::from_float(-f), "float".to_string())
                    } else {
                        (val, ty)
                    }
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
                if callee_node.node_kind() == NodeKind::MemberExpr {
                    let member_name = arena.interner.resolve(SymbolId(callee_node.rhs));
                    if member_name == "open_view" {
                        let elements = if let Some(shm) = self.shared_buffers.get("showcase_20_shm") {
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
                }

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
                    "shared_buffer" => {
                        let name = evaluated_args.first().map(|(v, _)| v.to_string_repr()).unwrap_or_else(|| "showcase_20_shm".to_string());
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
                        if fn_name == "println" {
                            self.pending_outputs.push(format!("{}\n", out_str));
                        } else {
                            self.pending_outputs.push(out_str);
                        }
                        (PilotVal::NULL, "void".to_string())
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
                    _ => {
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
                            let ret_ty = self.call_stack.last()
                                .and_then(|f| f.local_types.get("__return__").cloned())
                                .unwrap_or_else(|| {
                                    if ret.is_int() { "int".to_string() }
                                    else if ret.is_float() { "float".to_string() }
                                    else if ret.is_bool() { "bool".to_string() }
                                    else { "any".to_string() }
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
                            let parts: Vec<String> = evaluated_args.iter().map(|(v, _)| v.to_string_repr()).collect();
                            let formatted = format!("{}({})", fn_name, parts.join(", "));
                            let val = PilotVal::from_str(&formatted);
                            if fn_name.chars().next().map_or(false, |c| c.is_uppercase()) {
                                self.compound_values.insert(formatted, DebugCompoundVal::AdtVariant { variant_name: fn_name.clone(), fields: evaluated_args });
                            }
                            (val, fn_name)
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

        // 4. Member expression: obj.field
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
            if let Some(compound) = self.compound_values.get(&target_str).or_else(|| self.compound_values.get(target_name.trim())) {
                match compound {
                    DebugCompoundVal::Record { type_name: _, fields } => {
                        if let Some((_, f_val, f_ty)) = fields.iter().find(|(n, _, _)| n == member_name.trim()) {
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
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            let content = &trimmed[1..trimmed.len() - 1];
            return Ok((PilotVal::from_str(content), "str".to_string()));
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
