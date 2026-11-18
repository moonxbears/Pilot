#!/bin/bash
# tools/capped.sh SECONDS cmd...: run cmd with a time limit, at most 512 tasks
# and 12 GB of memory (no swap), so a runaway test cannot take the machine
# down (an uncapped run once did). Every heavy job goes through it
# (AGENTS.md section 12.1). CAP_MEM overrides the memory cap, e.g. CAP_MEM=8G.
# Needs systemd user sessions (Linux). Elsewhere it falls back to `timeout`
# alone, which caps time but not memory.
t=$1; shift
if command -v systemd-run >/dev/null 2>&1; then
    exec systemd-run --user --scope -q -p TasksMax=512 -p MemoryMax=${CAP_MEM:-12G} -p MemorySwapMax=0 -- timeout -k 10 "$t" "$@"
fi
exec timeout -k 10 "$t" "$@"
