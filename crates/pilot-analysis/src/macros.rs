//! Milestone 38.3 & Self-Hosting Phase 2: Homoiconic Macros, `[...]` Bracket Attribute Dispatch & `syntax` Macro Expansion.
//!
//! Provides:
//! - Phase 2 `[Attr(args...)]` left-to-right compile-time attribute desugaring (`desugar_bracket_attributes`)
//! - Phase 2 homoiconic `quote { ... }` / `$unquote` splicing and `syntax` macro expansion
//! - Legacy compile-time AST-to-AST decorator derives (`@builder`, `@json`, `@observable`, `@equality`, `@repr`, `@metadata`)

use pilot_ast::{AstArena, AstAttribute, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_parser::{FxHashMap, ParserTables};
use pilot_span::ByteSpan;
use crate::comptime::{BootstrapDiagnostic, ComptimeEngine, ComptimeValue};
use crate::diagnostics::AnalysisDiagnostic;

/// Desugars all `[Attr1(args...), Attr2(args...)]` bracket attributes attached to `target_node`
/// strictly left-to-right by invoking their registered `comptime fn` handlers in `ParserTables`.
pub fn desugar_bracket_attributes(
    arena: &mut AstArena,
    target_node: NodeId,
    parser_tables: &mut ParserTables,
    comptime_vm: &mut ComptimeEngine,
) -> Result<(), BootstrapDiagnostic> {
    let attrs: Vec<AstAttribute> = match arena.node_attributes.get(&target_node.0) {
        Some(list) => list.clone(),
        None => return Ok(()),
    };

    for attr in attrs {
        let attr_name = arena.interner.resolve(attr.name).to_string();
        let fn_id = if let Some(&id) = parser_tables.bracket_attributes_by_name.get(&attr_name) {
            Some(id)
        } else if let Some(&id) = parser_tables.bracket_attributes.get(&attr.name) {
            Some(id)
        } else if let Some(&id) = parser_tables.bracket_attributes_by_name.get(&format!("{}Attr", attr_name)) {
            Some(id)
        } else if comptime_vm.type_layouts.contains_key(&format!("{}Attr", attr_name))
            || comptime_vm.type_layouts.contains_key(&attr_name)
        {
            None
        } else {
            return Err(BootstrapDiagnostic::UnknownBracketAttribute {
                name: attr_name,
                span: attr.span,
            });
        };

        if let Some(fid) = fn_id {
            let mut call_args = Vec::with_capacity(1 + attr.args.len());
            call_args.push(ComptimeValue::AstNode(target_node));
            for &arg_node in &attr.args {
                let val = comptime_vm.eval_expr(arena, arg_node, parser_tables)?;
                call_args.push(val);
            }

            comptime_vm.invoke_function(fid, &call_args, arena, parser_tables, attr.span)?;
        } else {
            for &arg_node in &attr.args {
                let _ = comptime_vm.eval_expr(arena, arg_node, parser_tables)?;
            }
        }
    }

    Ok(())
}

/// Expands a dynamically registered `syntax` macro keyword (such as `syntax`, `break`, `continue`, `op`)
/// by invoking its `FunctionId` in `ComptimeEngine`.
pub fn expand_syntax_macro(
    arena: &mut AstArena,
    kw_name: &str,
    args: &[ComptimeValue],
    parser_tables: &mut ParserTables,
    comptime_vm: &mut ComptimeEngine,
    span: ByteSpan,
) -> Result<ComptimeValue, BootstrapDiagnostic> {
    let fn_id = parser_tables
        .syntax_macros_by_name
        .get(kw_name)
        .copied()
        .ok_or_else(|| BootstrapDiagnostic::UnknownSyntaxKeyword {
            keyword: kw_name.to_string(),
            span,
        })?;

    comptime_vm.invoke_function(fn_id, args, arena, parser_tables, span)
}

/// Splices a `quote { ... }` template node from `template_arena` into `target_arena`, replacing
/// `$ident` unquote nodes with their bound `ComptimeValue`s from `bindings`.
pub fn splice_quote_template(
    template_arena: &AstArena,
    template_node: NodeId,
    bindings: &FxHashMap<String, ComptimeValue>,
    target_arena: &mut AstArena,
) -> NodeId {
    if template_node.is_none() {
        return NodeId::NONE;
    }
    let node = *template_arena.node(template_node);
    let span = template_arena.span(template_node);

    match node.node_kind() {
        NodeKind::IdentExpr if node.flags == 1 => {
            let name = template_arena.interner.resolve(SymbolId(node.lhs));
            if let Some(bound) = bindings.get(name) {
                match bound {
                    ComptimeValue::AstNode(nid) => return *nid,
                    ComptimeValue::Str(sym) => {
                        let s = template_arena.interner.resolve(*sym);
                        let new_sym = target_arena.interner.intern(s);
                        return target_arena.alloc_node(NodeKind::StrLit, 0, span, new_sym.0, 0);
                    }
                    ComptimeValue::Ident(sym) => {
                        let s = template_arena.interner.resolve(*sym);
                        return target_arena.alloc_ident(s, span);
                    }
                    ComptimeValue::Uint(v) => {
                        return target_arena.alloc_int(*v as i64, span);
                    }
                    ComptimeValue::Int(v) => {
                        return target_arena.alloc_int(*v as i64, span);
                    }
                    ComptimeValue::Bool(b) => {
                        return target_arena.alloc_node(NodeKind::BoolLit, 0, span, u32::from(*b), 0);
                    }
                    _ => {}
                }
            }
            let name_str = template_arena.interner.resolve(SymbolId(node.lhs));
            target_arena.alloc_ident(name_str, span)
        }
        NodeKind::IdentExpr => {
            let name_str = template_arena.interner.resolve(SymbolId(node.lhs));
            target_arena.alloc_ident(name_str, span)
        }
        NodeKind::StrLit => {
            let s = template_arena.interner.resolve(SymbolId(node.lhs));
            let sym = target_arena.interner.intern(s);
            target_arena.alloc_node(NodeKind::StrLit, node.flags, span, sym.0, 0)
        }
        NodeKind::IntLit => {
            let val = template_arena.i64_pool.get(node.lhs as usize).copied().unwrap_or(0);
            target_arena.alloc_int(val, span)
        }
        NodeKind::BoolLit => target_arena.alloc_node(NodeKind::BoolLit, node.flags, span, node.lhs, node.rhs),
        NodeKind::BlockExpr => {
            let children: Vec<u32> = template_arena.children_slice(node.lhs, node.rhs).to_vec();
            let mut new_children = Vec::with_capacity(children.len());
            for c in children {
                let spliced = splice_quote_template(template_arena, NodeId(c), bindings, target_arena);
                if spliced.is_some() {
                    new_children.push(spliced);
                }
            }
            target_arena.alloc_list_node(NodeKind::BlockExpr, node.flags, span, &new_children)
        }
        NodeKind::FnDecl => {
            let name_str = template_arena.interner.resolve(SymbolId(node.lhs));
            let new_name_sym = target_arena.interner.intern(name_str);
            let slice: Vec<u32> = template_arena.children_slice(node. rhs, node.flags as u32).to_vec();
            let mut new_slice = Vec::with_capacity(slice.len());
            if let Some(&body_raw) = slice.first() {
                let spliced_body = splice_quote_template(template_arena, NodeId(body_raw), bindings, target_arena);
                new_slice.push(spliced_body);
            }
            for &param_raw in slice.iter().skip(1) {
                let p_node = *template_arena.node(NodeId(param_raw));
                if p_node.node_kind() == NodeKind::ParamDecl && p_node.flags == 1 {
                    let p_name = template_arena.interner.resolve(SymbolId(p_node.lhs));
                    if let Some(ComptimeValue::AstNode(list_id)) = bindings.get(p_name) {
                        new_slice.push(*list_id);
                        continue;
                    }
                }
                let spliced_param = splice_quote_template(template_arena, NodeId(param_raw), bindings, target_arena);
                new_slice.push(spliced_param);
            }
            let (fn_start, fn_len) = target_arena.alloc_child_slice(&new_slice);
            let new_fn = target_arena.alloc_node(NodeKind::FnDecl, fn_len as u16, span, new_name_sym.0, fn_start);
            if template_arena.comptime_fns.contains(&template_node.0) {
                target_arena.comptime_fns.insert(new_fn.0);
            }
            if let Some(attrs) = template_arena.node_attributes.get(&template_node.0) {
                let mut new_attrs = Vec::with_capacity(attrs.len());
                for attr in attrs {
                    let attr_name_str = template_arena.interner.resolve(attr.name);
                    let new_attr_name = target_arena.interner.intern(attr_name_str);
                    let mut new_args = Vec::with_capacity(attr.args.len());
                    for &arg_id in &attr.args {
                        new_args.push(splice_quote_template(template_arena, arg_id, bindings, target_arena));
                    }
                    new_attrs.push(AstAttribute {
                        name: new_attr_name,
                        args: new_args,
                        binding_sym: attr.binding_sym,
                        is_mutable_binding: attr.is_mutable_binding,
                        span: attr.span,
                    });
                }
                target_arena.node_attributes.insert(new_fn.0, new_attrs);
            }
            new_fn
        }
        NodeKind::ExprStmt | NodeKind::ReturnStmt => {
            let inner = splice_quote_template(template_arena, NodeId(node.lhs), bindings, target_arena);
            target_arena.alloc_node(node.node_kind(), node.flags, span, inner.0, node.rhs)
        }
        _ => {
            target_arena.alloc_node(node.node_kind(), node.flags, span, node.lhs, node.rhs)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DecoratorKind {
    Builder,
    Json,
    Observable,
    Equality,
    Repr,
    Metadata,
    Custom(String),
}

impl DecoratorKind {
    pub fn parse(name: &str) -> Self {
        let clean = name.trim_start_matches('@');
        match clean {
            "builder" => DecoratorKind::Builder,
            "json" => DecoratorKind::Json,
            "observable" => DecoratorKind::Observable,
            "equality" => DecoratorKind::Equality,
            "repr" => DecoratorKind::Repr,
            "metadata" => DecoratorKind::Metadata,
            other => DecoratorKind::Custom(other.to_string()),
        }
    }
}

/// Generated method or schema synthesized by a macro or decorator pass.
#[derive(Clone, Debug, PartialEq)]
pub struct SynthesizedMember {
    pub name: String,
    pub return_type: String,
    pub doc_comment: String,
}

/// Macro & Decorator Expansion Engine.
pub struct MacroExpander {
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub registered_macros: Vec<String>,
}

impl Default for MacroExpander {
    fn default() -> Self {
        Self::new()
    }
}

impl MacroExpander {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            registered_macros: Vec::new(),
        }
    }

    /// Registers a user-defined macro by name.
    pub fn register_macro(&mut self, name: &str) {
        if !self.registered_macros.iter().any(|m| m == name) {
            self.registered_macros.push(name.to_string());
        }
    }

    /// Synthesizes methods for a record or struct decorated with a known compile-time decorator.
    pub fn derive_decorator(
        &mut self,
        decorator: &DecoratorKind,
        target_name: &str,
        fields: &[(&str, &str)], // (field_name, field_type)
        span: ByteSpan,
    ) -> Vec<SynthesizedMember> {
        let mut members = Vec::new();

        match decorator {
            DecoratorKind::Builder => {
                for &(fname, _ftype) in fields {
                    members.push(SynthesizedMember {
                        name: format!("with_{}", fname),
                        return_type: target_name.to_string(),
                        doc_comment: format!("Sets '{}' and returns updated '{}'", fname, target_name),
                    });
                }
                members.push(SynthesizedMember {
                    name: "build".to_string(),
                    return_type: target_name.to_string(),
                    doc_comment: format!("Validates and constructs final '{}'", target_name),
                });
            }
            DecoratorKind::Json => {
                members.push(SynthesizedMember {
                    name: "to_json".to_string(),
                    return_type: "str".to_string(),
                    doc_comment: format!("Serializes '{}' to JSON format", target_name),
                });
                members.push(SynthesizedMember {
                    name: "from_json".to_string(),
                    return_type: format!("{}?", target_name),
                    doc_comment: format!("Deserializes JSON payload into '{}'", target_name),
                });
            }
            DecoratorKind::Observable => {
                members.push(SynthesizedMember {
                    name: "on_change".to_string(),
                    return_type: "void".to_string(),
                    doc_comment: "Registers a reactive subscriber for property changes".to_string(),
                });
                for &(fname, _) in fields {
                    members.push(SynthesizedMember {
                        name: format!("notify_{}_changed", fname),
                        return_type: "void".to_string(),
                        doc_comment: format!("Emits change notification for '{}'", fname),
                    });
                }
            }
            DecoratorKind::Equality => {
                members.push(SynthesizedMember {
                    name: "equals".to_string(),
                    return_type: "bool".to_string(),
                    doc_comment: "Structural field-by-field equality comparison".to_string(),
                });
                members.push(SynthesizedMember {
                    name: "hash_code".to_string(),
                    return_type: "int".to_string(),
                    doc_comment: "Computes deterministic 64-bit structural hash code".to_string(),
                });
            }
            DecoratorKind::Repr => {
                members.push(SynthesizedMember {
                    name: "to_string".to_string(),
                    return_type: "str".to_string(),
                    doc_comment: format!("Formats '{}' showing all fields and values", target_name),
                });
            }
            DecoratorKind::Metadata => {
                members.push(SynthesizedMember {
                    name: "__metadata__".to_string(),
                    return_type: "Map[str, str]".to_string(),
                    doc_comment: format!("Compile-time reflection metadata descriptor for '{}'", target_name),
                });
            }
            DecoratorKind::Custom(custom_name) => {
                self.diagnostics.push(AnalysisDiagnostic::warning(
                    "MACRO001",
                    format!("Custom decorator '@{}' evaluated via external macro handler", custom_name),
                    span,
                ));
            }
        }

        members
    }

    /// Checks if a node corresponds to a `comptime` or macro quotation construct.
    pub fn is_quote_node(arena: &AstArena, node_id: NodeId) -> bool {
        if node_id.is_none() {
            return false;
        }
        let node = *arena.node(node_id);
        node.node_kind() == NodeKind::ComptimeExpr
    }
}
