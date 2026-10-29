using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Pilot.Analysis;
using Pilot.Transpilation.Ingesters;
using Pilot.Transpilation.Ir;

namespace Pilot.Transpilation.Targets;

/// <summary>
/// Milestone 20.3 &amp; 20.4: AST-driven bidirectional code converter between Pilot and modern JavaScript (ES2024+).
/// Emits immutability-aware <c>const</c>/<c>let</c> bindings, strict equality (<c>===</c>/<c>!==</c>),
/// template literals (<c>`...${...}`</c>), prototype extensions, and comprehension chains compilable by <c>node --check</c>.
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
        bool isTypeScript = Language.ToLowerInvariant() is "ts" or "typescript";
        var sb = new StringBuilder();
        if (options.EmitHeaderComments)
        {
            if (isTypeScript)
            {
                sb.AppendLine("// Code transpiled & optimized from Pilot to TypeScript");
                sb.AppendLine();
            }
            else
            {
                sb.AppendLine("// Code transpiled & optimized from Pilot to JavaScript (ES2024+)");
                sb.AppendLine("\"use strict\";");
                sb.AppendLine();
            }
        }

        var emitter = new JsIrEmitter(module, options, isTypeScript);
        sb.Append(emitter.EmitModule());

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options) =>
        ForeignAstIngester.Ingest(foreignSource, SourceLanguage, options);

    private sealed class JsIrEmitter
    {
        private readonly PilotIrModule _module;
        private readonly TranspilationOptions _options;
        private readonly bool _isTypeScript;
        private readonly HashSet<string> _knownClasses = new(StringComparer.Ordinal);
        private int _indent;

        public JsIrEmitter(PilotIrModule module, TranspilationOptions options, bool isTypeScript)
        {
            _module = module;
            _options = options;
            _isTypeScript = isTypeScript;
            foreach (var c in module.Statements.OfType<PilotIrClassDefStmt>())
                _knownClasses.Add(c.Name);
        }

        private string Indent => string.Concat(Enumerable.Repeat(_options.IndentString, _indent));

        public string EmitModule()
        {
            var sb = new StringBuilder();
            foreach (var stmt in _module.Statements)
            {
                string s = EmitStmt(stmt);
                if (!string.IsNullOrWhiteSpace(s))
                {
                    sb.AppendLine(s);
                }
            }
            return sb.ToString();
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
            PilotIrExtensionDefStmt ext => EmitExtensionDef(ext),
            PilotIrIfStmt ifs => EmitIfStmt(ifs),
            PilotIrGuardStmt g => EmitGuardStmt(g),
            PilotIrWhileStmt w => EmitWhileStmt(w),
            PilotIrForStmt f => EmitForStmt(f),
            PilotIrMatchStmt m => EmitMatchStmt(m),
            PilotIrTryStmt t => EmitTryStmt(t),
            PilotIrDeferStmt d => string.Join(Environment.NewLine, d.Body.Select(EmitStmt)),
            PilotIrReturnStmt r => r.Value != null ? $"{Indent}return {EmitExpr(r.Value)};" : $"{Indent}return;",
            PilotIrRaiseStmt rs => rs.Value != null ? $"{Indent}throw new Error(String({EmitExpr(rs.Value)}));" : $"{Indent}throw new Error();",
            PilotIrBreakStmt => $"{Indent}break;",
            PilotIrContinueStmt => $"{Indent}continue;",
            PilotIrPassStmt => $"{Indent}// pass",
            PilotIrAssertStmt a => a.Message != null
                ? $"{Indent}if (!({EmitExpr(a.Condition)})) throw new Error({EmitExpr(a.Message)});"
                : $"{Indent}if (!({EmitExpr(a.Condition)})) throw new Error(\"Assertion failed\");",
            PilotIrComptimeStmt ct => string.Join(Environment.NewLine, ct.Body.Select(EmitStmt)),
            PilotIrExprStmt e => $"{Indent}{EmitExpr(e.Expression)};",
            _ => ""
        };

        private string EmitVarDecl(PilotIrVarDeclStmt v)
        {
            if (v.TupleDestructureNames.Count > 1)
            {
                string lhs = $"[{string.Join(", ", v.TupleDestructureNames)}]";
                string rhs = v.Initializer != null ? EmitExpr(v.Initializer) : "[]";
                return $"{Indent}const {lhs} = {rhs};";
            }

            string rhsVal = v.ComptimeFoldedLiteral ?? (v.Initializer != null ? EmitExpr(v.Initializer) : "null");
            string kw = (v.Kind == PilotVarDeclKind.Var || v.IsMutated || v.Initializer == null) ? "let" : "const";
            string typeAnnot = (_isTypeScript && _options.GenerateTypeAnnotations && !string.IsNullOrEmpty(v.DeclaredType))
                ? $": {MapType(v.DeclaredType)}"
                : "";
            return $"{Indent}{kw} {v.Name}{typeAnnot} = {rhsVal};";
        }

        private string EmitFuncDef(PilotIrFuncDefStmt f, bool isMethod)
        {
            var paramList = new List<string>();
            foreach (var p in f.Parameters)
            {
                if (p.Name is "self" or "this")
                    continue;
                string pStr = (p.IsVariadic ? "..." : "") + p.Name;
                if (_isTypeScript && _options.GenerateTypeAnnotations && !string.IsNullOrEmpty(p.TypeAnnotation))
                    pStr += $": {MapType(p.TypeAnnotation)}";
                if (p.DefaultValue != null)
                    pStr += $" = {EmitExpr(p.DefaultValue)}";
                paramList.Add(pStr);
            }

            string asyncPrefix = f.IsAsync ? "async " : "";
            bool isCtor = isMethod && f.Name == "__init__";
            string retAnnot = (_isTypeScript && _options.GenerateTypeAnnotations && !isCtor && !string.IsNullOrEmpty(f.DeclaredReturnType))
                ? $": {MapType(f.DeclaredReturnType)}"
                : "";
            string fnHeader = isMethod
                ? $"{(f.IsStatic ? "static " : "")}{asyncPrefix}{(isCtor ? "constructor" : f.Name)}({string.Join(", ", paramList)}){retAnnot}"
                : $"{asyncPrefix}function {f.Name}({string.Join(", ", paramList)}){retAnnot}";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}{fnHeader} {{");
            _indent++;
            foreach (var req in f.RequiresClauses)
                sb.AppendLine($"{Indent}if (!({EmitExpr(req)})) throw new Error(\"Contract requires failed\");");
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
            string ext = c.BaseTypes.Count > 0 ? $" extends {c.BaseTypes[0]}" : "";
            sb.AppendLine($"{Indent}class {c.Name}{ext} {{");
            _indent++;
            if (_isTypeScript)
            {
                foreach (var f in c.Fields)
                {
                    sb.AppendLine($"{Indent}{f.Name}: {MapType(f.TypeAnnotation)};");
                }
            }
            if (c.Fields.Count > 0 && !c.Methods.Any(m => m.Name is "__init__" or "constructor"))
            {
                var ctorParams = c.Fields.Select(f =>
                {
                    string tPart = _isTypeScript ? $": {MapType(f.TypeAnnotation)} | null" : "";
                    return f.DefaultValue != null
                        ? $"{f.Name}{(_isTypeScript ? $": {MapType(f.TypeAnnotation)}" : "")} = {EmitExpr(f.DefaultValue)}"
                        : $"{f.Name}{tPart} = null";
                });
                sb.AppendLine($"{Indent}constructor({string.Join(", ", ctorParams)}) {{");
                _indent++;
                foreach (var f in c.Fields)
                    sb.AppendLine($"{Indent}this.{f.Name} = {f.Name};");
                _indent--;
                sb.AppendLine($"{Indent}}}");
            }
            foreach (var m in c.Methods)
            {
                sb.AppendLine(EmitFuncDef(m, isMethod: true));
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitAdtDef(PilotIrAdtDefStmt adt)
        {
            var sb = new StringBuilder();
            foreach (var v in adt.Variants)
            {
                var argsList = v.Fields.Select(f =>
                    _isTypeScript && _options.GenerateTypeAnnotations
                        ? $"{f.Name}: {MapType(f.Type)}"
                        : f.Name).ToList();
                var propPairs = new List<string> { $"\"_tag\": \"{v.Name}\"" };
                propPairs.AddRange(v.Fields.Select(f => $"\"{f.Name}\": {f.Name}"));
                sb.AppendLine($"{Indent}function {v.Name}({string.Join(", ", argsList)}) {{");
                sb.AppendLine($"{Indent}{_options.IndentString}return {{ {string.Join(", ", propPairs)} }};");
                sb.AppendLine($"{Indent}}}");
            }
            if (_isTypeScript && adt.Variants.Count > 0)
            {
                string unionArms = string.Join(" | ", adt.Variants.Select(v => $"ReturnType<typeof {v.Name}>"));
                sb.AppendLine($"{Indent}type {adt.Name} = {unionArms};");
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitEnumDef(PilotIrEnumDefStmt en)
        {
            var sb = new StringBuilder();
            if (_isTypeScript)
            {
                sb.AppendLine($"{Indent}enum {en.Name} {{");
                _indent++;
                for (int i = 0; i < en.Members.Count; i++)
                {
                    var (mName, mVal) = en.Members[i];
                    string val = mVal != null ? EmitExpr(mVal) : i.ToString();
                    sb.AppendLine($"{Indent}{mName} = {val},");
                }
                _indent--;
                sb.Append($"{Indent}}}");
                return sb.ToString();
            }

            sb.AppendLine($"{Indent}const {en.Name} = Object.freeze({{");
            _indent++;
            for (int i = 0; i < en.Members.Count; i++)
            {
                var (mName, mVal) = en.Members[i];
                string val = mVal != null ? EmitExpr(mVal) : i.ToString();
                sb.AppendLine($"{Indent}{mName}: {val},");
            }
            _indent--;
            sb.Append($"{Indent}}});");
            return sb.ToString();
        }

        private static string MapType(string pilotType)
        {
            string t = pilotType.Trim();
            if (t.EndsWith("?"))
                return $"{MapType(t[..^1])} | null";
            if (t.Contains('!'))
                return MapType(t.Split('!', 2)[0].Trim());
            if ((t.StartsWith("list[") || t.StartsWith("[")) && t.EndsWith("]"))
            {
                string inner = t.StartsWith("list[") ? t[5..^1] : t[1..^1];
                return $"{MapType(inner)}[]";
            }

            return t switch
            {
                "i8" or "u8" or "i16" or "u16" or "i32" or "u32" or "i64" or "u64" or
                "int" or "uint" or "f8" or "f16" or "f32" or "f64" or "float" or "double" or "decimal" => "number",
                "str" or "string" => "string",
                "bool" => "boolean",
                "void" => "void",
                "list" => "number[]",
                "any" or "auto" => "any",
                _ => t
            };
        }

        private string EmitExtensionDef(PilotIrExtensionDefStmt ext)
        {
            var sb = new StringBuilder();
            string protoTarget = ext.TargetType switch
            {
                "str" or "string" => "String",
                "int" or "i32" or "i64" or "float" or "f64" => "Number",
                "list" => "Array",
                _ => ext.TargetType
            };
            foreach (var m in ext.Methods)
            {
                var pList = m.Parameters.Where(p => p.Name is not ("self" or "this")).Select(p => p.Name);
                sb.AppendLine($"{Indent}{protoTarget}.prototype.{m.Name} = function({string.Join(", ", pList)}) {{");
                _indent++;
                foreach (var s in m.Body)
                {
                    string line = EmitStmt(s);
                    if (!string.IsNullOrWhiteSpace(line))
                        sb.AppendLine(line);
                }
                _indent--;
                sb.AppendLine($"{Indent}}};");
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitIfStmt(PilotIrIfStmt ifs)
        {
            var sb = new StringBuilder();
            if (!string.IsNullOrEmpty(ifs.OptionalBindingVar))
            {
                sb.AppendLine($"{Indent}const {ifs.OptionalBindingVar} = {EmitExpr(ifs.Condition)};");
                sb.AppendLine($"{Indent}if ({ifs.OptionalBindingVar} !== null && {ifs.OptionalBindingVar} !== undefined) {{");
            }
            else
            {
                sb.AppendLine($"{Indent}if ({EmitExpr(ifs.Condition)}) {{");
            }
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
            if (g.IsSomeBinding && !string.IsNullOrEmpty(g.BoundVariable))
            {
                sb.AppendLine($"{Indent}const {g.BoundVariable} = {EmitExpr(g.ConditionOrSource)};");
                string cond = $"{g.BoundVariable} === null || {g.BoundVariable} === undefined";
                if (g.PatternGuard != null)
                    cond += $" || !({EmitExpr(g.PatternGuard)})";
                sb.AppendLine($"{Indent}if ({cond}) {{");
            }
            else
            {
                sb.AppendLine($"{Indent}if (!({EmitExpr(g.ConditionOrSource)})) {{");
            }
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
                string step = r.Step != null ? $"{target} += {EmitExpr(r.Step)}" : $"{target}++";
                sb.AppendLine($"{Indent}for (let {target} = {EmitExpr(r.Start)}; {target} {cmp} {EmitExpr(r.End)}; {step}) {{");
            }
            else
            {
                sb.AppendLine($"{Indent}for (const {target} of {EmitExpr(f.Iterable)}) {{");
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
            sb.AppendLine($"{Indent}switch ({EmitExpr(m.Scrutinee)}) {{");
            _indent++;
            foreach (var arm in m.Arms)
            {
                if (arm.Pattern is PilotIrWildcardPattern or PilotIrBindingPattern)
                    sb.AppendLine($"{Indent}default: {{");
                else if (arm.Pattern is PilotIrLiteralPattern lit)
                    sb.AppendLine($"{Indent}case {EmitExpr(lit.Literal)}: {{");
                else
                    sb.AppendLine($"{Indent}default: {{");
                EmitSuiteBody(sb, arm.Body);
                sb.AppendLine($"{Indent}{_options.IndentString}break;");
                sb.AppendLine($"{Indent}}}");
            }
            _indent--;
            sb.Append($"{Indent}}}");
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
                string v = !string.IsNullOrEmpty(c.VariableName) ? $" ({c.VariableName})" : " (_err)";
                sb.AppendLine($" catch{v} {{");
                EmitSuiteBody(sb, c.Body);
                sb.Append($"{Indent}}}");
            }
            if (t.FinallyBody.Count > 0)
            {
                sb.AppendLine(" finally {");
                EmitSuiteBody(sb, t.FinallyBody);
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

        public string EmitExpr(PilotIrExpr expr, PilotIrPrecedence parentPrec = PilotIrPrecedence.Lowest)
        {
            string core = expr switch
            {
                PilotIrLiteralExpr lit => EmitLiteral(lit),
                PilotIrInterpolatedStringExpr fstr => EmitInterpolatedString(fstr),
                PilotIrIdentifierExpr id => id.Name switch
                {
                    "null" or "none" or "None" => "null",
                    "true" or "True" => "true",
                    "false" or "False" => "false",
                    "self" => "this",
                    _ => id.Name
                },
                PilotIrUnaryExpr u => $"{(u.Operator == "not" ? "!" : u.Operator)}{EmitExpr(u.Operand, u.Precedence)}",
                PilotIrBinaryExpr b => EmitBinary(b),
                PilotIrRangeExpr r => EmitRangeArray(r),
                PilotIrCallExpr c => EmitCall(c),
                PilotIrMemberAccessExpr m => EmitMemberAccess(m),
                PilotIrIndexExpr ix => $"{EmitExpr(ix.Target, PilotIrPrecedence.Postfix)}[{EmitExpr(ix.Index)}]",
                PilotIrPostfixTryExpr pt => pt.CatchFallbackExpr != null
                    ? $"({EmitExpr(pt.Operand, PilotIrPrecedence.Coalesce)} ?? {EmitExpr(pt.CatchFallbackExpr, PilotIrPrecedence.Coalesce)})"
                    : EmitExpr(pt.Operand, parentPrec),
                PilotIrCollectionExpr col => EmitCollection(col),
                PilotIrLambdaExpr lam => $"({string.Join(", ", lam.Parameters.Select(p => p.Name))}) => {(lam.BodyExpr != null ? EmitExpr(lam.BodyExpr) : "{}")}",
                PilotIrForExpr fe => EmitForExpr(fe),
                PilotIrIfExpr ie => $"{EmitExpr(ie.Condition, PilotIrPrecedence.Conditional)} ? {EmitExpr(ie.ThenBranch, PilotIrPrecedence.Conditional)} : {EmitExpr(ie.ElseBranch, PilotIrPrecedence.Conditional)}",
                _ => "null"
            };

            if (parentPrec != PilotIrPrecedence.Lowest && expr.Precedence < parentPrec)
                return $"({core})";
            return core;
        }

        private static string EmitLiteral(PilotIrLiteralExpr lit) => lit.Kind switch
        {
            PilotLiteralKind.Null or PilotLiteralKind.None => "null",
            PilotLiteralKind.Boolean => lit.RawValue is "true" or "True" ? "true" : "false",
            PilotLiteralKind.DimensionalUnit => (lit.NormalizedNumericValue ?? 0.0).ToString(System.Globalization.CultureInfo.InvariantCulture),
            _ => lit.RawValue
        };

        private string EmitInterpolatedString(PilotIrInterpolatedStringExpr fstr)
        {
            var sb = new StringBuilder("`");
            foreach (var p in fstr.Parts)
            {
                if (!p.IsInterpolation)
                    sb.Append(p.LiteralText);
                else
                    sb.Append($"${{{(p.Expression != null ? EmitExpr(p.Expression) : p.LiteralText)}}}");
            }
            sb.Append('`');
            return sb.ToString();
        }

        private string EmitBinary(PilotIrBinaryExpr b)
        {
            if (b.Operator == "|>")
                return $"{EmitExpr(b.Right, PilotIrPrecedence.Postfix)}({EmitExpr(b.Left)})";
            if (b.Operator == "//")
                return $"Math.trunc({EmitExpr(b.Left)} / {EmitExpr(b.Right)})";

            string jsOp = b.Operator switch
            {
                "==" => "===",
                "!=" => "!==",
                "and" => "&&",
                "or" => "||",
                _ => b.Operator
            };
            string left = EmitExpr(b.Left, b.Precedence);
            var rightMinPrec = b.Operator is "-" or "/" or "%"
                ? (PilotIrPrecedence)((int)b.Precedence + 1)
                : b.Precedence;
            string right = EmitExpr(b.Right, rightMinPrec);
            return $"{left} {jsOp} {right}";
        }

        private string EmitRangeArray(PilotIrRangeExpr r)
        {
            string start = EmitExpr(r.Start);
            string end = EmitExpr(r.End);
            string len = r.IsInclusive ? $"({end} - {start} + 1)" : $"({end} - {start})";
            return $"Array.from({{ length: {len} }}, (_, _i) => ({start}) + _i)";
        }

        private string EmitCall(PilotIrCallExpr c)
        {
            var renderedArgs = c.Arguments.Select(a => EmitExpr(a.Value)).ToList();
            if (PilotLibraryMapper.TryFormatAstLibraryCall(c.LibraryCallKind, renderedArgs, "js", out string mapped))
                return mapped;

            if (c.Callee is PilotIrMemberAccessExpr mem && mem.MemberName == "append")
                return $"{EmitExpr(mem.Target, PilotIrPrecedence.Postfix)}.push({string.Join(", ", renderedArgs)})";

            string callee = EmitExpr(c.Callee, PilotIrPrecedence.Postfix);
            if (_knownClasses.Contains(callee))
                return $"new {callee}({string.Join(", ", renderedArgs)})";
            return $"{callee}({string.Join(", ", renderedArgs)})";
        }

        private string EmitMemberAccess(PilotIrMemberAccessExpr m)
        {
            string target = EmitExpr(m.Target, PilotIrPrecedence.Postfix);
            if (PilotLibraryMapper.TryFormatAstMemberAccess(target, m.MemberName, "js", out string mapped))
                return mapped;
            string op = m.IsSafeNavigation ? "?." : ".";
            return $"{target}{op}{m.MemberName}";
        }

        private string EmitCollection(PilotIrCollectionExpr col) => col.Kind switch
        {
            PilotCollectionKind.List or PilotCollectionKind.Tuple => $"[{string.Join(", ", col.Elements.Select(e => EmitExpr(e)))}]",
            PilotCollectionKind.Set => $"new Set([{string.Join(", ", col.Elements.Select(e => EmitExpr(e)))}])",
            PilotCollectionKind.Map => $"{{ {string.Join(", ", col.MapEntries.Select(kv => $"[{EmitExpr(kv.Key)}]: {EmitExpr(kv.Value)}"))} }}",
            _ => "[]"
        };

        private string EmitForExpr(PilotIrForExpr fe)
        {
            string target = fe.TargetVars.Count > 0 ? fe.TargetVars[0] : "x";
            string src = EmitExpr(fe.Iterable, PilotIrPrecedence.Postfix);
            string chain = src;
            if (fe.FilterCondition != null)
                chain += $".filter(({target}) => {EmitExpr(fe.FilterCondition)})";
            chain += $".map(({target}) => {EmitExpr(fe.YieldExpr)})";
            return chain;
        }
    }
}
