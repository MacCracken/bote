# MCP Spec Compliance

> **Spec Version**: 2025-11-25 | **Bote Version**: 0.91.0 | **Last Audited**: 2026-04-02

## Protocol Versions

| Version | Status |
|---------|--------|
| `2024-11-05` | Supported |
| `2025-03-26` | Supported |
| `2025-11-25` | **Supported (default)** |

Negotiated via `initialize` method. Clients request a version; bote responds with the highest mutually supported version.

---

## Feature Matrix

### Core Protocol

| Spec Requirement | Module | Status |
|---|---|---|
| JSON-RPC 2.0 (request/response/notification) | `protocol` | Complete |
| `initialize` / `initialized` handshake | `dispatch` | Complete |
| Protocol version negotiation | `dispatch` | Complete (3 versions) |
| `tools/list` with schema | `dispatch` + `registry` | Complete |
| `tools/call` with validation | `dispatch` + `schema` | Complete |
| Streaming tool responses | `stream` | Complete (progress + cancellation) |

### Tool Definitions (2025-11-25)

| Spec Requirement | Module | Status |
|---|---|---|
| Tool name, description, inputSchema | `registry::ToolDef` | Complete |
| Tool versioning | `registry::ToolDef.version` | Complete |
| Tool deprecation | `registry::ToolDef.deprecated` | Complete |
| **Tool annotations** | `registry::ToolAnnotations` | **Complete (0.91.0)** |
| `readOnlyHint` | `ToolAnnotations.read_only_hint` | Complete |
| `destructiveHint` | `ToolAnnotations.destructive_hint` | Complete |
| `idempotentHint` | `ToolAnnotations.idempotent_hint` | Complete |
| `openWorldHint` | `ToolAnnotations.open_world_hint` | Complete |

### Content Types

| Type | Module | Status |
|---|---|---|
| Text (`"text"`) | `host::McpContentBlock::text_block` | Complete |
| Image (`"image"`) | `host::McpContentBlock::image_block` | Complete |
| **Audio (`"audio"`)** | `host::McpContentBlock::audio_block` | **Complete (0.91.0)** |
| Resource (`"resource"`) | — | Planned |

### Transport

| Spec Requirement | Module | Status |
|---|---|---|
| stdio | `transport::stdio` | Complete |
| HTTP (POST) | `transport::http` | Complete (feature: `http`) |
| WebSocket | `transport::ws` | Complete (feature: `ws`) |
| Unix domain socket | `transport::unix` | Complete (feature: `unix`) |
| **Streamable HTTP** (POST+GET single endpoint) | `transport::streamable` | **Types complete (0.91.0)** |
| SSE event IDs | `transport::streamable::EventIdGenerator` | Complete |
| `Last-Event-ID` resumption | `transport::streamable::ResumptionBuffer` | Complete |
| SSE priming event | `transport::streamable::StreamEvent::primer` | Complete |
| `retry:` hint before close | `StreamableConfig.retry_ms` | Complete |

### Session Management (2025-11-25)

| Spec Requirement | Module | Status |
|---|---|---|
| **`MCP-Protocol-Version` header** | `session::MCP_PROTOCOL_VERSION_HEADER` | **Complete (0.91.0)** |
| **`MCP-Session-Id` header** | `session::MCP_SESSION_ID_HEADER` | **Complete (0.91.0)** |
| Session creation on initialize | `session::SessionStore::create` | Complete |
| Session validation on subsequent requests | `session::SessionStore::validate` | Complete |
| Session timeout + pruning | `session::SessionStore::prune_expired` | Complete |
| Protocol version validation | `session::validate_protocol_version` | Complete |

### Security (2025-11-25)

| Spec Requirement | Module | Status |
|---|---|---|
| **Origin header validation** | `session::validate_origin` | **Complete (0.91.0)** |
| DNS rebinding protection (403 on invalid Origin) | `session::validate_origin` | Complete |
| CORS headers | `bridge` | Complete (feature: `bridge`) |
| SSRF protection for callback URLs | `host::validate_callback_url` | Complete |

### Authorization (2025-11-25)

| Spec Requirement | Module | Status |
|---|---|---|
| **OAuth 2.1 framework** | `auth` | **Types complete (0.91.0)** |
| PKCE S256 (mandatory) | `auth::generate_code_verifier`, `auth::verify_pkce` | Complete |
| Resource indicators (RFC 8707) | `auth::TokenClaims.resource` | Complete |
| Bearer token claims | `auth::TokenClaims` | Complete |
| Scope checking | `auth::TokenClaims::has_scope` | Complete |
| Expiration checking | `auth::TokenClaims::is_expired` | Complete |
| Protected resource metadata (RFC 9728) | `auth::ProtectedResourceMetadata` | Complete |
| `WWW-Authenticate` header (401) | `auth::www_authenticate_header` | Complete |
| Insufficient scope header (403) | `auth::insufficient_scope_header` | Complete |
| Client metadata documents | `auth::ClientMetadata` | Types complete |
| Token endpoint integration | — | Planned (needs HTTP client) |
| Authorization server discovery | — | Planned |

### Other

| Spec Requirement | Module | Status |
|---|---|---|
| Schema validation (JSON Schema subset) | `schema` | Complete |
| Audit logging | `audit` (feature: `audit`) | Complete (libro integration) |
| Event publishing | `events` (feature: `events`) | Complete (majra integration) |
| Tool sandboxing | `sandbox` (feature: `sandbox`) | Complete (kavach integration) |
| Cross-node discovery | `discovery` (feature: `discovery`) | Complete |
| TypeScript bridge | `bridge` (feature: `bridge`) | Complete |
| MCP host registry | `host` (feature: `host`) | Complete |
| Elicitation (server→client info requests) | — | Not in spec yet |

---

## Compliance Gaps

### Implemented but not wired to transport

The following types and logic are implemented but need to be wired into the HTTP/streamable transport handlers as middleware:

1. **Session enforcement** — `SessionStore` exists but transport handlers don't yet check `MCP-Session-Id` on every request
2. **Origin enforcement** — `validate_origin` exists but isn't called by transport handlers automatically
3. **Protocol version enforcement** — `validate_protocol_version` exists but transport handlers don't reject requests with wrong `MCP-Protocol-Version` header
4. **OAuth token validation** — `TokenClaims` validation exists but no middleware extracts and validates Bearer tokens from requests

These are wiring tasks, not missing capabilities. The security logic is complete; the transport integration is next.

### Not yet implemented

1. **Resource content type** — `"resource"` content blocks (URI + text/blob)
2. **Token endpoint HTTP client** — fetch tokens from authorization server
3. **Authorization server discovery** — `/.well-known/oauth-authorization-server` fetch
4. **Client metadata fetch** — HTTP GET of client_id URL for metadata document

---

## Test Coverage

| Module | Tests | Notes |
|--------|-------|-------|
| protocol | 18 | JSON-RPC types, serde roundtrips |
| registry | 25 | Tool registration, schema, annotations |
| dispatch | 34 | Version negotiation, routing, streaming |
| schema | 22 | Compilation, validation, edge cases |
| session | 10 | Create, validate, prune, origin, protocol version |
| auth | 12 | PKCE, token claims, metadata, headers |
| host | 25 | Host registry, SSRF, content blocks, audio |
| transport/streamable | 8 | Event IDs, resumption buffer, config |
| conformance | 44 | End-to-end protocol conformance |

**Total**: 305 tests (248 lib + 44 conformance + 12 doc + 1 integration)

---

*Audit method: manual comparison against [MCP spec 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25)*
