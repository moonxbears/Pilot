// ==============================================================================
// crates/pilot-analysis/tests/milestone50_lossless_bridge_and_reflection.rs
// Milestone 50 — Uncheatable Adversarial Verification Suite
//
// Required `.pilot` Contracts (all tested simultaneously on randomized inputs):
//   1. `stdlib/core/bootstrap.pilot`:
//      - `pub pure fn bootstrap_stage_count() -> i64` (derived via `.len`, never hardcoded `8`)
//      - `pub pure fn bootstrap_stage_path(idx: i64) -> str`
//      - `pub pure fn bootstrap_stage_ring(idx: i64) -> i64`
//      - `pub pure fn bootstrap_stage_at(idx: i64) -> BootstrapStage`
//   2. `selfhost/diagnostics.pilot` & `selfhost/analyzer.pilot` (invoked by `AnalysisPipeline::analyze`):
//      - `pub fn verify_unit_diagnostics() -> i64`
//      - Enforces universal `LatticeContractMismatch` (`: IValue` declaring `~self()`)
//      - When `[EnforceCoreNaming]` is present in unit, enforces all taxonomy rules with real AST spans:
//        * `InterfacePrefix`: `interface` must start with `'I'` + uppercase ASCII
//        * `StructPascalCase`: non-`[Lang]` non-`partial` `struct` must start with uppercase ASCII
//        * `ReservedInterfacePrefix`: `struct` must NOT start with `'I'` + uppercase ASCII
//        * `LangPrimitiveLowercase`: non-`partial` `[Lang]` `struct` must be all lowercase/digits/`_`
//        * `AttributeSuffix`: `IAttribute` `struct` must end with `"Attr"`
//        * `ReferenceViewSuffix`: `0b01` `IReference` `struct` must end with `"View"|"Lens"|"Cursor"|"Enumerator"`
//        * `ScopedGuardSuffix`: `0b11` `IGuard` `struct` must end with `"Guard"|"Scope"|"Lease"`
//   3. `selfhost/attr_weaver.pilot`:
//      - `pub fn highest_weight_attr_index(nid: i64) -> i64`
//      - `pub fn weave_sorted_attr_names(nid: i64) -> str`
//        Returns comma-separated attribute names attached to `nid` sorted in strictly
//        DESCENDING order of `weight: i32` (last arg, default `10` if 0 args), preserving
//        original attachment index order on equal weights.
//      - `pub fn weave_unit_attributes() -> i64`
//        Returns total Pillar-7 (`[Rule]` / `IEcsRule`) systems in unit and emits `ATTR001`
//        (reactive/formula attr on `interface`) and `ATTR002` (interceptor attr on field/param).
//   4. `selfhost/analyzer.pilot`:
//      - `pub fn compute_field_projection_offset(agg_layout_id: i64, field_idx: i64) -> i64`
//      - `pub fn compute_soa_projection(elem_layout_id: i64, ptr_bit_width: i64) -> i64`
//      - `pub fn verify_capability_transition(from_cap: i64, to_cap: i64) -> bool`
// ==============================================================================

use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_analysis::{
    analyze_physics, eval_file_with_tables, flatten_ast, handle_engine_intrinsic,
    load_or_compile_analyzer_pilc, AnalysisPipeline, BootstrapSnapshot, CapabilityBits, LayoutId,
    PhysicsGeometry, PhysicsRegistry,
};
use pilot_ast::{CallQuoteKind, NodeKind};
use pilot_intern::SymbolId;
use pilot_parser::parse;
use pilot_rt::PilotVal;
use pilot_vm::pilc::{PilcBinary, PILC_V3_MAGIC};
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 50,
    test_file_rel: "crates/pilot-analysis/tests/milestone50_lossless_bridge_and_reflection.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "stdlib/core/bootstrap.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "bootstrap_stage_count",
                "bootstrap_stage_path",
                "bootstrap_stage_ring",
                "bootstrap_stage_at",
            ],
            required_code_markers: &[".len"],
            forbidden_code_markers: &["return 8;"],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/diagnostics.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["verify_unit_diagnostics"],
            required_code_markers: &["__engine", "EnforceCoreNaming", "LatticeContractMismatch"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/attr_weaver.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "highest_weight_attr_index",
                "weave_sorted_attr_names",
                "weave_unit_attributes",
            ],
            required_code_markers: &["__engine", "ATTR001", "ATTR002"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/analyzer.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "compute_field_projection_offset",
                "compute_soa_projection",
                "verify_capability_transition",
            ],
            required_code_markers: &["__engine", "layout_field_offset"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        let seed = nanos ^ ((std::process::id() as u64).rotate_left(17)) ^ salt;
        Self(if seed == 0 { 0xCAFE_BABE_DEAD_BEEF } else { seed })
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn range(&mut self, lo: i64, hi_inclusive: i64) -> i64 {
        let span = (hi_inclusive - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }

    fn alpha_tag(&mut self, len: usize) -> String {
        const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
        (0..len)
            .map(|_| LOWER[(self.next_u64() as usize) % LOWER.len()] as char)
            .collect()
    }

    fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = (self.next_u64() as usize) % (i + 1);
            slice.swap(i, j);
        }
    }
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

