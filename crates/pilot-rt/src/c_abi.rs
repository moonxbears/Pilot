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
#include <math.h>
#include <time.h>

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

/* Dynamic List Representation */
typedef struct PilotList {
    void** items;
    size_t len;
    size_t cap;
} PilotList;

typedef PilotList* list;

static inline PilotList* pilot_rt_list_new(void) {
    PilotList* l = (PilotList*)malloc(sizeof(PilotList));
    if (!l) return NULL;
    l->len = 0;
    l->cap = 8;
    l->items = (void**)malloc(sizeof(void*) * l->cap);
    return l;
}

static inline PilotList* pilot_rt_list_with_capacity(size_t cap) {
    PilotList* l = (PilotList*)malloc(sizeof(PilotList));
    if (!l) return NULL;
    if (cap < 4) cap = 4;
    l->len = 0;
    l->cap = cap;
    l->items = (void**)malloc(sizeof(void*) * l->cap);
    return l;
}

static inline PilotList* pilot_rt_list_from_array(size_t count, void** items) {
    PilotList* l = pilot_rt_list_with_capacity(count);
    if (!l) return NULL;
    for (size_t i = 0; i < count; i++) {
        l->items[i] = items[i];
    }
    l->len = count;
    return l;
}

#define PILOT_LIST_OF(...) pilot_rt_list_from_array(sizeof((void*[]){__VA_ARGS__}) / sizeof(void*), (void*[]){__VA_ARGS__})

static inline void pilot_rt_list_push(PilotList* l, void* item) {
    if (!l) return;
    if (l->len >= l->cap) {
        l->cap = (l->cap == 0) ? 8 : (l->cap * 2);
        l->items = (void**)realloc(l->items, sizeof(void*) * l->cap);
    }
    l->items[l->len++] = item;
}

static inline void* pilot_rt_list_pop(PilotList* l) {
    if (!l || l->len == 0) return NULL;
    return l->items[--l->len];
}

static inline void* pilot_rt_list_get(const PilotList* l, int64_t index) {
    if (!l) return NULL;
    if (index < 0) index += (int64_t)l->len;
    if (index < 0 || (size_t)index >= l->len) return NULL;
    return l->items[(size_t)index];
}

static inline void pilot_rt_list_set(PilotList* l, int64_t index, void* val) {
    if (!l) return;
    if (index < 0) index += (int64_t)l->len;
    if (index < 0 || (size_t)index >= l->len) return;
    l->items[(size_t)index] = val;
}

static inline void pilot_rt_list_insert(PilotList* l, int64_t index, void* item) {
    if (!l) return;
    if (index < 0) index = 0;
    if ((size_t)index > l->len) index = (int64_t)l->len;
    if (l->len >= l->cap) {
        l->cap = (l->cap == 0) ? 8 : (l->cap * 2);
        l->items = (void**)realloc(l->items, sizeof(void*) * l->cap);
    }
    memmove(&l->items[index + 1], &l->items[index], (l->len - (size_t)index) * sizeof(void*));
    l->items[index] = item;
    l->len++;
}

static inline void* pilot_rt_list_remove(PilotList* l, int64_t index) {
    if (!l || l->len == 0) return NULL;
    if (index < 0) index += (int64_t)l->len;
    if (index < 0 || (size_t)index >= l->len) return NULL;
    void* val = l->items[index];
    memmove(&l->items[index], &l->items[index + 1], (l->len - (size_t)index - 1) * sizeof(void*));
    l->len--;
    return val;
}

static inline void pilot_rt_list_clear(PilotList* l) {
    if (l) l->len = 0;
}

static inline void pilot_rt_list_reverse(PilotList* l) {
    if (!l || l->len <= 1) return;
    size_t i = 0, j = l->len - 1;
    while (i < j) {
        void* tmp = l->items[i];
        l->items[i] = l->items[j];
        l->items[j] = tmp;
        i++;
        j--;
    }
}

