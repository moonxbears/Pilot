//! Phase 4 Verification Gate Tests:
//! 1. `test_stage2_and_stage3_core_bootstrap_complete`
//! 2. `test_all_integer_and_float_widths_ssa_ops`
//! 3. `test_symbol_construct_lowering_equivalence`
//! 4. `test_c_abi_zero_marshalling_layouts`
//! 5. `test_3tier_for_null_error_and_call_quoting`
//! 6. `test_5_backend_codegen_parity`

use pilot_analysis::{
    bootstrap_core_file, eval_file_with_tables, verify_core_naming_rules, BundleDef, BundleId,
    CallQuotingEngine, ComptimeEngine, ErrorFlowAnalyzer, ForLoopAnalyzer, ForLoweringStrategy,
    ImplicitLiftKind, InterfaceDef, InterfaceId, LangItem, LatticeBits, LoopReturnMode,
    LoopScheduleMode, MarangetChecker, PackageAst, RegisterClass, TryPropagationPlan, TypeArena,
    TypeDef, TypeId, TypeInferenceEngine, TypeKind,
};
use pilot_ast::{CallQuoteKind, NodeId, NodeKind};
use pilot_codegen::{
    asm::AsmEmitter,
    c_target::CTargetEmitter,
    gpu::GpuEmitter,
    llvm::LlvmEmitter,
    wasm::WasmEmitter,
};
use pilot_intern::SymbolId;
use pilot_parser::{parse, PackageId, ParserTables};
use pilot_span::ByteSpan;

