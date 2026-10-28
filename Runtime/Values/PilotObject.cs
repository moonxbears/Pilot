using System;
using System.Collections.Generic;
using System.Text;
using Pilot.Runtime.Execution;

namespace Pilot.Runtime.Values;

/// <summary>
/// Represents a class definition blueprint in Pilot.
/// </summary>
public sealed class PilotClassDefinition
{
    public string Name { get; }
    public string? BaseClassName { get; set; }
    public Dictionary<string, PilotFunction> Methods { get; } = new(StringComparer.Ordinal);
    public Dictionary<string, PilotMethodGroup> MethodGroups { get; } = new(StringComparer.Ordinal);
    public Dictionary<string, PilotValue> StaticFields { get; } = new(StringComparer.Ordinal);
    public Dictionary<string, string> ContractDelegations { get; } = new(StringComparer.OrdinalIgnoreCase);
    public Dictionary<string, PilotClassDefinition> States { get; } = new(StringComparer.Ordinal);
    public string? InitialState { get; set; }
    public List<PilotFunction> GenerativeMacros { get; } = new();
    public List<string> OrderedFieldNames { get; } = new();
    public Dictionary<string, int> FieldSlotIndices { get; } = new(StringComparer.Ordinal);

    public int RegisterFieldSlot(string fieldName)
    {
        if (FieldSlotIndices.TryGetValue(fieldName, out int existing))
        {
            return existing;
        }
        int idx = OrderedFieldNames.Count;
        OrderedFieldNames.Add(fieldName);
        FieldSlotIndices[fieldName] = idx;
        return idx;
    }

    public PilotClassDefinition(string name, string? baseClassName = null)
    {
        Name = name;
        BaseClassName = baseClassName;
    }

    #region Algebraic Class Composition (Class Math)

    public PilotClassDefinition Union(PilotClassDefinition other, string? newName = null)
    {
        if (other == null) return this;
        string resultName = newName ?? $"{Name}_{other.Name}";
        var result = new PilotClassDefinition(resultName, BaseClassName ?? other.BaseClassName);

        foreach (var kvp in StaticFields) result.StaticFields[kvp.Key] = kvp.Value.CloneValue();
        foreach (var kvp in other.StaticFields) result.StaticFields[kvp.Key] = kvp.Value.CloneValue();

        foreach (var kvp in Methods) result.Methods[kvp.Key] = kvp.Value;
        foreach (var kvp in other.Methods) result.Methods[kvp.Key] = kvp.Value;

        foreach (var kvp in MethodGroups) result.MethodGroups[kvp.Key] = kvp.Value;
        foreach (var kvp in other.MethodGroups)
        {
            if (result.MethodGroups.TryGetValue(kvp.Key, out var existingMg))
                result.MethodGroups[kvp.Key] = existingMg.Union(kvp.Value);
            else
                result.MethodGroups[kvp.Key] = kvp.Value;
        }

        foreach (var kvp in States) result.States[kvp.Key] = kvp.Value;
        foreach (var kvp in other.States) result.States[kvp.Key] = kvp.Value;

        result.InitialState = InitialState ?? other.InitialState;
        return result;
    }

    public PilotClassDefinition Subtract(PilotClassDefinition other, string? newName = null)
    {
        if (other == null) return this;
        string resultName = newName ?? $"{Name}_sub_{other.Name}";
        var result = new PilotClassDefinition(resultName, BaseClassName);

        foreach (var kvp in StaticFields)
        {
            if (!other.StaticFields.ContainsKey(kvp.Key))
                result.StaticFields[kvp.Key] = kvp.Value.CloneValue();
        }

        foreach (var kvp in Methods)
        {
            if (!other.Methods.ContainsKey(kvp.Key) && !other.MethodGroups.ContainsKey(kvp.Key))
                result.Methods[kvp.Key] = kvp.Value;
        }

        foreach (var kvp in MethodGroups)
        {
            if (!other.MethodGroups.ContainsKey(kvp.Key) && !other.Methods.ContainsKey(kvp.Key))
                result.MethodGroups[kvp.Key] = kvp.Value;
        }

        foreach (var kvp in States)
        {
            if (!other.States.ContainsKey(kvp.Key))
                result.States[kvp.Key] = kvp.Value;
        }

        result.InitialState = InitialState;
        return result;
    }

