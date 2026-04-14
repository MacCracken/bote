# Proposal: `lib/ws_server.cyr` for cyrius stdlib

> **Status**: design + reference implementation, ready for cyrius lang-agent
> to integrate into stdlib. Companion to the existing `lib/ws.cyr` (client)
> and `lib/http_server.cyr` (HTTP — landed in 4.5.0).
>
> **Why**: every cyrius project that wants WebSocket server support is
> currently stuck. `lib/ws.cyr` is client-only (sends MASKED frames; a
> server needs to *receive* masked frames and *send* unmasked ones —
> nontrivial inversion). Bote v1.5+ needs server-side WS for the MCP
> WebSocket transport. Same shape will be useful to majra (push events
> over WS), vidya (live API), and any future RPC service.
>
> **Design**: integrates with `lib/http_server.cyr`. The HTTP handler
> detects a WS upgrade request and calls `ws_server_handshake(cfd, buf, n)`
> to upgrade in place. The WS read/write loop runs inside the same handler
> invocation — when it returns, `http_server_run` closes the cfd as it
> would for any HTTP request.

---

## Why each consumer would otherwise hand-roll

| Need | LOC |
|---|---|
| Parse HTTP Upgrade request, validate `Sec-WebSocket-*` headers | ~50 |
| Compute `Sec-WebSocket-Accept = base64(sha1(key + magic))` | ~80 (SHA-1) + ~10 |
| Send 101 Switching Protocols response | ~20 |
| Read a frame: FIN/opcode, MASK + length (1/16/64-bit), 4-byte mask key, unmask payload | ~80 |
| Write a frame: FIN/opcode, length encoding, unmasked payload | ~50 |
| Ping / pong / close-handshake | ~30 |
| **Total per consumer** | **~320 LOC** |

---

## Surface

### Server-side handshake

```cyr
# Parse HTTP request as a WebSocket upgrade. Validates:
#   - Upgrade: websocket
#   - Connection: Upgrade
#   - Sec-WebSocket-Version: 13
#   - Sec-WebSocket-Key present
# Computes Sec-WebSocket-Accept and sends the 101 Switching Protocols
# response on cfd. Returns an upgraded ws handle (24 bytes) on success,
# or 0 if the request isn't a valid WS upgrade (caller should send 400).
fn ws_server_handshake(cfd, req_buf, req_len) → ws_handle | 0
```

### Frame I/O (server side: read MASKED, write UNMASKED)

```cyr
# Read one frame. Unmasks payload in place. Writes opcode to *opcode_out
# and returns the payload length (or 0 - 1 on socket error / close).
# Caller passes a buffer for the payload; max payload size = max - 14.
fn ws_server_recv_frame(ws, payload_buf, max, opcode_out) → len

# Send one frame. Server-to-client frames are NOT masked per RFC 6455.
fn ws_server_send_frame(ws, opcode, data, len) → 0

# High-level helpers — handle control frames (ping/pong/close) automatically.
# ws_server_recv blocks until a TEXT or BINARY frame arrives, returns the
# payload as a cstr (alloc'd) — or 0 if the peer closed the connection.
fn ws_server_recv(ws) → cstr | 0
fn ws_server_send_text(ws, msg) → 0
fn ws_server_send_binary(ws, data, len) → 0
fn ws_server_send_ping(ws) → 0
fn ws_server_send_pong(ws, data, len) → 0
fn ws_server_send_close(ws, code, reason) → 0   # code 0 = normal; reason can be ""
fn ws_server_close(ws) → 0                       # full close handshake + sock_close
```

### Opcodes (re-exported from existing lib/ws.cyr conventions)

```cyr
# WS_OP_CONTINUATION = 0
# WS_OP_TEXT         = 1
# WS_OP_BINARY       = 2
# WS_OP_CLOSE        = 8
# WS_OP_PING         = 9
# WS_OP_PONG         = 10
```

### Handle layout

