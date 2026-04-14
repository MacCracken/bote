# Bote Architecture (Cyrius)

> MCP core service — JSON-RPC 2.0 protocol, tool registry, dispatch, six
> transports, bearer-token middleware, libro audit tools, typed content
> blocks, host registry with SSRF guard.
>
> **Name**: Bote (German) — messenger.
>
> **Lineage**: Originally a Rust crate. Ported to Cyrius via `cyrius port`
> on 2026-04-13 (v1.0.0). The Rust archive was retired in v1.0.1; the last
> Rust snapshot is at git tag `0.92.0`. This doc describes the live Cyrius
> implementation (current: **1.9.2**, cyrius 4.7.0).

---

## Design Principles

1. **One protocol implementation** — every consumer dispatches through bote instead of reimplementing JSON-RPC 2.0.
2. **Registry-driven** — tools registered with schemas, dispatch validates automatically.
3. **Transport-agnostic** — same `Dispatcher` powers six transports.
4. **Streaming-ready data layer** — progress + cancellation primitives in place; threaded dispatch deferred until cyrius's thread/async surface firms up.
5. **Audit + events as fn-pointer + ctx adapters** — libro and majra wired today; any other backend drops in via the same shape.
6. **Auth as opt-in middleware** — bearer-token validator is a fn-pointer slot on each HTTP-family transport config; unset = no overhead, no behavior change.
7. **No global state in the dispatcher** — caller owns the registry and dispatcher heap pointers; transports are per-instance.
8. **Defense in depth at every input boundary** — Content-Length clamping, SSRF guard, Origin allowlist, MCP-Protocol-Version validation, session ID validation, malformed-input fuzz coverage.

---

## System

```
┌────────────────────────────────────────────────────────────────────┐
│ Consumers (jalwa, shruti, tazama, daimon, agnoshi, …) +            │
│ TS clients (via bridge) + browser clients (via streamable / WS)    │
│                                                                    │
│ Client: JSON-RPC 2.0 over stdio / HTTP / Unix / streamable / WS    │
└──────────────────────────────┬─────────────────────────────────────┘
                               │
┌──────────────────────────────▼─────────────────────────────────────┐
│ Bote (Cyrius)                                                      │
│                                                                    │
│ ┌────────────────────────────────────────────────────────────────┐ │
│ │ Transport Layer (six transports — same Dispatcher backs all)   │ │
│ │                                                                │ │
│ │   stdio    HTTP/1.1     unix       bridge    streamable    ws  │ │
│ │                                                                │ │
│ │ HTTP-family middleware (per-request, in order):                │ │
│ │   1. Origin allowlist                                          │ │
│ │   2. Bearer-token validator (auth.cyr) — opt-in                │ │
│ │   3. MCP-Protocol-Version                                      │ │
│ │   4. MCP-Session-Id (auto-bypass for initialize)               │ │
│ │   5. Content-Length clamp (cap to bytes received)              │ │
│ └────────────────────────┬───────────────────────────────────────┘ │
│                          │                                         │
│ ┌────────────────────────▼───────────────────────────────────────┐ │
│ │ codec — parse_request / serialize_response /                   │ │
│ │         process_message (single + batch + notification)        │ │
│ └────────────────────────┬───────────────────────────────────────┘ │
│                          │                                         │
│ ┌──────────────┐ ┌───────▼────────┐ ┌────────────────────────────┐ │
│ │ registry     │ │ dispatch       │ │ stream                     │ │
│ │ (ToolDef +   │─│ (initialize /  │─│ (ProgressUpdate,           │ │
│ │  schemas +   │ │ tools/list /   │ │  CancellationToken,        │ │
│ │  versions)   │ │ tools/call)    │ │  progress notifications)   │ │
│ └──────┬───────┘ └───────┬────────┘ └────────────────────────────┘ │
│        │                 │                                         │
│ ┌──────▼─────────────────▼────────────────────────────────────────┐│
│ │ schema (CompiledSchema: type / enum / bounds / nested)          ││
│ └─────────────────────────────────────────────────────────────────┘│
│                                                                    │
│ ┌────────────────────────────────────────────────────────────────┐ │
│ │ Sinks (fn-pointer + ctx — adapters drop into the same shape)   │ │
│ │   audit  ──> audit_libro  ──> libro chain                      │ │
│ │   events ──> events_majra ──> majra pubsub                     │ │
│ └────────────────────────────────────────────────────────────────┘ │
│                                                                    │
│ ┌────────────────────────────────────────────────────────────────┐ │
│ │ Built-in tool surface (registered by main.cyr at startup)      │ │
│ │   bote_echo                                                    │ │
│ │   libro_query  libro_verify  libro_export                      │ │
│ │   libro_proof  libro_retention                                 │ │
│ └────────────────────────────────────────────────────────────────┘ │
│                                                                    │
│ ┌────────────────────────────────────────────────────────────────┐ │
│ │ Outbound-call utilities                                        │ │
│ │   content (typed MCP blocks: text/image/audio/resource/blob)   │ │
│ │   host    (HostRegistry + ssrf_check before any URL fetch)     │ │
│ └────────────────────────────────────────────────────────────────┘ │
│                                                                    │
│ ┌────────────────────────────────────────────────────────────────┐ │
│ │ jsonx  — nested-aware JSON value extractor                     │ │
│ │ session — SessionStore, validate_origin, validate_protocol     │ │
│ │ error  — BoteErrTag, rpc_code mapping, format                  │ │
│ │ auth   — bearer-token middleware (RFC 6750)                    │ │
│ └────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────┘
```

