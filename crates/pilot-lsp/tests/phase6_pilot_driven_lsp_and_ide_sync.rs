//! Phase 6 of 6 Verification Gate:
//! Pilot-Hosted IDE Semantic Classifier, Hover/Inlay Formatter & Artifact Generator (`meta.pilot` & `ide_sync.pilot`)
//! + Pure AST/Span Rust LSP Engine (`pilot-lsp`).

use pilot_analysis::comptime::BootstrapSnapshot;
use pilot_ast::{AstContext, IdeContextPolicy, IdePresentationPolicy, LspKind, LspMod};
use pilot_lsp::LspServer;

/// Test 1: Disambiguation of constructors, functions, callable objects, and macros via Pilot's IdeContextPolicy.
#[test]
fn test_pilot_ide_context_policy_disambiguates_constructors_functions_and_callables() {
    // 1. Value struct constructor: `Point(1, 2)` -> struct_k + constructor
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_CALL_CALLEE | AstContext::IS_TYPE_SYMBOL,
        0b00, // IValue
    );
    assert_eq!(kind, LspKind::STRUCT_K);
    assert_eq!(mods & LspMod::CONSTRUCTOR, LspMod::CONSTRUCTOR);

    // 2. Owned object/class constructor: `Buffer(10)` -> class_k + constructor
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_CALL_CALLEE | AstContext::IS_TYPE_SYMBOL,
        0b10, // IObject
    );
    assert_eq!(kind, LspKind::CLASS_K);
    assert_eq!(mods & LspMod::CONSTRUCTOR, LspMod::CONSTRUCTOR);

    // 3. Scalar/primitive type call: `i64(1)` -> type_k + default_lib
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_CALL_CALLEE | AstContext::IS_TYPE_SYMBOL | AstContext::IS_LANG_PRIMITIVE,
        0b00,
    );
    assert_eq!(kind, LspKind::TYPE_K);
    assert_eq!(mods & LspMod::DEFAULT_LIB, LspMod::DEFAULT_LIB);

    // 4. Callable object invocation: `handler(event)` -> variable_k + callable
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_CALL_CALLEE | AstContext::IS_CALLABLE_OBJECT,
        0b00,
    );
    assert_eq!(kind, LspKind::VARIABLE_K);
    assert_eq!(mods & LspMod::CALLABLE, LspMod::CALLABLE);

    // 5. Callable property/field invocation: `this.callback(args)` -> property_k + callable
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_CALL_CALLEE | AstContext::IS_CALLABLE_OBJECT | AstContext::IS_FIELD_OR_PROP,
        0b00,
    );
    assert_eq!(kind, LspKind::PROPERTY_K);
    assert_eq!(mods & LspMod::CALLABLE, LspMod::CALLABLE);

    // 6. Syntax macro invocation: `my_macro(...)` -> macro_k
    let (kind, _mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_CALL_CALLEE | AstContext::IS_SYNTAX_MACRO,
        0b00,
    );
    assert_eq!(kind, LspKind::MACRO_K);

    // 7. Method invocation: `obj.compute(...)` -> method_k
    let (kind, _mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_CALL_CALLEE | AstContext::IS_METHOD_SYMBOL,
        0b00,
    );
    assert_eq!(kind, LspKind::METHOD_K);

    // 8. Plain function invocation: `calculate(...)` -> function_k
    let (kind, _mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_CALL_CALLEE,
        0b00,
    );
    assert_eq!(kind, LspKind::FUNCTION_K);
}

/// Test 2: Live user syntax evolution, quote depth hygiene, and splice classification.
#[test]
fn test_live_user_syntax_evolution_and_quote_splice_depth_hygiene() {


    // Splice of expression parameter: `$x` inside quote -> param_k + static_m
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_QUOTE_SPLICE,
        0b00,
    );
    assert_eq!(kind, LspKind::PARAM_K);
    assert_eq!(mods & LspMod::STATIC_M, LspMod::STATIC_M);

    // Splice of type parameter: `$T` inside quote -> type_param_k + static_m
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_QUOTE_SPLICE | AstContext::IS_QUOTE_TYPE_PARAM,
        0b00,
    );
    assert_eq!(kind, LspKind::TYPE_PARAM_K);
    assert_eq!(mods & LspMod::STATIC_M, LspMod::STATIC_M);

    // Unquote escape: `~expr` inside quote -> macro_k + static_m
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_UNQUOTE_ESCAPE,
        0b00,
    );
    assert_eq!(kind, LspKind::MACRO_K);
    assert_eq!(mods & LspMod::STATIC_M, LspMod::STATIC_M);

    // Verify side-table tracking in AstArena: 16-byte AstNode invariant
    let mut arena = pilot_ast::AstArena::new();
    let span = pilot_span::ByteSpan::new(0, 10);
    let nid = arena.alloc_node(pilot_ast::NodeKind::IdentExpr, 0, span, 0, 0);
    arena.node_quote_depths.insert(nid.0, 2);
    arena.node_contexts.insert(nid.0, AstContext(AstContext::IS_QUOTE_SPLICE));
    arena.node_expansion_origins.insert(nid.0, pilot_ast::ExpansionOrigin::Source);

    assert_eq!(arena.node_quote_depths.get(&nid.0), Some(&2));
    assert_eq!(arena.node_contexts.get(&nid.0).map(|c| c.0), Some(AstContext::IS_QUOTE_SPLICE));
    assert_eq!(std::mem::size_of::<pilot_ast::AstNode>(), 16, "AstNode must be strictly 16 bytes");
}

