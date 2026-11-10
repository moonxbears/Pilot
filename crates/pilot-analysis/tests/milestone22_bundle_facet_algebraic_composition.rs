//! ============================================================================
//! Milestone 22 Verification Suite:
//! Complete *Bundle & *Facet Algebraic Composition Engine
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
    milestone_id: 22,
    test_file_rel: "crates/pilot-analysis/tests/milestone22_bundle_facet_algebraic_composition.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/types/bundle_splice.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "compute_struct_layout",
                "check_bundle_cycles",
                "bundle_def_new",
                "bundle_splice_directive_new",
                "struct_layout_plan_new",
            ],
            required_code_markers: &[
                "SpliceModifierKind",
                "PhysicalFieldSlot",
                "BundleFieldDef",
                "BundleMethodDef",
                "BundleDef",
                "BundleSpliceDirective",
                "StructLayoutPlan",
                "COMP001",
                "COMP002",
                "COMP003",
                "COMP004",
                "COMP005",
                "COMP006",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/types/method_table.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "method_table_new",
                "register_method",
                "resolve_static_delegation",
            ],
            required_code_markers: &[
                "MemberTier",
                "MethodTableEntry",
                "MethodTable",
                "Tier1ExplicitStruct",
                "Tier2SplicedBundleFacet",
                "Tier3SubInterfaceDefault",
                "Tier4BaseInterfaceDefault",
                "COMP004",
                "AmbiguousInterfaceDefault",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m22(deps: &[&str], script: &str) -> String {
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
// Test 1: Arbitrary 24-Field Layout Flattener & Cycle Detection (COMP002)
// -----------------------------------------------------------------------------
#[test]
fn test_m22_1_arbitrary_n_field_flattener_and_cycle_detection() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m22_1_arbitrary_n_field_flattener_and_cycle_detection",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            // 1. Build a 24-field struct proving complete removal of 6-field ceiling
            var plan = struct_layout_plan_new("BigEntity", false, 0);
            var prim_fields: List<BundleFieldDef> = [];

            var i: usize = 0;
            while i < 24 {
                let fname = "f_" + ("" + i);
                prim_fields.push(BundleFieldDef {
                    name: fname,
                    type_name: "i64",
                    layout_id: 1,
                    byte_size: 8,
                    lattice_bits: 0,
                    is_mutable: true,
                });
                i = i + 1;
            }

            let dirs: List<BundleSpliceDirective> = [];
            let catalog: Map<str, BundleDef> = [:];

            compute_struct_layout(var plan, prim_fields, dirs, catalog);

            if plan.physical_fields.len() != 24 { return 1; }
            if plan.total_size != 192 { return 2; } // 24 * 8 = 192 bytes
            if plan.alignment != 8 { return 3; }
            if plan.physical_fields[23].byte_offset != 184 { return 4; } // 23 * 8 = 184

            // 2. Test cycle detection COMP002 (A -> B -> A)
            var cat: Map<str, BundleDef> = [:];
            var bA = bundle_def_new("BundleA", false);
            bA.spliced_bundles.push("BundleB");

            var bB = bundle_def_new("BundleB", false);
            bB.spliced_bundles.push("BundleA");

            cat["BundleA"] = bA;
            cat["BundleB"] = bB;

            var diags: List<str> = [];
            let has_cycle = check_bundle_cycles(cat, "BundleA", var diags);
            if !has_cycle { return 5; }
            if diags.len() == 0 { return 6; }
            if !diags[0].contains("COMP002") { return 7; }

            return 42;
        }
    "#;

    let deps = &["selfhost/types/bundle_splice.pilot"];
    let res = eval_m22(deps, script);
    assert_eq!(res, "42", "Must flatten arbitrary 24 fields and detect cyclic bundle dependencies");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Horizontal Collision Algebra (coalesce, with, without, COMP003, COMP006)
// -----------------------------------------------------------------------------
#[test]
fn test_m22_2_horizontal_collision_algebra() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m22_2_horizontal_collision_algebra",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            var cat: Map<str, BundleDef> = [:];

            // BundleSpatial: has id: i64, x: f32, y: f32
            var b_spatial = bundle_def_new("SpatialBundle", false);
            b_spatial.fields.push(BundleFieldDef { name: "id", type_name: "i64", layout_id: 1, byte_size: 8, lattice_bits: 0, is_mutable: false });
            b_spatial.fields.push(BundleFieldDef { name: "x", type_name: "f32", layout_id: 2, byte_size: 4, lattice_bits: 0, is_mutable: true });
            b_spatial.fields.push(BundleFieldDef { name: "y", type_name: "f32", layout_id: 2, byte_size: 4, lattice_bits: 0, is_mutable: true });

            // BundleNetwork: has id: i64, flags: u32, scratch: i64
            var b_net = bundle_def_new("NetworkBundle", false);
            b_net.fields.push(BundleFieldDef { name: "id", type_name: "i64", layout_id: 1, byte_size: 8, lattice_bits: 0, is_mutable: false });
            b_net.fields.push(BundleFieldDef { name: "flags", type_name: "u32", layout_id: 3, byte_size: 4, lattice_bits: 0, is_mutable: true });
            b_net.fields.push(BundleFieldDef { name: "scratch", type_name: "i64", layout_id: 1, byte_size: 8, lattice_bits: 0, is_mutable: true });

            // Method ping accesses flags and scratch
            b_net.methods.push(BundleMethodDef {
                name: "ping",
                return_type: "void",
                params: "",
                is_mut: false,
                accessed_fields: ["flags", "scratch"],
            });

            cat["SpatialBundle"] = b_spatial;
            cat["NetworkBundle"] = b_net;

            // Scenario A: Clean coalesce on 'id', alias 'flags as net_flags', without { scratch } with ping also excluded
            var plan = struct_layout_plan_new("Actor", false, 0);
            var dir1 = bundle_splice_directive_new("SpatialBundle");
            var dir2 = bundle_splice_directive_new("NetworkBundle");
            dir2.coalesce_fields.push("id");
            dir2.with_aliases["flags"] = "net_flags";
            dir2.without_items.push("scratch");
            dir2.without_items.push("ping"); // ping excluded so scratch exclusion is safe!

            var dirs: List<BundleSpliceDirective> = [dir1, dir2];
            compute_struct_layout(var plan, [], dirs, cat);

            if plan.diagnostics.len() != 0 { return 1; }
            // Expected fields: Spatial.id, Spatial.x, Spatial.y, Network.net_flags -> 4 fields
            if plan.physical_fields.len() != 4 { return 2; }
            if plan.physical_fields[0].canonical_name != "id" { return 3; }
            if plan.physical_fields[3].canonical_name != "net_flags" { return 4; }

            // Verify remap table: both Spatial.id and Network.id point to offset 0
            if plan.remap_offsets["SpatialBundle.id"] != 0 { return 5; }
            if plan.remap_offsets["NetworkBundle.id"] != 0 { return 6; }

            // Scenario B: COMP006 - 'without { scratch }' but 'ping' is NOT excluded
            var plan_b = struct_layout_plan_new("BadActor", false, 0);
            var dir_bad = bundle_splice_directive_new("NetworkBundle");
            dir_bad.without_items.push("scratch"); // ping still active!
            var dirs_b: List<BundleSpliceDirective> = [dir_bad];
            compute_struct_layout(var plan_b, [], dirs_b, cat);

            if plan_b.diagnostics.len() == 0 { return 7; }
            if !plan_b.diagnostics[0].contains("COMP006") { return 8; }

            // Scenario C: COMP004 - Unresolved collision on 'id' without coalesce
            var plan_c = struct_layout_plan_new("CollidingActor", false, 0);
            var dir_c1 = bundle_splice_directive_new("SpatialBundle");
            var dir_c2 = bundle_splice_directive_new("NetworkBundle"); // no coalesce specified!
            var dirs_c: List<BundleSpliceDirective> = [dir_c1, dir_c2];
            compute_struct_layout(var plan_c, [], dirs_c, cat);

            var has_comp004 = false;
            var ci = 0;
            while ci < plan_c.diagnostics.len() {
                if plan_c.diagnostics[ci].contains("COMP004") { has_comp004 = true; }
                ci = ci + 1;
            }
            if !has_comp004 { return 9; }

            return 42;
        }
    "#;

    let deps = &["selfhost/types/bundle_splice.pilot"];
    let res = eval_m22(deps, script);
    assert_eq!(res, "42", "Must resolve coalesce/with/without algebra and catch COMP004 & COMP006");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Sealed Partial Struct (COMP001) & 2-Bit Lattice Law (COMP005)
