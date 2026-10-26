using System;
using System.Collections.Generic;

namespace Pilot.Compiler;

/// <summary>
/// Target representation for compilation.
/// </summary>
public enum CompilationTarget
{
    /// <summary>
    /// Parsed and pre-validated AST tree ready for immediate, high-performance visitor execution.
    /// </summary>
    ExecutableAst,

    /// <summary>
    /// Compact intermediate bytecode format.
    /// </summary>
    Bytecode,

    /// <summary>
    /// Native .NET dynamic assembly or compiled delegate via transpilation.
    /// </summary>
    DotNetAssembly
}

/// <summary>
/// Configuration options for the Pilot compiler.
/// Fully configurable, decoupled, and flexible.
/// </summary>
public sealed class PilotCompilerOptions
{
    /// <summary>
    /// Target execution format.
    /// </summary>
    public CompilationTarget Target { get; set; } = CompilationTarget.ExecutableAst;

    /// <summary>
    /// Whether to run the preprocessor before compilation.
    /// </summary>
    public bool EnablePreprocessing { get; set; } = true;

    /// <summary>
    /// Conditional symbols defined for the preprocessor.
    /// </summary>
    public List<string> DefinedSymbols { get; } = new();

    /// <summary>
    /// Whether strict compilation mode is enabled.
    /// </summary>
    public bool StrictMode { get; set; } = false;

    /// <summary>
    /// Whether to enable compiler optimizations.
    /// </summary>
    public bool EnableOptimization { get; set; } = true;

    /// <summary>
    /// Optional file path of the source code being compiled.
    /// </summary>
    public string? SourceFilePath { get; set; }

    /// <summary>
    /// Whether to cache compiled programs in memory.
    /// </summary>
    public bool EnableCaching { get; set; } = true;

    public PilotCompilerOptions() { }

    public PilotCompilerOptions(IEnumerable<string>? symbols)
    {
        if (symbols != null)
        {
            foreach (var s in symbols) WithSymbol(s);
        }
    }

    public PilotCompilerOptions WithSymbol(string symbol)
    {
        if (!string.IsNullOrWhiteSpace(symbol) && !DefinedSymbols.Contains(symbol))
        {
            DefinedSymbols.Add(symbol.Trim());
        }
        return this;
    }

    public PilotCompilerOptions WithTarget(CompilationTarget target)
    {
        Target = target;
        return this;
    }

    public PilotCompilerOptions WithSourceFile(string path)
    {
        SourceFilePath = path;
        return this;
    }

    public PilotCompilerOptions WithStrictMode(bool strict = true)
    {
        StrictMode = strict;
        return this;
    }

    public PilotCompilerOptions WithOptimization(bool optimize = true)
    {
        EnableOptimization = optimize;
        return this;
    }
}
