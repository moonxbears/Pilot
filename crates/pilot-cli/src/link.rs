//! Parsing several source files into one linked program.
//!
//! Each file is parsed as its own unit (`pilot_parser::parse_unit`) into a
//! shared arena. Unit spans live in one program-wide offset space where each
//! file starts at its own base, so a span from anywhere in the program (a
//! parse error, a compile error, a runtime fault) maps back to its file,
//! line and column.

use pilot_ast::AstArena;
use pilot_span::{ByteSpan, FileId};

/// The files of a linked program and the arena they were parsed into.
pub struct LinkedSource {
    pub arena: AstArena,
    /// Path, source text and base offset of each unit, in link order.
    files: Vec<(String, String, u32)>,
}

impl LinkedSource {
    /// Parses `files` (path and text) as the units of one program, in order.
    pub fn parse(files: Vec<(String, String)>) -> Self {
        let mut arena = AstArena::new();
        let mut table = Vec::with_capacity(files.len());
        let mut base = 0u32;
        for (i, (path, src)) in files.into_iter().enumerate() {
            arena = pilot_parser::parse_unit(arena, &src, FileId(i as u32), base);
            let next = base + src.len() as u32 + 1;
            table.push((path, src, base));
            base = next;
        }
        Self { arena, files: table }
    }

    /// The file a span lies in and the span's byte offsets within it.
    pub fn resolve(&self, span: ByteSpan) -> (String, u32, u32) {
        let Some((path, src, base)) = self.files.iter().rev().find(|(_, _, b)| *b <= span.start) else {
            return ("<unknown>".to_string(), 0, 0);
        };
        let len = src.len() as u32;
        let start = (span.start - base).min(len);
        let end = span.end.saturating_sub(*base).clamp(start, len);
        (path.clone(), start, end)
    }

    /// `file:line:col` for a program-wide offset.
    pub fn locate(&self, span: ByteSpan) -> String {
        let at = span.start;
        let Some((path, src, base)) = self.files.iter().rev().find(|(_, _, b)| *b <= at) else {
            return "<unknown>".to_string();
        };
        let off = ((at - base) as usize).min(src.len());
        let upto = &src[..off];
        let line = upto.matches('\n').count() + 1;
        let col = upto.rfind('\n').map_or(upto.len(), |nl| upto.len() - nl - 1) + 1;
        format!("{path}:{line}:{col}")
    }
}
