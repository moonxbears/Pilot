// Code transpiled from Pilot to C# (.NET 10)
#nullable enable
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;

namespace PilotTranspiledApp;

public static class ProgramModule
{
    public enum Priority
    {
        Low = 1,
        Medium = 2,
        High = 3,
    }

    public abstract record Shape;
    public sealed record Circle(double r) : Shape;
    public sealed record Rect(double w, double h) : Shape;

    public class Counter
    {
        public long step(long x, long delta)
        {
            return x + delta;
        }
    }

    public static long gcd(long a, long b)
    {
        long x = a;
        long y = b;
        while (y != 0)
        {
            long rem = x % y;
            x = y;
            y = rem;
        }
        return x;
    }

    public static long compute_weighted_sum(long limit, long scale)
    {
        long base_bonus = (limit + 2) * scale;
        long total = base_bonus;
        for (long i = 1; i <= limit; ++i)
        {
            if (i % 2 == 0)
            {
                total += i * scale;
            }
            else if (i == 5)
            {
                total += 10;
            }
            else
            {
                total += i;
            }
        }
        return total;
    }

    public static void RunMain()
    {
        const long BASE_SCALE = 3;
        long g = gcd(48, 18);
        long weighted = compute_weighted_sum(5, BASE_SCALE);
        var squares = (Enumerable.Range((int)(1), (int)((6 - 1 + 1)))).Where(x => x > 2).Select(x => x * x).ToList();
        if (!(g == 6)) throw new InvalidOperationException("Assertion failed");
        if (!(weighted == 53)) throw new InvalidOperationException("Assertion failed");
        Console.WriteLine($"GCD={g}, Weighted={weighted}");
    }
}

public static class i64PilotExtensions
{
    public static long doubled(this long self_val)
    {
        return self_val * 2;
    }
}
