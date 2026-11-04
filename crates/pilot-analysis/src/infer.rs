//! Milestone 33.1 & 37.1: Flow-Sensitive Bidirectional Type Inference, Complete AstArena Coverage & Narrowing.

use std::collections::HashMap;
use pilot_ast::{AstArena, AstNode, NodeId, NodeKind};
use pilot_intern::SymbolId;
use crate::diagnostics::AnalysisDiagnostic;
use crate::types::{TypeArena, TypeId, TypeKind};

pub struct Scope {
    pub bindings: HashMap<String, TypeId>,
    pub narrowed_types: HashMap<String, TypeId>,
    pub fn_owner: NodeId,
    pub is_var: HashMap<String, bool>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            narrowed_types: HashMap::new(),
            fn_owner: NodeId::NONE,
            is_var: HashMap::new(),
        }
    }
}

pub struct TypeInferenceEngine {
    pub types: TypeArena,
    scopes: Vec<Scope>,
    pub fn_stack: Vec<NodeId>,
    pub local_fn_captures: HashMap<u32, pilot_ast::LocalFnCaptureSpec>,
    pub patch_handles: HashMap<SymbolId, (bool, u32, u32)>,
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub inferred_node_types: HashMap<u32, TypeId>,
    pub type_defs: HashMap<String, TypeId>,
    pub struct_fields: HashMap<String, Vec<(String, TypeId)>>,
    pub adt_variants: HashMap<String, (String, Vec<TypeId>)>, // variant -> (adt_name, field_types)
    pub fn_decls: HashMap<String, NodeId>,
    pub instantiating_fns: std::collections::HashSet<String>,
    pub current_fn_is_static: bool,
    pub current_scope_clearance: u32,
    pub current_enclosing_type: Option<String>,
    pub fn_visibility: HashMap<String, pilot_ast::VisibilityTier>,
    pub fn_clearance: HashMap<String, pilot_ast::ClearanceLevel>,
    pub struct_member_visibility: HashMap<(String, String), pilot_ast::VisibilityTier>,
    pub struct_member_clearance: HashMap<(String, String), pilot_ast::ClearanceLevel>,
    pub dimensions: crate::dimensions::DimensionChecker,
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
            fn_stack: Vec::new(),
            local_fn_captures: HashMap::new(),
            patch_handles: HashMap::new(),
            diagnostics: Vec::new(),
            inferred_node_types: HashMap::new(),
            type_defs: HashMap::new(),
            struct_fields: HashMap::new(),
            adt_variants: HashMap::new(),
            fn_decls: HashMap::new(),
            instantiating_fns: std::collections::HashSet::new(),
            current_fn_is_static: false,
            current_scope_clearance: 0,
            current_enclosing_type: None,
            fn_visibility: HashMap::new(),
            fn_clearance: HashMap::new(),
            struct_member_visibility: HashMap::new(),
            struct_member_clearance: HashMap::new(),
            dimensions: crate::dimensions::DimensionChecker::new(),
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
        self.type_defs.insert("f32".to_string(), TypeId::F32);
        self.type_defs.insert("f16".to_string(), TypeId::F32);
        self.type_defs.insert("f8".to_string(), TypeId::F32);
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
        let order_ty = self.types.alloc(TypeKind::Primitive("order".to_string()));
        self.type_defs.insert("order".to_string(), order_ty);
        let mut order_td = crate::types::TypeDef::new(order_ty, SymbolId(0), crate::types::PackageId(0));
        order_td.physics = crate::types::HardwarePhysics {
            reg_class: crate::types::RegClass::GprSigned,
            bit_width: 8,
        };
        order_td.size = 1;
        order_td.align = 1;
        order_td.lattice = crate::types::LatticeBits::Value;
        order_td.shape = crate::types::StructuralShape::Scalar;
        order_td.is_lang_primitive = true;
        order_td.layout_sealed = true;
        self.types.type_defs.insert(order_ty, order_td);
        self.types.type_defs_by_name.insert("order".to_string(), order_ty);
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
        self.define_var("quote", TypeId::ANY);
        self.define_var("quasiquote", TypeId::ANY);
        let fn_connect = self.types.function_of(vec![TypeId::STR], TypeId::ANY);
        self.define_var("connect", fn_connect);
        let fn_nameof = self.types.function_of(vec![TypeId::ANY], TypeId::STR);
        self.define_var("nameof", fn_nameof);
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
        let list_any = self.types.list_of(TypeId::ANY);
        let fn_table = self.types.function_of(vec![TypeId::ANY], list_any);
        self.define_var("Table", fn_table);
        self.define_var("Dataset", fn_table);
        self.type_defs.insert("Table".to_string(), list_any);
        self.type_defs.insert("Dataset".to_string(), list_any);

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
        let list_i64 = self.types.list_of(TypeId::I64);
        let fn_crypto_bytes = self.types.function_of(vec![TypeId::I64], list_i64);
        self.define_var("__crypto_random_bytes", fn_crypto_bytes);

        // Network Intrinsics
        let fn_net_http_get = self.types.function_of(vec![TypeId::STR], TypeId::ANY);
        let fn_net_http_post = self.types.function_of(vec![TypeId::STR, TypeId::STR, TypeId::STR], TypeId::ANY);
        let fn_net_connect = self.types.function_of(vec![TypeId::STR, TypeId::I64], TypeId::ANY);
        let fn_net_listen = self.types.function_of(vec![TypeId::STR, TypeId::I64], TypeId::ANY);
        let fn_net_hostname = self.types.function_of(vec![], TypeId::STR);
        let fn_net_hostbyname = self.types.function_of(vec![TypeId::STR], TypeId::STR);
        self.define_var("__net_http_get", fn_net_http_get);
        self.define_var("__net_http_post", fn_net_http_post);
        self.define_var("__net_tcp_connect", fn_net_connect);
        self.define_var("__net_tcp_listen", fn_net_listen);
        self.define_var("__net_gethostname", fn_net_hostname);
        self.define_var("__net_gethostbyname", fn_net_hostbyname);

        // Time & Timer Intrinsics
        let fn_time_i64 = self.types.function_of(vec![], TypeId::I64);
        let fn_time_f64 = self.types.function_of(vec![], TypeId::F64);
        let fn_sleep_ms = self.types.function_of(vec![TypeId::I64], TypeId::VOID);
        self.define_var("__time_now_ns", fn_time_i64);
        self.define_var("__time_now_micros", fn_time_i64);
        self.define_var("__time_now_millis", fn_time_i64);
        self.define_var("__time_perf_counter", fn_time_f64);
        self.define_var("__time_sleep_ms", fn_sleep_ms);

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

        let fn_void_i64 = self.types.function_of(vec![], TypeId::I64);
        let fn_str_str = self.types.function_of(vec![TypeId::STR], TypeId::STR);
        let fn_str_str_bool = self.types.function_of(vec![TypeId::STR, TypeId::STR], TypeId::BOOL);
        let fn_str_bool = self.types.function_of(vec![TypeId::STR], TypeId::BOOL);
        let fn_str_i64 = self.types.function_of(vec![TypeId::STR], TypeId::I64);
        let fn_void_list_str = self.types.function_of(vec![], list_str);
        let fn_i64_void = self.types.function_of(vec![TypeId::I64], TypeId::VOID);
        let fn_void_void = self.types.function_of(vec![], TypeId::VOID);

        self.define_var("__os_platform", fn_void_str);
        self.define_var("__os_arch", fn_void_str);
        self.define_var("__os_family", fn_void_str);
        self.define_var("__os_is_linux", fn_void_bool);
        self.define_var("__os_is_macos", fn_void_bool);
        self.define_var("__os_is_android", fn_void_bool);
        self.define_var("__os_is_ios", fn_void_bool);
        self.define_var("__os_getenv", fn_str_str);
        self.define_var("__os_setenv", fn_str_str_bool);
        self.define_var("__os_unsetenv", fn_str_bool);
        self.define_var("__os_environ", fn_void_list_str);
        self.define_var("__os_getpid", fn_void_i64);
        self.define_var("__os_getppid", fn_void_i64);
        self.define_var("__os_getuid", fn_void_i64);
        self.define_var("__os_getgid", fn_void_i64);
        self.define_var("__os_cpu_count", fn_void_i64);
        self.define_var("__os_system", fn_str_i64);
        self.define_var("__os_homedir", fn_void_str);
        self.define_var("__os_chdir", fn_str_bool);
        self.define_var("__os_disk_usage", fn_fs_read);
        self.define_var("__os_abort", fn_void_void);
        self.define_var("__sys_argv", fn_void_list_str);
        self.define_var("__sys_exit", fn_i64_void);
        self.define_var("__sys_executable", fn_void_str);
        self.define_var("__sys_pointer_size", fn_void_i64);
        self.define_var("__sys_byteorder", fn_void_str);

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
        let cur_fn = self.fn_stack.last().copied().unwrap_or(NodeId::NONE);
        let mut sc = Scope::new();
        sc.fn_owner = cur_fn;
        self.scopes.push(sc);
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

