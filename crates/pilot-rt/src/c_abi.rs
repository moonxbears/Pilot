//! Milestone 40.1: Linkable Static & Shared C-ABI Runtime Library (`pilot_runtime.h` + `libpilot_rt.a`).
//!
//! Exports standard C-compatible ABI symbols for compiled Pilot programs,
//! supporting Tier 1 bump regions, Tier 2 Perceus ARC, and runtime error handlers.

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

// --- Tier 1 Bump Region Memory ABI ---

#[no_mangle]
pub extern "C" fn pilot_rt_region_enter() -> *mut c_void {
    let region = Box::new(BumpRegion::new());
    Box::into_raw(region) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_region_alloc(
    region_ptr: *mut c_void,
    size: usize,
    align: usize,
) -> *mut c_void {
    if region_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let region = &mut *(region_ptr as *mut BumpRegion);
    region.alloc(size, align).map(|p| p as *mut c_void).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_region_exit(region_ptr: *mut c_void) {
    if !region_ptr.is_null() {
        let _ = Box::from_raw(region_ptr as *mut BumpRegion);
    }
}

// --- Tier 2 Perceus Precise ARC & Drop-Guided Reuse ABI ---

#[no_mangle]
pub extern "C" fn pilot_rt_alloc_arc(payload_size: usize) -> *mut c_void {
    let total_size = std::mem::size_of::<PilotArcBox>() + payload_size;
    let layout = std::alloc::Layout::from_size_align(total_size, 8).unwrap();
    unsafe {
        let ptr = std::alloc::alloc(layout) as *mut PilotArcBox;
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        (*ptr).ref_count = AtomicUsize::new(1);
        (*ptr).size = payload_size;
        ptr.add(1) as *mut c_void
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_dup(payload_ptr: *mut c_void) -> *mut c_void {
    if payload_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let header = (payload_ptr as *mut PilotArcBox).sub(1);
    (*header).ref_count.fetch_add(1, Ordering::Relaxed);
    payload_ptr
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_drop(
    payload_ptr: *mut c_void,
    dtor: Option<unsafe extern "C" fn(*mut c_void)>,
) {
    if payload_ptr.is_null() {
        return;
    }
    let header = (payload_ptr as *mut PilotArcBox).sub(1);
    if (*header).ref_count.fetch_sub(1, Ordering::Release) == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        if let Some(drop_fn) = dtor {
            drop_fn(payload_ptr);
        }
        let total_size = std::mem::size_of::<PilotArcBox>() + (*header).size;
        let layout = std::alloc::Layout::from_size_align(total_size, 8).unwrap();
        std::alloc::dealloc(header as *mut u8, layout);
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_drop_reuse(
    payload_ptr: *mut c_void,
    required_size: usize,
) -> *mut c_void {
    if payload_ptr.is_null() {
        return pilot_rt_alloc_arc(required_size);
    }
    let header = (payload_ptr as *mut PilotArcBox).sub(1);
    // If uniquely owned (ref_count == 1) and size matches, recycle in-place with zero malloc
    if (*header).ref_count.load(Ordering::Acquire) == 1 && (*header).size >= required_size {
        return payload_ptr;
    }

    // Otherwise drop old and allocate new
    pilot_rt_drop(payload_ptr, None);
    pilot_rt_alloc_arc(required_size)
}

// --- Strings & Printing ABI ---

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_str_concat(
    s1: *const c_char,
    s2: *const c_char,
) -> *mut c_char {
    let str1 = if s1.is_null() { "" } else { CStr::from_ptr(s1).to_str().unwrap_or("") };
    let str2 = if s2.is_null() { "" } else { CStr::from_ptr(s2).to_str().unwrap_or("") };
    let combined = format!("{}{}", str1, str2);
    CString::new(combined).unwrap().into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_str_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

#[no_mangle]
pub extern "C" fn pilot_rt_print_i64(val: i64) {
    println!("{}", val);
}

#[no_mangle]
pub extern "C" fn pilot_rt_print_f64(val: f64) {
    println!("{}", val);
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_print_str(s: *const c_char) {
    if !s.is_null() {
        let cstr = CStr::from_ptr(s);
        println!("{}", cstr.to_str().unwrap_or(""));
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_panic_contract(
    msg: *const c_char,
    file: *const c_char,
    line: u32,
) -> ! {
    let message = if msg.is_null() { "Contract assertion failed" } else { CStr::from_ptr(msg).to_str().unwrap_or("Contract assertion failed") };
    let filename = if file.is_null() { "<unknown>" } else { CStr::from_ptr(file).to_str().unwrap_or("<unknown>") };
    eprintln!("Pilot runtime panic: {} at {}:{}", message, filename, line);
    std::process::exit(101);
}

// --- Collections & Channels C-ABI ---

#[no_mangle]
pub extern "C" fn pilot_rt_list_new() -> *mut c_void {
    let list: Vec<i64> = Vec::new();
    Box::into_raw(Box::new(list)) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_list_push(list_ptr: *mut c_void, val: i64) {
    if !list_ptr.is_null() {
        let list = &mut *(list_ptr as *mut Vec<i64>);
        list.push(val);
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_list_get(list_ptr: *mut c_void, idx: usize) -> i64 {
    if !list_ptr.is_null() {
        let list = &*(list_ptr as *mut Vec<i64>);
        list.get(idx).copied().unwrap_or(0)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_list_len(list_ptr: *mut c_void) -> usize {
    if !list_ptr.is_null() {
        let list = &*(list_ptr as *mut Vec<i64>);
        list.len()
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_list_free(list_ptr: *mut c_void) {
    if !list_ptr.is_null() {
        let _ = Box::from_raw(list_ptr as *mut Vec<i64>);
    }
}

#[no_mangle]
pub extern "C" fn pilot_rt_map_new() -> *mut c_void {
    let map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    Box::into_raw(Box::new(map)) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_map_insert(
    map_ptr: *mut c_void,
    key: *const c_char,
    val: i64,
) {
    if !map_ptr.is_null() && !key.is_null() {
        let map = &mut *(map_ptr as *mut std::collections::HashMap<String, i64>);
        let k = CStr::from_ptr(key).to_str().unwrap_or("").to_string();
        map.insert(k, val);
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_map_get(map_ptr: *mut c_void, key: *const c_char) -> i64 {
    if !map_ptr.is_null() && !key.is_null() {
        let map = &*(map_ptr as *mut std::collections::HashMap<String, i64>);
        let k = CStr::from_ptr(key).to_str().unwrap_or("");
        map.get(k).copied().unwrap_or(0)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_map_free(map_ptr: *mut c_void) {
    if !map_ptr.is_null() {
        let _ = Box::from_raw(map_ptr as *mut std::collections::HashMap<String, i64>);
    }
}

#[no_mangle]
pub extern "C" fn pilot_rt_table_new() -> *mut c_void {
    pilot_rt_map_new()
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_table_free(ptr: *mut c_void) {
    pilot_rt_map_free(ptr)
}

#[no_mangle]
pub extern "C" fn pilot_rt_chan_new(cap: usize) -> *mut c_void {
    let chan = crate::concurrency::Channel::new(cap);
    Box::into_raw(Box::new(chan)) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_chan_send(chan_ptr: *mut c_void, val: i64) -> bool {
    if !chan_ptr.is_null() {
        let chan = &*(chan_ptr as *mut crate::concurrency::Channel);
        chan.send(crate::value::PilotVal::from_int(val))
    } else {
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_chan_recv(chan_ptr: *mut c_void, out_val: *mut i64) -> bool {
    if !chan_ptr.is_null() && !out_val.is_null() {
        let chan = &*(chan_ptr as *mut crate::concurrency::Channel);
        if let Some(v) = chan.recv() {
            *out_val = v.as_int().unwrap_or(0);
            true
        } else {
            false
        }
    } else {
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_chan_free(chan_ptr: *mut c_void) {
    if !chan_ptr.is_null() {
        let _ = Box::from_raw(chan_ptr as *mut crate::concurrency::Channel);
    }
}

#[no_mangle]
pub extern "C" fn pilot_rt_time_now_ns() -> u64 {
    crate::time::now_nanos()
}

#[no_mangle]
pub extern "C" fn pilot_rt_sleep_ms(ms: u64) {
    crate::time::sleep_ms(ms);
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_crypto_sha256(data: *const u8, len: usize, out_hash: *mut u8) {
    if data.is_null() || out_hash.is_null() {
        return;
    }
    let slice = core::slice::from_raw_parts(data, len);
    let hash = crate::crypto::sha256(slice);
    core::ptr::copy_nonoverlapping(hash.as_ptr(), out_hash, 32);
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_crypto_random_bytes(out_ptr: *mut u8, len: usize) {
    if out_ptr.is_null() || len == 0 {
        return;
    }
    let bytes = crate::crypto::random_bytes(len);
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_ptr, len);
}

/// Generates the canonical `pilot_runtime.h` C header for native compilation.
pub fn generate_c_runtime_header() -> &'static str {
    r#"/* Pilot Language Native C-ABI Runtime Header (C11 / C23) */
#ifndef PILOT_RUNTIME_H
#define PILOT_RUNTIME_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Standard Pilot Option/Result & Keyword Macros */
#ifndef Ok
#define Ok(x) ((int64_t)(intptr_t)(x))
#endif
#ifndef Err
#define Err(x) ((int64_t)(intptr_t)(x))
#endif
#ifndef Some
#define Some(x) ((int64_t)(intptr_t)(x))
#endif
#ifndef None
#define None 0LL
#endif
#ifndef null
#define null NULL
#endif
#ifndef async
#define async
#endif
#ifndef await
#define await
#endif
#ifndef pure
#define pure
#endif
#ifndef noalloc
#define noalloc
#endif

/* Memory Tier 1: Bump Regions */
static inline void* pilot_rt_region_enter(void) { return malloc(65536); }
static inline void* pilot_rt_region_alloc(void* region, size_t size, size_t align) {
    (void)region; (void)align;
    return malloc(size);
}
static inline void pilot_rt_region_exit(void* region) { free(region); }

/* Zero-Marshalling Contiguous Shared Memory SliceBuffer (Milestone 15, 20, 23, 28, 35) */
typedef struct {
    void* ptr;
    size_t len;
    size_t capacity;
} PilotSliceBuffer;

static inline PilotSliceBuffer pilot_rt_slice_create(size_t capacity, size_t elem_size) {
    PilotSliceBuffer buf;
    buf.ptr = malloc(capacity * elem_size);
    buf.len = 0;
    buf.capacity = capacity;
    return buf;
}

static inline void pilot_rt_slice_free(PilotSliceBuffer* buf) {
    if (buf && buf->ptr) {
        free(buf->ptr);
        buf->ptr = NULL;
        buf->len = 0;
        buf->capacity = 0;
    }
}

/* Memory Tier 2: Perceus Precise ARC & Drop Reuse */
static inline void* pilot_rt_alloc_arc(size_t payload_size) {
    size_t* p = (size_t*)malloc(sizeof(size_t) * 2 + payload_size);
    if (!p) return NULL;
    p[0] = 1; // rc
    p[1] = payload_size;
    return (void*)(p + 2);
}

static inline void* pilot_rt_dup(void* ptr) {
    if (!ptr) return NULL;
    size_t* p = ((size_t*)ptr) - 2;
    p[0]++;
    return ptr;
}

static inline void pilot_rt_drop(void* ptr, void (*dtor)(void*)) {
    if (!ptr) return;
    size_t* p = ((size_t*)ptr) - 2;
    if (--p[0] == 0) {
        if (dtor) dtor(ptr);
        free(p);
    }
}

static inline void* pilot_rt_drop_reuse(void* ptr, size_t required_size) {
    if (!ptr) return pilot_rt_alloc_arc(required_size);
    size_t* p = ((size_t*)ptr) - 2;
    if (p[0] == 1 && p[1] >= required_size) return ptr;
    pilot_rt_drop(ptr, NULL);
    return pilot_rt_alloc_arc(required_size);
}

/* Strings & Intrinsics */
static inline char* pilot_rt_str_concat(const char* s1, const char* s2) {
    const char* str1 = s1 ? s1 : "";
    const char* str2 = s2 ? s2 : "";
    size_t len1 = strlen(str1);
    size_t len2 = strlen(str2);
    char* buf = (char*)malloc(len1 + len2 + 1);
    if (buf) {
        memcpy(buf, str1, len1);
        memcpy(buf + len1, str2, len2);
        buf[len1 + len2] = '\0';
    }
    return buf;
}

static inline void pilot_rt_str_free(char* ptr) { free(ptr); }
static inline void pilot_rt_print_i64(int64_t val) { printf("%lld\n", (long long)val); }
static inline void pilot_rt_print_f64(double val) { printf("%f\n", val); }
static inline void pilot_rt_print_str(const char* s) { printf("%s\n", s ? s : ""); }
static inline uint64_t pilot_rt_time_now_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
}
static inline void pilot_rt_sleep_ms(uint64_t ms) {
    struct timespec ts;
    ts.tv_sec = (time_t)(ms / 1000);
    ts.tv_nsec = (long)((ms % 1000) * 1000000);
    nanosleep(&ts, NULL);
}
static inline void pilot_rt_panic_contract(const char* msg, const char* file, uint32_t line) {
    fprintf(stderr, "Pilot runtime panic: %s at %s:%u\n", msg ? msg : "failed", file ? file : "<unknown>", line);
    exit(101);
}

#ifdef __cplusplus
}
#endif

#endif /* PILOT_RUNTIME_H */
"#
}
