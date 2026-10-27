using System;
using System.Collections.Generic;
using System.Linq;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Analysis.TypeSystem;

namespace Pilot.Analysis.Ownership;

/// <summary>
/// Ownership lifecycle state of a tracked resource binding.
/// </summary>
public enum ResourceState
{
    Active,
    Moved,
    BorrowedImmutable,
    BorrowedMutable,
    Consumed
}

/// <summary>
/// Resource affinity classification.
/// </summary>
public enum ResourceAffinity
{
    /// <summary>
    /// Normal Python/C#-like reference semantics. Handled transparently by runtime GC/ARC.
    /// Incurs zero borrow checking friction or lifetime restrictions.
    /// </summary>
    Normal,

    /// <summary>
    /// Affine resource: Can be used at most once; moving prevents subsequent reads or writes.
    /// </summary>
    Affine,

    /// <summary>
    /// Linear resource: Must be used/consumed/closed exactly once before leaving scope.
    /// </summary>
    Linear
}

public sealed class ResourceBinding
{
    public string Name { get; }
    public ResourceAffinity Affinity { get; }
    public ResourceState State { get; set; } = ResourceState.Active;
    public int DeclaredLine { get; }
    public int MovedAtLine { get; set; }
    public int SharedBorrowCount { get; set; }
    public string? MutableBorrower { get; set; }

    public ResourceBinding(string name, ResourceAffinity affinity, int line)
    {
        Name = name;
        Affinity = affinity;
        DeclaredLine = line;
    }
}

/// <summary>
/// Affine and Linear Resource Tracker & Borrow Checker for Pilot.
/// Ensures safe ownership transfer and exclusivity for designated resources (@resource, @linear, @affine)
/// and enforces zero-allocation bounds in @hard_realtime / noalloc scopes,
/// while preserving Python/C#-like simplicity for all regular scripting code.
/// </summary>
public sealed class PilotBorrowChecker : PilotBaseVisitor<object?>
{
    private readonly Stack<Dictionary<string, ResourceBinding>> _scopes = new();
    private readonly HashSet<string> _affineTypes = new(StringComparer.OrdinalIgnoreCase);
    private readonly HashSet<string> _linearTypes = new(StringComparer.OrdinalIgnoreCase);
    private readonly List<PilotAnalysisDiagnostic> _diagnostics = new();
    private bool _inHardRealtimeScope = false;
    private bool _inNoAllocScope = false;

    public IReadOnlyList<PilotAnalysisDiagnostic> Diagnostics => _diagnostics;

    public PilotBorrowChecker()
    {
        PushScope();
    }

    private void PushScope()
    {
        _scopes.Push(new Dictionary<string, ResourceBinding>(StringComparer.Ordinal));
    }

    private void PopScope()
    {
        if (_scopes.Count > 1)
        {
            var leavingScope = _scopes.Pop();
            foreach (var kvp in leavingScope)
            {
                var binding = kvp.Value;
                // Linear resources must be consumed or closed before leaving scope
                if (binding.Affinity == ResourceAffinity.Linear &&
                    binding.State != ResourceState.Consumed &&
                    binding.State != ResourceState.Moved)
                {
                    _diagnostics.Add(new PilotAnalysisDiagnostic(
                        "PILOT303",
                        $"Linear resource '{binding.Name}' (marked '@linear') was declared at line {binding.DeclaredLine} but was never consumed or closed before scope exit.",
                        binding.DeclaredLine,
                        0));
                }
            }
        }
    }

    private void CheckRootScopeExit()
    {
        if (_scopes.Count > 0)
        {
            var rootScope = _scopes.Peek();
            foreach (var kvp in rootScope)
            {
                var binding = kvp.Value;
                if (binding.Affinity == ResourceAffinity.Linear &&
                    binding.State != ResourceState.Consumed &&
                    binding.State != ResourceState.Moved)
                {
                    _diagnostics.Add(new PilotAnalysisDiagnostic(
                        "PILOT303",
                        $"Linear resource '{binding.Name}' (marked '@linear') was declared at line {binding.DeclaredLine} but was never consumed or closed before scope exit.",
                        binding.DeclaredLine,
                        0));
                }
            }
        }
    }