fn run_vm_with_registry(script: &str, reg: &mut PhysicsRegistry) -> PilotVal {
    let arena = parse(script);
    assert!(
        arena.diagnostics.is_empty(),
        "Pilot script failed to parse cleanly: {:?}",
        arena.diagnostics
    );
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    let ptr = reg as *mut PhysicsRegistry;
    vm.engine_handler = Some(Box::new(move |op, args| {
        handle_engine_intrinsic(unsafe { &mut *ptr }, op, args)
    }));
    vm.run()
}

#[test]
pub fn test_50_1_lossless_flatten_ast_randomized_quotes_attrs_bases_and_layouts() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_50_1_lossless_flatten_ast_randomized_quotes_attrs_bases_and_layouts",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5001_A11CE);
    let bit_widths = [8u32, 16, 32, 64, 128];

    for trial in 0..8 {
        let width_bits = bit_widths[(rng.next_u64() as usize) % bit_widths.len()];
        let expected_bytes = width_bits / 8;
        let expected_align = expected_bytes.min(16);
        let lang_slot = rng.range(1, 29);

        let scalar_name = format!("Scalar{}{}", width_bits, rng.alpha_tag(5));
        let body_name = format!("Body{}{}", trial, rng.alpha_tag(5));
        let attr_name = format!("CustomMeta{}", rng.alpha_tag(4));
        let neg_arg = rng.range(-9999, -2);
        let float_int = rng.range(0, 25) * 2 + 1;
        let float_val = (float_int as f64) * 0.25;
        let bool_val = (rng.next_u64() & 1) == 0;
        let str_arg = format!("tag_{}_{}", trial, rng.alpha_tag(6));
        let ident_arg = format!("abi_{}", rng.alpha_tag(4));
        let base_extra = format!("ICustom{}{}", trial, rng.alpha_tag(4));
        let list_len = rng.range(3, 7) as usize;

        let mut quote_modes = vec![
            ("!", CallQuoteKind::ShallowQuote as u8),
            ("!!", CallQuoteKind::DeepSplice as u8),
            ("!&", CallQuoteKind::AbiFrame as u8),
            ("!^", CallQuoteKind::ReactiveSignal as u8),
            ("!@", CallQuoteKind::SymbolicAdjoint as u8),
            ("!?", CallQuoteKind::EffectDryRun as u8),
        ];
        rng.shuffle(&mut quote_modes);

        let mut quote_stmts = String::new();
        for (idx, (sigil, _)) in quote_modes.iter().enumerate() {
            quote_stmts.push_str(&format!("let q_{} = step{}(a);\n", idx, sigil));
        }
        let list_elems = vec!["a"; list_len].join(", ");

        let snippet = format!(
            r#"
            [Lang({lang_slot}), Bits({width_bits}, 2)]
            pub partial struct {scalar_name} : IValue {{}}

            [{attr_name}({neg_arg}, {float_val:?}, {bool_val}, "{str_arg}", {ident_arg})]
            pub struct {body_name} : IObject, {base_extra} {{
                pub mass: {scalar_name};
                ~self() {{}}
            }}

            pub fn step(x: {scalar_name}) -> {scalar_name} {{
                return x;
            }}

            pub fn run_quotes(a: {scalar_name}) -> {scalar_name} {{
                {quote_stmts}
                let items = [{list_elems}];
                let pick = items[1];
                let inv = -a;
                return a;
            }}
            "#
        );

        let arena = parse(&snippet);
        assert!(
            arena.diagnostics.is_empty(),
            "Trial {} parse diagnostics: {:?}",
            trial,
            arena.diagnostics
        );
        let mut reg = analyze_physics(&arena, arena.root, None);

        let scalar_idx = arena
            .nodes
            .iter()
            .position(|n| {
                n.node_kind() == NodeKind::StructDecl
                    && arena.interner.resolve(SymbolId(n.lhs)) == scalar_name
            })
            .expect("randomized scalar struct node");
        let scalar_lid = reg.node_layouts[scalar_idx].expect("scalar layout must be bound");
        let scalar_lay = reg.layouts.get(scalar_lid).unwrap();
        assert_eq!(scalar_lay.byte_size, expected_bytes);
        assert_eq!(scalar_lay.byte_align, expected_align);

        let dflags_scalar = handle_engine_intrinsic(
            &mut reg,
            "ast_decl_flags",
            &[PilotVal::from_int(scalar_idx as i64)],
        )
        .unwrap()
        .as_int()
        .unwrap();
        assert_ne!(dflags_scalar & 16, 0, "partial bit must survive flatten_ast");

        let body_idx = arena
            .nodes
            .iter()
            .position(|n| {
                n.node_kind() == NodeKind::StructDecl
                    && arena.interner.resolve(SymbolId(n.lhs)) == body_name
            })
            .expect("randomized body struct node");

        let dflags_body = handle_engine_intrinsic(
            &mut reg,
            "ast_decl_flags",
            &[PilotVal::from_int(body_idx as i64)],
        )
        .unwrap()
        .as_int()
        .unwrap();
        assert_ne!(dflags_body & 1024, 0, "~self() destructor bit must survive");

        let attr_sym = handle_engine_intrinsic(
            &mut reg,
            "ast_attr_name",
            &[PilotVal::from_int(body_idx as i64), PilotVal::from_int(0)],
        )
        .unwrap()
        .as_int()
        .unwrap();
        let reflected_attr =
            handle_engine_intrinsic(&mut reg, "sym_str", &[PilotVal::from_int(attr_sym)])
                .unwrap()
                .to_string_repr();
        assert_eq!(reflected_attr, attr_name);

        let got_neg = handle_engine_intrinsic(
            &mut reg,
            "ast_attr_decl_arg",
            &[
                PilotVal::from_int(body_idx as i64),
                PilotVal::from_int(0),
                PilotVal::from_int(0),
            ],
        )
        .unwrap()
        .as_int()
        .unwrap();
        assert_eq!(got_neg, neg_arg);

        let got_float_bits = handle_engine_intrinsic(
            &mut reg,
            "ast_attr_decl_arg",
            &[
                PilotVal::from_int(body_idx as i64),
                PilotVal::from_int(0),
                PilotVal::from_int(1),
            ],
        )
        .unwrap()
        .as_int()
        .unwrap() as u64;
        assert_eq!(f64::from_bits(got_float_bits), float_val);

        let got_bool = handle_engine_intrinsic(
            &mut reg,
            "ast_attr_decl_arg",
            &[
                PilotVal::from_int(body_idx as i64),
                PilotVal::from_int(0),
                PilotVal::from_int(2),
            ],
        )
        .unwrap()
        .as_int()
        .unwrap();
        assert_eq!(got_bool, i64::from(bool_val));

        let got_str_sym = handle_engine_intrinsic(
            &mut reg,
            "ast_attr_decl_arg",
            &[
                PilotVal::from_int(body_idx as i64),
                PilotVal::from_int(0),
                PilotVal::from_int(3),
            ],
        )
        .unwrap()
        .as_int()
        .unwrap();
        let got_str =
            handle_engine_intrinsic(&mut reg, "sym_str", &[PilotVal::from_int(got_str_sym)])
                .unwrap()
                .to_string_repr();
        assert_eq!(got_str, str_arg);

        let got_ident_sym = handle_engine_intrinsic(
            &mut reg,
            "ast_attr_decl_arg",
            &[
                PilotVal::from_int(body_idx as i64),
                PilotVal::from_int(0),
                PilotVal::from_int(4),
            ],
        )
        .unwrap()
        .as_int()
        .unwrap();
        let got_ident =
            handle_engine_intrinsic(&mut reg, "sym_str", &[PilotVal::from_int(got_ident_sym)])
                .unwrap()
                .to_string_repr();
        assert_eq!(got_ident, ident_arg);

        let base1_sym = handle_engine_intrinsic(
            &mut reg,
            "ast_base_at",
            &[PilotVal::from_int(body_idx as i64), PilotVal::from_int(1)],
        )
        .unwrap()
        .as_int()
        .unwrap();
        let base1_str =
            handle_engine_intrinsic(&mut reg, "sym_str", &[PilotVal::from_int(base1_sym)])
                .unwrap()
                .to_string_repr();
        assert_eq!(base1_str, base_extra);

        let mut observed_quotes = Vec::new();
        let mut saw_list_n = false;
        for nid in 0..reg.arena.nodes.len() {
            let orig = handle_engine_intrinsic(
                &mut reg,
                "ast_orig_kind",
                &[PilotVal::from_int(nid as i64)],
            )
            .unwrap()
            .as_int()
            .unwrap() as u16;
            let nflags = handle_engine_intrinsic(
                &mut reg,
                "ast_node_flags",
                &[PilotVal::from_int(nid as i64)],
            )
            .unwrap()
            .as_int()
            .unwrap();
            let cc = handle_engine_intrinsic(
                &mut reg,
                "ast_child_count",
                &[PilotVal::from_int(nid as i64)],
            )
            .unwrap()
            .as_int()
            .unwrap() as usize;
            if orig == NodeKind::CallExpr as u16 && nflags > 0 {
                observed_quotes.push(nflags as u8);
            }
            if orig == NodeKind::ListExpr as u16 && cc == list_len {
                saw_list_n = true;
            }
        }
        let expected_quotes: Vec<u8> = quote_modes.iter().map(|&(_, code)| code).collect();
        assert_eq!(observed_quotes, expected_quotes);
        assert!(saw_list_n, "ListExpr child count {} must survive", list_len);
    }
    guard.finish();
}