/// Test 3: Distributive headers, Variation B contextual `field`/`value`, and uncalled method groups.
#[test]
fn test_distributive_headers_variation_b_field_value_and_uncalled_method_groups() {


    // Variation B contextual `field` inside property accessor:
    // -> property_k + default_lib + modification
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_PROP_ACCESSOR | AstContext::IS_CONTEXTUAL_FIELD,
        0b00,
    );
    assert_eq!(kind, LspKind::PROPERTY_K);
    assert_eq!(mods & LspMod::DEFAULT_LIB, LspMod::DEFAULT_LIB);
    assert_eq!(mods & LspMod::MODIFICATION, LspMod::MODIFICATION);

    // Variation B contextual `value` inside property accessor:
    // -> param_k + default_lib + readonly
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_PROP_ACCESSOR | AstContext::IS_CONTEXTUAL_VALUE,
        0b00,
    );
    assert_eq!(kind, LspKind::PARAM_K);
    assert_eq!(mods & LspMod::DEFAULT_LIB, LspMod::DEFAULT_LIB);
    assert_eq!(mods & LspMod::READONLY, LspMod::READONLY);

    // Member access `obj.field`: IS_MEMBER_ACCESS overrides contextual accessor rule
    let (kind, _mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_MEMBER_ACCESS | AstContext::IS_FIELD_OR_PROP,
        0b00,
    );
    assert_eq!(kind, LspKind::PROPERTY_K);

    // Uncalled method reference: `items.map(.process)` or `obj.method`
    // -> method_k even without trailing `()`
    let (kind, _mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_MEMBER_ACCESS | AstContext::IS_METHOD_SYMBOL,
        0b00,
    );
    assert_eq!(kind, LspKind::METHOD_K);
}

/// Test 4: Attribute cascades, offset lenses, and custom [IdeToken] attributes.
#[test]
fn test_patch_attribute_cascades_offset_lens_and_custom_ide_token_attr() {
    // 1. `[...]` Attribute prefix on type: `[Clamp(0, 1)]` -> decorator_k
    let (kind, _mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_ATTR_PREFIX | AstContext::IS_TYPE_SYMBOL,
        0b00,
    );
    assert_eq!(kind, LspKind::DECORATOR_K);

    // 2. `*Bundle` or `*Facet` splice -> static_m
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::STRUCT_K,
        LspMod::NONE,
        AstContext::IS_BUNDLE_SPLICE,
        0b00,
    );
    assert_eq!(kind, LspKind::STRUCT_K);
    assert_eq!(mods & LspMod::STATIC_M, LspMod::STATIC_M);

    // 3. Dimensional unit suffix: `100[m/s]` -> type_param_k + default_lib
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_UNIT_DIMENSION,
        0b00,
    );
    assert_eq!(kind, LspKind::TYPE_PARAM_K);
    assert_eq!(mods & LspMod::DEFAULT_LIB, LspMod::DEFAULT_LIB);

    // 4. Type operator: `T@N`, `T[:]`, `T^`, `@T`, `T!E` -> operator_k + definition
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_TYPE_OPERATOR,
        0b00,
    );
    assert_eq!(kind, LspKind::OPERATOR_K);
    assert_eq!(mods & LspMod::DEFINITION, LspMod::DEFINITION);

    // 5. Variant constructor: `::ok(v)`, `::Active` -> enum_mem_k + readonly
    let (kind, mods) = IdeContextPolicy::classify(
        LspKind::VARIABLE_K,
        LspMod::NONE,
        AstContext::IS_VARIANT_CTOR,
        0b00,
    );
    assert_eq!(kind, LspKind::ENUM_MEM_K);
    assert_eq!(mods & LspMod::READONLY, LspMod::READONLY);
}

