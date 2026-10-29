//! Milestone 35.1: WebAssembly & WASI Code Generator.

use std::fmt::Write;

pub struct WasmEmitter {
    out: String,
}

impl Default for WasmEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmEmitter {
    pub fn new() -> Self {
        Self {
            out: String::with_capacity(1024),
        }
    }

    pub fn emit_wat_module(&mut self, module_name: &str, main_ret_val: i64) -> String {
        writeln!(&mut self.out, "(module ;; {}", module_name).unwrap();
        writeln!(&mut self.out, "  (import \"wasi_snapshot_preview1\" \"proc_exit\" (func $proc_exit (param i32)))").unwrap();
        writeln!(&mut self.out, "  (memory (export \"memory\") 1)").unwrap();
        writeln!(&mut self.out, "  (func $main (export \"_start\") (result i64)").unwrap();
        writeln!(&mut self.out, "    i64.const {}", main_ret_val).unwrap();
        writeln!(&mut self.out, "    return").unwrap();
        writeln!(&mut self.out, "  )").unwrap();
        writeln!(&mut self.out, ")").unwrap();
        self.out.clone()
    }

    pub fn finish(self) -> String {
        self.out
    }
}
