using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Antlr4.Runtime;
using Pilot.Diagnostics;
using Pilot.Preprocessing;

namespace Pilot.Transpilation.Targets;

/// <summary>
/// Bidirectional compile-time code converter between Pilot and C# (.NET).
/// </summary>
public sealed class CSharpTargetConverter : ITargetLanguageConverter
{
    public string Language => "csharp";
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
            sb.AppendLine("// Code transpiled from Pilot to C# (.NET 10)");
            sb.AppendLine("#nullable enable");
            sb.AppendLine("using System;");
            sb.AppendLine("using System.Collections.Generic;");
            sb.AppendLine("using System.Linq;");
            sb.AppendLine();
        }

        string ns = options.TargetNamespaceOrModule ?? "PilotTranspiledApp";
        sb.AppendLine($"namespace {ns};");
        sb.AppendLine();
        sb.AppendLine("public static class ProgramModule");
        sb.AppendLine("{");

        var visitor = new PilotToCSharpVisitor(options, baseIndentLevel: 1);
        string body = visitor.VisitProgram(programAst);
        sb.Append(body);

        sb.AppendLine("}");

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options)
    {
        var sb = new StringBuilder();
        if (options.EmitHeaderComments)
        {
            sb.AppendLine("// Converted from C# to Pilot");
            sb.AppendLine();
        }

        var lines = foreignSource.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None);
        foreach (var line in lines)
        {
            string trimmed = line.Trim();

            if (string.IsNullOrWhiteSpace(trimmed))
            {
                sb.AppendLine();
                continue;
            }

            if (trimmed.StartsWith("using ") || trimmed.StartsWith("namespace ") || trimmed.StartsWith("#"))
            {
                // Skip or convert to comments
                continue;
            }

            string converted = ConvertCSharpLineToPilot(trimmed);
            int indent = line.TakeWhile(char.IsWhiteSpace).Count();
            sb.AppendLine(new string(' ', indent) + converted);
        }

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, "pilot");
    }

    private static string ConvertCSharpLineToPilot(string line)
    {
        string res = line;

        // Visibility modifiers
        res = res.Replace("public ", "").Replace("private ", "").Replace("protected ", "").Replace("internal ", "");
        res = res.Replace("static ", "");

        // Foreach
        if (res.StartsWith("foreach (var ") || res.StartsWith("foreach ("))
        {
            res = res.Replace("foreach (var ", "for ").Replace("foreach (", "for ");
            int closingParen = res.IndexOf(')');
            if (closingParen > 0)
            {
                res = res[..closingParen] + res[(closingParen + 1)..];
            }
        }

        // Variable declarations
        if (res.StartsWith("var "))
        {
            res = "let " + res["var ".Length..];
        }

        // Method declarations (e.g. void Foo() -> def Foo())
        string[] typeKeywords = { "void", "int", "string", "double", "float", "bool", "long" };
        foreach (var tk in typeKeywords)
        {
            if (res.StartsWith(tk + " ") && res.Contains('(') && !res.Contains('='))
            {
                res = "def " + res[(tk.Length + 1)..];
                break;
            }
        }

        // Print
        res = res.Replace("Console.WriteLine", "println").Replace("Console.Write", "print");

        return res;
    }

    private sealed class PilotToCSharpVisitor
    {
        private readonly TranspilationOptions _options;
        private int _indent;

        public PilotToCSharpVisitor(TranspilationOptions options, int baseIndentLevel = 0)
        {
            _options = options;
            _indent = baseIndentLevel;
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
                string typeStr = !string.IsNullOrEmpty(type) ? MapType(type) : "var";
                return val != null ? $"{typeStr} {name} = {val};" : $"{typeStr} {name} = default!;";
            }
            if (ctx.var_stmt() != null)
            {
                string name = ctx.var_stmt().binding_pattern().GetText();
                string? type = ctx.var_stmt().type_annotation()?.type_spec()?.GetText();
                string? val = ctx.var_stmt().test()?.GetText();
                string typeStr = !string.IsNullOrEmpty(type) ? MapType(type) : "var";
                return val != null ? $"{typeStr} {name} = {val};" : $"{typeStr} {name} = default!;";
            }
            if (ctx.const_stmt() != null)
            {
                string name = ctx.const_stmt().binding_pattern().GetText();
                string? type = ctx.const_stmt().type_annotation()?.type_spec()?.GetText();
                string val = ctx.const_stmt().test().GetText();
                string typeStr = !string.IsNullOrEmpty(type) ? MapType(type) : "const var";
                return $"{typeStr} {name} = {val};";
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
                    expr = "Console.WriteLine" + expr["println".Length..];
                }
                else if (expr.StartsWith("print("))
                {
                    expr = "Console.Write" + expr["print".Length..];
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
                    string typeStr = !string.IsNullOrEmpty(pType) ? MapType(pType) : "dynamic";
                    paramList.Add($"{typeStr} {pName}");
                }
            }

            string retType = ctx.type_spec()?.GetText() ?? "void";
            string retStr = MapType(retType);

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}public static {retStr} {name}({string.Join(", ", paramList)})");
            sb.Append(VisitSuite(ctx.suite()));
            return sb.ToString();
        }

        public string VisitClassDef(PilotParser.Class_defContext ctx)
        {
            string name = ctx.identifier().GetText();
            string? baseClass = ctx.class_bases()?.type_spec()?.FirstOrDefault()?.GetText();
            string inheritance = !string.IsNullOrEmpty(baseClass) ? $" : {MapType(baseClass)}" : "";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}public class {name}{inheritance}");
            sb.Append(VisitSuite(ctx.suite()));
            return sb.ToString();
        }

        public string VisitIfStmt(PilotParser.If_stmtContext ctx)
        {
            var tests = ctx.test();
            var suites = ctx.suite();
            var sb = new StringBuilder();

            sb.AppendLine($"{Indent}if ({tests[0].GetText()})");
            sb.Append(VisitSuite(suites[0]));

            for (int i = 1; i < tests.Length; i++)
            {
                sb.AppendLine($"{Indent}else if ({tests[i].GetText()})");
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
            sb.AppendLine($"{Indent}while ({ctx.test(0).GetText()})");
            sb.Append(VisitSuite(ctx.suite(0)));
            return sb.ToString();
        }

        public string VisitForStmt(PilotParser.For_stmtContext ctx)
        {
            var sb = new StringBuilder();
            string target = ctx.for_target().GetText();
            string iterable = ctx.test(0).GetText();
            sb.AppendLine($"{Indent}foreach (var {target} in {iterable})");
            sb.Append(VisitSuite(ctx.suite(0)));
            return sb.ToString();
        }

        public string VisitMatchStmt(PilotParser.Match_stmtContext ctx)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}switch ({ctx.test().GetText()})");
            sb.AppendLine($"{Indent}{{");
            _indent++;
            if (ctx.match_suite()?.match_arm() != null)
            {
                foreach (var arm in ctx.match_suite().match_arm())
                {
                    string pattern = arm.pattern().GetText().Replace("..", "..");
                    string guard = arm.test() != null ? $" when {arm.test().GetText()}" : "";
                    sb.AppendLine($"{Indent}case {pattern}{guard}:");
                    sb.Append(VisitSuite(arm.suite()));
                    sb.AppendLine($"{Indent}{_options.IndentString}break;");
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

        private string MapType(string pilotType)
        {
            if (_options.TypeMappings.TryGetValue(pilotType, out var mapped))
            {
                return mapped;
            }

            return pilotType switch
            {
                "i8" => "sbyte",
                "u8" => "byte",
                "i16" => "short",
                "u16" => "ushort",
                "i32" or "int" => "int",
                "u32" or "uint" => "uint",
                "i64" => "long",
                "u64" => "ulong",
                "f32" or "float" => "float",
                "f64" or "double" => "double",
                "decimal" => "decimal",
                "str" or "string" => "string",
                "bool" => "bool",
                "void" => "void",
                "list" => "List<object>",
                "map" => "Dictionary<object, object>",
                "set" => "HashSet<object>",
                _ => pilotType
            };
        }
    }
}