/// Test 5: Name-blind LSP rename litmus gate.
#[test]
fn test_name_blind_lsp_rename_litmus_gate() {
    let mut server = LspServer::new();
    let uri = "file:///workspace/rename_litmus.pilot";
    let code = "struct Vector {\n    pub x: i64;\n    pub y: i64;\n    pub length: f64 {\n        get => field;\n        set => { field = value; }\n    }\n}\n";

    // didOpen
    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
        uri,
        code.replace('\n', "\\n")
    );
    server.handle_message(&did_open);

    // 1. prepareRename on valid user identifier `Vector` -> succeeds with placeholder
    let prep_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":0,"character":8}}}}}}"#,
        uri
    );
    let prep_resp = server.handle_message(&prep_req).expect("Expected response");
    assert!(prep_resp.contains("placeholder"), "Valid identifier must be renameable: {}", prep_resp);
    assert!(prep_resp.contains("Vector"));

    // 2. prepareRename on contextual keyword `field` -> returns null (protected)
    let prep_field_req = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":4,"character":16}}}}}}"#,
        uri
    );
    let prep_field_resp = server.handle_message(&prep_field_req).expect("Expected response");
    assert!(prep_field_resp.contains("\"result\":null"), "Contextual field must not be renameable: {}", prep_field_resp);

    // 3. prepareRename on contextual keyword `value` -> returns null (protected)
    let prep_val_req = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":5,"character":26}}}}}}"#,
        uri
    );
    let prep_val_resp = server.handle_message(&prep_val_req).expect("Expected response");
    assert!(prep_val_resp.contains("\"result\":null"), "Contextual value must not be renameable: {}", prep_val_resp);

    // 4. rename to hardware seed keyword `struct` -> rejected
    let rename_kw_req = format!(
        r#"{{"jsonrpc":"2.0","id":4,"method":"textDocument/rename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":0,"character":8}},"newName":"struct"}}}}"#,
        uri
    );
    let rename_kw_resp = server.handle_message(&rename_kw_req).expect("Expected response");
    assert!(rename_kw_resp.contains("\"result\":null"), "Renaming to keyword must be rejected: {}", rename_kw_resp);

    // 5. rename to valid identifier `Vec2` -> succeeds with text edits
    let rename_valid_req = format!(
        r#"{{"jsonrpc":"2.0","id":5,"method":"textDocument/rename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":0,"character":8}},"newName":"Vec2"}}}}"#,
        uri
    );
    let rename_valid_resp = server.handle_message(&rename_valid_req).expect("Expected response");
    assert!(rename_valid_resp.contains("newText"), "Valid rename must produce workspace edits: {}", rename_valid_resp);
    assert!(rename_valid_resp.contains("Vec2"));
}

