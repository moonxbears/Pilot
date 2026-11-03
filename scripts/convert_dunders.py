#!/usr/bin/env python3
"""
convert_dunders.py — Pilot Dunder-to-First-Class Syntax Migrator

Converts Python-style dunder methods and explicit `self` receivers in Pilot
source files (.pilot) into first-class `self` and `op` grammar as specified
in AGENTS.md.

Uses pre-compiled regular expressions to match, extract, and rewrite method
signatures efficiently without naive string search-and-replace.

Usage:
    python scripts/convert_dunders.py --file path/to/file.pilot
    python scripts/convert_dunders.py --dir examples/ --dry-run --diff
    python scripts/convert_dunders.py --check
"""

import argparse
import difflib
import os
import re
import sys
from typing import Dict, List, Optional, Tuple

# ---------------------------------------------------------------------------
# Pre-compiled Regular Expressions
# ---------------------------------------------------------------------------

# Modifier prefix (visibility + clearance rings):
# Supports both short forms (pub, priv, prot, int, file, pkg)
# and long forms (public, private, protected, internal, fileprivate, package),
# combined forms (priv prot, private protected, prot int, protected internal),
# as well as open, export, lvl(N), lvl(Domain, N), lvlN.
MODIFIER_PATTERN = (
    r"(?P<indent>^[ \t]*)"
    r"(?P<modifiers>(?:(?:"
    r"pub(?:lic)?|"
    r"priv(?:ate)?(?:\s+prot(?:ected)?)?|"
    r"prot(?:ected)?(?:\s+int(?:ernal)?)?|"
    r"file(?:private)?|"
    r"pkg|package|"
    r"int(?:ernal)?|"
    r"open|"
    r"export|"
    r"lvl\([^\)]+\)|"
    r"lvl\d+"
    r")\s+)*)"
)

# Helper to strip leading `self` parameter from parameter strings
RE_SELF_ON_OWN_LINE = re.compile(r"^[ \t]*self(?::\s*[^,\)]+)?,?[ \t]*\r?\n", re.MULTILINE)
RE_SELF_INLINE = re.compile(r"^\s*self(?::\s*[^,\)]+)?\s*(?:,\s*|\s*$)", re.DOTALL)


def strip_self_parameter(params: str) -> str:
    """Strips leading `self` or `self: Type` from parameter list string."""
    if not params:
        return ""
    # Check if self is on its own line in a multiline signature
    cleaned, n = RE_SELF_ON_OWN_LINE.subn("", params, count=1)
    if n > 0:
        return cleaned
    # Check inline self
    return RE_SELF_INLINE.sub("", params, count=1)


# Constructor: def __init__(params) -> void
RE_CONSTRUCTOR = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__(?:init|new)__\s*\("
    + r"(?P<params>.*?)\)"
    + r"(?:\s*->\s*(?:void|None|Self))?"
    + r"(?P<rest>[ \t]*(?:\{|:|\n|$))",
    re.MULTILINE | re.DOTALL,
)

# Destructor: def __del__(self)
RE_DESTRUCTOR = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__del__\s*\(\s*(?:self\s*(?::\s*[^,\)]+)?)?\s*\)"
    + r"(?P<rest>[ \t]*(?:\{|:|\n|$))",
    re.MULTILINE,
)

# Binary operators mapping: __name__ -> op symbol
BIN_OPS: Dict[str, str] = {
    "add": "+",
    "sub": "-",
    "mul": "*",
    "truediv": "/",
    "div": "/",
    "floordiv": "//",
    "mod": "%",
    "pow": "**",
    "lshift": "<<",
    "rshift": ">>",
    "and": "&",
    "or": "|",
    "xor": "^",
    "matmul": "@",
}

# Reverse binary operators: __rname__ -> op symbol
REV_BIN_OPS: Dict[str, str] = {
    f"r{k}": v for k, v in BIN_OPS.items()
}

# In-place compound operators: __iname__ -> op= symbol
I_OPS: Dict[str, str] = {
    f"i{k}": f"{v}=" for k, v in BIN_OPS.items()
}

