using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Execution;

/// <summary>
/// Core AST visitor for executing Pilot statements, evaluating expressions,
/// and dispatching language constructs according to semantic specifications.
/// </summary>
public sealed class PilotInterpreterVisitor : PilotBaseVisitor<PilotValue>
{
    public PilotExecutionContext Context { get; }
    private PilotClassDefinition? _currentClassDef;

    private static readonly ConcurrentDictionary<PilotParser.LiteralContext, PilotValue> LiteralCache =
        new(ReferenceEqualityComparer.Instance);

    private static readonly ConcurrentDictionary<IParseTree, bool> ContainsYieldCache =
        new(ReferenceEqualityComparer.Instance);

    private static readonly ConcurrentDictionary<string, FStringSegment[]> FStringCache =
        new(StringComparer.Ordinal);

    private static readonly ConcurrentDictionary<PilotParser.TestContext, TargetAssignmentInfo> TargetCache =
        new(ReferenceEqualityComparer.Instance);

    private static readonly List<PilotValue> EmptyPositional = new(0);
    private static readonly Dictionary<string, PilotValue> EmptyNamed = new(StringComparer.Ordinal);
    private static readonly HashSet<int> EmptyEachPos = new();
    private static readonly HashSet<string> EmptyEachNamed = new(StringComparer.Ordinal);

    private readonly struct FStringSegment
    {
        public readonly bool IsExpression;
        public readonly string Text;
        public readonly bool IsSimpleIdentifier;
        public readonly PilotParser.TestContext? ParsedExpr;

        public FStringSegment(bool isExpression, string text, bool isSimpleIdentifier, PilotParser.TestContext? parsedExpr)
        {
            IsExpression = isExpression;
            Text = text;
            IsSimpleIdentifier = isSimpleIdentifier;
            ParsedExpr = parsedExpr;
        }
    }

    private enum TargetAssignmentKind : byte
    {
        Fallback = 0,
        SimpleIdentifier = 1,
        MemberAccess = 2,
        IndexAccess = 3,
        ContractForward = 4
    }

    private sealed class TargetAssignmentInfo
    {
        public TargetAssignmentKind Kind;
        public string TargetText = string.Empty;
        public string? Identifier;
        public PilotParser.Atom_exprContext? AtomExpr;
        public int ReceiverTrailerCount;
        public PilotParser.TestContext? IndexExpr;
        public string? NormalizedContract;
    }

    public PilotInterpreterVisitor(PilotExecutionContext context)
    {
        Context = context ?? throw new ArgumentNullException(nameof(context));
    }

    #region Program & Statements

    public override PilotValue VisitProgram(PilotParser.ProgramContext context)
    {
        PilotValue lastValue = PilotValue.Undefined;
        Pilot.Debugging.PilotStackFrame? rootFrame = null;
        if (Context.Debugger != null)
        {
            rootFrame = Context.Debugger.OnFunctionEnter(
                "<global>",
                Array.Empty<PilotValue>(),
                Context.Environment.GlobalScope,
                Context);
        }

        try
        {
            foreach (var stmt in context.stmt())
            {
                lastValue = Visit(stmt);
                if (Context.PendingFlow != PilotFlowSignalKind.None)
                {
                    if (Context.PendingFlow == PilotFlowSignalKind.Return)
                    {
                        lastValue = Context.PendingReturnValue;
                    }
                    Context.PendingFlow = PilotFlowSignalKind.None;
                    break;
                }
            }
        }
        catch (PilotReturnSignal ret)
        {
            lastValue = ret.Value;
        }
        finally
        {
            Context.ExecuteDefers();
            if (Context.Debugger != null && rootFrame != null)
            {
                Context.Debugger.OnFunctionExit(rootFrame, lastValue, Context);
            }
        }

        return lastValue;
    }

    public override PilotValue VisitStmt(PilotParser.StmtContext context)
    {
        if (Context.Debugger != null)
        {
            Context.Debugger.BeforeExecuteStatement(context.Start.Line, context.Start.Column, Context);
        }

        if (context.compound_stmt() != null)
            return Visit(context.compound_stmt());
        if (context.simple_stmt() != null)
            return Visit(context.simple_stmt());
        return PilotValue.Undefined;
    }

    public override PilotValue VisitSimple_stmt(PilotParser.Simple_stmtContext context)
    {
        PilotValue lastVal = PilotValue.Undefined;
        var smalls = context.small_stmt();
        for (int i = 0; i < smalls.Length; i++)
        {
            if (i > 0 && Context.Debugger != null)
            {
                Context.Debugger.BeforeExecuteStatement(smalls[i].Start.Line, smalls[i].Start.Column, Context);
            }
            lastVal = Visit(smalls[i]);
            if (Context.PendingFlow != PilotFlowSignalKind.None)
                break;
        }
        return lastVal;
    }

    public override PilotValue VisitSuite(PilotParser.SuiteContext context)
    {
        if (context.simple_stmt() != null)
        {
            return Visit(context.simple_stmt());
        }

        PilotValue lastVal = PilotValue.Undefined;
        if (context.stmt() != null)
        {
            foreach (var stmt in context.stmt())
            {
                lastVal = Visit(stmt);
                if (Context.PendingFlow != PilotFlowSignalKind.None)
                    break;
            }
        }
        return lastVal;
    }

    #endregion

    #region Variable Declarations & Assignments

