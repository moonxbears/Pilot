using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using Pilot.Compiler;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Pilot.Scripting;

namespace Pilot.Runtime.SelectiveExecution;

/// <summary>
/// Engine for selective code execution in Pilot.
/// Enables executing specific functions, tests, or benchmarks without running
/// unrelated top-level execution code.
/// </summary>
public static class PilotSelectiveExecutionEngine
{
    /// <summary>
    /// Selectively executes a single named function within a script file or code string.
    /// </summary>
    public static PilotExecutionResult ExecuteFunction(
        string scriptPathOrCode,
        string functionName,
        IReadOnlyList<string>? args = null,
        PilotExecutionOptions? options = null)
    {
        var sw = Stopwatch.StartNew();
        options ??= new PilotExecutionOptions();

        string source = File.Exists(scriptPathOrCode) ? File.ReadAllText(scriptPathOrCode) : scriptPathOrCode;
        string? filePath = File.Exists(scriptPathOrCode) ? Path.GetFullPath(scriptPathOrCode) : null;

        var compilerOptions = new PilotCompilerOptions().WithSourceFile(filePath ?? "snippet.pilot");
        var compilation = PilotCompiler.Default.Compile(source, compilerOptions);
        if (!compilation.Success || compilation.Program == null)
        {
            return PilotExecutionResult.CreateFailure(compilation.Diagnostics, "", "", sw.Elapsed);
        }

        // Configure context with selective execution markers
        var engine = new PilotEngine();
        var scope = new PilotScope(scopeName: "GlobalScope");
        scope.Declare("__name__", PilotValue.FromString(functionName));
        scope.Declare("__selective__", PilotValue.FromString(functionName));

        var execCtx = new PilotExecutionContext(engine, options, new PilotEnvironment(scope));
        PilotScriptContext.Inject(execCtx, args, filePath);

        // Run program to populate definitions
        var initResult = compilation.Program.Run(execCtx);
        if (!initResult.Success)
        {
            return initResult;
        }

        string stdout = options.StandardOutput?.ToString() ?? string.Empty;
        string stderr = options.StandardError?.ToString() ?? string.Empty;

        // Resolve target function
        if (!scope.TryResolve(functionName, out var funcVal) || funcVal.Kind != PilotValueKind.Function)
        {
            return PilotExecutionResult.CreateFailure(
                new[] { $"Target function '{functionName}' not found or is not a callable function." },
                stdout,
                stderr,
                sw.Elapsed);
        }

        var func = funcVal.AsFunction();
        var marshaledArgs = ConvertArgs(args);

        try
        {
            var resultVal = func.Invoke(execCtx, marshaledArgs);
            var success = PilotExecutionResult.CreateSuccess(
                resultVal,
                options.StandardOutput?.ToString() ?? string.Empty,
                options.StandardError?.ToString() ?? string.Empty,
                sw.Elapsed);
            success.Context = execCtx;
            return success;
        }
        catch (Exception ex)
        {
            var failure = PilotExecutionResult.CreateFailure(
                new[] { $"Error executing '{functionName}': {ex.Message}" },
                options.StandardOutput?.ToString() ?? string.Empty,
                options.StandardError?.ToString() ?? string.Empty,
                sw.Elapsed);
            failure.Context = execCtx;
            return failure;
        }
    }

