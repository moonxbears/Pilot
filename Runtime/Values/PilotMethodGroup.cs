using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;

namespace Pilot.Runtime.Values;

/// <summary>
/// First-class method group in Pilot.
/// Supports:
/// - Auto-routing switchboard based on argument types and arity.
/// - Method group currying (The Signature Sieve) for partial argument application.
/// - Method group fallback and union via bitwise OR (|).
/// - Compile-time and dynamic overload AST introspection (.overloads).
/// - Multi-cast broadcast execution (*).
/// - First-class keyword-dispatched overloads and reified type checking.
/// </summary>
public sealed class PilotMethodGroup
{
    public string Name { get; }
    public List<PilotFunction> Overloads { get; } = new();
    public PilotObject? TargetInstance { get; set; }
    public Dictionary<string, PilotValue> BoundArguments { get; } = new(StringComparer.Ordinal);

    public PilotMethodGroup(string name)
    {
        Name = name;
    }

    public PilotMethodGroup(
        string name,
        IEnumerable<PilotFunction> overloads,
        PilotObject? target = null,
        IDictionary<string, PilotValue>? boundArguments = null)
    {
        Name = name;
        TargetInstance = target;
        if (overloads != null)
        {
            Overloads.AddRange(overloads);
        }
        if (boundArguments != null)
        {
            foreach (var kvp in boundArguments)
            {
                BoundArguments[kvp.Key] = kvp.Value;
            }
        }
    }

    public void AddOverload(PilotFunction function)
    {
        if (function == null) return;
        if (Overloads.Contains(function)) return;

        string sig = function.GetSignatureKey();
        for (int i = 0; i < Overloads.Count; i++)
        {
            if (string.Equals(Overloads[i].Name, function.Name, StringComparison.Ordinal) &&
                Overloads[i].GetSignatureKey() == sig)
            {
                Overloads[i].ApplyPatchFrom(function);
                return;
            }
        }

        Overloads.Add(function);
    }

    public PilotMethodGroup CloneSnapshot()
    {
        var clones = Overloads.Select(o => o.CloneSnapshot()).ToList();
        return new PilotMethodGroup(Name, clones, TargetInstance, BoundArguments);
    }

    public void RestoreFromSnapshot(PilotMethodGroup snapshot)
    {
        Overloads.Clear();
        foreach (var fn in snapshot.Overloads)
        {
            Overloads.Add(fn.CloneSnapshot());
        }
    }

    /// <summary>
    /// Merges this method group with another method group into a unified overload set.
    /// Used by the group fallback and union operator (|).
    /// </summary>
    public PilotMethodGroup Union(PilotMethodGroup other)
    {
        if (other == null) return this;
        string combinedName = Name == other.Name ? Name : $"{Name}|{other.Name}";
        var merged = new PilotMethodGroup(combinedName, Overloads, TargetInstance, BoundArguments);
        foreach (var fn in other.Overloads)
        {
            merged.AddOverload(fn);
        }
        return merged;
    }

    /// <summary>
    /// Creates a copy of this method group bound to the specified object instance.
    /// </summary>
    public PilotMethodGroup BindTo(PilotObject target)
    {
        return new PilotMethodGroup(Name, Overloads, target, BoundArguments);
    }

