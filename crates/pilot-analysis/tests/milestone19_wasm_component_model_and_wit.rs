//! ============================================================================
//! Milestone 19 Verification Suite:
//! WebAssembly Component Model & WIT Universal Adapter (`use wasm`)
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 19,
    test_file_rel: "crates/pilot-analysis/tests/milestone19_wasm_component_model_and_wit.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/adapter/wit_parser.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "wit_parse",
                "wit_decode_custom_section",
                "wit_classify_lattice",
                "wit_diag_code",
                "wit_package_new",
            ],
            required_code_markers: &[
                "WitTypeKind",
                "WitTypeRef",
                "WitInterface",
                "WitWorld",
                "WitPackage",
                "WitFunction",
                "WitParam",
                "WASM001",
                "WASM002",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/adapter/wit_abi.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "cabi_flatten_type",
                "cabi_flatten_params",
                "cabi_requires_spill",
                "cabi_requires_result_pointer",
                "cabi_lower_signature",
                "wasm_resource_table_new",
                "wasm_resource_allocate",
                "wasm_resource_borrow",
                "wasm_resource_release_borrow",
                "wasm_resource_drop",
            ],
            required_code_markers: &[
                "CabiLoweredSignature",
                "WasmResourceTable",
                "WasmResourceEntry",
                "WASM005",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/adapter/wasm_runtime.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "wasm_memory_new",
                "wasm_memory_grow",
                "wasm_memory_check_bounds",
                "wasm_memory_store_byte",
                "wasm_memory_load_byte",
                "wasm_memory_store_i32",
                "wasm_memory_load_i32",
                "wasm_memory_store_str",
                "wasm_memory_load_str",
                "wasm_lease_new",
                "wasm_lease_consume_fuel",
                "wasm_instance_new",
                "wasm_validate_header",
                "wasm_invoke",
            ],
            required_code_markers: &[
                "WasmMemory",
                "WasmInstanceLease",
                "WasmInstance",
                "WasmExportDef",
                "WASM001",
                "WASM003",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/std/polyglot/wasm.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "wasm_trap_code",
                "wasm_trap_message",
                "wasm_slice_len",
                "wasm_sandbox_is_open",
            ],
            required_code_markers: &[
                "WasmTrap",
                "WasmMemorySlice",
                "WasmResourceHandle",
                "WasmSandboxLease",
                "WASM001_MAGIC_ERR",
                "WASM003_BOUNDS_ERR",
                "WASM005_BORROW_ERR",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m19(deps: &[&str], script: &str) -> String {
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
// Test 1: WIT Parser & 2-Bit Hardware Lattice Mapping
// -----------------------------------------------------------------------------
#[test]
fn test_m19_1_wit_parser_and_2bit_lattice_mapping() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m19_1_wit_parser_and_2bit_lattice_mapping",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            let wit_src = "package pilot:test@0.1.0;\n"
                + "interface engine {\n"
                + "    record Config {\n"
                + "        timeout: u32,\n"
                + "        retries: u32,\n"
                + "    }\n"
                + "    resource Session {\n"
                + "        constructor(name: string);\n"
                + "        ping: func(token: u64) -> u64;\n"
                + "    }\n"
                + "    open-session: func(cfg: Config) -> own<Session>;\n"
                + "    query-session: func(s: borrow<Session>) -> string;\n"
                + "}\n"
                + "world test-world {\n"
                + "    import engine;\n"
                + "    export open-session: func(cfg: Config) -> own<Session>;\n"
                + "}\n";

            let pkg = wit_parse(wit_src);
            if pkg.namespace != "pilot" { return 1; }
            if pkg.name != "test" { return 2; }
            if pkg.interfaces.len() != 1 { return 3; }

            let iface = pkg.interfaces.get(0);
            if iface.name != "engine" { return 4; }
            if iface.records.len() != 1 { return 5; }
            if iface.resources.len() != 1 { return 6; }
            if iface.functions.len() != 2 { return 7; }

            // 2-bit lattice classification:
            // 0b00: IValue (record / primitive)
            let val_ty = wit_type_primitive("u32");
            if wit_classify_lattice(val_ty) != 0 { return 8; }

            // 0b01: IReference (borrow<R> / view)
            let ref_ty = wit_type_borrow("Session");
            if wit_classify_lattice(ref_ty) != 1 { return 9; }

            // 0b10: IObject (own<R> / handle with dtor)
            let obj_ty = wit_type_own("Session");
            if wit_classify_lattice(obj_ty) != 2 { return 10; }

            // Diagnostic codes
            let diag1 = wit_diag_code(1);
            if !diag1.starts_with("WASM001") { return 11; }
            let diag5 = wit_diag_code(5);
            if !diag5.starts_with("WASM005") { return 12; }

            // Binary custom section decoder
            let cs_pkg = wit_decode_custom_section("component-type", wit_src);
            if cs_pkg.name != "test" { return 13; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/adapter/wit_parser.pilot",
        "selfhost/adapter/wit_abi.pilot",
        "selfhost/adapter/wasm_runtime.pilot",
        "stdlib/std/polyglot/wasm.pilot",
    ];
    let res = eval_m19(deps, script);
    assert_eq!(res, "42", "WIT parser & 2-bit lattice mapping must succeed");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Canonical ABI Flattening, Spills & Multi-Result Out-Pointers
// -----------------------------------------------------------------------------
#[test]
fn test_m19_2_canonical_abi_flattening_and_spill_rules() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m19_2_canonical_abi_flattening_and_spill_rules",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // Check Canonical ABI limits
            if cabi_max_flat_params() != 16 { return 1; }
            if cabi_max_flat_results() != 1 { return 2; }

            // Case A: 3 params (scalar, string = ptr + len) -> 3 flat params (<= 16, no spill)
            var fn_a = wit_function_new("small_call");
            fn_a.params.push(WitParam { name: "x", type_ref: wit_type_primitive("u32") });
            fn_a.params.push(WitParam { name: "s", type_ref: wit_type_primitive("string") });
            fn_a.results.push(WitParam { name: "ret", type_ref: wit_type_primitive("s64") });

            let sig_a = cabi_lower_signature(fn_a);
            if sig_a.uses_param_spill { return 3; }
            if sig_a.uses_result_ptr { return 4; }
            if sig_a.core_params.len() != 3 { return 5; } // 1 + 2 = 3
            if sig_a.core_results.len() != 1 { return 6; }

            // Case B: 17 scalar params (> 16 MAX_FLAT_PARAMS) -> triggers indirect memory spill
            var fn_b = wit_function_new("wide_call");
            var i = 0;
            while i < 17 {
                fn_b.params.push(WitParam { name: "p" + i.to_str(), type_ref: wit_type_primitive("u32") });
                i = i + 1;
            }
            fn_b.results.push(WitParam { name: "ret", type_ref: wit_type_primitive("u32") });

            let sig_b = cabi_lower_signature(fn_b);
            if !sig_b.uses_param_spill { return 7; } // Must trigger spill!
            if sig_b.core_params.len() != 1 { return 8; } // Replaced by single indirect pointer!
            if sig_b.uses_result_ptr { return 9; }

            // Case C: Multi-result function (2 results > 1 MAX_FLAT_RESULTS) -> triggers result pointer
            var fn_c = wit_function_new("multi_ret");
            fn_c.params.push(WitParam { name: "in", type_ref: wit_type_primitive("u32") });
            fn_c.results.push(WitParam { name: "r1", type_ref: wit_type_primitive("u32") });
            fn_c.results.push(WitParam { name: "r2", type_ref: wit_type_primitive("u32") });

            let sig_c = cabi_lower_signature(fn_c);
            if !sig_c.uses_result_ptr { return 10; } // Must trigger result out-pointer!
            if sig_c.core_results.len() != 0 { return 11; } // Returned via pointer, not core results
            if sig_c.core_params.len() != 2 { return 12; } // (in, result_ptr)

            // Allocation plan test for cabi_realloc
            let plan = cabi_plan_alloc(128, 8, 1024);
            if plan.ptr != 1024 { return 13; }
            if plan.size != 128 { return 14; }

            // cabi_post_* function naming
            let post_hook = cabi_post_call_name("query");
            if post_hook != "cabi_post_query" { return 15; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/adapter/wit_parser.pilot",
        "selfhost/adapter/wit_abi.pilot",
        "selfhost/adapter/wasm_runtime.pilot",
        "stdlib/std/polyglot/wasm.pilot",
    ];
    let res = eval_m19(deps, script);
    assert_eq!(res, "42", "Canonical ABI flattening, spills & out-pointers must pass");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Sandboxed WASM Runtime, Bounds Traps & Fuel Exhaustion
// -----------------------------------------------------------------------------
#[test]
fn test_m19_3_sandboxed_wasm_runtime_bounds_and_fuel_traps() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m19_3_sandboxed_wasm_runtime_bounds_and_fuel_traps",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // 1. WASM Header Validation
            let valid_hdr = wasm_validate_header(wasm_magic_bytes(), wasm_supported_version());
            if valid_hdr.is_err() { return 1; }

            let bad_magic = wasm_validate_header(12345, 1);
            if !bad_magic.is_err() { return 2; }
            if !bad_magic.unwrap_err().contains("WASM001") { return 3; }

            // 2. Sandboxed Linear Memory (64KiB per page)
            var mem = wasm_memory_new(1, 4); // 1 page init, 4 pages max
            if wasm_memory_byte_len(mem) != 65536 { return 4; }

            // Store and load string in linear memory
            let st_res = wasm_memory_store_str(var mem, 100, "hello wasm component");
            if st_res.is_err() { return 5; }
            let ld_res = wasm_memory_load_str(mem, 100, 20);
            if ld_res.is_err() { return 6; }
            if ld_res.unwrap() != "hello wasm component" { return 7; }

            // Bounds check trap: load outside allocated memory -> WASM003
            let oob_res = wasm_memory_load_byte(mem, 70000);
            if !oob_res.is_err() { return 8; }
            if !oob_res.unwrap_err().contains("WASM003") { return 9; }

            // Memory growth: grow by 1 page (to 2 pages)
            let grow_res = wasm_memory_grow(var mem, 1);
            if grow_res.is_err() { return 10; }
            if wasm_memory_byte_len(mem) != 131072 { return 11; }

            // Growth beyond max_pages (max is 4) -> traps with WASM003
            let over_grow = wasm_memory_grow(var mem, 10);
            if !over_grow.is_err() { return 12; }
            if !over_grow.unwrap_err().contains("WASM003") { return 13; }

            // 3. Sandboxed Instance Execution & Fuel Limit Trap
            var inst = wasm_instance_new("plugin", 1, 4, 1000); // 1000 fuel limit
            wasm_instance_add_export(var inst, "add", 2, 1);
            wasm_instance_add_export(var inst, "infinite_loop_test", 0, 1);

            // Valid invocation: add(15, 27) = 42
            var args: List<i64> = [15, 27];
            let call_res = wasm_invoke(var inst, "add", args);
            if call_res.is_err() { return 14; }
            if call_res.unwrap() != 42 { return 15; }

            // Infinite loop trap: fuel exhausted cleanly without crashing the host!
            var no_args: List<i64> = [];
            let loop_res = wasm_invoke(var inst, "infinite_loop_test", no_args);
            if !loop_res.is_err() { return 16; }
            if !loop_res.unwrap_err().contains("Fuel limit exhausted") { return 17; }
            if !loop_res.unwrap_err().contains("WASM003") { return 18; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/adapter/wit_parser.pilot",
        "selfhost/adapter/wit_abi.pilot",
        "selfhost/adapter/wasm_runtime.pilot",
        "stdlib/std/polyglot/wasm.pilot",
    ];
    let res = eval_m19(deps, script);
    assert_eq!(res, "42", "Sandboxed WASM runtime, bounds traps & fuel exhaustion must pass");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Resource Handle Table, Borrows & Component Lease Lifecycle
// -----------------------------------------------------------------------------
#[test]
fn test_m19_4_component_resource_table_and_lease_lifecycle() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m19_4_component_resource_table_and_lease_lifecycle",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // 1. WASM Resource Table (own<R> & borrow<R>)
            var tbl = wasm_resource_table_new();
            let h1 = wasm_resource_allocate(var tbl, "DatabaseSession", 0x7000);
            if h1 != 1 { return 1; }
            if wasm_resource_live_count(tbl) != 1 { return 2; }

            // 2. Borrow handle: increments borrow count
            let b1 = wasm_resource_borrow(var tbl, h1);
            if b1.is_err() { return 3; }
            if b1.unwrap() != 0x7000 { return 4; }
            if wasm_resource_active_borrow_count(tbl) != 1 { return 5; }

            // 3. Drop violation: attempting to drop while borrowed triggers WASM005!
            let bad_drop = wasm_resource_drop(var tbl, h1);
            if !bad_drop.is_err() { return 6; }
            if !bad_drop.unwrap_err().contains("WASM005") { return 7; }

            // 4. Release borrow then drop succeeds cleanly
            let rel_res = wasm_resource_release_borrow(var tbl, h1);
            if rel_res.is_err() { return 8; }
            if wasm_resource_active_borrow_count(tbl) != 0 { return 9; }

            let ok_drop = wasm_resource_drop(var tbl, h1);
            if ok_drop.is_err() { return 10; }
            if wasm_resource_live_count(tbl) != 0 { return 11; }

            // Double drop traps
            let double_drop = wasm_resource_drop(var tbl, h1);
            if !double_drop.is_err() { return 12; }

            // 5. WasmInstanceLease (0b11 : IGuard)
            var lease = wasm_lease_new(101, 500);
            wasm_lease_acquire_borrow(var lease);
            // Exiting lease with active borrow traps with WASM005
            let exit_err = wasm_lease_exit(var lease);
            if !exit_err.is_err() { return 13; }
            if !exit_err.unwrap_err().contains("WASM005") { return 14; }

            // Release borrow and clean exit
            let rel_lease = wasm_lease_release_borrow(var lease);
            if rel_lease.is_err() { return 15; }
            let exit_ok = wasm_lease_exit(var lease);
            if exit_ok.is_err() { return 16; }

            // 6. High-level std.polyglot.wasm types
            let trap = WasmTrap::create("WASM003", "memory violation");
            if wasm_trap_code(trap) != "WASM003" { return 17; }
            let slice_view = WasmMemorySlice::from_raw_parts(0x1000, 64);
            if wasm_slice_len(slice_view) != 64 { return 18; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/adapter/wit_parser.pilot",
        "selfhost/adapter/wit_abi.pilot",
        "selfhost/adapter/wasm_runtime.pilot",
        "stdlib/std/polyglot/wasm.pilot",
    ];
    let res = eval_m19(deps, script);
    assert_eq!(res, "42", "Component resource table and lease lifecycle must pass");

    guard.finish();
}
