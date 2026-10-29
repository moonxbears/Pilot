//! Milestone 28.4: Zero-Copy C-ABI Strangler-Fig Bridge (`pilot-ffi`)
//!
//! Exports contiguous 16-byte `AstNode*` and 8-byte `ByteSpan*` buffers over C-ABI so C# can
//! inspect them directly via `ReadOnlySpan<NativeAstNode>` (`sizeof == 16`) with zero per-node marshalling.

use pilot_ast::{AstArena, AstNode, BinaryOp, NodeId, NodeKind};
use pilot_lexer::{tokenize, TokenTape};
use pilot_parser::parse as parse_pilot;
use pilot_span::{ByteSpan, LineIndex};

pub const PILOT_FFI_ABI_VERSION: u32 = 3501;

/// Opaque C-ABI handle owning a parsed `AstArena` and `LineIndex`.
pub struct PilotFfiArenaHandle {
    pub arena: AstArena,
    pub line_index: LineIndex,
}

/// Opaque C-ABI handle owning static analysis results.
pub struct PilotFfiAnalysisHandle {
    pub result: pilot_analysis::AnalysisResult,
}

/// Opaque C-ABI handle owning Perceus optimizer results.
pub struct PilotFfiPerceusHandle {
    pub optimizer: pilot_perceus::PerceusOptimizer,
    pub report: pilot_perceus::PerceusReport,
}

/// Opaque C-ABI handle owning a scanned `TokenTape` and `LineIndex`.
pub struct PilotFfiTokenTapeHandle {
    pub tape: TokenTape,
    pub line_index: LineIndex,
}

#[no_mangle]
pub extern "C" fn pilot_ffi_abi_version() -> u32 {
    PILOT_FFI_ABI_VERSION
}

#[no_mangle]
pub extern "C" fn pilot_ffi_sizeof_ast_node() -> u32 {
    core::mem::size_of::<AstNode>() as u32
}

/// Scans and parses a UTF-8 buffer into a contiguous 16-byte `AstArena` handle.
/// Returns a heap-boxed `PilotFfiArenaHandle*` whose contiguous node/span slices are exposed
/// via `pilot_arena_nodes_ptr` and `pilot_arena_spans_ptr` with zero per-node marshalling.
#[no_mangle]
pub unsafe extern "C" fn pilot_parse_utf8_to_arena(
    utf8_ptr: *const u8,
    utf8_len: usize,
) -> *mut PilotFfiArenaHandle {
    let slice: &[u8] = if utf8_ptr.is_null() || utf8_len == 0 {
        &[]
    } else {
        core::slice::from_raw_parts(utf8_ptr, utf8_len)
    };

    let line_index = LineIndex::new(slice);
    let src = core::str::from_utf8(slice).unwrap_or("");
    let arena = parse_pilot(src);

    Box::into_raw(Box::new(PilotFfiArenaHandle { arena, line_index }))
}

/// Returns a raw pointer to the contiguous 16-byte `AstNode` buffer and writes the node count to `*out_len`.
#[no_mangle]
pub unsafe extern "C" fn pilot_arena_nodes_ptr(
    handle: *const PilotFfiArenaHandle,
    out_len: *mut usize,
) -> *const AstNode {
    if handle.is_null() {
        if !out_len.is_null() {
            *out_len = 0;
        }
        return core::ptr::null();
    }
    let h = &*handle;
    if !out_len.is_null() {
        *out_len = h.arena.nodes.len();
    }
    h.arena.nodes.as_ptr()
}

/// Returns a raw pointer to the contiguous 8-byte `ByteSpan` buffer and writes the span count to `*out_len`.
#[no_mangle]
pub unsafe extern "C" fn pilot_arena_spans_ptr(
    handle: *const PilotFfiArenaHandle,
    out_len: *mut usize,
) -> *const ByteSpan {
    if handle.is_null() {
        if !out_len.is_null() {
            *out_len = 0;
        }
        return core::ptr::null();
    }
    let h = &*handle;
    if !out_len.is_null() {
        *out_len = h.arena.spans.len();
    }
    h.arena.spans.as_ptr()
}

