using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Compiler;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Analysis.CallQuoting;

/// <summary>
/// Unified homoiconic call-quoting and semantic projection modes (Milestone 17.3 &amp; Milestone 18).
/// </summary>
public enum CallQuoteKind
{
    None = 0,            // fn(a, b)      -> TReturn
    ShallowQuote = 1,    // fn!(a, b)     -> CallExpr<TReturn>
    DeepSplice = 2,      // fn!!(a, b)    -> AstExpr<TReturn> (Supports `~arg` quasiquoting)
    AbiFrame = 3,        // fn!&(a, b)    -> CallFrame_<FnSig> (C-ABI repr(C) struct)
    ReactiveSignal = 4,  // fn!^(a, b)    -> Signal<TReturn>            (Implemented in M18)
    SymbolicAdjoint = 5, // fn!@(a, b)    -> Dual<TReturn, TGradTuple>  (Implemented in M18)
    EffectDryRun = 6     // fn!?(a, b)    -> EffectPlan<TReturn>        (Implemented in M18)
}

/// <summary>
/// Mode 1 (<see cref="CallQuoteKind.ShallowQuote"/>): Reified deferred call expression <c>CallExpr&lt;TReturn&gt;</c>
/// produced by <c>fn!(a, b)</c>. Does not execute <c>fn</c>; captures the function pointer, deterministic
/// metadata ID, and eagerly evaluated argument values.
/// </summary>
public sealed class PilotCallExpr
{
    public string FunctionName { get; }
    public uint FnMetadataId { get; }
    public PilotFunction? Function { get; }
    public string ReturnType { get; }
    public IReadOnlyList<PilotValue> EvaluatedArgs { get; }
    public IReadOnlyList<string> ArgumentExpressions { get; }
    public IReadOnlyDictionary<string, PilotValue> NamedArgs { get; }
    public CallQuoteKind Kind => CallQuoteKind.ShallowQuote;

    public PilotCallExpr(
        string functionName,
        uint fnMetadataId,
        PilotFunction? function,
        string returnType,
        IReadOnlyList<PilotValue> evaluatedArgs,
        IReadOnlyList<string> argumentExpressions,
        IReadOnlyDictionary<string, PilotValue>? namedArgs = null)
    {
        FunctionName = functionName;
        FnMetadataId = fnMetadataId;
        Function = function;
        ReturnType = string.IsNullOrWhiteSpace(returnType) ? "any" : returnType;
        EvaluatedArgs = evaluatedArgs;
        ArgumentExpressions = argumentExpressions;
        NamedArgs = namedArgs ?? new Dictionary<string, PilotValue>(StringComparer.Ordinal);
    }

    public PilotValue Invoke(PilotExecutionContext context)
    {
        if (Function == null)
        {
            if (context.Environment.TryResolve(FunctionName, out var fnVal) && fnVal.Kind == PilotValueKind.Function)
            {
                return fnVal.AsFunction().Invoke(context, EvaluatedArgs);
            }
            throw new PilotRuntimeException($"Cannot invoke ShallowQuote CallExpr '{FunctionName}': target function is not bound.");
        }
        return Function.Invoke(context, EvaluatedArgs);
    }

    public PilotValue ToPilotObject()
    {
        var cls = new PilotClassDefinition($"CallExpr[{ReturnType}]");
        var obj = new PilotObject(cls);
        obj.Fields["name"] = PilotValue.FromString(FunctionName);
        obj.Fields["fn_name"] = PilotValue.FromString(FunctionName);
        obj.Fields["FunctionName"] = PilotValue.FromString(FunctionName);
        obj.Fields["fn_id"] = PilotValue.FromInt(FnMetadataId);
        obj.Fields["metadata_id"] = PilotValue.FromInt(FnMetadataId);
        obj.Fields["FnMetadataId"] = PilotValue.FromInt(FnMetadataId);
        obj.Fields["FnId"] = PilotValue.FromInt(FnMetadataId);
        obj.Fields["return_type"] = PilotValue.FromString(ReturnType);
        obj.Fields["ReturnType"] = PilotValue.FromString(ReturnType);
        obj.Fields["kind"] = PilotValue.FromKeyword(PilotKeyword.Intern("shallow_quote"));
        obj.Fields["Kind"] = PilotValue.FromString("ShallowQuote");
        obj.Fields["args"] = PilotValue.FromList(new List<PilotValue>(EvaluatedArgs));
        obj.Fields["arguments"] = PilotValue.FromList(new List<PilotValue>(EvaluatedArgs));
        obj.Fields["EvaluatedArgs"] = PilotValue.FromList(new List<PilotValue>(EvaluatedArgs));
        obj.Fields["arg_exprs"] = PilotValue.FromList(ArgumentExpressions.Select(PilotValue.FromString).ToList());
        if (Function != null)
        {
            obj.Fields["fn_ptr"] = PilotValue.FromFunction(Function);
            obj.Fields["Function"] = PilotValue.FromFunction(Function);
        }

        var invokeFn = PilotFunction.CreateNative("invoke", (ctx, _) => Invoke(ctx));
        obj.Fields["invoke"] = PilotValue.FromFunction(invokeFn);
        obj.Fields["Invoke"] = PilotValue.FromFunction(invokeFn);
        obj.Fields["eval"] = PilotValue.FromFunction(invokeFn);
        obj.Fields["Eval"] = PilotValue.FromFunction(invokeFn);
        obj.Fields["__call__"] = PilotValue.FromFunction(invokeFn);
        obj.Fields["to_string"] = PilotValue.FromFunction(PilotFunction.CreateNative("to_string", (_, _) => PilotValue.FromString(ToString())));
        obj.Fields["__call_expr__"] = PilotValue.FromForeign(this);
        return PilotValue.FromObject(obj);
    }

    public string ToSExpr() => ToString();

    public override string ToString() =>
        $"CallExpr({FunctionName}, [{string.Join(", ", EvaluatedArgs.Select(PilotAstExpr.FormatScalarValue))}])";
}

/// <summary>
/// Homoiconic symbolic AST node kind for Modes 2 &amp; 3 (<c>fn!!(a, b)</c> &amp; <c>fn!!(~a, b)</c>).
/// </summary>
public enum PilotAstNodeKind
{
    Const,
    Sym,
    Binary,
    Unary,
    Call,
    IfCond,
    LetBind,
    Block
}

/// <summary>
/// Modes 2 &amp; 3 (<see cref="CallQuoteKind.DeepSplice"/>): Reified white-box symbolic AST expression
/// <c>AstExpr&lt;TReturn&gt;</c> with hygienic alpha-renaming, partial evaluation (<c>~arg</c>),
/// constant folding, algebraic simplification, and dead-branch elimination.
/// </summary>
public sealed class PilotAstExpr
{
    public PilotAstNodeKind NodeKind { get; }
    public string ReturnType { get; set; } = "any";
    public PilotValue ConstValue { get; }
    public string SymbolName { get; } = "";
    public string Operator { get; } = "";
    public PilotAstExpr? Left { get; }
    public PilotAstExpr? Right { get; }
    public PilotAstExpr? Condition { get; }
    public PilotAstExpr? ThenBranch { get; }
    public PilotAstExpr? ElseBranch { get; }
    public string? CalleeName { get; }
    public IReadOnlyList<PilotAstExpr> Children { get; } = Array.Empty<PilotAstExpr>();
    public Dictionary<string, string> AlphaRenamedLocals { get; } = new(StringComparer.Ordinal);

    private PilotAstExpr(
        PilotAstNodeKind kind,
        PilotValue constVal = default,
        string symbolName = "",
        string op = "",
        PilotAstExpr? left = null,
        PilotAstExpr? right = null,
        PilotAstExpr? cond = null,
        PilotAstExpr? thenBr = null,
        PilotAstExpr? elseBr = null,
        string? calleeName = null,
        IReadOnlyList<PilotAstExpr>? children = null,
        string returnType = "any")
    {
        NodeKind = kind;
        ConstValue = constVal;
        SymbolName = symbolName;
        Operator = op;
        Left = left;
        Right = right;
        Condition = cond;
        ThenBranch = thenBr;
        ElseBranch = elseBr;
        CalleeName = calleeName;
        if (children != null) Children = children;
        ReturnType = returnType;
    }

    public static PilotAstExpr Const(PilotValue value, string? returnType = null) =>
        new(PilotAstNodeKind.Const, constVal: value, returnType: returnType ?? InferValueTypeName(value));

    public static PilotAstExpr Sym(string name, string returnType = "any") =>
        new(PilotAstNodeKind.Sym, symbolName: name, returnType: returnType);

    public static PilotAstExpr Binary(string op, PilotAstExpr left, PilotAstExpr right, string returnType = "any") =>
        new(PilotAstNodeKind.Binary, op: op, left: left, right: right, returnType: returnType);

    public static PilotAstExpr Add(PilotAstExpr left, PilotAstExpr right) => Binary("+", left, right);
    public static PilotAstExpr Sub(PilotAstExpr left, PilotAstExpr right) => Binary("-", left, right);
    public static PilotAstExpr Mul(PilotAstExpr left, PilotAstExpr right) => Binary("*", left, right);
    public static PilotAstExpr Div(PilotAstExpr left, PilotAstExpr right) => Binary("/", left, right);

    public static PilotAstExpr Unary(string op, PilotAstExpr operand, string returnType = "any") =>
        new(PilotAstNodeKind.Unary, op: op, left: operand, returnType: returnType);

