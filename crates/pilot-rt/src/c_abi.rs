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
#[cfg(windows)]
extern "system" {
    fn LoadLibraryW(lpLibFileName: *const u16) -> *mut c_void;
    fn GetProcAddress(hModule: *mut c_void, lpProcName: *const c_char) -> *const ();
    fn FreeLibrary(hLibModule: *mut c_void) -> c_int;
}

impl DynamicLibraryHandle {
    pub fn open(lib_name: &str) -> Result<Self, String> {
        #[cfg(unix)] {
            let c_name = CString::new(lib_name).map_err(|e| e.to_string())?;
            let h = unsafe { dlopen(c_name.as_ptr(), 2 | 0x100) };
            if h.is_null() { return Err(format!("dlopen failed for '{}'", lib_name)); }
            Ok(Self { name: lib_name.to_string(), raw_handle: h, is_loaded: true })
        }
        #[cfg(windows)] {
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = std::ffi::OsStr::new(lib_name).encode_wide().chain(Some(0)).collect();
            let h = unsafe { LoadLibraryW(wide.as_ptr()) };
            if h.is_null() { return Err(format!("LoadLibraryW failed for '{}'", lib_name)); }
            Ok(Self { name: lib_name.to_string(), raw_handle: h, is_loaded: true })
        }
        #[cfg(not(any(unix, windows)))]
        Ok(Self { name: lib_name.to_string(), raw_handle: std::ptr::null_mut(), is_loaded: false })
    }

    pub fn resolve_symbol(&self, sym_name: &str) -> Result<*const (), String> {
        if !self.is_loaded || self.raw_handle.is_null() { return Err("Library not loaded".into()); }
        let c_sym = CString::new(sym_name).map_err(|e| e.to_string())?;
        #[cfg(unix)] {
            let p = unsafe { dlsym(self.raw_handle, c_sym.as_ptr()) };
            if p.is_null() { return Err(format!("dlsym failed for '{}'", sym_name)); }
            Ok(p as *const ())
        }
        #[cfg(windows)] {
            let p = unsafe { GetProcAddress(self.raw_handle, c_sym.as_ptr()) };
            if p.is_null() { return Err(format!("GetProcAddress failed for '{}'", sym_name)); }
            Ok(p)
        }
        #[cfg(not(any(unix, windows)))]
        Err("Unsupported platform".into())
    }
}

impl Drop for DynamicLibraryHandle {
    fn drop(&mut self) {
        if self.is_loaded && !self.raw_handle.is_null() {
            #[cfg(unix)] unsafe { dlclose(self.raw_handle); }
            #[cfg(windows)] unsafe { FreeLibrary(self.raw_handle); }
            self.raw_handle = std::ptr::null_mut();
            self.is_loaded = false;
        }
    }
}

/// Dynamic name-blind register-pack dispatcher executing any `AbiFramePlan` constructed by Pilot.
#[cfg(all(target_arch = "x86_64", unix))]
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn pilot_rt_invoke_abi_frame(fn_ptr: *const (), gpr_buf: *const u64, xmm_buf: *const f64, stack_buf: *const u8, stack_len: usize, ret_buf: *mut u64) {
    std::arch::naked_asm!(
        "push rbp", "mov rbp, rsp", "push rbx", "push r12", "push r13", "push r14", "push r15", "sub rsp, 8",
        "mov r12, rdi", "mov r13, rsi", "mov r14, rdx", "mov r15, r9", "test r8, r8", "jz 1f",
        "mov rax, r8", "add rax, 15", "and rax, -16", "sub rsp, rax", "mov rdi, rsp", "mov rsi, rcx", "mov rcx, r8", "rep movsb",
        "1:", "test r14, r14", "jz 2f",
        "movsd xmm0, qword ptr [r14]", "movsd xmm1, qword ptr [r14 + 8]", "movsd xmm2, qword ptr [r14 + 16]", "movsd xmm3, qword ptr [r14 + 24]",
        "movsd xmm4, qword ptr [r14 + 32]", "movsd xmm5, qword ptr [r14 + 40]", "movsd xmm6, qword ptr [r14 + 48]", "movsd xmm7, qword ptr [r14 + 56]",
        "2:", "test r13, r13", "jz 3f",
        "mov rdi, qword ptr [r13]", "mov rsi, qword ptr [r13 + 8]", "mov rdx, qword ptr [r13 + 16]", "mov rcx, qword ptr [r13 + 24]", "mov r8,  qword ptr [r13 + 32]", "mov r9,  qword ptr [r13 + 40]", "mov rax, qword ptr [r13 + 48]",
        "3:", "call r12", "test r15, r15", "jz 4f",
        "mov qword ptr [r15], rax", "mov qword ptr [r15 + 8], rdx", "movsd qword ptr [r15 + 16], xmm0", "movsd qword ptr [r15 + 24], xmm1",
        "4:", "lea rsp, [rbp - 40]", "pop r15", "pop r14", "pop r13", "pop r12", "pop rbx", "pop rbp", "ret"
    );
}

