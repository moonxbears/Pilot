//! Milestone 32.3 & Milestone 48: Zero-GC Standalone Native Language Server (`pilot-lsp`).

pub mod json;
pub mod server;

pub use json::{parse_json, JsonValue};
pub use server::{run_stdio_server, LspServer};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_server_initialize_and_did_open() {
        let mut server = LspServer::new();
        let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
        let init_resp = server.handle_message(init_req).expect("Expected initialize response");
        assert!(init_resp.contains("semanticTokensProvider"));
        assert!(init_resp.contains("hoverProvider"));

        let uri = "file:///test.pilot";
        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"fn main() {{}}"}}}}}}"#,
            uri
        );
        let resp = server.handle_message(&did_open);
        assert!(resp.is_some());
        assert!(resp.unwrap().contains("publishDiagnostics"));
    }
}
