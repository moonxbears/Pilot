//! Single runtime host airlock (`HostOp` `0..=50`, `PROP-002a` §1 & §3, Freeze Model B+).

use crate::value::PilotVal;
use crate::vm::PilVirtualMachine;
use core::ffi::c_void;
use core::mem::{align_of, offset_of, size_of};
use core::sync::atomic::{AtomicI64, Ordering};
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Seek, SeekFrom, Write};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, TcpStream, ToSocketAddrs};
use std::os::raw::{c_char, c_int};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HostOp {
    SlabAlloc = 0,
    SlabRealloc = 1,
    SlabFree = 2,
    MemMap = 3,
    MemCopy = 4,
    MemSet = 5,
    MemCmp = 6,
    LoadInt = 7,
    StoreInt = 8,
    LoadFloat = 9,
    StoreFloat = 10,
    AtomicInt = 11,
    ThreadSpawn = 12,
    ThreadJoin = 13,
    ThreadCtl = 14,
    Futex = 15,
    FdOpen = 16,
    FdClose = 17,
    FdRead = 18,
    FdWrite = 19,
    FdPread = 20,
    FdPwrite = 21,
    FdSeekSync = 22,
    FdPipeDup = 23,
    IoPoll = 24,
    FdRawCtl = 25,
    FsStat = 26,
    FsReadDir = 27,
    FsMkdir = 28,
    FsRemove = 29,
    FsRenameLink = 30,
    FsPathCtl = 31,
    ProcSpawn = 32,
    ProcWait = 33,
    ProcSignal = 34,
    EnvCtl = 35,
    ClockNowNs = 36,
    SleepNs = 37,
    OsRandom = 38,
    NetSocket = 39,
    NetConnectBind = 40,
    NetIo = 41,
    NetResolve = 42,
    DlOpen = 43,
    DlSym = 44,
    DlClose = 45,
    FfiCall = 46,
    FfiCallbackAlloc = 47,
    FfiCallbackFree = 48,
    TrapFault = 49,
    TrapSeam = 50,
}

pub const HOST_OP_COUNT: usize = 51;
const _: () = assert!(HostOp::TrapSeam as usize + 1 == HOST_OP_COUNT);

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct OsSlice { pub ptr: u64, pub len: u64 }

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct OsPollFd { pub fd: i64, pub events: u32, pub revents: u32 }

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct OsStat {
    pub kind: u64, pub size: u64, pub mode: u64,
    pub mtime_ns: i64, pub atime_ns: i64, pub ctime_ns: i64,
    pub dev: u64, pub ino: u64, pub nlink: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct OsProcSpec {
    pub prog_ptr: u64, pub prog_len: u64, pub argv_ptr: u64, pub argc: u64,
    pub envp_ptr: u64, pub envc: u64, pub cwd_ptr: u64, pub cwd_len: u64,
    pub stdin_fd: i64, pub stdout_fd: i64, pub stderr_fd: i64, pub env_mode: u64,
}

#[repr(C, align(8))]
#[derive(Copy, Clone)]
pub struct OsSockAddr {
    pub family: u32, pub port: u16, pub addr_len: u16,
    pub addr_bytes: [u8; 128], pub flowinfo: u32, pub scope_id: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct OsFfiAbiDesc {
    pub arch_kind: u32, pub gpr_count: u16, pub vec_count: u16, pub stack_bytes: u32,
    pub variadic_fixed_args: u16, pub flags: u16, pub sret_ptr: u64, pub _reserved: u64,
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Default)]
pub struct OsFfiRetRegs { pub gpr: [u64; 2], pub vec: [[u8; 16]; 4] }

const _: () = {
    assert!(size_of::<OsSlice>() == 16 && align_of::<OsSlice>() == 8 && offset_of!(OsSlice, len) == 8);
    assert!(size_of::<OsPollFd>() == 16 && align_of::<OsPollFd>() == 8 && offset_of!(OsPollFd, revents) == 12);
    assert!(size_of::<OsStat>() == 72 && align_of::<OsStat>() == 8 && offset_of!(OsStat, nlink) == 64);
    assert!(size_of::<OsProcSpec>() == 96 && align_of::<OsProcSpec>() == 8 && offset_of!(OsProcSpec, env_mode) == 88);
    assert!(size_of::<OsSockAddr>() == 144 && align_of::<OsSockAddr>() == 8 && offset_of!(OsSockAddr, scope_id) == 140);
    assert!(size_of::<OsFfiAbiDesc>() == 32 && align_of::<OsFfiAbiDesc>() == 8 && offset_of!(OsFfiAbiDesc, _reserved) == 24);
    assert!(size_of::<OsFfiRetRegs>() == 80 && align_of::<OsFfiRetRegs>() == 16 && offset_of!(OsFfiRetRegs, vec) == 16);
};

thread_local! { static LAST_RAW_OS_ERROR: core::cell::Cell<i64> = const { core::cell::Cell::new(0) }; }
static MONO_START: OnceLock<Instant> = OnceLock::new();

#[cfg(unix)]
extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    fn poll(fds: *mut LibcPollFd, nfds: usize, timeout: c_int) -> c_int;
}

#[cfg(unix)]
#[repr(C)]
struct LibcPollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

enum Desc {
    File(File),
    TcpStream(TcpStream),
    PendingTcp(bool),
}

