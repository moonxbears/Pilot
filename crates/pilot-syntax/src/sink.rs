//! Milestone 31.1 & 31.2: TreeSink Trait and LosslessCstSink Implementation.

use crate::green::{GreenElement, GreenNode, GreenToken};
use crate::kind::SyntaxKind;
use crate::recovery::SyntaxDiagnostic;
use crate::red::SyntaxNode;

/// Monomorphized sink interface for emitting syntax trees directly during parsing.
pub trait TreeSink {
    fn token(&mut self, kind: SyntaxKind, text: &str);
    fn start_node(&mut self, kind: SyntaxKind);
    fn finish_node(&mut self);
    fn error(&mut self, diagnostic: SyntaxDiagnostic);
}

/// Lossless Concrete Syntax Tree sink accumulating green tokens and nodes,
/// preserving 100% comments, whitespace, and #regions.
pub struct LosslessCstSink {
    stack: Vec<(SyntaxKind, Vec<GreenElement>)>,
    diagnostics: Vec<SyntaxDiagnostic>,
}

impl LosslessCstSink {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(32),
            diagnostics: Vec::new(),
        }
    }

    pub fn finish(mut self) -> (SyntaxNode, Vec<SyntaxDiagnostic>) {
        // Close any dangling nodes if necessary
        while self.stack.len() > 1 {
            let (kind, children) = self.stack.pop().unwrap();
            let parent = self.stack.last_mut().unwrap();
            parent.1.push(GreenElement::Node(GreenNode::new(kind, children)));
        }

        let root_green = if let Some((kind, children)) = self.stack.pop() {
            GreenNode::new(kind, children)
        } else {
            GreenNode::new(SyntaxKind::Program, Vec::new())
        };

        (SyntaxNode::new_root(root_green), self.diagnostics)
    }

    pub fn diagnostics(&self) -> &[SyntaxDiagnostic] {
        &self.diagnostics
    }
}

impl TreeSink for LosslessCstSink {
    fn token(&mut self, kind: SyntaxKind, text: &str) {
        let green_tok = GreenToken::new(kind, text);
        if let Some(top) = self.stack.last_mut() {
            top.1.push(GreenElement::Token(green_tok));
        } else {
            // Root-level token without a surrounding node
            let mut root_children = Vec::new();
            root_children.push(GreenElement::Token(green_tok));
            self.stack.push((SyntaxKind::Program, root_children));
        }
    }

    fn start_node(&mut self, kind: SyntaxKind) {
        self.stack.push((kind, Vec::new()));
    }

    fn finish_node(&mut self) {
        if self.stack.len() <= 1 {
            return;
        }
        let (kind, children) = self.stack.pop().unwrap();
        let green_node = GreenNode::new(kind, children);
        if let Some(parent) = self.stack.last_mut() {
            parent.1.push(GreenElement::Node(green_node));
        }
    }

    fn error(&mut self, diagnostic: SyntaxDiagnostic) {
        self.diagnostics.push(diagnostic);
    }
}
