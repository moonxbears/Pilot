//! Number literals with a suffix. A scalar type's name converts the number to
//! that type (`200u8` is `200 as u8`, `1.5f32` is `1.5 as f32`, `0xFFu8`);
//! any other word is a unit (`250ms`), valued by the module that binds
//! `bind_unit_literal`.
//!
//! Whether a word names a scalar type comes from the register layouts the
//! `[Bits]` descriptors declare: a type whose register class is a signed or
//! unsigned general-purpose register or a float register. No type name is
//! written here, so renaming a primitive renames its suffix.

use crate::*;

/// Register classes a literal can be converted to, in the `[Bits]` descriptor
/// vocabulary.
const SIGNED_CLASS: &str = "gpr_signed";
const UNSIGNED_CLASS: &str = "gpr_unsigned";
const FLOAT_CLASS: &str = "simd_float";

/// The number a literal spells before its suffix.
enum LitValue {
    Int(u128),
    Float(f64),
}

impl<'a> Parser<'a> {
    /// The suffixed number literal at the cursor, whose span is `sp`.
    pub(crate) fn parse_suffixed_number(&mut self, sp: ByteSpan) -> NodeId {
        let kind = self.peek();
        let text = self.text();
        let (digits, suffix) = text.split_at(suffix_start(text.as_bytes(), kind));
        let layout = self
            .tables
            .reg_layout(suffix)
            .filter(|l| matches!(l.class.as_str(), SIGNED_CLASS | UNSIGNED_CLASS | FLOAT_CLASS))
            .cloned();
        let decimal = matches!(kind, TokenKind::IntLit | TokenKind::FloatLit);
        let Some(layout) = layout else {
            if decimal {
                let sym = self.intern_cur();
                return self.arena.alloc_node(NodeKind::UnitLit, 1, sp, sym.0, 0);
            }
            self.advance();
            return self.number_error(sp, format!("`{text}`: a hexadecimal, binary or octal literal takes a type suffix, not `{suffix}`"));
        };
        self.advance();
        let Some(value) = literal_value(digits, kind) else {
            return self.number_error(sp, format!("`{text}`: `{digits}` is not a number"));
        };
        let number = match (value, layout.class.as_str()) {
            (LitValue::Float(_), SIGNED_CLASS | UNSIGNED_CLASS) => {
                return self.number_error(sp, format!("`{text}`: a number with a fraction or exponent cannot become the integer type `{suffix}`"));
            }
            (LitValue::Float(f), _) => self.arena.alloc_float(f, sp),
            (LitValue::Int(v), FLOAT_CLASS) => self.arena.alloc_float(v as f64, sp),
            (LitValue::Int(v), class) => {
                let signed = class == SIGNED_CLASS;
                let bits = layout.width.clamp(1, 128) as u32;
                let max = if signed { max_for(bits - 1) } else { max_for(bits) };
                if v > max {
                    let hint = if signed { format!("; write a negative value as `(-{v}) as {suffix}`") } else { String::new() };
                    return self.number_error(sp, format!("`{text}`: {v} does not fit `{suffix}` ({}-bit {}, at most {max}){hint}", layout.width, if signed { "signed" } else { "unsigned" }));
                }
                if v > u64::MAX as u128 {
                    return self.number_error(sp, format!("`{text}`: integer literals wider than 64 bits are not supported yet"));
                }
                self.arena.alloc_int(v as u64 as i64, sp)
            }
        };
        let sym = self.arena.interner.intern(suffix);
        let ty = self.arena.alloc_type_named(sym, &[], sp);
        self.arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::As as u16, sp, number.0, ty.0)
    }

    fn number_error(&mut self, sp: ByteSpan, message: String) -> NodeId {
        self.arena.diagnostics.push((sp, format!("SyntaxError: {message}")));
        self.arena.alloc_int(0, sp)
    }
}

/// The largest value of `bits` bits.
fn max_for(bits: u32) -> u128 {
    if bits >= 128 {
        u128::MAX
    } else {
        (1u128 << bits) - 1
    }
}

/// Where the suffix of number literal `b` (of token kind `kind`) starts.
fn suffix_start(b: &[u8], kind: TokenKind) -> usize {
    let run = |mut i: usize, ok: &dyn Fn(u8) -> bool| {
        while i < b.len() && (ok(b[i]) || b[i] == b'_') {
            i += 1;
        }
        i
    };
    match kind {
        TokenKind::HexIntLit => run(2, &|c| c.is_ascii_hexdigit()),
        TokenKind::BinIntLit => run(2, &|c| c == b'0' || c == b'1'),
        TokenKind::OctIntLit => run(2, &|c| (b'0'..=b'7').contains(&c)),
        _ => {
            let digit = |c: u8| c.is_ascii_digit();
            let mut i = run(0, &digit);
            if i + 1 < b.len() && b[i] == b'.' && b[i + 1].is_ascii_digit() {
                i = run(i + 1, &digit);
            }
            if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
                let next = b.get(i + 1).copied().unwrap_or(0);
                let signed = (next == b'+' || next == b'-') && b.get(i + 2).is_some_and(|c| c.is_ascii_digit());
                if next.is_ascii_digit() || signed {
                    i = run(i + if signed { 2 } else { 1 }, &digit);
                }
            }
            i
        }
    }
}

/// The value `digits` spells for a literal of token kind `kind`.
fn literal_value(digits: &str, kind: TokenKind) -> Option<LitValue> {
    let clean = digits.replace('_', "");
    let radix = match kind {
        TokenKind::HexIntLit => 16,
        TokenKind::BinIntLit => 2,
        TokenKind::OctIntLit => 8,
        _ => 10,
    };
    if radix != 10 {
        return u128::from_str_radix(&clean[2..], radix).ok().map(LitValue::Int);
    }
    if clean.bytes().all(|c| c.is_ascii_digit()) {
        return clean.parse::<u128>().ok().map(LitValue::Int);
    }
    clean.parse::<f64>().ok().map(LitValue::Float)
}
