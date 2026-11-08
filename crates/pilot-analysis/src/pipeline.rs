//! High-level analysis pipeline delegating semantic & layout passes to Pilot.

use pilot_ast::{AstArena, NodeId};
use crate::diagnostics::{AnalysisDiagnostic, DiagnosticSeverity};
use crate::infer::{analyze_physics, PhysicsRegistry};

pub struct AnalysisResult { pub diagnostics: Vec<AnalysisDiagnostic>, pub physics: PhysicsRegistry }

impl AnalysisResult {
    pub fn has_errors(&self) -> bool { self.diagnostics.iter().any(|d| d.is_error()) }
    pub fn is_ok(&self) -> bool { !self.has_errors() }
}

pub struct AnalysisPipeline;

impl AnalysisPipeline {
    pub fn analyze(arena: &AstArena, root: NodeId) -> AnalysisResult {
        let physics = analyze_physics(arena, root, None);
        let mut diagnostics = Vec::with_capacity(arena.diagnostics.len() + physics.diagnostics.len());
        for (span, msg) in &arena.diagnostics {
            let code = msg.split_once(": ").filter(|(c, _)| !c.is_empty() && c.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')).map_or_else(|| "SyntaxError".to_string(), |(c, _)| c.to_string());
            diagnostics.push(AnalysisDiagnostic { code, message: msg.clone(), span: *span, severity: DiagnosticSeverity::Error });
        }
        for d in &physics.diagnostics {
            let code = d.message.split_once(": ").filter(|(c, _)| !c.is_empty() && c.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')).map_or_else(|| "PhysicsError".to_string(), |(c, _)| c.to_string());
            let severity = match d.severity { 2 => DiagnosticSeverity::Warning, 3 => DiagnosticSeverity::Info, 4 => DiagnosticSeverity::Hint, _ => DiagnosticSeverity::Error };
            diagnostics.push(AnalysisDiagnostic { code, message: d.message.clone(), span: arena.span(NodeId(d.node_id)), severity });
        }
        AnalysisResult { diagnostics, physics }
    }

    pub fn load_program(path: &str) -> (AstArena, PhysicsRegistry) {
        let (src, mut loaded, mut queue, mut dep_srcs) = (std::fs::read_to_string(path).unwrap_or_default(), std::collections::HashSet::new(), Vec::new(), Vec::new());
        let resolve = |spec: &str| -> Option<String> {
            let raw = spec.trim_matches(|c: char| c == ';' || c == '*' || c == '.' || c.is_whitespace()).replace('.', "/");
            let sub = raw.strip_prefix("Pilot/").or_else(|| raw.strip_prefix("pilot/")).unwrap_or(&raw).to_ascii_lowercase();
            [format!("{raw}.pilot"), format!("src/{raw}.pilot"), format!("stdlib/{sub}.pilot"), format!("stdlib/{sub}/{sub}.pilot")].into_iter().find(|p| std::path::Path::new(p).exists())
        };
        let scan = |txt: &str, l: &mut std::collections::HashSet<String>, q: &mut Vec<String>| {
            for line in txt.lines() { if let Some(s) = line.trim().strip_prefix("use ") { if let Some(p) = resolve(s) { if l.insert(p.clone()) { q.push(p); } } } }
        };
        scan(&src, &mut loaded, &mut queue);
        let mut idx = 0;
        while idx < queue.len() {
            let p = queue[idx].clone(); idx += 1;
            if let Ok(c) = std::fs::read_to_string(&p) { scan(&c, &mut loaded, &mut queue); dep_srcs.push(c); }
        }
        let full = if dep_srcs.is_empty() { src } else { format!("{}\n{src}", dep_srcs.join("\n")) };
        let arena = pilot_parser::parse(&full);
        let mut physics = analyze_physics(&arena, arena.root, None);
        for p in &queue { let _ = crate::infer::handle_engine_intrinsic(&mut physics, "mount_unit", &[pilot_rt::PilotVal::from_str(p)]); }
        (arena, physics)
    }
}
