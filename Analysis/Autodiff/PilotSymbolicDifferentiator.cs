using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Runtime.InteropServices;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Analysis.CallQuoting;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Analysis.Autodiff;

/// <summary>
/// Unboxed 12-byte C-ABI sequential value struct emitted for <c>ComputeImpulse!@(mass, velocity)</c>
/// (Milestone 18.2 — 0 heap allocations).
/// </summary>
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public readonly struct Dual_ComputeImpulse : IEquatable<Dual_ComputeImpulse>
{
    public readonly float Value;
    public readonly float dP_0;
    public readonly float dP_1;

    public Dual_ComputeImpulse(float value, float dP0, float dP1)
    {
        Value = value;
        dP_0 = dP0;
        dP_1 = dP1;
    }

    public bool Equals(Dual_ComputeImpulse other) =>
        Value.Equals(other.Value) && dP_0.Equals(other.dP_0) && dP_1.Equals(other.dP_1);

    public override bool Equals(object? obj) => obj is Dual_ComputeImpulse other && Equals(other);

    public override int GetHashCode() => HashCode.Combine(Value, dP_0, dP_1);

    public override string ToString() =>
        string.Create(CultureInfo.InvariantCulture, $"({Value}, {dP_0}, {dP_1})");
}

/// <summary>
/// Represents a single Common Subexpression Elimination (CSE) temporary register
/// shared across primal and partial derivative expressions in <c>fn!@(...)</c>.
/// </summary>
public sealed record PilotCseBinding(
    string RegisterName,
    PilotAstExpr Expression,
    int ReferenceCount)
{
    public override string ToString() => $"{RegisterName} = {Expression.ToSExpr()} (refs={ReferenceCount})";
}

/// <summary>
/// Represents the evaluated result and symbolic derivative DAG of Mode 6 (<c>fn!@(a, b) -&gt; Dual[TReturn, Grad_&lt;FnId&gt;]</c>).
/// </summary>
public sealed class PilotDualValue
{
    public string StructTypeName { get; }
    public string FunctionName { get; }
    public uint FnId { get; }
    public PilotValue Value { get; }
    public IReadOnlyList<string> ParameterNames { get; }
    public IReadOnlyList<PilotValue> Gradients { get; }
    public PilotAstExpr PrimalAst { get; }
    public IReadOnlyList<PilotAstExpr> RawPartialAsts { get; }
    public PilotAstExpr CsePrimalAst { get; }
    public IReadOnlyList<PilotAstExpr> CsePartialAsts { get; }
    public IReadOnlyList<PilotCseBinding> CseSchedule { get; }
    public IReadOnlyDictionary<string, PilotValue> EvaluatedCseRegisters { get; }
    public byte[] RawBytes { get; }

    public PilotDualValue(
        string structTypeName,
        string functionName,
        uint fnId,
        PilotValue value,
        IReadOnlyList<string> parameterNames,
        IReadOnlyList<PilotValue> gradients,
        PilotAstExpr primalAst,
        IReadOnlyList<PilotAstExpr> rawPartialAsts,
        PilotAstExpr csePrimalAst,
        IReadOnlyList<PilotAstExpr> csePartialAsts,
        IReadOnlyList<PilotCseBinding> cseSchedule,
        IReadOnlyDictionary<string, PilotValue> evaluatedCseRegisters,
        byte[] rawBytes)
    {
        StructTypeName = structTypeName;
        FunctionName = functionName;
        FnId = fnId;
        Value = value;
        ParameterNames = parameterNames;
        Gradients = gradients;
        PrimalAst = primalAst;
        RawPartialAsts = rawPartialAsts;
        CsePrimalAst = csePrimalAst;
        CsePartialAsts = csePartialAsts;
        CseSchedule = cseSchedule;
        EvaluatedCseRegisters = evaluatedCseRegisters;
        RawBytes = rawBytes;
    }

    public Dual_ComputeImpulse ToDualComputeImpulseStruct()
    {
        if (RawBytes.Length >= 12)
        {
            return MemoryMarshal.Read<Dual_ComputeImpulse>(RawBytes.AsSpan(0, 12));
        }

        float v = (float)Value.AsFloat();
        float d0 = Gradients.Count > 0 ? (float)Gradients[0].AsFloat() : 0f;
        float d1 = Gradients.Count > 1 ? (float)Gradients[1].AsFloat() : 0f;
        return new Dual_ComputeImpulse(v, d0, d1);
    }

