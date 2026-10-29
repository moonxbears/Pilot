//! Milestone 34.1: 16-Byte Unboxed C-ABI Value Representation (`PilotVal`).
//!
//! Stores `i8`..`i64`, `u8`..`u64`, `f32`/`f64`, `bool`, `char`, `KeywordId`,
//! and Small Strings (SSO up to 14 UTF-8 bytes directly inside `PilotVal`, 0 heap allocations!)
//! directly in CPU registers.

use std::fmt;

pub const TAG_NULL: u8 = 0;
pub const TAG_UNDEFINED: u8 = 1;
pub const TAG_BOOL: u8 = 2;
pub const TAG_INT: u8 = 3;
pub const TAG_UINT: u8 = 4;
pub const TAG_FLOAT: u8 = 5;
pub const TAG_CHAR: u8 = 6;
pub const TAG_KEYWORD: u8 = 7;
pub const TAG_SSO_STR: u8 = 8;
pub const TAG_HEAP_STR: u8 = 9;
pub const TAG_LIST: u8 = 10;
pub const TAG_MAP: u8 = 11;
pub const TAG_TUPLE: u8 = 12;
pub const TAG_OPTION_SOME: u8 = 13;
pub const TAG_RESULT_OK: u8 = 14;
pub const TAG_RESULT_ERR: u8 = 15;

/// Universal 16-byte unboxed value representation matching native hardware CPU register pairs.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct PilotVal {
    pub tag_meta: u64,
    pub payload: u64,
}

const _: () = assert!(core::mem::size_of::<PilotVal>() == 16);
const _: () = assert!(core::mem::align_of::<PilotVal>() == 8);

impl PilotVal {
    pub const NULL: Self = Self {
        tag_meta: TAG_NULL as u64,
        payload: 0,
    };

    pub const UNDEFINED: Self = Self {
        tag_meta: TAG_UNDEFINED as u64,
        payload: 0,
    };

    pub const TRUE: Self = Self {
        tag_meta: TAG_BOOL as u64,
        payload: 1,
    };

    pub const FALSE: Self = Self {
        tag_meta: TAG_BOOL as u64,
        payload: 0,
    };

    #[inline]
    pub const fn tag(self) -> u8 {
        (self.tag_meta & 0xFF) as u8
    }

    #[inline]
    pub const fn is_null(self) -> bool {
        self.tag() == TAG_NULL
    }

    #[inline]
    pub const fn is_bool(self) -> bool {
        self.tag() == TAG_BOOL
    }

    #[inline]
    pub const fn is_int(self) -> bool {
        self.tag() == TAG_INT
    }

    #[inline]
    pub const fn is_float(self) -> bool {
        self.tag() == TAG_FLOAT
    }

    #[inline]
    pub const fn is_sso_str(self) -> bool {
        self.tag() == TAG_SSO_STR
    }

    #[inline]
    pub const fn from_bool(val: bool) -> Self {
        if val {
            Self::TRUE
        } else {
            Self::FALSE
        }
    }

    #[inline]
    pub const fn from_int(val: i64) -> Self {
        Self {
            tag_meta: TAG_INT as u64,
            payload: val as u64,
        }
    }

    #[inline]
    pub const fn from_uint(val: u64) -> Self {
        Self {
            tag_meta: TAG_UINT as u64,
            payload: val,
        }
    }

    #[inline]
    pub fn from_float(val: f64) -> Self {
        Self {
            tag_meta: TAG_FLOAT as u64,
            payload: val.to_bits(),
        }
    }

    #[inline]
    pub const fn from_char(val: char) -> Self {
        Self {
            tag_meta: TAG_CHAR as u64,
            payload: val as u64,
        }
    }

    #[inline]
    pub const fn from_keyword(kw_id: u32) -> Self {
        Self {
            tag_meta: TAG_KEYWORD as u64,
            payload: kw_id as u64,
        }
    }

    /// Creates an inline Small String (SSO) up to 14 UTF-8 bytes with ZERO heap allocations.
    pub fn from_str_sso(s: &str) -> Option<Self> {
        let bytes = s.as_bytes();
        let len = bytes.len();
        if len > 14 {
            return None;
        }

        // Layout of 16-byte PilotVal:
        // tag_meta (8 bytes):
        //   byte 0: TAG_SSO_STR (8)
        //   byte 1: len (0..14)
        //   bytes 2..7: string bytes 0..5
        // payload (8 bytes):
        //   bytes 0..7: string bytes 6..13
        let mut meta_bytes = [0u8; 8];
        meta_bytes[0] = TAG_SSO_STR;
        meta_bytes[1] = len as u8;

        let meta_copy = len.min(6);
        meta_bytes[2..2 + meta_copy].copy_from_slice(&bytes[..meta_copy]);
        let tag_meta = u64::from_le_bytes(meta_bytes);

        let mut payload_bytes = [0u8; 8];
        if len > 6 {
            let payload_copy = len - 6;
            payload_bytes[..payload_copy].copy_from_slice(&bytes[6..len]);
        }
        let payload = u64::from_le_bytes(payload_bytes);

        Some(Self { tag_meta, payload })
    }

