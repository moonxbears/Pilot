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

// Restriction: Static stdlib module path registry; Forbidden: dynamic evaluation or AST rewriting
pub struct PilotStdlibResolver;

impl PilotStdlibResolver {
    pub const MODULE_MAP: &'static [(&'static str, &'static str)] = &[
        ("std.core", "core/core.pilot"),
        ("std.option", "core/option.pilot"),
        ("std.result", "core/result.pilot"),
        ("std.mem", "core/mem.pilot"),
        ("std.meta", "core/meta.pilot"),
        ("std.collections", "collections/collections.pilot"),
        ("std.itertools", "collections/itertools.pilot"),
        ("std.text", "text/text.pilot"),
        ("std.math", "math/math.pilot"),
        ("std.random", "math/random.pilot"),
        ("std.io", "io/io.pilot"),
        ("std.fs", "io/fs.pilot"),
        ("std.path", "io/path.pilot"),
        ("std.pathlib", "io/pathlib.pilot"),
        ("std.shutil", "io/shutil.pilot"),
        ("std.os", "os/os.pilot"),
        ("std.sys", "sys/sys.pilot"),
        ("std.functools", "functools/functools.pilot"),
        ("std.json", "json/json.pilot"),
        ("std.csv", "csv/csv.pilot"),
        ("std.pickle", "pickle/pickle.pilot"),
        ("std.tomllib", "tomllib/tomllib.pilot"),
        ("std.secrets", "secrets/secrets.pilot"),
        ("std.decimal", "decimal/decimal.pilot"),
        ("std.threading", "threading/threading.pilot"),
        ("std.multiprocessing", "multiprocessing/multiprocessing.pilot"),
        ("std.asyncio", "asyncio/asyncio.pilot"),
        ("std.encoding", "encoding/encoding.pilot"),
        ("std.formats", "formats/formats.pilot"),
        ("std.socket", "socket/socket.pilot"),
        ("std.urllib", "urllib/urllib.pilot"),
        ("std.http", "http/http.pilot"),
        ("std.request", "request/request.pilot"),
        ("std.unittest", "unittest/unittest.pilot"),
        ("std.logging", "logging/logging.pilot"),
        ("std.timeit", "timeit/timeit.pilot"),
        ("std.numpy", "numpy/numpy.pilot"),
        ("std.pandas", "pandas/pandas.pilot"),
        ("std.matplotlib", "matplotlib/matplotlib.pilot"),
        ("std.scikit", "scikit/scikit.pilot"),
    ];

    // Restriction: Read-only check for registered stdlib module paths
    pub fn is_stdlib_module(name: &str) -> bool {
        Self::MODULE_MAP.iter().any(|(m, _)| *m == name)
    }

    // Restriction: Read-only relative path resolution for stdlib modules
    pub fn get_module_relative_path(name: &str) -> Option<&'static str> {
        Self::MODULE_MAP.iter().find(|(m, _)| *m == name).map(|(_, p)| *p)
    }
}