    public List<PilotValue> GetTupleElements()
    {
        var list = new List<PilotValue>(1 + Gradients.Count) { Value };
        list.AddRange(Gradients);
        return list;
    }

    public PilotValue ToTupleValue() => PilotValue.FromTuple(GetTupleElements());

    public PilotObject ToPilotObject()
    {
        var cls = new PilotClassDefinition(StructTypeName);
        var obj = new PilotObject(cls)
        {
            NativePayload = this
        };

        var tupleElems = GetTupleElements();
        var fullTuple = PilotValue.FromTuple(tupleElems);
        var gradTuple = PilotValue.FromTuple(Gradients.ToList());

        obj.Fields["__dual_tuple__"] = fullTuple;
        obj.Fields["Value"] = Value;
        obj.Fields["value"] = Value;
        obj.Fields["primal"] = Value;
        obj.Fields["grad"] = gradTuple;
        obj.Fields["gradients"] = gradTuple;
        obj.Fields["tuple"] = fullTuple;
        obj.Fields["size_bytes"] = PilotValue.FromInt(RawBytes.Length);
        obj.Fields["cse_count"] = PilotValue.FromInt(CseSchedule.Count);
        obj.Fields["cse_schedule"] = PilotValue.FromList(
            CseSchedule.Select(b => PilotValue.FromString(b.ToString())).ToList());
        obj.Fields["primal_sexpr"] = PilotValue.FromString(PrimalAst.ToSExpr());
        obj.Fields["cse_primal_sexpr"] = PilotValue.FromString(CsePrimalAst.ToSExpr());
        obj.Fields["partials_sexpr"] = PilotValue.FromList(
            RawPartialAsts.Select(p => PilotValue.FromString(p.ToSExpr())).ToList());
        obj.Fields["cse_partials_sexpr"] = PilotValue.FromList(
            CsePartialAsts.Select(p => PilotValue.FromString(p.ToSExpr())).ToList());

        for (int i = 0; i < Gradients.Count; i++)
        {
            obj.Fields[$"dP_{i}"] = Gradients[i];
            obj.Fields[$"dp_{i}"] = Gradients[i];
            if (i < ParameterNames.Count)
            {
                string pName = ParameterNames[i];
                obj.Fields[$"d_{pName}"] = Gradients[i];
                obj.Fields[$"d{char.ToUpperInvariant(pName[0])}{pName[1..]}"] = Gradients[i];
            }
        }

        obj.Fields["to_tuple"] = PilotValue.FromFunction(PilotFunction.CreateNative(
            "to_tuple",
            (_, _) => fullTuple));

        return obj;
    }
}

/// <summary>
/// Compile-time and runtime Symbolic / Adjoint Differentiation + Common Subexpression Elimination (CSE)
/// engine for Mode 6 (<c>fn!@(a, b) -&gt; Dual[TReturn, Grad_&lt;FnId&gt;]</c>, Milestone 18.2).
/// </summary>
public static class PilotSymbolicDifferentiator
{
    private static readonly HashSet<string> ImpureBuiltins = new(StringComparer.OrdinalIgnoreCase)
    {
        "print", "println", "input", "read_file", "write_file", "append_file", "delete_file",
        "http_get", "http_post", "http_put", "http_delete", "exec", "shell", "spawn", "sleep"
    };

    /// <summary>
    /// Validates whether a white-box function body parse tree is pure and bounded for <c>fn!@(...)</c>.
    /// Enforces <c>QUOTE004</c> on unbounded loops or impure I/O operations.
    /// </summary>
    public static bool ValidateDifferentiableBody(IParseTree? bodyTree, out string? nonDifferentiableReason)
    {
        nonDifferentiableReason = null;
        if (bodyTree == null)
        {
            return false;
        }

        return ScanTreeForNonDifferentiableConstructs(bodyTree, out nonDifferentiableReason);
    }

