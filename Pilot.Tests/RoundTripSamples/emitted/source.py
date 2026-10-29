# Code transpiled & optimized from Pilot to Python 3
# Target: Python 3.10+ (typed)
from __future__ import annotations
from dataclasses import dataclass
from typing import Any, List, Dict, Set, Tuple, Optional, Callable, Final, Union
from enum import Enum

# Pilot Bidirectional Multi-Language Round-Trip Showcase
# Exercises enums, ADTs, type extensions, classes, typed functions,
# while/for loops, elif branching, precedence, comprehensions, f-strings, and assertions.
class Priority(Enum):
    Low = 1
    Medium = 2
    High = 3
@dataclass
class Circle:
    r: float

@dataclass
class Rect:
    w: float
    h: float

Shape = Union[Circle, Rect]

def __ext_i64_doubled(self_val: int) -> int:
    return self_val * 2

class Counter:
    def step(self, x: int, delta: int) -> int:
        return x + delta

def gcd(a: int, b: int) -> int:
    x: int = a
    y: int = b
    while y != 0:
        rem: int = x % y
        x = y
        y = rem
    return x

def compute_weighted_sum(limit: int, scale: int) -> int:
    base_bonus: int = (limit + 2) * scale
    total: int = base_bonus
    for i in range(1, limit + 1):
        if i % 2 == 0:
            total += i * scale
        elif i == 5:
            total += 10
        else:
            total += i
    return total

BASE_SCALE: Final[int] = 3
g: int = gcd(48, 18)
weighted: int = compute_weighted_sum(5, BASE_SCALE)
squares = [x * x for x in range(1, 7) if x > 2]
assert g == 6
assert weighted == 53
print(f"GCD={g}, Weighted={weighted}")
