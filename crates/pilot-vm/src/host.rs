//! Host-VM value conversion (`HostValue`) and script-argument wiring.

use crate::value::PilotVal;
use crate::vm::PilVirtualMachine;

impl PilVirtualMachine {
    /// Sets the arguments after `--` on the command line.
    pub fn set_script_args(&mut self, args: Vec<String>) {
        self.script_args = args;
    }

    /// The elements of a list value.
    pub fn list_items(&self, v: PilotVal) -> Option<Vec<PilotVal>> {
        let slot = v.as_list_handle().filter(|&s| !self.list_is_dead(s))?;
        self.heap_lists.get(slot).cloned()
    }
}

/// A value passed between the VM and its embedding host (`STAGE_A_HOST_NATIVES`).
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

    pub fn from_host(&mut self, v: HostValue) -> PilotVal {
        match v {
            HostValue::Null => PilotVal::NULL,
            HostValue::Bool(b) => PilotVal::from_bool(b),
            HostValue::Int(i) => PilotVal::from_int(i),
            HostValue::Float(f) => PilotVal::from_float(f),
            HostValue::Str(s) => self.new_str(&s),
            HostValue::List(items) => {
                let vals = items.into_iter().map(|i| self.from_host(i)).collect();
                self.new_list(vals)
            }
        }
    }

    pub(crate) fn host_op(
        &mut self,
        op: &str,
        args: &[PilotVal],
    ) -> Option<Result<PilotVal, String>> {
        let mut ops = self.host_ops.take()?;
        let host_args: Vec<HostValue> = args.iter().map(|&a| self.to_host(a)).collect();
        let out = ops(op, &host_args);
        self.host_ops = Some(ops);
        Some(out?.map(|v| self.from_host(v)))
    }
}
