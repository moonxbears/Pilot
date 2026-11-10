//! 32-bit fixed-width 3-address register instruction architecture (`[opcode:8 | dst:8 | src1:8 | src2:8]`).
//!
//! Arithmetic and comparison opcodes are polymorphic over the runtime value
//! tag: the VM selects integer, float or string behaviour from the operand
//! kinds and faults on operands the operation does not accept.
//!
//! Jump encodings: `Jump` keeps its 16-bit target in `src1:src2`; the
//! conditional jumps keep the tested register in `src1` and the target split
//! across `dst` (high byte) and `src2` (low byte). `LoadConst` and `Trap` keep a
//! 16-bit constant-pool index in `src1:src2`.

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Opcode {
    Nop = 0,
    Add = 1,
    Sub = 2,
    Mul = 3,
    Div = 4,
    Rem = 5,
    Pow = 6,
    FloorDiv = 7,
    BitAnd = 8,
    LoadConst = 9,
    Move = 10,
    CmpEq = 11,
    CmpLt = 12,
    Jump = 13,
    JumpIfTrue = 14,
    JumpIfFalse = 15,
    Call = 16,
    Ret = 17,
    Halt = 18,
    CmpGt = 19,
    CmpLe = 20,
    CmpGe = 21,
    CmpNe = 22,
    BitOr = 23,
    Neg = 24,
    BitNot = 25,
    Not = 26,
    JumpIfNull = 27,
    JumpIfNotNull = 28,
    Xor = 31,
    Shl = 32,
    Shr = 33,
    EngineIntrinsic = 34,
    /// Raises a runtime fault whose message is the string constant at `src1:src2`.
    Trap = 35,
    /// Calls the function value in `src1` with `src2` arguments staged in
    /// `r1..`; the result lands in `dst`.
    CallClosure = 36,
    /// Any byte that does not decode to a known opcode. Executing it faults.
    Invalid = 255,
}

const OPCODE_TABLE: [Opcode; 35] = [
    Opcode::Nop,
    Opcode::Add,
    Opcode::Sub,
    Opcode::Mul,
    Opcode::Div,
    Opcode::Rem,
    Opcode::Pow,
    Opcode::FloorDiv,
    Opcode::BitAnd,
    Opcode::LoadConst,
    Opcode::Move,
    Opcode::CmpEq,
    Opcode::CmpLt,
    Opcode::Jump,
    Opcode::JumpIfTrue,
    Opcode::JumpIfFalse,
    Opcode::Call,
    Opcode::Ret,
    Opcode::Halt,
    Opcode::CmpGt,
    Opcode::CmpLe,
    Opcode::CmpGe,
    Opcode::CmpNe,
    Opcode::BitOr,
    Opcode::Neg,
    Opcode::BitNot,
    Opcode::Not,
    Opcode::JumpIfNull,
    Opcode::JumpIfNotNull,
    Opcode::Xor,
    Opcode::Shl,
    Opcode::Shr,
    Opcode::EngineIntrinsic,
    Opcode::Trap,
    Opcode::CallClosure,
];

impl Opcode {
    #[inline]
    pub fn from_u8(v: u8) -> Self {
        OPCODE_TABLE
            .iter()
            .copied()
            .find(|op| *op as u8 == v)
            .unwrap_or(Opcode::Invalid)
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Instruction(pub u32);

impl Instruction {
    #[inline]
    pub const fn encode(op: Opcode, r_dst: u8, r_src1: u8, r_src2: u8) -> Self {
        Self(((op as u32) << 24) | ((r_dst as u32) << 16) | ((r_src1 as u32) << 8) | (r_src2 as u32))
    }

    /// Encodes an instruction whose 16-bit operand lives in `src1:src2`.
    #[inline]
    pub const fn encode_wide(op: Opcode, r_dst: u8, wide: u16) -> Self {
        Self::encode(op, r_dst, (wide >> 8) as u8, (wide & 0xFF) as u8)
    }

    /// Encodes a conditional jump testing `r_cond` with target `target`.
    #[inline]
    pub const fn encode_branch(op: Opcode, r_cond: u8, target: u16) -> Self {
        Self::encode(op, (target >> 8) as u8, r_cond, (target & 0xFF) as u8)
    }

    #[inline]
    pub fn opcode(self) -> Opcode {
        Opcode::from_u8((self.0 >> 24) as u8)
    }

    #[inline]
    pub const fn dst(self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }

    #[inline]
    pub const fn src1(self) -> u8 {
        ((self.0 >> 8) & 0xFF) as u8
    }

    #[inline]
    pub const fn src2(self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    #[inline]
    pub const fn wide(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    #[inline]
    pub const fn branch_target(self) -> u16 {
        ((self.dst() as u16) << 8) | (self.src2() as u16)
    }
}

impl core::fmt::Debug for Instruction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?} r{}, r{}, r{}", self.opcode(), self.dst(), self.src1(), self.src2())
    }
}
