//! Low-level irreducible OS/libc syscall wrappers & stdlib module path resolver.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use crate::PilotVal;

fn walk_files(dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() { walk_files(&p, out); } else { out.push(p.to_string_lossy().replace('\\', "/")); }
    }
}

// Restriction: Raw irreducible OS syscalls; Forbidden: Pilot language semantics, string manipulations, or collections
pub struct PilotOsSyscall;

impl PilotOsSyscall {
    pub fn fs_open(path: &str, read: bool, write: bool, create: bool) -> std::io::Result<File> {
        OpenOptions::new().read(read).write(write).create(create).open(path)
    }
    pub fn fs_read(file: &mut File, buf: &mut [u8]) -> std::io::Result<usize> { file.read(buf) }
    pub fn fs_write(file: &mut File, buf: &[u8]) -> std::io::Result<usize> { file.write(buf) }
    pub fn monotonic_nanos() -> u64 { crate::time::now_nanos() }
    pub fn env_var(key: &str) -> Option<String> { std::env::var(key).ok() }
    pub fn cwd() -> Option<String> { std::env::current_dir().ok().map(|p| p.to_string_lossy().into_owned()) }
    pub fn args() -> Vec<String> { std::env::args().collect() }

    pub fn vm_intrinsic(op: &str, a: &[PilotVal]) -> PilotVal {
        let s0 = || a.first().map(|v| v.to_string_repr()).unwrap_or_default();
        match op {
            "fs_exists" => PilotVal::from_bool(Path::new(&s0()).exists()),
            "fs_read_text" => PilotVal::from_str(&fs::read_to_string(s0()).unwrap_or_default()),
            "fs_write_text" if a.len() >= 2 => {
                let p = s0();
                if let Some(d) = Path::new(&p).parent().filter(|d| !d.as_os_str().is_empty()) { let _ = fs::create_dir_all(d); }
                PilotVal::from_bool(fs::write(&p, a[1].to_string_repr()).is_ok())
            }
            "fs_walk_files" => { let mut v = Vec::new(); walk_files(Path::new(&s0()), &mut v); v.sort(); PilotVal::from_str(&v.join("\n")) }
            "fs_list_dirs" => {
                let mut v: Vec<String> = fs::read_dir(s0()).ok().into_iter().flatten().flatten()
                    .filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
                v.sort(); PilotVal::from_str(&v.join("\n"))
            }
            "fs_remove_all" => {
                let p = s0(); let path = Path::new(&p);
                PilotVal::from_bool(if path.is_dir() { fs::remove_dir_all(path).is_ok() } else { fs::remove_file(path).is_ok() })
            }
            "time_now_us" => PilotVal::from_int((Self::monotonic_nanos() / 1_000) as i64),
            "proc_pid" => PilotVal::from_int(std::process::id() as i64),
            _ => PilotVal::NULL,
        }
    }
}

// Restriction: Mechanical stdlib module path resolver; Forbidden: hardcoded module tables
pub struct PilotStdlibResolver;

impl PilotStdlibResolver {
    pub fn is_stdlib_module(name: &str) -> bool { name.starts_with("std.") && name.len() > 4 }
    pub fn resolve_relative_path(name: &str) -> Option<String> {
        let sub = name.strip_prefix("std.")?;
        if sub.is_empty() || sub.contains('/') || sub.contains('\\') { return None; }
        Some(format!("{}/{}.pilot", sub, sub))
    }
}