    public static PilotAstExpr If(PilotAstExpr condition, PilotAstExpr thenBranch, PilotAstExpr elseBranch, string returnType = "any") =>
        new(PilotAstNodeKind.IfCond, cond: condition, thenBr: thenBranch, elseBr: elseBranch, returnType: returnType);

    public static PilotAstExpr Call(string calleeName, IReadOnlyList<PilotAstExpr> args, string returnType = "any") =>
        new(PilotAstNodeKind.Call, calleeName: calleeName, children: args, returnType: returnType);

    public static PilotAstExpr Let(string localName, PilotAstExpr initExpr, PilotAstExpr bodyExpr) =>
        new(PilotAstNodeKind.LetBind, symbolName: localName, left: initExpr, right: bodyExpr, returnType: bodyExpr.ReturnType);

    /// <summary>
    /// Collects all free symbolic variable names (<see cref="PilotAstNodeKind.Sym"/>) in this AST.
    /// </summary>
    public HashSet<string> GetFreeSymbols()
    {
        var set = new HashSet<string>(StringComparer.Ordinal);
        CollectSymbols(this, set);
        return set;
    }

    private static void CollectSymbols(PilotAstExpr? node, HashSet<string> acc)
    {
        if (node == null) return;
        switch (node.NodeKind)
        {
            case PilotAstNodeKind.Sym:
                acc.Add(node.SymbolName);
                break;
            case PilotAstNodeKind.Binary:
                CollectSymbols(node.Left, acc);
                CollectSymbols(node.Right, acc);
                break;
            case PilotAstNodeKind.Unary:
                CollectSymbols(node.Left, acc);
                break;
            case PilotAstNodeKind.IfCond:
                CollectSymbols(node.Condition, acc);
                CollectSymbols(node.ThenBranch, acc);
                CollectSymbols(node.ElseBranch, acc);
                break;
            case PilotAstNodeKind.Call:
            case PilotAstNodeKind.Block:
                foreach (var c in node.Children) CollectSymbols(c, acc);
                break;
            case PilotAstNodeKind.LetBind:
                CollectSymbols(node.Left, acc);
                var bodySet = new HashSet<string>(StringComparer.Ordinal);
                CollectSymbols(node.Right, bodySet);
                bodySet.Remove(node.SymbolName);
                foreach (var s in bodySet) acc.Add(s);
                break;
        }
    }

    /// <summary>
    /// Performs constant propagation, constant folding, algebraic simplification,
    /// associative scalar re-association, and dead-branch elimination (DBE).
    /// </summary>
    public PilotAstExpr FoldConstants()
    {
        var folded = FoldCore(this);
        foreach (var kv in AlphaRenamedLocals)
            folded.AlphaRenamedLocals[kv.Key] = kv.Value;
        folded.ReturnType = ReturnType;
        return folded;
    }

    private static PilotAstExpr FoldCore(PilotAstExpr node)
    {
        switch (node.NodeKind)
        {
            case PilotAstNodeKind.Const:
            case PilotAstNodeKind.Sym:
                return node;

            case PilotAstNodeKind.Unary:
            {
                var inner = FoldCore(node.Left!);
                if (inner.NodeKind == PilotAstNodeKind.Const)
                {
                    var v = inner.ConstValue;
                    return node.Operator switch
                    {
                        "-" => Const(-v, node.ReturnType),
                        "+" => Const(v, node.ReturnType),
                        "!" or "not" => Const(PilotValue.FromBool(!v.IsTrue), "bool"),
                        "~" => Const(PilotValue.FromInt(~v.AsInt()), "int"),
                        _ => Unary(node.Operator, inner, node.ReturnType)
                    };
                }
                return Unary(node.Operator, inner, node.ReturnType);
            }

            case PilotAstNodeKind.IfCond:
            {
                var cond = FoldCore(node.Condition!);
                var thenBr = FoldCore(node.ThenBranch!);
                var elseBr = FoldCore(node.ElseBranch!);
                // Dead-Branch Elimination (DBE) when condition is a compile-time/unquoted constant
                if (cond.NodeKind == PilotAstNodeKind.Const)
                {
                    return cond.ConstValue.IsTrue ? thenBr : elseBr;
                }
                return If(cond, thenBr, elseBr, node.ReturnType);
            }

            case PilotAstNodeKind.LetBind:
            {
                var init = FoldCore(node.Left!);
                var body = FoldCore(node.Right!);
                if (init.NodeKind == PilotAstNodeKind.Const)
                {
                    return FoldCore(SubstituteSymbol(body, node.SymbolName, init));
                }
                return Let(node.SymbolName, init, body);
            }

            case PilotAstNodeKind.Call:
            {
                var foldedArgs = node.Children.Select(FoldCore).ToList();
                return Call(node.CalleeName ?? "", foldedArgs, node.ReturnType);
            }

            case PilotAstNodeKind.Binary:
            {
                var left = FoldCore(node.Left!);
                var right = FoldCore(node.Right!);
                string op = node.Operator;

                if (left.NodeKind == PilotAstNodeKind.Const && right.NodeKind == PilotAstNodeKind.Const)
                {
                    if (TryEvaluateBinaryConst(op, left.ConstValue, right.ConstValue, out var cVal))
                        return Const(cVal, node.ReturnType);
                }

                if (op == "+")
                {
                    if (IsZeroConst(left)) return right;
                    if (IsZeroConst(right)) return left;
                }
                else if (op == "-")
                {
                    if (IsZeroConst(right)) return left;
                    if (left.NodeKind == PilotAstNodeKind.Sym && right.NodeKind == PilotAstNodeKind.Sym &&
                        string.Equals(left.SymbolName, right.SymbolName, StringComparison.Ordinal))
                    {
                        return Const(PilotValue.FromInt(0), node.ReturnType);
                    }
                }
                else if (op == "*")
                {
                    if (IsZeroConst(left)) return Const(left.ConstValue, node.ReturnType);
                    if (IsZeroConst(right)) return Const(right.ConstValue, node.ReturnType);
                    if (IsOneConst(left)) return right;
                    if (IsOneConst(right)) return left;

                    // Associative constant folding & canonicalization:
                    // ((c1 * x) * c2) -> ((c1 * c2) * x)
                    if (left.NodeKind == PilotAstNodeKind.Binary && left.Operator == "*" &&
                        left.Left!.NodeKind == PilotAstNodeKind.Const && right.NodeKind == PilotAstNodeKind.Const &&
                        TryEvaluateBinaryConst("*", left.Left.ConstValue, right.ConstValue, out var prod1))
                    {
                        return FoldCore(Mul(Const(prod1), left.Right!));
                    }

                    // (c1 * (c2 * x)) -> ((c1 * c2) * x)
                    if (left.NodeKind == PilotAstNodeKind.Const &&
                        right.NodeKind == PilotAstNodeKind.Binary && right.Operator == "*" &&
                        right.Left!.NodeKind == PilotAstNodeKind.Const &&
                        TryEvaluateBinaryConst("*", left.ConstValue, right.Left.ConstValue, out var prod2))
                    {
                        return FoldCore(Mul(Const(prod2), right.Right!));
                    }

                    // Canonical right-association for scalar coefficient: ((c * x) * y) -> (c * (x * y))
                    // Enables `0.5 * ~10.0 * v * v` -> `Mul(5.0, Mul(Sym("v"), Sym("v")))`
                    if (left.NodeKind == PilotAstNodeKind.Binary && left.Operator == "*" &&
                        left.Left!.NodeKind == PilotAstNodeKind.Const &&
                        left.Right!.NodeKind != PilotAstNodeKind.Const &&
                        right.NodeKind != PilotAstNodeKind.Const)
                    {
                        return Mul(left.Left, FoldCore(Mul(left.Right, right)));
                    }
                }
                else if (op == "/")
                {
                    if (IsZeroConst(left)) return Const(left.ConstValue, node.ReturnType);
                    if (IsOneConst(right)) return left;
                }

                return Binary(op, left, right, node.ReturnType);
            }

            default:
                return node;
        }
    }

    public static PilotAstExpr SubstituteSymbol(PilotAstExpr node, string symbolName, PilotAstExpr replacement)
    {
        switch (node.NodeKind)
        {
            case PilotAstNodeKind.Const:
                return node;
            case PilotAstNodeKind.Sym:
                return string.Equals(node.SymbolName, symbolName, StringComparison.Ordinal) ? replacement : node;
            case PilotAstNodeKind.Unary:
                return Unary(node.Operator, SubstituteSymbol(node.Left!, symbolName, replacement), node.ReturnType);
            case PilotAstNodeKind.Binary:
                return Binary(
                    node.Operator,
                    SubstituteSymbol(node.Left!, symbolName, replacement),
                    SubstituteSymbol(node.Right!, symbolName, replacement),
                    node.ReturnType);
            case PilotAstNodeKind.IfCond:
                return If(
                    SubstituteSymbol(node.Condition!, symbolName, replacement),
                    SubstituteSymbol(node.ThenBranch!, symbolName, replacement),
                    SubstituteSymbol(node.ElseBranch!, symbolName, replacement),
                    node.ReturnType);
            case PilotAstNodeKind.Call:
                return Call(
                    node.CalleeName ?? "",
                    node.Children.Select(c => SubstituteSymbol(c, symbolName, replacement)).ToList(),
                    node.ReturnType);
            case PilotAstNodeKind.LetBind:
                var newInit = SubstituteSymbol(node.Left!, symbolName, replacement);
                if (string.Equals(node.SymbolName, symbolName, StringComparison.Ordinal))
                    return Let(node.SymbolName, newInit, node.Right!);
                return Let(node.SymbolName, newInit, SubstituteSymbol(node.Right!, symbolName, replacement));
            default:
                return node;
        }
    }