fn bootstrap_all_core_stages() -> (ParserTables, ComptimeEngine, Vec<(String, pilot_ast::AstArena)>) {
    let mut tables = ParserTables::new();
    let mut vm = ComptimeEngine::new();
    let mut arenas = Vec::new();

    let core_src = include_str!("../../../stdlib/core/core.pilot");
    let core_arena = bootstrap_core_file(core_src, &mut tables, &mut vm)
        .expect("Stage 1 core.pilot must bootstrap with 0 errors");
    assert!(tables.bootstrap_sealed, "Bootstrap bridge must be sealed after core.pilot");
    arenas.push(("Pilot.Core.Genesis".to_string(), core_arena));

    let stage2_and_3_files = [
        ("Pilot.Core.Operator", include_str!("../../../stdlib/core/operator.pilot")),
        ("Pilot.Core.Mem", include_str!("../../../stdlib/core/mem.pilot")),
        ("Pilot.Core.Option", include_str!("../../../stdlib/core/option.pilot")),
        ("Pilot.Core.Result", include_str!("../../../stdlib/core/result.pilot")),
        ("Pilot.Core.Task", include_str!("../../../stdlib/core/task.pilot")),
        (
            "Pilot.Core.Builtins",
            include_str!("../../../stdlib/core/builtins/builtins.pilot"),
        ),
    ];

    for (pkg_name, src) in stage2_and_3_files {
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
fn test_stage2_and_stage3_core_bootstrap_complete() {
    let (_tables, vm, arenas) = bootstrap_all_core_stages();
    assert!(
        vm.diagnostics.is_empty(),
        "All 7 stdlib/core/*.pilot files must produce 0 bootstrap diagnostics: {:?}",
        vm.diagnostics
    );

    // Verify all [Lang] items across Stages 1, 2 & 3 are bound
    let expected_lang_items = [
        LangItem::IAnyValue,
        LangItem::IValue,
        LangItem::IReference,
        LangItem::IObject,
        LangItem::IGuard,
        LangItem::Void,
        LangItem::Never,
        LangItem::Bool,
        LangItem::Char,
        LangItem::I8,
        LangItem::I16,
        LangItem::I32,
        LangItem::I64,
        LangItem::I128,
        LangItem::Isize,
        LangItem::U8,
        LangItem::U16,
        LangItem::U32,
        LangItem::U64,
        LangItem::U128,
        LangItem::Usize,
        LangItem::F16,
        LangItem::F32,
        LangItem::F64,
        LangItem::F128,
        LangItem::Str,
        LangItem::Ptr,
        LangItem::Slice,
        LangItem::Stride,
        LangItem::Array,
        LangItem::Map,
        LangItem::Option,
        LangItem::Result,
        LangItem::Fn,
        LangItem::Frame,
        LangItem::Range,
        LangItem::Span,
        LangItem::Step,
        LangItem::From,
        LangItem::Until,
        LangItem::Full,
        LangItem::Splat,
        LangItem::Pairs,
        LangItem::Expand,
        LangItem::Tuple,
        LangItem::Union,
    ];

    for item in expected_lang_items {
        assert!(
            vm.lang_item_node(item).is_some(),
            "Expected [Lang] item {:?} to be bound in ComptimeEngine",
            item
        );
    }

    // Verify `[EnforceCoreNaming]` is registered and passes with 0 diagnostics across all 7 files
    assert_eq!(vm.core_naming_enforced_pkgs.len(), 7);
    for (idx, (pkg_name, arena)) in arenas.iter().enumerate() {
        let pkg_ast = PackageAst::new(pkg_name.clone(), true);
        let mut type_arena = TypeArena::new();
        populate_type_arena_from_ast(PackageId(idx as u32), arena, &vm, &mut type_arena);
        let mut naming_diags = Vec::new();
        verify_core_naming_rules(&pkg_ast, &mut type_arena, &arena.interner, &mut naming_diags);
        assert!(
            naming_diags.is_empty(),
            "[EnforceCoreNaming] must pass with 0 diagnostics on {}: {:?}",
            pkg_name,
            naming_diags
        );
    }
}

#[test]
fn test_all_integer_and_float_widths_ssa_ops() {
    let (_tables, vm, _arenas) = bootstrap_all_core_stages();

    // 1. Signed integers: i8, i16, i32, i64, i128, isize -> *SignedIntSsaFacet (`sdiv`, `srem`, `ashr`, `slt`, `sle`, `sgt`, `sge`)
    let signed_types = [
        ("i8", 8),
        ("i16", 16),
        ("i32", 32),
        ("i64", 64),
        ("i128", 128),
        ("isize", 64),
    ];
    for (tname, bits) in signed_types {
        let layout = vm
            .type_layout(tname)
            .unwrap_or_else(|| panic!("Missing TypeLayoutSpec for {}", tname));
        assert_eq!(layout.bit_width, bits, "Bit width mismatch for {}", tname);
        assert_eq!(layout.reg_class, RegisterClass::GprSigned);
        assert_eq!(vm.ssa_opcode_for_operator(tname, "+"), Some("add"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "-"), Some("sub"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "*"), Some("mul"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "/"), Some("sdiv"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "%"), Some("srem"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, ">>"), Some("ashr"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "<"), Some("slt"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "<="), Some("sle"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, ">"), Some("sgt"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, ">="), Some("sge"));
    }

    // 2. Unsigned integers: u8, u16, u32, u64, u128, usize -> *UnsignedIntSsaFacet (`udiv`, `urem`, `lshr`, `ult`, `ule`, `ugt`, `uge`)
    let unsigned_types = [
        ("u8", 8),
        ("u16", 16),
        ("u32", 32),
        ("u64", 64),
        ("u128", 128),
        ("usize", 64),
    ];
    for (tname, bits) in unsigned_types {
        let layout = vm
            .type_layout(tname)
            .unwrap_or_else(|| panic!("Missing TypeLayoutSpec for {}", tname));
        assert_eq!(layout.bit_width, bits, "Bit width mismatch for {}", tname);
        assert_eq!(layout.reg_class, RegisterClass::GprUnsigned);
        assert_eq!(vm.ssa_opcode_for_operator(tname, "+"), Some("add"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "-"), Some("sub"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "*"), Some("mul"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "/"), Some("udiv"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "%"), Some("urem"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, ">>"), Some("lshr"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "<"), Some("ult"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "<="), Some("ule"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, ">"), Some("ugt"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, ">="), Some("uge"));
    }

    // 3. IEEE-754 floats: f16, f32, f64, f128 -> *FloatSsaFacet (`fadd`, `fsub`, `fmul`, `fdiv`, `fneg`, `feq`, `flt`, `fle`)
    let float_types = [("f16", 16), ("f32", 32), ("f64", 64), ("f128", 128)];
    for (tname, bits) in float_types {
        let layout = vm
            .type_layout(tname)
            .unwrap_or_else(|| panic!("Missing TypeLayoutSpec for {}", tname));
        assert_eq!(layout.bit_width, bits, "Bit width mismatch for {}", tname);
        assert_eq!(layout.reg_class, RegisterClass::SimdFloat);
        assert_eq!(vm.ssa_opcode_for_operator(tname, "+"), Some("fadd"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "-"), Some("fsub"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "*"), Some("fmul"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "/"), Some("fdiv"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "=="), Some("feq"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "<"), Some("flt"));
        assert_eq!(vm.ssa_opcode_for_operator(tname, "<="), Some("fle"));
    }
}

#[test]
fn test_symbol_construct_lowering_equivalence() {
    let src = r#"
        pub syntax type_construct {
            (T[])      => quote { slice<$T> };
            (T[..])    => quote { stride<$T> };
            (T?)       => quote { option<$T> };
            (T ! E)    => quote { result<$T, $E> };
            ((A) => R) => quote { fn<$A, $R> };
            (T1 | T2)  => quote { union<$T1, $T2> };
        }
        let a1: i32[];
        let a2: slice<i32>;
        let b1: f64[..];
        let b2: stride<f64>;
        let c1: u8?;
        let c2: option<u8>;
        let d1: i64 ! str;
        let d2: result<i64, str>;
        let e1: (i32) => bool;
        let e2: (i32) -> bool;
        let f1: i32 | str;
    "#;
    let arena = parse(src);
    let root = arena.node(arena.root);
    let stmts = arena.children_slice(root.lhs, root.rhs);
    assert_eq!(stmts.len(), 12);

    let ty_of = |stmt_idx: usize| -> pilot_ast::AstNode {
        let stmt_id = NodeId(stmts[stmt_idx + 1]);
        let ty_id = *arena.type_annotations.get(&stmt_id.0).unwrap();
        *arena.node(ty_id)
    };

    // 1. `i32[]` <-> `slice<i32>` -> both lower to `NodeKind::TypeSlice` with `flags == 0`
    let t_a1 = ty_of(0);
    let t_a2 = ty_of(1);
    assert_eq!(t_a1.node_kind(), NodeKind::TypeSlice);
    assert_eq!(t_a2.node_kind(), NodeKind::TypeSlice);
    assert_eq!(t_a1.flags, 0);
    assert_eq!(t_a2.flags, 0);

    // 2. `f64[..]` -> `NodeKind::TypeSlice` (`flags == 3`)
    let t_b1 = ty_of(2);
    assert_eq!(t_b1.node_kind(), NodeKind::TypeSlice);
    assert_eq!(t_b1.flags, 3);

    // 3. `u8?` <-> `option<u8>` -> both lower to `NodeKind::TypeOptional`
    let t_c1 = ty_of(4);
    let t_c2 = ty_of(5);
    assert_eq!(t_c1.node_kind(), NodeKind::TypeOptional);
    assert_eq!(t_c2.node_kind(), NodeKind::TypeOptional);

    // 4. `i64 ! str` <-> `result<i64, str>` -> both lower to `NodeKind::TypeFallible`
    let t_d1 = ty_of(6);
    let t_d2 = ty_of(7);
    assert_eq!(t_d1.node_kind(), NodeKind::TypeFallible);
    assert_eq!(t_d2.node_kind(), NodeKind::TypeFallible);

    // 5. `(i32) => bool` <-> `(i32) -> bool` -> both lower to `NodeKind::TypeFn`
    let t_e1 = ty_of(8);
    let t_e2 = ty_of(9);
    assert_eq!(t_e1.node_kind(), NodeKind::TypeFn);
    assert_eq!(t_e2.node_kind(), NodeKind::TypeFn);

    // 6. `i32 | str` -> `NodeKind::TypeUnion`
    let t_f1 = ty_of(10);
    assert_eq!(t_f1.node_kind(), NodeKind::TypeUnion);
}

#[test]
fn test_c_abi_zero_marshalling_layouts() {
    let (_tables, vm, _arenas) = bootstrap_all_core_stages();

    // 1. `RangeWindow<i32>` == 8 bytes (`{ current: i32, max: i32 }`)
    let (rw32_sz, rw32_align, rw32_lat) = vm.query_type_size_align_lattice("RangeWindow<i32>");
    assert_eq!(rw32_sz, 8);
    assert_eq!(rw32_align, 4);
    assert_eq!(rw32_lat, 0b00);

    // 2. `RangeWindow<i64>` == 16 bytes (`{ current: i64, max: i64 }`)
    let (rw64_sz, rw64_align, rw64_lat) = vm.query_type_size_align_lattice("RangeWindow<i64>");
    assert_eq!(rw64_sz, 16);
    assert_eq!(rw64_align, 8);
    assert_eq!(rw64_lat, 0b00);

    // 3. `slice<u8>` & `str` == 16 bytes (`{ data: ptr<u8>, len: usize }`), lattice `0b01` (`IReference`)
    let (slice_sz, slice_align, slice_lat) = vm.query_type_size_align_lattice("slice<u8>");
    assert_eq!(slice_sz, 16);
    assert_eq!(slice_align, 8);
    assert_eq!(slice_lat, 0b01);

    let (str_sz, str_align, str_lat) = vm.query_type_size_align_lattice("str");
    assert_eq!(str_sz, 16);
    assert_eq!(str_align, 8);
    assert_eq!(str_lat, 0b01);

    // 4. `stride<u8>` == 24 bytes (`{ data: ptr<u8>, len: usize, step_by: isize }`), lattice `0b01` (`IReference`)
    let (stride_sz, stride_align, stride_lat) = vm.query_type_size_align_lattice("stride<u8>");
    assert_eq!(stride_sz, 24);
    assert_eq!(stride_align, 8);
    assert_eq!(stride_lat, 0b01);

    // 5. `SliceBuffer<f32>` == 24 bytes (`{ data: ptr<f32>, len: usize, cap: usize }`), lattice `0b10` (`IObject`)
    let (buf_sz, buf_align, buf_lat) = vm.query_type_size_align_lattice("SliceBuffer<f32>");
    assert_eq!(buf_sz, 24);
    assert_eq!(buf_align, 8);
    assert_eq!(buf_lat, 0b10);

    // 6. `ArenaScope` == 16 bytes, lattice `0b11` (`IGuard`: borrows `SliceView<u8>` + runs `~self()`)
    let (scope_sz, _scope_align, scope_lat) = vm.query_type_size_align_lattice("ArenaScope");
    assert_eq!(scope_sz, 16);
    assert_eq!(scope_lat, 0b11);
}

#[test]
fn test_3tier_for_null_error_and_call_quoting() {
    // 1. 3-Tier Dynamic `for` Expression Analyzer (`FixedScalar`, `MutableBounds`, `WorklistStack`, `CollectionYield`)
    let loop_src = r#"
        for i in 0 .. 10 {
            let x = i;
        }
        for j in 0 .. 100 {
            continue j + 1 .. 50;
        }
        for k in 0 .. 100 {
            yield k * 2;
        }
    "#;
    let loop_arena = parse(loop_src);
    let root = loop_arena.node(loop_arena.root);
    let for_nodes = loop_arena.children_slice(root.lhs, root.rhs).to_vec();
    assert_eq!(for_nodes.len(), 3);

    let mut infer = TypeInferenceEngine::new();
    let mut loop_analyzer = ForLoopAnalyzer::new();

    // Tier 1: FixedScalar
    let strat1 = loop_analyzer.analyze_for_expr(&loop_arena, NodeId(for_nodes[0]), false, false, &mut infer);
    assert!(matches!(
        strat1,
        ForLoweringStrategy::ThreeTierDynamicSchedule {
            schedule: LoopScheduleMode::FixedScalar,
            ret_mode: LoopReturnMode::StatementVoid,
            ..
        }
    ));

    // Tier 2: MutableBounds (`continue a .. b;`)
    let strat2 = loop_analyzer.analyze_for_expr(&loop_arena, NodeId(for_nodes[1]), false, false, &mut infer);
    assert!(matches!(
        strat2,
        ForLoweringStrategy::ThreeTierDynamicSchedule {
            schedule: LoopScheduleMode::MutableBounds | LoopScheduleMode::WorklistStack,
            ..
        }
    ));

    // CollectionYield (`yield k * 2;`) -> `SliceBuffer<T>`
    let strat3 = loop_analyzer.analyze_for_expr(&loop_arena, NodeId(for_nodes[2]), true, false, &mut infer);
    assert!(matches!(
        strat3,
        ForLoweringStrategy::ThreeTierDynamicSchedule {
            ret_mode: LoopReturnMode::CollectionYield { .. },
            ..
        }
    ));

    // 2. Null & Values-First Error Analyzer (`option<T>` / `result<T, E>` `?` propagation & implicit lifting)
    let (_tables, vm, arenas) = bootstrap_all_core_stages();
    let mut types = TypeArena::new();
    for (idx, (_pkg_name, arena)) in arenas.iter().enumerate() {
        populate_type_arena_from_ast(PackageId(idx as u32), arena, &vm, &mut types);
    }
    let opt_int = types.alloc(TypeKind::Option(TypeId::I64));
    let opt_str = types.alloc(TypeKind::Option(TypeId::STR));
    let res_int_str = types.alloc(TypeKind::Result {
        ok: TypeId::I64,
        err: TypeId::STR,
    });

    let mut err_analyzer = ErrorFlowAnalyzer::new();
    let span = ByteSpan::new(0, 10);
    let opt_plan = err_analyzer.analyze_try_operator(opt_int, Some(opt_str), &types, "find_user", span);
    assert_eq!(
        opt_plan,
        TryPropagationPlan::OptionUnwrap {
            unwrapped_ty: TypeId::I64,
            lang_item: LangItem::Option,
        }
    );

    let res_plan = err_analyzer.analyze_try_operator(res_int_str, Some(res_int_str), &types, "parse_id", span);
    assert_eq!(
        res_plan,
        TryPropagationPlan::ResultUnwrap {
            unwrapped_ok_ty: TypeId::I64,
            propagated_err_ty: TypeId::STR,
            lang_item: LangItem::Result,
        }
    );
    assert!(err_analyzer.diagnostics.is_empty());

    assert_eq!(
        ErrorFlowAnalyzer::synthesize_implicit_lift(opt_int, TypeId::I64, false, &types),
        Some(ImplicitLiftKind::OptionSome {
            elem_ty: TypeId::I64
        })
    );
    assert_eq!(
        ErrorFlowAnalyzer::synthesize_implicit_lift(res_int_str, TypeId::I64, false, &types),
        Some(ImplicitLiftKind::ResultOk {
            ok_ty: TypeId::I64,
            err_ty: TypeId::STR
        })
    );
    assert_eq!(
        ErrorFlowAnalyzer::synthesize_implicit_lift(res_int_str, TypeId::STR, true, &types),
        Some(ImplicitLiftKind::ResultErr {
            ok_ty: TypeId::I64,
            err_ty: TypeId::STR
        })
    );

    // 3. Maranget Exhaustiveness Checker (`some`/`none` for `option<T>`, `ok`/`err` for `result<T, E>`)
    assert_eq!(
        MarangetChecker::canonical_variants_for_type(opt_int, &types),
        vec!["some".to_string(), "none".to_string()]
    );
    assert_eq!(
        MarangetChecker::canonical_variants_for_type(res_int_str, &types),
        vec!["ok".to_string(), "err".to_string()]
    );

    let switch_src = r#"
        switch res {
            case ::ok(v): return v;
            case ::fail(e): return 0;
        }
    "#;
    let switch_arena = parse(switch_src);
    let switch_root = switch_arena.node(switch_arena.root);
    let switch_id = NodeId(switch_arena.children_slice(switch_root.lhs, switch_root.rhs)[0]);
    let maranget_diags = MarangetChecker::check_match_expr(
        &switch_arena,
        switch_id,
        MarangetChecker::canonical_variants_for_type(res_int_str, &types),
    );
    assert!(
        maranget_diags.is_empty(),
        "Exhaustive switch on ::ok and ::fail must produce 0 diagnostics: {:?}",
        maranget_diags
    );

    // 4. All 6 Call-Quoting Modes (`!`, `!!`, `!&`, `!^`, `!@`, `!?`)
    let mut quote_engine = CallQuotingEngine::new();
    let p_shallow = quote_engine.resolve_call_quote_projection_type(
        CallQuoteKind::ShallowQuote,
        TypeId::F64,
        &[TypeId::F64, TypeId::F64],
        &mut types,
        span,
    );
    assert_eq!(p_shallow.canonical_type_name, "CallExpr<f64>");
    assert_eq!(p_shallow.lattice, LatticeBits::Value);

    let p_deep = quote_engine.resolve_call_quote_projection_type(
        CallQuoteKind::DeepSplice,
        TypeId::F64,
        &[TypeId::F64],
        &mut types,
        span,
    );
    assert_eq!(p_deep.canonical_type_name, "ast");
    assert_eq!(p_deep.lattice, LatticeBits::Value);

    let p_abi = quote_engine.resolve_call_quote_projection_type(
        CallQuoteKind::AbiFrame,
        TypeId::F64,
        &[TypeId::I64, TypeId::F64],
        &mut types,
        span,
    );
    assert!(p_abi.canonical_type_name.starts_with("frame<"));
    assert_eq!(p_abi.lattice, LatticeBits::Value);

    let p_react = quote_engine.resolve_call_quote_projection_type(
        CallQuoteKind::ReactiveSignal,
        TypeId::F64,
        &[TypeId::F64],
        &mut types,
        span,
    );
    assert_eq!(p_react.canonical_type_name, "Signal<f64>");
    assert_eq!(p_react.lattice, LatticeBits::Object);

    let p_adj = quote_engine.resolve_call_quote_projection_type(
        CallQuoteKind::SymbolicAdjoint,
        TypeId::F64,
        &[TypeId::F64],
        &mut types,
        span,
    );
    assert_eq!(p_adj.canonical_type_name, "Dual<f64, f64>");
    assert_eq!(p_adj.lattice, LatticeBits::Value);

    let p_eff = quote_engine.resolve_call_quote_projection_type(
        CallQuoteKind::EffectDryRun,
        TypeId::I64,
        &[TypeId::I64],
        &mut types,
        span,
    );
    assert_eq!(p_eff.canonical_type_name, "EffectPlan<i64>");
    assert_eq!(p_eff.lattice, LatticeBits::Object);
    assert!(quote_engine.diagnostics.is_empty());
}

#[test]
fn test_5_backend_codegen_parity() {
    use pilot_analysis::types::{HardwarePhysics, RegClass};

    let (_tables, vm, _arenas) = bootstrap_all_core_stages();
    let phys_of = |name: &str| -> HardwarePhysics {
        let spec = vm
            .type_layout(name)
            .unwrap_or_else(|| panic!("Missing layout for {}", name));
        let reg_class = match spec.reg_class {
            RegisterClass::Unit => RegClass::Unit,
            RegisterClass::NoReturn => RegClass::NoReturn,
            RegisterClass::GprSigned => {
                if spec.bit_width == 0 {
                    RegClass::Unit
                } else {
                    RegClass::GprSigned
                }
            }
            RegisterClass::GprUnsigned => {
                if spec.bit_width == 0 {
                    if name == "never" {
                        RegClass::NoReturn
                    } else {
                        RegClass::Unit
                    }
                } else {
                    RegClass::GprUnsigned
                }
            }
            RegisterClass::SimdFloat => RegClass::SimdFloat,
            RegisterClass::SimdVector { lanes } => RegClass::SimdVector { lanes },
            RegisterClass::RawPtr => RegClass::RawPtr,
            RegisterClass::Composite => RegClass::Composite,
        };
        HardwarePhysics {
            reg_class,
            bit_width: spec.bit_width,
        }
    };
    let comp = HardwarePhysics {
        reg_class: RegClass::Composite,
        bit_width: 0,
    };

    // 1. C11/C23 Backend (`CTargetEmitter::lower_physics_to_c`)
    assert_eq!(CTargetEmitter::lower_physics_to_c(phys_of("void"), &[]), "void");
    assert_eq!(CTargetEmitter::lower_physics_to_c(phys_of("never"), &[]), "_Noreturn void");
    assert_eq!(CTargetEmitter::lower_physics_to_c(phys_of("bool"), &[]), "bool");
    assert_eq!(CTargetEmitter::lower_physics_to_c(phys_of("i8"), &[]), "int8_t");
    assert_eq!(CTargetEmitter::lower_physics_to_c(phys_of("i32"), &[]), "int32_t");
    assert_eq!(CTargetEmitter::lower_physics_to_c(phys_of("i64"), &[]), "int64_t");
    assert_eq!(CTargetEmitter::lower_physics_to_c(phys_of("i128"), &[]), "__int128");
    assert_eq!(CTargetEmitter::lower_physics_to_c(phys_of("u64"), &[]), "uint64_t");
    assert_eq!(CTargetEmitter::lower_physics_to_c(phys_of("usize"), &[]), "uint64_t");
    assert_eq!(CTargetEmitter::lower_physics_to_c(phys_of("f32"), &[]), "float");
    assert_eq!(CTargetEmitter::lower_physics_to_c(phys_of("f64"), &[]), "double");
    assert_eq!(
        CTargetEmitter::lower_physics_to_c(
            comp,
            &[("data", "uint8_t*".into()), ("len", "size_t".into())]
        ),
        "struct { uint8_t* data; size_t len; }"
    );
    assert_eq!(
        CTargetEmitter::lower_physics_to_c(
            comp,
            &[
                ("d", "float*".into()),
                ("l", "size_t".into()),
                ("s", "ptrdiff_t".into())
            ]
        ),
        "struct { float* d; size_t l; ptrdiff_t s; }"
    );
    assert_eq!(
        CTargetEmitter::lower_physics_to_c(
            comp,
            &[("current", "int32_t".into()), ("max", "int32_t".into())]
        ),
        "struct { int32_t current; int32_t max; }"
    );

    // 2. LLVM IR Backend (`LlvmEmitter::lower_physics_to_llvm`)
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(phys_of("void"), &[]), "void");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(phys_of("bool"), &[]), "i1");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(phys_of("i8"), &[]), "i8");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(phys_of("i32"), &[]), "i32");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(phys_of("i64"), &[]), "i64");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(phys_of("i128"), &[]), "i128");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(phys_of("f16"), &[]), "half");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(phys_of("f32"), &[]), "float");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(phys_of("f64"), &[]), "double");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(phys_of("f128"), &[]), "fp128");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(phys_of("ptr"), &[]), "ptr");
    assert_eq!(
        LlvmEmitter::lower_physics_to_llvm(comp, &["ptr".into(), "i64".into()]),
        "{ ptr, i64 }"
    );
    assert_eq!(
        LlvmEmitter::lower_physics_to_llvm(comp, &["ptr".into(), "i64".into(), "i64".into()]),
        "{ ptr, i64, i64 }"
    );
    assert_eq!(
        LlvmEmitter::lower_physics_to_llvm(comp, &["i32".into(), "i32".into()]),
        "{ i32, i32 }"
    );
    assert_eq!(
        LlvmEmitter::lower_physics_to_llvm(comp, &["i64".into(), "i64".into()]),
        "{ i64, i64 }"
    );

    // 3. WebAssembly WasmGC + WASI Preview 2 Backend (`WasmEmitter::lower_physics_to_wasm`)
    assert_eq!(WasmEmitter::lower_physics_to_wasm(phys_of("bool"), &[]), "i32");
    assert_eq!(WasmEmitter::lower_physics_to_wasm(phys_of("i32"), &[]), "i32");
    assert_eq!(WasmEmitter::lower_physics_to_wasm(phys_of("i64"), &[]), "i64");
    assert_eq!(WasmEmitter::lower_physics_to_wasm(phys_of("usize"), &[]), "i64");
    assert_eq!(WasmEmitter::lower_physics_to_wasm(phys_of("f32"), &[]), "f32");
    assert_eq!(WasmEmitter::lower_physics_to_wasm(phys_of("f64"), &[]), "f64");
    assert_eq!(
        WasmEmitter::lower_physics_to_wasm(comp, &["i32".into(), "i32".into()]),
        "(i32, i32)"
    );
    assert_eq!(
        WasmEmitter::lower_physics_to_wasm(comp, &["i32".into(), "i32".into(), "i32".into()]),
        "(i32, i32, i32)"
    );

    // 4. Multi-Vendor GPU Backend (`GpuEmitter::lower_physics_to_gpu`)
    assert_eq!(GpuEmitter::lower_physics_to_gpu(phys_of("f32"), &[]), "float");
    assert_eq!(GpuEmitter::lower_physics_to_gpu(phys_of("u32"), &[]), "uint");
    assert_eq!(GpuEmitter::lower_physics_to_gpu(phys_of("i64"), &[]), "long");
    assert_eq!(
        GpuEmitter::lower_physics_to_gpu(phys_of("ptr"), &[("ptr", "float".into())]),
        "global float*"
    );
    assert_eq!(
        GpuEmitter::lower_physics_to_gpu(
            comp,
            &[("cur", "int".into()), ("max", "int".into())]
        ),
        "struct { int cur; int max; }"
    );

    // 5. Direct Native Assembly Backend (`AsmEmitter`)
    let sample_arena = parse("fn add_vec(a: i64, b: i64) -> i64 { return a + b; }");
    let asm_out = AsmEmitter::emit_assembly(&sample_arena, "test_mod", 3, true)
        .expect("AsmEmitter must synthesize assembly cleanly");
    assert!(asm_out.contains(".text"));
    assert!(asm_out.contains("test_mod.pilot"));
}

