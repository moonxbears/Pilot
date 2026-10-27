using System;
using System.Collections.Generic;
using Pilot.CodeGen.Model;

namespace Pilot.CodeGen.Abstractions;

/// <summary>
/// Execution context supplied to code generators.
/// Gives full access to the script model, metadata, diagnostics, and output generation.
/// </summary>
public sealed class PilotGeneratorContext
{
    public ScriptModel Script { get; }
    public Dictionary<string, string> GeneratedSources { get; } = new(StringComparer.OrdinalIgnoreCase);
    public List<string> Diagnostics { get; } = new();

    public PilotGeneratorContext(ScriptModel script)
    {
        Script = script ?? throw new ArgumentNullException(nameof(script));
    }

    /// <summary>
    /// Adds a new generated companion source file to the compilation unit.
    /// </summary>
    public void AddSource(string hintName, string source)
    {
        if (string.IsNullOrWhiteSpace(hintName)) return;
        GeneratedSources[hintName] = source ?? string.Empty;
        Script.AddScript(hintName, source ?? string.Empty);
    }

    /// <summary>
    /// Reports an informative diagnostic or warning during code generation.
    /// </summary>
    public void ReportDiagnostic(string message)
    {
        if (!string.IsNullOrWhiteSpace(message))
        {
            Diagnostics.Add(message);
        }
    }
}
