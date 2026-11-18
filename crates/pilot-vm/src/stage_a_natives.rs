//! Stage-A string-matched native dispatch table (`PROP-002a` §4).
//!
//! All string-matched `CallNative` / `EngineIntrinsic` arms live in this single
//! shrink-only file (`stage_a_vm_natives()` in `selfhost/gate/manifest.pilot`).

use crate::collect::Record;
use crate::heap::{arg, char_floor, int_arg, str_arg, with_key, IntrinsicResult};
use crate::instruction::Instruction;
use crate::value::PilotVal;
use crate::vm::PilVirtualMachine;

extern "C" fn stage_a_vm_thread_entry(_env: usize) -> i64 {
    0
}

extern "C" fn stage_a_vm_callback_dispatch(_slot: usize) -> i64 {
    0
}

impl PilVirtualMachine {
    pub(crate) fn gen_intrinsic(
        &mut self,
        hook: &str,
        ins: Instruction,
        args: &[PilotVal],
    ) -> Result<bool, String> {
        let code = match hook {
            "gen_new" => 0,
            "gen_resume" => 1,
            "gen_close" => 2,
            "gen_yield" => 3,
            "gen_done" => 4,
            "gen_current" => 5,
            "gen_closing" => 6,
            _ => return Ok(false),
        };
        self.exec_gen_op(code, ins, args)?;
        Ok(true)
    }

