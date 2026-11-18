#!/usr/bin/env bash
# The one-off comparison of the Pilot parser with the seed's (PROP-025 §4
# step 2): every expression the seed's parser reads in each `.pilot` file of
# the tree is parsed again by stdlib/pilot/parse/ and the trees compared
# (selfhost/check/parse_compare.pilot). Run from the repository root:
#   tools/parse_compare/run.sh <out dir> [jobs] [seconds per check]
# A check reads a file's whole package, and its report covers every file of
# it, so files are checked in rounds: one file per directory first, then
# every file no report covered. Rerunning into the same <out dir> resumes. Writes the totals, with every
# DIFF line, to <out dir>/summary.txt. A DIFF is a bug in one of the
# parsers. Run it under tools/capped.sh.
set -u
out="${1:?out dir}"
jobs="${2:-4}"
limit="${3:-300}"
root="$(pwd)"
mkdir -p "$out/files"
find stdlib selfhost tests examples tools -name '*.pilot' -not -path '*/target/*' | sort > "$out/list.txt"
one() {
  f="$1"
  key="$(printf '%s' "$f" | tr '/' '_')"
  rep="$OUT/files/$key.report"
  PILOT_PARSE_COMPARE="$rep" timeout "$LIMIT" "$ROOT/target/debug/pilot" check "$f" >/dev/null 2>&1
  code=$?
  if [ ! -f "$rep" ]; then
    if [ "$code" = 124 ]; then echo "$f TIMEOUT 0 $f" > "$rep"; else echo "$f NOREPORT 0 $f" > "$rep"; fi
  fi
}
export -f one
export OUT="$out" ROOT="$root" LIMIT="$limit"
covered() { cat "$out"/files/*.report 2>/dev/null | awk '$2=="COVERS"||$2=="NOREPORT"||$2=="TIMEOUT"{print $4}' | sort -u; }
round=0
while :; do
  round=$((round+1))
  covered > "$out/covered.txt"
  comm -23 "$out/list.txt" "$out/covered.txt" > "$out/todo.txt"
  [ -s "$out/todo.txt" ] || break
  # The first round checks one file per directory: a library package's check
  # covers its other files. Later rounds check every file left (a test
  # program covers only itself).
  if [ "$round" = 1 ]; then
    awk -F/ '{d=$0; sub(/\/[^\/]*$/,"",d); if(!(d in seen)){seen[d]=1; print}}' "$out/todo.txt" > "$out/round.txt"
  else
    cp "$out/todo.txt" "$out/round.txt"
  fi
  # A round that covers nothing new would repeat forever.
  [ "$round" -gt 1 ] && cmp -s "$out/round.txt" "$out/last_round.txt" && break
  cp "$out/round.txt" "$out/last_round.txt"
  echo "round $round: $(wc -l < "$out/todo.txt") files left, checking $(wc -l < "$out/round.txt")"
  xargs -a "$out/round.txt" -P "$jobs" -I{} bash -c 'one "$@"' _ {}
done
cat "$out"/files/*.report | awk '!seen[$0]++' > "$out/all.txt"
{
  echo "files: $(wc -l < "$out/list.txt")"
  echo "checks run: $(ls "$out/files" | wc -l)"
  echo "files covered by a report: $(grep -c ' COVERS ' "$out/all.txt" | head -1)"
  echo "files whose check did not link (no report): $(grep -c ' NOREPORT ' "$out/all.txt")"
  echo "files whose check ran past ${limit}s: $(grep -c ' TIMEOUT ' "$out/all.txt")"
  echo "SAME: $(grep -c ' SAME ' "$out/all.txt")"
  echo "UNBUILT: $(grep -c ' UNBUILT ' "$out/all.txt")"
  echo "DIFF: $(grep -c ' DIFF ' "$out/all.txt")"
  echo "files the declared lexer rejects: $(grep -c ' NOLEX ' "$out/all.txt")"
  echo "REWRITTEN by a seed desugaring (no source of its own): $(grep -c ' REWRITTEN ' "$out/all.txt")"
  echo "--- unbuilt forms, most common first"
  grep ' UNBUILT ' "$out/all.txt" | sed 's/^[^ ]* UNBUILT [0-9]* //' | sort | uniq -c | sort -rn
  echo "--- every DIFF"
  grep ' DIFF ' "$out/all.txt"
  echo "--- files with no report, and files that do not lex"
  grep ' NOREPORT \| TIMEOUT \| NOLEX ' "$out/all.txt"
} > "$out/summary.txt"
