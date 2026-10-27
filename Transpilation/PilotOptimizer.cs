using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using Antlr4.Runtime.Tree;

namespace Pilot.Transpilation;

/// <summary>
/// Shared compile-time optimization engine for both Pilot -> Target Language emission
/// and Foreign Language -> Pilot ingestion.
/// </summary>
public static partial class PilotOptimizer
{
    static PilotOptimizer()
    {
        if (Regex.CacheSize < 256)
            Regex.CacheSize = 256;
    }

    private static readonly Regex DurationLiteralRegex = MyRegex();
    private static readonly Regex MemoryLiteralRegex = MyRegex2();
    private static readonly Regex FrequencyLiteralRegex = MyRegex3();
    private static readonly Regex SimpleBinaryIntRegex = MyRegex4();
    private static readonly Regex StringConcatRegex = MyRegex5();
    private static readonly Regex RangeCallRegex = MyRegex6();

    /// <summary>
    /// Walks a Pilot AST subtree and returns the set of variable identifiers that are
    /// mutated (reassigned via '=', '+=', '-=', etc.) after their initial declaration.
    /// Allows target emitters (Rust, JS, C, C++) to emit immutable 'let' / 'const'
    /// whenever a variable is never reassigned.
    /// </summary>
    public static HashSet<string> AnalyzeMutatedVariables(IParseTree root)
    {
        var mutated = new HashSet<string>(StringComparer.Ordinal);
        CollectMutations(root, mutated);
        return mutated;
    }

    private static void CollectMutations(IParseTree node, HashSet<string> mutated)
    {
        if (node is PilotParser.Expr_stmtContext exprStmt)
        {
            // Check if this is an assignment or augmented assignment: lhs = rhs or lhs += rhs
            if (exprStmt.augassign() != null || exprStmt.EQUAL().Length > 0 || exprStmt.WALRUS().Length > 0)
            {
                var firstTestlist = exprStmt.testlist();
                if (firstTestlist != null)
                {
                    string lhs = firstTestlist.GetText().Trim();
                    // Extract base identifier if simple variable assignment
                    if (IsSimpleIdentifier(lhs))
                    {
                        mutated.Add(lhs);
                    }
                }
            }
        }
        else if (node is PilotParser.Var_stmtContext varStmt)
        {
            // Explicit 'var' signals mutable intent
            string name = varStmt.binding_pattern()?.GetText()?.Trim() ?? "";
            if (!string.IsNullOrEmpty(name))
            {
                mutated.Add(name);
            }
        }

        for (int i = 0; i < node.ChildCount; i++)
        {
            CollectMutations(node.GetChild(i), mutated);
        }
    }

    private static bool IsSimpleIdentifier(string s)
    {
        if (string.IsNullOrEmpty(s)) return false;
        char c0 = s[0];
        if (!((c0 >= 'a' && c0 <= 'z') || (c0 >= 'A' && c0 <= 'Z') || c0 == '_'))
            return false;
        for (int i = 1; i < s.Length; i++)
        {
            char c = s[i];
            if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_'))
                return false;
        }
        return true;
    }

    /// <summary>
    /// Folds constant arithmetic, dimensional literals (ms, s, KB, Hz), and boolean expressions.
    /// </summary>
    public static string FoldConstantsAndNormalize(string expr)
    {
        if (string.IsNullOrWhiteSpace(expr))
            return expr;

        string s = expr.Trim();

        // 1. Dimensional literal normalization (e.g., 50ms -> 50.0, 2s -> 2000.0, 64KB -> 65536)
        s = NormalizeDimensionalLiterals(s);

        // 2. Simple constant binary arithmetic folding: e.g., "2 + 3 * 4" or "10 * 20"
        if (TryFoldSimpleNumericExpression(s, out string folded))
        {
            return folded;
        }

        // 3. String literal concatenation folding: "hello " + "world" -> "hello world"
        if (s.StartsWith("\"", StringComparison.Ordinal) && s.Contains('+'))
        {
            var strConcatMatch = StringConcatRegex.Match(s);
            if (strConcatMatch.Success)
            {
                return $"\"{strConcatMatch.Groups[1].Value}{strConcatMatch.Groups[2].Value}\"";
            }
        }

        // 4. Boolean constant short-circuiting
        if (s.StartsWith("true and ", StringComparison.Ordinal))
            return s["true and ".Length..].Trim();
        if (s.StartsWith("false or ", StringComparison.Ordinal))
            return s["false or ".Length..].Trim();

        return s;
    }

