//! High-level coordinator for Milestone 33 static intelligence passes.

use std::collections::HashMap;
use pilot_ast::{AstArena, NodeId};
use crate::comptime::ComptimeEvaluator;
use crate::contracts::ContractVerifier;
use crate::diagnostics::AnalysisDiagnostic;
use crate::dimensions::DimensionChecker;
use crate::infer::TypeInferenceEngine;
use crate::loops::LoopAnalyzer;
use crate::patterns::MarangetChecker;
use crate::types::{TypeArena, TypeId};

pub struct AnalysisResult {
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub types: TypeArena,
    pub inferred_types: HashMap<u32, TypeId>,
    pub struct_fields: HashMap<String, Vec<(String, TypeId)>>,
    pub struct_member_visibility: HashMap<(String, String), pilot_ast::VisibilityTier>,
    pub struct_member_clearance: HashMap<(String, String), pilot_ast::ClearanceLevel>,
    pub fn_visibility: HashMap<String, pilot_ast::VisibilityTier>,
    pub fn_clearance: HashMap<String, pilot_ast::ClearanceLevel>,
    pub current_scope_clearance: u32,
    pub local_fn_captures: HashMap<u32, pilot_ast::LocalFnCaptureSpec>,
}

impl AnalysisResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    pub fn is_ok(&self) -> bool {
        !self.has_errors()
    }
}

pub struct AnalysisPipeline;

impl AnalysisPipeline {
    pub fn analyze(arena: &AstArena, root: NodeId) -> AnalysisResult {
        let mut diagnostics = Vec::new();

        // 1. Bidirectional Type Inference
        let mut infer_engine = TypeInferenceEngine::new();
        if root.is_some() {
            infer_engine.synth(arena, root);
        }
        infer_engine.finalize_inferred_types();

        // 2. Pattern Exhaustiveness & Redundancy Checking
        let loop_analyzer = LoopAnalyzer::new();
        let mut dim_checker = DimensionChecker::new();
        let mut contract_verifier = ContractVerifier::new();

        for i in 0..arena.nodes.len() {
            let node_id = NodeId(i as u32);
            let node = *arena.node(node_id);

            if node.node_kind() == pilot_ast::NodeKind::MatchExpr {
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut all_variants = Vec::new();
                if !children.is_empty() {
                    for &arm_id in &children[1..] {
                        let arm_node = *arena.node(pilot_ast::NodeId(arm_id));
                        let pat_id = pilot_ast::NodeId(arm_node.lhs);
                        if pat_id.is_some() {
                            let mut pat_node = *arena.node(pat_id);
                            if pat_node.node_kind() == pilot_ast::NodeKind::PatBind && pat_node.flags == 1 && pat_node.rhs != 0 {
                                pat_node = *arena.node(pilot_ast::NodeId(pat_node.rhs));
                            }
                            if pat_node.node_kind() == pilot_ast::NodeKind::PatVariant {
                                let name = arena.interner.resolve(pilot_intern::SymbolId(pat_node.lhs));
                                if name == "some" || name == "none" {
                                    all_variants = vec!["some".to_string(), "none".to_string()];
                                    break;
                                }
                                if let Some((adt_name, _)) = infer_engine.adt_variants.get(name) {
                                    for (v, (a, _)) in &infer_engine.adt_variants {
                                        if a == adt_name && !all_variants.contains(v) {
                                            all_variants.push(v.clone());
                                        }
                                    }
                                    break;
                                }
                            } else if pat_node.node_kind() == pilot_ast::NodeKind::PatLiteral {
                                let lit_id = pilot_ast::NodeId(pat_node.lhs);
                                if lit_id.is_some() && arena.node(lit_id).node_kind() == pilot_ast::NodeKind::NoneLit {
                                    all_variants = vec!["some".to_string(), "none".to_string()];
                                    break;
                                }
                            }
                        }
                    }
                }
                let match_diags = MarangetChecker::check_match_expr(arena, node_id, all_variants);
                diagnostics.extend(match_diags);
            }

            dim_checker.check_node(arena, node_id, &infer_engine);
            contract_verifier.check_node(arena, node_id);
        }

        diagnostics.extend(loop_analyzer.diagnostics);
        diagnostics.extend(dim_checker.diagnostics);
        diagnostics.extend(contract_verifier.diagnostics);

        // 6. Homoiconic Call-Quoting Projections
        let mut quoting_engine = crate::quoting::CallQuotingEngine::new();
        for i in 0..arena.nodes.len() {
            let node_id = NodeId(i as u32);
            let node = *arena.node(node_id);
            if node.node_kind() == pilot_ast::NodeKind::CallExpr {
                quoting_engine.project_call(arena, node_id);
            }
        }
        diagnostics.extend(quoting_engine.diagnostics);

        // 7. Comptime Evaluator & Static Demotion Pass
        let mut comptime = ComptimeEvaluator::new();
        comptime.demote_arena(arena, root);
        diagnostics.extend(comptime.diagnostics);

        // 8. 7-Pillar Attribute Weaver & .pchor Cue Boundary Verification
        let mut weaver = AttributeWeaver::new();
        weaver.weave(arena, &mut infer_engine.types);
        diagnostics.extend(weaver.diagnostics);
        diagnostics.extend(infer_engine.diagnostics);

        let local_fn_captures = infer_engine.local_fn_captures.clone();

        AnalysisResult {
            diagnostics,
            types: infer_engine.types,
            inferred_types: infer_engine.inferred_node_types,
            struct_fields: infer_engine.struct_fields,
            struct_member_visibility: infer_engine.struct_member_visibility,
            struct_member_clearance: infer_engine.struct_member_clearance,
            fn_visibility: infer_engine.fn_visibility,
            fn_clearance: infer_engine.fn_clearance,
            current_scope_clearance: infer_engine.current_scope_clearance,
            local_fn_captures,
        }
    }
}

