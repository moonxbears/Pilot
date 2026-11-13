//! Tag-dispatched arithmetic, comparison and logic for `PilotVal` operands.
//!
//! Every function either returns the result of the operation for the operand
//! kinds it receives or an error message naming the operation and the kinds.
//! String concatenation through `+` is handled by the VM because it needs the
//! heap to render list and record operands.

use crate::instruction::Opcode;
use pilot_rt::PilotVal;
use std::cmp::Ordering;

pub type OpResult = Result<PilotVal, String>;

enum Num {
    Int(i64),
    /// An integer in an unsigned 64-bit register.
    UInt(u64),
    Float(f64),
}

fn num(v: PilotVal) -> Option<Num> {
    // A character counts as its code point.
    if let Some(c) = v.as_char() {
        return Some(Num::UInt(c as u64));
    }
    match v.as_int() {
        Some(i) if v.is_uint() => Some(Num::UInt(i as u64)),
        Some(i) => Some(Num::Int(i)),
        None => v.as_float().map(Num::Float),
    }
}

fn as_f64(n: &Num) -> f64 {
    match n {
        Num::Int(i) => *i as f64,
        Num::UInt(u) => *u as f64,
        Num::Float(f) => *f,
    }
}

/// The exact value of an integer operand, whatever its signedness.
fn wide(n: &Num) -> Option<i128> {
    match n {
        Num::Int(i) => Some(*i as i128),
        Num::UInt(u) => Some(*u as i128),
        Num::Float(_) => None,
    }
}

/// Both operands as unsigned 64-bit integers when both are integers and at
/// least one is unsigned: the signed one is read as its two's-complement bits.
fn unsigned_pair(x: &Num, y: &Num) -> Option<(u64, u64)> {
    match (x, y) {
        (Num::UInt(a), Num::UInt(b)) => Some((*a, *b)),
        (Num::UInt(a), Num::Int(b)) => Some((*a, *b as u64)),
        (Num::Int(a), Num::UInt(b)) => Some((*a as u64, *b)),
        _ => None,
    }
}

pub fn symbol(op: Opcode) -> &'static str {
    match op {
        Opcode::Add => "+",
        Opcode::Sub => "-",
        Opcode::Mul => "*",
        Opcode::Div => "/",
        Opcode::Rem => "%",
        Opcode::Pow => "**",
        Opcode::FloorDiv => "//",
        Opcode::BitAnd => "&",
        Opcode::BitOr => "|",
        Opcode::Xor => "^",
        Opcode::Shl => "<<",
        Opcode::Shr => ">>",
        Opcode::CmpEq => "==",
        Opcode::CmpNe => "!=",
        Opcode::CmpLt => "<",
        Opcode::CmpLe => "<=",
        Opcode::CmpGt => ">",
        Opcode::CmpGe => ">=",
        Opcode::Neg => "unary -",
        Opcode::Not => "!",
        Opcode::BitNot => "~",
        _ => "?",
    }
}

fn type_error(op: Opcode, a: PilotVal, b: PilotVal) -> String {
    format!(
        "cannot apply `{}` to {} and {}",
        symbol(op),
        a.kind_label(),
        b.kind_label()
    )
}

/// Numeric binary arithmetic: `+ - * / % ** //`.
pub fn arith(op: Opcode, a: PilotVal, b: PilotVal) -> OpResult {
    let (Some(x), Some(y)) = (num(a), num(b)) else {
        return Err(type_error(op, a, b));
    };
    if let (Num::Int(x), Num::Int(y)) = (&x, &y) {
        return int_arith(op, *x, *y);
    }
    if let Some((x, y)) = unsigned_pair(&x, &y) {
        return uint_arith(op, x, y);
    }
    let (x, y) = (as_f64(&x), as_f64(&y));
    let r = match op {
        Opcode::Add => x + y,
        Opcode::Sub => x - y,
        Opcode::Mul => x * y,
        Opcode::Div => x / y,
        Opcode::Rem => x % y,
        Opcode::Pow => x.powf(y),
        Opcode::FloorDiv => (x / y).floor(),
        _ => return Err(type_error(op, a, b)),
    };
    Ok(PilotVal::from_float(r))
}

