using System;
using System.IO;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Debugging;

/// <summary>
/// Represents a breakpoint with source line location, conditional expression, and hit count thresholds.
/// </summary>
public sealed class PilotBreakpoint
{
    private static int _nextBreakpointId;

    public int Id { get; }
    public string SourceFile { get; }
    public int Line { get; }
    public string? Condition { get; set; }
    public int? HitCountThreshold { get; set; }
    public int HitCount { get; internal set; }
    public bool IsEnabled { get; set; } = true;

    public PilotBreakpoint(string sourceFile, int line, string? condition = null, int? hitCountThreshold = null)
    {
        Id = System.Threading.Interlocked.Increment(ref _nextBreakpointId);
        SourceFile = sourceFile ?? string.Empty;
        Line = line;
        Condition = condition;
        HitCountThreshold = hitCountThreshold;
    }

    /// <summary>
    /// Evaluates whether execution should break at the given stack frame.
    /// </summary>
    public bool ShouldBreak(PilotStackFrame frame, PilotExecutionContext context)
    {
        if (!IsEnabled) return false;
        if (frame.Line != Line) return false;

        // Check source file match if specified
        if (!string.IsNullOrEmpty(SourceFile) && SourceFile != "*")
        {
            if (!string.IsNullOrEmpty(frame.SourceFile))
            {
                string targetFileName = Path.GetFileName(SourceFile);
                string frameFileName = Path.GetFileName(frame.SourceFile);

                bool fileMatches = string.Equals(SourceFile, frame.SourceFile, StringComparison.OrdinalIgnoreCase) ||
                                  string.Equals(targetFileName, frameFileName, StringComparison.OrdinalIgnoreCase);

                if (!fileMatches) return false;
            }
        }

        HitCount++;

        // Hit count condition threshold check
        if (HitCountThreshold.HasValue && HitCount < HitCountThreshold.Value)
        {
            return false;
        }

        // Conditional expression check
        if (!string.IsNullOrWhiteSpace(Condition))
        {
            try
            {
                var condResult = frame.Evaluate(Condition);
                if (!condResult.IsTrue)
                {
                    return false;
                }
            }
            catch
            {
                // If evaluation fails, do not trigger false break
                return false;
            }
        }

        return true;
    }

    public override string ToString() =>
        $"Breakpoint {Id} at {SourceFile}:{Line} (Enabled={IsEnabled}, HitCount={HitCount})";
}
