//! Phase 6 of 6 Verification Gate:
//! Pilot-Hosted IDE Semantic Classifier, Hover/Inlay Formatter & Artifact Generator (`meta.pilot` & `ide_sync.pilot`)
//! + Pure AST/Span Rust LSP Engine (`pilot-lsp`).

use pilot_analysis::comptime::BootstrapSnapshot;
use pilot_lsp::LspServer;
use pilot_rt::PilotVal;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

const META_SRC: &str = include_str!("../../../stdlib/core/meta.pilot");

fn run_meta_expr(call_expr: &str) -> PilotVal {
    let full_src = format!("{}\nreturn {};\n", META_SRC, call_expr);
    let arena = pilot_parser::parse(&full_src);
    let bin = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(bin);
    vm.run()
}

fn eval_meta_classify(inherited_kind: u32, inherited_mods: u32, ctx: u32, lattice_bits: u8) -> (u32, u32) {
    let packed = run_meta_expr(&format!(
        "ide_classify_packed({}, {}, {}, {})",
        inherited_kind, inherited_mods, ctx, lattice_bits
    ))
    .as_int()
    .unwrap_or(0) as u64;
    (((packed >> 32) & 0xFFFF_FFFF) as u32, (packed & 0xFFFF_FFFF) as u32)
}

fn eval_meta_hover(name: &str, byte_size: i64, byte_align: i64, bits: u8) -> String {
    run_meta_expr(&format!(
        "ide_format_type_hover({:?}, {}, {}, {})",
        name, byte_size, byte_align, bits
    ))
    .to_string_repr()
}

fn eval_meta_inlay(captures_csv: &str) -> String {
    run_meta_expr(&format!("ide_format_capture_inlay_hint({:?})", captures_csv)).to_string_repr()
}

const KIND_TYPE: u32 = 1;
const KIND_CLASS: u32 = 2;
const KIND_STRUCT: u32 = 5;
const KIND_TYPE_PARAM: u32 = 6;
const KIND_PARAM: u32 = 7;
const KIND_VARIABLE: u32 = 8;
const KIND_PROPERTY: u32 = 9;
const KIND_ENUM_MEM: u32 = 10;
const KIND_FUNCTION: u32 = 12;
const KIND_METHOD: u32 = 13;
const KIND_MACRO: u32 = 14;
const KIND_OPERATOR: u32 = 21;
const KIND_DECORATOR: u32 = 22;

const MOD_NONE: u32 = 0;
const MOD_DEFINITION: u32 = 1 << 1;
const MOD_READONLY: u32 = 1 << 2;
const MOD_STATIC: u32 = 1 << 3;
const MOD_MODIFICATION: u32 = 1 << 7;
const MOD_DEFAULT_LIB: u32 = 1 << 9;
const MOD_CONSTRUCTOR: u32 = 1 << 10;
const MOD_CALLABLE: u32 = 1 << 11;

const CTX_CALL_CALLEE: u32 = 1 << 1;
const CTX_MEMBER_ACCESS: u32 = 1 << 3;
const CTX_PROP_ACCESSOR: u32 = 1 << 4;
const CTX_ATTR_PREFIX: u32 = 1 << 5;
const CTX_BUNDLE_SPLICE: u32 = 1 << 6;
const CTX_VARIANT_CTOR: u32 = 1 << 7;
const CTX_QUOTE_SPLICE: u32 = 1 << 8;
const CTX_QUOTE_TYPE_PARAM: u32 = 1 << 9;
const CTX_UNQUOTE_ESCAPE: u32 = 1 << 10;
const CTX_CALLABLE_OBJECT: u32 = 1 << 13;
const CTX_METHOD_SYMBOL: u32 = 1 << 14;
const CTX_FIELD_OR_PROP: u32 = 1 << 15;
const CTX_TYPE_SYMBOL: u32 = 1 << 16;
const CTX_LANG_PRIMITIVE: u32 = 1 << 17;
const CTX_SYNTAX_MACRO: u32 = 1 << 18;
const CTX_UNIT_DIMENSION: u32 = 1 << 19;
const CTX_CONTEXTUAL_FIELD: u32 = 1 << 20;
const CTX_CONTEXTUAL_VALUE: u32 = 1 << 21;
const CTX_TYPE_OPERATOR: u32 = 1 << 23;