# Comparison operators: __cmp__ -> symbol
CMP_OPS: Dict[str, str] = {
    "eq": "==",
    "ne": "!=",
    "lt": "<",
    "le": "<=",
    "gt": ">",
    "ge": ">=",
}

# Unary operators: __unop__ -> prefix symbol
UNARY_OPS: Dict[str, str] = {
    "neg": "-",
    "pos": "+",
    "invert": "~",
}

# Cast / conversion protocols: __name__ -> target type
CAST_PROTOCOLS: Dict[str, str] = {
    "bool": "bool",
    "int": "int",
    "float": "float",
    "str": "str",
    "repr": "str",
}

# Pattern for binary left operator: def __add__(self, rhs: R) -> Out
BIN_OP_NAMES = "|".join(BIN_OPS.keys())
RE_BIN_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__(?P<op>" + BIN_OP_NAMES + r")__\s*\("
    + r"(?P<params>.*?)\)"
    + r"(?P<ret>\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE | re.DOTALL,
)

# Pattern for reverse binary operator: def __radd__(self, lhs: L) -> Out
REV_OP_NAMES = "|".join(REV_BIN_OPS.keys())
RE_REV_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__(?P<op>" + REV_OP_NAMES + r")__\s*\("
    + r"(?P<params>.*?)\)"
    + r"(?P<ret>\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE | re.DOTALL,
)

# Pattern for in-place compound operator: def __iadd__(self, rhs: R)
I_OP_NAMES = "|".join(I_OPS.keys())
RE_I_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:mut\s+)?(?:def|fn)\s+__(?P<op>" + I_OP_NAMES + r")__\s*\("
    + r"(?P<params>.*?)\)"
    + r"(?P<ret>\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE | re.DOTALL,
)

# Pattern for comparison operator: def __eq__(self, rhs: R) -> bool
CMP_OP_NAMES = "|".join(CMP_OPS.keys())
RE_CMP_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__(?P<op>" + CMP_OP_NAMES + r")__\s*\("
    + r"(?P<params>.*?)\)"
    + r"(?P<ret>\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE | re.DOTALL,
)

# Pattern for unary prefix operator: def __neg__(self) -> Out
UNARY_OP_NAMES = "|".join(UNARY_OPS.keys())
RE_UNARY_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__(?P<op>" + UNARY_OP_NAMES + r")__\s*\("
    + r"(?P<params>.*?)\)"
    + r"(?P<ret>\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE | re.DOTALL,
)

# Pattern for cast protocols: def __int__(self) -> int
CAST_NAMES = "|".join(CAST_PROTOCOLS.keys())
RE_CAST_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__(?P<op>" + CAST_NAMES + r")__\s*\("
    + r"(?P<params>.*?)\)"
    + r"(?:\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE | re.DOTALL,
)

# Other protocol dunders
RE_CALL_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__call__\s*\("
    + r"(?P<params>.*?)\)"
    + r"(?P<ret>\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE | re.DOTALL,
)

RE_CONTAINS_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__contains__\s*\("
    + r"(?P<params>.*?)\)"
    + r"(?P<ret>\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE | re.DOTALL,
)

RE_HASH_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__hash__\s*\(\s*(?:self\s*(?::\s*[^,\)]+)?)?\s*\)"
    + r"(?P<ret>\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE,
)

RE_CLONE_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__(?:copy|deepcopy)__\s*\(\s*(?:self\s*(?::\s*[^,\)]+)?)?\s*(?:,\s*memo)?"
    + r"\s*\)"
    + r"(?P<ret>\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE,
)

RE_ITER_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__iter__\s*\(\s*(?:self\s*(?::\s*[^,\)]+)?)?\s*\)"
    + r"(?P<ret>\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE,
)

RE_AITER_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__aiter__\s*\(\s*(?:self\s*(?::\s*[^,\)]+)?)?\s*\)"
    + r"(?P<ret>\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE,
)

RE_AWAIT_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__await__\s*\(\s*(?:self\s*(?::\s*[^,\)]+)?)?\s*\)"
    + r"(?P<ret>\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE,
)

