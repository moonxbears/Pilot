//! VM heap intrinsics: lists, records, strings, printing and assertions.
//!
//! Lists and records are VM-owned slots addressed by tagged handles
//! (`TAG_LIST` / `TAG_MAP`). Intrinsics fault when they receive a value kind
//! they do not accept. Unknown intrinsic names are forwarded to the host
//! engine handler and then to the OS syscall table.

use crate::vm::PilVirtualMachine;
use pilot_rt::PilotVal;
use std::collections::HashMap;

/// Display nesting limit; deeper containers print as `...`.
const DISPLAY_DEPTH: usize = 8;

type IntrinsicResult = Result<PilotVal, String>;

fn arg(args: &[PilotVal], i: usize, op: &str) -> Result<PilotVal, String> {
    args.get(i)
        .copied()
        .ok_or_else(|| format!("`{op}` expects at least {} argument(s), got {}", i + 1, args.len()))
}

fn int_arg(args: &[PilotVal], i: usize, op: &str) -> Result<i64, String> {
    let v = arg(args, i, op)?;
    v.as_int()
        .ok_or_else(|| format!("`{op}` argument {} must be an integer, found {}", i + 1, v.kind_label()))
}

/// A string argument, read without copying it to the heap: a short string
/// is held inline, a long one is borrowed where it is stored. An intrinsic
/// reads its arguments before it returns, and the VM frees strings only
/// between instructions, so the borrow cannot outlive the bytes.
enum StrArg {
    Inline([u8; 14], usize),
    Stored(*const u8, usize),
}

impl std::ops::Deref for StrArg {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            StrArg::Inline(buf, len) => std::str::from_utf8(&buf[..*len]).unwrap_or(""),
            StrArg::Stored(ptr, len) => {
                let bytes = unsafe { std::slice::from_raw_parts(*ptr, *len) };
                std::str::from_utf8(bytes).unwrap_or("")
            }
        }
    }
}

impl std::fmt::Display for StrArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self)
    }
}

fn str_arg(args: &[PilotVal], i: usize, op: &str) -> Result<StrArg, String> {
    let v = arg(args, i, op)?;
    if let Some((ptr, len)) = v.heap_parts() {
        return Ok(StrArg::Stored(ptr, len));
    }
    let mut buf = [0u8; 14];
    match v.str_in(&mut buf).map(str::len) {
        Some(len) => Ok(StrArg::Inline(buf, len)),
        None => Err(format!("`{op}` argument {} must be a string, found {}", i + 1, v.kind_label())),
    }
}

/// Clamps a byte offset into `s` and moves it forward to a char boundary.
fn char_floor(s: &str, at: i64) -> usize {
    let mut u = (at.max(0) as usize).min(s.len());
    while u < s.len() && !s.is_char_boundary(u) {
        u += 1;
    }
    u
}

/// Record keys are the display form of the key value.
fn key_of(k: PilotVal) -> String {
    k.with_str(|s| s.to_string())
        .unwrap_or_else(|| k.to_string_repr())
}

/// Calls `f` with the key `k` names, borrowing it when `k` is a string so
/// that reading a field allocates nothing.
fn with_key<R>(k: PilotVal, f: impl FnOnce(&str) -> R) -> R {
    let mut buf = [0u8; 14];
    match k.str_in(&mut buf) {
        Some(s) => f(s),
        None => f(&k.to_string_repr()),
    }
}

impl PilVirtualMachine {
    /// Renders a value for printing and string concatenation.
    pub fn display(&self, v: PilotVal) -> String {
        self.display_depth(v, 0)
    }

    fn display_depth(&self, v: PilotVal, depth: usize) -> String {
        if depth > DISPLAY_DEPTH {
            return "...".to_string();
        }
        if let Some(slot) = v.as_list_handle() {
            let Some(items) = self.heap_lists.get(slot) else {
                return v.to_string_repr();
            };
            let parts: Vec<String> = items
                .iter()
                .map(|x| self.display_depth(*x, depth + 1))
                .collect();
            return format!("[{}]", parts.join(", "));
        }
        if let Some(slot) = v.as_map_handle() {
            let Some(m) = self.heap_maps.get(slot) else {
                return v.to_string_repr();
            };
            let mut parts: Vec<String> = m
                .iter()
                .map(|(k, x)| format!("{k}: {}", self.display_depth(*x, depth + 1)))
                .collect();
            parts.sort();
            return format!("{{ {} }}", parts.join(", "));
        }
        v.to_string_repr()
    }

