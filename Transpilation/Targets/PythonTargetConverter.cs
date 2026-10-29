using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Pilot.Analysis;
using Pilot.Transpilation.Ingesters;
using Pilot.Transpilation.Ir;

namespace Pilot.Transpilation.Targets;

/// <summary>
/// Milestone 20.3 &amp; 20.4: AST-driven bidirectional code converter between Pilot and Python 3.10+.
/// Emits directly from <see cref="PilotIrModule"/> with precedence-aware parenthesization,
/// <c>@dataclass</c> ADTs, <c>Optional[T]</c>, <c>match</c>/<c>case</c>, list comprehensions,
/// and AST-scoped <see cref="PilotLibraryMapper"/> calls without raw <c>ctx.GetText()</c>.
/// </summary>
public sealed class PythonTargetConverter : ITargetLanguageConverter
{
    public string Language { get; }
    public string TargetLanguage => Language;
    public string SourceLanguage => Language;

    public PythonTargetConverter(string alias = "python")
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
            sb.AppendLine("# Code transpiled & optimized from Pilot to Python 3");
            sb.AppendLine("# Target: Python 3.10+ (typed)");
            sb.AppendLine("from __future__ import annotations");
            sb.AppendLine("from dataclasses import dataclass");
            sb.AppendLine("from typing import Any, List, Dict, Set, Tuple, Optional, Callable, Final, Union");
            if (module.Statements.OfType<PilotIrEnumDefStmt>().Any())
                sb.AppendLine("from enum import Enum");
            if (module.UsesJsonPrimitives)
                sb.AppendLine("import json");
            sb.AppendLine();
        }

        var emitter = new PythonIrEmitter(module, options);
        sb.Append(emitter.EmitModule());

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options) =>
        ForeignAstIngester.Ingest(foreignSource, SourceLanguage, options);

    private sealed class PythonIrEmitter
    {
        private readonly PilotIrModule _module;
        private readonly TranspilationOptions _options;
        private readonly HashSet<string> _extensionTargets = new(StringComparer.Ordinal);
        private int _indent;

        public PythonIrEmitter(PilotIrModule module, TranspilationOptions options)
        {
            _module = module;
            _options = options;
            foreach (var ext in module.Statements.OfType<PilotIrExtensionDefStmt>())
            {
                foreach (var m in ext.Methods)
                    _extensionTargets.Add(m.Name);
            }
        }

        private string Indent => string.Concat(Enumerable.Repeat(_options.IndentString, _indent));

        public string EmitModule()
        {
            var sb = new StringBuilder();

            foreach (var imp in _module.Imports)
            {
                if (imp.IsMappedNativeEquivalent)
                    continue;
                string line = EmitImport(imp);
                if (!string.IsNullOrWhiteSpace(line))
                    sb.AppendLine(line);
            }

            for (int i = 0; i < _module.Statements.Count; i++)
            {
                var stmt = _module.Statements[i];
                EmitLeadingComments(sb, stmt);
                string emitted = EmitStmt(stmt);
                if (!string.IsNullOrWhiteSpace(emitted))
                {
                    sb.AppendLine(emitted);
                    if (stmt is PilotIrFuncDefStmt or PilotIrClassDefStmt or PilotIrAdtDefStmt or PilotIrExtensionDefStmt &&
                        i + 1 < _module.Statements.Count)
                    {
                        sb.AppendLine();
                    }
                }
            }

            return sb.ToString();
        }

        private void EmitLeadingComments(StringBuilder sb, PilotIrNode node)
        {
            foreach (var t in node.LeadingTrivia)
            {
                if (t.Kind is PilotTriviaKind.LineComment or PilotTriviaKind.DocComment)
                {
                    string clean = t.Text.TrimStart('/');
                    sb.AppendLine($"{Indent}#{clean}");
                }
            }
        }

        private string EmitImport(PilotIrImportStmt imp)
        {
            if (imp.ImportedItems.Count > 0)
            {
                var items = imp.ImportedItems.Select(i => i.Alias != null ? $"{i.Name} as {i.Alias}" : i.Name);
                return $"{Indent}from {imp.ModulePath} import {string.Join(", ", items)}";
            }
            return imp.Alias != null
                ? $"{Indent}import {imp.ModulePath} as {imp.Alias}"
                : $"{Indent}import {imp.ModulePath}";
        }

        public string EmitStmt(PilotIrStmt stmt) => stmt switch
        {
            PilotIrCommentStmt c => $"{Indent}#{c.CommentText.TrimStart('/')}",
            PilotIrImportStmt imp => imp.IsMappedNativeEquivalent ? "" : EmitImport(imp),
            PilotIrVarDeclStmt v => EmitVarDecl(v),
            PilotIrAssignStmt a => $"{Indent}{EmitExpr(a.Target)} {a.Operator} {EmitExpr(a.Value)}",
            PilotIrFuncDefStmt f => EmitFuncDef(f, isMethod: false),
            PilotIrClassDefStmt c => EmitClassDef(c),
            PilotIrAdtDefStmt adt => EmitAdtDef(adt),
            PilotIrEnumDefStmt en => EmitEnumDef(en),
            PilotIrTypeAliasStmt ta => $"{Indent}{ta.Name} = {MapType(ta.TargetType)}",
            PilotIrExtensionDefStmt ext => EmitExtensionDef(ext),
            PilotIrIfStmt ifs => EmitIfStmt(ifs),
            PilotIrGuardStmt g => EmitGuardStmt(g),
            PilotIrWhileStmt w => EmitWhileStmt(w),
            PilotIrForStmt f => EmitForStmt(f),
            PilotIrMatchStmt m => EmitMatchStmt(m),
            PilotIrTryStmt t => EmitTryStmt(t),
            PilotIrDeferStmt d => EmitDeferStmt(d),
            PilotIrReturnStmt r => r.Value != null ? $"{Indent}return {EmitExpr(r.Value)}" : $"{Indent}return",
            PilotIrYieldStmt y => y.Value != null ? $"{Indent}yield {EmitExpr(y.Value)}" : $"{Indent}yield",
            PilotIrRaiseStmt rs => rs.Value != null ? $"{Indent}raise RuntimeError({EmitExpr(rs.Value)})" : $"{Indent}raise RuntimeError()",
            PilotIrBreakStmt => $"{Indent}break",
            PilotIrContinueStmt => $"{Indent}continue",
            PilotIrPassStmt => $"{Indent}pass",
            PilotIrAssertStmt a => a.Message != null
                ? $"{Indent}assert {EmitExpr(a.Condition)}, {EmitExpr(a.Message)}"
                : $"{Indent}assert {EmitExpr(a.Condition)}",
            PilotIrComptimeStmt ct => string.Join(Environment.NewLine, ct.Body.Select(EmitStmt)),
            PilotIrExprStmt e => $"{Indent}{EmitExpr(e.Expression)}",
            _ => $"{Indent}pass"
        };

        private string EmitVarDecl(PilotIrVarDeclStmt v)
        {
            if (v.TupleDestructureNames.Count > 1)
            {
                string lhs = string.Join(", ", v.TupleDestructureNames);
                string rhs = v.Initializer != null ? EmitExpr(v.Initializer) : "None";
                return $"{Indent}{lhs} = {rhs}";
            }

            string rhsVal = v.ComptimeFoldedLiteral ?? (v.Initializer != null ? EmitExpr(v.Initializer) : "None");
            if (v.Kind == PilotVarDeclKind.Const)
            {
                string typeSuffix = !string.IsNullOrEmpty(v.DeclaredType) ? $"[{MapType(v.DeclaredType)}]" : "";
                return $"{Indent}{v.Name}: Final{typeSuffix} = {rhsVal}";
            }

            if (_options.GenerateTypeAnnotations && !string.IsNullOrEmpty(v.DeclaredType))
            {
                return $"{Indent}{v.Name}: {MapType(v.DeclaredType)} = {rhsVal}";
            }

            return $"{Indent}{v.Name} = {rhsVal}";
        }

        private string EmitFuncDef(PilotIrFuncDefStmt f, bool isMethod)
        {
            var sb = new StringBuilder();
            var paramList = new List<string>();

            foreach (var p in f.Parameters)
            {
                if (p.Name is "self" or "this")
                {
                    paramList.Add("self");
                    continue;
                }
                string pStr = (p.IsVariadic ? "*" : "") + p.Name;
                if (_options.GenerateTypeAnnotations && !string.IsNullOrEmpty(p.TypeAnnotation))
                {
                    pStr += $": {MapType(p.TypeAnnotation)}";
                }
                if (p.DefaultValue != null)
                {
                    pStr += $" = {EmitExpr(p.DefaultValue)}";
                }
                paramList.Add(pStr);
            }

            string asyncPrefix = f.IsAsync ? "async " : "";
            string fnName = isMethod && f.Name == "__init__" ? "__init__" : f.Name;
            string? retType = f.DeclaredReturnType;
            string retAnnot = (_options.GenerateTypeAnnotations && !string.IsNullOrEmpty(retType))
                ? $" -> {MapType(retType)}"
                : "";

            sb.AppendLine($"{Indent}{asyncPrefix}def {fnName}({string.Join(", ", paramList)}){retAnnot}:");
            _indent++;
            foreach (var req in f.RequiresClauses)
            {
                sb.AppendLine($"{Indent}assert {EmitExpr(req)}");
            }
            if (f.Body.Count == 0 && f.RequiresClauses.Count == 0)
            {
                sb.AppendLine($"{Indent}pass");
            }
            else
            {
                foreach (var stmt in f.Body)
                {
                    EmitLeadingComments(sb, stmt);
                    string line = EmitStmt(stmt);
                    if (!string.IsNullOrWhiteSpace(line))
                        sb.AppendLine(line);
                }
            }
            _indent--;
            return sb.ToString().TrimEnd();
        }

        private string EmitClassDef(PilotIrClassDefStmt c)
        {
            var sb = new StringBuilder();
            if (c.Kind is PilotTypeDeclKind.Record or PilotTypeDeclKind.Struct && c.Fields.Count > 0 && c.Methods.Count == 0)
            {
                sb.AppendLine($"{Indent}@dataclass");
            }
            string inheritance = c.BaseTypes.Count > 0
                ? $"({string.Join(", ", c.BaseTypes.Select(MapType))})"
                : "";
            sb.AppendLine($"{Indent}class {c.Name}{inheritance}:");
            _indent++;
            if (c.Fields.Count == 0 && c.Methods.Count == 0)
            {
                sb.AppendLine($"{Indent}pass");
            }
            else
            {
                foreach (var f in c.Fields)
                {
                    string defPart = f.DefaultValue != null ? $" = {EmitExpr(f.DefaultValue)}" : "";
                    sb.AppendLine($"{Indent}{f.Name}: {MapType(f.TypeAnnotation)}{defPart}");
                }
                if (c.Fields.Count > 0 && c.Kind == PilotTypeDeclKind.Class &&
                    !c.Methods.Any(m => m.Name is "__init__" or "init"))
                {
                    var ctorParams = c.Fields.Select(f =>
                        f.DefaultValue != null
                            ? $"{f.Name}: {MapType(f.TypeAnnotation)} = {EmitExpr(f.DefaultValue)}"
                            : $"{f.Name}: {MapType(f.TypeAnnotation)} = None");
                    sb.AppendLine($"{Indent}def __init__(self, {string.Join(", ", ctorParams)}):");
                    _indent++;
                    foreach (var f in c.Fields)
                        sb.AppendLine($"{Indent}self.{f.Name} = {f.Name}");
                    _indent--;
                }
                foreach (var m in c.Methods)
                {
                    if (m.Parameters.Count == 0 || m.Parameters[0].Name is not ("self" or "this"))
                    {
                        if (!m.IsStatic)
                            m.Parameters.Insert(0, new PilotIrParameter("self"));
                        else
                            sb.AppendLine($"{Indent}@staticmethod");
                    }
                    sb.AppendLine(EmitFuncDef(m, isMethod: true));
                }
            }
            _indent--;
            return sb.ToString().TrimEnd();
        }

        private string EmitAdtDef(PilotIrAdtDefStmt adt)
        {
            var sb = new StringBuilder();
            foreach (var v in adt.Variants)
            {
                sb.AppendLine($"{Indent}@dataclass");
                sb.AppendLine($"{Indent}class {v.Name}:");
                _indent++;
                if (v.Fields.Count == 0)
                {
                    sb.AppendLine($"{Indent}pass");
                }
                else
                {
                    foreach (var f in v.Fields)
                        sb.AppendLine($"{Indent}{f.Name}: {MapType(f.Type)}");
                }
                _indent--;
                sb.AppendLine();
            }
            if (adt.Variants.Count > 0)
            {
                string unionArms = string.Join(", ", adt.Variants.Select(v => v.Name));
                sb.Append($"{Indent}{adt.Name} = Union[{unionArms}]");
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitEnumDef(PilotIrEnumDefStmt en)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}class {en.Name}(Enum):");
            _indent++;
            if (en.Members.Count == 0)
            {
                sb.AppendLine($"{Indent}pass");
            }
            else
            {
                for (int i = 0; i < en.Members.Count; i++)
                {
                    var (mName, mVal) = en.Members[i];
                    string valStr = mVal != null ? EmitExpr(mVal) : i.ToString();
                    sb.AppendLine($"{Indent}{mName} = {valStr}");
                }
            }
            _indent--;
            return sb.ToString().TrimEnd();
        }

        private string EmitExtensionDef(PilotIrExtensionDefStmt ext)
        {
            var sb = new StringBuilder();
            foreach (var m in ext.Methods)
            {
                var clone = new PilotIrFuncDefStmt
                {
                    Name = $"__ext_{ext.TargetType}_{m.Name}",
                    DeclaredReturnType = m.DeclaredReturnType,
                    IsAsync = m.IsAsync
                };
                foreach (var p in m.Parameters)
                {
                    if (p.Name is "self" or "this")
                        clone.Parameters.Add(new PilotIrParameter("self_val", ext.TargetType));
                    else
                        clone.Parameters.Add(p);
                }
                if (clone.Parameters.Count == 0 || clone.Parameters[0].Name != "self_val")
                    clone.Parameters.Insert(0, new PilotIrParameter("self_val", ext.TargetType));
                clone.Body.AddRange(m.Body);
                sb.AppendLine(System.Text.RegularExpressions.Regex.Replace(
                    EmitFuncDef(clone, isMethod: false),
                    @"\bself\b",
                    "self_val"));
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitIfStmt(PilotIrIfStmt ifs)
        {
            var sb = new StringBuilder();
            if (!string.IsNullOrEmpty(ifs.OptionalBindingVar))
            {
                sb.AppendLine($"{Indent}{ifs.OptionalBindingVar} = {EmitExpr(ifs.Condition)}");
                sb.AppendLine($"{Indent}if {ifs.OptionalBindingVar} is not None:");
            }
            else
            {
                sb.AppendLine($"{Indent}if {EmitExpr(ifs.Condition)}:");
            }
            EmitSuite(sb, ifs.ThenBody);

            foreach (var (cond, body) in ifs.ElifBranches)
            {
                sb.AppendLine($"{Indent}elif {EmitExpr(cond)}:");
                EmitSuite(sb, body);
            }

            if (ifs.ElseBody.Count > 0)
            {
                sb.AppendLine($"{Indent}else:");
                EmitSuite(sb, ifs.ElseBody);
            }

            return sb.ToString().TrimEnd();
        }

        private string EmitGuardStmt(PilotIrGuardStmt g)
        {
            var sb = new StringBuilder();
            if (g.IsSomeBinding && !string.IsNullOrEmpty(g.BoundVariable))
            {
                sb.AppendLine($"{Indent}{g.BoundVariable} = {EmitExpr(g.ConditionOrSource)}");
                string cond = $"{g.BoundVariable} is None";
                if (g.PatternGuard != null)
                    cond += $" or not ({EmitExpr(g.PatternGuard)})";
                sb.AppendLine($"{Indent}if {cond}:");
            }
            else
            {
                sb.AppendLine($"{Indent}if not ({EmitExpr(g.ConditionOrSource)}):");
            }
            EmitSuite(sb, g.ElseBody);
            return sb.ToString().TrimEnd();
        }

        private string EmitWhileStmt(PilotIrWhileStmt w)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}while {EmitExpr(w.Condition)}:");
            EmitSuite(sb, w.Body);
            return sb.ToString().TrimEnd();
        }

        private string EmitForStmt(PilotIrForStmt f)
        {
            var sb = new StringBuilder();
            string target = f.TargetVars.Count > 0 ? string.Join(", ", f.TargetVars) : "_";
            string iterable = EmitForIterable(f.Iterable);

            sb.AppendLine($"{Indent}for {target} in {iterable}:");
            if (f.WhereFilter != null)
            {
                _indent++;
                sb.AppendLine($"{Indent}if not ({EmitExpr(f.WhereFilter)}):");
                sb.AppendLine($"{Indent}{_options.IndentString}continue");
                _indent--;
            }
            EmitSuite(sb, f.Body);
            foreach (var post in f.CompletionThenBody)
                sb.AppendLine(EmitStmt(post));
            return sb.ToString().TrimEnd();
        }

        private string EmitForIterable(PilotIrExpr iter)
        {
            if (iter is PilotIrRangeExpr r)
            {
                string start = EmitExpr(r.Start);
                string end = EmitExpr(r.End);
                string stopExpr = r.IsInclusive
                    ? (int.TryParse(end, out int endInt) ? (endInt + 1).ToString() : $"{end} + 1")
                    : end;
                if (r.Step != null)
                    return $"range({start}, {stopExpr}, {EmitExpr(r.Step)})";
                return start == "0" ? $"range({stopExpr})" : $"range({start}, {stopExpr})";
            }
            return EmitExpr(iter);
        }

        private string EmitMatchStmt(PilotIrMatchStmt m)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}match {EmitExpr(m.Scrutinee)}:");
            _indent++;
            if (m.Arms.Count == 0)
            {
                sb.AppendLine($"{Indent}case _:");
                sb.AppendLine($"{Indent}{_options.IndentString}pass");
            }
            else
            {
                foreach (var arm in m.Arms)
                {
                    var (patStr, rangeGuard) = EmitPattern(arm.Pattern);
                    var guards = new List<string>();
                    if (!string.IsNullOrEmpty(rangeGuard))
                        guards.Add(rangeGuard);
                    if (arm.Guard != null)
                        guards.Add(EmitExpr(arm.Guard));
                    string guardSuffix = guards.Count > 0 ? $" if {string.Join(" and ", guards)}" : "";

                    sb.AppendLine($"{Indent}case {patStr}{guardSuffix}:");
                    if (arm.ExpressionBody != null && arm.Body.Count == 0)
                    {
                        _indent++;
                        sb.AppendLine($"{Indent}{EmitExpr(arm.ExpressionBody)}");
                        _indent--;
                    }
                    else
                    {
                        EmitSuite(sb, arm.Body);
                    }
                }
            }
            _indent--;
            return sb.ToString().TrimEnd();
        }

        private (string Pattern, string? Guard) EmitPattern(PilotIrPattern pat) => pat switch
        {
            PilotIrWildcardPattern => ("_", null),
            PilotIrLiteralPattern lit => (EmitExpr(lit.Literal), null),
            PilotIrBindingPattern b => (!string.IsNullOrEmpty(b.TypeName) ? $"{MapType(b.TypeName)}() as {b.Name}" : b.Name, null),
            PilotIrOptionPattern opt => opt.IsSome
                ? (opt.InnerPattern != null ? EmitPattern(opt.InnerPattern).Pattern : "_val",
                   $"{(opt.InnerPattern != null ? EmitPattern(opt.InnerPattern).Pattern : "_val")} is not None")
                : ("None", null),
            PilotIrVariantPattern vp => (
                $"{vp.VariantName}({string.Join(", ", vp.Arguments.Select(a => a.FieldName != null ? $"{a.FieldName}={EmitPattern(a.SubPattern).Pattern}" : EmitPattern(a.SubPattern).Pattern))})",
                null),
            PilotIrRangePattern rp => ("_r", rp.IsInclusive
                ? $"({EmitExpr(rp.Start)} <= _r <= {EmitExpr(rp.End)})"
                : $"({EmitExpr(rp.Start)} <= _r < {EmitExpr(rp.End)})"),
            PilotIrRelationalPattern rel => ("_v", $"_v {rel.Operator} {EmitExpr(rel.Value)}"),
            PilotIrOrPattern orp => (string.Join(" | ", orp.Alternatives.Select(a => EmitPattern(a).Pattern)), null),
            PilotIrTuplePattern tp => ($"({string.Join(", ", tp.Elements.Select(e => EmitPattern(e).Pattern))})", null),
            _ => ("_", null)
        };

        private string EmitTryStmt(PilotIrTryStmt t)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}try:");
            EmitSuite(sb, t.TryBody);
            foreach (var c in t.Catches)
            {
                string exType = !string.IsNullOrEmpty(c.ExceptionType) && c.ExceptionType != "Error" ? c.ExceptionType : "Exception";
                string asVar = !string.IsNullOrEmpty(c.VariableName) ? $" as {c.VariableName}" : "";
                sb.AppendLine($"{Indent}except {exType}{asVar}:");
                EmitSuite(sb, c.Body);
            }
            if (t.FinallyBody.Count > 0)
            {
                sb.AppendLine($"{Indent}finally:");
                EmitSuite(sb, t.FinallyBody);
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitDeferStmt(PilotIrDeferStmt d)
        {
            if (d.Body.Count == 0)
                return $"{Indent}pass";
            return string.Join(Environment.NewLine, d.Body.Select(EmitStmt));
        }

        private void EmitSuite(StringBuilder sb, List<PilotIrStmt> stmts)
        {
            _indent++;
            if (stmts.Count == 0)
            {
                sb.AppendLine($"{Indent}pass");
            }
            else
            {
                foreach (var s in stmts)
                {
                    EmitLeadingComments(sb, s);
                    string line = EmitStmt(s);
                    if (!string.IsNullOrWhiteSpace(line))
                        sb.AppendLine(line);
                }
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
                    "true" => "True",
                    "false" => "False",
                    "null" or "none" or "undefined" => "None",
                    "self" or "this" => "self",
                    _ => id.Name
                },
                PilotIrUnaryExpr u => EmitUnary(u),
                PilotIrBinaryExpr b => EmitBinary(b),
                PilotIrRangeExpr r => EmitForIterable(r),
                PilotIrCallExpr c => EmitCall(c),
                PilotIrMemberAccessExpr m => EmitMemberAccess(m),
                PilotIrIndexExpr ix => EmitIndex(ix),
                PilotIrPostfixTryExpr pt => EmitPostfixTry(pt),
                PilotIrCollectionExpr col => EmitCollection(col),
                PilotIrLambdaExpr lam => EmitLambda(lam),
                PilotIrForExpr fe => EmitForExpr(fe),
                PilotIrIfExpr ie => $"{EmitExpr(ie.ThenBranch, PilotIrPrecedence.Conditional)} if {EmitExpr(ie.Condition, PilotIrPrecedence.Conditional)} else {EmitExpr(ie.ElseBranch, PilotIrPrecedence.Conditional)}",
                PilotIrMatchExpr me => EmitMatchExpr(me),
                PilotIrSqlQueryExpr sq => $"[{EmitExpr(sq.SelectProjection)} for {sq.RangeVariable} in {EmitExpr(sq.Source)}{(sq.WherePredicate != null ? $" if {EmitExpr(sq.WherePredicate)}" : "")}]",
                _ => "None"
            };

            if (parentPrec != PilotIrPrecedence.Lowest && expr.Precedence < parentPrec)
            {
                return $"({core})";
            }
            return core;
        }

        private static string EmitLiteral(PilotIrLiteralExpr lit) => lit.Kind switch
        {
            PilotLiteralKind.Boolean => lit.RawValue is "true" or "True" ? "True" : "False",
            PilotLiteralKind.Null or PilotLiteralKind.None => "None",
            PilotLiteralKind.DimensionalUnit => (lit.NormalizedNumericValue ?? 0.0).ToString(System.Globalization.CultureInfo.InvariantCulture),
            _ => lit.RawValue
        };

        private string EmitInterpolatedString(PilotIrInterpolatedStringExpr fstr)
        {
            var sb = new StringBuilder("f\"");
            foreach (var part in fstr.Parts)
            {
                if (!part.IsInterpolation)
                {
                    sb.Append(part.LiteralText);
                }
                else
                {
                    string inner = part.Expression != null ? EmitExpr(part.Expression) : part.LiteralText;
                    string spec = !string.IsNullOrEmpty(part.FormatSpec) ? $":{part.FormatSpec}" : "";
                    sb.Append($"{{{inner}{spec}}}");
                }
            }
            sb.Append('"');
            return sb.ToString();
        }

        private string EmitUnary(PilotIrUnaryExpr u)
        {
            string op = u.Operator is "!" or "not" ? "not " : u.Operator;
            return $"{op}{EmitExpr(u.Operand, u.Precedence)}";
        }

        private string EmitBinary(PilotIrBinaryExpr b)
        {
            if (b.Operator == "|>")
                return $"{EmitExpr(b.Right, PilotIrPrecedence.Postfix)}({EmitExpr(b.Left)})";
            if (b.Operator == "<|")
                return $"{EmitExpr(b.Left, PilotIrPrecedence.Postfix)}({EmitExpr(b.Right)})";
            if (b.Operator == "??")
            {
                string l = EmitExpr(b.Left, PilotIrPrecedence.Comparison);
                string r = EmitExpr(b.Right, PilotIrPrecedence.Conditional);
                return $"({l} if {l} is not None else {r})";
            }

            string pyOp = b.Operator switch
            {
                "&&" or "and" => "and",
                "||" or "or" => "or",
                "->" => "@",
                _ => b.Operator
            };

            string left = EmitExpr(b.Left, b.Precedence);
            var rightMinPrec = b.Operator is "-" or "/" or "//" or "%"
                ? (PilotIrPrecedence)((int)b.Precedence + 1)
                : b.Precedence;
            string right = EmitExpr(b.Right, rightMinPrec);
            return $"{left} {pyOp} {right}";
        }

        private string EmitCall(PilotIrCallExpr c)
        {
            var renderedArgs = c.Arguments.Select(a =>
                !string.IsNullOrEmpty(a.Name) ? $"{a.Name}={EmitExpr(a.Value)}" : EmitExpr(a.Value)).ToList();

            if (PilotLibraryMapper.TryFormatAstLibraryCall(c.LibraryCallKind, renderedArgs, "python", out string mapped))
            {
                return mapped;
            }

            if (c.Callee is PilotIrMemberAccessExpr mem)
            {
                string recv = EmitExpr(mem.Target, PilotIrPrecedence.Postfix);
                if (mem.MemberName is "push" or "Add")
                    return $"{recv}.append({string.Join(", ", renderedArgs)})";
                if (_extensionTargets.Contains(mem.MemberName))
                {
                    foreach (var ext in _module.Statements.OfType<PilotIrExtensionDefStmt>())
                    {
                        if (ext.Methods.Any(m => m.Name == mem.MemberName))
                        {
                            var extArgs = new List<string> { recv };
                            extArgs.AddRange(renderedArgs);
                            return $"__ext_{ext.TargetType}_{mem.MemberName}({string.Join(", ", extArgs)})";
                        }
                    }
                }
            }

            string callee = EmitExpr(c.Callee, PilotIrPrecedence.Postfix);
            return $"{callee}({string.Join(", ", renderedArgs)})";
        }

        private string EmitMemberAccess(PilotIrMemberAccessExpr m)
        {
            string target = EmitExpr(m.Target, PilotIrPrecedence.Postfix);
            if (PilotLibraryMapper.TryFormatAstMemberAccess(target, m.MemberName, "python", out string mapped))
                return mapped;
            if (m.IsSafeNavigation)
                return $"({target}.{m.MemberName} if {target} is not None else None)";
            if (m.MemberName is "Length" or "Count")
                return $"len({target})";
            return $"{target}.{m.MemberName}";
        }

        private string EmitIndex(PilotIrIndexExpr ix)
        {
            string target = EmitExpr(ix.Target, PilotIrPrecedence.Postfix);
            if (ix.IsSlice)
            {
                string start = EmitExpr(ix.Index);
                string end = ix.SliceEnd != null ? EmitExpr(ix.SliceEnd) : "";
                return $"{target}[{start}:{end}]";
            }
            return $"{target}[{EmitExpr(ix.Index)}]";
        }

        private string EmitPostfixTry(PilotIrPostfixTryExpr pt)
        {
            string op = EmitExpr(pt.Operand, PilotIrPrecedence.Postfix);
            if (!pt.IsQuestionPropagate && pt.CatchFallbackExpr != null)
            {
                string fb = EmitExpr(pt.CatchFallbackExpr, PilotIrPrecedence.Conditional);
                return $"({op} if {op} is not None else {fb})";
            }
            return op;
        }

        private string EmitCollection(PilotIrCollectionExpr col) => col.Kind switch
        {
            PilotCollectionKind.List => $"[{string.Join(", ", col.Elements.Select(e => EmitExpr(e)))}]",
            PilotCollectionKind.Tuple => col.Elements.Count == 1
                ? $"({EmitExpr(col.Elements[0])},)"
                : $"({string.Join(", ", col.Elements.Select(e => EmitExpr(e)))})",
            PilotCollectionKind.Set => col.Elements.Count == 0
                ? "set()"
                : $"{{{string.Join(", ", col.Elements.Select(e => EmitExpr(e)))}}}",
            PilotCollectionKind.Map => $"{{{string.Join(", ", col.MapEntries.Select(kv => $"{EmitExpr(kv.Key)}: {EmitExpr(kv.Value)}"))}}}",
            _ => "[]"
        };

        private string EmitLambda(PilotIrLambdaExpr lam)
        {
            string parms = string.Join(", ", lam.Parameters.Select(p => p.Name));
            string body = lam.BodyExpr != null ? EmitExpr(lam.BodyExpr) : "None";
            return string.IsNullOrEmpty(parms) ? $"lambda: {body}" : $"lambda {parms}: {body}";
        }

        private string EmitForExpr(PilotIrForExpr fe)
        {
            string target = fe.TargetVars.Count > 0 ? string.Join(", ", fe.TargetVars) : "_x";
            string iter = EmitForIterable(fe.Iterable);
            string yieldVal = EmitExpr(fe.YieldExpr);
            string filter = fe.FilterCondition != null ? $" if {EmitExpr(fe.FilterCondition)}" : "";
            return $"[{yieldVal} for {target} in {iter}{filter}]";
        }

        private string EmitMatchExpr(PilotIrMatchExpr me)
        {
            if (me.Arms.Count == 0)
                return "None";
            string scrut = EmitExpr(me.Scrutinee, PilotIrPrecedence.Comparison);
            string expr = "None";
            for (int i = me.Arms.Count - 1; i >= 0; i--)
            {
                var arm = me.Arms[i];
                string val = arm.ExpressionBody != null ? EmitExpr(arm.ExpressionBody) : "None";
                if (arm.Pattern is PilotIrWildcardPattern or PilotIrBindingPattern)
                {
                    expr = val;
                }
                else
                {
                    var (pat, _) = EmitPattern(arm.Pattern);
                    expr = $"({val} if {scrut} == {pat} else {expr})";
                }
            }
            return expr;
        }

        private string MapType(string pilotType)
        {
            string t = pilotType.Trim();
            if (_options.TypeMappings.TryGetValue(t, out var mapped))
                return mapped;

            if (t.EndsWith("?"))
                return $"Optional[{MapType(t[..^1])}]";
            if (t.Contains('!'))
            {
                var parts = t.Split('!', 2);
                return MapType(parts[0].Trim());
            }
            if (t.StartsWith("list[") && t.EndsWith("]"))
                return $"List[{MapType(t[5..^1])}]";
            if (t.StartsWith("set[") && t.EndsWith("]"))
                return $"Set[{MapType(t[4..^1])}]";
            if (t.StartsWith("map[") && t.EndsWith("]"))
            {
                var inner = t[4..^1].Split(',', 2);
                return inner.Length == 2
                    ? $"Dict[{MapType(inner[0])}, {MapType(inner[1])}]"
                    : "Dict[Any, Any]";
            }

            return t switch
            {
                "i8" or "i16" or "i32" or "i64" or "int" or "u8" or "u16" or "u32" or "u64" or "uint" => "int",
                "f8" or "f16" or "f32" or "f64" or "float" or "double" or "decimal" => "float",
                "str" or "string" => "str",
                "bool" => "bool",
                "void" => "None",
                "any" or "auto" or "object" => "Any",
                "list" => "List[Any]",
                "map" => "Dict[Any, Any]",
                "set" => "Set[Any]",
                "tuple" => "Tuple[Any, ...]",
                _ => t
            };
        }
    }
}
