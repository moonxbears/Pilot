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

    pub fn filter_pragma_suppressions(src: &str, diags: &mut Vec<AnalysisDiagnostic>) {
        let active_ns: Vec<String> = src.lines().filter_map(|l| ["pub pkg ", "pkg ", "pub namespace ", "namespace "].into_iter().find_map(|p| l.trim().strip_prefix(p))).flat_map(|r| r.trim_matches(|c: char| c == '{' || c == ';' || c.is_whitespace()).split(',')).map(|s| s.trim().trim_matches(|c: char| c == ';' || c.is_whitespace()).to_string()).filter(|s| !s.is_empty()).collect();
        let (mut cur, mut intervals, mut starts) = (0usize, Vec::new(), std::collections::HashMap::new());
        for line in src.split_inclusive('\n') {
            let tr = line.trim();
            let (target_ns, raw_cmd) = if let Some(r) = tr.strip_prefix("#pragma namespace ") { r.split_once(':').map(|(n, a)| (Some(n.trim()), a.trim())).unwrap_or((None, r.trim())) } else { (None, tr) };
            let pragma_cmd = raw_cmd.strip_prefix("#pragma ").unwrap_or(raw_cmd);
            let (is_dis, r) = if let Some(r) = pragma_cmd.strip_prefix("warning disable") { (Some(true), r) }
                else if let Some(r) = pragma_cmd.strip_prefix("warning restore") { (Some(false), r) }
                else if let Some(_) = pragma_cmd.strip_prefix("no_hardcode disable") { (Some(true), " HARD001") }
                else if let Some(_) = pragma_cmd.strip_prefix("no_hardcode restore") { (Some(false), " HARD001") } else { (None, "") };
            if let Some(dis) = is_dis {
                let (codes, t_ns) = if let Some((c, n)) = r.trim().split_once(" in ") { (c.trim(), Some(n.trim())) } else { (r.trim(), target_ns) };
                let ns_match = t_ns.map_or(true, |t| t.split(',').any(|n| n.trim() == "*" || active_ns.iter().any(|ns| ns == n.trim() || ns.starts_with(n.trim()))));
                if ns_match {
                    let mut apply = |k: Option<String>| if dis { starts.entry(k).or_insert(cur); } else if let Some(st) = starts.remove(&k) { intervals.push((st, cur + line.len(), k)); };
                    if codes.is_empty() { apply(None); } else { for c in codes.split(',') { let cd = c.trim().to_string(); if !cd.is_empty() { apply(Some(cd)); } } }
                } }
            cur += line.len();
        }
        for (k, st) in starts { intervals.push((st, cur, k)); }
        diags.retain(|d| !intervals.iter().any(|(st, en, c)| (d.span.start as usize) >= *st && (d.span.start as usize) < *en && c.as_ref().map_or(!d.is_error(), |code| code == &d.code)));
    }

    pub fn analyze_source(src: &str) -> AnalysisResult {
        let arena = pilot_parser::parse(src);
        let mut res = Self::analyze(&arena, arena.root);
        Self::filter_pragma_suppressions(src, &mut res.diagnostics); res
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
        while idx < queue.len() { let p = queue[idx].clone(); idx += 1; if let Ok(c) = std::fs::read_to_string(&p) { scan(&c, &mut loaded, &mut queue); dep_srcs.push(c); } }
        let full = if dep_srcs.is_empty() { src } else { format!("{}\n{src}", dep_srcs.join("\n")) };
        let arena = pilot_parser::parse(&full);
        let mut physics = analyze_physics(&arena, arena.root, None);
        for p in &queue { let _ = crate::infer::handle_engine_intrinsic(&mut physics, "mount_unit", &[pilot_rt::PilotVal::from_str(p)]); }
        (arena, physics)
    }
}
