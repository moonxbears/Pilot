using System;
using System.Collections.Generic;

namespace Pilot.CodeGen.Abstractions;

/// <summary>
/// Result produced by the Pilot Code Generator Pipeline.
/// Contains the final augmented source code, generated companion files, and diagnostics.
/// </summary>
public sealed class PilotGeneratorResult
{
    public string AugmentedSource { get; }
    public IReadOnlyDictionary<string, string> GeneratedSources { get; }
    public IReadOnlyList<string> Diagnostics { get; }
    public bool HasModifications { get; }

    public PilotGeneratorResult(
        string augmentedSource,
        IReadOnlyDictionary<string, string> generatedSources,
        IReadOnlyList<string> diagnostics,
        bool hasModifications)
    {
        AugmentedSource = augmentedSource;
        GeneratedSources = generatedSources;
        Diagnostics = diagnostics;
        HasModifications = hasModifications;
    }

    public static PilotGeneratorResult Unmodified(string source) =>
        new(source, new Dictionary<string, string>(), Array.Empty<string>(), false);
}
