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
    Float(f64),
}

fn num(v: PilotVal) -> Option<Num> {
    if let Some(i) = v.as_int() {
        Some(Num::Int(i))
    } else {
        v.as_float().map(Num::Float)
    }
}

fn as_f64(n: &Num) -> f64 {
    match n {
        Num::Int(i) => *i as f64,
        Num::Float(f) => *f,
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

/// Bitwise operators. Integers combine bitwise; booleans combine logically
/// (without short-circuiting) for `& | ^`.
pub fn bitwise(op: Opcode, a: PilotVal, b: PilotVal) -> OpResult {
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
        return match (x, y) {
            (Num::Int(x), Num::Int(y)) => x == y,
            (x, y) => as_f64(&x) == as_f64(&y),
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
        let ord = match (x, y) {
            (Num::Int(x), Num::Int(y)) => Some(x.cmp(&y)),
            (x, y) => as_f64(&x).partial_cmp(&as_f64(&y)),
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
            Some(Num::Float(f)) => PilotVal::from_float(-f),
            None => return Err(format!("cannot negate {}", a.kind_label())),
        },
        Opcode::Not => match a.as_bool() {
            Some(b) => PilotVal::from_bool(!b),
            None => return Err(format!("cannot apply `!` to {}", a.kind_label())),
        },
        Opcode::BitNot => match a.as_int() {
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