    pub fn intrinsic(&mut self, op: &str, args: &[PilotVal]) -> IntrinsicResult {
        match op {
            "host_op" => crate::bridge::dispatch(self, args),
            "map_set" => self.put(arg(args, 0, op)?, arg(args, 1, op)?, arg(args, 2, op)?, op),
            "map_get" | "list_get" => self.get(arg(args, 0, op)?, arg(args, 1, op)?, op),
            "map_has" => {
                let m = &self.heap_maps[self.map_slot(arg(args, 0, op)?, op)?];
                Ok(PilotVal::from_bool(with_key(arg(args, 1, op)?, |k| {
                    m.contains_key(k)
                })))
            }
            "str_len" | "list_len" => self.length(arg(args, 0, op)?, op),
            "list_new" => Ok(self.new_list(Vec::new())),
            "list_push" => {
                let (list, item) = (arg(args, 0, op)?, arg(args, 1, op)?);
                let slot = self.list_slot(list, op)?;
                self.heap_lists[slot].push(item);
                Ok(list)
            }
            "rt_print" | "rt_eprint" => {
                let line = args
                    .iter()
                    .map(|a| self.display(*a))
                    .collect::<Vec<_>>()
                    .join(" ");
                if op == "rt_print" {
                    println!("{line}");
                } else {
                    eprintln!("{line}");
                }
                Ok(PilotVal::NULL)
            }
            "field_or_native" => {
                let (obj, key) = (arg(args, 0, op)?, arg(args, 1, op)?);
                if obj.is_map() {
                    let slot = self.map_slot(obj, op)?;
                    if let Some(v) = with_key(key, |k| self.heap_maps[slot].get(k).copied()) {
                        return Ok(v);
                    }
                }
                self.intrinsic(&str_arg(args, 2, op)?, &[obj])
            }
            "list_extend" => {
                let list = arg(args, 0, op)?;
                let slot = self.list_slot(list, op)?;
                let mut leaves = Vec::new();
                self.collect_elements(
                    arg(args, 1, op)?,
                    arg(args, 2, op)?.as_bool().unwrap_or(false),
                    op,
                    &mut leaves,
                )?;
                self.heap_lists[slot].extend(leaves);
                Ok(list)
            }
            "map_extend" => {
                let map = arg(args, 0, op)?;
                let (dst, src) = (
                    self.map_slot(map, op)?,
                    self.map_slot(arg(args, 1, op)?, op)?,
                );
                let entries: Vec<(String, PilotVal)> = self.heap_maps[src]
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();
                self.heap_maps[dst].extend(entries);
                Ok(map)
            }
            "list_remove_at" => {
                let slot = self.list_slot(arg(args, 0, op)?, op)?;
                let idx = self.list_index(slot, arg(args, 1, op)?, op)?;
                Ok(self.heap_lists[slot].remove(idx))
            }
            "str_byte_at" => {
                let (s, i) = (str_arg(args, 0, op)?, int_arg(args, 1, op)?);
                s.as_bytes()
                    .get(i.max(0) as usize)
                    .filter(|_| i >= 0)
                    .map(|&b| PilotVal::from_int(b as i64))
                    .ok_or_else(|| format!("byte index {i} out of bounds for length {}", s.len()))
            }
            "str_from_bytes" => {
                let slot = self.list_slot(arg(args, 0, op)?, op)?;
                let mut bytes = Vec::with_capacity(self.heap_lists[slot].len());
                for v in &self.heap_lists[slot] {
                    bytes.push(
                        v.as_int()
                            .filter(|b| (0..=255).contains(b))
                            .ok_or_else(|| {
                                format!(
                                    "`{op}` takes bytes (0 to 255), found {}",
                                    v.to_string_repr()
                                )
                            })? as u8,
                    );
                }
                Ok(String::from_utf8(bytes).map_or(PilotVal::NULL, |s| self.new_str(&s)))
            }
            "str_copy_to_ptr" => {
                let (s, dst_ptr, cap) = (
                    str_arg(args, 0, op)?,
                    int_arg(args, 1, op)?,
                    int_arg(args, 2, op)?,
                );
                let n = s.len().min(cap.max(0) as usize);
                if n > 0 && dst_ptr != 0 {
                    unsafe { core::ptr::copy_nonoverlapping(s.as_ptr(), dst_ptr as *mut u8, n) };
                }
                Ok(PilotVal::from_int(s.len() as i64))
            }
            "str_from_ptr" => {
                let (src_ptr, len) = (int_arg(args, 0, op)?, int_arg(args, 1, op)?);
                if len <= 0 || src_ptr == 0 {
                    return Ok(self.new_str(""));
                }
                let s = String::from_utf8_lossy(unsafe {
                    core::slice::from_raw_parts(src_ptr as *const u8, len as usize)
                })
                .into_owned();
                Ok(self.new_str(&s))
            }
            "vm_thread_entry_ptr" => Ok(PilotVal::from_int(
                stage_a_vm_thread_entry as *const () as usize as i64,
            )),
            "vm_callback_dispatch_ptr" => Ok(PilotVal::from_int(
                stage_a_vm_callback_dispatch as *const () as usize as i64,
            )),
            "str_slice" => self.slice(args, op),
            "str_find" => {
                let (h, n) = (str_arg(args, 0, op)?, str_arg(args, 1, op)?);
                let start = if args.len() >= 3 {
                    int_arg(args, 2, op)?
                } else {
                    0
                };
                if start < 0 {
                    return Ok(PilotVal::from_int(-1));
                }
                let u = char_floor(&h, start);
                Ok(PilotVal::from_int(
                    h[u..].find(&*n).map_or(-1, |o| (u + o) as i64),
                ))
            }
            "str_replace" => {
                let (h, from, to) = (
                    str_arg(args, 0, op)?,
                    str_arg(args, 1, op)?,
                    str_arg(args, 2, op)?,
                );
                Ok(if from.is_empty() {
                    self.new_str(&h)
                } else {
                    self.new_str(&h.replace(&*from, &*to))
                })
            }
            "str_contains" => self.contains(arg(args, 0, op)?, arg(args, 1, op)?, op),
            "str_starts_with" => Ok(PilotVal::from_bool(
                str_arg(args, 0, op)?.starts_with(&*str_arg(args, 1, op)?),
            )),
            "str_ends_with" => Ok(PilotVal::from_bool(
                str_arg(args, 0, op)?.ends_with(&*str_arg(args, 1, op)?),
            )),
            "map_new" => Ok(self.new_map(Record::default())),
            "map_values" => {
                let slot = self.map_slot(arg(args, 0, op)?, op)?;
                let mut entries: Vec<(&String, &PilotVal)> = self.heap_maps[slot].iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                Ok(self.new_list(entries.into_iter().map(|(_, v)| *v).collect()))
            }
            "map_keys" => {
                let slot = self.map_slot(arg(args, 0, op)?, op)?;
                Ok(self.sorted_map_keys(slot))
            }
            "for_seq" => match arg(args, 0, op)? {
                v if v.is_list() || v.is_str() => Ok(v),
                v if v.is_map() => {
                    let slot = self.map_slot(v, op)?;
                    Ok(self.sorted_map_keys(slot))
                }
                v => Err(format!("{} is not iterable", v.kind_label())),
            },
            "native_in_check" => {
                let (h, n) = (arg(args, 0, op)?, arg(args, 1, op)?);
                if h.is_str() || h.is_map() {
                    self.contains(h, n, op)
                } else {
                    Ok(PilotVal::NULL)
                }
            }
            "record_set_type" => {
                let obj = arg(args, 0, op)?;
                let slot = self.map_slot(obj, op)?;
                self.record_types.insert(slot, arg(args, 1, op)?);
                Ok(obj)
            }
            "record_type" => Ok(arg(args, 0, op)?
                .as_map_handle()
                .and_then(|s| self.record_types.get(&s))
                .copied()
                .unwrap_or(PilotVal::NULL)),
            "record_copy" => {
                let obj = arg(args, 0, op)?;
                let Some(slot) = obj.as_map_handle().filter(|&s| s < self.heap_maps.len()) else {
                    return Ok(obj);
                };
                let copy = self.new_map(self.heap_maps[slot].clone());
                if let Some(&ty) = self.record_types.get(&slot) {
                    let cslot = self.map_slot(copy, op)?;
                    self.record_types.insert(cslot, ty);
                }
                Ok(copy)
            }
            "value_clone" => self.value_clone(arg(args, 0, op)?, 0),
            "map_remove" => {
                let slot = self.map_slot(arg(args, 0, op)?, op)?;
                Ok(PilotVal::from_bool(
                    with_key(arg(args, 1, op)?, |k| self.heap_maps[slot].remove(k)).is_some(),
                ))
            }
            "is_null" => Ok(PilotVal::from_bool(arg(args, 0, op)?.is_null())),
            "closure_new" => Ok(PilotVal::from_fn_handle(
                self.alloc_list_slot(args.to_vec()),
            )),
            "closure_env" => {
                let (f, i) = (arg(args, 0, op)?, arg(args, 1, op)?.as_int().unwrap_or(-1));
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
            "num_fits" => Ok(PilotVal::from_bool(crate::ops::fits(
                arg(args, 0, op)?,
                &str_arg(args, 1, op)?,
                int_arg(args, 2, op)?,
            ))),
            "num_convert" => crate::ops::convert(
                arg(args, 0, op)?,
                &str_arg(args, 1, op)?,
                int_arg(args, 2, op)?,
            ),
            "num_wrap" => crate::ops::wrap(
                arg(args, 0, op)?,
                &str_arg(args, 1, op)?,
                int_arg(args, 2, op)?,
            ),
            "scalar_from" => {
                let v = arg(args, 0, op)?;
                let one = v.with_str(|s| {
                    let mut it = s.chars();
                    it.next().filter(|_| it.next().is_none())
                });
                one.flatten().map_or_else(
                    || crate::ops::scalar_from(v),
                    |c| Ok(PilotVal::from_char(c)),
                )
            }
            "is_kind" => {
                let (v, kind) = (arg(args, 0, op)?, str_arg(args, 1, op)?);
                let yes = match &*kind {
                    "whole_number" => v.is_int(),
                    "fraction" => v.is_float(),
                    "truth" => v.is_bool(),
                    "text" => v.is_str(),
                    "character" => v.as_char().is_some(),
                    "absent" => v.is_null(),
                    "record" => {
                        let name = str_arg(args, 2, op)?;
                        v.as_map_handle()
                            .and_then(|s| self.record_types.get(&s))
                            .and_then(|t| t.with_str(|s| s == &*name))
                            .unwrap_or(false)
                    }
                    other => return Err(format!("`is` cannot test for a value of kind `{other}`")),
                };
                Ok(PilotVal::from_bool(yes))
            }
            "list_pop" => {
                let slot = self.list_slot(arg(args, 0, op)?, op)?;
                self.heap_lists[slot]
                    .pop()
                    .ok_or_else(|| "pop from an empty list".to_string())
            }
            "str_trim" => Ok(self.new_str(str_arg(args, 0, op)?.trim())),
            "to_str" => {
                let text = self.display(arg(args, 0, op)?);
                Ok(self.new_str(&text))
            }
            "get_env" => Ok(PilotVal::from_map_handle(0)),
            "sym_str" => {
                let sid = int_arg(args, 0, op)? as u32;
                match self.shared_symbols.as_ref().and_then(|t| t.get(&sid)) {
                    Some(s) => Ok(PilotVal::from_str(s)),
                    None => self.forward(op, args),
                }
            }
            "op_checked" => self.op_checked(args),
            "rt_fault" => Err(self.display(arg(args, 0, op)?)),
            "seam_open" => Err(format!(
                "SEAM001: unconnected seam `{}`: {}",
                str_arg(args, 0, op)?,
                args.get(1).map(|&v| self.display(v)).unwrap_or_default()
            )),
            _ => self.forward(op, args),
        }
    }
}