#[derive(Default)]
pub struct BridgeState {
    closed_stdio: [bool; 3],
    next_id: i64,
    descs: HashMap<i64, Desc>,
    procs: HashMap<i64, Child>,
    libs: HashMap<i64, usize>,
}

impl Drop for BridgeState {
    fn drop(&mut self) {
        #[cfg(unix)]
        for (_, h) in self.libs.drain().filter(|&(_, h)| h != 0) {
            unsafe { let _ = dlclose(h as *mut c_void); }
        }
    }
}

impl BridgeState {
    fn next_handle(&mut self) -> i64 {
        self.next_id += 1;
        self.next_id + 2
    }

    fn alloc_fd(&mut self, d: Desc) -> i64 {
        let fd = self.next_handle();
        self.descs.insert(fd, d);
        fd
    }

    #[cfg(unix)]
    fn raw_fd(&self, fd: i64) -> Option<c_int> {
        use std::os::unix::io::AsRawFd;
        if (0..=2).contains(&fd) {
            return (!self.closed_stdio[fd as usize]).then_some(fd as c_int);
        }
        match self.descs.get(&fd)? {
            Desc::File(f) => Some(f.as_raw_fd()),
            Desc::TcpStream(s) => Some(s.as_raw_fd()),
            Desc::PendingTcp(_) => None,
        }
    }

    fn stdio_for(&self, spec_fd: i64) -> Result<Stdio, OpFail> {
        match (spec_fd, self.descs.get(&spec_fd)) {
            (-1, _) => Ok(Stdio::inherit()),
            (-2, _) => Ok(Stdio::null()),
            (_, Some(Desc::File(f))) => io_res(f.try_clone()).map(Stdio::from),
            _ => Err(OpFail::Code(-11)),
        }
    }
}

enum OpFail {
    Code(i64),
    Trap(String),
}

fn seam(id: &str, note: &str) -> Result<i64, OpFail> {
    Err(OpFail::Trap(format!("SEAM001: unconnected seam `{id}`: {note}")))
}

fn io_res<T>(r: io::Result<T>) -> Result<T, OpFail> {
    r.map_err(|e| OpFail::Code(map_io_err(&e)))
}

#[rustfmt::skip]
const LINUX_ERRNO_MAP: &[(i32, i64)] = &[
    (1, 1), (2, 3), (3, 28), (4, 5), (5, 6), (7, 40), (9, 11), (10, 29),
    (11, 7), (12, 10), (13, 2), (16, 16), (17, 4), (18, 17), (20, 14), (21, 13),
    (22, 12), (23, 22), (24, 21), (27, 25), (28, 18), (29, 26), (30, 19), (32, 27),
    (34, 44), (35, 48), (36, 23), (38, 43), (39, 15), (40, 24), (84, 49), (90, 40),
    (95, 42), (97, 41), (98, 36), (99, 37), (101, 38), (103, 33), (104, 32), (106, 35),
    (107, 34), (110, 30), (111, 31), (113, 39), (114, 9), (115, 8), (122, 20), (125, 50),
];

fn map_io_err(err: &io::Error) -> i64 {
    if let Some(raw) = err.raw_os_error() {
        LAST_RAW_OS_ERROR.with(|c| c.set(raw as i64));
        if let Some(&(_, code)) = LINUX_ERRNO_MAP.iter().find(|&&(r, _)| r == raw) {
            return -code;
        }
    }
    match err.kind() {
        ErrorKind::PermissionDenied => -2,
        ErrorKind::NotFound => -3,
        ErrorKind::AlreadyExists => -4,
        ErrorKind::WouldBlock => -7,
        ErrorKind::InvalidInput | ErrorKind::InvalidData => -12,
        ErrorKind::TimedOut => -30,
        ErrorKind::ConnectionRefused => -31,
        ErrorKind::Unsupported => -42,
        _ => -51,
    }
}

unsafe fn raw_buf<'a>(ptr: i64, len: i64) -> Result<&'a [u8], OpFail> {
    (len >= 0 && (len == 0 || ptr != 0))
        .then(|| if len == 0 { &[][..] } else { core::slice::from_raw_parts(ptr as *const u8, len as usize) })
        .ok_or(OpFail::Code(-12))
}

unsafe fn raw_buf_mut<'a>(ptr: i64, len: i64) -> Result<&'a mut [u8], OpFail> {
    (len >= 0 && (len == 0 || ptr != 0))
        .then(|| if len == 0 { &mut [][..] } else { core::slice::from_raw_parts_mut(ptr as *mut u8, len as usize) })
        .ok_or(OpFail::Code(-12))
}

unsafe fn str_from_raw<'a>(ptr: i64, len: i64) -> Result<&'a str, OpFail> {
    core::str::from_utf8(raw_buf(ptr, len)?).map_err(|_| OpFail::Code(-49))
}

unsafe fn write_var_out(bytes: &[u8], dst: i64, cap: i64, out_len_ptr: i64, ok_rc: i64) -> Result<i64, OpFail> {
    if out_len_ptr != 0 {
        (out_len_ptr as *mut u64).write_unaligned(bytes.len() as u64);
    }
    if cap < bytes.len() as i64 {
        return Err(OpFail::Code(-44));
    }
    if !bytes.is_empty() && dst == 0 {
        return Err(OpFail::Code(-12));
    }
    if !bytes.is_empty() {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst as *mut u8, bytes.len());
    }
    Ok(ok_rc)
}

