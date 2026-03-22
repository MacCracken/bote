# bote

**MCP core service for Rust.**

JSON-RPC 2.0 protocol, tool registry, schema validation, and dispatch — in a single crate. Eliminates 23 separate MCP server implementations across the AGNOS ecosystem.

> **Name**: Bote (German) — messenger. The messenger between agents and tools.

[![Crates.io](https://img.shields.io/crates/v/bote.svg)](https://crates.io/crates/bote)
[![CI](https://github.com/MacCracken/bote/actions/workflows/ci.yml/badge.svg)](https://github.com/MacCracken/bote/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

---

## What it does

bote is the **MCP protocol layer** — it handles the JSON-RPC 2.0 wire format, tool registration, schema validation, and call dispatch so individual apps don't each reimplement the same protocol.

| Capability | Details |
|------------|---------|
| **JSON-RPC 2.0** | Requests, responses, notifications, batch arrays |
| **Tool registry** | Register tools with schemas, discover, validate params |
| **Dispatch** | Route `tools/call` to registered handler functions |
| **Streaming** | Progress notifications and cancellation for long-running tools |
| **Transports** | Stdio, HTTP (axum + SSE), WebSocket, Unix socket |
| **Audit** | Tool call logging via libro hash-linked chain |
| **Events** | Tool events published via majra pub/sub |
| **Protocol** | Version negotiation, batch requests, notifications |

## Feature Flags

| Flag | Description |
|------|-------------|
| `http` | HTTP transport via axum (POST + SSE streaming) |
| `ws` | WebSocket transport via tokio-tungstenite |
| `unix` | Unix domain socket transport |
| `all-transports` | Enables `http`, `ws`, and `unix` |
| `audit` | Audit logging via libro hash-linked chain |
| `events` | Event publishing via majra pub/sub |
| `full` | All transports + audit + events |

None are enabled by default — enable only what you need.

---

## Quick start

```toml
[dependencies]
bote = "0.22"
```

```rust
use std::sync::Arc;
use std::collections::HashMap;
use bote::{Dispatcher, ToolRegistry, ToolDef, ToolSchema, JsonRpcRequest};

// Register tools
let mut registry = ToolRegistry::new();
registry.register(ToolDef {
    name: "my_tool".into(),
    description: "Does something useful".into(),
    input_schema: ToolSchema {
        schema_type: "object".into(),
        properties: HashMap::new(),
        required: vec!["input".into()],
    },
});

// Wire up handlers
let mut dispatcher = Dispatcher::new(registry);
dispatcher.handle("my_tool", Arc::new(|params| {
    let input = params["input"].as_str().unwrap_or("none");
    serde_json::json!({
        "content": [{ "type": "text", "text": format!("result: {input}") }]
    })
}));

// Dispatch requests
let request = JsonRpcRequest::new(1, "tools/call")
    .with_params(serde_json::json!({
        "name": "my_tool",
        "arguments": { "input": "hello" }
    }));

let response = dispatcher.dispatch(&request).unwrap();
// response.result = {"content": [{"type": "text", "text": "result: hello"}]}
```

### MCP server (stdio)

```rust
use bote::transport::stdio;

// Runs a blocking loop: reads JSON-RPC from stdin, dispatches, writes to stdout.
stdio::run(&dispatcher)?;
```

### HTTP transport

```toml
[dependencies]
bote = { version = "0.22", features = ["http"] }
```

```rust
use bote::transport::http::{serve, HttpConfig};
use std::sync::Arc;

let config = HttpConfig { addr: "127.0.0.1:3000".parse().unwrap() };
serve(Arc::new(dispatcher), config, shutdown_signal).await?;
// POST / for JSON-RPC, GET /health for liveness
```

---

## MCP Protocol

bote implements the [Model Context Protocol](https://modelcontextprotocol.io/) over JSON-RPC 2.0:

| Method | Description | Response |
|--------|-------------|----------|
| `initialize` | Handshake | Server info, capabilities, negotiated protocol version |
| `tools/list` | Discovery | Array of tool definitions with schemas |
| `tools/call` | Execution | Tool result or JSON-RPC error |
| `$/cancelRequest` | Cancellation | Cancels an in-progress streaming call |

### Error codes

| Code | Meaning | When |
|------|---------|------|
| -32700 | Parse error | Malformed JSON |
| -32600 | Invalid request | Bad jsonrpc version, empty batch |
| -32601 | Method not found | Unknown method or tool name |
| -32602 | Invalid params | Missing required fields, empty tool name |
| -32000 | Execution error | Handler returned an error |
| -32603 | Internal error | Handler panicked |
| -32800 | Request cancelled | Streaming call cancelled by client |

---

## Why bote?

Every AGNOS consumer app currently implements its own MCP server:

```
Before bote:                          After bote:
─────────────                         ────────────
jalwa/src/mcp.rs    (150 lines)       jalwa: bote::Dispatcher + 5 handlers
shruti/src/mcp.rs   (180 lines)       shruti: bote::Dispatcher + 7 handlers
tazama/src/mcp.rs   (160 lines)       tazama: bote::Dispatcher + 7 handlers
rasa/src/mcp.rs     (200 lines)       rasa: bote::Dispatcher + 9 handlers
... × 23 apps       (~4000 lines)     ... × 23 apps (0 protocol code)
```

4000 lines of duplicated JSON-RPC parsing replaced by `bote = "0.22"` in Cargo.toml.

---

## Who uses this

| Project | Usage |
|---------|-------|
| **23 AGNOS consumer apps** | Replace inline MCP servers with bote dispatch |
| **daimon** | 144 MCP tools via bote registry |
| **SecureYeoman** | TypeScript bridge to Rust MCP core |
| **agnoshi** | Tool discovery and invocation |
| **AgnosAI** | Sandboxed tool execution via kavach + bote |

---

## Roadmap

| Version | Milestone | Key features |
|---------|-----------|--------------|
| **0.21.3** | Foundation | JSON-RPC 2.0, registry, dispatch, stdio |
| **0.22.3** | Transport & Streaming | HTTP, WebSocket, Unix, SSE, batch, audit, events |
| **0.23.3** | TypeScript Bridge | SY bridge, cross-node discovery |
| **0.24.3** | Advanced Registry | Full JSON Schema, versioning, hot-reload |
| **0.25.3** | Adoption | daimon, 23 apps, SY, agnoshi integration |
| **1.0.0** | Stable | Full compliance, < 500ns dispatch, all transports |

Full details: [docs/development/roadmap.md](docs/development/roadmap.md)

---

## Building from source

```bash
git clone https://github.com/MacCracken/bote.git
cd bote
make check          # fmt + clippy + test + audit
make test-all       # test every feature flag
make bench          # run benchmarks + log history
make coverage       # HTML coverage report
```

---

## Versioning

Pre-1.0: `0.D.M` SemVer (day.month). Post-1.0: standard SemVer.

---

## License

AGPL-3.0-only. See [LICENSE](LICENSE) for details.