static inline bool pilot_rt_list_contains(const PilotList* l, void* item) {
    if (!l) return false;
    for (size_t i = 0; i < l->len; i++) {
        if (l->items[i] == item) return true;
        if (item && l->items[i] && (uintptr_t)item > 0x10000 && (uintptr_t)l->items[i] > 0x10000) {
            if (strcmp((const char*)l->items[i], (const char*)item) == 0) return true;
        }
    }
    return false;
}

static inline int64_t pilot_rt_list_index_of(const PilotList* l, void* item) {
    if (!l) return -1;
    for (size_t i = 0; i < l->len; i++) {
        if (l->items[i] == item) return (int64_t)i;
        if (item && l->items[i] && (uintptr_t)item > 0x10000 && (uintptr_t)l->items[i] > 0x10000) {
            if (strcmp((const char*)l->items[i], (const char*)item) == 0) return (int64_t)i;
        }
    }
    return -1;
}

static inline PilotList* pilot_rt_list_slice(const PilotList* l, int64_t start, int64_t count) {
    if (!l) return pilot_rt_list_new();
    if (start < 0) start += (int64_t)l->len;
    if (start < 0) start = 0;
    if ((size_t)start >= l->len || count <= 0) return pilot_rt_list_new();
    size_t actual_count = (size_t)count;
    if ((size_t)start + actual_count > l->len) {
        actual_count = l->len - (size_t)start;
    }
    PilotList* res = pilot_rt_list_with_capacity(actual_count);
    for (size_t i = 0; i < actual_count; i++) {
        res->items[i] = l->items[(size_t)start + i];
    }
    res->len = actual_count;
    return res;
}

static inline void* pilot_rt_list_first(const PilotList* l) {
    if (!l || l->len == 0) return NULL;
    return l->items[0];
}

static inline void* pilot_rt_list_last(const PilotList* l) {
    if (!l || l->len == 0) return NULL;
    return l->items[l->len - 1];
}

static inline void pilot_rt_list_free(PilotList* l) {
    if (l) {
        if (l->items) free(l->items);
        free(l);
    }
}

static inline char* pilot_rt_list_join(const PilotList* l, const char* sep) {
    if (!l || l->len == 0) {
        char* empty = (char*)malloc(1);
        if (empty) empty[0] = '\0';
        return empty;
    }
    const char* separator = sep ? sep : "";
    size_t sep_len = strlen(separator);
    size_t total_len = 0;
    for (size_t i = 0; i < l->len; i++) {
        const char* str = (const char*)l->items[i];
        if (str && (uintptr_t)str > 0x10000) total_len += strlen(str);
        else total_len += 24;
        if (i + 1 < l->len) total_len += sep_len;
    }
    char* out = (char*)malloc(total_len + 1);
    if (!out) return NULL;
    out[0] = '\0';
    size_t curr = 0;
    for (size_t i = 0; i < l->len; i++) {
        const char* str = (const char*)l->items[i];
        char temp_num[32];
        if (!str || (uintptr_t)str <= 0x10000) {
            snprintf(temp_num, sizeof(temp_num), "%lld", (long long)(intptr_t)str);
            str = temp_num;
        }
        size_t slen = strlen(str);
        memcpy(out + curr, str, slen);
        curr += slen;
        if (i + 1 < l->len) {
            memcpy(out + curr, separator, sep_len);
            curr += sep_len;
        }
    }
    out[curr] = '\0';
    return out;
}