#[cfg(unix)]
fn fill_stat(meta: &fs::Metadata) -> OsStat {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let ft = meta.file_type();
    let kind = match () {
        _ if ft.is_symlink() => 3,
        _ if ft.is_file() => 1,
        _ if ft.is_dir() => 2,
        _ if ft.is_fifo() => 4,
        _ if ft.is_socket() => 5,
        _ if ft.is_block_device() || ft.is_char_device() => 6,
        _ => 0,
    };
    OsStat {
        kind,
        size: meta.len(),
        mode: meta.mode() as u64,
        mtime_ns: meta.mtime().saturating_mul(1_000_000_000).saturating_add(meta.mtime_nsec()),
        atime_ns: meta.atime().saturating_mul(1_000_000_000).saturating_add(meta.atime_nsec()),
        ctime_ns: meta.ctime().saturating_mul(1_000_000_000).saturating_add(meta.ctime_nsec()),
        dev: meta.dev(),
        ino: meta.ino(),
        nlink: meta.nlink(),
    }
}

fn sock_to_os(sa: SocketAddr) -> OsSockAddr {
    let mut out = OsSockAddr { family: 0, port: sa.port(), addr_len: 0, addr_bytes: [0u8; 128], flowinfo: 0, scope_id: 0 };
    match sa {
        SocketAddr::V4(v4) => {
            out.family = 4;
            out.addr_len = 4;
            out.addr_bytes[..4].copy_from_slice(&v4.ip().octets());
        }
        SocketAddr::V6(v6) => {
            out.family = 6;
            out.addr_len = 16;
            out.addr_bytes[..16].copy_from_slice(&v6.ip().octets());
            out.flowinfo = v6.flowinfo();
            out.scope_id = v6.scope_id();
        }
    }
    out
}

unsafe fn os_to_sock(ptr: i64) -> Result<SocketAddr, OpFail> {
    let a = (ptr != 0).then(|| (ptr as *const OsSockAddr).read_unaligned()).ok_or(OpFail::Code(-12))?;
    match a.family {
        4 => Ok(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(a.addr_bytes[0], a.addr_bytes[1], a.addr_bytes[2], a.addr_bytes[3]), a.port))),
        6 => Ok(SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::from(<[u8; 16]>::try_from(&a.addr_bytes[..16]).unwrap()), a.port, a.flowinfo, a.scope_id))),
        _ => Err(OpFail::Code(-41)),
    }
}

pub unsafe fn os_random_bytes(dst: *mut u8, len: usize) -> i64 {
    if len == 0 {
        return 0;
    }
    if dst.is_null() {
        return -12;
    }
    File::open("/dev/urandom").and_then(|mut f| f.read_exact(core::slice::from_raw_parts_mut(dst, len))).map(|()| len as i64).unwrap_or_else(|e| map_io_err(&e))
}

#[cfg(unix)]
pub unsafe fn os_dl_open(state: &mut BridgeState, path_ptr: i64, path_len: i64, flags: i64) -> i64 {
    let c_flags = if flags == 0 { 2 | 0x100 } else { flags as c_int };
    let h = if path_len == 0 {
        dlopen(core::ptr::null(), c_flags)
    } else {
        let Some(cs) = str_from_raw(path_ptr, path_len).ok().and_then(|s| CString::new(s).ok()) else { return -12 };
        dlopen(cs.as_ptr(), c_flags)
    };
    if h.is_null() {
        return -3;
    }
    let id = state.next_handle();
    state.libs.insert(id, h as usize);
    id
}

#[cfg(unix)]
pub unsafe fn os_dl_sym(state: &BridgeState, lib: i64, sym_ptr: i64, sym_len: i64, out_fn_ptr: i64) -> i64 {
    if out_fn_ptr == 0 {
        return -12;
    }
    let Some(&h) = state.libs.get(&lib) else { return -11 };
    let Some(cs) = str_from_raw(sym_ptr, sym_len).ok().and_then(|s| CString::new(s).ok()) else { return -12 };
    let p = dlsym(h as *mut c_void, cs.as_ptr());
    if p.is_null() {
        return -3;
    }
    (out_fn_ptr as *mut u64).write_unaligned(p as usize as u64);
    0
}

#[cfg(unix)]
pub unsafe fn os_dl_close(state: &mut BridgeState, lib: i64) -> i64 {
    let Some(h) = state.libs.remove(&lib) else { return -11 };
    let _ = dlclose(h as *mut c_void);
    0
}