    pub fn from_str(s: &str) -> Self {
        if let Some(sso) = Self::from_str_sso(s) {
            sso
        } else {
            // Heap allocated string
            let bytes = s.as_bytes();
            let len = bytes.len() as u32;
            let boxed = s.to_string().into_boxed_str();
            let raw_ptr = Box::into_raw(boxed) as *const u8;
            let tag_meta = (TAG_HEAP_STR as u64) | ((len as u64) << 8);
            Self {
                tag_meta,
                payload: raw_ptr as usize as u64,
            }
        }
    }

    #[inline]
    pub fn as_int(self) -> Option<i64> {
        if self.is_int() {
            Some(self.payload as i64)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_float(self) -> Option<f64> {
        if self.is_float() {
            Some(f64::from_bits(self.payload))
        } else {
            None
        }
    }

    #[inline]
    pub fn as_bool(self) -> Option<bool> {
        if self.is_bool() {
            Some(self.payload != 0)
        } else {
            None
        }
    }

    /// Reads string content without any heap allocation when SSO.
    pub fn with_str<R>(&self, f: impl FnOnce(&str) -> R) -> Option<R> {
        match self.tag() {
            TAG_SSO_STR => {
                let meta_bytes = self.tag_meta.to_le_bytes();
                let len = meta_bytes[1] as usize;
                let payload_bytes = self.payload.to_le_bytes();

                let mut buf = [0u8; 14];
                let meta_copy = len.min(6);
                buf[..meta_copy].copy_from_slice(&meta_bytes[2..2 + meta_copy]);
                if len > 6 {
                    buf[6..len].copy_from_slice(&payload_bytes[..len - 6]);
                }

                let s = core::str::from_utf8(&buf[..len]).unwrap_or("");
                Some(f(s))
            }
            TAG_HEAP_STR => {
                let len = ((self.tag_meta >> 8) & 0xFFFF_FFFF) as usize;
                let ptr = self.payload as usize as *const u8;
                if ptr.is_null() {
                    None
                } else {
                    let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
                    let s = core::str::from_utf8(slice).unwrap_or("");
                    Some(f(s))
                }
            }
            _ => None,
        }
    }

    pub fn to_string_repr(&self) -> String {
        match self.tag() {
            TAG_NULL => "null".to_string(),
            TAG_UNDEFINED => "undefined".to_string(),
            TAG_BOOL => if self.payload != 0 { "true" } else { "false" }.to_string(),
            TAG_INT => (self.payload as i64).to_string(),
            TAG_UINT => self.payload.to_string(),
            TAG_FLOAT => f64::from_bits(self.payload).to_string(),
            TAG_CHAR => (self.payload as u8 as char).to_string(),
            TAG_KEYWORD => format!(":kw#{}", self.payload),
            TAG_SSO_STR | TAG_HEAP_STR => self.with_str(|s| s.to_string()).unwrap_or_default(),
            _ => format!("PilotVal(tag={})", self.tag()),
        }
    }

    /// Fast register-level arithmetic addition.
    #[inline]
    pub fn add(self, other: Self) -> Option<Self> {
        match (self.tag(), other.tag()) {
            (TAG_INT, TAG_INT) => {
                let a = self.payload as i64;
                let b = other.payload as i64;
                Some(Self::from_int(a.wrapping_add(b)))
            }
            (TAG_FLOAT, TAG_FLOAT) => {
                let a = f64::from_bits(self.payload);
                let b = f64::from_bits(other.payload);
                Some(Self::from_float(a + b))
            }
            (TAG_INT, TAG_FLOAT) => {
                let a = self.payload as i64 as f64;
                let b = f64::from_bits(other.payload);
                Some(Self::from_float(a + b))
            }
            (TAG_FLOAT, TAG_INT) => {
                let a = f64::from_bits(self.payload);
                let b = other.payload as i64 as f64;
                Some(Self::from_float(a + b))
            }
            (TAG_SSO_STR | TAG_HEAP_STR, TAG_SSO_STR | TAG_HEAP_STR) => {
                let mut res = String::new();
                self.with_str(|s1| res.push_str(s1));
                other.with_str(|s2| res.push_str(s2));
                Some(Self::from_str(&res))
            }
            _ => None,
        }
    }

    /// Fast register-level arithmetic subtraction.
    #[inline]
    pub fn sub(self, other: Self) -> Option<Self> {
        match (self.tag(), other.tag()) {
            (TAG_INT, TAG_INT) => {
                let a = self.payload as i64;
                let b = other.payload as i64;
                Some(Self::from_int(a.wrapping_sub(b)))
            }
            (TAG_FLOAT, TAG_FLOAT) => {
                let a = f64::from_bits(self.payload);
                let b = f64::from_bits(other.payload);
                Some(Self::from_float(a - b))
            }
            _ => None,
        }
    }

    /// Fast register-level arithmetic multiplication.
    #[inline]
    pub fn mul(self, other: Self) -> Option<Self> {
        match (self.tag(), other.tag()) {
            (TAG_INT, TAG_INT) => {
                let a = self.payload as i64;
                let b = other.payload as i64;
                Some(Self::from_int(a.wrapping_mul(b)))
            }
            (TAG_FLOAT, TAG_FLOAT) => {
                let a = f64::from_bits(self.payload);
                let b = f64::from_bits(other.payload);
                Some(Self::from_float(a * b))
            }
            _ => None,
        }
    }
}

impl fmt::Debug for PilotVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_repr())
    }
}

impl fmt::Display for PilotVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_repr())
    }
}
