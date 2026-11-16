//! VM heap slot helpers: lists, records, strings and host forwarding.

use crate::collect::Record;
use crate::value::PilotVal;
use crate::vm::PilVirtualMachine;

/// Display nesting limit; deeper containers print as `...`.
const DISPLAY_DEPTH: usize = 8;

pub(crate) type IntrinsicResult = Result<PilotVal, String>;

/// Fast-path read names recognised by `PilVirtualMachine::reads`.
pub(crate) const READS: [&str; 2] = ["map_get", "list_get"];

pub(crate) fn arg(args: &[PilotVal], i: usize, op: &str) -> Result<PilotVal, String> {
    args.get(i).copied().ok_or_else(|| {
        format!(
            "`{op}` expects at least {} argument(s), got {}",
            i + 1,
            args.len()
        )
    })
}

pub(crate) fn int_arg(args: &[PilotVal], i: usize, op: &str) -> Result<i64, String> {
    let v = arg(args, i, op)?;
    v.as_int().ok_or_else(|| {
        format!(
            "`{op}` argument {} must be an integer, found {}",
            i + 1,
            v.kind_label()
        )
    })
}

pub(crate) enum StrArg {
    Inline([u8; 14], usize),
    Stored(*const u8, usize),
}

impl std::ops::Deref for StrArg {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            StrArg::Inline(buf, len) => unsafe { std::str::from_utf8_unchecked(&buf[..*len]) },
            StrArg::Stored(ptr, len) => unsafe {
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(*ptr, *len))
            },
        }
    }
}

impl std::fmt::Display for StrArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self)
    }
}

pub(crate) fn str_arg(args: &[PilotVal], i: usize, op: &str) -> Result<StrArg, String> {
    let v = arg(args, i, op)?;
    if let Some((ptr, len)) = v.heap_parts() {
        return Ok(StrArg::Stored(ptr, len));
    }
    let mut buf = [0u8; 14];
    match v.str_in(&mut buf).map(str::len) {
        Some(len) => Ok(StrArg::Inline(buf, len)),
        None => Err(format!(
            "`{op}` argument {} must be a string, found {}",
            i + 1,
            v.kind_label()
        )),
    }
}

pub(crate) fn char_floor(s: &str, at: i64) -> usize {
    let mut u = (at.max(0) as usize).min(s.len());
    while u < s.len() && !s.is_char_boundary(u) {
        u += 1;
    }
    u
}

fn key_of(k: PilotVal) -> String {
    k.with_str(|s| s.to_string())
        .unwrap_or_else(|| k.to_string_repr())
}

pub(crate) fn with_key<R>(k: PilotVal, f: impl FnOnce(&str) -> R) -> R {
    let mut buf = [0u8; 14];
    match k.str_in(&mut buf) {
        Some(s) => f(s),
        None => f(&k.to_string_repr()),
    }
}

impl PilVirtualMachine {
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

    pub(crate) fn list_slot(&self, v: PilotVal, op: &str) -> Result<usize, String> {
        let slot = v
            .as_list_handle()
            .filter(|s| *s < self.heap_lists.len())
            .ok_or_else(|| format!("`{op}` expects a list, found {}", v.kind_label()))?;
        if self.list_is_dead(slot) {
            return Err(format!(
                "internal error: `{op}` reads a list the collector freed"
            ));
        }
        Ok(slot)
    }

