using System;
using System.Collections.Generic;
using System.Threading;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Pilot.Scripting;

namespace Pilot.Debugging;

/// <summary>
/// High-performance, plug-and-play interactive debugger for the Pilot language.
/// Provides line/conditional breakpoints, step execution (Over/Into/Out), locals inspection, and call stack unwinding.
/// </summary>
public sealed class PilotDebugger : IPilotDebugger
{
    private readonly List<PilotBreakpoint> _breakpoints = new();
    private readonly object _lock = new();
    private readonly ManualResetEventSlim _resumeEvent = new(false);

    private int _stepTargetDepth;
    private bool _pauseRequested;

    public DebuggerState State { get; private set; } = DebuggerState.Running;
    public DebuggerStepMode StepMode { get; private set; } = DebuggerStepMode.Continue;
    public PilotCallStack CallStack { get; } = new();

    /// <summary>
    /// When true, execution blocks on thread wait handles when pausing (suitable for CLI or DAP).
    /// When false, execution invokes the Paused event synchronously on the running thread (suitable for automated tests).
    /// </summary>
    public bool WaitOnPause { get; set; } = false;

    public PilotStackFrame? CurrentFrame => CallStack.TopFrame;

    public IReadOnlyList<PilotBreakpoint> Breakpoints
    {
        get
        {
            lock (_lock) return new List<PilotBreakpoint>(_breakpoints);
        }
    }

    public event EventHandler<DebuggerPauseEventArgs>? Paused;
    public event EventHandler? Resumed;
    public event EventHandler<PilotStackFrame>? FrameEntered;
    public event EventHandler<(PilotStackFrame Frame, PilotValue ReturnValue)>? FrameExited;

    public PilotBreakpoint SetBreakpoint(string sourceFile, int line, string? condition = null, int? hitCountThreshold = null)
    {
        lock (_lock)
        {
            var bp = new PilotBreakpoint(sourceFile, line, condition, hitCountThreshold);
            _breakpoints.Add(bp);
            return bp;
        }
    }

    public bool RemoveBreakpoint(int breakpointId)
    {
        lock (_lock)
        {
            int idx = _breakpoints.FindIndex(b => b.Id == breakpointId);
            if (idx >= 0)
            {
                _breakpoints.RemoveAt(idx);
                return true;
            }
            return false;
        }
    }

    public void ClearBreakpoints()
    {
        lock (_lock)
        {
            _breakpoints.Clear();
        }
    }

    public void Continue()
    {
        StepMode = DebuggerStepMode.Continue;
        ResumeExecution();
    }

    public void StepInto()
    {
        StepMode = DebuggerStepMode.StepInto;
        ResumeExecution();
    }

    public void StepOver()
    {
        StepMode = DebuggerStepMode.StepOver;
        _stepTargetDepth = CallStack.Count;
        ResumeExecution();
    }

    public void StepOut()
    {
        StepMode = DebuggerStepMode.StepOut;
        _stepTargetDepth = CallStack.Count;
        ResumeExecution();
    }

    public void Pause()
    {
        _pauseRequested = true;
    }

    public void Stop()
    {
        State = DebuggerState.Terminated;
        _resumeEvent.Set();
        throw new PilotScriptExitException(1);
    }

    private void ResumeExecution()
    {
        State = DebuggerState.Running;
        _resumeEvent.Set();
        Resumed?.Invoke(this, EventArgs.Empty);
    }

    public IReadOnlyDictionary<string, PilotValue> GetLocals(int frameIndex = 0)
    {
        var frames = CallStack.Frames;
        if (frameIndex >= 0 && frameIndex < frames.Count)
        {
            return frames[frameIndex].GetLocals();
        }
        return new Dictionary<string, PilotValue>();
    }

    public IReadOnlyDictionary<string, PilotValue> GetAllVariables(int frameIndex = 0)
    {
        var frames = CallStack.Frames;
        if (frameIndex >= 0 && frameIndex < frames.Count)
        {
            return frames[frameIndex].GetAllVariables();
        }
        return new Dictionary<string, PilotValue>();
    }

    public PilotValue Evaluate(string expression, int frameIndex = 0)
    {
        var frames = CallStack.Frames;
        if (frameIndex >= 0 && frameIndex < frames.Count)
        {
            return frames[frameIndex].Evaluate(expression);
        }
        return PilotValue.Undefined;
    }

