# Bote Roadmap

> **Principle**: One protocol implementation for the entire ecosystem. Tools are registered, not reimplemented.

Completed items are in [CHANGELOG.md](../../CHANGELOG.md).

---

## v0.21.3 — Foundation (current)

- [x] JSON-RPC 2.0 types (JsonRpcRequest, JsonRpcResponse, JsonRpcError)
- [x] ToolRegistry with schema validation (required field checks)
- [x] ToolDef with ToolSchema (JSON Schema subset)
- [x] Dispatcher — initialize, tools/list, tools/call routing
- [x] Handler registration via Arc<Fn> closures
- [x] Stdio transport (parse_request, serialize_response)
- [x] BoteError with JSON-RPC error code mapping
- [x] 20+ tests, benchmark scaffold
- [x] CI/CD pipeline (lint, test, deny, MSRV, coverage, multi-arch release)

---

## v0.22.3 — Transport & Streaming

### Transport expansion
- [ ] HTTP transport (axum-based, for remote MCP servers)
- [ ] WebSocket transport (persistent connection, bidirectional)
- [ ] Unix domain socket transport (for local IPC)

### Streaming
- [ ] SSE streaming for long-running tool calls
- [ ] Progress notifications during execution
- [ ] Cancellation support (client sends cancel, handler receives signal)

### Protocol
- [ ] Batch requests (JSON-RPC 2.0 batch array)
- [ ] Notification support (no id, no response expected)
- [ ] Protocol version negotiation in initialize handshake

---

## v0.23.3 — TypeScript Bridge & libro Integration

### TypeScript bridge
- [ ] HTTP bridge server — Rust bote core exposes tools via HTTP, SY's TypeScript calls it
- [ ] Tool result serialization compatible with SY's MCP format
- [ ] Health endpoint for bridge liveness

### libro integration
- [ ] Every tools/call logged to libro audit chain
- [ ] Tool call duration, caller ID, result summary in audit entry
- [ ] Failed calls logged with error details

### majra integration
- [ ] Tool call events published to majra pub/sub
- [ ] Tool registration events (new tool available, tool removed)
- [ ] Cross-node tool discovery via majra

---

## v0.24.3 — Advanced Registry

### Schema validation
- [ ] Full JSON Schema validation (not just required fields)
- [ ] Type checking (string, number, boolean, array, object)
- [ ] Enum constraints
- [ ] Default values

### Tool versioning
- [ ] Version field in ToolDef
- [ ] Capability negotiation — client requests version, server matches
- [ ] Deprecation warnings for old tool versions

### Dynamic registration
- [ ] Runtime tool registration/deregistration
- [ ] Hot-reload tool handlers without restart
- [ ] Tool namespacing (project_tool format enforcement)

---

## v0.25.3 — Consumer Integration

### Adoption
- [ ] daimon replaces custom MCP dispatch with bote
- [ ] 23 consumer apps replace inline MCP servers with bote
- [ ] SY adopts bote via TypeScript bridge
- [ ] agnoshi uses bote for tool discovery

### Validation
- [ ] Cross-crate integration tests
- [ ] Protocol conformance test suite
- [ ] Performance: dispatch overhead < 1µs per tool call

---

## v1.0.0 Criteria

- [ ] JSON-RPC 2.0 fully compliant (batch, notifications, streaming)
- [ ] All 3 transports stable (stdio, HTTP, WebSocket)
- [ ] TypeScript bridge production-tested with SY
- [ ] libro audit integration for every tool call
- [ ] 3+ downstream consumers in production
- [ ] 90%+ test coverage
- [ ] docs.rs complete
- [ ] Protocol conformance test suite passing
- [ ] Dispatch benchmark: < 500ns per tool call

---

## Post-v1

### Advanced
- [ ] gRPC transport (proto definitions for MCP)
- [ ] Tool sandboxing via kavach (execute untrusted tool handlers in isolation)
- [ ] Tool marketplace — discover and install tools from mela
- [ ] AI-powered tool selection — hoosh suggests which tool to call based on intent
- [ ] Tool composition — chain multiple tools into a workflow via szál

### Platform
- [ ] WASM tool handlers (run tool logic in wasmtime)
- [ ] Python tool handlers (PyO3 bridge)
- [ ] C FFI for external tool integration

---

## Non-goals

- **Tool implementation** — bote dispatches to handlers, doesn't implement business logic
- **LLM integration** — that's hoosh. Bote doesn't decide which tool to call
- **Workflow orchestration** — that's szál. Bote calls one tool at a time
- **Agent lifecycle** — that's daimon/AgnosAI. Bote doesn't manage agents
