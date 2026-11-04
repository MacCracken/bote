#!/usr/bin/env bash
# transport-smoke.sh — every mode of the one bote binary answers on its
# transport (3.4.2).
#
#   ./scripts/build-all.sh && ./scripts/transport-smoke.sh
#
# Starts http / bridge / streamable / ws / unix on loopback ports 18390-18398
# and a socket under build/tsmoke/, and checks:
#   - each answers `initialize` (ws: the upgrade answers 101);
#   - the argv[0] aliases bote-streamable / bote-ws select their transports;
#   - streamable keeps the tool set it had as a separate binary (the echo
#     references, no fs_write) while http has the full one;
#   - each sandhi-served mode, started again on a port already held, exits 1
#     and names the step and errno (3.4.12);
#   - a hostile Origin gets 403 on every HTTP-family mode, a listed one passes,
#     an empty BOTE_BEARER_TOKENS refuses to start, and the fs tools stay in
#     their root through a symlink and do not hang on a FIFO (3.4.15);
#   - BOTE_TOOL_SCOPES answers a token without a scope with a 403 that names it,
#     and refuses to start on a name it cannot honour (3.6.3);
#   - a token seen twice is judged from the cache the second time, BOTE_JWT_CACHE=0
#     turns that off, and the token never reaches the log (3.6.4);
#   - web_search, made in a child, returns a loopback SearXNG's results and its
#     status, writes nothing to the stdio wire, and gives up on a server that never
#     answers by BOTE_WEB_TIMEOUT_MS (3.6.5);
#   - streamable on worker threads answers beside a held connection, which stops it
#     with BOTE_WORKERS=0, holds BOTE_WORKERS to its bound, and serves sixteen
#     clients at once while a jwks_uri is fetched every second from its workers (3.7.0);
#   - a subscriptions/listen stream on http, bridge and streamable: its acknowledgement,
#     keepalives, a client that goes (3.7.2); and over stdio, unix and ws, where the
#     connection is the stream: the acknowledgement first and a cancel that ends it (3.7.3).
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

