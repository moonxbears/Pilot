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
/// solves higher-order function types, performs dominator control-flow type narrowing,
/// infers structural row polymorphism, and emits LSP inlay hints.
/// </summary>
public sealed class PilotTypeInferenceEngine : PilotBaseVisitor<PilotType>
{
    private readonly Dictionary<string, PilotType> _environment = new(StringComparer.Ordinal);
    private readonly Stack<Dictionary<string, PilotType>> _scopes = new();
    private readonly Dictionary<int, PilotType> _substitutions = new();
    private readonly List<PilotAnalysisDiagnostic> _diagnostics = new();
    private readonly PilotBidirectionalInferencer _bidirectional;

    private static readonly HashSet<string> KnownUnitNames = new(StringComparer.OrdinalIgnoreCase)
    {
        "meter", "second", "speed", "kg", "gram", "hz", "rad", "deg", "joule", "watt", "volt", "amp", "ms", "us", "ns", "s", "m"
    };

    public IReadOnlyList<PilotAnalysisDiagnostic> Diagnostics => _diagnostics;
    public IReadOnlyDictionary<string, PilotType> InferredTypes => _environment;
    public Dictionary<int, PilotType> Substitutions => _substitutions;
    public PilotBidirectionalInferencer BidirectionalInferencer => _bidirectional;
    public IReadOnlyList<PilotTypeConstraint> Constraints => _bidirectional.Constraints;
    public IReadOnlyList<PilotMonomorphizedCallSite> MonomorphizedCallSites => _bidirectional.MonomorphizedCallSites;
    public IReadOnlyList<PilotInlayHint> InlayHints => _bidirectional.InlayHints;

    public PilotTypeInferenceEngine()
    {
        PushScope();
        RegisterBuiltins();
        _bidirectional = new PilotBidirectionalInferencer(this);
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
        _bidirectional.AnalyzeProgram(context);
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
        var resolved = ResolveType(type);
        if (_scopes.Count > 0)
        {
            _scopes.Peek()[name] = resolved;
        }
        _environment[name] = resolved;
    }

    public void RecordGlobalInferredType(string key, PilotType type)
    {
        _environment[key] = ResolveType(type);
    }

    public void ReportDiagnostic(string code, string message, int line, int column, bool isError = true)
    {
        if (!_diagnostics.Any(d => d.Code == code && d.Line == line && d.Message == message))
        {
            _diagnostics.Add(new PilotAnalysisDiagnostic(code, message, line, column, isError));
        }
    }

    public PilotType ResolveType(PilotType type) =>
        PilotType.Resolve(type, _substitutions);

    public PilotType? GetNarrowedTypeAt(ParserRuleContext node) =>
        _bidirectional.GetNarrowedTypeAt(node);