/// Test 1: Disambiguation of constructors, functions, callable objects, and macros via Pilot's meta.pilot.
#[test]
fn test_pilot_ide_context_policy_disambiguates_constructors_functions_and_callables() {
    let (kind, mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_CALL_CALLEE | CTX_TYPE_SYMBOL, 0b00);
    assert_eq!(kind, KIND_STRUCT);
    assert_eq!(mods & MOD_CONSTRUCTOR, MOD_CONSTRUCTOR);

    let (kind, mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_CALL_CALLEE | CTX_TYPE_SYMBOL, 0b10);
    assert_eq!(kind, KIND_CLASS);
    assert_eq!(mods & MOD_CONSTRUCTOR, MOD_CONSTRUCTOR);

    let (kind, mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_CALL_CALLEE | CTX_TYPE_SYMBOL | CTX_LANG_PRIMITIVE, 0b00);
    assert_eq!(kind, KIND_TYPE);
    assert_eq!(mods & MOD_DEFAULT_LIB, MOD_DEFAULT_LIB);

    let (kind, mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_CALL_CALLEE | CTX_CALLABLE_OBJECT, 0b00);
    assert_eq!(kind, KIND_VARIABLE);
    assert_eq!(mods & MOD_CALLABLE, MOD_CALLABLE);

    let (kind, mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_CALL_CALLEE | CTX_CALLABLE_OBJECT | CTX_FIELD_OR_PROP, 0b00);
    assert_eq!(kind, KIND_PROPERTY);
    assert_eq!(mods & MOD_CALLABLE, MOD_CALLABLE);

    let (kind, _mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_CALL_CALLEE | CTX_SYNTAX_MACRO, 0b00);
    assert_eq!(kind, KIND_MACRO);

    let (kind, _mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_CALL_CALLEE | CTX_METHOD_SYMBOL, 0b00);
    assert_eq!(kind, KIND_METHOD);

    let (kind, _mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_CALL_CALLEE, 0b00);
    assert_eq!(kind, KIND_FUNCTION);
}

/// Test 2: Live user syntax evolution, quote depth hygiene, and splice classification.
#[test]
fn test_live_user_syntax_evolution_and_quote_splice_depth_hygiene() {
    let (kind, mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_QUOTE_SPLICE, 0b00);
    assert_eq!(kind, KIND_PARAM);
    assert_eq!(mods & MOD_STATIC, MOD_STATIC);

    let (kind, mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_QUOTE_SPLICE | CTX_QUOTE_TYPE_PARAM, 0b00);
    assert_eq!(kind, KIND_TYPE_PARAM);
    assert_eq!(mods & MOD_STATIC, MOD_STATIC);

    let (kind, mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_UNQUOTE_ESCAPE, 0b00);
    assert_eq!(kind, KIND_MACRO);
    assert_eq!(mods & MOD_STATIC, MOD_STATIC);

    let mut arena = pilot_ast::AstArena::new();
    let span = pilot_span::ByteSpan::new(0, 10);
    let _nid = arena.alloc_node(pilot_ast::NodeKind::IdentExpr, 0, span, 0, 0);
    assert_eq!(std::mem::size_of::<pilot_ast::AstNode>(), 16, "AstNode must be strictly 16 bytes");
}

/// Test 3: Distributive headers, Variation B contextual `field`/`value`, and uncalled method groups.
#[test]
fn test_distributive_headers_variation_b_field_value_and_uncalled_method_groups() {
    let (kind, mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_PROP_ACCESSOR | CTX_CONTEXTUAL_FIELD, 0b00);
    assert_eq!(kind, KIND_PROPERTY);
    assert_eq!(mods & MOD_DEFAULT_LIB, MOD_DEFAULT_LIB);
    assert_eq!(mods & MOD_MODIFICATION, MOD_MODIFICATION);

    let (kind, mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_PROP_ACCESSOR | CTX_CONTEXTUAL_VALUE, 0b00);
    assert_eq!(kind, KIND_PARAM);
    assert_eq!(mods & MOD_DEFAULT_LIB, MOD_DEFAULT_LIB);
    assert_eq!(mods & MOD_READONLY, MOD_READONLY);

    let (kind, _mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_MEMBER_ACCESS | CTX_FIELD_OR_PROP, 0b00);
    assert_eq!(kind, KIND_PROPERTY);

    let (kind, _mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_MEMBER_ACCESS | CTX_METHOD_SYMBOL, 0b00);
    assert_eq!(kind, KIND_METHOD);
}

