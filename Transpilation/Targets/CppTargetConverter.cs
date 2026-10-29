using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Pilot.Analysis;
using Pilot.Transpilation.Ingesters;
using Pilot.Transpilation.Ir;

namespace Pilot.Transpilation.Targets;

/// <summary>
/// Milestone 20.3 &amp; 20.4: AST-driven bidirectional code converter between Pilot and Modern C++ (C++20 / C++23).
/// Emits exact-width <c>&lt;cstdint&gt;</c> types, <c>std::optional&lt;T&gt;</c>, <c>std::variant</c> ADTs,
/// <c>constexpr</c> functions, and <c>const auto</c> bindings compilable by <c>clang++ -std=c++20 -fsyntax-only</c>.
/// </summary>
public sealed class CppTargetConverter : ITargetLanguageConverter
{
    public string Language { get; }
    public string TargetLanguage => Language;
    public string SourceLanguage => Language;

    public CppTargetConverter(string alias = "cpp")
    {
        Language = alias;
    }

    public TranspilationResult TranspileSource(string pilotSource, TranspilationOptions options)
    {
        var (module, errors) = PilotTranspileIrBuilder.BuildFromSource(pilotSource, options);
        if (module == null || errors.Count > 0)
        {
            return TranspilationResult.CreateFailure(errors, TargetLanguage);
        }

        return TranspileModule(module, options);
    }

    public TranspilationResult Transpile(PilotParser.ProgramContext programAst, TranspilationOptions options)
    {
        PilotStaticAnalysisResult? analysis = null;
        try { analysis = PilotAnalysisPipeline.Analyze(programAst); } catch { }
        var module = PilotTranspileIrBuilder.Build(programAst, null, analysis, options);
        return TranspileModule(module, options);
    }

    public TranspilationResult TranspileModule(PilotIrModule module, TranspilationOptions options)
    {
        var sb = new StringBuilder();
        if (options.EmitHeaderComments)
        {
            sb.AppendLine("// Code transpiled & optimized from Pilot to Modern C++ (C++20/23)");
            sb.AppendLine("#include <iostream>");
            sb.AppendLine("#include <string>");
            sb.AppendLine("#include <vector>");
            sb.AppendLine("#include <cstdint>");
            sb.AppendLine("#include <cassert>");
            sb.AppendLine("#include <cmath>");
            sb.AppendLine("#include <optional>");
            sb.AppendLine("#include <variant>");
            sb.AppendLine();
        }

        var emitter = new CppIrEmitter(module, options);
        sb.Append(emitter.EmitModule());

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options) =>
        ForeignAstIngester.Ingest(foreignSource, SourceLanguage, options);

    private sealed class CppIrEmitter
    {
        private readonly PilotIrModule _module;
        private readonly TranspilationOptions _options;
        private int _indent;

        public CppIrEmitter(PilotIrModule module, TranspilationOptions options)
        {
            _module = module;
            _options = options;
        }

        private string Indent => string.Concat(Enumerable.Repeat(_options.IndentString, _indent));

        public string EmitModule()
        {
            var topDefs = new StringBuilder();
            var mainStmts = new List<PilotIrStmt>();
            bool hasExplicitMain = _module.Statements.OfType<PilotIrFuncDefStmt>().Any(f => f.Name == "main");

            foreach (var stmt in _module.Statements)
            {
                if (stmt is PilotIrFuncDefStmt or PilotIrClassDefStmt or PilotIrAdtDefStmt or
                            PilotIrEnumDefStmt or PilotIrTypeAliasStmt or PilotIrExtensionDefStmt)
                {
                    string defCode = EmitStmt(stmt);
                    if (!string.IsNullOrWhiteSpace(defCode))
                    {
                        topDefs.AppendLine(defCode);
                        topDefs.AppendLine();
                    }
                }
                else
                {
                    mainStmts.Add(stmt);
                }
            }

            if (mainStmts.Count > 0 && !hasExplicitMain)
            {
                topDefs.AppendLine("int main() {");
                _indent = 1;
                foreach (var s in mainStmts)
                {
                    string line = EmitStmt(s);
                    if (!string.IsNullOrWhiteSpace(line))
                        topDefs.AppendLine(line);
                }
                topDefs.AppendLine($"{_options.IndentString}return 0;");
                topDefs.AppendLine("}");
                _indent = 0;
            }

            return topDefs.ToString();
        }