pub struct AttributeWeaver {
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub ecs_rules: Vec<NodeId>,
}

impl AttributeWeaver {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            ecs_rules: Vec::new(),
        }
    }

    pub fn weave(&mut self, arena: &AstArena, types: &mut TypeArena) {
        // Pillar 1: Weight sorting & general attribute dispatch
        for (&node_id, attrs) in &arena.node_attributes {
            let mut sorted_attrs = attrs.clone();
            sorted_attrs.sort_by_key(|a| {
                let mut weight: i32 = 0;
                for &arg in &a.args {
                    let arg_node = arena.node(arg);
                    if arg_node.node_kind() == pilot_ast::NodeKind::IntLit {
                        weight = arena.i64_pool.get(arg_node.lhs as usize).copied().unwrap_or(0) as i32;
                    }
                }
                std::cmp::Reverse(weight)
            });

            for attr in &sorted_attrs {
                let attr_name = arena.interner.resolve(attr.name);
                if let Some(iface_id) = types.interfaces_by_name.get(attr_name) {
                    if *iface_id == crate::types::InterfaceId::IECS_RULE {
                        self.ecs_rules.push(NodeId(node_id));
                    }
                }
            }
        }

        // Gate 7: .pchor cue boundary safety verification
        for &cue_id in &arena.cue_declarations {
            let cue_node = arena.node(NodeId(cue_id));
            if cue_node.node_kind() == pilot_ast::NodeKind::FnDecl {
                let fn_start = cue_node.rhs;
                let fn_len = cue_node.flags as usize;
                if fn_len > 1 {
                    let children = arena.children_slice(fn_start, fn_len as u32);
                    for &param_id in &children[1..] {
                        let param_node = arena.node(NodeId(param_id));
                        if param_node.node_kind() == pilot_ast::NodeKind::ParamDecl {
                            let type_node_id = NodeId(param_node.rhs);
                            if type_node_id.is_some() {
                                let ty_node = arena.node(type_node_id);
                                if ty_node.node_kind() == pilot_ast::NodeKind::TypeNamed {
                                    let ty_name = arena.interner.resolve(pilot_intern::SymbolId(ty_node.lhs));
                                    if let Some(&tid) = types.type_defs_by_name.get(ty_name) {
                                        let lattice = types.type_lattice(tid);
                                        let is_iso = param_node.flags & 1 != 0;
                                        if !lattice.can_cross_cue_boundary(is_iso) {
                                            self.diagnostics.push(AnalysisDiagnostic::error(
                                                "PCHOR001",
                                                format!(
                                                    "Cue boundary violation: parameter of type '{}' has lattice 0b{:02b} (borrow/guard) and cannot cross .pchor cue boundary without iso qualification",
                                                    ty_name, lattice.raw()
                                                ),
                                                arena.span(NodeId(param_id)),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
