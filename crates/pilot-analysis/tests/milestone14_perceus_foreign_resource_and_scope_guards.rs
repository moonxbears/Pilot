//! ============================================================================
//! Milestone 14 Verification Suite:
//! Perceus Foreign Resource & Scope Guard Synchronization (`0b10` & `0b11`)
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 14,
    test_file_rel: "crates/pilot-analysis/tests/milestone14_perceus_foreign_resource_and_scope_guards.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "stdlib/std/polyglot/lifetime.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "current_thread_id",
                "invoke_foreign_destructor",
                "live_foreign_destructions_count",
                "reset_foreign_destructions_count",
            ],
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
            required_code_markers: &[
                "ShmBuffer",
                "ShmView",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/perceus/escape.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "verify_retained_escape",
                "verify_foreign_handle_dtor",
                "esc_fn_is_retained",
            ],
            required_code_markers: &[
                "ESCAPE004",
                "FFI001",
                "verify_retained_escape",
            ],
            forbidden_code_markers: &["@hard_realtime"],
            forbidden_string_substrings: &["dsp", "@hard_realtime"],
        },
        anticheat::FileAuditRule {
            path: "selfhost/perceus/arc.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "emit_foreign_drop_op",
            ],
            required_code_markers: &[
                "ForeignDrop",
                "emit_foreign_drop_op",
            ],
            forbidden_code_markers: &["@hard_realtime"],
            forbidden_string_substrings: &["dsp", "@hard_realtime"],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m14(deps: &[&str], script: &str) -> String {
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
    vm.run().to_string_repr()
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

    let script = r#"
        pub fn main() -> i64 {
            reset_foreign_destructions_count();

            // Null pointer produces invalid handle
            let null_h: ForeignHandle<i64> = ForeignHandle.wrap(0, 21845);
            if null_h.is_valid() { return 1; }
            if null_h.as_ptr() != 0 { return 2; }

            // Valid pointer wraps successfully
            var h1 = ForeignHandle.wrap(4096, 8192);
            if !h1.is_valid() { return 3; }
            if h1.as_ptr() != 4096 { return 4; }

            // take_raw transfers ownership back to C and disarms destructor
            let raw_p = h1.take_raw();
            if raw_p != 4096 { return 5; }
            if h1.is_valid() { return 6; }
            if h1.as_ptr() != 0 { return 7; }

            // Stress test allocating and dropping 100 ForeignHandles across branches
            var i: i64 = 0;
            while i < 100 {
                let dtor_addr = 32768 + i;
                let ptr_addr = 65536 + i * 8;
                var handle = ForeignHandle.wrap(ptr_addr, dtor_addr);
                if !handle.is_valid() { return 8; }

                // Branching logic: even numbers take_raw (retained by C), odd drop normally
                if (i & 1) == 0 {
                    let _ = handle.take_raw();
                } else {
                    // Drop naturally through ~self()
                    handle.~self();
                }
                i = i + 1;
            }

            // Exactly 50 odd handles dropped through destructor
            if live_foreign_destructions_count() != 50 { return 9; }

            return 42;
        }
    "#;

    let deps = &["stdlib/std/polyglot/lifetime.pilot"];
    let res = eval_m14(deps, script);
    assert_eq!(res, "42", "ForeignHandle lifecycle stress test failed, got: {}", res);

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
            reset_foreign_destructions_count();
            set_current_thread_id(1);

            // Handle created with owner_tid = 2 (belongs to worker thread 2)
            // Current thread is 1
            var h = ForeignHandle.wrap_thread_affine(43968, 57072, 2);
            if !h.is_valid() { return 1; }
            if h.owner_thread_id() != 2 { return 2; }

            // Drop handle on thread 1 (where owner_tid != current_tid)
            // Should be enqueued onto ThreadAffineDropQueue for thread 2
            h.~self();

            // Verify queued
            if ThreadAffineDropQueue.pending_drops_for(2) != 1 { return 3; }
            if ThreadAffineDropQueue.pending_drops_for(1) != 0 { return 4; }
            if live_foreign_destructions_count() != 0 { return 5; }

            // Drain thread 2
            let drained = ThreadAffineDropQueue.drain_thread(2);
            if drained != 1 { return 6; }
            if ThreadAffineDropQueue.pending_drops_for(2) != 0 { return 7; }
            if live_foreign_destructions_count() != 1 { return 8; }

            return 100;
        }
    "#;

    let deps = &["stdlib/std/polyglot/lifetime.pilot"];
    let res = eval_m14(deps, script);
    assert_eq!(res, "100", "Thread-affine drop queue test failed, got: {}", res);

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Compile-Time Escape Diagnostics (ESCAPE001, ESCAPE004, FFI001)
// -----------------------------------------------------------------------------
#[test]
fn test_m14_3_escape001_and_escape004_and_ffi001_diagnostics() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m14_3_escape001_and_escape004_and_ffi001_diagnostics",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            // 1. ESCAPE001: 0b01 borrowed view or 0b11 scoped guard crossing cue boundary
            let esc1 = verify_borrow_escape_by_lattice("slice_view", 1, true, -1, true);
            if esc1 != "ESCAPE001" { return 1; }

            let esc1_guard = verify_borrow_escape_by_lattice("guard_val", 3, true, -1, true);
            if esc1_guard != "ESCAPE001" { return 2; }

            // 2. ESCAPE004: Ephemeral 0b01 borrow passed to [Retained] async foreign parameter
            let esc4_borrow = verify_retained_escape(1, false);
            if esc4_borrow != "ESCAPE004" { return 3; }

            let esc4_guard = verify_retained_escape(3, false);
            if esc4_guard != "ESCAPE004" { return 4; }

            // Pinned or isolated objects pass safely without ESCAPE004
            let esc4_pinned = verify_retained_escape(1, true);
            if esc4_pinned != "" { return 5; }

            let esc4_owned = verify_retained_escape(2, false);
            if esc4_owned != "" { return 6; }

            // 3. FFI001: Null destructor on non-trivial foreign handle
            let ffi1_err = verify_foreign_handle_dtor(true, false);
            if ffi1_err != "FFI001" { return 7; }

            let ffi1_ok = verify_foreign_handle_dtor(false, false);
            if ffi1_ok != "" { return 8; }

            let ffi1_trivial = verify_foreign_handle_dtor(true, true);
            if ffi1_trivial != "" { return 9; }

            // 4. Verification of --explain explanations
            let exp_esc4 = get_diagnostic_explanation("ESCAPE004");
            if exp_esc4.code != "ESCAPE004" { return 10; }
            if !exp_esc4.title.contains("Retained") { return 11; }

            let exp_ffi1 = get_diagnostic_explanation("FFI001");
            if exp_ffi1.code != "FFI001" { return 12; }
            if !exp_ffi1.title.contains("Destructor") { return 13; }

            // 5. Emit foreign drop token
            let fdrop = emit_foreign_drop_op("native_res", "c_free");
            if fdrop != "ForeignDrop:native_res(c_free)" { return 14; }

            return 200;
        }
    "#;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/diagnostics/renderer.pilot",
        "selfhost/perceus/realtime.pilot",
        "selfhost/perceus/escape.pilot",
        "selfhost/perceus/arc.pilot",
    ];
    let res = eval_m14(deps, script);
    assert_eq!(res, "200", "Perceus foreign escape diagnostics test failed, got: {}", res);

    guard.finish();
}

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
            reset_foreign_destructions_count();

            // 1. PinnedSlice test
            var pinned: PinnedSlice<i64> = PinnedSlice.from_slice_buffer(327680, 16, 8);
            if pinned.len != 16 { return 1; }
            if pinned.byte_len() != 128 { return 2; }
            if pinned.as_ptr() != 327680 { return 3; }
            if !pinned.pinned() { return 4; }
            if pinned.is_empty() { return 5; }

            // 2. ForeignGuard test (0b11 scoped guard)
            var guard_obj: ForeignGuard<i64> = ForeignGuard.acquire(12345, 36864);
            if guard_obj.token() != 12345 { return 6; }
            if !guard_obj.active() { return 7; }

            // Drop runs release
            guard_obj.~self();
            if guard_obj.active() { return 8; }
            if live_foreign_destructions_count() != 1 { return 9; }

            // Disarmed guard does not invoke release on drop
            var guard_disarmed: ForeignGuard<i64> = ForeignGuard.acquire(67890, 36864);
            guard_disarmed.disarm();
            guard_disarmed.~self();
            if live_foreign_destructions_count() != 1 { return 10; }

            // 3. BorrowCallbackScope test (0b11)
            var bscope = BorrowCallbackScope.enter(999);
            if bscope.id() != 999 { return 11; }
            if !bscope.active() { return 12; }
            bscope.leave();
            if bscope.active() { return 13; }

            // 4. ShmBuffer & ShmView test (0b10 & 0b01)
            var shm: ShmBuffer<f64> = ShmBuffer.create(8192, 4096);
            if shm.capacity() != 8192 { return 14; }
            if !shm.is_active() { return 15; }

            let full_view = shm.as_view(8);
            if full_view.len != 1024 { return 16; }
            if full_view.byte_length() != 8192 { return 17; }
            if !full_view.is_valid() { return 18; }

            // Sub-view projection with valid bounds
            let sub_view = ShmView.project(ref shm, 10, 50, 8);
            if sub_view.len != 50 { return 19; }
            if sub_view.byte_length() != 400 { return 20; }
            if !sub_view.is_valid() { return 21; }

            // Out-of-bounds projection returns invalid view
            let bad_view = ShmView.project(ref shm, 1000, 100, 8);
            if bad_view.is_valid() { return 22; }

            // Release buffer invalidates views
            shm.release();
            if shm.is_active() { return 23; }

            return 300;
        }
    "#;

    let deps = &[
        "stdlib/std/polyglot/lifetime.pilot",
        "stdlib/std/polyglot/shm.pilot",
    ];
    let res = eval_m14(deps, script);
    assert_eq!(res, "300", "PinnedSlice, ForeignGuard, and ShmBuffer test failed, got: {}", res);

    guard.finish();
}
