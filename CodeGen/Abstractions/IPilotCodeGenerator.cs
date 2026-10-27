using System;

namespace Pilot.CodeGen.Abstractions;

/// <summary>
/// Interface implemented by Pilot code generators.
/// Contrast with Roslyn's complex multi-tiered interfaces: simple, single execute method.
/// </summary>
public interface IPilotCodeGenerator
{
    /// <summary>
    /// Generator identifier (e.g. "json", "builder", "repr", "observable", "equality").
    /// </summary>
    string Name { get; }

    /// <summary>
    /// Executes code generation against the current script and context.
    /// </summary>
    void Execute(PilotGeneratorContext context);
}
