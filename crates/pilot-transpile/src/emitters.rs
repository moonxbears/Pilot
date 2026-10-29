//! Milestone 35.2: Emitters for C, C++, C#, Rust, Python, JavaScript, and Lua (> 100 MB/s).

use std::fmt::Write;
use crate::ir::{IrExpr, IrFunction, IrModule, IrStmt, TargetLanguage};

pub struct Transpiler;

impl Transpiler {
    pub fn emit_module(module: &IrModule, target: TargetLanguage) -> String {
        let mut out = String::with_capacity(1024);

        match target {
            TargetLanguage::C => {
                writeln!(&mut out, "/* Transpiled to C from Pilot */").unwrap();
                writeln!(&mut out, "#include <stdint.h>").unwrap();
                writeln!(&mut out, "#include <stdbool.h>").unwrap();
                writeln!(&mut out).unwrap();
                for f in &module.functions {
                    Self::emit_c_function(&mut out, f);
                }
            }
            TargetLanguage::Cpp => {
                writeln!(&mut out, "// Transpiled to C++ from Pilot").unwrap();
                writeln!(&mut out, "#include <cstdint>").unwrap();
                writeln!(&mut out, "#include <string>").unwrap();
                writeln!(&mut out).unwrap();
                for f in &module.functions {
                    Self::emit_cpp_function(&mut out, f);
                }
            }
            TargetLanguage::CSharp => {
                writeln!(&mut out, "// Transpiled to C# from Pilot").unwrap();
                writeln!(&mut out, "using System;").unwrap();
                writeln!(&mut out, "public static class {} {{", module.name).unwrap();
                for f in &module.functions {
                    Self::emit_csharp_function(&mut out, f);
                }
                writeln!(&mut out, "}}").unwrap();
            }
            TargetLanguage::Rust => {
                writeln!(&mut out, "// Transpiled to Rust from Pilot").unwrap();
                writeln!(&mut out).unwrap();
                for f in &module.functions {
                    Self::emit_rust_function(&mut out, f);
                }
            }
            TargetLanguage::Python => {
                writeln!(&mut out, "# Transpiled to Python from Pilot").unwrap();
                writeln!(&mut out).unwrap();
                for f in &module.functions {
                    Self::emit_python_function(&mut out, f);
                }
            }
            TargetLanguage::JavaScript => {
                writeln!(&mut out, "// Transpiled to JavaScript from Pilot").unwrap();
                writeln!(&mut out).unwrap();
                for f in &module.functions {
                    Self::emit_js_function(&mut out, f);
                }
            }
            TargetLanguage::Lua => {
                writeln!(&mut out, "-- Transpiled to Lua from Pilot").unwrap();
                writeln!(&mut out).unwrap();
                for f in &module.functions {
                    Self::emit_lua_function(&mut out, f);
                }
            }
        }

        out
    }

