#!/bin/bash
# The official MCP conformance suite and TypeScript client against bote, at pinned
# versions, for both revisions on both HTTP transports, and the client over stdio too.
#
# Not a CI step: it installs third-party code from npm. Run it by hand at a release that
# touches the protocol. It needs node and npm, and writes only under build/conformance.
#
#   ./scripts/conformance.sh
#
# tests/fixtures/conformance-expected-<revision>.yaml lists what bote is known to fail,
# and the suite exits non-zero on a failure not listed there and on a listed one that
# now passes, so a regression and a stale entry both stop this script.
set -u
SUITE=0.2.0-alpha.12
CLIENT=2.4.0
PORT=19530

cd "$(dirname "$0")/.." || exit 1
ROOT=$(pwd)
W="$ROOT/build/conformance"
mkdir -p "$W"

cyrius build tests/fixtures/conformance_server.cyr build/conformance-server >/dev/null 2>&1 \
  || { echo "conformance: the fixture does not build" >&2; exit 1; }

if [ ! -d "$W/node_modules/@modelcontextprotocol/conformance" ]; then
  ( cd "$W" && npm init -y >/dev/null \
    && npm install --ignore-scripts --no-audit --no-fund \
      "@modelcontextprotocol/conformance@$SUITE" "@modelcontextprotocol/client@$CLIENT" >/dev/null ) \
    || { echo "conformance: npm install failed" >&2; exit 1; }
fi

rc=0
for rev in 2026-07-28 2025-11-25; do
  for mode in http streamable; do
    PORT=$((PORT + 1))
    out="$W/$mode-$rev"
    rm -rf "$out"
    mkdir -p "$out"
    "$ROOT/build/conformance-server" "$mode" "$PORT" >/dev/null 2>&1 &
    pid=$!
    for _ in $(seq 1 50); do
      (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null && break
      sleep 0.1
    done
    ( cd "$W" && npx --no-install @modelcontextprotocol/conformance server \
        --url "http://127.0.0.1:$PORT/mcp" --requirements "$rev" \
        --expected-failures "$ROOT/tests/fixtures/conformance-expected-$rev.yaml" \
        -o "$out" ) > "$out.log" 2>&1
    s=$?
    kill -0 "$pid" 2>/dev/null || { echo "conformance: the server died on $mode $rev" >&2; s=1; }
    kill "$pid" 2>/dev/null
    wait "$pid" 2>/dev/null
    echo "$mode $rev: $(grep -E '^Total:' "$out.log")"
    if [ "$s" -ne 0 ]; then
      rc=1
      echo "conformance: $mode $rev does not match its baseline; see $out.log" >&2
    fi
  done
done

cp tests/fixtures/conformance_client.mjs "$W/client.mjs"
( cd "$W" && node client.mjs "$ROOT/build/conformance-server" ) || rc=1
exit "$rc"
