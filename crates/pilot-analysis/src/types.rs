//! Name-blind physical layout types and layout registry.

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct LayoutId(pub u32);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct CapabilityBits(pub u8);

pub type CapabilityPhysics = CapabilityBits;

impl CapabilityBits {
    pub const IMMUTABLE: Self = Self(0);
    pub const MUTABLE: Self = Self(1);
    pub const UNIQUE: Self = Self(2);
    pub const LINEAR: Self = Self(4);
    pub const PINNED: Self = Self(8);

    pub const VALUE: Self = Self(0);
    pub const BORROW_REF: Self = Self(1);
    pub const OWNED_OBJ: Self = Self(4);
    pub const SCOPED_GUARD: Self = Self(5);

    #[inline(always)]
    pub const fn from_raw(b: u8) -> Self {
        Self(b)
    }

    #[inline(always)]
    pub const fn raw(self) -> u8 {
        self.0
    }

    #[inline(always)]
    pub const fn is_mutable(self) -> bool {
        (self.0 & 1) != 0
    }

    #[inline(always)]
    pub const fn is_unique(self) -> bool {
        (self.0 & 2) != 0
    }

    #[inline(always)]
    pub const fn is_linear(self) -> bool {
        (self.0 & 4) != 0
    }

    #[inline(always)]
    pub const fn is_pinned(self) -> bool {
        (self.0 & 8) != 0
    }

    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PhysicsGeometry {
    Scalar {
        bit_width: u16,
        lanes: u16,
        bank: u8,
    },
    Pointer {
        target: LayoutId,
        capability: CapabilityBits,
        addr_space: u8,
    },
    Aggregate {
        fields: Vec<FieldOffset>,
        is_soa: bool,
        is_packed: bool,
    },
    TaggedUnion {
        tag_bits: u8,
        payloads: Vec<LayoutId>,
    },
    CallFrame {
        params: Vec<LayoutId>,
        ret: LayoutId,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FieldOffset {
    pub positional_index: u16,
    pub layout: LayoutId,
    pub byte_offset: u32,
    pub element_stride: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalLayout {
    pub id: LayoutId,
    pub byte_size: u32,
    pub byte_align: u32,
    pub geometry: PhysicsGeometry,
}

#[derive(Default, Clone, Debug)]
pub struct LayoutTable {
    layouts: Vec<PhysicalLayout>,
}

impl LayoutTable {
    pub fn new() -> Self {
        Self { layouts: Vec::new() }
    }

    pub fn register_scalar(&mut self, bit_width: u16, lanes: u16, bank: u8) -> LayoutId {
        let id = LayoutId(self.layouts.len() as u32);
        let bit_total = bit_width as u32 * lanes as u32;
        let byte_size = if bit_total == 0 { 0 } else { (bit_total + 7) / 8 };
        let byte_align = byte_size.next_power_of_two().max(1).min(16);
        self.layouts.push(PhysicalLayout {
            id,
            byte_size,
            byte_align,
            geometry: PhysicsGeometry::Scalar { bit_width, lanes, bank },
        });
        id
    }

    pub fn register_pointer(&mut self, target: LayoutId, cap: CapabilityBits, space: u8) -> LayoutId {
        let id = LayoutId(self.layouts.len() as u32);
        self.layouts.push(PhysicalLayout {
            id,
            byte_size: 8,
            byte_align: 8,
            geometry: PhysicsGeometry::Pointer { target, capability: cap, addr_space: space },
        });
        id
    }

    pub fn register_aggregate(&mut self, field_layouts: &[LayoutId], is_soa: bool, is_packed: bool) -> LayoutId {
        let mut fields = Vec::with_capacity(field_layouts.len());
        let mut current_offset: u32 = 0;
        let mut max_align: u32 = 1;

        for (idx, &fl_id) in field_layouts.iter().enumerate() {
            let (f_size, f_align) = match self.get(fl_id) {
                Some(l) => (l.byte_size, l.byte_align),
                None => (8, 8),
            };
            if !is_packed {
                max_align = max_align.max(f_align);
                let padding = (f_align.wrapping_sub(current_offset % f_align)) % f_align;
                current_offset += padding;
            }
            fields.push(FieldOffset {
                positional_index: idx as u16,
                layout: fl_id,
                byte_offset: current_offset,
                element_stride: f_size,
            });
            current_offset += f_size;
        }

        let total_size = if !is_packed && max_align > 0 {
            let final_padding = (max_align.wrapping_sub(current_offset % max_align)) % max_align;
            current_offset + final_padding
        } else {
            current_offset
        };

        let id = LayoutId(self.layouts.len() as u32);
        self.layouts.push(PhysicalLayout {
            id,
            byte_size: total_size,
            byte_align: max_align,
            geometry: PhysicsGeometry::Aggregate { fields, is_soa, is_packed },
        });
        id
    }

    pub fn register_union(&mut self, tag_bits: u8, payloads: &[LayoutId]) -> LayoutId {
        let tag_bytes = ((tag_bits as u32) + 7) / 8;
        let max_payload = payloads.iter().map(|&p| self.get(p).map_or(0, |l| l.byte_size)).max().unwrap_or(0);
        let max_align = payloads.iter().map(|&p| self.get(p).map_or(1, |l| l.byte_align)).max().unwrap_or(1).max(tag_bytes.next_power_of_two());
        let total_size = ((tag_bytes + max_payload + max_align - 1) / max_align) * max_align;

        let id = LayoutId(self.layouts.len() as u32);
        self.layouts.push(PhysicalLayout {
            id,
            byte_size: total_size,
            byte_align: max_align,
            geometry: PhysicsGeometry::TaggedUnion { tag_bits, payloads: payloads.to_vec() },
        });
        id
    }

    pub fn register_call_frame(&mut self, params: &[LayoutId], ret: LayoutId) -> LayoutId {
        let mut total_size = 0;
        for &p in params {
            let sz = self.get(p).map_or(8, |l| l.byte_size);
            total_size += (sz + 7) / 8 * 8;
        }
        let id = LayoutId(self.layouts.len() as u32);
        self.layouts.push(PhysicalLayout {
            id,
            byte_size: total_size,
            byte_align: 8,
            geometry: PhysicsGeometry::CallFrame { params: params.to_vec(), ret },
        });
        id
    }

    pub fn get(&self, id: LayoutId) -> Option<&PhysicalLayout> {
        self.layouts.get(id.0 as usize)
    }

    pub fn len(&self) -> usize {
        self.layouts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layouts.is_empty()
    }
}
