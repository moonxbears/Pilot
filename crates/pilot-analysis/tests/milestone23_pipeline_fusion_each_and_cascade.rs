//! ============================================================================
//! Milestone 23 Verification Suite:
//! Zero-Allocation Pipeline Fusion, `each` Vectorization & Cascade Polish
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/vm_outcome.rs"]
mod vm_outcome;
#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 23,
    test_file_rel: "crates/pilot-analysis/tests/milestone23_pipeline_fusion_each_and_cascade.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "stdlib/core/stream.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "stream_sum",
                "stream_count",
                "stream_collect",
            ],
            required_code_markers: &[
                "SizeHint",
                "StreamOpt",
                "SliceStream",
                "TakeStream",
                "StepStream",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ir/pass_stream_fusion.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "fuse_stream_pipeline",
                "stream_pipeline_spec_new",
                "rewrite_fused_pipeline",
            ],
            required_code_markers: &[
                "StreamStageKind",
                "StreamTerminalKind",
                "StreamPipelineStage",
                "StreamPipelineSpec",
                "FusedLoopPlan",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ir/lower_each_cascade.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "lower_each_broadcast",
                "lower_cascade",
                "lower_dto_projection",
                "emit_cascade_ir",
            ],
            required_code_markers: &[
                "BroadcastMode",
                "EachBroadcastLowering",
                "CascadeLowering",
                "DtoProjectionLowering",
                "FUSE001",
                "FUSE002",
                "FUSE003",
                "FUSE004",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m23(deps: &[&str], script: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let mut combined = String::new();
    for rel in deps {
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
    vm_outcome::outcome(&mut vm)
}

// -----------------------------------------------------------------------------
// Test 1: Zero-Allocation Pull-Cursor Stream Combinators & Terminals
// -----------------------------------------------------------------------------
#[test]
fn test_m23_1_zero_allocation_pull_stream_combinators() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m23_1_zero_allocation_pull_stream_combinators",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            let data: slice<i64> = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

            // 1. SliceStream basic iteration & size_hint
            var s1 = SliceStream::create(data);
            let h1 = s1.size_hint();
            if h1.lower != 10 || !h1.has_upper || h1.upper != 10 { return 1; }

            let total = stream_sum(var s1);
            if total != 55 { return 2; }

            // 2. TakeStream: take 4 items
            var s2 = SliceStream::create(data);
            var take_s = TakeStream::create(s2, 4);
            let h2 = take_s.size_hint();
            if h2.upper != 4 { return 3; }

            var take_sum: i64 = 0;
            while true {
                let opt_v = take_s.next();
                if !opt_v.has_value { break; }
                take_sum = take_sum + opt_v.val;
            }
            if take_sum != 10 { return 4; } // 1 + 2 + 3 + 4 = 10

            // 3. StepStream: step 2 items (odd indices: 1, 3, 5, 7, 9)
            var s3 = SliceStream::create(data);
            var step_s = StepStream::create(s3, 2);
            var step_sum: i64 = 0;
            while true {
                let opt_v = step_s.next();
                if !opt_v.has_value { break; }
                step_sum = step_sum + opt_v.val;
            }
            if step_sum != 25 { return 5; } // 1 + 3 + 5 + 7 + 9 = 25

            // 4. stream_collect
            var s4 = SliceStream::create(data);
            let collected = stream_collect(var s4);
            if collected.len() != 10 { return 6; }
            if collected[0] != 1 || collected[9] != 10 { return 7; }

            return 42;
        }
    "#;

    let deps = &["stdlib/core/stream.pilot"];
    let res = eval_m23(deps, script);
    assert_eq!(res, "42", "Pull-cursor stream combinators must accurately evaluate sequences without allocation");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Stream Fusion Pass & Inlined Register Loop Comparison
// -----------------------------------------------------------------------------
#[test]
fn test_m23_2_stream_fusion_pass_and_loop_equivalence() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m23_2_stream_fusion_pass_and_loop_equivalence",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            // Pipeline: items.filter((x) => (x & 1) == 0).map((x) => x * 3 + 1).sum()
            var spec = stream_pipeline_spec_new("items", 1000, StreamTerminalKind::Sum);
            spec.stages.push(StreamPipelineStage {
                kind: StreamStageKind::Filter,
                predicate_op: "(x & 1) == 0",
                transform_expr: "",
                limit_val: 0,
            });
            spec.stages.push(StreamPipelineStage {
                kind: StreamStageKind::Map,
                predicate_op: "",
                transform_expr: "x * 3 + 1",
                limit_val: 0,
            });

            let plan = fuse_stream_pipeline(spec);
            if plan.stages_fused != 2 { return 1; }
            if plan.allocations_saved != 2 { return 2; } // Filter and Map intermediate buffers eliminated!
            if !plan.is_zero_alloc { return 3; } // Terminal Sum produces a scalar in SSA register (0 bytes heap)

            // A collecting terminal still allocates its output.
            var spec_c = stream_pipeline_spec_new("items", 1000, StreamTerminalKind::Collect);
            if fuse_stream_pipeline(spec_c).is_zero_alloc { return 4; }

            return 42;
        }
    "#;

    let deps = &["selfhost/ir/pass_stream_fusion.pilot"];
    let res = eval_m23(deps, script);
    assert_eq!(res, "42", "Stream fusion pass must eliminate intermediate allocations");

    // Rewriting the IR stops at its seam until the typed PilIR exists.
    let rewrite = eval_m23(
        deps,
        r#"
        var spec = stream_pipeline_spec_new("items", 4, StreamTerminalKind::Sum);
        rewrite_fused_pipeline(spec, fuse_stream_pipeline(spec));
    "#,
    );
    assert!(
        rewrite.starts_with("FAULT: SEAM001: unconnected seam `fusion.rewrite_ir`"),
        "fused loop rewrite must stop at its seam, got: {}",
        rewrite
    );

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: In-Place `each` Vectorization & Broadcast Diagnostics (FUSE001, FUSE002)
// -----------------------------------------------------------------------------
#[test]
fn test_m23_3_each_broadcast_and_diagnostics() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m23_3_each_broadcast_and_diagnostics",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            var diags: List<str> = [];

            // 1. In-place mutating each on mutable collection: statement position -> 0 alloc
            let b1 = lower_each_broadcast("scores", true, true, true, "+= 5", var diags);
            if diags.len() != 0 { return 1; }
            if b1.allocation_cost != 0 { return 2; }
            if b1.mode != BroadcastMode::InPlaceMutate { return 3; }

            // 2. Value-projecting each in expression position -> 1 alloc (output collection)
            let b2 = lower_each_broadcast("velocities", true, false, false, "* 0.5", var diags);
            if diags.len() != 0 { return 4; }
            if b2.allocation_cost != 1 { return 5; }
            if b2.mode != BroadcastMode::ValueProjection { return 6; }

            // 3. FUSE001: each applied to non-broadcastable scalar
            var diags_scalar: List<str> = [];
            let _ = lower_each_broadcast("single_int", false, true, true, "reset()", var diags_scalar);
            if diags_scalar.len() == 0 { return 7; }
            if !diags_scalar[0].contains("FUSE001") { return 8; }

            // 4. FUSE002: In-place mutating each on immutable 'let' collection
            var diags_immut: List<str> = [];
            let _ = lower_each_broadcast("frozen_list", true, false, true, "update()", var diags_immut);
            if diags_immut.len() == 0 { return 9; }
            if !diags_immut[0].contains("FUSE002") { return 10; }

            return 42;
        }
    "#;

    let deps = &["selfhost/ir/lower_each_cascade.pilot"];
    let res = eval_m23(deps, script);
    assert_eq!(res, "42", "each lowering must distinguish in-place mutation vs expression projection and emit FUSE001/FUSE002");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Fluid Cascades, Side-Effect Elimination & DTO Projections
