# Security Policy

## Scope

bote is an MCP core service library providing JSON-RPC 2.0 protocol handling,
tool dispatch, and multiple transport layers (stdio, HTTP, Unix socket,
HTTP↔stdio bridge, Streamable HTTP/SSE, WebSocket). It processes untrusted
JSON input from clients and dispatches to user-registered handlers.

The primary security-relevant surface areas are:

- **JSON-RPC parsing** — `codec` / `jsonx` parsing of untrusted client input.
  Malformed or oversized payloads could cause excessive memory allocation;
  batch length is capped to bound per-request allocation. Since 3.4.5 every
  message is syntax-checked (`jsonx_valid`, RFC 8259, nesting capped at 64)
  before any extractor reads it. Before that the scanning extractors read
  whatever they were given, and a truncated object containing a "method"
  was dispatched.
- **Transport layer** — HTTP, Streamable HTTP/SSE, WebSocket, Unix socket, and
  bridge transports accept network connections. Opt-in auth validators (bearer,
  allowlist, JWT HS256, PKCE) are built in; TLS is expected to be handled by
  the deployment environment. Each transport serves one connection at a time;
  since 3.4.3 every connection carries a send deadline (`BOTE_SEND_TIMEOUT_MS`,
  default 30 s), so a client that stops reading is dropped instead of wedging
  the server for everyone. Read deadlines: sandhi's idle and whole-request
  budgets on the HTTP family, opt-in `ws_config_with_idle_ms` on an upgraded
  WebSocket, opt-in `BOTE_UNIX_IDLE_MS` on the Unix transport — a quiet client
  holds those two single-threaded servers unless the operator opts in.
- **Tool dispatch** — handler functions are user-provided. Arguments are
  validated against the registered JSON Schema before invocation; validation
  failures are returned as error responses, not propagated.
- **Outbound requests** — the `host` module's SSRF guard rejects URLs targeting
  loopback / link-local / private / cloud-metadata endpoints (IPv4 and IPv6)
  before any network call. `web_fetch`, the built-in tool that fetches a
  client-supplied URL, applies it in two layers: the URL's host string (IP
  literals and a hostname blocklist) since **3.4.0**, and since **3.4.4** the
  address a name actually resolves to, classified inside sandhi's client
  resolve hook, so the address judged is the address dialled (no DNS-rebinding
  window, and `127.0.0.1.nip.io`-style names are refused). Before 3.4.0 it
  checked only the scheme. Known gaps, on the roadmap: the IPv4 classifier has
  no CGNAT (100.64.0.0/10), broadcast or 240.0.0.0/4 rule, and `web_search`
  is deliberately unguarded (its endpoint is the operator's
  `BOTE_SEARXNG_URL`).
- **Logging** — operational logs go through sakshi to stderr (or
  `BOTE_LOG_FILE`), never to stdout, which carries the stdio transport's
  JSON-RPC stream. No bearer token, JWT, request body, tool argument or full
  URL is written to a log line. Client-supplied values that are logged (a
  method or tool name, a refused path) are logfmt-escaped, so a value holding
  a newline cannot forge a second log line. Refusals (401, 403, 413, a rejected
  `Transfer-Encoding`, an SSRF block, a filesystem path escape) are logged at
  WARN; one line per refused request, so a flood of bad requests is a flood of
  lines — rate-limit at the edge if that matters for your deployment.
- **Audit chain** — the optional libro integration uses SHA-256 hash linking
  for tamper detection.

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 3.3.x   | Yes (current minor) |
| 3.2.x   | Yes (prior minor) |
| < 3.2   | No        |

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
- Every message is checked to be one valid JSON text before it is read
  (`-32700` otherwise), and the jsonrpc version, the id type and the params
  type are validated on every request (`-32600`).
- Empty/missing tool names rejected before dispatch.
- Tool arguments validated against the registered JSON Schema before dispatch.
  A violation never reaches the handler; it is answered as a result with
  `isError: true` (MCP 2025-11-25, SEP-1303).
- JSON-RPC batch length capped to bound per-request allocation.
- Profile-split distribution (`dist/bote-core.cyr`) — core has minimal attack
  surface.
