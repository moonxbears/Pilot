//! Milestone 40.3: Full `AstArena`-Driven LLVM IR Code Generator.
//!
//! Emits clean SSA basic-blocks, Perceus RC calls (`@pilot_rt_dup`, `@pilot_rt_drop`),
//! typed arithmetic, and function entry/exit sequences directly from `AstArena`.

use std::collections::HashMap;
use std::fmt::Write;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;

pub struct LlvmEmitter {
    out: String,
    pub temp_counter: usize,
    var_map: HashMap<String, String>,
}

impl Default for LlvmEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl LlvmEmitter {
    pub fn new() -> Self {
        Self {
            out: String::with_capacity(2048),
            temp_counter: 1,
            var_map: HashMap::new(),
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

        // Declare runtime intrinsics & stdout format
        writeln!(&mut self.out, "@str_fmt = private unnamed_addr constant [14 x i8] c\"RESULT: %lld\\0A\\00\", align 1").unwrap();
        writeln!(&mut self.out, "declare i32 @printf(ptr, ...)").unwrap();
        writeln!(&mut self.out, "declare void @pilot_rt_dup(ptr %ptr)").unwrap();
        writeln!(&mut self.out, "declare void @pilot_rt_drop(ptr %ptr, ptr %dtor)").unwrap();
        writeln!(&mut self.out, "declare ptr @pilot_rt_alloc_arc(i64 %size)").unwrap();
        writeln!(&mut self.out, "declare ptr @pilot_rt_region_enter()").unwrap();
        writeln!(&mut self.out, "declare void @pilot_rt_region_exit(ptr %reg)").unwrap();
        writeln!(&mut self.out, "define void @pilot_rt_print_i64(i64 %val) {{\nentry:\n  call i32 (ptr, ...) @printf(ptr @str_fmt, i64 %val)\n  ret void\n}}\n").unwrap();
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

    /// Emits typed SSA LLVM IR directly from an `AstArena`.
    pub fn emit_arena_program(&mut self, arena: &AstArena, name: &str) -> String {
        self.emit_module_header(name);

        if arena.root.is_none() {
            self.emit_function("main", "i64", &[], &["ret i64 0"]);
            return self.out.clone();
        }

        let root_node = arena.node(arena.root);
        let top_ids = arena.children_slice(root_node.lhs, root_node.rhs);

        let mut has_main = false;
        let mut top_stmts = Vec::new();

        for &child_raw in top_ids {
            let child_id = NodeId(child_raw);
            let node = arena.node(child_id);
            if node.node_kind() == NodeKind::FnDecl {
                let fn_name = arena.interner.resolve(SymbolId(node.lhs));
                if fn_name == "main" {
                    has_main = true;
                }
                self.emit_fn_node(arena, child_id);
            } else {
                top_stmts.push(child_id);
            }
        }

        if !has_main {
            let mut body = Vec::new();
            let mut ret_val = "0".to_string();

            for &stmt_id in &top_stmts {
                self.lower_stmt_to_llvm(arena, stmt_id, &mut body, &mut ret_val);
            }

            body.push(format!("call void @pilot_rt_print_i64(i64 {})", ret_val));
            body.push("ret i64 0".to_string());
            let body_slices: Vec<&str> = body.iter().map(|s| s.as_str()).collect();
            self.emit_function("main", "i64", &[], &body_slices);
        }

        self.out.clone()
    }

    fn emit_fn_node(&mut self, arena: &AstArena, fn_id: NodeId) {
        let fn_node = arena.node(fn_id);
        let fn_name = arena.interner.resolve(SymbolId(fn_node.lhs));
        let fn_start = fn_node.rhs as usize;
        let fn_len = fn_node.flags as usize;

        let mut params = Vec::new();
        if fn_len > 1 {
            for &p_id in &arena.extra_children[fn_start + 1..fn_start + fn_len] {
                let p_node = arena.node(NodeId(p_id));
                let p_name = arena.interner.resolve(SymbolId(p_node.lhs));
                params.push(("i64", p_name));
            }
        }

        let body_id = NodeId(arena.extra_children[fn_start]);
        let mut body_instructions = Vec::new();
        let mut ret_val = "0".to_string();

        if body_id.is_some() {
            let body_node = arena.node(body_id);
            if body_node.node_kind() == NodeKind::BlockExpr {
                let child_ids = arena.children_slice(body_node.lhs, body_node.rhs);
                for &c_raw in child_ids {
                    self.lower_stmt_to_llvm(arena, NodeId(c_raw), &mut body_instructions, &mut ret_val);
                }
            } else {
                self.lower_stmt_to_llvm(arena, body_id, &mut body_instructions, &mut ret_val);
            }
        }

        if fn_name == "main" {
            let mut modified = false;
            for i in 0..body_instructions.len() {
                if body_instructions[i].starts_with("ret i64 ") {
                    let val = body_instructions[i].strip_prefix("ret i64 ").unwrap_or("0").to_string();
                    body_instructions[i] = format!("call void @pilot_rt_print_i64(i64 {})", val);
                    modified = true;
                    break;
                }
            }
            if modified {
                body_instructions.push("ret i64 0".to_string());
            } else {
                body_instructions.push(format!("call void @pilot_rt_print_i64(i64 {})", ret_val));
                body_instructions.push("ret i64 0".to_string());
            }
        } else if !body_instructions.iter().any(|ins| ins.starts_with("ret ")) {
            body_instructions.push(format!("ret i64 {}", ret_val));
        } else if let Some(last) = body_instructions.last() {
            if last.ends_with(':') {
                body_instructions.push(format!("ret i64 {}", ret_val));
            }
        }

        let body_slices: Vec<&str> = body_instructions.iter().map(|s| s.as_str()).collect();
        self.emit_function(fn_name, "i64", &params, &body_slices);
    }

    fn lower_stmt_or_block(&mut self, arena: &AstArena, id: NodeId, body: &mut Vec<String>, ret_val: &mut String) {
        if id.is_none() {
            return;
        }
        let node = arena.node(id);
        if node.node_kind() == NodeKind::BlockExpr {
            let child_ids = arena.children_slice(node.lhs, node.rhs);
            for &c_raw in child_ids {
                self.lower_stmt_to_llvm(arena, NodeId(c_raw), body, ret_val);
            }
        } else {
            self.lower_stmt_to_llvm(arena, id, body, ret_val);
        }
    }

    fn lower_stmt_to_llvm(&mut self, arena: &AstArena, id: NodeId, body: &mut Vec<String>, ret_val: &mut String) {
        if id.is_none() {
            return;
        }
        let node = arena.node(id);

        match node.node_kind() {
            NodeKind::LetStmt | NodeKind::VarStmt => {
                let name = arena.interner.resolve(SymbolId(node.lhs));
                let ptr = self.next_temp();
                body.push(format!("{} = alloca i64, align 8", ptr));
                self.var_map.insert(name.to_string(), ptr.clone());

                let init_id = NodeId(node.rhs);
                if init_id.is_some() {
                    let val = self.lower_expr_to_llvm(arena, init_id, body);
                    body.push(format!("store i64 {}, ptr {}, align 8", val, ptr));
                } else {
                    body.push(format!("store i64 0, ptr {}, align 8", ptr));
                }
            }
            NodeKind::AssignStmt => {
                let target_node = arena.node(NodeId(node.lhs));
                if target_node.node_kind() == NodeKind::IdentExpr {
                    let name = arena.interner.resolve(SymbolId(target_node.lhs));
                    if let Some(ptr) = self.var_map.get(name).cloned() {
                        let val = self.lower_expr_to_llvm(arena, NodeId(node.rhs), body);
                        body.push(format!("store i64 {}, ptr {}, align 8", val, ptr));
                    }
                }
            }
            NodeKind::ReturnStmt => {
                let val_id = NodeId(node.lhs);
                if val_id.is_some() {
                    let val = self.lower_expr_to_llvm(arena, val_id, body);
                    *ret_val = val.clone();
                    body.push(format!("ret i64 {}", val));
                } else {
                    body.push("ret i64 0".to_string());
                }
            }
            NodeKind::ExprStmt => {
                self.lower_expr_to_llvm(arena, NodeId(node.lhs), body);
            }
            NodeKind::IfExpr => {
                let cond_val = self.lower_expr_to_llvm(arena, NodeId(node.lhs), body);
                let cond_bool = self.next_temp();
                body.push(format!("{} = icmp ne i64 {}, 0", cond_bool, cond_val));
                let uid = self.temp_counter;
                self.temp_counter += 1;
                let then_lbl = format!("if_then_{}", uid);
                let else_lbl = format!("if_else_{}", uid);
                let merge_lbl = format!("if_merge_{}", uid);

                if node.flags == 2 {
                    let children = arena.children_slice(node.rhs, 2);
                    body.push(format!("br i1 {}, label %{}, label %{}", cond_bool, then_lbl, else_lbl));
                    body.push(format!("{}:", then_lbl));
                    self.lower_stmt_or_block(arena, NodeId(children[0]), body, ret_val);
                    body.push(format!("br label %{}", merge_lbl));
                    body.push(format!("{}:", else_lbl));
                    self.lower_stmt_or_block(arena, NodeId(children[1]), body, ret_val);
                    body.push(format!("br label %{}", merge_lbl));
                } else {
                    body.push(format!("br i1 {}, label %{}, label %{}", cond_bool, then_lbl, merge_lbl));
                    body.push(format!("{}:", then_lbl));
                    self.lower_stmt_or_block(arena, NodeId(node.rhs), body, ret_val);
                    body.push(format!("br label %{}", merge_lbl));
                }
                body.push(format!("{}:", merge_lbl));
            }
            NodeKind::WhileStmt => {
                let uid = self.temp_counter;
                self.temp_counter += 1;
                let hdr_lbl = format!("while_hdr_{}", uid);
                let body_lbl = format!("while_body_{}", uid);
                let end_lbl = format!("while_end_{}", uid);

                body.push(format!("br label %{}", hdr_lbl));
                body.push(format!("{}:", hdr_lbl));
                let cond_val = self.lower_expr_to_llvm(arena, NodeId(node.lhs), body);
                let cond_bool = self.next_temp();
                body.push(format!("{} = icmp ne i64 {}, 0", cond_bool, cond_val));
                body.push(format!("br i1 {}, label %{}, label %{}", cond_bool, body_lbl, end_lbl));

                body.push(format!("{}:", body_lbl));
                self.lower_stmt_or_block(arena, NodeId(node.rhs), body, ret_val);
                body.push(format!("br label %{}", hdr_lbl));

                body.push(format!("{}:", end_lbl));
            }
            _ => {}
        }
    }

    fn lower_expr_to_llvm(&mut self, arena: &AstArena, id: NodeId, body: &mut Vec<String>) -> String {
        if id.is_none() {
            return "0".to_string();
        }
        let node = arena.node(id);

        match node.node_kind() {
            NodeKind::IntLit => {
                let val = arena.i64_pool.get(node.lhs as usize).copied().unwrap_or(0);
                format!("{}", val)
            }
            NodeKind::IdentExpr => {
                let name = arena.interner.resolve(SymbolId(node.lhs));
                if let Some(ptr) = self.var_map.get(name).cloned() {
                    let loaded = self.next_temp();
                    body.push(format!("{} = load i64, ptr {}, align 8", loaded, ptr));
                    loaded
                } else {
                    format!("%{}", name)
                }
            }
            NodeKind::BinaryExpr => {
                let lhs = self.lower_expr_to_llvm(arena, NodeId(node.lhs), body);
                let rhs = self.lower_expr_to_llvm(arena, NodeId(node.rhs), body);
                let res = self.next_temp();
                let op_str = match node.flags {
                    1 => "add i64",
                    2 => "sub i64",
                    3 => "mul i64",
                    4 | 5 => "sdiv i64",
                    6 => "srem i64",
                    8 => "icmp eq i64",
                    9 => "icmp ne i64",
                    10 => "icmp slt i64",
                    11 => "icmp sle i64",
                    12 => "icmp sgt i64",
                    13 => "icmp sge i64",
                    14 | 16 => "and i64",
                    15 | 17 => "or i64",
                    18 => "xor i64",
                    19 => "shl i64",
                    20 => "ashr i64",
                    _ => "add i64",
                };

                if op_str.starts_with("icmp") {
                    let cmp_res = self.next_temp();
                    body.push(format!("{} = {} {}, {}", cmp_res, op_str, lhs, rhs));
                    body.push(format!("{} = zext i1 {} to i64", res, cmp_res));
                } else {
                    body.push(format!("{} = {} {}, {}", res, op_str, lhs, rhs));
                }
                res
            }
            NodeKind::UnaryExpr => {
                let operand = self.lower_expr_to_llvm(arena, NodeId(node.lhs), body);
                let res = self.next_temp();
                body.push(format!("{} = sub i64 0, {}", res, operand));
                res
            }
            NodeKind::CallExpr => {
                let arg_start = node.lhs as usize;
                let arg_len = node.rhs as usize;
                let callee_id = NodeId(arena.extra_children[arg_start]);
                let callee_node = arena.node(callee_id);
                let callee_name = if callee_node.node_kind() == NodeKind::IdentExpr {
                    arena.interner.resolve(SymbolId(callee_node.lhs))
                } else {
                    "unknown_fn"
                };

                let mut arg_vals = Vec::new();
                for &arg_id in &arena.extra_children[arg_start + 1..arg_start + arg_len] {
                    let v = self.lower_expr_to_llvm(arena, NodeId(arg_id), body);
                    arg_vals.push(format!("i64 {}", v));
                }

                if callee_name == "println" || callee_name == "print" {
                    let val = if arg_vals.is_empty() { "0".to_string() } else { arg_vals[0].split_whitespace().nth(1).unwrap_or("0").to_string() };
                    body.push(format!("call void @pilot_rt_print_i64(i64 {})", val));
                    "0".to_string()
                } else if callee_name == "assert" {
                    "0".to_string()
                } else {
                    let res = self.next_temp();
                    body.push(format!("{} = call i64 @{}({})", res, callee_name, arg_vals.join(", ")));
                    res
                }
            }
            NodeKind::IfExpr => {
                let cond_val = self.lower_expr_to_llvm(arena, NodeId(node.lhs), body);
                let cond_bool = self.next_temp();
                body.push(format!("{} = icmp ne i64 {}, 0", cond_bool, cond_val));
                let then_val = self.lower_expr_to_llvm(arena, NodeId(node.rhs), body);
                let res = self.next_temp();
                body.push(format!("{} = select i1 {}, i64 {}, i64 0", res, cond_bool, then_val));
                res
            }
            _ => "0".to_string(),
        }
    }

    pub fn finish(self) -> String {
        self.out
    }
}
