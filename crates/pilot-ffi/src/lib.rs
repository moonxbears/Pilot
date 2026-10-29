//! Milestone 28.4: Zero-Copy C-ABI Strangler-Fig Bridge (`pilot-ffi`)
//!
//! Exports contiguous 16-byte `AstNode*` and 8-byte `ByteSpan*` buffers over C-ABI so C# can
//! inspect them directly via `ReadOnlySpan<NativeAstNode>` (`sizeof == 16`) with zero per-node marshalling.

use pilot_ast::{AstArena, AstNode, BinaryOp, NodeId, NodeKind};
use pilot_lexer::{tokenize, TokenTape};
use pilot_span::{ByteSpan, LineIndex};

pub const PILOT_FFI_ABI_VERSION: u32 = 2901;

/// Opaque C-ABI handle owning a parsed `AstArena` and `LineIndex`.
pub struct PilotFfiArenaHandle {
    pub arena: AstArena,
    pub line_index: LineIndex,
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
    let arena = parse_lightweight_skeleton_to_arena(src);

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
            assert_eq!(pilot_ffi_abi_version(), 2901);
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
}