    private static bool ScanTreeForNonDifferentiableConstructs(IParseTree tree, out string? reason)
    {
        reason = null;

        switch (tree)
        {
            case PilotParser.While_stmtContext whileStmt:
                if (whileStmt.BOUNDED() == null)
                {
                    reason = "unbounded 'while' loop (add 'bounded <N>' or use closed-form expression)";
                    return false;
                }
                break;

            case PilotParser.For_stmtContext forStmt:
                if (forStmt.BOUNDED() == null)
                {
                    reason = "unbounded 'for' loop without static 'bounded <N>' limit";
                    return false;
                }
                break;

            case PilotParser.Insert_stmtContext:
                reason = "impure database 'insert' statement";
                return false;

            case PilotParser.Update_stmtContext:
                reason = "impure database 'update' statement";
                return false;

            case PilotParser.Delete_stmtContext:
                reason = "impure database 'delete' statement";
                return false;

            case PilotParser.Atom_exprContext atomExpr:
            {
                var trailers = atomExpr.trailer();
                string? callee = atomExpr.atom()?.identifier()?.GetText();
                if (!string.IsNullOrEmpty(callee) &&
                    trailers != null &&
                    trailers.Length > 0 &&
                    trailers[0].LPAR() != null &&
                    ImpureBuiltins.Contains(callee))
                {
                    reason = $"impure I/O call '{callee}(...)'";
                    return false;
                }
                break;
            }
        }

        for (int i = 0; i < tree.ChildCount; i++)
        {
            if (!ScanTreeForNonDifferentiableConstructs(tree.GetChild(i), out reason))
            {
                return false;
            }
        }

        return true;
    }

    /// <summary>
    /// Evaluates <c>fn!@(args)</c>: validates <c>QUOTE001</c> (white-box AST required) and
    /// <c>QUOTE004</c> (differentiable pure/bounded body required), symbolically differentiates
    /// the function AST with respect to every parameter, runs Common Subexpression Elimination (CSE)
    /// across the primal and all partial derivatives, and evaluates the unboxed <see cref="PilotDualValue"/>.
    /// </summary>
    public static PilotDualValue EvaluateSymbolicAdjoint(
        PilotFunction? fn,
        string functionName,
        uint fnId,
        PilotParser.ArgumentContext[] args,
        ParserRuleContext callSiteCtx,
        PilotInterpreterVisitor visitor)
    {
        if (fn == null || (fn.AstBody == null && string.IsNullOrWhiteSpace(fn.SourceBodyText)))
        {
            throw new PilotRuntimeException(
                $"[QUOTE001] Cannot differentiate '{functionName}!@(...)': function is an external FFI or native symbol without a white-box Pilot AST body.",
                callSiteCtx.Start.Line,
                callSiteCtx.Start.Column);
        }

        IParseTree? bodyTree = fn.AstBody;
        if (bodyTree == null && !string.IsNullOrWhiteSpace(fn.SourceBodyText))
        {
            bodyTree = PilotCallQuotingEngine.ParseSnippetToSuite(fn.SourceBodyText!);
        }

        if (!ValidateDifferentiableBody(bodyTree, out string? nonDiffReason))
        {
            throw new PilotRuntimeException(
                $"[QUOTE004] Cannot differentiate '{functionName}!@(...)': function body contains non-differentiable construct ({nonDiffReason}).",
                callSiteCtx.Start.Line,
                callSiteCtx.Start.Column);
        }

        var paramNames = fn.Parameters.Select(p => p.Name).ToList();
        var evalArgs = new List<PilotValue>(args.Length);
        var argEnv = new Dictionary<string, PilotValue>(StringComparer.Ordinal);

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            if (a.test() == null) continue;
            var val = visitor.Visit(a.test());
            evalArgs.Add(val);
            if (a.identifier() != null && (a.EQUAL() != null || a.COLON() != null))
            {
                argEnv[a.identifier().GetText()] = val;
            }
            else if (i < paramNames.Count)
            {
                argEnv[paramNames[i]] = val;
            }
        }

        for (int i = 0; i < fn.Parameters.Count; i++)
        {
            string pName = fn.Parameters[i].Name;
            if (!argEnv.ContainsKey(pName))
            {
                argEnv[pName] = i < evalArgs.Count
                    ? evalArgs[i]
                    : fn.Parameters[i].DefaultValue ?? PilotValue.FromFloat(0.0);
            }
        }

        // Build symbolic parameter environment for primal AST extraction
        var symSubs = new Dictionary<string, PilotAstExpr>(StringComparer.Ordinal);
        foreach (var p in fn.Parameters)
        {
            symSubs[p.Name] = PilotAstExpr.Sym(p.Name, p.TypeName ?? "float");
        }

