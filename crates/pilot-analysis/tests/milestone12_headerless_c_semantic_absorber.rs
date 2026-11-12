//! ============================================================================
//! Milestone 12 Verification Suite:
//! Headerless C11/C23 Semantic Absorber (`use c "header.h"`)
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
    milestone_id: 12,
    test_file_rel: "crates/pilot-analysis/tests/milestone12_headerless_c_semantic_absorber.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/adapter/c_pp.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "cpp_env_new",
                "cpp_define_macro",
                "cpp_undef_macro",
                "cpp_is_defined",
                "cpp_eval_expr",
                "preprocess_c_header",
            ],
            required_code_markers: &[
                "__STDC__",
                "__STDC_VERSION__",
                "__x86_64__",
                "defined",
                "pragma",
                "once",
                "CAB003",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/adapter/c_parser.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "c_parse_translation_unit",
                "c_translation_unit_new",
                "c_type_size_and_align",
            ],
            required_code_markers: &[
                "CBitfieldSpec",
                "CFieldDecl",
                "CRecordDecl",
                "CFnDecl",
                "CEnumDecl",
                "CTranslationUnit",
                "CAB004",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/adapter/c_sysroot.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "sysroot_discovery_new",
                "sysroot_add_standard_paths",
                "parse_pkg_config_file",
                "resolve_c_header_path",
            ],
            required_code_markers: &[
                "SysrootDiscovery",
                "parse_pkg_config_file",
                "resolve_c_header_path",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ffi_bindgen.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "generate_ffi_binding",
                "generate_ffi_binding_multi",
                "generate_arena_c_ffi_header",
            ],
            required_code_markers: &[
                "FfiBindingBundle",
                "FfiParamSpec",
                "generate_ffi_binding_multi",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m12(deps: &[&str], script: &str) -> String {
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
// Test 1: C Preprocessor Conditionals, Integer Math & Directives (`c_pp.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m12_1_c_preprocessor_includes_macros_and_integer_eval() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m12_1_c_preprocessor_includes_macros_and_integer_eval",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            var env = cpp_env_new("linux", "x86_64");
            if !cpp_is_defined(env, "__STDC__") { return 1; }
            if !cpp_is_defined(env, "__x86_64__") { return 2; }
            if cpp_get_macro(env, "__STDC_VERSION__") != "201710L" { return 3; }
            if !cpp_is_defined(env, "__LP64__") || !cpp_is_defined(env, "__linux__") { return 30; }

            // Target macros follow the target: Apple ARM64 is LP64 without
            // x86 macros; Windows x64 is LLP64 (no __LP64__) and defines _WIN64.
            let mac = cpp_env_new("macos", "aarch64");
            if !cpp_is_defined(mac, "__aarch64__") || !cpp_is_defined(mac, "__APPLE__") { return 31; }
            if cpp_is_defined(mac, "__x86_64__") || cpp_is_defined(mac, "__linux__") { return 32; }
            let win = cpp_env_new("windows", "x86_64");
            if !cpp_is_defined(win, "_WIN64") || !cpp_is_defined(win, "_WIN32") { return 33; }
            if cpp_is_defined(win, "__LP64__") || cpp_is_defined(win, "__unix__") { return 34; }

            // CAB001: an #include no include path resolves.
            var miss = cpp_env_new("linux", "x86_64");
            let _ = preprocess_c_header("#include \"no_such_header_m12.h\"\n", "m.h", [], var miss);
            if cpp_error_code(miss) != "CAB001" { return 35; }

            cpp_define_macro(var env, "BUFFER_SIZE", "1024");
            if !cpp_is_defined(env, "BUFFER_SIZE") { return 4; }
            if cpp_get_macro(env, "BUFFER_SIZE") != "1024" { return 5; }

            // Test #if expression evaluation
            var pos: usize = 0;
            let val1 = cpp_eval_expr("1 + 2 * 3", var pos, env);
            if val1 != 7 { return 6; }

            pos = 0;
            let val2 = cpp_eval_expr("(10 >> 1) + (1 << 4)", var pos, env);
            if val2 != 21 { return 7; }

            pos = 0;
            let val3 = cpp_eval_expr("defined(BUFFER_SIZE) && BUFFER_SIZE > 512", var pos, env);
            if val3 != 1 { return 8; }

            pos = 0;
            let val4 = cpp_eval_expr("defined(UNKNOWN_MACRO)", var pos, env);
            if val4 != 0 { return 9; }

            // Test preprocessing text with #ifdef and #define
            let c_src = "#define FOO 42\n#ifdef FOO\nint a = FOO;\n#else\nint a = 0;\n#endif\n";
            let processed = preprocess_c_header(c_src, "test.h", [], var env);
            if !processed.contains("int a = 42;") { return 10; }
            if processed.contains("int a = 0;") { return 11; }

            // Test CAB003 #error directive
            let err_src = "#error Target unsupported\n";
            let _ = preprocess_c_header(err_src, "error.h", [], var env);
            if !cpp_has_error(env) { return 12; }
            if cpp_error_code(env) != "CAB003" { return 13; }

            // Test function-like macro expansion with parameter substitution, stringification and token pasting
            let fn_macro_src = "#define STR(x) #x\n"
                + "#define CONCAT(a, b) a ## b\n"
                + "#define SQR(x) ((x) * (x))\n"
                + "char* s = STR(hello_world);\n"
                + "int CONCAT(my_, var) = SQR(3);\n"
                + "int sqr_x = SQR(x + 1);\n";
            let fn_processed = preprocess_c_header(fn_macro_src, "macros.h", [], var env);
            if !fn_processed.contains("char* s = \"hello_world\";") { return 14; }
            if !fn_processed.contains("int my_var = ((3) * (3));") { return 15; }
            if !fn_processed.contains("int sqr_x = ((x + 1) * (x + 1));") { return 16; }

            return 42;
        }
    "##;

    let deps = &[
        "stdlib/core/file.pilot",
        "stdlib/std/text.pilot",
        "selfhost/adapter/c_pp.pilot",
    ];
    let res = eval_m12(deps, script);
    assert_eq!(res, "42", "C preprocessor must handle standard macros, arithmetic #if evaluation, and #error directives");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: C11 Declarative Parser Structs, Unions, Bitfields & Enums (`c_parser.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m12_2_c11_declarative_parser_structs_unions_bitfields_and_enums() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m12_2_c11_declarative_parser_structs_unions_bitfields_and_enums",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            let c_code = "typedef struct Point {\n"
                + "    int x;\n"
                + "    int y;\n"
                + "} Point;\n\n"
                + "typedef struct BitHeader {\n"
                + "    unsigned int flags : 3;\n"
                + "    unsigned int mode : 5;\n"
                + "} BitHeader;\n\n"
                + "typedef union AnyVal {\n"
                + "    int i;\n"
                + "    double d;\n"
                + "} AnyVal;\n\n"
                + "typedef enum Status {\n"
                + "    STATUS_OK = 0,\n"
                + "    STATUS_ERR_IO = 10,\n"
                + "    STATUS_ERR_PERM\n"
                + "} Status;\n\n"
                + "typedef struct Packet {\n"
                + "    uint32_t len;\n"
                + "    uint8_t payload[];\n"
                + "} Packet;\n\n"
                + "int sqlite3_open(const char* filename, void** ppDb);\n";

            var diags: List<Diagnostic> = [];
            let tu = c_parse_translation_unit(c_code, "test.h", var diags);

            // Verify records (Point, BitHeader, AnyVal, Packet)
            if tu.records.len() != 4 { return 1; }

            // Point
            let pt = tu.records[0];
            if pt.name != "Point" { return 2; }
            if pt.fields.len() != 2 { return 3; }
            if pt.fields[0].byte_offset != 0 { return 4; }
            if pt.fields[1].name != "y" { return 200; }
            if pt.fields[1].byte_offset != 4 { return 5; }

            // BitHeader
            let bh = tu.records[1];
            if bh.fields.len() != 2 { return 6; }
            if !bh.fields[0].is_bitfield { return 7; }
            if bh.fields[0].bitfield.bit_width != 3 { return 8; }
            if bh.fields[1].bitfield.bit_offset != 3 { return 9; }
            if bh.fields[1].bitfield.bit_width != 5 { return 10; }

            // AnyVal union
            let av = tu.records[2];
            if !av.is_union { return 11; }
            if av.fields[0].byte_offset != 0 { return 12; }
            if av.fields[1].byte_offset != 0 { return 13; }

            // Packet flexible array tail
            let pkt = tu.records[3];
            if !pkt.has_flexible_array_tail { return 14; }

            // Enum Status
            if tu.enums.len() != 1 { return 15; }
            let st = tu.enums[0];
            if st.name != "Status" { return 15; }
            if st.items.len() != 3 { return 16; }
            if st.items[0].value != 0 { return 17; }
            if st.items[1].value != 10 { return 18; }
            if st.items[2].value != 11 { return 19; }

            // Function declaration sqlite3_open
            if tu.functions.len() != 1 { return 20; }
            let fn_decl = tu.functions[0];
            if fn_decl.name != "sqlite3_open" { return 21; }
            if fn_decl.params.len() != 2 { return 22; }

            // Test CAB004 Bitfield width exceeding storage
            let bad_bf = "typedef struct BadBf { uint8_t wide : 16; } BadBf;\n";
            var bad_diags: List<Diagnostic> = [];
            let _ = c_parse_translation_unit(bad_bf, "bad.h", var bad_diags);
            if bad_diags.len() == 0 { return 23; }
            if bad_diags[0].code != "CAB004" { return 24; }

            return 42;
        }
    "#;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "stdlib/core/file.pilot",
        "selfhost/adapter/c_parser.pilot",
    ];
    let res = eval_m12(deps, script);
    assert_eq!(res, "42", "C11/C23 parser must accurately extract structs, unions, bitfields, enums, flexible tails, and emit CAB004");

    // A header on disk that includes a second header: the absorber reads
    // both from disk, the target's data model picks the `#if` branch, and the
    // layout follows C (int[4] = 16 bytes; LP64 long = 8 bytes).
    let dir = std::env::temp_dir().join(format!("pilot_m12_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("inner.h"), "#define N 4\ntypedef struct In { int a[N]; int b; } In;\n").unwrap();
    fs::write(
        dir.join("outer.h"),
        "#include \"inner.h\"\n#if defined(__LP64__)\ntypedef struct W { long v; } W;\n#else\ntypedef struct W { char v; } W;\n#endif\n",
    )
    .unwrap();
    let dir_s = dir.to_string_lossy().replace('\\', "/");
    let file_script = format!(
        r#"
        pub fn main() -> i64 {{
            var diags: List<Diagnostic> = [];
            let tu = absorb_c_header_file("{dir_s}/outer.h", ["{dir_s}"], "linux", "x86_64", var diags);
            if tu.records.len() != 2 {{ return 1; }}
            let inner = tu.records[0];
            if inner.name != "In" {{ return 2; }}
            if inner.fields[0].byte_size != 16 {{ return 3; }}
            if inner.fields[1].byte_offset != 16 {{ return 4; }}
            let w = tu.records[1];
            if w.name != "W" || w.fields[0].byte_size != 8 {{ return 5; }}
            return 42;
        }}
    "#
    );
    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "stdlib/core/file.pilot",
        "stdlib/std/text.pilot",
        "selfhost/adapter/c_pp.pilot",
        "selfhost/adapter/c_parser.pilot",
    ];
    let res = eval_m12(deps, &file_script);
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(res, "42", "absorbing a header from disk must follow its #include and the target's #if branches");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Cross-Platform Sysroot & Pkg-Config Discovery (`c_sysroot.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m12_3_cross_platform_sysroot_and_pkg_config_discovery() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m12_3_cross_platform_sysroot_and_pkg_config_discovery",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            let linux_sys = sysroot_discovery_new("linux", "x86_64");
            if linux_sys.include_dirs.len() == 0 { return 1; }
            var has_usr_inc = false;
            var i: usize = 0;
            while i < linux_sys.include_dirs.len() {
                if linux_sys.include_dirs[i] == "/usr/include" { has_usr_inc = true; }
                i = i + 1;
            }
            if !has_usr_inc { return 2; }

            let mac_sys = sysroot_discovery_new("macos", "aarch64");
            var has_mac_sdk = false;
            i = 0;
            while i < mac_sys.include_dirs.len() {
                if mac_sys.include_dirs[i].contains("MacOSX.sdk") { has_mac_sdk = true; }
                i = i + 1;
            }
            if !has_mac_sdk { return 3; }

            let win_sys = sysroot_discovery_new("windows", "x86_64");
            var has_win_kits = false;
            i = 0;
            while i < win_sys.include_dirs.len() {
                if win_sys.include_dirs[i].contains("Windows Kits") { has_win_kits = true; }
                i = i + 1;
            }
            if !has_win_kits { return 4; }

            // Test .pc (pkg-config) parser with nested variables
            let pc_text = "prefix=/usr\n"
                + "exec_prefix=${prefix}\n"
                + "libdir=${exec_prefix}/lib\n"
                + "includedir=${prefix}/include\n\n"
                + "Name: zlib\n"
                + "Description: zlib compression library\n"
                + "Version: 1.2.11\n"
                + "Requires:\n"
                + "Libs: -L${libdir} -lz\n"
                + "Cflags: -I${includedir}\n";

            let pcr = parse_pkg_config_file(pc_text);
            if pcr.includes.len() != 1 { return 5; }
            if pcr.includes[0] != "/usr/include" { return 6; }
            if pcr.libs.len() != 2 { return 7; }
            if pcr.libs[0] != "-L/usr/lib" { return 8; }
            if pcr.libs[1] != "-lz" { return 9; }

            return 42;
        }
    "#;

    let deps = &[
        "stdlib/core/file.pilot",
        "selfhost/adapter/c_sysroot.pilot",
    ];
    let res = eval_m12(deps, script);
    assert_eq!(res, "42", "Sysroot discovery and pure-Pilot pkg-config parser must handle platform paths and variable expansion");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Upgraded FFI Bindgen Multi-Param & Anti-Cheat Audit (`ffi_bindgen.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m12_4_ffi_bindgen_multi_param_and_anti_cheat_audit() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m12_4_ffi_bindgen_multi_param_and_anti_cheat_audit",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            // Test single-param backwards compatibility
            let b1 = generate_ffi_binding("libm", "sqrt", "x", 4, 64, 4, 64);
            if b1.symbol_name != "sqrt" { return 1; }
            if !b1.c_prototype.contains("double sqrt(double x);") { return 2; }

            // Test multi-param generate_ffi_binding_multi
            var params: List<FfiParamSpec> = [
                FfiParamSpec { name: "filename", reg_class: 5, bit_width: 64 },
                FfiParamSpec { name: "flags", reg_class: 2, bit_width: 32 },
                FfiParamSpec { name: "mode", reg_class: 3, bit_width: 32 },
            ];
            let b2 = generate_ffi_binding_multi("libc", "open", params, 2, 32);
            if b2.symbol_name != "open" { return 3; }
            if !b2.c_prototype.contains("int32_t open(void* filename, int32_t flags, uint32_t mode);") { return 4; }
            if !b2.csharp_library_import.contains("[LibraryImport(\"libc\", EntryPoint = \"open\")]") { return 5; }

            return 42;
        }
    "#;

    let deps = &[
        "stdlib/std/text.pilot",
        "selfhost/ffi_bindgen.pilot",
    ];
    let res = eval_m12(deps, script);
    assert_eq!(res, "42", "FFI bindgen must support both single-param and multi-param generation with typed prototypes");

    guard.finish();
}