    public void RegisterResource(string name, ResourceAffinity affinity, int line)
    {
        if (string.IsNullOrWhiteSpace(name)) return;
        if (_scopes.Count > 0)
        {
            _scopes.Peek()[name] = new ResourceBinding(name, affinity, line);
        }
    }

    private bool TryLookupResource(string name, out ResourceBinding binding)
    {
        foreach (var scope in _scopes)
        {
            if (scope.TryGetValue(name, out var b))
            {
                binding = b;
                return true;
            }
        }
        binding = null!;
        return false;
    }

    public override object? VisitProgram(PilotParser.ProgramContext context)
    {
        var stmts = context.stmt();
        if (stmts != null)
        {
            foreach (var s in stmts)
            {
                Visit(s);
            }
        }
        CheckRootScopeExit();
        return null;
    }

    public override object? VisitStruct_def(PilotParser.Struct_defContext context)
    {
        string structName = context.identifier().GetText();
        if (context.decorators() != null)
        {
            foreach (var d in context.decorators().decorator())
            {
                string dName = d.dotted_name().GetText();
                if (dName.Equals("resource", StringComparison.OrdinalIgnoreCase) || dName.Equals("affine", StringComparison.OrdinalIgnoreCase))
                    _affineTypes.Add(structName);
                else if (dName.Equals("linear", StringComparison.OrdinalIgnoreCase))
                    _linearTypes.Add(structName);
            }
        }
        return base.VisitStruct_def(context);
    }

    public override object? VisitClass_def(PilotParser.Class_defContext context)
    {
        string className = context.identifier().GetText();
        if (context.decorators() != null)
        {
            foreach (var d in context.decorators().decorator())
            {
                string dName = d.dotted_name().GetText();
                if (dName.Equals("resource", StringComparison.OrdinalIgnoreCase) || dName.Equals("affine", StringComparison.OrdinalIgnoreCase))
                    _affineTypes.Add(className);
                else if (dName.Equals("linear", StringComparison.OrdinalIgnoreCase))
                    _linearTypes.Add(className);
            }
        }
        return base.VisitClass_def(context);
    }

    public override object? VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "";
        if (string.IsNullOrEmpty(name) && context.binding_pattern() != null)
        {
            string raw = context.binding_pattern().GetText() ?? "";
            if (!raw.Contains('(') && !raw.Contains('[') && !raw.Contains('{'))
            {
                name = raw;
            }
        }
        string typeStr = context.type_annotation()?.type_spec()?.GetText() ?? "";
        string rhsExact = context.test() != null ? GetExactText(context.test()) : "";

        bool isLinear = _linearTypes.Contains(typeStr) || _linearTypes.Any(lt => rhsExact.Contains(lt));
        bool isAffine = _affineTypes.Contains(typeStr) || _affineTypes.Any(at => rhsExact.Contains(at));

        ResourceAffinity affinity = isLinear ? ResourceAffinity.Linear
                                 : isAffine ? ResourceAffinity.Affine
                                 : ResourceAffinity.Normal;

        // Check RHS for moves or allocations
        if (context.test() != null)
        {
            bool moved = CheckRhsForMove(name, context.test());
            if (!moved)
            {
                CheckExpressionUsage(context.test());
            }
            CheckZeroAllocation(context.test());
        }

        if (affinity != ResourceAffinity.Normal && !string.IsNullOrWhiteSpace(name))
        {
            RegisterResource(name, affinity, context.Start.Line);
        }

