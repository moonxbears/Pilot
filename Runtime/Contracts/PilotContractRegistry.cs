using System;
using System.Collections.Generic;
using System.Linq;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Contracts;

/// <summary>
/// Registry of standard contracts, interfaces, and forwarders in Pilot.
/// Provides compile-time and runtime synthesis of contract forwarding methods.
/// </summary>
public static class PilotContractRegistry
{
    private static readonly Dictionary<string, IReadOnlyList<string>> StandardContracts = new(StringComparer.OrdinalIgnoreCase)
    {
        ["IList"] = new[] { "add", "append", "remove", "pop", "count", "length", "get", "set", "clear", "contains", "insert" },
        ["IEquatable"] = new[] { "equals" },
        ["IComparable"] = new[] { "compare_to", "compareTo" },
        ["IMap"] = new[] { "get", "set", "contains_key", "remove", "keys", "values", "count", "clear" },
        ["IDictionary"] = new[] { "get", "set", "contains_key", "remove", "keys", "values", "count", "clear" },
        ["ISet"] = new[] { "add", "remove", "contains", "count", "clear" },
        ["IDisposable"] = new[] { "dispose" },
        ["IEnumerable"] = new[] { "get_enumerator", "__iter__" }
    };

    /// <summary>
    /// Normalizes a contract name by stripping generic type parameters (e.g. "IList<PilotValue>" -> "IList").
    /// </summary>
    public static string NormalizeContractName(string contractName)
    {
        if (string.IsNullOrWhiteSpace(contractName)) return string.Empty;
        int angleIdx = contractName.IndexOf('<');
        if (angleIdx > 0)
        {
            contractName = contractName[..angleIdx];
        }
        int bracketIdx = contractName.IndexOf('[');
        if (bracketIdx > 0)
        {
            contractName = contractName[..bracketIdx];
        }
        if (contractName.StartsWith("self.", StringComparison.OrdinalIgnoreCase))
        {
            contractName = contractName[5..];
        }
        return contractName.Trim();
    }

    /// <summary>
    /// Retrieves method names belonging to a contract or interface.
    /// </summary>
    public static IReadOnlyList<string> GetContractMethods(string contractName, PilotExecutionContext? context = null)
    {
        string normalized = NormalizeContractName(contractName);
        if (StandardContracts.TryGetValue(normalized, out var methods))
        {
            return methods;
        }

        // Check if an interface or class definition exists in the execution environment
        if (context != null && context.Environment.TryResolve(normalized, out var symbolVal))
        {
            if (symbolVal.Kind == PilotValueKind.Function && symbolVal.AsFunction().ClassDef != null)
            {
                return symbolVal.AsFunction().ClassDef!.Methods.Keys.ToList();
            }
        }

        return Array.Empty<string>();
    }

    /// <summary>
    /// Synthesizes a forwarding function for a contract method delegated to a target field or expression.
    /// </summary>
    public static PilotFunction SynthesizeForwarder(string contractName, string methodName, string targetMember)
    {
        string normContract = NormalizeContractName(contractName);
        return new PilotFunction(
            methodName,
            new[] { new PilotParameter("self"), new PilotParameter("args", isParams: true) },
            (ctx, callArgs) =>
            {
                if (callArgs.Count == 0 || callArgs[0].Kind != PilotValueKind.Object)
                {
                    throw new PilotRuntimeException($"Contract forwarder '{methodName}' called without a valid self instance.");
                }

                var self = callArgs[0].AsObject();
                var remainingArgs = callArgs.Skip(1).ToList();

                // Resolve target value on self
                var targetVal = ResolveTargetValue(self, targetMember, normContract);
                return DispatchToTarget(self, targetVal, normContract, methodName, targetMember, ctx, remainingArgs);
            });
    }

    /// <summary>
    /// Resolves the delegated target value on a given instance.
    /// </summary>
    public static PilotValue ResolveTargetValue(PilotObject self, string targetMember, string normalizedContract)
    {
        // 1. Check instance ContractSubstitutes
        if (self.ContractSubstitutes.TryGetValue(normalizedContract, out var subVal))
        {
            return subVal;
        }

        // 2. Check instance fields by member name
        if (self.Fields.TryGetValue(targetMember, out var fieldVal))
        {
            return fieldVal;
        }

        // 3. Check clean name (stripping leading 'self.' or '_')
        string cleanName = targetMember.StartsWith("self.", StringComparison.OrdinalIgnoreCase)
            ? targetMember[5..]
            : targetMember;

        if (self.Fields.TryGetValue(cleanName, out fieldVal))
        {
            return fieldVal;
        }

        // 4. Fallback: check if target is an expression or member on self
        return self.GetMember(cleanName);
    }

