//! Milestone 58 Verification Suite:
//! Runtime Purification, Real Crypto, Wire DB & Polyglot — Retire `pilot-polyglot`.

use std::path::Path;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[test]
fn test_58_1_and_58_2_rfc_crypto_and_ed25519_forgery_rejection_on_pilvm() {
    let ffi_bg = std::fs::read_to_string("../../selfhost/ffi_bindgen.pilot").unwrap();
    let hash_src = std::fs::read_to_string("../../stdlib/crypto/hash.pilot").unwrap();
    let ed_src = std::fs::read_to_string("../../stdlib/crypto/ed25519.pilot").unwrap();

    let script = format!(
        "{}\n{}\n{}\n\
        let bind = generate_ffi_binding(\"pilot_rt\", \"pilot_rt_dup\", \"ptr\", 6, 64, 6, 64);\n\
        let d_empty = sha256_digest(\"\");\n\
        let d_abc = sha256_digest(\"abc\");\n\
        let b3_a = blake3_digest(\"hello\");\n\
        let b3_b = blake3_digest(\"world\");\n\
        let mac = hmac_sha256_tag(\"secret_key\", \"payload\");\n\
        let kp = ed25519_keypair_from_seed(987654321);\n\
        let sig = ed25519_sign_message(kp, \"transfer_100\");\n\
        let valid_ok = ed25519_verify_signature(kp.public_point, \"transfer_100\", sig.r_point, sig.s_scalar);\n\
        let forged_s = !ed25519_verify_signature(kp.public_point, \"transfer_100\", sig.r_point, sig.s_scalar + 1);\n\
        let forged_r = !ed25519_verify_signature(kp.public_point, \"transfer_100\", sig.r_point + 1, sig.s_scalar);\n\
        let forged_msg = !ed25519_verify_signature(kp.public_point, \"transfer_999\", sig.r_point, sig.s_scalar);\n\
        let forged_pk = !ed25519_verify_signature(kp.public_point + 1, \"transfer_100\", sig.r_point, sig.s_scalar);\n\
        if bind.c_prototype.len() > 20 && d_empty.hex_prefix == \"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\" && d_abc.hex_prefix == \"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\" && b3_a.h0 != b3_b.h0 && mac > 0 && valid_ok && forged_s && forged_r && forged_msg && forged_pk {{\n\
            42\n\
        }} else {{\n\
            -1\n\
        }};\n",
        ffi_bg, hash_src, ed_src
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "Unexpected parse diagnostics: {:?}", arena.diagnostics);
    let binary = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(binary);
    let res = vm.run();
    assert_eq!(res.as_int(), Some(42));

    // Also verify OS CSPRNG syscall `pilot_rt_os_getrandom`
    let mut entropy = [0u8; 32];
    let n = unsafe { pilot_rt::pilot_rt_os_getrandom(entropy.as_mut_ptr(), entropy.len()) };
    assert_eq!(n, 32);
    assert!(entropy.iter().any(|&b| b != 0), "CSPRNG must produce non-zero random bytes");
}

#[test]
fn test_58_3_and_58_4_reactive_effects_polyglot_and_wire_db_on_pilvm() {
    let reactive = std::fs::read_to_string("../../stdlib/core/reactive.pilot").unwrap();
    let effects = std::fs::read_to_string("../../stdlib/core/effects.pilot").unwrap();
    let shm = std::fs::read_to_string("../../stdlib/polyglot/shm.pilot").unwrap();
    let marshal = std::fs::read_to_string("../../stdlib/polyglot/marshal.pilot").unwrap();
    let bridge = std::fs::read_to_string("../../stdlib/polyglot/bridge.pilot").unwrap();
    let sql = std::fs::read_to_string("../../stdlib/db/sql_engine.pilot").unwrap();
    let pg = std::fs::read_to_string("../../stdlib/db/postgres_wire.pilot").unwrap();
    let my = std::fs::read_to_string("../../stdlib/db/mysql_wire.pilot").unwrap();

    let script = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n\
        let sig0 = create_reactive_signal(1, 10);\n\
        let sig1 = update_reactive_signal(sig0, 25);\n\
        let ef0 = begin_effect_dry_run(100);\n\
        let ef1 = stage_effect_write(ef0, 250);\n\
        let ef_rb = rollback_effect_dry_run(ef1);\n\
        let ef_cm = commit_effect_dry_run(ef1);\n\
        let buf = create_shm_owned_buffer(4096, 128, 8);\n\
        let view = project_shm_borrowed_view(buf, 4, 16);\n\
        let m_int = marshal_int_binding(\"count\", \"42\");\n\
        let py_plan = build_polyglot_execution_plan(\"python\", m_int.json_payload, \"print(count)\");\n\
        let q_plan = build_sql_select_plan(\"users\", \"id, name\", \"active = 1\");\n\
        let pg_pkt = encode_postgres_query_packet(q_plan.sql_statement);\n\
        let my_pkt = encode_mysql_query_packet(1, q_plan.sql_statement);\n\
        if sig1.current_val == 25 && sig1.is_dirty && read_effect_value(ef_rb) == 100 && read_effect_value(ef_cm) == 250 && view.base_addr == 4128 && view.lattice_bits == 1 && py_plan.host_binary == \"python3\" && pg_pkt.msg_tag_byte == 81 && my_pkt.command_byte == 3 {{\n\
            sig1.current_val + read_effect_value(ef_cm) + my_pkt.command_byte\n\
        }} else {{\n\
            -1\n\
        }};\n",
        reactive, effects, shm, marshal, bridge, sql, pg, my
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "Unexpected parse diagnostics: {:?}", arena.diagnostics);
    let binary = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(binary);
    let res = vm.run();
    // 25 + 250 + 3 = 278
    assert_eq!(res.as_int(), Some(278));
}

#[test]
fn test_58_5_legacy_polyglot_and_crypto_deleted_and_line_caps_enforced() {
    assert!(
        !Path::new("../../crates/pilot-polyglot").exists(),
        "crates/pilot-polyglot must be deleted from disk in Milestone 58.5"
    );
    assert!(
        !Path::new("../../crates/pilot-rt/src/crypto.rs").exists(),
        "crates/pilot-rt/src/crypto.rs must be deleted from disk in Milestone 58.5"
    );

    let mut rt_lines = 0usize;
    for f in ["c_abi.rs", "concurrency.rs", "lib.rs", "memory.rs", "net.rs", "stdlib.rs", "time.rs", "value.rs"] {
        let p = format!("../../crates/pilot-rt/src/{}", f);
        rt_lines += std::fs::read_to_string(&p).unwrap().lines().count();
    }
    assert!(rt_lines <= 700, "crates/pilot-rt total LOC ({}) must be <= 700", rt_lines);

    let ffi_lines = std::fs::read_to_string("../../crates/pilot-ffi/src/lib.rs").unwrap().lines().count();
    assert!(ffi_lines <= 300, "crates/pilot-ffi total LOC ({}) must be <= 300", ffi_lines);
}
