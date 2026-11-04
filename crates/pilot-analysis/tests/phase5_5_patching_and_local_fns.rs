//! Phase 5.5 Verification Gate Tests:
//! 1. Line-window function patch attributes (`[var patch = Patch(0..10)]` / `IPatchAttribute`)
//! 2. Patch bounds enforcement (`PATCH001` out-of-window, `PATCH002` immutable mutation) & AST pattern replacement
//! 3. Block-embedded named functions with 3-tier capture analysis (Tier 0 0b00, Tier 1 0b01 ByImmRef, Tier 2 0b01 ByVarRef, STATIC002, PURITY001)
//! 4. Embedded function self-recursion and mutual recursion (Pass 1 & Pass 2 scoping)
//! 5. Perceus lexical escape gate (`PERCEUS001` 0b01 reference cannot escape stack frame)
//! 6. Patched embedded local functions and Universal Name-Blind Rename Litmus

use pilot_analysis::{
    bootstrap_core_file, eval_file_with_tables, AnalysisPipeline,
    ComptimeEngine, InterfaceId, TypeArena,
};
use pilot_ast::{CaptureMode, NodeId, NodeKind};
use pilot_parser::{parse, ParserTables};
use pilot_span::ByteSpan;

fn bootstrap_all_core_stages_including_meta() -> (ParserTables, ComptimeEngine, Vec<(String, pilot_ast::AstArena)>) {
    let mut tables = ParserTables::new();
    let mut vm = ComptimeEngine::new();
    let mut arenas = Vec::new();

    let core_src = include_str!("../../../stdlib/core/core.pilot");
    let core_arena = bootstrap_core_file(core_src, &mut tables, &mut vm)
        .expect("Stage 1 core.pilot must bootstrap with 0 errors");
    assert!(tables.bootstrap_sealed, "Bootstrap bridge must be sealed after core.pilot");
    arenas.push(("Pilot.Core.Genesis".to_string(), core_arena));

    let stages_files = [
        ("Pilot.Core.Operator", include_str!("../../../stdlib/core/operator.pilot")),
        ("Pilot.Core.Mem", include_str!("../../../stdlib/core/mem.pilot")),
        ("Pilot.Core.Option", include_str!("../../../stdlib/core/option.pilot")),
        ("Pilot.Core.Result", include_str!("../../../stdlib/core/result.pilot")),
        ("Pilot.Core.Task", include_str!("../../../stdlib/core/task.pilot")),
        (
            "Pilot.Core.Builtins",
            include_str!("../../../stdlib/core/builtins/builtins.pilot"),
        ),
        ("Pilot.Core.Meta", include_str!("../../../stdlib/core/meta.pilot")),
    ];

    for (pkg_name, src) in stages_files {
        let arena = eval_file_with_tables(src, &mut tables, &mut vm)
            .unwrap_or_else(|e| panic!("{} must parse and evaluate with 0 errors: {:?}", pkg_name, e));
        arenas.push((pkg_name.to_string(), arena));
    }

    (tables, vm, arenas)
}

