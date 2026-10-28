using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Text;
using Antlr4.Runtime;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.NativeMemory;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Effects;

/// <summary>
/// Categories of side-effects intercepted by Mode 7 (<c>fn!?(a, b) -&gt; EffectPlan[TReturn]</c>, Milestone 18.3).
/// </summary>
public enum EffectDiffKind
{
    StoreField,
    ArrayWrite,
    MapWrite,
    DatabaseExec,
    FileWrite,
    VarWrite,
    ConsoleOutput
}

/// <summary>
/// A single side-effect diff captured in a Copy-on-Write <see cref="ShadowEffectContext"/> during <c>fn!?(...)</c>.
/// </summary>
public sealed class PilotEffectDiff
{
    public EffectDiffKind Kind { get; }
    public string Target { get; }
    public string PropertyOrKey { get; }
    public PilotValue OldValue { get; }
    public PilotValue NewValue { get; }
    public string Summary { get; }
    internal Action? CommitAction { get; }

    public PilotEffectDiff(
        EffectDiffKind kind,
        string target,
        string propertyOrKey,
        PilotValue oldValue,
        PilotValue newValue,
        string summary,
        Action? commitAction = null)
    {
        Kind = kind;
        Target = target;
        PropertyOrKey = propertyOrKey;
        OldValue = oldValue;
        NewValue = newValue;
        Summary = summary;
        CommitAction = commitAction;
    }

    public PilotObject ToPilotObject()
    {
        var cls = new PilotClassDefinition("EffectDiff");
        var obj = new PilotObject(cls)
        {
            NativePayload = this
        };

        obj.Fields["Kind"] = PilotValue.FromString(Kind.ToString());
        obj.Fields["kind"] = obj.Fields["Kind"];
        obj.Fields["Target"] = PilotValue.FromString(Target);
        obj.Fields["target"] = obj.Fields["Target"];
        obj.Fields["Property"] = PilotValue.FromString(PropertyOrKey);
        obj.Fields["property"] = obj.Fields["Property"];
        obj.Fields["field"] = obj.Fields["Property"];
        obj.Fields["key"] = obj.Fields["Property"];
        obj.Fields["OldValue"] = OldValue;
        obj.Fields["old_value"] = OldValue;
        obj.Fields["oldValue"] = OldValue;
        obj.Fields["NewValue"] = NewValue;
        obj.Fields["new_value"] = NewValue;
        obj.Fields["newValue"] = NewValue;
        obj.Fields["Summary"] = PilotValue.FromString(Summary);
        obj.Fields["summary"] = obj.Fields["Summary"];
        return obj;
    }

    public override string ToString() => $"[{Kind}] {Summary}";
}

/// <summary>
/// Thread-local Copy-on-Write (CoW) transactional shadow memory context for <c>fn!?(a, b)</c>.
/// Redirects <c>StoreField</c>, <c>ArrayWrite</c>, <c>DatabaseExec</c>, <c>FileWrite</c>, and outer <c>VarWrite</c>
/// operations into a diff log without mutating live memory.
/// </summary>
public sealed class ShadowEffectContext
{
    public ShadowEffectContext? Parent { get; }
    public string FunctionName { get; }
    public List<PilotEffectDiff> Diffs { get; } = new();

    internal readonly HashSet<PilotObject> NewlyAllocatedObjects = new(ReferenceEqualityComparer.Instance);
    internal readonly HashSet<PilotScope> LocalDryRunScopes = new(ReferenceEqualityComparer.Instance);
    internal readonly Dictionary<(PilotObject Obj, string Field), PilotValue> ObjectFieldOverlay = new();
    internal readonly Dictionary<List<PilotValue>, List<PilotValue>> ListOverlay = new(ReferenceEqualityComparer.Instance);
    internal readonly Dictionary<Dictionary<PilotValue, PilotValue>, Dictionary<PilotValue, PilotValue>> MapOverlay = new(ReferenceEqualityComparer.Instance);
    internal readonly Dictionary<(PilotSharedBuffer Buf, int Index), PilotValue> BufferOverlay = new();
    internal readonly Dictionary<(PilotNativeArray Arr, int Index), PilotValue> NativeArrayOverlay = new();
    internal readonly Dictionary<(PilotScope Scope, string Name), PilotValue> ScopeVarOverlay = new();
    internal readonly Dictionary<string, string?> FileContentOverlay = new(StringComparer.OrdinalIgnoreCase);
    internal readonly StringBuilder CapturedOutput = new();

