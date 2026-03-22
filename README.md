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
| **JSON-RPC 2.0** | Full request/response types with proper error codes |
| **Tool registry** | Register tools with schemas, discover, validate params |
| **Dispatch** | Route `tools/call` to registered handler functions |
| **Schema validation** | Required field checks (full JSON Schema in v0.24) |
| **Stdio transport** | Read requests from stdin, write responses to stdout |
| **Error mapping** | BoteError → JSON-RPC error codes (-32700 to -32000) |

---

## Quick start

```toml
[dependencies]
bote = "0.21"
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

let response = dispatcher.dispatch(&request);
// response.result = {"content": [{"type": "text", "text": "result: hello"}]}
```

### MCP server (stdio)

```rust
use bote::transport;

// Read from stdin
let line = /* read line from stdin */;
let request = transport::parse_request(&line)?;

// Dispatch
let response = dispatcher.dispatch(&request);

// Write to stdout
let output = transport::serialize_response(&response)?;
println!("{output}");
```

---

## MCP Protocol

bote implements the [Model Context Protocol](https://modelcontextprotocol.io/) over JSON-RPC 2.0:

| Method | Description | Response |
|--------|-------------|----------|
| `initialize` | Handshake | Server info, capabilities, protocol version |
| `tools/list` | Discovery | Array of tool definitions with schemas |
| `tools/call` | Execution | Tool result or JSON-RPC error |

### Error codes

| Code | Meaning | When |
|------|---------|------|
| -32700 | Parse error | Malformed JSON |
| -32600 | Invalid request | Missing jsonrpc/id/method |
| -32601 | Method not found | Unknown method or tool name |
| -32602 | Invalid params | Missing required fields |
| -32000 | Execution error | Handler returned an error |

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

4000 lines of duplicated JSON-RPC parsing replaced by `bote = "0.21"` in Cargo.toml.

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
| **0.21.3** | Foundation | JSON-RPC 2.0, registry, dispatch, stdio, 20+ tests |
| **0.22.3** | Transport | HTTP, WebSocket, Unix socket, streaming, cancellation |
| **0.23.3** | Integration | TypeScript bridge, libro audit, majra pub/sub |
| **0.24.3** | Registry | Full JSON Schema, versioning, hot-reload, namespacing |
| **0.25.3** | Adoption | daimon, 23 apps, SY, agnoshi integration |
| **1.0.0** | Stable | Full compliance, < 500ns dispatch, all transports |

Full details: [docs/development/roadmap.md](docs/development/roadmap.md)

---

## Building from source

```bash
git clone https://github.com/MacCracken/bote.git
cd bote
cargo build
cargo test
make check
```

---

## Versioning

Pre-1.0: `0.D.M` SemVer (day.month). Post-1.0: standard SemVer.

---

## License

AGPL-3.0-only. See [LICENSE](LICENSE) for details.