---

## Module Layout

```
src/
├── main.cyr                — CLI entry: argv selects transport,
│                             BOTE_BEARER_TOKENS env-wires auth allowlist
├── error.cyr               — BoteErrTag (12 variants), rpc_code, format
├── protocol.cyr            — JsonRpcRequest / Response / Error data types
├── jsonx.cyr               — nested-aware JSON extractor
├── registry.cyr            — ToolDef, ToolSchema, ToolAnnotations, ToolRegistry
├── dispatch.cyr            — Dispatcher + dispatcher_dispatch
├── codec.cyr               — parse_request / serialize_response / process_message
├── schema.cyr              — CompiledSchema typed validation
├── stream.cyr              — progress + cancellation primitives
├── session.cyr             — SessionStore, protocol/origin validators
├── discovery.cyr           — Cross-node tool discovery
├── audit.cyr               — AuditSink (fn-ptr + ctx)
├── audit_libro.cyr         — LibroAudit adapter
├── events.cyr              — EventSink (fn-ptr + ctx) + topic constants
├── events_majra.cyr        — MajraEvents adapter
├── auth.cyr                — Bearer-token middleware (RFC 6750)
├── content.cyr             — Typed MCP content blocks
├── host.cyr                — HostRegistry + SSRF guard (IPv4 + IPv6)
├── libro_tools.cyr         — Five built-in MCP tools over a libro chain
├── transport_stdio.cyr     — line-oriented stdin/stdout loop
├── transport_http.cyr      — HTTP/1.1 server + middleware
├── transport_unix.cyr      — AF_UNIX line-oriented loop
├── bridge.cyr              — TS-client bridge with CORS
├── transport_streamable.cyr — Streamable HTTP / SSE (MCP 2025-11-25)
└── transport_ws.cyr        — WebSocket (RFC 6455)

lib/                        — vendored cyrius stdlib
                             (used: alloc, args, base64, chrono, fmt,
                              fnptr, hashmap, http_server, io, json, net,
                              str, string, syscalls, tagged, vec,
                              ws_server, freelist, thread, sigil, …)
[deps.libro]   git = "MacCracken/libro"   tag = "1.0.3"
[deps.majra]   git = "MacCracken/majra"   tag = "2.2.0"

tests/
├── bote.tcyr                  — 394 core assertions
├── bote_libro_tools.tcyr      — 22 (libro_tools)
├── bote_content.tcyr          — 18 (content blocks)
├── bote_host.tcyr             — 56 (host registry + SSRF)
├── bote_auth.tcyr             — 29 (bearer middleware)
└── bote.bcyr                  — 10 hot-path benchmarks

fuzz/
├── codec_parse.fcyr
├── codec_process.fcyr
├── jsonx_extract.fcyr
└── schema_validate.fcyr

docs/
├── architecture/overview.md   — this file
├── benchmarks-rust-v-cyrius.md
├── cyrius-feedback.md         — language issues found during the port
├── spec-compliance.md         — MCP 2025-11-25 conformance matrix
├── development/roadmap.md     — shipped per release, remaining for 2.0
├── bugs/                      — cyrius bug reports w/ reproducers
└── proposals/                 — stdlib proposals (http_server, ws_server)
```

