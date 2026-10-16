# MCP Spec Compliance

> **Spec Version**: 2025-11-25 (default) | **Bote Version**: 3.4.7 (cyrius 6.7.9) | **Last Audited**: 2026-10-15 (at 3.4.5, with `tests/conformance.tcyr`)

This file lists what the shipped Cyrius implementation **covers today**, including the
gaps — a ❌ row is a defect on the released binary, a ⏳ row is planned work. Both are
scheduled in [`docs/development/roadmap.md`](development/roadmap.md). Re-audited at the
3.3.12 documentation sweep by probing the built binary, not by reading the previous table.
Since 3.4.5 the protocol rows below are also asserted, by spec clause, in
`tests/conformance.tcyr`, which CI runs. On the 3.4.4 sources that suite failed 29 of its 117
assertions, which is how the rows marked (3.4.5) were found.

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
| Invalid JSON is `-32700`; valid JSON that is not a request is `-32600` (§5.1) | `jsonx_valid` + `codec` | ✅ (3.4.5) — the whole message is syntax-checked (RFC 8259, nesting ≤ 64) before anything reads it. Through 3.4.4 `not valid json` was `-32600`, the spec's truncated-object example was dispatched as a notification, and `"id":01` was echoed back as invalid JSON |
| `id` is a string, a number or null; `params`, if present, is an object or an array (§4) | `codec::_cdc_admit` | ✅ (3.4.5) — otherwise `-32600`. An explicit `"params":null` is read as absent. A null id is answered, though MCP forbids a client to send one |
| Spec error codes (-32700, -32600, -32601, -32602, -32002, -32000, -32003, -32603, -32800) | `error` + `dispatch` | ✅ (`-32002` since 3.4.5) |
| Every response is valid JSON: strings escaped per RFC 8259 §7, control bytes included | `jsonx::bote_json_escape` | ✅ (3.4.6) — through 3.4.5 the escaper left control bytes other than LF / CR / TAB raw, so a tool result holding an ESC or a BEL (an ANSI-coloured log through `fs_read`) was not JSON, and an embedder's `serverInfo` was not escaped at all. Asserted by the conformance suite's standing valid-JSON check (J15, F12) and a fuzz property |
| `initialize` handshake (serverInfo + capabilities + version negotiation; `serverInfo` configurable via `dispatcher_set_server_info`) | `dispatch` | ✅ |
| `ping` → empty `{}` result (spec: the receiver *MUST* respond promptly) | `dispatch` | ✅ (3.3.13) — answered ahead of every capability gate, so a bare dispatcher replies; emits no audit or event record (liveness, not activity). bote answers pings, it does not originate them |
| `notifications/initialized` / `notifications/cancelled` — accepted, never answered | `codec` (no response to any notification) | ✅ (`cancelled` is ignored: there is no in-flight work to cancel until threaded dispatch, 3.5.x) |
| `tools/list` with full `inputSchema` | `dispatch` + `registry` | ✅ |
| Pagination: `nextCursor` omitted when there is no next page; an invalid `cursor` is `-32602` | `dispatch` | ✅ (3.4.5) — bote returns every item in one page and never issues a cursor, so any `cursor` a client sends is invalid and gets `-32602`, the spec's code for one, on `tools/list`, `prompts/list`, `resources/list` and `resources/templates/list`. A null `cursor` is read as absent. Through 3.4.4 a cursor was ignored and the first page sent again, which a client concatenating pages would take as duplicates |
| `tools/call` with arguments + version selection | `dispatch` + `schema` | ✅ |
| `tools/call` error split: an unknown tool or a malformed CallToolRequest (missing / empty / non-string `name`, non-object `arguments`) is a protocol error `-32602`; arguments failing the tool's `inputSchema` are a tool execution error, a result with `isError: true` | `dispatch` | ✅ (3.4.5) — per the tools spec's "Error Handling" and SEP-1303. Through 3.4.4 an unknown tool was `-32601` and a schema violation `-32602`. The dispatcher has no per-session negotiated version, so this split applies to every revision. 2025-06-18 already listed "invalid input data" as a tool execution error |
| `tools/call` result is always a CallToolResult | `dispatch` | ✅ (3.4.5) — a handler that returns nothing is a failed call (`isError: true`), not `"result":null` |
| `prompts/list` + `prompts/get` (capability advertised iff a `PromptRegistry` is present) | `dispatch` + `prompts` | ✅ — an unknown name, a missing required argument and non-object `arguments` are `-32602` (the last two since 3.4.5) |
| `resources/list` + `resources/read` (capability advertised iff a `ResourceRegistry` is present) | `dispatch` + `resources` | ✅ — an unserved URI is `-32002` with `data.uri`, as in the resources spec's example (3.4.5; was `-32602`) |
| `resources/templates/list` (optional in the spec) | `dispatch` | ✅ (3.4.1) — answers `{"resourceTemplates":[]}` whenever resources are advertised (gated like `resources/list`; no registry → `-32601`). It answered `-32601` until 3.4.1, which some clients read as a failure rather than "no templates". bote has no URI-template registry (RFC 6570); the empty list is the honest reply. A `cursor` is `-32602` since 3.4.5, as on the other list methods |
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
| `ssrf_check(url)` — IPv4 (loopback / private / link-local / metadata / unspec / multicast; since 3.4.7 also 100.64.0.0/10 shared, 192.0.0.0/24 and 240.0.0.0/4 reserved) | `host::_ssrf_classify_v4` | ✅ (1.8.0; 3.4.7) — 198.18.0.0/15 and the documentation ranges are allowed on purpose (fake-IP DNS proxies answer inside 198.18/15; nothing routes the documentation ranges) |
| IPv6 literals parsed to bytes (RFC 4291 §2.2, zone id set aside) and classified on them: `::`, `::1`, IPv4-mapped / -compatible, NAT64 (`64:ff9b::/96`, `64:ff9b:1::/48`), 6to4 carrying an internal IPv4, `fe80::/10`, `fec0::/10`, `fc00::/7`, `ff00::/8` | `host::_ssrf_parse_ipv6` + `ssrf_classify_ipv6_bytes` | ✅ (1.9.1; bytes since 3.4.7) — a literal that does not parse is refused. Through 3.4.6 a text matcher missed spellings such as `0:0:0:0:0:0:0:1` and `fe9f::1` and passed anything it did not recognise. The parser is checked against cyrius's own (`lib/tls_hostid.cyr`) in `tests/bote_host.tcyr` and `fuzz/ssrf_ipv6.fcyr` |
| Hostname blocklist (`localhost`, `metadata.google.internal`, `metadata`) — case-insensitive | `host::_ssrf_classify_hostname` | ✅ |
| `user:pass@` userinfo stripping before classification | `host::_ssrf_extract_host` | ✅ |
| Scheme gate — only `http://` / `https://` | same | ✅ |
| DNS resolution (catch `127.0.0.1.nip.io` style bypasses) — the address a name resolves to is classified where sandhi connects | `web_tools::_web_ssrf_resolve` via `sandhi_client_set_resolver` | ✅ (3.4.4) — `web_fetch` only; `web_search` keeps stock resolution for the operator's own `BOTE_SEARXNG_URL`. The hook uses the same classifiers as the URL check, so 3.4.7's ranges apply to resolved addresses too |

