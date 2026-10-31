//! Procedural macros for Pilot's Query-Based Compiler-as-a-Library LSP architecture.
//!
//! Provides:
//! - `#[derive(LspQuery)]` with `#[lsp_method("...")]` and `#[lsp_capability("...")]`
//! - `#[lsp_query("...")]` attribute macro for decorating compiler query functions
//! - `#[derive(IntoLspDiagnostic)]` for auto-converting compiler diagnostic structs into LSP wire payloads

extern crate proc_macro;

use proc_macro::{Delimiter, TokenStream, TokenTree};

/// Flattens top-level `Delimiter::None` groups that `rustc` may insert during macro expansion.
fn flatten_tokens(stream: TokenStream) -> Vec<TokenTree> {
    let mut out = Vec::new();
    for tt in stream {
        match &tt {
            TokenTree::Group(g) if g.delimiter() == Delimiter::None => {
                out.extend(flatten_tokens(g.stream()));
            }
            _ => out.push(tt),
        }
    }
    out
}

/// Strips surrounding quotes from a string literal token.
fn parse_string_literal(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        Some(trimmed[1..trimmed.len() - 1].to_string())
    } else {
        None
    }
}

/// Extracts `#[attr_name("value")]` or `#[attr_name = "value"]` from a token list.
fn extract_attr_string(tokens: &[TokenTree], attr_name: &str) -> Option<String> {
    for i in 0..tokens.len() {
        if let TokenTree::Punct(p) = &tokens[i] {
            if p.as_char() == '#' {
                if let Some(TokenTree::Group(g)) = tokens.get(i + 1) {
                    if g.delimiter() == Delimiter::Bracket {
                        let inner = flatten_tokens(g.stream());
                        if let Some(TokenTree::Ident(id)) = inner.first() {
                            if id.to_string() == attr_name {
                                // Check for `#[attr_name("...")]`
                                if let Some(TokenTree::Group(paren)) = inner.get(1) {
                                    if paren.delimiter() == Delimiter::Parenthesis {
                                        let args = flatten_tokens(paren.stream());
                                        if let Some(TokenTree::Literal(lit)) = args.first() {
                                            if let Some(s) = parse_string_literal(&lit.to_string()) {
                                                return Some(s);
                                            }
                                        }
                                    }
                                }
                                // Check for `#[attr_name = "..."]`
                                if let (Some(TokenTree::Punct(eq)), Some(TokenTree::Literal(lit))) =
                                    (inner.get(1), inner.get(2))
                                {
                                    if eq.as_char() == '=' {
                                        if let Some(s) = parse_string_literal(&lit.to_string()) {
                                            return Some(s);
                                        }
                                    }
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

/// Extracts the type name following `struct` or `enum`.
fn extract_type_name(tokens: &[TokenTree]) -> Option<String> {
    for i in 0..tokens.len() {
        if let TokenTree::Ident(id) = &tokens[i] {
            let s = id.to_string();
            if s == "struct" || s == "enum" {
                if let Some(TokenTree::Ident(type_id)) = tokens.get(i + 1) {
                    return Some(type_id.to_string());
                }
            }
        }
    }
    None
}

/// Extracts the function name following `fn`.
fn extract_fn_name(tokens: &[TokenTree]) -> Option<String> {
    for i in 0..tokens.len() {
        if let TokenTree::Ident(id) = &tokens[i] {
            if id.to_string() == "fn" {
                if let Some(TokenTree::Ident(fn_id)) = tokens.get(i + 1) {
                    return Some(fn_id.to_string());
                }
            }
        }
    }
    None
}

/// Derives `LspQuery` metadata for a query endpoint struct.
///
/// Example:
/// ```ignore
/// #[derive(LspQuery)]
/// #[lsp_method("textDocument/hover")]
/// #[lsp_capability("hoverProvider")]
/// pub struct HoverEndpoint;
/// ```
#[proc_macro_derive(LspQuery, attributes(lsp_method, lsp_capability))]
pub fn derive_lsp_query(input: TokenStream) -> TokenStream {
    let tokens = flatten_tokens(input);
    let type_name = extract_type_name(&tokens).expect("#[derive(LspQuery)] requires a struct or enum");
    let method = extract_attr_string(&tokens, "lsp_method")
        .expect("#[derive(LspQuery)] requires #[lsp_method(\"<jsonrpc-method>\")]");
    let capability = extract_attr_string(&tokens, "lsp_capability");

    let cap_expr = match capability {
        Some(c) => format!("Some(\"{}\")", c),
        None => "None".to_string(),
    };

    let expanded = format!(
        r#"
        impl {type_name} {{
            pub const LSP_METHOD: &'static str = "{method}";
            pub const LSP_CAPABILITY: Option<&'static str> = {cap_expr};
        }}
        "#
    );

    expanded.parse().expect("valid generated Rust code for LspQuery")
}

/// Attribute macro `#[lsp_query("textDocument/...")]` that decorates a compiler query function
/// and generates a compile-time `LspQueryMeta` descriptor struct.
///
/// Example:
/// ```ignore
/// #[lsp_query("textDocument/hover")]
/// pub fn hover_at_offset(db: &mut Database, file_id: FileId, offset: u32) -> Option<String> {
///     ...
/// }
/// ```
#[proc_macro_attribute]
pub fn lsp_query(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_tokens = flatten_tokens(attr);
    let method = attr_tokens
        .first()
        .and_then(|tt| match tt {
            TokenTree::Literal(lit) => parse_string_literal(&lit.to_string()),
            _ => None,
        })
        .expect("#[lsp_query(\"...\")] requires a JSON-RPC method string literal");

    let item_str = item.to_string();
    let item_tokens = flatten_tokens(item);
    let fn_name = extract_fn_name(&item_tokens)
        .expect("#[lsp_query] must be applied to a function declaration");

    let meta_const_name = format!("{}_LSP_METHOD", fn_name.to_uppercase());

    let expanded = format!(
        r#"
        pub const {meta_const_name}: &'static str = "{method}";
        {item_str}
        "#
    );

    expanded.parse().expect("valid generated Rust code for #[lsp_query]")
}

/// Derives `IntoLspDiagnostic` conversion helper methods on a compiler diagnostic struct
/// containing `code`, `message`, `span`, and `severity` fields.
#[proc_macro_derive(IntoLspDiagnostic)]
pub fn derive_into_lsp_diagnostic(input: TokenStream) -> TokenStream {
    let tokens = flatten_tokens(input);
    let type_name = extract_type_name(&tokens)
        .expect("#[derive(IntoLspDiagnostic)] requires a struct");

    let expanded = format!(
        r#"
        impl {type_name} {{
            /// Auto-derived LSP severity code (`1` = Error, `2` = Warning, `3` = Info, `4` = Hint).
            #[inline]
            pub fn lsp_severity_code(&self) -> u8 {{
                self.severity.as_lsp_code()
            }}

            /// Auto-derived conversion into an LSP wire diagnostic descriptor `(ByteSpan, severity, code, message)`.
            #[inline]
            pub fn to_lsp_wire(&self) -> (pilot_span::ByteSpan, u8, &str, &str) {{
                (self.span, self.lsp_severity_code(), self.code.as_str(), self.message.as_str())
            }}
        }}
        "#
    );

    expanded.parse().expect("valid generated Rust code for IntoLspDiagnostic")
}
