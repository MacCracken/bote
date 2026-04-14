# MCP Spec Compliance

> **Spec Version**: 2025-11-25 | **Bote Version**: 1.9.2 (cyrius 4.7.0) | **Last Audited**: 2026-04-14

This file lists what the shipped Cyrius implementation **covers today**.
For deferred items see
[`docs/development/roadmap.md`](development/roadmap.md).

---

## Protocol Versions

| Version | Status |
|---|---|
| `2024-11-05` | ✅ |
| `2025-03-26` | ✅ |
| `2025-11-25` | ✅ **default** |

Negotiated via `initialize`. The server picks the highest mutually
supported version.

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
| Tool deprecation message | `tool_def_with_deprecated` | ✅ |
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
| `MCP-Protocol-Version` header parsing + validation | `session::validate_protocol_version` + per-transport `_check_protocol` | ✅ |
| `MCP-Session-Id` — server-issued on `initialize`, validated on subsequent | `session::SessionStore` + per-transport `_check_session` | ✅ HTTP / streamable / bridge |
| Session creation on initialize (auto, returned in response header) | `session_store_create` + handler hook | ✅ |
| Session timeout + pruning (configurable; default 1h) | `session_store_prune_expired` | ✅ |
| Random 128-bit SID via `/dev/urandom`, 32-hex encoded | `_gen_session_id` | ✅ |

## Security

| Spec Requirement | Module | Status |
|---|---|---|
| Origin allow-list (DNS rebinding protection — 403 on rejection) | `session::validate_origin` + per-transport `_check_origin` | ✅ |
| Wildcard `*` and exact-match origins | same | ✅ |
| Strict mode (empty allow-list rejects all) | same | ✅ |
| CORS preflight (OPTIONS / + 3 ACA-* headers) | `bridge::_bridge_cors_headers` | ✅ |
| **Bearer-token middleware (RFC 6750)** — opt-in fn-pointer + ctx validator on every HTTP-family transport | `auth::auth_bearer_check` + per-transport wiring | ✅ (1.9.0) |
| **`WWW-Authenticate: Bearer realm="..."` on 401** | `auth::auth_send_unauthorized` | ✅ |
| **Built-in validators**: `auth_validator_allow_all` (dev), `auth_validator_allowlist` (vec membership) | `auth` | ✅ |
| **`BOTE_BEARER_TOKENS` env var** wires an allowlist validator across all four HTTP-family transports at startup | `main::_bote_bearer_from_env` | ✅ (1.9.1) |
| **HTTP body-length clamp** — `clen = min(clen, n - bo)` so a lying Content-Length can't make `memcpy` read past the request buffer | `transport_http`, `transport_streamable`, `bridge` | ✅ (1.5.1) |
| **SSRF guard for outbound URL fetches** — IPv4 + IPv6 blocklists for loopback / private / link-local / cloud-metadata | `host::ssrf_check` | ✅ (1.8.0 / 1.9.1) |

## Transports

| Transport | Module | Status |
|---|---|---|
| stdio (line-oriented) | `transport_stdio` | ✅ |
| HTTP/1.1 (`POST <endpoint>`) with full middleware | `transport_http` | ✅ |
| Unix domain socket (line-oriented) | `transport_unix` | ✅ |
| TypeScript bridge (POST `/`, GET `/health`, OPTIONS, MCP envelope wrap) | `bridge` | ✅ |
| **Streamable HTTP (MCP 2025-11-25)** — single endpoint POST + GET SSE, `Last-Event-ID` resumption, bounded resumption buffer (default 1000), `retry:` hint | `transport_streamable` | ✅ (1.4.0) |
| **WebSocket (RFC 6455)** — handshake (`Sec-WebSocket-Accept = base64(sha1(key + magic))`), masked-client / unmasked-server frames, ping/pong/close handled transparently | `transport_ws` + `lib/ws_server.cyr` | ✅ (1.5.0) |

## Content Blocks (MCP 2025-11-25)

