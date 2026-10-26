using System.Collections.Generic;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Builtins;

/// <summary>
/// Contract for built-in or user-extended native functions in Pilot.
/// </summary>
public interface IBuiltinFunction
{
    string Name { get; }
    IReadOnlyList<string> ParameterNames { get; }
    PilotValue Call(PilotExecutionContext context, IReadOnlyList<PilotValue> arguments);
}
