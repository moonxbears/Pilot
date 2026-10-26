using System;
using System.Collections.Generic;

namespace Pilot.Compiler;

/// <summary>
/// Encapsulates the output and diagnostics of a Pilot compilation operation.
/// </summary>
public sealed class PilotCompilationResult
{
    public bool Success { get; }
    public IPilotProgram? Program { get; }
    public IReadOnlyList<string> Diagnostics { get; }
    public TimeSpan Elapsed { get; }

    private PilotCompilationResult(
        bool success,
        IPilotProgram? program,
        IReadOnlyList<string> diagnostics,
        TimeSpan elapsed)
    {
        Success = success;
        Program = program;
        Diagnostics = diagnostics;
        Elapsed = elapsed;
    }

    public static PilotCompilationResult CreateSuccess(IPilotProgram program, TimeSpan elapsed)
    {
        return new PilotCompilationResult(true, program, Array.Empty<string>(), elapsed);
    }

    public static PilotCompilationResult CreateFailure(IEnumerable<string> diagnostics, TimeSpan elapsed)
    {
        return new PilotCompilationResult(false, null, new List<string>(diagnostics), elapsed);
    }

    public static PilotCompilationResult CreateFailure(string diagnostic, TimeSpan elapsed)
    {
        return new PilotCompilationResult(false, null, new[] { diagnostic }, elapsed);
    }
}