    public PilotClassDefinition Subtract(string memberName, string? newName = null)
    {
        string resultName = newName ?? $"{Name}_sub_{memberName}";
        var result = new PilotClassDefinition(resultName, BaseClassName);

        foreach (var kvp in StaticFields) if (kvp.Key != memberName) result.StaticFields[kvp.Key] = kvp.Value.CloneValue();
        foreach (var kvp in Methods) if (kvp.Key != memberName) result.Methods[kvp.Key] = kvp.Value;
        foreach (var kvp in MethodGroups) if (kvp.Key != memberName) result.MethodGroups[kvp.Key] = kvp.Value;
        foreach (var kvp in States) if (kvp.Key != memberName) result.States[kvp.Key] = kvp.Value;

        result.InitialState = InitialState;
        return result;
    }

    public PilotClassDefinition Intersect(PilotClassDefinition other, string? newName = null)
    {
        if (other == null) return new PilotClassDefinition(newName ?? $"{Name}_empty");
        string resultName = newName ?? $"{Name}_intersect_{other.Name}";
        var result = new PilotClassDefinition(resultName);

        foreach (var kvp in StaticFields)
        {
            if (other.StaticFields.ContainsKey(kvp.Key))
                result.StaticFields[kvp.Key] = kvp.Value.CloneValue();
        }

        foreach (var kvp in Methods)
        {
            if (other.Methods.ContainsKey(kvp.Key) || other.MethodGroups.ContainsKey(kvp.Key))
                result.Methods[kvp.Key] = kvp.Value;
        }

        foreach (var kvp in MethodGroups)
        {
            if (other.MethodGroups.ContainsKey(kvp.Key) || other.Methods.ContainsKey(kvp.Key))
                result.MethodGroups[kvp.Key] = kvp.Value;
        }

        return result;
    }

    public PilotClassDefinition CloneWithName(string newName)
    {
        var cloned = new PilotClassDefinition(newName, BaseClassName);
        foreach (var kvp in StaticFields) cloned.StaticFields[kvp.Key] = kvp.Value.CloneValue();
        foreach (var kvp in Methods) cloned.Methods[kvp.Key] = kvp.Value;
        foreach (var kvp in MethodGroups) cloned.MethodGroups[kvp.Key] = kvp.Value;
        foreach (var kvp in ContractDelegations) cloned.ContractDelegations[kvp.Key] = kvp.Value;
        foreach (var kvp in States) cloned.States[kvp.Key] = kvp.Value;
        cloned.InitialState = InitialState;
        cloned.GenerativeMacros.AddRange(GenerativeMacros);
        return cloned;
    }

    /// <summary>
    /// Merges a newly defined or hot-patched function or method group into this class definition in-place.
    /// Live instances referencing this <see cref="PilotClassDefinition"/> immediately observe the patched methods.
    /// </summary>
    public void MergeMethodValue(PilotValue funcVal)
    {
        if (funcVal.Kind == PilotValueKind.MethodGroup)
        {
            var mg = funcVal.AsMethodGroup();
            if (MethodGroups.TryGetValue(mg.Name, out var existingMg))
            {
                foreach (var ov in mg.Overloads)
                {
                    existingMg.AddOverload(ov);
                }
            }
            else if (Methods.TryGetValue(mg.Name, out var oldSingle))
            {
                var combined = new PilotMethodGroup(mg.Name, new[] { oldSingle });
                foreach (var ov in mg.Overloads)
                {
                    combined.AddOverload(ov);
                }
                Methods.Remove(mg.Name);
                MethodGroups[mg.Name] = combined;
            }
            else
            {
                MethodGroups[mg.Name] = mg;
            }
        }
        else if (funcVal.Kind == PilotValueKind.Function)
        {
            var func = funcVal.AsFunction();
            if (MethodGroups.TryGetValue(func.Name, out var existingMg))
            {
                existingMg.AddOverload(func);
            }
            else if (Methods.TryGetValue(func.Name, out var existingMethod))
            {
                if (existingMethod.GetSignatureKey() == func.GetSignatureKey())
                {
                    existingMethod.ApplyPatchFrom(func);
                }
                else
                {
                    var combined = new PilotMethodGroup(func.Name, new[] { existingMethod, func });
                    Methods.Remove(func.Name);
                    MethodGroups[func.Name] = combined;
                }
            }
            else
            {
                Methods[func.Name] = func;
            }
        }
    }

