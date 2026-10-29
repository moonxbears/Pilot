//! Milestone 32.3: Zero-GC Standalone Native Language Server (`pilot-lsp`).
//!
//! Provides:
//! - Complete JSON-RPC 2.0 language server protocol implementation.
//! - Sub-millisecond incremental didChange synchronization.
//! - CST semantic tokens, hover cards, dot-completion, go-to-definition, and formatting.

pub mod json;
pub mod server;

pub use json::{parse_json, JsonValue};
pub use server::LspServer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_server_initialize_did_open_hover_and_semantic_tokens() {
        let mut server = LspServer::new();

        // 1. Initialize
        let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
        let init_resp = server.handle_message(init_req).expect("Expected initialize response");
        assert!(init_resp.contains("semanticTokensProvider"));
        assert!(init_resp.contains("hoverProvider"));
        assert!(init_resp.contains("completionProvider"));

        // 2. didOpen
        let uri = "file:///workspace/test.pilot";
        let code = "def compute(x: i64) -> i64 {\n    return x + 1;\n}\n";
        let did_open_notif = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            uri,
            code.replace('\n', "\\n")
        );
        let resp = server.handle_message(&did_open_notif);
        assert!(resp.is_none(), "Notifications do not produce direct responses");

        // 3. Hover over 'def' keyword at line 0, character 1
        let hover_req = format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":0,"character":1}}}}}}"#,
            uri
        );
        let hover_resp = server.handle_message(&hover_req).expect("Expected hover response");
        assert!(hover_resp.contains("Pilot Keyword"));
        assert!(hover_resp.contains("def"));

        // 4. Semantic tokens full
        let sem_req = format!(
            r#"{{"jsonrpc":"2.0","id":3,"method":"textDocument/semanticTokens/full","params":{{"textDocument":{{"uri":"{}"}}}}}}"#,
            uri
        );
        let sem_resp = server.handle_message(&sem_req).expect("Expected semantic tokens response");
        assert!(sem_resp.contains("data"));

        // 5. Completion
        let comp_req = format!(
            r#"{{"jsonrpc":"2.0","id":4,"method":"textDocument/completion","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":1,"character":4}}}}}}"#,
            uri
        );
        let comp_resp = server.handle_message(&comp_req).expect("Expected completion response");
        assert!(comp_resp.contains("return"));
        assert!(comp_resp.contains("let"));
    }

    #[test]
    fn test_lsp_incremental_did_change_and_definition() {
        let mut server = LspServer::new();
        let uri = "file:///workspace/test.pilot";
        let code = "def calculate(x: i64) -> i64 {\n    return x * 2;\n}\n";

        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            uri,
            code.replace('\n', "\\n")
        );
        server.handle_message(&did_open);

        // Incremental didChange: edit inside body
        let did_change = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{}","version":2}},"contentChanges":[{{"range":{{"start":{{"line":1,"character":13}},"end":{{"line":1,"character":14}}}},"text":"10"}}]}}}}"#,
            uri
        );
        server.handle_message(&did_change);

        // Go to definition on calculate at line 0, char 5
        let def_req = format!(
            r#"{{"jsonrpc":"2.0","id":10,"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":0,"character":5}}}}}}"#,
            uri
        );
        let def_resp = server.handle_message(&def_req).expect("Expected definition response");
        assert!(def_resp.contains(uri));
    }
}