    /// <summary>
    /// Discovers and selectively executes all test functions in the script.
    /// Matches functions decorated with @test or named with prefix 'test_'.
    /// </summary>
    public static PilotTestReport RunTests(
        string scriptPathOrCode,
        string? filter = null,
        PilotExecutionOptions? options = null)
    {
        var swTotal = Stopwatch.StartNew();
        options ??= new PilotExecutionOptions();

        string source = File.Exists(scriptPathOrCode) ? File.ReadAllText(scriptPathOrCode) : scriptPathOrCode;
        string targetName = File.Exists(scriptPathOrCode) ? Path.GetFileName(scriptPathOrCode) : "Snippet";
        var report = new PilotTestReport(targetName);

        var compilerOptions = new PilotCompilerOptions();
        var compilation = PilotCompiler.Default.Compile(source, compilerOptions);
        if (!compilation.Success || compilation.Program == null)
        {
            report.Results.Add(new PilotTestCaseResult(
                "Compilation",
                swTotal.Elapsed,
                false,
                $"Compilation error: {string.Join("; ", compilation.Diagnostics)}"));
            report.TotalDuration = swTotal.Elapsed;
            return report;
        }

        // Initialize script definitions with selective test marker
        var engine = new PilotEngine();
        var scope = new PilotScope(scopeName: "GlobalScope");
        scope.Declare("__name__", PilotValue.FromString("__test__"));
        scope.Declare("__selective__", PilotValue.FromString("test"));

        var execCtx = new PilotExecutionContext(engine, options, new PilotEnvironment(scope));
        var initResult = compilation.Program.Run(execCtx);
        if (!initResult.Success)
        {
            report.Results.Add(new PilotTestCaseResult(
                "Initialization",
                swTotal.Elapsed,
                false,
                $"Init error: {string.Join("; ", initResult.Diagnostics)}"));
            report.TotalDuration = swTotal.Elapsed;
            return report;
        }

        // Extract candidate test functions
        var scriptModel = CodeGen.Engine.PilotCodeGeneratorPipeline.ExtractScriptModel(source);
        var testFunctions = scriptModel.Functions.Where(f =>
            f.HasDecorator("test") ||
            f.Name.StartsWith("test_", StringComparison.OrdinalIgnoreCase) ||
            f.Name.EndsWith("_test", StringComparison.OrdinalIgnoreCase)).ToList();

        if (testFunctions.Count == 0)
        {
            // Also inspect scope directly for functions starting with test_
            foreach (var kvp in scope.GetLocalBindings())
            {
                if (kvp.Value.Value.Kind == PilotValueKind.Function &&
                    kvp.Key.StartsWith("test_", StringComparison.OrdinalIgnoreCase))
                {
                    testFunctions.Add(new CodeGen.Model.MethodModel { Name = kvp.Key });
                }
            }
        }

        if (!string.IsNullOrEmpty(filter))
        {
            var regex = new Regex(filter, RegexOptions.IgnoreCase);
            testFunctions = testFunctions.Where(f => regex.IsMatch(f.Name)).ToList();
        }

        foreach (var tf in testFunctions)
        {
            if (!scope.TryResolve(tf.Name, out var fnVal) || fnVal.Kind != PilotValueKind.Function)
            {
                continue;
            }

            var func = fnVal.AsFunction();
            var sw = Stopwatch.StartNew();
            try
            {
                func.Invoke(execCtx, Array.Empty<PilotValue>());
                sw.Stop();
                report.Results.Add(new PilotTestCaseResult(tf.Name, sw.Elapsed, true));
            }
            catch (Exception ex)
            {
                sw.Stop();
                string msg = ex.InnerException?.Message ?? ex.Message;
                report.Results.Add(new PilotTestCaseResult(tf.Name, sw.Elapsed, false, msg, ex));
            }
        }

        report.TotalDuration = swTotal.Elapsed;
        return report;
    }

