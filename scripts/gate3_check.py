import sys
import re
import glob

CAPS = {
    "crates/pilot-cli/src/main.rs": 520,
    "crates/pilot-lsp/src/server.rs": 350,
    "crates/pilot-lsp/src/lib.rs": 120,
}

BANNED_TOKENS = [
    "bool|usize|i8",
    "BUILTIN_DOCS",
    "KEYWORD_DOCS",
    "STR_METHOD_DOCS",
    "LIST_METHOD_DOCS",
    "INT_METHOD_DOCS",
    "TypeInferenceEngine",
    "TypeArena",
    "TypeId",
    "#[ignore]",
    "*Pilot Language*",
    'Custom("any"',
]

def check_caps():
    ok = True
    for path, max_lines in CAPS.items():
        with open(path, "r", encoding="utf-8") as f:
            count = sum(1 for _ in f)
        if count > max_lines:
            print(f"FAIL: {path} has {count} lines (hard maximum is {max_lines})")
            ok = False
        else:
            print(f"PASS: {path} line count = {count} (<= {max_lines})")
    return ok

def check_banned():
    ok = True
    targets = (
        glob.glob("crates/pilot-cli/src/**/*.rs", recursive=True)
        + glob.glob("crates/pilot-lsp/src/**/*.rs", recursive=True)
        + glob.glob("crates/pilot-lsp/tests/**/*.rs", recursive=True)
    )
    for path in targets:
        with open(path, "r", encoding="utf-8") as f:
            content = f.read()
        for token in BANNED_TOKENS:
            if token in content:
                print(f"FAIL: Banned token '{token}' found in {path}")
                ok = False
    if ok:
        print(f"PASS: Zero banned tokens found in CLI & LSP ({len(targets)} files checked)")
    return ok

def main():
    passed = True
    if not check_caps():
        passed = False
    if not check_banned():
        passed = False
    if not passed:
        sys.exit(1)
    print("ALL PHASE 3B GATES PASSED!")
    sys.exit(0)

if __name__ == "__main__":
    main()
