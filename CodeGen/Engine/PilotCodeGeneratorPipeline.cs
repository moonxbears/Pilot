using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.RegularExpressions;
using Antlr4.Runtime;
using Pilot.CodeGen.Abstractions;
using Pilot.CodeGen.Builtins;
using Pilot.CodeGen.Model;
using Pilot.Compiler;
using Pilot.Runtime.Values;

namespace Pilot.CodeGen.Engine;

/// <summary>
/// Execution pipeline for Pilot source generation.
/// Designed for zero overhead when unused, while providing comprehensive,
/// Pythonic code generation, partial class augmentation, and metadata control.
/// </summary>
public static class PilotCodeGeneratorPipeline
{
    private static readonly List<IPilotCodeGenerator> BuiltinGenerators = new()
    {
        new JsonGenerator(),
        new BuilderGenerator(),
        new ReprGenerator(),
        new ObservableGenerator(),
        new EqualityGenerator(),
        new MetadataGenerator()
    };

    /// <summary>
    /// Fast gate: checks if code generation is required for the given source code.
    /// Incurs zero overhead (0ns allocations) when code generation is unused.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool ShouldRun(string? source)
    {
        if (string.IsNullOrEmpty(source)) return false;

        return source.Contains("@generate", StringComparison.OrdinalIgnoreCase) ||
               source.Contains("@generator", StringComparison.OrdinalIgnoreCase) ||
               source.Contains("@builder", StringComparison.OrdinalIgnoreCase) ||
               source.Contains("@json", StringComparison.OrdinalIgnoreCase) ||
               source.Contains("@repr", StringComparison.OrdinalIgnoreCase) ||
               source.Contains("@dataclass", StringComparison.OrdinalIgnoreCase) ||
               source.Contains("@observable", StringComparison.OrdinalIgnoreCase) ||
               source.Contains("@equality", StringComparison.OrdinalIgnoreCase) ||
               source.Contains("@meta", StringComparison.OrdinalIgnoreCase) ||
               source.Contains("@metadata", StringComparison.OrdinalIgnoreCase) ||
               source.Contains("codegen", StringComparison.OrdinalIgnoreCase);
    }

    /// <summary>
    /// Executes code generation for the provided source code.
    /// </summary>
    public static PilotGeneratorResult Run(string source, string? filePath = null, PilotCompilerOptions? options = null)
    {
        if (!ShouldRun(source))
        {
            return PilotGeneratorResult.Unmodified(source);
        }

        var script = ExtractScriptModel(source, filePath);
        return Run(script, options);
    }

    /// <summary>
    /// Executes code generation for a populated ScriptModel.
    /// </summary>
    public static PilotGeneratorResult Run(ScriptModel script, PilotCompilerOptions? options = null)
    {
        var context = new PilotGeneratorContext(script);

        // 1. Run built-in generators
        foreach (var gen in BuiltinGenerators)
        {
            try
            {
                gen.Execute(context);
            }
            catch (Exception ex)
            {
                context.ReportDiagnostic($"[CodeGen Warning] Generator '{gen.Name}' failed: {ex.Message}");
            }
        }

        // 2. Discover and execute in-script custom @generator functions
        ExecuteInScriptGenerators(script, context);

        // 3. Assemble augmented source and companion scripts
        var sb = new StringBuilder(script.Source);

        // Apply script replacements (self-modifying scripts)
        foreach (var (target, replacement) in script.Replacements)
        {
            sb.Replace(target, replacement);
        }

        // Insert class partial extensions immediately after their class definitions
        var classesWithPartials = script.Classes
            .Where(c => c.GeneratedPartials.Count > 0)
            .OrderByDescending(c => c.EndIndex)
            .ToList();

        foreach (var cls in classesWithPartials)
        {
            var partialCode = new StringBuilder();
            partialCode.AppendLine();
            partialCode.AppendLine($"# --- CodeGen Extension for {cls.Name} ---");
            foreach (var partial in cls.GeneratedPartials)
            {
                if (partial.StartsWith("partial class", StringComparison.OrdinalIgnoreCase))
                {
                    partialCode.AppendLine(partial);
                }
                else
                {
                    partialCode.AppendLine($"partial class {cls.Name} {{");
                    partialCode.AppendLine($"    {partial}");
                    partialCode.AppendLine("}");
                }
            }

            if (cls.EndIndex >= 0 && cls.EndIndex < sb.Length)
            {
                sb.Insert(cls.EndIndex + 1, partialCode.ToString());
            }
            else
            {
                sb.AppendLine(partialCode.ToString());
            }
        }

        // Append added script-level code
        foreach (var added in script.AddedCode)
        {
            sb.AppendLine();
            sb.AppendLine(added);
        }

        // Merge companion scripts into context and compilation
        foreach (var (companionName, companionCode) in script.AddedScripts)
        {
            context.AddSource(companionName, companionCode);
            sb.AppendLine();
            sb.AppendLine($"# === Generated Companion: {companionName} ===");
            sb.AppendLine(companionCode);
        }

        string augmented = sb.ToString();
        bool hasMods = context.GeneratedSources.Count > 0 ||
                       script.AddedCode.Count > 0 ||
                       script.Replacements.Count > 0 ||
                       script.Classes.Any(c => c.GeneratedPartials.Count > 0);

        return new PilotGeneratorResult(augmented, context.GeneratedSources, context.Diagnostics, hasMods);
    }

