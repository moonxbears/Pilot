//! Milestone 31.3: CST Formatting Engine utilizing Wadler-Lindig Document Algebra.

use pilot_syntax::{parse_cst, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};
use crate::doc::Doc;
use crate::options::{FormatOptions, FormatResult};

pub fn format_source(src: &str, options: &FormatOptions) -> FormatResult {
    let cst = parse_cst(src);

    // If there were syntax errors, collect diagnostic messages
    let diagnostics: Vec<String> = cst.diagnostics.iter().map(|d| d.message.clone()).collect();

    let doc = format_root(&cst.root, options);
    let mut formatted = doc.render(options.max_line_width, &options.indent_string);

    // Ensure canonical single trailing newline if original had trailing newline or content
    if !formatted.is_empty() && !formatted.ends_with('\n') {
        formatted.push('\n');
    }

    let src_trimmed = src.trim().replace("\r\n", "\n");
    let fmt_trimmed = formatted.trim().replace("\r\n", "\n");
    let was_modified = src_trimmed != fmt_trimmed;

    FormatResult {
        success: true,
        formatted_code: formatted,
        was_modified,
        diagnostics,
    }
}

fn format_root<'a>(root: &SyntaxNode, options: &FormatOptions) -> Doc<'a> {
    let mut docs = Vec::new();
    let mut saw_newline = 0;

    for child in root.children() {
        match child {
            SyntaxElement::Token(token) => {
                let tk = token.kind();
                if tk == SyntaxKind::Newline {
                    saw_newline += 1;
                } else if tk == SyntaxKind::LineComment || tk == SyntaxKind::BlockComment || tk == SyntaxKind::RegionStart || tk == SyntaxKind::RegionEnd {
                    if options.preserve_comments {
                        if !docs.is_empty() {
                            docs.push(Doc::HardLine);
                            if saw_newline > 1 && options.preserve_blank_lines {
                                docs.push(Doc::HardLine);
                            }
                        }
                        docs.push(Doc::text(token.text().to_string()));
                    }
                    saw_newline = 0;
                }
            }
            SyntaxElement::Node(child_node) => {
                if !docs.is_empty() {
                    docs.push(Doc::HardLine);
                    if saw_newline > 1 && options.preserve_blank_lines {
                        docs.push(Doc::HardLine);
                    }
                }
                saw_newline = 0;
                format_node(&child_node, options, 0, &mut docs);
            }
        }
    }

    Doc::Concat(docs)
}

fn format_token<'a>(
    token: &SyntaxToken,
    options: &FormatOptions,
    docs: &mut Vec<Doc<'a>>,
) {
    let k = token.kind();
    match k {
        SyntaxKind::Whitespace | SyntaxKind::Newline => {}
        SyntaxKind::LineComment | SyntaxKind::BlockComment | SyntaxKind::Shebang | SyntaxKind::RegionStart | SyntaxKind::RegionEnd => {
            if options.preserve_comments {
                docs.push(Doc::text(token.text().to_string()));
            }
        }
        SyntaxKind::MissingIdent | SyntaxKind::MissingRParen | SyntaxKind::MissingRBrace | SyntaxKind::MissingRBracket | SyntaxKind::MissingSemicolon | SyntaxKind::MissingExpr | SyntaxKind::MissingType => {}
        SyntaxKind::Semicolon => {
            let emit = match options.use_colon_suites {
                Some(true) => false,
                _ => options.emit_semicolons,
            };
            if emit {
                docs.push(Doc::text(";"));
            }
        }
        SyntaxKind::Colon => {
            docs.push(Doc::text(": "));
        }
        SyntaxKind::Comma => {
            docs.push(Doc::text(", "));
        }
        SyntaxKind::Equal => {
            docs.push(Doc::text(" = "));
        }
        SyntaxKind::Plus => {
            docs.push(Doc::text(" + "));
        }
        SyntaxKind::Minus => {
            docs.push(Doc::text(" - "));
        }
        SyntaxKind::Star => {
            docs.push(Doc::text(" * "));
        }
        SyntaxKind::Slash => {
            docs.push(Doc::text(" / "));
        }
        SyntaxKind::Arrow => {
            docs.push(Doc::text(" -> "));
        }
        SyntaxKind::FatArrow => {
            docs.push(Doc::text(" => "));
        }
        SyntaxKind::EqEqual => {
            docs.push(Doc::text(" == "));
        }
        SyntaxKind::NotEqual => {
            docs.push(Doc::text(" != "));
        }
        SyntaxKind::Less => {
            docs.push(Doc::text(" < "));
        }
        SyntaxKind::LessEqual => {
            docs.push(Doc::text(" <= "));
        }
        SyntaxKind::Greater => {
            docs.push(Doc::text(" > "));
        }
        SyntaxKind::GreaterEqual => {
            docs.push(Doc::text(" >= "));
        }
        _ => {
            docs.push(Doc::text(token.text().to_string()));
        }
    }
}