    /// <summary>
    /// Creates a deep snapshot of this class definition (including methods, method groups, static fields, and typestates)
    /// for hot-reload rollback.
    /// </summary>
    public PilotClassDefinition CloneSnapshot()
    {
        var snap = new PilotClassDefinition(Name, BaseClassName)
        {
            InitialState = InitialState
        };
        foreach (var kvp in StaticFields) snap.StaticFields[kvp.Key] = kvp.Value.CloneValue();
        foreach (var kvp in Methods) snap.Methods[kvp.Key] = kvp.Value.CloneSnapshot();
        foreach (var kvp in MethodGroups) snap.MethodGroups[kvp.Key] = kvp.Value.CloneSnapshot();
        foreach (var kvp in ContractDelegations) snap.ContractDelegations[kvp.Key] = kvp.Value;
        foreach (var kvp in States) snap.States[kvp.Key] = kvp.Value.CloneSnapshot();
        snap.GenerativeMacros.AddRange(GenerativeMacros);
        return snap;
    }

    /// <summary>
    /// Restores this class definition in-place from a previous snapshot.
    /// </summary>
    public void RestoreFromSnapshot(PilotClassDefinition snapshot)
    {
        if (snapshot == null) return;
        BaseClassName = snapshot.BaseClassName;
        InitialState = snapshot.InitialState;

        StaticFields.Clear();
        foreach (var kvp in snapshot.StaticFields) StaticFields[kvp.Key] = kvp.Value.CloneValue();

        // Update existingPilotFunction references in-place if possible so direct method delegates also revert
        var oldMethods = new Dictionary<string, PilotFunction>(Methods, StringComparer.Ordinal);
        Methods.Clear();
        foreach (var kvp in snapshot.Methods)
        {
            if (oldMethods.TryGetValue(kvp.Key, out var liveFn))
            {
                liveFn.ApplyPatchFrom(kvp.Value, isRollback: true);
                Methods[kvp.Key] = liveFn;
            }
            else
            {
                Methods[kvp.Key] = kvp.Value.CloneSnapshot();
            }
        }

        var oldGroups = new Dictionary<string, PilotMethodGroup>(MethodGroups, StringComparer.Ordinal);
        MethodGroups.Clear();
        foreach (var kvp in snapshot.MethodGroups)
        {
            if (oldGroups.TryGetValue(kvp.Key, out var liveMg))
            {
                liveMg.RestoreFromSnapshot(kvp.Value);
                MethodGroups[kvp.Key] = liveMg;
            }
            else
            {
                MethodGroups[kvp.Key] = kvp.Value.CloneSnapshot();
            }
        }

        ContractDelegations.Clear();
        foreach (var kvp in snapshot.ContractDelegations) ContractDelegations[kvp.Key] = kvp.Value;

        var oldStates = new Dictionary<string, PilotClassDefinition>(States, StringComparer.Ordinal);
        States.Clear();
        foreach (var kvp in snapshot.States)
        {
            if (oldStates.TryGetValue(kvp.Key, out var liveState))
            {
                liveState.RestoreFromSnapshot(kvp.Value);
                States[kvp.Key] = liveState;
            }
            else
            {
                States[kvp.Key] = kvp.Value.CloneSnapshot();
            }
        }

        GenerativeMacros.Clear();
        GenerativeMacros.AddRange(snapshot.GenerativeMacros);
    }

    #endregion

