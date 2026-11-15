//! Integer literals.
//!
//! The engine's integers are 64 bits wide. A literal that fits an `i64` or a
//! `u64` is that value; a wider one (a 128-bit literal) has no value the
//! engine can hold, so it stops at the seam `convert.wide_scalar` instead
//! of becoming some other number.

use crate::Parser;
use pilot_ast::NodeId;
use pilot_span::ByteSpan;

/// The message of a literal wider than the engine's integers.
fn too_wide(text: &str) -> String {
    format!(
        "SEAM001: unconnected seam `convert.wide_scalar`: the integer literal `{text}` does not fit \
         64 bits, the widest integer the engine holds"
    )
}

impl<'a> Parser<'a> {
    /// The integer literal at the cursor, written in `radix` (10 for a
    /// decimal literal; 16, 2 or 8 after a two-character prefix).
    pub(crate) fn parse_int_lit(&mut self, sp: ByteSpan, radix: u32) -> NodeId {
        let text = self.text().to_string();
        let clean = text.replace('_', "");
        let digits = if radix == 10 { clean.as_str() } else { clean.get(2..).unwrap_or("") };
        let value = i64::from_str_radix(digits, radix)
            .or_else(|_| u64::from_str_radix(digits, radix).map(|u| u as i64));
        self.advance();
        match value {
            Ok(v) => self.arena.alloc_int(v, sp),
            Err(_) => {
                self.arena.diagnostics.push((sp, too_wide(&text)));
                self.arena.alloc_int(0, sp)
            }
        }
    }
}
