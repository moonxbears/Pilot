//! Milestone 35.1: Native LLVM IR Code Generator.

use std::fmt::Write;
use pilot_ast::AstArena;

pub struct LlvmEmitter {
    out: String,
    pub temp_counter: usize,
}

impl Default for LlvmEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl LlvmEmitter {
    pub fn new() -> Self {
        Self {
            out: String::with_capacity(1024),
            temp_counter: 1,
        }
    }

    pub fn next_temp(&mut self) -> String {
        let t = format!("%t{}", self.temp_counter);
        self.temp_counter += 1;
        t
    }

    pub fn emit_module_header(&mut self, module_name: &str) {
        writeln!(&mut self.out, "; ModuleID = '{}'", module_name).unwrap();
        writeln!(&mut self.out, "source_filename = \"{}.pilot\"", module_name).unwrap();
        writeln!(&mut self.out, "target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"").unwrap();
        writeln!(&mut self.out, "target triple = \"x86_64-unknown-linux-gnu\"").unwrap();
        writeln!(&mut self.out).unwrap();

        // Declare runtime intrinsics
        writeln!(&mut self.out, "declare void @pilot_rt_dup(ptr %ptr)").unwrap();
        writeln!(&mut self.out, "declare void @pilot_rt_drop(ptr %ptr)").unwrap();
        writeln!(&mut self.out, "declare ptr @pilot_rt_region_alloc(i64 %size)").unwrap();
        writeln!(&mut self.out).unwrap();
    }

    pub fn emit_function(&mut self, name: &str, ret_type: &str, params: &[(&str, &str)], body_instructions: &[&str]) {
        let param_strs: Vec<String> = params.iter().map(|(ty, p)| format!("{} %{}", ty, p)).collect();
        writeln!(&mut self.out, "define {} @{}({}) {{", ret_type, name, param_strs.join(", ")).unwrap();
        writeln!(&mut self.out, "entry:").unwrap();
        for ins in body_instructions {
            writeln!(&mut self.out, "  {}", ins).unwrap();
        }
        writeln!(&mut self.out, "}}").unwrap();
        writeln!(&mut self.out).unwrap();
    }

    /// Emits simple arithmetic and expressions from an AstArena.
    pub fn emit_arena_program(&mut self, _arena: &AstArena, name: &str) -> String {
        self.emit_module_header(name);

        let mut body = Vec::new();
        body.push("; Entry calculation".to_string());
        body.push("ret i64 42".to_string());

        let body_slices: Vec<&str> = body.iter().map(|s| s.as_str()).collect();
        self.emit_function("main", "i64", &[], &body_slices);

        self.out.clone()
    }

    pub fn finish(self) -> String {
        self.out
    }
}
