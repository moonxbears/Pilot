//! Milestone 33.1 & 37.1: Flow-Sensitive Bidirectional Type Inference, Complete AstArena Coverage & Narrowing.

use std::collections::HashMap;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use crate::diagnostics::AnalysisDiagnostic;
use crate::types::{TypeArena, TypeId, TypeKind};

pub struct Scope {
    pub bindings: HashMap<String, TypeId>,
    pub narrowed_types: HashMap<String, TypeId>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            narrowed_types: HashMap::new(),
        }
    }
}

pub struct TypeInferenceEngine {
    pub types: TypeArena,
    scopes: Vec<Scope>,
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub inferred_node_types: HashMap<u32, TypeId>,
    pub type_defs: HashMap<String, TypeId>,
    pub struct_fields: HashMap<String, Vec<(String, TypeId)>>,
    pub adt_variants: HashMap<String, (String, Vec<TypeId>)>, // variant -> (adt_name, field_types)
}

impl Default for TypeInferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeInferenceEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            types: TypeArena::new(),
            scopes: vec![Scope::new()],
            diagnostics: Vec::new(),
            inferred_node_types: HashMap::new(),
            type_defs: HashMap::new(),
            struct_fields: HashMap::new(),
            adt_variants: HashMap::new(),
        };

        engine.init_builtins();
        engine
    }

    fn init_builtins(&mut self) {
        // Standard primitives in type_defs
        self.type_defs.insert("int".to_string(), TypeId::I64);
        self.type_defs.insert("i64".to_string(), TypeId::I64);
        self.type_defs.insert("i32".to_string(), TypeId::I64);
        self.type_defs.insert("i16".to_string(), TypeId::I64);
        self.type_defs.insert("i8".to_string(), TypeId::I64);
        self.type_defs.insert("uint".to_string(), TypeId::I64);
        self.type_defs.insert("u64".to_string(), TypeId::I64);
        self.type_defs.insert("u32".to_string(), TypeId::I64);
        self.type_defs.insert("u16".to_string(), TypeId::I64);
        self.type_defs.insert("u8".to_string(), TypeId::I64);
        self.type_defs.insert("usize".to_string(), TypeId::I64);
        self.type_defs.insert("isize".to_string(), TypeId::I64);
        self.type_defs.insert("byte".to_string(), TypeId::I64);
        self.type_defs.insert("float".to_string(), TypeId::F64);
        self.type_defs.insert("f64".to_string(), TypeId::F64);
        self.type_defs.insert("f32".to_string(), TypeId::F64);
        self.type_defs.insert("f16".to_string(), TypeId::F64);
        self.type_defs.insert("f8".to_string(), TypeId::F64);
        self.type_defs.insert("double".to_string(), TypeId::F64);
        self.type_defs.insert("decimal".to_string(), TypeId::F64);
        self.type_defs.insert("bool".to_string(), TypeId::BOOL);
        self.type_defs.insert("str".to_string(), TypeId::STR);
        self.type_defs.insert("string".to_string(), TypeId::STR);
        self.type_defs.insert("bytes".to_string(), TypeId::STR);
        self.type_defs.insert("bytearray".to_string(), TypeId::STR);
        self.type_defs.insert("char".to_string(), TypeId::I64);
        self.type_defs.insert("int64_t".to_string(), TypeId::I64);
        self.type_defs.insert("uint64_t".to_string(), TypeId::I64);
        self.type_defs.insert("timespec".to_string(), TypeId::ANY);
        self.type_defs.insert("struct".to_string(), TypeId::ANY);
        self.type_defs.insert("void".to_string(), TypeId::VOID);
        self.type_defs.insert("unit".to_string(), TypeId::VOID);
        self.type_defs.insert("never".to_string(), TypeId::NEVER);
        self.type_defs.insert("any".to_string(), TypeId::ANY);
        self.type_defs.insert("dynamic".to_string(), TypeId::ANY);
        let sym_ty = self.types.alloc(TypeKind::Primitive("symbol".to_string()));
        self.type_defs.insert("symbol".to_string(), sym_ty);
        self.type_defs.insert("keyword".to_string(), sym_ty);
        let dur_ty = self.types.unit_of(TypeId::I64, "Duration".to_string());
        let freq_ty = self.types.unit_of(TypeId::I64, "Frequency".to_string());
        let mem_ty = self.types.unit_of(TypeId::I64, "Memory".to_string());
        self.type_defs.insert("Duration".to_string(), dur_ty);
        self.type_defs.insert("Frequency".to_string(), freq_ty);
        self.type_defs.insert("Memory".to_string(), mem_ty);

        // Standard functions in scope
        let fn_print = self.types.function_of(vec![TypeId::ANY], TypeId::VOID);
        self.define_var("print", fn_print);
        self.define_var("println", fn_print);

        let fn_assert = self.types.function_of(vec![TypeId::BOOL], TypeId::VOID);
        self.define_var("assert", fn_assert);

        let fn_panic = self.types.function_of(vec![TypeId::STR], TypeId::NEVER);
        self.define_var("panic", fn_panic);

        let fn_len = self.types.function_of(vec![TypeId::ANY], TypeId::I64);
        self.define_var("len", fn_len);

        let int_list = self.types.list_of(TypeId::I64);
        let fn_range = self.types.function_of(vec![TypeId::I64], int_list);
        self.define_var("range", fn_range);

        let fn_malloc = self.types.function_of(vec![TypeId::I64], TypeId::ANY);
        self.define_var("malloc", fn_malloc);

        let fn_free = self.types.function_of(vec![TypeId::ANY], TypeId::VOID);
        self.define_var("free", fn_free);

        let fn_memset = self.types.function_of(vec![TypeId::ANY, TypeId::I64, TypeId::I64], TypeId::ANY);
        self.define_var("memset", fn_memset);

        let fn_clock = self.types.function_of(vec![TypeId::I64, TypeId::ANY], TypeId::I64);
        self.define_var("clock_gettime", fn_clock);

        let fn_puts = self.types.function_of(vec![TypeId::ANY], TypeId::VOID);
        self.define_var("puts", fn_puts);

        let fn_printf = self.types.function_of(vec![TypeId::ANY], TypeId::VOID);
        self.define_var("printf", fn_printf);

        let fn_sprintf = self.types.function_of(vec![TypeId::ANY], TypeId::I64);
        self.define_var("sprintf", fn_sprintf);

        let fn_fopen = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], TypeId::ANY);
        self.define_var("fopen", fn_fopen);

        let fn_fclose = self.types.function_of(vec![TypeId::ANY], TypeId::I64);
        self.define_var("fclose", fn_fclose);

        let fn_fputs = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], TypeId::I64);
        self.define_var("fputs", fn_fputs);

        let fn_fgets = self.types.function_of(vec![TypeId::ANY, TypeId::ANY, TypeId::ANY], TypeId::ANY);
        self.define_var("fgets", fn_fgets);

        let fn_strcmp = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], TypeId::I64);
        self.define_var("strcmp", fn_strcmp);

        let fn_strlen = self.types.function_of(vec![TypeId::ANY], TypeId::I64);
        self.define_var("strlen", fn_strlen);

        let fn_system = self.types.function_of(vec![TypeId::ANY], TypeId::I64);
        self.define_var("system", fn_system);

        let fn_exit = self.types.function_of(vec![TypeId::I64], TypeId::VOID);
        self.define_var("exit", fn_exit);
        self.define_var("CLOCK_MONOTONIC", TypeId::I64);

        // Result and Option constructors
        let fn_ok = self.types.function_of(vec![TypeId::ANY], TypeId::ANY);
        self.define_var("Ok", fn_ok);
        let fn_err = self.types.function_of(vec![TypeId::ANY], TypeId::ANY);
        self.define_var("Err", fn_err);
        let fn_some = self.types.function_of(vec![TypeId::ANY], TypeId::ANY);
        self.define_var("Some", fn_some);
        self.define_var("None", TypeId::NULL);
        self.define_var("null", TypeId::NULL);
        self.define_var("True", TypeId::BOOL);
        self.define_var("False", TypeId::BOOL);

        self.define_var("some", fn_some);
        self.define_var("none", TypeId::NULL);
        self.define_var("nil", TypeId::NULL);
        self.define_var("ok", fn_ok);
        self.define_var("err", fn_err);
        self.define_var("true", TypeId::BOOL);
        self.define_var("false", TypeId::BOOL);

        // Concurrency & Async
        let fn_async = self.types.function_of(vec![TypeId::ANY], TypeId::ANY);
        self.define_var("async", fn_async);
        self.define_var("await", fn_async);
        self.define_var("spawn", fn_async);
        self.define_var("atomic", fn_async);
        self.define_var("sync", fn_async);
        self.define_var("isolate", fn_async);
        self.define_var("select", fn_async);
        self.define_var("parallel", fn_async);
        self.define_var("timeout", fn_async);
        self.define_var("yield_execution", fn_async);

        // Realtime
        self.define_var("deadline", fn_async);
        self.define_var("within", fn_async);
        self.define_var("periodic", fn_async);
        self.define_var("priority", fn_async);
        self.define_var("jitter", fn_async);
        self.define_var("stackalloc", fn_async);
        self.define_var("bounded", fn_async);
        self.define_var("on_missed", fn_async);
        self.define_var("fallback", fn_async);

        // Meta, Keywords & DSL
        self.define_var("default", fn_async);
        self.define_var("keyword", fn_async);
        self.define_var("defkeyword", fn_async);
        self.define_var("pure", fn_async);
        self.define_var("noalloc", fn_async);
        self.define_var("unsafe", fn_async);
        self.define_var("current_scope", TypeId::ANY);
        self.define_var("parallel_map", fn_async);
        self.define_var("lazy", fn_async);
        self.define_var("json", TypeId::ANY);
        self.define_var("__name__", TypeId::STR);
        self.define_var("cons", fn_async);
        self.define_var("car", fn_async);
        self.define_var("cdr", fn_async);
        let fn_atom = self.types.function_of(vec![TypeId::ANY], TypeId::BOOL);
        self.define_var("atom", fn_atom);
        self.define_var("venv", TypeId::ANY);
        let fn_sizeof = self.types.function_of(vec![TypeId::ANY], TypeId::I64);
        self.define_var("sizeof_type", fn_sizeof);
        self.define_var("option_has_niche", fn_atom);
        let fn_ignore_err = self.types.function_of(vec![TypeId::ANY], TypeId::VOID);
        self.define_var("ignore_err", fn_ignore_err);
        self.define_var("signal", fn_async);
        self.define_var("fail", fn_async);
        self.define_var("shared_buffer", fn_async);
        let fn_binop = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], TypeId::ANY);
        self.define_var("+", fn_binop);
        self.define_var("-", fn_binop);
        self.define_var("*", fn_binop);
        self.define_var("/", fn_binop);
        self.define_var("%", fn_binop);
        self.define_var("==", fn_binop);

        // Polyglot
        self.define_var("js", fn_async);
        self.define_var("lua", fn_async);
        self.define_var("python", fn_async);
        self.define_var("java", fn_async);
        self.define_var("c", fn_async);
        self.define_var("cpp", fn_async);
        self.define_var("asm", fn_async);
        self.define_var("rust", fn_async);
        self.define_var("csharp", fn_async);
        self.define_var("sql", fn_async);
        self.define_var("postgres", fn_async);
        self.define_var("postgresql", fn_async);
        self.define_var("mysql", fn_async);
        self.define_var("sqlite", fn_async);

        // Types and math constants
        self.define_var("PI", TypeId::F64);
        self.define_var("E", TypeId::F64);
        self.define_var("Math", TypeId::ANY);
        self.define_var("undefined", TypeId::ANY);
        self.define_var("Channel", fn_async);
        self.define_var("Chan", fn_async);
        self.define_var("old", fn_async);
        self.define_var("result", TypeId::ANY);
        self.define_var("Exception", TypeId::ANY);
        self.define_var("Error", TypeId::ANY);
        self.define_var("DivideByZeroError", TypeId::ANY);
        self.define_var("NullReferenceError", TypeId::ANY);
        self.define_var("TypeError", TypeId::ANY);
        self.define_var("ValueError", fn_async);
        self.define_var("KeyError", fn_async);
        self.define_var("IndexError", fn_async);
        self.define_var("RuntimeError", fn_async);
        self.define_var("ZeroDivisionError", fn_async);
        self.define_var("AssertionError", TypeId::ANY);
        self.type_defs.insert("Channel".to_string(), TypeId::ANY);
        self.type_defs.insert("Chan".to_string(), TypeId::ANY);
        self.type_defs.insert("Exception".to_string(), TypeId::ANY);
        self.type_defs.insert("Error".to_string(), TypeId::ANY);
        self.type_defs.insert("DivideByZeroError".to_string(), TypeId::ANY);
        self.type_defs.insert("NullReferenceError".to_string(), TypeId::ANY);
        self.type_defs.insert("TypeError".to_string(), TypeId::ANY);
        self.type_defs.insert("AssertionError".to_string(), TypeId::ANY);
        self.type_defs.insert("list".to_string(), self.types.list_of(TypeId::ANY));
        self.type_defs.insert("map".to_string(), TypeId::ANY);
        self.type_defs.insert("tuple".to_string(), TypeId::ANY);

        // Standard Math Functions
        let fn_math_1 = self.types.function_of(vec![TypeId::F64], TypeId::F64);
        let fn_math_2 = self.types.function_of(vec![TypeId::F64, TypeId::F64], TypeId::F64);
        let fn_math_pred = self.types.function_of(vec![TypeId::F64], TypeId::BOOL);
        let fn_any_2 = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], TypeId::ANY);
        let fn_any_1 = self.types.function_of(vec![TypeId::ANY], TypeId::ANY);

        self.define_var("sqrt", fn_math_1);
        self.define_var("sin", fn_math_1);
        self.define_var("cos", fn_math_1);
        self.define_var("tan", fn_math_1);
        self.define_var("exp", fn_math_1);
        self.define_var("ln", fn_math_1);
        self.define_var("log", fn_math_1);
        self.define_var("floor", fn_math_1);
        self.define_var("ceil", fn_math_1);
        self.define_var("pow", fn_math_2);
        self.define_var("isnan", fn_math_pred);
        self.define_var("isinf", fn_math_pred);
        self.define_var("abs", fn_any_1);
        self.define_var("min", fn_any_2);
        self.define_var("max", fn_any_2);

        // Core / Reflection / Type Builtins
        let fn_typeof = self.types.function_of(vec![TypeId::ANY], TypeId::STR);
        self.define_var("typeof", fn_typeof);
        self.define_var("type_of", fn_typeof);
        self.define_var("str", fn_typeof);
        self.define_var("name", fn_typeof);

        let fn_slice_buf = self.types.function_of(vec![TypeId::I64, TypeId::STR, TypeId::I64], TypeId::ANY);
        self.define_var("slice_buffer", fn_slice_buf);

        let fn_compile_regex = self.types.function_of(vec![TypeId::STR], TypeId::ANY);
        self.define_var("compile_regex", fn_compile_regex);

        // Option & Result Intrinsics
        let fn_pred_any = self.types.function_of(vec![TypeId::ANY], TypeId::BOOL);
        self.define_var("__opt_is_some", fn_pred_any);
        self.define_var("__opt_is_none", fn_pred_any);
        self.define_var("__opt_unwrap", fn_any_1);
        self.define_var("__result_is_ok", fn_pred_any);
        self.define_var("__result_is_err", fn_pred_any);
        self.define_var("__result_unwrap_ok", fn_any_1);
        self.define_var("__result_unwrap_err", fn_any_1);

        // String Intrinsics
        let fn_str_slice = self.types.function_of(vec![TypeId::STR, TypeId::I64, TypeId::I64], TypeId::STR);
        self.define_var("__str_slice", fn_str_slice);
        let fn_str_code = self.types.function_of(vec![TypeId::STR, TypeId::I64], TypeId::I64);
        self.define_var("__str_char_code", fn_str_code);
        let fn_char_code = self.types.function_of(vec![TypeId::I64], TypeId::STR);
        self.define_var("__char_from_code", fn_char_code);
        let fn_str_index = self.types.function_of(vec![TypeId::STR, TypeId::STR, TypeId::I64], TypeId::I64);
        self.define_var("__str_index_of", fn_str_index);
        self.define_var("__str_last_index_of", fn_str_index);
        let fn_str_replace = self.types.function_of(vec![TypeId::STR, TypeId::STR, TypeId::STR], TypeId::STR);
        self.define_var("__str_replace", fn_str_replace);
        let list_str = self.types.list_of(TypeId::STR);
        let fn_str_split = self.types.function_of(vec![TypeId::STR, TypeId::STR], list_str);
        let fn_str_1 = self.types.function_of(vec![TypeId::STR], TypeId::STR);
        let fn_str_case = self.types.function_of(vec![TypeId::STR, TypeId::BOOL], TypeId::STR);

        // OS & Filesystem Intrinsics
        let fn_void_str = self.types.function_of(vec![], TypeId::STR);
        let fn_void_bool = self.types.function_of(vec![], TypeId::BOOL);
        let fn_fs_pred = self.types.function_of(vec![TypeId::STR], TypeId::BOOL);
        let fn_fs_size = self.types.function_of(vec![TypeId::STR], TypeId::I64);
        let fn_fs_read = self.types.function_of(vec![TypeId::STR], TypeId::ANY);
        let fn_fs_write = self.types.function_of(vec![TypeId::STR, TypeId::STR], TypeId::ANY);
        let fn_fs_copy = self.types.function_of(vec![TypeId::STR, TypeId::STR], TypeId::ANY);
        let fn_list_dir = self.types.function_of(vec![TypeId::STR], list_str);

        // Memory & Slice Intrinsics
        let fn_slice_i64 = self.types.function_of(vec![TypeId::ANY], TypeId::I64);
        let fn_slice_get = self.types.function_of(vec![TypeId::ANY, TypeId::I64], TypeId::ANY);
        let fn_slice_byte_at = self.types.function_of(vec![TypeId::ANY, TypeId::I64], TypeId::I64);
        let fn_slice_set = self.types.function_of(vec![TypeId::ANY, TypeId::I64, TypeId::ANY], TypeId::VOID);
        let fn_slice_push = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], TypeId::VOID);
        let fn_slice_clear = self.types.function_of(vec![TypeId::ANY], TypeId::VOID);
        let fn_slice_subview = self.types.function_of(vec![TypeId::ANY, TypeId::I64, TypeId::I64], TypeId::ANY);
        let fn_slice_dot = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], TypeId::F64);
        let fn_slice_saxpy = self.types.function_of(vec![TypeId::F64, TypeId::ANY, TypeId::ANY], TypeId::ANY);
        let fn_slice_scale = self.types.function_of(vec![TypeId::F64, TypeId::ANY], TypeId::ANY);
        let fn_simd_bits = self.types.function_of(vec![], TypeId::I64);

        // DFA & Regex Intrinsics
        let fn_dfa_match = self.types.function_of(vec![TypeId::ANY, TypeId::STR], TypeId::BOOL);
        let fn_dfa_find = self.types.function_of(vec![TypeId::ANY, TypeId::STR, TypeId::I64], TypeId::ANY);
        let fn_dfa_find_all = self.types.function_of(vec![TypeId::ANY, TypeId::STR], list_str);
        let fn_dfa_find_spans = self.types.function_of(vec![TypeId::ANY, TypeId::STR], TypeId::ANY);
        let fn_dfa_split = self.types.function_of(vec![TypeId::ANY, TypeId::STR], list_str);
        let fn_dfa_replace_all = self.types.function_of(vec![TypeId::ANY, TypeId::STR, TypeId::STR], TypeId::STR);

        // UTF-8 & Parsing Intrinsics
        let fn_utf8_encode = self.types.function_of(vec![TypeId::STR], TypeId::ANY);
        let fn_utf8_decode = self.types.function_of(vec![TypeId::ANY], TypeId::STR);
        let fn_utf8_boundary = self.types.function_of(vec![TypeId::ANY, TypeId::I64], TypeId::BOOL);
        let fn_parse_int = self.types.function_of(vec![TypeId::STR, TypeId::I64], TypeId::ANY);
        let fn_parse_float = self.types.function_of(vec![TypeId::STR], TypeId::ANY);

        // Metaprogramming & Reflection Intrinsics
        let fn_ast_subst = self.types.function_of(vec![TypeId::ANY, TypeId::STR, TypeId::ANY], TypeId::ANY);
        let fn_ast_diff = self.types.function_of(vec![TypeId::ANY, TypeId::STR], TypeId::ANY);
        let fn_ast_eval = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], TypeId::ANY);
        let fn_kw_dist = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], TypeId::I64);
        let fn_isa = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], TypeId::BOOL);

        self.define_var("__str_split", fn_str_split);
        self.define_var("__str_trim", fn_str_1);
        self.define_var("__str_case", fn_str_case);

        self.define_var("__os_cwd", fn_void_str);
        self.define_var("__os_temp_dir", fn_void_str);
        self.define_var("__os_is_windows", fn_void_bool);
        self.define_var("__fs_exists", fn_fs_pred);
        self.define_var("__fs_is_file", fn_fs_pred);
        self.define_var("__fs_is_dir", fn_fs_pred);
        self.define_var("__fs_file_size", fn_fs_size);
        self.define_var("__fs_read_text", fn_fs_read);
        self.define_var("__fs_read_bytes", fn_fs_read);
        self.define_var("__fs_list_dir", fn_list_dir);
        self.define_var("__fs_walk_dir", fn_list_dir);
        self.define_var("__fs_mmap_read", fn_fs_read);
        self.define_var("__fs_mmap_rw", fn_fs_read);
        self.define_var("__fs_write_text", fn_fs_write);
        self.define_var("__fs_copy_file", fn_fs_copy);
        self.define_var("__fs_move_file", fn_fs_copy);
        self.define_var("__fs_remove_file", fn_fs_read);
        self.define_var("__fs_mkdir", fn_fs_read);
        self.define_var("__fs_rmdir", fn_fs_read);

        self.define_var("__slice_len", fn_slice_i64);
        self.define_var("__slice_cap", fn_slice_i64);
        self.define_var("__slice_stride", fn_slice_i64);
        self.define_var("__slice_ptr", fn_slice_i64);
        self.define_var("__slice_is_readonly", fn_pred_any);
        self.define_var("__slice_get", fn_slice_get);
        self.define_var("__slice_byte_at", fn_slice_byte_at);
        self.define_var("__slice_set", fn_slice_set);
        self.define_var("__slice_push", fn_slice_push);
        self.define_var("__slice_clear", fn_slice_clear);
        self.define_var("__slice_subview", fn_slice_subview);
        self.define_var("__slice_to_list", fn_any_1);
        self.define_var("__slice_sum", fn_any_1);
        self.define_var("__slice_dot", fn_slice_dot);
        self.define_var("__slice_saxpy", fn_slice_saxpy);
        self.define_var("__slice_scale", fn_slice_scale);
        self.define_var("__slice_sort", fn_any_1);
        self.define_var("__slice_welford", fn_any_1);
        self.define_var("__cpp_backend_available", fn_void_bool);
        self.define_var("__cpp_simd_bits", fn_simd_bits);
        self.define_var("__freeze_value", fn_any_1);
        self.define_var("__is_frozen", fn_pred_any);
        self.define_var("__is_perfect_hash_map", fn_pred_any);
        self.define_var("__frozen_map_table_size", fn_slice_i64);

        self.define_var("__dfa_is_match", fn_dfa_match);
        self.define_var("__dfa_find", fn_dfa_find);
        self.define_var("__dfa_find_all", fn_dfa_find_all);
        self.define_var("__dfa_find_spans", fn_dfa_find_spans);
        self.define_var("__dfa_split", fn_dfa_split);
        self.define_var("__dfa_replace_all", fn_dfa_replace_all);

        self.define_var("__utf8_encode_slice", fn_utf8_encode);
        self.define_var("__utf8_decode_slice", fn_utf8_decode);
        self.define_var("__utf8_validate_slice", fn_pred_any);
        self.define_var("__utf8_rune_count", fn_slice_i64);
        self.define_var("__utf8_is_valid_boundary", fn_utf8_boundary);
        self.define_var("__parse_int_strict", fn_parse_int);
        self.define_var("__parse_float_strict", fn_parse_float);
        self.define_var("__map_entries_sorted", fn_any_1);

        self.define_var("__ast_fold", fn_any_1);
        self.define_var("__ast_subst", fn_ast_subst);
        self.define_var("__ast_diff", fn_ast_diff);
        self.define_var("__ast_eval", fn_ast_eval);
        self.define_var("__kw_parents", fn_any_1);
        self.define_var("__kw_ancestors", fn_any_1);
        self.define_var("__kw_descendants", fn_any_1);
        self.define_var("__kw_distance", fn_kw_dist);
        self.define_var("derive", fn_any_1);
        self.define_var("underive", fn_any_1);
        self.define_var("isa", fn_isa);
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn define_var(&mut self, name: &str, ty: TypeId) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(name.to_string(), ty);
        }
    }

    pub fn narrow_var(&mut self, name: &str, narrowed_ty: TypeId) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.narrowed_types.insert(name.to_string(), narrowed_ty);
        }
    }

    pub fn lookup_var(&self, name: &str) -> Option<TypeId> {
        for scope in self.scopes.iter().rev() {
            if let Some(&narrowed) = scope.narrowed_types.get(name) {
                return Some(narrowed);
            }
            if let Some(&ty) = scope.bindings.get(name) {
                return Some(ty);
            }
        }
        if name == "self" || name == "this" {
            return Some(TypeId::ANY);
        }
        self.type_defs.get(name).copied()
    }

    pub fn resolve_type_spec(&mut self, arena: &AstArena, node_id: NodeId) -> TypeId {
        if node_id.is_none() {
            return TypeId::ANY;
        }

        let node = *arena.node(node_id);
        let ty = match node.node_kind() {
            NodeKind::TypeNamed => {
                let name = arena.interner.resolve(SymbolId(node.lhs));
                if let Some(&ty) = self.type_defs.get(name) {
                    ty
                } else {
                    let ty = self.types.alloc(TypeKind::Primitive(name.to_string()));
                    self.type_defs.insert(name.to_string(), ty);
                    ty
                }
            }
            NodeKind::TypeOptional => {
                let inner = self.resolve_type_spec(arena, NodeId(node.lhs));
                self.types.option_of(inner)
            }
            NodeKind::TypeFallible => {
                let ok = self.resolve_type_spec(arena, NodeId(node.lhs));
                let err = self.resolve_type_spec(arena, NodeId(node.rhs));
                self.types.result_of(ok, err)
            }
            NodeKind::TypeList | NodeKind::TypeSlice => {
                let elem = if node.rhs > 0 {
                    let children = arena.children_slice(node.lhs, node.rhs);
                    children
                        .first()
                        .map(|&c| self.resolve_type_spec(arena, NodeId(c)))
                        .unwrap_or(TypeId::ANY)
                } else {
                    self.resolve_type_spec(arena, NodeId(node.lhs))
                };
                self.types.list_of(elem)
            }
            NodeKind::TypeMap => {
                let key = self.resolve_type_spec(arena, NodeId(node.lhs));
                let val = self.resolve_type_spec(arena, NodeId(node.rhs));
                self.types.map_of(key, val)
            }
            NodeKind::TypeTuple => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let elems = children
                    .iter()
                    .map(|&c| self.resolve_type_spec(arena, NodeId(c)))
                    .collect();
                self.types.tuple_of(elems)
            }
            NodeKind::TypeUnion => {
                let lhs = self.resolve_type_spec(arena, NodeId(node.lhs));
                let rhs = self.resolve_type_spec(arena, NodeId(node.rhs));
                self.types.alloc(TypeKind::Union(vec![lhs, rhs]))
            }
            NodeKind::TypeIntersection => {
                let lhs = self.resolve_type_spec(arena, NodeId(node.lhs));
                let rhs = self.resolve_type_spec(arena, NodeId(node.rhs));
                self.types.alloc(TypeKind::Union(vec![lhs, rhs]))
            }
            NodeKind::TypeFn => {
                let lhs = self.resolve_type_spec(arena, NodeId(node.lhs));
                let ret = self.resolve_type_spec(arena, NodeId(node.rhs));
                self.types.function_of(vec![lhs], ret)
            }
            _ => TypeId::ANY,
        };
        self.inferred_node_types.insert(node_id.0, ty);
        ty
    }

    /// Extracts field & method signatures from a struct/class/record declaration.
    fn collect_decl_fields(&mut self, arena: &AstArena, node_id: NodeId) -> Vec<(String, TypeId)> {
        let mut fields = Vec::new();
        let node = *arena.node(node_id);
        if let Some(rec_fields) = arena.record_fields.get(&node_id.0) {
            for &(f_sym, f_ty_node) in rec_fields {
                let f_name = arena.interner.resolve(f_sym).to_string();
                let f_ty = if f_ty_node.is_some() {
                    self.resolve_type_spec(arena, f_ty_node)
                } else {
                    TypeId::ANY
                };
                fields.push((f_name, f_ty));
            }
        }
        let body = NodeId(node.rhs);
        if body.is_some() {
            let b_node = *arena.node(body);
            if b_node.node_kind() == NodeKind::BlockExpr {
                let b_children = arena.children_slice(b_node.lhs, b_node.rhs);
                for &stmt_id in b_children {
                    let s_node = *arena.node(NodeId(stmt_id));
                    match s_node.node_kind() {
                        NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl if s_node.flags == 0 => {
                            let f_name = arena.interner.resolve(SymbolId(s_node.lhs)).to_string();
                            let f_ty = if let Some(&ann_id) = arena.type_annotations.get(&stmt_id) {
                                self.resolve_type_spec(arena, ann_id)
                            } else if NodeId(s_node.rhs).is_some() {
                                self.synth(arena, NodeId(s_node.rhs))
                            } else {
                                TypeId::ANY
                            };
                            self.inferred_node_types.insert(stmt_id, f_ty);
                            fields.push((f_name, f_ty));
                        }
                        NodeKind::FnDecl => {
                            let m_name = arena.interner.resolve(SymbolId(s_node.lhs)).to_string();
                            let fn_start = s_node.rhs;
                            let fn_len = s_node.flags as usize;
                            let mut param_tys = Vec::new();
                            if fn_len > 0 {
                                let fn_children = arena.children_slice(fn_start, fn_len as u32);
                                for &p_id in &fn_children[1..] {
                                    let p_node = *arena.node(NodeId(p_id));
                                    let p_name = arena.interner.resolve(SymbolId(p_node.lhs));
                                    if p_name == "self" || p_name == "this" {
                                        continue;
                                    }
                                    let p_ty = if p_node.rhs != 0 {
                                        self.resolve_type_spec(arena, NodeId(p_node.rhs))
                                    } else {
                                        TypeId::ANY
                                    };
                                    param_tys.push(p_ty);
                                }
                            }
                            let ret_ty = arena
                                .type_annotations
                                .get(&stmt_id)
                                .map(|&ann| self.resolve_type_spec(arena, ann))
                                .unwrap_or(TypeId::ANY);
                            let m_ty = self.types.function_of(param_tys, ret_ty);
                            fields.push((m_name, m_ty));
                        }
                        _ => {}
                    }
                }
            }
        }
        fields
    }

    /// Bidirectional: synthesizes the type of an AST expression or declaration node.
    pub fn synth(&mut self, arena: &AstArena, node_id: NodeId) -> TypeId {
        if node_id.is_none() {
            return TypeId::VOID;
        }

        let node = *arena.node(node_id);
        let ty = match node.node_kind() {
            // --- Literals ---
            NodeKind::IntLit => TypeId::I64,
            NodeKind::FloatLit => TypeId::F64,
            NodeKind::StrLit => TypeId::STR,
            NodeKind::BoolLit => TypeId::BOOL,
            NodeKind::NoneLit => TypeId::NULL,
            NodeKind::KeywordLit => self.types.alloc(TypeKind::Primitive("keyword".to_string())),
            NodeKind::UnitLit => {
                let unit_kind = match node.flags {
                    1 => "Duration",
                    2 => "Frequency",
                    3 => "Memory",
                    _ => "dim",
                };
                self.types.alloc(TypeKind::Unit {
                    base: TypeId::I64,
                    unit_name: unit_kind.to_string(),
                })
            }
            NodeKind::PolyglotBlock => {
                if (node.flags & 1) != 0 {
                    self.types.function_of(vec![TypeId::ANY], TypeId::ANY)
                } else {
                    TypeId::ANY
                }
            }

            // --- Identifier & Variable Lookup ---
            NodeKind::IdentExpr => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym);
                if name == "_" {
                    TypeId::ANY
                } else if let Some(t) = self.lookup_var(name) {
                    t
                } else if let Some(&t) = self.type_defs.get(name) {
                    t
                } else if self.adt_variants.contains_key(name) {
                    let (adt_name, _) = self.adt_variants[name].clone();
                    self.type_defs.get(&adt_name).copied().unwrap_or(TypeId::ANY)
                } else {
                    self.diagnostics.push(AnalysisDiagnostic::error(
                        "TYPE001",
                        format!("Unresolved identifier '{}'", name),
                        arena.span(node_id),
                    ));
                    TypeId::ANY
                }
            }

            // --- Arithmetic & Logical Binary Expressions ---
            NodeKind::BinaryExpr => {
                let op = node.flags;
                if op == 26 {
                    // BinaryOp::Walrus (pat := val or let/some pat = val)
                    let raw_val_ty = self.synth(arena, NodeId(node.rhs));
                    let pruned_val = self.types.prune(raw_val_ty);
                    let lhs = NodeId(node.lhs);
                    let lhs_node = *arena.node(lhs);
                    let val_ty = if matches!(lhs_node.node_kind(), NodeKind::IdentExpr | NodeKind::PatBind) {
                        match self.types.get(pruned_val).clone() {
                            TypeKind::Option(inner) => inner,
                            TypeKind::Result { ok, .. } => ok,
                            _ => pruned_val,
                        }
                    } else {
                        pruned_val
                    };
                    self.bind_pattern_vars_typed(arena, lhs, val_ty);
                    if lhs_node.node_kind() == NodeKind::IdentExpr || lhs_node.node_kind() == NodeKind::PatBind {
                        let name = arena.interner.resolve(SymbolId(lhs_node.lhs));
                        self.define_var(name, val_ty);
                        self.inferred_node_types.insert(lhs.0, val_ty);
                    }
                    self.inferred_node_types.insert(node_id.0, val_ty);
                    return val_ty;
                }
                if op == 25 {
                    // BinaryOp::Is (e.g. s is Circle(r))
                    let lhs_ty = self.synth(arena, NodeId(node.lhs));
                    let rhs = NodeId(node.rhs);
                    let rhs_node = *arena.node(rhs);
                    if rhs_node.node_kind() == NodeKind::CallExpr {
                        let arg_start = rhs_node.lhs as usize;
                        let arg_len = rhs_node.rhs as usize;
                        let mut variant_param_tys = Vec::new();
                        if arg_len > 0 && arg_start < arena.extra_children.len() {
                            let callee_id = NodeId(arena.extra_children[arg_start]);
                            let callee_node = *arena.node(callee_id);
                            if callee_node.node_kind() == NodeKind::IdentExpr {
                                let v_name = arena.interner.resolve(SymbolId(callee_node.lhs));
                                if let Some((adt_name, p_tys)) = self.adt_variants.get(v_name).cloned() {
                                    variant_param_tys = p_tys;
                                    if let Some(&adt_ty) = self.type_defs.get(&adt_name) {
                                        self.inferred_node_types.insert(callee_id.0, adt_ty);
                                    }
                                }
                            }
                        }
                        if arg_len > 1 && arg_start + arg_len <= arena.extra_children.len() {
                            for (idx, &arg_id) in arena.extra_children[arg_start + 1..arg_start + arg_len].iter().enumerate() {
                                let a_node = *arena.node(NodeId(arg_id));
                                let p_ty = variant_param_tys.get(idx).copied().unwrap_or(TypeId::ANY);
                                if a_node.node_kind() == NodeKind::IdentExpr {
                                    let a_name = arena.interner.resolve(SymbolId(a_node.lhs));
                                    self.define_var(a_name, p_ty);
                                    self.inferred_node_types.insert(arg_id, p_ty);
                                }
                            }
                        }
                    } else {
                        self.bind_pattern_vars_typed(arena, rhs, lhs_ty);
                    }
                    self.inferred_node_types.insert(node_id.0, TypeId::BOOL);
                    return TypeId::BOOL;
                }
                let lhs_ty = self.synth(arena, NodeId(node.lhs));
                if op == 22 {
                    // BinaryOp::Coalesce (`??` or `catch`)
                    let pruned_lhs = self.types.prune(lhs_ty);
                    let rhs_node_id = NodeId(node.rhs);
                    let rhs_node = *arena.node(rhs_node_id);
                    if let TypeKind::Result { ok, err } = self.types.get(pruned_lhs).clone() {
                        if rhs_node.node_kind() == NodeKind::LambdaExpr && rhs_node.flags == 1 {
                            let params = arena.children_slice(rhs_node.lhs, 1);
                            if !params.is_empty() {
                                let p_id = params[0];
                                let p_name = arena.interner.resolve(SymbolId(arena.node(NodeId(p_id)).lhs)).to_string();
                                self.enter_scope();
                                self.inferred_node_types.insert(p_id, err);
                                self.define_var(&p_name, err);
                                let _ = self.synth(arena, NodeId(rhs_node.rhs));
                                self.exit_scope();
                                return ok;
                            }
                        }
                        let _ = self.synth(arena, rhs_node_id);
                        return ok;
                    }
                    let rhs_ty = self.synth(arena, rhs_node_id);
                    return match self.types.get(pruned_lhs).clone() {
                        TypeKind::Option(inner) => {
                            let _ = self.types.unify(inner, rhs_ty);
                            inner
                        }
                        _ if pruned_lhs == TypeId::NULL => rhs_ty,
                        _ => pruned_lhs,
                    };
                }
                let rhs_ty = self.synth(arena, NodeId(node.rhs));
                // Comparison operators (Eq=8..Ge=13) and logical operators (And=14, Or=15)
                if (8..=15).contains(&op) {
                    let _ = self.types.unify(lhs_ty, rhs_ty);
                    TypeId::BOOL
                } else if op == 1 && lhs_ty == TypeId::STR {
                    // String concatenation (str + any -> str)
                    TypeId::STR
                } else if (lhs_ty.is_integer() && rhs_ty.is_float()) || (lhs_ty.is_float() && rhs_ty.is_integer()) {
                    // Automatic numeric widening: int <op> float -> f64
                    TypeId::F64
                } else if op == 17 || op == 16 || op == 2 {
                    // Algebraic class union (|), intersection (&), and subtraction (-)
                    lhs_ty
                } else if op == 3 && matches!(self.types.get(self.types.prune(lhs_ty)), TypeKind::Function { .. }) {
                    // Multi-cast broadcast invocation: `fn*(arg)`
                    if let TypeKind::Function { ret, .. } = self.types.get(self.types.prune(lhs_ty)).clone() {
                        self.types.list_of(ret)
                    } else {
                        TypeId::ANY
                    }
                } else if op == 21 {
                    // BinaryOp::Pipe (|>)
                    if let TypeKind::Function { ret, .. } = self.types.get(rhs_ty).clone() {
                        ret
                    } else {
                        TypeId::ANY
                    }
                } else if op == 22 {
                    // BinaryOp::Coalesce (??)
                    if lhs_ty == TypeId::NULL {
                        rhs_ty
                    } else {
                        lhs_ty
                    }
                } else {
                    match self.types.unify(lhs_ty, rhs_ty) {
                        Ok(unified) => unified,
                        Err(e) => {
                            self.diagnostics.push(AnalysisDiagnostic::error(
                                "TYPE002",
                                format!("Type mismatch in binary expression: {}", e),
                                arena.span(node_id),
                            ));
                            TypeId::ANY
                        }
                    }
                }
            }

            NodeKind::UnaryExpr => {
                let op_flag = node.flags;
                let operand_ty = self.synth(arena, NodeId(node.lhs));
                if op_flag == 2 {
                    // Spread operator (..collection) in C# collection expressions
                    match self.types.get(self.types.prune(operand_ty)).clone() {
                        TypeKind::Collection { element, .. } => element,
                        _ => operand_ty,
                    }
                } else {
                    operand_ty
                }
            }

            // --- Compound Collections ---
            NodeKind::ListExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                if children.is_empty() {
                    let var = self.types.new_var(0);
                    self.types.list_of(var)
                } else {
                    let mut elem_ty = self.synth(arena, NodeId(children[0]));
                    for &c in &children[1..] {
                        let c_ty = self.synth(arena, NodeId(c));
                        match self.types.unify(elem_ty, c_ty) {
                            Ok(unified) => elem_ty = unified,
                            Err(_) => {
                                elem_ty = TypeId::ANY;
                            }
                        }
                    }
                    self.types.list_of(elem_ty)
                }
            }

            NodeKind::TupleExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut elem_types = Vec::with_capacity(children.len());
                for &c in children {
                    elem_types.push(self.synth(arena, NodeId(c)));
                }
                self.types.tuple_of(elem_types)
            }

            NodeKind::MapExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut k_ty = TypeId::ANY;
                let mut v_ty: Option<TypeId> = None;
                for chunk in children.chunks(2) {
                    if chunk.len() == 2 {
                        k_ty = self.synth(arena, NodeId(chunk[0]));
                        let cur_v = self.synth(arena, NodeId(chunk[1]));
                        v_ty = Some(match v_ty {
                            None => cur_v,
                            Some(prev) => self.types.unify(prev, cur_v).unwrap_or(TypeId::ANY),
                        });
                    }
                }
                self.types.map_of(k_ty, v_ty.unwrap_or(TypeId::ANY))
            }

            NodeKind::RangeExpr => self.types.list_of(TypeId::I64),

            // --- Blocks & Program Root ---
            NodeKind::Program => {
                let children = arena.children_slice(node.lhs, node.rhs);
                // Pass 1: Register top-level function, class, struct, record, and ADT signatures
                for &c in children {
                    let child_node = *arena.node(NodeId(c));
                    match child_node.node_kind() {
                        NodeKind::FnDecl => {
                            let fn_name = arena.interner.resolve(SymbolId(child_node.lhs)).to_string();
                            let fn_start = child_node.rhs;
                            let fn_len = child_node.flags as usize;
                            let mut param_tys = Vec::new();
                            if fn_len > 0 {
                                let fn_children = arena.children_slice(fn_start, fn_len as u32);
                                for &p_id in &fn_children[1..] {
                                    let p_node = *arena.node(NodeId(p_id));
                                    let p_ty = if p_node.rhs != 0 {
                                        self.resolve_type_spec(arena, NodeId(p_node.rhs))
                                    } else {
                                        TypeId::ANY
                                    };
                                    self.inferred_node_types.insert(p_id, p_ty);
                                    param_tys.push(p_ty);
                                }
                            }
                            let ret_ty = arena
                                .type_annotations
                                .get(&c)
                                .map(|&ann| self.resolve_type_spec(arena, ann))
                                .unwrap_or(TypeId::ANY);
                            let fn_ty = self.types.function_of(param_tys, ret_ty);
                            self.define_var(&fn_name, fn_ty);
                        }
                        NodeKind::RecordDecl | NodeKind::StructDecl => {
                            let rec_name = arena.interner.resolve(SymbolId(child_node.lhs)).to_string();
                            let fields = self.collect_decl_fields(arena, NodeId(c));
                            self.struct_fields.insert(rec_name.clone(), fields.clone());
                            let ty = self.types.row_record_of(fields.clone(), true, None);
                            self.type_defs.insert(rec_name.clone(), ty);
                            let param_tys = if fields.is_empty() {
                                vec![TypeId::ANY]
                            } else {
                                fields.iter().map(|(_, t)| *t).collect()
                            };
                            let ctor_ty = self.types.function_of(param_tys, ty);
                            self.define_var(&rec_name, ctor_ty);
                        }
                        NodeKind::TypeAdtDecl => {
                            let adt_name = arena.interner.resolve(SymbolId(child_node.lhs)).to_string();
                            let var_start = child_node.rhs as usize;
                            let var_len = child_node.flags as usize;
                            let mut adt_variants_list = Vec::new();
                            let adt_placeholder = self.types.adt_of(adt_name.clone(), Vec::new());
                            self.type_defs.insert(adt_name.clone(), adt_placeholder);

                            if var_len > 0 && var_start + var_len <= arena.extra_children.len() {
                                for &v_id in &arena.extra_children[var_start..var_start + var_len] {
                                    let v_node = *arena.node(NodeId(v_id));
                                    if v_node.node_kind() == NodeKind::AdtVariant {
                                        let v_name = arena.interner.resolve(SymbolId(v_node.lhs)).to_string();
                                        let param_start = v_node.rhs;
                                        let param_len = v_node.flags as usize;
                                        let mut p_tys = Vec::new();
                                        if param_len > 0 {
                                            let p_slice = arena.children_slice(param_start, param_len as u32);
                                            for &p_id in p_slice {
                                                let p_node = *arena.node(NodeId(p_id));
                                                let p_ty = if p_node.rhs != 0 {
                                                    self.resolve_type_spec(arena, NodeId(p_node.rhs))
                                                } else {
                                                    TypeId::ANY
                                                };
                                                self.inferred_node_types.insert(p_id, p_ty);
                                                p_tys.push(p_ty);
                                            }
                                        }
                                        adt_variants_list.push((v_name.clone(), p_tys.clone()));
                                        self.adt_variants.insert(v_name.clone(), (adt_name.clone(), p_tys.clone()));
                                        if param_len == 0 {
                                            self.define_var(&v_name, adt_placeholder);
                                            self.inferred_node_types.insert(v_id, adt_placeholder);
                                        } else {
                                            let ctor_ty = self.types.function_of(p_tys, adt_placeholder);
                                            self.define_var(&v_name, ctor_ty);
                                            self.inferred_node_types.insert(v_id, ctor_ty);
                                        }
                                    }
                                }
                            }
                            let adt_ty = self.types.adt_of(adt_name.clone(), adt_variants_list);
                            self.type_defs.insert(adt_name.clone(), adt_ty);
                        }
                        NodeKind::ClassDecl | NodeKind::InterfaceDecl => {
                            let cls_name = arena.interner.resolve(SymbolId(child_node.lhs)).to_string();
                            let ty = self.types.alloc(TypeKind::Primitive(cls_name.clone()));
                            self.type_defs.insert(cls_name.clone(), ty);
                            let fields = self.collect_decl_fields(arena, NodeId(c));
                            self.struct_fields.insert(cls_name.clone(), fields);
                            let ctor_ty = self.types.function_of(vec![TypeId::ANY], ty);
                            self.define_var(&cls_name, ctor_ty);
                        }
                        NodeKind::ImportDecl => {
                            if child_node.rhs != u32::MAX {
                                let alias = arena.interner.resolve(SymbolId(child_node.rhs)).to_string();
                                self.define_var(&alias, TypeId::ANY);
                                self.type_defs.insert(alias, TypeId::ANY);
                            }
                        }
                        NodeKind::FromImportDecl => {
                            let sub_children = arena.children_slice(child_node.lhs, child_node.rhs);
                            for &sym_id in sub_children {
                                let sym_node = arena.node(NodeId(sym_id));
                                if sym_node.node_kind() == NodeKind::IdentExpr {
                                    let name = arena.interner.resolve(SymbolId(sym_node.lhs)).to_string();
                                    self.define_var(&name, TypeId::ANY);
                                    self.type_defs.insert(name, TypeId::ANY);
                                }
                            }
                        }
                        NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
                            if child_node.flags == 1 {
                                self.bind_pattern_vars(arena, NodeId(child_node.lhs));
                            } else {
                                let name = arena.interner.resolve(SymbolId(child_node.lhs)).to_string();
                                let ann_ty = arena
                                    .type_annotations
                                    .get(&c)
                                    .map(|&ann| self.resolve_type_spec(arena, ann))
                                    .unwrap_or(TypeId::ANY);
                                self.define_var(&name, ann_ty);
                            }
                        }
                        _ => {}
                    }
                }

                // Pass 2: Synthesize all statements and expressions
                let mut last_ty = TypeId::VOID;
                for &c in children {
                    last_ty = self.synth(arena, NodeId(c));
                }
                last_ty
            }

            NodeKind::BlockExpr => {
                self.enter_scope();
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut last_ty = TypeId::VOID;
                for &c in children {
                    let child_node = *arena.node(NodeId(c));
                    let ty = self.synth(arena, NodeId(c));
                    if child_node.node_kind() == NodeKind::ReturnStmt {
                        last_ty = ty;
                    } else if last_ty == TypeId::VOID {
                        last_ty = ty;
                    }
                }
                self.exit_scope();
                last_ty
            }

            NodeKind::ExprStmt => {
                let expr_node = NodeId(node.lhs);
                if expr_node.is_some() {
                    self.synth(arena, expr_node)
                } else {
                    TypeId::VOID
                }
            }

            NodeKind::TryStmt => {
                let children = arena.children_slice(node.lhs, node.rhs);
                for &child_id in children {
                    self.synth(arena, NodeId(child_id));
                }
                TypeId::VOID
            }

            NodeKind::CatchClause => {
                let var_sym = SymbolId(node.lhs);
                let body_node = NodeId(node.rhs);
                self.enter_scope();
                if var_sym.0 != 0 {
                    let var_name = arena.interner.resolve(var_sym);
                    let err_ty = self.type_defs.get("Error").copied().unwrap_or(TypeId::ANY);
                    self.define_var(var_name, err_ty);
                }
                if body_node.is_some() {
                    self.synth(arena, body_node);
                }
                self.exit_scope();
                TypeId::VOID
            }

            // --- Variable Declarations & Assignments ---
            NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
                let ann_ty = arena
                    .type_annotations
                    .get(&node_id.0)
                    .map(|&ann| self.resolve_type_spec(arena, ann));
                let init_node = NodeId(node.rhs);
                let init_ty = if init_node.is_some() {
                    self.synth(arena, init_node)
                } else {
                    TypeId::ANY
                };
                let val_ty = if let Some(explicit_ty) = ann_ty {
                    if init_node.is_some() && init_ty != TypeId::ANY {
                        let _ = self.types.unify(explicit_ty, init_ty);
                    }
                    if explicit_ty != TypeId::ANY {
                        explicit_ty
                    } else {
                        init_ty
                    }
                } else {
                    init_ty
                };
                if node.flags == 1 {
                    self.bind_pattern_vars_typed(arena, NodeId(node.lhs), val_ty);
                } else {
                    let sym = SymbolId(node.lhs);
                    let name = arena.interner.resolve(sym).to_string();
                    self.define_var(&name, val_ty);
                }
                val_ty
            }

            NodeKind::AssignStmt => {
                let target = NodeId(node.lhs);
                let val = NodeId(node.rhs);
                let val_ty = self.synth(arena, val);
                if target.is_some() {
                    let t_kind = arena.node(target).node_kind();
                    if matches!(t_kind, NodeKind::MapExpr | NodeKind::ListExpr | NodeKind::TupleExpr) {
                        self.bind_destructuring_expr(arena, target, val_ty);
                    } else {
                        let target_ty = self.synth(arena, target);
                        let _ = self.types.unify(target_ty, val_ty);
                    }
                }
                TypeId::VOID
            }

            NodeKind::ReturnStmt => {
                let expr_node = NodeId(node.lhs);
                if expr_node.is_some() {
                    self.synth(arena, expr_node)
                } else {
                    TypeId::VOID
                }
            }

            NodeKind::YieldStmt => {
                let expr_node = NodeId(node.lhs);
                if expr_node.is_some() {
                    self.synth(arena, expr_node)
                } else {
                    TypeId::VOID
                }
            }

            // --- Function, Class, Struct, and ADT Declarations ---
            NodeKind::FnDecl => {
                let sym = SymbolId(node.lhs);
                let fn_name = arena.interner.resolve(sym).to_string();
                let fn_start = node.rhs;
                let fn_len = node.flags as usize;

                let explicit_ret_ty = arena
                    .type_annotations
                    .get(&node_id.0)
                    .map(|&ann| self.resolve_type_spec(arena, ann));

                let mut param_tys = Vec::new();
                if fn_len > 0 {
                    let fn_children = arena.children_slice(fn_start, fn_len as u32);
                    for &p_id in &fn_children[1..] {
                        let p_node = *arena.node(NodeId(p_id));
                        let p_ty = if p_node.rhs != 0 {
                            self.resolve_type_spec(arena, NodeId(p_node.rhs))
                        } else {
                            TypeId::ANY
                        };
                        self.inferred_node_types.insert(p_id, p_ty);
                        param_tys.push(p_ty);
                    }
                }
                let fn_placeholder = self
                    .types
                    .function_of(param_tys.clone(), explicit_ret_ty.unwrap_or(TypeId::ANY));
                self.define_var(&fn_name, fn_placeholder);

                self.enter_scope();
                let body_ty = if fn_len > 0 {
                    let fn_children = arena.children_slice(fn_start, fn_len as u32);
                    let body_node = NodeId(fn_children[0]);
                    for (idx, &p_id) in fn_children[1..].iter().enumerate() {
                        let p_node = *arena.node(NodeId(p_id));
                        let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                        let p_ty = param_tys.get(idx).copied().unwrap_or(TypeId::ANY);
                        self.define_var(&p_name, p_ty);
                    }
                    self.synth(arena, body_node)
                } else {
                    let body_node = NodeId(node.rhs);
                    self.synth(arena, body_node)
                };
                self.exit_scope();

                let ret_ty = match explicit_ret_ty {
                    Some(exp) if exp != TypeId::ANY => {
                        if exp != TypeId::VOID && body_ty != TypeId::VOID && body_ty != TypeId::ANY {
                            let pruned_exp = self.types.prune(exp);
                            if let TypeKind::Result { ok, .. } = *self.types.get(pruned_exp) {
                                let _ = self.types.unify(ok, body_ty);
                            } else {
                                let _ = self.types.unify(exp, body_ty);
                            }
                        }
                        exp
                    }
                    _ => body_ty,
                };
                let fn_ty = self.types.function_of(param_tys, ret_ty);
                self.define_var(&fn_name, fn_ty);
                fn_ty
            }

            NodeKind::RecordDecl | NodeKind::StructDecl => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym).to_string();
                let fields = self.collect_decl_fields(arena, node_id);
                self.struct_fields.insert(name.clone(), fields.clone());
                let ty = self.types.row_record_of(fields.clone(), true, None);
                self.type_defs.insert(name.clone(), ty);
                let param_tys = if fields.is_empty() {
                    vec![TypeId::ANY]
                } else {
                    fields.iter().map(|(_, t)| *t).collect()
                };
                let ctor = self.types.function_of(param_tys, ty);
                self.define_var(&name, ctor);
                let body = NodeId(node.rhs);
                if body.is_some() {
                    self.enter_scope();
                    self.define_var("self", ty);
                    self.define_var("this", ty);
                    self.synth(arena, body);
                    self.exit_scope();
                }
                ty
            }

            NodeKind::TypeAdtDecl => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym).to_string();
                let ty = self
                    .type_defs
                    .get(&name)
                    .copied()
                    .unwrap_or_else(|| self.types.adt_of(name.clone(), Vec::new()));
                self.type_defs.insert(name, ty);
                ty
            }

            NodeKind::ClassDecl | NodeKind::InterfaceDecl => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym).to_string();
                let ty = self.types.alloc(TypeKind::Primitive(name.clone()));
                self.type_defs.insert(name.clone(), ty);
                let fields = self.collect_decl_fields(arena, node_id);
                self.struct_fields.insert(name.clone(), fields);
                let ctor = self.types.function_of(vec![TypeId::ANY], ty);
                self.define_var(&name, ctor);
                let body = NodeId(node.rhs);
                if body.is_some() {
                    self.enter_scope();
                    self.define_var("self", ty);
                    self.define_var("this", ty);
                    self.synth(arena, body);
                    self.exit_scope();
                }
                ty
            }

            NodeKind::ExtendDecl => {
                let target_name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                let target_ty = self.type_defs.get(&target_name).copied().unwrap_or(TypeId::ANY);
                let ext_fields = self.collect_decl_fields(arena, node_id);
                if !ext_fields.is_empty() {
                    for f in &ext_fields {
                        let ext_fn_name = format!("{}Ext_{}", target_name, f.0);
                        self.define_var(&ext_fn_name, f.1);
                    }
                    let entry = self.struct_fields.entry(target_name).or_default();
                    for f in ext_fields {
                        entry.push(f);
                    }
                }
                let body = NodeId(node.rhs);
                if body.is_some() {
                    self.enter_scope();
                    self.define_var("self", target_ty);
                    self.define_var("this", target_ty);
                    self.synth(arena, body);
                    self.exit_scope();
                }
                TypeId::VOID
            }

            // --- Control Flow ---
            NodeKind::IfExpr => {
                let cond_node = NodeId(node.lhs);
                self.enter_scope();
                if cond_node.is_some() {
                    let cond_kind = arena.node(cond_node).node_kind();
                    let cond_flags = arena.node(cond_node).flags;
                    if cond_kind == NodeKind::BinaryExpr && cond_flags == 26 {
                        self.synth(arena, cond_node);
                    } else if cond_kind == NodeKind::PatBind {
                        let var_name = arena.interner.resolve(SymbolId(arena.node(cond_node).lhs)).to_string();
                        self.narrow_on_guard_some(&var_name);
                        if let Some(t) = self.lookup_var(&var_name) {
                            self.inferred_node_types.insert(cond_node.0, t);
                        }
                    } else {
                        self.check(arena, cond_node, TypeId::BOOL);
                        if cond_kind == NodeKind::BinaryExpr && cond_flags == 9 {
                            let lhs_node = *arena.node(NodeId(arena.node(cond_node).lhs));
                            if lhs_node.node_kind() == NodeKind::IdentExpr {
                                let var_name = arena.interner.resolve(SymbolId(lhs_node.lhs));
                                self.narrow_on_guard_some(var_name);
                            }
                        }
                    }
                }
                let (then_node, else_node) = if node.flags == 2 {
                    let children = arena.children_slice(node.rhs, 2);
                    (NodeId(children[0]), NodeId(children[1]))
                } else {
                    (NodeId(node.rhs), NodeId::NONE)
                };
                let t_then = self.synth(arena, then_node);
                self.exit_scope();
                if else_node.is_some() {
                    self.synth(arena, else_node);
                }
                t_then
            }

            NodeKind::WhileStmt => {
                let cond_node = NodeId(node.lhs);
                self.check(arena, cond_node, TypeId::BOOL);
                let body_node = NodeId(node.rhs);
                self.enter_scope();
                self.synth(arena, body_node);
                self.exit_scope();
                TypeId::VOID
            }

            NodeKind::ForExpr => {
                let sym = SymbolId(node.lhs);
                let var_name = arena.interner.resolve(sym).to_string();
                let body_node = NodeId(node.rhs);

                let elem_ty = if let Some(&iter_node) = arena.for_iter_nodes.get(&node_id.0) {
                    let iter_ty = self.synth(arena, iter_node);
                    let pruned = self.types.prune(iter_ty);
                    match self.types.get(pruned).clone() {
                        TypeKind::Collection { element, .. } => element,
                        TypeKind::Map { key, value } => self.types.tuple_of(vec![key, value]),
                        _ if pruned == TypeId::STR => TypeId::STR,
                        _ => TypeId::I64,
                    }
                } else {
                    TypeId::I64
                };

                self.enter_scope();
                self.define_var(&var_name, elem_ty);
                let body_ty = self.synth(arena, body_node);
                self.exit_scope();

                if body_ty != TypeId::VOID {
                    self.types.list_of(body_ty)
                } else {
                    TypeId::VOID
                }
            }

            NodeKind::GuardStmt => {
                let cond_node = NodeId(node.lhs);
                if cond_node.is_some() {
                    let cond_node_val = *arena.node(cond_node);
                    if cond_node_val.node_kind() == NodeKind::IdentExpr {
                        let var_name = arena.interner.resolve(SymbolId(cond_node_val.lhs));
                        self.narrow_on_guard_some(var_name);
                    } else if cond_node_val.node_kind() == NodeKind::BinaryExpr && cond_node_val.flags == 9 {
                        // guard x != none / guard x != null
                        let lhs_node = *arena.node(NodeId(cond_node_val.lhs));
                        if lhs_node.node_kind() == NodeKind::IdentExpr {
                            let var_name = arena.interner.resolve(SymbolId(lhs_node.lhs));
                            self.narrow_on_guard_some(var_name);
                        }
                    }
                    self.synth(arena, cond_node);
                }
                let else_node = NodeId(node.rhs);
                if else_node.is_some() {
                    self.synth(arena, else_node);
                }
                TypeId::VOID
            }

            NodeKind::DeferStmt => {
                let body = NodeId(node.lhs);
                if body.is_some() {
                    self.synth(arena, body);
                }
                TypeId::VOID
            }

            NodeKind::BreakStmt | NodeKind::ContinueStmt => TypeId::NEVER,

            // --- Function & Method Invocations ---
            NodeKind::CallExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                if children.is_empty() {
                    return TypeId::ANY;
                }
                let callee_node = NodeId(children[0]);
                let callee_ty = self.synth(arena, callee_node);

                // If this is a homoiconic call-quote (`fn!(...)`, `fn!!(...)`, `fn!&(...)`),
                // symbolic arguments inside `children[1..]` may be unbound quoted AST symbols.
                let is_quoted_call = node.flags != 0;
                let prev_diag_len = self.diagnostics.len();
                for &arg_id in &children[1..] {
                    self.synth(arena, NodeId(arg_id));
                }
                if is_quoted_call {
                    self.diagnostics.truncate(prev_diag_len);
                    return TypeId::ANY;
                }

                match self.types.get(self.types.prune(callee_ty)) {
                    TypeKind::Function { ret, .. } => *ret,
                    TypeKind::RowRecord { .. } => callee_ty,
                    TypeKind::Adt { .. } => callee_ty,
                    _ => TypeId::ANY,
                }
            }

            // --- Member & Index Projections ---
            NodeKind::MemberExpr => {
                let target_node = NodeId(node.lhs);
                let target_ty = self.synth(arena, target_node);
                let member_name = arena.interner.resolve(SymbolId(node.rhs));
                let mut pruned = self.types.prune(target_ty);
                let is_safe_nav = node.flags == 1;

                // Member on Option: handle intrinsic Option methods/properties directly
                if let TypeKind::Option(inner) = *self.types.get(pruned) {
                    if !is_safe_nav {
                        match member_name {
                            "OrPanic" | "or_panic" | "UnwrapOr" | "unwrap_or" | "unwrap" | "expect" => {
                                let res = self.types.function_of(vec![inner], inner);
                                self.inferred_node_types.insert(node_id.0, res);
                                return res;
                            }
                            "is_some" | "is_none" | "is_unboxed" => {
                                self.inferred_node_types.insert(node_id.0, TypeId::BOOL);
                                return TypeId::BOOL;
                            }
                            _ => {
                                self.diagnostics.push(AnalysisDiagnostic::error(
                                    "NULL001",
                                    format!("Cannot access member '{}' on nullable value without unwrapping", member_name),
                                    arena.span(node_id),
                                ));
                            }
                        }
                    }
                    pruned = self.types.prune(inner);
                }

                // 1. Check RowRecord fields
                if let TypeKind::RowRecord { fields, .. } = self.types.get(pruned) {
                    if let Some((_, f_ty)) = fields.iter().find(|(k, _)| k == member_name) {
                        let base_res = *f_ty;
                        let res = if is_safe_nav && !matches!(self.types.get(self.types.prune(base_res)), TypeKind::Option(_)) {
                            self.types.option_of(base_res)
                        } else {
                            base_res
                        };
                        self.inferred_node_types.insert(node_id.0, res);
                        return res;
                    }
                }

                // 2. Check struct_fields for named structs/classes/records/ADTs
                let type_name_opt = match self.types.get(pruned) {
                    TypeKind::Primitive(n) => Some(n.clone()),
                    TypeKind::Adt { name, .. } => Some(name.clone()),
                    _ => {
                        let t_node = *arena.node(target_node);
                        if t_node.node_kind() == NodeKind::IdentExpr {
                            let ident_name = arena.interner.resolve(SymbolId(t_node.lhs));
                            if self.struct_fields.contains_key(ident_name) {
                                Some(ident_name.to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                };
                if let Some(type_name) = type_name_opt {
                    if let Some(fields) = self.struct_fields.get(&type_name) {
                        if let Some((_, f_ty)) = fields.iter().find(|(k, _)| k == member_name) {
                            let base_res = *f_ty;
                            let res = if is_safe_nav && !matches!(self.types.get(self.types.prune(base_res)), TypeKind::Option(_)) {
                                self.types.option_of(base_res)
                            } else {
                                base_res
                            };
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                    }
                }

                // 3. Well-known metadata attributes & standard methods
                let str_ret = if is_safe_nav {
                    self.types.option_of(TypeId::STR)
                } else {
                    TypeId::STR
                };
                match member_name {
                    "len" | "size_of" | "tag_bytes" | "length" | "Length" | "count" | "size_bytes" | "table_size" | "abi_header_size" | "managed_list_allocations" => TypeId::I64,
                    "is_unboxed" | "is_recursive" | "is_empty" | "is_some" | "is_none" | "is_ok" | "is_err" | "is_frozen" | "is_perfect_hash" => TypeId::BOOL,
                    "contains" | "starts_with" | "ends_with" | "Toggle" | "IsA" | "Check" => self.types.function_of(vec![TypeId::ANY], TypeId::BOOL),
                    "trim" | "to_upper" | "to_lower" | "to_string" | "Trim" | "ToUpper" | "ToLower" | "ToHex" | "Replace" => self.types.function_of(vec![], str_ret),
                    "ToInt" => {
                        let opt_int = self.types.option_of(TypeId::I64);
                        self.types.function_of(vec![], opt_int)
                    }
                    "BitLength" | "ToBytes" => self.types.function_of(vec![], TypeId::I64),
                    "ToFloat" | "Sqrt" | "Lerp" | "Period" => self.types.function_of(vec![], TypeId::F64),
                    _ => TypeId::ANY,
                }
            }

            NodeKind::IndexExpr => {
                let target_node = NodeId(node.lhs);
                let target_ty = self.synth(arena, target_node);
                let idx_node = NodeId(node.rhs);
                if idx_node.is_some() {
                    self.synth(arena, idx_node);
                }

                match self.types.get(self.types.prune(target_ty)) {
                    TypeKind::Collection { element, .. } => *element,
                    TypeKind::Map { value, .. } => {
                        if *value == TypeId::ANY {
                            TypeId::ANY
                        } else {
                            self.types.option_of(*value)
                        }
                    }
                    TypeKind::Tuple(elems) => {
                        if idx_node.is_some() && arena.node(idx_node).node_kind() == NodeKind::IntLit {
                            let idx = arena.node(idx_node).lhs as usize;
                            elems.get(idx).copied().unwrap_or(TypeId::ANY)
                        } else {
                            elems.first().copied().unwrap_or(TypeId::ANY)
                        }
                    }
                    TypeKind::Option(_) => {
                        self.diagnostics.push(AnalysisDiagnostic::error(
                            "NULL001",
                            "Cannot index into nullable value without unwrapping",
                            arena.span(node_id),
                        ));
                        TypeId::ANY
                    }
                    _ => TypeId::ANY,
                }
            }

            // --- Operators: Pipe, Cascade, Coalesce, TryPropagate, CatchRecovery ---
            NodeKind::PipeExpr => {
                let lhs_ty = self.synth(arena, NodeId(node.lhs));
                let rhs_ty = self.synth(arena, NodeId(node.rhs));
                match self.types.get(self.types.prune(rhs_ty)) {
                    TypeKind::Function { ret, .. } => *ret,
                    _ => lhs_ty,
                }
            }

            NodeKind::CascadeExpr => {
                let receiver_ty = self.synth(arena, NodeId(node.lhs));
                let rhs_id = NodeId(node.rhs);
                self.synth_cascade_op(arena, rhs_id);
                receiver_ty
            }

            NodeKind::TryPropagateExpr => {
                let inner_ty = self.synth(arena, NodeId(node.lhs));
                match self.types.get(self.types.prune(inner_ty)) {
                    TypeKind::Option(elem) => *elem,
                    TypeKind::Result { ok, .. } => *ok,
                    _ => inner_ty,
                }
            }

            NodeKind::CatchRecoveryExpr => {
                let lhs_ty = self.synth(arena, NodeId(node.lhs));
                let rhs_ty = self.synth(arena, NodeId(node.rhs));
                let _ = self.types.unify(lhs_ty, rhs_ty);
                lhs_ty
            }

            NodeKind::CoalesceExpr => {
                let lhs_ty = self.synth(arena, NodeId(node.lhs));
                let rhs_ty = self.synth(arena, NodeId(node.rhs));
                let kind = self.types.get(self.types.prune(lhs_ty)).clone();
                match kind {
                    TypeKind::Option(elem) => {
                        let _ = self.types.unify(elem, rhs_ty);
                        elem
                    }
                    _ => lhs_ty,
                }
            }

            NodeKind::LambdaExpr => {
                self.enter_scope();
                let param_start = node.lhs;
                let param_len = node.flags as usize;
                let mut param_tys = Vec::with_capacity(param_len);
                if param_len > 0 {
                    let params = arena.children_slice(param_start, param_len as u32);
                    for &p_id in params {
                        let p_node = *arena.node(NodeId(p_id));
                        let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                        let p_ty = if p_node.rhs != 0 {
                            self.resolve_type_spec(arena, NodeId(p_node.rhs))
                        } else {
                            TypeId::ANY
                        };
                        self.inferred_node_types.insert(p_id, p_ty);
                        self.define_var(&p_name, p_ty);
                        param_tys.push(p_ty);
                    }
                }
                let body_node = NodeId(node.rhs);
                let body_ty = self.synth(arena, body_node);
                self.exit_scope();
                self.types.function_of(param_tys, body_ty)
            }

            // --- Pattern Matching ---
            NodeKind::MatchExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                if children.is_empty() {
                    return TypeId::ANY;
                }
                let scrutinee = NodeId(children[0]);
                let scrutinee_ty = self.synth(arena, scrutinee);

                let mut unified_arm_ty = TypeId::VOID;
                for &arm_id in &children[1..] {
                    let arm_ty = self.synth_match_arm(arena, NodeId(arm_id), scrutinee_ty);
                    if unified_arm_ty == TypeId::VOID {
                        unified_arm_ty = arm_ty;
                    } else if arm_ty != TypeId::VOID && arm_ty != TypeId::NEVER {
                        let _ = self.types.unify(unified_arm_ty, arm_ty);
                    }
                }
                unified_arm_ty
            }

            NodeKind::MatchArm => {
                self.synth_match_arm(arena, node_id, TypeId::ANY)
            }

            NodeKind::ComptimeExpr => {
                let inner = NodeId(node.lhs);
                self.synth(arena, inner)
            }

            _ => TypeId::ANY,
        };

        self.inferred_node_types.insert(node_id.0, ty);
        ty
    }

    fn synth_match_arm(&mut self, arena: &AstArena, arm_id: NodeId, scrutinee_ty: TypeId) -> TypeId {
        if arm_id.is_none() {
            return TypeId::VOID;
        }
        let node = *arena.node(arm_id);
        let pat_node = NodeId(node.lhs);
        let body_node = NodeId(node.rhs);
        self.enter_scope();
        if pat_node.is_some() {
            self.bind_pattern_vars_typed(arena, pat_node, scrutinee_ty);
        }
        let arm_ty = self.synth(arena, body_node);
        self.exit_scope();
        self.inferred_node_types.insert(arm_id.0, arm_ty);
        arm_ty
    }

    /// Bidirectional: checks that an expression node conforms to expected type.
    pub fn check(&mut self, arena: &AstArena, node_id: NodeId, expected: TypeId) {
        if node_id.is_none() {
            return;
        }

        let actual = self.synth(arena, node_id);
        if let Err(e) = self.types.unify(actual, expected) {
            self.diagnostics.push(AnalysisDiagnostic::error(
                "TYPE004",
                format!(
                    "Type check failed: expected '{}', found '{}' ({})",
                    self.types.display(expected),
                    self.types.display(actual),
                    e
                ),
                arena.span(node_id),
            ));
        }
    }

    /// Binds variables declared inside a pattern into the current scope.
    pub fn bind_pattern_vars(&mut self, arena: &AstArena, pat_id: NodeId) {
        self.bind_pattern_vars_typed(arena, pat_id, TypeId::ANY);
    }

    /// Binds variables declared inside a pattern into the current scope with expected type propagation.
    pub fn bind_pattern_vars_typed(&mut self, arena: &AstArena, pat_id: NodeId, expected_ty: TypeId) {
        if pat_id.is_none() {
            return;
        }
        let node = *arena.node(pat_id);
        let pruned_expected = self.types.prune(expected_ty);
        match node.node_kind() {
            NodeKind::PatBind => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                if name != "_" {
                    self.define_var(&name, pruned_expected);
                }
                self.inferred_node_types.insert(pat_id.0, pruned_expected);
                if node.flags == 1 && node.rhs != 0 {
                    self.bind_pattern_vars_typed(arena, NodeId(node.rhs), pruned_expected);
                }
            }
            NodeKind::PatVariant => {
                let v_name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                let start = node.rhs as usize;
                let len = node.flags as usize;
                let mut payload_tys = Vec::new();
                if let Some((adt_name, p_tys)) = self.adt_variants.get(&v_name).cloned() {
                    payload_tys = p_tys;
                    if let Some(&adt_ty) = self.type_defs.get(&adt_name) {
                        self.inferred_node_types.insert(pat_id.0, adt_ty);
                    }
                } else {
                    match (v_name.as_str(), self.types.get(pruned_expected).clone()) {
                        ("Some" | "some", TypeKind::Option(inner)) => {
                            payload_tys.push(inner);
                        }
                        ("Ok" | "ok", TypeKind::Result { ok, .. }) => {
                            payload_tys.push(ok);
                        }
                        ("Err" | "err", TypeKind::Result { err, .. }) => {
                            payload_tys.push(err);
                        }
                        _ => {}
                    }
                    self.inferred_node_types.insert(pat_id.0, pruned_expected);
                }
                if len > 0 && start + len <= arena.extra_children.len() {
                    let sub_ids: Vec<u32> = arena.extra_children[start..start + len].to_vec();
                    for (idx, sub_id) in sub_ids.into_iter().enumerate() {
                        let sub_ty = payload_tys.get(idx).copied().unwrap_or(TypeId::ANY);
                        self.bind_pattern_vars_typed(arena, NodeId(sub_id), sub_ty);
                    }
                }
            }
            NodeKind::PatTuple => {
                let start = node.lhs as usize;
                let len = node.rhs as usize;
                self.inferred_node_types.insert(pat_id.0, pruned_expected);
                let elem_tys = match self.types.get(pruned_expected).clone() {
                    TypeKind::Tuple(elems) => elems,
                    _ => Vec::new(),
                };
                if len > 0 && start + len <= arena.extra_children.len() {
                    let sub_ids: Vec<u32> = arena.extra_children[start..start + len].to_vec();
                    for (idx, sub_id) in sub_ids.into_iter().enumerate() {
                        let sub_ty = elem_tys.get(idx).copied().unwrap_or(TypeId::ANY);
                        self.bind_pattern_vars_typed(arena, NodeId(sub_id), sub_ty);
                    }
                }
            }
            NodeKind::PatSlice => {
                let start = node.lhs as usize;
                let len = node.rhs as usize;
                self.inferred_node_types.insert(pat_id.0, pruned_expected);
                let elem_ty = match self.types.get(pruned_expected).clone() {
                    TypeKind::Collection { element, .. } => element,
                    _ => TypeId::ANY,
                };
                if len > 0 && start + len <= arena.extra_children.len() {
                    let sub_ids: Vec<u32> = arena.extra_children[start..start + len].to_vec();
                    for sub_id in sub_ids {
                        let sub_node = *arena.node(NodeId(sub_id));
                        let sub_expected = if sub_node.node_kind() == NodeKind::PatSlice
                            || (sub_node.node_kind() == NodeKind::PatBind && sub_node.flags == 2)
                        {
                            self.types.list_of(elem_ty)
                        } else {
                            elem_ty
                        };
                        self.bind_pattern_vars_typed(arena, NodeId(sub_id), sub_expected);
                    }
                } else if node.lhs != 0 {
                    // Rest pattern `..tail`
                    let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                    if name != "_" && !name.is_empty() {
                        self.define_var(&name, pruned_expected);
                    }
                }
            }
            NodeKind::PatOr => {
                self.inferred_node_types.insert(pat_id.0, pruned_expected);
                self.bind_pattern_vars_typed(arena, NodeId(node.lhs), pruned_expected);
                self.bind_pattern_vars_typed(arena, NodeId(node.rhs), pruned_expected);
            }
            NodeKind::IdentExpr => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                if name != "_" && !name.chars().next().map_or(false, |c| c.is_uppercase()) {
                    self.define_var(&name, pruned_expected);
                }
                self.inferred_node_types.insert(pat_id.0, pruned_expected);
            }
            _ => {
                self.inferred_node_types.insert(pat_id.0, pruned_expected);
            }
        }
    }

    fn bind_destructuring_expr(&mut self, arena: &AstArena, target: NodeId, expected_ty: TypeId) {
        if target.is_none() {
            return;
        }
        let node = *arena.node(target);
        match node.node_kind() {
            NodeKind::IdentExpr => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                if name != "_" {
                    self.define_var(&name, expected_ty);
                }
                self.inferred_node_types.insert(target.0, expected_ty);
            }
            NodeKind::UnaryExpr => {
                // Spread `..payload_slice`
                let inner = NodeId(node.lhs);
                let slice_ty = self.types.list_of(expected_ty);
                self.bind_destructuring_expr(arena, inner, slice_ty);
            }
            NodeKind::ListExpr => {
                let elem_ty = match self.types.get(self.types.prune(expected_ty)).clone() {
                    TypeKind::Collection { element, .. } => element,
                    _ => TypeId::ANY,
                };
                let children = arena.children_slice(node.lhs, node.rhs).to_vec();
                for c in children {
                    self.bind_destructuring_expr(arena, NodeId(c), elem_ty);
                }
            }
            NodeKind::TupleExpr => {
                let children = arena.children_slice(node.lhs, node.rhs).to_vec();
                for c in children {
                    self.bind_destructuring_expr(arena, NodeId(c), TypeId::ANY);
                }
            }
            NodeKind::MapExpr => {
                let children = arena.children_slice(node.lhs, node.rhs).to_vec();
                for chunk in children.chunks(2) {
                    if chunk.len() == 2 {
                        self.bind_destructuring_expr(arena, NodeId(chunk[1]), TypeId::ANY);
                    } else if chunk.len() == 1 {
                        self.bind_destructuring_expr(arena, NodeId(chunk[0]), TypeId::ANY);
                    }
                }
            }
            _ => {}
        }
    }

    fn synth_cascade_op(&mut self, arena: &AstArena, op_id: NodeId) {
        if op_id.is_none() {
            return;
        }
        let op_node = *arena.node(op_id);
        match op_node.node_kind() {
            NodeKind::CallExpr => {
                let arg_start = op_node.lhs as usize;
                let arg_len = op_node.rhs as usize;
                // Callee is at arg_start (method name on receiver); synthesize arguments without global lookup of method name
                if arg_len > 1 && arg_start + arg_len <= arena.extra_children.len() {
                    for &arg_id in &arena.extra_children[arg_start + 1..arg_start + arg_len] {
                        self.synth(arena, NodeId(arg_id));
                    }
                }
            }
            NodeKind::IdentExpr => {
                // Member property access on receiver in cascade
            }
            NodeKind::MemberExpr => {
                self.synth_cascade_op(arena, NodeId(op_node.lhs));
            }
            NodeKind::CascadeExpr => {
                self.synth_cascade_op(arena, NodeId(op_node.lhs));
                self.synth_cascade_op(arena, NodeId(op_node.rhs));
            }
            NodeKind::BinaryExpr => {
                if op_node.flags == 26 {
                    // `..checksum_tag() -> let tag`
                    self.synth_cascade_op(arena, NodeId(op_node.rhs));
                    let lhs = NodeId(op_node.lhs);
                    let lhs_node = *arena.node(lhs);
                    if lhs_node.node_kind() == NodeKind::IdentExpr || lhs_node.node_kind() == NodeKind::PatBind {
                        let name = arena.interner.resolve(SymbolId(lhs_node.lhs)).to_string();
                        self.define_var(&name, TypeId::ANY);
                        self.inferred_node_types.insert(lhs.0, TypeId::ANY);
                    }
                } else {
                    self.synth(arena, NodeId(op_node.rhs));
                }
            }
            NodeKind::AssignStmt => {
                self.synth(arena, NodeId(op_node.lhs));
                self.synth_cascade_op(arena, NodeId(op_node.rhs));
            }
            NodeKind::LetStmt | NodeKind::VarStmt => {
                self.synth_cascade_op(arena, NodeId(op_node.rhs));
                let sym = SymbolId(op_node.lhs);
                let name = arena.interner.resolve(sym).to_string();
                self.define_var(&name, TypeId::ANY);
                self.inferred_node_types.insert(op_id.0, TypeId::ANY);
            }
            _ => {
                self.synth(arena, op_id);
            }
        }
    }

    /// Dominator flow narrowing: narrows an `Option[T]` to `T` in downstream block.
    pub fn narrow_on_guard_some(&mut self, var_name: &str) {
        if let Some(existing) = self.lookup_var(var_name) {
            let existing = self.types.prune(existing);
            if let TypeKind::Option(inner) = self.types.get(existing).clone() {
                self.narrow_var(var_name, inner);
            }
        }
    }

    /// Dominator flow narrowing: narrows a base type to active typestate.
    pub fn narrow_on_typestate_transition(&mut self, var_name: &str, state: &str) {
        if let Some(existing) = self.lookup_var(var_name) {
            let base = match self.types.get(existing) {
                TypeKind::Typestate { base, .. } => *base,
                _ => existing,
            };
            let narrowed = self.types.typestate_of(base, state.to_string());
            self.narrow_var(var_name, narrowed);
        }
    }
}