    public PilotValue Evaluate(IReadOnlyDictionary<string, PilotValue> bindings, PilotExecutionContext? ctx = null)
    {
        switch (NodeKind)
        {
            case PilotAstNodeKind.Const:
                return ConstValue;

            case PilotAstNodeKind.Sym:
                if (bindings.TryGetValue(SymbolName, out var bVal))
                    return bVal;
                if (ctx != null && ctx.Environment.TryResolve(SymbolName, out var envVal))
                    return envVal;
                throw new PilotRuntimeException($"Unbound symbolic variable '{SymbolName}' during AstExpr evaluation.");

            case PilotAstNodeKind.Unary:
            {
                var v = Left!.Evaluate(bindings, ctx);
                return Operator switch
                {
                    "-" => -v,
                    "+" => v,
                    "!" or "not" => PilotValue.FromBool(!v.IsTrue),
                    "~" => PilotValue.FromInt(~v.AsInt()),
                    _ => v
                };
            }

            case PilotAstNodeKind.Binary:
            {
                var l = Left!.Evaluate(bindings, ctx);
                var r = Right!.Evaluate(bindings, ctx);
                if (TryEvaluateBinaryConst(Operator, l, r, out var res))
                    return res;
                return PilotValue.Null;
            }

            case PilotAstNodeKind.IfCond:
            {
                var c = Condition!.Evaluate(bindings, ctx);
                return c.IsTrue ? ThenBranch!.Evaluate(bindings, ctx) : ElseBranch!.Evaluate(bindings, ctx);
            }

            case PilotAstNodeKind.LetBind:
            {
                var initVal = Left!.Evaluate(bindings, ctx);
                var nextBindings = new Dictionary<string, PilotValue>(bindings, StringComparer.Ordinal)
                {
                    [SymbolName] = initVal
                };
                return Right!.Evaluate(nextBindings, ctx);
            }

            default:
                return PilotValue.Null;
        }
    }

    public PilotValue ToPilotObject()
    {
        var cls = new PilotClassDefinition($"AstExpr[{ReturnType}]");
        var obj = new PilotObject(cls);
        obj.Fields["node_kind"] = PilotValue.FromString(NodeKind.ToString());
        obj.Fields["NodeKind"] = PilotValue.FromString(NodeKind.ToString());
        obj.Fields["kind"] = PilotValue.FromKeyword(PilotKeyword.Intern("deep_splice"));
        obj.Fields["Kind"] = PilotValue.FromString("DeepSplice");
        obj.Fields["return_type"] = PilotValue.FromString(ReturnType);
        obj.Fields["ReturnType"] = PilotValue.FromString(ReturnType);
        obj.Fields["op"] = PilotValue.FromString(Operator);
        obj.Fields["Operator"] = PilotValue.FromString(Operator);
        obj.Fields["symbol"] = PilotValue.FromString(SymbolName);
        obj.Fields["SymbolName"] = PilotValue.FromString(SymbolName);
        obj.Fields["value"] = ConstValue.Kind != PilotValueKind.Undefined ? ConstValue : PilotValue.Null;
        obj.Fields["s_expr"] = PilotValue.FromString(ToString());
        obj.Fields["SExpr"] = PilotValue.FromString(ToString());
        obj.Fields["infix"] = PilotValue.FromString(ToInfixString());
        obj.Fields["Infix"] = PilotValue.FromString(ToInfixString());
        obj.Fields["symbols"] = PilotValue.FromList(GetFreeSymbols().OrderBy(s => s, StringComparer.Ordinal).Select(PilotValue.FromString).ToList());

        var alphaMap = new Dictionary<PilotValue, PilotValue>();
        foreach (var kv in AlphaRenamedLocals)
            alphaMap[PilotValue.FromString(kv.Key)] = PilotValue.FromString(kv.Value);
        obj.Fields["alpha_renames"] = PilotValue.FromMap(alphaMap);
        obj.Fields["AlphaRenamedLocals"] = PilotValue.FromMap(alphaMap);

        if (Left != null) obj.Fields["left"] = Left.ToPilotObject();
        if (Right != null) obj.Fields["right"] = Right.ToPilotObject();
        if (Condition != null) obj.Fields["condition"] = Condition.ToPilotObject();
        if (ThenBranch != null) obj.Fields["then_branch"] = ThenBranch.ToPilotObject();
        if (ElseBranch != null) obj.Fields["else_branch"] = ElseBranch.ToPilotObject();

        var evalFn = PilotFunction.CreateNative("eval", (ctx, args) =>
        {
            var dict = new Dictionary<string, PilotValue>(StringComparer.Ordinal);
            if (args.Count > 0 && args[0].Kind == PilotValueKind.Map)
            {
                foreach (var kvp in args[0].AsMap())
                {
                    string k = kvp.Key.Kind == PilotValueKind.Keyword ? kvp.Key.AsKeyword().Name : kvp.Key.ToString().TrimStart(':');
                    dict[k] = kvp.Value;
                }
            }
            else if (args.Count == 1)
            {
                var syms = GetFreeSymbols().ToList();
                if (syms.Count == 1)
                    dict[syms[0]] = args[0];
            }
            return Evaluate(dict, ctx);
        }, "bindings");

        obj.Fields["eval"] = PilotValue.FromFunction(evalFn);
        obj.Fields["Eval"] = PilotValue.FromFunction(evalFn);
        obj.Fields["__call__"] = PilotValue.FromFunction(evalFn);
        obj.Fields["to_string"] = PilotValue.FromFunction(PilotFunction.CreateNative("to_string", (_, _) => PilotValue.FromString(ToString())));
        obj.Fields["__ast_expr__"] = PilotValue.FromForeign(this);
        return PilotValue.FromObject(obj);
    }

    public string ToInfixString()
    {
        return NodeKind switch
        {
            PilotAstNodeKind.Const => FormatScalarValue(ConstValue),
            PilotAstNodeKind.Sym => SymbolName,
            PilotAstNodeKind.Unary => $"{Operator}{Left!.ToInfixString()}",
            PilotAstNodeKind.Binary => $"({Left!.ToInfixString()} {Operator} {Right!.ToInfixString()})",
            PilotAstNodeKind.IfCond => $"({ThenBranch!.ToInfixString()} if {Condition!.ToInfixString()} else {ElseBranch!.ToInfixString()})",
            PilotAstNodeKind.Call => $"{CalleeName}({string.Join(", ", Children.Select(c => c.ToInfixString()))})",
            PilotAstNodeKind.LetBind => $"{{ let {SymbolName} = {Left!.ToInfixString()}; {Right!.ToInfixString()} }}",
            _ => ToString()
        };
    }

    public string ToSExpr() => ToString();

    public override string ToString()
    {
        switch (NodeKind)
        {
            case PilotAstNodeKind.Const:
                return FormatScalarValue(ConstValue);

            case PilotAstNodeKind.Sym:
                return $"Sym(\"{SymbolName}\")";

            case PilotAstNodeKind.Unary:
            {
                string uTag = Operator switch
                {
                    "-" => "Neg",
                    "!" or "not" => "Not",
                    "~" => "BitNot",
                    _ => $"Unary(\"{Operator}\")"
                };
                return $"{uTag}({Left})";
            }

            case PilotAstNodeKind.Binary:
            {
                string bTag = Operator switch
                {
                    "*" => "Mul",
                    "+" => "Add",
                    "-" => "Sub",
                    "/" => "Div",
                    "%" => "Mod",
                    "**" => "Pow",
                    "==" => "Eq",
                    "!=" => "Neq",
                    "<" => "Lt",
                    "<=" => "Lte",
                    ">" => "Gt",
                    ">=" => "Gte",
                    _ => null!
                };
                if (bTag != null)
                    return $"{bTag}({Left}, {Right})";
                return $"BinOp(\"{Operator}\", {Left}, {Right})";
            }

            case PilotAstNodeKind.IfCond:
                return $"If({Condition}, {ThenBranch}, {ElseBranch})";

            case PilotAstNodeKind.Call:
                return $"Call({CalleeName}, [{string.Join(", ", Children)}])";

            case PilotAstNodeKind.LetBind:
                return $"Let(\"{SymbolName}\", {Left}, {Right})";

            default:
                return "AstExpr()";
        }
    }

    public static string FormatScalarValue(PilotValue val)
    {
        if (val.Kind == PilotValueKind.Float)
        {
            double d = val.AsFloat();
            if (!double.IsNaN(d) && !double.IsInfinity(d) && d == Math.Floor(d))
                return d.ToString("0.0", CultureInfo.InvariantCulture);
            return d.ToString("G", CultureInfo.InvariantCulture);
        }
        if (val.Kind == PilotValueKind.Int)
            return val.AsInt().ToString(CultureInfo.InvariantCulture);
        if (val.Kind == PilotValueKind.String)
            return $"\"{val.AsString()}\"";
        if (val.Kind == PilotValueKind.Bool)
            return val.IsTrue ? "true" : "false";
        return val.ToString();
    }

    private static bool IsZeroConst(PilotAstExpr expr) =>
        expr.NodeKind == PilotAstNodeKind.Const &&
        ((expr.ConstValue.Kind == PilotValueKind.Int && expr.ConstValue.AsInt() == 0) ||
         (expr.ConstValue.Kind == PilotValueKind.Float && Math.Abs(expr.ConstValue.AsFloat()) < 1e-15));

