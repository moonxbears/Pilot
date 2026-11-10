//! ============================================================================
//! Milestone 15 Verification Suite:
//! Native C++ Class, RAII, Vtable & Exception Boundary Adapter (`use cpp`)
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/vm_outcome.rs"]
mod vm_outcome;
#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 15,
    test_file_rel: "crates/pilot-analysis/tests/milestone15_cpp_class_raii_vtable_and_exceptions.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/adapter/cpp_mangle.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "mangle_itanium",
                "mangle_msvc_x64",
                "demangle_itanium",
                "demangle_msvc_x64",
            ],
            required_code_markers: &[
                "CppTypeKind",
                "CppType",
                "itanium_operator_code",
                "msvc_operator_code",
                "_ZN",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/adapter/cpp_abi.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "compute_cpp_class_layout",
                "resolve_cpp_method_overload",
                "is_non_trivial_cpp_type",
                "compute_vtable_slots",
            ],
            required_code_markers: &[
                "AbiPassMode",
                "CppAccess",
                "CppBaseSpec",
                "CppFieldDecl",
                "CppMethodDecl",
                "CppClassDecl",
                "CppVtableSlot",
                "CppClassLayout",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/adapter/cpp_parse.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "cpp_parse_translation_unit",
                "cpp_diag_code",
                "check_abstract_instantiation",
                "check_deleted_method_call",
                "check_member_access",
            ],
            required_code_markers: &[
                "CppDiagnosticKind",
                "CppEnumDecl",
                "CppTranslationUnit",
                "CPP001",
                "CPP002",
                "CPP005",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/std/polyglot/cpp.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "cpp_exception_what",
                "cpp_exception_type",
                "cpp_call_catching",
            ],
            required_code_markers: &[
                "CppException",
                "CppStringView",
                "CppSpan",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m15(deps: &[&str], script: &str) -> String {
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
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    vm_outcome::outcome(&mut vm)
}

// -----------------------------------------------------------------------------
// Test 1: Itanium & MSVC x64 C++ Name Mangling and Demangling (`cpp_mangle.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m15_1_itanium_and_msvc_name_mangling_and_demangling() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m15_1_itanium_and_msvc_name_mangling_and_demangling",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            // 1. Itanium namespace free function: pilot::engine::init()
            let it_init = mangle_itanium(["pilot", "engine"], "init", false, 0, []);
            if it_init != "_ZN5pilot6engine4initEv" { return 1; }

            // Demangle Itanium free function
            let dem_init = demangle_itanium(it_init);
            if dem_init != "pilot::engine::init()" { return 2; }

            // 2. Itanium const member method: pilot::Widget::draw() const
            let it_draw = mangle_itanium(["pilot", "Widget"], "draw", true, 0, []);
            if it_draw != "_ZNK5pilot6Widget4drawEv" { return 3; }

            let dem_draw = demangle_itanium(it_draw);
            if dem_draw != "pilot::Widget::draw() const" { return 4; }

            // 3. Itanium complete object constructor & destructor: pilot::Tracker
            let it_ctor = mangle_itanium(["pilot", "Tracker"], "__ctor", false, 1, []);
            if it_ctor != "_ZN5pilot7TrackerC1Ev" { return 5; }

            let it_dtor = mangle_itanium(["pilot", "Tracker"], "__dtor", false, 3, []);
            if it_dtor != "_ZN5pilot7TrackerD1Ev" { return 6; }

            let dem_dtor = demangle_itanium(it_dtor);
            if dem_dtor != "pilot::Tracker::~Tracker()" { return 7; }

            // 4. MSVC x64 free function: pilot::engine::init()
            let ret_void = cpp_type_primitive(CppTypeKind::Void);
            let msvc_init = mangle_msvc_x64(["pilot", "engine"], "init", 3, false, [], ret_void);
            if msvc_init != "?init@engine@pilot@@YAXXZ" { return 8; }

            let dem_msvc = demangle_msvc_x64(msvc_init);
            if dem_msvc != "pilot::engine::init()" { return 9; }

            // 5. MSVC x64 member method: pilot::Widget::draw()
            let msvc_draw = mangle_msvc_x64(["pilot", "Widget"], "draw", 0, false, [], ret_void);
            if msvc_draw != "?draw@Widget@pilot@@QEAAXXZ" { return 10; }

            return 42;
        }
    "#;

    let deps = &[
        "selfhost/adapter/cpp_mangle.pilot",
    ];
    let res = eval_m15(deps, script);
    assert_eq!(res, "42", "Itanium & MSVC x64 manglers and demanglers must correctly encode/decode names");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: C++ Class Layout, Vtable Slots & Non-Trivial ABI Passing (`cpp_abi.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m15_2_cpp_class_layout_vtable_and_invisible_ref_abi() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m15_2_cpp_class_layout_vtable_and_invisible_ref_abi",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            let target = TargetDescriptor {
                arch:             "x86_64",
                os:               "linux",
                pointer_bytes:    8,
                endian:           "little",
                backend:          "llvm",
                simd_width_bits:  256,
                cache_line_bytes: 64,
                opt_level:        2,
            };

            // 1. The Invisible Non-Trivial Struct ABI Rule:
            // Small 8-byte C++ struct with user destructor MUST be passed via HiddenRef!
            var raii_tracker = cpp_class_decl_new("Tracker", false);
            raii_tracker.has_user_dtor = true;
            raii_tracker.fields.push(cpp_field_decl_new("id", "int", 4, 4, CppAccess::Private));
            raii_tracker.fields.push(cpp_field_decl_new("mode", "int", 4, 4, CppAccess::Private));

            let raii_layout = compute_cpp_class_layout(raii_tracker, target);
            if raii_layout.total_size != 8 { return 1; }
            if raii_layout.pass_mode != AbiPassMode::HiddenRef { return 2; }

            // Trivial 8-byte struct without destructor passes in TrivialRegisters
            var trivial_pt = cpp_class_decl_new("Point", true);
            trivial_pt.fields.push(cpp_field_decl_new("x", "int", 4, 4, CppAccess::Public));
            trivial_pt.fields.push(cpp_field_decl_new("y", "int", 4, 4, CppAccess::Public));

            let pt_layout = compute_cpp_class_layout(trivial_pt, target);
            if pt_layout.total_size != 8 { return 3; }
            if pt_layout.pass_mode != AbiPassMode::TrivialRegisters { return 4; }

            // 2. Primary Vtable Layout with Itanium 2-Slot Virtual Destructor
            var cb_class = cpp_class_decl_new("ICallback", false);
            let v_dtor = cpp_method_decl_new("~ICallback", true, true, false, false, true, CppAccess::Public, "void");
            let v_event = cpp_method_decl_new("on_event", true, true, false, false, false, CppAccess::Public, "void");
            cb_class.methods.push(v_dtor);
            cb_class.methods.push(v_event);

            let cb_layout = compute_cpp_class_layout(cb_class, target);
            if !cb_layout.has_vptr { return 5; }
            if cb_layout.vptr_offset != 0 { return 6; }
            if cb_layout.vtable_slots.len() != 3 { return 7; }
            // Slot 0: Complete object dtor
            if cb_layout.vtable_slots[0].slot_index != 0 || cb_layout.vtable_slots[0].dtor_variant != 1 { return 8; }
            // Slot 1: Deleting dtor
            if cb_layout.vtable_slots[1].slot_index != 1 || cb_layout.vtable_slots[1].dtor_variant != 2 { return 9; }
            // Slot 2: on_event
            if cb_layout.vtable_slots[2].slot_index != 2 || cb_layout.vtable_slots[2].method_name != "on_event" { return 10; }

            // 3. Overload Resolution
            var m_int = cpp_method_decl_new("process", false, false, false, false, false, CppAccess::Public, "void");
            m_int.param_type_names.push("int");
            var m_dbl = cpp_method_decl_new("process", false, false, false, false, false, CppAccess::Public, "void");
            m_dbl.param_type_names.push("double");

            var methods: List<CppMethodDecl> = [m_int, m_dbl];
            let res_int = resolve_cpp_method_overload(methods, "process", ["int"]);
            if !res_int.is_ok || res_int.winner_idx != 0 { return 11; }

            let res_dbl = resolve_cpp_method_overload(methods, "process", ["double"]);
            if !res_dbl.is_ok || res_dbl.winner_idx != 1 { return 12; }

            return 42;
        }
    "#;

    let deps = &[
        "selfhost/adapter/cpp_mangle.pilot",
        "selfhost/adapter/cpp_abi.pilot",
    ];
    let res = eval_m15(deps, script);
    assert_eq!(res, "42", "C++ class layout, vtable slot assignment, and invisible-ref passing must be exact");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Pure-Pilot C++ Header Parser (`cpp_parse.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m15_3_cpp_header_parser_classes_virtuals_and_string_view() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m15_3_cpp_header_parser_classes_virtuals_and_string_view",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            let header = "namespace pilot {\n"
                + "namespace engine {\n\n"
                + "class ICallback {\n"
                + "public:\n"
                + "    virtual ~ICallback() = 0;\n"
                + "    virtual void on_event(int event_id) = 0;\n"
                + "};\n\n"
                + "class Tracker {\n"
                + "private:\n"
                + "    int id;\n"
                + "    void* handle;\n"
                + "public:\n"
                + "    Tracker(int track_id);\n"
                + "    ~Tracker();\n"
                + "    int get_id() const noexcept;\n"
                + "    void process(std::string_view msg);\n"
                + "};\n\n"
                + "enum class EngineState : uint32_t {\n"
                + "    Stopped = 0,\n"
                + "    Running = 1,\n"
                + "    Paused = 2\n"
                + "};\n\n"
                + "}\n"
                + "}\n";

            var diags: List<Diagnostic> = [];
            let tu = cpp_parse_translation_unit(header, "test.hpp", var diags);

            // Namespaces
            if tu.namespaces.len() != 2 { return 1; }
            if tu.namespaces[0] != "pilot" || tu.namespaces[1] != "engine" { return 2; }

            // Classes
            if tu.classes.len() != 2 { return 3; }

            // ICallback
            let cb = tu.classes[0];
            if cb.name != "ICallback" { return 4; }
            if !cb.is_abstract { return 5; }
            if cb.methods.len() != 2 { return 6; }
            if !cb.methods[0].is_pure_virtual || !cb.methods[1].is_pure_virtual { return 7; }

            // Tracker
            let trk = tu.classes[1];
            if trk.name != "Tracker" { return 8; }
            if !trk.has_user_dtor { return 9; }
            if trk.fields.len() != 2 { return 10; }
            if trk.fields[0].byte_size != 4 || trk.fields[1].byte_size != 8 { return 11; }
            if trk.methods.len() != 4 { return 12; }

            let get_id_m = trk.methods[2];
            if get_id_m.name != "get_id" || !get_id_m.is_const || !get_id_m.is_noexcept { return 13; }

            let proc_m = trk.methods[3];
            if proc_m.name != "process" || proc_m.param_type_names.len() != 1 || proc_m.param_type_names[0] != "std::string_view" { return 14; }

            // Enum class EngineState
            if tu.enums.len() != 1 { return 15; }
            let en = tu.enums[0];
            if en.name != "EngineState" || !en.is_scoped { return 16; }
            if en.variants.len() != 3 { return 17; }
            if en.variants[0].value != 0 || en.variants[1].value != 1 || en.variants[2].value != 2 { return 18; }

            return 42;
        }
    "#;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/cpp_mangle.pilot",
        "selfhost/adapter/cpp_abi.pilot",
        "selfhost/adapter/cpp_parse.pilot",
    ];
    let res = eval_m15(deps, script);
    assert_eq!(res, "42", "C++ header parser must extract namespaces, classes, pure virtuals, noexcept, and enum classes");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Diagnostics CPP001–CPP006 & Safe Exception Boundary (`std.polyglot.cpp`)