    pub(crate) fn collect_elements(
        &self,
        v: PilotVal,
        deep: bool,
        op: &str,
        out: &mut Vec<PilotVal>,
    ) -> Result<(), String> {
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

    pub(crate) fn value_clone(&mut self, v: PilotVal, depth: usize) -> IntrinsicResult {
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
            let entries: Vec<(String, PilotVal)> = self.heap_maps[slot]
                .iter()
                .map(|(k, x)| (k.clone(), *x))
                .collect();
            let mut out = Record::with_capacity_and_hasher(entries.len(), Default::default());
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

    pub(crate) fn map_slot(&self, v: PilotVal, op: &str) -> Result<usize, String> {
        let slot = v
            .as_map_handle()
            .filter(|s| *s < self.heap_maps.len())
            .ok_or_else(|| format!("`{op}` expects a record, found {}", v.kind_label()))?;
        if self.map_is_dead(slot) {
            return Err(format!(
                "internal error: `{op}` reads a record the collector freed"
            ));
        }
        Ok(slot)
    }

    pub(crate) fn list_index(&self, slot: usize, k: PilotVal, op: &str) -> Result<usize, String> {
        let len = self.heap_lists[slot].len();
        let i = k.as_int().ok_or_else(|| {
            format!(
                "`{op}` list index must be an integer, found {}",
                k.kind_label()
            )
        })?;
        if i < 0 || i as usize >= len {
            return Err(format!("list index {i} out of bounds for length {len}"));
        }
        Ok(i as usize)
    }

    pub fn new_list(&mut self, items: Vec<PilotVal>) -> PilotVal {
        PilotVal::from_list_handle(self.alloc_list_slot(items))
    }

    pub fn new_map(&mut self, m: Record) -> PilotVal {
        PilotVal::from_map_handle(self.alloc_map_slot(m))
    }

    pub(crate) fn length(&self, v: PilotVal, op: &str) -> IntrinsicResult {
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
        Err(format!(
            "`{op}` expects a string, list or record, found {}",
            v.kind_label()
        ))
    }

    pub(crate) fn sorted_map_keys(&mut self, slot: usize) -> PilotVal {
        let mut keys: Vec<String> = self.heap_maps[slot].keys().cloned().collect();
        keys.sort();
        let vals = keys.into_iter().map(|k| self.new_str(&k)).collect();
        self.new_list(vals)
    }

    pub(crate) fn get(&self, obj: PilotVal, k: PilotVal, op: &str) -> IntrinsicResult {
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
        let slot = self
            .map_slot(obj, op)
            .map_err(|_| format!("cannot read `{}` from {}", key_of(k), obj.kind_label()))?;
        let m = &self.heap_maps[slot];
        Ok(with_key(k, |key| m.get(key).copied()).unwrap_or(PilotVal::NULL))
    }

    pub(crate) fn put(
        &mut self,
        obj: PilotVal,
        k: PilotVal,
        v: PilotVal,
        op: &str,
    ) -> IntrinsicResult {
        if obj.is_list() {
            let slot = self.list_slot(obj, op)?;
            let i = self.list_index(slot, k, op)?;
            self.heap_lists[slot][i] = v;
            return Ok(obj);
        }
        let slot = self
            .map_slot(obj, op)
            .map_err(|_| format!("cannot assign `{}` on {}", key_of(k), obj.kind_label()))?;
        let m = &mut self.heap_maps[slot];
        match with_key(k, |key| m.get_mut(key).map(|e| *e = v)) {
            Some(()) => {}
            None => {
                m.insert(key_of(k), v);
            }
        }
        Ok(obj)
    }

    pub(crate) fn contains(&self, hay: PilotVal, needle: PilotVal, op: &str) -> IntrinsicResult {
        if hay.is_list() {
            let slot = self.list_slot(hay, op)?;
            return Ok(PilotVal::from_bool(
                self.heap_lists[slot]
                    .iter()
                    .any(|x| crate::ops::equals(*x, needle)),
            ));
        }
        if hay.is_map() {
            let slot = self.map_slot(hay, op)?;
            let m = &self.heap_maps[slot];
            return Ok(PilotVal::from_bool(with_key(needle, |key| {
                m.contains_key(key)
            })));
        }
        let (h, n) = (str_arg(&[hay], 0, op)?, str_arg(&[needle], 0, op)?);
        Ok(PilotVal::from_bool(h.contains(&*n)))
    }

    pub(crate) fn slice(&mut self, args: &[PilotVal], op: &str) -> IntrinsicResult {
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

    pub(crate) fn forward(&mut self, op: &str, args: &[PilotVal]) -> IntrinsicResult {
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
        Ok(PilotVal::NULL)
    }
}
