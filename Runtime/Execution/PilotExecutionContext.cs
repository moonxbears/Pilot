using System;
using System.Collections.Generic;
using System.Threading;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Execution;

/// <summary>
/// Execution state context passed through evaluation of Pilot AST.
/// </summary>
public sealed class PilotExecutionContext
{
    private Dictionary<string, PilotValue>? _oldSnapshots;
    private PilotInterpreterVisitor? _visitor;
    private Pilot.Runtime.HotReload.PilotHotReloadSession? _hotReload;

    public IPilotEngine Engine { get; }
    public PilotExecutionOptions Options { get; }
    public PilotEnvironment Environment { get; }
    public PilotScope CurrentScope => Environment.CurrentScope;
    public Pilot.Runtime.HotReload.PilotHotReloadSession HotReload => _hotReload ??= new Pilot.Runtime.HotReload.PilotHotReloadSession(this);
    public Stack<Action> DeferStack { get; } = new();
    public int CallDepth { get; set; }
    public CancellationToken CancellationToken { get; }
    public Func<PilotValue, PilotValue>? YieldHandler { get; set; }
    public Pilot.Debugging.IPilotDebugger? Debugger => Options.Debugger;

    /// <summary>
    /// Lightweight control-flow state for zero-exception return/break/continue unwinding.
    /// </summary>
    public PilotFlowSignalKind PendingFlow { get; set; }
    public PilotValue PendingReturnValue { get; set; }
    public string? PendingFlowLabel { get; set; }
    public bool HasPendingBreakValue { get; set; }
    public PilotValue PendingBreakValue { get; set; }
    public bool HasPendingContinueRange { get; set; }
    public PilotRange? PendingContinueRange { get; set; }
    public bool PendingContinueWorklistSpread { get; set; }
    public Action<PilotValue>? LoopYieldAccumulator { get; set; }
    public Pilot.Runtime.NativeMemory.PilotSharedBuffer? PendingSliceYieldTarget { get; set; }
    public PilotFunction? CurrentFunction { get; set; }

    public PilotValue AdaptPropagatedError(PilotValue errVal, string? targetErrorType = null)
    {
        targetErrorType ??= CurrentFunction?.ErrorTypeSpec;
        if (!string.IsNullOrWhiteSpace(targetErrorType) &&
            TypeAdapters.TryConvert(errVal, targetErrorType, this, out var adapted))
        {
            return adapted;
        }
        return errVal;
    }

    /// <summary>
    /// Gets or creates a reusable <see cref="PilotInterpreterVisitor"/> bound to this context.
    /// </summary>
    public PilotInterpreterVisitor GetOrCreateVisitor() => _visitor ??= new PilotInterpreterVisitor(this);

    public Stack<Pilot.Runtime.Extensions.PilotLexicalExtension> ActiveExtensions { get; } = new();
    public Dictionary<string, Pilot.Runtime.Extensions.PilotLexicalExtension> RegisteredExtensions { get; } = new(StringComparer.Ordinal);
    public Stack<Dictionary<string, PilotValue>> ContextualBindings { get; } = new();
    public Dictionary<string, PilotValue> OldSnapshots => _oldSnapshots ??= new(StringComparer.Ordinal);

    // Milestone 19.1.7: O(1) Integer Display Slot Table for Contextual DI
    private static readonly Dictionary<string, int> GlobalContextSlotIds = new(StringComparer.Ordinal);
    private static readonly List<string> GlobalContextSlotNames = new();
    private static readonly object ContextSlotLock = new();
    private PilotValue[] _contextDisplaySlots = new PilotValue[32];
    private readonly Stack<(int SlotId, PilotValue PreviousValue)> _contextSlotHistory = new();

    // Milestone 19.2: Deterministic Comptime Execution Budget & Sandboxing
    public bool IsComptimeEvaluation { get; set; }
    public int ComptimeStepBudget { get; set; }
    public int ComptimeStepsExecuted { get; set; }

