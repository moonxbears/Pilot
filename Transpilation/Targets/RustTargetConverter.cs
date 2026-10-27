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
/// Bidirectional optimizing compile-time code converter between Pilot and Rust (2024 Edition).
/// Performs mutability analysis to emit immutable 'let' vs 'let mut', inclusive ranges 'a..=b',
/// and hardware-exact scalar types.
/// </summary>
public sealed class RustTargetConverter : ITargetLanguageConverter
{
    public string Language { get; }
    public string TargetLanguage => Language;
    public string SourceLanguage => Language;

    public RustTargetConverter(string alias = "rust")
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
            sb.AppendLine("// Code transpiled & optimized from Pilot to Rust (2024 Edition)");
            sb.AppendLine("#![allow(unused_variables, dead_code)]");
            sb.AppendLine();
        }

        var mutatedVars = PilotOptimizer.AnalyzeMutatedVariables(programAst);
        var visitor = new PilotToRustVisitor(options, mutatedVars);
        string code = visitor.VisitProgram(programAst);
        sb.Append(code);

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options)
    {
        var sb = new StringBuilder();
        if (options.EmitHeaderComments)
        {
            sb.AppendLine("// Converted & optimized from Rust to Pilot");
            sb.AppendLine();
        }

        var lines = foreignSource.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None);
        foreach (var line in lines)
        {
            string trimmed = line.Trim();
            if (string.IsNullOrWhiteSpace(trimmed) || trimmed.StartsWith("#!"))
            {
                sb.AppendLine();
                continue;
            }

            string converted = ConvertRustLineToPilot(trimmed, options.Optimize);
            int indent = line.TakeWhile(char.IsWhiteSpace).Count();
            sb.AppendLine(new string(' ', indent) + converted);
        }

        string outputCode = options.Optimize ? PilotOptimizer.OptimizeIngestedPilotCode(sb.ToString()) : sb.ToString().TrimEnd() + Environment.NewLine;
        return TranspilationResult.CreateSuccess(outputCode, "pilot");
    }

    private static string ConvertRustLineToPilot(string line, bool optimize = false)
    {
        string res = optimize ? line.TrimEnd(';') : line;

        // Visibility & Functions
        res = res.Replace("pub fn ", "def ").Replace("fn ", "def ");
        res = res.Replace("pub struct ", "class ").Replace("struct ", "class ");

        // Inclusive range a..=b -> a..b
        res = res.Replace("..=", "..");

        // Variables
        res = res.Replace("let mut ", optimize ? "var " : "let ");

        // println!("Hello {}", x) -> println(f"Hello {x}")
        var printMatch = Regex.Match(res, @"^println!\(\s*""([^""]*)""\s*(?:,\s*(.+))?\)$");
        if (printMatch.Success)
        {
            string fmt = printMatch.Groups[1].Value;
            string argsStr = printMatch.Groups[2].Value;
            if (!string.IsNullOrWhiteSpace(argsStr))
            {
                var args = argsStr.Split(',').Select(a => a.Trim()).ToArray();
                int idx = 0;
                string fstr = Regex.Replace(fmt, @"\{\}", _ => idx < args.Length ? $"{{{args[idx++]}}}" : "");
                return $"println(f\"{fstr}\")";
            }
            return $"println(\"{fmt}\")";
        }

        res = res.Replace("println!", "println").Replace("print!", "print");

        return res;
    }

    private sealed class PilotToRustVisitor
    {
        private readonly TranspilationOptions _options;
        private readonly HashSet<string> _mutatedVars;
        private int _indent;

        public PilotToRustVisitor(TranspilationOptions options, HashSet<string> mutatedVars)
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
                string? type = ctx.let_stmt().type_annotation()?.type_spec()?.GetText();
                string? val = ctx.let_stmt().test()?.GetText();
                string typeAnnot = !string.IsNullOrEmpty(type) ? $": {MapType(type)}" : "";
                string valStr = val != null ? FormatExpr(val) : "";
                string mutKw = _options.Optimize ? (_mutatedVars.Contains(name) ? "let mut" : "let") : "let mut";
                return val != null ? $"{mutKw} {name}{typeAnnot} = {valStr};" : $"let mut {name}{typeAnnot};";
            }
            if (ctx.var_stmt() != null)
            {
                string name = ctx.var_stmt().binding_pattern().GetText();
                string? type = ctx.var_stmt().type_annotation()?.type_spec()?.GetText();
                string? val = ctx.var_stmt().test()?.GetText();
                string typeAnnot = !string.IsNullOrEmpty(type) ? $": {MapType(type)}" : "";
                return val != null ? $"let mut {name}{typeAnnot} = {FormatExpr(val)};" : $"let mut {name}{typeAnnot};";
            }
            if (ctx.const_stmt() != null)
            {
                string name = ctx.const_stmt().binding_pattern().GetText();
                string? type = ctx.const_stmt().type_annotation()?.type_spec()?.GetText();
                string val = ctx.const_stmt().test().GetText();
                string typeStr = !string.IsNullOrEmpty(type) ? MapType(type) : "i64";
                return $"const {name}: {typeStr} = {FormatExpr(val)};";
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
                    string ret = flow.testlist() != null ? FormatExpr(flow.testlist().GetText()) : "";
                    return string.IsNullOrEmpty(ret) ? "return;" : $"return {ret};";
                }
                if (flow.BREAK() != null) return "break;";
                if (flow.CONTINUE() != null) return "continue;";
            }
            if (ctx.assert_stmt() != null)
            {
                var tests = ctx.assert_stmt().test();
                return $"assert!({FormatExpr(tests[0].GetText())});";
            }
            if (ctx.expr_stmt() != null)
            {
                string expr = ctx.expr_stmt().GetText().TrimEnd(';');
                if (expr.StartsWith("println("))
                {
                    string inner = expr["println(".Length..^1].Trim();
                    if (inner.StartsWith("f\"") && inner.EndsWith("\""))
                    {
                        return $"println!(\"{inner[2..^1]}\");";
                    }
                    if (inner.StartsWith("\"") && inner.EndsWith("\""))
                    {
                        return $"println!({inner});";
                    }
                    return $"println!(\"{{:?}}\", {FormatExpr(inner)});";
                }
                if (expr.StartsWith("print("))
                {
                    expr = "print!" + expr["print".Length..];
                }
                return FormatExpr(expr) + ";";
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
            string cond = ctx.test() != null ? FormatExpr(ctx.test().GetText()) : "true";
            var sb = new StringBuilder();
            sb.AppendLine($"{prefix}if !({cond})");
            sb.Append(VisitSuite(ctx.suite()));
            return sb.ToString().TrimEnd();
        }

        public string VisitFuncDef(PilotParser.Func_defContext ctx)
        {
            string name = ctx.identifier().GetText();
            var paramList = new List<string>();

            if (ctx.parameters()?.parameter() != null)
            {
                foreach (var p in ctx.parameters().parameter())
                {
                    string pName = p.identifier()?.GetText() ?? p.splice_expr()?.GetText() ?? "arg";
                    string? pType = p.type_annotation()?.type_spec()?.GetText();
                    string typeStr = !string.IsNullOrEmpty(pType) ? MapType(pType) : "i64";
                    paramList.Add($"{pName}: {typeStr}");
                }
            }

            string? retType = ctx.type_spec()?.GetText();
            string retStr = !string.IsNullOrEmpty(retType) && retType != "void" ? $" -> {MapType(retType)}" : "";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}pub fn {name}({string.Join(", ", paramList)}){retStr}");
            sb.Append(VisitSuite(ctx.suite()));
            return sb.ToString();
        }

        public string VisitClassDef(PilotParser.Class_defContext ctx)
        {
            string name = ctx.identifier().GetText();
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}pub struct {name} {{}}");
            sb.AppendLine($"{Indent}impl {name}");
            sb.Append(VisitSuite(ctx.suite()));
            return sb.ToString();
        }

        public string VisitIfStmt(PilotParser.If_stmtContext ctx)
        {
            var tests = ctx.test();
            var suites = ctx.suite();
            var sb = new StringBuilder();

            sb.AppendLine($"{Indent}if {FormatExpr(tests[0].GetText())}");
            sb.Append(VisitSuite(suites[0]));

            for (int i = 1; i < tests.Length; i++)
            {
                sb.AppendLine($"{Indent}else if {FormatExpr(tests[i].GetText())}");
                sb.Append(VisitSuite(suites[i]));
            }

            if (suites.Length > tests.Length)
            {
                sb.AppendLine($"{Indent}else");
                sb.Append(VisitSuite(suites[^1]));
            }

            return sb.ToString();
        }

        public string VisitWhileStmt(PilotParser.While_stmtContext ctx)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}while {FormatExpr(ctx.test(0).GetText())}");
            sb.Append(VisitSuite(ctx.suite(0)));
            return sb.ToString();
        }

        public string VisitForStmt(PilotParser.For_stmtContext ctx)
        {
            var sb = new StringBuilder();
            string target = ctx.for_target().GetText();
            string rawIter = ctx.test(0).GetText();

            string iterable = PilotOptimizer.TryParseRangeExpr(rawIter, out string startR, out string endR)
                ? $"{FormatExpr(startR)}..={FormatExpr(endR)}"
                : FormatExpr(rawIter);

            sb.AppendLine($"{Indent}for {target} in {iterable}");
            sb.Append(VisitSuite(ctx.suite(0)));
            return sb.ToString();
        }

        public string VisitMatchStmt(PilotParser.Match_stmtContext ctx)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}match {FormatExpr(ctx.test().GetText())} {{");
            _indent++;
            if (ctx.match_suite()?.match_arm() != null)
            {
                foreach (var arm in ctx.match_suite().match_arm())
                {
                    string pattern = arm.pattern().GetText();
                    string guard = arm.test() != null ? $" if {FormatExpr(arm.test().GetText())}" : "";
                    sb.AppendLine($"{Indent}{pattern}{guard} => {{");
                    _indent++;
                    sb.AppendLine(VisitSuite(arm.suite()).Trim());
                    _indent--;
                    sb.AppendLine($"{Indent}}},");
                }
            }
            _indent--;
            sb.AppendLine($"{Indent}}}");
            return sb.ToString();
        }

        private string VisitForeignBlock(PilotParser.Inline_foreign_blockContext ctx)
        {
            string lang = ctx.foreign_language().GetText().ToLowerInvariant();
            if (lang == "rust" && ctx.stmt() != null && ctx.stmt().Length > 0)
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

        private string VisitSuite(PilotParser.SuiteContext ctx)
        {
            if (ctx == null) return $"{Indent}{{\n{Indent}}}\n";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}{{");
            _indent++;

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
            sb.AppendLine($"{Indent}}}");
            return sb.ToString();
        }

        private static string FormatExpr(string expr)
        {
            string formatted = PilotOptimizer.FoldConstantsAndNormalize(expr);
            if (formatted.StartsWith("f\"") && formatted.EndsWith("\""))
            {
                return $"format!(\"{formatted[2..^1]}\")";
            }

            foreach (var op in new[] { "==", "!=", "<=", ">=", "+", "-", "*", "/", "%" })
            {
                if (formatted.Contains(op) && !formatted.Contains($" {op} "))
                {
                    formatted = formatted.Replace(op, $" {op} ");
                }
            }
            return formatted;
        }

        private string MapType(string pilotType)
        {
            if (_options.TypeMappings.TryGetValue(pilotType, out var mapped))
            {
                return mapped;
            }

            return pilotType switch
            {
                "i8" => "i8",
                "u8" => "u8",
                "i16" => "i16",
                "u16" => "u16",
                "i32" => "i32",
                "u32" => "u32",
                "i64" or "int" => "i64",
                "u64" or "uint" => "u64",
                "f32" or "float" => "f32",
                "f64" or "double" => "f64",
                "str" or "string" => "String",
                "bool" => "bool",
                "void" => "()",
                "list" => "Vec<Box<dyn std::any::Any>>",
                _ => pilotType
            };
        }
    }
}
