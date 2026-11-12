//! String literals: plain, raw (`r"..."`), triple-quoted (`"""..."""`) and
//! interpolated (`f"..."`, raw interpolated `rf"..."`) forms.
//!
//! The lexer marks a literal's prefix with `TokenFlags::IS_RAW_STRING` and
//! `TokenFlags::IS_INTERPOLATED`; the letters themselves are only stripped
//! here. A raw literal keeps every backslash as written. An interpolated
//! literal becomes string concatenation; a hole ending in `:pattern`
//! (`{n:#_###}`) calls the Pilot function a module binds with
//! `__engine("bind_format_spec", nameof(f))` as `f(value, "pattern")`.

use crate::*;
use pilot_lexer::TokenFlags;

impl<'a> Parser<'a> {
    /// The string literal at the cursor, whose span is `sp`.
    pub(crate) fn parse_str_lit(&mut self, sp: ByteSpan) -> NodeId {
        let raw_text = self.text();
        let flags = self.tape.flag(self.cur());
        let is_raw = flags.contains(TokenFlags::IS_RAW_STRING);
        let is_f = flags.contains(TokenFlags::IS_INTERPOLATED);
        let s = raw_text.trim_start_matches(|c: char| c.is_ascii_alphabetic());
        let triple = (s.starts_with("\"\"\"") || s.starts_with("'''")) && s.len() >= 6;
        let inner = if triple {
            &s[3..s.len() - 3]
        } else if s.len() >= 2 {
            &s[1..s.len() - 1]
        } else {
            s
        };
        self.advance();
        if is_f {
            return self.lower_fstring(inner, sp, is_raw);
        }
        let text = if is_raw { inner.to_string() } else { unescape_str(inner) };
        let sym = self.arena.interner.intern(&text);
        self.arena.alloc_node(NodeKind::StrLit, 0, sp, sym.0, 0)
    }

    /// A string literal node holding the segment `lit`, escapes processed
    /// unless `is_raw`.
    fn fstring_segment(&mut self, lit: &str, sp: ByteSpan, is_raw: bool) -> NodeId {
        let text = if is_raw { lit.to_string() } else { unescape_str(lit) };
        let sym = self.arena.interner.intern(&text);
        self.arena.alloc_node(NodeKind::StrLit, 0, sp, sym.0, 0)
    }

    /// Lowers the body of `f"..."` into string concatenation: literal
    /// segments become string literals and every `{expr}` hole is parsed as an
    /// expression. `{{` and `}}` stand for literal braces. The chain starts
    /// with a string so `+` concatenates instead of adding numbers.
    fn lower_fstring(&mut self, body: &'a str, sp: ByteSpan, is_raw: bool) -> NodeId {
        let bytes = body.as_bytes();
        let mut parts: Vec<NodeId> = Vec::new();
        let mut lit = String::new();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i];
            if (c == b'{' || c == b'}') && bytes.get(i + 1) == Some(&c) {
                lit.push(c as char);
                i += 2;
                continue;
            }
            if c == b'}' {
                self.arena.diagnostics.push((sp, "SyntaxError: unmatched `}` in f-string; write `}}` for a literal brace".into()));
                i += 1;
                continue;
            }
            if c != b'{' {
                let ch = body[i..].chars().next().unwrap_or('?');
                lit.push(ch);
                i += ch.len_utf8();
                continue;
            }
            let Some(end) = fstring_hole_end(bytes, i + 1) else {
                self.arena.diagnostics.push((sp, "SyntaxError: unterminated `{` in f-string".into()));
                break;
            };
            if !lit.is_empty() {
                parts.push(self.fstring_segment(&lit, sp, is_raw));
                lit.clear();
            }
            let hole = &body[i + 1..end];
            let offset = (hole.as_ptr() as usize).saturating_sub(self.src.as_ptr() as usize) as u32;
            let (expr, pattern) = split_format_pattern(hole);
            let value = self.parse_fstring_hole(unescape_hole_quotes(expr), offset, sp);
            parts.push(match pattern {
                Some(p) => self.format_call(value, p, sp),
                None => value,
            });
            i = end + 1;
        }
        if !lit.is_empty() || parts.is_empty() {
            parts.push(self.fstring_segment(&lit, sp, is_raw));
        }
        let first_is_str = self.arena.node(parts[0]).node_kind() == NodeKind::StrLit;
        let mut acc = if first_is_str {
            parts[0]
        } else {
            let empty = self.arena.interner.intern("");
            let e = self.arena.alloc_node(NodeKind::StrLit, 0, sp, empty.0, 0);
            self.arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::Add as u16, sp, e.0, parts[0].0)
        };
        for &p in &parts[1..] {
            acc = self.arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::Add as u16, sp, acc.0, p.0);
        }
        acc
    }

    /// `value` written by `pattern`: a call of the bound format function.
    fn format_call(&mut self, value: NodeId, pattern: &str, sp: ByteSpan) -> NodeId {
        let Some(func) = self.tables.format_spec().map(str::to_string) else {
            self.arena.diagnostics.push((sp, format!("SyntaxError: no module binds `{HOOK_FORMAT_SPEC}`, so the format pattern `{pattern}` has no meaning")));
            return value;
        };
        let fsym = self.arena.interner.intern(&func);
        let callee = self.arena.alloc_node(NodeKind::IdentExpr, 0, sp, fsym.0, 0);
        let psym = self.arena.interner.intern(pattern);
        let pat = self.arena.alloc_node(NodeKind::StrLit, 0, sp, psym.0, 0);
        self.arena.alloc_list_node(NodeKind::CallExpr, 0, sp, &[callee, value, pat])
    }

    /// Parses one `{expr}` hole of an f-string with a temporary token tape.
    /// Spans of the hole's nodes are shifted back into the enclosing source.
    /// `offset` is where the hole starts in the enclosing source.
    fn parse_fstring_hole(&mut self, hole: &'a str, offset: u32, sp: ByteSpan) -> NodeId {
        let first_node = self.arena.nodes.len();
        let first_diag = self.arena.diagnostics.len();
        let saved_tape = std::mem::replace(&mut self.tape, tokenize(hole));
        let saved_src = std::mem::replace(&mut self.src, hole);
        let saved_pos = std::mem::replace(&mut self.pos, 0);
        let e = if self.at_end() { NodeId::NONE } else { self.parse_expr() };
        let trailing = !self.at_end();
        self.tape = saved_tape;
        self.src = saved_src;
        self.pos = saved_pos;
        for span in &mut self.arena.spans[first_node..] {
            *span = ByteSpan::new(span.start + offset, span.end + offset);
        }
        for d in &mut self.arena.diagnostics[first_diag..] {
            d.0 = ByteSpan::new(d.0.start + offset, d.0.end + offset);
        }
        if e.is_none() {
            self.arena.diagnostics.push((sp, "SyntaxError: empty `{}` in f-string".into()));
        } else if trailing {
            self.arena.diagnostics.push((sp, "SyntaxError: f-string hole must be a single expression, optionally followed by `:` and a format pattern of `# 0 , _ .`".into()));
        }
        e
    }
}

