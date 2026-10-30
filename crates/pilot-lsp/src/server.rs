//! Milestone 32.3: Zero-GC Standalone Native Language Server (`pilot-lsp`).

use std::collections::HashMap;
use pilot_db::{Database, FileId};
use pilot_fmt::{format_source, FormatOptions};
use pilot_incremental::TextEdit;
use pilot_span::LineIndex;
use pilot_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};

use crate::json::{parse_json, JsonValue};

pub struct LspServer {
    db: Database,
    uri_to_file_id: HashMap<String, FileId>,
    file_id_to_uri: HashMap<FileId, String>,
    line_indices: HashMap<FileId, LineIndex>,
    next_file_id: u32,
    pub shutdown_requested: bool,
    pub pending_notifications: Vec<String>,
}

impl LspServer {
    pub fn new() -> Self {
        Self {
            db: Database::new(),
            uri_to_file_id: HashMap::new(),
            file_id_to_uri: HashMap::new(),
            line_indices: HashMap::new(),
            next_file_id: 1,
            shutdown_requested: false,
            pending_notifications: Vec::new(),
        }
    }

    pub fn take_pending_notifications(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_notifications)
    }

    fn get_or_create_file_id(&mut self, uri: &str) -> FileId {
        if let Some(&id) = self.uri_to_file_id.get(uri) {
            return id;
        }
        let id = FileId(self.next_file_id);
        self.next_file_id += 1;
        self.uri_to_file_id.insert(uri.to_string(), id);
        self.file_id_to_uri.insert(id, uri.to_string());
        id
    }

    /// Dispatches an incoming JSON-RPC string message and returns an optional response string.
    pub fn handle_message(&mut self, raw_json: &str) -> Option<String> {
        let msg = match parse_json(raw_json) {
            Ok(val) => val,
            Err(_) => return None,
        };

        let method = msg.get("method").and_then(|v| v.as_str());
        let id = msg.get("id").cloned();
        let params = msg.get("params");

        match method {
            Some("initialize") => {
                let resp = self.handle_initialize(id.as_ref());
                Some(resp.serialize())
            }
            Some("initialized") => None,
            Some("shutdown") => {
                self.shutdown_requested = true;
                let resp = JsonValue::Object(vec![
                    ("jsonrpc".to_string(), JsonValue::String("2.0".to_string())),
                    ("id".to_string(), id.unwrap_or(JsonValue::Null)),
                    ("result".to_string(), JsonValue::Null),
                ]);
                Some(resp.serialize())
            }
            Some("exit") => None,
            Some("textDocument/didOpen") => {
                if let Some(p) = params {
                    self.handle_did_open(p);
                }
                None
            }
            Some("textDocument/didChange") => {
                if let Some(p) = params {
                    self.handle_did_change(p);
                }
                None
            }
            Some("textDocument/hover") => {
                if let Some(p) = params {
                    let res = self.handle_hover(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Null).serialize())
                }
            }
            Some("textDocument/completion") => {
                if let Some(p) = params {
                    let res = self.handle_completion(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Array(Vec::new())).serialize())
                }
            }
            Some("textDocument/definition") => {
                if let Some(p) = params {
                    let res = self.handle_definition(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Null).serialize())
                }
            }
            Some("textDocument/documentSymbol") => {
                if let Some(p) = params {
                    let res = self.handle_document_symbol(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Array(Vec::new())).serialize())
                }
            }
            Some("textDocument/formatting") => {
                if let Some(p) = params {
                    let res = self.handle_formatting(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Array(Vec::new())).serialize())
                }
            }
            Some("textDocument/inlayHint") => {
                if let Some(p) = params {
                    let res = self.handle_inlay_hint(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Array(Vec::new())).serialize())
                }
            }
            Some("textDocument/signatureHelp") => {
                if let Some(p) = params {
                    let res = self.handle_signature_help(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Null).serialize())
                }
            }
            Some("textDocument/codeAction") => {
                if let Some(p) = params {
                    let res = self.handle_code_action(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Array(Vec::new())).serialize())
                }
            }
            Some("textDocument/semanticTokens/full") => {
                if let Some(p) = params {
                    let res = self.handle_semantic_tokens_full(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Object(vec![("data".to_string(), JsonValue::Array(Vec::new()))])).serialize())
                }
            }
            _ => {
                // If it was a request with an id, return MethodNotFound
                if let Some(req_id) = id {
                    let err = JsonValue::Object(vec![
                        ("jsonrpc".to_string(), JsonValue::String("2.0".to_string())),
                        ("id".to_string(), req_id),
                        ("error".to_string(), JsonValue::Object(vec![
                            ("code".to_string(), JsonValue::Number(-32601.0)),
                            ("message".to_string(), JsonValue::String("Method not found".to_string())),
                        ])),
                    ]);
                    Some(err.serialize())
                } else {
                    None
                }
            }
        }
    }

    pub const TOKEN_TYPES: &'static [&'static str] = &[
        "namespace",     // 0
        "type",          // 1
        "class",         // 2
        "enum",          // 3
        "interface",     // 4
        "struct",        // 5
        "typeParameter", // 6
        "parameter",     // 7
        "variable",      // 8
        "property",      // 9
        "enumMember",    // 10
        "event",         // 11
        "function",      // 12
        "method",        // 13
        "macro",         // 14
        "keyword",       // 15
        "modifier",      // 16
        "comment",       // 17
        "string",        // 18
        "number",        // 19
        "regexp",        // 20
        "operator",      // 21
        "decorator",     // 22
    ];

    pub const TOKEN_MODIFIERS: &'static [&'static str] = &[
        "declaration",    // 0
        "definition",     // 1
        "readonly",       // 2
        "static",         // 3
        "deprecated",     // 4
        "abstract",       // 5
        "async",          // 6
        "modification",   // 7
        "documentation",  // 8
        "defaultLibrary", // 9
    ];

    fn handle_initialize(&self, id: Option<&JsonValue>) -> JsonValue {
        let token_types: Vec<JsonValue> = Self::TOKEN_TYPES
            .iter()
            .map(|&s| JsonValue::String(s.to_string()))
            .collect();

        let token_modifiers: Vec<JsonValue> = Self::TOKEN_MODIFIERS
            .iter()
            .map(|&s| JsonValue::String(s.to_string()))
            .collect();

        let capabilities = JsonValue::Object(vec![
            ("textDocumentSync".to_string(), JsonValue::Number(2.0)), // Incremental
            ("hoverProvider".to_string(), JsonValue::Bool(true)),
            ("completionProvider".to_string(), JsonValue::Object(vec![
                ("resolveProvider".to_string(), JsonValue::Bool(false)),
                ("triggerCharacters".to_string(), JsonValue::Array(vec![
                    JsonValue::String(".".to_string()),
                    JsonValue::String(":".to_string()),
                ])),
            ])),
            ("signatureHelpProvider".to_string(), JsonValue::Object(vec![
                ("triggerCharacters".to_string(), JsonValue::Array(vec![
                    JsonValue::String("(".to_string()),
                    JsonValue::String(",".to_string()),
                ])),
            ])),
            ("definitionProvider".to_string(), JsonValue::Bool(true)),
            ("documentSymbolProvider".to_string(), JsonValue::Bool(true)),
            ("documentFormattingProvider".to_string(), JsonValue::Bool(true)),
            ("inlayHintProvider".to_string(), JsonValue::Bool(true)),
            ("codeActionProvider".to_string(), JsonValue::Bool(true)),
            ("semanticTokensProvider".to_string(), JsonValue::Object(vec![
                ("legend".to_string(), JsonValue::Object(vec![
                    ("tokenTypes".to_string(), JsonValue::Array(token_types)),
                    ("tokenModifiers".to_string(), JsonValue::Array(token_modifiers)),
                ])),
                ("full".to_string(), JsonValue::Bool(true)),
            ])),
        ]);

        JsonValue::Object(vec![
            ("jsonrpc".to_string(), JsonValue::String("2.0".to_string())),
            ("id".to_string(), id.cloned().unwrap_or(JsonValue::Null)),
            ("result".to_string(), JsonValue::Object(vec![
                ("capabilities".to_string(), capabilities),
            ])),
        ])
    }

    fn handle_did_open(&mut self, params: &JsonValue) {
        let doc = match params.get("textDocument") {
            Some(d) => d,
            None => return,
        };
        let uri = match doc.get("uri").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => return,
        };
        let text = match doc.get("text").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return,
        };

        let file_id = self.get_or_create_file_id(uri);
        self.line_indices.insert(file_id, LineIndex::new(text.as_bytes()));
        self.db.set_source(file_id, text);
        self.emit_diagnostics_for_file(file_id, uri);
    }

    fn handle_did_change(&mut self, params: &JsonValue) {
        let doc = match params.get("textDocument") {
            Some(d) => d,
            None => return,
        };
        let uri = match doc.get("uri").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => return,
        };
        let file_id = self.get_or_create_file_id(uri);

        let changes = match params.get("contentChanges").and_then(|v| v.as_array()) {
            Some(c) => c,
            None => return,
        };

        for change in changes {
            let replacement = change.get("text").and_then(|v| v.as_str()).unwrap_or_default();
            if let Some(range) = change.get("range") {
                // Incremental edit
                let line_index = self.line_indices.get(&file_id);
                let (start_byte, end_byte) = if let Some(li) = line_index {
                    let start_line = range.get("start").and_then(|s| s.get("line")).and_then(|l| l.as_u64()).unwrap_or(0) as u32;
                    let start_col = range.get("start").and_then(|s| s.get("character")).and_then(|c| c.as_u64()).unwrap_or(0) as u32;
                    let end_line = range.get("end").and_then(|e| e.get("line")).and_then(|l| l.as_u64()).unwrap_or(0) as u32;
                    let end_col = range.get("end").and_then(|e| e.get("character")).and_then(|c| c.as_u64()).unwrap_or(0) as u32;

                    let s_offset = li.line_col_to_offset(start_line + 1, start_col + 1).unwrap_or(0);
                    let e_offset = li.line_col_to_offset(end_line + 1, end_col + 1).unwrap_or(s_offset);
                    (s_offset, e_offset)
                } else {
                    (0, 0)
                };

                let edit = TextEdit::new(start_byte, end_byte, replacement);
                self.db.apply_edit(file_id, &edit);

                if let Some(new_src) = self.db.source_text(file_id) {
                    self.line_indices.insert(file_id, LineIndex::new(new_src.as_bytes()));
                }
            } else {
                // Full sync replacement
                self.db.set_source(file_id, replacement);
                self.line_indices.insert(file_id, LineIndex::new(replacement.as_bytes()));
            }
        }
        self.emit_diagnostics_for_file(file_id, uri);
    }

    fn handle_hover(&self, params: &JsonValue) -> JsonValue {
        let (file_id, byte_offset) = match self.get_file_and_offset(params) {
            Some(res) => res,
            None => return JsonValue::Null,
        };

        let src = match self.db.source_text(file_id) {
            Some(s) => s,
            None => return JsonValue::Null,
        };

        let green = match self.db.green_cst(file_id) {
            Some(g) => g,
            None => return JsonValue::Null,
        };

        let root = SyntaxNode::new_root(green);
        let mut token = root.find_token_at_offset(byte_offset);
        if token.as_ref().map(|t| t.kind().is_trivia()).unwrap_or(true) && byte_offset > 0 {
            if let Some(prev) = root.find_token_at_offset(byte_offset - 1) {
                if !prev.kind().is_trivia() {
                    token = Some(prev);
                }
            }
        }

        let txt = match token {
            Some(ref t) => t.text(),
            None => return JsonValue::Null,
        };

        // 1. Built-in standard library functions
        if let Some((sig, summary)) = get_builtin_doc(txt) {
            let md = format!("```pilot\n{}\n```\n\n{}", sig, summary);
            return make_hover_card(&md);
        }

        // 2. Polyglot engine blocks
        if let Some(doc) = get_polyglot_doc(txt) {
            let md = format!("**Pilot Polyglot Block**: `{}`\n\n{}", txt, doc);
            return make_hover_card(&md);
        }

        // 3. Types and primitives
        if let Some(doc) = get_type_doc(txt) {
            let md = format!("**Pilot Type**: `{}`\n\n{}", txt, doc);
            return make_hover_card(&md);
        }

        // 4. Pilot Keywords
        if let Some((domain, summary)) = get_keyword_doc(txt) {
            let md = format!("**Pilot Keyword**: `{}` ({})\n\n{}", txt, domain, summary);
            return make_hover_card(&md);
        }

        // 5. AST-based Type Analysis (Functions, Parameters, Variables, Classes, Structs)
        let arena = pilot_parser::parse(&src);
        let root_id = pilot_ast::NodeId(if arena.nodes.is_empty() { 0 } else { (arena.nodes.len() - 1) as u32 });
        let analysis = pilot_analysis::AnalysisPipeline::analyze(&arena, root_id);

        // 5a. Functions in file (definitions & call-sites)
        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == pilot_ast::NodeKind::FnDecl {
                let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                if name == txt {
                    let fn_start = node.rhs;
                    let fn_len = node.flags as usize;
                    let mut param_strs = Vec::new();
                    if fn_len > 0 {
                        let fn_children = arena.children_slice(fn_start, fn_len as u32);
                        for &p_id in &fn_children[1..] {
                            let p_node = *arena.node(pilot_ast::NodeId(p_id));
                            let p_name = arena.interner.resolve(pilot_intern::SymbolId(p_node.lhs));
                            let p_ty = if p_node.rhs != 0 {
                                analysis.inferred_types.get(&p_id).copied().unwrap_or(pilot_analysis::TypeId::ANY)
                            } else {
                                pilot_analysis::TypeId::ANY
                            };
                            param_strs.push(format!("{}: {}", p_name, analysis.types.display(p_ty)));
                        }
                    }
                    let ret_ty = analysis.inferred_types.get(&(idx as u32)).copied().unwrap_or(pilot_analysis::TypeId::ANY);
                    let ret_str = if let pilot_analysis::TypeKind::Function { ret, .. } = analysis.types.get(ret_ty) {
                        analysis.types.display(*ret)
                    } else {
                        analysis.types.display(ret_ty)
                    };
                    let sig = format!("def {}({}) -> {}", name, param_strs.join(", "), ret_str);
                    let md = format!("```pilot\n{}\n```\n\n*Pilot Function Declaration*", sig);
                    return make_hover_card(&md);
                }
            }
        }

        // 5b. Parameters
        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == pilot_ast::NodeKind::ParamDecl {
                let p_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                if p_name == txt {
                    let ty_id = analysis.inferred_types.get(&(idx as u32)).copied().unwrap_or(pilot_analysis::TypeId::ANY);
                    let ty_str = analysis.types.display(ty_id);
                    let md = format!("```pilot\n(parameter) {}: {}\n```", p_name, ty_str);
                    return make_hover_card(&md);
                }
            }
        }

        // 5c. Variables (let, var, const)
        for (idx, node) in arena.nodes.iter().enumerate() {
            let nk = node.node_kind();
            if matches!(nk, pilot_ast::NodeKind::LetStmt | pilot_ast::NodeKind::VarStmt | pilot_ast::NodeKind::ConstDecl) {
                let var_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                if var_name == txt {
                    let ty_id = analysis.inferred_types.get(&(idx as u32)).copied().unwrap_or(pilot_analysis::TypeId::ANY);
                    let ty_str = analysis.types.display(ty_id);
                    let kw = match nk {
                        pilot_ast::NodeKind::LetStmt => "let",
                        pilot_ast::NodeKind::VarStmt => "var",
                        pilot_ast::NodeKind::ConstDecl => "const",
                        _ => "let",
                    };
                    let md = format!("```pilot\n{} {}: {}\n```\n\n*Pilot Variable*", kw, var_name, ty_str);
                    return make_hover_card(&md);
                }
            }
        }

        // 6. Top-level definitions in ItemTree fallback
        if let Some(item_tree) = self.db.module_item_tree(file_id) {
            for item in &item_tree.items {
                if item.name == txt {
                    let md = format!("**Pilot Definition**: `{}`\n```pilot\n{}\n```", item.name, item.signature_text);
                    return make_hover_card(&md);
                }
            }
        }

        JsonValue::Null
    }

    fn handle_signature_help(&self, params: &JsonValue) -> JsonValue {
        let (file_id, byte_offset) = match self.get_file_and_offset(params) {
            Some(res) => res,
            None => return JsonValue::Null,
        };

        let src = match self.db.source_text(file_id) {
            Some(s) => s,
            None => return JsonValue::Null,
        };

        let offset = (byte_offset as usize).min(src.len());
        let prefix = &src[..offset];

        let mut depth = 0i32;
        let mut open_paren_idx = None;
        let mut comma_count = 0u32;

        let bytes = prefix.as_bytes();
        for i in (0..bytes.len()).rev() {
            let b = bytes[i];
            if b == b')' {
                depth += 1;
            } else if b == b'(' {
                if depth == 0 {
                    open_paren_idx = Some(i);
                    break;
                }
                depth -= 1;
            } else if b == b',' && depth == 0 {
                comma_count += 1;
            }
        }

        let paren_idx = match open_paren_idx {
            Some(idx) => idx,
            None => return JsonValue::Null,
        };

        let before_paren = prefix[..paren_idx].trim_end();
        let fn_name_start = before_paren.rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let fn_name = &before_paren[fn_name_start..];
        if fn_name.is_empty() {
            return JsonValue::Null;
        }

        // 1. Built-in functions
        if let Some((sig, summary)) = get_builtin_doc(fn_name) {
            let param_list = extract_params_from_sig(sig);
            return make_signature_help_resp(sig, summary, &param_list, comma_count);
        }

        // 2. AST Functions
        let arena = pilot_parser::parse(&src);
        let root_id = pilot_ast::NodeId(if arena.nodes.is_empty() { 0 } else { (arena.nodes.len() - 1) as u32 });
        let analysis = pilot_analysis::AnalysisPipeline::analyze(&arena, root_id);

        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == pilot_ast::NodeKind::FnDecl {
                let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                if name == fn_name {
                    let fn_start = node.rhs;
                    let fn_len = node.flags as usize;
                    let mut param_strs = Vec::new();
                    if fn_len > 0 {
                        let fn_children = arena.children_slice(fn_start, fn_len as u32);
                        for &p_id in &fn_children[1..] {
                            let p_node = *arena.node(pilot_ast::NodeId(p_id));
                            let p_name = arena.interner.resolve(pilot_intern::SymbolId(p_node.lhs));
                            let p_ty = if p_node.rhs != 0 {
                                analysis.inferred_types.get(&p_id).copied().unwrap_or(pilot_analysis::TypeId::ANY)
                            } else {
                                pilot_analysis::TypeId::ANY
                            };
                            param_strs.push(format!("{}: {}", p_name, analysis.types.display(p_ty)));
                        }
                    }
                    let ret_ty = analysis.inferred_types.get(&(idx as u32)).copied().unwrap_or(pilot_analysis::TypeId::ANY);
                    let ret_str = if let pilot_analysis::TypeKind::Function { ret, .. } = analysis.types.get(ret_ty) {
                        analysis.types.display(*ret)
                    } else {
                        analysis.types.display(ret_ty)
                    };
                    let sig = format!("{}({}) -> {}", name, param_strs.join(", "), ret_str);
                    return make_signature_help_resp(&sig, "Pilot Function Declaration", &param_strs, comma_count);
                }
            }
        }

        // 3. ItemTree fallback
        if let Some(item_tree) = self.db.module_item_tree(file_id) {
            for item in &item_tree.items {
                if item.name == fn_name {
                    let param_list = extract_params_from_sig(&item.signature_text);
                    return make_signature_help_resp(&item.signature_text, "Pilot Definition", &param_list, comma_count);
                }
            }
        }

        JsonValue::Null
    }

    fn handle_completion(&self, params: &JsonValue) -> JsonValue {
        let mut items = Vec::new();

        // 1. Built-in functions
        for (name, sig, summary) in BUILTIN_DOCS.iter() {
            items.push(JsonValue::Object(vec![
                ("label".to_string(), JsonValue::String(name.to_string())),
                ("kind".to_string(), JsonValue::Number(3.0)), // Function
                ("detail".to_string(), JsonValue::String(sig.to_string())),
                ("documentation".to_string(), JsonValue::String(summary.to_string())),
            ]));
        }

        // 2. Keyword completions
        for (kw, (domain, summary)) in KEYWORD_DOCS.iter() {
            items.push(JsonValue::Object(vec![
                ("label".to_string(), JsonValue::String(kw.to_string())),
                ("kind".to_string(), JsonValue::Number(14.0)), // Keyword
                ("detail".to_string(), JsonValue::String(domain.to_string())),
                ("documentation".to_string(), JsonValue::String(summary.to_string())),
            ]));
        }

        // 3. Polyglot engines
        for (lang, desc) in POLYGLOT_DOCS.iter() {
            items.push(JsonValue::Object(vec![
                ("label".to_string(), JsonValue::String(lang.to_string())),
                ("kind".to_string(), JsonValue::Number(14.0)), // Keyword
                ("detail".to_string(), JsonValue::String("Polyglot Engine".to_string())),
                ("documentation".to_string(), JsonValue::String(desc.to_string())),
            ]));
        }

        // 4. Primitive types
        for (ty, desc) in TYPE_DOCS.iter() {
            items.push(JsonValue::Object(vec![
                ("label".to_string(), JsonValue::String(ty.to_string())),
                ("kind".to_string(), JsonValue::Number(7.0)), // Class / Interface / Type
                ("detail".to_string(), JsonValue::String("Pilot Type".to_string())),
                ("documentation".to_string(), JsonValue::String(desc.to_string())),
            ]));
        }

        // 5. Top-level module symbols & variables
        if let Some((file_id, _)) = self.get_file_and_offset(params) {
            if let Some(item_tree) = self.db.module_item_tree(file_id) {
                for item in &item_tree.items {
                    items.push(JsonValue::Object(vec![
                        ("label".to_string(), JsonValue::String(item.name.clone())),
                        ("kind".to_string(), JsonValue::Number(3.0)), // Function
                        ("detail".to_string(), JsonValue::String(item.signature_text.clone())),
                    ]));
                }
            }
            if let Some(src) = self.db.source_text(file_id) {
                let arena = pilot_parser::parse(&src);
                for node in &arena.nodes {
                    let nk = node.node_kind();
                    if matches!(nk, pilot_ast::NodeKind::LetStmt | pilot_ast::NodeKind::VarStmt) {
                        let var_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                        items.push(JsonValue::Object(vec![
                            ("label".to_string(), JsonValue::String(var_name.to_string())),
                            ("kind".to_string(), JsonValue::Number(6.0)), // Variable
                            ("detail".to_string(), JsonValue::String("Variable".to_string())),
                        ]));
                    }
                }
            }
        }

        JsonValue::Array(items)
    }

    fn handle_definition(&self, params: &JsonValue) -> JsonValue {
        let (file_id, byte_offset) = match self.get_file_and_offset(params) {
            Some(res) => res,
            None => return JsonValue::Null,
        };

        let green = match self.db.green_cst(file_id) {
            Some(g) => g,
            None => return JsonValue::Null,
        };

        let root = SyntaxNode::new_root(green);
        let token = match root.find_token_at_offset(byte_offset) {
            Some(t) => t,
            None => return JsonValue::Null,
        };

        let txt = token.text();
        let uri = self.file_id_to_uri.get(&file_id).cloned().unwrap_or_default();
        let line_index = self.line_indices.get(&file_id);

        if let Some(item_tree) = self.db.module_item_tree(file_id) {
            for item in &item_tree.items {
                if item.name == txt {
                    // Found item definition
                    let (line, col) = line_index.map(|li| li.offset_to_line_col(token.offset())).unwrap_or((1, 1));
                    let range = make_range(line - 1, col - 1, line - 1, col - 1 + txt.len() as u32);
                    return JsonValue::Object(vec![
                        ("uri".to_string(), JsonValue::String(uri)),
                        ("range".to_string(), range),
                    ]);
                }
            }
        }

        JsonValue::Null
    }

    fn handle_document_symbol(&self, params: &JsonValue) -> JsonValue {
        let uri = match params.get("textDocument").and_then(|d| d.get("uri")).and_then(|u| u.as_str()) {
            Some(u) => u,
            None => return JsonValue::Array(Vec::new()),
        };
        let file_id = match self.uri_to_file_id.get(uri) {
            Some(&id) => id,
            None => return JsonValue::Array(Vec::new()),
        };

        let mut symbols = Vec::new();
        if let Some(item_tree) = self.db.module_item_tree(file_id) {
            for item in &item_tree.items {
                let (line, col) = (0u32, 0u32);
                let range = make_range(line, col, line, col + item.name.len() as u32);

                symbols.push(JsonValue::Object(vec![
                    ("name".to_string(), JsonValue::String(item.name.clone())),
                    ("kind".to_string(), JsonValue::Number(12.0)), // Function
                    ("range".to_string(), range.clone()),
                    ("selectionRange".to_string(), range),
                ]));
            }
        }

        JsonValue::Array(symbols)
    }

    fn handle_formatting(&self, params: &JsonValue) -> JsonValue {
        let uri = match params.get("textDocument").and_then(|d| d.get("uri")).and_then(|u| u.as_str()) {
            Some(u) => u,
            None => return JsonValue::Array(Vec::new()),
        };
        let file_id = match self.uri_to_file_id.get(uri) {
            Some(&id) => id,
            None => return JsonValue::Array(Vec::new()),
        };

        let src = match self.db.source_text(file_id) {
            Some(s) => s,
            None => return JsonValue::Array(Vec::new()),
        };

        let options = FormatOptions::default();
        let res = format_source(&src, &options);

        let line_index = self.line_indices.get(&file_id);
        let end_line = line_index.map(|li| li.line_count() as u32).unwrap_or(1);
        let full_range = make_range(0, 0, end_line, 0);

        JsonValue::Array(vec![
            JsonValue::Object(vec![
                ("range".to_string(), full_range),
                ("newText".to_string(), JsonValue::String(res.formatted_code)),
            ]),
        ])
    }

    fn decode_uri_path(uri: &str) -> Option<String> {
        let stripped = uri.strip_prefix("file://")?;
        let mut path = String::new();
        let bytes = stripped.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let Ok(b) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..=i + 2]).unwrap_or(""), 16) {
                    path.push(b as char);
                    i += 3;
                    continue;
                }
            }
            path.push(bytes[i] as char);
            i += 1;
        }
        Some(path)
    }

    fn is_pilot_primitive_type(s: &str) -> bool {
        matches!(
            s,
            "int"
                | "i64"
                | "i32"
                | "i16"
                | "i8"
                | "uint"
                | "u64"
                | "u32"
                | "u16"
                | "u8"
                | "float"
                | "f64"
                | "f32"
                | "f16"
                | "f8"
                | "str"
                | "string"
                | "char"
                | "bool"
                | "void"
                | "any"
                | "symbol"
                | "list"
                | "map"
                | "dict"
                | "set"
                | "tuple"
                | "range"
                | "bytes"
                | "bytearray"
                | "dynamic"
                | "decimal"
                | "double"
                | "complex"
                | "frozenset"
                | "Table"
                | "Dataset"
                | "Relation"
                | "Channel"
                | "Chan"
                | "Result"
                | "Option"
                | "Span"
                | "Memory"
                | "SliceBuffer"
                | "SharedBuffer"
                | "RangeWindow"
                | "NativeArray"
                | "NativeMap"
                | "AstNode"
        )
    }

    fn handle_semantic_tokens_full(&mut self, params: &JsonValue) -> JsonValue {
        let uri = match params.get("textDocument").and_then(|d| d.get("uri")).and_then(|u| u.as_str()) {
            Some(u) => u,
            None => return JsonValue::Object(vec![("data".to_string(), JsonValue::Array(Vec::new()))]),
        };
        let file_id = match self.uri_to_file_id.get(uri).copied() {
            Some(id) => id,
            None => {
                if let Some(path) = Self::decode_uri_path(uri) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let id = self.get_or_create_file_id(uri);
                        self.line_indices.insert(id, LineIndex::new(content.as_bytes()));
                        self.db.set_source(id, &content);
                        id
                    } else {
                        return JsonValue::Object(vec![("data".to_string(), JsonValue::Array(Vec::new()))]);
                    }
                } else {
                    return JsonValue::Object(vec![("data".to_string(), JsonValue::Array(Vec::new()))]);
                }
            }
        };

        let green = match self.db.green_cst(file_id) {
            Some(g) => g,
            None => return JsonValue::Object(vec![("data".to_string(), JsonValue::Array(Vec::new()))]),
        };

        let line_index = match self.line_indices.get(&file_id) {
            Some(li) => li,
            None => return JsonValue::Object(vec![("data".to_string(), JsonValue::Array(Vec::new()))]),
        };

        let src = match self.db.source_text(file_id) {
            Some(s) => s,
            None => return JsonValue::Object(vec![("data".to_string(), JsonValue::Array(Vec::new()))]),
        };

        let root = SyntaxNode::new_root(green);
        let mut data = Vec::new();
        let mut prev_line = 0u32;
        let mut prev_col = 0u32;
        let mut prev_non_trivia_kind = SyntaxKind::Eof;

        for elem in root.descendants() {
            if let SyntaxElement::Token(token) = elem {
                let k = token.kind();
                if k == SyntaxKind::Whitespace || k == SyntaxKind::Newline {
                    continue;
                }

                let token_desc: Option<(u32, u32)> = match k {
                    // Keywords (15) and Modifiers (16)
                    k if (k as u16 >= SyntaxKind::KwLet as u16 && k as u16 <= SyntaxKind::KwDefault as u16) => {
                        match k {
                            SyntaxKind::KwPure
                            | SyntaxKind::KwComptime
                            | SyntaxKind::KwMut
                            | SyntaxKind::KwImmut
                            | SyntaxKind::KwRef
                            | SyntaxKind::KwDynamic
                            | SyntaxKind::KwAsync => Some((16, 0)), // modifier
                            _ => Some((15, 0)), // keyword
                        }
                    }

                    // Identifiers (function / method / type / class / struct / property / decorator / variable)
                    SyntaxKind::Ident => {
                        let text = token.text();
                        let end_offset = (token.offset() + token.text_len()) as usize;
                        let remainder = src.get(end_offset..).unwrap_or("").trim_start();

                        if pilot_parser::is_polyglot_lang(text) && (remainder.starts_with('{') || remainder.starts_with("```") || remainder.starts_with("\"\"\"")) {
                            Some((15, 0)) // keyword
                        } else if matches!(text, "public" | "private" | "protected" | "internal" | "static" | "partial" | "readonly" | "override" | "virtual" | "abstract" | "sealed") {
                            Some((16, 0)) // modifier
                        } else if prev_non_trivia_kind == SyntaxKind::At {
                            Some((22, 0)) // decorator
                        } else if prev_non_trivia_kind == SyntaxKind::KwClass || prev_non_trivia_kind == SyntaxKind::KwActor || prev_non_trivia_kind == SyntaxKind::KwState {
                            Some((2, 3)) // class (declaration + definition)
                        } else if prev_non_trivia_kind == SyntaxKind::KwStruct || prev_non_trivia_kind == SyntaxKind::KwRecord {
                            Some((5, 3)) // struct (declaration + definition)
                        } else if prev_non_trivia_kind == SyntaxKind::KwInterface || prev_non_trivia_kind == SyntaxKind::KwProtocol {
                            Some((4, 3)) // interface (declaration + definition)
                        } else if prev_non_trivia_kind == SyntaxKind::KwEnum {
                            Some((3, 3)) // enum (declaration + definition)
                        } else if prev_non_trivia_kind == SyntaxKind::KwType || prev_non_trivia_kind == SyntaxKind::KwExtend || prev_non_trivia_kind == SyntaxKind::KwExtension {
                            Some((1, 3)) // type (declaration + definition)
                        } else if prev_non_trivia_kind == SyntaxKind::KwProperty {
                            Some((9, 3)) // property (declaration + definition)
                        } else if prev_non_trivia_kind == SyntaxKind::KwMacro {
                            Some((14, 3)) // macro (declaration + definition)
                        } else if prev_non_trivia_kind == SyntaxKind::KwDef || prev_non_trivia_kind == SyntaxKind::KwFn {
                            Some((12, 3)) // function definition
                        } else if prev_non_trivia_kind == SyntaxKind::Dot || prev_non_trivia_kind == SyntaxKind::SafeNav || prev_non_trivia_kind == SyntaxKind::Cascade || prev_non_trivia_kind == SyntaxKind::DotDot {
                            if remainder.starts_with('(') {
                                Some((13, 0)) // method
                            } else {
                                Some((9, 0)) // property
                            }
                        } else if remainder.starts_with('(') {
                            if text.starts_with(char::is_uppercase) {
                                Some((2, 0)) // class constructor call
                            } else {
                                Some((12, 0)) // function call
                            }
                        } else if Self::is_pilot_primitive_type(text) {
                            Some((1, 512)) // type (defaultLibrary)
                        } else if text.starts_with(char::is_uppercase) {
                            if text.chars().all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit()) && text.len() > 1 {
                                Some((8, 4)) // constant (variable + readonly)
                            } else {
                                Some((1, 0)) // type
                            }
                        } else if text == "self" || text == "this" {
                            Some((8, 4)) // variable + readonly
                        } else if prev_non_trivia_kind == SyntaxKind::KwLet || prev_non_trivia_kind == SyntaxKind::KwVar {
                            Some((8, 1)) // variable declaration
                        } else if prev_non_trivia_kind == SyntaxKind::KwConst {
                            Some((8, 5)) // variable declaration + readonly
                        } else {
                            Some((8, 0)) // variable
                        }
                    }

                    // Strings (18)
                    SyntaxKind::StrLit
                    | SyntaxKind::TripleStrLit
                    | SyntaxKind::RawDslBlock
                    | SyntaxKind::FStrStart
                    | SyntaxKind::FStrMiddle
                    | SyntaxKind::FStrEnd => Some((18, 0)), // string

                    // Keyword literal / symbols (10: enumMember)
                    SyntaxKind::KeywordLit => Some((10, 0)),

                    // Numbers (19)
                    SyntaxKind::IntLit
                    | SyntaxKind::HexIntLit
                    | SyntaxKind::BinIntLit
                    | SyntaxKind::OctIntLit
                    | SyntaxKind::FloatLit
                    | SyntaxKind::DurationLit
                    | SyntaxKind::FrequencyLit
                    | SyntaxKind::MemoryLit => Some((19, 0)), // number

                    // Comments (17)
                    SyntaxKind::LineComment | SyntaxKind::BlockComment | SyntaxKind::Shebang => Some((17, 0)), // comment

                    // Operators (21)
                    k if (k as u16 >= SyntaxKind::Plus as u16 && k as u16 <= SyntaxKind::DotDotLess as u16)
                        || (k as u16 >= SyntaxKind::CallQuoteShallow as u16 && k as u16 <= SyntaxKind::CallQuoteEffect as u16) => Some((21, 0)), // operator

                    // Decorators (22)
                    SyntaxKind::At => Some((22, 0)), // decorator

                    _ => None,
                };

                prev_non_trivia_kind = k;

                if let Some((t_idx, m_bits)) = token_desc {
                    if token.text_len() == 0 {
                        continue;
                    }
                    let (line_1, col_1) = line_index.offset_to_line_col(token.offset());
                    let line_0 = line_1.saturating_sub(1);
                    let col_0 = col_1.saturating_sub(1);

                    if line_0 < prev_line || (line_0 == prev_line && col_0 < prev_col) {
                        continue;
                    }

                    let delta_line = line_0 - prev_line;
                    let delta_col = if delta_line == 0 { col_0 - prev_col } else { col_0 };

                    data.push(JsonValue::Number(delta_line as f64));
                    data.push(JsonValue::Number(delta_col as f64));
                    data.push(JsonValue::Number(token.text_len() as f64));
                    data.push(JsonValue::Number(t_idx as f64));
                    data.push(JsonValue::Number(m_bits as f64));

                    prev_line = line_0;
                    prev_col = col_0;
                }
            }
        }

        JsonValue::Object(vec![
            ("data".to_string(), JsonValue::Array(data)),
        ])
    }

    fn get_file_and_offset(&self, params: &JsonValue) -> Option<(FileId, u32)> {
        let uri = params.get("textDocument")?.get("uri")?.as_str()?;
        let file_id = *self.uri_to_file_id.get(uri)?;
        let pos = params.get("position")?;
        let line = pos.get("line")?.as_u64()? as u32;
        let col = pos.get("character")?.as_u64()? as u32;

        let li = self.line_indices.get(&file_id)?;
        let byte_offset = li.line_col_to_offset(line + 1, col + 1)?;
        Some((file_id, byte_offset))
    }

    fn handle_inlay_hint(&self, params: &JsonValue) -> JsonValue {
        let uri = match params.get("textDocument").and_then(|d| d.get("uri")).and_then(|u| u.as_str()) {
            Some(u) => u,
            None => return JsonValue::Array(Vec::new()),
        };
        let file_id = match self.uri_to_file_id.get(uri) {
            Some(&id) => id,
            None => return JsonValue::Array(Vec::new()),
        };
        let src = match self.db.source_text(file_id) {
            Some(s) => s,
            None => return JsonValue::Array(Vec::new()),
        };

        let arena = pilot_parser::parse(&src);
        let root = pilot_ast::NodeId(if arena.nodes.is_empty() { 0 } else { (arena.nodes.len() - 1) as u32 });
        let analysis = pilot_analysis::AnalysisPipeline::analyze(&arena, root);

        let mut hints = Vec::new();
        let li = match self.line_indices.get(&file_id) {
            Some(l) => l,
            None => return JsonValue::Array(Vec::new()),
        };

        for (node_idx, node) in arena.nodes.iter().enumerate() {
            if node.kind == pilot_ast::NodeKind::LetStmt as u16 || node.kind == pilot_ast::NodeKind::VarStmt as u16 {
                if let Some(&ty_id) = analysis.inferred_types.get(&(node_idx as u32)) {
                    let type_name = analysis.types.display(ty_id);

                    let span = arena.spans[node_idx];
                    let (line, col) = li.offset_to_line_col(span.end);
                    hints.push(JsonValue::Object(vec![
                        ("position".to_string(), JsonValue::Object(vec![
                            ("line".to_string(), JsonValue::Number(line.saturating_sub(1) as f64)),
                            ("character".to_string(), JsonValue::Number(col.saturating_sub(1) as f64)),
                        ])),
                        ("label".to_string(), JsonValue::String(format!(": {}", type_name))),
                        ("kind".to_string(), JsonValue::Number(1.0)),
                        ("paddingLeft".to_string(), JsonValue::Bool(true)),
                    ]));
                }
            }
        }

        JsonValue::Array(hints)
    }

    fn handle_code_action(&self, params: &JsonValue) -> JsonValue {
        let uri = match params.get("textDocument").and_then(|d| d.get("uri")).and_then(|u| u.as_str()) {
            Some(u) => u,
            None => return JsonValue::Array(Vec::new()),
        };
        let _ = uri;

        let actions = vec![
            JsonValue::Object(vec![
                ("title".to_string(), JsonValue::String("Format Document with pilot fmt".to_string())),
                ("kind".to_string(), JsonValue::String("source.fixAll".to_string())),
                ("command".to_string(), JsonValue::Object(vec![
                    ("title".to_string(), JsonValue::String("Format Document".to_string())),
                    ("command".to_string(), JsonValue::String("editor.action.formatDocument".to_string())),
                ])),
            ]),
        ];

        JsonValue::Array(actions)
    }

    fn emit_diagnostics_for_file(&mut self, file_id: FileId, uri: &str) {
        let src = match self.db.source_text(file_id) {
            Some(s) => s,
            None => return,
        };

        let arena = pilot_parser::parse(&src);
        let root = pilot_ast::NodeId(if arena.nodes.is_empty() { 0 } else { (arena.nodes.len() - 1) as u32 });
        let analysis = pilot_analysis::AnalysisPipeline::analyze(&arena, root);

        let mut diags_json = Vec::new();
        if let Some(li) = self.line_indices.get(&file_id) {
            for diag in analysis.diagnostics {
                let span = diag.span;
                let (sl, sc) = li.offset_to_line_col(span.start);
                let (el, ec) = li.offset_to_line_col(span.end.max(span.start));
                let s_line = sl.saturating_sub(1);
                let s_col = sc.saturating_sub(1);
                let e_line = el.saturating_sub(1);
                let e_col = ec.saturating_sub(1);

                let sev = match diag.severity {
                    pilot_analysis::DiagnosticSeverity::Error => 1.0,
                    pilot_analysis::DiagnosticSeverity::Warning => 2.0,
                    pilot_analysis::DiagnosticSeverity::Info => 3.0,
                    pilot_analysis::DiagnosticSeverity::Hint => 4.0,
                };

                diags_json.push(JsonValue::Object(vec![
                    ("range".to_string(), make_range(s_line, s_col, e_line, e_col)),
                    ("severity".to_string(), JsonValue::Number(sev)),
                    ("message".to_string(), JsonValue::String(diag.message)),
                    ("source".to_string(), JsonValue::String("pilot".to_string())),
                ]));
            }
        }

        let notif = JsonValue::Object(vec![
            ("jsonrpc".to_string(), JsonValue::String("2.0".to_string())),
            ("method".to_string(), JsonValue::String("textDocument/publishDiagnostics".to_string())),
            ("params".to_string(), JsonValue::Object(vec![
                ("uri".to_string(), JsonValue::String(uri.to_string())),
                ("diagnostics".to_string(), JsonValue::Array(diags_json)),
            ])),
        ]);
        self.pending_notifications.push(notif.serialize());
    }
}