// -----------------------------------------------------------------------------
#[test]
fn test_m22_3_sealed_partial_struct_and_lattice_law() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m22_3_sealed_partial_struct_and_lattice_law",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            var cat: Map<str, BundleDef> = [:];

            // 0-field Facet (InterpolateFacet)
            var facet = bundle_def_new("InterpolateFacet", true);
            facet.methods.push(BundleMethodDef {
                name: "lerp",
                return_type: "f32",
                params: "t: f32",
                is_mut: false,
                accessed_fields: [],
            });

            // Stateful bundle (StorageBundle)
            var storage = bundle_def_new("StorageBundle", false);
            storage.fields.push(BundleFieldDef {
                name: "bytes",
                type_name: "slice<u8>",
                layout_id: 10,
                byte_size: 16,
                lattice_bits: 1, // 0b01 = Reference View
                is_mutable: false,
            });

            cat["InterpolateFacet"] = facet;
            cat["StorageBundle"] = storage;

            // 1. Partial struct splicing 0-field Facet -> SUCCEEDS
            var plan_ok = struct_layout_plan_new("Entity", true, 0); // is_partial = true
            var dir_facet = bundle_splice_directive_new("InterpolateFacet");
            compute_struct_layout(var plan_ok, [], [dir_facet], cat);
            if plan_ok.diagnostics.len() != 0 { return 1; }
            if plan_ok.physical_fields.len() != 0 { return 2; }

            // 2. Partial struct splicing stateful StorageBundle -> COMP001
            var plan_fail = struct_layout_plan_new("Entity", true, 0); // is_partial = true
            var dir_storage = bundle_splice_directive_new("StorageBundle");
            compute_struct_layout(var plan_fail, [], [dir_storage], cat);
            if plan_fail.diagnostics.len() == 0 { return 3; }
            if !plan_fail.diagnostics[0].contains("COMP001") { return 4; }

            // 3. Primary struct declared IValue (0b00) splices 0b01 borrowed field -> COMP005
            var plan_lattice = struct_layout_plan_new("PureValue", false, 0); // declared_lattice = 0b00
            compute_struct_layout(var plan_lattice, [], [dir_storage], cat);
            if plan_lattice.diagnostics.len() == 0 { return 5; }
            var has_comp005 = false;
            var i = 0;
            while i < plan_lattice.diagnostics.len() {
                if plan_lattice.diagnostics[i].contains("COMP005") { has_comp005 = true; }
                i = i + 1;
            }
            if !has_comp005 { return 6; }

            return 42;
        }
    "#;

    let deps = &["selfhost/types/bundle_splice.pilot"];
    let res = eval_m22(deps, script);
    assert_eq!(res, "42", "Must enforce COMP001 on partial struct and COMP005 on 2-bit lattice violation");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Deterministic 4-Tier MethodTable & Static Delegation