    public static string NormalizeDimensionalLiterals(string expr)
    {
        bool hasDigit = false;
        for (int i = 0; i < expr.Length; i++)
        {
            if (char.IsDigit(expr[i])) { hasDigit = true; break; }
        }
        if (!hasDigit) return expr;

        // Duration literals: ns, us, ms, s, min, h -> milliseconds
        expr = DurationLiteralRegex.Replace(expr, m =>
        {
            double val = double.Parse(m.Groups[1].Value, CultureInfo.InvariantCulture);
            string unit = m.Groups[2].Value;
            double ms = unit switch
            {
                "ns" => val / 1_000_000.0,
                "us" => val / 1_000.0,
                "ms" => val,
                "s" => val * 1_000.0,
                "min" => val * 60_000.0,
                "h" => val * 3_600_000.0,
                _ => val
            };
            return ms % 1 == 0 ? ((long)ms).ToString(CultureInfo.InvariantCulture) : ms.ToString(CultureInfo.InvariantCulture);
        });

        // Memory literals: B, KB, MB, GB, TB -> bytes
        expr = MemoryLiteralRegex.Replace(expr, m =>
        {
            long val = long.Parse(m.Groups[1].Value, CultureInfo.InvariantCulture);
            string unit = m.Groups[2].Value;
            long bytes = unit switch
            {
                "B" => val,
                "KB" => val * 1024L,
                "MB" => val * 1024L * 1024L,
                "GB" => val * 1024L * 1024L * 1024L,
                "TB" => val * 1024L * 1024L * 1024L * 1024L,
                _ => val
            };
            return bytes.ToString(CultureInfo.InvariantCulture);
        });

        // Frequency literals: Hz, kHz, MHz, GHz -> Hz
        expr = FrequencyLiteralRegex.Replace(expr, m =>
        {
            double val = double.Parse(m.Groups[1].Value, CultureInfo.InvariantCulture);
            string unit = m.Groups[2].Value;
            double hz = unit switch
            {
                "Hz" => val,
                "kHz" => val * 1_000.0,
                "MHz" => val * 1_000_000.0,
                "GHz" => val * 1_000_000_000.0,
                _ => val
            };
            return hz % 1 == 0 ? ((long)hz).ToString(CultureInfo.InvariantCulture) : hz.ToString(CultureInfo.InvariantCulture);
        });

        return expr;
    }

    private static bool TryFoldSimpleNumericExpression(string expr, out string folded)
    {
        folded = expr;
        if (expr.Length == 0 || (!char.IsDigit(expr[0]) && expr[0] != '-'))
            return false;

        // Match pure integer binary op: e.g., 10 + 20, 12 * 4, 100 - 25
        var intMatch = SimpleBinaryIntRegex.Match(expr);
        if (intMatch.Success &&
            long.TryParse(intMatch.Groups[1].Value, NumberStyles.Integer, CultureInfo.InvariantCulture, out long a) &&
            long.TryParse(intMatch.Groups[3].Value, NumberStyles.Integer, CultureInfo.InvariantCulture, out long b))
        {
            long res = intMatch.Groups[2].Value switch
            {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                _ => 0
            };
            folded = res.ToString(CultureInfo.InvariantCulture);
            return true;
        }
        return false;
    }

