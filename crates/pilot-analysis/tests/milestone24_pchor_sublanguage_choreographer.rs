//! ============================================================================
//! Milestone 24 Verification Suite:
//! .pchor Sublanguage Choreographer Compiler (`blueprint`, `role`, `cue`, `route`)
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
    milestone_id: 24,
    test_file_rel: "crates/pilot-analysis/tests/milestone24_pchor_sublanguage_choreographer.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/chor/ast.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "blueprint_def_new",
                "role_contract_new",
            ],
            required_code_markers: &[
                "TransportKind",
                "BackpressurePolicy",
                "TransportSpec",
                "CueContract",
                "StorageBinding",
                "RoleContract",
                "RouteEdge",
                "ArchPolicyRule",
                "BlueprintDef",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/chor/parser.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "parse_pchor_source",
            ],
            required_code_markers: &[
                "blueprint",
                "storage",
                "role",
                "cue",
                "route",
                "law",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/chor/verifier.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "verify_blueprint_topology",
            ],
            required_code_markers: &[
                "CHOR001",
                "CHOR002",
                "CHOR003",
                "CHOR004",
                "CHOR005",
                "CHOR006",
                "CHOR007",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/chor/lower.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "lower_topology",
            ],
            required_code_markers: &[
                "LoweredRoutePlan",
                "LoweredTopologyPlan",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m24(deps: &[&str], script: &str) -> String {
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
// Test 1: Full .pchor Blueprint Parsing & AST Construction
// -----------------------------------------------------------------------------
#[test]
fn test_m24_1_pchor_parsing_and_ast() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m24_1_pchor_parsing_and_ast",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            let pchor_text = "blueprint GameTopology {\n"
                + "    storage Particles: Particle(strategy: \"SoA\", capacity: 100000);\n"
                + "    role Broadphase [HardRealtime] {\n"
                + "        reads: Particles.pos;\n"
                + "        emits: CollisionBatch;\n"
                + "    }\n"
                + "    role Solver {\n"
                + "        writes: Particles.pos;\n"
                + "        receives: CollisionBatch;\n"
                + "    }\n"
                + "    cue CollisionBatch(iso List<Pair>);\n"
                + "    route Broadphase.CollisionBatch -> Solver.handle (transport: ::RingBuffer(1024, ::CoalesceLatest));\n"
                + "    law PureDomain: role Broadphase cannot use std.net.*;\n"
                + "}\n";

            let bp = parse_pchor_source(pchor_text);

            if bp.name != "GameTopology" { return 1; }
            if bp.storages.len() != 1 { return 2; }
            if bp.storages[0].strategy != "SoA" { return 3; }
            if bp.storages[0].capacity != 100000 { return 15; }
            let bp_tiled = parse_pchor_source("blueprint T {\n    storage P: Particle(strategy: \"AoSoA\", capacity: 64);\n}\n");
            if bp_tiled.storages[0].strategy != "AoSoA" { return 18; }

            if bp.roles.len() != 2 { return 4; }
            if !bp.roles[0].is_hard_realtime { return 5; }
            if bp.roles[0].read_storages.len() != 1 { return 6; }
            if bp.roles[1].write_storages.len() != 1 { return 7; }

            if bp.cues.len() != 1 { return 8; }
            if !bp.cues[0].requires_iso_move { return 9; }
            if bp.cues[0].payload_type != "List<Pair>" { return 10; }
            // The lattice is left for the type checker; the parser never guesses it from a name.
            if bp.cues[0].payload_lattice != none { return 16; }

            if bp.routes.len() != 1 { return 11; }
            if bp.routes[0].transport.kind != TransportKind::RingBuffer { return 12; }
            if bp.routes[0].transport.capacity != 1024 { return 17; }

            if bp.laws.len() != 1 { return 13; }
            if bp.laws[0].target_role_or_pkg != "Broadphase" { return 14; }

            return 42;
        }
    "#;

    let deps = &[
        "selfhost/chor/ast.pilot",
        "selfhost/chor/parser.pilot",
    ];
    let res = eval_m24(deps, script);
    assert_eq!(res, "42", ".pchor parser must accurately construct BlueprintDef graph");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: 2-Bit Cue Boundary Law Enforcement (CHOR001)
// -----------------------------------------------------------------------------
#[test]
fn test_m24_2_two_bit_cue_boundary_law() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m24_2_two_bit_cue_boundary_law",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            // Scenario 1: Borrowed slice<u8> (0b01) in cue -> REJECTED (CHOR001)
            let pchor_bad1 = "blueprint Bad1 {\n"
                + "    cue BorrowedData(slice<u8>);\n"
                + "}\n";
            // The type checker would resolve each payload lattice; supply it here.
            var bp1 = parse_pchor_source(pchor_bad1);
            bp1.cues[0].payload_lattice = 1;
            let d1 = verify_blueprint_topology(bp1);
            if d1.len() == 0 { return 1; }
            if !d1[0].contains("CHOR001") { return 2; }

            // Scenario 2: Non-iso owned Buffer (0b10) in cue -> REJECTED (CHOR001)
            let pchor_bad2 = "blueprint Bad2 {\n"
                + "    cue NonIsoObject(SliceBuffer<u8>);\n"
                + "}\n";
            var bp2 = parse_pchor_source(pchor_bad2);
            bp2.cues[0].payload_lattice = 2;
            let d2 = verify_blueprint_topology(bp2);
            if d2.len() == 0 { return 3; }
            if !d2[0].contains("CHOR001") { return 4; }

            // Scenario 3: Scoped Guard (0b11) in cue -> REJECTED (CHOR001)
            let pchor_bad3 = "blueprint Bad3 {\n"
                + "    cue GuardData(MutexGuard<u8>);\n"
                + "}\n";
            var bp3 = parse_pchor_source(pchor_bad3);
            bp3.cues[0].payload_lattice = 3;
            let d3 = verify_blueprint_topology(bp3);
            if d3.len() == 0 { return 5; }
            if !d3[0].contains("CHOR001") { return 6; }

            // Scenario 4: Pure 0b00 Value (Vec3) in cue -> ALLOWED
            let pchor_ok = "blueprint Ok1 {\n"
                + "    cue ValueData(Vec3);\n"
                + "}\n";
            var bp4 = parse_pchor_source(pchor_ok);
            bp4.cues[0].payload_lattice = 0;
            let d4 = verify_blueprint_topology(bp4);
            if d4.len() != 0 { return 7; }

            // Scenario 5: iso-moved owned object (0b10) -> ALLOWED
            var bp5 = parse_pchor_source("blueprint Ok2 {\n    cue Moved(iso SliceBuffer<u8>);\n}\n");
            bp5.cues[0].payload_lattice = 2;
            if verify_blueprint_topology(bp5).len() != 0 { return 8; }

            return 42;
        }
    "#;

    let deps = &[
        "selfhost/chor/ast.pilot",
        "selfhost/chor/parser.pilot",
        "selfhost/chor/verifier.pilot",
    ];
    let res = eval_m24(deps, script);
    assert_eq!(res, "42", "2-Bit Cue Boundary Law must catch 0b01 borrowed views, 0b11 guards, and non-iso 0b10 objects");

    // Without a resolved payload lattice the verifier stops at its seam.
    let unresolved = eval_m24(
        deps,
        r#"
        verify_blueprint_topology(parse_pchor_source("blueprint U {\n    cue C(Vec3);\n}\n"));
    "#,
    );
    assert!(
        unresolved.starts_with("FAULT: SEAM001: unconnected seam `chor.cue_lattice`"),
        "unresolved cue lattice must stop at its seam, got: {}",
        unresolved
    );

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Route Cycles (CHOR004), Real-Time (CHOR005), Unhandled Cues (CHOR003)
// -----------------------------------------------------------------------------
#[test]
fn test_m24_3_route_cycles_and_realtime_checks() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m24_3_route_cycles_and_realtime_checks",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            // 1. DirectCall cycle: A -> B -> A over DirectCall -> CHOR004
            let pchor_cycle = "blueprint CycleBp {\n"
                + "    cue CueAB(i64);\n"
                + "    cue CueBA(i64);\n"
                + "    role RoleA { emits: CueAB; }\n"
                + "    role RoleB { emits: CueBA; }\n"
                + "    route RoleA.CueAB -> RoleB.h;\n"
                + "    route RoleB.CueBA -> RoleA.h;\n"
                + "}\n";
            var bp1 = parse_pchor_source(pchor_cycle);
            bp1.cues[0].payload_lattice = 0;
            bp1.cues[1].payload_lattice = 0;
            let d1 = verify_blueprint_topology(bp1);
            var has_chor004 = false;
            var i = 0;
            while i < d1.len() {
                if d1[i].contains("CHOR004") { has_chor004 = true; }
                i = i + 1;
            }
            if !has_chor004 { return 1; }

            // 1b. Three-role DirectCall cycle A -> B -> C -> A is reported once
            var bp_tri = parse_pchor_source("blueprint Tri {\n"
                + "    route RA.X -> RB.h;\n"
                + "    route RB.Y -> RC.h;\n"
                + "    route RC.Z -> RA.h;\n"
                + "}\n");
            let d_tri = verify_blueprint_topology(bp_tri);
            var tri_count = 0;
            i = 0;
            while i < d_tri.len() {
                if d_tri[i].contains("CHOR004") { tri_count = tri_count + 1; }
                i = i + 1;
            }
            if tri_count != 1 { return 4; }

            // 1c. Acyclic chain and a cycle broken by a RingBuffer -> no CHOR004
            var bp_dag = parse_pchor_source("blueprint Dag {\n"
                + "    route RA.X -> RB.h;\n"
                + "    route RB.Y -> RC.h;\n"
                + "    route RA.W -> RC.h;\n"
                + "    route RC.Z -> RA.h (transport: ::RingBuffer(64, ::TryFail));\n"
                + "}\n");
            let d_dag = verify_blueprint_topology(bp_dag);
            i = 0;
            while i < d_dag.len() {
                if d_dag[i].contains("CHOR004") { return 5; }
                i = i + 1;
            }

            // 2. [HardRealtime] role over blocking transport -> CHOR005
            let pchor_hrt = "blueprint HrtBp {\n"
                + "    cue SampleCue(i64);\n"
                + "    role Sensor [HardRealtime] { emits: SampleCue; }\n"
                + "    role Logger { receives: SampleCue; }\n"
                + "    route Sensor.SampleCue -> Logger.log (transport: ::Block);\n"
                + "}\n";
            var bp2 = parse_pchor_source(pchor_hrt);
            bp2.cues[0].payload_lattice = 0;
            let d2 = verify_blueprint_topology(bp2);
            var has_chor005 = false;
            i = 0;
            while i < d2.len() {
                if d2[i].contains("CHOR005") { has_chor005 = true; }
                i = i + 1;
            }
            if !has_chor005 { return 2; }

            // 3. Emitted cue with zero active routes -> CHOR003
            let pchor_unhandled = "blueprint UnhandledBp {\n"
                + "    cue OrphanCue(i64);\n"
                + "    role Sender { emits: OrphanCue; }\n"
                + "}\n";
            var bp3 = parse_pchor_source(pchor_unhandled);
            bp3.cues[0].payload_lattice = 0;
            let d3 = verify_blueprint_topology(bp3);
            var has_chor003 = false;
            i = 0;
            while i < d3.len() {
                if d3[i].contains("CHOR003") { has_chor003 = true; }
                i = i + 1;
            }
            if !has_chor003 { return 3; }

            return 42;
        }
    "#;

    let deps = &[
        "selfhost/chor/ast.pilot",
        "selfhost/chor/parser.pilot",
        "selfhost/chor/verifier.pilot",
    ];
    let res = eval_m24(deps, script);
    assert_eq!(res, "42", "Must detect direct-call cycles (CHOR004), HRT blocking (CHOR005), and unhandled cues (CHOR003)");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Architecture Laws (CHOR006), Lowering & Mermaid Diagram Export
