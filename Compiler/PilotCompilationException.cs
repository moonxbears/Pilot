using System;

namespace Pilot.Compiler;

/// <summary>
/// Exception thrown when compilation encounters a semantic or static error.
/// </summary>
public sealed class PilotCompilationException : Exception
{
    public int Line { get; }
    public int Column { get; }

    public PilotCompilationException(string message, int line = 0, int column = 0)
        : base(line > 0 ? $"[Line {line}:{column}] {message}" : message)
    {
        Line = line;
        Column = column;
    }
}
