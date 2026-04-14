# Bote Architecture (Cyrius)

> MCP core service — JSON-RPC 2.0 protocol, tool registry, dispatch, three transports.
>
> **Name**: Bote (German) — messenger.
>
> **Lineage**: Originally a Rust crate. Ported to Cyrius via `cyrius port`
> on 2026-04-13 (v1.0.0). The Rust archive was retired in v1.0.1; the last
> Rust snapshot is at git tag `0.92.0`. This doc describes the live Cyrius
> implementation.

---

## Design Principles

1. **One protocol implementation** — every consumer dispatches through bote instead of reimplementing JSON-RPC 2.0.
2. **Registry-driven** — tools registered with schemas, dispatch validates automatically.
3. **Transport-agnostic** — same `Dispatcher` powers stdio, HTTP, Unix sockets.
4. **Streaming-ready data layer** — progress + cancellation primitives in place; thread integration to come with future cyrius features.
5. **Audit-ready hooks** — sites identified for libro/majra integration; modules to land in a future port.
6. **No global state in the dispatcher** — caller owns the registry and dispatcher heap pointers; transports are stateless.

---

## System

```
┌─────────────────────────────────────────────────────────────────┐
│ Consumers (jalwa, shruti, tazama, daimon, agnoshi, …)            │
│                                                                  │
│ Client: JSON-RPC 2.0 over stdio / HTTP / Unix socket             │
└──────────────────────────────┬───────────────────────────────────┘
                               │
┌──────────────────────────────▼───────────────────────────────────┐
│ Bote (Cyrius)                                                    │
│                                                                  │
│ ┌──────────────────────────────────────────────────────────────┐ │
│ │ Transport Layer                                              │ │
│ │ stdio          HTTP/1.1 (own server)        unix (AF_UNIX)   │ │
│ │ +middleware:   Origin / Protocol-Version /                   │ │
│ │                Session-Id                                    │ │
│ └────────────────────────┬─────────────────────────────────────┘ │
│                          │                                       │
│ ┌────────────────────────▼─────────────────────────────────────┐ │
│ │ codec — parse_request / serialize_response /                 │ │
│ │         process_message (single + batch + notif)             │ │
│ └────────────────────────┬─────────────────────────────────────┘ │
│                          │                                       │
│ ┌──────────────┐ ┌───────▼────────┐ ┌──────────────────────────┐ │
│ │ registry     │ │ dispatch       │ │ stream                   │ │
│ │ (ToolDef +   │─│ (initialize /  │─│ (ProgressUpdate,         │ │
│ │  schemas +   │ │ tools/list /   │ │  CancellationToken,      │ │
│ │  versions)   │ │ tools/call)    │ │  progress_notification)  │ │
│ └──────┬───────┘ └───────┬────────┘ └──────────────────────────┘ │
│        │                 │                                       │
│ ┌──────▼─────────────────▼──────────────────────────────────────┐│
│ │ schema (CompiledSchema: type / enum / bounds / nested)        ││
│ └───────────────────────────────────────────────────────────────┘│
│                                                                  │
│ ┌──────────────────────────────────────────────────────────────┐ │
│ │ jsonx — nested-aware JSON value extractor                    │ │
│ │ session — SessionStore, validate_origin, validate_protocol   │ │
│ │ error  — BoteErrTag, rpc_code mapping, format                │ │
│ └──────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

---

## Module Layout

```
src/
├── main.cyr                — CLI entry: argv selects transport
├── error.cyr               — BoteErrTag (12 variants), rpc_code, format
├── protocol.cyr            — JsonRpcRequest / Response / Error data types
├── jsonx.cyr               — nested-aware JSON extractor
├── registry.cyr            — ToolDef, ToolSchema, ToolAnnotations, ToolRegistry
├── dispatch.cyr            — Dispatcher + dispatcher_dispatch
├── codec.cyr               — parse_request / serialize_response / process_message
├── schema.cyr              — CompiledSchema typed validation
├── stream.cyr              — progress + cancellation primitives
├── session.cyr             — SessionStore, protocol/origin validators
├── transport_stdio.cyr     — line-buffered stdin/stdout loop
├── transport_http.cyr      — HTTP/1.1 server + middleware
└── transport_unix.cyr      — AF_UNIX line-buffered loop

lib/                        — vendored cyrius stdlib (47 modules)
tests/
├── bote.tcyr               — 251 unit assertions
└── bote.bcyr               — 10 hot-path benchmarks
fuzz/
├── codec_parse.fcyr
├── codec_process.fcyr
├── jsonx_extract.fcyr
└── schema_validate.fcyr
                            (Rust archive retired in v1.0.1; see git tag 0.92.0)
```

---

## Data Representation Conventions

Cyrius is i64-only (no floats, no generics, no traits). Bote follows these
conventions throughout:

| Pattern | Example |
|---|---|
| Structs are heap-alloc'd byte ranges with fixed offsets | `var d = alloc(48); store64(d + 8, name);` |
| Accessors are `module_field(ptr) → load64(ptr + offset)` | `tool_def_name(d) → load64(d)` |
| Optional fields use `0` as the sentinel | `tool_def_version(d) == 0` means no version |
| Tagged enums = i64 tag at offset 0 | `bote_err_tag(e) == ERR_INVALID_PARAMS` |
| Lists are `vec_*` from `lib/vec.cyr`; maps are `map_*` from `lib/hashmap.cyr` | |
| Strings are null-terminated cstrs unless prefixed `s_` (then `Str` from `lib/str.cyr`) | |
| Function pointers via `&fn_name`, called with `fncall1(fp, arg)` | |

JSON-RPC `id`, `params`, `result`, and error `data` are stored as **raw
JSON-literal cstrs** (e.g. `"1"`, `"\"abc\""`, `"null"`, `"{...}"`) since cyrius
has no nested JSON value type. The `jsonx` module extracts subtrees by slicing
the source bytes (respecting nested braces, brackets, and quoted strings).

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
Line-oriented JSON-RPC over fd 0 / fd 1. Reads chunks from stdin, splits on
`\n`, dispatches each complete line, leaves partial lines in a 128KB
heap-allocated buffer. EOF flushes any final non-terminated line.

### HTTP/1.1
Own minimal server (no `axum` equivalent in cyrius stdlib). Bind to
`127.0.0.1:port`, accept-loop, single-recv per connection (suitable for
typical JSON-RPC payloads under the 64KB request buffer). Routes:

| Method/Path | Action |
|---|---|
| `POST <endpoint>` (default `/mcp`) | dispatch JSON-RPC |
| Anything else | 404 / 405 |

Middleware (when configured on `HttpConfig`):
- **Origin** allow-list (403 on rejection; wildcard `*` allows all; empty list = strict mode rejects all)
- **MCP-Protocol-Version** (400 if invalid; 400 if absent and `require_protocol == 1`)
- **MCP-Session-Id** (404 on unknown SID; auto-bypass for `initialize`)

If a `SessionStore` is configured and the request is `initialize`, the response
includes a fresh `MCP-Session-Id` header (32-hex random from `/dev/urandom`).

### Unix domain socket
Same line-oriented protocol as stdio, but over `AF_UNIX`. Socket file is
unlinked + recreated on bind. Per-connection 128KB buffer.

---

## Verification

| Artifact | Count | Where |
|---|---|---|
| Unit tests | 251 | `tests/bote.tcyr` |
| Benchmarks | 10 | `tests/bote.bcyr` |
| Fuzz harnesses | 4 | `fuzz/*.fcyr` |

All hot paths are sub-10µs on x86_64.
