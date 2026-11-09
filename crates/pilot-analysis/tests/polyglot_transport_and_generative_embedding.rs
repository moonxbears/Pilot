//! ============================================================================
//! Polyglot Transport Languages & Generative Cross-Language Data Manipulation
//! Subsystems: Programming Languages (Execution/FFI) vs Transport Languages (Data/DOM)
//! Supported Formats: JSON, YAML/YML, XML, TOML
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

fn eval_transport(deps: &[&str], script: &str) -> String {
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
// Test 1: Bidirectional JSON DOM Creation, Mutation & Path Queries
// -----------------------------------------------------------------------------
#[test]
fn test_polyglot_json_bidirectional_manipulation() {
    let script = r##"
        pub fn main() -> i64 {
            // 1. Construct JSON Object
            var doc = json_object();
            doc.set("name", json_str("Pilot"));
            doc.set("is_fast", json_bool(true));

            // Nested config object
            var cfg = json_object();
            cfg.set("port", json_num(8080.0));
            cfg.set("host", json_str("127.0.0.1"));
            doc.set("server", cfg);

            // Array of tags
            var tags = json_array();
            tags.push(json_str("compiler"));
            tags.push(json_str("polyglot"));
            tags.push(json_str("zero-copy"));
            doc.set("tags", tags);

            // 2. Validate lookups & mutations
            if !doc.has("name") { return 1; }
            if doc.get("name").str_val != "Pilot" { return 2; }
            if !doc.get("is_fast").bool_val { return 3; }

            // 3. Nested path query
            let host_node = doc.query_path("server.host");
            if host_node.str_val != "127.0.0.1" { return 4; }

            // 4. Array inspection
            let arr_node = doc.get("tags");
            if arr_node.len() != 3 { return 5; }
            if arr_node.get_at(1).str_val != "polyglot" { return 6; }

            // 5. Dynamic mutation & deletion
            doc.set("version", json_str("1.0.0"));
            if !doc.has("version") { return 7; }
            doc.remove("version");
            if doc.has("version") { return 8; }

            // 6. Serialization
            let s = doc.stringify();
            if !s.starts_with("{") || !s.ends_with("}") { return 9; }
            if !s.contains("\"name\": \"Pilot\"") { return 10; }

            return 42;
        }
    "##;

    let deps = &["stdlib/std/polyglot/transport/json.pilot"];
    let res = eval_transport(deps, script);
    assert_eq!(res, "42", "JSON bidirectional manipulation and path querying must pass");
}

// -----------------------------------------------------------------------------
// Test 2: Bidirectional YAML AST & Indentation-Aware Processing
// -----------------------------------------------------------------------------
#[test]
fn test_polyglot_yaml_bidirectional_manipulation() {
    let script = r##"
        pub fn main() -> i64 {
            // 1. Construct YAML Mapping
            var doc = yaml_mapping();
            doc.set("service", yaml_scalar("gateway"));
            doc.set("replicas", yaml_scalar("3"));

            // Sequence
            var endpoints = yaml_sequence();
            endpoints.push(yaml_scalar("/api/v1/auth"));
            endpoints.push(yaml_scalar("/api/v1/query"));
            doc.set("endpoints", endpoints);

            // 2. Lookups
            if !doc.has("service") { return 1; }
            if doc.get("service").scalar_val != "gateway" { return 2; }

            let ep = doc.get("endpoints");
            if ep.len() != 2 { return 3; }
            if ep.get_at(0).scalar_val != "/api/v1/auth" { return 4; }

            // 3. Emission
            let yml_text = doc.emit();
            if !yml_text.contains("service: gateway") { return 5; }

            // 4. Parsing
            let src = "app: pilot_agent\nenv: production\n";
            let parsed = yaml_parse(src);
            if parsed.get("app").scalar_val != "pilot_agent" { return 6; }
            if parsed.get("env").scalar_val != "production" { return 7; }

            return 42;
        }
    "##;

    let deps = &["stdlib/std/polyglot/transport/yaml.pilot"];
    let res = eval_transport(deps, script);
    assert_eq!(res, "42", "YAML bidirectional mapping and sequence operations must pass");
}

// -----------------------------------------------------------------------------
// Test 3: Bidirectional XML DOM, Attributes & Tag Search
// -----------------------------------------------------------------------------
#[test]
fn test_polyglot_xml_bidirectional_manipulation() {
    let script = r##"
        pub fn main() -> i64 {
            // 1. Construct XML Tree
            var root = xml_element("configuration");
            root.set_attr("version", "2.0");
            root.set_attr("env", "staging");

            var db_elem = xml_element("database");
            db_elem.set_attr("driver", "postgres");
            db_elem.set_attr("pool", "32");

            var host = xml_element("host");
            host.text_content = "db.internal.net";
            db_elem.add_child(host);

            root.add_child(db_elem);

            // 2. Validate Attributes
            if !root.has_attr("version") { return 1; }
            if root.get_attr("version") != "2.0" { return 2; }

            // 3. Child Navigation
            let db_node = root.find_first("database");
            if db_node.get_attr("driver") != "postgres" { return 3; }

            let all_dbs = root.find_all("database");
            if all_dbs.len() != 1 { return 4; }

            // 4. Emission
            let xml_str = root.emit();
            if !xml_str.starts_with("<configuration") { return 5; }
            if !xml_str.contains("version=\"2.0\"") { return 6; }
            if !xml_str.contains("</configuration>") { return 7; }

            // 5. Parse
            let sample = "<user id=\"100\"><name>Alice</name></user>";
            let parsed = xml_parse(sample);
            if parsed.tag_name != "user" { return 8; }
            if parsed.get_attr("id") != "100" { return 9; }

            return 42;
        }
    "##;

    let deps = &["stdlib/std/polyglot/transport/xml.pilot"];
    let res = eval_transport(deps, script);
    assert_eq!(res, "42", "XML bidirectional DOM manipulation and attribute queries must pass");
}

// -----------------------------------------------------------------------------
// Test 4: Bidirectional TOML AST & Language Taxonomy Classification
// -----------------------------------------------------------------------------
#[test]
fn test_polyglot_toml_and_taxonomy_duality() {
    let script = r##"
        pub fn main() -> i64 {
            // 1. Construct TOML Table
            var doc = toml_table();
            doc.set("package_name", toml_str("pilot_core"));
            doc.set("edition", toml_int(2026));
            doc.set("is_release", toml_bool(true));

            // Array
            var authors = toml_array();
            authors.push(toml_str("Pilot Authors"));
            doc.set("authors", authors);

            // 2. Validate Table Lookups
            if !doc.has("package_name") { return 1; }
            if doc.get("package_name").str_val != "pilot_core" { return 2; }
            if doc.get("edition").int_val != 2026 { return 3; }

            // 3. TOML Emission
            let toml_str_out = doc.emit();
            if !toml_str_out.contains("package_name = \"pilot_core\"") { return 4; }

            // 4. Parse simple TOML
            let src = "title = \"Pilot Manifest\"\ncount = 10\n";
            let parsed: TomlValue = toml_parse(src);
            if parsed.get("title").str_val != "Pilot Manifest" { return 5; }

            // 5. Polyglot Subsystem Taxonomy Duality
            if !transport_is_data_language("json") { return 6; }
            if !transport_is_data_language("yaml") { return 7; }
            if !transport_is_data_language("xml") { return 8; }
            if !transport_is_data_language("toml") { return 9; }

            if !transport_is_programming_language("rust") { return 10; }
            if !transport_is_programming_language("py") { return 11; }
            if !transport_is_programming_language("js") { return 12; }
            if !transport_is_programming_language("c") { return 13; }
            if !transport_is_programming_language("wasm") { return 14; }

            if polyglot_subsystem_category("json") != "transport" { return 15; }
            if polyglot_subsystem_category("python") != "programming" { return 16; }

            return 42;
        }
    "##;

    let deps = &[
        "stdlib/std/polyglot/transport/json.pilot",
        "stdlib/std/polyglot/transport/yaml.pilot",
        "stdlib/std/polyglot/transport/xml.pilot",
        "stdlib/std/polyglot/transport/toml.pilot",
        "stdlib/std/polyglot/transport/mod.pilot",
    ];
    let res = eval_transport(deps, script);
    assert_eq!(res, "42", "TOML AST and language taxonomy duality must pass");
}
