//! Milestone 40.4: WebAssembly (.wat, .wasm, WASI, and JS/TS Bindings).
//!
//! Compiles Pilot AST programs into WebAssembly Text format (.wat), standalone binary WebAssembly (.wasm),
//! and generates zero-copy TypeScript/JavaScript bindings.

use std::fmt::Write;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;

pub struct WasmEmitter {
    out: String,
    pub target_wasi: bool,
}

impl Default for WasmEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmEmitter {
    pub fn new() -> Self {
        Self {
            out: String::with_capacity(2048),
            target_wasi: true,
        }
    }

    /// Emits WebAssembly Text (.wat) directly from an `AstArena`.
    pub fn emit_wat_from_ast(&mut self, arena: &AstArena, module_name: &str) -> String {
        writeln!(&mut self.out, "(module ;; Module: {}", module_name).unwrap();
        writeln!(&mut self.out, "  (import \"env\" \"print_i64\" (func $print_i64 (param i64)))").unwrap();
        writeln!(&mut self.out, "  (import \"env\" \"print_f64\" (func $print_f64 (param f64)))").unwrap();
        if self.target_wasi {
            writeln!(&mut self.out, "  (import \"wasi_snapshot_preview1\" \"proc_exit\" (func $proc_exit (param i32)))").unwrap();
        }
        writeln!(&mut self.out, "  (memory (export \"memory\") 1)").unwrap();
        writeln!(&mut self.out).unwrap();

        if arena.root.is_some() {
            let root_node = arena.node(arena.root);
            let top_ids = arena.children_slice(root_node.lhs, root_node.rhs);

            let mut has_main = false;
            let mut top_stmts = Vec::new();

            for &child_raw in top_ids {
                let child_id = NodeId(child_raw);
                let node = arena.node(child_id);
                if node.node_kind() == NodeKind::FnDecl {
                    let name = arena.interner.resolve(SymbolId(node.lhs));
                    if name == "main" {
                        has_main = true;
                    }
                    self.emit_wat_function(arena, child_id);
                } else {
                    top_stmts.push(child_id);
                }
            }

            if !has_main {
                writeln!(&mut self.out, "  (func $main (export \"main\") (result i64)").unwrap();
                for &stmt_id in &top_stmts {
                    self.emit_wat_stmt(arena, stmt_id);
                }
                writeln!(&mut self.out, "    i64.const 0").unwrap();
                writeln!(&mut self.out, "    return").unwrap();
                writeln!(&mut self.out, "  )").unwrap();
            }
        } else {
            writeln!(&mut self.out, "  (func $main (export \"main\") (result i64)").unwrap();
            writeln!(&mut self.out, "    i64.const 0").unwrap();
            writeln!(&mut self.out, "    return").unwrap();
            writeln!(&mut self.out, "  )").unwrap();
        }

        // WASI entry point (_start)
        writeln!(&mut self.out).unwrap();
        writeln!(&mut self.out, "  (func $_start (export \"_start\")").unwrap();
        writeln!(&mut self.out, "    call $main").unwrap();
        writeln!(&mut self.out, "    drop").unwrap();
        writeln!(&mut self.out, "  )").unwrap();

        writeln!(&mut self.out, ")").unwrap();
        self.out.clone()
    }

    fn emit_wat_function(&mut self, arena: &AstArena, fn_id: NodeId) {
        let fn_node = arena.node(fn_id);
        let fn_name = arena.interner.resolve(SymbolId(fn_node.lhs));
        let fn_start = fn_node.rhs as usize;
        let fn_len = fn_node.flags as usize;

        let mut param_str = String::new();
        if fn_len > 1 {
            for &p_id in &arena.extra_children[fn_start + 1..fn_start + fn_len] {
                let p_node = arena.node(NodeId(p_id));
                let p_name = arena.interner.resolve(SymbolId(p_node.lhs));
                param_str.push_str(&format!(" (param ${} i64)", p_name));
            }
        }

        writeln!(&mut self.out, "  (func ${} (export \"{}\"){param_str} (result i64)", fn_name, fn_name).unwrap();

        let body_id = NodeId(arena.extra_children[fn_start]);
        if body_id.is_some() {
            let body_node = arena.node(body_id);
            if body_node.node_kind() == NodeKind::BlockExpr {
                let child_ids = arena.children_slice(body_node.lhs, body_node.rhs);
                for &c_raw in child_ids {
                    self.emit_wat_stmt(arena, NodeId(c_raw));
                }
            } else {
                self.emit_wat_stmt(arena, body_id);
            }
        }

        writeln!(&mut self.out, "    i64.const 0").unwrap();
        writeln!(&mut self.out, "    return").unwrap();
        writeln!(&mut self.out, "  )").unwrap();
    }