        public string EmitStmt(PilotIrStmt stmt) => stmt switch
        {
            PilotIrCommentStmt c => $"{Indent}//{c.CommentText.TrimStart('/')}",
            PilotIrVarDeclStmt v => EmitVarDecl(v),
            PilotIrAssignStmt a => $"{Indent}{EmitExpr(a.Target)} {a.Operator} {EmitExpr(a.Value)};",
            PilotIrFuncDefStmt f => EmitFuncDef(f, isMethod: false),
            PilotIrClassDefStmt c => EmitClassDef(c),
            PilotIrAdtDefStmt adt => EmitAdtDef(adt),
            PilotIrEnumDefStmt en => EmitEnumDef(en),
            PilotIrTypeAliasStmt ta => $"{Indent}using {ta.Name} = {MapType(ta.TargetType)};",
            PilotIrExtensionDefStmt ext => EmitExtensionDef(ext),
            PilotIrIfStmt ifs => EmitIfStmt(ifs),
            PilotIrGuardStmt g => EmitGuardStmt(g),
            PilotIrWhileStmt w => EmitWhileStmt(w),
            PilotIrForStmt f => EmitForStmt(f),
            PilotIrMatchStmt m => EmitMatchStmt(m),
            PilotIrTryStmt t => EmitTryStmt(t),
            PilotIrDeferStmt d => string.Join(Environment.NewLine, d.Body.Select(EmitStmt)),
            PilotIrReturnStmt r => r.Value != null ? $"{Indent}return {EmitExpr(r.Value)};" : $"{Indent}return;",
            PilotIrRaiseStmt rs => rs.Value != null ? $"{Indent}throw std::runtime_error({EmitExpr(rs.Value)});" : $"{Indent}throw std::runtime_error(\"error\");",
            PilotIrBreakStmt => $"{Indent}break;",
            PilotIrContinueStmt => $"{Indent}continue;",
            PilotIrPassStmt => $"{Indent}// pass",
            PilotIrAssertStmt a => $"{Indent}assert({EmitExpr(a.Condition)});",
            PilotIrComptimeStmt ct => string.Join(Environment.NewLine, ct.Body.Select(EmitStmt)),
            PilotIrExprStmt e => EmitExprStmt(e),
            _ => ""
        };

        private string EmitVarDecl(PilotIrVarDeclStmt v)
        {
            if (v.TupleDestructureNames.Count > 1)
            {
                string lhs = $"auto [{string.Join(", ", v.TupleDestructureNames)}]";
                string rhs = v.Initializer != null ? EmitExpr(v.Initializer) : "{}";
                return $"{Indent}{lhs} = {rhs};";
            }

            string cppType = !string.IsNullOrEmpty(v.DeclaredType) ? MapType(v.DeclaredType) : "auto";
            string rhsVal = v.ComptimeFoldedLiteral ?? (v.Initializer != null ? EmitExpr(v.Initializer) : "");

            if (v.Kind == PilotVarDeclKind.Const)
            {
                return $"{Indent}constexpr {cppType} {v.Name} = {rhsVal};";
            }

            string constMod = (v.Kind == PilotVarDeclKind.Let && !v.IsMutated && !string.IsNullOrEmpty(rhsVal))
                ? "const "
                : "";
            return !string.IsNullOrEmpty(rhsVal)
                ? $"{Indent}{constMod}{cppType} {v.Name} = {rhsVal};"
                : $"{Indent}{(cppType == "auto" ? "int64_t" : cppType)} {v.Name}{{}};";
        }