/// Runs a persistent `Content-Length`-framed JSON-RPC 2.0 loop reading from `reader` and writing to `writer`.
pub fn run_stdio_server<R: std::io::BufRead, W: std::io::Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut server = LspServer::new();
    loop {
        if server.shutdown_requested {
            break;
        }

        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break; // EOF
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            let content_len: usize = match rest.trim().parse() {
                Ok(len) => len,
                Err(_) => continue,
            };

            // Read headers until empty line
            loop {
                line.clear();
                let n = reader.read_line(&mut line)?;
                if n == 0 || line.trim().is_empty() {
                    break;
                }
            }

            // Read exact payload bytes
            let mut payload = vec![0u8; content_len];
            reader.read_exact(&mut payload)?;
            let raw_json = String::from_utf8_lossy(&payload);

            if let Some(resp) = server.handle_message(&raw_json) {
                let out = format!("Content-Length: {}\r\n\r\n{}", resp.len(), resp);
                writer.write_all(out.as_bytes())?;
            }

            for notif in server.take_pending_notifications() {
                let out = format!("Content-Length: {}\r\n\r\n{}", notif.len(), notif);
                writer.write_all(out.as_bytes())?;
            }

            writer.flush()?;
        }
    }
    Ok(())
}

fn make_response(id: Option<JsonValue>, result: JsonValue) -> JsonValue {
    JsonValue::Object(vec![
        ("jsonrpc".to_string(), JsonValue::String("2.0".to_string())),
        ("id".to_string(), id.unwrap_or(JsonValue::Null)),
        ("result".to_string(), result),
    ])
}