    public ShadowEffectContext(string functionName, ShadowEffectContext? parent = null)
    {
        FunctionName = functionName;
        Parent = parent;
    }
}

/// <summary>
/// Result of Mode 7 (<c>fn!?(a, b) -&gt; EffectPlan[TReturn]</c>): holds the dry-run return value,
/// the ordered <see cref="PilotEffectDiff"/> log, and <c>.Commit()</c> / <c>.Inspect()</c> methods.
/// </summary>
public sealed class PilotEffectPlan
{
    public string FunctionName { get; }
    public PilotValue Value { get; }
    public IReadOnlyList<PilotEffectDiff> Diffs { get; }
    public string CapturedConsoleOutput { get; }
    public bool IsCommitted { get; private set; }
    public bool IsRolledBack { get; private set; }

    public PilotEffectPlan(
        string functionName,
        PilotValue value,
        IReadOnlyList<PilotEffectDiff> diffs,
        string capturedConsoleOutput)
    {
        FunctionName = functionName;
        Value = value;
        Diffs = diffs;
        CapturedConsoleOutput = capturedConsoleOutput;
    }

    public string Inspect()
    {
        if (Diffs.Count == 0)
        {
            return $"EffectPlan[{FunctionName}] => {Value} (0 diffs, pure)";
        }

        var sb = new StringBuilder();
        sb.Append($"EffectPlan[{FunctionName}] => {Value} ({Diffs.Count} diff{(Diffs.Count == 1 ? "" : "s")}): ");
        for (int i = 0; i < Diffs.Count; i++)
        {
            if (i > 0) sb.Append("; ");
            sb.Append(Diffs[i]);
        }
        return sb.ToString();
    }

    public PilotValue Commit()
    {
        if (IsRolledBack)
        {
            throw new PilotRuntimeException($"Cannot commit EffectPlan[{FunctionName}] after it has been rolled back.");
        }

        if (!IsCommitted)
        {
            PilotEffectDryRunEngine.ExecuteCommitActions(Diffs);
            IsCommitted = true;
        }

        return Value;
    }

    public PilotValue Rollback()
    {
        if (!IsCommitted)
        {
            IsRolledBack = true;
        }
        return PilotValue.FromBool(IsRolledBack);
    }

    public PilotObject ToPilotObject()
    {
        var cls = new PilotClassDefinition("EffectPlan");
        var obj = new PilotObject(cls)
        {
            NativePayload = this,
            DynamicMemberResolver = member => member switch
            {
                "IsCommitted" or "is_committed" or "isCommitted" => PilotValue.FromBool(IsCommitted),
                "IsRolledBack" or "is_rolled_back" or "isRolledBack" => PilotValue.FromBool(IsRolledBack),
                _ => null
            }
        };

        var diffObjects = Diffs.Select(d => PilotValue.FromObject(d.ToPilotObject())).ToList();
        var diffListVal = PilotValue.FromList(diffObjects);

        obj.Fields["Value"] = Value;
        obj.Fields["value"] = Value;
        obj.Fields["Diffs"] = diffListVal;
        obj.Fields["diffs"] = diffListVal;
        obj.Fields["diff_count"] = PilotValue.FromInt(Diffs.Count);
        obj.Fields["DiffCount"] = PilotValue.FromInt(Diffs.Count);
        obj.Fields["is_pure"] = PilotValue.FromBool(Diffs.Count == 0);
        obj.Fields["IsPure"] = PilotValue.FromBool(Diffs.Count == 0);
        obj.Fields["stdout"] = PilotValue.FromString(CapturedConsoleOutput);

        obj.Fields["Commit"] = PilotValue.FromFunction(PilotFunction.CreateNative(
            "Commit",
            (_, _) => Commit()));
        obj.Fields["commit"] = obj.Fields["Commit"];

        obj.Fields["Inspect"] = PilotValue.FromFunction(PilotFunction.CreateNative(
            "Inspect",
            (_, _) => PilotValue.FromString(Inspect())));
        obj.Fields["inspect"] = obj.Fields["Inspect"];

        obj.Fields["Rollback"] = PilotValue.FromFunction(PilotFunction.CreateNative(
            "Rollback",
            (_, _) => Rollback()));
        obj.Fields["rollback"] = obj.Fields["Rollback"];

        return obj;
    }
}

