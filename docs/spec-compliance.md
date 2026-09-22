# MCP Spec Compliance

> **Spec Version**: 2025-11-25 (default) | **Bote Version**: 3.3.13 (cyrius 6.6.6) | **Last Audited**: 2026-09-22

This file lists what the shipped Cyrius implementation **covers today**, including the
gaps — a ❌ row is a defect on the released binary, a ⏳ row is planned work. Both are
scheduled in [`docs/development/roadmap.md`](development/roadmap.md). Re-audited at the
3.3.12 documentation sweep by probing the built binary, not by reading the previous table.

---

## Protocol Versions

| Version | Status |
|---|---|
| `2024-11-05` | ✅ |
| `2025-03-26` | ✅ |
| `2025-06-18` | ✅ (3.3.13) |
| `2025-11-25` | ✅ **default** |

Negotiated via `initialize`. The server picks the highest mutually
supported version. The list lives in exactly one place — `_mcp_is_supported`
(`src/dispatch.cyr`); `session.cyr`'s header validator defers to it. Through
3.3.12 there were two copies, and the drift they allowed was asymmetric: a
version missing from the header validator 400s on every HTTP-family transport
while stdio negotiates it happily. Four assertions now pin the two readers
against each other per version.

JSON-RPC batching was removed from the spec at `2025-06-18`; bote keeps accepting
batch arrays on every version (a superset is harmless to a client that never
sends one, and refusing them per-version would break the earlier clients that
legitimately batch).

---

## Core Protocol

