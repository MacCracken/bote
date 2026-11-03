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
  the deployment environment. `bote streamable` serves on `BOTE_WORKERS` threads
  since 3.7.0 (4 by default), so a slow client holds one worker; every other
  transport serves one connection at a time. Since 3.4.3 every connection carries a send deadline (`BOTE_SEND_TIMEOUT_MS`,
  default 30 s), so a client that stops reading is dropped instead of wedging
  the server for everyone. Read deadlines: sandhi's idle and whole-request
  budgets on the HTTP family, opt-in `ws_config_with_idle_ms` on an upgraded
  WebSocket (`BOTE_WS_IDLE_MS` in the binary), opt-in `BOTE_UNIX_IDLE_MS` on the
  Unix transport — a quiet client holds those two single-threaded servers unless
  the operator opts in. The send deadline is per write, so a client that keeps
  reading a little at a time is not dropped: on streamable it holds a worker, and
  `BOTE_WORKERS` such clients hold them all; on the other transports one holds
  the server. Since **3.7.2** a `subscriptions/listen` stream on `http`, `bridge` or
  `streamable` is held by one thread of its own, not by a worker or the serving loop,
  and at most `BOTE_LISTEN_MAX` (256) are open at once; past that a listen gets
  `-32603`. Each holds one socket and keeps nothing on the heap. The thread never
  waits on a client: a frame the socket cannot take whole ends that stream, as does a
  client that closes or sends anything. Over TCP that frame comes only once the
  client's receive window is full, about a megabyte, which at one keepalive every
  15 s is days: a client that stops reading keeps its stream that long, and one that
  reads its keepalives keeps it for good. So the bound is on memory and sockets, not on
  who holds the slots, and one client can hold them all (roadmap, open decisions).
  Since **3.7.3** the socket a stream will be is held, close-on-exec, before anything
  is sent, and one that cannot be is refused with `-32603` and a WARN. Leave the process's file limit room for
  `BOTE_LISTEN_MAX` sockets beside the rest. A stream's token is checked when it
  opens and not again; it carries only that a list or a resource changed, never what is in it.
  Since **3.7.3** a listen on stdio, a Unix socket or a WebSocket is the connection
  itself: at most `BOTE_LISTEN_MAX` per connection, nothing per listen on the heap, and
  the changes written by the connection's own loop, under the same send deadline as its
  replies, so a client that stops reading is dropped as it was. On a WebSocket the token
  is judged again before a change goes out, as before a frame is served, so a connection
  whose token has expired is closed with 1008 rather than told of a change. The idle
  deadlines count from what the client last sent, not from what bote sent it.
  Since **3.7.4** a listen can ask for resource updates. What it keeps of its URIs is
  bounded, at most 32 in 4 KB, in a room per stream slot made the first time that slot
  needs one and kept for the next, so a listen allocates nothing after its slot's first
  and the whole is at most `BOTE_LISTEN_MAX` rooms. An update carries a URI and no
  payload, waits in a queue of 16 per hub or connection, and one that cannot be queued
  tells every subscription of every URI it holds rather than being lost.
  Since **3.7.5** a resource's required scopes hold for its updates. A listen made with a
  token is honored no URI the token may not read, judged by what would serve it as
  `resources/read` judges it, and is told of an update to a URI that requires a scope only
  when it holds that URI exactly: a stream keeps no token, so it is not told of one through a
  prefix, nor of a URI whose scope came after it listened. Through 3.7.4 a token without the
  scope heard of every change to a scoped resource, by name or through a prefix, and learned
  the URIs behind a scoped template. A listen made with no token compares nothing, as
  `resources/read` does. The thread that writes the streams gives each frame's bytes back
  once it is sent, so a pass costs one frame: through 3.7.4 it kept them all, 38 MB for 256
  streams told of 16 updates of a kilobyte URI, for the life of the process.
- **Cancellation** — since **3.7.5** a client can end a request of its own early, and no
  other: on `http`, `streamable` and `bridge`, for MCP 2026-07-28, by closing its connection;
  on stdio and a Unix socket by `notifications/cancelled` on its own connection. That is read
  only from what has already arrived, within the transport's buffer, and each line once; the
  check gives back what it allocated. A handler stops only if it asks (`bote_cancelled()`),
  and nothing more goes out for a request once one has: a client that cancels gets less, never
  more. The legacy HTTP path never asks, since 2025-11-25 says a disconnect is no cancellation.
