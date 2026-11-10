//! ============================================================================
//! Milestone 11 Verification Suite:
//! First-Class `adapter` Blueprint Engine & Zero-Cost Idiom Lifter
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
    milestone_id: 11,
    test_file_rel: "crates/pilot-analysis/tests/milestone11_adapter_blueprint_engine.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/adapter/rules.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "slice_pair_rule_new",
                "out_param_rule_new",
                "callback_pair_rule_new",
                "adapter_blueprint_new",
                "adapter_strip_prefix",
                "adapter_case_norm",
            ],
            required_code_markers: &[
                "NamingStyle",
                "ErrorLiftPolicy",
                "SlicePairRule",
                "OutParamRule",
                "CallbackPairRule",
                "AdapterBlueprint",
                "AdapterTransformEntry",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/adapter/compiler.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "compile_adapter_method_thunk",
                "verify_and_compile_blueprint",
            ],
            required_code_markers: &[
                "AdaptedParam",
                "AdaptedMethod",
                "AdapterCompilationReport",
                "compile_adapter_method_thunk",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/std/polyglot/adapter.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "c_error_new",
                "errno_new",
                "null_ptr_err_new",
                "adapt_attr_new",
            ],
            required_code_markers: &[
                "CError",
                "Errno",
                "NullPtrErr",
                "AdaptAttr",
                "PosixAdapterFacet",
                "Win32AdapterFacet",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m11(deps: &[&str], script: &str) -> String {
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
// Test 1: Adapter Rule Registry & Prefix/Case Transformations (`rules.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m11_1_adapter_rule_registry_and_prefix_case_transformations() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m11_1_adapter_rule_registry_and_prefix_case_transformations",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            let n_rules = adapter_rule_count();
            if n_rules != 9 { return 1; }

            let r1 = adapter_rule_get(0);
            if r1.name != "strip_prefix" { return 2; }

            // Test prefix stripping
            var prefixes: List<str> = ["libuv_", "uv_"];
            let s1 = adapter_strip_prefix("uv_tcp_connect", prefixes);
            if s1 != "tcp_connect" { return 3; }

            let s2 = adapter_strip_prefix("libuv_run", prefixes);
            if s2 != "run" { return 4; }

            // Test case normalization with style constants
            let c1 = adapter_case_norm("tcp_connect", naming_style_camel_case());
            if c1 != "tcpConnect" { return 5; }

            let c2 = adapter_case_norm("tcp_connect", naming_style_pascal_case());
            if c2 != "TcpConnect" { return 6; }

            let c3 = adapter_case_norm("tcp_connect", naming_style_preserve());
            if c3 != "tcp_connect" { return 7; }

            return 42;
        }
    "#;

    let res = eval_m11(&["selfhost/adapter/rules.pilot"], script);
    assert_eq!(res, "42", "Adapter rule registry & string transforms must execute correctly");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Zero-Allocation PilIR Thunk Lowering (`compiler.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m11_2_zero_allocation_pilir_thunk_lowering() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m11_2_zero_allocation_pilir_thunk_lowering",
        &ANTICHEAT_SPEC,
    );

    // A 12-function C API: handle receivers, a `(const uint8_t*, size_t)` pair
    // in slots 1-2, an `int*` out-parameter in slot 3 and zero-is-ok status
    // codes. Expected op counts are worked by hand from spec 11.4:
    // receiver 1 op, slice pair 2, out-param stack_slot + store (zero-init)
    // + load on the ok path 3, call_direct 1, cmp + cond_br 2, return 1.
    // Nine plain methods give 5 ops, read/write_buffer 7, poll 10 -> 69.
    let script = r#"
        fn mk(sym: str, params: List<str>) -> AdapterFnSig => adapter_fn_sig_new(sym, params, "int");

        pub fn main() -> i64 {
            var bp = adapter_blueprint_new("libfoo", "FooHandle");
            bp.strip_prefixes.push("foo_");
            bp.slice_rules.push(slice_pair_rule_new(1, 2, true, false));
            bp.out_rules.push(out_param_rule_new(3, 4, true));

            var sigs: List<AdapterFnSig> = [
                mk("foo_get_timeout", ["foo_t*"]),
                mk("foo_set_timeout", ["foo_t*", "int"]),
                mk("foo_read_buffer", ["foo_t*", "uint8_t*", "size_t"]),
                mk("foo_write_buffer", ["foo_t*", "const uint8_t *", "size_t"]),
                mk("foo_connect", ["foo_t*", "int"]),
                mk("foo_disconnect", ["foo_t*"]),
                mk("foo_flush", ["foo_t*"]),
                mk("foo_poll", ["foo_t*", "const uint8_t*", "size_t", "int*"]),
                mk("foo_reset", ["foo_t*"]),
                mk("foo_status", ["foo_t*"]),
                mk("foo_bind", ["foo_t*", "int"]),
                mk("foo_close", ["foo_t*"]),
            ];

            let report = verify_and_compile_blueprint(ref bp, sigs);
            if report.diagnostics.len() != 0 { return 1; }
            if report.methods.len() != 12 { return 2; }
            if report.properties_count != 2 { return 3; }
            if report.total_ir_ops != 69 { return 4; }

            let read = report.methods[2];
            if read.clean_name != "read_buffer" { return 5; }
            if read.params.len() != 2 { return 6; }
            if !read.params[0].is_receiver { return 7; }
            if read.params[1].pilot_type != "slice<u8>" { return 8; }
            if read.return_type != "void ! int" { return 9; }

            // poll: receiver, slice pair, out-param slot, call, branch, load, return.
            let poll = report.methods[7];
            let want: List<u16> = [
                op_extract_field(), op_extract_field(), op_extract_field(),
                op_stack_slot(), op_store(), op_call_direct(), op_cmp(),
                op_cond_br(), op_load(), op_return(),
            ];
            if poll.ir_ops.len() != want.len() { return 10; }
            var i: usize = 0;
            while i < want.len() {
                if poll.ir_ops[i] != want[i] { return 11; }
                i = i + 1;
            }
            if poll.params.len() != 2 { return 12; }

            let get = report.methods[0];
            if !get.is_property_get || get.property_name != "timeout" { return 13; }
            return 42;
        }
    "#;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "stdlib/core/reflect.pilot",
        "selfhost/ir/model.pilot",
        "selfhost/adapter/rules.pilot",
        "selfhost/adapter/compiler.pilot",
    ];
    let res = eval_m11(deps, script);
    assert_eq!(res, "42", "12-function C API adaptation must plan the PilIR thunks of spec 11.4 with 0 diagnostics");

    // Installing the planned thunks needs the PilIR function model (F5).
    let install = r#"
        pub fn main() -> i64 {
            var bp = adapter_blueprint_new("libfoo", "Foo");
            var sigs: List<AdapterFnSig> = [adapter_fn_sig_new("foo_close", ["foo_t*"], "int")];
            let report = verify_and_compile_blueprint(ref bp, sigs);
            let n = install_adapter_thunks(ref report);
            return 42;
        }
    "#;
    let res = eval_m11(deps, install);
    assert!(
        res.starts_with("FAULT: SEAM001: unconnected seam `adapter.install_thunks`"),
        "installing adapter thunks must stop at its seam, got {res}"
    );

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: User-Facing [Adapt] Attributes and Error Facets (`adapter.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m11_3_user_facing_adapt_attributes_and_error_facets() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m11_3_user_facing_adapt_attributes_and_error_facets",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            let err = c_error_new(-1, "Operation failed");
            if err.code != -1 { return 1; }
            if err.message != "Operation failed" { return 2; }

            let e = errno_new(11);
            if !errno_is_eagain(e) { return 3; }

            let null_err = null_ptr_err_new("fopen");
            if null_err.symbol != "fopen" { return 4; }

            let attr = adapt_attr_new("sqlite3", "sqlite3_", 1);
            if attr.source_module != "sqlite3" { return 5; }
            if attr.strip_prefix != "sqlite3_" { return 6; }
            if attr.error_policy != 1 { return 7; }

            return 42;
        }
    "#;

    let res = eval_m11(&["stdlib/std/polyglot/adapter.pilot"], script);
    assert_eq!(res, "42", "std.polyglot.adapter error types and attribute constructors must operate accurately");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: pilot-ffi Mock Deletion and Adapter Diagnostics (`ADP001`..`ADP004`)
// -----------------------------------------------------------------------------
#[test]
fn test_m11_4_pilot_ffi_mock_deletion_and_adapter_diagnostics() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m11_4_pilot_ffi_mock_deletion_and_adapter_diagnostics",
        &ANTICHEAT_SPEC,
    );

    // Verify complete deletion of fake mocks from crates/pilot-ffi/src/lib.rs
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let ffi_src = fs::read_to_string(root.join("crates/pilot-ffi/src/lib.rs")).unwrap();
    assert!(!ffi_src.contains("pilot_transpile_sample"), "pilot_transpile_sample mock must be permanently deleted!");
    assert!(!ffi_src.contains("pilot_polyglot_execute"), "pilot_polyglot_execute mock must be permanently deleted!");

    // ADP001..ADP004, each from the shape of a real signature.
    let script = r#"
        fn first_code(bp: ref<AdapterBlueprint>, sigs: List<AdapterFnSig>) -> str {
            let report = verify_and_compile_blueprint(bp, sigs);
            if report.diagnostics.len() == 0 { return "none"; }
            return report.diagnostics[0].code;
        }

        pub fn main() -> i64 {
            var empty = adapter_blueprint_new("libfoo", "");
            var s1: List<AdapterFnSig> = [adapter_fn_sig_new("foo_work", ["foo_t*"], "int")];
            if first_code(ref empty, s1) != "ADP001" { return 1; }

            var bp = adapter_blueprint_new("libfoo", "Foo");
            bp.strip_prefixes.push("foo_");
            bp.strip_prefixes.push("bar_");
            var s2: List<AdapterFnSig> = [
                adapter_fn_sig_new("foo_close", ["foo_t*"], "int"),
                adapter_fn_sig_new("bar_close", ["foo_t*"], "int"),
            ];
            if first_code(ref bp, s2) != "ADP002" { return 2; }

            var cb = adapter_blueprint_new("libfoo", "Foo");
            cb.callbacks.push(callback_pair_rule_new(1, 2));
            var s3: List<AdapterFnSig> = [
                adapter_fn_sig_new("foo_on_event", ["foo_t*", "void (*)(int, void*)", "int"], "int"),
            ];
            if first_code(ref cb, s3) != "ADP003" { return 3; }
            var s3ok: List<AdapterFnSig> = [
                adapter_fn_sig_new("foo_on_event", ["foo_t*", "void (*)(int, void*)", "void *"], "int"),
            ];
            if first_code(ref cb, s3ok) != "none" { return 4; }

            var sp = adapter_blueprint_new("libfoo", "Foo");
            sp.slice_rules.push(slice_pair_rule_new(1, 2, true, false));
            var s4: List<AdapterFnSig> = [
                adapter_fn_sig_new("foo_read", ["foo_t*", "uint8_t*", "double"], "int"),
            ];
            if first_code(ref sp, s4) != "ADP004" { return 5; }
            return 42;
        }
    "#;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "stdlib/core/reflect.pilot",
        "selfhost/ir/model.pilot",
        "selfhost/adapter/rules.pilot",
        "selfhost/adapter/compiler.pilot",
    ];
    let res = eval_m11(deps, script);
    assert_eq!(res, "42", "Adapter must emit ADP001..ADP004 from the signatures it is given");

    guard.finish();
}