// -----------------------------------------------------------------------------
#[test]
fn test_m23_4_cascades_side_effects_and_dto_projections() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m23_4_cascades_side_effects_and_dto_projections",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            var diags: List<str> = [];

            // 1. Regular cascade .. executes side effects exactly once
            let c1 = lower_cascade("make_box()", false, false, ["add(1)", "add(2)"], var diags);
            if diags.len() != 0 { return 1; }
            if c1.guards_receiver { return 2; }
            if c1.action_count != 2 { return 3; }

            // 2. Null-safe cascade ?.. on an optional receiver guards every action on one none-test
            let c2 = lower_cascade("opt_box", true, true, ["add(side_effect())"], var diags);
            if diags.len() != 0 { return 4; }
            if !c2.guards_receiver { return 5; }

            // 3. FUSE004: ?.. applied to non-optional receiver
            var diags_f4: List<str> = [];
            let _ = lower_cascade("regular_box", true, false, ["clear()"], var diags_f4);
            if diags_f4.len() == 0 { return 6; }
            if !diags_f4[0].contains("FUSE004") { return 7; }

            // 4. DTO Projection ->
            let src_fields: List<str> = ["id", "username", "email", "created_at"];
            let dst_fields: List<str> = ["id", "username", "email"];
            let dto_ok = lower_dto_projection("User", src_fields, "UserDto", dst_fields, false, var diags);
            if diags.len() != 0 { return 8; }
            if dto_ok.mapped_field_count != 3 { return 9; }
            if dto_ok.has_missing_fields { return 10; }

            // 5. FUSE003: DTO Projection missing required target field
            var diags_f3: List<str> = [];
            let bad_target_fields: List<str> = ["id", "username", "missing_token"];
            let dto_bad = lower_dto_projection("User", src_fields, "SecureDto", bad_target_fields, false, var diags_f3);
            if diags_f3.len() == 0 { return 11; }
            if !diags_f3[0].contains("FUSE003") { return 12; }
            if dto_bad.missing_field_name != "missing_token" { return 13; }

            return 42;
        }
    "#;

    let deps = &["selfhost/ir/lower_each_cascade.pilot"];
    let res = eval_m23(deps, script);
    assert_eq!(res, "42", "Cascades must guard optional receivers, and DTO projection must catch FUSE003/FUSE004");

    // Emitting the cascade into PilIR stops at its seam until the typed PilIR exists.
    let emit = eval_m23(
        deps,
        r#"
        var diags: List<str> = [];
        emit_cascade_ir(lower_cascade("make_box()", false, false, ["add(1)"], var diags));
    "#,
    );
    assert!(
        emit.starts_with("FAULT: SEAM001: unconnected seam `each_cascade.emit_ir`"),
        "cascade emission must stop at its seam, got: {}",
        emit
    );

    guard.finish();
}
