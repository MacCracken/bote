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
#     their root through a symlink and do not hang on a FIFO (3.4.15).
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

for f in "$L"/*.err; do grep -q 'transport failed' "$f" && err "$(basename "$f"): $(grep 'transport failed' "$f")"; done
[ $fail -eq 0 ] || exit 1
echo "transport smoke: http bridge streamable ws unix + both argv[0] aliases answer; streamable keeps its tool set; a held port names its step"