        var alphaRenames = new Dictionary<string, string>(StringComparer.Ordinal);
        var primalAst = PilotCallQuotingEngine.ExtractSymbolicBodyAst(bodyTree!, symSubs, alphaRenames).FoldConstants();

        // Compute symbolic partial derivative d(primal)/d(P_i) for each parameter P_i
        var rawPartials = new List<PilotAstExpr>(paramNames.Count);
        foreach (string pName in paramNames)
        {
            var diffAst = Differentiate(primalAst, pName).FoldConstants();
            rawPartials.Add(diffAst);
        }

        // Run Common Subexpression Elimination (CSE) across [primalAst, rawPartials...]
        var (cseSchedule, csePrimal, csePartials) = RunCommonSubexpressionElimination(primalAst, rawPartials);

        // Execute CSE schedule once into register file, then evaluate primal + partials
        var registerEnv = new Dictionary<string, PilotValue>(argEnv, StringComparer.Ordinal);
        var evaluatedCse = new Dictionary<string, PilotValue>(StringComparer.Ordinal);

        foreach (var cseBind in cseSchedule)
        {
            var regVal = EvaluateSymbolicAstWithEnv(cseBind.Expression, registerEnv, visitor.Context);
            registerEnv[cseBind.RegisterName] = regVal;
            evaluatedCse[cseBind.RegisterName] = regVal;
        }

        var primalVal = EvaluateSymbolicAstWithEnv(csePrimal, registerEnv, visitor.Context);
        var gradients = new List<PilotValue>(csePartials.Count);
        foreach (var csePartial in csePartials)
        {
            var gVal = EvaluateSymbolicAstWithEnv(csePartial, registerEnv, visitor.Context);
            gradients.Add(gVal);
        }

        // Pack into sequential unboxed float struct bytes (Value, dP_0, dP_1, ...)
        int floatCount = 1 + gradients.Count;
        byte[] rawBytes = new byte[floatCount * 4];
        float fPrimal = (float)primalVal.AsFloat();
        MemoryMarshal.Write(rawBytes.AsSpan(0, 4), in fPrimal);
        for (int i = 0; i < gradients.Count; i++)
        {
            float fGrad = (float)gradients[i].AsFloat();
            MemoryMarshal.Write(rawBytes.AsSpan((i + 1) * 4, 4), in fGrad);
        }

