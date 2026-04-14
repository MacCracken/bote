# Changelog

All notable changes to bote are documented here.

## [1.0.0] — 2026-04-13 — Stable cyrius MCP core

Bote's cyrius implementation is **stable**. The MCP protocol surface, registry,
dispatcher, schema validation, sessions, discovery, and four transports
(stdio, HTTP, Unix socket, TS bridge) are feature-complete and verified:

- **298 unit assertions** all passing
- **10 hot-path benchmarks**, all sub-10µs on x86_64
- **4 fuzz harnesses**, ~330 calls across malformed and edge-case inputs, no crashes
- **End-to-end smoke tests** for stdio (pipe), HTTP (curl), Unix socket (Python AF_UNIX), bridge (curl + CORS)

The data shapes for `JsonRpcRequest`, `JsonRpcResponse`, `ToolDef`, `ToolSchema`,
`ToolAnnotations`, `CompiledSchema`, `BoteError`, `McpSession`, and the four
`HttpConfig` / `BridgeConfig` flavours are **frozen** — additive changes only
within the 1.x series.

### What's in 1.0.0

| Area | Status |
|---|---|
| JSON-RPC 2.0 (request, response, notification, batch) | ✅ |
| MCP `initialize` / `tools/list` / `tools/call` | ✅ |
| Tool registry with versioning + deprecation + annotations | ✅ |
| Compiled schema (type/enum/bounds/nested object/array items, multi-violation) | ✅ |
| `JsonRpcError` codes — full spec mapping | ✅ |
| Session management (create/validate/prune, MCP-Session-Id header) | ✅ |
| Origin allow-list + protocol-version header validation | ✅ |
| stdio transport | ✅ |
| HTTP/1.1 transport with middleware | ✅ |
| Unix domain socket transport | ✅ |
| TypeScript bridge (CORS + MCP envelope wrap) | ✅ |
| Discovery (data layer + pluggable publish_fp) | ✅ |
| Streaming primitives (ProgressUpdate, CancellationToken) | ✅ data layer |

### Post-1.0 extensions (1.x minor bumps)

These are additive — none change existing API shapes.

| Module | Status |
|---|---|
| `src/audit.cyr` + `LibroAudit` adapter | **Ready to port** — libro v1.0.3 available via `[deps.libro] path = "../libro"` |
| `src/events.cyr` + `MajraEvents` adapter | **Ready to port** — majra v2.2.0 available via `[deps.majra] path = "../majra"` |
| `src/discovery.cyr` wire-up to majra pubsub | **Ready to port** — depends on events |
| `src/libro_tools.cyr` (5 built-in audit tools) | **Ready to port** — depends on audit + libro |
| `src/sandbox.cyr` + kavach integration | Wait — kavach v2-arch hardening in flight |
| `src/host.cyr` (content blocks, host registry) | Ready (no AGNOS dep) |
| `src/auth.cyr` (OAuth 2.1 / PKCE / bearer) | Ready (no AGNOS dep) |
| `src/transport_streamable.cyr` (POST + SSE single endpoint) | Ready (rolls SSE on top of `transport_http`) |
| `src/transport_ws.cyr` (server-side WebSocket) | Cyrius `lib/ws.cyr` is client-only; needs server handshake + frame unmasking written |
| Threaded streaming dispatch | Needs `lib/thread.cyr` MPSC wired into `dispatcher_dispatch_streaming` |

### Versioning policy from here

Pre-1.0 used `0.D.M` (day.month). From 1.0.0 forward, **standard SemVer**:
- **Major** — break a frozen data shape or remove a public function.
- **Minor** — add a module / function / config option.
- **Patch** — fix bugs, refactor internals, improve diagnostics.

### Cyrius toolchain pin

Built and tested against cyrius **4.4.0** (`cyriusly use 4.4.0`).

---

## [0.1.1] — 2026-04-13 — Bridge + cyrius 4.4.0 + review punch list