    public override PilotValue VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        PilotValue initialVal = context.test() != null ? Visit(context.test()) : PilotValue.Undefined;
        DeclareBinding(context.binding_pattern(), initialVal, isConst: true);
        return initialVal;
    }

    public override PilotValue VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        PilotValue initialVal = context.test() != null ? Visit(context.test()) : PilotValue.Undefined;
        DeclareBinding(context.binding_pattern(), initialVal, isConst: false);
        return initialVal;
    }

    public override PilotValue VisitConst_stmt(PilotParser.Const_stmtContext context)
    {
        PilotValue initialVal = context.test() != null ? Visit(context.test()) : PilotValue.Undefined;
        DeclareBinding(context.binding_pattern(), initialVal, isConst: true);
        return initialVal;
    }

    private void DeclareBinding(PilotParser.Binding_patternContext context, PilotValue val, bool isConst)
    {
        if (context.identifier() != null)
        {
            string name = context.identifier().GetText();
            if (!Context.Environment.CurrentScope.Assign(name, val))
            {
                Context.Environment.CurrentScope.Declare(name, val, isConst);
            }
            return;
        }

        if (context.record_pattern() != null)
        {
            DeclareRecordPattern(context.record_pattern(), val, isConst);
            return;
        }

        if (context.tuple_pattern() != null)
        {
            DeclareTuplePattern(context.tuple_pattern(), val, isConst);
            return;
        }

        if (context.list_pattern() != null)
        {
            DeclareListPattern(context.list_pattern(), val, isConst);
            return;
        }
    }

    private void DeclareRecordPattern(PilotParser.Record_patternContext context, PilotValue val, bool isConst)
    {
        if (val.Kind != PilotValueKind.Map && val.Kind != PilotValueKind.Object)
        {
            throw new Pilot.Runtime.ControlFlow.PilotPatternMatchingException(
                $"Pattern match failed: expected map or object record, but got {val.Kind}");
        }

        var fields = context.record_pattern_field();
        if (fields == null || fields.Length == 0) return;

        var map = val.Kind == PilotValueKind.Map ? val.AsMap() : null;
        var obj = val.Kind == PilotValueKind.Object ? val.AsObject() : null;

        var visitedKeys = new HashSet<string>(StringComparer.Ordinal);

        foreach (var field in fields)
        {
            if (field.DOUBLESTAR() != null && field.identifier() != null)
            {
                ProcessRecordRestPattern(field, map, obj, visitedKeys, isConst);
                continue;
            }

            ProcessRecordFieldPattern(field, val, map, obj, visitedKeys, isConst);
        }
    }

    private void ProcessRecordRestPattern(
        PilotParser.Record_pattern_fieldContext field,
        Dictionary<PilotValue, PilotValue>? map,
        PilotObject? obj,
        HashSet<string> visitedKeys,
        bool isConst)
    {
        string restName = field.identifier().GetText();
        var restMap = new Dictionary<PilotValue, PilotValue>();
        if (map != null)
        {
            foreach (var kvp in map)
            {
                if (!visitedKeys.Contains(kvp.Key.ToString()))
                {
                    restMap[kvp.Key] = kvp.Value;
                }
            }
        }
        else if (obj != null)
        {
            foreach (var kvp in obj.Fields)
            {
                if (!visitedKeys.Contains(kvp.Key))
                {
                    restMap[PilotValue.FromString(kvp.Key)] = kvp.Value;
                }
            }
        }
        Context.Environment.CurrentScope.Declare(restName, PilotValue.FromMap(restMap), isConst);
    }

    private void ProcessRecordFieldPattern(
        PilotParser.Record_pattern_fieldContext field,
        PilotValue val,
        Dictionary<PilotValue, PilotValue>? map,
        PilotObject? obj,
        HashSet<string> visitedKeys,
        bool isConst)
    {
        string key = field.identifier()?.GetText()
                     ?? (field.STRING() != null ? UnescapeString(field.STRING().GetText()) : "");
        if (string.IsNullOrEmpty(key)) return;

        visitedKeys.Add(key);
        PilotValue propVal = PilotValue.Undefined;
        bool found = false;

        if (map != null)
        {
            if (map.TryGetValue(PilotValue.FromString(key), out propVal)) found = true;
        }
        else if (obj != null)
        {
            if (obj.Fields.TryGetValue(key, out propVal)) found = true;
            else
            {
                var gm = obj.GetMember(key, Context);
                if (!gm.IsUndefined)
                {
                    propVal = gm;
                    found = true;
                }
            }
        }

        if (!found)
        {
            if (field.test() != null)
            {
                propVal = Visit(field.test());
                found = true;
            }
            else
            {
                throw new Pilot.Runtime.ControlFlow.PilotPatternMatchingException(
                    $"Pattern match failed: required property '{key}' not found on record {val}.");
            }
        }

        if (field.binding_pattern() != null)
        {
            DeclareBinding(field.binding_pattern(), propVal, isConst);
        }
        else if (field.identifier() != null)
        {
            string varName = field.identifier().GetText();
            if (!Context.Environment.CurrentScope.Assign(varName, propVal))
            {
                Context.Environment.CurrentScope.Declare(varName, propVal, isConst);
            }
        }
    }

    private void DeclareListPattern(PilotParser.List_patternContext context, PilotValue val, bool isConst)
    {
        if (val.Kind != PilotValueKind.List && val.Kind != PilotValueKind.Tuple)
        {
            throw new Pilot.Runtime.ControlFlow.PilotPatternMatchingException(
                $"Pattern match failed: expected list or sequence, but got {val.Kind}");
        }

        var items = val.Kind == PilotValueKind.List ? val.AsList() : val.AsTuple();
        var patternItems = context.binding_pattern_item();
        if (patternItems == null || patternItems.Length == 0)
        {
            if (items.Count > 0)
            {
                throw new Pilot.Runtime.ControlFlow.PilotPatternMatchingException(
                    $"Pattern match failed: expected empty list, but got {items.Count} items.");
            }
            return;
        }

        int restIndex = -1;
        for (int i = 0; i < patternItems.Length; i++)
        {
            if (patternItems[i].STAR() != null || patternItems[i].DOUBLESTAR() != null || patternItems[i].DOTDOT() != null)
            {
                restIndex = i;
                break;
            }
        }

        if (restIndex == -1)
        {
            if (items.Count != patternItems.Length)
            {
                throw new Pilot.Runtime.ControlFlow.PilotPatternMatchingException(
                    $"Pattern match failed: expected exactly {patternItems.Length} elements, but got {items.Count}.");
            }
            for (int i = 0; i < patternItems.Length; i++)
            {
                DeclareBinding(patternItems[i].binding_pattern(), items[i], isConst);
            }
        }
        else
        {
            int prefixCount = restIndex;
            int suffixCount = patternItems.Length - restIndex - 1;
            if (items.Count < prefixCount + suffixCount)
            {
                throw new Pilot.Runtime.ControlFlow.PilotPatternMatchingException(
                    $"Pattern match failed: expected at least {prefixCount + suffixCount} elements, but got {items.Count}.");
            }

            for (int i = 0; i < prefixCount; i++)
            {
                DeclareBinding(patternItems[i].binding_pattern(), items[i], isConst);
            }

            var restList = new List<PilotValue>();
            for (int i = prefixCount; i < items.Count - suffixCount; i++)
            {
                restList.Add(items[i]);
            }
            DeclareBinding(patternItems[restIndex].binding_pattern(), PilotValue.FromList(restList), isConst);

            for (int i = 0; i < suffixCount; i++)
            {
                int itemIdx = items.Count - suffixCount + i;
                int patIdx = restIndex + 1 + i;
                DeclareBinding(patternItems[patIdx].binding_pattern(), items[itemIdx], isConst);
            }
        }
    }

    private void DeclareTuplePattern(PilotParser.Tuple_patternContext context, PilotValue val, bool isConst)
    {
        if (val.Kind != PilotValueKind.Tuple && val.Kind != PilotValueKind.List)
        {
            throw new Pilot.Runtime.ControlFlow.PilotPatternMatchingException(
                $"Pattern match failed: expected tuple or sequence, but got {val.Kind}");
        }

        var items = val.Kind == PilotValueKind.Tuple ? val.AsTuple() : val.AsList();
        var patternItems = context.binding_pattern_item();
        if (patternItems == null || patternItems.Length == 0)
        {
            if (items.Count > 0)
            {
                throw new Pilot.Runtime.ControlFlow.PilotPatternMatchingException(
                    $"Pattern match failed: expected empty tuple, but got {items.Count} items.");
            }
            return;
        }

        int restIndex = -1;
        for (int i = 0; i < patternItems.Length; i++)
        {
            if (patternItems[i].STAR() != null || patternItems[i].DOUBLESTAR() != null || patternItems[i].DOTDOT() != null)
            {
                restIndex = i;
                break;
            }
        }

        if (restIndex == -1)
        {
            if (items.Count != patternItems.Length)
            {
                throw new Pilot.Runtime.ControlFlow.PilotPatternMatchingException(
                    $"Pattern match failed: expected exactly {patternItems.Length} elements, but got {items.Count}.");
            }
            for (int i = 0; i < patternItems.Length; i++)
            {
                DeclareBinding(patternItems[i].binding_pattern(), items[i], isConst);
            }
        }
        else
        {
            int prefixCount = restIndex;
            int suffixCount = patternItems.Length - restIndex - 1;
            if (items.Count < prefixCount + suffixCount)
            {
                throw new Pilot.Runtime.ControlFlow.PilotPatternMatchingException(
                    $"Pattern match failed: expected at least {prefixCount + suffixCount} elements, but got {items.Count}.");
            }

            for (int i = 0; i < prefixCount; i++)
            {
                DeclareBinding(patternItems[i].binding_pattern(), items[i], isConst);
            }

            var restList = new List<PilotValue>();
            for (int i = prefixCount; i < items.Count - suffixCount; i++)
            {
                restList.Add(items[i]);
            }
            DeclareBinding(patternItems[restIndex].binding_pattern(), PilotValue.FromList(restList), isConst);

            for (int i = 0; i < suffixCount; i++)
            {
                int itemIdx = items.Count - suffixCount + i;
                int patIdx = restIndex + 1 + i;
                DeclareBinding(patternItems[patIdx].binding_pattern(), items[itemIdx], isConst);
            }
        }
    }

    public override PilotValue VisitPattern_assign_stmt(PilotParser.Pattern_assign_stmtContext context)
    {
        var rhsVal = Visit(context.test());
        if (context.record_pattern() != null)
        {
            DeclareRecordPattern(context.record_pattern(), rhsVal, isConst: false);
        }
        else if (context.list_pattern() != null)
        {
            DeclareListPattern(context.list_pattern(), rhsVal, isConst: false);
        }
        else if (context.tuple_pattern() != null)
        {
            DeclareTuplePattern(context.tuple_pattern(), rhsVal, isConst: false);
        }
        return rhsVal;
    }

    public override PilotValue VisitExpr_stmt(PilotParser.Expr_stmtContext context)
    {
        var testList = context.testlist();
        if (testList == null) return PilotValue.Undefined;

        var tests = testList.test();
        if (context.EQUAL() != null && context.EQUAL().Length > 0 && context.test() != null && context.test().Length > 0)
        {
            // Simple assignment: a = val
            var rhsNode = context.test(0);
            var rhsVal = Visit(rhsNode);
            AssignTarget(tests[0], rhsVal, rhsNode);
            return rhsVal;
        }

        if (context.augassign() != null && context.augassign().Length > 0 && context.test() != null && context.test().Length > 0)
        {
            // Augmented assignment: a += val
            var lhsVal = Visit(tests[0]);
            var rhsVal = Visit(context.test(0));
            var op = context.augassign(0);
            PilotValue computed = lhsVal;

            if (op.PLUSEQUAL() != null) computed = lhsVal + rhsVal;
            else if (op.MINEQUAL() != null) computed = lhsVal - rhsVal;
            else if (op.STAREQUAL() != null) computed = lhsVal * rhsVal;
            else if (op.SLASHEQUAL() != null) computed = lhsVal / rhsVal;
            else if (op.PERCENTEQUAL() != null) computed = lhsVal % rhsVal;

            AssignTarget(tests[0], computed);
            return computed;
        }

        // Standalone expression statement
        PilotValue result = PilotValue.Undefined;
        foreach (var t in tests)
        {
            result = Visit(t);
        }
        return result;
    }

    private static PilotParser.Atom_exprContext? TryUnwrapAtomExpr(PilotParser.TestContext test)
    {
        if (test.lambda_expr() != null || test.IF() != null) return null;
        var pipes = test.pipe_expr();
        if (pipes == null || pipes.Length != 1) return null;
        var queries = pipes[0].set_query_expr();
        if (queries == null || queries.Length != 1) return null;
        var coalesces = queries[0].coalesce_expr();
        if (coalesces == null || coalesces.Length != 1) return null;
        var ors = coalesces[0].or_test();
        if (ors == null || ors.Length != 1) return null;
        var ands = ors[0].and_test();
        if (ands == null || ands.Length != 1) return null;
        var notTests = ands[0].not_test();
        if (notTests == null || notTests.Length != 1 || notTests[0].NOT() != null) return null;
        var comp = notTests[0].comparison();
        if (comp == null || comp.comp_op()?.Length > 0 || comp.IS()?.Length > 0) return null;
        var ranges = comp.range_expr();
        if (ranges == null || ranges.Length != 1) return null;
        var bitors = ranges[0].bitor_expr();
        if (bitors == null || bitors.Length != 1) return null;
        var bitxors = bitors[0].bitxor_expr();
        if (bitxors == null || bitxors.Length != 1) return null;
        var bitands = bitxors[0].bitand_expr();
        if (bitands == null || bitands.Length != 1) return null;
        var shifts = bitands[0].shift_expr();
        if (shifts == null || shifts.Length != 1) return null;
        var ariths = shifts[0].arith_expr();
        if (ariths == null || ariths.Length != 1) return null;
        var terms = ariths[0].term();
        if (terms == null || terms.Length != 1) return null;
        var factors = terms[0].factor();
        if (factors == null || factors.Length != 1) return null;
        var factor = factors[0];
        if (factor.PLUS() != null || factor.MINUS() != null || factor.unary_op() != null) return null;
        var power = factor.power();
        if (power == null || power.DOUBLESTAR() != null) return null;
        return power.atom_expr();
    }

    private static TargetAssignmentInfo AnalyzeTarget(PilotParser.TestContext targetContext)
    {
        var atomExpr = TryUnwrapAtomExpr(targetContext);
        if (atomExpr != null)
        {
            var trailers = atomExpr.trailer();
            if (trailers == null || trailers.Length == 0)
            {
                var id = atomExpr.atom()?.identifier();
                if (id != null)
                {
                    string idText = id.GetText();
                    return new TargetAssignmentInfo
                    {
                        Kind = TargetAssignmentKind.SimpleIdentifier,
                        Identifier = idText,
                        TargetText = idText
                    };
                }
            }
            else
            {
                var lastTrailer = trailers[^1];
                if (lastTrailer.DOT() != null && lastTrailer.identifier() != null)
                {
                    string memberName = lastTrailer.identifier().GetText();
                    if (!memberName.Equals("Contracts", StringComparison.OrdinalIgnoreCase))
                    {
                        return new TargetAssignmentInfo
                        {
                            Kind = TargetAssignmentKind.MemberAccess,
                            AtomExpr = atomExpr,
                            ReceiverTrailerCount = trailers.Length - 1,
                            Identifier = memberName,
                            TargetText = targetContext.GetText()
                        };
                    }
                }
                else if (lastTrailer.LSQB() != null)
                {
                    var subTest = lastTrailer.subscriptlist()?.subscript(0)?.test(0);
                    if (subTest != null)
                    {
                        return new TargetAssignmentInfo
                        {
                            Kind = TargetAssignmentKind.IndexAccess,
                            AtomExpr = atomExpr,
                            ReceiverTrailerCount = trailers.Length - 1,
                            IndexExpr = subTest,
                            TargetText = targetContext.GetText()
                        };
                    }
                }
            }
        }

        string targetText = targetContext.GetText();
        if (targetText.EndsWith(".Contracts", StringComparison.OrdinalIgnoreCase))
        {
            int cIdx = targetText.LastIndexOf(".Contracts", StringComparison.OrdinalIgnoreCase);
            string contractPart = targetText[..cIdx];
            string normContract = Pilot.Runtime.Contracts.PilotContractRegistry.NormalizeContractName(contractPart);
            return new TargetAssignmentInfo
            {
                Kind = TargetAssignmentKind.ContractForward,
                TargetText = targetText,
                NormalizedContract = normContract
            };
        }

        return new TargetAssignmentInfo
        {
            Kind = TargetAssignmentKind.Fallback,
            TargetText = targetText
        };
    }

    private PilotValue EvaluateAtomExprPrefix(PilotParser.Atom_exprContext context, int trailerCount)
    {
        PilotValue current = Visit(context.atom());
        var trailers = context.trailer();
        for (int i = 0; i < trailerCount; i++)
        {
            var trailer = trailers[i];
            if ((trailer.STAR() != null && trailer.LPAR() != null) || trailer.LPAR() != null)
            {
                current = DispatchCall(current, trailer, trailers, i, context);
            }
            else if (trailer.DOT() != null || trailer.SAFE_NAV() != null)
            {
                if (trailer.SAFE_NAV() != null && (current.IsNull || current.IsUndefined))
                {
                    current = PilotValue.Undefined;
                    continue;
                }
                if (trailer.DOT() != null && (current.IsNull || current.IsUndefined))
                {
                    string mName = trailer.identifier()?.GetText() ?? "unknown";
                    throw new PilotNullReferenceException($"Cannot access member '{mName}' on null or undefined value.", context.Start.Line, context.Start.Column);
                }
                string memberName = trailer.identifier().GetText();
                current = current.GetMember(memberName, Context);
            }
            else if (trailer.LSQB() != null)
            {
                var subCtx = trailer.subscriptlist()?.subscript(0);
                if (subCtx != null)
                {
                    var idxVal = Visit(subCtx.test(0));
                    current = current[idxVal];
                }
            }
            else if (trailer.cascade_trailer() != null)
            {
                current = EvaluateCascade(current, trailer.cascade_trailer());
            }
        }
        return current;
    }

    private void AssignTarget(PilotParser.TestContext targetContext, PilotValue value, PilotParser.TestContext? rhsNode = null)
    {
        var info = TargetCache.GetOrAdd(targetContext, AnalyzeTarget);

        if (info.Kind == TargetAssignmentKind.SimpleIdentifier)
        {
            string name = info.Identifier!;
            if (!Context.Environment.CurrentScope.Assign(name, value))
            {
                Context.Environment.CurrentScope.Declare(name, value);
            }
            return;
        }

        if (info.Kind == TargetAssignmentKind.MemberAccess)
        {
            try
            {
                var receiverVal = EvaluateAtomExprPrefix(info.AtomExpr!, info.ReceiverTrailerCount);
                if (receiverVal.Kind == PilotValueKind.Object)
                {
                    receiverVal.AsObject().SetMember(info.Identifier!, value);
                    return;
                }
            }
            catch
            {
                // Fallback to scope
            }
        }
        else if (info.Kind == TargetAssignmentKind.IndexAccess)
        {
            try
            {
                var receiverVal = EvaluateAtomExprPrefix(info.AtomExpr!, info.ReceiverTrailerCount);
                var indexVal = Visit(info.IndexExpr!);
                receiverVal[indexVal] = value;
                return;
            }
            catch
            {
                // Fallback to scope
            }
        }
        else if (info.Kind == TargetAssignmentKind.ContractForward)
        {
            string normContract = info.NormalizedContract!;
            string? rhsText = rhsNode?.GetText();
            string targetMember = !string.IsNullOrEmpty(rhsText)
                ? rhsText
                : (value.Kind == PilotValueKind.String ? value.AsString() : info.TargetText);

            if (targetMember.StartsWith("self.", StringComparison.OrdinalIgnoreCase))
            {
                targetMember = targetMember[5..];
            }

            if (_currentClassDef != null)
            {
                _currentClassDef.DelegateContract(normContract, targetMember, Context);
                return;
            }

            if (Context.Environment.TryResolve("self", out var selfVal) && selfVal.Kind == PilotValueKind.Object)
            {
                var obj = selfVal.AsObject();
                obj.ContractSubstitutes[normContract] = value;
                obj.ClassDef?.DelegateContract(normContract, targetMember, Context);
                return;
            }
        }

        string targetText = info.TargetText;

        int dotIdx = targetText.LastIndexOf('.');
        if (dotIdx > 0 && !targetText.EndsWith("."))
        {
            string receiverExpr = targetText[..dotIdx];
            string member = targetText[(dotIdx + 1)..];
            try
            {
                var receiverVal = Context.Engine.EvaluateExpression(receiverExpr, Context.Environment.CurrentScope);
                if (receiverVal.Kind == PilotValueKind.Object)
                {
                    receiverVal.AsObject().SetMember(member, value);
                    return;
                }
            }
            catch
            {
                // Fallback to scope
            }
        }

        int bracketIdx = targetText.IndexOf('[');
        if (bracketIdx > 0 && targetText.EndsWith("]"))
        {
            string receiverExpr = targetText[..bracketIdx];
            string indexExpr = targetText[(bracketIdx + 1)..^1];
            try
            {
                var receiverVal = Context.Engine.EvaluateExpression(receiverExpr, Context.Environment.CurrentScope);
                var indexVal = Context.Engine.EvaluateExpression(indexExpr, Context.Environment.CurrentScope);
                receiverVal[indexVal] = value;
                return;
            }
            catch
            {
                // Fallback to scope
            }
        }

        // Fallback assign to scope by text
        if (!Context.Environment.CurrentScope.Assign(targetText, value))
        {
            Context.Environment.CurrentScope.Declare(targetText, value);
        }
    }

    #endregion

    #region Functions & Classes

    public override PilotValue VisitFunc_def(PilotParser.Func_defContext context)
    {
        string funcName = context.identifier().GetText();
        var paramList = new List<PilotParameter>();

        if (context.parameters()?.parameter() != null)
        {
            foreach (var p in context.parameters().parameter())
            {
                string pName = p.identifier()?.GetText() ?? p.splice_expr()?.GetText() ?? "arg";
                string? pType = p.type_annotation()?.type_spec()?.GetText();
                PilotValue? defaultVal = p.test() != null ? Visit(p.test()) : default(PilotValue?);
                bool isParams = p.PARAMS() != null || p.STAR() != null;
                bool isKwargs = p.DOUBLESTAR() != null;
                bool isContextual = p.CONTEXT() != null;
                bool isEach = p.EACH() != null || p.REPEAT() != null;
                paramList.Add(new PilotParameter(pName, pType, defaultVal, isParams, isKwargs, isContextual, isEach));
            }
        }

        bool isGen = ContainsYield(context.suite());
        bool isVectorized = false;
        if (context.decorators() != null)
        {
            foreach (var dec in context.decorators().decorator())
            {
                string decName = dec.dotted_name()?.GetText() ?? "";
                if (decName is "vectorized" or "repeat" or "auto_repeat") isVectorized = true;
            }
        }

        var function = new PilotFunction(
            funcName,
            paramList,
            context.suite(),
            Context.Environment.CurrentScope,
            isAsync: context.ASYNC() != null,
            isGenerator: isGen);
        function.IsVectorized = isVectorized;
        Pilot.Runtime.Burst.PilotBurstEngine.TryAttachBurstKernel(function, context);

        if (context.contract_clause() != null)
        {
            foreach (var cc in context.contract_clause())
            {
                if (cc.REQUIRES() != null)
                {
                    function.RequiresContracts.Add(cc.test());
                }
                else if (cc.ENSURES() != null)
                {
                    function.EnsuresContracts.Add(cc.test());
                }
            }
        }

        if (Context.Environment.CurrentScope.ContainsLocal(funcName))
        {
            var existing = Context.Environment.CurrentScope.Resolve(funcName);
            if (existing.Kind == PilotValueKind.MethodGroup)
            {
                var mg = existing.AsMethodGroup();
                mg.AddOverload(function);
                return existing;
            }
            else if (existing.Kind == PilotValueKind.Function)
            {
                var oldFunc = existing.AsFunction();
                if (oldFunc.ClassDef == null && oldFunc.GetSignatureKey() == function.GetSignatureKey())
                {
                    oldFunc.ApplyPatchFrom(function);
                    return existing;
                }
                var mg = new PilotMethodGroup(funcName, new[] { oldFunc, function });
                var mgVal = PilotValue.FromMethodGroup(mg);
                Context.Environment.CurrentScope.ForceSet(funcName, mgVal);
                return mgVal;
            }
        }

        Context.Environment.CurrentScope.Declare(funcName, PilotValue.FromFunction(function));
        return PilotValue.FromFunction(function);
    }

    private static bool ContainsYield(Antlr4.Runtime.Tree.IParseTree? tree)
    {
        if (tree == null) return false;
        return ContainsYieldCache.GetOrAdd(tree, static t => ContainsYieldCore(t));
    }

    private static bool ContainsYieldCore(Antlr4.Runtime.Tree.IParseTree? tree)
    {
        if (tree == null) return false;
        if (tree is PilotParser.Func_defContext) return false;
        if (tree is PilotParser.Flow_stmtContext flow && flow.YIELD() != null) return true;
        for (int i = 0; i < tree.ChildCount; i++)
        {
            if (ContainsYieldCore(tree.GetChild(i))) return true;
        }
        return false;
    }

    public override PilotValue VisitClass_def(PilotParser.Class_defContext context)
    {
        string className = context.identifier().GetText();

        // 1. Algebraic Class Composition: class Admin = BaseUser | AdminPrivileges
        if (context.EQUAL() != null && context.test() != null)
        {
            var rhsVal = Visit(context.test());
            if (rhsVal.Kind == PilotValueKind.Function && rhsVal.AsFunction().ClassDef != null)
            {
                var baseDef = rhsVal.AsFunction().ClassDef!;
                var namedDef = baseDef.CloneWithName(className);
                var ctor = PilotFunction.CreateNative(className, (ctx, args) => PilotValue.FromObject(namedDef.Instantiate(ctx, args)));
                ctor.ClassDef = namedDef;
                Context.Environment.CurrentScope.Declare(className, PilotValue.FromFunction(ctor));
                return PilotValue.FromFunction(ctor);
            }
            throw new PilotTypeException($"Right-hand side of class composition must evaluate to a class, got {rhsVal.Kind}", context.Start.Line, context.Start.Column);
        }

        string? baseName = context.class_bases()?.type_spec()?.FirstOrDefault()?.GetText();

        PilotClassDefinition classDef;
        if (Context.Environment.TryResolve(className, out var existingVal) &&
            existingVal.Kind == PilotValueKind.Function &&
            existingVal.AsFunction().ClassDef != null)
        {
            // Partial class: merge into existing definition
            classDef = existingVal.AsFunction().ClassDef!;
            if (baseName != null && string.IsNullOrEmpty(classDef.BaseClassName))
            {
                classDef.BaseClassName = baseName;
            }
        }
        else
        {
            classDef = new PilotClassDefinition(className, baseName);
        }

        // Enter temporary class definition scope to extract methods
        var prevClass = _currentClassDef;
        _currentClassDef = classDef;
        Context.Environment.PushScope($"ClassDef:{className}");
        try
        {
            if (context.suite()?.stmt() != null)
            {
                foreach (var stmt in context.suite().stmt())
                {
                    if (stmt.compound_stmt()?.state_def() != null)
                    {
                        VisitState_def(stmt.compound_stmt().state_def());
                    }
                    else if (stmt.compound_stmt()?.macro_def() != null)
                    {
                        var mVal = VisitMacro_def(stmt.compound_stmt().macro_def());
                        if (mVal.Kind == PilotValueKind.Function)
                        {
                            classDef.GenerativeMacros.Add(mVal.AsFunction());
                        }
                    }
                    else if (stmt.compound_stmt()?.func_def() != null)
                    {
                        var funcDef = stmt.compound_stmt().func_def();
                        var funcVal = VisitFunc_def(funcDef);
                        if (funcVal.Kind == PilotValueKind.MethodGroup)
                        {
                            var mg = funcVal.AsMethodGroup();
                            classDef.MethodGroups[mg.Name] = mg;
                        }
                        else
                        {
                            var func = funcVal.AsFunction();
                            classDef.Methods[func.Name] = func;
                        }
                    }
                    else if (stmt.simple_stmt()?.small_stmt() != null)
                    {
                        foreach (var s in stmt.simple_stmt().small_stmt())
                        {
                            if (s.steal_stmt() != null) VisitSteal_stmt(s.steal_stmt());
                            else if (s.delegate_stmt() != null) VisitDelegate_stmt(s.delegate_stmt());
                            else if (s.contract_forward_stmt() != null) VisitContract_forward_stmt(s.contract_forward_stmt());
                            else if (s.annassign_stmt() != null)
                            {
                                var ann = s.annassign_stmt();
                                string fieldName = ann.test(0)?.GetText() ?? "";
                                string fieldType = ann.type_spec()?.GetText() ?? "any";
                                PilotValue defVal = ann.test().Length > 1 ? Visit(ann.test(1)) : PilotValue.Null;
                                if (!string.IsNullOrEmpty(fieldName))
                                {
                                    classDef.StaticFields[fieldName] = defVal;
                                }
                            }
                            else if (s.let_stmt() != null)
                            {
                                var val = VisitLet_stmt(s.let_stmt());
                                string varName = s.let_stmt().binding_pattern()?.identifier()?.GetText() ?? "";
                                if (!string.IsNullOrEmpty(varName)) classDef.StaticFields[varName] = val;
                            }
                            else if (s.var_stmt() != null)
                            {
                                var val = VisitVar_stmt(s.var_stmt());
                                string varName = s.var_stmt().binding_pattern()?.identifier()?.GetText() ?? "";
                                if (!string.IsNullOrEmpty(varName)) classDef.StaticFields[varName] = val;
                            }
                            else Visit(s);
                        }
                    }
                    else
                    {
                        Visit(stmt);
                    }
                }
            }
        }
        finally
        {
            _currentClassDef = prevClass;
            Context.Environment.PopScope();
        }

        // Inherit generative macros from base classes
        if (!string.IsNullOrEmpty(classDef.BaseClassName) &&
            Context.Environment.TryResolve(classDef.BaseClassName, out var baseVal) &&
            baseVal.Kind == PilotValueKind.Function && baseVal.AsFunction().ClassDef != null)
        {
            foreach (var gm in baseVal.AsFunction().ClassDef!.GenerativeMacros)
            {
                if (!classDef.GenerativeMacros.Contains(gm))
                {
                    classDef.GenerativeMacros.Add(gm);
                }
            }
        }

        // Execute compile-time generative macros on class definition
        if (classDef.GenerativeMacros.Count > 0)
        {
            var classNode = new Pilot.Runtime.Generative.PilotClassNode(classDef);
            var classNodeVal = PilotValue.FromObject(classNode.ToPilotObject(Context));
            foreach (var macro in classDef.GenerativeMacros)
            {
                macro.Invoke(Context, new[] { classNodeVal });
            }
        }

        // Register class constructor function
        var ctorFunction = PilotFunction.CreateNative(
            className,
            (ctx, args) => PilotValue.FromObject(classDef.Instantiate(ctx, args)));
        ctorFunction.ClassDef = classDef;

        if (Context.Environment.CurrentScope.ContainsLocal(className))
        {
            Context.Environment.CurrentScope.Assign(className, PilotValue.FromFunction(ctorFunction));
        }
        else
        {
            Context.Environment.CurrentScope.Declare(className, PilotValue.FromFunction(ctorFunction));
        }

        return PilotValue.FromFunction(ctorFunction);
    }

    public override PilotValue VisitState_def(PilotParser.State_defContext context)
    {
        string stateName = context.identifier().GetText();
        if (_currentClassDef == null)
        {
            throw new PilotTypeException($"Typestate '{stateName}' can only be defined inside a class.", context.Start.Line, context.Start.Column);
        }

        var stateDef = new PilotClassDefinition($"{_currentClassDef.Name}[{stateName}]");
        var prevClass = _currentClassDef;
        _currentClassDef = stateDef;
        Context.Environment.PushScope($"StateDef:{stateName}");
        try
        {
            if (context.suite()?.stmt() != null)
            {
                foreach (var stmt in context.suite().stmt())
                {
                    var funcDef = stmt.compound_stmt()?.func_def();
                    if (funcDef != null)
                    {
                        var funcVal = VisitFunc_def(funcDef);
                        if (funcVal.Kind == PilotValueKind.MethodGroup)
                        {
                            stateDef.MethodGroups[funcVal.AsMethodGroup().Name] = funcVal.AsMethodGroup();
                        }
                        else
                        {
                            stateDef.Methods[funcVal.AsFunction().Name] = funcVal.AsFunction();
                        }
                    }
                    else
                    {
                        Visit(stmt);
                    }
                }
            }
        }
        finally
        {
            _currentClassDef = prevClass;
            Context.Environment.PopScope();
        }

        _currentClassDef.States[stateName] = stateDef;
        if (_currentClassDef.InitialState == null)
        {
            _currentClassDef.InitialState = stateName;
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitMacro_def(PilotParser.Macro_defContext context)
    {
        string macroName = context.identifier().GetText();
        var paramList = new List<PilotParameter>();

        if (context.parameters()?.parameter() != null)
        {
            foreach (var p in context.parameters().parameter())
            {
                string pName = p.identifier()?.GetText() ?? p.splice_expr()?.GetText() ?? "arg";
                string? pType = p.type_annotation()?.type_spec()?.GetText();
                PilotValue? defaultVal = p.test() != null ? Visit(p.test()) : default(PilotValue?);
                paramList.Add(new PilotParameter(pName, pType, defaultVal));
            }
        }

        var macroFunction = new PilotFunction(
            macroName,
            paramList,
            context.suite(),
            Context.Environment.CurrentScope,
            isAsync: false,
            isGenerator: false);

        Context.Environment.CurrentScope.Declare(macroName, PilotValue.FromFunction(macroFunction));
        return PilotValue.FromFunction(macroFunction);
    }

    public override PilotValue VisitExtension_def(PilotParser.Extension_defContext context)
    {
        string extName = context.identifier()?.GetText() ?? "AnonymousExtension";
        string targetType = context.type_spec().GetText();

        var ext = new Pilot.Runtime.Extensions.PilotLexicalExtension(extName, targetType);
        Context.Environment.PushScope($"ExtensionDef:{extName}");
        try
        {
            if (context.suite()?.stmt() != null)
            {
                foreach (var stmt in context.suite().stmt())
                {
                    var funcDef = stmt.compound_stmt()?.func_def();
                    if (funcDef != null)
                    {
                        var funcVal = VisitFunc_def(funcDef);
                        if (funcVal.Kind == PilotValueKind.MethodGroup)
                        {
                            ext.MethodGroups[funcVal.AsMethodGroup().Name] = funcVal.AsMethodGroup();
                        }
                        else
                        {
                            ext.Methods[funcVal.AsFunction().Name] = funcVal.AsFunction();
                        }
                    }
                }
            }
        }
        finally
        {
            Context.Environment.PopScope();
        }

        Context.RegisteredExtensions[extName] = ext;
        Context.Environment.CurrentScope.Declare(extName, PilotValue.FromForeign(ext));
        return PilotValue.FromForeign(ext);
    }

    public override PilotValue VisitAdapter_def(PilotParser.Adapter_defContext context)
    {
        string fromType = "";
        string toType = "";
        if (context.type_spec().Length >= 2)
        {
            fromType = context.type_spec(0).GetText();
            toType = context.type_spec(1).GetText();
        }
        else if (context.identifier() != null)
        {
            fromType = context.identifier().GetText();
            toType = "any";
        }

        var adapterFn = new PilotFunction(
            $"adapter_{fromType}_to_{toType}",
            new[] { new PilotParameter("self", fromType) },
            context.suite(),
            Context.Environment.CurrentScope);

        Pilot.Runtime.Adapters.PilotTypeAdapterRegistry.Default.Register(fromType, toType, adapterFn);
        if (context.identifier() != null)
        {
            Context.Environment.CurrentScope.Declare(context.identifier().GetText(), PilotValue.FromFunction(adapterFn));
        }
        return PilotValue.FromFunction(adapterFn);
    }

    public override PilotValue VisitSteal_stmt(PilotParser.Steal_stmtContext context)
    {
        string sourceClassName = context.dotted_name().GetText();
        PilotClassDefinition? sourceClass = null;

        if (Context.Environment.TryResolve(sourceClassName, out var val))
        {
            if (val.Kind == PilotValueKind.Function && val.AsFunction().ClassDef != null)
            {
                sourceClass = val.AsFunction().ClassDef;
            }
            else if (val.Kind == PilotValueKind.Object && val.AsObject().ClassDef != null)
            {
                sourceClass = val.AsObject().ClassDef;
            }
        }

        if (sourceClass == null)
        {
            throw new PilotRuntimeException($"Cannot steal methods: class or type '{sourceClassName}' not found.");
        }

        List<string>? specific = null;
        if (context.identifier() != null && context.identifier().Length > 0)
        {
            specific = context.identifier().Select(id => id.GetText()).ToList();
        }

        if (_currentClassDef != null)
        {
            _currentClassDef.StealFrom(sourceClass, specific);
        }
        else if (Context.Environment.TryResolve("self", out var selfVal) && selfVal.Kind == PilotValueKind.Object)
        {
            selfVal.AsObject().ClassDef?.StealFrom(sourceClass, specific);
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitDelegate_stmt(PilotParser.Delegate_stmtContext context)
    {
        string typeName = context.type_spec().GetText();
        string targetMember = context.test().GetText();
        var targetVal = Visit(context.test());

        if (_currentClassDef != null)
        {
            _currentClassDef.DelegateContract(typeName, targetMember, Context);
        }
        else if (Context.Environment.TryResolve("self", out var selfVal) && selfVal.Kind == PilotValueKind.Object)
        {
            var obj = selfVal.AsObject();
            string norm = Pilot.Runtime.Contracts.PilotContractRegistry.NormalizeContractName(typeName);
            obj.ContractSubstitutes[norm] = targetVal;
            obj.ClassDef?.DelegateContract(norm, targetMember, Context);
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitContract_forward_stmt(PilotParser.Contract_forward_stmtContext context)
    {
        string typeName = context.type_spec().GetText();
        string targetMember = context.test().GetText();
        var targetVal = Visit(context.test());

        if (_currentClassDef != null)
        {
            _currentClassDef.DelegateContract(typeName, targetMember, Context);
        }
        else if (Context.Environment.TryResolve("self", out var selfVal) && selfVal.Kind == PilotValueKind.Object)
        {
            var obj = selfVal.AsObject();
            string norm = Pilot.Runtime.Contracts.PilotContractRegistry.NormalizeContractName(typeName);
            obj.ContractSubstitutes[norm] = targetVal;
            obj.ClassDef?.DelegateContract(norm, targetMember, Context);
        }

        return PilotValue.Undefined;
    }

    #endregion

    #region Control Flow (If, While, For, Match, Guard, Defer)

    public override PilotValue VisitIf_stmt(PilotParser.If_stmtContext context)
    {
        var tests = context.test();
        var suites = context.suite();

        // Main if condition
        if (Visit(tests[0]).IsTrue)
        {
            return Visit(suites[0]);
        }

        // Elif / Else If branches
        int elifCount = tests.Length - 1;
        for (int i = 0; i < elifCount; i++)
        {
            if (Visit(tests[i + 1]).IsTrue)
            {
                return Visit(suites[i + 1]);
            }
        }

        // Else branch
        if (suites.Length > tests.Length)
        {
            return Visit(suites[^1]);
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitWhile_stmt(PilotParser.While_stmtContext context)
    {
        PilotValue lastVal = PilotValue.Undefined;
        var condNode = context.test(0);
        var bodyNode = context.suite(0);
        while (Visit(condNode).IsTrue)
        {
            try
            {
                lastVal = Visit(bodyNode);
            }
            catch (PilotBreakSignal)
            {
                break;
            }
            catch (PilotContinueSignal)
            {
                continue;
            }

            if (Context.PendingFlow != PilotFlowSignalKind.None)
            {
                if (Context.PendingFlow == PilotFlowSignalKind.Continue && string.IsNullOrEmpty(Context.PendingFlowLabel))
                {
                    Context.PendingFlow = PilotFlowSignalKind.None;
                    continue;
                }
                if (Context.PendingFlow == PilotFlowSignalKind.Break && string.IsNullOrEmpty(Context.PendingFlowLabel))
                {
                    Context.PendingFlow = PilotFlowSignalKind.None;
                    break;
                }
                break;
            }
        }
        return lastVal;
    }

    public override PilotValue VisitLabeled_stmt(PilotParser.Labeled_stmtContext context)
    {
        string label = context.identifier().GetText();
        try
        {
            PilotValue res = PilotValue.Undefined;
            if (context.for_stmt() != null) res = Visit(context.for_stmt());
            else if (context.while_stmt() != null) res = Visit(context.while_stmt());
            else if (context.block() != null) res = Visit(context.block());

            if ((Context.PendingFlow == PilotFlowSignalKind.Break || Context.PendingFlow == PilotFlowSignalKind.Continue) &&
                (string.IsNullOrEmpty(Context.PendingFlowLabel) || Context.PendingFlowLabel == label))
            {
                Context.PendingFlow = PilotFlowSignalKind.None;
                Context.PendingFlowLabel = null;
                return PilotValue.Undefined;
            }

            return res;
        }
        catch (PilotBreakSignal b) when (b.Label == label || string.IsNullOrEmpty(b.Label))
        {
            return PilotValue.Undefined;
        }
    }

    public override PilotValue VisitBlock(PilotParser.BlockContext context)
    {
        PilotValue lastVal = PilotValue.Undefined;
        if (context.stmt() != null)
        {
            foreach (var stmt in context.stmt())
            {
                lastVal = Visit(stmt);
                if (Context.PendingFlow != PilotFlowSignalKind.None)
                    break;
            }
        }
        return lastVal;
    }

    public override PilotValue VisitFor_stmt(PilotParser.For_stmtContext context)
    {
        var iterable = Visit(context.test(0));
        var forTarget = context.for_target();
        var bodySuite = context.suite(0);
        PilotValue lastVal = PilotValue.Undefined;

        string? simpleTargetName = TryGetSimpleForTargetName(forTarget);

        // Fast-path 1: Range iteration without boxing or LINQ enumerator allocation
        if (iterable.Kind == PilotValueKind.Range)
        {
            var range = iterable.AsRange();
            long step = range.Step;
            if (step == 0) return lastVal;

            long current = range.Start;
            long end = range.End;
            bool inclusive = range.Inclusive;

            PilotBinding? binding = simpleTargetName != null
                ? Context.Environment.CurrentScope.GetOrDeclareBinding(simpleTargetName, PilotValue.Undefined)
                : null;

            while (step > 0 ? (inclusive ? current <= end : current < end) : (inclusive ? current >= end : current > end))
            {
                var itemVal = PilotValue.FromInt(current);
                if (binding != null)
                {
                    binding.Value = itemVal;
                }
                else
                {
                    AssignForTarget(forTarget, itemVal);
                }

                try
                {
                    lastVal = Visit(bodySuite);
                }
                catch (PilotBreakSignal)
                {
                    break;
                }
                catch (PilotContinueSignal)
                {
                    current += step;
                    continue;
                }

                if (Context.PendingFlow != PilotFlowSignalKind.None)
                {
                    if (Context.PendingFlow == PilotFlowSignalKind.Continue && string.IsNullOrEmpty(Context.PendingFlowLabel))
                    {
                        Context.PendingFlow = PilotFlowSignalKind.None;
                        current += step;
                        continue;
                    }
                    if (Context.PendingFlow == PilotFlowSignalKind.Break && string.IsNullOrEmpty(Context.PendingFlowLabel))
                    {
                        Context.PendingFlow = PilotFlowSignalKind.None;
                        break;
                    }
                    break;
                }

                current += step;
            }

            return lastVal;
        }

        // Fast-path 2: List iteration by index
        if (iterable.Kind == PilotValueKind.List)
        {
            var list = iterable.AsList();
            PilotBinding? binding = simpleTargetName != null
                ? Context.Environment.CurrentScope.GetOrDeclareBinding(simpleTargetName, PilotValue.Undefined)
                : null;

            for (int i = 0; i < list.Count; i++)
            {
                var itemVal = list[i];
                if (binding != null)
                {
                    binding.Value = itemVal;
                }
                else
                {
                    AssignForTarget(forTarget, itemVal);
                }

                try
                {
                    lastVal = Visit(bodySuite);
                }
                catch (PilotBreakSignal)
                {
                    break;
                }
                catch (PilotContinueSignal)
                {
                    continue;
                }

                if (Context.PendingFlow != PilotFlowSignalKind.None)
                {
                    if (Context.PendingFlow == PilotFlowSignalKind.Continue && string.IsNullOrEmpty(Context.PendingFlowLabel))
                    {
                        Context.PendingFlow = PilotFlowSignalKind.None;
                        continue;
                    }
                    if (Context.PendingFlow == PilotFlowSignalKind.Break && string.IsNullOrEmpty(Context.PendingFlowLabel))
                    {
                        Context.PendingFlow = PilotFlowSignalKind.None;
                        break;
                    }
                    break;
                }
            }

            return lastVal;
        }

        IEnumerable<PilotValue> collection;
        if (iterable.Kind == PilotValueKind.Tuple)
        {
            collection = iterable.AsTuple();
        }
        else if (iterable.Kind == PilotValueKind.Set)
        {
            collection = iterable.AsSet();
        }
        else if (iterable.Kind == PilotValueKind.Generator)
        {
            collection = iterable.AsGenerator();
        }
        else if (iterable.Kind == PilotValueKind.String)
        {
            collection = iterable.AsString().Select(c => PilotValue.FromChar(c));
        }
        else if (iterable.Kind == PilotValueKind.Map)
        {
            collection = iterable.AsMap().Keys;
        }
        else
        {
            collection = Array.Empty<PilotValue>();
        }

        PilotBinding? genBinding = simpleTargetName != null
            ? Context.Environment.CurrentScope.GetOrDeclareBinding(simpleTargetName, PilotValue.Undefined)
            : null;

        foreach (var item in collection)
        {
            if (genBinding != null)
            {
                genBinding.Value = item;
            }
            else
            {
                AssignForTarget(forTarget, item);
            }

            try
            {
                lastVal = Visit(bodySuite);
            }
            catch (PilotBreakSignal)
            {
                break;
            }
            catch (PilotContinueSignal)
            {
                continue;
            }

            if (Context.PendingFlow != PilotFlowSignalKind.None)
            {
                if (Context.PendingFlow == PilotFlowSignalKind.Continue && string.IsNullOrEmpty(Context.PendingFlowLabel))
                {
                    Context.PendingFlow = PilotFlowSignalKind.None;
                    continue;
                }
                if (Context.PendingFlow == PilotFlowSignalKind.Break && string.IsNullOrEmpty(Context.PendingFlowLabel))
                {
                    Context.PendingFlow = PilotFlowSignalKind.None;
                    break;
                }
                break;
            }
        }

        return lastVal;
    }

    private static string? TryGetSimpleForTargetName(PilotParser.For_targetContext target)
    {
        var bps = target.binding_pattern();
        if (bps != null && bps.Length == 1 && bps[0].identifier() != null)
        {
            return bps[0].identifier().GetText();
        }
        if (target.exprlist() != null)
        {
            string name = target.exprlist().GetText();
            if (IsSimpleIdentifier(name))
            {
                return name;
            }
        }
        return null;
    }

    public override PilotValue VisitWith_stmt(PilotParser.With_stmtContext context)
    {
        var pushedExtensions = new List<Pilot.Runtime.Extensions.PilotLexicalExtension>();
        var pushedContextFrames = new List<Dictionary<string, PilotValue>>();
        int pushedAutoRepeat = 0;

        foreach (var item in context.with_item())
        {
            if (item.CONTEXT() != null && item.identifier() != null)
            {
                string ctxName = item.identifier().GetText();
                var ctxVal = Visit(item.test());
                var frame = new Dictionary<string, PilotValue>(StringComparer.Ordinal)
                {
                    [ctxName] = ctxVal
                };
                Context.ContextualBindings.Push(frame);
                pushedContextFrames.Add(frame);
                Context.Environment.CurrentScope.Declare(ctxName, ctxVal);
            }
            else if (item.EXTENSION() != null && item.identifier() != null)
            {
                string extName = item.identifier().GetText();
                if (Context.RegisteredExtensions.TryGetValue(extName, out var ext))
                {
                    Context.ActiveExtensions.Push(ext);
                    pushedExtensions.Add(ext);
                }
                else if (Context.Environment.TryResolve(extName, out var extVal) &&
                         extVal.AsForeign() is Pilot.Runtime.Extensions.PilotLexicalExtension foreignExt)
                {
                    Context.ActiveExtensions.Push(foreignExt);
                    pushedExtensions.Add(foreignExt);
                }
            }
            else if (item.test() != null)
            {
                string rawText = item.test().GetText().Trim();
                if (rawText is "auto_repeat" or "repeat" or "vectorize" or "autorepeat")
                {
                    Context.AutoRepeatDepth++;
                    pushedAutoRepeat++;
                }
                else
                {
                    var val = Visit(item.test());
                    if (val.AsForeign() is Pilot.Runtime.Extensions.PilotLexicalExtension ext)
                    {
                        Context.ActiveExtensions.Push(ext);
                        pushedExtensions.Add(ext);
                    }
                    if (item.identifier() != null)
                    {
                        Context.Environment.CurrentScope.Declare(item.identifier().GetText(), val);
                    }
                }
            }
        }

        try
        {
            return Visit(context.suite());
        }
        finally
        {
            for (int i = 0; i < pushedExtensions.Count; i++)
            {
                Context.ActiveExtensions.Pop();
            }
            for (int i = 0; i < pushedContextFrames.Count; i++)
            {
                Context.ContextualBindings.Pop();
            }
            for (int i = 0; i < pushedAutoRepeat; i++)
            {
                Context.AutoRepeatDepth--;
            }
        }
    }

    private void AssignForTarget(PilotParser.For_targetContext target, PilotValue value)
    {
        if (target.binding_pattern()?.Length > 0)
        {
            var bp = target.binding_pattern(0);
            if (bp.identifier() != null)
            {
                string name = bp.identifier().GetText();
                if (!Context.Environment.CurrentScope.Assign(name, value))
                {
                    Context.Environment.CurrentScope.Declare(name, value, isConst: false);
                }
                return;
            }
            DeclareBinding(bp, value, isConst: false);
        }
        else if (target.exprlist() != null)
        {
            string name = target.exprlist().GetText();
            if (!Context.Environment.CurrentScope.Assign(name, value))
            {
                Context.Environment.CurrentScope.Declare(name, value);
            }
        }
    }

    public override PilotValue VisitMatch_stmt(PilotParser.Match_stmtContext context)
    {
        var subject = Visit(context.test());
        var matchSuite = context.match_suite();
        if (matchSuite?.match_arm() == null) return PilotValue.Undefined;

        foreach (var arm in matchSuite.match_arm())
        {
            Context.Environment.PushScope("MatchArm");
            try
            {
                if (PilotPatternMatcher.Match(arm.pattern(), subject, Context.Environment.CurrentScope, this))
                {
                    // Check optional when/if guard: case pat when x > 10 => suite
                    if (arm.test() != null && !Visit(arm.test()).IsTrue)
                    {
                        continue;
                    }

                    return Visit(arm.suite());
                }
            }
            finally
            {
                Context.Environment.PopScope();
            }
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitGuard_stmt(PilotParser.Guard_stmtContext context)
    {
        PilotValue condVal;
        if (context.LET() != null && context.pattern() != null)
        {
            var rhs = Visit(context.test());
            bool matched = PilotPatternMatcher.Match(context.pattern(), rhs, Context.Environment.CurrentScope, this);
            condVal = PilotValue.FromBool(matched);
        }
        else
        {
            condVal = Visit(context.test());
        }

        if (!condVal.IsTrue)
        {
            // Guard precondition failed: execute else exit suite
            return Visit(context.suite());
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitDefer_stmt(PilotParser.Defer_stmtContext context)
    {
        var capturedScope = Context.Environment.CurrentScope;
        Context.PushDefer(() =>
        {
            var prevFlow = Context.PendingFlow;
            var prevRet = Context.PendingReturnValue;
            var prevLabel = Context.PendingFlowLabel;
            Context.PendingFlow = PilotFlowSignalKind.None;
            try
            {
                if (context.suite() != null) Visit(context.suite());
                else if (context.small_stmt() != null) Visit(context.small_stmt());
                else if (context.test() != null) Visit(context.test());
            }
            finally
            {
                if (Context.PendingFlow == PilotFlowSignalKind.None)
                {
                    Context.PendingFlow = prevFlow;
                    Context.PendingReturnValue = prevRet;
                    Context.PendingFlowLabel = prevLabel;
                }
            }
        });

        return PilotValue.Undefined;
    }

    private static readonly HashSet<string> KnownExceptionNames = new(StringComparer.OrdinalIgnoreCase)
    {
        "Exception",
        "Error",
        "RuntimeError",
        "TypeError",
        "DivideByZeroError",
        "IndexError",
        "KeyError",
        "NullReferenceError",
        "AssertionError",
        "RecursionError",
        "ContractError",
        "TimeoutError",
        "UserError",
        "PilotTypeException",
        "PilotDivideByZeroException",
        "PilotIndexOutOfRangeException",
        "PilotKeyNotFoundException",
        "PilotNullReferenceException",
        "PilotAssertionException",
        "PilotRecursionException",
        "PilotContractException",
        "PilotTimeoutException",
        "PilotUserException",
        "PilotRuntimeException"
    };

    private static bool IsKnownExceptionTypeName(string name)
    {
        if (KnownExceptionNames.Contains(name))
            return true;
        if (name.EndsWith("Error", StringComparison.OrdinalIgnoreCase) ||
            name.EndsWith("Exception", StringComparison.OrdinalIgnoreCase))
            return true;
        return false;
    }

    public override PilotValue VisitTry_stmt(PilotParser.Try_stmtContext context)
    {
        bool executedWithoutException = false;
        PilotValue lastVal = PilotValue.Undefined;

        try
        {
            try
            {
                lastVal = Visit(context.suite(0));
                executedWithoutException = Context.PendingFlow == PilotFlowSignalKind.None;
            }
            catch (PilotControlFlowSignal)
            {
                // Flow control signals (return, break, continue, yield) bypass except clauses
                throw;
            }
            catch (Exception ex)
            {
                bool handled = false;
                var exceptClauses = context.except_clause();
                if (exceptClauses != null && exceptClauses.Length > 0)
                {
                    foreach (var clause in exceptClauses)
                    {
                        string? typeSpecText = clause.type_spec()?.GetText();
                        string? varName = clause.identifier()?.GetText();
                        string? expectedType = typeSpecText;

                        if (string.IsNullOrEmpty(varName) && !string.IsNullOrEmpty(typeSpecText))
                        {
                            if (!IsKnownExceptionTypeName(typeSpecText))
                            {
                                varName = typeSpecText;
                                expectedType = null;
                            }
                        }

                        if (IsExceptionMatching(ex, expectedType))
                        {
                            handled = true;
                            Context.Environment.PushScope("ExceptHandler");
                            try
                            {
                                if (!string.IsNullOrEmpty(varName))
                                {
                                    PilotValue errPayload = ex is PilotUserException pue
                                        ? pue.UserPayload
                                        : (ex is PilotRuntimeException pre
                                            ? PilotValue.FromString(pre.Message)
                                            : PilotValue.FromString(ex.Message));
                                    Context.Environment.CurrentScope.Declare(varName, errPayload);
                                }
                                lastVal = Visit(clause.suite());
                            }
                            finally
                            {
                                Context.Environment.PopScope();
                            }
                            break;
                        }
                    }
                }

                if (!handled)
                {
                    throw;
                }
            }

            // Execute else suite if try block succeeded without any exception or flow jump
            if (executedWithoutException && context.ELSE() != null)
            {
                var suites = context.suite();
                if (suites.Length > 1)
                {
                    lastVal = Visit(suites[1]);
                }
            }
        }
        finally
        {
            // Execute finally suite if present (always runs regardless of try/except/else outcome)
            if (context.FINALLY() != null)
            {
                var prevFlow = Context.PendingFlow;
                var prevRet = Context.PendingReturnValue;
                var prevLabel = Context.PendingFlowLabel;
                Context.PendingFlow = PilotFlowSignalKind.None;
                try
                {
                    var suites = context.suite();
                    int finallyIdx = context.ELSE() != null ? 2 : 1;
                    if (finallyIdx < suites.Length)
                    {
                        Visit(suites[finallyIdx]);
                    }
                    else if (suites.Length > 1)
                    {
                        Visit(suites[suites.Length - 1]);
                    }
                }
                finally
                {
                    if (Context.PendingFlow == PilotFlowSignalKind.None)
                    {
                        Context.PendingFlow = prevFlow;
                        Context.PendingReturnValue = prevRet;
                        Context.PendingFlowLabel = prevLabel;
                    }
                }
            }
        }

        return lastVal;
    }

    private static bool IsExceptionMatching(Exception ex, string? expectedType)
    {
        if (string.IsNullOrWhiteSpace(expectedType) ||
            expectedType.Equals("Exception", StringComparison.OrdinalIgnoreCase) ||
            expectedType.Equals("Error", StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        if (ex is PilotRuntimeException pre)
        {
            if (pre.TypeName.Equals(expectedType, StringComparison.OrdinalIgnoreCase)) return true;
            if (pre.GetType().Name.Equals(expectedType, StringComparison.OrdinalIgnoreCase)) return true;
            if (expectedType.Equals("RuntimeError", StringComparison.OrdinalIgnoreCase)) return true;
        }

        if (ex.GetType().Name.Equals(expectedType, StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        return false;
    }

    public override PilotValue VisitAssert_stmt(PilotParser.Assert_stmtContext context)
    {
        var tests = context.test();
        var cond = Visit(tests[0]);
        if (!cond.IsTrue)
        {
            string message = tests.Length > 1
                ? Visit(tests[1]).ToString()
                : $"Assertion failed: '{tests[0].GetText()}' evaluated to false.";
            throw new PilotAssertionException(message, context.Start.Line, context.Start.Column);
        }
        return PilotValue.True;
    }

    public override PilotValue VisitFlow_stmt(PilotParser.Flow_stmtContext context)
    {
        if (context.RETURN() != null)
        {
            PilotValue retVal = context.testlist() != null ? VisitTestlist(context.testlist()) : PilotValue.Undefined;
            if (Context.YieldHandler != null)
            {
                throw new PilotReturnSignal(retVal);
            }
            Context.PendingFlow = PilotFlowSignalKind.Return;
            Context.PendingReturnValue = retVal;
            return retVal;
        }

        if (context.BREAK() != null)
        {
            Context.PendingFlow = PilotFlowSignalKind.Break;
            Context.PendingFlowLabel = context.identifier()?.GetText();
            return PilotValue.Undefined;
        }

        if (context.CONTINUE() != null)
        {
            Context.PendingFlow = PilotFlowSignalKind.Continue;
            Context.PendingFlowLabel = context.identifier()?.GetText();
            return PilotValue.Undefined;
        }

        if (context.YIELD() != null)
        {
            PilotValue yieldVal = context.testlist() != null ? VisitTestlist(context.testlist()) : PilotValue.Undefined;
            if (Context.YieldHandler != null)
            {
                return Context.YieldHandler(yieldVal);
            }
            throw new PilotYieldSignal(yieldVal);
        }

        if (context.RAISE() != null)
        {
            var tests = context.test();
            if (tests != null && tests.Length > 0)
            {
                var errVal = Visit(tests[0]);
                if (errVal.Kind == PilotValueKind.Object)
                {
                    var msg = errVal.AsObject().GetMember("message");
                    string messageStr = msg != PilotValue.Undefined ? msg.ToString() : errVal.ToString();
                    throw new PilotUserException(messageStr, errVal.AsObject().TypeName, errVal, context.Start.Line, context.Start.Column);
                }
                throw new PilotUserException(errVal.ToString(), "UserError", errVal, context.Start.Line, context.Start.Column);
            }
            throw new PilotUserException("Error raised.", "UserError", null, context.Start.Line, context.Start.Column);
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitTestlist(PilotParser.TestlistContext context)
    {
        var tests = context.test();
        if (tests.Length == 1) return Visit(tests[0]);

        var list = new List<PilotValue>(tests.Length);
        foreach (var t in tests)
        {
            list.Add(Visit(t));
        }
        return PilotValue.FromTuple(list);
    }

    #endregion

    #region Expressions & Operators

    public override PilotValue VisitTest(PilotParser.TestContext context)
    {
        if (context.lambda_expr() != null)
        {
            return Visit(context.lambda_expr());
        }

        var pipeExprs = context.pipe_expr();
        if (pipeExprs?.Length > 0)
        {
            var primaryVal = Visit(pipeExprs[0]);

            // Ternary expression: expr IF condition ELSE alt
            if (context.IF() != null && pipeExprs.Length > 1 && context.test() != null)
            {
                var conditionVal = Visit(pipeExprs[1]);
                return conditionVal.IsTrue ? primaryVal : Visit(context.test());
            }

            return primaryVal;
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitPipe_expr(PilotParser.Pipe_exprContext context)
    {
        var queries = context.set_query_expr();
        var val = Visit(queries[0]);

        for (int i = 1; i < queries.Length; i++)
        {
            var next = Visit(queries[i]);
            if (next.Kind == PilotValueKind.Function)
            {
                val = next.AsFunction().Invoke(Context, new[] { val });
            }
        }

        return val;
    }

    public override PilotValue VisitSet_query_expr(PilotParser.Set_query_exprContext context)
    {
        if (context.coalesce_expr()?.Length > 0)
        {
            return Visit(context.coalesce_expr(0));
        }
        return PilotValue.Undefined;
    }

    public override PilotValue VisitCoalesce_expr(PilotParser.Coalesce_exprContext context)
    {
        var tests = context.or_test();
        var val = Visit(tests[0]);

        for (int i = 1; i < tests.Length; i++)
        {
            if (val.IsNull || val.IsUndefined)
            {
                val = Visit(tests[i]);
            }
            else
            {
                break;
            }
        }

        return val;
    }

    public override PilotValue VisitOr_test(PilotParser.Or_testContext context)
    {
        var tests = context.and_test();
        var val = Visit(tests[0]);

        for (int i = 1; i < tests.Length; i++)
        {
            if (val.IsTrue) return val;
            val = Visit(tests[i]);
        }

        return val;
    }

    public override PilotValue VisitAnd_test(PilotParser.And_testContext context)
    {
        var tests = context.not_test();
        var val = Visit(tests[0]);

        for (int i = 1; i < tests.Length; i++)
        {
            if (!val.IsTrue) return val;
            val = Visit(tests[i]);
        }

        return val;
    }

    public override PilotValue VisitNot_test(PilotParser.Not_testContext context)
    {
        if (context.NOT() != null)
        {
            return !Visit(context.not_test());
        }
        return Visit(context.comparison());
    }

    public override PilotValue VisitComparison(PilotParser.ComparisonContext context)
    {
        var ranges = context.range_expr();
        var left = Visit(ranges[0]);

        if (ranges.Length == 1 && context.pattern()?.Length == 0)
        {
            return left;
        }

        // IS / IS NOT pattern
        if (context.IS()?.Length > 0 && context.pattern()?.Length > 0)
        {
            bool matched = PilotPatternMatcher.Match(context.pattern(0), left, Context.Environment.CurrentScope, this);
            return context.NOT()?.Length > 0 ? PilotValue.FromBool(!matched) : PilotValue.FromBool(matched);
        }

        // Comparison operators: ==, !=, <, <=, >, >=, in, not in
        if (context.comp_op()?.Length > 0 && ranges.Length > 1)
        {
            var op = context.comp_op(0);
            var right = Visit(ranges[1]);

            if (op.EQEQUAL() != null) return PilotValue.FromBool(left == right);
            if (op.NOTEQUAL() != null) return PilotValue.FromBool(left != right);
            if (op.LESS() != null) return PilotValue.FromBool(left < right);
            if (op.LESSEQUAL() != null) return PilotValue.FromBool(left <= right);
            if (op.GREATER() != null) return PilotValue.FromBool(left > right);
            if (op.GREATEREQUAL() != null) return PilotValue.FromBool(left >= right);

            if (op.IN() != null)
            {
                if (right.Kind == PilotValueKind.List) return PilotValue.FromBool(right.AsList().Contains(left));
                if (right.Kind == PilotValueKind.Set) return PilotValue.FromBool(right.AsSet().Contains(left));
                if (right.Kind == PilotValueKind.Map) return PilotValue.FromBool(right.AsMap().ContainsKey(left));
                if (right.Kind == PilotValueKind.String) return PilotValue.FromBool(right.AsString().Contains(left.ToString()));
                return PilotValue.False;
            }
        }

        return left;
    }

    public override PilotValue VisitRange_expr(PilotParser.Range_exprContext context)
    {
        var bitor = context.bitor_expr();
        var startVal = Visit(bitor[0]);

        if (bitor.Length > 1)
        {
            var endVal = Visit(bitor[1]);
            bool inclusive = context.DOTDOT_EQUAL() != null;
            return PilotValue.FromRange(new PilotRange(startVal.AsInt(), endVal.AsInt(), 1, inclusive));
        }

        return startVal;
    }

    public override PilotValue VisitBitor_expr(PilotParser.Bitor_exprContext context)
    {
        var xors = context.bitxor_expr();
        var val = Visit(xors[0]);

        for (int i = 1; i < xors.Length; i++)
        {
            var rhs = Visit(xors[i]);
            val = val | rhs;
        }

        return val;
    }

    public override PilotValue VisitBitxor_expr(PilotParser.Bitxor_exprContext context)
    {
        var ands = context.bitand_expr();
        var val = Visit(ands[0]);

        for (int i = 1; i < ands.Length; i++)
        {
            var rhs = Visit(ands[i]);
            val = PilotValue.FromInt(val.AsInt() ^ rhs.AsInt());
        }

        return val;
    }

    public override PilotValue VisitBitand_expr(PilotParser.Bitand_exprContext context)
    {
        var shifts = context.shift_expr();
        var val = Visit(shifts[0]);

        for (int i = 1; i < shifts.Length; i++)
        {
            var rhs = Visit(shifts[i]);
            val = val & rhs;
        }

        return val;
    }

    public override PilotValue VisitShift_expr(PilotParser.Shift_exprContext context)
    {
        var ariths = context.arith_expr();
        var val = Visit(ariths[0]);

        for (int i = 1; i < ariths.Length; i++)
        {
            var rhs = Visit(ariths[i]);
            var opNode = context.GetChild(2 * i - 1) as ITerminalNode;
            int opType = opNode?.Symbol.Type ?? 0;
            if (opType == PilotParser.LEFTSHIFT) val = PilotValue.FromInt(val.AsInt() << (int)rhs.AsInt());
            else if (opType == PilotParser.RIGHTSHIFT) val = PilotValue.FromInt(val.AsInt() >> (int)rhs.AsInt());
        }

        return val;
    }

    public override PilotValue VisitArith_expr(PilotParser.Arith_exprContext context)
    {
        var terms = context.term();
        var val = Visit(terms[0]);

        for (int i = 1; i < terms.Length; i++)
        {
            var rhs = Visit(terms[i]);
            var opNode = context.GetChild(2 * i - 1) as ITerminalNode;
            int opType = opNode?.Symbol.Type ?? 0;
            if (opType == PilotParser.PLUS) val = val + rhs;
            else if (opType == PilotParser.MINUS) val = val - rhs;
        }

        return val;
    }

    public override PilotValue VisitTerm(PilotParser.TermContext context)
    {
        var factors = context.factor();
        var val = Visit(factors[0]);

        for (int i = 1; i < factors.Length; i++)
        {
            var opNode = context.GetChild(2 * i - 1) as ITerminalNode;
            int opType = opNode?.Symbol.Type ?? 0;
            if (opType == PilotParser.ARROW)
            {
                // Structural Projection: source -> {field1, field2}
                val = EvaluateProjection(val, factors[i]);
                continue;
            }

            var rhs = Visit(factors[i]);
            if (opType == PilotParser.STAR) val = val * rhs;
            else if (opType == PilotParser.SLASH)
            {
                if ((rhs.Kind == PilotValueKind.Int || rhs.Kind == PilotValueKind.UInt) && rhs.AsInt() == 0)
                {
                    throw new PilotDivideByZeroException("Division by zero.", context.Start.Line, context.Start.Column);
                }
                else if (rhs.Kind == PilotValueKind.Float && rhs.AsFloat() == 0.0)
                {
                    throw new PilotDivideByZeroException("Division by zero.", context.Start.Line, context.Start.Column);
                }
                val = val / rhs;
            }
            else if (opType == PilotParser.PERCENT)
            {
                if ((rhs.Kind == PilotValueKind.Int || rhs.Kind == PilotValueKind.UInt) && rhs.AsInt() == 0)
                {
                    throw new PilotDivideByZeroException("Modulo by zero.", context.Start.Line, context.Start.Column);
                }
                val = val % rhs;
            }
            else if (opType == PilotParser.DOUBLESLASH)
            {
                long divisor = rhs.AsInt();
                if (divisor == 0)
                {
                    throw new PilotDivideByZeroException("Integer division by zero.", context.Start.Line, context.Start.Column);
                }
                val = PilotValue.FromInt(val.AsInt() / divisor);
            }
        }

        return val;
    }

    private PilotValue EvaluateProjection(PilotValue source, PilotParser.FactorContext factorCtx)
    {
        var fieldNames = new List<string>();

        var atom = factorCtx.power()?.atom_expr()?.atom();
        if (atom?.LBRACE() != null && atom.dictorsetmaker() != null)
        {
            var dmaker = atom.dictorsetmaker();
            if (dmaker.collection_element() != null)
            {
                foreach (var elem in dmaker.collection_element())
                {
                    string fName = elem.test()?.GetText()?.Trim('"', '\'') ?? "";
                    if (!string.IsNullOrEmpty(fName)) fieldNames.Add(fName);
                }
            }
            if (dmaker.dict_entry() != null)
            {
                foreach (var entry in dmaker.dict_entry())
                {
                    string fName = entry.test(0)?.GetText()?.Trim('"', '\'') ?? "";
                    if (!string.IsNullOrEmpty(fName)) fieldNames.Add(fName);
                }
            }
        }
        else if (atom?.collection_expr() != null)
        {
            foreach (var el in atom.collection_expr().collection_element())
            {
                string fName = el.test()?.GetText()?.Trim('"', '\'') ?? "";
                if (!string.IsNullOrEmpty(fName)) fieldNames.Add(fName);
            }
        }
        else
        {
            var rhsVal = Visit(factorCtx);
            if (rhsVal.Kind == PilotValueKind.List)
            {
                foreach (var item in rhsVal.AsList()) fieldNames.Add(item.AsString());
            }
            else if (rhsVal.Kind == PilotValueKind.Set)
            {
                foreach (var item in rhsVal.AsSet()) fieldNames.Add(item.AsString());
            }
            else if (rhsVal.Kind == PilotValueKind.String)
            {
                fieldNames.Add(rhsVal.AsString());
            }
        }

        var map = new Dictionary<PilotValue, PilotValue>();
        foreach (var field in fieldNames)
        {
            var fVal = source.GetMember(field, Context);
            map[PilotValue.FromString(field)] = fVal;
        }

        return PilotValue.FromMap(map);
    }

    public override PilotValue VisitFactor(PilotParser.FactorContext context)
    {
        if (context.PLUS() != null) return Visit(context.factor());
        if (context.MINUS() != null) return -Visit(context.factor());
        if (context.power() != null) return Visit(context.power());
        if (context.unary_op() != null) return Visit(context.unary_op());
        return PilotValue.Undefined;
    }

    public override PilotValue VisitUnary_op(PilotParser.Unary_opContext context)
    {
        if (context.AWAIT() != null)
        {
            var val = Visit(context.factor());
            if (val.Kind == PilotValueKind.Future)
            {
                return val.AsFuture().Await();
            }
            return val;
        }

        if (context.SPAWN() != null)
        {
            var factor = context.factor();
            var future = Pilot.Runtime.Concurrency.PilotFuture.Run(() =>
            {
                var subContext = new PilotExecutionContext(
                    Context.Engine,
                    Context.Options,
                    new PilotEnvironment(Context.Environment.CurrentScope));
                var subVisitor = new PilotInterpreterVisitor(subContext);
                return subVisitor.Visit(factor);
            });
            return PilotValue.FromFuture(future);
        }

        if (context.ARROW_LEFT() != null)
        {
            return Visit(context.factor());
        }

        if (context.quote_expr() != null) return Visit(context.quote_expr());
        if (context.quasiquote_expr() != null) return Visit(context.quasiquote_expr());
        if (context.splice_expr() != null) return Visit(context.splice_expr());

        return PilotValue.Undefined;
    }

    public override PilotValue VisitPower(PilotParser.PowerContext context)
    {
        var baseVal = Visit(context.atom_expr());
        if (context.DOUBLESTAR() != null && context.factor() != null)
        {
            var expVal = Visit(context.factor());
            return PilotValue.FromFloat(Math.Pow(baseVal.AsFloat(), expVal.AsFloat()));
        }
        return baseVal;
    }

    #endregion

    #region Atom Expressions, Cascade Chaining, & Member Access

    public override PilotValue VisitAtom_expr(PilotParser.Atom_exprContext context)
    {
        PilotValue current;
        int startTrailerIdx = 0;

        var trailers = context.trailer();
        // Design-by-Contract: old(expr) interception during postcondition evaluations
        if (trailers?.Length > 0 &&
            trailers[0].LPAR() != null &&
            context.atom()?.identifier()?.GetText() == "old")
        {
            var arglist = trailers[0].arglist();
            string exprKey = arglist?.GetText() ?? "";
            if (Context.OldSnapshots.TryGetValue(exprKey, out var oldVal))
            {
                current = oldVal;
                startTrailerIdx = 1;
            }
            else
            {
                current = Visit(context.atom());
            }
        }
        else
        {
            current = Visit(context.atom());
        }

        for (int i = startTrailerIdx; i < trailers!.Length; i++)
        {
            var trailer = trailers[i];
            // Multi-cast Broadcast Call or Standard Function/Method Call: *(...) or (...)
            if ((trailer.STAR() != null && trailer.LPAR() != null) || trailer.LPAR() != null)
            {
                current = DispatchCall(current, trailer, trailers, i, context);
            }
            // Dot Member Access: .member
            else if (trailer.DOT() != null || trailer.SAFE_NAV() != null)
            {
                if (trailer.SAFE_NAV() != null && (current.IsNull || current.IsUndefined))
                {
                    current = PilotValue.Undefined;
                    continue;
                }

                if (trailer.DOT() != null && (current.IsNull || current.IsUndefined))
                {
                    string mName = trailer.identifier()?.GetText() ?? "unknown";
                    throw new PilotNullReferenceException($"Cannot access member '{mName}' on null or undefined value.", context.Start.Line, context.Start.Column);
                }

                string memberName = trailer.identifier().GetText();

                // Fast-path direct member call: receiver.method(args) without allocating an intermediate PilotFunction
                if (trailer.DOT() != null &&
                    i + 1 < trailers.Length &&
                    trailers[i + 1].LPAR() != null &&
                    trailers[i + 1].STAR() == null &&
                    !Context.IsAutoRepeatActive &&
                    TryInvokeDirectMemberCall(current, memberName, trailers[i + 1].arglist(), out var directResult))
                {
                    current = directResult;
                    i++;
                    continue;
                }

                current = current.GetMember(memberName, Context);
            }
            // Indexing: [index]
            else if (trailer.LSQB() != null)
            {
                var subCtx = trailer.subscriptlist()?.subscript(0);
                if (subCtx != null)
                {
                    var idxVal = Visit(subCtx.test(0));
                    current = current[idxVal];
                }
            }
            // Cascade Operator: ..method() or ..prop = val
            else if (trailer.cascade_trailer() != null)
            {
                current = EvaluateCascade(current, trailer.cascade_trailer());
            }
        }

        return current;
    }

    private bool TryInvokeDirectMemberCall(
        PilotValue receiver,
        string memberName,
        PilotParser.ArglistContext? arglist,
        out PilotValue result)
    {
        result = PilotValue.Undefined;
        var args = arglist?.argument();
        int argCount = args?.Length ?? 0;

        // Ensure no argument uses named or 'each'/'repeat'/'..' modifiers before taking direct fast-path
        for (int i = 0; i < argCount; i++)
        {
            var a = args![i];
            if (a.EACH() != null || a.REPEAT() != null || a.DOTDOT() != null || a.EQUAL() != null || a. test() == null)
            {
                return false;
            }
        }

        if (receiver.Kind == PilotValueKind.List)
        {
            var list = receiver.AsList();
            switch (memberName)
            {
                case "append":
                case "push":
                    if (argCount == 1)
                    {
                        list.Add(Visit(args![0].test()));
                        result = PilotValue.Null;
                        return true;
                    }
                    break;
                case "pop":
                    if (argCount == 0)
                    {
                        if (list.Count == 0)
                        {
                            result = PilotValue.Undefined;
                        }
                        else
                        {
                            result = list[^1];
                            list.RemoveAt(list.Count - 1);
                        }
                        return true;
                    }
                    break;
                case "clear":
                    if (argCount == 0)
                    {
                        list.Clear();
                        result = PilotValue.Null;
                        return true;
                    }
                    break;
            }
            return false;
        }

        if (receiver.Kind == PilotValueKind.Map)
        {
            var map = receiver.AsMap();
            if (map.ContainsKey(PilotValue.FromString(memberName)))
            {
                return false;
            }
            switch (memberName)
            {
                case "contains_key":
                case "ContainsKey":
                    if (argCount == 1)
                    {
                        var key = Visit(args![0].test());
                        result = PilotValue.FromBool(map.ContainsKey(key));
                        return true;
                    }
                    break;
                case "keys":
                    if (argCount == 0)
                    {
                        result = PilotValue.FromList(new List<PilotValue>(map.Keys));
                        return true;
                    }
                    break;
                case "values":
                    if (argCount == 0)
                    {
                        result = PilotValue.FromList(new List<PilotValue>(map.Values));
                        return true;
                    }
                    break;
                case "clear":
                    if (argCount == 0)
                    {
                        map.Clear();
                        result = PilotValue.Null;
                        return true;
                    }
                    break;
            }
            return false;
        }

        if (receiver.Kind == PilotValueKind.Set)
        {
            var set = receiver.AsSet();
            switch (memberName)
            {
                case "add":
                    if (argCount == 1)
                    {
                        set.Add(Visit(args![0].test()));
                        result = PilotValue.Null;
                        return true;
                    }
                    break;
                case "contains":
                case "Contains":
                    if (argCount == 1)
                    {
                        result = PilotValue.FromBool(set.Contains(Visit(args![0].test())));
                        return true;
                    }
                    break;
                case "clear":
                    if (argCount == 0)
                    {
                        set.Clear();
                        result = PilotValue.Null;
                        return true;
                    }
                    break;
            }
            return false;
        }

        if (receiver.Kind == PilotValueKind.String)
        {
            string s = receiver.AsString();
            switch (memberName)
            {
                case "to_upper":
                case "ToUpper":
                    if (argCount == 0)
                    {
                        result = PilotValue.FromString(s.ToUpperInvariant());
                        return true;
                    }
                    break;
                case "to_lower":
                case "ToLower":
                    if (argCount == 0)
                    {
                        result = PilotValue.FromString(s.ToLowerInvariant());
                        return true;
                    }
                    break;
                case "trim":
                case "Trim":
                    if (argCount == 0)
                    {
                        result = PilotValue.FromString(s.Trim());
                        return true;
                    }
                    break;
                case "starts_with":
                case "StartsWith":
                    if (argCount == 1)
                    {
                        result = PilotValue.FromBool(s.StartsWith(Visit(args![0].test()).ToString()));
                        return true;
                    }
                    break;
                case "ends_with":
                case "EndsWith":
                    if (argCount == 1)
                    {
                        result = PilotValue.FromBool(s.EndsWith(Visit(args![0].test()).ToString()));
                        return true;
                    }
                    break;
                case "contains":
                case "Contains":
                    if (argCount == 1)
                    {
                        result = PilotValue.FromBool(s.Contains(Visit(args![0].test()).ToString()));
                        return true;
                    }
                    break;
            }
            return false;
        }

        return false;
    }

    private PilotValue EvaluateCascade(PilotValue receiver, PilotParser.Cascade_trailerContext cascadeCtx)
    {
        var target = cascadeCtx.cascade_target();
        if (target == null) return receiver;

        // Cascade method call: ..func(args) or ..let res = func(args) or ..func(args) -> res
        if (target.dotted_name() != null && target.LPAR() != null)
        {
            string methodName = target.dotted_name().GetText();
            var member = receiver.GetMember(methodName, Context);
            var (posArgs, namedArgs) = EvaluateCallArguments(target.arglist());

            PilotValue callResult = PilotValue.Undefined;
            if (member.Kind == PilotValueKind.Function)
            {
                var boundArgs = BindArgumentsToFunction(member.AsFunction(), posArgs, namedArgs);
                callResult = member.AsFunction().Invoke(Context, boundArgs);
            }
            else if (member.Kind == PilotValueKind.MethodGroup)
            {
                callResult = member.AsMethodGroup().Invoke(Context, posArgs, namedArgs, isBroadcast: false);
            }

            // Bind intermediate return value if requested: (let res = func()) or func() -> res
            var binding = target.binding_pattern();
            if (binding?.Length > 0)
            {
                DeclareBinding(binding[0], callResult, isConst: false);
            }

            // In cascade chaining, return receiver self!
            return receiver;
        }

        // Cascade property assignment: ..prop = val
        if (target.dotted_name() != null && target.EQUAL() != null && target.test() != null)
        {
            string propName = target.dotted_name().GetText();
            var propVal = Visit(target.test());
            if (receiver.Kind == PilotValueKind.Object)
            {
                receiver.AsObject().SetMember(propName, propVal);
            }
            return receiver;
        }

        return receiver;
    }

    private PilotValue DispatchCall(
        PilotValue current,
        PilotParser.TrailerContext trailer,
        IReadOnlyList<PilotParser.TrailerContext> trailers,
        int trailerIndex,
        PilotParser.Atom_exprContext context)
    {
        bool isBroadcast = trailer.STAR() != null;
        var (posArgs, namedArgs, eachPos, eachNamed) = EvaluateCallArgumentsWithModifiers(trailer.arglist());

        // Check if repeated call (vectorization) is requested or required
        bool shouldRepeat = eachPos.Count > 0 || eachNamed.Count > 0;

        // Auto-repeat context (e.g. with auto_repeat: / with repeat:)
        if (!shouldRepeat && Context.IsAutoRepeatActive)
        {
            for (int i = 0; i < posArgs.Count; i++)
            {
                if (posArgs[i].Kind is PilotValueKind.List or PilotValueKind.Tuple or PilotValueKind.Set)
                {
                    if (ReferenceEquals(eachPos, EmptyEachPos)) eachPos = new HashSet<int>();
                    eachPos.Add(i);
                    shouldRepeat = true;
                    break;
                }
            }
        }

        // Broadcast on function with collection arguments (*(...) call)
        if (!shouldRepeat && isBroadcast && current.Kind == PilotValueKind.Function)
        {
            for (int i = 0; i < posArgs.Count; i++)
            {
                if (posArgs[i].Kind is PilotValueKind.List or PilotValueKind.Tuple or PilotValueKind.Set)
                {
                    if (ReferenceEquals(eachPos, EmptyEachPos)) eachPos = new HashSet<int>();
                    eachPos.Add(i);
                    shouldRepeat = true;
                    break;
                }
            }
        }

        // Target function is marked @vectorized or has a parameter marked 'each'
        if (!shouldRepeat && current.Kind == PilotValueKind.Function)
        {
            var fn = current.AsFunction();
            if (fn.IsVectorized)
            {
                for (int i = 0; i < posArgs.Count; i++)
                {
                    if (posArgs[i].Kind is PilotValueKind.List or PilotValueKind.Tuple or PilotValueKind.Set)
                    {
                        if (ReferenceEquals(eachPos, EmptyEachPos)) eachPos = new HashSet<int>();
                        eachPos.Add(i);
                        shouldRepeat = true;
                        break;
                    }
                }
            }
            else if (fn.Parameters.Count > 0 && posArgs.Count > 0)
            {
                for (int i = 0; i < fn.Parameters.Count && i < posArgs.Count; i++)
                {
                    if (fn.Parameters[i].IsEach && posArgs[i].Kind is PilotValueKind.List or PilotValueKind.Tuple or PilotValueKind.Set)
                    {
                        if (ReferenceEquals(eachPos, EmptyEachPos)) eachPos = new HashSet<int>();
                        eachPos.Add(i);
                        shouldRepeat = true;
                        break;
                    }
                }

                if (!shouldRepeat)
                {
                    for (int i = 0; i < fn.Parameters.Count && i < posArgs.Count; i++)
                    {
                        var pType = fn.Parameters[i].TypeName;
                        if (!string.IsNullOrWhiteSpace(pType) &&
                            !pType.Equals("list", StringComparison.OrdinalIgnoreCase) &&
                            !pType.Equals("array", StringComparison.OrdinalIgnoreCase) &&
                            !pType.Equals("any", StringComparison.OrdinalIgnoreCase) &&
                            !pType.Equals("dynamic", StringComparison.OrdinalIgnoreCase) &&
                            (posArgs[i].Kind is PilotValueKind.List or PilotValueKind.Tuple or PilotValueKind.Set))
                        {
                            if (ReferenceEquals(eachPos, EmptyEachPos)) eachPos = new HashSet<int>();
                            eachPos.Add(i);
                            shouldRepeat = true;
                            break;
                        }
                    }
                }
            }
        }

        // If repeated call is requested:
        if (shouldRepeat && eachPos.Count > 0)
        {
            int targetIdx = eachPos.First();
            var collectionVal = posArgs[targetIdx];
            var items = ExtractCollectionItems(collectionVal);
            var results = new List<PilotValue>(items.Count);

            foreach (var item in items)
            {
                var currentPos = new List<PilotValue>(posArgs);
                currentPos[targetIdx] = item;
                results.Add(InvokeCallableSingle(current, currentPos, namedArgs, context));
            }

            // If all returned null / undefined (like append / side-effect only methods), return Null
            if (results.All(r => r.IsNull || r.IsUndefined))
            {
                return PilotValue.Null;
            }
            return PilotValue.FromList(results);
        }

        // Standard Single Invocation / MethodGroup Broadcast
        return InvokeCallableSingle(current, posArgs, namedArgs, context, isBroadcast);
    }

    private PilotValue InvokeCallableSingle(
        PilotValue current,
        IReadOnlyList<PilotValue> posArgs,
        IReadOnlyDictionary<string, PilotValue> namedArgs,
        PilotParser.Atom_exprContext context,
        bool isBroadcast = false)
    {
        if (current.Kind == PilotValueKind.MethodGroup)
        {
            return current.AsMethodGroup().Invoke(Context, posArgs, namedArgs, isBroadcast: isBroadcast);
        }
        else if (current.Kind == PilotValueKind.Function)
        {
            var fn = current.AsFunction();
            var boundArgs = BindArgumentsToFunction(fn, posArgs, namedArgs);
            var res = fn.Invoke(Context, boundArgs);
            if (isBroadcast)
            {
                return PilotValue.FromList(new List<PilotValue> { res });
            }
            return res;
        }
        else if (current.IsNumber && posArgs.Count > 0 && isBroadcast)
        {
            return current * posArgs[0];
        }
        else
        {
            throw new PilotTypeException($"Attempted to invoke non-function value: {current}", context.Start.Line, context.Start.Column);
        }
    }

    private static List<PilotValue> ExtractCollectionItems(PilotValue val)
    {
        if (val.Kind == PilotValueKind.List) return val.AsList();
        if (val.Kind == PilotValueKind.Tuple) return val.AsTuple().ToList();
        if (val.Kind == PilotValueKind.Set) return val.AsSet().ToList();
        if (val.Kind == PilotValueKind.Range) return val.AsRange().Select(PilotValue.FromInt).ToList();
        if (val.Kind == PilotValueKind.String) return val.AsString().Select(PilotValue.FromChar).ToList();
        return new List<PilotValue> { val };
    }

    private (List<PilotValue> Positional, Dictionary<string, PilotValue> Named, HashSet<int> EachPositions, HashSet<string> EachNamed)
        EvaluateCallArgumentsWithModifiers(PilotParser.ArglistContext? arglist)
    {
        var args = arglist?.argument();
        if (args == null || args.Length == 0)
        {
            return (EmptyPositional, EmptyNamed, EmptyEachPos, EmptyEachNamed);
        }

        var positional = new List<PilotValue>(args.Length);
        Dictionary<string, PilotValue>? named = null;
        HashSet<int>? eachPos = null;
        HashSet<string>? eachNamed = null;

        for (int i = 0; i < args.Length; i++)
        {
            var arg = args[i];
            bool isEach = arg.EACH() != null || arg.REPEAT() != null || arg.DOTDOT() != null;
            if (arg.identifier() != null && arg.EQUAL() != null && arg.test() != null)
            {
                string key = arg.identifier().GetText();
                var val = Visit(arg.test());
                named ??= new Dictionary<string, PilotValue>(StringComparer.Ordinal);
                named[key] = val;
                if (isEach)
                {
                    eachNamed ??= new HashSet<string>(StringComparer.Ordinal);
                    eachNamed.Add(key);
                }
            }
            else if (arg.test() != null)
            {
                var val = Visit(arg.test());
                int idx = positional.Count;
                positional.Add(val);
                if (isEach)
                {
                    eachPos ??= new HashSet<int>();
                    eachPos.Add(idx);
                }
            }
        }
        return (positional, named ?? EmptyNamed, eachPos ?? EmptyEachPos, eachNamed ?? EmptyEachNamed);
    }

    private (List<PilotValue> Positional, Dictionary<string, PilotValue> Named) EvaluateCallArguments(PilotParser.ArglistContext? arglist)
    {
        var (pos, named, _, _) = EvaluateCallArgumentsWithModifiers(arglist);
        return (pos, named);
    }

    private IReadOnlyList<PilotValue> EvaluateArglist(PilotParser.ArglistContext? arglist)
    {
        return EvaluateCallArguments(arglist).Positional;
    }

    private IReadOnlyList<PilotValue> BindArgumentsToFunction(
        PilotFunction function,
        IReadOnlyList<PilotValue> posArgs,
        IReadOnlyDictionary<string, PilotValue> namedArgs)
    {
        if (namedArgs.Count == 0)
        {
            int paramCount = function.Parameters.Count;
            if (paramCount == 0 || posArgs.Count == 0)
            {
                return posArgs;
            }

            List<PilotValue>? adapted = null;
            for (int i = 0; i < posArgs.Count; i++)
            {
                var val = posArgs[i];
                if (i < paramCount && function.Parameters[i].TypeName != null && !val.IsUndefined)
                {
                    if (Context.TypeAdapters.TryConvert(val, function.Parameters[i].TypeName, Context, out var conv))
                    {
                        if (adapted == null)
                        {
                            adapted = new List<PilotValue>(posArgs.Count);
                            for (int j = 0; j < i; j++) adapted.Add(posArgs[j]);
                        }
                        val = conv;
                    }
                }
                adapted?.Add(val);
            }
            return adapted ?? posArgs;
        }

        var full = new List<PilotValue>();
        int posIdx = 0;
        for (int i = 0; i < function.Parameters.Count; i++)
        {
            var p = function.Parameters[i];
            if (namedArgs.TryGetValue(p.Name, out var nVal))
            {
                if (p.TypeName != null && !nVal.IsUndefined && Context.TypeAdapters.TryConvert(nVal, p.TypeName, Context, out var conv))
                {
                    nVal = conv;
                }
                full.Add(nVal);
            }
            else if (posIdx < posArgs.Count)
            {
                var val = posArgs[posIdx++];
                if (p.TypeName != null && !val.IsUndefined && Context.TypeAdapters.TryConvert(val, p.TypeName, Context, out var conv))
                {
                    val = conv;
                }
                full.Add(val);
            }
            else if (p.IsContextual)
            {
                break;
            }
            else if (p.DefaultValue.HasValue)
            {
                full.Add(p.DefaultValue.Value);
            }
            else
            {
                full.Add(PilotValue.Undefined);
            }
        }
        while (posIdx < posArgs.Count)
        {
            full.Add(posArgs[posIdx++]);
        }
        return full;
    }

    #endregion

    #region Atoms, Literals & Collections

    public override PilotValue VisitAtom(PilotParser.AtomContext context)
    {
        if (context.literal() != null) return Visit(context.literal());
        if (context.identifier() != null) return ResolveIdentifier(context.identifier().GetText());
        if (context.collection_expr() != null) return VisitCollection_expr(context.collection_expr());
        if (context.LPAR() != null && context.testlist_comp() != null) return VisitTestlist_comp(context.testlist_comp());
        if (context.LBRACE() != null && context.dictorsetmaker() != null) return VisitDictorsetmaker(context.dictorsetmaker());
        if (context.nameof_expr() != null) return PilotValue.FromString(context.nameof_expr().dotted_name()?.GetText() ?? context.nameof_expr().test()?.GetText() ?? "");
        if (context.inline_foreign_block() != null) return VisitInline_foreign_block(context.inline_foreign_block());
        if (context.quote_expr() != null) return PilotValue.FromString(context.quote_expr().GetText());
        if (context.quasiquote_expr() != null) return PilotValue.FromString(context.quasiquote_expr().GetText());
        if (context.splice_expr() != null) return ResolveIdentifier(context.splice_expr().GetText().TrimStart('$', '@'));

        return PilotValue.Undefined;
    }

    private PilotValue ResolveIdentifier(string name)
    {
        if (name == "true") return PilotValue.True;
        if (name == "false") return PilotValue.False;
        if (name == "null" || name == "none") return PilotValue.Null;
        if (name == "undefined") return PilotValue.Undefined;

        if (Context.Environment.TryResolve(name, out var val))
        {
            return val;
        }

        if (Context.Options.Builtins.TryGet(name, out var builtin))
        {
            return PilotValue.FromFunction(builtin);
        }

        throw new PilotRuntimeException($"Undefined variable '{name}'.");
    }

    public override PilotValue VisitLiteral(PilotParser.LiteralContext context)
    {
        if (context.TRUE() != null) return PilotValue.True;
        if (context.FALSE() != null) return PilotValue.False;
        if (context.NULL() != null || context.NONE() != null) return PilotValue.Null;
        if (context.UNDEFINED() != null) return PilotValue.Undefined;

        if (context.FSTRING()?.Length > 0)
        {
            string raw = context.FSTRING(0).GetText();
            return EvaluateFString(raw);
        }

        if (LiteralCache.TryGetValue(context, out var cached))
        {
            return cached;
        }

        if (context.INTEGER() != null)
        {
            string text = context.INTEGER().GetText();
            if (text.IndexOf('_') >= 0) text = text.Replace("_", "");
            var intVal = PilotValue.FromInt(long.Parse(text, CultureInfo.InvariantCulture));
            LiteralCache[context] = intVal;
            return intVal;
        }

        if (context.HEX_INTEGER() != null)
        {
            string text = context.HEX_INTEGER().GetText();
            if (text.IndexOf('_') >= 0) text = text.Replace("_", "");
            var intVal = PilotValue.FromInt(Convert.ToInt64(text[2..], 16));
            LiteralCache[context] = intVal;
            return intVal;
        }

        if (context.BIN_INTEGER() != null)
        {
            string text = context.BIN_INTEGER().GetText();
            if (text.IndexOf('_') >= 0) text = text.Replace("_", "");
            var intVal = PilotValue.FromInt(Convert.ToInt64(text[2..], 2));
            LiteralCache[context] = intVal;
            return intVal;
        }

        if (context.OCT_INTEGER() != null)
        {
            string text = context.OCT_INTEGER().GetText();
            if (text.IndexOf('_') >= 0) text = text.Replace("_", "");
            var intVal = PilotValue.FromInt(Convert.ToInt64(text[2..], 8));
            LiteralCache[context] = intVal;
            return intVal;
        }

        if (context.FLOAT_NUMBER() != null)
        {
            string text = context.FLOAT_NUMBER().GetText();
            if (text.IndexOf('_') >= 0) text = text.Replace("_", "");
            var floatVal = PilotValue.FromFloat(double.Parse(text, CultureInfo.InvariantCulture));
            LiteralCache[context] = floatVal;
            return floatVal;
        }

        if (context.STRING()?.Length > 0)
        {
            string raw = context.STRING(0).GetText();
            if ((raw.StartsWith("f\"", StringComparison.OrdinalIgnoreCase) || raw.StartsWith("f'", StringComparison.OrdinalIgnoreCase)) &&
                raw.Length >= 3 && (raw.EndsWith("\"") || raw.EndsWith("'")))
            {
                return EvaluateFString(raw);
            }
            var strVal = PilotValue.FromString(UnescapeString(raw));
            LiteralCache[context] = strVal;
            return strVal;
        }

        if (context.TRIPLE_STRING()?.Length > 0)
        {
            string raw = context.TRIPLE_STRING(0).GetText();
            if (raw.Length >= 6) raw = raw[3..^3];
            var tripleVal = PilotValue.FromString(raw);
            LiteralCache[context] = tripleVal;
            return tripleVal;
        }

        return PilotValue.Undefined;
    }

    private static bool IsSimpleIdentifier(string s)
    {
        if (string.IsNullOrEmpty(s)) return false;
        char first = s[0];
        if (!char.IsLetter(first) && first != '_') return false;
        for (int i = 1; i < s.Length; i++)
        {
            char c = s[i];
            if (!char.IsLetterOrDigit(c) && c != '_') return false;
        }
        return true;
    }

    private static FStringSegment[] ParseFStringSegments(string raw)
    {
        string content = raw.Substring(2, raw.Length - 3);
        var segments = new List<FStringSegment>();
        int i = 0;
        int literalStart = 0;
        while (i < content.Length)
        {
            if (content[i] == '{')
            {
                int endBrace = content.IndexOf('}', i);
                if (endBrace > i)
                {
                    if (i > literalStart)
                    {
                        segments.Add(new FStringSegment(false, content.Substring(literalStart, i - literalStart), false, null));
                    }
                    string expr = content.Substring(i + 1, endBrace - i - 1).Trim();
                    bool isSimple = IsSimpleIdentifier(expr);
                    PilotParser.TestContext? parsedExpr = null;
                    if (!isSimple)
                    {
                        try
                        {
                            parsedExpr = PilotParser.GetOrParseExpression(expr);
                        }
                        catch
                        {
                            parsedExpr = null;
                        }
                    }
                    segments.Add(new FStringSegment(true, expr, isSimple, parsedExpr));
                    i = endBrace + 1;
                    literalStart = i;
                    continue;
                }
            }
            i++;
        }
        if (literalStart < content.Length)
        {
            segments.Add(new FStringSegment(false, content.Substring(literalStart), false, null));
        }
        return segments.ToArray();
    }

    private PilotValue EvaluateFString(string raw)
    {
        var segments = FStringCache.GetOrAdd(raw, ParseFStringSegments);
        if (segments.Length == 0) return PilotValue.FromString(string.Empty);
        if (segments.Length == 1 && !segments[0].IsExpression) return PilotValue.FromString(segments[0].Text);

        var sb = new StringBuilder(raw.Length);
        for (int i = 0; i < segments.Length; i++)
        {
            ref readonly var seg = ref segments[i];
            if (!seg.IsExpression)
            {
                sb.Append(seg.Text);
            }
            else if (seg.IsSimpleIdentifier)
            {
                try
                {
                    sb.Append(ResolveIdentifier(seg.Text).ToString());
                }
                catch
                {
                    sb.Append(seg.Text);
                }
            }
            else if (seg.ParsedExpr != null)
            {
                try
                {
                    sb.Append(Visit(seg.ParsedExpr).ToString());
                }
                catch
                {
                    sb.Append(seg.Text);
                }
            }
            else
            {
                sb.Append(seg.Text);
            }
        }
        return PilotValue.FromString(sb.ToString());
    }

    private static string UnescapeString(string str)
    {
        if (str.Length >= 2 && ((str[0] == '"' && str[^1] == '"') || (str[0] == '\'' && str[^1] == '\'')))
        {
            str = str[1..^1];
        }
        if (str.IndexOf('\\') < 0)
        {
            return str;
        }
        return str.Replace("\\n", "\n").Replace("\\r", "\r").Replace("\\t", "\t").Replace("\\\"", "\"").Replace("\\\\", "\\");
    }

    public override PilotValue VisitCollection_expr(PilotParser.Collection_exprContext context)
    {
        var elements = context.collection_element();
        if (elements == null || elements.Length == 0)
        {
            return PilotValue.FromList(new List<PilotValue>());
        }

        var list = new List<PilotValue>(elements.Length);
        foreach (var el in elements)
        {
            var val = Visit(el.test());
            // Spread operator: ..sublist or *sublist
            if (el.DOTDOT() != null || el.STAR() != null)
            {
                if (val.Kind == PilotValueKind.List) list.AddRange(val.AsList());
                else if (val.Kind == PilotValueKind.Tuple) list.AddRange(val.AsTuple());
                else if (val.Kind == PilotValueKind.Set) list.AddRange(val.AsSet());
                else if (val.Kind == PilotValueKind.Range) list.AddRange(val.AsRange().Select(PilotValue.FromInt));
                else if (val.Kind == PilotValueKind.Generator) list.AddRange(val.AsGenerator());
                else list.Add(val);
            }
            else
            {
                list.Add(val);
            }
        }

        return PilotValue.FromList(list);
    }

    public override PilotValue VisitDictorsetmaker(PilotParser.DictorsetmakerContext context)
    {
        var dictEntries = context.dict_entry();
        if (dictEntries != null && dictEntries.Length > 0)
        {
            var map = new Dictionary<PilotValue, PilotValue>();
            foreach (var e in dictEntries)
            {
                if (e.DOTDOT() != null || e.DOUBLESTAR() != null)
                {
                    var spreadVal = Visit(e.test(0));
                    if (spreadVal.Kind == PilotValueKind.Map)
                    {
                        foreach (var kvp in spreadVal.AsMap())
                        {
                            map[kvp.Key] = kvp.Value;
                        }
                    }
                    else if (spreadVal.Kind == PilotValueKind.Object)
                    {
                        foreach (var kvp in spreadVal.AsObject().Fields)
                        {
                            map[PilotValue.FromString(kvp.Key)] = kvp.Value;
                        }
                    }
                }
                else
                {
                    var tests = e.test();
                    if (tests.Length >= 2)
                    {
                        map[Visit(tests[0])] = Visit(tests[1]);
                    }
                }
            }
            return PilotValue.FromMap(map);
        }

        var colElements = context.collection_element();
        if (colElements != null && colElements.Length > 0)
        {
            var set = new HashSet<PilotValue>();
            foreach (var el in colElements)
            {
                var val = Visit(el.test());
                if (el.DOTDOT() != null || el.STAR() != null)
                {
                    if (val.Kind == PilotValueKind.Set)
                    {
                        foreach (var item in val.AsSet()) set.Add(item);
                    }
                    else if (val.Kind == PilotValueKind.List)
                    {
                        foreach (var item in val.AsList()) set.Add(item);
                    }
                    else if (val.Kind == PilotValueKind.Tuple)
                    {
                        foreach (var item in val.AsTuple()) set.Add(item);
                    }
                    else if (val.Kind == PilotValueKind.Range)
                    {
                        foreach (var item in val.AsRange()) set.Add(PilotValue.FromInt(item));
                    }
                    else if (val.Kind == PilotValueKind.Generator)
                    {
                        foreach (var item in val.AsGenerator()) set.Add(item);
                    }
                    else
                    {
                        set.Add(val);
                    }
                }
                else
                {
                    set.Add(val);
                }
            }
            return PilotValue.FromSet(set);
        }

        return PilotValue.FromMap(new Dictionary<PilotValue, PilotValue>());
    }

    public override PilotValue VisitTestlist_comp(PilotParser.Testlist_compContext context)
    {
        var tests = context.test();
        if (tests == null || tests.Length == 0)
        {
            return PilotValue.FromTuple(Array.Empty<PilotValue>());
        }

        var items = new List<PilotValue>(tests.Length);
        foreach (var t in tests)
        {
            items.Add(Visit(t));
        }
        return items.Count == 1 ? items[0] : PilotValue.FromTuple(items);
    }

    public override PilotValue VisitInline_foreign_block(PilotParser.Inline_foreign_blockContext context)
    {
        string rawLang = context.foreign_language()?.GetText()?.ToLowerInvariant() ?? "python";
        if (rawLang == "c++") rawLang = "cpp";

        // Check if block contains yield -> Resumable Foreign Coroutine / Callable!
        if (context.stmt() != null && ContainsYield(context))
        {
            var stmts = context.stmt();
            int currentStmtIdx = 0;
            var coroutineScope = new PilotScope(Context.Environment.CurrentScope, $"{rawLang}:coroutine");

            return PilotValue.FromFunction(PilotFunction.CreateNative(
                $"{rawLang}_coroutine",
                (callCtx, args) =>
                {
                    var execContext = new PilotExecutionContext(
                        callCtx.Engine,
                        callCtx.Options,
                        new PilotEnvironment(coroutineScope),
                        callCtx.CancellationToken);
                    var subVisitor = execContext.GetOrCreateVisitor();

                    while (currentStmtIdx < stmts.Length)
                    {
                        var stmt = stmts[currentStmtIdx++];
                        // Check if statement contains a yield
                        if (stmt.simple_stmt()?.small_stmt() != null)
                        {
                            var flow = stmt.simple_stmt().small_stmt().FirstOrDefault()?.flow_stmt();
                            if (flow?.YIELD() != null)
                            {
                                PilotValue yielded = PilotValue.Undefined;
                                if (flow.testlist() != null)
                                {
                                    yielded = subVisitor.Visit(flow.testlist());
                                }

                                // If yielded value is a callable function and arguments were passed to coroutine:
                                if (yielded.Kind == PilotValueKind.Function && args.Count > 0)
                                {
                                    return yielded.AsFunction().Invoke(callCtx, args);
                                }

                                return yielded;
                            }
                        }

                        // Normal statement execution
                        subVisitor.Visit(stmt);
                        if (execContext.PendingFlow == PilotFlowSignalKind.Return)
                        {
                            var retVal = execContext.PendingReturnValue;
                            execContext.PendingFlow = PilotFlowSignalKind.None;
                            return retVal;
                        }
                    }

                    return PilotValue.Undefined;
                }));
        }

        string code = GetBlockBodyText(context);

        // For foreign languages with distinct dialects/statements (SQL, ASM, JS, Lua, Java, C, C++),
        // execute directly via dedicated polyglot provider
        if (rawLang is "postgresql" or "postgres" or "psql" or "mysql" or "sql" or "asm" or "assembly" or "js" or "javascript" or "lua" or "java" or "c" or "cpp")
        {
            if (Context.Options.Polyglot.TryGet(rawLang, out var langProvider))
            {
                return langProvider.ExecuteBlock(code, Context);
            }
        }

        // For Python/C#/Rust or general blocks with Pilot statements
        if (context.stmt() != null && context.stmt().Length > 0)
        {
            var subScope = new PilotScope(Context.Environment.CurrentScope, $"{rawLang}:inline");
            var subContext = new PilotExecutionContext(
                Context.Engine,
                Context.Options,
                new PilotEnvironment(subScope),
                Context.CancellationToken);
            var subVisitor = subContext.GetOrCreateVisitor();

            PilotValue lastVal = PilotValue.Undefined;
            try
            {
                foreach (var stmt in context.stmt())
                {
                    lastVal = subVisitor.Visit(stmt);
                    if (subContext.PendingFlow != PilotFlowSignalKind.None)
                    {
                        if (subContext.PendingFlow == PilotFlowSignalKind.Return)
                        {
                            lastVal = subContext.PendingReturnValue;
                        }
                        subContext.PendingFlow = PilotFlowSignalKind.None;
                        break;
                    }
                }
                if (!lastVal.IsUndefined) return lastVal;
            }
            catch (PilotReturnSignal ret)
            {
                return ret.Value;
            }
            catch
            {
                // Foreign syntax or unresolved foreign module (e.g. math.sin in python) -> fall back to polyglot provider
            }
        }

        if (Context.Options.Polyglot.TryGet(rawLang, out var provider))
        {
            return provider.ExecuteBlock(code, Context);
        }

        return PilotValue.FromString($"[{rawLang} block: {code}]");
    }

    private static string GetBlockBodyText(PilotParser.Inline_foreign_blockContext context)
    {
        if (context.RAW_DSL_BLOCK() != null)
        {
            string raw = context.RAW_DSL_BLOCK().GetText();
            int firstNl = raw.IndexOf('\n');
            if (firstNl >= 0 && raw.EndsWith("```"))
            {
                return raw[(firstNl + 1)..^3].Trim();
            }
            return raw.Trim('`', ' ');
        }
        if (context.TRIPLE_STRING() != null)
        {
            return context.TRIPLE_STRING().GetText()[3..^3].Trim();
        }
        if (context.STRING() != null)
        {
            return context.STRING().GetText()[1..^1].Trim();
        }
        if (context.LBRACE() != null && context.RBRACE() != null)
        {
            int start = context.LBRACE().Symbol.StartIndex + 1;
            int stop = context.RBRACE().Symbol.StartIndex - 1;
            if (stop >= start)
            {
                return context.Start.InputStream.GetText(new Antlr4.Runtime.Misc.Interval(start, stop)).Trim();
            }
        }
        return string.Empty;
    }

    #endregion
}