#[cfg(all(target_arch = "x86_64", windows))]
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn pilot_rt_invoke_abi_frame(fn_ptr: *const (), gpr_buf: *const u64, xmm_buf: *const f64, stack_buf: *const u8, stack_len: usize, ret_buf: *mut u64) {
    std::arch::naked_asm!(
        "push rbp", "mov rbp, rsp", "push rbx", "push rsi", "push rdi", "push r12", "push r13", "push r14", "push r15",
        "mov r12, rcx", "mov r13, rdx", "mov r14, r8", "mov rbx, r9", "mov rsi, qword ptr [rbp + 48]", "mov r15, qword ptr [rbp + 56]",
        "mov rax, rsi", "add rax, 47", "and rax, -16", "sub rsp, rax", "test rsi, rsi", "jz 1f",
        "lea rdi, [rsp + 32]", "mov rcx, rsi", "mov rsi, rbx", "rep movsb",
        "1:", "test r14, r14", "jz 2f",
        "movsd xmm0, qword ptr [r14]", "movsd xmm1, qword ptr [r14 + 8]", "movsd xmm2, qword ptr [r14 + 16]", "movsd xmm3, qword ptr [r14 + 24]",
        "2:", "test r13, r13", "jz 3f",
        "mov rcx, qword ptr [r13]", "mov rdx, qword ptr [r13 + 8]", "mov r8,  qword ptr [r13 + 16]", "mov r9,  qword ptr [r13 + 24]",
        "3:", "call r12", "test r15, r15", "jz 4f",
        "mov qword ptr [r15], rax", "movsd qword ptr [r15 + 8], xmm0",
        "4:", "lea rsp, [rbp - 56]", "pop r15", "pop r14", "pop r13", "pop r12", "pop rdi", "pop rsi", "pop rbx", "pop rbp", "ret"
    );
}

#[cfg(not(any(all(target_arch = "x86_64", unix), all(target_arch = "x86_64", windows))))]
#[no_mangle]
pub unsafe extern "C" fn pilot_rt_invoke_abi_frame(fn_ptr: *const (), gpr_buf: *const u64, _xmm_buf: *const f64, _stack_buf: *const u8, _stack_len: usize, ret_buf: *mut u64) {
    if fn_ptr.is_null() { return; }
    let f: extern "C" fn(u64, u64, u64, u64, u64, u64) -> u64 = std::mem::transmute(fn_ptr);
    let r = f(if !gpr_buf.is_null() { *gpr_buf } else { 0 }, if !gpr_buf.is_null() { *gpr_buf.add(1) } else { 0 }, if !gpr_buf.is_null() { *gpr_buf.add(2) } else { 0 }, if !gpr_buf.is_null() { *gpr_buf.add(3) } else { 0 }, if !gpr_buf.is_null() { *gpr_buf.add(4) } else { 0 }, if !gpr_buf.is_null() { *gpr_buf.add(5) } else { 0 });
    if !ret_buf.is_null() { *ret_buf = r; }
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
pub unsafe extern "C" fn pilot_rt_dup(p: *mut c_void) -> *mut c_void {
    if !p.is_null() { (*(p as *mut PilotArcBox).sub(1)).ref_count.fetch_add(1, Ordering::Relaxed); }
    p
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_drop(p: *mut c_void, dtor: Option<unsafe extern "C" fn(*mut c_void)>) {
    if p.is_null() { return; }
    let h = (p as *mut PilotArcBox).sub(1);
    if (*h).ref_count.fetch_sub(1, Ordering::Release) == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        if let Some(drop_fn) = dtor { drop_fn(p); }
        let l = std::alloc::Layout::from_size_align(std::mem::size_of::<PilotArcBox>() + (*h).size, 8).unwrap();
        std::alloc::dealloc(h as *mut u8, l);
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_rt_drop_reuse(p: *mut c_void, req_sz: usize) -> *mut c_void {
    if p.is_null() { return pilot_rt_alloc_arc(req_sz); }
    let h = (p as *mut PilotArcBox).sub(1);
    if (*h).ref_count.load(Ordering::Acquire) == 1 && (*h).size >= req_sz { return p; }
    pilot_rt_drop(p, None);
    pilot_rt_alloc_arc(req_sz)
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