    fn emit_rust_function(out: &mut String, f: &IrFunction) {
        let params: Vec<String> = f.params.iter().map(|(n, _)| format!("{}: i64", n)).collect();
        writeln!(out, "pub fn {}({}) -> i64 {{", f.name, params.join(", ")).unwrap();
        for stmt in &f.body {
            match stmt {
                IrStmt::Let { name, init, .. } => {
                    writeln!(out, "    let {} = {};", name, Self::emit_expr(init)).unwrap();
                }
                IrStmt::Return(Some(expr)) => {
                    writeln!(out, "    return {};", Self::emit_expr(expr)).unwrap();
                }
                _ => {}
            }
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    fn emit_c_function(out: &mut String, f: &IrFunction) {
        let params: Vec<String> = f.params.iter().map(|(n, _)| format!("int64_t {}", n)).collect();
        writeln!(out, "int64_t {}({}) {{", f.name, params.join(", ")).unwrap();
        for stmt in &f.body {
            match stmt {
                IrStmt::Let { name, init, .. } => {
                    writeln!(out, "    int64_t {} = {};", name, Self::emit_expr(init)).unwrap();
                }
                IrStmt::Return(Some(expr)) => {
                    writeln!(out, "    return {};", Self::emit_expr(expr)).unwrap();
                }
                _ => {}
            }
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    fn emit_cpp_function(out: &mut String, f: &IrFunction) {
        let params: Vec<String> = f.params.iter().map(|(n, _)| format!("int64_t {}", n)).collect();
        writeln!(out, "int64_t {}({}) {{", f.name, params.join(", ")).unwrap();
        for stmt in &f.body {
            match stmt {
                IrStmt::Let { name, init, .. } => {
                    writeln!(out, "    auto {} = {};", name, Self::emit_expr(init)).unwrap();
                }
                IrStmt::Return(Some(expr)) => {
                    writeln!(out, "    return {};", Self::emit_expr(expr)).unwrap();
                }
                _ => {}
            }
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    fn emit_csharp_function(out: &mut String, f: &IrFunction) {
        let params: Vec<String> = f.params.iter().map(|(n, _)| format!("long {}", n)).collect();
        writeln!(out, "    public static long {}({}) {{", f.name, params.join(", ")).unwrap();
        for stmt in &f.body {
            match stmt {
                IrStmt::Let { name, init, .. } => {
                    writeln!(out, "        var {} = {};", name, Self::emit_expr(init)).unwrap();
                }
                IrStmt::Return(Some(expr)) => {
                    writeln!(out, "        return {};", Self::emit_expr(expr)).unwrap();
                }
                _ => {}
            }
        }
        writeln!(out, "    }}").unwrap();
    }

    fn emit_python_function(out: &mut String, f: &IrFunction) {
        let params: Vec<String> = f.params.iter().map(|(n, _)| n.clone()).collect();
        writeln!(out, "def {}({}):", f.name, params.join(", ")).unwrap();
        for stmt in &f.body {
            match stmt {
                IrStmt::Let { name, init, .. } => {
                    writeln!(out, "    {} = {}", name, Self::emit_expr(init)).unwrap();
                }
                IrStmt::Return(Some(expr)) => {
                    writeln!(out, "    return {}", Self::emit_expr(expr)).unwrap();
                }
                _ => {}
            }
        }
        writeln!(out).unwrap();
    }

    fn emit_js_function(out: &mut String, f: &IrFunction) {
        let params: Vec<String> = f.params.iter().map(|(n, _)| n.clone()).collect();
        writeln!(out, "export function {}({}) {{", f.name, params.join(", ")).unwrap();
        for stmt in &f.body {
            match stmt {
                IrStmt::Let { name, init, .. } => {
                    writeln!(out, "    const {} = {};", name, Self::emit_expr(init)).unwrap();
                }
                IrStmt::Return(Some(expr)) => {
                    writeln!(out, "    return {};", Self::emit_expr(expr)).unwrap();
                }
                _ => {}
            }
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    fn emit_lua_function(out: &mut String, f: &IrFunction) {
        let params: Vec<String> = f.params.iter().map(|(n, _)| n.clone()).collect();
        writeln!(out, "local function {}({})", f.name, params.join(", ")).unwrap();
        for stmt in &f.body {
            match stmt {
                IrStmt::Let { name, init, .. } => {
                    writeln!(out, "    local {} = {}", name, Self::emit_expr(init)).unwrap();
                }
                IrStmt::Return(Some(expr)) => {
                    writeln!(out, "    return {}", Self::emit_expr(expr)).unwrap();
                }
                _ => {}
            }
        }
        writeln!(out, "end").unwrap();
        writeln!(out).unwrap();
    }

    fn emit_expr(expr: &IrExpr) -> String {
        match expr {
            IrExpr::LitInt(i) => i.to_string(),
            IrExpr::LitFloat(f) => f.to_string(),
            IrExpr::LitStr(s) => format!("\"{}\"", s),
            IrExpr::LitBool(b) => if *b { "true" } else { "false" }.to_string(),
            IrExpr::Var(v) => v.clone(),
            IrExpr::Binary { op, lhs, rhs } => {
                format!("({} {} {})", Self::emit_expr(lhs), op, Self::emit_expr(rhs))
            }
            IrExpr::Call { callee, args } => {
                let a: Vec<String> = args.iter().map(Self::emit_expr).collect();
                format!("{}({})", callee, a.join(", "))
            }
        }
    }
}