    public PilotType? GetFlowTypeAtLine(int oneBasedLine, string variableName) =>
        _bidirectional.GetFlowTypeAtLine(oneBasedLine, variableName);

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
            var parts = SplitTopLevelByChar(typeStr, '|').Select(p => ParseTypeString(p.Trim())).ToArray();
            if (parts.Length > 1)
                return PilotType.UnionOf(parts);
        }

        // Function type `(T1, T2) -> R`
        int arrowIdx = FindTopLevelArrow(typeStr);
        if (arrowIdx > 0)
        {
            string paramPart = typeStr[..arrowIdx].Trim();
            string retPart = typeStr[(arrowIdx + 2)..].Trim();
            if (paramPart.StartsWith('(') && paramPart.EndsWith(')'))
                paramPart = paramPart[1..^1].Trim();
            var paramTypes = string.IsNullOrWhiteSpace(paramPart)
                ? Array.Empty<PilotType>()
                : SplitTopLevelByChar(paramPart, ',').Select(p => ParseTypeString(p.Trim())).ToArray();
            return PilotType.FunctionOf(paramTypes, ParseTypeString(retPart));
        }

        if ((typeStr.StartsWith("List<", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith(">")) ||
            (typeStr.StartsWith("List[", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith("]")))
        {
            var inner = ParseTypeString(typeStr[5..^1]);
            return PilotType.ListOf(inner);
        }

        if ((typeStr.StartsWith("SliceBuffer<", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith(">")) ||
            (typeStr.StartsWith("SliceBuffer[", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith("]")))
        {
            var inner = ParseTypeString(typeStr[12..^1]);
            return new PilotType(PilotTypeKind.Collection, "SliceBuffer", new[] { inner });
        }

        if ((typeStr.StartsWith("SharedBuffer<", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith(">")) ||
            (typeStr.StartsWith("SharedBuffer[", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith("]")))
        {
            var inner = ParseTypeString(typeStr[13..^1]);
            return new PilotType(PilotTypeKind.Collection, "SharedBuffer", new[] { inner });
        }

        if ((typeStr.StartsWith("Set<", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith(">")) ||
            (typeStr.StartsWith("Set[", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith("]")))
        {
            var inner = ParseTypeString(typeStr[4..^1]);
            return PilotType.SetOf(inner);
        }

        if ((typeStr.StartsWith("Future<", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith(">")) ||
            (typeStr.StartsWith("Future[", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith("]")))
        {
            var inner = ParseTypeString(typeStr[7..^1]);
            return PilotType.FutureOf(inner);
        }

        if ((typeStr.StartsWith("Option<", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith(">")) ||
            (typeStr.StartsWith("Option[", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith("]")))
        {
            var inner = ParseTypeString(typeStr[7..^1]);
            return PilotType.OptionOf(inner);
        }

        if ((typeStr.StartsWith("Map<", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith(">")) ||
            (typeStr.StartsWith("Map[", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith("]")) ||
            (typeStr.StartsWith("Dict<", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith(">")) ||
            (typeStr.StartsWith("Dict[", StringComparison.OrdinalIgnoreCase) && typeStr.EndsWith("]")))
        {
            int openIdx = typeStr.IndexOfAny(new[] { '<', '[' });
            var innerParts = SplitTopLevelByChar(typeStr[(openIdx + 1)..^1], ',');
            var k = innerParts.Count > 0 ? ParseTypeString(innerParts[0].Trim()) : PilotType.Any;
            var v = innerParts.Count > 1 ? ParseTypeString(innerParts[1].Trim()) : PilotType.Any;
            return PilotType.MapOf(k, v);
        }

        if (typeStr.StartsWith("(") && typeStr.EndsWith(")") && !typeStr.Contains("->"))
        {
            var parts = SplitTopLevelByChar(typeStr[1..^1], ',').Select(p => ParseTypeString(p.Trim())).ToArray();
            return PilotType.TupleOf(parts);
        }

        // Typestate or Unit suffix: `Conn[Closed]` or `float<meter>`
        int sqOpen = typeStr.IndexOf('[');
        int sqClose = typeStr.LastIndexOf(']');
        if (sqOpen > 0 && sqClose == typeStr.Length - 1)
        {
            string basePart = typeStr[..sqOpen].Trim();
            string statePart = typeStr[(sqOpen + 1)..sqClose].Trim();
            return PilotType.WithTypestate(ParseTypeString(basePart), statePart);
        }

        int unitOpen = typeStr.IndexOf('<');
        int unitClose = typeStr.LastIndexOf('>');
        if (unitOpen > 0 && unitClose > unitOpen)
        {
            string basePart = typeStr[..unitOpen].Trim();
            string unitPart = typeStr[(unitOpen + 1)..unitClose].Trim();
            var parsedBase = ParseTypeString(basePart);
            if (parsedBase.IsNumericType || KnownUnitNames.Contains(unitPart) || unitPart.Contains('/') || unitPart.Contains('*'))
            {
                return PilotType.WithUnit(parsedBase, unitPart);
            }
            return PilotType.WithTypestate(parsedBase, unitPart);
        }

        return typeStr.ToLowerInvariant() switch
        {
            "int" or "i32" or "i64" or "long" => PilotType.Int,
            "float" or "f32" or "f64" or "double" => PilotType.Float,
            "str" or "string" => PilotType.String,
            "bool" or "boolean" => PilotType.Bool,
            "void" => PilotType.Void,
            "null" or "none" => PilotType.Null,
            "any" => PilotType.Any,
            _ => new PilotType(PilotTypeKind.Primitive, typeStr)
        };
    }

    private static List<string> SplitTopLevelByChar(string input, char delimiter)
    {
        var result = new List<string>();
        int depth = 0;
        int start = 0;
        for (int i = 0; i < input.Length; i++)
        {
            char c = input[i];
            if (c is '<' or '[' or '(' or '{') depth++;
            else if (c is '>' or ']' or ')' or '}') depth = Math.Max(0, depth - 1);
            else if (c == delimiter && depth == 0)
            {
                result.Add(input[start..i]);
                start = i + 1;
            }
        }
        result.Add(input[start..]);
        return result;
    }

    private static int FindTopLevelArrow(string input)
    {
        int depth = 0;
        for (int i = 0; i < input.Length - 1; i++)
        {
            char c = input[i];
            if (c is '<' or '[' or '(' or '{') depth++;
            else if (c is '>' or ']' or ')' or '}') depth = Math.Max(0, depth - 1);
            else if (depth == 0 && c == '-' && input[i + 1] == '>')
                return i;
        }
        return -1;
    }
}
