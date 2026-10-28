using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Threading;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Reactive;

/// <summary>
/// Base node in Pilot's push-pull glitch-free topological Reactive Signal graph (Milestone 18.1 — Mode 5 <c>fn!^(a, b)</c>).
/// </summary>
public abstract class PilotSignalNode
{
    private static int _nextId;

    public int Id { get; } = Interlocked.Increment(ref _nextId);
    public string Name { get; }
    public int Rank { get; protected set; }
    public int Version { get; protected set; }
    public PilotValue Value { get; protected set; } = PilotValue.Undefined;

    internal readonly List<PilotComputedSignal> Subscribers = new();
    internal readonly List<(PilotExecutionContext? Context, PilotFunction Callback)> UserWatchers = new();
    private PilotObject? _cachedObject;

    protected PilotSignalNode(string name, int rank, PilotValue initialValue)
    {
        Name = name;
        Rank = rank;
        Value = initialValue;
        Version = 1;
    }

    public abstract PilotValue Read();

    public void Subscribe(PilotFunction callback, PilotExecutionContext? context = null)
    {
        if (callback == null) return;
        UserWatchers.Add((context, callback));
    }

    internal void NotifyWatchers()
    {
        if (UserWatchers.Count == 0) return;
        var currentVal = Value;
        foreach (var (ctx, cb) in UserWatchers.ToArray())
        {
            if (ctx != null)
            {
                cb.Invoke(ctx, new[] { currentVal });
            }
        }
    }

    public PilotObject ToPilotObject()
    {
        if (_cachedObject != null)
        {
            SynchronizeObjectFields(_cachedObject);
            return _cachedObject;
        }

        string typeName = this is PilotComputedSignal ? "ComputedSignal" : "Signal";
        var cls = new PilotClassDefinition(typeName);
        var obj = new PilotObject(cls)
        {
            NativePayload = this,
            DynamicMemberResolver = ResolveDynamicMember
        };

        SynchronizeObjectFields(obj);

        obj.Fields["get"] = PilotValue.FromFunction(PilotFunction.CreateNative(
            "get",
            (_, _) => Read()));
        obj.Fields["read"] = obj.Fields["get"];

        obj.Fields["set"] = PilotValue.FromFunction(PilotFunction.CreateNative(
            "set",
            (_, args) =>
            {
                if (this is PilotSourceSignal src)
                {
                    var next = args.Count > 0 ? args[0] : PilotValue.Null;
                    src.Set(next);
                    return next;
                }
                throw new PilotRuntimeException($"Cannot call '.set(...)' on read-only ComputedSignal '{Name}'.", 0, 0);
            },
            "value"));

        obj.Fields["subscribe"] = PilotValue.FromFunction(PilotFunction.CreateNative(
            "subscribe",
            (ctx, args) =>
            {
                if (args.Count > 0 && args[0].Kind == PilotValueKind.Function)
                {
                    Subscribe(args[0].AsFunction(), ctx);
                }
                return PilotValue.FromObject(obj);
            },
            "callback"));

        _cachedObject = obj;
        return obj;
    }

    private PilotValue? ResolveDynamicMember(string memberName)
    {
        return memberName switch
        {
            "Value" or "value" => Read(),
            "Rank" or "rank" => PilotValue.FromInt(Rank),
            "Version" or "version" => PilotValue.FromInt(Version),
            "Name" or "name" => PilotValue.FromString(Name),
            "IsComputed" or "is_computed" or "isComputed" => PilotValue.FromBool(this is PilotComputedSignal),
            "EvalCount" or "eval_count" or "evalCount" => PilotValue.FromInt(this is PilotComputedSignal cs ? cs.EvaluationCount : Version),
            _ => null
        };
    }

    internal void SynchronizeObjectFields(PilotObject obj)
    {
        obj.Fields["Value"] = Value;
        obj.Fields["value"] = Value;
        obj.Fields["Rank"] = PilotValue.FromInt(Rank);
        obj.Fields["rank"] = PilotValue.FromInt(Rank);
        obj.Fields["Version"] = PilotValue.FromInt(Version);
        obj.Fields["version"] = PilotValue.FromInt(Version);
        obj.Fields["Name"] = PilotValue.FromString(Name);
        obj.Fields["name"] = PilotValue.FromString(Name);
        obj.Fields["is_computed"] = PilotValue.FromBool(this is PilotComputedSignal);
        if (this is PilotComputedSignal cs)
        {
            obj.Fields["EvalCount"] = PilotValue.FromInt(cs.EvaluationCount);
            obj.Fields["eval_count"] = PilotValue.FromInt(cs.EvaluationCount);
            obj.Fields["dependencies"] = PilotValue.FromList(
                cs.Dependencies.Select(d => PilotValue.FromString(d.Name)).ToList());
        }
    }
}