    /// <summary>
    /// Steals methods from a non-inherited class into this class definition.
    /// Existing or explicitly defined methods on this class are never overwritten.
    /// </summary>
    public void StealFrom(PilotClassDefinition sourceClass, IReadOnlyList<string>? specificMethods = null)
    {
        if (sourceClass == null) return;

        if (specificMethods != null && specificMethods.Count > 0)
        {
            foreach (var m in specificMethods)
            {
                if (sourceClass.Methods.TryGetValue(m, out var fn) && !Methods.ContainsKey(m))
                {
                    Methods[m] = fn;
                }
            }
        }
        else
        {
            foreach (var kvp in sourceClass.Methods)
            {
                string mName = kvp.Key;
                if (mName == "__init__" || mName == "constructor" || mName == sourceClass.Name)
                {
                    continue; // Skip constructors
                }
                if (!Methods.ContainsKey(mName))
                {
                    Methods[mName] = kvp.Value;
                }
            }
        }
    }

    /// <summary>
    /// Delegates a contract/interface to a specified field or substitute target.
    /// Synthesizes forwarding methods for any interface functions not explicitly implemented.
    /// </summary>
    public void DelegateContract(string contractName, string targetMember, PilotExecutionContext? context = null)
    {
        string normalized = Pilot.Runtime.Contracts.PilotContractRegistry.NormalizeContractName(contractName);
        ContractDelegations[normalized] = targetMember;

        var contractMethods = Pilot.Runtime.Contracts.PilotContractRegistry.GetContractMethods(normalized, context);
        foreach (var method in contractMethods)
        {
            if (!Methods.ContainsKey(method))
            {
                Methods[method] = Pilot.Runtime.Contracts.PilotContractRegistry.SynthesizeForwarder(normalized, method, targetMember);
            }
        }
    }

    public PilotObject Instantiate(PilotExecutionContext context, IReadOnlyList<PilotValue> constructorArgs)
    {
        var instance = new PilotObject(this);
        if (!string.IsNullOrEmpty(InitialState))
        {
            instance.CurrentState = InitialState;
        }

        // Initialize default fields from base classes and this definition
        PopulateDefaultFields(instance, context);

        // Check for constructor (__init__ or constructor matching class name or constructor)
        if (Methods.TryGetValue("__init__", out var initFunc) ||
            Methods.TryGetValue(Name, out initFunc) ||
            Methods.TryGetValue("constructor", out initFunc))
        {
            var argsWithSelf = new PilotValue[constructorArgs.Count + 1];
            argsWithSelf[0] = PilotValue.FromObject(instance);
            for (int i = 0; i < constructorArgs.Count; i++)
            {
                argsWithSelf[i + 1] = constructorArgs[i];
            }
            initFunc.Invoke(context, argsWithSelf);
        }

        return instance;
    }

    private void PopulateDefaultFields(PilotObject instance, PilotExecutionContext context)
    {
        if (!string.IsNullOrEmpty(BaseClassName) && context.Environment.TryResolve(BaseClassName, out var baseVal) &&
            baseVal.Kind == PilotValueKind.Function && baseVal.AsFunction().ClassDef != null)
        {
            baseVal.AsFunction().ClassDef!.PopulateDefaultFields(instance, context);
        }

        foreach (var kvp in StaticFields)
        {
            int slot = RegisterFieldSlot(kvp.Key);
            var cloned = kvp.Value.CloneValue();
            instance.Fields[kvp.Key] = cloned;
            instance.SetFieldSlotDirect(slot, cloned);
        }
    }

    public override string ToString() => $"<class {Name}>";
}

/// <summary>
/// Represents an object instance of a Pilot class.
/// </summary>
public sealed class PilotObject : IEquatable<PilotObject>
{
    private Dictionary<PilotFunction, PilotValue>? _boundMethodCache;