fn make_range(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> JsonValue {
    JsonValue::Object(vec![
        ("start".to_string(), JsonValue::Object(vec![
            ("line".to_string(), JsonValue::Number(start_line as f64)),
            ("character".to_string(), JsonValue::Number(start_col as f64)),
        ])),
        ("end".to_string(), JsonValue::Object(vec![
            ("line".to_string(), JsonValue::Number(end_line as f64)),
            ("character".to_string(), JsonValue::Number(end_col as f64)),
        ])),
    ])
}

fn make_hover_card(md: &str) -> JsonValue {
    JsonValue::Object(vec![
        ("contents".to_string(), JsonValue::Object(vec![
            ("kind".to_string(), JsonValue::String("markdown".to_string())),
            ("value".to_string(), JsonValue::String(md.to_string())),
        ])),
    ])
}

fn extract_params_from_sig(sig: &str) -> Vec<String> {
    if let Some(open) = sig.find('(') {
        if let Some(close) = sig[open..].find(')') {
            let inner = &sig[open + 1..open + close];
            if !inner.trim().is_empty() {
                return inner.split(',').map(|s| s.trim().to_string()).collect();
            }
        }
    }
    Vec::new()
}

fn make_signature_help_resp(sig: &str, doc: &str, params: &[String], active_param: u32) -> JsonValue {
    let param_objs: Vec<JsonValue> = params
        .iter()
        .map(|p| JsonValue::Object(vec![("label".to_string(), JsonValue::String(p.clone()))]))
        .collect();

    let sig_info = JsonValue::Object(vec![
        ("label".to_string(), JsonValue::String(sig.to_string())),
        ("documentation".to_string(), JsonValue::String(doc.to_string())),
        ("parameters".to_string(), JsonValue::Array(param_objs)),
    ]);

    JsonValue::Object(vec![
        ("signatures".to_string(), JsonValue::Array(vec![sig_info])),
        ("activeSignature".to_string(), JsonValue::Number(0.0)),
        ("activeParameter".to_string(), JsonValue::Number(active_param as f64)),
    ])
}

fn get_keyword_doc(ident: &str) -> Option<(&'static str, &'static str)> {
    KEYWORD_DOCS.iter().find(|(k, _)| *k == ident).map(|(_, v)| *v)
}

fn get_builtin_doc(ident: &str) -> Option<(&'static str, &'static str)> {
    BUILTIN_DOCS.iter().find(|(k, _, _)| *k == ident).map(|(_, sig, sum)| (*sig, *sum))
}

fn get_polyglot_doc(ident: &str) -> Option<&'static str> {
    POLYGLOT_DOCS.iter().find(|(k, _)| *k == ident).map(|(_, v)| *v)
}

