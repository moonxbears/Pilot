using System;
using System.Collections.Generic;

namespace Pilot.Runtime.SelectiveExecution;

/// <summary>
/// Individual result for a selectively executed benchmark function.
/// </summary>
public sealed class PilotBenchmarkResult
{
    public string Name { get; }
    public string BenchmarkName => Name;
    public int Iterations { get; }
    public TimeSpan TotalTime { get; }
    public TimeSpan MeanTime => Iterations > 0 ? TotalTime / Iterations : TimeSpan.Zero;
    public double OpsPerSecond => TotalTime.TotalSeconds > 0 ? Iterations / TotalTime.TotalSeconds : 0.0;

    public PilotBenchmarkResult(string name, int iterations, TimeSpan totalTime)
    {
        Name = name;
        Iterations = iterations;
        TotalTime = totalTime;
    }

    public string FormatMeanTime()
    {
        double ms = MeanTime.TotalMilliseconds;
        if (ms < 0.001) return $"{ms * 1_000_000:F2} ns";
        if (ms < 1.0) return $"{ms * 1_000:F2} µs";
        return $"{ms:F3} ms";
    }
}

/// <summary>
/// Aggregated report from a selective benchmark run.
/// </summary>
public sealed class PilotBenchmarkReport
{
    public string TargetFile { get; }
    public List<PilotBenchmarkResult> Results { get; } = new();
    public TimeSpan TotalDuration { get; set; }

    public PilotBenchmarkReport(string targetFile)
    {
        TargetFile = targetFile;
    }
}