    fn list_slot(&self, v: PilotVal, op: &str) -> Result<usize, String> {
        let slot = v
            .as_list_handle()
            .filter(|s| *s < self.heap_lists.len())
            .ok_or_else(|| format!("`{op}` expects a list, found {}", v.kind_label()))?;
        if self.list_is_dead(slot) {
            return Err(format!("internal error: `{op}` reads a list the collector freed"));
        }
        Ok(slot)
    }

    /// The elements of the list `v`, or with `deep` its leaves at any depth.
    fn collect_elements(&self, v: PilotVal, deep: bool, op: &str, out: &mut Vec<PilotVal>) -> Result<(), String> {
        let slot = self.list_slot(v, op)?;
        for &e in &self.heap_lists[slot] {
            if deep && e.as_list_handle().is_some() {
                self.collect_elements(e, true, op, out)?;
            } else {
                out.push(e);
            }
        }
        Ok(())
    }

    /// A deep copy of `v`: lists and maps (records) are copied with their
    /// contents copied the same way, keeping a record's type; anything else
    /// (strings, scalars, function values) is shared.
    fn value_clone(&mut self, v: PilotVal, depth: usize) -> IntrinsicResult {
        if depth > 4096 {
            return Err("a value nests too deeply to copy".to_string());
        }
        if let Some(slot) = v.as_list_handle().filter(|s| *s < self.heap_lists.len()) {
            let items = self.heap_lists[slot].clone();
            let mut out = Vec::with_capacity(items.len());
            for x in items {
                out.push(self.value_clone(x, depth + 1)?);
            }
            return Ok(self.new_list(out));
        }
        if let Some(slot) = v.as_map_handle().filter(|s| *s < self.heap_maps.len()) {
            let entries: Vec<(String, PilotVal)> = self.heap_maps[slot].iter().map(|(k, x)| (k.clone(), *x)).collect();
            let mut out = HashMap::with_capacity(entries.len());
            for (k, x) in entries {
                out.insert(k, self.value_clone(x, depth + 1)?);
            }
            let copy = self.new_map(out);
            if let Some(ty) = self.record_types.get(&slot).copied() {
                let new_slot = self.map_slot(copy, "value_clone")?;
                self.record_types.insert(new_slot, ty);
            }
            return Ok(copy);
        }
        Ok(v)
    }

    fn map_slot(&self, v: PilotVal, op: &str) -> Result<usize, String> {
        let slot = v
            .as_map_handle()
            .filter(|s| *s < self.heap_maps.len())
            .ok_or_else(|| format!("`{op}` expects a record, found {}", v.kind_label()))?;
        if self.map_is_dead(slot) {
            return Err(format!("internal error: `{op}` reads a record the collector freed"));
        }
        Ok(slot)
    }

    fn list_index(&self, slot: usize, k: PilotVal, op: &str) -> Result<usize, String> {
        let len = self.heap_lists[slot].len();
        let i = k
            .as_int()
            .ok_or_else(|| format!("`{op}` list index must be an integer, found {}", k.kind_label()))?;
        if i < 0 || i as usize >= len {
            return Err(format!("list index {i} out of bounds for length {len}"));
        }
        Ok(i as usize)
    }

    pub fn new_list(&mut self, items: Vec<PilotVal>) -> PilotVal {
        PilotVal::from_list_handle(self.alloc_list_slot(items))
    }

    pub fn new_map(&mut self, m: HashMap<String, PilotVal>) -> PilotVal {
        PilotVal::from_map_handle(self.alloc_map_slot(m))
    }