---

## Test Coverage (bote 3.4.5)

| Scope | Count | Source |
|---|---|---|
| **Protocol conformance by spec clause** — JSON-RPC 2.0 §4 / §5 / §4.1 / §5.1 / §6 (incl. the spec's own examples), MCP initialize + all four revisions, ping, cancellation, capability gating, pagination cursors, tools/list, the tools/call protocol-vs-execution error split, prompts, resources, completion, control bytes in a tool result; every response checked to be valid JSON and a §5 Response object | **119** | `tests/conformance.tcyr` (3.4.5) |
| Core — error / protocol / jsonx / registry / prompts / resources / completion / dispatch / codec / schema / stream / session / HTTP helpers / discovery / bridge / events / audit / the per-request DEBUG line, the string escaper | **496** | `tests/bote.tcyr` |
| Bearer + allowlist + JWT + PKCE validators | **45** | `tests/bote_auth.tcyr` |
| Content blocks + annotations | **25** | `tests/bote_content.tcyr` |
| `fs_tools` path safety + root confinement + control bytes as valid JSON | **34** | `tests/bote_fs_tools.tcyr` |
| HostRegistry + IPv4 / IPv6 SSRF (URL and resolved address; each 3.4.7 range named) + the IPv6 parser against cyrius's + hot-reload | **197** | `tests/bote_host.tcyr` |
| JWT HS256 — signature, exact `alg`, `exp` (mutation-proven) | **53** | `tests/bote_jwt.tcyr` |
| `libro_tools` incl. tampered-chain decode + populated-chain proof | **38** | `tests/bote_libro_tools.tcyr` |
| Operational logging over sakshi | **50** | `tests/bote_log.tcyr` |
| RFC 7636 PKCE-S256 | **17** | `tests/bote_pkce.tcyr` |
| Sandbox runner adapter | **13** | `tests/bote_sandbox.tcyr` |
| Streamable HTTP — event ids, resumption, per-session outbound, drain selection | **53** | `tests/bote_streamable.tcyr` |
| Unix transport — sockaddr, accept-error policy, backoff, deadlines, closed-peer write | **68** | `tests/bote_transport_unix.tcyr` |
| `web_tools` — scheme guard, SSRF guard (URL and resolved address), HTML stripper, entities | **56** | `tests/bote_web_tools.tcyr` |
| WebSocket config + wire-up + the send-deadline drop | **21** | `tests/bote_ws.tcyr` |
| **Total assertions** | **1285** | + `bote_core_only_smoke.tcyr` (drift guard over `dist/bote-core.cyr`, exit-code driven); all of them also run under `qemu-aarch64` |
| Hot-path benchmarks | **15** | `tests/bote.bcyr` |
| Fuzz harnesses | **6** | `fuzz/*.fcyr` (in CI since 3.3.8; `jsonx_valid.fcyr` since 3.4.5, `ssrf_ipv6.fcyr` since 3.4.7) |
| End-to-end transport round trips | 6 (stdio, HTTP, Unix, bridge, streamable, WS) | CI's "Transport smoke" step plus a per-release stdio probe on the built binary |

The Rust archive's conformance suite (`tests/conformance.rs`, tag `0.92.0`, 44 scenarios) is
ported as `tests/conformance.tcyr` (3.4.5). Its scenarios are there, with the JSON-RPC and MCP
requirements it never covered, and where the Rust suite and the spec disagree (an unknown tool's
code, a schema violation's shape) the spec wins. The bridge's error-envelope scenario lives in
`tests/bote.tcyr`, which links the bridge.

---

*Audit method: manual comparison against [MCP spec 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25), with every method row probed against the built binary over stdio (the 3.3.12 sweep is how the `ping`, `2025-06-18` and `resources/templates/list` rows were found). For the explicit security-property table see [SECURITY.md](../SECURITY.md).*
