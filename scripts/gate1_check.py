import os
import re
import glob

PATTERNS = ["Int", "Float", "String", "Bool", "Char", "List", "Dict", "Map", "Option", "Result", "SliceBuffer", "SliceMap", "Tensor", "Matrix"]
REGEX = re.compile(r'\"(' + '|'.join(PATTERNS) + r')\"')
REGEX_LAYOUT_ID = re.compile(r'LayoutId\([0-9]+\)')

def main():
    failed = False
    matches1 = []
    matches2 = []

    for filepath in glob.glob("crates/pilot-analysis/src/**/*.rs", recursive=True):
        with open(filepath, "r", encoding="utf-8") as f:
            for idx, line in enumerate(f, 1):
                m = REGEX.search(line)
                if m:
                    matches1.append((filepath, idx, m.group(1), line.strip()))
                m2 = REGEX_LAYOUT_ID.search(line)
                if m2:
                    matches2.append((filepath, idx, m2.group(0), line.strip()))

    print(f"Gate 3.1: Found {len(matches1)} matches for named types in crates/pilot-analysis/src/")
    for fp, idx, matched, text in matches1:
        print(f"  {fp}:{idx} [{matched}] -> {text}")

    print(f"Gate 3.2: Found {len(matches2)} matches for LayoutId([0-9]+) in crates/pilot-analysis/src/")
    for fp, idx, matched, text in matches2:
        print(f"  {fp}:{idx} [{matched}] -> {text}")

    if len(matches1) == 0 and len(matches2) == 0:
        print("ALL GATE 3 STATIC AUDITS PASSED WITH 0 MATCHES!")
        return 0
    else:
        return 1

if __name__ == "__main__":
    exit(main())