#[cfg(all(target_arch = "x86_64", unix))]
#[unsafe(naked)]
unsafe extern "C" fn sysv_ffi_trampoline(fn_ptr: *const (), gpr_buf: *const u64, vec_buf: *const u8, stack_buf: *const u8, stack_len: usize, ret_buf: *mut OsFfiRetRegs) {
    std::arch::naked_asm!(
        "push rbp\nmov rbp, rsp\npush rbx\npush r12\npush r13\npush r14\npush r15\nsub rsp, 8",
        "mov r12, rdi\nmov r13, rsi\nmov r14, rdx\nmov r15, r9\ntest r8, r8\njz 2f",
        "mov rax, r8\nadd rax, 15\nand rax, -16\nsub rsp, rax\nmov rdi, rsp\nmov rsi, rcx\nmov rcx, r8\nrep movsb\n2:",
        "test r14, r14\njz 3f\nmovdqu xmm0, xmmword ptr [r14]\nmovdqu xmm1, xmmword ptr [r14 + 16]",
        "movdqu xmm2, xmmword ptr [r14 + 32]\nmovdqu xmm3, xmmword ptr [r14 + 48]\nmovdqu xmm4, xmmword ptr [r14 + 64]",
        "movdqu xmm5, xmmword ptr [r14 + 80]\nmovdqu xmm6, xmmword ptr [r14 + 96]\nmovdqu xmm7, xmmword ptr [r14 + 112]\n3:",
        "mov rdi, qword ptr [r13]\nmov rsi, qword ptr [r13 + 8]\nmov rdx, qword ptr [r13 + 16]\nmov rcx, qword ptr [r13 + 24]",
        "mov r8, qword ptr [r13 + 32]\nmov r9, qword ptr [r13 + 40]\nmov rax, qword ptr [r13 + 48]\ncall r12",
        "test r15, r15\njz 4f\nmov qword ptr [r15], rax\nmov qword ptr [r15 + 8], rdx",
        "movdqu xmmword ptr [r15 + 16], xmm0\nmovdqu xmmword ptr [r15 + 32], xmm1\n4:",
        "lea rsp, [rbp - 40]\npop r15\npop r14\npop r13\npop r12\npop rbx\npop rbp\nret",
    );
}

pub unsafe fn os_ffi_call(fn_ptr: i64, abi_desc_ptr: i64, gpr_ptr: i64, vec_ptr: i64, stack_ptr: i64, ret_ptr: i64) -> Result<i64, String> {
    let Some(d) = (abi_desc_ptr != 0).then(|| (abi_desc_ptr as *const OsFfiAbiDesc).read_unaligned()) else { return Ok(-12) };
    if d.arch_kind > 6 || (d.flags & !1) != 0 || d._reserved != 0 || d.gpr_count > 8 || d.vec_count > 8 {
        return Ok(-12);
    }
    #[cfg(all(target_arch = "x86_64", unix))]
    {
        if d.arch_kind > 1 {
            return Ok(-43);
        }
        if fn_ptr == 0 {
            return Ok(-12);
        }
        let mut gprs = [0u64; 8];
        let sret = (d.flags & 1) != 0 && d.sret_ptr != 0;
        if gpr_ptr != 0 {
            let src = core::slice::from_raw_parts(gpr_ptr as *const u64, d.gpr_count as usize);
            if sret && src.first().copied() != Some(d.sret_ptr) {
                gprs[0] = d.sret_ptr;
                gprs[1..1 + src.len().min(5)].copy_from_slice(&src[..src.len().min(5)]);
            } else {
                gprs[..src.len()].copy_from_slice(src);
            }
        } else if sret {
            gprs[0] = d.sret_ptr;
        }
        if d.variadic_fixed_args != 0xFFFF {
            gprs[6] = d.vec_count as u64;
        }
        let mut vecs = [[0u8; 16]; 8];
        let v_ptr = if vec_ptr != 0 && d.vec_count > 0 {
            core::ptr::copy_nonoverlapping(vec_ptr as *const [u8; 16], vecs.as_mut_ptr(), d.vec_count as usize);
            vecs.as_ptr() as *const u8
        } else {
            core::ptr::null()
        };
        let mut local_ret = OsFfiRetRegs::default();
        let r_ptr = if ret_ptr != 0 { ret_ptr as *mut OsFfiRetRegs } else { &mut local_ret };
        let s_ptr = if d.stack_bytes > 0 { stack_ptr as *const u8 } else { core::ptr::null() };
        sysv_ffi_trampoline(fn_ptr as usize as *const (), gprs.as_ptr(), v_ptr, s_ptr, d.stack_bytes as usize, r_ptr);
        Ok(0)
    }
    #[cfg(not(all(target_arch = "x86_64", unix)))]
    {
        let _ = (fn_ptr, gpr_ptr, vec_ptr, stack_ptr, ret_ptr);
        if d.arch_kind != 0 {
            return Ok(-43);
        }
        Err("SEAM001: unconnected seam `bridge.ffi_call_target`: raw register FFI trampoline is not yet implemented for this host".into())
    }
}

fn slab_layout(size: i64, align: i64) -> Option<std::alloc::Layout> {
    let sz = usize::try_from(size).ok().filter(|&n| n > 0)?;
    std::alloc::Layout::from_size_align(sz, usize::try_from(align.max(1)).ok()?.checked_next_power_of_two()?).ok()
}

