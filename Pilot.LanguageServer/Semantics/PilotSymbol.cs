using System;
using System.Collections.Generic;
using System.Linq;

namespace Pilot.LanguageServer.Semantics;

public enum PilotSymbolKind
{
    Variable,
    Parameter,
    Function,
    Method,
    Class,
    Struct,
    Interface,
    Enum,
    EnumMember,
    Actor,
    Model,
    Table,
    Database,
    Property,
    Operator,
    Extension,
    Unit,
    Dsl,
    Macro,
    TypeAlias,
    TypeParameter,
    QueryVariable,
    Savepoint,
    Module
}

public enum PilotDiagnosticSeverity
{
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4
}

public sealed class PilotCodeAction
{
    public string Title { get; init; } = string.Empty;
    public string Kind { get; init; } = "quickfix";
    public bool IsPreferred { get; init; } = true;
    public string? NewText { get; init; }
    public int StartLine { get; init; }
    public int StartColumn { get; init; }
    public int EndLine { get; init; }
    public int EndColumn { get; init; }
}

public sealed class PilotDiagnostic
{
    public int StartLine { get; init; }
    public int StartColumn { get; init; }
    public int EndLine { get; init; }
    public int EndColumn { get; init; }
    public PilotDiagnosticSeverity Severity { get; init; }
    public string Code { get; init; } = "PILOT000";
    public string Message { get; init; } = string.Empty;
    public string Source { get; init; } = "pilot";
    public List<PilotCodeAction> CodeActions { get; } = new();
}

public static class PilotSemanticTokenLegend
{
    public static readonly string[] TokenTypes = new[]
    {
        "namespace",     // 0
        "type",          // 1
        "class",         // 2
        "enum",          // 3
        "interface",     // 4
        "struct",        // 5
        "typeParameter", // 6
        "parameter",     // 7
        "variable",      // 8
        "property",      // 9
        "enumMember",    // 10
        "event",         // 11
        "function",      // 12
        "method",        // 13
        "macro",         // 14
        "keyword",       // 15
        "modifier",      // 16
        "comment",       // 17
        "string",        // 18
        "number",        // 19
        "regexp",        // 20
        "operator",      // 21
        "decorator"      // 22
    };

    public static readonly string[] TokenModifiers = new[]
    {
        "declaration",    // bit 0 (1)
        "definition",     // bit 1 (2)
        "readonly",       // bit 2 (4)
        "static",         // bit 3 (8)
        "deprecated",     // bit 4 (16)
        "abstract",       // bit 5 (32)
        "async",          // bit 6 (64)
        "modification",   // bit 7 (128)
        "documentation",  // bit 8 (256)
        "defaultLibrary"  // bit 9 (512)
    };

    public const int TypeNamespace = 0;
    public const int TypeType = 1;
    public const int TypeClass = 2;
    public const int TypeEnum = 3;
    public const int TypeInterface = 4;
    public const int TypeStruct = 5;
    public const int TypeTypeParameter = 6;
    public const int TypeParameter = 7;
    public const int TypeVariable = 8;
    public const int TypeProperty = 9;
    public const int TypeEnumMember = 10;
    public const int TypeEvent = 11;
    public const int TypeFunction = 12;
    public const int TypeMethod = 13;
    public const int TypeMacro = 14;
    public const int TypeKeyword = 15;
    public const int TypeModifier = 16;
    public const int TypeString = 18;
    public const int TypeNumber = 19;
    public const int TypeOperator = 21;
    public const int TypeDecorator = 22;

    public const int ModDeclaration = 1 << 0;
    public const int ModDefinition = 1 << 1;
    public const int ModReadonly = 1 << 2;
    public const int ModStatic = 1 << 3;
    public const int ModAsync = 1 << 6;
    public const int ModModification = 1 << 7;
    public const int ModDefaultLibrary = 1 << 9;
}

public sealed class PilotSemanticToken
{
    public int Line { get; init; }
    public int StartColumn { get; init; }
    public int Length { get; init; }
    public int TokenType { get; init; }
    public int TokenModifiers { get; init; }
}

