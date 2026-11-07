//! `.pilc` v3 Zero-Copy Binary Container & Name-Blind `VmHook` Debug Table.

use pilot_rt::PilotVal;
use pilot_span::ByteSpan;
use crate::instruction::Instruction;

pub const PILC_V3_MAGIC: &[u8; 4] = b"PIL3";
pub const PILC_V3_VERSION: u32 = 3;

#[derive(Clone, Debug)]
pub struct PilcBinary {
    pub constants: Vec<PilotVal>,
    pub instructions: Vec<Instruction>,
    pub pc_spans: Vec<ByteSpan>,
    pub reg_symbols: Vec<(u8, u32, u32)>,
}

impl PilcBinary {
    pub fn new(constants: Vec<PilotVal>, instructions: Vec<Instruction>) -> Self {
        Self { constants, instructions, pc_spans: Vec::new(), reg_symbols: Vec::new() }
    }

    pub fn with_debug_tables(
        constants: Vec<PilotVal>, instructions: Vec<Instruction>,
        pc_spans: Vec<ByteSpan>, reg_symbols: Vec<(u8, u32, u32)>,
    ) -> Self {
        Self { constants, instructions, pc_spans, reg_symbols }
    }

    #[inline]
    pub fn span_for_pc(&self, pc: usize) -> Option<ByteSpan> {
        self.pc_spans.get(pc).copied().filter(|s| s.start > 0 || s.end > 0)
    }

    #[inline]
    pub fn symbol_for_reg(&self, reg: u8) -> Option<(u32, u32)> {
        self.reg_symbols.iter().find(|&&(r, _, _)| r == reg).map(|&(_, sym, lay)| (sym, lay))
    }

    pub fn serialize(&self) -> Vec<u8> {
        let (mut buf, mut heap_str_data) = (Vec::new(), Vec::new());
        buf.extend_from_slice(PILC_V3_MAGIC);
        buf.extend_from_slice(&PILC_V3_VERSION.to_le_bytes());
        buf.extend_from_slice(&(self.constants.len() as u32).to_le_bytes());
        for c in &self.constants {
            if c.tag() == pilot_rt::TAG_HEAP_STR {
                let str_offset = heap_str_data.len() as u64;
                c.with_str(|s| {
                    heap_str_data.extend_from_slice(&(s.len() as u32).to_le_bytes());
                    heap_str_data.extend_from_slice(s.as_bytes());
                });
                buf.extend_from_slice(&c.tag_meta.to_le_bytes());
                buf.extend_from_slice(&str_offset.to_le_bytes());
            } else {
                buf.extend_from_slice(&c.tag_meta.to_le_bytes());
                buf.extend_from_slice(&c.payload.to_le_bytes());
            }
        }
        buf.extend_from_slice(&(heap_str_data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&heap_str_data);
        buf.extend_from_slice(&(self.instructions.len() as u32).to_le_bytes());
        for ins in &self.instructions { buf.extend_from_slice(&ins.0.to_le_bytes()); }
        buf
    }

    pub fn load_from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 16 { return Err("Binary size too small for .pilc header".into()); }
        if &bytes[0..4] != PILC_V3_MAGIC { return Err("Invalid magic header: expected 'PIL3'".into()); }
        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        if version != PILC_V3_VERSION { return Err(format!("Unsupported version: expected 3, got {}", version)); }
        let const_count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let (mut offset, mut raw_consts) = (12, Vec::with_capacity(const_count));
        for _ in 0..const_count {
            if offset + 16 > bytes.len() { return Err("Truncated constant pool data".into()); }
            let tag_meta = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            let payload = u64::from_le_bytes(bytes[offset + 8..offset + 16].try_into().unwrap());
            raw_consts.push((tag_meta, payload));
            offset += 16;
        }
        if offset + 4 > bytes.len() { return Err("Truncated heap string table header".into()); }
        let heap_len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if offset + heap_len > bytes.len() { return Err("Truncated heap string table data".into()); }
        let heap_str_data = &bytes[offset..offset + heap_len];
        offset += heap_len;
        let mut constants = Vec::with_capacity(const_count);
        for (tag_meta, payload) in raw_consts {
            if (tag_meta & 0xFF) as u8 == pilot_rt::TAG_HEAP_STR {
                let str_off = payload as usize;
                let s_len = if str_off + 4 <= heap_str_data.len() { u32::from_le_bytes(heap_str_data[str_off..str_off + 4].try_into().unwrap()) as usize } else { usize::MAX };
                if str_off.saturating_add(4).saturating_add(s_len) <= heap_str_data.len() {
                    constants.push(PilotVal::from_str(core::str::from_utf8(&heap_str_data[str_off + 4..str_off + 4 + s_len]).unwrap_or("")));
                } else { constants.push(PilotVal::NULL); }
            } else { constants.push(PilotVal { tag_meta, payload }); }
        }
        if offset + 4 > bytes.len() { return Err("Truncated code section header".into()); }
        let ins_count = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        let mut instructions = Vec::with_capacity(ins_count);
        for _ in 0..ins_count {
            if offset + 4 > bytes.len() { return Err("Truncated instruction data".into()); }
            instructions.push(Instruction(u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())));
            offset += 4;
        }
        Ok(Self::new(constants, instructions))
    }
}
