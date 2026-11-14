//! LINQ-style query expressions: `from x in xs where cond order by key [ascending|descending] select e`.
//!
//! Queries over in-memory sequences desugar to collecting `for` loops (and,
//! when `order` clauses are present, a stable insertion sort over the
//! collected key/value lists) so they run identically on every engine without
//! hardcoded type or method names.

use crate::Parser;
use pilot_ast::{BinaryOp, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_lexer::TokenKind;
use pilot_span::ByteSpan;

const REC_WHERE: u8 = 28;
const REC_IN: u8 = 10;
const REC_QUERY: u8 = 33;
const SUB_SELECT: u8 = 7;
const SUB_ORDER: u8 = 8;
const SUB_ASCENDING: u8 = 9;
const SUB_DESCENDING: u8 = 10;
const SUB_BY: u8 = 11;

impl Parser<'_> {
    pub(crate) fn parse_linq_query(&mut self) -> NodeId {
        let sp = self.span();
        self.advance();
        let vsym = self.intern_cur();
        if self.is_rec(REC_IN) {
            self.advance();
        }
        let src_e = self.parse_expr_bp(20);
        let mut conds = Vec::new();
        let mut orders: Vec<(NodeId, bool)> = Vec::new();
        while !self.at_end() && self.receptor() != Some((REC_QUERY, SUB_SELECT)) {
            if self.is_rec(REC_WHERE) {
                self.advance();
                conds.push(self.parse_expr_bp(20));
            } else if self.receptor() == Some((REC_QUERY, SUB_ORDER)) {
                self.advance();
                if self.receptor() == Some((REC_QUERY, SUB_BY)) {
                    self.advance();
                }
                loop {
                    let key = self.parse_expr_bp(20);
                    let desc = if self.receptor() == Some((REC_QUERY, SUB_ASCENDING)) {
                        self.advance();
                        false
                    } else if self.receptor() == Some((REC_QUERY, SUB_DESCENDING)) {
                        self.advance();
                        true
                    } else {
                        false
                    };
                    orders.push((key, desc));
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
            } else if matches!(
                self.receptor(),
                Some((REC_QUERY, SUB_ASCENDING)) | Some((REC_QUERY, SUB_DESCENDING))
            ) {
                let (csp, word) = (self.span(), self.text().to_string());
                self.arena.diagnostics.push((
                    csp,
                    format!("`{word}` must follow `order by <key>`"),
                ));
                return NodeId::NONE;
            } else if self.at_unbuilt_clause() {
                let (csp, word) = (self.span(), self.text().to_string());
                self.report_unbuilt(csp, &word);
                return NodeId::NONE;
            } else {
                let (csp, word) = (self.span(), self.text().to_string());
                self.arena.diagnostics.push((
                    csp,
                    format!(
                        "query clause `{word}` is not supported: a query is `from x in xs`, `where cond`, `order by key [ascending|descending]`, then `select e`"
                    ),
                ));
                return NodeId::NONE;
            }
        }
        if self.receptor() != Some((REC_QUERY, SUB_SELECT)) {
            let csp = self.span();
            self.arena
                .diagnostics
                .push((csp, "a query must end with `select e`".to_string()));
            return NodeId::NONE;
        }
        self.advance();
        let proj = self.parse_expr_bp(15);
        let span = ByteSpan::new(sp.start, self.prev_span().end);
        if orders.is_empty() {
            return self.build_query_loop(vsym, src_e, &conds, proj, span);
        }
        self.build_ordered_query(vsym, src_e, &conds, &orders, proj, span)
    }

    fn build_query_loop(
        &mut self,
        vsym: SymbolId,
        src: NodeId,
        conds: &[NodeId],
        yield_expr: NodeId,
        span: ByteSpan,
    ) -> NodeId {
        let mut item = self.arena.alloc_node(NodeKind::YieldStmt, 0, span, yield_expr.0, 0);
        for &c in conds.iter().rev() {
            let then_b = self.arena.alloc_list_node(NodeKind::BlockExpr, 0, span, &[item]);
            item = self.arena.alloc_node(NodeKind::IfExpr, 0, span, c.0, then_b.0);
        }
        let body = self.arena.alloc_list_node(NodeKind::BlockExpr, 0, span, &[item]);
        self.arena.alloc_for(vsym, src, body, NodeId::NONE, span)
    }

    fn build_ordered_query(
        &mut self,
        vsym: SymbolId,
        src: NodeId,
        conds: &[NodeId],
        orders: &[(NodeId, bool)],
        proj: NodeId,
        span: ByteSpan,
    ) -> NodeId {
        let tag = span.start;
        let items_sym = self.arena.interner.intern(&format!("__q_items_{tag}"));
        let vals_sym = self.arena.interner.intern(&format!("__q_vals_{tag}"));
        let n_sym = self.arena.interner.intern(&format!("__q_n_{tag}"));
        let blank_sym = self.arena.interner.intern("_");

        let var_ident = self.arena.alloc_node(NodeKind::IdentExpr, 0, span, vsym.0, 0);
        let items_loop = self.build_query_loop(vsym, src, conds, var_ident, span);
        let items_let = self.arena.alloc_node(NodeKind::LetStmt, 0, span, items_sym.0, items_loop.0);

        let items_ref = self.arena.alloc_node(NodeKind::IdentExpr, 0, span, items_sym.0, 0);
        let vals_loop = self.build_query_loop(vsym, items_ref, &[], proj, span);
        let vals_var = self.arena.alloc_node(NodeKind::VarStmt, 0, span, vals_sym.0, vals_loop.0);

        let zero = self.q_int(0, span);
        let one = self.q_int(1, span);
        let n_var = self.arena.alloc_node(NodeKind::VarStmt, 0, span, n_sym.0, zero.0);

        let n_id = self.q_ident(n_sym, span);
        let n_plus_1 = self.q_bin(BinaryOp::Add, n_id, one, span);
        let n_inc = self.arena.alloc_node(NodeKind::AssignStmt, 0, span, n_id.0, n_plus_1.0);
        let count_body = self.arena.alloc_list_node(NodeKind::BlockExpr, 0, span, &[n_inc]);
        let items_ref2 = self.q_ident(items_sym, span);
        let count_for = self.arena.alloc_for(blank_sym, items_ref2, count_body, NodeId::NONE, span);

        let mut stmts = vec![items_let, vals_var, n_var, count_for];
        let multi = orders.len() > 1;
        for (idx, &(key, desc)) in orders.iter().enumerate().rev() {
            let keys_sym = self.arena.interner.intern(&format!("__q_keys_{tag}_{idx}"));
            let i_sym = self.arena.interner.intern(&format!("__q_i_{tag}_{idx}"));
            let j_sym = self.arena.interner.intern(&format!("__q_j_{tag}_{idx}"));
            let tk_sym = self.arena.interner.intern(&format!("__q_tk_{tag}_{idx}"));
            let tv_sym = self.arena.interner.intern(&format!("__q_tv_{tag}_{idx}"));

            let items_ref3 = self.q_ident(items_sym, span);
            let keys_loop = self.build_query_loop(vsym, items_ref3, &[], key, span);
            let keys_var = self.arena.alloc_node(NodeKind::VarStmt, 0, span, keys_sym.0, keys_loop.0);
            stmts.push(keys_var);

            let i_var = self.arena.alloc_node(NodeKind::VarStmt, 0, span, i_sym.0, one.0);
            stmts.push(i_var);

            let i_id = self.q_ident(i_sym, span);
            let n_id2 = self.q_ident(n_sym, span);
            let outer_cond = self.q_bin(BinaryOp::Lt, i_id, n_id2, span);

            let j_var = self.arena.alloc_node(NodeKind::VarStmt, 0, span, j_sym.0, i_id.0);
            let j_id = self.q_ident(j_sym, span);
            let j_prev = self.q_bin(BinaryOp::Sub, j_id, one, span);
            let j_pos = self.q_bin(BinaryOp::Gt, j_id, zero, span);
            let k_prev = self.q_index(keys_sym, j_prev, span);
            let k_cur = self.q_index(keys_sym, j_id, span);
            let cmp_op = if desc { BinaryOp::Lt } else { BinaryOp::Gt };
            let k_cmp = self.q_bin(cmp_op, k_prev, k_cur, span);
            let inner_cond = self.q_bin(BinaryOp::And, j_pos, k_cmp, span);

            let tk_let = self.arena.alloc_node(NodeKind::LetStmt, 0, span, tk_sym.0, k_prev.0);
            let k_set1 = self.arena.alloc_node(NodeKind::AssignStmt, 0, span, k_prev.0, k_cur.0);
            let tk_id = self.q_ident(tk_sym, span);
            let k_set2 = self.arena.alloc_node(NodeKind::AssignStmt, 0, span, k_cur.0, tk_id.0);

            let v_prev = self.q_index(vals_sym, j_prev, span);
            let v_cur = self.q_index(vals_sym, j_id, span);
            let tv_let = self.arena.alloc_node(NodeKind::LetStmt, 0, span, tv_sym.0, v_prev.0);
            let v_set1 = self.arena.alloc_node(NodeKind::AssignStmt, 0, span, v_prev.0, v_cur.0);
            let tv_id = self.q_ident(tv_sym, span);
            let v_set2 = self.arena.alloc_node(NodeKind::AssignStmt, 0, span, v_cur.0, tv_id.0);

            let mut inner_stmts = vec![tk_let, k_set1, k_set2, tv_let, v_set1, v_set2];
            if multi && idx > 0 {
                let ti_sym = self.arena.interner.intern(&format!("__q_ti_{tag}_{idx}"));
                let it_prev = self.q_index(items_sym, j_prev, span);
                let it_cur = self.q_index(items_sym, j_id, span);
                let ti_let = self.arena.alloc_node(NodeKind::LetStmt, 0, span, ti_sym.0, it_prev.0);
                let it_set1 = self.arena.alloc_node(NodeKind::AssignStmt, 0, span, it_prev.0, it_cur.0);
                let ti_id = self.q_ident(ti_sym, span);
                let it_set2 = self.arena.alloc_node(NodeKind::AssignStmt, 0, span, it_cur.0, ti_id.0);
                inner_stmts.extend([ti_let, it_set1, it_set2]);
            }
            let j_dec = self.arena.alloc_node(NodeKind::AssignStmt, 0, span, j_id.0, j_prev.0);
            inner_stmts.push(j_dec);

            let inner_body = self.arena.alloc_list_node(NodeKind::BlockExpr, 0, span, &inner_stmts);
            let inner_while = self.arena.alloc_node(NodeKind::WhileStmt, 0, span, inner_cond.0, inner_body.0);

            let i_next = self.q_bin(BinaryOp::Add, i_id, one, span);
            let i_inc = self.arena.alloc_node(NodeKind::AssignStmt, 0, span, i_id.0, i_next.0);
            let outer_body = self.arena.alloc_list_node(NodeKind::BlockExpr, 0, span, &[j_var, inner_while, i_inc]);
            let outer_while = self.arena.alloc_node(NodeKind::WhileStmt, 0, span, outer_cond.0, outer_body.0);
            stmts.push(outer_while);
        }

        let vals_out = self.q_ident(vals_sym, span);
        let vals_stmt = self.arena.alloc_node(NodeKind::ExprStmt, 0, span, vals_out.0, 0);
        stmts.push(vals_stmt);
        self.arena.alloc_list_node(NodeKind::BlockExpr, 0, span, &stmts)
    }

    fn q_ident(&mut self, sym: SymbolId, span: ByteSpan) -> NodeId {
        self.arena.alloc_node(NodeKind::IdentExpr, 0, span, sym.0, 0)
    }

    fn q_int(&mut self, val: i64, span: ByteSpan) -> NodeId {
        let idx = self.arena.i64_pool.len() as u32;
        self.arena.i64_pool.push(val);
        self.arena.alloc_node(NodeKind::IntLit, 0, span, idx, 0)
    }

    fn q_bin(&mut self, op: BinaryOp, lhs: NodeId, rhs: NodeId, span: ByteSpan) -> NodeId {
        self.arena.alloc_node(NodeKind::BinaryExpr, op as u16, span, lhs.0, rhs.0)
    }

    fn q_index(&mut self, arr: SymbolId, idx: NodeId, span: ByteSpan) -> NodeId {
        let arr_id = self.q_ident(arr, span);
        self.arena.alloc_node(NodeKind::IndexExpr, 0, span, arr_id.0, idx.0)
    }
}
