//! 64-bit fixed-width 3-address register instruction architecture: the low
//! word is `[opcode:8 | dst:8 | src1:8 | src2:8]`, the high word a 32-bit
//! operand (`ext`).
//!
//! Arithmetic and comparison opcodes are polymorphic over the runtime value
//! tag: the VM selects integer, float or string behaviour from the operand
//! kinds and faults on operands the operation does not accept.
//!
//! Jump encodings: `Jump` keeps its target in `ext`; the conditional jumps
//! keep the tested register in `src1` and the target in `ext`. `Call` keeps
//! its entry in `ext`, and `LoadConst` and `Trap` keep a constant-pool index
//! there.

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
    /// Calls `wide` with the arguments staged in `r1..`, reusing the current
    /// frame: the callee returns straight to this frame's caller (`become`).
    TailCall = 37,
    /// Copies `src1` into out-slot `dst` just before a `Ret`: the final value
    /// of a parameter that lends the caller's storage for changing (`var ref`).
    SetOut = 38,
    /// Loads out-slot `src1`, which the callee just returned, into `dst`.
    GetOut = 39,
    /// Any byte that does not decode to a known opcode. Executing it faults.
    Invalid = 255,
}

const OPCODE_TABLE: [Opcode; 38] = [
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
    Opcode::TailCall,
    Opcode::SetOut,
    Opcode::GetOut,
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
pub struct Instruction(pub u64);

impl Instruction {
    #[inline]
    pub const fn encode(op: Opcode, r_dst: u8, r_src1: u8, r_src2: u8) -> Self {
        Self(((op as u64) << 24) | ((r_dst as u64) << 16) | ((r_src1 as u64) << 8) | (r_src2 as u64))
    }

    /// Encodes an instruction whose 32-bit operand lives in `ext`.
    #[inline]
    pub const fn encode_wide(op: Opcode, r_dst: u8, wide: u32) -> Self {
        Self(Self::encode(op, r_dst, 0, 0).0 | ((wide as u64) << 32))
    }

    /// Encodes a conditional jump testing `r_cond` with target `target`.
    #[inline]
    pub const fn encode_branch(op: Opcode, r_cond: u8, target: u32) -> Self {
        Self(Self::encode(op, 0, r_cond, 0).0 | ((target as u64) << 32))
    }

    #[inline]
    pub fn opcode(self) -> Opcode {
        Opcode::from_u8(((self.0 >> 24) & 0xFF) as u8)
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
    pub const fn wide(self) -> u32 {
        (self.0 >> 32) as u32
    }

    #[inline]
    pub const fn branch_target(self) -> u32 {
        self.wide()
    }
}

impl core::fmt::Debug for Instruction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?} r{}, r{}, r{}", self.opcode(), self.dst(), self.src1(), self.src2())
    }
}
