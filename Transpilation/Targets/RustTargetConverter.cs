using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Pilot.Analysis;
using Pilot.Transpilation.Ingesters;
using Pilot.Transpilation.Ir;

namespace Pilot.Transpilation.Targets;

/// <summary>
/// Milestone 20.3 &amp; 20.4: AST-driven bidirectional code converter between Pilot and Rust (2024 Edition).
/// Emits directly from <see cref="PilotIrModule"/> with precedence-aware parenthesization,
/// <c>#[derive(Debug, Clone, PartialEq)]</c> enum ADTs, <c>Option&lt;T&gt;</c>/<c>Result&lt;T, E&gt;</c>,
/// extension traits for <c>extend</c>, and iterator chains for <c>ForExpr</c>.
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
            sb.AppendLine("// Code transpiled & optimized from Pilot to Rust (2024 Edition)");
            sb.AppendLine("#![allow(unused_variables, dead_code, non_snake_case, non_camel_case_types)]");
            sb.AppendLine();
        }

        var emitter = new RustIrEmitter(module, options);
        sb.Append(emitter.EmitModule());

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options) =>
        ForeignAstIngester.Ingest(foreignSource, SourceLanguage, options);

    private sealed class RustIrEmitter
    {
        private readonly PilotIrModule _module;
        private readonly TranspilationOptions _options;
        private readonly Dictionary<string, string> _variantToAdt = new(StringComparer.Ordinal);
        private int _indent;
        private bool _currentFuncReturnsResult;

        public RustIrEmitter(PilotIrModule module, TranspilationOptions options)
        {
            _module = module;
            _options = options;
            foreach (var adt in module.Statements.OfType<PilotIrAdtDefStmt>())
            {
                foreach (var v in adt.Variants)
                    _variantToAdt[v.Name] = adt.Name;
            }
        }

        private string Indent => string.Concat(Enumerable.Repeat(_options.IndentString, _indent));

        public string EmitModule()
        {
            var sb = new StringBuilder();
            var topDecls = new List<PilotIrStmt>();
            var mainStmts = new List<PilotIrStmt>();
            bool hasExplicitMain = _module.Statements.OfType<PilotIrFuncDefStmt>().Any(f => f.Name == "main");

            foreach (var stmt in _module.Statements)
            {
                if (stmt is PilotIrFuncDefStmt or PilotIrClassDefStmt or PilotIrAdtDefStmt or
                            PilotIrEnumDefStmt or PilotIrTypeAliasStmt or PilotIrExtensionDefStmt)
                {
                    topDecls.Add(stmt);
                }
                else
                {
                    mainStmts.Add(stmt);
                }
            }

            foreach (var decl in topDecls)
            {
                string s = EmitStmt(decl);
                if (!string.IsNullOrWhiteSpace(s))
                {
                    sb.AppendLine(s);
                    sb.AppendLine();
                }
            }

            if (mainStmts.Count > 0 && !hasExplicitMain)
            {
                sb.AppendLine("pub fn main() {");
                _indent = 1;
                foreach (var s in mainStmts)
                {
                    string line = EmitStmt(s);
                    if (!string.IsNullOrWhiteSpace(line))
                        sb.AppendLine(line);
                }
                _indent = 0;
                sb.AppendLine("}");
            }

            return sb.ToString();
        }

        public string EmitStmt(PilotIrStmt stmt) => stmt switch
        {
            PilotIrCommentStmt c => $"{Indent}//{c.CommentText.TrimStart('/')}",
            PilotIrVarDeclStmt v => EmitVarDecl(v),
            PilotIrAssignStmt a => $"{Indent}{EmitExpr(a.Target)} {a.Operator} {EmitExpr(a.Value)};",
            PilotIrFuncDefStmt f => EmitFuncDef(f, isTraitImpl: false),
            PilotIrClassDefStmt c => EmitClassDef(c),
            PilotIrAdtDefStmt adt => EmitAdtDef(adt),
            PilotIrEnumDefStmt en => EmitEnumDef(en),
            PilotIrTypeAliasStmt ta => $"{Indent}pub type {ta.Name} = {MapType(ta.TargetType)};",
            PilotIrExtensionDefStmt ext => EmitExtensionDef(ext),
            PilotIrIfStmt ifs => EmitIfStmt(ifs),
            PilotIrGuardStmt g => EmitGuardStmt(g),
            PilotIrWhileStmt w => EmitWhileStmt(w),
            PilotIrForStmt f => EmitForStmt(f),
            PilotIrMatchStmt m => EmitMatchStmt(m),
            PilotIrDeferStmt d => string.Join(Environment.NewLine, d.Body.Select(EmitStmt)),
            PilotIrReturnStmt r => EmitReturnStmt(r),
            PilotIrRaiseStmt rs => rs.Value != null
                ? (_currentFuncReturnsResult ? $"{Indent}return Err({EmitExpr(rs.Value)}.into());" : $"{Indent}panic!(\"{{:?}}\", {EmitExpr(rs.Value)});")
                : $"{Indent}panic!(\"error\");",
            PilotIrBreakStmt => $"{Indent}break;",
            PilotIrContinueStmt => $"{Indent}continue;",
            PilotIrPassStmt => $"{Indent}// pass",
            PilotIrAssertStmt a => a.Message != null
                ? $"{Indent}assert!({EmitExpr(a.Condition)}, \"{{}}\", {EmitExpr(a.Message)});"
                : $"{Indent}assert!({EmitExpr(a.Condition)});",
            PilotIrComptimeStmt ct => string.Join(Environment.NewLine, ct.Body.Select(EmitStmt)),
            PilotIrExprStmt e => $"{Indent}{EmitExpr(e.Expression)};",
            _ => ""
        };

        private string EmitVarDecl(PilotIrVarDeclStmt v)
        {
            if (v.TupleDestructureNames.Count > 1)
            {
                string lhs = $"({string.Join(", ", v.TupleDestructureNames)})";
                string rhs = v.Initializer != null ? EmitExpr(v.Initializer) : "Default::default()";
                return $"{Indent}let {lhs} = {rhs};";
            }

            string rhsVal = v.ComptimeFoldedLiteral ?? (v.Initializer != null ? EmitExpr(v.Initializer) : "");
            if (v.Kind == PilotVarDeclKind.Const)
            {
                string cType = !string.IsNullOrEmpty(v.DeclaredType) ? MapType(v.DeclaredType) : "i64";
                return $"{Indent}const {v.Name}: {cType} = {rhsVal};";
            }

            string typeAnnot = !string.IsNullOrEmpty(v.DeclaredType) ? $": {MapType(v.DeclaredType)}" : "";
            string mutKw;
            if (v.Kind == PilotVarDeclKind.Var)
            {
                mutKw = "let mut";
            }
            else
            {
                mutKw = _options.Optimize ? (v.IsMutated ? "let mut" : "let") : "let mut";
            }

            return !string.IsNullOrEmpty(rhsVal)
                ? $"{Indent}{mutKw} {v.Name}{typeAnnot} = {rhsVal};"
                : $"{Indent}let mut {v.Name}{typeAnnot};";
        }

        private string EmitReturnStmt(PilotIrReturnStmt r)
        {
            if (r.Value == null)
                return $"{Indent}return;";
            if (_currentFuncReturnsResult &&
                r.Value is not PilotIrCallExpr { LibraryCallKind: PilotLibraryCallKind.FailConstructor })
            {
                string val = EmitExpr(r.Value);
                if (!val.StartsWith("Ok(") && !val.StartsWith("Err("))
                    return $"{Indent}return Ok({val});";
            }
            return $"{Indent}return {EmitExpr(r.Value)};";
        }

        private string EmitFuncDef(PilotIrFuncDefStmt f, bool isTraitImpl)
        {
            bool prevResult = _currentFuncReturnsResult;
            _currentFuncReturnsResult = (f.DeclaredReturnType ?? "").Contains('!');

            var paramList = new List<string>();
            foreach (var p in f.Parameters)
            {
                if (p.Name is "self" or "this")
                {
                    paramList.Add("&self");
                    continue;
                }
                string pType = !string.IsNullOrEmpty(p.TypeAnnotation) ? MapType(p.TypeAnnotation) : "i64";
                string mutMod = p.IsMutable || f.MutatedLocals.Contains(p.Name) ? "mut " : "";
                paramList.Add($"{mutMod}{p.Name}: {pType}");
            }

            string? retType = f.DeclaredReturnType;
            string retStr = !string.IsNullOrEmpty(retType) && retType != "void"
                ? $" -> {MapType(retType)}"
                : "";
            string pubMod = isTraitImpl ? "" : "pub ";
            string genPart = f.GenericParams.Count > 0 ? $"<{string.Join(", ", f.GenericParams)}>" : "";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}{pubMod}fn {f.Name}{genPart}({string.Join(", ", paramList)}){retStr}");
            sb.AppendLine($"{Indent}{{");
            _indent++;
            foreach (var req in f.RequiresClauses)
                sb.AppendLine($"{Indent}assert!({EmitExpr(req)});");
            foreach (var s in f.Body)
            {
                string line = EmitStmt(s);
                if (!string.IsNullOrWhiteSpace(line))
                    sb.AppendLine(line);
            }
            _indent--;
            sb.Append($"{Indent}}}");

            _currentFuncReturnsResult = prevResult;
            return sb.ToString();
        }

        private string EmitClassDef(PilotIrClassDefStmt c)
        {
            var sb = new StringBuilder();
            string genPart = c.GenericParams.Count > 0 ? $"<{string.Join(", ", c.GenericParams)}>" : "";
            sb.AppendLine($"{Indent}#[derive(Debug, Clone, PartialEq)]");
            if (c.Fields.Count == 0)
            {
                sb.AppendLine($"{Indent}pub struct {c.Name}{genPart} {{}}");
            }
            else
            {
                sb.AppendLine($"{Indent}pub struct {c.Name}{genPart} {{");
                _indent++;
                foreach (var f in c.Fields)
                {
                    sb.AppendLine($"{Indent}pub {f.Name}: {MapType(f.TypeAnnotation)},");
                }
                _indent--;
                sb.AppendLine($"{Indent}}}");
            }

            if (c.Methods.Count > 0)
            {
                sb.AppendLine($"{Indent}impl{genPart} {c.Name}{genPart}");
                sb.AppendLine($"{Indent}{{");
                _indent++;
                foreach (var m in c.Methods)
                {
                    sb.AppendLine(EmitFuncDef(m, isTraitImpl: false));
                }
                _indent--;
                sb.Append($"{Indent}}}");
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitAdtDef(PilotIrAdtDefStmt adt)
        {
            var sb = new StringBuilder();
            string genPart = adt.GenericParams.Count > 0 ? $"<{string.Join(", ", adt.GenericParams)}>" : "";
            sb.AppendLine($"{Indent}#[derive(Debug, Clone, PartialEq)]");
            sb.AppendLine($"{Indent}pub enum {adt.Name}{genPart} {{");
            _indent++;
            foreach (var v in adt.Variants)
            {
                if (v.Fields.Count == 0)
                {
                    sb.AppendLine($"{Indent}{v.Name},");
                }
                else
                {
                    string fields = string.Join(", ", v.Fields.Select(f => MapType(f.Type)));
                    sb.AppendLine($"{Indent}{v.Name}({fields}),");
                }
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitEnumDef(PilotIrEnumDefStmt en)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
            sb.AppendLine($"{Indent}pub enum {en.Name} {{");
            _indent++;
            foreach (var (mName, mVal) in en.Members)
            {
                string valPart = mVal != null ? $" = {EmitExpr(mVal)}" : "";
                sb.AppendLine($"{Indent}{mName}{valPart},");
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitExtensionDef(PilotIrExtensionDefStmt ext)
        {
            var sb = new StringBuilder();
            string traitName = $"{ext.ExtensionName ?? ext.TargetType}Ext";
            string targetRustType = MapType(ext.TargetType);

            sb.AppendLine($"{Indent}pub trait {traitName} {{");
            _indent++;
            foreach (var m in ext.Methods)
            {
                var pList = new List<string> { "&self" };
                foreach (var p in m.Parameters)
                {
                    if (p.Name is not ("self" or "this"))
                        pList.Add($"{p.Name}: {MapType(p.TypeAnnotation ?? "i64")}");
                }
                string ret = !string.IsNullOrEmpty(m.DeclaredReturnType) && m.DeclaredReturnType != "void"
                    ? $" -> {MapType(m.DeclaredReturnType)}"
                    : "";
                sb.AppendLine($"{Indent}fn {m.Name}({string.Join(", ", pList)}){ret};");
            }
            _indent--;
            sb.AppendLine($"{Indent}}}");
            sb.AppendLine();
            sb.AppendLine($"{Indent}impl {traitName} for {targetRustType} {{");
            _indent++;
            foreach (var m in ext.Methods)
            {
                if (m.Parameters.Count == 0 || m.Parameters[0].Name is not ("self" or "this"))
                    m.Parameters.Insert(0, new PilotIrParameter("self"));
                sb.AppendLine(EmitFuncDef(m, isTraitImpl: true));
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitIfStmt(PilotIrIfStmt ifs)
        {
            var sb = new StringBuilder();
            if (!string.IsNullOrEmpty(ifs.OptionalBindingVar))
                sb.AppendLine($"{Indent}if let Some({ifs.OptionalBindingVar}) = {EmitExpr(ifs.Condition)}");
            else
                sb.AppendLine($"{Indent}if {EmitExpr(ifs.Condition)}");
            EmitBlock(sb, ifs.ThenBody);

            foreach (var (cond, body) in ifs.ElifBranches)
            {
                sb.AppendLine($"{Indent}else if {EmitExpr(cond)}");
                EmitBlock(sb, body);
            }
            if (ifs.ElseBody.Count > 0)
            {
                sb.AppendLine($"{Indent}else");
                EmitBlock(sb, ifs.ElseBody);
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitGuardStmt(PilotIrGuardStmt g)
        {
            var sb = new StringBuilder();
            if (g.IsSomeBinding && !string.IsNullOrEmpty(g.BoundVariable))
            {
                sb.AppendLine($"{Indent}let Some({g.BoundVariable}) = {EmitExpr(g.ConditionOrSource)} else {{");
                _indent++;
                foreach (var s in g.ElseBody)
                    sb.AppendLine(EmitStmt(s));
                _indent--;
                sb.AppendLine($"{Indent}}};");
                if (g.PatternGuard != null)
                {
                    sb.AppendLine($"{Indent}if !({EmitExpr(g.PatternGuard)})");
                    EmitBlock(sb, g.ElseBody);
                }
                return sb.ToString().TrimEnd();
            }

            sb.AppendLine($"{Indent}if !({EmitExpr(g.ConditionOrSource)})");
            EmitBlock(sb, g.ElseBody);
            return sb.ToString().TrimEnd();
        }

        private string EmitWhileStmt(PilotIrWhileStmt w)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}while {EmitExpr(w.Condition)}");
            EmitBlock(sb, w.Body);
            return sb.ToString().TrimEnd();
        }

        private string EmitForStmt(PilotIrForStmt f)
        {
            var sb = new StringBuilder();
            string target = f.TargetVars.Count > 0 ? string.Join(", ", f.TargetVars) : "_item";
            sb.AppendLine($"{Indent}for {target} in {EmitExpr(f.Iterable)}");
            sb.AppendLine($"{Indent}{{");
            _indent++;
            if (f.WhereFilter != null)
                sb.AppendLine($"{Indent}if !({EmitExpr(f.WhereFilter)}) {{ continue; }}");
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
            sb.AppendLine($"{Indent}match {EmitExpr(m.Scrutinee)} {{");
            _indent++;
            foreach (var arm in m.Arms)
            {
                string pat = EmitPattern(arm.Pattern);
                string guard = arm.Guard != null ? $" if {EmitExpr(arm.Guard)}" : "";
                sb.AppendLine($"{Indent}{pat}{guard} => {{");
                _indent++;
                foreach (var s in arm.Body)
                {
                    string line = EmitStmt(s);
                    if (!string.IsNullOrWhiteSpace(line))
                        sb.AppendLine(line);
                }
                _indent--;
                sb.AppendLine($"{Indent}}},");
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitPattern(PilotIrPattern pat) => pat switch
        {
            PilotIrWildcardPattern => "_",
            PilotIrLiteralPattern lit => EmitExpr(lit.Literal),
            PilotIrBindingPattern b => b.IsMutable ? $"mut {b.Name}" : b.Name,
            PilotIrOptionPattern opt => opt.IsSome
                ? $"Some({(opt.InnerPattern != null ? EmitPattern(opt.InnerPattern) : "_")})"
                : "None",
            PilotIrVariantPattern vp => EmitVariantPattern(vp),
            PilotIrRangePattern rp => $"{EmitExpr(rp.Start)}{(rp.IsInclusive ? "..=" : "..")}{EmitExpr(rp.End)}",
            PilotIrOrPattern orp => string.Join(" | ", orp.Alternatives.Select(EmitPattern)),
            PilotIrTuplePattern tp => $"({string.Join(", ", tp.Elements.Select(EmitPattern))})",
            _ => "_"
        };

        private string EmitVariantPattern(PilotIrVariantPattern vp)
        {
            string prefix = !string.IsNullOrEmpty(vp.TypeName)
                ? $"{vp.TypeName}::{vp.VariantName}"
                : (_variantToAdt.TryGetValue(vp.VariantName, out var adt) ? $"{adt}::{vp.VariantName}" : vp.VariantName);
            if (vp.Arguments.Count == 0)
                return prefix;
            return $"{prefix}({string.Join(", ", vp.Arguments.Select(a => EmitPattern(a.SubPattern)))})";
        }

        private void EmitBlock(StringBuilder sb, List<PilotIrStmt> stmts)
        {
            sb.AppendLine($"{Indent}{{");
            _indent++;
            foreach (var s in stmts)
            {
                string line = EmitStmt(s);
                if (!string.IsNullOrWhiteSpace(line))
                    sb.AppendLine(line);
            }
            _indent--;
            sb.AppendLine($"{Indent}}}");
        }

        public string EmitExpr(PilotIrExpr expr, PilotIrPrecedence parentPrec = PilotIrPrecedence.Lowest)
        {
            string core = expr switch
            {
                PilotIrLiteralExpr lit => EmitLiteral(lit),
                PilotIrInterpolatedStringExpr fstr => EmitInterpolatedString(fstr, isPrintln: false),
                PilotIrIdentifierExpr id => id.Name switch
                {
                    "null" or "none" or "None" or "undefined" => "None",
                    "this" => "self",
                    _ => _variantToAdt.TryGetValue(id.Name, out var adt) ? $"{adt}::{id.Name}" : id.Name
                },
                PilotIrUnaryExpr u => EmitUnary(u),
                PilotIrBinaryExpr b => EmitBinary(b),
                PilotIrRangeExpr r => $"{EmitExpr(r.Start, PilotIrPrecedence.Range)}{(r.IsInclusive ? "..=" : "..")}{EmitExpr(r.End, PilotIrPrecedence.Range)}",
                PilotIrCallExpr c => EmitCall(c),
                PilotIrMemberAccessExpr m => EmitMemberAccess(m),
                PilotIrIndexExpr ix => $"{EmitExpr(ix.Target, PilotIrPrecedence.Postfix)}[{EmitExpr(ix.Index)} as usize]",
                PilotIrPostfixTryExpr pt => EmitPostfixTry(pt),
                PilotIrCollectionExpr col => EmitCollection(col),
                PilotIrLambdaExpr lam => $"|{string.Join(", ", lam.Parameters.Select(p => p.Name))}| {(lam.BodyExpr != null ? EmitExpr(lam.BodyExpr) : "{}")}",
                PilotIrForExpr fe => EmitForExpr(fe),
                PilotIrIfExpr ie => $"if {EmitExpr(ie.Condition)} {{ {EmitExpr(ie.ThenBranch)} }} else {{ {EmitExpr(ie.ElseBranch)} }}",
                _ => "()"
            };

            if (parentPrec != PilotIrPrecedence.Lowest && expr.Precedence < parentPrec)
                return $"({core})";
            return core;
        }

        private static string EmitLiteral(PilotIrLiteralExpr lit) => lit.Kind switch
        {
            PilotLiteralKind.Null or PilotLiteralKind.None => "None",
            PilotLiteralKind.Boolean => lit.RawValue is "true" or "True" ? "true" : "false",
            PilotLiteralKind.DimensionalUnit => (lit.NormalizedNumericValue ?? 0.0).ToString(System.Globalization.CultureInfo.InvariantCulture),
            _ => lit.RawValue
        };

        private string EmitInterpolatedString(PilotIrInterpolatedStringExpr fstr, bool isPrintln)
        {
            var fmt = new StringBuilder("\"");
            var args = new List<string>();
            foreach (var part in fstr.Parts)
            {
                if (!part.IsInterpolation)
                {
                    fmt.Append(part.LiteralText);
                }
                else
                {
                    fmt.Append("{}");
                    args.Add(part.Expression != null ? EmitExpr(part.Expression) : part.LiteralText);
                }
            }
            fmt.Append('"');
            string macro = isPrintln ? "println!" : "format!";
            return args.Count > 0
                ? $"{macro}({fmt}, {string.Join(", ", args)})"
                : $"{macro}({fmt})";
        }

        private string EmitUnary(PilotIrUnaryExpr u)
        {
            string op = u.Operator is "not" ? "!" : u.Operator;
            return $"{op}{EmitExpr(u.Operand, u.Precedence)}";
        }

        private string EmitBinary(PilotIrBinaryExpr b)
        {
            if (b.Operator == "**")
                return $"({EmitExpr(b.Left)} as f64).powf({EmitExpr(b.Right)} as f64)";
            if (b.Operator == "|>")
                return $"{EmitExpr(b.Right, PilotIrPrecedence.Postfix)}({EmitExpr(b.Left)})";
            if (b.Operator == "??")
                return $"{EmitExpr(b.Left, PilotIrPrecedence.Postfix)}.unwrap_or({EmitExpr(b.Right)})";

            string rsOp = b.Operator switch
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
            return $"{left} {rsOp} {right}";
        }

        private string EmitCall(PilotIrCallExpr c)
        {
            if (c.LibraryCallKind == PilotLibraryCallKind.Println && c.Arguments.Count == 1)
            {
                var arg0 = c.Arguments[0].Value;
                if (arg0 is PilotIrInterpolatedStringExpr fstr)
                    return EmitInterpolatedString(fstr, isPrintln: true);
                if (arg0 is PilotIrLiteralExpr { Kind: PilotLiteralKind.String } sLit)
                    return $"println!({sLit.RawValue})";
                return $"println!(\"{{:?}}\", {EmitExpr(arg0)})";
            }

            var renderedArgs = c.Arguments.Select(a => EmitExpr(a.Value)).ToList();
            if (PilotLibraryMapper.TryFormatAstLibraryCall(c.LibraryCallKind, renderedArgs, "rust", out string mapped))
                return mapped;

            if (c.Callee is PilotIrMemberAccessExpr mem && mem.MemberName is "append")
            {
                return $"{EmitExpr(mem.Target, PilotIrPrecedence.Postfix)}.push({string.Join(", ", renderedArgs)})";
            }

            string callee = EmitExpr(c.Callee, PilotIrPrecedence.Postfix);
            if (_variantToAdt.TryGetValue(callee, out var adt))
            {
                return $"{adt}::{callee}({string.Join(", ", renderedArgs)})";
            }
            return $"{callee}({string.Join(", ", renderedArgs)})";
        }

        private string EmitMemberAccess(PilotIrMemberAccessExpr m)
        {
            string target = EmitExpr(m.Target, PilotIrPrecedence.Postfix);
            if (PilotLibraryMapper.TryFormatAstMemberAccess(target, m.MemberName, "rust", out string mapped))
                return mapped;
            return $"{target}.{m.MemberName}";
        }

        private string EmitPostfixTry(PilotIrPostfixTryExpr pt)
        {
            string op = EmitExpr(pt.Operand, PilotIrPrecedence.Postfix);
            if (pt.IsQuestionPropagate)
                return $"{op}?";
            if (pt.CatchFallbackExpr != null)
            {
                string errVar = pt.CatchVarName ?? "_err";
                return $"{op}.unwrap_or_else(|{errVar}| {EmitExpr(pt.CatchFallbackExpr)})";
            }
            return $"{op}?";
        }

        private string EmitCollection(PilotIrCollectionExpr col) => col.Kind switch
        {
            PilotCollectionKind.List => $"vec![{string.Join(", ", col.Elements.Select(e => EmitExpr(e)))}]",
            PilotCollectionKind.Tuple => $"({string.Join(", ", col.Elements.Select(e => EmitExpr(e)))})",
            _ => $"vec![{string.Join(", ", col.Elements.Select(e => EmitExpr(e)))}]"
        };

        private string EmitForExpr(PilotIrForExpr fe)
        {
            string target = fe.TargetVars.Count > 0 ? fe.TargetVars[0] : "x";
            string src = EmitExpr(fe.Iterable, PilotIrPrecedence.Postfix);
            string iterChain = $"({src}).into_iter()";
            if (fe.FilterCondition != null)
                iterChain += $".filter(|&{target}| {EmitExpr(fe.FilterCondition)})";
            iterChain += $".map(|{target}| {EmitExpr(fe.YieldExpr)}).collect::<Vec<_>>()";
            return iterChain;
        }

        private string MapType(string pilotType)
        {
            string t = pilotType.Trim();
            if (_options.TypeMappings.TryGetValue(t, out var mapped))
                return mapped;

            if (t.EndsWith("?"))
                return $"Option<{MapType(t[..^1])}>";
            if (t.Contains('!'))
            {
                var parts = t.Split('!', 2);
                string okT = MapType(parts[0].Trim());
                string errT = MapType(parts[1].Trim());
                return $"Result<{okT}, {errT}>";
            }
            if (t.StartsWith("list[") && t.EndsWith("]"))
                return $"Vec<{MapType(t[5..^1])}>";

            return t switch
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
                "str" or "string" or "Error" => "String",
                "bool" => "bool",
                "void" => "()",
                "list" => "Vec<i64>",
                _ => t
            };
        }
    }
}