#[test]
fn test_15_new_symbols_and_alu_operators() {
    use pilot_analysis::types::{HardwarePhysics, RegClass, StructuralShape};
    let src = r#"
        pub fn test_ops() {
            let v4: f32@4;
            let g:  f64[4, 4];
            let r:  u8[:];
            let s:  i64^;
            let p:  @u8;

            var `type`: i64 = 10;
            var `fn`:   i64 = 20;
            `type` :=: `fn`;

            let cmp_res = `type` <=> `fn`;
            let sat_a   = `type` +| `fn`;
            let sat_s   = `type` -| `fn`;
            let sat_m   = `type` *| `fn`;
            let wrp_a   = `type` +% `fn`;
            let wrp_s   = `type` -% `fn`;
            let wrp_m   = `type` *% `fn`;
            let chk_a   = `type` +? `fn`;
            let chk_s   = `type` -? `fn`;
            let chk_m   = `type` *? `fn`;
            let euc_m   = `type` %% `fn`;
            let open_r  = 0 >..< 10;
            let after_r = 5 >..;
        }
    "#;
    let arena = parse(src);
    let mut engine = TypeInferenceEngine::new();
    let _ = engine.synth(&arena, arena.root);
    assert!(
        engine.diagnostics.is_empty(),
        "Expected 0 diagnostics, got: {:?}",
        engine.diagnostics
    );

    let root = arena.node(arena.root);
    let fn_id = NodeId(arena.children_slice(root.lhs, root.rhs)[0]);
    let fn_node = arena.node(fn_id);
    let body_id = NodeId(arena.children_slice(fn_node.rhs, fn_node.flags as u32)[0]);
    let body_node = arena.node(body_id);
    let stmts = arena.children_slice(body_node.lhs, body_node.rhs);

    // 1. `f32@4` -> `simd<f32, 4>` (`RegClass::SimdVector { lanes: 4 }`, 128 bits, 16 bytes)
    let v4_ty = engine.inferred_node_types[&stmts[0]];
    assert_eq!(
        engine.types.type_physics(v4_ty),
        HardwarePhysics {
            reg_class: RegClass::SimdVector { lanes: 4 },
            bit_width: 128
        }
    );
    assert_eq!(engine.types.type_size(v4_ty), 16);

    // 2. `f64[4, 4]` -> `grid<f64, 4, 4>` (`4 * 4 * 8 = 128` bytes, `0b00` `IValue`)
    let g_ty = engine.inferred_node_types[&stmts[1]];
    assert_eq!(engine.types.type_size(g_ty), 128);
    assert_eq!(engine.types.type_lattice(g_ty), LatticeBits::Value);

    // 3. `u8[:]` -> `ring<u8>` (`0b01` `IReference`, 24 bytes)
    let r_ty = engine.inferred_node_types[&stmts[2]];
    assert_eq!(engine.types.type_lattice(r_ty), LatticeBits::Reference);
    assert_eq!(engine.types.type_size(r_ty), 24);

    // 4. `i64^` -> `signal<i64>` (`0b10` `IObject`)
    let s_ty = engine.inferred_node_types[&stmts[3]];
    assert_eq!(engine.types.type_lattice(s_ty), LatticeBits::Object);

    // 5. `@u8` -> `pin<u8>` (`0b11` `IGuard`)
    let p_ty = engine.inferred_node_types[&stmts[4]];
    assert_eq!(engine.types.type_lattice(p_ty), LatticeBits::Guard);

    // 6. `<=>` -> `order` (`8`-bit signed `RegClass::GprSigned`)
    let cmp_ty = engine.inferred_node_types[&stmts[8]];
    assert_eq!(
        engine.types.type_physics(cmp_ty),
        HardwarePhysics {
            reg_class: RegClass::GprSigned,
            bit_width: 8
        }
    );

    // 7. `+?` -> `option<i64>` (`TaggedOutcome`)
    let chk_ty = engine.inferred_node_types[&stmts[15]];
    assert!(matches!(
        engine.types.type_shape(chk_ty),
        StructuralShape::TaggedOutcome { err_ty: None, .. }
    ));

    // 8. `>..<` -> `open<i64>` (`BoundsPair`)
    let open_ty = engine.inferred_node_types[&stmts[19]];
    assert!(matches!(
        engine.types.type_shape(open_ty),
        StructuralShape::BoundsPair {
            inclusive: false,
            ..
        }
    ));
}

