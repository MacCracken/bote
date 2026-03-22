# Security Policy

## Scope

bote is an MCP core service library providing JSON-RPC 2.0 protocol handling,
tool dispatch, and multiple transport layers (stdio, HTTP, WebSocket, Unix
socket). It processes untrusted JSON input from clients and dispatches to
user-registered handlers.

The primary security-relevant surface areas are:

- **JSON-RPC parsing** — `serde_json` deserialization of untrusted client input.
  Malformed or oversized payloads could cause excessive memory allocation.
- **Transport layer** — HTTP, WebSocket, and Unix socket transports accept
  network connections. No authentication or TLS is built in; these are expected
  to be handled by the deployment environment.
- **Tool dispatch** — handler functions are user-provided. Panics in handlers
  are caught and converted to error responses, not propagated.
- **Concurrency** — streaming handlers run on spawned threads/tasks. Mutex
  poisoning is handled gracefully (unwrap_or_else, not unwrap).
- **Audit chain** — the optional libro integration uses SHA-256 hash linking
  for tamper detection.

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.22.x  | Yes       |
| < 0.22  | No        |

## Reporting a Vulnerability

If you discover a security vulnerability in bote, please report it
responsibly:

1. **Email** [security@agnos.dev](mailto:security@agnos.dev) with a description
   of the issue, steps to reproduce, and any relevant context.
2. **Do not** open a public issue for security vulnerabilities.
3. You will receive an acknowledgment within **48 hours**.
4. We follow a **90-day disclosure timeline**. We will work with you to
   coordinate public disclosure after a fix is available.

## Security Design

- No `unsafe` code in the library.
- Compile-time `Send + Sync` assertions on all public types.
- Mutex poisoning handled gracefully across all transports.
- Handler panics caught and returned as JSON-RPC error responses.
- jsonrpc version validated on every request.
- Empty/missing tool names rejected before dispatch.
- Feature-gated dependencies — core has minimal attack surface.