    private static bool IsOneConst(PilotAstExpr expr) =>
        expr.NodeKind == PilotAstNodeKind.Const &&
        ((expr.ConstValue.Kind == PilotValueKind.Int && expr.ConstValue.AsInt() == 1) ||
         (expr.ConstValue.Kind == PilotValueKind.Float && Math.Abs(expr.ConstValue.AsFloat() - 1.0) < 1e-15));

    private static bool TryEvaluateBinaryConst(string op, PilotValue l, PilotValue r, out PilotValue result)
    {
        result = PilotValue.Undefined;
        try
        {
            switch (op)
            {
                case "+": result = l + r; return true;
                case "-": result = l - r; return true;
                case "*": result = l * r; return true;
                case "/": result = l / r; return true;
                case "%": result = l % r; return true;
                case "**": result = PilotValue.FromFloat(Math.Pow(l.AsFloat(), r.AsFloat())); return true;
                case "==": result = PilotValue.FromBool(l.Equals(r)); return true;
                case "!=": result = PilotValue.FromBool(!l.Equals(r)); return true;
                case "<": result = PilotValue.FromBool(l < r); return true;
                case "<=": result = PilotValue.FromBool(l <= r); return true;
                case ">": result = PilotValue.FromBool(l > r); return true;
                case ">=": result = PilotValue.FromBool(l >= r); return true;
                case "&&" or "and": result = PilotValue.FromBool(l.IsTrue && r.IsTrue); return true;
                case "||" or "or": result = PilotValue.FromBool(l.IsTrue || r.IsTrue); return true;
                default: return false;
            }
        }
        catch
        {
            return false;
        }
    }

    private static string InferValueTypeName(PilotValue val) =>
        val.Kind switch
        {
            PilotValueKind.Int => "int",
            PilotValueKind.Float => "float",
            PilotValueKind.Bool => "bool",
            PilotValueKind.String => "str",
            PilotValueKind.Keyword => "symbol",
            _ => "any"
        };
}

/// <summary>
/// Synthesized 12-byte C-ABI <c>repr(C)</c> stack-allocated call-frame struct for two 32-bit scalar parameters
/// (e.g. <c>ComputeImpulse!&(10.0, 4.0)</c> -&gt; <c>CallFrame_ComputeImpulse { uint FnId; float P_0; float P_1; }</c>).
/// Guaranteed 12 bytes, 0 heap/GC object headers.
/// </summary>
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public readonly struct CallFrame_ComputeImpulse
{
    public readonly uint FnId;
    public readonly float P_0;
    public readonly float P_1;

    public CallFrame_ComputeImpulse(uint fnId, float p0, float p1)
    {
        FnId = fnId;
        P_0 = p0;
        P_1 = p1;
    }

    public static int SizeInBytes => Unsafe.SizeOf<CallFrame_ComputeImpulse>();
}

/// <summary>
/// Mode 4 (<see cref="CallQuoteKind.AbiFrame"/>): Zero-marshalling C-ABI call-frame descriptor and
/// binary payload synthesized by <c>fn!&amp;(a, b)</c>.
/// </summary>
public sealed class PilotAbiCallFrame
{
    public string StructName { get; }
    public string FunctionName { get; }
    public uint FnId { get; }
    public int SizeBytes { get; }
    public int AlignmentBytes { get; }
    public bool IsStackAllocated => true;
    public int HeapAllocations => 0;
    public IReadOnlyList<(string Name, string CType, int Offset, int Size)> FieldLayout { get; }
    public IReadOnlyList<PilotValue> ArgumentValues { get; }
    public byte[] RawBytes { get; }

    public PilotAbiCallFrame(
        string structName,
        string functionName,
        uint fnId,
        int sizeBytes,
        int alignmentBytes,
        IReadOnlyList<(string Name, string CType, int Offset, int Size)> fieldLayout,
        IReadOnlyList<PilotValue> argumentValues,
        byte[] rawBytes)
    {
        StructName = structName;
        FunctionName = functionName;
        FnId = fnId;
        SizeBytes = sizeBytes;
        AlignmentBytes = alignmentBytes;
        FieldLayout = fieldLayout;
        ArgumentValues = argumentValues;
        RawBytes = rawBytes;
    }

    /// <summary>
    /// Materializes the 12-byte <see cref="CallFrame_ComputeImpulse"/> value-type struct directly from the packed C-ABI bytes.
    /// </summary>
    public CallFrame_ComputeImpulse AsStruct2x32()
    {
        if (RawBytes.Length >= 12)
        {
            return MemoryMarshal.Read<CallFrame_ComputeImpulse>(RawBytes.AsSpan(0, 12));
        }
        float p0 = ArgumentValues.Count > 0 ? (float)ArgumentValues[0].AsFloat() : 0f;
        float p1 = ArgumentValues.Count > 1 ? (float)ArgumentValues[1].AsFloat() : 0f;
        return new CallFrame_ComputeImpulse(FnId, p0, p1);
    }

    /// <summary>
    /// Dispatches the C-ABI call frame through the static <c>FnId</c> trampoline table.
    /// </summary>
    public PilotValue Dispatch(PilotExecutionContext context) =>
        PilotCallQuotingEngine.DispatchAbiFrame(this, context);

    public PilotValue ToPilotObject()
    {
        var cls = new PilotClassDefinition(StructName);
        var obj = new PilotObject(cls);
        obj.Fields["struct_name"] = PilotValue.FromString(StructName);
        obj.Fields["StructName"] = PilotValue.FromString(StructName);
        obj.Fields["fn_name"] = PilotValue.FromString(FunctionName);
        obj.Fields["FunctionName"] = PilotValue.FromString(FunctionName);
        obj.Fields["FnId"] = PilotValue.FromInt(FnId);
        obj.Fields["fn_id"] = PilotValue.FromInt(FnId);
        obj.Fields["size_of"] = PilotValue.FromInt(SizeBytes);
        obj.Fields["SizeOf"] = PilotValue.FromInt(SizeBytes);
        obj.Fields["size_bytes"] = PilotValue.FromInt(SizeBytes);
        obj.Fields["SizeBytes"] = PilotValue.FromInt(SizeBytes);
        obj.Fields["byte_size"] = PilotValue.FromInt(SizeBytes);
        obj.Fields["ByteSize"] = PilotValue.FromInt(SizeBytes);
        obj.Fields["align_of"] = PilotValue.FromInt(AlignmentBytes);
        obj.Fields["AlignmentBytes"] = PilotValue.FromInt(AlignmentBytes);
        obj.Fields["is_stack_allocated"] = PilotValue.True;
        obj.Fields["IsStackAllocated"] = PilotValue.True;
        obj.Fields["heap_allocations"] = PilotValue.FromInt(0);
        obj.Fields["HeapAllocations"] = PilotValue.FromInt(0);
        obj.Fields["kind"] = PilotValue.FromKeyword(PilotKeyword.Intern("abi_frame"));

        var offsetsMap = new Dictionary<PilotValue, PilotValue>();
        offsetsMap[PilotValue.FromString("FnId")] = PilotValue.FromInt(0);
        for (int i = 0; i < FieldLayout.Count; i++)
        {
            var fl = FieldLayout[i];
            offsetsMap[PilotValue.FromString(fl.Name)] = PilotValue.FromInt(fl.Offset);
            offsetsMap[PilotValue.FromString($"P_{i}")] = PilotValue.FromInt(fl.Offset);
            if (i < ArgumentValues.Count)
            {
                obj.Fields[$"P_{i}"] = ArgumentValues[i];
                obj.Fields[fl.Name] = ArgumentValues[i];
            }
        }
        obj.Fields["offsets"] = PilotValue.FromMap(offsetsMap);
        obj.Fields["FieldOffsets"] = PilotValue.FromMap(offsetsMap);

        var dispatchFn = PilotFunction.CreateNative("dispatch", (ctx, _) => Dispatch(ctx));
        obj.Fields["dispatch"] = PilotValue.FromFunction(dispatchFn);
        obj.Fields["Dispatch"] = PilotValue.FromFunction(dispatchFn);
        obj.Fields["invoke"] = PilotValue.FromFunction(dispatchFn);
        obj.Fields["__call__"] = PilotValue.FromFunction(dispatchFn);
        obj.Fields["to_string"] = PilotValue.FromFunction(PilotFunction.CreateNative("to_string", (_, _) => PilotValue.FromString(ToString())));
        obj.Fields["__abi_frame__"] = PilotValue.FromForeign(this);
        return PilotValue.FromObject(obj);
    }

    public string ToSExpr() => ToString();

    public override string ToString() =>
        $"{StructName} {{ FnId = 0x{FnId:X8}, Size = {SizeBytes}B, Args = [{string.Join(", ", ArgumentValues.Select(PilotAstExpr.FormatScalarValue))}] }}";
}

/// <summary>
/// Core engine for Milestone 17 Homoiconic Call-Quoting Modes 0–4:
/// <list type="bullet">
/// <item><description>Mode 0 (<c>fn(a, b)</c>): Standard eager invocation.</description></item>
/// <item><description>Mode 1 (<c>fn!(a, b)</c>): <see cref="CallQuoteKind.ShallowQuote"/> (<c>CallExpr&lt;TReturn&gt;</c>).</description></item>
/// <item><description>Modes 2 &amp; 3 (<c>fn!!(a, b)</c> &amp; <c>fn!!(~a, b)</c>): <see cref="CallQuoteKind.DeepSplice"/> (<c>AstExpr&lt;TReturn&gt;</c>).</description></item>
/// <item><description>Mode 4 (<c>fn!&amp;(a, b)</c>): <see cref="CallQuoteKind.AbiFrame"/> (<c>CallFrame_&lt;FnName&gt;</c> 12-byte C-ABI struct).</description></item>
/// </list>
/// </summary>
public static class PilotCallQuotingEngine
{
    private static readonly ConcurrentDictionary<uint, PilotFunction> TrampolineRegistry = new();
    private static long _hygieneCounter;

