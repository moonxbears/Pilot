using System;
using System.Collections.Generic;
using Antlr4.Runtime.Tree;
using Pilot.Analysis.TypeSystem;

namespace Pilot.Analysis.Loops;

/// <summary>
/// Milestone 15.2: Schedule lowering tier inferred in a single scoped pass over a <c>for</c> loop body.
/// </summary>
public enum LoopScheduleMode
{
    /// <summary>Tier 1: Standard fixed-bound scalar register iteration (<c>0</c> allocations).</summary>
    FixedScalar,

    /// <summary>Tier 2: Destructive range pivot via <c>continue start..end;</c> (<c>0</c> allocations, mutable register bounds).</summary>
    MutableBounds,

    /// <summary>Tier 3: Resumable sub-loop prepend via <c>continue start..end, ...;</c> (LIFO <c>RangeWindow&lt;TIter&gt;</c> stack).</summary>
    WorklistStack
}

/// <summary>
/// Milestone 15.2: Return semantics inferred in a single scoped pass over a <c>for</c> loop.
/// </summary>
public enum LoopReturnMode
{
    /// <summary>Statement-level loop producing no value.</summary>
    Void,

    /// <summary>Scalar search/reduction loop exiting via <c>break &lt;expr&gt;;</c> (with optional <c>then &lt;expr&gt;</c> completion branch).</summary>
    ScalarBreak,

    /// <summary>Collection-building expression loop appending via <c>yield &lt;expr&gt;;</c> into a contiguous buffer/slice.</summary>
    CollectionYield
}

/// <summary>
/// Result of single-pass scoped inference on a single <c>for</c> statement/expression AST node.
/// </summary>
public sealed class PilotForLoopInfo
{
    public LoopScheduleMode ScheduleMode { get; init; }
    public LoopReturnMode ReturnMode { get; init; }
    public bool IsExpressionPosition { get; init; }
    public bool HasCompletionBranch { get; init; }
    public bool CanPreallocateYieldArray { get; init; }
    public bool HasValuedBreak { get; init; }
    public bool HasPlainBreak { get; init; }
    public bool HasYield { get; init; }
}

/// <summary>
/// Milestone 15.2: Single-pass scoped static analyzer and lowering classifier for Pilot <c>for</c> loops.
/// Walks a <c>for</c> loop body stopping at nested <c>for_stmt</c> boundaries so inner loop control flow
/// never pollutes outer loop inference, and emits diagnostics <c>LOOP001</c>–<c>LOOP004</c>.
/// </summary>
public sealed class PilotForLoopAnalyzer : PilotBaseVisitor<object?>
{
    private readonly List<PilotAnalysisDiagnostic> _diagnostics = new();
    private readonly Dictionary<PilotParser.For_stmtContext, PilotForLoopInfo> _loopInfos = new();

    public IReadOnlyList<PilotAnalysisDiagnostic> Diagnostics => _diagnostics;
    public IReadOnlyDictionary<PilotParser.For_stmtContext, PilotForLoopInfo> LoopInfos => _loopInfos;

    public void CheckProgram(PilotParser.ProgramContext program)
    {
        Visit(program);
    }

    public override object? VisitFor_stmt(PilotParser.For_stmtContext context)
    {
        var info = AnalyzeSingleLoop(context, _diagnostics);
        _loopInfos[context] = info;
        return base.VisitFor_stmt(context);
    }

