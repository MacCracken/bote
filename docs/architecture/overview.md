# Bote Architecture

> MCP core service — JSON-RPC 2.0 protocol, tool registry, dispatch, and TypeScript bridge.
>
> **Name**: Bote (German) — messenger. The messenger between agents and tools.
> Eliminates 23 separate MCP stdio implementations across AGNOS consumer apps.

---

## Design Principles

1. **One protocol implementation** — every app uses bote instead of reimplementing JSON-RPC 2.0
2. **Registry-driven** — tools are registered with schemas, dispatch validates automatically
3. **Transport-agnostic** — stdio is the default, but HTTP/WebSocket transports plug in
4. **Audit-ready** — every tool call is loggable via libro integration
5. **TypeScript bridge** — SY and other TS apps call Rust MCP tools via bote's FFI/HTTP bridge

---

## System Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  Consumers (agnoshi, daimon, SY, AgnosAI, consumer apps)      │
│                                                               │
│  Client: JSON-RPC 2.0 over stdio / HTTP / WebSocket           │
└───────────────────────────┬──────────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────────┐
│  Bote Core                                                    │
│                                                               │
│  ┌───────────┐   ┌──────────────┐   ┌────────────────────┐  │
│  │ Transport │   │   Registry   │   │    Dispatcher      │  │
│  │ (stdio)   │──▶│ (tool defs   │──▶│ (route + validate  │  │
│  │           │   │  + schemas)  │   │  + call handler)   │  │
│  └───────────┘   └──────────────┘   └─────────┬──────────┘  │
│                                                │              │
│  ┌─────────────────────────────────────────────▼────────────┐│
│  │  Tool Handlers                                            ││
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────┐  ││
│  │  │ Rust     │ │ TypeScript│ │ Python   │ │  External  │  ││
│  │  │ (native) │ │ (bridge) │ │ (bridge) │ │  (HTTP)    │  ││
│  │  └──────────┘ └──────────┘ └──────────┘ └────────────┘  ││
│  └───────────────────────────────────────────────────────────┘│
│                                                               │
│  ┌──────────────────┐  ┌──────────────────┐                  │
│  │ libro (audit)    │  │ majra (events)   │                  │
│  │ Every call logged│  │ Tool call pub/sub│                  │
│  └──────────────────┘  └──────────────────┘                  │
└───────────────────────────────────────────────────────────────┘
```

---

## Module Structure

```
src/
├── lib.rs              Public API, Result type
├── error.rs            BoteError with JSON-RPC error codes
├── protocol.rs         JsonRpcRequest, JsonRpcResponse, JsonRpcError
├── registry.rs         ToolRegistry, ToolDef, ToolSchema, validation
├── transport.rs        Stdio read/write (parse_request, serialize_response)
├── dispatch.rs         Dispatcher — route calls to handlers, MCP methods
└── tests/
    └── mod.rs          Integration tests
```

---

## MCP Protocol Methods

| Method | Description |
|--------|-------------|
| `initialize` | Handshake — returns server info and capabilities |
| `tools/list` | List all registered tools with schemas |
| `tools/call` | Call a tool by name with arguments |

---

## Key Types

### ToolDef
```rust
pub struct ToolDef {
    pub name: String,           // e.g. "jalwa_play"
    pub description: String,    // Human-readable
    pub input_schema: ToolSchema, // JSON Schema for validation
}
```

### Dispatcher
Routes `tools/call` requests to registered handler functions. Validates params against the tool's schema before dispatch. Returns JSON-RPC errors for unknown tools, missing params, or handler failures.

### JsonRpcError Codes
| Code | Meaning |
|------|---------|
| -32700 | Parse error |
| -32600 | Invalid request |
| -32601 | Method/tool not found |
| -32602 | Invalid params |
| -32000 | Tool execution error |
| -32003 | Transport closed |

---

## Consumers

| Project | Current | With bote |
|---------|---------|-----------|
| **23 consumer apps** | Each reimplements JSON-RPC 2.0 stdio MCP server | `bote::Dispatcher` + register tools |
| **daimon** | 144 MCP tools with custom dispatch | `bote::ToolRegistry` + `bote::Dispatcher` |
| **SecureYeoman** | TypeScript MCP with custom JSON-RPC | bote Rust core via FFI/HTTP bridge |
| **agnoshi** | Calls MCP tools via stdio | `bote::transport` for consistent protocol |
| **AgnosAI** | Agent tool execution | `bote::Dispatcher` for sandboxed tool calls via kavach |
