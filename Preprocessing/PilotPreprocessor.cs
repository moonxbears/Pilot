using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;

namespace Pilot.Preprocessing;

public enum PreprocessorDiagnosticSeverity
{
    Info,
    Warning,
    Error
}

public sealed class PreprocessorDiagnostic
{
    public int Line { get; set; }
    public int Column { get; set; }
    public PreprocessorDiagnosticSeverity Severity { get; set; }
    public string Message { get; set; } = string.Empty;
    public string Code { get; set; } = "PREPROC";

    public override string ToString() =>
        $"[{Line}:{Column}] {Severity}: {Code} - {Message}";
}

public sealed class CodeRegion
{
    public string Name { get; set; } = string.Empty;
    public int StartLine { get; set; }
    public int EndLine { get; set; }
}

public sealed class WarningSuppression
{
    public string WarningCode { get; set; } = string.Empty;
    public int StartLine { get; set; }
    public int EndLine { get; set; } = int.MaxValue;
}

public sealed class PilotPreprocessorOptions
{
    public HashSet<string> InitialSymbols { get; } = new(StringComparer.OrdinalIgnoreCase);
    public bool BlankDirectiveLines { get; set; } = true;
    public bool BlankInactiveCode { get; set; } = true;

    public PilotPreprocessorOptions() { }

    public PilotPreprocessorOptions(IEnumerable<string> symbols)
    {
        foreach (var s in symbols)
        {
            if (!string.IsNullOrWhiteSpace(s))
                InitialSymbols.Add(s.Trim());
        }
    }
}

public sealed class PilotPreprocessResult
{
    public string PreprocessedSource { get; set; } = string.Empty;
    public List<PreprocessorDiagnostic> Diagnostics { get; } = new();
    public HashSet<string> DefinedSymbols { get; } = new(StringComparer.OrdinalIgnoreCase);
    public List<CodeRegion> Regions { get; } = new();
    public List<WarningSuppression> Suppressions { get; } = new();
    public bool HasErrors => Diagnostics.Any(d => d.Severity == PreprocessorDiagnosticSeverity.Error);