    pub fn define_var_mut(&mut self, name: &str, ty: TypeId, is_var: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(name.to_string(), ty);
            scope.is_var.insert(name.to_string(), is_var);
        }
    }

    pub fn lookup_var_owner(&self, name: &str) -> Option<(NodeId, bool)> {
        for scope in self.scopes.iter().rev() {
            if scope.bindings.contains_key(name) {
                let is_v = scope.is_var.get(name).copied().unwrap_or(false);
                return Some((scope.fn_owner, is_v));
            }
        }
        None
    }

    pub fn record_capture(
        &mut self,
        _arena: &AstArena,
        local_fn: NodeId,
        _owner_fn: NodeId,
        sym: SymbolId,
        ty: TypeId,
        is_var: bool,
        span: pilot_span::ByteSpan,
    ) {
        if let Some(spec) = self.local_fn_captures.get_mut(&local_fn.0) {
            if !spec.captures.iter().any(|c| c.sym == sym) {
                let mode = if is_var {
                    pilot_ast::CaptureMode::ByVarRef
                } else {
                    pilot_ast::CaptureMode::ByImmRef
                };
                spec.captures.push(pilot_ast::CapturedVar {
                    sym,
                    ty: ty.0,
                    mode,
                    outer_slot_coord: (0, 0),
                    span,
                });
                spec.lattice_bits = 0b01; // IReference
            }
        }
    }

    pub fn mark_capture_mutating(&mut self, local_fn: NodeId, sym: SymbolId) {
        if let Some(spec) = self.local_fn_captures.get_mut(&local_fn.0) {
            for cap in &mut spec.captures {
                if cap.sym == sym {
                    cap.mode = pilot_ast::CaptureMode::ByVarRef;
                }
            }
            spec.lattice_bits = 0b01;
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
                return Some(self.types.prune(narrowed));
            }
            if let Some(&ty) = scope.bindings.get(name) {
                return Some(self.types.prune(ty));
            }
        }
        if name == "self" || name == "this" || name == "field" || name == "value" {
            return Some(TypeId::ANY);
        }
        self.type_defs.get(name).copied()
    }

    /// Finalizes inferred types by deep-pruning all type variables across all AST nodes and scopes.
    pub fn finalize_inferred_types(&mut self) {
        let keys: Vec<u32> = self.inferred_node_types.keys().copied().collect();
        for k in keys {
            if let Some(&ty) = self.inferred_node_types.get(&k) {
                let pruned = self.types.deep_prune(ty);
                self.inferred_node_types.insert(k, pruned);
            }
        }
        for scope in &mut self.scopes {
            for (_, ty) in scope.bindings.iter_mut() {
                *ty = self.types.deep_prune(*ty);
            }
            for (_, ty) in scope.narrowed_types.iter_mut() {
                *ty = self.types.deep_prune(*ty);
            }
        }
    }

    fn eval_const_usize(&self, arena: &AstArena, node_id: NodeId) -> Option<usize> {
        if node_id.is_none() || (node_id.0 as usize) >= arena.nodes.len() {
            return None;
        }
        let n = *arena.node(node_id);
        match n.node_kind() {
            NodeKind::IntLit => arena
                .i64_pool
                .get(n.lhs as usize)
                .map(|&v| v.max(0) as usize),
            NodeKind::TypeNamed | NodeKind::IdentExpr => {
                let s = arena.interner.resolve(SymbolId(n.lhs));
                s.parse::<usize>().ok()
            }
            _ => None,
        }
    }

    pub fn alloc_simd_type(&mut self, elem_ty: TypeId, lanes: usize) -> TypeId {
        let (elem_sz, _) = self.types.type_size_align(elem_ty);
        let elem_bits = (elem_sz * 8).max(8) as u32;
        let ty = self.types.alloc(TypeKind::Primitive(format!(
            "simd<{},{}>",
            self.types.display(elem_ty),
            lanes
        )));
        let mut td = crate::types::TypeDef::new(
            ty,
            SymbolId(0),
            crate::types::PackageId(0),
        );
        td.physics = crate::types::HardwarePhysics {
            reg_class: crate::types::RegClass::SimdVector {
                lanes: lanes as u16,
            },
            bit_width: (lanes as u32) * elem_bits,
        };
        td.size = lanes * elem_sz.max(1);
        td.align = td.size.min(64).max(1);
        td.lattice = crate::types::LatticeBits::Value;
        td.shape = crate::types::StructuralShape::InlineRepeat {
            elem_ty,
            count: lanes,
        };
        td.is_lang_primitive = true;
        td.layout_sealed = true;
        self.types.type_defs.insert(ty, td);
        ty
    }

    pub fn alloc_grid_type(&mut self, elem_ty: TypeId, rows: usize, cols: usize) -> TypeId {
        let (elem_sz, elem_al) = self.types.type_size_align(elem_ty);
        let count = rows.max(1) * cols.max(1);
        let ty = self.types.alloc(TypeKind::Matrix {
            rows: rows as u32,
            cols: cols as u32,
        });
        let mut td = crate::types::TypeDef::new(
            ty,
            SymbolId(0),
            crate::types::PackageId(0),
        );
        td.size = count * elem_sz.max(1);
        td.align = elem_al.max(1);
        td.lattice = crate::types::LatticeBits::Value;
        td.shape = crate::types::StructuralShape::InlineRepeat { elem_ty, count };
        td.is_lang_primitive = true;
        td.layout_sealed = true;
        self.types.type_defs.insert(ty, td);
        ty
    }

    pub fn alloc_array_type(&mut self, elem_ty: TypeId, count: usize) -> TypeId {
        let (elem_sz, elem_al) = self.types.type_size_align(elem_ty);
        let ty = self.types.alloc(TypeKind::Primitive(format!(
            "array<{},{}>",
            self.types.display(elem_ty),
            count
        )));
        let mut td = crate::types::TypeDef::new(
            ty,
            SymbolId(0),
            crate::types::PackageId(0),
        );
        td.size = count.max(1) * elem_sz.max(1);
        td.align = elem_al.max(1);
        td.lattice = crate::types::LatticeBits::Value;
        td.shape = crate::types::StructuralShape::InlineRepeat { elem_ty, count };
        td.is_lang_primitive = true;
        td.layout_sealed = true;
        self.types.type_defs.insert(ty, td);
        ty
    }

    pub fn alloc_ring_type(&mut self, elem_ty: TypeId) -> TypeId {
        let ty = self.types.alloc(TypeKind::Primitive(format!(
            "ring<{}>",
            self.types.display(elem_ty)
        )));
        let mut td = crate::types::TypeDef::new(
            ty,
            SymbolId(0),
            crate::types::PackageId(0),
        );
        td.size = 24;
        td.align = 8;
        td.lattice = crate::types::LatticeBits::Reference;
        td.shape = crate::types::StructuralShape::StridedView { elem_ty };
        td.is_lang_primitive = true;
        td.layout_sealed = true;
        self.types.type_defs.insert(ty, td);
        ty
    }

    pub fn alloc_signal_type(&mut self, elem_ty: TypeId) -> TypeId {
        let ty = self.types.alloc(TypeKind::Primitive(format!(
            "signal<{}>",
            self.types.display(elem_ty)
        )));
        let mut td = crate::types::TypeDef::new(
            ty,
            SymbolId(0),
            crate::types::PackageId(0),
        );
        td.size = 16;
        td.align = 8;
        td.lattice = crate::types::LatticeBits::Object;
        td.is_lang_primitive = true;
        td.layout_sealed = true;
        self.types.type_defs.insert(ty, td);
        ty
    }

    pub fn alloc_pin_type(&mut self, elem_ty: TypeId) -> TypeId {
        let ty = self.types.alloc(TypeKind::Primitive(format!(
            "pin<{}>",
            self.types.display(elem_ty)
        )));
        let mut td = crate::types::TypeDef::new(
            ty,
            SymbolId(0),
            crate::types::PackageId(0),
        );
        td.size = 8;
        td.align = 8;
        td.lattice = crate::types::LatticeBits::Guard;
        td.is_lang_primitive = true;
        td.layout_sealed = true;
        self.types.type_defs.insert(ty, td);
        ty
    }

    pub fn alloc_bounds_pair_type(&mut self, bound_ty: TypeId, inclusive: bool) -> TypeId {
        let (b_sz, b_al) = self.types.type_size_align(bound_ty);
        let ty = self.types.alloc(TypeKind::Primitive(format!(
            "bounds<{},{}>",
            self.types.display(bound_ty),
            inclusive
        )));
        let mut td = crate::types::TypeDef::new(
            ty,
            SymbolId(0),
            crate::types::PackageId(0),
        );
        td.size = 2 * b_sz.max(1);
        td.align = b_al.max(1);
        td.lattice = crate::types::LatticeBits::Value;
        td.shape = crate::types::StructuralShape::BoundsPair {
            bound_ty,
            inclusive,
        };
        td.is_lang_primitive = true;
        td.layout_sealed = true;
        self.types.type_defs.insert(ty, td);
        ty
    }

    pub fn extract_element_type(&self, ty: TypeId) -> TypeId {
        let pruned = self.types.prune(ty);
        if let Some(td) = self.types.type_defs.get(&pruned) {
            match td.shape {
                crate::types::StructuralShape::ContiguousView { elem_ty }
                | crate::types::StructuralShape::StridedView { elem_ty }
                | crate::types::StructuralShape::OwnedBuffer { elem_ty }
                | crate::types::StructuralShape::InlineRepeat { elem_ty, .. } => return elem_ty,
                _ => {}
            }
        }
        match self.types.get(pruned) {
            TypeKind::Collection { element, .. } => self.types.prune(*element),
            _ => pruned,
        }
    }

    pub fn extract_simd_or_tuple_lane_type(&self, ty: TypeId) -> TypeId {
        let pruned = self.types.prune(ty);
        if let Some(td) = self.types.type_defs.get(&pruned) {
            if let crate::types::StructuralShape::InlineRepeat { elem_ty, .. } = td.shape {
                return elem_ty;
            }
        }
        match self.types.get(pruned) {
            TypeKind::Tuple(elems) => elems.first().copied().unwrap_or(TypeId::F32),
            TypeKind::Collection { element, .. } => self.types.prune(*element),
            _ => TypeId::F32,
        }
    }

    pub fn resolve_field_on_type(&self, ty: TypeId, field_name: &str) -> TypeId {
        let pruned = self.types.prune(ty);
        if let Some(name) = self.types.nominal_names.get(&pruned) {
            if let Some(fields) = self.struct_fields.get(name) {
                if let Some((_, fty)) = fields.iter().find(|(n, _)| n == field_name) {
                    return self.types.prune(*fty);
                }
            }
        }
        match self.types.get(pruned) {
            TypeKind::RowRecord { fields, name, .. } => {
                if let Some((_, fty)) = fields.iter().find(|(n, _)| n == field_name) {
                    return self.types.prune(*fty);
                }
                if let Some(n) = name {
                    if let Some(sfields) = self.struct_fields.get(n) {
                        if let Some((_, fty)) = sfields.iter().find(|(fnm, _)| fnm == field_name) {
                            return self.types.prune(*fty);
                        }
                    }
                }
                TypeId::ANY
            }
            TypeKind::Primitive(name) => {
                if let Some(fields) = self.struct_fields.get(name) {
                    if let Some((_, fty)) = fields.iter().find(|(n, _)| n == field_name) {
                        return self.types.prune(*fty);
                    }
                }
                TypeId::ANY
            }
            _ => TypeId::ANY,
        }
    }

    fn eval_unit_formula_exponents(
        &self,
        arena: &AstArena,
        expr_id: NodeId,
    ) -> std::collections::BTreeMap<String, i32> {
        if expr_id.is_none() || (expr_id.0 as usize) >= arena.nodes.len() {
            return std::collections::BTreeMap::new();
        }
        let n = *arena.node(expr_id);
        match n.node_kind() {
            NodeKind::IdentExpr | NodeKind::TypeNamed => {
                let u_name = arena.interner.resolve(SymbolId(n.lhs)).to_string();
                self.dimensions
                    .unit_exponents
                    .get(&u_name)
                    .cloned()
                    .unwrap_or_else(|| {
                        let mut m = std::collections::BTreeMap::new();
                        m.insert(u_name, 1);
                        m
                    })
            }
            NodeKind::BinaryExpr => {
                let lhs_map = self.eval_unit_formula_exponents(arena, NodeId(n.lhs));
                let rhs_map = self.eval_unit_formula_exponents(arena, NodeId(n.rhs));
                let mut res = lhs_map;
                let sign = if n.flags == 3 { 1 } else { -1 };
                for (k, v) in rhs_map {
                    let entry = res.entry(k.clone()).or_insert(0);
                    *entry += sign * v;
                    if *entry == 0 {
                        res.remove(&k);
                    }
                }
                res
            }
            _ => std::collections::BTreeMap::new(),
        }
    }

    fn register_unit_decl(&mut self, arena: &AstArena, node_id: NodeId, node: &AstNode) -> TypeId {
        let sym = SymbolId(node.lhs);
        let unit_name = arena.interner.resolve(sym).to_string();
        let base_ty = arena
            .type_annotations
            .get(&node_id.0)
            .map(|&ann| self.resolve_type_spec(arena, ann))
            .unwrap_or(TypeId::F64);
        if node.rhs == u32::MAX || NodeId(node.rhs).is_none() {
            self.dimensions.register_base_unit(&unit_name);
        } else {
            let exps = self.eval_unit_formula_exponents(arena, NodeId(node.rhs));
            self.dimensions.register_derived_unit(&unit_name, exps);
        }
        let unit_ty = self.types.unit_of(base_ty, unit_name.clone());
        let (sz, al) = self.types.type_size_align(base_ty);
        let physics = self.types.type_physics(base_ty);
        let mut td = crate::types::TypeDef::new(
            unit_ty,
            sym,
            crate::types::PackageId(0),
        );
        td.physics = physics;
        td.size = sz;
        td.align = al;
        td.lattice = crate::types::LatticeBits::Value;
        td.shape = crate::types::StructuralShape::Scalar;
        td.layout_sealed = true;
        self.types.register_type_def(td, &arena.interner);
        self.type_defs.insert(unit_name.clone(), unit_ty);
        let ctor_ty = self.types.function_of(vec![base_ty], unit_ty);
        self.define_var(&unit_name, ctor_ty);
        self.inferred_node_types.insert(node_id.0, unit_ty);
        unit_ty
    }

    fn register_struct_typedef_if_needed(
        &mut self,
        arena: &AstArena,
        node_id: NodeId,
        ty: TypeId,
        name: &str,
        fields: &[(String, TypeId)],
    ) {
        if self.types.type_defs.contains_key(&ty) {
            return;
        }
        let sym = arena.node(node_id).lhs;
        let mut td = crate::types::TypeDef::new(
            ty,
            SymbolId(sym),
            crate::types::PackageId(0),
        );
        if let Some(base_nodes) = arena.struct_base_types.get(&node_id.0) {
            for &bnode in base_nodes {
                let isym = arena
                    .type_base_symbols
                    .get(&bnode.0)
                    .copied()
                    .unwrap_or_else(|| SymbolId(arena.node(bnode).lhs));
                let iname = arena.interner.resolve(isym);
                if let Some(&iid) = self.types.interfaces_by_name.get(iname) {
                    td.declared_interfaces.push(iid);
                }
            }
        }
        let body = NodeId(arena.node(node_id).rhs);
        if body.is_some() && (body.0 as usize) < arena.nodes.len() {
            let b_node = *arena.node(body);
            if b_node.node_kind() == NodeKind::BlockExpr {
                for &cid in arena.children_slice(b_node.lhs, b_node.rhs) {
                    if arena.node(NodeId(cid)).node_kind() == NodeKind::DestructorDecl {
                        td.destructor_fn = Some(crate::types::FunctionId(cid));
                    }
                }
            }
        }
        self.types.register_type_def(td, &arena.interner);
        let rec_fields = arena.record_fields.get(&node_id.0);
        for (idx, (fname, fty)) in fields.iter().enumerate() {
            if fname == "this[]"
                || fname == "self[]"
                || fname == "self"
                || fname == "~self"
                || fname == "scope"
                || matches!(self.types.get(self.types.prune(*fty)), TypeKind::Function { .. })
            {
                continue;
            }
            let fsym = rec_fields
                .and_then(|rf| {
                    rf.iter()
                        .find(|(s, _)| arena.interner.resolve(*s) == fname)
                        .map(|(s, _)| *s)
                })
                .unwrap_or(SymbolId(idx as u32));
            self.types
                .add_struct_field(ty, fsym, *fty, 0, arena.span(node_id));
        }
        let mut seal_diags = Vec::new();
        self.types
            .seal_primary_struct_layout(ty, &arena.interner, &mut seal_diags);
        self.type_defs.insert(name.to_string(), ty);
    }

    pub fn resolve_type_spec(&mut self, arena: &AstArena, node_id: NodeId) -> TypeId {
        if node_id.is_none() {
            return TypeId::ANY;
        }

        let node = *arena.node(node_id);
        let ty = match node.node_kind() {
            NodeKind::TypeNamed => {
                let name = arena.interner.resolve(SymbolId(node.lhs));
                if let Some(&ty) = self.types.type_defs_by_name.get(name) {
                    ty
                } else if let Some(&ty) = self.type_defs.get(name) {
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
            NodeKind::TypeList => {
                let children: Vec<u32> = if node.rhs > 0 {
                    arena.children_slice(node.lhs, node.rhs).to_vec()
                } else if node.lhs != 0 {
                    vec![node.lhs]
                } else {
                    Vec::new()
                };
                let elem = children
                    .first()
                    .map(|&c| self.resolve_type_spec(arena, NodeId(c)))
                    .unwrap_or(TypeId::ANY);
                if let Some(&base_sym) = arena.type_base_symbols.get(&node_id.0) {
                    let base_name = arena.interner.resolve(base_sym);
                    match base_name {
                        "simd" => {
                            let lanes = children
                                .get(1)
                                .and_then(|&c| self.eval_const_usize(arena, NodeId(c)))
                                .unwrap_or(4);
                            return {
                                let res = self.alloc_simd_type(elem, lanes);
                                self.inferred_node_types.insert(node_id.0, res);
                                res
                            };
                        }
                        "grid" => {
                            let rows = children
                                .get(1)
                                .and_then(|&c| self.eval_const_usize(arena, NodeId(c)))
                                .unwrap_or(1);
                            let cols = children
                                .get(2)
                                .and_then(|&c| self.eval_const_usize(arena, NodeId(c)))
                                .unwrap_or(1);
                            return {
                                let res = self.alloc_grid_type(elem, rows, cols);
                                self.inferred_node_types.insert(node_id.0, res);
                                res
                            };
                        }
                        "array" => {
                            let count = children
                                .get(1)
                                .and_then(|&c| self.eval_const_usize(arena, NodeId(c)))
                                .unwrap_or(1);
                            return {
                                let res = self.alloc_array_type(elem, count);
                                self.inferred_node_types.insert(node_id.0, res);
                                res
                            };
                        }
                        "ring" => {
                            let res = self.alloc_ring_type(elem);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "signal" => {
                            let res = self.alloc_signal_type(elem);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "pin" => {
                            let res = self.alloc_pin_type(elem);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "stride" => {
                            let res = self.types.strided_slice_of(elem);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "slice" => {
                            let res = self.types.slice_of(elem);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "option" => {
                            let res = self.types.option_of(elem);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "result" => {
                            let err = children
                                .get(1)
                                .map(|&c| self.resolve_type_spec(arena, NodeId(c)))
                                .unwrap_or(TypeId::ANY);
                            let res = self.types.result_of(elem, err);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "map" => {
                            let val = children
                                .get(1)
                                .map(|&c| self.resolve_type_spec(arena, NodeId(c)))
                                .unwrap_or(TypeId::ANY);
                            let res = self.types.slice_map_of(elem, val);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        _ => {}
                    }
                }
                self.types.list_of(elem)
            }
            NodeKind::TypeSlice => {
                match node.flags {
                    5 => {
                        // T@N -> simd<T, N>
                        let elem = self.resolve_type_spec(arena, NodeId(node.lhs));
                        let lanes = self.eval_const_usize(arena, NodeId(node.rhs)).unwrap_or(4);
                        self.alloc_simd_type(elem, lanes)
                    }
                    6 => {
                        // T[:] -> ring<T>
                        let elem = self.resolve_type_spec(arena, NodeId(node.lhs));
                        self.alloc_ring_type(elem)
                    }
                    7 => {
                        // T^ -> signal<T>
                        let elem = self.resolve_type_spec(arena, NodeId(node.lhs));
                        self.alloc_signal_type(elem)
                    }
                    8 => {
                        // @T -> pin<T>
                        let elem = self.resolve_type_spec(arena, NodeId(node.lhs));
                        self.alloc_pin_type(elem)
                    }
                    9 => {
                        // T[N, M] -> grid<T, N, M>
                        let children: Vec<u32> = arena.children_slice(node.lhs, node.rhs).to_vec();
                        let elem = children
                            .first()
                            .map(|&c| self.resolve_type_spec(arena, NodeId(c)))
                            .unwrap_or(TypeId::ANY);
                        let rows = children
                            .get(1)
                            .and_then(|&c| self.eval_const_usize(arena, NodeId(c)))
                            .unwrap_or(1);
                        let cols = children
                            .get(2)
                            .and_then(|&c| self.eval_const_usize(arena, NodeId(c)))
                            .unwrap_or(1);
                        self.alloc_grid_type(elem, rows, cols)
                    }
                    _ => {
                        let children: Vec<u32> = if node.rhs > 0 {
                            arena.children_slice(node.lhs, node.rhs).to_vec()
                        } else {
                            Vec::new()
                        };
                        let elem = if !children.is_empty() {
                            self.resolve_type_spec(arena, NodeId(children[0]))
                        } else {
                            self.resolve_type_spec(arena, NodeId(node.lhs))
                        };
                        match node.flags {
                            1 => {
                                if let Some(count) = children
                                    .get(1)
                                    .and_then(|&c| self.eval_const_usize(arena, NodeId(c)))
                                {
                                    self.alloc_array_type(elem, count)
                                } else {
                                    self.types.slice_buffer_of(elem)
                                }
                            }
                            2 => self.types.slice_view_of(elem),
                            3 => self.types.strided_slice_of(elem),
                            4 => self.types.view_of(elem),
                            _ => self.types.slice_of(elem),
                        }
                    }
                }
            }
            NodeKind::TypeMap => {
                let key = self.resolve_type_spec(arena, NodeId(node.lhs));
                let val = self.resolve_type_spec(arena, NodeId(node.rhs));
                if node.flags == 1 {
                    self.types.slice_map_of(key, val)
                } else {
                    self.types.map_of(key, val)
                }
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
                self.types.union_of(vec![lhs, rhs])
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

    /// Recursively collects member property names accessed on a parameter name within an AST subtree.
    fn collect_member_fields(&self, arena: &AstArena, node_id: NodeId, param_name: &str, fields: &mut Vec<String>) {
        if node_id.is_none() {
            return;
        }
        let node = *arena.node(node_id);
        if node.node_kind() == NodeKind::MemberExpr {
            let target_node = *arena.node(NodeId(node.lhs));
            if target_node.node_kind() == NodeKind::IdentExpr {
                let ident_name = arena.interner.resolve(SymbolId(target_node.lhs));
                if ident_name == param_name {
                    let member_name = arena.interner.resolve(SymbolId(node.rhs)).to_string();
                    if !fields.contains(&member_name) {
                        fields.push(member_name);
                    }
                }
            }
        }
        match node.node_kind() {
            NodeKind::BinaryExpr => {
                self.collect_member_fields(arena, NodeId(node.lhs), param_name, fields);
                self.collect_member_fields(arena, NodeId(node.rhs), param_name, fields);
            }
            NodeKind::UnaryExpr | NodeKind::ReturnStmt | NodeKind::YieldStmt | NodeKind::ExprStmt => {
                self.collect_member_fields(arena, NodeId(node.lhs), param_name, fields);
            }
            NodeKind::BlockExpr | NodeKind::ListExpr | NodeKind::TupleExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                for &cid in children {
                    self.collect_member_fields(arena, NodeId(cid), param_name, fields);
                }
            }
            NodeKind::IfExpr => {
                self.collect_member_fields(arena, NodeId(node.lhs), param_name, fields);
                if node.flags == 0 {
                    self.collect_member_fields(arena, NodeId(node.rhs), param_name, fields);
                } else {
                    let extra = arena.children_slice(node.rhs, 2);
                    if extra.len() >= 2 {
                        self.collect_member_fields(arena, NodeId(extra[0]), param_name, fields);
                        self.collect_member_fields(arena, NodeId(extra[1]), param_name, fields);
                    }
                }
            }
            _ => {}
        }
    }

    /// Infers the type of a function/lambda parameter, checking for explicit type specs
    /// or synthesizing open row records from member accesses in the function body.
    fn infer_param_type(&mut self, arena: &AstArena, p_node: &AstNode, body_node: NodeId) -> TypeId {
        let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
        let p_has_type = NodeId(p_node.rhs).is_some()
            && (p_node.rhs as usize) < arena.nodes.len()
            && arena.node(NodeId(p_node.rhs)).node_kind().is_type_spec();
        if p_has_type {
            self.resolve_type_spec(arena, NodeId(p_node.rhs))
        } else if (p_name == "self" || p_name == "this") && self.lookup_var(&p_name).is_some() {
            self.lookup_var(&p_name).unwrap()
        } else {
            let mut accessed_fields = Vec::new();
            self.collect_member_fields(arena, body_node, &p_name, &mut accessed_fields);
            if !accessed_fields.is_empty() {
                let mut fields = Vec::new();
                for f in accessed_fields {
                    fields.push((f, TypeId::ANY));
                }
                fields.sort_by(|a, b| a.0.cmp(&b.0));
                self.types.row_record_of(fields, true, None)
            } else {
                TypeId::ANY
            }
        }
    }

    fn extract_clearance_elevation(&self, arena: &AstArena, cond_id: NodeId) -> Option<u32> {
        if cond_id.is_none() {
            return None;
        }
        let c_node = *arena.node(cond_id);
        if c_node.node_kind() == NodeKind::BinaryExpr {
            let lhs_id = NodeId(c_node.lhs);
            let rhs_id = NodeId(c_node.rhs);
            if lhs_id.is_some() && rhs_id.is_some() {
                let lhs = *arena.node(lhs_id);
                let rhs = *arena.node(rhs_id);
                if lhs.node_kind() == NodeKind::IdentExpr && rhs.node_kind() == NodeKind::IntLit {
                    let name = arena.interner.resolve(SymbolId(lhs.lhs));
                    if name == "lvl" || name == "clearance" {
                        if let Some(&val) = arena.i64_pool.get(rhs.lhs as usize) {
                            if val >= 0 {
                                let uval = val as u32;
                                if c_node.flags == pilot_ast::BinaryOp::Ge as u16 {
                                    return Some(uval);
                                } else if c_node.flags == pilot_ast::BinaryOp::Gt as u16 {
                                    return Some(uval + 1);
                                }
                            }
                        }
                    }
                } else if lhs.node_kind() == NodeKind::IntLit && rhs.node_kind() == NodeKind::IdentExpr {
                    let name = arena.interner.resolve(SymbolId(rhs.lhs));
                    if name == "lvl" || name == "clearance" {
                        if let Some(&val) = arena.i64_pool.get(lhs.lhs as usize) {
                            if val >= 0 {
                                let uval = val as u32;
                                if c_node.flags == pilot_ast::BinaryOp::Le as u16 {
                                    return Some(uval);
                                } else if c_node.flags == pilot_ast::BinaryOp::Lt as u16 {
                                    return Some(uval + 1);
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Extracts field & method signatures from a struct/class/record declaration.
    fn collect_decl_fields(&mut self, arena: &AstArena, node_id: NodeId, parent_name: &str) -> Vec<(String, TypeId)> {
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
                self.struct_member_visibility.insert((parent_name.to_string(), f_name.clone()), pilot_ast::VisibilityTier::Internal);
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
                            let f_vis = arena.node_visibility.get(&stmt_id).copied().unwrap_or(pilot_ast::VisibilityTier::Internal);
                            self.struct_member_visibility.insert((parent_name.to_string(), f_name.clone()), f_vis);
                            if let Some(&clr) = arena.node_clearance.get(&stmt_id) {
                                self.struct_member_clearance.insert((parent_name.to_string(), f_name.clone()), clr);
                            }
                            self.inferred_node_types.insert(stmt_id, f_ty);
                            fields.push((f_name, f_ty));
                        }
                        NodeKind::FnDecl => {
                            let m_name = arena.interner.resolve(SymbolId(s_node.lhs)).to_string();
                            let m_vis = arena.node_visibility.get(&stmt_id).copied().unwrap_or(pilot_ast::VisibilityTier::Private);
                            self.struct_member_visibility.insert((parent_name.to_string(), m_name.clone()), m_vis);
                            if let Some(&clr) = arena.node_clearance.get(&stmt_id) {
                                self.struct_member_clearance.insert((parent_name.to_string(), m_name.clone()), clr);
                            }
                            let fn_start = s_node.rhs;
                            let fn_len = s_node.flags as usize;
                            if fn_len == 0 {
                                // Property with get/set accessors
                                let ret_ty = if let Some(&ann) = arena.type_annotations.get(&stmt_id) {
                                    self.resolve_type_spec(arena, ann)
                                } else if NodeId(s_node.rhs).is_some() {
                                    self.synth(arena, NodeId(s_node.rhs))
                                } else {
                                    TypeId::ANY
                                };
                                self.inferred_node_types.insert(stmt_id, ret_ty);
                                fields.push((m_name, ret_ty));
                                continue;
                            }
                            let mut param_tys = Vec::new();
                            if fn_len > 0 {
                                let fn_children = arena.children_slice(fn_start, fn_len as u32);
                                for &p_id in &fn_children[1..] {
                                    let p_node = *arena.node(NodeId(p_id));
                                    let p_name = arena.interner.resolve(SymbolId(p_node.lhs));
                                    if p_name == "self" || p_name == "this" {
                                        continue;
                                    }
                                    let p_ty = if NodeId(p_node.rhs).is_some()
                                        && (p_node.rhs as usize) < arena.nodes.len()
                                        && arena.node(NodeId(p_node.rhs)).node_kind().is_type_spec()
                                    {
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
                            fields.push((m_name.clone(), m_ty));

                            if m_name.starts_with("__") && m_name.ends_with("__") && m_name.len() > 4 {
                                let span = arena.span(NodeId(stmt_id));
                                self.diagnostics.push(AnalysisDiagnostic::error(
                                    "E0100",
                                    format!("Python-style dunder method '{}' is prohibited in Pilot. Use first-class 'self' or 'op' declarations instead.", m_name),
                                    span,
                                ));
                            }
                        }
                        NodeKind::ConstructorDecl => {
                            let fn_start = s_node.lhs;
                            let fn_len = s_node.rhs as usize;
                            let mut param_tys = Vec::new();
                            if fn_len > 1 {
                                let fn_children = arena.children_slice(fn_start, fn_len as u32);
                                for &p_id in &fn_children[1..] {
                                    let p_node = *arena.node(NodeId(p_id));
                                    let p_ty = if NodeId(p_node.rhs).is_some()
                                        && (p_node.rhs as usize) < arena.nodes.len()
                                        && arena.node(NodeId(p_node.rhs)).node_kind().is_type_spec()
                                    {
                                        self.resolve_type_spec(arena, NodeId(p_node.rhs))
                                    } else {
                                        TypeId::ANY
                                    };
                                    param_tys.push(p_ty);
                                }
                            }
                            let ctor_ty = self.types.function_of(param_tys, TypeId::VOID);
                            fields.push(("self".to_string(), ctor_ty));
                        }
                        NodeKind::DestructorDecl => {
                            let dtor_ty = self.types.function_of(vec![], TypeId::VOID);
                            fields.push(("~self".to_string(), dtor_ty));
                        }
                        NodeKind::ScopeDecl => {
                            let scope_ty = self.types.function_of(vec![], TypeId::VOID);
                            fields.push(("scope".to_string(), scope_ty));
                        }
                        NodeKind::IndexerDecl => {
                            let ret_ty = arena.type_annotations.get(&stmt_id).map(|&ann| self.resolve_type_spec(arena, ann)).unwrap_or(TypeId::ANY);
                            let params_slice = arena.children_slice(s_node.lhs, s_node.rhs);
                            let mut param_tys = Vec::new();
                            for &p_id in params_slice {
                                let p_node = *arena.node(NodeId(p_id));
                                let p_ty = if NodeId(p_node.rhs).is_some() && (p_node.rhs as usize) < arena.nodes.len() && arena.node(NodeId(p_node.rhs)).node_kind().is_type_spec() {
                                    self.resolve_type_spec(arena, NodeId(p_node.rhs))
                                } else {
                                    TypeId::ANY
                                };
                                param_tys.push(p_ty);
                            }
                            let idx_ty = self.types.function_of(param_tys, ret_ty);
                            fields.push(("this[]".to_string(), idx_ty));
                            fields.push(("self[]".to_string(), idx_ty));
                        }
                        NodeKind::DynamicMemberDecl => {
                            let ret_ty = arena.type_annotations.get(&stmt_id).map(|&ann| self.resolve_type_spec(arena, ann)).unwrap_or(TypeId::ANY);
                            let dyn_ty = self.types.function_of(vec![TypeId::STR], ret_ty);
                            fields.push(("self.()".to_string(), dyn_ty));
                        }
                        NodeKind::OperatorDecl => {
                            let sym_name = if let Some(&sym) = arena.operator_symbols.get(&stmt_id) {
                                arena.interner.resolve(sym).to_string()
                            } else {
                                "op".to_string()
                            };
                            let fn_start = s_node.lhs;
                            let fn_len = s_node.rhs as usize;
                            let mut param_tys = Vec::new();
                            if fn_len > 1 {
                                let fn_children = arena.children_slice(fn_start, fn_len as u32);
                                for &p_id in &fn_children[1..] {
                                    let p_node = *arena.node(NodeId(p_id));
                                    let p_ty = if NodeId(p_node.rhs).is_some() && (p_node.rhs as usize) < arena.nodes.len() && arena.node(NodeId(p_node.rhs)).node_kind().is_type_spec() {
                                        self.resolve_type_spec(arena, NodeId(p_node.rhs))
                                    } else {
                                        TypeId::ANY
                                    };
                                    param_tys.push(p_ty);
                                }
                            }
                            let ret_ty = arena.type_annotations.get(&stmt_id).map(|&ann| self.resolve_type_spec(arena, ann)).unwrap_or(TypeId::ANY);
                            let op_ty = self.types.function_of(param_tys, ret_ty);
                            fields.push((format!("op {}", sym_name), op_ty));
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

        if self.patch_handles.is_empty() && !arena.node_attributes.is_empty() {
            for (&_nid, attrs) in &arena.node_attributes {
                for attr in attrs {
                    if let Some(sym) = attr.binding_sym {
                        let mut lo = 0u32;
                        let mut hi = 1000u32;
                        if !attr.args.is_empty() {
                            let arg0 = *arena.node(attr.args[0]);
                            if arg0.node_kind() == NodeKind::BinaryExpr || arg0.node_kind() == NodeKind::RangeExpr {
                                if arg0.lhs != 0 {
                                    let l_node = *arena.node(NodeId(arg0.lhs));
                                    if l_node.node_kind() == NodeKind::IntLit {
                                        lo = arena.i64_pool.get(l_node.lhs as usize).copied().unwrap_or(0) as u32;
                                    }
                                }
                                if arg0.rhs != 0 {
                                    let r_node = *arena.node(NodeId(arg0.rhs));
                                    if r_node.node_kind() == NodeKind::IntLit {
                                        hi = arena.i64_pool.get(r_node.lhs as usize).copied().unwrap_or(1000) as u32;
                                    }
                                }
                            } else if arg0.node_kind() == NodeKind::IntLit {
                                if attr.args.len() >= 2 {
                                    lo = arena.i64_pool.get(arg0.lhs as usize).copied().unwrap_or(0) as u32;
                                    let arg1 = *arena.node(attr.args[1]);
                                    if arg1.node_kind() == NodeKind::IntLit {
                                        hi = arena.i64_pool.get(arg1.lhs as usize).copied().unwrap_or(1000) as u32;
                                    }
                                } else {
                                    hi = arena.i64_pool.get(arg0.lhs as usize).copied().unwrap_or(1000) as u32;
                                }
                            }
                        }
                        self.patch_handles.insert(sym, (attr.is_mutable_binding, lo, hi));
                        let name = arena.interner.resolve(sym);
                        self.define_var(name, TypeId::ANY);
                    }
                }
            }
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
                if let Some(&conn_node) = arena.polyglot_conns.get(&node_id.0) {
                    let _ = self.synth(arena, conn_node);
                }

                if (node.flags & 1) != 0 {
                    self.types.function_of(vec![TypeId::ANY], TypeId::ANY)
                } else {
                    let lang_sym = SymbolId(node.lhs);
                    let lang = arena.interner.resolve(lang_sym);
                    if matches!(lang, "sql" | "postgres" | "postgresql" | "mysql" | "sqlite" | "db" | "database") {
                        let code_sym = SymbolId(node.rhs);
                        let code_str = arena.interner.resolve(code_sym);
                        self.infer_sql_query_type(code_str)
                    } else {
                        TypeId::ANY
                    }
                }
            }

            // --- Identifier & Variable Lookup ---
            NodeKind::IdentExpr => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym);
                if name == "self" && self.current_fn_is_static {
                    self.diagnostics.push(AnalysisDiagnostic::error(
                        "STATIC001",
                        "Cannot access 'self' or '.field' inside static method",
                        arena.span(node_id),
                    ));
                    return TypeId::ANY;
                }
                if name == "_" {
                    TypeId::ANY
                } else if let Some(t) = self.lookup_var(name) {
                    if self.fn_stack.len() > 1 {
                        let cur_fn = *self.fn_stack.last().unwrap();
                        if let Some((owner_fn, is_var)) = self.lookup_var_owner(name) {
                            if owner_fn.is_some() && owner_fn != cur_fn {
                                if arena.static_methods.contains(&cur_fn.0) {
                                    self.diagnostics.push(AnalysisDiagnostic::error(
                                        "STATIC002",
                                        format!("Static local function cannot capture enclosing state '{}'", name),
                                        arena.span(node_id),
                                    ));
                                } else {
                                    self.record_capture(arena, cur_fn, owner_fn, sym, t, is_var, arena.span(node_id));
                                }
                            }
                        }
                    }
                    t
                } else if !self.current_fn_is_static && self.lookup_var("self").is_some() {
                    let self_ty = self.lookup_var("self").unwrap();
                    let pruned_self = self.types.prune(self_ty);
                    let self_type_name = match self.types.get(pruned_self) {
                        TypeKind::Primitive(n) => Some(n.clone()),
                        TypeKind::Adt { name, .. } => Some(name.clone()),
                        _ => self.types.nominal_names.get(&pruned_self).cloned(),
                    };
                    if let Some(sname) = self_type_name {
                        if let Some(fields) = self.struct_fields.get(&sname) {
                            if let Some((_, fty)) = fields.iter().find(|(fname, _)| fname == name) {
                                return *fty;
                            }
                        }
                    }
                    if let Some(&t) = self.type_defs.get(name) {
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
                if op == 32 {
                    // BinaryOp::As (Bit-cast or SliceView)
                    let target_ty = self.resolve_type_spec(arena, NodeId(node.rhs));
                    let _ = self.synth(arena, NodeId(node.lhs));
                    self.inferred_node_types.insert(node_id.0, target_ty);
                    return target_ty;
                }
                if op == 33 {
                    // BinaryOp::AsView (xs as view<Target>)
                    let target_spec = self.resolve_type_spec(arena, NodeId(node.rhs));
                    let lhs_ty = self.synth(arena, NodeId(node.lhs));
                    let src_elem = self.extract_element_type(lhs_ty);
                    let src_lat = self.types.type_lattice(src_elem);
                    let dst_lat = self.types.type_lattice(target_spec);
                    if src_lat != crate::types::LatticeBits::Value {
                        self.diagnostics.push(AnalysisDiagnostic::lattice_contract_mismatch(
                            arena.span(node_id),
                            &self.types.display(src_elem),
                            "IValue",
                            0b00,
                            src_lat.raw(),
                            "source element type of 'as view<T>' must be a 0b00 IValue type",
                        ));
                    } else if dst_lat != crate::types::LatticeBits::Value {
                        self.diagnostics.push(AnalysisDiagnostic::lattice_contract_mismatch(
                            arena.span(node_id),
                            &self.types.display(target_spec),
                            "IValue",
                            0b00,
                            dst_lat.raw(),
                            "target type of 'as view<T>' must be a 0b00 IValue type",
                        ));
                    }
                    let view_ty = self.types.strided_slice_of(target_spec);
                    self.inferred_node_types.insert(node_id.0, view_ty);
                    return view_ty;
                }
                if op == 45 {
                    // BinaryOp::Lens (`xs ~> .field`, `xs ~> (.k, .v)`, `vec ~> (.2, .1, .0)`)
                    let lhs_ty = self.synth(arena, NodeId(node.lhs));
                    let rhs_id = NodeId(node.rhs);
                    if rhs_id.is_some() && (rhs_id.0 as usize) < arena.nodes.len() {
                        let rhs_node = *arena.node(rhs_id);
                        match rhs_node.node_kind() {
                            NodeKind::MemberExpr => {
                                let src_elem = self.extract_element_type(lhs_ty);
                                let src_lat = self.types.type_lattice(src_elem);
                                if src_lat != crate::types::LatticeBits::Value {
                                    self.diagnostics.push(
                                        AnalysisDiagnostic::lattice_contract_mismatch(
                                            arena.span(node_id),
                                            &self.types.display(src_elem),
                                            "IValue",
                                            0b00,
                                            src_lat.raw(),
                                            "OffsetLens ('~>') requires 0b00 IValue element type",
                                        ),
                                    );
                                }
                                let f_name = arena.interner.resolve(SymbolId(rhs_node.rhs));
                                let f_ty = self.resolve_field_on_type(src_elem, f_name);
                                let res = self.types.strided_slice_of(f_ty);
                                self.inferred_node_types.insert(node_id.0, res);
                                return res;
                            }
                            NodeKind::TupleExpr => {
                                let t_children: Vec<u32> =
                                    arena.children_slice(rhs_node.lhs, rhs_node.rhs).to_vec();
                                let mut all_lane_indices = !t_children.is_empty();
                                let mut field_names = Vec::new();
                                for &cid in &t_children {
                                    let cn = *arena.node(NodeId(cid));
                                    if cn.node_kind() == NodeKind::IntLit {
                                        continue;
                                    } else if cn.node_kind() == NodeKind::MemberExpr {
                                        let m_name =
                                            arena.interner.resolve(SymbolId(cn.rhs)).to_string();
                                        if m_name.parse::<usize>().is_err() {
                                            all_lane_indices = false;
                                        }
                                        field_names.push(m_name);
                                    } else {
                                        all_lane_indices = false;
                                    }
                                }
                                if all_lane_indices {
                                    let lane_ty = self.extract_simd_or_tuple_lane_type(lhs_ty);
                                    let res = self.alloc_simd_type(lane_ty, t_children.len());
                                    self.inferred_node_types.insert(node_id.0, res);
                                    return res;
                                } else if field_names.len() == 2 {
                                    let src_elem = self.extract_element_type(lhs_ty);
                                    let src_lat = self.types.type_lattice(src_elem);
                                    if src_lat != crate::types::LatticeBits::Value {
                                        self.diagnostics.push(
                                            AnalysisDiagnostic::lattice_contract_mismatch(
                                                arena.span(node_id),
                                                &self.types.display(src_elem),
                                                "IValue",
                                                0b00,
                                                src_lat.raw(),
                                                "SliceMap projection ('~>') requires 0b00 IValue element type",
                                            ),
                                        );
                                    }
                                    let k_ty = self.resolve_field_on_type(src_elem, &field_names[0]);
                                    let v_ty = self.resolve_field_on_type(src_elem, &field_names[1]);
                                    let res = self.types.slice_map_of(k_ty, v_ty);
                                    self.inferred_node_types.insert(node_id.0, res);
                                    return res;
                                }
                            }
                            _ => {}
                        }
                    }
                    self.inferred_node_types.insert(node_id.0, TypeId::ANY);
                    return TypeId::ANY;
                }
                if op == 47 && (node.rhs == u32::MAX || NodeId(node.rhs).is_none()) {
                    // Postfix `a >..` (BinaryOp::AfterRange)
                    let lhs_ty = self.synth(arena, NodeId(node.lhs));
                    let p_lhs = self.types.prune(lhs_ty);
                    let res = self.alloc_bounds_pair_type(p_lhs, false);
                    self.inferred_node_types.insert(node_id.0, res);
                    return res;
                }
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
                    if rhs.is_none() || (rhs.0 as usize) >= arena.nodes.len() {
                        self.inferred_node_types.insert(node_id.0, TypeId::BOOL);
                        return TypeId::BOOL;
                    }
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
                let p_lhs = self.types.prune(lhs_ty);
                let p_rhs = self.types.prune(rhs_ty);
                // Check compile-time Unit dimensional algebra (`+`, `-`, `*`, `/`)
                if let (
                    TypeKind::Unit { base: b1, unit_name: u1 },
                    TypeKind::Unit { base: _b2, unit_name: u2 },
                ) = (self.types.get(p_lhs).clone(), self.types.get(p_rhs).clone())
                {
                    if op == 1 || op == 2 {
                        if !self.dimensions.units_match(&u1, &u2) {
                            self.diagnostics.push(AnalysisDiagnostic::error(
                                "DIM001",
                                format!(
                                    "Incompatible dimensional units in addition/subtraction: '{}' vs '{}'",
                                    u1, u2
                                ),
                                arena.span(node_id),
                            ));
                        }
                        self.inferred_node_types.insert(node_id.0, p_lhs);
                        return p_lhs;
                    } else if op == 3 || op == 4 || op == 5 {
                        let exps = self.dimensions.combine_unit_exponents(&u1, op, &u2);
                        let res_ty = if let Some(derived_name) =
                            self.dimensions.find_unit_by_exponents(&exps)
                        {
                            self.types.unit_of(b1, derived_name)
                        } else {
                            self.types.unit_of(b1, format!("{}#{}", u1, u2))
                        };
                        self.inferred_node_types.insert(node_id.0, res_ty);
                        return res_ty;
                    }
                }
                if op == 34 {
                    // BinaryOp::Spaceship (`<=>`)
                    let _ = self.types.unify(lhs_ty, rhs_ty);
                    let order_ty = self
                        .types
                        .type_defs_by_name
                        .get("order")
                        .copied()
                        .or_else(|| self.type_defs.get("order").copied())
                        .unwrap_or(TypeId::I64);
                    self.inferred_node_types.insert(node_id.0, order_ty);
                    return order_ty;
                }
                if (35..=40).contains(&op) || op == 44 {
                    // Saturating (`+|`, `-|`, `*|`), Wrapping (`+%`, `-%`, `*%`), Euclidean (`%%`)
                    let unified = self.types.unify(lhs_ty, rhs_ty).unwrap_or(p_lhs);
                    self.inferred_node_types.insert(node_id.0, unified);
                    return unified;
                }
                if (41..=43).contains(&op) {
                    // Checked (`+?`, `-?`, `*?`) -> option<T>
                    let unified = self.types.unify(lhs_ty, rhs_ty).unwrap_or(p_lhs);
                    let opt_ty = self.types.option_of(unified);
                    self.inferred_node_types.insert(node_id.0, opt_ty);
                    return opt_ty;
                }
                if op == 46 || op == 47 {
                    // OpenRange (`>..<`) and AfterRange (`>..`) -> BoundsPair
                    let res = self.alloc_bounds_pair_type(p_lhs, false);
                    self.inferred_node_types.insert(node_id.0, res);
                    return res;
                }
                if op == 48 {
                    // Swap (`:=:`) -> void
                    let _ = self.types.unify(lhs_ty, rhs_ty);
                    self.inferred_node_types.insert(node_id.0, TypeId::VOID);
                    return TypeId::VOID;
                }
                // Comparison operators (Eq=8..Ge=13) and logical operators (And=14, Or=15)
                if (8..=15).contains(&op) {
                    let _ = self.types.unify(lhs_ty, rhs_ty);
                    TypeId::BOOL
                } else if op == 1 && p_lhs == TypeId::STR {
                    // String concatenation (str + any -> str)
                    TypeId::STR
                } else if (p_lhs.is_integer() && p_rhs.is_float()) || (p_lhs.is_float() && p_rhs.is_integer()) {
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
                } else if op == 23 || op == 24 {
                    // BinaryOp::RangeInclusive (..) and BinaryOp::RangeHalfOpen (..<)
                    if p_lhs == TypeId::STR && p_rhs == TypeId::STR {
                        let res = self.types.list_of(TypeId::STR);
                        self.inferred_node_types.insert(node_id.0, res);
                        return res;
                    } else if p_lhs.is_integer() || p_rhs.is_integer() {
                        let res = self.types.list_of(TypeId::I64);
                        self.inferred_node_types.insert(node_id.0, res);
                        return res;
                    } else {
                        let res = self.types.list_of(p_lhs);
                        self.inferred_node_types.insert(node_id.0, res);
                        return res;
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
                if op_flag == 2 || op_flag == 6 {
                    // Spread operator (..collection) or Splat (*)
                    match self.types.get(self.types.prune(operand_ty)).clone() {
                        TypeKind::Collection { element, .. } => element,
                        _ => operand_ty,
                    }
                } else if op_flag == 7 {
                    // Double splat (**)
                    operand_ty
                } else if op_flag == 8 {
                    // Deep flatten (*..)
                    let pruned = self.types.prune(operand_ty);
                    let nominal_name = self.types.nominal_names.get(&pruned).cloned();
                    if let Some(nom) = nominal_name {
                        if let Some(fields) = self.struct_fields.get(&nom).cloned() {
                            if let Some((_, flat_fn_ty)) = fields.iter().find(|(n, _)| n == "op flat" || n.starts_with("op flat")) {
                                let pruned_flat = self.types.prune(*flat_fn_ty);
                                if let TypeKind::Function { ret, .. } = self.types.get(pruned_flat) {
                                    return self.types.prune(*ret);
                                }
                            }
                        }
                    }
                    let mut curr = pruned;
                    while let TypeKind::Collection { element, .. } = *self.types.get(curr) {
                        curr = self.types.prune(element);
                    }
                    curr
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
                    let mut elem_types = Vec::with_capacity(children.len());
                    for &c in children {
                        elem_types.push(self.synth(arena, NodeId(c)));
                    }
                    let elem_ty = self.types.unify_or_union_all(elem_types);
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

            NodeKind::MapExpr if node.flags == 1 => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let elem_ty = if let Some(&first) = children.first() {
                    self.synth(arena, NodeId(first))
                } else {
                    TypeId::ANY
                };
                for &child in children {
                    let _ = self.synth(arena, NodeId(child));
                }
                self.types.alloc(TypeKind::Collection {
                    element: elem_ty,
                    kind: crate::types::CollectionKind::Set,
                })
            }

            NodeKind::MapExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut is_record = !children.is_empty();
                let mut record_fields = Vec::new();
                let mut seen_keys = std::collections::HashSet::new();

                let mut k_types = Vec::new();
                let mut v_types = Vec::new();
                for chunk in children.chunks(2) {
                    if chunk.len() == 2 {
                        let k_id = NodeId(chunk[0]);
                        let v_id = NodeId(chunk[1]);
                        let k_node = *arena.node(k_id);
                        let k_ty = self.synth(arena, k_id);
                        let v_ty = self.synth(arena, v_id);
                        k_types.push(k_ty);
                        v_types.push(v_ty);

                        if is_record {
                            let key_name_opt = match k_node.node_kind() {
                                NodeKind::StrLit | NodeKind::IdentExpr => {
                                    Some(arena.interner.resolve(SymbolId(k_node.lhs)).to_string())
                                }
                                NodeKind::KeywordLit => {
                                    arena.interner.resolve_keyword(pilot_intern::KeywordId(k_node.lhs))
                                        .map(|e| arena.interner.resolve(e.name).to_string())
                                }
                                _ => None,
                            };
                            if let Some(key_name) = key_name_opt {
                                if seen_keys.insert(key_name.clone()) {
                                    record_fields.push((key_name, v_ty));
                                } else {
                                    is_record = false;
                                }
                            } else {
                                is_record = false;
                            }
                        }
                    }
                }

                if is_record {
                    record_fields.sort_by(|a, b| a.0.cmp(&b.0));
                    self.types.row_record_of(record_fields, false, None)
                } else {
                    let k_ty = if k_types.is_empty() { TypeId::ANY } else { self.types.unify_or_union_all(k_types) };
                    let v_ty = if v_types.is_empty() { TypeId::ANY } else { self.types.unify_or_union_all(v_types) };
                    self.types.map_of(k_ty, v_ty)
                }
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
                            let vis = arena.node_visibility.get(&c).copied().unwrap_or(pilot_ast::VisibilityTier::Private);
                            self.fn_visibility.insert(fn_name.clone(), vis);
                            if let Some(&clr) = arena.node_clearance.get(&c) {
                                self.fn_clearance.insert(fn_name.clone(), clr);
                            }
                            self.fn_decls.insert(fn_name.clone(), NodeId(c));
                            let fn_start = child_node.rhs;
                            let fn_len = child_node.flags as usize;
                            let mut param_tys = Vec::new();
                            if fn_len > 0 {
                                let fn_children = arena.children_slice(fn_start, fn_len as u32);
                                let body_node = NodeId(fn_children[0]);
                                for &p_id in &fn_children[1..] {
                                    let p_node = *arena.node(NodeId(p_id));
                                    let p_ty = self.infer_param_type(arena, &p_node, body_node);
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
                            self.inferred_node_types.insert(c, fn_ty);
                        }
                        NodeKind::RecordDecl | NodeKind::StructDecl => {
                            if child_node.flags == 2 {
                                self.register_unit_decl(arena, NodeId(c), &child_node);
                                continue;
                            }
                            let rec_name = arena.interner.resolve(SymbolId(child_node.lhs)).to_string();
                            let fields = self.collect_decl_fields(arena, NodeId(c), &rec_name);
                            self.struct_fields.insert(rec_name.clone(), fields.clone());
                            let ty = self.types.row_record_with_name(fields.clone(), true, None, Some(rec_name.clone()));
                            self.type_defs.insert(rec_name.clone(), ty);
                            self.types.register_nominal_name(ty, rec_name.clone());
                            self.register_struct_typedef_if_needed(arena, NodeId(c), ty, &rec_name, &fields);
                            let data_fields: Vec<TypeId> = fields
                                .iter()
                                .filter(|(n, t)| n != "this[]" && n != "self[]" && !matches!(self.types.get(self.types.prune(*t)), TypeKind::Function { .. }))
                                .map(|(_, t)| *t)
                                .collect();
                            let param_tys = if data_fields.is_empty() {
                                vec![TypeId::ANY]
                            } else {
                                data_fields
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
                            self.types.register_nominal_name(adt_ty, adt_name.clone());
                            self.type_defs.insert(adt_name.clone(), adt_ty);
                        }
                        NodeKind::ClassDecl | NodeKind::InterfaceDecl => {
                            let cls_name = arena.interner.resolve(SymbolId(child_node.lhs)).to_string();
                            let ty = self.types.alloc(TypeKind::Primitive(cls_name.clone()));
                            self.types.register_nominal_name(ty, cls_name.clone());
                            self.type_defs.insert(cls_name.clone(), ty);
                            let fields = self.collect_decl_fields(arena, NodeId(c), &cls_name);
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
                self.finalize_inferred_types();
                last_ty
            }

            NodeKind::BlockExpr => {
                self.enter_scope();
                let children = arena.children_slice(node.lhs, node.rhs);

                // Pass 1: Pre-register sibling local function headers in lexical scope
                // This enables forward references, mutual recursion, and self-recursion!
                for &c in children {
                    let child_node = *arena.node(NodeId(c));
                    if child_node.node_kind() == NodeKind::FnDecl {
                        let sym = SymbolId(child_node.lhs);
                        if sym.0 != 0 {
                            let fn_name = arena.interner.resolve(sym).to_string();
                            self.fn_decls.insert(fn_name.clone(), NodeId(c));
                            let explicit_ret = arena
                                .type_annotations
                                .get(&c)
                                .map(|&ann| self.resolve_type_spec(arena, ann));
                            let fn_start = child_node.rhs;
                            let fn_len = child_node.flags as usize;
                            let mut param_tys = Vec::new();
                            if fn_len > 1 {
                                let fn_children = arena.children_slice(fn_start, fn_len as u32);
                                for &p_id in &fn_children[1..] {
                                    let p_ty = if let Some(&ann) = arena.type_annotations.get(&p_id) {
                                        self.resolve_type_spec(arena, ann)
                                    } else {
                                        TypeId::ANY
                                    };
                                    param_tys.push(p_ty);
                                }
                            }
                            let placeholder = self
                                .types
                                .function_of(param_tys, explicit_ret.unwrap_or(TypeId::ANY));
                            self.define_var(&fn_name, placeholder);
                        }
                    }
                }

                // Pass 2: Typecheck statements & local function bodies
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
                        let p_exp = self.types.prune(explicit_ty);
                        let p_init = self.types.prune(init_ty);
                        if let (
                            TypeKind::Unit { unit_name: u1, .. },
                            TypeKind::Unit { unit_name: u2, .. },
                        ) = (self.types.get(p_exp).clone(), self.types.get(p_init).clone())
                        {
                            if !self.dimensions.units_match(&u1, &u2) {
                                self.diagnostics.push(AnalysisDiagnostic::error(
                                    "DIM001",
                                    format!("Unit dimension mismatch: expected '{}', found '{}'", u1, u2),
                                    arena.span(node_id),
                                ));
                            }
                        }
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
                let is_var = node.node_kind() == NodeKind::VarStmt;
                if node.flags == 1 {
                    self.bind_pattern_vars_typed(arena, NodeId(node.lhs), val_ty);
                } else {
                    let sym = SymbolId(node.lhs);
                    let name = arena.interner.resolve(sym).to_string();
                    self.define_var_mut(&name, val_ty, is_var);
                }
                val_ty
            }

            NodeKind::AssignStmt => {
                let target = NodeId(node.lhs);
                let val = NodeId(node.rhs);
                let val_ty = self.synth(arena, val);
                if target.is_some() {
                    let target_node = *arena.node(target);
                    if target_node.node_kind() == NodeKind::IdentExpr {
                        let sym = SymbolId(target_node.lhs);
                        let name = arena.interner.resolve(sym);
                        if self.fn_stack.len() > 1 {
                            let cur_fn = *self.fn_stack.last().unwrap();
                            if let Some((owner_fn, _is_var)) = self.lookup_var_owner(name) {
                                if owner_fn.is_some() && owner_fn != cur_fn {
                                    if arena.pure_methods.contains(&cur_fn.0) {
                                        self.diagnostics.push(AnalysisDiagnostic::error(
                                            "PURITY001",
                                            format!("Pure local function cannot mutate captured outer variable '{}'", name),
                                            arena.span(target),
                                        ));
                                    } else {
                                        self.mark_capture_mutating(cur_fn, sym);
                                    }
                                }
                            }
                        }
                    }
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
                    let ret_ty = self.synth(arena, expr_node);
                    let ret_n = *arena.node(expr_node);
                    if ret_n.node_kind() == NodeKind::IdentExpr {
                        let sym = SymbolId(ret_n.lhs);
                        let name = arena.interner.resolve(sym);
                        if let Some(&fn_id) = self.fn_decls.get(name) {
                            if let Some(spec) = self.local_fn_captures.get_mut(&fn_id.0) {
                                if spec.lattice_bits == 0b01 {
                                    spec.escapes_as_first_class_value = true;
                                    self.diagnostics.push(AnalysisDiagnostic::error(
                                        "PERCEUS001",
                                        format!("Local function '{}' capturing enclosing scope has lattice 0b01 (IReference) and cannot escape enclosing stack frame", name),
                                        arena.span(expr_node),
                                    ));
                                }
                            }
                        }
                    }
                    ret_ty
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
                self.fn_decls.insert(fn_name.clone(), node_id);
                let fn_start = node.rhs;
                let fn_len = node.flags as usize;
                let is_static = arena.static_methods.contains(&node_id.0);
                let old_is_static = self.current_fn_is_static;
                self.current_fn_is_static = is_static;
                let old_clearance = self.current_scope_clearance;
                if let Some(clr) = arena.node_clearance.get(&node_id.0) {
                    self.current_scope_clearance = clr.level;
                }

                let is_embedded = !self.fn_stack.is_empty();
                let enclosing_fn = self.fn_stack.last().copied().unwrap_or(NodeId::NONE);
                self.fn_stack.push(node_id);

                if is_embedded {
                    self.local_fn_captures.insert(node_id.0, pilot_ast::LocalFnCaptureSpec {
                        fn_node: node_id,
                        enclosing_fn_node: enclosing_fn,
                        lifted_symbol: sym,
                        captures: Vec::new(),
                        lattice_bits: 0b00,
                        escapes_as_first_class_value: false,
                    });
                }

                let explicit_ret_ty = arena
                    .type_annotations
                    .get(&node_id.0)
                    .map(|&ann| self.resolve_type_spec(arena, ann));

                let mut param_tys = Vec::new();
                if fn_len > 0 {
                    let fn_children = arena.children_slice(fn_start, fn_len as u32);
                    let body_node = NodeId(fn_children[0]);
                    for &p_id in &fn_children[1..] {
                        let p_node = *arena.node(NodeId(p_id));
                        let p_ty = self.infer_param_type(arena, &p_node, body_node);
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
                self.fn_stack.pop();
                self.current_fn_is_static = old_is_static;
                self.current_scope_clearance = old_clearance;

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

            NodeKind::ConstructorDecl => {
                let fn_start = node.lhs;
                let fn_len = node.rhs as usize;
                if fn_len > 0 {
                    let fn_children = arena.children_slice(fn_start, fn_len as u32);
                    let body_node = NodeId(fn_children[0]);
                    self.enter_scope();
                    for &p_id in &fn_children[1..] {
                        let p_node = *arena.node(NodeId(p_id));
                        let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                        let p_ty = if NodeId(p_node.rhs).is_some() && (p_node.rhs as usize) < arena.nodes.len() && arena.node(NodeId(p_node.rhs)).node_kind().is_type_spec() {
                            self.resolve_type_spec(arena, NodeId(p_node.rhs))
                        } else {
                            TypeId::ANY
                        };
                        self.define_var(&p_name, p_ty);
                    }
                    if body_node.is_some() {
                        self.synth(arena, body_node);
                    }
                    self.exit_scope();
                }
                TypeId::VOID
            }

            NodeKind::DestructorDecl => {
                let body_node = NodeId(node.lhs);
                if body_node.is_some() {
                    self.enter_scope();
                    self.synth(arena, body_node);
                    self.exit_scope();
                }
                TypeId::VOID
            }

            NodeKind::ScopeDecl => {
                let enter_body = NodeId(node.lhs);
                let exit_body = NodeId(node.rhs);
                if enter_body.is_some() {
                    self.enter_scope();
                    self.synth(arena, enter_body);
                    self.exit_scope();
                }
                if exit_body.is_some() {
                    self.enter_scope();
                    self.synth(arena, exit_body);
                    self.exit_scope();
                }
                TypeId::VOID
            }

            NodeKind::IndexerDecl => {
                let ret_ty = arena.type_annotations.get(&node_id.0).map(|&ann| self.resolve_type_spec(arena, ann)).unwrap_or(TypeId::ANY);
                if let Some(clauses) = arena.accessor_clauses.get(&node_id.0) {
                    if clauses.get_body.is_some() {
                        self.enter_scope();
                        self.synth(arena, clauses.get_body);
                        self.exit_scope();
                    }
                    if clauses.set_body.is_some() {
                        self.enter_scope();
                        if let Some((sym, ty_node)) = clauses.set_param {
                            let name = arena.interner.resolve(sym).to_string();
                            let ty = if ty_node.is_some() && (ty_node.0 as usize) < arena.nodes.len() && arena.node(ty_node).node_kind().is_type_spec() {
                                self.resolve_type_spec(arena, ty_node)
                            } else {
                                ret_ty
                            };
                            self.define_var(&name, ty);
                        }
                        self.synth(arena, clauses.set_body);
                        self.exit_scope();
                    }
                    if clauses.default_body.is_some() {
                        self.enter_scope();
                        self.synth(arena, clauses.default_body);
                        self.exit_scope();
                    }
                }
                ret_ty
            }

            NodeKind::DynamicMemberDecl => {
                let ret_ty = arena.type_annotations.get(&node_id.0).map(|&ann| self.resolve_type_spec(arena, ann)).unwrap_or(TypeId::ANY);
                if let Some(clauses) = arena.accessor_clauses.get(&node_id.0) {
                    if clauses.get_body.is_some() {
                        self.enter_scope();
                        self.synth(arena, clauses.get_body);
                        self.exit_scope();
                    }
                    if clauses.set_body.is_some() {
                        self.enter_scope();
                        if let Some((sym, ty_node)) = clauses.set_param {
                            let name = arena.interner.resolve(sym).to_string();
                            let ty = if ty_node.is_some() && (ty_node.0 as usize) < arena.nodes.len() && arena.node(ty_node).node_kind().is_type_spec() {
                                self.resolve_type_spec(arena, ty_node)
                            } else {
                                ret_ty
                            };
                            self.define_var(&name, ty);
                        }
                        self.synth(arena, clauses.set_body);
                        self.exit_scope();
                    }
                }
                ret_ty
            }

            NodeKind::OperatorDecl => {
                let fn_start = node.lhs;
                let fn_len = node.rhs as usize;
                let ret_ty = arena.type_annotations.get(&node_id.0).map(|&ann| self.resolve_type_spec(arena, ann)).unwrap_or(TypeId::ANY);
                if fn_len > 0 {
                    let fn_children = arena.children_slice(fn_start, fn_len as u32);
                    let body_node = NodeId(fn_children[0]);
                    self.enter_scope();
                    for &p_id in &fn_children[1..] {
                        let p_node = *arena.node(NodeId(p_id));
                        let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                        let p_ty = if NodeId(p_node.rhs).is_some() && (p_node.rhs as usize) < arena.nodes.len() && arena.node(NodeId(p_node.rhs)).node_kind().is_type_spec() {
                            self.resolve_type_spec(arena, NodeId(p_node.rhs))
                        } else {
                            TypeId::ANY
                        };
                        self.define_var(&p_name, p_ty);
                    }
                    if body_node.is_some() {
                        self.synth(arena, body_node);
                    }
                    self.exit_scope();
                }
                ret_ty
            }

            NodeKind::RecordDecl | NodeKind::StructDecl => {
                if node.flags == 2 {
                    return self.register_unit_decl(arena, node_id, &node);
                }
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym).to_string();
                let fields = self.collect_decl_fields(arena, node_id, &name);
                self.struct_fields.insert(name.clone(), fields.clone());
                let ty = self.types.row_record_with_name(fields.clone(), true, None, Some(name.clone()));
                self.type_defs.insert(name.clone(), ty);
                self.types.register_nominal_name(ty, name.clone());
                self.register_struct_typedef_if_needed(arena, node_id, ty, &name, &fields);
                let data_fields: Vec<TypeId> = fields
                    .iter()
                    .filter(|(n, t)| n != "this[]" && n != "self[]" && !matches!(self.types.get(self.types.prune(*t)), TypeKind::Function { .. }))
                    .map(|(_, t)| *t)
                    .collect();
                let param_tys = if data_fields.is_empty() {
                    vec![TypeId::ANY]
                } else {
                    data_fields
                };
                let ctor = self.types.function_of(param_tys, ty);
                self.define_var(&name, ctor);
                let body = NodeId(node.rhs);
                if body.is_some() {
                    let old_enclosing = self.current_enclosing_type.take();
                    self.current_enclosing_type = Some(name.clone());
                    self.enter_scope();
                    self.define_var("self", ty);
                    self.define_var("this", ty);
                    self.synth(arena, body);
                    self.exit_scope();
                    self.current_enclosing_type = old_enclosing;
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
                self.types.register_nominal_name(ty, name.clone());
                self.type_defs.insert(name, ty);
                ty
            }

            NodeKind::ClassDecl | NodeKind::InterfaceDecl => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym).to_string();
                let ty = self.types.alloc(TypeKind::Primitive(name.clone()));
                self.types.register_nominal_name(ty, name.clone());
                self.type_defs.insert(name.clone(), ty);
                let fields = self.collect_decl_fields(arena, node_id, &name);
                self.struct_fields.insert(name.clone(), fields);
                let ctor = self.types.function_of(vec![TypeId::ANY], ty);
                self.define_var(&name, ctor);
                let body = NodeId(node.rhs);
                if body.is_some() {
                    let old_enclosing = self.current_enclosing_type.take();
                    self.current_enclosing_type = Some(name.clone());
                    self.enter_scope();
                    self.define_var("self", ty);
                    self.define_var("this", ty);
                    self.synth(arena, body);
                    self.exit_scope();
                    self.current_enclosing_type = old_enclosing;
                }
                ty
            }

            NodeKind::ExtendDecl => {
                let target_name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                let target_ty = self.type_defs.get(&target_name).copied().unwrap_or(TypeId::ANY);
                let ext_fields = self.collect_decl_fields(arena, node_id, &target_name);
                if !ext_fields.is_empty() {
                    for f in &ext_fields {
                        let ext_fn_name = format!("{}Ext_{}", target_name, f.0);
                        self.define_var(&ext_fn_name, f.1);
                    }
                    let entry = self.struct_fields.entry(target_name.clone()).or_default();
                    for f in ext_fields {
                        entry.push(f);
                    }
                }
                let body = NodeId(node.rhs);
                if body.is_some() {
                    let old_enclosing = self.current_enclosing_type.take();
                    self.current_enclosing_type = Some(target_name.clone());
                    self.enter_scope();
                    self.define_var("self", target_ty);
                    self.define_var("this", target_ty);
                    self.synth(arena, body);
                    self.exit_scope();
                    self.current_enclosing_type = old_enclosing;
                }
                TypeId::VOID
            }

            // --- Control Flow ---
            NodeKind::IfExpr => {
                let cond_node = NodeId(node.lhs);
                self.enter_scope();
                let old_clearance = self.current_scope_clearance;
                if let Some(elevated) = self.extract_clearance_elevation(arena, cond_node) {
                    self.current_scope_clearance = self.current_scope_clearance.max(elevated);
                }
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
                self.current_scope_clearance = old_clearance;
                self.exit_scope();
                if else_node.is_some() {
                    let t_else = self.synth(arena, else_node);
                    if t_then != TypeId::VOID && t_else != TypeId::VOID {
                        return self.types.unify_or_union_all(vec![t_then, t_else]);
                    }
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
                        TypeKind::Collection { element, .. } => self.types.prune(element),
                        TypeKind::Map { key, value } => self.types.tuple_of(vec![key, value]),
                        _ if pruned == TypeId::STR => TypeId::STR,
                        _ => {
                            let nominal_name = self.types.nominal_names.get(&pruned).cloned();
                            if let Some(nom) = nominal_name {
                                if let Some(fields) = self.struct_fields.get(&nom).cloned() {
                                    if let Some((_, for_fn_ty)) = fields.iter().find(|(n, _)| n == "op for" || n.starts_with("op for")) {
                                        let pruned_for = self.types.prune(*for_fn_ty);
                                        if let TypeKind::Function { ret, .. } = self.types.get(pruned_for) {
                                            self.types.prune(*ret)
                                        } else {
                                            TypeId::I64
                                        }
                                    } else {
                                        TypeId::I64
                                    }
                                } else {
                                    TypeId::I64
                                }
                            } else {
                                TypeId::I64
                            }
                        }
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
                if let Some(elevated) = self.extract_clearance_elevation(arena, cond_node) {
                    self.current_scope_clearance = self.current_scope_clearance.max(elevated);
                }
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

                if arena.node(callee_node).node_kind() == NodeKind::MemberExpr {
                    let member_n = *arena.node(callee_node);
                    let target_n = *arena.node(NodeId(member_n.lhs));
                    if target_n.node_kind() == NodeKind::IdentExpr {
                        let patch_sym = SymbolId(target_n.lhs);
                        if let Some(&(is_mut, lo, hi)) = self.patch_handles.get(&patch_sym) {
                            let method = arena.interner.resolve(SymbolId(member_n.rhs));
                            if method == "set_line" {
                                if !is_mut {
                                    self.diagnostics.push(AnalysisDiagnostic::error(
                                        "PATCH002",
                                        "Cannot mutate line through immutable patch binding; declare with 'var'".to_string(),
                                        arena.span(callee_node),
                                    ));
                                }
                                if children.len() > 1 {
                                    let arg0 = *arena.node(NodeId(children[1]));
                                    if arg0.node_kind() == NodeKind::IntLit {
                                        let line_val = arena.i64_pool.get(arg0.lhs as usize).copied().unwrap_or(0);
                                        if line_val < lo as i64 || line_val > hi as i64 {
                                            self.diagnostics.push(AnalysisDiagnostic::error(
                                                "PATCH001",
                                                format!("Line {} is outside permitted patch window {}..{}", line_val, lo, hi),
                                                arena.span(callee_node),
                                            ));
                                        }
                                    }
                                }
                            } else if method == "get_line" {
                                if children.len() > 1 {
                                    let arg0 = *arena.node(NodeId(children[1]));
                                    if arg0.node_kind() == NodeKind::IntLit {
                                        let line_val = arena.i64_pool.get(arg0.lhs as usize).copied().unwrap_or(0);
                                        if line_val < lo as i64 || line_val > hi as i64 {
                                            self.diagnostics.push(AnalysisDiagnostic::error(
                                                "PATCH001",
                                                format!("Line {} is outside permitted patch window {}..{}", line_val, lo, hi),
                                                arena.span(callee_node),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // If this is a homoiconic call-quote (`fn!(...)`, `fn!!(...)`, `fn!&(...)`),
                // symbolic arguments inside `children[1..]` may be unbound quoted AST symbols.
                let is_quoted_call = node.flags != 0;
                let prev_diag_len = self.diagnostics.len();
                let mut arg_tys = Vec::new();
                for &arg_id in &children[1..] {
                    arg_tys.push(self.synth(arena, NodeId(arg_id)));
                }
                if is_quoted_call {
                    self.diagnostics.truncate(prev_diag_len);
                    return TypeId::ANY;
                }

                let fn_name_opt = if arena.node(callee_node).node_kind() == NodeKind::IdentExpr {
                    Some(arena.interner.resolve(SymbolId(arena.node(callee_node).lhs)).to_string())
                } else {
                    None
                };

                if let Some(ref fn_name) = fn_name_opt {
                    if let Some(clr) = self.fn_clearance.get(fn_name) {
                        if clr.level > self.current_scope_clearance {
                            self.diagnostics.push(AnalysisDiagnostic::error(
                                "LVL001",
                                format!("Insufficient clearance: call to '{}' requires clearance lvl({}), but current scope has clearance lvl({})", fn_name, clr.level, self.current_scope_clearance),
                                arena.span(node_id),
                            ));
                        }
                    }
                    if let Some(&fn_decl_id) = self.fn_decls.get(fn_name) {
                        let fn_node = *arena.node(fn_decl_id);
                        let explicit_ret = arena.type_annotations.get(&fn_decl_id.0);
                        if explicit_ret.is_none() && !self.instantiating_fns.contains(fn_name) {
                            let fn_start = fn_node.rhs;
                            let fn_len = fn_node.flags as usize;
                            if fn_len > 0 {
                                let fn_children = arena.children_slice(fn_start, fn_len as u32);
                                let body_node = NodeId(fn_children[0]);
                                self.instantiating_fns.insert(fn_name.clone());
                                self.enter_scope();
                                for (idx, &p_id) in fn_children[1..].iter().enumerate() {
                                    let p_node = *arena.node(NodeId(p_id));
                                    let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                                    let p_ty = arg_tys.get(idx).copied().unwrap_or(TypeId::ANY);
                                    self.define_var(&p_name, p_ty);
                                }
                                let prev_diag = self.diagnostics.len();
                                let instantiated_ret = self.synth(arena, body_node);
                                self.diagnostics.truncate(prev_diag);
                                self.exit_scope();
                                self.instantiating_fns.remove(fn_name);
                                if instantiated_ret != TypeId::ANY && instantiated_ret != TypeId::VOID {
                                    self.inferred_node_types.insert(node_id.0, instantiated_ret);
                                    return instantiated_ret;
                                }
                            }
                        }
                    }
                }

                let res = match self.types.get(self.types.prune(callee_ty)).clone() {
                    TypeKind::Function { params, ret } => {
                        for (param_t, arg_t) in params.iter().zip(arg_tys.iter()) {
                            if let Err(_) = self.types.unify(*param_t, *arg_t) {
                                let widened = self.types.union_of(vec![*param_t, *arg_t]);
                                let _ = self.types.unify(*param_t, widened);
                            }
                        }
                        ret
                    }
                    TypeKind::RowRecord { .. } => callee_ty,
                    TypeKind::Adt { .. } => callee_ty,
                    TypeKind::Primitive(ref name) if self.type_defs.contains_key(name) || self.struct_fields.contains_key(name) => callee_ty,
                    _ => TypeId::ANY,
                };
                self.inferred_node_types.insert(node_id.0, res);
                res
            }

            // --- Member & Index Projections ---
            NodeKind::MemberExpr => {
                let target_node = NodeId(node.lhs);
                if node.flags == 2 || target_node.is_none() {
                    let member_name = arena.interner.resolve(SymbolId(node.rhs));
                    if let Some((adt_name, _)) = self.adt_variants.get(member_name) {
                        let res = self.type_defs.get(adt_name).copied().unwrap_or(TypeId::ANY);
                        self.inferred_node_types.insert(node_id.0, res);
                        return res;
                    } else {
                        self.diagnostics.push(AnalysisDiagnostic::error(
                            "ENUM001",
                            format!("Unresolved contextual enum variant '::{}'", member_name),
                            arena.span(node_id),
                        ));
                        return TypeId::ANY;
                    }
                }
                let target_ty = self.synth(arena, target_node);
                let member_name = arena.interner.resolve(SymbolId(node.rhs));
                let mut pruned = self.types.prune(target_ty);
                let is_safe_nav = node.flags == 1;

                if let TypeKind::SliceMap { key, value } = *self.types.get(pruned) {
                    match member_name {
                        "keys" => {
                            let res = self.types.strided_slice_of(key);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "values" => {
                            let res = self.types.strided_slice_of(value);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "invert" => {
                            let inv_map = self.types.slice_map_of(value, key);
                            let res = self.types.function_of(Vec::new(), inv_map);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "len" | "length" | "count" => {
                            self.inferred_node_types.insert(node_id.0, TypeId::I64);
                            return TypeId::I64;
                        }
                        _ => {}
                    }
                }

                if matches!(self.types.get(pruned), TypeKind::Collection { kind: crate::types::CollectionKind::Slice | crate::types::CollectionKind::SliceBuffer, .. }) {
                    if member_name == "map_view" {
                        let ret_map = self.types.slice_map_of(TypeId::ANY, TypeId::ANY);
                        let res = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], ret_map);
                        self.inferred_node_types.insert(node_id.0, res);
                        return res;
                    }
                }

                if let TypeKind::Collection { kind: crate::types::CollectionKind::SliceView, .. } = *self.types.get(pruned) {
                    if member_name == "len" || member_name == "length" || member_name == "count" {
                        self.inferred_node_types.insert(node_id.0, TypeId::I64);
                        return TypeId::I64;
                    }
                }

                if let TypeKind::Collection { kind: crate::types::CollectionKind::View, element } = *self.types.get(pruned) {
                    pruned = self.types.prune(element);
                }

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
                if let TypeKind::RowRecord { fields, is_open, .. } = self.types.get(pruned) {
                    if let Some((_, f_ty)) = fields.iter().find(|(k, _)| k == member_name) {
                        let base_res = *f_ty;
                        let res = if is_safe_nav && !matches!(self.types.get(self.types.prune(base_res)), TypeKind::Option(_)) {
                            self.types.option_of(base_res)
                        } else {
                            base_res
                        };
                        self.inferred_node_types.insert(node_id.0, res);
                        return res;
                    } else if *is_open {
                        self.inferred_node_types.insert(node_id.0, TypeId::ANY);
                        return TypeId::ANY;
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
                    if let Some(clr) = self.struct_member_clearance.get(&(type_name.clone(), member_name.to_string())) {
                        if clr.level > self.current_scope_clearance {
                            self.diagnostics.push(AnalysisDiagnostic::error(
                                "LVL001",
                                format!("Insufficient clearance: access to '{}' requires clearance lvl({}), but current scope has clearance lvl({})", member_name, clr.level, self.current_scope_clearance),
                                arena.span(node_id),
                            ));
                        }
                    }

                    if let Some(vis) = self.struct_member_visibility.get(&(type_name.clone(), member_name.to_string())) {
                        if *vis == pilot_ast::VisibilityTier::Private {
                            let is_accessible = self.current_enclosing_type.as_deref() == Some(type_name.as_str());
                            if !is_accessible {
                                self.diagnostics.push(AnalysisDiagnostic::error(
                                    "VIS001",
                                    format!("Member '{}' is private to '{}' and cannot be accessed from the current context", member_name, type_name),
                                    arena.span(node_id),
                                ));
                            }
                        }
                    }

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

                // 3. Collection & Map standard methods
                if let TypeKind::Collection { kind: _, element } = self.types.get(pruned).clone() {
                    let pruned_elem = self.types.prune(element);
                    match member_name {
                        "append" | "push" | "add" => {
                            let res = self.types.function_of(vec![element], TypeId::VOID);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "insert" => {
                            let res = self.types.function_of(vec![TypeId::I64, element], TypeId::VOID);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "extend" => {
                            let res = self.types.function_of(vec![pruned], TypeId::VOID);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "pop" => {
                            let res = self.types.function_of(vec![], pruned_elem);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "remove" => {
                            let res = self.types.function_of(vec![TypeId::I64], pruned_elem);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "contains" => {
                            let res = self.types.function_of(vec![element], TypeId::BOOL);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "index_of" => {
                            let res = self.types.function_of(vec![element], TypeId::I64);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "clear" => {
                            let res = self.types.function_of(vec![], TypeId::VOID);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "where" | "filter" => {
                            let pred_fn = self.types.function_of(vec![element], TypeId::BOOL);
                            let res = self.types.function_of(vec![pred_fn], pruned);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "limit" | "take" => {
                            let res = self.types.function_of(vec![TypeId::I64], pruned);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "skip" | "offset" => {
                            let res = self.types.function_of(vec![TypeId::I64], pruned);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "first" => {
                            let opt_elem = self.types.option_of(element);
                            let res = self.types.function_of(vec![], opt_elem);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "order" | "order_by" | "sort" => {
                            let res = self.types.function_of(vec![TypeId::ANY], pruned);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "select" | "map" => {
                            let ret_list = self.types.list_of(TypeId::ANY);
                            let res = self.types.function_of(vec![TypeId::ANY], ret_list);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "join" => {
                            let res = self.types.function_of(vec![TypeId::STR], TypeId::STR);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "reverse" => {
                            let res = self.types.function_of(vec![], TypeId::VOID);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "slice" => {
                            let res = self.types.function_of(vec![TypeId::I64, TypeId::I64], pruned);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "last" => {
                            let opt_elem = self.types.option_of(element);
                            let res = self.types.function_of(vec![], opt_elem);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        _ => {}
                    }
                }

                if let TypeKind::Map { key, value } = self.types.get(pruned).clone() {
                    match member_name {
                        "set" => {
                            let res = self.types.function_of(vec![key, value], TypeId::VOID);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "get" => {
                            let opt_val = self.types.option_of(value);
                            let res = self.types.function_of(vec![key], opt_val);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "contains_key" | "has_key" => {
                            let res = self.types.function_of(vec![key], TypeId::BOOL);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "keys" => {
                            let list_k = self.types.list_of(key);
                            let res = self.types.function_of(vec![], list_k);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "values" => {
                            let list_v = self.types.list_of(value);
                            let res = self.types.function_of(vec![], list_v);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "clear" => {
                            let res = self.types.function_of(vec![], TypeId::VOID);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        _ => {}
                    }
                }

                // String built-in methods
                if pruned == TypeId::STR || matches!(self.types.get(pruned), TypeKind::Primitive(p) if p == "str" || p == "string") {
                    let str_ret = if is_safe_nav {
                        self.types.option_of(TypeId::STR)
                    } else {
                        TypeId::STR
                    };
                    match member_name {
                        "split" => {
                            let str_list = self.types.list_of(TypeId::STR);
                            let res = self.types.function_of(vec![TypeId::STR], str_list);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "join" => {
                            let str_list = self.types.list_of(TypeId::STR);
                            let res = self.types.function_of(vec![str_list], str_ret);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "replace" | "Replace" => {
                            let res = self.types.function_of(vec![TypeId::STR, TypeId::STR], str_ret);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "trim" | "Trim" | "trim_start" | "trim_left" | "TrimStart" | "trim_end" | "trim_right" | "TrimEnd" | "to_upper" | "ToUpper" | "to_lower" | "ToLower" => {
                            let res = self.types.function_of(vec![], str_ret);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "starts_with" | "StartsWith" | "ends_with" | "EndsWith" | "contains" | "Contains" => {
                            let res = self.types.function_of(vec![TypeId::STR], TypeId::BOOL);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "index_of" | "IndexOf" | "last_index_of" | "LastIndexOf" => {
                            let res = self.types.function_of(vec![TypeId::STR], TypeId::I64);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "substring" | "Substring" | "slice" | "Slice" => {
                            let res = self.types.function_of(vec![TypeId::I64, TypeId::I64], str_ret);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "repeat" => {
                            let res = self.types.function_of(vec![TypeId::I64], str_ret);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "pad_start" | "PadStart" | "pad_end" | "PadEnd" => {
                            let res = self.types.function_of(vec![TypeId::I64, TypeId::STR], str_ret);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "is_digit" | "IsDigit" | "is_alpha" | "IsAlpha" | "is_alphanumeric" | "IsAlphanumeric" | "is_empty" | "IsEmpty" => {
                            let res = self.types.function_of(vec![], TypeId::BOOL);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "char_at" | "CharAt" => {
                            let res = self.types.function_of(vec![TypeId::I64], str_ret);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "chars" | "Chars" => {
                            let str_list = self.types.list_of(TypeId::STR);
                            let res = self.types.function_of(vec![], str_list);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "to_int" | "ToInt" => {
                            let res = self.types.function_of(vec![], TypeId::I64);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "to_float" | "ToFloat" => {
                            let res = self.types.function_of(vec![], TypeId::F64);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "to_string" | "to_str" | "ToString" => {
                            let res = self.types.function_of(vec![], str_ret);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "len" | "length" | "Length" => {
                            self.inferred_node_types.insert(node_id.0, TypeId::I64);
                            return TypeId::I64;
                        }
                        _ => {}
                    }
                }

                // Integer built-in methods
                if pruned == TypeId::I64 || pruned == TypeId::I32 || matches!(self.types.get(pruned), TypeKind::Primitive(p) if p == "int" || p == "i64" || p == "i32") {
                    match member_name {
                        "abs" | "Abs" => {
                            let res = self.types.function_of(vec![], TypeId::I64);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "clamp" | "Clamp" => {
                            let res = self.types.function_of(vec![TypeId::I64, TypeId::I64], TypeId::I64);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "min" | "Min" | "max" | "Max" => {
                            let res = self.types.function_of(vec![TypeId::I64], TypeId::I64);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "to_string" | "to_str" | "ToString" | "ToHex" | "to_hex" | "to_bin" => {
                            let res = self.types.function_of(vec![], TypeId::STR);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "to_float" | "ToFloat" => {
                            let res = self.types.function_of(vec![], TypeId::F64);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "BitLength" | "bit_length" => {
                            let res = self.types.function_of(vec![], TypeId::I64);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        _ => {}
                    }
                }

                // Float built-in methods
                if pruned == TypeId::F64 || pruned == TypeId::F32 || matches!(self.types.get(pruned), TypeKind::Primitive(p) if p == "float" || p == "f64" || p == "f32") {
                    match member_name {
                        "abs" | "Abs" | "round" | "Round" | "floor" | "Floor" | "ceil" | "Ceil" | "trunc" | "Trunc" | "sqrt" | "Sqrt" => {
                            let res = self.types.function_of(vec![], TypeId::F64);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "clamp" | "Clamp" => {
                            let res = self.types.function_of(vec![TypeId::F64, TypeId::F64], TypeId::F64);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "min" | "Min" | "max" | "Max" | "pow" | "Pow" => {
                            let res = self.types.function_of(vec![TypeId::F64], TypeId::F64);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "to_string" | "to_str" | "ToString" => {
                            let res = self.types.function_of(vec![], TypeId::STR);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "to_int" | "ToInt" => {
                            let res = self.types.function_of(vec![], TypeId::I64);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        _ => {}
                    }
                }

                // Bool built-in methods
                if pruned == TypeId::BOOL || matches!(self.types.get(pruned), TypeKind::Primitive(p) if p == "bool") {
                    match member_name {
                        "to_string" | "to_str" | "ToString" => {
                            let res = self.types.function_of(vec![], TypeId::STR);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        "toggle" | "Toggle" => {
                            let res = self.types.function_of(vec![], TypeId::BOOL);
                            self.inferred_node_types.insert(node_id.0, res);
                            return res;
                        }
                        _ => {}
                    }
                }

                // 4. Well-known metadata attributes & standard methods
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
                let idx_ty = if idx_node.is_some() {
                    self.synth(arena, idx_node)
                } else {
                    TypeId::ANY
                };

                let pruned = self.types.prune(target_ty);
                let res = match self.types.get(pruned).clone() {
                    TypeKind::SliceMap { value, .. } => value,
                    TypeKind::Collection { element, .. } => element,
                    TypeKind::Map { value, .. } => {
                        if value == TypeId::ANY {
                            TypeId::ANY
                        } else {
                            self.types.option_of(value)
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
                    TypeKind::RowRecord { fields, is_open, .. } => {
                        let field_name_opt = if idx_node.is_some() {
                            let i_node = *arena.node(idx_node);
                            match i_node.node_kind() {
                                NodeKind::StrLit | NodeKind::IdentExpr => {
                                    Some(arena.interner.resolve(SymbolId(i_node.lhs)).to_string())
                                }
                                NodeKind::KeywordLit => {
                                    arena.interner.resolve_keyword(pilot_intern::KeywordId(i_node.lhs))
                                        .map(|e| arena.interner.resolve(e.name).to_string())
                                }
                                _ => None,
                            }
                        } else {
                            None
                        };

                        if let Some(ref f_name) = field_name_opt {
                            if let Some((_, f_ty)) = fields.iter().find(|(k, _)| k == f_name) {
                                *f_ty
                            } else if let Some((_, idx_fn)) = fields.iter().find(|(k, _)| k == "this[]" || k == "self[]") {
                                self.invoke_indexer_fn(*idx_fn, idx_ty, node_id, arena)
                            } else if is_open {
                                TypeId::ANY
                            } else {
                                self.diagnostics.push(AnalysisDiagnostic::error(
                                    "REC001",
                                    format!("Row record has no field '{}'", f_name),
                                    arena.span(node_id),
                                ));
                                TypeId::ANY
                            }
                        } else if let Some((_, idx_fn)) = fields.iter().find(|(k, _)| k == "this[]" || k == "self[]") {
                            self.invoke_indexer_fn(*idx_fn, idx_ty, node_id, arena)
                        } else {
                            let val_types: Vec<TypeId> = fields.iter().map(|(_, t)| *t).collect();
                            if val_types.is_empty() {
                                TypeId::ANY
                            } else {
                                let union_ty = self.types.unify_or_union_all(val_types);
                                self.types.option_of(union_ty)
                            }
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
                    _ => {
                        let field_name_opt = if idx_node.is_some() {
                            let i_node = *arena.node(idx_node);
                            match i_node.node_kind() {
                                NodeKind::StrLit | NodeKind::IdentExpr => {
                                    Some(arena.interner.resolve(SymbolId(i_node.lhs)).to_string())
                                }
                                _ => None,
                            }
                        } else {
                            None
                        };

                        let mut struct_f_ty = None;
                        if let Some(ref f_name) = field_name_opt {
                            let struct_name = match self.types.get(pruned) {
                                TypeKind::Adt { name, .. } | TypeKind::Primitive(name) => Some(name.clone()),
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
                            if let Some(sname) = struct_name {
                                if let Some(fields) = self.struct_fields.get(&sname) {
                                    if let Some((_, f_ty)) = fields.iter().find(|(n, _)| n == f_name) {
                                        struct_f_ty = Some(*f_ty);
                                    }
                                }
                            }
                        }

                        if let Some(f_ty) = struct_f_ty {
                            f_ty
                        } else {
                            // Check for custom indexer: "this[]" or "self[]"
                            let indexer_fn_ty = match self.types.get(pruned) {
                                TypeKind::RowRecord { fields, .. } => {
                                    fields.iter().find(|(n, _)| n == "this[]" || n == "self[]").map(|(_, ty)| *ty)
                                }
                                TypeKind::Adt { name, .. } | TypeKind::Primitive(name) => {
                                    self.struct_fields.get(name).and_then(|fields| {
                                        fields.iter().find(|(n, _)| n == "this[]" || n == "self[]").map(|(_, ty)| *ty)
                                    })
                                }
                                _ => {
                                    let t_node = *arena.node(target_node);
                                    if t_node.node_kind() == NodeKind::IdentExpr {
                                        let ident_name = arena.interner.resolve(SymbolId(t_node.lhs));
                                        self.struct_fields.get(ident_name).and_then(|fields| {
                                            fields.iter().find(|(n, _)| n == "this[]" || n == "self[]").map(|(_, ty)| *ty)
                                        })
                                    } else {
                                        None
                                    }
                                }
                            };

                            if let Some(fn_ty) = indexer_fn_ty {
                                self.invoke_indexer_fn(fn_ty, idx_ty, node_id, arena)
                            } else {
                                TypeId::ANY
                            }
                        }
                    }
                };
                self.inferred_node_types.insert(node_id.0, res);
                res
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
                    let body_node = NodeId(node.rhs);
                    for &p_id in params {
                        let p_node = *arena.node(NodeId(p_id));
                        let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                        let p_ty = self.infer_param_type(arena, &p_node, body_node);
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

                let mut arm_tys = Vec::new();
                for &arm_id in &children[1..] {
                    let arm_ty = self.synth_match_arm(arena, NodeId(arm_id), scrutinee_ty);
                    if arm_ty != TypeId::VOID && arm_ty != TypeId::NEVER {
                        arm_tys.push(arm_ty);
                    }
                }
                if arm_tys.is_empty() {
                    TypeId::VOID
                } else {
                    self.types.unify_or_union_all(arm_tys)
                }
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

    fn invoke_indexer_fn(&mut self, fn_ty: TypeId, idx_ty: TypeId, node_id: NodeId, arena: &AstArena) -> TypeId {
        if let TypeKind::Function { params, ret } = self.types.get(self.types.prune(fn_ty)).clone() {
            let arg_tys = match self.types.get(self.types.prune(idx_ty)) {
                TypeKind::Tuple(elems) if params.len() > 1 && elems.len() == params.len() => elems.clone(),
                _ => vec![idx_ty],
            };

            if arg_tys.len() != params.len() {
                self.diagnostics.push(AnalysisDiagnostic::error(
                    "IDX001",
                    format!("Indexer expects {} argument(s), got {}", params.len(), arg_tys.len()),
                    arena.span(node_id),
                ));
            } else {
                for (arg_t, param_t) in arg_tys.iter().zip(params.iter()) {
                    if let Err(e) = self.types.unify(*arg_t, *param_t) {
                        self.diagnostics.push(AnalysisDiagnostic::error(
                            "TYPE001",
                            format!("Indexer argument type mismatch: {}", e),
                            arena.span(node_id),
                        ));
                    }
                }
            }
            ret
        } else {
            TypeId::ANY
        }
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
            NodeKind::MapExpr if node.flags == 1 => {
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

    /// Compile-time SQL query analyzer and RowRecord column property synthesizer.
    pub fn infer_sql_query_type(&mut self, query: &str) -> TypeId {
        let mut clean_lines = Vec::new();
        for line in query.lines() {
            let l = line.trim();
            if !l.starts_with("--") && !l.starts_with('#') {
                clean_lines.push(line);
            }
        }
        let cleaned = clean_lines.join(" ");
        let trimmed = cleaned.trim().trim_end_matches(';').trim();
        let upper = trimmed.to_ascii_uppercase();

        if upper.starts_with("SELECT") {
            let from_idx_opt = find_sql_keyword_outside_parens(&upper, "FROM");
            let select_part = if let Some(f_idx) = from_idx_opt {
                trimmed["SELECT".len()..f_idx].trim()
            } else {
                trimmed["SELECT".len()..].trim()
            };

            let targets = split_sql_outside_parens(select_part, ',');
            let mut fields: Vec<(String, TypeId)> = Vec::new();

            for raw_target in targets {
                let target = raw_target.trim();
                if target.is_empty() {
                    continue;
                }

                if target == "*" {
                    let row_rec = self.types.row_record_of(vec![], true, None);
                    return self.types.list_of(row_rec);
                }

                let (col_name, col_ty) = parse_sql_column_target(target);
                fields.push((col_name, col_ty));
            }

            fields.sort_by(|a, b| a.0.cmp(&b.0));
            fields.dedup_by(|a, b| a.0 == b.0);

            let row_rec = self.types.row_record_of(fields, true, None);
            return self.types.list_of(row_rec);
        }

        if upper.starts_with("INSERT") || upper.starts_with("UPDATE") || upper.starts_with("DELETE") {
            if let Some(ret_idx) = upper.find("RETURNING") {
                let ret_part = trimmed[ret_idx + "RETURNING".len()..].trim();
                let mut fields: Vec<(String, TypeId)> = Vec::new();
                for raw_t in split_sql_outside_parens(ret_part, ',') {
                    let t = raw_t.trim();
                    if !t.is_empty() {
                        let (cname, cty) = parse_sql_column_target(t);
                        fields.push((cname, cty));
                    }
                }
                fields.sort_by(|a, b| a.0.cmp(&b.0));
                fields.dedup_by(|a, b| a.0 == b.0);
                let row_rec = self.types.row_record_of(fields, true, None);
                return self.types.list_of(row_rec);
            }
            return TypeId::I64;
        }

        TypeId::ANY
    }
}

fn find_sql_keyword_outside_parens(s: &str, kw: &str) -> Option<usize> {
    let mut depth = 0;
    let bytes = s.as_bytes();
    let kw_bytes = kw.as_bytes();
    let kw_len = kw_bytes.len();
    let mut i = 0;

    while i + kw_len <= bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' if depth > 0 => depth -= 1,
            _ => {
                if depth == 0 && &bytes[i..i + kw_len] == kw_bytes {
                    let before_ok = i == 0 || bytes[i - 1].is_ascii_whitespace();
                    let after_ok = i + kw_len == bytes.len() || bytes[i + kw_len].is_ascii_whitespace();
                    if before_ok && after_ok {
                        return Some(i);
                    }
                }
            }
        }
        i += 1;
    }
    None
}

fn split_sql_outside_parens(s: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;

    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            d if d == delimiter && depth == 0 => {
                parts.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    if start < s.len() {
        parts.push(&s[start..]);
    }
    parts
}

fn parse_sql_column_target(target: &str) -> (String, TypeId) {
    let upper = target.to_ascii_uppercase();

    // 1. Extract expression and column name (checking `AS <alias>`)
    let (expr_part, col_name) = if let Some(as_idx) = upper.rfind(" AS ") {
        let expr = target[..as_idx].trim();
        let alias = target[as_idx + 4..].trim().trim_matches('\"').trim_matches('`').trim_matches('\'');
        (expr, alias.to_string())
    } else {
        let unaliased = if let Some(dot_idx) = target.rfind('.') {
            target[dot_idx + 1..].trim()
        } else {
            target.trim()
        };
        let cleaned = if unaliased.to_ascii_uppercase().starts_with("COUNT(") {
            "count".to_string()
        } else if unaliased.to_ascii_uppercase().starts_with("SUM(") {
            "sum".to_string()
        } else if unaliased.to_ascii_uppercase().starts_with("AVG(") {
            "avg".to_string()
        } else if unaliased.to_ascii_uppercase().starts_with("MIN(") {
            "min".to_string()
        } else if unaliased.to_ascii_uppercase().starts_with("MAX(") {
            "max".to_string()
        } else {
            let last_tok = unaliased.split_whitespace().last().unwrap_or(unaliased);
            last_tok.trim_matches('\"').trim_matches('`').trim_matches('\'').to_string()
        };
        (target, cleaned)
    };

    // 2. Infer column type
    let expr_upper = expr_part.to_ascii_uppercase();
    let col_lower = col_name.to_ascii_lowercase();

    let ty = if expr_upper.starts_with("COUNT(") || expr_upper == "COUNT(*)" {
        TypeId::I64
    } else if expr_upper.starts_with("AVG(") {
        TypeId::F64
    } else if expr_part.contains('.') && expr_part.chars().any(|c| c.is_ascii_digit()) {
        TypeId::F64
    } else if col_lower.contains("amount")
        || col_lower.contains("salary")
        || col_lower.contains("price")
        || col_lower.contains("cost")
        || col_lower.contains("balance")
        || col_lower.contains("total")
        || col_lower.contains("ratio")
        || col_lower.contains("rate")
        || col_lower.contains("percent")
        || col_lower.contains("spend")
        || col_lower.contains("revenue")
        || (col_lower.contains("score") && expr_part.contains('/'))
    {
        TypeId::F64
    } else if col_lower == "id"
        || col_lower.ends_with("_id")
        || col_lower.starts_with("id_")
        || col_lower.contains("count")
        || col_lower.contains("age")
        || col_lower.contains("qty")
        || col_lower.contains("quantity")
        || col_lower.contains("num")
        || (col_lower.contains("status") && !col_lower.contains("text"))
        || col_lower.contains("offset")
        || col_lower.contains("limit")
        || col_lower.contains("score")
    {
        TypeId::I64
    } else if col_lower.starts_with("is_")
        || col_lower.starts_with("has_")
        || col_lower == "active"
        || col_lower == "enabled"
        || col_lower == "valid"
    {
        TypeId::BOOL
    } else {
        TypeId::STR
    };

    (col_name, ty)
}

// =========================================================================================
// PHASE 3: O(1) TRAIT BOUND VERIFICATION, STATIC DELEGATION & AST FIELD-RENAMING REWRITER
// =========================================================================================

impl TypeInferenceEngine {
    /// Verifies in O(1) time via the `TypeDef::implemented_interfaces` bitset whether `type_id`
    /// satisfies `required_iface`. Emits a `LatticeContractMismatch` or `TYPE002` diagnostic if not.
    pub fn verify_trait_bound(
        &mut self,
        type_id: TypeId,
        required_iface: crate::types::InterfaceId,
        span: pilot_span::ByteSpan,
    ) -> bool {
        if self.types.implements_interface(type_id, required_iface) {
            return true;
        }
        let type_name = self.types.display(type_id);
        let iface_name = match required_iface {
            crate::types::InterfaceId::IANY_VALUE => "IAnyValue".to_string(),
            crate::types::InterfaceId::IVALUE => "IValue".to_string(),
            crate::types::InterfaceId::IREFERENCE => "IReference".to_string(),
            crate::types::InterfaceId::IOBJECT => "IObject".to_string(),
            crate::types::InterfaceId::IGUARD => "IGuard".to_string(),
            other => format!("Interface#{}", other.0),
        };
        self.diagnostics.push(AnalysisDiagnostic::error(
            "TYPE002",
            format!("Type `{}` does not satisfy required interface bound `{}`", type_name, iface_name),
            span,
        ));
        false
    }

    /// Resolves static delegation `BundleName.method(var ref self, ...)` inside an enclosing struct's
    /// Tier 1 override body using `TypeDef::static_delegates`.
    pub fn resolve_static_delegate(
        &self,
        enclosing_type: TypeId,
        bundle_or_iface: SymbolId,
        method_sym: SymbolId,
    ) -> Option<pilot_parser::FunctionId> {
        let td = self.types.type_defs.get(&enclosing_type)?;
        td.static_delegates.get(&(bundle_or_iface, method_sym)).copied()
    }

    /// Resolves a method call on `type_id` in O(1) time using the 4-tier `MethodTable`.
    pub fn resolve_method_tier(
        &self,
        type_id: TypeId,
        method_sym: SymbolId,
    ) -> Option<&crate::types::MethodEntry> {
        let td = self.types.type_defs.get(&type_id)?;
        td.method_table.get(&method_sym)
    }
}

/// Rewrites in-place all `.old_name` (`PrefixDot`) and `self.old_name` (`InfixDot`) field accesses
/// inside the AST subtree rooted at `node_id` according to `renamed_symbols`.
pub fn rewrite_self_field_accesses(
    arena: &mut AstArena,
    node_id: NodeId,
    renamed_symbols: &crate::types::FxHashMap<SymbolId, SymbolId>,
) {
    if node_id.is_none() || (node_id.0 as usize) >= arena.nodes.len() || renamed_symbols.is_empty() {
        return;
    }
    let node = arena.nodes[node_id.0 as usize];
    match node.node_kind() {
        NodeKind::MemberExpr => {
            let lhs_id = NodeId(node.lhs);
            let field_sym = SymbolId(node.rhs);
            if lhs_id.is_some() && (lhs_id.0 as usize) < arena.nodes.len() {
                let lhs_node = arena.nodes[lhs_id.0 as usize];
                if lhs_node.node_kind() == NodeKind::IdentExpr {
                    let recv_name = arena.interner.resolve(SymbolId(lhs_node.lhs));
                    if recv_name == "self" || recv_name == "this" {
                        if let Some(&new_sym) = renamed_symbols.get(&field_sym) {
                            arena.nodes[node_id.0 as usize].rhs = new_sym.0;
                        }
                    }
                }
            }
            rewrite_self_field_accesses(arena, lhs_id, renamed_symbols);
        }
        NodeKind::Program
        | NodeKind::BlockExpr
        | NodeKind::TupleExpr
        | NodeKind::ListExpr
        | NodeKind::MapExpr => {
            let children: Vec<u32> = arena.children_slice(node.lhs, node.rhs).to_vec();
            for c in children {
                rewrite_self_field_accesses(arena, NodeId(c), renamed_symbols);
            }
        }
        NodeKind::FnDecl | NodeKind::ConstructorDecl | NodeKind::DestructorDecl => {
            let fn_start = node.rhs;
            let fn_len = node.flags as usize;
            if fn_len > 0 {
                let children: Vec<u32> = arena.children_slice(fn_start, fn_len as u32).to_vec();
                for c in children {
                    rewrite_self_field_accesses(arena, NodeId(c), renamed_symbols);
                }
            } else if NodeId(node.rhs).is_some() {
                rewrite_self_field_accesses(arena, NodeId(node.rhs), renamed_symbols);
            }
        }
        NodeKind::IfExpr => {
            rewrite_self_field_accesses(arena, NodeId(node.lhs), renamed_symbols);
            if node.flags == 0 {
                rewrite_self_field_accesses(arena, NodeId(node.rhs), renamed_symbols);
            } else {
                let extra: Vec<u32> = arena.children_slice(node.rhs, 2).to_vec();
                for c in extra {
                    rewrite_self_field_accesses(arena, NodeId(c), renamed_symbols);
                }
            }
        }
        NodeKind::CallExpr => {
            rewrite_self_field_accesses(arena, NodeId(node.lhs), renamed_symbols);
            let arg_count = (node.flags & 0x0FFF) as u32;
            let args: Vec<u32> = arena.children_slice(node.rhs, arg_count).to_vec();
            for a in args {
                rewrite_self_field_accesses(arena, NodeId(a), renamed_symbols);
            }
        }
        NodeKind::CallArg | NodeKind::UnaryExpr | NodeKind::ReturnStmt | NodeKind::YieldStmt | NodeKind::ExprStmt | NodeKind::DeferStmt => {
            rewrite_self_field_accesses(arena, NodeId(node.lhs), renamed_symbols);
        }
        NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
            rewrite_self_field_accesses(arena, NodeId(node.rhs), renamed_symbols);
        }
        NodeKind::BinaryExpr | NodeKind::AssignStmt | NodeKind::WhileStmt | NodeKind::IndexExpr | NodeKind::PipeExpr | NodeKind::CoalesceExpr | NodeKind::RangeExpr => {
            rewrite_self_field_accesses(arena, NodeId(node.lhs), renamed_symbols);
            rewrite_self_field_accesses(arena, NodeId(node.rhs), renamed_symbols);
        }
        _ => {}
    }
}

/// Clones the AST subtree rooted at `node_id` in `arena` and rewrites all `.old_name` and
/// `self.old_name` field accesses to `new_name` according to `renamed_symbols`.
pub fn clone_and_rewrite_self_field_accesses(
    arena: &mut AstArena,
    node_id: NodeId,
    renamed_symbols: &crate::types::FxHashMap<SymbolId, SymbolId>,
) -> NodeId {
    let cloned = clone_ast_subtree(arena, node_id);
    rewrite_self_field_accesses(arena, cloned, renamed_symbols);
    cloned
}

pub fn clone_ast_subtree(arena: &mut AstArena, node_id: NodeId) -> NodeId {
    clone_ast_subtree_with_replacements(arena, node_id, &[])
}

pub fn clone_ast_subtree_with_replacements(
    arena: &mut AstArena,
    node_id: NodeId,
    replacements: &[(NodeId, NodeId)],
) -> NodeId {
    if node_id.is_none() || (node_id.0 as usize) >= arena.nodes.len() {
        return node_id;
    }
    for &(target, replacement) in replacements {
        if node_id == target {
            return clone_ast_subtree(arena, replacement);
        }
    }
    let node = arena.nodes[node_id.0 as usize];
    let span = arena.span(node_id);
    let kind = node.node_kind();

    let new_id = match kind {
        NodeKind::Program
        | NodeKind::BlockExpr
        | NodeKind::TupleExpr
        | NodeKind::ListExpr
        | NodeKind::MapExpr => {
            let children: Vec<u32> = arena.children_slice(node.lhs, node.rhs).to_vec();
            let mut cloned_children = Vec::with_capacity(children.len());
            for c in children {
                cloned_children.push(clone_ast_subtree_with_replacements(arena, NodeId(c), replacements));
            }
            arena.alloc_list_node(kind, node.flags, span, &cloned_children)
        }
        NodeKind::FnDecl | NodeKind::ConstructorDecl | NodeKind::DestructorDecl => {
            let fn_start = node.rhs;
            let fn_len = node.flags as usize;
            if fn_len > 0 {
                let children: Vec<u32> = arena.children_slice(fn_start, fn_len as u32).to_vec();
                let mut cloned_children = Vec::with_capacity(children.len());
                for c in children {
                    cloned_children.push(clone_ast_subtree_with_replacements(arena, NodeId(c), replacements));
                }
                let (start, len) = arena.alloc_child_slice(&cloned_children);
                arena.alloc_node(kind, len as u16, span, node.lhs, start)
            } else {
                let rhs_cloned = clone_ast_subtree_with_replacements(arena, NodeId(node.rhs), replacements);
                arena.alloc_node(kind, node.flags, span, node.lhs, rhs_cloned.0)
            }
        }
        NodeKind::IfExpr => {
            let cond_cloned = clone_ast_subtree_with_replacements(arena, NodeId(node.lhs), replacements);
            if node.flags == 0 {
                let then_cloned = clone_ast_subtree_with_replacements(arena, NodeId(node.rhs), replacements);
                arena.alloc_node(kind, 0, span, cond_cloned.0, then_cloned.0)
            } else {
                let extra: Vec<u32> = arena.children_slice(node.rhs, 2).to_vec();
                let mut cloned_extra = Vec::with_capacity(extra.len());
                for c in extra {
                    cloned_extra.push(clone_ast_subtree_with_replacements(arena, NodeId(c), replacements));
                }
                let (start, _) = arena.alloc_child_slice(&cloned_extra);
                arena.alloc_node(kind, node.flags, span, cond_cloned.0, start)
            }
        }
        NodeKind::CallExpr => {
            let callee_cloned = clone_ast_subtree_with_replacements(arena, NodeId(node.lhs), replacements);
            let arg_count = (node.flags & 0x0FFF) as u32;
            let args: Vec<u32> = arena.children_slice(node.rhs, arg_count).to_vec();
            let mut cloned_args = Vec::with_capacity(args.len());
            for a in args {
                cloned_args.push(clone_ast_subtree_with_replacements(arena, NodeId(a), replacements));
            }
            let (start, _) = arena.alloc_child_slice(&cloned_args);
            arena.alloc_node(kind, node.flags, span, callee_cloned.0, start)
        }
        NodeKind::MemberExpr => {
            let lhs_cloned = clone_ast_subtree_with_replacements(arena, NodeId(node.lhs), replacements);
            arena.alloc_node(kind, node.flags, span, lhs_cloned.0, node.rhs)
        }
        NodeKind::CallArg
        | NodeKind::UnaryExpr
        | NodeKind::ReturnStmt
        | NodeKind::YieldStmt
        | NodeKind::ExprStmt
        | NodeKind::DeferStmt => {
            let lhs_cloned = clone_ast_subtree_with_replacements(arena, NodeId(node.lhs), replacements);
            arena.alloc_node(kind, node.flags, span, lhs_cloned.0, node.rhs)
        }
        NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
            let rhs_cloned = clone_ast_subtree_with_replacements(arena, NodeId(node.rhs), replacements);
            arena.alloc_node(kind, node.flags, span, node.lhs, rhs_cloned.0)
        }
        NodeKind::BinaryExpr
        | NodeKind::AssignStmt
        | NodeKind::WhileStmt
        | NodeKind::IndexExpr
        | NodeKind::PipeExpr
        | NodeKind::CoalesceExpr
        | NodeKind::RangeExpr => {
            let lhs_cloned = clone_ast_subtree_with_replacements(arena, NodeId(node.lhs), replacements);
            let rhs_cloned = clone_ast_subtree_with_replacements(arena, NodeId(node.rhs), replacements);
            arena.alloc_node(kind, node.flags, span, lhs_cloned.0, rhs_cloned.0)
        }
        _ => arena.alloc_node(kind, node.flags, span, node.lhs, node.rhs),
    };

    if let Some(&ann) = arena.type_annotations.get(&node_id.0) {
        arena.type_annotations.insert(new_id.0, ann);
    }
    new_id
}
