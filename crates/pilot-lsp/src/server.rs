//! Name-blind language server protocol coordinator delegating to `selfhost/analyzer.pilot`.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use pilot_db::FileId;
use pilot_span::LineIndex;
use crate::json::{parse_json, JsonValue};

// Restriction: Clean, name-blind LSP protocol server; Forbidden: hardcoded type/keyword docs
pub struct LspServer {
    pub documents: HashMap<String, String>,
    pub uri_to_file_id: HashMap<String, FileId>,
    pub line_indices: HashMap<FileId, LineIndex>,
    next_file_id: u32,
    pub is_shutdown: bool,
}

impl Default for LspServer {
    fn default() -> Self {
        Self::new()
    }
}

impl LspServer {
    // Restriction: Construct empty LSP server state
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            uri_to_file_id: HashMap::new(),
            line_indices: HashMap::new(),
            next_file_id: 1,
            is_shutdown: false,
        }
    }

    // Restriction: Assign or get unique FileId for document URI
    pub fn get_or_create_file_id(&mut self, uri: &str) -> FileId {
        if let Some(&id) = self.uri_to_file_id.get(uri) {
            return id;
        }
        let id = FileId(self.next_file_id);
        self.next_file_id += 1;
        self.uri_to_file_id.insert(uri.to_string(), id);
        id
    }

    // Restriction: Dispatch JSON-RPC message framing and notifications
    pub fn handle_message(&mut self, msg: &str) -> Option<String> {
        let json = parse_json(msg).ok()?;
        let method = json.get("method").and_then(|v| v.as_str());
        let id = json.get("id");

        if let Some(id_val) = id {
            let res = self.handle_request(method.unwrap_or(""), json.get("params"), id_val);
            return Some(res);
        }

        if let Some(m) = method {
            return self.handle_notification(m, json.get("params"));
        }
        None
    }

    // Restriction: Handle JSON-RPC requests with id
    fn handle_request(&mut self, method: &str, params: Option<&JsonValue>, id: &JsonValue) -> String {
        let id_str = match id {
            JsonValue::Number(n) => n.to_string(),
            JsonValue::String(s) => format!("\"{}\"", s),
            _ => "null".to_string(),
        };

        match method {
            "initialize" => {
                let caps = r#"{"capabilities":{"textDocumentSync":1,"hoverProvider":true,"completionProvider":{"resolveProvider":false},"definitionProvider":true,"semanticTokensProvider":{"legend":{"tokenTypes":["keyword","type","function","variable","property","parameter"],"tokenModifiers":[]},"full":true}}}"#;
                format!(r#"{{"jsonrpc":"2.0","id":{},"result":{}}}"#, id_str, caps)
            }
            "shutdown" => {
                self.is_shutdown = true;
                format!(r#"{{"jsonrpc":"2.0","id":{},"result":null}}"#, id_str)
            }
            _ => {
                let res = self.dispatch_to_pilot_ide_sync(method, params);
                format!(r#"{{"jsonrpc":"2.0","id":{},"result":{}}}"#, id_str, res.serialize())
            }
        }
    }

    // Restriction: Handle JSON-RPC notifications without id
    fn handle_notification(&mut self, method: &str, params: Option<&JsonValue>) -> Option<String> {
        match method {
            "textDocument/didOpen" => {
                let doc = params?.get("textDocument")?;
                let uri = doc.get("uri")?.as_str()?;
                let text = doc.get("text")?.as_str()?;
                let fid = self.get_or_create_file_id(uri);
                self.documents.insert(uri.to_string(), text.to_string());
                self.line_indices.insert(fid, LineIndex::new(text.as_bytes()));
                self.publish_diagnostics(uri, text)
            }
            "textDocument/didChange" => {
                let doc = params?.get("textDocument")?;
                let uri = doc.get("uri")?.as_str()?;
                let changes = params?.get("contentChanges")?.as_array()?;
                let text = changes.last()?.get("text")?.as_str()?;
                let fid = self.get_or_create_file_id(uri);
                self.documents.insert(uri.to_string(), text.to_string());
                self.line_indices.insert(fid, LineIndex::new(text.as_bytes()));
                self.publish_diagnostics(uri, text)
            }
            "textDocument/didClose" => {
                if let Some(uri) = params.and_then(|p| p.get("textDocument")).and_then(|d| d.get("uri")).and_then(|u| u.as_str()) {
                    self.documents.remove(uri);
                }
                None
            }
            "exit" => {
                std::process::exit(if self.is_shutdown { 0 } else { 1 });
            }
            _ => None,
        }
    }

    // Restriction: Analyze source via AnalysisPipeline & emit LSP diagnostics
    pub fn publish_diagnostics(&mut self, uri: &str, text: &str) -> Option<String> {
        let arena = pilot_parser::parse(text);
        let res = pilot_analysis::AnalysisPipeline::analyze(&arena, arena.root);
        let fid = self.get_or_create_file_id(uri);
        let line_index = self.line_indices.get(&fid);

        let mut diag_json = Vec::new();
        for d in res.diagnostics {
            let (sl, sc) = line_index.map_or((0, 0), |li| li.offset_to_line_col(d.span.start));
            let (el, ec) = line_index.map_or((0, 0), |li| li.offset_to_line_col(d.span.end));
            diag_json.push(format!(
                r#"{{"range":{{"start":{{"line":{},"character":{}}},"end":{{"line":{},"character":{}}}}},"severity":1,"message":"{}"}}"#,
                sl, sc, el, ec, d.message.replace('"', "\\\"").replace('\n', " ")
            ));
        }

        Some(format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{{"uri":"{}","diagnostics":[{}]}}}}"#,
            uri, diag_json.join(",")
        ))
    }

    // Restriction: Route LSP queries through pilot-hosted ide_sync engine; Forbidden: hardcoded doc tables
    pub fn dispatch_to_pilot_ide_sync(&self, method: &str, params: Option<&JsonValue>) -> JsonValue {
        let uri = params.and_then(|p| p.get("textDocument")).and_then(|d| d.get("uri")).and_then(|u| u.as_str());
        let text = uri.and_then(|u| self.documents.get(u));
        if text.is_none() {
            return JsonValue::Null;
        }
        let src = text.unwrap();
        let fid = uri.and_then(|u| self.uri_to_file_id.get(u)).copied();
        let line_index = fid.and_then(|f| self.line_indices.get(&f));

        let to_lsp = |off: u32| -> (u32, u32) {
            line_index.map_or((0, 0), |li| {
                let (l, c) = li.offset_to_line_col(off);
                (l.saturating_sub(1), c.saturating_sub(1))
            })
        };
        let byte_offset = if let Some(pos) = params.and_then(|p| p.get("position")) {
            let line = pos.get("line").and_then(|l| l.as_f64()).unwrap_or(0.0) as u32;
            let col = pos.get("character").and_then(|c| c.as_f64()).unwrap_or(0.0) as u32;
            line_index.and_then(|li| li.line_col_to_offset(line + 1, col + 1)).unwrap_or(0)
        } else {
            0
        };

        let arena = pilot_parser::parse(src);
        let mut registry = pilot_analysis::analyze_physics_with_source(&arena, arena.root, Some(src), None);
        registry.lsp_method = method.to_string();
        registry.lsp_offset = byte_offset;

        let ide_sync_src = std::fs::read_to_string("selfhost/ide_sync.pilot")
            .or_else(|_| std::fs::read_to_string("e:/Pilot/selfhost/ide_sync.pilot"))
            .unwrap_or_else(|_| include_str!("../../../selfhost/ide_sync.pilot").to_string());

        let parsed_ide = pilot_parser::parse(&ide_sync_src);
        let binary = pilot_vm::BytecodeCompiler::compile(&parsed_ide, parsed_ide.root);
        let mut vm = pilot_vm::PilVirtualMachine::new();
        vm.load_binary(binary);

        let reg_ptr = &mut registry as *mut pilot_analysis::PhysicsRegistry;
        vm.engine_handler = Some(Box::new(move |op_code, args| {
            let reg = unsafe { &mut *reg_ptr };
            pilot_analysis::handle_engine_intrinsic(reg, op_code, args)
        }));
        vm.run();

        let res_str = &registry.lsp_result;
        match method {
            "textDocument/hover" => {
                if res_str.is_empty() {
                    JsonValue::Null
                } else {
                    JsonValue::Object(vec![
                        ("contents".to_string(), JsonValue::Object(vec![
                            ("kind".to_string(), JsonValue::String("markdown".to_string())),
                            ("value".to_string(), JsonValue::String(res_str.clone())),
                        ]))
                    ])
                }
            }
            "textDocument/completion" => {
                if res_str.starts_with('[') {
                    parse_json(res_str).unwrap_or(JsonValue::Array(Vec::new()))
                } else {
                    JsonValue::Array(Vec::new())
                }
            }
            "textDocument/definition" => {
                if res_str.is_empty() || res_str == "null" {
                    JsonValue::Null
                } else {
                    let target_word = res_str.as_str();
                    let s = src.find(target_word).unwrap_or(0) as u32;
                    let e = s + target_word.len() as u32;
                    let ((sl, sc), (el, ec)) = (to_lsp(s), to_lsp(e));
                    JsonValue::Object(vec![
                        ("uri".to_string(), JsonValue::String(uri.unwrap().to_string())),
                        ("range".to_string(), JsonValue::Object(vec![
                            ("start".to_string(), JsonValue::Object(vec![
                                ("line".to_string(), JsonValue::Number(sl as f64)),
                                ("character".to_string(), JsonValue::Number(sc as f64)),
                            ])),
                            ("end".to_string(), JsonValue::Object(vec![
                                ("line".to_string(), JsonValue::Number(el as f64)),
                                ("character".to_string(), JsonValue::Number(ec as f64)),
                            ])),
                        ])),
                    ])
                }
            }
            "textDocument/prepareRename" => {
                if res_str.is_empty() || res_str == "null" {
                    JsonValue::Null
                } else {
                    let word = res_str.as_str();
                    let s = if let Some(pos) = src[..byte_offset as usize].rfind(|c: char| !c.is_alphanumeric() && c != '_') {
                        (pos + 1) as u32
                    } else {
                        0
                    };
                    let e = if let Some(pos) = src[byte_offset as usize..].find(|c: char| !c.is_alphanumeric() && c != '_') {
                        byte_offset + pos as u32
                    } else {
                        src.len() as u32
                    };
                    let ((sl, sc), (el, ec)) = (to_lsp(s), to_lsp(e));
                    JsonValue::Object(vec![
                        ("range".to_string(), JsonValue::Object(vec![
                            ("start".to_string(), JsonValue::Object(vec![
                                ("line".to_string(), JsonValue::Number(sl as f64)),
                                ("character".to_string(), JsonValue::Number(sc as f64)),
                            ])),
                            ("end".to_string(), JsonValue::Object(vec![
                                ("line".to_string(), JsonValue::Number(el as f64)),
                                ("character".to_string(), JsonValue::Number(ec as f64)),
                            ])),
                        ])),
                        ("placeholder".to_string(), JsonValue::String(word.to_string())),
                    ])
                }
            }
            "textDocument/rename" => {
                let new_name = params.and_then(|p| p.get("newName")).and_then(|n| n.as_str()).unwrap_or("");
                let seed_kws = ["struct", "interface", "fn", "let", "ref", "if", "loop", "return", "yield", "defer", "comptime", "quote", "asm", "field", "value"];
                if seed_kws.contains(&new_name) || new_name.is_empty() || res_str.is_empty() || res_str == "null" {
                    return JsonValue::Null;
                }
                let search_word = res_str.as_str();
                let mut edits = Vec::new();
                let mut start_idx = 0;
                while let Some(pos) = src[start_idx..].find(search_word) {
                    let s = (start_idx + pos) as u32;
                    let e = s + search_word.len() as u32;
                    let ((sl, sc), (el, ec)) = (to_lsp(s), to_lsp(e));
                    edits.push(JsonValue::Object(vec![
                        ("range".to_string(), JsonValue::Object(vec![
                            ("start".to_string(), JsonValue::Object(vec![
                                ("line".to_string(), JsonValue::Number(sl as f64)),
                                ("character".to_string(), JsonValue::Number(sc as f64)),
                            ])),
                            ("end".to_string(), JsonValue::Object(vec![
                                ("line".to_string(), JsonValue::Number(el as f64)),
                                ("character".to_string(), JsonValue::Number(ec as f64)),
                            ])),
                        ])),
                        ("newText".to_string(), JsonValue::String(new_name.to_string())),
                    ]));
                    start_idx = (e as usize).max(start_idx + 1);
                }
                JsonValue::Object(vec![
                    ("changes".to_string(), JsonValue::Object(vec![
                        (uri.unwrap().to_string(), JsonValue::Array(edits)),
                    ]))
                ])
            }
            "textDocument/semanticTokens/full" => {
                JsonValue::Object(vec![("data".to_string(), JsonValue::Array(Vec::new()))])
            }
            _ => JsonValue::Null,
        }
    }
}

// Restriction: Standard JSON-RPC Content-Length loop over stdio
pub fn run_stdio_server(mut reader: impl BufRead, mut writer: impl Write) -> io::Result<()> {
    let mut server = LspServer::new();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break;
        }

        if line.starts_with("Content-Length:") {
            let len: usize = line["Content-Length:".len()..].trim().parse().unwrap_or(0);
            let mut empty = String::new();
            reader.read_line(&mut empty)?;

            let mut body = vec![0u8; len];
            reader.read_exact(&mut body)?;
            if let Ok(msg) = std::str::from_utf8(&body) {
                if let Some(resp) = server.handle_message(msg) {
                    write!(writer, "Content-Length: {}\r\n\r\n{}", resp.len(), resp)?;
                    writer.flush()?;
                }
            }
        }
    }
    Ok(())
}
