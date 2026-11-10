//! ============================================================================
//! Milestone 21 Verification Suite:
//! Standard `[...]` Boilerplate Weaver Suite & Structural Delegation
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
    milestone_id: 21,
    test_file_rel: "crates/pilot-analysis/tests/milestone21_boilerplate_weaver_and_structural_delegation.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/attr_weaver.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "highest_weight_attr_index",
                "weave_sorted_attr_names",
                "weave_unit_attributes",
            ],
            required_code_markers: &[
                "WeaverProtocolRegistry",
                "ProtocolRegistryEntry",
                "ATTR001",
                "ATTR002",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/core/derive.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "normalize_float_for_hash",
                "combine_hash",
                "format_display_field",
                "validate_lattice_for_clone",
                "derive_eq_source",
                "derive_ord_source",
                "derive_hash_source",
                "derive_clone_source",
                "derive_default_source",
                "derive_display_source",
                "derive_all_structural",
            ],
            required_code_markers: &[
                "EqAttr",
                "OrdAttr",
                "HashAttr",
                "CloneAttr",
                "DefaultAttr",
                "DisplayAttr",
                "SkipEqAttr",
                "SkipHashAttr",
                "RedactAttr",
                "WEAVE003",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/std/serde/derive_json.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "to_camel_case",
                "to_snake_case",
                "to_pascal_case",
                "apply_case_style",
                "check_json_wire_collisions",
                "validate_lattice_for_json",
                "derive_json_serializer",
                "derive_json_deserializer",
                "derive_json_codecs",
            ],
            required_code_markers: &[
                "JsonAttr",
                "JsonRenameAttr",
                "JsonSkipAttr",
                "JsonFlattenAttr",
                "JsonDeriveConfig",
                "JsonFieldMeta",
                "WEAVE003",
                "WEAVE004",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/std/serde/derive_bin.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "bin_encode_varint64",
                "bin_decode_varint64",
                "validate_lattice_for_binary",
                "derive_binary_codecs",
            ],
            required_code_markers: &[
                "BinaryAttr",
                "BinEndianAttr",
                "VarintAttr",
                "BinFieldMeta",
                "VarintDecodeResult",
                "WEAVE003",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/types/delegation.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "validate_interface_compliance",
                "validate_wildcard_ambiguity",
                "synthesize_delegation_thunks",
                "render_thunk_source",
            ],
            required_code_markers: &[
                "DelegationSpec",
                "MethodContract",
                "DelegationThunk",
                "WEAVE005",
                "WEAVE006",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m21(deps: &[&str], script: &str) -> String {
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
// Test 1: Core Derives (Eq, Ord, Hash, Clone, Default, Display, Redact)
// -----------------------------------------------------------------------------
#[test]
fn test_m21_1_core_derives_eq_ord_hash_clone_default_display_and_redact() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m21_1_core_derives_eq_ord_hash_clone_default_display_and_redact",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
fn run_derive_tests() -> i64 {
    // 1. Float hash normalization
    let f1 = normalize_float_for_hash(0.0);
    let f2 = normalize_float_for_hash(-0.0);
    if f1 != f2 {
        return 1;
    }

    // 2. Redacted field formatting in Display
    let normal_repr = format_display_field("username", "alice", false);
    if normal_repr != "username: alice" {
        return 2;
    }
    let redacted_repr = format_display_field("api_key", "secret123", true);
    if redacted_repr != "api_key: ***REDACTED***" {
        return 3;
    }

    // 3. 2-Bit Lattice validation for Clone & Default
    let clone_val_err = validate_lattice_for_clone(0);
    if clone_val_err.len() != 0 {
        return 4;
    }
    let clone_guard_err = validate_lattice_for_clone(3);
    if !clone_guard_err.starts_with("WEAVE003") {
        return 5;
    }
    let def_guard_err = validate_lattice_for_default(3);
    if !def_guard_err.starts_with("WEAVE003") {
        return 6;
    }

    // 4. Source synthesis for structural suite
    var fields: List<DeriveField> = [];
    fields.push(DeriveField {
        name: "id",
        type_name: "i64",
        skip_eq: false,
        skip_hash: false,
        is_redacted: false,
        field_lattice: 0,
    });
    fields.push(DeriveField {
        name: "token",
        type_name: "str",
        skip_eq: true,
        skip_hash: true,
        is_redacted: true,
        field_lattice: 2,
    });

    var derives: List<str> = [];
    derives.push("Eq");
    derives.push("Hash");
    derives.push("Display");

    let synth = derive_all_structural("Session", derives, fields, 2);
    if !synth.contains("IEquatable<Session>") {
        return 7;
    }
    if !synth.contains("IHashable") {
        return 8;
    }
    if !synth.contains("token: ***REDACTED***") {
        return 9;
    }

    return 100;
}
run_derive_tests();
"#;

    let res = eval_m21(&["stdlib/core/derive.pilot"], script);
    assert_eq!(res, "100", "test_m21_1 failed with exit code: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Streaming UTF-8 JSON Weaver, Case Styles & Collision Diagnostics
// -----------------------------------------------------------------------------
#[test]
fn test_m21_2_streaming_utf8_json_weaver_case_styles_and_collision_diagnostics() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m21_2_streaming_utf8_json_weaver_case_styles_and_collision_diagnostics",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
fn run_json_tests() -> i64 {
    // 1. Case conversion: camelCase, snake_case, PascalCase
    let c1 = to_camel_case("user_first_name");
    if c1 != "userFirstName" {
        return 1;
    }
    let s1 = to_snake_case("userFirstName");
    if s1 != "user_first_name" {
        return 2;
    }
    let p1 = to_pascal_case("user_first_name");
    if p1 != "UserFirstName" {
        return 3;
    }

    // 2. WEAVE004 Collision Detection
    var colliding: List<str> = [];
    colliding.push("first_name");
    colliding.push("firstName");
    let col_err = check_json_wire_collisions(colliding, 0); // camelCase
    if !col_err.starts_with("WEAVE004") {
        return 4;
    }

    var non_colliding: List<str> = [];
    non_colliding.push("first_name");
    non_colliding.push("last_name");
    let non_col = check_json_wire_collisions(non_colliding, 0);
    if non_col.len() != 0 {
        return 5;
    }

    // 3. WEAVE003 Lattice check
    let guard_err = validate_lattice_for_json(3);
    if !guard_err.starts_with("WEAVE003") {
        return 6;
    }

    // 4. Synthesizer check
    var fields: List<JsonFieldMeta> = [];
    fields.push(JsonFieldMeta {
        name: "user_name",
        wire_name: "",
        type_name: "str",
        is_skip: false,
        is_flatten: false,
        is_optional: false,
    });
    fields.push(JsonFieldMeta {
        name: "secret_token",
        wire_name: "",
        type_name: "str",
        is_skip: true,
        is_flatten: false,
        is_optional: false,
    });

    let cfg = JsonDeriveConfig::default_camel();
    let code = derive_json_serializer("UserProfile", fields, cfg, 2);
    if !code.contains("userName") {
        return 7;
    }
    if code.contains("secretToken") {
        return 8; // was skipped!
    }

    // 5. The deserializer reads back the wire key the serializer writes and
    //    leaves skipped fields to their defaults.
    let de = derive_json_deserializer("UserProfile", fields, cfg, 2);
    if !de.contains("root.get(\"userName\")") {
        return 9;
    }
    if de.contains("secretToken") || de.contains(".default()") {
        return 10;
    }

    return 200;
}
run_json_tests();
"#;

    let res = eval_m21(&["stdlib/std/serde/derive_json.pilot"], script);
    assert_eq!(res, "200", "test_m21_2 failed with exit code: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Deterministic Wire-Safe Binary Weaver & LEB128 Varint Codecs
// -----------------------------------------------------------------------------
#[test]
fn test_m21_3_deterministic_wire_safe_binary_weaver_and_varint() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m21_3_deterministic_wire_safe_binary_weaver_and_varint",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
fn run_bin_tests() -> i64 {
    // 1. LEB128 Varint Encode & Decode for 300
    let enc300 = bin_encode_varint64(300);
    if enc300.len() != 2 {
        return 1;
    }
    if enc300.get(0) != 172 || enc300.get(1) != 2 {
        return 2;
    }

    let dec300 = bin_decode_varint64(enc300, 0);
    if dec300.value != 300 || dec300.next_offset != 2 {
        return 3;
    }

    // Varint for 0
    let enc0 = bin_encode_varint64(0);
    if enc0.len() != 1 || enc0.get(0) != 0 {
        return 4;
    }
    let dec0 = bin_decode_varint64(enc0, 0);
    if dec0.value != 0 || dec0.next_offset != 1 {
        return 5;
    }

    // 2. WEAVE003 Lattice check
    let guard_err = validate_lattice_for_binary(3);
    if !guard_err.starts_with("WEAVE003") {
        return 6;
    }

    // 3. Binary Codec Synthesis
    var fields: List<BinFieldMeta> = [];
    fields.push(BinFieldMeta {
        name: "count",
        type_name: "i64",
        is_varint: true,
        is_big_endian: false,
        byte_size: 8,
    });

    let pod_code = derive_binary_codecs("PodPoint", fields, 0); // 0b00 POD
    if !pod_code.contains("0b00 POD fast-path") {
        return 7;
    }

    let obj_code = derive_binary_codecs("ObjMsg", fields, 2); // 0b10 Object
    if !obj_code.contains("write_varint") {
        return 8;
    }
    // The decoder mirrors the encoder field by field.
    if !obj_code.contains("read_varint<i64>(offset)") || obj_code.contains(".default()") {
        return 9;
    }

    // 4. Protobuf int64 wire form of -1: nine 0xFF bytes then 0x01.
    let enc_neg = bin_encode_varint64(-1);
    if enc_neg.len() != 10 || enc_neg.get(0) != 255 || enc_neg.get(9) != 1 {
        return 10;
    }
    let dec_neg = bin_decode_varint64(enc_neg, 0);
    if dec_neg.value != -1 || dec_neg.next_offset != 10 {
        return 11;
    }

    return 300;
}
run_bin_tests();
"#;

    let res = eval_m21(&["stdlib/std/serde/derive_bin.pilot"], script);
    assert_eq!(res, "300", "test_m21_3 failed with exit code: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Structural Delegation Thunk Synthesis & Weaver Protocol Registry
// -----------------------------------------------------------------------------
#[test]
fn test_m21_4_structural_delegation_thunk_synthesis_and_weaver_protocol_registry() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m21_4_structural_delegation_thunk_synthesis_and_weaver_protocol_registry",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
fn run_delegation_and_registry_tests() -> i64 {
    // 1. Structural Delegation Thunk Synthesis
    var contract_methods: List<MethodContract> = [];
    contract_methods.push(MethodContract {
        name: "read",
        return_type: "usize",
        params: "buf: slice<u8>",
        is_mut: true,
    });
    contract_methods.push(MethodContract {
        name: "write",
        return_type: "usize",
        params: "buf: slice<u8>",
        is_mut: true,
    });
    contract_methods.push(MethodContract {
        name: "flush",
        return_type: "void",
        params: "",
        is_mut: true,
    });

    var owner_tier1: List<str> = [];
    owner_tier1.push("flush"); // Tier 1 override exists for flush!

    var excluded: List<str> = [];

    let thunks = synthesize_delegation_thunks("socket", contract_methods, owner_tier1, excluded);
    if thunks.len() != 2 {
        return 1; // flush should NOT be delegated because Tier-1 won!
    }
    let t0 = thunks.get(0);
    if t0.name != "read" || t0.target_field != "socket" {
        return 2;
    }
    let rendered = render_thunk_source(t0);
    if !rendered.contains("self.socket.read") || !rendered.contains("Tier 2") {
        return 3;
    }

    // 2. WEAVE005 Missing Interface Compliance
    var target_methods: List<str> = [];
    target_methods.push("read");
    var req_methods: List<str> = [];
    req_methods.push("read");
    req_methods.push("write");
    let comp_err = validate_interface_compliance(target_methods, req_methods, "IReadWrite");
    if !comp_err.starts_with("WEAVE005") {
        return 4;
    }

    // 3. WEAVE006 Wildcard Ambiguity
    var m1: List<str> = [];
    m1.push("close");
    var m2: List<str> = [];
    m2.push("close");
    let amb_err = validate_wildcard_ambiguity("streamA", m1, "streamB", m2);
    if !amb_err.starts_with("WEAVE006") {
        return 5;
    }

    // 4. WeaverProtocolRegistry holds the pillar interfaces (numbered as in
    //    stdlib/core/meta.pilot), never attribute names: attribute pillars are
    //    derived from declarations (milestone50 test_50_3).
    var reg = WeaverProtocolRegistry.create();
    if reg.lookup("IReactiveAttribute") != 5 {
        return 6;
    }
    if reg.lookup("IEcsRule") != 7 {
        return 7;
    }
    if reg.lookup("Clamp") != 0 || reg.lookup("Rule") != 0 || reg.lookup("Eq") != 0 {
        return 8;
    }

    return 400;
}
run_delegation_and_registry_tests();
"#;

    let res = eval_m21(&["selfhost/types/delegation.pilot", "selfhost/attr_weaver.pilot"], script);
    assert_eq!(res, "400", "test_m21_4 failed with exit code: {}", res);

    // 5. Built-in attribute bodies in stdlib/core/meta.pilot: [OnChange]
    //    calls its callback with the old and new value; [TraceSpan] has no
    //    clock or sink in the freestanding core and stops at its seams.
    let on_change = eval_m21(
        &["stdlib/core/meta.pilot"],
        "var seen: List<i64> = [];\n\
         let a = OnChangeAttr { callback: (o, n) => { seen.push(o * 100 + n); }, weight: 50 };\n\
         a.on_changed(3, 7);\n\
         a.on_changed(7, 9);\n\
         return seen.get(0) + \",\" + seen.get(1);\n",
    );
    assert_eq!(on_change, "307,709", "[OnChange] must call its callback with (old, new)");
    let formula_rule = eval_m21(
        &["stdlib/core/meta.pilot"],
        "struct Unit { atk: i64, mult: i64, hp: i64 }\n\
         let f = FormulaAttr { expr_fn: (u) => u.atk * u.mult, weight: 10 };\n\
         var hits: List<i64> = [];\n\
         let r = RuleAttr { when_fn: (u) => u.hp < 10, then_fn: (u) => { hits.push(u.hp); }, weight: 10 };\n\
         let weak = Unit { atk: 6, mult: 7, hp: 4 };\n\
         let strong = Unit { atk: 3, mult: 5, hp: 40 };\n\
         var applied = 0;\n\
         if r.matches(weak) { r.apply(weak); applied = applied + 1; }\n\
         if r.matches(strong) { r.apply(strong); applied = applied + 1; }\n\
         return f.evaluate(weak) + \",\" + f.evaluate(strong) + \",\" + applied + \",\" + hits.get(0);\n",
    );
    assert_eq!(formula_rule, "42,15,1,4", "[Formula] evaluates its expression; [Rule] applies only where it matches");
    let enter = eval_m21(
        &["stdlib/core/meta.pilot"],
        "let t = TraceSpanAttr { name: \"step\", weight: 50 };\nreturn t.on_enter(\"step\");\n",
    );
    assert!(
        enter.starts_with("FAULT: SEAM001: unconnected seam `trace.span_enter`"),
        "TraceSpan.on_enter must stop at its seam, got: {enter}"
    );
    let exit = eval_m21(
        &["stdlib/core/meta.pilot"],
        "let t = TraceSpanAttr { name: \"step\", weight: 50 };\nt.on_exit(0);\nreturn 1;\n",
    );
    assert!(
        exit.starts_with("FAULT: SEAM001: unconnected seam `trace.span_exit`"),
        "TraceSpan.on_exit must stop at its seam, got: {exit}"
    );
    guard.finish();
}
