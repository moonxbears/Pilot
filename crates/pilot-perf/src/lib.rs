//! Milestone 25 & Milestone 44.1: Multi-Target Profiler, WCET Analyzer & Trivia-Preserving CST Auto-Optimizer (`pilot-perf`).
//!
//! Enforces:
//! - Execution timing, instruction count, and memory allocation profiling across PilVM and native targets.
//! - Chrome Tracing / Speedscope flamegraph export and PGO trace generation (`.pilprof`).
//! - Static WCET analysis and performance diagnostic audit (`PERF001`–`PERF020`).
//! - Trivia-preserving Red-Green CST optimization and unified diff generation.

pub mod profile;
pub mod analyze;
pub mod optimize;

pub use profile::{ExecutionProfiler, ProfileReport, ProfileSample};
pub use analyze::{PerfDiagnostic, StaticWcetAnalyzer};
pub use optimize::CstSourceOptimizer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_reports_and_flamegraph_exports() {
        let code = "def compute() -> i64 {\n    let mut sum: i64 = 0;\n    return sum;\n}\n";
        let report = ExecutionProfiler::profile_source(code, "test.pilot").expect("Profile failed");

        assert_eq!(report.file_path, "test.pilot");
        assert!(report.function_samples.contains_key("main"));

        let chrome_json = report.to_chrome_tracing_json();
        assert!(chrome_json.contains("\"ph\":\"X\""));
        assert!(chrome_json.contains("\"name\":\"main\""));

        let speedscope_json = report.to_speedscope_json();
        assert!(speedscope_json.contains("speedscope.app"));

        let pilprof = report.to_pilprof_trace();
        assert!(pilprof.contains("[hot_functions]"));
    }

    #[test]
    fn test_static_wcet_analyzer_diagnostics() {
        let code = r#"
@hard_realtime
def realtime_task() {
    let list = [1, 2, 3];
    for i in 0..10 {
        for j in 0..10 {
            list.append(i * j);
        }
    }
}
"#;
        let arena = pilot_parser::parse(code);
        let diags = StaticWcetAnalyzer::analyze(code, &arena);

        assert!(diags.iter().any(|d| d.code == "PERF001")); // Nested loop O(N^2)
        assert!(diags.iter().any(|d| d.code == "PERF003")); // Allocation in @hard_realtime
        assert!(diags.iter().any(|d| d.code == "PERF006")); // Append in loop
    }

    #[test]
    fn test_cst_source_optimizer_diff_and_apply() {
        let original = "def calc() {\n    for i in 0..10 {\n        let k = 1024 * 1024;\n    }\n}\n";
        let optimized = CstSourceOptimizer::optimize_source(original);
        assert!(optimized.contains("comptime { 1024 * 1024 }"));

        let diff = CstSourceOptimizer::generate_diff("calc.pilot", original, &optimized);
        assert!(diff.contains("--- a/calc.pilot"));
        assert!(diff.contains("+ "));
    }
}
