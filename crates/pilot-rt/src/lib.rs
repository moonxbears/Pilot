//! Zero-GC Bootstrap Runtime Kernel (`pilot-rt`).

pub mod c_abi;
pub mod stdlib;
pub mod time;
pub mod value;

pub use c_abi::{
    pilot_rt_invoke_abi_frame, pilot_rt_os_getrandom, DynamicLibraryHandle,
};
pub use stdlib::{PilotOsSyscall, PilotStdlibResolver};
pub use time::now_nanos;
pub use value::{
    PilotVal, TAG_BOOL, TAG_FLOAT, TAG_FN, TAG_HEAP_STR, TAG_INT, TAG_LIST, TAG_MAP, TAG_NULL,
    TAG_SSO_STR, TAG_UINT,
};
