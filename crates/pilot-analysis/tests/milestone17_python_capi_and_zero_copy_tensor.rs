//! ============================================================================
//! Milestone 17 Verification Suite:
//! In-Process Python C-API Adapter & Zero-Copy Tensor Views (`use py`)
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 17,
    test_file_rel: "crates/pilot-analysis/tests/milestone17_python_capi_and_zero_copy_tensor.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/adapter/py_stub.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "py_parse_stub",
                "py_classify_type",
                "py_diag_code",
                "py_synthesize_vectorcall_wrapper",
                "py_module_stub_new",
            ],
            required_code_markers: &[
                "PyStubItemKind",
                "PyStubTypeKind",
                "PyParamDecl",
                "PyMethodDecl",
                "PyClassDecl",
                "PyModuleStub",
                "PY001",
                "PY002",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/std/polyglot/py_capi.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "py_gil_acquire",
                "py_gil_release",
                "py_allow_threads",
                "py_restore_threads",
                "py_obj_new",
                "py_obj_inc_ref",
                "py_obj_dec_ref",
                "py_buffer_acquire",
                "py_buffer_release",
                "py_vectorcall_fast",
                "py_err_fetch",
            ],
            required_code_markers: &[
                "PyBufferRaw",
                "PyGilGuard",
                "PyAllowThreadsGuard",
                "PyObj",
                "PyBufferLease",
                "PyError",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ir/pass_gil_coalesce.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "pass_coalesce_gil_scopes",
                "analyze_gil_hot_regions",
                "can_coalesce_scopes",
                "gil_scope_new",
                "gil_coalesce_report_new",
            ],
            required_code_markers: &[
                "GilScopeKind",
                "GilScope",
                "GilCoalesceReport",
                "AcquireGil",
                "AllowThreads",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m17(deps: &[&str], script: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let mut combined = String::new();
    for rel in deps {
        let p = root.join(rel);
        let src = fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("read {}: {:?}", p.display(), e));
        combined.push_str(&src);
        combined.push('\n');
    }
    combined.push_str(script);
    let arena = parse(&combined);
    let (flat, syms) = pilot_analysis::flatten_ast(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    let mut phy = PhysicsRegistry::new(flat, syms);
    let reg_ptr = &mut phy as *mut PhysicsRegistry;
    vm.engine_handler = Some(Box::new(move |op, a| {
        pilot_analysis::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, a)
    }));
    let bin = BytecodeCompiler::compile(&arena, arena.root);
    vm.load_binary(bin);
    vm.run().to_string_repr()
}

