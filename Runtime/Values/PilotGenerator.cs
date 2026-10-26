using System;
using System.Collections;
using System.Collections.Generic;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;

namespace Pilot.Runtime.Values;

/// <summary>
/// State of a Pilot generator / coroutine.
/// </summary>
public enum PilotGeneratorState
{
    Suspended,
    Running,
    Completed,
    Closed
}

/// <summary>
/// Represents a stateful generator or coroutine in Pilot.
/// Supports .next(), .send(val), .close(), and iteration in for-in loops.
/// </summary>
public sealed class PilotGenerator : IEnumerable<PilotValue>, IDisposable
{
    private readonly Func<PilotValue, PilotValue> _stepFunction;
    private readonly Action? _onClose;
    public string Name { get; }
    public PilotGeneratorState State { get; private set; } = PilotGeneratorState.Suspended;
    public bool IsDone => State == PilotGeneratorState.Completed || State == PilotGeneratorState.Closed;

    public PilotGenerator(string name, Func<PilotValue, PilotValue> stepFunction, Action? onClose = null)
    {
        Name = name;
        _stepFunction = stepFunction ?? throw new ArgumentNullException(nameof(stepFunction));
        _onClose = onClose;
    }

    /// <summary>
    /// Creates a generator from an existing enumerable sequence.
    /// </summary>
    public static PilotGenerator FromEnumerable(string name, IEnumerable<PilotValue> sequence)
    {
        var enumerator = sequence.GetEnumerator();
        return new PilotGenerator(name, _ =>
        {
            if (enumerator.MoveNext())
            {
                return enumerator.Current;
            }
            throw new PilotReturnSignal(PilotValue.Undefined);
        });
    }

    public PilotValue Next() => Send(PilotValue.Undefined);

    public PilotValue Send(PilotValue input)
    {
        if (IsDone)
        {
            return PilotValue.Undefined;
        }

        State = PilotGeneratorState.Running;
        try
        {
            var yieldedVal = _stepFunction(input);
            State = PilotGeneratorState.Suspended;
            return yieldedVal;
        }
        catch (PilotYieldSignal y)
        {
            State = PilotGeneratorState.Suspended;
            return y.Value;
        }
        catch (PilotReturnSignal)
        {
            State = PilotGeneratorState.Completed;
            return PilotValue.Undefined;
        }
        catch
        {
            State = PilotGeneratorState.Closed;
            throw;
        }
    }

    public void Close()
    {
        State = PilotGeneratorState.Closed;
        _onClose?.Invoke();
    }

    public void Dispose() => Close();

    public IEnumerator<PilotValue> GetEnumerator()
    {
        while (!IsDone)
        {
            var val = Next();
            if (IsDone) break;
            yield return val;
        }
    }

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    public PilotValue GetMember(string memberName)
    {
        return memberName switch
        {
            "next" => PilotValue.FromFunction(PilotFunction.CreateNative("next", (_, _) => Next())),
            "send" => PilotValue.FromFunction(PilotFunction.CreateNative("send", (_, args) => Send(args.Count > 0 ? args[0] : PilotValue.Undefined))),
            "close" => PilotValue.FromFunction(PilotFunction.CreateNative("close", (_, _) => { Close(); return PilotValue.Null; })),
            "is_done" => PilotValue.FromBool(IsDone),
            "state" => PilotValue.FromString(State.ToString().ToLowerInvariant()),
            _ => PilotValue.Undefined
        };
    }

    public override string ToString() => $"<generator {Name} [{State}]>";
}
