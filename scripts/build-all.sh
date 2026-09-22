#!/bin/sh
# build-all.sh — build the bote binary family.
#
# Per-transport binary split — originally (2.7.2) a workaround for the
# cyrius 5.10.x 2 MB compile-source cap, which 6.1.24 raised; folding the
# three back into one `bote` is a 3.4.x roadmap item. Builds three
# binaries from three entries:
#
#   build/bote              src/main.cyr            stdio + http + unix + bridge
#   build/bote-streamable   src/main_streamable.cyr Streamable HTTP / SSE
#   build/bote-ws           src/main_ws.cyr         WebSocket MCP
#
set -e

cyrius build src/main.cyr            build/bote
cyrius build src/main_streamable.cyr build/bote-streamable
cyrius build src/main_ws.cyr         build/bote-ws

echo
echo "Built:"
ls -lh build/bote build/bote-streamable build/bote-ws
