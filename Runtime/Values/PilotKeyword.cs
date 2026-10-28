using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;

namespace Pilot.Runtime.Values;

/// <summary>
/// Clojure-like first-class interned Keyword / Symbol in Pilot (Milestone 17.1).
/// Keywords are interned, self-evaluating, immutable, support DAG hierarchies (<c>:admin.Derive(:user)</c>,
/// <c>:admin.IsA(:user)</c>), and are callable as functions for map/object/record lookup (<c>:user/role(u, default=:guest)</c>).
/// </summary>
public sealed class PilotKeyword : IEquatable<PilotKeyword>, IComparable<PilotKeyword>
{
    private static readonly ConcurrentDictionary<string, PilotKeyword> Pool = new(StringComparer.Ordinal);
    private static readonly object HierarchyLock = new();
    private static readonly Dictionary<PilotKeyword, HashSet<PilotKeyword>> DirectParents = new();
    private static readonly Dictionary<PilotKeyword, HashSet<PilotKeyword>> DirectChildren = new();

    public string Name { get; }
    public string? Namespace { get; }
    public string FullName { get; }
    public bool IsSpecialForm { get; set; }
    public PilotFunction? SpecialFormHandler { get; set; }

    private PilotKeyword(string name, string? ns)
    {
        Name = name;
        Namespace = string.IsNullOrWhiteSpace(ns) ? null : ns;
        FullName = Namespace != null ? $"{Namespace}/{Name}" : Name;
    }

    public static PilotKeyword Intern(string text)
    {
        if (string.IsNullOrEmpty(text)) text = "empty";
        text = text.TrimStart(':');
        return Pool.GetOrAdd(text, t =>
        {
            int slash = t.IndexOf('/');
            if (slash > 0 && slash < t.Length - 1)
            {
                return new PilotKeyword(t[(slash + 1)..], t[..slash]);
            }
            return new PilotKeyword(t, null);
        });
    }

    public static PilotKeyword Intern(string name, string? ns)
    {
        string cleanName = name.TrimStart(':');
        string? cleanNs = string.IsNullOrWhiteSpace(ns) ? null : ns.TrimStart(':');
        string key = cleanNs == null ? cleanName : $"{cleanNs}/{cleanName}";
        return Pool.GetOrAdd(key, _ => new PilotKeyword(cleanName, cleanNs));
    }

    /// <summary>
    /// Establishes a DAG inheritance relationship <c>this &lt;: parent</c> (<c>:admin.Derive(:user)</c>).
    /// Rejects cycles.
    /// </summary>
    public PilotKeyword Derive(PilotKeyword parent)
    {
        if (parent is null || Equals(parent))
            return this;

        lock (HierarchyLock)
        {
            if (parent.IsAInternal(this))
            {
                throw new InvalidOperationException($"Cyclic keyword hierarchy derivation: ':{parent.FullName}' already derives from ':{FullName}'.");
            }

            if (!DirectParents.TryGetValue(this, out var parents))
            {
                parents = new HashSet<PilotKeyword>();
                DirectParents[this] = parents;
            }
            parents.Add(parent);

            if (!DirectChildren.TryGetValue(parent, out var children))
            {
                children = new HashSet<PilotKeyword>();
                DirectChildren[parent] = children;
            }
            children.Add(this);
        }

        return this;
    }

    /// <summary>
    /// Removes a direct derivation link between <c>this</c> and <paramref name="parent"/>.
    /// </summary>
    public PilotKeyword Underive(PilotKeyword parent)
    {
        if (parent is null) return this;
        lock (HierarchyLock)
        {
            if (DirectParents.TryGetValue(this, out var parents))
                parents.Remove(parent);
            if (DirectChildren.TryGetValue(parent, out var children))
                children.Remove(this);
        }
        return this;
    }

    /// <summary>
    /// Returns true if <c>this</c> is equal to <paramref name="ancestor"/> or transitively derives from it.
    /// </summary>
    public bool IsA(PilotKeyword? ancestor)
    {
        if (ancestor is null) return false;
        if (Equals(ancestor)) return true;
        lock (HierarchyLock)
        {
            return IsAInternal(ancestor);
        }
    }

    /// <summary>
    /// Returns the shortest derivation distance to <paramref name="ancestor"/> (0 if equal, 1 if direct parent, -1 if unrelated).
    /// Used by keyword-dispatched method groups for specificity ranking.
    /// </summary>
    public int GetDerivationDistance(PilotKeyword? ancestor)
    {
        if (ancestor is null) return -1;
        if (Equals(ancestor)) return 0;

        lock (HierarchyLock)
        {
            var visited = new HashSet<PilotKeyword> { this };
            var queue = new Queue<(PilotKeyword Kw, int Depth)>();
            queue.Enqueue((this, 0));

            while (queue.Count > 0)
            {
                var (cur, depth) = queue.Dequeue();
                if (DirectParents.TryGetValue(cur, out var parents))
                {
                    foreach (var p in parents)
                    {
                        if (p.Equals(ancestor))
                            return depth + 1;
                        if (visited.Add(p))
                            queue.Enqueue((p, depth + 1));
                    }
                }
            }
            return -1;
        }
    }

