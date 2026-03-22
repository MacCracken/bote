# Bote Architecture

> MCP core service — JSON-RPC 2.0 protocol, tool registry, dispatch, streaming, and observability.
>
> **Name**: Bote (German) — messenger. The messenger between agents and tools.
> Eliminates 23 separate MCP stdio implementations across AGNOS consumer apps.

---

## Design Principles

1. **One protocol implementation** — every app uses bote instead of reimplementing JSON-RPC 2.0
2. **Registry-driven** — tools are registered with schemas, dispatch validates automatically
3. **Transport-agnostic** — stdio, HTTP, WebSocket, Unix socket via feature flags
4. **Streaming-ready** — progress notifications and cancellation for long-running tools
5. **Audit-ready** — every tool call loggable via libro; events published via majra
6. **Minimal by default** — no default features; consumers opt in to what they need

---

## System Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  Consumers (agnoshi, daimon, SY, AgnosAI, consumer apps)      │
│                                                               │
│  Client: JSON-RPC 2.0 over stdio / HTTP / WebSocket / Unix    │
└───────────────────────────┬──────────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────────┐
│  Bote Core                                                    │
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  Transport Layer (feature-gated)                         │ │
│  │  stdio │ HTTP (axum) │ WebSocket │ Unix socket           │ │
│  │        │ + SSE stream │           │                       │ │
│  └────────────────────────┬────────────────────────────────┘ │
│                           │                                   │
│  ┌────────────┐  ┌────────▼─────────┐  ┌──────────────────┐ │
│  │  Registry  │  │   Dispatcher     │  │  Stream Context  │ │
│  │ (tool defs │──│ dispatch()       │  │  (progress +     │ │
│  │  + schemas)│  │ dispatch_stream()│──│   cancellation)  │ │
│  └────────────┘  └────────┬─────────┘  └──────────────────┘ │
│                           │                                   │
│  ┌────────────────────────▼────────────────────────────────┐ │
│  │  Tool Handlers                                           │ │
│  │  Sync: Fn(Value) -> Value                                │ │
│  │  Streaming: Fn(Value, StreamContext) -> Value             │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                               │
│  ┌──────────────────┐  ┌──────────────────┐                  │
│  │ libro (audit)    │  │ majra (events)   │                  │
│  │ AuditSink trait  │  │ EventSink trait  │                  │
│  │ feature: audit   │  │ feature: events  │                  │
│  └──────────────────┘  └──────────────────┘                  │
└───────────────────────────────────────────────────────────────┘
```

---

## Module Structure

```
src/
├── lib.rs              Public API, Result type, Send+Sync assertions
├── error.rs            BoteError (#[non_exhaustive]) with JSON-RPC error codes
├── protocol.rs         JsonRpcRequest, JsonRpcResponse, JsonRpcError
├── registry.rs         ToolRegistry, ToolDef, ToolSchema, validation
├── dispatch.rs         Dispatcher, DispatchOutcome, tool name extraction
├── stream.rs           CancellationToken, ProgressUpdate, StreamContext
├── audit.rs            AuditSink trait, ToolCallEvent, LibroAudit (feature: audit)
├── events.rs           EventSink trait, topic constants, MajraEvents (feature: events)
├── transport/
│   ├── mod.rs          Re-exports: parse_request, serialize_response, process_message
│   ├── codec.rs        JSON-RPC codec, batch processing, jsonrpc validation
│   ├── stdio.rs        Blocking line-oriented transport (sync + streaming)
│   ├── http.rs         Axum-based HTTP + SSE streaming (feature: http)
│   ├── ws.rs           WebSocket bidirectional transport (feature: ws)
│   └── unix.rs         Unix domain socket transport (feature: unix)
├── tests/
│   └── mod.rs          Integration tests
└── (benches/dispatch.rs  8 benchmarks)
```

---

## Feature Flags

| Flag | Dependencies | Description |
|------|-------------|-------------|
| `http` | axum, tokio, futures-util | HTTP transport (POST + SSE streaming) |
| `ws` | tokio, tokio-tungstenite, futures-util | WebSocket transport |
| `unix` | tokio | Unix domain socket transport |
| `all-transports` | all above | Enables http + ws + unix |
| `audit` | libro | Audit logging via hash-linked chain |
| `events` | majra | Event publishing via pub/sub |
| `full` | all above | All transports + audit + events |

---

## MCP Protocol Methods

| Method | Description |
|--------|-------------|
| `initialize` | Handshake — returns server info, capabilities, negotiated protocol version |
| `tools/list` | List all registered tools with schemas |
| `tools/call` | Call a tool by name with arguments |
| `$/cancelRequest` | Cancel an in-progress streaming tool call |

---

## JSON-RPC Error Codes

| Code | Meaning |
|------|---------|
| -32700 | Parse error |
| -32600 | Invalid request (bad jsonrpc version, empty batch, non-object) |
| -32601 | Method/tool not found |
| -32602 | Invalid params (missing required fields, empty tool name) |
| -32000 | Tool execution error |
| -32003 | Transport closed / bind failed |
| -32603 | Internal error (handler panicked) |
| -32800 | Request cancelled |

---

## Event Topics (majra)

| Topic | When |
|-------|------|
| `bote/tool/completed` | Tool call succeeded |
| `bote/tool/failed` | Tool call failed |
| `bote/tool/registered` | Tool registered |

---

## Consumers

| Project | Usage |
|---------|-------|
| **23 AGNOS consumer apps** | Replace inline MCP servers with bote dispatch |
| **daimon** | 144 MCP tools via bote registry |
| **SecureYeoman** | TypeScript bridge to Rust MCP core |
| **agnoshi** | Tool discovery and invocation |
| **AgnosAI** | Sandboxed tool execution via kavach + bote |
