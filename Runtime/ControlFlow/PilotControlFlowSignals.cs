using System;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.ControlFlow;

/// <summary>
/// Base class for all internal control flow signals (Return, Break, Continue, Yield).
/// These signals bypass normal exception catch blocks so that user try/except cannot swallow them.
/// </summary>
public abstract class PilotControlFlowSignal : Exception
{
    protected PilotControlFlowSignal(string message) : base(message) { }
}

/// <summary>
/// Control flow signal for function return statements.
/// </summary>
public sealed class PilotReturnSignal : PilotControlFlowSignal
{
    public PilotValue Value { get; }

    public PilotReturnSignal(PilotValue value) : base("Return signal")
    {
        Value = value;
    }
}

/// <summary>
/// Control flow signal for loop break statements (supports optional label).
/// </summary>
public sealed class PilotBreakSignal : PilotControlFlowSignal
{
    public string? Label { get; }

    public PilotBreakSignal(string? label = null) : base("Break signal")
    {
        Label = label;
    }
}

/// <summary>
/// Control flow signal for loop continue statements (supports optional label).
/// </summary>
public sealed class PilotContinueSignal : PilotControlFlowSignal
{
    public string? Label { get; }

    public PilotContinueSignal(string? label = null) : base("Continue signal")
    {
        Label = label;
    }
}

/// <summary>
/// Control flow signal thrown when a generator executes a yield statement.
/// </summary>
public sealed class PilotYieldSignal : PilotControlFlowSignal
{
    public PilotValue Value { get; }

    public PilotYieldSignal(PilotValue value) : base("Yield signal")
    {
        Value = value;
    }
}

/// <summary>
/// Base exception thrown when a Pilot runtime error occurs during script evaluation.
/// </summary>
public class PilotRuntimeException : Exception
{
    public string TypeName { get; }
    public int Line { get; }
    public int Column { get; }

    public PilotRuntimeException(string message, string typeName = "RuntimeError", int line = 0, int column = 0, Exception? inner = null)
        : base(line > 0 ? $"[{typeName} at line {line}:{column}] {message}" : $"[{typeName}] {message}", inner)
    {
        TypeName = typeName;
        Line = line;
        Column = column;
    }

    public PilotRuntimeException(string message, int line, int column, Exception? inner = null)
        : this(message, "RuntimeError", line, column, inner)
    {
    }
}