fn get_type_doc(ident: &str) -> Option<&'static str> {
    TYPE_DOCS.iter().find(|(k, _)| *k == ident).map(|(_, v)| *v)
}

const BUILTIN_DOCS: &[(&str, &str, &str)] = &[
    ("println", "println(val: any) -> void", "Prints values to standard output followed by a newline."),
    ("print", "print(val: any) -> void", "Prints values to standard output without a trailing newline."),
    ("panic", "panic(msg: str) -> never", "Aborts program execution immediately with an unrecoverable error message."),
    ("assert", "assert(condition: bool, msg: str = \"\") -> void", "Asserts that condition is true; aborts with an assertion failure if false."),
    ("len", "len(collection: any) -> int", "Returns the number of elements in a list, map, string, or collection."),
    ("range", "range(start: int, end: int, step: int = 1) -> List[int]", "Generates an iterable sequence of integers from `start` up to `end`."),
    ("typeof", "typeof(val: any) -> str", "Returns the runtime type name of the given value."),
    ("append", "append(item: any) -> void", "Appends an element to the end of a list or buffer."),
    ("malloc", "malloc(size: int) -> *void", "Allocates unmanaged heap memory of the given byte size."),
    ("free", "free(ptr: *void) -> void", "Frees previously allocated unmanaged heap memory."),
    ("memset", "memset(dest: *void, val: int, count: int) -> *void", "Fills memory block with the given byte value."),
    ("clock_gettime", "clock_gettime(clock_id: int) -> int", "Returns high-resolution system clock timestamp in nanoseconds."),
];

