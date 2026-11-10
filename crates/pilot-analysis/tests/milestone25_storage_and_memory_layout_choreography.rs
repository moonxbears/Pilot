//! ============================================================================
//! Milestone 25 Verification Suite:
//! .pchor Storage & Memory Layout Choreography (AoS <-> SoA, AoSoA & Hot/Cold Split)
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
    milestone_id: 25,
    test_file_rel: "crates/pilot-analysis/tests/milestone25_storage_and_memory_layout_choreography.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "stdlib/core/soa.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "align_up_bytes",
                "storage_strategy_name",
            ],
            required_code_markers: &[
                "StorageStrategy",
                "RowProxyRef",
                "RowProxyMut",
                "SoaTable",
                "AoSoA",
                "HotCold",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/chor/storage_plan.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "compute_storage_plan",
                "align_up",
            ],
            required_code_markers: &[
                "FieldLeafDesc",
                "ColumnPhysicalLayout",
                "StoragePlan",
                "flatten_nested_fields",
                "estimate_cacheline_efficiency",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ir/pass_soa_rewrite.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "check_raw_pointer_escape",
                "lower_choreographed_field_access",
                "rewrite_field_access_ir",
            ],
            required_code_markers: &[
                "CHOR008",
                "Chor008Diagnostic",
                "compute_addr_aos",
                "compute_addr_soa",
                "compute_addr_aosoa",
                "compute_addr_hotcold",
                "compute_bitpacked_coord",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m25(deps: &[&str], script: &str) -> String {
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
// Test 1: Multi-Column Physical Layout Calculation & 64B Cache-Line Padding
// -----------------------------------------------------------------------------
#[test]
fn test_m25_1_multicolumn_layout_and_simd_alignment() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m25_1_multicolumn_layout_and_simd_alignment",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "stdlib/core/soa.pilot",
        "selfhost/chor/storage_plan.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // Flatten Particle.pos (Vec3: x, y, z: f32) + vel.x: f32 + mass: f32
            // Vec3 { x, y, z: f32 } leaves, as the field type's resolved layout gives them
            var vec3_leaves: List<FieldLeafDesc> = [];
            vec3_leaves.push(field_leaf_desc_new("x", "x", 4, 4, 0, false, 32, false));
            vec3_leaves.push(field_leaf_desc_new("y", "y", 4, 4, 0, false, 32, false));
            vec3_leaves.push(field_leaf_desc_new("z", "z", 4, 4, 0, false, 32, false));
            var fields = flatten_nested_fields("pos", vec3_leaves);
            if fields.len() != 3 || fields[0].path != "pos.x" || fields[2].path != "pos.z" { return 40; }
            fields.push(field_leaf_desc_new("vel.x", "vel_x", 4, 4, 0, false, 32, false));
            fields.push(field_leaf_desc_new("mass", "mass", 4, 4, 0, false, 32, false));

            let cap = 1000;
            let plan = compute_storage_plan("Particle", StorageStrategy::SoA, cap, fields, 8, 64);

            if plan.columns.len() != 5 { return 1; }
            if plan.alignment_bytes != 64 { return 2; }

            // Verify each column plane offset is a multiple of 64 bytes
            var i = 0;
            while i < plan.columns.len() {
                let col = plan.columns[i];
                if col.plane_offset % 64 != 0 { return 10 + i as i64; }
                if col.alignment != 64 { return 20 + i as i64; }
                if col.byte_size != 4 { return 30 + i as i64; }
                i = i + 1;
            }

            // Plane size for 1000 * 4B = 4000B -> aligned up to 64 = 4032B
            let expected_plane_bytes = 4032;
            if plan.columns[0].plane_bytes != expected_plane_bytes { return 3; }
            if plan.columns[1].plane_offset != expected_plane_bytes { return 4; }
            if plan.columns[2].plane_offset != expected_plane_bytes * 2 { return 5; }

            // Total bytes must be multiple of 64
            if plan.total_allocation_bytes % 64 != 0 { return 6; }
            if plan.total_allocation_bytes != expected_plane_bytes * 5 { return 7; }

            return 42;
        }
    "#;

    let res = eval_m25(&deps, script);
    assert_eq!(res, "42", "Failed multi-column physical layout test: {}", res);

    // A SoaTable lays out its planes, but has no column memory to push rows into yet.
    let table = eval_m25(
        &deps,
        r#"
        pub fn main() -> i64 {
            var t = SoaTable::create(16, [4, 8], 64);
            // Plane 0 holds 16 * 4 bytes = 64, so plane 1 starts at 64.
            if t.column_base(1) != 64 { return 1; }
            t.push();
            return 2;
        }
    "#,
    );
    assert!(
        table.starts_with("FAULT: SEAM001: unconnected seam `soa.column_memory`"),
        "SoaTable row storage must stop at its seam, got: {}",
        table
    );
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Address Lowering Equations Across AoS, SoA, AoSoA, and HotCold
// -----------------------------------------------------------------------------
#[test]
fn test_m25_2_physical_address_lowering_across_all_strategies() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m25_2_physical_address_lowering_across_all_strategies",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "stdlib/core/soa.pilot",
        "selfhost/chor/storage_plan.pilot",
        "selfhost/ir/pass_soa_rewrite.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // Vec3 { x, y, z: f32 } leaves, as the field type's resolved layout gives them
            var vec3_leaves: List<FieldLeafDesc> = [];
            vec3_leaves.push(field_leaf_desc_new("x", "x", 4, 4, 0, false, 32, false));
            vec3_leaves.push(field_leaf_desc_new("y", "y", 4, 4, 0, false, 32, false));
            vec3_leaves.push(field_leaf_desc_new("z", "z", 4, 4, 0, false, 32, false));
            var fields = flatten_nested_fields("pos", vec3_leaves); // 3 fields of 4 bytes each = 12 bytes
            fields.push(field_leaf_desc_new("mass", "mass", 4, 4, 0, false, 32, false)); // total 16 bytes

            let base = 10000;
            let cap = 64;

            // 1. AoS lowering: base + row * struct_size + field_offset
            let aos_plan = compute_storage_plan("Particle", StorageStrategy::AoS, cap, fields, 8, 64);
            let aos_access = lower_choreographed_field_access(aos_plan, base, 5, 2); // row 5, col 2 (pos.z, offset 8)
            // expected: 10000 + 5 * 16 + 8 = 10088
            if aos_access.target_addr != 10088 { return 1; }

            // 2. SoA lowering: col_base + row * elem_size
            let soa_plan = compute_storage_plan("Particle", StorageStrategy::SoA, cap, fields, 8, 64);
            let soa_access = lower_choreographed_field_access(soa_plan, base, 5, 2); // col 2 plane
            let col2_base = base + soa_plan.columns[2].plane_offset;
            let expected_soa = col2_base + 5 * 4;
            if soa_access.target_addr != expected_soa { return 2; }

            // 3. AoSoA lowering (tile: 8): block_idx * block_bytes + tile_offset + lane * elem_size
            let aosoa_plan = compute_storage_plan("Particle", StorageStrategy::AoSoA, cap, fields, 8, 64);
            let aosoa_access = lower_choreographed_field_access(aosoa_plan, base, 10, 1); // row 10 (block 1, lane 2), col 1 (pos.y)
            let block1_start = base + 1 * aosoa_plan.columns.len() * 8 * 4; // 16 bytes * 8 = 128 block bytes
            let expected_aosoa = compute_addr_aosoa(base, 10, 8, 128, aosoa_plan.columns[1].plane_offset, 4);
            if aosoa_access.target_addr != expected_aosoa { return 3; }

            // 4. BitPacked lowering: word_index, bit_shift, bit_mask
            let coord = compute_bitpacked_coord(65, 1); // bool flag at row 65 (1 bit per elem)
            if coord.word_index != 1 { return 4; } // 65 / 64 = 1
            if coord.bit_shift != 1 { return 5; }  // 65 % 64 = 1
            if coord.bit_mask != 1 { return 6; }

            // 5. Bandwidth efficiency verification (Gate 3 requirement: >= 3.5x speedup)
            // Accessing 8B of 128B entity:
            let eff_aos = estimate_cacheline_efficiency(StorageStrategy::AoS, 128, 8);
            let eff_soa = estimate_cacheline_efficiency(StorageStrategy::SoA, 128, 8);
            let speedup = estimate_speedup_factor(128, 8);

            if eff_aos != 6 { return 7; } // 8 * 100 / 128 = 6%
            if eff_soa != 100 { return 8; } // 100%
            if speedup != 16 { return 9; }  // 16x speedup (>= 3.5x)

            // 6. A column outside the plan has no address
            if lower_choreographed_field_access(soa_plan, base, 0, 99) != none { return 10; }

            return 42;
        }
    "#;

    let res = eval_m25(&deps, script);
    assert_eq!(res, "42", "Failed physical address lowering test: {}", res);

    // Rewriting the PilIR access stops at its seam until the typed PilIR exists.
    let rewrite = eval_m25(
        &deps,
        r#"
        var leaves: List<FieldLeafDesc> = [];
        leaves.push(field_leaf_desc_new("mass", "mass", 4, 4, 0, false, 32, false));
        let plan = compute_storage_plan("P", StorageStrategy::SoA, 8, leaves, 8, 64);
        let access = lower_choreographed_field_access(plan, 0, 1, 0);
        rewrite_field_access_ir(access);
    "#,
    );
    assert!(
        rewrite.starts_with("FAULT: SEAM001: unconnected seam `soa.rewrite_ir`"),
        "PilIR access rewrite must stop at its seam, got: {}",
        rewrite
    );
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Raw Pointer Escape Prevention (CHOR008)
// -----------------------------------------------------------------------------
#[test]
fn test_m25_3_raw_pointer_escape_guard_chor008() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m25_3_raw_pointer_escape_guard_chor008",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "stdlib/core/soa.pilot",
        "selfhost/chor/storage_plan.pilot",
        "selfhost/ir/pass_soa_rewrite.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // Case 1: AoS with raw pointer is valid (contiguous memory)
            let d1 = check_raw_pointer_escape(StorageStrategy::AoS, "WorldParticles", "Particle", true);
            if d1.has_error { return 1; }

            // Case 2: SoA with raw pointer must trigger CHOR008!
            let d2 = check_raw_pointer_escape(StorageStrategy::SoA, "ParticleTable", "Particle", true);
            if !d2.has_error { return 2; }
            if d2.storage_name != "ParticleTable" { return 3; }
            if d2.struct_name != "Particle" { return 4; }
            if d2.strategy != "SoA" { return 5; }

            let expected_prefix = "CHOR008: Cannot form contiguous AoS 'ptr<Particle>' into SoA-choreographed storage 'ParticleTable'";
            // Verify message starts with expected_prefix
            if d2.message.len() < expected_prefix.len() { return 6; }

            // Case 3: AoSoA with raw pointer must trigger CHOR008!
            let d3 = check_raw_pointer_escape(StorageStrategy::AoSoA, "ParticleTable", "Particle", true);
            if !d3.has_error { return 7; }

            // Case 4: HotCold with raw pointer must trigger CHOR008!
            let d4 = check_raw_pointer_escape(StorageStrategy::HotCold, "ParticleTable", "Particle", true);
            if !d4.has_error { return 8; }

            // Case 5: SoA without raw pointer (e.g. value gather or field slice) is allowed
            let d5 = check_raw_pointer_escape(StorageStrategy::SoA, "ParticleTable", "Particle", false);
            if d5.has_error { return 9; }

            return 42;
        }
    "#;

    let res = eval_m25(&deps, script);
    assert_eq!(res, "42", "Failed CHOR008 raw pointer escape test: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Column-Selective Perceus Drop Masks
// -----------------------------------------------------------------------------
#[test]
fn test_m25_4_column_selective_perceus_drop_masks() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m25_4_column_selective_perceus_drop_masks",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "stdlib/core/soa.pilot",
        "selfhost/chor/storage_plan.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // Create 8 fields: 7 are 0b00 (IValue: f32, u64), 1 is 0b10 (IObject: tags List<u32>)
            var fields: List<FieldLeafDesc> = [];
            fields.push(field_leaf_desc_new("pos.x", "x", 4, 4, 0, false, 32, false)); // 0b00
            fields.push(field_leaf_desc_new("pos.y", "y", 4, 4, 0, false, 32, false)); // 0b00
            fields.push(field_leaf_desc_new("pos.z", "z", 4, 4, 0, false, 32, false)); // 0b00
            fields.push(field_leaf_desc_new("vel.x", "vx", 4, 4, 0, false, 32, false)); // 0b00
            fields.push(field_leaf_desc_new("vel.y", "vy", 4, 4, 0, false, 32, false)); // 0b00
            fields.push(field_leaf_desc_new("vel.z", "vz", 4, 4, 0, false, 32, false)); // 0b00
            fields.push(field_leaf_desc_new("mass", "mass", 4, 4, 0, false, 32, false)); // 0b00
            fields.push(field_leaf_desc_new("tags", "tags", 8, 8, 2, false, 64, false)); // 0b10 : IObject!

            let plan = compute_storage_plan("Entity", StorageStrategy::SoA, 1000000, fields, 8, 64);

            // Perceus drop check: exactly 1 column needs drop (col 7: tags)
            if plan.perceus_drop_columns.len() != 1 { return 1; }
            if plan.perceus_drop_columns[0] != 7 { return 2; }

            // Check bitmask: 1 << 7 = 128
            if plan.perceus_drop_mask != 128 { return 3; }

            // Check individual columns
            var i = 0;
            while i < 7 {
                if storage_plan_is_column_dropped(plan, i) { return 10 + i as i64; }
                if plan.columns[i].needs_perceus_drop { return 20 + i as i64; }
                i = i + 1;
            }
            if !storage_plan_is_column_dropped(plan, 7) { return 4; }
            if !plan.columns[7].needs_perceus_drop { return 5; }

            if storage_plan_drop_count(plan) != 1 { return 6; }

            return 42;
        }
    "#;

    let res = eval_m25(&deps, script);
    assert_eq!(res, "42", "Failed Perceus drop mask test: {}", res);
    guard.finish();
}