static inline char* pilot_rt_list_to_str(const PilotList* l) {
    if (!l) {
        char* n = (char*)malloc(5);
        if (n) strcpy(n, "null");
        return n;
    }
    size_t cap = 64;
    char* buf = (char*)malloc(cap);
    if (!buf) return NULL;
    strcpy(buf, "[");
    size_t curr = 1;
    for (size_t i = 0; i < l->len; i++) {
        char item_buf[64];
        const char* item_str = NULL;
        void* item = l->items[i];
        if ((uintptr_t)item > 0x10000 && (uintptr_t)item < 0x7FFFFFFFFFFFLL) {
            const char* s = (const char*)item;
            if (s[0] >= 32 && s[0] <= 126) {
                snprintf(item_buf, sizeof(item_buf), "\"%s\"", s);
                item_str = item_buf;
            }
        }
        if (!item_str) {
            snprintf(item_buf, sizeof(item_buf), "%lld", (long long)(intptr_t)item);
            item_str = item_buf;
        }
        size_t slen = strlen(item_str);
        if (curr + slen + 4 >= cap) {
            cap = (curr + slen + 4) * 2;
            buf = (char*)realloc(buf, cap);
        }
        memcpy(buf + curr, item_str, slen);
        curr += slen;
        if (i + 1 < l->len) {
            memcpy(buf + curr, ", ", 2);
            curr += 2;
        }
    }
    buf[curr++] = ']';
    buf[curr] = '\0';
    return buf;
}