/// <summary>
/// Runtime coordinator for Mode 7 (<c>fn!?(a, b) -&gt; EffectPlan[TReturn]</c>, Milestone 18.3).
/// </summary>
public static class PilotEffectDryRunEngine
{
    [ThreadStatic]
    private static ShadowEffectContext? _currentShadow;

    [ThreadStatic]
    private static bool _isCommitting;

    public static ShadowEffectContext? CurrentShadow => _isCommitting ? null : _currentShadow;

    public static bool IsDryRunActive => _currentShadow != null && !_isCommitting;

    public static void NotifyObjectCreated(PilotObject obj)
    {
        if (_currentShadow != null && !_isCommitting)
        {
            _currentShadow.NewlyAllocatedObjects.Add(obj);
        }
    }

    public static void NotifyScopeCreated(PilotScope scope)
    {
        if (_currentShadow != null && !_isCommitting)
        {
            _currentShadow.LocalDryRunScopes.Add(scope);
        }
    }

    /// <summary>
    /// Executes a call-quoted dry-run <c>fn!?(args)</c> inside a fresh <see cref="ShadowEffectContext"/>.
    /// </summary>
    public static PilotEffectPlan EvaluateEffectDryRun(
        PilotFunction? fn,
        string calleeName,
        PilotParser.ArgumentContext[] args,
        ParserRuleContext callSiteCtx,
        PilotInterpreterVisitor visitor)
    {
        if (fn == null)
        {
            throw new PilotRuntimeException(
                $"Cannot execute effect dry-run '{calleeName}!?(...)': target did not resolve to a callable function.",
                callSiteCtx.Start.Line,
                callSiteCtx.Start.Column);
        }

        // Evaluate arguments prior to entering the dry-run shadow context
        var evalArgs = new List<PilotValue>(args.Length);
        var namedArgs = new Dictionary<string, PilotValue>(StringComparer.Ordinal);
        foreach (var a in args)
        {
            if (a.test() == null) continue;
            var v = visitor.Visit(a.test());
            if (a.identifier() != null && (a.EQUAL() != null || a.COLON() != null))
            {
                namedArgs[a.identifier().GetText()] = v;
            }
            else
            {
                evalArgs.Add(v);
            }
        }

        var prevShadow = _currentShadow;
        var shadow = new ShadowEffectContext(calleeName, prevShadow);
        _currentShadow = shadow;

        var prevStdout = visitor.Context.Options.StandardOutput;
        using var shadowWriter = new StringWriter(shadow.CapturedOutput);
        visitor.Context.Options.StandardOutput = shadowWriter;

        PilotValue returnVal;
        try
        {
            returnVal = fn.Invoke(visitor.Context, evalArgs);
        }
        finally
        {
            shadowWriter.Flush();
            visitor.Context.Options.StandardOutput = prevStdout;
            _currentShadow = prevShadow;
        }

        return new PilotEffectPlan(
            calleeName,
            returnVal,
            shadow.Diffs,
            shadow.CapturedOutput.ToString());
    }

    internal static void ExecuteCommitActions(IReadOnlyList<PilotEffectDiff> diffs)
    {
        bool prevCommitting = _isCommitting;
        _isCommitting = true;
        try
        {
            foreach (var diff in diffs)
            {
                diff.CommitAction?.Invoke();
            }
        }
        finally
        {
            _isCommitting = prevCommitting;
        }
    }

    #region CoW Object Field Interception (StoreField)

    public static bool TryReadObjectField(PilotObject obj, string fieldName, out PilotValue shadowValue)
    {
        var shadow = CurrentShadow;
        while (shadow != null)
        {
            if (shadow.ObjectFieldOverlay.TryGetValue((obj, fieldName), out shadowValue))
            {
                return true;
            }
            shadow = shadow.Parent;
        }

        shadowValue = PilotValue.Undefined;
        return false;
    }