    /// <summary>
    /// Discovers and selectively benchmarks performance of functions decorated with @bench / @benchmark
    /// or prefixed with 'bench_'.
    /// </summary>
    public static PilotBenchmarkReport RunBenchmarks(
        string scriptPathOrCode,
        string? filter = null,
        int defaultIterations = 100,
        int warmupIterations = 5,
        PilotExecutionOptions? options = null)
    {
        var swTotal = Stopwatch.StartNew();
        options ??= new PilotExecutionOptions();

        string source = File.Exists(scriptPathOrCode) ? File.ReadAllText(scriptPathOrCode) : scriptPathOrCode;
        string targetName = File.Exists(scriptPathOrCode) ? Path.GetFileName(scriptPathOrCode) : "Snippet";
        var report = new PilotBenchmarkReport(targetName);

        var compilerOptions = new PilotCompilerOptions();
        var compilation = PilotCompiler.Default.Compile(source, compilerOptions);
        if (!compilation.Success || compilation.Program == null)
        {
            report.TotalDuration = swTotal.Elapsed;
            return report;
        }

        // Initialize script definitions with selective benchmark marker
        var engine = new PilotEngine();
        var scope = new PilotScope(scopeName: "GlobalScope");
        scope.Declare("__name__", PilotValue.FromString("__bench__"));
        scope.Declare("__selective__", PilotValue.FromString("bench"));

        var execCtx = new PilotExecutionContext(engine, options, new PilotEnvironment(scope));
        var initResult = compilation.Program.Run(execCtx);
        if (!initResult.Success)
        {
            report.TotalDuration = swTotal.Elapsed;
            return report;
        }

        var scriptModel = CodeGen.Engine.PilotCodeGeneratorPipeline.ExtractScriptModel(source);
        var benchFunctions = scriptModel.Functions.Where(f =>
            f.HasDecorator("bench") ||
            f.HasDecorator("benchmark") ||
            f.Name.StartsWith("bench_", StringComparison.OrdinalIgnoreCase) ||
            f.Name.StartsWith("benchmark_", StringComparison.OrdinalIgnoreCase)).ToList();

        if (benchFunctions.Count == 0)
        {
            foreach (var kvp in scope.GetLocalBindings())
            {
                if (kvp.Value.Value.Kind == PilotValueKind.Function &&
                    (kvp.Key.StartsWith("bench_", StringComparison.OrdinalIgnoreCase) ||
                     kvp.Key.StartsWith("benchmark_", StringComparison.OrdinalIgnoreCase)))
                {
                    benchFunctions.Add(new CodeGen.Model.MethodModel { Name = kvp.Key });
                }
            }
        }

        if (!string.IsNullOrEmpty(filter))
        {
            var regex = new Regex(filter, RegexOptions.IgnoreCase);
            benchFunctions = benchFunctions.Where(f => regex.IsMatch(f.Name)).ToList();
        }

        foreach (var bf in benchFunctions)
        {
            if (!scope.TryResolve(bf.Name, out var fnVal) || fnVal.Kind != PilotValueKind.Function)
            {
                continue;
            }

            var func = fnVal.AsFunction();

            // Extract iterations override if provided in decorator e.g. @benchmark(iterations=500)
            int iterations = defaultIterations;
            var dec = bf.GetDecorator("benchmark") ?? bf.GetDecorator("bench");
            if (dec != null)
            {
                string? itStr = dec.GetNamed("iterations") ?? dec.GetArg(0);
                if (itStr != null && int.TryParse(itStr, out int customIt) && customIt > 0)
                {
                    iterations = customIt;
                }
            }

            // Warmup phase
            for (int w = 0; w < warmupIterations; w++)
            {
                func.Invoke(execCtx, Array.Empty<PilotValue>());
            }

            // Measured phase
            var sw = Stopwatch.StartNew();
            for (int i = 0; i < iterations; i++)
            {
                func.Invoke(execCtx, Array.Empty<PilotValue>());
            }
            sw.Stop();

            report.Results.Add(new PilotBenchmarkResult(bf.Name, iterations, sw.Elapsed));
        }

        report.TotalDuration = swTotal.Elapsed;
        return report;
    }

    private static IReadOnlyList<PilotValue> ConvertArgs(IReadOnlyList<string>? args)
    {
        if (args == null || args.Count == 0) return Array.Empty<PilotValue>();

        var list = new PilotValue[args.Count];
        for (int i = 0; i < args.Count; i++)
        {
            string s = args[i];
            if (long.TryParse(s, out long lVal)) list[i] = PilotValue.FromInt(lVal);
            else if (double.TryParse(s, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out double dVal)) list[i] = PilotValue.FromFloat(dVal);
            else if (bool.TryParse(s, out bool bVal)) list[i] = PilotValue.FromBool(bVal);
            else list[i] = PilotValue.FromString(s);
        }
        return list;
    }
}
