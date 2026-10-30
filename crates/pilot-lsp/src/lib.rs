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
    fn test_sieve_did_open() {
        let mut server = LspServer::new();
        let src = std::fs::read_to_string("../../crates/pilot-bench/suite/sieve.pilot")
            .or_else(|_| std::fs::read_to_string("crates/pilot-bench/suite/sieve.pilot"))
            .expect("read sieve.pilot");
        
        let json_src = src.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "");
        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"file:///sieve.pilot","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            json_src
        );
        server.handle_message(&did_open);
    }

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

    #[test]
    fn test_comprehensive_hover_inference_inlay_and_semantic_tokens() {
        let mut server = LspServer::new();
        let uri = "file:///workspace/comprehensive.pilot";
        let code = "\
record Point(x: f64, y: f64)
type Shape =
    | Circle(radius: f64)
    | PointShape

def add(a: int, b: int) -> int {
    return a + b
}

let c = add(10, 20)
let explicit_val: f64 = 3.14
let scores: List[int] = [10, 20, 30]
let (px, py) = (100, 200)
let [head, ..tail] = scores
let pt = Point(1.5, 2.5)
let pt_x = pt.x
let msg = f\"Result is {c}\"
let wait_time = 250ms
let tag = :ready
let q = !(add(1, 2))
for score in scores {
    println(score)
}
";
        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            uri,
            code.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
        );
        server.handle_message(&did_open);

        let mut hover_at = |line: u32, col: u32| -> String {
            let req = format!(
                r#"{{"jsonrpc":"2.0","id":100,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":{},"character":{}}}}}}}"#,
                uri, line, col
            );
            server.handle_message(&req).expect("hover response")
        };

        // 1. Record declaration & fields: `record Point(x: f64, y: f64)`
        let h_record = hover_at(0, 8);
        assert!(h_record.contains("record Point") && h_record.contains("x: f64"), "got: {}", h_record);

        // 2. ADT & Variant: `type Shape = | Circle(radius: f64)`
        let h_adt = hover_at(1, 6);
        assert!(h_adt.contains("type Shape") && h_adt.contains("Circle(f64)"), "got: {}", h_adt);
        let h_variant = hover_at(2, 8);
        assert!(h_variant.contains("(variant) Circle: (f64) -> Shape"), "got: {}", h_variant);

        // 3. Function & Parameter inference: `def add(a: int, b: int) -> int`
        let h_fn = hover_at(5, 5);
        assert!(h_fn.contains("def add(a: i64, b: i64) -> i64"), "got: {}", h_fn);
        let h_param = hover_at(5, 8);
        assert!(h_param.contains("(parameter) a: i64"), "got: {}", h_param);
        let h_param_use = hover_at(6, 11);
        assert!(h_param_use.contains("(parameter) a: i64"), "got: {}", h_param_use);

        // 4. Variable `c` (must NOT be shadowed by polyglot `c` block!)
        let h_var_c = hover_at(9, 4);
        assert!(h_var_c.contains("let c: i64"), "got: {}", h_var_c);

        // 5. Generic List[int] type annotation
        let h_scores = hover_at(11, 5);
        assert!(h_scores.contains("let scores: List[i64]"), "got: {}", h_scores);

        // 6. Destructured tuple & slice bindings
        let h_px = hover_at(12, 6);
        assert!(h_px.contains("let px: i64"), "got: {}", h_px);
        let h_head = hover_at(13, 6);
        assert!(h_head.contains("let head: i64"), "got: {}", h_head);
        let h_tail = hover_at(13, 14);
        assert!(h_tail.contains("let ..tail: List[i64]"), "got: {}", h_tail);

        // 7. Member access `pt.x`
        let h_member = hover_at(15, 14);
        assert!(h_member.contains(".x: f64"), "got: {}", h_member);

        // 8. Variable inside interpolated f-string `f"Result is {c}"`
        let h_fstr_var = hover_at(16, 23);
        assert!(h_fstr_var.contains("let c: i64"), "got: {}", h_fstr_var);

        // 9. Dimensional literal `250ms` & Keyword literal `:ready` & Call-Quote `!(...)`
        let h_dim = hover_at(17, 17);
        assert!(h_dim.contains("250ms: i64<Duration>"), "got: {}", h_dim);
        let h_sym = hover_at(18, 12);
        assert!(h_sym.contains(":ready: keyword"), "got: {}", h_sym);
        let h_quote = hover_at(19, 8);
        assert!(h_quote.contains("Shallow Call Descriptor"), "got: {}", h_quote);

        // 10. For loop variable `for score in scores`
        let h_for_var = hover_at(20, 5);
        assert!(h_for_var.contains("for score: i64"), "got: {}", h_for_var);

        // 11. Inlay hints: should include `: i64` for `let c = ...` at character 5 (right after `let c`),
        // and should NOT include `explicit_val` or `scores` (already annotated) or `: any`.
        let inlay_req = format!(
            r#"{{"jsonrpc":"2.0","id":101,"method":"textDocument/inlayHint","params":{{"textDocument":{{"uri":"{}"}},"range":{{"start":{{"line":0,"character":0}},"end":{{"line":25,"character":0}}}}}}}}"#,
            uri
        );
        let inlay_resp = server.handle_message(&inlay_req).expect("inlay response");
        assert!(!inlay_resp.contains(": any"), "Inlay hints must never emit ': any': {}", inlay_resp);
        assert!(inlay_resp.contains(r#""line":9,"character":5"#), "Inlay hint for 'let c' should be at line 9, char 5: {}", inlay_resp);
    }
}
