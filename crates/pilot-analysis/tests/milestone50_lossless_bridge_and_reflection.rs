// ==============================================================================
// tests/milestone50_lossless_bridge_and_reflection.rs
// Milestone 50 End-to-End Verification Suite:
//   50.1: Lossless 1-to-1 `flatten_ast` (CallQuoteKind, child links, attrs, flags)
//   50.2: Open Keyword & Category Reflection (`0..14` + user categories `>= 100`)
//   50.3: `bootstrap.pilot`, real `[EnforceCoreNaming]`, `LatticeContractMismatch`,
//         span-accurate `AnalysisPipeline`, and `selfhost/attr_weaver.pilot`
//   50.4: `.pilc` snapshot caching & `PilcBinary::load_from_bytes` parity
// ==============================================================================

use pilot_analysis::{
    analyze_physics, flatten_ast, handle_engine_intrinsic, load_or_compile_analyzer_pilc,
    AnalysisPipeline, BootstrapSnapshot, PhysicsRegistry,
};
use pilot_ast::{CallQuoteKind, NodeKind};
use pilot_parser::parse;
use pilot_rt::PilotVal;
use pilot_vm::pilc::{PilcBinary, PILC_V3_MAGIC};
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[test]
fn test_50_1_lossless_flatten_ast_call_quotes_attrs_bases_and_multi_attr_layout() {
    let snippet = r#"
        [Lang(5), Bits(32, 2)]
        pub partial struct Scalar32 : IValue {}

        [Clamp(-25, 0.5, true, "bounds", c_abi)]
        pub struct PhysicsBody : IObject, IEquatable {
            pub mass: Scalar32;
            ~self() {}
        }

        pub fn step(x: Scalar32) -> Scalar32 {
            return x;
        }

        pub fn exercise_quotes(a: Scalar32) -> Scalar32 {
            let q1 = step!(a);
            let q2 = step!!(a);
            let q3 = step!&(a);
            let q4 = step!^(a);
            let q5 = step!@(a);
            let q6 = step!?(a);
            let arr = [a, a];
            let elem = arr[0];
            let neg = -a;
            return a;
        }
    "#;

    let arena = parse(snippet);
    let mut reg = analyze_physics(&arena, arena.root, None);

    // 1. Multi-attribute [Lang(5), Bits(32, 2)] must infer 4-byte (32-bit) scalar, NOT 5-bit!
    let scalar32_idx = arena
        .nodes
        .iter()
        .position(|n| n.node_kind() == NodeKind::StructDecl && arena.interner.resolve(pilot_intern::SymbolId(n.lhs)) == "Scalar32")
        .expect("Scalar32 node");
    let scalar32_lid = reg.node_layouts[scalar32_idx].expect("Scalar32 layout bound");
    let scalar32_layout = reg.layouts.get(scalar32_lid).unwrap();
    assert_eq!(scalar32_layout.byte_size, 4, "[Lang(5), Bits(32, 2)] must be 4 bytes");
    assert_eq!(scalar32_layout.byte_align, 4);

    // Check partial + pub decl_flags on Scalar32
    let dflags_s32 = handle_engine_intrinsic(&mut reg, "ast_decl_flags", &[PilotVal::from_int(scalar32_idx as i64)])
        .unwrap()
        .as_int()
        .unwrap();
    assert_ne!(dflags_s32 & 16, 0, "Scalar32 must preserve partial flag (bit 4)");

    // 2. Check PhysicsBody structured attribute name, 5 heterogeneous args (-25, 0.5, true, "bounds", c_abi), bases, and ~self() flag
    let body_idx = arena
        .nodes
        .iter()
        .position(|n| n.node_kind() == NodeKind::StructDecl && arena.interner.resolve(pilot_intern::SymbolId(n.lhs)) == "PhysicsBody")
        .expect("PhysicsBody node");

    let dflags_body = handle_engine_intrinsic(&mut reg, "ast_decl_flags", &[PilotVal::from_int(body_idx as i64)])
        .unwrap()
        .as_int()
        .unwrap();
    assert_ne!(dflags_body & 1024, 0, "PhysicsBody must preserve ~self() destructor flag (bit 10)");

    let attr_decl_cnt = handle_engine_intrinsic(&mut reg, "ast_attr_decl_count", &[PilotVal::from_int(body_idx as i64)])
        .unwrap()
        .as_int()
        .unwrap();
    assert_eq!(attr_decl_cnt, 1);

    let attr_name_sym = handle_engine_intrinsic(&mut reg, "ast_attr_name", &[PilotVal::from_int(body_idx as i64), PilotVal::from_int(0)])
        .unwrap()
        .as_int()
        .unwrap();
    let attr_name = handle_engine_intrinsic(&mut reg, "sym_str", &[PilotVal::from_int(attr_name_sym)])
        .unwrap()
        .to_string_repr();
    assert_eq!(attr_name, "Clamp");

    let arg_neg = handle_engine_intrinsic(&mut reg, "ast_attr_decl_arg", &[PilotVal::from_int(body_idx as i64), PilotVal::from_int(0), PilotVal::from_int(0)])
        .unwrap()
        .as_int()
        .unwrap();
    assert_eq!(arg_neg, -25, "Negative integer attribute arg must round-trip losslessly");

    let arg_float_bits = handle_engine_intrinsic(&mut reg, "ast_attr_decl_arg", &[PilotVal::from_int(body_idx as i64), PilotVal::from_int(0), PilotVal::from_int(1)])
        .unwrap()
        .as_int()
        .unwrap() as u64;
    assert_eq!(f64::from_bits(arg_float_bits), 0.5, "Float attribute arg must round-trip losslessly as IEEE-754 bits");

    let arg_bool = handle_engine_intrinsic(&mut reg, "ast_attr_decl_arg", &[PilotVal::from_int(body_idx as i64), PilotVal::from_int(0), PilotVal::from_int(2)])
        .unwrap()
        .as_int()
        .unwrap();
    assert_eq!(arg_bool, 1, "Bool attribute arg must round-trip losslessly");

    let arg_str_sym = handle_engine_intrinsic(&mut reg, "ast_attr_decl_arg", &[PilotVal::from_int(body_idx as i64), PilotVal::from_int(0), PilotVal::from_int(3)])
        .unwrap()
        .as_int()
        .unwrap();
    let arg_str = handle_engine_intrinsic(&mut reg, "sym_str", &[PilotVal::from_int(arg_str_sym)])
        .unwrap()
        .to_string_repr();
    assert_eq!(arg_str, "bounds");

    let base_cnt = handle_engine_intrinsic(&mut reg, "ast_base_count", &[PilotVal::from_int(body_idx as i64)])
        .unwrap()
        .as_int()
        .unwrap();
    assert_eq!(base_cnt, 2);
    let base0_sym = handle_engine_intrinsic(&mut reg, "ast_base_at", &[PilotVal::from_int(body_idx as i64), PilotVal::from_int(0)])
        .unwrap()
        .as_int()
        .unwrap();
    let base0 = handle_engine_intrinsic(&mut reg, "sym_str", &[PilotVal::from_int(base0_sym)])
        .unwrap()
        .to_string_repr();
    assert_eq!(base0, "IObject");

    // 3. Verify all 6 CallQuoteKind flags (!, !!, !&, !^, !@, !?) & ListExpr/IndexExpr/UnaryExpr child links
    let mut seen_quote_kinds = Vec::new();
    let mut saw_list_children = false;
    let mut saw_index_children = false;
    let mut saw_unary_child = false;

    for nid in 0..reg.arena.nodes.len() {
        let orig = handle_engine_intrinsic(&mut reg, "ast_orig_kind", &[PilotVal::from_int(nid as i64)])
            .unwrap()
            .as_int()
            .unwrap() as u16;
        let nflags = handle_engine_intrinsic(&mut reg, "ast_node_flags", &[PilotVal::from_int(nid as i64)])
            .unwrap()
            .as_int()
            .unwrap();
        let cc = handle_engine_intrinsic(&mut reg, "ast_child_count", &[PilotVal::from_int(nid as i64)])
            .unwrap()
            .as_int()
            .unwrap();

        if orig == NodeKind::CallExpr as u16 && nflags > 0 {
            seen_quote_kinds.push(nflags as u8);
        }
        if orig == NodeKind::ListExpr as u16 && cc == 2 {
            saw_list_children = true;
        }
        if orig == NodeKind::IndexExpr as u16 && cc == 2 {
            saw_index_children = true;
        }
        if orig == NodeKind::UnaryExpr as u16 && cc == 1 {
            saw_unary_child = true;
        }
    }

    assert_eq!(
        seen_quote_kinds,
        vec![
            CallQuoteKind::ShallowQuote as u8,
            CallQuoteKind::DeepSplice as u8,
            CallQuoteKind::AbiFrame as u8,
            CallQuoteKind::ReactiveSignal as u8,
            CallQuoteKind::SymbolicAdjoint as u8,
            CallQuoteKind::EffectDryRun as u8,
        ],
        "All 6 CallQuoteKind projections must be preserved losslessly in RawAstNode::node_flags"
    );
    assert!(saw_list_children, "ListExpr child links must be projected");
    assert!(saw_index_children, "IndexExpr child links must be projected");
    assert!(saw_unary_child, "UnaryExpr child link must be projected");
}

