using System;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.ControlFlow;

/// <summary>
/// Domain-specific type mismatch or invalid operation error in Pilot.
/// </summary>
public class PilotTypeException : PilotRuntimeException
{
    public PilotTypeException(string message, int line = 0, int column = 0, Exception? inner = null)
        : base(message, "TypeError", line, column, inner) { }
}

/// <summary>
/// Thrown when an arithmetic division or modulo by zero is attempted.
/// </summary>
public class PilotDivideByZeroException : PilotRuntimeException
{
    public PilotDivideByZeroException(string message = "Division by zero.", int line = 0, int column = 0, Exception? inner = null)
        : base(message, "DivideByZeroError", line, column, inner) { }
}

/// <summary>
/// Thrown when an index is outside the bounds of a collection or string.
/// </summary>
public class PilotIndexOutOfRangeException : PilotRuntimeException
{
    public PilotIndexOutOfRangeException(string message, int line = 0, int column = 0, Exception? inner = null)
        : base(message, "IndexError", line, column, inner) { }
}

/// <summary>
/// Thrown when a specified key is not found in a map or dictionary.
/// </summary>
public class PilotKeyNotFoundException : PilotRuntimeException
{
    public PilotKeyNotFoundException(string message, int line = 0, int column = 0, Exception? inner = null)
        : base(message, "KeyError", line, column, inner) { }
}

/// <summary>
/// Thrown when dereferencing null or undefined without safe navigation (?. or ?[]).
/// </summary>
public class PilotNullReferenceException : PilotRuntimeException
{
    public PilotNullReferenceException(string message = "Null reference encountered.", int line = 0, int column = 0, Exception? inner = null)
        : base(message, "NullReferenceError", line, column, inner) { }
}

/// <summary>
/// Thrown when an assertion condition evaluates to false.
/// </summary>
public class PilotAssertionException : PilotRuntimeException
{
    public PilotAssertionException(string message = "Assertion failed.", int line = 0, int column = 0, Exception? inner = null)
        : base(message, "AssertionError", line, column, inner) { }
}

/// <summary>
/// Thrown when maximum call depth or recursion depth is exceeded.
/// </summary>
public class PilotRecursionException : PilotRuntimeException
{
    public PilotRecursionException(string message = "Maximum call depth exceeded.", int line = 0, int column = 0, Exception? inner = null)
        : base(message, "RecursionError", line, column, inner) { }
}

/// <summary>
/// Thrown when an interface or contract requirement cannot be fulfilled by a substitute.
/// </summary>
public class PilotContractException : PilotRuntimeException
{
    public PilotContractException(string message, int line = 0, int column = 0, Exception? inner = null)
        : base(message, "ContractError", line, column, inner) { }
}

/// <summary>
/// Thrown when an asynchronous execution, future, or concurrent operation times out.
/// </summary>
public class PilotTimeoutException : PilotRuntimeException
{
    public PilotTimeoutException(string message = "Operation timed out.", int line = 0, int column = 0, Exception? inner = null)
        : base(message, "TimeoutError", line, column, inner) { }
}

/// <summary>
/// Thrown when a destructuring pattern assignment or exhaustive match fails to match the required structure.
/// </summary>
public class PilotPatternMatchingException : PilotRuntimeException
{
    public PilotPatternMatchingException(string message = "Pattern matching assignment failed.", int line = 0, int column = 0, Exception? inner = null)
        : base(message, "MatchError", line, column, inner) { }
}

/// <summary>
/// Thrown when a contextual parameter cannot be resolved from the caller environment.
/// </summary>
public class PilotContextException : PilotRuntimeException
{
    public PilotContextException(string message = "Contextual parameter resolution failed.", int line = 0, int column = 0, Exception? inner = null)
        : base(message, "ContextError", line, column, inner) { }
}

/// <summary>
/// Thrown by user explicit 'raise' statements.
/// </summary>
public class PilotUserException : PilotRuntimeException
{
    public PilotValue UserPayload { get; }

    public PilotUserException(
        string message,
        string typeName = "UserError",
        PilotValue? payload = null,
        int line = 0,
        int column = 0,
        Exception? inner = null)
        : base(message, typeName, line, column, inner)
    {
        UserPayload = payload ?? PilotValue.FromString(message);
    }
}
