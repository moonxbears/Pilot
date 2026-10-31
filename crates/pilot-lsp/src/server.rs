//! Milestone 32.3: Zero-GC Standalone Native Language Server (`pilot-lsp`).

use std::collections::HashMap;
use pilot_db::{diagnostics_query, Database, FileId};
use pilot_fmt::{format_source, FormatOptions};
use pilot_incremental::TextEdit;
use pilot_lsp_macros::LspQuery;
use pilot_span::LineIndex;
use pilot_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};

use crate::json::{parse_json, JsonValue};

#[derive(LspQuery)]
#[lsp_method("textDocument/hover")]
#[lsp_capability("hoverProvider")]
pub struct HoverQuery;

#[derive(LspQuery)]
#[lsp_method("textDocument/completion")]
#[lsp_capability("completionProvider")]
pub struct CompletionQuery;

#[derive(LspQuery)]
#[lsp_method("textDocument/definition")]
#[lsp_capability("definitionProvider")]
pub struct DefinitionQuery;

#[derive(LspQuery)]
#[lsp_method("textDocument/documentSymbol")]
#[lsp_capability("documentSymbolProvider")]
pub struct DocumentSymbolQuery;

#[derive(LspQuery)]
#[lsp_method("textDocument/formatting")]
#[lsp_capability("documentFormattingProvider")]
pub struct FormattingQuery;

#[derive(LspQuery)]
#[lsp_method("textDocument/inlayHint")]
#[lsp_capability("inlayHintProvider")]
pub struct InlayHintQuery;

#[derive(LspQuery)]
#[lsp_method("textDocument/signatureHelp")]
#[lsp_capability("signatureHelpProvider")]
pub struct SignatureHelpQuery;

#[derive(LspQuery)]
#[lsp_method("textDocument/codeAction")]
#[lsp_capability("codeActionProvider")]
pub struct CodeActionQuery;

#[derive(LspQuery)]
#[lsp_method("textDocument/semanticTokens/full")]
#[lsp_capability("semanticTokensProvider")]
pub struct SemanticTokensFullQuery;

#[derive(LspQuery)]
#[lsp_method("textDocument/references")]
#[lsp_capability("referencesProvider")]
pub struct ReferencesQuery;

#[derive(LspQuery)]
#[lsp_method("textDocument/implementation")]
#[lsp_capability("implementationProvider")]
pub struct ImplementationQuery;

#[derive(LspQuery)]
#[lsp_method("textDocument/declaration")]
#[lsp_capability("declarationProvider")]
pub struct DeclarationQuery;

#[derive(LspQuery)]
#[lsp_method("textDocument/prepareRename")]
pub struct PrepareRenameQuery;

#[derive(LspQuery)]
#[lsp_method("textDocument/rename")]
#[lsp_capability("renameProvider")]
pub struct RenameQuery;

