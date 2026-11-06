//! Milestone 35.3 & 42.1: Native Dynamic FFI Invocation Engine (`dlopen`/`LoadLibrary`).

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};

pub struct DynamicLibraryHandle {
    pub name: String,
    raw_handle: *mut c_void,
    pub is_loaded: bool,
}

unsafe impl Send for DynamicLibraryHandle {}
unsafe impl Sync for DynamicLibraryHandle {}

#[cfg(unix)]
mod os_dl {
    use super::*;
    pub const RTLD_NOW: c_int = 2;
    pub const RTLD_GLOBAL: c_int = 0x100;

    extern "C" {
        pub fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
        pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        pub fn dlclose(handle: *mut c_void) -> c_int;
        pub fn dlerror() -> *const c_char;
    }
}

#[cfg(windows)]
mod os_dl {
    use super::*;
    extern "system" {
        pub fn LoadLibraryA(lpLibFileName: *const c_char) -> *mut c_void;
        pub fn GetProcAddress(hModule: *mut c_void, lpProcName: *const c_char) -> *mut c_void;
        pub fn FreeLibrary(hModule: *mut c_void) -> c_int;
    }
}

impl DynamicLibraryHandle {
    pub fn open(lib_name: &str) -> Result<Self, String> {
        let c_name = CString::new(lib_name).map_err(|e| format!("Invalid library path: {}", e))?;

        #[cfg(unix)]
        {
            let handle = unsafe {
                os_dl::dlopen(c_name.as_ptr(), os_dl::RTLD_NOW | os_dl::RTLD_GLOBAL)
            };
            if handle.is_null() {
                let err_msg = unsafe {
                    let err = os_dl::dlerror();
                    if !err.is_null() {
                        CStr::from_ptr(err).to_string_lossy().to_string()
                    } else {
                        "Unknown dlopen error".to_string()
                    }
                };
                return Err(format!("Failed to load library '{}': {}", lib_name, err_msg));
            }
            Ok(Self {
                name: lib_name.to_string(),
                raw_handle: handle,
                is_loaded: true,
            })
        }

        #[cfg(windows)]
        {
            let handle = unsafe { os_dl::LoadLibraryA(c_name.as_ptr()) };
            if handle.is_null() {
                return Err(format!("Failed to load library '{}' via LoadLibraryA", lib_name));
            }
            Ok(Self {
                name: lib_name.to_string(),
                raw_handle: handle,
                is_loaded: true,
            })
        }

        #[cfg(not(any(unix, windows)))]
        {
            Ok(Self {
                name: lib_name.to_string(),
                raw_handle: std::ptr::null_mut(),
                is_loaded: false,
            })
        }
    }

    pub fn resolve_symbol(&self, symbol_name: &str) -> Result<*const (), String> {
        if !self.is_loaded || self.raw_handle.is_null() {
            return Err("Library is not loaded".to_string());
        }

        let c_symbol = CString::new(symbol_name)
            .map_err(|e| format!("Invalid symbol name: {}", e))?;

        #[cfg(unix)]
        {
            let sym = unsafe { os_dl::dlsym(self.raw_handle, c_symbol.as_ptr()) };
            if sym.is_null() {
                let err_msg = unsafe {
                    let err = os_dl::dlerror();
                    if !err.is_null() {
                        CStr::from_ptr(err).to_string_lossy().to_string()
                    } else {
                        "Symbol not found".to_string()
                    }
                };
                return Err(format!("Failed to resolve symbol '{}': {}", symbol_name, err_msg));
            }
            Ok(sym as *const ())
        }

        #[cfg(windows)]
        {
            let sym = unsafe { os_dl::GetProcAddress(self.raw_handle, c_symbol.as_ptr()) };
            if sym.is_null() {
                return Err(format!("Failed to resolve symbol '{}'", symbol_name));
            }
            Ok(sym as *const ())
        }

        #[cfg(not(any(unix, windows)))]
        {
            Err("Unsupported platform for dynamic symbol resolution".to_string())
        }
    }

    pub fn raw_handle(&self) -> *mut c_void {
        self.raw_handle
    }
}

impl Drop for DynamicLibraryHandle {
    fn drop(&mut self) {
        if self.is_loaded && !self.raw_handle.is_null() {
            #[cfg(unix)]
            unsafe {
                os_dl::dlclose(self.raw_handle);
            }
            #[cfg(windows)]
            unsafe {
                os_dl::FreeLibrary(self.raw_handle);
            }
            self.raw_handle = std::ptr::null_mut();
            self.is_loaded = false;
        }
    }
}
