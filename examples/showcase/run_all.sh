#!/usr/bin/env bash
# Runner script for all Pilot Concept Showcases on Unix/Linux/macOS
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
echo "=========================================================="
echo "         PILOT INTERACTIVE CONCEPT SHOWCASE RUNNER        "
echo "=========================================================="

PILOT_BIN="${DIR}/../../target/release/pilot"
if [ ! -f "$PILOT_BIN" ]; then
    PILOT_BIN="${DIR}/../../target/debug/pilot"
fi
if [ ! -f "$PILOT_BIN" ] && command -v pilot &> /dev/null; then
    PILOT_BIN="pilot"
fi

count=0
for script in $(ls "$DIR"/*.pilot | sort); do
    filename=$(basename "$script")
    echo ""
    echo ">>> CHECKING: $filename <<<"
    out=$("$PILOT_BIN" check "$script")
    echo "$out"
    if ! echo "$out" | grep -q ": 0 diagnostics"; then
        echo "ERROR: $filename reported non-zero diagnostics!" >&2
        exit 1
    fi
    count=$((count + 1))
    echo "----------------------------------------------------------"
done

echo ""
echo "All $count Concept Showcases verified with 0 diagnostics!"
