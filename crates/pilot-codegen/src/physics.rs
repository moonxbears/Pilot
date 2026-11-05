//! Name-blind hardware register and layout physics for codegen backends.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RegClass {
    Unit,
    NoReturn,
    GprSigned,
    GprUnsigned,
    SimdFloat,
    RawPtr,
    SimdVector { lanes: u8 },
    Composite,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HardwarePhysics {
    pub reg_class: RegClass,
    pub bit_width: u32,
}