public sealed class PilotSymbol
{
    public string Name { get; init; } = string.Empty;
    public PilotSymbolKind Kind { get; init; }
    public string TypeName { get; set; } = "any";
    public bool IsMutable { get; init; }
    public bool IsConstant { get; init; }
    public bool IsAsync { get; init; }
    public bool IsPure { get; init; }
    public bool IsStatic { get; init; }
    public bool IsParams { get; init; }
    public bool IsHardRealtime { get; init; }
    public bool IsSoftRealtime { get; init; }
    public string? WcetBudget { get; init; }
    public string Domain { get; init; } = "Core";
    public int ReferenceCount { get; set; }

    // 0-based LSP coordinates for the identifier token
    public int SelectionLine { get; init; }
    public int SelectionColumn { get; init; }
    public int SelectionEndColumn { get; init; }

    // 0-based LSP coordinates for the full construct (for DocumentSymbol range)
    public int StartLine { get; init; }
    public int StartColumn { get; init; }
    public int EndLine { get; init; }
    public int EndColumn { get; init; }

    public string Signature { get; set; } = string.Empty;
    public string Documentation { get; set; } = string.Empty;

    public List<(string Name, string Type)> Parameters { get; } = new();
    public List<string> GenericParameters { get; } = new();
    public List<string> BaseTypes { get; } = new();
    public List<string> IndexedFields { get; } = new();
    public Dictionary<string, PilotSymbol> Members { get; } = new(StringComparer.Ordinal);
    public List<PilotSymbol> Children { get; } = new();

    public int GetLspDocumentSymbolKind() => Kind switch
    {
        PilotSymbolKind.Class => 5,          // Class
        PilotSymbolKind.Actor => 5,          // Class (Actor)
        PilotSymbolKind.Model => 23,         // Struct/Model
        PilotSymbolKind.Table => 23,         // Table
        PilotSymbolKind.Database or PilotSymbolKind.Module or PilotSymbolKind.Extension or PilotSymbolKind.Dsl => 2,       // Module/Namespace
        PilotSymbolKind.Struct => 23,        // Struct
        PilotSymbolKind.Interface => 11,     // Interface
        PilotSymbolKind.Enum => 10,          // Enum
        PilotSymbolKind.EnumMember => 22,    // EnumMember
        PilotSymbolKind.Function => 12,      // Function
        PilotSymbolKind.Method => 6,         // Method
        PilotSymbolKind.Property => 7,       // Property
        PilotSymbolKind.Operator => 25,      // Operator
        PilotSymbolKind.Unit => 26,          // TypeParameter/Unit
        PilotSymbolKind.Macro => 12,         // Function/Macro
        PilotSymbolKind.TypeAlias => 26,     // TypeParameter
        PilotSymbolKind.Parameter => 13,     // Variable
        PilotSymbolKind.Variable => IsConstant ? 14 : 13, // Constant : Variable
        _ => 13
    };

    public string FormatHoverMarkdown()
    {
        var lines = new List<string>
        {
            "```pilot",
            string.IsNullOrWhiteSpace(Signature) ? $"{Kind.ToString().ToLowerInvariant()} {Name}: {TypeName}" : Signature,
            "```"
        };

        var badges = new List<string> { $"**Domain**: `{Domain}`" };
        if (Kind == PilotSymbolKind.Variable)
        {
            badges.Add(IsConstant ? "**Binding**: `const` (compile-time constant)"
                : IsMutable ? "**Binding**: `var` (mutable)"
                : "**Binding**: `let` (immutable)");
        }
        if (IsAsync) badges.Add("**Concurrency**: `async`");
        if (IsPure) badges.Add("**Purity**: `pure`");
        if (IsHardRealtime) badges.Add($"**Real-Time**: `@hard_realtime`{(WcetBudget != null ? $" (WCET: `{WcetBudget}`)" : "")}");
        if (IsSoftRealtime) badges.Add("**Real-Time**: `@soft_realtime`");
        if (IndexedFields.Count > 0) badges.Add($"**Indexes**: `{string.Join(", ", IndexedFields)}`");

        lines.Add(string.Join(" | ", badges));

        if (!string.IsNullOrWhiteSpace(Documentation))
        {
            lines.Add(string.Empty);
            lines.Add(Documentation);
        }

        if (Members.Count > 0)
        {
            lines.Add(string.Empty);
            lines.Add("**Members:**");
            foreach (var member in Members.Values.Take(12))
            {
                lines.Add($"- `{member.Name}`: `{member.TypeName}` ({member.Kind})");
            }
        }

        return string.Join("\n", lines);
    }
}

