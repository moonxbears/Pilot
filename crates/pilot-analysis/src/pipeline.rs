//! High-level analysis pipeline delegating semantic & layout passes to Pilot.

use pilot_ast::{AstArena, NodeId};
use crate::diagnostics::{AnalysisDiagnostic, DiagnosticSeverity};
use crate::infer::{analyze_physics, PhysicsRegistry};

pub struct AnalysisResult { pub diagnostics: Vec<AnalysisDiagnostic>, pub physics: PhysicsRegistry }

impl AnalysisResult { pub fn has_errors(&self) -> bool { self.diagnostics.iter().any(|d| d.is_error()) } pub fn is_ok(&self) -> bool { !self.has_errors() } }

pub struct AnalysisPipeline;

impl AnalysisPipeline {
    pub fn analyze(arena: &AstArena, root: NodeId) -> AnalysisResult {
        let physics = analyze_physics(arena, root, None);
        let mut diagnostics = Vec::with_capacity(arena.diagnostics.len() + physics.diagnostics.len());
        let parse_code = |m: &str, def: &str| m.split_once(": ").filter(|(c, _)| !c.is_empty() && c.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')).map_or(def, |(c, _)| c).to_string();
        for (span, msg) in &arena.diagnostics { diagnostics.push(AnalysisDiagnostic { code: parse_code(msg, "SyntaxError"), message: msg.clone(), span: *span, severity: DiagnosticSeverity::Error }); }
        for d in &physics.diagnostics {
            let severity = match d.severity { 2 => DiagnosticSeverity::Warning, 3 => DiagnosticSeverity::Info, 4 => DiagnosticSeverity::Hint, _ => DiagnosticSeverity::Error };
            let (span, code) = (arena.span(NodeId(d.node_id)), parse_code(&d.message, "PhysicsError"));
            if !diagnostics.iter().any(|x| x.span == span && x.code == code) { diagnostics.push(AnalysisDiagnostic { code, message: d.message.clone(), span, severity }); }
        }
        AnalysisResult { diagnostics, physics }
    }

    pub fn analyze_source(src: &str) -> AnalysisResult {
        let arena = pilot_parser::parse(src);
        Self::analyze(&arena, arena.root)
    }
}
