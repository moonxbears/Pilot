//! The result of a milestone test's VM run, as the string the test compares.
//!
//! A run that faults (for example at a seam, `SEAM001`) yields
//! `FAULT: <message>` rather than the `null` the VM returns, so a fault can
//! never be mistaken for a value.
use pilot_vm::PilVirtualMachine;

#[allow(dead_code)]
pub fn outcome(vm: &mut PilVirtualMachine) -> String {
    let value = vm.run();
    match vm.fault.take() {
        Some(fault) => format!("FAULT: {}", fault.message),
        None => value.to_string_repr(),
    }
}
