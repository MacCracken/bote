#!/bin/sh
# build-all.sh — build the bote binary family.
#
# 2.7.2 (cyrius 5.10.x): per-transport binary split workaround for the
# 2 MB compile-source cap. Builds three binaries from three entries:
#
#   build/bote              src/main.cyr            stdio + http + unix + bridge
#   build/bote-streamable   src/main_streamable.cyr Streamable HTTP / SSE
#   build/bote-ws           src/main_ws.cyr         WebSocket MCP
#
# Reconsolidates to a single `bote` binary on cyrius 5.11.x migration.
set -e

cyrius build src/main.cyr            build/bote
cyrius build src/main_streamable.cyr build/bote-streamable
cyrius build src/main_ws.cyr         build/bote-ws

echo
echo "Built:"
ls -lh build/bote build/bote-streamable build/bote-ws