    public static int InternContextSlotId(string name)
    {
        lock (ContextSlotLock)
        {
            if (GlobalContextSlotIds.TryGetValue(name, out int id))
            {
                return id;
            }
            id = GlobalContextSlotNames.Count;
            GlobalContextSlotNames.Add(name);
            GlobalContextSlotIds[name] = id;
            return id;
        }
    }

    public void PushContextDisplaySlot(int slotId, string? name, PilotValue value)
    {
        if (slotId >= _contextDisplaySlots.Length)
        {
            int newLen = Math.Max(slotId + 1, _contextDisplaySlots.Length * 2);
            Array.Resize(ref _contextDisplaySlots, newLen);
        }
        _contextSlotHistory.Push((slotId, _contextDisplaySlots[slotId]));
        _contextDisplaySlots[slotId] = value;
        if (!string.IsNullOrEmpty(name))
        {
            ContextualBindings.Push(new Dictionary<string, PilotValue>(StringComparer.Ordinal) { [name] = value });
        }
    }

    public void PopContextDisplaySlot(int slotId, bool popDictionary = true)
    {
        if (_contextSlotHistory.Count > 0)
        {
            var top = _contextSlotHistory.Pop();
            if ((uint)top.SlotId < (uint)_contextDisplaySlots.Length)
            {
                _contextDisplaySlots[top.SlotId] = top.PreviousValue;
            }
        }
        if (popDictionary && ContextualBindings.Count > 0)
        {
            ContextualBindings.Pop();
        }
    }

    public bool TryResolveContextSlot(int slotId, string? fallbackName, out PilotValue value)
    {
        if ((uint)slotId < (uint)_contextDisplaySlots.Length)
        {
            var slotVal = _contextDisplaySlots[slotId];
            if (slotVal.Kind != PilotValueKind.Null && !slotVal.IsUndefined)
            {
                value = slotVal;
                return true;
            }
        }
        if (!string.IsNullOrEmpty(fallbackName))
        {
            return TryResolveContextual(fallbackName, out value);
        }
        value = PilotValue.Undefined;
        return false;
    }

    internal Dictionary<string, PilotValue>? SwapOldSnapshots(Dictionary<string, PilotValue>? next)
    {
        var prev = _oldSnapshots;
        _oldSnapshots = next;
        return prev;
    }

    public int AutoRepeatDepth { get; set; }
    public bool IsAutoRepeatActive => AutoRepeatDepth > 0;
    public Pilot.Runtime.Adapters.PilotTypeAdapterRegistry TypeAdapters { get; } = Pilot.Runtime.Adapters.PilotTypeAdapterRegistry.Default;
    public Dictionary<string, PilotAdtDefinition> AdtDefinitions { get; } = new(StringComparer.Ordinal);
    public Dictionary<string, PilotRecordDefinition> RecordDefinitions { get; } = new(StringComparer.Ordinal);
    public Dictionary<string, PilotAdtVariantDefinition> VariantDefinitions { get; } = new(StringComparer.Ordinal);

    public bool TryResolveContextual(string name, out PilotValue value)
    {
        // 0. O(1) Integer Display Slot Table fast path (Milestone 19.1.7)
        int slotId = InternContextSlotId(name);
        if ((uint)slotId < (uint)_contextDisplaySlots.Length)
        {
            var slotVal = _contextDisplaySlots[slotId];
            if (slotVal.Kind != PilotValueKind.Null && !slotVal.IsUndefined)
            {
                value = slotVal;
                return true;
            }
        }

        // 1. Search active ContextualBindings stack frames from top to bottom
        if (ContextualBindings.Count > 0)
        {
            foreach (var frame in ContextualBindings)
            {
                if (frame.TryGetValue(name, out value))
                {
                    return true;
                }
            }
        }

        // 2. Fall back to current lexical scope environment
        if (Environment.CurrentScope.TryResolve(name, out value))
        {
            return true;
        }

        value = PilotValue.Undefined;
        return false;
    }

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

    public void ExecuteDefersDownTo(int countWatermark)
    {
        while (DeferStack.Count > countWatermark)
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
