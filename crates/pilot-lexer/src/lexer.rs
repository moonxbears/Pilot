//! Milestone 29.3, 29.4 & 29.5: Modal Lexer, Indentation Desugaring & Delimiter Linking.

use pilot_span::ByteSpan;
use crate::keywords::lookup_keyword;
use crate::preproc::{Preprocessor, PreprocessorOptions, PreprocessorResult};
use crate::simd::*;
use crate::token::{TokenFlags, TokenKind, TokenTape};

pub struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    cursor: usize,
    tape: TokenTape,
    preproc: PreprocessorResult,
    inactive_cursor: usize,

    // Delimiter nesting state (suppresses indentation inside (), [], {})
    paren_depth: u32,
    bracket_depth: u32,
    brace_depth: u32,

    // Indentation stack for Pythonic `:` suite desugaring (Invariant #2)
    indent_stack: Vec<u16>,
    pending_virtual_rbraces: u32,
    awaiting_indent_after_colon: bool,

    // Balanced delimiter stack for O(1) bracket_links
    bracket_stack: Vec<(TokenKind, u32)>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str, options: PreprocessorOptions) -> Self {
        let preproc = Preprocessor::new(options).process(src);
        Self {
            src,
            bytes: src.as_bytes(),
            cursor: 0,
            tape: TokenTape::with_capacity(src.len() / 4 + 32),
            preproc,
            inactive_cursor: 0,
            paren_depth: 0,
            bracket_depth: 0,
            brace_depth: 0,
            indent_stack: vec![0],
            pending_virtual_rbraces: 0,
            awaiting_indent_after_colon: false,
            bracket_stack: Vec::new(),
        }
    }

    /// Tokenizes the entire source, populating and returning the SoA `TokenTape`.
    pub fn tokenize(mut self) -> TokenTape {
        while self.cursor < self.bytes.len() {
            // Check if cursor is within an inactive span from the preprocessor
            if self.skip_inactive_if_any() {
                continue;
            }

            // Emit any pending VirtualRBrace tokens before scanning next token
            if self.pending_virtual_rbraces > 0 {
                self.pending_virtual_rbraces -= 1;
                let cur = self.cursor as u32;
                self.push_token(TokenKind::VirtualRBrace, TokenFlags::IS_VIRTUAL | TokenFlags::IS_DELIMITER_CLOSE, ByteSpan::new(cur, cur));
                continue;
            }

            let start = self.cursor;
            let b = self.bytes[start];

            // 1. Newlines and Line Starts (Pythonic Indentation Handling)
            if b == b'\n' || (b == b'\r' && start + 1 < self.bytes.len() && self.bytes[start + 1] == b'\n') {
                let nl_len = if b == b'\r' { 2 } else { 1 };
                self.cursor += nl_len;
                self.handle_newline_and_indentation(start as u32, (start + nl_len) as u32);
                continue;
            }

            // 2. Whitespace run
            let ws_len = scan_whitespace_run(self.bytes, self.cursor);
            if ws_len > 0 {
                self.cursor += ws_len;
                continue;
            }

            // 3. Comments (# or // or /* ... */)
            if b == b'#' {
                let end = scan_to_newline(self.bytes, self.cursor);
                let span = ByteSpan::new(start as u32, end as u32);
                self.cursor = end;
                self.push_token(TokenKind::Comment, TokenFlags::NONE, span);
                continue;
            }
            if b == b'/' && start + 1 < self.bytes.len() {
                if self.bytes[start + 1] == b'/' {
                    let end = scan_to_newline(self.bytes, self.cursor);
                    let span = ByteSpan::new(start as u32, end as u32);
                    self.cursor = end;
                    self.push_token(TokenKind::Comment, TokenFlags::NONE, span);
                    continue;
                } else if self.bytes[start + 1] == b'*' {
                    let end = scan_to_block_comment_end(self.bytes, self.cursor + 2);
                    let span = ByteSpan::new(start as u32, end as u32);
                    self.cursor = end;
                    self.push_token(TokenKind::BlockComment, TokenFlags::NONE, span);
                    continue;
                }
            }

            // 4. Raw DSL Blocks: ```lang ... ```
            if b == b'`' && start + 2 < self.bytes.len() && self.bytes[start + 1] == b'`' && self.bytes[start + 2] == b'`' {
                let mut end = start + 3;
                while end + 2 < self.bytes.len() {
                    if self.bytes[end] == b'`' && self.bytes[end + 1] == b'`' && self.bytes[end + 2] == b'`' {
                        end += 3;
                        break;
                    }
                    end += 1;
                }
                let span = ByteSpan::new(start as u32, end as u32);
                self.cursor = end;
                self.push_token(TokenKind::RawDslBlock, TokenFlags::NONE, span);
                continue;
            }

            // 5. Triple Strings (""" or ''')
            if (b == b'"' || b == b'\'') && start + 2 < self.bytes.len() && self.bytes[start + 1] == b && self.bytes[start + 2] == b {
                let end = scan_triple_string_end(self.bytes, start + 3, b);
                let span = ByteSpan::new(start as u32, end as u32);
                self.cursor = end;
                self.push_token(TokenKind::TripleStrLit, TokenFlags::NONE, span);
                continue;
            }

            // 6. F-Strings: f"..." or f'...' or $"..." or $'...'
            if (b == b'f' || b == b'F' || b == b'$') && start + 1 < self.bytes.len() && (self.bytes[start + 1] == b'"' || self.bytes[start + 1] == b'\'') {
                self.scan_fstring(start, self.bytes[start + 1]);
                continue;
            }

            // 7. Regular Strings: "..." or '...'
            if b == b'"' || b == b'\'' {
                let end_quote = scan_quoted_string_end(self.bytes, start + 1, b);
                let end = if end_quote < self.bytes.len() && self.bytes[end_quote] == b { end_quote + 1 } else { end_quote };
                let span = ByteSpan::new(start as u32, end as u32);
                self.cursor = end;
                self.push_token(TokenKind::StrLit, TokenFlags::NONE, span);
                continue;
            }

            // 8. Call-Quoting Projections (Invariant #1 & M17):
            // Emit CallQuote* IF AND ONLY IF immediately followed by `(` with zero intervening whitespace
            if b == b'!' {
                if self.try_scan_call_quote(start).is_some() {
                    continue;
                }
            }

            // 9. Numeric and Dimensional Literals (Int, Hex, Bin, Oct, Float, Duration, Frequency, Memory)
            if b >= b'0' && b <= b'9' {
                self.scan_number_or_dimension(start);
                continue;
            }

            // 10. First-Class Keywords (:ns/kw or :kw)
            if b == b':' && start + 1 < self.bytes.len() && (self.bytes[start + 1].is_ascii_alphabetic() || self.bytes[start + 1] == b'_') {
                let mut end = start + 1;
                while end < self.bytes.len() {
                    let c = self.bytes[end];
                    if c.is_ascii_alphanumeric() || c == b'_' || c == b'/' {
                        end += 1;
                    } else {
                        break;
                    }
                }
                let span = ByteSpan::new(start as u32, end as u32);
                self.cursor = end;
                self.push_token(TokenKind::KeywordLit, TokenFlags::NONE, span);
                continue;
            }

            // 11. Identifiers and Keywords
            if (b >= b'a' && b <= b'z') || (b >= b'A' && b <= b'Z') || b == b'_' {
                let id_len = scan_ident_run(self.bytes, start);
                let end = start + id_len;
                let ident = &self.src[start..end];
                let span = ByteSpan::new(start as u32, end as u32);
                self.cursor = end;

                if let Some((kw_kind, is_ctx)) = lookup_keyword(ident) {
                    let mut flags = TokenFlags::IS_KEYWORD;
                    if is_ctx {
                        flags.insert(TokenFlags::IS_CONTEXTUAL_IDENT);
                    }
                    self.push_token(kw_kind, flags, span);
                } else {
                    self.push_token(TokenKind::Ident, TokenFlags::NONE, span);
                }
                continue;
            }

            // 12. Multi-character and Single-character Operators and Delimiters
            self.scan_operator_or_delimiter(start);
        }

        // EOF: Close any remaining open Pythonic suites with VirtualRBrace
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            let cur = self.cursor as u32;
            self.push_token(TokenKind::VirtualRBrace, TokenFlags::IS_VIRTUAL | TokenFlags::IS_DELIMITER_CLOSE, ByteSpan::new(cur, cur));
        }

        let eof_span = ByteSpan::new(self.cursor as u32, self.cursor as u32);
        self.push_token(TokenKind::Eof, TokenFlags::NONE, eof_span);

        self.tape
    }

    fn skip_inactive_if_any(&mut self) -> bool {
        while self.inactive_cursor < self.preproc.inactive_spans.len() {
            let span = self.preproc.inactive_spans[self.inactive_cursor];
            if (self.cursor as u32) < span.start {
                break;
            }
            if (self.cursor as u32) < span.end {
                self.cursor = span.end as usize;
                return true;
            }
            self.inactive_cursor += 1;
        }
        false
    }

    fn push_token(&mut self, kind: TokenKind, flags: TokenFlags, span: ByteSpan) {
        let idx = self.tape.push(kind, flags, span);

        // Update balanced delimiter jump table (O(1) bracket_links)
        if kind.is_delimiter_open() {
            self.bracket_stack.push((kind, idx));
        } else if kind.is_delimiter_close() {
            if let Some((open_kind, open_idx)) = self.bracket_stack.pop() {
                if open_kind.matching_delimiter() == Some(kind) {
                    self.tape.bracket_links[open_idx as usize] = idx;
                    self.tape.bracket_links[idx as usize] = open_idx;
                }
            }
        }
    }

    fn handle_newline_and_indentation(&mut self, nl_start: u32, nl_end: u32) {
        // If inside (), [], {}, suppress virtual newline and indentation
        if self.paren_depth > 0 || self.bracket_depth > 0 || self.brace_depth > 0 {
            return;
        }

        self.push_token(TokenKind::Newline, TokenFlags::NONE, ByteSpan::new(nl_start, nl_end));

        // Peek ahead to measure the indentation of the upcoming line
        let mut peek = self.cursor;
        let mut indent_col = 0u16;

        while peek < self.bytes.len() {
            let b = self.bytes[peek];
            if b == b' ' {
                indent_col += 1;
                peek += 1;
            } else if b == b'\t' {
                indent_col += 4;
                peek += 1;
            } else {
                break;
            }
        }

        // Blank lines or comment-only lines do not trigger indentation changes
        if peek < self.bytes.len() && (self.bytes[peek] == b'\n' || self.bytes[peek] == b'\r' || self.bytes[peek] == b'#') {
            return;
        }

        let current_indent = *self.indent_stack.last().unwrap_or(&0);

        if self.awaiting_indent_after_colon {
            self.awaiting_indent_after_colon = false;
            if indent_col > current_indent {
                self.indent_stack.push(indent_col);
                let cur = self.cursor as u32;
                self.push_token(TokenKind::VirtualLBrace, TokenFlags::IS_VIRTUAL | TokenFlags::IS_DELIMITER_OPEN, ByteSpan::new(cur, cur));
            }
        } else if indent_col < current_indent {
            // Dedent: pop indentation levels and queue VirtualRBrace tokens
            while let Some(&top) = self.indent_stack.last() {
                if top > indent_col {
                    self.indent_stack.pop();
                    self.pending_virtual_rbraces += 1;
                } else {
                    break;
                }
            }
        }
    }

    fn try_scan_call_quote(&mut self, start: usize) -> Option<()> {
        let b1 = if start + 1 < self.bytes.len() { self.bytes[start + 1] } else { 0 };

        let (kind, len) = match b1 {
            b'!' => (TokenKind::CallQuoteDeep, 2),
            b'&' => (TokenKind::CallQuoteAbi, 2),
            b'^' => (TokenKind::CallQuoteReact, 2),
            b'@' => (TokenKind::CallQuoteAdjoint, 2),
            b'?' => (TokenKind::CallQuoteEffect, 2),
            _ => (TokenKind::CallQuoteShallow, 1),
        };

        // Milestone 17 & Invariant #1: Only emit CallQuote* if followed immediately by '(' with zero whitespace
        if start + len < self.bytes.len() && self.bytes[start + len] == b'(' {
            let span = ByteSpan::new(start as u32, (start + len) as u32);
            self.cursor = start + len;
            self.push_token(kind, TokenFlags::NONE, span);
            return Some(());
        }

        None
    }

    fn scan_number_or_dimension(&mut self, start: usize) {
        let mut cur = start;

        // Check for Hex (0x), Bin (0b), Oct (0o)
        if self.bytes[cur] == b'0' && cur + 1 < self.bytes.len() {
            let spec = self.bytes[cur + 1];
            if spec == b'x' || spec == b'X' {
                cur += 2;
                while cur < self.bytes.len() && (self.bytes[cur].is_ascii_hexdigit() || self.bytes[cur] == b'_') {
                    cur += 1;
                }
                let span = ByteSpan::new(start as u32, cur as u32);
                self.cursor = cur;
                self.push_token(TokenKind::HexIntLit, TokenFlags::NONE, span);
                return;
            } else if spec == b'b' || spec == b'B' {
                cur += 2;
                while cur < self.bytes.len() && (self.bytes[cur] == b'0' || self.bytes[cur] == b'1' || self.bytes[cur] == b'_') {
                    cur += 1;
                }
                let span = ByteSpan::new(start as u32, cur as u32);
                self.cursor = cur;
                self.push_token(TokenKind::BinIntLit, TokenFlags::NONE, span);
                return;
            } else if spec == b'o' || spec == b'O' {
                cur += 2;
                while cur < self.bytes.len() && (self.bytes[cur] >= b'0' && self.bytes[cur] <= b'7' || self.bytes[cur] == b'_') {
                    cur += 1;
                }
                let span = ByteSpan::new(start as u32, cur as u32);
                self.cursor = cur;
                self.push_token(TokenKind::OctIntLit, TokenFlags::NONE, span);
                return;
            }
        }

        // Standard decimal integer or float
        let mut is_float = false;
        while cur < self.bytes.len() && (self.bytes[cur].is_ascii_digit() || self.bytes[cur] == b'_') {
            cur += 1;
        }

        // Check for decimal point not followed by a dot (to avoid consuming `..`)
        if cur + 1 < self.bytes.len() && self.bytes[cur] == b'.' && self.bytes[cur + 1].is_ascii_digit() {
            is_float = true;
            cur += 1;
            while cur < self.bytes.len() && (self.bytes[cur].is_ascii_digit() || self.bytes[cur] == b'_') {
                cur += 1;
            }
        }

        // Exponent: e.g. 1e-4
        if cur < self.bytes.len() && (self.bytes[cur] == b'e' || self.bytes[cur] == b'E') {
            is_float = true;
            cur += 1;
            if cur < self.bytes.len() && (self.bytes[cur] == b'+' || self.bytes[cur] == b'-') {
                cur += 1;
            }
            while cur < self.bytes.len() && (self.bytes[cur].is_ascii_digit() || self.bytes[cur] == b'_') {
                cur += 1;
            }
        }

        // Check for physical unit suffix (Duration, Frequency, Memory)
        let rem = &self.src[cur..];
        let (dim_kind, dim_len) = if rem.starts_with("ns") {
            (Some(TokenKind::DurationLit), 2)
        } else if rem.starts_with("us") || rem.starts_with("ms") {
            (Some(TokenKind::DurationLit), 2)
        } else if rem.starts_with("min") {
            (Some(TokenKind::DurationLit), 3)
        } else if rem.starts_with('s') && !rem.starts_with("struct") {
            (Some(TokenKind::DurationLit), 1)
        } else if rem.starts_with('h') {
            (Some(TokenKind::DurationLit), 1)
        } else if rem.starts_with("GHz") || rem.starts_with("MHz") || rem.starts_with("kHz") {
            (Some(TokenKind::FrequencyLit), 3)
        } else if rem.starts_with("Hz") {
            (Some(TokenKind::FrequencyLit), 2)
        } else if rem.starts_with("TB") || rem.starts_with("GB") || rem.starts_with("MB") || rem.starts_with("KB") {
            (Some(TokenKind::MemoryLit), 2)
        } else if rem.starts_with('B') {
            (Some(TokenKind::MemoryLit), 1)
        } else {
            (None, 0)
        };

        if let Some(kind) = dim_kind {
            cur += dim_len;
            let span = ByteSpan::new(start as u32, cur as u32);
            self.cursor = cur;
            self.push_token(kind, TokenFlags::IS_DIMENSIONAL, span);
        } else {
            let kind = if is_float { TokenKind::FloatLit } else { TokenKind::IntLit };
            let span = ByteSpan::new(start as u32, cur as u32);
            self.cursor = cur;
            self.push_token(kind, TokenFlags::NONE, span);
        }
    }

    fn scan_fstring(&mut self, start: usize, quote: u8) {
        // e.g. f"hello {x} world"
        let mut cur = start + 2; // skip f"
        let seg_start = start;

        while cur < self.bytes.len() {
            let b = self.bytes[cur];
            if b == b'\\' {
                cur += 2;
                continue;
            }
            if b == quote {
                cur += 1;
                let span = ByteSpan::new(seg_start as u32, cur as u32);
                self.cursor = cur;
                self.push_token(TokenKind::StrLit, TokenFlags::NONE, span);
                return;
            }
            if b == b'{' {
                // If escaped `{{`, treat as literal brace
                if cur + 1 < self.bytes.len() && self.bytes[cur + 1] == b'{' {
                    cur += 2;
                    continue;
                }
                cur += 1;
                let span = ByteSpan::new(seg_start as u32, cur as u32);
                self.cursor = cur;
                self.push_token(TokenKind::FStrStart, TokenFlags::NONE, span);
                return;
            }
            cur += 1;
        }

        let span = ByteSpan::new(start as u32, cur as u32);
        self.cursor = cur;
        self.push_token(TokenKind::StrLit, TokenFlags::NONE, span);
    }

    fn scan_operator_or_delimiter(&mut self, start: usize) {
        let b = self.bytes[start];
        let b1 = if start + 1 < self.bytes.len() { self.bytes[start + 1] } else { 0 };
        let b2 = if start + 2 < self.bytes.len() { self.bytes[start + 2] } else { 0 };

        // 3-character operators
        let (kind, len) = match (b, b1, b2) {
            (b'-', b'>', b'>') => (TokenKind::JsonGetText, 3),
            (b'?', b'.', b'.') => (TokenKind::SafeCascade, 3),
            (b'?', b'?', b'=') => (TokenKind::CoalesceEqual, 3),
            (b'.', b'.', b'=') => (TokenKind::DotDotEqual, 3),
            (b'.', b'.', b'<') => (TokenKind::DotDotLess, 3),
            (b'.', b'.', b'.') => (TokenKind::Ellipsis, 3),
            (b'*', b'*', b'=') => (TokenKind::DoubleStarEqual, 3),
            (b'/', b'/', b'=') => (TokenKind::DoubleSlashEqual, 3),
            (b'<', b'<', b'=') => (TokenKind::ShiftLeftEqual, 3),
            (b'>', b'>', b'=') => (TokenKind::ShiftRightEqual, 3),
            _ => (TokenKind::ErrorToken, 0),
        };

        if len > 0 {
            self.cursor = start + len;
            self.push_token(kind, TokenFlags::NONE, ByteSpan::new(start as u32, (start + len) as u32));
            return;
        }

        // 2-character operators
        let (kind, len) = match (b, b1) {
            (b':', b'=') => (TokenKind::Walrus, 2),
            (b'<', b'-') => (TokenKind::ArrowLeft, 2),
            (b'-', b'>') => (TokenKind::Arrow, 2),
            (b'=', b'>') => (TokenKind::FatArrow, 2),
            (b'|', b'>') => (TokenKind::PipeForward, 2),
            (b'<', b'|') => (TokenKind::PipeBackward, 2),
            (b'?', b'.') => (TokenKind::SafeNav, 2),
            (b'?', b'?') => (TokenKind::Coalesce, 2),
            (b'@', b'>') => (TokenKind::JsonContains, 2),
            (b'<', b'@') => (TokenKind::JsonContained, 2),
            (b'.', b'.') => (TokenKind::DotDot, 2),
            (b'*', b'*') => (TokenKind::DoubleStar, 2),
            (b'/', b'/') => (TokenKind::DoubleSlash, 2),
            (b'=', b'=') => (TokenKind::EqEqual, 2),
            (b'!', b'=') | (b'<', b'>') => (TokenKind::NotEqual, 2),
            (b'<', b'=') => (TokenKind::LessEqual, 2),
            (b'>', b'=') => (TokenKind::GreaterEqual, 2),
            (b'<', b'<') => (TokenKind::ShiftLeft, 2),
            (b'>', b'>') => (TokenKind::ShiftRight, 2),
            (b'+', b'=') => (TokenKind::PlusEqual, 2),
            (b'-', b'=') => (TokenKind::MinusEqual, 2),
            (b'*', b'=') => (TokenKind::StarEqual, 2),
            (b'/', b'=') => (TokenKind::SlashEqual, 2),
            (b'%', b'=') => (TokenKind::PercentEqual, 2),
            (b'&', b'=') => (TokenKind::AmperEqual, 2),
            (b'|', b'=') => (TokenKind::VbarEqual, 2),
            (b'^', b'=') => (TokenKind::CaretEqual, 2),
            _ => (TokenKind::ErrorToken, 0),
        };

        if len > 0 {
            self.cursor = start + len;
            self.push_token(kind, TokenFlags::NONE, ByteSpan::new(start as u32, (start + len) as u32));
            return;
        }

        // Single-character delimiters and operators
        let (kind, flags) = match b {
            b'(' => {
                self.paren_depth += 1;
                (TokenKind::LParen, TokenFlags::IS_DELIMITER_OPEN)
            }
            b')' => {
                self.paren_depth = self.paren_depth.saturating_sub(1);
                (TokenKind::RParen, TokenFlags::IS_DELIMITER_CLOSE)
            }
            b'[' => {
                self.bracket_depth += 1;
                (TokenKind::LBracket, TokenFlags::IS_DELIMITER_OPEN)
            }
            b']' => {
                self.bracket_depth = self.bracket_depth.saturating_sub(1);
                (TokenKind::RBracket, TokenFlags::IS_DELIMITER_CLOSE)
            }
            b'{' => {
                self.brace_depth += 1;
                (TokenKind::LBrace, TokenFlags::IS_DELIMITER_OPEN)
            }
            b'}' => {
                self.brace_depth = self.brace_depth.saturating_sub(1);
                (TokenKind::RBrace, TokenFlags::IS_DELIMITER_CLOSE)
            }
            b':' => {
                // If colon ends logical line outside (), [], {}, record awaiting indent
                if self.paren_depth == 0 && self.bracket_depth == 0 && self.brace_depth == 0 {
                    self.awaiting_indent_after_colon = true;
                }
                (TokenKind::Colon, TokenFlags::NONE)
            }
            b';' => (TokenKind::Semicolon, TokenFlags::NONE),
            b',' => (TokenKind::Comma, TokenFlags::NONE),
            b'.' => (TokenKind::Dot, TokenFlags::NONE),
            b'@' => (TokenKind::At, TokenFlags::NONE),
            b'?' => (TokenKind::Question, TokenFlags::NONE),
            b'!' => (TokenKind::Exclamation, TokenFlags::NONE),
            b'~' => (TokenKind::Tilde, TokenFlags::NONE),
            b'$' => (TokenKind::Dollar, TokenFlags::NONE),
            b'`' => (TokenKind::Backquote, TokenFlags::NONE),
            b'_' => (TokenKind::Underscore, TokenFlags::NONE),
            b'+' => (TokenKind::Plus, TokenFlags::NONE),
            b'-' => (TokenKind::Minus, TokenFlags::NONE),
            b'*' => (TokenKind::Star, TokenFlags::NONE),
            b'/' => (TokenKind::Slash, TokenFlags::NONE),
            b'%' => (TokenKind::Percent, TokenFlags::NONE),
            b'=' => (TokenKind::Equal, TokenFlags::NONE),
            b'<' => (TokenKind::Less, TokenFlags::NONE),
            b'>' => (TokenKind::Greater, TokenFlags::NONE),
            b'&' => (TokenKind::Amper, TokenFlags::NONE),
            b'|' => (TokenKind::Vbar, TokenFlags::NONE),
            b'^' => (TokenKind::Caret, TokenFlags::NONE),
            _ => (TokenKind::ErrorToken, TokenFlags::NONE),
        };

        self.cursor = start + 1;
        self.push_token(kind, flags, ByteSpan::new(start as u32, (start + 1) as u32));
    }
}
