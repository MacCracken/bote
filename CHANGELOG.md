# Changelog

All notable changes to bote are documented here.

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
