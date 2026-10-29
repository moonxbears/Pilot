//! Milestone 34.3: 32-bit fixed-width 3-address register instruction architecture.

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Opcode {
    Nop = 0,
    AddI64 = 1,
    SubI64 = 2,
    MulI64 = 3,
    DivI64 = 4,
    AddF64 = 5,
    SubF64 = 6,
    MulF64 = 7,
    DivF64 = 8,
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
    RemI64 = 23,
    NegI64 = 24,
    NegF64 = 25,
    Not = 26,
    And = 27,
    Or = 28,
    Print = 29,
    Assert = 30,
    Xor = 31,
    Shl = 32,
    Shr = 33,
}

impl Opcode {
    pub const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::AddI64,
            2 => Self::SubI64,
            3 => Self::MulI64,
            4 => Self::DivI64,
            5 => Self::AddF64,
            6 => Self::SubF64,
            7 => Self::MulF64,
            8 => Self::DivF64,
            9 => Self::LoadConst,
            10 => Self::Move,
            11 => Self::CmpEq,
            12 => Self::CmpLt,
            13 => Self::Jump,
            14 => Self::JumpIfTrue,
            15 => Self::JumpIfFalse,
            16 => Self::Call,
            17 => Self::Ret,
            18 => Self::Halt,
            19 => Self::CmpGt,
            20 => Self::CmpLe,
            21 => Self::CmpGe,
            22 => Self::CmpNe,
            23 => Self::RemI64,
            24 => Self::NegI64,
            25 => Self::NegF64,
            26 => Self::Not,
            27 => Self::And,
            28 => Self::Or,
            29 => Self::Print,
            30 => Self::Assert,
            31 => Self::Xor,
            32 => Self::Shl,
            33 => Self::Shr,
            _ => Self::Nop,
        }
    }
}

/// Compact 32-bit fixed-width register instruction:
/// `[ opcode: 8 | r_dst: 8 | r_src1: 8 | r_src2: 8 ]`
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Instruction(pub u32);

impl Instruction {
    pub const fn encode(op: Opcode, r_dst: u8, r_src1: u8, r_src2: u8) -> Self {
        let word = ((op as u32) << 24)
            | ((r_dst as u32) << 16)
            | ((r_src1 as u32) << 8)
            | (r_src2 as u32);
        Self(word)
    }

    #[inline]
    pub const fn opcode(self) -> Opcode {
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
}

impl core::fmt::Debug for Instruction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:?} r{}, r{}, r{}",
            self.opcode(),
            self.dst(),
            self.src1(),
            self.src2()
        )
    }
}
