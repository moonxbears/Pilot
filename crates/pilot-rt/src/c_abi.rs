//! Linkable Static & Shared C-ABI Runtime Library (`libpilot_rt.a`).
//! Exports standard C-compatible ABI symbols for Tier 1 bump regions,
//! Tier 2 Perceus ARC (`pilot_rt_dup`, `pilot_rt_drop`, `pilot_rt_drop_reuse`), and I/O.

use core::ffi::c_void;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::memory::BumpRegion;

#[repr(C)]
pub struct PilotArcBox {
    pub ref_count: AtomicUsize,
    pub size: usize,
    pub payload: [u8; 0],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PilotSlice { pub ptr: *mut u8, pub len: usize }

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PilotSliceView { pub base_ptr: *mut u8, pub len: usize, pub byte_stride: usize }

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PilotStridedSlice { pub base_ptr: *mut u8, pub len: usize, pub byte_stride: usize, pub byte_offset: usize }

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PilotSliceMap {
    pub keys_ptr: *mut u8, pub vals_ptr: *mut u8, pub len: usize,
    pub key_stride: usize, pub val_stride: usize, pub is_soa: bool,
}

#[no_mangle]
pub extern "C" fn pilot_rt_region_enter() -> *mut c_void {
    Box::into_raw(Box::new(BumpRegion::new())) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_region_alloc(region_ptr: *mut c_void, size: usize, align: usize) -> *mut c_void {
    if region_ptr.is_null() { return std::ptr::null_mut(); }
    (&mut *(region_ptr as *mut BumpRegion)).alloc(size, align).map(|p| p as *mut c_void).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_region_exit(region_ptr: *mut c_void) {
    if !region_ptr.is_null() { let _ = Box::from_raw(region_ptr as *mut BumpRegion); }
}

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
    let header = (payload_ptr as *mut PilotArcBox).sub(1);
    (*header).ref_count.fetch_add(1, Ordering::Relaxed);
    payload_ptr
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_drop(payload_ptr: *mut c_void, dtor: Option<unsafe extern "C" fn(*mut c_void)>) {
    if payload_ptr.is_null() { return; }
    let header = (payload_ptr as *mut PilotArcBox).sub(1);
    if (*header).ref_count.fetch_sub(1, Ordering::Release) == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        if let Some(drop_fn) = dtor { drop_fn(payload_ptr); }
        let total_size = std::mem::size_of::<PilotArcBox>() + (*header).size;
        let layout = std::alloc::Layout::from_size_align(total_size, 8).unwrap();
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

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_str_concat(s1: *const c_char, s2: *const c_char) -> *mut c_char {
    let a = if s1.is_null() { "" } else { CStr::from_ptr(s1).to_str().unwrap_or("") };
    let b = if s2.is_null() { "" } else { CStr::from_ptr(s2).to_str().unwrap_or("") };
    CString::new(format!("{a}{b}")).unwrap().into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_str_free(ptr: *mut c_char) {
    if !ptr.is_null() { let _ = CString::from_raw(ptr); }
}

#[no_mangle] pub extern "C" fn pilot_rt_print_i64(val: i64) { println!("{}", val); }
#[no_mangle] pub extern "C" fn pilot_rt_print_f64(val: f64) { println!("{}", val); }

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_print_str(s: *const c_char) {
    if !s.is_null() { println!("{}", CStr::from_ptr(s).to_str().unwrap_or("")); }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_panic_contract(msg: *const c_char, file: *const c_char, line: u32) -> ! {
    let m = if msg.is_null() { "Contract assertion failed" } else { CStr::from_ptr(msg).to_str().unwrap_or("Contract assertion failed") };
    let f = if file.is_null() { "<unknown>" } else { CStr::from_ptr(file).to_str().unwrap_or("<unknown>") };
    eprintln!("Pilot runtime panic: {} at {}:{}", m, f, line);
    std::process::exit(101);
}

#[no_mangle] pub extern "C" fn pilot_rt_list_new() -> *mut c_void { Box::into_raw(Box::new(Vec::<i64>::new())) as *mut c_void }
#[no_mangle] pub unsafe extern "C" fn pilot_rt_list_push(p: *mut c_void, v: i64) { if !p.is_null() { (&mut *(p as *mut Vec<i64>)).push(v); } }
#[no_mangle] pub unsafe extern "C" fn pilot_rt_list_get(p: *mut c_void, i: usize) -> i64 { if !p.is_null() { (&*(p as *mut Vec<i64>)).get(i).copied().unwrap_or(0) } else { 0 } }
#[no_mangle] pub unsafe extern "C" fn pilot_rt_list_len(p: *mut c_void) -> usize { if !p.is_null() { (&*(p as *mut Vec<i64>)).len() } else { 0 } }
#[no_mangle] pub unsafe extern "C" fn pilot_rt_list_free(p: *mut c_void) { if !p.is_null() { let _ = Box::from_raw(p as *mut Vec<i64>); } }

#[no_mangle] pub extern "C" fn pilot_rt_map_new() -> *mut c_void { Box::into_raw(Box::new(std::collections::HashMap::<String, i64>::new())) as *mut c_void }
#[no_mangle]
pub unsafe extern "C" fn pilot_rt_map_insert(p: *mut c_void, k: *const c_char, v: i64) {
    if !p.is_null() && !k.is_null() { (&mut *(p as *mut std::collections::HashMap<String, i64>)).insert(CStr::from_ptr(k).to_str().unwrap_or("").to_string(), v); }
}
#[no_mangle]
pub unsafe extern "C" fn pilot_rt_map_get(p: *mut c_void, k: *const c_char) -> i64 {
    if !p.is_null() && !k.is_null() { (&*(p as *mut std::collections::HashMap<String, i64>)).get(CStr::from_ptr(k).to_str().unwrap_or("")).copied().unwrap_or(0) } else { 0 }
}
#[no_mangle] pub unsafe extern "C" fn pilot_rt_map_free(p: *mut c_void) { if !p.is_null() { let _ = Box::from_raw(p as *mut std::collections::HashMap<String, i64>); } }
#[no_mangle] pub extern "C" fn pilot_rt_table_new() -> *mut c_void { pilot_rt_map_new() }
#[no_mangle] pub unsafe extern "C" fn pilot_rt_table_free(p: *mut c_void) { pilot_rt_map_free(p) }

#[no_mangle] pub extern "C" fn pilot_rt_chan_new(cap: usize) -> *mut c_void { Box::into_raw(Box::new(crate::concurrency::Channel::new(cap))) as *mut c_void }
#[no_mangle]
pub unsafe extern "C" fn pilot_rt_chan_send(p: *mut c_void, v: i64) -> bool {
    if !p.is_null() { (&*(p as *mut crate::concurrency::Channel)).send(crate::value::PilotVal::from_int(v)) } else { false }
}
#[no_mangle]
pub unsafe extern "C" fn pilot_rt_chan_recv(p: *mut c_void, out: *mut i64) -> bool {
    if !p.is_null() && !out.is_null() {
        if let Some(v) = (&*(p as *mut crate::concurrency::Channel)).recv() { *out = v.as_int().unwrap_or(0); true } else { false }
    } else { false }
}
#[no_mangle] pub unsafe extern "C" fn pilot_rt_chan_free(p: *mut c_void) { if !p.is_null() { let _ = Box::from_raw(p as *mut crate::concurrency::Channel); } }
#[no_mangle] pub extern "C" fn pilot_rt_time_now_ns() -> u64 { crate::time::now_nanos() }
#[no_mangle] pub extern "C" fn pilot_rt_sleep_ms(ms: u64) { crate::time::sleep_ms(ms); }

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_crypto_sha256(data: *const u8, len: usize, out_hash: *mut u8) {
    if data.is_null() || out_hash.is_null() { return; }
    let hash = crate::crypto::sha256(core::slice::from_raw_parts(data, len));
    core::ptr::copy_nonoverlapping(hash.as_ptr(), out_hash, 32);
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_crypto_random_bytes(out_ptr: *mut u8, len: usize) {
    if out_ptr.is_null() || len == 0 { return; }
    let bytes = crate::crypto::random_bytes(len);
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_ptr, len);
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