### Added
- **`src/bridge.cyr`** — TypeScript-bridge HTTP transport: CORS preflight (`OPTIONS /`), `GET /health`, `POST /` JSON-RPC dispatch with MCP-envelope wrapping for `tools/call` results. `wrap_tool_result` (passthrough if already shaped, else wraps text), `wrap_error_result` (adds `isError: true`).
- **CLI**: `./build/bote bridge [port]` (default 8391).
- 29 new unit assertions: bridge wrappers, CORS origin selection, `bridge_process_message` round-trips, schema bounds at exact `min` / `max`, codec pure-notification batch.

### Fixed (review punch list)
- **`src/jsonx.cyr`**: `key_len_actual == klen && memeq(...)` was unsafe because cyrius `&&` doesn't short-circuit — `memeq` was called on truncated input. Now nested as separate `if`s.
- **`src/transport_http.cyr`**: when `Content-Length` was absent and `body_off > n` (malformed request), `clen = n - body_off` could be negative → `memcpy` UB. Now guarded.
- **`src/schema.cyr`**: `_sch_parse_int` replaced `i = i + 999999` marker-hack with proper `break` (per-block scoping now works in cyrius 4.4.0).
- **`src/transport_http.cyr`**: `http_find_header` similarly cleaned — replaced `vs = vs - 0; line_start = headers_end; vs = vs - 0;` marker hack with structured loops + `break`.

### Verified against cyrius 4.4.0 (`cyriusly install 4.4.0 && cyriusly use 4.4.0`)
- ✅ `\r` escape now emits CR (13) — fixed upstream
- ✅ Per-block `var` shadowing now works — fixed upstream
- ❌ `&&` / `||` short-circuit still missing — workarounds retained
- ➕ DCE now available via `CYRIUS_DCE=1` at build time

`docs/cyrius-feedback.md` updated with v4.4.0 verification status against each repro.

### Performance
Bench numbers unchanged from 0.1.0 — bridge adds a thin envelope-wrap layer with no measurable overhead on the hot dispatch path.

---

## [0.1.0] — 2026-04-13 — Cyrius port baseline

### Breaking
- **Language switch**: bote moved from Rust to Cyrius. The Rust source is preserved under `rust-old/` for reference and recovery. Version reset to `0.1.0` to mark the new lineage.
- **API change**: idiomatic Cyrius — module-prefixed function APIs (`registry_register`, `dispatcher_dispatch`, `codec_process_message`) over offset-addressed structs (`store64`/`load64`). No traits, generics, async, or borrow checking. Handler functions are i64 function pointers (`fn h(args_cstr) → result_cstr`).