    public static bool TryRecordObjectFieldWrite(PilotObject obj, string fieldName, PilotValue newValue)
    {
        var shadow = CurrentShadow;
        if (shadow == null) return false;

        // Objects allocated internally during this dry-run can mutate their own fields directly
        if (shadow.NewlyAllocatedObjects.Contains(obj))
        {
            return false;
        }

        PilotValue oldValue = TryReadObjectField(obj, fieldName, out var existingShadow)
            ? existingShadow
            : (obj.Fields.TryGetValue(fieldName, out var liveVal) ? liveVal : PilotValue.Undefined);

        shadow.ObjectFieldOverlay[(obj, fieldName)] = newValue;
        string targetDesc = $"{obj.TypeName}.{fieldName}";
        string summary = $"{targetDesc}: {oldValue} -> {newValue}";

        shadow.Diffs.Add(new PilotEffectDiff(
            EffectDiffKind.StoreField,
            targetDesc,
            fieldName,
            oldValue,
            newValue,
            summary,
            commitAction: () => obj.SetMember(fieldName, newValue)));

        return true;
    }

    #endregion

    #region CoW Scope Variable Interception (VarWrite)

    public static bool TryReadScopeVariable(PilotScope definingScope, string name, out PilotValue shadowValue)
    {
        var shadow = CurrentShadow;
        while (shadow != null)
        {
            if (shadow.ScopeVarOverlay.TryGetValue((definingScope, name), out shadowValue))
            {
                return true;
            }
            shadow = shadow.Parent;
        }

        shadowValue = PilotValue.Undefined;
        return false;
    }

    public static bool TryRecordScopeVariableWrite(
        PilotScope definingScope,
        string name,
        PilotValue currentLiveValue,
        PilotValue newValue)
    {
        var shadow = CurrentShadow;
        if (shadow == null) return false;

        // Scopes created inside the dry-run (e.g., local function frame) mutate their own local variables normally
        if (shadow.LocalDryRunScopes.Contains(definingScope))
        {
            return false;
        }

        PilotValue oldValue = TryReadScopeVariable(definingScope, name, out var prevShadow)
            ? prevShadow
            : currentLiveValue;

        shadow.ScopeVarOverlay[(definingScope, name)] = newValue;
        string summary = $"{name}: {oldValue} -> {newValue}";

        shadow.Diffs.Add(new PilotEffectDiff(
            EffectDiffKind.VarWrite,
            name,
            name,
            oldValue,
            newValue,
            summary,
            commitAction: () => definingScope.Assign(name, newValue)));

        return true;
    }

    #endregion

    #region CoW Array / List / Map / SliceBuffer Interception (ArrayWrite & MapWrite)

    public static List<PilotValue> GetReadableList(List<PilotValue> liveList)
    {
        var shadow = CurrentShadow;
        while (shadow != null)
        {
            if (shadow.ListOverlay.TryGetValue(liveList, out var cloned))
            {
                return cloned;
            }
            shadow = shadow.Parent;
        }
        return liveList;
    }

    public static bool TryRecordListIndexWrite(List<PilotValue> liveList, int index, PilotValue newValue)
    {
        var shadow = CurrentShadow;
        if (shadow == null) return false;

        if (!shadow.ListOverlay.TryGetValue(liveList, out var shadowList))
        {
            shadowList = new List<PilotValue>(GetReadableList(liveList));
            shadow.ListOverlay[liveList] = shadowList;
        }

        int resolvedIdx = index < 0 ? index + shadowList.Count : index;
        if (resolvedIdx < 0 || resolvedIdx >= shadowList.Count)
        {
            return true;
        }

        var oldValue = shadowList[resolvedIdx];
        shadowList[resolvedIdx] = newValue;

        string targetDesc = $"List[{resolvedIdx}]";
        shadow.Diffs.Add(new PilotEffectDiff(
            EffectDiffKind.ArrayWrite,
            targetDesc,
            resolvedIdx.ToString(),
            oldValue,
            newValue,
            $"{targetDesc}: {oldValue} -> {newValue}",
            commitAction: () =>
            {
                int liveIdx = index < 0 ? index + liveList.Count : index;
                if (liveIdx >= 0 && liveIdx < liveList.Count)
                {
                    liveList[liveIdx] = newValue;
                }
            }));

        return true;
    }

