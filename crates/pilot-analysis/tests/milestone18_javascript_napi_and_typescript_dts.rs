//! ============================================================================
//! Milestone 18 Verification Suite:
//! In-Process JS/TypeScript Adapter (`use js` / `use ts` via Node-API & Embedded QuickJS)
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

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
    vm.run().to_string_repr()
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
            let env: usize = 0x1000;
            let val_ptr: usize = 0x50000;

            // 1. Full buffer acquisition
            var lease = napi_acquire_typedarray(env, val_ptr, "Float64Array", 0, 1024);
            if !lease.is_active { return 1; }
            if lease.raw_ptr != 0x50000 { return 2; }
            if lease.byte_length != 1024 { return 3; }
            if lease.byte_offset != 0 { return 4; }
            if lease.element_type != "Float64Array" { return 5; }

            // 2. Subarray acquisition with byte_offset > 0
            var sub_lease = napi_acquire_typedarray(env, val_ptr, "Float64Array", 128, 512);
            if !sub_lease.is_active { return 6; }
            if sub_lease.raw_ptr != (0x50000 + 128) { return 7; }
            if sub_lease.byte_offset != 128 { return 8; }
            if sub_lease.byte_length != 512 { return 9; }

            // 3. Buffer release
            napi_release_typedarray(var lease);
            if lease.is_active { return 10; }

            // 4. Null safety
            var null_lease = napi_acquire_typedarray(0, 0, "Uint8Array", 0, 0);
            if null_lease.is_active { return 11; }
            if null_lease.raw_ptr != 0 { return 12; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/ts_dts.pilot",
        "stdlib/std/polyglot/js_napi.pilot",
    ];
    let res = eval_m18(deps, script);
    assert_eq!(res, "42", "Zero-copy TypedArray and subarray buffer operations must pass");

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
            let env: usize = 0x1000;
            let i64_val: i64 = 9223372036854775807; // i64.max

            // 1. BigInt creation
            let big_js = napi_create_bigint_int64(env, i64_val);
            if big_js.val_id == 0 { return 1; }
            if big_js.type_tag != 2 { return 2; } // BigInt tag == 2

            // 2. Round-trip recovery
            let recovered = napi_get_value_bigint_int64(env, ref big_js);
            if recovered != i64_val { return 3; }

            // 3. Precision protection: regular JS number (tag == 1) rejected for 64-bit int
            let num_js = JsValue { val_id: 0x4000, type_tag: 1 };
            let rej = napi_get_value_bigint_int64(env, ref num_js);
            if rej != 0 { return 4; }

            // 4. Null env check
            let null_js = napi_create_bigint_int64(0, i64_val);
            if null_js.val_id != 0 { return 5; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/ts_dts.pilot",
        "stdlib/std/polyglot/js_napi.pilot",
    ];
    let res = eval_m18(deps, script);
    assert_eq!(res, "42", "Lossless BigInt fidelity and precision protection must pass");

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

            // 1. Handle Scope Guard (0b11: IGuard)
            var scope = napi_open_handle_scope(env);
            if !scope.is_open { return 1; }
            if scope.scope_id != 0x1000 { return 2; }
            napi_close_handle_scope(var scope);
            if scope.is_open { return 3; }

            // 2. JsRef (0b10: IObject) Refcounting
            var r = js_ref_new(env, 0x3000);
            if r.ref_id != 0x2000 { return 4; }
            if r.refcount != 1 { return 5; }
            js_ref_inc(ref r);
            js_ref_dec(var r);
            if r.refcount != 0 { return 6; }

            // 3. Function Invocation
            var args: List<usize> = [10, 20];
            let call_res = napi_call_function(env, 0x4000, args.slice(0, 2));
            if call_res.status != 0 { return 7; }
            if call_res.val != 0x3000 { return 8; }

            // Null func call
            let err_call = napi_call_function(env, 0, args.slice(0, 0));
            if err_call.status == 0 { return 9; }
            if err_call.err.name != "TypeError" { return 10; }

            // 4. Promise Lifecycle
            var prom = js_promise_new(101);
            if prom.promise_id != 101 { return 11; }
            if prom.state != 0 { return 12; }
            js_promise_resolve(var prom);
            if prom.state != 1 { return 13; }

            // 5. N-API Wrapper Synthesizer
            var mod_decl = ts_module_decl_new("my_engine");
            let wrapper = ts_synthesize_napi_wrapper(ref mod_decl, "execute");
            if !wrapper.contains("napi_call_function") { return 14; }
            if !wrapper.contains("napi_open_handle_scope") { return 15; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/ts_dts.pilot",
        "stdlib/std/polyglot/js_napi.pilot",
    ];
    let res = eval_m18(deps, script);
    assert_eq!(res, "42", "Node-API handle scopes, promises, and wrapper synthesizer must pass");

    guard.finish();
}
