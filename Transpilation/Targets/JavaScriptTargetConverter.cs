using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using Antlr4.Runtime;
using Pilot.Diagnostics;
using Pilot.Preprocessing;

namespace Pilot.Transpilation.Targets;

/// <summary>
/// Bidirectional optimizing compile-time code converter between Pilot and modern JavaScript (ES2024+).
/// Emits immutability-aware const/let bindings, strict equality, template literals, and zero-allocation numeric loops.
/// </summary>
public sealed class JavaScriptTargetConverter : ITargetLanguageConverter
{
    public string Language { get; }
    public string TargetLanguage => Language;
    public string SourceLanguage => Language;

    public JavaScriptTargetConverter(string alias = "js")
    {
        Language = alias;
    }

    public TranspilationResult TranspileSource(string pilotSource, TranspilationOptions options)
    {
        var preproc = PilotPreprocessor.Preprocess(pilotSource, "");
        if (preproc.HasErrors)
        {
            return TranspilationResult.CreateFailure(
                preproc.Diagnostics.Select(d => d.Message),
                TargetLanguage);
        }

        var inputStream = new AntlrInputStream(preproc.PreprocessedSource);
        var lexer = new PilotLexer(inputStream);
        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);

        var errorTracker = new ErrorTracker();
        parser.RemoveErrorListeners();
        parser.AddErrorListener(errorTracker);

        var tree = parser.ParseProgramFast();
        if (errorTracker.HasErrors)
        {
            return TranspilationResult.CreateFailure(errorTracker.Errors, TargetLanguage);
        }

