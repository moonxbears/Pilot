//! Low-level irreducible OS/libc syscall wrappers & stdlib module path resolver.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

// Restriction: Raw irreducible OS syscalls; Forbidden: Pilot language semantics, string manipulations, or collections
pub struct PilotOsSyscall;

impl PilotOsSyscall {
    // Restriction: File open syscall wrapper
    pub fn fs_open(path: &str, read: bool, write: bool, create: bool) -> std::io::Result<File> {
        OpenOptions::new().read(read).write(write).create(create).open(path)
    }

    // Restriction: File read syscall wrapper
    pub fn fs_read(file: &mut File, buf: &mut [u8]) -> std::io::Result<usize> {
        file.read(buf)
    }

    // Restriction: File write syscall wrapper
    pub fn fs_write(file: &mut File, buf: &[u8]) -> std::io::Result<usize> {
        file.write(buf)
    }

    // Restriction: Monotonic clock nanoseconds syscall
    pub fn monotonic_nanos() -> u64 {
        crate::time::now_nanos()
    }

    // Restriction: Process environment variable syscall
    pub fn env_var(key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    // Restriction: Process current working directory syscall
    pub fn cwd() -> Option<String> {
        std::env::current_dir().ok().map(|p| p.to_string_lossy().into_owned())
    }

    // Restriction: Process CLI arguments syscall
    pub fn args() -> Vec<String> {
        std::env::args().collect()
    }
}

// Restriction: Mechanical stdlib module path resolver; Forbidden: hardcoded module tables
pub struct PilotStdlibResolver;

impl PilotStdlibResolver {
    // Restriction: Name-blind prefix check; Forbidden: hardcoded module name tables
    pub fn is_stdlib_module(name: &str) -> bool {
        name.starts_with("std.") && name.len() > 4
    }

    // Restriction: Mechanical path derivation `<seg>/<seg>.pilot`; Forbidden: hardcoded module lists
    pub fn resolve_relative_path(name: &str) -> Option<String> {
        let sub = name.strip_prefix("std.")?;
        if sub.is_empty() || sub.contains('/') || sub.contains('\\') {
            return None;
        }
        Some(format!("{}/{}.pilot", sub, sub))
    }
}
