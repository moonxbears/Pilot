//! Irreducible Wall-Clock Epoch Nanosecond Query (`pilot-rt`).

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns current wall-clock epoch timestamp in nanoseconds.
pub fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}