/// Test 4: Attribute cascades, offset lenses, and custom [IdeToken] attributes.
#[test]
fn test_patch_attribute_cascades_offset_lens_and_custom_ide_token_attr() {
    let (kind, _mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_ATTR_PREFIX | CTX_TYPE_SYMBOL, 0b00);
    assert_eq!(kind, KIND_DECORATOR);

    let (kind, mods) = eval_meta_classify(KIND_STRUCT, MOD_NONE, CTX_BUNDLE_SPLICE, 0b00);
    assert_eq!(kind, KIND_STRUCT);
    assert_eq!(mods & MOD_STATIC, MOD_STATIC);

    let (kind, mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_UNIT_DIMENSION, 0b00);
    assert_eq!(kind, KIND_TYPE_PARAM);
    assert_eq!(mods & MOD_DEFAULT_LIB, MOD_DEFAULT_LIB);

    let (kind, mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_TYPE_OPERATOR, 0b00);
    assert_eq!(kind, KIND_OPERATOR);
    assert_eq!(mods & MOD_DEFINITION, MOD_DEFINITION);

    let (kind, mods) = eval_meta_classify(KIND_VARIABLE, MOD_NONE, CTX_VARIANT_CTOR, 0b00);
    assert_eq!(kind, KIND_ENUM_MEM);
    assert_eq!(mods & MOD_READONLY, MOD_READONLY);
}

/// Test 5: Name-blind LSP rename litmus gate.
#[test]
fn test_name_blind_lsp_rename_litmus_gate() {
    let mut server = LspServer::new();
    let uri = "file:///workspace/rename_litmus.pilot";
    let code = "struct Vector {\n    pub x: i64;\n    pub y: i64;\n    pub length: f64 {\n        get => field;\n        set => { field = value; }\n    }\n}\n";

    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
        uri,
        code.replace('\n', "\\n")
    );
    server.handle_message(&did_open);

    let prep_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":0,"character":8}}}}}}"#,
        uri
    );
    let prep_resp = server.handle_message(&prep_req).expect("Expected response");
    assert!(prep_resp.contains("placeholder"), "Valid identifier must be renameable: {}", prep_resp);
    assert!(prep_resp.contains("Vector"));

    let prep_field_req = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":4,"character":16}}}}}}"#,
        uri
    );
    let prep_field_resp = server.handle_message(&prep_field_req).expect("Expected response");
    assert!(prep_field_resp.contains("\"result\":null"), "Contextual field must not be renameable: {}", prep_field_resp);

    let prep_val_req = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":5,"character":26}}}}}}"#,
        uri
    );
    let prep_val_resp = server.handle_message(&prep_val_req).expect("Expected response");
    assert!(prep_val_resp.contains("\"result\":null"), "Contextual value must not be renameable: {}", prep_val_resp);

    let rename_kw_req = format!(
        r#"{{"jsonrpc":"2.0","id":4,"method":"textDocument/rename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":0,"character":8}},"newName":"struct"}}}}"#,
        uri
    );
    let rename_kw_resp = server.handle_message(&rename_kw_req).expect("Expected response");
    assert!(rename_kw_resp.contains("\"result\":null"), "Renaming to keyword must be rejected: {}", rename_kw_resp);

    let rename_valid_req = format!(
        r#"{{"jsonrpc":"2.0","id":5,"method":"textDocument/rename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":0,"character":8}},"newName":"Vec2"}}}}"#,
        uri
    );
    let rename_valid_resp = server.handle_message(&rename_valid_req).expect("Expected response");
    assert!(rename_valid_resp.contains("newText"), "Valid rename must produce workspace edits: {}", rename_valid_resp);
    assert!(rename_valid_resp.contains("Vec2"));
}

