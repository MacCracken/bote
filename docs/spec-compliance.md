# MCP Spec Compliance

> **Spec Version**: 2025-11-25 (default), 2026-07-28 (stateless core) | **Bote Version**: 3.6.0 (cyrius 6.7.18) | **Last Audited**: 2026-11-01 ([closeout audit](audit/2026-11-01-closeout.md))
>
> ⚠ **bote speaks the current revision, 2026-07-28, beside the older ones.** [That section](#mcp-2026-07-28-stateless-path) lists what is in and what is not. Every other table here is against 2025-11-25.

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
| `2026-07-28` | ✅ stateless, on every transport (3.5.0; HTTP 3.5.1). Gaps listed below |

The first four are negotiated via `initialize`. The server picks the highest
mutually supported version, so `initialize` never lands on `2026-07-28`, which
has no `initialize`. A request selects that revision itself, in `_meta`, and one
process serves both eras. The legacy list lives in exactly one place —
`_mcp_is_supported` (`src/dispatch.cyr`); `session.cyr`'s header validator defers to it. Through
3.3.12 there were two copies, and the drift they allowed was asymmetric: a
version missing from the header validator 400s on every HTTP-family transport
while stdio negotiates it happily. Four assertions now pin the two readers
against each other per version.

JSON-RPC batching was removed from the spec at `2025-06-18`; bote keeps accepting
batch arrays on every version (a superset is harmless to a client that never
sends one, and refusing them per-version would break the earlier clients that
legitimately batch).

---

## MCP 2026-07-28, stateless path

Selected per request by `params._meta["io.modelcontextprotocol/protocolVersion"]`. A request without
that key, and every `initialize`, takes the legacy path unchanged. Handlers are the same on both. On HTTP
the header and `_meta` must agree first; a POST with neither is treated as `2025-03-26`, as the spec allows.
Asserted in `tests/conformance.tcyr`, M1 to M75, through `codec_process_message`, and in
`tests/bote_http_era.tcyr` on the real HTTP handlers.

| Spec Requirement | Status |
|---|---|
| Every request carries its version and client capabilities in `_meta`; a required field missing or of the wrong type is `-32602` | ✅ (3.5.0) |
| An unsupported version is `-32022`, with `data.supported` and `data.requested` | ✅ (3.5.0) — the supported list is the same on both eras. `requested` is escaped, since a client wrote it |
| An older revision named in `_meta` is served on the legacy path | ✅ (3.5.0) |
| `server/discover`, which servers MUST implement: `resultType`, `supportedVersions`, `capabilities`, `ttlMs`, `cacheScope`. A discover request without `_meta` is `-32602` | ✅ (3.5.0) |
| `resultType: "complete"` on every result | ✅ (3.5.0) — added to what a handler returned, and left alone if the handler set it |
| `ttlMs` + `cacheScope` on `server/discover`, the `tools`, `prompts` and `resources` lists, `resources/templates/list` and `resources/read` | ✅ (3.5.0) — `0` and `"private"`. A longer lifetime needs change notification to invalidate it, and that is `subscriptions/listen`, roadmap 3.7.x |
| `serverInfo` in `result._meta["io.modelcontextprotocol/serverInfo"]` | ✅ (3.5.0) |
| A missing resource is `-32602`, never `-32002` | ✅ (3.5.0) — `-32002` stays on the legacy path |
| `ping` and `initialize` are gone | ✅ (3.5.0) — a modern `ping` is `-32601`. An `initialize` selects the legacy path whatever `_meta` it carries, so it never lands on a revision that has no `initialize` |
| No `listChanged` advertised | ✅ (3.5.0) — the modern era has no way to deliver it until `subscriptions/listen` |
| `tools/list` in a deterministic order | ✅ registration order, in both eras |
| Every POST: `MCP-Protocol-Version` and `Mcp-Method`, and `Mcp-Name` on `tools/call`, `resources/read` and `prompts/get`. Missing, invalid or disagreeing with the body is `400` + `-32020` | ✅ (3.5.1) `src/http_era.cyr`, on `bote http`, `bote streamable` and (3.5.2) `bote bridge`. Names are case-insensitive, values case-sensitive. A field on two lines combines into a value that matches nothing, so it is refused (3.5.2) |
| `Mcp-Name` in the `=?base64?…?=` sentinel, decoded before the comparison | ✅ (3.5.1) lower-case markers only, canonical Base64 only |
| An unsupported version is `400` + `-32022`; a method bote does not implement is `404` + `-32601`; a `_meta` missing a required field is `400` + `-32602` | ✅ (3.5.1, corrected at 3.5.2) that clause is about `_meta`. Every other `-32602` and `-32603` is in-band on a `200`: the spec gives them no status, and the official client reads a JSON-RPC error out of a `200` or a `400`, not out of a `500` |
| A notification is `202` with no body | ✅ (3.5.1), in both eras. It was `204` through 3.5.0, which 2025-11-25 does not allow either |
| One message per POST | ✅ (3.5.1) a batch, a response or a non-object is `400`, a batch that names this revision included |
| Responses in `application/json` | ✅ the spec lets the server choose per request. SSE arrives with `subscriptions/listen` |
| `GET` and `DELETE` are `405` to a request naming this revision, with `Allow` | ✅ (3.5.1). A legacy `GET` keeps its stream |
| `Mcp-Session-Id` and `Last-Event-ID` are ignored, and no session is minted | ✅ (3.5.1) |
| `Mcp-Param-{Name}`, for a tool that declares `x-mcp-header` | ✅ (3.5.2) a value in the body needs its header, decoded and equal, integers compared by value and booleans as `true` / `false`. Missing, malformed, mismatched or undecodable is `400` + `-32020`. A header with no value in the body is refused too, which the spec's table does not ask for and no conforming client meets. A header the tool does not declare is ignored |
| An `x-mcp-header` that breaks the rules makes the tool invalid | ✅ (3.5.2) registration refuses it, since a client would drop the tool: an empty or non-token name, a type other than string, integer or boolean, a repeat ignoring case, or anywhere but a chain of `properties` keys |
| Integers mirrored in a header stay in the safe range | ✅ (3.5.2) a value past ±(2^53 − 1) is refused with `-32020`, even with its header |
| `-32021` for a missing client capability | ⏳ no request bote serves needs one yet; capabilities are required and not otherwise read |
| A third-party client | ✅ (3.5.3) `@modelcontextprotocol/client` 2.4.0 connects and uses every capability bote has, 158 checks on `http`, `streamable` and stdio in both eras and `bridge` in the modern one, and (3.6.0) its whole OAuth flow. `./scripts/conformance.sh` repeats it |
| `@modelcontextprotocol/conformance`, by its frozen requirement list | ⏳ (3.5.3) **25 of 37** on 2026-07-28, **21 of 30** on 2025-11-25, the same on `http` and `streamable`. What fails is a feature bote lacks: `InputRequiredResult`, progress, sampling and elicitation, logging, subscriptions. `tests/fixtures/conformance-expected-<revision>.yaml` lists each, and why |
| `subscriptions/listen` and the rest of its notifications | ⏳ roadmap 3.7.x |

---

## Core Protocol

| Spec Requirement | Module | Status |
|---|---|---|
| JSON-RPC 2.0 — request, response, notification, batch | `protocol` + `codec` | ✅ |
| Invalid JSON is `-32700`; valid JSON that is not a request is `-32600` (§5.1) | `jsonx_valid` + `codec` | ✅ (3.4.5) — the whole message is syntax-checked (RFC 8259, nesting ≤ 64) before anything reads it. Through 3.4.4 `not valid json` was `-32600`, the spec's truncated-object example was dispatched as a notification, and `"id":01` was echoed back as invalid JSON |
| `id` is a string, a number or null; `params`, if present, is an object or an array (§4) | `codec::_cdc_admit` | ✅ (3.4.5) — otherwise `-32600`. An explicit `"params":null` is read as absent. A null id is answered, though MCP forbids a client to send one |
| Spec error codes (-32700, -32600, -32601, -32602, -32002, -32000, -32003, -32603, -32800) | `error` + `dispatch` | ✅ (`-32002` since 3.4.5) |
| Every response is valid JSON: strings escaped per RFC 8259 §7, control bytes included, and valid UTF-8 (§8.1), invalid bytes as U+FFFD (3.4.15); every handler's result on one line of valid JSON (3.4.15) | `jsonx::bote_json_escape`, `dispatch::_dsp_checked_result` | ✅ (3.4.6) — through 3.4.5 the escaper left control bytes other than LF / CR / TAB raw, so a tool result holding an ESC or a BEL (an ANSI-coloured log through `fs_read`) was not JSON, and an embedder's `serverInfo` was not escaped at all. Asserted by the conformance suite's standing valid-JSON check (J15, F12) and a fuzz property |
| `initialize` handshake (serverInfo + capabilities + version negotiation; `serverInfo` configurable via `dispatcher_set_server_info`) | `dispatch` | ✅ |
| `ping` → empty `{}` result (spec: the receiver *MUST* respond promptly) | `dispatch` | ✅ (3.3.13) — answered ahead of every capability gate, so a bare dispatcher replies; emits no audit or event record (liveness, not activity). bote answers pings, it does not originate them |
| `notifications/initialized` / `notifications/cancelled` — accepted, never answered | `codec` (no response to any notification) | ✅ (`cancelled` is ignored: there is no in-flight work to cancel until threaded dispatch, 3.5.x) |
| `tools/list` with full `inputSchema` | `dispatch` + `registry` | ✅ |
| `outputSchema`; `title` and `icons`; `instructions` in `initialize` | — | ⏳ not implemented, roadmap 3.8.x. `structuredContent` goes out on bote's own libro tools |
| Pagination: `nextCursor` omitted when there is no next page; an invalid `cursor` is `-32602` | `dispatch` | ✅ (3.4.5) — bote returns every item in one page and never issues a cursor, so any `cursor` a client sends is invalid and gets `-32602`, the spec's code for one, on `tools/list`, `prompts/list`, `resources/list` and `resources/templates/list`. A null `cursor` is read as absent. Through 3.4.4 a cursor was ignored and the first page sent again, which a client concatenating pages would take as duplicates |
| `tools/call` with arguments + version selection | `dispatch` + `schema` | ✅ |
| `tools/call` error split: an unknown tool or a malformed CallToolRequest (missing / empty / non-string `name`, non-object `arguments`) is a protocol error `-32602`; arguments failing the tool's `inputSchema` are a tool execution error, a result with `isError: true` | `dispatch` | ✅ (3.4.5) — per the tools spec's "Error Handling" and SEP-1303. Through 3.4.4 an unknown tool was `-32601` and a schema violation `-32602`. The dispatcher has no per-session negotiated version, so this split applies to every revision. 2025-06-18 already listed "invalid input data" as a tool execution error |
| `tools/call` result is always a CallToolResult | `dispatch::_dsp_coerce_result` | ✅ (3.4.5; 3.4.8) — a handler that returns nothing or invalid JSON is a failed call (`isError: true`); bare JSON arrives as a text block, one content block is lifted into `content`, and a CallToolResult passes through. A result carrying `isError` is audited and published as a failure. Through 3.4.7 bare JSON went out as the result, so bote's own libro tools answered `{"ok":true}` |
| `prompts/list` + `prompts/get` (capability advertised iff a `PromptRegistry` is present) | `dispatch` + `prompts` | ✅ — an unknown name, a missing required argument and non-object `arguments` are `-32602` (the last two since 3.4.5) |
| `resources/list` + `resources/read` (capability advertised iff a `ResourceRegistry` is present) | `dispatch` + `resources` | ✅ — an unserved URI is `-32002` with `data.uri`, as in the resources spec's example (3.4.5; was `-32602`) |
| `resources/templates/list` and reading through a template (optional in the spec) | `dispatch` + `resources` | ✅ (3.5.3) `dispatcher_register_resource_template`. RFC 6570 level 1: `{name}` is one non-empty value with no `/`, `?` or `#`, one to a segment. An exact resource wins, and the handler owns percent-decoding. Gated like `resources/*`, and an empty list with none registered |
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
| Tool names follow the spec's guidance (1–128 characters from `A-Z a-z 0-9 _ - .`) and inputSchema is an object schema ("MUST be a valid JSON Schema object") | `registry::registry_check_tool_def` | ✅ (3.4.8) — checked at every registration; a refused definition is not registered, an ERROR line says why, and the call returns a BoteError. It refuses the `Str`-for-cstr registrations the consumer audit found in t-ron and phylax |

## Schema Validation

| Spec Requirement | Module | Status |
|---|---|---|
| Input schemas compiled at registration, so every tool's arguments are validated | `registry::registry_register` → `compiled_compile` | ✅ (3.4.8) — through 3.4.7 nothing compiled a schema, so only `required` was ever checked in production |
| JSON Schema subset — string, number, integer, boolean, array, object, ANY | `schema::SchemaTypeTag` | ✅ |
| String enum constraint | `prop_enum_values` | ✅ |
| Numeric bounds (`minimum`, `maximum`) for number + integer | `prop_min` / `prop_max` | ✅ — exponents read (3.4.8); a fractional value is compared as itself (3.4.15), and a fractional bound is truncated |
| `integer` per JSON Schema 2020-12 (a zero fractional part counts: `5.0`, `1e2`) | `schema::_sch_num_parse` | ✅ (3.4.8; any `.` was refused before) |
| Required field check | `compiled_required` | ✅ |
| Recursive nested object validation | `_validate_against` | ✅ |
| Array `items` typing | `prop_items` | ✅ |
| Multi-violation reporting (collect all, don't fail-fast) | `compiled_validate` returns vec | ✅ |
| Permissive on extra (unknown) fields | by design | ✅ |
| Schema `false` allows nothing | `SCH_NEVER` | ✅ (3.4.15; it allowed anything) |
| Keywords outside the subset (`anyOf`, `oneOf`, `allOf`, `not`, `$ref`, `pattern`, length and item bounds, `const`, type arrays, non-string enums) | `registry::_reg_warn_unenforced` | 🟡 advertised in `tools/list`, not enforced; a WARN at registration names each (3.4.15). Roadmap 3.8.x |

## Session Management

| Spec Requirement | Module | Status |
|---|---|---|
| `MCP-Protocol-Version` header parsing + validation | `session::bote_protocol_known` + `http_era::bote_http_check_version` | ✅ — absent is taken as `2025-03-26`, as the spec allows, unless `require_protocol` is set. Through 3.5.0 `bote streamable` refused a POST without it, `initialize` included (3.5.1) |
| `MCP-Session-Id` — server-issued on `initialize`, validated on subsequent | `session::SessionStore` + per-transport `_check_session` | ✅ HTTP / streamable / bridge |
| Session creation on initialize (auto, returned in response header) | `session_store_create` + handler hook | ✅ |
| Session timeout + pruning (configurable; default 1h) | `session_store_validate` + `session_store_prune_expired` | ✅ (3.4.15) — through 3.4.14 nothing checked a session's age and nothing called the pruner, so a session never expired. The store now holds at most `BOTE_SESSION_MAX` (4096) and answers 503 when full of live sessions; a session is minted only after `initialize` succeeds |
| Random 128-bit SID, 32-hex encoded, from the kernel CSPRNG via `random_bytes()` (`getrandom(2)`); the process **refuses to mint a session ID** if that fails rather than fall back to anything guessable | `_gen_session_id` | ✅ (3.2.0; was `/dev/urandom` by raw syscall) |

## Security

| Spec Requirement | Module | Status |
|---|---|---|
| Origin validated on every HTTP-family transport (DNS rebinding protection — 403 on rejection); `BOTE_ALLOWED_ORIGINS` sets the list | `session::bote_origin_ok` + per-transport `_check_origin` | ✅ (3.5.2) with no list the server's own loopback address passes, an Origin of `http://` + the request's `Host` on `localhost`, `127.0.0.1` or `[::1]`, which the conformance suite requires and which no rebinding page can send. (3.4.15) — a present Origin must be listed, and with no list every present one is refused. Through 3.4.14 no list meant no check, the binary had no way to set one, and `bote bridge` allowed `*` |
| Wildcard `*` and exact-match origins | same | ✅ |
| Strict mode (empty allow-list rejects all) | same | ✅ |
| CORS preflight (OPTIONS / + 3 ACA-* headers) | `bridge::_bridge_cors_headers` | ✅ |
| **Bearer-token middleware (RFC 6750)** — opt-in fn-pointer + ctx validator on every HTTP-family transport | `auth::auth_bearer_check` + per-transport wiring | ✅ (1.9.0) |
| **`WWW-Authenticate: Bearer realm="mcp"` on 401**, with no metadata configured | `auth::auth_send_challenge` | ✅ |
| **Built-in validators**: `auth_validator_allow_all` (dev), `auth_validator_allowlist` (vec membership) | `auth` | ✅ |
| **`BOTE_BEARER_TOKENS` env var** wires an allowlist validator across all four HTTP-family transports at startup | `main::_bote_bearer_from_env` | ✅ (1.9.1) |
| **HTTP body-length clamp** — `clen = min(clen, n - bo)` so a lying Content-Length can't make `memcpy` read past the request buffer | `transport_http`, `transport_streamable`, `bridge` | ✅ (1.5.1) |
| **SSRF guard for outbound URL fetches** — IPv4 + IPv6 blocklists for loopback / private / link-local / cloud-metadata | `host::ssrf_check` | ✅ (1.8.0 / 1.9.1) |
| **JWT HS256 verifier** (RFC 7519 / 7515) — `alg` read as an exact JSON field (not a substring scan), constant-time HMAC compare, the signature segment canonical base64url, `exp` enforced *after* the signature verifies with no leeway, malformed `exp` rejects, `exp` absent accepts; `auth_validator_jwt_hs256` plugs into the bearer middleware | `jwt::jwt_verify_hs256` | ✅ (2.2.0; `exp` + exact `alg` at 3.2.0; ships in `dist/bote.cyr` since 3.2.0) |
| **RFC 7636 PKCE-S256** — `pkce_code_verifier` (CSPRNG) + `pkce_code_challenge_s256` | `pkce` | ✅ (2.3.0; ships in `dist/bote.cyr` since 3.2.0) |
| **Pluggable sandbox runner** — fn-pointer + ctx adapter for tool handlers (kavach-shaped), noop default, error envelope on a null runner | `sandbox::sandbox_run` | ✅ (2.1.0; ships in `dist/bote.cyr` since **3.3.13** — before that it was in `src/` only and reached no consumer). Not in `dist/bote-core.cyr` |
| Asymmetric JWT (RS256 / ES256) | — | ⏳ roadmap 3.6.x |
| OAuth 2.0 Protected Resource Metadata (RFC 9728): a document at `/.well-known/oauth-protected-resource` + the resource's path, with `resource` and at least one authorization server, readable with no token | `oauth_metadata` | ✅ (3.6.0) on `http`, `streamable` and `bridge`, from `bote_prm_new` or `BOTE_RESOURCE_URL` + `BOTE_AUTH_SERVERS` + `BOTE_AUTH_SCOPES`. `resource` is the operator's and never comes from a request (RFC 9728 §7.3). `scopes_supported`, `bearer_methods_supported` and `resource_name` go with it; `offline_access` is refused |
| `resource_metadata` and `scope` in the `WWW-Authenticate` of a 401, and `error="invalid_token"` when a token was refused | `auth::auth_bearer_check_c` | ✅ (3.6.0), which meets both of MCP's discovery mechanisms at once. The official TypeScript client completes its OAuth flow on this, in both eras |
| `403` with `error="insufficient_scope"`, audience validation (RFC 8707) | — | ⏳ roadmap 3.6.x |

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

## Test Coverage (bote 3.5.1)

| Scope | Count | Source |
|---|---|---|
| **Protocol conformance by spec clause** — JSON-RPC 2.0 §4 / §5 / §4.1 / §5.1 / §6 (incl. the spec's own examples), MCP initialize + all four revisions, ping, cancellation, capability gating, pagination cursors, tools/list, the tools/call protocol-vs-execution error split, prompts, resources, completion, control bytes in a tool result, every handler-result shape, typed arguments; the 2026-07-28 stateless path; every response checked to be valid JSON and a §5 Response object | **203** | `tests/conformance.tcyr` (3.4.5; 3.5.0) |
| Core — error / protocol / jsonx / registry / prompts / resources / completion / dispatch / codec / schema / stream / session / HTTP helpers / discovery / bridge / events / audit / the per-request DEBUG line, the string escaper, registration checks, numbers per 2020-12 | **668** | `tests/bote.tcyr` |
| Bearer + allowlist + JWT + PKCE validators | **55** | `tests/bote_auth.tcyr` |
| Content blocks + annotations | **25** | `tests/bote_content.tcyr` |
| `fs_tools` path safety + root confinement + control bytes as valid JSON + schemas and annotations; decoded names, truncation, fs_mkdir | **56** | `tests/bote_fs_tools.tcyr` |
| HostRegistry + IPv4 / IPv6 SSRF (URL and resolved address; each 3.4.7 range named) + the IPv6 parser against cyrius's + hot-reload | **203** | `tests/bote_host.tcyr` |
| JWT HS256 — signature, exact `alg`, `exp` (mutation-proven), canonical signature on every toolchain | **59** | `tests/bote_jwt.tcyr` |
| `libro_tools` incl. tampered-chain decode + populated-chain proof + CallToolResults, the retention guard, RFC 3339 timestamps, schemas, integers as validated, audit identity | **73** | `tests/bote_libro_tools.tcyr` |
| Operational logging over sakshi | **50** | `tests/bote_log.tcyr` |
| RFC 7636 PKCE-S256 | **17** | `tests/bote_pkce.tcyr` |
| Sandbox runner adapter | **13** | `tests/bote_sandbox.tcyr` |
| Streamable HTTP — event ids, resumption, per-session outbound, drain selection | **53** | `tests/bote_streamable.tcyr` |
| Unix transport — sockaddr, accept-error policy, backoff, deadlines, closed-peer write, overlong lines, socket-path refusals | **76** | `tests/bote_transport_unix.tcyr` |
| `web_tools` — scheme guard, SSRF guard (URL and resolved address), HTML stripper, entities, schemas and annotations, outbound deadline | **66** | `tests/bote_web_tools.tcyr` |
| MCP 2026-07-28 on HTTP, each clause on the real handlers over a socketpair | **517** | `tests/bote_http_era.tcyr` |
| WebSocket config + wire-up + the send-deadline drop + the upgrade's version check | **24** | `tests/bote_ws.tcyr` |
| **Total assertions** | **2158** | + `bote_core_only_smoke.tcyr` (drift guard over `dist/bote-core.cyr`, exit-code driven); all of them also run under `qemu-aarch64` |
| Hot-path benchmarks | **18** | `tests/bote.bcyr` |
| Fuzz harnesses | **7** | `fuzz/*.fcyr` (in CI since 3.3.8; `jsonx_valid.fcyr` since 3.4.5, `ssrf_ipv6.fcyr` since 3.4.7) |
| End-to-end transport round trips | 6 (stdio, HTTP, Unix, bridge, streamable, WS) | CI's "Transport smoke" step plus a per-release stdio probe on the built binary |

The Rust archive's conformance suite (`tests/conformance.rs`, tag `0.92.0`, 44 scenarios) is
ported as `tests/conformance.tcyr` (3.4.5). Its scenarios are there, with the JSON-RPC and MCP
requirements it never covered, and where the Rust suite and the spec disagree (an unknown tool's
code, a schema violation's shape) the spec wins. The bridge's error-envelope scenario lives in
`tests/bote.tcyr`, which links the bridge.

---

*Audit method: manual comparison against [MCP spec 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25), with every method row probed against the built binary over stdio (the 3.3.12 sweep is how the `ping`, `2025-06-18` and `resources/templates/list` rows were found). For the explicit security-property table see [SECURITY.md](../SECURITY.md).*