    public static bool TryRecordListAppend(List<PilotValue> liveList, PilotValue item)
    {
        var shadow = CurrentShadow;
        if (shadow == null) return false;

        if (!shadow.ListOverlay.TryGetValue(liveList, out var shadowList))
        {
            shadowList = new List<PilotValue>(GetReadableList(liveList));
            shadow.ListOverlay[liveList] = shadowList;
        }

        int appendIdx = shadowList.Count;
        shadowList.Add(item);

        string targetDesc = $"List[{appendIdx}]";
        shadow.Diffs.Add(new PilotEffectDiff(
            EffectDiffKind.ArrayWrite,
            targetDesc,
            appendIdx.ToString(),
            PilotValue.Undefined,
            item,
            $"List.append({item}) at [{appendIdx}]",
            commitAction: () => liveList.Add(item)));

        return true;
    }

    public static bool TryReadMapEntry(Dictionary<PilotValue, PilotValue> liveMap, PilotValue key, out PilotValue value)
    {
        var shadow = CurrentShadow;
        while (shadow != null)
        {
            if (shadow.MapOverlay.TryGetValue(liveMap, out var shadowMap))
            {
                return shadowMap.TryGetValue(key, out value);
            }
            shadow = shadow.Parent;
        }

        return liveMap.TryGetValue(key, out value);
    }

    public static bool TryRecordMapWrite(Dictionary<PilotValue, PilotValue> liveMap, PilotValue key, PilotValue newValue)
    {
        var shadow = CurrentShadow;
        if (shadow == null) return false;

        if (!shadow.MapOverlay.TryGetValue(liveMap, out var shadowMap))
        {
            shadowMap = new Dictionary<PilotValue, PilotValue>(liveMap);
            shadow.MapOverlay[liveMap] = shadowMap;
        }

        var oldValue = shadowMap.TryGetValue(key, out var existing) ? existing : PilotValue.Undefined;
        shadowMap[key] = newValue;

        string targetDesc = $"Map[{key}]";
        shadow.Diffs.Add(new PilotEffectDiff(
            EffectDiffKind.MapWrite,
            targetDesc,
            key.ToString(),
            oldValue,
            newValue,
            $"{targetDesc}: {oldValue} -> {newValue}",
            commitAction: () => liveMap[key] = newValue));

        return true;
    }

    public static bool TryReadSliceBufferElement(PilotSharedBuffer buffer, int index, out PilotValue shadowValue)
    {
        var shadow = CurrentShadow;
        while (shadow != null)
        {
            if (shadow.BufferOverlay.TryGetValue((buffer, index), out shadowValue))
            {
                return true;
            }
            shadow = shadow.Parent;
        }

        shadowValue = PilotValue.Undefined;
        return false;
    }

    public static bool TryRecordSliceBufferWrite(PilotSharedBuffer buffer, int index, PilotValue newValue)
    {
        var shadow = CurrentShadow;
        if (shadow == null) return false;

        var oldValue = TryReadSliceBufferElement(buffer, index, out var prev)
            ? prev
            : buffer.GetElement(index);

        shadow.BufferOverlay[(buffer, index)] = newValue;
        string targetDesc = $"SliceBuffer[{index}]";
        shadow.Diffs.Add(new PilotEffectDiff(
            EffectDiffKind.ArrayWrite,
            targetDesc,
            index.ToString(),
            oldValue,
            newValue,
            $"{targetDesc}: {oldValue} -> {newValue}",
            commitAction: () => buffer.SetElement(index, newValue)));

        return true;
    }

    public static bool TryReadNativeArrayElement(PilotNativeArray array, int index, out PilotValue shadowValue)
    {
        var shadow = CurrentShadow;
        while (shadow != null)
        {
            if (shadow.NativeArrayOverlay.TryGetValue((array, index), out shadowValue))
            {
                return true;
            }
            shadow = shadow.Parent;
        }

        shadowValue = PilotValue.Undefined;
        return false;
    }