    /// <summary>
    /// Performs single-pass scoped inference on a single <paramref name="context"/>, stopping at nested <c>for_stmt</c> boundaries.
    /// </summary>
    public static PilotForLoopInfo AnalyzeSingleLoop(
        PilotParser.For_stmtContext context,
        List<PilotAnalysisDiagnostic>? diagnostics = null,
        bool? isExpressionPositionOverride = null)
    {
        bool isExprPos = isExpressionPositionOverride ?? IsInExpressionPosition(context);
        bool hasCompletionBranch = context.THEN() != null || context.ELSE() != null;
        var activeLabels = CollectEnclosingLabels(context);

        var bodySuite = context.suite(0);
        var state = new ScopedBodyScanState(activeLabels);

        if (bodySuite != null)
        {
            ScanLoopBodyTopLevel(bodySuite, state);
        }

        // 1. Schedule Mode Inference
        LoopScheduleMode scheduleMode;
        if (state.HasWorklistContinue)
        {
            scheduleMode = LoopScheduleMode.WorklistStack;
        }
        else if (state.HasRangeContinue)
        {
            scheduleMode = LoopScheduleMode.MutableBounds;
        }
        else
        {
            scheduleMode = LoopScheduleMode.FixedScalar;
        }

        // 2. Diagnostics Verification (LOOP001 - LOOP004)
        int line = context.Start?.Line ?? 1;
        int col = context.Start?.Column ?? 0;

        // LOOP001: Cannot mix `break <expr>` and `yield <expr>` in the same `for` loop
        if (state.HasValuedBreak && state.HasYield)
        {
            diagnostics?.Add(new PilotAnalysisDiagnostic(
                "LOOP001",
                "Cannot mix valued 'break <expr>' and 'yield <expr>' in the same 'for' loop.",
                 state.FirstYieldToken?.Line ?? line,
                state.FirstYieldToken?.Column ?? col,
                isError: true));
        }

        // LOOP002: Cannot mix valueless `break;` and valued `break <expr>;` in the same `for` loop
        if (state.HasPlainBreak && state.HasValuedBreak)
        {
            diagnostics?.Add(new PilotAnalysisDiagnostic(
                "LOOP002",
                "Cannot mix valueless 'break;' and valued 'break <expr>;' in the same 'for' loop.",
                state.FirstPlainBreakToken?.Line ?? line,
                state.FirstPlainBreakToken?.Column ?? col,
                isError: true));
        }

        // LOOP003: `then` (or `else`) completion value branch is only valid with `ScalarBreak`
        if (hasCompletionBranch && (state.HasYield || (!state.HasValuedBreak && (context.THEN() != null || isExprPos))))
        {
            var branchTok = context.THEN()?.Symbol ?? context.ELSE()?.Symbol ?? context.Start;
            diagnostics?.Add(new PilotAnalysisDiagnostic(
                "LOOP003",
                "Loop completion 'then' branch is only valid on a ScalarBreak 'for' loop (containing 'break <expr>').",
                branchTok?.Line ?? line,
                branchTok?.Column ?? col,
                isError: true));
        }

        // LOOP004: `yield <expr>` in statement-position (`Void`) `for` loop outside a generator function
        if (!isExprPos && state.HasYield && !IsInsideCoroutineGeneratorFunction(context))
        {
            diagnostics?.Add(new PilotAnalysisDiagnostic(
                "LOOP004",
                "'yield <expr>' inside a statement-position 'for' loop is invalid outside of a generator function. Assign or return the 'for' expression to use CollectionYield.",
                state.FirstYieldToken?.Line ?? line,
                state.FirstYieldToken?.Column ?? col,
                isError: true));
        }

        // 3. Return Mode Inference
        LoopReturnMode returnMode;
        if (isExprPos && state.HasYield && !state.HasValuedBreak)
        {
            returnMode = LoopReturnMode.CollectionYield;
        }
        else if (state.HasValuedBreak)
        {
            returnMode = LoopReturnMode.ScalarBreak;
        }
        else
        {
            returnMode = LoopReturnMode.Void;
        }

        // 4. Exact Array Pre-allocation Eligibility
        bool canPreallocate =
            returnMode == LoopReturnMode.CollectionYield &&
            scheduleMode == LoopScheduleMode.FixedScalar &&
            state.TotalYieldCount == 1 &&
            state.TopLevelUnconditionalYieldCount == 1 &&
            !state.HasPlainBreak &&
            !state.HasValuedBreak &&
            !state.HasPlainContinue &&
            !state.HasRangeContinue &&
            !state.HasWorklistContinue;

        return new PilotForLoopInfo
        {
            ScheduleMode = scheduleMode,
            ReturnMode = returnMode,
            IsExpressionPosition = isExprPos,
            HasCompletionBranch = hasCompletionBranch,
            CanPreallocateYieldArray = canPreallocate,
            HasValuedBreak = state.HasValuedBreak,
            HasPlainBreak = state.HasPlainBreak,
            HasYield = state.HasYield
        };
    }

