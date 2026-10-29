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

    fn handle_initialize(&self, id: Option<&JsonValue>) -> JsonValue {
        let token_types: Vec<JsonValue> = Self::TOKEN_TYPES
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
            ("definitionProvider".to_string(), JsonValue::Bool(true)),
            ("documentSymbolProvider".to_string(), JsonValue::Bool(true)),
            ("documentFormattingProvider".to_string(), JsonValue::Bool(true)),
            ("inlayHintProvider".to_string(), JsonValue::Bool(true)),
            ("codeActionProvider".to_string(), JsonValue::Bool(true)),
            ("semanticTokensProvider".to_string(), JsonValue::Object(vec![
                ("legend".to_string(), JsonValue::Object(vec![
                    ("tokenTypes".to_string(), JsonValue::Array(token_types)),
                    ("tokenModifiers".to_string(), JsonValue::Array(Vec::new())),
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

        // 1. Check keywords
        if let Some((domain, summary)) = get_keyword_doc(txt) {
            let md = format!("**Pilot Keyword**: `{}` ({})\n\n{}", txt, domain, summary);
            return JsonValue::Object(vec![
                ("contents".to_string(), JsonValue::Object(vec![
                    ("kind".to_string(), JsonValue::String("markdown".to_string())),
                    ("value".to_string(), JsonValue::String(md)),
                ])),
            ]);
        }

        // 2. Check top-level items in file
        if let Some(item_tree) = self.db.module_item_tree(file_id) {
            for item in &item_tree.items {
                if item.name == txt {
                    let md = format!("**Pilot Definition**: `{}`\n```pilot\n{}\n```", item.name, item.signature_text);
                    return JsonValue::Object(vec![
                        ("contents".to_string(), JsonValue::Object(vec![
                            ("kind".to_string(), JsonValue::String("markdown".to_string())),
                            ("value".to_string(), JsonValue::String(md)),
                        ])),
                    ]);
                }
            }
        }

        JsonValue::Null
    }

    fn handle_completion(&self, params: &JsonValue) -> JsonValue {
        let mut items = Vec::new();

        // Keyword completions
        for (kw, (domain, summary)) in KEYWORD_DOCS.iter() {
            items.push(JsonValue::Object(vec![
                ("label".to_string(), JsonValue::String(kw.to_string())),
                ("kind".to_string(), JsonValue::Number(14.0)), // Keyword
                ("detail".to_string(), JsonValue::String(domain.to_string())),
                ("documentation".to_string(), JsonValue::String(summary.to_string())),
            ]));
        }

        // Top-level module symbols
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

    fn handle_semantic_tokens_full(&self, params: &JsonValue) -> JsonValue {
        let uri = match params.get("textDocument").and_then(|d| d.get("uri")).and_then(|u| u.as_str()) {
            Some(u) => u,
            None => return JsonValue::Object(vec![("data".to_string(), JsonValue::Array(Vec::new()))]),
        };
        let file_id = match self.uri_to_file_id.get(uri) {
            Some(&id) => id,
            None => return JsonValue::Object(vec![("data".to_string(), JsonValue::Array(Vec::new()))]),
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

                let type_idx = match k {
                    // Keywords (15)
                    k if (k as u16 >= SyntaxKind::KwLet as u16 && k as u16 <= SyntaxKind::KwDefault as u16) => {
                        match k {
                            SyntaxKind::KwClass => Some(2), // class
                            SyntaxKind::KwStruct | SyntaxKind::KwRecord => Some(5), // struct
                            SyntaxKind::KwInterface | SyntaxKind::KwProtocol => Some(4), // interface
                            SyntaxKind::KwEnum => Some(3), // enum
                            SyntaxKind::KwType => Some(1), // type
                            _ => Some(15), // keyword
                        }
                    }

                    // Identifiers (function / type / decorator / variable)
                    SyntaxKind::Ident => {
                        let end_offset = (token.offset() + token.text_len()) as usize;
                        let remainder = src.get(end_offset..).unwrap_or("").trim_start();
                        if pilot_parser::is_polyglot_lang(token.text()) && (remainder.starts_with('{') || remainder.starts_with("```") || remainder.starts_with("\"\"\"")) {
                            Some(15) // keyword
                        } else if prev_non_trivia_kind == SyntaxKind::At {
                            Some(22) // decorator
                        } else if prev_non_trivia_kind == SyntaxKind::KwClass {
                            Some(2) // class
                        } else if prev_non_trivia_kind == SyntaxKind::KwStruct || prev_non_trivia_kind == SyntaxKind::KwRecord {
                            Some(5) // struct
                        } else if prev_non_trivia_kind == SyntaxKind::KwInterface || prev_non_trivia_kind == SyntaxKind::KwProtocol {
                            Some(4) // interface
                        } else if prev_non_trivia_kind == SyntaxKind::KwEnum {
                            Some(3) // enum
                        } else if prev_non_trivia_kind == SyntaxKind::KwType {
                            Some(1) // type
                        } else if prev_non_trivia_kind == SyntaxKind::KwDef || prev_non_trivia_kind == SyntaxKind::KwFn {
                            Some(12) // function definition
                        } else {
                            let end_offset = (token.offset() + token.text_len()) as usize;
                            let remainder = src.get(end_offset..).unwrap_or("").trim_start();
                            if remainder.starts_with('(') {
                                Some(12) // function call
                            } else if token.text().starts_with(char::is_uppercase) {
                                Some(1) // type (PascalCase)
                            } else {
                                Some(8) // variable
                            }
                        }
                    }

                    // Strings (18)
                    SyntaxKind::StrLit
                    | SyntaxKind::TripleStrLit
                    | SyntaxKind::FStrStart
                    | SyntaxKind::FStrMiddle
                    | SyntaxKind::FStrEnd => Some(18), // string

                    // Numbers (19)
                    SyntaxKind::IntLit
                    | SyntaxKind::HexIntLit
                    | SyntaxKind::BinIntLit
                    | SyntaxKind::OctIntLit
                    | SyntaxKind::FloatLit
                    | SyntaxKind::DurationLit
                    | SyntaxKind::FrequencyLit
                    | SyntaxKind::MemoryLit => Some(19), // number

                    // Comments (17)
                    SyntaxKind::LineComment | SyntaxKind::BlockComment | SyntaxKind::Shebang => Some(17), // comment

                    // Operators (21)
                    k if (k as u16 >= SyntaxKind::Plus as u16 && k as u16 <= SyntaxKind::DotDotLess as u16) => Some(21), // operator

                    // Decorators (22)
                    SyntaxKind::At => Some(22), // decorator

                    _ => None,
                };

                prev_non_trivia_kind = k;

                if let Some(t_idx) = type_idx {
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
                    data.push(JsonValue::Number(0.0)); // tokenModifiers

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
                    let type_name = match analysis.types.get(ty_id) {
                        pilot_analysis::TypeKind::Primitive(p) => p.clone(),
                        pilot_analysis::TypeKind::Collection { element, .. } => format!("List[{}]", element.0),
                        pilot_analysis::TypeKind::Map { key, value } => format!("Map[{}, {}]", key.0, value.0),
                        pilot_analysis::TypeKind::Option(inner) => format!("{}?", inner.0),
                        _ => "any".to_string(),
                    };

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

fn get_keyword_doc(ident: &str) -> Option<(&'static str, &'static str)> {
    KEYWORD_DOCS.iter().find(|(k, _)| *k == ident).map(|(_, v)| *v)
}

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
