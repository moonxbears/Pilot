//! High-level coordinator for Milestone 33 static intelligence passes.

use pilot_ast::{AstArena, NodeId};
use crate::diagnostics::AnalysisDiagnostic;
use crate::infer::{analyze_physics, PhysicsRegistry};

pub struct AnalysisResult {
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub physics: PhysicsRegistry,
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
        let physics = analyze_physics(arena, root, None);
        let mut diagnostics = Vec::new();
        for d in &physics.diagnostics {
            diagnostics.push(AnalysisDiagnostic {
                code: "PhysicsError".to_string(),
                message: d.message.clone(),
                span: pilot_span::ByteSpan::new(0, 0),
                severity: crate::diagnostics::DiagnosticSeverity::Error,
            });
        }
        AnalysisResult {
            diagnostics,
            physics,
        }
    }
}

pub struct AttributeWeaver {
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub ecs_rules: Vec<NodeId>,
}

impl Default for AttributeWeaver {
    fn default() -> Self {
        Self::new()
    }
}

impl AttributeWeaver {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            ecs_rules: Vec::new(),
        }
    }

    pub fn weave(&mut self, _arena: &AstArena) {}
}
