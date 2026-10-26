using System;
using System.Collections;
using System.Collections.Generic;

namespace Pilot.Runtime.Values;

/// <summary>
/// Represents a range sequence in Pilot (e.g. 0..10 or 0..2..10).
/// </summary>
public sealed class PilotRange : IEnumerable<long>, IEquatable<PilotRange>
{
    public long Start { get; }
    public long End { get; }
    public long Step { get; }
    public bool Inclusive { get; }

    public PilotRange(long start, long end, long step = 1, bool inclusive = false)
    {
        if (step == 0)
            throw new ArgumentException("Range step cannot be zero.", nameof(step));

        Start = start;
        End = end;
        Step = step;
        Inclusive = inclusive;
    }

    public long Count
    {
        get
        {
            if (Step > 0)
            {
                if (Start > End) return 0;
                long diff = End - Start;
                return Inclusive ? (diff / Step) + 1 : (diff + Step - 1) / Step;
            }
            else
            {
                if (Start < End) return 0;
                long diff = Start - End;
                long absStep = Math.Abs(Step);
                return Inclusive ? (diff / absStep) + 1 : (diff + absStep - 1) / absStep;
            }
        }
    }

    public bool Contains(long value)
    {
        if (Step > 0)
        {
            if (value < Start) return false;
            if (Inclusive ? value > End : value >= End) return false;
            return (value - Start) % Step == 0;
        }
        else
        {
            if (value > Start) return false;
            if (Inclusive ? value < End : value <= End) return false;
            return (Start - value) % Math.Abs(Step) == 0;
        }
    }

    public IEnumerator<long> GetEnumerator()
    {
        if (Step > 0)
        {
            for (long cur = Start; Inclusive ? cur <= End : cur < End; cur += Step)
            {
                yield return cur;
            }
        }
        else
        {
            for (long cur = Start; Inclusive ? cur >= End : cur > End; cur += Step)
            {
                yield return cur;
            }
        }
    }

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    public bool Equals(PilotRange? other)
    {
        if (other is null) return false;
        return Start == other.Start && End == other.End && Step == other.Step && Inclusive == other.Inclusive;
    }

    public override bool Equals(object? obj) => Equals(obj as PilotRange);

    public override int GetHashCode() => HashCode.Combine(Start, End, Step, Inclusive);

    public override string ToString() =>
        Step == 1
            ? (Inclusive ? $"{Start}..={End}" : $"{Start}..{End}")
            : (Inclusive ? $"{Start}..{Step}..={End}" : $"{Start}..{Step}..{End}");
}
