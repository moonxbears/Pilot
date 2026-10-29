//! Milestone 24 & Milestone 43.4: High-Precision Clocks, Monotonic Timers & Timespec (`stdlib/time`).

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static MONOTONIC_BASE: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn get_base() -> Instant {
    *MONOTONIC_BASE.get_or_init(Instant::now)
}

/// Returns current wall-clock epoch timestamp in nanoseconds.
pub fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Returns current wall-clock epoch timestamp in milliseconds.
pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Returns monotonic elapsed microseconds since process start.
pub fn monotonic_micros() -> u64 {
    let base = get_base();
    base.elapsed().as_micros() as u64
}

/// Returns monotonic elapsed nanoseconds since process start.
pub fn monotonic_nanos() -> u64 {
    let base = get_base();
    base.elapsed().as_nanos() as u64
}

/// Sleeps the current thread for `ms` milliseconds.
pub fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/// Returns a (seconds, nanoseconds) pair matching POSIX `struct timespec`.
pub fn timespec_now() -> (i64, i64) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    (now.as_secs() as i64, now.subsec_nanos() as i64)
}
