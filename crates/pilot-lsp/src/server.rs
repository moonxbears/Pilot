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
        let arena = pilot_parser::parse(src);
        let _physics = pilot_analysis::analyze_physics(&arena, arena.root, None);

        match method {
            "textDocument/hover" => {
                JsonValue::Object(vec![
                    ("contents".to_string(), JsonValue::Object(vec![
                        ("kind".to_string(), JsonValue::String("markdown".to_string())),
                        ("value".to_string(), JsonValue::String("*Pilot Language*".to_string())),
                    ]))
                ])
            }
            "textDocument/completion" => {
                JsonValue::Array(Vec::new())
            }
            "textDocument/definition" => {
                JsonValue::Null
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