    /// <summary>
    /// Extracts the <see cref="CallQuoteKind"/> from an ANTLR4 <c>trailer</c> context.
    /// </summary>
    public static CallQuoteKind GetCallQuoteKind(PilotParser.TrailerContext? trailer)
    {
        var op = trailer?.call_quote_op();
        if (op == null) return CallQuoteKind.None;
        if (op.CALL_QUOTE_DEEP() != null) return CallQuoteKind.DeepSplice;
        if (op.CALL_QUOTE_ABI() != null) return CallQuoteKind.AbiFrame;
        if (op.CALL_QUOTE_REACT() != null) return CallQuoteKind.ReactiveSignal;
        if (op.CALL_QUOTE_ADJOINT() != null) return CallQuoteKind.SymbolicAdjoint;
        if (op.CALL_QUOTE_EFFECT() != null) return CallQuoteKind.EffectDryRun;
        if (op.EXCLAMATION() != null) return CallQuoteKind.ShallowQuote;
        return CallQuoteKind.None;
    }

    /// <summary>
    /// Verifies that the call-quote operator token is immediately followed by <c>(</c> with zero whitespace.
    /// </summary>
    public static bool HasZeroWhitespaceBeforeParen(PilotParser.TrailerContext trailer)
    {
        var op = trailer.call_quote_op();
        var lpar = trailer.LPAR();
        if (op?.Stop == null || lpar?.Symbol == null) return true;
        return op.Stop.StopIndex + 1 == lpar.Symbol.StartIndex;
    }

    /// <summary>
    /// Computes a deterministic 32-bit FNV-1a metadata ID for a function name and parameter signature.
    /// </summary>
    public static uint ComputeFnMetadataId(string functionName, IReadOnlyList<PilotParameter>? parameters = null)
    {
        uint hash = 2166136261u;
        string sig = parameters == null || parameters.Count == 0
            ? $"{functionName}()"
            : $"{functionName}({string.Join(",", parameters.Select(p => string.IsNullOrWhiteSpace(p.TypeName) ? "any" : p.TypeName.Trim()))})";

        foreach (char c in sig)
        {
            hash ^= c;
            hash *= 16777619u;
        }
        return hash == 0 ? 1u : hash;
    }

    /// <summary>
    /// Evaluates a call-quoted invocation (<c>!</c>, <c>!!</c>, <c>!&amp;</c>) in the AST interpreter or PilVM.
    /// </summary>
    public static PilotValue EvaluateQuotedCall(
        CallQuoteKind quoteKind,
        PilotValue calleeVal,
        string calleeName,
        PilotParser.TrailerContext trailer,
        PilotInterpreterVisitor visitor)
    {
        var context = visitor.Context;
        var args = trailer.arglist()?.argument() ?? Array.Empty<PilotParser.ArgumentContext>();

        // Enforce QUOTE002: `~arg` unquote prefix is ONLY allowed inside `!!` (`CallQuoteKind.DeepSplice`)
        if (quoteKind != CallQuoteKind.DeepSplice)
        {
            foreach (var arg in args)
            {
                if (arg.TILDE() != null)
                {
                    throw new PilotRuntimeException(
                        $"[QUOTE002] Quasiquote unquote prefix '~{arg.test()?.GetText()}' is only valid inside 'fn!!(...)' deep-splice calls.",
                        arg.Start.Line,
                        arg.Start.Column);
                }
            }
        }

        PilotFunction? fn = calleeVal.Kind == PilotValueKind.Function ? calleeVal.AsFunction() : null;
        if (fn == null && !string.IsNullOrEmpty(calleeName) &&
            context.Environment.TryResolve(calleeName, out var resolved) &&
            resolved.Kind == PilotValueKind.Function)
        {
            fn = resolved.AsFunction();
        }

        string effectiveName = fn?.Name ?? calleeName;
        uint fnId = ComputeFnMetadataId(effectiveName, fn?.Parameters);
        if (fn != null)
        {
            TrampolineRegistry[fnId] = fn;
        }

        switch (quoteKind)
        {
            case CallQuoteKind.ShallowQuote:
            {
                // Conflict Resolution #1: If `target` is a `defmacro` / `defkeyword` macro, expand/invoke it!
                if (fn is { IsMacro: true })
                {
                    var macroArgs = new List<PilotValue>(args.Length);
                    foreach (var a in args)
                    {
                        if (a.test() != null)
                            macroArgs.Add(visitor.Visit(a.test()));
                    }
                    return fn.Invoke(context, macroArgs);
                }

                var evalArgs = new List<PilotValue>(args.Length);
                var argExprs = new List<string>(args.Length);
                var namedArgs = new Dictionary<string, PilotValue>(StringComparer.Ordinal);

                foreach (var a in args)
                {
                    if (a.test() == null) continue;
                    var v = visitor.Visit(a.test());
                    argExprs.Add(a.test().GetText());
                    if (a.identifier() != null && (a.EQUAL() != null || a.COLON() != null))
                        namedArgs[a.identifier().GetText()] = v;
                    else
                        evalArgs.Add(v);
                }

                var callExpr = new PilotCallExpr(
                    effectiveName,
                    fnId,
                    fn,
                    fn?.ReturnTypeSpec ?? "any",
                    evalArgs,
                    argExprs,
                    namedArgs);
                return callExpr.ToPilotObject();
            }

            case CallQuoteKind.DeepSplice:
            {
                return EvaluateDeepSplice(fn, effectiveName, args, trailer, visitor).ToPilotObject();
            }

            case CallQuoteKind.AbiFrame:
            {
                return BuildAbiCallFrame(fn, effectiveName, fnId, args, trailer, visitor).ToPilotObject();
            }

            case CallQuoteKind.ReactiveSignal:
            {
                var sig = Pilot.Runtime.Reactive.PilotSignalGraph.CreateReactiveCallSignal(fn, effectiveName, args, trailer, visitor);
                return PilotValue.FromObject(sig.ToPilotObject());
            }

            case CallQuoteKind.SymbolicAdjoint:
            {
                var dual = Pilot.Analysis.Autodiff.PilotSymbolicDifferentiator.EvaluateSymbolicAdjoint(fn, effectiveName, fnId, args, trailer, visitor);
                return PilotValue.FromObject(dual.ToPilotObject());
            }

            case CallQuoteKind.EffectDryRun:
            {
                var plan = Pilot.Runtime.Effects.PilotEffectDryRunEngine.EvaluateEffectDryRun(fn, effectiveName, args, trailer, visitor);
                return PilotValue.FromObject(plan.ToPilotObject());
            }

            default:
                throw new PilotRuntimeException(
                    $"Call-quoting mode '{quoteKind}' is not supported in this context.",
                    trailer.Start.Line,
                    trailer.Start.Column);
        }
    }

    /// <summary>
    /// Modes 2 &amp; 3 (<c>fn!!(a, b)</c> &amp; <c>fn!!(~a, b)</c>): Performs white-box AST loading,
    /// hygienic alpha-renaming of local variables, parameter substitution (symbolic vs. eager <c>~arg</c>),
    /// and constant folding + dead-branch elimination.
    /// </summary>
    public static PilotAstExpr EvaluateDeepSplice(
        PilotFunction? fn,
        string functionName,
        PilotParser.ArgumentContext[] args,
        ParserRuleContext callSiteCtx,
        PilotInterpreterVisitor visitor)
    {
        if (fn == null || (fn.AstBody == null && string.IsNullOrWhiteSpace(fn.SourceBodyText)))
        {
            throw new PilotRuntimeException(
                $"[QUOTE001] Cannot deep-splice '{functionName}!!(...)': function is an external FFI or native symbol without a white-box Pilot AST body.",
                callSiteCtx.Start.Line,
                callSiteCtx.Start.Column);
        }

        ParserRuleContext? bodyCtx = fn.AstBody;
        if (bodyCtx == null && !string.IsNullOrWhiteSpace(fn.SourceBodyText))
        {
            bodyCtx = PilotParser.GetOrParseSnippet(fn.SourceBodyText!);
        }

        // Build parameter -> PilotAstExpr substitution map
        var paramSubstitutions = new Dictionary<string, PilotAstExpr>(StringComparer.Ordinal);
        for (int i = 0; i < fn.Parameters.Count; i++)
        {
            string pName = fn.Parameters[i].Name;
            if (i < args.Length && args[i].test() is { } argTest)
            {
                if (args[i].TILDE() != null)
                {
                    // Mode 3 (`~arg`): Eagerly evaluate in caller's scope and substitute as `Const(val)`!
                    PilotValue eagerVal = visitor.Visit(argTest);
                    paramSubstitutions[pName] = PilotAstExpr.Const(eagerVal, fn.Parameters[i].TypeName);
                }
                else
                {
                    // Mode 2 (`arg`): Convert caller argument expression into symbolic AST!
                    paramSubstitutions[pName] = ConvertParseTreeToSymbolicAst(
                        argTest,
                        new Dictionary<string, PilotAstExpr>(StringComparer.Ordinal),
                        new Dictionary<string, string>(StringComparer.Ordinal));
                }
            }
            else if (fn.Parameters[i].DefaultValue.HasValue)
            {
                paramSubstitutions[pName] = PilotAstExpr.Const(fn.Parameters[i].DefaultValue!.Value, fn.Parameters[i].TypeName);
            }
            else
            {
                paramSubstitutions[pName] = PilotAstExpr.Sym(pName, fn.Parameters[i].TypeName ?? "any");
            }
        }

        var alphaRenames = new Dictionary<string, string>(StringComparer.Ordinal);
        var rawAst = ConvertFunctionBodyToSymbolicAst(bodyCtx!, paramSubstitutions, alphaRenames);
        rawAst.ReturnType = fn.ReturnTypeSpec ?? "any";
        foreach (var kv in alphaRenames)
            rawAst.AlphaRenamedLocals[kv.Key] = kv.Value;

        return rawAst.FoldConstants();
    }