/// <summary>
/// Root observable source signal (Rank 0). Can represent an explicit <c>signal(val)</c>,
/// an observable object property path <c>obj.Prop</c>, or a mutable <c>var</c> binding in scope.
/// </summary>
public sealed class PilotSourceSignal : PilotSignalNode
{
    public PilotObject? BoundObject { get; }
    public string? BoundPropertyName { get; }
    public PilotScope? BoundScope { get; }
    public string? BoundVariableName { get; }
    private bool _updatingBackingStore;

    public PilotSourceSignal(
        string name,
        PilotValue initialValue,
        PilotObject? boundObject = null,
        string? boundPropertyName = null,
        PilotScope? boundScope = null,
        string? boundVariableName = null)
        : base(name, rank: 0, initialValue)
    {
        BoundObject = boundObject;
        BoundPropertyName = boundPropertyName;
        BoundScope = boundScope;
        BoundVariableName = boundVariableName;
    }

    public override PilotValue Read()
    {
        if (BoundObject != null && BoundPropertyName != null &&
            BoundObject.Fields.TryGetValue(BoundPropertyName, out var fieldVal))
        {
            Value = fieldVal;
        }
        else if (BoundScope != null && BoundVariableName != null &&
                 BoundScope.TryResolve(BoundVariableName, out var scopeVal))
        {
            Value = scopeVal;
        }
        return Value;
    }

    public void Set(PilotValue newValue, bool fromBackingStoreNotification = false)
    {
        if (_updatingBackingStore) return;

        bool changed = !Value.Equals(newValue);
        Value = newValue;
        if (!changed && Version > 1)
        {
            return;
        }

        Version++;

        if (!fromBackingStoreNotification)
        {
            try
            {
                _updatingBackingStore = true;
                if (BoundObject != null && BoundPropertyName != null)
                {
                    BoundObject.Fields[BoundPropertyName] = newValue;
                }
                else if (BoundScope != null && BoundVariableName != null)
                {
                    BoundScope.ForceSet(BoundVariableName, newValue);
                }
            }
            finally
            {
                _updatingBackingStore = false;
            }
        }

        NotifyWatchers();
        PilotSignalGraph.PropagateFrom(this);
    }
}

/// <summary>
/// Derived reactive signal created by <c>fn!^(a, b)</c> whose topological rank is
/// <c>1 + max(dep.Rank)</c>. Recomputed in topological order without diamond glitches.
/// </summary>
public sealed class PilotComputedSignal : PilotSignalNode
{
    public IReadOnlyList<PilotSignalNode> Dependencies { get; }
    public Func<PilotValue> ComputeFunc { get; }
    public int EvaluationCount { get; private set; }

    public PilotComputedSignal(
        string name,
        IReadOnlyList<PilotSignalNode> dependencies,
        Func<PilotValue> computeFunc)
        : base(
            name,
            rank: dependencies.Count == 0 ? 1 : dependencies.Max(d => d.Rank) + 1,
            initialValue: PilotValue.Undefined)
    {
        Dependencies = dependencies;
        ComputeFunc = computeFunc;

        foreach (var dep in dependencies)
        {
            if (!dep.Subscribers.Contains(this))
            {
                dep.Subscribers.Add(this);
            }
        }

        // Initial eager evaluation
        Recompute();
    }

    public override PilotValue Read() => Value;

    internal bool Recompute()
    {
        var next = ComputeFunc();
        EvaluationCount++;
        bool changed = EvaluationCount == 1 || !Value.Equals(next);
        Value = next;
        if (changed)
        {
            Version++;
            NotifyWatchers();
        }
        return changed;
    }
}

