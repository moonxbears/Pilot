//! Operations that report their failure as a value instead of stopping the
//! program, so a Pilot-hosted executor (the PilIR interpreter,
//! `selfhost/ir/interp.pilot`) can surface the seed's own runtime error at
//! the place in the program it is running.
//!
//! `op_checked(name, args)` runs a binary operator (`name` is its symbol,
//! `+` .. `>=`), the branch-condition test (`if`), or the intrinsic `name`,
//! on the values of the list `args`. Its result is the list `[true, value]`,
//! or `[false, message]` with the message the seed reports for that failure.

use crate::instruction::Opcode;
use crate::ops;
use crate::vm::PilVirtualMachine;
use pilot_rt::PilotVal;

/// The binary operators by the opcode that runs each.
const BINARY: [Opcode; 18] = [
    Opcode::Add,
    Opcode::Sub,
    Opcode::Mul,
    Opcode::Div,
    Opcode::Rem,
    Opcode::Pow,
    Opcode::FloorDiv,
    Opcode::BitAnd,
    Opcode::BitOr,
    Opcode::Xor,
    Opcode::Shl,
    Opcode::Shr,
    Opcode::CmpEq,
    Opcode::CmpNe,
    Opcode::CmpLt,
    Opcode::CmpLe,
    Opcode::CmpGt,
    Opcode::CmpGe,
];

/// The name `op_checked` gives the branch-condition test.
const CONDITION: &str = "if";

impl PilVirtualMachine {
    /// A binary operator on two values, as the run loop executes it.
    pub(crate) fn binary_op(&mut self, op: Opcode, r1: PilotVal, r2: PilotVal) -> Result<PilotVal, String> {
        match op {
            Opcode::Add if r1.is_str() || r2.is_str() => {
                let mut s = self.display(r1);
                s.push_str(&self.display(r2));
                Ok(self.new_str(&s))
            }
            Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div | Opcode::Rem | Opcode::Pow | Opcode::FloorDiv => {
                ops::arith(op, r1, r2)
            }
            Opcode::BitAnd | Opcode::BitOr | Opcode::Xor | Opcode::Shl | Opcode::Shr => ops::bitwise(op, r1, r2),
            Opcode::CmpEq => Ok(PilotVal::from_bool(self.values_equal(r1, r2))),
            Opcode::CmpNe => Ok(PilotVal::from_bool(!self.values_equal(r1, r2))),
            _ => ops::compare(op, r1, r2),
        }
    }

    /// `op_checked(name, args)`.
    pub(crate) fn op_checked(&mut self, args: &[PilotVal]) -> Result<PilotVal, String> {
        let name = args
            .first()
            .and_then(|v| v.with_str(str::to_string))
            .ok_or("`op_checked` expects an operation name")?;
        let list = args.get(1).copied().ok_or("`op_checked` expects an argument list")?;
        let values = self.list_items(list).ok_or("`op_checked` expects an argument list")?;
        let outcome = if name == CONDITION {
            let v = values.first().copied().unwrap_or(PilotVal::NULL);
            ops::truth(v).map(PilotVal::from_bool)
        } else if let Some(&op) = BINARY.iter().find(|&&op| ops::symbol(op) == name) {
            match values.as_slice() {
                [a, b] => self.binary_op(op, *a, *b),
                _ => Err(format!("`{name}` takes two operands")),
            }
        } else {
            self.intrinsic(&name, &values)
        };
        Ok(match outcome {
            Ok(v) => self.new_list(vec![PilotVal::from_bool(true), v]),
            Err(message) => {
                let m = self.new_str(&message);
                self.new_list(vec![PilotVal::from_bool(false), m])
            }
        })
    }
}
