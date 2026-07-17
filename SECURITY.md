# Security Policy

## Scope

bote is an MCP core service library providing JSON-RPC 2.0 protocol handling,
tool dispatch, and multiple transport layers (stdio, HTTP, Unix socket,
HTTP↔stdio bridge, Streamable HTTP/SSE, WebSocket). It processes untrusted
JSON input from clients and dispatches to user-registered handlers.

The primary security-relevant surface areas are:

- **JSON-RPC parsing** — `codec` / `jsonx` parsing of untrusted client input.
  Malformed or oversized payloads could cause excessive memory allocation;
  batch length is capped to bound per-request allocation.
- **Transport layer** — HTTP, Streamable HTTP/SSE, WebSocket, Unix socket, and
  bridge transports accept network connections. Opt-in auth validators (bearer,
  allowlist, JWT HS256, PKCE) are built in; TLS is expected to be handled by
  the deployment environment.
- **Tool dispatch** — handler functions are user-provided. Arguments are
  validated against the registered JSON Schema before invocation; validation
  failures are returned as error responses, not propagated.
- **Outbound requests** — the `host` module's SSRF guard rejects URLs targeting
  loopback / link-local / private / cloud-metadata endpoints (IPv4 and IPv6)
  before any network call.
- **Audit chain** — the optional libro integration uses SHA-256 hash linking
  for tamper detection.

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 3.1.x   | Yes (current minor) |
| 3.0.x   | Yes (prior minor) |
| < 3.0   | No        |

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

- No panic/abort in library code — errors return as 0 / -1 / error tags; the
  consumer decides.
- jsonrpc version validated on every request.
- Empty/missing tool names rejected before dispatch.
- Tool arguments validated against the registered JSON Schema before dispatch.
- JSON-RPC batch length capped to bound per-request allocation.
- Profile-split distribution (`dist/bote-core.cyr`) — core has minimal attack
  surface.