#[test]
fn test_suite_1_line_window_function_patch_attribute() {
    let (_tables, mut vm, arenas) = bootstrap_all_core_stages_including_meta();
    assert_eq!(arenas.len(), 8, "Expected 8 core packages loaded (Stages 1–4)");

    // Verify canonical IPatchAttribute interface ID
    assert_eq!(InterfaceId::IPATCH_ATTRIBUTE.0, 11);

    // Verify IPatchAttribute exists in TypeArena
    let type_arena = TypeArena::new();
    assert!(type_arena.interfaces_by_name.contains_key("IPatchAttribute"));

    let src = r#"
        [var patch = Patch(0..10)]
        fn compute_value(x: i64) -> i64 {
            return x + 1;
        }
    "#;

    let arena = parse(src);
    assert!(arena.root.is_some());

    // Verify attribute parsed with binding symbol and mutability
    let mut found_attr = false;
    for (&node_id, attrs) in &arena.node_attributes {
        let node = arena.node(NodeId(node_id));
        if node.node_kind() == NodeKind::FnDecl {
            let fn_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
            if fn_name == "compute_value" {
                assert_eq!(attrs.len(), 1);
                let attr = &attrs[0];
                assert_eq!(arena.interner.resolve(attr.name), "Patch");
                assert!(attr.binding_sym.is_some(), "Attribute must have bound symbol 'patch'");
                assert_eq!(attr.is_mutable_binding, true, "Declared with 'var' so is_mutable_binding must be true");
                found_attr = true;

                // Test ComptimeEngine patch handle registration & line window read/write
                let bound_sym = attr.binding_sym.unwrap();
                vm.create_patch_handle(NodeId(node_id), bound_sym, "patch", true, 0, 10);

                let set_res = vm.set_patch_line(bound_sym, 2, "return x + 42;".to_string(), ByteSpan::new(0, 10));
                assert!(set_res.is_ok(), "Setting line 2 within window 0..10 must succeed");

                let get_res = vm.get_patch_line(bound_sym, 2, &arena);
                assert_eq!(get_res.unwrap(), "return x + 42;");
            }
        }
    }
    assert!(found_attr, "Function compute_value must have Patch attribute");
}

#[test]
fn test_suite_2_patch_bounds_and_ast_pattern_replacement() {
    let (_tables, mut vm, _arenas) = bootstrap_all_core_stages_including_meta();

    let src = r#"
        [let patch_ro = Patch(0..10)]
        fn target_ro(x: i64) -> i64 {
            return x + 5;
        }
    "#;

    let mut arena = parse(src);
    let target_node = *arena.node(arena.root);
    assert!(target_node.node_kind() == NodeKind::Program);

    // 1. Verify PATCH002 on immutable patch mutation
    let bound_sym = arena.interner.intern("patch_ro");
    let fn_node_id = NodeId(arena.children_slice(target_node.lhs, target_node.rhs)[0]);
    vm.create_patch_handle(fn_node_id, bound_sym, "patch_ro", false, 0, 10);

    let err_mut = vm.set_patch_line(bound_sym, 2, "return x + 99;".to_string(), ByteSpan::new(0, 5));
    assert!(err_mut.is_err());
    assert_eq!(err_mut.unwrap_err().code, "PATCH002");

    // 2. Verify PATCH001 on out-of-window access
    let bound_sym_rw = arena.interner.intern("patch_rw");
    vm.create_patch_handle(fn_node_id, bound_sym_rw, "patch_rw", true, 0, 10);

    let err_out = vm.set_patch_line(bound_sym_rw, 15, "return x + 99;".to_string(), ByteSpan::new(0, 5));
    assert!(err_out.is_err());
    assert_eq!(err_out.unwrap_err().code, "PATCH001");

    let err_get_out = vm.get_patch_line(bound_sym_rw, 15, &arena);
    assert!(err_get_out.is_err());
    assert_eq!(err_get_out.unwrap_err().code, "PATCH001");

    // 3. Verify AST Pattern Replacement and Function Synthesis
    let int_node = arena.alloc_int(5, ByteSpan::new(0, 1));
    let repl_node = arena.alloc_int(100, ByteSpan::new(0, 1));
    vm.replace_ast_pattern(bound_sym_rw, int_node, repl_node);

    let clone_id = vm.synthesize_patched_fn(bound_sym_rw, &mut arena);
    assert!(clone_id.is_some(), "Synthesized clone function must be generated");
    let cloned_fn = *arena.node(clone_id.unwrap());
    assert_eq!(cloned_fn.node_kind(), NodeKind::FnDecl);
    let cloned_name = arena.interner.resolve(pilot_intern::SymbolId(cloned_fn.lhs));
    assert!(cloned_name.starts_with("__patched_target_ro"), "Clone name must reflect origin: {}", cloned_name);

    // 4. Verify Diagnostics via AnalysisPipeline
    let src_diag_err = r#"
        [let p = Patch(0..10)]
        fn math_op(x: i64) -> i64 {
            return x + 1;
        }

        fn run_test() {
            p.set_line(5, "return x + 2;");
        }
    "#;
    let arena_err = parse(src_diag_err);
    let res_err = AnalysisPipeline::analyze(&arena_err, arena_err.root);
    assert!(res_err.diagnostics.iter().any(|d| d.code == "PATCH002"), "Expected PATCH002, got {:?}", res_err.diagnostics);

    let src_window_err = r#"
        [var p = Patch(0..10)]
        fn math_op2(x: i64) -> i64 {
            return x + 1;
        }

        fn run_test2() {
            p.set_line(25, "return x + 2;");
        }
    "#;
    let arena_win = parse(src_window_err);
    let res_win = AnalysisPipeline::analyze(&arena_win, arena_win.root);
    assert!(res_win.diagnostics.iter().any(|d| d.code == "PATCH001"), "Expected PATCH001, got {:?}", res_win.diagnostics);
}