#[test]
fn test_50_2_open_keyword_category_reflection_and_user_categories() {
    let empty_arena = parse("");
    let (flat, syms) = flatten_ast(&empty_arena, empty_arena.root);
    let mut reg = PhysicsRegistry::new(flat, syms);

    // Mount stdlib/core/keywords.pilot dynamically
    let kw_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../stdlib/core/keywords.pilot")
        .to_string_lossy()
        .to_string();
    let total_nodes = handle_engine_intrinsic(&mut reg, "mount_unit", &[PilotVal::from_str(&kw_path)])
        .unwrap()
        .as_int()
        .unwrap();
    assert!(total_nodes > 50, "Mounting keywords.pilot must populate RawAstArena");

    // Reflect standard Category 1 (Control Flow) and Category 8 (Design by Contract)
    let cat1_scope = handle_engine_intrinsic(&mut reg, "query_registered_symbols", &[PilotVal::from_int(1), PilotVal::from_int(1)])
        .unwrap()
        .to_string_repr();
    assert_eq!(cat1_scope, "keyword.control.flow.pilot");

    let cat1_badge = handle_engine_intrinsic(&mut reg, "query_registered_symbols", &[PilotVal::from_int(1), PilotVal::from_int(2)])
        .unwrap()
        .to_string_repr();
    assert_eq!(cat1_badge, "Control Flow");

    let cat1_words = handle_engine_intrinsic(&mut reg, "query_registered_symbols", &[PilotVal::from_int(1)])
        .unwrap()
        .to_string_repr();
    assert!(cat1_words.split('|').any(|w| w == "guard"), "Category 1 must reflect 'guard': {}", cat1_words);
    assert!(cat1_words.split('|').any(|w| w == "switch"), "Category 1 must reflect 'switch': {}", cat1_words);

    let guard_doc = handle_engine_intrinsic(
        &mut reg,
        "query_registered_symbols",
        &[PilotVal::from_int(1), PilotVal::from_int(3), PilotVal::from_str("guard")],
    )
    .unwrap()
    .to_string_repr();
    assert!(!guard_doc.is_empty(), "Keyword hover doc for 'guard' must be reflected losslessly");

    // Dynamically register a user-defined Category >= 100 (e.g., GPU Kernel DSL = 101)
    handle_engine_intrinsic(
        &mut reg,
        "register_keyword_category",
        &[
            PilotVal::from_int(101),
            PilotVal::from_str("keyword.other.gpu.pilot"),
            PilotVal::from_str("GPU Kernel"),
            PilotVal::from_int(14),
        ],
    )
    .unwrap();
    handle_engine_intrinsic(
        &mut reg,
        "register_keyword",
        &[
            PilotVal::from_int(101),
            PilotVal::from_str("workgroup_barrier"),
            PilotVal::from_str("Synchronizes all lanes in the active GPU workgroup."),
        ],
    )
    .unwrap();

    let cat101_words = handle_engine_intrinsic(&mut reg, "query_registered_symbols", &[PilotVal::from_int(101)])
        .unwrap()
        .to_string_repr();
    assert_eq!(cat101_words, "workgroup_barrier");
    let cat101_badge = handle_engine_intrinsic(&mut reg, "query_registered_symbols", &[PilotVal::from_int(101), PilotVal::from_int(2)])
        .unwrap()
        .to_string_repr();
    assert_eq!(cat101_badge, "GPU Kernel");
}