/// <summary>
/// Glitch-free topological signal coordinator for Mode 5 (<c>fn!^(a, b) -&gt; Signal[TReturn]</c>).
/// Uses a rank-ordered priority queue so diamond dependencies evaluate strictly once per wave.
/// </summary>
public static class PilotSignalGraph
{
    private sealed class TopologicalRankComparer : IComparer<PilotComputedSignal>
    {
        public static readonly TopologicalRankComparer Instance = new();

        public int Compare(PilotComputedSignal? x, PilotComputedSignal? y)
        {
            if (ReferenceEquals(x, y)) return 0;
            if (x is null) return -1;
            if (y is null) return 1;
            int rankCmp = x.Rank.CompareTo(y.Rank);
            return rankCmp != 0 ? rankCmp : x.Id.CompareTo(y.Id);
        }
    }

    private sealed class ObjectPropertyKey : IEquatable<ObjectPropertyKey>
    {
        public readonly PilotObject Target;
        public readonly string PropertyName;

        public ObjectPropertyKey(PilotObject target, string propertyName)
        {
            Target = target;
            PropertyName = propertyName;
        }

        public bool Equals(ObjectPropertyKey? other) =>
            other != null &&
            ReferenceEquals(Target, other.Target) &&
            string.Equals(PropertyName, other.PropertyName, StringComparison.Ordinal);

        public override bool Equals(object? obj) => obj is ObjectPropertyKey other && Equals(other);

        public override int GetHashCode() =>
            HashCode.Combine(RuntimeHelpers.GetHashCode(Target), StringComparer.Ordinal.GetHashCode(PropertyName));
    }

    private sealed class ScopeVariableKey : IEquatable<ScopeVariableKey>
    {
        public readonly PilotScope Scope;
        public readonly string VariableName;

        public ScopeVariableKey(PilotScope scope, string variableName)
        {
            Scope = scope;
            VariableName = variableName;
        }

        public bool Equals(ScopeVariableKey? other) =>
            other != null &&
            ReferenceEquals(Scope, other.Scope) &&
            string.Equals(VariableName, other.VariableName, StringComparison.Ordinal);

        public override bool Equals(object? obj) => obj is ScopeVariableKey other && Equals(other);

        public override int GetHashCode() =>
            HashCode.Combine(RuntimeHelpers.GetHashCode(Scope), StringComparer.Ordinal.GetHashCode(VariableName));
    }

    private static readonly object SyncLock = new();
    private static readonly Dictionary<ObjectPropertyKey, PilotSourceSignal> PropertySignals = new();
    private static readonly Dictionary<ScopeVariableKey, PilotSourceSignal> VariableSignals = new();

    [ThreadStatic]
    private static SortedSet<PilotComputedSignal>? _dirtyQueue;

    [ThreadStatic]
    private static int _batchDepth;

    [ThreadStatic]
    private static bool _isPropagating;

    public static PilotSourceSignal CreateSourceSignal(string name, PilotValue initialValue) =>
        new(name, initialValue);

    public static PilotSourceSignal GetOrCreatePropertySignal(PilotObject target, string propertyName)
    {
        lock (SyncLock)
        {
            var key = new ObjectPropertyKey(target, propertyName);
            if (!PropertySignals.TryGetValue(key, out var sig))
            {
                var initVal = target.Fields.TryGetValue(propertyName, out var v) ? v : target.GetMember(propertyName);
                sig = new PilotSourceSignal($"{target.TypeName}.{propertyName}", initVal, boundObject: target, boundPropertyName: propertyName);
                PropertySignals[key] = sig;
            }
            return sig;
        }
    }

    public static PilotSourceSignal GetOrCreateVariableSignal(PilotScope definingScope, string variableName)
    {
        lock (SyncLock)
        {
            var key = new ScopeVariableKey(definingScope, variableName);
            if (!VariableSignals.TryGetValue(key, out var sig))
            {
                definingScope.TryResolve(variableName, out var initVal);
                sig = new PilotSourceSignal(variableName, initVal, boundScope: definingScope, boundVariableName: variableName);
                VariableSignals[key] = sig;
            }
            return sig;
        }
    }

    public static void NotifyObjectPropertyChanged(PilotObject target, string propertyName, PilotValue newValue)
    {
        PilotSourceSignal? sig;
        lock (SyncLock)
        {
            if (PropertySignals.Count == 0) return;
            PropertySignals.TryGetValue(new ObjectPropertyKey(target, propertyName), out sig);
        }

        sig?.Set(newValue, fromBackingStoreNotification: true);
    }

