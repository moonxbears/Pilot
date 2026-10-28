using System;
using System.Collections.Generic;
using System.Linq;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Analysis.TypeSystem;

namespace Pilot.Analysis.ContractVerification;

/// <summary>
/// Static contract solver and design-by-contract predicate verifier for Pilot (Milestone 3 &amp; 19.1.6).
/// Evaluates preconditions (`requires`), postconditions (`ensures`), and assertions (`assert`) at compile time
/// using constant &amp; interval propagation, recording statically discharged contracts so bytecode and native
/// emitters can elide proven runtime checks.
/// </summary>
public sealed class PilotContractVerifier : PilotBaseVisitor<object?>
{
    private readonly Dictionary<string, FunctionContractInfo> _contracts = new(StringComparer.Ordinal);
    private readonly Dictionary<string, long> _knownConstants = new(StringComparer.Ordinal);
    private readonly HashSet<IParseTree> _dischargedNodes = new(ReferenceEqualityComparer.Instance);
    private readonly HashSet<string> _dischargedFunctions = new(StringComparer.Ordinal);
    private readonly HashSet<string> _functionsWithUnprovenCallSites = new(StringComparer.Ordinal);
    private readonly List<PilotAnalysisDiagnostic> _diagnostics = new();

    public IReadOnlyList<PilotAnalysisDiagnostic> Diagnostics => _diagnostics;
    public IReadOnlySet<IParseTree> DischargedNodes => _dischargedNodes;
    public IReadOnlySet<string> DischargedFunctions => _dischargedFunctions;

    public bool IsFunctionContractFullyDischarged(string funcName) =>
        _dischargedFunctions.Contains(funcName) && !_functionsWithUnprovenCallSites.Contains(funcName);

    private sealed class FunctionContractInfo
    {
        public string Name { get; }
        public PilotParser.Func_defContext Context { get; }
        public IReadOnlyList<string> Parameters { get; }
        public List<string> Preconditions { get; } = new();
        public List<string> Postconditions { get; } = new();
        public int VerifiedCallCount { get; set; }
        public int UnprovenCallCount { get; set; }

        public FunctionContractInfo(string name, PilotParser.Func_defContext context, IReadOnlyList<string> parameters)
        {
            Name = name;
            Context = context;
            Parameters = parameters;
        }
    }

    public override object? VisitProgram(PilotParser.ProgramContext context)
    {
        base.VisitProgram(context);

        foreach (var (name, info) in _contracts)
        {
            bool hasClauses = info.Preconditions.Count > 0 || info.Postconditions.Count > 0;
            if (!hasClauses)
            {
                _dischargedFunctions.Add(name);
                _dischargedNodes.Add(info.Context);
            }
            else if (info.UnprovenCallCount == 0 && info.VerifiedCallCount > 0 && info.Postconditions.Count == 0)
            {
                _dischargedFunctions.Add(name);
                _dischargedNodes.Add(info.Context);
            }
        }

        return null;
    }

    public override object? VisitConst_stmt(PilotParser.Const_stmtContext context)
    {
        string? name = context.binding_pattern()?.identifier()?.GetText();
        string? rhs = context.test()?.GetText();
        if (!string.IsNullOrEmpty(name) && !string.IsNullOrEmpty(rhs) && TryEvaluateConstantInt(rhs, out long val))
        {
            _knownConstants[name] = val;
        }
        return base.VisitConst_stmt(context);
    }

