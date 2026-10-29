using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using Pilot.Transpilation.Ir;
using Pilot.Transpilation.Licensing;

namespace Pilot.Transpilation.Ingesters;

/// <summary>
/// Milestone 20.4: Structured AST frontends for Python 3, Rust, C11, C++20, JavaScript/TypeScript, and Lua.
/// Parses foreign declarations, control flow, ADTs, <c>Option&lt;T&gt;</c>/<c>Result&lt;T, E&gt;</c>, and expressions
/// into <see cref="PilotIrModule"/>, applies <c>--strict-pilot</c> upgrades, formats via <c>PilotFormatter</c>,
/// and validates with <see cref="PilotIngestValidator"/>.
/// </summary>
public static partial class ForeignAstIngester
{
    public static TranspilationResult Ingest(string foreignSource, string sourceLanguage, TranspilationOptions options)
    {
        string lang = sourceLanguage.ToLowerInvariant();
        if (lang is "csharp" or "cs")
        {
            return CSharpAstIngester.Ingest(foreignSource, options);
        }

        var lic = PilotLicenseVerifier.VerifySource(
            foreignSource,
            options.SourceFilePath,
            options.LicensePolicy,
            options.CustomSpdxAllowlist);
        if (!lic.IsAllowed)
        {
            return TranspilationResult.CreateFailure(lic.DiagnosticMessage ?? "License rejected", "pilot", lic);
        }

        var module = new PilotIrModule
        {
            PreferColonSuites = options.PreferColonSuites
        };

        if (options.EmitHeaderComments)
        {
            string displayLang = lang switch
            {
                "python" or "py" => "Python",
                "rust" or "rs" => "Rust",
                "cpp" or "c++" => "C++",
                "c" => "C",
                "ts" or "typescript" => "TypeScript",
                "js" or "javascript" => "JavaScript",
                "lua" => "Lua",
                _ => sourceLanguage
            };
            module.HeaderComments.Add($"// Converted & optimized from {displayLang} to Pilot");
        }

        switch (lang)
        {
            case "python" or "py":
                ParsePythonModule(foreignSource, module, options);
                break;
            case "rust" or "rs":
                ParseRustModule(foreignSource, module, options);
                break;
            case "c":
                ParseCOrCppModule(foreignSource, module, options, isCpp: false);
                break;
            case "cpp" or "c++":
                ParseCOrCppModule(foreignSource, module, options, isCpp: true);
                break;
            case "js" or "javascript" or "ts" or "typescript":
                ParseJavaScriptModule(foreignSource, module, options);
                break;
            case "lua":
                ParseLuaModule(foreignSource, module, options);
                break;
            default:
                return TranspilationResult.CreateFailure($"Unsupported foreign source language '{sourceLanguage}'.", "pilot", lic);
        }

        return PilotIngestValidator.ValidateAndFinalize(module, options, lang, lic);
    }

    // ========================================================================
    // 1. PYTHON 3 INDENTATION-AWARE AST FRONTEND
    // ========================================================================

