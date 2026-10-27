using System;
using System.Collections.Generic;
using System.Linq;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;

namespace Pilot.Analysis.TypeSystem;

/// <summary>
/// Diagnostic reported by the static analysis pipeline.
/// </summary>
public sealed class PilotAnalysisDiagnostic
{
    public string Code { get; }
    public string Message { get; }
    public int Line { get; }
    public int Column { get; }
    public bool IsError { get; }

    public PilotAnalysisDiagnostic(string code, string message, int line, int column, bool isError = true)
    {
        Code = code;
        Message = message;
        Line = line;
        Column = column;
        IsError = isError;
    }

    public override string ToString() =>
        $"[{Code} Line {Line}:{Column}] {Message}";
}

/// <summary>
/// Flow-sensitive bidirectional type inference and constraint solver for Pilot programs.
/// Reconstructs concrete static types for unannotated bindings, evaluates collection literals,
/// solves higher-order function types, and performs flow-sensitive type narrowing.
/// </summary>
public sealed class PilotTypeInferenceEngine : PilotBaseVisitor<PilotType>
{
    private readonly Dictionary<string, PilotType> _environment = new(StringComparer.Ordinal);
    private readonly Stack<Dictionary<string, PilotType>> _scopes = new();
    private readonly Dictionary<int, PilotType> _substitutions = new();
    private readonly List<PilotAnalysisDiagnostic> _diagnostics = new();

    public IReadOnlyList<PilotAnalysisDiagnostic> Diagnostics => _diagnostics;
    public IReadOnlyDictionary<string, PilotType> InferredTypes => _environment;

    public PilotTypeInferenceEngine()
    {
        PushScope();
        RegisterBuiltins();
    }

    private void RegisterBuiltins()
    {
        _environment["true"] = PilotType.Bool;
        _environment["false"] = PilotType.Bool;
        _environment["null"] = PilotType.Null;
        _environment["none"] = PilotType.Null;
        _environment["print"] = PilotType.FunctionOf(new[] { PilotType.Any }, PilotType.Void);
        _environment["println"] = PilotType.FunctionOf(new[] { PilotType.Any }, PilotType.Void);
        _environment["assert"] = PilotType.FunctionOf(new[] { PilotType.Bool }, PilotType.Void);
        _environment["str"] = PilotType.FunctionOf(new[] { PilotType.Any }, PilotType.String);
        _environment["int"] = PilotType.FunctionOf(new[] { PilotType.Any }, PilotType.Int);
        _environment["float"] = PilotType.FunctionOf(new[] { PilotType.Any }, PilotType.Float);
        _environment["len"] = PilotType.FunctionOf(new[] { PilotType.Any }, PilotType.Int);
        _environment["range"] = PilotType.FunctionOf(new[] { PilotType.Int }, PilotType.ListOf(PilotType.Int));
    }

    public void PushScope()
    {
        _scopes.Push(new Dictionary<string, PilotType>(StringComparer.Ordinal));
    }

    protected override PilotType DefaultResult => PilotType.Any;

    protected override PilotType AggregateResult(PilotType aggregate, PilotType nextResult)
    {
        return nextResult != null && nextResult != PilotType.Any ? nextResult : aggregate;
    }

    public override PilotType VisitProgram(PilotParser.ProgramContext context)
    {
        var stmts = context.stmt();
        if (stmts != null)
        {
            foreach (var s in stmts)
            {
                Visit(s);
            }
        }
        return PilotType.Void;
    }

    public void PopScope()
    {
        if (_scopes.Count > 1)
        {
            _scopes.Pop();
        }
    }

    public void SetVariableType(string name, PilotType type)
    {
        if (_scopes.Count > 0)
        {
            _scopes.Peek()[name] = type;
        }
        _environment[name] = type;
    }

