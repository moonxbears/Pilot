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

fn str_arg(args: &[PilotVal], i: usize, op: &str) -> Result<String, String> {
    let v = arg(args, i, op)?;
    v.with_str(|s| s.to_string())
        .ok_or_else(|| format!("`{op}` argument {} must be a string, found {}", i + 1, v.kind_label()))
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
        v.as_list_handle()
            .filter(|s| *s < self.heap_lists.len())
            .ok_or_else(|| format!("`{op}` expects a list, found {}", v.kind_label()))
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

    fn map_slot(&self, v: PilotVal, op: &str) -> Result<usize, String> {
        v.as_map_handle()
            .filter(|s| *s < self.heap_maps.len())
            .ok_or_else(|| format!("`{op}` expects a record, found {}", v.kind_label()))
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
        self.heap_lists.push(items);
        PilotVal::from_list_handle(self.heap_lists.len() - 1)
    }

    pub fn new_map(&mut self, m: HashMap<String, PilotVal>) -> PilotVal {
        self.heap_maps.push(m);
        PilotVal::from_map_handle(self.heap_maps.len() - 1)
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
            let s = obj.with_str(|s| s.to_string()).unwrap_or_default();
            let i = k.as_int().unwrap_or(0);
            return s
                .as_bytes()
                .get(i.max(0) as usize)
                .filter(|_| i >= 0)
                .map(|b| PilotVal::from_int(*b as i64))
                .ok_or_else(|| format!("string index {i} out of bounds for length {}", s.len()));
        }
        let slot = self.map_slot(obj, op).map_err(|_| {
            format!(
                "cannot read `{}` from {}",
                key_of(k),
                obj.kind_label()
            )
        })?;
        Ok(self.heap_maps[slot].get(&key_of(k)).copied().unwrap_or(PilotVal::NULL))
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
        self.heap_maps[slot].insert(key_of(k), v);
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
            return Ok(PilotVal::from_bool(self.heap_maps[slot].contains_key(&key_of(needle))));
        }
        let h = str_arg(&[hay], 0, op)?;
        let n = str_arg(&[needle], 0, op)?;
        Ok(PilotVal::from_bool(h.contains(&n)))
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
        Ok(PilotVal::from_str(&s[a..b.max(a)]))
    }

    /// Executes a VM intrinsic by name.
    pub fn intrinsic(&mut self, op: &str, args: &[PilotVal]) -> IntrinsicResult {
        match op {
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
            "stdin_read_line" => Ok(crate::host::stdin_read_line()),
            "stdin_read_all" => Ok(crate::host::stdin_read_all()),
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
            "proc_cpus" => Ok(PilotVal::from_int(crate::host::cpu_count())),
            // `field_or_native(obj, key, intrinsic)`: the record's field
            // `key` if it has one, else `intrinsic(obj)` (a native property).
            "field_or_native" => {
                let (obj, key) = (arg(args, 0, op)?, arg(args, 1, op)?);
                if let Some(slot) = obj.as_map_handle().filter(|s| *s < self.heap_maps.len()) {
                    if let Some(v) = self.heap_maps[slot].get(&key_of(key)) {
                        return Ok(*v);
                    }
                }
                let hook = str_arg(args, 2, op)?;
                self.intrinsic(&hook, &[obj])
            }
            "rt_assert" => {
                let v = arg(args, 0, op)?;
                let note = args.get(1).map(|m| format!(": {}", key_of(*m))).unwrap_or_default();
                match v.as_bool() {
                    Some(true) => Ok(PilotVal::NULL),
                    Some(false) => Err(format!("assertion failed{note}")),
                    None => Err(format!("assert expects a boolean, found {}", v.kind_label())),
                }
            }
            "str_len" => self.length(arg(args, 0, op)?, op),
            "list_new" => Ok(self.new_list(Vec::new())),
            "list_len" => {
                let v = arg(args, 0, op)?;
                let slot = v
                    .as_list_handle()
                    .filter(|s| *s < self.heap_lists.len())
                    .ok_or_else(|| format!("{} is not iterable", v.kind_label()))?;
                Ok(PilotVal::from_int(self.heap_lists[slot].len() as i64))
            }
            "list_push" => {
                let list = arg(args, 0, op)?;
                let slot = self.list_slot(list, op)?;
                self.heap_lists[slot].push(arg(args, 1, op)?);
                Ok(list)
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
                Ok(String::from_utf8(bytes).map_or(PilotVal::NULL, |s| PilotVal::from_str(&s)))
            }
            "str_slice" => self.slice(args, op),
            "str_find" => {
                let (h, n) = (str_arg(args, 0, op)?, str_arg(args, 1, op)?);
                let start = if args.len() >= 3 { int_arg(args, 2, op)? } else { 0 };
                if start < 0 {
                    return Ok(PilotVal::from_int(-1));
                }
                let u = char_floor(&h, start);
                Ok(PilotVal::from_int(h[u..].find(&n).map_or(-1, |o| (u + o) as i64)))
            }
            "str_replace" => {
                let (h, from, to) = (str_arg(args, 0, op)?, str_arg(args, 1, op)?, str_arg(args, 2, op)?);
                if from.is_empty() {
                    return Ok(PilotVal::from_str(&h));
                }
                Ok(PilotVal::from_str(&h.replace(&from, &to)))
            }
            "str_contains" => self.contains(arg(args, 0, op)?, arg(args, 1, op)?, op),
            "str_starts_with" => {
                let (h, n) = (str_arg(args, 0, op)?, str_arg(args, 1, op)?);
                Ok(PilotVal::from_bool(h.starts_with(&n)))
            }
            "str_ends_with" => {
                let (h, n) = (str_arg(args, 0, op)?, str_arg(args, 1, op)?);
                Ok(PilotVal::from_bool(h.ends_with(&n)))
            }
            "map_new" => Ok(self.new_map(HashMap::new())),
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
            "map_set" => self.put(arg(args, 0, op)?, arg(args, 1, op)?, arg(args, 2, op)?, op),
            "map_get" => self.get(arg(args, 0, op)?, arg(args, 1, op)?, op),
            "map_has" => {
                let slot = self.map_slot(arg(args, 0, op)?, op)?;
                let k = key_of(arg(args, 1, op)?);
                Ok(PilotVal::from_bool(self.heap_maps[slot].contains_key(&k)))
            }
            "map_remove" => {
                let slot = self.map_slot(arg(args, 0, op)?, op)?;
                let k = key_of(arg(args, 1, op)?);
                Ok(PilotVal::from_bool(self.heap_maps[slot].remove(&k).is_some()))
            }
            "is_null" => Ok(PilotVal::from_bool(arg(args, 0, op)?.is_null())),
            // `closure_new(entry, arity, captured...)` builds a function value.
            "closure_new" => {
                let slot = self.heap_lists.len();
                self.heap_lists.push(args.to_vec());
                Ok(PilotVal::from_fn_handle(slot))
            }
            // `closure_env(fn, i)` reads the i-th captured value of a closure.
            "closure_env" => {
                let f = arg(args, 0, op)?;
                let i = arg(args, 1, op)?.as_int().unwrap_or(-1);
                f.as_fn_handle()
                    .and_then(|slot| self.heap_lists.get(slot))
                    .and_then(|env| usize::try_from(i).ok().and_then(|i| env.get(i + 2)))
                    .copied()
                    .ok_or_else(|| format!("{} has no captured value #{i}", f.kind_label()))
            }
            "global_get" => {
                let name = str_arg(args, 0, op)?;
                self.globals
                    .get(&name)
                    .copied()
                    .ok_or_else(|| format!("`{name}` is read before its declaration has run"))
            }
            "global_set" => {
                let v = arg(args, 1, op)?;
                self.globals.insert(str_arg(args, 0, op)?, v);
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
            "list_pop" => {
                let list = arg(args, 0, op)?;
                let slot = self.list_slot(list, op)?;
                self.heap_lists[slot]
                    .pop()
                    .ok_or_else(|| "pop from an empty list".to_string())
            }
            "str_trim" => Ok(PilotVal::from_str(str_arg(args, 0, op)?.trim())),
            "to_str" => {
                let v = arg(args, 0, op)?;
                Ok(PilotVal::from_str(&self.display(v)))
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