#[cfg(unix)]
unsafe fn poll_fds(state: &BridgeState, p_ptr: i64, nfds: i64, timeout_ms: i64) -> Result<i64, OpFail> {
    if nfds < 0 || (nfds > 0 && p_ptr == 0) {
        return Err(OpFail::Code(-12));
    }
    if nfds == 0 {
        if timeout_ms > 0 {
            std::thread::sleep(Duration::from_millis(timeout_ms as u64));
        }
        return Ok(0);
    }
    let items = core::slice::from_raw_parts_mut(p_ptr as *mut OsPollFd, nfds as usize);
    let mut raw: Vec<LibcPollFd> = items
        .iter()
        .map(|it| LibcPollFd {
            fd: state.raw_fd(it.fd).unwrap_or(-1),
            events: ((if it.events & 1 != 0 { 1 } else { 0 }) | (if it.events & 2 != 0 { 4 } else { 0 })) as i16,
            revents: 0,
        })
        .collect();
    let r = poll(raw.as_mut_ptr(), raw.len(), timeout_ms as c_int);
    if r < 0 {
        return Err(OpFail::Code(map_io_err(&io::Error::last_os_error())));
    }
    let mut ready = 0i64;
    for (it, rf) in items.iter_mut().zip(raw.iter()) {
        if rf.fd < 0 && it.fd >= 0 {
            it.revents = 16;
            ready += 1;
        } else {
            it.revents = [(1, 1), (4, 2), (8, 4), (16, 8), (32, 16)].into_iter().fold(0u32, |acc, (m, b)| acc | if rf.revents & m != 0 { b } else { 0 });
            ready += i64::from(it.revents != 0 && r == 0);
        }
    }
    Ok(if r > 0 { r as i64 } else { ready })
}

unsafe fn spawn_proc(state: &mut BridgeState, spec_ptr: i64, out_pid_ptr: i64) -> Result<i64, OpFail> {
    let sp = (spec_ptr != 0).then(|| (spec_ptr as *const OsProcSpec).read_unaligned()).ok_or(OpFail::Code(-12))?;
    let mut cmd = Command::new(str_from_raw(sp.prog_ptr as i64, sp.prog_len as i64)?);
    if sp.argc > 0 && sp.argv_ptr != 0 {
        for sl in core::slice::from_raw_parts(sp.argv_ptr as *const OsSlice, sp.argc as usize) {
            cmd.arg(str_from_raw(sl.ptr as i64, sl.len as i64)?);
        }
    }
    if sp.env_mode == 1 || sp.env_mode == 2 {
        cmd.env_clear();
    }
    if sp.env_mode != 2 && sp.envc > 0 && sp.envp_ptr != 0 {
        for sl in core::slice::from_raw_parts(sp.envp_ptr as *const OsSlice, sp.envc as usize) {
            if let Some((k, v)) = str_from_raw(sl.ptr as i64, sl.len as i64)?.split_once('=') {
                cmd.env(k, v);
            }
        }
    }
    if sp.cwd_len > 0 {
        cmd.current_dir(Path::new(str_from_raw(sp.cwd_ptr as i64, sp.cwd_len as i64)?));
    }
    let child = io_res(cmd.stdin(state.stdio_for(sp.stdin_fd)?).stdout(state.stdio_for(sp.stdout_fd)?).stderr(state.stdio_for(sp.stderr_fd)?).spawn())?;
    if out_pid_ptr != 0 {
        (out_pid_ptr as *mut i64).write_unaligned(child.id() as i64);
    }
    let h = state.next_handle();
    state.procs.insert(h, child);
    Ok(h)
}