The per-module test-file split (five `tests/bote_*.tcyr` files) is a
deliberate organization choice — it mirrors `src/` layout and makes
per-module compile times tight. (It originated as a workaround for the
cyrius 4.5.1 parser identifier-buffer cap; that cap has since been
raised in 4.6.2 and 4.7.x, but the layout reads cleanly enough that we
kept it.)

---

## Data Representation Conventions

Cyrius is i64-only (no floats, no generics, no traits, no closures).
Bote follows these conventions throughout:

| Pattern | Example |
|---|---|
| Structs are heap-alloc'd byte ranges with fixed offsets | `var d = alloc(48); store64(d + 8, name);` |
| Accessors are `module_field(ptr) → load64(ptr + offset)` | `tool_def_name(d) → load64(d)` |
| Optional fields use `0` as the sentinel | `tool_def_version(d) == 0` means no version |
| Tagged enums = i64 tag at offset 0 | `bote_err_tag(e) == ERR_INVALID_PARAMS` |
| Lists are `vec_*` from `lib/vec.cyr`; maps are `map_*` from `lib/hashmap.cyr` | |
| Strings are NUL-terminated cstrs unless a libro/majra boundary calls for `Str` (fat string from `lib/str.cyr`) | wrap with `str_from(cstr)` at the boundary |
| Function pointers via `&fn_name`, called with `fncall1(fp, arg)` / `fncall2(fp, a, b)` | adapters: `audit`, `events`, `auth` |

JSON-RPC `id`, `params`, `result`, and error `data` are stored as **raw
JSON-literal cstrs** (e.g. `"1"`, `"\"abc\""`, `"null"`, `"{...}"`)
since cyrius has no nested JSON value type. The `jsonx` module extracts
subtrees by slicing the source bytes (respecting nested braces,
brackets, and quoted strings).

### Adapter pattern (for sinks + auth)

Cyrius has no traits. Bote uses **fn-pointer + ctx void\*** adapters
everywhere a backend swap is needed:

```cyr
# AuditSink (24 bytes): { fp, ctx, _reserved }
# Caller wires with: audit_sink_new(&libro_audit_log, libro_chain_handle)
# Dispatcher invokes via: fncall2(fp, ctx, event_ptr)

# Bearer validator: fn(token_cstr, ctx) -> claims | 0
# http_config_with_bearer_validator(cfg, &my_validator, &my_token_store)
```

The same shape covers `audit_libro`, `events_majra`, and the
`auth_validator_*` family. New backends drop in without touching
dispatch.

---

## Error Codes

| Code | Meaning |
|---|---|
| -32700 | Parse error |
| -32600 | Invalid request (bad jsonrpc version, empty batch, non-object element) |
| -32601 | Method not found / tool not found |
| -32602 | Invalid params (missing required, schema violation, empty tool name) |
| -32000 | Tool execution error / sandbox error |
| -32003 | Transport closed / bind failed |
| -32603 | Internal error |
| -32800 | Request cancelled |

Maintained in `src/error.cyr::bote_err_rpc_code`.

---

## Transports

### stdio
Line-oriented JSON-RPC over fd 0 / fd 1. Reads chunks from stdin,
splits on `\n`, dispatches each complete line, leaves partial lines in
a 128 KB heap-allocated buffer. EOF flushes any final non-terminated
line.

### HTTP/1.1
Own minimal server (no `axum` equivalent in cyrius stdlib). Bind to
`127.0.0.1:port`, accept-loop, single-recv per connection. Routes:

| Method/Path | Action |
|---|---|
| `POST <endpoint>` (default `/mcp`) | dispatch JSON-RPC |
| Anything else | 404 / 405 |