# Indexer: def __getitem__(self, key: K) -> V
RE_GETITEM_OP = re.compile(
    MODIFIER_PATTERN
    + r"(?:def|fn)\s+__getitem__\s*\("
    + r"(?P<params>.*?)\)"
    + r"(?P<ret>\s*->\s*[^\{:\n]+?)?"
    + r"(?P<rest>\s*(?:\{|:|\n|$))",
    re.MULTILINE | re.DOTALL,
)

# Strip explicit `self` from regular instance methods inside a class
# e.g., `def foo(self, a: int)` -> `def foo(a: int)`
# or `def foo(self)` -> `def foo()`
RE_METHOD_SELF = re.compile(
    r"(^[ \t]+(?:(?:"
    r"pub(?:lic)?|"
    r"priv(?:ate)?(?:\s+prot(?:ected)?)?|"
    r"prot(?:ected)?(?:\s+int(?:ernal)?)?|"
    r"file(?:private)?|"
    r"pkg|package|"
    r"int(?:ernal)?|"
    r"open|"
    r"export|"
    r"lvl\([^\)]+\)|"
    r"lvl\d+"
    r")\s+)*(?:def|mut def)\s+[a-zA-Z_]\w*\s*\()"
    r"\s*self\s*(?::\s*[^,\)]+)?\s*(?:,\s*|\s*(?=\)))",
    re.MULTILINE,
)


class MigrationStats:
    def __init__(self) -> None:
        self.files_scanned = 0
        self.files_modified = 0
        self.conversions: Dict[str, int] = {}

    def record(self, category: str, count: int = 1) -> None:
        self.conversions[category] = self.conversions.get(category, 0) + count


def normalize_tail(ret: Optional[str], rest: str, default_ret: str = "") -> str:
    r = ret.strip() if ret else default_ret.strip()
    suffix = rest
    if r:
        if not r.startswith(" "):
            r = " " + r
        if suffix.startswith(("{", ":")) and not r.endswith(" "):
            r = r + " "
        return f"{r}{suffix}"
    else:
        if suffix.startswith(("{", ":")):
            return f" {suffix}"
        return suffix