pub struct LspServer {
    db: Database,
    uri_to_file_id: HashMap<String, FileId>,
    file_id_to_uri: HashMap<FileId, String>,
    line_indices: HashMap<FileId, LineIndex>,
    next_file_id: u32,
    pub shutdown_requested: bool,
    pub pending_notifications: Vec<String>,
    pub workspace_folders: Vec<String>,
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
            workspace_folders: Vec::new(),
        }
    }

    pub fn take_pending_notifications(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_notifications)
    }

    pub fn analysis_cache_hits(&self) -> usize {
        self.db.analysis_cache_hits
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
                let resp = self.handle_initialize(id.as_ref(), params);
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
            Some(HoverQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_hover(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Null).serialize())
                }
            }
            Some(CompletionQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_completion(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Array(Vec::new())).serialize())
                }
            }
            Some(DefinitionQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_definition(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Null).serialize())
                }
            }
            Some(DocumentSymbolQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_document_symbol(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Array(Vec::new())).serialize())
                }
            }
            Some(FormattingQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_formatting(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Array(Vec::new())).serialize())
                }
            }
            Some(InlayHintQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_inlay_hint(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Array(Vec::new())).serialize())
                }
            }
            Some(SignatureHelpQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_signature_help(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Null).serialize())
                }
            }
            Some(CodeActionQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_code_action(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Array(Vec::new())).serialize())
                }
            }
            Some(SemanticTokensFullQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_semantic_tokens_full(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Object(vec![("data".to_string(), JsonValue::Array(Vec::new()))])).serialize())
                }
            }
            Some(ReferencesQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_references(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Array(Vec::new())).serialize())
                }
            }
            Some(ImplementationQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_implementation(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Array(Vec::new())).serialize())
                }
            }
            Some(DeclarationQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_definition(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Null).serialize())
                }
            }
            Some(PrepareRenameQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_prepare_rename(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Null).serialize())
                }
            }
            Some(RenameQuery::LSP_METHOD) => {
                if let Some(p) = params {
                    let res = self.handle_rename(p);
                    Some(make_response(id, res).serialize())
                } else {
                    Some(make_response(id, JsonValue::Null).serialize())
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

    fn handle_initialize(&mut self, id: Option<&JsonValue>, params: Option<&JsonValue>) -> JsonValue {
        if let Some(p) = params {
            if let Some(root_uri) = p.get("rootUri").and_then(|u| u.as_str()) {
                self.workspace_folders.push(root_uri.to_string());
            } else if let Some(root_path) = p.get("rootPath").and_then(|u| u.as_str()) {
                self.workspace_folders.push(format!("file://{}", root_path));
            }
            if let Some(folders) = p.get("workspaceFolders").and_then(|f| f.as_array()) {
                for f in folders {
                    if let Some(uri) = f.get("uri").and_then(|u| u.as_str()) {
                        if !self.workspace_folders.contains(&uri.to_string()) {
                            self.workspace_folders.push(uri.to_string());
                        }
                    }
                }
            }
            self.discover_workspace_files();
        }

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
            ("declarationProvider".to_string(), JsonValue::Bool(true)),
            ("implementationProvider".to_string(), JsonValue::Bool(true)),
            ("referencesProvider".to_string(), JsonValue::Bool(true)),
            ("renameProvider".to_string(), JsonValue::Object(vec![
                ("prepareProvider".to_string(), JsonValue::Bool(true)),
            ])),
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

    fn handle_hover(&mut self, params: &JsonValue) -> JsonValue {
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

        // 0. Check for Call-Quote operators (!(), !!(), !&(), !^(), !@(), !?()) around cursor
        if let Some((q_syntax, q_title, q_desc)) = detect_call_quote_at(&src, byte_offset as usize) {
            let md = format!("**Pilot Call-Quoting Operator**: `{}` — *{}*\n\n{}", q_syntax, q_title, q_desc);
            return make_hover_card(&md);
        }

        let root = SyntaxNode::new_root(green);
        let mut token = root.find_token_at_offset(byte_offset);
        if token.as_ref().map(|t| t.kind().is_trivia()).unwrap_or(true) && byte_offset > 0 {
            if let Some(prev) = root.find_token_at_offset(byte_offset - 1) {
                if !prev.kind().is_trivia() {
                    token = Some(prev);
                }
            }
        }

        let tok = match token {
            Some(ref t) => t,
            None => return JsonValue::Null,
        };

        let tok_kind = tok.kind();
        let raw_txt = tok.text();

        // 0a. Dimensional literals (250ms, 60Hz, 64KB)
        match tok_kind {
            SyntaxKind::DurationLit => {
                let md = format!(
                    "```pilot\n(literal) {}: i64<Duration>\n```\n\n**Pilot Dimensional Duration Literal**\n\nCompile-time verified physical duration (`ns`, `us`, `ms`, `s`) scaled to nanoseconds for deterministic real-time scheduling (`deadline`, `within`, `periodic`, `timeout`).",
                    raw_txt
                );
                return make_hover_card(&md);
            }
            SyntaxKind::FrequencyLit => {
                let md = format!(
                    "```pilot\n(literal) {}: i64<Frequency>\n```\n\n**Pilot Dimensional Frequency Literal**\n\nCompile-time verified frequency (`Hz`, `kHz`, `MHz`, `GHz`) for real-time periodic control loops (`periodic(60Hz)`).",
                    raw_txt
                );
                return make_hover_card(&md);
            }
            SyntaxKind::MemoryLit => {
                let md = format!(
                    "```pilot\n(literal) {}: i64<Memory>\n```\n\n**Pilot Dimensional Memory Literal**\n\nCompile-time verified memory capacity (`B`, `KB`, `MB`, `GB`) for bounded stack and region allocations (`bounded(64KB)`, `stackalloc`).",
                    raw_txt
                );
                return make_hover_card(&md);
            }
            SyntaxKind::KeywordLit => {
                let md = format!(
                    "```pilot\n(symbol) {}: keyword\n```\n\n**Pilot Interned Symbol / Keyword Literal**\n\nCompile-time interned symbol with $O(1)$ pointer-equality comparison, used for state tags, actor messages, and DSL attributes.",
                    raw_txt
                );
                return make_hover_card(&md);
            }
            _ => {}
        }

        // 0b. Operators & punctuation hover
        if tok_kind == SyntaxKind::LBracket || tok_kind == SyntaxKind::RBracket {
            if let Some(mod_analysis) = self.db.module_analysis(file_id) {
                let arena = &mod_analysis.arena;
                let analysis = &mod_analysis.analysis;
                for (idx, node) in arena.nodes.iter().enumerate() {
                    if node.node_kind() == pilot_ast::NodeKind::IndexExpr {
                        let span = arena.spans[idx];
                        if byte_offset >= span.start && byte_offset <= span.end {
                            let mut indexer_sig: Option<(Vec<String>, String)> = None;
                            for (fn_idx, fn_node) in arena.nodes.iter().enumerate() {
                                if fn_node.node_kind() == pilot_ast::NodeKind::FnDecl {
                                    let name = arena.interner.resolve(pilot_intern::SymbolId(fn_node.lhs));
                                    if name == "this[]" {
                                        let fn_start = fn_node.rhs;
                                        let fn_len = fn_node.flags as usize;
                                        let mut param_strs = Vec::new();
                                        if fn_len > 0 {
                                            let fn_children = arena.children_slice(fn_start, fn_len as u32);
                                            for &p_id in &fn_children[1..] {
                                                let p_node = *arena.node(pilot_ast::NodeId(p_id));
                                                let p_name = arena.interner.resolve(pilot_intern::SymbolId(p_node.lhs));
                                                let p_ty = analysis
                                                    .inferred_types
                                                    .get(&p_id)
                                                    .copied()
                                                    .unwrap_or(pilot_analysis::TypeId::ANY);
                                                param_strs.push(format!("{}: {}", p_name, analysis.types.display(p_ty)));
                                            }
                                        }
                                        let ret_ty = analysis
                                            .inferred_types
                                            .get(&(fn_idx as u32))
                                            .copied()
                                            .unwrap_or(pilot_analysis::TypeId::ANY);
                                        let ret_str = if let pilot_analysis::TypeKind::Function { ret, .. } = analysis.types.get(analysis.types.prune(ret_ty)) {
                                            analysis.types.display(*ret)
                                        } else {
                                            analysis.types.display(ret_ty)
                                        };
                                        indexer_sig = Some((param_strs, ret_str));
                                        break;
                                    }
                                }
                            }
                            if let Some((param_strs, ret_str)) = indexer_sig {
                                let md = format!("```pilot\nproperty this[{}]: {}\n```\n\n*Pilot Indexer Access*", param_strs.join(", "), ret_str);
                                return make_hover_card(&md);
                            }
                        }
                    }
                }
            }
        }
        if let Some((op_title, op_desc)) = get_operator_doc(raw_txt) {
            let md = format!("**Pilot Operator**: `{}` ({})\n\n{}", raw_txt, op_title, op_desc);
            return make_hover_card(&md);
        }

        // If inside an interpolated string `f"..."`, extract the identifier word under the cursor
        let word_in_str = if tok_kind == SyntaxKind::FStrStart || tok_kind == SyntaxKind::StrLit {
            extract_word_at(&src, byte_offset as usize)
        } else {
            None
        };
        let txt = word_in_str.as_deref().unwrap_or(raw_txt);

        // If hovering on an f-string outside any identifier (e.g. on `f"` or `{` / `}`), explain f-strings
        if tok_kind == SyntaxKind::FStrStart && word_in_str.is_none() {
            let md = "**Pilot Interpolated String**: `f\"...{expr}...\"` / `$\"...{expr}...\"`\n\nEvaluates embedded Pilot expressions inside `{...}` and formats them into a UTF-8 string with Small String Optimization (SSO).";
            return make_hover_card(md);
        }

        // Check if preceded by `.` / `?.` / `..` (member access context)
        let is_after_dot = {
            let tok_start = tok.offset() as usize;
            let before = src.get(..tok_start).unwrap_or("").trim_end();
            before.ends_with('.') || before.ends_with("?.") || before.ends_with("..")
        };

        // Check if followed by `{` or `"""` or ```` ``` ```` (polyglot block context)
        let is_polyglot_head = {
            let tok_end = (tok.offset() + tok.text_len()) as usize;
            let after = src.get(tok_end..).unwrap_or("").trim_start();
            pilot_parser::is_polyglot_lang(txt)
                && (after.starts_with('{') || after.starts_with("```") || after.starts_with("\"\"\""))
        };

        if is_polyglot_head {
            if let Some(doc) = get_polyglot_doc(txt) {
                let md = format!("**Pilot Polyglot Block**: `{}`\n\n{}", txt, doc);
                return make_hover_card(&md);
            }
        }

        // If `txt` is a strict keyword (and not after `.` or inside an f-string), show keyword doc immediately
        if word_in_str.is_none() && !is_after_dot && tok_kind.is_keyword() && !tok_kind.is_contextual_keyword() {
            if let Some((sig, summary)) = get_builtin_doc(txt) {
                let md = format!("```pilot\n{}\n```\n\n{}", sig, summary);
                return make_hover_card(&md);
            }
            if let Some(doc) = get_type_doc(txt) {
                let md = format!("**Pilot Type**: `{}`\n\n{}", txt, doc);
                return make_hover_card(&md);
            }
            if let Some((domain, summary)) = get_keyword_doc(txt) {
                let md = format!("**Pilot Keyword**: `{}` ({})\n\n{}", txt, domain, summary);
                return make_hover_card(&md);
            }
        }

        // Query Memoized AST & Bidirectional Type Inference from `pilot-db`
        let mod_analysis = match self.db.module_analysis(file_id) {
            Some(m) => m,
            None => return JsonValue::Null,
        };
        let arena = &mod_analysis.arena;
        let analysis = &mod_analysis.analysis;

        // 1. Member access hover (`obj.member` or `obj?.member`)
        if is_after_dot {
            let mut best_member: Option<(usize, u32)> = None;
            for (idx, node) in arena.nodes.iter().enumerate() {
                if node.node_kind() == pilot_ast::NodeKind::MemberExpr {
                    let m_name = arena.interner.resolve(pilot_intern::SymbolId(node.rhs));
                    if m_name == txt {
                        let span = arena.spans[idx];
                        let dist = if byte_offset >= span.start {
                            byte_offset - span.start
                        } else {
                            span.start - byte_offset
                        };
                        if best_member.map_or(true, |(_, b_dist)| dist < b_dist) {
                            best_member = Some((idx, dist));
                        }
                    }
                }
            }
            if let Some((idx, _)) = best_member {
                let node = arena.nodes[idx];
                let target_ty = analysis
                    .inferred_types
                    .get(&node.lhs)
                    .copied()
                    .unwrap_or(pilot_analysis::TypeId::ANY);
                let member_ty = analysis
                    .inferred_types
                    .get(&(idx as u32))
                    .copied()
                    .unwrap_or(pilot_analysis::TypeId::ANY);
                let target_str = analysis.types.display(target_ty);
                let member_str = analysis.types.display(member_ty);
                let kind_label = if matches!(analysis.types.get(analysis.types.prune(member_ty)), pilot_analysis::TypeKind::Function { .. }) {
                    "method"
                } else {
                    "property"
                };
                let extra_doc = match txt {
                    "len" | "length" | "count" => "\n\nReturns the number of elements or bytes in the receiver.",
                    "size_of" => "\n\nCompile-time size in bytes of the value or type layout.",
                    "tag_bytes" => "\n\nNumber of discriminant tag bytes used by an algebraic data type.",
                    "is_unboxed" => "\n\n`true` if the ADT or struct is unboxed inline without heap allocation.",
                    "is_recursive" => "\n\n`true` if the type contains recursive indirection.",
                    "is_empty" => "\n\n`true` if the collection or string contains zero elements.",
                    "is_some" | "is_none" => "\n\nChecks whether an `Option[T]` contains a value.",
                    "is_ok" | "is_err" => "\n\nChecks whether a `Result[T, E]` is `Ok` or `Err`.",
                    _ => "",
                };
                let md = format!(
                    "```pilot\n({}) {}.{}: {}\n```\n\n*Pilot Member Access*{}",
                    kind_label, target_str, txt, member_str, extra_doc
                );
                return make_hover_card(&md);
            }
        }

        // 2. Functions in file (definitions & call-sites)
        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == pilot_ast::NodeKind::FnDecl {
                let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                let span = arena.spans[idx];
                let is_indexer_match = name == "this[]" && txt == "this" && byte_offset >= span.start && byte_offset <= span.end;
                if name == txt || is_indexer_match {
                    let fn_start = node.rhs;
                    let fn_len = node.flags as usize;
                    let mut param_strs = Vec::new();
                    if fn_len > 0 {
                        let fn_children = arena.children_slice(fn_start, fn_len as u32);
                        for &p_id in &fn_children[1..] {
                            let p_node = *arena.node(pilot_ast::NodeId(p_id));
                            let p_name = arena.interner.resolve(pilot_intern::SymbolId(p_node.lhs));
                            let p_ty = analysis
                                .inferred_types
                                .get(&p_id)
                                .copied()
                                .unwrap_or(pilot_analysis::TypeId::ANY);
                            param_strs.push(format!("{}: {}", p_name, analysis.types.display(p_ty)));
                        }
                    }
                    let ret_ty = analysis
                        .inferred_types
                        .get(&(idx as u32))
                        .copied()
                        .unwrap_or(pilot_analysis::TypeId::ANY);
                    let ret_str = if let pilot_analysis::TypeKind::Function { ret, .. } = analysis.types.get(analysis.types.prune(ret_ty)) {
                        analysis.types.display(*ret)
                    } else {
                        analysis.types.display(ret_ty)
                    };
                    if name == "this[]" {
                        let sig = format!("property this[{}]: {}", param_strs.join(", "), ret_str);
                        let md = format!("```pilot\n{}\n```\n\n*Pilot Indexer*", sig);
                        return make_hover_card(&md);
                    }
                    let sig = format!("def {}({}) -> {}", name, param_strs.join(", "), ret_str);
                    let md = format!("```pilot\n{}\n```\n\n*Pilot Function*", sig);
                    return make_hover_card(&md);
                }
            }
        }

        // 3. Classes, Interfaces, Structs, Records, Type Aliases, ADTs, and ADT Variants
        for (idx, node) in arena.nodes.iter().enumerate() {
            match node.node_kind() {
                pilot_ast::NodeKind::StructDecl | pilot_ast::NodeKind::RecordDecl => {
                    let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                    if name == txt {
                        let kw = if node.node_kind() == pilot_ast::NodeKind::RecordDecl { "record" } else { "struct" };
                        let ty_id = analysis
                            .inferred_types
                            .get(&(idx as u32))
                            .copied()
                            .unwrap_or(pilot_analysis::TypeId::ANY);
                        let fields_str = analysis.types.display(ty_id);
                        let md = format!("```pilot\n{} {} {}\n```\n\n*Pilot Value Type*", kw, name, fields_str);
                        return make_hover_card(&md);
                    }
                }
                pilot_ast::NodeKind::ClassDecl | pilot_ast::NodeKind::InterfaceDecl => {
                    let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                    if name == txt {
                        let kw = if node.node_kind() == pilot_ast::NodeKind::InterfaceDecl { "interface" } else { "class" };
                        let md = format!("```pilot\n{} {}\n```\n\n*Pilot Object Type*", kw, name);
                        return make_hover_card(&md);
                    }
                }
                pilot_ast::NodeKind::TypeAdtDecl => {
                    let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                    if name == txt {
                        let var_start = node.rhs as usize;
                        let var_len = node.flags as usize;
                        let mut variants = Vec::new();
                        if var_len > 0 && var_start + var_len <= arena.extra_children.len() {
                            for &v_id in &arena.extra_children[var_start..var_start + var_len] {
                                let v_node = *arena.node(pilot_ast::NodeId(v_id));
                                if v_node.node_kind() == pilot_ast::NodeKind::AdtVariant {
                                    let v_name = arena.interner.resolve(pilot_intern::SymbolId(v_node.lhs));
                                    let p_len = v_node.flags as usize;
                                    if p_len > 0 {
                                        let p_slice = arena.children_slice(v_node.rhs, p_len as u32);
                                        let p_strs: Vec<String> = p_slice
                                            .iter()
                                            .map(|&p_id| {
                                                let p_ty = analysis
                                                    .inferred_types
                                                    .get(&p_id)
                                                    .copied()
                                                    .unwrap_or(pilot_analysis::TypeId::ANY);
                                                analysis.types.display(p_ty)
                                            })
                                            .collect();
                                        variants.push(format!("{}({})", v_name, p_strs.join(", ")));
                                    } else {
                                        variants.push(v_name.to_string());
                                    }
                                }
                            }
                        }
                        let rhs_str = if variants.is_empty() { String::new() } else { format!(" = {}", variants.join(" | ")) };
                        let md = format!("```pilot\ntype {}{}\n```\n\n*Pilot Algebraic Data Type*", name, rhs_str);
                        return make_hover_card(&md);
                    }
                }
                pilot_ast::NodeKind::AdtVariant => {
                    let v_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                    if v_name == txt {
                        let ty_id = analysis
                            .inferred_types
                            .get(&(idx as u32))
                            .copied()
                            .unwrap_or(pilot_analysis::TypeId::ANY);
                        let sig = analysis.types.display(ty_id);
                        let md = format!("```pilot\n(variant) {}: {}\n```\n\n*Pilot ADT Variant Constructor*", v_name, sig);
                        return make_hover_card(&md);
                    }
                }
                _ => {}
            }
        }

        // Check if cursor is directly on an `IdentExpr` with a known flow-sensitive inferred type
        let mut cursor_ident_ty: Option<pilot_analysis::TypeId> = None;
        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == pilot_ast::NodeKind::IdentExpr {
                let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                if name == txt {
                    let span = arena.spans[idx];
                    if byte_offset >= span.start && byte_offset <= span.end {
                        if let Some(&t) = analysis.inferred_types.get(&(idx as u32)) {
                            if t != pilot_analysis::TypeId::ANY {
                                cursor_ident_ty = Some(t);
                                break;
                            }
                        }
                    }
                }
            }
        }

        // 4. Parameters (pick closest preceding ParamDecl)
        let mut best_param: Option<(usize, u32)> = None;
        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == pilot_ast::NodeKind::ParamDecl {
                let p_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                if p_name == txt {
                    let span = arena.spans[idx];
                    let dist = if byte_offset >= span.start {
                        byte_offset - span.start
                    } else {
                        (span.start - byte_offset) + 100_000
                    };
                    if best_param.map_or(true, |(_, b_dist)| dist < b_dist) {
                        best_param = Some((idx, dist));
                    }
                }
            }
        }

        // 5. Variables (let, var, const), Pattern Bindings (PatBind, PatSlice), For loops, Catch clauses
        let mut best_var: Option<(String, pilot_analysis::TypeId, u32, &'static str)> = None;
        for (idx, node) in arena.nodes.iter().enumerate() {
            let nk = node.node_kind();
            let span = arena.spans[idx];
            let dist = if byte_offset >= span.start {
                byte_offset - span.start
            } else {
                (span.start - byte_offset) + 100_000
            };
            match nk {
                pilot_ast::NodeKind::LetStmt | pilot_ast::NodeKind::VarStmt | pilot_ast::NodeKind::ConstDecl if node.flags == 0 => {
                    let var_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                    if var_name == txt {
                        let ty_id = analysis
                            .inferred_types
                            .get(&(idx as u32))
                            .copied()
                            .unwrap_or(pilot_analysis::TypeId::ANY);
                        let kw = match nk {
                            pilot_ast::NodeKind::LetStmt => "let",
                            pilot_ast::NodeKind::VarStmt => "var",
                            pilot_ast::NodeKind::ConstDecl => "const",
                            _ => "let",
                        };
                        if best_var.as_ref().map_or(true, |(_, _, b_dist, _)| dist < *b_dist) {
                            best_var = Some((format!("{} {}", kw, var_name), ty_id, dist, "Pilot Variable"));
                        }
                    }
                }
                pilot_ast::NodeKind::PatBind => {
                    let var_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                    if var_name == txt {
                        let ty_id = analysis
                            .inferred_types
                            .get(&(idx as u32))
                            .copied()
                            .unwrap_or(pilot_analysis::TypeId::ANY);
                        if best_var.as_ref().map_or(true, |(_, _, b_dist, _)| dist < *b_dist) {
                            if node.flags == 2 {
                                best_var = Some((format!("let ..{}", var_name), ty_id, dist, "Pilot Slice Rest Binding"));
                            } else {
                                best_var = Some((format!("let {}", var_name), ty_id, dist, "Pilot Pattern Binding"));
                            }
                        }
                    }
                }
                pilot_ast::NodeKind::PatSlice if node.rhs == 0 && node.lhs != 0 => {
                    let var_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                    if var_name == txt {
                        let ty_id = analysis
                            .inferred_types
                            .get(&(idx as u32))
                            .copied()
                            .unwrap_or(pilot_analysis::TypeId::ANY);
                        if best_var.as_ref().map_or(true, |(_, _, b_dist, _)| dist < *b_dist) {
                            best_var = Some((format!("let ..{}", var_name), ty_id, dist, "Pilot Slice Rest Binding"));
                        }
                    }
                }
                pilot_ast::NodeKind::ForExpr => {
                    let var_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                    if var_name == txt {
                        let elem_ty = if let Some(&iter_node) = arena.for_iter_nodes.get(&(idx as u32)) {
                            let iter_ty = analysis
                                .inferred_types
                                .get(&iter_node.0)
                                .copied()
                                .unwrap_or(pilot_analysis::TypeId::ANY);
                            let pruned = analysis.types.prune(iter_ty);
                            match analysis.types.get(pruned).clone() {
                                pilot_analysis::TypeKind::Collection { element, .. } => analysis.types.prune(element),
                                pilot_analysis::TypeKind::Map { key, value } => {
                                    let _ = (key, value);
                                    pilot_analysis::TypeId::ANY
                                }
                                _ if pruned == pilot_analysis::TypeId::STR => pilot_analysis::TypeId::STR,
                                _ => pilot_analysis::TypeId::I64,
                            }
                        } else {
                            pilot_analysis::TypeId::I64
                        };
                        if best_var.as_ref().map_or(true, |(_, _, b_dist, _)| dist < *b_dist) {
                            best_var = Some((format!("for {}", var_name), elem_ty, dist, "Pilot Loop Binding"));
                        }
                    }
                }
                pilot_ast::NodeKind::CatchClause if node.lhs != 0 => {
                    let var_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                    if var_name == txt {
                        if best_var.as_ref().map_or(true, |(_, _, b_dist, _)| dist < *b_dist) {
                            best_var = Some((format!("catch {}", var_name), pilot_analysis::TypeId::ANY, dist, "Pilot Exception Binding"));
                        }
                    }
                }
                _ => {}
            }
        }

        // Compare closest parameter vs closest variable binding
        match (best_param, best_var) {
            (Some((p_idx, p_dist)), Some((v_prefix, v_ty, v_dist, v_desc))) => {
                if p_dist <= v_dist {
                    let p_ty = cursor_ident_ty.unwrap_or_else(|| {
                        analysis
                            .inferred_types
                            .get(&(p_idx as u32))
                            .copied()
                            .unwrap_or(pilot_analysis::TypeId::ANY)
                    });
                    let md = format!("```pilot\n(parameter) {}: {}\n```\n\n*Pilot Parameter*", txt, analysis.types.display(p_ty));
                    return make_hover_card(&md);
                } else {
                    let final_ty = cursor_ident_ty.unwrap_or(v_ty);
                    let md = format!("```pilot\n{}: {}\n```\n\n*{}*", v_prefix, analysis.types.display(final_ty), v_desc);
                    return make_hover_card(&md);
                }
            }
            (Some((p_idx, _)), None) => {
                let p_ty = cursor_ident_ty.unwrap_or_else(|| {
                    analysis
                        .inferred_types
                        .get(&(p_idx as u32))
                        .copied()
                        .unwrap_or(pilot_analysis::TypeId::ANY)
                });
                let md = format!("```pilot\n(parameter) {}: {}\n```\n\n*Pilot Parameter*", txt, analysis.types.display(p_ty));
                return make_hover_card(&md);
            }
            (None, Some((v_prefix, v_ty, _, v_desc))) => {
                let final_ty = cursor_ident_ty.unwrap_or(v_ty);
                let md = format!("```pilot\n{}: {}\n```\n\n*{}*", v_prefix, analysis.types.display(final_ty), v_desc);
                return make_hover_card(&md);
            }
            (None, None) => {
                if let Some((sig, summary)) = get_builtin_doc(txt) {
                    let md = format!("```pilot\n{}\n```\n\n{}", sig, summary);
                    return make_hover_card(&md);
                }
                if let Some(ident_ty) = cursor_ident_ty {
                    let md = format!("```pilot\n{}: {}\n```\n\n*Pilot Binding*", txt, analysis.types.display(ident_ty));
                    return make_hover_card(&md);
                }
            }
        }

        // 6. Built-in standard library functions
        if let Some((sig, summary)) = get_builtin_doc(txt) {
            let md = format!("```pilot\n{}\n```\n\n{}", sig, summary);
            return make_hover_card(&md);
        }

        // 7. Types and primitives
        if let Some(doc) = get_type_doc(txt) {
            let md = format!("**Pilot Type**: `{}`\n\n{}", txt, doc);
            return make_hover_card(&md);
        }

        // 8. Pilot Keywords (including contextual keywords)
        if let Some((domain, summary)) = get_keyword_doc(txt) {
            let md = format!("**Pilot Keyword**: `{}` ({})\n\n{}", txt, domain, summary);
            return make_hover_card(&md);
        }

        // 9. Polyglot engine blocks
        if let Some(doc) = get_polyglot_doc(txt) {
            let md = format!("**Pilot Polyglot Block**: `{}`\n\n{}", txt, doc);
            return make_hover_card(&md);
        }

        // 10. Top-level definitions in ItemTree fallback
        if let Some(item_tree) = self.db.module_item_tree(file_id) {
            for item in &item_tree.items {
                if item.name == txt {
                    let md = format!("**Pilot Definition**: `{}`\n```pilot\n{}\n```", item.name, item.signature_text);
                    return make_hover_card(&md);
                }
            }
        }

        // 11. Automatic Compiler CST SyntaxKind Fallback (for any new keyword/modifier/operator added to `pilot-syntax`)
        if tok_kind.is_modifier_keyword() {
            let md = format!("**Pilot Modifier Keyword**: `{}` (`SyntaxKind::{:?}`)", txt, tok_kind);
            return make_hover_card(&md);
        }
        if tok_kind.is_keyword() {
            let md = format!("**Pilot Keyword**: `{}` (`SyntaxKind::{:?}`)", txt, tok_kind);
            return make_hover_card(&md);
        }
        if tok_kind.is_operator() {
            let md = format!("**Pilot Operator**: `{}` (`SyntaxKind::{:?}`)", raw_txt, tok_kind);
            return make_hover_card(&md);
        }

        JsonValue::Null
    }

    fn handle_signature_help(&mut self, params: &JsonValue) -> JsonValue {
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

        // 2. AST Functions (via memoized `pilot-db` ModuleAnalysis)
        let mod_analysis = match self.db.module_analysis(file_id) {
            Some(m) => m,
            None => return JsonValue::Null,
        };
        let arena = &mod_analysis.arena;
        let analysis = &mod_analysis.analysis;

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

    fn handle_completion(&mut self, params: &JsonValue) -> JsonValue {
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
            if let Some(mod_analysis) = self.db.module_analysis(file_id) {
                let arena = &mod_analysis.arena;
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

    fn span_of_name(src: &str, span: pilot_span::ByteSpan, name: &str, line_index: &LineIndex) -> JsonValue {
        let start = span.start as usize;
        let end = span.end as usize;
        if start < src.len() && end <= src.len() && start <= end {
            if let Some(sub) = src.get(start..end) {
                if let Some(offset) = sub.find(name) {
                    let sym_start = (start + offset) as u32;
                    let sym_end = sym_start + name.len() as u32;
                    let (sl, sc) = line_index.offset_to_line_col(sym_start);
                    let (el, ec) = line_index.offset_to_line_col(sym_end);
                    return make_range(sl.saturating_sub(1), sc.saturating_sub(1), el.saturating_sub(1), ec.saturating_sub(1));
                }
            }
        }
        let (sl, sc) = line_index.offset_to_line_col(span.start);
        let (el, ec) = line_index.offset_to_line_col(span.start + name.len() as u32);
        make_range(sl.saturating_sub(1), sc.saturating_sub(1), el.saturating_sub(1), ec.saturating_sub(1))
    }

    fn find_member_in_type(
        &self,
        file_id: FileId,
        arena: &pilot_ast::AstArena,
        src: &str,
        line_index: &LineIndex,
        type_name: &str,
        member_name: &str,
    ) -> Option<JsonValue> {
        let uri = self.file_id_to_uri.get(&file_id).cloned().unwrap_or_default();

        for (idx, node) in arena.nodes.iter().enumerate() {
            let is_target_type = match node.node_kind() {
                pilot_ast::NodeKind::ClassDecl
                | pilot_ast::NodeKind::StructDecl
                | pilot_ast::NodeKind::InterfaceDecl
                | pilot_ast::NodeKind::RecordDecl => {
                    arena.interner.resolve(pilot_intern::SymbolId(node.lhs)) == type_name
                }
                _ => false,
            };

            if is_target_type {
                if node.node_kind() == pilot_ast::NodeKind::RecordDecl {
                    if let Some(fields) = arena.record_fields.get(&(idx as u32)) {
                        for (f_sym, _) in fields {
                            if arena.interner.resolve(*f_sym) == member_name {
                                for (p_idx, p_node) in arena.nodes.iter().enumerate() {
                                    if p_node.node_kind() == pilot_ast::NodeKind::ParamDecl && p_node.lhs == f_sym.0 {
                                        let range = Self::span_of_name(src, arena.spans[p_idx], member_name, line_index);
                                        return Some(JsonValue::Object(vec![
                                            ("uri".to_string(), JsonValue::String(uri.clone())),
                                            ("range".to_string(), range),
                                        ]));
                                    }
                                }
                            }
                        }
                    }
                }

                let body_id = pilot_ast::NodeId(node.rhs);
                if body_id.is_some() {
                    let body_node = *arena.node(body_id);
                    if body_node.node_kind() == pilot_ast::NodeKind::BlockExpr {
                        let children = arena.children_slice(body_node.lhs, body_node.rhs);
                        for &child_id in children {
                            let child_node = *arena.node(pilot_ast::NodeId(child_id));
                            let c_span = arena.spans[child_id as usize];
                            match child_node.node_kind() {
                                pilot_ast::NodeKind::FnDecl => {
                                    let fn_name = arena.interner.resolve(pilot_intern::SymbolId(child_node.lhs));
                                    if fn_name == member_name || (fn_name == "this[]" && member_name == "this") {
                                        let range = Self::span_of_name(src, c_span, member_name, line_index);
                                        return Some(JsonValue::Object(vec![
                                            ("uri".to_string(), JsonValue::String(uri.clone())),
                                            ("range".to_string(), range),
                                        ]));
                                    }
                                }
                                pilot_ast::NodeKind::LetStmt
                                | pilot_ast::NodeKind::VarStmt
                                | pilot_ast::NodeKind::ConstDecl => {
                                    let field_name = arena.interner.resolve(pilot_intern::SymbolId(child_node.lhs));
                                    if field_name == member_name {
                                        let range = Self::span_of_name(src, c_span, member_name, line_index);
                                        return Some(JsonValue::Object(vec![
                                            ("uri".to_string(), JsonValue::String(uri.clone())),
                                            ("range".to_string(), range),
                                        ]));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            if node.node_kind() == pilot_ast::NodeKind::ExtendDecl {
                let ext_target = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                if ext_target == type_name {
                    let body_id = pilot_ast::NodeId(node.rhs);
                    if body_id.is_some() {
                        let body_node = *arena.node(body_id);
                        if body_node.node_kind() == pilot_ast::NodeKind::BlockExpr {
                            let children = arena.children_slice(body_node.lhs, body_node.rhs);
                            for &child_id in children {
                                let child_node = *arena.node(pilot_ast::NodeId(child_id));
                                let c_span = arena.spans[child_id as usize];
                                if child_node.node_kind() == pilot_ast::NodeKind::FnDecl {
                                    let fn_name = arena.interner.resolve(pilot_intern::SymbolId(child_node.lhs));
                                    if fn_name == member_name {
                                        let range = Self::span_of_name(src, c_span, member_name, line_index);
                                        return Some(JsonValue::Object(vec![
                                            ("uri".to_string(), JsonValue::String(uri.clone())),
                                            ("range".to_string(), range),
                                        ]));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn find_member_anywhere(
        &self,
        file_id: FileId,
        arena: &pilot_ast::AstArena,
        src: &str,
        line_index: &LineIndex,
        member_name: &str,
    ) -> Option<JsonValue> {
        let uri = self.file_id_to_uri.get(&file_id).cloned().unwrap_or_default();
        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == pilot_ast::NodeKind::FnDecl {
                let fn_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                if fn_name == member_name {
                    let range = Self::span_of_name(src, arena.spans[idx], member_name, line_index);
                    return Some(JsonValue::Object(vec![
                        ("uri".to_string(), JsonValue::String(uri)),
                        ("range".to_string(), range),
                    ]));
                }
            }
        }
        None
    }

    fn find_member_in_workspace(&mut self, current_file_id: FileId, member_name: &str) -> Option<JsonValue> {
        let other_files: Vec<(FileId, String)> = self
            .file_id_to_uri
            .iter()
            .filter(|(&id, _)| id != current_file_id)
            .map(|(&id, u)| (id, u.clone()))
            .collect();

        for (other_id, other_uri) in other_files {
            if let (Some(mod_analysis), Some(src), Some(li)) = (
                self.db.module_analysis(other_id),
                self.db.source_text(other_id),
                self.line_indices.get(&other_id),
            ) {
                let arena = &mod_analysis.arena;
                for (idx, node) in arena.nodes.iter().enumerate() {
                    if node.node_kind() == pilot_ast::NodeKind::FnDecl {
                        let fn_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                        if fn_name == member_name {
                            let range = Self::span_of_name(&src, arena.spans[idx], member_name, li);
                            return Some(JsonValue::Object(vec![
                                ("uri".to_string(), JsonValue::String(other_uri)),
                                ("range".to_string(), range),
                            ]));
                        }
                    }
                }
            }
        }
        None
    }

    fn find_symbol_in_workspace(&mut self, current_file_id: FileId, sym_name: &str) -> Option<JsonValue> {
        let other_files: Vec<(FileId, String)> = self
            .file_id_to_uri
            .iter()
            .filter(|(&id, _)| id != current_file_id)
            .map(|(&id, u)| (id, u.clone()))
            .collect();

        for (other_id, other_uri) in other_files {
            if let (Some(other_mod), Some(o_src), Some(o_li)) = (
                self.db.module_analysis(other_id),
                self.db.source_text(other_id),
                self.line_indices.get(&other_id),
            ) {
                let o_arena = &other_mod.arena;
                for (idx, node) in o_arena.nodes.iter().enumerate() {
                    let matches_ident = match node.node_kind() {
                        pilot_ast::NodeKind::FnDecl
                        | pilot_ast::NodeKind::ClassDecl
                        | pilot_ast::NodeKind::StructDecl
                        | pilot_ast::NodeKind::RecordDecl
                        | pilot_ast::NodeKind::InterfaceDecl
                        | pilot_ast::NodeKind::TypeAdtDecl
                        | pilot_ast::NodeKind::ConstDecl => {
                            o_arena.interner.resolve(pilot_intern::SymbolId(node.lhs)) == sym_name
                        }
                        _ => false,
                    };
                    if matches_ident {
                        let range = Self::span_of_name(&o_src, o_arena.spans[idx], sym_name, o_li);
                        return Some(JsonValue::Object(vec![
                            ("uri".to_string(), JsonValue::String(other_uri)),
                            ("range".to_string(), range),
                        ]));
                    }
                }
            }
        }
        None
    }

    fn discover_workspace_files(&mut self) {
        let roots = self.workspace_folders.clone();
        for root_uri in roots {
            if let Some(root_path) = Self::decode_uri_path(&root_uri) {
                let p = std::path::Path::new(&root_path);
                if p.is_dir() {
                    self.scan_dir_pilot_files(p);
                }
            }
        }
    }

    fn scan_dir_pilot_files(&mut self, dir: &std::path::Path) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
            if file_name.starts_with('.')
                || file_name == "target"
                || file_name == "node_modules"
                || file_name == "bin"
                || file_name == "obj"
            {
                continue;
            }
            if path.is_dir() {
                self.scan_dir_pilot_files(&path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("pilot") {
                let uri = format!("file://{}", path.to_string_lossy());
                self.ensure_file_loaded(&uri);
            }
        }
    }

    fn handle_definition(&mut self, params: &JsonValue) -> JsonValue {
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

        let tok = match token {
            Some(ref t) => t,
            None => return JsonValue::Null,
        };

        let txt = tok.text();
        if txt.is_empty() || tok.kind().is_trivia() || !txt.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
            return JsonValue::Null;
        }

        let uri = self.file_id_to_uri.get(&file_id).cloned().unwrap_or_default();
        let line_index = match self.line_indices.get(&file_id) {
            Some(li) => li.clone(),
            None => return JsonValue::Null,
        };

        let is_after_dot = {
            let tok_start = tok.offset() as usize;
            let before = src.get(..tok_start).unwrap_or("").trim_end();
            before.ends_with('.') || before.ends_with("?.") || before.ends_with("..")
        };

        let local_result = {
            let mod_analysis = match self.db.module_analysis(file_id) {
                Some(m) => m,
                None => return JsonValue::Null,
            };
            let arena = &mod_analysis.arena;
            let analysis = &mod_analysis.analysis;

            let mut res: Option<JsonValue> = None;

            // 1. Member Access Navigation (obj.method, obj.property, obj..method)
            if is_after_dot {
                let receiver_word = {
                    let tok_start = tok.offset() as usize;
                    let before = src.get(..tok_start).unwrap_or("").trim_end();
                    let dot_trimmed = before.trim_end_matches('.').trim_end_matches('?').trim_end();
                    dot_trimmed
                        .split(|c: char| !c.is_alphanumeric() && c != '_')
                        .filter(|s| !s.is_empty())
                        .last()
                        .map(|s| s.to_string())
                };

                let mut target_ty_name: Option<String> = None;
                if let Some(ref r_name) = receiver_word {
                    if r_name == "self" || r_name == "this" {
                        for (c_idx, c_node) in arena.nodes.iter().enumerate() {
                            if matches!(
                                c_node.node_kind(),
                                pilot_ast::NodeKind::ClassDecl
                                    | pilot_ast::NodeKind::StructDecl
                                    | pilot_ast::NodeKind::InterfaceDecl
                                    | pilot_ast::NodeKind::ExtendDecl
                            ) {
                                let c_span = arena.spans[c_idx];
                                if byte_offset >= c_span.start && byte_offset <= c_span.end {
                                    target_ty_name = Some(arena.interner.resolve(pilot_intern::SymbolId(c_node.lhs)).to_string());
                                    break;
                                }
                            }
                        }
                    } else {
                        for (idx, node) in arena.nodes.iter().enumerate() {
                            if matches!(
                                node.node_kind(),
                                pilot_ast::NodeKind::LetStmt
                                    | pilot_ast::NodeKind::VarStmt
                                    | pilot_ast::NodeKind::ParamDecl
                                    | pilot_ast::NodeKind::IdentExpr
                            ) {
                                if arena.interner.resolve(pilot_intern::SymbolId(node.lhs)) == r_name {
                                    if let Some(&ty) = analysis.inferred_types.get(&(idx as u32)) {
                                        let pruned = analysis.types.prune(ty);
                                        match analysis.types.get(pruned) {
                                            pilot_analysis::TypeKind::Primitive(n) => {
                                                target_ty_name = Some(n.clone());
                                                break;
                                            }
                                            pilot_analysis::TypeKind::Adt { name, .. } => {
                                                target_ty_name = Some(name.clone());
                                                break;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(ref type_name) = target_ty_name {
                    if let Some(loc) = self.find_member_in_type(file_id, arena, &src, &line_index, type_name, txt) {
                        res = Some(loc);
                    }
                }

                if res.is_none() {
                    if let Some(loc) = self.find_member_anywhere(file_id, arena, &src, &line_index, txt) {
                        res = Some(loc);
                    }
                }
            }

            // 2. Self / This Navigation
            if res.is_none() && (txt == "self" || txt == "this") {
                for (c_idx, c_node) in arena.nodes.iter().enumerate() {
                    if matches!(
                        c_node.node_kind(),
                        pilot_ast::NodeKind::ClassDecl
                            | pilot_ast::NodeKind::StructDecl
                            | pilot_ast::NodeKind::InterfaceDecl
                            | pilot_ast::NodeKind::ExtendDecl
                    ) {
                        let c_span = arena.spans[c_idx];
                        if byte_offset >= c_span.start && byte_offset <= c_span.end {
                            let name = arena.interner.resolve(pilot_intern::SymbolId(c_node.lhs));
                            let range = Self::span_of_name(&src, c_span, name, &line_index);
                            res = Some(JsonValue::Object(vec![
                                ("uri".to_string(), JsonValue::String(uri.clone())),
                                ("range".to_string(), range),
                            ]));
                            break;
                        }
                    }
                }
            }

            // 3. Local Variables, Parameters, Pattern Bindings, For-Loop Variables
            if res.is_none() {
                let mut best_local_decl: Option<usize> = None;
                for (idx, node) in arena.nodes.iter().enumerate() {
                    let nk = node.node_kind();
                    if matches!(
                        nk,
                        pilot_ast::NodeKind::LetStmt
                            | pilot_ast::NodeKind::VarStmt
                            | pilot_ast::NodeKind::ConstDecl
                            | pilot_ast::NodeKind::ParamDecl
                            | pilot_ast::NodeKind::PatBind
                            | pilot_ast::NodeKind::ForExpr
                    ) {
                        let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                        if name == txt {
                            let span = arena.spans[idx];
                            if span.start <= byte_offset {
                                best_local_decl = Some(idx);
                            } else if best_local_decl.is_none() {
                                best_local_decl = Some(idx);
                            }
                        }
                    }
                }

                if let Some(idx) = best_local_decl {
                    let range = Self::span_of_name(&src, arena.spans[idx], txt, &line_index);
                    res = Some(JsonValue::Object(vec![
                        ("uri".to_string(), JsonValue::String(uri.clone())),
                        ("range".to_string(), range),
                    ]));
                }
            }

            // 4. Functions, Methods, Indexers
            if res.is_none() {
                for (idx, node) in arena.nodes.iter().enumerate() {
                    if node.node_kind() == pilot_ast::NodeKind::FnDecl {
                        let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                        if name == txt || (name == "this[]" && txt == "this") {
                            let range = Self::span_of_name(&src, arena.spans[idx], txt, &line_index);
                            res = Some(JsonValue::Object(vec![
                                ("uri".to_string(), JsonValue::String(uri.clone())),
                                ("range".to_string(), range),
                            ]));
                            break;
                        }
                    }
                }
            }

            // 5. Classes, Structs, Interfaces, Records, Type ADTs, ADT Variants
            if res.is_none() {
                for (idx, node) in arena.nodes.iter().enumerate() {
                    if matches!(
                        node.node_kind(),
                        pilot_ast::NodeKind::ClassDecl
                            | pilot_ast::NodeKind::StructDecl
                            | pilot_ast::NodeKind::InterfaceDecl
                            | pilot_ast::NodeKind::RecordDecl
                            | pilot_ast::NodeKind::TypeAdtDecl
                            | pilot_ast::NodeKind::AdtVariant
                    ) {
                        let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                        if name == txt {
                            let range = Self::span_of_name(&src, arena.spans[idx], txt, &line_index);
                            res = Some(JsonValue::Object(vec![
                                ("uri".to_string(), JsonValue::String(uri.clone())),
                                ("range".to_string(), range),
                            ]));
                            break;
                        }
                    }
                }
            }

            // 6. Fallback to ItemTree
            if res.is_none() {
                if let Some(item_tree) = self.db.module_item_tree(file_id) {
                    for item in &item_tree.items {
                        if item.name == txt {
                            let (line, col) = line_index.offset_to_line_col(tok.offset());
                            let range = make_range(line - 1, col - 1, line - 1, col - 1 + txt.len() as u32);
                            res = Some(JsonValue::Object(vec![
                                ("uri".to_string(), JsonValue::String(uri.clone())),
                                ("range".to_string(), range),
                            ]));
                            break;
                        }
                    }
                }
            }

            res
        };

        if let Some(loc) = local_result {
            return loc;
        }

        // 7. Workspace search fallback
        if is_after_dot {
            if let Some(loc) = self.find_member_in_workspace(file_id, txt) {
                return loc;
            }
        } else {
            if let Some(loc) = self.find_symbol_in_workspace(file_id, txt) {
                return loc;
            }
        }

        JsonValue::Null
    }

    fn handle_references(&mut self, params: &JsonValue) -> JsonValue {
        let (file_id, byte_offset) = match self.get_file_and_offset(params) {
            Some(res) => res,
            None => return JsonValue::Array(Vec::new()),
        };

        let include_decl = params
            .get("context")
            .and_then(|c| c.get("includeDeclaration"))
            .and_then(|b| b.as_bool())
            .unwrap_or(true);

        let green = match self.db.green_cst(file_id) {
            Some(g) => g,
            None => return JsonValue::Array(Vec::new()),
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

        let tok = match token {
            Some(ref t) => t,
            None => return JsonValue::Array(Vec::new()),
        };

        let sym_name = tok.text().to_string();
        if sym_name.is_empty() || tok.kind().is_trivia() || !sym_name.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
            return JsonValue::Array(Vec::new());
        }

        // Determine if sym_name is a local variable/parameter in an enclosing function
        let mut local_scope_span: Option<pilot_span::ByteSpan> = None;
        if let Some(mod_analysis) = self.db.module_analysis(file_id) {
            let arena = &mod_analysis.arena;
            for (idx, node) in arena.nodes.iter().enumerate() {
                if node.node_kind() == pilot_ast::NodeKind::FnDecl {
                    let fn_span = arena.spans[idx];
                    if byte_offset >= fn_span.start && byte_offset <= fn_span.end {
                        let mut is_local = false;
                        let fn_start = node.rhs;
                        let fn_len = node.flags as usize;
                        if fn_len > 0 {
                            let children = arena.children_slice(fn_start, fn_len as u32);
                            for &p_id in &children[1..] {
                                let p_node = *arena.node(pilot_ast::NodeId(p_id));
                                if p_node.node_kind() == pilot_ast::NodeKind::ParamDecl {
                                    if arena.interner.resolve(pilot_intern::SymbolId(p_node.lhs)) == sym_name {
                                        is_local = true;
                                        break;
                                    }
                                }
                            }
                        }
                        if !is_local {
                            for (n_idx, n) in arena.nodes.iter().enumerate() {
                                let n_span = arena.spans[n_idx];
                                if n_span.start >= fn_span.start && n_span.end <= fn_span.end {
                                    match n.node_kind() {
                                        pilot_ast::NodeKind::LetStmt
                                        | pilot_ast::NodeKind::VarStmt
                                        | pilot_ast::NodeKind::PatBind => {
                                            if arena.interner.resolve(pilot_intern::SymbolId(n.lhs)) == sym_name {
                                                is_local = true;
                                                break;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        if is_local {
                            local_scope_span = Some(fn_span);
                            break;
                        }
                    }
                }
            }
        }

        let mut results = Vec::new();
        let mut visited: std::collections::HashSet<(String, u32, u32)> = std::collections::HashSet::new();

        // 1. Collect references from current file
        let current_uri = self.file_id_to_uri.get(&file_id).cloned().unwrap_or_default();
        if let Some(li) = self.line_indices.get(&file_id) {
            for elem in root.descendants() {
                if let SyntaxElement::Token(t) = elem {
                    if !t.kind().is_trivia() && t.text() == sym_name {
                        let offset = t.offset();
                        let len = t.text_len();

                        if let Some(span) = local_scope_span {
                            if offset < span.start || offset + len > span.end {
                                continue;
                            }
                        }

                        if !include_decl && offset == tok.offset() {
                            continue;
                        }

                        let (sl, sc) = li.offset_to_line_col(offset);
                        let (el, ec) = li.offset_to_line_col(offset + len);
                        let s_line = sl.saturating_sub(1);
                        let s_col = sc.saturating_sub(1);
                        let e_line = el.saturating_sub(1);
                        let e_col = ec.saturating_sub(1);

                        if visited.insert((current_uri.clone(), s_line, s_col)) {
                            results.push(JsonValue::Object(vec![
                                ("uri".to_string(), JsonValue::String(current_uri.clone())),
                                ("range".to_string(), make_range(s_line, s_col, e_line, e_col)),
                            ]));
                        }
                    }
                }
            }
        }

        // 2. If not local, collect across all other loaded workspace files
        if local_scope_span.is_none() {
            let other_files: Vec<(FileId, String)> = self
                .file_id_to_uri
                .iter()
                .filter(|(&id, _)| id != file_id)
                .map(|(&id, u)| (id, u.clone()))
                .collect();

            for (other_id, other_uri) in other_files {
                if let (Some(o_green), Some(o_li)) = (self.db.green_cst(other_id), self.line_indices.get(&other_id)) {
                    let o_root = SyntaxNode::new_root(o_green);
                    for elem in o_root.descendants() {
                        if let SyntaxElement::Token(t) = elem {
                            if !t.kind().is_trivia() && t.text() == sym_name {
                                let offset = t.offset();
                                let len = t.text_len();
                                let (sl, sc) = o_li.offset_to_line_col(offset);
                                let (el, ec) = o_li.offset_to_line_col(offset + len);
                                let s_line = sl.saturating_sub(1);
                                let s_col = sc.saturating_sub(1);
                                let e_line = el.saturating_sub(1);
                                let e_col = ec.saturating_sub(1);

                                if visited.insert((other_uri.clone(), s_line, s_col)) {
                                    results.push(JsonValue::Object(vec![
                                        ("uri".to_string(), JsonValue::String(other_uri.clone())),
                                        ("range".to_string(), make_range(s_line, s_col, e_line, e_col)),
                                    ]));
                                }
                            }
                        }
                    }
                }
            }
        }

        JsonValue::Array(results)
    }

    fn handle_implementation(&mut self, params: &JsonValue) -> JsonValue {
        let (file_id, byte_offset) = match self.get_file_and_offset(params) {
            Some(res) => res,
            None => return JsonValue::Array(Vec::new()),
        };

        let green = match self.db.green_cst(file_id) {
            Some(g) => g,
            None => return JsonValue::Array(Vec::new()),
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

        let tok = match token {
            Some(ref t) => t,
            None => return JsonValue::Array(Vec::new()),
        };

        let sym_name = tok.text().to_string();
        if sym_name.is_empty() || tok.kind().is_trivia() || !sym_name.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
            return JsonValue::Array(Vec::new());
        }

        let mut results = Vec::new();
        let all_files: Vec<(FileId, String)> = self
            .file_id_to_uri
            .iter()
            .map(|(&id, u)| (id, u.clone()))
            .collect();

        for (fid, f_uri) in all_files {
            if let (Some(mod_analysis), Some(src), Some(li)) = (
                self.db.module_analysis(fid),
                self.db.source_text(fid),
                self.line_indices.get(&fid),
            ) {
                let arena = &mod_analysis.arena;
                for (idx, node) in arena.nodes.iter().enumerate() {
                    match node.node_kind() {
                        pilot_ast::NodeKind::ClassDecl | pilot_ast::NodeKind::StructDecl => {
                            let name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                            let span = arena.spans[idx];
                            if let Some(slice) = src.get(span.start as usize..span.end as usize) {
                                if slice.contains(&format!(": {}", sym_name))
                                    || slice.contains(&format!(":{}", sym_name))
                                    || slice.contains(&format!("implements {}", sym_name))
                                    || slice.contains(&format!("with {}", sym_name))
                                {
                                    let range = Self::span_of_name(&src, span, name, li);
                                    results.push(JsonValue::Object(vec![
                                        ("uri".to_string(), JsonValue::String(f_uri.clone())),
                                        ("range".to_string(), range),
                                    ]));
                                }
                            }
                        }
                        pilot_ast::NodeKind::ExtendDecl => {
                            let target = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                            if target == sym_name {
                                let span = arena.spans[idx];
                                let range = Self::span_of_name(&src, span, target, li);
                                results.push(JsonValue::Object(vec![
                                    ("uri".to_string(), JsonValue::String(f_uri.clone())),
                                    ("range".to_string(), range),
                                ]));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        JsonValue::Array(results)
    }

    fn handle_prepare_rename(&mut self, params: &JsonValue) -> JsonValue {
        let (file_id, byte_offset) = match self.get_file_and_offset(params) {
            Some(res) => res,
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

        let tok = match token {
            Some(ref t) => t,
            None => return JsonValue::Null,
        };

        let txt = tok.text();
        if tok.kind().is_trivia() || !txt.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
            return JsonValue::Null;
        }

        if (tok.kind().is_keyword() && !tok.kind().is_contextual_keyword())
            || txt == "self"
            || txt == "this"
            || txt == "field"
            || txt == "value"
        {
            return JsonValue::Null;
        }

        let line_index = match self.line_indices.get(&file_id) {
            Some(li) => li,
            None => return JsonValue::Null,
        };

        let offset = tok.offset();
        let len = tok.text_len();
        let (sl, sc) = line_index.offset_to_line_col(offset);
        let (el, ec) = line_index.offset_to_line_col(offset + len);
        let range = make_range(sl.saturating_sub(1), sc.saturating_sub(1), el.saturating_sub(1), ec.saturating_sub(1));

        JsonValue::Object(vec![
            ("range".to_string(), range),
            ("placeholder".to_string(), JsonValue::String(txt.to_string())),
        ])
    }

    fn handle_rename(&mut self, params: &JsonValue) -> JsonValue {
        let (_file_id, _) = match self.get_file_and_offset(params) {
            Some(res) => res,
            None => return JsonValue::Null,
        };

        let new_name = match params.get("newName").and_then(|n| n.as_str()) {
            Some(n) => n.trim(),
            None => return JsonValue::Null,
        };

        if new_name.is_empty() || !is_valid_identifier(new_name) {
            return JsonValue::Null;
        }

        let mut ref_params = params.clone();
        if let JsonValue::Object(ref mut map) = ref_params {
            map.push((
                "context".to_string(),
                JsonValue::Object(vec![("includeDeclaration".to_string(), JsonValue::Bool(true))]),
            ));
        }

        let refs = self.handle_references(&ref_params);
        let ref_array = match refs {
            JsonValue::Array(a) => a,
            _ => return JsonValue::Null,
        };

        if ref_array.is_empty() {
            return JsonValue::Null;
        }

        let mut changes_map: HashMap<String, Vec<JsonValue>> = HashMap::new();
        for loc in ref_array {
            if let JsonValue::Object(fields) = loc {
                let uri_val = fields
                    .iter()
                    .find(|(k, _)| k == "uri")
                    .and_then(|(_, v)| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let range_val = fields
                    .into_iter()
                    .find(|(k, _)| k == "range")
                    .map(|(_, v)| v)
                    .unwrap_or(JsonValue::Null);

                if !uri_val.is_empty() && range_val != JsonValue::Null {
                    let edit = JsonValue::Object(vec![
                        ("range".to_string(), range_val),
                        ("newText".to_string(), JsonValue::String(new_name.to_string())),
                    ]);
                    changes_map.entry(uri_val).or_default().push(edit);
                }
            }
        }

        let mut changes_json = Vec::new();
        for (uri, edits) in changes_map {
            changes_json.push((uri, JsonValue::Array(edits)));
        }

        JsonValue::Object(vec![("changes".to_string(), JsonValue::Object(changes_json))])
    }

    fn handle_document_symbol(&mut self, params: &JsonValue) -> JsonValue {
        let uri = match params.get("textDocument").and_then(|d| d.get("uri")).and_then(|u| u.as_str()) {
            Some(u) => u,
            None => return JsonValue::Array(Vec::new()),
        };
        let file_id = match self.ensure_file_loaded(uri) {
            Some(id) => id,
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

    fn handle_formatting(&mut self, params: &JsonValue) -> JsonValue {
        let uri = match params.get("textDocument").and_then(|d| d.get("uri")).and_then(|u| u.as_str()) {
            Some(u) => u,
            None => return JsonValue::Array(Vec::new()),
        };
        let file_id = match self.ensure_file_loaded(uri) {
            Some(id) => id,
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
        let file_id = match self.ensure_file_loaded(uri) {
            Some(id) => id,
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

                let token_desc: Option<(u32, u32)> = match k {
                    // Keywords (15) and Modifiers (16), with context awareness for member/variable positions
                    k if k.is_keyword() => {
                        let text = token.text();
                        let end_offset = (token.offset() + token.text_len()) as usize;
                        let remainder = src.get(end_offset..).unwrap_or("").trim_start();

                        if prev_non_trivia_kind == SyntaxKind::Dot
                            || prev_non_trivia_kind == SyntaxKind::SafeNav
                            || prev_non_trivia_kind == SyntaxKind::Cascade
                            || prev_non_trivia_kind == SyntaxKind::DotDot
                        {
                            if remainder.starts_with('(') {
                                Some((13, 0)) // method
                            } else {
                                Some((9, 0)) // property
                            }
                        } else if (prev_non_trivia_kind == SyntaxKind::KwLet
                            || prev_non_trivia_kind == SyntaxKind::KwVar
                            || prev_non_trivia_kind == SyntaxKind::KwConst)
                            && !k.is_modifier_keyword()
                        {
                            Some((8, 1)) // variable declaration
                        } else if k.is_modifier_keyword() {
                            Some((16, 0)) // modifier
                        } else if Self::is_pilot_primitive_type(text) {
                            Some((1, 512)) // type (defaultLibrary)
                        } else {
                            Some((15, 0)) // keyword
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
                        } else if prev_non_trivia_kind == SyntaxKind::KwMacro || prev_non_trivia_kind == SyntaxKind::KwDefmacro {
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
                            } else if get_builtin_doc(text).is_some() {
                                Some((12, 512)) // builtin function call + defaultLibrary
                            } else {
                                Some((12, 0)) // function call
                            }
                        } else if get_builtin_doc(text).is_some() {
                            Some((12, 512)) // builtin function identifier + defaultLibrary
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

                    // Single-line Strings (18). Leave FStrStart, TripleStrLit, and RawDslBlock to TextMate so
                    // interpolated `{...}` expressions and embedded polyglot blocks keep their syntax coloring!
                    SyntaxKind::StrLit if !token.text().contains('\n') => Some((18, 0)),

                    // Keyword literal / symbols (10: enumMember)
                    SyntaxKind::KeywordLit => Some((10, 0)),

                    // Numbers & Dimensional Literals (19)
                    SyntaxKind::IntLit
                    | SyntaxKind::HexIntLit
                    | SyntaxKind::BinIntLit
                    | SyntaxKind::OctIntLit
                    | SyntaxKind::FloatLit
                    | SyntaxKind::DurationLit
                    | SyntaxKind::FrequencyLit
                    | SyntaxKind::MemoryLit => Some((19, 0)), // number

                    // Single-line Comments (17)
                    SyntaxKind::LineComment | SyntaxKind::Shebang => Some((17, 0)), // comment
                    SyntaxKind::BlockComment if !token.text().contains('\n') => Some((17, 0)),

                    // Operators (21)
                    k if k.is_operator() => Some((21, 0)), // operator

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

    fn ensure_file_loaded(&mut self, uri: &str) -> Option<FileId> {
        if let Some(&id) = self.uri_to_file_id.get(uri) {
            return Some(id);
        }
        if let Some(path) = Self::decode_uri_path(uri) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let id = self.get_or_create_file_id(uri);
                self.line_indices.insert(id, LineIndex::new(content.as_bytes()));
                self.db.set_source(id, &content);
                return Some(id);
            }
        }
        None
    }

    fn get_file_and_offset(&mut self, params: &JsonValue) -> Option<(FileId, u32)> {
        let uri = params.get("textDocument")?.get("uri")?.as_str()?;
        let file_id = self.ensure_file_loaded(uri)?;
        let pos = params.get("position")?;
        let line = pos.get("line")?.as_u64()? as u32;
        let col = pos.get("character")?.as_u64()? as u32;

        let li = self.line_indices.get(&file_id)?;
        let byte_offset = li.line_col_to_offset(line + 1, col + 1)?;
        Some((file_id, byte_offset))
    }

    fn handle_inlay_hint(&mut self, params: &JsonValue) -> JsonValue {
        let uri = match params.get("textDocument").and_then(|d| d.get("uri")).and_then(|u| u.as_str()) {
            Some(u) => u,
            None => return JsonValue::Array(Vec::new()),
        };
        let file_id = match self.ensure_file_loaded(uri) {
            Some(id) => id,
            None => return JsonValue::Array(Vec::new()),
        };
        let src = match self.db.source_text(file_id) {
            Some(s) => s,
            None => return JsonValue::Array(Vec::new()),
        };

        let mod_analysis = match self.db.module_analysis(file_id) {
            Some(m) => m,
            None => return JsonValue::Array(Vec::new()),
        };
        let arena = &mod_analysis.arena;
        let analysis = &mod_analysis.analysis;

        let mut hints = Vec::new();
        let li = match self.line_indices.get(&file_id) {
            Some(l) => l,
            None => return JsonValue::Array(Vec::new()),
        };

        for (node_idx, node) in arena.nodes.iter().enumerate() {
            if (node.kind == pilot_ast::NodeKind::LetStmt as u16 || node.kind == pilot_ast::NodeKind::VarStmt as u16)
                && node.flags == 0
                && !arena.type_annotations.contains_key(&(node_idx as u32))
            {
                if let Some(&ty_id) = analysis.inferred_types.get(&(node_idx as u32)) {
                    let pruned = analysis.types.prune(ty_id);
                    if pruned == pilot_analysis::TypeId::ANY || pruned == pilot_analysis::TypeId::VOID {
                        continue;
                    }
                    let type_name = analysis.types.display(pruned);
                    let var_name = arena.interner.resolve(pilot_intern::SymbolId(node.lhs));
                    if var_name.is_empty() || var_name == "_" {
                        continue;
                    }

                    let span = arena.spans[node_idx];
                    let stmt_slice = src.get(span.start as usize..(span.end as usize).min(src.len())).unwrap_or("");
                    let hint_offset = if let Some(rel_pos) = stmt_slice.find(var_name) {
                        span.start + (rel_pos + var_name.len()) as u32
                    } else {
                        span.end
                    };
                    let (line, col) = li.offset_to_line_col(hint_offset);
                    hints.push(JsonValue::Object(vec![
                        ("position".to_string(), JsonValue::Object(vec![
                            ("line".to_string(), JsonValue::Number(line.saturating_sub(1) as f64)),
                            ("character".to_string(), JsonValue::Number(col.saturating_sub(1) as f64)),
                        ])),
                        ("label".to_string(), JsonValue::String(format!(": {}", type_name))),
                        ("kind".to_string(), JsonValue::Number(1.0)),
                        ("paddingLeft".to_string(), JsonValue::Bool(false)),
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

        let mut actions = vec![
            JsonValue::Object(vec![
                ("title".to_string(), JsonValue::String("Format Document with pilot fmt".to_string())),
                ("kind".to_string(), JsonValue::String("source.fixAll".to_string())),
                ("command".to_string(), JsonValue::Object(vec![
                    ("title".to_string(), JsonValue::String("Format Document".to_string())),
                    ("command".to_string(), JsonValue::String("editor.action.formatDocument".to_string())),
                ])),
            ]),
        ];

        // Check if a range was selected to offer "Extract Variable" refactoring
        if let Some(range) = params.get("range") {
            let s_line = range.get("start").and_then(|p| p.get("line")).and_then(|l| l.as_u64()).unwrap_or(0);
            let s_char = range.get("start").and_then(|p| p.get("character")).and_then(|c| c.as_u64()).unwrap_or(0);
            let e_line = range.get("end").and_then(|p| p.get("line")).and_then(|l| l.as_u64()).unwrap_or(0);
            let e_char = range.get("end").and_then(|p| p.get("character")).and_then(|c| c.as_u64()).unwrap_or(0);

            if s_line != e_line || s_char != e_char {
                let extract_action = JsonValue::Object(vec![
                    ("title".to_string(), JsonValue::String("Refactor: Extract Local Variable (`let extracted = ...`)".to_string())),
                    ("kind".to_string(), JsonValue::String("refactor.extract.variable".to_string())),
                    ("edit".to_string(), JsonValue::Object(vec![
                        ("changes".to_string(), JsonValue::Object(vec![
                            (uri.to_string(), JsonValue::Array(vec![
                                JsonValue::Object(vec![
                                    ("range".to_string(), range.clone()),
                                    ("newText".to_string(), JsonValue::String("extracted".to_string())),
                                ]),
                            ])),
                        ])),
                    ])),
                ]);
                actions.push(extract_action);
            }
        }

        JsonValue::Array(actions)
    }

    fn emit_diagnostics_for_file(&mut self, file_id: FileId, uri: &str) {
        let raw_diags = diagnostics_query(&mut self.db, file_id);

        let mut diags_json = Vec::new();
        if let Some(li) = self.line_indices.get(&file_id) {
            for (span, sev_code, _code, message) in raw_diags {
                let (sl, sc) = li.offset_to_line_col(span.start);
                let (el, ec) = li.offset_to_line_col(span.end.max(span.start));
                let s_line = sl.saturating_sub(1);
                let s_col = sc.saturating_sub(1);
                let e_line = el.saturating_sub(1);
                let e_col = ec.saturating_sub(1);

                diags_json.push(JsonValue::Object(vec![
                    ("range".to_string(), make_range(s_line, s_col, e_line, e_col)),
                    ("severity".to_string(), JsonValue::Number(sev_code as f64)),
                    ("message".to_string(), JsonValue::String(message)),
                    ("source".to_string(), JsonValue::String("pilot".to_string())),
                ]));
            }
        }

        let notif = JsonValue::Object(vec![
            ("jsonrpc".to_string(), JsonValue::String("2.0".to_string())),
            ("method".to_string(), JsonValue::String(pilot_db::DIAGNOSTICS_QUERY_LSP_METHOD.to_string())),
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

fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return false;
        }
    }
    !matches!(
        name,
        "def"
            | "class"
            | "struct"
            | "interface"
            | "record"
            | "extend"
            | "type"
            | "enum"
            | "let"
            | "var"
            | "const"
            | "return"
            | "if"
            | "else"
            | "while"
            | "for"
            | "in"
            | "break"
            | "continue"
            | "yield"
            | "match"
            | "try"
            | "catch"
            | "finally"
            | "except"
            | "import"
            | "from"
            | "as"
            | "true"
            | "false"
            | "null"
            | "self"
            | "this"
            | "field"
            | "value"
    )
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

fn extract_word_at(src: &str, offset: usize) -> Option<String> {
    if src.is_empty() {
        return None;
    }
    let bytes = src.as_bytes();
    let idx = offset.min(bytes.len().saturating_sub(1));
    let is_ident_char = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let pos = if is_ident_char(bytes[idx]) {
        idx
    } else if idx > 0 && is_ident_char(bytes[idx - 1]) {
        idx - 1
    } else {
        return None;
    };
    let mut start = pos;
    while start > 0 && is_ident_char(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = pos + 1;
    while end < bytes.len() && is_ident_char(bytes[end]) {
        end += 1;
    }
    let word = &src[start..end];
    if word.chars().next().map_or(false, |c| c.is_ascii_alphabetic() || c == '_') {
        Some(word.to_string())
    } else {
        None
    }
}

fn detect_call_quote_at(src: &str, offset: usize) -> Option<(&'static str, &'static str, &'static str)> {
    let start = offset.saturating_sub(3);
    let end = (offset + 4).min(src.len());
    let window = src.get(start..end)?;
    let rel = offset - start;

    const QUOTES: &[(&str, &str, &str, &str)] = &[
        ("!!(", "!!(fn(args))", "Deep Inline AST Splice (Mode 2)", "Inlines and beta-reduces the target function's AST at compile time, capturing free variables and folding constants."),
        ("!&(", "!&(fn(args))", "Zero-Heap Call Frame Descriptor (Mode 3)", "Synthesizes a deterministic stack-allocated call frame descriptor with 8-byte aligned parameter offsets for deferred zero-allocation dispatch."),
        ("!^(", "!^(fn(args))", "Symbolic Autodiff Dual Graph (Mode 4)", "Projects the function call into a forward/reverse-mode symbolic differentiation graph with automatic Common Subexpression Elimination (CSE)."),
        ("!@(", "!@(fn(args))", "Topological Reactive Signal Node (Mode 5)", "Wraps the call as a rank-ordered reactive signal node that automatically recomputes when upstream state dependencies change."),
        ("!?(","!?(fn(args))", "Transactional Effect Plan (Mode 6)", "Captures state mutations as a reversible transactional effect diff log that can be inspected, committed, or rolled back."),
        ("!(", "!(fn(args))", "Shallow Call Descriptor (Mode 1)", "Captures the function pointer and eagerly evaluated argument tuple as a first-class reified call descriptor without executing the body."),
    ];

    for &(pat, syntax, title, desc) in QUOTES {
        if let Some(pos) = window.find(pat) {
            if rel >= pos && rel <= pos + pat.len() {
                return Some((syntax, title, desc));
            }
        }
    }
    None
}

fn get_operator_doc(op: &str) -> Option<(&'static str, &'static str)> {
    OPERATOR_DOCS.iter().find(|(k, _)| *k == op).map(|(_, v)| *v)
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

const OPERATOR_DOCS: &[(&str, (&str, &str))] = &[
    ("|>", ("Pipe Operator", "`lhs |> fn(args)` — Passes `lhs` as the first argument to the right-hand function or lambda pipeline stage.")),
    ("..", ("Range / Spread / Cascade Operator", "`start..end` creates a half-open range; `..list` spreads elements into a C# collection expression `[..a, ..b]`; `obj..method()` cascades mutations on the receiver.")),
    ("..=", ("Inclusive Range Operator", "`start..=end` — Creates an inclusive integer range from `start` through `end`.")),
    ("..<", ("Half-Open Range Operator", "`start..<end` — Creates a half-open range excluding `end`.")),
    ("?.", ("Safe Navigation Operator", "`obj?.member` — Evaluates `member` only if `obj` is non-null (`Some`), otherwise short-circuits to `None`.")),
    ("??", ("Null Coalescing Operator", "`opt ?? fallback` — Unwraps an `Option[T]` if `Some`, or evaluates to `fallback` if `None`/`null`.")),
    ("??=", ("Null Coalescing Assignment", "`target ??= fallback` — Assigns `fallback` to `target` only if `target` is currently `None`/`null`.")),
    ("?!", ("Catch Recovery Operator", "`fallible_expr ?! fallback` — Unwraps an `Ok`/`Some` value or recovers inline with `fallback` on `Err`/`None`.")),
    (":=", ("Walrus Binding Operator", "`pat := expr` — Evaluates `expr`, binds it to `pat` in the current scope, and yields the bound value.")),
    ("=>", ("Fat Arrow", "`(x: T) => expr` or `case pat => expr` — Defines a C#-style lambda body or pattern match arm.")),
    ("->", ("Return Type / Typestate Transition Arrow", "`def fn() -> RetType` or `StateA -> StateB` — Specifies a function return type or typestate transition.")),
    ("<-", ("Channel Send / Receive Arrow", "`ch <- val` or `val <- ch` — Sends or receives a message across a concurrent CSP `Channel`.")),
    ("<=>", ("Three-Way Spaceship Comparison", "`a <=> b` — Returns `-1` if `a < b`, `0` if `a == b`, and `1` if `a > b`.")),
    ("**", ("Exponentiation Operator", "`base ** exp` — Raises `base` to the power of `exp`.")),
    ("//", ("Floor Division Operator", "`a // b` — Divides `a` by `b` and truncates toward negative infinity.")),
    ("==", ("Equality Operator", "`a == b` — Structural equality comparison.")),
    ("!=", ("Inequality Operator", "`a != b` — Structural inequality comparison.")),
    ("<=", ("Less-Than-Or-Equal Operator", "`a <= b` — Relational comparison.")),
    (">=", ("Greater-Than-Or-Equal Operator", "`a >= b` — Relational comparison.")),
    ("&&", ("Logical AND Operator", "`a && b` — Short-circuiting boolean conjunction.")),
    ("||", ("Logical OR Operator", "`a || b` — Short-circuiting boolean disjunction.")),
    ("<<", ("Bitwise Left Shift", "`a << bits` — Shifts bits of `a` left by `bits` positions.")),
    (">>", ("Bitwise Right Shift", "`a >> bits` — Shifts bits of `a` right by `bits` positions.")),
    ("+=", ("Add-Assign Operator", "`a += b` — In-place addition assignment.")),
    ("-=", ("Subtract-Assign Operator", "`a -= b` — In-place subtraction assignment.")),
    ("*=", ("Multiply-Assign Operator", "`a *= b` — In-place multiplication assignment.")),
    ("/=", ("Divide-Assign Operator", "`a /= b` — In-place division assignment.")),
    ("%=", ("Modulo-Assign Operator", "`a %= b` — In-place remainder assignment.")),
    ("?", ("Try Propagate / Optional Type Suffix", "`expr?` propagates `Err`/`None` to the caller; `T?` denotes `Option[T]`.")),
    ("@", ("Decorator / Typestate Attachment", "`@derive(...)` attaches compile-time macros; `Type@State` specifies an active typestate.")),
];

const BUILTIN_DOCS: &[(&str, &str, &str)] = &[
    ("println", "println(val: any) -> void", "Prints values to standard output followed by a newline."),
    ("print", "print(val: any) -> void", "Prints values to standard output without a trailing newline."),
    ("panic", "panic(msg: str) -> never", "Aborts program execution immediately with an unrecoverable error message."),
    ("assert", "assert(condition: bool, msg: str = \"\") -> void", "Asserts that condition is true; aborts with an assertion failure if false."),
    ("len", "len(collection: any) -> int", "Returns the number of elements in a list, map, string, or collection."),
    ("range", "range(start: int, end: int, step: int = 1) -> List[int]", "Generates an iterable sequence of integers from `start` up to `end`."),
    ("typeof", "typeof(val: any) -> str", "Returns the runtime type name of the given value."),
    ("type_of", "type_of(val: any) -> str", "Returns the runtime type name of the given value (alias for typeof)."),
    ("sqrt", "sqrt(x: float) -> float", "Returns the non-negative square root of a floating-point number."),
    ("sin", "sin(x: float) -> float", "Computes the sine of a floating-point angle in radians."),
    ("cos", "cos(x: float) -> float", "Computes the cosine of a floating-point angle in radians."),
    ("tan", "tan(x: float) -> float", "Computes the tangent of a floating-point angle in radians."),
    ("exp", "exp(x: float) -> float", "Computes the natural exponential function e^x."),
    ("ln", "ln(x: float) -> float", "Computes the natural logarithm (base e) of a positive number."),
    ("log", "log(x: float) -> float", "Computes the natural logarithm (base e) of a positive number."),
    ("floor", "floor(x: float) -> float", "Returns the largest integer less than or equal to x."),
    ("ceil", "ceil(x: float) -> float", "Returns the smallest integer greater than or equal to x."),
    ("pow", "pow(base: float, exponent: float) -> float", "Raises a number to the specified power."),
    ("isnan", "isnan(x: float) -> bool", "Returns true if the float value is NaN (Not a Number)."),
    ("isinf", "isinf(x: float) -> bool", "Returns true if the float value is positive or negative infinity."),
    ("is_some", "is_some(opt: Option[T]) -> bool", "Returns true if the option contains a present value."),
    ("is_none", "is_none(opt: Option[T]) -> bool", "Returns true if the option is none."),
    ("unwrap", "unwrap(opt_or_res: any) -> any", "Extracts the contained value, aborting with a panic if none or error."),
    ("is_ok", "is_ok(res: Result[T, E]) -> bool", "Returns true if the result represents success."),
    ("is_err", "is_err(res: Result[T, E]) -> bool", "Returns true if the result represents an error."),
    ("unwrap_err", "unwrap_err(res: Result[T, E]) -> E", "Extracts the contained error value, aborting with a panic if ok."),
    ("append", "append(collection: any, item: any) -> void", "Appends an element to the end of a list or buffer."),
    ("sum", "sum(collection: any) -> int | float", "Calculates the sum of all elements in the collection."),
    ("min", "min(a: any, b: any) -> any", "Returns the lesser of two values, or smallest item in a collection."),
    ("max", "max(a: any, b: any) -> any", "Returns the greater of two values, or largest item in a collection."),
    ("abs", "abs(val: int | float) -> int | float", "Returns the absolute magnitude of a numeric value."),
    ("keys", "keys(map: Map[K, V]) -> List[K]", "Returns a list containing all keys in a map."),
    ("values", "values(map: Map[K, V]) -> List[V]", "Returns a list containing all values in a map."),
    ("filter", "filter(predicate: (T) => bool, list: List[T]) -> List[T]", "Filters collection elements matching the predicate function."),
    ("map", "map(transform: (T) => U, list: List[T]) -> List[U]", "Applies transformation function to each element of a collection."),
    ("zip", "zip(a: List[T], b: List[U]) -> List[(T, U)]", "Combines corresponding elements from two collections into a list of tuples."),
    ("enumerate", "enumerate(list: List[T]) -> List[(int, T)]", "Returns an indexed sequence pairing zero-based indices with elements."),
    ("reversed", "reversed(list: List[T]) -> List[T]", "Returns a reversed sequence of elements."),
    ("sorted", "sorted(list: List[T]) -> List[T]", "Returns a new list with elements sorted in ascending order."),
    ("Some", "Some(val: T) -> Option[T]", "Constructs an `Option[T]` containing a present value `val`."),
    ("None", "None: Option[T]", "Represents the absence of a value in `Option[T]`."),
    ("Ok", "Ok(val: T) -> Result[T, E]", "Constructs a successful `Result[T, E]` containing `val`."),
    ("Err", "Err(err: E) -> Result[T, E]", "Constructs a failed `Result[T, E]` containing error `err`."),
    ("Channel", "Channel(capacity: int = 0) -> Channel[T]", "Creates a concurrent CSP message-passing channel with optional buffer capacity."),
    ("Chan", "Chan(capacity: int = 0) -> Channel[T]", "Creates a concurrent CSP message-passing channel with optional buffer capacity."),
    ("old", "old(expr: T) -> T", "Captures the pre-state value of `expr` upon function entry for use in `ensures` postcondition contracts."),
    ("malloc", "malloc(size: int) -> *void", "Allocates unmanaged heap memory of the given byte size."),
    ("free", "free(ptr: *void) -> void", "Frees previously allocated unmanaged heap memory."),
    ("memset", "memset(dest: *void, val: int, count: int) -> *void", "Fills memory block with the given byte value."),
    ("clock_gettime", "clock_gettime(clock_id: int) -> int", "Returns high-resolution system clock timestamp in nanoseconds."),
    ("puts", "puts(s: str) -> int", "Writes a C string to standard output followed by a newline."),
    ("printf", "printf(fmt: str, ...) -> int", "Formats and prints data to standard output via C ABI."),
    ("sprintf", "sprintf(buf: *u8, fmt: str, ...) -> int", "Writes formatted output into a character buffer."),
    ("fopen", "fopen(path: str, mode: str) -> *FILE", "Opens a file stream with the specified mode."),
    ("fclose", "fclose(stream: *FILE) -> int", "Closes an open file stream."),
    ("fputs", "fputs(s: str, stream: *FILE) -> int", "Writes a string to the specified file stream."),
    ("fgets", "fgets(buf: *u8, n: int, stream: *FILE) -> *u8", "Reads up to `n - 1` characters from a file stream."),
    ("strcmp", "strcmp(a: str, b: str) -> int", "Compares two null-terminated strings lexicographically."),
    ("strlen", "strlen(s: str) -> int", "Returns the byte length of a null-terminated C string."),
    ("system", "system(cmd: str) -> int", "Executes a host shell command and returns its exit code."),
    ("exit", "exit(code: int) -> never", "Terminates the process immediately with the given status code."),
    ("parallel_map", "parallel_map(items: List[T], fn: (T) => U) -> List[U]", "Executes `fn` across `items` in parallel on the work-stealing thread pool."),
    ("lazy", "lazy(thunk: () => T) -> Lazy[T]", "Creates a memoized on-demand lazy computation evaluated at most once."),
    ("json", "json", "Built-in high-performance SIMD JSON serializer and parser module."),
    ("__name__", "__name__: str", "Special module attribute evaluating to `\"__main__\"` when executed as the entry script."),
    ("cons", "cons(head: any, tail: any) -> List[any]", "Constructs a homoiconic S-expression pair or prepends `head` to `tail`."),
    ("car", "car(pair: List[any]) -> any", "Returns the head (first element) of a homoiconic S-expression list or pair."),
    ("cdr", "cdr(pair: List[any]) -> List[any]", "Returns the tail (remaining elements) of a homoiconic S-expression list or pair."),
    ("atom", "atom(val: any) -> bool", "Returns `true` if `val` is an atomic scalar value rather than a cons cell or list."),
    ("venv", "venv", "Built-in virtual environment and isolated package manager controller."),
    ("sizeof_type", "sizeof_type(type_name: str) -> int", "Returns the compile-time C-ABI byte size of the specified type."),
    ("fail", "fail(err: E) -> T ! E", "Constructs an error return value for a fallible `T ! E` function."),
    ("shared_buffer", "shared_buffer(name: str, count: int, dtype: symbol) -> SharedBuffer", "Allocates a cross-language zero-copy shared memory buffer with a 24-byte C-ABI header."),
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
    ("sql", "Embedded SQL query block with compile-time schema and parameter verification."),
    ("postgres", "Embedded PostgreSQL query block."),
    ("postgresql", "Embedded PostgreSQL query block."),
    ("mysql", "Embedded MySQL query block."),
    ("sqlite", "Embedded SQLite query block."),
];

const TYPE_DOCS: &[(&str, &str)] = &[
    ("str", "UTF-8 string with SSO (Small String Optimization, <= 23 bytes inline, zero allocations) and Perceus refcounting."),
    ("string", "UTF-8 string with SSO (Small String Optimization, <= 23 bytes inline, zero allocations) and Perceus refcounting."),
    ("int", "64-bit signed two's complement integer (`i64`)."),
    ("i64", "64-bit signed integer."),
    ("i32", "32-bit signed integer."),
    ("i16", "16-bit signed integer."),
    ("i8", "8-bit signed integer."),
    ("isize", "Pointer-sized signed integer (`i64` on 64-bit targets)."),
    ("uint", "64-bit unsigned integer (`u64`)."),
    ("u64", "64-bit unsigned integer."),
    ("u32", "32-bit unsigned integer."),
    ("u16", "16-bit unsigned integer."),
    ("u8", "8-bit unsigned byte."),
    ("usize", "Pointer-sized unsigned integer (`u64` on 64-bit targets)."),
    ("byte", "8-bit unsigned byte (`u8`)."),
    ("char", "Unicode scalar character value (`u32`)."),
    ("float", "64-bit IEEE 754 double-precision floating-point number (`f64`)."),
    ("double", "64-bit IEEE 754 double-precision floating-point number (`f64`)."),
    ("decimal", "128-bit high-precision decimal floating-point number."),
    ("f64", "64-bit IEEE 754 double-precision floating-point number."),
    ("f32", "32-bit IEEE 754 single-precision floating-point number."),
    ("f16", "16-bit IEEE 754 half-precision floating-point number."),
    ("f8", "8-bit quantized floating-point number for ML/tensor workloads."),
    ("bool", "Boolean logical value (`true` or `false`)."),
    ("void", "Represents the absence of a value or void return type."),
    ("unit", "Unit type representing a single trivial value `()`."),
    ("any", "Dynamic row-polymorphic open type capable of holding any Pilot value."),
    ("dynamic", "Dynamically dispatched type with runtime member resolution."),
    ("symbol", "Compile-time interned keyword/symbol identifier (`:name`)."),
    ("keyword", "Compile-time interned keyword/symbol identifier (`:name`)."),
    ("bytes", "Immutable contiguous byte buffer."),
    ("bytearray", "Mutable contiguous byte array."),
    ("never", "Bottom type representing a computation that never completes or halts."),
    ("List", "Dynamic growable array with O(1) amortized append and Perceus in-place reuse."),
    ("list", "Dynamic growable array (`List[T]`) with Perceus in-place reuse."),
    ("Map", "Robin Hood hashed key-value map with cache-coherent flat storage."),
    ("map", "Robin Hood hashed key-value map (`Map[K, V]`)."),
    ("dict", "Robin Hood hashed key-value dictionary (`Map[K, V]`)."),
    ("Set", "High-performance hash set with O(1) lookup and deduplication."),
    ("set", "High-performance hash set (`Set[T]`)."),
    ("Tuple", "Fixed-size ordered heterogeneous collection of values."),
    ("tuple", "Fixed-size ordered heterogeneous collection of values."),
    ("Array", "Fixed-capacity contiguous sequence with zero heap allocation overhead."),
    ("Option", "Optional value type representing `Some(T)` or `None`."),
    ("Result", "Error handling type representing either `Ok(T)` or `Err(E)`."),
    ("Duration", "Physical time duration (`ns`, `us`, `ms`, `s`) verified by dimensional analysis."),
    ("Frequency", "Physical frequency (`Hz`, `kHz`, `MHz`, `GHz`) verified by dimensional analysis."),
    ("Span", "Non-allocating contiguous memory view slice across arrays or buffers."),
    ("Memory", "Low-level unmanaged memory block or capacity unit (`B`, `KB`, `MB`, `GB`)."),
    ("SliceBuffer", "Fast zero-copy slicing buffer backed by linear memory."),
    ("SharedBuffer", "Thread-safe reference-counted shared byte buffer."),
    ("RangeWindow", "Lightweight sliding iterator window over numeric ranges."),
    ("NativeArray", "Raw C-ABI compatible contiguous array layout."),
    ("NativeMap", "High-throughput native Robin Hood hash table."),
    ("Table", "Relational in-memory table supporting native LINQ/SQL query operators."),
    ("Dataset", "Columnar dataset collection for vectorized query execution."),
    ("Relation", "Typed relational projection over structured records."),
    ("AstNode", "16-byte compact uniform abstract syntax tree node representation."),
    ("Error", "Standard runtime error or exception value."),
    ("Exception", "Base exception type for `try` / `catch` blocks."),
];

const KEYWORD_DOCS: &[(&str, (&str, &str))] = &[
    // Core bindings & declarations
    ("let", ("Core / Immutability", "`let <name>[: <Type>] = <expr>` — Declares an immutable lexical binding.")),
    ("var", ("Core / Mutability", "`var <name>[: <Type>] = <expr>` — Declares a mutable lexical variable.")),
    ("const", ("Core / Constants", "`const <name>[: <Type>] = <expr>` — Declares a compile-time constant.")),
    ("def", ("Declarations", "`def <name>(<params>) [-> <ret>] { ... }` — Declares a function, method, or generator.")),
    ("fn", ("Declarations", "`fn <name>(<params>) [-> <ret>] { ... }` — Declares a function or anonymous closure.")),
    ("lambda", ("Declarations", "`(x: int) => x * 2` or `lambda(x) => ...` — Declares an inline anonymous function.")),
    ("class", ("OOP", "`class <Name> [< Base] { ... }` — Declares an object-oriented class with fields, properties, and methods.")),
    ("struct", ("Value Types", "`struct <Name> { ... }` — Declares a contiguous value-type structure.")),
    ("record", ("Data Types", "`record <Name>(<field>: <Type>, ...)` — Declares an immutable structural record with automatic equality and deconstruction.")),
    ("interface", ("Contracts", "`interface <Name> { ... }` — Declares a structural or nominal interface contract.")),
    ("protocol", ("Contracts", "`protocol <Name> { ... }` — Declares a communication or behavioral protocol contract.")),
    ("trait", ("Traits", "`trait <Name> { ... }` — Declares reusable behavioral trait methods.")),
    ("extend", ("Extensions", "`extend <Type> { ... }` — Adds extension methods, operators, or protocol conformance to an existing type.")),
    ("extension", ("Extensions", "`extension <Type> { ... }` — Declares an extension block adding methods to `<Type>`.")),
    ("type", ("Types", "`type <Name> = <TypeExpr | Variants>` — Declares a type alias or algebraic data type (ADT).")),
    ("enum", ("Types", "`enum <Name> { <Variants> }` — Declares a sum type enumeration or tagged union.")),
    ("actor", ("Concurrency", "`actor <Name> { ... }` — Declares an isolated concurrent actor entity with sequential message processing.")),
    ("state", ("Typestate", "`state <Name> { ... }` — Declares a compile-time verified typestate or state machine node.")),
    ("property", ("OOP", "`property <Name>: <Type> { get { ... } set(v) { ... } }` — Declares a computed or backed property.")),
    ("get", ("Accessors", "`get { ... }` — Defines the getter accessor of a property.")),
    ("set", ("Accessors", "`set(val) { ... }` — Defines the setter accessor of a property or collection.")),
    ("operator", ("Overloading", "`operator <op>(lhs: T, rhs: U) -> R { ... }` — Overloads an operator for custom types.")),
    // Modifiers & ownership
    ("pure", ("Effects & Modifiers", "`pure def ...` — Proves at compile time that the function has no side effects and is eligible for Static Demotion (`M19`).")),
    ("comptime", ("Metaprogramming", "`comptime { ... }` — Evaluates the expression or block at compile time with zero runtime cost.")),
    ("mut", ("Ownership & Mutability", "`mut` — Marks a reference, parameter, or binding as mutable.")),
    ("immut", ("Ownership & Mutability", "`immut` — Marks a binding or parameter as deeply immutable.")),
    ("ref", ("Ownership & References", "`ref <param>` — Passes a value by reference without copying.")),
    ("public", ("Visibility", "`public` — Makes a declaration visible across all modules.")),
    ("private", ("Visibility", "`private` — Restricts visibility to the enclosing class, struct, or module.")),
    ("protected", ("Visibility", "`protected` — Restricts visibility to the declaring class and its subclasses.")),
    ("internal", ("Visibility", "`internal` — Restricts visibility to the current package/crate.")),
    ("static", ("Modifiers", "`static` — Associates a member with the type itself rather than an instance.")),
    ("partial", ("Modifiers", "`partial class/struct` — Allows splitting a type definition across multiple declarations.")),
    ("steal", ("Perceus Ownership", "`steal <param>` — Transfers unique ownership for Perceus in-place memory reuse.")),
    ("extern", ("FFI", "`extern \"C\" def ...` — Declares an external foreign function interface binding.")),
    // Control flow & pattern matching
    ("if", ("Control Flow", "`if <cond> { ... } [elif <cond> { ... }] [else { ... }]` — Conditional branch expression.")),
    ("elif", ("Control Flow", "`elif <cond> { ... }` — Chained else-if conditional branch.")),
    ("else", ("Control Flow", "`else { ... }` — Fallback branch for `if`, `guard`, `for`, or `try`.")),
    ("then", ("Control Flow", "`if <cond> then <expr> else <expr>` — Inline conditional expression branch.")),
    ("while", ("Control Flow", "`while <cond> { ... }` — Executes loop body while `<cond>` evaluates to `true`.")),
    ("for", ("Control Flow", "`for <pat> in <iterable> { ... }` — Iterates over ranges, lists, maps, channels, or generators.")),
    ("in", ("Operators & Loops", "`<item> in <collection>` — Tests element membership or binds loop variables in `for` loops.")),
    ("each", ("Iteration", "`each` — Iterates over every element in a collection or stream.")),
    ("repeat", ("Control Flow", "`repeat <count> { ... }` — Repeats a block a fixed number of times.")),
    ("loop", ("Control Flow", "`loop { ... }` — Infinite loop terminated explicitly via `break` or `return`.")),
    ("break", ("Control Flow", "`break ['label] [<expr>]` — Exits the enclosing(or labeled) loop.")),
    ("continue", ("Control Flow", "`continue ['label]` — Advances immediately to the next iteration of the enclosing (or labeled) loop.")),
    ("return", ("Control Flow", "`return [<expr>]` — Returns a value from the enclosing function.")),
    ("yield", ("Coroutines", "`yield <expr>` — Emits a value from a generator or cooperative coroutine.")),
    ("pass", ("Control Flow", "`pass` — No-op placeholder statement.")),
    ("match", ("Pattern Matching", "`match <scrutinee> { case <pat> => <body_or_expr> }` — Exhaustive Maranget-verified pattern matching.")),
    ("case", ("Pattern Matching", "`case <pat> [if/when <guard>] => <expr>` — Pattern arm inside a `match` or `select` block.")),
    ("when", ("Pattern Matching", "`case <pat> when <cond> => ...` — Guard condition on a pattern match arm.")),
    ("is", ("Type & Pattern Matching", "`<expr> is <Pattern>` — Tests whether `<expr>` matches a type or destructuring ADT pattern (e.g. `s is Circle(r)`).")),
    ("as", ("Type Casting", "`<expr> as <Type>` — Converts or casts `<expr>` to `<Type>`, or aliases an import.")),
    // Safety, contracts & deterministic cleanup
    ("guard", ("Preconditions & Narrowing", "`guard <cond> else { <diverging_block> }` — Enforces an invariant and narrows nullable/union types in downstream code.")),
    ("defer", ("Deterministic Cleanup", "`defer { ... }` — Schedules deterministic LIFO cleanup when the current lexical scope exits.")),
    ("with", ("Resource & Record Update", "`with (<resource>) { ... }` or `rec with { field: val }` — Scoped resource management or functional record update.")),
    ("context", ("Implicit Context", "`context <Name>` — Declares or provides a scoped implicit context parameter.")),
    ("requires", ("Design by Contract", "`requires <bool_expr>` — Precondition contract verified statically by interval analysis and checked at entry.")),
    ("ensures", ("Design by Contract", "`ensures <bool_expr>` — Postcondition contract verified upon function return (may reference `result` and `old(...)`).")),
    ("contracts", ("Design by Contract", "`contracts { ... }` — Groups formal precondition, postcondition, and invariant specifications.")),
    ("assert", ("Verification", "`assert(<cond>)` — Verifies a runtime and static assertion invariant.")),
    // Error handling
    ("try", ("Error Handling", "`try { ... } catch <err> { ... } [finally { ... }]` — Structured exception and error handling block.")),
    ("catch", ("Error Handling", "`catch <err> { ... }` — Captures and recovers from exceptions thrown in a `try` block.")),
    ("finally", ("Error Handling", "`finally { ... }` — Executes guaranteed cleanup after `try`/`catch`.")),
    ("raise", ("Error Handling", "`raise <error_expr>` — Raises an exception or error value.")),
    ("throw", ("Error Handling", "`throw <error_expr>` — Throws an exception or error value.")),
    // Concurrency & Actors
    ("async", ("Concurrency", "`async { ... }` or `async def ...` — Creates an asynchronous task or coroutine.")),
    ("await", ("Concurrency", "`await <task>` — Suspends current fiber until the asynchronous task completes.")),
    ("spawn", ("Concurrency", "`spawn { ... }` — Spawns a lightweight concurrent fiber on the work-stealing scheduler.")),
    ("parallel", ("Data Parallelism", "`parallel { ... }` or `parallel for ...` — Executes loop iterations or tasks in parallel across worker threads.")),
    ("chan", ("Concurrency", "`chan[T](capacity)` — Creates a typed CSP communication channel.")),
    ("channel", ("Concurrency", "`channel[T](capacity)` — Creates a typed CSP communication channel.")),
    ("select", ("Concurrency & Queries", "`select { ... }` — Multiplexes across ready channels, or projects columns in a relational query.")),
    ("timeout", ("Concurrency & Realtime", "`timeout(<Duration>) { ... }` — Bounds an operation or `select` wait by a maximum duration.")),
    ("atomic", ("Synchronization", "`atomic { ... }` — Executes memory operations with hardware atomic ordering.")),
    ("sync", ("Synchronization", "`sync { ... }` — Synchronized mutual-exclusion block.")),
    ("lock", ("Synchronization", "`lock(<mutex>) { ... }` — Acquires a scoped lock for the duration of the block.")),
    ("isolate", ("Concurrency", "`isolate { ... }` — Executes a block in an isolated heap region with zero shared mutable state.")),
    // Real-time systems & deterministic memory
    ("realtime", ("Real-Time Systems", "`realtime { ... }` — Marks a block or function as subject to deterministic real-time scheduling constraints.")),
    ("hard_rt", ("Real-Time Systems", "`hard_rt` — Enforces strict compile-time WCET, `noalloc`, and zero-jitter guarantees.")),
    ("soft_rt", ("Real-Time Systems", "`soft_rt` — Schedules with real-time priority while permitting graceful deadline miss handlers.")),
    ("deadline", ("Real-Time Systems", "`deadline(250ms) { ... }` — Enforces a temporal completion deadline on the enclosed block.")),
    ("within", ("Real-Time Systems", "`within(10ms) { ... }` — Constrains execution to complete within the specified duration window.")),
    ("on_missed", ("Real-Time Systems", "`on_missed { ... }` — Handler executed immediately if a `deadline` or `periodic` window is missed.")),
    ("fallback", ("Real-Time Systems", "`fallback { ... }` — Provides a deterministic degraded result when a deadline or primary computation fails.")),
    ("period", ("Real-Time Systems", "`period(16ms)` — Specifies the cycle period for a real-time task.")),
    ("periodic", ("Real-Time Systems", "`periodic(60Hz) { ... }` — Executes a control loop at a deterministic fixed frequency or period.")),
    ("priority", ("Real-Time Systems", "`priority(99) { ... }` — Assigns a real-time OS scheduler priority level.")),
    ("jitter", ("Real-Time Systems", "`jitter(50us)` — Specifies maximum allowable timing jitter for a periodic task.")),
    ("wcet", ("Real-Time Systems", "`wcet(500us)` — Declares or asserts the Worst-Case Execution Time bound.")),
    ("bounded", ("Deterministic Memory", "`bounded(64KB) { ... }` — Restricts dynamic allocations inside the block to a fixed memory budget.")),
    ("noalloc", ("Deterministic Memory", "`noalloc { ... }` — Statically forbids heap allocations inside the block or function.")),
    ("stackalloc", ("Deterministic Memory", "`stackalloc(size)` — Allocates a zero-overhead buffer directly on the current stack frame.")),
    ("region", ("Deterministic Memory", "`region { ... }` — Allocates objects inside a scoped arena region freed in O(1) on scope exit.")),
    // Logical operators (word forms)
    ("and", ("Logical Operators", "`a and b` — Short-circuiting logical conjunction (`&&`).")),
    ("or", ("Logical Operators", "`a or b` — Short-circuiting logical disjunction (`||`).")),
    ("not", ("Logical Operators", "`not expr` — Logical boolean negation (`!`).")),
    // Modules, FFI & reflection
    ("import", ("Modules", "`import <module> [as <alias>]` — Imports a module namespace.")),
    ("from", ("Modules & Queries", "`from <module> import <items>` or `from <row> in <table>` — Imports symbols or begins a LINQ/relational query.")),
    ("module", ("Modules", "`module <Name>` — Declares a module namespace.")),
    ("lib", ("Modules", "`lib <Name>` — Declares or links a library target.")),
    ("global", ("Scope", "`global <name>` — References or declares a module-global variable.")),
    ("delegate", ("Types & Callbacks", "`delegate <Name>(<params>) -> <Ret>` — Declares a typed function pointer / callback signature.")),
    ("params", ("Parameters", "`params <args>: List[T]` — Accepts a variable number of arguments as a collection.")),
    ("args", ("Parameters", "`args` — Variadic positional arguments tuple.")),
    ("kwargs", ("Parameters", "`kwargs` — Variadic keyword arguments map.")),
    ("nameof", ("Reflection", "`nameof(<symbol>)` — Evaluates at compile time to the string name of a variable, type, or member.")),
    ("adapter", ("Interop", "`adapter <Name>` — Defines a zero-cost protocol or type conversion adapter.")),
    ("converter", ("Interop", "`converter <From> -> <To>` — Defines a custom type conversion rule.")),
    ("default", ("Core", "`default` or `default(T)` — Default match branch or default value of type `T`.")),
    // Relational / LINQ / SQL keywords
    ("where", ("Queries & Generics", "`where <condition>` — Filters query rows or constrains generic type parameters.")),
    ("join", ("Relational Queries", "`join <table2> on <cond>` — Relational join across two collections or tables.")),
    ("inner", ("Relational Queries", "`inner join` — Inner relational join.")),
    ("left", ("Relational Queries", "`left join` — Left outer relational join.")),
    ("right", ("Relational Queries", "`right join` — Right outer relational join.")),
    ("full", ("Relational Queries", "`full outer join` — Full outer relational join.")),
    ("cross", ("Relational Queries", "`cross join` — Cartesian product relational join.")),
    ("outer", ("Relational Queries", "`outer join` — Outer relational join modifier.")),
    ("on", ("Relational Queries", "`join ... on <a.id == b.id>` — Join key equality condition.")),
    ("group", ("Relational Queries", "`group by <key>` — Groups query rows by key expression.")),
    ("by", ("Relational Queries", "`group by` / `order by` — Specifies grouping or sorting key.")),
    ("having", ("Relational Queries", "`having <aggregate_cond>` — Filters grouped query partitions.")),
    ("order", ("Relational Queries", "`order by <expr> [asc|desc]` — Sorts query results.")),
    ("asc", ("Relational Queries", "`order by <expr> asc` — Ascending sort order.")),
    ("ascending", ("Relational Queries", "`order by <expr> ascending` — Ascending sort order.")),
    ("desc", ("Relational Queries", "`order by <expr> desc` — Descending sort order.")),
    ("descending", ("Relational Queries", "`order by <expr> descending` — Descending sort order.")),
    ("limit", ("Relational Queries", "`limit <count>` — Restricts the maximum number of returned rows.")),
    ("offset", ("Relational Queries", "`offset <count>` — Skips the first `<count>` rows in a query.")),
    ("into", ("Relational Queries", "`select ... into <target>` — Redirects query results into a new binding or table.")),
    ("distinct", ("Relational Queries", "`select distinct ...` — Deduplicates query result rows.")),
    ("union", ("Relational & Type Queries", "`union [all]` — Combines rows from two queries or forms a union type.")),
    ("all", ("Relational Queries", "`union all` — Retains duplicate rows in a relational union.")),
    ("intersect", ("Relational Queries", "`intersect` — Returns rows common to both queries.")),
    ("except", ("Relational Queries", "`except` — Returns rows in the first query that are absent from the second.")),
    ("difference", ("Relational Queries", "`difference` — Relational set difference.")),
    ("index", ("Relational & Data", "`index <Name> on <Table>(<Cols>)` — Declares a lookup index.")),
    ("indexed", ("Relational & Data", "`indexed` — Marks a field or collection as indexed for O(1)/O(log n) lookup.")),
    ("lookup", ("Relational & Data", "`lookup` — Performs an indexed key lookup.")),
    ("table", ("Relational & Data", "`table <Name> { ... }` — Declares an in-memory or persistent relational table.")),
    ("schema", ("Relational & Data", "`schema <Name> { ... }` — Declares a typed database or data schema.")),
    ("insert", ("Relational & Data", "`insert into <Table> values (...)` — Inserts rows into a table.")),
    ("update", ("Relational & Data", "`update <Table> set <col> = <val>` — Updates rows in a table.")),
    ("delete", ("Relational & Data", "`delete from <Table> where <cond>` — Deletes matching rows from a table.")),
    ("returning", ("Relational & Data", "`returning <cols>` — Returns modified rows from an `insert`, `update`, or `delete` statement.")),
    ("conflict", ("Relational & Data", "`on conflict do ...` — Specifies upsert conflict resolution policy.")),
    ("do", ("Control Flow & Queries", "`do { ... }` or `on conflict do nothing` — Executes a block or query action.")),
    ("nothing", ("Relational Queries", "`on conflict do nothing` — Ignores conflicting rows during insert.")),
    ("over", ("Window Functions", "`<agg>() over (partition by ...)` — Defines a SQL/relational window frame.")),
    ("partition", ("Window Functions", "`partition by <expr>` — Partitions rows for window function evaluation.")),
    ("window", ("Window Functions", "`window <Name> as (...)` — Declares a named relational window specification.")),
    ("transaction", ("Transactions", "`transaction { ... }` — Executes database or state operations inside an ACID transaction.")),
    ("commit", ("Transactions", "`commit` — Commits the active transaction.")),
    ("rollback", ("Transactions", "`rollback [to <savepoint>]` — Rolls back the active transaction.")),
    ("savepoint", ("Transactions", "`savepoint <Name>` — Creates a nested rollback savepoint inside a transaction.")),
    ("to", ("Core & Queries", "`rollback to <savepoint>` — Target specifier.")),
    ("connect", ("Database", "`connect <uri>` — Establishes a database or service connection.")),
    ("database", ("Database", "`database <Name>` — Declares or selects a database context.")),
    ("model", ("Data Modeling", "`model <Name> { ... }` — Declares an ORM/relational entity model.")),
    // Metaprogramming, Macros & DSLs
    ("dsl", ("Metaprogramming", "`dsl <Name> { ... }` — Defines a custom embedded domain-specific language block.")),
    ("domain", ("Dimensional Analysis", "`domain <Name> { ... }` — Declares a physical or domain-specific type system.")),
    ("unit", ("Dimensional Analysis", "`unit <Name>` — Declares a physical measurement unit (`m`, `s`, `kg`, `Hz`).")),
    ("macro", ("Metaprogramming", "`macro <name>(<params>) { ... }` — Declares a hygienic syntax-transforming macro.")),
    ("defmacro", ("Metaprogramming", "`defmacro <name>(<params>) { ... }` — Declares a compile-time AST macro.")),
    ("defkeyword", ("Metaprogramming", "`defkeyword <name>(<params>) { ... }` — Defines a custom language keyword macro.")),
    ("quote", ("Metaprogramming", "`quote { ... }` — Captures Pilot code as a compile-time `AstNode` tree.")),
    ("quasiquote", ("Metaprogramming", "`quasiquote { ... }` — Captures an AST template supporting `unquote` (`$`) interpolation.")),
    ("splice", ("Metaprogramming", "`splice(<ast>)` — Splices a quoted AST node into the surrounding syntax tree.")),
    ("unquote", ("Metaprogramming", "`unquote(<expr>)` — Evaluates `<expr>` and inserts its resulting AST into a `quasiquote`.")),
    ("unquote_splicing", ("Metaprogramming", "`unquote_splicing(<list>)` — Splices a list of AST nodes into a `quasiquote` block.")),
    // Literals & built-in values
    ("true", ("Boolean Literal", "`true: bool` — Boolean truth value.")),
    ("false", ("Boolean Literal", "`false: bool` — Boolean falsehood value.")),
    ("some", ("Option Literal", "`some(val): Option[T]` — Constructs an `Option[T]` containing `val`.")),
    ("none", ("Option Literal", "`none: Option[T]` — Represents no value in `Option[T]`.")),
    ("null", ("Null Literal", "`null` — Represents an absent or null reference (`Option[T]`).")),
    ("undefined", ("Literal", "`undefined` — Represents an uninitialized or undefined polyglot value.")),
    ("unsafe", ("Low-Level", "`unsafe { ... }` — Enables raw pointer arithmetic and unchecked FFI intrinsics.")),
    ("simd", ("Vectorization", "`simd { ... }` — Explicit SIMD hardware vector lane block.")),
];
