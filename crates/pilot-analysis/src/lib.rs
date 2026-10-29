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
pub mod autodiff;
pub mod quoting;
pub mod macros;

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
pub use autodiff::{DualValue, SymbolicDifferentiator, SymbolicExpr};
pub use quoting::{
    AstSpliceResult, CallFrameLayout, CallQuotingEngine, EffectDiff, EffectPlan,
    ReactiveSignalNode, ShallowCallDescriptor,
};
pub use macros::{DecoratorKind, MacroExpander, SynthesizedMember};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
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

    #[test]
    fn test_symbolic_differentiation_compute_impulse() {
        // E = 0.5 * mass * (velocity * velocity)
        let half = SymbolicExpr::Const(0.5);
        let mass = SymbolicExpr::Var("mass".to_string());
        let vel = SymbolicExpr::Var("velocity".to_string());
        let vel2 = SymbolicExpr::Binary("*".to_string(), Box::new(vel.clone()), Box::new(vel.clone()));
        let half_mass = SymbolicExpr::Binary("*".to_string(), Box::new(half), Box::new(mass));
        let impulse = SymbolicExpr::Binary("*".to_string(), Box::new(half_mass), Box::new(vel2));

        let mut env = std::collections::HashMap::new();
        env.insert("mass".to_string(), 10.0);
        env.insert("velocity".to_string(), 5.0);

        let dual = SymbolicDifferentiator::differentiate_function(
            &impulse,
            &["mass", "velocity"],
            &env,
        );

        assert_eq!(dual.value, 125.0);
        assert_eq!(dual.get_gradient("mass"), Some(12.5));
        assert_eq!(dual.get_gradient("velocity"), Some(50.0));
        assert!(dual.cse_count >= 1, "Expected CSE to eliminate (velocity * velocity)");
    }

    #[test]
    fn test_call_quoting_projection_modes() {
        // Mode 1: Shallow
        let shallow = ShallowCallDescriptor {
            fn_name: "ComputeImpulse".to_string(),
            return_type: "f64".to_string(),
            evaluated_args: vec![10.0, 4.0],
            span: ByteSpan::new(0, 10),
        };
        assert_eq!(shallow.eval(), 80.0);

        // Mode 2: Deep Splice
        let mut inlined = HashMap::new();
        inlined.insert("mass".to_string(), 10.0);
        let splice = AstSpliceResult {
            fn_name: "ComputeImpulse".to_string(),
            representation: "Mul(5.0, Mul(Sym(\"v\"), Sym(\"v\")))".to_string(),
            inlined_constants: inlined,
            free_vars: vec!["v".to_string()],
        };
        let mut env = HashMap::new();
        env.insert("v".to_string(), 4.0);
        assert_eq!(splice.eval(&env), 80.0);

        // Mode 3: Call Frame Layout (12 bytes for 2 floats)
        let frame = CallQuotingEngine::synthesize_call_frame(
            "ComputeImpulse",
            &[("mass", 4), ("velocity", 4)],
            vec![10.0, 4.0],
        );
        assert_eq!(frame.size_bytes, 8); // 4 + 4 aligned
        assert_eq!(frame.dispatch(), 80.0);

        // Mode 4: Reactive Signal Node
        let mut sig = ReactiveSignalNode::new("impulseSig", 125.0, 1, vec!["ship.Mass".to_string()]);
        assert_eq!(sig.value, 125.0);
        assert_eq!(sig.rank, 1);
        sig.update(250.0);
        assert_eq!(sig.value, 250.0);
        assert_eq!(sig.eval_count, 2);

        // Mode 5: Transactional Effect Plan
        let plan = EffectPlan {
            value: 52,
            diffs: vec![EffectDiff {
                kind: "StoreField".to_string(),
                target: "rec".to_string(),
                field_name: "Val".to_string(),
                old_value: 10,
                new_value: 42,
            }],
        };
        assert_eq!(plan.diff_count(), 1);
        assert_eq!(plan.commit(), 52);
    }

    #[test]
    fn test_compile_time_decorator_derives() {
        let mut expander = MacroExpander::new();
        let span = ByteSpan::new(0, 10);

        // @builder
        let builder_members = expander.derive_decorator(
            &DecoratorKind::Builder,
            "User",
            &[("name", "str"), ("age", "int")],
            span,
        );
        assert!(builder_members.iter().any(|m| m.name == "with_name"));
        assert!(builder_members.iter().any(|m| m.name == "with_age"));
        assert!(builder_members.iter().any(|m| m.name == "build"));

        // @json
        let json_members = expander.derive_decorator(
            &DecoratorKind::Json,
            "User",
            &[("name", "str")],
            span,
        );
        assert!(json_members.iter().any(|m| m.name == "to_json"));
        assert!(json_members.iter().any(|m| m.name == "from_json"));

        // @observable
        let obs_members = expander.derive_decorator(
            &DecoratorKind::Observable,
            "Ship",
            &[("Mass", "float"), ("Vel", "float")],
            span,
        );
        assert!(obs_members.iter().any(|m| m.name == "notify_Mass_changed"));

        // @equality
        let eq_members = expander.derive_decorator(
            &DecoratorKind::Equality,
            "Point",
            &[("x", "float"), ("y", "float")],
            span,
        );
        assert!(eq_members.iter().any(|m| m.name == "equals"));
        assert!(eq_members.iter().any(|m| m.name == "hash_code"));
    }

    #[test]
    fn test_static_demotion_pass_and_comptime_side_effects() {
        let code = r#"
def calculate(a: int, b: int) -> int {
    let x = a + b
    var y = x * 2
    return y
}
"#;
        let arena = pilot_parser::parse(code);
        let mut comptime = ComptimeEvaluator::new();
        comptime.demote_arena(&arena, arena.root);

        assert!(comptime.demoted_slots.iter().any(|s| s.var_name == "x" && s.is_read_only));
        assert!(comptime.demoted_slots.iter().any(|s| s.var_name == "y" && !s.is_read_only));
        assert!(comptime.diagnostics.is_empty());
    }
}