        private string EmitFuncDef(PilotIrFuncDefStmt f, bool isMethod)
        {
            var paramList = new List<string>();
            foreach (var p in f.Parameters)
            {
                if (p.Name is "self" or "this")
                    continue;
                string cppType = !string.IsNullOrEmpty(p.TypeAnnotation) ? MapType(p.TypeAnnotation) : "auto";
                string defPart = p.DefaultValue != null ? $" = {EmitExpr(p.DefaultValue)}" : "";
                paramList.Add($"{cppType} {p.Name}{defPart}");
            }

            string cppRet = f.Name == "main"
                ? "int"
                : (!string.IsNullOrEmpty(f.DeclaredReturnType) ? MapType(f.DeclaredReturnType) : "auto");
            string mod = f.Name == "main" || isMethod ? "" : "constexpr ";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}{mod}{cppRet} {f.Name}({string.Join(", ", paramList)}) {{");
            _indent++;
            foreach (var req in f.RequiresClauses)
                sb.AppendLine($"{Indent}assert({EmitExpr(req)});");
            foreach (var s in f.Body)
            {
                string line = EmitStmt(s);
                if (!string.IsNullOrWhiteSpace(line))
                    sb.AppendLine(line);
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitClassDef(PilotIrClassDefStmt c)
        {
            var sb = new StringBuilder();
            string inheritance = c.BaseTypes.Count > 0
                ? $" : public {string.Join(", public ", c.BaseTypes.Select(MapType))}"
                : "";
            sb.AppendLine($"{Indent}class {c.Name}{inheritance} {{");
            sb.AppendLine($"{Indent}public:");
            _indent++;
            foreach (var f in c.Fields)
            {
                string defVal = f.DefaultValue != null ? $" = {EmitExpr(f.DefaultValue)}" : "{}";
                sb.AppendLine($"{Indent}{MapType(f.TypeAnnotation)} {f.Name}{defVal};");
            }
            foreach (var m in c.Methods)
            {
                sb.AppendLine(EmitFuncDef(m, isMethod: true));
            }
            _indent--;
            sb.Append($"{Indent}}};");
            return sb.ToString();
        }

        private string EmitAdtDef(PilotIrAdtDefStmt adt)
        {
            var sb = new StringBuilder();
            foreach (var v in adt.Variants)
            {
                sb.AppendLine($"{Indent}struct {v.Name} {{");
                _indent++;
                foreach (var f in v.Fields)
                    sb.AppendLine($"{Indent}{MapType(f.Type)} {f.Name};");
                _indent--;
                sb.AppendLine($"{Indent}}};");
            }
            if (adt.Variants.Count > 0)
            {
                sb.Append($"{Indent}using {adt.Name} = std::variant<{string.Join(", ", adt.Variants.Select(v => v.Name))}>;");
            }
            return sb.ToString();
        }

        private string EmitEnumDef(PilotIrEnumDefStmt en)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}enum class {en.Name} {{");
            _indent++;
            foreach (var (mName, mVal) in en.Members)
            {
                string valPart = mVal != null ? $" = {EmitExpr(mVal)}" : "";
                sb.AppendLine($"{Indent}{mName}{valPart},");
            }
            _indent--;
            sb.Append($"{Indent}}};");
            return sb.ToString();
        }

        private string EmitExtensionDef(PilotIrExtensionDefStmt ext)
        {
            var sb = new StringBuilder();
            foreach (var m in ext.Methods)
            {
                var pList = new List<string> { $"const {MapType(ext.TargetType)}& self_val" };
                foreach (var p in m.Parameters)
                {
                    if (p.Name is not ("self" or "this"))
                        pList.Add($"{MapType(p.TypeAnnotation ?? "auto")} {p.Name}");
                }
                string ret = !string.IsNullOrEmpty(m.DeclaredReturnType) ? MapType(m.DeclaredReturnType) : "auto";
                sb.AppendLine($"{Indent}inline {ret} {ext.TargetType}_{m.Name}({string.Join(", ", pList)}) {{");
                _indent++;
                foreach (var s in m.Body)
                {
                    string line = EmitStmt(s).Replace("this->", "self_val.").Replace("self.", "self_val.").Replace("(*this)", "self_val");
                    if (!string.IsNullOrWhiteSpace(line))
                        sb.AppendLine(line);
                }
                _indent--;
                sb.AppendLine($"{Indent}}}");
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitIfStmt(PilotIrIfStmt ifs)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}if ({EmitExpr(ifs.Condition)}) {{");
            EmitSuiteBody(sb, ifs.ThenBody);
            sb.Append($"{Indent}}}");

            foreach (var (cond, body) in ifs.ElifBranches)
            {
                sb.AppendLine($" else if ({EmitExpr(cond)}) {{");
                EmitSuiteBody(sb, body);
                sb.Append($"{Indent}}}");
            }
            if (ifs.ElseBody.Count > 0)
            {
                sb.AppendLine(" else {");
                EmitSuiteBody(sb, ifs.ElseBody);
                sb.Append($"{Indent}}}");
            }
            return sb.ToString();
        }

        private string EmitGuardStmt(PilotIrGuardStmt g)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}if (!({EmitExpr(g.ConditionOrSource)})) {{");
            EmitSuiteBody(sb, g.ElseBody);
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitWhileStmt(PilotIrWhileStmt w)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}while ({EmitExpr(w.Condition)}) {{");
            EmitSuiteBody(sb, w.Body);
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitForStmt(PilotIrForStmt f)
        {
            var sb = new StringBuilder();
            string target = f.TargetVars.Count > 0 ? f.TargetVars[0] : "_item";
            if (f.Iterable is PilotIrRangeExpr r)
            {
                string cmp = r.IsInclusive ? "<=" : "<";
                string step = r.Step != null ? $"{target} += {EmitExpr(r.Step)}" : $"++{target}";
                sb.AppendLine($"{Indent}for (int64_t {target} = {EmitExpr(r.Start)}; {target} {cmp} {EmitExpr(r.End)}; {step}) {{");
            }
            else
            {
                sb.AppendLine($"{Indent}for (const auto& {target} : {EmitExpr(f.Iterable)}) {{");
            }
            _indent++;
            if (f.WhereFilter != null)
                sb.AppendLine($"{Indent}if (!({EmitExpr(f.WhereFilter)})) continue;");
            foreach (var s in f.Body)
            {
                string line = EmitStmt(s);
                if (!string.IsNullOrWhiteSpace(line))
                    sb.AppendLine(line);
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitMatchStmt(PilotIrMatchStmt m)
        {
            var sb = new StringBuilder();
            string scrut = EmitExpr(m.Scrutinee);
            for (int i = 0; i < m.Arms.Count; i++)
            {
                var arm = m.Arms[i];
                string kw = i == 0 ? $"{Indent}if" : " else if";
                if (arm.Pattern is PilotIrLiteralPattern lit)
                    sb.AppendLine($"{kw} ({scrut} == {EmitExpr(lit.Literal)}) {{");
                else
                    sb.AppendLine(i == 0 ? $"{Indent}if (true) {{" : " else {");
                EmitSuiteBody(sb, arm.Body);
                sb.Append($"{Indent}}}");
            }
            return sb.ToString();
        }

        private string EmitTryStmt(PilotIrTryStmt t)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}try {{");
            EmitSuiteBody(sb, t.TryBody);
            sb.Append($"{Indent}}}");
            foreach (var c in t.Catches)
            {
                string varPart = !string.IsNullOrEmpty(c.VariableName) ? $"const std::exception& {c.VariableName}" : "...";
                sb.AppendLine($" catch ({varPart}) {{");
                EmitSuiteBody(sb, c.Body);
                sb.Append($"{Indent}}}");
            }
            return sb.ToString();
        }

        private void EmitSuiteBody(StringBuilder sb, List<PilotIrStmt> stmts)
        {
            _indent++;
            foreach (var s in stmts)
            {
                string line = EmitStmt(s);
                if (!string.IsNullOrWhiteSpace(line))
                    sb.AppendLine(line);
            }
            _indent--;
        }

        private string EmitExprStmt(PilotIrExprStmt e)
        {
            if (e.Expression is PilotIrCallExpr { LibraryCallKind: PilotLibraryCallKind.Println or PilotLibraryCallKind.Print } call)
            {
                return $"{Indent}{EmitCoutCall(call, newline: call.LibraryCallKind == PilotLibraryCallKind.Println)};";
            }
            return $"{Indent}{EmitExpr(e.Expression)};";
        }

        private string EmitCoutCall(PilotIrCallExpr call, bool newline)
        {
            var sb = new StringBuilder("std::cout");
            if (call.Arguments.Count > 0)
            {
                var arg0 = call.Arguments[0].Value;
                if (arg0 is PilotIrInterpolatedStringExpr fstr)
                {
                    foreach (var p in fstr.Parts)
                    {
                        if (!p.IsInterpolation)
                        {
                            if (!string.IsNullOrEmpty(p.LiteralText))
                                sb.Append($" << \"{p.LiteralText}\"");
                        }
                        else
                        {
                            string expr = p.Expression != null ? EmitExpr(p.Expression) : p.LiteralText;
                            sb.Append($" << ({expr})");
                        }
                    }
                }
                else
                {
                    sb.Append($" << {EmitExpr(arg0)}");
                }
            }
            if (newline)
                sb.Append(" << '\\n'");
            return sb.ToString();
        }

        public string EmitExpr(PilotIrExpr expr, PilotIrPrecedence parentPrec = PilotIrPrecedence.Lowest)
        {
            string core = expr switch
            {
                PilotIrLiteralExpr lit => EmitLiteral(lit),
                PilotIrInterpolatedStringExpr fstr => EmitInterpolatedExpr(fstr),
                PilotIrIdentifierExpr id => id.Name switch
                {
                    "null" or "none" or "None" or "undefined" => "std::nullopt",
                    "true" or "True" => "true",
                    "false" or "False" => "false",
                    "self" => "(*this)",
                    _ => id.Name
                },
                PilotIrUnaryExpr u => $"{(u.Operator == "not" ? "!" : u.Operator)}{EmitExpr(u.Operand, u.Precedence)}",
                PilotIrBinaryExpr b => EmitBinary(b),
                PilotIrCallExpr c => EmitCall(c),
                PilotIrMemberAccessExpr m => EmitMemberAccess(m),
                PilotIrIndexExpr ix => $"{EmitExpr(ix.Target, PilotIrPrecedence.Postfix)}[{EmitExpr(ix.Index)}]",
                PilotIrPostfixTryExpr pt => pt.CatchFallbackExpr != null
                    ? $"{EmitExpr(pt.Operand, PilotIrPrecedence.Postfix)}.value_or({EmitExpr(pt.CatchFallbackExpr)})"
                    : EmitExpr(pt.Operand, parentPrec),
                PilotIrCollectionExpr col => $"std::vector<int64_t>{{{string.Join(", ", col.Elements.Select(e => EmitExpr(e)))}}}",
                PilotIrLambdaExpr lam => $"[&]({string.Join(", ", lam.Parameters.Select(p => $"auto {p.Name}"))}) {{ return {(lam.BodyExpr != null ? EmitExpr(lam.BodyExpr) : "0")}; }}",
                PilotIrForExpr fe => EmitForExpr(fe),
                PilotIrIfExpr ie => $"{EmitExpr(ie.Condition, PilotIrPrecedence.Conditional)} ? {EmitExpr(ie.ThenBranch, PilotIrPrecedence.Conditional)} : {EmitExpr(ie.ElseBranch, PilotIrPrecedence.Conditional)}",
                _ => "0"
            };

            if (parentPrec != PilotIrPrecedence.Lowest && expr.Precedence < parentPrec)
                return $"({core})";
            return core;
        }

        private static string EmitLiteral(PilotIrLiteralExpr lit) => lit.Kind switch
        {
            PilotLiteralKind.Null or PilotLiteralKind.None => "std::nullopt",
            PilotLiteralKind.Boolean => lit.RawValue is "true" or "True" ? "true" : "false",
            PilotLiteralKind.DimensionalUnit => (lit.NormalizedNumericValue ?? 0.0).ToString(System.Globalization.CultureInfo.InvariantCulture),
            _ => lit.RawValue
        };

        private string EmitInterpolatedExpr(PilotIrInterpolatedStringExpr fstr)
        {
            var parts = new List<string>();
            foreach (var p in fstr.Parts)
            {
                if (!p.IsInterpolation)
                {
                    if (!string.IsNullOrEmpty(p.LiteralText))
                        parts.Add($"std::string(\"{p.LiteralText}\")");
                }
                else
                {
                    string inner = p.Expression != null ? EmitExpr(p.Expression) : p.LiteralText;
                    if (p.Expression is PilotIrLiteralExpr { Kind: PilotLiteralKind.String })
                        parts.Add($"std::string({inner})");
                    else
                        parts.Add($"std::to_string({inner})");
                }
            }
            return parts.Count > 0 ? $"({string.Join(" + ", parts)})" : "std::string(\"\")";
        }

        private string EmitBinary(PilotIrBinaryExpr b)
        {
            if (b.Operator == "**")
                return $"std::pow({EmitExpr(b.Left)}, {EmitExpr(b.Right)})";
            if (b.Operator == "|>")
                return $"{EmitExpr(b.Right, PilotIrPrecedence.Postfix)}({EmitExpr(b.Left)})";
            if (b.Operator == "??")
                return $"{EmitExpr(b.Left, PilotIrPrecedence.Postfix)}.value_or({EmitExpr(b.Right)})";

            string cppOp = b.Operator switch
            {
                "and" => "&&",
                "or" => "||",
                "//" => "/",
                _ => b.Operator
            };
            string left = EmitExpr(b.Left, b.Precedence);
            var rightMinPrec = b.Operator is "-" or "/" or "//" or "%"
                ? (PilotIrPrecedence)((int)b.Precedence + 1)
                : b.Precedence;
            string right = EmitExpr(b.Right, rightMinPrec);
            return $"{left} {cppOp} {right}";
        }

        private string EmitCall(PilotIrCallExpr c)
        {
            if (c.LibraryCallKind is PilotLibraryCallKind.Println or PilotLibraryCallKind.Print)
                return EmitCoutCall(c, newline: c.LibraryCallKind == PilotLibraryCallKind.Println);

            var renderedArgs = c.Arguments.Select(a => EmitExpr(a.Value)).ToList();
            if (PilotLibraryMapper.TryFormatAstLibraryCall(c.LibraryCallKind, renderedArgs, "cpp", out string mapped))
                return mapped;

            if (c.Callee is PilotIrMemberAccessExpr mem && mem.MemberName is "append" or "push")
                return $"{EmitExpr(mem.Target, PilotIrPrecedence.Postfix)}.push_back({string.Join(", ", renderedArgs)})";

            string callee = EmitExpr(c.Callee, PilotIrPrecedence.Postfix);
            return $"{callee}({string.Join(", ", renderedArgs)})";
        }

        private string EmitMemberAccess(PilotIrMemberAccessExpr m)
        {
            string target = EmitExpr(m.Target, PilotIrPrecedence.Postfix);
            if (PilotLibraryMapper.TryFormatAstMemberAccess(target, m.MemberName, "cpp", out string mapped))
                return mapped;
            return $"{target}.{m.MemberName}";
        }

        private string EmitForExpr(PilotIrForExpr fe)
        {
            string target = fe.TargetVars.Count > 0 ? fe.TargetVars[0] : "x";
            string bodyFilter = fe.FilterCondition != null ? $"if (!({EmitExpr(fe.FilterCondition)})) continue; " : "";
            string loopHeader = fe.Iterable is PilotIrRangeExpr r
                ? $"for (int64_t {target} = {EmitExpr(r.Start)}; {target} {(r.IsInclusive ? "<=" : "<")} {EmitExpr(r.End)}; ++{target})"
                : $"for (const auto& {target} : {EmitExpr(fe.Iterable)})";
            return $"([&]() {{ std::vector<int64_t> __res; {loopHeader} {{ {bodyFilter}__res.push_back({EmitExpr(fe.YieldExpr)}); }} return __res; }})()";
        }

        private string MapType(string pilotType)
        {
            string t = pilotType.Trim();
            if (_options.TypeMappings.TryGetValue(t, out var mapped))
                return mapped;

            if (t.EndsWith("?"))
                return $"std::optional<{MapType(t[..^1])}>";
            if (t.Contains('!'))
                return MapType(t.Split('!', 2)[0].Trim());
            if (t.StartsWith("list[") && t.EndsWith("]"))
                return $"std::vector<{MapType(t[5..^1])}>";

            return t switch
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
                "str" or "string" => "std::string",
                "list" => "std::vector<int64_t>",
                "void" => "void",
                "any" or "auto" => "auto",
                _ => t
            };
        }
    }
}
