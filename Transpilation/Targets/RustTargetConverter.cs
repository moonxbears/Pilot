using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Antlr4.Runtime;
using Pilot.Diagnostics;
using Pilot.Preprocessing;

namespace Pilot.Transpilation.Targets;

/// <summary>
/// Bidirectional compile-time code converter between Pilot and Rust.
/// </summary>
public sealed class RustTargetConverter : ITargetLanguageConverter
{
    public string Language => "rust";
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

        var tree = parser.program();
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
            sb.AppendLine("// Code transpiled from Pilot to Rust (2024 Edition)");
            sb.AppendLine("#![allow(unused_variables, dead_code)]");
            sb.AppendLine();
        }

        var visitor = new PilotToRustVisitor(options);
        string code = visitor.VisitProgram(programAst);
        sb.Append(code);

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options)
    {
        var sb = new StringBuilder();
        if (options.EmitHeaderComments)
        {
            sb.AppendLine("// Converted from Rust to Pilot");
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

            string converted = ConvertRustLineToPilot(trimmed);
            int indent = line.TakeWhile(char.IsWhiteSpace).Count();
            sb.AppendLine(new string(' ', indent) + converted);
        }

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, "pilot");
    }

    private static string ConvertRustLineToPilot(string line)
    {
        string res = line;

        // Visibility
        res = res.Replace("pub fn ", "def ").Replace("fn ", "def ");
        res = res.Replace("pub struct ", "class ").Replace("struct ", "class ");

        // Variables
        res = res.Replace("let mut ", "let ");

        // Print
        res = res.Replace("println!", "println").Replace("print!", "print");

        return res;
    }

    private sealed class PilotToRustVisitor
    {
        private readonly TranspilationOptions _options;
        private int _indent;

        public PilotToRustVisitor(TranspilationOptions options)
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
                string? type = ctx.let_stmt().type_annotation()?.type_spec()?.GetText();
                string? val = ctx.let_stmt().test()?.GetText();
                string typeAnnot = !string.IsNullOrEmpty(type) ? $": {MapType(type)}" : "";
                string valStr = val != null ? FormatExpr(val) : "";
                return val != null ? $"let mut {name}{typeAnnot} = {valStr};" : $"let mut {name}{typeAnnot};";
            }
            if (ctx.var_stmt() != null)
            {
                string name = ctx.var_stmt().binding_pattern().GetText();
                string? type = ctx.var_stmt().type_annotation()?.type_spec()?.GetText();
                string? val = ctx.var_stmt().test()?.GetText();
                string typeAnnot = !string.IsNullOrEmpty(type) ? $": {MapType(type)}" : "";
                return val != null ? $"let mut {name}{typeAnnot} = {val};" : $"let mut {name}{typeAnnot};";
            }
            if (ctx.flow_stmt() != null)
            {
                var flow = ctx.flow_stmt();
                if (flow.RETURN() != null)
                {
                    string ret = flow.testlist() != null ? flow.testlist().GetText() : "";
                    return string.IsNullOrEmpty(ret) ? "return;" : $"return {ret};";
                }
                if (flow.BREAK() != null) return "break;";
                if (flow.CONTINUE() != null) return "continue;";
            }
            if (ctx.expr_stmt() != null)
            {
                string expr = ctx.expr_stmt().GetText().TrimEnd(';');
                if (expr.StartsWith("println("))
                {
                    expr = "println!" + expr["println".Length..];
                }
                else if (expr.StartsWith("print("))
                {
                    expr = "print!" + expr["print".Length..];
                }
                return expr + ";";
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
            if (ctx.match_stmt() != null) return VisitMatchStmt(ctx.match_stmt());
            return Indent + "// " + ctx.GetText();
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

            sb.AppendLine($"{Indent}if {tests[0].GetText()}");
            sb.Append(VisitSuite(suites[0]));

            for (int i = 1; i < tests.Length; i++)
            {
                sb.AppendLine($"{Indent}else if {tests[i].GetText()}");
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
            sb.AppendLine($"{Indent}while {ctx.test(0).GetText()}");
            sb.Append(VisitSuite(ctx.suite(0)));
            return sb.ToString();
        }

        public string VisitForStmt(PilotParser.For_stmtContext ctx)
        {
            var sb = new StringBuilder();
            string target = ctx.for_target().GetText();
            string iterable = ctx.test(0).GetText();
            sb.AppendLine($"{Indent}for {target} in {iterable}");
            sb.Append(VisitSuite(ctx.suite(0)));
            return sb.ToString();
        }

        public string VisitMatchStmt(PilotParser.Match_stmtContext ctx)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}match {ctx.test().GetText()} {{");
            _indent++;
            if (ctx.match_suite()?.match_arm() != null)
            {
                foreach (var arm in ctx.match_suite().match_arm())
                {
                    string pattern = arm.pattern().GetText();
                    string guard = arm.test() != null ? $" if {arm.test().GetText()}" : "";
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
            string formatted = expr;
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
