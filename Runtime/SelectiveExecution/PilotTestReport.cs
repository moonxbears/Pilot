using System;
using System.Collections.Generic;

namespace Pilot.Runtime.SelectiveExecution;

/// <summary>
/// Individual result for a selectively executed test case.
/// </summary>
public sealed class PilotTestCaseResult
{
    public string Name { get; }
    public string TestName => Name;
    public TimeSpan Duration { get; }
    public bool Success { get; }
    public bool Passed => Success;
    public string? ErrorMessage { get; }
    public Exception? Exception { get; }

    public PilotTestCaseResult(string name, TimeSpan duration, bool success, string? errorMessage = null, Exception? exception = null)
    {
        Name = name;
        Duration = duration;
        Success = success;
        ErrorMessage = errorMessage;
        Exception = exception;
    }
}

/// <summary>
/// Aggregated report from a selective test suite run.
/// </summary>
public sealed class PilotTestReport
{
    public string TargetFile { get; }
    public List<PilotTestCaseResult> Results { get; } = new();
    public TimeSpan TotalDuration { get; set; }

    public int PassedCount => Results.Count(r => r.Success);
    public int FailedCount => Results.Count(r => !r.Success);
    public int TotalCount => Results.Count;

    public PilotTestReport(string targetFile)
    {
        TargetFile = targetFile;
    }
}
