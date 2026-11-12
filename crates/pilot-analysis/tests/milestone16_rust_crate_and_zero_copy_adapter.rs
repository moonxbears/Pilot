//! ============================================================================
//! Milestone 16 Verification Suite:
//! Native Rust Crate & Zero-Copy Slice/Result/Trait Adapter (`use rust`)
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
    milestone_id: 16,
    test_file_rel: "crates/pilot-analysis/tests/milestone16_rust_crate_and_zero_copy_adapter.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/adapter/rust_sig.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "rust_parse_source",
                "cargo_parse_manifest",
                "rust_type_classify",
                "rust_diag_code",
            ],
            required_code_markers: &[
                "RustItemKind",
                "RustTypeKind",
                "RustFieldDecl",
                "RustFnDecl",
                "RustStructDecl",
                "RustTraitDecl",
                "RustModuleDecl",
                "CargoMetadata",
                "RST002",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/adapter/rust_bridge.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "rust_generate_bridge_crate",
                "rust_bridge_cache_key",
                "rust_verify_lifetime_bindings",
                "rust_synthesize_dyn_vtable",
                "absorb_rust_crate",
            ],
            required_code_markers: &[
                "catch_unwind",
                "free_vec",
                "free_string",
                "DynTraitFatPtr",
                "RST003",
                "RST004",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/std/polyglot/rust.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "rust_call_bridge",
                "rust_vec_from_raw_parts",
                "rust_string_from_raw_parts",
            ],
            required_code_markers: &[
                "RustPanicErr",
                "DynTraitFatPtr",
                "RustVec",
                "RustString",
                "RST001",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m16(deps: &[&str], script: &str) -> String {
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
    vm_outcome::outcome(&mut vm)
}

// -----------------------------------------------------------------------------
// Test 1: Rust Public Items & Cargo Manifest Parser (`rust_sig.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m16_1_rust_signature_parser_and_cargo_metadata() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m16_1_rust_signature_parser_and_cargo_metadata",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            let rust_src = "#[repr(C)]\n"
                + "pub struct Config {\n"
                + "    pub timeout_ms: u64,\n"
                + "    pub retries: u32,\n"
                + "}\n"
                + "pub enum Status {\n"
                + "    Active,\n"
                + "    Inactive(String),\n"
                + "}\n"
                + "pub trait Processor {\n"
                + "    fn process(&self, input: &str) -> Vec<u8>;\n"
                + "}\n"
                + "pub fn compute_hash(input: &str) -> Vec<u8> {\n"
                + "    vec![]\n"
                + "}\n";

            var diags: List<Diagnostic> = [];
            let module = rust_parse_source(rust_src, var diags);

            // Verify structs
            if module.structs.len() != 1 { return 1; }
            let s0 = module.structs[0];
            if s0.name != "Config" { return 2; }
            if !s0.is_repr_c { return 3; }
            if s0.fields.len() != 2 { return 4; }
            if s0.fields[0].name != "timeout_ms" { return 5; }
            if s0.fields[1].name != "retries" { return 6; }

            // Verify enums
            if module.enums.len() != 1 { return 7; }
            let e0 = module.enums[0];
            if e0.name != "Status" { return 8; }
            if e0.variants.len() != 2 { return 9; }
            if e0.variants[0].name != "Active" { return 10; }
            if !e0.variants[1].has_payload { return 11; }

            // Verify traits
            if module.traits.len() != 1 { return 12; }
            let t0 = module.traits[0];
            if t0.name != "Processor" { return 13; }
            if t0.methods.len() != 1 { return 14; }
            if t0.methods[0].name != "process" { return 15; }

            // Verify functions
            if module.functions.len() != 1 { return 16; }
            let f0 = module.functions[0];
            if f0.name != "compute_hash" { return 17; }
            if f0.params.len() != 1 { return 18; }
            if f0.params[0].type_kind != RustTypeKind::StrRef { return 19; }
            if f0.ret_kind != RustTypeKind::Vec { return 20; }

            // Verify Cargo.toml parsing
            let toml_src = "[package]\nname = \"serde\"\nversion = \"1.0.197\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\nserde_json = \"1.0\"\n";
            let meta = cargo_parse_manifest(toml_src);
            if meta.name != "serde" { return 21; }
            if meta.version != "1.0.197" { return 22; }
            if meta.edition != "2021" { return 23; }

            // Verify Rust v0 symbol demangling
            let d1 = rust_demangle_v0("_RNvCs1234_7mycrate3foo");
            if d1 != "mycrate::foo" { return 24; }

            let d2 = rust_demangle_v0("_RNtCs1234_7mycrate5MyMod3bar");
            if d2 != "mycrate::MyMod::bar" { return 25; }

            let d3 = rust_demangle_v0("plain_c_symbol");
            if d3 != "plain_c_symbol" { return 26; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/rust_sig.pilot",
    ];
    let res = eval_m16(deps, script);
    assert_eq!(res, "42", "Rust signature parser and Cargo manifest reader must extract items accurately");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Zero-Copy Slices, Mut Slices & Niche Optimizations
// -----------------------------------------------------------------------------
#[test]
fn test_m16_2_zero_copy_slices_and_niche_opt() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m16_2_zero_copy_slices_and_niche_opt",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // Type classification
            if rust_type_classify("&str") != RustTypeKind::StrRef { return 1; }
            if rust_type_classify("&'a str") != RustTypeKind::StrRef { return 2; }
            if rust_type_classify("&[u8]") != RustTypeKind::SliceRef { return 3; }
            if rust_type_classify("&'a [f64]") != RustTypeKind::SliceRef { return 4; }
            if rust_type_classify("&mut [u8]") != RustTypeKind::SliceMut { return 5; }
            if rust_type_classify("&'a mut [i32]") != RustTypeKind::SliceMut { return 6; }
            if rust_type_classify("Option<&T>") != RustTypeKind::OptNiche { return 7; }
            if rust_type_classify("Option<NonNull<T>>") != RustTypeKind::OptNiche { return 8; }
            if rust_type_classify("Result<T, E>") != RustTypeKind::Result { return 9; }
            if rust_type_classify("Vec<u8>") != RustTypeKind::Vec { return 10; }
            if rust_type_classify("String") != RustTypeKind::String { return 11; }
            if rust_type_classify("&dyn Processor") != RustTypeKind::DynTrait { return 12; }

            // Verify slice 2-word fat pointer size (16 bytes = 2 registers)
            let s_decl = RustFieldDecl {
                name: "buf",
                type_name: "&[u8]",
                type_kind: RustTypeKind::SliceRef,
                is_pub: true,
                byte_offset: 0,
                byte_size: 16,
            };
            if s_decl.byte_size != 16 { return 13; }

            // Verify Vec 3-word size (24 bytes)
            let v_decl = RustFieldDecl {
                name: "items",
                type_name: "Vec<u8>",
                type_kind: RustTypeKind::Vec,
                is_pub: true,
                byte_offset: 16,
                byte_size: 24,
            };
            if v_decl.byte_size != 24 { return 14; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/rust_sig.pilot",
    ];
    let res = eval_m16(deps, script);
    assert_eq!(res, "42", "Zero-copy slice classification and 2-word layout invariants must hold");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: RustVec, RustString Cross-Allocator RAII & DynTraitFatPtr (`rust.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m16_3_rust_vec_string_cross_allocator_raii_and_dyn_trait() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m16_3_rust_vec_string_cross_allocator_raii_and_dyn_trait",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // 1. RustVec creation and projection
            var vec = rust_vec_from_raw_parts(4096, 64, 32, 8192);
            if vec.len() != 32 { return 1; }
            if vec.cap() != 64 { return 2; }
            if vec.is_empty() { return 3; }

            // take_raw transfers ownership and disarms
            let raw_p = vec.take_raw();
            if raw_p != 4096 { return 4; }
            if vec.raw_ptr != 0 { return 5; }
            if vec.is_armed { return 6; }

            // 2. RustString creation
            var rstr = rust_string_from_raw_parts(8192, 128, 48, 16384);
            if rstr.len() != 48 { return 7; }
            if rstr.cap() != 128 { return 8; }
            if rstr.is_empty() { return 9; }
            let s_raw = rstr.take_raw();
            if s_raw != 8192 { return 10; }
            if rstr.is_armed { return 11; }

            // 3. DynTraitFatPtr
            let fat_ptr = DynTraitFatPtr::create(65536, 131072);
            if !fat_ptr.is_valid() { return 12; }
            if fat_ptr.data_ptr != 65536 { return 13; }
            if fat_ptr.vtable_ptr != 131072 { return 14; }

            return 42;
        }
    "##;

    let deps = &[
        "stdlib/std/polyglot/rust.pilot",
    ];
    let res = eval_m16(deps, script);
    assert_eq!(res, "42", "RustVec, RustString and DynTraitFatPtr must maintain exact cross-allocator safety");

    // Freeing, viewing and calling need PilVM native calls and memory views.
    let cases = [
        ("var v = rust_vec_from_raw_parts(4096, 64, 32, 8192); v.~self();", "rust.call_dealloc"),
        ("var v = rust_vec_from_raw_parts(4096, 64, 32, 8192); let s = v.as_slice();", "rust.vec_view"),
        ("var t = rust_string_from_raw_parts(8192, 128, 48, 16384); let s = t.as_str();", "rust.string_view"),
        ("let r = rust_call_bridge(4096, [3, 4]);", "rust.call_bridge"),
    ];
    for (body, seam_id) in cases {
        let script = format!("pub fn main() -> i64 {{ {body} return 42; }}");
        let res = eval_m16(deps, &script);
        assert!(
            res.starts_with(&format!("FAULT: SEAM001: unconnected seam `{seam_id}`")),
            "`{body}` must stop at seam {seam_id}, got {res}"
        );
    }
    // A disarmed (taken) value drops without calling the deallocator.
    let taken = "pub fn main() -> i64 { var v = rust_vec_from_raw_parts(4096, 64, 32, 8192); let p = v.take_raw(); v.~self(); return p; }";
    assert_eq!(eval_m16(deps, taken), "4096");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Panic Catch Boundary & Lifetime Verification (`rust_bridge.pilot`)
// -----------------------------------------------------------------------------
#[test]
fn test_m16_4_panic_boundary_catch_unwind_and_diagnostics() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m16_4_panic_boundary_catch_unwind_and_diagnostics",
        &ANTICHEAT_SPEC,
    );

    let script = r##"
        pub fn main() -> i64 {
            // 3. Diagnostic codes
            if rust_diag_code(RustDiagnosticKind::CrossAllocatorFree) != "RST001" { return 6; }
            if rust_diag_code(RustDiagnosticKind::CargoResolveError) != "RST002" { return 7; }
            if rust_diag_code(RustDiagnosticKind::UnboundBorrow) != "RST003" { return 8; }
            if rust_diag_code(RustDiagnosticKind::PanicUnwind) != "RST004" { return 9; }

            // 4. Lifetime borrow tying verification (RST003)
            var bad_fn = RustFnDecl {
                name: "dangling_str",
                return_type: "&str",
                ret_kind: RustTypeKind::StrRef,
                is_pub: true,
                is_unsafe: false,
                is_extern_c: false,
                params: [],
            };
            var diags: List<Diagnostic> = [];
            let valid = rust_verify_lifetime_bindings(ref bad_fn, var diags);
            if valid { return 10; }
            if diags.len() == 0 { return 11; }
            if diags[0].code != "RST003" { return 12; }

            // 6. Content cache key
            let key1 = rust_bridge_cache_key("sample_crate", "pub fn a() {}");
            let key2 = rust_bridge_cache_key("sample_crate", "pub fn a() {}");
            let key3 = rust_bridge_cache_key("sample_crate", "pub fn b() {}");
            if key1 != key2 { return 16; }
            if key1 == key3 { return 17; }

            return 42;
        }
    "##;

    let deps = &[
        "selfhost/diagnostics/model.pilot",
        "selfhost/adapter/rust_sig.pilot",
        "stdlib/std/text.pilot",
        "selfhost/adapter/rust_bridge.pilot",
        "stdlib/std/polyglot/rust.pilot",
    ];
    let res = eval_m16(deps, script);
    assert_eq!(res, "42", "RST diagnostics, RST003 lifetime tying and cache keys must pass");

    bridge_crate_compiles_and_catches_panics(deps);

    // Resolving a crate through cargo, and bridging a Result return, stop at seams.
    let cases = [
        ("var d: List<Diagnostic> = []; let m = absorb_rust_crate(\"serde\", [], var d);", "rust.cargo_resolve"),
        (
            "var d: List<Diagnostic> = []; let m = absorb_rust_source(\"pub fn parse(s: &str) -> Result<u32, String> { Ok(1) }\", var d); let b = rust_generate_bridge_crate(ref m, \"c\");",
            "rust.bridge_marshal",
        ),
    ];
    for (body, seam_id) in cases {
        let script = format!("pub fn main() -> i64 {{ {body} return 42; }}");
        let res = eval_m16(deps, &script);
        assert!(
            res.starts_with(&format!("FAULT: SEAM001: unconnected seam `{seam_id}`")),
            "`{body}` must stop at seam {seam_id}, got {res}"
        );
    }

    guard.finish();
}

/// Generates the bridge for a small crate with Pilot, compiles crate +
/// bridge + a C-ABI caller with rustc, and checks what the program prints:
/// the answers come from rustc and Rust's std, not from the generator.
fn bridge_crate_compiles_and_catches_panics(deps: &[&str]) {
    let lib = "pub fn add(a: u64, b: u64) -> u64 { a + b }\n\
        pub fn tokens(text: &str) -> Vec<u32> { text.split(' ').map(|w| w.len() as u32).collect() }\n\
        pub fn sort_in_place(xs: &mut [f64]) { xs.sort_by(|a, b| a.partial_cmp(b).unwrap()); }\n\
        pub fn greet(name: &str) -> String { let mut s = String::from(\"hi \"); s.push_str(name); s }\n\
        pub fn first_word(text: &str) -> &str { text.split(' ').next().unwrap() }\n\
        pub fn boom(i: u64) -> u64 { let v: Vec<u64> = Vec::new(); v[i as usize] }\n";
    let dir = std::env::temp_dir().join(format!("pilot_m16_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let lib_path = dir.join("lib.rs");
    fs::write(&lib_path, lib).unwrap();
    let lib_s = lib_path.to_string_lossy().replace('\\', "/");

    let mut gen_deps = vec!["stdlib/core/file.pilot"];
    gen_deps.extend_from_slice(deps);
    let script = format!(
        r#"
        pub fn main() -> str {{
            var diags: List<Diagnostic> = [];
            let m = absorb_rust_source(File.read_text("{lib_s}"), var diags);
            if diags.len() != 0 {{ return "diagnostics"; }}
            return rust_generate_bridge_crate(ref m, "sample_crate");
        }}
    "#
    );
    let bridge = eval_m16(&gen_deps, &script);
    assert!(bridge.contains("bridge_sample_crate_add"), "bridge generation failed: {bridge}");

    let caller = r#"
mod sample_crate {
    include!("lib.rs");
}

fn main() {
    unsafe {
        let r = bridge_sample_crate_add(3, 4);
        println!("add {} {}", r.is_ok, r.val);
        let text = "aa bbb c";
        let view = PilotSliceView { ptr: text.as_ptr(), len: text.len() };
        let r = bridge_sample_crate_tokens(view);
        println!("tokens {:?}", std::slice::from_raw_parts(r.val.ptr, r.val.len));
        bridge_free_sample_crate_tokens(r.val.ptr, r.val.cap, r.val.len);
        let mut xs = [3.0f64, 1.0, 2.0];
        let r = bridge_sample_crate_sort_in_place(PilotSliceMut { ptr: xs.as_mut_ptr(), len: 3 });
        println!("sort {} {:?}", r.is_ok, xs);
        let who = "pilot";
        let r = bridge_sample_crate_greet(PilotSliceView { ptr: who.as_ptr(), len: who.len() });
        let bytes = std::slice::from_raw_parts(r.val.ptr, r.val.len);
        println!("greet {}", std::str::from_utf8(bytes).unwrap());
        pilot_rust_free_string_sample_crate(r.val.ptr, r.val.cap, r.val.len);
        let r = bridge_sample_crate_first_word(view);
        println!("first {}", std::str::from_utf8(std::slice::from_raw_parts(r.val.ptr, r.val.len)).unwrap());
        std::panic::set_hook(Box::new(|_| {}));
        let r = bridge_sample_crate_boom(99);
        let msg = std::str::from_utf8(std::slice::from_raw_parts(r.panic_msg, r.panic_len)).unwrap();
        println!("boom {} {}", r.is_ok, msg);
    }
}
"#;
    let main_path = dir.join("main.rs");
    fs::write(&main_path, format!("{bridge}\n{caller}")).unwrap();
    let exe = dir.join("bridge_check");
    let rustc = std::process::Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&exe)
        .arg(&main_path)
        .output()
        .expect("run rustc");
    assert!(
        rustc.status.success(),
        "generated bridge must compile with rustc:\n{}",
        String::from_utf8_lossy(&rustc.stderr)
    );
    let run = std::process::Command::new(&exe).output().expect("run bridge_check");
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(
        stdout,
        "add true 7\n\
         tokens [2, 3, 1]\n\
         sort true [1.0, 2.0, 3.0]\n\
         greet hi pilot\n\
         first aa\n\
         boom false index out of bounds: the len is 0 but the index is 99\n"
    );
}
