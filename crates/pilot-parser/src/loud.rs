//! Forms the parser reads but no engine supports yet. Each is reported with a
//! diagnostic instead of being parsed and thrown away (AGENTS.md section 11,
//! rule 2): a misremembered form must fail, never run as something else.
//! Two forms once refused here are now kept for the engines: the step of
//! `a..b : step` and a literal parameter default.

use crate::Parser;
use pilot_ast::{NodeId, NodeKind};
use pilot_lexer::TokenKind;
use pilot_span::ByteSpan;

impl Parser<'_> {
    fn report_loud(&mut self, start: u32, message: String) {
        let span = ByteSpan::new(start, self.prev_span().end.max(start));
        self.arena.diagnostics.push((span, message));
    }

    /// A function body: a block in which visibility words have no meaning.
    pub(crate) fn parse_fn_body(&mut self) -> NodeId {
        self.fn_body_depth += 1;
        let body = self.parse_block();
        self.fn_body_depth -= 1;
        body
    }

    /// Whether the braces at the cursor hold a `;` at their own depth: they are
    /// a block of statements (`{ i = i + 1; }`), never a map or record.
    pub(crate) fn braces_hold_statement(&self) -> bool {
        let Some(close) = self.tape.matching_bracket(self.cur()) else {
            return false;
        };
        let mut depth = 0i32;
        for i in (self.cur() + 1)..(close as usize) {
            match self.tape.kind(i) {
                TokenKind::LBrace | TokenKind::LParen | TokenKind::LBracket => depth += 1,
                TokenKind::RBrace | TokenKind::RParen | TokenKind::RBracket => depth -= 1,
                TokenKind::Semicolon if depth == 0 => return true,
                _ => {}
            }
        }
        false
    }

    /// `*args` / `**kwargs` before a parameter name: read past and reported.
    pub(crate) fn rest_param_loudly(&mut self) {
        if matches!(self.peek(), TokenKind::Star | TokenKind::DoubleStar) {
            let start = self.span().start;
            self.advance();
            self.report_loud(
                start,
                "a parameter that gathers the rest of the arguments (`*args`, `**kwargs`) is not built yet".into(),
            );
        }
    }

    /// `= value` after a parameter: its default, which a call that leaves
    /// the argument out is given. Only a literal is accepted for now, since
    /// the call site, not the function, fills it in.
    pub(crate) fn param_default(&mut self) -> NodeId {
        if !self.at(TokenKind::Equal) {
            return NodeId::NONE;
        }
        let start = self.span().start;
        self.advance();
        let value = self.parse_expr();
        if !self.is_literal(value) {
            self.report_loud(
                start,
                "a default parameter value must be a literal for now (`= 4`, `= -1`, `= \"x\"`, `= true`)".into(),
            );
            return NodeId::NONE;
        }
        value
    }

    /// A literal, or `-` on a number literal.
    fn is_literal(&self, n: NodeId) -> bool {
        if n == NodeId::NONE {
            return false;
        }
        let node = self.arena.node(n);
        match node.node_kind() {
            NodeKind::IntLit | NodeKind::FloatLit | NodeKind::BoolLit | NodeKind::StrLit => true,
            NodeKind::UnaryExpr if node.flags == 0 && NodeId(node.lhs) != NodeId::NONE => matches!(
                self.arena.node(NodeId(node.lhs)).node_kind(),
                NodeKind::IntLit | NodeKind::FloatLit
            ),
            _ => false,
        }
    }

    /// Generic parameters where `what` takes none yet (`enum Box<T>`,
    /// `let x<T>`): read past and reported.
    pub(crate) fn generics_loudly(&mut self, what: &str) {
        let start = self.span().start;
        if !self.parse_generic_params().is_empty() {
            self.report_loud(start, format!("{what} takes no generic parameters yet"));
        }
    }

    /// `catch e: T`: the type would not be checked, so it is refused.
    pub(crate) fn catch_type_loudly(&mut self) {
        if self.at(TokenKind::Colon) {
            let start = self.span().start;
            self.advance();
            let _ = self.parse_type();
            self.report_loud(start, "a typed `catch e: T` is not checked yet; write `catch e`".into());
        }
    }

    /// `where` clauses after a type's header: read past and reported, since
    /// nothing checks them.
    pub(crate) fn type_where_loudly(&mut self) {
        while self.is_rec(28) {
            let start = self.span().start;
            self.advance();
            while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Semicolon) {
                self.advance();
            }
            self.report_loud(start, "a `where` clause on a type declaration is not checked yet".into());
        }
    }

    /// `~self(..)`: a destructor takes no parameters.
    pub(crate) fn dtor_params_loudly(&mut self) {
        let start = self.span().start;
        if !self.parse_param_list().is_empty() {
            self.report_loud(start, "`~self()` takes no parameters".into());
        }
    }

    /// `x[a, b]`: each further index is read past and reported.
    pub(crate) fn extra_indices_loudly(&mut self) {
        while self.at(TokenKind::Comma) {
            let start = self.span().start;
            self.advance();
            if self.at(TokenKind::RBracket) {
                break;
            }
            let _ = self.parse_expr();
            self.report_loud(start, "an index takes one value; indexing by several (`x[a, b]`) is not built yet".into());
        }
    }

    /// `a..b : step`: a `Step` node whose lhs is the range and whose rhs is
    /// the step; without `:` the range is returned as it is.
    pub(crate) fn range_step(&mut self, range: NodeId, bp: u8) -> NodeId {
        if !self.at(TokenKind::Colon) {
            return range;
        }
        self.advance();
        let step = self.parse_expr_bp(bp);
        let span = ByteSpan::new(self.arena.span(range).start, self.prev_span().end);
        let op = pilot_ast::BinaryOp::Step as u16;
        self.arena.alloc_node(pilot_ast::NodeKind::BinaryExpr, op, span, range.0, step.0)
    }

    /// `for (a, b) in ..`: the pattern is read past and reported.
    pub(crate) fn for_pattern_loudly(&mut self) {
        let start = self.span().start;
        let _ = self.parse_pattern();
        self.report_loud(start, "a pattern in `for` (`for (a, b) in ..`) is not built yet; bind one name".into());
    }

    /// `name?: T` on a field: `?` belongs to the type (`name: T?`).
    pub(crate) fn field_question_loudly(&mut self) {
        if self.at(TokenKind::Question) {
            let start = self.span().start;
            self.advance();
            self.report_loud(start, "an optional field is written `name: T?`, not `name?: T`".into());
        }
    }
}