    public PilotClassDefinition? ClassDef { get; }
    public string? CurrentState { get; set; }
    public bool IsFrozen { get; set; }
    public PilotValue[]? FieldSlots { get; private set; }
    public string TypeName => ClassDef?.Name ?? "object";
    public string FullTypeName => !string.IsNullOrEmpty(CurrentState) ? $"{TypeName}[{CurrentState}]" : TypeName;
    public Dictionary<string, PilotValue> Fields { get; } = new(StringComparer.Ordinal);
    public Dictionary<string, PilotValue> ContractSubstitutes { get; } = new(StringComparer.OrdinalIgnoreCase);
    public object? NativePayload { get; set; }
    public Func<string, PilotValue?>? DynamicMemberResolver { get; set; }

    public PilotObject(PilotClassDefinition? classDef = null)
    {
        ClassDef = classDef;
        Pilot.Runtime.Effects.PilotEffectDryRunEngine.NotifyObjectCreated(this);
        if (classDef?.InitialState != null)
        {
            CurrentState = classDef.InitialState;
        }
    }

    public PilotValue GetFieldSlot(int slotIndex, string? fallbackName = null)
    {
        if (FieldSlots != null && (uint)slotIndex < (uint)FieldSlots.Length)
        {
            var val = FieldSlots[slotIndex];
            if (val.Kind != PilotValueKind.Null || (fallbackName != null && Fields.ContainsKey(fallbackName)))
            {
                return val;
            }
        }
        if (fallbackName != null && Fields.TryGetValue(fallbackName, out var dictVal))
        {
            return dictVal;
        }
        if (ClassDef != null && (uint)slotIndex < (uint)ClassDef.OrderedFieldNames.Count)
        {
            string fieldName = ClassDef.OrderedFieldNames[slotIndex];
            if (Fields.TryGetValue(fieldName, out var bySlotName))
            {
                return bySlotName;
            }
        }
        return PilotValue.Undefined;
    }

    public void SetFieldSlotDirect(int slotIndex, PilotValue value)
    {
        if (slotIndex < 0) return;
        if (FieldSlots == null)
        {
            int size = Math.Max(slotIndex + 1, ClassDef?.OrderedFieldNames.Count ?? (slotIndex + 1));
            FieldSlots = new PilotValue[size];
        }
        else if (slotIndex >= FieldSlots.Length)
        {
            var resized = new PilotValue[slotIndex + 1];
            Array.Copy(FieldSlots, resized, FieldSlots.Length);
            FieldSlots = resized;
        }
        FieldSlots[slotIndex] = value;
    }

    public void SetFieldSlot(int slotIndex, PilotValue value, string? fieldName = null)
    {
        if (IsFrozen)
        {
            throw new InvalidOperationException($"Cannot mutate field on compile-time frozen object '{TypeName}'.");
        }
        SetFieldSlotDirect(slotIndex, value);
        string? resolvedName = fieldName ??
            (ClassDef != null && (uint)slotIndex < (uint)ClassDef.OrderedFieldNames.Count
                ? ClassDef.OrderedFieldNames[slotIndex]
                : null);
        if (resolvedName != null)
        {
            if (Pilot.Runtime.Effects.PilotEffectDryRunEngine.TryRecordObjectFieldWrite(this, resolvedName, value))
            {
                return;
            }
            Fields[resolvedName] = value;
            Pilot.Runtime.Reactive.PilotSignalGraph.NotifyObjectPropertyChanged(this, resolvedName, value);
        }
    }