Middleware (when configured on `HttpConfig`):
- **Origin** allow-list (403 on rejection; wildcard `*` allows all; empty list = strict mode rejects all)
- **Bearer-token validator** — `auth_bearer_check` runs after Origin, before protocol; opt-in via `http_config_with_bearer_validator(cfg, fp, ctx)`. Missing / wrong token → 401 with `WWW-Authenticate: Bearer realm="mcp"`.
- **MCP-Protocol-Version** (400 if invalid; 400 if absent and `require_protocol == 1`)
- **MCP-Session-Id** (404 on unknown SID; auto-bypass for `initialize`)
- **Content-Length clamp** (`clen = min(clen, n - bo)` so a lying header can't make memcpy read past the request buffer; v1.5.1 hardening)

If a `SessionStore` is configured and the request is `initialize`, the
response includes a fresh `MCP-Session-Id` header (32-hex random from
`/dev/urandom`).

### Unix domain socket
Same line-oriented protocol as stdio, but over `AF_UNIX`. Socket file
is unlinked + recreated on bind. Per-connection 128 KB buffer.
Local-only — no auth or origin checks.

### Bridge (TS clients)
HTTP transport with CORS preflight (`OPTIONS /`) and `GET /health`. Tool
results auto-wrapped in MCP envelope (`{"content":[{"type":"text",...}]}`)
unless the handler already returns a `content` array. Same middleware
stack as plain HTTP plus CORS headers on every response.

### Streamable HTTP (MCP 2025-11-25)
Single endpoint (`/mcp`) serves both `POST` (JSON-RPC dispatch, identical
shape to plain HTTP) and `GET` (opens an SSE stream). GET stream:
priming event + retry hint; replays events past `Last-Event-ID` if the
client supplies one; bounded `ResumptionBuffer` (default 1000 events).
Same middleware; `MCP-Protocol-Version` is **required** here per spec
(stricter than plain HTTP's optional default).

### WebSocket (RFC 6455)
Full handshake (`Sec-WebSocket-Accept = base64(sha1(key + magic))`),
masked-client / unmasked-server frame I/O via stdlib `lib/ws_server.cyr`
(landed in cyrius 4.5.1). Each TEXT frame is one JSON-RPC message;
control frames (ping/pong/close) handled transparently. Auth applies
on the upgrade HTTP request only — the connection's identity is fixed
at that point.

---

## Outbound utilities

### content
Typed MCP content blocks for richer-than-text tool results:

| Constructor | Block type |
|---|---|
| `content_text(text)` | `{"type":"text","text":"..."}` |
| `content_image(b64, mime)` / `content_audio(b64, mime)` | binary inline |
| `content_resource(uri, mime, text)` | embedded text resource |
| `content_resource_blob(uri, mime, b64)` | embedded binary resource |
| `content_resource_link(uri, name, mime)` | reference (client fetches by URI) |
| `content_array(blocks)` / `content_array_error(blocks)` | envelope; second sets `isError:true` |
| `content_single(block)` / `content_text_response(text)` | shorthand for single-block case |

`src/bridge.cyr::wrap_tool_result` already passes through a ready-made
content envelope untouched, so handlers opt in without any transport
change.

### host
- `HostRegistry` — name → entry map with `host_entry_with_capabilities` allowlist (fail-open when unset)
- `ssrf_check(url)` — call before any outbound URL fetch. Returns `SSRF_OK` or one of `SSRF_PARSE` / `SSRF_SCHEME` / `SSRF_LOOPBACK` / `SSRF_LINK_LOCAL` / `SSRF_PRIVATE` / `SSRF_METADATA` / `SSRF_UNSPEC` / `SSRF_MULTICAST`. Covers IPv4 + IPv6 literals (bracket form), case-insensitive, strips userinfo, conservative hostname blocklist (`localhost`, `metadata.google.internal`, `metadata`).

---

## Verification

| Artifact | Count | Where |
|---|---|---|
| Core unit tests | 394 | `tests/bote.tcyr` |
| Module tests | 125 | `tests/bote_libro_tools.tcyr` (22) + `bote_content.tcyr` (18) + `bote_host.tcyr` (56) + `bote_auth.tcyr` (29) |
| **Total assertions** | **519** | |
| Benchmarks | 10 | `tests/bote.bcyr` |
| Fuzz harnesses | 4 | `fuzz/*.fcyr` |

All hot paths sub-10 µs on x86_64 (`AMD Ryzen 7 5800H`). Side-by-side
with the Rust v0.92.0 baseline in
[docs/benchmarks-rust-v-cyrius.md](../benchmarks-rust-v-cyrius.md).