fn format_node<'a>(
    node: &SyntaxNode,
    options: &FormatOptions,
    depth: usize,
    docs: &mut Vec<Doc<'a>>,
) {
    let k = node.kind();
    match k {
        SyntaxKind::BlockExpr => {
            format_block(node, options, depth, docs);
        }
        SyntaxKind::FnDecl => {
            format_fn_decl(node, options, depth, docs);
        }
        SyntaxKind::IfExpr => {
            format_if_expr(node, options, depth, docs);
        }
        SyntaxKind::LetStmt | SyntaxKind::VarStmt | SyntaxKind::ConstDecl => {
            format_binding_stmt(node, options, docs);
        }
        SyntaxKind::ReturnStmt => {
            format_return_stmt(node, options, docs);
        }
        _ => {
            for child in node.children() {
                match child {
                    SyntaxElement::Token(token) => {
                        format_token(&token, options, docs);
                    }
                    SyntaxElement::Node(inner) => {
                        format_node(&inner, options, depth, docs);
                    }
                }
            }
        }
    }
}

fn format_binding_stmt<'a>(node: &SyntaxNode, options: &FormatOptions, docs: &mut Vec<Doc<'a>>) {
    let mut parts = Vec::new();

    for child in node.children() {
        match child {
            SyntaxElement::Token(token) => {
                let tk = token.kind();
                if tk == SyntaxKind::Whitespace || tk == SyntaxKind::Newline {
                    continue;
                }
                if matches!(tk, SyntaxKind::KwLet | SyntaxKind::KwVar | SyntaxKind::KwConst) {
                    parts.push(Doc::text(token.text().to_string()));
                    parts.push(Doc::space());
                } else if tk == SyntaxKind::Semicolon {
                    let emit = match options.use_colon_suites {
                        Some(true) => false,
                        _ => options.emit_semicolons,
                    };
                    if emit {
                        parts.push(Doc::text(";"));
                    }
                } else {
                    format_token(&token, options, &mut parts);
                }
            }
            SyntaxElement::Node(inner) => {
                format_node(&inner, options, 0, &mut parts);
            }
        }
    }

    docs.push(Doc::Concat(parts));
}

fn format_return_stmt<'a>(node: &SyntaxNode, options: &FormatOptions, docs: &mut Vec<Doc<'a>>) {
    let mut parts = Vec::new();

    for child in node.children() {
        match child {
            SyntaxElement::Token(token) => {
                let tk = token.kind();
                if tk == SyntaxKind::Whitespace || tk == SyntaxKind::Newline {
                    continue;
                }
                if tk == SyntaxKind::KwReturn {
                    parts.push(Doc::text("return"));
                    parts.push(Doc::space());
                } else if tk == SyntaxKind::Semicolon {
                    let emit = match options.use_colon_suites {
                        Some(true) => false,
                        _ => options.emit_semicolons,
                    };
                    if emit {
                        parts.push(Doc::text(";"));
                    }
                } else {
                    format_token(&token, options, &mut parts);
                }
            }
            SyntaxElement::Node(inner) => {
                format_node(&inner, options, 0, &mut parts);
            }
        }
    }

    docs.push(Doc::Concat(parts));
}

fn format_fn_decl<'a>(node: &SyntaxNode, options: &FormatOptions, depth: usize, docs: &mut Vec<Doc<'a>>) {
    let mut parts = Vec::new();
    let mut block_opt = None;

    for child in node.children() {
        match child {
            SyntaxElement::Token(token) => {
                let tk = token.kind();
                if tk == SyntaxKind::Whitespace || tk == SyntaxKind::Newline {
                    continue;
                }
                if matches!(tk, SyntaxKind::KwDef | SyntaxKind::KwFn) {
                    parts.push(Doc::text(token.text().to_string()));
                    parts.push(Doc::space());
                } else {
                    format_token(&token, options, &mut parts);
                }
            }
            SyntaxElement::Node(child_node) => {
                if child_node.kind() == SyntaxKind::BlockExpr {
                    block_opt = Some(child_node);
                } else {
                    format_node(&child_node, options, depth, &mut parts);
                }
            }
        }
    }

    if let Some(block) = block_opt {
        format_block_with_header(Doc::Concat(parts), &block, options, depth, docs);
    } else {
        docs.push(Doc::Concat(parts));
    }
}