static inline void pilot_rt_print_list(const PilotList* l) {
    char* str = pilot_rt_list_to_str(l);
    if (str) {
        printf("%s\n", str);
        free(str);
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
static inline int64_t pilot_rt_str_len(const char* s) {
    return s ? (int64_t)strlen(s) : 0;
}

static inline bool pilot_rt_str_is_empty(const char* s) {
    return !s || s[0] == '\0';
}

static inline bool pilot_rt_str_contains(const char* s, const char* sub) {
    if (!s || !sub) return false;
    return strstr(s, sub) != NULL;
}

static inline bool pilot_rt_str_starts_with(const char* s, const char* prefix) {
    if (!s || !prefix) return false;
    size_t pre_len = strlen(prefix);
    size_t s_len = strlen(s);
    if (pre_len > s_len) return false;
    return strncmp(s, prefix, pre_len) == 0;
}

static inline bool pilot_rt_str_ends_with(const char* s, const char* suffix) {
    if (!s || !suffix) return false;
    size_t suf_len = strlen(suffix);
    size_t s_len = strlen(s);
    if (suf_len > s_len) return false;
    return strcmp(s + (s_len - suf_len), suffix) == 0;
}

static inline int64_t pilot_rt_str_index_of(const char* s, const char* sub) {
    if (!s || !sub) return -1;
    const char* found = strstr(s, sub);
    return found ? (int64_t)(found - s) : -1;
}

static inline int64_t pilot_rt_str_last_index_of(const char* s, const char* sub) {
    if (!s || !sub) return -1;
    size_t sub_len = strlen(sub);
    size_t s_len = strlen(s);
    if (sub_len > s_len) return -1;
    for (int64_t i = (int64_t)(s_len - sub_len); i >= 0; i--) {
        if (strncmp(s + i, sub, sub_len) == 0) return i;
    }
    return -1;
}

static inline char* pilot_rt_str_to_upper(const char* s) {
    if (!s) return NULL;
    size_t len = strlen(s);
    char* buf = (char*)malloc(len + 1);
    if (!buf) return NULL;
    for (size_t i = 0; i < len; i++) {
        buf[i] = (char)((s[i] >= 'a' && s[i] <= 'z') ? (s[i] - 32) : s[i]);
    }
    buf[len] = '\0';
    return buf;
}

static inline char* pilot_rt_str_to_lower(const char* s) {
    if (!s) return NULL;
    size_t len = strlen(s);
    char* buf = (char*)malloc(len + 1);
    if (!buf) return NULL;
    for (size_t i = 0; i < len; i++) {
        buf[i] = (char)((s[i] >= 'A' && s[i] <= 'Z') ? (s[i] + 32) : s[i]);
    }
    buf[len] = '\0';
    return buf;
}

static inline char* pilot_rt_str_trim(const char* s) {
    if (!s) return NULL;
    while (*s == ' ' || *s == '\t' || *s == '\r' || *s == '\n') s++;
    size_t len = strlen(s);
    while (len > 0 && (s[len - 1] == ' ' || s[len - 1] == '\t' || s[len - 1] == '\r' || s[len - 1] == '\n')) len--;
    char* buf = (char*)malloc(len + 1);
    if (!buf) return NULL;
    memcpy(buf, s, len);
    buf[len] = '\0';
    return buf;
}

static inline char* pilot_rt_str_trim_start(const char* s) {
    if (!s) return NULL;
    while (*s == ' ' || *s == '\t' || *s == '\r' || *s == '\n') s++;
    size_t len = strlen(s);
    char* buf = (char*)malloc(len + 1);
    if (!buf) return NULL;
    memcpy(buf, s, len);
    buf[len] = '\0';
    return buf;
}

static inline char* pilot_rt_str_trim_end(const char* s) {
    if (!s) return NULL;
    size_t len = strlen(s);
    while (len > 0 && (s[len - 1] == ' ' || s[len - 1] == '\t' || s[len - 1] == '\r' || s[len - 1] == '\n')) len--;
    char* buf = (char*)malloc(len + 1);
    if (!buf) return NULL;
    memcpy(buf, s, len);
    buf[len] = '\0';
    return buf;
}

static inline char* pilot_rt_str_replace(const char* s, const char* old_sub, const char* new_sub) {
    if (!s) return NULL;
    if (!old_sub || old_sub[0] == '\0') {
        size_t len = strlen(s);
        char* buf = (char*)malloc(len + 1);
        if (buf) memcpy(buf, s, len + 1);
        return buf;
    }
    const char* replacement = new_sub ? new_sub : "";
    size_t old_len = strlen(old_sub);
    size_t new_len = strlen(replacement);
    size_t count = 0;
    const char* tmp = s;
    while ((tmp = strstr(tmp, old_sub)) != NULL) {
        count++;
        tmp += old_len;
    }
    size_t result_len = strlen(s) + count * (new_len > old_len ? (new_len - old_len) : 0);
    char* result = (char*)malloc(result_len + 1);
    if (!result) return NULL;
    char* dst = result;
    const char* src = s;
    while ((tmp = strstr(src, old_sub)) != NULL) {
        size_t chunk = (size_t)(tmp - src);
        memcpy(dst, src, chunk);
        dst += chunk;
        memcpy(dst, replacement, new_len);
        dst += new_len;
        src = tmp + old_len;
    }
    strcpy(dst, src);
    return result;
}

static inline char* pilot_rt_str_substring(const char* s, int64_t start, int64_t count) {
    if (!s) return NULL;
    int64_t s_len = (int64_t)strlen(s);
    if (start < 0) start += s_len;
    if (start < 0) start = 0;
    if (start >= s_len || count <= 0) {
        char* empty = (char*)malloc(1);
        if (empty) empty[0] = '\0';
        return empty;
    }
    if (start + count > s_len) count = s_len - start;
    char* buf = (char*)malloc((size_t)count + 1);
    if (!buf) return NULL;
    memcpy(buf, s + start, (size_t)count);
    buf[count] = '\0';
    return buf;
}

static inline char* pilot_rt_str_repeat(const char* s, int64_t count) {
    if (!s || count <= 0) {
        char* empty = (char*)malloc(1);
        if (empty) empty[0] = '\0';
        return empty;
    }
    size_t s_len = strlen(s);
    size_t total = s_len * (size_t)count;
    char* buf = (char*)malloc(total + 1);
    if (!buf) return NULL;
    for (int64_t i = 0; i < count; i++) {
        memcpy(buf + i * s_len, s, s_len);
    }
    buf[total] = '\0';
    return buf;
}

static inline char* pilot_rt_str_pad_start(const char* s, int64_t target_len, const char* pad_char) {
    if (!s) s = "";
    int64_t s_len = (int64_t)strlen(s);
    if (s_len >= target_len) {
        char* res = (char*)malloc(s_len + 1);
        if (res) strcpy(res, s);
        return res;
    }
    char ch = (pad_char && pad_char[0] != '\0') ? pad_char[0] : ' ';
    int64_t pad_amount = target_len - s_len;
    char* buf = (char*)malloc((size_t)target_len + 1);
    if (!buf) return NULL;
    memset(buf, ch, (size_t)pad_amount);
    memcpy(buf + pad_amount, s, (size_t)s_len);
    buf[target_len] = '\0';
    return buf;
}

static inline char* pilot_rt_str_pad_end(const char* s, int64_t target_len, const char* pad_char) {
    if (!s) s = "";
    int64_t s_len = (int64_t)strlen(s);
    if (s_len >= target_len) {
        char* res = (char*)malloc(s_len + 1);
        if (res) strcpy(res, s);
        return res;
    }
    char ch = (pad_char && pad_char[0] != '\0') ? pad_char[0] : ' ';
    int64_t pad_amount = target_len - s_len;
    char* buf = (char*)malloc((size_t)target_len + 1);
    if (!buf) return NULL;
    memcpy(buf, s, (size_t)s_len);
    memset(buf + s_len, ch, (size_t)pad_amount);
    buf[target_len] = '\0';
    return buf;
}

static inline bool pilot_rt_str_is_digit(const char* s) {
    if (!s || s[0] == '\0') return false;
    for (size_t i = 0; s[i] != '\0'; i++) {
        if (s[i] < '0' || s[i] > '9') return false;
    }
    return true;
}

static inline bool pilot_rt_str_is_alpha(const char* s) {
    if (!s || s[0] == '\0') return false;
    for (size_t i = 0; s[i] != '\0'; i++) {
        if (!((s[i] >= 'a' && s[i] <= 'z') || (s[i] >= 'A' && s[i] <= 'Z'))) return false;
    }
    return true;
}

static inline bool pilot_rt_str_is_alphanumeric(const char* s) {
    if (!s || s[0] == '\0') return false;
    for (size_t i = 0; s[i] != '\0'; i++) {
        char c = s[i];
        if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9'))) return false;
    }
    return true;
}

static inline int64_t pilot_rt_str_to_int(const char* s) {
    if (!s) return 0;
    return (int64_t)strtoll(s, NULL, 10);
}

static inline double pilot_rt_str_to_float(const char* s) {
    if (!s) return 0.0;
    return strtod(s, NULL);
}

static inline char* pilot_rt_str_char_at(const char* s, int64_t index) {
    if (!s) return NULL;
    int64_t s_len = (int64_t)strlen(s);
    if (index < 0) index += s_len;
    if (index < 0 || index >= s_len) {
        char* empty = (char*)malloc(1);
        if (empty) empty[0] = '\0';
        return empty;
    }
    char* buf = (char*)malloc(2);
    if (!buf) return NULL;
    buf[0] = s[index];
    buf[1] = '\0';
    return buf;
}

static inline PilotList* pilot_rt_str_chars(const char* s) {
    if (!s) return pilot_rt_list_new();
    size_t len = strlen(s);
    PilotList* list = pilot_rt_list_with_capacity(len);
    for (size_t i = 0; i < len; i++) {
        char* ch = (char*)malloc(2);
        if (ch) {
            ch[0] = s[i];
            ch[1] = '\0';
            pilot_rt_list_push(list, ch);
        }
    }
    return list;
}

static inline PilotList* pilot_rt_str_split(const char* s, const char* sep) {
    if (!s) return pilot_rt_list_new();
    if (!sep || sep[0] == '\0') {
        return pilot_rt_str_chars(s);
    }
    size_t sep_len = strlen(sep);
    PilotList* list = pilot_rt_list_new();
    const char* cur = s;
    const char* found;
    while ((found = strstr(cur, sep)) != NULL) {
        size_t part_len = (size_t)(found - cur);
        char* part = (char*)malloc(part_len + 1);
        if (part) {
            memcpy(part, cur, part_len);
            part[part_len] = '\0';
            pilot_rt_list_push(list, part);
        }
        cur = found + sep_len;
    }
    size_t rem_len = strlen(cur);
    char* rem = (char*)malloc(rem_len + 1);
    if (rem) {
        memcpy(rem, cur, rem_len + 1);
        pilot_rt_list_push(list, rem);
    }
    return list;
}

static inline PilotList* pilot_rt_str_range(const char* start, const char* end, bool inclusive) {
    if (!start || !end) return pilot_rt_list_new();
    size_t s_len = strlen(start);
    size_t e_len = strlen(end);
    if (s_len == 0 || e_len == 0) return pilot_rt_list_new();

    char s0 = start[0];
    char e0 = end[0];

    bool s_lower = (s0 >= 'a' && s0 <= 'z');
    bool s_upper = (s0 >= 'A' && s0 <= 'Z');
    bool s_digit = (s0 >= '0' && s0 <= '9');

    bool e_lower = (e0 >= 'a' && e0 <= 'z');
    bool e_upper = (e0 >= 'A' && e0 <= 'Z');
    bool e_digit = (e0 >= '0' && e0 <= '9');

    char alpha[64];
    size_t M = 0;

    if (s_digit && e_digit) {
        for (char c = '0'; c <= '9'; c++) alpha[M++] = c;
    } else if (s_lower && e_lower) {
        for (char c = 'a'; c <= 'z'; c++) alpha[M++] = c;
    } else if (s_upper && e_upper) {
        for (char c = 'A'; c <= 'Z'; c++) alpha[M++] = c;
    } else if (s_lower && e_upper) {
        if (s0 <= 'm') {
            for (char c = 'a'; c <= 'z'; c++) alpha[M++] = c;
            for (char c = 'A'; c <= 'Z'; c++) alpha[M++] = c;
        } else {
            for (char c = 'A'; c <= 'Z'; c++) alpha[M++] = c;
            for (char c = 'a'; c <= 'z'; c++) alpha[M++] = c;
        }
    } else if (s_upper && e_lower) {
        if (s0 <= 'M') {
            for (char c = 'A'; c <= 'Z'; c++) alpha[M++] = c;
            for (char c = 'a'; c <= 'z'; c++) alpha[M++] = c;
        } else {
            for (char c = 'a'; c <= 'z'; c++) alpha[M++] = c;
            for (char c = 'A'; c <= 'Z'; c++) alpha[M++] = c;
        }
    } else {
        for (char c = 'a'; c <= 'z'; c++) alpha[M++] = c;
    }

    if (M == 0) return pilot_rt_list_new();

    uint64_t s_rank = 0;
    uint64_t s_pow = 1;
    for (size_t k = 1; k < s_len; k++) {
        s_pow *= M;
        s_rank += s_pow;
    }
    uint64_t s_offset = 0;
    for (size_t i = 0; i < s_len; i++) {
        int64_t idx = -1;
        for (size_t k = 0; k < M; k++) { if (alpha[k] == start[i]) { idx = (int64_t)k; break; } }
        if (idx < 0) idx = 0;
        s_offset = s_offset * M + (uint64_t)idx;
    }
    s_rank += s_offset;

    uint64_t e_rank = 0;
    uint64_t e_pow = 1;
    for (size_t k = 1; k < e_len; k++) {
        e_pow *= M;
        e_rank += e_pow;
    }
    uint64_t e_offset = 0;
    for (size_t i = 0; i < e_len; i++) {
        int64_t idx = -1;
        for (size_t k = 0; k < M; k++) { if (alpha[k] == end[i]) { idx = (int64_t)k; break; } }
        if (idx < 0) idx = 0;
        e_offset = e_offset * M + (uint64_t)idx;
    }
    e_rank += e_offset;

    PilotList* list = pilot_rt_list_new();

    if (s_rank <= e_rank) {
        uint64_t limit = inclusive ? e_rank : (e_rank > 0 ? e_rank - 1 : 0);
        if (inclusive || e_rank > s_rank) {
            for (uint64_t r = s_rank; r <= limit; r++) {
                uint64_t _r = r;
                size_t _L = 1;
                uint64_t _p = M;
                while (_r >= _p) {
                    _r -= _p;
                    _L++;
                    _p *= M;
                }
                char* _res = (char*)malloc(_L + 1);
                if (_res) {
                    _res[_L] = '\0';
                    for (size_t _j = 0; _j < _L; _j++) {
                        _res[_L - 1 - _j] = alpha[_r % M];
                        _r /= M;
                    }
                    pilot_rt_list_push(list, _res);
                }
                if (list->len >= 1000000) break;
            }
        }
    } else {
        uint64_t limit = inclusive ? e_rank : e_rank + 1;
        if (inclusive || s_rank > e_rank) {
            uint64_t r = s_rank;
            while (1) {
                uint64_t _r = r;
                size_t _L = 1;
                uint64_t _p = M;
                while (_r >= _p) {
                    _r -= _p;
                    _L++;
                    _p *= M;
                }
                char* _res = (char*)malloc(_L + 1);
                if (_res) {
                    _res[_L] = '\0';
                    for (size_t _j = 0; _j < _L; _j++) {
                        _res[_L - 1 - _j] = alpha[_r % M];
                        _r /= M;
                    }
                    pilot_rt_list_push(list, _res);
                }
                if (list->len >= 1000000 || r == limit) break;
                r--;
            }
        }
    }

    return list;
}

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

/* Math & Conversions */
static inline int64_t pilot_rt_int_abs(int64_t x) { return x < 0 ? -x : x; }
static inline int64_t pilot_rt_int_clamp(int64_t x, int64_t min_v, int64_t max_v) {
    if (x < min_v) return min_v;
    if (x > max_v) return max_v;
    return x;
}
static inline int64_t pilot_rt_int_min(int64_t a, int64_t b) { return a < b ? a : b; }
static inline int64_t pilot_rt_int_max(int64_t a, int64_t b) { return a > b ? a : b; }

static inline char* pilot_rt_int_to_str(int64_t x) {
    char* buf = (char*)malloc(32);
    if (!buf) return NULL;
    snprintf(buf, 32, "%lld", (long long)x);
    return buf;
}

static inline char* pilot_rt_int_to_hex(int64_t x) {
    char* buf = (char*)malloc(32);
    if (!buf) return NULL;
    snprintf(buf, 32, "0x%llX", (unsigned long long)x);
    return buf;
}

static inline char* pilot_rt_int_to_bin(int64_t x) {
    char* buf = (char*)malloc(68);
    if (!buf) return NULL;
    if (x == 0) {
        strcpy(buf, "0b0");
        return buf;
    }
    char tmp[65];
    int idx = 0;
    uint64_t ux = (uint64_t)x;
    while (ux > 0) {
        tmp[idx++] = (ux & 1) ? '1' : '0';
        ux >>= 1;
    }
    buf[0] = '0';
    buf[1] = 'b';
    int out_idx = 2;
    for (int i = idx - 1; i >= 0; i--) {
        buf[out_idx++] = tmp[i];
    }
    buf[out_idx] = '\0';
    return buf;
}

static inline int64_t pilot_rt_int_bit_length(int64_t x) {
    if (x == 0) return 0;
    uint64_t ux = (uint64_t)(x < 0 ? -x : x);
    int64_t bits = 0;
    while (ux > 0) {
        bits++;
        ux >>= 1;
    }
    return bits;
}

static inline double pilot_rt_float_abs(double x) { return fabs(x); }
static inline double pilot_rt_float_clamp(double x, double min_v, double max_v) {
    if (x < min_v) return min_v;
    if (x > max_v) return max_v;
    return x;
}
static inline double pilot_rt_float_min(double a, double b) { return a < b ? a : b; }
static inline double pilot_rt_float_max(double a, double b) { return a > b ? a : b; }
static inline double pilot_rt_float_round(double x) { return round(x); }
static inline double pilot_rt_float_floor(double x) { return floor(x); }
static inline double pilot_rt_float_ceil(double x) { return ceil(x); }
static inline double pilot_rt_float_trunc(double x) { return trunc(x); }

static inline char* pilot_rt_float_to_str(double x) {
    char* buf = (char*)malloc(40);
    if (!buf) return NULL;
    snprintf(buf, 40, "%g", x);
    return buf;
}

static inline char* pilot_rt_bool_to_str(bool b) {
    return (char*)(b ? "true" : "false");
}

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