| Block type | Constructor | Status |
|---|---|---|
| `text` | `content_text(text)` | ✅ (1.7.0) |
| `image` (base64 inline) | `content_image(b64, mime)` | ✅ (1.7.0) |
| `audio` (base64 inline) | `content_audio(b64, mime)` | ✅ (1.7.0) |
| `resource` (embedded text body) | `content_resource(uri, mime, text)` | ✅ (1.7.0) |
| `resource` (embedded binary `blob`) | `content_resource_blob(uri, mime, b64)` | ✅ (1.9.1) |
| `resource_link` (reference) | `content_resource_link(uri, name, mime)` | ✅ (1.7.0) |
| Envelope: `{"content":[...]}` | `content_array(blocks)` / `content_single(block)` / `content_text_response(text)` | ✅ |
| Tool-error envelope: `{"content":[...],"isError":true}` | `content_array_error(blocks)` | ✅ |
| Block-level annotations (`audience`, `priority`) | `content_with_annotations` | ⏳ — reverted from 1.9.1 (cap), planned for 2.0 |

## Built-in `libro_*` Tools (1.6.0)

| Tool | Purpose | Module |
|---|---|---|
| `libro_query` | Filter / paginate chain entries by source / agent / severity / time | `libro_tools::libro_tool_query` |
| `libro_verify` | Hash-link integrity check | `libro_tool_verify` |
| `libro_export` | Every entry as a JSON array | `libro_tool_export` |
| `libro_proof` | Merkle inclusion proof for entry at index | `libro_tool_proof` |
| `libro_retention` | Apply policy (`keep_count` / `keep_duration` / `keep_after` / `pci_dss` / `hipaa` / `sox`) | `libro_tool_retention` |

Registered by default in `main.cyr` against an empty chain at startup;
clients see them in `tools/list` immediately.

## Bridge (MCP envelope contract)

| Spec Requirement | Module | Status |
|---|---|---|
| `tools/call` success → wrap result in `{"content":[{"type":"text","text":...}]}` | `wrap_tool_result` | ✅ (passthrough if already shaped) |
| `tools/call` error → `result: {"content":[...],"isError":true}` | `wrap_error_result` + `_bridge_process_single` | ✅ |
| Other methods (`initialize`, `tools/list`) — pass through unchanged | `bridge_process_message` | ✅ |
| `GET /health` returns `200 ok` | `_bridge_send_health` | ✅ |
| Pre-built `content` envelope from typed-block constructors → passed through | verified in `bote_content.tcyr` | ✅ |

## Discovery (data layer)

| Spec Requirement | Module | Status |
|---|---|---|
| `ToolAnnouncement` JSON envelope with `node_id` + `tools[]` | `announcement_to_json` | ✅ |
| `DiscoveryService` with pluggable publish function pointer | `discovery_new` + `discovery_publish_fp` | ✅ |
| `DiscoveryReceiver` queue with `try_recv` | `discovery_receiver_*` | ✅ |
| Wired to majra pubsub | `events_majra_publish` adapter | ✅ (1.2.0) |

## Streaming primitives (data layer)

| Spec Requirement | Module | Status |
|---|---|---|
| `ProgressUpdate` (progress / total / message) | `progress_update_*` | ✅ |
| `CancellationToken` (clone-shared flag) | `cancel_token_*` | ✅ |
| `notifications/progress` JSON builder | `progress_notification` | ✅ |
| Threaded streaming dispatch | — | ⏳ deferred — waits on cyrius `lib/thread.cyr` MPSC + `lib/async.cyr` cancellation |
| `$/cancelRequest` mid-stream handling | — | ⏳ pairs with streaming dispatch |
| Server-initiated event push on streamable GET stream | `transport_streamable` (data path) | 🟡 transport opens stream + replays buffer; live push waits on streaming dispatch |

## Audit / Events Sinks

