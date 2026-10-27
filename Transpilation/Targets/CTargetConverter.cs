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
/// Bidirectional optimizing compile-time code converter between Pilot and C (C11 / C23).
/// Emits exact hardware integer types (<stdint.h>), static inline functions, const promotion,
/// zero-allocation range loops, and automatic main() entry point wrapping.
/// </summary>
public sealed class CTargetConverter : ITargetLanguageConverter
{
    public string Language => "c";
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
            sb.AppendLine("/* Code transpiled & optimized from Pilot to C (C11/C23) */");
            sb.AppendLine("#include <stdint.h>");
            sb.AppendLine("#include <stdbool.h>");
            sb.AppendLine("#include <stdio.h>");
            sb.AppendLine("#include <assert.h>");
            sb.AppendLine();
        }

        var mutatedVars = PilotOptimizer.AnalyzeMutatedVariables(programAst);
        var visitor = new PilotToCVisitor(options, mutatedVars);
        string code = visitor.VisitProgram(programAst);
        sb.Append(code);

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options)
    {
        var sb = new StringBuilder();
        if (options.EmitHeaderComments)
        {
            sb.AppendLine("// Converted & optimized from C to Pilot");
            sb.AppendLine();
        }

        var lines = foreignSource.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None);
        foreach (var line in lines)
        {
            string trimmed = line.Trim();
            if (trimmed.StartsWith("#include"))
                continue;

            int indent = line.TakeWhile(char.IsWhiteSpace).Count();
            string pad = new string(' ', indent);

            if (string.IsNullOrWhiteSpace(trimmed))
            {
                sb.AppendLine();
                continue;
            }

            string converted = ConvertCLineToPilot(trimmed);
            sb.AppendLine($"{pad}{converted}");
        }

        string optimized = PilotOptimizer.OptimizeIngestedPilotCode(sb.ToString());
        return TranspilationResult.CreateSuccess(optimized, "pilot");
    }

    private static string ConvertCLineToPilot(string line)
    {
        string res = line.TrimEnd(';');

        // printf("msg\n") -> println("msg")
        var printfMatch = Regex.Match(res, @"^printf\(\s*""([^""]*?)\\n""\s*(?:,\s*(.+))?\)$");
        if (printfMatch.Success)
        {
            string fmt = printfMatch.Groups[1].Value;
            string argsStr = printfMatch.Groups[2].Value;
            if (string.IsNullOrWhiteSpace(argsStr))
            {
                return $"println(\"{fmt}\")";
            }

            var args = argsStr.Split(',').Select(a => a.Trim()).ToArray();
            int idx = 0;
            string fstr = Regex.Replace(fmt, @"%[dlsfu]", _ => idx < args.Length ? $"{{{args[idx++]}}}" : "");
            return $"println(f\"{fstr}\")";
        }

        // C-style for loop: for (int i = a; i <= b; ++i) -> for i in a..b {
        var forMatch = Regex.Match(res, @"^for\s*\(\s*(?:int64_t|int32_t|int|size_t)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([^;]+);\s*\1\s*(<=|<)\s*([^;]+);\s*(?:\+\+\1|\1\+\+)\s*\)\s*\{?$");
        if (forMatch.Success)
        {
            string v = forMatch.Groups[1].Value;
            string start = forMatch.Groups[2].Value.Trim();
            string op = forMatch.Groups[3].Value;
            string end = forMatch.Groups[4].Value.Trim();
            if (op == "<" && int.TryParse(end, out int endNum))
                end = (endNum - 1).ToString();
            else if (op == "<")
                end = $"({end} - 1)";
            return $"for {v} in {start}..{end} {{";
        }

        // Function definition: static inline int64_t add(int64_t a, int64_t b) {
        var fnMatch = Regex.Match(res, @"^(?:static\s+)?(?:inline\s+)?(int8_t|int16_t|int32_t|int64_t|uint8_t|uint16_t|uint32_t|uint64_t|int|long|float|double|bool|void|const\s+char\s*\*)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)\s*(\{?)$");
        if (fnMatch.Success)
        {
            string retType = MapCTypeToPilot(fnMatch.Groups[1].Value);
            string fnName = fnMatch.Groups[2].Value;
            string rawParams = fnMatch.Groups[3].Value.Trim();
            string brace = fnMatch.Groups[4].Value;

            if (fnName == "main")
            {
                return $"def main() -> i32 {brace}".TrimEnd();
            }

            var pilotParams = new List<string>();
            if (!string.IsNullOrEmpty(rawParams) && rawParams != "void")
            {
                foreach (var p in rawParams.Split(','))
                {
                    var parts = p.Trim().Split(' ', StringSplitOptions.RemoveEmptyEntries);
                    if (parts.Length >= 2)
                    {
                        string pName = parts[^1].TrimStart('*');
                        string pType = MapCTypeToPilot(string.Join(" ", parts[..^1]));
                        pilotParams.Add($"{pName}: {pType}");
                    }
                }
            }

            string retAnnot = retType != "void" ? $" -> {retType}" : "";
            return $"def {fnName}({string.Join(", ", pilotParams)}){retAnnot} {brace}".TrimEnd();
        }

        // Typed variable declaration: const int64_t x = 10 -> const x: i64 = 10
        var varMatch = Regex.Match(res, @"^(const\s+)?(int8_t|int16_t|int32_t|int64_t|uint8_t|uint16_t|uint32_t|uint64_t|int|long|float|double|bool|const\s+char\s*\*)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+)$");
        if (varMatch.Success)
        {
            bool isConst = !string.IsNullOrEmpty(varMatch.Groups[1].Value);
            string pType = MapCTypeToPilot(varMatch.Groups[2].Value);
            string vName = varMatch.Groups[3].Value;
            string rhs = varMatch.Groups[4].Value.Trim();
            return $"{(isConst ? "const" : "let")} {vName}: {pType} = {rhs}";
        }

        return res;
    }

    private static string MapCTypeToPilot(string cType) => cType.Trim() switch
    {
        "int8_t" => "i8",
        "int16_t" => "i16",
        "int32_t" or "int" => "i32",
        "int64_t" or "long" => "i64",
        "uint8_t" => "u8",
        "uint16_t" => "u16",
        "uint32_t" => "u32",
        "uint64_t" => "u64",
        "float" => "f32",
        "double" => "f64",
        "bool" => "bool",
        "const char*" or "const char *" or "char*" => "str",
        _ => cType.Trim()
    };

    private sealed class PilotToCVisitor
    {
        private readonly TranspilationOptions _options;
        private readonly HashSet<string> _mutatedVars;
        private int _indent;

        public PilotToCVisitor(TranspilationOptions options, HashSet<string> mutatedVars)
        {
            _options = options;
            _mutatedVars = mutatedVars;
        }

        private string Indent => string.Concat(Enumerable.Repeat(_options.IndentString, _indent));

        public string VisitProgram(PilotParser.ProgramContext ctx)
        {
            var topDefs = new StringBuilder();
            var mainStmts = new List<PilotParser.StmtContext>();

            foreach (var stmt in ctx.stmt())
            {
                if (stmt.compound_stmt()?.func_def() != null || stmt.compound_stmt()?.class_def() != null || stmt.compound_stmt()?.struct_def() != null)
                {
                    topDefs.AppendLine(VisitStmt(stmt));
                    topDefs.AppendLine();
                }
                else
                {
                    mainStmts.Add(stmt);
                }
            }

            if (mainStmts.Count > 0)
            {
                topDefs.AppendLine("int main(void) {");
                _indent = 1;
                foreach (var s in mainStmts)
                {
                    string line = VisitStmt(s);
                    if (!string.IsNullOrWhiteSpace(line))
                        topDefs.AppendLine(line);
                }
                topDefs.AppendLine($"{_options.IndentString}return 0;");
                topDefs.AppendLine("}");
                _indent = 0;
            }

            return topDefs.ToString();
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
                string? rawType = ctx.let_stmt().type_annotation()?.type_spec()?.GetText();
                string? val = ctx.let_stmt().test()?.GetText();
                string cType = !string.IsNullOrEmpty(rawType) ? MapType(rawType) : InferCType(val);
                string constMod = (!_mutatedVars.Contains(name) && val != null) ? "const " : "";
                return val != null ? $"{constMod}{cType} {name} = {TranslateExpr(val)};" : $"{cType} {name} = 0;";
            }
            if (ctx.var_stmt() != null)
            {
                string name = ctx.var_stmt().binding_pattern().GetText();
                string? rawType = ctx.var_stmt().type_annotation()?.type_spec()?.GetText();
                string? val = ctx.var_stmt().test()?.GetText();
                string cType = !string.IsNullOrEmpty(rawType) ? MapType(rawType) : InferCType(val);
                return val != null ? $"{cType} {name} = {TranslateExpr(val)};" : $"{cType} {name} = 0;";
            }
            if (ctx.const_stmt() != null)
            {
                string name = ctx.const_stmt().binding_pattern().GetText();
                string? rawType = ctx.const_stmt().type_annotation()?.type_spec()?.GetText();
                string val = ctx.const_stmt().test().GetText();
                string cType = !string.IsNullOrEmpty(rawType) ? MapType(rawType) : InferCType(val);
                return $"const {cType} {name} = {TranslateExpr(val)};";
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
            }
            if (ctx.assert_stmt() != null)
            {
                var tests = ctx.assert_stmt().test();
                return $"assert({TranslateExpr(tests[0].GetText())});";
            }
            if (ctx.expr_stmt() != null)
            {
                return TranslateExprStmt(ctx.expr_stmt().GetText().TrimEnd(';'));
            }
            return "/* " + ctx.GetText().TrimEnd(';') + " */";
        }

        public string VisitCompoundStmt(PilotParser.Compound_stmtContext ctx)
        {
            if (ctx.func_def() != null) return VisitFuncDef(ctx.func_def());
            if (ctx.class_def() != null) return VisitClassDef(ctx.class_def());
            if (ctx.if_stmt() != null) return VisitIfStmt(ctx.if_stmt());
            if (ctx.while_stmt() != null) return VisitWhileStmt(ctx.while_stmt());
            if (ctx.for_stmt() != null) return VisitForStmt(ctx.for_stmt());
            if (ctx.guard_stmt() != null) return VisitGuardStmt(ctx.guard_stmt(), inlineIndent: true);
            if (ctx.inline_foreign_block() != null) return VisitForeignBlock(ctx.inline_foreign_block());
            return Indent + "/* " + ctx.GetText() + " */";
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
            var paramList = new List<string>();

            if (ctx.parameters()?.parameter() != null)
            {
                foreach (var p in ctx.parameters().parameter())
                {
                    string pName = p.identifier()?.GetText() ?? "arg";
                    string? pType = p.type_annotation()?.type_spec()?.GetText();
                    string cType = !string.IsNullOrEmpty(pType) ? MapType(pType) : "int64_t";
                    paramList.Add($"{cType} {pName}");
                }
            }

            string? retType = ctx.type_spec()?.GetText();
            string cRet = !string.IsNullOrEmpty(retType) ? MapType(retType) : "void";
            string paramsStr = paramList.Count > 0 ? string.Join(", ", paramList) : "void";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}static inline {cRet} {name}({paramsStr}) {{");
            sb.Append(VisitSuiteBody(ctx.suite()));
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        public string VisitClassDef(PilotParser.Class_defContext ctx)
        {
            string name = ctx.identifier().GetText();
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}typedef struct {name} {{");
            sb.AppendLine($"{Indent}{_options.IndentString}int64_t _reserved;");
            sb.Append($"{Indent}}} {name};");
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

            if (PilotOptimizer.TryParseRangeExpr(rawIter, out string startR, out string endR))
            {
                sb.AppendLine($"{Indent}for (int64_t {target} = {TranslateExpr(startR)}; {target} <= {TranslateExpr(endR)}; ++{target}) {{");
            }
            else if (PilotOptimizer.TryParseRangeCall(rawIter, out string startC, out string stopC))
            {
                sb.AppendLine($"{Indent}for (int64_t {target} = {TranslateExpr(startC)}; {target} < {TranslateExpr(stopC)}; ++{target}) {{");
            }
            else
            {
                sb.AppendLine($"{Indent}for (int64_t {target} = 0; {target} < 1; ++{target}) {{");
            }

            sb.Append(VisitSuiteBody(ctx.suite(0)));
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string VisitForeignBlock(PilotParser.Inline_foreign_blockContext ctx)
        {
            string lang = ctx.foreign_language().GetText().ToLowerInvariant();
            if (lang == "c" && ctx.stmt() != null && ctx.stmt().Length > 0)
            {
                var sb = new StringBuilder();
                foreach (var s in ctx.stmt())
                {
                    sb.AppendLine(VisitStmt(s));
                }
                return sb.ToString().TrimEnd();
            }
            return $"{Indent}/* [polyglot:{lang}] {ctx.GetText()} */";
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

        private static string TranslateExprStmt(string expr)
        {
            if (expr.StartsWith("println(", StringComparison.Ordinal))
            {
                string inner = expr["println(".Length..^1].Trim();
                if (inner.StartsWith("f\"", StringComparison.Ordinal) && inner.EndsWith("\"", StringComparison.Ordinal))
                {
                    string raw = inner[2..^1];
                    var args = new List<string>();
                    string fmt = Regex.Replace(raw, @"\{([^}]+)\}", m =>
                    {
                        args.Add(m.Groups[1].Value.Trim());
                        return "%lld";
                    });
                    return args.Count > 0
                        ? $"printf(\"{fmt}\\n\", (long long)({string.Join("), (long long)(", args)}));"
                        : $"printf(\"{fmt}\\n\");";
                }
                if (inner.StartsWith("\"", StringComparison.Ordinal))
                {
                    return $"printf(\"%s\\n\", {inner});";
                }
                return $"printf(\"%lld\\n\", (long long)({TranslateExpr(inner)}));";
            }
            return TranslateExpr(expr) + ";";
        }

        private static string TranslateExpr(string expr)
        {
            return PilotOptimizer.FoldConstantsAndNormalize(expr)
                .Replace(" and ", " && ")
                .Replace(" or ", " || ")
                .Replace("null", "NULL")
                .Replace("None", "NULL");
        }

        private static string InferCType(string? val)
        {
            if (string.IsNullOrEmpty(val)) return "int64_t";
            string v = val.Trim();
            if (v.StartsWith("\"", StringComparison.Ordinal) || v.StartsWith("f\"", StringComparison.Ordinal))
                return "const char*";
            if (v == "true" || v == "false")
                return "bool";
            if (v.Contains('.') && double.TryParse(v, out _))
                return "double";
            return "int64_t";
        }

        private string MapType(string pilotType)
        {
            if (_options.TypeMappings.TryGetValue(pilotType, out var mapped))
                return mapped;

            return pilotType switch
            {
                "i8" => "int8_t",
                "u8" => "uint8_t",
                "i16" => "int16_t",
                "u16" => "uint16_t",
                "i32" => "int32_t",
                "u32" => "uint32_t",
                "i64" or "int" => "int64_t",
                "u64" or "uint" => "uint64_t",
                "f8" or "f16" or "f32" => "float",
                "f64" or "float" or "double" => "double",
                "bool" => "bool",
                "str" or "string" => "const char*",
                "void" => "void",
                _ => pilotType
            };
        }
    }
}
