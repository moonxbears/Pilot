//! ============================================================================
//! Milestone 30 Verification Suite:
//! Self-Generating Service, CLI & Persistence Blueprints ([Cli], [Service], [Entity])
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 30,
    test_file_rel: "crates/pilot-analysis/tests/milestone30_self_generating_blueprints.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "stdlib/std/cli/weaver.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "cli_arg_def_new",
                "cli_app_def_new",
                "cli_add_arg",
                "cli_generate_help",
                "cli_parse_argv",
                "cli_parsed_get",
            ],
            required_code_markers: &[
                "CliArgDef",
                "CliAppDef",
                "CliParsedResult",
                "GEN005",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/std/http/service_weaver.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "http_route_def_new",
                "http_service_def_new",
                "http_service_add_route",
                "http_service_generate_openapi",
                "http_service_dispatch",
                "format_rfc9457_problem_json",
            ],
            required_code_markers: &[
                "HttpRouteDef",
                "HttpServiceDef",
                "HttpResponse",
                "GEN001",
                "GEN002",
                "openapi",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/std/db/entity_weaver.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "entity_column_def_new",
                "entity_schema_def_new",
                "entity_add_column",
                "entity_validate_primary_key",
                "entity_validate_predicate",
                "generate_create_table_sql",
                "generate_select_by_id_sql",
                "generate_insert_sql",
                "generate_update_with_optimistic_lock_sql",
                "entity_diff_ddl",
            ],
            required_code_markers: &[
                "EntityColumnDef",
                "EntitySchemaDef",
                "DdlDiffResult",
                "GEN003",
                "GEN004",
                "GEN006",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m30(deps: &[&str], script: &str) -> String {
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
// Test 1: [Cli] Argv Parsing, Subcommands & Flag Collision (GEN005)
// -----------------------------------------------------------------------------
#[test]
fn test_m30_1_cli_argv_parsing_subcommands_and_collision_gen005() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m30_1_cli_argv_parsing_subcommands_and_collision_gen005",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "stdlib/std/cli/weaver.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            var app = cli_app_def_new("pilot-order", "1.0.0", "Pilot Order CLI Tool");
            cli_add_subcommand(var ref app, "create");
            cli_add_subcommand(var ref app, "get");

            let a1 = cli_arg_def_new("customer_id", "-c", "--customer-id", "Target Customer ID", "CUSTOMER_ID", true, "", "u64");
            let a2 = cli_arg_def_new("total_cents", "-t", "--total-cents", "Order Total in Cents", "TOTAL_CENTS", true, "", "i64");
            let a3 = cli_arg_def_new("verbose", "-v", "--verbose", "Enable Verbose Logging", "", false, "false", "bool");

            cli_add_arg(var ref app, a1);
            cli_add_arg(var ref app, a2);
            cli_add_arg(var ref app, a3);

            // Test 1a: Collision detection (GEN005) with short flag collision
            let dup_short = cli_arg_def_new("count", "-c", "--count", "Item Count", "", false, "1", "i64");
            let err_short = cli_add_arg(var ref app, dup_short);
            if err_short.len() == 0 { return 1; }
            let exp_gen005 = "GEN005";
            if err_short.slice(0, exp_gen005.len()) != exp_gen005 { return 2; }

            // Test 1b: Collision detection (GEN005) with long flag collision
            let dup_long = cli_arg_def_new("target_customer", "-x", "--customer-id", "Target customer", "", false, "", "u64");
            let err_long = cli_add_arg(var ref app, dup_long);
            if err_long.len() == 0 { return 3; }
            if err_long.slice(0, exp_gen005.len()) != exp_gen005 { return 4; }

            // Test 1c: Help screen generation
            let help = cli_generate_help(ref app);
            if help.len() < 50 { return 5; }

            // Test 1d: Argv parsing with subcommand and flags
            let argv: List<str> = ["create", "--customer-id", "42", "--total-cents", "2500", "--verbose"];
            let parsed = cli_parse_argv(ref app, argv);
            if !parsed.is_ok { return 6; }
            if parsed.subcommand != "create" { return 7; }
            if cli_parsed_get(ref parsed, "customer_id") != "42" { return 8; }
            if cli_parsed_get(ref parsed, "total_cents") != "2500" { return 9; }
            if cli_parsed_get(ref parsed, "verbose") != "true" { return 10; }

            return 42;
        }
    "#;

    let res = eval_m30(&deps, script);
    assert_eq!(res, "42", "Failed CLI weaver test: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: [Service] OpenAPI 3.1 & Contract Validation (GEN001 & GEN002)
// -----------------------------------------------------------------------------
#[test]
fn test_m30_2_service_openapi_generation_and_contract_validation_gen001_gen002() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m30_2_service_openapi_generation_and_contract_validation_gen001_gen002",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "stdlib/std/http/service_weaver.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            var svc = http_service_def_new("OrdersService", "/v1/orders", "1.0.0");

            // Test 2a: Valid routes
            let r1 = http_route_def_new("POST", "/v1/orders", "create_order", "OrderCreateReq", true);
            let r2 = http_route_def_new("GET", "/v1/orders/{id}", "get_order_by_id", "void", true);
            let e1 = http_service_add_route(var ref svc, r1);
            let e2 = http_service_add_route(var ref svc, r2);
            if e1.len() != 0 || e2.len() != 0 { return 1; }

            // Test 2b: GEN001 Unclosed route parameter brace
            let bad_route_1 = http_route_def_new("GET", "/v1/orders/{id", "bad_handler_1", "void", true);
            let err_gen001 = http_service_add_route(var ref svc, bad_route_1);
            if err_gen001.len() == 0 { return 2; }
            let exp_gen001 = "GEN001";
            if err_gen001.slice(0, exp_gen001.len()) != exp_gen001 { return 3; }

            // Test 2c: GEN002 Payload missing [Json] serialization contract
            let bad_route_2 = http_route_def_new("POST", "/v1/orders/raw", "bad_handler_2", "RawPacket", false);
            let err_gen002 = http_service_add_route(var ref svc, bad_route_2);
            if err_gen002.len() == 0 { return 4; }
            let exp_gen002 = "GEN002";
            if err_gen002.slice(0, exp_gen002.len()) != exp_gen002 { return 5; }

            // Test 2d: OpenAPI 3.1 JSON spec synthesis
            let openapi = http_service_generate_openapi(ref svc);
            if openapi.len() < 100 { return 6; }

            // Test 2e: RFC 9457 Problem JSON generation
            let problem = format_rfc9457_problem_json(422, "Unprocessable Content", "total_cents must be > 0");
            if problem.len() < 50 { return 7; }

            return 42;
        }
    "#;

    let res = eval_m30(&deps, script);
    assert_eq!(res, "42", "Failed Service weaver test: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: [Entity] CRUD, Optimistic Locking & DDL Diff (GEN003, GEN004, GEN006)
// -----------------------------------------------------------------------------
#[test]
fn test_m30_3_entity_crud_optimistic_locking_and_ddl_diff_gen003_gen006() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m30_3_entity_crud_optimistic_locking_and_ddl_diff_gen003_gen006",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "stdlib/std/db/entity_weaver.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            var schema = entity_schema_def_new("orders");

            // Test 3a: Primary key validation (GEN003) when no [Id] is present
            let col_cust = entity_column_def_new("customer_id", "BIGINT", false, false, true, false, false, false);
            entity_add_column(var ref schema, col_cust);
            let err_gen003 = entity_validate_primary_key(ref schema);
            if err_gen003.len() == 0 { return 1; }
            let exp_gen003 = "GEN003";
            if err_gen003.slice(0, exp_gen003.len()) != exp_gen003 { return 2; }

            // Now add [Id] and other columns
            let col_id = entity_column_def_new("id", "BIGINT", true, true, true, false, false, false);
            let col_total = entity_column_def_new("total_cents", "BIGINT", false, false, false, false, false, false);
            let col_ver = entity_column_def_new("ver", "INTEGER", false, false, false, false, true, false);

            var full_schema = entity_schema_def_new("orders");
            entity_add_column(var ref full_schema, col_id);
            entity_add_column(var ref full_schema, col_cust);
            entity_add_column(var ref full_schema, col_total);
            entity_add_column(var ref full_schema, col_ver);

            // Valid primary key passes
            if entity_validate_primary_key(ref full_schema).len() != 0 { return 3; }

            // Test 3b: Column type mismatch in predicate (GEN004)
            let err_gen004 = entity_validate_predicate(ref full_schema, "customer_id", "VARCHAR");
            if err_gen004.len() == 0 { return 4; }
            let exp_gen004 = "GEN004";
            if err_gen004.slice(0, exp_gen004.len()) != exp_gen004 { return 5; }

            // Test 3c: SQL generation
            let create_sql = generate_create_table_sql(ref full_schema);
            if create_sql.len() < 30 { return 6; }

            let select_sql = generate_select_by_id_sql(ref full_schema);
            if select_sql.len() < 25 { return 7; }

            let insert_sql = generate_insert_sql(ref full_schema);
            if insert_sql.len() < 30 { return 8; }

            let update_sql = generate_update_with_optimistic_lock_sql(ref full_schema);
            if update_sql.len() < 30 { return 9; }

            // Test 3d: Schema diff and destructive column drop check (GEN006)
            var prev_schema = entity_schema_def_new("orders");
            let col_legacy = entity_column_def_new("legacy_tag", "VARCHAR", false, false, false, false, false, false); // allow_drop is false
            entity_add_column(var ref prev_schema, col_id);
            entity_add_column(var ref prev_schema, col_legacy);

            let diff_err = entity_diff_ddl(ref full_schema, ref prev_schema);
            if diff_err.is_ok { return 10; }
            let exp_gen006 = "GEN006";
            if diff_err.error_message.slice(0, exp_gen006.len()) != exp_gen006 { return 11; }

            // When allow_drop is true, migration SQL is generated
            var prev_schema2 = entity_schema_def_new("orders");
            let col_legacy_droppable = entity_column_def_new("legacy_tag", "VARCHAR", false, false, false, false, false, true);
            entity_add_column(var ref prev_schema2, col_id);
            entity_add_column(var ref prev_schema2, col_legacy_droppable);

            let diff_ok = entity_diff_ddl(ref full_schema, ref prev_schema2);
            if !diff_ok.is_ok { return 12; }
            if diff_ok.migration_sql.len() < 20 { return 13; }

            return 42;
        }
    "#;

    let res = eval_m30(&deps, script);
    assert_eq!(res, "42", "Failed Entity weaver test: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: End-to-End Microservice, CLI & Persistence Integration
// -----------------------------------------------------------------------------
#[test]
fn test_m30_4_full_stack_microservice_cli_and_persistence_integration() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m30_4_full_stack_microservice_cli_and_persistence_integration",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "stdlib/std/cli/weaver.pilot",
        "stdlib/std/http/service_weaver.pilot",
        "stdlib/std/db/entity_weaver.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // 1. Setup CLI
            var app = cli_app_def_new("admin-cli", "1.0.0", "Admin Order Dispatcher");
            cli_add_subcommand(var ref app, "create");
            cli_add_arg(var ref app, cli_arg_def_new("customer_id", "-c", "--customer-id", "Customer ID", "", true, "", "u64"));
            cli_add_arg(var ref app, cli_arg_def_new("total_cents", "-t", "--total-cents", "Total Cents", "", true, "", "i64"));

            // 2. Setup HTTP Service
            var svc = http_service_def_new("OrdersService", "/v1/orders", "1.0.0");
            http_service_add_route(var ref svc, http_route_def_new("POST", "/v1/orders", "create_order", "OrderCreateReq", true));
            http_service_add_route(var ref svc, http_route_def_new("GET", "/v1/orders/{id}", "get_order_by_id", "void", true));

            // 3. Setup Entity
            var schema = entity_schema_def_new("orders");
            entity_add_column(var ref schema, entity_column_def_new("id", "BIGINT", true, true, true, false, false, false));
            entity_add_column(var ref schema, entity_column_def_new("customer_id", "BIGINT", false, false, true, false, false, false));
            entity_add_column(var ref schema, entity_column_def_new("total_cents", "BIGINT", false, false, false, false, false, false));
            entity_add_column(var ref schema, entity_column_def_new("ver", "INTEGER", false, false, false, false, true, false));

            // 4. Parse incoming CLI args
            let argv: List<str> = ["create", "--customer-id", "42", "--total-cents", "2500"];
            let parsed = cli_parse_argv(ref app, argv);
            if !parsed.is_ok || parsed.subcommand != "create" { return 1; }

            let cid = cli_parsed_get(ref parsed, "customer_id");
            let total = cli_parsed_get(ref parsed, "total_cents");
            if cid != "42" || total != "2500" { return 2; }

            // 5. Construct payload & dispatch HTTP POST
            let req_body = "{\"customer_id\":" + cid + ",\"total_cents\":" + total + "}";
            let resp_post = http_service_dispatch(ref svc, "POST", "/v1/orders", req_body);
            if resp_post.status_code != 201 { return 3; }
            if resp_post.content_type != "application/json" { return 4; }

            // 6. Persist order via Entity SQL
            let insert_sql = generate_insert_sql(ref schema);
            if insert_sql.len() == 0 { return 5; }

            // 7. Dispatch HTTP GET /v1/orders/1 to verify retrieval
            let resp_get = http_service_dispatch(ref svc, "GET", "/v1/orders/1", "");
            if resp_get.status_code != 200 { return 6; }
            if resp_get.body.len() == 0 { return 7; }

            // 8. Contract violation test: invalid negative total_cents produces RFC 9457 Problem JSON
            let invalid_req_body = "{\"customer_id\":42,\"total_cents\":-500}";
            let resp_invalid = http_service_dispatch(ref svc, "POST", "/v1/orders", invalid_req_body);
            if resp_invalid.status_code != 422 { return 8; }
            if resp_invalid.content_type != "application/problem+json" { return 9; }

            return 42;
        }
    "#;

    let res = eval_m30(&deps, script);
    assert_eq!(res, "42", "Failed End-to-End integration test: {}", res);
    guard.finish();
}
