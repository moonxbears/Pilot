with open('crates/pilot-analysis/src/pipeline.rs', 'r', encoding='utf-8') as f:
    content = f.read()

target1 = """    pub struct_member_visibility: HashMap<(String, String), pilot_ast::VisibilityTier>,
    pub struct_member_clearance: HashMap<(String, String), pilot_ast::ClearanceLevel>,
}"""

repl1 = """    pub struct_member_visibility: HashMap<(String, String), pilot_ast::VisibilityTier>,
    pub struct_member_clearance: HashMap<(String, String), pilot_ast::ClearanceLevel>,
    pub fn_visibility: HashMap<String, pilot_ast::VisibilityTier>,
    pub fn_clearance: HashMap<String, pilot_ast::ClearanceLevel>,
    pub current_scope_clearance: u32,
}"""

target2 = """            struct_member_visibility: infer_engine.struct_member_visibility,
            struct_member_clearance: infer_engine.struct_member_clearance,
        }"""

repl2 = """            struct_member_visibility: infer_engine.struct_member_visibility,
            struct_member_clearance: infer_engine.struct_member_clearance,
            fn_visibility: infer_engine.fn_visibility,
            fn_clearance: infer_engine.fn_clearance,
            current_scope_clearance: infer_engine.current_scope_clearance,
        }"""

c_nl = '\r\n' if '\r\n' in content else '\n'
def norm(s):
    return s.replace('\r\n', '\n').replace('\n', c_nl)

content = content.replace(norm(target1), norm(repl1), 1)
content = content.replace(norm(target2), norm(repl2), 1)

with open('crates/pilot-analysis/src/pipeline.rs', 'w', encoding='utf-8', newline='') as f:
    f.write(content)
print("Updated pipeline.rs")
