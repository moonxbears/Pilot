with open('crates/pilot-lsp/src/server.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Update TYPE_DOCS
t_typedocs = '    ("SliceBuffer", "Fast zero-copy slicing buffer backed by linear memory."),'
r_typedocs = '''    ("Slice", "Contiguous zero-marshalling memory slice (`Slice[T]`) with pointer + length."),
    ("SliceBuffer", "Fast zero-copy slicing buffer backed by linear memory."),
    ("SliceView", "Compile-time offset-swizzled zero-copy view (`SliceView[T]`) with base pointer, length, and stride."),
    ("StridedSlice", "Strided non-contiguous zero-copy memory slice (`StridedSlice[T]`) with base pointer, length, stride, and offset."),
    ("View", "Zero-copy scalar reference view (`View[T]`) swizzling field offsets in-place without memory movement."),
    ("SliceMap", "Universal zero-copy key-value projection (`SliceMap[K, V]`) over contiguous struct buffers or SoA parallel arrays."),'''

# 2. Update KEYWORD_DOCS to add view
t_view = '    ("as", ("Type Casting", "`<expr> as <Type>` — Converts or casts `<expr>` to `<Type>`, or aliases an import.")),'
r_view = '''    ("as", ("Type Casting & Bit-Cast", "`<expr> as <Type>` — Isomorphic bit-cast (`expr as Target`, `Slice[T] as Slice[U]`), cast, or import alias.")),
    ("view", ("Offset Swizzling", "`<expr> as view <Type>` — Zero-copy compile-time offset swizzled view into `<Type>` without heap allocation or copying.")),'''

# 3. Update member completion clearance check
t_mem_vis = '''                                            if m_vis == pilot_ast::VisibilityTier::Private && !is_inside_type {
                                                continue;
                                            }'''

r_mem_vis = '''                                            if m_vis == pilot_ast::VisibilityTier::Private && !is_inside_type {
                                                continue;
                                            }

                                            if let Some(clr) = analysis.struct_member_clearance.get(&(ty_str.clone(), f_name.clone())) {
                                                if clr.level > analysis.current_scope_clearance {
                                                    continue;
                                                }
                                            }'''

t_child_vis = '''                                                                if m_vis == pilot_ast::VisibilityTier::Private && !is_inside_type {
                                                                    continue;
                                                                }'''

r_child_vis = '''                                                                if m_vis == pilot_ast::VisibilityTier::Private && !is_inside_type {
                                                                    continue;
                                                                }

                                                                if let Some(clr) = arena.node_clearance.get(&child_id) {
                                                                    if clr.level > analysis.current_scope_clearance {
                                                                        continue;
                                                                    }
                                                                }'''

# 4. Update top-level symbol clearance check in completions
t_top_sym = '''        // 6. Top-level module symbols & variables
        if let Some(file_id) = file_id_opt {
            if let Some(item_tree) = self.db.module_item_tree(file_id) {
                for item in &item_tree.items {
                    items.push(JsonValue::Object(vec![
                        ("label".to_string(), JsonValue::String(item.name.clone())),
                        ("kind".to_string(), JsonValue::Number(3.0)), // Function
                        ("detail".to_string(), JsonValue::String(item.signature_text.clone())),
                    ]));
                }
            }'''

r_top_sym = '''        // 6. Top-level module symbols & variables
        if let Some(file_id) = file_id_opt {
            let mod_analysis = self.db.module_analysis(file_id);
            let analysis = mod_analysis.as_ref().map(|m| &m.analysis);
            let cur_clr = analysis.map(|a| a.current_scope_clearance).unwrap_or(0);

            if let Some(item_tree) = self.db.module_item_tree(file_id) {
                for item in &item_tree.items {
                    if let Some(a) = analysis {
                        if let Some(clr) = a.fn_clearance.get(&item.name) {
                            if clr.level > cur_clr {
                                continue;
                            }
                        }
                    }
                    items.push(JsonValue::Object(vec![
                        ("label".to_string(), JsonValue::String(item.name.clone())),
                        ("kind".to_string(), JsonValue::Number(3.0)), // Function
                        ("detail".to_string(), JsonValue::String(item.signature_text.clone())),
                    ]));
                }
            }'''

# 5. Update code actions for VIS001, LVL001, STATIC001
t_code_act = '''        // Check if a range was selected to offer "Extract Variable" refactoring'''
r_code_act = '''        if let Some(context) = params.get("context") {
            if let Some(diags) = context.get("diagnostics").and_then(|d| d.as_array()) {
                for diag in diags {
                    let code = diag.get("code").and_then(|c| c.as_str()).unwrap_or("");
                    if code == "VIS001" {
                        actions.push(JsonValue::Object(vec![
                            ("title".to_string(), JsonValue::String("Quick Fix: Change member visibility to public (`pub`)".to_string())),
                            ("kind".to_string(), JsonValue::String("quickfix".to_string())),
                            ("isPreferred".to_string(), JsonValue::Bool(true)),
                            ("diagnostics".to_string(), JsonValue::Array(vec![diag.clone()])),
                        ]));
                        actions.push(JsonValue::Object(vec![
                            ("title".to_string(), JsonValue::String("Quick Fix: Change member visibility to internal (`int`)".to_string())),
                            ("kind".to_string(), JsonValue::String("quickfix".to_string())),
                            ("diagnostics".to_string(), JsonValue::Array(vec![diag.clone()])),
                        ]));
                    } else if code == "LVL001" {
                        actions.push(JsonValue::Object(vec![
                            ("title".to_string(), JsonValue::String("Quick Fix: Guard call with clearance check (`if lvl >= ...:`)".to_string())),
                            ("kind".to_string(), JsonValue::String("quickfix".to_string())),
                            ("isPreferred".to_string(), JsonValue::Bool(true)),
                            ("diagnostics".to_string(), JsonValue::Array(vec![diag.clone()])),
                        ]));
                        actions.push(JsonValue::Object(vec![
                            ("title".to_string(), JsonValue::String("Quick Fix: Elevate caller clearance (`lvlN def ...`)".to_string())),
                            ("kind".to_string(), JsonValue::String("quickfix".to_string())),
                            ("diagnostics".to_string(), JsonValue::Array(vec![diag.clone()])),
                        ]));
                    } else if code == "STATIC001" {
                        actions.push(JsonValue::Object(vec![
                            ("title".to_string(), JsonValue::String("Quick Fix: Remove 'static' modifier from method declaration".to_string())),
                            ("kind".to_string(), JsonValue::String("quickfix".to_string())),
                            ("isPreferred".to_string(), JsonValue::Bool(true)),
                            ("diagnostics".to_string(), JsonValue::Array(vec![diag.clone()])),
                        ]));
                    }
                }
            }
        }

        // Check if a range was selected to offer "Extract Variable" refactoring'''

# 6. Semantic tokens modifier matching
t_sem_mod = '''} else if matches!(text, "public" | "private" | "protected" | "internal" | "static" | "partial" | "readonly" | "override" | "virtual" | "abstract" | "sealed") {
                            Some((16, 0)) // modifier'''

r_sem_mod = '''} else if matches!(text, "priv" | "pub" | "file" | "fileprivate" | "prot" | "pkg" | "package" | "int" | "open" | "export" | "iso" | "steal" | "public" | "private" | "protected" | "internal" | "static" | "partial" | "readonly" | "override" | "virtual" | "abstract" | "sealed") || (text.starts_with("lvl") && (text.len() == 3 || text[3..].chars().all(|c| c.is_ascii_digit()))) {
                            Some((16, 0)) // modifier
                        } else if text == "view" && prev_non_trivia_kind == SyntaxKind::KwAs {
                            Some((15, 0)) // keyword'''

c_nl = '\r\n' if '\r\n' in content else '\n'
def norm(s):
    return s.replace('\r\n', '\n').replace('\n', c_nl)

content = content.replace(norm(t_typedocs), norm(r_typedocs), 1)
content = content.replace(norm(t_view), norm(r_view), 1)
content = content.replace(norm(t_mem_vis), norm(r_mem_vis), 1)
content = content.replace(norm(t_child_vis), norm(r_child_vis), 1)
content = content.replace(norm(t_top_sym), norm(r_top_sym), 1)
content = content.replace(norm(t_code_act), norm(r_code_act), 1)
content = content.replace(norm(t_sem_mod), norm(r_sem_mod), 1)

with open('crates/pilot-lsp/src/server.rs', 'w', encoding='utf-8', newline='') as f:
    f.write(content)
print("Updated pilot-lsp server.rs")