    private bool IsAInternal(PilotKeyword ancestor)
    {
        if (Equals(ancestor)) return true;
        var visited = new HashSet<PilotKeyword> { this };
        var stack = new Stack<PilotKeyword>();
        stack.Push(this);

        while (stack.Count > 0)
        {
            var cur = stack.Pop();
            if (DirectParents.TryGetValue(cur, out var parents))
            {
                foreach (var p in parents)
                {
                    if (p.Equals(ancestor))
                        return true;
                    if (visited.Add(p))
                        stack.Push(p);
                }
            }
        }
        return false;
    }

    public IReadOnlyList<PilotKeyword> GetParents()
    {
        lock (HierarchyLock)
        {
            if (DirectParents.TryGetValue(this, out var parents))
                return parents.OrderBy(k => k.FullName, StringComparer.Ordinal).ToList();
            return Array.Empty<PilotKeyword>();
        }
    }

    public IReadOnlyList<PilotKeyword> GetAncestors()
    {
        lock (HierarchyLock)
        {
            var result = new HashSet<PilotKeyword>();
            var stack = new Stack<PilotKeyword>();
            stack.Push(this);
            while (stack.Count > 0)
            {
                var cur = stack.Pop();
                if (DirectParents.TryGetValue(cur, out var parents))
                {
                    foreach (var p in parents)
                    {
                        if (result.Add(p))
                            stack.Push(p);
                    }
                }
            }
            return result.OrderBy(k => k.FullName, StringComparer.Ordinal).ToList();
        }
    }

    public IReadOnlyList<PilotKeyword> GetDescendants()
    {
        lock (HierarchyLock)
        {
            var result = new HashSet<PilotKeyword>();
            var stack = new Stack<PilotKeyword>();
            stack.Push(this);
            while (stack.Count > 0)
            {
                var cur = stack.Pop();
                if (DirectChildren.TryGetValue(cur, out var children))
                {
                    foreach (var c in children)
                    {
                        if (result.Add(c))
                            stack.Push(c);
                    }
                }
            }
            return result.OrderBy(k => k.FullName, StringComparer.Ordinal).ToList();
        }
    }

    /// <summary>
    /// Invokes the keyword as a function against a target map, dictionary, object, or record
    /// (e.g. <c>:user/role(u, default=:guest)</c> or <c>:id(u)</c>).
    /// </summary>
    public PilotValue Invoke(PilotValue target, PilotValue defaultValue = default)
    {
        if (target.Kind == PilotValueKind.Map && target.AsMap() is { } map)
        {
            var keyVal = PilotValue.FromKeyword(this);
            if (map.TryGetValue(keyVal, out var found))
                return found;

            // Check hierarchy-compatible keyword keys if any key in map is a sub-keyword of `this`
            foreach (var kvp in map)
            {
                if (kvp.Key.Kind == PilotValueKind.Keyword && kvp.Key.AsKeyword().IsA(this))
                    return kvp.Value;
            }

            // Fallback to string key lookup for interop
            var strKey = PilotValue.FromString(FullName);
            if (map.TryGetValue(strKey, out var strFound))
                return strFound;

            var shortKey = PilotValue.FromString(Name);
            if (map.TryGetValue(shortKey, out var shortFound))
                return shortFound;

            return defaultValue.Kind != PilotValueKind.Undefined ? defaultValue : PilotValue.Null;
        }

        if (target.Kind == PilotValueKind.Object && target.AsObject() is { } obj)
        {
            if (obj.Fields.TryGetValue(FullName, out var fullVal))
                return fullVal;
            if (obj.Fields.TryGetValue(Name, out var pVal))
                return pVal;

            return defaultValue.Kind != PilotValueKind.Undefined ? defaultValue : PilotValue.Null;
        }

        if (target.Kind == PilotValueKind.Record)
        {
            var rd = target.GetRecordDefinition();
            int fIdx = rd.GetFieldIndex(FullName);
            if (fIdx < 0) fIdx = rd.GetFieldIndex(Name);
            if (fIdx >= 0)
                return target.GetRecordField(fIdx);

            return defaultValue.Kind != PilotValueKind.Undefined ? defaultValue : PilotValue.Null;
        }

        return defaultValue.Kind != PilotValueKind.Undefined ? defaultValue : PilotValue.Null;
    }

    public bool Equals(PilotKeyword? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        return string.Equals(FullName, other.FullName, StringComparison.Ordinal);
    }

    public override bool Equals(object? obj) => obj is PilotKeyword kw && Equals(kw);
    public override int GetHashCode() => StringComparer.Ordinal.GetHashCode(FullName);
    public override string ToString() => $":{FullName}";
    public int CompareTo(PilotKeyword? other) => string.CompareOrdinal(FullName, other?.FullName);

    public static bool operator ==(PilotKeyword? left, PilotKeyword? right) => Equals(left, right);
    public static bool operator !=(PilotKeyword? left, PilotKeyword? right) => !Equals(left, right);
}