// -----------------------------------------------------------------------------
#[test]
fn test_m15_4_cpp_diagnostics_and_safe_exception_boundary() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m15_4_cpp_diagnostics_and_safe_exception_boundary",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            var diags: List<Diagnostic> = [];

            // 1. Diagnostic CPP005: Abstract class instantiation
            var abs_cls = cpp_class_decl_new("ICallback", false);
            abs_cls.is_abstract = true;
            let can_inst = check_abstract_instantiation(abs_cls, var diags);
            if can_inst { return 1; }
            if diags.len() != 1 || diags[0].code != "CPP005" { return 2; }

            // 2. Diagnostic CPP001: Deleted member invocation
            var del_m = cpp_method_decl_new("operator=", false, false, false, false, false, CppAccess::Public, "void");
            del_m.is_deleted = true;
            let can_call_del = check_deleted_method_call(del_m, var diags);
            if can_call_del { return 3; }
            if diags.len() != 2 || diags[1].code != "CPP001" { return 4; }

            // 3. Diagnostic CPP002: Inaccessible member access
            let can_access = check_member_access(CppAccess::Private, false, var diags);
            if can_access { return 5; }
            if diags.len() != 3 || diags[2].code != "CPP002" { return 6; }

            // 4. Safe Exception Boundary & CppException recovery
            let exc = CppException::create("connection refused", "std::runtime_error");
            if cpp_exception_what(exc) != "connection refused" { return 7; }
            if cpp_exception_type(exc) != "std::runtime_error" { return 8; }

            let failed = CppCatchResult.fail(exc);
            if failed.is_ok || failed.err_val.to_string() != "std::runtime_error: connection refused" { return 9; }

            return 42;
        }
    "#;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/cpp_mangle.pilot",
        "selfhost/adapter/cpp_abi.pilot",
        "selfhost/adapter/cpp_parse.pilot",
        "stdlib/std/polyglot/cpp.pilot",
    ];
    let res = eval_m15(deps, script);
    assert_eq!(res, "42", "C++ diagnostic checks and exception values must function correctly");

    // Calling into C++ and viewing Pilot strings as std::string_view need
    // native calls and raw memory views.
    let cases = [
        ("let r = cpp_call_catching(4096, [1, 2]);", "cpp.catch_boundary"),
        ("let sv = CppStringView::from_str(\"hello cpp\");", "cpp.string_view_from_str"),
    ];
    for (body, seam_id) in cases {
        let script = format!("pub fn main() -> i64 {{ {body} return 42; }}");
        let res = eval_m15(&["stdlib/std/polyglot/cpp.pilot"], &script);
        assert!(
            res.starts_with(&format!("FAULT: SEAM001: unconnected seam `{seam_id}`")),
            "`{body}` must stop at seam {seam_id}, got {res}"
        );
    }

    guard.finish();
}
