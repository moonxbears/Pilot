//! Milestone 32.3 & Milestone 48: Zero-GC Standalone Native Language Server (`pilot-lsp`).
//!
//! Provides:
//! - Complete JSON-RPC 2.0 language server protocol implementation.
//! - Sub-millisecond incremental didChange synchronization.
//! - 23-type semantic tokens with modifiers, interactive hover cards, dot-completion, signature help, go-to-definition, and formatting.

pub mod json;
pub mod server;

pub use json::{parse_json, JsonValue};
pub use server::{
    run_stdio_server, CodeActionQuery, CompletionQuery, DeclarationQuery, DefinitionQuery,
    DocumentSymbolQuery, FormattingQuery, HoverQuery, ImplementationQuery, InlayHintQuery,
    LspServer, PrepareRenameQuery, ReferencesQuery, RenameQuery, SemanticTokensFullQuery,
    SignatureHelpQuery,
};

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

        // 12. Verify that all hover & inlay queries hit the memoized ModuleAnalysis in `pilot-db`
        assert!(
            server.analysis_cache_hits() >= 10,
            "Expected >= 10 memoized analysis cache hits, got {}",
            server.analysis_cache_hits()
        );
    }

    #[test]
    fn test_showcase_21_hover() {
        let mut server = LspServer::new();
        let uri = "file:///workspace/21_advanced_type_inference.pilot";
        let code = std::fs::read_to_string("examples/showcase/21_advanced_type_inference.pilot")
            .or_else(|_| std::fs::read_to_string("../../examples/showcase/21_advanced_type_inference.pilot"))
            .unwrap();
        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            uri,
            code.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
        );
        server.handle_message(&did_open);

        // Hover over 'frame_area' at line 11 (0-indexed), char 6
        let hover_req_fa = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":11,"character":6}}}}}}"#,
            uri
        );
        let resp_fa = server.handle_message(&hover_req_fa).expect("Expected hover response for frame_area");
        println!("frame_area hover: {}", resp_fa);
        assert!(resp_fa.contains("let frame_area: i64"), "Expected i64 for frame_area, got: {}", resp_fa);

        // Hover over 'canvas_area' at line 12 (0-indexed), char 6
        let hover_req_ca = format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":12,"character":6}}}}}}"#,
            uri
        );
        let resp_ca = server.handle_message(&hover_req_ca).expect("Expected hover response for canvas_area");
        println!("canvas_area hover: {}", resp_ca);
        assert!(resp_ca.contains("let canvas_area: f64"), "Expected f64 for canvas_area, got: {}", resp_ca);

        // Hover over 'compute_area' at line 9 (0-indexed), char 6
        let hover_req_fn = format!(
            r#"{{"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":9,"character":6}}}}}}"#,
            uri
        );
        let resp_fn = server.handle_message(&hover_req_fn).expect("Expected hover response for compute_area");
        println!("compute_area hover: {}", resp_fn);

        // Hover over 'surface' at line 9 (0-indexed), char 18
        let hover_req_surf = format!(
            r#"{{"jsonrpc":"2.0","id":4,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":9,"character":18}}}}}}"#,
            uri
        );
        let resp_surf = server.handle_message(&hover_req_surf).expect("Expected hover response for surface");
        println!("surface hover: {}", resp_surf);
    }

    #[test]
    fn test_heterogeneous_collection_hover() {
        let mut server = LspServer::new();
        let uri = "file:///workspace/test_unions.pilot";
        let code = "let mylist = [\"apples\", 5, 2];\nlet item = mylist[0];\n";
        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            uri,
            code.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
        );
        server.handle_message(&did_open);

        // Hover over 'mylist' at line 0, char 6
        let hover_req = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":0,"character":6}}}}}}"#,
            uri
        );
        let resp = server.handle_message(&hover_req).expect("Expected hover response for mylist");
        println!("mylist hover: {}", resp);
        assert!(resp.contains("let mylist: List[i64 | str]"), "Expected List[i64 | str], got: {}", resp);

        // Hover over 'item' at line 1, char 6
        let hover_item = format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":1,"character":6}}}}}}"#,
            uri
        );
        let resp_item = server.handle_message(&hover_item).expect("Expected hover response for item");
        println!("item hover: {}", resp_item);
        assert!(resp_item.contains("let item: i64 | str"), "Expected i64 | str, got: {}", resp_item);
    }

    #[test]
    fn test_indexer_hover() {
        let mut server = LspServer::new();
        let uri = "file:///workspace/test_indexer.pilot";
        let code = "class MemoryCache {\n    property this[key: str]: int {\n        get => 42;\n        set(v) => {};\n    }\n}\nlet cache = MemoryCache();\nlet val = cache[\"foo\"];\n";
        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            uri,
            code.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
        );
        server.handle_message(&did_open);

        // Hover over 'this' in indexer declaration at line 1, char 14
        let hover_this = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":1,"character":14}}}}}}"#,
            uri
        );
        let resp_this = server.handle_message(&hover_this).expect("Expected hover response for this indexer");
        println!("this hover: {}", resp_this);
        assert!(resp_this.contains("property this[key: str]: i64"), "Expected property this[key: str]: i64, got: {}", resp_this);
        assert!(resp_this.contains("Pilot Indexer"), "Expected Pilot Indexer, got: {}", resp_this);

        // Hover over '[' at line 7, char 15
        let hover_bracket = format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":7,"character":15}}}}}}"#,
            uri
        );
        let resp_bracket = server.handle_message(&hover_bracket).expect("Expected hover response for indexer bracket");
        println!("bracket hover: {}", resp_bracket);
        assert!(resp_bracket.contains("property this[key: str]: i64"), "Expected property this[key: str]: i64, got: {}", resp_bracket);

        // Hover over 'val' at line 7, char 5
        let hover_val = format!(
            r#"{{"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":7,"character":5}}}}}}"#,
            uri
        );
        let resp_val = server.handle_message(&hover_val).expect("Expected hover response for val");
        println!("val hover: {}", resp_val);
        assert!(resp_val.contains("let val: i64"), "Expected let val: i64, got: {}", resp_val);
    }

    #[test]
    fn test_proc_macro_lsp_query_endpoints() {
        assert_eq!(HoverQuery::LSP_METHOD, "textDocument/hover");
        assert_eq!(HoverQuery::LSP_CAPABILITY, Some("hoverProvider"));
        assert_eq!(CompletionQuery::LSP_METHOD, "textDocument/completion");
        assert_eq!(CompletionQuery::LSP_CAPABILITY, Some("completionProvider"));
        assert_eq!(DefinitionQuery::LSP_METHOD, "textDocument/definition");
        assert_eq!(DocumentSymbolQuery::LSP_METHOD, "textDocument/documentSymbol");
        assert_eq!(FormattingQuery::LSP_METHOD, "textDocument/formatting");
        assert_eq!(InlayHintQuery::LSP_METHOD, "textDocument/inlayHint");
        assert_eq!(SignatureHelpQuery::LSP_METHOD, "textDocument/signatureHelp");
        assert_eq!(CodeActionQuery::LSP_METHOD, "textDocument/codeAction");
        assert_eq!(SemanticTokensFullQuery::LSP_METHOD, "textDocument/semanticTokens/full");
        assert_eq!(ReferencesQuery::LSP_METHOD, "textDocument/references");
        assert_eq!(ReferencesQuery::LSP_CAPABILITY, Some("referencesProvider"));
        assert_eq!(ImplementationQuery::LSP_METHOD, "textDocument/implementation");
        assert_eq!(ImplementationQuery::LSP_CAPABILITY, Some("implementationProvider"));
        assert_eq!(DeclarationQuery::LSP_METHOD, "textDocument/declaration");
        assert_eq!(DeclarationQuery::LSP_CAPABILITY, Some("declarationProvider"));
        assert_eq!(PrepareRenameQuery::LSP_METHOD, "textDocument/prepareRename");
        assert_eq!(RenameQuery::LSP_METHOD, "textDocument/rename");
        assert_eq!(RenameQuery::LSP_CAPABILITY, Some("renameProvider"));
    }

    #[test]
    fn test_object_definition_navigation_methods_and_properties() {
        let mut server = LspServer::new();
        let uri = "file:///workspace/bank.pilot";
        let code = r#"class BankAccount {
    property holder: str
    property balance: float

    def deposit(amount: float) -> float {
        self.balance = self.balance + amount;
        return self.balance;
    }
}