```cyr
# 24 bytes:
#   +0  fd        socket file descriptor
#   +8  state     WS_OPEN | WS_CLOSING | WS_CLOSED
#   +16 partial   in-progress message buffer (for fragmented messages, or 0)
```

---

## Integration with `lib/http_server.cyr`

This is the key design point. Inside an HTTP handler:

```cyr
fn my_handler(ctx, cfd, buf, n) {
    var path = http_get_path(buf, n);
    if (streq(http_path_only(path), "/ws") == 1) {
        var ws = ws_server_handshake(cfd, buf, n);
        if (ws == 0) {
            http_send_status(cfd, HTTP_BAD_REQUEST, "Bad Request");
            return 0;
        }
        # The 101 has already been sent. Run the WS lifetime here.
        while (1 == 1) {
            var msg = ws_server_recv(ws);
            if (msg == 0) { return 0; }   # peer closed
            ws_server_send_text(ws, msg);  # echo
        }
    }
    # ... regular HTTP routes ...
}
```

When the handler returns, `http_server_run` calls `sock_close(cfd)` —
which closes the (already-half-closed) WS socket cleanly. **No changes
needed in `http_server.cyr`.**

---

## Reference implementation

Lang-agent: this is ready to drop into `lib/ws_server.cyr`. Includes a
self-contained SHA-1 (~85 LOC) — the algorithm is small, only used for
the WS handshake, and avoids growing `lib/sigil.cyr` for one consumer.

The implementation lives in `docs/proposals/lib_ws_server.cyr` alongside
this doc.

---

## Adoption — what changes in bote (v1.5+)

A new `src/transport_ws.cyr`:

```cyr
# bote::transport::ws — MCP over WebSocket
# Built on lib/http_server.cyr (HTTP routing) + lib/ws_server.cyr (WS upgrade + frames).

fn _bote_ws_handler(ctx, cfd, buf, n) {
    var config = ctx;     # carries dispatcher + path
    var path = http_get_path(buf, n);
    if (streq(http_path_only(path), ws_config_path(config)) == 0) {
        http_send_status(cfd, HTTP_NOT_FOUND, "Not Found");
        return 0;
    }
    var ws = ws_server_handshake(cfd, buf, n);
    if (ws == 0) {
        http_send_status(cfd, HTTP_BAD_REQUEST, "Bad Request");
        return 0;
    }
    # Each WS message is one JSON-RPC line.
    while (1 == 1) {
        var msg = ws_server_recv(ws);
        if (msg == 0) { return 0; }
        var resp = codec_process_message(msg, ws_config_dispatcher(config));
        if (resp != 0) {
            ws_server_send_text(ws, resp);
        }
    }
    return 0;
}

fn transport_ws_run(dispatcher, config) {
    ws_config_with_dispatcher(config, dispatcher);
    return http_server_run(ws_config_addr(config),
                           ws_config_port(config),
                           &_bote_ws_handler,
                           config);
}
```

**Total bote-side LOC**: ~80 (vs ~400 if hand-rolled). Same shape as
the v1.3 transport_http refactor: bote contributes the MCP-specific
wire-up; the WS protocol guts live in stdlib.

---

## Adoption — what changes in majra (future)

Majra has push semantics — pubsub publishes get fanned out to subscribers.
Today subscribers connect via TCP/IPC. With `lib/ws_server.cyr`, browsers
and other WS clients can subscribe directly:

```cyr
fn _majra_ws_handler(pubsub, cfd, buf, n) {
    var path = http_get_path(buf, n);
    var topic = http_get_param(path, "topic");    # ?topic=foo
    if (topic == 0) { http_send_status(cfd, HTTP_BAD_REQUEST, "..."); return 0; }
    var ws = ws_server_handshake(cfd, buf, n);
    if (ws == 0) { http_send_status(cfd, HTTP_BAD_REQUEST, "..."); return 0; }
    var rx = pubsub_subscribe(pubsub, topic);
    while (1 == 1) {
        var msg = chan_recv(rx);
        if (msg == 0) { return 0; }
        ws_server_send_text(ws, msg);
    }
    return 0;
}
```

