#!/usr/bin/env bash
# bundle-min-cyrius.sh — which installed cyrius toolchains compile and run
# this checkout, so the "Requires cyrius" line at the top of src/log.cyr (and
# so of both dist bundles) is a measurement, not a guess.
#
# Usage: ./scripts/bundle-min-cyrius.sh [VERSION ...]
#   With no arguments, every toolchain under ~/.cyrius/versions/ from 6.6.0 on.
#
# For each version V it copies the working tree (no lib/, build/, .git) into a
# scratch directory, pins cyrius.cyml to V, provisions it from scratch the way
# CLAUDE.md's toolchain A/B recipe does (rm lib + lock, `cyrius lib sync
# --full`, `cyrius deps`), and then requires three things:
#   build  `cyrius build src/main.cyr` exits 0 and reports no undefined
#          function. cycc lowers an unreachable undefined fn to a ud2 stub and
#          still exits 0, so the exit code alone is not the bar.
#   core   tests/bote_core_only_smoke.tcyr passes. It includes only
#          dist/bote-core.cyr.
#   ping   the built binary answers a JSON-RPC ping over stdio.
# The first line of each failure is printed, which is usually the construct
# the old compiler stops at.
#
# The scratch copies go under $BOTE_MC_DIR (default: a fresh mktemp dir), which
# is printed and left in place for inspection. Needs network for the git deps
# on first use (cyrius caches them).
set -u

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="${BOTE_MC_DIR:-$(mktemp -d)}"
mkdir -p "$ROOT"

if [ "$#" -gt 0 ]; then
    versions="$*"
else
    versions=$(ls "$HOME/.cyrius/versions" 2>/dev/null | grep -E '^6\.(6|7|8|9)\.[0-9]+$' | sort -V)
fi

echo "scratch: $ROOT"
printf '%-8s %-6s %-6s %-6s %s\n' "cyrius" "build" "core" "ping" "first failure"
for v in $versions; do
    d="$ROOT/$v"
    rm -rf "$d"
    mkdir -p "$d"
    (cd "$REPO" && tar --exclude=./lib --exclude=./build --exclude=./.git -cf - .) | (cd "$d" && tar -xf -)
    sed -i -E "s/^cyrius = \"[0-9.]+\"/cyrius = \"$v\"/" "$d/cyrius.cyml"
    mkdir -p "$d/build"
    log="$d/mc.log"
    b="FAIL"; c="-"; p="-"; why=""
    (cd "$d" && rm -rf lib cyrius.lock && cyrius lib sync --full && cyrius deps) > "$log" 2>&1
    out=$(cd "$d" && cyrius build src/main.cyr build/bote 2>&1)
    rc=$?
    echo "$out" >> "$log"
    if [ $rc -eq 0 ] && ! echo "$out" | grep -q "undefined function"; then
        b="ok"
        tout=$(cd "$d" && cyrius test tests/bote_core_only_smoke.tcyr 2>&1); trc=$?
        echo "$tout" >> "$log"
        if [ $trc -eq 0 ] && ! echo "$tout" | grep -q "undefined function"; then c="ok"; else c="FAIL"; why=$(echo "$tout" | grep -m1 -E "error|undefined|FAIL"); fi
        pout=$(printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"ping"}' | (cd "$d" && timeout 10 ./build/bote stdio 2>/dev/null))
        if echo "$pout" | grep -q '"result":{}'; then p="ok"; else p="FAIL"; [ -n "$why" ] || why="no ping reply: $pout"; fi
    else
        why=$(echo "$out" | grep -m1 -E "error|undefined")
        [ -n "$why" ] || why=$(grep -m1 -E "error|refus" "$log")
    fi
    printf '%-8s %-6s %-6s %-6s %s\n' "$v" "$b" "$c" "$p" "${why:0:110}"
done
