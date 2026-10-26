using System;
using System.Collections.Generic;
using Pilot.Compiler;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Scripting;

/// <summary>
/// Static, high-level scripting API for running and evaluating Pilot code like Python or Lua.
/// Suitable for embedding into C# applications or quick command-line scripting execution.
/// </summary>
public static class PilotScript
{
    /// <summary>
    /// Compiles Pilot source code into an executable program.
    /// </summary>
    public static IPilotProgram Compile(string source, PilotCompilerOptions? options = null)
    {
        var result = PilotCompiler.Default.Compile(source, options);
        if (!result.Success || result.Program == null)
        {
            throw new InvalidOperationException($"Compilation failed: {string.Join("; ", result.Diagnostics)}");
        }
        return result.Program;
    }

    /// <summary>
    /// Compiles a Pilot script file from disk into an executable program.
    /// </summary>
    public static IPilotProgram CompileFile(string filePath, PilotCompilerOptions? options = null)
    {
        var result = PilotCompiler.Default.CompileFile(filePath, options);
        if (!result.Success || result.Program == null)
        {
            throw new InvalidOperationException($"Compilation failed for '{filePath}': {string.Join("; ", result.Diagnostics)}");
        }
        return result.Program;
    }

    /// <summary>
    /// Runs a Pilot code string directly like a script, injecting arguments into global 'args'.
    /// </summary>
    public static PilotExecutionResult Run(string code, params string[] args)
    {
        var program = Compile(code);
        return program.RunWithArgs(args);
    }

    /// <summary>
    /// Runs a Pilot script file from disk directly, passing arguments into global 'args'.
    /// </summary>
    public static PilotExecutionResult RunFile(string filePath, params string[] args)
    {
        // Support running pre-compiled binary (.pilc) directly
        if (filePath.EndsWith(".pilc", StringComparison.OrdinalIgnoreCase))
        {
            var compiled = PilotCompiler.Default.LoadCompiled(filePath);
            return compiled.RunWithArgs(args);
        }

        var program = CompileFile(filePath);
        return program.RunWithArgs(args);
    }

    /// <summary>
    /// Evaluates a single Pilot expression string and returns the resulting PilotValue.
    /// </summary>
    public static PilotValue Evaluate(string expression)
    {
        var engine = new PilotEngine();
        return engine.EvaluateExpression(expression);
    }

    /// <summary>
    /// Evaluates a single Pilot expression string and converts the result to a native .NET type.
    /// </summary>
    public static T? Evaluate<T>(string expression)
    {
        var val = Evaluate(expression);
        return ConvertValue<T>(val);
    }

    private static T? ConvertValue<T>(PilotValue val)
    {
        var targetType = typeof(T);
        if (targetType == typeof(PilotValue)) return (T)(object)val;
        if (targetType == typeof(string)) return (T)(object)val.AsString();
        if (targetType == typeof(int)) return (T)(object)(int)val.AsInt();
        if (targetType == typeof(long)) return (T)(object)val.AsInt();
        if (targetType == typeof(double)) return (T)(object)val.AsFloat();
        if (targetType == typeof(bool)) return (T)(object)val.IsTrue;

        object? foreign = val.AsForeign();
        if (foreign is T tVal) return tVal;

        return (T?)Convert.ChangeType(val.ToString(), targetType);
    }
}
