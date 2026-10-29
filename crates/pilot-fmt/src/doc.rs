//! Milestone 31.3: Wadler-Lindig Document Algebra for high-performance formatting.

use std::borrow::Cow;

/// Wadler-Lindig document algebra node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Doc<'a> {
    Nil,
    Text(Cow<'a, str>),
    Line,         // Line break if group is broken, single space if group fits on line
    HardLine,     // Unconditional line break
    Space,        // Literal single space " "
    Indent(Box<Doc<'a>>),
    Concat(Vec<Doc<'a>>),
    Group(Box<Doc<'a>>),
}

impl<'a> Doc<'a> {
    #[inline]
    pub fn nil() -> Self {
        Doc::Nil
    }

    #[inline]
    pub fn text(s: impl Into<Cow<'a, str>>) -> Self {
        Doc::Text(s.into())
    }

    #[inline]
    pub fn line() -> Self {
        Doc::Line
    }

    #[inline]
    pub fn hard_line() -> Self {
        Doc::HardLine
    }

    #[inline]
    pub fn space() -> Self {
        Doc::Space
    }

    #[inline]
    pub fn indent(doc: Doc<'a>) -> Self {
        Doc::Indent(Box::new(doc))
    }

    #[inline]
    pub fn group(doc: Doc<'a>) -> Self {
        Doc::Group(Box::new(doc))
    }

    #[inline]
    pub fn concat(docs: Vec<Doc<'a>>) -> Self {
        Doc::Concat(docs)
    }

    /// Measures whether a document fits within `remaining` columns in flat (unbroken) mode.
    pub fn fits(&self, remaining: isize) -> bool {
        if remaining < 0 {
            return false;
        }
        match self {
            Doc::Nil => true,
            Doc::Text(s) => (s.len() as isize) <= remaining,
            Doc::Line => 1 <= remaining, // becomes a space in flat mode
            Doc::HardLine => false,      // hard line never fits flat
            Doc::Space => 1 <= remaining,
            Doc::Indent(inner) => inner.fits(remaining),
            Doc::Group(inner) => inner.fits(remaining),
            Doc::Concat(children) => {
                let mut rem = remaining;
                for child in children {
                    if !child.fits(rem) {
                        return false;
                    }
                    rem -= child.flat_len();
                }
                true
            }
        }
    }

    /// Computes the horizontal length of the document when rendered completely flat.
    pub fn flat_len(&self) -> isize {
        match self {
            Doc::Nil => 0,
            Doc::Text(s) => s.len() as isize,
            Doc::Line => 1,
            Doc::HardLine => 100_000,
            Doc::Space => 1,
            Doc::Indent(inner) => inner.flat_len(),
            Doc::Group(inner) => inner.flat_len(),
            Doc::Concat(children) => children.iter().map(|c| c.flat_len()).sum(),
        }
    }

    /// Renders the document to a string with the given options.
    pub fn render(&self, max_width: usize, indent_str: &str) -> String {
        let mut out = String::with_capacity(256);
        let mut stack: Vec<(usize, bool, &Doc<'a>)> = vec![(0, false, self)];

        while let Some((indent_level, broken, doc)) = stack.pop() {
            match doc {
                Doc::Nil => {}
                Doc::Text(s) => {
                    out.push_str(s);
                }
                Doc::Space => {
                    out.push(' ');
                }
                Doc::Line => {
                    if broken {
                        out.push('\n');
                        for _ in 0..indent_level {
                            out.push_str(indent_str);
                        }
                    } else {
                        out.push(' ');
                    }
                }
                Doc::HardLine => {
                    out.push('\n');
                    for _ in 0..indent_level {
                        out.push_str(indent_str);
                    }
                }
                Doc::Indent(inner) => {
                    stack.push((indent_level + 1, broken, inner));
                }
                Doc::Group(inner) => {
                    // Check if inner fits on remaining line
                    let current_line_len = out.rfind('\n').map_or(out.len(), |pos| out.len() - pos - 1);
                    let rem = (max_width as isize) - (current_line_len as isize);
                    let should_break = !inner.fits(rem);
                    stack.push((indent_level, should_break, inner));
                }
                Doc::Concat(children) => {
                    for child in children.iter().rev() {
                        stack.push((indent_level, broken, child));
                    }
                }
            }
        }

        out
    }
}
