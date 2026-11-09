//! ============================================================================
//! Milestone 7 Verification Suite:
//! Zero-Black-Box Transparency Tool (`pilot inspect`)
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 7,
    test_file_rel: "crates/pilot-analysis/tests/milestone07_zero_black_box_transparency_tool.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/inspect/selector.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "target_kind_name",
                "parse_int",
                "parse_inspect_selector",
            ],
            required_code_markers: &[
                "TargetQuery",
                "target_kind_whole_file",
                "target_kind_line_col_cursor",
                "parse_inspect_selector",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/inspect/layout_lens.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "align_padding",
                "render_field_box",
                "calculate_optimal_packing_savings",
            ],
            required_code_markers: &[
                "FieldProbeEntry",
                "cache_line_bytes",
                "render_field_box",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/inspect/method_lens.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "tier_name",
                "resolve_winning_tier",
                "render_method_trace",
            ],
            required_code_markers: &[
                "MethodCandidateTrace",
                "resolve_winning_tier",
                "render_method_trace",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/inspect/perceus_lens.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "format_perceus_marker",
            ],
            required_code_markers: &[
                "PerceusEventNote",
                "format_perceus_marker",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/inspect/weave_lens.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "format_woven_statement",
                "expand_property_accessor",
            ],
            required_code_markers: &[
                "format_woven_statement",
                "expand_property_accessor",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/cli/inspect_cmd.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "inspect_cmd_help",
                "inspect_run",
            ],
            required_code_markers: &[
                "inspect_cmd_help",
                "inspect_run",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_inspect(script: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let deps = [
        "selfhost/ir/model.pilot",
        "selfhost/inspect/selector.pilot",
        "selfhost/inspect/layout_lens.pilot",
        "selfhost/inspect/method_lens.pilot",
        "selfhost/inspect/perceus_lens.pilot",
        "selfhost/inspect/weave_lens.pilot",
        "selfhost/cli/inspect_cmd.pilot",
    ];
    let mut combined = String::new();
    for rel in &deps {
        let p = root.join(rel);
        let src = fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("read {}: {:?}", p.display(), e));
        combined.push_str(&src);
        combined.push('\n');
    }
    combined.push_str(script);
    let arena = parse(&combined);
    let (flat, syms) = pilot_analysis::flatten_ast(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    let mut phy = PhysicsRegistry::new(flat, syms);
    let reg_ptr = &mut phy as *mut PhysicsRegistry;
    vm.engine_handler = Some(Box::new(move |op, a| {
        pilot_analysis::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, a)
    }));
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    vm.run().to_string_repr()
}