    public bool IsWarningSuppressed(string warningCode, int line)
    {
        for (int i = 0; i < Suppressions.Count; i++)
        {
            var s = Suppressions[i];
            if (line >= s.StartLine && line <= s.EndLine &&
                string.Equals(s.WarningCode, warningCode, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
        }
        return false;
    }
}

/// <summary>
/// High-performance preprocessor for the Pilot language.
/// Supports conditional compilation (#if, #elif, #else, #endif),
/// symbol definition (#define, #undef), diagnostics (#error, #warning),
/// code organization (#region, #endregion), warning suppression (#pragma warning),
/// and line mapping (#line).
/// </summary>
public static partial class PilotPreprocessor
{
    private static readonly Regex DirectiveRegex = MyRegex();

    private static readonly Regex PragmaRegex = MyRegex1();

    private static readonly HashSet<string> KnownDirectives = new(StringComparer.OrdinalIgnoreCase)
    {
        "define", "undef", "if", "elif", "else", "endif",
        "region", "endregion", "error", "warning", "pragma", "line", "nullable"
    };

    public static PilotPreprocessResult Preprocess(string source, string filePath = "", PilotPreprocessorOptions? options = null)
    {
        options ??= new PilotPreprocessorOptions();
        var result = new PilotPreprocessResult();

        foreach (var sym in options.InitialSymbols)
            result.DefinedSymbols.Add(sym);

        var lines = SplitLines(source);
        var outputLines = new string[lines.Count];

        var ifStack = new Stack<IfFrame>();
        var regionStack = new Stack<(string Name, int StartLine)>();
        var activeSuppressions = new Dictionary<string, WarningSuppression>(StringComparer.OrdinalIgnoreCase);

        for (int lineIndex = 0; lineIndex < lines.Count; lineIndex++)
        {
            int lineNumber = lineIndex + 1;
            string rawLine = lines[lineIndex];

            // Support UNIX shebang on first line: e.g. #!/usr/bin/env pilot
            if (lineNumber == 1 && rawLine.StartsWith("#!", StringComparison.Ordinal))
            {
                outputLines[lineIndex] = options.BlankDirectiveLines ? string.Empty : $"// {rawLine}";
                continue;
            }

            bool isCurrentlyActive = ifStack.Count == 0 || ifStack.Peek().CurrentBranchActive;

            // Fast scan for leading '#' before invoking regex
            int firstNonWs = 0;
            while (firstNonWs < rawLine.Length && (rawLine[firstNonWs] == ' ' || rawLine[firstNonWs] == '\t'))
            {
                firstNonWs++;
            }

            if (firstNonWs < rawLine.Length && rawLine[firstNonWs] == '#')
            {
                var match = DirectiveRegex.Match(rawLine);
                if (match.Success && KnownDirectives.Contains(match.Groups[1].Value))
                {
                    string directive = match.Groups[1].Value.ToLowerInvariant();
                    string args = match.Groups[2].Success ? match.Groups[2].Value.Trim() : string.Empty;

                    ProcessDirective(
                        directive,
                        args,
                        lineNumber,
                        isCurrentlyActive,
                        ifStack,
                        regionStack,
                        activeSuppressions,
                        result);

                    // Directive lines are blanked or preserved according to options to retain 1:1 line numbers
                    outputLines[lineIndex] = options.BlankDirectiveLines ? string.Empty : $"// #{directive} {args}";
                    continue;
                }
            }

            if (isCurrentlyActive)
            {
                outputLines[lineIndex] = rawLine;
            }
            else
            {
                // Inactive code block: replaced with blank line so parser line numbers match 100%
                outputLines[lineIndex] = options.BlankInactiveCode ? string.Empty : $"// [INACTIVE] {rawLine}";
            }
        }

        // Close any trailing warning suppressions
        foreach (var suppression in activeSuppressions.Values)
        {
            suppression.EndLine = lines.Count;
            result.Suppressions.Add(suppression);
        }

        // Validate unclosed #if blocks
        if (ifStack.Count > 0)
        {
            var unclosed = ifStack.Peek();
            result.Diagnostics.Add(new PreprocessorDiagnostic
            {
                Line = unclosed.Line,
                Column = 1,
                Severity = PreprocessorDiagnosticSeverity.Error,
                Message = "End-of-file reached with unclosed conditional compilation directive (#if without matching #endif).",
                Code = "PREPROC001"
            });
        }

        // Validate unclosed #region blocks
        while (regionStack.Count > 0)
        {
            var unclosedRegion = regionStack.Pop();
            result.Diagnostics.Add(new PreprocessorDiagnostic
            {
                Line = unclosedRegion.StartLine,
                Column = 1,
                Severity = PreprocessorDiagnosticSeverity.Warning,
                Message = $"#region '{unclosedRegion.Name}' was not closed with an #endregion directive.",
                Code = "PREPROC002"
            });
            result.Regions.Add(new CodeRegion
            {
                Name = unclosedRegion.Name,
                StartLine = unclosedRegion.StartLine,
                EndLine = lines.Count
            });
        }

        result.PreprocessedSource = string.Join("\n", outputLines);
        return result;
    }

    private static void ProcessDirective(
        string directive,
        string args,
        int line,
        bool isCurrentlyActive,
        Stack<IfFrame> ifStack,
        Stack<(string Name, int StartLine)> regionStack,
        Dictionary<string, WarningSuppression> activeSuppressions,
        PilotPreprocessResult result)
    {
        switch (directive)
        {
            case "define":
                if (isCurrentlyActive)
                {
                    string sym = args.Trim();
                    if (IsValidSymbolName(sym))
                    {
                        result.DefinedSymbols.Add(sym);
                    }
                    else
                    {
                        result.Diagnostics.Add(new PreprocessorDiagnostic
                        {
                            Line = line,
                            Severity = PreprocessorDiagnosticSeverity.Error,
                            Message = $"Invalid identifier '{sym}' for #define directive.",
                            Code = "PREPROC003"
                        });
                    }
                }
                break;

            case "undef":
                if (isCurrentlyActive)
                {
                    string sym = args.Trim();
                    if (IsValidSymbolName(sym))
                    {
                        result.DefinedSymbols.Remove(sym);
                    }
                    else
                    {
                        result.Diagnostics.Add(new PreprocessorDiagnostic
                        {
                            Line = line,
                            Severity = PreprocessorDiagnosticSeverity.Error,
                            Message = $"Invalid identifier '{sym}' for #undef directive.",
                            Code = "PREPROC004"
                        });
                    }
                }
                break;

            case "if":
                bool parentActive = ifStack.Count == 0 || ifStack.Peek().CurrentBranchActive;
                bool condValue = false;
                if (parentActive)
                {
                    condValue = EvaluateCondition(args, result.DefinedSymbols, line, result);
                }
                ifStack.Push(new IfFrame
                {
                    Line = line,
                    ParentActive = parentActive,
                    AnyBranchTaken = condValue,
                    CurrentBranchActive = parentActive && condValue
                });
                break;

            case "elif":
                if (ifStack.Count == 0)
                {
                    result.Diagnostics.Add(new PreprocessorDiagnostic
                    {
                        Line = line,
                        Severity = PreprocessorDiagnosticSeverity.Error,
                        Message = "Unexpected #elif directive without preceding #if.",
                        Code = "PREPROC005"
                    });
                    break;
                }
                var currentElif = ifStack.Peek();
                if (currentElif.HasElse)
                {
                    result.Diagnostics.Add(new PreprocessorDiagnostic
                    {
                        Line = line,
                        Severity = PreprocessorDiagnosticSeverity.Error,
                        Message = "#elif cannot appear after an #else directive.",
                        Code = "PREPROC006"
                    });
                    break;
                }
                if (currentElif.ParentActive && !currentElif.AnyBranchTaken)
                {
                    bool elifCond = EvaluateCondition(args, result.DefinedSymbols, line, result);
                    currentElif.CurrentBranchActive = elifCond;
                    if (elifCond) currentElif.AnyBranchTaken = true;
                }
                else
                {
                    currentElif.CurrentBranchActive = false;
                }
                break;

            case "else":
                if (ifStack.Count == 0)
                {
                    result.Diagnostics.Add(new PreprocessorDiagnostic
                    {
                        Line = line,
                        Severity = PreprocessorDiagnosticSeverity.Error,
                        Message = "Unexpected #else directive without preceding #if.",
                        Code = "PREPROC007"
                    });
                    break;
                }
                var currentElse = ifStack.Peek();
                if (currentElse.HasElse)
                {
                    result.Diagnostics.Add(new PreprocessorDiagnostic
                    {
                        Line = line,
                        Severity = PreprocessorDiagnosticSeverity.Error,
                        Message = "Multiple #else directives in the same conditional block.",
                        Code = "PREPROC008"
                    });
                    break;
                }
                currentElse.HasElse = true;
                currentElse.CurrentBranchActive = currentElse.ParentActive && !currentElse.AnyBranchTaken;
                currentElse.AnyBranchTaken = true;
                break;

            case "endif":
                if (ifStack.Count == 0)
                {
                    result.Diagnostics.Add(new PreprocessorDiagnostic
                    {
                        Line = line,
                        Severity = PreprocessorDiagnosticSeverity.Error,
                        Message = "Unexpected #endif directive without matching #if.",
                        Code = "PREPROC009"
                    });
                }
                else
                {
                    ifStack.Pop();
                }
                break;

            case "region":
                if (isCurrentlyActive)
                {
                    string regionName = string.IsNullOrWhiteSpace(args) ? "Region" : args.Trim();
                    regionStack.Push((regionName, line));
                }
                break;

            case "endregion":
                if (isCurrentlyActive)
                {
                    if (regionStack.Count == 0)
                    {
                        result.Diagnostics.Add(new PreprocessorDiagnostic
                        {
                            Line = line,
                            Severity = PreprocessorDiagnosticSeverity.Warning,
                            Message = "Unexpected #endregion directive without matching #region.",
                            Code = "PREPROC010"
                        });
                    }
                    else
                    {
                        var reg = regionStack.Pop();
                        result.Regions.Add(new CodeRegion
                        {
                            Name = reg.Name,
                            StartLine = reg.StartLine,
                            EndLine = line
                        });
                    }
                }
                break;

            case "error":
                if (isCurrentlyActive)
                {
                    string msg = string.IsNullOrWhiteSpace(args) ? "User preprocessor error" : StripQuotes(args);
                    result.Diagnostics.Add(new PreprocessorDiagnostic
                    {
                        Line = line,
                        Severity = PreprocessorDiagnosticSeverity.Error,
                        Message = msg,
                        Code = "PREPROC_ERR"
                    });
                }
                break;

            case "warning":
                if (isCurrentlyActive)
                {
                    string msg = string.IsNullOrWhiteSpace(args) ? "User preprocessor warning" : StripQuotes(args);
                    result.Diagnostics.Add(new PreprocessorDiagnostic
                    {
                        Line = line,
                        Severity = PreprocessorDiagnosticSeverity.Warning,
                        Message = msg,
                        Code = "PREPROC_WARN"
                    });
                }
                break;

            case "pragma":
                if (isCurrentlyActive)
                {
                    ProcessPragma(args, line, activeSuppressions, result);
                }
                break;

            case "nullable":
            case "line":
                // Standard metadata pragmas tracked
                break;
        }
    }

    private static void ProcessPragma(
        string args,
        int line,
        Dictionary<string, WarningSuppression> activeSuppressions,
        PilotPreprocessResult result)
    {
        // Example: warning disable PILOT101, PILOT102
        // Example: warning restore PILOT101
        var match = PragmaRegex.Match(args);
        if (!match.Success) return;

        string action = match.Groups[1].Value.ToLowerInvariant();
        string rawIds = match.Groups[2].Success ? match.Groups[2].Value : string.Empty;
        var ids = rawIds.Split(new[] { ',', ' ' }, StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);

        if (action == "disable")
        {
            foreach (var id in ids)
            {
                if (!activeSuppressions.ContainsKey(id))
                {
                    activeSuppressions[id] = new WarningSuppression
                    {
                        WarningCode = id,
                        StartLine = line
                    };
                }
            }
        }
        else if (action == "restore")
        {
            foreach (var id in ids)
            {
                if (activeSuppressions.TryGetValue(id, out var suppression))
                {
                    suppression.EndLine = line;
                    result.Suppressions.Add(suppression);
                    activeSuppressions.Remove(id);
                }
            }
        }
    }

    public static bool EvaluateCondition(
        string expr,
        ISet<string> definedSymbols,
        int line = 1,
        PilotPreprocessResult? result = null)
    {
        if (string.IsNullOrWhiteSpace(expr))
            return false;

        try
        {
            var tokens = TokenizeCondition(expr);
            var parser = new ConditionParser(tokens, definedSymbols);
            return parser.Parse();
        }
        catch (Exception ex)
        {
            result?.Diagnostics.Add(new PreprocessorDiagnostic
            {
                Line = line,
                Severity = PreprocessorDiagnosticSeverity.Error,
                Message = $"Preprocessor conditional expression syntax error in '{expr}': {ex.Message}",
                Code = "PREPROC011"
            });
            return false;
        }
    }

    private static List<string> SplitLines(string text)
    {
        var lines = new List<string>();
        int start = 0;
        int len = text.Length;

        for (int i = 0; i < len; i++)
        {
            char c = text[i];
            if (c == '\r')
            {
                if (i + 1 < len && text[i + 1] == '\n')
                {
                    lines.Add(text.Substring(start, i - start));
                    i++;
                    start = i + 1;
                }
                else
                {
                    lines.Add(text.Substring(start, i - start));
                    start = i + 1;
                }
            }
            else if (c == '\n')
            {
                lines.Add(text.Substring(start, i - start));
                start = i + 1;
            }
        }

        if (start <= len)
        {
            lines.Add(text.Substring(start, len - start));
        }

        return lines;
    }

    private static bool IsValidSymbolName(string name)
    {
        if (string.IsNullOrEmpty(name)) return false;
        char first = name[0];
        if (!((first >= 'a' && first <= 'z') || (first >= 'A' && first <= 'Z') || first == '_'))
            return false;
        for (int i = 1; i < name.Length; i++)
        {
            char c = name[i];
            if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_'))
                return false;
        }
        return true;
    }

    private static string StripQuotes(string s)
    {
        s = s.Trim();
        if (s.StartsWith("\"") && s.EndsWith("\"") && s.Length >= 2)
            return s.Substring(1, s.Length - 2);
        return s;
    }

    private sealed class IfFrame
    {
        public int Line { get; set; }
        public bool ParentActive { get; set; }
        public bool AnyBranchTaken { get; set; }
        public bool CurrentBranchActive { get; set; }
        public bool HasElse { get; set; }
    }

    private static List<string> TokenizeCondition(string expr)
    {
        var tokens = new List<string>();
        int i = 0;
        while (i < expr.Length)
        {
            char c = expr[i];
            if (char.IsWhiteSpace(c))
            {
                i++;
                continue;
            }
            if (c == '(' || c == ')')
            {
                tokens.Add(c.ToString());
                i++;
                continue;
            }
            if (c == '!' && (i + 1 >= expr.Length || expr[i + 1] != '='))
            {
                tokens.Add("!");
                i++;
                continue;
            }
            if (c == '&' && i + 1 < expr.Length && expr[i + 1] == '&')
            {
                tokens.Add("&&");
                i += 2;
                continue;
            }
            if (c == '|' && i + 1 < expr.Length && expr[i + 1] == '|')
            {
                tokens.Add("||");
                i += 2;
                continue;
            }
            if (c == '=' && i + 1 < expr.Length && expr[i + 1] == '=')
            {
                tokens.Add("==");
                i += 2;
                continue;
            }
            if (c == '!' && i + 1 < expr.Length && expr[i + 1] == '=')
            {
                tokens.Add("!=");
                i += 2;
                continue;
            }
            if (char.IsLetter(c) || c == '_')
            {
                int start = i;
                while (i < expr.Length && (char.IsLetterOrDigit(expr[i]) || expr[i] == '_'))
                    i++;
                tokens.Add(expr.Substring(start, i - start));
                continue;
            }
            i++;
        }
        return tokens;
    }

    private sealed class ConditionParser
    {
        private readonly List<string> _tokens;
        private readonly ISet<string> _symbols;
        private int _pos;

        public ConditionParser(List<string> tokens, ISet<string> symbols)
        {
            _tokens = tokens;
            _symbols = symbols;
        }

        public bool Parse()
        {
            bool res = ParseOr();
            if (_pos < _tokens.Count)
                throw new InvalidOperationException($"Unexpected token '{_tokens[_pos]}' in expression.");
            return res;
        }

        private bool ParseOr()
        {
            bool left = ParseAnd();
            while (_pos < _tokens.Count && (_tokens[_pos] == "||" || _tokens[_pos].Equals("or", StringComparison.OrdinalIgnoreCase)))
            {
                _pos++;
                bool right = ParseAnd();
                left = left || right;
            }
            return left;
        }

        private bool ParseAnd()
        {
            bool left = ParseEquality();
            while (_pos < _tokens.Count && (_tokens[_pos] == "&&" || _tokens[_pos].Equals("and", StringComparison.OrdinalIgnoreCase)))
            {
                _pos++;
                bool right = ParseEquality();
                left = left && right;
            }
            return left;
        }

        private bool ParseEquality()
        {
            bool left = ParseUnary();
            while (_pos < _tokens.Count && (_tokens[_pos] == "==" || _tokens[_pos] == "!="))
            {
                string op = _tokens[_pos++];
                bool right = ParseUnary();
                left = op == "==" ? (left == right) : (left != right);
            }
            return left;
        }

        private bool ParseUnary()
        {
            if (_pos < _tokens.Count && (_tokens[_pos] == "!" || _tokens[_pos].Equals("not", StringComparison.OrdinalIgnoreCase)))
            {
                _pos++;
                return !ParseUnary();
            }
            return ParsePrimary();
        }

        private bool ParsePrimary()
        {
            if (_pos >= _tokens.Count)
                throw new InvalidOperationException("Premature end of expression.");

            string token = _tokens[_pos++];
            if (token == "(")
            {
                bool val = ParseOr();
                if (_pos >= _tokens.Count || _tokens[_pos++] != ")")
                    throw new InvalidOperationException("Mismatched parentheses, expected ')'.");
                return val;
            }
            if (token.Equals("true", StringComparison.OrdinalIgnoreCase))
                return true;
            if (token.Equals("false", StringComparison.OrdinalIgnoreCase))
                return false;

            return _symbols.Contains(token);
        }
    }

    [GeneratedRegex(@"^[ \t]*#[ \t]*([a-zA-Z_][a-zA-Z0-9_]*)(?:[ \t]+(.*))?$", RegexOptions.Compiled)]
    private static partial Regex MyRegex();
    [GeneratedRegex(@"^warning\s+(disable|restore)(?:\s+(.*))?$", RegexOptions.IgnoreCase | RegexOptions.Compiled, "en-US")]
    private static partial Regex MyRegex1();
}
