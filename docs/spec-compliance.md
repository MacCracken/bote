# MCP Spec Compliance

> **Spec Version**: 2025-11-25 | **Bote Version**: 1.0.1 (cyrius) | **Last Audited**: 2026-04-13

This file lists what the shipped Cyrius implementation **covers today**. For
deferred items (audit, events, sandbox, host, auth, streamable HTTP, WS,
streaming dispatch, libro_tools), see
[`docs/development/roadmap.md`](development/roadmap.md).

---

## Protocol Versions

| Version | Status |
|---|---|
| `2024-11-05` | Supported |
| `2025-03-26` | Supported |
| `2025-11-25` | **Supported (default)** |

Negotiated via `initialize`. The server picks the highest mutually supported
version.

---

## Core Protocol

| Spec Requirement | Module | Status |
|---|---|---|
| JSON-RPC 2.0 — request, response, notification, batch | `protocol` + `codec` | ✅ |
| Spec error codes (-32700, -32600, -32601, -32602, -32000, -32003, -32603, -32800) | `error` | ✅ |
| `initialize` handshake (serverInfo + capabilities + version negotiation) | `dispatch` | ✅ |
| `tools/list` with full `inputSchema` | `dispatch` + `registry` | ✅ |
| `tools/call` with arguments + version selection | `dispatch` + `schema` | ✅ |
| Notifications produce no response | `dispatch` + `codec` | ✅ |
| Batch arrays — mixed req + notif return only req responses | `codec` | ✅ |

## Tool Definitions

| Spec Requirement | Module | Status |
|---|---|---|
| Tool name, description, inputSchema | `registry::ToolDef` | ✅ |
| Tool versioning + version negotiation in `tools/call` | `tool_def_with_version` + `registry_get_versioned` | ✅ |
| Tool deprecation message | `tool_def_with_deprecated` | ✅ (emitted in `tools/list`) |
| Tool annotations: `readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint` | `ToolAnnotations` (+ `ann_read_only` / `ann_destructive` presets) | ✅ |
| `project_tool` naming convention enforced on dynamic register | `validate_tool_name` | ✅ |

## Schema Validation