    /// <summary>
    /// Invokes the method group with the given positional and named arguments.
    /// Performs auto-routing, currying (signature sieve), or multi-cast broadcast based on flags.
    /// </summary>
    public PilotValue Invoke(
        PilotExecutionContext context,
        IReadOnlyList<PilotValue> positionalArgs,
        IReadOnlyDictionary<string, PilotValue>? namedArgs = null,
        bool isBroadcast = false)
    {
        // 1. Merge provided named arguments with previously bound arguments
        var allNamed = new Dictionary<string, PilotValue>(BoundArguments, StringComparer.Ordinal);
        if (namedArgs != null)
        {
            foreach (var kvp in namedArgs)
            {
                allNamed[kvp.Key] = kvp.Value;
            }
        }

        // 2. Search for all complete matching overloads
        var completeMatches = new List<(PilotFunction Function, IReadOnlyList<PilotValue> Args)>();

        foreach (var overload in Overloads)
        {
            if (TryMatchOverload(overload, positionalArgs, allNamed, context, out var fullArgs))
            {
                completeMatches.Add((overload, fullArgs));
            }
        }

        // 3. If complete matching overloads are found:
        if (completeMatches.Count > 0)
        {
            // If broadcast multi-cast requested (*): invoke ALL matching overloads
            if (isBroadcast)
            {
                var broadcastResults = new List<PilotValue>(completeMatches.Count);
                foreach (var match in completeMatches)
                {
                    broadcastResults.Add(ExecuteOverload(match.Function, match.Args, context));
                }
                return PilotValue.FromList(broadcastResults);
            }

            // Normal invocation: invoke best (most specific / first) match
            var bestMatch = completeMatches
                .OrderByDescending(m => ComputeOverloadSpecificityScore(m.Function, m.Args))
                .First();
            return ExecuteOverload(bestMatch.Function, bestMatch.Args, context);
        }

        // 4. If no complete match was found, attempt Method Group Currying (The Signature Sieve)
        if (!isBroadcast && (positionalArgs.Count > 0 || allNamed.Count > 0))
        {
            var compatibleOverloads = new List<PilotFunction>();
            foreach (var overload in Overloads)
            {
                if (CanPartiallyMatch(overload, positionalArgs, allNamed, context))
                {
                    compatibleOverloads.Add(overload);
                }
            }

            // If we filtered down to a valid subset of overloads that accept these partial arguments:
            if (compatibleOverloads.Count > 0)
            {
                // Pre-bind positional arguments by mapping them to parameter names if possible
                var curriedBounds = new Dictionary<string, PilotValue>(allNamed, StringComparer.Ordinal);
                if (compatibleOverloads.Count > 0)
                {
                    int startParamIdx = 0;
                    if (TargetInstance != null && compatibleOverloads[0].Parameters.Count > 0 &&
                        (compatibleOverloads[0].Parameters[0].Name == "self" || compatibleOverloads[0].Parameters[0].Name == "this"))
                    {
                        startParamIdx = 1;
                    }
                    for (int i = 0; i < positionalArgs.Count; i++)
                    {
                        int pIdx = startParamIdx + i;
                        if (pIdx < compatibleOverloads[0].Parameters.Count)
                        {
                            var pName = compatibleOverloads[0].Parameters[pIdx].Name;
                            curriedBounds[pName] = positionalArgs[i];
                        }
                    }
                }

                return PilotValue.FromMethodGroup(new PilotMethodGroup(
                    Name,
                    compatibleOverloads,
                    TargetInstance,
                    curriedBounds));
            }
        }

        // 5. Automatic Repeated Call / Vectorization Fallback for Collections
        for (int i = 0; i < positionalArgs.Count; i++)
        {
            var pVal = positionalArgs[i];
            if (pVal.Kind is PilotValueKind.List or PilotValueKind.Tuple or PilotValueKind.Set)
            {
                var collectionItems = ExtractCollectionElements(pVal);
                if (collectionItems.Count > 0)
                {
                    var firstElem = collectionItems[0];
                    bool matchesAny = Overloads.Any(ov =>
                    {
                        int pIndex = (TargetInstance != null && ov.Parameters.Count > 0 &&
                                     (ov.Parameters[0].Name == "self" || ov.Parameters[0].Name == "this")) ? i + 1 : i;
                        return ov.Parameters.Count > pIndex && IsTypeCompatible(ov.Parameters[pIndex].TypeName, firstElem, context, out _);
                    });

                    if (matchesAny)
                    {
                        var repeatedResults = new List<PilotValue>(collectionItems.Count);
                        foreach (var elem in collectionItems)
                        {
                            var clonedPos = new List<PilotValue>(positionalArgs);
                            clonedPos[i] = elem;
                            repeatedResults.Add(Invoke(context, clonedPos, allNamed, isBroadcast: false));
                        }
                        if (repeatedResults.All(r => r.IsNull || r.IsUndefined))
                        {
                            return PilotValue.Null;
                        }
                        return PilotValue.FromList(repeatedResults);
                    }
                }
            }
        }

        // 6. If no overloads matched even partially, throw descriptive error
        string argTypes = string.Join(", ", positionalArgs.Select(a => a.Kind.ToString().ToLowerInvariant())
            .Concat(allNamed.Select(kv => $"{kv.Key}={kv.Value.Kind.ToString().ToLowerInvariant()}")));
        throw new PilotTypeException($"No matching overload in method group '{Name}' for arguments ({argTypes}).");
    }