const POLYGLOT_DOCS: &[(&str, &str)] = &[
    ("python", "Inline Python block with zero-copy bidirectional variable binding and generator/coroutine yield bridges."),
    ("js", "Inline JavaScript engine execution block with seamless host data interchange."),
    ("lua", "Ultra-fast embedded Lua runtime block for high-throughput scripting."),
    ("java", "JVM polyglot execution block supporting Java class and method interop."),
    ("c", "Zero-overhead native C code block compiled and linked directly into the binary."),
    ("cpp", "Native C++ execution block with template, class, and STL support."),
    ("asm", "Inline assembly block using machine registers and SIMD instructions."),
    ("rust", "Inline Rust block compiled natively with Perceus memory management semantics."),
    ("csharp", "Inline .NET/C# execution block with CLR interop."),
    ("ruby", "Inline Ruby scripting block with dynamic evaluation."),
    ("go", "Inline Go execution block with goroutine support."),
    ("zig", "Inline Zig execution block with compile-time metaprogramming."),
];

const TYPE_DOCS: &[(&str, &str)] = &[
    ("str", "UTF-8 string with SSO (Small String Optimization, <= 23 bytes inline, zero allocations) and Perceus refcounting."),
    ("int", "64-bit signed two's complement integer (`i64`)."),
    ("i64", "64-bit signed integer."),
    ("i32", "32-bit signed integer."),
    ("i16", "16-bit signed integer."),
    ("i8", "8-bit signed integer."),
    ("u64", "64-bit unsigned integer."),
    ("u32", "32-bit unsigned integer."),
    ("u16", "16-bit unsigned integer."),
    ("u8", "8-bit unsigned byte."),
    ("float", "64-bit IEEE 754 double-precision floating-point number (`f64`)."),
    ("f64", "64-bit IEEE 754 double-precision floating-point number."),
    ("f32", "32-bit IEEE 754 single-precision floating-point number."),
    ("bool", "Boolean logical value (`true` or `false`)."),
    ("void", "Represents the absence of a value or void return type."),
    ("any", "Dynamic row-polymorphic open type capable of holding any Pilot value."),
    ("never", "Bottom type representing a computation that never completes or halts."),
    ("List", "Dynamic growable array with O(1) amortized append and Perceus in-place reuse."),
    ("Map", "Robin Hood hashed key-value map with cache-coherent flat storage."),
    ("Option", "Optional value type representing `Some(T)` or `None`."),
    ("Result", "Error handling type representing either `Ok(T)` or `Err(E)`."),
];