fn int_arith(op: Opcode, x: i64, y: i64) -> OpResult {
    let zero_check = || {
        if y == 0 {
            Err("integer division by zero".to_string())
        } else {
            Ok(())
        }
    };
    let r = match op {
        Opcode::Add => x.wrapping_add(y),
        Opcode::Sub => x.wrapping_sub(y),
        Opcode::Mul => x.wrapping_mul(y),
        Opcode::Div => {
            zero_check()?;
            x.wrapping_div(y)
        }
        Opcode::Rem => {
            zero_check()?;
            x.wrapping_rem(y)
        }
        Opcode::FloorDiv => {
            zero_check()?;
            x.div_euclid(y) - if y < 0 && x.rem_euclid(y) != 0 { 1 } else { 0 }
        }
        Opcode::Pow => {
            if y < 0 {
                return Ok(PilotVal::from_float((x as f64).powf(y as f64)));
            }
            x.wrapping_pow(y.min(u32::MAX as i64) as u32)
        }
        _ => return Err(format!("cannot apply `{}` to integers", symbol(op))),
    };
    Ok(PilotVal::from_int(r))
}

/// Arithmetic on unsigned 64-bit integers, wrapping at 2^64.
fn uint_arith(op: Opcode, x: u64, y: u64) -> OpResult {
    if y == 0 && matches!(op, Opcode::Div | Opcode::Rem | Opcode::FloorDiv) {
        return Err("integer division by zero".to_string());
    }
    let r = match op {
        Opcode::Add => x.wrapping_add(y),
        Opcode::Sub => x.wrapping_sub(y),
        Opcode::Mul => x.wrapping_mul(y),
        Opcode::Div | Opcode::FloorDiv => x / y,
        Opcode::Rem => x % y,
        Opcode::Pow => x.wrapping_pow(y.min(u32::MAX as u64) as u32),
        _ => return Err(format!("cannot apply `{}` to integers", symbol(op))),
    };
    Ok(PilotVal::from_uint(r))
}

/// Bitwise operators. Integers combine bitwise; booleans combine logically
/// (without short-circuiting) for `& | ^`. When either operand is unsigned
/// the result is unsigned and `>>` shifts in zeros.
pub fn bitwise(op: Opcode, a: PilotVal, b: PilotVal) -> OpResult {
    if let Some((x, y)) = num(a).zip(num(b)).and_then(|(x, y)| unsigned_pair(&x, &y)) {
        let r = match op {
            Opcode::BitAnd => x & y,
            Opcode::BitOr => x | y,
            Opcode::Xor => x ^ y,
            Opcode::Shl => x.wrapping_shl(y as u32),
            Opcode::Shr => x.wrapping_shr(y as u32),
            _ => return Err(type_error(op, a, b)),
        };
        return Ok(PilotVal::from_uint(r));
    }
    if let (Some(x), Some(y)) = (a.as_int(), b.as_int()) {
        let r = match op {
            Opcode::BitAnd => x & y,
            Opcode::BitOr => x | y,
            Opcode::Xor => x ^ y,
            Opcode::Shl => x.wrapping_shl(y as u32),
            Opcode::Shr => x.wrapping_shr(y as u32),
            _ => return Err(type_error(op, a, b)),
        };
        return Ok(PilotVal::from_int(r));
    }
    if let (Some(x), Some(y)) = (a.as_bool(), b.as_bool()) {
        let r = match op {
            Opcode::BitAnd => x & y,
            Opcode::BitOr => x | y,
            Opcode::Xor => x ^ y,
            _ => return Err(type_error(op, a, b)),
        };
        return Ok(PilotVal::from_bool(r));
    }
    Err(type_error(op, a, b))
}

