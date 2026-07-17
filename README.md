# bote

**MCP core service in Cyrius.**

JSON-RPC 2.0 protocol, tool registry, schema validation, dispatch, six
transports, bearer-token auth, libro audit tools, filesystem + web
tools, prompts / resources / completion capabilities, typed content
blocks, host registry with SSRF guard — in a self-hosting Cyrius project.
Eliminates per-app MCP implementations across the AGNOS ecosystem.

> **Name**: Bote (German) — messenger. The messenger between agents and tools.

[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)

> bote was originally a Rust crate. Ported to Cyrius for v1.0.0; the Rust
> archive was retired in v1.0.1 (the snapshot lives at git tag `0.92.0`
> for reference). See
> [docs/cyrius-feedback.md](docs/cyrius-feedback.md) for language issues
> discovered along the way and
> [docs/benchmarks-rust-v-cyrius.md](docs/benchmarks-rust-v-cyrius.md)
> for the side-by-side performance comparison.

---

## What it does

bote is the **MCP protocol layer** — handles JSON-RPC 2.0 wire format,
tool registration, schema validation, and call dispatch so individual
apps don't each reimplement the same protocol.

| Capability | Status |
|---|---|
| **JSON-RPC 2.0** — requests, responses, notifications, batch arrays | ✅ |
| **Tool registry** — register / discover / version / deprecate, insertion-order list | ✅ |
| **Compiled schema validation** — type, enum, bounds, nested objects, multi-violation | ✅ |
| **Dispatch** — `initialize`, `tools/list`, `tools/call`; `project_tool` name validation | ✅ |
| **Prompts / resources / completion** — `prompts/*`, `resources/*`, `completion/complete`; polled `list_changed` push | ✅ |
| **Stdio transport** — line-oriented JSON-RPC | ✅ |
| **HTTP/1.1 transport** — own server, Origin allow-list, MCP-Protocol-Version, MCP-Session-Id | ✅ |
| **Unix domain socket transport** | ✅ |
| **Bridge transport** — TS clients, CORS, MCP envelope wrap | ✅ |
| **Streamable HTTP transport (MCP 2025-11-25)** — POST + GET SSE, Last-Event-ID resumption | ✅ |
| **WebSocket transport (RFC 6455)** — full handshake, masked client / unmasked server frames | ✅ |
| **Bearer-token middleware (RFC 6750)** — opt-in per transport, fn-pointer + ctx validator | ✅ |
| **Sessions** — create / validate / prune; auto-create on `initialize` | ✅ |
| **`libro_tools`** — 5 built-in MCP tools (query / verify / export / proof / retention) over a libro audit chain | ✅ |
| **`fs_tools`** — 3 built-in MCP tools (`fs_write` / `fs_read` / `fs_mkdir`), root-confined via `BOTE_FS_ROOT` | ✅ |
| **`web_tools`** — `web_fetch` (HTML→text, size-capped) + `web_search` (SearXNG via `BOTE_SEARXNG_URL`) | ✅ |
| **Typed MCP content blocks** — text / image / audio / resource / resource_link / blob | ✅ |
| **`HostRegistry` + SSRF guard** — IPv4 + IPv6 blocklists for loopback, private, link-local, cloud-metadata | ✅ |
| **Audit / events sinks** — fn-pointer + ctx adapters, libro + majra wired | ✅ |
| **Streaming primitives** — `ProgressUpdate`, `CancellationToken`, progress notifications | data layer ✅ / threaded dispatch ⏳ |
| **OAuth 2.1 substrate** — bearer (RFC 6750), JWT HS256 verifier, PKCE-S256 helpers | ✅ |
| **Sandbox runner** — fn-pointer + ctx adapter (kavach 3.0 shape), noop default | ✅ |

---

## Quick start

### Build

```sh
cyrius deps            # resolve [deps.*] → lib/ (gitignored)
./scripts/build-all.sh # build/bote, build/bote-streamable, build/bote-ws
```

Static ELF binaries, no libc dependency.

### Run

```sh
./build/bote                    # stdio transport (default)
./build/bote http [port]        # HTTP on 127.0.0.1:port (default 8390)
./build/bote unix <path>        # Unix domain socket
./build/bote bridge [port]      # TS bridge with CORS (default 8391)
./build/bote-streamable [port]  # Streamable HTTP / SSE (default 8392)
./build/bote-ws [port]          # WebSocket (default 8393)
```

The default binary registers `bote_echo`, the five `libro_*` tools, the
three `fs_*` tools, and the two `web_*` tools.

### Bearer auth

```sh
BOTE_BEARER_TOKENS="tok-a,tok-b" ./build/bote http 8390
# Now every POST /mcp requires Authorization: Bearer tok-a (or tok-b);
# missing or wrong → 401 with WWW-Authenticate: Bearer realm="mcp"
```

Stdio + Unix sockets are local-only and skip the bearer check.

### Try it

