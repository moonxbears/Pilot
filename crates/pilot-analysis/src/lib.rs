//! Milestone 33: Native Static Analysis, Bidirectional Inference, Perceus Escape Analysis & `comptime` (`pilot-analysis`)
//!
//! Enforces:
//! - 33.1: Data-Oriented `TypeArena` & Bidirectional + Row-Polymorphic Inferencer
//! - 33.2: Pattern Exhaustiveness (`M11`), Null/Error Flow (`M12`–`M13`), Loop (`M15`), Dimension & Contract Verifiers
//! - 33.3: Call-Quoting Projections (`M17`–`M18`) & Static Demotion (`M19`)

pub mod diagnostics;
pub mod types;
pub mod infer;
pub mod patterns;
pub mod null_error;
pub mod loops;
pub mod contracts;
pub mod dimensions;
pub mod comptime;
pub mod pipeline;

pub use diagnostics::{AnalysisDiagnostic, DiagnosticSeverity};
pub use types::{TypeArena, TypeId, TypeKind};
pub use infer::TypeInferenceEngine;
pub use patterns::{MarangetChecker, PatternCase};
pub use null_error::{ErrorFlowAnalyzer, NullSafetyAnalyzer};
pub use loops::LoopAnalyzer;
pub use contracts::{ContractVerifier, Interval};
pub use dimensions::{DimensionChecker, DimensionVector};
pub use comptime::{CallFrameLayoutInfo, ComptimeEvaluator, QuoteKind, StaticDemotedSlot};
pub use pipeline::{AnalysisPipeline, AnalysisResult};

#[cfg(test)]
mod tests {
    use super::*;
    use pilot_span::ByteSpan;

    #[test]
    fn test_type_arena_interning_and_equality() {
        let mut arena = TypeArena::new();

        // 1. Primitive singletons are pre-interned and strictly equal by 32-bit ID
        let i64_1 = TypeId::I64;
        let i64_2 = arena.alloc(TypeKind::Primitive("i64".to_string()));
        assert_eq!(i64_1, i64_2);
        assert_eq!(i64_1.0, 5);

        // 2. Complex types are interned and deduplicated
        let list1 = arena.list_of(TypeId::I64);
        let list2 = arena.list_of(TypeId::I64);
        assert_eq!(list1, list2);

        // 3. Option type
        let opt1 = arena.option_of(TypeId::STR);
        let opt2 = arena.option_of(TypeId::STR);
        assert_eq!(opt1, opt2);
        assert_eq!(arena.display(opt1), "str?");
    }

    #[test]
    fn test_structural_row_polymorphism_and_monomorphization() {
        let mut arena = TypeArena::new();

        // Open row record: { x: i64, .. }
        let open_row = arena.row_record_of(
            vec![("x".to_string(), TypeId::I64)],
            true,
            Some(1),
        );

        // Required concrete fields: x: i64, y: str
        let monomorphized = arena
            .monomorphize_row(
                open_row,
                &[
                    ("x".to_string(), TypeId::I64),
                    ("y".to_string(), TypeId::STR),
                ],
            )
            .unwrap();

        let display_str = arena.display(monomorphized);
        assert_eq!(display_str, "{ x: i64, y: str }");
    }

    #[test]
    fn test_maranget_pattern_exhaustiveness_and_redundancy() {
        let mut checker = MarangetChecker::new(vec![
            "Ok".to_string(),
            "Err".to_string(),
        ]);

        let dummy_span = ByteSpan::new(0, 10);

        // Add arm for "Ok"
        checker.add_arm(PatternCase::Variant("Ok".to_string()), false, dummy_span);

        // Missing "Err" -> MATCH001
        let diags = checker.check(dummy_span);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "MATCH001");
        assert!(diags[0].message.contains("missing cases [Err]"));

        // Add arm for "Err", plus duplicate "Err"
        checker.add_arm(PatternCase::Variant("Err".to_string()), false, dummy_span);
        checker.add_arm(PatternCase::Variant("Err".to_string()), false, dummy_span);

        let diags2 = checker.check(dummy_span);
        // Should have redundant arm warning MATCH002
        assert_eq!(diags2.len(), 1);
        assert_eq!(diags2[0].code, "MATCH002");
    }

    #[test]
    fn test_call_frame_layout_synthesis() {
        let layout = ComptimeEvaluator::compute_call_frame_layout(
            "calculate",
            &[("a", 8), ("b", 8), ("flag", 1)],
        );
        assert_eq!(layout.fn_name, "calculate");
        assert_eq!(layout.param_offsets.len(), 3);
        // a at 0 (size 8), b at 8 (size 8), flag at 16 (size 1) -> total aligned to 8 = 24
        assert_eq!(layout.param_offsets[0], ("a".to_string(), 0, 8));
        assert_eq!(layout.param_offsets[1], ("b".to_string(), 8, 8));
        assert_eq!(layout.param_offsets[2], ("flag".to_string(), 16, 1));
        assert_eq!(layout.total_size_bytes, 24);
    }

    #[test]
    fn test_infer_full_ast_pipeline_clean() {
        let code = r#"
def fib(n: int) -> int {
    if n <= 1 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}
let res = fib(10)
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(result.is_ok(), "Expected clean analysis, got: {:?}", result.diagnostics);
        assert_eq!(result.diagnostics.len(), 0);
    }

    #[test]
    fn test_infer_type_mismatch_diagnostics() {
        let code = r#"
let a: int = 10
let b: str = "hello"
let c = a + b
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(!result.is_ok());
        let type_errors: Vec<_> = result.diagnostics.iter().filter(|d| d.code == "TYPE002").collect();
        assert!(!type_errors.is_empty(), "Expected TYPE002 diagnostic for a + b");
    }

    #[test]
    fn test_infer_unresolved_identifier_diagnostics() {
        let code = r#"
let y = undeclared_variable + 5
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(!result.is_ok());
        let unres_errors: Vec<_> = result.diagnostics.iter().filter(|d| d.code == "TYPE001").collect();
        assert!(!unres_errors.is_empty(), "Expected TYPE001 diagnostic for undeclared_variable");
    }

    #[test]
    fn test_contract_verifier_division_by_zero() {
        let code = r#"
def div_test(x: int) -> int {
    return x / 0
}
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(!result.is_ok());
        let div_errors: Vec<_> = result.diagnostics.iter().filter(|d| d.code == "DIV001").collect();
        assert!(!div_errors.is_empty(), "Expected DIV001 static division by zero error");
    }

    #[test]
    fn test_pattern_exhaustiveness_ast_walk() {
        let code_exhaustive = r#"
def test_match(x: int) -> int {
    match x {
        case 0 { return 10 }
        case _ { return 20 }
    }
}
"#;
        let arena1 = pilot_parser::parse(code_exhaustive);
        let res1 = AnalysisPipeline::analyze(&arena1, arena1.root);
        let match_diags: Vec<_> = res1.diagnostics.iter().filter(|d| d.code.starts_with("MATCH")).collect();
        assert!(match_diags.is_empty(), "Expected exhaustive match, got: {:?}", match_diags);

        let code_non_exhaustive = r#"
def test_match_incomplete(x: int) -> int {
    match x {
        case 0 { return 10 }
    }
}
"#;
        let arena2 = pilot_parser::parse(code_non_exhaustive);
        let res2 = AnalysisPipeline::analyze(&arena2, arena2.root);
        let match_errors: Vec<_> = res2.diagnostics.iter().filter(|d| d.code == "MATCH001").collect();
        assert!(!match_errors.is_empty(), "Expected MATCH001 for match without default/wildcard");
    }
}
