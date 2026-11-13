//! 16-Byte Unboxed C-ABI Value Representation (`PilotVal`) with 14-Byte SSO.

use std::fmt;

static STRING_INTERNER: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<&'static str>>> = std::sync::OnceLock::new();

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
/// Handle to a VM-owned ordered sequence. The payload is the heap slot index.
pub const TAG_LIST: u8 = 10;
/// Handle to a VM-owned keyed record (maps and struct instances). The payload
/// is the heap slot index.
pub const TAG_MAP: u8 = 11;
/// Handle to a VM-owned function value (closure). The payload is the heap
/// slot of its environment: entry point, arity, then captured values.
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
    pub const NULL: Self = Self { tag_meta: TAG_NULL as u64, payload: 0 };
    pub const UNDEFINED: Self = Self { tag_meta: TAG_UNDEFINED as u64, payload: 0 };
    pub const TRUE: Self = Self { tag_meta: TAG_BOOL as u64, payload: 1 };
    pub const FALSE: Self = Self { tag_meta: TAG_BOOL as u64, payload: 0 };

    #[inline] pub const fn tag(self) -> u8 { (self.tag_meta & 0xFF) as u8 }
    #[inline] pub const fn is_null(self) -> bool { self.tag() == TAG_NULL }
    #[inline] pub const fn is_bool(self) -> bool { self.tag() == TAG_BOOL }
    /// An integer of either signedness; `as_int` reads its 64 bits.
    #[inline] pub const fn is_int(self) -> bool { let t = self.tag(); t == TAG_INT || t == TAG_UINT }
    /// An integer held in an unsigned 64-bit register: arithmetic, shifts,
    /// comparisons and printing treat its bits as unsigned.
    #[inline] pub const fn is_uint(self) -> bool { self.tag() == TAG_UINT }
    #[inline] pub const fn is_float(self) -> bool { self.tag() == TAG_FLOAT }
    #[inline] pub const fn is_sso_str(self) -> bool { self.tag() == TAG_SSO_STR }
    #[inline] pub const fn is_str(self) -> bool { let t = self.tag(); t == TAG_SSO_STR || t == TAG_HEAP_STR }
    #[inline] pub const fn is_list(self) -> bool { self.tag() == TAG_LIST }
    #[inline] pub const fn is_map(self) -> bool { self.tag() == TAG_MAP }
    #[inline] pub const fn is_fn(self) -> bool { self.tag() == TAG_FN }

    #[inline]
    pub const fn from_fn_handle(slot: usize) -> Self {
        Self { tag_meta: TAG_FN as u64, payload: slot as u64 }
    }

    #[inline]
    pub const fn as_fn_handle(self) -> Option<usize> {
        if self.is_fn() { Some(self.payload as usize) } else { None }
    }

    #[inline]
    pub const fn from_list_handle(slot: usize) -> Self {
        Self { tag_meta: TAG_LIST as u64, payload: slot as u64 }
    }

    #[inline]
    pub const fn from_map_handle(slot: usize) -> Self {
        Self { tag_meta: TAG_MAP as u64, payload: slot as u64 }
    }

    #[inline]
    pub const fn as_list_handle(self) -> Option<usize> {
        if self.is_list() { Some(self.payload as usize) } else { None }
    }

    #[inline]
    pub const fn as_map_handle(self) -> Option<usize> {
        if self.is_map() { Some(self.payload as usize) } else { None }
    }

    /// Human-readable value kind used in runtime diagnostics.
    pub const fn kind_label(self) -> &'static str {
        match self.tag() {
            TAG_NULL => "null",
            TAG_UNDEFINED => "undefined",
            TAG_BOOL => "boolean",
            TAG_INT | TAG_UINT => "integer",
            TAG_FLOAT => "float",
            TAG_CHAR => "character",
            TAG_KEYWORD => "keyword",
            TAG_SSO_STR | TAG_HEAP_STR => "string",
            TAG_LIST => "list",
            TAG_MAP => "record",
            TAG_FN => "function",
            _ => "unknown value",
        }
    }

    #[inline] pub const fn from_bool(val: bool) -> Self { if val { Self::TRUE } else { Self::FALSE } }
    #[inline] pub const fn from_int(val: i64) -> Self { Self { tag_meta: TAG_INT as u64, payload: val as u64 } }
    #[inline] pub const fn from_uint(val: u64) -> Self { Self { tag_meta: TAG_UINT as u64, payload: val } }
    #[inline] pub fn from_float(val: f64) -> Self { Self { tag_meta: TAG_FLOAT as u64, payload: val.to_bits() } }
    #[inline] pub const fn from_char(val: char) -> Self { Self { tag_meta: TAG_CHAR as u64, payload: val as u64 } }
    #[inline] pub const fn from_keyword(kw_id: u32) -> Self { Self { tag_meta: TAG_KEYWORD as u64, payload: kw_id as u64 } }

    pub fn from_str_sso(s: &str) -> Option<Self> {
        let bytes = s.as_bytes();
        let len = bytes.len();
        if len > 14 { return None; }
        let mut meta_bytes = [0u8; 8];
        meta_bytes[0] = TAG_SSO_STR;
        meta_bytes[1] = len as u8;
        let meta_copy = len.min(6);
        meta_bytes[2..2 + meta_copy].copy_from_slice(&bytes[..meta_copy]);
        let mut payload_bytes = [0u8; 8];
        if len > 6 { payload_bytes[..len - 6].copy_from_slice(&bytes[6..len]); }
        Some(Self { tag_meta: u64::from_le_bytes(meta_bytes), payload: u64::from_le_bytes(payload_bytes) })
    }

    pub fn from_str(s: &str) -> Self {
        if let Some(sso) = Self::from_str_sso(s) { sso } else {
            let len = s.len() as u32;
            let pool = STRING_INTERNER.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()));
            let mut guard = pool.lock().unwrap();
            let ptr = if let Some(&existing) = guard.get(s) {
                existing.as_ptr()
            } else {
                let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
                guard.insert(leaked);
                leaked.as_ptr()
            };
            Self { tag_meta: (TAG_HEAP_STR as u64) | ((len as u64) << 8), payload: ptr as usize as u64 }
        }
    }

    #[inline] pub fn as_int(self) -> Option<i64> { if self.is_int() { Some(self.payload as i64) } else { None } }
    #[inline] pub fn as_float(self) -> Option<f64> { if self.is_float() { Some(f64::from_bits(self.payload)) } else { None } }
    /// The Unicode scalar value of a character value.
    #[inline] pub fn as_char(self) -> Option<char> { if self.tag() == TAG_CHAR { char::from_u32(self.payload as u32) } else { None } }
    #[inline] pub fn as_bool(self) -> Option<bool> { if self.is_bool() { Some(self.payload != 0) } else { None } }

    /// The string this value holds, without allocating: a short string is
    /// decoded into `buf`, a long one is borrowed where it is stored.
    pub fn str_in<'a>(&self, buf: &'a mut [u8; 14]) -> Option<&'a str> {
        match self.tag() {
            TAG_SSO_STR => {
                let (meta_bytes, payload_bytes) = (self.tag_meta.to_le_bytes(), self.payload.to_le_bytes());
                let len = meta_bytes[1] as usize;
                let meta_copy = len.min(6);
                buf[..meta_copy].copy_from_slice(&meta_bytes[2..2 + meta_copy]);
                if len > 6 {
                    buf[6..len].copy_from_slice(&payload_bytes[..len - 6]);
                }
                Some(core::str::from_utf8(&buf[..len]).unwrap_or(""))
            }
            TAG_HEAP_STR => {
                let (ptr, len) = self.heap_parts()?;
                // The bytes outlive this borrow: they are interned for good,
                // or owned by the VM, which frees them only between
                // instructions.
                let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
                Some(core::str::from_utf8(bytes).unwrap_or(""))
            }
            _ => None,
        }
    }

    /// A long string whose bytes someone else owns: the VM's run-time
    /// strings (`pilot_vm`), which it frees once no value refers to them.
    ///
    /// # Safety
    /// `ptr` must point to `len` bytes of UTF-8 that stay valid for as long
    /// as the value (or any copy of it) is read.
    pub unsafe fn from_heap_parts(ptr: *const u8, len: usize) -> Self {
        Self { tag_meta: (TAG_HEAP_STR as u64) | ((len as u64) << 8), payload: ptr as usize as u64 }
    }

    /// The address and length of a long string's bytes.
    pub fn heap_parts(&self) -> Option<(*const u8, usize)> {
        if self.tag() != TAG_HEAP_STR || self.payload == 0 {
            return None;
        }
        Some((self.payload as usize as *const u8, ((self.tag_meta >> 8) & 0xFFFF_FFFF) as usize))
    }

    pub fn with_str<R>(&self, f: impl FnOnce(&str) -> R) -> Option<R> {
        match self.tag() {
            TAG_SSO_STR => {
                let (meta_bytes, payload_bytes) = (self.tag_meta.to_le_bytes(), self.payload.to_le_bytes());
                let len = meta_bytes[1] as usize;
                let mut buf = [0u8; 14];
                let meta_copy = len.min(6);
                buf[..meta_copy].copy_from_slice(&meta_bytes[2..2 + meta_copy]);
                if len > 6 { buf[6..len].copy_from_slice(&payload_bytes[..len - 6]); }
                Some(f(core::str::from_utf8(&buf[..len]).unwrap_or("")))
            }
            TAG_HEAP_STR => {
                let len = ((self.tag_meta >> 8) & 0xFFFF_FFFF) as usize;
                let ptr = self.payload as usize as *const u8;
                if ptr.is_null() { None } else {
                    Some(f(core::str::from_utf8(unsafe { core::slice::from_raw_parts(ptr, len) }).unwrap_or("")))
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
            TAG_CHAR => char::from_u32(self.payload as u32).unwrap_or(char::REPLACEMENT_CHARACTER).to_string(),
            TAG_KEYWORD => format!(":kw#{}", self.payload),
            TAG_SSO_STR | TAG_HEAP_STR => self.with_str(|s| s.to_string()).unwrap_or_default(),
            TAG_LIST => format!("<list #{}>", self.payload),
            TAG_MAP => format!("<record #{}>", self.payload),
            TAG_FN => format!("<function #{}>", self.payload),
            _ => format!("PilotVal(tag={})", self.tag()),
        }
    }

    #[inline]
    pub fn add(self, other: Self) -> Option<Self> {
        match (self.tag(), other.tag()) {
            (TAG_INT, TAG_INT) => Some(Self::from_int((self.payload as i64).wrapping_add(other.payload as i64))),
            (TAG_FLOAT, TAG_FLOAT) => Some(Self::from_float(f64::from_bits(self.payload) + f64::from_bits(other.payload))),
            (TAG_INT, TAG_FLOAT) => Some(Self::from_float((self.payload as i64 as f64) + f64::from_bits(other.payload))),
            (TAG_FLOAT, TAG_INT) => Some(Self::from_float(f64::from_bits(self.payload) + (other.payload as i64 as f64))),
            _ => None,
        }
    }

    #[inline]
    pub fn sub(self, other: Self) -> Option<Self> {
        match (self.tag(), other.tag()) {
            (TAG_INT, TAG_INT) => Some(Self::from_int((self.payload as i64).wrapping_sub(other.payload as i64))),
            (TAG_FLOAT, TAG_FLOAT) => Some(Self::from_float(f64::from_bits(self.payload) - f64::from_bits(other.payload))),
            _ => None,
        }
    }

    #[inline]
    pub fn mul(self, other: Self) -> Option<Self> {
        match (self.tag(), other.tag()) {
            (TAG_INT, TAG_INT) => Some(Self::from_int((self.payload as i64).wrapping_mul(other.payload as i64))),
            (TAG_FLOAT, TAG_FLOAT) => Some(Self::from_float(f64::from_bits(self.payload) * f64::from_bits(other.payload))),
            _ => None,
        }
    }
}

impl fmt::Debug for PilotVal { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.to_string_repr()) } }
impl fmt::Display for PilotVal { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.to_string_repr()) } }
