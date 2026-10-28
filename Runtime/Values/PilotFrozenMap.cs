using System;
using System.Collections.Generic;

namespace Pilot.Runtime.Values;

/// <summary>
/// Compile-time frozen map (`19.2`) backed by a static minimal displacement perfect-hash table
/// suitable for direct `.pilc` constant-pool serialization and native `.rodata` emission.
/// Inherits from <see cref="Dictionary{TKey, TValue}"/> so all existing map consumers
/// (<see cref="PilotValue.AsMap"/>) interoperate seamlessly while PilVM and native targets
/// use the $O(1)$ displacement table directly.
/// </summary>
public sealed class PilotFrozenMap : Dictionary<PilotValue, PilotValue>
{
    public int TableSize { get; }
    public int[] BucketDisplacements { get; }
    public PilotValue[] SlotKeys { get; }
    public PilotValue[] SlotValues { get; }
    public bool IsPerfectHash { get; }

    public PilotFrozenMap(IDictionary<PilotValue, PilotValue> source)
        : base(source)
    {
        int n = source.Count;
        if (n == 0)
        {
            TableSize = 0;
            BucketDisplacements = Array.Empty<int>();
            SlotKeys = Array.Empty<PilotValue>();
            SlotValues = Array.Empty<PilotValue>();
            IsPerfectHash = true;
            return;
        }

        // Next power of two >= 2 * n for fast mask indexing
        int size = 1;
        while (size < n * 2) size <<= 1;

        TableSize = size;
        BucketDisplacements = new int[size];
        SlotKeys = new PilotValue[size];
        SlotValues = new PilotValue[size];
        var occupied = new bool[size];

        int mask = size - 1;
        var buckets = new List<(PilotValue Key, PilotValue Value, uint Hash)>[size];
        foreach (var kvp in source)
        {
            uint h = ComputeDeterministicHash(kvp.Key);
            int b = (int)(h & (uint)mask);
            buckets[b] ??= new List<(PilotValue, PilotValue, uint)>();
            buckets[b].Add((kvp.Key, kvp.Value, h));
        }

        // Sort buckets by descending size (CHD / FKS perfect hash construction)
        var bucketOrder = new List<int>(size);
        for (int i = 0; i < size; i++)
        {
            if (buckets[i] is { Count: > 0 })
            {
                bucketOrder.Add(i);
            }
        }
        bucketOrder.Sort((a, b) => buckets[b].Count.CompareTo(buckets[a].Count));

        bool success = true;
        var candidateSlots = new List<int>(8);
        foreach (int b in bucketOrder)
        {
            var items = buckets[b];
            bool placed = false;

            for (int disp = 1; disp <= size * 4; disp++)
            {
                candidateSlots.Clear();
                bool collision = false;

                for (int i = 0; i < items.Count; i++)
                {
                    int slot = (int)((MixHash(items[i].Hash, (uint)disp)) & (uint)mask);
                    if (occupied[slot] || candidateSlots.Contains(slot))
                    {
                        collision = true;
                        break;
                    }
                    candidateSlots.Add(slot);
                }

                if (!collision)
                {
                    BucketDisplacements[b] = disp;
                    for (int i = 0; i < items.Count; i++)
                    {
                        int slot = candidateSlots[i];
                        occupied[slot] = true;
                        SlotKeys[slot] = items[i].Key;
                        SlotValues[slot] = items[i].Value;
                    }
                    placed = true;
                    break;
                }
            }

            if (!placed)
            {
                success = false;
                break;
            }
        }

        IsPerfectHash = success;
    }

    /// <summary>
    /// Performs an $O(1)$ zero-chain lookup using the precomputed displacement table.
    /// </summary>
    public bool TryLookupFast(PilotValue key, out PilotValue value)
    {
        if (TableSize == 0)
        {
            value = PilotValue.Undefined;
            return false;
        }

        if (IsPerfectHash)
        {
            int mask = TableSize - 1;
            uint h = ComputeDeterministicHash(key);
            int b = (int)(h & (uint)mask);
            int disp = BucketDisplacements[b];
            if (disp <= 0)
            {
                value = PilotValue.Undefined;
                return false;
            }

            int slot = (int)(MixHash(h, (uint)disp) & (uint)mask);
            if (SlotKeys[slot].Equals(key))
            {
                value = SlotValues[slot];
                return true;
            }

            value = PilotValue.Undefined;
            return false;
        }

        return TryGetValue(key, out value);
    }

    public static uint ComputeDeterministicHash(PilotValue key)
    {
        unchecked
        {
            return key.Kind switch
            {
                PilotValueKind.Int or PilotValueKind.UInt or PilotValueKind.Bool or PilotValueKind.Char =>
                    Mix64((ulong)key.AsInt()),
                PilotValueKind.Keyword =>
                    Mix64((ulong)key.AsKeyword().Id ^ 0x9E3779B97F4A7C15UL),
                PilotValueKind.String =>
                    Fnv1aString(key.AsString()),
                _ => (uint)key.GetHashCode()
            };
        }
    }

    private static uint MixHash(uint h, uint disp)
    {
        unchecked
        {
            uint x = h ^ (disp * 0x9E3779B9u);
            x ^= x >> 16;
            x *= 0x85EBCA6Bu;
            x ^= x >> 13;
            x *= 0xC2B2AE35u;
            x ^= x >> 16;
            return x;
        }
    }

    private static uint Mix64(ulong z)
    {
        unchecked
        {
            z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9UL;
            z = (z ^ (z >> 27)) * 0x94D049BB133111EBUL;
            z = z ^ (z >> 31);
            return (uint)z;
        }
    }

    private static uint Fnv1aString(string s)
    {
        unchecked
        {
            uint hash = 2166136261u;
            for (int i = 0; i < s.Length; i++)
            {
                hash ^= s[i];
                hash *= 16777619u;
            }
            return hash;
        }
    }
}
