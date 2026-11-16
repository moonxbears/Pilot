//! 16-Byte Unboxed C-ABI Value Representation (`PilotVal`) with 14-Byte SSO.

use std::collections::HashSet;
use std::fmt;
use std::sync::{Mutex, OnceLock};

static STRING_INTERNER: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();

pub const TAG_NULL: u8 = 0;
pub const TAG_BOOL: u8 = 2;
pub const TAG_INT: u8 = 3;
pub const TAG_UINT: u8 = 4;
pub const TAG_FLOAT: u8 = 5;
pub const TAG_CHAR: u8 = 6;
pub const TAG_SSO_STR: u8 = 8;
pub const TAG_HEAP_STR: u8 = 9;
pub const TAG_LIST: u8 = 10;
pub const TAG_MAP: u8 = 11;
pub const TAG_FN: u8 = 12;

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
    pub const TRUE: Self = Self {
        tag_meta: TAG_BOOL as u64,
        payload: 1,
    };
    pub const FALSE: Self = Self {
        tag_meta: TAG_BOOL as u64,
        payload: 0,
    };

    pub const fn tag(self) -> u8 {
        (self.tag_meta & 0xFF) as u8
    }
    pub const fn is_null(self) -> bool {
        self.tag() == TAG_NULL
    }
    pub const fn is_bool(self) -> bool {
        self.tag() == TAG_BOOL
    }
    pub const fn is_int(self) -> bool {
        matches!(self.tag(), TAG_INT | TAG_UINT)
    }
    pub const fn is_uint(self) -> bool {
        self.tag() == TAG_UINT
    }
    pub const fn is_float(self) -> bool {
        self.tag() == TAG_FLOAT
    }
    pub const fn is_str(self) -> bool {
        matches!(self.tag(), TAG_SSO_STR | TAG_HEAP_STR)
    }
    pub const fn is_list(self) -> bool {
        self.tag() == TAG_LIST
    }
    pub const fn is_map(self) -> bool {
        self.tag() == TAG_MAP
    }
    pub const fn is_fn(self) -> bool {
        self.tag() == TAG_FN
    }

    pub const fn from_fn_handle(slot: usize) -> Self {
        Self {
            tag_meta: TAG_FN as u64,
            payload: slot as u64,
        }
    }
    pub const fn as_fn_handle(self) -> Option<usize> {
        if self.is_fn() {
            Some(self.payload as usize)
        } else {
            None
        }
    }
    pub const fn from_list_handle(slot: usize) -> Self {
        Self {
            tag_meta: TAG_LIST as u64,
            payload: slot as u64,
        }
    }
    pub const fn from_map_handle(slot: usize) -> Self {
        Self {
            tag_meta: TAG_MAP as u64,
            payload: slot as u64,
        }
    }
    pub const fn as_list_handle(self) -> Option<usize> {
        if self.is_list() {
            Some(self.payload as usize)
        } else {
            None
        }
    }
    pub const fn as_map_handle(self) -> Option<usize> {
        if self.is_map() {
            Some(self.payload as usize)
        } else {
            None
        }
    }

    pub const fn from_bool(val: bool) -> Self {
        if val {
            Self::TRUE
        } else {
            Self::FALSE
        }
    }
    pub const fn from_int(val: i64) -> Self {
        Self {
            tag_meta: TAG_INT as u64,
            payload: val as u64,
        }
    }
    pub const fn from_uint(val: u64) -> Self {
        Self {
            tag_meta: TAG_UINT as u64,
            payload: val,
        }
    }
    pub fn from_float(val: f64) -> Self {
        Self {
            tag_meta: TAG_FLOAT as u64,
            payload: val.to_bits(),
        }
    }
    pub const fn from_char(val: char) -> Self {
        Self {
            tag_meta: TAG_CHAR as u64,
            payload: val as u64,
        }
    }

    pub fn as_int(self) -> Option<i64> {
        self.is_int().then_some(self.payload as i64)
    }
    pub fn as_float(self) -> Option<f64> {
        self.is_float().then(|| f64::from_bits(self.payload))
    }
    pub fn as_char(self) -> Option<char> {
        (self.tag() == TAG_CHAR)
            .then(|| char::from_u32(self.payload as u32))
            .flatten()
    }
    pub fn as_bool(self) -> Option<bool> {
        self.is_bool().then_some(self.payload != 0)
    }

    pub const fn kind_label(self) -> &'static str {
        match self.tag() {
            TAG_NULL => "null",
            TAG_BOOL => "boolean",
            TAG_INT | TAG_UINT => "integer",
            TAG_FLOAT => "float",
            TAG_CHAR => "character",
            TAG_SSO_STR | TAG_HEAP_STR => "string",
            TAG_LIST => "list",
            TAG_MAP => "record",
            TAG_FN => "function",
            _ => "unknown value",
        }
    }

    pub fn from_str_sso(s: &str) -> Option<Self> {
        let bytes = s.as_bytes();
        if bytes.len() > 14 {
            return None;
        }
        let mut buf = [0u8; 16];
        buf[0] = TAG_SSO_STR;
        buf[1] = bytes.len() as u8;
        buf[2..2 + bytes.len()].copy_from_slice(bytes);
        Some(Self {
            tag_meta: u64::from_le_bytes(buf[..8].try_into().unwrap()),
            payload: u64::from_le_bytes(buf[8..].try_into().unwrap()),
        })
    }

    pub fn from_str(s: &str) -> Self {
        if let Some(sso) = Self::from_str_sso(s) {
            return sso;
        }
        let mut guard = STRING_INTERNER
            .get_or_init(|| Mutex::new(HashSet::new()))
            .lock()
            .unwrap();
        let ptr = match guard.get(s) {
            Some(&existing) => existing.as_ptr(),
            None => {
                let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
                guard.insert(leaked);
                leaked.as_ptr()
            }
        };
        unsafe { Self::from_heap_parts(ptr, s.len()) }
    }

    pub fn is_well_formed(self) -> bool {
        if self.tag() != TAG_SSO_STR {
            return true;
        }
        let len = ((self.tag_meta >> 8) & 0xFF) as usize;
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&self.tag_meta.to_le_bytes());
        bytes[8..].copy_from_slice(&self.payload.to_le_bytes());
        len <= 14 && core::str::from_utf8(&bytes[2..2 + len]).is_ok()
    }

    pub fn str_in<'a>(&self, buf: &'a mut [u8; 14]) -> Option<&'a str> {
        match self.tag() {
            TAG_SSO_STR => {
                let meta = self.tag_meta.to_le_bytes();
                let len = (meta[1] as usize).min(14);
                buf[..6].copy_from_slice(&meta[2..]);
                buf[6..].copy_from_slice(&self.payload.to_le_bytes());
                Some(unsafe { core::str::from_utf8_unchecked(&buf[..len]) })
            }
            TAG_HEAP_STR => {
                let (ptr, len) = self.heap_parts()?;
                Some(unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr, len))
                })
            }
            _ => None,
        }
    }

    pub unsafe fn from_heap_parts(ptr: *const u8, len: usize) -> Self {
        Self {
            tag_meta: (TAG_HEAP_STR as u64) | ((len as u64) << 8),
            payload: ptr as usize as u64,
        }
    }

    pub fn heap_parts(&self) -> Option<(*const u8, usize)> {
        (self.tag() == TAG_HEAP_STR && self.payload != 0).then_some((
            self.payload as usize as *const u8,
            ((self.tag_meta >> 8) & 0xFFFF_FFFF) as usize,
        ))
    }

    pub fn with_str<R>(&self, f: impl FnOnce(&str) -> R) -> Option<R> {
        let mut buf = [0u8; 14];
        self.str_in(&mut buf).map(f)
    }

    pub fn to_string_repr(&self) -> String {
        match self.tag() {
            TAG_NULL => "null".to_string(),
            TAG_BOOL => if self.payload != 0 { "true" } else { "false" }.to_string(),
            TAG_INT => (self.payload as i64).to_string(),
            TAG_UINT => self.payload.to_string(),
            TAG_FLOAT => f64::from_bits(self.payload).to_string(),
            TAG_CHAR => char::from_u32(self.payload as u32)
                .unwrap_or(char::REPLACEMENT_CHARACTER)
                .to_string(),
            TAG_SSO_STR | TAG_HEAP_STR => self.with_str(str::to_string).unwrap_or_default(),
            TAG_LIST => format!("<list #{}>", self.payload),
            TAG_MAP => format!("<record #{}>", self.payload),
            TAG_FN => format!("<function #{}>", self.payload),
            _ => format!("PilotVal(tag={})", self.tag()),
        }
    }
}

impl fmt::Debug for PilotVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_repr())
    }
}

impl fmt::Display for PilotVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_repr())
    }
}