def convert_source(content: str, stats: Optional[MigrationStats] = None, strip_method_self: bool = False) -> str:
    """Applies all regex transformations in order."""

    # 1. Constructors: __init__, __new__ -> self(...)
    def repl_constructor(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        raw_params = m.group("params")
        params = strip_self_parameter(raw_params)
        rest = normalize_tail(None, m.group("rest"))
        if stats:
            stats.record("constructor")
        return f"{indent}{mods}self({params}){rest}"

    content = RE_CONSTRUCTOR.sub(repl_constructor, content)

    # 2. Destructors: __del__ -> ~self
    def repl_destructor(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        rest = normalize_tail(None, m.group("rest"))
        if stats:
            stats.record("destructor")
        return f"{indent}{mods}~self{rest}"

    content = RE_DESTRUCTOR.sub(repl_destructor, content)

    # 3. Binary Left: op self + (rhs)
    def repl_bin_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        op_name = m.group("op")
        sym = BIN_OPS[op_name]
        raw_params = m.group("params")
        rhs = strip_self_parameter(raw_params).strip()
        tail = normalize_tail(m.group("ret"), m.group("rest"))
        if stats:
            stats.record(f"bin_op:{op_name}")
        return f"{indent}{mods}op self {sym} ({rhs}){tail}"

    content = RE_BIN_OP.sub(repl_bin_op, content)

    # 4. Reverse Binary: op (lhs) + self
    def repl_rev_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        op_name = m.group("op")
        sym = REV_BIN_OPS[op_name]
        raw_params = m.group("params")
        lhs = strip_self_parameter(raw_params).strip()
        tail = normalize_tail(m.group("ret"), m.group("rest"))
        if stats:
            stats.record(f"rev_op:{op_name}")
        return f"{indent}{mods}op ({lhs}) {sym} self{tail}"

    content = RE_REV_OP.sub(repl_rev_op, content)

    # 5. Compound Mutating: op mut self += (rhs)
    def repl_i_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        op_name = m.group("op")
        sym = I_OPS[op_name]
        raw_params = m.group("params")
        rhs = strip_self_parameter(raw_params).strip()
        tail = normalize_tail(m.group("ret"), m.group("rest"))
        if stats:
            stats.record(f"compound_op:{op_name}")
        return f"{indent}{mods}op mut self {sym} ({rhs}){tail}"

    content = RE_I_OP.sub(repl_i_op, content)

    # 6. Comparison: op self == (rhs) -> bool
    def repl_cmp_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        op_name = m.group("op")
        sym = CMP_OPS[op_name]
        raw_params = m.group("params")
        rhs = strip_self_parameter(raw_params).strip()
        tail = normalize_tail(m.group("ret"), m.group("rest"), default_ret="-> bool")
        if stats:
            stats.record(f"cmp_op:{op_name}")
        return f"{indent}{mods}op self {sym} ({rhs}){tail}"

    content = RE_CMP_OP.sub(repl_cmp_op, content)

    # 7. Unary Prefix: op -self
    def repl_unary_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        op_name = m.group("op")
        sym = UNARY_OPS[op_name]
        tail = normalize_tail(m.group("ret"), m.group("rest"))
        if stats:
            stats.record(f"unary_op:{op_name}")
        return f"{indent}{mods}op {sym}self{tail}"

    content = RE_UNARY_OP.sub(repl_unary_op, content)

    # 8. Cast: op self as <type> / op self -> bool
    def repl_cast_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        op_name = m.group("op")
        target = CAST_PROTOCOLS[op_name]
        rest = normalize_tail(None, m.group("rest"))
        if stats:
            stats.record(f"cast:{op_name}")
        if target == "bool":
            return f"{indent}{mods}op self -> bool{rest}"
        else:
            return f"{indent}{mods}op self as {target}{rest}"

    content = RE_CAST_OP.sub(repl_cast_op, content)

    # 9. Functor Call: op self(...)
    def repl_call_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        raw_params = m.group("params")
        params = strip_self_parameter(raw_params).strip()
        tail = normalize_tail(m.group("ret"), m.group("rest"))
        if stats:
            stats.record("call")
        return f"{indent}{mods}op self({params}){tail}"

    content = RE_CALL_OP.sub(repl_call_op, content)

    # 10. Membership: op (item) in self -> bool
    def repl_contains_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        raw_params = m.group("params")
        item = strip_self_parameter(raw_params).strip()
        tail = normalize_tail(m.group("ret"), m.group("rest"), default_ret="-> bool")
        if stats:
            stats.record("contains")
        return f"{indent}{mods}op ({item}) in self{tail}"

    content = RE_CONTAINS_OP.sub(repl_contains_op, content)

    # 11. Hash: op hash self -> u64
    def repl_hash_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        tail = normalize_tail(m.group("ret"), m.group("rest"), default_ret="-> u64")
        if stats:
            stats.record("hash")
        return f"{indent}{mods}op hash self{tail}"

    content = RE_HASH_OP.sub(repl_hash_op, content)

    # 12. Clone: op clone self -> Self
    def repl_clone_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        tail = normalize_tail(m.group("ret"), m.group("rest"), default_ret="-> Self")
        if stats:
            stats.record("clone")
        return f"{indent}{mods}op clone self{tail}"

    content = RE_CLONE_OP.sub(repl_clone_op, content)

    # 13. Iteration: op for self -> T / op async for self -> T
    def repl_iter_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        tail = normalize_tail(m.group("ret"), m.group("rest"))
        if stats:
            stats.record("iter")
        return f"{indent}{mods}op for self{tail}"

    content = RE_ITER_OP.sub(repl_iter_op, content)

    def repl_aiter_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        tail = normalize_tail(m.group("ret"), m.group("rest"))
        if stats:
            stats.record("aiter")
        return f"{indent}{mods}op async for self{tail}"

    content = RE_AITER_OP.sub(repl_aiter_op, content)

    # 14. Await: op await self -> T
    def repl_await_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        tail = normalize_tail(m.group("ret"), m.group("rest"))
        if stats:
            stats.record("await")
        return f"{indent}{mods}op await self{tail}"

    content = RE_AWAIT_OP.sub(repl_await_op, content)

    # 15. Indexer: def __getitem__(self, key: K) -> V -> self[key: K] -> V
    def repl_getitem_op(m: re.Match) -> str:
        indent = m.group("indent")
        mods = m.group("modifiers")
        raw_params = m.group("params")
        params = strip_self_parameter(raw_params).strip()
        tail = normalize_tail(m.group("ret"), m.group("rest"))
        if stats:
            stats.record("indexer_get")
        return f"{indent}{mods}self[{params}]{tail}"

    content = RE_GETITEM_OP.sub(repl_getitem_op, content)

    # 15. Optional: Strip `self` from regular instance method declarations
    if strip_method_self:
        def repl_method_self(m: re.Match) -> str:
            prefix = m.group(1)
            if stats:
                stats.record("strip_method_self")
            return prefix

        content = RE_METHOD_SELF.sub(repl_method_self, content)

    return content


def process_file(
    file_path: str,
    dry_run: bool = False,
    show_diff: bool = False,
    check_only: bool = False,
    strip_method_self: bool = False,
    stats: Optional[MigrationStats] = None,
) -> bool:
    """Processes a single file. Returns True if file needs changes or was changed."""
    try:
        with open(file_path, "r", encoding="utf-8") as f:
            original = f.read()
    except Exception as e:
        print(f"Error reading {file_path}: {e}", file=sys.stderr)
        return False

    if stats:
        stats.files_scanned += 1

    converted = convert_source(original, stats, strip_method_self=strip_method_self)
    changed = original != converted

    if changed:
        if stats:
            stats.files_modified += 1

        if show_diff:
            diff = difflib.unified_diff(
                original.splitlines(keepends=True),
                converted.splitlines(keepends=True),
                fromfile=f"a/{file_path}",
                tofile=f"b/{file_path}",
            )
            sys.stdout.writelines(diff)

        if not dry_run and not check_only:
            with open(file_path, "w", encoding="utf-8") as f:
                f.write(converted)
            print(f"[CONVERTED] {file_path}")
        elif check_only:
            print(f"[DUNDER DETECTED] {file_path}")
        else:
            print(f"[WOULD CONVERT] {file_path}")

    return changed


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Convert Python dunder methods to Pilot first-class self/op grammar."
    )
    parser.add_argument("--file", type=str, help="Single .pilot file to process")
    parser.add_argument(
        "--dir",
        type=str,
        default=".",
        help="Root directory to search recursively for .pilot files (default: .)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview changes without modifying files on disk",
    )
    parser.add_argument(
        "--diff",
        action="store_true",
        help="Print unified diff of changes",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Check if any files contain old dunder syntax and exit non-zero if found",
    )
    parser.add_argument(
        "--strip-method-self",
        action="store_true",
        help="Also strip leading 'self' or 'self: Type' parameter from regular methods",
    )

    args = parser.parse_args()
    stats = MigrationStats()

    targets: List[str] = []
    if args.file:
        targets.append(args.file)
    else:
        # Ignore target, .git, and vscode artifacts
        ignore_dirs = {"target", ".git", ".vscode", "node_modules", "build"}
        for root, dirs, files in os.walk(args.dir):
            dirs[:] = [d for d in dirs if d not in ignore_dirs]
            for file in files:
                if file.endswith(".pilot"):
                    targets.append(os.path.join(root, file))

    any_changed = False
    for path in sorted(targets):
        changed = process_file(
            path,
            dry_run=args.dry_run,
            show_diff=args.diff,
            check_only=args.check,
            strip_method_self=args.strip_method_self,
            stats=stats,
        )
        if changed:
            any_changed = True

    print("\n--- Migration Summary ---")
    print(f"Files scanned:  {stats.files_scanned}")
    print(f"Files modified: {stats.files_modified}")
    if stats.conversions:
        print("Transformations by category:")
        for cat, cnt in sorted(stats.conversions.items()):
            print(f"  - {cat:20s}: {cnt}")

    if args.check and any_changed:
        print("\nError: Legacy dunder syntax detected. Run `python scripts/convert_dunders.py` to upgrade.")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
