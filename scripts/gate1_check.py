import sys
import re
import glob

ALLOWED_INFER_STRINGS = {
    "ast_node_count", "ast_node_kind", "ast_orig_kind", "ast_node_flags", "ast_decl_flags",
    "ast_parent", "ast_node_sym", "ast_node_lit_kind", "ast_node_raw_bits",
    "ast_child_count", "ast_child_at",
    "ast_attr_count", "ast_attr_arg",
    "ast_attr_decl_count", "ast_attr_name", "ast_attr_decl_arg_count", "ast_attr_decl_arg",
    "ast_base_count", "ast_base_at",
    "layout_hw_header", "layout_scalar", "layout_ptr", "layout_aggregate", "layout_union",
    "layout_fn_abi", "layout_metrics", "layout_field_offset",
    "layout_physically_equal", "ast_bind_layout", "ast_get_bound_layout",
    "ast_bind_target", "ast_get_bound_target", "ast_emit_diag",
    "sym_str", "str_len", "str_byte_at",
    "ast_span_start", "ast_span_end", "source_slice",
    "register_keyword_category", "register_keyword", "query_registered_symbols",
    "mount_unit", "lsp_query_method", "lsp_query_offset", "lsp_set_result",
    "selfhost/analyzer.pilot", "../../../selfhost/analyzer.pilot",
    "selfhost/.pilc-cache", "selfhost/.pilc-cache/analyzer.pilc",
    "selfhost/.pilc-cache/analyzer.pilc.tmp",
}

BANNED_IDENTIFIERS = [
    "TypeInferenceEngine", "TypeArena", "TypeKind", "CollectionKind",
    "LangItem", "ReceptorSlot", "StructuralShape",
]

STRING_LITERAL_RE = re.compile(r'"([^"\\]*(?:\\.[^"\\]*)*)"')

def check_file_lines(path, max_lines):
    with open(path, "r", encoding="utf-8") as f:
        count = sum(1 for _ in f)
    if count > max_lines:
        print(f"FAIL: {path} has {count} lines (hard maximum is {max_lines})")
        return False
    print(f"PASS: {path} line count = {count} (<= {max_lines})")
    return True

def check_whitelist_strings(path, allowed_set):
    ok = True
    with open(path, "r", encoding="utf-8") as f:
        for idx, line in enumerate(f, 1):
            for m in STRING_LITERAL_RE.finditer(line):
                val = m.group(1)
                if val not in allowed_set:
                    print(f"FAIL: Unapproved string literal \"{val}\" in {path}:{idx}")
                    ok = False
    return ok

def check_banned_identifiers():
    ok = True
    pattern = re.compile(r'\b(' + '|'.join(BANNED_IDENTIFIERS) + r')\b')
    for path in glob.glob("crates/pilot-analysis/src/**/*.rs", recursive=True):
        with open(path, "r", encoding="utf-8") as f:
            for idx, line in enumerate(f, 1):
                m = pattern.search(line)
                if m:
                    print(f"FAIL: Banned legacy symbol '{m.group(1)}' found in {path}:{idx}")
                    ok = False
    return ok

def main():
    passed = True
    if not check_file_lines("crates/pilot-analysis/src/types.rs", 250):
        passed = False
    if not check_file_lines("crates/pilot-analysis/src/infer.rs", 440):
        passed = False
    if not check_whitelist_strings("crates/pilot-analysis/src/types.rs", set()):
        passed = False
    if not check_whitelist_strings("crates/pilot-analysis/src/infer.rs", ALLOWED_INFER_STRINGS):
        passed = False
    if not check_banned_identifiers():
        passed = False

    if not passed:
        sys.exit(1)
    print("ALL PHASE 1A SCORCHED-EARTH GATES PASSED!")
    sys.exit(0)

if __name__ == "__main__":
    main()
