using System;
using Pilot.Runtime.Execution;

namespace Pilot.Runtime.Values;

/// <summary>
/// Encapsulates a deferred lazy expression in Pilot that is evaluated only upon first demand.
/// </summary>
public sealed class PilotLazy
{
    private readonly Func<PilotExecutionContext, PilotValue> _evaluator;
    private readonly PilotExecutionContext _context;
    private PilotValue? _cachedValue;
    private readonly object _lock = new();

    public bool IsEvaluated => _cachedValue.HasValue;

    public PilotLazy(Func<PilotExecutionContext, PilotValue> evaluator, PilotExecutionContext context)
    {
        _evaluator = evaluator ?? throw new ArgumentNullException(nameof(evaluator));
        _context = context ?? throw new ArgumentNullException(nameof(context));
    }

    public PilotValue Value
    {
        get
        {
            if (!_cachedValue.HasValue)
            {
                lock (_lock)
                {
                    if (!_cachedValue.HasValue)
                    {
                        _cachedValue = _evaluator(_context);
                    }
                }
            }
            return _cachedValue.Value;
        }
    }

    public PilotValue GetMember(string memberName)
    {
        return memberName switch
        {
            "value" => Value,
            "eval" => PilotValue.FromFunction(PilotFunction.CreateNative("eval", (_, _) => Value)),
            "is_evaluated" => PilotValue.FromBool(IsEvaluated),
            _ => Value.GetMember(memberName) // Fallback to inner value member
        };
    }

    public override string ToString() =>
        IsEvaluated && _cachedValue.HasValue ? $"lazy({_cachedValue.Value})" : "lazy(<unevaluated>)";
}
