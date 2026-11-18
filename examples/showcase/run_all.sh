#!/usr/bin/env bash
# Checks every showcase with `pilot check`. A showcase that uses a feature
# not built yet stops at a registered seam (SEAM001); that is reported, not
# counted as a failure. Any other error fails the run.
set -uo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
PILOT_BIN="${PILOT_BIN:-${DIR}/../../target/debug/pilot}"
if [ ! -x "$PILOT_BIN" ]; then PILOT_BIN="pilot"; fi

passed=0; seams=0; failed=0
for script in "$DIR"/*.pilot; do
    name=$(basename "$script")
    if out=$("$PILOT_BIN" check "$script" 2>&1); then
        echo "ok      $name"; passed=$((passed + 1))
    elif echo "$out" | grep -q "SEAM001" && ! echo "$out" | grep -E "error\[" | grep -vq "SEAM001"; then
        echo "seam    $name ($(echo "$out" | grep -m1 -o 'seam `[^`]*`'))"; seams=$((seams + 1))
    else
        echo "FAIL    $name"; echo "$out" | grep -m3 "error" | sed 's/^/        /'; failed=$((failed + 1))
    fi
done
echo "$passed passed, $seams stopped at a seam, $failed failed"
[ "$failed" -eq 0 ]