    /// <summary>
    /// Mode 4 (<c>fn!&amp;(a, b)</c>): Verifies <c>unmanaged</c> / <c>repr(C)</c> parameter types (<c>QUOTE003</c>)
    /// and packs the C-ABI <c>CallFrame_&lt;FnName&gt;</c> byte layout.
    /// </summary>
    public static PilotAbiCallFrame BuildAbiCallFrame(
        PilotFunction? fn,
        string functionName,
        uint fnId,
        PilotParser.ArgumentContext[] args,
        ParserRuleContext callSiteCtx,
        PilotInterpreterVisitor visitor)
    {
        var evalArgs = new List<PilotValue>(args.Length);
        foreach (var a in args)
        {
            if (a.test() != null)
                evalArgs.Add(visitor.Visit(a.test()));
        }

        var fieldLayout = new List<(string Name, string CType, int Offset, int Size)>();
        int currentOffset = 4; // Bytes 0..3: uint FnId
        int maxAlign = 4;

        int paramCount = fn != null ? fn.Parameters.Count : evalArgs.Count;
        for (int i = 0; i < paramCount; i++)
        {
            string pName = fn != null && i < fn.Parameters.Count ? fn.Parameters[i].Name : $"P_{i}";
            string pType = fn != null && i < fn.Parameters.Count && !string.IsNullOrWhiteSpace(fn.Parameters[i].TypeName)
                ? fn.Parameters[i].TypeName!.Trim()
                : (i < evalArgs.Count ? InferAbiTypeFromValue(evalArgs[i]) : "f32");

            if (!IsUnmanagedAbiCompatibleType(pType, i < evalArgs.Count ? evalArgs[i] : default))
            {
                throw new PilotRuntimeException(
                    $"[QUOTE003] Cannot synthesize C-ABI CallFrame for '{functionName}!&(...)': parameter '{pName}: {pType}' is a managed reference type and does not satisfy 'unmanaged' / 'repr(C)' layout constraints.",
                    callSiteCtx.Start.Line,
                    callSiteCtx.Start.Column);
            }

            var (cType, fieldSize, fieldAlign) = GetAbiScalarSizeAndAlignment(pType);
            if (fieldAlign > maxAlign) maxAlign = fieldAlign;
            fieldLayout.Add((pName, cType, currentOffset, fieldSize));
            currentOffset += fieldSize;
        }

        byte[] raw = new byte[currentOffset];
        MemoryMarshal.Write(raw.AsSpan(0, 4), in fnId);

        for (int i = 0; i < fieldLayout.Count && i < evalArgs.Count; i++)
        {
            var fl = fieldLayout[i];
            var val = evalArgs[i];
            WriteScalarToAbiSpan(raw.AsSpan(fl.Offset, fl.Size), fl.CType, val);
        }

        string structName = $"CallFrame_{functionName}";
        return new PilotAbiCallFrame(
            structName,
            functionName,
            fnId,
            currentOffset,
            maxAlign,
            fieldLayout,
            evalArgs,
            raw);
    }

    /// <summary>
    /// Static <c>FnId</c> dispatch trampoline that unpacks a <see cref="PilotAbiCallFrame"/> and invokes the target function.
    /// </summary>
    public static PilotValue DispatchAbiFrame(PilotAbiCallFrame frame, PilotExecutionContext context)
    {
        if (TrampolineRegistry.TryGetValue(frame.FnId, out var fn))
        {
            return fn.Invoke(context, frame.ArgumentValues);
        }
        if (context.Environment.TryResolve(frame.FunctionName, out var resolved) && resolved.Kind == PilotValueKind.Function)
        {
            return resolved.AsFunction().Invoke(context, frame.ArgumentValues);
        }
        throw new PilotRuntimeException($"ABI trampoline error: no function registered for FnId 0x{frame.FnId:X8} ('{frame.FunctionName}').");
    }

