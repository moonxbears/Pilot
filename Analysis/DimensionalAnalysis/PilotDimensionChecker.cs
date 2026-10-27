using System;
using System.Collections.Generic;
using System.Linq;
using Antlr4.Runtime;
using Pilot.Analysis.TypeSystem;

namespace Pilot.Analysis.DimensionalAnalysis;

/// <summary>
/// Represents the dimensional signature of a physical unit as a vector of integer exponents
/// over base dimensions (e.g. Length, Time, Mass, Current, Temperature).
/// </summary>
public sealed class UnitDimensionSignature : IEquatable<UnitDimensionSignature>
{
    private readonly Dictionary<string, int> _exponents;

    public static readonly UnitDimensionSignature Dimensionless = new(new Dictionary<string, int>());

    public UnitDimensionSignature(IDictionary<string, int>? exponents = null)
    {
        _exponents = new Dictionary<string, int>(StringComparer.OrdinalIgnoreCase);
        if (exponents != null)
        {
            foreach (var kvp in exponents)
            {
                if (kvp.Value != 0)
                {
                    _exponents[kvp.Key] = kvp.Value;
                }
            }
        }
    }

    public static UnitDimensionSignature Base(string baseDimension) =>
        new(new Dictionary<string, int> { [baseDimension] = 1 });

    public UnitDimensionSignature Multiply(UnitDimensionSignature other)
    {
        var result = new Dictionary<string, int>(_exponents, StringComparer.OrdinalIgnoreCase);
        foreach (var kvp in other._exponents)
        {
            result.TryGetValue(kvp.Key, out int current);
            int updated = current + kvp.Value;
            if (updated != 0) result[kvp.Key] = updated;
            else result.Remove(kvp.Key);
        }
        return new UnitDimensionSignature(result);
    }

    public UnitDimensionSignature Divide(UnitDimensionSignature other)
    {
        var result = new Dictionary<string, int>(_exponents, StringComparer.OrdinalIgnoreCase);
        foreach (var kvp in other._exponents)
        {
            result.TryGetValue(kvp.Key, out int current);
            int updated = current - kvp.Value;
            if (updated != 0) result[kvp.Key] = updated;
            else result.Remove(kvp.Key);
        }
        return new UnitDimensionSignature(result);
    }

    public bool Equals(UnitDimensionSignature? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        if (_exponents.Count != other._exponents.Count) return false;

        foreach (var kvp in _exponents)
        {
            if (!other._exponents.TryGetValue(kvp.Key, out int exp) || exp != kvp.Value)
                return false;
        }
        return true;
    }

    public override bool Equals(object? obj) => obj is UnitDimensionSignature sig && Equals(sig);

    public override int GetHashCode()
    {
        int hash = 17;
        foreach (var kvp in _exponents.OrderBy(k => k.Key, StringComparer.OrdinalIgnoreCase))
        {
            hash = hash * 31 + kvp.Key.ToLowerInvariant().GetHashCode();
            hash = hash * 31 + kvp.Value.GetHashCode();
        }
        return hash;
    }

    public override string ToString()
    {
        if (_exponents.Count == 0) return "1";
        return string.Join(" * ", _exponents.OrderBy(k => k.Key).Select(k => k.Value == 1 ? k.Key : $"{k.Key}^{k.Value}"));
    }
}

/// <summary>
/// Static dimension checker verifying physical units of measure algebra at compile time.
/// Prevents dimensional arithmetic violations (e.g. adding distance to time, or assigning acceleration to speed).
/// </summary>
public sealed class PilotDimensionChecker : PilotBaseVisitor<UnitDimensionSignature>
{
    private readonly Dictionary<string, UnitDimensionSignature> _unitRegistry = new(StringComparer.OrdinalIgnoreCase);
    private readonly Dictionary<string, UnitDimensionSignature> _variableDimensions = new(StringComparer.Ordinal);
    private readonly List<PilotAnalysisDiagnostic> _diagnostics = new();

    public IReadOnlyList<PilotAnalysisDiagnostic> Diagnostics => _diagnostics;

    public PilotDimensionChecker()
    {
        RegisterStandardDimensions();
    }