- **Browsers** — since **3.4.15** every HTTP-family transport refuses a present
  `Origin` that is not on `BOTE_ALLOWED_ORIGINS` with 403, before anything else
  runs, and with no list set every present `Origin` is refused, except since **3.5.2**
  the server's own loopback address: `http://` plus the request's `Host`, when that
  `Host` is `localhost`, `127.0.0.1` or `[::1]`. Nothing on bote's port serves a page,
  and a rebinding page's `Origin` carries the attacker's name, so that one cannot be a
  website. Another local port is still refused. No `Origin` passes, because a
  command-line client sends none. Through 3.4.14 the transports checked
  only when a list was configured, the binary offered no way to configure one, and
  `bote bridge` answered `Access-Control-Allow-Origin: *` with its file tools
  registered, so any website the operator visited could read and write under
  `BOTE_FS_ROOT`. `*` in the list is the operator's explicit choice and logs a WARN.
- **Headers on MCP 2026-07-28** — a gateway can route on `Mcp-Method` and `Mcp-Name`
  without reading the body, so since **3.5.1** bote refuses a request whose headers
  disagree with its body (`400` + `-32020`), after decoding the Base64 sentinel. What
  a header claimed is what runs. Sessions play no part on this path, and the bearer
  and Origin checks come first, as on the legacy path. Since **3.5.2** that covers
  `Mcp-Param-{Name}` too, a repeated field is refused, because a proxy that reads the
  last line would act on a value bote never compared, and `bote bridge` holds a modern
  request to the same rules. A tool's author should not mark a secret with
  `x-mcp-header`: the value goes out in a header, visible to every intermediary. bote
  logs no header value, and a refusal names the header and not its value.
- **Protected Resource Metadata** — since **3.6.0** a server can publish RFC 9728 metadata, and
  it is public on purpose, because a client reads it to learn where to get a token. It says
  nothing a client could not be told. The `resource` in it comes from the operator and
  never from a request: a client compares it with the URL it used, and that is its defence
  against impersonation, so a `Host` an attacker chose must not reach it. Every value is checked
  at startup, so none can end a quoted header parameter or split a line, and what fails
  stops the server. The document says where to get a token and enforces nothing: the
  bearer validator does that, and a server with metadata and no validator logs a WARN.
- **Tokens** — MCP says a server must validate that a token was issued for it, and since
  **3.6.1** `auth_validator_jwt_hs256` does, given an audience: a good signature on a token
  meant for another server gets a 401, which is what stops a confused deputy. Every
  check runs on bytes the signature has already covered, and the decoder takes one spelling
  of a token, so a token's text can key a revocation list. A key under 32 bytes does not start.
  Through 3.6.0 the verifier decoded a client's bytes onto the heap, so a forged token with a
  60 KB header kept 45,024 B per request. Nothing called it then, and it now uses the request
  arena. A validator an embedder supplies has to check the audience itself. HS256 means the
  key is shared with the authorization server, so anyone holding it can mint a token for
  this server.
- **Public keys** — since **3.6.2** RS256 and ES256 tokens verify against a JWK Set the operator
  supplies. Algorithm confusion cannot happen here by construction: a key has one algorithm, a
  token's `alg` only selects among keys of it, a verifier holds a shared secret or public keys and
  never both, and a public key never reaches the HMAC. Nothing in a token can supply a key:
  `jwk`, `jku`, `x5u` and `x5c` are not read, and `crit` is refused. A set holding a private key
  stops the server. The cost is real and is the attack: a verify takes about 10.5 ms for ES256 and
  1.3 ms for RS256 on one thread, for a forged token as much as a good one, and it needs no
  credentials to reach. Put a rate limit in front of a server that takes tokens from the internet.
  ES256 also has two spellings of every signature, so a token's text cannot key a revocation list
  there: use `jti`.
- **Keys from a `jwks_uri`** — since **3.6.6** `BOTE_JWKS_URL` fetches the authorization server's
  key set: once at startup, which must succeed, again every `BOTE_JWKS_REFRESH` seconds (600), and
  when a token names a `kid` the set does not hold, never more often than a 60 s cooldown, so a
  client sending made-up kids gets the server one fetch a minute. While the server is down the last
  good set keeps serving, and asks back off to the refresh interval. The URL is the operator's, so the
  SSRF guard, which would refuse an authorization server on the same network, does not apply; what
  does is `https://`, against the system's roots or `BOTE_JWKS_CA`, or `http://` to a loopback
  address, which no one on a network can answer for, no userinfo, no redirect, at most 1 MiB, and a
  5 s deadline for the whole fetch, its DNS lookup included: it is made in a child that is killed at
  the deadline, which also keeps sandhi's 175 KB per TLS connection off the server's heap. A set that
  changes forgets every remembered token, so a key the authorization server removes stops passing
  tokens within one refresh. A plain-http URL to anywhere but loopback would let anyone on the path
  mint tokens, and is refused.
