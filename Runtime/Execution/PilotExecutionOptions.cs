using System;
using System.Collections.Generic;
using System.IO;
using Pilot.Runtime.Builtins;
using Pilot.Runtime.Interop;

namespace Pilot.Runtime.Execution;

/// <summary>
/// Execution configuration options for the Pilot engine.
/// Highly configurable, flexible, and decoupled.
/// </summary>
public sealed class PilotExecutionOptions
{
    public TextWriter StandardOutput { get; set; } = Console.Out;
    public TextWriter StandardError { get; set; } = Console.Error;
    public TextReader StandardInput { get; set; } = Console.In;

    public int MaxCallDepth { get; set; } = 1000;
    public TimeSpan? ExecutionTimeout { get; set; } = null;
    public bool EnablePreprocessing { get; set; } = true;
    public List<string> DefinedSymbols { get; } = new();

    public PilotBuiltinRegistry Builtins { get; set; } = new();
    public PolyglotRegistry Polyglot { get; set; } = new();

    public bool StrictMode { get; set; } = false;

    /// <summary>
    /// Optional source file path for relative module resolution and diagnostics.
    /// </summary>
    public string? SourceFilePath { get; set; }

    /// <summary>
    /// Optional initial scope to execute code within.
    /// </summary>
    public Pilot.Runtime.Environment.PilotScope? InitialScope { get; set; }

    /// <summary>
    /// Optional attached debugger session.
    /// </summary>
    public Pilot.Debugging.IPilotDebugger? Debugger { get; set; }

    public PilotExecutionOptions()
    {
    }

    public PilotExecutionOptions WithDebugger(Pilot.Debugging.IPilotDebugger debugger)
    {
        Debugger = debugger;
        return this;
    }

    public PilotExecutionOptions WithStandardOutput(TextWriter writer)
    {
        StandardOutput = writer ?? throw new ArgumentNullException(nameof(writer));
        return this;
    }

    public PilotExecutionOptions WithStandardError(TextWriter writer)
    {
        StandardError = writer ?? throw new ArgumentNullException(nameof(writer));
        return this;
    }

    public PilotExecutionOptions WithSymbol(string symbol)
    {
        if (!string.IsNullOrWhiteSpace(symbol) && !DefinedSymbols.Contains(symbol))
        {
            DefinedSymbols.Add(symbol);
        }
        return this;
    }

    public PilotExecutionOptions WithTimeout(TimeSpan timeout)
    {
        ExecutionTimeout = timeout;
        return this;
    }

    public PilotExecutionOptions WithMaxCallDepth(int maxDepth)
    {
        MaxCallDepth = maxDepth;
        return this;
    }

    public PilotExecutionOptions WithStrictMode(bool strict = true)
    {
        StrictMode = strict;
        return this;
    }
}
