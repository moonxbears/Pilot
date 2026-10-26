using System;
using System.Threading;
using System.Threading.Tasks;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Concurrency;

/// <summary>
/// Represents the result of an asynchronous multiprocessing or spawned execution in Pilot.
/// </summary>
public sealed class PilotFuture
{
    private readonly Task<PilotValue> _task;
    private readonly CancellationTokenSource? _cts;

    public bool IsCompleted => _task.IsCompleted;
    public bool IsFaulted => _task.IsFaulted;

    public PilotFuture(Task<PilotValue> task, CancellationTokenSource? cts = null)
    {
        _task = task ?? throw new ArgumentNullException(nameof(task));
        _cts = cts;
    }

    public static PilotFuture Run(Func<PilotValue> action)
    {
        var cts = new CancellationTokenSource();
        var task = Task.Run(action, cts.Token);
        return new PilotFuture(task, cts);
    }

    public PilotValue Await()
    {
        return _task.GetAwaiter().GetResult();
    }

    public PilotValue GetResult(int timeoutMilliseconds = -1)
    {
        if (timeoutMilliseconds < 0)
        {
            return Await();
        }

        if (_task.Wait(timeoutMilliseconds))
        {
            return _task.Result;
        }

        throw new TimeoutException($"Asynchronous task timed out after {timeoutMilliseconds}ms.");
    }

    public void Cancel()
    {
        _cts?.Cancel();
    }

    public PilotValue GetMember(string memberName)
    {
        return memberName switch
        {
            "await" => PilotValue.FromFunction(PilotFunction.CreateNative("await", (_, _) => Await())),
            "get_result" or "get" => PilotValue.FromFunction(PilotFunction.CreateNative("get_result", (_, args) =>
                GetResult(args.Count > 0 ? (int)args[0].AsInt() : -1))),
            "cancel" => PilotValue.FromFunction(PilotFunction.CreateNative("cancel", (_, _) => { Cancel(); return PilotValue.Null; })),
            "is_completed" => PilotValue.FromBool(IsCompleted),
            "is_faulted" => PilotValue.FromBool(IsFaulted),
            _ => PilotValue.Undefined
        };
    }

    public override string ToString() => $"<future [completed: {IsCompleted}]>";
}