        return new PilotDualValue(
            $"Dual_{functionName}",
            functionName,
            fnId,
            primalVal,
            paramNames,
            gradients,
            primalAst,
            rawPartials,
            csePrimal,
            csePartials,
            cseSchedule,
            evaluatedCse,
            rawBytes);
    }

    /// <summary>
    /// Symbolically differentiates <paramref name="expr"/> with respect to parameter <paramref name="wrtVar"/>.
    /// </summary>
    public static PilotAstExpr Differentiate(PilotAstExpr expr, string wrtVar)
    {
        switch (expr.NodeKind)
        {
            case PilotAstNodeKind.Const:
                return PilotAstExpr.Const(PilotValue.FromFloat(0.0), "float");

            case PilotAstNodeKind.Sym:
                return PilotAstExpr.Const(
                    PilotValue.FromFloat(string.Equals(expr.SymbolName, wrtVar, StringComparison.Ordinal) ? 1.0 : 0.0),
                    "float");

            case PilotAstNodeKind.Unary:
            {
                var u = expr.Left!;
                var du = Differentiate(u, wrtVar);
                return expr.Operator switch
                {
                    "-" => PilotAstExpr.Unary("-", du, "float"),
                    "+" => du,
                    _ => PilotAstExpr.Const(PilotValue.FromFloat(0.0), "float")
                };
            }

            case PilotAstNodeKind.IfCond:
            {
                return PilotAstExpr.If(
                    expr.Condition!,
                    Differentiate(expr.ThenBranch!, wrtVar),
                    Differentiate(expr.ElseBranch!, wrtVar),
                    "float");
            }

            case PilotAstNodeKind.LetBind:
            {
                var substituted = PilotAstExpr.SubstituteSymbol(expr.Right!, expr.SymbolName, expr.Left!);
                return Differentiate(substituted, wrtVar);
            }

            case PilotAstNodeKind.Binary:
            {
                var u = expr.Left!;
                var v = expr.Right!;
                var du = Differentiate(u, wrtVar);
                var dv = Differentiate(v, wrtVar);

                switch (expr.Operator)
                {
                    case "+":
                        return PilotAstExpr.Add(du, dv);

                    case "-":
                        return PilotAstExpr.Sub(du, dv);

                    case "*":
                        // Product rule: d(u * v) = du * v + u * dv
                        return PilotAstExpr.Add(
                            PilotAstExpr.Mul(du, v),
                            PilotAstExpr.Mul(u, dv));

                    case "/":
                        // Quotient rule: d(u / v) = (du * v - u * dv) / (v * v)
                        return PilotAstExpr.Div(
                            PilotAstExpr.Sub(
                                PilotAstExpr.Mul(du, v),
                                PilotAstExpr.Mul(u, dv)),
                            PilotAstExpr.Mul(v, v));

                    case "**" or "^":
                    {
                        if (v.NodeKind == PilotAstNodeKind.Const)
                        {
                            double exp = v.ConstValue.AsFloat();
                            if (Math.Abs(exp) < 1e-12)
                                return PilotAstExpr.Const(PilotValue.FromFloat(0.0), "float");
                            if (Math.Abs(exp - 1.0) < 1e-12)
                                return du;
                            if (Math.Abs(exp - 2.0) < 1e-12)
                                return PilotAstExpr.Mul(PilotAstExpr.Mul(PilotAstExpr.Const(PilotValue.FromFloat(2.0)), u), du);

                            var coeff = PilotAstExpr.Const(PilotValue.FromFloat(exp), "float");
                            var newPow = PilotAstExpr.Binary("**", u, PilotAstExpr.Const(PilotValue.FromFloat(exp - 1.0), "float"), "float");
                            return PilotAstExpr.Mul(PilotAstExpr.Mul(coeff, newPow), du);
                        }

                        // General power rule: u^v * (dv * ln(u) + v * du / u)
                        var lnU = PilotAstExpr.Call("ln", new[] { u }, "float");
                        var term1 = PilotAstExpr.Mul(dv, lnU);
                        var term2 = PilotAstExpr.Div(PilotAstExpr.Mul(v, du), u);
                        return PilotAstExpr.Mul(expr, PilotAstExpr.Add(term1, term2));
                    }

                    default:
                        return PilotAstExpr.Const(PilotValue.FromFloat(0.0), "float");
                }
            }

            case PilotAstNodeKind.Call:
            {
                string rawCallee = expr.CalleeName ?? "";
                string fnName = rawCallee.Split('.')[^1].ToLowerInvariant();
                if (expr.Children.Count == 0)
                    return PilotAstExpr.Const(PilotValue.FromFloat(0.0), "float");

                var u = expr.Children[0];
                var du = Differentiate(u, wrtVar);
                var one = PilotAstExpr.Const(PilotValue.FromFloat(1.0), "float");

                return fnName switch
                {
                    "sin" => PilotAstExpr.Mul(PilotAstExpr.Call("cos", new[] { u }, "float"), du),
                    "cos" => PilotAstExpr.Mul(PilotAstExpr.Unary("-", PilotAstExpr.Call("sin", new[] { u }, "float"), "float"), du),
                    "tan" => PilotAstExpr.Div(
                        du,
                        PilotAstExpr.Mul(PilotAstExpr.Call("cos", new[] { u }, "float"), PilotAstExpr.Call("cos", new[] { u }, "float"))),
                    "asin" => PilotAstExpr.Div(
                        du,
                        PilotAstExpr.Call("sqrt", new[] { PilotAstExpr.Sub(one, PilotAstExpr.Mul(u, u)) }, "float")),
                    "acos" => PilotAstExpr.Unary("-", PilotAstExpr.Div(
                        du,
                        PilotAstExpr.Call("sqrt", new[] { PilotAstExpr.Sub(one, PilotAstExpr.Mul(u, u)) }, "float")), "float"),
                    "atan" => PilotAstExpr.Div(du, PilotAstExpr.Add(one, PilotAstExpr.Mul(u, u))),
                    "sinh" => PilotAstExpr.Mul(PilotAstExpr.Call("cosh", new[] { u }, "float"), du),
                    "cosh" => PilotAstExpr.Mul(PilotAstExpr.Call("sinh", new[] { u }, "float"), du),
                    "tanh" => PilotAstExpr.Mul(
                        PilotAstExpr.Sub(one, PilotAstExpr.Mul(PilotAstExpr.Call("tanh", new[] { u }, "float"), PilotAstExpr.Call("tanh", new[] { u }, "float"))),
                        du),
                    "exp" => PilotAstExpr.Mul(PilotAstExpr.Call("exp", new[] { u }, "float"), du),
                    "log" or "ln" => PilotAstExpr.Div(du, u),
                    "log2" => PilotAstExpr.Div(du, PilotAstExpr.Mul(u, PilotAstExpr.Const(PilotValue.FromFloat(Math.Log(2.0)), "float"))),
                    "log10" => PilotAstExpr.Div(du, PilotAstExpr.Mul(u, PilotAstExpr.Const(PilotValue.FromFloat(Math.Log(10.0)), "float"))),
                    "sqrt" => PilotAstExpr.Div(
                        du,
                        PilotAstExpr.Mul(PilotAstExpr.Const(PilotValue.FromFloat(2.0), "float"), PilotAstExpr.Call("sqrt", new[] { u }, "float"))),
                    "cbrt" => PilotAstExpr.Div(
                        du,
                        PilotAstExpr.Mul(
                            PilotAstExpr.Const(PilotValue.FromFloat(3.0), "float"),
                            PilotAstExpr.Mul(PilotAstExpr.Call("cbrt", new[] { u }, "float"), PilotAstExpr.Call("cbrt", new[] { u }, "float")))),
                    "sigmoid" => PilotAstExpr.Mul(
                        PilotAstExpr.Mul(
                            PilotAstExpr.Call("sigmoid", new[] { u }, "float"),
                            PilotAstExpr.Sub(one, PilotAstExpr.Call("sigmoid", new[] { u }, "float"))),
                        du),
                    "softplus" => PilotAstExpr.Mul(PilotAstExpr.Call("sigmoid", new[] { u }, "float"), du),
                    "abs" => PilotAstExpr.If(
                        PilotAstExpr.Binary(">=", u, PilotAstExpr.Const(PilotValue.FromFloat(0.0), "float"), "bool"),
                        du,
                        PilotAstExpr.Unary("-", du, "float"),
                        "float"),
                    "relu" => PilotAstExpr.If(
                        PilotAstExpr.Binary(">", u, PilotAstExpr.Const(PilotValue.FromFloat(0.0), "float"), "bool"),
                        du,
                        PilotAstExpr.Const(PilotValue.FromFloat(0.0), "float"),
                        "float"),
                    "pow" when expr.Children.Count >= 2 =>
                        Differentiate(PilotAstExpr.Binary("**", expr.Children[0], expr.Children[1], "float"), wrtVar),
                    _ => PilotAstExpr.Const(PilotValue.FromFloat(0.0), "float")
                };
            }

            default:
                return PilotAstExpr.Const(PilotValue.FromFloat(0.0), "float");
        }
    }

    /// <summary>
    /// Runs Common Subexpression Elimination (CSE) across the primal expression and all partial derivatives
    /// so shared compound subexpressions execute once in temporary registers (<c>_cse_0</c>, <c>_cse_1</c>, ...).
    /// </summary>
    public static (IReadOnlyList<PilotCseBinding> Schedule, PilotAstExpr CsePrimal, IReadOnlyList<PilotAstExpr> CsePartials)
        RunCommonSubexpressionElimination(PilotAstExpr primal, IReadOnlyList<PilotAstExpr> partials)
    {
        var counts = new Dictionary<string, int>(StringComparer.Ordinal);
        var canonicalNodes = new Dictionary<string, PilotAstExpr>(StringComparer.Ordinal);
        var postOrderKeys = new List<string>();

        void CountSubtrees(PilotAstExpr? node)
        {
            if (node == null) return;
            if (node.NodeKind is PilotAstNodeKind.Const or PilotAstNodeKind.Sym)
                return;

            switch (node.NodeKind)
            {
                case PilotAstNodeKind.Unary:
                    CountSubtrees(node.Left);
                    break;
                case PilotAstNodeKind.Binary:
                    CountSubtrees(node.Left);
                    CountSubtrees(node.Right);
                    break;
                case PilotAstNodeKind.IfCond:
                    CountSubtrees(node.Condition);
                    CountSubtrees(node.ThenBranch);
                    CountSubtrees(node.ElseBranch);
                    break;
                case PilotAstNodeKind.Call:
                    foreach (var c in node.Children) CountSubtrees(c);
                    break;
            }

            string key = node.ToSExpr();
            if (!counts.TryGetValue(key, out int prev))
            {
                counts[key] = 1;
                canonicalNodes[key] = node;
                postOrderKeys.Add(key);
            }
            else
            {
                counts[key] = prev + 1;
            }
        }

        CountSubtrees(primal);
        foreach (var p in partials)
        {
            CountSubtrees(p);
        }

        var keyToRegName = new Dictionary<string, string>(StringComparer.Ordinal);
        var schedule = new List<PilotCseBinding>();

        foreach (string key in postOrderKeys)
        {
            int refCount = counts[key];
            if (refCount < 2) continue;

            var rawSubtree = canonicalNodes[key];
            // Rewrite children of this subtree using already-assigned lower-level CSE registers
            var rewrittenChildrenSubtree = RewriteChildrenWithCse(rawSubtree, keyToRegName);
            string regName = $"_cse_{schedule.Count}";
            keyToRegName[key] = regName;
            schedule.Add(new PilotCseBinding(regName, rewrittenChildrenSubtree, refCount));
        }

        if (schedule.Count == 0)
        {
            return (schedule, primal, partials);
        }

        var csePrimal = RewriteTreeWithCse(primal, keyToRegName);
        var csePartials = partials.Select(p => RewriteTreeWithCse(p, keyToRegName)).ToList();
        return (schedule, csePrimal, csePartials);
    }

    private static PilotAstExpr RewriteChildrenWithCse(PilotAstExpr node, Dictionary<string, string> keyToRegName)
    {
        return node.NodeKind switch
        {
            PilotAstNodeKind.Unary => PilotAstExpr.Unary(
                node.Operator,
                RewriteTreeWithCse(node.Left!, keyToRegName),
                node.ReturnType),
            PilotAstNodeKind.Binary => PilotAstExpr.Binary(
                node.Operator,
                RewriteTreeWithCse(node.Left!, keyToRegName),
                RewriteTreeWithCse(node.Right!, keyToRegName),
                node.ReturnType),
            PilotAstNodeKind.IfCond => PilotAstExpr.If(
                RewriteTreeWithCse(node.Condition!, keyToRegName),
                RewriteTreeWithCse(node.ThenBranch!, keyToRegName),
                RewriteTreeWithCse(node.ElseBranch!, keyToRegName),
                node.ReturnType),
            PilotAstNodeKind.Call => PilotAstExpr.Call(
                node.CalleeName ?? "",
                node.Children.Select(c => RewriteTreeWithCse(c, keyToRegName)).ToList(),
                node.ReturnType),
            _ => node
        };
    }

    private static PilotAstExpr RewriteTreeWithCse(PilotAstExpr node, Dictionary<string, string> keyToRegName)
    {
        if (node.NodeKind is PilotAstNodeKind.Const or PilotAstNodeKind.Sym)
            return node;

        string key = node.ToSExpr();
        if (keyToRegName.TryGetValue(key, out string? regName))
        {
            return PilotAstExpr.Sym(regName, node.ReturnType);
        }

        return RewriteChildrenWithCse(node, keyToRegName);
    }

    public static PilotValue EvaluateSymbolicAstWithEnv(
        PilotAstExpr expr,
        Dictionary<string, PilotValue> env,
        PilotExecutionContext context)
    {
        switch (expr.NodeKind)
        {
            case PilotAstNodeKind.Const:
                return expr.ConstValue;

            case PilotAstNodeKind.Sym:
            {
                if (env.TryGetValue(expr.SymbolName, out var v))
                    return v;
                if (context.Environment.CurrentScope.TryResolve(expr.SymbolName, out var sv))
                    return sv;
                if (context.Environment.TryResolve(expr.SymbolName, out var ev))
                    return ev;
                throw new PilotRuntimeException($"Unbound symbolic variable '{expr.SymbolName}' during symbolic differentiation evaluation.");
            }

            case PilotAstNodeKind.Unary:
            {
                var inner = EvaluateSymbolicAstWithEnv(expr.Left!, env, context);
                return expr.Operator switch
                {
                    "-" => -inner,
                    "+" => inner,
                    "!" or "not" => PilotValue.FromBool(!inner.IsTrue),
                    _ => inner
                };
            }

            case PilotAstNodeKind.IfCond:
            {
                var cond = EvaluateSymbolicAstWithEnv(expr.Condition!, env, context);
                return cond.IsTrue
                    ? EvaluateSymbolicAstWithEnv(expr.ThenBranch!, env, context)
                    : EvaluateSymbolicAstWithEnv(expr.ElseBranch!, env, context);
            }

            case PilotAstNodeKind.LetBind:
            {
                var initVal = EvaluateSymbolicAstWithEnv(expr.Left!, env, context);
                var childEnv = new Dictionary<string, PilotValue>(env, StringComparer.Ordinal)
                {
                    [expr.SymbolName] = initVal
                };
                return EvaluateSymbolicAstWithEnv(expr.Right!, childEnv, context);
            }

            case PilotAstNodeKind.Binary:
            {
                var lhs = EvaluateSymbolicAstWithEnv(expr.Left!, env, context);
                var rhs = EvaluateSymbolicAstWithEnv(expr.Right!, env, context);
                return expr.Operator switch
                {
                    "+" => lhs + rhs,
                    "-" => lhs - rhs,
                    "*" => lhs * rhs,
                    "/" => lhs / rhs,
                    "%" => lhs % rhs,
                    "**" or "^" => PilotValue.FromFloat(Math.Pow(lhs.AsFloat(), rhs.AsFloat())),
                    "==" => PilotValue.FromBool(lhs.Equals(rhs)),
                    "!=" => PilotValue.FromBool(!lhs.Equals(rhs)),
                    "<" => PilotValue.FromBool(lhs < rhs),
                    "<=" => PilotValue.FromBool(lhs <= rhs),
                    ">" => PilotValue.FromBool(lhs > rhs),
                    ">=" => PilotValue.FromBool(lhs >= rhs),
                    _ => PilotValue.FromFloat(0.0)
                };
            }

            case PilotAstNodeKind.Call:
            {
                string callee = (expr.CalleeName ?? "").Split('.')[^1].ToLowerInvariant();
                var args = expr.Children.Select(c => EvaluateSymbolicAstWithEnv(c, env, context)).ToList();
                double a0 = args.Count > 0 ? args[0].AsFloat() : 0.0;
                return callee switch
                {
                    "sin" => PilotValue.FromFloat(Math.Sin(a0)),
                    "cos" => PilotValue.FromFloat(Math.Cos(a0)),
                    "tan" => PilotValue.FromFloat(Math.Tan(a0)),
                    "asin" => PilotValue.FromFloat(Math.Asin(a0)),
                    "acos" => PilotValue.FromFloat(Math.Acos(a0)),
                    "atan" => PilotValue.FromFloat(Math.Atan(a0)),
                    "atan2" when args.Count >= 2 => PilotValue.FromFloat(Math.Atan2(a0, args[1].AsFloat())),
                    "sinh" => PilotValue.FromFloat(Math.Sinh(a0)),
                    "cosh" => PilotValue.FromFloat(Math.Cosh(a0)),
                    "tanh" => PilotValue.FromFloat(Math.Tanh(a0)),
                    "exp" => PilotValue.FromFloat(Math.Exp(a0)),
                    "log" or "ln" => PilotValue.FromFloat(Math.Log(a0)),
                    "log2" => PilotValue.FromFloat(Math.Log2(a0)),
                    "log10" => PilotValue.FromFloat(Math.Log10(a0)),
                    "sqrt" => PilotValue.FromFloat(Math.Sqrt(a0)),
                    "cbrt" => PilotValue.FromFloat(Math.Cbrt(a0)),
                    "sigmoid" => PilotValue.FromFloat(1.0 / (1.0 + Math.Exp(-a0))),
                    "softplus" => PilotValue.FromFloat(Math.Log(1.0 + Math.Exp(a0))),
                    "abs" => PilotValue.FromFloat(Math.Abs(a0)),
                    "relu" => PilotValue.FromFloat(Math.Max(0.0, a0)),
                    "pow" when args.Count >= 2 => PilotValue.FromFloat(Math.Pow(a0, args[1].AsFloat())),
                    _ => context.Environment.TryResolve(expr.CalleeName ?? "", out var fnVal) && fnVal.Kind == PilotValueKind.Function
                        ? fnVal.AsFunction().Invoke(context, args)
                        : PilotValue.FromFloat(0.0)
                };
            }

            default:
                return PilotValue.FromFloat(0.0);
        }
    }
}
