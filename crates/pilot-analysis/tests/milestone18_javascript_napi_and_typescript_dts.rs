//! ============================================================================
//! Milestone 18 Verification Suite:
//! In-Process JS/TypeScript Adapter (`use js` / `use ts` via Node-API & Embedded QuickJS)
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
    milestone_id: 18,
    test_file_rel: "crates/pilot-analysis/tests/milestone18_javascript_napi_and_typescript_dts.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/adapter/ts_dts.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "ts_parse_dts",
                "ts_classify_type",
                "ts_diag_code",
                "ts_synthesize_napi_wrapper",
                "ts_module_decl_new",
            ],
            required_code_markers: &[
                "TsItemKind",
                "TsTypeKind",
                "TsParamDecl",
                "TsMethodDecl",
                "TsInterfaceDecl",
                "TsClassDecl",
                "TsModuleDecl",
                "JS001",
                "JS003",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/std/polyglot/js_napi.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "napi_open_handle_scope",
                "napi_close_handle_scope",
                "js_ref_new",
                "js_ref_inc",
                "js_ref_dec",
                "napi_acquire_typedarray",
                "napi_release_typedarray",
                "napi_create_bigint_int64",
                "napi_get_value_bigint_int64",
                "napi_call_function",
                "js_err_fetch",
            ],
            required_code_markers: &[
                "NapiScopeGuard",
                "JsRef",
                "JsError",
                "JsBufferLease",
                "JsValue",
                "NapiResult",
                "JsPromise",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m18(deps: &[&str], script: &str) -> String {
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
    let bin = BytecodeCompiler::compile(&arena, arena.root);
    vm.load_binary(bin);
    vm_outcome::outcome(&mut vm)
}

// -----------------------------------------------------------------------------
// Test 1: TypeScript .d.ts Declaration Parser (`ts_dts.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m18_1_typescript_dts_parser_and_inspect_signatures() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m18_1_typescript_dts_parser_and_inspect_signatures",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            let dts_src = "export interface Config extends BaseConfig {\n"
                + "    readonly timeoutMs: number;\n"
                + "    retries?: number;\n"
                + "    validate(strict: boolean): boolean;\n"
                + "}\n"
                + "export class TensorEngine implements IEngine {\n"
                + "    weights: Float64Array;\n"
                + "    predict(batch: Float64Array, count?: number, ...rest: any[]): Promise<number>;\n"
                + "}\n"
                + "export function computeHash(data: Uint8Array): bigint;\n"
                + "const VERSION: string;\n";

            var diags: List<Diagnostic> = [];
            let module = ts_parse_dts(dts_src, var diags);

            // 1. Verify Interface
            if module.interfaces.len() != 1 { return 1; }
            let iface = module.interfaces[0];
            if iface.name != "Config" { return 2; }
            if iface.extends_names.len() != 1 { return 3; }
            if iface.extends_names[0] != "BaseConfig" { return 4; }
            if iface.properties.len() != 2 { return 5; }
            if !iface.properties[0].is_readonly { return 6; }
            if iface.properties[0].name != "timeoutMs" { return 7; }
            if !iface.properties[1].is_optional { return 8; }
            if iface.methods.len() != 1 { return 9; }
            if iface.methods[0].name != "validate" { return 10; }

            // 2. Verify Class
            if module.classes.len() != 1 { return 11; }
            let cls = module.classes[0];
            if cls.name != "TensorEngine" { return 12; }
            if cls.implements_names.len() != 1 { return 13; }
            if cls.implements_names[0] != "IEngine" { return 14; }
            if cls.properties.len() != 1 { return 15; }
            if cls.properties[0].type_kind != TsTypeKind::TypedArray { return 16; }
            if cls.methods.len() != 1 { return 17; }
            let meth = cls.methods[0];
            if meth.name != "predict" { return 18; }
            if meth.params.len() != 3 { return 19; }
            if !meth.params[1].is_optional { return 20; }
            if !meth.params[2].is_rest { return 21; }
            if meth.ret_kind != TsTypeKind::Promise { return 22; }

            // 3. Verify Function & Variable
            if module.functions.len() != 1 { return 23; }
            if module.functions[0].name != "computeHash" { return 24; }
            if module.functions[0].ret_kind != TsTypeKind::BigInt { return 25; }
            if module.variables.len() != 1 { return 26; }
            if module.variables[0].name != "VERSION" { return 27; }
            if !module.variables[0].is_readonly { return 28; }

            // 4. Type classification
            if ts_classify_type("void") != TsTypeKind::Void { return 29; }
            if ts_classify_type("number") != TsTypeKind::Number { return 30; }
            if ts_classify_type("bigint") != TsTypeKind::BigInt { return 31; }
            if ts_classify_type("boolean") != TsTypeKind::Boolean { return 32; }
            if ts_classify_type("string") != TsTypeKind::String { return 33; }
            if ts_classify_type("Float64Array") != TsTypeKind::TypedArray { return 34; }
            if ts_classify_type("number[]") != TsTypeKind::Array { return 35; }
            if ts_classify_type("Promise<string>") != TsTypeKind::Promise { return 36; }
            if ts_classify_type("string | undefined") != TsTypeKind::OptUnion { return 37; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/ts_dts.pilot",
        "stdlib/std/polyglot/js_napi.pilot",
    ];
    let res = eval_m18(deps, script);
    assert_eq!(res, "42", "TypeScript .d.ts parser and inspect signatures must pass");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Zero-Copy TypedArray & Subarray Buffers (`js_napi.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m18_2_zero_copy_typedarray_and_subarray_buffers() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m18_2_zero_copy_typedarray_and_subarray_buffers",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // 1. TypedArray kinds absorbed from .d.ts map to borrowed views
            if ts_classify_type("Uint8Array") != TsTypeKind::TypedArray { return 1; }
            if ts_classify_type("Int32Array") != TsTypeKind::TypedArray { return 2; }
            if ts_classify_type("Float64Array") != TsTypeKind::TypedArray { return 3; }
            if ts_diag_code(TsDiagnosticKind::DetachedBufferError) != "JS004" { return 4; }

            // 2. Borrowing the memory needs napi_get_typedarray_info from a loaded host.
            var lease = napi_acquire_typedarray(0x1000, 0x50000);
            return 43;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/ts_dts.pilot",
        "stdlib/std/polyglot/js_napi.pilot",
    ];
    let res = eval_m18(deps, script);
    assert!(
        res.starts_with("FAULT: SEAM001: unconnected seam `js.typedarray_info`"),
        "TypedArray classification must pass, then borrowing must stop at its seam, got {res}"
    );

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Lossless 64-Bit BigInt Fidelity (`js_napi.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m18_3_lossless_bigint_fidelity_and_precision_protection() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m18_3_lossless_bigint_fidelity_and_precision_protection",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // 1. bigint declarations map to 64-bit integers; JS003 guards precision loss
            if ts_classify_type("bigint") != TsTypeKind::BigInt { return 1; }
            if ts_classify_type("number") != TsTypeKind::Number { return 2; }
            if ts_diag_code(TsDiagnosticKind::PrecisionLossError) != "JS003" { return 3; }

            // 2. Creating a BigInt needs napi_create_bigint_int64 from a loaded host.
            let i64_val: i64 = 9223372036854775807;
            let big_js = napi_create_bigint_int64(0x1000, i64_val);
            return 43;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/ts_dts.pilot",
        "stdlib/std/polyglot/js_napi.pilot",
    ];
    let res = eval_m18(deps, script);
    assert!(
        res.starts_with("FAULT: SEAM001: unconnected seam `js.bigint_create`"),
        "bigint classification must pass, then BigInt creation must stop at its seam, got {res}"
    );

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Node-API Handle Scope, Promises & Synthesizer (`ts_dts.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m18_4_handle_scopes_promises_and_napi_synthesizer() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m18_4_handle_scopes_promises_and_napi_synthesizer",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            let env: usize = 0x1000;

            // 1. A null function is refused with napi_invalid_arg before reaching JS
            var args: List<usize> = [10, 20];
            let err_call = napi_call_function(env, 0, args.slice(0, 0));
            if err_call.status != NAPI_INVALID_ARG { return 9; }
            if err_call.err.name != "TypeError" { return 10; }

            // 2. N-API Wrapper Synthesizer
            var mod_decl = ts_module_decl_new("my_engine");
            let wrapper = ts_synthesize_napi_wrapper(ref mod_decl, "execute");
            if !wrapper.contains("my_engine.execute") { return 13; }
            if !wrapper.contains("napi_call_function") { return 14; }
            if !wrapper.contains("napi_open_handle_scope") { return 15; }

            // 3. Opening a handle scope needs napi_open_handle_scope from a loaded host.
            var scope = napi_open_handle_scope(env);
            return 43;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/ts_dts.pilot",
        "stdlib/std/polyglot/js_napi.pilot",
    ];
    let res = eval_m18(deps, script);
    assert!(
        res.starts_with("FAULT: SEAM001: unconnected seam `js.open_handle_scope`"),
        "Argument check and wrapper synthesizer must pass, then handle scopes must stop at their seam, got {res}"
    );

    // Every other Node-API entry point stops at its own seam.
    let calls: &[(&str, &str)] = &[
        ("js_err_fetch()", "js.err_fetch"),
        ("napi_close_handle_scope(var scope)", "js.close_handle_scope"),
        ("js_ref_new(0x1000, 0x2000)", "js.ref_new"),
        ("js_ref_inc(ref r)", "js.ref_inc"),
        ("js_ref_dec(var r)", "js.ref_dec"),
        ("napi_release_typedarray(var lease)", "js.typedarray_release"),
        ("napi_get_value_bigint_int64(0x1000, ref v)", "js.bigint_read"),
        ("napi_call_function(0x1000, 0x3000, args.slice(0, 1))", "js.call_function"),
        ("js_promise_new(0x1000)", "js.promise_new"),
        ("js_promise_resolve(var prom, 0x4000)", "js.promise_resolve"),
        ("js_promise_reject(var prom, 0x4000)", "js.promise_reject"),
    ];
    for (call, id) in calls {
        let script = format!(
            r##"
            pub fn main() -> i64 {{
                var scope = NapiScopeGuard {{ scope_id: 0x10, is_open: true }};
                var r = JsRef {{ ref_id: 0x20, env_ptr: 0x1000 }};
                var lease = JsBufferLease {{ raw_ptr: 0x30, byte_length: 8, byte_offset: 0, is_active: true, element_type: "Uint8Array" }};
                let v = JsValue {{ val_id: 0x40, type_tag: 9 }};
                var prom = JsPromise {{ promise_id: 0x50, deferred: 0x60 }};
                var args: List<usize> = [1];
                {call};
                return 43;
            }}
        "##
        );
        let res = eval_m18(deps, &script);
        assert!(
            res.starts_with(&format!("FAULT: SEAM001: unconnected seam `{id}`")),
            "{call} must stop at seam {id}, got {res}"
        );
    }

    guard.finish();
}