#[test]
pub fn test_50_2_open_keyword_reflection_and_randomized_user_categories() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_50_2_open_keyword_reflection_and_randomized_user_categories",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5002_BEEF);
    let empty_arena = parse("");
    let (flat, syms) = flatten_ast(&empty_arena, empty_arena.root);
    let mut reg = PhysicsRegistry::new(flat, syms);

    let kw_path = workspace_root()
        .join("stdlib/core/keywords.pilot")
        .to_string_lossy()
        .to_string();
    let mounted =
        handle_engine_intrinsic(&mut reg, "mount_unit", &[PilotVal::from_str(&kw_path)])
            .unwrap()
            .as_int()
            .unwrap();
    assert!(mounted > 50);

    // Verify core categories 0..=14 are dynamically reflected
    for cat_id in [0i64, 1, 2, 5, 8] {
        let scope = handle_engine_intrinsic(
            &mut reg,
            "query_registered_symbols",
            &[PilotVal::from_int(cat_id), PilotVal::from_int(1)],
        )
        .unwrap()
        .to_string_repr();
        let badge = handle_engine_intrinsic(
            &mut reg,
            "query_registered_symbols",
            &[PilotVal::from_int(cat_id), PilotVal::from_int(2)],
        )
        .unwrap()
        .to_string_repr();
        assert!(
            scope.ends_with(".pilot") && !badge.is_empty(),
            "Category {} must reflect valid scope/badge, got {:?}/{:?}",
            cat_id,
            scope,
            badge
        );
    }

    // Register 6 random user categories >= 100 and verify strict isolation
    for idx in 0..6 {
        let cid = 100 + idx * 37 + rng.range(1, 20);
        let scope = format!("keyword.dsl.{}.pilot", rng.alpha_tag(6));
        let badge = format!("DSL-{}", rng.alpha_tag(5));
        handle_engine_intrinsic(
            &mut reg,
            "register_keyword_category",
            &[
                PilotVal::from_int(cid),
                PilotVal::from_str(&scope),
                PilotVal::from_str(&badge),
                PilotVal::from_int(14),
            ],
        )
        .unwrap();

        let word_cnt = rng.range(3, 6) as usize;
        let mut expected_words = Vec::new();
        for w_i in 0..word_cnt {
            let kw = format!("kw_{}_{}_{}", idx, w_i, rng.alpha_tag(5));
            let doc = format!("Doc {} {}", cid, rng.alpha_tag(10));
            handle_engine_intrinsic(
                &mut reg,
                "register_keyword",
                &[
                    PilotVal::from_int(cid),
                    PilotVal::from_str(&kw),
                    PilotVal::from_str(&doc),
                ],
            )
            .unwrap();
            expected_words.push((kw, doc));
        }

        let got_scope = handle_engine_intrinsic(
            &mut reg,
            "query_registered_symbols",
            &[PilotVal::from_int(cid), PilotVal::from_int(1)],
        )
        .unwrap()
        .to_string_repr();
        let got_badge = handle_engine_intrinsic(
            &mut reg,
            "query_registered_symbols",
            &[PilotVal::from_int(cid), PilotVal::from_int(2)],
        )
        .unwrap()
        .to_string_repr();
        assert_eq!(got_scope, scope);
        assert_eq!(got_badge, badge);

        let words_joined = handle_engine_intrinsic(
            &mut reg,
            "query_registered_symbols",
            &[PilotVal::from_int(cid)],
        )
        .unwrap()
        .to_string_repr();
        let got_list: Vec<&str> = words_joined.split('|').collect();
        assert_eq!(got_list.len(), expected_words.len());
        for (kw, doc) in &expected_words {
            assert!(got_list.contains(&kw.as_str()));
            let got_doc = handle_engine_intrinsic(
                &mut reg,
                "query_registered_symbols",
                &[
                    PilotVal::from_int(cid),
                    PilotVal::from_int(3),
                    PilotVal::from_str(kw),
                ],
            )
            .unwrap()
            .to_string_repr();
            assert_eq!(&got_doc, doc);
        }
    }
    guard.finish();
}