    public override object? VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        string? name = context.binding_pattern()?.identifier()?.GetText();
        string? rhs = context.test()?.GetText();
        if (!string.IsNullOrEmpty(name) && !string.IsNullOrEmpty(rhs) && TryEvaluateConstantInt(rhs, out long val))
        {
            _knownConstants[name] = val;
        }
        return base.VisitLet_stmt(context);
    }

    public override object? VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        string? name = context.binding_pattern()?.identifier()?.GetText();
        if (!string.IsNullOrEmpty(name))
        {
            _knownConstants.Remove(name);
        }
        return base.VisitVar_stmt(context);
    }

    public override object? VisitFunc_def(PilotParser.Func_defContext context)
    {
        string funcName = context.identifier().GetText();
        var paramNames = new List<string>();
        var parameters = context.parameters()?.parameter();
        if (parameters != null)
        {
            foreach (var p in parameters)
            {
                paramNames.Add(p.identifier()?.GetText() ?? "arg");
            }
        }

        var contractInfo = new FunctionContractInfo(funcName, context, paramNames);

        var clauses = context.contract_clause();
        if (clauses != null)
        {
            foreach (var c in clauses)
            {
                if (c.REQUIRES() != null && c.test() != null)
                {
                    contractInfo.Preconditions.Add(c.test().GetText());
                }
                else if (c.ENSURES() != null && c.test() != null)
                {
                    contractInfo.Postconditions.Add(c.test().GetText());
                }
            }
        }

        _contracts[funcName] = contractInfo;

        if (context.suite() != null)
        {
            VisitSuite(context.suite());
        }
        return null;
    }

    public override object? VisitAtom_expr(PilotParser.Atom_exprContext context)
    {
        var atom = context.atom();
        var trailers = context.trailer();

        if (atom?.identifier() != null && trailers != null && trailers.Length > 0)
        {
            string funcName = atom.identifier().GetText();
            var firstTrailer = trailers[0];

            if (firstTrailer.LPAR() != null && _contracts.TryGetValue(funcName, out var contract))
            {
                var argList = firstTrailer.arglist()?.argument() ?? Array.Empty<PilotParser.ArgumentContext>();
                VerifyCallPreconditions(context, funcName, contract, argList, context.Start.Line, context.Start.Column);
            }
        }

        return base.VisitAtom_expr(context);
    }

    public override object? VisitAssert_stmt(PilotParser.Assert_stmtContext context)
    {
        if (context.test() != null && context.test().Length > 0)
        {
            var conditionNode = context.test(0);
            string condText = conditionNode.GetText();

            if (TryEvaluateConstantBool(condText, out bool isTrue))
            {
                if (!isTrue)
                {
                    _diagnostics.Add(new PilotAnalysisDiagnostic(
                        context.COMPTIME() != null ? "COMPTIME008" : "PILOT306",
                        $"Static assertion guaranteed to fail at compile-time: 'assert {condText}'.",
                        context.Start.Line,
                        context.Start.Column));
                }
                else
                {
                    _dischargedNodes.Add(context);
                }
            }
        }
        return null;
    }

    private void VerifyCallPreconditions(
        PilotParser.Atom_exprContext callNode,
        string funcName,
        FunctionContractInfo contract,
        PilotParser.ArgumentContext[] args,
        int line,
        int column)
    {
        if (contract.Preconditions.Count == 0)
        {
            _dischargedNodes.Add(callNode);
            contract.VerifiedCallCount++;
            return;
        }

        var paramValues = new Dictionary<string, long>(StringComparer.Ordinal);
        for (int i = 0; i < Math.Min(contract.Parameters.Count, args.Length); i++)
        {
            string argText = args[i].test()?.GetText() ?? "";
            if (TryEvaluateConstantInt(argText, out long constVal))
            {
                paramValues[contract.Parameters[i]] = constVal;
            }
        }

        bool allProven = true;
        foreach (var pre in contract.Preconditions)
        {
            if (TryEvaluatePrecondition(pre, paramValues, out bool passed))
            {
                if (!passed)
                {
                    allProven = false;
                    _diagnostics.Add(new PilotAnalysisDiagnostic(
                        "PILOT306",
                        $"Static contract violation calling '{funcName}': precondition 'requires {pre}' evaluated to false with constant argument(s).",
                        line,
                        column));
                }
            }
            else
            {
                allProven = false;
            }
        }

        if (allProven)
        {
            _dischargedNodes.Add(callNode);
            contract.VerifiedCallCount++;
        }
        else
        {
            contract.UnprovenCallCount++;
            _functionsWithUnprovenCallSites.Add(funcName);
        }
    }

    private bool TryEvaluateConstantInt(string expr, out long value)
    {
        if (long.TryParse(expr, out value)) return true;
        if (_knownConstants.TryGetValue(expr, out value)) return true;
        return false;
    }

    private static bool TryEvaluatePrecondition(string expr, Dictionary<string, long> paramValues, out bool result)
    {
        result = true;
        if (expr.Contains("&&", StringComparison.Ordinal))
        {
            var conjuncts = expr.Split("&&", StringSplitOptions.RemoveEmptyEntries);
            foreach (var c in conjuncts)
            {
                if (!TryEvaluatePrecondition(c.Trim(), paramValues, out bool sub) || !sub)
                {
                    result = false;
                    return sub ? false : true;
                }
            }
            return true;
        }

        foreach (var kvp in paramValues)
        {
            if (expr.StartsWith($"{kvp.Key}>="))
            {
                if (long.TryParse(expr[(kvp.Key.Length + 2)..], out long threshold))
                {
                    result = kvp.Value >= threshold;
                    return true;
                }
            }
            if (expr.StartsWith($"{kvp.Key}>"))
            {
                if (long.TryParse(expr[(kvp.Key.Length + 1)..], out long threshold))
                {
                    result = kvp.Value > threshold;
                    return true;
                }
            }
            if (expr.StartsWith($"{kvp.Key}<="))
            {
                if (long.TryParse(expr[(kvp.Key.Length + 2)..], out long threshold))
                {
                    result = kvp.Value <= threshold;
                    return true;
                }
            }
            if (expr.StartsWith($"{kvp.Key}<"))
            {
                if (long.TryParse(expr[(kvp.Key.Length + 1)..], out long threshold))
                {
                    result = kvp.Value < threshold;
                    return true;
                }
            }
            if (expr.StartsWith($"{kvp.Key}!=") || expr.StartsWith($"{kvp.Key}=="))
            {
                bool isEq = expr.Contains("==");
                string rhs = expr[(kvp.Key.Length + 2)..];
                if (long.TryParse(rhs, out long threshold))
                {
                    result = isEq ? kvp.Value == threshold : kvp.Value != threshold;
                    return true;
                }
            }
        }

        return false;
    }

    private bool TryEvaluateConstantBool(string expr, out bool result)
    {
        result = true;
        if (expr == "false" || expr == "False") { result = false; return true; }
        if (expr == "true" || expr == "True") { result = true; return true; }

        string[] ops = { ">=", "<=", "==", "!=", ">", "<" };
        foreach (var op in ops)
        {
            int idx = expr.IndexOf(op, StringComparison.Ordinal);
            if (idx > 0)
            {
                string lhsStr = expr[..idx].Trim();
                string rhsStr = expr[(idx + op.Length)..].Trim();
                if (TryEvaluateConstantInt(lhsStr, out long left) && TryEvaluateConstantInt(rhsStr, out long right))
                {
                    result = op switch
                    {
                        ">=" => left >= right,
                        "<=" => left <= right,
                        "==" => left == right,
                        "!=" => left != right,
                        ">" => left > right,
                        "<" => left < right,
                        _ => false
                    };
                    return true;
                }
            }
        }
        return false;
    }
}