- **Tokens already verified** — since **3.6.4** a token that passed is remembered by the SHA-256 of
  its whole text, with its `exp` and `nbf`, and judged again on the clock each time it returns, by
  the same rules: the cache can only answer what the verify would. Changing the audience or the
  issuers forgets everything. A token that failed is never stored, so a forged token costs what it
  did, and the cache gives that time back to legitimate clients only. bote revokes nothing, cache or
  not: a token stays good until its `exp`, so keep access tokens short-lived. `BOTE_JWT_CACHE=0`
  turns the cache off. Since **3.6.6** a key set that changes forgets everything too.
- **WebSocket upgrades** — cyrius's `ws_server_handshake` copies four of the client's headers to the
  heap before it judges them, and through **3.6.3** a refused upgrade with a 60 KB header kept 60 KB,
  with no token needed on `bote ws`. Since **3.6.4** bote judges those headers on its request arena
  first, by the handshake's own rules and with `Connection` at most 256 bytes, so a refused upgrade
  keeps nothing. A completed one keeps about 1.6 KB, all of it the handshake's: a client that can
  connect can still grow the heap by that much a connection, and only an upstream fix ends it.
  Since **3.6.6** a connection's token is judged again by the validator that took it, at most once a
  second, so a token that expires, or whose key leaves the set, ends the connection with close code
  1008 within a second, where it used to last as long as the connection.
- **Scopes** — since **3.6.3** a tool can require scopes, and a token lacking any of them gets a
  `403` with `error="insufficient_scope"`. Since **3.6.6** so can a prompt, a resource and a
  resource template, judged by whatever would serve the request, and a `completion/complete` is held
  to the scopes of the prompt or template its `ref` names, because what it completes is theirs. The dispatcher holds the check, and HTTP's 403 is a
  courtesy ahead of it: a transport-only check would be walked around by a JSON-RPC batch and would
  miss WebSocket, so a batch element or a frame gets `-31403` from the same code. Fuzzing asserts no
  substitution of a request reaches a scoped tool without its scope. A tool that requires a scope
  with nothing configured to read one is refused, not waved through, and a name in
  `BOTE_TOOL_SCOPES` that matches no tool stops the server. With no token in play there is nothing to
  compare, so scopes do not apply on stdio or the Unix socket, and bote logs a WARN there. Since
  **3.7.2** a call is judged and served by one look at the
  registries: through **3.7.1** a tool registered while a call for it was in flight could run for a
  token without its scope, since the check found no tool and the handler was found after, and a
  prompt or resource replaced meanwhile was judged by one registration and served by another.
- **Memory** — `alloc` never frees, so what a request leaves on the heap stays for
  the life of the process. Since **3.4.15** everything bote allocates to answer a
  message goes on a request arena that the next message rewinds, including
  refusals before and after auth, so an unauthenticated client cannot grow the
  heap with a refused request. Until **3.5.3**, `resources/list` and `prompts/list` were an
  exception past 4 KB of list: 200 entries kept up to 128,792 B per request, for any
  client. That was only true for short values until **3.5.1**:
  through 3.4.15 a long `id`, method, name, URI or cursor retained 61 to 577 KB per
  refused request, and through 3.5.0 a long `MCP-Protocol-Version` retained 84 KB.
  Through **3.6.2** every request an operator's validator accepted kept its token, a
  kilobyte or so for a JWT, and a 60 KB token kept 60 KB per request: it was copied for a
  handler before anything knew one would run. That took a valid token to reach. A token now
  reaches the heap only when a handler is handed it.
  A valid tool, prompt, resource or completion call still leaves
  what its handler is handed and whatever the handler allocates, because the 2.0
  handler ABI lets a handler keep them (roadmap 3.8.x). On a server without auth,
  that means any client that can call one.
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
  call. Since **3.6.5** each request is made in a child process
  (`src/fetch.cyr`), which the parent kills at that deadline, DNS included, and
  which takes away what sandhi keeps on the heap for an `https://` request: about
  175 KB each, which a client calling either tool could otherwise have grown the
  server by for good. The parent reads back only the record the child writes,
  bounded in size.
- **Request heads** — since **3.6.5** a request whose head holds a NUL byte is
  refused with a 400 on `http`, `streamable`, `bridge` and `ws`, before any header
  is read (RFC 9110 §5.5). bote reads a value up to a NUL and the stdlib's
  WebSocket handshake reads it to the end of its line, so the two would otherwise
  judge different bytes: 3.6.4's bound on what an upgrade keeps was passed that way.
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
| 3.7.x   | Yes (current minor) |
| 3.6.x   | Yes (prior minor) |
| < 3.6   | No        |

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
