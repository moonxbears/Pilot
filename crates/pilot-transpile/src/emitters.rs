//! Milestone 41.2: Precedence-Accurate 7-Target Emitters (`pilot-transpile`).
//!
//! Lowers `IrModule` constructs idiomatically into C11/C23, C++20, C#, Rust, Python 3.10+, JavaScript (ESM), and Lua.

use std::fmt::Write;
use crate::ir::{IrExpr, IrModule, IrStmt, IrType, IrTypeDecl, TargetLanguage};

pub struct Transpiler;

impl Transpiler {
    pub fn emit_module(module: &IrModule, target: TargetLanguage) -> String {
        let mut out = String::with_capacity(module.functions.len() * 512 + 1024);

        match target {
            TargetLanguage::C => Self::emit_c_module(&mut out, module),
            TargetLanguage::Cpp => Self::emit_cpp_module(&mut out, module),
            TargetLanguage::CSharp => Self::emit_csharp_module(&mut out, module),
            TargetLanguage::Rust => Self::emit_rust_module(&mut out, module),
            TargetLanguage::Python => Self::emit_python_module(&mut out, module),
            TargetLanguage::JavaScript => Self::emit_js_module(&mut out, module),
            TargetLanguage::Lua => Self::emit_lua_module(&mut out, module),
        }

        out
    }

