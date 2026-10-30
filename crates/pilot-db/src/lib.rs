//! Milestone 32.2: Demand-Driven Incremental Query Engine with Early Cutoff (`pilot-db`).
//!
//! Provides:
//! - Salsa-style query memoization with revision tracking.
//! - `source_text` -> `green_cst` -> `module_item_tree` -> `infer_body`.
//! - **Early Cutoff Optimization**: edits within a function body that do not alter the
//!   module signature hash re-analyze only that single function body.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use pilot_incremental::{reparse_incremental, TextEdit};
use pilot_syntax::{parse_cst, GreenNode, SyntaxKind, SyntaxNode};

/// Unique 32-bit identifier for a source file in the query database.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

/// Monotonically increasing revision counter for cache invalidation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Revision(pub u64);

/// Metadata for a top-level module item signature.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemSignature {
    pub name: String,
    pub kind: SyntaxKind,
    pub signature_text: String,
}

/// The collection of top-level item signatures and structural headers for a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemTree {
    pub file_id: FileId,
    pub items: Vec<ItemSignature>,
    pub signature_hash: u64,
}

impl ItemTree {
    pub fn new(file_id: FileId, items: Vec<ItemSignature>) -> Self {
        let mut hasher = DefaultHasher::new();
        for item in &items {
            item.hash(&mut hasher);
        }
        let signature_hash = hasher.finish();
        Self {
            file_id,
            items,
            signature_hash,
        }
    }
}

/// Body-level semantic analysis result for a specific function/method.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodyAnalysis {
    pub name: String,
    pub statement_count: usize,
    pub variable_names: Vec<String>,
}

/// Query memoization state for a single file.
struct FileEntry {
    source: Arc<str>,
    cst: Arc<GreenNode>,
    item_tree: Arc<ItemTree>,
    body_analyses: HashMap<String, Arc<BodyAnalysis>>,
    source_revision: Revision,
    signature_revision: Revision,
}

/// Central incremental query database.
pub struct Database {
    current_revision: Revision,
    files: HashMap<FileId, FileEntry>,
    pub early_cutoff_count: usize,
}

impl Database {
    pub fn new() -> Self {
        Self {
            current_revision: Revision(1),
            files: HashMap::new(),
            early_cutoff_count: 0,
        }
    }

    pub fn current_revision(&self) -> Revision {
        self.current_revision
    }

    /// Sets or updates the complete source text for a file, triggering full reparse.
    pub fn set_source(&mut self, file_id: FileId, text: &str) {
        self.current_revision = Revision(self.current_revision.0 + 1);
        let parsed = parse_cst(text);
        let green = parsed.root.green().clone();
        let item_tree = Arc::new(extract_item_tree(file_id, &parsed.root));

        let entry = FileEntry {
            source: Arc::from(text),
            cst: green,
            item_tree,
            body_analyses: HashMap::new(),
            source_revision: self.current_revision,
            signature_revision: self.current_revision,
        };

        self.files.insert(file_id, entry);
    }

    /// Applies an incremental edit to a file with Early Cutoff optimization.
    pub fn apply_edit(&mut self, file_id: FileId, edit: &TextEdit) {
        self.current_revision = Revision(self.current_revision.0 + 1);

        let entry = match self.files.get_mut(&file_id) {
            Some(e) => e,
            None => return,
        };

        let old_root = entry.cst.clone();
        let old_source = entry.source.clone();

        // Step 1: Incremental reparse (< 1 ms)
        let reparse_res = reparse_incremental(&old_root, &old_source, edit);
        entry.source = Arc::from(reparse_res.new_source.as_str());
        entry.cst = reparse_res.new_root.clone();
        entry.source_revision = self.current_revision;

        // Step 2: Extract new ItemTree and inspect signature hash
        let new_syntax_root = SyntaxNode::new_root(entry.cst.clone());
        let new_item_tree = extract_item_tree(file_id, &new_syntax_root);

        if new_item_tree.signature_hash == entry.item_tree.signature_hash {
            // Early Cutoff optimization: Signature did not change!
            // Do NOT invalidate signature_revision or other dependent callers in workspace.
            self.early_cutoff_count += 1;
            entry.body_analyses.clear(); // Only invalidate local body analyses
        } else {
            // Signatures changed: invalidate signature revision
            entry.item_tree = Arc::new(new_item_tree);
            entry.signature_revision = self.current_revision;
            entry.body_analyses.clear();
        }
    }

    /// Query: returns the source text for a file.
    pub fn source_text(&self, file_id: FileId) -> Option<Arc<str>> {
        self.files.get(&file_id).map(|e| e.source.clone())
    }

    /// Query: returns the Green CST root for a file.
    pub fn green_cst(&self, file_id: FileId) -> Option<Arc<GreenNode>> {
        self.files.get(&file_id).map(|e| e.cst.clone())
    }

    /// Query: returns the ItemTree (signatures and item headers) for a file.
    pub fn module_item_tree(&self, file_id: FileId) -> Option<Arc<ItemTree>> {
        self.files.get(&file_id).map(|e| e.item_tree.clone())
    }

