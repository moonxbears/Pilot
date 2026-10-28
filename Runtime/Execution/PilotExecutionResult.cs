using System;
using System.Collections.Generic;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Execution;

/// <summary>
/// Encapsulates the output and outcome of a Pilot execution run.
/// </summary>
public sealed class PilotExecutionResult
{
    public bool Success { get; }
    public PilotValue Value { get; }
    public string StandardOutput { get; }
    public string StandardError { get; }
    public IReadOnlyList<string> Diagnostics { get; }
    public string ErrorMessage => Diagnostics.Count > 0 ? string.Join(System.Environment.NewLine, Diagnostics) : StandardError;
    public TimeSpan Elapsed { get; }
    public PilotExecutionContext? Context { get; set; }

    public PilotExecutionResult(
        bool success,
        PilotValue value,
        string standardOutput,
        string standardError,
        IReadOnlyList<string> diagnostics,
        TimeSpan elapsed)
    {
        Success = success;
        Value = value;
        StandardOutput = standardOutput;
        StandardError = standardError;
        Diagnostics = diagnostics;
        Elapsed = elapsed;
    }

    public static PilotExecutionResult CreateSuccess(
        PilotValue value,
        string stdout,
        string stderr,
        TimeSpan elapsed) =>
        new(true, value, stdout, stderr, Array.Empty<string>(), elapsed);

    public static PilotExecutionResult CreateFailure(
        string errorMessage,
        string stdout,
        string stderr,
        TimeSpan elapsed) =>
        new(false, PilotValue.Undefined, stdout, stderr, new[] { errorMessage }, elapsed);

    public static PilotExecutionResult CreateFailure(
        IEnumerable<string> errors,
        string stdout,
        string stderr,
        TimeSpan elapsed) =>
        new(false, PilotValue.Undefined, stdout, stderr, new List<string>(errors), elapsed);
}