    public PilotValue GetMember(string memberName, PilotExecutionContext? context = null)
    {
        if (Pilot.Runtime.Effects.PilotEffectDryRunEngine.TryReadObjectField(this, memberName, out var shadowVal))
        {
            return shadowVal;
        }
        if (DynamicMemberResolver != null && DynamicMemberResolver(memberName) is { } dynamicVal)
        {
            return dynamicVal;
        }
        if (Fields.TryGetValue(memberName, out var fieldVal))
        {
            return fieldVal;
        }

        // Built-in state functions: self.transition_to("StateName"), self.current_state()
        if (memberName == "current_state" || memberName == "currentState")
        {
            return PilotValue.FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                CurrentState != null ? PilotValue.FromString(CurrentState) : PilotValue.Null));
        }

        if (memberName == "transition_to")
        {
            return PilotValue.FromFunction(PilotFunction.CreateNative("transition_to", (ctx, args) =>
            {
                if (args.Count > 0)
                {
                    CurrentState = args[0].ToString();
                }
                return PilotValue.FromObject(this);
            }, "newState"));
        }

        // Check Typestate specific methods if class has active typestate
        if (ClassDef != null && !string.IsNullOrEmpty(CurrentState))
        {
            if (ClassDef.States.TryGetValue(CurrentState, out var stateDef))
            {
                if (stateDef.MethodGroups.TryGetValue(memberName, out var smg))
                {
                    return PilotValue.FromMethodGroup(smg.BindTo(this));
                }
                if (stateDef.Methods.TryGetValue(memberName, out var smethod))
                {
                    return BindMethodValue(smethod);
                }
                if (stateDef.StaticFields.TryGetValue(memberName, out var sfield))
                {
                    return sfield;
                }
            }

            // Verify if method belongs to another state of this class (disallowing invalid state calls)
            foreach (var kvp in ClassDef.States)
            {
                if (!string.Equals(kvp.Key, CurrentState, StringComparison.Ordinal))
                {
                    if (kvp.Value.Methods.ContainsKey(memberName) || kvp.Value.MethodGroups.ContainsKey(memberName))
                    {
                        throw new Pilot.Runtime.ControlFlow.PilotTypeException(
                            $"Method or member '{memberName}' does not exist on {ClassDef.Name}[{CurrentState}]. Valid state: {kvp.Key}.");
                    }
                }
            }
        }

        if (ClassDef != null)
        {
            if (ClassDef.MethodGroups.TryGetValue(memberName, out var mg))
            {
                return PilotValue.FromMethodGroup(mg.BindTo(this));
            }
            if (ClassDef.Methods.TryGetValue(memberName, out var method))
            {
                // Bind method to self
                return BindMethodValue(method);
            }
            if (ClassDef.StaticFields.TryGetValue(memberName, out var sf))
            {
                return sf;
            }
        }

        // Check active lexical extensions
        if (context != null && context.ActiveExtensions.Count > 0)
        {
            var selfVal = PilotValue.FromObject(this);
            foreach (var ext in context.ActiveExtensions)
            {
                if (ext.MatchesType(selfVal))
                {
                    if (ext.MethodGroups.TryGetValue(memberName, out var emg))
                    {
                        return PilotValue.FromMethodGroup(emg.BindTo(this));
                    }
                    if (ext.Methods.TryGetValue(memberName, out var emethod))
                    {
                        return BindMethodValue(emethod);
                    }
                }
            }
        }

        // Check dynamic contract substitutes (e.g. self.IList.Contracts = substitute)
        if (ContractSubstitutes.Count > 0)
        {
            foreach (var kvp in ContractSubstitutes)
            {
                var subVal = kvp.Value;
                var subMember = subVal.GetMember(memberName);
                if (subMember.Kind == PilotValueKind.Function)
                {
                    return subMember;
                }
                if (subVal.Kind == PilotValueKind.List || subVal.Kind == PilotValueKind.Map)
                {
                    try
                    {
                        var synthesized = Pilot.Runtime.Contracts.PilotContractRegistry.SynthesizeForwarder(
                            kvp.Key, memberName, kvp.Key);
                        return BindMethodValue(synthesized);
                    }
                    catch { }
                }
            }

            if (ContractSubstitutes.TryGetValue(memberName, out var directSub))
            {
                return directSub;
            }
        }

        // Fallback root object member resolution
        return memberName switch
        {
            "to_string" or "ToString" or "__str__" => PilotValue.FromFunction(PilotFunction.CreateNative(
                memberName, (_, _) => PilotValue.FromString(ToString()))),
            "type_of" or "GetType" => PilotValue.FromFunction(PilotFunction.CreateNative(
                memberName, (_, _) => PilotValue.FromString(TypeName))),
            "hash" or "GetHashCode" or "__hash__" => PilotValue.FromFunction(PilotFunction.CreateNative(
                memberName, (_, _) => PilotValue.FromInt(GetHashCode()))),
            "clone" or "Clone" or "__clone__" => PilotValue.FromFunction(PilotFunction.CreateNative(
                memberName, (_, _) => PilotValue.FromObject(Clone()))),
            "equals" or "Equals" or "__eq__" => PilotValue.FromFunction(PilotFunction.CreateNative(
                memberName, (_, args) => PilotValue.FromBool(args.Count > 0 && Equals(args[0].AsObject())))),
            "to_json" or "toJson" => PilotValue.FromFunction(PilotFunction.CreateNative(
                memberName, (_, args) => PilotValue.FromString(Pilot.Runtime.Serialization.PilotJsonSerializer.Serialize(PilotValue.FromObject(this), args.Count > 0 && args[0].IsTrue)))),
            _ => PilotValue.Undefined
        };
    }

    public void SetMember(string memberName, PilotValue value)
    {
        if (Pilot.Runtime.Effects.PilotEffectDryRunEngine.TryRecordObjectFieldWrite(this, memberName, value))
        {
            return;
        }
        if (IsFrozen)
        {
            throw new InvalidOperationException($"Cannot mutate field '{memberName}' on compile-time frozen object '{TypeName}'.");
        }
        Fields[memberName] = value;
        if (ClassDef != null && ClassDef.FieldSlotIndices.TryGetValue(memberName, out int slot))
        {
            SetFieldSlotDirect(slot, value);
        }
        Pilot.Runtime.Reactive.PilotSignalGraph.NotifyObjectPropertyChanged(this, memberName, value);
    }

    private PilotValue BindMethodValue(PilotFunction unbound)
    {
        _boundMethodCache ??= new Dictionary<PilotFunction, PilotValue>(ReferenceEqualityComparer.Instance);
        if (_boundMethodCache.TryGetValue(unbound, out var cached))
        {
            return cached;
        }

        var selfVal = PilotValue.FromObject(this);
        var selfOnlyArgs = new[] { selfVal };
        var boundFn = new PilotFunction(
            unbound.Name,
            unbound.Parameters,
            (ctx, args) =>
            {
                if (args.Count == 0)
                {
                    return unbound.Invoke(ctx, selfOnlyArgs);
                }
                var boundArgs = new PilotValue[args.Count + 1];
                boundArgs[0] = selfVal;
                for (int i = 0; i < args.Count; i++)
                {
                    boundArgs[i + 1] = args[i];
                }
                return unbound.Invoke(ctx, boundArgs);
            });

        var result = PilotValue.FromFunction(boundFn);
        _boundMethodCache[unbound] = result;
        return result;
    }

    private PilotFunction BindMethod(PilotFunction unbound) => BindMethodValue(unbound).AsFunction();

    public PilotObject Clone()
    {
        var clone = new PilotObject(ClassDef);
        foreach (var kvp in Fields)
        {
            clone.Fields[kvp.Key] = kvp.Value;
        }
        return clone;
    }

    public bool Equals(PilotObject? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        if (TypeName != other.TypeName) return false;
        if (Fields.Count != other.Fields.Count) return false;
        foreach (var kvp in Fields)
        {
            if (!other.Fields.TryGetValue(kvp.Key, out var otherVal) || !kvp.Value.Equals(otherVal))
                return false;
        }
        return true;
    }

    public override bool Equals(object? obj) => Equals(obj as PilotObject);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(TypeName);
        foreach (var kvp in Fields)
        {
            hash.Add(kvp.Key);
            hash.Add(kvp.Value);
        }
        return hash.ToHashCode();
    }

    public override string ToString()
    {
        if (Fields.Count == 0) return $"{TypeName} {{}}";
        var sb = new StringBuilder();
        sb.Append($"{TypeName} {{ ");
        int i = 0;
        foreach (var kvp in Fields)
        {
            if (i++ > 0) sb.Append(", ");
            sb.Append($"{kvp.Key}: {kvp.Value}");
        }
        sb.Append(" }");
        return sb.ToString();
    }
}