    /// <summary>
    /// Checks if an expression is a Pilot inclusive range `start..end` and extracts its bounds.
    /// </summary>
    public static bool TryParseRangeExpr(string expr, out string startExpr, out string endExpr)
    {
        startExpr = "";
        endExpr = "";
        string trimmed = expr.Trim();
        int idx = trimmed.IndexOf("..", StringComparison.Ordinal);
        if (idx > 0 && idx + 2 < trimmed.Length && !trimmed.StartsWith("\"", StringComparison.Ordinal))
        {
            startExpr = trimmed[..idx].Trim();
            endExpr = trimmed[(idx + 2)..].Trim();
            return !string.IsNullOrEmpty(startExpr) && !string.IsNullOrEmpty(endExpr);
        }
        return false;
    }

    /// <summary>
    /// Checks if an expression is a Pilot `range(start, stop)` or `range(stop)` call.
    /// </summary>
    public static bool TryParseRangeCall(string expr, out string startExpr, out string stopExpr)
    {
        startExpr = "0";
        stopExpr = "";
        string trimmed = expr.Trim();
        if (!trimmed.StartsWith("range(", StringComparison.Ordinal))
            return false;

        var m = RangeCallRegex.Match(trimmed);
        if (!m.Success)
            return false;

        if (m.Groups[2].Success)
        {
            startExpr = m.Groups[1].Value.Trim();
            stopExpr = m.Groups[2].Value.Trim();
        }
        else
        {
            startExpr = "0";
            stopExpr = m.Groups[1].Value.Trim();
        }
        return true;
    }

    /// <summary>
    /// Post-processes ingested Pilot code to elevate verbose foreign idioms into
    /// optimized, idiomatic Pilot constructs (guard statements, native ranges, f-strings, const/let).
    /// </summary>
    public static string OptimizeIngestedPilotCode(string rawPilotCode)
    {
        var lines = rawPilotCode.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None);
        var optimized = new List<string>(lines.Length);

        for (int i = 0; i < lines.Length; i++)
        {
            string line = lines[i];
            string trimmed = line.Trim();
            int wsCount = 0;
            while (wsCount < line.Length && char.IsWhiteSpace(line[wsCount])) wsCount++;
            string indent = line[..wsCount];

            // 1. Elevate single-line early-exit 'if (!(cond)) { return ...; }' or 'if (!cond) { return ...; }' or 'if not cond { return ...; }' to 'guard'
            if (trimmed.StartsWith("if ", StringComparison.Ordinal))
            {
                var inlineGuardMatch = MyRegex1().Match(trimmed);
                if (inlineGuardMatch.Success)
                {
                    string cond = (inlineGuardMatch.Groups[1].Value + inlineGuardMatch.Groups[2].Value + inlineGuardMatch.Groups[3].Value).Trim();
                    string exitAction = inlineGuardMatch.Groups[4].Value.Trim().TrimEnd(';');
                    optimized.Add($"{indent}guard {cond} else {{ {exitAction} }}");
                    continue;
                }

                // 2. Elevate multi-line early-exit:
                //    if !cond { (or if (a < 0) {)
                //        return ...
                //    }
                if (i + 2 < lines.Length &&
                    MyRegex7().IsMatch(trimmed) &&
                    MyRegex8().IsMatch(lines[i + 1].Trim()) &&
                    lines[i + 2].Trim() == "}")
                {
                    var condMatch = MyRegex7().Match(trimmed);
                    string rawCond = condMatch.Groups[1].Value.Trim();
                    if (rawCond.StartsWith("(") && rawCond.EndsWith(")"))
                        rawCond = rawCond[1..^1].Trim();

                    string inverted = InvertConditionForGuard(rawCond);
                    string exitStmt = lines[i + 1].Trim().TrimEnd(';');
                    optimized.Add($"{indent}guard {inverted} else {{ {exitStmt} }}");
                    i += 2;
                    continue;
                }
            }

            if (trimmed.StartsWith("for ", StringComparison.Ordinal))
            {
                // 3. Elevate range(0, N) in 'for' loops to native inclusive range 0..(N-1) when N is integer literal
                var rangeForMatch = MyRegex9().Match(trimmed);
                if (rangeForMatch.Success &&
                    int.TryParse(rangeForMatch.Groups[2].Value, out int startVal) &&
                    int.TryParse(rangeForMatch.Groups[3].Value, out int endExclusive) &&
                    endExclusive > startVal)
                {
                    string loopVar = rangeForMatch.Groups[1].Value;
                    optimized.Add($"{indent}for {loopVar} in {startVal}..{endExclusive - 1} {{");
                    continue;
                }

                var singleRangeForMatch = MyRegex10().Match(trimmed);
                if (singleRangeForMatch.Success &&
                    int.TryParse(singleRangeForMatch.Groups[2].Value, out int stopVal) &&
                    stopVal > 0)
                {
                    string loopVar = singleRangeForMatch.Groups[1].Value;
                    optimized.Add($"{indent}for {loopVar} in 0..{stopVal - 1} {{");
                    continue;
                }
            }

            // 4. Strip redundant trailing semicolons for clean, idiomatic Pilot output
            if (trimmed.EndsWith(";") && !trimmed.StartsWith("//"))
            {
                line = line.TrimEnd();
                line = line[..^1];
            }

            optimized.Add(line);
        }