// -----------------------------------------------------------------------------
#[test]
fn test_m22_4_four_tier_method_table_and_static_delegation() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m22_4_four_tier_method_table_and_static_delegation",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            var mt = method_table_new("MyStruct");

            // Register Tier 4 Base Interface method 'render'
            register_method(var mt, MethodTableEntry {
                method_name: "render",
                tier: MemberTier::Tier4BaseInterfaceDefault,
                origin_type_name: "IBaseRenderer",
                return_type: "void",
                params: "",
                is_mut: false,
                is_static: false,
                field_remap_bundle: "",
                clearance_lvl: 0,
                visibility: 1,
            }, false);

            if !mt.methods.has("render") { return 1; }
            if mt.methods["render"].origin_type_name != "IBaseRenderer" { return 2; }

            // Register Tier 2 Spliced Bundle method 'render' -> OVERRIDES Tier 4
            register_method(var mt, MethodTableEntry {
                method_name: "render",
                tier: MemberTier::Tier2SplicedBundleFacet,
                origin_type_name: "RenderBundle",
                return_type: "void",
                params: "",
                is_mut: false,
                is_static: false,
                field_remap_bundle: "RenderBundle",
                clearance_lvl: 0,
                visibility: 1,
            }, false);

            if mt.methods["render"].origin_type_name != "RenderBundle" { return 3; }
            if mt.methods["render"].tier != MemberTier::Tier2SplicedBundleFacet { return 4; }

            // Register Tier 1 Explicit Struct method 'render' -> WINS UNCONDITIONALLY
            register_method(var mt, MethodTableEntry {
                method_name: "render",
                tier: MemberTier::Tier1ExplicitStruct,
                origin_type_name: "MyStruct",
                return_type: "void",
                params: "",
                is_mut: false,
                is_static: false,
                field_remap_bundle: "",
                clearance_lvl: 0,
                visibility: 1,
            }, false);

            if mt.methods["render"].origin_type_name != "MyStruct" { return 5; }
            if mt.methods["render"].tier != MemberTier::Tier1ExplicitStruct { return 6; }

            // Splicing a competing Tier 2 method now does not collide because Tier 1 overrides
            register_method(var mt, MethodTableEntry {
                method_name: "render",
                tier: MemberTier::Tier2SplicedBundleFacet,
                origin_type_name: "AlternateRenderBundle",
                return_type: "void",
                params: "",
                is_mut: false,
                is_static: false,
                field_remap_bundle: "AlternateRenderBundle",
                clearance_lvl: 0,
                visibility: 1,
            }, false);

            if mt.diagnostics.len() != 0 { return 7; } // Overridden by Tier 1, so 0 collision diagnostics!
            if mt.methods["render"].origin_type_name != "MyStruct" { return 8; }

            // Test static delegation lookup: RenderBundle.render
            let static_entry = resolve_static_delegation(mt, "RenderBundle", "render");
            if static_entry.method_name != "render" { return 9; }
            if static_entry.origin_type_name != "RenderBundle" { return 10; }

            return 42;
        }
    "#;

    let deps = &["selfhost/types/method_table.pilot"];
    let res = eval_m22(deps, script);
    assert_eq!(res, "42", "Must resolve 4-tier precedence and support static bundle delegation");

    guard.finish();
}