let acct = BankAccount("Alice", 100.0);
acct.deposit(50.0);
let b = acct.balance;
"#;
        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            uri,
            code.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
        );
        server.handle_message(&did_open);

        // 1. Navigate to class definition from instantiation `BankAccount` at line 10, char 14
        let def_class = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":10,"character":14}}}}}}"#,
            uri
        );
        let resp_class = server.handle_message(&def_class).expect("Definition for BankAccount");
        println!("def BankAccount: {}", resp_class);
        assert!(resp_class.contains(r#""line":0"#), "Expected line 0 for class BankAccount, got: {}", resp_class);

        // 2. Navigate to variable `acct` from receiver `acct.deposit` at line 11, char 1
        let def_acct = format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":11,"character":1}}}}}}"#,
            uri
        );
        let resp_acct = server.handle_message(&def_acct).expect("Definition for acct");
        println!("def acct: {}", resp_acct);
        assert!(resp_acct.contains(r#""line":10"#), "Expected line 10 for let acct, got: {}", resp_acct);

        // 3. Navigate to method `deposit` from `acct.deposit` at line 11, char 6
        let def_deposit = format!(
            r#"{{"jsonrpc":"2.0","id":3,"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":11,"character":6}}}}}}"#,
            uri
        );
        let resp_deposit = server.handle_message(&def_deposit).expect("Definition for deposit method");
        println!("def deposit: {}", resp_deposit);
        assert!(resp_deposit.contains(r#""line":4"#), "Expected line 4 for def deposit, got: {}", resp_deposit);

        // 4. Navigate to property `balance` from `acct.balance` at line 12, char 15
        let def_balance = format!(
            r#"{{"jsonrpc":"2.0","id":4,"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":12,"character":15}}}}}}"#,
            uri
        );
        let resp_balance = server.handle_message(&def_balance).expect("Definition for balance property");
        println!("def balance: {}", resp_balance);
        assert!(resp_balance.contains(r#""line":2"#), "Expected line 2 for property balance, got: {}", resp_balance);

        // 5. Navigate from `self` inside method body at line 5, char 9
        let def_self = format!(
            r#"{{"jsonrpc":"2.0","id":5,"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":5,"character":9}}}}}}"#,
            uri
        );
        let resp_self = server.handle_message(&def_self).expect("Definition for self");
        println!("def self: {}", resp_self);
        assert!(resp_self.contains(r#""line":0"#), "Expected line 0 for self -> BankAccount, got: {}", resp_self);
    }

    #[test]
    fn test_find_all_references() {
        let mut server = LspServer::new();
        let uri = "file:///workspace/calc.pilot";
        let code = r#"def calculate(x: i64) -> i64 {
    let result = x * 2;
    return result + x;
}

let val = calculate(5);
let total = calculate(10);
"#;
        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            uri,
            code.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
        );
        server.handle_message(&did_open);

        // Find all references for `calculate` with includeDeclaration: true
        let ref_calc = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/references","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":0,"character":5}},"context":{{"includeDeclaration":true}}}}}}"#,
            uri
        );
        let resp_calc = server.handle_message(&ref_calc).expect("References for calculate");
        println!("references calculate: {}", resp_calc);
        // Expect 3 occurrences: line 0 (decl), line 5 (call), line 6 (call)
        assert!(resp_calc.contains(r#""line":0"#), "Missing decl reference on line 0");
        assert!(resp_calc.contains(r#""line":5"#), "Missing call reference on line 5");
        assert!(resp_calc.contains(r#""line":6"#), "Missing call reference on line 6");

        // Find all references for local variable `result`
        let ref_result = format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":1,"character":9}},"context":{{"includeDeclaration":true}}}}}}"#,
            uri
        );
        let resp_result = server.handle_message(&ref_result).expect("References for result");
        println!("references result: {}", resp_result);
        assert!(resp_result.contains(r#""line":1"#));
        assert!(resp_result.contains(r#""line":2"#));
    }

    #[test]
    fn test_find_implementation() {
        let mut server = LspServer::new();
        let uri = "file:///workspace/shapes.pilot";
        let code = r#"interface IShape {
    def area() -> float;
}

class Circle : IShape {
    property radius: float
    def area() -> float {
        return 3.14159 * self.radius * self.radius;
    }
}

extend IShape {
    def description() -> str {
        return "Shape";
    }
}
"#;
        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            uri,
            code.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
        );
        server.handle_message(&did_open);

        // Find implementation of IShape at line 0, char 12
        let impl_req = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/implementation","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":0,"character":12}}}}}}"#,
            uri
        );
        let resp_impl = server.handle_message(&impl_req).expect("Implementations for IShape");
        println!("implementations IShape: {}", resp_impl);
        assert!(resp_impl.contains(r#""line":4"#), "Expected Circle class implementing IShape on line 4");
        assert!(resp_impl.contains(r#""line":11"#), "Expected extend IShape block on line 11");
    }

    #[test]
    fn test_rename_refactoring_and_prepare_rename() {
        let mut server = LspServer::new();
        let uri = "file:///workspace/refactor.pilot";
        let code = r#"def compute(val: i64) -> i64 {
    let intermediate = val + 1;
    return intermediate * 2;
}

let out = compute(10);
"#;
        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            uri,
            code.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
        );
        server.handle_message(&did_open);

        // 1. Prepare rename on `intermediate` at line 1, char 10
        let prep_req = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":1,"character":10}}}}}}"#,
            uri
        );
        let prep_resp = server.handle_message(&prep_req).expect("Prepare rename response");
        println!("prepare rename: {}", prep_resp);
        assert!(prep_resp.contains("intermediate"), "Expected placeholder 'intermediate'");
        assert!(prep_resp.contains(r#""line":1"#));

        // 2. Perform rename to `subtotal`
        let rename_req = format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/rename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":1,"character":10}},"newName":"subtotal"}}}}"#,
            uri
        );
        let rename_resp = server.handle_message(&rename_req).expect("Rename response");
        println!("rename result: {}", rename_resp);
        assert!(rename_resp.contains("subtotal"), "Expected newText 'subtotal' in WorkspaceEdit");
        assert!(rename_resp.contains("changes"), "Expected changes map in WorkspaceEdit");
        assert!(rename_resp.contains(uri), "Expected uri key in changes map");

        // 3. Prepare rename on keyword 'def' must be rejected (null)
        let prep_kw = format!(
            r#"{{"jsonrpc":"2.0","id":3,"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":0,"character":1}}}}}}"#,
            uri
        );
        let prep_kw_resp = server.handle_message(&prep_kw).expect("Prepare rename on keyword");
        println!("prepare rename keyword: {}", prep_kw_resp);
        assert!(prep_kw_resp.contains(r#""result":null"#), "Keywords must not be renameable");
    }

    #[test]
    fn test_code_action_extract_variable() {
        let mut server = LspServer::new();
        let uri = "file:///workspace/action.pilot";
        let code = "let total = 10 * 20 + 30;\n";
        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            uri,
            code.replace('\n', "\\n")
        );
        server.handle_message(&did_open);

        // Request code actions on non-empty range `10 * 20` (line 0, col 12 to line 0, col 19)
        let ca_req = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/codeAction","params":{{"textDocument":{{"uri":"{}"}},"range":{{"start":{{"line":0,"character":12}},"end":{{"line":0,"character":19}}}},"context":{{"diagnostics":[]}}}}}}"#,
            uri
        );
        let ca_resp = server.handle_message(&ca_req).expect("CodeAction response");
        println!("code action: {}", ca_resp);
        assert!(ca_resp.contains("Extract Local Variable"), "Expected extract variable refactoring");
        assert!(ca_resp.contains("refactor.extract.variable"));
    }

    #[test]
    fn test_database_showcase_10_hover_types() {
        let mut server = LspServer::new();
        let uri = "file:///workspace/10_database.pilot";
        let code = r#"let transactions = [
    { "id": 1, "customer": "Alice", "amount": 250.0, "category": "Hardware" },
    { "id": 2, "customer": "Bob",   "amount": 420.0, "category": "Software" }
];
let hardware_txns = [];
var total_hardware_spend = 0.0;
for t in transactions {
    if t["category"] == "Hardware" {
        hardware_txns.append(t);
        total_hardware_spend = total_hardware_spend + t["amount"];
    }
}
"#;
        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"pilot","version":1,"text":"{}"}}}}}}"#,
            uri,
            code.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
        );
        server.handle_message(&did_open);

        // Hover over 'hardware_txns' at line 4, char 5
        let hover_hw = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":4,"character":5}}}}}}"#,
            uri
        );
        let resp_hw = server.handle_message(&hover_hw).expect("Expected hover response for hardware_txns");
        println!("hardware_txns hover: {}", resp_hw);
        assert!(!resp_hw.contains("'T"), "Expected concrete inferred type without unconstrained type variable, got: {}", resp_hw);
        assert!(resp_hw.contains("hardware_txns: List["), "Expected List type, got: {}", resp_hw);
        assert!(resp_hw.contains("amount: f64"), "Expected amount: f64 in record, got: {}", resp_hw);

        // Hover over 'total_hardware_spend' at line 5, char 5
        let hover_spend = format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":5,"character":5}}}}}}"#,
            uri
        );
        let resp_spend = server.handle_message(&hover_spend).expect("Expected hover response for total_hardware_spend");
        println!("total_hardware_spend hover: {}", resp_spend);
        assert!(resp_spend.contains("total_hardware_spend: f64"), "Expected f64, got: {}", resp_spend);
    }
}
