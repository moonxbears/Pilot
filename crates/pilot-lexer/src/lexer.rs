//! Milestone 29.3 & 52: Single-Pass Name-Blind Seed Lexer.
//!
//! Emits a Struct-of-Arrays `TokenTape` recognizing strictly the 14 hardware seed keywords,
//! raw atomic literals (with name-blind `TokenFlags::IS_DIMENSIONAL` and `IS_INTERPOLATED`),
//! two-way dot (`InfixDot` vs `PrefixDot`), and 6 homoiconic call-quote projections.

use pilot_span::ByteSpan;
use crate::keywords::lookup_keyword;
use crate::simd::{
    scan_ident_run, scan_quoted_string_end, scan_to_block_comment_end,
    scan_to_newline, scan_triple_string_end, scan_whitespace_run,
};
use crate::token::{TokenFlags, TokenKind, TokenTape};

pub struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
    tape: TokenTape,
    open_brackets: Vec<(TokenKind, u32)>,
    last_token_can_precede_infix_dot: bool,
    preceded_by_ws: bool,
    preceded_by_nl: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let est_tokens = (source.len() / 4).max(16);
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            tape: TokenTape::with_capacity(est_tokens),
            open_brackets: Vec::with_capacity(32),
            last_token_can_precede_infix_dot: false,
            preceded_by_ws: true,
            preceded_by_nl: true,
        }
    }

    pub fn tokenize(mut self) -> TokenTape {
        while self.pos < self.bytes.len() {
            let ws_len = scan_whitespace_run(self.bytes, self.pos);
            if ws_len > 0 {
                self.pos += ws_len;
                self.preceded_by_ws = true;
                continue;
            }
            if self.pos >= self.bytes.len() {
                break;
            }

            let start = self.pos;
            let b0 = self.bytes[start];
            let b1 = self.peek_at(start + 1);
            let b2 = self.peek_at(start + 2);

            if b0 == b'\n' {
                self.pos += 1;
                self.emit(TokenKind::Newline, TokenFlags::NONE, start, self.pos, false);
                self.preceded_by_ws = true;
                self.preceded_by_nl = true;
                continue;
            }

            // Comments & Stage-0 Preprocessor Directive Lines (`//`, `/* ... */`, `#...`)
            if b0 == b'#' {
                let kind = if start == 0 && b1 == b'!' { TokenKind::Shebang } else { TokenKind::Comment };
                let end = scan_to_newline(self.bytes, start + 1);
                self.pos = end;
                self.emit(kind, TokenFlags::NONE, start, end, false);
                continue;
            }
            if b0 == b'/' && b1 == b'/' && b2 != b'=' {
                let end = scan_to_newline(self.bytes, start + 2);
                self.pos = end;
                self.emit(TokenKind::Comment, TokenFlags::NONE, start, end, false);
                continue;
            }
            if b0 == b'/' && b1 == b'*' {
                let end = scan_to_block_comment_end(self.bytes, start + 2);
                self.pos = end;
                self.emit(TokenKind::BlockComment, TokenFlags::NONE, start, end, false);
                continue;
            }

            // Prefixed strings (`f"..."`, `r"..."`, `b"..."`)
            if (b0 == b'f' || b0 == b'F' || b0 == b'r' || b0 == b'R' || b0 == b'b' || b0 == b'B')
                && (b1 == b'"' || b1 == b'\'')
            {
                let mut extra = TokenFlags::NONE;
                if b0 == b'f' || b0 == b'F' {
                    extra.insert(TokenFlags::IS_INTERPOLATED);
                } else if b0 == b'r' || b0 == b'R' {
                    extra.insert(TokenFlags::IS_RAW_STRING);
                }
                self.scan_string_literal(start, start + 1, b1, extra);
                continue;
            }

            // Standard or triple-quoted strings (`"..."`, `"""..."""`)
            if b0 == b'"' || b0 == b'\'' {
                self.scan_string_literal(start, start, b0, TokenFlags::NONE);
                continue;
            }

            // Numeric & Name-Blind Dimensional Literals (`42`, `0xFF`, `3.14`, `250ms`, `64KB`)
            if b0.is_ascii_digit() {
                self.scan_number_or_dimension(start);
                continue;
            }

            // Hierarchical `:ns/kw` Keyword Literals vs `:` / `::` / `:=` / `:=:`
            if b0 == b':' {
                if b1 == b':' {
                    self.pos += 2;
                    self.emit(TokenKind::DoubleColon, TokenFlags::NONE, start, self.pos, false);
                    continue;
                }
                if b1 == b'=' {
                    if b2 == b':' {
                        self.pos += 3;
                        self.emit(TokenKind::SwapEqual, TokenFlags::NONE, start, self.pos, false);
                    } else {
                        self.pos += 2;
                        self.emit(TokenKind::Walrus, TokenFlags::NONE, start, self.pos, false);
                    }
                    continue;
                }
                if (b1.is_ascii_alphabetic() || b1 == b'_') && self.preceded_by_ws {
                    let mut end = start + 1 + scan_ident_run(self.bytes, start + 1);
                    if self.peek_at(end) == b'/' {
                        let ns_part = scan_ident_run(self.bytes, end + 1);
                        if ns_part > 0 {
                            end += 1 + ns_part;
                        }
                    }
                    self.pos = end;
                    self.emit(TokenKind::KeywordLit, TokenFlags::NONE, start, end, true);
                    continue;
                }
                self.pos += 1;
                self.emit(TokenKind::Colon, TokenFlags::NONE, start, self.pos, false);
                continue;
            }

            // Identifiers & The 14 Hardware Seed Keywords
            if b0.is_ascii_alphabetic() || b0 == b'_' {
                let len = scan_ident_run(self.bytes, start);
                let end = start + len;
                let ident_str = &self.source[start..end];
                let kind = lookup_keyword(ident_str).unwrap_or(TokenKind::Ident);
                self.pos = end;
                let can_infix = matches!(kind, TokenKind::Ident | TokenKind::KwEngine);
                self.emit(kind, TokenFlags::NONE, start, end, can_infix);
                continue;
            }

            // Homoiconic Call-Quoting Projections (`!(`, `!!(`, `!&(`, `!^(`, `!@(`, `!?(`, `!=`, `!`)
            if b0 == b'!' {
                if b1 == b'=' {
                    self.pos += 2;
                    self.emit(TokenKind::NotEqual, TokenFlags::NONE, start, self.pos, false);
                    continue;
                }
                if b1 == b'(' {
                    self.pos += 1;
                    self.emit(TokenKind::CallQuoteShallow, TokenFlags::FOLLOWED_BY_LPAREN, start, self.pos, false);
                    continue;
                }
                let cq = match (b1, b2) {
                    (b'!', b'(') => Some(TokenKind::CallQuoteDeep),
                    (b'&', b'(') => Some(TokenKind::CallQuoteAbi),
                    (b'^', b'(') => Some(TokenKind::CallQuoteReact),
                    (b'@', b'(') => Some(TokenKind::CallQuoteAdjoint),
                    (b'?', b'(') => Some(TokenKind::CallQuoteEffect),
                    _ => None,
                };
                if let Some(kind) = cq {
                    self.pos += 2;
                    self.emit(kind, TokenFlags::FOLLOWED_BY_LPAREN, start, self.pos, false);
                    continue;
                }
                self.pos += 1;
                self.emit(TokenKind::Exclamation, TokenFlags::NONE, start, self.pos, true);
                continue;
            }

            // Dots & Ranges (`.`, `..`, `..=`, `..<`, `...`)
            if b0 == b'.' {
                if b1 == b'.' {
                    let (kind, adv) = match b2 {
                        b'=' => (TokenKind::DotDotEqual, 3),
                        b'<' => (TokenKind::DotDotLess, 3),
                        b'.' => (TokenKind::Ellipsis, 3),
                        _ => (TokenKind::DotDot, 2),
                    };
                    self.pos += adv;
                    self.emit(kind, TokenFlags::NONE, start, self.pos, false);
                    continue;
                }
                self.pos += 1;
                let dot_kind = if !self.preceded_by_ws && !self.preceded_by_nl && self.last_token_can_precede_infix_dot {
                    TokenKind::InfixDot
                } else {
                    TokenKind::PrefixDot
                };
                self.emit(dot_kind, TokenFlags::NONE, start, self.pos, false);
                continue;
            }

            self.scan_operator_or_delimiter(start, b0, b1, b2);
        }

        let eof_pos = self.bytes.len();
        self.emit(TokenKind::Eof, TokenFlags::NONE, eof_pos, eof_pos, false);
        self.tape
    }

    fn scan_string_literal(&mut self, token_start: usize, quote_pos: usize, quote: u8, extra_flags: TokenFlags) {
        if self.peek_at(quote_pos + 1) == quote && self.peek_at(quote_pos + 2) == quote {
            let end = scan_triple_string_end(self.bytes, quote_pos + 3, quote);
            self.pos = end;
            self.emit(TokenKind::TripleStrLit, extra_flags, token_start, end, true);
        } else {
            let close_idx = scan_quoted_string_end(self.bytes, quote_pos + 1, quote);
            let end = if close_idx < self.bytes.len() && self.bytes[close_idx] == quote {
                close_idx + 1
            } else {
                close_idx
            };
            self.pos = end;
            self.emit(TokenKind::StrLit, extra_flags, token_start, end, true);
        }
    }

    fn scan_number_or_dimension(&mut self, start: usize) {
        let b0 = self.bytes[start];
        let b1 = self.peek_at(start + 1);
        let mut cur = start;
        let mut kind = TokenKind::IntLit;

        if b0 == b'0' && (b1 == b'x' || b1 == b'X') {
            kind = TokenKind::HexIntLit;
            cur += 2;
            while cur < self.bytes.len() && (self.bytes[cur].is_ascii_hexdigit() || self.bytes[cur] == b'_') {
                cur += 1;
            }
        } else if b0 == b'0' && (b1 == b'b' || b1 == b'B') {
            kind = TokenKind::BinIntLit;
            cur += 2;
            while cur < self.bytes.len() && (self.bytes[cur] == b'0' || self.bytes[cur] == b'1' || self.bytes[cur] == b'_') {
                cur += 1;
            }
        } else if b0 == b'0' && (b1 == b'o' || b1 == b'O') {
            kind = TokenKind::OctIntLit;
            cur += 2;
            while cur < self.bytes.len() && ((self.bytes[cur] >= b'0' && self.bytes[cur] <= b'7') || self.bytes[cur] == b'_') {
                cur += 1;
            }
        } else {
            while cur < self.bytes.len() && (self.bytes[cur].is_ascii_digit() || self.bytes[cur] == b'_') {
                cur += 1;
            }
            if cur + 1 < self.bytes.len() && self.bytes[cur] == b'.' && self.bytes[cur + 1].is_ascii_digit() {
                kind = TokenKind::FloatLit;
                cur += 1;
                while cur < self.bytes.len() && (self.bytes[cur].is_ascii_digit() || self.bytes[cur] == b'_') {
                    cur += 1;
                }
            }
            if cur < self.bytes.len() && (self.bytes[cur] == b'e' || self.bytes[cur] == b'E') {
                let next_e = self.peek_at(cur + 1);
                if next_e.is_ascii_digit() || ((next_e == b'+' || next_e == b'-') && self.peek_at(cur + 2).is_ascii_digit()) {
                    kind = TokenKind::FloatLit;
                    cur += if next_e == b'+' || next_e == b'-' { 2 } else { 1 };
                    while cur < self.bytes.len() && (self.bytes[cur].is_ascii_digit() || self.bytes[cur] == b'_') {
                        cur += 1;
                    }
                }
            }
        }

        // Name-blind dimensional unit suffix check (`250ms`, `2.4GHz`, `64KB`, `100mol`, etc.):
        // Any adjacent non-seed alphabetic identifier run marks `TokenFlags::IS_DIMENSIONAL`.
        let mut flags = TokenFlags::NONE;
        if (kind == TokenKind::IntLit || kind == TokenKind::FloatLit)
            && cur < self.bytes.len()
            && self.bytes[cur].is_ascii_alphabetic()
        {
            let unit_len = scan_ident_run(self.bytes, cur);
            if unit_len > 0 && lookup_keyword(&self.source[cur..cur + unit_len]).is_none() {
                cur += unit_len;
                flags.insert(TokenFlags::IS_DIMENSIONAL);
            }
        }

        self.pos = cur;
        self.emit(kind, flags, start, cur, true);
    }

    fn scan_operator_or_delimiter(&mut self, start: usize, b0: u8, b1: u8, b2: u8) {
        let (kind, len, can_infix_dot) = match (b0, b1, b2) {
            (b'(', _, _) => (TokenKind::LParen, 1, false),
            (b')', _, _) => (TokenKind::RParen, 1, true),
            (b'[', _, _) => (TokenKind::LBracket, 1, false),
            (b']', _, _) => (TokenKind::RBracket, 1, true),
            (b'{', _, _) => (TokenKind::LBrace, 1, false),
            (b'}', _, _) => (TokenKind::RBrace, 1, true),
            (b';', _, _) => (TokenKind::Semicolon, 1, false),
            (b',', _, _) => (TokenKind::Comma, 1, false),
            (b'$', _, _) => (TokenKind::Dollar, 1, false),
            (b'~', b'>', _) => (TokenKind::TildeGreater, 2, false),
            (b'~', _, _) => (TokenKind::Tilde, 1, false),
            (b'@', b'>', _) => (TokenKind::JsonContains, 2, false),
            (b'@', _, _) => (TokenKind::At, 1, false),
            (b'?', b'.', b'.') => (TokenKind::SafeCascade, 3, false),
            (b'?', b'?', b'=') => (TokenKind::CoalesceEqual, 3, false),
            (b'?', b'.', _) => (TokenKind::SafeNav, 2, false),
            (b'?', b'?', _) => (TokenKind::Coalesce, 2, false),
            (b'?', _, _) => (TokenKind::Question, 1, true),
            (b'+', b'=', _) => (TokenKind::PlusEqual, 2, false),
            (b'+', b'|', _) => (TokenKind::PlusPipe, 2, false),
            (b'+', b'%', _) => (TokenKind::PlusPercent, 2, false),
            (b'+', b'?', _) => (TokenKind::PlusQuestion, 2, false),
            (b'+', _, _) => (TokenKind::Plus, 1, false),
            (b'-', b'>', b'>') => (TokenKind::JsonGetText, 3, false),
            (b'-', b'>', _) => (TokenKind::Arrow, 2, false),
            (b'-', b'=', _) => (TokenKind::MinusEqual, 2, false),
            (b'-', b'|', _) => (TokenKind::MinusPipe, 2, false),
            (b'-', b'%', _) => (TokenKind::MinusPercent, 2, false),
            (b'-', b'?', _) => (TokenKind::MinusQuestion, 2, false),
            (b'-', _, _) => (TokenKind::Minus, 1, false),
            (b'*', b'*', b'=') => (TokenKind::DoubleStarEqual, 3, false),
            (b'*', b'.', b'.') => (TokenKind::StarDotDot, 3, false),
            (b'*', b'*', _) => (TokenKind::DoubleStar, 2, false),
            (b'*', b'=', _) => (TokenKind::StarEqual, 2, false),
            (b'*', b'|', _) => (TokenKind::StarPipe, 2, false),
            (b'*', b'%', _) => (TokenKind::StarPercent, 2, false),
            (b'*', b'?', _) => (TokenKind::StarQuestion, 2, false),
            (b'*', _, _) => (TokenKind::Star, 1, false),
            (b'/', b'/', b'=') => (TokenKind::DoubleSlashEqual, 3, false),
            (b'/', b'/', _) => (TokenKind::DoubleSlash, 2, false),
            (b'/', b'=', _) => (TokenKind::SlashEqual, 2, false),
            (b'/', _, _) => (TokenKind::Slash, 1, false),
            (b'%', b'%', _) => (TokenKind::DoublePercent, 2, false),
            (b'%', b'=', _) => (TokenKind::PercentEqual, 2, false),
            (b'%', _, _) => (TokenKind::Percent, 1, false),
            (b'&', b'&', _) => (TokenKind::DoubleAmper, 2, false),
            (b'&', b'=', _) => (TokenKind::AmperEqual, 2, false),
            (b'&', _, _) => (TokenKind::Amper, 1, false),
            (b'|', b'|', _) => (TokenKind::DoublePipe, 2, false),
            (b'|', b'>', _) => (TokenKind::PipeForward, 2, false),
            (b'|', b'=', _) => (TokenKind::VbarEqual, 2, false),
            (b'|', _, _) => (TokenKind::Vbar, 1, false),
            (b'^', b'=', _) => (TokenKind::CaretEqual, 2, false),
            (b'^', _, _) => (TokenKind::Caret, 1, false),
            (b'=', b'=', _) => (TokenKind::EqEqual, 2, false),
            (b'=', b'>', _) => (TokenKind::FatArrow, 2, false),
            (b'=', _, _) => (TokenKind::Equal, 1, false),
            (b'<', b'=', b'>') => (TokenKind::Spaceship, 3, false),
            (b'<', b'<', b'=') => (TokenKind::ShiftLeftEqual, 3, false),
            (b'<', b'<', _) => (TokenKind::ShiftLeft, 2, false),
            (b'<', b'=', _) => (TokenKind::LessEqual, 2, false),
            (b'<', b'-', _) => (TokenKind::ArrowLeft, 2, false),
            (b'<', b'|', _) => (TokenKind::PipeBackward, 2, false),
            (b'<', b'@', _) => (TokenKind::JsonContained, 2, false),
            (b'<', _, _) => (TokenKind::Less, 1, false),
            (b'>', b'.', b'.') => {
                if self.peek_at(start + 3) == b'<' {
                    (TokenKind::GreaterDotDotLess, 4, false)
                } else {
                    (TokenKind::GreaterDotDot, 3, false)
                }
            }
            (b'>', b'>', b'=') => (TokenKind::ShiftRightEqual, 3, false),
            (b'>', b'>', _) => (TokenKind::ShiftRight, 2, false),
            (b'>', b'=', _) => (TokenKind::GreaterEqual, 2, false),
            (b'>', _, _) => (TokenKind::Greater, 1, false),
            _ => (TokenKind::ErrorToken, 1, false),
        };
        self.pos = start + len;
        self.emit(kind, TokenFlags::NONE, start, self.pos, can_infix_dot);
    }

    #[inline(always)]
    fn peek_at(&self, idx: usize) -> u8 {
        if idx < self.bytes.len() { self.bytes[idx] } else { 0 }
    }

    #[inline]
    fn emit(&mut self, kind: TokenKind, mut flags: TokenFlags, start: usize, end: usize, can_precede_infix_dot: bool) {
        if self.preceded_by_ws { flags.insert(TokenFlags::PRECEDED_BY_WS); }
        if self.preceded_by_nl { flags.insert(TokenFlags::PRECEDED_BY_NEWLINE); }
        if self.peek_at(end) == b'(' { flags.insert(TokenFlags::FOLLOWED_BY_LPAREN); }
        let idx = self.tape.push(kind, flags, ByteSpan::new(start as u32, end as u32));
        match kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => self.open_brackets.push((kind, idx)),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let expected = match kind {
                    TokenKind::RParen => TokenKind::LParen,
                    TokenKind::RBracket => TokenKind::LBracket,
                    _ => TokenKind::LBrace,
                };
                if let Some(&(_, open_idx)) = self.open_brackets.last().filter(|&&(k, _)| k == expected) {
                    self.open_brackets.pop();
                    self.tape.bracket_links[open_idx as usize] = idx;
                    self.tape.bracket_links[idx as usize] = open_idx;
                }
            }
            _ => {}
        }
        if !matches!(kind, TokenKind::Comment | TokenKind::BlockComment | TokenKind::Newline | TokenKind::Shebang) {
            self.last_token_can_precede_infix_dot = can_precede_infix_dot;
            self.preceded_by_ws = false;
            self.preceded_by_nl = false;
        }
    }
}
