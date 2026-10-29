//! Milestone 31.1: Lazy Red Cursor Façade for the Concrete Syntax Tree.

use std::sync::Arc;
use pilot_span::ByteSpan;
use crate::green::{GreenElement, GreenNode, GreenToken};
use crate::kind::SyntaxKind;

/// A lazy cursor wrapping an immutable `GreenNode` with computed absolute UTF-8 byte offsets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxNode {
    green: Arc<GreenNode>,
    offset: u32,
}

impl SyntaxNode {
    #[inline]
    pub fn new_root(green: Arc<GreenNode>) -> Self {
        Self { green, offset: 0 }
    }

    #[inline]
    pub fn from_green(green: Arc<GreenNode>, offset: u32) -> Self {
        Self { green, offset }
    }

    #[inline]
    pub fn kind(&self) -> SyntaxKind {
        self.green.kind()
    }

    #[inline]
    pub fn offset(&self) -> u32 {
        self.offset
    }

    #[inline]
    pub fn text_len(&self) -> u32 {
        self.green.text_len()
    }

    #[inline]
    pub fn text_range(&self) -> ByteSpan {
        ByteSpan::new(self.offset, self.offset + self.green.text_len())
    }

    #[inline]
    pub fn green(&self) -> &Arc<GreenNode> {
        &self.green
    }

    /// Iterates over all immediate child elements (both nodes and tokens).
    pub fn children(&self) -> SyntaxElementIterator<'_> {
        SyntaxElementIterator {
            children: self.green.children(),
            index: 0,
            current_offset: self.offset,
        }
    }

    /// Iterates over all immediate child composite nodes.
    pub fn child_nodes(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.children().filter_map(|elem| match elem {
            SyntaxElement::Node(node) => Some(node),
            SyntaxElement::Token(_) => None,
        })
    }

    /// Iterates over all immediate child tokens.
    pub fn tokens(&self) -> impl Iterator<Item = SyntaxToken> + '_ {
        self.children().filter_map(|elem| match elem {
            SyntaxElement::Token(token) => Some(token),
            SyntaxElement::Node(_) => None,
        })
    }

    /// Recursively formats and reconstructs the full source text of this node.
    pub fn to_text(&self) -> String {
        self.green.to_text()
    }

    /// Recursively yields all descendant elements in pre-order traversal.
    pub fn descendants(&self) -> DescendantsIterator {
        DescendantsIterator {
            stack: vec![SyntaxElement::Node(self.clone())],
        }
    }

    /// Finds the leaf `SyntaxToken` containing or immediately preceding the given absolute byte offset.
    pub fn find_token_at_offset(&self, target_offset: u32) -> Option<SyntaxToken> {
        for elem in self.children() {
            let range = elem.text_range();
            if target_offset >= range.start && target_offset < range.end {
                match elem {
                    SyntaxElement::Node(child_node) => {
                        return child_node.find_token_at_offset(target_offset);
                    }
                    SyntaxElement::Token(token) => {
                        return Some(token);
                    }
                }
            }
        }
        None
    }
}

/// A leaf token in the Red cursor façade.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxToken {
    green: GreenToken,
    offset: u32,
}

impl SyntaxToken {
    #[inline]
    pub fn from_green(green: GreenToken, offset: u32) -> Self {
        Self { green, offset }
    }

    #[inline]
    pub fn kind(&self) -> SyntaxKind {
        self.green.kind()
    }

    #[inline]
    pub fn text(&self) -> &str {
        self.green.text()
    }

    #[inline]
    pub fn offset(&self) -> u32 {
        self.offset
    }

    #[inline]
    pub fn text_len(&self) -> u32 {
        self.green.text_len()
    }

    #[inline]
    pub fn text_range(&self) -> ByteSpan {
        ByteSpan::new(self.offset, self.offset + self.green.text_len())
    }

    #[inline]
    pub fn green(&self) -> &GreenToken {
        &self.green
    }
}

/// A node or leaf token in the Red cursor façade.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyntaxElement {
    Node(SyntaxNode),
    Token(SyntaxToken),
}

impl SyntaxElement {
    #[inline]
    pub fn kind(&self) -> SyntaxKind {
        match self {
            SyntaxElement::Node(node) => node.kind(),
            SyntaxElement::Token(token) => token.kind(),
        }
    }

    #[inline]
    pub fn text_range(&self) -> ByteSpan {
        match self {
            SyntaxElement::Node(node) => node.text_range(),
            SyntaxElement::Token(token) => token.text_range(),
        }
    }

    #[inline]
    pub fn is_node(&self) -> bool {
        matches!(self, SyntaxElement::Node(_))
    }

    #[inline]
    pub fn is_token(&self) -> bool {
        matches!(self, SyntaxElement::Token(_))
    }

    #[inline]
    pub fn as_node(&self) -> Option<&SyntaxNode> {
        match self {
            SyntaxElement::Node(node) => Some(node),
            SyntaxElement::Token(_) => None,
        }
    }

    #[inline]
    pub fn as_token(&self) -> Option<&SyntaxToken> {
        match self {
            SyntaxElement::Token(token) => Some(token),
            SyntaxElement::Node(_) => None,
        }
    }
}

pub struct SyntaxElementIterator<'a> {
    children: &'a [GreenElement],
    index: usize,
    current_offset: u32,
}

impl<'a> Iterator for SyntaxElementIterator<'a> {
    type Item = SyntaxElement;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.children.len() {
            return None;
        }

        let element = &self.children[self.index];
        let offset = self.current_offset;
        self.current_offset += element.text_len();
        self.index += 1;

        match element {
            GreenElement::Node(child_node) => {
                Some(SyntaxElement::Node(SyntaxNode::from_green(child_node.clone(), offset)))
            }
            GreenElement::Token(child_token) => {
                Some(SyntaxElement::Token(SyntaxToken::from_green(child_token.clone(), offset)))
            }
        }
    }
}

pub struct DescendantsIterator {
    stack: Vec<SyntaxElement>,
}

impl Iterator for DescendantsIterator {
    type Item = SyntaxElement;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.stack.pop()?;
        if let SyntaxElement::Node(ref node) = current {
            let mut children: Vec<_> = node.children().collect();
            children.reverse();
            self.stack.extend(children);
        }
        Some(current)
    }
}
