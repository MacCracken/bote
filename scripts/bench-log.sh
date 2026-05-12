#!/usr/bin/env bash
# Run Cyrius benchmarks and append to benches/history.log.
#
# 2.7.2: ported from cargo-bench (Rust-era stale) to cyrius bench.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="$REPO_ROOT/benches"
LOG="$LOG_DIR/history.log"
VERSION=$(cat "$REPO_ROOT/VERSION")
COMMIT=$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo "unknown")
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

mkdir -p "$LOG_DIR"

echo "Running benchmarks..."

# Capture only the bench result lines (cyrius bench prints build
# chatter + DCE warnings before, and the "=== Benchmarks ===" marker
# is glued to the tail of the last warning line). Each result line
# looks like: "  name: 2us avg (min=… max=…) [N iters]".
OUTPUT=$(cd "$REPO_ROOT" && cyrius bench tests/bote.bcyr 2>&1 \
    | grep -E '^[[:space:]]+[a-zA-Z_][a-zA-Z0-9_]*: .* avg \(' )

{
    echo ""
    echo "## $TIMESTAMP  v$VERSION ($COMMIT)"
    echo "$OUTPUT"
} >> "$LOG"

echo "Logged to $LOG"
echo
echo "$OUTPUT"
