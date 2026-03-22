#!/usr/bin/env bash
# Run benchmarks and append results to benches/history.log
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="$REPO_ROOT/benches/history.log"
VERSION=$(cat "$REPO_ROOT/VERSION")
COMMIT=$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo "unknown")
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

echo "Running benchmarks..."

# Criterion outputs either "name  time: [...]" on one line or
# "name\n                        time: [...]" on two lines (for long names).
# This awk script joins them into "name  time: [...]" consistently.
OUTPUT=$(cargo bench --bench dispatch 2>&1 | awk '
    /time:/ {
        if (prev != "" && $0 ~ /^[[:space:]]+time:/) {
            sub(/^[[:space:]]+/, "", $0)
            print prev "  " $0
            prev = ""
        } else {
            print
        }
        next
    }
    /^[a-z_]/ { prev = $0; next }
    { prev = "" }
')

{
    echo ""
    echo "## $TIMESTAMP  v$VERSION ($COMMIT)"
    echo "$OUTPUT"
} >> "$LOG"

echo "Logged to $LOG"
cat "$LOG"