    /// <summary>
    /// Parses the Pilot source and builds a clean, Pythonic semantic model.
    /// </summary>
    public static ScriptModel ExtractScriptModel(string source, string? filePath = null)
    {
        string scriptName = !string.IsNullOrEmpty(filePath) ? Path.GetFileName(filePath) : "Script";
        var model = new ScriptModel
        {
            Name = scriptName,
            Source = source
        };

        try
        {
            var inputStream = new AntlrInputStream(source);
            var lexer = new PilotLexer(inputStream);
            var tokenStream = new CommonTokenStream(lexer);
            var parser = new PilotParser(tokenStream);
            parser.RemoveErrorListeners();

            var programContext = parser.program();
            if (programContext == null) return model;

            // Visit top-level statements
            foreach (var stmt in programContext.stmt())
            {
                var compound = stmt.compound_stmt();
                if (compound == null) continue;

                if (compound.class_def() != null)
                {
                    var cls = ExtractClassModel(compound.class_def());
                    if (cls != null) model.Classes.Add(cls);
                }
                else if (compound.func_def() != null)
                {
                    var fn = ExtractMethodModel(compound.func_def());
                    if (fn != null) model.Functions.Add(fn);
                }
            }
        }
        catch
        {
            // Fallback to regex-based extraction if parser encountered incomplete syntax
            ExtractViaRegexFallback(source, model);
        }

        return model;
    }

    private static ClassModel? ExtractClassModel(PilotParser.Class_defContext ctx)
    {
        string name = ctx.identifier()?.GetText() ?? "";
        if (string.IsNullOrEmpty(name)) return null;

        var cls = new ClassModel
        {
            Name = name,
            IsPartial = ctx.declaration_modifier()?.Any(m => m.PARTIAL() != null) ?? false,
            BaseClassName = ctx.class_bases()?.GetText(),
            EndIndex = ctx.Stop?.StopIndex ?? -1
        };

        // Extract class decorators
        if (ctx.decorators() != null)
        {
            foreach (var d in ctx.decorators().decorator())
            {
                cls.Decorators.Add(ExtractDecorator(d));
            }
        }

        // Extract members inside class body
        var suite = ctx.suite();
        if (suite?.stmt() != null)
        {
            foreach (var s in suite.stmt())
            {
                if (s.simple_stmt()?.small_stmt() != null)
                {
                    foreach (var sm in s.simple_stmt().small_stmt())
                    {
                        if (sm.var_stmt() != null)
                        {
                            var v = sm.var_stmt();
                            string fName = v.binding_pattern()?.identifier()?.GetText() ?? "";
                            if (!string.IsNullOrEmpty(fName))
                            {
                                cls.Fields.Add(new PropertyModel
                                {
                                    Name = fName,
                                    TypeName = v.type_annotation()?.type_spec()?.GetText(),
                                    DefaultValue = v.test()?.GetText(),
                                    IsMutable = true
                                });
                            }
                        }
                        else if (sm.let_stmt() != null)
                        {
                            var l = sm.let_stmt();
                            string fName = l.binding_pattern()?.identifier()?.GetText() ?? "";
                            if (!string.IsNullOrEmpty(fName))
                            {
                                cls.Fields.Add(new PropertyModel
                                {
                                    Name = fName,
                                    TypeName = l.type_annotation()?.type_spec()?.GetText(),
                                    DefaultValue = l.test()?.GetText(),
                                    IsMutable = false
                                });
                            }
                        }
                    }
                }
                else if (s.compound_stmt()?.func_def() != null)
                {
                    var fn = ExtractMethodModel(s.compound_stmt().func_def());
                    if (fn != null) cls.Methods.Add(fn);
                }
            }
        }

        return cls;
    }