#[test]
pub fn test_50_3_bootstrap_manifest_randomized_naming_verifier_and_full_weight_sorted_weaver() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_50_3_bootstrap_manifest_randomized_naming_verifier_and_full_weight_sorted_weaver",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5003_CAFE);
    let root = workspace_root();

    // 1. Verify bootstrap.pilot uses scalable `.len` map lookup rather than hardcoded `return 8`
    let bootstrap_src = std::fs::read_to_string(root.join("stdlib/core/bootstrap.pilot"))
        .expect("read stdlib/core/bootstrap.pilot");
    assert!(
        !bootstrap_src.contains("return 8;") && bootstrap_src.contains(".len"),
        "stdlib/core/bootstrap.pilot must compute stage count dynamically via `.len`, never `return 8;`"
    );

    let snap = BootstrapSnapshot::global();
    assert!(snap.arenas.len() >= 8);
    assert_eq!(snap.arenas[0].0, "Pilot.Core.Genesis");

    for rel in [
        "stdlib/core/bootstrap.pilot",
        "stdlib/core/naming_attr.pilot",
        "stdlib/core/mem.pilot",
        "stdlib/core/meta.pilot",
    ] {
        let src = std::fs::read_to_string(root.join(rel)).unwrap();
        let mut tables = snap.tables.clone();
        let mut cvm = snap.vm.clone();
        let arena = eval_file_with_tables(&src, &mut tables, &mut cvm).expect("parse core file");
        let res = AnalysisPipeline::analyze(&arena, arena.root);
        assert!(
            res.is_ok(),
            "Core file '{}' must satisfy [EnforceCoreNaming] with 0 errors: {:?}",
            rel,
            res.diagnostics
        );
    }

    // 2. Randomized [EnforceCoreNaming] & LatticeContractMismatch adversarial trials
    let diag_pilot_src = std::fs::read_to_string(root.join("selfhost/diagnostics.pilot"))
        .expect("read selfhost/diagnostics.pilot");

    for trial in 0..6 {
        let tag = {
            let s = rng.alpha_tag(5);
            let mut c = s.chars();
            format!("{}{}", c.next().unwrap().to_ascii_uppercase(), c.as_str())
        };
        let bad_iface = format!("BrokenIface{}", tag);
        let bad_lower_struct = format!("lower_{}", rng.alpha_tag(6));
        let bad_i_prefix_struct = format!("IMisleading{}", tag);
        let bad_lang_upper = format!("BadPrim{}", tag);
        let bad_attr = format!("Clamp{}", tag);
        let bad_view = format!("Window{}", tag);
        let bad_guard = format!("Mutex{}", tag);
        let bad_val_dtor = format!("CorruptVal{}", tag);

        let ok_iface = format!("IValid{}", tag);
        let ok_lang = format!("prim_{}", rng.alpha_tag(4));
        let ok_attr = format!("Clamp{}Attr", tag);
        let ok_view_suffix = ["View", "Lens", "Cursor", "Enumerator"][trial % 4];
        let ok_view = format!("Slice{}{}", tag, ok_view_suffix);
        let ok_guard_suffix = ["Guard", "Scope", "Lease"][trial % 3];
        let ok_guard = format!("Lock{}{}", tag, ok_guard_suffix);
        let ok_val = format!("Plain{}", tag);
        let ok_obj = format!("Buffer{}Handle", tag);

        let test_pkg_src = format!(
            r#"
            [EnforceCoreNaming]
            pub pkg Pilot.Core.RandTest{trial};

            pub interface {ok_iface} {{}}
            [Lang(99)] pub struct {ok_lang} : IValue {{}}
            pub struct {ok_attr} : IAttribute, IValue {{}}
            pub struct {ok_view} : IReference {{}}
            pub struct {ok_guard} : IGuard {{}}
            pub struct {ok_val} : IValue {{}}
            pub struct {ok_obj} : IObject {{ ~self() {{}} }}

            pub interface {bad_iface} {{}}
            pub struct {bad_lower_struct} : IValue {{}}
            pub struct {bad_i_prefix_struct} : IValue {{}}
            [Lang(98)] pub struct {bad_lang_upper} : IValue {{}}
            pub struct {bad_attr} : IAttribute, IValue {{}}
            pub struct {bad_view} : IReference {{}}
            pub struct {bad_guard} : IGuard {{}}
            pub struct {bad_val_dtor} : IValue {{ ~self() {{}} }}
            "#
        );

        let parsed = parse(&test_pkg_src);
        assert!(parsed.diagnostics.is_empty());

        // Test BOTH `AnalysisPipeline::analyze` (via `analyzer.pilot`) AND `diagnostics.pilot`
        let res = AnalysisPipeline::analyze(&parsed, parsed.root);
        assert_eq!(
            res.diagnostics.len(),
            8,
            "Trial {}: expected exactly 8 violations (7 naming + 1 lattice), got {:?}",
            trial,
            res.diagnostics
        );
        for (rule, offender) in [
            ("InterfacePrefix", &bad_iface),
            ("StructPascalCase", &bad_lower_struct),
            ("ReservedInterfacePrefix", &bad_i_prefix_struct),
            ("LangPrimitiveLowercase", &bad_lang_upper),
            ("AttributeSuffix", &bad_attr),
            ("ReferenceViewSuffix", &bad_view),
            ("ScopedGuardSuffix", &bad_guard),
        ] {
            let matched = res.diagnostics.iter().find(|d| {
                d.code == "EnforceCoreNaming"
                    && d.message.contains(rule)
                    && d.message.contains(offender.as_str())
            });
            assert!(
                matched.is_some(),
                "Trial {}: missing EnforceCoreNaming [{}] for '{}'; got {:?}",
                trial,
                rule,
                offender,
                res.diagnostics
            );
            let sp = matched.unwrap().span;
            assert!(sp.start > 0 && sp.end > sp.start);
            assert!(
                test_pkg_src[sp.start as usize..sp.end as usize].contains(offender.as_str()),
                "Diagnostic span {:?} must cover offending declaration '{}'",
                sp,
                offender
            );
        }
        assert!(res.diagnostics.iter().any(|d| {
            d.code == "LatticeContractMismatch" && d.message.contains(bad_val_dtor.as_str())
        }));

        // Verify `selfhost/diagnostics.pilot::verify_unit_diagnostics()` directly
        let (flat, syms) = flatten_ast(&parsed, parsed.root);
        let mut d_reg = PhysicsRegistry::new(flat, syms);
        let diag_cnt = run_vm_with_registry(
            &format!("{}\nverify_unit_diagnostics();\n", diag_pilot_src),
            &mut d_reg,
        )
        .as_int()
        .unwrap_or(-1);
        assert_eq!(
            diag_cnt, 8,
            "selfhost/diagnostics.pilot::verify_unit_diagnostics() must catch all 8 violations"
        );

        // Verify omitting `[EnforceCoreNaming]` suppresses the 7 naming lints while keeping `LatticeContractMismatch`!
        let unguarded_src = test_pkg_src.replace("[EnforceCoreNaming]", "");
        let ug_parsed = parse(&unguarded_src);
        let ug_res = AnalysisPipeline::analyze(&ug_parsed, ug_parsed.root);
        assert_eq!(
            ug_res.diagnostics.len(),
            1,
            "Without [EnforceCoreNaming], ONLY universal LatticeContractMismatch must fire, got: {:?}",
            ug_res.diagnostics
        );
        assert_eq!(ug_res.diagnostics[0].code, "LatticeContractMismatch");
    }

    // 3. Full N-attribute weight-sorting & pillar verification in `selfhost/attr_weaver.pilot`
    let weaver_src = std::fs::read_to_string(root.join("selfhost/attr_weaver.pilot"))
        .expect("read selfhost/attr_weaver.pilot");

    for trial in 0..8 {
        let attr_cnt = rng.range(5, 7) as usize;
        let mut attrs: Vec<(String, i64, usize)> = Vec::new();
        let mut attr_syntax_items = Vec::new();
        for idx in 0..attr_cnt {
            let nm = format!(
                "Stage{}{}",
                idx,
                rng.alpha_tag(4).to_ascii_uppercase()
            );
            // Include occasional duplicate weights to test stable tie-breaking!
            let weight = if idx == 3 && !attrs.is_empty() {
                attrs[0].1
            } else {
                rng.range(-800, 800)
            };
            attrs.push((nm.clone(), weight, idx));
            attr_syntax_items.push(format!("{}({})", nm, weight));
        }

        let ecs_sys_cnt = rng.range(1, 4) as usize;
        let mut ecs_decls = String::new();
        for e in 0..ecs_sys_cnt {
            ecs_decls.push_str(&format!(
                "[Rule({})]\npub struct EcsSys{}{} {{}}\n",
                rng.range(1, 99),
                trial,
                e
            ));
        }

        let w_src = format!(
            r#"
            {ecs_decls}
            [{}]
            pub fn target_fn_{trial}(x: i64) -> i64 {{ return x; }}

            [Clamp(50)]
            pub interface IBadIface{trial} {{}}

            pub struct BadFieldHost{trial} {{
                [TraceSpan(15)]
                pub count: i64;
            }}
            "#,
            attr_syntax_items.join(", ")
        );

        let w_arena = parse(&w_src);
        assert!(w_arena.diagnostics.is_empty());
        let (w_flat, w_syms) = flatten_ast(&w_arena, w_arena.root);
        let mut w_reg = PhysicsRegistry::new(w_flat, w_syms);

        let fn_nid = w_arena
            .nodes
            .iter()
            .position(|n| n.node_kind() == NodeKind::FnDecl)
            .expect("target FnDecl");

        let mut expected_sorted = attrs.clone();
        expected_sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));
        let expected_top_idx = expected_sorted[0].2 as i64;
        let expected_names_csv = expected_sorted
            .iter()
            .map(|(n, _, _)| n.as_str())
            .collect::<Vec<_>>()
            .join(",");

        let got_top = run_vm_with_registry(
            &format!("{}\nhighest_weight_attr_index({});\n", weaver_src, fn_nid),
            &mut w_reg,
        )
        .as_int()
        .unwrap_or(-99);
        assert_eq!(got_top, expected_top_idx);

        let got_sorted_csv = run_vm_with_registry(
            &format!("{}\nweave_sorted_attr_names({});\n", weaver_src, fn_nid),
            &mut w_reg,
        )
        .to_string_repr();
        assert_eq!(
            got_sorted_csv, expected_names_csv,
            "weave_sorted_attr_names(nid) must return comma-separated attr names sorted by descending weight (stable on ties)"
        );

        let got_ecs = run_vm_with_registry(
            &format!("{}\nweave_unit_attributes();\n", weaver_src),
            &mut w_reg,
        )
        .as_int()
        .unwrap_or(-1);
        assert_eq!(got_ecs, ecs_sys_cnt as i64);
        assert!(
            w_reg.diagnostics.iter().any(|d| d.message.contains("ATTR001")),
            "Must emit ATTR001 for reactive attribute on interface"
        );
        assert!(
            w_reg.diagnostics.iter().any(|d| d.message.contains("ATTR002")),
            "Must emit ATTR002 for interceptor attribute on field"
        );
    }
    guard.finish();
}