// -----------------------------------------------------------------------------
// Test 1: Selector Resolution & Target Query Parsing
// -----------------------------------------------------------------------------
#[test]
pub fn test_m7_1_selector_resolution_and_target_query_parsing() {
    let guard = anticheat::AntiCheatGuard::arm("test_m7_1_selector_resolution_and_target_query_parsing", &ANTICHEAT_SPEC);

    let script = r#"
        let q_file = parse_inspect_selector("src/app.pilot");
        let q_cursor = parse_inspect_selector("src/app.pilot:42:10");
        let q_range = parse_inspect_selector("src/app.pilot:40-55");
        let q_sym = parse_inspect_selector("App.Billing.Account.balance");
        let q_gen = parse_inspect_selector("List<i64>");

        target_kind_name(q_file.kind) + "|" +
        target_kind_name(q_cursor.kind) + ":" + q_cursor.start_line + ":" + q_cursor.col + "|" +
        target_kind_name(q_range.kind) + ":" + q_range.start_line + "-" + q_range.end_line + "|" +
        target_kind_name(q_sym.kind) + "|" +
        target_kind_name(q_gen.kind);
    "#;

    let res = eval_inspect(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 5, "Expected 5 parts, got: {}", res);
    assert_eq!(parts[0], "WholeFile", "WholeFile kind mismatch");
    assert_eq!(parts[1], "LineColCursor:42:10", "LineColCursor parse mismatch");
    assert_eq!(parts[2], "LineRange:40-55", "LineRange parse mismatch");
    assert_eq!(parts[3], "SymbolPath", "SymbolPath kind mismatch");
    assert_eq!(parts[4], "MonomorphizedType", "MonomorphizedType kind mismatch");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Layout Lens Byte Offsets, Cache Line & Optimal Packing
// -----------------------------------------------------------------------------
#[test]
pub fn test_m7_2_layout_lens_byte_offsets_cache_line_and_optimal_packing() {
    let guard = anticheat::AntiCheatGuard::arm("test_m7_2_layout_lens_byte_offsets_cache_line_and_optimal_packing", &ANTICHEAT_SPEC);

    let script = r#"
        // 1. Padding calculation
        let pad1 = align_padding(1, 8); // 7 bytes padding to reach 8
        let pad2 = align_padding(8, 8); // 0 bytes padding

        // 2. 64B Cache line crossing
        let cross_no = field_crosses_cache_line(16, 32);  // 16..47 within line 0
        let cross_yes = field_crosses_cache_line(48, 32); // 48..79 crosses into line 1

        // 3. Field probe rendering
        let f1 = field_probe_new("header", 0, 16, 0, lattice_value(), "HeaderBundle");
        let s_box = render_field_box(f1);

        // 4. Optimal field packing savings
        let savings = calculate_optimal_packing_savings(24, 8); // 24B unaligned - 8B optimal = 16B saved

        pad1 + "|" + pad2 + "|" +
        (if !cross_no { "SAFE" } else { "CROSS" }) + "|" +
        (if cross_yes { "CROSS" } else { "SAFE" }) + "|" +
        (if s_box.len() > 0 { "BOX_OK" } else { "FAIL" }) + "|" +
        savings;
    "#;

    let res = eval_inspect(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 6, "Expected 6 parts, got: {}", res);
    assert_eq!(parts[0], "7", "Padding to align 1 to 8 should be 7");
    assert_eq!(parts[1], "0", "Padding to align 8 to 8 should be 0");
    assert_eq!(parts[2], "SAFE", "Offset 16 size 32 should not cross cache line");
    assert_eq!(parts[3], "CROSS", "Offset 48 size 32 should cross cache line");
    assert_eq!(parts[4], "BOX_OK", "Field box rendering failed");
    assert_eq!(parts[5], "16", "Expected 16 bytes optimal packing savings");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Method Lens 4-Tier Resolution & Collision Modifiers
// -----------------------------------------------------------------------------
#[test]
pub fn test_m7_3_method_lens_4_tier_resolution_and_collision_modifiers() {
    let guard = anticheat::AntiCheatGuard::arm("test_m7_3_method_lens_4_tier_resolution_and_collision_modifiers", &ANTICHEAT_SPEC);

    let script = r#"
        // 1. Tier precedence: Tier 1 beats Tier 2, Tier 2 beats Tier 3, Tier 3 beats Tier 4
        let win_1_2 = resolve_winning_tier(1, 2);
        let win_2_3 = resolve_winning_tier(2, 3);
        let win_3_4 = resolve_winning_tier(3, 4);
        let win_4_5 = resolve_winning_tier(4, 5);

        // 2. Trace rendering
        let t1 = method_trace_new("get_balance", "() -> f64", 1, "Account", true, "");
        let t2 = method_trace_new("validate", "() -> bool", 2, "IValidatable", true, "coalesce");
        let t3 = method_trace_new("default_hash", "() -> u64", 4, "IHashable", false, "shadowed");

        let s_t1 = render_method_trace(t1);
        let s_t2 = render_method_trace(t2);
        let s_t3 = render_method_trace(t3);

        win_1_2 + "|" + win_2_3 + "|" + win_3_4 + "|" + win_4_5 + "|" +
        (if s_t1.find("WINNER") >= 0 { "WIN1_OK" } else { "FAIL" }) + "|" +
        (if s_t2.find("coalesce") >= 0 { "COALESCE_OK" } else { "FAIL" }) + "|" +
        (if s_t3.find("SHADOWED") >= 0 { "SHADOW_OK" } else { "FAIL" });
    "#;

    let res = eval_inspect(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 7, "Expected 7 parts, got: {}", res);
    assert_eq!(parts[0], "1", "Tier 1 should beat Tier 2");
    assert_eq!(parts[1], "2", "Tier 2 should beat Tier 3");
    assert_eq!(parts[2], "3", "Tier 3 should beat Tier 4");
    assert_eq!(parts[3], "4", "Tier 4 should beat Tier 5");
    assert_eq!(parts[4], "WIN1_OK", "Winning method trace format failed");
    assert_eq!(parts[5], "COALESCE_OK", "Coalesce modifier provenance failed");
    assert_eq!(parts[6], "SHADOW_OK", "Shadowed method trace format failed");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Perceus Lens Inline Markers & Weave Lens Expand
// -----------------------------------------------------------------------------
#[test]
pub fn test_m7_4_perceus_lens_inline_markers_and_weave_lens_expand() {
    let guard = anticheat::AntiCheatGuard::arm("test_m7_4_perceus_lens_inline_markers_and_weave_lens_expand", &ANTICHEAT_SPEC);

    let script = r#"
        // 1. Perceus inline markers
        let ev_borrow = perceus_event_new("buf", "borrow", 10, 5, "");
        let ev_move = perceus_event_new("buf", "move", 20, 5, "");
        let ev_dup = perceus_event_new("buf", "dup", 30, 5, "");
        let ev_drop = perceus_event_new("buf", "drop", 40, 5, "");
        let ev_reuse = perceus_event_new("buf", "drop_reuse", 50, 5, "NewBuffer");
        let ev_guard = perceus_event_new("lock", "guard_exit", 60, 5, "");

        let s_borrow = format_perceus_marker(ev_borrow);
        let s_move = format_perceus_marker(ev_move);
        let s_dup = format_perceus_marker(ev_dup);
        let s_drop = format_perceus_marker(ev_drop);
        let s_reuse = format_perceus_marker(ev_reuse);
        let s_guard = format_perceus_marker(ev_guard);

        // 2. Weave Lens property accessor expansion
        let s_prop = expand_property_accessor("balance", "f64", "backing_balance", true);

        // 3. Full CLI driver inspection run
        let full_out = inspect_run("Account", "--all");

        (if s_borrow.find("borrow") >= 0 { "BORROW_OK" } else { "FAIL" }) + "|" +
        (if s_move.find("move") >= 0 { "MOVE_OK" } else { "FAIL" }) + "|" +
        (if s_dup.find("dup") >= 0 { "DUP_OK" } else { "FAIL" }) + "|" +
        (if s_drop.find("drop") >= 0 { "DROP_OK" } else { "FAIL" }) + "|" +
        (if s_reuse.find("drop_reuse") >= 0 { "REUSE_OK" } else { "FAIL" }) + "|" +
        (if s_guard.find("guard-exit") >= 0 { "GUARD_OK" } else { "FAIL" }) + "|" +
        (if s_prop.find("get_balance") >= 0 && s_prop.find("set_balance") >= 0 { "PROP_OK" } else { "FAIL" }) + "|" +
        (if full_out.find("Weave Lens") >= 0 && full_out.find("Layout Lens") >= 0 { "INSPECT_OK" } else { "FAIL" });
    "#;

    let res = eval_inspect(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 8, "Expected 8 parts, got: {}", res);
    assert_eq!(parts[0], "BORROW_OK", "Perceus borrow marker mismatch");
    assert_eq!(parts[1], "MOVE_OK", "Perceus move marker mismatch");
    assert_eq!(parts[2], "DUP_OK", "Perceus dup marker mismatch");
    assert_eq!(parts[3], "DROP_OK", "Perceus drop marker mismatch");
    assert_eq!(parts[4], "REUSE_OK", "Perceus reuse marker mismatch");
    assert_eq!(parts[5], "GUARD_OK", "Perceus guard-exit marker mismatch");
    assert_eq!(parts[6], "PROP_OK", "Weave property expansion mismatch");
    assert_eq!(parts[7], "INSPECT_OK", "Inspect CLI run output mismatch");

    guard.finish();
}
