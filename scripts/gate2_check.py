import os
import sys

DELETED_FILES = [
    "crates/pilot-analysis/src/autodiff.rs",
    "crates/pilot-analysis/src/contracts.rs",
    "crates/pilot-analysis/src/dimensions.rs",
    "crates/pilot-analysis/src/loops.rs",
    "crates/pilot-analysis/src/macros.rs",
    "crates/pilot-analysis/src/patterns.rs",
    "crates/pilot-analysis/src/quoting.rs",
    "crates/pilot-analysis/src/null_error.rs",
]

LINE_CAPS = {
    "crates/pilot-analysis/src/types.rs": 250,
    "crates/pilot-analysis/src/infer.rs": 440,
    "crates/pilot-analysis/src/comptime.rs": 220,
    "crates/pilot-analysis/src/pipeline.rs": 80,
    "crates/pilot-analysis/src/lib.rs": 60,
    "crates/pilot-rt/src/stdlib.rs": 90,
}

BANNED_STDLIB_TOKENS = ["pandas", "matplotlib", "scikit", "MODULE_MAP"]

def main():
    passed = True

    for path in DELETED_FILES:
        if os.path.exists(path):
            print(f"FAIL: Legacy Rust pass file still exists and must be deleted: {path}")
            passed = False
        else:
            print(f"PASS: Deleted {path}")

    for path, cap in LINE_CAPS.items():
        if not os.path.exists(path):
            print(f"FAIL: Expected file missing: {path}")
            passed = False
            continue
        with open(path, "r", encoding="utf-8") as f:
            lines = sum(1 for _ in f)
        if lines > cap:
            print(f"FAIL: {path} has {lines} lines (hard cap is {cap})")
            passed = False
        else:
            print(f"PASS: {path} line count = {lines} (<= {cap})")

    stdlib_path = "crates/pilot-rt/src/stdlib.rs"
    if os.path.exists(stdlib_path):
        with open(stdlib_path, "r", encoding="utf-8") as f:
            content = f.read()
        for token in BANNED_STDLIB_TOKENS:
            if token in content:
                print(f"FAIL: {stdlib_path} contains banned token '{token}'")
                passed = False
            else:
                print(f"PASS: {stdlib_path} free of '{token}'")

    if not passed:
        sys.exit(1)

    print("ALL PHASE 2B GATES PASSED!")
    sys.exit(0)

if __name__ == "__main__":
    main()