    protected override UnitDimensionSignature DefaultResult => UnitDimensionSignature.Dimensionless;

    protected override UnitDimensionSignature AggregateResult(UnitDimensionSignature aggregate, UnitDimensionSignature nextResult)
    {
        return nextResult != null && !nextResult.Equals(UnitDimensionSignature.Dimensionless) ? nextResult : aggregate;
    }

    public override UnitDimensionSignature VisitProgram(PilotParser.ProgramContext context)
    {
        var stmts = context.stmt();
        if (stmts != null)
        {
            foreach (var s in stmts)
            {
                Visit(s);
            }
        }
        return UnitDimensionSignature.Dimensionless;
    }

    private void RegisterStandardDimensions()
    {
        _unitRegistry["meter"] = UnitDimensionSignature.Base("Length");
        _unitRegistry["m"] = UnitDimensionSignature.Base("Length");
        _unitRegistry["second"] = UnitDimensionSignature.Base("Time");
        _unitRegistry["s"] = UnitDimensionSignature.Base("Time");
        _unitRegistry["kg"] = UnitDimensionSignature.Base("Mass");
        _unitRegistry["gram"] = UnitDimensionSignature.Base("Mass");
        _unitRegistry["speed"] = _unitRegistry["meter"].Divide(_unitRegistry["second"]);
        _unitRegistry["velocity"] = _unitRegistry["speed"];
        _unitRegistry["accel"] = _unitRegistry["speed"].Divide(_unitRegistry["second"]);
    }

    public override UnitDimensionSignature VisitUnit_stmt(PilotParser.Unit_stmtContext context)
    {
        string name = context.identifier().GetText();
        if (context.unit_expr() != null)
        {
            var derived = EvaluateUnitExpr(context.unit_expr());
            _unitRegistry[name] = derived;
            return derived;
        }

        var baseDim = UnitDimensionSignature.Base(name);
        _unitRegistry[name] = baseDim;
        return baseDim;
    }

    public override UnitDimensionSignature VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "";
        string typeSpec = context.type_annotation()?.type_spec()?.GetText() ?? "";

        UnitDimensionSignature? expectedDim = ExtractDimensionFromTypeSpec(typeSpec);
        UnitDimensionSignature actualDim = context.test() != null ? Visit(context.test()) : UnitDimensionSignature.Dimensionless;

        if (expectedDim != null && !expectedDim.Equals(UnitDimensionSignature.Dimensionless) &&
            !actualDim.Equals(UnitDimensionSignature.Dimensionless) &&
            !expectedDim.Equals(actualDim))
        {
            _diagnostics.Add(new PilotAnalysisDiagnostic(
                "PILOT305",
                $"Dimensional algebra mismatch on '{name}': declared unit '{expectedDim}' does not match expression unit '{actualDim}'.",
                context.Start.Line,
                context.Start.Column));
        }