    fn length(&self, v: PilotVal, op: &str) -> IntrinsicResult {
        if let Some(n) = v.with_str(|s| s.len()) {
            return Ok(PilotVal::from_int(n as i64));
        }
        if v.is_list() {
            let slot = self.list_slot(v, op)?;
            return Ok(PilotVal::from_int(self.heap_lists[slot].len() as i64));
        }
        if v.is_map() {
            let m = &self.heap_maps[self.map_slot(v, op)?];
            return Ok(PilotVal::from_int(m.len() as i64));
        }
        Err(format!("`{op}` expects a string, list or record, found {}", v.kind_label()))
    }

    fn get(&self, obj: PilotVal, k: PilotVal, op: &str) -> IntrinsicResult {
        if obj.is_list() {
            let slot = self.list_slot(obj, op)?;
            let i = self.list_index(slot, k, op)?;
            return Ok(self.heap_lists[slot][i]);
        }
        if obj.is_str() && k.as_int().is_some() {
            let i = k.as_int().unwrap_or(0);
            let (byte, len) = obj
                .with_str(|s| (s.as_bytes().get(i.max(0) as usize).copied(), s.len()))
                .unwrap_or((None, 0));
            return byte
                .filter(|_| i >= 0)
                .map(|b| PilotVal::from_int(b as i64))
                .ok_or_else(|| format!("string index {i} out of bounds for length {len}"));
        }
        let slot = self.map_slot(obj, op).map_err(|_| {
            format!(
                "cannot read `{}` from {}",
                key_of(k),
                obj.kind_label()
            )
        })?;
        let m = &self.heap_maps[slot];
        Ok(with_key(k, |key| m.get(key).copied()).unwrap_or(PilotVal::NULL))
    }

    fn put(&mut self, obj: PilotVal, k: PilotVal, v: PilotVal, op: &str) -> IntrinsicResult {
        if obj.is_list() {
            let slot = self.list_slot(obj, op)?;
            let i = self.list_index(slot, k, op)?;
            self.heap_lists[slot][i] = v;
            return Ok(obj);
        }
        let slot = self.map_slot(obj, op).map_err(|_| {
            format!("cannot assign `{}` on {}", key_of(k), obj.kind_label())
        })?;
        let m = &mut self.heap_maps[slot];
        match with_key(k, |key| m.get_mut(key).map(|e| *e = v)) {
            Some(()) => {}
            None => {
                m.insert(key_of(k), v);
            }
        }
        Ok(obj)
    }

    fn contains(&self, hay: PilotVal, needle: PilotVal, op: &str) -> IntrinsicResult {
        if hay.is_list() {
            let slot = self.list_slot(hay, op)?;
            let found = self.heap_lists[slot]
                .iter()
                .any(|x| crate::ops::equals(*x, needle));
            return Ok(PilotVal::from_bool(found));
        }
        if hay.is_map() {
            let slot = self.map_slot(hay, op)?;
            let m = &self.heap_maps[slot];
            return Ok(PilotVal::from_bool(with_key(needle, |key| m.contains_key(key))));
        }
        let h = str_arg(&[hay], 0, op)?;
        let n = str_arg(&[needle], 0, op)?;
        Ok(PilotVal::from_bool(h.contains(&*n)))
    }

    fn slice(&mut self, args: &[PilotVal], op: &str) -> IntrinsicResult {
        let v = arg(args, 0, op)?;
        let (a, b) = (int_arg(args, 1, op)?, int_arg(args, 2, op)?);
        if v.is_list() {
            let slot = self.list_slot(v, op)?;
            let len = self.heap_lists[slot].len() as i64;
            let (a, b) = (a.clamp(0, len) as usize, b.clamp(0, len) as usize);
            let sub = self.heap_lists[slot][a..b.max(a)].to_vec();
            return Ok(self.new_list(sub));
        }
        let s = str_arg(args, 0, op)?;
        let (a, b) = (char_floor(&s, a), char_floor(&s, b));
        Ok(self.new_str(&s[a..b.max(a)]))
    }

