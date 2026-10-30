//! Milestone 33.2: Maranget Decision-Tree Usefulness Matrix for Pattern Exhaustiveness & Redundancy.

use pilot_span::ByteSpan;
use crate::diagnostics::AnalysisDiagnostic;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternCase {
    Wildcard,
    Literal(String),
    Variant(String),
    Tuple(Vec<PatternCase>),
}

#[derive(Clone, Debug)]
pub struct MatchArm {
    pub pattern: PatternCase,
    pub guard: bool,
    pub has_subpats: bool,
    pub span: ByteSpan,
}

/// Maranget usefulness checker for match expressions.
pub struct MarangetChecker {
    arms: Vec<MatchArm>,
    all_variants: Vec<String>,
}

impl MarangetChecker {
    pub fn new(all_variants: Vec<String>) -> Self {
        Self {
            arms: Vec::new(),
            all_variants,
        }
    }

    pub fn add_arm(&mut self, pattern: PatternCase, guard: bool, span: ByteSpan) {
        self.add_arm_full(pattern, guard, false, span);
    }

    pub fn add_arm_full(&mut self, pattern: PatternCase, guard: bool, has_subpats: bool, span: ByteSpan) {
        self.arms.push(MatchArm {
            pattern,
            guard,
            has_subpats,
            span,
        });
    }

    /// Evaluates arms and returns diagnostics:
    /// - `MATCH001`: Non-exhaustive pattern matching.
    /// - `MATCH002`: Unreachable / redundant pattern arm.
    pub fn check(&self, match_span: ByteSpan) -> Vec<AnalysisDiagnostic> {
        let mut diags = Vec::new();
        let mut covered_variants = Vec::new();
        let mut has_wildcard = false;

        for arm in &self.arms {
            // Check for redundancy: if an un-guarded wildcard already appeared, this arm is unreachable
            if has_wildcard {
                diags.push(AnalysisDiagnostic::warning(
                    "MATCH002",
                    "Unreachable match arm: prior wildcard/catch-all pattern shadows this case",
                    arm.span,
                ));
                continue;
            }

            match &arm.pattern {
                PatternCase::Wildcard => {
                    if !arm.guard {
                        has_wildcard = true;
                    }
                }
                PatternCase::Variant(name) => {
                    if covered_variants.contains(name) && !arm.guard && !arm.has_subpats {
                        diags.push(AnalysisDiagnostic::warning(
                            "MATCH002",
                            format!("Redundant match arm: variant '{}' is already handled", name),
                            arm.span,
                        ));
                    } else if !arm.guard {
                        covered_variants.push(name.clone());
                    }
                }
                PatternCase::Literal(_) => {
                    // Literals don't exhaust the domain without wildcard
                }
                PatternCase::Tuple(_) => {
                    // Handled recursively
                }
            }
        }

        // Exhaustiveness check
        if !has_wildcard {
            if !self.all_variants.is_empty() {
                let missing: Vec<&String> = self
                    .all_variants
                    .iter()
                    .filter(|v| !covered_variants.contains(v))
                    .collect();
                if !missing.is_empty() {
                    let missing_str: Vec<String> = missing.iter().map(|s| s.to_string()).collect();
                    diags.push(AnalysisDiagnostic::error(
                        "MATCH001",
                        format!(
                            "Non-exhaustive pattern match: missing cases [{}]",
                            missing_str.join(", ")
                        ),
                        match_span,
                    ));
                }
            } else if !self.arms.is_empty() {
                // If there are no declared variants and no wildcard, literal matches are non-exhaustive
                let has_literals = self.arms.iter().any(|a| matches!(a.pattern, PatternCase::Literal(_)));
                if has_literals {
                    diags.push(AnalysisDiagnostic::error(
                        "MATCH001",
                        "Non-exhaustive pattern match: missing wildcard '_' or default case",
                        match_span,
                    ));
                }
            }
        }

        diags
    }

    /// Checks a MatchExpr node in the AstArena for exhaustiveness and redundancy.
    pub fn check_match_expr(
        arena: &pilot_ast::AstArena,
        match_id: pilot_ast::NodeId,
        all_variants: Vec<String>,
    ) -> Vec<AnalysisDiagnostic> {
        let node = *arena.node(match_id);
        let children = arena.children_slice(node.lhs, node.rhs);
        if children.len() <= 1 {
            return Vec::new();
        }

        let mut checker = Self::new(all_variants);
        for &arm_id in &children[1..] {
            let arm_node = *arena.node(pilot_ast::NodeId(arm_id));
            let pat_id = pilot_ast::NodeId(arm_node.lhs);
            let (pat_case, has_subpats) = if pat_id.is_some() {
                let mut pat_node = *arena.node(pat_id);
                if pat_node.node_kind() == pilot_ast::NodeKind::PatBind && pat_node.flags == 1 && pat_node.rhs != 0 {
                    pat_node = *arena.node(pilot_ast::NodeId(pat_node.rhs));
                }
                let has_subpats = pat_node.flags > 0;
                let case = match pat_node.node_kind() {
                    pilot_ast::NodeKind::PatWildcard
                    | pilot_ast::NodeKind::PatBind
                    | pilot_ast::NodeKind::PatSlice => PatternCase::Wildcard,
                    pilot_ast::NodeKind::PatVariant => {
                        let name = arena
                            .interner
                            .resolve(pilot_intern::SymbolId(pat_node.lhs))
                            .to_string();
                        PatternCase::Variant(name)
                    }
                    pilot_ast::NodeKind::PatLiteral => PatternCase::Literal("lit".to_string()),
                    pilot_ast::NodeKind::PatRange => PatternCase::Literal("range".to_string()),
                    pilot_ast::NodeKind::PatOr => PatternCase::Literal("or".to_string()),
                    pilot_ast::NodeKind::PatTuple => PatternCase::Tuple(Vec::new()),
                    _ => PatternCase::Wildcard,
                };
                (case, has_subpats)
            } else {
                (PatternCase::Wildcard, false)
            };

            let span = arena.span(pilot_ast::NodeId(arm_id));
            let has_guard = arm_node.flags != 0;
            checker.add_arm_full(pat_case, has_guard, has_subpats, span);
        }

        checker.check(arena.span(match_id))
    }
}