        UnitDimensionSignature finalDim = expectedDim ?? actualDim;
        if (!string.IsNullOrEmpty(name))
        {
            _variableDimensions[name] = finalDim;
        }
        return finalDim;
    }

    public override UnitDimensionSignature VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "";
        string typeSpec = context.type_annotation()?.type_spec()?.GetText() ?? "";

        UnitDimensionSignature? expectedDim = ExtractDimensionFromTypeSpec(typeSpec);
        UnitDimensionSignature actualDim = context.test() != null ? Visit(context.test()) : UnitDimensionSignature.Dimensionless;

        if (expectedDim != null && !expectedDim.Equals(UnitDimensionSignature.Dimensionless) &&
            !actualDim.Equals(UnitDimensionSignature.Dimensionless) &&
            !expectedDim.Equals(actualDim))
        {
            _diagnostics.Add(new PilotAnalysisDiagnostic(
                "PILOT305",
                $"Dimensional algebra mismatch on '{name}': declared unit '{expectedDim}' does not match expression unit '{actualDim}'.",
                context.Start.Line,
                context.Start.Column));
        }

        UnitDimensionSignature finalDim = expectedDim ?? actualDim;
        if (!string.IsNullOrEmpty(name))
        {
            _variableDimensions[name] = finalDim;
        }
        return finalDim;
    }

    public override UnitDimensionSignature VisitTest(PilotParser.TestContext context)
    {
        var pipes = context.pipe_expr();
        if (pipes != null && pipes.Length > 0)
        {
            return Visit(pipes[0]);
        }
        return UnitDimensionSignature.Dimensionless;
    }

    public override UnitDimensionSignature VisitArith_expr(PilotParser.Arith_exprContext context)
    {
        var terms = context.term();
        if (terms == null || terms.Length == 0) return UnitDimensionSignature.Dimensionless;

        var current = Visit(terms[0]);
        for (int i = 1; i < terms.Length; i++)
        {
            var next = Visit(terms[i]);
            if (!current.Equals(UnitDimensionSignature.Dimensionless) &&
                !next.Equals(UnitDimensionSignature.Dimensionless) &&
                !current.Equals(next))
            {
                _diagnostics.Add(new PilotAnalysisDiagnostic(
                    "PILOT305",
                    $"Cannot add or subtract values with incompatible dimensions: '{current}' and '{next}'.",
                    context.Start.Line,
                    context.Start.Column));
            }
        }
        return current;
    }

    public override UnitDimensionSignature VisitTerm(PilotParser.TermContext context)
    {
        var factors = context.factor();
        if (factors == null || factors.Length == 0) return UnitDimensionSignature.Dimensionless;

        var current = Visit(factors[0]);
        for (int i = 1; i < factors.Length; i++)
        {
            var next = Visit(factors[i]);
            // Multiplication adds powers, division subtracts powers
            bool isDiv = i - 1 < context.SLASH().Length;
            current = isDiv ? current.Divide(next) : current.Multiply(next);
        }
        return current;
    }

    public override UnitDimensionSignature VisitAtom(PilotParser.AtomContext context)
    {
        if (context.identifier() != null)
        {
            string id = context.identifier().GetText();
            if (_variableDimensions.TryGetValue(id, out var dim))
            {
                return dim;
            }
            if (_unitRegistry.TryGetValue(id, out var unitDim))
            {
                return unitDim;
            }
        }

        // Literal with unit: 100.0<meter>
        string atomText = context.GetText();
        int openIdx = atomText.IndexOf('<');
        int closeIdx = atomText.LastIndexOf('>');
        if (openIdx > 0 && closeIdx > openIdx)
        {
            string unitName = atomText[(openIdx + 1)..closeIdx];
            if (_unitRegistry.TryGetValue(unitName, out var unitDim))
            {
                return unitDim;
            }
            return UnitDimensionSignature.Base(unitName);
        }

        return UnitDimensionSignature.Dimensionless;
    }

    private UnitDimensionSignature? ExtractDimensionFromTypeSpec(string typeSpec)
    {
        if (string.IsNullOrEmpty(typeSpec)) return null;
        int open = typeSpec.IndexOf('<');
        int close = typeSpec.LastIndexOf('>');
        if (open > 0 && close > open)
        {
            string unitName = typeSpec[(open + 1)..close];
            if (_unitRegistry.TryGetValue(unitName, out var dim))
            {
                return dim;
            }
            return UnitDimensionSignature.Base(unitName);
        }
        return null;
    }

    private UnitDimensionSignature EvaluateUnitExpr(PilotParser.Unit_exprContext expr)
    {
        string raw = expr.GetText();
        if (raw.Contains('/'))
        {
            var parts = raw.Split('/');
            var numerator = LookupOrRegister(parts[0].Trim());
            var denominator = LookupOrRegister(parts[1].Trim());
            return numerator.Divide(denominator);
        }
        if (raw.Contains('*'))
        {
            var parts = raw.Split('*');
            var first = LookupOrRegister(parts[0].Trim());
            var second = LookupOrRegister(parts[1].Trim());
            return first.Multiply(second);
        }
        return LookupOrRegister(raw.Trim());
    }

    private UnitDimensionSignature LookupOrRegister(string name)
    {
        if (_unitRegistry.TryGetValue(name, out var dim))
        {
            return dim;
        }
        var created = UnitDimensionSignature.Base(name);
        _unitRegistry[name] = created;
        return created;
    }
}