```sh
# stdio
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}' | ./build/bote

# HTTP
./build/bote http 8390 &
curl -X POST http://127.0.0.1:8390/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2025-11-25' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'

# WebSocket (with wscat)
./build/bote-ws 8393 &
wscat -c ws://127.0.0.1:8393/mcp
> {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}
```

---

## MCP Protocol

bote implements [MCP](https://modelcontextprotocol.io/) over JSON-RPC 2.0.

| Method | Description | Response |
|---|---|---|
| `initialize` | Handshake | Server info, capabilities, negotiated protocol version |
| `tools/list` | Discovery | Array of tool definitions with schemas |
| `tools/call` | Execution | Tool result or JSON-RPC error |
| `prompts/list` / `prompts/get` | Prompts capability (when a `PromptRegistry` is wired) | Prompt metadata / generated messages |
| `resources/list` / `resources/read` | Resources capability (when a `ResourceRegistry` is wired) | Resource metadata / contents |
| `completion/complete` | Completion capability | Argument completion values |

Supported MCP protocol versions: `2024-11-05`, `2025-03-26`,
`2025-11-25` (default).

### Built-in tools (registered by default)

| Tool | Purpose |
|---|---|
| `bote_echo` | Echoes its arguments verbatim — smoke-test target |
| `libro_query` | Filter / paginate libro chain entries (source / agent / severity / time) |
| `libro_verify` | Hash-link integrity check |
| `libro_export` | Every entry as a JSON array |
| `libro_proof` | Merkle inclusion proof for an entry by index |
| `libro_retention` | Apply a policy (`keep_count` / `keep_duration` / `keep_after` / `pci_dss` / `hipaa` / `sox`) |
| `fs_write` / `fs_read` / `fs_mkdir` | Filesystem ops confined to `BOTE_FS_ROOT` (default `.`) |
| `web_fetch` | GET a URL (http/https only), strip HTML to readable text, size-capped |
| `web_search` | Query a SearXNG JSON endpoint (`BOTE_SEARXNG_URL`) |

### Error codes

| Code | Meaning |
|---|---|
| -32700 | Parse error |
| -32600 | Invalid request |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32000 | Tool execution / sandbox error |
| -32003 | Transport closed / bind failed |
| -32603 | Internal error |
| -32800 | Request cancelled |

---

## Modules

```
src/error.cyr              BoteErrTag enum, rpc_code mapping, format
src/protocol.cyr           JsonRpcRequest / Response / Error
src/jsonx.cyr              Nested-aware JSON value extractor
src/registry.cyr           ToolDef / ToolSchema / ToolRegistry / annotations
src/prompts.cyr            PromptRegistry (prompts/list + prompts/get)
src/resources.cyr          ResourceRegistry (resources/list + resources/read)
src/dispatch.cyr           Dispatcher + initialize / tools / prompts / resources / completion
src/codec.cyr              parse_request, serialize_response, process_message
src/schema.cyr             CompiledSchema (typed validation)
src/stream.cyr             ProgressUpdate, CancellationToken, progress notifications
src/session.cyr            SessionStore, validate_protocol_version, validate_origin
src/discovery.cyr          Cross-node tool discovery
src/audit.cyr              AuditSink (fn-ptr + ctx)
src/audit_libro.cyr        LibroAudit adapter
src/events.cyr             EventSink (fn-ptr + ctx) + topic constants
src/events_majra.cyr       MajraEvents adapter
src/auth.cyr               Bearer-token middleware (RFC 6750)
src/jwt.cyr                JWT HS256 verifier (RFC 7519)
src/pkce.cyr               RFC 7636 PKCE-S256 helpers
src/sandbox.cyr            Pluggable sandbox runner (kavach 3.0 adapter shape)
src/content.cyr            Typed MCP content blocks (text/image/audio/resource/blob)
src/host.cyr               HostRegistry + SSRF guard (IPv4 + IPv6)
src/libro_tools.cyr        Five built-in MCP tools over a libro chain
src/fs_tools.cyr           fs_write / fs_read / fs_mkdir (BOTE_FS_ROOT-confined)
src/web_tools.cyr          web_fetch / web_search (SearXNG)
src/transport_stdio.cyr    Line-oriented JSON-RPC over stdin/stdout
src/transport_http.cyr     HTTP/1.1 server with middleware
src/transport_unix.cyr     AF_UNIX line-oriented transport
src/bridge.cyr             TS-client bridge with CORS
src/transport_streamable.cyr  Streamable HTTP / SSE (MCP 2025-11-25)
src/transport_ws.cyr       WebSocket (RFC 6455)
src/main.cyr               CLI entry: stdio / http / unix / bridge
src/main_streamable.cyr    CLI entry: Streamable HTTP binary
src/main_ws.cyr            CLI entry: WebSocket binary
src/main_common.cyr        Shared binary setup (dispatcher + env bearer wiring)
```

Dependencies rehydrate into `lib/` (gitignored) via `cyrius deps`.
Cross-project deps (libro, majra, sigil, sakshi) are git-pinned via
`[deps.<name>]` in `cyrius.cyml`. Two consumer bundles ship in `dist/`:
`bote.cyr` (full, 28 modules) and `bote-core.cyr` (core 11,
transport-free) — see [DEPS-PATTERN.md](DEPS-PATTERN.md).

---

## Verification

### Tests — 786 total across twelve files (+ a core-only drift smoke)

```sh
cyrius test tests/bote.tcyr               # 415 — core protocol/dispatch/codec/schema/session/transports
cyrius test tests/bote_auth.tcyr          # 38 — bearer + allowlist + JWT + PKCE validators
cyrius test tests/bote_content.tcyr       # 24 — typed content blocks
cyrius test tests/bote_fs_tools.tcyr      # 26 — fs_write / fs_read / fs_mkdir
cyrius test tests/bote_host.tcyr          # 113 — host registry + SSRF guard (IPv4 + IPv6)
cyrius test tests/bote_jwt.tcyr           # 28 — JWT HS256 verify
cyrius test tests/bote_libro_tools.tcyr   # 22 — libro_tools wrappers
cyrius test tests/bote_pkce.tcyr          # 17 — RFC 7636 PKCE-S256
cyrius test tests/bote_sandbox.tcyr       # 13 — kavach 3.0 runner adapter
cyrius test tests/bote_streamable.tcyr    # 53 — Streamable HTTP / SSE internals
cyrius test tests/bote_web_tools.tcyr     # 27 — web_fetch / web_search
cyrius test tests/bote_ws.tcyr            # 10 — WebSocket config + wire-up
cyrius test tests/bote_core_only_smoke.tcyr  # drift guard — includes only dist/bote-core.cyr
```

### Benchmarks

```sh
cyrius bench tests/bote.bcyr
```

| Hot path | Avg |
|---|---|
| `dispatch_initialize` | ~2.9 µs |
| `dispatch_tools_list` | ~3.5 µs |
| `dispatch_tools_call` | ~6.6 µs |
| `jsonx_get_str_flat` | ~1.5 µs |
| `jsonx_get_raw_nested` | ~1.8 µs |
| `codec_parse_request` | ~2.7 µs |
| `codec_serialize_response` | ~1.9 µs |
| `codec_process_message` (full pipeline) | ~8.8 µs |
| `validate_compiled_simple` | ~1.9 µs |
| `validate_compiled_nested` | ~3.6 µs |
| `schema_compile_simple` | ~3.8 µs |
| `schema_compile_nested` | ~7.5 µs |
| `auth_bearer_check_unset` | ~1.3 µs |
| `auth_bearer_check_set` | ~2.1 µs |

### Fuzz

```sh
cyrius fuzz fuzz/codec_parse.fcyr
cyrius fuzz fuzz/codec_process.fcyr
cyrius fuzz fuzz/jsonx_extract.fcyr
cyrius fuzz fuzz/schema_validate.fcyr
# 4 passed, 0 failed each
```

---

## Why bote

Every AGNOS consumer used to implement its own MCP server. After bote
lands as a dep, those become a `Dispatcher` + a handful of tool
handlers — protocol code drops from ~150 LOC per app to zero.

The Cyrius port (v1.0.0) extended the same value into the cyrius
ecosystem, shipping a binary one-tenth the size of the Rust release for
the same surface.

---

## Documentation

| Doc | Topic |
|---|---|
| [docs/architecture/overview.md](docs/architecture/overview.md) | Module map, data flow, six-transport surface |
| [docs/development/roadmap.md](docs/development/roadmap.md) | Shipped per release, backlog, future |
| [docs/spec-compliance.md](docs/spec-compliance.md) | MCP 2025-11-25 conformance matrix |
| [docs/benchmarks-rust-v-cyrius.md](docs/benchmarks-rust-v-cyrius.md) | Side-by-side performance: Rust v0.92.0 vs Cyrius |
| [docs/cyrius-feedback.md](docs/cyrius-feedback.md) | Cyrius language issues found during the port |
| [docs/development/issues/](docs/development/issues/) | Open cyrius language/toolchain issues with reproducers |
| [docs/resolved-lang-issues.md](docs/resolved-lang-issues.md) | Historical index of resolved upstream cyrius issues |
| [DEPS-PATTERN.md](DEPS-PATTERN.md) | Distribution contract (`dist/bote.cyr` / `dist/bote-core.cyr`) |
| [SECURITY.md](SECURITY.md) | Threat model, reporting policy |

---

## Versioning

**Current**: `3.1.2` — full MCP capability suite (prompts / resources /
completion + polled `list_changed` push), filesystem + web tools, six
transports across three binaries, JWT/PKCE auth substrate. SemVer; the
2.0 handler ABI is stable across the 2.x→3.x line. See
[CHANGELOG.md](CHANGELOG.md) for the full history.

**Roadmap** — see [roadmap](docs/development/roadmap.md).

---

## License

GPL-3.0-only. See [LICENSE](LICENSE).
