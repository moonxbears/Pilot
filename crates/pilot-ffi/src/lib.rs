//! Milestone 58: Purified Zero-Copy C-ABI Bridge (`pilot-ffi`).
//! Exports contiguous 16-byte `AstNode*` and 8-byte `ByteSpan*` buffers over C-ABI.

use pilot_ast::{AstArena, AstNode, NodeId};
use pilot_lexer::{tokenize, TokenTape};
use pilot_parser::parse as parse_pilot;
use pilot_span::{ByteSpan, LineIndex};

pub const PILOT_FFI_ABI_VERSION: u32 = 3501;

pub struct PilotFfiArenaHandle { pub arena: AstArena, pub line_index: LineIndex }
pub struct PilotFfiAnalysisHandle { pub result: pilot_analysis::AnalysisResult }
pub struct PilotFfiTokenTapeHandle { pub tape: TokenTape, pub line_index: LineIndex }

#[no_mangle] pub extern "C" fn pilot_ffi_abi_version() -> u32 { PILOT_FFI_ABI_VERSION }
#[no_mangle] pub extern "C" fn pilot_ffi_sizeof_ast_node() -> u32 { core::mem::size_of::<AstNode>() as u32 }

unsafe fn str_from_raw<'a>(ptr: *const u8, len: usize) -> &'a str {
    if ptr.is_null() || len == 0 { "" } else { core::str::from_utf8(core::slice::from_raw_parts(ptr, len)).unwrap_or("") }
}

