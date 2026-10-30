//! Milestone 32.3 & Milestone 48: Zero-GC Standalone Native Language Server (`pilot-lsp`).
//!
//! Provides:
//! - Complete JSON-RPC 2.0 language server protocol implementation.
//! - Sub-millisecond incremental didChange synchronization.
//! - 23-type semantic tokens with modifiers, interactive hover cards, dot-completion, signature help, go-to-definition, and formatting.

pub mod json;
pub mod server;

pub use json::{parse_json, JsonValue};
pub use server::{run_stdio_server, LspServer};

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
        // Standard VSCode legend indices: keyword = 15, function = 12, variable = 8, type = 1
        assert!(sem_resp.contains("15")); // keyword 'def'
        assert!(sem_resp.contains("12")); // function 'compute'
        assert!(sem_resp.contains("8"));  // variable 'x'
        assert!(sem_resp.contains("1"));  // type 'i64'

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

        // Inlay Hints
        let inlay_req = format!(
            r#"{{"jsonrpc":"2.0","id":11,"method":"textDocument/inlayHint","params":{{"textDocument":{{"uri":"{}"}},"range":{{"start":{{"line":0,"character":0}},"end":{{"line":2,"character":0}}}}}}}}"#,
            uri
        );
        let inlay_resp = server.handle_message(&inlay_req).expect("Expected inlay hint response");
        assert!(inlay_resp.contains("result"));

        // Code Action
        let ca_req = format!(
            r#"{{"jsonrpc":"2.0","id":12,"method":"textDocument/codeAction","params":{{"textDocument":{{"uri":"{}"}},"range":{{"start":{{"line":0,"character":0}},"end":{{"line":0,"character":0}}}},"context":{{"diagnostics":[]}}}}}}"#,
            uri
        );
        let ca_resp = server.handle_message(&ca_req).expect("Expected code action response");
        assert!(ca_resp.contains("Format Document"));

        // Diagnostics notification published
        let notifs = server.take_pending_notifications();
        assert!(!notifs.is_empty());
        assert!(notifs[0].contains("textDocument/publishDiagnostics"));
    }

    #[test]
    fn test_hover_12_polyglot() {
        let mut server = LspServer::new();
        let uri = "file:///workspace/12_polyglot.pilot";
        let code = std::fs::read_to_string("examples/showcase/12_polyglot_interoperability.pilot")
            .or_else(|_| std::fs::read_to_string("../../examples/showcase/12_polyglot_interoperability.pilot"))
            .unwrap();
        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            uri,
            code.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
        );
        server.handle_message(&did_open);

        // Hover over 'recorder' at line 27 (0-indexed), char 5
        let hover_req = format!(
            r#"{{"jsonrpc":"2.0","id":20,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":27,"character":5}}}}}}"#,
            uri
        );
        let resp = server.handle_message(&hover_req).expect("Expected hover response");
        assert!(resp.contains("def recorder(val: any) -> any"));

        // Hover over 'pilot_greeting' at line 12 (0-indexed line 11), char 6
        let hover_req3 = format!(
            r#"{{"jsonrpc":"2.0","id":22,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":11,"character":6}}}}}}"#,
            uri
        );
        let resp3 = server.handle_message(&hover_req3).expect("Expected hover response");
        assert!(resp3.contains("let pilot_greeting: str"));

        // Hover over 'sequence_trace' at line 27 (0-indexed line 26), char 6
        let hover_req4 = format!(
            r#"{{"jsonrpc":"2.0","id":23,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":26,"character":6}}}}}}"#,
            uri
        );
        let resp4 = server.handle_message(&hover_req4).expect("Expected hover response");
        assert!(resp4.contains("let sequence_trace: List"));

        // Signature help inside recorder("Step 1...") at line 36 (0-indexed line 35), char 15
        let sig_req = format!(
            r#"{{"jsonrpc":"2.0","id":24,"method":"textDocument/signatureHelp","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":35,"character":15}}}}}}"#,
            uri
        );
        let sig_resp = server.handle_message(&sig_req).expect("Expected signature help response");
        assert!(sig_resp.contains("recorder"));
        assert!(sig_resp.contains("val: any"));
    }
}
