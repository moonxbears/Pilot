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