    /// Executes a VM intrinsic by name.
    pub fn intrinsic(&mut self, op: &str, args: &[PilotVal]) -> IntrinsicResult {
        match op {
            // The calls the tools make most come first.
            "map_set" => self.put(arg(args, 0, op)?, arg(args, 1, op)?, arg(args, 2, op)?, op),
            // `list_get` is the name the bindings give a list's element at a position.
            "map_get" | "list_get" => self.get(arg(args, 0, op)?, arg(args, 1, op)?, op),
            "map_has" => {
                let slot = self.map_slot(arg(args, 0, op)?, op)?;
                let m = &self.heap_maps[slot];
                Ok(PilotVal::from_bool(with_key(arg(args, 1, op)?, |k| m.contains_key(k))))
            }
            "str_len" => self.length(arg(args, 0, op)?, op),
            "list_new" => Ok(self.new_list(Vec::new())),
            "list_len" => {
                let v = arg(args, 0, op)?;
                let slot = v
                    .as_list_handle()
                    .filter(|s| *s < self.heap_lists.len())
                    .ok_or_else(|| format!("{} is not iterable", v.kind_label()))?;
                if self.list_is_dead(slot) {
                    return Err(format!("internal error: `{op}` reads a list the collector freed"));
                }
                Ok(PilotVal::from_int(self.heap_lists[slot].len() as i64))
            }
            "list_push" => {
                let list = arg(args, 0, op)?;
                let slot = self.list_slot(list, op)?;
                self.heap_lists[slot].push(arg(args, 1, op)?);
                Ok(list)
            }
            "rt_print" => {
                let parts: Vec<String> = args.iter().map(|a| self.display(*a)).collect();
                println!("{}", parts.join(" "));
                Ok(PilotVal::NULL)
            }
            "rt_eprint" => {
                let parts: Vec<String> = args.iter().map(|a| self.display(*a)).collect();
                eprintln!("{}", parts.join(" "));
                Ok(PilotVal::NULL)
            }
            "stdin_read_line" => Ok(match crate::host::stdin_read_line() {
                Some(line) => self.new_str(&line),
                None => PilotVal::NULL,
            }),
            "stdin_read_all" => {
                let text = crate::host::stdin_read_all();
                Ok(self.new_str(&text))
            }
            "proc_exe" => Ok(crate::host::proc_exe()),
            "proc_run" | "proc_spawn" => {
                let program = str_arg(args, 0, op)?;
                let argv = self.str_list(arg(args, 1, op)?, op)?;
                let input = args.get(2).and_then(|v| v.with_str(str::to_string)).unwrap_or_default();
                let dir = args.get(3).and_then(|v| v.with_str(str::to_string));
                let child = crate::host::spawn(&program, &argv, input, dir.as_deref())?;
                if op == "proc_run" {
                    return self.finish_child(child);
                }
                self.children.push(Some(child));
                Ok(PilotVal::from_int(self.children.len() as i64 - 1))
            }
            "proc_wait" => {
                let handle = int_arg(args, 0, op)?;
                let child = usize::try_from(handle)
                    .ok()
                    .and_then(|h| self.children.get_mut(h))
                    .and_then(Option::take)
                    .ok_or_else(|| format!("`proc_wait`: no running child process {handle}"))?;
                self.finish_child(child)
            }
            // `proc_first_done(handles)`: blocks until one of the running
            // children in `handles` has finished, and returns its handle.
            "proc_first_done" => {
                let handles = self
                    .list_items(arg(args, 0, op)?)
                    .ok_or_else(|| format!("`{op}` expects a list of process handles"))?;
                let mut slots = Vec::with_capacity(handles.len());
                for h in handles {
                    let id = h.as_int().ok_or_else(|| format!("`{op}` expects a list of process handles"))?;
                    match usize::try_from(id).ok().and_then(|i| self.children.get(i)) {
                        Some(Some(_)) => slots.push(id),
                        _ => return Err(format!("`{op}`: no running child process {id}")),
                    }
                }
                if slots.is_empty() {
                    return Err(format!("`{op}`: no child processes to wait for"));
                }
                loop {
                    for &id in &slots {
                        if let Some(Some(child)) = self.children.get(id as usize) {
                            if child.1.is_finished() {
                                return Ok(PilotVal::from_int(id));
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
            "proc_cpus" => Ok(PilotVal::from_int(crate::host::cpu_count())),
            // `field_or_native(obj, key, intrinsic)`: the record's field
            // `key` if it has one, else `intrinsic(obj)` (a native property).
            "field_or_native" => {
                let (obj, key) = (arg(args, 0, op)?, arg(args, 1, op)?);
                if obj.is_map() {
                    let slot = self.map_slot(obj, op)?;
                    if let Some(v) = with_key(key, |k| self.heap_maps[slot].get(k).copied()).as_ref() {
                        return Ok(*v);
                    }
                }
                let hook = str_arg(args, 2, op)?;
                self.intrinsic(&hook, &[obj])
            }
            // `list_extend(list, other, deep)`: appends `other`'s elements;
            // with `deep` set, nested lists are flattened down to their leaves.
            "list_extend" => {
                let list = arg(args, 0, op)?;
                let slot = self.list_slot(list, op)?;
                let deep = arg(args, 2, op)?.as_bool().unwrap_or(false);
                let mut leaves = Vec::new();
                self.collect_elements(arg(args, 1, op)?, deep, op, &mut leaves)?;
                self.heap_lists[slot].extend(leaves);
                Ok(list)
            }
            "list_remove_at" => {
                let list = arg(args, 0, op)?;
                let slot = self.list_slot(list, op)?;
                let i = self.list_index(slot, arg(args, 1, op)?, op)?;
                Ok(self.heap_lists[slot].remove(i))
            }
            "str_byte_at" => {
                let s = str_arg(args, 0, op)?;
                let i = int_arg(args, 1, op)?;
                s.as_bytes()
                    .get(i.max(0) as usize)
                    .filter(|_| i >= 0)
                    .map(|b| PilotVal::from_int(*b as i64))
                    .ok_or_else(|| format!("byte index {i} out of bounds for length {}", s.len()))
            }
            // The str whose UTF-8 bytes are the list's elements; none when
            // they are not valid UTF-8 (RFC 3629).
            "str_from_bytes" => {
                let slot = self.list_slot(arg(args, 0, op)?, op)?;
                let mut bytes = Vec::with_capacity(self.heap_lists[slot].len());
                for v in &self.heap_lists[slot] {
                    let b = v
                        .as_int()
                        .filter(|b| (0..=255).contains(b))
                        .ok_or_else(|| format!("`{op}` takes bytes (0 to 255), found {}", v.to_string_repr()))?;
                    bytes.push(b as u8);
                }
                Ok(String::from_utf8(bytes).map_or(PilotVal::NULL, |s| self.new_str(&s)))
            }
            "str_slice" => self.slice(args, op),
            "str_find" => {
                let (h, n) = (str_arg(args, 0, op)?, str_arg(args, 1, op)?);
                let start = if args.len() >= 3 { int_arg(args, 2, op)? } else { 0 };
                if start < 0 {
                    return Ok(PilotVal::from_int(-1));
                }
                let u = char_floor(&h, start);
                Ok(PilotVal::from_int(h[u..].find(&*n).map_or(-1, |o| (u + o) as i64)))
            }
            "str_replace" => {
                let (h, from, to) = (str_arg(args, 0, op)?, str_arg(args, 1, op)?, str_arg(args, 2, op)?);
                if from.is_empty() {
                    return Ok(self.new_str(&h));
                }
                Ok(self.new_str(&h.replace(&*from, &*to)))
            }
            "str_contains" => self.contains(arg(args, 0, op)?, arg(args, 1, op)?, op),
            "str_starts_with" => {
                let (h, n) = (str_arg(args, 0, op)?, str_arg(args, 1, op)?);
                Ok(PilotVal::from_bool(h.starts_with(&*n)))
            }
            "str_ends_with" => {
                let (h, n) = (str_arg(args, 0, op)?, str_arg(args, 1, op)?);
                Ok(PilotVal::from_bool(h.ends_with(&*n)))
            }
            "map_new" => Ok(self.new_map(HashMap::new())),
            // `map_values(m)`: the values of map `m` as a new list, in the
            // order of their keys (the order a map is shown in).
            "map_values" => {
                let slot = self.map_slot(arg(args, 0, op)?, op)?;
                let mut entries: Vec<(&String, &PilotVal)> = self.heap_maps[slot].iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                let values = entries.into_iter().map(|(_, v)| *v).collect();
                Ok(self.new_list(values))
            }
            // `record_set_type(obj, name)` marks a record as built from struct `name`.
            "record_set_type" => {
                let obj = arg(args, 0, op)?;
                let slot = self.map_slot(obj, op)?;
                self.record_types.insert(slot, arg(args, 1, op)?);
                Ok(obj)
            }
            // The struct a record was built as, or `none`.
            "record_type" => {
                let v = arg(args, 0, op)?;
                let ty = v.as_map_handle().and_then(|slot| self.record_types.get(&slot));
                Ok(ty.copied().unwrap_or(PilotVal::NULL))
            }
            // `record_copy(obj)`: a new record with the same type and fields
            // (a value type is copied where it is bound, passed or stored).
            "record_copy" => {
                let obj = arg(args, 0, op)?;
                let Some(slot) = obj.as_map_handle().filter(|s| *s < self.heap_maps.len()) else {
                    return Ok(obj);
                };
                let fields = self.heap_maps[slot].clone();
                let copy = self.new_map(fields);
                if let Some(ty) = self.record_types.get(&slot).copied() {
                    self.record_types.insert(self.map_slot(copy, op)?, ty);
                }
                Ok(copy)
            }
            // `value_clone(v)`: a deep copy of `v` sharing nothing that could
            // change (a constant's copy, `var x = CONST;`); strings are shared.
            "value_clone" => self.value_clone(arg(args, 0, op)?, 0),
            "map_remove" => {
                let slot = self.map_slot(arg(args, 0, op)?, op)?;
                let m = &mut self.heap_maps[slot];
                Ok(PilotVal::from_bool(with_key(arg(args, 1, op)?, |k| m.remove(k)).is_some()))
            }
            "is_null" => Ok(PilotVal::from_bool(arg(args, 0, op)?.is_null())),
            // `closure_new(entry, arity, captured...)` builds a function value.
            "closure_new" => {
                Ok(PilotVal::from_fn_handle(self.alloc_list_slot(args.to_vec())))
            }
            // `closure_env(fn, i)` reads the i-th captured value of a closure.
            "closure_env" => {
                let f = arg(args, 0, op)?;
                let i = arg(args, 1, op)?.as_int().unwrap_or(-1);
                f.as_fn_handle()
                    .filter(|&slot| !self.list_is_dead(slot))
                    .and_then(|slot| self.heap_lists.get(slot))
                    .and_then(|env| usize::try_from(i).ok().and_then(|i| env.get(i + 2)))
                    .copied()
                    .ok_or_else(|| format!("{} has no captured value #{i}", f.kind_label()))
            }
            "global_get" => {
                let name = str_arg(args, 0, op)?;
                self.globals
                    .get(&*name)
                    .copied()
                    .ok_or_else(|| format!("`{name}` is read before its declaration has run"))
            }
            "global_set" => {
                let v = arg(args, 1, op)?;
                self.globals.insert(str_arg(args, 0, op)?.to_string(), v);
                Ok(v)
            }
            // `num_fits(v, register_class, width)`: see `ops::fits`.
            "num_fits" => {
                let class = str_arg(args, 1, op)?;
                let fits = crate::ops::fits(arg(args, 0, op)?, &class, int_arg(args, 2, op)?);
                Ok(PilotVal::from_bool(fits))
            }
            // `num_convert(v, register_class, width)`: see `ops::convert`.
            "num_convert" => {
                let class = str_arg(args, 1, op)?;
                crate::ops::convert(arg(args, 0, op)?, &class, int_arg(args, 2, op)?)
            }
            // `scalar_from(v)`: `v` as a character. A string that holds
            // exactly one character is that character; see `ops::scalar_from`.
            "scalar_from" => {
                let v = arg(args, 0, op)?;
                let one = v.with_str(|s| {
                    let mut it = s.chars();
                    match (it.next(), it.next()) {
                        (Some(c), None) => Some(c),
                        _ => None,
                    }
                });
                match one.flatten() {
                    Some(c) => Ok(PilotVal::from_char(c)),
                    None => crate::ops::scalar_from(v),
                }
            }
            // `is_kind(v, kind, name)`: whether `v` is a value of the kind the
            // checker names for a type (`x is T`): a whole number, a fraction,
            // a truth value, text, a character, the absent value, or a
            // record of the type `name`.
            "is_kind" => {
                let v = arg(args, 0, op)?;
                let kind = str_arg(args, 1, op)?;
                let yes = match &*kind {
                    "whole_number" => v.is_int(),
                    "fraction" => v.is_float(),
                    "truth" => v.is_bool(),
                    "text" => v.is_str(),
                    "character" => v.as_char().is_some(),
                    "absent" => v.is_null(),
                    "record" => {
                        let name = str_arg(args, 2, op)?;
                        let ty = v.as_map_handle().and_then(|slot| self.record_types.get(&slot)).copied();
                        ty.and_then(|t| t.with_str(|s| s == &*name)).unwrap_or(false)
                    }
                    other => return Err(format!("`is` cannot test for a value of kind `{other}`")),
                };
                Ok(PilotVal::from_bool(yes))
            }
            // `num_wrap(v, register_class, width)`: see `ops::wrap`.
            "num_wrap" => {
                let class = str_arg(args, 1, op)?;
                crate::ops::wrap(arg(args, 0, op)?, &class, int_arg(args, 2, op)?)
            }
            "list_pop" => {
                let list = arg(args, 0, op)?;
                let slot = self.list_slot(list, op)?;
                self.heap_lists[slot]
                    .pop()
                    .ok_or_else(|| "pop from an empty list".to_string())
            }
            "str_trim" => {
                let s = str_arg(args, 0, op)?;
                Ok(self.new_str(s.trim()))
            }
            "to_str" => {
                let text = self.display(arg(args, 0, op)?);
                Ok(self.new_str(&text))
            }
            // The comptime environment record is heap slot 0 (see pilot-analysis comptime.rs).
            "get_env" => Ok(PilotVal::from_map_handle(0)),
            "sym_str" => {
                let sid = int_arg(args, 0, op)? as u32;
                if let Some(s) = self.shared_symbols.as_ref().and_then(|t| t.get(&sid)) {
                    return Ok(PilotVal::from_str(s));
                }
                self.forward(op, args)
            }
            "proc_args" => Ok(self.proc_args()),
            // An operation whose failure comes back as a value (checked.rs).
            "op_checked" => self.op_checked(args),
            // An intentionally unconnected entry or exit point was reached.
            // The program states it cannot go on; the message is its own.
            "rt_fault" => Err(self.display(arg(args, 0, op)?)),
            "seam_open" => {
                let id = str_arg(args, 0, op)?;
                let note = args.get(1).map(|&v| self.display(v)).unwrap_or_default();
                Err(format!("SEAM001: unconnected seam `{id}`: {note}"))
            }
            "unit_header" => {
                let path = str_arg(args, 0, op)?;
                Ok(self.unit_header(&path))
            }
            _ => self.forward(op, args),
        }
    }

    /// Forwards an intrinsic to the host engine handler, then to the OS table.
    fn forward(&mut self, op: &str, args: &[PilotVal]) -> IntrinsicResult {
        if let Some(r) = self.host_op(op, args) {
            return r.map_err(|e| format!("intrinsic `{op}` failed: {e}"));
        }
        if let Some(h) = self.engine_handler.as_mut() {
            match h(op, args) {
                Ok(v) if !v.is_null() => return Ok(v),
                Err(e) => return Err(format!("intrinsic `{op}` failed: {e}")),
                _ => {}
            }
        }
        Ok(pilot_rt::PilotOsSyscall::vm_intrinsic(op, args))
    }
}