| Spec Requirement | Module | Status |
|---|---|---|
| `AuditSink` (fn-pointer + ctx) — sinks-noop default = zero overhead | `audit::AuditSink` + `dispatcher_set_audit` | ✅ (1.1.0) |
| `EventSink` (fn-pointer + ctx) | `events::EventSink` + `dispatcher_set_events` | ✅ (1.1.0) |
| Topic constants (10 well-known) | `events::TOPIC_*` | ✅ |
| Dispatcher emits audit + events on tools/call, register, dereg, deprecate | `dispatch` | ✅ |
| `LibroAudit` adapter | `audit_libro::libro_audit_log` | ✅ (1.2.0) |
| `MajraEvents` adapter | `events_majra::majra_events_publish` | ✅ (1.2.0) |

## Host / SSRF

| Spec Requirement | Module | Status |
|---|---|---|
| `HostRegistry` — name → `{url, headers, capabilities}` map | `host::HostRegistry` | ✅ (1.8.0) |
| Capability allowlist (`host_entry_allows`); fail-open when unset | same | ✅ |
| `ssrf_check(url)` — IPv4 (loopback / private / link-local / metadata / unspec / multicast) | `host::_ssrf_classify_ipv4` | ✅ (1.8.0) |
| IPv6 blocklist (bracket form, `::1`, `::`, `fe80::/10`, `fc00::/7`, `ff00::/8`) | `host::_ssrf_classify_ipv6` | ✅ (1.9.1) |
| Hostname blocklist (`localhost`, `metadata.google.internal`, `metadata`) — case-insensitive | `host::_ssrf_classify_hostname` | ✅ |
| `user:pass@` userinfo stripping before classification | `host::_ssrf_extract_host` | ✅ |
| Scheme gate — only `http://` / `https://` | same | ✅ |
| DNS resolution (catch `127.0.0.1.nip.io` style bypasses) | — | ⏳ needs cyrius DNS stub |

---

## Test Coverage (Cyrius v1.9.2)

| Scope | Count | Source |
|---|---|---|
| Unit assertions (core protocol/dispatch/codec/schema/session/transports) | **394** | `tests/bote.tcyr` |
| `libro_tools` assertions | **22** | `tests/bote_libro_tools.tcyr` |
| Content-block assertions | **18** | `tests/bote_content.tcyr` |
| Host-registry + SSRF assertions | **56** | `tests/bote_host.tcyr` |
| Bearer-middleware assertions | **29** | `tests/bote_auth.tcyr` |
| **Total assertions** | **519** | (was 251 at v1.0.0) |
| Hot-path benchmarks | **10** | `tests/bote.bcyr` |
| Fuzz harnesses | **4** | `fuzz/*.fcyr` |
| End-to-end transport smokes | 6 (stdio, HTTP, Unix, bridge, streamable, WS) | manual via `./build/bote` + curl/wscat |

What the unit suite covers: every BoteError variant + format, every
protocol struct + accessor, full registry lifecycle (register / get /
list / contains / dereg / versioned / deprecate / annotations), every
CompiledSchema type + bounds (including exact-min and exact-max
boundaries), every codec path (single + batch + notification + parse
error + invalid version + non-object), full dispatcher routing
(initialize / tools/list / tools/call / unknown / dynamic register /
dereg), session lifecycle + prune + origin + protocol-version
validation, all jsonx extractors with truncated/escaped/nested inputs,
bridge wrap variants + CORS origin selection, discovery announcements
+ receiver queue, HTTP request parser, **streamable EventIdGenerator +
ResumptionBuffer + StreamableConfig**, **WsConfig + handler addressability**,
**libro_tools registration + empty-chain shape**, **every content
block constructor**, **every SSRF block-list code path including IPv4
edges and IPv6 prefix forms**, **bearer scheme parsing + OWS handling
+ allowlist + middleware short-circuit**.

Conformance test suite (44 protocol-level scenarios in the Rust
archive) is **not yet ported** — would land as `tests/conformance.tcyr`.
Tracked under v2.0.

---

*Audit method: manual comparison against [MCP spec 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25). For the explicit security-property table see [SECURITY.md](../SECURITY.md).*