        return string.Join(Environment.NewLine, optimized).TrimEnd() + Environment.NewLine;
    }

    private static string InvertConditionForGuard(string cond)
    {
        if (cond.StartsWith("!"))
            return cond[1..].Trim();
        if (cond.StartsWith("not "))
            return cond["not ".Length..].Trim();
        if (cond.Contains(" == "))
            return cond.Replace(" == ", " != ");
        if (cond.Contains(" != "))
            return cond.Replace(" != ", " == ");
        if (cond.Contains(" < "))
            return cond.Replace(" < ", " >= ");
        if (cond.Contains(" <= "))
            return cond.Replace(" <= ", " > ");
        if (cond.Contains(" > "))
            return cond.Replace(" > ", " <= ");
        if (cond.Contains(" >= "))
            return cond.Replace(" >= ", " < ");
        return $"not ({cond})";
    }

    [GeneratedRegex(@"\b(\d+(?:\.\d+)?)(ns|us|ms|s|min|h)\b", RegexOptions.Compiled)]
    private static partial Regex MyRegex();
    [GeneratedRegex(@"^if\s+(?:\(\s*)?(?:!\((.+?)\)|!([A-Za-z_][A-Za-z0-9_]*)|not\s+\(?(.+?)\)?)(?:\s*\))?\s*\{\s*(return\b[^}]*|raise\b[^}]*|break\b[^}]*|continue\b[^}]*)\s*\}$")]
    private static partial Regex MyRegex1();
    [GeneratedRegex(@"\b(\d+)(B|KB|MB|GB|TB)\b", RegexOptions.Compiled)]
    private static partial Regex MyRegex2();
    [GeneratedRegex(@"\b(\d+(?:\.\d+)?)(Hz|kHz|MHz|GHz)\b", RegexOptions.Compiled)]
    private static partial Regex MyRegex3();
    [GeneratedRegex(@"^(-?\d+)\s*([+\-*])\s*(-?\d+)$", RegexOptions.Compiled)]
    private static partial Regex MyRegex4();
    [GeneratedRegex(@"^""([^""\\]*)""\s*\+\s*""([^""\\]*)""$", RegexOptions.Compiled)]
    private static partial Regex MyRegex5();
    [GeneratedRegex(@"^range\(\s*([^,)]+)(?:\s*,\s*([^,)]+))?\s*\)$", RegexOptions.Compiled)]
    private static partial Regex MyRegex6();
    [GeneratedRegex(@"^if\s+(.+?)\s*\{$")]
    private static partial Regex MyRegex7();
    [GeneratedRegex(@"^(return\b|raise\b)")]
    private static partial Regex MyRegex8();
    [GeneratedRegex(@"^for\s+([A-Za-z_][A-Za-z0-9_]*)\s+in\s+range\(\s*(\d+)\s*,\s*(\d+)\s*\)\s*\{$")]
    private static partial Regex MyRegex9();
    [GeneratedRegex(@"^for\s+([A-Za-z_][A-Za-z0-9_]*)\s+in\s+range\(\s*(\d+)\s*\)\s*\{$")]
    private static partial Regex MyRegex10();

}
