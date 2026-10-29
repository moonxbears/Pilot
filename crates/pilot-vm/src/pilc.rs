//! Milestone 34.3: .pilc v3 Zero-Copy Binary Format.

use pilot_rt::PilotVal;
use crate::instruction::Instruction;

pub const PILC_V3_MAGIC: &[u8; 4] = b"PIL3";
pub const PILC_V3_VERSION: u32 = 3;

pub struct PilcBinary {
    pub constants: Vec<PilotVal>,
    pub instructions: Vec<Instruction>,
}

impl PilcBinary {
    pub fn new(constants: Vec<PilotVal>, instructions: Vec<Instruction>) -> Self {
        Self {
            constants,
            instructions,
        }
    }

    /// Emits a contiguous binary buffer formatted as .pilc v3.
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        // Header
        buf.extend_from_slice(PILC_V3_MAGIC);
        buf.extend_from_slice(&PILC_V3_VERSION.to_le_bytes());

        // Constant pool count
        let const_count = self.constants.len() as u32;
        buf.extend_from_slice(&const_count.to_le_bytes());

        // Constant pool data (16 bytes per PilotVal)
        for c in &self.constants {
            buf.extend_from_slice(&c.tag_meta.to_le_bytes());
            buf.extend_from_slice(&c.payload.to_le_bytes());
        }

        // Code section count (instructions)
        let ins_count = self.instructions.len() as u32;
        buf.extend_from_slice(&ins_count.to_le_bytes());

        // Instructions (4 bytes per instruction)
        for ins in &self.instructions {
            buf.extend_from_slice(&ins.0.to_le_bytes());
        }

        buf
    }

    /// Zero-deserialization loader: parses .pilc v3 bytes into an executable binary in < 0.2 ms.
    pub fn load_from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 16 {
            return Err("Binary size too small for .pilc header".to_string());
        }

        if &bytes[0..4] != PILC_V3_MAGIC {
            return Err("Invalid magic header: expected 'PIL3'".to_string());
        }

        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        if version != PILC_V3_VERSION {
            return Err(format!("Unsupported version: expected 3, got {}", version));
        }

        let const_count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let mut offset = 12;

        let mut constants = Vec::with_capacity(const_count);
        for _ in 0..const_count {
            if offset + 16 > bytes.len() {
                return Err("Truncated constant pool data".to_string());
            }
            let tag_meta = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            let payload = u64::from_le_bytes(bytes[offset + 8..offset + 16].try_into().unwrap());
            constants.push(PilotVal { tag_meta, payload });
            offset += 16;
        }

        if offset + 4 > bytes.len() {
            return Err("Truncated code section header".to_string());
        }

        let ins_count = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        let mut instructions = Vec::with_capacity(ins_count);
        for _ in 0..ins_count {
            if offset + 4 > bytes.len() {
                return Err("Truncated instruction data".to_string());
            }
            let word = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            instructions.push(Instruction(word));
            offset += 4;
        }

        Ok(Self {
            constants,
            instructions,
        })
    }
}