#[test]
pub fn test_50_4_and_50_5_target_width_projections_pilc_snapshot_and_gate1_gate2_contraction() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_50_4_and_50_5_target_width_projections_pilc_snapshot_and_gate1_gate2_contraction",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5004_F00D);
    let root = workspace_root();

    // 1. Run gate1_check.py and gate2_check.py to enforce pilot-analysis contraction & whitelist
    for gate in ["scripts/gate1_check.py", "scripts/gate2_check.py"] {
        let status = Command::new("python3")
            .arg(gate)
            .current_dir(root)
            .status()
            .unwrap_or_else(|e| panic!("Failed to execute {}: {}", gate, e));
        assert!(status.success(), "{} must pass 100%", gate);
    }

    // 2. Randomized multi-field aggregate offsets, target-width SOA projections & capability lattice
    let analyzer_raw = std::fs::read_to_string(root.join("selfhost/analyzer.pilot"))
        .expect("read selfhost/analyzer.pilot");
    let analyzer_defs = analyzer_raw
        .trim_end()
        .strip_suffix("analyze_unit(0);")
        .unwrap_or(&analyzer_raw);

    let empty_arena = parse("");
    let (flat, syms) = flatten_ast(&empty_arena, empty_arena.root);
    let mut reg = PhysicsRegistry::new(flat, syms);

    let scalar_choices = [8u16, 16, 32, 64, 128];
    for _ in 0..8 {
        let n_fields = rng.range(3, 6) as usize;
        let mut field_lids = Vec::new();
        for _ in 0..n_fields {
            let bw = scalar_choices[(rng.next_u64() as usize) % scalar_choices.len()];
            field_lids.push(reg.layouts.register_scalar(bw, 1, 2));
        }
        let agg_lid = reg.layouts.register_aggregate(&field_lids, false, false);
        let expected_offsets: Vec<u32> = match &reg.layouts.get(agg_lid).unwrap().geometry {
            PhysicsGeometry::Aggregate { fields, .. } => {
                fields.iter().map(|f| f.byte_offset).collect()
            }
            other => panic!("Expected Aggregate, got {:?}", other),
        };

        for (f_idx, &exp_off) in expected_offsets.iter().enumerate() {
            let got_off = run_vm_with_registry(
                &format!(
                    "{}\ncompute_field_projection_offset({}, {});\n",
                    analyzer_defs, agg_lid.0, f_idx
                ),
                &mut reg,
            )
            .as_int()
            .unwrap_or(-1);
            assert_eq!(
                got_off, exp_off as i64,
                "compute_field_projection_offset mismatch on field {}",
                f_idx
            );
        }

        // Pointer-width SOA slice projection: 64-bit -> 16B (align 8), 32-bit -> 8B (align 4)
        let elem_lid = field_lids[0];
        let soa64 = LayoutId(
            run_vm_with_registry(
                &format!("{}\ncompute_soa_projection({}, 64);\n", analyzer_defs, elem_lid.0),
                &mut reg,
            )
            .as_int()
            .unwrap() as u32,
        );
        let soa32 = LayoutId(
            run_vm_with_registry(
                &format!("{}\ncompute_soa_projection({}, 32);\n", analyzer_defs, elem_lid.0),
                &mut reg,
            )
            .as_int()
            .unwrap() as u32,
        );
        assert_eq!(reg.layouts.get(soa64).unwrap().byte_size, 16);
        assert_eq!(reg.layouts.get(soa64).unwrap().byte_align, 8);
        assert_eq!(reg.layouts.get(soa32).unwrap().byte_size, 8);
        assert_eq!(reg.layouts.get(soa32).unwrap().byte_align, 4);
    }

    // Verify capability transitions
    for (from_c, to_c, expected) in [
        (CapabilityBits::VALUE, CapabilityBits::BORROW_REF, true),
        (CapabilityBits::OWNED_OBJ, CapabilityBits::BORROW_REF, true),
        (CapabilityBits::OWNED_OBJ, CapabilityBits::SCOPED_GUARD, true),
        (CapabilityBits::BORROW_REF, CapabilityBits::OWNED_OBJ, false),
        (CapabilityBits::VALUE, CapabilityBits::OWNED_OBJ, false),
    ] {
        let ok = run_vm_with_registry(
            &format!(
                "{}\nverify_capability_transition({}, {});\n",
                analyzer_defs,
                from_c.raw(),
                to_c.raw()
            ),
            &mut reg,
        )
        .as_bool();
        assert_eq!(ok, Some(expected));
    }

    // 3. `.pilc` snapshot roundtrip & functional execution parity
    let bin = load_or_compile_analyzer_pilc();
    let bytes = bin.serialize();
    assert_eq!(&bytes[0..4], PILC_V3_MAGIC);
    let reloaded = PilcBinary::load_from_bytes(&bytes).expect("PilcBinary::load_from_bytes");
    assert_eq!(reloaded.instructions.len(), bin.instructions.len());
    assert_eq!(reloaded.constants.len(), bin.constants.len());
    guard.finish();
}