unsafe fn dispatch_op(vm: &mut PilVirtualMachine, op_id: i64, args: &[PilotVal]) -> Result<i64, OpFail> {
    let i_at = |idx: usize| args.get(idx + 1).and_then(|v| v.as_int()).unwrap_or(0);
    let f_at = |idx: usize| args.get(idx + 1).and_then(|v| v.as_float().or_else(|| v.as_int().map(|n| n as f64))).unwrap_or(0.0);
    match op_id {
        0 => Ok(slab_layout(i_at(0), i_at(1)).map_or(0, |l| std::alloc::alloc(l) as usize as i64)),
        1 if i_at(0) == 0 => Ok(slab_layout(i_at(2), i_at(3)).map_or(0, |l| std::alloc::alloc(l) as usize as i64)),
        1 if i_at(2) <= 0 => {
            if let Some(l) = slab_layout(i_at(1), i_at(3)) {
                std::alloc::dealloc(i_at(0) as *mut u8, l);
            }
            Ok(0)
        }
        1 => Ok(slab_layout(i_at(1), i_at(3)).zip(usize::try_from(i_at(2)).ok()).map_or(0, |(ol, nsz)| std::alloc::realloc(i_at(0) as *mut u8, ol, nsz) as usize as i64)),
        2 => {
            if let Some(l) = (i_at(0) != 0).then(|| slab_layout(i_at(1), i_at(2))).flatten() {
                std::alloc::dealloc(i_at(0) as *mut u8, l);
            }
            Ok(0)
        }
        3 => seam("bridge.mem_map", "virtual memory mapping ops are not yet implemented"),
        4 => {
            let len = i_at(2).max(0) as usize;
            if len > 0 && i_at(0) != 0 && i_at(1) != 0 {
                core::ptr::copy(i_at(1) as *const u8, i_at(0) as *mut u8, len);
            }
            Ok(len as i64)
        }
        5 => {
            let len = i_at(2).max(0) as usize;
            if len > 0 && i_at(0) != 0 {
                core::ptr::write_bytes(i_at(0) as *mut u8, i_at(1) as u8, len);
            }
            Ok(len as i64)
        }
        6 => {
            let len = i_at(2).max(0) as usize;
            Ok(if len == 0 || i_at(0) == i_at(1) { 0 } else { core::slice::from_raw_parts(i_at(0) as *const u8, len).cmp(core::slice::from_raw_parts(i_at(1) as *const u8, len)) as i8 as i64 })
        }
        7 => Ok(match (i_at(1), i_at(2) != 0) {
            (8, true) => (i_at(0) as *const i8).read_unaligned() as i64,
            (8, false) => (i_at(0) as *const u8).read_unaligned() as i64,
            (16, true) => (i_at(0) as *const i16).read_unaligned() as i64,
            (16, false) => (i_at(0) as *const u16).read_unaligned() as i64,
            (32, true) => (i_at(0) as *const i32).read_unaligned() as i64,
            (32, false) => (i_at(0) as *const u32).read_unaligned() as i64,
            _ => (i_at(0) as *const i64).read_unaligned(),
        }),
        8 => {
            match i_at(2) {
                8 => (i_at(0) as *mut u8).write_unaligned(i_at(1) as u8),
                16 => (i_at(0) as *mut u16).write_unaligned(i_at(1) as u16),
                32 => (i_at(0) as *mut u32).write_unaligned(i_at(1) as u32),
                _ => (i_at(0) as *mut u64).write_unaligned(i_at(1) as u64),
            }
            Ok(0)
        }
        10 => {
            if i_at(2) == 32 {
                (i_at(0) as *mut f32).write_unaligned(f_at(1) as f32);
            } else {
                (i_at(0) as *mut f64).write_unaligned(f_at(1));
            }
            Ok(0)
        }
        11 => {
            let atom = &*(i_at(1) as *const AtomicI64);
            match i_at(0) {
                0 => Ok(atom.load(Ordering::SeqCst)),
                1 => {
                    atom.store(i_at(3), Ordering::SeqCst);
                    Ok(0)
                }
                3 => Ok(atom.fetch_add(i_at(3), Ordering::SeqCst)),
                9 => Ok(i64::from(atom.compare_exchange(i_at(2), i_at(3), Ordering::SeqCst, Ordering::Relaxed).is_ok())),
                _ => seam("bridge.atomic_int", "extended atomic sub-ops are not yet implemented"),
            }
        }
        12 => seam("bridge.thread_spawn", "OS thread spawn is not yet implemented"),
        13 => seam("bridge.thread_join", "OS thread join is not yet implemented"),
        14 => Ok(match i_at(0) {
            0 => 1,
            1 => {
                std::thread::yield_now();
                0
            }
            2 => std::thread::available_parallelism().map_or(1, |n| n.get() as i64),
            3 => -11,
            4 => LAST_RAW_OS_ERROR.with(|c| c.get()),
            5 => -42,
            _ => -12,
        }),
        15 => seam("bridge.futex", "futex wait/wake is not yet implemented"),
        #[cfg(unix)]
        16 => {
            use std::os::unix::fs::OpenOptionsExt;
            let (p, fl) = (str_from_raw(i_at(0), i_at(1))?, i_at(2));
            let mut oo = OpenOptions::new();
            oo.read((fl & 1) != 0).write((fl & 18) != 0).create((fl & 36) == 4).create_new((fl & 36) == 36).truncate((fl & 8) != 0).append((fl & 16) != 0);
            if i_at(3) > 0 {
                oo.mode(i_at(3) as u32);
            }
            if (fl & 64) != 0 {
                oo.custom_flags(0o4000);
            }
            Ok(vm.bridge.alloc_fd(Desc::File(io_res(oo.open(p))?)))
        }
        17 => {
            let fd = i_at(0);
            if (0..=2).contains(&fd) && !vm.bridge.closed_stdio[fd as usize] {
                vm.bridge.closed_stdio[fd as usize] = true;
                Ok(0)
            } else {
                vm.bridge.descs.remove(&fd).map(|_| 0).ok_or(OpFail::Code(-11))
            }
        }
        18 => {
            let (fd, buf) = (i_at(0), raw_buf_mut(i_at(1), i_at(2))?);
            if buf.is_empty() {
                return Ok(0);
            }
            match (fd, vm.bridge.descs.get_mut(&fd)) {
                (0, _) if !vm.bridge.closed_stdio[0] => Ok(io_res(io::stdin().read(buf))? as i64),
                (_, Some(Desc::File(f))) => Ok(io_res(f.read(buf))? as i64),
                (_, Some(Desc::TcpStream(s))) => Ok(io_res(s.read(buf))? as i64),
                _ => Err(OpFail::Code(-11)),
            }
        }
        19 => {
            let (fd, buf) = (i_at(0), raw_buf(i_at(1), i_at(2))?);
            if buf.is_empty() {
                return Ok(0);
            }
            match (fd, vm.bridge.descs.get_mut(&fd)) {
                (1, _) if !vm.bridge.closed_stdio[1] => Ok(io_res(io::stdout().write(buf))? as i64),
                (2, _) if !vm.bridge.closed_stdio[2] => Ok(io_res(io::stderr().write(buf))? as i64),
                (_, Some(Desc::File(f))) => Ok(io_res(f.write(buf))? as i64),
                (_, Some(Desc::TcpStream(s))) => Ok(io_res(s.write(buf))? as i64),
                _ => Err(OpFail::Code(-11)),
            }
        }
        #[cfg(unix)]
        20 => {
            use std::os::unix::fs::FileExt;
            let buf = raw_buf_mut(i_at(1), i_at(2))?;
            let off = u64::try_from(i_at(3)).map_err(|_| OpFail::Code(-12))?;
            match vm.bridge.descs.get(&i_at(0)) {
                Some(Desc::File(f)) => Ok(io_res(f.read_at(buf, off))? as i64),
                _ => Err(OpFail::Code(-11)),
            }
        }
        21 => seam("bridge.fd_pwrite", "positional descriptor write is not yet implemented"),
        22 if i_at(0) == 0 => match vm.bridge.descs.get_mut(&i_at(1)) {
            Some(Desc::File(f)) => Ok(io_res(f.seek(SeekFrom::Start(i_at(2).max(0) as u64)))? as i64),
            _ => Err(OpFail::Code(-11)),
        },
        22 => seam("bridge.fd_seek_sync", "non-seek_set/sync/fstat/flock sub-ops are not yet implemented"),
        #[cfg(unix)]
        23 if i_at(0) == 0 => {
            use std::os::unix::io::FromRawFd;
            let mut raw = [0 as c_int; 2];
            if i_at(1) == 0 {
                return Err(OpFail::Code(-12));
            }
            if pipe(raw.as_mut_ptr()) != 0 {
                return Err(OpFail::Code(map_io_err(&io::Error::last_os_error())));
            }
            // Close-on-exec (F_SETFD = 2, FD_CLOEXEC = 1 on Linux and macOS):
            // a spawned child gets only the ends passed as its stdio, so it
            // sees end-of-input once the parent closes its write end.
            for fd in raw {
                if fcntl(fd, 2, 1) != 0 {
                    return Err(OpFail::Code(map_io_err(&io::Error::last_os_error())));
                }
            }
            (i_at(1) as *mut [i64; 2]).write_unaligned([vm.bridge.alloc_fd(Desc::File(File::from_raw_fd(raw[0]))), vm.bridge.alloc_fd(Desc::File(File::from_raw_fd(raw[1])))]);
            Ok(0)
        }
        23 => seam("bridge.fd_dup", "descriptor dup/dup2 sub-ops are not yet implemented"),
        #[cfg(unix)]
        24 => poll_fds(&vm.bridge, i_at(0), i_at(1), i_at(2)),
        25 => seam("bridge.fd_raw_ctl", "raw OS handle adoption/extraction is not yet implemented"),
        #[cfg(unix)]
        26 => {
            let (p, out) = (str_from_raw(i_at(1), i_at(2))?, (i_at(3) != 0).then_some(i_at(3) as *mut OsStat).ok_or(OpFail::Code(-12))?);
            out.write_unaligned(fill_stat(&io_res(if i_at(0) != 0 { fs::metadata(p) } else { fs::symlink_metadata(p) })?));
            Ok(0)
        }
        27 => {
            let (mut packed, mut count) = (Vec::new(), 0i64);
            for ent in io_res(fs::read_dir(str_from_raw(i_at(0), i_at(1))?))? {
                let ent = io_res(ent)?;
                let name = ent.file_name().to_string_lossy().into_owned();
                let nb = name.as_bytes();
                let nlen = nb.len().min(u16::MAX as usize);
                packed.push(ent.file_type().map_or(0u8, |ft| if ft.is_symlink() { 3 } else if ft.is_file() { 1 } else if ft.is_dir() { 2 } else { 0 }));
                packed.extend_from_slice(&(nlen as u16).to_le_bytes());
                packed.extend_from_slice(&nb[..nlen]);
                count += 1;
            }
            write_var_out(&packed, i_at(2), i_at(3), i_at(4), count)
        }
        28 => io_res(if i_at(2) != 0 { fs::create_dir_all(str_from_raw(i_at(0), i_at(1))?) } else { fs::create_dir(str_from_raw(i_at(0), i_at(1))?) }).map(|()| 0),
        29 => {
            let p = str_from_raw(i_at(0), i_at(1))?;
            let res = if io_res(fs::symlink_metadata(p))?.file_type().is_dir() {
                if i_at(2) != 0 { fs::remove_dir_all(p) } else { fs::remove_dir(p) }
            } else {
                fs::remove_file(p)
            };
            io_res(res).map(|()| 0)
        }
        30 if i_at(0) == 0 => io_res(fs::rename(str_from_raw(i_at(1), i_at(2))?, str_from_raw(i_at(3), i_at(4))?)).map(|()| 0),
        30 => seam("bridge.fs_rename_link", "hard_link/symlink/copy_file/readlink sub-ops are not yet implemented"),
        31 => seam("bridge.fs_path_ctl", "getcwd/chdir/realpath/chmod/utimens sub-ops are not yet implemented"),
        32 => spawn_proc(&mut vm.bridge, i_at(0), i_at(1)),
        33 => {
            let (h, nonblock, out_code) = (i_at(0), i_at(1) != 0, i_at(2));
            let st = if nonblock {
                let Some(st) = io_res(vm.bridge.procs.get_mut(&h).ok_or(OpFail::Code(-11))?.try_wait())? else { return Ok(0) };
                let _ = vm.bridge.procs.remove(&h);
                st
            } else {
                io_res(vm.bridge.procs.remove(&h).ok_or(OpFail::Code(-11))?.wait())?
            };
            if out_code != 0 {
                (out_code as *mut i64).write_unaligned(st.code().map_or(-1, i64::from));
            }
            Ok(1)
        }
        34 => match i_at(0) {
            2 => Ok(std::process::id() as i64),
            3 => std::process::exit(i_at(2) as i32),
            _ => seam("bridge.proc_signal", "signal_handle/signal_pid sub-ops are not yet implemented"),
        },
        35 => match i_at(0) {
            0 => std::env::var(str_from_raw(i_at(1), i_at(2))?).map_err(|_| OpFail::Code(-3)).and_then(|v| write_var_out(v.as_bytes(), i_at(3), i_at(4), i_at(5), 0)),
            2 => {
                let mut packed = Vec::new();
                for a in &vm.script_args {
                    packed.extend_from_slice(&(a.len() as u32).to_le_bytes());
                    packed.extend_from_slice(a.as_bytes());
                }
                write_var_out(&packed, i_at(3), i_at(4), i_at(5), vm.script_args.len() as i64)
            }
            3 => write_var_out(io_res(std::env::current_exe())?.to_string_lossy().replace('\\', "/").as_bytes(), i_at(3), i_at(4), i_at(5), 0),
            _ => seam("bridge.env_ctl", "environ/setenv/unsetenv sub-ops are not yet implemented"),
        },
        36 => match i_at(0) {
            0 => Ok(SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos() as i64)),
            1 | 2 | 3 => Ok(MONO_START.get_or_init(Instant::now).elapsed().as_nanos() as i64 + 1),
            _ => Err(OpFail::Code(-12)),
        },
        37 => {
            if i_at(0) > 0 {
                std::thread::sleep(Duration::from_nanos(i_at(0) as u64));
            }
            Ok(0)
        }
        38 => Ok(os_random_bytes(i_at(0) as *mut u8, i_at(1).max(0) as usize)),
        39 if i_at(0) != 4 && i_at(0) != 6 => Err(OpFail::Code(-41)),
        39 if i_at(1) != 1 => seam("bridge.net_dgram", "non-stream sockets are not yet implemented"),
        39 => Ok(vm.bridge.alloc_fd(Desc::PendingTcp((i_at(3) & 64) != 0))),
        40 if i_at(0) != 0 => seam("bridge.net_server", "server/shutdown socket sub-ops are not yet implemented"),
        40 => {
            let sa = os_to_sock(i_at(2))?;
            let Some(&Desc::PendingTcp(nb)) = vm.bridge.descs.get(&i_at(1)) else { return Err(OpFail::Code(-11)) };
            let s = io_res(TcpStream::connect(sa))?;
            let _ = s.set_nonblocking(nb);
            vm.bridge.descs.insert(i_at(1), Desc::TcpStream(s));
            Ok(0)
        }
        41 => {
            let Some(Desc::TcpStream(s)) = vm.bridge.descs.get_mut(&i_at(1)) else { return Err(OpFail::Code(-11)) };
            match i_at(0) {
                0 => Ok(io_res(s.read(raw_buf_mut(i_at(2), i_at(3))?))? as i64),
                1 => Ok(io_res(s.write(raw_buf(i_at(2), i_at(3))?))? as i64),
                _ => seam("bridge.net_io", "advanced socket I/O sub-ops are not yet implemented"),
            }
        }
        42 => {
            let addrs: Vec<OsSockAddr> = (str_from_raw(i_at(0), i_at(1))?, i_at(2) as u16).to_socket_addrs().map_err(|_| OpFail::Code(-45))?.map(sock_to_os).collect();
            write_var_out(core::slice::from_raw_parts(addrs.as_ptr() as *const u8, addrs.len() * size_of::<OsSockAddr>()), i_at(3), i_at(4), i_at(5), addrs.len() as i64)
        }
        #[cfg(unix)]
        43 => Ok(os_dl_open(&mut vm.bridge, i_at(0), i_at(1), i_at(2))),
        #[cfg(unix)]
        44 => Ok(os_dl_sym(&vm.bridge, i_at(0), i_at(1), i_at(2), i_at(3))),
        #[cfg(unix)]
        45 => Ok(os_dl_close(&mut vm.bridge, i_at(0))),
        46 => os_ffi_call(i_at(0), i_at(1), i_at(2), i_at(3), i_at(4), i_at(5)).map_err(OpFail::Trap),
        47 => seam("bridge.ffi_callback_alloc", "C-callable callback trampolines are not yet implemented"),
        48 => seam("bridge.ffi_callback_free", "C-callable callback trampolines are not yet implemented"),
        49 => Err(OpFail::Trap(str_from_raw(i_at(0), i_at(1)).unwrap_or("host_op fault").to_string())),
        50 => seam(str_from_raw(i_at(0), i_at(1)).unwrap_or("bridge.unknown"), str_from_raw(i_at(2), i_at(3)).unwrap_or("")),
        _ => Err(OpFail::Code(-43)),
    }
}

pub fn dispatch(vm: &mut PilVirtualMachine, args: &[PilotVal]) -> Result<PilotVal, String> {
    let op_id = args.first().and_then(|v| v.as_int()).ok_or_else(|| "`host_op` expects an integer op ID".to_string())?;
    if op_id == 9 {
        let (p, bits) = (args.get(1).and_then(|v| v.as_int()).unwrap_or(0), args.get(2).and_then(|v| v.as_int()).unwrap_or(64));
        let f = unsafe { if bits == 32 { (p as *const f32).read_unaligned() as f64 } else { (p as *const f64).read_unaligned() } };
        return Ok(PilotVal::from_float(f));
    }
    match unsafe { dispatch_op(vm, op_id, args) } {
        Ok(v) | Err(OpFail::Code(v)) => Ok(PilotVal::from_int(v)),
        Err(OpFail::Trap(msg)) => Err(msg),
    }
}
