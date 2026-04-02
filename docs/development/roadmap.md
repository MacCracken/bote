# Bote Roadmap

> **Principle**: One protocol implementation for the entire ecosystem. Tools are registered, not reimplemented.
> **Spec**: MCP 2025-11-25 | **Compliance**: [spec-compliance.md](../spec-compliance.md)

Completed items are in [CHANGELOG.md](../../CHANGELOG.md).

---

## v0.92.0 — Transport Middleware Integration

Wire the session, origin, and auth types (added in 0.91.0) into the HTTP and streamable transport handlers as axum middleware layers.

### Session enforcement
- [ ] HTTP transport: extract `MCP-Session-Id` header, validate via `SessionStore`, return 404 if invalid
- [ ] HTTP transport: return `MCP-Session-Id` header on `initialize` response
- [ ] Streamable transport: same session lifecycle via middleware
- [ ] Session prune on periodic timer (tokio interval)

### Origin enforcement
- [ ] HTTP/streamable middleware: extract `Origin` header, call `validate_origin`, return 403 on failure
- [ ] Configurable allowed origins per transport config

### Protocol version enforcement
- [ ] HTTP/streamable middleware: extract `MCP-Protocol-Version` header, validate, return 400 on mismatch

### Bearer token extraction
- [ ] HTTP/streamable middleware: extract `Authorization: Bearer <token>` header
- [ ] Pass `TokenClaims` into dispatch context so handlers can check scopes
- [ ] Return 401 with `WWW-Authenticate` header when token missing/invalid
- [ ] Return 403 with `insufficient_scope` header when scope insufficient

---

## v0.93.0 — OAuth 2.1 Client Flow

Complete the OAuth 2.1 authorization flow with HTTP client integration.

### Token endpoint
- [ ] HTTP client to exchange auth code for token (PKCE flow)
- [ ] Token refresh flow
- [ ] Token caching with expiry tracking

### Discovery
- [ ] Fetch `/.well-known/oauth-authorization-server` metadata
- [ ] Fetch `/.well-known/oauth-protected-resource` metadata
- [ ] Client metadata document fetch (GET client_id URL)

### Protected resource endpoint
- [ ] Serve `/.well-known/oauth-protected-resource` from `ProtectedResourceMetadata`

---

## v0.94.0 — Resource Content & Completions

### Resource content type
- [ ] `McpContentBlock::resource_block` with URI + text/blob content
- [ ] Resource subscription (notifications on resource change)

### Completions
- [ ] `completion/complete` method — argument autocompletion for tools
- [ ] Completion providers registered alongside tool handlers

---

## v0.95.0 — Streamable HTTP Full Implementation

Wire the streamable transport types (0.91.0) into a working axum router.

### Single endpoint router
- [ ] POST to `/mcp` — JSON-RPC request → response (or SSE stream)
- [ ] GET to `/mcp` — open SSE stream for server-initiated messages
- [ ] `Last-Event-ID` header → replay from `ResumptionBuffer`
- [ ] `retry:` field sent before closing SSE connection
- [ ] Priming event on stream open (empty data)

### Integration
- [ ] Streamable transport passes full conformance suite
- [ ] Benchmark: streamable vs plain HTTP latency overhead

---

## v1.0.0 Criteria

- [ ] MCP 2025-11-25 fully compliant (all features, not just types)
- [ ] Session, origin, auth middleware wired and tested
- [ ] OAuth 2.1 flow end-to-end (with test auth server)
- [ ] Streamable HTTP transport passes conformance
- [ ] Resource content type complete
- [ ] API frozen — no breaking changes
- [ ] `#![forbid(unsafe_code)]`
- [ ] `#![warn(missing_docs)]` — full doc coverage
- [ ] Benchmark regression thresholds in CI
- [ ] 19+ downstream consumers validated

---

## Post-v1.0

### Advanced
- [ ] gRPC transport (proto definitions for MCP)
- [ ] Tool marketplace — discover and install tools from mela
- [ ] AI-powered tool selection — hoosh suggests which tool based on intent
- [ ] Tool composition — chain multiple tools via szal

### Platform
- [ ] WASM tool handlers (wasmtime)
- [ ] Python tool handlers (PyO3)
- [ ] C FFI for external tool integration

### Ecosystem
- [ ] jnana knowledge tools registered as bote MCP tools

---

## Non-goals

- **Tool implementation** — bote dispatches to handlers, doesn't implement business logic
- **LLM integration** — that's hoosh. Bote doesn't decide which tool to call
- **Workflow orchestration** — that's szal. Bote calls one tool at a time
- **Agent lifecycle** — that's daimon/agnosai. Bote doesn't manage agents
