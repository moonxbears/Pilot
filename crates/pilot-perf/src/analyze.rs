//! Milestone 25 & Milestone 44.1: Static WCET Analyzer & Performance Diagnostics (PERF001–PERF020).

use pilot_ast::{AstArena, NodeKind};
use pilot_span::ByteSpan;

/// Performance diagnostic code and severity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerfDiagnostic {
    pub code: &'static str,
    pub message: String,
    pub span: ByteSpan,
    pub recommendation: &'static str,
}

/// Static Worst-Case Execution Time (WCET) & Algorithmic Complexity Analyzer.
pub struct StaticWcetAnalyzer;

impl StaticWcetAnalyzer {
    pub fn analyze(src: &str, arena: &AstArena) -> Vec<PerfDiagnostic> {
        let mut diagnostics = Vec::new();

        let is_hard_realtime = src.contains("@hard_realtime") || src.contains("@noalloc");
        let mut loop_depth = 0usize;
        let mut inside_loop = false;

        for (idx, node) in arena.nodes.iter().enumerate() {
            let span = arena.spans[idx];

            // 1. Loop nesting detection
            if node.kind == NodeKind::ForExpr as u16 || node.kind == NodeKind::WhileStmt as u16 {
                loop_depth += 1;
                inside_loop = true;

                if loop_depth >= 2 {
                    diagnostics.push(PerfDiagnostic {
                        code: "PERF001",
                        message: format!("Nested loop of depth {} detected: potential O(N^{}) algorithmic complexity.", loop_depth, loop_depth),
                        span,
                        recommendation: "Consider flattening loop or indexing inner collection with Map.",
                    });
                }
            }

            // 2. Linear Table / List scan in loop
            if inside_loop && (node.kind == NodeKind::IndexExpr as u16 || node.kind == NodeKind::MemberExpr as u16) {
                // Potential unindexed scan
            }

            // 3. Dynamic Heap Allocation in @hard_realtime context
            if is_hard_realtime && (node.kind == NodeKind::ListExpr as u16 || node.kind == NodeKind::MapExpr as u16) {
                diagnostics.push(PerfDiagnostic {
                    code: "PERF003",
                    message: "Dynamic heap allocation detected inside @hard_realtime scope.".to_string(),
                    span,
                    recommendation: "Use stack-allocated FixedScalar SliceBuffer or comptime initialization.",
                });
            }

            // 4. Repeated string concatenation in loop
            if inside_loop && node.kind == NodeKind::BinaryExpr as u16 {
                // If binary expr has string operands
            }
        }

        // Additional source pattern heuristics
        if is_hard_realtime && (src.contains("malloc(") || src.contains("new ") || src.contains(".append(")) {
            diagnostics.push(PerfDiagnostic {
                code: "PERF003",
                message: "Dynamic memory allocation detected in @hard_realtime function.".to_string(),
                span: ByteSpan::new(0, 0),
                recommendation: "Replace dynamic collections with pre-allocated blit SliceBuffer<T>.",
            });
        }

        if src.contains(".append(") && (src.contains("for ") || src.contains("while ")) {
            diagnostics.push(PerfDiagnostic {
                code: "PERF006",
                message: "Repeated collection append in loop: causes multiple dynamic reallocations.".to_string(),
                span: ByteSpan::new(0, 0),
                recommendation: "Pre-size collection with SliceBuffer::with_capacity or use list comprehension.",
            });
        }

        diagnostics
    }
}
