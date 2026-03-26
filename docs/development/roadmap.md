# Bote Roadmap

> **Principle**: One protocol implementation for the entire ecosystem. Tools are registered, not reimplemented.

Completed items are in [CHANGELOG.md](../../CHANGELOG.md).

---

## v0.23.3 — TypeScript Bridge (done)

### TypeScript bridge
- [x] HTTP bridge server — Rust bote core exposes tools via HTTP, SY's TypeScript calls it
- [x] Tool result serialization compatible with SY's MCP format
- [x] Health endpoint for bridge liveness

### Cross-node discovery
- [x] Cross-node tool discovery via majra

---

## v0.24.3 — Advanced Registry (done)

### Schema validation
- [x] Full JSON Schema validation (not just required fields)
- [x] Type checking (string, number, boolean, array, object)
- [x] Enum constraints
- [x] Default values

### Tool versioning
- [x] Version field in ToolDef
- [x] Capability negotiation — client requests version, server matches
- [x] Deprecation warnings for old tool versions

### Dynamic registration
- [x] Runtime tool registration/deregistration
- [x] Hot-reload tool handlers without restart
- [x] Tool namespacing (project_tool format enforcement)

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