public sealed class PilotSymbolReference
{
    public int Line { get; init; }
    public int StartColumn { get; init; }
    public int EndColumn { get; init; }
    public PilotSymbol Symbol { get; init; } = null!;
}

public sealed class PilotScope
{
    public string Name { get; init; } = "global";
    public PilotScope? Parent { get; init; }
    public List<PilotScope> Children { get; } = new();
    public Dictionary<string, PilotSymbol> Symbols { get; } = new(StringComparer.Ordinal);

    public int StartLine { get; init; }
    public int EndLine { get; set; } = int.MaxValue;

    public bool InHardRealtime { get; init; }
    public bool InSoftRealtime { get; init; }
    public bool InNoAlloc { get; init; }
    public bool InAsync { get; init; }
    public bool InLoop { get; init; }
    public bool InFunction { get; init; }
    public bool InActor { get; init; }
    public bool IsGenerator { get; set; }
    public bool InTransaction { get; init; }
    public string? EnclosingReturnType { get; init; }
    public PilotSymbol? EnclosingType { get; init; }

    public PilotSymbol? Resolve(string name)
    {
        for (var scope = this; scope != null; scope = scope.Parent)
        {
            if (scope.Symbols.TryGetValue(name, out var symbol))
                return symbol;
        }
        return null;
    }

    public IEnumerable<PilotSymbol> GetAllVisibleSymbols()
    {
        var seen = new HashSet<string>(StringComparer.Ordinal);
        for (var scope = this; scope != null; scope = scope.Parent)
        {
            foreach (var kvp in scope.Symbols)
            {
                if (seen.Add(kvp.Key))
                    yield return kvp.Value;
            }
        }
    }
}

public sealed class PilotAnalysisResult
{
    public string Uri { get; init; } = string.Empty;
    public PilotScope RootScope { get; init; } = new();
    public List<PilotDiagnostic> Diagnostics { get; } = new();
    public List<PilotSemanticToken> SemanticTokens { get; } = new();
    public List<PilotSymbolReference> References { get; } = new();
    public List<PilotSymbol> TopLevelSymbols { get; } = new();
    public List<Pilot.Preprocessing.CodeRegion> Regions { get; init; } = new();

    public PilotSymbol? FindSymbolAtPosition(int line, int character)
    {
        for (int i = 0; i < References.Count; i++)
        {
            var r = References[i];
            if (r.Line == line && character >= r.StartColumn && character <= r.EndColumn)
                return r.Symbol;
        }
        return null;
    }

    public PilotScope FindScopeAtLine(int line)
    {
        return FindDeepestScope(RootScope, line);
    }

    private static PilotScope FindDeepestScope(PilotScope current, int line)
    {
        foreach (var child in current.Children)
        {
            if (line >= child.StartLine && line <= child.EndLine)
                return FindDeepestScope(child, line);
        }
        return current;
    }

    public int[] EncodeSemanticTokens()
    {
        if (SemanticTokens.Count == 0)
            return Array.Empty<int>();

        // Deduplicate by (Line, StartColumn) keeping the last emitted token, then sort
        var uniqueByPos = new Dictionary<(int Line, int Col), PilotSemanticToken>(SemanticTokens.Count);
        for (int i = 0; i < SemanticTokens.Count; i++)
        {
            var t = SemanticTokens[i];
            if (t.Length > 0 && t.Line >= 0 && t.StartColumn >= 0)
            {
                uniqueByPos[(t.Line, t.StartColumn)] = t;
            }
        }

        var sorted = new List<PilotSemanticToken>(uniqueByPos.Values);
        sorted.Sort(static (a, b) =>
        {
            int cmp = a.Line.CompareTo(b.Line);
            return cmp != 0 ? cmp : a.StartColumn.CompareTo(b.StartColumn);
        });

        var data = new List<int>(sorted.Count * 5);
        int prevLine = 0;
        int prevChar = 0;

        for (int i = 0; i < sorted.Count; i++)
        {
            var tok = sorted[i];
            int deltaLine = tok.Line - prevLine;
            int deltaStart = deltaLine == 0 ? tok.StartColumn - prevChar : tok.StartColumn;
            if (deltaStart < 0) continue;

            data.Add(deltaLine);
            data.Add(deltaStart);
            data.Add(tok.Length);
            data.Add(tok.TokenType);
            data.Add(tok.TokenModifiers);

            prevLine = tok.Line;
            prevChar = tok.StartColumn;
        }

        return data.ToArray();
    }
}