    private static List<PilotValue> ExtractCollectionElements(PilotValue val)
    {
        if (val.Kind == PilotValueKind.List) return val.AsList();
        if (val.Kind == PilotValueKind.Tuple) return val.AsTuple().ToList();
        if (val.Kind == PilotValueKind.Set) return val.AsSet().ToList();
        return new List<PilotValue> { val };
    }

    private PilotValue ExecuteOverload(
        PilotFunction function,
        IReadOnlyList<PilotValue> fullArgs,
        PilotExecutionContext context)
    {
        if (TargetInstance != null && function.Parameters.Count > 0 &&
            (function.Parameters[0].Name == "self" || function.Parameters[0].Name == "this"))
        {
            var boundArgs = new List<PilotValue>(fullArgs.Count + 1) { PilotValue.FromObject(TargetInstance) };
            boundArgs.AddRange(fullArgs);
            return function.Invoke(context, boundArgs);
        }
        return function.Invoke(context, fullArgs);
    }

    /// <summary>
    /// Checks whether an overload can be completely satisfied by the provided arguments.
    /// </summary>
    private bool TryMatchOverload(
        PilotFunction function,
        IReadOnlyList<PilotValue> positional,
        IReadOnlyDictionary<string, PilotValue> named,
        PilotExecutionContext context,
        out IReadOnlyList<PilotValue> fullArgs)
    {
        fullArgs = Array.Empty<PilotValue>();
        var paramsList = function.Parameters;
        var constructedArgs = new List<PilotValue>();

        int startParamIdx = 0;
        if (TargetInstance != null && paramsList.Count > 0 &&
            (paramsList[0].Name == "self" || paramsList[0].Name == "this"))
        {
            startParamIdx = 1;
        }

        int posIdx = 0;
        var usedNamed = new HashSet<string>(StringComparer.Ordinal);

        for (int i = startParamIdx; i < paramsList.Count; i++)
        {
            var p = paramsList[i];

            // 1. Check if supplied via named argument
            if (named.TryGetValue(p.Name, out var namedVal))
            {
                if (!IsTypeCompatible(p.TypeName, namedVal, context, out var adaptedNamed))
                {
                    return false;
                }
                constructedArgs.Add(adaptedNamed);
                usedNamed.Add(p.Name);
            }
            // 2. Check if supplied via positional argument
            else if (posIdx < positional.Count)
            {
                var posVal = positional[posIdx++];
                if (!IsTypeCompatible(p.TypeName, posVal, context, out var adaptedPos))
                {
                    return false;
                }
                constructedArgs.Add(adaptedPos);
            }
            // 3. Fallback to default parameter value
            else if (p.DefaultValue.HasValue)
            {
                constructedArgs.Add(p.DefaultValue.Value);
            }
            // 4. Params / varargs parameter
            else if (p.IsParams)
            {
                var rest = new List<PilotValue>();
                while (posIdx < positional.Count)
                {
                    rest.Add(positional[posIdx++]);
                }
                constructedArgs.Add(PilotValue.FromList(rest));
            }
            // 5. Missing required parameter
            else
            {
                return false;
            }
        }

        // Positional args must not exceed non-params parameters
        if (posIdx < positional.Count)
        {
            return false;
        }

        // All named arguments must correspond to actual parameters
        if (usedNamed.Count < named.Count)
        {
            return false;
        }

        fullArgs = constructedArgs;
        return true;
    }

    /// <summary>
    /// Checks whether an overload can accept the provided partial arguments (Signature Sieve).
    /// </summary>
    private bool CanPartiallyMatch(
        PilotFunction function,
        IReadOnlyList<PilotValue> positional,
        IReadOnlyDictionary<string, PilotValue> named,
        PilotExecutionContext context)
    {
        var paramsList = function.Parameters;

        int startParamIdx = 0;
        if (TargetInstance != null && paramsList.Count > 0 &&
            (paramsList[0].Name == "self" || paramsList[0].Name == "this"))
        {
            startParamIdx = 1;
        }

        // 1. Every named argument MUST correspond to a parameter with compatible type
        foreach (var kvp in named)
        {
            var p = paramsList.Skip(startParamIdx).FirstOrDefault(param => string.Equals(param.Name, kvp.Key, StringComparison.Ordinal));
            if (p == null) return false;
            if (!IsTypeCompatible(p.TypeName, kvp.Value, context)) return false;
        }

        // 2. Every positional argument must be within parameter bounds and type-compatible
        for (int i = 0; i < positional.Count; i++)
        {
            int pIdx = startParamIdx + i;
            if (pIdx >= paramsList.Count) return false;
            if (!IsTypeCompatible(paramsList[pIdx].TypeName, positional[i], context)) return false;
        }

        return true;
    }