### Added
- **`src/error.cyr`** — `BoteErrTag` enum (12 variants), `bote_err_rpc_code`, `bote_err_format`, schema-violation list support.
- **`src/protocol.cyr`** — `JsonRpcRequest` / `JsonRpcResponse` / `JsonRpcError` with raw-JSON-literal id/params/result/data slots.
- **`src/jsonx.cyr`** — Nested-aware JSON value extractor (`jsonx_get_raw`, `jsonx_get_str`, `jsonx_has`, `jsonx_is_object`). Handles nested objects, arrays, escaped strings; needed because `lib/json.cyr` is flat-only.
- **`src/registry.cyr`** — `ToolDef` (with `version`, `deprecated`, `annotations`, `compiled` slots), `ToolSchema`, `ToolAnnotations` (presets `read_only` / `destructive`), `ToolRegistry` (insertion-ordered, hashmap-indexed). Versioned tools, deprecation, validate-by-required-fields fallback.
- **`src/dispatch.cyr`** — `Dispatcher`, sync handler dispatch, `initialize` / `tools/list` / `tools/call` routing, MCP protocol-version negotiation, `validate_tool_name` (project_tool format, 256 char max), dynamic register/deregister.
- **`src/codec.cyr`** — `codec_parse_request`, `codec_serialize_response`, `codec_process_message` (single + batch + notification + error responses), JSON-message escaping reused from dispatch.
- **`src/schema.cyr`** — `CompiledSchema` with full type-checking (`string`, `number`, `integer`, `boolean`, `array`, `object`, `Any`), enum constraints, numeric bounds, recursive nested objects + array items, multi-violation reporting. `tool_def_with_compiled` slot wires it into `registry_validate_params`.
- **`src/stream.cyr`** — `CancellationToken`, `ProgressUpdate`, `ProgressSender`, `StreamContext`, `progress_notification` JSON builder. (Thread integration deferred.)
- **`src/session.cyr`** — `SessionStore` (hex-encoded 16-byte SIDs from `/dev/urandom`), `validate_protocol_version`, `validate_origin` (wildcard `*`, exact match, strict mode).
- **`src/transport_stdio.cyr`** — Line-oriented JSON-RPC over stdin/stdout, 128KB heap-allocated buffer, partial-line shifting.
- **`src/transport_http.cyr`** — HTTP/1.1 server (`POST /mcp` → JSON-RPC). Origin/MCP-Protocol-Version/MCP-Session-Id middleware. Auto-creates a session on `initialize` and emits the new `MCP-Session-Id` response header. Case-insensitive header lookup. 64KB request buffer.
- **`src/transport_unix.cyr`** — `AF_UNIX` line-oriented transport (own socket-creation code since `lib/net.cyr` is `AF_INET`-only). 128KB per-connection buffer.
- **CLI** — `./build/bote [stdio|http <port>|unix <path>]` selects transport.
- **Tests** — `tests/bote.tcyr` with **251 unit assertions** covering all modules.
- **Benchmarks** — `tests/bote.bcyr` with 10 hot-path benchmarks (all sub-10µs on x86_64).
- **Fuzz** — `fuzz/codec_parse.fcyr`, `fuzz/codec_process.fcyr`, `fuzz/jsonx_extract.fcyr`, `fuzz/schema_validate.fcyr` (~330 fuzzed calls; no crashes).
- **`docs/cyrius-feedback.md`** — language-level issues found during the port.
- `.gitignore` rules for `rust-old/target/` and `/build/`.

### Performance
- `dispatch_initialize` ~2µs avg
- `dispatch_tools_list` ~2µs avg
- `dispatch_tools_call` ~1µs avg
- `jsonx_get_str_flat` 600ns avg
- `jsonx_get_raw_nested` ~1µs avg
- `codec_parse_request` ~2µs avg
- `codec_serialize_response` ~1µs avg
- `codec_process_message` (full pipeline) ~5µs avg
- `validate_compiled_simple` ~1µs avg
- `validate_compiled_nested` ~3µs avg

### Deferred to future cyrius releases
- `bridge` — TypeScript bridge with CORS / MCP envelope wrapping.
- `audit` — libro hash-linked audit chain integration.
- `events` — majra pub/sub event publishing.
- `discovery` — cross-node tool announcements (depends on `events`).
- `sandbox` — kavach tool isolation.
- `host` — MCP hosting layer (content blocks, host registry).
- `libro_tools` — 5 built-in libro audit MCP tools.
- `auth` — OAuth 2.1 / PKCE / bearer-token middleware.
- `transport_ws` — server-side WebSocket (cyrius `lib/ws.cyr` is client-only).
- `transport_streamable` — streamable HTTP (POST + SSE single endpoint).
- Streaming dispatch (needs thread + channel integration).

### Known cyrius-language workarounds applied
- `\r` string escape emits byte `r` (114) instead of CR (13) — built CRLF via `store8`.
- `&&` / `||` operators do not short-circuit — guarded null derefs nested as `if (p != 0) { if (...) { ... } }`.
- No per-block local scoping — distinct names per `fn` body (`req_one`, `rcompiled`, `prog_notif`, etc.).
- Static `var buf[N] >~ 16KB` exhausts the output buffer — large buffers heap-allocated (`var ptr = 0;` global + `ptr = alloc(N);` at startup).

