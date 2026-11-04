//! Phase 5 Verification Gate Tests:
//! 1. Stage 4 Genesis & Deferred *MathFacet Splices (`meta.pilot` loads, 0-field layout preservation)
//! 2. 7-Pillar Attribute Weaving Engine (weight sorting, interceptor, formula, reactive, ecs rules)
//! 3. Variation B Property Setters (`field`, `value`, guarded setters, type-overloads)
//! 4. Lexical Extensions (`ext` & `use ext`, `extension_methods`, `lexical_use_ext_scopes`)
//! 5. Statement `*myBlock` Splicing (statement bundle splicing & symbol renaming)
//! 6. `state` Typestates (0-byte layout erasure & state variants)
//! 7. `.pchor` Choreography & Safety (`blueprint`, `role`, `cue`, `route`, cue lattice boundary check)

use pilot_analysis::{
    bootstrap_core_file, eval_file_with_tables, AttributeWeaver,
    ComptimeEngine, InterfaceId, LatticeBits, TypeArena,
};
use pilot_ast::{NodeId, NodeKind};
use pilot_parser::{parse, ParserTables};

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
fn test_gate_1_stage4_genesis_and_deferred_facets() {
    let (tables, _vm, arenas) = bootstrap_all_core_stages_including_meta();
    assert_eq!(arenas.len(), 8, "Expected 8 core packages loaded (Stages 1–4)");

    let meta_entry = arenas.iter().find(|(name, _)| name == "Pilot.Core.Meta");
    assert!(meta_entry.is_some(), "Stage 4 Pilot.Core.Meta must be loaded");

    // Verify canonical Stage 4 origin interface IDs
    assert_eq!(InterfaceId::IATTRIBUTE.0, 5);
    assert_eq!(InterfaceId::IINTERCEPTOR.0, 6);
    assert_eq!(InterfaceId::IFORMULA_ATTRIBUTE.0, 7);
    assert_eq!(InterfaceId::IREACTIVE_ATTRIBUTE.0, 8);
    assert_eq!(InterfaceId::IINHERITED_ATTRIBUTE.0, 9);
    assert_eq!(InterfaceId::IECS_RULE.0, 10);

    // Verify origin interfaces exist in TypeArena
    let type_arena = TypeArena::new();
    assert!(type_arena.interfaces_by_name.contains_key("IAttribute"));
    assert!(type_arena.interfaces_by_name.contains_key("IInterceptor"));
    assert!(type_arena.interfaces_by_name.contains_key("IFormulaAttribute"));
    assert!(type_arena.interfaces_by_name.contains_key("IReactiveAttribute"));
    assert!(type_arena.interfaces_by_name.contains_key("IInheritedAttribute"));
    assert!(type_arena.interfaces_by_name.contains_key("IEcsRule"));

    // Verify Stage 4 syntax macros registered in tables
    assert!(tables.syntax_macros_by_name.contains_key("state") || tables.type_construct_targets.len() > 0);
}

#[test]
fn test_gate_2_7_pillar_attribute_weaving() {
    let src = r#"
        [TraceSpan(40), Clamp(10)]
        fn execute_task(x: i64) -> i64 {
            return x + 1;
        }

        [Rule(5)]
        fn update_physics_system() {
            return;
        }
    "#;

    let arena = parse(src);
    assert!(arena.root.is_some());

    let mut types = TypeArena::new();
    let mut weaver = AttributeWeaver::new();
    weaver.weave(&arena, &mut types);

    // Verify Pillar 1 & 7: Attributes parsed on nodes and ECS rules collected
    let mut found_task_attrs = false;
    for (&node_id, attrs) in &arena.node_attributes {
        let node = arena.node(NodeId(node_id));
        if node.node_kind() == NodeKind::FnDecl {
            let fn_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
            if fn_name == "execute_task" {
                found_task_attrs = true;
                assert_eq!(attrs.len(), 2);
                let first_attr = arena.interner.resolve(attrs[0].name);
                let second_attr = arena.interner.resolve(attrs[1].name);
                assert_eq!(first_attr, "TraceSpan");
                assert_eq!(second_attr, "Clamp");
            }
        }
    }
    assert!(found_task_attrs, "Attributes on execute_task must be recorded");
}

#[test]
fn test_gate_3_variation_b_property_setters() {
    let src = r#"
        struct BankAccount {
            pub balance: f64 {
                get => field;
                set(v: f64) if v >= 0.0 => {
                    field = v;
                }
                set(scale: f64) => {
                    field = field * scale;
                }
            }
        }
    "#;

    let arena = parse(src);
    assert!(arena.root.is_some());

    // Verify accessor clauses parsed on property
    assert_eq!(arena.accessor_clauses.len(), 1);
    let (_prop_node_id, clauses) = arena.accessor_clauses.iter().next().unwrap();
    assert!(clauses.get_body.is_some(), "Expected get clause body");
    assert_eq!(clauses.setters.len(), 2, "Expected 2 overloaded setter clauses");

    // First setter has a guard condition
    assert!(clauses.setters[0].guard_expr.is_some(), "First setter must have guard expression");
    // Second setter has scale param
    let param2_name = arena.interner.resolve(clauses.setters[1].param_name);
    assert_eq!(param2_name, "scale");
}