// -----------------------------------------------------------------------------
#[test]
fn test_m24_4_laws_lowering_and_mermaid_export() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m24_4_laws_lowering_and_mermaid_export",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            let pchor_text = "blueprint Pipeline {\n"
                + "    storage Data: Buffer(strategy: \"AoS\", capacity: 2048);\n"
                + "    role Producer {\n"
                + "        emits: ItemCue;\n"
                + "    }\n"
                + "    role Consumer {\n"
                + "        receives: ItemCue;\n"
                + "    }\n"
                + "    cue ItemCue(i64);\n"
                + "    route Producer.ItemCue -> Consumer.process (transport: ::RingBuffer(512, ::CoalesceLatest));\n"
                + "    law NoFsInConsumer: role Consumer cannot use std.fs.*;\n"
                + "}\n";

            var bp = parse_pchor_source(pchor_text);
            bp.cues[0].payload_lattice = 0; // i64 is 0b00

            // 1. Test Law Enforcement CHOR006
            bp.roles[1].prohibited_imports.push("std.fs.File"); // Consumer illegally imports std.fs
            let diags = verify_blueprint_topology(bp);
            var has_chor006 = false;
            var i = 0;
            while i < diags.len() {
                if diags[i].contains("CHOR006") { has_chor006 = true; }
                i = i + 1;
            }
            if !has_chor006 { return 1; }

            // A package that merely shares a prefix is not under std.fs.*
            bp.roles[1].prohibited_imports = ["std.fsx.Thing"];
            let diags_prefix = verify_blueprint_topology(bp);
            i = 0;
            while i < diags_prefix.len() {
                if diags_prefix[i].contains("CHOR006") { return 11; }
                i = i + 1;
            }

            // Clear illegal import
            bp.roles[1].prohibited_imports = [];

            // 2. Test Topology Lowering
            let lowered = lower_topology(bp);
            if lowered.routes.len() != 1 { return 2; }
            if lowered.routes[0].transport_kind != TransportKind::RingBuffer { return 3; }
            if lowered.routes[0].ring_buffer_cap != 512 { return 4; } // declared RingBuffer(512, ..)
            if !lowered.routes[0].is_zero_copy { return 5; }

            // 3. Test Mermaid Diagram Generation
            let mermaid = export_mermaid_diagram(bp);
            if !mermaid.contains("flowchart LR") { return 6; }
            if !mermaid.contains("subgraph Storages") { return 7; }
            if !mermaid.contains("role_Producer") { return 8; }
            if !mermaid.contains("role_Consumer") { return 9; }
            if !mermaid.contains("ItemCue (RingBuffer)") { return 10; }

            return 42;
        }
    "#;

    let deps = &[
        "selfhost/chor/ast.pilot",
        "selfhost/chor/parser.pilot",
        "selfhost/chor/verifier.pilot",
        "selfhost/chor/lower.pilot",
        "stdlib/pilot/chor.pilot",
    ];
    let res = eval_m24(deps, script);
    assert_eq!(res, "42", "Must enforce laws (CHOR006), compile lowered route plans, and export Mermaid diagrams");

    // Emitting the dispatchers into PilIR stops at its seam until the typed PilIR exists.
    let emit = eval_m24(
        deps,
        r#"
        emit_topology_ir(lower_topology(parse_pchor_source("blueprint E {\n    route A.X -> B.h;\n}\n")));
    "#,
    );
    assert!(
        emit.starts_with("FAULT: SEAM001: unconnected seam `chor.emit_dispatchers`"),
        "dispatcher emission must stop at its seam, got: {}",
        emit
    );

    guard.finish();
}
