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
/// Bidirectional optimizing compile-time code converter between Pilot and Lua 5.4 / LuaJIT.
/// Emits register-allocated 'local' bindings, native numeric 'for' loops, and metatable classes.
/// </summary>
public sealed class LuaTargetConverter : ITargetLanguageConverter
{
    public string Language => "lua";
    public string TargetLanguage => Language;
    public string SourceLanguage => Language;

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
            sb.AppendLine("-- Code transpiled & optimized from Pilot to Lua 5.4 / LuaJIT");
            sb.AppendLine("-- Optimizations: local register bindings, numeric range loops, constant folding");
            sb.AppendLine();
        }

        var visitor = new PilotToLuaVisitor(options);
        string code = visitor.VisitProgram(programAst);
        sb.Append(code);

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options)
    {
        var sb = new StringBuilder();
        if (options.EmitHeaderComments)
        {
            sb.AppendLine("// Converted & optimized from Lua to Pilot");
            sb.AppendLine();
        }

        var lines = foreignSource.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None);
        foreach (var line in lines)
        {
            string trimmed = line.Trim();
            int indent = line.TakeWhile(char.IsWhiteSpace).Count();
            string pad = new string(' ', indent);

            if (string.IsNullOrWhiteSpace(trimmed))
            {
                sb.AppendLine();
                continue;
            }

            if (trimmed.StartsWith("--"))
            {
                sb.AppendLine($"{pad}//{trimmed[2..]}");
                continue;
            }

            if (trimmed == "end")
            {
                sb.AppendLine($"{pad}}}");
                continue;
            }

            string converted = ConvertLuaLineToPilot(trimmed);
            sb.AppendLine($"{pad}{converted}");
        }

        string optimized = PilotOptimizer.OptimizeIngestedPilotCode(sb.ToString());
        return TranspilationResult.CreateSuccess(optimized, "pilot");
    }

    private static string ConvertLuaLineToPilot(string line)
    {
        string res = line;

        // Operators and literals
        res = res.Replace("~=", "!=").Replace(" nil", " null").Replace("=nil", "=null");
        res = Regex.Replace(res, @"\s+\.\.\s+", " + ");

        // local function name(args) -> def name(args) {
        var localFn = Regex.Match(res, @"^(?:local\s+)?function\s+([A-Za-z0-9_.:]+)\s*\((.*)\)\s*$");
        if (localFn.Success)
        {
            string fnName = localFn.Groups[1].Value.Replace(":", ".");
            return $"def {fnName}({localFn.Groups[2].Value}) {{";
        }

        // Numeric for: for i = a, b do -> for i in a..b {
        var numFor = Regex.Match(res, @"^for\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([^,]+),\s*([^\s]+)\s+do$");
        if (numFor.Success)
        {
            return $"for {numFor.Groups[1].Value} in {numFor.Groups[2].Value.Trim()}..{numFor.Groups[3].Value.Trim()} {{";
        }

        // Generic for: for _, v in ipairs(list) do -> for v in list {
        var genFor = Regex.Match(res, @"^for\s+(?:[A-Za-z_][A-Za-z0-9_]*\s*,\s*)?([A-Za-z_][A-Za-z0-9_]*)\s+in\s+(?:i?pairs\((.+)\)|(.+))\s+do$");
        if (genFor.Success)
        {
            string coll = !string.IsNullOrEmpty(genFor.Groups[2].Value) ? genFor.Groups[2].Value : genFor.Groups[3].Value;
            return $"for {genFor.Groups[1].Value} in {coll.Trim()} {{";
        }

        // elseif cond then -> } elif cond {
        var elifMatch = Regex.Match(res, @"^elseif\s+(.+?)\s+then$");
        if (elifMatch.Success)
        {
            return $"}} elif {elifMatch.Groups[1].Value} {{";
        }

        // if cond then -> if cond {
        var ifMatch = Regex.Match(res, @"^if\s+(.+?)\s+then$");
        if (ifMatch.Success)
        {
            return $"if {ifMatch.Groups[1].Value} {{";
        }

        if (res == "else")
        {
            return "} else {";
        }

        // while cond do -> while cond {
        var whileMatch = Regex.Match(res, @"^while\s+(.+?)\s+do$");
        if (whileMatch.Success)
        {
            return $"while {whileMatch.Groups[1].Value} {{";
        }

        // local x = val -> let x = val
        if (res.StartsWith("local "))
        {
            res = "let " + res["local ".Length..];
        }

        // print(...) -> println(...)
        if (res.StartsWith("print("))
        {
            res = "println(" + res["print(".Length..];
        }

        return res;
    }

    private sealed class PilotToLuaVisitor
    {
        private readonly TranspilationOptions _options;
        private int _indent;

        public PilotToLuaVisitor(TranspilationOptions options)
        {
            _options = options;
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
                return val != null ? $"local {name} = {TranslateExpr(val)}" : $"local {name} = nil";
            }
            if (ctx.var_stmt() != null)
            {
                string name = ctx.var_stmt().binding_pattern().GetText();
                string? val = ctx.var_stmt().test()?.GetText();
                return val != null ? $"local {name} = {TranslateExpr(val)}" : $"local {name} = nil";
            }
            if (ctx.const_stmt() != null)
            {
                string name = ctx.const_stmt().binding_pattern().GetText();
                string val = ctx.const_stmt().test().GetText();
                return $"local {name} <const> = {TranslateExpr(val)}";
            }
            if (ctx.guard_stmt() != null)
            {
                return VisitGuardStmt(ctx.guard_stmt(), inlineIndent: false);
            }
            if (ctx.defer_stmt() != null)
            {
                string body = ctx.defer_stmt().small_stmt() != null
                    ? VisitSmallStmt(ctx.defer_stmt().small_stmt())
                    : TranslateExpr(ctx.defer_stmt().GetText()["defer".Length..].Trim());
                return $"-- [defer] {body}";
            }
            if (ctx.flow_stmt() != null)
            {
                var flow = ctx.flow_stmt();
                if (flow.RETURN() != null)
                {
                    string ret = flow.testlist() != null ? TranslateExpr(flow.testlist().GetText()) : "";
                    return string.IsNullOrEmpty(ret) ? "return" : $"return {ret}";
                }
                if (flow.BREAK() != null) return "break";
                if (flow.RAISE() != null)
                {
                    string err = flow.test() != null && flow.test().Length > 0 ? TranslateExpr(flow.test(0).GetText()) : "\"error\"";
                    return $"error({err})";
                }
            }
            if (ctx.assert_stmt() != null)
            {
                var tests = ctx.assert_stmt().test();
                return tests.Length > 1
                    ? $"assert({TranslateExpr(tests[0].GetText())}, {TranslateExpr(tests[1].GetText())})"
                    : $"assert({TranslateExpr(tests[0].GetText())})";
            }
            if (ctx.expr_stmt() != null)
            {
                return TranslateExpr(ctx.expr_stmt().GetText().TrimEnd(';'));
            }
            return "-- " + ctx.GetText().TrimEnd(';');
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
            return Indent + "-- " + ctx.GetText();
        }

        private string VisitGuardStmt(PilotParser.Guard_stmtContext ctx, bool inlineIndent)
        {
            string prefix = inlineIndent ? Indent : "";
            string cond = ctx.test() != null ? TranslateExpr(ctx.test().GetText()) : "true";
            var sb = new StringBuilder();
            sb.AppendLine($"{prefix}if not ({cond}) then");
            sb.Append(VisitSuite(ctx.suite()));
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        public string VisitFuncDef(PilotParser.Func_defContext ctx)
        {
            string name = ctx.identifier().GetText();
            var paramList = new List<string>();

            if (ctx.parameters()?.parameter() != null)
            {
                foreach (var p in ctx.parameters().parameter())
                {
                    string pName = p.identifier()?.GetText() ?? "arg";
                    paramList.Add(pName);
                }
            }

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}local function {name}({string.Join(", ", paramList)})");
            sb.Append(VisitSuite(ctx.suite()));
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        public string VisitClassDef(PilotParser.Class_defContext ctx)
        {
            string name = ctx.identifier().GetText();
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}local {name} = {{}}");
            sb.AppendLine($"{Indent}{name}.__index = {name}");
            sb.AppendLine($"{Indent}function {name}.new(init)");
            sb.AppendLine($"{Indent}{_options.IndentString}return setmetatable(init or {{}}, {name})");
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        public string VisitIfStmt(PilotParser.If_stmtContext ctx)
        {
            var tests = ctx.test();
            var suites = ctx.suite();
            var sb = new StringBuilder();

            sb.AppendLine($"{Indent}if {TranslateExpr(tests[0].GetText())} then");
            sb.Append(VisitSuite(suites[0]));

            for (int i = 1; i < tests.Length; i++)
            {
                sb.AppendLine($"{Indent}elseif {TranslateExpr(tests[i].GetText())} then");
                sb.Append(VisitSuite(suites[i]));
            }

            if (suites.Length > tests.Length)
            {
                sb.AppendLine($"{Indent}else");
                sb.Append(VisitSuite(suites[^1]));
            }

            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        public string VisitWhileStmt(PilotParser.While_stmtContext ctx)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}while {TranslateExpr(ctx.test(0).GetText())} do");
            sb.Append(VisitSuite(ctx.suite(0)));
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        public string VisitForStmt(PilotParser.For_stmtContext ctx)
        {
            var sb = new StringBuilder();
            string target = ctx.for_target().GetText();
            string rawIter = ctx.test(0).GetText();

            // Optimize inclusive range 'a..b' or 'range(a, b)' into native Lua numeric 'for i = a, b do'
            if (PilotOptimizer.TryParseRangeExpr(rawIter, out string startR, out string endR))
            {
                sb.AppendLine($"{Indent}for {target} = {TranslateExpr(startR)}, {TranslateExpr(endR)} do");
            }
            else if (PilotOptimizer.TryParseRangeCall(rawIter, out string startC, out string stopC))
            {
                sb.AppendLine($"{Indent}for {target} = {TranslateExpr(startC)}, {TranslateExpr(stopC)} - 1 do");
            }
            else
            {
                sb.AppendLine($"{Indent}for _, {target} in ipairs({TranslateExpr(rawIter)}) do");
            }

            sb.Append(VisitSuite(ctx.suite(0)));
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        public string VisitMatchStmt(PilotParser.Match_stmtContext ctx)
        {
            var sb = new StringBuilder();
            string subject = TranslateExpr(ctx.test().GetText());
            string matchVar = "__match_val";
            sb.AppendLine($"{Indent}local {matchVar} = {subject}");

            if (ctx.match_suite()?.match_arm() != null)
            {
                var arms = ctx.match_suite().match_arm();
                for (int i = 0; i < arms.Length; i++)
                {
                    string pat = arms[i].pattern().GetText();
                    string kw = i == 0 ? "if" : "elseif";
                    if (pat == "_" || pat == "default")
                    {
                        sb.AppendLine($"{Indent}else");
                    }
                    else
                    {
                        sb.AppendLine($"{Indent}{kw} {matchVar} == {TranslateExpr(pat)} then");
                    }
                    sb.Append(VisitSuite(arms[i].suite()));
                }
                if (arms.Length > 0)
                {
                    sb.Append($"{Indent}end");
                }
            }
            return sb.ToString();
        }

        private string VisitForeignBlock(PilotParser.Inline_foreign_blockContext ctx)
        {
            string lang = ctx.foreign_language().GetText().ToLowerInvariant();
            if (lang == "lua" && ctx.stmt() != null && ctx.stmt().Length > 0)
            {
                var sb = new StringBuilder();
                foreach (var s in ctx.stmt())
                {
                    sb.AppendLine(VisitStmt(s));
                }
                return sb.ToString().TrimEnd();
            }
            return $"{Indent}-- [polyglot:{lang}] {ctx.GetText()}";
        }

        private string VisitSuite(PilotParser.SuiteContext? ctx)
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

            // Convert f"Hello {name}" -> string.format("Hello %s", tostring(name))
            if (s.StartsWith("f\"", StringComparison.Ordinal) && s.EndsWith("\"", StringComparison.Ordinal))
            {
                string inner = s[2..^1];
                var args = new List<string>();
                string fmt = Regex.Replace(inner, @"\{([^}]+)\}", m =>
                {
                    args.Add($"tostring({m.Groups[1].Value.Trim()})");
                    return "%s";
                });
                return args.Count > 0
                    ? $"string.format(\"{fmt}\", {string.Join(", ", args)})"
                    : $"\"{fmt}\"";
            }

            if (s.StartsWith("println(", StringComparison.Ordinal))
            {
                string inner = s["println(".Length..^1];
                return $"print({TranslateExpr(inner)})";
            }

            s = s.Replace("!=", "~=")
                 .Replace("null", "nil")
                 .Replace("None", "nil")
                 .Replace("&&", " and ")
                 .Replace("||", " or ");

            return s;
        }
    }
}
