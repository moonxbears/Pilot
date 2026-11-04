//! Milestone 33.2 & Self-Hosting Phase 4: Dual-Path 3-Tier `ForExpr` & `IEnumerable<T>` Loop Analyzer (`LOOP001`–`LOOP004`).

use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_span::ByteSpan;
use crate::diagnostics::AnalysisDiagnostic;
use crate::infer::TypeInferenceEngine;
use crate::types::{TypeId, TypeKind};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LoopScheduleMode {
    /// Tier 1: Fixed scalar range bounds (`for i in 0 .. 10` or `for i in 0 ..< 10`)
    FixedScalar,
    /// Tier 2: Mutable bounds updated via `continue a .. b;`
    MutableBounds,
    /// Tier 3: Worklist stack backed by `#[repr(C)] RangeWindow<TIter>` via `continue a .. b, ...;`
    WorklistStack,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LoopReturnMode {
    /// Statement-position loop returning `void`
    StatementVoid,
    /// Expression-position comprehension yielding into a 24-byte `SliceBuffer<T>` (`yield val;`)
    CollectionYield { elem_ty: TypeId },
    /// Expression-position search loop returning a scalar or `option<T>` (`break val;` + optional `then fallback`)
    ScalarBreak { value_ty: TypeId, has_then_fallback: bool },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ForLoweringStrategy {
    /// Path A: Milestone 15 3-Tier Engine (`FixedScalar`, `MutableBounds`, `WorklistStack` with `RangeWindow<TIter>`)
    /// Used when iterating over `range<T>` (`..<`), `span<T>` (`..`/`..=`), `step<T>`, OR when the loop
    /// is in expression position (`CollectionYield` -> `SliceBuffer<T>` / `ScalarBreak` -> `then`),
    /// OR when the loop body contains `continue r` / `continue r, ...`.
    ThreeTierDynamicSchedule {
        schedule: LoopScheduleMode,
        ret_mode: LoopReturnMode,
        can_preallocate_yield_buffer: bool,
    },
    /// Path B: C#-Style `IEnumerable<T>` Statement Iterator Protocol
    /// Desugars to `var __iter = iterable.get_enumerator(); while __iter.move_next() { let item = __iter.current; ... }`
    /// and fuses stack coroutines when `op for () -> T` is present.
    EnumerableProtocol {
        enumerator_ty: TypeId,
        item_ty: TypeId,
    },
}

pub type ForLoopAnalyzer = LoopAnalyzer;

pub struct LoopAnalyzer {
    pub diagnostics: Vec<AnalysisDiagnostic>,
}

impl Default for LoopAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LoopAnalyzer {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn report_non_iterable(&mut self, ty_name: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "LOOP001",
            format!("Type '{}' is not iterable: must implement iterator protocol, range, or collection", ty_name),
            span,
        ));
    }

    pub fn report_mixed_break_and_yield(&mut self, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "LOOP001",
            "Cannot mix 'break <expr>' and 'yield <expr>' inside the same 'for' expression",
            span,
        ));
    }

    pub fn report_mutation_during_iteration(&mut self, coll_name: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "LOOP002",
            format!("Illegal concurrent modification: cannot mutate collection '{}' while actively iterating over it", coll_name),
            span,
        ));
    }

    pub fn report_mixed_void_and_valued_break(&mut self, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "LOOP002",
            "Cannot mix bare 'break;' with value-returning 'break <expr>;' in the same 'for' expression",
            span,
        ));
    }

    pub fn report_zero_step_range(&mut self, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "LOOP003",
            "Range step argument cannot be zero",
            span,
        ));
    }

    pub fn report_then_on_non_scalar_break(&mut self, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "LOOP003",
            "'then' fallback branch is only valid on scalar-break 'for' expressions",
            span,
        ));
    }

    pub fn report_yield_outside_comprehension(&mut self, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "LOOP004",
            "'yield' is only permitted within comprehension or generator expressions",
            span,
        ));
    }

    /// Analyzes a `ForExpr` node and selects either Path A (`ThreeTierDynamicSchedule`) or
    /// Path B (`EnumerableProtocol`), enforcing `LOOP001`–`LOOP004`.
    pub fn analyze_for_expr(
        &mut self,
        arena: &AstArena,
        for_id: NodeId,
        is_expr_position: bool,
        in_generator_fn: bool,
        infer: &mut TypeInferenceEngine,
    ) -> ForLoweringStrategy {
        let node = *arena.node(for_id);
        let span = arena.span(for_id);
        let body_id = NodeId(node.rhs);
        let iter_id = arena.for_iter_nodes.get(&for_id.0).copied().unwrap_or(NodeId::NONE);
        let then_id = arena.for_then_nodes.get(&for_id.0).copied().unwrap_or(NodeId::NONE);

        // Scan body for `yield`, `break`, `continue <range>`, and `continue <range>, ...`
        let mut stats = BodyControlStats::default();
        self.scan_loop_body(arena, body_id, infer, &mut stats);

        if !stats.yield_types.is_empty() && !stats.valued_break_types.is_empty() {
            self.report_mixed_break_and_yield(span);
        }
        if stats.has_void_break && !stats.valued_break_types.is_empty() {
            self.report_mixed_void_and_valued_break(span);
        }
        if !is_expr_position && !in_generator_fn && !stats.yield_types.is_empty() {
            self.report_yield_outside_comprehension(span);
        }
        if then_id.is_some() && stats.valued_break_types.is_empty() {
            self.report_then_on_non_scalar_break(arena.span(then_id));
        }

        let schedule = if stats.has_worklist_continue {
            LoopScheduleMode::WorklistStack
        } else if stats.has_mutable_bounds_continue {
            LoopScheduleMode::MutableBounds
        } else {
            LoopScheduleMode::FixedScalar
        };

        // Determine whether iterable is a range (`range<T>`, `span<T>`, `step<T>`, `NodeKind::RangeExpr`)
        let mut is_range_iterable = false;
        let mut item_ty = TypeId::I64;
        let mut enumerator_ty = TypeId::ANY;

        if iter_id.is_some() {
            let iter_node = *arena.node(iter_id);
            if iter_node.node_kind() == NodeKind::RangeExpr {
                is_range_iterable = true;
                item_ty = TypeId::I64;
            } else if iter_node.node_kind() == NodeKind::BinaryExpr {
                // Check if this is `a ..< b`, `a .. b`, or `a ..= b`
                is_range_iterable = true;
                item_ty = TypeId::I64;
            } else {
                let iter_ty = infer
                    .inferred_node_types
                    .get(&iter_id.0)
                    .copied()
                    .unwrap_or(TypeId::ANY);
                let pruned = infer.types.prune(iter_ty);
                match infer.types.get(pruned).clone() {
                    TypeKind::Primitive(ref name)
                        if name.starts_with("range")
                            || name.starts_with("span")
                            || name.starts_with("step") =>
                    {
                        is_range_iterable = true;
                        item_ty = TypeId::I64;
                    }
                    TypeKind::Adt { ref name, .. }
                        if name == "range" || name == "span" || name == "step" =>
                    {
                        is_range_iterable = true;
                        item_ty = TypeId::I64;
                    }
                    TypeKind::Collection { element, .. } => {
                        item_ty = element;
                        enumerator_ty = infer
                            .types
                            .type_defs_by_name
                            .get("SliceEnumerator")
                            .copied()
                            .unwrap_or_else(|| {
                                infer.types.alloc(TypeKind::Primitive("SliceEnumerator".to_string()))
                            });
                    }
                    _ => {}
                }
            }
        }

        let ret_mode = if !stats.yield_types.is_empty() {
            let elem_ty = stats.yield_types[0];
            LoopReturnMode::CollectionYield { elem_ty }
        } else if !stats.valued_break_types.is_empty() {
            let base_ty = stats.valued_break_types[0];
            let has_then = then_id.is_some();
            let value_ty = if has_then {
                base_ty
            } else {
                infer.types.option_of(base_ty)
            };
            LoopReturnMode::ScalarBreak {
                value_ty,
                has_then_fallback: has_then,
            }
        } else {
            LoopReturnMode::StatementVoid
        };

        if is_range_iterable
            || is_expr_position
            || schedule != LoopScheduleMode::FixedScalar
            || ret_mode != LoopReturnMode::StatementVoid
        {
            let can_preallocate = is_range_iterable && schedule == LoopScheduleMode::FixedScalar;
            ForLoweringStrategy::ThreeTierDynamicSchedule {
                schedule,
                ret_mode,
                can_preallocate_yield_buffer: can_preallocate,
            }
        } else {
            ForLoweringStrategy::EnumerableProtocol {
                enumerator_ty,
                item_ty,
            }
        }
    }

    fn scan_loop_body(
        &self,
        arena: &AstArena,
        node_id: NodeId,
        infer: &TypeInferenceEngine,
        stats: &mut BodyControlStats,
    ) {
        if node_id.is_none() || (node_id.0 as usize) >= arena.nodes.len() {
            return;
        }
        let node = *arena.node(node_id);
        match node.node_kind() {
            NodeKind::YieldStmt => {
                let val_id = NodeId(node.lhs);
                let ty = infer
                    .inferred_node_types
                    .get(&val_id.0)
                    .copied()
                    .unwrap_or(TypeId::I64);
                stats.yield_types.push(ty);
            }
            NodeKind::BreakStmt => {
                let val_id = NodeId(node.lhs);
                if val_id.is_some() {
                    let ty = infer
                        .inferred_node_types
                        .get(&val_id.0)
                        .copied()
                        .unwrap_or(TypeId::I64);
                    stats.valued_break_types.push(ty);
                } else {
                    stats.has_void_break = true;
                }
            }
            NodeKind::ContinueStmt => {
                if node.flags >= 2 {
                    stats.has_worklist_continue = true;
                } else if node.flags == 1 || NodeId(node.lhs).is_some() {
                    stats.has_mutable_bounds_continue = true;
                }
            }
            NodeKind::ForExpr | NodeKind::WhileStmt | NodeKind::FnDecl | NodeKind::LambdaExpr => {
                // Do not cross nested loop or function boundaries
            }
            NodeKind::BlockExpr | NodeKind::Program | NodeKind::TupleExpr | NodeKind::ListExpr => {
                for &c in arena.children_slice(node.lhs, node.rhs) {
                    self.scan_loop_body(arena, NodeId(c), infer, stats);
                }
            }
            NodeKind::IfExpr => {
                self.scan_loop_body(arena, NodeId(node.lhs), infer, stats);
                if node.flags == 0 {
                    self.scan_loop_body(arena, NodeId(node.rhs), infer, stats);
                } else {
                    for &c in arena.children_slice(node.rhs, 2) {
                        self.scan_loop_body(arena, NodeId(c), infer, stats);
                    }
                }
            }
            NodeKind::MatchExpr => {
                for &c in arena.children_slice(node.lhs, node.rhs) {
                    self.scan_loop_body(arena, NodeId(c), infer, stats);
                }
            }
            NodeKind::MatchArm => {
                self.scan_loop_body(arena, NodeId(node.rhs), infer, stats);
            }
            NodeKind::LetStmt | NodeKind::VarStmt => {
                self.scan_loop_body(arena, NodeId(node.rhs), infer, stats);
            }
            NodeKind::ExprStmt => {
                let inner = NodeId(node.lhs);
                if inner.is_some() {
                    let inner_node = *arena.node(inner);
                    if inner_node.node_kind() == NodeKind::IdentExpr {
                        let sym = arena.interner.resolve(SymbolId(inner_node.lhs));
                        if sym.contains("continue") && sym.contains("...") {
                            stats.has_worklist_continue = true;
                        }
                    }
                    self.scan_loop_body(arena, inner, infer, stats);
                }
            }
            _ => {}
        }
    }
}

#[derive(Default)]
struct BodyControlStats {
    yield_types: Vec<TypeId>,
    valued_break_types: Vec<TypeId>,
    has_void_break: bool,
    has_mutable_bounds_continue: bool,
    has_worklist_continue: bool,
}
