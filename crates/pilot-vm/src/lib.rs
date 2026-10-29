//! Milestone 34.3: Register-Based `PilVM` v3 & `.pilc` v3 Zero-Copy Binary Format (`pilot-vm`).
//!
//! Enforces:
//! - 32-bit fixed-width 3-address register instructions (`[opcode:8 | r_dst:8 | r_src1:8 | r_src2:8]`).
//! - Zero-deserialization `.pilc` v3 format with 64-byte aligned comptime pools and direct memory execution.
//! - Fast register VM execution loop with debugger hook traps (`VMHook`).

pub mod instruction;
pub mod pilc;
pub mod vm;

pub use instruction::{Instruction, Opcode};
pub use pilc::{PilcBinary, PILC_V3_MAGIC, PILC_V3_VERSION};
pub use vm::{PilVirtualMachine, VmHook};

#[cfg(test)]
mod tests {
    use super::*;
    use pilot_rt::PilotVal;

    #[test]
    fn test_instruction_32bit_fixed_width_encoding() {
        let ins = Instruction::encode(Opcode::AddI64, 0, 1, 2);
        assert_eq!(ins.opcode(), Opcode::AddI64);
        assert_eq!(ins.dst(), 0);
        assert_eq!(ins.src1(), 1);
        assert_eq!(ins.src2(), 2);
        assert_eq!(core::mem::size_of::<Instruction>(), 4);
    }

    #[test]
    fn test_pilc_v3_zero_copy_serialization_and_loading() {
        let constants = vec![
            PilotVal::from_int(100),
            PilotVal::from_str_sso("test_sso").unwrap(),
        ];
        let instructions = vec![
            Instruction::encode(Opcode::LoadConst, 1, 0, 0),
            Instruction::encode(Opcode::LoadConst, 2, 0, 1),
            Instruction::encode(Opcode::Ret, 0, 1, 0),
        ];

        let binary = PilcBinary::new(constants, instructions);
        let serialized = binary.serialize();
        assert!(serialized.starts_with(PILC_V3_MAGIC));

        let loaded = PilcBinary::load_from_bytes(&serialized).expect("Failed to load .pilc v3");
        assert_eq!(loaded.constants.len(), 2);
        assert_eq!(loaded.constants[0].as_int(), Some(100));
        assert_eq!(loaded.instructions.len(), 3);
        assert_eq!(loaded.instructions[0].opcode(), Opcode::LoadConst);
    }

    #[test]
    fn test_pil_virtual_machine_register_arithmetic_and_return() {
        // Program computing (10 + 32) -> 42
        let constants = vec![
            PilotVal::from_int(10),
            PilotVal::from_int(32),
        ];
        let instructions = vec![
            Instruction::encode(Opcode::LoadConst, 1, 0, 0), // r1 = 10
            Instruction::encode(Opcode::LoadConst, 2, 0, 1), // r2 = 32
            Instruction::encode(Opcode::AddI64, 3, 1, 2),    // r3 = r1 + r2
            Instruction::encode(Opcode::Ret, 0, 3, 0),       // return r3
        ];

        let binary = PilcBinary::new(constants, instructions);
        let mut vm = PilVirtualMachine::new();
        vm.load_binary(binary);

        let res = vm.run();
        assert_eq!(res.as_int(), Some(42));
    }
}
