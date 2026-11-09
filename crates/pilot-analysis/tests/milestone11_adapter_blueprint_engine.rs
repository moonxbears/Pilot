//! ============================================================================
//! Milestone 11 Verification Suite:
//! First-Class `adapter` Blueprint Engine & Zero-Cost Idiom Lifter
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

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
    vm.run().to_string_repr()
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

    let script = r#"
        pub fn main() -> i64 {
            var bp = adapter_blueprint_new("libfoo", "FooHandle");
            bp.strip_prefixes.push("foo_");
            bp.slice_rules.push(slice_pair_rule_new(1, 2, true, false));
            bp.out_rules.push(out_param_rule_new(3, 4, true));

            var raw_syms: List<str> = [
                "foo_get_timeout",
                "foo_set_timeout",
                "foo_read_buffer",
                "foo_write_buffer",
                "foo_connect",
                "foo_disconnect",
                "foo_flush",
                "foo_poll",
                "foo_reset",
                "foo_status",
                "foo_bind",
                "foo_close",
            ];

            let report = verify_and_compile_blueprint(ref bp, raw_syms);
            if report.diagnostics.len() != 0 { return 1; }
            if report.methods.len() != 12 { return 2; }
            if report.properties_count < 2 { return 3; }
            if report.total_ir_ops < 70 { return 4; }

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
    assert_eq!(res, "42", "12-function C mock API adaptation must synthesize clean PilIR thunks with 0 diagnostics");

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

    // Verify ADP001 Diagnostic emission on empty target type
    let script = r#"
        pub fn main() -> i64 {
            var bp = adapter_blueprint_new("libfoo", "");
            var raw_syms: List<str> = ["foo_work"];
            let report = verify_and_compile_blueprint(ref bp, raw_syms);
            if report.diagnostics.len() == 0 { return 1; }
            let diag = report.diagnostics[0];
            if diag.code != "ADP001" { return 2; }
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
    assert_eq!(res, "42", "Adapter must emit ADP001 diagnostic when target type is empty");

    guard.finish();
}