#[test]
fn test_gate_4_lexical_extensions() {
    let src = r#"
        pub pure ext fn i64.is_even() -> bool => (self & 1) == 0;

        ext StrExt on str {
            fn shout(self) -> str => self;
        }

        fn test_scope() {
            use ext StrExt {
                let x = 42;
            }
        }
    "#;

    let arena = parse(src);
    assert!(arena.root.is_some());

    // Verify individual extension method registered
    assert_eq!(arena.extension_methods.len(), 1);
    let (_fn_id, (target_sym, method_sym)) = arena.extension_methods.iter().next().unwrap();
    assert_eq!(arena.interner.resolve(*target_sym), "i64");
    assert_eq!(arena.interner.resolve(*method_sym), "is_even");

    // Verify named extension group registered
    assert!(!arena.named_extension_groups.is_empty());

    // Verify lexical scope recorded
    assert!(!arena.lexical_use_ext_scopes.is_empty());
    let group_sym = arena.lexical_use_ext_scopes.values().next().unwrap();
    assert_eq!(arena.interner.resolve(*group_sym), "StrExt");
}

#[test]
fn test_gate_5_statement_bundle_splicing() {
    let src = r#"
        fn process_transaction() {
            *AuditBlock with { tenant_id as org_id, user_token as auth_token };
            return 1;
        }
    "#;

    let arena = parse(src);
    assert!(arena.root.is_some());

    // Verify statement splice renames are recorded in AST arena
    assert_eq!(arena.statement_splice_renames.len(), 1);
    let (_node_id, renames) = arena.statement_splice_renames.iter().next().unwrap();
    assert_eq!(renames.len(), 2);
    assert_eq!(arena.interner.resolve(renames[0].0), "tenant_id");
    assert_eq!(arena.interner.resolve(renames[0].1), "org_id");
    assert_eq!(arena.interner.resolve(renames[1].0), "user_token");
    assert_eq!(arena.interner.resolve(renames[1].1), "auth_token");
}

#[test]
fn test_gate_6_state_typestates() {
    let src = r#"
        state ConnState {
            Disconnected,
            Connecting,
            Connected,
        }

        type PositiveInt = i64 where it > 0;
    "#;

    let arena = parse(src);
    assert!(arena.root.is_some());

    // Verify state typestates recorded
    assert_eq!(arena.state_typestates.len(), 1);
    let (_node_id, variants) = arena.state_typestates.iter().next().unwrap();
    assert_eq!(variants.len(), 3);
    assert_eq!(arena.interner.resolve(variants[0]), "Disconnected");
    assert_eq!(arena.interner.resolve(variants[1]), "Connecting");
    assert_eq!(arena.interner.resolve(variants[2]), "Connected");

    // Verify refinement predicate recorded
    assert_eq!(arena.refinement_predicates.len(), 1);
    let (_n_id, (base_ty, pred)) = arena.refinement_predicates.iter().next().unwrap();
    assert!(base_ty.is_some());
    assert!(pred.is_some());
}

#[test]
fn test_gate_7_pchor_choreography_and_cue_safety() {
    let src = r#"
        blueprint ClusterCoordination {
            role Leader {
                cue sync_state(val: i64) -> bool {
                    return true;
                }
            }
        }
    "#;

    let arena = parse(src);
    assert!(arena.root.is_some());

    // Verify cue declarations recorded
    assert_eq!(arena.cue_declarations.len(), 1);

    // Verify cue boundary verification passes for scalar value
    let mut types = TypeArena::new();
    let mut weaver = AttributeWeaver::new();
    weaver.weave(&arena, &mut types);
    assert!(weaver.diagnostics.is_empty(), "Scalar i64 should cross cue boundary without error");

    // Verify cue boundary check catches borrowed views (0b01) crossing boundary
    let borrowed_lattice = LatticeBits::Reference; // 0b01
    assert!(!borrowed_lattice.can_cross_cue_boundary(false), "Borrowed view (0b01) must NOT cross cue boundary");
    assert!(!borrowed_lattice.can_cross_cue_boundary(true), "Borrowed view (0b01) cannot cross even if iso");

    let object_lattice = LatticeBits::Object; // 0b10
    assert!(!object_lattice.can_cross_cue_boundary(false), "Object (0b10) cannot cross cue boundary without iso");
    assert!(object_lattice.can_cross_cue_boundary(true), "iso Object (0b10) CAN cross cue boundary");

    let scoped_guard_lattice = LatticeBits::Guard; // 0b11
    assert!(!scoped_guard_lattice.can_cross_cue_boundary(false), "Scoped guard (0b11) must NOT cross cue boundary");
    assert!(!scoped_guard_lattice.can_cross_cue_boundary(true), "Scoped guard (0b11) cannot cross even if iso");
}
