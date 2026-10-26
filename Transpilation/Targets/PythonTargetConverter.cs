using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Antlr4.Runtime;
using Pilot.Diagnostics;
using Pilot.Preprocessing;

namespace Pilot.Transpilation.Targets;

/// <summary>
/// Bidirectional compile-time code converter between Pilot and Python 3.
/// </summary>
public sealed class PythonTargetConverter : ITargetLanguageConverter
{
    public string Language => "python";
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
            sb.AppendLine("# Code transpiled from Pilot to Python 3");
            sb.AppendLine("# Target: Python 3.10+ (typed)");
            sb.AppendLine("from __future__ import annotations");
            sb.AppendLine("from typing import Any, List, Dict, Set, Tuple, Optional, Callable");
            sb.AppendLine();
        }

        var visitor = new PilotToPythonVisitor(options);
        string code = visitor.VisitProgram(programAst);
        sb.Append(code);

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options)
    {
        var sb = new StringBuilder();
        if (options.EmitHeaderComments)
        {
            sb.AppendLine("// Converted from Python to Pilot");
            sb.AppendLine();
        }

        var lines = foreignSource.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None);
        var indentStack = new Stack<int>();
        indentStack.Push(0);

        for (int i = 0; i < lines.Length; i++)
        {
            string line = lines[i];
            string trimmed = line.Trim();

            if (string.IsNullOrWhiteSpace(trimmed) || trimmed.StartsWith("#"))
            {
                if (trimmed.StartsWith("#"))
                    sb.AppendLine("//" + trimmed[1..]);
                else
                    sb.AppendLine();
                continue;
            }

            int currentIndent = line.TakeWhile(char.IsWhiteSpace).Count();

            while (indentStack.Peek() > currentIndent)
            {
                indentStack.Pop();
                sb.AppendLine(new string(' ', indentStack.Peek()) + "}");
            }

            // Convert Python constructs to Pilot
            string converted = ConvertPythonLineToPilot(trimmed);

            if (converted.EndsWith(":"))
            {
                converted = converted[..^1].TrimEnd() + " {";
                sb.AppendLine(new string(' ', currentIndent) + converted);
                indentStack.Push(currentIndent + 4);
            }
            else
            {
                if (!converted.EndsWith(";") && !converted.EndsWith("{") && !converted.EndsWith("}"))
                {
                    converted += ";";
                }
                sb.AppendLine(new string(' ', currentIndent) + converted);
            }
        }

        while (indentStack.Pop() > 0)
        {
            sb.AppendLine(new string(' ', indentStack.Count * 4) + "}");
        }

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, "pilot");
    }

    private static string ConvertPythonLineToPilot(string line)
    {
        string res = line;

        // Literals
        res = res.Replace("True", "true").Replace("False", "false").Replace("None", "null");

        // Elif
        if (res.StartsWith("elif "))
        {
            res = "else if " + res["elif ".Length..];
        }

        // Variable assignment: x = 10 -> let x = 10
        if (!res.StartsWith("def ") && !res.StartsWith("class ") && !res.StartsWith("if ") &&
            !res.StartsWith("while ") && !res.StartsWith("for ") && !res.StartsWith("return ") &&
            !res.StartsWith("else") && !res.StartsWith("match ") && !res.StartsWith("case ") &&
            res.Contains('=') && !res.Contains("==") && !res.Contains("<=") && !res.Contains(">="))
        {
            if (!res.StartsWith("self."))
            {
                res = "let " + res;
            }
        }

        return res;
    }

    private sealed class PilotToPythonVisitor
    {
        private readonly TranspilationOptions _options;
        private int _indent;

        public PilotToPythonVisitor(TranspilationOptions options)
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
                return val != null ? $"{name} = {TranslateExpr(val)}" : $"{name} = None";
            }
            if (ctx.var_stmt() != null)
            {
                string name = ctx.var_stmt().binding_pattern().GetText();
                string? val = ctx.var_stmt().test()?.GetText();
                return val != null ? $"{name} = {TranslateExpr(val)}" : $"{name} = None";
            }
            if (ctx.const_stmt() != null)
            {
                string name = ctx.const_stmt().binding_pattern().GetText();
                string val = ctx.const_stmt().test().GetText();
                return $"{name} = {TranslateExpr(val)}";
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
                if (flow.CONTINUE() != null) return "continue";
            }
            if (ctx.expr_stmt() != null)
            {
                return TranslateExpr(ctx.expr_stmt().GetText().TrimEnd(';'));
            }
            if (ctx.assert_stmt() != null)
            {
                var tests = ctx.assert_stmt().test();
                return tests.Length > 1
                    ? $"assert {TranslateExpr(tests[0].GetText())}, {TranslateExpr(tests[1].GetText())}"
                    : $"assert {TranslateExpr(tests[0].GetText())}";
            }
            return ctx.GetText().TrimEnd(';');
        }

        public string VisitCompoundStmt(PilotParser.Compound_stmtContext ctx)
        {
            if (ctx.func_def() != null) return VisitFuncDef(ctx.func_def());
            if (ctx.class_def() != null) return VisitClassDef(ctx.class_def());
            if (ctx.if_stmt() != null) return VisitIfStmt(ctx.if_stmt());
            if (ctx.while_stmt() != null) return VisitWhileStmt(ctx.while_stmt());
            if (ctx.for_stmt() != null) return VisitForStmt(ctx.for_stmt());
            if (ctx.match_stmt() != null) return VisitMatchStmt(ctx.match_stmt());
            return Indent + "# " + ctx.GetText();
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
                    string? defaultVal = p.test()?.GetText();

                    string paramStr = pName;
                    if (_options.GenerateTypeAnnotations && !string.IsNullOrEmpty(pType))
                    {
                        paramStr += $": {MapType(pType)}";
                    }
                    if (!string.IsNullOrEmpty(defaultVal))
                    {
                        paramStr += $" = {TranslateExpr(defaultVal)}";
                    }
                    paramList.Add(paramStr);
                }
            }

            string? retType = ctx.type_spec()?.GetText();
            string retAnnot = (_options.GenerateTypeAnnotations && !string.IsNullOrEmpty(retType))
                ? $" -> {MapType(retType)}"
                : "";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}def {name}({string.Join(", ", paramList)}){retAnnot}:");
            sb.Append(VisitSuite(ctx.suite()));
            return sb.ToString();
        }

        public string VisitClassDef(PilotParser.Class_defContext ctx)
        {
            string name = ctx.identifier().GetText();
            string? baseClass = ctx.class_bases()?.type_spec()?.FirstOrDefault()?.GetText();
            string inheritance = !string.IsNullOrEmpty(baseClass) ? $"({MapType(baseClass)})" : "";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}class {name}{inheritance}:");
            sb.Append(VisitSuite(ctx.suite()));
            return sb.ToString();
        }

        public string VisitIfStmt(PilotParser.If_stmtContext ctx)
        {
            var tests = ctx.test();
            var suites = ctx.suite();
            var sb = new StringBuilder();

            sb.AppendLine($"{Indent}if {TranslateExpr(tests[0].GetText())}:");
            sb.Append(VisitSuite(suites[0]));

            for (int i = 1; i < tests.Length; i++)
            {
                sb.AppendLine($"{Indent}elif {TranslateExpr(tests[i].GetText())}:");
                sb.Append(VisitSuite(suites[i]));
            }

            if (suites.Length > tests.Length)
            {
                sb.AppendLine($"{Indent}else:");
                sb.Append(VisitSuite(suites[^1]));
            }

            return sb.ToString();
        }

        public string VisitWhileStmt(PilotParser.While_stmtContext ctx)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}while {TranslateExpr(ctx.test(0).GetText())}:");
            sb.Append(VisitSuite(ctx.suite(0)));
            return sb.ToString();
        }

        public string VisitForStmt(PilotParser.For_stmtContext ctx)
        {
            var sb = new StringBuilder();
            string target = ctx.for_target().GetText();
            string iterable = TranslateExpr(ctx.test(0).GetText());
            sb.AppendLine($"{Indent}for {target} in {iterable}:");
            sb.Append(VisitSuite(ctx.suite(0)));
            return sb.ToString();
        }

        public string VisitMatchStmt(PilotParser.Match_stmtContext ctx)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}match {TranslateExpr(ctx.test().GetText())}:");
            _indent++;
            if (ctx.match_suite()?.match_arm() != null)
            {
                foreach (var arm in ctx.match_suite().match_arm())
                {
                    string pattern = TranslatePattern(arm.pattern().GetText());
                    string guard = arm.test() != null ? $" if {TranslateExpr(arm.test().GetText())}" : "";
                    sb.AppendLine($"{Indent}case {pattern}{guard}:");
                    sb.Append(VisitSuite(arm.suite()));
                }
            }
            _indent--;
            return sb.ToString();
        }

        private string VisitSuite(PilotParser.SuiteContext ctx)
        {
            if (ctx == null) return $"{Indent}{_options.IndentString}pass\n";

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
            else
            {
                sb.AppendLine($"{Indent}pass");
            }

            _indent--;
            return sb.ToString();
        }

        private string MapType(string pilotType)
        {
            if (_options.TypeMappings.TryGetValue(pilotType, out var mapped))
            {
                return mapped;
            }

            return pilotType switch
            {
                "i8" or "i16" or "i32" or "i64" or "int" or "u8" or "u16" or "u32" or "u64" or "uint" => "int",
                "f8" or "f16" or "f32" or "f64" or "float" or "double" => "float",
                "str" or "string" => "str",
                "bool" => "bool",
                "void" => "None",
                "list" => "List[Any]",
                "map" => "Dict[Any, Any]",
                "set" => "Set[Any]",
                "tuple" => "Tuple[Any, ...]",
                _ => pilotType
            };
        }

        private static string TranslateExpr(string expr)
        {
            // If string literal, return as is
            if ((expr.StartsWith("\"") && expr.EndsWith("\"")) || (expr.StartsWith("'") && expr.EndsWith("'")))
            {
                return expr;
            }

            string formatted = expr
                .Replace("true", "True")
                .Replace("false", "False")
                .Replace("null", "None")
                .Replace("undefined", "None")
                .Replace("&&", " and ")
                .Replace("||", " or ");

            if (formatted.StartsWith("!"))
            {
                formatted = "not " + formatted[1..];
            }

            foreach (var op in new[] { "==", "!=", "<=", ">=", "+", "-", "*", "/", "%", ">", "<" })
            {
                if (formatted.Contains(op) && !formatted.Contains($" {op} "))
                {
                    formatted = formatted.Replace(op, $" {op} ");
                }
            }

            return formatted;
        }

        private static string TranslatePattern(string pat)
        {
            return pat
                .Replace("..", "*")
                .Replace("_", "_");
        }
    }
}