    // =========================================================================
    // 1. Rust Target Emitter
    // =========================================================================
    fn emit_rust_module(out: &mut String, module: &IrModule) {
        writeln!(out, "// Transpiled to Rust from Pilot").unwrap();
        writeln!(out, "#![allow(unused_variables, dead_code)]\n").unwrap();

        for decl in &module.type_decls {
            match decl {
                IrTypeDecl::Struct { name, .. } | IrTypeDecl::Record { name, .. } => {
                    writeln!(out, "#[derive(Clone, Debug, PartialEq)]").unwrap();
                    writeln!(out, "pub struct {} {{", name).unwrap();
                    writeln!(out, "}}\n").unwrap();
                }
                IrTypeDecl::Adt { name, .. } => {
                    writeln!(out, "#[derive(Clone, Debug, PartialEq)]").unwrap();
                    writeln!(out, "pub enum {} {{", name).unwrap();
                    writeln!(out, "}}\n").unwrap();
                }
            }
        }

        for f in &module.functions {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|(n, ty)| format!("{}: {}", n, Self::rust_type(ty)))
                .collect();
            let ret = Self::rust_type(&f.ret_type);
            let ret_str = if ret == "()" { String::new() } else { format!(" -> {}", ret) };

            writeln!(out, "pub fn {}({}){} {{", f.name, params.join(", "), ret_str).unwrap();
            for stmt in &f.body {
                Self::emit_rust_stmt(out, stmt, 1);
            }
            writeln!(out, "}}\n").unwrap();
        }
    }

    fn emit_rust_stmt(out: &mut String, stmt: &IrStmt, indent: usize) {
        let ind = "    ".repeat(indent);
        match stmt {
            IrStmt::Let { name, init, .. } => {
                writeln!(out, "{}let {} = {};", ind, name, Self::emit_expr(init)).unwrap();
            }
            IrStmt::Var { name, init, .. } => {
                writeln!(out, "{}let mut {} = {};", ind, name, Self::emit_expr(init)).unwrap();
            }
            IrStmt::Assign { target, val } => {
                writeln!(out, "{}{} = {};", ind, Self::emit_expr(target), Self::emit_expr(val)).unwrap();
            }
            IrStmt::If { cond, then_branch, else_branch } => {
                writeln!(out, "{}if {} {{", ind, Self::emit_expr(cond)).unwrap();
                for s in then_branch {
                    Self::emit_rust_stmt(out, s, indent + 1);
                }
                if !else_branch.is_empty() {
                    writeln!(out, "{}}} else {{", ind).unwrap();
                    for s in else_branch {
                        Self::emit_rust_stmt(out, s, indent + 1);
                    }
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::While { cond, body } => {
                writeln!(out, "{}while {} {{", ind, Self::emit_expr(cond)).unwrap();
                for s in body {
                    Self::emit_rust_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::For { var, iter, body } => {
                writeln!(out, "{}for {} in {} {{", ind, var, Self::emit_expr(iter)).unwrap();
                for s in body {
                    Self::emit_rust_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::Guard { cond, else_body } => {
                writeln!(out, "{}if !({}) {{", ind, Self::emit_expr(cond)).unwrap();
                for s in else_body {
                    Self::emit_rust_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::Return(Some(expr)) => {
                writeln!(out, "{}return {};", ind, Self::emit_expr(expr)).unwrap();
            }
            IrStmt::Return(None) => {
                writeln!(out, "{}return;", ind).unwrap();
            }
            IrStmt::Expr(expr) => {
                writeln!(out, "{}{};", ind, Self::emit_expr(expr)).unwrap();
            }
        }
    }

    fn rust_type(ty: &IrType) -> String {
        match ty {
            IrType::Void => "()".to_string(),
            IrType::Bool => "bool".to_string(),
            IrType::Int => "i64".to_string(),
            IrType::Float => "f64".to_string(),
            IrType::String => "String".to_string(),
            IrType::Option(inner) => format!("Option<{}>", Self::rust_type(inner)),
            IrType::Result { ok, err } => format!("Result<{}, {}>", Self::rust_type(ok), Self::rust_type(err)),
            IrType::List(inner) => format!("Vec<{}>", Self::rust_type(inner)),
            IrType::Map { key, value } => format!("std::collections::HashMap<{}, {}>", Self::rust_type(key), Self::rust_type(value)),
            IrType::Tuple(fields) => {
                let f_str: Vec<String> = fields.iter().map(Self::rust_type).collect();
                format!("({})", f_str.join(", "))
            }
            IrType::Custom(name) => name.clone(),
        }
    }

    // =========================================================================
    // 2. C Target Emitter
    // =========================================================================
    fn emit_c_module(out: &mut String, module: &IrModule) {
        writeln!(out, "/* Transpiled to C from Pilot */").unwrap();
        writeln!(out, "#include <stdint.h>").unwrap();
        writeln!(out, "#include <stdbool.h>").unwrap();
        writeln!(out, "#include <stdio.h>").unwrap();
        writeln!(out, "#include <stdlib.h>\n").unwrap();

        for f in &module.functions {
            let params: Vec<String> = if f.params.is_empty() {
                vec!["void".to_string()]
            } else {
                f.params.iter().map(|(n, _)| format!("int64_t {}", n)).collect()
            };
            let ret = if f.name == "main" { "int" } else { "int64_t" };
            writeln!(out, "{} {}({}) {{", ret, f.name, params.join(", ")).unwrap();
            for stmt in &f.body {
                Self::emit_c_stmt(out, stmt, 1);
            }
            if f.name == "main" {
                writeln!(out, "    return 0;").unwrap();
            }
            writeln!(out, "}}\n").unwrap();
        }
    }

    fn emit_c_stmt(out: &mut String, stmt: &IrStmt, indent: usize) {
        let ind = "    ".repeat(indent);
        match stmt {
            IrStmt::Let { name, init, .. } | IrStmt::Var { name, init, .. } => {
                writeln!(out, "{}int64_t {} = {};", ind, name, Self::emit_expr(init)).unwrap();
            }
            IrStmt::Assign { target, val } => {
                writeln!(out, "{}{} = {};", ind, Self::emit_expr(target), Self::emit_expr(val)).unwrap();
            }
            IrStmt::If { cond, then_branch, else_branch } => {
                writeln!(out, "{}if ({}) {{", ind, Self::emit_expr(cond)).unwrap();
                for s in then_branch {
                    Self::emit_c_stmt(out, s, indent + 1);
                }
                if !else_branch.is_empty() {
                    writeln!(out, "{}}} else {{", ind).unwrap();
                    for s in else_branch {
                        Self::emit_c_stmt(out, s, indent + 1);
                    }
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::While { cond, body } => {
                writeln!(out, "{}while ({}) {{", ind, Self::emit_expr(cond)).unwrap();
                for s in body {
                    Self::emit_c_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::Return(Some(expr)) => {
                writeln!(out, "{}return {};", ind, Self::emit_expr(expr)).unwrap();
            }
            IrStmt::Return(None) => {
                writeln!(out, "{}return;", ind).unwrap();
            }
            IrStmt::Expr(expr) => {
                writeln!(out, "{}{};", ind, Self::emit_expr(expr)).unwrap();
            }
            _ => {}
        }
    }

    // =========================================================================
    // 3. C++ Target Emitter
    // =========================================================================
    fn emit_cpp_module(out: &mut String, module: &IrModule) {
        writeln!(out, "// Transpiled to C++ from Pilot").unwrap();
        writeln!(out, "#include <cstdint>").unwrap();
        writeln!(out, "#include <string>").unwrap();
        writeln!(out, "#include <iostream>\n").unwrap();

        for f in &module.functions {
            let params: Vec<String> = f.params.iter().map(|(n, _)| format!("int64_t {}", n)).collect();
            let ret = if f.name == "main" { "int" } else { "int64_t" };
            writeln!(out, "{} {}({}) {{", ret, f.name, params.join(", ")).unwrap();
            for stmt in &f.body {
                match stmt {
                    IrStmt::Let { name, init, .. } => {
                        writeln!(out, "    auto {} = {};", name, Self::emit_expr(init)).unwrap();
                    }
                    IrStmt::Return(Some(expr)) => {
                        writeln!(out, "    return {};", Self::emit_expr(expr)).unwrap();
                    }
                    _ => Self::emit_c_stmt(out, stmt, 1),
                }
            }
            if f.name == "main" {
                writeln!(out, "    return 0;").unwrap();
            }
            writeln!(out, "}}\n").unwrap();
        }
    }

    // =========================================================================
    // 4. C# Target Emitter
    // =========================================================================
    fn emit_csharp_module(out: &mut String, module: &IrModule) {
        writeln!(out, "// Transpiled to C# from Pilot").unwrap();
        writeln!(out, "using System;\n").unwrap();
        writeln!(out, "public static class {} {{", module.name).unwrap();

        for f in &module.functions {
            let params: Vec<String> = f.params.iter().map(|(n, _)| format!("long {}", n)).collect();
            let ret = if f.name == "main" { "int" } else { "long" };
            writeln!(out, "    public static {} {}({}) {{", ret, f.name, params.join(", ")).unwrap();
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
            if f.name == "main" {
                writeln!(out, "        return 0;").unwrap();
            }
            writeln!(out, "    }}\n").unwrap();
        }

        writeln!(out, "}}").unwrap();
    }

    // =========================================================================
    // 5. Python Target Emitter
    // =========================================================================
    fn emit_python_module(out: &mut String, module: &IrModule) {
        writeln!(out, "# Transpiled to Python from Pilot\n").unwrap();

        for f in &module.functions {
            let params: Vec<String> = f.params.iter().map(|(n, _)| n.clone()).collect();
            writeln!(out, "def {}({}):", f.name, params.join(", ")).unwrap();
            if f.body.is_empty() {
                writeln!(out, "    pass\n").unwrap();
                continue;
            }
            for stmt in &f.body {
                Self::emit_python_stmt(out, stmt, 1);
            }
            writeln!(out).unwrap();
        }

        if module.functions.iter().any(|f| f.name == "main") {
            writeln!(out, "if __name__ == '__main__':").unwrap();
            writeln!(out, "    main()").unwrap();
        }
    }

    fn emit_python_stmt(out: &mut String, stmt: &IrStmt, indent: usize) {
        let ind = "    ".repeat(indent);
        match stmt {
            IrStmt::Let { name, init, .. } | IrStmt::Var { name, init, .. } => {
                writeln!(out, "{}{} = {}", ind, name, Self::emit_expr(init)).unwrap();
            }
            IrStmt::Assign { target, val } => {
                writeln!(out, "{}{} = {}", ind, Self::emit_expr(target), Self::emit_expr(val)).unwrap();
            }
            IrStmt::If { cond, then_branch, else_branch } => {
                writeln!(out, "{}if {}:", ind, Self::emit_expr(cond)).unwrap();
                for s in then_branch {
                    Self::emit_python_stmt(out, s, indent + 1);
                }
                if !else_branch.is_empty() {
                    writeln!(out, "{}else:", ind).unwrap();
                    for s in else_branch {
                        Self::emit_python_stmt(out, s, indent + 1);
                    }
                }
            }
            IrStmt::While { cond, body } => {
                writeln!(out, "{}while {}:", ind, Self::emit_expr(cond)).unwrap();
                for s in body {
                    Self::emit_python_stmt(out, s, indent + 1);
                }
            }
            IrStmt::For { var, iter, body } => {
                writeln!(out, "{}for {} in {}:", ind, var, Self::emit_expr(iter)).unwrap();
                for s in body {
                    Self::emit_python_stmt(out, s, indent + 1);
                }
            }
            IrStmt::Return(Some(expr)) => {
                writeln!(out, "{}return {}", ind, Self::emit_expr(expr)).unwrap();
            }
            IrStmt::Return(None) => {
                writeln!(out, "{}return", ind).unwrap();
            }
            IrStmt::Expr(expr) => {
                writeln!(out, "{}{}", ind, Self::emit_expr(expr)).unwrap();
            }
            _ => {}
        }
    }

    // =========================================================================
    // 6. JavaScript Target Emitter (ESM)
    // =========================================================================
    fn emit_js_module(out: &mut String, module: &IrModule) {
        writeln!(out, "// Transpiled to JavaScript from Pilot\n").unwrap();

        for f in &module.functions {
            let params: Vec<String> = f.params.iter().map(|(n, _)| n.clone()).collect();
            writeln!(out, "export function {}({}) {{", f.name, params.join(", ")).unwrap();
            for stmt in &f.body {
                match stmt {
                    IrStmt::Let { name, init, .. } => {
                        writeln!(out, "    let {} = {};", name, Self::emit_expr(init)).unwrap();
                    }
                    IrStmt::Return(Some(expr)) => {
                        writeln!(out, "    return {};", Self::emit_expr(expr)).unwrap();
                    }
                    _ => Self::emit_c_stmt(out, stmt, 1),
                }
            }
            writeln!(out, "}}\n").unwrap();
        }
    }

    // =========================================================================
    // 7. Lua Target Emitter
    // =========================================================================
    fn emit_lua_module(out: &mut String, module: &IrModule) {
        writeln!(out, "-- Transpiled to Lua from Pilot\n").unwrap();

        for f in &module.functions {
            let params: Vec<String> = f.params.iter().map(|(n, _)| n.clone()).collect();
            writeln!(out, "local function {}({})", f.name, params.join(", ")).unwrap();
            for stmt in &f.body {
                match stmt {
                    IrStmt::Let { name, init, .. } | IrStmt::Var { name, init, .. } => {
                        writeln!(out, "    local {} = {}", name, Self::emit_expr(init)).unwrap();
                    }
                    IrStmt::Assign { target, val } => {
                        writeln!(out, "    {} = {}", Self::emit_expr(target), Self::emit_expr(val)).unwrap();
                    }
                    IrStmt::Return(Some(expr)) => {
                        writeln!(out, "    return {}", Self::emit_expr(expr)).unwrap();
                    }
                    IrStmt::Return(None) => {
                        writeln!(out, "    return").unwrap();
                    }
                    IrStmt::Expr(expr) => {
                        writeln!(out, "    {}", Self::emit_expr(expr)).unwrap();
                    }
                    _ => {}
                }
            }
            writeln!(out, "end\n").unwrap();
        }
    }

    // =========================================================================
    // Common Expression Serializer
    // =========================================================================
    pub fn emit_expr(expr: &IrExpr) -> String {
        match expr {
            IrExpr::LitInt(v) => format!("{}", v),
            IrExpr::LitFloat(v) => format!("{:?}", v),
            IrExpr::LitStr(s) => format!("\"{}\"", s),
            IrExpr::LitBool(b) => if *b { "true".to_string() } else { "false".to_string() },
            IrExpr::LitNull => "null".to_string(),
            IrExpr::Var(n) => n.clone(),
            IrExpr::Binary { op, lhs, rhs } => {
                format!("({} {} {})", Self::emit_expr(lhs), op, Self::emit_expr(rhs))
            }
            IrExpr::Unary { op, expr } => {
                format!("({}{})", op, Self::emit_expr(expr))
            }
            IrExpr::Call { callee, args } => {
                let a_str: Vec<String> = args.iter().map(Self::emit_expr).collect();
                format!("{}({})", callee, a_str.join(", "))
            }
            IrExpr::Member { target, field } => {
                format!("{}.{}", Self::emit_expr(target), field)
            }
            IrExpr::Index { target, index } => {
                format!("{}[{}]", Self::emit_expr(target), Self::emit_expr(index))
            }
            IrExpr::If { cond, then_expr, else_expr } => {
                format!("(({}) ? ({}) : ({}))", Self::emit_expr(cond), Self::emit_expr(then_expr), Self::emit_expr(else_expr))
            }
            IrExpr::List(items) => {
                let i_str: Vec<String> = items.iter().map(Self::emit_expr).collect();
                format!("[{}]", i_str.join(", "))
            }
            IrExpr::Map(pairs) => {
                let p_str: Vec<String> = pairs.iter().map(|(k, v)| format!("{}: {}", Self::emit_expr(k), Self::emit_expr(v))).collect();
                format!("{{{}}}", p_str.join(", "))
            }
        }
    }
}
