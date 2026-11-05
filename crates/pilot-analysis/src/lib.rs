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

pub use diagnostics::{
    verify_core_naming_rules, AnalysisDiagnostic, Diagnostic, DiagnosticSeverity, PackageAst,
};
pub use types::{
    classify_perceus_action, BitSet, BundleDef, BundleId, CapabilityBits, FieldDef, FieldOffset,
    FxHashMap, FxHashSet, HardwarePhysics, InterfaceDef, InterfaceId, LatticeBits, LayoutId,
    LayoutTable, MethodEntry, MethodSignature, MethodTier, OriginId, OwnershipAction, PhysicalLayout,
    PhysicsGeometry, RegClass, SpliceDirectives, StructuralShape, TypeArena, TypeDef, TypeId, TypeKind,
};
pub use infer::{
    analyze_physics, clone_and_rewrite_self_field_accesses, flatten_ast, handle_engine_intrinsic,
    rewrite_self_field_accesses, run_pilot_analyzer, NodeLayoutMap, PhysicsDiagnostic, PhysicsRegistry,
    RawAstArena, RawAstNode, TypeInferenceEngine,
};
pub use patterns::{MarangetChecker, PatternCase};
pub use null_error::{
    ErrorFlowAnalyzer, ImplicitLiftKind, NullSafetyAnalyzer, TryPropagationPlan,
};
pub use loops::{
    ForLoopAnalyzer, ForLoweringStrategy, LoopAnalyzer, LoopReturnMode, LoopScheduleMode,
};
pub use contracts::{ContractVerifier, Interval};
pub use dimensions::{DimensionChecker, DimensionVector};
pub use comptime::{
    bootstrap_core_file, eval_file_with_tables, BootstrapDiagnostic, BootstrapSnapshot,
    CallFrameLayoutInfo, ComptimeEngine, ComptimeEvaluator, ComptimeFnDef, ComptimeValue,
    InlineHint, LangItem, MemoryLayoutMode, QuoteKind, RegisterClass, StaticDemotedSlot,
    TypeLayoutSpec,
};
pub use pipeline::{AnalysisPipeline, AnalysisResult, AttributeWeaver};
pub use autodiff::{DualValue, SymbolicDifferentiator, SymbolicExpr};
pub use quoting::{
    AstSpliceResult, CallFrameLayout, CallQuoteProjectionInfo, CallQuotingEngine, EffectDiff,
    EffectPlan, ReactiveSignalNode, ShallowCallDescriptor,
};
pub use macros::{
    desugar_bracket_attributes, expand_syntax_macro, splice_quote_template, DecoratorKind,
    MacroExpander, SynthesizedMember,
};

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
fn fib(n: i64) -> i64 {
    if n <= 1 {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}
let res = fib(10);
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(result.is_ok(), "Expected clean analysis, got: {:?}", result.diagnostics);
        assert_eq!(result.diagnostics.len(), 0);
    }

    #[test]
    fn test_infer_type_mismatch_diagnostics() {
        let code = r#"
let a: i64 = 10;
let b: str = "hello";
let c = a + b;
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
let y = undeclared_variable + 5;
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
fn div_test(x: i64) -> i64 {
    return x / 0;
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
fn test_match(x: i64) -> i64 {
    switch x {
        case 0: { return 10; }
        case _: { return 20; }
    }
}
"#;
        let arena1 = pilot_parser::parse(code_exhaustive);
        let res1 = AnalysisPipeline::analyze(&arena1, arena1.root);
        let match_diags: Vec<_> = res1.diagnostics.iter().filter(|d| d.code.starts_with("MATCH")).collect();
        assert!(match_diags.is_empty(), "Expected exhaustive match, got: {:?}", match_diags);

        let code_non_exhaustive = r#"
fn test_match_incomplete(x: i64) -> i64 {
    switch x {
        case 0: { return 10; }
    }
}
"#;
        let arena2 = pilot_parser::parse(code_non_exhaustive);
        let res2 = AnalysisPipeline::analyze(&arena2, arena2.root);
        let match_errors: Vec<_> = res2.diagnostics.iter().filter(|d| d.code == "MATCH001").collect();
        assert!(!match_errors.is_empty(), "Expected MATCH001 for match without default/wildcard");
    }

    #[test]
    fn test_row_polymorphism_inference() {
        let code = r#"
record Frame2D(width: int, height: int, name: str);
record Canvas2D(width: float, height: float);

fn compute_area(surface) => surface.width * surface.height;

let frame_area = compute_area(Frame2D(1920, 1080, "HD"));
let canvas_area = compute_area(Canvas2D(12.5, 8.0));
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(result.is_ok(), "Expected clean analysis: {:?}", result.diagnostics);

        let mut frame_area_ty = None;
        let mut canvas_area_ty = None;
        let mut surface_ty = None;
        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == pilot_ast::NodeKind::LetStmt {
                let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                if name == "frame_area" {
                    frame_area_ty = result.inferred_types.get(&(idx as u32)).copied();
                } else if name == "canvas_area" {
                    canvas_area_ty = result.inferred_types.get(&(idx as u32)).copied();
                }
            } else if node.node_kind() == pilot_ast::NodeKind::ParamDecl {
                let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                if name == "surface" {
                    surface_ty = result.inferred_types.get(&(idx as u32)).copied();
                }
            }
        }

        assert_eq!(frame_area_ty, Some(TypeId::I64), "frame_area should infer to int (i64)");
        assert_eq!(canvas_area_ty, Some(TypeId::F64), "canvas_area should infer to float (f64)");
        assert!(surface_ty.is_some(), "surface param type should be inferred");
        let surf_disp = result.types.display(surface_ty.unwrap());
        assert!(surf_disp.contains("width") && surf_disp.contains("height"), "surface should infer row record: {}", surf_disp);
    }

    #[test]
    fn test_heterogeneous_collection_union_inference() {
        let code = r#"
let mylist = ["apples", 5, 2];
let first_elem = mylist[0];
let mymap = {"name": "Pilot", "count": 42};
let branch_val = if true { "ok" } else { 404 };
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(result.is_ok(), "Expected clean analysis: {:?}", result.diagnostics);

        let mut mylist_ty = None;
        let mut first_elem_ty = None;
        let mut mymap_ty = None;
        let mut branch_val_ty = None;

        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == pilot_ast::NodeKind::LetStmt {
                let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                match name {
                    "mylist" => mylist_ty = result.inferred_types.get(&(idx as u32)).copied(),
                    "first_elem" => first_elem_ty = result.inferred_types.get(&(idx as u32)).copied(),
                    "mymap" => mymap_ty = result.inferred_types.get(&(idx as u32)).copied(),
                    "branch_val" => branch_val_ty = result.inferred_types.get(&(idx as u32)).copied(),
                    _ => {}
                }
            }
        }

        assert!(mylist_ty.is_some(), "mylist should be inferred");
        let mylist_disp = result.types.display(mylist_ty.unwrap());
        println!("mylist inferred type: {}", mylist_disp);
        assert!(mylist_disp.to_lowercase().contains("list[") && mylist_disp.contains("str") && mylist_disp.contains("i64"),
            "Expected List[str | i64] (or int), got: {}", mylist_disp);

        assert!(first_elem_ty.is_some(), "first_elem should be inferred");
        let first_disp = result.types.display(first_elem_ty.unwrap());
        println!("first_elem inferred type: {}", first_disp);
        assert!(first_disp.contains("str") && first_disp.contains("i64"),
            "Expected str | i64, got: {}", first_disp);

        assert!(mymap_ty.is_some(), "mymap should be inferred");
        let mymap_disp = result.types.display(mymap_ty.unwrap());
        println!("mymap inferred type: {}", mymap_disp);
        assert!(mymap_disp.contains("name: str") && mymap_disp.contains("count: i64"),
            "Expected {{ count: i64, name: str }}, got: {}", mymap_disp);

        assert!(branch_val_ty.is_some(), "branch_val should be inferred");
        let branch_disp = result.types.display(branch_val_ty.unwrap());
        println!("branch_val inferred type: {}", branch_disp);
        assert!(branch_disp.contains("str") && branch_disp.contains("i64"),
            "Expected str | i64, got: {}", branch_disp);
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
fn calculate(a: i64, b: i64) -> i64 {
    let x = a + b;
    var y = x * 2;
    return y;
}
"#;
        let arena = pilot_parser::parse(code);
        let mut comptime = ComptimeEvaluator::new();
        comptime.demote_arena(&arena, arena.root);

        assert!(comptime.demoted_slots.iter().any(|s| s.var_name == "x" && s.is_read_only));
        assert!(comptime.demoted_slots.iter().any(|s| s.var_name == "y" && !s.is_read_only));
        assert!(comptime.diagnostics.is_empty());
    }

    #[test]
    fn test_custom_indexer_inference() {
        let code = r#"
class MemoryCache {
    self[key: str] -> int {
        get => 42;
        set(v) => {};
    }
}

class Matrix {
    self[row: int, col: int] -> float => 3.14;
}

class SimpleBuffer {
    self[idx: int] -> str => "item";
}

let cache = MemoryCache();
let item = cache["hello"];
cache["test"] = 100;

let mat = Matrix();
let val = mat[1, 2];

let buf = SimpleBuffer();
let entry = buf[0];
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(result.is_ok(), "Expected clean analysis, got: {:?}", result.diagnostics);

        // Find `item` node and assert inferred type is int
        let mut item_ty = None;
        let mut val_ty = None;
        let mut entry_ty = None;

        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == pilot_ast::NodeKind::LetStmt {
                let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                if name == "item" {
                    item_ty = result.inferred_types.get(&(idx as u32)).copied();
                } else if name == "val" {
                    val_ty = result.inferred_types.get(&(idx as u32)).copied();
                } else if name == "entry" {
                    entry_ty = result.inferred_types.get(&(idx as u32)).copied();
                }
            }
        }

        assert_eq!(item_ty, Some(TypeId::I64));
        assert_eq!(val_ty, Some(TypeId::F64));
        assert_eq!(entry_ty, Some(TypeId::STR));
    }

    #[test]
    fn test_indexer_diagnostics() {
        // Test type mismatch in indexer argument
        let code_mismatch = r#"
class Cache {
    self[key: str] -> int => 0;
}
let c = Cache();
let bad = c[123];
"#;
        let arena_m = pilot_parser::parse(code_mismatch);
        let res_m = AnalysisPipeline::analyze(&arena_m, arena_m.root);
        assert!(!res_m.is_ok(), "Diagnostics: {:?}", res_m.diagnostics);
        assert!(res_m.diagnostics.iter().any(|d| d.code == "TYPE001"));

        // Test argument count mismatch in multi-dimensional indexer
        let code_count = r#"
class Grid {
    self[r: int, c: int] -> float => 1.0;
}
let g = Grid();
let bad = g[5];
"#;
        let arena_c = pilot_parser::parse(code_count);
        let res_c = AnalysisPipeline::analyze(&arena_c, arena_c.root);
        assert!(!res_c.is_ok());
        assert!(res_c.diagnostics.iter().any(|d| d.code == "IDX001"));
    }

    #[test]
    fn test_relational_record_indexing_and_collection_append_inference() {
        let code = r#"
let transactions = [
    { "id": 1, "customer": "Alice", "amount": 250.0, "category": "Hardware" },
    { "id": 2, "customer": "Bob",   "amount": 420.0, "category": "Software" }
];
let hardware_txns = [];
var total_hardware_spend = 0.0;

for t in transactions {
    if t["category"] == "Hardware" {
        hardware_txns.append(t);
        total_hardware_spend = total_hardware_spend + t["amount"];
    }
}
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(result.is_ok(), "Expected clean analysis, got: {:?}", result.diagnostics);

        let mut hw_txns_ty = None;
        let mut total_spend_ty = None;

        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == pilot_ast::NodeKind::LetStmt {
                let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                if name == "hardware_txns" {
                    hw_txns_ty = result.inferred_types.get(&(idx as u32)).copied();
                }
            } else if node.node_kind() == pilot_ast::NodeKind::VarStmt {
                let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                if name == "total_hardware_spend" {
                    total_spend_ty = result.inferred_types.get(&(idx as u32)).copied();
                }
            }
        }

        assert_eq!(total_spend_ty, Some(TypeId::F64));
        assert!(hw_txns_ty.is_some());
        let hw_ty = hw_txns_ty.unwrap();
        let hw_display = result.types.display(hw_ty);
        assert!(!hw_display.contains("'T"), "Expected concrete inferred type without unconstrained type variables, got: {}", hw_display);
        assert!(hw_display.to_lowercase().contains("list["), "Expected List type, got: {}", hw_display);
        assert!(hw_display.contains("amount: f64"), "Expected amount: f64, got: {}", hw_display);
        assert!(hw_display.contains("category: str"), "Expected category: str, got: {}", hw_display);
    }

    #[test]
    fn test_compile_time_sql_query_type_inference() {
        let code = r#"
let psql_records = postgresql {
    select id, name, count(*) as total_count, amount * 1.1 as adjusted_amount
    from system_metrics tb
    where tb.id in (1, 2, 3, 4, 5);
}

for row in psql_records {
    let r_id = row.id;
    let r_name = row.name;
    let r_total = row.total_count;
    let r_adj = row.adjusted_amount;
}
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(result.is_ok(), "Expected clean analysis, got: {:?}", result.diagnostics);

        let mut psql_ty = None;
        let mut r_id_ty = None;
        let mut r_name_ty = None;
        let mut r_total_ty = None;
        let mut r_adj_ty = None;

        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == pilot_ast::NodeKind::LetStmt {
                let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                match name {
                    "psql_records" => psql_ty = result.inferred_types.get(&(idx as u32)).copied(),
                    "r_id" => r_id_ty = result.inferred_types.get(&(idx as u32)).copied(),
                    "r_name" => r_name_ty = result.inferred_types.get(&(idx as u32)).copied(),
                    "r_total" => r_total_ty = result.inferred_types.get(&(idx as u32)).copied(),
                    "r_adj" => r_adj_ty = result.inferred_types.get(&(idx as u32)).copied(),
                    _ => {}
                }
            }
        }

        assert_eq!(r_id_ty, Some(TypeId::I64));
        assert_eq!(r_name_ty, Some(TypeId::STR));
        assert_eq!(r_total_ty, Some(TypeId::I64));
        assert_eq!(r_adj_ty, Some(TypeId::F64));

        let psql_display = result.types.display(psql_ty.unwrap());
        assert!(psql_display.contains("id: i64"));
        assert!(psql_display.contains("name: str"));
        assert!(psql_display.contains("total_count: i64"));
        assert!(psql_display.contains("adjusted_amount: f64"));

        // Test parameterized connection syntax: sql(conn) { ... }
        let conn_code = r#"
