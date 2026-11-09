//! ============================================================================
//! Milestone 13 Verification Suite:
//! Adaptive Hardware Calling-Convention Descriptors & Dynamic Frame Trampoline (`frame` / `!&`)
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 13,
    test_file_rel: "crates/pilot-analysis/tests/milestone13_adaptive_hardware_calling_conventions.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "stdlib/pilot/abi/descriptor.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "classify_call_frame",
                "invoke_dynamic_frame",
                "calling_conv_count",
                "calling_conv_get",
            ],
            required_code_markers: &[
                "ArgPassKind",
                "ArgLocation",
                "AbiFramePlan",
                "RawDispatchPacket",
                "CallingConvSpec",
                "ICallingConvClassifier",
                "AbiDiagnostic",
                "ABI001",
                "ABI002",
                "ABI003",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/pilot/abi/sysv_x64.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[
                "SysVConvClassifier",
                "sysv_x64",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/pilot/abi/win64.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[
                "Win64ConvClassifier",
                "win64",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/pilot/abi/aapcs64.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[
                "Aapcs64ConvClassifier",
                "aapcs64",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/pilot/abi/riscv_wasm.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[
                "Riscv64ConvClassifier",
                "Wasm32ConvClassifier",
                "riscv64",
                "wasm32",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m13(deps: &[&str], script: &str) -> String {
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
    vm.run().to_string_repr()
}

// Target test functions for native dynamic frame invocation
extern "C" fn add10(
    a: i64, b: i64, c: i64, d: i64, e: i64,
    f: i64, g: i64, h: i64, i: i64, j: i64
) -> i64 {
    a + b + c + d + e + f + g + h + i + j
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Pair16 {
    a: u64,
    b: u64,
}

extern "C" fn make_pair(a: u64, b: u64) -> Pair16 {
    Pair16 { a, b }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct BigStruct32 {
    a: u64,
    b: u64,
    c: u64,
    d: u64,
}

extern "C" fn sum_and_fill(a: u64, b: u64, c: u64) -> BigStruct32 {
    BigStruct32 { a, b, c, d: a + b + c }
}

#[test]
fn test_m13_1_calling_convention_descriptors_and_sysv_x64_eightbyte_classification() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m13_1_calling_convention_descriptors_and_sysv_x64_eightbyte_classification",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            let cnt = calling_conv_count();
            if cnt != 6 { return 1; }

            let e0 = calling_conv_get(0);
            if e0.conv_id != "sysv_x64" { return 2; }

            // SysV Classifier test
            let spec = SysVConvClassifier::spec();
            if spec.max_gpr_args != 6 { return 3; }
            if spec.max_vec_args != 8 { return 4; }
            if spec.stack_align != 16 { return 5; }

            // Case A: 2 scalars returning scalar
            let p_i64 = TypeLayout::create(8, 8, 0, 0);
            let ret_i64 = TypeLayout::create(8, 8, 0, 0);
            let params_a: slice<TypeLayout> = [p_i64, p_i64];
            let plan_a = SysVConvClassifier::classify(params_a, ref ret_i64, 0);

            if plan_a.sret_uses_gpr0 { return 6; }
            if plan_a.aligned_stack_bytes != 0 { return 7; }
            if plan_a.param_count() != 2 { return 8; }
            let p0 = plan_a.get_param(0);
            let p1 = plan_a.get_param(1);
            if p0.reg0 != 0 { return 9; } // RDI
            if p1.reg0 != 1 { return 10; } // RSI

            // Case B: 16B struct pair return in RAX:RDX
            let ret_struct16 = TypeLayout::create(16, 8, 0, 5);
            let plan_b = SysVConvClassifier::classify(params_a, ref ret_struct16, 0);
            if plan_b.ret_loc.reg0 != 0 { return 11; } // RAX
            if plan_b.ret_loc.reg1 != 2 { return 12; } // RDX

            // Case C: Sret struct > 16B (RDI hidden pointer, arg 0 shifts to RSI)
            let ret_struct32 = TypeLayout::create(32, 8, 0, 5);
            let plan_c = SysVConvClassifier::classify(params_a, ref ret_struct32, 0);
            if !plan_c.sret_uses_gpr0 { return 13; }
            let p0_shifted = plan_c.get_param(0);
            if p0_shifted.reg0 != 1 { return 14; } // Shifted to RSI!

            return 100;
        }
    "#;

    let res = eval_m13(
        &[
            "stdlib/core/reflect.pilot",
            "stdlib/pilot/abi/descriptor.pilot",
            "stdlib/pilot/abi/sysv_x64.pilot",
        ],
        script,
    );
    assert_eq!(res, "100", "SysV classification verification passed");

    guard.finish();
}

#[test]
fn test_m13_2_win64_shadow_space_and_aapcs64_x8_sret_hfa() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m13_2_win64_shadow_space_and_aapcs64_x8_sret_hfa",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        pub fn main() -> i64 {
            // 1. Win64: 32-byte shadow space & IndirectByRef for 16B struct
            let win_spec = Win64ConvClassifier::spec();
            if win_spec.shadow_space_bytes != 32 { return 1; }
            if win_spec.max_gpr_args != 4 { return 2; }

            let s16 = TypeLayout::create(16, 8, 0, 5);
            let ret_void = TypeLayout::create(0, 0, 0, 6);
            let params_win: slice<TypeLayout> = [s16];
            let plan_win = Win64ConvClassifier::classify(params_win, ref ret_void, 0);
            let win_p0 = plan_win.get_param(0);
            if !win_p0.is_indirect() { return 3; } // 16B struct passed by reference!

            // 2. AAPCS64: Dedicated X8 sret register (sret_uses_gpr0 == false!)
            let arm_spec = Aapcs64ConvClassifier::spec();
            if !arm_spec.sret_dedicated_reg { return 4; }
            if arm_spec.sret_reg != 8 { return 5; }

            let ret_struct32 = TypeLayout::create(32, 8, 0, 5);
            let p_i64 = TypeLayout::create(8, 8, 0, 0);
            let params_arm: slice<TypeLayout> = [p_i64];
            let plan_arm = Aapcs64ConvClassifier::classify(params_arm, ref ret_struct32, 0);
            if plan_arm.sret_uses_gpr0 { return 6; } // Does NOT use GPR0!
            let arm_p0 = plan_arm.get_param(0);
            if arm_p0.reg0 != 0 { return 7; } // Explicit arg 0 remains in X0!

            // 3. AAPCS64: Homogeneous Floating-Point Aggregate (HFA 3-float 12B)
            let hfa3 = TypeLayout::create(12, 4, 0, 5);
            let params_hfa: slice<TypeLayout> = [hfa3];
            let plan_hfa = Aapcs64ConvClassifier::classify(params_hfa, ref ret_void, 0);
            let hfa_p0 = plan_hfa.get_param(0);
            if hfa_p0.reg1 != 3 { return 8; } // 3 SIMD registers used for HFA3!

            // 4. RISC-V and WASM specs
            let rv_spec = Riscv64ConvClassifier::spec();
            if rv_spec.max_gpr_args != 8 { return 9; }
            let wasm_spec = Wasm32ConvClassifier::spec();
            if wasm_spec.max_gpr_args != 16 { return 10; }

            return 200;
        }
    "#;

    let res = eval_m13(
        &[
            "stdlib/core/reflect.pilot",
            "stdlib/pilot/abi/descriptor.pilot",
            "stdlib/pilot/abi/win64.pilot",
            "stdlib/pilot/abi/aapcs64.pilot",
            "stdlib/pilot/abi/riscv_wasm.pilot",
        ],
        script,
    );
    assert_eq!(res, "200", "Win64 & AAPCS64 HFA & X8 sret verified");

    guard.finish();
}

#[test]
fn test_m13_3_dynamic_frame_trampoline_native_invocation() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m13_3_dynamic_frame_trampoline_native_invocation",
        &ANTICHEAT_SPEC,
    );

    unsafe {
        // Test A: 10 scalar arguments (testing stack spilling + 16-byte stack alignment)
        let gprs_a = [1u64, 2, 3, 4, 5, 6, 0, 0];
        let stack_args = [7u64, 8, 9, 10]; // g, h, i, j on stack
        let mut ret = [0u64; 4];
        pilot_rt::pilot_rt_invoke_abi_frame(
            add10 as *const (),
            gprs_a.as_ptr(),
            std::ptr::null(),
            stack_args.as_ptr() as *const u8,
            32,
            ret.as_mut_ptr(),
        );
        assert_eq!(ret[0], 55, "add10 must return sum 1..10 = 55");

        // Test B: 16-byte struct return in RAX:RDX
        let gprs_b = [100u64, 200, 0, 0, 0, 0, 0, 0];
        pilot_rt::pilot_rt_invoke_abi_frame(
            make_pair as *const (),
            gprs_b.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            ret.as_mut_ptr(),
        );
        assert_eq!(ret[0], 100, "Pair16.a must be 100");
        assert_eq!(ret[1], 200, "Pair16.b must be 200");

        // Test C: 32-byte sret struct return via RDI hidden pointer
        let mut big = BigStruct32 { a: 0, b: 0, c: 0, d: 0 };
        let gprs_c = [
            &mut big as *mut BigStruct32 as u64, // RDI = sret pointer
            10, // RSI = arg a
            20, // RDX = arg b
            30, // RCX = arg c
            0, 0, 0, 0,
        ];
        pilot_rt::pilot_rt_invoke_abi_frame(
            sum_and_fill as *const (),
            gprs_c.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            ret.as_mut_ptr(),
        );
        assert_eq!(big.a, 10);
        assert_eq!(big.b, 20);
        assert_eq!(big.c, 30);
        assert_eq!(big.d, 60);

        // Test D: Variadic snprintf with %al = 1 (1 vector register used)
        extern "C" {
            fn snprintf(buf: *mut u8, size: usize, format: *const u8, ...) -> i32;
        }
        let mut buf = [0u8; 64];
        let fmt = b"Val: %d, Float: %.1f\0";
        let float_val: f64 = 3.5;
        let xmm_vals = [float_val, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let gprs_d = [
            buf.as_mut_ptr() as u64, // RDI = buf
            64,                      // RSI = size
            fmt.as_ptr() as u64,     // RDX = format
            42,                      // RCX = int val
            0, 0,
            1,                       // RAX = 1 (1 vector register used: xmm0)
            0,
        ];
        pilot_rt::pilot_rt_invoke_abi_frame(
            snprintf as *const (),
            gprs_d.as_ptr(),
            xmm_vals.as_ptr(),
            std::ptr::null(),
            0,
            ret.as_mut_ptr(),
        );
        let s = std::ffi::CStr::from_ptr(buf.as_ptr() as *const i8).to_str().unwrap();
        assert_eq!(s, "Val: 42, Float: 3.5");
        assert_eq!(ret[0] as i32, 19);
    }

    guard.finish();
}

#[test]
fn test_m13_4_pilvm_frame_homoiconic_call_and_diagnostics() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m13_4_pilvm_frame_homoiconic_call_and_diagnostics",
        &ANTICHEAT_SPEC,
    );

    // Verify DynamicLibraryHandle and dynamic resolution from libc
    let libc_res = pilot_rt::DynamicLibraryHandle::open("libc.so.6")
        .or_else(|_| pilot_rt::DynamicLibraryHandle::open("libc.so"));
    if let Ok(libc) = libc_res {
        if let Ok(puts_sym) = libc.resolve_symbol("puts") {
            assert!(!puts_sym.is_null());
        }
    }

    // Verify diagnostics and RawDispatchPacket in PilVM
    let script = r#"
        pub fn main() -> i64 {
            // 1. RawDispatchPacket layout & setters
            var pkt = RawDispatchPacket::create_empty();
            pkt.set_gpr(0, 42);
            pkt.set_gpr(1, 84);
            pkt.set_vec(0, 3.14);

            if pkt.get_gpr(0) != 42 { return 1; }
            if pkt.get_gpr(1) != 84 { return 2; }

            // 2. Diagnostics verification
            let d1 = AbiDiagnostic::symbol_not_found("my_sym", "libtest.so");
            if d1.code != "ABI001" { return 3; }

            let d2 = AbiDiagnostic::library_load_failed("missing.so", "not found");
            if d2.code != "ABI002" { return 4; }

            let d3 = AbiDiagnostic::unsupported_conv("bad_conv", "sparc");
            if d3.code != "ABI003" { return 5; }

            return 300;
        }
    "#;

    let res = eval_m13(&["stdlib/pilot/abi/descriptor.pilot"], script);
    assert_eq!(res, "300", "PilVM frame & diagnostics passed");

    guard.finish();
}