    public static bool IsInExpressionPosition(PilotParser.For_stmtContext context)
    {
        if (context.Parent is PilotParser.AtomContext)
        {
            return true;
        }

        // Also check if `for_stmt` is the trailing expression-statement of a block/lambda/match arm
        if (context.Parent is PilotParser.Compound_stmtContext compound &&
            compound.Parent is PilotParser.StmtContext stmt)
        {
            if (stmt.Parent is PilotParser.BlockContext block && block.Parent is PilotParser.AtomContext)
            {
                var stmts = block.stmt();
                if (stmts.Length > 0 && ReferenceEquals(stmts[^1], stmt))
                {
                    return true;
                }
            }
        }

        return false;
    }

    private static bool IsInsideCoroutineGeneratorFunction(PilotParser.For_stmtContext context)
    {
        IParseTree? cur = context.Parent;
        while (cur != null)
        {
            if (cur is PilotParser.Func_defContext funcDef)
            {
                // If the function declares an explicit non-generator scalar/void return type, it is NOT a generator
                string? retType = funcDef.type_spec()?.GetText()?.Trim();
                if (!string.IsNullOrEmpty(retType))
                {
                    if (retType.Equals("void", StringComparison.OrdinalIgnoreCase) ||
                        retType.Equals("int", StringComparison.OrdinalIgnoreCase) ||
                        retType.Equals("i64", StringComparison.OrdinalIgnoreCase) ||
                        retType.Equals("i32", StringComparison.OrdinalIgnoreCase) ||
                        retType.Equals("float", StringComparison.OrdinalIgnoreCase) ||
                        retType.Equals("f64", StringComparison.OrdinalIgnoreCase) ||
                        retType.Equals("bool", StringComparison.OrdinalIgnoreCase) ||
                        retType.Equals("str", StringComparison.OrdinalIgnoreCase) ||
                        retType.Equals("string", StringComparison.OrdinalIgnoreCase))
                    {
                        return false;
                    }
                }

                // If the function also contains a valued `return <expr>;` outside nested functions, it's a normal function, not a generator
                if (FunctionHasValuedReturn(funcDef.suite()))
                {
                    return false;
                }

                return true;
            }
            cur = cur.Parent;
        }
        return false;
    }

    private static bool FunctionHasValuedReturn(IParseTree? tree)
    {
        if (tree == null) return false;
        if (tree is PilotParser.Func_defContext or PilotParser.Lambda_exprContext) return false;
        if (tree is PilotParser.Flow_stmtContext flow && flow.RETURN() != null && flow.testlist() != null)
        {
            return true;
        }
        for (int i = 0; i < tree.ChildCount; i++)
        {
            if (FunctionHasValuedReturn(tree.GetChild(i)))
                return true;
        }
        return false;
    }

    private static HashSet<string> CollectEnclosingLabels(IParseTree node)
    {
        var labels = new HashSet<string>(StringComparer.Ordinal);
        IParseTree? cur = node.Parent;
        while (cur != null)
        {
            if (cur is PilotParser.Labeled_stmtContext labeled && labeled.identifier() != null)
            {
                labels.Add(labeled.identifier().GetText());
            }
            if (cur is PilotParser.Func_defContext or PilotParser.Lambda_exprContext)
            {
                break;
            }
            cur = cur.Parent;
        }
        return labels;
    }

    private static void ScanLoopBodyTopLevel(PilotParser.SuiteContext bodySuite, ScopedBodyScanState state)
    {
        if (bodySuite.simple_stmt() != null)
        {
            ScanSimpleStmt(bodySuite.simple_stmt(), state, isTopLevelUnconditional: true, insideInnerWhile: false);
            return;
        }

        var stmts = bodySuite.stmt();
        if (stmts == null) return;

        foreach (var stmt in stmts)
        {
            if (stmt.simple_stmt() != null)
            {
                ScanSimpleStmt(stmt.simple_stmt(), state, isTopLevelUnconditional: true, insideInnerWhile: false);
            }
            else if (stmt.compound_stmt() != null)
            {
                ScanTree(stmt.compound_stmt(), state, isTopLevelUnconditional: false, insideInnerWhile: false);
            }
        }
    }