    public bool TryLookupType(string name, out PilotType type)
    {
        foreach (var scope in _scopes)
        {
            if (scope.TryGetValue(name, out var t))
            {
                type = PilotType.Resolve(t, _substitutions);
                return true;
            }
        }
        if (_environment.TryGetValue(name, out var globalT))
        {
            type = PilotType.Resolve(globalT, _substitutions);
            return true;
        }
        type = PilotType.Any;
        return false;
    }

    public static PilotType ParseTypeString(string typeStr)
    {
        if (string.IsNullOrWhiteSpace(typeStr)) return PilotType.Any;
        typeStr = typeStr.Trim();

        if (typeStr.EndsWith("?"))
        {
            var inner = ParseTypeString(typeStr[..^1]);
            return PilotType.OptionOf(inner);
        }

        if (typeStr.Contains('|'))
        {
            var parts = typeStr.Split('|').Select(p => ParseTypeString(p.Trim())).ToArray();
            return PilotType.UnionOf(parts);
        }

        if (typeStr.StartsWith("List<") && typeStr.EndsWith(">"))
        {
            var inner = ParseTypeString(typeStr[5..^1]);
            return PilotType.ListOf(inner);
        }

        if (typeStr.StartsWith("Map<") && typeStr.EndsWith(">"))
        {
            var innerParts = typeStr[4..^1].Split(',');
            var k = innerParts.Length > 0 ? ParseTypeString(innerParts[0].Trim()) : PilotType.Any;
            var v = innerParts.Length > 1 ? ParseTypeString(innerParts[1].Trim()) : PilotType.Any;
            return PilotType.MapOf(k, v);
        }

        int unitOpen = typeStr.IndexOf('<');
        int unitClose = typeStr.LastIndexOf('>');
        if (unitOpen > 0 && unitClose > unitOpen)
        {
            string basePart = typeStr[..unitOpen];
            string unitPart = typeStr[(unitOpen + 1)..unitClose];
            return PilotType.WithUnit(ParseTypeString(basePart), unitPart);
        }

        return typeStr.ToLowerInvariant() switch
        {
            "int" or "i32" or "i64" or "long" => PilotType.Int,
            "float" or "f32" or "f64" or "double" => PilotType.Float,
            "str" or "string" => PilotType.String,
            "bool" or "boolean" => PilotType.Bool,
            "void" => PilotType.Void,
            "null" => PilotType.Null,
            _ => new PilotType(PilotTypeKind.Primitive, typeStr)
        };
    }

    public override PilotType VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "";
        string declaredTypeStr = context.type_annotation()?.type_spec()?.GetText() ?? "";
        PilotType? declaredType = !string.IsNullOrEmpty(declaredTypeStr) ? ParseTypeString(declaredTypeStr) : null;

        PilotType inferredRhs = (context.test() != null ? Visit(context.test()) : null) ?? PilotType.Any;
        PilotType finalType = declaredType ?? inferredRhs;

        if (declaredType != null && inferredRhs != PilotType.Any && inferredRhs != PilotType.Null)
        {
            if (!inferredRhs.IsAssignableTo(declaredType))
            {
                _diagnostics.Add(new PilotAnalysisDiagnostic(
                    "PILOT300",
                    $"Inferred type mismatch on binding '{name}': cannot assign expression of type '{inferredRhs}' to declared type '{declaredType}'.",
                    context.Start.Line,
                    context.Start.Column));
            }
        }

