//! Milestone 55 Verification Suite:
//! Self-Hosted PilVM v3 Bytecode Compiler, `.pilc` Serializer & `VmHook` Debug Table.

use std::path::Path;
use std::sync::{Arc, Mutex};
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, Opcode, PilVirtualMachine, PILC_V3_MAGIC, PILC_V3_VERSION};

#[test]
fn test_selfhost_vm_compiler_and_pilc_writer_execute_on_pilvm() {
    let vm_compiler = std::fs::read_to_string("../../selfhost/vm/compiler.pilot")
        .expect("selfhost/vm/compiler.pilot must exist");
    let pilc_writer = std::fs::read_to_string("../../selfhost/vm/pilc_writer.pilot")
        .expect("selfhost/vm/pilc_writer.pilot must exist");

    let script = format!(
        "{}\n{}\n\
        let ins_add = encode_pilvm_instruction(1, 5, 2, 3);\n\
        let op = decode_pilvm_opcode(ins_add);\n\
        let dst = decode_pilvm_dst(ins_add);\n\
        let s1 = decode_pilvm_src1(ins_add);\n\
        let s2 = decode_pilvm_src2(ins_add);\n\
        let jmp = encode_jump_to_pc(515);\n\
        let patched = patch_jump_if_false(2, 260);\n\
        let mul_op = binary_flag_to_opcode(3);\n\
        let is_ret = is_seed_control_keyword(nameof(return));\n\
        let span = attach_debug_span(12, 100, 148);\n\
        let reg_bind = create_debug_reg_binding(17, 42, 7);\n\
        let pilc_summary = create_pilc_container_summary(4, 32, 18, 18, 3);\n\
        let header_ok = validate_pilc_v3_header(\"PIL3\", 3);\n\
        if op == 1 && dst == 5 && s1 == 2 && s2 == 3 && mul_op == 3 && is_ret && header_ok {{\n\
            span.word + reg_bind.symbol_id + pilc_summary.total_byte_size + ((jmp + patched) & 0)\n\
        }} else {{\n\
            -1\n\
        }};\n",
        vm_compiler, pilc_writer
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "Unexpected parse diagnostics: {:?}", arena.diagnostics);

    let binary = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(binary);
    let res = vm.run();
    // span.word (12) + reg_bind.symbol_id (42) + pilc_summary.total_byte_size (20 + 64 + 32 + 72 + 144 + 36 = 368) = 422
    assert_eq!(res.as_int(), Some(422));
}

#[test]
fn test_pilvm_debug_tables_and_purified_opcode_execution() {
    let src = "\
        let alpha = 20;\n\
        let beta = 22;\n\
        assert(alpha + beta == 42);\n\
        alpha + beta;\n";
    let arena = parse(src);
    assert!(arena.diagnostics.is_empty(), "Unexpected diagnostics: {:?}", arena.diagnostics);

    let binary = BytecodeCompiler::compile(&arena, arena.root);
    assert_eq!(binary.instructions.len(), binary.pc_spans.len());
    assert!(!binary.reg_symbols.is_empty(), "Expected register-to-SymbolId debug mappings");
    assert!(binary.span_for_pc(0).is_some(), "Expected valid ByteSpan for PC 0");

    // Verify assert(...) lowers to Opcode::EngineIntrinsic (no Opcode::Assert or Opcode::Print exists)
    assert!(
        binary.instructions.iter().any(|ins| ins.opcode() == Opcode::EngineIntrinsic),
        "assert(...) must lower to Opcode::EngineIntrinsic"
    );

    // Roundtrip serialize & load .pilc v3 binary
    let raw_bytes = binary.serialize();
    assert_eq!(&raw_bytes[0..4], PILC_V3_MAGIC);
    assert_eq!(u32::from_le_bytes(raw_bytes[4..8].try_into().unwrap()), PILC_V3_VERSION);

    let visited_pcs = Arc::new(Mutex::new(Vec::new()));
    let visited_clone = Arc::clone(&visited_pcs);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(binary);
    vm.set_hook(Some(Box::new(move |pc, _ins, _regs| {
        visited_clone.lock().unwrap().push(pc);
    })));
    let out = vm.run();
    assert_eq!(out.as_int(), Some(42));
    assert!(!visited_pcs.lock().unwrap().is_empty(), "VmHook must trace executed PCs");
}

#[test]
fn test_legacy_vm_compiler_deleted_and_line_budget_enforced() {
    assert!(
        !Path::new("../../crates/pilot-vm/src/compiler.rs").exists(),
        "Legacy crates/pilot-vm/src/compiler.rs must be deleted in Milestone 55.4"
    );
    let mut total_lines = 0usize;
    for file in ["instruction.rs", "pilc.rs", "vm.rs", "lib.rs"] {
        let p = format!("../../crates/pilot-vm/src/{}", file);
        let content = std::fs::read_to_string(&p).unwrap_or_else(|_| panic!("Missing {}", p));
        total_lines += content.lines().count();
    }
    assert!(
        total_lines <= 650,
        "crates/pilot-vm total LOC ({}) must be <= 650",
        total_lines
    );
}