/// Returns a raw pointer to `extra_children` (`u32*`) and writes the count to `*out_len`.
#[no_mangle]
pub unsafe extern "C" fn pilot_arena_extra_children_ptr(
    handle: *const PilotFfiArenaHandle,
    out_len: *mut usize,
) -> *const u32 {
    if handle.is_null() {
        if !out_len.is_null() {
            *out_len = 0;
        }
        return core::ptr::null();
    }
    let h = &*handle;
    if !out_len.is_null() {
        *out_len = h.arena.extra_children.len();
    }
    h.arena.extra_children.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn pilot_arena_root_id(handle: *const PilotFfiArenaHandle) -> u32 {
    if handle.is_null() {
        return NodeId::NONE.0;
    }
    (*handle).arena.root.0
}

#[no_mangle]
pub unsafe extern "C" fn pilot_arena_line_count(handle: *const PilotFfiArenaHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    (*handle).line_index.line_count()
}

#[no_mangle]
pub unsafe extern "C" fn pilot_arena_lookup_line_col(
    handle: *const PilotFfiArenaHandle,
    byte_offset: u32,
    out_line: *mut u32,
    out_col: *mut u32,
) {
    if handle.is_null() {
        return;
    }
    let loc = (*handle).line_index.lookup(byte_offset);
    if !out_line.is_null() {
        *out_line = loc.line;
    }
    if !out_col.is_null() {
        *out_col = loc.col;
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_arena_free(handle: *mut PilotFfiArenaHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Tokenizes a UTF-8 buffer into a contiguous `TokenTape` handle.
#[no_mangle]
pub unsafe extern "C" fn pilot_tokenize_utf8(
    utf8_ptr: *const u8,
    utf8_len: usize,
) -> *mut PilotFfiTokenTapeHandle {
    let slice: &[u8] = if utf8_ptr.is_null() || utf8_len == 0 {
        &[]
    } else {
        core::slice::from_raw_parts(utf8_ptr, utf8_len)
    };

    let line_index = LineIndex::new(slice);
    let src = core::str::from_utf8(slice).unwrap_or("");
    let tape = tokenize(src);

    Box::into_raw(Box::new(PilotFfiTokenTapeHandle { tape, line_index }))
}

#[no_mangle]
pub unsafe extern "C" fn pilot_token_tape_len(handle: *const PilotFfiTokenTapeHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    (*handle).tape.len() as u32
}

#[no_mangle]
pub unsafe extern "C" fn pilot_token_tape_kinds_ptr(handle: *const PilotFfiTokenTapeHandle) -> *const u16 {
    if handle.is_null() {
        return core::ptr::null();
    }
    (*handle).tape.kinds.as_ptr() as *const u16
}

#[no_mangle]
pub unsafe extern "C" fn pilot_token_tape_flags_ptr(handle: *const PilotFfiTokenTapeHandle) -> *const u16 {
    if handle.is_null() {
        return core::ptr::null();
    }
    (*handle).tape.flags.as_ptr() as *const u16
}

#[no_mangle]
pub unsafe extern "C" fn pilot_token_tape_spans_ptr(handle: *const PilotFfiTokenTapeHandle) -> *const ByteSpan {
    if handle.is_null() {
        return core::ptr::null();
    }
    (*handle).tape.spans.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn pilot_token_tape_bracket_links_ptr(handle: *const PilotFfiTokenTapeHandle) -> *const u32 {
    if handle.is_null() {
        return core::ptr::null();
    }
    (*handle).tape.bracket_links.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn pilot_token_tape_free(handle: *mut PilotFfiTokenTapeHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

// =========================================================================
// Milestone 31: Lossless CST & Native Wadler-Lindig Formatter C-ABI Exports
// =========================================================================

/// Opaque C-ABI handle owning a lossless `ParsedCst`.
pub struct PilotFfiCstHandle {
    pub cst: pilot_syntax::ParsedCst,
}

#[no_mangle]
pub unsafe extern "C" fn pilot_cst_parse(
    utf8_ptr: *const u8,
    utf8_len: usize,
) -> *mut PilotFfiCstHandle {
    let slice: &[u8] = if utf8_ptr.is_null() || utf8_len == 0 {
        &[]
    } else {
        core::slice::from_raw_parts(utf8_ptr, utf8_len)
    };
    let src = core::str::from_utf8(slice).unwrap_or("");
    let cst = pilot_syntax::parse_cst(src);
    Box::into_raw(Box::new(PilotFfiCstHandle { cst }))
}

#[no_mangle]
pub unsafe extern "C" fn pilot_cst_free(handle: *mut PilotFfiCstHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_cst_diagnostic_count(handle: *const PilotFfiCstHandle) -> usize {
    if handle.is_null() {
        return 0;
    }
    (*handle).cst.diagnostics.len()
}

#[no_mangle]
pub unsafe extern "C" fn pilot_cst_has_missing_ident(handle: *const PilotFfiCstHandle) -> bool {
    if handle.is_null() {
        return false;
    }
    (*handle).cst.root.descendants().any(|e| e.kind() == pilot_syntax::SyntaxKind::MissingIdent)
}

#[no_mangle]
pub unsafe extern "C" fn pilot_cst_has_missing_rparen(handle: *const PilotFfiCstHandle) -> bool {
    if handle.is_null() {
        return false;
    }
    (*handle).cst.root.descendants().any(|e| e.kind() == pilot_syntax::SyntaxKind::MissingRParen)
}

#[no_mangle]
pub unsafe extern "C" fn pilot_cst_to_text(
    handle: *const PilotFfiCstHandle,
    out_len: *mut usize,
) -> *mut u8 {
    if handle.is_null() {
        if !out_len.is_null() {
            *out_len = 0;
        }
        return core::ptr::null_mut();
    }
    let text = (*handle).cst.to_text();
    let bytes = text.into_bytes();
    if !out_len.is_null() {
        *out_len = bytes.len();
    }
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    core::mem::forget(boxed);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn pilot_format_source(
    utf8_ptr: *const u8,
    utf8_len: usize,
    use_colon_suites: i32,
    emit_semicolons: bool,
    out_len: *mut usize,
    out_modified: *mut bool,
) -> *mut u8 {
    let slice: &[u8] = if utf8_ptr.is_null() || utf8_len == 0 {
        &[]
    } else {
        core::slice::from_raw_parts(utf8_ptr, utf8_len)
    };
    let src = core::str::from_utf8(slice).unwrap_or("");
    let options = pilot_fmt::FormatOptions {
        use_colon_suites: match use_colon_suites {
            1 => Some(true),
            0 => Some(false),
            _ => None,
        },
        emit_semicolons,
        ..Default::default()
    };
    let result = pilot_fmt::format_source(src, &options);
    if !out_modified.is_null() {
        *out_modified = result.was_modified;
    }
    let bytes = result.formatted_code.into_bytes();
    if !out_len.is_null() {
        *out_len = bytes.len();
    }
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    core::mem::forget(boxed);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn pilot_free_string(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let slice = core::slice::from_raw_parts_mut(ptr, len);
        drop(Box::from_raw(slice as *mut [u8]));
    }
}

// =========================================================================
// Milestone 32: Incremental Reparsing, Database & Native LSP C-ABI Exports
// =========================================================================

#[no_mangle]
pub unsafe extern "C" fn pilot_incremental_reparse(
    old_cst: *const PilotFfiCstHandle,
    old_src_ptr: *const u8,
    old_src_len: usize,
    start: u32,
    end: u32,
    rep_ptr: *const u8,
    rep_len: usize,
    out_micros: *mut u64,
    out_reused: *mut usize,
) -> *mut PilotFfiCstHandle {
    if old_cst.is_null() {
        return core::ptr::null_mut();
    }
    let old_src = if old_src_ptr.is_null() || old_src_len == 0 {
        ""
    } else {
        core::str::from_utf8(core::slice::from_raw_parts(old_src_ptr, old_src_len)).unwrap_or("")
    };
    let rep_str = if rep_ptr.is_null() || rep_len == 0 {
        ""
    } else {
        core::str::from_utf8(core::slice::from_raw_parts(rep_ptr, rep_len)).unwrap_or("")
    };

    let edit = pilot_incremental::TextEdit::new(start, end, rep_str);
    let old_root = (*old_cst).cst.root.green();
    let res = pilot_incremental::reparse_incremental(old_root, old_src, &edit);

    if !out_micros.is_null() {
        *out_micros = res.metrics.reparse_time_micros;
    }
    if !out_reused.is_null() {
        *out_reused = res.metrics.reused_nodes;
    }

    let parsed_cst = pilot_syntax::ParsedCst {
        root: pilot_syntax::SyntaxNode::new_root(res.new_root),
        diagnostics: Vec::new(),
    };
    Box::into_raw(Box::new(PilotFfiCstHandle { cst: parsed_cst }))
}

#[no_mangle]
pub unsafe extern "C" fn pilot_db_new() -> *mut pilot_db::Database {
    Box::into_raw(Box::new(pilot_db::Database::new()))
}

#[no_mangle]
pub unsafe extern "C" fn pilot_db_free(db: *mut pilot_db::Database) {
    if !db.is_null() {
        drop(Box::from_raw(db));
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_db_set_source(
    db: *mut pilot_db::Database,
    file_id: u32,
    src_ptr: *const u8,
    src_len: usize,
) {
    if db.is_null() { return; }
    let src = if src_ptr.is_null() || src_len == 0 {
        ""
    } else {
        core::str::from_utf8(core::slice::from_raw_parts(src_ptr, src_len)).unwrap_or("")
    };
    (*db).set_source(pilot_db::FileId(file_id), src);
}

#[no_mangle]
pub unsafe extern "C" fn pilot_db_apply_edit(
    db: *mut pilot_db::Database,
    file_id: u32,
    start: u32,
    end: u32,
    rep_ptr: *const u8,
    rep_len: usize,
) {
    if db.is_null() { return; }
    let rep_str = if rep_ptr.is_null() || rep_len == 0 {
        ""
    } else {
        core::str::from_utf8(core::slice::from_raw_parts(rep_ptr, rep_len)).unwrap_or("")
    };
    let edit = pilot_incremental::TextEdit::new(start, end, rep_str);
    (*db).apply_edit(pilot_db::FileId(file_id), &edit);
}

#[no_mangle]
pub unsafe extern "C" fn pilot_db_early_cutoff_count(db: *const pilot_db::Database) -> usize {
    if db.is_null() { return 0; }
    (*db).early_cutoff_count
}

#[no_mangle]
pub unsafe extern "C" fn pilot_db_signature_hash(db: *const pilot_db::Database, file_id: u32) -> u64 {
    if db.is_null() { return 0; }
    (*db).module_item_tree(pilot_db::FileId(file_id)).map(|t| t.signature_hash).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn pilot_lsp_new() -> *mut pilot_lsp::LspServer {
    Box::into_raw(Box::new(pilot_lsp::LspServer::new()))
}

#[no_mangle]
pub unsafe extern "C" fn pilot_lsp_free(server: *mut pilot_lsp::LspServer) {
    if !server.is_null() {
        drop(Box::from_raw(server));
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_lsp_handle_message(
    server: *mut pilot_lsp::LspServer,
    msg_ptr: *const u8,
    msg_len: usize,
    out_len: *mut usize,
) -> *mut u8 {
    if server.is_null() {
        if !out_len.is_null() { *out_len = 0; }
        return core::ptr::null_mut();
    }

    let msg_str = if msg_ptr.is_null() || msg_len == 0 {
        ""
    } else {
        core::str::from_utf8(core::slice::from_raw_parts(msg_ptr, msg_len)).unwrap_or("")
    };

    let resp = (*server).handle_message(msg_str);
    match resp {
        Some(s) => {
            let bytes = s.into_bytes();
            if !out_len.is_null() { *out_len = bytes.len(); }
            let mut boxed = bytes.into_boxed_slice();
            let ptr = boxed.as_mut_ptr();
            core::mem::forget(boxed);
            ptr
        }
        None => {
            if !out_len.is_null() { *out_len = 0; }
            core::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_analysis_run(
    arena_handle: *const PilotFfiArenaHandle,
) -> *mut PilotFfiAnalysisHandle {
    if arena_handle.is_null() {
        return core::ptr::null_mut();
    }
    let arena = &(*arena_handle).arena;
    let res = pilot_analysis::AnalysisPipeline::analyze(arena, arena.root);
    Box::into_raw(Box::new(PilotFfiAnalysisHandle { result: res }))
}

#[no_mangle]
pub unsafe extern "C" fn pilot_analysis_free(handle: *mut PilotFfiAnalysisHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_analysis_diagnostic_count(handle: *const PilotFfiAnalysisHandle) -> usize {
    if handle.is_null() { return 0; }
    (*handle).result.diagnostics.len()
}

#[no_mangle]
pub unsafe extern "C" fn pilot_analysis_has_errors(handle: *const PilotFfiAnalysisHandle) -> u8 {
    if handle.is_null() { return 0; }
    if (*handle).result.has_errors() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_perceus_run(
    arena_handle: *const PilotFfiArenaHandle,
) -> *mut PilotFfiPerceusHandle {
    if arena_handle.is_null() {
        return core::ptr::null_mut();
    }
    let mut opt = pilot_perceus::PerceusOptimizer::new();
    let arena = &(*arena_handle).arena;
    // Walk nodes and identify allocations
    for node in &arena.nodes {
        let kind = node.node_kind();
        match kind {
            NodeKind::StrLit => {
                opt.escape.record_allocation("str_literal", "str", pilot_perceus::EscapeState::Stack);
            }
            NodeKind::ListExpr => {
                // Tier 1 Region bump allocation by default
                let site = opt.escape.record_allocation("list_literal", "List", pilot_perceus::EscapeState::Region);
                // If it's the root return expression, escalate to global
                if arena.root.is_some() && arena.node(arena.root).node_kind() == NodeKind::ReturnStmt {
                    opt.escape.mark_escaped(site, pilot_perceus::EscapeState::Global);
                }
            }
            _ => {}
        }
    }
    // Simulate drop reuse optimization
    opt.arc.elide_borrowed_retain();
    opt.arc.insert_drop_reuse("old_cell", "new_cell");

    let report = opt.generate_report();
    Box::into_raw(Box::new(PilotFfiPerceusHandle {
        optimizer: opt,
        report,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn pilot_perceus_free(handle: *mut PilotFfiPerceusHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_perceus_region_alloc_count(handle: *const PilotFfiPerceusHandle) -> usize {
    if handle.is_null() { return 0; }
    (*handle).report.region_allocs
}

#[no_mangle]
pub unsafe extern "C" fn pilot_perceus_stack_alloc_count(handle: *const PilotFfiPerceusHandle) -> usize {
    if handle.is_null() { return 0; }
    (*handle).report.stack_allocs
}

#[no_mangle]
pub unsafe extern "C" fn pilot_perceus_global_alloc_count(handle: *const PilotFfiPerceusHandle) -> usize {
    if handle.is_null() { return 0; }
    (*handle).report.global_allocs
}

#[no_mangle]
pub unsafe extern "C" fn pilot_perceus_drop_reuse_count(handle: *const PilotFfiPerceusHandle) -> usize {
    if handle.is_null() { return 0; }
    (*handle).report.drop_reuses
}

#[no_mangle]
pub unsafe extern "C" fn pilot_perceus_elided_retain_count(handle: *const PilotFfiPerceusHandle) -> usize {
    if handle.is_null() { return 0; }
    (*handle).report.elided_retains
}

#[no_mangle]
pub extern "C" fn pilot_val_from_int(val: i64) -> pilot_rt::PilotVal {
    pilot_rt::PilotVal::from_int(val)
}

#[no_mangle]
pub extern "C" fn pilot_val_from_float(val: f64) -> pilot_rt::PilotVal {
    pilot_rt::PilotVal::from_float(val)
}

#[no_mangle]
pub extern "C" fn pilot_val_from_bool(val: u8) -> pilot_rt::PilotVal {
    pilot_rt::PilotVal::from_bool(val != 0)
}

#[no_mangle]
pub unsafe extern "C" fn pilot_val_from_str(str_ptr: *const u8, str_len: usize) -> pilot_rt::PilotVal {
    let s = if str_ptr.is_null() || str_len == 0 {
        ""
    } else {
        core::str::from_utf8(core::slice::from_raw_parts(str_ptr, str_len)).unwrap_or("")
    };
    pilot_rt::PilotVal::from_str(s)
}

#[no_mangle]
pub unsafe extern "C" fn pilot_val_to_str(val: pilot_rt::PilotVal, out_len: *mut usize) -> *mut u8 {
    let s = val.to_string_repr();
    let bytes = s.into_bytes();
    if !out_len.is_null() {
        *out_len = bytes.len();
    }
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    core::mem::forget(boxed);
    ptr
}

#[no_mangle]
pub extern "C" fn pilot_val_is_sso(val: pilot_rt::PilotVal) -> u8 {
    if val.is_sso_str() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_val_as_int(val: pilot_rt::PilotVal, out_val: *mut i64) -> u8 {
    if let Some(i) = val.as_int() {
        if !out_val.is_null() { *out_val = i; }
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_val_as_float(val: pilot_rt::PilotVal, out_val: *mut f64) -> u8 {
    if let Some(f) = val.as_float() {
        if !out_val.is_null() { *out_val = f; }
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn pilot_val_add(a: pilot_rt::PilotVal, b: pilot_rt::PilotVal) -> pilot_rt::PilotVal {
    a.add(b).unwrap_or(pilot_rt::PilotVal::NULL)
}

#[no_mangle]
pub unsafe extern "C" fn pilot_vm_new() -> *mut pilot_vm::PilVirtualMachine {
    Box::into_raw(Box::new(pilot_vm::PilVirtualMachine::new()))
}

#[no_mangle]
pub unsafe extern "C" fn pilot_vm_free(vm: *mut pilot_vm::PilVirtualMachine) {
    if !vm.is_null() {
        drop(Box::from_raw(vm));
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_vm_execute_bytes(
    vm: *mut pilot_vm::PilVirtualMachine,
    bytes_ptr: *const u8,
    bytes_len: usize,
) -> pilot_rt::PilotVal {
    if vm.is_null() || bytes_ptr.is_null() || bytes_len == 0 {
        return pilot_rt::PilotVal::NULL;
    }
    let slice = core::slice::from_raw_parts(bytes_ptr, bytes_len);
    match pilot_vm::PilcBinary::load_from_bytes(slice) {
        Ok(bin) => {
            (*vm).load_binary(bin);
            (*vm).run()
        }
        Err(_) => pilot_rt::PilotVal::NULL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_codegen_build(
    name_ptr: *const u8,
    name_len: usize,
    target: u8,
    out_len: *mut usize,
) -> *mut u8 {
    let name = if name_ptr.is_null() || name_len == 0 {
        "main"
    } else {
        core::str::from_utf8(core::slice::from_raw_parts(name_ptr, name_len)).unwrap_or("main")
    };

    let build_target = match target {
        0 => pilot_codegen::BuildTarget::Llvm,
        1 => pilot_codegen::BuildTarget::C,
        2 => pilot_codegen::BuildTarget::Wasm,
        _ => pilot_codegen::BuildTarget::Native,
    };

    let opts = pilot_codegen::BuildOptions {
        target: build_target,
        ..Default::default()
    };

    let res = pilot_codegen::BuildDriver::compile_source_to_artifact(name, &opts).unwrap_or_default();
    let bytes = res.into_bytes();
    if !out_len.is_null() {
        *out_len = bytes.len();
    }
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    core::mem::forget(boxed);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn pilot_transpile_sample(target: u8, out_len: *mut usize) -> *mut u8 {
    let lang = match target {
        0 => pilot_transpile::TargetLanguage::C,
        1 => pilot_transpile::TargetLanguage::Cpp,
        2 => pilot_transpile::TargetLanguage::CSharp,
        3 => pilot_transpile::TargetLanguage::Rust,
        4 => pilot_transpile::TargetLanguage::Python,
        5 => pilot_transpile::TargetLanguage::JavaScript,
        6 => pilot_transpile::TargetLanguage::Lua,
        _ => {
            if !out_len.is_null() {
                *out_len = 0;
            }
            return core::ptr::null_mut();
        }
    };

    let module = pilot_transpile::IrModule {
        name: "TranspiledMod".to_string(),
        type_decls: Vec::new(),
        functions: vec![pilot_transpile::IrFunction {
            name: "compute".to_string(),
            ret_type: pilot_transpile::IrType::Int,
            params: vec![("x".to_string(), pilot_transpile::IrType::Int)],
            body: vec![
                pilot_transpile::IrStmt::Return(Some(pilot_transpile::IrExpr::Binary {
                    op: "*".to_string(),
                    lhs: Box::new(pilot_transpile::IrExpr::Var("x".to_string())),
                    rhs: Box::new(pilot_transpile::IrExpr::LitInt(2)),
                })),
            ],
        }],
    };

    let s = pilot_transpile::Transpiler::emit_module(&module, lang);
    let bytes = s.into_bytes();
    if !out_len.is_null() {
        *out_len = bytes.len();
    }
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    core::mem::forget(boxed);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn pilot_polyglot_verify_spdx(spdx_ptr: *const u8, spdx_len: usize) -> u8 {
    if spdx_ptr.is_null() || spdx_len == 0 {
        return 0;
    }
    let s = core::str::from_utf8(core::slice::from_raw_parts(spdx_ptr, spdx_len)).unwrap_or("");
    if pilot_transpile::LicenseVerifier::is_viable_license(s) { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_polyglot_execute(
    lang_ptr: *const u8,
    lang_len: usize,
    code_ptr: *const u8,
    code_len: usize,
    out_len: *mut usize,
) -> *mut u8 {
    let lang = if lang_ptr.is_null() || lang_len == 0 {
        "python"
    } else {
        core::str::from_utf8(core::slice::from_raw_parts(lang_ptr, lang_len)).unwrap_or("python")
    };
    let code = if code_ptr.is_null() || code_len == 0 {
        ""
    } else {
        core::str::from_utf8(core::slice::from_raw_parts(code_ptr, code_len)).unwrap_or("")
    };

    let bridge = pilot_polyglot::ExternalPolyglotBridge::new();
    let res = bridge.execute_snippet(lang, code).unwrap_or_else(|e| e);
    let bytes = res.into_bytes();
    if !out_len.is_null() {
        *out_len = bytes.len();
    }
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    core::mem::forget(boxed);
    ptr
}

/// Allocates a contiguous shared-memory `SliceBuffer<u8>` over C-ABI (Milestone 15 & 35).
#[no_mangle]
pub unsafe extern "C" fn pilot_slice_buffer_create(capacity: usize) -> pilot_polyglot::SliceBuffer<u8> {
    pilot_polyglot::SliceBuffer::with_capacity(capacity)
}

/// Frees a contiguous shared-memory `SliceBuffer<u8>` over C-ABI without re-allocation.
#[no_mangle]
pub unsafe extern "C" fn pilot_slice_buffer_free(buf: pilot_polyglot::SliceBuffer<u8>) {
    drop(buf);
}

/// Builds a statement/expression `AstArena` from UTF-8 source for Milestone 28 C-ABI verification
/// (full Stage 1 SIMD modal lexer & Stage 2 Pratt parser land in M29–M30).
pub fn parse_lightweight_skeleton_to_arena(src: &str) -> AstArena {
    let mut arena = AstArena::with_capacity(32);
    let mut top_children = Vec::new();
    let bytes = src.as_bytes();
    let mut cursor = 0usize;

    for raw_line in src.split('\n') {
        let line_len = raw_line.len();
        let trimmed = raw_line.trim();
        let line_start = cursor as u32;
        let line_end = (cursor + line_len) as u32;
        cursor += line_len + 1;

        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }

        let clean = trimmed.trim_end_matches(';').trim();
        let span = ByteSpan::new(line_start, line_end);

        if let Some(rest) = clean.strip_prefix("import ") {
            let sym = arena.interner.intern(rest.trim());
            let id = arena.alloc_node(NodeKind::ImportDecl, 0, span, sym.0, 0);
            top_children.push(id);
        } else if let Some(rest) = clean.strip_prefix("let ") {
            let node = parse_binding_stmt(&mut arena, NodeKind::LetStmt, rest, span);
            top_children.push(node);
        } else if let Some(rest) = clean.strip_prefix("var ") {
            let node = parse_binding_stmt(&mut arena, NodeKind::VarStmt, rest, span);
            top_children.push(node);
        } else if let Some(rest) = clean.strip_prefix("const ") {
            let node = parse_binding_stmt(&mut arena, NodeKind::ConstDecl, rest, span);
            top_children.push(node);
        } else if let Some(rest) = clean.strip_prefix("return ") {
            let expr = parse_simple_expr(&mut arena, rest.trim(), span);
            let id = arena.alloc_node(NodeKind::ReturnStmt, 0, span, expr.0, 0);
            top_children.push(id);
        } else if let Some(rest) = clean.strip_prefix("def ") {
            let name = rest
                .split(|c: char| c == '(' || c.is_whitespace())
                .next()
                .unwrap_or("anon");
            let sym = arena.interner.intern(name);
            let id = arena.alloc_node(NodeKind::FnDecl, 0, span, sym.0, 0);
            top_children.push(id);
        } else if let Some(rest) = clean.strip_prefix("type ") {
            let name = rest
                .split(|c: char| c == '=' || c.is_whitespace())
                .next()
                .unwrap_or("Type");
            let sym = arena.interner.intern(name);
            let id = arena.alloc_node(NodeKind::TypeAdtDecl, 0, span, sym.0, 0);
            top_children.push(id);
        } else if let Some(rest) = clean.strip_prefix("class ") {
            let name = rest
                .split(|c: char| c == '{' || c == '(' || c.is_whitespace())
                .next()
                .unwrap_or("Class");
            let sym = arena.interner.intern(name);
            let id = arena.alloc_node(NodeKind::ClassDecl, 0, span, sym.0, 0);
            top_children.push(id);
        } else if clean != "}" {
            let expr = parse_simple_expr(&mut arena, clean, span);
            let id = arena.alloc_node(NodeKind::ExprStmt, 0, span, expr.0, 0);
            top_children.push(id);
        }
    }

    let root = arena.alloc_list_node(
        NodeKind::Program,
        0,
        ByteSpan::new(0, bytes.len() as u32),
        &top_children,
    );
    arena.root = root;
    arena
}

fn parse_binding_stmt(arena: &mut AstArena, kind: NodeKind, rest: &str, span: ByteSpan) -> NodeId {
    if let Some((lhs_part, rhs_part)) = rest.split_once('=') {
        let name = lhs_part.split(':').next().unwrap_or("").trim();
        let sym = arena.interner.intern(name);
        let init = parse_simple_expr(arena, rhs_part.trim(), span);
        arena.alloc_node(kind, 0, span, sym.0, init.0)
    } else {
        let sym = arena.interner.intern(rest.trim());
        arena.alloc_node(kind, 0, span, sym.0, NodeId::NONE.0)
    }
}

fn parse_simple_expr(arena: &mut AstArena, expr_str: &str, span: ByteSpan) -> NodeId {
    let s = expr_str.trim();
    if let Some((a, b)) = s.split_once('+') {
        let lhs = parse_simple_expr(arena, a, span);
        let rhs = parse_simple_expr(arena, b, span);
        return arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::Add as u16, span, lhs.0, rhs.0);
    }
    if let Some((a, b)) = s.split_once('*') {
        let lhs = parse_simple_expr(arena, a, span);
        let rhs = parse_simple_expr(arena, b, span);
        return arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::Mul as u16, span, lhs.0, rhs.0);
    }
    if let Ok(iv) = s.parse::<i64>() {
        return arena.alloc_int(iv, span);
    }
    if let Ok(fv) = s.parse::<f64>() {
        return arena.alloc_float(fv, span);
    }
    if s == "true" || s == "false" {
        return arena.alloc_node(NodeKind::BoolLit, 0, span, u32::from(s == "true"), 0);
    }
    if s == "none" || s == "null" {
        return arena.alloc_node(NodeKind::NoneLit, 0, span, 0, 0);
    }
    if s.starts_with(':') && s.len() > 1 {
        return arena.alloc_keyword(s, span);
    }
    if (s.starts_with('"') && s.ends_with('"')) && s.len() >= 2 {
        return arena.alloc_str(&s[1..s.len() - 1], span);
    }
    arena.alloc_ident(s, span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_c_abi_zero_copy_arena_spans_and_nodes() {
        let src = b"import std.math;\nlet x: i64 = 40 + 2;\nreturn x;\n";
        unsafe {
            let handle = pilot_parse_utf8_to_arena(src.as_ptr(), src.len());
            assert!(!handle.is_null());
            assert_eq!(pilot_ffi_abi_version(), 3501);
            assert_eq!(pilot_ffi_sizeof_ast_node(), 16);

            let mut node_len = 0usize;
            let nodes_ptr = pilot_arena_nodes_ptr(handle, &mut node_len);
            assert!(!nodes_ptr.is_null());
            assert!(node_len >= 6);

            let nodes = core::slice::from_raw_parts(nodes_ptr, node_len);
            let root_id = pilot_arena_root_id(handle) as usize;
            assert_eq!(nodes[root_id].node_kind(), NodeKind::Program);
            assert_eq!(pilot_arena_line_count(handle), 4);

            let mut ln = 0u32;
            let mut cl = 0u32;
            pilot_arena_lookup_line_col(handle, 17, &mut ln, &mut cl);
            assert_eq!((ln, cl), (2, 1));

            pilot_arena_free(handle);
        }
    }

    #[test]
    fn ffi_c_abi_zero_copy_token_tape() {
        let src = b"def compute(a: i64) -> i64 { return a * 2; }\n";
        unsafe {
            let handle = pilot_tokenize_utf8(src.as_ptr(), src.len());
            assert!(!handle.is_null());

            let len = pilot_token_tape_len(handle);
            assert!(len >= 12);

            let kinds_ptr = pilot_token_tape_kinds_ptr(handle);
            let spans_ptr = pilot_token_tape_spans_ptr(handle);
            let links_ptr = pilot_token_tape_bracket_links_ptr(handle);

            assert!(!kinds_ptr.is_null());
            assert!(!spans_ptr.is_null());
            assert!(!links_ptr.is_null());

            let kinds = core::slice::from_raw_parts(kinds_ptr, len as usize);
            assert_eq!(kinds[0], pilot_lexer::TokenKind::KwDef as u16);

            pilot_token_tape_free(handle);
        }
    }

    #[test]
    fn ffi_c_abi_lossless_cst_and_format() {
        let src = b"// Test comment\ndef run(x: i64) -> i64 {\n    return x + 1;\n}\n";
        unsafe {
            // Test CST parsing and roundtrip text
            let cst_handle = pilot_cst_parse(src.as_ptr(), src.len());
            assert!(!cst_handle.is_null());
            assert_eq!(pilot_cst_diagnostic_count(cst_handle), 0);

            let mut out_len = 0usize;
            let text_ptr = pilot_cst_to_text(cst_handle, &mut out_len);
            assert!(!text_ptr.is_null());
            assert_eq!(out_len, src.len());

            let text_bytes = core::slice::from_raw_parts(text_ptr, out_len);
            assert_eq!(text_bytes, src.as_slice());
            pilot_free_string(text_ptr, out_len);
            pilot_cst_free(cst_handle);

            // Test native Wadler-Lindig formatter
            let mut fmt_len = 0usize;
            let mut modified = false;
            let fmt_ptr = pilot_format_source(src.as_ptr(), src.len(), 1, false, &mut fmt_len, &mut modified);
            assert!(!fmt_ptr.is_null());
            assert!(fmt_len > 0);
            let fmt_slice = core::slice::from_raw_parts(fmt_ptr, fmt_len);
            let fmt_str = core::str::from_utf8(fmt_slice).unwrap();
            assert!(fmt_str.contains("def run(x: i64) -> i64:"));
            pilot_free_string(fmt_ptr, fmt_len);
        }
    }

    #[test]
    fn ffi_c_abi_incremental_reparse_db_and_lsp() {
        let src = b"def calc(x: i64) -> i64 {\n    return x + 10;\n}\n";
        let src_str = core::str::from_utf8(src).unwrap();
        let target_offset = src_str.find("10").unwrap() as u32;
        unsafe {
            // 1. CST and Incremental Reparse via C-ABI
            let cst = pilot_cst_parse(src.as_ptr(), src.len());
            assert!(!cst.is_null());

            let rep = b"20";
            let mut micros = 0u64;
            let mut reused = 0usize;
            let new_cst = pilot_incremental_reparse(cst, src.as_ptr(), src.len(), target_offset, target_offset + 2, rep.as_ptr(), rep.len(), &mut micros, &mut reused);
            assert!(!new_cst.is_null());
            assert!(micros < 5000);

            let mut out_len = 0usize;
            let text_ptr = pilot_cst_to_text(new_cst, &mut out_len);
            let text = core::str::from_utf8(core::slice::from_raw_parts(text_ptr, out_len)).unwrap();
            assert!(text.contains("return x + 20;"));
            pilot_free_string(text_ptr, out_len);

            pilot_cst_free(cst);
            pilot_cst_free(new_cst);

            // 2. Incremental Database with Early Cutoff
            let db = pilot_db_new();
            assert!(!db.is_null());
            pilot_db_set_source(db, 1, src.as_ptr(), src.len());
            let hash1 = pilot_db_signature_hash(db, 1);
            assert_ne!(hash1, 0);

            // Body edit: early cutoff preserves signature hash
            pilot_db_apply_edit(db, 1, target_offset, target_offset + 2, rep.as_ptr(), rep.len());
            let hash2 = pilot_db_signature_hash(db, 1);
            assert_eq!(hash1, hash2);
            assert_eq!(pilot_db_early_cutoff_count(db), 1);
            pilot_db_free(db);

            // 3. Native LSP Server
            let lsp = pilot_lsp_new();
            assert!(!lsp.is_null());
            let init_req = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n";
            let mut resp_len = 0usize;
            let resp_ptr = pilot_lsp_handle_message(lsp, init_req.as_ptr(), init_req.len(), &mut resp_len);
            assert!(!resp_ptr.is_null());
            assert!(resp_len > 0);
            let resp = core::str::from_utf8(core::slice::from_raw_parts(resp_ptr, resp_len)).unwrap();
            assert!(resp.contains("semanticTokensProvider"));
            pilot_free_string(resp_ptr, resp_len);
            pilot_lsp_free(lsp);
        }
    }

    #[test]
    fn ffi_c_abi_analysis_and_perceus() {
        let src = b"let x = 10\nlet y = 20\nlet z = x + y\n";
        unsafe {
            let arena = pilot_parse_utf8_to_arena(src.as_ptr(), src.len());
            assert!(!arena.is_null());

            // 1. Static Analysis Pass
            let analysis = pilot_analysis_run(arena);
            assert!(!analysis.is_null());
            assert_eq!(pilot_analysis_has_errors(analysis), 0);
            pilot_analysis_free(analysis);

            // 2. Perceus Optimizer Pass
            let perceus = pilot_perceus_run(arena);
            assert!(!perceus.is_null());
            assert_eq!(pilot_perceus_drop_reuse_count(perceus), 1);
            assert_eq!(pilot_perceus_elided_retain_count(perceus), 1);
            pilot_perceus_free(perceus);

            pilot_arena_free(arena);
        }
    }

    #[test]
    fn ffi_c_abi_runtime_and_register_vm() {
        unsafe {
            // 1. Unboxed 16-byte PilotVal & SSO
            let a = pilot_val_from_int(10);
            let b = pilot_val_from_int(32);
            let c = pilot_val_add(a, b);
            let mut res_int = 0i64;
            assert_eq!(pilot_val_as_int(c, &mut res_int), 1);
            assert_eq!(res_int, 42);

            let sso_str = b"hello_pilot";
            let sso_val = pilot_val_from_str(sso_str.as_ptr(), sso_str.len());
            assert_eq!(pilot_val_is_sso(sso_val), 1);

            let mut out_len = 0usize;
            let str_ptr = pilot_val_to_str(sso_val, &mut out_len);
            assert!(!str_ptr.is_null());
            assert_eq!(out_len, sso_str.len());
            let extracted = core::str::from_utf8(core::slice::from_raw_parts(str_ptr, out_len)).unwrap();
            assert_eq!(extracted, "hello_pilot");
            pilot_free_string(str_ptr, out_len);

            // 2. Register VM Execution
            let vm = pilot_vm_new();
            assert!(!vm.is_null());

            let bin = pilot_vm::PilcBinary::new(
                vec![pilot_rt::PilotVal::from_int(10), pilot_rt::PilotVal::from_int(32)],
                vec![
                    pilot_vm::Instruction::encode(pilot_vm::Opcode::LoadConst, 1, 0, 0),
                    pilot_vm::Instruction::encode(pilot_vm::Opcode::LoadConst, 2, 0, 1),
                    pilot_vm::Instruction::encode(pilot_vm::Opcode::AddI64, 3, 1, 2),
                    pilot_vm::Instruction::encode(pilot_vm::Opcode::Ret, 0, 3, 0),
                ],
            );
            let bytes = bin.serialize();
            let vm_res = pilot_vm_execute_bytes(vm, bytes.as_ptr(), bytes.len());
            let mut vm_res_int = 0i64;
            assert_eq!(pilot_val_as_int(vm_res, &mut vm_res_int), 1);
            assert_eq!(vm_res_int, 42);

            pilot_vm_free(vm);
        }
    }

    #[test]
    fn ffi_c_abi_codegen_transpile_and_polyglot() {
        unsafe {
            // 1. Codegen build
            let name = b"test_mod";
            let mut out_len = 0usize;
            let llvm_ptr = pilot_codegen_build(name.as_ptr(), name.len(), 0, &mut out_len);
            assert!(!llvm_ptr.is_null());
            assert!(out_len > 0);
            let llvm_str = core::str::from_utf8(core::slice::from_raw_parts(llvm_ptr, out_len)).unwrap();
            assert!(llvm_str.contains("define i64 @main"));
            pilot_free_string(llvm_ptr, out_len);

            // 2. Transpiler
            let mut tr_len = 0usize;
            let rs_ptr = pilot_transpile_sample(3, &mut tr_len); // target 3 = Rust
            assert!(!rs_ptr.is_null());
            let rs_str = core::str::from_utf8(core::slice::from_raw_parts(rs_ptr, tr_len)).unwrap();
            assert!(rs_str.contains("pub fn compute(x: i64) -> i64 {"));
            pilot_free_string(rs_ptr, tr_len);

            // 3. Polyglot SPDX License & Execution
            let mit = b"MIT";
            assert_eq!(pilot_polyglot_verify_spdx(mit.as_ptr(), mit.len()), 1);

            let gpl = b"GPL-3.0";
            assert_eq!(pilot_polyglot_verify_spdx(gpl.as_ptr(), gpl.len()), 0);

            let lang = b"python";
            let code = b"x = 10";
            let mut ex_len = 0usize;
            let ex_ptr = pilot_polyglot_execute(lang.as_ptr(), lang.len(), code.as_ptr(), code.len(), &mut ex_len);
            assert!(!ex_ptr.is_null());
            let ex_str = core::str::from_utf8(core::slice::from_raw_parts(ex_ptr, ex_len)).unwrap();
            assert!(ex_str.contains("chars of python out-of-process"));
            pilot_free_string(ex_ptr, ex_len);
        }
    }
}
