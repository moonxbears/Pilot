//! What the VM shares with its host: the arguments a script was started
//! with, the module headers the loader reads, and access to the values a
//! program hands back.
//!
//! - `proc_args()` is the list of arguments after `--` on the command line.
//! - `unit_header(path)` lists the `pkg` and `use` declarations at the head
//!   of a source file, as parsed by the seed parser: one
//!   `[is_package, path, alias, offset]` list per declaration, or `none` when
//!   the file cannot be read.

use crate::vm::PilVirtualMachine;
use pilot_rt::PilotVal;

impl PilVirtualMachine {
    /// Sets the arguments `proc_args()` returns.
    pub fn set_script_args(&mut self, args: Vec<String>) {
        self.script_args = args;
    }

    pub(crate) fn proc_args(&mut self) -> PilotVal {
        let items = self.script_args.iter().map(|a| PilotVal::from_str(a)).collect();
        self.new_list(items)
    }

    pub(crate) fn unit_header(&mut self, path: &str) -> PilotVal {
        let Ok(src) = std::fs::read_to_string(path) else {
            return PilotVal::NULL;
        };
        let decls: Vec<PilotVal> = pilot_parser::parse_header(&src)
            .into_iter()
            .map(|d| {
                let fields = vec![
                    PilotVal::from_bool(d.is_package),
                    PilotVal::from_str(&d.path),
                    PilotVal::from_str(&d.alias),
                    PilotVal::from_int(d.offset as i64),
                ];
                self.new_list(fields)
            })
            .collect();
        self.new_list(decls)
    }

    /// The elements of a list value.
    pub fn list_items(&self, v: PilotVal) -> Option<Vec<PilotVal>> {
        let slot = v.as_list_handle()?;
        self.heap_lists.get(slot).cloned()
    }

    /// Field `name` of a record value.
    pub fn record_field(&self, v: PilotVal, name: &str) -> Option<PilotVal> {
        let slot = v.as_map_handle()?;
        self.heap_maps.get(slot)?.get(name).copied()
    }
}

/// A value passed between the VM and its host: the shapes host operations
/// take and return. Lists may nest.
#[derive(Clone, Debug, PartialEq)]
pub enum HostValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<HostValue>),
}

impl HostValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            HostValue::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            HostValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[HostValue]> {
        match self {
            HostValue::List(l) => Some(l),
            _ => None,
        }
    }
}

impl PilVirtualMachine {
    /// The host's value for a VM value. Records and functions have no host
    /// shape and cross as `Null`.
    pub fn to_host(&self, v: PilotVal) -> HostValue {
        if let Some(items) = self.list_items(v) {
            return HostValue::List(items.into_iter().map(|i| self.to_host(i)).collect());
        }
        if let Some(s) = v.with_str(str::to_string) {
            return HostValue::Str(s);
        }
        if let Some(b) = v.as_bool() {
            return HostValue::Bool(b);
        }
        if let Some(i) = v.as_int() {
            return HostValue::Int(i);
        }
        if let Some(f) = v.as_float() {
            return HostValue::Float(f);
        }
        HostValue::Null
    }

    /// The VM value for a host value.
    pub fn from_host(&mut self, v: HostValue) -> PilotVal {
        match v {
            HostValue::Null => PilotVal::NULL,
            HostValue::Bool(b) => PilotVal::from_bool(b),
            HostValue::Int(i) => PilotVal::from_int(i),
            HostValue::Float(f) => PilotVal::from_float(f),
            HostValue::Str(s) => PilotVal::from_str(&s),
            HostValue::List(items) => {
                let vals = items.into_iter().map(|i| self.from_host(i)).collect();
                self.new_list(vals)
            }
        }
    }

    /// Runs `op` as a host operation, if the host provides it.
    pub(crate) fn host_op(&mut self, op: &str, args: &[PilotVal]) -> Option<Result<PilotVal, String>> {
        let mut ops = self.host_ops.take()?;
        let host_args: Vec<HostValue> = args.iter().map(|&a| self.to_host(a)).collect();
        let out = ops(op, &host_args);
        self.host_ops = Some(ops);
        Some(out?.map(|v| self.from_host(v)))
    }

    /// The strings of a list value.
    pub(crate) fn str_list(&self, v: PilotVal, op: &str) -> Result<Vec<String>, String> {
        let items = self
            .list_items(v)
            .ok_or_else(|| format!("`{op}` expects a list of strings"))?;
        Ok(items.iter().map(|&i| self.display(i)).collect())
    }

    /// Waits for `child` and returns `[exit_code, stdout, stderr]`. One
    /// killed by a signal exits with `-1`.
    pub(crate) fn finish_child(&mut self, child: Child) -> Result<PilotVal, String> {
        let (program, done) = child;
        let out = done
            .join()
            .map_err(|_| format!("the thread waiting for `{program}` failed"))?
            .map_err(|e| format!("cannot wait for `{program}`: {e}"))?;
        let fields = vec![
            PilotVal::from_int(out.status.code().map_or(-1, i64::from)),
            PilotVal::from_str(&String::from_utf8_lossy(&out.stdout)),
            PilotVal::from_str(&String::from_utf8_lossy(&out.stderr)),
        ];
        Ok(self.new_list(fields))
    }
}

/// A started child process: its program, and the thread that feeds its
/// standard input and collects its output.
pub type Child = (String, std::thread::JoinHandle<std::io::Result<std::process::Output>>);

/// Starts `program` with `argv` in working directory `dir` (the caller's when
/// absent), feeding it `input` on standard input. A program that cannot be
/// started is an intrinsic failure.
pub(crate) fn spawn(program: &str, argv: &[String], input: String, dir: Option<&str>) -> Result<Child, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut cmd = Command::new(program);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let mut child = cmd
        .args(argv)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start `{program}`: {e}"))?;
    let stdin = child.stdin.take();
    let done = std::thread::spawn(move || {
        if let Some(mut stdin) = stdin {
            // A program that exits without reading its input closes the
            // pipe; that is not a failure of the run.
            let _ = stdin.write_all(input.as_bytes());
        }
        child.wait_with_output()
    });
    Ok((program.to_string(), done))
}

/// How many processes the machine runs at once.
pub(crate) fn cpu_count() -> i64 {
    std::thread::available_parallelism().map_or(1, |n| n.get() as i64)
}

/// The next line of standard input without its line ending, or `none` at
/// the end of input.
pub(crate) fn stdin_read_line() -> PilotVal {
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => PilotVal::NULL,
        Ok(_) => {
            let trimmed = line.strip_suffix('\n').unwrap_or(&line);
            PilotVal::from_str(trimmed.strip_suffix('\r').unwrap_or(trimmed))
        }
    }
}

/// The rest of standard input.
pub(crate) fn stdin_read_all() -> PilotVal {
    use std::io::Read;
    let mut text = String::new();
    let _ = std::io::stdin().read_to_string(&mut text);
    PilotVal::from_str(&text)
}

/// The path of the running executable, with `/` separators, or `none`.
pub(crate) fn proc_exe() -> PilotVal {
    std::env::current_exe()
        .ok()
        .map_or(PilotVal::NULL, |p| PilotVal::from_str(&p.to_string_lossy().replace('\\', "/")))
}