// -----------------------------------------------------------------------------
// Test 1: Python .pyi Stub Parser & Inspect Signatures (`py_stub.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m17_1_py_stub_parser_and_inspect_signatures() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m17_1_py_stub_parser_and_inspect_signatures",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            let pyi_src = "class TensorModel(BaseEstimator):\n"
                + "    @property\n"
                + "    def weights(self) -> NDArray[np.float64]: ...\n"
                + "    @staticmethod\n"
                + "    def create(dim: int = 128) -> TensorModel: ...\n"
                + "    def predict(self, x: NDArray[np.float64], batch_size: int = 32, *args, **kwargs) -> int | None: ...\n"
                + "DEFAULT_LR: float = 0.001\n"
                + "def train_step(data: list[float], lr: float = 0.01) -> bool: ...\n";

            var diags: List<Diagnostic> = [];
            let module = py_parse_stub(pyi_src, var diags);

            // 1. Verify Classes
            if module.classes.len() != 1 { return 1; }
            let cls = module.classes[0];
            if cls.name != "TensorModel" { return 2; }
            if cls.bases.len() != 1 { return 3; }
            if cls.bases[0] != "BaseEstimator" { return 4; }

            // 2. Verify Methods
            if cls.methods.len() != 3 { return 5; }
            let m0 = cls.methods[0];
            if m0.name != "weights" { return 6; }
            if !m0.is_property { return 7; }
            if m0.ret_kind != PyStubTypeKind::StrideTensor { return 8; }

            let m1 = cls.methods[1];
            if m1.name != "create" { return 9; }
            if !m1.is_static { return 10; }
            if m1.params.len() != 1 { return 11; }
            if m1.params[0].name != "dim" { return 12; }
            if !m1.params[0].has_default { return 13; }

            let m2 = cls.methods[2];
            if m2.name != "predict" { return 14; }
            if m2.params.len() != 4 { return 15; }
            if m2.params[0].name != "x" { return 16; }
            if m2.params[0].type_kind != PyStubTypeKind::StrideTensor { return 17; }
            if !m2.params[2].is_vararg { return 18; }
            if !m2.params[3].is_kwarg { return 19; }
            if m2.ret_kind != PyStubTypeKind::OptUnion { return 20; }

            // 3. Verify Functions & Constants
            if module.functions.len() != 1 { return 21; }
            if module.functions[0].name != "train_step" { return 22; }
            if module.functions[0].ret_kind != PyStubTypeKind::Bool { return 23; }

            if module.constants.len() != 1 { return 24; }
            if module.constants[0].name != "DEFAULT_LR" { return 25; }
            if module.constants[0].type_kind != PyStubTypeKind::Float { return 26; }

            // 4. Type classification
            if py_classify_type("int") != PyStubTypeKind::Int { return 27; }
            if py_classify_type("float") != PyStubTypeKind::Float { return 28; }
            if py_classify_type("str") != PyStubTypeKind::Str { return 29; }
            if py_classify_type("Optional[int]") != PyStubTypeKind::OptUnion { return 30; }
            if py_classify_type("list[str]") != PyStubTypeKind::List { return 31; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/py_stub.pilot",
        "stdlib/std/polyglot/py_capi.pilot",
    ];
    let res = eval_m17(deps, script);
    assert_eq!(res, "42", "Python stub parser and inspect signatures must pass");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Zero-Copy PEP 3118 Buffer & Strided Tensor Views (`py_capi.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m17_2_zero_copy_pep3118_buffer_and_strided_tensor_views() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m17_2_zero_copy_pep3118_buffer_and_strided_tensor_views",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // 1. Exact PEP 3118 PyBufferRaw Layout
            let raw_buf = py_buffer_raw_new(
                0x7fff0000, // buf virtual address
                0x7fff1000, // obj owner ptr
                8000,       // total len (8 KB)
                8,          // itemsize (8 bytes for f64)
                0,          // readonly = false (mutable)
                1,          // ndim = 1
                "d",        // format = 'd' (f64)
                1000,       // shape = 1000 elements
                8           // strides = 8 bytes contiguous
            );

            if raw_buf.buf != 0x7fff0000 { return 1; }
            if raw_buf.itemsize != 8 { return 2; }
            if raw_buf.readonly != 0 { return 3; }
            if raw_buf.format != "d" { return 4; }
            if raw_buf.strides != 8 { return 5; }

            // 2. Strided buffer lease (IGuard 0b11)
            let obj = py_obj_new(0x7fff0000);
            var lease = py_buffer_acquire(ref obj, 0);
            if !lease.is_active { return 6; }
            if lease.raw.itemsize != 8 { return 7; }

            py_buffer_release(var lease);
            if lease.is_active { return 8; }

            // 3. Diagnostics
            if py_diag_code(PyDiagnosticKind::LibPythonNotFound) != "PY001" { return 9; }
            if py_diag_code(PyDiagnosticKind::BufferFormatMismatch) != "PY002" { return 10; }
            if py_diag_code(PyDiagnosticKind::ReadOnlyMutBorrow) != "PY003" { return 11; }
            if py_diag_code(PyDiagnosticKind::SubByteStrided) != "PY004" { return 12; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/py_stub.pilot",
        "stdlib/std/polyglot/py_capi.pilot",
    ];
    let res = eval_m17(deps, script);
    assert_eq!(res, "42", "PEP 3118 buffer and zero-copy strided tensor views must pass");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Vectorcall Fast-Path & GIL Guards (`py_capi.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m17_3_vectorcall_fast_path_and_gil_guards() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m17_3_vectorcall_fast_path_and_gil_guards",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // 1. GIL Guard (0b11) acquisition and release
            var gil = py_gil_acquire();
            if !gil.is_active { return 1; }
            if gil.state != 1 { return 2; }

            py_gil_release(var gil);
            if gil.is_active { return 3; }
            if gil.state != 0 { return 4; }

            // 2. AllowThreads Guard (0b11) for pure compute loops
            var allow_th = py_allow_threads();
            if !allow_th.is_released { return 5; }

            py_restore_threads(var allow_th);
            if allow_th.is_released { return 6; }

            // 3. PyObj (0b10) Perceus refcounting
            var obj = py_obj_new(0x2000);
            if obj.refcount != 1 { return 7; }
            py_obj_inc_ref(ref obj);
            py_obj_dec_ref(var obj);

            // 4. Vectorcall fast-path with kwnames
            let mod_obj = py_import_module("numpy");
            if mod_obj.ptr == 0 { return 8; }

            var args: List<usize> = [100, 200];
            var kwnames: List<str> = ["batch_size", "verbose"];
            let call_res = py_vectorcall_fast(ref mod_obj, args.slice(0, args.len()), kwnames.slice(0, kwnames.len()));
            if !call_res.is_ok { return 9; }
            if call_res.val.ptr == 0 { return 10; }

            // 5. Error checking
            let null_obj = py_obj_new(0);
            let err_call = py_vectorcall_fast(ref null_obj, args.slice(0, 0), kwnames.slice(0, 0));
            if err_call.is_ok { return 11; }
            if err_call.err.exc_type != "TypeError" { return 12; }

            // 6. Vectorcall wrapper synthesizer
            var stub = py_module_stub_new("test_mod");
            let wrapper = py_synthesize_vectorcall_wrapper(ref stub, "predict");
            if !wrapper.contains("py_vectorcall_fast") { return 13; }
            if !wrapper.contains("py_gil_acquire") { return 14; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/py_stub.pilot",
        "stdlib/std/polyglot/py_capi.pilot",
    ];
    let res = eval_m17(deps, script);
    assert_eq!(res, "42", "Vectorcall fast-path and GIL guards must pass");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: PilIR GIL Coalesce Pass & Pure Compute Insertion (`pass_gil_coalesce.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m17_4_gil_coalesce_ir_pass_and_diagnostics() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m17_4_gil_coalesce_ir_pass_and_diagnostics",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // 1. Create simulated block scopes
            var s1 = gil_scope_new(1, GilScopeKind::AcquireGil, 0, 5, 10);
            var s2 = gil_scope_new(2, GilScopeKind::AcquireGil, 10, 15, 10);
            var s3 = gil_scope_new(3, GilScopeKind::AcquireGil, 40, 45, 10);
            var s_diff = gil_scope_new(4, GilScopeKind::AcquireGil, 50, 55, 20);

            // 2. Coalescing predicates
            if !can_coalesce_scopes(ref s1, ref s2) { return 1; } // within block 10, gap <= 16
            if can_coalesce_scopes(ref s1, ref s_diff) { return 2; } // different block!

            // 3. Main Pass execution
            var scopes: List<GilScope> = [s1, s2, s3, s_diff];
            let report = pass_coalesce_gil_scopes(var scopes);

            if report.total_scopes_before != 4 { return 3; }
            if report.merged_count != 1 { return 4; } // s1 and s2 merged
            if report.allow_threads_inserted != 1 { return 5; } // gap s2 -> s3 is 25 (> 20)
            if report.total_scopes_after != 3 { return 6; }

            // 4. Hot regions analyzer
            var blocks: List<u32> = [1, 2];
            var inst_counts: List<u32> = [25, 15];
            let hot_scopes = analyze_gil_hot_regions(blocks.slice(0, 2), inst_counts.slice(0, 2));
            if hot_scopes.len() == 0 { return 7; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/py_stub.pilot",
        "stdlib/std/polyglot/py_capi.pilot",
        "selfhost/ir/pass_gil_coalesce.pilot",
    ];
    let res = eval_m17(deps, script);
    assert_eq!(res, "42", "PilIR GIL coalesce pass and diagnostics must pass");

    guard.finish();
}