    private static void ScanSimpleStmt(
        PilotParser.Simple_stmtContext simpleStmt,
        ScopedBodyScanState state,
        bool isTopLevelUnconditional,
        bool insideInnerWhile)
    {
        var smallStmts = simpleStmt.small_stmt();
        if (smallStmts == null) return;

        foreach (var small in smallStmts)
        {
            if (small.flow_stmt() != null)
            {
                InspectFlowStmt(small.flow_stmt(), state, isTopLevelUnconditional, insideInnerWhile);
            }
            else
            {
                ScanTree(small, state, isTopLevelUnconditional: false, insideInnerWhile);
            }
        }
    }

    private static void ScanTree(
        IParseTree? tree,
        ScopedBodyScanState state,
        bool isTopLevelUnconditional,
        bool insideInnerWhile)
    {
        if (tree == null) return;

        // Hard boundary: stop at nested `for_stmt`, `func_def`, or `lambda_expr`
        if (tree is PilotParser.For_stmtContext or PilotParser.Func_defContext or PilotParser.Lambda_exprContext)
        {
            return;
        }

        if (tree is PilotParser.While_stmtContext)
        {
            insideInnerWhile = true;
            isTopLevelUnconditional = false;
        }

        if (tree is PilotParser.Flow_stmtContext flow)
        {
            InspectFlowStmt(flow, state, isTopLevelUnconditional, insideInnerWhile);
            return;
        }

        for (int i = 0; i < tree.ChildCount; i++)
        {
            ScanTree(tree.GetChild(i), state, isTopLevelUnconditional: false, insideInnerWhile);
        }
    }

    private static void InspectFlowStmt(
        PilotParser.Flow_stmtContext flow,
        ScopedBodyScanState state,
        bool isTopLevelUnconditional,
        bool insideInnerWhile)
    {
        if (flow.YIELD() != null)
        {
            state.HasYield = true;
            state.FirstYieldToken ??= flow.YIELD().Symbol;
            state.TotalYieldCount++;
            if (isTopLevelUnconditional)
            {
                state.TopLevelUnconditionalYieldCount++;
            }
            // Also scan the yielded expression in case it contains nested expressions
            if (flow.testlist() != null)
            {
                ScanTree(flow.testlist(), state, isTopLevelUnconditional: false, insideInnerWhile);
            }
            return;
        }

        if (flow.BREAK() != null)
        {
            if (!insideInnerWhile)
            {
                var breakTests = flow.test();
                var breakId = flow.identifier();

                if ((breakTests == null || breakTests.Length == 0) && breakId == null)
                {
                    state.HasPlainBreak = true;
                    state.FirstPlainBreakToken ??= flow.BREAK().Symbol;
                }
                else
                {
                    string exprText = breakTests != null && breakTests.Length > 0
                        ? breakTests[0].GetText()
                        : breakId!.GetText();

                    if (state.ActiveLabels.Contains(exprText))
                    {
                        state.HasPlainBreak = true;
                        state.FirstPlainBreakToken ??= flow.BREAK().Symbol;
                    }
                    else
                    {
                        state.HasValuedBreak = true;
                        state.FirstValuedBreakToken ??= flow.BREAK().Symbol;
                    }
                }
            }
            return;
        }

        if (flow.CONTINUE() != null)
        {
            if (!insideInnerWhile)
            {
                if (flow.ELLIPSIS() != null)
                {
                    state.HasWorklistContinue = true;
                }
                else if (flow.range_expr() != null && flow.range_expr().bitor_expr().Length > 1)
                {
                    state.HasRangeContinue = true;
                }
                else
                {
                    state.HasPlainContinue = true;
                }
            }
        }
    }

    private sealed class ScopedBodyScanState
    {
        public HashSet<string> ActiveLabels { get; }
        public bool HasPlainContinue { get; set; }
        public bool HasRangeContinue { get; set; }
        public bool HasWorklistContinue { get; set; }
        public bool HasPlainBreak { get; set; }
        public bool HasValuedBreak { get; set; }
        public bool HasYield { get; set; }
        public int TotalYieldCount { get; set; }
        public int TopLevelUnconditionalYieldCount { get; set; }
        public Antlr4.Runtime.IToken? FirstYieldToken { get; set; }
        public Antlr4.Runtime.IToken? FirstPlainBreakToken { get; set; }
        public Antlr4.Runtime.IToken? FirstValuedBreakToken { get; set; }

        public ScopedBodyScanState(HashSet<string> activeLabels)
        {
            ActiveLabels = activeLabels;
        }
    }
}
