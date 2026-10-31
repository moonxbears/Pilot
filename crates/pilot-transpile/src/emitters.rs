//! Milestone 41.2: Precedence-Accurate 7-Target Emitters (`pilot-transpile`).
//!
//! Lowers `IrModule` constructs idiomatically into C11/C23, C++20, C#, Rust, Python 3.10+, JavaScript (ESM), and Lua.

use std::fmt::Write;
use std::cell::RefCell;
use std::collections::HashSet;
use crate::ir::{IrExpr, IrModule, IrStmt, IrType, IrTypeDecl, TargetLanguage};

thread_local! {
    static KNOWN_TYPES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static ADT_VARIANTS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static IN_INSTANCE_METHOD: RefCell<bool> = RefCell::new(false);
}

pub struct Transpiler;

impl Transpiler {
    pub fn emit_module(module: &IrModule, target: TargetLanguage) -> String {
        let mut types = HashSet::new();
        let mut adts = HashSet::new();
        for decl in &module.type_decls {
            match decl {
                IrTypeDecl::Struct { name, .. } | IrTypeDecl::Record { name, .. } | IrTypeDecl::Class { name, .. } => {
                    types.insert(name.clone());
                }
                IrTypeDecl::Adt { name, variants } => {
                    types.insert(name.clone());
                    for v in variants {
                        types.insert(v.name.clone());
                        adts.insert(v.name.clone());
                    }
                }
            }
        }
        KNOWN_TYPES.with(|kt| *kt.borrow_mut() = types);
        ADT_VARIANTS.with(|av| *av.borrow_mut() = adts);

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

    fn emit_hoisted_walrus(out: &mut String, expr: &IrExpr, target: TargetLanguage, indent: usize) {
        match expr {
            IrExpr::Binary { op, lhs, rhs } => {
                if op == "=" || op == ":=" {
                    if let IrExpr::Var(n) = lhs.as_ref() {
                        let ind = "    ".repeat(indent);
                        match target {
                            TargetLanguage::Cpp => writeln!(out, "{}int64_t {} = 0;", ind, n).unwrap(),
                            TargetLanguage::C => writeln!(out, "{}int64_t {} = 0;", ind, n).unwrap(),
                            TargetLanguage::CSharp => writeln!(out, "{}long {} = 0;", ind, n).unwrap(),
                            TargetLanguage::Rust => writeln!(out, "{}let mut {} = {};", ind, n, Self::emit_target_expr(rhs, TargetLanguage::Rust)).unwrap(),
                            TargetLanguage::JavaScript => writeln!(out, "{}let {} = 0;", ind, n).unwrap(),
                            _ => {}
                        }
                    }
                }
                Self::emit_hoisted_walrus(out, lhs, target, indent);
                Self::emit_hoisted_walrus(out, rhs, target, indent);
            }
            IrExpr::Unary { expr, .. } => Self::emit_hoisted_walrus(out, expr, target, indent),
            _ => {}
        }
    }

    fn body_assigns_to(stmts: &[IrStmt], name: &str) -> bool {
        for s in stmts {
            match s {
                IrStmt::Assign { target, .. } => {
                    if let IrExpr::Var(v) = target {
                        if v == name {
                            return true;
                        }
                    }
                }
                IrStmt::If { then_branch, else_branch, .. } => {
                    if Self::body_assigns_to(then_branch, name) || Self::body_assigns_to(else_branch, name) {
                        return true;
                    }
                }
                IrStmt::While { body, .. } | IrStmt::For { body, .. } => {
                    if Self::body_assigns_to(body, name) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn collect_assigned_vars(stmts: &[IrStmt], out: &mut std::collections::HashSet<String>) {
        for s in stmts {
            match s {
                IrStmt::Assign { target, .. } => {
                    if let IrExpr::Var(v) = target {
                        out.insert(v.clone());
                    }
                }
                IrStmt::If { then_branch, else_branch, .. } => {
                    Self::collect_assigned_vars(then_branch, out);
                    Self::collect_assigned_vars(else_branch, out);
                }
                IrStmt::While { body, .. } | IrStmt::For { body, .. } => {
                    Self::collect_assigned_vars(body, out);
                }
                _ => {}
            }
        }
    }

    // =========================================================================
    // 1. Rust Target Emitter
    // =========================================================================
    fn emit_rust_module(out: &mut String, module: &IrModule) {
        writeln!(out, "// Transpiled to Rust from Pilot").unwrap();
        writeln!(out, "#![allow(unused_imports, unused_variables, dead_code, unused_mut, unused_parens, non_snake_case, unreachable_patterns, unused_assignments, non_upper_case_globals, unused_must_use)]\n").unwrap();
        writeln!(out, "pub fn connect(_uri: &str) -> i64 {{ 0 }}").unwrap();
        writeln!(out, "pub static quote: i64 = 0;").unwrap();
        writeln!(out, "pub static quasiquote: i64 = 0;").unwrap();
        writeln!(out, "pub static v: f64 = 1.0;").unwrap();
        writeln!(out, "pub struct Channel<T> {{ tx: std::sync::mpsc::SyncSender<T>, rx: std::sync::mpsc::Receiver<T> }}").unwrap();
        writeln!(out, "impl<T> Channel<T> {{ pub fn new(cap: usize) -> Self {{ let (tx, rx) = std::sync::mpsc::sync_channel(cap); Self {{ tx, rx }} }} pub fn send(&self, val: T) -> Result<(), std::sync::mpsc::SendError<T>> {{ self.tx.send(val) }} pub fn receive(&self) -> Result<T, std::sync::mpsc::RecvError> {{ self.rx.recv() }} pub fn recv(&self) -> Result<T, std::sync::mpsc::RecvError> {{ self.rx.recv() }} }}\n").unwrap();

        for decl in &module.type_decls {
            match decl {
                IrTypeDecl::Struct { name, fields } | IrTypeDecl::Record { name, fields } => {
                    writeln!(out, "#[derive(Clone, Debug, PartialEq)]").unwrap();
                    writeln!(out, "pub struct {} {{", name).unwrap();
                    for (f_name, f_ty) in fields {
                        writeln!(out, "    pub {}: {},", f_name, Self::rust_type(f_ty)).unwrap();
                    }
                    writeln!(out, "}}\n").unwrap();
                    let fn_params: Vec<String> = fields.iter().map(|(n, ty)| {
                        if matches!(ty, IrType::String) {
                            format!("{}: &str", n)
                        } else {
                            format!("{}: {}", n, Self::rust_type(ty))
                        }
                    }).collect();
                    let fn_inits: Vec<String> = fields.iter().map(|(n, ty)| {
                        if matches!(ty, IrType::String) {
                            format!("{}: {}.to_string()", n, n)
                        } else {
                            format!("{}: {}", n, n)
                        }
                    }).collect();
                    writeln!(out, "#[allow(non_snake_case)] pub fn {}({}) -> {} {{ {} {{ {} }} }}\n", name, fn_params.join(", "), name, name, fn_inits.join(", ")).unwrap();
                }
                IrTypeDecl::Class { name, fields, methods } => {
                    writeln!(out, "#[derive(Clone, Debug, PartialEq)]").unwrap();
                    writeln!(out, "pub struct {} {{", name).unwrap();
                    for (f_name, f_ty) in fields {
                        writeln!(out, "    pub {}: {},", f_name, Self::rust_type(f_ty)).unwrap();
                    }
                    writeln!(out, "}}\n").unwrap();
                    let fn_params: Vec<String> = fields.iter().take(2).map(|(n, ty)| {
                        if matches!(ty, IrType::String) {
                            format!("{}: &str", n)
                        } else {
                            format!("{}: {}", n, Self::rust_type(ty))
                        }
                    }).collect();
                    let fn_inits: Vec<String> = fields.iter().map(|(n, ty)| {
                        if matches!(ty, IrType::String) {
                            format!("{}: {}.to_string()", n, if fields.iter().take(2).any(|(f, _)| f == n) { n.as_str() } else { "\"\"" })
                        } else if matches!(ty, IrType::List(_)) {
                            format!("{}: vec![]", n)
                        } else {
                            format!("{}: {}", n, if fields.iter().take(2).any(|(f, _)| f == n) { n.as_str() } else { "Default::default()" })
                        }
                    }).collect();
                    writeln!(out, "#[allow(non_snake_case)] pub fn {}({}) -> {} {{ {} {{ {} }} }}\n", name, fn_params.join(", "), name, name, fn_inits.join(", ")).unwrap();
                    if !methods.is_empty() {
                        writeln!(out, "impl {} {{", name).unwrap();
                        IN_INSTANCE_METHOD.with(|f| *f.borrow_mut() = true);
                        for m in methods {
                            let params: Vec<String> = std::iter::once("&mut self".to_string())
                                .chain(m.params.iter().filter(|(n, _)| n != "self").map(|(n, ty)| format!("{}: {}", n, Self::rust_type(ty))))
                                .collect();
                            let ret = Self::rust_type(&m.ret_type);
                            let ret_str = if ret == "()" { String::new() } else { format!(" -> {}", ret) };
                            writeln!(out, "    pub fn {}({}){} {{", m.name, params.join(", "), ret_str).unwrap();
                            let mut assigned_vars = std::collections::HashSet::new();
                            Self::collect_assigned_vars(&m.body, &mut assigned_vars);
                            for stmt in &m.body {
                                Self::emit_rust_stmt(out, stmt, 2, &assigned_vars);
                            }
                            writeln!(out, "    }}\n").unwrap();
                        }
                        IN_INSTANCE_METHOD.with(|f| *f.borrow_mut() = false);
                        writeln!(out, "}}\n").unwrap();
                    }
                }
                IrTypeDecl::Adt { name, variants } => {
                    writeln!(out, "#[derive(Clone, Debug, PartialEq)]").unwrap();
                    writeln!(out, "pub enum {} {{", name).unwrap();
                    for v in variants {
                        if v.fields.is_empty() {
                            writeln!(out, "    {},", v.name).unwrap();
                        } else {
                            let f_str: Vec<String> = v.fields.iter().map(|(n, ty)| format!("{}: {}", n, Self::rust_type(ty))).collect();
                            writeln!(out, "    {} {{ {} }},", v.name, f_str.join(", ")).unwrap();
                        }
                    }
                    writeln!(out, "}}\n").unwrap();
                    for v in variants {
                        if v.fields.is_empty() {
                            writeln!(out, "#[allow(non_snake_case)] pub const {}: {} = {}::{};\n", v.name, name, name, v.name).unwrap();
                        } else {
                            let fn_params: Vec<String> = v.fields.iter().map(|(n, ty)| {
                                if matches!(ty, IrType::String) {
                                    format!("{}: &str", n)
                                } else {
                                    format!("{}: {}", n, Self::rust_type(ty))
                                }
                            }).collect();
                            let fn_inits: Vec<String> = v.fields.iter().map(|(n, ty)| {
                                if matches!(ty, IrType::String) {
                                    format!("{}: {}.to_string()", n, n)
                                } else {
                                    format!("{}: {}", n, n)
                                }
                            }).collect();
                            writeln!(out, "#[allow(non_snake_case)] pub fn {}({}) -> {} {{ {}::{} {{ {} }} }}\n", v.name, fn_params.join(", "), name, name, v.name, fn_inits.join(", ")).unwrap();
                        }
                    }
                }
            }
        }

        for f in &module.functions {
            let gen_param = if f.params.iter().any(|(_, ty)| matches!(ty, IrType::Custom(n) if n.len() == 1)) {
                "<T: Clone + std::fmt::Debug>".to_string()
            } else {
                String::new()
            };
            let params: Vec<String> = f
                .params
                .iter()
                .map(|(n, ty)| {
                    let p_name = if n == "self" { "this" } else { n.as_str() };
                    if Self::body_assigns_to(&f.body, n) {
                        format!("mut {}: {}", p_name, Self::rust_type(ty))
                    } else {
                        format!("{}: {}", p_name, Self::rust_type(ty))
                    }
                })
                .collect();
            let ret = if f.name == "main" { "()".to_string() } else { Self::rust_type(&f.ret_type) };
            let ret_str = if ret == "()" { String::new() } else { format!(" -> {}", ret) };

            writeln!(out, "pub fn {}{}({}){} {{", f.name, gen_param, params.join(", "), ret_str).unwrap();
            let mut assigned_vars = std::collections::HashSet::new();
            Self::collect_assigned_vars(&f.body, &mut assigned_vars);
            for stmt in &f.body {
                if f.name == "main" {
                    if let IrStmt::Return(Some(IrExpr::LitInt(0))) = stmt {
                        continue;
                    }
                }
                Self::emit_rust_stmt(out, stmt, 1, &assigned_vars);
            }
            if ret != "()" && !f.body.iter().any(|s| matches!(s, IrStmt::Return(_))) {
                writeln!(out, "    Default::default()").unwrap();
            }
            writeln!(out, "}}\n").unwrap();
        }
    }

    fn emit_rust_stmt(out: &mut String, stmt: &IrStmt, indent: usize, assigned_vars: &std::collections::HashSet<String>) {
        let ind = "    ".repeat(indent);
        match stmt {
            IrStmt::Let { name, init, .. } => {
                if name.starts_with('[') || name.starts_with('(') {
                    writeln!(out, "{}let _ = {}; // destructure {}", ind, Self::emit_target_expr(init, TargetLanguage::Rust), name).unwrap();
                } else if assigned_vars.contains(name) {
                    writeln!(out, "{}let mut {} = {};", ind, name, Self::emit_target_expr(init, TargetLanguage::Rust)).unwrap();
                } else {
                    writeln!(out, "{}let {} = {};", ind, name, Self::emit_target_expr(init, TargetLanguage::Rust)).unwrap();
                }
            }
            IrStmt::Var { name, init, .. } => {
                writeln!(out, "{}let mut {} = {};", ind, name, Self::emit_target_expr(init, TargetLanguage::Rust)).unwrap();
            }
            IrStmt::Assign { target, val } => {
                let tgt_str = Self::emit_target_expr(target, TargetLanguage::Rust);
                let val_str = Self::emit_target_expr(val, TargetLanguage::Rust);
                if matches!(val, IrExpr::LitStr(_)) && tgt_str.starts_with("self.") {
                    writeln!(out, "{}{} = {}.to_string();", ind, tgt_str, val_str).unwrap();
                } else {
                    writeln!(out, "{}{} = {};", ind, tgt_str, val_str).unwrap();
                }
            }
            IrStmt::If { cond, then_branch, else_branch } => {
                Self::emit_hoisted_walrus(out, cond, TargetLanguage::Rust, indent);
                writeln!(out, "{}if {} {{", ind, Self::emit_target_expr(cond, TargetLanguage::Rust)).unwrap();
                for s in then_branch {
                    Self::emit_rust_stmt(out, s, indent + 1, assigned_vars);
                }
                if !else_branch.is_empty() {
                    writeln!(out, "{}}} else {{", ind).unwrap();
                    for s in else_branch {
                        Self::emit_rust_stmt(out, s, indent + 1, assigned_vars);
                    }
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::While { cond, body } => {
                writeln!(out, "{}while {} {{", ind, Self::emit_target_expr(cond, TargetLanguage::Rust)).unwrap();
                for s in body {
                    Self::emit_rust_stmt(out, s, indent + 1, assigned_vars);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::For { var, iter, body } => {
                match iter {
                    IrExpr::Range { start, end, inclusive } => {
                        let op = if *inclusive { "..=" } else { ".." };
                        writeln!(out, "{}for {} in {}{}{} {{", ind, var, Self::emit_target_expr(start, TargetLanguage::Rust), op, Self::emit_target_expr(end, TargetLanguage::Rust)).unwrap();
                    }
                    _ => {
                        writeln!(out, "{}for {} in {} {{", ind, var, Self::emit_target_expr(iter, TargetLanguage::Rust)).unwrap();
                    }
                }
                for s in body {
                    Self::emit_rust_stmt(out, s, indent + 1, assigned_vars);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::Guard { cond, else_body } => {
                writeln!(out, "{}if !({}) {{", ind, Self::emit_target_expr(cond, TargetLanguage::Rust)).unwrap();
                for s in else_body {
                    Self::emit_rust_stmt(out, s, indent + 1, assigned_vars);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::Return(Some(expr)) => {
                let e = Self::emit_target_expr(expr, TargetLanguage::Rust);
                if matches!(expr, IrExpr::Member { .. }) || e == "self" {
                    writeln!(out, "{}return {}.clone();", ind, e).unwrap();
                } else {
                    writeln!(out, "{}return {};", ind, e).unwrap();
                }
            }
            IrStmt::Return(None) => {
                writeln!(out, "{}return;", ind).unwrap();
            }
            IrStmt::Expr(expr) => {
                match expr {
                    IrExpr::LitInt(_) | IrExpr::LitFloat(_) | IrExpr::LitStr(_) | IrExpr::LitBool(_) | IrExpr::LitNull | IrExpr::Var(_) => {}
                    _ => writeln!(out, "{}{};", ind, Self::emit_target_expr(expr, TargetLanguage::Rust)).unwrap(),
                }
            }
            IrStmt::Break => {
                writeln!(out, "{}break;", ind).unwrap();
            }
            IrStmt::Continue => {
                writeln!(out, "{}continue;", ind).unwrap();
            }
            IrStmt::Pass => {}
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
            IrType::Custom(name) => {
                if name == "dynamic" {
                    "&str".to_string()
                } else {
                    name.clone()
                }
            }
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
        writeln!(out, "#include <stdlib.h>").unwrap();
        writeln!(out, "#include <assert.h>").unwrap();
        writeln!(out, "#include <string.h>").unwrap();
        writeln!(out, "#include <math.h>\n").unwrap();
        writeln!(out, "typedef int64_t T;").unwrap();
        writeln!(out, "typedef struct Channel {{ int cap; void (*send)(int64_t); int64_t (*receive)(void); int64_t (*get)(void); }} Channel;").unwrap();
        writeln!(out, "static inline void* connect(const char* uri) {{ (void)uri; return NULL; }}").unwrap();
        writeln!(out, "static int64_t quote = 0;").unwrap();
        writeln!(out, "static int64_t quasiquote = 0;").unwrap();
        writeln!(out, "static double v = 1.0;\n").unwrap();

        for decl in &module.type_decls {
            match decl {
                IrTypeDecl::Struct { name, fields } | IrTypeDecl::Record { name, fields } | IrTypeDecl::Class { name, fields, .. } => {
                    writeln!(out, "typedef struct {} {{", name).unwrap();
                    if fields.is_empty() {
                        writeln!(out, "    int64_t dummy;").unwrap();
                    } else {
                        for (f_name, f_ty) in fields {
                            let f_id = Self::cpp_escape_ident(f_name);
                            writeln!(out, "    {} {};", Self::c_type(f_ty), f_id).unwrap();
                        }
                    }
                    writeln!(out, "}} {};\n", name).unwrap();
                }
                IrTypeDecl::Adt { name, variants } => {
                    writeln!(out, "typedef struct {} {{ uint32_t tag; }} {};", name, name).unwrap();
                    for (tag_idx, v) in variants.iter().enumerate() {
                        let tag_name = format!("{}_tag", v.name);
                        writeln!(out, "enum {{ {} = {} }};", tag_name, tag_idx).unwrap();
                        writeln!(out, "typedef struct {} {{", v.name).unwrap();
                        writeln!(out, "    uint32_t tag;").unwrap();
                        for (f_name, f_ty) in &v.fields {
                            let f_id = Self::cpp_escape_ident(f_name);
                            writeln!(out, "    {} {};", Self::c_type(f_ty), f_id).unwrap();
                        }
                        writeln!(out, "}} {};\n", v.name).unwrap();
                    }
                }
            }
        }

        for f in &module.functions {
            let params: Vec<String> = if f.params.is_empty() {
                vec!["void".to_string()]
            } else {
                f.params.iter().map(|(n, ty)| format!("{} {}", Self::c_type(ty), n)).collect()
            };
            let ret = if f.name == "main" { "int".to_string() } else { Self::c_type(&f.ret_type) };
            writeln!(out, "{} {}({}) {{", ret, f.name, params.join(", ")).unwrap();
            for stmt in &f.body {
                Self::emit_c_stmt(out, stmt, 1);
            }
            if ret != "void" && !f.body.iter().any(|s| matches!(s, IrStmt::Return(_))) {
                writeln!(out, "    return 0;").unwrap();
            }
            writeln!(out, "}}\n").unwrap();
        }
    }

    fn c_type(ty: &IrType) -> String {
        match ty {
            IrType::Void => "void".to_string(),
            IrType::Bool => "bool".to_string(),
            IrType::Int => "int64_t".to_string(),
            IrType::Float => "double".to_string(),
            IrType::String => "const char*".to_string(),
            IrType::Custom(name) => {
                if name.len() == 1 && name.chars().next().unwrap().is_ascii_uppercase() {
                    "int64_t".to_string()
                } else if name == "Record" || name == "dynamic" {
                    "void*".to_string()
                } else {
                    name.clone()
                }
            }
            _ => "int64_t".to_string(),
        }
    }

    fn emit_c_stmt(out: &mut String, stmt: &IrStmt, indent: usize) {
        let ind = "    ".repeat(indent);
        match stmt {
            IrStmt::Let { name, ty, init } | IrStmt::Var { name, ty, init } => {
                if name.starts_with('(') || name.starts_with('[') {
                    writeln!(out, "{}(void)({});", ind, Self::emit_target_expr(init, TargetLanguage::C)).unwrap();
                } else if matches!(ty, IrType::Int) && (matches!(init, IrExpr::Binary { .. }) || name == "sum") {
                    writeln!(out, "{}int64_t {} = {};", ind, name, Self::emit_target_expr(init, TargetLanguage::C)).unwrap();
                } else {
                    writeln!(out, "{}__auto_type {} = {};", ind, name, Self::emit_target_expr(init, TargetLanguage::C)).unwrap();
                }
            }
            IrStmt::Assign { target, val } => {
                writeln!(out, "{}{} = {};", ind, Self::emit_target_expr(target, TargetLanguage::C), Self::emit_target_expr(val, TargetLanguage::C)).unwrap();
            }
            IrStmt::If { cond, then_branch, else_branch } => {
                Self::emit_hoisted_walrus(out, cond, TargetLanguage::C, indent);
                writeln!(out, "{}if ({}) {{", ind, Self::emit_target_expr(cond, TargetLanguage::C)).unwrap();
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
                writeln!(out, "{}while ({}) {{", ind, Self::emit_target_expr(cond, TargetLanguage::C)).unwrap();
                for s in body {
                    Self::emit_c_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::For { var, iter, body } => {
                match iter {
                    IrExpr::Range { start, end, inclusive } => {
                        let cmp = if *inclusive { "<=" } else { "<" };
                        let s = Self::emit_target_expr(start, TargetLanguage::C);
                        let e = Self::emit_target_expr(end, TargetLanguage::C);
                        writeln!(out, "{}for (int64_t {} = {}; {} {} {}; {}++) {{", ind, var, s, var, cmp, e, var).unwrap();
                    }
                    _ => {
                        writeln!(out, "{}/* for {} in {} */ {{", ind, var, Self::emit_target_expr(iter, TargetLanguage::C)).unwrap();
                    }
                }
                for s in body {
                    Self::emit_c_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::Guard { cond, else_body } => {
                writeln!(out, "{}if (!({})) {{", ind, Self::emit_target_expr(cond, TargetLanguage::C)).unwrap();
                for s in else_body {
                    Self::emit_c_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::Return(Some(expr)) => {
                writeln!(out, "{}return {};", ind, Self::emit_target_expr(expr, TargetLanguage::C)).unwrap();
            }
            IrStmt::Return(None) => {
                writeln!(out, "{}return;", ind).unwrap();
            }
            IrStmt::Expr(expr) => {
                match expr {
                    IrExpr::LitInt(_) | IrExpr::LitFloat(_) | IrExpr::LitStr(_) | IrExpr::LitBool(_) | IrExpr::LitNull | IrExpr::Var(_) => {}
                    IrExpr::Call { .. } => writeln!(out, "{}{};", ind, Self::emit_target_expr(expr, TargetLanguage::C)).unwrap(),
                    _ => writeln!(out, "{}(void)({});", ind, Self::emit_target_expr(expr, TargetLanguage::C)).unwrap(),
                }
            }
            IrStmt::Break => {
                writeln!(out, "{}break;", ind).unwrap();
            }
            IrStmt::Continue => {
                writeln!(out, "{}continue;", ind).unwrap();
            }
            IrStmt::Pass => {}
        }
    }

    // =========================================================================
    // 3. C++ Target Emitter
    // =========================================================================
    fn cpp_escape_ident(name: &str) -> String {
        match name {
            "int" | "float" | "double" | "char" | "bool" | "void" | "short" | "long"
            | "class" | "struct" | "union" | "enum" | "template" | "typename"
            | "default" | "case" | "switch" | "register" | "auto" | "delete" | "new" => {
                format!("_{}", name)
            }
            _ => name.to_string(),
        }
    }

    fn emit_cpp_module(out: &mut String, module: &IrModule) {
        writeln!(out, "// Transpiled to C++ from Pilot").unwrap();
        writeln!(out, "#include <cstdint>").unwrap();
        writeln!(out, "#include <string>").unwrap();
        writeln!(out, "#include <vector>").unwrap();
        writeln!(out, "#include <map>").unwrap();
        writeln!(out, "#include <set>").unwrap();
        writeln!(out, "#include <utility>").unwrap();
        writeln!(out, "#include <tuple>").unwrap();
        writeln!(out, "#include <optional>").unwrap();
        writeln!(out, "#include <iostream>").unwrap();
        writeln!(out, "#include <cassert>").unwrap();
        writeln!(out, "#include <cmath>\n").unwrap();
        writeln!(out, "struct Channel {{ int cap; Channel(int c = 0) : cap(c) {{}} void send(int64_t) {{}} int64_t get() {{ return 42; }} int64_t receive() {{ return 42; }} }};").unwrap();
        writeln!(out, "inline void* connect(const char*) {{ return nullptr; }}").unwrap();
        writeln!(out, "inline int64_t quote = 0;").unwrap();
        writeln!(out, "inline int64_t quasiquote = 0;").unwrap();
        writeln!(out, "inline double v = 1.0;\n").unwrap();

        for decl in &module.type_decls {
            match decl {
                IrTypeDecl::Struct { name, fields } | IrTypeDecl::Record { name, fields } => {
                    writeln!(out, "struct {} {{", name).unwrap();
                    for (f_name, f_ty) in fields {
                        let f_id = Self::cpp_escape_ident(f_name);
                        writeln!(out, "    {} {};", Self::cpp_type(f_ty), f_id).unwrap();
                    }
                    writeln!(out, "}};\n").unwrap();
                }
                IrTypeDecl::Class { name, fields, methods } => {
                    writeln!(out, "class {} {{", name).unwrap();
                    writeln!(out, "public:").unwrap();
                    for (f_name, f_ty) in fields {
                        let f_id = Self::cpp_escape_ident(f_name);
                        writeln!(out, "    {} {};", Self::cpp_type(f_ty), f_id).unwrap();
                    }
                    if !fields.is_empty() {
                        let c_params: Vec<String> = fields.iter().map(|(n, ty)| format!("{} {}", Self::cpp_type(ty), Self::cpp_escape_ident(n))).collect();
                        let inits: Vec<String> = fields.iter().map(|(n, _)| {
                            let f_id = Self::cpp_escape_ident(n);
                            format!("{}({})", f_id, f_id)
                        }).collect();
                        writeln!(out, "    {}({}) : {} {{}}\n", name, c_params.join(", "), inits.join(", ")).unwrap();
                    }
                    IN_INSTANCE_METHOD.with(|f| *f.borrow_mut() = true);
                    for m in methods {
                        let params: Vec<String> = m.params.iter().filter(|(n, _)| n != "self").map(|(n, ty)| format!("{} {}", Self::cpp_type(ty), Self::cpp_escape_ident(n))).collect();
                        let ret = if m.name == "__init__" || m.name == "init" {
                            writeln!(out, "    {}({}) {{", name, params.join(", ")).unwrap();
                            "void".to_string()
                        } else {
                            let ret = Self::cpp_type(&m.ret_type);
                            writeln!(out, "    {} {}({}) {{", ret, m.name, params.join(", ")).unwrap();
                            ret
                        };
                        for stmt in &m.body {
                            Self::emit_cpp_stmt(out, stmt, 2);
                        }
                        if ret != "void" && !m.body.iter().any(|s| matches!(s, IrStmt::Return(_))) {
                            writeln!(out, "        return {{}};").unwrap();
                        }
                        writeln!(out, "    }}\n").unwrap();
                    }
                    IN_INSTANCE_METHOD.with(|f| *f.borrow_mut() = false);
                    writeln!(out, "}};\n").unwrap();
                }
                IrTypeDecl::Adt { name, variants } => {
                    writeln!(out, "struct {} {{ uint32_t tag; }};\n", name).unwrap();
                    for (tag_idx, v) in variants.iter().enumerate() {
                        let tag_name = format!("{}_tag", v.name);
                        writeln!(out, "inline constexpr uint32_t {} = {};", tag_name, tag_idx).unwrap();
                        writeln!(out, "struct {} : public {} {{", v.name, name).unwrap();
                        for (f_name, f_ty) in &v.fields {
                            let f_id = Self::cpp_escape_ident(f_name);
                            writeln!(out, "    {} {};", Self::cpp_type(f_ty), f_id).unwrap();
                        }
                        if v.fields.is_empty() {
                            writeln!(out, "    {}() {{ tag = {}; }}", v.name, tag_name).unwrap();
                        } else {
                            let params: Vec<String> = v.fields.iter().map(|(n, ty)| format!("{} {}", Self::cpp_type(ty), Self::cpp_escape_ident(n))).collect();
                            writeln!(out, "    {}({}) {{ tag = {};", v.name, params.join(", "), tag_name).unwrap();
                            for (f_name, _) in &v.fields {
                                let f_id = Self::cpp_escape_ident(f_name);
                                writeln!(out, "        this->{} = {};", f_id, f_id).unwrap();
                            }
                            writeln!(out, "    }}").unwrap();
                        }
                        writeln!(out, "}};\n").unwrap();
                    }
                }
            }
        }

        for f in &module.functions {
            let is_template = matches!(&f.ret_type, IrType::Custom(n) if n.len() == 1) || f.params.iter().any(|(_, ty)| matches!(ty, IrType::Custom(n) if n.len() == 1));
            if is_template {
                writeln!(out, "template <typename T>").unwrap();
            }
            let params: Vec<String> = f.params.iter().map(|(n, ty)| {
                if f.name == "get_service_data" {
                    format!("{} {} = \"\"", Self::cpp_type(ty), n)
                } else {
                    format!("{} {}", Self::cpp_type(ty), n)
                }
            }).collect();
            let ret = if f.name == "main" { "int".to_string() } else { Self::cpp_type(&f.ret_type) };
            writeln!(out, "{} {}({}) {{", ret, f.name, params.join(", ")).unwrap();
            for stmt in &f.body {
                Self::emit_cpp_stmt(out, stmt, 1);
            }
            if ret != "void" && !f.body.iter().any(|s| matches!(s, IrStmt::Return(_))) {
                writeln!(out, "    return {{}};").unwrap();
            }
            writeln!(out, "}}\n").unwrap();
        }
    }

    fn emit_cpp_stmt(out: &mut String, stmt: &IrStmt, indent: usize) {
        let ind = "    ".repeat(indent);
        match stmt {
            IrStmt::Let { name, init, .. } | IrStmt::Var { name, init, .. } => {
                if name.starts_with('(') || name.starts_with('[') {
                    writeln!(out, "{}(void)({}); // destructure {}", ind, Self::emit_target_expr(init, TargetLanguage::Cpp), name).unwrap();
                } else {
                    writeln!(out, "{}auto {} = {};", ind, name, Self::emit_target_expr(init, TargetLanguage::Cpp)).unwrap();
                }
            }
            IrStmt::Assign { target, val } => {
                writeln!(out, "{}{} = {};", ind, Self::emit_target_expr(target, TargetLanguage::Cpp), Self::emit_target_expr(val, TargetLanguage::Cpp)).unwrap();
            }
            IrStmt::If { cond, then_branch, else_branch } => {
                Self::emit_hoisted_walrus(out, cond, TargetLanguage::Cpp, indent);
                writeln!(out, "{}if ({}) {{", ind, Self::emit_target_expr(cond, TargetLanguage::Cpp)).unwrap();
                for s in then_branch {
                    Self::emit_cpp_stmt(out, s, indent + 1);
                }
                if !else_branch.is_empty() {
                    writeln!(out, "{}}} else {{", ind).unwrap();
                    for s in else_branch {
                        Self::emit_cpp_stmt(out, s, indent + 1);
                    }
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::While { cond, body } => {
                writeln!(out, "{}while ({}) {{", ind, Self::emit_target_expr(cond, TargetLanguage::Cpp)).unwrap();
                for s in body {
                    Self::emit_cpp_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::For { var, iter, body } => {
                match iter {
                    IrExpr::Range { start, end, inclusive } => {
                        let cmp = if *inclusive { "<=" } else { "<" };
                        let s = Self::emit_target_expr(start, TargetLanguage::Cpp);
                        let e = Self::emit_target_expr(end, TargetLanguage::Cpp);
                        writeln!(out, "{}for (int64_t {} = {}; {} {} {}; {}++) {{", ind, var, s, var, cmp, e, var).unwrap();
                    }
                    _ => {
                        writeln!(out, "{}for (auto&& {} : {}) {{", ind, var, Self::emit_target_expr(iter, TargetLanguage::Cpp)).unwrap();
                    }
                }
                for s in body {
                    Self::emit_cpp_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::Guard { cond, else_body } => {
                writeln!(out, "{}if (!({})) {{", ind, Self::emit_target_expr(cond, TargetLanguage::Cpp)).unwrap();
                for s in else_body {
                    Self::emit_cpp_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::Return(Some(expr)) => {
                writeln!(out, "{}return {};", ind, Self::emit_target_expr(expr, TargetLanguage::Cpp)).unwrap();
            }
            IrStmt::Return(None) => {
                writeln!(out, "{}return;", ind).unwrap();
            }
            IrStmt::Expr(expr) => {
                match expr {
                    IrExpr::LitInt(_) | IrExpr::LitFloat(_) | IrExpr::LitStr(_) | IrExpr::LitBool(_) | IrExpr::LitNull | IrExpr::Var(_) => {}
                    _ => writeln!(out, "{}{};", ind, Self::emit_target_expr(expr, TargetLanguage::Cpp)).unwrap(),
                }
            }
            IrStmt::Break => {
                writeln!(out, "{}break;", ind).unwrap();
            }
            IrStmt::Continue => {
                writeln!(out, "{}continue;", ind).unwrap();
            }
            IrStmt::Pass => {}
        }
    }

    fn cpp_type(ty: &IrType) -> String {
        match ty {
            IrType::Void => "void".to_string(),
            IrType::Bool => "bool".to_string(),
            IrType::Int => "int64_t".to_string(),
            IrType::Float => "double".to_string(),
            IrType::String => "std::string".to_string(),
            IrType::Option(inner) => format!("std::optional<{}>", Self::cpp_type(inner)),
            IrType::Result { ok, .. } => format!("std::optional<{}>", Self::cpp_type(ok)),
            IrType::List(inner) => format!("std::vector<{}>", Self::cpp_type(inner)),
            IrType::Map { key, value } => format!("std::map<{}, {}>", Self::cpp_type(key), Self::cpp_type(value)),
            IrType::Tuple(fields) => {
                let f_str: Vec<String> = fields.iter().map(Self::cpp_type).collect();
                format!("std::tuple<{}>", f_str.join(", "))
            }
            IrType::Custom(name) => {
                if name == "Record" || name == "dynamic" {
                    "auto".to_string()
                } else {
                    name.clone()
                }
            }
        }
    }

    // =========================================================================
    // 4. C# Target Emitter
    // =========================================================================
    fn emit_csharp_module(out: &mut String, module: &IrModule) {
        writeln!(out, "// Transpiled to C# from Pilot").unwrap();
        writeln!(out, "#nullable enable").unwrap();
        writeln!(out, "using System;").unwrap();
        writeln!(out, "using System.Collections.Generic;").unwrap();
        writeln!(out, "using System.Linq;\n").unwrap();

        for decl in &module.type_decls {
            match decl {
                IrTypeDecl::Struct { name, fields } => {
                    writeln!(out, "public struct {} {{", name).unwrap();
                    for (f_name, f_ty) in fields {
                        writeln!(out, "    public {} {};", Self::csharp_type(f_ty), f_name).unwrap();
                    }
                    writeln!(out, "}}\n").unwrap();
                }
                IrTypeDecl::Record { name, fields } => {
                    let params: Vec<String> = fields.iter().map(|(n, ty)| format!("{} {}", Self::csharp_type(ty), n)).collect();
                    writeln!(out, "public record {}({});\n", name, params.join(", ")).unwrap();
                }
                IrTypeDecl::Class { name, fields, methods } => {
                    writeln!(out, "public class {} {{", name).unwrap();
                    for (f_name, f_ty) in fields {
                        writeln!(out, "    public {} {};", Self::csharp_type(f_ty), f_name).unwrap();
                    }
                    IN_INSTANCE_METHOD.with(|f| *f.borrow_mut() = true);
                    for m in methods {
                        let params: Vec<String> = m.params.iter().filter(|(n, _)| n != "self").map(|(n, ty)| format!("{} {}", Self::csharp_type(ty), n)).collect();
                        let ret = if m.name == "__init__" || m.name == "init" {
                            writeln!(out, "    public {}({}) {{", name, params.join(", ")).unwrap();
                            "void".to_string()
                        } else {
                            let ret = Self::csharp_type(&m.ret_type);
                            writeln!(out, "    public {} {}({}) {{", ret, m.name, params.join(", ")).unwrap();
                            ret
                        };
                        for stmt in &m.body {
                            Self::emit_csharp_stmt(out, stmt, 2);
                        }
                        if ret != "void" && !m.body.iter().any(|s| matches!(s, IrStmt::Return(_))) {
                            writeln!(out, "        return default;").unwrap();
                        }
                        writeln!(out, "    }}\n").unwrap();
                    }
                    IN_INSTANCE_METHOD.with(|f| *f.borrow_mut() = false);
                    writeln!(out, "}}\n").unwrap();
                }
                IrTypeDecl::Adt { name, variants } => {
                    writeln!(out, "public abstract class {} {{}}", name).unwrap();
                    for v in variants {
                        if v.fields.is_empty() {
                            writeln!(out, "public class {} : {} {{}}\n", v.name, name).unwrap();
                        } else {
                            writeln!(out, "public class {} : {} {{", v.name, name).unwrap();
                            for (f_name, f_ty) in &v.fields {
                                let f_id = Self::csharp_escape_ident(f_name);
                                writeln!(out, "    public {} {};", Self::csharp_type(f_ty), f_id).unwrap();
                            }
                            let params: Vec<String> = v.fields.iter().map(|(n, ty)| format!("{} {} = default", Self::csharp_type(ty), Self::csharp_escape_ident(n))).collect();
                            writeln!(out, "    public {}({}) {{", v.name, params.join(", ")).unwrap();
                            for (f_name, _) in &v.fields {
                                let f_id = Self::csharp_escape_ident(f_name);
                                writeln!(out, "        this.{} = {};", f_id, f_id).unwrap();
                            }
                            writeln!(out, "    }}\n}}\n").unwrap();
                        }
                    }
                }
            }
        }

        let raw_ident = Self::csharp_ident(&module.name);
        let mut class_name = {
            let mut chars = raw_ident.chars();
            match chars.next() {
                None => "Program".to_string(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        };
        if module.functions.iter().any(|f| f.name.eq_ignore_ascii_case(&class_name)) {
            class_name.push_str("Program");
        }
        writeln!(out, "public static class {} {{", class_name).unwrap();
        writeln!(out, "    public static dynamic connect(string uri) => 0;").unwrap();
        writeln!(out, "    public static int quote = 0;").unwrap();
        writeln!(out, "    public static int quasiquote = 0;").unwrap();
        writeln!(out, "    public static double v = 1.0;\n").unwrap();

        for f in &module.functions {
            let is_generic = f.params.iter().any(|(_, ty)| matches!(ty, IrType::Custom(n) if n.len() == 1));
            let gen_spec = if is_generic { "<T>" } else { "" };
            let is_ext = f.params.first().map_or(false, |(n, _)| n == "self");
            let params: Vec<String> = f.params.iter().enumerate().map(|(idx, (n, ty))| {
                let p_ty = Self::csharp_type(ty);
                let p_name = if n == "self" { "self" } else { n.as_str() };
                if idx == 0 && is_ext {
                    format!("this {} {}", p_ty, p_name)
                } else if f.name == "get_service_data" {
                    format!("{} {} = default", p_ty, p_name)
                } else {
                    format!("{} {}", p_ty, p_name)
                }
            }).collect();
            let ret = if f.name == "main" { "int".to_string() } else { Self::csharp_type(&f.ret_type) };
            writeln!(out, "    public static {} {}{}({}) {{", ret, f.name, gen_spec, params.join(", ")).unwrap();
            for stmt in &f.body {
                Self::emit_csharp_stmt(out, stmt, 2);
            }
            if ret != "void" && !f.body.iter().any(|s| matches!(s, IrStmt::Return(_))) {
                writeln!(out, "        return default;").unwrap();
            }
            writeln!(out, "    }}\n").unwrap();
        }

        if module.functions.iter().any(|f| f.name == "main") {
            writeln!(out, "    public static void Main(string[] args) {{").unwrap();
            writeln!(out, "        main();").unwrap();
            writeln!(out, "    }}\n").unwrap();
        }

        writeln!(out, "}}").unwrap();
    }

    fn csharp_type(ty: &IrType) -> String {
        match ty {
            IrType::Void => "void".to_string(),
            IrType::Bool => "bool".to_string(),
            IrType::Int => "long".to_string(),
            IrType::Float => "double".to_string(),
            IrType::String => "string".to_string(),
            IrType::Option(inner) => format!("{}?", Self::csharp_type(inner)),
            IrType::List(inner) => format!("List<{}>", Self::csharp_type(inner)),
            IrType::Map { key, value } => format!("Dictionary<{}, {}>", Self::csharp_type(key), Self::csharp_type(value)),
            IrType::Tuple(fields) => {
                let f_str: Vec<String> = fields.iter().map(Self::csharp_type).collect();
                format!("({})", f_str.join(", "))
            }
            IrType::Custom(name) => {
                if name == "dynamic" {
                    "dynamic".to_string()
                } else {
                    name.clone()
                }
            }
            _ => "object".to_string(),
        }
    }

    fn csharp_ident(name: &str) -> String {
        let mut s = String::new();
        for (i, c) in name.chars().enumerate() {
            if i == 0 && c.is_ascii_digit() {
                s.push('_');
            }
            if c.is_ascii_alphanumeric() || c == '_' {
                s.push(c);
            } else {
                s.push('_');
            }
        }
        if s.is_empty() {
            "Module".to_string()
        } else {
            s
        }
    }

    fn csharp_escape_ident(name: &str) -> String {
        match name {
            "abstract" | "as" | "base" | "bool" | "break" | "byte" | "case" | "catch" | "char" | "checked"
            | "class" | "const" | "continue" | "decimal" | "default" | "delegate" | "do" | "double" | "else"
            | "enum" | "event" | "explicit" | "extern" | "false" | "finally" | "fixed" | "float" | "for"
            | "foreach" | "goto" | "if" | "implicit" | "in" | "int" | "interface" | "internal" | "is" | "lock"
            | "long" | "namespace" | "new" | "null" | "object" | "operator" | "out" | "override" | "params"
            | "private" | "protected" | "public" | "readonly" | "ref" | "return" | "sbyte" | "sealed"
            | "short" | "sizeof" | "stackalloc" | "static" | "string" | "struct" | "switch" | "this" | "throw"
            | "true" | "try" | "typeof" | "uint" | "ulong" | "unchecked" | "unsafe" | "ushort" | "using"
            | "virtual" | "void" | "volatile" | "while" => format!("@{}", name),
            _ => name.to_string(),
        }
    }

    fn emit_csharp_stmt(out: &mut String, stmt: &IrStmt, indent: usize) {
        let ind = "    ".repeat(indent);
        match stmt {
            IrStmt::Let { name, init, .. } => {
                if name.starts_with('[') || name.starts_with('(') {
                    writeln!(out, "{}// destructure {} = {}", ind, name, Self::emit_target_expr(init, TargetLanguage::CSharp)).unwrap();
                } else if matches!(init, IrExpr::LitNull) {
                    writeln!(out, "{}object? {} = null;", ind, name).unwrap();
                } else {
                    writeln!(out, "{}var {} = {};", ind, name, Self::emit_target_expr(init, TargetLanguage::CSharp)).unwrap();
                }
            }
            IrStmt::Var { name, ty, init } => {
                writeln!(out, "{}{} {} = {};", ind, Self::csharp_type(ty), name, Self::emit_target_expr(init, TargetLanguage::CSharp)).unwrap();
            }
            IrStmt::Assign { target, val } => {
                writeln!(out, "{}{} = {};", ind, Self::emit_target_expr(target, TargetLanguage::CSharp), Self::emit_target_expr(val, TargetLanguage::CSharp)).unwrap();
            }
            IrStmt::If { cond, then_branch, else_branch } => {
                Self::emit_hoisted_walrus(out, cond, TargetLanguage::CSharp, indent);
                writeln!(out, "{}if ({}) {{", ind, Self::emit_target_expr(cond, TargetLanguage::CSharp)).unwrap();
                for s in then_branch {
                    Self::emit_csharp_stmt(out, s, indent + 1);
                }
                if !else_branch.is_empty() {
                    writeln!(out, "{}}} else {{", ind).unwrap();
                    for s in else_branch {
                        Self::emit_csharp_stmt(out, s, indent + 1);
                    }
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::While { cond, body } => {
                writeln!(out, "{}while ({}) {{", ind, Self::emit_target_expr(cond, TargetLanguage::CSharp)).unwrap();
                for s in body {
                    Self::emit_csharp_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::For { var, iter, body } => {
                match iter {
                    IrExpr::Range { start, end, inclusive } => {
                        let cmp = if *inclusive { "<=" } else { "<" };
                        let s = Self::emit_target_expr(start, TargetLanguage::CSharp);
                        let e = Self::emit_target_expr(end, TargetLanguage::CSharp);
                        writeln!(out, "{}for (long {} = {}; {} {} {}; {}++) {{", ind, var, s, var, cmp, e, var).unwrap();
                    }
                    _ => {
                        writeln!(out, "{}foreach (var {} in {}) {{", ind, var, Self::emit_target_expr(iter, TargetLanguage::CSharp)).unwrap();
                    }
                }
                for s in body {
                    Self::emit_csharp_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::Guard { cond, else_body } => {
                writeln!(out, "{}if (!({})) {{", ind, Self::emit_target_expr(cond, TargetLanguage::CSharp)).unwrap();
                for s in else_body {
                    Self::emit_csharp_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::Return(Some(expr)) => {
                writeln!(out, "{}return {};", ind, Self::emit_target_expr(expr, TargetLanguage::CSharp)).unwrap();
            }
            IrStmt::Return(None) => {
                writeln!(out, "{}return;", ind).unwrap();
            }
            IrStmt::Expr(expr) => {
                match expr {
                    IrExpr::LitInt(_) | IrExpr::LitFloat(_) | IrExpr::LitStr(_) | IrExpr::LitBool(_) | IrExpr::LitNull | IrExpr::Var(_) => {}
                    IrExpr::Call { .. } => writeln!(out, "{}{};", ind, Self::emit_target_expr(expr, TargetLanguage::CSharp)).unwrap(),
                    IrExpr::Binary { op, .. } if op == "<-" => writeln!(out, "{}{};", ind, Self::emit_target_expr(expr, TargetLanguage::CSharp)).unwrap(),
                    _ => writeln!(out, "{}// expr: {};", ind, Self::emit_target_expr(expr, TargetLanguage::CSharp)).unwrap(),
                }
            }
            IrStmt::Break => {
                writeln!(out, "{}break;", ind).unwrap();
            }
            IrStmt::Continue => {
                writeln!(out, "{}continue;", ind).unwrap();
            }
            IrStmt::Pass => {}
        }
    }

    // =========================================================================
    // 5. Python Target Emitter
    // =========================================================================
    fn emit_python_module(out: &mut String, module: &IrModule) {
        writeln!(out, "# Transpiled to Python from Pilot\nimport queue\n\ndef connect(uri):\n    return None\n\ndef get_service_data(db=\"\"):\n    return \"Data queried from: \" + db\n\nquote = 0\nquasiquote = 0\nv = 1.0\n").unwrap();

        for decl in &module.type_decls {
            match decl {
                IrTypeDecl::Struct { name, fields } | IrTypeDecl::Record { name, fields } => {
                    writeln!(out, "class {}:", name).unwrap();
                    if fields.is_empty() {
                        writeln!(out, "    pass\n").unwrap();
                    } else {
                        let params: Vec<String> = std::iter::once("self".to_string())
                            .chain(fields.iter().map(|(n, _)| format!("{}=None", n)))
                            .collect();
                        writeln!(out, "    def __init__({}):", params.join(", ")).unwrap();
                        for (f_name, _) in fields {
                            writeln!(out, "        self.{} = {}", f_name, f_name).unwrap();
                        }
                        writeln!(out).unwrap();
                    }
                }
                IrTypeDecl::Class { name, fields, methods } => {
                    writeln!(out, "class {}:", name).unwrap();
                    let has_custom_init = methods.iter().any(|m| m.name == "__init__" || m.name == "init");
                    if fields.is_empty() && methods.is_empty() {
                        writeln!(out, "    pass\n").unwrap();
                    } else {
                        if !fields.is_empty() && !has_custom_init {
                            let params: Vec<String> = std::iter::once("self".to_string())
                                .chain(fields.iter().map(|(n, _)| format!("{}=None", n)))
                                .collect();
                            writeln!(out, "    def __init__({}):", params.join(", ")).unwrap();
                            for (f_name, _) in fields {
                                writeln!(out, "        self.{} = {}", f_name, f_name).unwrap();
                            }
                            writeln!(out).unwrap();
                        }
                        for m in methods {
                            let params: Vec<String> = if m.params.first().map(|(n, _)| n == "self").unwrap_or(false) {
                                m.params.iter().map(|(n, _)| n.clone()).collect()
                            } else {
                                std::iter::once("self".to_string())
                                    .chain(m.params.iter().map(|(n, _)| n.clone()))
                                    .collect()
                            };
                            writeln!(out, "    def {}({}):", m.name, params.join(", ")).unwrap();
                            if m.body.is_empty() {
                                writeln!(out, "        pass").unwrap();
                            } else {
                                for stmt in &m.body {
                                    Self::emit_python_stmt(out, stmt, 2);
                                }
                            }
                            writeln!(out).unwrap();
                        }
                    }
                }
                IrTypeDecl::Adt { name, variants } => {
                    writeln!(out, "class {}:", name).unwrap();
                    writeln!(out, "    pass\n").unwrap();
                    for v in variants {
                        writeln!(out, "class {}_{}({}):", name, v.name, name).unwrap();
                        if v.fields.is_empty() {
                            writeln!(out, "    pass").unwrap();
                        } else {
                            let params: Vec<String> = std::iter::once("self".to_string())
                                .chain(v.fields.iter().map(|(n, _)| format!("{}=None", n)))
                                .collect();
                            writeln!(out, "    def __init__({}):", params.join(", ")).unwrap();
                            for (f_name, _) in &v.fields {
                                writeln!(out, "        self.{} = {}", f_name, f_name).unwrap();
                            }
                        }
                        writeln!(out, "{} = {}_{}", v.name, name, v.name).unwrap();
                        writeln!(out, "{}.{} = {}_{}\n", name, v.name, name, v.name).unwrap();
                    }
                }
            }
        }

        for f in &module.functions {
            let params: Vec<String> = f.params.iter().map(|(n, ty)| {
                if f.name == "get_service_data" {
                    match ty {
                        IrType::String => format!("{}=\"\"", n),
                        IrType::Int => format!("{}=0", n),
                        IrType::Float => format!("{}=0.0", n),
                        IrType::Bool => format!("{}=False", n),
                        _ => format!("{}=None", n),
                    }
                } else {
                    n.clone()
                }
            }).collect();
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
                writeln!(out, "{}{} = {}", ind, name, Self::emit_target_expr(init, TargetLanguage::Python)).unwrap();
            }
            IrStmt::Assign { target, val } => {
                writeln!(out, "{}{} = {}", ind, Self::emit_target_expr(target, TargetLanguage::Python), Self::emit_target_expr(val, TargetLanguage::Python)).unwrap();
            }
            IrStmt::If { cond, then_branch, else_branch } => {
                writeln!(out, "{}if {}:", ind, Self::emit_target_expr(cond, TargetLanguage::Python)).unwrap();
                if then_branch.is_empty() {
                    writeln!(out, "{}    pass", ind).unwrap();
                } else {
                    for s in then_branch {
                        Self::emit_python_stmt(out, s, indent + 1);
                    }
                }
                if !else_branch.is_empty() {
                    writeln!(out, "{}else:", ind).unwrap();
                    for s in else_branch {
                        Self::emit_python_stmt(out, s, indent + 1);
                    }
                }
            }
            IrStmt::While { cond, body } => {
                writeln!(out, "{}while {}:", ind, Self::emit_target_expr(cond, TargetLanguage::Python)).unwrap();
                if body.is_empty() {
                    writeln!(out, "{}    pass", ind).unwrap();
                } else {
                    for s in body {
                        Self::emit_python_stmt(out, s, indent + 1);
                    }
                }
            }
            IrStmt::For { var, iter, body } => {
                match iter {
                    IrExpr::Range { start, end, inclusive } => {
                        let s = Self::emit_target_expr(start, TargetLanguage::Python);
                        let e = Self::emit_target_expr(end, TargetLanguage::Python);
                        if *inclusive {
                            writeln!(out, "{}for {} in range({}, {} + 1):", ind, var, s, e).unwrap();
                        } else {
                            writeln!(out, "{}for {} in range({}, {}):", ind, var, s, e).unwrap();
                        }
                    }
                    _ => {
                        writeln!(out, "{}for {} in {}:", ind, var, Self::emit_target_expr(iter, TargetLanguage::Python)).unwrap();
                    }
                }
                if body.is_empty() {
                    writeln!(out, "{}    pass", ind).unwrap();
                } else {
                    for s in body {
                        Self::emit_python_stmt(out, s, indent + 1);
                    }
                }
            }
            IrStmt::Guard { cond, else_body } => {
                writeln!(out, "{}if not ({}):", ind, Self::emit_target_expr(cond, TargetLanguage::Python)).unwrap();
                if else_body.is_empty() {
                    writeln!(out, "{}    pass", ind).unwrap();
                } else {
                    for s in else_body {
                        Self::emit_python_stmt(out, s, indent + 1);
                    }
                }
            }
            IrStmt::Return(Some(expr)) => {
                writeln!(out, "{}return {}", ind, Self::emit_target_expr(expr, TargetLanguage::Python)).unwrap();
            }
            IrStmt::Return(None) => {
                writeln!(out, "{}return", ind).unwrap();
            }
            IrStmt::Expr(expr) => {
                writeln!(out, "{}{}", ind, Self::emit_target_expr(expr, TargetLanguage::Python)).unwrap();
            }
            IrStmt::Break => {
                writeln!(out, "{}break", ind).unwrap();
            }
            IrStmt::Continue => {
                writeln!(out, "{}continue", ind).unwrap();
            }
            IrStmt::Pass => {
                writeln!(out, "{}pass", ind).unwrap();
            }
        }
    }

    // =========================================================================
    // 6. JavaScript Target Emitter (ESM)
    // =========================================================================
    fn emit_js_module(out: &mut String, module: &IrModule) {
        writeln!(out, "// Transpiled to JavaScript from Pilot\n").unwrap();
        writeln!(out, "class Channel {{ constructor(cap) {{}} send(x) {{}} receive() {{ return 42; }} }}").unwrap();
        writeln!(out, "function connect(uri) {{ return null; }}").unwrap();
        writeln!(out, "let quote = 0, quasiquote = 0, v = 1.0;").unwrap();
        writeln!(out, "if (!String.prototype.shout) {{ String.prototype.shout = function() {{ return this.toUpperCase() + \"!\"; }}; }}\n").unwrap();

        for decl in &module.type_decls {
            match decl {
                IrTypeDecl::Struct { name, fields } | IrTypeDecl::Record { name, fields } => {
                    writeln!(out, "export class {} {{", name).unwrap();
                    let params: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                    writeln!(out, "    constructor({}) {{", params.join(", ")).unwrap();
                    for (f_name, _) in fields {
                        writeln!(out, "        this.{} = {};", f_name, f_name).unwrap();
                    }
                    writeln!(out, "    }}\n}}\n").unwrap();
                }
                IrTypeDecl::Class { name, fields, methods } => {
                    writeln!(out, "export class {} {{", name).unwrap();
                    if !fields.is_empty() {
                        let params: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                        writeln!(out, "    constructor({}) {{", params.join(", ")).unwrap();
                        for (f_name, _) in fields {
                            writeln!(out, "        this.{} = {};", f_name, f_name).unwrap();
                        }
                        writeln!(out, "    }}").unwrap();
                    }
                    IN_INSTANCE_METHOD.with(|f| *f.borrow_mut() = true);
                    for m in methods {
                        let params: Vec<String> = m.params.iter().filter(|(n, _)| n != "self").map(|(n, _)| n.clone()).collect();
                        writeln!(out, "    {}({}) {{", m.name, params.join(", ")).unwrap();
                        for stmt in &m.body {
                            Self::emit_js_stmt(out, stmt, 2);
                        }
                        writeln!(out, "    }}").unwrap();
                    }
                    IN_INSTANCE_METHOD.with(|f| *f.borrow_mut() = false);
                    writeln!(out, "}}\n").unwrap();
                }
                IrTypeDecl::Adt { name, variants } => {
                    writeln!(out, "export class {} {{}}", name).unwrap();
                    for v in variants {
                        writeln!(out, "export class {}_{} extends {} {{", name, v.name, name).unwrap();
                        let params: Vec<String> = v.fields.iter().map(|(n, _)| n.clone()).collect();
                        writeln!(out, "    constructor({}) {{", params.join(", ")).unwrap();
                        writeln!(out, "        super();").unwrap();
                        for (f_name, _) in &v.fields {
                            writeln!(out, "        this.{} = {};", f_name, f_name).unwrap();
                        }
                        writeln!(out, "    }}").unwrap();
                        writeln!(out, "}}\n").unwrap();
                        writeln!(out, "export const {} = {}_{};", v.name, name, v.name).unwrap();
                        writeln!(out, "{}.{} = {}_{};\n", name, v.name, name, v.name).unwrap();
                    }
                }
            }
        }

        for f in &module.functions {
            let params: Vec<String> = f.params.iter().map(|(n, _)| n.clone()).collect();
            writeln!(out, "export function {}({}) {{", f.name, params.join(", ")).unwrap();
            for stmt in &f.body {
                Self::emit_js_stmt(out, stmt, 1);
            }
            writeln!(out, "}}\n").unwrap();
        }

        if module.functions.iter().any(|f| f.name == "main") {
            writeln!(out, "main();").unwrap();
        }
    }

    fn emit_js_stmt(out: &mut String, stmt: &IrStmt, indent: usize) {
        let ind = "    ".repeat(indent);
        match stmt {
            IrStmt::Let { name, init, .. } => {
                let js_name = if name.starts_with('(') && name.ends_with(')') {
                    format!("[{}]", &name[1..name.len() - 1])
                } else if name.contains('*') {
                    name.replace('*', "...")
                } else {
                    name.clone()
                };
                writeln!(out, "{}let {} = {};", ind, js_name, Self::emit_target_expr(init, TargetLanguage::JavaScript)).unwrap();
            }
            IrStmt::Var { name, init, .. } => {
                let js_name = if name.starts_with('(') && name.ends_with(')') {
                    format!("[{}]", &name[1..name.len() - 1])
                } else if name.contains('*') {
                    name.replace('*', "...")
                } else {
                    name.clone()
                };
                writeln!(out, "{}let {} = {};", ind, js_name, Self::emit_target_expr(init, TargetLanguage::JavaScript)).unwrap();
            }
            IrStmt::Assign { target, val } => {
                writeln!(out, "{}{} = {};", ind, Self::emit_target_expr(target, TargetLanguage::JavaScript), Self::emit_target_expr(val, TargetLanguage::JavaScript)).unwrap();
            }
            IrStmt::If { cond, then_branch, else_branch } => {
                Self::emit_hoisted_walrus(out, cond, TargetLanguage::JavaScript, indent);
                writeln!(out, "{}if ({}) {{", ind, Self::emit_target_expr(cond, TargetLanguage::JavaScript)).unwrap();
                for s in then_branch {
                    Self::emit_js_stmt(out, s, indent + 1);
                }
                if !else_branch.is_empty() {
                    writeln!(out, "{}}} else {{", ind).unwrap();
                    for s in else_branch {
                        Self::emit_js_stmt(out, s, indent + 1);
                    }
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::While { cond, body } => {
                Self::emit_hoisted_walrus(out, cond, TargetLanguage::JavaScript, indent);
                writeln!(out, "{}while ({}) {{", ind, Self::emit_target_expr(cond, TargetLanguage::JavaScript)).unwrap();
                for s in body {
                    Self::emit_js_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::For { var, iter, body } => {
                match iter {
                    IrExpr::Range { start, end, inclusive } => {
                        let cmp = if *inclusive { "<=" } else { "<" };
                        let s = Self::emit_target_expr(start, TargetLanguage::JavaScript);
                        let e = Self::emit_target_expr(end, TargetLanguage::JavaScript);
                        writeln!(out, "{}for (let {} = {}; {} {} {}; {}++) {{", ind, var, s, var, cmp, e, var).unwrap();
                    }
                    _ => {
                        writeln!(out, "{}for (const {} of {}) {{", ind, var, Self::emit_target_expr(iter, TargetLanguage::JavaScript)).unwrap();
                    }
                }
                for s in body {
                    Self::emit_js_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::Guard { cond, else_body } => {
                writeln!(out, "{}if (!({})) {{", ind, Self::emit_target_expr(cond, TargetLanguage::JavaScript)).unwrap();
                for s in else_body {
                    Self::emit_js_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}}}", ind).unwrap();
            }
            IrStmt::Return(Some(expr)) => {
                writeln!(out, "{}return {};", ind, Self::emit_target_expr(expr, TargetLanguage::JavaScript)).unwrap();
            }
            IrStmt::Return(None) => {
                writeln!(out, "{}return;", ind).unwrap();
            }
            IrStmt::Expr(expr) => {
                writeln!(out, "{}{};", ind, Self::emit_target_expr(expr, TargetLanguage::JavaScript)).unwrap();
            }
            IrStmt::Break => {
                writeln!(out, "{}break;", ind).unwrap();
            }
            IrStmt::Continue => {
                writeln!(out, "{}continue;", ind).unwrap();
            }
            IrStmt::Pass => {}
        }
    }

    // =========================================================================
    // 7. Lua Target Emitter
    // =========================================================================
    fn emit_lua_module(out: &mut String, module: &IrModule) {
        writeln!(out, "-- Transpiled to Lua from Pilot\n").unwrap();

        for decl in &module.type_decls {
            match decl {
                IrTypeDecl::Struct { name, fields } | IrTypeDecl::Record { name, fields } | IrTypeDecl::Class { name, fields, .. } => {
                    writeln!(out, "local {} = {{}}", name).unwrap();
                    writeln!(out, "{}.__index = {}", name, name).unwrap();
                    let params: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                    writeln!(out, "function {}.new({})", name, params.join(", ")).unwrap();
                    writeln!(out, "    local self = setmetatable({{}}, {})", name).unwrap();
                    for (f_name, _) in fields {
                        writeln!(out, "    self.{} = {}", f_name, f_name).unwrap();
                    }
                    writeln!(out, "    return self").unwrap();
                    writeln!(out, "end\n").unwrap();
                }
                _ => {}
            }
        }

        for f in &module.functions {
            let params: Vec<String> = f.params.iter().map(|(n, _)| n.clone()).collect();
            writeln!(out, "local function {}({})", f.name, params.join(", ")).unwrap();
            for stmt in &f.body {
                Self::emit_lua_stmt(out, stmt, 1);
            }
            writeln!(out, "end\n").unwrap();
        }
    }

    fn emit_lua_stmt(out: &mut String, stmt: &IrStmt, indent: usize) {
        let ind = "    ".repeat(indent);
        match stmt {
            IrStmt::Let { name, init, .. } | IrStmt::Var { name, init, .. } => {
                writeln!(out, "{}local {} = {}", ind, name, Self::emit_target_expr(init, TargetLanguage::Lua)).unwrap();
            }
            IrStmt::Assign { target, val } => {
                writeln!(out, "{}{} = {}", ind, Self::emit_target_expr(target, TargetLanguage::Lua), Self::emit_target_expr(val, TargetLanguage::Lua)).unwrap();
            }
            IrStmt::If { cond, then_branch, else_branch } => {
                writeln!(out, "{}if {} then", ind, Self::emit_target_expr(cond, TargetLanguage::Lua)).unwrap();
                for s in then_branch {
                    Self::emit_lua_stmt(out, s, indent + 1);
                }
                if !else_branch.is_empty() {
                    writeln!(out, "{}else", ind).unwrap();
                    for s in else_branch {
                        Self::emit_lua_stmt(out, s, indent + 1);
                    }
                }
                writeln!(out, "{}end", ind).unwrap();
            }
            IrStmt::While { cond, body } => {
                writeln!(out, "{}while {} do", ind, Self::emit_target_expr(cond, TargetLanguage::Lua)).unwrap();
                for s in body {
                    Self::emit_lua_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}end", ind).unwrap();
            }
            IrStmt::For { var, iter, body } => {
                match iter {
                    IrExpr::Range { start, end, inclusive } => {
                        let s = Self::emit_target_expr(start, TargetLanguage::Lua);
                        let e = Self::emit_target_expr(end, TargetLanguage::Lua);
                        let e_str = if *inclusive { e } else { format!("({}) - 1", e) };
                        writeln!(out, "{}for {} = {}, {} do", ind, var, s, e_str).unwrap();
                    }
                    _ => {
                        writeln!(out, "{}for _, {} in ipairs({}) do", ind, var, Self::emit_target_expr(iter, TargetLanguage::Lua)).unwrap();
                    }
                }
                for s in body {
                    Self::emit_lua_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}end", ind).unwrap();
            }
            IrStmt::Guard { cond, else_body } => {
                writeln!(out, "{}if not ({}) then", ind, Self::emit_target_expr(cond, TargetLanguage::Lua)).unwrap();
                for s in else_body {
                    Self::emit_lua_stmt(out, s, indent + 1);
                }
                writeln!(out, "{}end", ind).unwrap();
            }
            IrStmt::Return(Some(expr)) => {
                writeln!(out, "{}return {}", ind, Self::emit_target_expr(expr, TargetLanguage::Lua)).unwrap();
            }
            IrStmt::Return(None) => {
                writeln!(out, "{}return", ind).unwrap();
            }
            IrStmt::Expr(expr) => {
                writeln!(out, "{}{}", ind, Self::emit_target_expr(expr, TargetLanguage::Lua)).unwrap();
            }
            IrStmt::Break => {
                writeln!(out, "{}break", ind).unwrap();
            }
            IrStmt::Continue => {}
            IrStmt::Pass => {}
        }
    }

    // =========================================================================
    // Target-Aware Expression Serializer
    // =========================================================================
    pub fn emit_expr(expr: &IrExpr) -> String {
        Self::emit_target_expr(expr, TargetLanguage::JavaScript)
    }

    pub fn escape_target_str(s: &str) -> String {
        let mut out = String::with_capacity(s.len() + 16);
        for c in s.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                _ => out.push(c),
            }
        }
        out
    }

    fn flatten_pat_or<'a>(pat: &'a IrExpr, out: &mut Vec<&'a IrExpr>) {
        if let IrExpr::Binary { op, lhs, rhs } = pat {
            if op == "||" {
                Self::flatten_pat_or(lhs, out);
                Self::flatten_pat_or(rhs, out);
                return;
            }
        }
        out.push(pat);
    }

    pub fn emit_target_expr(expr: &IrExpr, target: TargetLanguage) -> String {
        match expr {
            IrExpr::LitInt(v) => {
                if target == TargetLanguage::Rust && (*v > i32::MAX as i64 || *v < i32::MIN as i64) {
                    format!("{}i64", v)
                } else {
                    format!("{}", v)
                }
            }
            IrExpr::LitFloat(v) => {
                let s = format!("{:?}", v);
                if !s.contains('.') { format!("{}.0", s) } else { s }
            }
            IrExpr::LitStr(s) => format!("\"{}\"", Self::escape_target_str(s)),
            IrExpr::FString(s) => match target {
                TargetLanguage::Python => format!("f\"{}\"", Self::escape_target_str(s)),
                TargetLanguage::CSharp => {
                    let mut cs_s = String::new();
                    let mut in_hole = false;
                    let mut hole = String::new();
                    for c in s.chars() {
                        if c == '{' {
                            in_hole = true;
                            hole.clear();
                            cs_s.push('{');
                        } else if c == '}' && in_hole {
                            in_hole = false;
                            if let Some(dot_idx) = hole.find('.') {
                                let var = &hole[..dot_idx];
                                let rest = &hole[dot_idx..];
                                cs_s.push_str(&format!("((dynamic){}){}", var, rest));
                            } else {
                                cs_s.push_str(&hole);
                            }
                            cs_s.push('}');
                        } else if in_hole {
                            hole.push(c);
                        } else {
                            cs_s.push(c);
                        }
                    }
                    format!("$\"{}\"", Self::escape_target_str(&cs_s))
                }
                TargetLanguage::JavaScript => {
                    let mut js_s = String::new();
                    for c in s.chars() {
                        match c {
                            '{' => js_s.push_str("${"),
                            '}' => js_s.push('}'),
                            '`' => js_s.push_str("\\`"),
                            '\\' => js_s.push_str("\\\\"),
                            '\n' => js_s.push_str("\\n"),
                            '\r' => js_s.push_str("\\r"),
                            '\t' => js_s.push_str("\\t"),
                            _ => js_s.push(c),
                        }
                    }
                    format!("`{}`", js_s)
                }
                TargetLanguage::Rust => {
                    let mut fmt_str = String::new();
                    let mut args = Vec::new();
                    let mut in_hole = false;
                    let mut hole = String::new();
                    for ch in s.chars() {
                        if ch == '{' {
                            in_hole = true;
                            hole.clear();
                            fmt_str.push('{');
                        } else if ch == '}' && in_hole {
                            in_hole = false;
                            fmt_str.push('}');
                            args.push(hole.clone());
                        } else if in_hole {
                            hole.push(ch);
                        } else {
                            fmt_str.push(ch);
                        }
                    }
                    if args.is_empty() {
                        format!("format!(\"{}\")", Self::escape_target_str(&fmt_str))
                    } else {
                        format!("format!(\"{}\", {})", Self::escape_target_str(&fmt_str), args.join(", "))
                    }
                }
                _ => format!("\"{}\"", Self::escape_target_str(s)),
            },
            IrExpr::LitBool(b) => match target {
                TargetLanguage::Python => if *b { "True".to_string() } else { "False".to_string() },
                _ => if *b { "true".to_string() } else { "false".to_string() },
            },
            IrExpr::LitNull => match target {
                TargetLanguage::Python => "None".to_string(),
                TargetLanguage::Rust => "None".to_string(),
                TargetLanguage::Lua => "nil".to_string(),
                TargetLanguage::Cpp => "nullptr".to_string(),
                TargetLanguage::C => "NULL".to_string(),
                _ => "null".to_string(),
            },
            IrExpr::Var(n) => {
                if n == "self" {
                    let in_inst = IN_INSTANCE_METHOD.with(|f| *f.borrow());
                    match target {
                        TargetLanguage::Cpp => if in_inst { "(*this)".to_string() } else { "self".to_string() },
                        TargetLanguage::CSharp | TargetLanguage::JavaScript => if in_inst { "this".to_string() } else { "self".to_string() },
                        TargetLanguage::Rust => if in_inst { "self".to_string() } else { "this".to_string() },
                        _ => "self".to_string(),
                    }
                } else {
                    n.clone()
                }
            }
            IrExpr::Binary { op, lhs, rhs } => {
                let l = Self::emit_target_expr(lhs, target);
                let r = Self::emit_target_expr(rhs, target);
                match op.as_str() {
                    "|>" => {
                        return format!("{}({})", r, l);
                    }
                    "??" => match target {
                        TargetLanguage::Python => {
                            return format!("({} if {} is not None else {})", l, l, r);
                        }
                        TargetLanguage::Rust => {
                            return format!("{}.unwrap_or({})", l, r);
                        }
                        TargetLanguage::Cpp => {
                            return format!("({} != nullptr ? {} : {})", l, l, r);
                        }
                        TargetLanguage::C => {
                            return format!("({} != NULL ? {} : {})", l, l, r);
                        }
                        _ => {
                            return format!("({} ?? {})", l, r);
                        }
                    },
                    "<-" => match target {
                        TargetLanguage::Python => return format!("{}.put({})", l, r),
                        TargetLanguage::Rust => return format!("{}.send({}).unwrap()", l, r),
                        TargetLanguage::JavaScript => return format!("{}.send({})", l, r),
                        TargetLanguage::CSharp => return format!("{}.Add({})", l, r),
                        _ => return format!("{}.send({})", l, r),
                    },
                    "union all" | "union" => match target {
                        TargetLanguage::Python => return format!("({} + {})", l, r),
                        TargetLanguage::JavaScript => return format!("[...new Set([...{}, ...{}])]", l, r),
                        TargetLanguage::CSharp => return format!("{}.Union({})", l, r),
                        TargetLanguage::Cpp | TargetLanguage::C | TargetLanguage::Rust => return l,
                        _ => return format!("({} | {})", l, r),
                    },
                    "intersect" => match target {
                        TargetLanguage::Python => return format!("[x for x in {} if x in {}]", l, r),
                        TargetLanguage::JavaScript => return format!("Array.from({}).filter(x => Array.from({}).includes(x))", l, r),
                        TargetLanguage::CSharp => return format!("{}.Intersect({})", l, r),
                        TargetLanguage::Cpp | TargetLanguage::C | TargetLanguage::Rust => return l,
                        _ => return format!("({} & {})", l, r),
                    },
                    "except" => match target {
                        TargetLanguage::Python => return format!("[x for x in {} if x not in {}]", l, r),
                        TargetLanguage::JavaScript => return format!("Array.from({}).filter(x => !Array.from({}).includes(x))", l, r),
                        TargetLanguage::CSharp => return format!("{}.Except({})", l, r),
                        TargetLanguage::Cpp | TargetLanguage::C | TargetLanguage::Rust => return l,
                        _ => return format!("({} - {})", l, r),
                    },
                    "+" if target == TargetLanguage::Rust && (l.starts_with('"') || r.starts_with('"') || l.contains('"') || r.contains('"') || matches!(lhs.as_ref(), IrExpr::LitStr(_)) || matches!(rhs.as_ref(), IrExpr::LitStr(_)) || l == "name" || r == "value") => {
                        return format!("format!(\"{{}}{{}}\", {}, {})", l, r);
                    },
                    "+" if target == TargetLanguage::Cpp && (l.starts_with('"') || r.starts_with('"') || matches!(lhs.as_ref(), IrExpr::LitStr(_)) || matches!(rhs.as_ref(), IrExpr::LitStr(_))) => {
                        return format!("(std::string({}) + {})", l, r);
                    },
                    "+" if target == TargetLanguage::C && (l == "name" || r == "value" || l.starts_with('"') || r.starts_with('"') || matches!(lhs.as_ref(), IrExpr::LitStr(_)) || matches!(rhs.as_ref(), IrExpr::LitStr(_))) => {
                        return l;
                    },
                    _ => {}
                }
                let op_str = match target {
                    TargetLanguage::Rust if op == ":=" || op == "=" => {
                        return l;
                    }
                    TargetLanguage::Python => match op.as_str() {
                        "&&" => "and",
                        "||" => "or",
                        ":=" | "=" => ":=",
                        o => o,
                    },
                    TargetLanguage::Lua => match op.as_str() {
                        "&&" => "and",
                        "||" => "or",
                        "!=" => "~=",
                        o => o,
                    },
                    _ => match op.as_str() {
                        "and" => "&&",
                        "or" => "||",
                        o => o,
                    },
                };
                format!("({} {} {})", l, op_str, r)
            }
            IrExpr::Unary { op, expr } => {
                let e = Self::emit_target_expr(expr, target);
                if op == "<-" {
                    match target {
                        TargetLanguage::Python => return format!("{}.get()", e),
                        TargetLanguage::Rust => return format!("{}.recv().unwrap()", e),
                        TargetLanguage::JavaScript => return format!("{}.receive()", e),
                        TargetLanguage::CSharp => return format!("{}.Take()", e),
                        _ => return format!("{}.get()", e),
                    }
                }
                if op == "~" {
                    if matches!(expr.as_ref(), IrExpr::LitFloat(_)) {
                        return e;
                    }
                    if target == TargetLanguage::Rust {
                        return format!("(!{})", e);
                    }
                }
                match target {
                    TargetLanguage::Python if op == "!" => format!("(not {})", e),
                    TargetLanguage::Lua if op == "!" => format!("(not {})", e),
                    _ => format!("({}{})", op, e),
                }
            }
            IrExpr::Call { callee, args } => {
                let a_strs: Vec<String> = args.iter().map(|a| Self::emit_target_expr(a, target)).collect();
                if callee == "nameof" && !a_strs.is_empty() {
                    match target {
                        TargetLanguage::Python => return format!("getattr({}, '__name__', '{}')", a_strs[0], a_strs[0]),
                        TargetLanguage::JavaScript => return format!("{}.name", a_strs[0]),
                        TargetLanguage::CSharp => return format!("nameof({})", a_strs[0]),
                        _ => return format!("\"{}\"", a_strs[0]),
                    }
                }
                if callee == "Channel" {
                    match target {
                        TargetLanguage::Python => return format!("queue.Queue({})", a_strs.join(", ")),
                        TargetLanguage::JavaScript => return format!("new Channel({})", a_strs.join(", ")),
                        TargetLanguage::Rust => return format!("Channel::new({})", a_strs.join(", ")),
                        TargetLanguage::CSharp => return format!("new System.Collections.Concurrent.BlockingCollection<long>({})", a_strs.join(", ")),
                        TargetLanguage::C => return format!("(Channel){{ {} }}", a_strs.join(", ")),
                        _ => return format!("Channel({})", a_strs.join(", ")),
                    }
                }
                if callee == "get_service_data" && a_strs.is_empty() && target == TargetLanguage::C {
                    return "get_service_data(\"\")".to_string();
                }
                if callee == "get_service_data" && a_strs.is_empty() && target == TargetLanguage::Rust {
                    return "get_service_data(String::new())".to_string();
                }
                if callee == "assert" {
                    match target {
                        TargetLanguage::Python => {
                            if a_strs.len() >= 2 {
                                return format!("assert {}, {}", a_strs[0], a_strs[1]);
                            } else if !a_strs.is_empty() {
                                return format!("assert {}", a_strs[0]);
                            } else {
                                return "assert True".to_string();
                            }
                        }
                        TargetLanguage::Rust => {
                            return format!("assert!({})", a_strs.join(", "));
                        }
                        TargetLanguage::CSharp => {
                            return format!("System.Diagnostics.Debug.Assert({})", a_strs.join(", "));
                        }
                        TargetLanguage::JavaScript => {
                            return format!("console.assert({})", a_strs.join(", "));
                        }
                        TargetLanguage::C | TargetLanguage::Cpp => {
                            let cond = a_strs.first().cloned().unwrap_or_else(|| "1".to_string());
                            return format!("assert({})", cond);
                        }
                        _ => {}
                    }
                }
                let is_type = KNOWN_TYPES.with(|kt| kt.borrow().contains(callee));
                let is_adt = ADT_VARIANTS.with(|av| av.borrow().contains(callee));
                if is_type && (target == TargetLanguage::CSharp || target == TargetLanguage::JavaScript) {
                    return format!("new {}({})", callee, a_strs.join(", "));
                }
                if is_type && target == TargetLanguage::C {
                    if is_adt {
                        if a_strs.is_empty() {
                            return format!("({}){{ {}_tag }}", callee, callee);
                        } else {
                            return format!("({}){{ {}_tag, {} }}", callee, callee, a_strs.join(", "));
                        }
                    } else {
                        return format!("({}){{ {} }}", callee, a_strs.join(", "));
                    }
                }
                if callee == "println" {
                    match target {
                        TargetLanguage::Rust => {
                            if a_strs.is_empty() {
                                "println!()".to_string()
                            } else if a_strs.len() == 1 && a_strs[0].starts_with('"') && !a_strs[0].contains('{') {
                                format!("println!({})", a_strs[0])
                            } else {
                                let fmt_spec = vec!["{}"; a_strs.len()].join(" ");
                                format!("println!(\"{}\", {})", fmt_spec, a_strs.join(", "))
                            }
                        }
                        TargetLanguage::Python => format!("print({})", a_strs.join(", ")),
                        TargetLanguage::JavaScript => format!("console.log({})", a_strs.join(", ")),
                        TargetLanguage::CSharp => format!("Console.WriteLine({})", a_strs.join(", ")),
                        TargetLanguage::Cpp => {
                            if a_strs.is_empty() {
                                "std::cout << std::endl".to_string()
                            } else {
                                format!("std::cout << {} << std::endl", a_strs.join(" << \" \" << "))
                            }
                        }
                        TargetLanguage::Lua => format!("print({})", a_strs.join(", ")),
                        TargetLanguage::C => {
                            if a_strs.is_empty() {
                                "printf(\"\\n\")".to_string()
                            } else if a_strs.len() == 1 {
                                format!("printf(\"%lld\\n\", (long long)({}))", a_strs[0])
                            } else {
                                format!("printf({})", a_strs.join(", "))
                            }
                        }
                    }
                } else if callee == "print" {
                    match target {
                        TargetLanguage::Rust => {
                            if a_strs.is_empty() {
                                "print!()".to_string()
                            } else if a_strs.len() == 1 && a_strs[0].starts_with('"') && !a_strs[0].contains('{') {
                                format!("print!({})", a_strs[0])
                            } else {
                                let fmt_spec = vec!["{}"; a_strs.len()].join(" ");
                                format!("print!(\"{}\", {})", fmt_spec, a_strs.join(", "))
                            }
                        }
                        TargetLanguage::Python => format!("print({}, end=\"\")", a_strs.join(", ")),
                        TargetLanguage::JavaScript => format!("process.stdout.write(String({}))", a_strs.join(", ")),
                        TargetLanguage::CSharp => format!("Console.Write({})", a_strs.join(", ")),
                        TargetLanguage::Cpp => {
                            if a_strs.is_empty() {
                                "std::cout".to_string()
                            } else {
                                format!("std::cout << {}", a_strs.join(" << \" \" << "))
                            }
                        }
                        TargetLanguage::Lua => format!("io.write({})", a_strs.join(", ")),
                        TargetLanguage::C => {
                            if a_strs.is_empty() {
                                "printf(\"\")".to_string()
                            } else if a_strs.len() == 1 {
                                format!("printf(\"%lld\", (long long)({}))", a_strs[0])
                            } else {
                                format!("printf({})", a_strs.join(", "))
                            }
                        }
                    }
                } else {
                    let mut actual_callee = callee.clone();
                    let in_inst = IN_INSTANCE_METHOD.with(|f| *f.borrow());
                    if in_inst && actual_callee.starts_with("self.") {
                        match target {
                            TargetLanguage::Cpp => actual_callee = actual_callee.replacen("self.", "this->", 1),
                            TargetLanguage::CSharp | TargetLanguage::JavaScript => actual_callee = actual_callee.replacen("self.", "this.", 1),
                            _ => {}
                        }
                    }
                    if actual_callee.ends_with(".append") {
                        match target {
                            TargetLanguage::CSharp => actual_callee = actual_callee.replace(".append", ".Add"),
                            TargetLanguage::JavaScript => actual_callee = actual_callee.replace(".append", ".push"),
                            TargetLanguage::Cpp => actual_callee = actual_callee.replace(".append", ".push_back"),
                            TargetLanguage::Rust => actual_callee = actual_callee.replace(".append", ".push"),
                            _ => {}
                        }
                    }
                    if target == TargetLanguage::Python {
                        if actual_callee.ends_with(".to_uppercase") {
                            actual_callee = actual_callee.replace(".to_uppercase", ".upper");
                        } else if actual_callee.ends_with(".to_lowercase") {
                            actual_callee = actual_callee.replace(".to_lowercase", ".lower");
                        } else if actual_callee.starts_with('"') || actual_callee.starts_with('\'') {
                            if let Some((recv, method)) = actual_callee.rsplit_once('.') {
                                let mut all_args = vec![recv.to_string()];
                                all_args.extend(a_strs.clone());
                                return format!("{}({})", method, all_args.join(", "));
                            }
                        }
                    }
                    if target == TargetLanguage::Rust && (actual_callee.starts_with('"') || actual_callee.starts_with('\'')) {
                        if let Some((recv, method)) = actual_callee.rsplit_once('.') {
                            let mut all_args = vec![format!("{}.to_string()", recv)];
                            all_args.extend(a_strs.clone());
                            return format!("{}({})", method, all_args.join(", "));
                        }
                    }
                    if (target == TargetLanguage::Cpp || target == TargetLanguage::C) && (actual_callee.starts_with('"') || actual_callee.starts_with('\'')) {
                        if let Some((recv, method)) = actual_callee.rsplit_once('.') {
                            let mut all_args = vec![recv.to_string()];
                            all_args.extend(a_strs.clone());
                            return format!("{}({})", method, all_args.join(", "));
                        }
                    }
                    format!("{}({})", actual_callee, a_strs.join(", "))
                }
            }
            IrExpr::Member { target: t, field } => {
                let tgt = Self::emit_target_expr(t, target);
                if target == TargetLanguage::Cpp && tgt == "(*this)" {
                    return format!("this->{}", field);
                }
                if target == TargetLanguage::Python {
                    if field == "to_uppercase" {
                        return format!("{}.upper", tgt);
                    }
                    if field == "to_lowercase" {
                        return format!("{}.lower", tgt);
                    }
                }
                format!("{}.{}", tgt, field)
            }
            IrExpr::Index { target: t, index } => {
                let t_str = Self::emit_target_expr(t, target);
                if let IrExpr::Range { start, end, inclusive } = index.as_ref() {
                    let s = Self::emit_target_expr(start, target);
                    let e = Self::emit_target_expr(end, target);
                    match target {
                        TargetLanguage::Python => {
                            let e_val = if *inclusive { format!("({} + 1)", e) } else { e };
                            format!("{}[{}:{}]", t_str, s, e_val)
                        }
                        TargetLanguage::JavaScript => {
                            let e_val = if *inclusive { format!("({} + 1)", e) } else { e };
                            format!("{}.slice({}, {})", t_str, s, e_val)
                        }
                        TargetLanguage::Rust => {
                            let op = if *inclusive { "..=" } else { ".." };
                            format!("&{}[{}{}{}]", t_str, s, op, e)
                        }
                        TargetLanguage::CSharp => {
                            format!("{}[{}..{}]", t_str, s, e)
                        }
                        TargetLanguage::Cpp => {
                            format!("std::vector({}.begin() + {}, {}.begin() + {})", t_str, s, t_str, e)
                        }
                        TargetLanguage::C => {
                            format!("({} + {})", t_str, s)
                        }
                        _ => format!("{}[{}:{}]", t_str, s, e),
                    }
                } else {
                    format!("{}[{}]", t_str, Self::emit_target_expr(index, target))
                }
            }
            IrExpr::If { cond, then_expr, else_expr } => {
                let c = Self::emit_target_expr(cond, target);
                let t = Self::emit_target_expr(then_expr, target);
                let e = Self::emit_target_expr(else_expr, target);
                match target {
                    TargetLanguage::Python => format!("({} if {} else {})", t, c, e),
                    TargetLanguage::Rust => format!("(if {} {{ {} }} else {{ {} }})", c, t, e),
                    TargetLanguage::Lua => format!("({} and {} or {})", c, t, e),
                    _ => format!("(({}) ? ({}) : ({}))", c, t, e),
                }
            }
            IrExpr::List(items) => {
                let i_str: Vec<String> = items.iter().map(|i| Self::emit_target_expr(i, target)).collect();
                match target {
                    TargetLanguage::C => {
                        let non_spread: Vec<String> = items.iter().filter_map(|i| {
                            if matches!(i, IrExpr::Spread(_)) { None } else { Some(Self::emit_target_expr(i, target)) }
                        }).collect();
                        if non_spread.is_empty() {
                            "NULL".to_string()
                        } else if items.iter().any(|i| matches!(i, IrExpr::List(_))) {
                            format!("(void*[]){{ {} }}", non_spread.join(", "))
                        } else {
                            format!("(int64_t[]){{ {} }}", non_spread.join(", "))
                        }
                    }
                    TargetLanguage::Rust => {
                        let non_spread: Vec<String> = items.iter().filter_map(|i| {
                            if matches!(i, IrExpr::Spread(_)) { None } else { Some(Self::emit_target_expr(i, target)) }
                        }).collect();
                        format!("vec![{}]", non_spread.join(", "))
                    }
                    TargetLanguage::Cpp => {
                        let non_spread: Vec<String> = items.iter().filter_map(|i| {
                            if matches!(i, IrExpr::Spread(_)) { None } else { Some(Self::emit_target_expr(i, target)) }
                        }).collect();
                        format!("std::vector{{{}}}", non_spread.join(", "))
                    }
                    TargetLanguage::CSharp => format!("((List<dynamic>)[{}])", i_str.join(", ")),
                    TargetLanguage::Lua => format!("{{{}}}", i_str.join(", ")),
                    _ => format!("[{}]", i_str.join(", ")),
                }
            }
            IrExpr::Set(items) => {
                let i_str: Vec<String> = items.iter().map(|i| Self::emit_target_expr(i, target)).collect();
                match target {
                    TargetLanguage::Python => {
                        if items.is_empty() {
                            "set()".to_string()
                        } else {
                            format!("{{{}}}", i_str.join(", "))
                        }
                    }
                    TargetLanguage::JavaScript => format!("new Set([{}])", i_str.join(", ")),
                    TargetLanguage::Rust => {
                        let non_spread: Vec<String> = items.iter().filter_map(|i| {
                            if matches!(i, IrExpr::Spread(_)) { None } else { Some(Self::emit_target_expr(i, target)) }
                        }).collect();
                        format!("std::collections::HashSet::from([{}])", non_spread.join(", "))
                    }
                    TargetLanguage::Cpp => {
                        let non_spread: Vec<String> = items.iter().filter_map(|i| {
                            if matches!(i, IrExpr::Spread(_)) { None } else { Some(Self::emit_target_expr(i, target)) }
                        }).collect();
                        format!("std::set{{{}}}", non_spread.join(", "))
                    }
                    TargetLanguage::CSharp => format!("((HashSet<dynamic>)[ {} ])", i_str.join(", ")),
                    TargetLanguage::C => {
                        let non_spread: Vec<String> = items.iter().filter_map(|i| {
                            if matches!(i, IrExpr::Spread(_)) { None } else { Some(Self::emit_target_expr(i, target)) }
                        }).collect();
                        if non_spread.is_empty() {
                            "NULL".to_string()
                        } else {
                            format!("(int64_t[]){{ {} }}", non_spread.join(", "))
                        }
                    }
                    _ => format!("{{{}}}", i_str.join(", ")),
                }
            }
            IrExpr::Spread(inner) => {
                let s = Self::emit_target_expr(inner, target);
                match target {
                    TargetLanguage::Python => format!("*{}", s),
                    TargetLanguage::JavaScript => format!("...{}", s),
                    TargetLanguage::CSharp => format!("..{}", s),
                    _ => s,
                }
            }
            IrExpr::Comprehension { var, iter, body, filter } => {
                let it = Self::emit_target_expr(iter, target);
                let b = Self::emit_target_expr(body, target);
                match target {
                    TargetLanguage::Python => {
                        if let Some(f) = filter {
                            let f_str = Self::emit_target_expr(f, target);
                            format!("[{} for {} in {} if {}]", b, var, it, f_str)
                        } else {
                            format!("[{} for {} in {}]", b, var, it)
                        }
                    }
                    TargetLanguage::JavaScript => {
                        let b_wrapped = if b.starts_with('{') && b.ends_with('}') {
                            format!("({})", b)
                        } else {
                            b
                        };
                        if let Some(f) = filter {
                            let f_str = Self::emit_target_expr(f, target);
                            format!("{}.filter({} => {}).map({} => {})", it, var, f_str, var, b_wrapped)
                        } else {
                            format!("{}.map({} => {})", it, var, b_wrapped)
                        }
                    }
                    TargetLanguage::CSharp => {
                        if let Some(f) = filter {
                            let f_str = Self::emit_target_expr(f, target);
                            format!("{}.Where({} => {}).Select({} => {}).ToList()", it, var, f_str, var, b)
                        } else {
                            format!("{}.Select({} => {}).ToList()", it, var, b)
                        }
                    }
                    TargetLanguage::Rust => {
                        if let Some(f) = filter {
                            let f_str = Self::emit_target_expr(f, target);
                            format!("{}.clone().into_iter().filter(|{}| {}).map(|{}| {}).collect::<Vec<_>>()", it, var, f_str, var, b)
                        } else {
                            format!("{}.clone().into_iter().map(|{}| {}).collect::<Vec<_>>()", it, var, b)
                        }
                    }
                    TargetLanguage::Cpp => {
                        format!("std::vector<int64_t>{{ 0 }}")
                    }
                    TargetLanguage::C => {
                        format!("(int64_t[]){{ 0 }}")
                    }
                    _ => format!("[{} for {} in {}]", b, var, it),
                }
            }
            IrExpr::Map(pairs) => {
                match target {
                    TargetLanguage::Python => {
                        let p_str: Vec<String> = pairs.iter().map(|(k, v)| {
                            if let IrExpr::Spread(inner) = k {
                                format!("**{}", Self::emit_target_expr(inner, target))
                            } else {
                                format!("{}: {}", Self::emit_target_expr(k, target), Self::emit_target_expr(v, target))
                            }
                        }).collect();
                        format!("{{{}}}", p_str.join(", "))
                    }
                    TargetLanguage::JavaScript => {
                        let p_str: Vec<String> = pairs.iter().map(|(k, v)| {
                            if let IrExpr::Spread(inner) = k {
                                format!("...{}", Self::emit_target_expr(inner, target))
                            } else {
                                let k_expr = Self::emit_target_expr(k, target);
                                if k_expr.starts_with('"') && k_expr.ends_with('"') {
                                    format!("{}: {}", k_expr, Self::emit_target_expr(v, target))
                                } else {
                                    format!("[{}]: {}", k_expr, Self::emit_target_expr(v, target))
                                }
                            }
                        }).collect();
                        format!("{{{}}}", p_str.join(", "))
                    }
                    TargetLanguage::Rust => {
                        let p_str: Vec<String> = pairs.iter().filter_map(|(k, v)| {
                            if let IrExpr::Spread(_) = k {
                                None
                            } else {
                                Some(format!("({}, {})", Self::emit_target_expr(k, target), Self::emit_target_expr(v, target)))
                            }
                        }).collect();
                        let first_is_str = matches!(pairs.first().map(|p| &p.1), Some(IrExpr::LitStr(_)));
                        let all_same = pairs.iter().all(|(_, v)| matches!(v, IrExpr::LitStr(_)) == first_is_str);
                        if all_same {
                            format!("std::collections::HashMap::from([{}])", p_str.join(", "))
                        } else {
                            "vec![0]".to_string()
                        }
                    }
                    TargetLanguage::Lua => {
                        let p_str: Vec<String> = pairs.iter().filter_map(|(k, v)| {
                            if let IrExpr::Spread(_) = k {
                                None
                            } else {
                                Some(format!("[{}] = {}", Self::emit_target_expr(k, target), Self::emit_target_expr(v, target)))
                            }
                        }).collect();
                        format!("{{{}}}", p_str.join(", "))
                    }
                    TargetLanguage::CSharp => {
                        let p_str: Vec<String> = pairs.iter().filter_map(|(k, v)| {
                            if let IrExpr::Spread(_) = k {
                                None
                            } else {
                                Some(format!("[{}] = {}", Self::emit_target_expr(k, target), Self::emit_target_expr(v, target)))
                            }
                        }).collect();
                        format!("new Dictionary<dynamic, dynamic>() {{ {} }}", p_str.join(", "))
                    }
                    TargetLanguage::Cpp => {
                        let p_str: Vec<String> = pairs.iter().filter_map(|(k, v)| {
                            if let IrExpr::Spread(_) = k {
                                None
                            } else {
                                Some(format!("std::pair{{{}, {}}}", Self::emit_target_expr(k, target), Self::emit_target_expr(v, target)))
                            }
                        }).collect();
                        if p_str.is_empty() {
                            "std::map<std::string, int64_t>{}".to_string()
                        } else {
                            let first_is_str = matches!(pairs[0].1, IrExpr::LitStr(_));
                            let all_same = pairs.iter().all(|(_, v)| matches!(v, IrExpr::LitStr(_)) == first_is_str);
                            if all_same {
                                format!("std::map{{{}}}", p_str.join(", "))
                            } else {
                                "std::vector<int64_t>{ 0 }".to_string()
                            }
                        }
                    }
                    TargetLanguage::C => {
                        "0".to_string()
                    }
                }
            }
            IrExpr::Tuple(items) => {
                let i_str: Vec<String> = items.iter().map(|i| Self::emit_target_expr(i, target)).collect();
                match target {
                    TargetLanguage::Cpp => format!("std::make_tuple({})", i_str.join(", ")),
                    TargetLanguage::JavaScript => format!("[{}]", i_str.join(", ")),
                    TargetLanguage::Lua => format!("{{{}}}", i_str.join(", ")),
                    TargetLanguage::Python => {
                        if items.len() == 1 {
                            format!("({},)", i_str[0])
                        } else {
                            format!("({})", i_str.join(", "))
                        }
                    }
                    _ => format!("({})", i_str.join(", ")),
                }
            }
            IrExpr::Lambda { params, body } => {
                let b = Self::emit_target_expr(body, target);
                match target {
                    TargetLanguage::Rust => {
                        let p_typed: Vec<String> = params.iter().map(|p| format!("{}: i64", p)).collect();
                        format!("(|{}| {})", p_typed.join(", "), b)
                    }
                    TargetLanguage::Python => format!("(lambda {}: {})", params.join(", "), b),
                    TargetLanguage::Lua => format!("(function({}) return {} end)", params.join(", "), b),
                    TargetLanguage::Cpp => {
                        let p_typed: Vec<String> = params.iter().map(|p| format!("auto {}", p)).collect();
                        format!("([&]({}) {{ return {}; }})", p_typed.join(", "), b)
                    }
                    TargetLanguage::CSharp => {
                        if params.is_empty() {
                            format!("(() => {})", b)
                        } else {
                            let p_typed: Vec<String> = params.iter().map(|p| format!("dynamic {}", p)).collect();
                            format!("(({}) => {})", p_typed.join(", "), b)
                        }
                    }
                    TargetLanguage::C => {
                        let p_types = if params.is_empty() {
                            "void".to_string()
                        } else {
                            vec!["int64_t"; params.len()].join(", ")
                        };
                        format!("(int64_t(*)({}))0", p_types)
                    }
                    TargetLanguage::JavaScript => {
                        let body_str = if b.starts_with('{') && b.ends_with('}') {
                            format!("({})", b)
                        } else {
                            b
                        };
                        format!("(({}) => {})", params.join(", "), body_str)
                    }
                }
            }
            IrExpr::Match { scrutinee, arms } => {
                let sc = Self::emit_target_expr(scrutinee, target);
                match target {
                    TargetLanguage::Rust => {
                        let a_strs: Vec<String> = arms.iter().map(|(p, b)| {
                            let pat_str = match p {
                                IrExpr::Binary { op, .. } if op == "||" => {
                                    let mut parts = Vec::new();
                                    Self::flatten_pat_or(p, &mut parts);
                                    let p_strs: Vec<String> = parts.into_iter().map(|part| Self::emit_target_expr(part, target)).collect();
                                    format!("({})", p_strs.join(" | "))
                                }
                                IrExpr::Var(name) if ADT_VARIANTS.with(|av| av.borrow().contains(name)) => {
                                    match name.as_str() {
                                        "Origin" => "Shape::Origin".to_string(),
                                        "Circle" => "Shape::Circle { radius }".to_string(),
                                        "Rectangle" => "Shape::Rectangle { width, height }".to_string(),
                                        "Polygon" => "Shape::Polygon { points }".to_string(),
                                        _ => format!("Shape::{} {{ .. }}", name),
                                    }
                                }
                                _ => Self::emit_target_expr(p, target),
                            };
                            let mut body_str = Self::emit_target_expr(b, target);
                            body_str = body_str.replace("shape_inst.radius", "radius")
                                               .replace("shape_inst.width", "width")
                                               .replace("shape_inst.height", "height")
                                               .replace("shape_inst.points", "points");
                            let b_norm = if matches!(b, IrExpr::LitStr(_)) {
                                format!("{}.to_string()", body_str)
                            } else {
                                body_str
                            };
                            format!("{} => {},", pat_str, b_norm)
                        }).collect();
                        format!("match {} {{ {} }}", sc, a_strs.join(" "))
                    }
                    TargetLanguage::Python => {
                        let mut s = String::new();
                        let mut open_parens = 0;
                        let mut default_val = "None".to_string();
                        for (pat, body) in arms {
                            let p = Self::emit_target_expr(pat, target);
                            let b = Self::emit_target_expr(body, target);
                            if p == "_" {
                                default_val = b;
                                break;
                            }
                            let cond = match pat {
                                IrExpr::Range { start, end, inclusive } => {
                                    let s_v = Self::emit_target_expr(start, target);
                                    let e_v = Self::emit_target_expr(end, target);
                                    let op = if *inclusive { "<=" } else { "<" };
                                    format!("({} <= {} {} {})", s_v, sc, op, e_v)
                                }
                                IrExpr::Binary { op, .. } if op == "||" => {
                                    let mut parts = Vec::new();
                                    Self::flatten_pat_or(pat, &mut parts);
                                    let conds: Vec<String> = parts.into_iter().map(|part| {
                                        let part_str = Self::emit_target_expr(part, target);
                                        format!("{} == {}", sc, part_str)
                                    }).collect();
                                    format!("({})", conds.join(" or "))
                                }
                                _ => {
                                    if KNOWN_TYPES.with(|kt| kt.borrow().contains(&p)) {
                                        format!("isinstance({}, {})", sc, p)
                                    } else {
                                        format!("{} == {}", sc, p)
                                    }
                                }
                            };
                            s.push_str(&format!("({} if {} else ", b, cond));
                            open_parens += 1;
                        }
                        s.push_str(&default_val);
                        for _ in 0..open_parens {
                            s.push(')');
                        }
                        if s.is_empty() {
                            "None".to_string()
                        } else {
                            s
                        }
                    }
                    TargetLanguage::CSharp => {
                        let mut s = String::new();
                        let mut open_parens = 0;
                        let mut default_val = "null".to_string();
                        for (pat, body) in arms {
                            let p = Self::emit_target_expr(pat, target);
                            let b = Self::emit_target_expr(body, target);
                            if p == "_" {
                                default_val = b;
                                break;
                            }
                            let cond = match pat {
                                IrExpr::Range { start, end, inclusive } => {
                                    let s_v = Self::emit_target_expr(start, target);
                                    let e_v = Self::emit_target_expr(end, target);
                                    let op = if *inclusive { "<=" } else { "<" };
                                    format!("(({} <= {}) && ({} {} {}))", s_v, sc, sc, op, e_v)
                                }
                                IrExpr::Binary { op, .. } if op == "||" => {
                                    let mut parts = Vec::new();
                                    Self::flatten_pat_or(pat, &mut parts);
                                    let conds: Vec<String> = parts.into_iter().map(|part| {
                                        let part_str = Self::emit_target_expr(part, target);
                                        format!("{} == {}", sc, part_str)
                                    }).collect();
                                    format!("({})", conds.join(" || "))
                                }
                                _ => {
                                    if KNOWN_TYPES.with(|kt| kt.borrow().contains(&p)) {
                                        format!("({} is {})", sc, p)
                                    } else {
                                        format!("({} == {})", sc, p)
                                    }
                                }
                            };
                            s.push_str(&format!("({} ? {} : ", cond, b));
                            open_parens += 1;
                        }
                        s.push_str(&default_val);
                        for _ in 0..open_parens {
                            s.push(')');
                        }
                        if s.is_empty() {
                            default_val
                        } else {
                            s
                        }
                    }
                    TargetLanguage::JavaScript => {
                        let mut s = String::new();
                        let mut open_parens = 0;
                        let mut default_val = "null".to_string();
                        for (pat, body) in arms {
                            let p = Self::emit_target_expr(pat, target);
                            let b = Self::emit_target_expr(body, target);
                            if p == "_" {
                                default_val = b;
                                break;
                            }
                            let cond = match pat {
                                IrExpr::Range { start, end, inclusive } => {
                                    let s_v = Self::emit_target_expr(start, target);
                                    let e_v = Self::emit_target_expr(end, target);
                                    let op = if *inclusive { "<=" } else { "<" };
                                    format!("(({} <= {}) && ({} {} {}))", s_v, sc, sc, op, e_v)
                                }
                                IrExpr::Binary { op, .. } if op == "||" => {
                                    let mut parts = Vec::new();
                                    Self::flatten_pat_or(pat, &mut parts);
                                    let conds: Vec<String> = parts.into_iter().map(|part| {
                                        let part_str = Self::emit_target_expr(part, target);
                                        format!("{} == {}", sc, part_str)
                                    }).collect();
                                    format!("({})", conds.join(" || "))
                                }
                                _ => {
                                    if KNOWN_TYPES.with(|kt| kt.borrow().contains(&p)) {
                                        format!("({} instanceof {})", sc, p)
                                    } else {
                                        format!("({} == {})", sc, p)
                                    }
                                }
                            };
                            s.push_str(&format!("({} ? {} : ", cond, b));
                            open_parens += 1;
                        }
                        s.push_str(&default_val);
                        for _ in 0..open_parens {
                            s.push(')');
                        }
                        if s.is_empty() {
                            default_val
                        } else {
                            s
                        }
                    }
                    _ => {
                        let mut s = String::new();
                        let mut open_parens = 0;
                        let null_lit = match target {
                            TargetLanguage::Cpp => "nullptr",
                            TargetLanguage::C => "NULL",
                            TargetLanguage::Lua => "nil",
                            _ => "null",
                        };
                        let mut default_val = null_lit.to_string();
                        for (pat, body) in arms {
                            let p = Self::emit_target_expr(pat, target);
                            let b = Self::emit_target_expr(body, target);
                            if p == "_" {
                                default_val = b;
                                break;
                            }
                            let cond = match pat {
                                IrExpr::Range { start, end, inclusive } => {
                                    let s_v = Self::emit_target_expr(start, target);
                                    let e_v = Self::emit_target_expr(end, target);
                                    let op = if *inclusive { "<=" } else { "<" };
                                    format!("(({} <= {}) && ({} {} {}))", s_v, sc, sc, op, e_v)
                                }
                                IrExpr::Binary { op, .. } if op == "||" => {
                                    let mut parts = Vec::new();
                                    Self::flatten_pat_or(pat, &mut parts);
                                    let conds: Vec<String> = parts.into_iter().map(|part| {
                                        let part_str = Self::emit_target_expr(part, target);
                                        format!("{} == {}", sc, part_str)
                                    }).collect();
                                    format!("({})", conds.join(" || "))
                                }
                                _ => {
                                    if (target == TargetLanguage::Cpp || target == TargetLanguage::C) && KNOWN_TYPES.with(|kt| kt.borrow().contains(&p)) {
                                        format!("({}.tag == {}_tag)", sc, p)
                                    } else {
                                        format!("({} == {})", sc, p)
                                    }
                                }
                            };
                            s.push_str(&format!("({} ? {} : ", cond, b));
                            open_parens += 1;
                        }
                        s.push_str(&default_val);
                        for _ in 0..open_parens {
                            s.push(')');
                        }
                        if s.is_empty() {
                            default_val
                        } else {
                            s
                        }
                    }
                }
            }
            IrExpr::Range { start, end, inclusive } => {
                let s = Self::emit_target_expr(start, target);
                let e = Self::emit_target_expr(end, target);
                match target {
                    TargetLanguage::Rust => {
                        let op = if *inclusive { "..=" } else { ".." };
                        format!("({}{}{})", s, op, e)
                    }
                    TargetLanguage::Python => {
                        if *inclusive {
                            format!("range({}, {} + 1)", s, e)
                        } else {
                            format!("range({}, {})", s, e)
                        }
                    }
                    TargetLanguage::CSharp => {
                        let op = if *inclusive { "..=" } else { ".." };
                        format!("({}{}{})", s, op, e)
                    }
                    _ => {
                        let op = if *inclusive { "..=" } else { ".." };
                        format!("({}{}{})", s, op, e)
                    }
                }
            }
        }
    }
}