    public static bool TryRecordNativeArrayWrite(PilotNativeArray array, int index, PilotValue newValue)
    {
        var shadow = CurrentShadow;
        if (shadow == null) return false;

        var oldValue = TryReadNativeArrayElement(array, index, out var prev)
            ? prev
            : array.GetValue(index);

        shadow.NativeArrayOverlay[(array, index)] = newValue;
        string targetDesc = $"NativeArray[{index}]";
        shadow.Diffs.Add(new PilotEffectDiff(
            EffectDiffKind.ArrayWrite,
            targetDesc,
            index.ToString(),
            oldValue,
            newValue,
            $"{targetDesc}: {oldValue} -> {newValue}",
            commitAction: () => array.SetValue(index, newValue)));

        return true;
    }

    #endregion

    #region CoW Database & File I/O Interception (DatabaseExec & FileWrite)

    public static bool TryRecordDatabaseExec(
        string operation,
        string tableName,
        string statementText,
        PilotValue payload,
        Action? commitAction = null)
    {
        var shadow = CurrentShadow;
        if (shadow == null) return false;

        string targetDesc = $"db.{tableName}";
        string summary = $"{operation} on {targetDesc}: {statementText}";
        shadow.Diffs.Add(new PilotEffectDiff(
            EffectDiffKind.DatabaseExec,
            targetDesc,
            operation,
            PilotValue.Null,
            payload,
            summary,
            commitAction));

        return true;
    }

    public static bool TryReadFileContent(string filePath, out string? content)
    {
        var shadow = CurrentShadow;
        string fullPath = Path.GetFullPath(filePath);
        while (shadow != null)
        {
            if (shadow.FileContentOverlay.TryGetValue(fullPath, out content))
            {
                return true;
            }
            shadow = shadow.Parent;
        }

        content = null;
        return false;
    }

    public static bool TryRecordFileWrite(string filePath, string content, bool append = false)
    {
        var shadow = CurrentShadow;
        if (shadow == null) return false;

        string fullPath = Path.GetFullPath(filePath);
        string? oldContent = TryReadFileContent(fullPath, out var existingShadow)
            ? existingShadow
            : (File.Exists(fullPath) ? File.ReadAllText(fullPath) : null);

        string nextContent = append && oldContent != null ? oldContent + content : content;
        shadow.FileContentOverlay[fullPath] = nextContent;

        string targetDesc = $"file:{Path.GetFileName(fullPath)}";
        PilotValue oldVal = oldContent != null ? PilotValue.FromString(oldContent) : PilotValue.Null;
        PilotValue newVal = PilotValue.FromString(nextContent);

        shadow.Diffs.Add(new PilotEffectDiff(
            EffectDiffKind.FileWrite,
            targetDesc,
            fullPath,
            oldVal,
            newVal,
            $"{(append ? "append_file" : "write_file")}({Path.GetFileName(fullPath)}, {content.Length} bytes)",
            commitAction: () =>
            {
                if (append)
                {
                    File.AppendAllText(fullPath, content);
                }
                else
                {
                    File.WriteAllText(fullPath, content);
                }
            }));

        return true;
    }

    public static bool TryRecordFileDelete(string filePath)
    {
        var shadow = CurrentShadow;
        if (shadow == null) return false;

        string fullPath = Path.GetFullPath(filePath);
        string? oldContent = TryReadFileContent(fullPath, out var existingShadow)
            ? existingShadow
            : (File.Exists(fullPath) ? File.ReadAllText(fullPath) : null);

        shadow.FileContentOverlay[fullPath] = null;
        string targetDesc = $"file:{Path.GetFileName(fullPath)}";

        shadow.Diffs.Add(new PilotEffectDiff(
            EffectDiffKind.FileWrite,
            targetDesc,
            fullPath,
            oldContent != null ? PilotValue.FromString(oldContent) : PilotValue.Null,
            PilotValue.Null,
            $"delete_file({Path.GetFileName(fullPath)})",
            commitAction: () =>
            {
                if (File.Exists(fullPath))
                {
                    File.Delete(fullPath);
                }
            }));

        return true;
    }

    #endregion
}
