using System;
using System.Collections.Generic;
using System.Threading;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Execution;

/// <summary>
/// Execution state context passed through evaluation of Pilot AST.
/// </summary>
public sealed class PilotExecutionContext
{
    public IPilotEngine Engine { get; }
    public PilotExecutionOptions Options { get; }
    public PilotEnvironment Environment { get; }
    public Stack<Action> DeferStack { get; } = new();
    public int CallDepth { get; set; }
    public CancellationToken CancellationToken { get; }
    public Func<PilotValue, PilotValue>? YieldHandler { get; set; }
    public Pilot.Debugging.IPilotDebugger? Debugger => Options.Debugger;

    public Stack<Pilot.Runtime.Extensions.PilotLexicalExtension> ActiveExtensions { get; } = new();
    public Dictionary<string, Pilot.Runtime.Extensions.PilotLexicalExtension> RegisteredExtensions { get; } = new(StringComparer.Ordinal);

    public PilotExecutionContext(
        IPilotEngine engine,
        PilotExecutionOptions options,
        PilotEnvironment? environment = null,
        CancellationToken cancellationToken = default)
    {
        Engine = engine ?? throw new ArgumentNullException(nameof(engine));
        Options = options ?? throw new ArgumentNullException(nameof(options));
        Environment = environment ?? new PilotEnvironment();
        CancellationToken = cancellationToken;
    }

    public void PushDefer(Action action)
    {
        if (action != null)
        {
            DeferStack.Push(action);
        }
    }

    public void ExecuteDefers()
    {
        while (DeferStack.Count > 0)
        {
            var action = DeferStack.Pop();
            try
            {
                action();
            }
            catch
            {
                // Defer cleanup shouldn't mask original control flow if possible
            }
        }
    }
}