See [docs/cyrius-feedback.md](docs/cyrius-feedback.md) for full reproductions.

---

## Historical (Rust) — preserved under `rust-old/`

## [0.91.0] — 2026-04-02

### Added
- `libro_tools` module (feature: `audit`) — 5 built-in MCP tools for libro audit chain operations:
  - `libro_query` — query audit entries by source, severity, action, agent, min_severity, with limit
  - `libro_verify` — verify chain integrity and return structured `ChainReview` JSON with integrity status, entry count, time range, source/severity/agent distributions (was text-only)
  - `libro_export` — export chain as JSON Lines or CSV
  - `libro_proof` — generate Merkle inclusion proof for an entry by index, returns structured proof JSON with verification status
  - `libro_retention` — apply retention policies (PCI-DSS, HIPAA, SOX, keep_count) and report archived entries (destructive, not read-only)
- `libro_tools::register()` — convenience function to register all 5 libro tools on a dispatcher
- Read-only tools annotated with `ToolAnnotations::read_only()` (MCP 2025-11-25); `libro_retention` is destructive (no annotation)
- `LibroAudit::with_source()` — custom source tag for audit entries (default: `"bote"`)
- `LibroAudit::with_agent_id()` — server agent identity on all entries; `caller_id` from events takes precedence
- `LibroAudit` now uses `append_with_agent()` when caller_id or agent_id is present, populating libro's agent tracking
- 17 libro_tools tests + 8 audit tests (was 8 + 3)
- **HTTP transport middleware**: Origin validation (403), `MCP-Protocol-Version` enforcement (400), `MCP-Session-Id` session lifecycle (404), bearer token extraction with 401/403 responses (feature `auth`)
- **Streamable HTTP transport router**: axum router with POST (JSON-RPC) and GET (SSE stream) on configurable endpoint path, same middleware stack as HTTP, SSE event IDs via `EventIdGenerator`, `Last-Event-ID` resumption via `ResumptionBuffer` replay, `retry:` hint before close, priming event on connect
- `HttpConfig` builder: `with_allowed_origins()`, `with_session_timeout()`, `with_token_validator()` (feature `auth`)
- `StreamableConfig` builder: `with_session_timeout()`, `without_sessions()`
- `TokenValidator` trait (feature `auth`) — consumers implement to validate bearer tokens
- Shared `transport::middleware` module — `check_origin`, `check_protocol_version`, `check_protocol_version_required`, `check_session`, `check_bearer` reused by both transports
- Periodic session pruning via tokio interval in both `http::serve()` and `streamable::serve()`
- `streamable::streamable_router()` — build router without binding a port (for testing)
- 35 new transport middleware tests (origin, protocol version, session enforcement in both transports)
- `cargo vet` supply chain auditing: 156 crates fully audited via trusted imports (mozilla, google, bytecode-alliance, isrg, zcash, ariel-os, embark-studios) and 27 trusted publishers (dtolnay, seanmonstar, Manishearth, epage, fitzgen, kennykerr, Amanieu, BurntSushi, Thomasdezeeuw, cuviper, alexcrichton, carllerche, Darksonn, rust-lang-owner), 66 exempted, CI integration

### Changed
- Upgraded libro dependency from 0.25 to 0.91 (BLAKE3 hashing, serde on all types, key rotation support)
- `HttpConfig` expanded with `allowed_origins`, `session_timeout`, `token_validator` fields
- `StreamableConfig` expanded with `session_timeout` field
- Streamable transport `MCP-Protocol-Version` header is **required** (per MCP 2025-11-25), unlike plain HTTP where it is optional

## [0.90.0] — 2026-04-01

### Fixed
- **JSON-RPC 2.0 spec compliance**: Unknown methods now return `-32601` (Method not found) instead of `-32600` (Invalid Request)
- **Bridge spec compliance**: Error wrapping no longer sets both `result` and `error` on the response (JSON-RPC 2.0 violation)
- `scripts/bench-log.sh`: Added missing `--features bridge` flag