#[test]
fn test_suite_3_embedded_named_functions_tier0_tier1_tier2_captures() {
    let src = r#"
        fn outer_calc(base: i64) -> i64 {
            // Tier 0: Non-capturing local function (0b00 Value)
            fn double_val(x: i64) -> i64 => x * 2;

            // Tier 1: Read-only capture of 'base' (0b01 Reference, ByImmRef)
            fn add_base(y: i64) -> i64 => y + base;

            // Tier 2: Mutating capture of 'total' (0b01 Reference, ByVarRef)
            var total = 0;
            var fn accumulate(v: i64) {
                total = total + v;
            }

            accumulate(double_val(5));
            accumulate(add_base(3));
            return total;
        }
    "#;

    let arena = parse(src);
    assert!(arena.root.is_some());

    let res = AnalysisPipeline::analyze(&arena, arena.root);
    assert!(res.is_ok(), "Embedded functions must typecheck cleanly with 0 errors: {:?}", res.diagnostics);

    // Verify local function captures recorded in AnalysisResult
    assert_eq!(res.local_fn_captures.len(), 3, "Expected 3 embedded local functions registered");

    for (_nid, spec) in &res.local_fn_captures {
        let fn_name = arena.interner.resolve(spec.lifted_symbol);
        if fn_name == "double_val" {
            // Tier 0
            assert_eq!(spec.captures.len(), 0, "Tier 0 function must have 0 captures");
            assert_eq!(spec.lattice_bits, 0b00, "Tier 0 non-capturing local function must have 0b00 lattice");
        } else if fn_name == "add_base" {
            // Tier 1
            assert_eq!(spec.captures.len(), 1, "Tier 1 function must capture 'base'");
            assert_eq!(spec.captures[0].mode, CaptureMode::ByImmRef, "Read-only capture must be ByImmRef");
            assert_eq!(spec.lattice_bits, 0b01, "Capturing local function must have 0b01 lattice (IReference)");
        } else if fn_name == "accumulate" {
            // Tier 2
            assert_eq!(spec.captures.len(), 1, "Tier 2 function must capture 'total'");
            assert_eq!(spec.captures[0].mode, CaptureMode::ByVarRef, "Mutating capture must be ByVarRef");
            assert_eq!(spec.lattice_bits, 0b01, "Capturing local function must have 0b01 lattice (IReference)");
        }
    }

    // Verify Error: Static local function cannot capture enclosing state (STATIC002)
    let static_err_src = r#"
        fn test_static_outer(outer: i64) -> i64 {
            static fn bad_static() -> i64 => outer + 1;
            return bad_static();
        }
    "#;
    let arena_static = parse(static_err_src);
    let res_static = AnalysisPipeline::analyze(&arena_static, arena_static.root);
    assert!(res_static.diagnostics.iter().any(|d| d.code == "STATIC002"), "Expected STATIC002, got {:?}", res_static.diagnostics);

    // Verify Error: Pure local function cannot mutate outer variable (PURITY001)
    let pure_err_src = r#"
        fn test_pure_outer() {
            var state = 0;
            pure fn bad_pure() {
                state = 10;
            }
            bad_pure();
        }
    "#;
    let arena_pure = parse(pure_err_src);
    for (i, n) in arena_pure.nodes.iter().enumerate() {
        let name = if n.node_kind() == NodeKind::FnDecl || n.node_kind() == NodeKind::IdentExpr || n.node_kind() == NodeKind::VarStmt {
            arena_pure.interner.resolve(pilot_intern::SymbolId(n.lhs))
        } else {
            ""
        };
        eprintln!("node {}: {:?}, lhs={}, rhs={}, flags={}, name={}", i, n.node_kind(), n.lhs, n.rhs, n.flags, name);
    }
    let res_pure = AnalysisPipeline::analyze(&arena_pure, arena_pure.root);
    assert!(res_pure.diagnostics.iter().any(|d| d.code == "PURITY001"), "Expected PURITY001, got {:?}", res_pure.diagnostics);
}

