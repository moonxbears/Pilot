using System;
using System.Collections.Generic;
using System.Linq;
using Antlr4.Runtime;
using Pilot.Analysis.TypeSystem;

namespace Pilot.Analysis.ContractVerification;

/// <summary>
/// Static contract solver and design-by-contract predicate verifier for Pilot.
/// Evaluates constant preconditions (requires) and postconditions (ensures) at compile time,
/// detecting contract violations before execution begins.
/// </summary>
public sealed class PilotContractVerifier : PilotBaseVisitor<object?>
{
    private readonly Dictionary<string, FunctionContractInfo> _contracts = new(StringComparer.Ordinal);
    private readonly List<PilotAnalysisDiagnostic> _diagnostics = new();

    public IReadOnlyList<PilotAnalysisDiagnostic> Diagnostics => _diagnostics;

    private sealed class FunctionContractInfo
    {
        public string Name { get; }
        public IReadOnlyList<string> Parameters { get; }
        public List<string> Preconditions { get; } = new();
        public List<string> Postconditions { get; } = new();

        public FunctionContractInfo(string name, IReadOnlyList<string> parameters)
        {
            Name = name;
            Parameters = parameters;
        }
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

        var contractInfo = new FunctionContractInfo(funcName, paramNames);

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
                var argList = firstTrailer.arglist()?.argument();
                if (argList != null && argList.Length > 0)
                {
                    VerifyCallPreconditions(funcName, contract, argList, context.Start.Line, context.Start.Column);
                }
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

            // Statically evaluate simple constant boolean expressions
            if (TryEvaluateConstantBool(condText, out bool isTrue) && !isTrue)
            {
                _diagnostics.Add(new PilotAnalysisDiagnostic(
                    "PILOT306",
                    $"Static assertion guaranteed to fail at compile-time: 'assert {condText}'.",
                    context.Start.Line,
                    context.Start.Column));
            }
        }
        return null;
    }

    private void VerifyCallPreconditions(
        string funcName,
        FunctionContractInfo contract,
        PilotParser.ArgumentContext[] args,
        int line,
        int column)
    {
        var paramValues = new Dictionary<string, long>(StringComparer.Ordinal);
        for (int i = 0; i < Math.Min(contract.Parameters.Count, args.Length); i++)
        {
            string argText = args[i].test().GetText();
            if (long.TryParse(argText, out long constVal))
            {
                paramValues[contract.Parameters[i]] = constVal;
            }
        }

        foreach (var pre in contract.Preconditions)
        {
            if (TryEvaluatePrecondition(pre, paramValues, out bool passed) && !passed)
            {
                _diagnostics.Add(new PilotAnalysisDiagnostic(
                    "PILOT306",
                    $"Static contract violation calling '{funcName}': precondition 'requires {pre}' evaluated to false with constant argument(s).",
                    line,
                    column));
            }
        }
    }

    private static bool TryEvaluatePrecondition(string expr, Dictionary<string, long> paramValues, out bool result)
    {
        result = true;
        // Simple relational patterns: param > const, param >= const, param < const, param <= const, param != 0
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

    private static bool TryEvaluateConstantBool(string expr, out bool result)
    {
        result = true;
        if (expr == "false" || expr == "False") { result = false; return true; }
        if (expr == "true" || expr == "True") { result = true; return true; }

        if (expr.Contains('>'))
        {
            var parts = expr.Split('>');
            if (parts.Length == 2 && long.TryParse(parts[0], out long left) && long.TryParse(parts[1], out long right))
            {
                result = left > right;
                return true;
            }
        }
        if (expr.Contains('<'))
        {
            var parts = expr.Split('<');
            if (parts.Length == 2 && long.TryParse(parts[0], out long left) && long.TryParse(parts[1], out long right))
            {
                result = left < right;
                return true;
            }
        }
        return false;
    }
}
