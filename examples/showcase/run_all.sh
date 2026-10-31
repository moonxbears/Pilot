#!/usr/bin/env bash
# Runner script for all Pilot Concept Showcases on Unix/Linux/macOS
set -e

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

for script in $(ls "$DIR"/*.pilot | sort); do
    filename=$(basename "$script")
    echo ""
    echo ">>> CHECKING: $filename <<<"
    "$PILOT_BIN" check "$script"
    echo "----------------------------------------------------------"
done

echo ""
echo "All Concept Showcases checked successfully!"