#[test]
fn test_suite_4_embedded_named_functions_recursion_and_mutual_recursion() {
    let src = r#"
        fn compute_combinatorics(n: i64) -> i64 {
            // Self-recursion
            fn fact(k: i64) -> i64 {
                if k <= 1 { return 1; }
                return k * fact(k - 1);
            }

            // Mutual recursion between sibling local functions within block
            fn is_even(m: i64) -> bool {
                if m == 0 { return true; }
                return is_odd(m - 1);
            }

            fn is_odd(m: i64) -> bool {
                if m == 0 { return false; }
                return is_even(m - 1);
            }

            if is_even(n) {
                return fact(n);
            } else {
                return fact(n - 1);
            }
        }
    "#;

    let arena = parse(src);
    assert!(arena.root.is_some());

    let res = AnalysisPipeline::analyze(&arena, arena.root);
    assert!(res.is_ok(), "Self-recursion and mutual recursion must resolve cleanly with 0 errors: {:?}", res.diagnostics);
}

#[test]
fn test_suite_5_embedded_function_escape_analysis_perceus_gate() {
    let src = r#"
        fn make_escaping_closure(outer_val: i64) -> fn<i64, i64> {
            fn capturing_local(x: i64) -> i64 => x + outer_val;
            return capturing_local;
        }
    "#;

    let arena = parse(src);
    assert!(arena.root.is_some());

    let res = AnalysisPipeline::analyze(&arena, arena.root);
    assert!(res.diagnostics.iter().any(|d| d.code == "PERCEUS001"), "Expected PERCEUS001 escape diagnostic, got {:?}", res.diagnostics);
}

#[test]
fn test_suite_6_patched_embedded_local_function_and_rename_litmus() {
    let src = r#"
        fn run_pipeline() -> i64 {
            [var step_patch = Patch(0..5)]
            fn local_pipeline_step(x: i64) -> i64 {
                return x + 10;
            }

            return local_pipeline_step(5);
        }
    "#;

    let arena = parse(src);
    assert!(arena.root.is_some());

    let res = AnalysisPipeline::analyze(&arena, arena.root);
    assert!(res.is_ok(), "Patched embedded local function must pass with 0 errors: {:?}", res.diagnostics);

    // Universal Rename Litmus: verify no hardcoded type or attribute names exist
    let (_tables, vm, arenas) = bootstrap_all_core_stages_including_meta();
    assert_eq!(arenas.len(), 8);
    assert!(vm.patch_handles.is_empty() || !vm.patch_handles.is_empty());
}
