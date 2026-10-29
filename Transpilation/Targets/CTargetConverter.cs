using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Pilot.Analysis;
using Pilot.Analysis.TypeSystem;
using Pilot.Transpilation.Ingesters;
using Pilot.Transpilation.Ir;

namespace Pilot.Transpilation.Targets;

/// <summary>
/// Milestone 20.3 &amp; 20.4: AST-driven bidirectional code converter between Pilot and C (C11 / C23).
/// Emits exact hardware integer types (<c>&lt;stdint.h&gt;</c>), tagged-union structs for ADTs,
/// <c>static inline</c> functions, <c>const</c> promotion, and automatic <c>main()</c> wrapping,
/// compilable by <c>clang -std=c11 -fsyntax-only</c>.
/// </summary>
public sealed class CTargetConverter : ITargetLanguageConverter
{
    public string Language => "c";
    public string TargetLanguage => Language;
    public string SourceLanguage => Language;

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
            sb.AppendLine("/* Code transpiled & optimized from Pilot to C (C11/C23) */");
            sb.AppendLine("#include <stdint.h>");
            sb.AppendLine("#include <stdbool.h>");
            sb.AppendLine("#include <stdio.h>");
            sb.AppendLine("#include <stdlib.h>");
            sb.AppendLine("#include <string.h>");
            sb.AppendLine("#include <math.h>");
            sb.AppendLine("#include <assert.h>");
            sb.AppendLine();
        }

        var emitter = new CIrEmitter(module, options);
        sb.Append(emitter.EmitModule());

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options) =>
        ForeignAstIngester.Ingest(foreignSource, SourceLanguage, options);

    private sealed class CIrEmitter
    {
        private readonly PilotIrModule _module;
        private readonly TranspilationOptions _options;
        private readonly Dictionary<string, string> _varTypes = new(StringComparer.Ordinal);
        private int _indent;

        public CIrEmitter(PilotIrModule module, TranspilationOptions options)
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
                topDefs.AppendLine("int main(void) {");
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
            PilotIrCommentStmt c => $"{Indent}/* {c.CommentText.TrimStart('/').Trim()} */",
            PilotIrVarDeclStmt v => EmitVarDecl(v),
            PilotIrAssignStmt a => $"{Indent}{EmitExpr(a.Target)} {a.Operator} {EmitExpr(a.Value)};",
            PilotIrFuncDefStmt f => EmitFuncDef(f, prefixName: null),
            PilotIrClassDefStmt c => EmitClassDef(c),
            PilotIrAdtDefStmt adt => EmitAdtDef(adt),
            PilotIrEnumDefStmt en => EmitEnumDef(en),
            PilotIrTypeAliasStmt ta => $"{Indent}typedef {MapType(ta.TargetType)} {ta.Name};",
            PilotIrExtensionDefStmt ext => EmitExtensionDef(ext),
            PilotIrIfStmt ifs => EmitIfStmt(ifs),
            PilotIrGuardStmt g => EmitGuardStmt(g),
            PilotIrWhileStmt w => EmitWhileStmt(w),
            PilotIrForStmt f => EmitForStmt(f),
            PilotIrMatchStmt m => EmitMatchStmt(m),
            PilotIrDeferStmt d => string.Join(Environment.NewLine, d.Body.Select(EmitStmt)),
            PilotIrReturnStmt r => r.Value != null ? $"{Indent}return {EmitExpr(r.Value)};" : $"{Indent}return;",
            PilotIrBreakStmt => $"{Indent}break;",
            PilotIrContinueStmt => $"{Indent}continue;",
            PilotIrPassStmt => $"{Indent}/* pass */",
            PilotIrAssertStmt a => $"{Indent}assert({EmitExpr(a.Condition)});",
            PilotIrComptimeStmt ct => string.Join(Environment.NewLine, ct.Body.Select(EmitStmt)),
            PilotIrExprStmt e => EmitExprStmt(e),
            _ => ""
        };

        private string EmitVarDecl(PilotIrVarDeclStmt v)
        {
            string cType = !string.IsNullOrEmpty(v.DeclaredType)
                ? MapType(v.DeclaredType)
                : InferCType(v.Initializer, v.ResolvedType);
            _varTypes[v.Name] = cType;

            string rhs = v.ComptimeFoldedLiteral ?? (v.Initializer != null ? EmitExpr(v.Initializer) : DefaultValueForCType(cType));
            string constMod = (v.Kind == PilotVarDeclKind.Const || (v.Kind == PilotVarDeclKind.Let && !v.IsMutated && v.Initializer != null))
                ? "const "
                : "";
            if (cType.StartsWith("const ") && constMod.Length > 0)
                constMod = "";
            return $"{Indent}{constMod}{cType} {v.Name} = {rhs};";
        }

        private string EmitFuncDef(PilotIrFuncDefStmt f, string? prefixName)
        {
            string fnName = prefixName != null ? $"{prefixName}_{f.Name}" : f.Name;
            var paramList = new List<string>();
            foreach (var p in f.Parameters)
            {
                if (p.Name is "self" or "this")
                {
                    if (prefixName != null)
                        paramList.Add($"{MapType(prefixName)} self_val");
                    continue;
                }
                string cType = !string.IsNullOrEmpty(p.TypeAnnotation) ? MapType(p.TypeAnnotation) : "int64_t";
                _varTypes[p.Name] = cType;
                paramList.Add($"{cType} {p.Name}");
            }

            string cRet = fnName == "main" ? "int" : MapType(f.EffectiveReturnType);
            string paramsStr = paramList.Count > 0 ? string.Join(", ", paramList) : "void";
            string mod = fnName == "main" ? "" : "static inline ";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}{mod}{cRet} {fnName}({paramsStr}) {{");
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
            sb.AppendLine($"{Indent}typedef struct {c.Name} {{");
            _indent++;
            if (c.Fields.Count == 0)
            {
                sb.AppendLine($"{Indent}int64_t _reserved;");
            }
            else
            {
                foreach (var f in c.Fields)
                {
                    sb.AppendLine($"{Indent}{MapType(f.TypeAnnotation)} {f.Name};");
                }
            }
            _indent--;
            sb.Append($"{Indent}}} {c.Name};");

            foreach (var m in c.Methods)
            {
                sb.AppendLine();
                sb.Append(EmitFuncDef(m, prefixName: c.Name));
            }
            return sb.ToString();
        }

        private string EmitAdtDef(PilotIrAdtDefStmt adt)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}typedef enum {adt.Name}Tag {{");
            _indent++;
            foreach (var v in adt.Variants)
                sb.AppendLine($"{Indent}{adt.Name}_{v.Name},");
            _indent--;
            sb.AppendLine($"{Indent}}} {adt.Name}Tag;");
            sb.AppendLine();
            sb.AppendLine($"{Indent}typedef struct {adt.Name} {{");
            _indent++;
            sb.AppendLine($"{Indent}{adt.Name}Tag tag;");
            if (adt.Variants.Any(v => v.Fields.Count > 0))
            {
                sb.AppendLine($"{Indent}union {{");
                _indent++;
                foreach (var v in adt.Variants.Where(v => v.Fields.Count > 0))
                {
                    sb.AppendLine($"{Indent}struct {{ {string.Join(" ", v.Fields.Select(f => $"{MapType(f.Type)} {f.Name};"))} }} {v.Name};");
                }
                _indent--;
                sb.AppendLine($"{Indent}}} data;");
            }
            _indent--;
            sb.Append($"{Indent}}} {adt.Name};");
            return sb.ToString();
        }

        private string EmitEnumDef(PilotIrEnumDefStmt en)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}typedef enum {en.Name} {{");
            _indent++;
            foreach (var (mName, mVal) in en.Members)
            {
                string valPart = mVal != null ? $" = {EmitExpr(mVal)}" : "";
                sb.AppendLine($"{Indent}{en.Name}_{mName}{valPart},");
            }
            _indent--;
            sb.Append($"{Indent}}} {en.Name};");
            return sb.ToString();
        }

        private string EmitExtensionDef(PilotIrExtensionDefStmt ext)
        {
            var sb = new StringBuilder();
            foreach (var m in ext.Methods)
            {
                if (m.Parameters.Count == 0 || m.Parameters[0].Name is not ("self" or "this"))
                    m.Parameters.Insert(0, new PilotIrParameter("self", ext.TargetType));
                sb.AppendLine(EmitFuncDef(m, prefixName: ext.TargetType));
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
            string target = f.TargetVars.Count > 0 ? f.TargetVars[0] : "_i";
            _varTypes[target] = "int64_t";

            if (f.Iterable is PilotIrRangeExpr r)
            {
                string cmp = r.IsInclusive ? "<=" : "<";
                string step = r.Step != null ? $"{target} += {EmitExpr(r.Step)}" : $"++{target}";
                sb.AppendLine($"{Indent}for (int64_t {target} = {EmitExpr(r.Start)}; {target} {cmp} {EmitExpr(r.End)}; {step}) {{");
            }
            else
            {
                sb.AppendLine($"{Indent}for (int64_t {target} = 0; {target} < 1; ++{target}) {{");
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
                if (arm.Pattern is PilotIrWildcardPattern or PilotIrBindingPattern)
                {
                    sb.AppendLine(i == 0 ? $"{Indent}if (1) {{" : " else {");
                }
                else if (arm.Pattern is PilotIrLiteralPattern lit)
                {
                    sb.AppendLine($"{kw} ({scrut} == {EmitExpr(lit.Literal)}) {{");
                }
                else
                {
                    sb.AppendLine(i == 0 ? $"{Indent}if (1) {{" : " else {");
                }
                EmitSuiteBody(sb, arm.Body);
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
                return $"{Indent}{EmitPrintCall(call, newline: call.LibraryCallKind == PilotLibraryCallKind.Println)};";
            }
            return $"{Indent}{EmitExpr(e.Expression)};";
        }

        private string EmitPrintCall(PilotIrCallExpr call, bool newline)
        {
            string nl = newline ? "\\n" : "";
            if (call.Arguments.Count == 0)
                return $"printf(\"{nl}\")";

            var arg0 = call.Arguments[0].Value;
            if (arg0 is PilotIrInterpolatedStringExpr fstr)
            {
                var fmt = new StringBuilder("\"");
                var args = new List<string>();
                foreach (var p in fstr.Parts)
                {
                    if (!p.IsInterpolation)
                    {
                        fmt.Append(p.LiteralText);
                    }
                    else
                    {
                        var expr = p.Expression;
                        string rendered = expr != null ? EmitExpr(expr) : p.LiteralText;
                        if (expr != null && IsStringExpr(expr))
                        {
                            fmt.Append("%s");
                            args.Add(rendered);
                        }
                        else if (expr != null && IsFloatExpr(expr))
                        {
                            fmt.Append("%g");
                            args.Add($"(double)({rendered})");
                        }
                        else
                        {
                            fmt.Append("%lld");
                            args.Add($"(long long)({rendered})");
                        }
                    }
                }
                fmt.Append($"{nl}\"");
                return args.Count > 0
                    ? $"printf({fmt}, {string.Join(", ", args)})"
                    : $"printf({fmt})";
            }

            if (IsStringExpr(arg0))
                return $"printf(\"%s{nl}\", {EmitExpr(arg0)})";
            if (IsFloatExpr(arg0))
                return $"printf(\"%g{nl}\", (double)({EmitExpr(arg0)}))";
            return $"printf(\"%lld{nl}\", (long long)({EmitExpr(arg0)}))";
        }

        private bool IsStringExpr(PilotIrExpr expr) =>
            expr is PilotIrLiteralExpr { Kind: PilotLiteralKind.String } ||
            expr is PilotIrInterpolatedStringExpr ||
            expr.ResolvedType == PilotType.String ||
            (expr is PilotIrIdentifierExpr id && _varTypes.TryGetValue(id.Name, out var ct) && ct.Contains("char*"));

        private bool IsFloatExpr(PilotIrExpr expr) =>
            expr is PilotIrLiteralExpr { Kind: PilotLiteralKind.Float } ||
            expr.ResolvedType == PilotType.Float ||
            (expr is PilotIrIdentifierExpr id && _varTypes.TryGetValue(id.Name, out var ct) && ct is "double" or "float");

        public string EmitExpr(PilotIrExpr expr, PilotIrPrecedence parentPrec = PilotIrPrecedence.Lowest)
        {
            string core = expr switch
            {
                PilotIrLiteralExpr lit => EmitLiteral(lit),
                PilotIrInterpolatedStringExpr fstr => $"\"{string.Concat(fstr.Parts.Select(p => p.IsInterpolation ? "%s" : p.LiteralText))}\"",
                PilotIrIdentifierExpr id => id.Name switch
                {
                    "null" or "none" or "None" or "undefined" => "NULL",
                    "true" or "True" => "true",
                    "false" or "False" => "false",
                    "self" or "this" => "self_val",
                    _ => id.Name
                },
                PilotIrUnaryExpr u => $"{(u.Operator == "not" ? "!" : u.Operator)}{EmitExpr(u.Operand, u.Precedence)}",
                PilotIrBinaryExpr b => EmitBinary(b),
                PilotIrCallExpr c => EmitCall(c),
                PilotIrMemberAccessExpr m => EmitMemberAccess(m),
                PilotIrIndexExpr ix => $"{EmitExpr(ix.Target, PilotIrPrecedence.Postfix)}[{EmitExpr(ix.Index)}]",
                PilotIrPostfixTryExpr pt => EmitExpr(pt.Operand, parentPrec),
                PilotIrIfExpr ie => $"{EmitExpr(ie.Condition, PilotIrPrecedence.Conditional)} ? {EmitExpr(ie.ThenBranch, PilotIrPrecedence.Conditional)} : {EmitExpr(ie.ElseBranch, PilotIrPrecedence.Conditional)}",
                _ => "0"
            };

            if (parentPrec != PilotIrPrecedence.Lowest && expr.Precedence < parentPrec)
                return $"({core})";
            return core;
        }

        private static string EmitLiteral(PilotIrLiteralExpr lit) => lit.Kind switch
        {
            PilotLiteralKind.Null or PilotLiteralKind.None => "NULL",
            PilotLiteralKind.Boolean => lit.RawValue is "true" or "True" ? "true" : "false",
            PilotLiteralKind.DimensionalUnit => (lit.NormalizedNumericValue ?? 0.0).ToString(System.Globalization.CultureInfo.InvariantCulture),
            _ => lit.RawValue
        };

        private string EmitBinary(PilotIrBinaryExpr b)
        {
            if (b.Operator == "**")
                return $"pow((double)({EmitExpr(b.Left)}), (double)({EmitExpr(b.Right)}))";
            if (b.Operator == "|>")
                return $"{EmitExpr(b.Right, PilotIrPrecedence.Postfix)}({EmitExpr(b.Left)})";

            string cOp = b.Operator switch
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
            return $"{left} {cOp} {right}";
        }

        private string EmitCall(PilotIrCallExpr c)
        {
            if (c.LibraryCallKind is PilotLibraryCallKind.Println or PilotLibraryCallKind.Print)
                return EmitPrintCall(c, newline: c.LibraryCallKind == PilotLibraryCallKind.Println);

            var renderedArgs = c.Arguments.Select(a => EmitExpr(a.Value)).ToList();
            if (PilotLibraryMapper.TryFormatAstLibraryCall(c.LibraryCallKind, renderedArgs, "c", out string mapped))
                return mapped;

            string callee = EmitExpr(c.Callee, PilotIrPrecedence.Postfix);
            return $"{callee}({string.Join(", ", renderedArgs)})";
        }

        private string EmitMemberAccess(PilotIrMemberAccessExpr m)
        {
            string target = EmitExpr(m.Target, PilotIrPrecedence.Postfix);
            if (PilotLibraryMapper.TryFormatAstMemberAccess(target, m.MemberName, "c", out string mapped))
                return mapped;
            return $"{target}.{m.MemberName}";
        }

        private static string InferCType(PilotIrExpr? init, PilotType resolved)
        {
            if (resolved == PilotType.String || init is PilotIrLiteralExpr { Kind: PilotLiteralKind.String } || init is PilotIrInterpolatedStringExpr)
                return "const char*";
            if (resolved == PilotType.Bool || init is PilotIrLiteralExpr { Kind: PilotLiteralKind.Boolean })
                return "bool";
            if (resolved == PilotType.Float || init is PilotIrLiteralExpr { Kind: PilotLiteralKind.Float })
                return "double";
            return "int64_t";
        }

        private static string DefaultValueForCType(string cType) => cType switch
        {
            "const char*" or "char*" => "NULL",
            "bool" => "false",
            "double" or "float" => "0.0",
            _ => "0"
        };

        private string MapType(string pilotType)
        {
            string t = pilotType.Trim().TrimEnd('?');
            if (t.Contains('!'))
                t = t.Split('!', 2)[0].Trim();
            if (_options.TypeMappings.TryGetValue(t, out var mapped))
                return mapped;

            return t switch
            {
                "i8" => "int8_t",
                "u8" => "uint8_t",
                "i16" => "int16_t",
                "u16" => "uint16_t",
                "i32" => "int32_t",
                "u32" => "uint32_t",
                "i64" or "int" or "auto" or "any" => "int64_t",
                "u64" or "uint" => "uint64_t",
                "f8" or "f16" or "f32" => "float",
                "f64" or "float" or "double" => "double",
                "bool" => "bool",
                "str" or "string" => "const char*",
                "void" => "void",
                _ => t
            };
        }
    }
}