---

## Spec compliance (RFC 6455)

| Requirement | Coverage |
|---|---|
| HTTP/1.1 Upgrade handshake | ✅ |
| `Sec-WebSocket-Key` validation + `Sec-WebSocket-Accept` reply | ✅ |
| `Sec-WebSocket-Version: 13` enforced | ✅ |
| Frame format (FIN, opcode, MASK, payload length encoding) | ✅ |
| Server reads MASKED client frames, writes UNMASKED frames | ✅ |
| Text + Binary data frames | ✅ |
| Continuation frames (fragmented messages) | ✅ via `partial` slot in handle |
| Ping / Pong control frames | ✅ |
| Close handshake (opcode 0x8 + 2-byte status code + optional reason) | ✅ |
| Max frame payload (default 1MB; configurable) | ✅ via `WS_MAX_PAYLOAD` |
| Per-message deflate (RFC 7692) | ❌ deferred |
| Subprotocol negotiation (`Sec-WebSocket-Protocol`) | 🟡 read but not enforced; consumer can inspect |

---

## Open questions for cyrius lang-agent

1. ~~**Move SHA-1 into `lib/sigil.cyr`?**~~ **Resolved: no.** sigil is deliberately scoped to trust primitives (signing, SHA-256, ed25519, policy). SHA-1 is only needed here for the RFC 6455 handshake — it isn't a general-purpose hash the rest of the stdlib should grow a dependency on. Keep `_wss_sha1_*` inlined in `lib/ws_server.cyr`; sigil stays trust-only.

2. **Naming consistency**: existing client API is `ws_send_text`/`ws_recv` etc. Server API is `ws_server_send_text`/`ws_server_recv` to disambiguate. Alternative: rename existing to `ws_client_*`. Backward-compat hit isn't worth it IMO.

3. **Async/select**: this proposal is fully blocking — `ws_server_recv` blocks until a frame arrives. Eventually we'll want non-blocking + multiplex. Out of scope for 4.5.x; needed when streaming dispatch lands.

4. **Per-message deflate**: significant code (LZ77 + Huffman). Skipped for this round; the stdlib's `lib/dynlib.cyr` could host a future zlib binding.

5. **Frame size cap**: hardcoded `WS_MAX_PAYLOAD = 1048576` (1MB). Could be a config var on the handle if any consumer needs different bounds.

---

## Test plan

1. New stdlib tests `lib/ws_server.tcyr` cover:
   - SHA-1 known vectors (empty, "abc", longer)
   - `Sec-WebSocket-Accept` for the RFC 6455 example (`dGhlIHNhbXBsZSBub25jZQ==` → `s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`)
   - Frame parsing: small (≤125), medium (16-bit length), large (64-bit length)
   - Mask XOR round-trip
   - Control frames (ping/pong/close) handled internally by `ws_server_recv`
   - Continuation frames assembled correctly
2. Bote `transport_ws.cyr` end-to-end: connect via `wscat` → send JSON-RPC → receive response.
3. Echo server in `docs/proposals/lib_ws_server_example.cyr` runnable + verifiable against `wscat -c ws://localhost:8080`.

---

## Summary

| | Before | After |
|---|---|---|
| Lines of WS server plumbing per consumer | ~320 (bote, majra, vidya would each duplicate) | **0** |
| `lib/ws_server.cyr` size | n/a | **~480 LOC** (incl. inline SHA-1) |
| WS server availability | nowhere | every cyrius project |
| Bote `transport_ws.cyr` (v1.5+) | hand-rolled ~400 LOC | **~80 LOC** wrapper |
| RFC 6455 spec coverage | n/a | core (data frames + control + fragmentation + close handshake) |