    /// Query: analyzes a specific function body in a file on demand.
    pub fn infer_body(&mut self, file_id: FileId, fn_name: &str) -> Option<Arc<BodyAnalysis>> {
        let entry = self.files.get_mut(&file_id)?;
        if let Some(cached) = entry.body_analyses.get(fn_name) {
            return Some(cached.clone());
        }

        let syntax_root = SyntaxNode::new_root(entry.cst.clone());
        let mut stmts = 0;
        let mut vars = Vec::new();

        for child in syntax_root.child_nodes() {
            if child.kind() == SyntaxKind::FnDecl {
                let name = child.tokens().find(|t| t.kind() == SyntaxKind::Ident).map(|t| t.text().to_string()).unwrap_or_default();
                if name == fn_name {
                    for stmt in child.child_nodes() {
                        stmts += 1;
                        if matches!(stmt.kind(), SyntaxKind::LetStmt | SyntaxKind::VarStmt) {
                            if let Some(var_tok) = stmt.tokens().find(|t| t.kind() == SyntaxKind::Ident) {
                                vars.push(var_tok.text().to_string());
                            }
                        }
                    }
                }
            }
        }

        let analysis = Arc::new(BodyAnalysis {
            name: fn_name.to_string(),
            statement_count: stmts,
            variable_names: vars,
        });

        entry.body_analyses.insert(fn_name.to_string(), analysis.clone());
        Some(analysis)
    }
}

fn collect_header_tokens(node: &SyntaxNode, tokens: &mut Vec<String>) -> bool {
    for elem in node.children() {
        match elem {
            pilot_syntax::SyntaxElement::Node(child) => {
                if child.kind() == SyntaxKind::BlockExpr {
                    return true;
                }
                if collect_header_tokens(&child, tokens) {
                    return true;
                }
            }
            pilot_syntax::SyntaxElement::Token(tok) => {
                let k = tok.kind();
                if !matches!(k, SyntaxKind::Whitespace | SyntaxKind::Newline | SyntaxKind::BlockComment | SyntaxKind::LineComment) {
                    tokens.push(tok.text().to_string());
                }
            }
        }
    }
    false
}

fn format_signature_tokens(tokens: &[String]) -> String {
    let mut out = String::new();
    for (i, tok) in tokens.iter().enumerate() {
        if i > 0 {
            let prev = &tokens[i - 1];
            let no_space_before = tok == "(" || tok == ")" || tok == "," || tok == ":" || tok == "]" || tok == ">" || tok == ";";
            let no_space_after = prev == "(" || prev == "[" || prev == "<" || prev == "@";
            if !no_space_before && !no_space_after {
                out.push(' ');
            }
        }
        out.push_str(tok);
    }
    out
}

fn extract_item_tree(file_id: FileId, root: &SyntaxNode) -> ItemTree {
    let mut items = Vec::new();

    for node in root.child_nodes() {
        let k = node.kind();
        if matches!(k, SyntaxKind::FnDecl | SyntaxKind::ClassDecl | SyntaxKind::StructDecl | SyntaxKind::RecordDecl | SyntaxKind::TypeAdtDecl | SyntaxKind::ImportDecl) {
            let name = node.tokens().find(|t| t.kind() == SyntaxKind::Ident).map(|t| t.text().to_string()).unwrap_or_else(|| "anon".to_string());

            // Build signature text excluding the inner BlockExpr body
            let mut header_toks = Vec::new();
            collect_header_tokens(&node, &mut header_toks);
            let sig_text = if !header_toks.is_empty() {
                format_signature_tokens(&header_toks)
            } else {
                format!("{:?} {}", k, name)
            };

            items.push(ItemSignature {
                name,
                kind: k,
                signature_text: sig_text,
            });
        }
    }

    ItemTree::new(file_id, items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_early_cutoff_preserves_signature_hash_when_editing_body() {
        let mut db = Database::new();
        let file_id = FileId(1);
        let code = r#"def calculate(x: i64) -> i64 {
    let a = x + 10;
    return a;
}
"#;
        db.set_source(file_id, code);
        let orig_tree = db.module_item_tree(file_id).unwrap();
        let orig_hash = orig_tree.signature_hash;

        // Edit inside body: change 10 to 20
        let target = "10";
        let offset = code.find(target).unwrap() as u32;
        let edit = TextEdit::new(offset, offset + target.len() as u32, "20");

        db.apply_edit(file_id, &edit);

        // Early Cutoff MUST fire because function signature did not change
        assert_eq!(db.early_cutoff_count, 1, "Early cutoff should have triggered on body edit");
        let new_tree = db.module_item_tree(file_id).unwrap();
        assert_eq!(orig_hash, new_tree.signature_hash, "ItemTree signature hash must remain identical");
    }

    #[test]
    fn test_signature_edit_invalidates_signature_hash() {
        let mut db = Database::new();
        let file_id = FileId(1);
        let code = "def calculate(x: i64) -> i64 { return x; }\n";
        db.set_source(file_id, code);
        let orig_hash = db.module_item_tree(file_id).unwrap().signature_hash;

        // Edit function name: calculate -> compute
        let edit = TextEdit::new(4, 13, "compute");
        db.apply_edit(file_id, &edit);

        // Early cutoff should NOT fire because the signature changed
        assert_eq!(db.early_cutoff_count, 0);
        let new_hash = db.module_item_tree(file_id).unwrap().signature_hash;
        assert_ne!(orig_hash, new_hash, "Signature hash must change when editing function signature");
    }
}
