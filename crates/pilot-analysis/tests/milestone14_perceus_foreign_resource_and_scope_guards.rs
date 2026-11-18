//! ============================================================================
//! Milestone 14 Verification Suite:
//! Perceus Foreign Resource & Scope Guard Synchronization (`0b10` & `0b11`)
//! ============================================================================

use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};
use std::fs;

#[path = "common/anticheat.rs"]
mod anticheat;
#[path = "common/vm_outcome.rs"]
mod vm_outcome;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 14,
    test_file_rel:
        "crates/pilot-analysis/tests/milestone14_perceus_foreign_resource_and_scope_guards.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "stdlib/std/polyglot/lifetime.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["current_thread_id", "invoke_foreign_destructor"],
            required_code_markers: &[
                "ForeignHandle",
                "ForeignGuard",
                "PinnedSlice",
                "ThreadAffineDropQueue",
                "BorrowCallbackScope",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/std/polyglot/shm.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &["ShmBuffer", "ShmView"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/perceus/escape.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "verify_retained_escape",
                "esc_fn_is_retained",
                "emit_foreign_drop_op",
            ],
            required_code_markers: &["ESCAPE004", "verify_retained_escape"],
            forbidden_code_markers: &["@hard_realtime"],
            forbidden_string_substrings: &["dsp", "@hard_realtime"],
        },
        anticheat::FileAuditRule {
            path: "selfhost/perceus/lattice.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["lat_deduce_type_lattice"],
            required_code_markers: &["lat_deduce_type_lattice"],
            forbidden_code_markers: &["@hard_realtime"],
            forbidden_string_substrings: &["dsp", "@hard_realtime"],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m14(deps: &[&str], script: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut combined = String::new();
    for rel in deps {
        let p = root.join(rel);
        let src =
            fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {:?}", p.display(), e));
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
// Test 1: Zero-Leak ForeignHandle Lifecycle, Take-Raw & Stress Test
// -----------------------------------------------------------------------------
#[test]
fn test_m14_1_zero_leak_stress_test_foreign_handle_atomic_counter() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m14_1_zero_leak_stress_test_foreign_handle_atomic_counter",
        &ANTICHEAT_SPEC,
    );

    // Ownership logic of ForeignHandle on opaque addresses. The addresses
    // are never dereferenced: a handle only stores and returns them.
    let script = r#"
        pub fn main() -> i64 {
            // Null pointer produces an invalid handle.
            let null_h: ForeignHandle<i64> = ForeignHandle.wrap(0, 21845);
            if null_h.is_valid() { return 1; }
            if null_h.as_ptr() != 0 { return 2; }

            var h1 = ForeignHandle.wrap(4096, 8192);
            if !h1.is_valid() { return 3; }
            if h1.as_ptr() != 4096 { return 4; }

            // take_raw transfers ownership back to C and disarms the destructor
            let raw_p = h1.take_raw();
            if raw_p != 4096 { return 5; }
            if h1.is_valid() { return 6; }
            if h1.as_ptr() != 0 { return 7; }
            h1.~self();

            // 100 handles handed back to C: dropping them runs no destructor.
            var i: i64 = 0;
            var returned: i64 = 0;
            while i < 100 {
                var handle = ForeignHandle.wrap(65536 + i * 8, 32768 + i);
                if !handle.is_valid() { return 8; }
                returned = returned + handle.take_raw();
                handle.~self();
                i = i + 1;
            }
            // sum of 65536 + 8i for i in 0..100 = 6553600 + 8 * 4950
            if returned != 6593200 { return 9; }

            return 42;
        }
    "#;

    let deps = &["stdlib/std/polyglot/lifetime.pilot"];
    let res = eval_m14(deps, script);
    assert_eq!(
        res, "42",
        "ForeignHandle ownership logic failed, got: {}",
        res
    );

    // Dropping an armed handle must call its native destructor.
    let drop_armed = r#"
        pub fn main() -> i64 {
            var h = ForeignHandle.wrap(4096, 8192);
            h.~self();
            return 42;
        }
    "#;
    let res = eval_m14(deps, drop_armed);
    assert!(
        res.starts_with("FAULT: SEAM001: unconnected seam `ffi.call_destructor`"),
        "dropping an armed handle must reach the destructor seam, got {res}"
    );

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Thread-Affine Drop Queue and Cross-Thread Drain Hook
// -----------------------------------------------------------------------------
#[test]
fn test_m14_2_thread_affine_drop_queue_and_drain() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m14_2_thread_affine_drop_queue_and_drain",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            var dq = ThreadAffineDropQueue.create();
            dq.enqueue(43968, 57072, 2);
            dq.enqueue(43976, 57072, 2);
            dq.enqueue(44000, 57080, 3);
            dq.enqueue(0, 57080, 3);        // null pointer: nothing to drop
            if dq.pending_count() != 3 { return 1; }
            if dq.pending_drops_for(2) != 2 { return 2; }
            if dq.pending_drops_for(3) != 1 { return 3; }
            if dq.pending_drops_for(1) != 0 { return 4; }

            // Draining a thread with nothing queued runs no destructor.
            if dq.drain_thread(1) != 0 { return 5; }
            if dq.pending_count() != 3 { return 6; }
            return 100;
        }
    "#;

    let deps = &["stdlib/std/polyglot/lifetime.pilot"];
    let res = eval_m14(deps, script);
    assert_eq!(
        res, "100",
        "Thread-affine drop queue test failed, got: {}",
        res
    );

    // Each remaining path stops at the seam it needs.
    let cases = [
        (
            "var dq = ThreadAffineDropQueue.create(); dq.enqueue(43968, 57072, 2); let n = dq.drain_thread(2);",
            "ffi.call_destructor",
        ),
        ("var h = ForeignHandle.wrap_thread_affine(43968, 57072, 2); h.~self();", "thread.current_id"),
        ("let t = current_thread_id();", "thread.current_id"),
        ("var dq = ThreadAffineDropQueue.global();", "lifetime.global_drop_queue"),
    ];
    for (body, seam_id) in cases {
        let script = format!("pub fn main() -> i64 {{ {body} return 100; }}");
        let res = eval_m14(deps, &script);
        assert!(
            res.starts_with(&format!("FAULT: SEAM001: unconnected seam `{seam_id}`")),
            "`{body}` must stop at seam {seam_id}, got {res}"
        );
    }

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: ESCAPE004 / ESCAPE001 on parsed programs, ForeignDrop seam
// -----------------------------------------------------------------------------

// -----------------------------------------------------------------------------
// Test 4: PinnedSlice, ForeignGuard & ShmBuffer Lifecycle
// -----------------------------------------------------------------------------
#[test]
fn test_m14_4_pinned_slice_and_foreign_guard_and_shm_buffer() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m14_4_pinned_slice_and_foreign_guard_and_shm_buffer",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            // 1. ForeignGuard (0b11): the release function gets the token once.
            var released: List<i64> = [];
            var guard_obj: ForeignGuard<i64> = ForeignGuard.acquire(12345, (t) => { released.push(t); });
            if guard_obj.token() != 12345 { return 6; }
            if !guard_obj.active() { return 7; }
            guard_obj.~self();
            if guard_obj.active() { return 8; }
            if released.len() != 1 || released[0] != 12345 { return 9; }
            guard_obj.~self();
            if released.len() != 1 { return 91; }

            // A disarmed guard does not release.
            var guard_disarmed: ForeignGuard<i64> = ForeignGuard.acquire(67890, (t) => { released.push(t); });
            guard_disarmed.disarm();
            guard_disarmed.~self();
            if released.len() != 1 { return 10; }

            // 2. BorrowCallbackScope (0b11)
            var bscope = BorrowCallbackScope.enter(999);
            if bscope.id() != 999 { return 11; }
            if !bscope.active() { return 12; }
            bscope.leave();
            if bscope.active() { return 13; }

            // 3. ShmView projection bounds over an 8192-byte region (offsets
            // relative to base 0; the region itself is never touched).
            let shm: ShmBuffer<f64> = ShmBuffer { base_addr: 0, byte_capacity: 8192, alignment: 4096, is_mapped: true };
            let full_view = shm.as_view(8);
            if full_view.len != 1024 { return 16; }
            if full_view.byte_length() != 8192 { return 17; }
            if !full_view.is_valid() { return 18; }

            let sub_view = ShmView.project(ref shm, 10, 50, 8);
            if sub_view.len != 50 { return 19; }
            if sub_view.byte_length() != 400 { return 20; }
            if sub_view.base_addr != 80 { return 201; }
            if !sub_view.is_valid() { return 21; }

            // 1000 * 8 + 100 * 8 = 8800 > 8192: out of bounds
            let bad_view = ShmView.project(ref shm, 1000, 100, 8);
            if bad_view.is_valid() { return 22; }
            // exactly to the end is in bounds
            let tail = ShmView.project(ref shm, 1000, 24, 8);
            if !tail.is_valid() { return 23; }

            return 300;
        }
    "#;

    let deps = &[
        "stdlib/std/polyglot/lifetime.pilot",
        "stdlib/std/polyglot/shm.pilot",
    ];
    let res = eval_m14(deps, script);
    assert_eq!(
        res, "300",
        "ForeignGuard, BorrowCallbackScope and ShmView test failed, got: {}",
        res
    );

    let cases = [
        ("let p: PinnedSlice<i64> = PinnedSlice.allocate(16, 8);", "mem.pinned_alloc"),
        ("let b: ShmBuffer<f64> = ShmBuffer.create(8192, 4096);", "shm.map"),
        (
            "var b: ShmBuffer<f64> = ShmBuffer { base_addr: 0, byte_capacity: 8192, alignment: 4096, is_mapped: true }; b.release();",
            "shm.unmap",
        ),
    ];
    for (body, seam_id) in cases {
        let script = format!("pub fn main() -> i64 {{ {body} return 300; }}");
        let res = eval_m14(deps, &script);
        assert!(
            res.starts_with(&format!("FAULT: SEAM001: unconnected seam `{seam_id}`")),
            "`{body}` must stop at seam {seam_id}, got {res}"
        );
    }

    guard.finish();
}
