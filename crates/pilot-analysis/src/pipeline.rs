//! High-level coordinator for Milestone 33 static intelligence passes.

use std::collections::HashMap;
use pilot_ast::{AstArena, NodeId};
use crate::comptime::ComptimeEvaluator;
use crate::contracts::ContractVerifier;
use crate::diagnostics::AnalysisDiagnostic;
use crate::dimensions::DimensionChecker;
use crate::infer::TypeInferenceEngine;
use crate::loops::LoopAnalyzer;
use crate::null_error::{ErrorFlowAnalyzer, NullSafetyAnalyzer};
use crate::patterns::MarangetChecker;
use crate::types::{TypeArena, TypeId};

pub struct AnalysisResult {
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub types: TypeArena,
    pub inferred_types: HashMap<u32, TypeId>,
    pub struct_fields: HashMap<String, Vec<(String, TypeId)>>,
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
        let mut null_safety = NullSafetyAnalyzer::new();
        let mut error_flow = ErrorFlowAnalyzer::new();
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

            null_safety.check_node(arena, node_id, &infer_engine);
            error_flow.check_node(arena, node_id, &infer_engine);
            dim_checker.check_node(arena, node_id, &infer_engine);
            contract_verifier.check_node(arena, node_id);
        }

        diagnostics.extend(null_safety.diagnostics);
        diagnostics.extend(error_flow.diagnostics);
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
        diagnostics.extend(infer_engine.diagnostics);

        AnalysisResult {
            diagnostics,
            types: infer_engine.types,
            inferred_types: infer_engine.inferred_node_types,
            struct_fields: infer_engine.struct_fields,
        }
    }
}