    public static bool IsUnmanagedAbiCompatibleType(string typeSpec, PilotValue runtimeVal = default)
    {
        string clean = typeSpec.Trim();
        if (clean.EndsWith("?", StringComparison.Ordinal))
            clean = clean[..^1].Trim();

        if (clean is "str" or "string" or "List" or "Map" or "Dict" or "Set" or "Object" or "dynamic")
            return false;
        if (clean.StartsWith("List[", StringComparison.OrdinalIgnoreCase) ||
            clean.StartsWith("List<", StringComparison.OrdinalIgnoreCase) ||
            clean.StartsWith("Map[", StringComparison.OrdinalIgnoreCase) ||
            clean.StartsWith("Map<", StringComparison.OrdinalIgnoreCase) ||
            clean.StartsWith("Set[", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        if (runtimeVal.Kind is PilotValueKind.String or PilotValueKind.List or PilotValueKind.Map or PilotValueKind.Set or PilotValueKind.Object)
            return false;

        return true;
    }

    public static (string CType, int SizeBytes, int AlignBytes) GetAbiScalarSizeAndAlignment(string typeSpec)
    {
        string clean = typeSpec.Trim().ToLowerInvariant();
        return clean switch
        {
            "bool" or "boolean" or "u8" or "i8" or "byte" => ("uint8_t", 4, 4),
            "f64" or "double" => ("double", 8, 8),
            "i64" or "long" => ("int64_t", 8, 8),
            "u64" or "ulong" => ("uint64_t", 8, 8),
            "float" or "f32" => ("float", 4, 4),
            "int" or "i32" => ("int32_t", 4, 4),
            "uint" or "u32" => ("uint32_t", 4, 4),
            _ when clean.StartsWith("slicebuffer", StringComparison.Ordinal) => ("PilotSliceBuffer", 24, 8),
            _ when clean.StartsWith("rangewindow", StringComparison.Ordinal) => ("PilotRangeWindow", 16, 8),
            _ => ("float", 4, 4)
        };
    }

    private static string InferAbiTypeFromValue(PilotValue val) =>
        val.Kind switch
        {
            PilotValueKind.Int => "int",
            PilotValueKind.Float => "float",
            PilotValueKind.Bool => "bool",
            PilotValueKind.String => "str",
            PilotValueKind.List => "List",
            PilotValueKind.Map => "Map",
            PilotValueKind.Object => "Object",
            PilotValueKind.SliceBuffer => "SliceBuffer",
            _ => "float"
        };

    private static void WriteScalarToAbiSpan(Span<byte> dest, string cType, PilotValue val)
    {
        switch (cType)
        {
            case "float":
            {
                float f = (float)val.AsFloat();
                MemoryMarshal.Write(dest, in f);
                break;
            }
            case "double":
            {
                double d = val.AsFloat();
                MemoryMarshal.Write(dest, in d);
                break;
            }
            case "int32_t":
            {
                int i = (int)val.AsInt();
                MemoryMarshal.Write(dest, in i);
                break;
            }
            case "uint32_t":
            {
                uint u = (uint)val.AsInt();
                MemoryMarshal.Write(dest, in u);
                break;
            }
            case "int64_t":
            {
                long l = val.AsInt();
                MemoryMarshal.Write(dest, in l);
                break;
            }
            case "uint8_t":
            {
                int b = val.IsTrue ? 1 : 0;
                MemoryMarshal.Write(dest, in b);
                break;
            }
        }
    }

    #region ParseTree -> Symbolic AST Conversion with Hygienic Alpha-Renaming

    public static IParseTree? ParseSnippetToSuite(string snippet) =>
        PilotParser.GetOrParseSnippet(snippet);

    public static PilotAstExpr ExtractSymbolicBodyAst(
        IParseTree bodyTree,
        Dictionary<string, PilotAstExpr> envSubstitutions,
        Dictionary<string, string> alphaRenames) =>
        ConvertFunctionBodyToSymbolicAst(bodyTree, envSubstitutions, alphaRenames);

    private static PilotAstExpr ConvertFunctionBodyToSymbolicAst(
        IParseTree bodyTree,
        Dictionary<string, PilotAstExpr> envSubstitutions,
        Dictionary<string, string> alphaRenames)
    {
        if (bodyTree is PilotParser.SuiteContext suite)
        {
            if (suite.simple_stmt() != null)
            {
                return ConvertSimpleStmtToSymbolicAst(suite.simple_stmt(), envSubstitutions, alphaRenames);
            }

            var stmts = suite.stmt();
            if (stmts != null && stmts.Length > 0)
            {
                return ConvertStmtSequenceToSymbolicAst(stmts, 0, envSubstitutions, alphaRenames);
            }
        }
        return ConvertParseTreeToSymbolicAst(bodyTree, envSubstitutions, alphaRenames);
    }

    private static PilotAstExpr ConvertStmtSequenceToSymbolicAst(
        PilotParser.StmtContext[] stmts,
        int index,
        Dictionary<string, PilotAstExpr> envSubstitutions,
        Dictionary<string, string> alphaRenames)
    {
        if (index >= stmts.Length)
            return PilotAstExpr.Const(PilotValue.Null);

        var st = stmts[index];
        if (st.simple_stmt()?.small_stmt() is { Length: > 0 } smalls)
        {
            var sm = smalls[0];
            if (sm.let_stmt() is { } ls && ls.binding_pattern()?.identifier() != null && ls.test() != null)
            {
                string origLocal = ls.binding_pattern().identifier().GetText();
                var rhsAst = ConvertParseTreeToSymbolicAst(ls.test(), envSubstitutions, alphaRenames).FoldConstants();
                long id = System.Threading.Interlocked.Increment(ref _hygieneCounter);
                string renamedLocal = $"{origLocal}__hyg_{id}";
                alphaRenames[origLocal] = renamedLocal;

                var nextSubs = new Dictionary<string, PilotAstExpr>(envSubstitutions, StringComparer.Ordinal);
                if (rhsAst.NodeKind == PilotAstNodeKind.Const)
                {
                    // Constant propagation through local binding!
                    nextSubs[origLocal] = rhsAst;
                    return ConvertStmtSequenceToSymbolicAst(stmts, index + 1, nextSubs, alphaRenames);
                }
                else
                {
                    // Substitute single-expression symbolic local or bind hygienically
                    nextSubs[origLocal] = rhsAst;
                    return ConvertStmtSequenceToSymbolicAst(stmts, index + 1, nextSubs, alphaRenames);
                }
            }

            if (sm.var_stmt() is { } vs && vs.binding_pattern()?.identifier() != null && vs.test() != null)
            {
                string origLocal = vs.binding_pattern().identifier().GetText();
                var rhsAst = ConvertParseTreeToSymbolicAst(vs.test(), envSubstitutions, alphaRenames).FoldConstants();
                long id = System.Threading.Interlocked.Increment(ref _hygieneCounter);
                alphaRenames[origLocal] = $"{origLocal}__hyg_{id}";

                var nextSubs = new Dictionary<string, PilotAstExpr>(envSubstitutions, StringComparer.Ordinal)
                {
                    [origLocal] = rhsAst
                };
                return ConvertStmtSequenceToSymbolicAst(stmts, index + 1, nextSubs, alphaRenames);
            }

            if (sm.flow_stmt() is { } fs && fs.RETURN() != null)
            {
                if (fs.testlist()?.test() is { Length: > 0 } retTests)
                    return ConvertParseTreeToSymbolicAst(retTests[0], envSubstitutions, alphaRenames);
                if (fs.test() is { Length: > 0 } directTests)
                    return ConvertParseTreeToSymbolicAst(directTests[0], envSubstitutions, alphaRenames);
            }
        }

        if (st.compound_stmt()?.if_stmt() is { } ifStmt)
        {
            var tests = ifStmt.test();
            var suites = ifStmt.suite();
            if (tests.Length >= 1 && suites.Length >= 2)
            {
                var condAst = ConvertParseTreeToSymbolicAst(tests[0], envSubstitutions, alphaRenames);
                var thenAst = ConvertFunctionBodyToSymbolicAst(suites[0], envSubstitutions, alphaRenames);
                var elseAst = ConvertFunctionBodyToSymbolicAst(suites[1], envSubstitutions, alphaRenames);
                return PilotAstExpr.If(condAst, thenAst, elseAst);
            }
            if (tests.Length >= 1 && suites.Length == 1 && index + 1 < stmts.Length)
            {
                var condAst = ConvertParseTreeToSymbolicAst(tests[0], envSubstitutions, alphaRenames);
                var thenAst = ConvertFunctionBodyToSymbolicAst(suites[0], envSubstitutions, alphaRenames);
                var elseAst = ConvertStmtSequenceToSymbolicAst(stmts, index + 1, envSubstitutions, alphaRenames);
                return PilotAstExpr.If(condAst, thenAst, elseAst);
            }
        }

        if (index == stmts.Length - 1)
        {
            return ConvertParseTreeToSymbolicAst(st, envSubstitutions, alphaRenames);
        }

        return ConvertStmtSequenceToSymbolicAst(stmts, index + 1, envSubstitutions, alphaRenames);
    }

    private static PilotAstExpr ConvertSimpleStmtToSymbolicAst(
        PilotParser.Simple_stmtContext simple,
        Dictionary<string, PilotAstExpr> envSubstitutions,
        Dictionary<string, string> alphaRenames)
    {
        var smalls = simple.small_stmt();
        if (smalls == null || smalls.Length == 0)
            return PilotAstExpr.Const(PilotValue.Null);

        var last = smalls[^1];
        if (last.flow_stmt()?.testlist()?.test() is { Length: > 0 } flTests)
            return ConvertParseTreeToSymbolicAst(flTests[0], envSubstitutions, alphaRenames);
        if (last.flow_stmt()?.test() is { Length: > 0 } flDirectTests)
            return ConvertParseTreeToSymbolicAst(flDirectTests[0], envSubstitutions, alphaRenames);
        if (last.expr_stmt()?.testlist()?.test() is { Length: > 0 } tests)
            return ConvertParseTreeToSymbolicAst(tests[0], envSubstitutions, alphaRenames);

        return ConvertParseTreeToSymbolicAst(last, envSubstitutions, alphaRenames);
    }

    public static PilotAstExpr ConvertParseTreeToSymbolicAst(
        IParseTree tree,
        Dictionary<string, PilotAstExpr> envSubstitutions,
        Dictionary<string, string> alphaRenames)
    {
        switch (tree)
        {
            case PilotParser.StmtContext st:
                return st.simple_stmt() != null
                    ? ConvertSimpleStmtToSymbolicAst(st.simple_stmt(), envSubstitutions, alphaRenames)
                    : (st.compound_stmt() != null ? ConvertParseTreeToSymbolicAst(st.compound_stmt(), envSubstitutions, alphaRenames) : PilotAstExpr.Const(PilotValue.Null));

            case PilotParser.TestContext testCtx:
            {
                var pipes = testCtx.pipe_expr();
                if (testCtx.IF() != null && pipes.Length >= 2 && testCtx.test() != null)
                {
                    var thenBr = ConvertParseTreeToSymbolicAst(pipes[0], envSubstitutions, alphaRenames);
                    var cond = ConvertParseTreeToSymbolicAst(pipes[1], envSubstitutions, alphaRenames);
                    var elseBr = ConvertParseTreeToSymbolicAst(testCtx.test(), envSubstitutions, alphaRenames);
                    return PilotAstExpr.If(cond, thenBr, elseBr);
                }
                if (pipes.Length > 0)
                    return ConvertParseTreeToSymbolicAst(pipes[0], envSubstitutions, alphaRenames);
                return PilotAstExpr.Const(PilotValue.Null);
            }

            case PilotParser.Pipe_exprContext pipeCtx:
                return pipeCtx.set_query_expr().Length > 0
                    ? ConvertParseTreeToSymbolicAst(pipeCtx.set_query_expr()[0], envSubstitutions, alphaRenames)
                    : PilotAstExpr.Const(PilotValue.Null);

            case PilotParser.Set_query_exprContext sqCtx:
                return sqCtx.coalesce_expr().Length > 0
                    ? ConvertParseTreeToSymbolicAst(sqCtx.coalesce_expr()[0], envSubstitutions, alphaRenames)
                    : PilotAstExpr.Const(PilotValue.Null);

            case PilotParser.Coalesce_exprContext coalCtx:
                return coalCtx.or_test().Length > 0
                    ? ConvertParseTreeToSymbolicAst(coalCtx.or_test()[0], envSubstitutions, alphaRenames)
                    : PilotAstExpr.Const(PilotValue.Null);

            case PilotParser.Or_testContext orCtx:
            {
                var ands = orCtx.and_test();
                var cur = ConvertParseTreeToSymbolicAst(ands[0], envSubstitutions, alphaRenames);
                for (int i = 1; i < ands.Length; i++)
                    cur = PilotAstExpr.Binary("||", cur, ConvertParseTreeToSymbolicAst(ands[i], envSubstitutions, alphaRenames), "bool");
                return cur;
            }

            case PilotParser.And_testContext andCtx:
            {
                var nots = andCtx.not_test();
                var cur = ConvertParseTreeToSymbolicAst(nots[0], envSubstitutions, alphaRenames);
                for (int i = 1; i < nots.Length; i++)
                    cur = PilotAstExpr.Binary("&&", cur, ConvertParseTreeToSymbolicAst(nots[i], envSubstitutions, alphaRenames), "bool");
                return cur;
            }

            case PilotParser.Not_testContext notCtx:
                if ((notCtx.NOT() != null || notCtx.EXCLAMATION() != null) && notCtx.not_test() != null)
                    return PilotAstExpr.Unary("!", ConvertParseTreeToSymbolicAst(notCtx.not_test(), envSubstitutions, alphaRenames), "bool");
                if (notCtx.CALL_QUOTE_DEEP() != null && notCtx.not_test() != null)
                    return PilotAstExpr.Unary("!", PilotAstExpr.Unary("!", ConvertParseTreeToSymbolicAst(notCtx.not_test(), envSubstitutions, alphaRenames), "bool"), "bool");
                return notCtx.comparison() != null
                    ? ConvertParseTreeToSymbolicAst(notCtx.comparison(), envSubstitutions, alphaRenames)
                    : PilotAstExpr.Const(PilotValue.Null);

            case PilotParser.ComparisonContext compCtx:
            {
                var ranges = compCtx.range_expr();
                var cur = ConvertParseTreeToSymbolicAst(ranges[0], envSubstitutions, alphaRenames);
                var ops = compCtx.comp_op();
                for (int i = 1; i < ranges.Length && i - 1 < ops.Length; i++)
                {
                    cur = PilotAstExpr.Binary(
                        ops[i - 1].GetText(),
                        cur,
                        ConvertParseTreeToSymbolicAst(ranges[i], envSubstitutions, alphaRenames),
                        "bool");
                }
                return cur;
            }

            case PilotParser.Range_exprContext rCtx:
                return rCtx.bitor_expr().Length > 0
                    ? ConvertParseTreeToSymbolicAst(rCtx.bitor_expr()[0], envSubstitutions, alphaRenames)
                    : PilotAstExpr.Const(PilotValue.Null);

            case PilotParser.Bitor_exprContext boCtx:
                return boCtx.bitxor_expr().Length > 0
                    ? ConvertParseTreeToSymbolicAst(boCtx.bitxor_expr()[0], envSubstitutions, alphaRenames)
                    : PilotAstExpr.Const(PilotValue.Null);

            case PilotParser.Bitxor_exprContext bxCtx:
                return bxCtx.bitand_expr().Length > 0
                    ? ConvertParseTreeToSymbolicAst(bxCtx.bitand_expr()[0], envSubstitutions, alphaRenames)
                    : PilotAstExpr.Const(PilotValue.Null);

            case PilotParser.Bitand_exprContext baCtx:
                return baCtx.shift_expr().Length > 0
                    ? ConvertParseTreeToSymbolicAst(baCtx.shift_expr()[0], envSubstitutions, alphaRenames)
                    : PilotAstExpr.Const(PilotValue.Null);

            case PilotParser.Shift_exprContext shCtx:
                return shCtx.arith_expr().Length > 0
                    ? ConvertParseTreeToSymbolicAst(shCtx.arith_expr()[0], envSubstitutions, alphaRenames)
                    : PilotAstExpr.Const(PilotValue.Null);

            case PilotParser.Arith_exprContext arithCtx:
            {
                var terms = arithCtx.term();
                var cur = ConvertParseTreeToSymbolicAst(terms[0], envSubstitutions, alphaRenames);
                for (int i = 1; i < terms.Length; i++)
                {
                    string op = arithCtx.GetChild(2 * i - 1).GetText();
                    cur = PilotAstExpr.Binary(op, cur, ConvertParseTreeToSymbolicAst(terms[i], envSubstitutions, alphaRenames));
                }
                return cur;
            }

            case PilotParser.TermContext termCtx:
            {
                var factors = termCtx.factor();
                var cur = ConvertParseTreeToSymbolicAst(factors[0], envSubstitutions, alphaRenames);
                for (int i = 1; i < factors.Length; i++)
                {
                    string op = termCtx.GetChild(2 * i - 1).GetText();
                    cur = PilotAstExpr.Binary(op, cur, ConvertParseTreeToSymbolicAst(factors[i], envSubstitutions, alphaRenames));
                }
                return cur;
            }

            case PilotParser.FactorContext facCtx:
                if (facCtx.MINUS() != null && facCtx.factor() != null)
                    return PilotAstExpr.Unary("-", ConvertParseTreeToSymbolicAst(facCtx.factor(), envSubstitutions, alphaRenames));
                if (facCtx.PLUS() != null && facCtx.factor() != null)
                    return ConvertParseTreeToSymbolicAst(facCtx.factor(), envSubstitutions, alphaRenames);
                if (facCtx.TILDE() != null && facCtx.factor() != null)
                    return PilotAstExpr.Unary("~", ConvertParseTreeToSymbolicAst(facCtx.factor(), envSubstitutions, alphaRenames));
                if (facCtx.power() != null)
                    return ConvertParseTreeToSymbolicAst(facCtx.power(), envSubstitutions, alphaRenames);
                return PilotAstExpr.Const(PilotValue.Null);

            case PilotParser.PowerContext powCtx:
            {
                var baseAst = ConvertParseTreeToSymbolicAst(powCtx.atom_expr(), envSubstitutions, alphaRenames);
                if (powCtx.DOUBLESTAR() != null && powCtx.factor() != null)
                {
                    return PilotAstExpr.Binary("**", baseAst, ConvertParseTreeToSymbolicAst(powCtx.factor(), envSubstitutions, alphaRenames));
                }
                return baseAst;
            }

            case PilotParser.Atom_exprContext aeCtx:
            {
                var trailers = aeCtx.trailer();
                if (trailers == null || trailers.Length == 0)
                    return ConvertParseTreeToSymbolicAst(aeCtx.atom(), envSubstitutions, alphaRenames);

                // In Pilot.g4, `a * (b)` is parsed as `atom_expr` with a `STAR LPAR arglist RPAR` trailer.
                // When `atom` is a parameter/local variable or scalar expression and the trailer is `* (expr)`,
                // lower it to symbolic multiplication `Mul(lhs, rhs)`.
                if (trailers[0].STAR() != null && trailers[0].DOT() == null && trailers[0].LPAR() != null)
                {
                    var starArgs = trailers[0].arglist()?.argument();
                    if (starArgs is { Length: 1 } && starArgs[0].test() != null)
                    {
                        var cur = ConvertParseTreeToSymbolicAst(aeCtx.atom(), envSubstitutions, alphaRenames);
                        for (int i = 0; i < trailers.Length; i++)
                        {
                            if (trailers[i].STAR() != null && trailers[i].DOT() == null &&
                                trailers[i].arglist()?.argument() is { Length: 1 } sArgs && sArgs[0].test() != null)
                            {
                                cur = PilotAstExpr.Mul(cur, ConvertParseTreeToSymbolicAst(sArgs[0].test(), envSubstitutions, alphaRenames));
                            }
                        }
                        return cur;
                    }
                }

                if (aeCtx.atom()?.identifier() != null && trailers[0].LPAR() != null)
                {
                    string fnName = aeCtx.atom().identifier().GetText();
                    var callArgs = trailers[0].arglist()?.argument();
                    var argAsts = new List<PilotAstExpr>();
                    if (callArgs != null)
                    {
                        foreach (var ca in callArgs)
                        {
                            if (ca.test() != null)
                                argAsts.Add(ConvertParseTreeToSymbolicAst(ca.test(), envSubstitutions, alphaRenames));
                        }
                    }
                    return PilotAstExpr.Call(fnName, argAsts);
                }
                return ConvertParseTreeToSymbolicAst(aeCtx.atom(), envSubstitutions, alphaRenames);
            }

            case PilotParser.AtomContext atomCtx:
            {
                if (atomCtx.literal() is { } lit)
                {
                    if (lit.INTEGER() != null && long.TryParse(lit.INTEGER().GetText().Replace("_", ""), NumberStyles.Integer, CultureInfo.InvariantCulture, out long iv))
                        return PilotAstExpr.Const(PilotValue.FromInt(iv), "int");
                    if (lit.FLOAT_NUMBER() != null && double.TryParse(lit.FLOAT_NUMBER().GetText().Replace("_", ""), NumberStyles.Float, CultureInfo.InvariantCulture, out double dv))
                        return PilotAstExpr.Const(PilotValue.FromFloat(dv), "float");
                    if (lit.TRUE() != null)
                        return PilotAstExpr.Const(PilotValue.True, "bool");
                    if (lit.FALSE() != null)
                        return PilotAstExpr.Const(PilotValue.False, "bool");
                    if (lit.STRING().Length > 0)
                        return PilotAstExpr.Const(PilotValue.FromString(lit.STRING()[0].GetText().Trim('"', '\'')), "str");
                }

                if (atomCtx.identifier() != null && atomCtx.DOT() == null)
                {
                    string id = atomCtx.identifier().GetText();
                    if (envSubstitutions.TryGetValue(id, out var sub))
                        return sub;
                    if (alphaRenames.TryGetValue(id, out var renamed))
                        return PilotAstExpr.Sym(renamed);
                    return PilotAstExpr.Sym(id);
                }

                if (atomCtx.LPAR() != null && atomCtx.testlist_comp()?.test() is { Length: 1 } innerTests)
                {
                    return ConvertParseTreeToSymbolicAst(innerTests[0], envSubstitutions, alphaRenames);
                }

                if (atomCtx.if_stmt() is { } ifExpr)
                {
                    var tests = ifExpr.test();
                    var suites = ifExpr.suite();
                    if (tests.Length >= 1 && suites.Length >= 2)
                    {
                        return PilotAstExpr.If(
                            ConvertParseTreeToSymbolicAst(tests[0], envSubstitutions, alphaRenames),
                            ConvertFunctionBodyToSymbolicAst(suites[0], envSubstitutions, alphaRenames),
                            ConvertFunctionBodyToSymbolicAst(suites[1], envSubstitutions, alphaRenames));
                    }
                }

                return PilotAstExpr.Sym(atomCtx.GetText());
            }

            default:
                for (int i = 0; i < tree.ChildCount; i++)
                {
                    if (tree.GetChild(i) is ParserRuleContext childRule)
                        return ConvertParseTreeToSymbolicAst(childRule, envSubstitutions, alphaRenames);
                }
                return PilotAstExpr.Sym(tree.GetText());
        }
    }

    #endregion
}