/// Structural equality: numbers by value (across int/float), strings by
/// content, everything else by tag and payload (handles by identity).
pub fn equals(a: PilotVal, b: PilotVal) -> bool {
    if let (Some(x), Some(y)) = (num(a), num(b)) {
        return match (wide(&x), wide(&y)) {
            (Some(x), Some(y)) => x == y,
            _ => as_f64(&x) == as_f64(&y),
        };
    }
    if a.is_str() && b.is_str() {
        return a
            .with_str(|x| b.with_str(|y| x == y).unwrap_or(false))
            .unwrap_or(false);
    }
    a == b
}

fn ordering(op: Opcode, a: PilotVal, b: PilotVal) -> Result<Ordering, String> {
    if let (Some(x), Some(y)) = (num(a), num(b)) {
        let ord = match (wide(&x), wide(&y)) {
            (Some(x), Some(y)) => Some(x.cmp(&y)),
            _ => as_f64(&x).partial_cmp(&as_f64(&y)),
        };
        return ord.ok_or_else(|| "comparison with NaN".to_string());
    }
    if a.is_str() && b.is_str() {
        if let Some(o) = a.with_str(|x| b.with_str(|y| x.cmp(y))).flatten() {
            return Ok(o);
        }
    }
    if let (Some(x), Some(y)) = (a.as_bool(), b.as_bool()) {
        return Ok(x.cmp(&y));
    }
    Err(type_error(op, a, b))
}

pub fn compare(op: Opcode, a: PilotVal, b: PilotVal) -> OpResult {
    let r = match op {
        Opcode::CmpEq => equals(a, b),
        Opcode::CmpNe => !equals(a, b),
        Opcode::CmpLt => ordering(op, a, b)?.is_lt(),
        Opcode::CmpLe => ordering(op, a, b)?.is_le(),
        Opcode::CmpGt => ordering(op, a, b)?.is_gt(),
        Opcode::CmpGe => ordering(op, a, b)?.is_ge(),
        _ => return Err(type_error(op, a, b)),
    };
    Ok(PilotVal::from_bool(r))
}

pub fn unary(op: Opcode, a: PilotVal) -> OpResult {
    let r = match op {
        Opcode::Neg => match num(a) {
            Some(Num::Int(i)) => PilotVal::from_int(i.wrapping_neg()),
            Some(Num::UInt(u)) => PilotVal::from_uint(u.wrapping_neg()),
            Some(Num::Float(f)) => PilotVal::from_float(-f),
            None => return Err(format!("cannot negate {}", a.kind_label())),
        },
        Opcode::Not => match a.as_bool() {
            Some(b) => PilotVal::from_bool(!b),
            None => return Err(format!("cannot apply `!` to {}", a.kind_label())),
        },
        Opcode::BitNot => match a.as_int() {
            Some(i) if a.is_uint() => PilotVal::from_uint(!(i as u64)),
            Some(i) => PilotVal::from_int(!i),
            None => return Err(format!("cannot apply `~` to {}", a.kind_label())),
        },
        _ => return Err(format!("`{}` is not a unary operator", symbol(op))),
    };
    Ok(r)
}

/// Branch condition. Only booleans are conditions.
pub fn truth(v: PilotVal) -> Result<bool, String> {
    v.as_bool()
        .ok_or_else(|| format!("condition must be a boolean, found {}", v.kind_label()))
}

/// Converts a number (or boolean) to the register layout of a primitive
/// type: `class` is the descriptor's register class and `width` its bit
/// width. Floats go to integers by truncation toward zero; integers wrap to
/// `width` bits; a 1-bit layout is a boolean.
/// Whether `v` already is a value of a type laid out in `class` registers of
/// `width` bits: a bool for one bit, a float for a float class, an integer
/// for an integer class.
pub fn fits(v: PilotVal, class: &str, width: i64) -> bool {
    if width == 1 {
        return v.as_bool().is_some();
    }
    match class {
        "simd_float" => v.as_float().is_some(),
        "gpr_signed" | "gpr_unsigned" => v.as_int().is_some(),
        _ => false,
    }
}

