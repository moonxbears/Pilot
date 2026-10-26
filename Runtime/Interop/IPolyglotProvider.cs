using System.Collections.Generic;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Interop;

/// <summary>
/// Defines a plug-and-play provider for foreign language interoperability (e.g. Python, C#, Rust).
/// </summary>
public interface IPolyglotProvider
{
    /// <summary>
    /// Target language identifier (e.g. "python", "csharp", "rust").
    /// </summary>
    string Language { get; }

    /// <summary>
    /// Executes inline foreign code and returns a PilotValue.
    /// </summary>
    PilotValue ExecuteBlock(string code, PilotExecutionContext context);

    /// <summary>
    /// Invokes a foreign function or member.
    /// </summary>
    PilotValue Invoke(string memberName, IReadOnlyList<PilotValue> args, PilotExecutionContext context);

    /// <summary>
    /// Resolves or imports a foreign module or type.
    /// </summary>
    PilotValue Import(string modulePath, string? alias);
}