let pg_db = "postgres://localhost:5432/app";
let emp_records = sql(pg_db) {
    select emp_id, emp_name, salary from staff;
};

for e in emp_records {
    let eid = e.emp_id;
    let ename = e.emp_name;
    let esal = e.salary;
}
"#;
        let arena2 = pilot_parser::parse(conn_code);
        let result2 = AnalysisPipeline::analyze(&arena2, arena2.root);
        assert!(result2.is_ok(), "Expected clean analysis for parameterized sql(conn), got: {:?}", result2.diagnostics);
        assert_eq!(arena2.polyglot_conns.len(), 1);
    }

    #[test]
    fn test_nominal_type_display_vs_structural_representation() {
        let code = r#"
struct OddHeader {
    var tag: u8
    var flags: u8
    var pad: u8
}

struct TransformFrame {
    var x: i32
    var y: i32
    var z: i32
    var weight: i64
    var mask: u16
    var hop: u8
    var pad: u8
}

fn step_frame(var f: TransformFrame, h: OddHeader, delta: i32) -> TransformFrame {
    f.x = f.x + delta;
    return f;
}

let f0 = TransformFrame(10, 20, 0x55, 7, 3, 1, 0);
let h0 = OddHeader(3, 0x0F, 0xAA);
let f1 = step_frame(f0, h0, 4);
var arena: TransformFrame[] = [f0, f1];
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(result.is_ok(), "Expected clean analysis: {:?}", result.diagnostics);

        let mut f0_ty = None;
        let mut h0_ty = None;
        let mut f1_ty = None;
        let mut arena_ty = None;
        let mut step_frame_ty = None;

        for (idx, node) in arena.nodes.iter().enumerate() {
            match node.node_kind() {
                pilot_ast::NodeKind::LetStmt | pilot_ast::NodeKind::VarStmt => {
                    let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                    match name {
                        "f0" => f0_ty = result.inferred_types.get(&(idx as u32)).copied(),
                        "h0" => h0_ty = result.inferred_types.get(&(idx as u32)).copied(),
                        "f1" => f1_ty = result.inferred_types.get(&(idx as u32)).copied(),
                        "arena" => arena_ty = result.inferred_types.get(&(idx as u32)).copied(),
                        _ => {}
                    }
                }
                pilot_ast::NodeKind::FnDecl => {
                    let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                    if name == "step_frame" {
                        step_frame_ty = result.inferred_types.get(&(idx as u32)).copied();
                    }
                }
                _ => {}
            }
        }

        assert!(f0_ty.is_some());
        assert_eq!(result.types.display(f0_ty.unwrap()), "TransformFrame");
        assert_eq!(result.types.display(h0_ty.unwrap()), "OddHeader");
        assert_eq!(result.types.display(f1_ty.unwrap()), "TransformFrame");
        let disp_arena = result.types.display(arena_ty.unwrap());
        assert!(disp_arena == "slice[TransformFrame]" || disp_arena == "Slice[TransformFrame]");

        let sf_str = result.types.display(step_frame_ty.unwrap());
        assert_eq!(sf_str, "(TransformFrame, OddHeader, i32) -> TransformFrame");

        // Verify structural display returns full dictionary makeup
        let structural = result.types.display_structural(f0_ty.unwrap());
        assert!(structural.contains("weight: i64"));
        assert!(structural.contains("mask: i64") || structural.contains("mask: u16"));
        assert!(structural.starts_with("{ "));
        assert!(structural.ends_with(" }"));
    }

    #[test]
    fn test_relational_queries_and_acid_transactions() {
        let code = r#"
fn test_queries() {
    let users_table = [{"name": "Alice", "role": "admin", "active": true}];
    let active_admins = from u in users_table
        where u["active"] == true and u["role"] == "admin"
        order by u["name"] ascending
        select u["name"];

    let engineer_names = select u["name"] from users_table as u where u["role"] == "engineer";

    let combined_users = active_admins union all engineer_names;
    let intersect_users = active_admins intersect engineer_names;
    let except_users = active_admins except engineer_names;

    database app_db = connect("memory://");
    transaction {
        savepoint sp1;
        commit;
        rollback to savepoint sp1;
    }
}
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(result.is_ok(), "Expected clean analysis for relational queries and acid transaction, got: {:?}", result.diagnostics);
    }

    #[test]
    fn test_builtin_type_methods_inference() {
        let code = r#"
fn test_builtins() {
    let s = "hello,world";
    let parts = s.split(",");
    let joined = parts.join("-");
    let rep = s.replace("world", "pilot");
    let trimmed = s.trim();
    let is_emp = s.is_empty();
    let has_lo = s.contains("lo");
    let idx = s.index_of("world");

    let num = -42;
    let abs_val = num.abs();
    let clamped = num.clamp(0, 100);
    let s_num = num.to_string();

    let f = 3.14;
    let r = f.round();
    let fl = f.floor();
}
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(result.is_ok(), "Expected clean analysis for built-in methods, got: {:?}", result.diagnostics);
    }

    #[test]
    fn test_comprehensive_syntax_clean_analysis() {
        let src = r#"
            record Point(x: int, y: int);
            record Config(timeout: float, retry: bool);

            fn calculate(p: Point, c: Config) -> float {
                if c.retry {
                    return (p.x + p.y) as float * c.timeout;
                }
                return 0.0;
            }

            let p = Point(10, 20);
            let c = Config(1.5, true);
            let result = calculate(p, c);
        "#;
        let arena = pilot_parser::parse(src);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        let errors: Vec<_> = result.diagnostics.iter().filter(|d| d.severity == DiagnosticSeverity::Error).collect();
        assert!(errors.is_empty(), "Expected 0 errors on clean syntax analysis, got: {:?}", errors);
    }

    #[test]
    fn test_string_range_type_inference() {
        let code = r#"
fn check_ranges() {
    let forward = "a".."z";
    let reverse = "z".."a";
    let mixed = "a".."Z";
    let big = "a".."zzza";
    for ch in "a".."z" {
        let upper = ch.to_upper();
    }
}
"#;
        let arena = pilot_parser::parse(code);
        let result = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(result.is_ok(), "Expected clean string range analysis, got: {:?}", result.diagnostics);
    }

    #[test]
    fn test_static_methods_contextual_enums_and_deep_flatten() {
        // 1. Static fn rejecting self / .field
        let bad_static = r#"
        class Service {
            x: int;
            static fn compute() -> int {
                return .x;
            }
        }
        "#;
        let arena_bad = pilot_parser::parse(bad_static);
        let result_bad = AnalysisPipeline::analyze(&arena_bad, arena_bad.root);
        assert!(!result_bad.is_ok(), "Expected static method error");
        assert!(result_bad.diagnostics.iter().any(|d| d.code == "STATIC001"), "Expected STATIC001, got: {:?}", result_bad.diagnostics);

        // 2. Valid static fn and instance fn accessing fields
        let good_code = r#"
        class Vector2D {
            x: int;
            y: int;

            fn get_sum() -> int {
                return .x + .y;
            }

            static fn origin() -> int {
                return 0;
            }
        }
        "#;
        let arena_good = pilot_parser::parse(good_code);
        let result_good = AnalysisPipeline::analyze(&arena_good, arena_good.root);
        assert!(result_good.is_ok(), "Expected clean analysis, got: {:?}", result_good.diagnostics);

        // 3. Comptime deep flattening literal helper
        let nested_code = r#"
        let nested = [1, [2, [3, 4]], 5];
        "#;
        let arena_nested = pilot_parser::parse(nested_code);
        let flat_leaves = crate::comptime::ComptimeEvaluator::flatten_literal_to_1d(&arena_nested, arena_nested.root);
        assert!(flat_leaves.len() >= 5, "Expected flattened 1D leaves, got: {}", flat_leaves.len());
    }

    #[test]
    fn test_accessibility_modifiers_and_clearance_lattice() {
        // 1. Inaccessible private method accessed from outside triggers VIS001
        let priv_code = r#"
        class BankAccount {
            balance: int;

            fn internal_vault() -> int {
                return .balance;
            }

            pub fn get_balance() -> int {
                return .internal_vault();
            }
        }

        fn test_access() {
            let acct = BankAccount();
            acct.internal_vault();
        }
        "#;
        let arena_priv = pilot_parser::parse(priv_code);
        let res_priv = AnalysisPipeline::analyze(&arena_priv, arena_priv.root);
        assert!(!res_priv.is_ok(), "Expected VIS001 diagnostic for calling private method from outside");
        assert!(res_priv.diagnostics.iter().any(|d| d.code == "VIS001"), "Expected VIS001, got: {:?}", res_priv.diagnostics);

        // 2. Calling private method from inside class method via self or prefix dot succeeds
        let valid_priv = r#"
        class BankAccount {
            balance: int;

            fn internal_vault() -> int {
                return .balance;
            }

            pub fn get_balance() -> int {
                return .internal_vault();
            }
        }

        fn test_access() {
            let acct = BankAccount();
            acct.get_balance();
        }
        "#;
        let arena_valid = pilot_parser::parse(valid_priv);
        let res_valid = AnalysisPipeline::analyze(&arena_valid, arena_valid.root);
        assert!(res_valid.is_ok(), "Expected clean analysis, got: {:?}", res_valid.diagnostics);

        // 3. Insufficient clearance calling lvl(30) function from lvl(0) triggers LVL001
        let low_clearance = r#"
        lvl30 fn secure_wipe() {
        }

        fn regular_user() {
            secure_wipe();
        }
        "#;
        let arena_low = pilot_parser::parse(low_clearance);
        let res_low = AnalysisPipeline::analyze(&arena_low, arena_low.root);
        assert!(!res_low.is_ok(), "Expected LVL001 diagnostic for insufficient clearance");
        assert!(res_low.diagnostics.iter().any(|d| d.code == "LVL001"), "Expected LVL001, got: {:?}", res_low.diagnostics);

        // 4. Elevated clearance via function declaration lvl30 allows call
        let elevated_fn = r#"
        lvl30 fn secure_wipe() {
        }

        lvl30 fn admin_panel() {
            secure_wipe();
        }
        "#;
        let arena_elevated = pilot_parser::parse(elevated_fn);
        let res_elevated = AnalysisPipeline::analyze(&arena_elevated, arena_elevated.root);
        assert!(res_elevated.is_ok(), "Expected clean analysis with matching clearance, got: {:?}", res_elevated.diagnostics);

        // 5. Flow-typed elevation via `if lvl >= 30 { ... }`
        let guard_code = r#"
        lvl30 fn secure_wipe() {
        }

        fn dispatch(lvl: int) {
            if lvl >= 30 {
                secure_wipe();
            }
        }
        "#;
        let arena_guard = pilot_parser::parse(guard_code);
        let res_guard = AnalysisPipeline::analyze(&arena_guard, arena_guard.root);
        assert!(res_guard.is_ok(), "Expected clean analysis with flow-typed elevation, got: {:?}", res_guard.diagnostics);
    }

    #[test]
    fn test_flat_sibling_setters_variation_b() {
        // Variation B flat sibling property and indexer setters
        let code = r#"
        class ConfigBuffer {
            data: int;

            timeout: int {
                get => .data;
                pub set => { .data = value; }
                pub set(multiplier: float) => { .data = 100; }
                priv set(raw: (int, int)) => { .data = 200; }
            }

            self[key: str] -> int {
                get => .data;
                default => 0;
                set => { .data = value; }
                set(iso token: str) => { .data = 50; }
            }
        }
        "#;
        let arena = pilot_parser::parse(code);
        assert!(!arena.accessor_clauses.is_empty(), "Expected parsed accessor clauses");
        let res = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(res.is_ok(), "Expected clean analysis for Variation B flat sibling accessors, got: {:?}", res.diagnostics);
    }
}
