//! Phase 5 Verification Gate Tests:
//! 1. `test_stage4_meta_pilot_bootstraps_and_passes_core_naming`
//! 2. `test_7pillar_pilot_quote_weavers_and_variation_b_properties`
//! 3. `test_lexical_extensions_and_statement_block_splicing`
//! 4. `test_deferred_partial_struct_math_facets`
//! 5. `test_state_typestates_and_where_refinement_types`
//! 6. `test_pchor_cue_verification_via_meta_pilot_and_soa_layout`
//! 7. `test_name_blind_attribute_and_facet_rename_litmus`

use pilot_analysis::{
    bootstrap_core_file, eval_file_with_tables, verify_core_naming_rules, AttributeWeaver,
    BundleDef, BundleId, ComptimeEngine, InterfaceDef, InterfaceId, LatticeBits, PackageAst,
    TypeArena, TypeDef, TypeId, TypeKind,
};
use pilot_ast::{NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_parser::{parse, PackageId, ParserTables};
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

fn populate_type_arena_from_ast(
    pkg_id: PackageId,
    arena: &pilot_ast::AstArena,
    vm: &ComptimeEngine,
    type_arena: &mut TypeArena,
) {
    for (idx, node) in arena.nodes.iter().enumerate() {
        let _nid = NodeId(idx as u32);
        match node.node_kind() {
            NodeKind::StructDecl => {
                let name_sym = SymbolId(node.lhs);
                let name_str = arena.interner.resolve(name_sym);
                let ty_id = type_arena.alloc(TypeKind::Primitive(name_str.to_string()));
                let mut td = TypeDef::new(ty_id, name_sym, pkg_id);
                if vm.lang_items_by_name.contains_key(name_str) {
                    td.is_lang_primitive = true;
                }
                type_arena.register_type_def(td, &arena.interner);
            }
            NodeKind::InterfaceDecl => {
                let name_sym = SymbolId(node.lhs);
                let iface_id = InterfaceId(type_arena.interface_defs.len() as u32 + 10);
                let idef = InterfaceDef {
                    id: iface_id,
                    name: name_sym,
                    home_pkg: pkg_id,
                    super_interfaces: Vec::new(),
                    explicit_lattice: None,
                    required_contracts: Vec::new(),
                    default_methods: Vec::new(),
                    span: ByteSpan::default(),
                };
                type_arena.register_interface_def(idef, &arena.interner);
            }
            NodeKind::LetStmt => {
                let rhs_id = NodeId(node.rhs);
                if rhs_id.is_some()
                    && (rhs_id.0 as usize) < arena.nodes.len()
                    && arena.node(rhs_id).node_kind() == NodeKind::BlockExpr
                {
                    let name_sym = SymbolId(node.lhs);
                    let name_str = arena.interner.resolve(name_sym);
                    let bid = BundleId(type_arena.bundle_defs.len() as u32 + 1);
                    let mut bdef = BundleDef {
                        id: bid,
                        name: name_sym,
                        fields: Vec::new(),
                        methods: Vec::new(),
                        span: ByteSpan::default(),
                    };
                    if name_str.ends_with("Bundle") {
                        bdef.fields.push(pilot_analysis::FieldDef {
                            name: name_sym,
                            ty: TypeId::I64,
                            byte_offset: 0,
                            visibility: 8,
                            source_bundle: Some(bid),
                            span: ByteSpan::default(),
                        });
                    }
                    type_arena.register_bundle_def(bdef, &arena.interner);
                }
            }
            _ => {}
        }
    }
}

#[test]
fn test_stage4_meta_pilot_bootstraps_and_passes_core_naming() {
    let (tables, vm, arenas) = bootstrap_all_core_stages_including_meta();
    assert_eq!(arenas.len(), 8, "Expected 8 core packages loaded (Stages 1–4)");

    let meta_entry = arenas.iter().find(|(name, _)| name == "Pilot.Core.Meta");
    assert!(meta_entry.is_some(), "Stage 4 Pilot.Core.Meta must be loaded");
    let (_, meta_arena) = meta_entry.unwrap();

    // Verify canonical Stage 4 origin interface IDs
    assert_eq!(InterfaceId::IATTRIBUTE.0, 5);
    assert_eq!(InterfaceId::IINTERCEPTOR.0, 6);
    assert_eq!(InterfaceId::IFORMULA_ATTRIBUTE.0, 7);
    assert_eq!(InterfaceId::IREACTIVE_ATTRIBUTE.0, 8);
    assert_eq!(InterfaceId::IINHERITED_ATTRIBUTE.0, 9);
    assert_eq!(InterfaceId::IECS_RULE.0, 10);

    // Verify [EnforceCoreNaming] passes on Pilot.Core.Meta
    let pkg_ast = PackageAst::new("Pilot.Core.Meta".to_string(), true);
    let mut type_arena = TypeArena::new();
    populate_type_arena_from_ast(PackageId(7), meta_arena, &vm, &mut type_arena);
    let mut naming_diags = Vec::new();
    verify_core_naming_rules(&pkg_ast, &mut type_arena, &meta_arena.interner, &mut naming_diags);
    assert!(
        naming_diags.is_empty(),
        "[EnforceCoreNaming] must pass with 0 diagnostics on Pilot.Core.Meta: {:?}",
        naming_diags
    );

    // Verify Stage 4 syntax macros registered in tables
    assert!(tables.syntax_macros_by_name.contains_key("state") || tables.type_construct_targets.len() > 0);
}

#[test]
fn test_7pillar_pilot_quote_weavers_and_variation_b_properties() {
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

    // Verify Pillar 1 & 7: Attributes parsed on nodes and sorted/collected
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

    // Test Variation B property setters
    let prop_src = r#"
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

    let prop_arena = parse(prop_src);
    assert_eq!(prop_arena.accessor_clauses.len(), 1);
    let (_prop_node_id, clauses) = prop_arena.accessor_clauses.iter().next().unwrap();
    assert!(clauses.get_body.is_some(), "Expected get clause body");
    assert_eq!(clauses.setters.len(), 2, "Expected 2 overloaded setter clauses");
    assert!(clauses.setters[0].guard_expr.is_some(), "First setter must have guard expression");
    let param2_name = prop_arena.interner.resolve(clauses.setters[1].param_name);
    assert_eq!(param2_name, "scale");
}

#[test]
fn test_lexical_extensions_and_statement_block_splicing() {
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

    // Verify statement bundle splicing with rename
    let splice_src = r#"
        fn process_transaction() {
            *AuditBlock with { tenant_id as org_id, user_token as auth_token };
            return 1;
        }
    "#;
    let splice_arena = parse(splice_src);
    assert_eq!(splice_arena.statement_splice_renames.len(), 1);
    let (_node_id, renames) = splice_arena.statement_splice_renames.iter().next().unwrap();
    assert_eq!(renames.len(), 2);
    assert_eq!(splice_arena.interner.resolve(renames[0].0), "tenant_id");
    assert_eq!(splice_arena.interner.resolve(renames[0].1), "org_id");
}

#[test]
fn test_deferred_partial_struct_math_facets() {
    let (_tables, vm, arenas) = bootstrap_all_core_stages_including_meta();

    // Verify that scalar primitives (i8..i128, u8..u128, f16..f128) retain 0 runtime wrapper bytes
    let meta_entry = arenas.iter().find(|(name, _)| name == "Pilot.Core.Meta");
    assert!(meta_entry.is_some());

    let i64_spec = vm.type_layout("i64").expect("i64 layout exists");
    assert_eq!(i64_spec.bit_width, 64);
    assert_eq!(i64_spec.fields.len(), 0, "Partial struct facets must add 0 fields");

    let u32_spec = vm.type_layout("u32").expect("u32 layout exists");
    assert_eq!(u32_spec.bit_width, 32);
    assert_eq!(u32_spec.fields.len(), 0, "Unsigned facets must add 0 fields");

    let f64_spec = vm.type_layout("f64").expect("f64 layout exists");
    assert_eq!(f64_spec.bit_width, 64);
    assert_eq!(f64_spec.fields.len(), 0, "Float facets must add 0 fields");
}

#[test]
fn test_state_typestates_and_where_refinement_types() {
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
fn test_pchor_cue_verification_via_meta_pilot_and_soa_layout() {
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

#[test]
fn test_name_blind_attribute_and_facet_rename_litmus() {
    let (mut tables, mut vm, _arenas) = bootstrap_all_core_stages_including_meta();

    // The Universal Rename Litmus Test for Phase 5:
    // Rename ClampAttr -> BoundAttr, SignedMathFacet -> IntMathFacet,
    // and verify that Pilot parser and analysis compile and evaluate with 0 errors.
    let meta_src = include_str!("../../../stdlib/core/meta.pilot");
    let renamed_meta = meta_src
        .replace("ClampAttr", "BoundAttr")
        .replace("SignedMathFacet", "IntMathFacet")
        .replace("verify_cue_payload", "check_cue_lattice");

    let eval_res = eval_file_with_tables(&renamed_meta, &mut tables, &mut vm);
    assert!(
        eval_res.is_ok(),
        "Renamed meta.pilot must parse and evaluate cleanly: {:?}",
        eval_res.err()
    );
}