### Performance
- **Notification dispatch 17x faster** (170ns → 10ns): Early-return before lock acquisition when request is a notification
- **Parameter validation 26% faster** (47ns → 35ns): Merged `tools` + `compiled` HashMaps into single `entries` map, eliminating key duplication
- **Schema validation 8% faster** (107ns → 99ns): Same registry merge reduces lookup overhead

### Changed
- `ToolRegistry` internal structure: merged separate `tools` and `compiled` maps into unified `entries` map
- CLAUDE.md: Added task sizing, refactoring guidelines, testing section, documentation structure, CHANGELOG format, module table, stack table

### Added
- 3 new conformance tests: `error_codes_comply_with_spec`, `bridge_error_response_is_spec_compliant`, `registry_deregister_cleans_up_compiled_schema`
- 18 downstream consumers integrated (daimon, agnoshi, t-ron, jalwa, nein, stiva, itihas, varna, selah, hoosh, vidya, rasayan, szal, tarang, vidhana, nazar, mneme, tazama)

## [0.50.0] — 2026-03-26

### Added
- Protocol conformance test suite (41 tests in `tests/conformance.rs`)
- Streaming audit logging — all transports now call `log_tool_call()` after streaming handler completion with timing and success/error status
- `BoteError::SandboxError` variant for sandbox execution failures

### Fixed
- Streaming tool calls in HTTP/SSE, WebSocket, Unix, and stdio transports now correctly produce audit events via `log_tool_call()`
- Added missing doc comment on `Dispatcher::new()`

## [0.25.3] — 2026-03-26

### Added
- Tool sandboxing via kavach (feature `sandbox`)
- `ToolSandboxConfig` with presets: `basic()`, `strict()`, `noop()`
- `SandboxExecutor` for running commands in kavach sandboxes
- `wrap_command()` and `wrap_streaming_command()` handler wrappers
- `Dispatcher::register_sandboxed_tool()` and `register_sandboxed_streaming_tool()` convenience methods
- `BoteError::SandboxError` variant for sandbox execution failures
- Sandbox lifecycle event topics: `bote/sandbox/created`, `bote/sandbox/destroyed`, `bote/sandbox/error`
- Async-sync bridge with `OnceLock<Runtime>` fallback for non-tokio contexts

## [0.24.3] — 2026-03-26

### Added
- Full JSON Schema validation: type checking (string, number, integer, boolean, array, object), enum constraints, numeric bounds, nested object/array validation
- `CompiledSchema` — compile `ToolSchema` into typed representation for fast validation
- Default value injection via `CompiledSchema::apply_defaults()`
- `SchemaType`, `PropertyDef` types in new `schema` module
- `BoteError::SchemaViolation` variant with multiple violation reporting
- Tool versioning: `version` and `deprecated` fields on `ToolDef`
- `ToolDef::with_version()` and `ToolDef::with_deprecated()` builder methods
- `ToolRegistry::get_versioned()`, `list_versions()`, `deprecate()`, `deregister()`
- Version negotiation in `tools/call` dispatch
- Deprecation warnings via tracing + event publishing
- Dynamic tool registration/deregistration via `Dispatcher::register_tool()`, `deregister_tool()`
- Hot-reload: re-registering a tool atomically replaces its handler
- Tool namespacing: `project_tool` format enforcement on dynamic registration
- `TOPIC_TOOL_DEPRECATED` and `TOPIC_TOOL_DEREGISTERED` event topics
- Schema validation, versioning, and dynamic registration benchmarks

### Changed
- `Dispatcher` internals migrated to `RwLock` for thread-safe dynamic registration
- `ToolRegistry::validate_params()` now uses compiled schema for full type validation
- `tools/list` response includes `version` and `deprecated` fields when present

## [0.23.3] — 2026-03-26