| Spec Requirement | Module | Status |
|---|---|---|
| JSON-RPC 2.0 — request, response, notification, batch | `protocol` + `codec` | ✅ |
| Spec error codes (-32700, -32600, -32601, -32602, -32000, -32003, -32603, -32800) | `error` | ✅ |
| `initialize` handshake (serverInfo + capabilities + version negotiation; `serverInfo` configurable via `dispatcher_set_server_info`) | `dispatch` | ✅ |
| `ping` → empty `{}` result (spec: the receiver *MUST* respond promptly) | `dispatch` | ✅ (3.3.13) — answered ahead of every capability gate, so a bare dispatcher replies; emits no audit or event record (liveness, not activity). bote answers pings, it does not originate them |
| `notifications/initialized` / `notifications/cancelled` — accepted, never answered | `codec` (no response to any notification) | ✅ (`cancelled` is ignored: there is no in-flight work to cancel until threaded dispatch, 3.5.x) |
| `tools/list` with full `inputSchema` | `dispatch` + `registry` | ✅ |
| `tools/call` with arguments + version selection | `dispatch` + `schema` | ✅ |
| `prompts/list` + `prompts/get` (capability advertised iff a `PromptRegistry` is present) | `dispatch` + `prompts` | ✅ |
| `resources/list` + `resources/read` (capability advertised iff a `ResourceRegistry` is present) | `dispatch` + `resources` | ✅ |
| `resources/templates/list` (optional in the spec) | — | ❌ answers `-32601`; some clients call it whenever `resources` is advertised and read the error as a failure rather than "no templates". Roadmap — an empty list is the honest reply until a template registry exists. |
| `resources/subscribe` / `unsubscribe` + `notifications/resources/updated`; `resources` `listChanged` | — | ⏳ not implemented and not advertised — wait on real-time push (roadmap 3.5.x) |
| `completion/complete` (capability advertised iff a completion handler is set) | `dispatch` (`dispatcher_set_completion`) | ✅ |
| `notifications/tools/list_changed` + `notifications/prompts/list_changed` (buffered per session, drained on the client's next streamable `GET`; `listChanged` advertised only by a transport with a drain path) | `dispatch` (`dispatcher_set_notifications`) + `transport_streamable` (`strm_notify_sink`) | ✅ streamable (polled) |
| `logging/setLevel` + `notifications/message` | — | ⏳ not advertised — advertising it would promise messages bote cannot deliver; waits on real-time push (roadmap 3.5.x) |
| Notifications produce no response | `dispatch` + `codec` | ✅ |
| Batch arrays — mixed req + notif return only req responses (spec ≤ 2025-03-26; still accepted for later versions) | `codec` | ✅ |

## Tool Definitions

| Spec Requirement | Module | Status |
|---|---|---|
| Tool name, description, inputSchema | `registry::ToolDef` | ✅ |
| Tool versioning + version negotiation in `tools/call` | `tool_def_with_version` + `registry_get_versioned` | ✅ |
| Tool deprecation message | `tool_def_with_deprecated` | ✅ |
| Tool annotations: `readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint` — serialized in `tools/list` | `ToolAnnotations` (+ `ann_read_only` / `ann_destructive` presets); `_emit_annotations` | ✅ |
| Tool profile tags + optional `{"profile":"<tag>"}` filter on `tools/list` (bote extension) | `tool_def_with_profiles` + `_build_tools_list_result` | ✅ |
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
| Random 128-bit SID, 32-hex encoded, from the kernel CSPRNG via `random_bytes()` (`getrandom(2)`); the process **refuses to mint a session ID** if that fails rather than fall back to anything guessable | `_gen_session_id` | ✅ (3.2.0; was `/dev/urandom` by raw syscall) |

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
| **JWT HS256 verifier** (RFC 7519 / 7515) — `alg` read as an exact JSON field (not a substring scan), constant-time HMAC compare, `exp` enforced *after* the signature verifies with no leeway, malformed `exp` rejects, `exp` absent accepts; `auth_validator_jwt_hs256` plugs into the bearer middleware | `jwt::jwt_verify_hs256` | ✅ (2.2.0; `exp` + exact `alg` at 3.2.0; ships in `dist/bote.cyr` since 3.2.0) |
| **RFC 7636 PKCE-S256** — `pkce_code_verifier` (CSPRNG) + `pkce_code_challenge_s256` | `pkce` | ✅ (2.3.0; ships in `dist/bote.cyr` since 3.2.0) |
| **Pluggable sandbox runner** — fn-pointer + ctx adapter for tool handlers (kavach-shaped), noop default, error envelope on a null runner | `sandbox::sandbox_run` | ✅ (2.1.0; ships in `dist/bote.cyr` since **3.3.13** — before that it was in `src/` only and reached no consumer). Not in `dist/bote-core.cyr` |
| Asymmetric JWT (RS256 / ES256) | — | ⏳ an open decision, not a dependency — see the roadmap |

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
| Block-level annotations (`audience`, `priority`) | `content_with_annotations`; preserved through `bridge::wrap_tool_result` | ✅ (1.9.6; propagation 2.7.0) |

## Built-in tools

### `libro_*` (1.6.0)

| Tool | Purpose | Module |
|---|---|---|
| `libro_query` | Filter / paginate chain entries by source / agent / severity / time | `libro_tools::libro_tool_query` |
| `libro_verify` | Hash-link integrity check | `libro_tool_verify` |
| `libro_export` | Every entry as a JSON array | `libro_tool_export` |
| `libro_proof` | Merkle inclusion proof for entry at index | `libro_tool_proof` |
| `libro_retention` | Apply policy (`keep_count` / `keep_duration` / `keep_after` / `pci_dss` / `hipaa` / `sox`) | `libro_tool_retention` |

Registered by default in `main.cyr` against an empty chain at startup;
clients see them in `tools/list` immediately.

### `fs_*` (2.8.0) and `web_*` (3.1.0)

| Tool | Purpose | Guard |
|---|---|---|
| `fs_write` / `fs_read` / `fs_mkdir` | File operations | Root-confined to `BOTE_FS_ROOT` (default `.`); absolute and `..` paths refused |
| `web_fetch` | GET a URL and strip HTML to readable text | `http` / `https` only, 64 KiB cap, control bytes dropped |
| `web_search` | Query a SearXNG JSON endpoint | `BOTE_SEARXNG_URL` — self-hostable, no third-party key |

All three families are opt-in via `*_tools_register()` for library consumers and
registered by default in the binaries.

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
| `CancellationToken` (clone-shared flag) | `bote_cancel_token_*` | ✅ |
| `notifications/progress` JSON builder | `progress_notification` | ✅ |
| Threaded streaming dispatch | — | ⏳ bote-side work (roadmap 3.5.x). The cyrius primitives this once waited on — `lib/thread.cyr` MPSC, `lib/async.cyr`, `thread_local_alloc`, `arena_*` — are complete and pinned |
| `$/cancelRequest` mid-stream handling | — | ⏳ pairs with threaded dispatch |
| Server → client push (`tools` / `prompts` `list_changed`) | `transport_streamable` (`SessionOutbound`) | ✅ polled — buffered per session at produce time, drained on the next `GET` or piggybacked as SSE on a `POST`; advertised only by a transport with a drain path (3.0.0) |
| Held-open `GET` stream with live push | `transport_streamable` | 🟡 the stream opens and replays the resumption buffer; *live* delivery waits on threaded dispatch |

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
| DNS resolution (catch `127.0.0.1.nip.io` style bypasses) | — | ⏳ needs a resolve hook on the sandhi HTTP *client* so the classified address is the connected one (roadmap 3.6.x); `sandhi_resolve_ipv4` itself exists |

---

## Test Coverage (bote 3.3.12)

| Scope | Count | Source |
|---|---|---|
| Core — error / protocol / jsonx / registry / prompts / resources / completion / dispatch (incl. `ping` + version negotiation) / codec / schema / stream / session / HTTP helpers / discovery / bridge / events / audit | **439** | `tests/bote.tcyr` |
| Bearer + allowlist + JWT + PKCE validators | **38** | `tests/bote_auth.tcyr` |
| Content blocks + annotations | **24** | `tests/bote_content.tcyr` |
| `fs_tools` path safety + root confinement | **26** | `tests/bote_fs_tools.tcyr` |
| HostRegistry + IPv4 / IPv6 SSRF + hot-reload | **113** | `tests/bote_host.tcyr` |
| JWT HS256 — signature, exact `alg`, `exp` (mutation-proven) | **53** | `tests/bote_jwt.tcyr` |
| `libro_tools` incl. tampered-chain decode + populated-chain proof | **38** | `tests/bote_libro_tools.tcyr` |
| RFC 7636 PKCE-S256 | **17** | `tests/bote_pkce.tcyr` |
| Sandbox runner adapter | **13** | `tests/bote_sandbox.tcyr` |
| Streamable HTTP — event ids, resumption, per-session outbound, drain selection | **53** | `tests/bote_streamable.tcyr` |
| Unix transport — sockaddr, accept-error policy, backoff | **47** | `tests/bote_transport_unix.tcyr` |
| `web_tools` — scheme guard, HTML stripper, entities | **27** | `tests/bote_web_tools.tcyr` |
| WebSocket config + wire-up | **14** | `tests/bote_ws.tcyr` |
| **Total assertions** | **902** | + `bote_core_only_smoke.tcyr` (drift guard over `dist/bote-core.cyr`, exit-code driven); all 902 also run under `qemu-aarch64` |
| Hot-path benchmarks | **14** | `tests/bote.bcyr` |
| Fuzz harnesses | **4** | `fuzz/*.fcyr` (in CI since 3.3.8) |
| End-to-end transport round trips | 6 (stdio, HTTP, Unix, bridge, streamable, WS) | one `tools/call` per transport on the built binaries, run per release |

The conformance test suite — 44 protocol-level scenarios in the Rust archive's
`tests/conformance.rs` (tag `0.92.0`) — is **still not ported**. It would land as
`tests/conformance.tcyr`. The argument for it is 3.3.13: `ping` had been answering
`-32601` since the port and no assertion covered it, because the suite tests the
methods bote implements rather than the methods the spec requires. Roadmap 3.4.x.

---

*Audit method: manual comparison against [MCP spec 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25), with every method row probed against the built binary over stdio (the 3.3.12 sweep is how the `ping`, `2025-06-18` and `resources/templates/list` rows were found). For the explicit security-property table see [SECURITY.md](../SECURITY.md).*