#[test]
fn test_universal_offset_lens_swizzle_and_slicemap() {
    use pilot_analysis::types::{HardwarePhysics, RegClass, StructuralShape};
    let src = r#"
        struct Vertex : IValue {
            id:  u32;
            pos: f32@3;
        }
        struct PosOnly : IValue {
            pos: f32@3;
        }
        pub fn test_lenses(verts: Vertex[], vec: f32@4) {
            let pos_view = verts ~> .pos;
            let xy_map   = verts ~> (.id, .pos);
            let zyx      = vec ~> (.2, .1, .0);
            let pts      = verts as view<PosOnly>;
        }
    "#;
    let arena = parse(src);
    let mut engine = TypeInferenceEngine::new();
    let _ = engine.synth(&arena, arena.root);
    assert!(
        engine.diagnostics.is_empty(),
        "Expected 0 diagnostics on 0b00 IValue lens & view casts: {:?}",
        engine.diagnostics
    );

    let root = arena.node(arena.root);
    let fn_id = NodeId(arena.children_slice(root.lhs, root.rhs)[2]);
    let fn_node = arena.node(fn_id);
    let body_id = NodeId(arena.children_slice(fn_node.rhs, fn_node.flags as u32)[0]);
    let body_node = arena.node(body_id);
    let stmts = arena.children_slice(body_node.lhs, body_node.rhs);

    // 1. `verts ~> .pos` -> `stride<f32@3>` (`0b01` `IReference`, 24 bytes, `StridedView`)
    let pos_view_ty = engine.inferred_node_types[&stmts[0]];
    assert_eq!(engine.types.type_lattice(pos_view_ty), LatticeBits::Reference);
    assert_eq!(engine.types.type_size(pos_view_ty), 24);
    assert!(matches!(
        engine.types.type_shape(pos_view_ty),
        StructuralShape::StridedView { .. }
    ));

    // 2. `verts ~> (.id, .pos)` -> `map<u32, f32@3>` (`0b01` `IReference`, 48 bytes)
    let map_ty = engine.inferred_node_types[&stmts[1]];
    assert_eq!(engine.types.type_lattice(map_ty), LatticeBits::Reference);
    assert_eq!(engine.types.type_size(map_ty), 48);

    // 3. `vec ~> (.2, .1, .0)` -> `f32@3` (`RegClass::SimdVector { lanes: 3 }`)
    let zyx_ty = engine.inferred_node_types[&stmts[2]];
    assert_eq!(
        engine.types.type_physics(zyx_ty),
        HardwarePhysics {
            reg_class: RegClass::SimdVector { lanes: 3 },
            bit_width: 3 * 32
        }
    );

    // 4. `verts as view<PosOnly>` -> `stride<PosOnly>` (`0b01` `IReference`, 24 bytes)
    let pts_ty = engine.inferred_node_types[&stmts[3]];
    assert_eq!(engine.types.type_lattice(pts_ty), LatticeBits::Reference);
    assert_eq!(engine.types.type_size(pts_ty), 24);

    // 5. Negative test: `~>` or `as view` on `0b10` `IObject` must emit `LatticeContractMismatch`!
    let bad_src = r#"
        struct Bad : IObject {
            ptr: ptr<u8>;
            ~self() {}
        }
        pub fn test_bad(items: Bad[]) {
            let p = items ~> .ptr;
        }
    "#;
    let bad_arena = parse(bad_src);
    let mut bad_engine = TypeInferenceEngine::new();
    let _ = bad_engine.synth(&bad_arena, bad_arena.root);
    assert!(
        !bad_engine.diagnostics.is_empty(),
        "OffsetLens '~>' on 0b10 IObject must be rejected!"
    );
}