        return null;
    }

    public override object? VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "";
        if (string.IsNullOrEmpty(name) && context.binding_pattern() != null)
        {
            string raw = context.binding_pattern().GetText() ?? "";
            if (!raw.Contains('(') && !raw.Contains('[') && !raw.Contains('{'))
            {
                name = raw;
            }
        }
        string typeStr = context.type_annotation()?.type_spec()?.GetText() ?? "";
        string rhsExact = context.test() != null ? GetExactText(context.test()) : "";

        bool isLinear = _linearTypes.Contains(typeStr) || _linearTypes.Any(lt => rhsExact.Contains(lt));
        bool isAffine = _affineTypes.Contains(typeStr) || _affineTypes.Any(at => rhsExact.Contains(at));

        ResourceAffinity affinity = isLinear ? ResourceAffinity.Linear
                                 : isAffine ? ResourceAffinity.Affine
                                 : ResourceAffinity.Normal;

        if (context.test() != null)
        {
            bool moved = CheckRhsForMove(name, context.test());
            if (!moved)
            {
                CheckExpressionUsage(context.test());
            }
            CheckZeroAllocation(context.test());
        }

        if (affinity != ResourceAffinity.Normal && !string.IsNullOrWhiteSpace(name))
        {
            RegisterResource(name, affinity, context.Start.Line);
        }

        return null;
    }

    public override object? VisitFunc_def(PilotParser.Func_defContext context)
    {
        bool prevRt = _inHardRealtimeScope;
        bool prevNoAlloc = _inNoAllocScope;

        if (context.decorators() != null)
        {
            foreach (var d in context.decorators().decorator())
            {
                string dName = d.dotted_name().GetText();
                if (dName.Equals("hard_realtime", StringComparison.OrdinalIgnoreCase)) _inHardRealtimeScope = true;
                if (dName.Equals("noalloc", StringComparison.OrdinalIgnoreCase)) _inNoAllocScope = true;
            }
        }

        PushScope();
        VisitSuite(context.suite());
        PopScope();

        _inHardRealtimeScope = prevRt;
        _inNoAllocScope = prevNoAlloc;
        return null;
    }

    public override object? VisitExpr_stmt(PilotParser.Expr_stmtContext context)
    {
        var testList = context.testlist();
        if (testList == null) return null;

        var tests = testList.test();
        if (tests != null && tests.Length > 0)
        {
            // Simple assignment: target = rhs
            if (context.EQUAL()?.Length > 0 && context.test()?.Length > 0)
            {
                string targetName = tests[0].GetText();
                bool moved = CheckRhsForMove(targetName, context.test(0));
                if (!moved)
                {
                    CheckExpressionUsage(context.test(0));
                }
                CheckZeroAllocation(context.test(0));
            }
            else
            {
                foreach (var t in tests)
                {
                    CheckExpressionUsage(t);
                    CheckZeroAllocation(t);
                }
            }
        }
        return null;
    }

    private bool CheckRhsForMove(string targetName, PilotParser.TestContext rhs)
    {
        string rhsExact = GetExactText(rhs).Trim();
        string rhsRaw = rhs.GetText() ?? "";

        // 1. Explicit move: move x or take(x)
        if (rhsExact.StartsWith("move ", StringComparison.Ordinal) ||
            rhsExact.StartsWith("take(", StringComparison.Ordinal) ||
            rhsRaw.StartsWith("move", StringComparison.Ordinal) ||
            rhsRaw.StartsWith("take(", StringComparison.Ordinal))
        {
            string sourceVar = rhsExact.StartsWith("move ", StringComparison.Ordinal)
                ? rhsExact[5..].Trim()
                : rhsExact.StartsWith("take(", StringComparison.Ordinal) && rhsExact.EndsWith(")")
                    ? rhsExact[5..^1].Trim()
                    : rhsRaw.StartsWith("move", StringComparison.Ordinal) && rhsRaw.Length > 4
                        ? rhsRaw[4..].Trim()
                        : rhsRaw.StartsWith("take(", StringComparison.Ordinal) && rhsRaw.EndsWith(")")
                            ? rhsRaw[5..^1].Trim()
                            : "";

            if (string.IsNullOrWhiteSpace(sourceVar)) return false;

            if (TryLookupResource(sourceVar, out var res))
            {
                if (res.State == ResourceState.Moved)
                {
                    _diagnostics.Add(new PilotAnalysisDiagnostic(
                        "PILOT301",
                        $"Use-after-move violation: resource '{sourceVar}' was already moved at line {res.MovedAtLine}.",
                        rhs.Start.Line,
                        rhs.Start.Column));
                }
                else if (res.State == ResourceState.BorrowedImmutable || res.State == ResourceState.BorrowedMutable)
                {
                    _diagnostics.Add(new PilotAnalysisDiagnostic(
                        "PILOT302",
                        $"Cannot move resource '{sourceVar}' while it is currently borrowed.",
                        rhs.Start.Line,
                        rhs.Start.Column));
                }
                else
                {
                    res.State = ResourceState.Moved;
                    res.MovedAtLine = rhs.Start.Line;
                    if (!string.IsNullOrWhiteSpace(targetName))
                    {
                        RegisterResource(targetName, res.Affinity, rhs.Start.Line);
                    }
                }
            }
            else
            {
                var newRes = new ResourceBinding(sourceVar, ResourceAffinity.Affine, rhs.Start.Line)
                {
                    State = ResourceState.Moved,
                    MovedAtLine = rhs.Start.Line
                };
                if (_scopes.Count > 0) _scopes.Peek()[sourceVar] = newRes;
                if (!string.IsNullOrWhiteSpace(targetName))
                {
                    RegisterResource(targetName, ResourceAffinity.Affine, rhs.Start.Line);
                }
            }
            return true;
        }

        // 2. Linear/Affine resource direct assignment moves ownership (e.g. let b = a)
        string simpleId = rhsExact;
        if (!string.IsNullOrWhiteSpace(simpleId) &&
            TryLookupResource(simpleId, out var directRes) &&
            directRes.Affinity != ResourceAffinity.Normal)
        {
            if (directRes.State == ResourceState.Moved)
            {
                _diagnostics.Add(new PilotAnalysisDiagnostic(
                    "PILOT301",
                    $"Use-after-move violation: resource '{simpleId}' was already moved at line {directRes.MovedAtLine}.",
                    rhs.Start.Line,
                    rhs.Start.Column));
            }
            else
            {
                directRes.State = ResourceState.Moved;
                directRes.MovedAtLine = rhs.Start.Line;
                if (!string.IsNullOrWhiteSpace(targetName))
                {
                    RegisterResource(targetName, directRes.Affinity, rhs.Start.Line);
                }
            }
            return true;
        }

        return false;
    }

    private void CheckExpressionUsage(PilotParser.TestContext expr)
    {
        string exprText = GetExactText(expr);

        // Check if an identifier in the expression is a moved resource
        foreach (var scope in _scopes)
        {
            foreach (var kvp in scope)
            {
                string varName = kvp.Key;
                if (string.IsNullOrWhiteSpace(varName)) continue;
                var res = kvp.Value;

                if (System.Text.RegularExpressions.Regex.IsMatch(exprText, $@"\b{System.Text.RegularExpressions.Regex.Escape(varName)}\b") ||
                    expr.GetText().Contains(varName))
                {
                    // If invoked as close() or dispose(), mark consumed
                    if (exprText.Contains($"{varName}.close()") || exprText.Contains($"{varName}.dispose()"))
                    {
                        res.State = ResourceState.Consumed;
                        return;
                    }

                    if (res.State == ResourceState.Moved)
                    {
                        _diagnostics.Add(new PilotAnalysisDiagnostic(
                            "PILOT301",
                            $"Cannot access resource '{varName}'. It was previously moved at line {res.MovedAtLine}.",
                            expr.Start.Line,
                            expr.Start.Column));
                    }
                }
            }
        }
    }

    private void CheckZeroAllocation(PilotParser.TestContext expr)
    {
        if (!_inHardRealtimeScope && !_inNoAllocScope) return;

        string text = GetExactText(expr);
        // Disallowed dynamic heap allocations in hard_realtime or noalloc
        if (text.Contains(".append(") || text.Contains("spawn ") || text.Contains("async ") || text.Contains("new "))
        {
            _diagnostics.Add(new PilotAnalysisDiagnostic(
                "PILOT304",
                $"Dynamic heap allocation operation is forbidden inside '@hard_realtime' or 'noalloc' context: '{text}'.",
                expr.Start.Line,
                expr.Start.Column));
        }
    }

    private static string GetExactText(Antlr4.Runtime.ParserRuleContext? context)
    {
        if (context == null) return string.Empty;
        if (context.Start == null || context.Stop == null || context.Start.InputStream == null)
            return context.GetText() ?? string.Empty;
        int start = Math.Min(context.Start.StartIndex, context.Stop.StopIndex);
        int stop = Math.Max(context.Start.StartIndex, context.Stop.StopIndex);
        if (start < 0 || stop < 0) return context.GetText() ?? string.Empty;
        return context.Start.InputStream.GetText(new Antlr4.Runtime.Misc.Interval(start, stop));
    }

    private static bool HasDecorator(PilotParser.Declaration_modifierContext[]? modifiers, string name)
    {
        if (modifiers == null) return false;
        return modifiers.Any(m => string.Equals(m.GetText().TrimStart('@'), name, StringComparison.OrdinalIgnoreCase));
    }
}
