// Code transpiled & optimized from Pilot to Java 21+
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import java.util.function.*;
import java.util.stream.*;

class ProgramModule
{
    // Pilot Bidirectional Multi-Language Round-Trip Showcase
    // Exercises enums, ADTs, type extensions, classes, typed functions,
    // while/for loops, elif branching, precedence, comprehensions, f-strings, and assertions.
    public enum Priority
    {
        Low(1),
        Medium(2),
        High(3);

        public final long value;
        Priority(long value) { this.value = value; }
    }

    public sealed interface Shape permits Circle, Rect {}
    public record Circle(double r) implements Shape {}
    public record Rect(double w, double h) implements Shape {}

    public static final class i64PilotExtensions
    {
        public static long doubled(long self_val)
        {
            return self_val * 2;
        }
    }

    public static class Counter
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
            final long rem = x % y;
            x = y;
            y = rem;
        }
        return x;
    }

    public static long compute_weighted_sum(long limit, long scale)
    {
        final long base_bonus = (limit + 2) * scale;
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

    public static final long BASE_SCALE = 3;

    public static void main(String[] args)
    {
        final long g = gcd(48, 18);
        final long weighted = compute_weighted_sum(5, BASE_SCALE);
        final var squares = LongStream.rangeClosed(1, 6).filter(x -> x > 2).map(x -> x * x).boxed().collect(Collectors.toList());
        if (!(g == 6)) throw new AssertionError("Assertion failed");
        if (!(weighted == 53)) throw new AssertionError("Assertion failed");
        System.out.println(String.format("GCD=%s, Weighted=%s", g, weighted));
    }
}