fn format_if_expr<'a>(node: &SyntaxNode, options: &FormatOptions, depth: usize, docs: &mut Vec<Doc<'a>>) {
    let mut header_parts = Vec::new();
    let mut block_opt = None;
    let mut else_block_opt = None;
    let mut saw_else = false;

    for child in node.children() {
        match child {
            SyntaxElement::Token(token) => {
                let tk = token.kind();
                if tk == SyntaxKind::Whitespace || tk == SyntaxKind::Newline {
                    continue;
                }
                if tk == SyntaxKind::KwIf {
                    header_parts.push(Doc::text("if "));
                } else if tk == SyntaxKind::KwElse {
                    saw_else = true;
                } else {
                    format_token(&token, options, &mut header_parts);
                }
            }
            SyntaxElement::Node(child_node) => {
                if child_node.kind() == SyntaxKind::BlockExpr {
                    if !saw_else {
                        block_opt = Some(child_node);
                    } else {
                        else_block_opt = Some(child_node);
                    }
                } else {
                    format_node(&child_node, options, depth, &mut header_parts);
                }
            }
        }
    }

    if let Some(then_block) = block_opt {
        format_block_with_header(Doc::Concat(header_parts), &then_block, options, depth, docs);
    } else {
        docs.push(Doc::Concat(header_parts));
    }

    if let Some(else_block) = else_block_opt {
        docs.push(Doc::HardLine);
        format_block_with_header(Doc::text("else"), &else_block, options, depth, docs);
    }
}

fn format_block_with_header<'a>(
    header: Doc<'a>,
    block: &SyntaxNode,
    options: &FormatOptions,
    depth: usize,
    docs: &mut Vec<Doc<'a>>,
) {
    let use_colon = match options.use_colon_suites {
        Some(true) => true,
        Some(false) => false,
        None => block.tokens().any(|t| t.kind() == SyntaxKind::Colon),
    };

    let mut body_docs = Vec::new();
    let mut saw_newline = 0;

    for child in block.children() {
        match child {
            SyntaxElement::Token(token) => {
                let tk = token.kind();
                if tk == SyntaxKind::Newline {
                    saw_newline += 1;
                } else if tk == SyntaxKind::LineComment || tk == SyntaxKind::BlockComment {
                    if options.preserve_comments {
                        body_docs.push(Doc::HardLine);
                        if saw_newline > 1 && options.preserve_blank_lines {
                            body_docs.push(Doc::HardLine);
                        }
                        body_docs.push(Doc::text(token.text().to_string()));
                    }
                    saw_newline = 0;
                }
            }
            SyntaxElement::Node(child_node) => {
                body_docs.push(Doc::HardLine);
                if saw_newline > 1 && options.preserve_blank_lines {
                    body_docs.push(Doc::HardLine);
                }
                saw_newline = 0;
                format_node(&child_node, options, depth + 1, &mut body_docs);
            }
        }
    }

    if use_colon {
        docs.push(header);
        docs.push(Doc::text(":"));
        docs.push(Doc::Indent(Box::new(Doc::Concat(body_docs))));
    } else {
        docs.push(header);
        docs.push(Doc::text(" {"));
        docs.push(Doc::Indent(Box::new(Doc::Concat(body_docs))));
        docs.push(Doc::HardLine);
        docs.push(Doc::text("}"));
    }
}

fn format_block<'a>(
    block: &SyntaxNode,
    options: &FormatOptions,
    depth: usize,
    docs: &mut Vec<Doc<'a>>,
) {
    let mut body_docs = Vec::new();
    let mut saw_newline = 0;

    for child in block.children() {
        match child {
            SyntaxElement::Token(token) => {
                let tk = token.kind();
                if tk == SyntaxKind::Newline {
                    saw_newline += 1;
                } else if tk == SyntaxKind::LineComment || tk == SyntaxKind::BlockComment {
                    if options.preserve_comments {
                        body_docs.push(Doc::HardLine);
                        if saw_newline > 1 && options.preserve_blank_lines {
                            body_docs.push(Doc::HardLine);
                        }
                        body_docs.push(Doc::text(token.text().to_string()));
                    }
                    saw_newline = 0;
                }
            }
            SyntaxElement::Node(child_node) => {
                body_docs.push(Doc::HardLine);
                if saw_newline > 1 && options.preserve_blank_lines {
                    body_docs.push(Doc::HardLine);
                }
                saw_newline = 0;
                format_node(&child_node, options, depth + 1, &mut body_docs);
            }
        }
    }

    docs.push(Doc::Concat(body_docs));
}