#[test]
fn test_50_3_bootstrap_manifest_enforce_core_naming_and_attr_weaver() {
    // 1. BootstrapSnapshot driven by stdlib/core/bootstrap.pilot
    let snap = BootstrapSnapshot::global();
    assert_eq!(snap.arenas.len(), 8, "All 8 core bootstrap stages must load from bootstrap.pilot");
    assert_eq!(snap.arenas[0].0, "Pilot.Core.Genesis");
    assert_eq!(snap.arenas[7].0, "Pilot.Core.Meta");

    // 2. Valid [EnforceCoreNaming] core files must pass with 0 errors
    for rel in [
        "stdlib/core/bootstrap.pilot",
        "stdlib/core/naming_attr.pilot",
        "stdlib/core/mem.pilot",
        "stdlib/core/meta.pilot",
    ] {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel);
        let src = std::fs::read_to_string(&p).expect("read core file");
        let mut tables = snap.tables.clone();
        let mut cvm = snap.vm.clone();
        let arena = pilot_analysis::eval_file_with_tables(&src, &mut tables, &mut cvm).expect("parse core file");
        let res = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(
            res.is_ok(),
            "Core file '{}' must satisfy [EnforceCoreNaming] & lattice laws with 0 errors, got: {:?}",
            rel,
            res.diagnostics
        );
    }

    // 3. Invalid declarations in an [EnforceCoreNaming] package must emit span-accurate diagnostics
    let bad_core_src = r#"
        [EnforceCoreNaming]
        pub pkg Pilot.Core.BrokenTest;

        pub interface BadInterfaceName {}
        pub struct lowercase_struct : IValue {}
        pub struct BadClamp : IAttribute, IValue {}
        pub struct BadRef : IReference {}
        pub struct BadLock : IGuard {}
        pub struct IllegalValueWithDtor : IValue {
            ~self() {}
        }
    "#;
    let bad_arena = parse(bad_core_src);
    let bad_res = AnalysisPipeline::analyze(&bad_arena, bad_arena.root);
    assert_eq!(bad_res.diagnostics.len(), 6, "Expected 6 exact naming/lattice diagnostics, got: {:?}", bad_res.diagnostics);
    assert!(bad_res.diagnostics.iter().all(|d| d.span.start > 0 && d.span.end > d.span.start), "All diagnostics must carry real AST ByteSpans");
    assert!(bad_res.diagnostics.iter().any(|d| d.code == "EnforceCoreNaming" && d.message.contains("InterfacePrefix")));
    assert!(bad_res.diagnostics.iter().any(|d| d.code == "EnforceCoreNaming" && d.message.contains("StructPascalCase")));
    assert!(bad_res.diagnostics.iter().any(|d| d.code == "EnforceCoreNaming" && d.message.contains("AttributeSuffix")));
    assert!(bad_res.diagnostics.iter().any(|d| d.code == "EnforceCoreNaming" && d.message.contains("ReferenceViewSuffix")));
    assert!(bad_res.diagnostics.iter().any(|d| d.code == "EnforceCoreNaming" && d.message.contains("ScopedGuardSuffix")));
    assert!(bad_res.diagnostics.iter().any(|d| d.code == "LatticeContractMismatch" && d.message.contains("IllegalValueWithDtor")));

    // 4. Execute selfhost/attr_weaver.pilot directly on PilVirtualMachine
    let weaver_target_src = r#"
        [Rule(10)]
        pub struct PhysicsStepSystem {}

        [OnChange(20), Clamp(100)]
        pub fn update_velocity() -> i64 { return 0; }

        [Clamp(10)]
        pub interface IInvalidReactiveTarget {}
    "#;
    let w_arena = parse(weaver_target_src);
    let (w_flat, w_syms) = flatten_ast(&w_arena, w_arena.root);
    let mut w_reg = PhysicsRegistry::new(w_flat, w_syms);

    let fn_nid = w_arena
        .nodes
        .iter()
        .position(|n| n.node_kind() == NodeKind::FnDecl)
        .unwrap();

    let weaver_src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../selfhost/attr_weaver.pilot"),
    )
    .expect("read selfhost/attr_weaver.pilot");

    let run_weaver_call = |reg: &mut PhysicsRegistry, expr: &str| -> i64 {
        let script = format!("{}\n{};\n", weaver_src, expr);
        let p = parse(&script);
        let mut vm = PilVirtualMachine::new();
        vm.load_binary(BytecodeCompiler::compile(&p, p.root));
        let ptr = reg as *mut PhysicsRegistry;
        vm.engine_handler = Some(Box::new(move |op, args| handle_engine_intrinsic(unsafe { &mut *ptr }, op, args)));
        vm.run().as_int().unwrap_or(-999)
    };

    let top_idx = run_weaver_call(&mut w_reg, &format!("highest_weight_attr_index({})", fn_nid));
    assert_eq!(top_idx, 1, "Clamp(weight=100) at index 1 must outrank OnChange(weight=20) at index 0");

    let ecs_count = run_weaver_call(&mut w_reg, "weave_unit_attributes()");
    assert_eq!(ecs_count, 1, "Expected 1 [Rule] ECS system discovered");
    assert!(
        w_reg.diagnostics.iter().any(|d| d.message.contains("ATTR001")),
        "Attaching reactive [Clamp] to an interface must emit ATTR001, got: {:?}",
        w_reg.diagnostics
    );
}

#[test]
fn test_50_4_pilc_snapshot_roundtrip_and_loader() {
    let bin = load_or_compile_analyzer_pilc();
    let bytes = bin.serialize();
    assert_eq!(&bytes[0..4], PILC_V3_MAGIC, ".pilc snapshot must start with PIL3 magic");

    let t0 = std::time::Instant::now();
    let reloaded = PilcBinary::load_from_bytes(&bytes).expect("PilcBinary::load_from_bytes must succeed");
    let elapsed = t0.elapsed();
    assert!(
        elapsed.as_millis() < 10,
        ".pilc snapshot deserialization must be sub-millisecond/near-instant, took {:?}",
        elapsed
    );
    assert_eq!(reloaded.instructions.len(), bin.instructions.len());
    assert_eq!(reloaded.constants.len(), bin.constants.len());
}