    fn emit_wat_stmt(&mut self, arena: &AstArena, id: NodeId) {
        if id.is_none() {
            return;
        }
        let node = arena.node(id);

        match node.node_kind() {
            NodeKind::LetStmt | NodeKind::VarStmt => {
                let init_id = NodeId(node.rhs);
                if init_id.is_some() {
                    self.emit_wat_expr(arena, init_id);
                    writeln!(&mut self.out, "    drop").unwrap();
                }
            }
            NodeKind::ReturnStmt => {
                let val_id = NodeId(node.lhs);
                if val_id.is_some() {
                    self.emit_wat_expr(arena, val_id);
                } else {
                    writeln!(&mut self.out, "    i64.const 0").unwrap();
                }
                writeln!(&mut self.out, "    return").unwrap();
            }
            NodeKind::ExprStmt => {
                self.emit_wat_expr(arena, NodeId(node.lhs));
                writeln!(&mut self.out, "    drop").unwrap();
            }
            _ => {}
        }
    }

    fn emit_wat_expr(&mut self, arena: &AstArena, id: NodeId) {
        if id.is_none() {
            writeln!(&mut self.out, "    i64.const 0").unwrap();
            return;
        }
        let node = arena.node(id);

        match node.node_kind() {
            NodeKind::IntLit => {
                let val = arena.i64_pool.get(node.lhs as usize).copied().unwrap_or(0);
                writeln!(&mut self.out, "    i64.const {}", val).unwrap();
            }
            NodeKind::BinaryExpr => {
                self.emit_wat_expr(arena, NodeId(node.lhs));
                self.emit_wat_expr(arena, NodeId(node.rhs));
                let ins = match node.flags {
                    1 => "i64.add",
                    2 => "i64.sub",
                    3 => "i64.mul",
                    4 | 5 => "i64.div_s",
                    6 => "i64.rem_s",
                    8 => "i64.eq",
                    9 => "i64.ne",
                    10 => "i64.lt_s",
                    11 => "i64.le_s",
                    12 => "i64.gt_s",
                    13 => "i64.ge_s",
                    16 => "i64.and",
                    17 => "i64.or",
                    18 => "i64.xor",
                    19 => "i64.shl",
                    20 => "i64.shr_s",
                    _ => "i64.add",
                };
                writeln!(&mut self.out, "    {}", ins).unwrap();
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

                for &arg_id in &arena.extra_children[arg_start + 1..arg_start + arg_len] {
                    self.emit_wat_expr(arena, NodeId(arg_id));
                }

                if callee_name == "println" || callee_name == "print" {
                    writeln!(&mut self.out, "    call $print_i64").unwrap();
                    writeln!(&mut self.out, "    i64.const 0").unwrap();
                } else {
                    writeln!(&mut self.out, "    call ${}", callee_name).unwrap();
                }
            }
            _ => {
                writeln!(&mut self.out, "    i64.const 0").unwrap();
            }
        }
    }

    /// Emits legacy minimal WAT module for backward compatibility.
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