### Added
- TypeScript bridge module with CORS and MCP result formatting (feature `bridge`)
- `wrap_tool_result` adapter — converts raw results to SY's `{ content: [{ type, text }] }` envelope
- Bridge CORS preflight handling for cross-origin TypeScript clients
- Cross-node tool discovery via majra pub/sub (feature `discovery`)
- `DiscoveryService` for announcing and subscribing to tool announcements
- `ToolAnnouncement` type for cross-node tool broadcast
- New event topics: `bote/tool/announce`, `bote/tool/discovered`
- Bridge benchmark (`wrap_tool_result` overhead)

### Changed
- `full` feature now includes `bridge` and `discovery`
- Transport codec module visibility changed to `pub(crate)` for bridge reuse

## [0.22.3] — 2026-03-22

### Added
- HTTP transport (axum-based, feature `http`)
- WebSocket transport (bidirectional, feature `ws`)
- Unix domain socket transport (newline-delimited JSON, feature `unix`)
- Graceful shutdown on all network transports via shutdown future
- SSE streaming for long-running tool calls (HTTP)
- Progress notifications during execution (StreamContext, ProgressSender, CancellationToken)
- Streaming handler type (StreamingToolHandler, dispatch_streaming, DispatchOutcome)
- Cancellation support ($/cancelRequest, CancellationToken)
- Batch requests (JSON-RPC 2.0 batch array)
- Notification support (no id, no response expected)
- Protocol version negotiation in initialize handshake (2024-11-05, 2025-03-26)
- process_message() codec function for batch/notification/single dispatch
- Audit logging via libro (AuditSink trait, LibroAudit adapter, feature `audit`)
- Event publishing via majra (EventSink trait, MajraEvents adapter, feature `events`)
- Tool call timing with automatic audit + event logging in dispatch
- Event topic constants (TOPIC_TOOL_COMPLETED, TOPIC_TOOL_FAILED, TOPIC_TOOL_REGISTERED)
- Feature flags: http, ws, unix, all-transports, audit, events, full
- progress_notification() helper for consistent JSON-RPC notification format
- extract_tool_name() helper for validated tool name extraction
- Send + Sync compile-time assertions on all public types
- Benchmark suite: 8 benchmarks (dispatch, process_message, batch, streaming, validation)
- Benchmark history logging via scripts/bench-log.sh
- CODE_OF_CONDUCT.md, CONTRIBUTING.md, SECURITY.md
- codecov.yml with 80% project / 75% patch targets
- 129 tests across all modules

### Changed
- Transport module restructured: transport.rs -> transport/ directory (codec, stdio, http, ws, unix)
- Dispatcher.dispatch() returns Option<JsonRpcResponse> (None for notifications)
- JsonRpcRequest.id is now Option<serde_json::Value> (supports notifications)
- All transports use process_message() for unified dispatch
- WS/Unix transports use outgoing message channel pattern for streaming
- Mutex locks are poison-safe across all transports (unwrap_or_else)
- Handler panics caught and returned as -32603 error responses
- Cancelled tasks return -32800 (distinguished from panics in async transports)
- BoteError is now #[non_exhaustive]
- Serialization calls use explicit BUG labels instead of bare unwrap()
- deny.toml: tightened license list, added version 2 advisories, explicit allow-registry
- Makefile: added coverage, bench, --no-default-features clippy, RUSTDOCFLAGS for doc
- Cargo.toml: added documentation, exclude, full feature

### Fixed
- HTTP returns proper JSON-RPC error for malformed JSON (was returning 422)
- validate_params rejects non-object params (was silently passing)
- Dispatch uses BoteError consistently (was hardcoding error codes)
- transport::parse_request uses Json error variant (was converting to string)
- Empty tool name now returns -32602 instead of falling through to not-found
- jsonrpc version validated (rejects non-"2.0")
- Progress notification JSON deduplicated via stream::progress_notification()
- Tool name extraction deduplicated via Dispatcher::extract_tool_name()

### Removed
- Unused dependencies: anyhow, tokio, uuid (from core; tokio re-added as optional for transports)