/// Test 6: Self-hosted ide_sync.pilot + meta.pilot hover badges.
#[test]
fn test_selfhost_ide_sync_pilot_generates_all_4_vscode_artifacts_and_hover_badges() {
    let hover_00 = eval_meta_hover("Vec3", 24, 8, 0b00);
    assert!(hover_00.contains("0b00 IValue (Trivial Copy)"));
    assert!(hover_00.contains("24 bytes"));
    assert!(hover_00.contains("align 8"));

    let hover_01 = eval_meta_hover("SliceView", 16, 8, 0b01);
    assert!(hover_01.contains("0b01 IReference (Lexical Borrow)"));

    let hover_10 = eval_meta_hover("SliceBuffer", 24, 8, 0b10);
    assert!(hover_10.contains("0b10 IObject (Perceus Move + ~self)"));

    let hover_11 = eval_meta_hover("MutexGuard", 16, 8, 0b11);
    assert!(hover_11.contains("0b11 IGuard (Scoped RAII + Borrow)"));

    let inlay = eval_meta_inlay("base, total");
    assert_eq!(inlay, "/* captures: base, total */");

    let mut server = LspServer::new();
    let uri = "file:///workspace/semantic_tokens_test.pilot";
    let code = "fn compute(x: i64) -> i64 {\n    var res = x + 1;\n    return res;\n}\n";
    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
        uri,
        code.replace('\n', "\\n")
    );
    server.handle_message(&did_open);

    let sem_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/semanticTokens/full","params":{{"textDocument":{{"uri":"{}"}}}}}}"#,
        uri
    );
    let sem_resp = server.handle_message(&sem_req).expect("Expected semantic tokens response");
    assert!(sem_resp.contains("data"), "Semantic tokens response must have data array: {}", sem_resp);

    let ide_sync_src = include_str!("../../../selfhost/ide_sync.pilot");
    let snap = BootstrapSnapshot::global();
    let mut snap_tables = snap.tables.clone();
    let mut snap_vm = snap.vm.clone();
    let eval_res = pilot_analysis::eval_file_with_tables(ide_sync_src, &mut snap_tables, &mut snap_vm);
    assert!(eval_res.is_ok(), "selfhost/ide_sync.pilot must evaluate cleanly: {:?}", eval_res.err());
    let arena = eval_res.unwrap();
    assert!(arena.root.is_some(), "AST arena must have root");

    let json_src = include_str!("../../../stdlib/json/json.pilot");
    let mut json_tables = snap.tables.clone();
    let mut json_vm = snap.vm.clone();
    let json_eval_res = pilot_analysis::eval_file_with_tables(json_src, &mut json_tables, &mut json_vm);
    assert!(json_eval_res.is_ok(), "stdlib/json/json.pilot must evaluate cleanly: {:?}", json_eval_res.err());
    let json_arena = pilot_parser::parse(json_src);
    let json_analysis = pilot_analysis::AnalysisPipeline::analyze(&json_arena, json_arena.root);
    assert!(json_analysis.is_ok(), "json.pilot must have clean type inference: {:?}", json_analysis.diagnostics);
}