    public static void NotifyScopeVariableChanged(PilotScope definingScope, string variableName, PilotValue newValue)
    {
        PilotSourceSignal? sig;
        lock (SyncLock)
        {
            if (VariableSignals.Count == 0) return;
            VariableSignals.TryGetValue(new ScopeVariableKey(definingScope, variableName), out sig);
        }

        sig?.Set(newValue, fromBackingStoreNotification: true);
    }

    /// <summary>
    /// Executes <paramref name="action"/> inside a coalesced reactive batch transaction so all mutated
    /// source signals propagate in a single glitch-free topological wave when the batch exits.
    /// </summary>
    public static void Batch(Action action)
    {
        _batchDepth++;
        try
        {
            action();
        }
        finally
        {
            _batchDepth--;
            if (_batchDepth == 0 && !_isPropagating)
            {
                FlushDirtyQueue();
            }
        }
    }

    public static PilotValue Batch(Func<PilotValue> func)
    {
        _batchDepth++;
        try
        {
            return func();
        }
        finally
        {
            _batchDepth--;
            if (_batchDepth == 0 && !_isPropagating)
            {
                FlushDirtyQueue();
            }
        }
    }

    internal static void PropagateFrom(PilotSignalNode changedNode)
    {
        if (changedNode.Subscribers.Count == 0) return;

        _dirtyQueue ??= new SortedSet<PilotComputedSignal>(TopologicalRankComparer.Instance);
        foreach (var sub in changedNode.Subscribers)
        {
            _dirtyQueue.Add(sub);
        }

        if (_batchDepth > 0 || _isPropagating)
        {
            return;
        }

        FlushDirtyQueue();
    }

    private static void FlushDirtyQueue()
    {
        if (_dirtyQueue == null || _dirtyQueue.Count == 0) return;

        _isPropagating = true;
        try
        {
            while (_dirtyQueue.Count > 0)
            {
                var next = _dirtyQueue.Min!;
                _dirtyQueue.Remove(next);

                bool changed = next.Recompute();
                if (changed && next.Subscribers.Count > 0)
                {
                    foreach (var downstream in next.Subscribers)
                    {
                        _dirtyQueue.Add(downstream);
                    }
                }
            }
        }
        finally
        {
            _isPropagating = false;
        }
    }

    public static bool TryExtractSignalNode(PilotValue value, out PilotSignalNode signalNode)
    {
        if (value.Kind == PilotValueKind.Object && value.AsObject().NativePayload is PilotSignalNode node)
        {
            signalNode = node;
            return true;
        }

        signalNode = null!;
        return false;
    }

    /// <summary>
    /// Mode 5 (<c>fn!^(a, b) -&gt; Signal[TReturn]</c>): Walks argument ASTs to extract observable
    /// LValue property paths, mutable scope bindings, or <c>Signal[U]</c> inputs (enforcing <c>QUOTE005</c>
    /// when zero observable inputs exist) and constructs a topological <see cref="PilotComputedSignal"/>.
    /// </summary>
    public static PilotComputedSignal CreateReactiveCallSignal(
        PilotFunction? fn,
        string calleeName,
        PilotParser.ArgumentContext[] args,
        ParserRuleContext callSiteCtx,
        PilotInterpreterVisitor visitor)
    {
        var dependencies = new List<PilotSignalNode>();
        var seenIds = new HashSet<int>();

        void AddDependency(PilotSignalNode node)
        {
            if (seenIds.Add(node.Id))
            {
                dependencies.Add(node);
            }
        }

        foreach (var arg in args)
        {
            var testCtx = arg.test();
            if (testCtx == null) continue;
            ExtractObservableDependenciesFromTree(testCtx, visitor, AddDependency);
        }

        if (dependencies.Count == 0)
        {
            throw new PilotRuntimeException(
                $"[QUOTE005] Reactive signal call quote '{calleeName}!^(...)' requires at least one observable input (Signal[T], object property path 'obj.Prop', or mutable 'var' binding), but all arguments are constant or immutable.",
                callSiteCtx.Start.Line,
                callSiteCtx.Start.Column);
        }

        var capturedScope = visitor.Context.Environment.CurrentScope;

        PilotValue ComputeProjection()
        {
            var prevScope = visitor.Context.Environment.CurrentScope;
            try
            {
                // Ensure the visitor evaluates arguments in the lexical scope where the reactive signal was bound
                var evalArgs = new List<PilotValue>(args.Length);
                var namedArgs = new Dictionary<string, PilotValue>(StringComparer.Ordinal);

                foreach (var a in args)
                {
                    if (a.test() == null) continue;
                    var rawVal = EvaluateReactiveArgumentExpr(a.test(), visitor, capturedScope);
                    if (TryExtractSignalNode(rawVal, out var sigNode))
                    {
                        rawVal = sigNode.Read();
                    }

                    if (a.identifier() != null && (a.EQUAL() != null || a.COLON() != null))
                    {
                        namedArgs[a.identifier().GetText()] = rawVal;
                    }
                    else
                    {
                        evalArgs.Add(rawVal);
                    }
                }

                if (fn != null)
                {
                    return fn.Invoke(visitor.Context, evalArgs);
                }

                throw new PilotRuntimeException(
                    $"Reactive call target '{calleeName}' could not be resolved to an executable function.",
                    callSiteCtx.Start.Line,
                    callSiteCtx.Start.Column);
            }
            finally
            {
                _ = prevScope;
            }
        }

        string argSummary = string.Join(", ", args.Select(a => a.GetText()));
        return new PilotComputedSignal($"{calleeName}!^({argSummary})", dependencies, ComputeProjection);
    }

