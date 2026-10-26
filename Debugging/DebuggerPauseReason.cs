namespace Pilot.Debugging;

/// <summary>
/// Specifies the reason why the debugger paused execution.
/// </summary>
public enum DebuggerPauseReason
{
    None,
    Breakpoint,
    Step,
    Pause,
    Exception
}
