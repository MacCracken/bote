# bote

**MCP core service in Cyrius.**

JSON-RPC 2.0 protocol, tool registry, schema validation, dispatch, and three
transports — in a self-hosting Cyrius project. Eliminates per-app MCP
implementations across the AGNOS ecosystem.

> **Name**: Bote (German) — messenger. The messenger between agents and tools.

[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)

> **Note**: bote was originally a Rust crate (see `rust-old/` for the archive).
> It was ported to Cyrius via `cyrius port` — see [docs/development/cyrius-port.md](docs/development/cyrius-port.md)
> for the porting log and [docs/cyrius-feedback.md](docs/cyrius-feedback.md) for
> language issues discovered along the way.

---

## What it does

bote is the **MCP protocol layer** — it handles JSON-RPC 2.0 wire format, tool
registration, schema validation, and call dispatch so individual apps don't each
reimplement the same protocol.

| Capability | Status |
|---|---|
| **JSON-RPC 2.0** — requests, responses, notifications, batch arrays | Complete |
| **Tool registry** — register/discover/version/deprecate, insertion-order list | Complete |
| **Compiled schema validation** — type, enum, bounds, nested objects, multi-violation | Complete |
| **Dispatch** — `initialize`, `tools/list`, `tools/call`; project_tool name validation | Complete |
| **Transports** — stdio, HTTP/1.1, Unix domain socket | Complete |
| **HTTP middleware** — Origin allow-list, MCP-Protocol-Version, MCP-Session-Id | Complete |
| **Sessions** — create/validate/prune; auto-create on `initialize` | Complete |
| **Streaming primitives** — ProgressUpdate, CancellationToken, progress notifications | Data layer (no thread integration yet) |
| **Audit / events / sandbox** — libro / majra / kavach integrations | Pending (Rust modules archived in `rust-old/`) |
| **TypeScript bridge** — CORS + MCP envelope | Pending |
| **WebSocket transport** | Pending (cyrius `lib/ws.cyr` is client-side only) |

---

## Quick start

### Build

```sh
cyrius build src/main.cyr build/bote
```

That produces a single static ELF binary (`build/bote`, ~250KB).

### Run

```sh
./build/bote                    # stdio transport
./build/bote http [port]        # HTTP transport on 127.0.0.1:port (default 8390)
./build/bote unix <path>        # Unix domain socket transport
```

The default binary registers a single `bote_echo` tool that returns its
arguments verbatim — useful for end-to-end smoke tests.

### Try it

```sh
# stdio
echo '{"jsonrpc":"2.0","id":1,"method":"initialize"}' | ./build/bote

# HTTP
./build/bote http 8390 &
curl -X POST http://127.0.0.1:8390/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

---

## MCP Protocol

bote implements [MCP](https://modelcontextprotocol.io/) over JSON-RPC 2.0.

| Method | Description | Response |
|---|---|---|
| `initialize` | Handshake | Server info, capabilities, negotiated protocol version |
| `tools/list` | Discovery | Array of tool definitions with schemas |
| `tools/call` | Execution | Tool result or JSON-RPC error |

Supported MCP protocol versions: `2024-11-05`, `2025-03-26`, `2025-11-25` (default).

### Error codes

| Code | Meaning |
|---|---|
| -32700 | Parse error |
| -32600 | Invalid request (bad jsonrpc version, empty batch, non-object) |
| -32601 | Method not found |
| -32602 | Invalid params (missing required, schema violation) |
| -32000 | Tool execution error / sandbox error |
| -32003 | Transport closed / bind failed |
| -32603 | Internal error |
| -32800 | Request cancelled |

---

## Modules

Cyrius modules live in `src/`:

```
src/error.cyr          BoteErrTag enum, rpc_code mapping, format
src/protocol.cyr       JsonRpcRequest / Response / Error
src/jsonx.cyr          Nested-aware JSON value extractor
src/registry.cyr       ToolDef / ToolSchema / ToolRegistry / ToolAnnotations
src/dispatch.cyr       Dispatcher + initialize / tools/list / tools/call
src/codec.cyr          parse_request, serialize_response, process_message
src/schema.cyr         CompiledSchema (typed validation)
src/stream.cyr         ProgressUpdate, CancellationToken, progress_notification
src/session.cyr        SessionStore, validate_protocol_version, validate_origin
src/transport_stdio.cyr  Line-oriented JSON-RPC over stdin/stdout
src/transport_http.cyr   HTTP/1.1 server with middleware
src/transport_unix.cyr   AF_UNIX line-oriented transport
src/main.cyr           CLI: argv switch over the three transports
```

Stdlib dependencies are vendored in `lib/` (47 modules: alloc, vec, hashmap,
str, json, fnptr, chrono, tagged, net, syscalls, …).

---

## Verification

### Tests

```sh
cyrius test tests/bote.tcyr
# 251 passed, 0 failed (251 total)
```

### Benchmarks

```sh
cyrius bench tests/bote.bcyr
```

| Hot path | Avg |
|---|---|
| `dispatch_initialize` | ~2µs |
| `dispatch_tools_list` | ~2µs |
| `dispatch_tools_call` | ~1µs |
| `jsonx_get_str_flat` | 600ns |
| `jsonx_get_raw_nested` | ~1µs |
| `codec_parse_request` | ~2µs |
| `codec_serialize_response` | ~1µs |
| `codec_process_message` (full pipeline) | ~5µs |
| `validate_compiled_simple` | ~1µs |
| `validate_compiled_nested` | ~3µs |

### Fuzz

```sh
cyrius fuzz
# 4 passed, 0 failed
```

Fuzz harnesses in `fuzz/`: `codec_parse`, `codec_process`, `jsonx_extract`,
`schema_validate`. ~330 calls across malformed and edge-case inputs; no crashes.

---

## Why bote

Every AGNOS consumer app currently implements its own MCP server (Rust):

```
Before bote:                          After bote:
─────────────                         ────────────
jalwa/src/mcp.rs    (150 lines)       jalwa: bote::Dispatcher + 5 handlers
shruti/src/mcp.rs   (180 lines)       shruti: bote::Dispatcher + 7 handlers
tazama/src/mcp.rs   (160 lines)       tazama: bote::Dispatcher + 7 handlers
... × 23 apps       (~4000 lines)     ... × 23 apps (0 protocol code)
```

The cyrius port lifts the same value into the cyrius ecosystem.

---

## Documentation

| Doc | Topic |
|---|---|
| [docs/architecture/overview.md](docs/architecture/overview.md) | Module map, data flow |
| [docs/development/roadmap.md](docs/development/roadmap.md) | What's done, what's next |
| [docs/spec-compliance.md](docs/spec-compliance.md) | MCP 2025-11-25 conformance matrix |
| [docs/cyrius-feedback.md](docs/cyrius-feedback.md) | Cyrius language issues found during the port |

---

## Versioning

**Current**: `1.0.0` — stable cyrius MCP core. Standard SemVer from here. See [CHANGELOG.md](CHANGELOG.md) for the full history.

---

## License

GPL-3.0-only. See [LICENSE](LICENSE).