/// `v` as a character: a character itself, or the character an integer
/// codes. An integer that is not a Unicode scalar value (above 0x10FFFF or
/// a surrogate) is an error; any other value is given back as it is.
pub fn scalar_from(v: PilotVal) -> Result<PilotVal, String> {
    if v.as_char().is_some() {
        return Ok(v);
    }
    match num(v) {
        Some(Num::Int(i)) => scalar_of(u64::try_from(i).ok(), i.to_string()),
        Some(Num::UInt(u)) => scalar_of(Some(u), u.to_string()),
        _ => Ok(v),
    }
}

fn scalar_of(code: Option<u64>, shown: String) -> Result<PilotVal, String> {
    code.and_then(|u| u32::try_from(u).ok())
        .and_then(char::from_u32)
        .map(PilotVal::from_char)
        .ok_or_else(|| format!("{shown} is not a Unicode scalar value"))
}

/// The result of arithmetic in a register of `class` and `width`: an
/// integer wrapped to an integer register's width, a float rounded to a
/// float register's. Any other value (a record an operator method made, a
/// string) is given back as it is.
pub fn wrap(v: PilotVal, class: &str, width: i64) -> Result<PilotVal, String> {
    let is_int_class = matches!(class, "gpr_signed" | "gpr_unsigned");
    match num(v) {
        Some(Num::Int(_)) | Some(Num::UInt(_)) if is_int_class && width > 1 => convert(v, class, width),
        Some(Num::Float(_)) if class == "simd_float" => convert(v, class, width),
        _ => Ok(v),
    }
}

pub fn convert(v: PilotVal, class: &str, width: i64) -> Result<PilotVal, String> {
    let is_int_class = matches!(class, "gpr_signed" | "gpr_unsigned");
    // A heap handle (record, list, function) in a pointer-width integer
    // register is still that handle: Pilot code types handles as integers.
    let is_handle = v.as_map_handle().is_some()
        || v.as_list_handle().is_some()
        || v.as_fn_handle().is_some();
    if is_handle && is_int_class && width >= 64 {
        return Ok(v);
    }
    let n = match (num(v), v.as_bool()) {
        (Some(n), _) => n,
        (None, Some(b)) => Num::Int(b as i64),
        _ => return Err(format!("cannot convert {} to a number", v.kind_label())),
    };
    if width == 1 {
        return Ok(PilotVal::from_bool(as_f64(&n) != 0.0));
    }
    if width <= 0 {
        return Err("cannot convert a value to a type with no bits".to_string());
    }
    match class {
        "simd_float" => {
            let f = as_f64(&n);
            Ok(PilotVal::from_float(if width == 32 { f as f32 as f64 } else { f }))
        }
        _ if is_int_class => {
            let unsigned = class == "gpr_unsigned";
            let i = match n {
                Num::Int(i) => i,
                Num::UInt(u) => u as i64,
                Num::Float(f) if unsigned && f.is_finite() && f.trunc() >= 0.0 && f.trunc() < u64::MAX as f64 => {
                    f.trunc() as u64 as i64
                }
                Num::Float(f) if f.is_finite() && f.trunc() >= i64::MIN as f64 && f.trunc() < i64::MAX as f64 => {
                    f.trunc() as i64
                }
                Num::Float(f) => return Err(format!("{f} does not fit an integer")),
            };
            if width >= 64 {
                return Ok(if unsigned { PilotVal::from_uint(i as u64) } else { PilotVal::from_int(i) });
            }
            let shift = 64 - width as u32;
            let wrapped = if class == "gpr_signed" {
                (i << shift) >> shift
            } else {
                ((i as u64) << shift >> shift) as i64
            };
            Ok(PilotVal::from_int(wrapped))
        }
        other => Err(format!("no numeric conversion to a `{other}` register")),
    }
}