| Spec Requirement | Module | Status |
|---|---|---|
| JSON Schema subset — string, number, integer, boolean, array, object, ANY | `schema::SchemaTypeTag` | ✅ |
| String enum constraint | `prop_enum_values` | ✅ |
| Numeric bounds (`minimum`, `maximum`) for number + integer | `prop_min` / `prop_max` | ✅ |
| Required field check | `compiled_required` | ✅ |
| Recursive nested object validation | `_validate_against` | ✅ |
| Array `items` typing | `prop_items` | ✅ |
| Multi-violation reporting (collect all, don't fail-fast) | `compiled_validate` returns vec | ✅ |
| Permissive on extra (unknown) fields | by design | ✅ |

## Session Management

| Spec Requirement | Module | Status |
|---|---|---|
| `MCP-Protocol-Version` header parsing + validation | `session::validate_protocol_version` + `_http_check_protocol` | ✅ |
| `MCP-Session-Id` header — server-issued on `initialize`, validated on subsequent | `session::SessionStore` + `_http_check_session` | ✅ (HTTP transport; bridge inherits) |
| Session creation on initialize (auto, returned in response header) | `session_store_create` + handler hook | ✅ |
| Session timeout + pruning (configurable; default 1h) | `session_store_prune_expired` | ✅ |
| Random 128-bit SID via `/dev/urandom`, 32-hex encoded | `_gen_session_id` | ✅ |

## Security

| Spec Requirement | Module | Status |
|---|---|---|
| Origin allow-list (DNS rebinding protection — 403 on rejection) | `session::validate_origin` + `_http_check_origin` | ✅ |
| Wildcard `*` and exact-match origins | same | ✅ |
| Strict mode (empty allow-list rejects all) | same | ✅ |
| CORS preflight (OPTIONS / + 3 ACA-* headers) | `bridge::_bridge_cors_headers` | ✅ |

## Transports

| Transport | Module | Status |
|---|---|---|
| stdio (line-oriented) | `transport_stdio` | ✅ |
| HTTP/1.1 (`POST <endpoint>`) with full middleware | `transport_http` | ✅ |
| Unix domain socket (line-oriented) | `transport_unix` | ✅ |
| TypeScript bridge (POST `/`, GET `/health`, OPTIONS, MCP envelope wrap) | `bridge` | ✅ |

## TypeScript Bridge (MCP envelope contract)

| Spec Requirement | Module | Status |
|---|---|---|
| `tools/call` success → wrap result in `{"content":[{"type":"text","text":...}]}` | `wrap_tool_result` | ✅ (passthrough if already shaped) |
| `tools/call` error → `result: {"content":[...],"isError":true}` (not error obj) | `wrap_error_result` + `_bridge_process_single` | ✅ |
| Other methods (`initialize`, `tools/list`) — pass through unchanged | `bridge_process_message` | ✅ |
| `GET /health` returns `200 ok` | `_bridge_send_health` | ✅ |

## Discovery (data layer)

| Spec Requirement | Module | Status |
|---|---|---|
| `ToolAnnouncement` JSON envelope with `node_id` + `tools[]` | `announcement_to_json` | ✅ |
| `DiscoveryService` with pluggable publish function pointer | `discovery_new` + `discovery_publish_fp` | ✅ |
| `DiscoveryReceiver` queue with `try_recv` | `discovery_receiver_*` | ✅ |
| Wired to majra pubsub | — | 🟡 deferred to v1.1.0 (stub uses fn-pointer) |

## Streaming primitives (data layer)

| Spec Requirement | Module | Status |
|---|---|---|
| `ProgressUpdate` (progress / total / message) | `progress_update_*` | ✅ |
| `CancellationToken` (clone-shared flag) | `cancel_token_*` | ✅ |
| `notifications/progress` JSON builder | `progress_notification` | ✅ |
| Threaded streaming dispatch (handler runs on a worker; progress drained to transport) | — | 🟡 deferred to v1.3.0 (waits on `lib/thread.cyr` MPSC) |

---

## Gaps that need coverage

These are spec items the cyrius port doesn't cover yet. Each is roadmapped;
listed here because they affect *real conformance*, not just feature breadth.

1. **Threaded streaming dispatch** — `dispatcher_dispatch_streaming` returning `DispatchOutcome::Streaming` so the transport can interleave `notifications/progress` messages with the final result. Data primitives are in place; only the dispatch wire-up + thread spawn is missing. → **v1.3.0**
2. **Streamable HTTP transport** — single endpoint POST + GET (SSE) with `Last-Event-ID` resumption and `retry:` hints. → **v1.2.0**
3. **WebSocket server** — Cyrius `lib/ws.cyr` is client-only. Server-side handshake (Sec-WebSocket-Accept) + incoming-frame unmasking is new. → **v1.3.0**
4. **OAuth 2.1 / PKCE-S256 bearer middleware** — token validator function pointer on `HttpConfig` + 401/403 emission with `WWW-Authenticate` headers. No external deps. → **v1.2.0**
5. **MCP `resource` content type** — content blocks with URI + text/blob; resource subscriptions. Lands with `host` module. → **v1.2.0**
6. **`completion/complete` method** — argument autocompletion. → **v1.x**
7. **`$/cancelRequest` handling** — needs CancellationToken polling at handler boundaries. Pairs with streaming dispatch. → **v1.3.0**
8. **`apply_defaults`** — schema's default-value injection. Single helper that fills missing optional fields with their `default`. → patchable any time.

---

## Test Coverage (Cyrius v1.0.1)

| Scope | Count | Source |
|---|---|---|
| Unit assertions | **301** | `tests/bote.tcyr` |
| Hot-path benchmarks | **10** | `tests/bote.bcyr` |
| Fuzz harnesses | **4** (~330 cross-product calls) | `fuzz/*.fcyr` |
| End-to-end transport smokes | 4 (stdio, HTTP, Unix, bridge) | manual via `./build/bote` + curl/python |

What the unit suite covers: every BoteError variant + format, every protocol struct + accessor, full registry lifecycle (register / get / list / contains / dereg / versioned / deprecate / annotations), every CompiledSchema type + bounds (including exact-min and exact-max boundaries), every codec path (single + batch + notification + parse error + invalid version + non-object), full dispatcher routing (initialize / tools/list / tools/call / unknown / dynamic register / dereg), session lifecycle + prune + origin + protocol-version validation, all jsonx extractors with truncated/escaped/nested inputs, bridge wrap variants + CORS origin selection, discovery announcements + receiver queue, HTTP request parser (method/path/headers/body offset).

Conformance test suite (44 protocol-level scenarios in the Rust archive) is **not yet ported** — would land as `tests/conformance.tcyr`. Tracked under v1.x.

---

*Audit method: manual comparison against [MCP spec 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25).*
