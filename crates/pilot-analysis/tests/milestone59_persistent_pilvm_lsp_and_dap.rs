use std::path::Path;
use pilot_analysis::{flatten_ast, handle_engine_intrinsic, PhysicsRegistry};
use pilot_parser::parse;
use pilot_rt::PilotVal;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

fn run_pilot_script(src: &str, method: &str, offset: usize) -> (PilotVal, String) {
    let arena = parse(src);
    assert!(
        arena.diagnostics.is_empty(),
        "Parse diagnostics: {:?}",
        arena.diagnostics
    );
    let (flat, syms) = flatten_ast(&arena, arena.root);
    let mut registry = PhysicsRegistry::new(flat, syms);
    registry.source_text = src.to_string();
    registry.lsp_method = method.to_string();
    registry.lsp_offset = offset as u32;
    let reg_ptr = &mut registry as *mut PhysicsRegistry;
    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(move |name, args| {
        if name == "fs_write_text" {
            Ok(PilotVal::from_int(1))
        } else if name == "fs_read_text" {
            Ok(PilotVal::from_str(""))
        } else {
            let reg = unsafe { &mut *reg_ptr };
            handle_engine_intrinsic(reg, name, args)
        }
    }));
    let chunk = BytecodeCompiler::compile(&arena, arena.root);
    vm.load_binary(chunk);
    let val = vm.run();
    (val, registry.lsp_result)
}

#[test]
fn test_m59_1_split_vscode_gen_and_lsp_handlers_under_1000_loc() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    assert!(
        !root.join("selfhost/ide_sync.pilot").exists(),
        "Monolithic selfhost/ide_sync.pilot must be deleted in M59.4"
    );

    let vscode_gen_path = root.join("selfhost/ide/vscode_gen.pilot");
    let handlers_path = root.join("selfhost/lsp/handlers.pilot");
    let vscode_gen_src = std::fs::read_to_string(&vscode_gen_path).unwrap();
    let handlers_src = std::fs::read_to_string(&handlers_path).unwrap();

    let vscode_lines = vscode_gen_src.lines().count();
    let handlers_lines = handlers_src.lines().count();
    assert!(vscode_lines <= 1000, "vscode_gen.pilot must be <= 1000 lines, got {}", vscode_lines);
    assert!(handlers_lines <= 1000, "handlers.pilot must be <= 1000 lines, got {}", handlers_lines);

    // Verify dumps_tm_language and dumps_package_json generate real JSON without fs_read_text pass-through
    let script = format!(
        "{}\nlet tm = dumps_tm_language(\"\", \"\", \"\", \"\", \"\", 2);\nlet pkg = dumps_package_json(\"1.1.51\", 2);\nlet lc = dumps_language_configuration(2);\nlet sn = dumps_snippets(2);\ntm + \"|||\" + pkg + \"|||\" + lc + \"|||\" + sn;\n",
        vscode_gen_src
    );
    let (val, _) = run_pilot_script(&script, "test", 0);
    let out = val.to_string_repr();
    assert!(out.contains("\"scopeName\": \"source.pilot\""));
    assert!(out.contains("\"displayName\": \"Pilot Language Support\""));
    assert!(out.contains("\"lineComment\": \"//\""));
    assert!(out.contains("Distributive Struct Block"));

    // Verify handlers.pilot uses nameof() for seed keywords and resolves hover/prepareRename
    assert!(handlers_src.contains("nameof(struct)"));
    assert!(handlers_src.contains("nameof(fn)"));
    let hover_target = "/// Computes rocket thrust.\npub fn compute_thrust(mass: i64) -> i64 {\n    return mass * 2;\n}\n";
    let hover_offset = hover_target.find("compute_thrust").unwrap() + 2;
    let full_hover_src = format!("{}\n{}", hover_target, handlers_src);
    let (_, lsp_res) = run_pilot_script(&full_hover_src, "textDocument/hover", hover_offset);
    assert!(lsp_res.contains("pub fn compute_thrust(mass: i64) -> i64"));
    assert!(lsp_res.contains("Computes rocket thrust."));
}

#[test]
fn test_m59_2_persistent_pilvm_lsp_and_incremental_ide_db() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let ide_db_src = std::fs::read_to_string(root.join("selfhost/lsp/ide_db.pilot")).unwrap();
    let lsp_server_src = std::fs::read_to_string(root.join("selfhost/lsp/server.pilot")).unwrap();

    let script = format!(
        "{}\n{}\n\
        let doc1 = \"pub fn calc(x: i64) -> i64 {{ return x + 10; }}\";\n\
        let doc2 = apply_incremental_doc_edit(\"pub fn calc(x: i64) -> i64 {{ return x + \", \"42\", \"; }}\");\n\
        let cutoff = should_early_cutoff(doc1, doc2);\n\
        let resp = route_lsp_request(\"initialize\", 1);\n\
        let framed = encode_lsp_frame(resp);\n\
        if cutoff == 1 {{ framed }} else {{ \"cutoff_failed\" }}\n",
        ide_db_src, lsp_server_src
    );
    let (val, _) = run_pilot_script(&script, "", 0);
    let framed = val.to_string_repr();
    assert!(framed.starts_with("Content-Length: "));
    assert!(framed.contains("\"hoverProvider\":true"));
    assert!(framed.contains("\"renameProvider\":{\"prepareProvider\":true}"));
}

#[test]
fn test_m59_3_and_4_vmhook_dap_debugger_and_retired_crates_purge() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();

    // Verify all 5 legacy crates are deleted from disk
    for retired in [
        "crates/pilot-dap",
        "crates/pilot-lsp",
        "crates/pilot-lsp-macros",
        "crates/pilot-db",
        "crates/pilot-incremental",
    ] {
        assert!(
            !root.join(retired).exists(),
            "{} must be retired and deleted in M59.4",
            retired
        );
    }

    let dap_server = std::fs::read_to_string(root.join("selfhost/dap/server.pilot")).unwrap();
    let vm_ctrl = std::fs::read_to_string(root.join("selfhost/dap/vm_controller.pilot")).unwrap();
    let inspector = std::fs::read_to_string(root.join("selfhost/dap/inspector.pilot")).unwrap();
    let repl_eval = std::fs::read_to_string(root.join("selfhost/dap/repl_eval.pilot")).unwrap();
    let cli_dbg = std::fs::read_to_string(root.join("selfhost/dap/cli_dbg.pilot")).unwrap();

    let script = format!(
        "{}\n{}\n{}\n{}\n{}\n\
        let init_resp = route_dap_command(\"initialize\", 1);\n\
        let pause_bp = should_pause_at_pc(0, 12, 10, 12, 1, 1);\n\
        let pause_over = should_pause_at_pc(2, 15, 12, 99, 2, 1);\n\
        let var_json = format_register_variable_json(\"counter\", 3, \"42\", -1);\n\
        let watch_fn = wrap_repl_watch_expr(\"counter + 1\");\n\
        let cmd_code = parse_cli_dbg_command(\"next\");\n\
        if pause_bp == 1 && pause_over == 0 && cmd_code == 2 {{\n\
            init_resp + \"|\" + var_json + \"|\" + watch_fn\n\
        }} else {{\n\
            \"dap_check_failed\"\n\
        }}\n",
        dap_server, vm_ctrl, inspector, repl_eval, cli_dbg
    );
    let (val, _) = run_pilot_script(&script, "", 0);
    let res = val.to_string_repr();
    assert!(res.contains("\"supportsConfigurationDoneRequest\":true"));
    assert!(res.contains("\"register\":\"r3\""));
    assert!(res.contains("fn __dap_repl_eval() { return counter + 1; }"));
}
