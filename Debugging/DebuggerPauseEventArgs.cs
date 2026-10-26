using System;

namespace Pilot.Debugging;

/// <summary>
/// Event arguments delivered when the debugger pauses execution.
/// </summary>
public sealed class DebuggerPauseEventArgs : EventArgs
{
    public DebuggerPauseReason Reason { get; }
    public PilotStackFrame Frame { get; }
    public PilotBreakpoint? Breakpoint { get; }
    public Exception? Exception { get; }

    public DebuggerPauseEventArgs(
        DebuggerPauseReason reason,
        PilotStackFrame frame,
        PilotBreakpoint? breakpoint = null,
        Exception? exception = null)
    {
        Reason = reason;
        Frame = frame ?? throw new ArgumentNullException(nameof(frame));
        Breakpoint = breakpoint;
        Exception = exception;
    }
}