    /// <summary>
    /// Evaluates whether an argument value matches an expected type annotation, optionally using argument adapters.
    /// </summary>
    public static bool IsTypeCompatible(string? expectedType, PilotValue arg, PilotExecutionContext? context = null)
    {
        return IsTypeCompatible(expectedType, arg, context, out _);
    }

    public static bool IsTypeCompatible(string? expectedType, PilotValue arg, PilotExecutionContext? context, out PilotValue adapted)
    {
        adapted = arg;
        if (string.IsNullOrWhiteSpace(expectedType)) return true;
        expectedType = expectedType.Trim();

        // 1. Direct type matching
        if (IsDirectlyCompatible(expectedType, arg, context))
        {
            return true;
        }

        // 2. Argument Type Adapters
        if (Pilot.Runtime.Adapters.PilotTypeAdapterRegistry.Default.TryConvert(arg, expectedType, context, out adapted))
        {
            return true;
        }

        return false;
    }

    private static bool IsDirectlyCompatible(string expectedType, PilotValue arg, PilotExecutionContext? context = null)
    {
        if (expectedType.Equals("any", StringComparison.OrdinalIgnoreCase) ||
            expectedType.Equals("dynamic", StringComparison.OrdinalIgnoreCase) ||
            expectedType.Equals("object", StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        switch (expectedType.ToLowerInvariant())
        {
            case "int" or "i8" or "i16" or "i32" or "i64" or "integer":
                return arg.Kind == PilotValueKind.Int || arg.Kind == PilotValueKind.UInt;

            case "uint" or "u8" or "u16" or "u32" or "u64":
                return arg.Kind == PilotValueKind.UInt;

            case "float" or "f16" or "f32" or "f64" or "double":
                return arg.Kind == PilotValueKind.Float || arg.Kind == PilotValueKind.Int || arg.Kind == PilotValueKind.UInt;

            case "str" or "string":
                return arg.Kind == PilotValueKind.String;

            case "bool" or "boolean":
                return arg.Kind == PilotValueKind.Bool;

            case "list" or "array":
                return arg.Kind == PilotValueKind.List;

            case "map" or "dict" or "dictionary":
                return arg.Kind == PilotValueKind.Map;

            case "set":
                return arg.Kind == PilotValueKind.Set;

            case "tuple":
                return arg.Kind == PilotValueKind.Tuple;

            case "matrix":
                return arg.Kind == PilotValueKind.Matrix;

            case "keyword" or "symbol" or "atom":
                return arg.Kind == PilotValueKind.Keyword;
        }

        // First-class keyword-dispatched parameter type: `def Handle(role: :admin)` or `def Handle(role: :user/admin)`
        if (expectedType.StartsWith(':'))
        {
            var expectedKw = PilotKeyword.Intern(expectedType.AsSpan(1).ToString());
            if (arg.Kind == PilotValueKind.Keyword)
            {
                return arg.AsKeyword().IsA(expectedKw);
            }
            if (arg.Kind == PilotValueKind.KeywordVariant)
            {
                return arg.AsKeywordVariant().Keyword.IsA(expectedKw);
            }
            return false;
        }

        // Custom class / inheritance matching
        if (arg.Kind == PilotValueKind.Object)
        {
            var obj = arg.AsObject();
            if (string.Equals(obj.TypeName, expectedType, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }

            // Check inheritance hierarchy
            var baseName = obj.ClassDef?.BaseClassName;
            while (!string.IsNullOrEmpty(baseName))
            {
                if (string.Equals(baseName, expectedType, StringComparison.OrdinalIgnoreCase))
                {
                    return true;
                }

                // If base class is resolvable in scope, continue up the hierarchy
                if (context != null && context.Environment.TryResolve(baseName, out var baseVal) && baseVal.Kind == PilotValueKind.Function)
                {
                    baseName = baseVal.AsFunction().ClassDef?.BaseClassName;
                }
                else
                {
                    break;
                }
            }
        }

        return false;
    }

    private static int ComputeOverloadSpecificityScore(PilotFunction fn, IReadOnlyList<PilotValue> args)
    {
        int score = 0;
        int count = Math.Min(fn.Parameters.Count, args.Count);
        for (int i = 0; i < count; i++)
        {
            string? expectedType = fn.Parameters[i].TypeName;
            if (string.IsNullOrEmpty(expectedType) ||
                expectedType.Equals("any", StringComparison.OrdinalIgnoreCase) ||
                expectedType.Equals("dynamic", StringComparison.OrdinalIgnoreCase) ||
                expectedType.Equals("object", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            if (expectedType.StartsWith(':'))
            {
                var expectedKw = PilotKeyword.Intern(expectedType.AsSpan(1).ToString());
                PilotKeyword? actualKw = args[i].Kind == PilotValueKind.Keyword
                    ? args[i].AsKeyword()
                    : (args[i].Kind == PilotValueKind.KeywordVariant ? args[i].AsKeywordVariant().Keyword : null);
                if (actualKw != null)
                {
                    int dist = actualKw.GetDerivationDistance(expectedKw);
                    if (dist >= 0)
                    {
                        score += Math.Max(10, 100 - dist * 10);
                    }
                }
            }
            else
            {
                score += 20;
            }
        }
        return score;
    }

    /// <summary>
    /// Checks whether a runtime value satisfies this reified type union / method group (e.g., `let T = int | str`).
    /// </summary>
    public bool MatchesReifiedType(PilotValue value, PilotExecutionContext? context = null)
    {
        foreach (var ov in Overloads)
        {
            if (MatchesSingleTypeFunction(ov, value, context))
            {
                return true;
            }
        }
        return false;
    }

    public static bool MatchesSingleTypeFunction(PilotFunction fn, PilotValue value, PilotExecutionContext? context = null)
    {
        if (fn.ClassDef != null)
        {
            return IsDirectlyCompatible(fn.Name, value, context);
        }
        return IsDirectlyCompatible(fn.Name, value, context);
    }

    /// <summary>
    /// Generates introspection objects for every overload in this method group.
    /// Exposes: name, signature, parameters, return_type, is_async, is_generator.
    /// </summary>
    public List<PilotValue> GetOverloadValues()
    {
        var result = new List<PilotValue>(Overloads.Count);
        foreach (var ov in Overloads)
        {
            var ovObj = new PilotObject(new PilotClassDefinition("OverloadInfo"));
            ovObj.SetMember("name", PilotValue.FromString(ov.Name));
            ovObj.SetMember("signature", PilotValue.FromString(BuildSignature(ov)));
            ovObj.SetMember("return_type", PilotValue.FromString("any"));
            ovObj.SetMember("is_async", PilotValue.FromBool(ov.IsAsync));
            ovObj.SetMember("is_generator", PilotValue.FromBool(ov.IsGenerator));

            var paramList = new List<PilotValue>(ov.Parameters.Count);
            foreach (var p in ov.Parameters)
            {
                var pMap = new Dictionary<PilotValue, PilotValue>
                {
                    [PilotValue.FromString("name")] = PilotValue.FromString(p.Name),
                    [PilotValue.FromString("type")] = PilotValue.FromString(p.TypeName ?? "any"),
                    [PilotValue.FromString("is_params")] = PilotValue.FromBool(p.IsParams)
                };
                paramList.Add(PilotValue.FromMap(pMap));
            }
            ovObj.SetMember("parameters", PilotValue.FromList(paramList));
            ovObj.SetMember("args", PilotValue.FromList(paramList));

            result.Add(PilotValue.FromObject(ovObj));
        }
        return result;
    }

    private static string BuildSignature(PilotFunction fn)
    {
        var sb = new StringBuilder();
        sb.Append('(');
        for (int i = 0; i < fn.Parameters.Count; i++)
        {
            if (i > 0) sb.Append(", ");
            var p = fn.Parameters[i];
            sb.Append(p.Name);
            if (!string.IsNullOrEmpty(p.TypeName))
            {
                sb.Append(": ").Append(p.TypeName);
            }
        }
        sb.Append(") -> any");
        return sb.ToString();
    }

    public override string ToString() =>
        $"<method-group '{Name}' with {Overloads.Count} overload(s)>";
}
