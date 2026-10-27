using System;
using System.Collections.Concurrent;
using System.Collections.Generic;

namespace Pilot.Runtime.Values;

/// <summary>
/// Clojure-like first-class Keyword / Symbol in Pilot.
/// Keywords are interned, self-evaluating, immutable, and callable as functions for map lookup.
/// E.g. :name, :id, :status, :user/email
/// </summary>
public sealed class PilotKeyword : IEquatable<PilotKeyword>, IComparable<PilotKeyword>
{
    private static readonly ConcurrentDictionary<string, PilotKeyword> Pool = new(StringComparer.Ordinal);

    public string Name { get; }
    public string? Namespace { get; }
    public string FullName { get; }

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
        string key = string.IsNullOrWhiteSpace(ns) ? name.TrimStart(':') : $"{ns.TrimStart(':')}/{name.TrimStart(':')}";
        return Pool.GetOrAdd(key, _ => new PilotKeyword(name.TrimStart(':'), string.IsNullOrWhiteSpace(ns) ? null : ns.TrimStart(':')));
    }

    /// <summary>
    /// Invokes the keyword as a function against a target map, dictionary, or object (Clojure-like (:key map default?)).
    /// </summary>
    public PilotValue Invoke(PilotValue target, PilotValue defaultValue = default)
    {
        if (target.Kind == PilotValueKind.Map && target.AsMap() is { } map)
        {
            var keyVal = PilotValue.FromKeyword(this);
            if (map.TryGetValue(keyVal, out var found))
                return found;

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
            if (obj.Fields.TryGetValue(Name, out var pVal))
                return pVal;

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