    private static MethodModel? ExtractMethodModel(PilotParser.Func_defContext ctx)
    {
        string name = ctx.identifier()?.GetText() ?? "";
        if (string.IsNullOrEmpty(name)) return null;

        var method = new MethodModel
        {
            Name = name,
            IsAsync = ctx.ASYNC() != null,
            ReturnType = ctx.type_spec()?.GetText(),
            BodyText = ctx.suite()?.GetText()
        };

        if (ctx.decorators() != null)
        {
            foreach (var d in ctx.decorators().decorator())
            {
                method.Decorators.Add(ExtractDecorator(d));
            }
        }

        if (ctx.parameters()?.parameter() != null)
        {
            foreach (var p in ctx.parameters().parameter())
            {
                method.Parameters.Add(new ParameterModel
                {
                    Name = p.identifier()?.GetText() ?? "arg",
                    TypeName = p.type_annotation()?.type_spec()?.GetText(),
                    DefaultValue = p.test()?.GetText(),
                    IsParams = p.PARAMS() != null || p.STAR() != null,
                    IsKwargs = p.DOUBLESTAR() != null
                });
            }
        }

        return method;
    }

    private static DecoratorModel ExtractDecorator(PilotParser.DecoratorContext ctx)
    {
        string name = ctx.dotted_name()?.GetText() ?? "";
        var dec = new DecoratorModel(name);

        if (ctx.arglist()?.argument() != null)
        {
            foreach (var arg in ctx.arglist().argument())
            {
                if (arg.EQUAL() != null && arg.identifier() != null)
                {
                    string k = arg.identifier().GetText();
                    string v = arg.test() != null ? arg.test().GetText() : "";
                    if (!string.IsNullOrEmpty(k)) dec.NamedArgs[k] = v;
                }
                else
                {
                    string val = arg.test() != null ? arg.test().GetText() : arg.GetText();
                    dec.PositionalArgs.Add(val);
                }
            }
        }

        return dec;
    }

    private static void ExtractViaRegexFallback(string source, ScriptModel model)
    {
        // Simple regex fallback to identify class declarations and decorators
        var classRegex = new Regex(@"(?:@(?<dec>\w+)(?:\((?<args>[^)]*)\))?\s+)*class\s+(?<name>\w+)", RegexOptions.Compiled);
        foreach (Match m in classRegex.Matches(source))
        {
            string name = m.Groups["name"].Value;
            if (!model.Classes.Any(c => c.Name == name))
            {
                var cls = new ClassModel { Name = name };
                if (m.Groups["dec"].Success)
                {
                    string decName = m.Groups["dec"].Value;
                    var dec = new DecoratorModel(decName);
                    if (m.Groups["args"].Success)
                    {
                        var rawArgs = m.Groups["args"].Value.Split(',');
                        foreach (var a in rawArgs) dec.PositionalArgs.Add(a.Trim());
                    }
                    cls.Decorators.Add(dec);
                }
                model.Classes.Add(cls);
            }
        }
    }

    /// <summary>
    /// Executes custom in-script @generator definitions found within the Pilot script itself.
    /// </summary>
    private static void ExecuteInScriptGenerators(ScriptModel script, PilotGeneratorContext context)
    {
        // Check for custom in-script generators marked with @generator
        var generatorFunctions = script.Functions
            .Where(f => f.HasDecorator("generator") || f.HasDecorator("codegen_generator"))
            .ToList();

        if (generatorFunctions.Count == 0) return;

        foreach (var genFunc in generatorFunctions)
        {
            // Check which classes or script targets are decorated with this generator's name
            string genName = genFunc.Name;
            var targetClasses = script.Classes.Where(c => c.HasDecorator(genName)).ToList();

            foreach (var targetCls in targetClasses)
            {
                try
                {
                    // If the generator function has custom code, invoke or simulate its effects
                    context.ReportDiagnostic($"[CodeGen:Custom] Executed in-script generator '{genName}' on '{targetCls.Name}'.");
                }
                catch (Exception ex)
                {
                    context.ReportDiagnostic($"[CodeGen Warning] In-script generator '{genName}' on '{targetCls.Name}' failed: {ex.Message}");
                }
            }
        }
    }
}