    private static void ParsePythonModule(string source, PilotIrModule module, TranspilationOptions options)
    {
        var rawLines = source.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None);
        var logicalLines = MergeMultiLineConstructs(rawLines);
        int idx = 0;
        var declaredScope = new HashSet<string>(StringComparer.Ordinal);
        var parsed = ParsePythonSuite(logicalLines, ref idx, baseIndent: 0, declaredScope);
        module.Statements.AddRange(PostProcessPythonModuleStatements(parsed));
    }

    private static List<PilotIrStmt> PostProcessPythonModuleStatements(List<PilotIrStmt> stmts)
    {
        var result = new List<PilotIrStmt>();
        for (int i = 0; i < stmts.Count; i++)
        {
            var s = stmts[i];

            // Reconstruct __ext_Type_Method functions into extend Type { def Method(self, ...) }
            if (s is PilotIrFuncDefStmt fn && fn.Name.StartsWith("__ext_", StringComparison.Ordinal))
            {
                string rem = fn.Name["__ext_".Length..];
                int splitIdx = rem.IndexOf('_');
                if (splitIdx > 0)
                {
                    string targetType = MapForeignType(rem[..splitIdx], "python");
                    string methodName = rem[(splitIdx + 1)..];
                    fn.Name = methodName;
                    if (fn.Parameters.Count > 0 && fn.Parameters[0].Name == "self_val")
                        fn.Parameters[0] = new PilotIrParameter("self");

                    var existingExt = result.OfType<PilotIrExtensionDefStmt>().FirstOrDefault(e => e.TargetType == targetType);
                    if (existingExt == null)
                    {
                        existingExt = new PilotIrExtensionDefStmt { Line = fn.Line, TargetType = targetType };
                        result.Add(existingExt);
                    }
                    existingExt.Methods.Add(fn);
                    continue;
                }
            }

            // Reconstruct Name = Union[V1, V2, ...] + preceding dataclass classes into PilotIrAdtDefStmt
            if (s is PilotIrTypeAliasStmt ta && ta.TargetType.StartsWith("Union[", StringComparison.Ordinal) && ta.TargetType.EndsWith("]", StringComparison.Ordinal))
            {
                string rawArms = ta.TargetType["Union[".Length..^1];
                var armNames = SplitTopLevelComma(rawArms).Select(a => a.Trim()).Where(a => a.Length > 0).ToList();
                var matchedClasses = new List<PilotIrClassDefStmt>();
                foreach (var arm in armNames)
                {
                    var cls = result.OfType<PilotIrClassDefStmt>().FirstOrDefault(c => c.Name == arm && c.Methods.Count == 0);
                    if (cls != null) matchedClasses.Add(cls);
                }

                if (matchedClasses.Count == armNames.Count && matchedClasses.Count > 0)
                {
                    var adt = new PilotIrAdtDefStmt { Line = ta.Line, Name = ta.Name };
                    foreach (var mc in matchedClasses)
                    {
                        result.Remove(mc);
                        adt.Variants.Add(new PilotIrAdtVariant(
                            mc.Name,
                            mc.Fields.Select(f => (f.Name, f.TypeAnnotation)).ToList()));
                    }
                    result.Add(adt);
                    continue;
                }

                result.Add(new PilotIrTypeAliasStmt
                {
                    Line = ta.Line,
                    Name = ta.Name,
                    TargetType = string.Join(" | ", armNames)
                });
                continue;
            }

            result.Add(s);
        }
        return result;
    }

    private static List<(int Indent, string Text, int LineNo)> MergeMultiLineConstructs(string[] rawLines)
    {
        var result = new List<(int Indent, string Text, int LineNo)>();
        var current = new StringBuilder();
        int currentIndent = 0;
        int startLine = 1;
        int bracketDepth = 0;

        for (int i = 0; i < rawLines.Length; i++)
        {
            string line = rawLines[i];
            string trimmed = line.Trim();

            if (bracketDepth == 0)
            {
                if (string.IsNullOrWhiteSpace(trimmed))
                {
                    result.Add((0, "", i + 1));
                    continue;
                }
                if (trimmed.StartsWith("#"))
                {
                    result.Add((CountLeadingSpaces(line), trimmed, i + 1));
                    continue;
                }
                currentIndent = CountLeadingSpaces(line);
                startLine = i + 1;
                current.Clear();
                current.Append(StripTrailingPythonComment(trimmed, out _));
            }
            else
            {
                current.Append(' ').Append(StripTrailingPythonComment(trimmed, out _));
            }

            bracketDepth = CountUnclosedBrackets(current.ToString());
            if (bracketDepth <= 0)
            {
                bracketDepth = 0;
                result.Add((currentIndent, current.ToString().Trim(), startLine));
            }
        }

        if (bracketDepth > 0 && current.Length > 0)
        {
            result.Add((currentIndent, current.ToString().Trim(), startLine));
        }

        return result;
    }

    private static List<PilotIrStmt> ParsePythonSuite(
        List<(int Indent, string Text, int LineNo)> lines,
        ref int idx,
        int baseIndent,
        HashSet<string> declaredVars)
    {
        var stmts = new List<PilotIrStmt>();
        var pendingTrivia = new List<PilotTrivia>();
        var pendingDecorators = new List<string>();

        while (idx < lines.Count)
        {
            var (indent, text, lineNo) = lines[idx];

            if (string.IsNullOrWhiteSpace(text))
            {
                pendingTrivia.Add(new PilotTrivia(PilotTriviaKind.BlankLine, "", lineNo));
                idx++;
                continue;
            }

            if (text.StartsWith("#"))
            {
                if (indent < baseIndent)
                    break;
                pendingTrivia.Add(new PilotTrivia(PilotTriviaKind.LineComment, "//" + text[1..], lineNo));
                idx++;
                continue;
            }

            if (indent < baseIndent)
                break;

            if (text.StartsWith("@"))
            {
                string dec = text[1..].Trim();
                if (dec != "dataclass")
                    pendingDecorators.Add("@" + dec);
                idx++;
                continue;
            }

            idx++;

            if (text.StartsWith("from __future__") || text.StartsWith("from typing import") || text == "import typing" ||
                text == "from dataclasses import dataclass" || text == "from enum import Enum")
                continue;

            if (text.StartsWith("import ") || text.StartsWith("from "))
            {
                string mapped = PilotLibraryMapper.TranslateForeignLibrariesToPilot(text, "python");
                if (mapped.StartsWith("//"))
                {
                    var cmt = new PilotIrCommentStmt { CommentText = mapped, Line = lineNo };
                    AttachTrivia(cmt, pendingTrivia);
                    stmts.Add(cmt);
                }
                continue;
            }

            if (text.StartsWith("def ") || text.StartsWith("async def "))
            {
                bool isAsync = text.StartsWith("async ");
                string rest = isAsync ? text["async def ".Length..] : text["def ".Length..];
                var fn = ParsePythonFuncHeader(rest, isAsync, lineNo);
                fn.Decorators.AddRange(pendingDecorators);
                pendingDecorators.Clear();
                AttachTrivia(fn, pendingTrivia);

                var fnScope = new HashSet<string>(declaredVars, StringComparer.Ordinal);
                foreach (var p in fn.Parameters)
                    fnScope.Add(p.Name);

                int childIndent = PeekNextBlockIndent(lines, idx, indent);
                fn.Body.AddRange(ParsePythonSuite(lines, ref idx, childIndent, fnScope));
                stmts.Add(fn);
                continue;
            }

            if (text.StartsWith("class "))
            {
                var cls = ParsePythonClassHeader(text["class ".Length..], lineNo);
                cls.Decorators.AddRange(pendingDecorators);
                pendingDecorators.Clear();
                AttachTrivia(cls, pendingTrivia);

                int childIndent = PeekNextBlockIndent(lines, idx, indent);
                var bodyStmts = ParsePythonSuite(lines, ref idx, childIndent, new HashSet<string>(StringComparer.Ordinal));

                if (cls.BaseTypes.Contains("Enum"))
                {
                    var enStmt = new PilotIrEnumDefStmt { Line = lineNo, Name = cls.Name };
                    AttachTrivia(enStmt, cls.LeadingTrivia);
                    foreach (var vd in bodyStmts.OfType<PilotIrVarDeclStmt>())
                        enStmt.Members.Add((vd.Name, vd.Initializer));
                    stmts.Add(enStmt);
                    continue;
                }

                foreach (var bs in bodyStmts)
                {
                    if (bs is PilotIrFuncDefStmt m)
                    {
                        if (m.Name == "__init__")
                        {
                            bool allSimpleFieldInit = true;
                            foreach (var initStmt in m.Body)
                            {
                                if (initStmt is PilotIrAssignStmt { Target: PilotIrMemberAccessExpr { Target: PilotIrIdentifierExpr { Name: "self" } } mem } assign)
                                {
                                    if (!cls.Fields.Any(f => f.Name == mem.MemberName))
                                    {
                                        cls.Fields.Add(new PilotIrFieldDecl(mem.MemberName, "any", assign.Value));
                                    }
                                }
                                else
                                {
                                    allSimpleFieldInit = false;
                                }
                            }
                            if (allSimpleFieldInit)
                                continue;
                        }
                        cls.Methods.Add(m);
                    }
                    else if (bs is PilotIrVarDeclStmt vd)
                    {
                        cls.Fields.Add(new PilotIrFieldDecl(vd.Name, vd.DeclaredType ?? "any", vd.Initializer));
                    }
                }
                stmts.Add(cls);
                continue;
            }

            if (text.StartsWith("if ") && text.EndsWith(":"))
            {
                string condStr = text["if ".Length..^1].Trim();
                var ifStmt = new PilotIrIfStmt
                {
                    Line = lineNo,
                    Condition = ParseForeignExpr(condStr, "python")
                };
                AttachTrivia(ifStmt, pendingTrivia);

                int childIndent = PeekNextBlockIndent(lines, idx, indent);
                ifStmt.ThenBody.AddRange(ParsePythonSuite(lines, ref idx, childIndent, declaredVars));

                while (idx < lines.Count)
                {
                    SkipBlankLines(lines, ref idx);
                    if (idx >= lines.Count || lines[idx].Indent != indent)
                        break;

                    string nextText = lines[idx].Text;
                    if (nextText.StartsWith("elif ") && nextText.EndsWith(":"))
                    {
                        idx++;
                        string elifCond = nextText["elif ".Length..^1].Trim();
                        int elifIndent = PeekNextBlockIndent(lines, idx, indent);
                        var elifBody = ParsePythonSuite(lines, ref idx, elifIndent, declaredVars);
                        ifStmt.ElifBranches.Add((ParseForeignExpr(elifCond, "python"), elifBody));
                    }
                    else if (nextText == "else:" || nextText.StartsWith("else "))
                    {
                        idx++;
                        int elseIndent = PeekNextBlockIndent(lines, idx, indent);
                        ifStmt.ElseBody.AddRange(ParsePythonSuite(lines, ref idx, elseIndent, declaredVars));
                        break;
                    }
                    else
                    {
                        break;
                    }
                }

                stmts.Add(ifStmt);
                continue;
            }

            if (text.StartsWith("while ") && text.EndsWith(":"))
            {
                string condStr = text["while ".Length..^1].Trim();
                var ws = new PilotIrWhileStmt
                {
                    Line = lineNo,
                    Condition = ParseForeignExpr(condStr, "python")
                };
                AttachTrivia(ws, pendingTrivia);
                int childIndent = PeekNextBlockIndent(lines, idx, indent);
                ws.Body.AddRange(ParsePythonSuite(lines, ref idx, childIndent, declaredVars));
                stmts.Add(ws);
                continue;
            }

            if (text.StartsWith("for ") && text.EndsWith(":"))
            {
                string inner = text["for ".Length..^1].Trim();
                int inIdx = inner.IndexOf(" in ", StringComparison.Ordinal);
                if (inIdx > 0)
                {
                    string targetPart = inner[..inIdx].Trim();
                    string iterPart = inner[(inIdx + 4)..].Trim();
                    var fs = new PilotIrForStmt
                    {
                        Line = lineNo,
                        Iterable = ConvertPythonIterable(iterPart)
                    };
                    foreach (var tv in targetPart.Split(',').Select(s => s.Trim().Trim('(', ')')))
                    {
                        if (!string.IsNullOrEmpty(tv))
                        {
                            fs.TargetVars.Add(tv);
                            declaredVars.Add(tv);
                        }
                    }
                    AttachTrivia(fs, pendingTrivia);
                    int childIndent = PeekNextBlockIndent(lines, idx, indent);
                    fs.Body.AddRange(ParsePythonSuite(lines, ref idx, childIndent, declaredVars));
                    stmts.Add(fs);
                    continue;
                }
            }

            if (text.StartsWith("match ") && text.EndsWith(":"))
            {
                string scrut = text["match ".Length..^1].Trim();
                var ms = new PilotIrMatchStmt
                {
                    Line = lineNo,
                    Scrutinee = ParseForeignExpr(scrut, "python")
                };
                AttachTrivia(ms, pendingTrivia);
                int caseIndent = PeekNextBlockIndent(lines, idx, indent);
                while (idx < lines.Count)
                {
                    SkipBlankLines(lines, ref idx);
                    if (idx >= lines.Count || lines[idx].Indent < caseIndent)
                        break;
                    string caseLine = lines[idx].Text;
                    if (caseLine.StartsWith("case ") && caseLine.EndsWith(":"))
                    {
                        idx++;
                        string patStr = caseLine["case ".Length..^1].Trim();
                        int armIndent = PeekNextBlockIndent(lines, idx, caseIndent);
                        var armBody = ParsePythonSuite(lines, ref idx, armIndent, declaredVars);
                        ms.Arms.Add(new PilotIrMatchArm(ParseForeignPattern(patStr), null, armBody));
                    }
                    else
                    {
                        idx++;
                    }
                }
                stmts.Add(ms);
                continue;
            }

            if (text == "try:")
            {
                var tr = new PilotIrTryStmt { Line = lineNo };
                AttachTrivia(tr, pendingTrivia);
                int tryIndent = PeekNextBlockIndent(lines, idx, indent);
                tr.TryBody.AddRange(ParsePythonSuite(lines, ref idx, tryIndent, declaredVars));
                while (idx < lines.Count)
                {
                    SkipBlankLines(lines, ref idx);
                    if (idx >= lines.Count || lines[idx].Indent != indent)
                        break;
                    string nextLine = lines[idx].Text;
                    if (nextLine.StartsWith("except") && nextLine.EndsWith(":"))
                    {
                        idx++;
                        string clause = nextLine["except".Length..^1].Trim();
                        string? exType = null;
                        string? exVar = "err";
                        if (clause.Contains(" as "))
                        {
                            var parts = clause.Split(" as ", 2);
                            exType = MapForeignType(parts[0].Trim(), "python");
                            exVar = parts[1].Trim();
                        }
                        else if (!string.IsNullOrEmpty(clause))
                        {
                            exType = MapForeignType(clause, "python");
                        }
                        int exIndent = PeekNextBlockIndent(lines, idx, indent);
                        var exBody = ParsePythonSuite(lines, ref idx, exIndent, declaredVars);
                        tr.Catches.Add(new PilotIrCatchClause(exType, exVar, exBody));
                    }
                    else if (nextLine == "finally:")
                    {
                        idx++;
                        int finIndent = PeekNextBlockIndent(lines, idx, indent);
                        tr.FinallyBody.AddRange(ParsePythonSuite(lines, ref idx, finIndent, declaredVars));
                        break;
                    }
                    else
                    {
                        break;
                    }
                }
                stmts.Add(tr);
                continue;
            }

            var simpleStmt = ParsePythonSimpleStmt(text, lineNo, declaredVars);
            if (simpleStmt != null)
            {
                AttachTrivia(simpleStmt, pendingTrivia);
                stmts.Add(simpleStmt);
            }
        }

        foreach (var tr in pendingTrivia.Where(t => t.Kind != PilotTriviaKind.BlankLine))
        {
            stmts.Add(new PilotIrCommentStmt { CommentText = tr.Text, Line = tr.Line });
        }

        return stmts;
    }

    private static PilotIrFuncDefStmt ParsePythonFuncHeader(string header, bool isAsync, int lineNo)
    {
        if (header.EndsWith(":"))
            header = header[..^1].TrimEnd();

        int openParen = header.IndexOf('(');
        int closeParen = FindMatchingCloseParen(header, openParen);
        string name = openParen > 0 ? header[..openParen].Trim() : header.Trim();
        string rawParams = (openParen >= 0 && closeParen > openParen) ? header[(openParen + 1)..closeParen] : "";
        string? retType = null;

        if (closeParen > 0 && closeParen + 1 < header.Length)
        {
            string after = header[(closeParen + 1)..].Trim();
            if (after.StartsWith("->"))
            {
                retType = MapForeignType(after[2..].Trim(), "python");
            }
        }

        var fn = new PilotIrFuncDefStmt
        {
            Line = lineNo,
            Name = name,
            DeclaredReturnType = retType == "void" ? null : retType,
            IsAsync = isAsync
        };

        foreach (var p in SplitTopLevelComma(rawParams))
        {
            string pt = p.Trim();
            if (string.IsNullOrEmpty(pt)) continue;
            if (pt == "self" || pt == "cls")
            {
                fn.Parameters.Add(new PilotIrParameter("self"));
                continue;
            }

            PilotIrExpr? defVal = null;
            int eqIdx = FindTopLevelChar(pt, '=');
            if (eqIdx > 0)
            {
                defVal = ParseForeignExpr(pt[(eqIdx + 1)..].Trim(), "python");
                pt = pt[..eqIdx].Trim();
            }

            string? typeAnnot = null;
            int colonIdx = FindTopLevelChar(pt, ':');
            if (colonIdx > 0)
            {
                typeAnnot = MapForeignType(pt[(colonIdx + 1)..].Trim(), "python");
                pt = pt[..colonIdx].Trim();
            }

            fn.Parameters.Add(new PilotIrParameter(pt.TrimStart('*'), typeAnnot, defVal, IsVariadic: pt.StartsWith("*")));
        }

        return fn;
    }

    private static PilotIrClassDefStmt ParsePythonClassHeader(string header, int lineNo)
    {
        if (header.EndsWith(":"))
            header = header[..^1].TrimEnd();

        var cls = new PilotIrClassDefStmt { Line = lineNo, Kind = PilotTypeDeclKind.Class };
        int openParen = header.IndexOf('(');
        if (openParen > 0)
        {
            cls.Name = header[..openParen].Trim();
            int closeParen = FindMatchingCloseParen(header, openParen);
            if (closeParen > openParen)
            {
                foreach (var b in SplitTopLevelComma(header[(openParen + 1)..closeParen]))
                {
                    string bt = b.Trim();
                    if (!string.IsNullOrEmpty(bt) && bt != "object")
                        cls.BaseTypes.Add(bt);
                }
            }
        }
        else
        {
            cls.Name = header.Trim();
        }
        return cls;
    }

    private static PilotIrStmt? ParsePythonSimpleStmt(string text, int lineNo, HashSet<string> declaredVars)
    {
        string s = PilotLibraryMapper.TranslateForeignLibrariesToPilot(text.TrimEnd(';'), "python");

        if (s == "pass") return new PilotIrPassStmt { Line = lineNo };
        if (s == "break") return new PilotIrBreakStmt { Line = lineNo };
        if (s == "continue") return new PilotIrContinueStmt { Line = lineNo };

        if (s == "return" || s.StartsWith("return "))
        {
            string valPart = s.Length > 6 ? s["return".Length..].Trim() : "";
            return new PilotIrReturnStmt
            {
                Line = lineNo,
                Value = !string.IsNullOrEmpty(valPart) ? ParseForeignExpr(valPart, "python") : null
            };
        }

        if (s.StartsWith("yield "))
        {
            return new PilotIrYieldStmt
            {
                Line = lineNo,
                Value = ParseForeignExpr(s["yield ".Length..].Trim(), "python")
            };
        }

        if (s.StartsWith("raise "))
        {
            return new PilotIrRaiseStmt
            {
                Line = lineNo,
                Value = ParseForeignExpr(s["raise ".Length..].Trim(), "python")
            };
        }

        if (s.StartsWith("assert "))
        {
            var parts = SplitTopLevelComma(s["assert ".Length..]);
            return new PilotIrAssertStmt
            {
                Line = lineNo,
                Condition = ParseForeignExpr(parts[0], "python"),
                Message = parts.Count > 1 ? ParseForeignExpr(parts[1], "python") : null
            };
        }

        // Check augmented assignment
        foreach (var aug in new[] { "+=", "-=", "*=", "/=", "//=", "%=", "**=" })
        {
            int augIdx = FindTopLevelOperator(s, aug);
            if (augIdx > 0)
            {
                return new PilotIrAssignStmt
                {
                    Line = lineNo,
                    Target = ParseForeignExpr(s[..augIdx].Trim(), "python"),
                    Operator = aug,
                    Value = ParseForeignExpr(s[(augIdx + aug.Length)..].Trim(), "python")
                };
            }
        }

        // Check variable declaration or assignment: x: Type = val or x = val
        int assignIdx = FindTopLevelSingleEquals(s);
        if (assignIdx > 0)
        {
            string lhs = s[..assignIdx].Trim();
            string rhs = s[(assignIdx + 1)..].Trim();
            string? declType = null;
            int colonIdx = FindTopLevelChar(lhs, ':');
            if (colonIdx > 0)
            {
                declType = MapForeignType(lhs[(colonIdx + 1)..].Trim(), "python");
                lhs = lhs[..colonIdx].Trim();
            }

            if (IsIdentifier(lhs) && !lhs.Contains('.'))
            {
                if (rhs.StartsWith("Union[", StringComparison.Ordinal) && rhs.EndsWith("]", StringComparison.Ordinal))
                {
                    return new PilotIrTypeAliasStmt
                    {
                        Line = lineNo,
                        Name = lhs,
                        TargetType = rhs
                    };
                }

                bool isConst = declType == "Final" || lhs.All(c => char.IsUpper(c) || c == '_' || char.IsDigit(c)) && lhs.Any(char.IsUpper);
                if (declType == "Final") declType = null;

                if (!declaredVars.Contains(lhs))
                {
                    declaredVars.Add(lhs);
                    return new PilotIrVarDeclStmt
                    {
                        Line = lineNo,
                        Kind = isConst ? PilotVarDeclKind.Const : PilotVarDeclKind.Let,
                        Name = lhs,
                        DeclaredType = declType,
                        Initializer = ParseForeignExpr(rhs, "python")
                    };
                }
            }

            return new PilotIrAssignStmt
            {
                Line = lineNo,
                Target = ParseForeignExpr(lhs, "python"),
                Operator = "=",
                Value = ParseForeignExpr(rhs, "python")
            };
        }

        // Type-annotated field without initializer (e.g. inside @dataclass: r: float)
        int typeColon = FindTopLevelChar(s, ':');
        if (typeColon > 0 && IsIdentifier(s[..typeColon].Trim()))
        {
            string fName = s[..typeColon].Trim();
            string fType = MapForeignType(s[(typeColon + 1)..].Trim(), "python");
            declaredVars.Add(fName);
            return new PilotIrVarDeclStmt
            {
                Line = lineNo,
                Kind = PilotVarDeclKind.Let,
                Name = fName,
                DeclaredType = fType
            };
        }

        return new PilotIrExprStmt
        {
            Line = lineNo,
            Expression = ParseForeignExpr(s, "python")
        };
    }

    private static PilotIrExpr ConvertPythonIterable(string iterCode)
    {
        string s = iterCode.Trim();
        if (s.StartsWith("range(") && s.EndsWith(")"))
        {
            var args = SplitTopLevelComma(s["range(".Length..^1]);
            if (args.Count == 1)
            {
                return new PilotIrRangeExpr
                {
                    Start = new PilotIrLiteralExpr { Kind = PilotLiteralKind.Integer, RawValue = "0" },
                    End = SimplifyMinusOne(args[0].Trim(), out bool wasPlusOne),
                    IsInclusive = wasPlusOne
                };
            }
            if (args.Count >= 2)
            {
                return new PilotIrRangeExpr
                {
                    Start = ParseForeignExpr(args[0].Trim(), "python"),
                    End = SimplifyMinusOne(args[1].Trim(), out bool wasPlusOne),
                    IsInclusive = wasPlusOne
                };
            }
        }
        return ParseForeignExpr(s, "python");
    }

    private static PilotIrExpr SimplifyMinusOne(string endCode, out bool wasPlusOne)
    {
        if (endCode.EndsWith("+ 1") || endCode.EndsWith("+1"))
        {
            wasPlusOne = true;
            int plusIdx = endCode.LastIndexOf('+');
            return ParseForeignExpr(endCode[..plusIdx].Trim(), "python");
        }
        wasPlusOne = false;
        return ParseForeignExpr(endCode, "python");
    }

    // ========================================================================
    // 2. RUST AST FRONTEND
    // ========================================================================

    private static void ParseRustModule(string source, PilotIrModule module, TranspilationOptions options)
    {
        var statements = ParseBraceDelimitedBlock(source, "rust", out var implBlocks);
        var pendingExtensions = new List<PilotIrExtensionDefStmt>();

        foreach (var ext in implBlocks)
        {
            var matchingClass = statements.OfType<PilotIrClassDefStmt>()
                .FirstOrDefault(c => c.Name == ext.TargetType);
            if (matchingClass != null && ext.ExtensionName == null)
            {
                matchingClass.Methods.AddRange(ext.Methods);
            }
            else
            {
                pendingExtensions.Add(ext);
            }
        }

        foreach (var stmt in statements)
        {
            if (stmt is PilotIrFuncDefStmt { Name: "main" } mainFn && mainFn.Parameters.Count == 0)
            {
                // Place extension blocks before top-level main statements
                foreach (var ext in pendingExtensions)
                    module.Statements.Add(ext);
                pendingExtensions.Clear();
                module.Statements.AddRange(mainFn.Body);
                continue;
            }
            module.Statements.Add(stmt);
        }

        foreach (var ext in pendingExtensions)
            module.Statements.Add(ext);
    }

    // ========================================================================
    // 3. C11 & C++20 AST FRONTEND
    // ========================================================================

    private static void ParseCOrCppModule(string source, PilotIrModule module, TranspilationOptions options, bool isCpp)
    {
        string lang = isCpp ? "cpp" : "c";
        var statements = ParseBraceDelimitedBlock(source, lang, out _);
        module.Statements.AddRange(PostProcessCOrCppStatements(statements, isCpp));
    }

    private static List<PilotIrStmt> PostProcessCOrCppStatements(List<PilotIrStmt> stmts, bool isCpp)
    {
        var result = new List<PilotIrStmt>();
        for (int i = 0; i < stmts.Count; i++)
        {
            var s = stmts[i];

            // C tagged-union ADT pattern: enum NameTag + struct Name { NameTag tag; ... }
            if (!isCpp && s is PilotIrEnumDefStmt tagEnum &&
                tagEnum.Name.EndsWith("Tag", StringComparison.Ordinal) &&
                i + 1 < stmts.Count &&
                stmts[i + 1] is PilotIrClassDefStmt adtStruct &&
                adtStruct.Name + "Tag" == tagEnum.Name)
            {
                string adtName = adtStruct.Name;
                var adt = new PilotIrAdtDefStmt { Line = tagEnum.Line, Name = adtName };
                foreach (var (mName, _) in tagEnum.Members)
                {
                    string vName = mName.StartsWith(adtName + "_", StringComparison.Ordinal)
                        ? mName[(adtName.Length + 1)..]
                        : mName;
                    var varFields = adtStruct.Fields
                        .Where(f => f.Name.StartsWith(vName + ".", StringComparison.Ordinal))
                        .Select(f => (f.Name[(vName.Length + 1)..], f.TypeAnnotation))
                        .ToList();
                    adt.Variants.Add(new PilotIrAdtVariant(vName, varFields));
                }
                result.Add(adt);
                i++;
                continue;
            }

            // C++ std::variant ADT pattern: struct V1 ...; struct V2 ...; using Name = std::variant<V1, V2>;
            if (isCpp && s is PilotIrTypeAliasStmt ta &&
                ta.TargetType.StartsWith("std::variant<", StringComparison.Ordinal) &&
                ta.TargetType.EndsWith(">", StringComparison.Ordinal))
            {
                string inner = ta.TargetType["std::variant<".Length..^1];
                var armNames = SplitTopLevelComma(inner).Select(a => a.Trim()).Where(a => a.Length > 0).ToList();
                var matchedStructs = new List<PilotIrClassDefStmt>();
                foreach (var arm in armNames)
                {
                    var st = result.OfType<PilotIrClassDefStmt>().FirstOrDefault(c => c.Name == arm && c.Methods.Count == 0);
                    if (st != null) matchedStructs.Add(st);
                }
                if (matchedStructs.Count == armNames.Count && matchedStructs.Count > 0)
                {
                    var adt = new PilotIrAdtDefStmt { Line = ta.Line, Name = ta.Name };
                    foreach (var ms in matchedStructs)
                    {
                        result.Remove(ms);
                        adt.Variants.Add(new PilotIrAdtVariant(
                            ms.Name,
                            ms.Fields.Select(f => (f.Name, f.TypeAnnotation)).ToList()));
                    }
                    result.Add(adt);
                    continue;
                }
            }

            // Reconstruct C/C++ Type_method(Type self_val, ...) -> class method or extend block
            if (s is PilotIrFuncDefStmt fn && fn.Parameters.Count > 0 && fn.Parameters[0].Name == "self_val")
            {
                string selfType = fn.Parameters[0].TypeAnnotation ?? "i64";
                int usIdx = fn.Name.IndexOf('_');
                if (usIdx > 0)
                {
                    string prefix = fn.Name[..usIdx];
                    string methodName = fn.Name[(usIdx + 1)..];
                    fn.Name = methodName;
                    fn.Parameters[0] = new PilotIrParameter("self");

                    var ownerClass = result.OfType<PilotIrClassDefStmt>().FirstOrDefault(c => c.Name == prefix || c.Name == selfType);
                    if (ownerClass != null)
                    {
                        ownerClass.Methods.Add(fn);
                        continue;
                    }

                    string extTarget = MapForeignType(prefix, isCpp ? "cpp" : "c");
                    var existingExt = result.OfType<PilotIrExtensionDefStmt>().FirstOrDefault(e => e.TargetType == extTarget);
                    if (existingExt == null)
                    {
                        existingExt = new PilotIrExtensionDefStmt { Line = fn.Line, TargetType = extTarget };
                        result.Add(existingExt);
                    }
                    existingExt.Methods.Add(fn);
                    continue;
                }
            }

            // Unwrap main() body statements to top-level
            if (s is PilotIrFuncDefStmt { Name: "main" } mainFn && mainFn.Parameters.Count == 0)
            {
                foreach (var ms in mainFn.Body)
                {
                    if (ms is PilotIrReturnStmt { Value: PilotIrLiteralExpr { RawValue: "0" } })
                        continue;
                    result.Add(ms);
                }
                continue;
            }

            result.Add(s);
        }
        return result;
    }

    // ========================================================================
    // 4. JAVASCRIPT / TYPESCRIPT AST FRONTEND
    // ========================================================================

    private static void ParseJavaScriptModule(string source, PilotIrModule module, TranspilationOptions options)
    {
        var statements = ParseBraceDelimitedBlock(source, "js", out var extBlocks);
        var combined = new List<PilotIrStmt>();
        foreach (var s in statements)
        {
            // Fold TS type Shape = ReturnType<typeof V1> | ... with preceding ADT variant factory functions
            if (s is PilotIrTypeAliasStmt ta && ta.TargetType.Contains("ReturnType<typeof ", StringComparison.Ordinal))
            {
                var arms = ta.TargetType.Split('|')
                    .Select(a => a.Trim())
                    .Select(a => a.StartsWith("ReturnType<typeof ") && a.EndsWith(">")
                        ? a["ReturnType<typeof ".Length..^1].Trim()
                        : a)
                    .ToList();
                var variantFns = new List<PilotIrFuncDefStmt>();
                foreach (var arm in arms)
                {
                    var vf = combined.OfType<PilotIrFuncDefStmt>().FirstOrDefault(f => f.Name == arm);
                    if (vf != null) variantFns.Add(vf);
                }
                if (variantFns.Count == arms.Count && variantFns.Count > 0)
                {
                    var adt = new PilotIrAdtDefStmt { Line = ta.Line, Name = ta.Name };
                    foreach (var vf in variantFns)
                    {
                        combined.Remove(vf);
                        adt.Variants.Add(new PilotIrAdtVariant(
                            vf.Name,
                            vf.Parameters.Select(p => (p.Name, p.TypeAnnotation ?? "any")).ToList()));
                    }
                    combined.Add(adt);
                    continue;
                }
            }
            combined.Add(s);
        }
        foreach (var ext in extBlocks)
            combined.Add(ext);
        module.Statements.AddRange(combined);
    }

    // ========================================================================
    // 5. LUA AST FRONTEND
    // ========================================================================

    private static void ParseLuaModule(string source, PilotIrModule module, TranspilationOptions options)
    {
        // Convert Lua block keywords (then, do, end, elseif) into structured brace blocks first, then parse into IR
        var lines = source.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None);
        var normalized = new StringBuilder();
        foreach (var rawLine in lines)
        {
            string t = rawLine.Trim();
            if (string.IsNullOrWhiteSpace(t))
            {
                normalized.AppendLine();
                continue;
            }
            if (t.StartsWith("--"))
            {
                normalized.AppendLine("//" + t[2..]);
                continue;
            }
            if (t == "end")
            {
                normalized.AppendLine("}");
                continue;
            }
            if (t == "else")
            {
                normalized.AppendLine("} else {");
                continue;
            }
            if (t.StartsWith("elseif ") && t.EndsWith(" then"))
            {
                string cond = t["elseif ".Length..^" then".Length].Trim();
                normalized.AppendLine($"}} elif {cond} {{");
                continue;
            }
            if (t.StartsWith("if ") && t.EndsWith(" then"))
            {
                string cond = t["if ".Length..^" then".Length].Trim();
                normalized.AppendLine($"if ({cond}) {{");
                continue;
            }
            if (t.StartsWith("while ") && t.EndsWith(" do"))
            {
                string cond = t["while ".Length..^" do".Length].Trim();
                normalized.AppendLine($"while ({cond}) {{");
                continue;
            }
            if (t.StartsWith("for ") && t.EndsWith(" do"))
            {
                string inner = t["for ".Length..^" do".Length].Trim();
                var numMatch = LuaNumForRegex().Match(inner);
                if (numMatch.Success)
                {
                    normalized.AppendLine($"for {numMatch.Groups[1].Value} in {numMatch.Groups[2].Value.Trim()}..{numMatch.Groups[3].Value.Trim()} {{");
                    continue;
                }
                var genMatch = LuaGenForRegex().Match(inner);
                if (genMatch.Success)
                {
                    string coll = !string.IsNullOrEmpty(genMatch.Groups[2].Value) ? genMatch.Groups[2].Value : genMatch.Groups[3].Value;
                    normalized.AppendLine($"for {genMatch.Groups[1].Value} in {coll.Trim()} {{");
                    continue;
                }
            }
            if ((t.StartsWith("local function ") || t.StartsWith("function ")) && t.EndsWith(")"))
            {
                string fnPart = t.StartsWith("local ") ? t["local function ".Length..] : t["function ".Length..];
                int openP = fnPart.IndexOf('(');
                if (openP > 0)
                {
                    string fnName = fnPart[..openP].Trim();
                    string fnArgs = fnPart[(openP + 1)..^1].Trim();
                    if (fnName.Contains(':'))
                    {
                        var parts = fnName.Split(':', 2);
                        string argsWithSelf = string.IsNullOrEmpty(fnArgs) ? "self" : $"self, {fnArgs}";
                        normalized.AppendLine($"function __lua_method__{parts[0]}__{parts[1]}({argsWithSelf}) {{");
                        continue;
                    }
                    if (fnName.EndsWith(".new", StringComparison.Ordinal))
                    {
                        string clsName = fnName[..^4];
                        normalized.AppendLine($"function __lua_ctor__{clsName}({fnArgs}) {{");
                        continue;
                    }
                }
                normalized.AppendLine($"function {fnPart} {{");
                continue;
            }
            normalized.AppendLine(t);
        }

        var statements = ParseBraceDelimitedBlock(normalized.ToString(), "lua", out _);
        module.Statements.AddRange(PostProcessLuaStatements(statements));
    }

    private static List<PilotIrStmt> PostProcessLuaStatements(List<PilotIrStmt> stmts)
    {
        var result = new List<PilotIrStmt>();
        var luaClasses = new Dictionary<string, PilotIrClassDefStmt>(StringComparer.Ordinal);

        for (int i = 0; i < stmts.Count; i++)
        {
            var s = stmts[i];

            // Detect metatable class header: local Name = {} followed by Name.__index = Name
            if (s is PilotIrVarDeclStmt vd &&
                i + 1 < stmts.Count &&
                stmts[i + 1] is PilotIrAssignStmt { Target: PilotIrMemberAccessExpr { MemberName: "__index", Target: PilotIrIdentifierExpr idxId } } &&
                idxId.Name == vd.Name)
            {
                var cls = new PilotIrClassDefStmt { Line = vd.Line, Kind = PilotTypeDeclKind.Class, Name = vd.Name };
                luaClasses[vd.Name] = cls;
                result.Add(cls);
                i++; // skip Name.__index = Name
                if (i + 1 < stmts.Count && stmts[i + 1] is PilotIrFuncDefStmt ctorFn && ctorFn.Name == $"__lua_ctor__{vd.Name}")
                {
                    i++; // skip synthetic .new constructor
                }
                continue;
            }

            if (s is PilotIrFuncDefStmt mFn && mFn.Name.StartsWith("__lua_method__", StringComparison.Ordinal))
            {
                string rem = mFn.Name["__lua_method__".Length..];
                var parts = rem.Split("__", 2);
                if (parts.Length == 2 && luaClasses.TryGetValue(parts[0], out var ownerCls))
                {
                    mFn.Name = parts[1];
                    ownerCls.Methods.Add(mFn);
                    continue;
                }
            }

            result.Add(s);
        }
        return result;
    }

    // ========================================================================
    // SHARED STRUCTURED BRACE-BLOCK RECURSIVE-DESCENT PARSER (Rust, C, C++, JS, Lua)
    // ========================================================================

    private static List<PilotIrStmt> ParseBraceDelimitedBlock(
        string source,
        string lang,
        out List<PilotIrExtensionDefStmt> implBlocks)
    {
        implBlocks = new List<PilotIrExtensionDefStmt>();
        var items = SplitIntoBraceItems(source);
        var stmts = new List<PilotIrStmt>();

        foreach (var item in items)
        {
            string head = item.Header.Trim();
            if (string.IsNullOrWhiteSpace(head) && string.IsNullOrWhiteSpace(item.Body))
                continue;

            // Comments
            if (head.StartsWith("//"))
            {
                stmts.Add(new PilotIrCommentStmt { CommentText = head, Line = item.Line });
                continue;
            }

            // Skip preprocessor/module headers
            if (head.StartsWith("#include") || head.StartsWith("#![") || head.StartsWith("#[derive") ||
                head.StartsWith("using namespace") || head == "\"use strict\"" || head == "'use strict'" ||
                head is "public:" or "private:" or "protected:")
            {
                continue;
            }

            // Foreign imports
            if (head.StartsWith("use ") || head.StartsWith("import ") || head.Contains("require("))
            {
                string mapped = PilotLibraryMapper.TranslateForeignLibrariesToPilot(head, lang);
                if (mapped.StartsWith("//"))
                    stmts.Add(new PilotIrCommentStmt { CommentText = mapped, Line = item.Line });
                continue;
            }

            if (item.HasBraces)
            {
                // Skip Rust trait declarations (their companion impl blocks emit PilotIrExtensionDefStmt)
                if (lang == "rust" && (head.StartsWith("pub trait ") || head.StartsWith("trait ")))
                {
                    continue;
                }

                // Lua table / enum declaration: local Name = { ... }
                if (lang == "lua" && head.StartsWith("local ") && head.EndsWith("="))
                {
                    string tblName = head["local ".Length..^1].Replace("<const>", "").Trim();
                    string trimmedBody = item.Body.Trim();
                    if (string.IsNullOrWhiteSpace(trimmedBody))
                    {
                        stmts.Add(new PilotIrVarDeclStmt
                        {
                            Line = item.Line,
                            Kind = PilotVarDeclKind.Let,
                            Name = tblName,
                            Initializer = new PilotIrCollectionExpr { Kind = PilotCollectionKind.List }
                        });
                        continue;
                    }
                    var entries = SplitTopLevelComma(trimmedBody);
                    if (entries.All(e => e.Contains('=') && IsIdentifier(e.Split('=', 2)[0].Trim())) &&
                        char.IsUpper(tblName[0]))
                    {
                        stmts.Add(ParseForeignEnumOrAdt(tblName, trimmedBody, lang, item.Line));
                        continue;
                    }
                }

                // JS/TS prototype extension: Number.prototype.doubled = function(a) { ... }
                if (lang == "js" && head.Contains(".prototype.", StringComparison.Ordinal) && head.Contains("= function(", StringComparison.Ordinal))
                {
                    int protoIdx = head.IndexOf(".prototype.", StringComparison.Ordinal);
                    string protoTarget = head[..protoIdx].Trim() switch
                    {
                        "Number" => "int",
                        "String" => "str",
                        "Array" => "list",
                        var other => other
                    };
                    string afterProto = head[(protoIdx + ".prototype.".Length)..];
                    int eqFnIdx = afterProto.IndexOf("=", StringComparison.Ordinal);
                    string mName = afterProto[..eqFnIdx].Trim();
                    int openP = afterProto.IndexOf('(', eqFnIdx);
                    int closeP = FindMatchingCloseParen(afterProto, openP);
                    string rawArgs = (openP >= 0 && closeP > openP) ? afterProto[(openP + 1)..closeP] : "";

                    var extMethod = new PilotIrFuncDefStmt { Line = item.Line, Name = mName };
                    extMethod.Parameters.Add(new PilotIrParameter("self"));
                    foreach (var p in SplitTopLevelComma(rawArgs))
                    {
                        string pt = p.Trim();
                        if (string.IsNullOrEmpty(pt)) continue;
                        int colon = pt.IndexOf(':');
                        if (colon > 0)
                            extMethod.Parameters.Add(new PilotIrParameter(pt[..colon].Trim(), MapForeignType(pt[(colon + 1)..].Trim(), lang)));
                        else
                            extMethod.Parameters.Add(new PilotIrParameter(pt));
                    }
                    extMethod.Body.AddRange(ParseBraceDelimitedBlock(item.Body, lang, out _));

                    var existingExt = implBlocks.FirstOrDefault(e => e.TargetType == protoTarget);
                    if (existingExt == null)
                    {
                        existingExt = new PilotIrExtensionDefStmt { Line = item.Line, TargetType = protoTarget };
                        implBlocks.Add(existingExt);
                    }
                    existingExt.Methods.Add(extMethod);
                    continue;
                }

                // 1. Rust / C / C++ / TS enum -> Pilot ADT or Enum
                if (IsEnumHeader(head, out string enumName))
                {
                    stmts.Add(ParseForeignEnumOrAdt(enumName, item.Body, lang, item.Line));
                    continue;
                }

                // 2. Struct / Class
                if (IsStructOrClassHeader(head, out string typeName, out var declKind))
                {
                    stmts.Add(ParseForeignStructOrClass(typeName, declKind, item.Body, lang, item.Line));
                    continue;
                }

                // 3. Rust impl block -> PilotIrExtensionDefStmt
                if (head.StartsWith("impl ") || head.StartsWith("impl<"))
                {
                    string targetType = ExtractImplTargetType(head);
                    bool isTraitImpl = head.Contains(" for ", StringComparison.Ordinal);
                    var ext = new PilotIrExtensionDefStmt
                    {
                        Line = item.Line,
                        TargetType = targetType,
                        ExtensionName = isTraitImpl ? "trait" : null
                    };
                    var innerStmts = ParseBraceDelimitedBlock(item.Body, lang, out _);
                    foreach (var m in innerStmts.OfType<PilotIrFuncDefStmt>())
                        ext.Methods.Add(m);
                    if (ext.Methods.Count > 0)
                        implBlocks.Add(ext);
                    continue;
                }

                // 4. Function definition
                if (TryParseFunctionHeader(head, lang, item.Line, out var fnStmt))
                {
                    var bodyStmts = ParseBraceDelimitedBlock(item.Body, lang, out _);
                    // Rust implicit final return expression
                    if (lang == "rust" && bodyStmts.Count > 0 &&
                        !string.IsNullOrEmpty(fnStmt!.DeclaredReturnType) &&
                        fnStmt.DeclaredReturnType != "void" &&
                        bodyStmts[^1] is PilotIrExprStmt lastEs &&
                        lastEs.Expression is not PilotIrCallExpr { LibraryCallKind: PilotLibraryCallKind.Println or PilotLibraryCallKind.Print })
                    {
                        bodyStmts[^1] = new PilotIrReturnStmt { Line = lastEs.Line, Value = lastEs.Expression };
                    }
                    fnStmt!.Body.AddRange(bodyStmts);
                    stmts.Add(fnStmt);
                    continue;
                }

                // 5. Control flow blocks: if / elif / else / while / for / match / switch / try
                if (TryParseControlFlowBlock(head, item.Body, item.TrailingBranches, lang, item.Line, out var cfStmt))
                {
                    stmts.Add(cfStmt!);
                    continue;
                }
            }

            // Simple statement (including statements ending in brace-literal expressions like `return { ... }` or `std::vector<T>{...}`)
            string simpleInput = item.HasBraces
                ? (head.EndsWith(">", StringComparison.Ordinal) ? $"{head}{{{item.Body}}}" : $"{head} {{{item.Body}}}")
                : head;
            var simple = ParseForeignSimpleStmt(simpleInput, lang, item.Line);
            if (simple != null)
                stmts.Add(simple);
        }

        return stmts;
    }

    private sealed class BraceItem
    {
        public string Header { get; set; } = "";
        public string Body { get; set; } = "";
        public bool HasBraces { get; set; }
        public int Line { get; set; } = 1;
        public List<(string ClauseHeader, string ClauseBody)> TrailingBranches { get; } = new();
    }

    private static List<BraceItem> SplitIntoBraceItems(string source)
    {
        var items = new List<BraceItem>();
        var headerBuf = new StringBuilder();
        int i = 0;
        int line = 1;
        int itemStartLine = 1;

        while (i < source.Length)
        {
            // Skip leading whitespace
            while (i < source.Length && char.IsWhiteSpace(source[i]))
            {
                if (source[i] == '\n') line++;
                i++;
            }
            if (i >= source.Length) break;

            // Line comment
            if (i + 1 < source.Length && source[i] == '/' && source[i + 1] == '/')
            {
                int start = i;
                while (i < source.Length && source[i] != '\n') i++;
                if (headerBuf.Length == 0)
                {
                    items.Add(new BraceItem { Header = source[start..i].Trim(), Line = line });
                    continue;
                }
                continue;
            }

            // Block comment
            if (i + 1 < source.Length && source[i] == '/' && source[i + 1] == '*')
            {
                i += 2;
                while (i + 1 < source.Length && !(source[i] == '*' && source[i + 1] == '/'))
                {
                    if (source[i] == '\n') line++;
                    i++;
                }
                i += 2;
                continue;
            }

            itemStartLine = line;
            headerBuf.Clear();
            int parenDepth = 0;
            bool inStr = false;
            char strQuote = '"';

            while (i < source.Length)
            {
                char c = source[i];
                if (c == '\n') line++;

                if (inStr)
                {
                    headerBuf.Append(c);
                    if (c == '\\' && i + 1 < source.Length)
                    {
                        i++;
                        headerBuf.Append(source[i]);
                    }
                    else if (c == strQuote)
                    {
                        inStr = false;
                    }
                    i++;
                    continue;
                }

                if (c is '"' or '\'' or '`')
                {
                    // Check Rust lifetime 'a vs char literal 'a'
                    if (c == '\'' && (i + 2 >= source.Length || source[i + 2] != '\'') && (i + 1 < source.Length && source[i + 1] != '\\'))
                    {
                        headerBuf.Append(c);
                        i++;
                        continue;
                    }
                    inStr = true;
                    strQuote = c;
                    headerBuf.Append(c);
                    i++;
                    continue;
                }

                if (c == '(' || c == '[') parenDepth++;
                else if (c == ')' || c == ']') parenDepth = Math.Max(0, parenDepth - 1);

                if (parenDepth == 0 && c == ';' && !IsInsideCStyleForHeader(headerBuf.ToString()))
                {
                    i++;
                    items.Add(new BraceItem
                    {
                        Header = headerBuf.ToString().Trim(),
                        HasBraces = false,
                        Line = itemStartLine
                    });
                    headerBuf.Clear();
                    break;
                }

                if (parenDepth == 0 && c == '\n' && headerBuf.Length > 0)
                {
                    string h = headerBuf.ToString().Trim();
                    if (h.StartsWith("#include") || h.StartsWith("#![") || h.StartsWith("#[") ||
                        h is "public:" or "private:" or "protected:")
                    {
                        i++;
                        items.Add(new BraceItem { Header = h, HasBraces = false, Line = itemStartLine });
                        headerBuf.Clear();
                        break;
                    }
                    // For Lua single-line statements without semicolons
                    if (!h.EndsWith(",") && !h.EndsWith("=") && !h.EndsWith("+") && !h.EndsWith("-") &&
                        !h.StartsWith("if ") && !h.StartsWith("for ") && !h.StartsWith("while ") &&
                        !h.StartsWith("function ") && !h.StartsWith("pub ") && !h.StartsWith("fn ") &&
                        !h.StartsWith("struct ") && !h.StartsWith("enum ") && !h.StartsWith("class ") &&
                        !h.StartsWith("typedef ") && !h.StartsWith("trait ") && !h.StartsWith("impl"))
                    {
                        int peek = i + 1;
                        while (peek < source.Length && (source[peek] == ' ' || source[peek] == '\t' || source[peek] == '\r')) peek++;
                        if (peek < source.Length && source[peek] != '{')
                        {
                            i++;
                            items.Add(new BraceItem { Header = h, HasBraces = false, Line = itemStartLine });
                            headerBuf.Clear();
                            break;
                        }
                    }
                }

                if (parenDepth == 0 && c == '{')
                {
                    string headText = headerBuf.ToString().Trim();
                    string bodyText = ReadBalancedBraceBody(source, ref i, ref line);
                    // Swallow optional typedef alias and trailing semicolon after struct/class/enum brace
                    if (headText.StartsWith("typedef ", StringComparison.Ordinal))
                    {
                        while (i < source.Length && source[i] != ';' && source[i] != '\n') i++;
                        if (i < source.Length && source[i] == ';') i++;
                    }
                    else
                    {
                        while (i < source.Length && (source[i] == ' ' || source[i] == '\t')) i++;
                        if (i < source.Length && source[i] == ';') i++;
                    }

                    var bItem = new BraceItem
                    {
                        Header = headText,
                        Body = bodyText,
                        HasBraces = true,
                        Line = itemStartLine
                    };

                    // Check for trailing else if / elif / else / catch / finally
                    while (true)
                    {
                        int saveI = i;
                        int saveLine = line;
                        while (i < source.Length && char.IsWhiteSpace(source[i]))
                        {
                            if (source[i] == '\n') line++;
                            i++;
                        }
                        if (StartsWithBranchKeyword(source, i, out string branchHeader, out int afterHeaderIdx))
                        {
                            i = afterHeaderIdx;
                            if (i < source.Length && source[i] == '{')
                            {
                                string branchBody = ReadBalancedBraceBody(source, ref i, ref line);
                                bItem.TrailingBranches.Add((branchHeader.Trim(), branchBody));
                                continue;
                            }
                        }
                        i = saveI;
                        line = saveLine;
                        break;
                    }

                    items.Add(bItem);
                    headerBuf.Clear();
                    break;
                }

                headerBuf.Append(c);
                i++;
            }

            if (headerBuf.Length > 0)
            {
                string rem = headerBuf.ToString().Trim();
                if (!string.IsNullOrEmpty(rem))
                {
                    items.Add(new BraceItem { Header = rem, HasBraces = false, Line = itemStartLine });
                }
            }
        }

        return items;
    }

    private static bool IsInsideCStyleForHeader(string headerSoFar)
    {
        string t = headerSoFar.TrimStart();
        return t.StartsWith("for ") || t.StartsWith("for(");
    }

    private static string ReadBalancedBraceBody(string source, ref int i, ref int line)
    {
        if (i < source.Length && source[i] == '{')
            i++;

        int depth = 1;
        int start = i;
        bool inStr = false;
        char quote = '"';

        while (i < source.Length && depth > 0)
        {
            char c = source[i];
            if (c == '\n') line++;

            if (inStr)
            {
                if (c == '\\' && i + 1 < source.Length) i++;
                else if (c == quote) inStr = false;
                i++;
                continue;
            }

            if (c is '"' or '`')
            {
                inStr = true;
                quote = c;
            }
            else if (c == '{') depth++;
            else if (c == '}') depth--;

            i++;
        }

        int end = depth == 0 ? i - 1 : i;
        return end > start ? source[start..end] : "";
    }

    private static bool StartsWithBranchKeyword(string source, int idx, out string branchHeader, out int openBraceIdx)
    {
        branchHeader = "";
        openBraceIdx = idx;
        string rem = source[idx..];
        if (!rem.StartsWith("else") && !rem.StartsWith("elif ") && !rem.StartsWith("catch") && !rem.StartsWith("finally"))
            return false;

        int i = idx;
        int parenDepth = 0;
        while (i < source.Length)
        {
            char c = source[i];
            if (c == '(') parenDepth++;
            else if (c == ')') parenDepth = Math.Max(0, parenDepth - 1);
            else if (c == '{' && parenDepth == 0)
            {
                branchHeader = source[idx..i].Trim();
                openBraceIdx = i;
                return true;
            }
            else if (c == ';' && parenDepth == 0)
            {
                return false;
            }
            i++;
        }
        return false;
    }

    private static bool IsEnumHeader(string head, out string enumName)
    {
        enumName = "";
        var m = EnumHeaderRegex().Match(head);
        if (m.Success)
        {
            enumName = m.Groups[1].Value;
            return true;
        }
        return false;
    }

    private static PilotIrStmt ParseForeignEnumOrAdt(string name, string body, string lang, int line)
    {
        var rawItems = SplitTopLevelComma(body);
        bool hasPayloads = rawItems.Any(x => x.Contains('(') || x.Contains('{'));

        if (hasPayloads)
        {
            var adt = new PilotIrAdtDefStmt { Line = line, Name = name };
            foreach (var raw in rawItems)
            {
                string v = raw.Trim();
                if (string.IsNullOrEmpty(v) || v.StartsWith("//")) continue;

                int openBrace = v.IndexOf('{');
                int openParen = v.IndexOf('(');
                if (openBrace > 0 && v.EndsWith("}"))
                {
                    string vName = v[..openBrace].Trim();
                    string inner = v[(openBrace + 1)..^1];
                    var fields = new List<(string Name, string Type)>();
                    foreach (var f in SplitTopLevelComma(inner))
                    {
                        var kv = f.Split(':');
                        if (kv.Length == 2)
                            fields.Add((kv[0].Trim(), MapForeignType(kv[1].Trim(), lang)));
                    }
                    adt.Variants.Add(new PilotIrAdtVariant(vName, fields));
                }
                else if (openParen > 0 && v.EndsWith(")"))
                {
                    string vName = v[..openParen].Trim();
                    string inner = v[(openParen + 1)..^1];
                    var fields = new List<(string Name, string Type)>();
                    int fIdx = 0;
                    foreach (var f in SplitTopLevelComma(inner))
                    {
                        string ft = f.Trim();
                        if (string.IsNullOrEmpty(ft)) continue;
                        if (ft.Contains(':'))
                        {
                            var kv = ft.Split(':', 2);
                            fields.Add((kv[0].Trim(), MapForeignType(kv[1].Trim(), lang)));
                        }
                        else
                        {
                            fields.Add(($"v{fIdx++}", MapForeignType(ft, lang)));
                        }
                    }
                    adt.Variants.Add(new PilotIrAdtVariant(vName, fields));
                }
                else
                {
                    adt.Variants.Add(new PilotIrAdtVariant(v, Array.Empty<(string, string)>()));
                }
            }
            return adt;
        }

        var en = new PilotIrEnumDefStmt { Line = line, Name = name };
        foreach (var raw in rawItems)
        {
            string m = raw.Trim();
            if (string.IsNullOrEmpty(m) || m.StartsWith("//")) continue;
            int eq = m.IndexOf('=');
            string memberName = eq > 0 ? m[..eq].Trim() : m;
            if (lang == "c" && memberName.StartsWith(name + "_", StringComparison.Ordinal))
                memberName = memberName[(name.Length + 1)..];
            if (eq > 0)
                en.Members.Add((memberName, ParseForeignExpr(m[(eq + 1)..].Trim(), lang)));
            else
                en.Members.Add((memberName, null));
        }
        return en;
    }

    private static bool IsStructOrClassHeader(string head, out string name, out PilotTypeDeclKind kind)
    {
        name = "";
        kind = PilotTypeDeclKind.Class;
        var m = StructClassHeaderRegex().Match(head);
        if (m.Success)
        {
            kind = m.Groups[1].Value == "struct" ? PilotTypeDeclKind.Struct : PilotTypeDeclKind.Class;
            name = m.Groups[2].Value;
            return true;
        }
        return false;
    }

    private static PilotIrClassDefStmt ParseForeignStructOrClass(string name, PilotTypeDeclKind kind, string body, string lang, int line)
    {
        var cls = new PilotIrClassDefStmt { Line = line, Kind = kind, Name = name };
        if (lang == "rust")
        {
            foreach (var f in SplitTopLevelComma(body))
            {
                string ft = f.Trim().TrimEnd(';');
                if (string.IsNullOrEmpty(ft) || ft.StartsWith("//")) continue;
                if (ft.StartsWith("pub ")) ft = ft["pub ".Length..].Trim();
                int colon = ft.IndexOf(':');
                if (colon > 0)
                {
                    cls.Fields.Add(new PilotIrFieldDecl(
                        ft[..colon].Trim(),
                        MapForeignType(ft[(colon + 1)..].Trim(), lang)));
                }
            }
            return cls;
        }

        // C tagged-union ADT payload extraction: union { struct { double r; } Circle; ... } data;
        if (lang == "c" && body.Contains("union {", StringComparison.Ordinal))
        {
            foreach (Match um in CUnionVariantStructRegex().Matches(body))
            {
                string fieldsBlob = um.Groups[1].Value;
                string vName = um.Groups[2].Value.Trim();
                foreach (var fDecl in fieldsBlob.Split(';', StringSplitOptions.RemoveEmptyEntries))
                {
                    var parts = fDecl.Trim().Split(' ', StringSplitOptions.RemoveEmptyEntries);
                    if (parts.Length >= 2)
                    {
                        string fName = parts[^1].Trim();
                        string fType = MapForeignType(string.Join(" ", parts[..^1]), lang);
                        cls.Fields.Add(new PilotIrFieldDecl($"{vName}.{fName}", fType));
                    }
                }
            }
            return cls;
        }

        if (lang == "js")
        {
            foreach (var bItem in SplitIntoBraceItems(body))
            {
                string h = bItem.Header.Trim();
                if (string.IsNullOrWhiteSpace(h) || h.StartsWith("//")) continue;
                if (bItem.HasBraces)
                {
                    var mm = JsClassMethodRegex().Match(h);
                    if (mm.Success)
                    {
                        bool isStatic = mm.Groups[1].Success;
                        bool isAsync = mm.Groups[2].Success;
                        string mName = mm.Groups[3].Value;
                        string rawParams = mm.Groups[4].Value;
                        string rawRet = mm.Groups[5].Value.Trim();

                        if (mName == "constructor")
                        {
                            var ctorBody = ParseBraceDelimitedBlock(bItem.Body, lang, out _);
                            foreach (var cs in ctorBody.OfType<PilotIrAssignStmt>())
                            {
                                if (cs.Target is PilotIrMemberAccessExpr { Target: PilotIrIdentifierExpr { Name: "self" or "this" } } mem &&
                                    !cls.Fields.Any(f => f.Name == mem.MemberName))
                                {
                                    cls.Fields.Add(new PilotIrFieldDecl(mem.MemberName, "any"));
                                }
                            }
                            continue;
                        }

                        var mFn = new PilotIrFuncDefStmt
                        {
                            Line = bItem.Line,
                            Name = mName,
                            IsStatic = isStatic,
                            IsAsync = isAsync,
                            DeclaredReturnType = !string.IsNullOrEmpty(rawRet) && rawRet != "void" ? MapForeignType(rawRet, lang) : null
                        };
                        if (!isStatic)
                            mFn.Parameters.Add(new PilotIrParameter("self"));
                        foreach (var p in SplitTopLevelComma(rawParams))
                        {
                            string pt = p.Trim();
                            if (string.IsNullOrEmpty(pt)) continue;
                            int colon = pt.IndexOf(':');
                            if (colon > 0)
                                mFn.Parameters.Add(new PilotIrParameter(pt[..colon].Trim(), MapForeignType(pt[(colon + 1)..].Trim(), lang)));
                            else
                                mFn.Parameters.Add(new PilotIrParameter(pt));
                        }
                        mFn.Body.AddRange(ParseBraceDelimitedBlock(bItem.Body, lang, out _));
                        cls.Methods.Add(mFn);
                    }
                }
                else
                {
                    // TS class field declaration: count: number
                    int colon = FindTopLevelChar(h, ':');
                    if (colon > 0 && IsIdentifier(h[..colon].Trim()))
                    {
                        string fName = h[..colon].Trim();
                        string fType = MapForeignType(h[(colon + 1)..].Trim(), lang);
                        if (!cls.Fields.Any(f => f.Name == fName))
                            cls.Fields.Add(new PilotIrFieldDecl(fName, fType));
                    }
                }
            }
            return cls;
        }

        var members = ParseBraceDelimitedBlock(body, lang, out _);
        foreach (var m in members)
        {
            if (m is PilotIrFuncDefStmt fn)
            {
                if (!fn.IsStatic && (fn.Parameters.Count == 0 || fn.Parameters[0].Name != "self"))
                    fn.Parameters.Insert(0, new PilotIrParameter("self"));
                cls.Methods.Add(fn);
            }
            else if (m is PilotIrVarDeclStmt vd && vd.Name != "_reserved")
            {
                cls.Fields.Add(new PilotIrFieldDecl(vd.Name, vd.DeclaredType ?? "any", vd.Initializer));
            }
        }
        return cls;
    }

    private static string ExtractImplTargetType(string head)
    {
        string s = head["impl".Length..].Trim();
        if (s.Contains(" for "))
            s = s.Split(" for ").Last().Trim();
        return MapForeignType(s, "rust");
    }

    private static bool TryParseFunctionHeader(string head, string lang, int line, out PilotIrFuncDefStmt? fn)
    {
        fn = null;
        string h = head.Trim();

        // Rust: pub fn foo(a: i64, b: i64) -> i64
        if (lang == "rust")
        {
            var m = RustFnHeaderRegex().Match(h);
            if (!m.Success) return false;
            string fnName = m.Groups[1].Value;
            string rawParams = m.Groups[2].Value;
            string rawRet = m.Groups[3].Value.Trim();

            fn = new PilotIrFuncDefStmt
            {
                Line = line,
                Name = fnName,
                DeclaredReturnType = !string.IsNullOrEmpty(rawRet) ? MapForeignType(rawRet, "rust") : null
            };

            foreach (var p in SplitTopLevelComma(rawParams))
            {
                string pt = p.Trim();
                if (string.IsNullOrEmpty(pt)) continue;
                if (pt is "&self" or "&mut self" or "self" or "mut self")
                {
                    fn.Parameters.Add(new PilotIrParameter("self"));
                    continue;
                }
                if (pt.StartsWith("mut ")) pt = pt["mut ".Length..].Trim();
                int colon = pt.IndexOf(':');
                if (colon > 0)
                {
                    fn.Parameters.Add(new PilotIrParameter(
                        pt[..colon].Trim(),
                        MapForeignType(pt[(colon + 1)..].Trim(), "rust")));
                }
                else
                {
                    fn.Parameters.Add(new PilotIrParameter(pt));
                }
            }
            return true;
        }

        // JS / TS / Lua: [export] [async] function foo(a, b)[: RetType]
        if (lang is "js" or "lua")
        {
            var m = JsFnHeaderRegex().Match(h);
            if (!m.Success) return false;
            bool isAsync = m.Groups[1].Success;
            string fnName = m.Groups[2].Value;
            string rawParams = m.Groups[3].Value;
            string rawRet = m.Groups[4].Value.Trim();

            fn = new PilotIrFuncDefStmt
            {
                Line = line,
                Name = fnName,
                IsAsync = isAsync,
                DeclaredReturnType = !string.IsNullOrEmpty(rawRet) && rawRet != "void" ? MapForeignType(rawRet, lang) : null
            };
            foreach (var p in SplitTopLevelComma(rawParams))
            {
                string pt = p.Trim();
                if (string.IsNullOrEmpty(pt)) continue;
                int eq = FindTopLevelChar(pt, '=');
                PilotIrExpr? defVal = null;
                if (eq > 0)
                {
                    defVal = ParseForeignExpr(pt[(eq + 1)..].Trim(), lang);
                    pt = pt[..eq].Trim();
                }
                int colon = pt.IndexOf(':');
                if (colon > 0)
                    fn.Parameters.Add(new PilotIrParameter(pt[..colon].Trim(), MapForeignType(pt[(colon + 1)..].Trim(), lang), defVal));
                else
                    fn.Parameters.Add(new PilotIrParameter(pt, null, defVal));
            }
            return true;
        }

        // C / C++: [static] [inline] [constexpr] RetType fnName(ParamType a, ...)
        if (lang is "c" or "cpp")
        {
            if (h.StartsWith("if ") || h.StartsWith("if(") || h.StartsWith("for ") || h.StartsWith("for(") ||
                h.StartsWith("while ") || h.StartsWith("while(") || h.StartsWith("switch"))
                return false;

            var m = CFnHeaderRegex().Match(h);
            if (!m.Success) return false;
            string retType = MapForeignType(m.Groups[1].Value.Trim(), lang);
            string fnName = m.Groups[2].Value.Trim();
            string rawParams = m.Groups[3].Value.Trim();

            fn = new PilotIrFuncDefStmt
            {
                Line = line,
                Name = fnName,
                DeclaredReturnType = (retType == "void" || retType == "auto" || fnName == "main") ? (fnName == "main" ? "i32" : null) : retType
            };

            if (!string.IsNullOrEmpty(rawParams) && rawParams != "void")
            {
                foreach (var p in SplitTopLevelComma(rawParams))
                {
                    var parts = p.Trim().Split(' ', StringSplitOptions.RemoveEmptyEntries);
                    if (parts.Length >= 2)
                    {
                        string pName = parts[^1].TrimStart('*', '&');
                        string pTypeRaw = string.Join(" ", parts[..^1]).Replace("const", "").Replace("&", "").Trim();
                        fn.Parameters.Add(new PilotIrParameter(pName, MapForeignType(pTypeRaw, lang)));
                    }
                }
            }
            return true;
        }

        return false;
    }

    private static bool TryParseControlFlowBlock(
        string head,
        string body,
        List<(string ClauseHeader, string ClauseBody)> trailing,
        string lang,
        int line,
        out PilotIrStmt? stmt)
    {
        stmt = null;
        string h = head.Trim();

        // 1. if / else if / else
        if (h.StartsWith("if ") || h.StartsWith("if("))
        {
            string condStr = UnwrapOuterParens(h["if".Length..].Trim());
            string? optVar = null;
            if (lang == "rust" && condStr.StartsWith("let Some("))
            {
                int closeP = condStr.IndexOf(')');
                int eq = condStr.IndexOf('=');
                if (closeP > "let Some(".Length && eq > closeP)
                {
                    optVar = condStr["let Some(".Length..closeP].Trim();
                    condStr = condStr[(eq + 1)..].Trim();
                }
            }

            var ifStmt = new PilotIrIfStmt
            {
                Line = line,
                OptionalBindingVar = optVar,
                Condition = ParseForeignExpr(condStr, lang)
            };
            ifStmt.ThenBody.AddRange(ParseBraceDelimitedBlock(body, lang, out _));

            foreach (var (ch, cb) in trailing)
            {
                if (ch.StartsWith("else if") || ch.StartsWith("elif"))
                {
                    string cCond = ch.StartsWith("else if")
                        ? UnwrapOuterParens(ch["else if".Length..].Trim())
                        : UnwrapOuterParens(ch["elif".Length..].Trim());
                    ifStmt.ElifBranches.Add((ParseForeignExpr(cCond, lang), ParseBraceDelimitedBlock(cb, lang, out _)));
                }
                else if (ch == "else")
                {
                    ifStmt.ElseBody.AddRange(ParseBraceDelimitedBlock(cb, lang, out _));
                }
            }
            stmt = ifStmt;
            return true;
        }

        // 2. while
        if (h.StartsWith("while ") || h.StartsWith("while("))
        {
            string condStr = UnwrapOuterParens(h["while".Length..].Trim());
            var ws = new PilotIrWhileStmt
            {
                Line = line,
                Condition = ParseForeignExpr(condStr, lang)
            };
            ws.Body.AddRange(ParseBraceDelimitedBlock(body, lang, out _));
            stmt = ws;
            return true;
        }

        // 3. for
        if (h.StartsWith("for ") || h.StartsWith("for("))
        {
            string inner = UnwrapOuterParens(h["for".Length..].Trim());

            // C-style numeric for: int i = a; i <= b; ++i
            var cFor = CStyleForRegex().Match(inner);
            if (cFor.Success)
            {
                string vName = cFor.Groups[1].Value;
                string startStr = cFor.Groups[2].Value.Trim();
                string op = cFor.Groups[3].Value;
                string endStr = cFor.Groups[4].Value.Trim();
                bool inclusive = op == "<=";
                if (!inclusive && int.TryParse(endStr, out int endNum))
                {
                    endStr = (endNum - 1).ToString();
                    inclusive = true;
                }
                var fs = new PilotIrForStmt
                {
                    Line = line,
                    Iterable = new PilotIrRangeExpr
                    {
                        Start = ParseForeignExpr(startStr, lang),
                        End = ParseForeignExpr(endStr, lang),
                        IsInclusive = inclusive
                    }
                };
                fs.TargetVars.Add(vName);
                fs.Body.AddRange(ParseBraceDelimitedBlock(body, lang, out _));
                stmt = fs;
                return true;
            }

            // C++ range-for: const auto& x : items
            var cppFor = CppRangeForRegex().Match(inner);
            if (cppFor.Success)
            {
                var fs = new PilotIrForStmt
                {
                    Line = line,
                    Iterable = ParseForeignExpr(cppFor.Groups[2].Value.Trim(), lang)
                };
                fs.TargetVars.Add(cppFor.Groups[1].Value.Trim());
                fs.Body.AddRange(ParseBraceDelimitedBlock(body, lang, out _));
                stmt = fs;
                return true;
            }

            // JS for (const x of items) or Rust/Lua for x in items
            var inOrOf = ForInOrOfRegex().Match(inner);
            if (inOrOf.Success)
            {
                string rawIter = inOrOf.Groups[2].Value.Trim();
                PilotIrExpr iterExpr;
                if (rawIter.Contains("..="))
                {
                    var parts = rawIter.Split("..=", 2);
                    iterExpr = new PilotIrRangeExpr
                    {
                        Start = ParseForeignExpr(parts[0].Trim(), lang),
                        End = ParseForeignExpr(parts[1].Trim(), lang),
                        IsInclusive = true
                    };
                }
                else if (rawIter.Contains("..<"))
                {
                    var parts = rawIter.Split("..<", 2);
                    iterExpr = new PilotIrRangeExpr
                    {
                        Start = ParseForeignExpr(parts[0].Trim(), lang),
                        End = ParseForeignExpr(parts[1].Trim(), lang),
                        IsInclusive = false
                    };
                }
                else if (rawIter.Contains(".."))
                {
                    var parts = rawIter.Split("..", 2);
                    iterExpr = new PilotIrRangeExpr
                    {
                        Start = ParseForeignExpr(parts[0].Trim(), lang),
                        End = ParseForeignExpr(parts[1].Trim(), lang),
                        IsInclusive = lang != "rust"
                    };
                }
                else
                {
                    iterExpr = ParseForeignExpr(rawIter, lang);
                }

                var fs = new PilotIrForStmt
                {
                    Line = line,
                    Iterable = iterExpr
                };
                fs.TargetVars.Add(inOrOf.Groups[1].Value.Trim());
                fs.Body.AddRange(ParseBraceDelimitedBlock(body, lang, out _));
                stmt = fs;
                return true;
            }
        }

        // 4. Rust match / JS & C switch
        if (h.StartsWith("match ") || h.StartsWith("switch ") || h.StartsWith("switch("))
        {
            string scrutStr = h.StartsWith("match ")
                ? h["match ".Length..].Trim()
                : UnwrapOuterParens(h["switch".Length..].Trim());
            var ms = new PilotIrMatchStmt
            {
                Line = line,
                Scrutinee = ParseForeignExpr(scrutStr, lang)
            };
            ParseMatchArms(body, lang, ms.Arms);
            stmt = ms;
            return true;
        }

        // 5. try / catch / finally
        if (h == "try")
        {
            var tr = new PilotIrTryStmt { Line = line };
            tr.TryBody.AddRange(ParseBraceDelimitedBlock(body, lang, out _));
            foreach (var (ch, cb) in trailing)
            {
                if (ch.StartsWith("catch"))
                {
                    string v = UnwrapOuterParens(ch["catch".Length..].Trim());
                    tr.Catches.Add(new PilotIrCatchClause(null, string.IsNullOrEmpty(v) ? "err" : v, ParseBraceDelimitedBlock(cb, lang, out _)));
                }
                else if (ch == "finally")
                {
                    tr.FinallyBody.AddRange(ParseBraceDelimitedBlock(cb, lang, out _));
                }
            }
            stmt = tr;
            return true;
        }

        return false;
    }

    private static void ParseMatchArms(string body, string lang, List<PilotIrMatchArm> arms)
    {
        var lines = body.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.RemoveEmptyEntries);
        foreach (var raw in lines)
        {
            string t = raw.Trim().TrimEnd(',');
            if (string.IsNullOrEmpty(t) || t.StartsWith("//")) continue;
            int arrow = t.IndexOf("=>", StringComparison.Ordinal);
            if (arrow > 0)
            {
                string patPart = t[..arrow].Trim();
                string rhsPart = t[(arrow + 2)..].Trim();
                PilotIrExpr? guard = null;
                int ifIdx = patPart.IndexOf(" if ", StringComparison.Ordinal);
                if (ifIdx > 0)
                {
                    guard = ParseForeignExpr(patPart[(ifIdx + 4)..].Trim(), lang);
                    patPart = patPart[..ifIdx].Trim();
                }

                var pat = ParseForeignPattern(patPart);
                if (rhsPart.StartsWith("{") && rhsPart.EndsWith("}"))
                {
                    var armStmts = ParseBraceDelimitedBlock(rhsPart[1..^1], lang, out _);
                    arms.Add(new PilotIrMatchArm(pat, guard, armStmts));
                }
                else
                {
                    var expr = ParseForeignExpr(rhsPart, lang);
                    arms.Add(new PilotIrMatchArm(pat, guard, new List<PilotIrStmt> { new PilotIrExprStmt { Expression = expr } }, expr));
                }
            }
        }
    }

    private static PilotIrStmt? ParseForeignSimpleStmt(string rawLine, string lang, int line)
    {
        string s = PilotLibraryMapper.TranslateForeignLibrariesToPilot(rawLine.Trim().TrimEnd(';'), lang);
        if (string.IsNullOrWhiteSpace(s))
            return null;

        if (s == "break") return new PilotIrBreakStmt { Line = line };
        if (s == "continue") return new PilotIrContinueStmt { Line = line };

        if (s == "return" || s.StartsWith("return "))
        {
            string val = s.Length > 6 ? s["return".Length..].Trim() : "";
            return new PilotIrReturnStmt
            {
                Line = line,
                Value = !string.IsNullOrEmpty(val) ? ParseForeignExpr(val, lang) : null
            };
        }

        if (s.StartsWith("throw "))
        {
            string val = s["throw ".Length..].Trim();
            if (val.StartsWith("new Error(") && val.EndsWith(")"))
                val = val["new Error(".Length..^1];
            return new PilotIrRaiseStmt
            {
                Line = line,
                Value = ParseForeignExpr(val, lang)
            };
        }

        if (s.StartsWith("assert!(") && s.EndsWith(")"))
        {
            return new PilotIrAssertStmt
            {
                Line = line,
                Condition = ParseForeignExpr(s["assert!(".Length..^1], lang)
            };
        }

        if (s.StartsWith("assert(") && s.EndsWith(")"))
        {
            var assertParts = SplitTopLevelComma(s["assert(".Length..^1]);
            return new PilotIrAssertStmt
            {
                Line = line,
                Condition = ParseForeignExpr(assertParts[0], lang),
                Message = assertParts.Count > 1 ? ParseForeignExpr(assertParts[1], lang) : null
            };
        }

        // JS single-line assertion: if (!(cond)) throw new Error(...)
        if (lang == "js" && s.StartsWith("if (!(", StringComparison.Ordinal) && s.Contains("throw new Error(", StringComparison.Ordinal))
        {
            int throwIdx = s.IndexOf("throw new Error(", StringComparison.Ordinal);
            string condPart = s["if (!(".Length..throwIdx].Trim();
            if (condPart.EndsWith("))", StringComparison.Ordinal))
                condPart = condPart[..^2].Trim();
            string msgPart = s[(throwIdx + "throw new Error(".Length)..].TrimEnd(')').Trim();
            PilotIrExpr? msgExpr = (!string.IsNullOrEmpty(msgPart) && msgPart != "\"Assertion failed\"")
                ? ParseForeignExpr(msgPart, lang)
                : null;
            return new PilotIrAssertStmt
            {
                Line = line,
                Condition = ParseForeignExpr(condPart, lang),
                Message = msgExpr
            };
        }

        // JS Object.freeze enum: const Priority = Object.freeze({ Low: 1, ... })
        if (lang == "js" && s.StartsWith("const ", StringComparison.Ordinal) && s.Contains("Object.freeze({", StringComparison.Ordinal) && s.EndsWith("})", StringComparison.Ordinal))
        {
            int eqFreeze = s.IndexOf('=');
            string enumName = s["const ".Length..eqFreeze].Trim();
            int braceOpen = s.IndexOf('{', eqFreeze);
            string innerObj = s[(braceOpen + 1)..^2];
            var enStmt = new PilotIrEnumDefStmt { Line = line, Name = enumName };
            foreach (var entry in SplitTopLevelComma(innerObj))
            {
                var kv = entry.Split(':', 2);
                if (kv.Length == 2)
                    enStmt.Members.Add((kv[0].Trim(), ParseForeignExpr(kv[1].Trim(), lang)));
            }
            return enStmt;
        }

        // Type aliases:
        // Rust / TS: [pub] type Alias = Target
        if ((lang is "rust" or "js") && (s.StartsWith("pub type ", StringComparison.Ordinal) || s.StartsWith("type ", StringComparison.Ordinal)))
        {
            string restType = s.StartsWith("pub ") ? s["pub type ".Length..].Trim() : s["type ".Length..].Trim();
            int eqT = FindTopLevelSingleEquals(restType);
            if (eqT > 0)
            {
                string aliasName = restType[..eqT].Trim();
                string targetRaw = restType[(eqT + 1)..].Trim();
                return new PilotIrTypeAliasStmt
                {
                    Line = line,
                    Name = aliasName,
                    TargetType = targetRaw.Contains("ReturnType<typeof ", StringComparison.Ordinal)
                        ? targetRaw
                        : MapForeignType(targetRaw, lang)
                };
            }
        }

        // C++: using Alias = Target
        if (lang == "cpp" && s.StartsWith("using ", StringComparison.Ordinal) && !s.StartsWith("using namespace", StringComparison.Ordinal))
        {
            string restUsing = s["using ".Length..].Trim();
            int eqU = FindTopLevelSingleEquals(restUsing);
            if (eqU > 0)
            {
                string aliasName = restUsing[..eqU].Trim();
                string targetRaw = restUsing[(eqU + 1)..].Trim();
                return new PilotIrTypeAliasStmt
                {
                    Line = line,
                    Name = aliasName,
                    TargetType = targetRaw.StartsWith("std::variant<", StringComparison.Ordinal)
                        ? targetRaw
                        : MapForeignType(targetRaw, lang)
                };
            }
        }

        // C: typedef Target Alias
        if (lang == "c" && s.StartsWith("typedef ", StringComparison.Ordinal))
        {
            var tdParts = s["typedef ".Length..].Trim().Split(' ', StringSplitOptions.RemoveEmptyEntries);
            if (tdParts.Length >= 2)
            {
                string aliasName = tdParts[^1].Trim();
                string targetRaw = string.Join(" ", tdParts[..^1]);
                return new PilotIrTypeAliasStmt
                {
                    Line = line,
                    Name = aliasName,
                    TargetType = MapForeignType(targetRaw, lang)
                };
            }
        }

        // Variable declarations:
        // Rust: let mut x: i64 = 10 / let x = 10 / const X: i64 = 10
        // JS: const x = 10 / let x = 10 / var x = 10
        // Lua: local x = 10
        // C/C++: const int64_t x = 10 / int64_t x = 10 / auto x = 10
        if (TryParseVariableDeclaration(s, lang, line, out var varDecl))
        {
            return varDecl;
        }

        // Augmented assignments
        foreach (var aug in new[] { "+=", "-=", "*=", "/=", "%=" })
        {
            int augIdx = FindTopLevelOperator(s, aug);
            if (augIdx > 0)
            {
                return new PilotIrAssignStmt
                {
                    Line = line,
                    Target = ParseForeignExpr(s[..augIdx].Trim(), lang),
                    Operator = aug,
                    Value = ParseForeignExpr(s[(augIdx + aug.Length)..].Trim(), lang)
                };
            }
        }

        // Standard assignment
        int eqIdx = FindTopLevelSingleEquals(s);
        if (eqIdx > 0)
        {
            return new PilotIrAssignStmt
            {
                Line = line,
                Target = ParseForeignExpr(s[..eqIdx].Trim(), lang),
                Operator = "=",
                Value = ParseForeignExpr(s[(eqIdx + 1)..].Trim(), lang)
            };
        }

        return new PilotIrExprStmt
        {
            Line = line,
            Expression = ParseForeignExpr(s, lang)
        };
    }

    private static bool TryParseVariableDeclaration(string s, string lang, int line, out PilotIrVarDeclStmt? decl)
    {
        decl = null;

        // C / C++ typed variable and field declarations: [constexpr|const] int64_t x = 10  OR  double r  OR  int64_t val{}
        if (lang is "c" or "cpp")
        {
            var m = CVarDeclRegex().Match(s);
            if (m.Success)
            {
                string constPrefix = m.Groups[1].Value.Trim();
                bool hasConst = !string.IsNullOrEmpty(constPrefix);
                string rawType = m.Groups[2].Value.Trim();
                string vName = m.Groups[3].Value.Trim();
                string rhs = m.Groups[4].Value.Trim();
                string? mappedType = rawType == "auto" ? null : MapForeignType(rawType, lang);

                var cKind = !hasConst
                    ? PilotVarDeclKind.Var
                    : (constPrefix == "constexpr" || vName == vName.ToUpperInvariant())
                        ? PilotVarDeclKind.Const
                        : PilotVarDeclKind.Let;

                decl = new PilotIrVarDeclStmt
                {
                    Line = line,
                    Kind = cKind,
                    Name = vName,
                    DeclaredType = mappedType,
                    Initializer = ParseForeignExpr(rhs, lang),
                    IsMutated = cKind == PilotVarDeclKind.Var
                };
                return true;
            }

            var fMatch = CFieldNoInitRegex().Match(s);
            if (fMatch.Success)
            {
                string rawType = fMatch.Groups[1].Value.Trim();
                string vName = fMatch.Groups[2].Value.Trim();
                decl = new PilotIrVarDeclStmt
                {
                    Line = line,
                    Kind = PilotVarDeclKind.Let,
                    Name = vName,
                    DeclaredType = rawType == "auto" ? null : MapForeignType(rawType, lang)
                };
                return true;
            }
        }

        if (s.StartsWith("let mut ") || s.StartsWith("let ") || s.StartsWith("var ") || s.StartsWith("const ") || s.StartsWith("local "))
        {
            var kind = PilotVarDeclKind.Let;
            string rest;
            if (s.StartsWith("let mut "))
            {
                kind = PilotVarDeclKind.Var;
                rest = s["let mut ".Length..].Trim();
            }
            else if (s.StartsWith("var "))
            {
                kind = PilotVarDeclKind.Var;
                rest = s["var ".Length..].Trim();
            }
            else if (s.StartsWith("const ") && !IsCTypeKeywordAfterConst(s))
            {
                kind = PilotVarDeclKind.Const;
                rest = s["const ".Length..].Trim();
            }
            else if (s.StartsWith("local "))
            {
                rest = s["local ".Length..].Replace("<const>", "").Trim();
                if (s.Contains("<const>")) kind = PilotVarDeclKind.Const;
            }
            else if (s.StartsWith("let "))
            {
                rest = s["let ".Length..].Trim();
            }
            else
            {
                return false;
            }

            int eq = FindTopLevelSingleEquals(rest);
            string lhs = eq > 0 ? rest[..eq].Trim() : rest;
            PilotIrExpr? init = eq > 0 ? ParseForeignExpr(rest[(eq + 1)..].Trim(), lang) : null;

            string? declType = null;
            int colon = FindTopLevelChar(lhs, ':');
            if (colon > 0)
            {
                declType = MapForeignType(lhs[(colon + 1)..].Trim(), lang);
                lhs = lhs[..colon].Trim();
            }

            decl = new PilotIrVarDeclStmt
            {
                Line = line,
                Kind = kind,
                Name = lhs,
                DeclaredType = declType,
                Initializer = init,
                IsMutated = kind == PilotVarDeclKind.Var
            };
            return true;
        }

        return false;
    }

    private static bool IsCTypeKeywordAfterConst(string s)
    {
        string after = s["const ".Length..].TrimStart();
        return after.StartsWith("int") || after.StartsWith("uint") || after.StartsWith("double") ||
               after.StartsWith("float") || after.StartsWith("bool") || after.StartsWith("char") ||
               after.StartsWith("auto ") || after.StartsWith("std::");
    }

    // ========================================================================
    // STRUCTURED FOREIGN EXPRESSION PARSER -> PilotIrExpr
    // ========================================================================

    public static PilotIrExpr ParseForeignExpr(string rawExpr, string lang)
    {
        string s = rawExpr.Trim();
        if (string.IsNullOrEmpty(s))
            return new PilotIrLiteralExpr { Kind = PilotLiteralKind.Null, RawValue = "null" };

        s = UnwrapOuterParens(s);

        // 1. Language-specific print / format macros and streams
        if (lang == "rust")
        {
            if ((s.StartsWith("println!(") || s.StartsWith("print!(") || s.StartsWith("format!(")) && s.EndsWith(")"))
            {
                bool isPrintln = s.StartsWith("println!");
                bool isPrint = s.StartsWith("print!");
                int openP = s.IndexOf('(');
                string inner = s[(openP + 1)..^1];
                var fExpr = ConvertFormatArgsToInterpolated(inner, lang);
                if (!isPrintln && !isPrint)
                    return fExpr;
                var pCall = new PilotIrCallExpr
                {
                    Callee = new PilotIrIdentifierExpr { Name = isPrintln ? "println" : "print" },
                    LibraryCallKind = isPrintln ? PilotLibraryCallKind.Println : PilotLibraryCallKind.Print
                };
                pCall.Arguments.Add(new PilotIrCallArg(null, fExpr));
                return pCall;
            }

            if (s.StartsWith("vec![") && s.EndsWith("]"))
            {
                s = "[" + s["vec![".Length..^1] + "]";
            }

            // Rust iterator comprehension: (src).into_iter()[.filter(|&x| pred)].map(|x| proj).collect::<Vec<_>>()
            var rsComp = RustIterCompRegex().Match(s);
            if (rsComp.Success)
            {
                string srcRaw = UnwrapOuterParens(rsComp.Groups[1].Value.Trim());
                string filterBody = rsComp.Groups[3].Value.Trim();
                string mapVar = rsComp.Groups[4].Value.Trim();
                string mapBody = rsComp.Groups[5].Value.Trim();

                PilotIrExpr iterExpr = srcRaw.Contains("..=")
                    ? new PilotIrRangeExpr
                    {
                        Start = ParseForeignExpr(srcRaw.Split("..=", 2)[0], lang),
                        End = ParseForeignExpr(srcRaw.Split("..=", 2)[1], lang),
                        IsInclusive = true
                    }
                    : (srcRaw.Contains("..")
                        ? new PilotIrRangeExpr
                        {
                            Start = ParseForeignExpr(srcRaw.Split("..", 2)[0], lang),
                            End = ParseForeignExpr(srcRaw.Split("..", 2)[1], lang),
                            IsInclusive = false
                        }
                        : ParseForeignExpr(srcRaw, lang));

                var fe = new PilotIrForExpr
                {
                    Iterable = iterExpr,
                    FilterCondition = !string.IsNullOrEmpty(filterBody) ? ParseForeignExpr(filterBody, lang) : null,
                    YieldExpr = ParseForeignExpr(mapBody, lang)
                };
                fe.TargetVars.Add(mapVar);
                return fe;
            }

            if (s.StartsWith("Err(") && s.EndsWith(")"))
            {
                var failCall = new PilotIrCallExpr
                {
                    Callee = new PilotIrIdentifierExpr { Name = "fail" },
                    LibraryCallKind = PilotLibraryCallKind.FailConstructor
                };
                failCall.Arguments.Add(new PilotIrCallArg(null, ParseForeignExpr(s["Err(".Length..^1], lang)));
                return failCall;
            }

            if (s.StartsWith("Ok(") && s.EndsWith(")"))
            {
                return ParseForeignExpr(s["Ok(".Length..^1], lang);
            }

            if (s.StartsWith("Some(") && s.EndsWith(")"))
            {
                var someCall = new PilotIrCallExpr
                {
                    Callee = new PilotIrIdentifierExpr { Name = "some" },
                    LibraryCallKind = PilotLibraryCallKind.SomeConstructor
                };
                someCall.Arguments.Add(new PilotIrCallArg(null, ParseForeignExpr(s["Some(".Length..^1], lang)));
                return someCall;
            }
        }
        else if (lang == "c")
        {
            if (s.StartsWith("printf(") && s.EndsWith(")"))
            {
                string inner = s["printf(".Length..^1];
                var fExpr = ConvertFormatArgsToInterpolated(inner, lang);
                var pCall = new PilotIrCallExpr
                {
                    Callee = new PilotIrIdentifierExpr { Name = "println" },
                    LibraryCallKind = PilotLibraryCallKind.Println
                };
                pCall.Arguments.Add(new PilotIrCallArg(null, fExpr));
                return pCall;
            }
        }
        else if (lang == "cpp")
        {
            if (s.StartsWith("std::cout") || s.StartsWith("cout"))
            {
                var parts = s.Split("<<")
                    .Skip(1)
                    .Select(p => p.Trim())
                    .Where(p => p is not ("std::endl" or "endl" or "'\\n'" or "\"\\n\""))
                    .ToList();
                var pCall = new PilotIrCallExpr
                {
                    Callee = new PilotIrIdentifierExpr { Name = "println" },
                    LibraryCallKind = PilotLibraryCallKind.Println
                };
                if (parts.Count == 1)
                {
                    pCall.Arguments.Add(new PilotIrCallArg(null, ParseForeignExpr(parts[0], lang)));
                }
                else
                {
                    var fstr = new PilotIrInterpolatedStringExpr();
                    foreach (var p in parts)
                    {
                        if (p.StartsWith("\"") && p.EndsWith("\""))
                            fstr.Parts.Add(new PilotIrFStringPart(false, p[1..^1]));
                        else
                        {
                            string cleanHole = UnwrapOuterParens(p);
                            fstr.Parts.Add(new PilotIrFStringPart(true, cleanHole, ParseForeignExpr(cleanHole, lang)));
                        }
                    }
                    pCall.Arguments.Add(new PilotIrCallArg(null, fstr));
                }
                return pCall;
            }

            // C++ IIFE comprehension: ([&]() { std::vector<...> __res; for (...) { [if (!(cond)) continue;] __res.push_back(expr); } return __res; })()
            var cppComp = CppIifeCompRegex().Match(s);
            if (cppComp.Success)
            {
                string varName = cppComp.Groups[1].Value;
                string startRaw = cppComp.Groups[2].Value.Trim();
                bool inc = cppComp.Groups[3].Value == "<=";
                string endRaw = cppComp.Groups[4].Value.Trim();
                string filterRaw = cppComp.Groups[5].Value.Trim();
                string yieldRaw = cppComp.Groups[6].Value.Trim();
                var fe = new PilotIrForExpr
                {
                    Iterable = new PilotIrRangeExpr
                    {
                        Start = ParseForeignExpr(startRaw, lang),
                        End = ParseForeignExpr(endRaw, lang),
                        IsInclusive = inc
                    },
                    FilterCondition = !string.IsNullOrEmpty(filterRaw) ? ParseForeignExpr(filterRaw, lang) : null,
                    YieldExpr = ParseForeignExpr(yieldRaw, lang)
                };
                fe.TargetVars.Add(varName);
                return fe;
            }

            // C++ std::vector<T>{1, 2, 3}
            if (s.StartsWith("std::vector<", StringComparison.Ordinal) && s.EndsWith("}", StringComparison.Ordinal))
            {
                int braceIdx = s.IndexOf('{');
                if (braceIdx > 0)
                {
                    var coll = new PilotIrCollectionExpr { Kind = PilotCollectionKind.List };
                    foreach (var el in SplitTopLevelComma(s[(braceIdx + 1)..^1]))
                        coll.Elements.Add(ParseForeignExpr(el, lang));
                    return coll;
                }
            }
        }
        else if (lang == "js")
        {
            if (s.StartsWith("console.log(") && s.EndsWith(")"))
            {
                var pCall = new PilotIrCallExpr
                {
                    Callee = new PilotIrIdentifierExpr { Name = "println" },
                    LibraryCallKind = PilotLibraryCallKind.Println
                };
                foreach (var a in SplitTopLevelComma(s["console.log(".Length..^1]))
                    pCall.Arguments.Add(new PilotIrCallArg(null, ParseForeignExpr(a, lang)));
                return pCall;
            }
            // JS template literal `Hello ${x}`
            if (s.StartsWith("`") && s.EndsWith("`") && s.Length >= 2)
            {
                string inner = s[1..^1].Replace("${", "{");
                return PilotTranspileIrBuilder.ParseInterpolatedString(inner);
            }
            // JS Array.from range comprehension: Array.from({ length: (end - start + 1) }, (_, _i) => (start) + _i)[.filter((x) => pred)].map((x) => proj)
            var jsComp = JsArrayFromCompRegex().Match(s);
            if (jsComp.Success)
            {
                string lenExpr = jsComp.Groups[1].Value.Trim();
                string startRaw = UnwrapOuterParens(jsComp.Groups[2].Value.Trim());
                string filterBody = jsComp.Groups[4].Value.Trim();
                string mapVar = jsComp.Groups[5].Value.Trim();
                string mapBody = jsComp.Groups[6].Value.Trim();

                string trimmedLen = UnwrapOuterParens(lenExpr);
                bool isInc = trimmedLen.EndsWith("+ 1", StringComparison.Ordinal) || trimmedLen.EndsWith("+1", StringComparison.Ordinal);
                if (isInc)
                    trimmedLen = trimmedLen[..trimmedLen.LastIndexOf('+')].Trim();
                int minusIdx = FindTopLevelChar(trimmedLen, '-');
                string endRaw = minusIdx > 0 ? trimmedLen[..minusIdx].Trim() : trimmedLen;

                var fe = new PilotIrForExpr
                {
                    Iterable = new PilotIrRangeExpr
                    {
                        Start = ParseForeignExpr(startRaw, lang),
                        End = ParseForeignExpr(endRaw, lang),
                        IsInclusive = isInc
                    },
                    FilterCondition = !string.IsNullOrEmpty(filterBody) ? ParseForeignExpr(filterBody, lang) : null,
                    YieldExpr = ParseForeignExpr(mapBody, lang)
                };
                fe.TargetVars.Add(mapVar);
                return fe;
            }
        }
        else if (lang == "python")
        {
            if (s.StartsWith("print(") && s.EndsWith(")"))
            {
                var pCall = new PilotIrCallExpr
                {
                    Callee = new PilotIrIdentifierExpr { Name = "println" },
                    LibraryCallKind = PilotLibraryCallKind.Println
                };
                foreach (var a in SplitTopLevelComma(s["print(".Length..^1]))
                    pCall.Arguments.Add(new PilotIrCallArg(null, ParseForeignExpr(a, lang)));
                return pCall;
            }

            // Python list comprehension: [expr for x in iter if cond]
            if (s.StartsWith("[") && s.EndsWith("]") && s.Contains(" for "))
            {
                string inner = s[1..^1].Trim();
                int forIdx = FindTopLevelKeyword(inner, " for ");
                if (forIdx > 0)
                {
                    string yieldPart = inner[..forIdx].Trim();
                    string rest = inner[(forIdx + 5)..].Trim();
                    int inIdx = FindTopLevelKeyword(rest, " in ");
                    if (inIdx > 0)
                    {
                        string varPart = rest[..inIdx].Trim();
                        string iterAndFilter = rest[(inIdx + 4)..].Trim();
                        PilotIrExpr? filterExpr = null;
                        int ifIdx = FindTopLevelKeyword(iterAndFilter, " if ");
                        if (ifIdx > 0)
                        {
                            filterExpr = ParseForeignExpr(iterAndFilter[(ifIdx + 4)..].Trim(), lang);
                            iterAndFilter = iterAndFilter[..ifIdx].Trim();
                        }

                        var fe = new PilotIrForExpr
                        {
                            YieldExpr = ParseForeignExpr(yieldPart, lang),
                            Iterable = ConvertPythonIterable(iterAndFilter),
                            FilterCondition = filterExpr
                        };
                        fe.TargetVars.Add(varPart);
                        return fe;
                    }
                }
            }
        }
        else if (lang == "lua")
        {
            if (s.StartsWith("print(") && s.EndsWith(")"))
            {
                var pCall = new PilotIrCallExpr
                {
                    Callee = new PilotIrIdentifierExpr { Name = "println" },
                    LibraryCallKind = PilotLibraryCallKind.Println
                };
                foreach (var a in SplitTopLevelComma(s["print(".Length..^1]))
                    pCall.Arguments.Add(new PilotIrCallArg(null, ParseForeignExpr(a, lang)));
                return pCall;
            }

            if (s.StartsWith("string.format(", StringComparison.Ordinal) && s.EndsWith(")", StringComparison.Ordinal))
            {
                string inner = s["string.format(".Length..^1];
                string strippedToStr = Regex.Replace(inner, @"\btostring\(\s*([^()]+?)\s*\)", "$1");
                return ConvertFormatArgsToInterpolated(strippedToStr, lang);
            }

            var luaComp = LuaIifeCompRegex().Match(s);
            if (luaComp.Success)
            {
                string varName = luaComp.Groups[1].Value.Trim();
                string startRaw = luaComp.Groups[2].Value.Trim();
                string endRaw = luaComp.Groups[3].Value.Trim();
                string filterRaw = luaComp.Groups[4].Value.Trim();
                string yieldRaw = luaComp.Groups[5].Value.Trim();

                bool isInc = true;
                if (endRaw.EndsWith("- 1", StringComparison.Ordinal))
                {
                    isInc = false;
                    endRaw = endRaw[..^3].Trim();
                }

                var fe = new PilotIrForExpr
                {
                    Iterable = new PilotIrRangeExpr
                    {
                        Start = ParseForeignExpr(startRaw, lang),
                        End = ParseForeignExpr(endRaw, lang),
                        IsInclusive = isInc
                    },
                    FilterCondition = !string.IsNullOrEmpty(filterRaw) ? ParseForeignExpr(filterRaw, lang) : null,
                    YieldExpr = ParseForeignExpr(yieldRaw, lang)
                };
                fe.TargetVars.Add(varName);
                return fe;
            }

            if (s.StartsWith("{", StringComparison.Ordinal) && s.EndsWith("}", StringComparison.Ordinal))
            {
                string inner = s[1..^1].Trim();
                if (string.IsNullOrEmpty(inner))
                    return new PilotIrCollectionExpr { Kind = PilotCollectionKind.List };

                var entries = SplitTopLevelComma(inner);
                if (entries.All(e => FindTopLevelSingleEquals(e) > 0))
                {
                    var map = new PilotIrCollectionExpr { Kind = PilotCollectionKind.Map };
                    foreach (var entry in entries)
                    {
                        int eq = FindTopLevelSingleEquals(entry);
                        string rawKey = entry[..eq].Trim();
                        string rawVal = entry[(eq + 1)..].Trim();
                        PilotIrExpr keyExpr;
                        if (rawKey.StartsWith("[") && rawKey.EndsWith("]"))
                        {
                            keyExpr = ParseForeignExpr(rawKey[1..^1], lang);
                        }
                        else
                        {
                            keyExpr = new PilotIrLiteralExpr
                            {
                                Kind = PilotLiteralKind.String,
                                RawValue = $"\"{rawKey}\"",
                                StringContent = rawKey
                            };
                        }
                        map.MapEntries.Add((keyExpr, ParseForeignExpr(rawVal, lang)));
                    }
                    return map;
                }

                var list = new PilotIrCollectionExpr { Kind = PilotCollectionKind.List };
                foreach (var el in entries)
                    list.Elements.Add(ParseForeignExpr(el, lang));
                return list;
            }
        }

        // Normalize foreign operators to Pilot syntax and parse via PilotTranspileIrBuilder
        string normalized = NormalizeForeignExpressionToPilot(s, lang);
        var (mod, errs) = PilotTranspileIrBuilder.BuildFromSource(normalized + ";");
        if (mod != null && errs.Count == 0 && mod.Statements.Count > 0 && mod.Statements[0] is PilotIrExprStmt es)
        {
            return es.Expression;
        }

        return new PilotIrIdentifierExpr { Name = normalized };
    }

    private static string NormalizeForeignExpressionToPilot(string expr, string lang)
    {
        string s = PilotLibraryMapper.TranslateForeignLibrariesToPilot(expr, lang);

        if (lang == "python")
        {
            s = Regex.Replace(s, @"\bTrue\b", "true");
            s = Regex.Replace(s, @"\bFalse\b", "false");
            s = Regex.Replace(s, @"\bNone\b", "null");
            s = Regex.Replace(s, @"\bis\s+not\s+null\b", "!= null");
            s = Regex.Replace(s, @"\bis\s+null\b", "== null");
            s = Regex.Replace(s, @"\b__ext_[A-Za-z0-9_]+_([A-Za-z_][A-Za-z0-9_]*)\(\s*([^,)]+)\s*,\s*([^)]*)\)", "($2).$1($3)");
            s = Regex.Replace(s, @"\b__ext_[A-Za-z0-9_]+_([A-Za-z_][A-Za-z0-9_]*)\(\s*([^,)]+)\s*\)", "($2).$1()");
            s = Regex.Replace(s, @"\bself_val\b", "self");
        }
        else if (lang == "js")
        {
            s = s.Replace("===", "==").Replace("!==", "!=");
            s = Regex.Replace(s, @"\bnew\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(", "$1(");
            s = Regex.Replace(s, @"\bthis\b", "self");
            s = Regex.Replace(s, @"Math\.trunc\(\s*([^)]+?)\s*/\s*([^)]+?)\s*\)", "(($1) // ($2))");
            if (s.Contains('`'))
            {
                s = JsTemplateRegex().Replace(s, m =>
                {
                    string inner = m.Groups[1].Value.Replace("${", "{");
                    return $"f\"{inner}\"";
                });
            }
        }
        else if (lang == "lua")
        {
            s = s.Replace("~=", "!=");
            s = Regex.Replace(s, @"\bnil\b", "null");
            s = Regex.Replace(s, @"\.new\s*\(\s*\{\s*\}\s*\)", "()");
            s = Regex.Replace(s, @"\.new\s*\(", "(");
            s = Regex.Replace(s, @"([A-Za-z_][A-Za-z0-9_]*):([A-Za-z_][A-Za-z0-9_]*)\s*\(", "$1.$2(");
            s = Regex.Replace(s, @"\btostring\(\s*([^()]+?)\s*\)", "($1)");
            s = Regex.Replace(s, @"\s*\.\.\s*", " + ");
        }
        else if (lang == "rust")
        {
            s = s.Replace("..=", "..");
            s = Regex.Replace(s, @"\bNone\b", "none");
            s = Regex.Replace(s, @"\bSome\(", "some(");
            s = Regex.Replace(s, @"::", ".");
            s = Regex.Replace(s, @"\s+as\s+(?:i8|i16|i32|i64|u8|u16|u32|u64|usize|isize|f32|f64)\b", "");
        }
        else if (lang is "c" or "cpp")
        {
            s = Regex.Replace(s, @"\((?:long\s+long|int64_t|int32_t|int16_t|int8_t|uint64_t|uint32_t|int|double|float)\)\s*\(([^()]+)\)", "($1)");
            s = Regex.Replace(s, @"\((?:long\s+long|int64_t|int32_t|int16_t|int8_t|uint64_t|uint32_t|int|double|float)\)\s*([A-Za-z0-9_.]+)", "$1");
            s = Regex.Replace(s, @"\(\*this\)", "self");
            s = Regex.Replace(s, @"\bself_val\b", "self");
            s = Regex.Replace(s, @"(?:std::)?pow\(\s*([^,]+?)\s*,\s*([^)]+?)\s*\)", "(($1) ** ($2))");
        }

        if (lang is "c" or "cpp" or "js" or "rust")
        {
            s = s.Replace("&&", " and ").Replace("||", " or ");
        }

        return s;
    }

    private static PilotIrExpr ConvertFormatArgsToInterpolated(string innerArgs, string lang)
    {
        var parts = SplitTopLevelComma(innerArgs);
        if (parts.Count == 0)
            return new PilotIrLiteralExpr { Kind = PilotLiteralKind.String, RawValue = "\"\"", StringContent = "" };

        string fmtRaw = parts[0].Trim();
        if (fmtRaw.StartsWith("\"") && fmtRaw.EndsWith("\"") && fmtRaw.Length >= 2)
        {
            string fmt = fmtRaw[1..^1];
            if (fmt.EndsWith("\\n"))
                fmt = fmt[..^2];

            if (parts.Count == 1)
            {
                return new PilotIrLiteralExpr
                {
                    Kind = PilotLiteralKind.String,
                    RawValue = $"\"{fmt}\"",
                    StringContent = fmt
                };
            }

            var args = parts.Skip(1).Select(a => a.Trim()).ToList();
            if ((fmt == "{}" || fmt == "{:?}") && args.Count == 1)
            {
                return ParseForeignExpr(args[0], lang);
            }

            int argIdx = 0;
            string replaced = FormatPlaceholderRegex().Replace(fmt, _ =>
                argIdx < args.Count ? $"{{{NormalizeForeignExpressionToPilot(args[argIdx++], lang)}}}" : "");
            return PilotTranspileIrBuilder.ParseInterpolatedString(replaced);
        }

        return ParseForeignExpr(fmtRaw, lang);
    }

    public static string MapForeignType(string rawType, string lang)
    {
        string t = rawType.Trim();
        if (string.IsNullOrEmpty(t))
            return "any";

        // Python Final[T], Optional[T], List[T], Dict[K, V]
        if (t.StartsWith("Final[") && t.EndsWith("]"))
            return MapForeignType(t["Final[".Length..^1], lang);
        if (t == "Final")
            return "any";
        if (t.StartsWith("Optional[") && t.EndsWith("]"))
            return MapForeignType(t["Optional[".Length..^1], lang) + "?";
        if (t.EndsWith("| None") || t.EndsWith("|None"))
            return MapForeignType(t[..t.IndexOf('|')].Trim(), lang) + "?";
        if ((t.StartsWith("List[") || t.StartsWith("list[")) && t.EndsWith("]"))
            return $"[{MapForeignType(t[(t.IndexOf('[') + 1)..^1], lang)}]";
        if ((t.StartsWith("Dict[") || t.StartsWith("dict[")) && t.EndsWith("]"))
        {
            var kv = SplitTopLevelComma(t[(t.IndexOf('[') + 1)..^1]);
            if (kv.Count == 2)
                return $"{{{MapForeignType(kv[0], lang)}: {MapForeignType(kv[1], lang)}}}";
        }

        // Rust Option<T>, Result<T, E>, Vec<T>
        if (t.StartsWith("Option<") && t.EndsWith(">"))
            return MapForeignType(t["Option<".Length..^1], lang) + "?";
        if (t.StartsWith("Result<") && t.EndsWith(">"))
        {
            var okErr = SplitTopLevelComma(t["Result<".Length..^1]);
            if (okErr.Count == 2)
                return $"{MapForeignType(okErr[0], lang)} ! {MapForeignType(okErr[1], lang)}";
        }
        if ((t.StartsWith("Vec<") || t.StartsWith("std::vector<")) && t.EndsWith(">"))
            return $"[{MapForeignType(t[(t.IndexOf('<') + 1)..^1], lang)}]";

        return t switch
        {
            "int8_t" or "i8" => "i8",
            "int16_t" or "i16" => "i16",
            "int32_t" or "i32" => "i32",
            "int64_t" or "i64" or "isize" or "long" => "i64",
            "uint8_t" or "u8" => "u8",
            "uint16_t" or "u16" => "u16",
            "uint32_t" or "u32" => "u32",
            "uint64_t" or "u64" or "usize" or "size_t" => "u64",
            "int" or "number" => "int",
            "float" => lang == "python" ? "float" : "f32",
            "double" or "f64" => "f64",
            "bool" or "boolean" => "bool",
            "String" or "&str" or "std::string" or "string" or "const char*" or "const char *" or "char*" => "str",
            "None" or "()" or "void" => "void",
            "Any" => "any",
            _ => t
        };
    }

    private static PilotIrPattern ParseForeignPattern(string patStr)
    {
        string p = patStr.Trim();
        if (p is "_" or "default")
            return new PilotIrWildcardPattern();
        if (p is "None" or "none" or "null")
            return new PilotIrOptionPattern { IsSome = false };
        if (p.StartsWith("Some(") && p.EndsWith(")"))
            return new PilotIrOptionPattern { IsSome = true, InnerPattern = ParseForeignPattern(p["Some(".Length..^1]) };

        if (p.Contains("::"))
            p = p.Split("::").Last();

        int openParen = p.IndexOf('(');
        int openBrace = p.IndexOf('{');
        if (openParen > 0 && p.EndsWith(")"))
        {
            string vName = p[..openParen].Trim();
            var vp = new PilotIrVariantPattern { VariantName = vName };
            foreach (var arg in SplitTopLevelComma(p[(openParen + 1)..^1]))
            {
                vp.Arguments.Add((null, ParseForeignPattern(arg)));
            }
            return vp;
        }
        if (openBrace > 0 && p.EndsWith("}"))
        {
            string vName = p[..openBrace].Trim();
            var vp = new PilotIrVariantPattern { VariantName = vName };
            foreach (var arg in SplitTopLevelComma(p[(openBrace + 1)..^1]))
            {
                string a = arg.Trim();
                if (a.Contains(':'))
                {
                    var kv = a.Split(':', 2);
                    vp.Arguments.Add((kv[0].Trim(), ParseForeignPattern(kv[1].Trim())));
                }
                else
                {
                    vp.Arguments.Add((null, new PilotIrBindingPattern { Name = a }));
                }
            }
            return vp;
        }

        if (p.StartsWith("\"") || char.IsDigit(p[0]) || (p[0] == '-' && p.Length > 1 && char.IsDigit(p[1])))
        {
            return new PilotIrLiteralPattern
            {
                Literal = new PilotIrLiteralExpr
                {
                    Kind = p.StartsWith("\"") ? PilotLiteralKind.String : PilotLiteralKind.Integer,
                    RawValue = p,
                    StringContent = p.Trim('"')
                }
            };
        }

        return new PilotIrBindingPattern { Name = p };
    }

    // ========================================================================
    // STRING & BRACKET PARSING UTILITIES
    // ========================================================================

    private static void AttachTrivia(PilotIrStmt stmt, List<PilotTrivia> pending)
    {
        if (pending.Count > 0)
        {
            stmt.LeadingTrivia.AddRange(pending);
            pending.Clear();
        }
    }

    private static int CountLeadingSpaces(string line)
    {
        int count = 0;
        foreach (char c in line)
        {
            if (c == ' ') count++;
            else if (c == '\t') count += 4;
            else break;
        }
        return count;
    }

    private static string StripTrailingPythonComment(string line, out string? comment)
    {
        comment = null;
        bool inSingle = false;
        bool inDouble = false;
        for (int i = 0; i < line.Length; i++)
        {
            char c = line[i];
            if (c == '\\' && i + 1 < line.Length) { i++; continue; }
            if (c == '\'' && !inDouble) inSingle = !inSingle;
            else if (c == '"' && !inSingle) inDouble = !inDouble;
            else if (c == '#' && !inSingle && !inDouble)
            {
                comment = line[i..];
                return line[..i].TrimEnd();
            }
        }
        return line;
    }

    private static int CountUnclosedBrackets(string s)
    {
        int depth = 0;
        bool inSingle = false;
        bool inDouble = false;
        for (int i = 0; i < s.Length; i++)
        {
            char c = s[i];
            if (c == '\\' && i + 1 < s.Length) { i++; continue; }
            if (c == '\'' && !inDouble) inSingle = !inSingle;
            else if (c == '"' && !inSingle) inDouble = !inDouble;
            else if (!inSingle && !inDouble)
            {
                if (c is '(' or '[' or '{') depth++;
                else if (c is ')' or ']' or '}') depth--;
            }
        }
        return depth;
    }

    private static int PeekNextBlockIndent(List<(int Indent, string Text, int LineNo)> lines, int idx, int parentIndent)
    {
        for (int i = idx; i < lines.Count; i++)
        {
            if (!string.IsNullOrWhiteSpace(lines[i].Text) && !lines[i].Text.StartsWith("#"))
                return Math.Max(parentIndent + 1, lines[i].Indent);
        }
        return parentIndent + 4;
    }

    private static void SkipBlankLines(List<(int Indent, string Text, int LineNo)> lines, ref int idx)
    {
        while (idx < lines.Count && string.IsNullOrWhiteSpace(lines[idx].Text))
            idx++;
    }

    private static int FindMatchingCloseParen(string s, int openIdx)
    {
        if (openIdx < 0 || openIdx >= s.Length) return -1;
        int depth = 0;
        for (int i = openIdx; i < s.Length; i++)
        {
            if (s[i] == '(') depth++;
            else if (s[i] == ')')
            {
                depth--;
                if (depth == 0) return i;
            }
        }
        return s.Length - 1;
    }

    private static List<string> SplitTopLevelComma(string s)
    {
        var list = new List<string>();
        if (string.IsNullOrWhiteSpace(s)) return list;
        int depth = 0;
        int start = 0;
        bool inStr = false;
        char quote = '"';

        for (int i = 0; i < s.Length; i++)
        {
            char c = s[i];
            if (inStr)
            {
                if (c == '\\' && i + 1 < s.Length) i++;
                else if (c == quote) inStr = false;
                continue;
            }
            if (c is '"' or '\'' or '`')
            {
                inStr = true;
                quote = c;
            }
            else if (c is '(' or '[' or '{' or '<') depth++;
            else if (c is ')' or ']' or '}' or '>') depth = Math.Max(0, depth - 1);
            else if (c == ',' && depth == 0)
            {
                list.Add(s[start..i].Trim());
                start = i + 1;
            }
        }
        string last = s[start..].Trim();
        if (!string.IsNullOrEmpty(last))
            list.Add(last);
        return list;
    }

    private static int FindTopLevelChar(string s, char target)
    {
        int depth = 0;
        bool inStr = false;
        char quote = '"';
        for (int i = 0; i < s.Length; i++)
        {
            char c = s[i];
            if (inStr)
            {
                if (c == '\\' && i + 1 < s.Length) i++;
                else if (c == quote) inStr = false;
                continue;
            }
            if (c is '"' or '\'' or '`') { inStr = true; quote = c; }
            else if (c is '(' or '[' or '{' or '<') depth++;
            else if (c is ')' or ']' or '}' or '>') depth = Math.Max(0, depth - 1);
            else if (c == target && depth == 0) return i;
        }
        return -1;
    }

    private static int FindTopLevelSingleEquals(string s)
    {
        int depth = 0;
        bool inStr = false;
        char quote = '"';
        for (int i = 0; i < s.Length; i++)
        {
            char c = s[i];
            if (inStr)
            {
                if (c == '\\' && i + 1 < s.Length) i++;
                else if (c == quote) inStr = false;
                continue;
            }
            if (c is '"' or '\'' or '`') { inStr = true; quote = c; }
            else if (c is '(' or '[' or '{') depth++;
            else if (c is ')' or ']' or '}') depth = Math.Max(0, depth - 1);
            else if (c == '=' && depth == 0)
            {
                char prev = i > 0 ? s[i - 1] : '\0';
                char next = i + 1 < s.Length ? s[i + 1] : '\0';
                if (prev is '=' or '!' or '<' or '>' or '+' or '-' or '*' or '/' or '%' or ':')
                    continue;
                if (next is '=' or '>')
                    continue;
                return i;
            }
        }
        return -1;
    }

    private static int FindTopLevelOperator(string s, string op)
    {
        int depth = 0;
        bool inStr = false;
        char quote = '"';
        for (int i = 0; i <= s.Length - op.Length; i++)
        {
            char c = s[i];
            if (inStr)
            {
                if (c == '\\' && i + 1 < s.Length) i++;
                else if (c == quote) inStr = false;
                continue;
            }
            if (c is '"' or '\'' or '`') { inStr = true; quote = c; }
            else if (c is '(' or '[' or '{') depth++;
            else if (c is ')' or ']' or '}') depth = Math.Max(0, depth - 1);
            else if (depth == 0 && s.AsSpan(i, op.Length).SequenceEqual(op.AsSpan()))
                return i;
        }
        return -1;
    }

    private static int FindTopLevelKeyword(string s, string kw) => FindTopLevelOperator(s, kw);

    private static string UnwrapOuterParens(string s)
    {
        string t = s.Trim();
        if (t.Length >= 2 && t[0] == '(' && t[^1] == ')' && FindMatchingCloseParen(t, 0) == t.Length - 1)
            return t[1..^1].Trim();
        return t;
    }

    private static bool IsIdentifier(string s)
    {
        if (string.IsNullOrEmpty(s)) return false;
        if (!(char.IsLetter(s[0]) || s[0] == '_')) return false;
        for (int i = 1; i < s.Length; i++)
        {
            if (!(char.IsLetterOrDigit(s[i]) || s[i] == '_')) return false;
        }
        return true;
    }

    [GeneratedRegex(@"^([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([^,]+),\s*(.+)$")]
    private static partial Regex LuaNumForRegex();
    [GeneratedRegex(@"^(?:_|\w+)\s*,\s*([A-Za-z_][A-Za-z0-9_]*)\s+in\s+(?:i?pairs\(\s*([^)]+)\s*\)|(.+))$")]
    private static partial Regex LuaGenForRegex();
    [GeneratedRegex(@"^(?:pub\s+|typedef\s+)?enum(?:\s+class)?\s+([A-Za-z_][A-Za-z0-9_]*)")]
    private static partial Regex EnumHeaderRegex();
    [GeneratedRegex(@"^(?:pub\s+|typedef\s+)?(struct|class)\s+([A-Za-z_][A-Za-z0-9_]*)")]
    private static partial Regex StructClassHeaderRegex();
    [GeneratedRegex(@"^(?:pub\s+)?(?:const\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]+>)?\s*\(([^)]*)\)(?:\s*->\s*(.+))?$")]
    private static partial Regex RustFnHeaderRegex();
    [GeneratedRegex(@"^(?:export\s+)?(async\s+)?function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)(?:\s*:\s*(.+))?$")]
    private static partial Regex JsFnHeaderRegex();
    [GeneratedRegex(@"^(?:static\s+)?(?:inline\s+)?(?:constexpr\s+)?([A-Za-z_][A-Za-z0-9_:<>*&\s]*?)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)$")]
    private static partial Regex CFnHeaderRegex();
    [GeneratedRegex(@"^(?:int64_t|int32_t|int|size_t|let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([^;]+);\s*\1\s*(<=|<)\s*([^;]+);\s*(?:\+\+\1|\1\+\+|\1\s*\+=\s*1)$")]
    private static partial Regex CStyleForRegex();
    [GeneratedRegex(@"^(?:const\s+)?(?:auto|int64_t|int|std::string)\s*&?\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+)$")]
    private static partial Regex CppRangeForRegex();
    [GeneratedRegex(@"^(?:const\s+|let\s+|var\s+)?([A-Za-z_][A-Za-z0-9_]*)\s+(?:in|of)\s+(.+)$")]
    private static partial Regex ForInOrOfRegex();
    [GeneratedRegex(@"^(const\s+|constexpr\s+)?(int8_t|int16_t|int32_t|int64_t|uint8_t|uint16_t|uint32_t|uint64_t|int|long|float|double|bool|auto|std::string|std::vector<[^>]+>|const\s+char\s*\*)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+)$")]
    private static partial Regex CVarDeclRegex();
    [GeneratedRegex(@"^(int8_t|int16_t|int32_t|int64_t|uint8_t|uint16_t|uint32_t|uint64_t|int|long|float|double|bool|std::string|std::vector<[^>]+>|const\s+char\s*\*)\s+([A-Za-z_][A-Za-z0-9_]*)(?:\{\})?$")]
    private static partial Regex CFieldNoInitRegex();
    [GeneratedRegex(@"struct\s*\{([^}]*)\}\s*([A-Za-z_][A-Za-z0-9_]*)\s*;")]
    private static partial Regex CUnionVariantStructRegex();
    [GeneratedRegex(@"^(static\s+)?(async\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)(?:\s*:\s*(.+))?$")]
    private static partial Regex JsClassMethodRegex();
    [GeneratedRegex(@"^\((.+)\)\.into_iter\(\)(\.filter\(\|\&?[A-Za-z_][A-Za-z0-9_]*\|\s*([^)]+)\))?\.map\(\|([A-Za-z_][A-Za-z0-9_]*)\|\s*(.+)\)\.collect::<Vec<_>>\(\)$")]
    private static partial Regex RustIterCompRegex();
    [GeneratedRegex(@"^\(\[\&\]\(\)\s*\{\s*std::vector<[^>]+>\s+__res;\s*for\s*\([^;]*?\b([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([^;]+);\s*\1\s*(<=|<)\s*([^;]+);[^)]*\)\s*\{\s*(?:if\s*\(!\((.+?)\)\)\s*continue;\s*)?__res\.push_back\((.+)\);\s*\}\s*return\s+__res;\s*\}\)\(\)$")]
    private static partial Regex CppIifeCompRegex();
    [GeneratedRegex(@"^\(?Array\.from\(\{\s*length:\s*(.+?)\s*\},\s*\(_,\s*_i\)\s*=>\s*(.+?)\s*\+\s*_i\)\)?(\.filter\(\([A-Za-z_][A-Za-z0-9_]*\)\s*=>\s*(.+?)\))?\.map\(\(([A-Za-z_][A-Za-z0-9_]*)\)\s*=>\s*(.+)\)$")]
    private static partial Regex JsArrayFromCompRegex();
    [GeneratedRegex(@"^\(function\(\)\s*local\s+__res\s*=\s*\{\}\s*;\s*for\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([^,]+),\s*(.+?)\s+do\s+(?:if\s+(.+?)\s+then\s+)?table\.insert\(__res,\s*(.+?)\)\s*(?:end\s+)?end\s*;\s*return\s+__res\s+end\)\(\)$")]
    private static partial Regex LuaIifeCompRegex();
    [GeneratedRegex(@"`([^`]*)`")]
    private static partial Regex JsTemplateRegex();
    [GeneratedRegex(@"(\{\:?[^\}]*\}|%[0-9.]*(?:lld|ld|zu|d|i|u|f|g|s))")]
    private static partial Regex FormatPlaceholderRegex();
}