        return Transpile(tree, options);
    }

    public TranspilationResult Transpile(PilotParser.ProgramContext programAst, TranspilationOptions options)
    {
        var sb = new StringBuilder();
        if (options.EmitHeaderComments)
        {
            sb.AppendLine("// Code transpiled & optimized from Pilot to JavaScript (ES2024+)");
            sb.AppendLine("\"use strict\";");
            sb.AppendLine();
        }

        var mutatedVars = PilotOptimizer.AnalyzeMutatedVariables(programAst);
        var visitor = new PilotToJsVisitor(options, mutatedVars);
        string code = visitor.VisitProgram(programAst);
        sb.Append(code);

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options)
    {
        var sb = new StringBuilder();
        if (options.EmitHeaderComments)
        {
            sb.AppendLine("// Converted & optimized from JavaScript to Pilot");
            sb.AppendLine();
        }

        var lines = foreignSource.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None);
        foreach (var line in lines)
        {
            string trimmed = line.Trim();
            if (trimmed == "\"use strict\";" || trimmed == "'use strict';")
                continue;

            int indent = line.TakeWhile(char.IsWhiteSpace).Count();
            string pad = new string(' ', indent);

            if (string.IsNullOrWhiteSpace(trimmed))
            {
                sb.AppendLine();
                continue;
            }

            string converted = ConvertJsLineToPilot(trimmed);
            sb.AppendLine($"{pad}{converted}");
        }

        string optimized = PilotOptimizer.OptimizeIngestedPilotCode(sb.ToString());
        return TranspilationResult.CreateSuccess(optimized, "pilot");
    }

    private static string ConvertJsLineToPilot(string line)
    {
        string res = line.TrimEnd(';');

        // Strict equality -> Pilot equality
        res = res.Replace("===", "==").Replace("!==", "!=");

        // Template literals `Hello ${x}` -> f"Hello {x}"
        if (res.Contains('`'))
        {
            res = Regex.Replace(res, @"`([^`]*)`", m =>
            {
                string inner = Regex.Replace(m.Groups[1].Value, @"\$\{([^}]+)\}", "{$1}");
                return $"f\"{inner}\"";
            });
        }

        // console.log -> println
        res = res.Replace("console.log(", "println(");

        // function foo(a, b) { -> def foo(a, b) {
        if (res.StartsWith("function "))
        {
            res = "def " + res["function ".Length..];
        }
        else if (res.StartsWith("async function "))
        {
            res = "async def " + res["async function ".Length..];
        }

        // C-style for loop: for (let i = a; i <= b; i++) { -> for i in a..b {
        var forIncr = Regex.Match(res, @"^for\s*\(\s*(?:let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([^;]+);\s*\1\s*(<=|<)\s*([^;]+);\s*\1\+\+\s*\)\s*\{?$");
        if (forIncr.Success)
        {
            string v = forIncr.Groups[1].Value;
            string start = forIncr.Groups[2].Value.Trim();
            string op = forIncr.Groups[3].Value;
            string end = forIncr.Groups[4].Value.Trim();
            if (op == "<" && int.TryParse(end, out int endNum))
                end = (endNum - 1).ToString();
            else if (op == "<")
                end = $"({end} - 1)";
            return $"for {v} in {start}..{end} {{";
        }

        // for (const x of items) { -> for x in items {
        var forOf = Regex.Match(res, @"^for\s*\(\s*(?:const|let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s+of\s+(.+)\)\s*\{?$");
        if (forOf.Success)
        {
            return $"for {forOf.Groups[1].Value} in {forOf.Groups[2].Value.Trim()} {{";
        }

        return res;
    }

    private sealed class PilotToJsVisitor
    {
        private readonly TranspilationOptions _options;
        private readonly HashSet<string> _mutatedVars;
        private int _indent;

        public PilotToJsVisitor(TranspilationOptions options, HashSet<string> mutatedVars)
        {
            _options = options;
            _mutatedVars = mutatedVars;
        }

        private string Indent => string.Concat(Enumerable.Repeat(_options.IndentString, _indent));

        public string VisitProgram(PilotParser.ProgramContext ctx)
        {
            var sb = new StringBuilder();
            foreach (var stmt in ctx.stmt())
            {
                string s = VisitStmt(stmt);
                if (!string.IsNullOrWhiteSpace(s))
                {
                    sb.AppendLine(s);
                }
            }
            return sb.ToString();
        }

        public string VisitStmt(PilotParser.StmtContext ctx)
        {
            if (ctx.compound_stmt() != null) return VisitCompoundStmt(ctx.compound_stmt());
            if (ctx.simple_stmt() != null) return VisitSimpleStmt(ctx.simple_stmt());
            return string.Empty;
        }

        public string VisitSimpleStmt(PilotParser.Simple_stmtContext ctx)
        {
            var sb = new StringBuilder();
            foreach (var small in ctx.small_stmt())
            {
                string s = VisitSmallStmt(small);
                if (!string.IsNullOrWhiteSpace(s))
                {
                    sb.AppendLine(Indent + s);
                }
            }
            return sb.ToString().TrimEnd();
        }

        public string VisitSmallStmt(PilotParser.Small_stmtContext ctx)
        {
            if (ctx.let_stmt() != null)
            {
                string name = ctx.let_stmt().binding_pattern().GetText();
                string? val = ctx.let_stmt().test()?.GetText();
                // Optimization: emit 'const' if variable is never mutated
                string kw = (_mutatedVars.Contains(name) || val == null) ? "let" : "const";
                return val != null ? $"{kw} {name} = {TranslateExpr(val)};" : $"let {name} = null;";
            }
            if (ctx.var_stmt() != null)
            {
                string name = ctx.var_stmt().binding_pattern().GetText();
                string? val = ctx.var_stmt().test()?.GetText();
                return val != null ? $"let {name} = {TranslateExpr(val)};" : $"let {name} = null;";
            }
            if (ctx.const_stmt() != null)
            {
                string name = ctx.const_stmt().binding_pattern().GetText();
                string val = ctx.const_stmt().test().GetText();
                return $"const {name} = {TranslateExpr(val)};";
            }
            if (ctx.guard_stmt() != null)
            {
                return VisitGuardStmt(ctx.guard_stmt(), inlineIndent: false);
            }
            if (ctx.flow_stmt() != null)
            {
                var flow = ctx.flow_stmt();
                if (flow.RETURN() != null)
                {
                    string ret = flow.testlist() != null ? TranslateExpr(flow.testlist().GetText()) : "";
                    return string.IsNullOrEmpty(ret) ? "return;" : $"return {ret};";
                }
                if (flow.BREAK() != null) return "break;";
                if (flow.CONTINUE() != null) return "continue;";
                if (flow.RAISE() != null)
                {
                    string err = flow.test() != null && flow.test().Length > 0 ? TranslateExpr(flow.test(0).GetText()) : "\"Error\"";
                    return $"throw new Error({err});";
                }
            }
            if (ctx.assert_stmt() != null)
            {
                var tests = ctx.assert_stmt().test();
                string cond = TranslateExpr(tests[0].GetText());
                string msg = tests.Length > 1 ? TranslateExpr(tests[1].GetText()) : $"\"Assertion failed: {tests[0].GetText()}\"";
                return $"if (!({cond})) throw new Error({msg});";
            }
            if (ctx.expr_stmt() != null)
            {
                return TranslateExpr(ctx.expr_stmt().GetText().TrimEnd(';')) + ";";
            }
            return ctx.GetText().TrimEnd(';') + ";";
        }

        public string VisitCompoundStmt(PilotParser.Compound_stmtContext ctx)
        {
            if (ctx.func_def() != null) return VisitFuncDef(ctx.func_def());
            if (ctx.class_def() != null) return VisitClassDef(ctx.class_def());
            if (ctx.if_stmt() != null) return VisitIfStmt(ctx.if_stmt());
            if (ctx.while_stmt() != null) return VisitWhileStmt(ctx.while_stmt());
            if (ctx.for_stmt() != null) return VisitForStmt(ctx.for_stmt());
            if (ctx.guard_stmt() != null) return VisitGuardStmt(ctx.guard_stmt(), inlineIndent: true);
            if (ctx.match_stmt() != null) return VisitMatchStmt(ctx.match_stmt());
            if (ctx.inline_foreign_block() != null) return VisitForeignBlock(ctx.inline_foreign_block());
            return Indent + "// " + ctx.GetText();
        }

        private string VisitGuardStmt(PilotParser.Guard_stmtContext ctx, bool inlineIndent)
        {
            string prefix = inlineIndent ? Indent : "";
            string cond = ctx.test() != null ? TranslateExpr(ctx.test().GetText()) : "true";
            var sb = new StringBuilder();
            sb.AppendLine($"{prefix}if (!({cond})) {{");
            sb.Append(VisitSuiteBody(ctx.suite()));
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        public string VisitFuncDef(PilotParser.Func_defContext ctx)
        {
            string name = ctx.identifier().GetText();
            string asyncPrefix = ctx.ASYNC() != null ? "async " : "";
            var paramList = new List<string>();

            if (ctx.parameters()?.parameter() != null)
            {
                foreach (var p in ctx.parameters().parameter())
                {
                    string pName = p.identifier()?.GetText() ?? "arg";
                    string? defaultVal = p.test()?.GetText();
                    paramList.Add(defaultVal != null ? $"{pName} = {TranslateExpr(defaultVal)}" : pName);
                }
            }

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}{asyncPrefix}function {name}({string.Join(", ", paramList)}) {{");
            sb.Append(VisitSuiteBody(ctx.suite()));
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        public string VisitClassDef(PilotParser.Class_defContext ctx)
        {
            string name = ctx.identifier().GetText();
            string? baseClass = ctx.class_bases()?.type_spec()?.FirstOrDefault()?.GetText();
            string ext = !string.IsNullOrEmpty(baseClass) ? $" extends {baseClass}" : "";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}class {name}{ext} {{");
            sb.Append(VisitSuiteBody(ctx.suite()));
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        public string VisitIfStmt(PilotParser.If_stmtContext ctx)
        {
            var tests = ctx.test();
            var suites = ctx.suite();
            var sb = new StringBuilder();

            sb.AppendLine($"{Indent}if ({TranslateExpr(tests[0].GetText())}) {{");
            sb.Append(VisitSuiteBody(suites[0]));
            sb.Append($"{Indent}}}");

            for (int i = 1; i < tests.Length; i++)
            {
                sb.AppendLine($" else if ({TranslateExpr(tests[i].GetText())}) {{");
                sb.Append(VisitSuiteBody(suites[i]));
                sb.Append($"{Indent}}}");
            }

            if (suites.Length > tests.Length)
            {
                sb.AppendLine(" else {");
                sb.Append(VisitSuiteBody(suites[^1]));
                sb.Append($"{Indent}}}");
            }

            return sb.ToString();
        }

        public string VisitWhileStmt(PilotParser.While_stmtContext ctx)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}while ({TranslateExpr(ctx.test(0).GetText())}) {{");
            sb.Append(VisitSuiteBody(ctx.suite(0)));
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        public string VisitForStmt(PilotParser.For_stmtContext ctx)
        {
            var sb = new StringBuilder();
            string target = ctx.for_target().GetText();
            string rawIter = ctx.test(0).GetText();

            // Optimization: zero-allocation numeric loop for 'a..b' or 'range(a, b)'
            if (PilotOptimizer.TryParseRangeExpr(rawIter, out string startR, out string endR))
            {
                sb.AppendLine($"{Indent}for (let {target} = {TranslateExpr(startR)}; {target} <= {TranslateExpr(endR)}; {target}++) {{");
            }
            else if (PilotOptimizer.TryParseRangeCall(rawIter, out string startC, out string stopC))
            {
                sb.AppendLine($"{Indent}for (let {target} = {TranslateExpr(startC)}; {target} < {TranslateExpr(stopC)}; {target}++) {{");
            }
            else
            {
                sb.AppendLine($"{Indent}for (const {target} of {TranslateExpr(rawIter)}) {{");
            }

            sb.Append(VisitSuiteBody(ctx.suite(0)));
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        public string VisitMatchStmt(PilotParser.Match_stmtContext ctx)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}switch ({TranslateExpr(ctx.test().GetText())}) {{");
            _indent++;
            if (ctx.match_suite()?.match_arm() != null)
            {
                foreach (var arm in ctx.match_suite().match_arm())
                {
                    string pat = arm.pattern().GetText();
                    if (pat == "_" || pat == "default")
                    {
                        sb.AppendLine($"{Indent}default: {{");
                    }
                    else
                    {
                        sb.AppendLine($"{Indent}case {TranslateExpr(pat)}: {{");
                    }
                    sb.Append(VisitSuiteBody(arm.suite()));
                    sb.AppendLine($"{Indent}{_options.IndentString}break;");
                    sb.AppendLine($"{Indent}}}");
                }
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string VisitForeignBlock(PilotParser.Inline_foreign_blockContext ctx)
        {
            string lang = ctx.foreign_language().GetText().ToLowerInvariant();
            if ((lang == "js" || lang == "javascript") && ctx.stmt() != null && ctx.stmt().Length > 0)
            {
                var sb = new StringBuilder();
                foreach (var s in ctx.stmt())
                {
                    sb.AppendLine(VisitStmt(s));
                }
                return sb.ToString().TrimEnd();
            }
            return $"{Indent}// [polyglot:{lang}] {ctx.GetText()}";
        }

        private string VisitSuiteBody(PilotParser.SuiteContext? ctx)
        {
            if (ctx == null) return string.Empty;

            _indent++;
            var sb = new StringBuilder();
            if (ctx.simple_stmt() != null)
            {
                sb.AppendLine(VisitSimpleStmt(ctx.simple_stmt()));
            }
            else if (ctx.stmt() != null && ctx.stmt().Length > 0)
            {
                foreach (var s in ctx.stmt())
                {
                    string res = VisitStmt(s);
                    if (!string.IsNullOrWhiteSpace(res))
                        sb.AppendLine(res);
                }
            }
            _indent--;
            return sb.ToString();
        }

        private static string TranslateExpr(string expr)
        {
            string s = PilotOptimizer.FoldConstantsAndNormalize(expr);

            // Convert f"Hello {name}" -> `Hello ${name}`
            if (s.StartsWith("f\"", StringComparison.Ordinal) && s.EndsWith("\"", StringComparison.Ordinal))
            {
                string inner = s[2..^1];
                string tpl = Regex.Replace(inner, @"\{([^}]+)\}", "${$1}");
                return $"`{tpl}`";
            }

            if (s.StartsWith("println(", StringComparison.Ordinal))
            {
                string inner = s["println(".Length..^1];
                return $"console.log({TranslateExpr(inner)})";
            }
            if (s.StartsWith("print(", StringComparison.Ordinal))
            {
                string inner = s["print(".Length..^1];
                return $"process.stdout.write(String({TranslateExpr(inner)}))";
            }

            s = s.Replace("==", "===").Replace("!=", "!==")
                 .Replace(" and ", " && ").Replace(" or ", " || ");

            return s;
        }
    }
}