    /// <summary>
    /// Dispatches a method invocation to the delegated target.
    /// </summary>
    public static PilotValue DispatchToTarget(
        PilotObject self,
        PilotValue targetVal,
        string normalizedContract,
        string methodName,
        string targetMember,
        PilotExecutionContext ctx,
        IReadOnlyList<PilotValue> args)
    {
        // Special case: IEquatable.equals
        if (methodName.Equals("equals", StringComparison.OrdinalIgnoreCase) ||
            normalizedContract.Equals("IEquatable", StringComparison.OrdinalIgnoreCase))
        {
            if (args.Count == 0) return PilotValue.False;
            var other = args[0];

            if (other.Kind == PilotValueKind.Object)
            {
                var otherSelf = other.AsObject();
                var otherTargetVal = ResolveTargetValue(otherSelf, targetMember, normalizedContract);
                if (otherTargetVal != PilotValue.Undefined)
                {
                    return PilotValue.FromBool(targetVal.Equals(otherTargetVal));
                }
            }

            return PilotValue.FromBool(targetVal.Equals(other));
        }

        // Target is a native Pilot List
        if (targetVal.Kind == PilotValueKind.List)
        {
            var list = targetVal.AsList();
            switch (methodName.ToLowerInvariant())
            {
                case "add":
                case "append":
                case "push":
                    if (args.Count > 0) list.Add(args[0]);
                    return PilotValue.Null;

                case "remove":
                    if (args.Count > 0)
                    {
                        bool removed = list.Remove(args[0]);
                        return PilotValue.FromBool(removed);
                    }
                    return PilotValue.False;

                case "pop":
                    if (list.Count == 0) return PilotValue.Undefined;
                    int popIdx = args.Count > 0 ? (int)args[0].AsInt() : list.Count - 1;
                    if (popIdx < 0) popIdx = list.Count + popIdx;
                    if (popIdx >= 0 && popIdx < list.Count)
                    {
                        var val = list[popIdx];
                        list.RemoveAt(popIdx);
                        return val;
                    }
                    return PilotValue.Undefined;

                case "count":
                case "length":
                case "size":
                    return PilotValue.FromInt(list.Count);

                case "get":
                    if (args.Count > 0)
                    {
                        int idx = (int)args[0].AsInt();
                        if (idx < 0) idx = list.Count + idx;
                        return idx >= 0 && idx < list.Count ? list[idx] : PilotValue.Undefined;
                    }
                    return PilotValue.Undefined;

                case "set":
                    if (args.Count >= 2)
                    {
                        int idx = (int)args[0].AsInt();
                        if (idx >= 0 && idx < list.Count) list[idx] = args[1];
                        return PilotValue.Null;
                    }
                    return PilotValue.Null;

                case "clear":
                    list.Clear();
                    return PilotValue.Null;

                case "contains":
                    return PilotValue.FromBool(args.Count > 0 && list.Contains(args[0]));

                case "insert":
                    if (args.Count >= 2)
                    {
                        int idx = (int)args[0].AsInt();
                        list.Insert(idx, args[1]);
                    }
                    return PilotValue.Null;
            }
        }

        // Target is a native Pilot Map
        if (targetVal.Kind == PilotValueKind.Map)
        {
            var map = targetVal.AsMap();
            switch (methodName.ToLowerInvariant())
            {
                case "get":
                    return args.Count > 0 && map.TryGetValue(args[0], out var val) ? val : PilotValue.Undefined;

                case "set":
                    if (args.Count >= 2) map[args[0]] = args[1];
                    return PilotValue.Null;

                case "contains_key":
                case "contains":
                    return PilotValue.FromBool(args.Count > 0 && map.ContainsKey(args[0]));

                case "remove":
                    return PilotValue.FromBool(args.Count > 0 && map.Remove(args[0]));

                case "keys":
                    return PilotValue.FromList(map.Keys.ToList());

                case "values":
                    return PilotValue.FromList(map.Values.ToList());

                case "count":
                case "length":
                    return PilotValue.FromInt(map.Count);

                case "clear":
                    map.Clear();
                    return PilotValue.Null;
            }
        }

        // Target is another PilotObject or general value with members
        var member = targetVal.GetMember(methodName);
        if (member.Kind == PilotValueKind.Function)
        {
            return member.AsFunction().Invoke(ctx, args);
        }

        if (member != PilotValue.Undefined)
        {
            return member;
        }

        throw new PilotRuntimeException($"Contract method '{methodName}' not found on target substitute '{targetMember}'.");
    }
}
