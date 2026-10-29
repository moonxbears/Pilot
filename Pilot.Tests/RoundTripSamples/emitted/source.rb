# frozen_string_literal: true
# Code transpiled & optimized from Pilot to Ruby 3.2+

# Pilot Bidirectional Multi-Language Round-Trip Showcase
# Exercises enums, ADTs, type extensions, classes, typed functions,
# while/for loops, elif branching, precedence, comprehensions, f-strings, and assertions.
module Priority
    Low = 1
    Medium = 2
    High = 3
end

Circle = Struct.new(:r)
Rect = Struct.new(:w, :h)
Shape = [Circle, Rect].freeze

class Integer
    def doubled
        return self * 2
    end
end

class Counter
    def step(x, delta)
        return x + delta
    end
end

def gcd(a, b)
    x = a
    y = b
    while y != 0
        rem = x % y
        x = y
        y = rem
    end
    return x
end

def compute_weighted_sum(limit, scale)
    base_bonus = (limit + 2) * scale
    total = base_bonus
    for i in 1..limit
        if i % 2 == 0
            total += i * scale
        elsif i == 5
            total += 10
        else
            total += i
        end
    end
    return total
end

BASE_SCALE = 3
g = gcd(48, 18)
weighted = compute_weighted_sum(5, BASE_SCALE)
squares = (1..6).select { |x| x > 2 }.map { |x| x * x }
raise "Assertion failed" unless g == 6
raise "Assertion failed" unless weighted == 53
puts("GCD=#{g}, Weighted=#{weighted}")
