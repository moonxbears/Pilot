using System;
using System.Collections.Generic;
using System.Linq;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Runtime.Values;

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

        if (PilotAdtMemoryLayout.TrySplitFallibleTypeSpec(typeStr, out string okSpec, out string errSpec))
        {
            var okType = ParseTypeString(okSpec);
            var errType = string.IsNullOrEmpty(errSpec) ? PilotType.Any : ParseTypeString(errSpec);
            return PilotType.ResultOf(okType, errType);
        }

        if (typeStr.EndsWith("?"))
        {
            var inner = ParseTypeString(typeStr[..^1]);
            return PilotType.OptionOf(inner);
        }

        if (typeStr.EndsWith("[]"))
        {
            var inner = ParseTypeString(typeStr[..^2]);
            return PilotType.ListOf(inner);
        }

        if (typeStr.Contains('|'))
        {
            var parts = typeStr.Split('|').Select(p => ParseTypeString(p.Trim())).ToArray();
            return PilotType.UnionOf(parts);
        }

        if ((typeStr.StartsWith("List<") && typeStr.EndsWith(">")) ||
            (typeStr.StartsWith("List[") && typeStr.EndsWith("]")))
        {
            var inner = ParseTypeString(typeStr[5..^1]);
            return PilotType.ListOf(inner);
        }

        if ((typeStr.StartsWith("Map<") && typeStr.EndsWith(">")) ||
            (typeStr.StartsWith("Map[") && typeStr.EndsWith("]")) ||
            (typeStr.StartsWith("Dict[") && typeStr.EndsWith("]")))
        {
            int openIdx = typeStr.IndexOfAny(new[] { '<', '[' });
            var innerParts = typeStr[(openIdx + 1)..^1].Split(',');
            var k = innerParts.Length > 0 ? ParseTypeString(innerParts[0].Trim()) : PilotType.Any;
            var v = innerParts.Length > 1 ? ParseTypeString(innerParts[1].Trim()) : PilotType.Any;
            return PilotType.MapOf(k, v);
        }

        if (typeStr.StartsWith("(") && typeStr.EndsWith(")") && !typeStr.Contains("->"))
        {
            var parts = typeStr[1..^1].Split(',').Select(p => ParseTypeString(p.Trim())).ToArray();
            return PilotType.TupleOf(parts);
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

        if (context.suite() != null)
        {
            VisitSuite(context.suite());
        }
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

    public override PilotType VisitCoalesce_expr(PilotParser.Coalesce_exprContext context)
    {
        var orTests = context.or_test();
        if (orTests == null || orTests.Length == 0) return PilotType.Any;

        PilotType current = Visit(orTests[0]);
        var catches = context.catch_clause();
        if (catches != null && catches.Length > 0)
        {
            PilotType errType = current.IsResult ? current.TypeArguments[1] : PilotType.Any;
            foreach (var cc in catches)
            {
                PushScope();
                if (cc.identifier() != null)
                {
                    SetVariableType(cc.identifier().GetText(), errType);
                }
                Visit(cc);
                PopScope();
            }
            if (current.IsResult)
                current = current.TypeArguments[0];
        }
        else if (orTests.Length > 1)
        {
            for (int i = 1; i < orTests.Length; i++)
                Visit(orTests[i]);
            if (current.Kind == PilotTypeKind.Option && current.TypeArguments.Count == 1)
                current = current.TypeArguments[0];
        }
        return current;
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

        if (context.match_stmt() != null)
        {
            Visit(context.match_stmt());
            return PilotType.Any;
        }

        return PilotType.Any;
    }

    public override PilotType VisitAtom_expr(PilotParser.Atom_exprContext context)
    {
        var trailers = context.trailer();
        if (trailers == null || trailers.Length == 0)
        {
            return Visit(context.atom());
        }

        var atomType = Visit(context.atom());
        foreach (var tr in trailers)
        {
            if (tr.QUESTION() != null)
            {
                if (atomType.Kind == PilotTypeKind.Option && atomType.TypeArguments.Count == 1)
                    atomType = atomType.TypeArguments[0];
                else if (atomType.IsResult)
                    atomType = atomType.TypeArguments[0];
            }
            else if (tr.LPAR() != null && atomType.Kind == PilotTypeKind.Function && atomType.ReturnType != null)
            {
                Visit(tr);
                atomType = atomType.ReturnType;
            }
            else if (tr.DOT() != null && tr.identifier() != null && atomType.Kind == PilotTypeKind.Primitive)
            {
                string qualified = $"{atomType.Name}.{tr.identifier().GetText()}";
                if (TryLookupType(qualified, out var memberType))
                {
                    atomType = memberType;
                }
                else
                {
                    return base.VisitAtom_expr(context);
                }
            }
            else
            {
                return base.VisitAtom_expr(context);
            }
        }
        return atomType;
    }

    public override PilotType VisitType_alias_stmt(PilotParser.Type_alias_stmtContext context)
    {
        string name = context.identifier().GetText();
        if (context.adt_sum_body() is { } sumBody)
        {
            var adtType = new PilotType(PilotTypeKind.Primitive, name);
            SetVariableType(name, adtType);

            for (int i = 0; i < sumBody.ChildCount; i++)
            {
                var ch = sumBody.GetChild(i);
                if (ch is PilotParser.IdentifierContext unitId)
                {
                    string vName = unitId.GetText();
                    SetVariableType(vName, adtType);
                    SetVariableType($"{name}.{vName}", adtType);
                }
                else if (ch is PilotParser.Adt_variantContext vc)
                {
                    RegisterAdtVariantType(name, adtType, vc.identifier().GetText(), vc.adt_field_list(), vc.struct_member());
                }
                else if (ch is PilotParser.Adt_variant_payloadContext vpc)
                {
                    RegisterAdtVariantType(name, adtType, vpc.identifier().GetText(), vpc.adt_field_list(), vpc.struct_member());
                }
            }
            return adtType;
        }

        if (context.type_spec() != null)
        {
            var target = ParseTypeString(context.type_spec().GetText());
            SetVariableType(name, target);
            return target;
        }

        return PilotType.Void;
    }

    public override PilotType VisitEnum_def(PilotParser.Enum_defContext context)
    {
        string name = context.identifier().GetText();
        var enumType = new PilotType(PilotTypeKind.Primitive, name);
        SetVariableType(name, enumType);

        if (context.enum_suite()?.enum_member() is { } members)
        {
            foreach (var m in members)
            {
                RegisterAdtVariantType(name, enumType, m.identifier().GetText(), m.adt_field_list(), m.struct_member());
            }
        }
        return enumType;
    }

    public override PilotType VisitRecord_def(PilotParser.Record_defContext context)
    {
        string name = context.identifier().GetText();
        var recType = new PilotType(PilotTypeKind.Primitive, name);
        var fieldTypes = ExtractFieldTypes(context.adt_field_list(), context.struct_member());
        var ctorType = PilotType.FunctionOf(fieldTypes, recType);
        SetVariableType(name, ctorType);
        if (context.stmt() is { Length: > 0 } stmts)
        {
            PushScope();
            SetVariableType("self", recType);
            foreach (var st in stmts) Visit(st);
            PopScope();
        }
        return recType;
    }

    private void RegisterAdtVariantType(
        string adtName,
        PilotType adtType,
        string variantName,
        PilotParser.Adt_field_listContext? fieldList,
        PilotParser.Struct_memberContext[]? structMembers)
    {
        bool hasPayload = fieldList != null || (structMembers != null && structMembers.Length > 0);
        if (!hasPayload)
        {
            SetVariableType(variantName, adtType);
            SetVariableType($"{adtName}.{variantName}", adtType);
        }
        else
        {
            var fieldTypes = ExtractFieldTypes(fieldList, structMembers);
            var ctor = PilotType.FunctionOf(fieldTypes, adtType);
            SetVariableType(variantName, ctor);
            SetVariableType($"{adtName}.{variantName}", ctor);
        }
    }

    private static List<PilotType> ExtractFieldTypes(
        PilotParser.Adt_field_listContext? fieldList,
        PilotParser.Struct_memberContext[]? structMembers)
    {
        var types = new List<PilotType>();
        if (fieldList?.adt_field() != null)
        {
            foreach (var f in fieldList.adt_field())
            {
                types.Add(ParseTypeString(f.type_spec().GetText()));
            }
        }
        else if (structMembers != null)
        {
            foreach (var m in structMembers)
            {
                types.Add(ParseTypeString(m.type_annotation()?.type_spec()?.GetText() ?? "any"));
            }
        }
        return types;
    }
}