#[test]
fn test_compile_time_units_and_zero_overhead_erasure() {
    use pilot_analysis::types::{HardwarePhysics, RegClass};
    let src = r#"
        pub unit Meters(f64);
        pub unit Seconds(f64);
        pub unit Velocity(f64) = Meters / Seconds;

        pub fn compute_speed(d: Meters, t: Seconds) -> Velocity {
            let d2: Meters = d + d;
            let v: Velocity = d2 / t;
            return v;
        }
    "#;
    let arena = parse(src);
    let mut engine = TypeInferenceEngine::new();
    let _ = engine.synth(&arena, arena.root);
    assert!(
        engine.diagnostics.is_empty(),
        "Valid dimensional algebra must pass with 0 diagnostics: {:?}",
        engine.diagnostics
    );

    // Verify 0-byte wrapper erasure: `Meters` has identical hardware physics to `f64` (8 bytes, SimdFloat)
    let m_ty = *engine.type_defs.get("Meters").expect("Meters unit registered");
    assert_eq!(engine.types.type_size(m_ty), 8);
    assert_eq!(
        engine.types.type_physics(m_ty),
        HardwarePhysics {
            reg_class: RegClass::SimdFloat,
            bit_width: 64
        }
    );

    // Negative test: adding `Meters + Seconds` must emit `DIM001`!
    let bad_unit_src = r#"
        pub unit Meters(f64);
        pub unit Seconds(f64);
        pub fn bad_add(d: Meters, t: Seconds) {
            let invalid = d + t;
        }
    "#;
    let bad_arena = parse(bad_unit_src);
    let mut bad_engine = TypeInferenceEngine::new();
    let _ = bad_engine.synth(&bad_arena, bad_arena.root);
    assert!(
        bad_engine.diagnostics.iter().any(|d| d.code == "DIM001"),
        "Adding Meters + Seconds must emit DIM001 diagnostic! Got: {:?}",
        bad_engine.diagnostics
    );
}