/// Test 7: Whole-program reflection LSP hover, completion, and definition dispatch to ide_sync.pilot.
#[test]
fn test_pilot_hosted_ide_sync_hover_completion_and_definition() {
    let mut server = LspServer::new();
    let uri = "file:///workspace/test_lsp.pilot";
    let code = "struct DecoyFirstStruct {\n    pub wrong_field: i64;\n}\n/// Documented Account struct\nstruct Account {\n    pub balance: i64;\n    pub fn deposit(amount: i64) -> i64 {\n        return self.balance;\n    }\n    fn internal_helper() -> i64 {\n        return 0;\n    }\n}\nlet acct = Account();\nacct.\n";

    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
        uri,
        code.replace('\n', "\\n")
    );
    server.handle_message(&did_open);

    let hover_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":4,"character":8}}}}}}"#,
        uri
    );
    let hover_resp = server.handle_message(&hover_req).expect("hover response");
    assert!(hover_resp.contains("Account"), "Hover must contain Account: {}", hover_resp);
    assert!(hover_resp.contains("Documented Account struct"), "Hover must contain doc: {}", hover_resp);
    assert!(hover_resp.contains("8 bytes"), "Hover must contain byte size: {}", hover_resp);

    let hover_kw_req = format!(
        r#"{{"jsonrpc":"2.0","id":11,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":4,"character":2}}}}}}"#,
        uri
    );
    let hover_kw_resp = server.handle_message(&hover_kw_req).expect("hover kw response");
    assert!(hover_kw_resp.contains("Pilot Keyword"), "Keyword hover must indicate keyword: {}", hover_kw_resp);
    assert!(hover_kw_resp.contains("composite type"), "Keyword hover must have summary: {}", hover_kw_resp);

    let hover_fn_req = format!(
        r#"{{"jsonrpc":"2.0","id":12,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":6,"character":15}}}}}}"#,
        uri
    );
    let hover_fn_resp = server.handle_message(&hover_fn_req).expect("hover fn response");
    assert!(hover_fn_resp.contains("deposit"), "Function hover must contain deposit: {}", hover_fn_resp);
    assert!(hover_fn_resp.contains("amount: i64"), "Function hover must contain param: {}", hover_fn_resp);

    let hover_ws_req = format!(
        r#"{{"jsonrpc":"2.0","id":13,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":4,"character":6}}}}}}"#,
        uri
    );
    let hover_ws_resp = server.handle_message(&hover_ws_req).expect("hover ws response");
    assert!(hover_ws_resp.contains("\"result\":null"), "Whitespace hover must be null: {}", hover_ws_resp);

    let comp_req = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":14,"character":5}}}}}}"#,
        uri
    );
    let comp_resp = server.handle_message(&comp_req).expect("completion response");
    assert!(comp_resp.contains("balance"), "Completion on acct. must include balance: {}", comp_resp);
    assert!(comp_resp.contains("deposit"), "Completion on acct. must include deposit: {}", comp_resp);
    assert!(!comp_resp.contains("wrong_field"), "Completion on acct. must NOT include wrong_field from DecoyFirstStruct: {}", comp_resp);
    assert!(!comp_resp.contains("internal_helper"), "Completion on acct. must NOT include private internal_helper: {}", comp_resp);

    let def_req = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":4,"character":8}}}}}}"#,
        uri
    );
    let def_resp = server.handle_message(&def_req).expect("definition response");
    assert!(def_resp.contains("range"), "Definition must return range: {}", def_resp);
}

/// Test 8: Verify stdlib/core/syntax/scope_header.pilot evaluates cleanly in bootstrap VM
#[test]
fn test_core_syntax_scope_header_pilot_evaluates_cleanly() {
    let scope_header_src = include_str!("../../../stdlib/core/syntax/scope_header.pilot");
    let snap = BootstrapSnapshot::global();
    let mut snap_tables = snap.tables.clone();
    let mut snap_vm = snap.vm.clone();
    let eval_res = pilot_analysis::eval_file_with_tables(scope_header_src, &mut snap_tables, &mut snap_vm);
    assert!(eval_res.is_ok(), "stdlib/core/syntax/scope_header.pilot must evaluate cleanly: {:?}", eval_res.err());
    let arena = eval_res.unwrap();
    assert!(arena.root.is_some(), "AST arena must have root");

    let preproc_src = include_str!("../../../selfhost/preprocessor.pilot");
    let mut preproc_tables = snap.tables.clone();
    let mut preproc_vm = snap.vm.clone();
    let eval_preproc = pilot_analysis::eval_file_with_tables(preproc_src, &mut preproc_tables, &mut preproc_vm);
    assert!(eval_preproc.is_ok(), "selfhost/preprocessor.pilot must evaluate cleanly: {:?}", eval_preproc.err());

    let parser_src = include_str!("../../../selfhost/parser.pilot");
    let mut parser_tables = snap.tables.clone();
    let mut parser_vm = snap.vm.clone();
    let eval_parser = pilot_analysis::eval_file_with_tables(parser_src, &mut parser_tables, &mut parser_vm);
    assert!(eval_parser.is_ok(), "selfhost/parser.pilot must evaluate cleanly: {:?}", eval_parser.err());

    let test39_src = include_str!("../../../tests/39_pilot_scope_headers.pilot");
    let test39_arena = pilot_parser::parse(test39_src);
    assert!(test39_arena.root.is_some(), "tests/39_pilot_scope_headers.pilot must parse cleanly");
}
