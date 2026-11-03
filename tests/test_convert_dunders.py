"""
Unit tests for scripts/convert_dunders.py
Verifies that all Python dunder conversions to Pilot first-class self/op grammar
work accurately across operators, modifiers, clearance tiers, and multiline signatures,
while preserving C-ABI runtime intrinsics.
"""

import sys
import unittest
from pathlib import Path

# Add repo root to import path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from scripts.convert_dunders import MigrationStats, convert_source


class TestConvertDunders(unittest.TestCase):
    def test_constructor_variations(self):
        # 1. Simple constructor
        src = "    def __init__(self, x: i32, y: i32) {\n        .x = x;\n    }"
        expected = "    self(x: i32, y: i32) {\n        .x = x;\n    }"
        self.assertEqual(convert_source(src), expected)

        # 2. No params
        src = "    def __init__(self) {\n        .x = 0;\n    }"
        expected = "    self() {\n        .x = 0;\n    }"
        self.assertEqual(convert_source(src), expected)

        # 3. No self in param list originally
        src = "    def __init__(x: i32) {\n        .x = x;\n    }"
        expected = "    self(x: i32) {\n        .x = x;\n    }"
        self.assertEqual(convert_source(src), expected)

        # 4. Modifiers and clearance ring
        src = "    pub lvl30 def __init__(self, key: str) -> void {"
        expected = "    pub lvl30 self(key: str) {"
        self.assertEqual(convert_source(src), expected)

        # 5. Multiline constructor with self on own line
        src = """    def __init__(
        self,
        delimiter: str = ",",
        quotechar: str = "\"",
    ) {"""
        expected = """    self(
        delimiter: str = ",",
        quotechar: str = "\"",
    ) {"""
        self.assertEqual(convert_source(src), expected)

    def test_destructor(self):
        src = "    def __del__(self) {\n        .cleanup();\n    }"
        expected = "    ~self {\n        .cleanup();\n    }"
        self.assertEqual(convert_source(src), expected)

        src = "    pub ~self {"
        self.assertEqual(convert_source(src), src)

    def test_binary_operators(self):
        src = "    def __add__(self, rhs: Vec2) -> Vec2 {\n        return Vec2(.x + rhs.x, .y + rhs.y);\n    }"
        expected = "    op self + (rhs: Vec2) -> Vec2 {\n        return Vec2(.x + rhs.x, .y + rhs.y);\n    }"
        self.assertEqual(convert_source(src), expected)

        src = "    def __mul__(self, scale: f64) -> Vec2 {\n        return Vec2(.x * scale, .y * scale);\n    }"
        expected = "    op self * (scale: f64) -> Vec2 {\n        return Vec2(.x * scale, .y * scale);\n    }"
        self.assertEqual(convert_source(src), expected)

    def test_reverse_binary_operators(self):
        src = "    def __radd__(self, lhs: i32) -> i32 {\n        return lhs + .val;\n    }"
        expected = "    op (lhs: i32) + self -> i32 {\n        return lhs + .val;\n    }"
        self.assertEqual(convert_source(src), expected)

        src = "    def __rmul__(self, lhs: f64) -> Vec2 {\n        return .mul(lhs);\n    }"
        expected = "    op (lhs: f64) * self -> Vec2 {\n        return .mul(lhs);\n    }"
        self.assertEqual(convert_source(src), expected)

    def test_compound_mutating_operators(self):
        src = "    mut def __iadd__(self, rhs: Vec2) {\n        .x += rhs.x;\n        .y += rhs.y;\n    }"
        expected = "    op mut self += (rhs: Vec2) {\n        .x += rhs.x;\n        .y += rhs.y;\n    }"
        self.assertEqual(convert_source(src), expected)

    def test_comparison_operators(self):
        src = "    def __eq__(self, rhs: Point) -> bool {\n        return .x == rhs.x and .y == rhs.y;\n    }"
        expected = "    op self == (rhs: Point) -> bool {\n        return .x == rhs.x and .y == rhs.y;\n    }"
        self.assertEqual(convert_source(src), expected)

        src = "    def __lt__(self, rhs: Point) -> bool {\n        return .x < rhs.x;\n    }"
        expected = "    op self < (rhs: Point) -> bool {\n        return .x < rhs.x;\n    }"
        self.assertEqual(convert_source(src), expected)

    def test_unary_operators(self):
        src = "    def __neg__(self) -> Vec2 {\n        return Vec2(-.x, -.y);\n    }"
        expected = "    op -self -> Vec2 {\n        return Vec2(-.x, -.y);\n    }"
        self.assertEqual(convert_source(src), expected)

        src = "    def __invert__(self) -> Mask {\n        return Mask(~.bits);\n    }"
        expected = "    op ~self -> Mask {\n        return Mask(~.bits);\n    }"
        self.assertEqual(convert_source(src), expected)

    def test_protocols(self):
        # __call__
        src = "    def __call__(self, arg: i32) -> i32 {\n        return arg * 2;\n    }"
        expected = "    op self(arg: i32) -> i32 {\n        return arg * 2;\n    }"
        self.assertEqual(convert_source(src), expected)

        # __contains__
        src = "    def __contains__(self, item: str) -> bool {\n        return .has(item);\n    }"
        expected = "    op (item: str) in self -> bool {\n        return .has(item);\n    }"
        self.assertEqual(convert_source(src), expected)

        # __bool__
        src = "    def __bool__(self) -> bool {\n        return .count > 0;\n    }"
        expected = "    op self -> bool {\n        return .count > 0;\n    }"
        self.assertEqual(convert_source(src), expected)

        # __str__
        src = "    def __str__(self) -> str {\n        return .name;\n    }"
        expected = "    op self as str {\n        return .name;\n    }"
        self.assertEqual(convert_source(src), expected)

        # __hash__
        src = "    def __hash__(self) -> u64 {\n        return .id;\n    }"
        expected = "    op hash self -> u64 {\n        return .id;\n    }"
        self.assertEqual(convert_source(src), expected)

        # __iter__
        src = "    def __iter__(self) -> Item {\n        for x in .items { yield x; }\n    }"
        expected = "    op for self -> Item {\n        for x in .items { yield x; }\n    }"
        self.assertEqual(convert_source(src), expected)

        # __await__
        src = "    def __await__(self) -> Response {\n        return yield_async(self);\n    }"
        expected = "    op await self -> Response {\n        return yield_async(self);\n    }"
        self.assertEqual(convert_source(src), expected)

    def test_indexer(self):
        src = "    def __getitem__(self, key: str) -> i32 {\n        return .map.get(key);\n    }"
        expected = "    self[key: str] -> i32 {\n        return .map.get(key);\n    }"
        self.assertEqual(convert_source(src), expected)

    def test_clearance_domain_and_vis(self):
        src = "    prot int lvl(Crypto, 50) def __init__(self, seed: [u8]) -> void {"
        expected = "    prot int lvl(Crypto, 50) self(seed: [u8]) {"
        self.assertEqual(convert_source(src), expected)

    def test_preserve_runtime_intrinsics(self):
        # Top-level intrinsic functions starting with __ must NOT be changed
        src = """def __slice_push(buf: [u8], item: u8) -> void {
    // intrinsic implementation
}
def __to_utf8_bytes(s: str) -> [u8] {
    return [];
}
if __name__ == "__main__":
    run();
"""
        self.assertEqual(convert_source(src), src)

    def test_strip_method_self(self):
        src = """class MathNode {
    pub def eval(self, env: Env) -> f64 {
        return 0.0;
    }
    def reset(self) {
        .val = 0;
    }
    mut def scale(self, factor: f64) {
        .val *= factor;
    }
}"""
        expected = """class MathNode {
    pub def eval(env: Env) -> f64 {
        return 0.0;
    }
    def reset() {
        .val = 0;
    }
    mut def scale(factor: f64) {
        .val *= factor;
    }
}"""
        self.assertEqual(convert_source(src, strip_method_self=True), expected)


if __name__ == "__main__":
    unittest.main()
