#!/usr/bin/env bash
# transport-smoke.sh — every mode of the one bote binary answers on its
# transport (3.4.2).
#
#   ./scripts/build-all.sh && ./scripts/transport-smoke.sh
#
# Starts http / bridge / streamable / ws / unix on loopback ports 18390-18395
# and a socket under build/tsmoke/, and checks:
#   - each answers `initialize` (ws: the upgrade answers 101);
#   - the argv[0] aliases bote-streamable / bote-ws select their transports;
#   - streamable keeps the tool set it had as a separate binary (the echo
#     references, no fs_write) while http has the full one.
# Needs curl and python3. CI runs it as the "Transport smoke" step.
set -uo pipefail
cd "$(dirname "$0")/.."
B=./build/bote
[ -x "$B" ] || { echo "::error::$B not built (run ./scripts/build-all.sh)"; exit 1; }
ln -sf bote build/bote-streamable
ln -sf bote build/bote-ws
INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}'
LIST='{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
H=(-H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2025-11-25')
fail=0; pids=()
cleanup() { for p in "${pids[@]}"; do kill "$p" 2>/dev/null; done; wait 2>/dev/null; }
trap cleanup EXIT
err() { echo "::error::$*"; fail=1; }
start() { # name logfile cmd...
  local name="$1" log="$2"; shift 2
  "$@" > /dev/null 2> "$log" & pids+=($!)
}
wait_port() { # port
  for _ in $(seq 1 50); do (exec 3<>/dev/tcp/127.0.0.1/"$1") 2>/dev/null && return 0; sleep 0.1; done
  return 1
}
mkdir -p build/tsmoke; L=build/tsmoke

start http "$L/http.err" $B http 18390
start bridge "$L/bridge.err" $B bridge 18391
start streamable "$L/strm.err" $B streamable 18392
start ws "$L/ws.err" $B ws 18393
start alias-streamable "$L/astrm.err" ./build/bote-streamable 18394
start alias-ws "$L/aws.err" ./build/bote-ws 18395
for p in 18390 18391 18392 18393 18394 18395; do wait_port $p || err "nothing listening on $p"; done

# http: initialize, and the full tool set (fs_write is registered for http).
r=$(curl -s "${H[@]}" -d "$INIT" http://127.0.0.1:18390/mcp)
echo "$r" | grep -q '"protocolVersion":"2025-11-25"' || err "http initialize: $r"
r=$(curl -s "${H[@]}" -d "$LIST" http://127.0.0.1:18390/mcp)
echo "$r" | grep -q '"name":"fs_write"' || err "http tools/list lacks fs_write: $r"

# bridge: POST / answers initialize.
r=$(curl -s -H 'Content-Type: application/json' -d "$INIT" http://127.0.0.1:18391/)
echo "$r" | grep -q '"protocolVersion"' || err "bridge initialize: $r"

# streamable: initialize mints a session; tools/list on it has the echo
# reference tools but NOT fs_write (the pre-3.4.2 bote-streamable tool set);
# listChanged is advertised because this transport has a drain path.
hdr=$(curl -s -D - -o "$L/strm-init.json" "${H[@]}" -d "$INIT" http://127.0.0.1:18392/mcp)
sid=$(echo "$hdr" | tr -d '\r' | sed -n 's/^MCP-Session-Id: //Ip' | head -1)
[ -n "$sid" ] || err "streamable initialize returned no MCP-Session-Id"
grep -q '"listChanged":true' "$L/strm-init.json" || err "streamable does not advertise listChanged: $(cat "$L/strm-init.json")"
r=$(curl -s "${H[@]}" -H "MCP-Session-Id: $sid" -d "$LIST" http://127.0.0.1:18392/mcp)
echo "$r" | grep -q '"name":"bote_echo"' || err "streamable tools/list lacks bote_echo: $r"
echo "$r" | grep -q '"name":"fs_write"' && err "streamable tools/list gained fs_write (tool-set parity broken): $r"

# ws: the handshake upgrades.
code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 2 \
  -H 'Connection: Upgrade' -H 'Upgrade: websocket' -H 'Sec-WebSocket-Version: 13' \
  -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' -H 'MCP-Protocol-Version: 2025-11-25' \
  http://127.0.0.1:18393/mcp)
[ "$code" = "101" ] || err "ws upgrade answered $code, not 101"

# argv[0] aliases: the old binary names select their transports.
grep -q 'bote starting .*transport=streamable .*port=18394' "$L/astrm.err" || err "bote-streamable alias did not start streamable: $(cat "$L/astrm.err")"
grep -q 'bote starting .*transport=ws .*port=18395' "$L/aws.err" || err "bote-ws alias did not start ws: $(cat "$L/aws.err")"
r=$(curl -s "${H[@]}" -d "$INIT" http://127.0.0.1:18394/mcp)
echo "$r" | grep -q '"protocolVersion"' || err "bote-streamable alias initialize: $r"

# unix: one line in, one line out.
sock="$L/bote.sock"; rm -f "$sock"
start unix "$L/unix.err" $B unix "$sock"
for _ in $(seq 1 50); do [ -S "$sock" ] && break; sleep 0.1; done
r=$(python3 -I -c 'import socket,sys
s=socket.socket(socket.AF_UNIX); s.settimeout(3); s.connect(sys.argv[1])
s.sendall(sys.argv[2].encode()+b"\n"); b=b""
while b"\n" not in b:
    c=s.recv(4096)
    if not c: break
    b+=c
print(b.decode().strip())' "$sock" "$INIT")
echo "$r" | grep -q '"protocolVersion"' || err "unix initialize: $r"

for f in "$L"/*.err; do grep -q 'transport failed' "$f" && err "$(basename "$f"): $(grep 'transport failed' "$f")"; done
[ $fail -eq 0 ] || exit 1
echo "transport smoke: http bridge streamable ws unix + both argv[0] aliases answer; streamable keeps its tool set"