const KEYWORD_DOCS: &[(&str, (&str, &str))] = &[
    ("let", ("Core / Type Safety", "`let <name>: <type> = <expr>` — Declares an immutable binding.")),
    ("var", ("Core / Type Safety", "`var <name>: <type> = <expr>` — Declares a mutable variable.")),
    ("const", ("Core / Type Safety", "`const <name>: <type> = <expr>` — Declares a compile-time constant.")),
    ("def", ("Declarations", "`def <name>(<params>) [-> <ret>] { ... }` — Declares a function or method.")),
    ("fn", ("Declarations", "`fn <name>(<params>) [-> <ret>] { ... }` — Declares a function or method.")),
    ("class", ("Declarations", "`class <Name> { ... }` — Declares an object-oriented class.")),
    ("struct", ("Declarations", "`struct <Name> { ... }` — Declares a lightweight value-type struct.")),
    ("return", ("Control Flow", "`return [<expr>]` — Exits current function.")),
    ("if", ("Control Flow", "`if <condition> { ... }` — Conditional execution branch.")),
    ("else", ("Control Flow", "`else { ... }` — Fallback alternative branch.")),
    ("while", ("Control Flow", "`while <condition> { ... }` — Loop while condition is true.")),
    ("for", ("Control Flow", "`for <item> in <collection> { ... }` — Iterates through collections.")),
    ("match", ("Pattern Matching", "`match <expr> { case <pat> => ... }` — Pattern matching control structure.")),
    ("guard", ("Core / Preconditions", "`guard <cond> else { ... }` — Mandatory early-exit precondition.")),
    ("defer", ("Core / Cleanup", "`defer <stmt | { ... }>` — Schedules deterministic cleanup execution.")),
];
