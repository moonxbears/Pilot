//! Milestone 33.3: Call-Quoting Projections (!, !!, !&, !^, !@, !?) & Static Demotion Pass (COMPTIME001–COMPTIME010).

use pilot_span::ByteSpan;
use crate::diagnostics::AnalysisDiagnostic;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum QuoteKind {
    /// `!expr`: Shallow quote (unevaluated AST node)
    Shallow,
    /// `!!expr`: Deep splice with hygienic constant folding
    DeepSplice,
    /// `!&fn`: C-ABI call frame layout synthesis
    CallFrameLayout,
    /// `!^lval`: Observable LValue extraction
    ObservableLValue,
    /// `!@expr`: Symbolic differentiation + Common Subexpression Elimination (CSE)
    SymbolicDiff,
    /// `!?call`: Copy-on-Write dry run
    CoWDryRun,
}

#[derive(Clone, Debug)]
pub struct CallFrameLayoutInfo {
    pub fn_name: String,
    pub total_size_bytes: usize,
    pub param_offsets: Vec<(String, usize, usize)>, // (name, offset, size)
}

#[derive(Clone, Debug)]
pub struct StaticDemotedSlot {
    pub var_name: String,
    pub frame_depth: u32,
    pub slot_index: u32,
    pub is_read_only: bool,
}

pub struct ComptimeEvaluator {
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub demoted_slots: Vec<StaticDemotedSlot>,
}

impl Default for ComptimeEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl ComptimeEvaluator {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            demoted_slots: Vec::new(),
        }
    }

    pub fn compute_call_frame_layout(fn_name: &str, params: &[(&str, usize)]) -> CallFrameLayoutInfo {
        let mut offset = 0;
        let mut param_offsets = Vec::new();
        for (name, size) in params {
            // Align to 8 bytes
            let aligned = (offset + 7) & !7;
            param_offsets.push((name.to_string(), aligned, *size));
            offset = aligned + size;
        }
        let total_size_bytes = (offset + 7) & !7;
        CallFrameLayoutInfo {
            fn_name: fn_name.to_string(),
            total_size_bytes,
            param_offsets,
        }
    }

    pub fn report_comptime_side_effect(&mut self, operation: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "COMPTIME001",
            format!("Side-effect '{}' is illegal in comptime execution context", operation),
            span,
        ));
    }

    pub fn report_dynamic_comptime_dependency(&mut self, var_name: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "COMPTIME002",
            format!("Comptime evaluation depends on runtime variable '{}' which cannot be known at compile time", var_name),
            span,
        ));
    }

    pub fn record_demoted_slot(&mut self, var_name: &str, frame_depth: u32, slot_index: u32, is_read_only: bool) {
        self.demoted_slots.push(StaticDemotedSlot {
            var_name: var_name.to_string(),
            frame_depth,
            slot_index,
            is_read_only,
        });
    }

    /// Performs the Milestone 19 compiler-wide static demotion pass across `AstArena`.
    pub fn demote_arena(&mut self, arena: &pilot_ast::AstArena, root: pilot_ast::NodeId) {
        if root.is_none() {
            return;
        }

        let mut current_depth: u32 = 0;
        let mut slot_counter: u32 = 0;

        for i in 0..arena.nodes.len() {
            let node_id = pilot_ast::NodeId(i as u32);
            let node = *arena.node(node_id);
            let span = arena.span(node_id);

            match node.node_kind() {
                pilot_ast::NodeKind::FnDecl => {
                    current_depth += 1;
                    slot_counter = 0;
                }
                pilot_ast::NodeKind::LetStmt => {
                    let sym = pilot_intern::SymbolId(node.lhs);
                    let var_name = arena.interner.resolve(sym);
                    self.record_demoted_slot(var_name, current_depth, slot_counter, true);
                    slot_counter += 1;
                }
                pilot_ast::NodeKind::VarStmt => {
                    let sym = pilot_intern::SymbolId(node.lhs);
                    let var_name = arena.interner.resolve(sym);
                    self.record_demoted_slot(var_name, current_depth, slot_counter, false);
                    slot_counter += 1;
                }
                pilot_ast::NodeKind::ParamDecl => {
                    let sym = pilot_intern::SymbolId(node.lhs);
                    let var_name = arena.interner.resolve(sym);
                    self.record_demoted_slot(var_name, current_depth, slot_counter, true);
                    slot_counter += 1;
                }
                pilot_ast::NodeKind::ComptimeExpr => {
                    // Check if comptime block contains illegal dynamic constructs
                    let inner_id = pilot_ast::NodeId(node.lhs);
                    if inner_id.is_some() {
                        let inner_node = *arena.node(inner_id);
                        if inner_node.node_kind() == pilot_ast::NodeKind::CallExpr {
                            let callee = *arena.node(pilot_ast::NodeId(inner_node.lhs));
                            if callee.node_kind() == pilot_ast::NodeKind::IdentExpr {
                                let cname = arena.interner.resolve(pilot_intern::SymbolId(callee.lhs));
                                if cname == "print" || cname == "write" || cname == "malloc" {
                                    self.report_comptime_side_effect(cname, span);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
