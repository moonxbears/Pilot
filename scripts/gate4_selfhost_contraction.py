import os
import sys
import glob

RETIRED_CRATES = [
    "crates/pilot-syntax",
    "crates/pilot-fmt",
    "crates/pilot-perceus",
    "crates/pilot-transpile",
    "crates/pilot-polyglot",
    "crates/pilot-dap",
    "crates/pilot-lsp",
    "crates/pilot-lsp-macros",
    "crates/pilot-db",
    "crates/pilot-incremental",
    "crates/pilot-pkg",
    "crates/pilot-perf",
    "crates/pilot-bench",
]

DELETED_LEGACY_FILES = [
    "selfhost/scope_header.pilot",
    "selfhost/emitter.pilot",
    "selfhost/ide_sync.pilot",
    "crates/pilot-vm/src/compiler.rs",
    "crates/pilot-codegen/src/c_target.rs",
    "crates/pilot-codegen/src/gpu.rs",
    "crates/pilot-codegen/src/llvm.rs",
    "crates/pilot-codegen/src/wasm.rs",
    "crates/pilot-codegen/src/asm.rs",
    "crates/pilot-codegen/src/driver.rs",
    "crates/pilot-codegen/src/physics.rs",
    "crates/pilot-rt/src/crypto.rs",
    "crates/pilot-cli/src/commands.rs",
]

CRATE_LINE_CAPS = {
    "crates/pilot-lexer": 860,
    "crates/pilot-ast": 450,
    "crates/pilot-parser": 1100,
    "crates/pilot-analysis": 780,
    "crates/pilot-vm": 650,
    "crates/pilot-codegen": 120,
    "crates/pilot-rt": 700,
    "crates/pilot-ffi": 300,
    "crates/pilot-cli": 100,
}

MAX_TOTAL_RUST_SRC_LINES = 5900
MAX_PILOT_FILE_LINES = 1500


def count_lines(path):
    with open(path, "r", encoding="utf-8") as f:
        return sum(1 for _ in f)


def check_retired_and_deleted():
    ok = True
    for crate_dir in RETIRED_CRATES:
        if os.path.exists(crate_dir):
            print(f"FAIL: Retired crate still exists on disk: {crate_dir}")
            ok = False
    for fpath in DELETED_LEGACY_FILES:
        if os.path.exists(fpath):
            print(f"FAIL: Deleted legacy file still exists on disk: {fpath}")
            ok = False
    if ok:
        print(f"PASS: All {len(RETIRED_CRATES)} retired crates and {len(DELETED_LEGACY_FILES)} legacy files purged")
    return ok


def check_rust_line_caps_and_dead_code():
    ok = True
    total_rust_lines = 0
    for rs_file in sorted(glob.glob("crates/*/src/**/*.rs", recursive=True)):
        lines = count_lines(rs_file)
        total_rust_lines += lines
        with open(rs_file, "r", encoding="utf-8") as f:
            content = f.read()
        if "allow(dead_code)" in content:
            print(f"FAIL: #[allow(dead_code)] found in {rs_file}")
            ok = False

    for crate_dir, max_lines in CRATE_LINE_CAPS.items():
        crate_lines = sum(
            count_lines(p) for p in glob.glob(f"{crate_dir}/src/**/*.rs", recursive=True)
        )
        if crate_lines > max_lines:
            print(f"FAIL: {crate_dir} has {crate_lines} lines (cap is {max_lines})")
            ok = False
        else:
            print(f"PASS: {crate_dir} src lines = {crate_lines} (<= {max_lines})")

    if total_rust_lines > MAX_TOTAL_RUST_SRC_LINES:
        print(f"FAIL: Total workspace Rust src/*.rs lines = {total_rust_lines} (> {MAX_TOTAL_RUST_SRC_LINES})")
        ok = False
    else:
        print(f"PASS: Total workspace Rust src/*.rs lines = {total_rust_lines} (<= {MAX_TOTAL_RUST_SRC_LINES})")
    return ok


def check_pilot_file_caps():
    ok = True
    pilot_files = sorted(
        glob.glob("selfhost/**/*.pilot", recursive=True)
        + glob.glob("stdlib/core/**/*.pilot", recursive=True)
    )
    max_seen = 0
    for pf in pilot_files:
        lines = count_lines(pf)
        if lines > max_seen:
            max_seen = lines
        if lines > MAX_PILOT_FILE_LINES:
            print(f"FAIL: {pf} has {lines} lines (hard cap is {MAX_PILOT_FILE_LINES})")
            ok = False
    if ok:
        print(f"PASS: All {len(pilot_files)} .pilot files in selfhost/ & stdlib/core/ <= {MAX_PILOT_FILE_LINES} lines (max = {max_seen})")
    return ok


def main():
    passed = True
    if not check_retired_and_deleted():
        passed = False
    if not check_rust_line_caps_and_dead_code():
        passed = False
    if not check_pilot_file_caps():
        passed = False
    if not passed:
        sys.exit(1)
    print("ALL PHASE 4 SELF-HOST CONTRACTION GATES PASSED!")
    sys.exit(0)


if __name__ == "__main__":
    main()
