//! ============================================================================
//! Milestone 20 Verification Suite:
//! Live Schema & Wire Protocol Absorber (`use sql`, `use openapi`, `use proto`)
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 20,
    test_file_rel: "crates/pilot-analysis/tests/milestone20_live_schema_and_wire_protocol_absorber.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/adapter/absorb_sql.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "sql_parse_create_table",
                "sql_apply_alter_table",
                "sql_replay_migration",
                "sql_parse_annotated_query",
                "sql_synthesize_query_wrapper",
            ],
            required_code_markers: &[
                "SqlDialect",
                "SqlColumnDef",
                "SqlTableDef",
                "SqlCatalog",
                "SqlQueryDef",
                "SCH002",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/adapter/absorb_openapi.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "openapi_doc_new",
                "openapi_resolve_ref",
                "openapi_detect_recursion",
                "openapi_parse_spec",
                "openapi_synthesize_client_interface",
            ],
            required_code_markers: &[
                "OpenApiProperty",
                "OpenApiSchema",
                "OpenApiOperation",
                "OpenApiDoc",
                "allOf",
                "oneOf",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/adapter/absorb_proto.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "proto_make_tag",
                "proto_zigzag_encode64",
                "proto_zigzag_decode64",
                "proto_encode_varint",
                "proto_decode_varint",
                "proto_parse_source",
                "proto_validate_reserved_tags",
                "proto_synthesize_dual_views",
            ],
            required_code_markers: &[
                "ProtoWireType",
                "ProtoVarintResult",
                "ProtoFieldDef",
                "ProtoMessageDef",
                "ProtoDoc",
                "SCH003",
                "SCH005",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m20(deps: &[&str], script: &str) -> String {
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
// Test 1: SQL DDL Replay, Multi-Migration & Annotated Query Synthesis
// -----------------------------------------------------------------------------
#[test]
fn test_m20_1_sql_ddl_replay_and_annotated_query_synthesis() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m20_1_sql_ddl_replay_and_annotated_query_synthesis",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // 1. Initial Migration (CREATE TABLE)
            var cat = sql_catalog_new();
            let mig1 = "CREATE TABLE users (\n"
                + "    id INT PRIMARY KEY,\n"
                + "    name TEXT NOT NULL,\n"
                + "    status INT\n"
                + ");\n";
            let c1 = sql_replay_migration(var cat, mig1);
            if c1 != 1 { return 1; }
            if cat.tables.len() != 1 { return 2; }

            let t0 = cat.tables.get(0);
            if t0.name != "users" { return 3; }
            if t0.columns.len() != 3 { return 4; }
            if t0.primary_keys.len() != 1 { return 5; }

            // 2. Second Migration (ALTER TABLE ADD COLUMN & DROP COLUMN)
            let mig2 = "ALTER TABLE users ADD COLUMN email TEXT;\n"
                + "ALTER TABLE users DROP COLUMN status;\n";
            let c2 = sql_replay_migration(var cat, mig2);
            if c2 != 2 { return 6; }

            let t0_updated = cat.tables.get(0);
            if t0_updated.columns.len() != 3 { return 7; }
            if sql_table_find_col(t0_updated, "email") < 0 { return 8; }
            if sql_table_find_col(t0_updated, "status") >= 0 { return 9; }

            // 3. Parse annotated query
            let query_src = "-- name: GetUserById :one\n"
                + "SELECT id, name, email FROM users WHERE id = $1;\n";
            let q = sql_parse_annotated_query(query_src);
            if q.name != "GetUserById" { return 10; }
            if q.return_kind != "one" { return 11; }
            if q.target_tbl != "users" { return 12; }
            if q.params.len() != 1 { return 13; }

            // 4. Synthesize query wrapper with dialect placeholders
            let wrapper_pg = sql_synthesize_query_wrapper(cat, q, SqlDialect::Postgres);
            if !wrapper_pg.contains("$1") { return 14; }
            if !wrapper_pg.contains("opt<RowView>") { return 15; }

            let wrapper_sqlite = sql_synthesize_query_wrapper(cat, q, SqlDialect::Sqlite);
            if !wrapper_sqlite.contains("?1") { return 16; }

            // 5. Query on non-existent table emits SCH002 diagnostic
            var bad_q = sql_parse_annotated_query("-- name: BadQuery :many\nSELECT * FROM non_existent WHERE id = ?;");
            let bad_wrapper = sql_synthesize_query_wrapper(cat, bad_q, SqlDialect::MySql);
            if !bad_wrapper.contains("SCH002") { return 17; }

            return 42;
        }
    "##;

    let deps = &["selfhost/adapter/absorb_sql.pilot"];
    let res = eval_m20(deps, script);
    assert_eq!(res, "42", "SQL DDL replay and query synthesis must succeed");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: OpenAPI Schema DAG, allOf Splicing & Client Interfaces
// -----------------------------------------------------------------------------
#[test]
fn test_m20_2_openapi_schema_dag_allof_and_client_interfaces() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m20_2_openapi_schema_dag_allof_and_client_interfaces",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // $ref resolver
            let resolved = openapi_resolve_ref("#/components/schemas/UserProfile");
            if resolved != "UserProfile" { return 1; }

            // OpenAPI spec with components, allOf, oneOf, and paths
            let spec_json = "{\n"
                + "  \"openapi\": \"3.0.0\",\n"
                + "  \"info\": { \"title\": \"PetStoreAPI\", \"version\": \"2.4.0\" },\n"
                + "  \"components\": {\n"
                + "    \"schemas\": {\n"
                + "      \"User\": {\n"
                + "        \"type\": \"object\",\n"
                + "        \"allOf\": [ { \"$ref\": \"#/components/schemas/BaseEntity\" } ],\n"
                + "        \"properties\": {\n"
                + "          \"id\": { \"type\": \"integer\" },\n"
                + "          \"name\": { \"type\": \"string\" },\n"
                + "          \"parent\": { \"$ref\": \"#/components/schemas/User\" }\n"
                + "        }\n"
                + "      }\n"
                + "    }\n"
                + "  },\n"
                + "  \"paths\": {\n"
                + "    \"/users/{id}\": {\n"
                + "      \"get\": {\n"
                + "        \"operationId\": \"getUserById\"\n"
                + "      }\n"
                + "    }\n"
                + "  }\n"
                + "}\n";

            let doc = openapi_parse_spec(spec_json);
            if doc.title != "PetStoreAPI" { return 2; }
            if doc.schemas.len() != 1 { return 3; }

            let s = doc.schemas.get(0);
            if s.name != "User" { return 4; }
            if s.all_of.len() != 1 { return 5; }
            if s.all_of.get(0) != "BaseEntity" { return 6; }
            if s.properties.len() != 3 { return 7; }

            // Self-referential cycle detected
            if !s.is_recursive { return 8; }

            // Path operations
            if doc.operations.len() != 1 { return 9; }
            let op = doc.operations.get(0);
            if op.path != "/users/{id}" { return 10; }
            if op.operation_id != "getUserById" { return 11; }
            if op.path_params.len() != 1 { return 12; }
            if op.path_params.get(0) != "id" { return 13; }

            // Synthesize client interface
            let client_iface = openapi_synthesize_client_interface(doc);
            if !client_iface.contains("pub interface IApiClient") { return 14; }
            if !client_iface.contains("fn getUserById(id: int64) -> User;") { return 15; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/adapter/absorb_sql.pilot",
        "selfhost/adapter/absorb_openapi.pilot",
    ];
    let res = eval_m20(deps, script);
    assert_eq!(res, "42", "OpenAPI schema DAG and client interface synthesis must succeed");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Protobuf ZigZag, Varint & Wire Packet Codecs
// -----------------------------------------------------------------------------
#[test]
fn test_m20_3_protobuf_zigzag_varint_and_wire_packet_codecs() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m20_3_protobuf_zigzag_varint_and_wire_packet_codecs",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // 1. Tag packing & unpacking
            let tag = proto_make_tag(2, 0); // field 2, varint (0)
            if tag != 16 { return 1; }
            if proto_tag_field_number(tag) != 2 { return 2; }
            if proto_tag_wire_type(tag) != 0 { return 3; }

            // 2. ZigZag 64-bit encoding & decoding
            if proto_zigzag_encode64(0) != 0 { return 4; }
            if proto_zigzag_encode64(-1) != 1 { return 5; }
            if proto_zigzag_encode64(1) != 2 { return 6; }
            if proto_zigzag_encode64(-2) != 3 { return 7; }

            if proto_zigzag_decode64(0) != 0 { return 8; }
            if proto_zigzag_decode64(1) != -1 { return 9; }
            if proto_zigzag_decode64(2) != 1 { return 10; }
            if proto_zigzag_decode64(3) != -2 { return 11; }

            // Round-trip
            let val: i64 = 12345678;
            if proto_zigzag_decode64(proto_zigzag_encode64(val)) != val { return 12; }
            let neg_val: i64 = -87654321;
            if proto_zigzag_decode64(proto_zigzag_encode64(neg_val)) != neg_val { return 13; }

            // 3. LEB128 Varint encode & decode
            // 300 = 0x12C -> (0x2C | 0x80) = 0xAC (172), 0x02 (2)
            let encoded = proto_encode_varint(300);
            if encoded.len() != 2 { return 14; }
            if encoded.get(0) != 172 { return 15; }
            if encoded.get(1) != 2 { return 16; }

            let decoded = proto_decode_varint(encoded, 0);
            if !decoded.is_valid { return 17; }
            if decoded.value != 300 { return 18; }
            if decoded.bytes_read != 2 { return 19; }
            if decoded.value != 300 { return 18; }
            if decoded.bytes_read != 2 { return 19; }

            // 4. Truncated wire packet (SCH005)
            var truncated: List<i64> = [];
            truncated.push(128); // Bit 7 set indicates continuation, but stream ends!
            let bad_decode = proto_decode_varint(truncated, 0);
            if bad_decode.is_valid { return 20; }
            if !bad_decode.err_message.contains("SCH005") { return 21; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/adapter/absorb_sql.pilot",
        "selfhost/adapter/absorb_proto.pilot",
    ];
    let res = eval_m20(deps, script);
    assert_eq!(res, "42", "Protobuf ZigZag and Varint codecs must succeed");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Protobuf Reserved Tags & Dual View Synthesis
// -----------------------------------------------------------------------------
#[test]
fn test_m20_4_protobuf_reserved_tags_and_dual_view_synthesis() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m20_4_protobuf_reserved_tags_and_dual_view_synthesis",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // 1. Diagnostic code catalog (SCH001 .. SCH005)
            if !schema_diag_code(1).starts_with("SCH001") { return 1; }
            if !schema_diag_code(2).starts_with("SCH002") { return 2; }
            if !schema_diag_code(3).starts_with("SCH003") { return 3; }
            if !schema_diag_code(4).starts_with("SCH004") { return 4; }
            if !schema_diag_code(5).starts_with("SCH005") { return 5; }

            // 2. Parse Protobuf source
            let proto_src = "syntax = \"proto3\";\n"
                + "package telemetry.v1;\n"
                + "message MetricBatch {\n"
                + "    int64 timestamp = 1;\n"
                + "    repeated int32 values = 2 [packed=true];\n"
                + "}\n";

            let doc = proto_parse_source(proto_src);
            if doc.syntax != "proto3" { return 6; }
            if doc.package_name != "telemetry.v1" { return 7; }
            if doc.messages.len() != 1 { return 8; }

            let msg = doc.messages.get(0);
            if msg.name != "MetricBatch" { return 9; }
            if msg.fields.len() != 2 { return 10; }

            let f0 = msg.fields.get(0);
            if f0.name != "timestamp" { return 11; }
            if f0.field_number != 1 { return 12; }
            if f0.is_repeated { return 13; }

            let f1 = msg.fields.get(1);
            if f1.name != "values" { return 14; }
            if f1.field_number != 2 { return 15; }
            if !f1.is_repeated { return 16; }
            if !f1.is_packed { return 17; }

            // 3. Valid reserved tags validation
            let v_ok = proto_validate_reserved_tags(msg);
            if !v_ok.is_ok { return 18; }

            // 4. Invalid reserved tag (19000..19999 -> SCH003)
            var bad_msg = proto_message_new("BadMessage");
            bad_msg.fields.push(ProtoFieldDef {
                name:         "reserved_field",
                field_number: 19001,
                field_type:   "int32",
                is_repeated:  false,
                is_packed:    false,
                oneof_group:  "",
            });
            let v_bad = proto_validate_reserved_tags(bad_msg);
            if v_bad.is_ok { return 19; }
            if !v_bad.err_message.contains("SCH003") { return 20; }

            // 5. Dual view synthesis (0b10 Owned Builder + 0b01 Borrowed View)
            let views_code = proto_synthesize_dual_views(msg);
            if !views_code.contains("pub struct MetricBatch : IObject") { return 21; }
            if !views_code.contains("pub struct MetricBatchView : IReference") { return 22; }
            if !views_code.contains("pub fn get_timestamp(ref self) -> int64") { return 23; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/adapter/absorb_sql.pilot",
        "selfhost/adapter/absorb_proto.pilot",
    ];
    let res = eval_m20(deps, script);
    assert_eq!(res, "42", "Protobuf reserved tags and dual views synthesis must succeed");

    guard.finish();
}