# MCP 2026-07-28 on every HTTP server, each of which just served a legacy client.
# A client of 2025-11-25 has no version to send on `initialize`, either.
MD='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}'
DISC="{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server/discover\",\"params\":{$MD}}"
CALL="{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{$MD,\"name\":\"bote_echo\",\"arguments\":{\"msg\":\"hi\"}}}"
MV=(-H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2026-07-28')
NOTOOL="{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{$MD,\"name\":\"no_such_tool\"}}"
for m in "18390 /mcp" "18391 /" "18392 /mcp"; do
  read -r p path <<< "$m"
  r=$(curl -s -w ' HTTP%{http_code}' "${MV[@]}" -H 'Mcp-Method: server/discover' -d "$DISC" "http://127.0.0.1:$p$path")
  echo "$r" | grep -q '"supportedVersions":\["2026-07-28".* HTTP200$' || err "$p modern server/discover: $r"
  r=$(curl -s -w ' HTTP%{http_code}' "${MV[@]}" -H 'Mcp-Method: tools/call' -H 'Mcp-Name: bote_echo' -d "$CALL" "http://127.0.0.1:$p$path")
  echo "$r" | grep -q '"resultType":"complete".* HTTP200$' || err "$p modern tools/call: $r"
  r=$(curl -s -w ' HTTP%{http_code}' "${MV[@]}" -H 'Mcp-Method: tools/call' -H 'Mcp-Name: other' -d "$CALL" "http://127.0.0.1:$p$path")
  echo "$r" | grep -q '"code":-32020.* HTTP400$' || err "$p Mcp-Name mismatch: $r"
  r=$(curl -s -w ' HTTP%{http_code}' "${MV[@]}" -H 'Mcp-Method: tools/call' -H 'Mcp-Name: no_such_tool' -d "$NOTOOL" "http://127.0.0.1:$p$path")
  echo "$r" | grep -q '"code":-32602.* HTTP200$' || err "$p an unknown tool, not in-band: $r"
  r=$(curl -s -w ' HTTP%{http_code}' -H 'Content-Type: application/json' -d "$DISC" "http://127.0.0.1:$p$path")
  echo "$r" | grep -q '"code":-32020.* HTTP400$' || err "$p modern body with no version header: $r"
  r=$(curl -s -D - -o /dev/null -H 'Content-Type: application/json' -d "$INIT" "http://127.0.0.1:$p$path")
  echo "$r" | sed -n 1p | grep -q ' 200 ' || err "$p initialize with no version header: $r"
done
c=$(curl -s -o /dev/null -w '%{http_code}' -H 'MCP-Protocol-Version: 2026-07-28' http://127.0.0.1:18392/mcp)
[ "$c" = "405" ] || err "a modern GET answered $c, not 405"

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
r=$(python3 -I -c 'import socket,sys
s=socket.socket(socket.AF_UNIX); s.settimeout(3); s.connect(sys.argv[1])
s.sendall(sys.argv[2].encode()+b"\n"); b=b""
while b"\n" not in b:
    c=s.recv(4096)
    if not c: break
    b+=c
print(b.decode().strip())' "$sock" "$DISC")
echo "$r" | grep -q '"supportedVersions":\["2026-07-28"' || err "unix modern server/discover: $r"

# ws: a modern request in a real frame.
r=$(python3 -I -c 'import base64,os,socket,struct,sys
s=socket.create_connection(("127.0.0.1",18393),3); s.settimeout(3)
k=base64.b64encode(os.urandom(16)).decode()
s.sendall(("GET /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: %s\r\n\r\n"%k).encode())
b=b""
while b"\r\n\r\n" not in b: b+=s.recv(4096)
b=b.split(b"\r\n\r\n",1)[1]
p=sys.argv[1].encode(); m=os.urandom(4)
h=b"\x81"+(bytes([128|len(p)]) if len(p)<126 else bytes([254])+struct.pack(">H",len(p)))
s.sendall(h+m+bytes(c^m[i%4] for i,c in enumerate(p)))
while len(b)<4: b+=s.recv(4096)
n=b[1]&127; o=2
if n==126: n=struct.unpack(">H",b[2:4])[0]; o=4
while len(b)<o+n: b+=s.recv(4096)
print(b[o:o+n].decode())' "$DISC")
echo "$r" | grep -q '"supportedVersions":\["2026-07-28"' || err "ws modern server/discover: $r"

# Every server above is still up, so a second one on its port has to fail. These
# logs are .held, not .err: the check below fails the run on 'transport failed'.
for m in "http 18390" "bridge 18391" "streamable 18392" "ws 18393"; do
  read -r mode port <<< "$m"
  rc=0; timeout 5 $B "$mode" "$port" 2> "$L/$mode.held" || rc=$?
  [ "$rc" -eq 1 ] || err "$mode on a held port: rc=$rc, not 1"
  grep -q "\[ERROR\] serve failed transport=$mode step=bind port=$port errno=98 cause=\"port in use\"" "$L/$mode.held" \
    || err "$mode on a held port names no step: $(cat "$L/$mode.held")"
  grep -q "\[ERROR\] transport failed transport=$mode rc=1" "$L/$mode.held" \
    || err "$mode on a held port has no exit line: $(cat "$L/$mode.held")"
done

# Origin (3.4.15): MCP says a present, invalid Origin gets 403. No list is set
# here, so every website is refused, and a client that sends no Origin is not.
# The server's own loopback address is no website: nothing on this port serves a page.
for m in "http 18390 /mcp" "bridge 18391 /" "streamable 18392 /mcp"; do
  read -r mode port path <<< "$m"
  c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H 'Origin: https://evil.example' -d "$INIT" "http://127.0.0.1:$port$path")
  [ "$c" = "403" ] || err "$mode answered a hostile Origin with $c, not 403"
  c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H "Origin: http://127.0.0.1:$port" -d "$INIT" "http://127.0.0.1:$port$path")
  [ "$c" = "200" ] || err "$mode answered its own Origin with $c, not 200"
  c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H 'Origin: http://127.0.0.1:1' -d "$INIT" "http://127.0.0.1:$port$path")
  [ "$c" = "403" ] || err "$mode answered another local port with $c, not 403"
done
c=$(curl -s -o /dev/null -w '%{http_code}' -X OPTIONS -H 'Origin: https://evil.example' http://127.0.0.1:18391/)
[ "$c" = "403" ] || err "bridge answered a hostile preflight with $c, not 403"
c=$(curl -s -o /dev/null -w '%{http_code}' -H 'Origin: https://evil.example' -H 'Connection: Upgrade' -H 'Upgrade: websocket' \
  -H 'Sec-WebSocket-Version: 13' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' http://127.0.0.1:18393/mcp)
[ "$c" = "403" ] || err "ws answered a hostile Origin with $c, not 403"
# An operator's list lets a named Origin in, and only that one.
BOTE_ALLOWED_ORIGINS=https://ok.example start origins "$L/origins.err" $B bridge 18396
wait_port 18396 || err "nothing listening on 18396"
h=$(curl -s -D - -o /dev/null -X OPTIONS -H 'Origin: https://ok.example' http://127.0.0.1:18396/)
echo "$h" | grep -qi '^access-control-allow-origin: https://ok.example' || err "bridge did not allow a listed Origin: $h"
c=$(curl -s -o /dev/null -w '%{http_code}' -H 'Origin: https://evil.example' -d "$INIT" http://127.0.0.1:18396/)
[ "$c" = "403" ] || err "bridge with a list answered a hostile Origin with $c, not 403"
# Protected Resource Metadata: public where the rest wants a token, and a 401 points at it.
BOTE_RESOURCE_URL=http://127.0.0.1:18397/mcp BOTE_AUTH_SERVERS=https://as.example.com BOTE_AUTH_SCOPES=mcp:read \
  BOTE_BEARER_TOKENS=tok start prm "$L/prm.err" $B http 18397
wait_port 18397 || err "nothing listening on 18397"
r=$(curl -s -w ' HTTP%{http_code}' http://127.0.0.1:18397/.well-known/oauth-protected-resource/mcp)
echo "$r" | grep -q '"resource":"http://127.0.0.1:18397/mcp","authorization_servers":\["https://as.example.com"\].* HTTP200$' \
  || err "the metadata document: $r"
h=$(curl -s -D - -o /dev/null -X POST -H 'Content-Type: application/json' -d "$INIT" http://127.0.0.1:18397/mcp)
echo "$h" | sed -n 1p | grep -q ' 401 ' || err "no token, metadata on: $h"
echo "$h" | grep -qi '^www-authenticate: Bearer resource_metadata="http://127.0.0.1:18397/.well-known/oauth-protected-resource/mcp", scope="mcp:read"' \
  || err "the 401 does not name the metadata: $h"
# What cannot be published stops the server, which says why. These go to .held.
rc=0; BOTE_RESOURCE_URL=http://example.com/mcp BOTE_AUTH_SERVERS=https://as.example.com timeout 5 $B http 18399 2> "$L/prm-http.held" || rc=$?
[ "$rc" -eq 2 ] || err "a resource on plain http off loopback: rc=$rc, not 2"
grep -q '\[ERROR\] resource metadata refused' "$L/prm-http.held" || err "no reason for the refused resource: $(cat "$L/prm-http.held")"
rc=0; BOTE_AUTH_SERVERS=https://as.example.com timeout 5 $B http 18399 2> "$L/prm-half.held" || rc=$?
[ "$rc" -eq 2 ] || err "authorization servers with no resource: rc=$rc, not 2"

# A signed token is let in only if it was issued for this server, and a bad configuration
# stops the server. The key must not reach a log.
JKEY=smoke-secret-smoke-secret-smoke-secret
jwt() {
  python3 -I -c 'import base64,hashlib,hmac,json,sys
e=lambda b: base64.urlsafe_b64encode(b).rstrip(b"=").decode()
s=e(b"{\"alg\":\"HS256\"}")+"."+e(sys.argv[2].encode())
print(s+"."+e(hmac.new(sys.argv[1].encode(),s.encode(),hashlib.sha256).digest()))' "$JKEY" "$1"
}
BOTE_LOG=debug BOTE_RESOURCE_URL=http://127.0.0.1:18399/mcp BOTE_AUTH_SERVERS=https://as.example.com BOTE_JWT_SECRET=$JKEY \
  start jwt "$L/jwt.err" $B http 18399
wait_port 18399 || err "nothing listening on 18399"
ok_t=$(jwt '{"aud":"http://127.0.0.1:18399/mcp","iss":"https://as.example.com","exp":4102444800}')
c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H "Authorization: Bearer $ok_t" -d "$INIT" http://127.0.0.1:18399/mcp)
[ "$c" = "200" ] || err "a token for this server answered $c, not 200"
bad_t=$(jwt '{"aud":"https://other.example.com/mcp","iss":"https://as.example.com","exp":4102444800}')
h=$(curl -s -D - -o /dev/null "${H[@]}" -H "Authorization: Bearer $bad_t" -d "$INIT" http://127.0.0.1:18399/mcp)
echo "$h" | sed -n 1p | grep -q ' 401 ' || err "a token for another server: $h"
echo "$h" | grep -qi 'error="invalid_token"' || err "a token for another server has no error code: $h"
bad_t=$(jwt '{"aud":"http://127.0.0.1:18399/mcp","iss":"https://evil.example.com","exp":4102444800}')
c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H "Authorization: Bearer $bad_t" -d "$INIT" http://127.0.0.1:18399/mcp)
[ "$c" = "401" ] || err "a token from another issuer answered $c, not 401"
grep -q "$JKEY" "$L/jwt.err" && err "the JWT key reached a log"

# Tokens already verified: the second request of a token is judged from the cache, at
# TRACE, with neither the token nor its claims in the line.
CURL4=http://127.0.0.1:18384/mcp
ctok=$(jwt '{"aud":"http://127.0.0.1:18384/mcp","iss":"https://as.example.com","exp":4102444800}')
cache_run() { # name expected-hit-lines env...
  local name=$1 want=$2; shift 2
  env "$@" BOTE_LOG=trace BOTE_RESOURCE_URL=$CURL4 BOTE_AUTH_SERVERS=https://as.example.com BOTE_JWT_SECRET=$JKEY \
    bash -c "exec $B http 18384" 2> "$L/cache-$name.err" &
  local pid=$!
  wait_port 18384 || err "cache $name: nothing listening on 18384"
  for _ in 1 2 3; do
    c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H "Authorization: Bearer $ctok" -d "$INIT" $CURL4)
    [ "$c" = "200" ] || err "cache $name: a good token answered $c"
  done
  kill $pid 2>/dev/null; wait $pid 2>/dev/null
  local got
  got=$(grep -c 'jwt verified from the cache' "$L/cache-$name.err")
  [ "$got" = "$want" ] || err "cache $name: $got requests judged from the cache, not $want"
  grep -q "$ctok" "$L/cache-$name.err" && err "cache $name: the token reached a log"
  grep -q "$JKEY" "$L/cache-$name.err" && err "cache $name: the key reached a log"
}
cache_run default 2
cache_run off 0 BOTE_JWT_CACHE=0
cache_run sized 2 BOTE_JWT_CACHE=8
grep -q 'verified tokens remembered.*slots=8' "$L/cache-sized.err" || err "a cache size was not logged at startup"
cache_run garbage 2 BOTE_JWT_CACHE=lots
grep -q 'not a count; keeping the default.*var=BOTE_JWT_CACHE' "$L/cache-garbage.err" || err "BOTE_JWT_CACHE=lots was not called out"

# Required scopes, on the real binary: a 403 naming the scope, the same on a token that
# holds it or a broader one, and a refusal to start on anything that would leave a tool
# open. The token never reaches a log.
SURL=http://127.0.0.1:18386/mcp
SENV=(BOTE_RESOURCE_URL=$SURL BOTE_AUTH_SERVERS=https://as.example.com BOTE_JWT_SECRET=$JKEY)
env "${SENV[@]}" BOTE_LOG=debug BOTE_TOOL_SCOPES='bote_echo=files:write' BOTE_SCOPE_IMPLIES='admin=files:write' \
  bash -c "exec $B http 18386" 2> "$L/scopes.err" &
pids+=($!)
wait_port 18386 || err "nothing listening on 18386"
CALL='{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"bote_echo","arguments":{"message":"hi"}}}'
stok() { jwt "{\"aud\":\"$SURL\",\"iss\":\"https://as.example.com\",\"exp\":4102444800,$1}"; }
poor=$(stok '"scope":"files:read"')
h=$(curl -s -D - -o /dev/null "${H[@]}" -H "Authorization: Bearer $poor" -d "$CALL" $SURL)
echo "$h" | sed -n 1p | grep -q ' 403 ' || err "a token without the scope: $h"
echo "$h" | grep -qi 'error="insufficient_scope", scope="files:write", resource_metadata="http://127.0.0.1:18386/.well-known/oauth-protected-resource/mcp"' \
  || err "the 403 does not name the scope and the metadata: $h"
c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H "Authorization: Bearer $(stok '"scope":"a files:write"')" -d "$CALL" $SURL)
[ "$c" = "200" ] || err "a token with the scope answered $c, not 200"
c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H "Authorization: Bearer $(stok '"scope":"admin"')" -d "$CALL" $SURL)
[ "$c" = "200" ] || err "a broader scope answered $c, not 200"
c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H "Authorization: Bearer $poor" -d "$LIST" $SURL)
[ "$c" = "200" ] || err "a token without the scope could not list tools: $c"
curl -s "${H[@]}" -H "Authorization: Bearer $poor" -d "[$CALL]" $SURL | grep -q '"code":-31403' || err "a batch element was not refused with -31403"
grep -q 'status=403 reason=insufficient_scope' "$L/scopes.err" || err "a 403 for a scope was not logged"
grep -q "$poor" "$L/scopes.err" && err "a token reached a log"
srefused() { # why expected-log env...
  local why=$1 want=$2; shift 2
  local rc=0
  env "$@" timeout 5 $B http 18385 2> "$L/scopes-refused.held" || rc=$?
  [ "$rc" -eq 2 ] || err "$why: rc=$rc, not 2"
  grep -q "$want" "$L/scopes-refused.held" || err "$why: no reason: $(cat "$L/scopes-refused.held")"
}
srefused "a scope on a tool that does not exist" 'tool=nope' "${SENV[@]}" BOTE_TOOL_SCOPES=nope=files:write
srefused "scopes with no verifier" 'needs BOTE_JWT_SECRET' BOTE_TOOL_SCOPES=bote_echo=files:write
srefused "scopes on a token list" 'needs BOTE_JWT_SECRET' BOTE_BEARER_TOKENS=tok BOTE_TOOL_SCOPES=bote_echo=files:write
srefused "an entry with no =" 'entries are tool=scope' "${SENV[@]}" BOTE_TOOL_SCOPES=bote_echo
srefused "an empty scope" 'BOTE_TOOL_SCOPES refused' "${SENV[@]}" BOTE_TOOL_SCOPES=bote_echo=
srefused "a scope that is no token" 'BOTE_TOOL_SCOPES refused' "${SENV[@]}" 'BOTE_TOOL_SCOPES=bote_echo=a"b'
srefused "a rule with no =" 'entries are broad=narrow' "${SENV[@]}" BOTE_TOOL_SCOPES=bote_echo=a BOTE_SCOPE_IMPLIES=admin
srefused "a tool named twice" 'names a tool twice.*tool=bote_echo' "${SENV[@]}" 'BOTE_TOOL_SCOPES=bote_echo=files:write admin,bote_echo=x'
echo "$LIST" | env BOTE_TOOL_SCOPES=bote_echo=files:write timeout 5 $B stdio 2> "$L/scopes-stdio.held" | grep -q '"tools"' \
  || err "stdio stopped serving with BOTE_TOOL_SCOPES set"
grep -q 'no effect here' "$L/scopes-stdio.held" || err "stdio did not say scopes have no effect on it"

rc=0; BOTE_JWT_SECRET=short BOTE_RESOURCE_URL=http://127.0.0.1:18399/mcp BOTE_AUTH_SERVERS=https://as.example.com timeout 5 $B http 18388 2> "$L/jwt-short.held" || rc=$?
[ "$rc" -eq 2 ] || err "a short JWT key: rc=$rc, not 2"
rc=0; BOTE_JWT_SECRET=$JKEY timeout 5 $B http 18388 2> "$L/jwt-noaud.held" || rc=$?
[ "$rc" -eq 2 ] || err "a JWT key with no resource: rc=$rc, not 2"
grep -q "$JKEY" "$L/jwt-noaud.held" && err "the JWT key reached a log on the way out"
rc=0; BOTE_JWT_SECRET=$JKEY BOTE_BEARER_TOKENS=tok BOTE_RESOURCE_URL=http://127.0.0.1:18399/mcp BOTE_AUTH_SERVERS=https://as.example.com timeout 5 $B http 18388 2> "$L/jwt-both.held" || rc=$?
[ "$rc" -eq 2 ] || err "a JWT key and a token list: rc=$rc, not 2"

# Public keys from a JWK Set: a token signed by another implementation is let in, and one
# for another server is not. The vectors are valid until 2100.
vec() { python3 -I -c 'import re,sys
for l in open("tests/fixtures/jwt_vectors.cyr"):
    m=re.match(r"var "+sys.argv[1]+r" = \"(.*)\";$",l)
    if m: print(m.group(1).replace("\\\"","\"").replace("\\\\","\\"))' "$1"; }
vec JV_JWKS > "$L/jwks.json"
BOTE_LOG=debug BOTE_RESOURCE_URL=https://mcp.example.com/mcp BOTE_AUTH_SERVERS=https://as.example.com BOTE_JWKS_FILE="$L/jwks.json" \
  start jwks "$L/jwks.err" $B http 18387
wait_port 18387 || err "nothing listening on 18387"
for t in JV_RS_GOOD JV_ES_GOOD; do
  c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H "Authorization: Bearer $(vec $t)" -d "$INIT" http://127.0.0.1:18387/mcp)
  [ "$c" = "200" ] || err "$t answered $c, not 200"
done
for t in JV_RS_OTHER_AUD JV_ES_OTHER_AUD JV_HS_WITH_PEM JV_ES_ZERO_SIG; do
  c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H "Authorization: Bearer $(vec $t)" -d "$INIT" http://127.0.0.1:18387/mcp)
  [ "$c" = "401" ] || err "$t answered $c, not 401"
done
rc=0; BOTE_JWKS_FILE="$L/jwks.json" BOTE_JWT_SECRET=$JKEY BOTE_RESOURCE_URL=https://mcp.example.com/mcp BOTE_AUTH_SERVERS=https://as.example.com timeout 5 $B http 18388 2> "$L/jwks-both.held" || rc=$?
[ "$rc" -eq 2 ] || err "a JWK Set and a shared key: rc=$rc, not 2"
rc=0; BOTE_JWKS_FILE="$L/no-such-file" BOTE_RESOURCE_URL=https://mcp.example.com/mcp BOTE_AUTH_SERVERS=https://as.example.com timeout 5 $B http 18388 2> "$L/jwks-gone.held" || rc=$?
[ "$rc" -eq 2 ] || err "a JWK Set that is not there: rc=$rc, not 2"
vec JV_JWK_PRIVATE_RSA | python3 -I -c 'import sys; print("{\"keys\":["+sys.stdin.read().strip()+"]}")' > "$L/jwks-private.json"
rc=0; BOTE_JWKS_FILE="$L/jwks-private.json" BOTE_RESOURCE_URL=https://mcp.example.com/mcp BOTE_AUTH_SERVERS=https://as.example.com timeout 5 $B http 18388 2> "$L/jwks-private.held" || rc=$?
[ "$rc" -eq 2 ] || err "a JWK Set holding a private key: rc=$rc, not 2"
grep -q '\[ERROR\] BOTE_JWKS_FILE refused' "$L/jwks-private.held" || err "no reason for the refused set: $(cat "$L/jwks-private.held")"
# Keys from a jwks_uri (3.6.6): fetched at startup, asked again for a key the set does not
# hold, refreshed so that a key that left the set stops passing, and kept while the key
# server is down. Over http to loopback, then over https with BOTE_JWKS_CA.
jk="$PWD/$L/jwksrv"; mkdir -p "$jk"
vec JV_JWKS > "$jk/full.json"
vec JV_JWK_EC | python3 -I -c 'import sys; print("{\"keys\":["+sys.stdin.read().strip()+"]}")' > "$jk/ec.json"
cp "$jk/ec.json" "$jk/jwks.json"
python3 -I -m http.server 18382 --bind 127.0.0.1 --directory "$jk" > /dev/null 2>&1 & jkpid=$!; pids+=($jkpid)
wait_port 18382 || err "jwks url: the loopback key server did not start"
JENV=(BOTE_RESOURCE_URL=https://mcp.example.com/mcp BOTE_AUTH_SERVERS=https://as.example.com)
jcode() { curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H "Authorization: Bearer $(vec "$1")" -d "$INIT" "http://127.0.0.1:$2/mcp"; }
env "${JENV[@]}" BOTE_LOG=debug BOTE_JWKS_URL=http://127.0.0.1:18382/jwks.json BOTE_JWKS_REFRESH=1 $B http 18383 > /dev/null 2> "$L/jwksurl.err" & pids+=($!)
wait_port 18383 || err "jwks url: nothing listening on 18383: $(cat "$L/jwksurl.err")"
grep -q 'jwt keys from a jwks_uri host=127.0.0.1 keys=1 refresh_s=1' "$L/jwksurl.err" || err "jwks url: no startup line: $(cat "$L/jwksurl.err")"
c=$(jcode JV_ES_NO_KID 18383); [ "$c" = "200" ] || err "jwks url: a token on the one key answered $c, not 200"
c=$(jcode JV_RS_GOOD 18383); [ "$c" = "401" ] || err "jwks url: an RS256 token on a set with no RS256 key answered $c, not 401"
cp "$jk/full.json" "$jk/jwks.json"; sleep 1.2
c=$(jcode JV_RS_GOOD 18383); [ "$c" = "200" ] || err "jwks url: once the set held r1, its token answered $c, not 200"
grep -q 'jwks keys changed keys=6' "$L/jwksurl.err" || err "jwks url: no line for the new set"
cp "$jk/ec.json" "$jk/jwks.json"; sleep 1.2
c=$(jcode JV_RS_GOOD 18383); [ "$c" = "401" ] || err "jwks url: once r1 left the set, its remembered token answered $c, not 401"
kill "$jkpid" 2>/dev/null; wait "$jkpid" 2>/dev/null; sleep 1.2
c=$(jcode JV_ES_NO_KID 18383); [ "$c" = "200" ] || err "jwks url: with the key server down, a token on the set it had answered $c, not 200"
grep -q 'jwks refresh failed; keeping the keys it has reason="could not connect' "$L/jwksurl.err" || err "jwks url: no WARN for the server that is down: $(tail -3 "$L/jwksurl.err")"
grep -q 'eyJ\|AQAB' "$L/jwksurl.err" && err "jwks url: a token or a key reached the log"
rc=0; env "${JENV[@]}" BOTE_JWKS_URL=http://example.com/jwks.json timeout 5 $B http 18388 2> "$L/jwksurl-http.held" || rc=$?
[ "$rc" -eq 2 ] && grep -q 'BOTE_JWKS_URL refused reason="an http:// jwks_uri must name a loopback address' "$L/jwksurl-http.held" \
  || err "jwks url: http to a non-loopback host: rc=$rc $(cat "$L/jwksurl-http.held")"
rc=0; env "${JENV[@]}" BOTE_JWKS_URL=http://127.0.0.1:1/jwks.json timeout 10 $B http 18388 2> "$L/jwksurl-down.held" || rc=$?
[ "$rc" -eq 2 ] && grep -q 'BOTE_JWKS_URL: the first fetch failed' "$L/jwksurl-down.held" \
  || err "jwks url: a key server that is down at startup: rc=$rc $(cat "$L/jwksurl-down.held")"
rc=0; env "${JENV[@]}" BOTE_JWKS_URL=http://127.0.0.1:18382/jwks.json BOTE_JWKS_FILE="$L/jwks.json" timeout 5 $B http 18388 2> "$L/jwksurl-both.held" || rc=$?
[ "$rc" -eq 2 ] && grep -q 'verifies with one of them' "$L/jwksurl-both.held" || err "jwks url: a URL and a file: rc=$rc"
if command -v openssl > /dev/null; then
  jt="$jk/tls"; mkdir -p "$jt"
  openssl req -x509 -newkey rsa:2048 -nodes -keyout "$jt/ca.key" -out "$jt/ca.pem" -days 2 -subj "/CN=bote smoke CA" 2> /dev/null
  openssl req -newkey rsa:2048 -nodes -keyout "$jt/srv.key" -out "$jt/srv.csr" -subj "/CN=localhost" 2> /dev/null
  printf 'subjectAltName=DNS:localhost\n' > "$jt/ext.cnf"
  openssl x509 -req -in "$jt/srv.csr" -CA "$jt/ca.pem" -CAkey "$jt/ca.key" -CAcreateserial -out "$jt/srv.pem" -days 2 -extfile "$jt/ext.cnf" 2> /dev/null
  python3 -I -c 'import http.server,ssl,sys,os
os.chdir(sys.argv[1])
class Q(http.server.SimpleHTTPRequestHandler):
    def log_message(self,*a): pass
s=http.server.HTTPServer(("127.0.0.1",18389),Q)
c=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER); c.load_cert_chain(sys.argv[2],sys.argv[3])
s.socket=c.wrap_socket(s.socket,server_side=True)
s.serve_forever()' "$jk" "$jt/srv.pem" "$jt/srv.key" > /dev/null 2>&1 & pids+=($!)
  wait_port 18389 || err "jwks tls: the key server did not start"
  env "${JENV[@]}" BOTE_JWKS_URL=https://localhost:18389/full.json BOTE_JWKS_CA="$jt/ca.pem" $B http 18379 > /dev/null 2> "$L/jwkstls.err" & pids+=($!)
  wait_port 18379 || err "jwks tls: nothing listening on 18379: $(cat "$L/jwkstls.err")"
  c=$(jcode JV_RS_GOOD 18379); [ "$c" = "200" ] || err "jwks tls: an RS256 token on keys fetched over TLS answered $c, not 200"
  rc=0; env "${JENV[@]}" BOTE_JWKS_URL=https://localhost:18389/full.json timeout 10 $B http 18388 2> "$L/jwkstls-noca.held" || rc=$?
  [ "$rc" -eq 2 ] && grep -q 'TLS to the jwks_uri failed' "$L/jwkstls-noca.held" \
    || err "jwks tls: a private CA without BOTE_JWKS_CA: rc=$rc $(cat "$L/jwkstls-noca.held")"
else
  echo "jwks tls: no openssl, skipped"
fi
# Worker threads (3.7.0): streamable serves on BOTE_WORKERS threads, 4 by default, so a
# client that holds a connection open holds one worker and not the server. Sixteen clients
# at once then answer while the keys are fetched from a jwks_uri every second, each fetch
# forked from a worker beside the others. BOTE_WORKERS=0 is the one loop it was, which the
# same held connection stops.
grep -q 'worker threads workers=4' "$L/strm.err" || err "workers: streamable did not start 4: $(head -3 "$L/strm.err")"
hold() { python3 -I -c 'import socket,sys,time
s=socket.create_connection(("127.0.0.1",int(sys.argv[1])))
s.sendall(b"POST /mcp HTTP/1.1\r\nHost: x\r\n")
time.sleep(float(sys.argv[2]))' "$1" "$2" & pids+=($!); sleep 0.2; }
hold 18392 4
c=$(curl -s -o /dev/null -w '%{http_code}' --max-time 2 "${H[@]}" -d "$INIT" http://127.0.0.1:18392/mcp)
[ "$c" = "200" ] || err "workers: a held connection kept another client from an answer ($c)"
BOTE_WORKERS=0 start workers0 "$L/workers0.err" $B streamable 18378
wait_port 18378 || err "workers: nothing listening on 18378"
grep -q 'worker threads workers=0' "$L/workers0.err" || err "workers: BOTE_WORKERS=0 not taken: $(head -3 "$L/workers0.err")"
hold 18378 3
c=$(curl -s -o /dev/null -w '%{http_code}' --max-time 1 "${H[@]}" -d "$INIT" http://127.0.0.1:18378/mcp)
[ "$c" = "200" ] && err "workers: with BOTE_WORKERS=0 a held connection held nothing, which this check needs to show"
# Twelve digits: a count past what an int of nine holds is held to the bound too (3.7.1).
BOTE_WORKERS=500000000000 start workers500 "$L/workers500.err" $B streamable 18377
wait_port 18377 || err "workers: nothing listening on 18377"
grep -q 'BOTE_WORKERS past its bound; holding it there workers=999999999 max=64' "$L/workers500.err" && grep -q 'worker threads workers=64' "$L/workers500.err" \
  || err "workers: a twelve-digit count not held at 64: $(head -3 "$L/workers500.err")"
# Leading zeros are not digits of the count (3.7.2): ten of them were 999999999, and 64 workers.
BOTE_WORKERS=0000000000 start workers00 "$L/workers00.err" $B streamable 18374
BOTE_WORKERS=0000000004 start workers04 "$L/workers04.err" $B streamable 18373
wait_port 18374 && wait_port 18373 || err "workers: nothing listening on 18374 or 18373"
grep -q 'worker threads workers=0' "$L/workers00.err" || err "workers: ten zeros not read as 0: $(head -3 "$L/workers00.err")"
grep -q 'worker threads workers=4' "$L/workers04.err" || err "workers: 0000000004 not read as 4: $(head -3 "$L/workers04.err")"
python3 -I -m http.server 18376 --bind 127.0.0.1 --directory "$jk" > /dev/null 2>&1 & pids+=($!)
wait_port 18376 || err "workers: the key server did not start"
env "${JENV[@]}" BOTE_LOG=debug BOTE_JWKS_URL=http://127.0.0.1:18376/full.json BOTE_JWKS_REFRESH=1 BOTE_WORKERS=4 \
  $B streamable 18375 > /dev/null 2> "$L/workersjk.err" & pids+=($!)
wait_port 18375 || err "workers: nothing listening on 18375: $(cat "$L/workersjk.err")"
r=$(python3 -I -c 'import http.client,json,sys,threading,time
port=int(sys.argv[1]); toks=sys.argv[2:]; bad=[]; ok=[0]; lk=threading.Lock()
disc=json.dumps({"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}})
hd={"Content-Type":"application/json","Accept":"application/json","MCP-Protocol-Version":"2026-07-28","Mcp-Method":"server/discover"}
def run(t):
    end=time.time()+3.5
    while time.time()<end:
        c=http.client.HTTPConnection("127.0.0.1",port,timeout=10)
        c.request("POST","/mcp",disc,dict(hd,Authorization="Bearer "+toks[t%len(toks)]))
        r=c.getresponse(); d=r.read(); c.close()
        with lk:
            if r.status==200 and b"supportedVersions" in d: ok[0]+=1
            else: bad.append(r.status)
th=[threading.Thread(target=run,args=(t,)) for t in range(16)]
[x.start() for x in th]; [x.join() for x in th]
print(ok[0],len(bad),bad[:3])' 18375 "$(vec JV_RS_GOOD)" "$(vec JV_ES_GOOD)")
set -- $r
[ "${1:-0}" -gt 200 ] && [ "${2:-1}" = "0" ] || err "workers: sixteen clients beside a refresh a second: $r"
n=$(grep -c 'jwks keys unchanged' "$L/workersjk.err")
[ "$n" -ge 2 ] || err "workers: the keys were refreshed $n times in 3.5 s, not every second"
grep -q 'jwks refresh failed' "$L/workersjk.err" && err "workers: a refresh forked from a worker failed: $(grep 'jwks refresh failed' "$L/workersjk.err" | head -2)"

# A session exists once initialize has succeeded (3.4.15): a refused one gets none.
h=$(curl -s -D - -o /dev/null "${H[@]}" -d '{"method":"initialize"' http://127.0.0.1:18392/mcp)
echo "$h" | grep -qi '^mcp-session-id:' && err "a refused initialize was given a session: $h"
h=$(curl -s -D - -o /dev/null "${H[@]}" -d "$INIT" http://127.0.0.1:18392/mcp)
echo "$h" | grep -qi '^mcp-session-id:' || err "initialize got no session: $h"
# A Content-Length that is no number, and a NUL hiding bytes (3.4.15).
c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H 'Content-Length: abc' -d "$INIT" http://127.0.0.1:18390/mcp)
[ "$c" = "400" ] || err "a malformed Content-Length got $c, not 400"
r=$(python3 -I -c 'import socket,sys
b=sys.argv[1].encode()+b"\0garbage"
s=socket.create_connection(("127.0.0.1",18390),timeout=3)
s.sendall(b"POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: %d\r\nConnection: close\r\n\r\n"%len(b)+b)
d=b""
while 1:
    c=s.recv(4096)
    if not c: break
    d+=c
print(d.split(b"\r\n")[0].decode())' "$INIT")
echo "$r" | grep -q ' 400 ' || err "a NUL in a body was not refused: $r"
# A NUL in a header, on each HTTP-family mode, ws included: RFC 9110 5.5 (3.6.5).
for p in 18390 18391 18392 18393; do
  r=$(python3 -I -c 'import socket,sys
s=socket.create_connection(("127.0.0.1",int(sys.argv[1])),timeout=3)
s.sendall(b"GET /mcp HTTP/1.1\r\nHost: x\r\nUpgrade: websocket\r\nConnection: Upgrade\0" + b"x"*20000 + b"\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n")
d=b""
while 1:
    c=s.recv(4096)
    if not c: break
    d+=c
print(d.split(b"\r\n")[0].decode())' "$p")
  echo "$r" | grep -q ' 400 ' || err "$p: a NUL in a header was not refused: $r"
done
# MCP 2026-07-28 over stdio, and a legacy client on the same binary (3.5.0).
MM='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}'
r=$(printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server/discover\",\"params\":{$MM}}" "$INIT" "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/list\",\"params\":{$MM}}" \
  | timeout 5 $B stdio 2>/dev/null)
echo "$r" | sed -n 1p | grep -q '"supportedVersions":\["2026-07-28"' || err "server/discover over stdio: $r"
echo "$r" | sed -n 2p | grep -q '"protocolVersion":"2025-11-25"' || err "initialize beside a modern request: $r"
echo "$r" | sed -n 3p | grep -q '"resultType":"complete"' || err "modern tools/list over stdio: $r"
# What a root can hold (3.4.15): a tool stays in it through a symlink, and a FIFO
# cannot hang the server.
fsr="$L/fsroot"; fso="$PWD/$L/outside"; rm -rf "$fsr" "$fso"; mkdir -p "$fsr" "$fso"
echo secret-bytes > "$fso/secret"
ln -s "$fso" "$fsr/dlink"; ln -s "$fso/secret" "$fsr/flink"; ln -s "$fso/new" "$fsr/dangling"; mkfifo "$fsr/pipe"
fscall() { # tool json-args
  printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"$1\",\"arguments\":$2}}" \
    | BOTE_FS_ROOT="$fsr" timeout 5 $B stdio 2>/dev/null
}
r=$(fscall fs_read '{"path":"dlink/secret"}'); echo "$r" | grep -q '"isError":true' || err "fs_read went through a directory symlink: $r"
r=$(fscall fs_read '{"path":"flink"}'); echo "$r" | grep -q '"isError":true' || err "fs_read went through a file symlink: $r"
r=$(fscall fs_write '{"path":"dlink/planted","content":"x"}'); echo "$r" | grep -q '"isError":true' || err "fs_write went through a directory symlink: $r"
[ -e "$fso/planted" ] && err "fs_write planted a file outside the root"
fscall fs_write '{"path":"flink","content":"overwritten"}' > /dev/null
grep -q secret-bytes "$fso/secret" || err "fs_write wrote through a file symlink"
fscall fs_write '{"path":"dangling","content":"x"}' > /dev/null
[ -e "$fso/new" ] && err "fs_write created a file through a dangling symlink"
r=$(fscall fs_mkdir '{"path":"dlink/made"}'); echo "$r" | grep -q '"isError":true' || err "fs_mkdir went through a symlink: $r"
[ -e "$fso/made" ] && err "fs_mkdir created a directory outside the root"
r=$(fscall fs_read '{"path":"pipe"}'); [ -n "$r" ] || err "fs_read hung on a FIFO"
r=$(fscall fs_write '{"path":"pipe","content":"x"}'); [ -n "$r" ] || err "fs_write hung on a FIFO"
# Bearer auth: a token list starts and is enforced.
BOTE_BEARER_TOKENS=tok-a,tok-b start bearer "$L/bearer.err" $B http 18398
wait_port 18398 || err "bearer: nothing listening on 18398 (did a token list stop it starting?)"
c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -d "$INIT" http://127.0.0.1:18398/mcp)
[ "$c" = "401" ] || err "bearer: no token got $c, not 401"
c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H 'Authorization: Bearer wrong' -d "$INIT" http://127.0.0.1:18398/mcp)
[ "$c" = "401" ] || err "bearer: a wrong token got $c, not 401"
c=$(curl -s -o /dev/null -w '%{http_code}' "${H[@]}" -H 'Authorization: Bearer tok-b' -d "$INIT" http://127.0.0.1:18398/mcp)
[ "$c" = "200" ] || err "bearer: a right token got $c, not 200"
# Bearer auth asked for and not configured is a refusal to start, not auth off.
rc=0; BOTE_BEARER_TOKENS=, timeout 5 $B http 18397 2> "$L/nobearer.held" || rc=$?
[ "$rc" -eq 2 ] || err "BOTE_BEARER_TOKENS=, started anyway (rc=$rc)"
# web_search makes its request in a child (3.6.5): against a loopback SearXNG, its
# results, a status, and a server that never answers, which the deadline ends.
sx="$L/searx"; mkdir -p "$sx"
printf '%s' '{"results":[{"title":"Bote smoke","url":"https://example.org/a","content":"a snippet"}]}' > "$sx/search"
python3 -I -m http.server 18380 --bind 127.0.0.1 --directory "$sx" > /dev/null 2>&1 & pids+=($!)
python3 -I -c 'import socket
s=socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1); s.bind(("127.0.0.1", 18381)); s.listen(8)
held=[]
while 1:
    held.append(s.accept()[0])' > /dev/null 2>&1 & pids+=($!)
wait_port 18380 || err "searx: the loopback server did not start"
wait_port 18381 || err "searx: the silent server did not start"
wscall() { # base env...
  local base="$1"; shift
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"web_search","arguments":{"query":"bote"}}}' \
    | env BOTE_SEARXNG_URL="$base" "$@" timeout 10 $B stdio 2>> "$L/searx.held"
}
r=$(wscall http://127.0.0.1:18380)
echo "$r" | grep -q 'Bote smoke' && echo "$r" | grep -q 'a snippet' || err "web_search through a child: $r"
[ "$(printf '%s\n' "$r" | wc -l)" -eq 1 ] || err "web_search's child wrote to the wire: $r"
r=$(wscall http://127.0.0.1:18380/nope)
echo "$r" | grep -q 'SearXNG returned HTTP 404' || err "web_search on a 404: $r"
t0=$(date +%s%N)
r=$(wscall http://127.0.0.1:18381 BOTE_WEB_TIMEOUT_MS=500)
ms=$(( ($(date +%s%N) - t0) / 1000000 ))
echo "$r" | grep -q 'could not reach the SearXNG endpoint' || err "web_search on a silent server: $r"
[ "$ms" -lt 3000 ] || err "web_search on a silent server took $ms ms, past its 500 ms deadline"

# subscriptions/listen (3.7.2): on http, bridge and streamable a listen opens an event stream
# whose first message is the acknowledgement, stamped with the request's id; a keepalive keeps
# it, and a client that goes is gone from the server's count. stdio and ws have no such method.
LB="{\"jsonrpc\":\"2.0\",\"id\":\"s1\",\"method\":\"subscriptions/listen\",\"params\":{$MD,\"notifications\":{\"toolsListChanged\":true}}}"
LACK='"method":"notifications/subscriptions/acknowledged","params":{"notifications":{"toolsListChanged":true},"_meta":{"io.modelcontextprotocol/subscriptionId":"s1"}}'
for m in "18390 /mcp" "18391 /" "18392 /mcp"; do
  read -r p path <<< "$m"
  r=$(curl -s -N -D - --max-time 1 "${MV[@]}" -H 'Mcp-Method: subscriptions/listen' -d "$LB" "http://127.0.0.1:$p$path")
  echo "$r" | grep -qi '^content-type: text/event-stream' && echo "$r" | grep -qF "$LACK" || err "$p listen: no acknowledgement on an event stream: $r"
done
BOTE_LOG=debug BOTE_LISTEN_KEEPALIVE_MS=200 start listen "$L/listen.err" $B http 18372
wait_port 18372 || err "listen: nothing listening on 18372"
r=$(curl -s -N --max-time 1 "${MV[@]}" -H 'Mcp-Method: subscriptions/listen' -d "$LB" http://127.0.0.1:18372/mcp)
[ "$(echo "$r" | grep -c '^: keepalive')" -ge 3 ] || err "listen: fewer than three keepalives in a second at 200 ms: $r"
for _ in $(seq 1 20); do grep -q 'listen stream closed reason=gone open=0' "$L/listen.err" && break; sleep 0.1; done
grep -q 'listen streams max=256 keepalive_ms=200' "$L/listen.err" && grep -q 'listen stream opened honored=1 open=1' "$L/listen.err" \
  && grep -q 'listen stream closed reason=gone open=0' "$L/listen.err" || err "listen: the log does not show a stream opened and its client gone: $(grep listen "$L/listen.err")"
# Where the connection is the stream (3.7.3): the acknowledgement is the first message on it,
# a cancel ends the subscription, and the connection goes on serving.
LC='{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"s1"}}'
LPING='{"jsonrpc":"2.0","id":2,"method":"ping"}'
r=$(printf '%s\n%s\n%s\n' "$LB" "$LC" "$LPING" | BOTE_LOG=debug timeout 5 $B stdio 2> "$L/stdio-listen.held")
echo "$r" | sed -n 1p | grep -qF "$LACK" && echo "$r" | sed -n 2p | grep -q '"id":2,"result":{}' \
  || err "listen over stdio: not the acknowledgement, then the ping's reply: $r"
grep -q 'listen subscription ended reason=cancelled open=0' "$L/stdio-listen.held" || err "listen over stdio: the cancel did not end it: $(grep listen "$L/stdio-listen.held")"
# BOTE_LISTEN_MAX is held to its bound: each slot is made at startup.
printf '%s\n' "$LPING" | BOTE_LISTEN_MAX=99999999 timeout 5 $B stdio > /dev/null 2> "$L/listen-max.held"
grep -q 'BOTE_LISTEN_MAX past its bound; holding it there max=99999999 bound=65536' "$L/listen-max.held" && grep -q 'listen subscriptions max=65536' "$L/listen-max.held" \
  || err "listen: BOTE_LISTEN_MAX not held to its bound: $(cat "$L/listen-max.held")"
r=$(python3 -I -c 'import socket,sys
s=socket.socket(socket.AF_UNIX); s.settimeout(3); s.connect(sys.argv[1])
s.sendall(sys.argv[2].encode()+b"\n"+sys.argv[3].encode()+b"\n"+sys.argv[4].encode()+b"\n"); b=b""
while b.count(b"\n")<2:
    c=s.recv(4096)
    if not c: break
    b+=c
print(b.decode().strip())' "$sock" "$LB" "$LC" "$LPING")
echo "$r" | sed -n 1p | grep -qF "$LACK" && echo "$r" | sed -n 2p | grep -q '"id":2,"result":{}' \
  || err "listen over unix: not the acknowledgement, then the ping's reply: $r"
r=$(python3 -I -c 'import base64,os,socket,struct,sys
s=socket.create_connection(("127.0.0.1",18393),3); s.settimeout(3)
k=base64.b64encode(os.urandom(16)).decode()
s.sendall(("GET /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: %s\r\n\r\n"%k).encode())
b=b""
while b"\r\n\r\n" not in b: b+=s.recv(4096)
b=b.split(b"\r\n\r\n",1)[1]
p=sys.argv[1].encode(); m=os.urandom(4)
h=b"\x81"+(bytes([128|len(p)]) if len(p)<126 else bytes([254])+struct.pack(">H",len(p)))
s.sendall(h+m+bytes(c^m[i%4] for i,c in enumerate(p)))
while len(b)<4: b+=s.recv(4096)
n=b[1]&127; o=2
if n==126: n=struct.unpack(">H",b[2:4])[0]; o=4
while len(b)<o+n: b+=s.recv(4096)
print(b[o:o+n].decode())' "$LB")
echo "$r" | grep -qF "$LACK" || err "listen over ws: no acknowledgement: $r"

# BOTE_STATE_KEY (3.8.2): a key that is no key ends the start with a line, and never the
# key itself; a good one serves.
REQ='{"jsonrpc":"2.0","id":1,"method":"ping"}'
for bad in zz 0123 "$(printf '%063d' 0)"; do
  rc=0; printf '%s\n' "$REQ" | BOTE_STATE_KEY="$bad" "$B" > "$L/key.out" 2> "$L/key.err" || rc=$?
  [ "$rc" -eq 2 ] || err "BOTE_STATE_KEY=$bad: rc=$rc, not 2"
  grep -q 'BOTE_STATE_KEY: 64 hex digits or more' "$L/key.err" || err "BOTE_STATE_KEY=$bad: no line saying why"
  [ -s "$L/key.out" ] && err "BOTE_STATE_KEY=$bad: answered anyway"
done
good=$(printf '%064x' 7)
printf '%s\n' "$REQ" | BOTE_STATE_KEY="$good" "$B" > "$L/key.out" 2> "$L/key.err" || true
grep -q '"id":1,"result":{}' "$L/key.out" || err "BOTE_STATE_KEY with a good key: no answer: $(cat "$L/key.out" "$L/key.err")"
grep -q "$good" "$L/key.err" && err "BOTE_STATE_KEY: the key reached the log"

for f in "$L"/*.err; do grep -q 'transport failed' "$f" && err "$(basename "$f"): $(grep 'transport failed' "$f")"; done
[ $fail -eq 0 ] || exit 1
echo "transport smoke: http bridge streamable ws unix + both argv[0] aliases answer; streamable keeps its tool set; a held port names its step; BOTE_STATE_KEY refused aloud or taken"