    private static PilotValue EvaluateReactiveArgumentExpr(
        PilotParser.TestContext testCtx,
        PilotInterpreterVisitor visitor,
        PilotScope capturedScope)
    {
        // Fast path for single identifier in captured scope: unwrap Signal if bound
        string text = testCtx.GetText().Trim();
        if (IsSimpleIdentifier(text) && capturedScope.TryResolve(text, out var boundVal))
        {
            return TryExtractSignalNode(boundVal, out var sig) ? sig.Read() : boundVal;
        }

        return visitor.Visit(testCtx);
    }

    private static bool IsSimpleIdentifier(string s)
    {
        if (string.IsNullOrEmpty(s)) return false;
        if (!(char.IsLetter(s[0]) || s[0] == '_')) return false;
        for (int i = 1; i < s.Length; i++)
        {
            if (!(char.IsLetterOrDigit(s[i]) || s[i] == '_')) return false;
        }
        return true;
    }

    private static void ExtractObservableDependenciesFromTree(
        IParseTree tree,
        PilotInterpreterVisitor visitor,
        Action<PilotSignalNode> addDependency)
    {
        if (tree is PilotParser.Atom_exprContext atomExpr)
        {
            var trailers = atomExpr.trailer();
            var atom = atomExpr.atom();
            string? baseIdent = atom?.identifier()?.GetText();

            // Case A: Member property path e.g. `ship.Mass` or `ship.Transform.Pos`
            if (trailers != null && trailers.Length > 0)
            {
                // Find the last `.Prop` trailer that is NOT followed by `(` (i.e. property access, not method call)
                int lastDotIdx = -1;
                for (int i = trailers.Length - 1; i >= 0; i--)
                {
                    if ((trailers[i].DOT() != null || trailers[i].SAFE_NAV() != null) &&
                        trailers[i].identifier() != null &&
                        (i + 1 >= trailers.Length || trailers[i + 1].LPAR() == null))
                    {
                        lastDotIdx = i;
                        break;
                    }
                }

                if (lastDotIdx >= 0)
                {
                    string propName = trailers[lastDotIdx].identifier().GetText();
                    PilotValue receiverVal = EvaluateAtomAndTrailersPrefix(atom!, trailers, lastDotIdx, visitor);
                    if (TryExtractSignalNode(receiverVal, out var receiverSig))
                    {
                        addDependency(receiverSig);
                    }
                    else if (receiverVal.Kind == PilotValueKind.Object)
                    {
                        var targetObj = receiverVal.AsObject();
                        if (targetObj.Fields.TryGetValue(propName, out var fieldVal) &&
                            TryExtractSignalNode(fieldVal, out var fieldSig))
                        {
                            addDependency(fieldSig);
                        }
                        else
                        {
                            addDependency(GetOrCreatePropertySignal(targetObj, propName));
                        }
                    }

                    // Also inspect any arguments inside earlier trailers
                    for (int i = 0; i < trailers.Length; i++)
                    {
                        if (trailers[i].arglist() != null)
                        {
                            ExtractObservableDependenciesFromTree(trailers[i].arglist(), visitor, addDependency);
                        }
                    }
                    return;
                }
            }

            // Case B: Bare identifier `x` — could be a `Signal[T]` value or a mutable `var x` binding
            if ((trailers == null || trailers.Length == 0) && !string.IsNullOrEmpty(baseIdent))
            {
                if (visitor.Context.Environment.CurrentScope.TryResolveBinding(baseIdent, out var ownerScope, out var binding))
                {
                    if (TryExtractSignalNode(binding.Value, out var sigNode))
                    {
                        addDependency(sigNode);
                        return;
                    }
                    if (!binding.IsConst)
                    {
                        addDependency(GetOrCreateVariableSignal(ownerScope, baseIdent));
                        return;
                    }
                }
                else if (visitor.Context.Environment.TryResolve(baseIdent, out var envVal) &&
                         TryExtractSignalNode(envVal, out var envSig))
                {
                    addDependency(envSig);
                    return;
                }
            }

            // Case C: Expression itself evaluates to a Signal (e.g., nested `fn!^(...)` or `signal(...)`)
            if (trailers != null && trailers.Length > 0 &&
                trailers[^1].call_quote_op()?.CALL_QUOTE_REACT() != null)
            {
                var nestedVal = visitor.Visit(atomExpr);
                if (TryExtractSignalNode(nestedVal, out var nestedSig))
                {
                    addDependency(nestedSig);
                    return;
                }
            }
        }

        for (int i = 0; i < tree.ChildCount; i++)
        {
            ExtractObservableDependenciesFromTree(tree.GetChild(i), visitor, addDependency);
        }
    }