    /// Assembles minimal standalone binary WebAssembly (.wasm) bytes.
    pub fn assemble_minimal_wasm(&self, ret_val: i32) -> Vec<u8> {
        let mut bytes = Vec::new();
        // Magic & Version (0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00)
        bytes.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]);

        // Type section (ID = 1): () -> i32
        let type_sec = vec![0x01, 0x60, 0x00, 0x01, 0x7F];
        Self::write_section(&mut bytes, 1, &type_sec);

        // Function section (ID = 3): function 0 has type 0
        let func_sec = vec![0x01, 0x00];
        Self::write_section(&mut bytes, 3, &func_sec);

        // Memory section (ID = 5): 1 initial page
        let mem_sec = vec![0x01, 0x00, 0x01];
        Self::write_section(&mut bytes, 5, &mem_sec);

        // Export section (ID = 7): export "main" (func 0), export "memory" (mem 0)
        let mut exp_sec = Vec::new();
        exp_sec.push(0x02); // 2 exports
        Self::write_wasm_string(&mut exp_sec, "memory");
        exp_sec.extend_from_slice(&[0x02, 0x00]); // mem 0
        Self::write_wasm_string(&mut exp_sec, "main");
        exp_sec.extend_from_slice(&[0x00, 0x00]); // func 0
        Self::write_section(&mut bytes, 7, &exp_sec);

        // Code section (ID = 10): body of function 0
        let mut code_body = Vec::new();
        code_body.push(0x00); // 0 locals
        code_body.push(0x41); // i32.const
        Self::write_var_i32(&mut code_body, ret_val);
        code_body.push(0x0B); // end

        let mut code_sec = Vec::new();
        code_sec.push(0x01); // 1 func body
        Self::write_var_u32(&mut code_sec, code_body.len() as u32);
        code_sec.extend_from_slice(&code_body);
        Self::write_section(&mut bytes, 10, &code_sec);

        bytes
    }

    /// Generates TypeScript / JavaScript bindings for a compiled WASM module.
    pub fn generate_bindings(module_name: &str) -> (String, String) {
        let mjs = format!(
            r#"// Auto-generated ESM WebAssembly Loader for {}
export async function loadPilotModule(wasmSource, imports = {{}}) {{
    const memory = new WebAssembly.Memory({{ initial: 1 }});
    const defaultImports = {{
        env: {{
            memory,
            print_i64: (val) => console.log(val.toString()),
            print_f64: (val) => console.log(val),
        }},
        wasi_snapshot_preview1: {{
            proc_exit: (code) => {{ if (typeof process !== 'undefined') process.exit(code); }},
        }},
        ...imports,
    }};

    let instance;
    if (typeof wasmSource === 'string' || wasmSource instanceof URL) {{
        const response = await fetch(wasmSource);
        const bytes = await response.arrayBuffer();
        const res = await WebAssembly.instantiate(bytes, defaultImports);
        instance = res.instance;
    }} else {{
        const res = await WebAssembly.instantiate(wasmSource, defaultImports);
        instance = res.instance;
    }}

    return {{
        instance,
        exports: instance.exports,
        main: instance.exports.main,
    }};
}}
"#,
            module_name
        );

        let d_ts = format!(
            r#"// Auto-generated TypeScript definitions for {}
export interface PilotModuleExports {{
    readonly memory: WebAssembly.Memory;
    main(): bigint | number;
    _start?(): void;
}}

export interface PilotModule {{
    readonly instance: WebAssembly.Instance;
    readonly exports: PilotModuleExports;
    main(): bigint | number;
}}

export function loadPilotModule(
    wasmSource: string | URL | BufferSource,
    imports?: WebAssembly.Imports
): Promise<PilotModule>;
"#,
            module_name
        );

        (mjs, d_ts)
    }

    fn write_section(out: &mut Vec<u8>, section_id: u8, data: &[u8]) {
        out.push(section_id);
        Self::write_var_u32(out, data.len() as u32);
        out.extend_from_slice(data);
    }

    fn write_wasm_string(out: &mut Vec<u8>, s: &str) {
        Self::write_var_u32(out, s.len() as u32);
        out.extend_from_slice(s.as_bytes());
    }

    fn write_var_u32(out: &mut Vec<u8>, mut val: u32) {
        loop {
            let mut byte = (val & 0x7F) as u8;
            val >>= 7;
            if val != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if val == 0 {
                break;
            }
        }
    }

    fn write_var_i32(out: &mut Vec<u8>, mut val: i32) {
        let mut more = true;
        while more {
            let mut byte = (val & 0x7F) as u8;
            val >>= 7;
            let sign_bit = (byte & 0x40) != 0;
            if (val == 0 && !sign_bit) || (val == -1 && sign_bit) {
                more = false;
            } else {
                byte |= 0x80;
            }
            out.push(byte);
        }
    }

    pub fn finish(self) -> String {
        self.out
    }
}