/// Splits a hole into its expression and its format pattern: the text after
/// the last `:` outside brackets and quotes that is not part of `::`, when
/// that text is a pattern (only `# 0 , _ .`, with at least one `#` or `0`).
fn split_format_pattern(hole: &str) -> (&str, Option<&str>) {
    let b = hole.as_bytes();
    let (mut depth, mut quote, mut colon) = (0i32, None::<u8>, None::<usize>);
    let mut j = 0;
    while j < b.len() {
        let c = b[j];
        if let Some(q) = quote {
            if c == b'\\' {
                j += 1;
            } else if c == q {
                quote = None;
            }
        } else if c == b'"' || c == b'\'' {
            quote = Some(c);
        } else if matches!(c, b'(' | b'[' | b'{') {
            depth += 1;
        } else if matches!(c, b')' | b']' | b'}') {
            depth -= 1;
        } else if c == b':' && depth == 0 {
            let doubled = b.get(j + 1) == Some(&b':') || (j > 0 && b[j - 1] == b':');
            if !doubled {
                colon = Some(j);
            }
        }
        j += 1;
    }
    let Some(at) = colon else { return (hole, None) };
    let pattern = hole[at + 1..].trim();
    let is_pattern = !pattern.is_empty()
        && pattern.bytes().all(|c| matches!(c, b'#' | b'0' | b',' | b'_' | b'.'))
        && pattern.bytes().any(|c| c == b'#' || c == b'0');
    if is_pattern {
        (&hole[..at], Some(pattern))
    } else {
        (hole, None)
    }
}

/// Marks a string inside a hole delimited by escaped quotes (`\"`), the only
/// way to write a double-quoted string inside `f"..."`.
const ESCAPED_QUOTE: u8 = 0;

/// A hole with its escaped quotes made plain: `m[\"k\"]` becomes `m[ "k" ]`.
/// Each `\"` becomes two characters, so every offset in the hole is kept.
/// The rewritten text must live as long as the source it stands in for; only
/// holes that contain `\"` are copied, and each copy is kept for the life of
/// the process.
fn unescape_hole_quotes(hole: &str) -> &str {
    if !hole.contains("\\\"") {
        return hole;
    }
    let b = hole.as_bytes();
    let (mut out, mut j, mut open) = (String::with_capacity(hole.len()), 0, false);
    while j < b.len() {
        if b[j] == b'\\' && b.get(j + 1) == Some(&b'"') {
            out.push_str(if open { "\" " } else { " \"" });
            open = !open;
            j += 2;
        } else {
            let ch = hole[j..].chars().next().unwrap_or(' ');
            out.push(ch);
            j += ch.len_utf8();
        }
    }
    Box::leak(out.into_boxed_str())
}

/// Index of the `}` closing an f-string hole that starts at `start`, skipping
/// nested braces and quoted strings inside the hole.
fn fstring_hole_end(b: &[u8], start: usize) -> Option<usize> {
    let (mut depth, mut j, mut quote) = (1usize, start, None::<u8>);
    while j < b.len() {
        let c = b[j];
        let escaped_quote = c == b'\\' && b.get(j + 1) == Some(&b'"');
        if let Some(q) = quote {
            if q == ESCAPED_QUOTE && escaped_quote {
                quote = None;
                j += 1;
            } else if c == b'\\' {
                j += 1;
            } else if c == q {
                quote = None;
            }
        } else if escaped_quote {
            quote = Some(ESCAPED_QUOTE);
            j += 1;
        } else if c == b'"' || c == b'\'' {
            quote = Some(c);
        } else if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(j);
            }
        }
        j += 1;
    }
    None
}

/// `s` with its backslash escapes replaced by the characters they stand for.
pub(crate) fn unescape_str(s: &str) -> String {
    if !s.contains('\\') {
        return s.to_string();
    }
    let (mut out, mut ch) = (String::with_capacity(s.len()), s.chars());
    while let Some(c) = ch.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match ch.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some(o) => {
                out.push('\\');
                out.push(o);
            }
            None => out.push('\\'),
        }
    }
    out
}
