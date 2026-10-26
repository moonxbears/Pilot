using System;
using System.Collections.Generic;

namespace Pilot.Debugging;

/// <summary>
/// Thread-safe collection representing the execution call stack of stack frames.
/// </summary>
public sealed class PilotCallStack
{
    private readonly List<PilotStackFrame> _frames = new();
    private readonly object _lock = new();

    public int Count
    {
        get
        {
            lock (_lock) return _frames.Count;
        }
    }

    public PilotStackFrame? TopFrame
    {
        get
        {
            lock (_lock)
            {
                return _frames.Count > 0 ? _frames[^1] : null;
            }
        }
    }

    public IReadOnlyList<PilotStackFrame> Frames
    {
        get
        {
            lock (_lock)
            {
                // Return copy from top of stack (index 0) to bottom (index Count - 1)
                var list = new List<PilotStackFrame>(_frames);
                list.Reverse();
                return list;
            }
        }
    }

    public void Push(PilotStackFrame frame)
    {
        if (frame == null) throw new ArgumentNullException(nameof(frame));
        lock (_lock)
        {
            _frames.Add(frame);
        }
    }

    public PilotStackFrame? Pop()
    {
        lock (_lock)
        {
            if (_frames.Count == 0) return null;
            var top = _frames[^1];
            _frames.RemoveAt(_frames.Count - 1);
            return top;
        }
    }

    public void Clear()
    {
        lock (_lock)
        {
            _frames.Clear();
        }
    }
}