    private static PilotValue EvaluateAtomAndTrailersPrefix(
        PilotParser.AtomContext atom,
        PilotParser.TrailerContext[] trailers,
        int stopBeforeIndex,
        PilotInterpreterVisitor visitor)
    {
        PilotValue current = visitor.Visit(atom);
        for (int i = 0; i < stopBeforeIndex; i++)
        {
            var tr = trailers[i];
            if ((tr.DOT() != null || tr.SAFE_NAV() != null) && tr.identifier() != null)
            {
                string member = tr.identifier().GetText();
                if (current.Kind == PilotValueKind.Object)
                {
                    current = current.AsObject().GetMember(member, visitor.Context);
                }
                else
                {
                    current = current.GetMember(member);
                }
            }
        }
        return current;
    }

    /// <summary>
    /// Used by static type analysis (<c>PilotBidirectionalInferencer</c>) to verify that at least one argument
    /// in <c>fn!^(...)</c> contains an observable input (member property path, Signal type, or mutable variable).
    /// </summary>
    public static bool HasStaticObservableInput(
        PilotParser.ArgumentContext[]? args,
        Func<string, bool> isObservableIdentifier)
    {
        if (args == null || args.Length == 0) return false;

        foreach (var arg in args)
        {
            if (arg.test() != null && TreeContainsStaticObservable(arg.test(), isObservableIdentifier))
            {
                return true;
            }
        }

        return false;
    }

    private static bool TreeContainsStaticObservable(IParseTree tree, Func<string, bool> isObservableIdentifier)
    {
        if (tree is PilotParser.Atom_exprContext atomExpr)
        {
            var trailers = atomExpr.trailer();
            if (trailers != null && trailers.Length > 0)
            {
                foreach (var tr in trailers)
                {
                    if ((tr.DOT() != null || tr.SAFE_NAV() != null) && tr.identifier() != null)
                    {
                        return true;
                    }
                    if (tr.call_quote_op()?.CALL_QUOTE_REACT() != null)
                    {
                        return true;
                    }
                }
            }

            string? ident = atomExpr.atom()?.identifier()?.GetText();
            if (!string.IsNullOrEmpty(ident) && isObservableIdentifier(ident))
            {
                return true;
            }
        }

        for (int i = 0; i < tree.ChildCount; i++)
        {
            if (TreeContainsStaticObservable(tree.GetChild(i), isObservableIdentifier))
            {
                return true;
            }
        }

        return false;
    }
}
