#!/bin/sh
# build-all.sh — build the bote binary.
#
# One binary since 3.4.2, every transport selected by argv:
#
#   build/bote              src/main.cyr    stdio | http | unix | bridge | streamable | ws
#
# From 2.7.2 to 3.4.1 streamable HTTP and WebSocket were separate binaries
# (bote-streamable, bote-ws), a workaround for the cyrius 5.10.x 2 MB
# compile-source cap that 6.1.24 raised. bote still answers to those two
# names as argv[0] aliases, so the symlinks below keep an old invocation
# (`build/bote-streamable 8392`) working. `ln -sf` also replaces a stale
# pre-3.4.2 binary of the same name left in build/.
set -e

cyrius build src/main.cyr build/bote

ln -sf bote build/bote-streamable
ln -sf bote build/bote-ws

echo
echo "Built:"
ls -lh build/bote build/bote-streamable build/bote-ws