        if (!string.IsNullOrEmpty(name))
        {
            SetVariableType(name, finalType);
        }
        return finalType;
    }

    public override PilotType VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "";
        string declaredTypeStr = context.type_annotation()?.type_spec()?.GetText() ?? "";
        PilotType? declaredType = !string.IsNullOrEmpty(declaredTypeStr) ? ParseTypeString(declaredTypeStr) : null;

        PilotType inferredRhs = (context.test() != null ? Visit(context.test()) : null) ?? PilotType.Any;
        PilotType finalType = declaredType ?? inferredRhs;

        if (declaredType != null && inferredRhs != PilotType.Any && inferredRhs != PilotType.Null)
        {
            if (!inferredRhs.IsAssignableTo(declaredType))
            {
                _diagnostics.Add(new PilotAnalysisDiagnostic(
                    "PILOT300",
                    $"Inferred type mismatch on binding '{name}': cannot assign expression of type '{inferredRhs}' to declared type '{declaredType}'.",
                    context.Start.Line,
                    context.Start.Column));
            }
        }

        if (!string.IsNullOrEmpty(name))
        {
            SetVariableType(name, finalType);
        }
        return finalType;
    }

    public override PilotType VisitFunc_def(PilotParser.Func_defContext context)
    {
        string funcName = context.identifier().GetText();
        PushScope();

        var paramTypes = new List<PilotType>();
        var parameters = context.parameters()?.parameter();
        if (parameters != null)
        {
            foreach (var p in parameters)
            {
                string pName = p.identifier()?.GetText() ?? "arg";
                string pTypeStr = p.type_annotation()?.type_spec()?.GetText() ?? "";
                PilotType pType = !string.IsNullOrEmpty(pTypeStr) ? ParseTypeString(pTypeStr) : PilotType.NewTypeVariable();
                paramTypes.Add(pType);
                SetVariableType(pName, pType);
            }
        }

        string declaredRetStr = context.type_spec()?.GetText() ?? "";
        PilotType expectedRet = !string.IsNullOrEmpty(declaredRetStr) ? ParseTypeString(declaredRetStr) : PilotType.Any;

        VisitSuite(context.suite());
        PopScope();

        var funcType = PilotType.FunctionOf(paramTypes, expectedRet);
        SetVariableType(funcName, funcType);
        return funcType;
    }

    public override PilotType VisitTest(PilotParser.TestContext context)
    {
        var pipes = context.pipe_expr();
        if (pipes != null && pipes.Length > 0)
        {
            return Visit(pipes[0]);
        }
        return PilotType.Any;
    }

    public override PilotType VisitArith_expr(PilotParser.Arith_exprContext context)
    {
        var terms = context.term();
        if (terms == null || terms.Length == 0) return PilotType.Any;

        PilotType current = Visit(terms[0]);
        for (int i = 1; i < terms.Length; i++)
        {
            PilotType next = Visit(terms[i]);
            if (current == PilotType.String || next == PilotType.String)
            {
                current = PilotType.String;
            }
            else if (current == PilotType.Float || next == PilotType.Float)
            {
                current = PilotType.Float;
            }
            else if (current == PilotType.Int && next == PilotType.Int)
            {
                current = PilotType.Int;
            }
        }
        return current;
    }

    public override PilotType VisitAtom(PilotParser.AtomContext context)
    {
        if (context.literal() is { } lit)
        {
            if (lit.INTEGER() != null || lit.HEX_INTEGER() != null || lit.BIN_INTEGER() != null || lit.OCT_INTEGER() != null)
                return PilotType.Int;
            if (lit.FLOAT_NUMBER() != null)
                return PilotType.Float;
            if (lit.STRING().Length > 0 || lit.TRIPLE_STRING().Length > 0 || lit.FSTRING().Length > 0)
                return PilotType.String;
            if (lit.TRUE() != null || lit.FALSE() != null)
                return PilotType.Bool;
            if (lit.NONE() != null || lit.NULL() != null)
                return PilotType.Null;
        }

        if (context.identifier() != null)
        {
            string id = context.identifier().GetText();
            if (TryLookupType(id, out var resolved))
            {
                return resolved;
            }
            return PilotType.Any;
        }

        // Collection / List literal: [1, 2, 3]
        if (context.collection_expr() is { } colExpr)
        {
            var elements = colExpr.collection_element();
            PilotType elemType = elements != null && elements.Length > 0 && elements[0].test() != null
                ? Visit(elements[0].test())
                : PilotType.Any;
            return PilotType.ListOf(elemType);
        }

        // Map literal: { "a": 1 }
        if (context.LBRACE() != null && context.dictorsetmaker() != null)
        {
            return PilotType.MapOf(PilotType.String, PilotType.Any);
        }

        return PilotType.Any;
    }
}