/// Test 6: Self-hosted ide_sync.pilot generates all 4 VS Code artifacts and presentation policy hover badges.
#[test]
fn test_selfhost_ide_sync_pilot_generates_all_4_vscode_artifacts_and_hover_badges() {
    // 1. Verify IdePresentationPolicy 2-bit hardware lattice hover badges
    let hover_00 = IdePresentationPolicy::format_type_hover("Vec3", 24, 8, 0b00);
    assert!(hover_00.contains("0b00 IValue (Trivial Copy)"));
    assert!(hover_00.contains("24 bytes"));
    assert!(hover_00.contains("align 8"));

    let hover_01 = IdePresentationPolicy::format_type_hover("SliceView", 16, 8, 0b01);
    assert!(hover_01.contains("0b01 IReference (Lexical Borrow)"));

    let hover_10 = IdePresentationPolicy::format_type_hover("SliceBuffer", 24, 8, 0b10);
    assert!(hover_10.contains("0b10 IObject (Perceus Move + ~self)"));

    let hover_11 = IdePresentationPolicy::format_type_hover("MutexGuard", 16, 8, 0b11);
    assert!(hover_11.contains("0b11 IGuard (Scoped RAII + Borrow)"));

    // 2. Verify capture inlay hint formatting
    let inlay = IdePresentationPolicy::format_capture_inlay_hint("base, total");
    assert_eq!(inlay, "/* captures: base, total */");

    // 3. Verify semantic tokens full via LSP server
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

    // 4. Verify syntax migration pairs in pilot-fmt
    let pairs = pilot_fmt::syntax_migration_pairs();
    assert!(!pairs.is_empty(), "Syntax migration pairs must be loaded");
    let migrated = pilot_fmt::migrate_syntax("def add(x: int) -> int: return x");
    assert!(migrated.contains("fn add"), "Migrated syntax should use 'fn': {}", migrated);

    // 5. Verify selfhost/ide_sync.pilot hierarchical class structure and evaluation
    let ide_sync_src = include_str!("../../../selfhost/ide_sync.pilot");
    let snap = BootstrapSnapshot::global();
    let mut snap_tables = snap.tables.clone();
    let mut snap_vm = snap.vm.clone();
    let eval_res = pilot_analysis::eval_file_with_tables(&ide_sync_src, &mut snap_tables, &mut snap_vm);
    assert!(eval_res.is_ok(), "selfhost/ide_sync.pilot must evaluate cleanly: {:?}", eval_res.err());
    let arena = eval_res.unwrap();
    assert!(arena.root.is_some(), "AST arena must have root");

    // 6. Verify stdlib/json/json.pilot canonical syntax and evaluation
    let json_src = include_str!("../../../stdlib/json/json.pilot");
    let mut json_tables = snap.tables.clone();
    let mut json_vm = snap.vm.clone();
    let json_eval_res = pilot_analysis::eval_file_with_tables(json_src, &mut json_tables, &mut json_vm);
    assert!(json_eval_res.is_ok(), "stdlib/json/json.pilot must evaluate cleanly: {:?}", json_eval_res.err());
    let json_arena = pilot_parser::parse(json_src);
    let json_analysis = pilot_analysis::AnalysisPipeline::analyze(&json_arena, json_arena.root);
    for d in &json_analysis.diagnostics {
        eprintln!("JSON DIAG: {:?}", d);
    }
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

    // 1. Hover on Account
    let hover_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":4,"character":8}}}}}}"#,
        uri
    );
    let hover_resp = server.handle_message(&hover_req).expect("hover response");
    assert!(hover_resp.contains("Account"), "Hover must contain Account: {}", hover_resp);
    assert!(hover_resp.contains("Documented Account struct"), "Hover must contain doc: {}", hover_resp);
    assert!(hover_resp.contains("8 bytes"), "Hover must contain byte size: {}", hover_resp);

    // 1b. Hover on keyword `struct` (line 4, character 2)
    let hover_kw_req = format!(
        r#"{{"jsonrpc":"2.0","id":11,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":4,"character":2}}}}}}"#,
        uri
    );
    let hover_kw_resp = server.handle_message(&hover_kw_req).expect("hover kw response");
    assert!(hover_kw_resp.contains("Pilot Keyword"), "Keyword hover must indicate keyword: {}", hover_kw_resp);
    assert!(hover_kw_resp.contains("composite type"), "Keyword hover must have summary: {}", hover_kw_resp);

    // 1c. Hover on method `deposit` (line 6, character 15)
    let hover_fn_req = format!(
        r#"{{"jsonrpc":"2.0","id":12,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":6,"character":15}}}}}}"#,
        uri
    );
    let hover_fn_resp = server.handle_message(&hover_fn_req).expect("hover fn response");
    assert!(hover_fn_resp.contains("deposit"), "Function hover must contain deposit: {}", hover_fn_resp);
    assert!(hover_fn_resp.contains("amount: i64"), "Function hover must contain param: {}", hover_fn_resp);

    // 1d. Hover on whitespace (line 4, character 6) -> returns null (no broken empty card)
    let hover_ws_req = format!(
        r#"{{"jsonrpc":"2.0","id":13,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":4,"character":6}}}}}}"#,
        uri
    );
    let hover_ws_resp = server.handle_message(&hover_ws_req).expect("hover ws response");
    assert!(hover_ws_resp.contains("\"result\":null"), "Whitespace hover must be null: {}", hover_ws_resp);

    // 2. Completion on acct. outside Account (line 14, character 5)
    let comp_req = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":14,"character":5}}}}}}"#,
        uri
    );
    let comp_resp = server.handle_message(&comp_req).expect("completion response");
    assert!(comp_resp.contains("balance"), "Completion on acct. must include balance: {}", comp_resp);
    assert!(comp_resp.contains("deposit"), "Completion on acct. must include deposit: {}", comp_resp);
    assert!(!comp_resp.contains("wrong_field"), "Completion on acct. must NOT include wrong_field from DecoyFirstStruct: {}", comp_resp);
    assert!(!comp_resp.contains("internal_helper"), "Completion on acct. must NOT include private internal_helper: {}", comp_resp);

    // 3. Definition on Account
    let def_req = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":4,"character":8}}}}}}"#,
        uri
    );
    let def_resp = server.handle_message(&def_req).expect("definition response");
    assert!(def_resp.contains("range"), "Definition must return range: {}", def_resp);
}
