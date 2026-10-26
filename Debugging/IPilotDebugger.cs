using System;
using System.Collections.Generic;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Debugging;

/// <summary>
/// Primary plug-and-play contract for debugging Pilot scripts and applications.
/// Supports breakpoints, step execution, call stacks, locals inspection, and interactive evaluation.
/// </summary>
public interface IPilotDebugger
{
    /// <summary>
    /// Current execution state of the debugger.
    /// </summary>
    DebuggerState State { get; }

    /// <summary>
    /// Active stepping mode.
    /// </summary>
    DebuggerStepMode StepMode { get; }

    /// <summary>
    /// The active call stack of frames.
    /// </summary>
    PilotCallStack CallStack { get; }

    /// <summary>
    /// Gets the current top active stack frame.
    /// </summary>
    PilotStackFrame? CurrentFrame { get; }

    /// <summary>
    /// Gets all registered breakpoints.
    /// </summary>
    IReadOnlyList<PilotBreakpoint> Breakpoints { get; }

    /// <summary>
    /// Fired when the debugger pauses execution (e.g. at a breakpoint, step complete, or exception).
    /// </summary>
    event EventHandler<DebuggerPauseEventArgs>? Paused;

    /// <summary>
    /// Fired when execution resumes from a paused state.
    /// </summary>
    event EventHandler? Resumed;

    /// <summary>
    /// Fired when a function or root module scope is entered.
    /// </summary>
    event EventHandler<PilotStackFrame>? FrameEntered;

    /// <summary>
    /// Fired when a function or root module scope exits.
    /// </summary>
    event EventHandler<(PilotStackFrame Frame, PilotValue ReturnValue)>? FrameExited;

    /// <summary>
    /// Sets a breakpoint on a specific source line with optional condition or hit threshold.
    /// </summary>
    PilotBreakpoint SetBreakpoint(string sourceFile, int line, string? condition = null, int? hitCountThreshold = null);

    /// <summary>
    /// Removes a breakpoint by its unique ID.
    /// </summary>
    bool RemoveBreakpoint(int breakpointId);

    /// <summary>
    /// Clears all registered breakpoints.
    /// </summary>
    void ClearBreakpoints();

    /// <summary>
    /// Resumes normal execution until the next breakpoint or program completion.
    /// </summary>
    void Continue();

    /// <summary>
    /// Steps into the next statement, following into function calls.
    /// </summary>
    void StepInto();

    /// <summary>
    /// Steps over the next statement, skipping inner function execution.
    /// </summary>
    void StepOver();

    /// <summary>
    /// Steps out of the current function to its caller.
    /// </summary>
    void StepOut();

    /// <summary>
    /// Requests an asynchronous pause at the next executed statement.
    /// </summary>
    void Pause();

    /// <summary>
    /// Aborts execution immediately.
    /// </summary>
    void Stop();

    /// <summary>
    /// Gets the local variables of the stack frame at the specified depth (0 = top frame).
    /// </summary>
    IReadOnlyDictionary<string, PilotValue> GetLocals(int frameIndex = 0);

    /// <summary>
    /// Gets all variables visible from the stack frame at the specified depth (0 = top frame).
    /// </summary>
    IReadOnlyDictionary<string, PilotValue> GetAllVariables(int frameIndex = 0);

    /// <summary>
    /// Evaluates an expression inside the scope of the stack frame at the specified depth.
    /// </summary>
    PilotValue Evaluate(string expression, int frameIndex = 0);

    /// <summary>
    /// Modifies a variable in the stack frame at the specified depth.
    /// </summary>
    bool SetVariable(string name, PilotValue value, int frameIndex = 0);

    /// <summary>
    /// Hook invoked by the interpreter before executing each statement.
    /// </summary>
    void BeforeExecuteStatement(int line, int column, PilotExecutionContext context);

    /// <summary>
    /// Hook invoked when entering a function call.
    /// </summary>
    PilotStackFrame OnFunctionEnter(string name, IReadOnlyList<PilotValue> args, PilotScope scope, PilotExecutionContext context);

    /// <summary>
    /// Hook invoked when exiting a function call.
    /// </summary>
    void OnFunctionExit(PilotStackFrame frame, PilotValue returnValue, PilotExecutionContext context);

    /// <summary>
    /// Hook invoked when an exception occurs during execution.
    /// </summary>
    void OnException(Exception ex, PilotExecutionContext context);
}