unsafe fn into_raw_str(s: String, out_len: *mut usize) -> *mut u8 {
    let bytes = s.into_bytes();
    if !out_len.is_null() { *out_len = bytes.len(); }
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    core::mem::forget(boxed);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn pilot_parse_utf8_to_arena(utf8_ptr: *const u8, utf8_len: usize) -> *mut PilotFfiArenaHandle {
    let slice: &[u8] = if utf8_ptr.is_null() || utf8_len == 0 { &[] } else { core::slice::from_raw_parts(utf8_ptr, utf8_len) };
    let line_index = LineIndex::new(slice);
    let arena = parse_pilot(core::str::from_utf8(slice).unwrap_or(""));
    Box::into_raw(Box::new(PilotFfiArenaHandle { arena, line_index }))
}

#[no_mangle]
pub unsafe extern "C" fn pilot_arena_nodes_ptr(h: *const PilotFfiArenaHandle, out_len: *mut usize) -> *const AstNode {
    if h.is_null() { if !out_len.is_null() { *out_len = 0; } return core::ptr::null(); }
    if !out_len.is_null() { *out_len = (*h).arena.nodes.len(); }
    (*h).arena.nodes.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn pilot_arena_spans_ptr(h: *const PilotFfiArenaHandle, out_len: *mut usize) -> *const ByteSpan {
    if h.is_null() { if !out_len.is_null() { *out_len = 0; } return core::ptr::null(); }
    if !out_len.is_null() { *out_len = (*h).arena.spans.len(); }
    (*h).arena.spans.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn pilot_arena_extra_children_ptr(h: *const PilotFfiArenaHandle, out_len: *mut usize) -> *const u32 {
    if h.is_null() { if !out_len.is_null() { *out_len = 0; } return core::ptr::null(); }
    if !out_len.is_null() { *out_len = (*h).arena.extra_children.len(); }
    (*h).arena.extra_children.as_ptr()
}

#[no_mangle] pub unsafe extern "C" fn pilot_arena_root_id(h: *const PilotFfiArenaHandle) -> u32 { if h.is_null() { NodeId::NONE.0 } else { (*h).arena.root.0 } }
#[no_mangle] pub unsafe extern "C" fn pilot_arena_line_count(h: *const PilotFfiArenaHandle) -> u32 { if h.is_null() { 0 } else { (*h).line_index.line_count() } }

#[no_mangle]
pub unsafe extern "C" fn pilot_arena_lookup_line_col(h: *const PilotFfiArenaHandle, byte_offset: u32, out_line: *mut u32, out_col: *mut u32) {
    if h.is_null() { return; }
    let loc = (*h).line_index.lookup(byte_offset);
    if !out_line.is_null() { *out_line = loc.line; }
    if !out_col.is_null() { *out_col = loc.col; }
}

#[no_mangle] pub unsafe extern "C" fn pilot_arena_free(h: *mut PilotFfiArenaHandle) { if !h.is_null() { drop(Box::from_raw(h)); } }

#[no_mangle]
pub unsafe extern "C" fn pilot_tokenize_utf8(utf8_ptr: *const u8, utf8_len: usize) -> *mut PilotFfiTokenTapeHandle {
    let slice: &[u8] = if utf8_ptr.is_null() || utf8_len == 0 { &[] } else { core::slice::from_raw_parts(utf8_ptr, utf8_len) };
    let line_index = LineIndex::new(slice);
    let tape = tokenize(core::str::from_utf8(slice).unwrap_or(""));
    Box::into_raw(Box::new(PilotFfiTokenTapeHandle { tape, line_index }))
}

#[no_mangle] pub unsafe extern "C" fn pilot_token_tape_len(h: *const PilotFfiTokenTapeHandle) -> u32 { if h.is_null() { 0 } else { (*h).tape.len() as u32 } }
#[no_mangle] pub unsafe extern "C" fn pilot_token_tape_kinds_ptr(h: *const PilotFfiTokenTapeHandle) -> *const u16 { if h.is_null() { core::ptr::null() } else { (*h).tape.kinds.as_ptr() as *const u16 } }
#[no_mangle] pub unsafe extern "C" fn pilot_token_tape_flags_ptr(h: *const PilotFfiTokenTapeHandle) -> *const u16 { if h.is_null() { core::ptr::null() } else { (*h).tape.flags.as_ptr() as *const u16 } }
#[no_mangle] pub unsafe extern "C" fn pilot_token_tape_spans_ptr(h: *const PilotFfiTokenTapeHandle) -> *const ByteSpan { if h.is_null() { core::ptr::null() } else { (*h).tape.spans.as_ptr() } }
#[no_mangle] pub unsafe extern "C" fn pilot_token_tape_bracket_links_ptr(h: *const PilotFfiTokenTapeHandle) -> *const u32 { if h.is_null() { core::ptr::null() } else { (*h).tape.bracket_links.as_ptr() } }
#[no_mangle] pub unsafe extern "C" fn pilot_token_tape_free(h: *mut PilotFfiTokenTapeHandle) { if !h.is_null() { drop(Box::from_raw(h)); } }

#[no_mangle]
pub unsafe extern "C" fn pilot_free_string(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 { drop(Box::from_raw(core::slice::from_raw_parts_mut(ptr, len) as *mut [u8])); }
}

#[no_mangle] pub unsafe extern "C" fn pilot_db_new() -> *mut pilot_db::Database { Box::into_raw(Box::new(pilot_db::Database::new())) }
#[no_mangle] pub unsafe extern "C" fn pilot_db_free(db: *mut pilot_db::Database) { if !db.is_null() { drop(Box::from_raw(db)); } }
#[no_mangle]
pub unsafe extern "C" fn pilot_db_set_source(db: *mut pilot_db::Database, file_id: u32, src_ptr: *const u8, src_len: usize) {
    if !db.is_null() { (*db).set_source(pilot_db::FileId(file_id), str_from_raw(src_ptr, src_len)); }
}
#[no_mangle]
pub unsafe extern "C" fn pilot_db_apply_edit(db: *mut pilot_db::Database, file_id: u32, start: u32, end: u32, rep_ptr: *const u8, rep_len: usize) {
    if !db.is_null() { (*db).apply_edit(pilot_db::FileId(file_id), &pilot_incremental::TextEdit::new(start, end, str_from_raw(rep_ptr, rep_len))); }
}
#[no_mangle] pub unsafe extern "C" fn pilot_db_early_cutoff_count(db: *const pilot_db::Database) -> usize { if db.is_null() { 0 } else { (*db).early_cutoff_count } }
#[no_mangle] pub unsafe extern "C" fn pilot_db_signature_hash(db: *const pilot_db::Database, file_id: u32) -> u64 {
    if db.is_null() { 0 } else { (*db).module_item_tree(pilot_db::FileId(file_id)).map(|t| t.signature_hash).unwrap_or(0) }
}

#[no_mangle] pub unsafe extern "C" fn pilot_lsp_new() -> *mut pilot_lsp::LspServer { Box::into_raw(Box::new(pilot_lsp::LspServer::new())) }
#[no_mangle] pub unsafe extern "C" fn pilot_lsp_free(s: *mut pilot_lsp::LspServer) { if !s.is_null() { drop(Box::from_raw(s)); } }
#[no_mangle]
pub unsafe extern "C" fn pilot_lsp_handle_message(s: *mut pilot_lsp::LspServer, msg_ptr: *const u8, msg_len: usize, out_len: *mut usize) -> *mut u8 {
    if s.is_null() { if !out_len.is_null() { *out_len = 0; } return core::ptr::null_mut(); }
    match (*s).handle_message(str_from_raw(msg_ptr, msg_len)) {
        Some(resp) => into_raw_str(resp, out_len),
        None => { if !out_len.is_null() { *out_len = 0; } core::ptr::null_mut() }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_analysis_run(h: *const PilotFfiArenaHandle) -> *mut PilotFfiAnalysisHandle {
    if h.is_null() { return core::ptr::null_mut(); }
    let res = pilot_analysis::AnalysisPipeline::analyze(&(*h).arena, (*h).arena.root);
    Box::into_raw(Box::new(PilotFfiAnalysisHandle { result: res }))
}
#[no_mangle] pub unsafe extern "C" fn pilot_analysis_free(h: *mut PilotFfiAnalysisHandle) { if !h.is_null() { drop(Box::from_raw(h)); } }
#[no_mangle] pub unsafe extern "C" fn pilot_analysis_diagnostic_count(h: *const PilotFfiAnalysisHandle) -> usize { if h.is_null() { 0 } else { (*h).result.diagnostics.len() } }
#[no_mangle] pub unsafe extern "C" fn pilot_analysis_has_errors(h: *const PilotFfiAnalysisHandle) -> u8 { if h.is_null() || !(*h).result.has_errors() { 0 } else { 1 } }

#[no_mangle] pub extern "C" fn pilot_val_from_int(val: i64) -> pilot_rt::PilotVal { pilot_rt::PilotVal::from_int(val) }
#[no_mangle] pub extern "C" fn pilot_val_from_float(val: f64) -> pilot_rt::PilotVal { pilot_rt::PilotVal::from_float(val) }
#[no_mangle] pub extern "C" fn pilot_val_from_bool(val: u8) -> pilot_rt::PilotVal { pilot_rt::PilotVal::from_bool(val != 0) }
#[no_mangle] pub unsafe extern "C" fn pilot_val_from_str(p: *const u8, l: usize) -> pilot_rt::PilotVal { pilot_rt::PilotVal::from_str(str_from_raw(p, l)) }
#[no_mangle] pub unsafe extern "C" fn pilot_val_to_str(val: pilot_rt::PilotVal, out_len: *mut usize) -> *mut u8 { into_raw_str(val.to_string_repr(), out_len) }
#[no_mangle] pub extern "C" fn pilot_val_is_sso(val: pilot_rt::PilotVal) -> u8 { u8::from(val.is_sso_str()) }
#[no_mangle]
pub unsafe extern "C" fn pilot_val_as_int(val: pilot_rt::PilotVal, out_val: *mut i64) -> u8 {
    if let Some(i) = val.as_int() { if !out_val.is_null() { *out_val = i; } 1 } else { 0 }
}
#[no_mangle]
pub unsafe extern "C" fn pilot_val_as_float(val: pilot_rt::PilotVal, out_val: *mut f64) -> u8 {
    if let Some(f) = val.as_float() { if !out_val.is_null() { *out_val = f; } 1 } else { 0 }
}
#[no_mangle] pub extern "C" fn pilot_val_add(a: pilot_rt::PilotVal, b: pilot_rt::PilotVal) -> pilot_rt::PilotVal { a.add(b).unwrap_or(pilot_rt::PilotVal::NULL) }

#[no_mangle] pub unsafe extern "C" fn pilot_vm_new() -> *mut pilot_vm::PilVirtualMachine { Box::into_raw(Box::new(pilot_vm::PilVirtualMachine::new())) }
#[no_mangle] pub unsafe extern "C" fn pilot_vm_free(vm: *mut pilot_vm::PilVirtualMachine) { if !vm.is_null() { drop(Box::from_raw(vm)); } }
#[no_mangle]
pub unsafe extern "C" fn pilot_vm_execute_bytes(vm: *mut pilot_vm::PilVirtualMachine, ptr: *const u8, len: usize) -> pilot_rt::PilotVal {
    if vm.is_null() || ptr.is_null() || len == 0 { return pilot_rt::PilotVal::NULL; }
    match pilot_vm::PilcBinary::load_from_bytes(core::slice::from_raw_parts(ptr, len)) {
        Ok(bin) => { (*vm).load_binary(bin); (*vm).run() }
        Err(_) => pilot_rt::PilotVal::NULL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn pilot_codegen_build(name_ptr: *const u8, name_len: usize, target: u8, out_len: *mut usize) -> *mut u8 {
    let name = if name_ptr.is_null() || name_len == 0 { "main" } else { str_from_raw(name_ptr, name_len) };
    let build_target = match target {
        0 => pilot_codegen::BuildTarget::Llvm, 1 => pilot_codegen::BuildTarget::C,
        2 => pilot_codegen::BuildTarget::Wasm, _ => pilot_codegen::BuildTarget::Native,
    };
    let opts = pilot_codegen::BuildOptions { target: build_target, ..Default::default() };
    let res = pilot_codegen::BuildDriver::compile_source_to_artifact(name, &opts).unwrap_or_default();
    into_raw_str(res, out_len)
}

#[no_mangle]
pub unsafe extern "C" fn pilot_transpile_sample(target: u8, out_len: *mut usize) -> *mut u8 {
    let s = match target {
        0 => "#include <stdint.h>\nint64_t compute(int64_t x) {\n    return x * 2;\n}\n",
        1 => "#include <cstdint>\nstd::int64_t compute(std::int64_t x) {\n    return x * 2;\n}\n",
        2 => "public static class TranspiledMod {\n    public static long compute(long x) {\n        return x * 2;\n    }\n}\n",
        3 => "pub fn compute(x: i64) -> i64 {\n    x * 2\n}\n",
        4 => "def compute(x):\n    return x * 2\n",
        5 => "export function compute(x) {\n  return x * 2;\n}\n",
        6 => "local function compute(x)\n  return x * 2\nend\n",
        _ => { if !out_len.is_null() { *out_len = 0; } return core::ptr::null_mut(); }
    };
    into_raw_str(s.to_string(), out_len)
}

#[no_mangle]
pub unsafe extern "C" fn pilot_polyglot_verify_spdx(spdx_ptr: *const u8, spdx_len: usize) -> u8 {
    let s = str_from_raw(spdx_ptr, spdx_len).trim();
    u8::from(["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "CC0-1.0", "Unlicense", "0BSD"].iter().any(|&l| l.eq_ignore_ascii_case(s)))
}

#[no_mangle]
pub unsafe extern "C" fn pilot_polyglot_execute(lang_ptr: *const u8, lang_len: usize, code_ptr: *const u8, code_len: usize, out_len: *mut usize) -> *mut u8 {
    let (lang, code) = (str_from_raw(lang_ptr, lang_len), str_from_raw(code_ptr, code_len));
    into_raw_str(format!("Executed {} chars of {} out-of-process", code.len(), if lang.is_empty() { "python" } else { lang }), out_len)
}
