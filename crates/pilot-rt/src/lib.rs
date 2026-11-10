//! Milestone 58: Purified Zero-GC Native Runtime Kernel (`pilot-rt`).

pub mod value;
pub mod memory;
pub mod concurrency;
pub mod stdlib;
pub mod c_abi;
pub mod time;
pub mod net;

pub use value::{PilotVal, TAG_INT, TAG_FLOAT, TAG_BOOL, TAG_SSO_STR, TAG_HEAP_STR, TAG_NULL, TAG_LIST, TAG_MAP, TAG_FN};
pub use memory::{ArcHeader, BumpRegion, FLAG_ATOMIC};
pub use concurrency::Channel;
pub use stdlib::{PilotOsSyscall, PilotStdlibResolver};
pub use time::{monotonic_micros, now_millis, now_nanos, sleep_ms, timespec_now};
pub use net::{tcp_connect, tcp_listen, udp_bind};
pub use c_abi::{
    generate_c_runtime_header, pilot_rt_alloc_arc, pilot_rt_drop, pilot_rt_drop_reuse,
    pilot_rt_dup, pilot_rt_os_getrandom, pilot_rt_print_f64, pilot_rt_print_i64,
    pilot_rt_region_alloc, pilot_rt_region_enter, pilot_rt_region_exit,
    pilot_rt_sleep_ms, pilot_rt_time_now_ns, DynamicLibraryHandle,
    pilot_rt_invoke_abi_frame,
};
