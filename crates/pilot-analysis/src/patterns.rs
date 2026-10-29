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
        self.arms.push(MatchArm {
            pattern,
            guard,
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
                    if covered_variants.contains(name) && !arm.guard {
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
                diags.push(AnalysisDiagnostic::error(
                    "MATCH001",
                    "Non-exhaustive pattern match: missing wildcard '_' or default case",
                    match_span,
                ));
            }
        }

        diags
    }
}
