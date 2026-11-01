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
  WebSocket (`BOTE_WS_IDLE_MS` in the binary), opt-in `BOTE_UNIX_IDLE_MS` on the
  Unix transport — a quiet client holds those two single-threaded servers unless
  the operator opts in. The send deadline is per write, so a client that keeps
  reading a little at a time is not dropped and can slow every other client;
  threaded dispatch (roadmap 3.7.x) is the answer to that one.
- **Browsers** — since **3.4.15** every HTTP-family transport refuses a present
  `Origin` that is not on `BOTE_ALLOWED_ORIGINS` with 403, before anything else
  runs, and with no list set every present `Origin` is refused. No `Origin` passes,
  because a command-line client sends none. Through 3.4.14 the transports checked
  only when a list was configured, the binary offered no way to configure one, and
  `bote bridge` answered `Access-Control-Allow-Origin: *` with its file tools
  registered, so any website the operator visited could read and write under
  `BOTE_FS_ROOT`. `*` in the list is the operator's explicit choice and logs a WARN.
- **Memory** — `alloc` never frees, so what a request leaves on the heap stays for
  the life of the process. Since **3.4.15** everything bote allocates to answer a
  message goes on a request arena that the next message rewinds, including
  refusals before and after auth, so an unauthenticated client cannot grow the
  heap. A valid tool call still leaves its `args` and whatever its handler
  allocates, because the 2.0 handler ABI lets a handler keep them (roadmap 3.8.x).
  Sessions expire, the store holds at most `BOTE_SESSION_MAX` (4096) and answers
  503 when full of live ones, and a session is minted only after `initialize`
  has succeeded.
- **Tool dispatch** — handler functions are user-provided. Arguments are
  validated against the registered JSON Schema before invocation; validation
  failures are returned as error responses, not propagated. Registration warns
  about any schema keyword it advertises and does not enforce. Every call,
  refused ones included, reaches the audit sink, and the audited identity is the
  configured one: a client's `caller_id` is recorded as a claim, not as who acted.
- **Filesystem tools** — confined to `BOTE_FS_ROOT`. Since **3.4.15** a path may not
  pass through a symlink, `fs_write` replaces a symlink rather than writing through
  it, and a FIFO cannot hang the server. The check is by name, because the stdlib
  has no `openat`, so a local writer racing it inside the root could still swap a
  link in. No tool can create one. This is a floor, not a sandbox.
- **Outbound requests** — the `host` module's SSRF guard rejects URLs targeting
  loopback / link-local / private / cloud-metadata endpoints (IPv4 and IPv6)
  before any network call. `web_fetch`, the built-in tool that fetches a
  client-supplied URL, applies it in two layers: the URL's host string (IP
  literals and a hostname blocklist) since **3.4.0**, and since **3.4.4** the
  address a name actually resolves to, classified inside sandhi's client
  resolve hook, so the address judged is the address dialled (no DNS-rebinding
  window, and `127.0.0.1.nip.io`-style names are refused). Before 3.4.0 it
  checked only the scheme. Since **3.4.7** the IPv4 classifier also refuses
  CGNAT / tailnet space (100.64.0.0/10), 192.0.0.0/24 and 240.0.0.0/4
  (which holds the broadcast address). IPv6 literals are parsed to their 16
  bytes and judged there, so any spelling of an internal address is refused,
  and a literal that does not parse is refused too. 198.18.0.0/15 is
  deliberately allowed: fake-IP DNS proxies answer every name inside it.
  `web_search` is deliberately unguarded (its endpoint is the operator's
  `BOTE_SEARXNG_URL`). Since **3.4.15** both have a deadline
  (`BOTE_WEB_TIMEOUT_MS`, default 15 s), because every client waits on an outbound
  call. It does not cover DNS: sandhi's resolver has no timeout yet, so a lost
  reply can still stall the server.
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
- Every string bote writes into JSON is escaped per RFC 8259 §7, control
  bytes included (`bote_json_escape`, complete since 3.4.6), so a tool
  result cannot break the framing of the reply that carries it.
- Every message is checked to be one valid JSON text before it is read
  (`-32700` otherwise), and the jsonrpc version, the id type and the params
  type are validated on every request (`-32600`).
- Empty/missing tool names rejected before dispatch.
- Tool arguments validated against the registered JSON Schema before dispatch:
  types, enums, bounds, nested objects and required fields. The schema is
  compiled at registration since 3.4.8; until then only `required` was checked
  in production. A violation never reaches the handler; it is answered as a
  result with `isError: true` (MCP 2025-11-25, SEP-1303).
- Tool definitions are checked at registration (name, an object inputSchema,
  valid property schemas), and whatever a handler returns reaches the client as
  a CallToolResult that is valid JSON (3.4.8).
- JSON-RPC batch length capped to bound per-request allocation.
- Profile-split distribution (`dist/bote-core.cyr`) — core has minimal attack
  surface.
