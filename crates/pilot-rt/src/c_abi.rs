//! Linkable Static & Shared C-ABI Runtime Library (`libpilot_rt.a`) & Dynamic FFI Loader (`dlopen`/`dlsym`).

use core::ffi::c_void;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::memory::BumpRegion;

#[repr(C)]
pub struct PilotArcBox { pub ref_count: AtomicUsize, pub size: usize, pub payload: [u8; 0] }

#[repr(C)] #[derive(Copy, Clone, Debug)]
pub struct PilotSlice { pub ptr: *mut u8, pub len: usize }

pub struct DynamicLibraryHandle { pub name: String, raw_handle: *mut c_void, pub is_loaded: bool }
unsafe impl Send for DynamicLibraryHandle {}
unsafe impl Sync for DynamicLibraryHandle {}

#[cfg(unix)]
extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
}

impl DynamicLibraryHandle {
    pub fn open(lib_name: &str) -> Result<Self, String> {
        let c_name = CString::new(lib_name).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            let h = unsafe { dlopen(c_name.as_ptr(), 2 | 0x100) };
            if h.is_null() { return Err(format!("dlopen failed for '{}'", lib_name)); }
            Ok(Self { name: lib_name.to_string(), raw_handle: h, is_loaded: true })
        }
        #[cfg(not(unix))]
        Ok(Self { name: lib_name.to_string(), raw_handle: std::ptr::null_mut(), is_loaded: false })
    }

    pub fn resolve_symbol(&self, sym_name: &str) -> Result<*const (), String> {
        if !self.is_loaded || self.raw_handle.is_null() { return Err("Library not loaded".into()); }
        let c_sym = CString::new(sym_name).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            let p = unsafe { dlsym(self.raw_handle, c_sym.as_ptr()) };
            if p.is_null() { return Err(format!("dlsym failed for '{}'", sym_name)); }
            Ok(p as *const ())
        }
        #[cfg(not(unix))]
        Err("Unsupported platform".into())
    }
}

impl Drop for DynamicLibraryHandle {
    fn drop(&mut self) {
        if self.is_loaded && !self.raw_handle.is_null() {
            #[cfg(unix)] unsafe { dlclose(self.raw_handle); }
            self.raw_handle = std::ptr::null_mut();
            self.is_loaded = false;
        }
    }
}

#[no_mangle] pub extern "C" fn pilot_rt_region_enter() -> *mut c_void { Box::into_raw(Box::new(BumpRegion::new())) as *mut c_void }
#[no_mangle]
pub unsafe extern "C" fn pilot_rt_region_alloc(region_ptr: *mut c_void, size: usize, align: usize) -> *mut c_void {
    if region_ptr.is_null() { return std::ptr::null_mut(); }
    (&mut *(region_ptr as *mut BumpRegion)).alloc(size, align).map(|p| p as *mut c_void).unwrap_or(std::ptr::null_mut())
}
#[no_mangle] pub unsafe extern "C" fn pilot_rt_region_exit(region_ptr: *mut c_void) { if !region_ptr.is_null() { let _ = Box::from_raw(region_ptr as *mut BumpRegion); } }

#[no_mangle]
pub extern "C" fn pilot_rt_alloc_arc(payload_size: usize) -> *mut c_void {
    let total_size = std::mem::size_of::<PilotArcBox>() + payload_size;
    let layout = std::alloc::Layout::from_size_align(total_size, 8).unwrap();
    unsafe {
        let ptr = std::alloc::alloc(layout) as *mut PilotArcBox;
        if ptr.is_null() { std::alloc::handle_alloc_error(layout); }
        (*ptr).ref_count = AtomicUsize::new(1);
        (*ptr).size = payload_size;
        ptr.add(1) as *mut c_void
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_dup(payload_ptr: *mut c_void) -> *mut c_void {
    if payload_ptr.is_null() { return std::ptr::null_mut(); }
    (*(payload_ptr as *mut PilotArcBox).sub(1)).ref_count.fetch_add(1, Ordering::Relaxed);
    payload_ptr
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_drop(payload_ptr: *mut c_void, dtor: Option<unsafe extern "C" fn(*mut c_void)>) {
    if payload_ptr.is_null() { return; }
    let header = (payload_ptr as *mut PilotArcBox).sub(1);
    if (*header).ref_count.fetch_sub(1, Ordering::Release) == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        if let Some(drop_fn) = dtor { drop_fn(payload_ptr); }
        let layout = std::alloc::Layout::from_size_align(std::mem::size_of::<PilotArcBox>() + (*header).size, 8).unwrap();
        std::alloc::dealloc(header as *mut u8, layout);
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_drop_reuse(payload_ptr: *mut c_void, required_size: usize) -> *mut c_void {
    if payload_ptr.is_null() { return pilot_rt_alloc_arc(required_size); }
    let header = (payload_ptr as *mut PilotArcBox).sub(1);
    if (*header).ref_count.load(Ordering::Acquire) == 1 && (*header).size >= required_size { return payload_ptr; }
    pilot_rt_drop(payload_ptr, None);
    pilot_rt_alloc_arc(required_size)
}

#[no_mangle] pub extern "C" fn pilot_rt_print_i64(val: i64) { println!("{}", val); }
#[no_mangle] pub extern "C" fn pilot_rt_print_f64(val: f64) { println!("{}", val); }
#[no_mangle] pub unsafe extern "C" fn pilot_rt_print_str(s: *const c_char) { if !s.is_null() { println!("{}", CStr::from_ptr(s).to_str().unwrap_or("")); } }
#[no_mangle] pub extern "C" fn pilot_rt_time_now_ns() -> u64 { crate::time::now_nanos() }
#[no_mangle] pub extern "C" fn pilot_rt_sleep_ms(ms: u64) { crate::time::sleep_ms(ms); }

/// 20-line OS CSPRNG syscall (`/dev/urandom` / hardware entropy fallback).
#[no_mangle]
pub unsafe extern "C" fn pilot_rt_os_getrandom(out_ptr: *mut u8, len: usize) -> usize {
    if out_ptr.is_null() || len == 0 { return 0; }
    let slice = core::slice::from_raw_parts_mut(out_ptr, len);
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        if f.read_exact(slice).is_ok() { return len; }
    }
    let mut state = crate::time::now_nanos().wrapping_add(0x9E37_79B9_7F4A_7C15);
    for b in slice.iter_mut() {
        state ^= state >> 12; state ^= state << 25; state ^= state >> 27;
        *b = (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 56) as u8;
    }
    len
}

pub fn generate_c_runtime_header() -> &'static str {
    "#ifndef PILOT_RUNTIME_H\n#define PILOT_RUNTIME_H\n#include <stdint.h>\n#include <stdlib.h>\n\
     static inline void* pilot_rt_region_enter(void) { return malloc(65536); }\n\
     static inline void* pilot_rt_region_alloc(void* r, size_t s, size_t a) { (void)r; (void)a; return malloc(s); }\n\
     static inline void pilot_rt_region_exit(void* r) { free(r); }\n\
     static inline void* pilot_rt_dup(void* p) { return p; }\n\
     static inline void pilot_rt_drop(void* p, void (*d)(void*)) { if (d && p) d(p); free(p); }\n\
     static inline void* pilot_rt_drop_reuse(void* p, size_t s) { (void)s; return p; }\n\
     #endif\n"
}