    public bool SetVariable(string name, PilotValue value, int frameIndex = 0)
    {
        var frames = CallStack.Frames;
        if (frameIndex >= 0 && frameIndex < frames.Count)
        {
            return frames[frameIndex].SetVariable(name, value);
        }
        return false;
    }

    public void BeforeExecuteStatement(int line, int column, PilotExecutionContext context)
    {
        if (State == DebuggerState.Terminated) return;

        var frame = CallStack.TopFrame;
        if (frame != null)
        {
            frame.Line = line;
            frame.Column = column;
        }

        // 1. Check for breakpoint hit
        PilotBreakpoint? hitBreakpoint = null;
        if (frame != null)
        {
            lock (_lock)
            {
                foreach (var bp in _breakpoints)
                {
                    if (bp.ShouldBreak(frame, context))
                    {
                        hitBreakpoint = bp;
                        break;
                    }
                }
            }
        }

        if (hitBreakpoint != null)
        {
            TriggerPause(DebuggerPauseReason.Breakpoint, frame!, hitBreakpoint);
            return;
        }

        // 2. Check for manual pause
        if (_pauseRequested)
        {
            _pauseRequested = false;
            TriggerPause(DebuggerPauseReason.Pause, frame ?? CreateFallbackFrame(context, line, column));
            return;
        }

        // 3. Check for step modes
        if (StepMode == DebuggerStepMode.StepInto)
        {
            TriggerPause(DebuggerPauseReason.Step, frame ?? CreateFallbackFrame(context, line, column));
        }
        else if (StepMode == DebuggerStepMode.StepOver)
        {
            if (CallStack.Count <= _stepTargetDepth)
            {
                TriggerPause(DebuggerPauseReason.Step, frame ?? CreateFallbackFrame(context, line, column));
            }
        }
        else if (StepMode == DebuggerStepMode.StepOut)
        {
            if (CallStack.Count < _stepTargetDepth)
            {
                TriggerPause(DebuggerPauseReason.Step, frame ?? CreateFallbackFrame(context, line, column));
            }
        }
    }

    private void TriggerPause(
        DebuggerPauseReason reason,
        PilotStackFrame frame,
        PilotBreakpoint? breakpoint = null,
        Exception? exception = null)
    {
        State = DebuggerState.Paused;
        var args = new DebuggerPauseEventArgs(reason, frame, breakpoint, exception);

        Paused?.Invoke(this, args);

        if (WaitOnPause)
        {
            _resumeEvent.Reset();
            _resumeEvent.Wait();
        }
    }

    public PilotStackFrame OnFunctionEnter(
        string name,
        IReadOnlyList<PilotValue> args,
        PilotScope scope,
        PilotExecutionContext context)
    {
        string sourceFile = context.Options.DefinedSymbols.Count > 0 ? string.Empty : string.Empty;
        var frame = new PilotStackFrame(name, sourceFile, 0, 0, scope, context, args);
        CallStack.Push(frame);
        FrameEntered?.Invoke(this, frame);
        return frame;
    }

    public void OnFunctionExit(
        PilotStackFrame frame,
        PilotValue returnValue,
        PilotExecutionContext context)
    {
        CallStack.Pop();
        FrameExited?.Invoke(this, (frame, returnValue));

        // If stepping out and now at caller level, pause
        if (StepMode == DebuggerStepMode.StepOut && CallStack.Count < _stepTargetDepth)
        {
            var top = CallStack.TopFrame;
            if (top != null)
            {
                TriggerPause(DebuggerPauseReason.Step, top);
            }
        }
    }

    public void OnException(Exception ex, PilotExecutionContext context)
    {
        if (State == DebuggerState.Terminated) return;

        var frame = CallStack.TopFrame ?? CreateFallbackFrame(context, 0, 0);
        TriggerPause(DebuggerPauseReason.Exception, frame, null, ex);
    }

    private PilotStackFrame CreateFallbackFrame(PilotExecutionContext context, int line, int column)
    {
        return new PilotStackFrame("<global>", string.Empty, line, column, context.Environment.CurrentScope, context);
    }
}
