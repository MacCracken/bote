# Bote Roadmap

> **Forward-facing only.** What shipped is in [CHANGELOG.md](../../CHANGELOG.md), per release. The
> current version is in [`VERSION`](../../VERSION), pins are in [`cyrius.cyml`](../../cyrius.cyml),
> and test and benchmark counts are in [`CLAUDE.md`](../../CLAUDE.md)'s testing table.
>
> **Spec**: bote's default is MCP 2025-11-25. **2026-07-28** is served beside it on every transport. The
> official client works on both, and the official suite passes 26 of the 37 scenarios 2026-07-28 requires.
> [spec-compliance.md](../spec-compliance.md) · [SECURITY.md](../../SECURITY.md) ·
> [2026-11-01 closeout audit](../audit/2026-11-01-closeout.md)

Work here is decided on bote's own merits: the MCP specification, bote's correctness and security,
and its performance. A consumer's missing feature is not a roadmap item. A bug a consumer reports is
ordinary work: reproduce it in bote and fix it.

## Where bote stands

The 2.0 handler ABI (`fn h(args, claims) → result_cstr`) and the six transports are stable across the
2.x → 3.x line, and patch releases add capabilities, not shape changes. Nothing below touches the ABI.
Authorization, when an operator turns it on, meets the 2026-07-28 spec's `MUST`s on every HTTP transport:
metadata, a token's audience, scopes with a step-up on tools, prompts and resources, and keys from a file or
from the authorization server's `jwks_uri`.

**What the official suite's required lists still fail** (`scripts/conformance.sh`), and where each is closed:

| Scenarios | Revision | Needs | Where |
|---|---|---|---|
| 10 `input-required-*`, and 3 checks of `server-stateless` | 2026-07-28 | `InputRequiredResult`, and `-32021` | 3.8.x |
| `logging-set-level`, `tools-call-with-logging`, `resources-subscribe`, `resources-unsubscribe` | 2025-11-25 | optional, and removed from 2026-07-28 | not planned |
| `tools-call-sampling`, `tools-call-elicitation`, 2 `elicitation-*` | 2025-11-25 | a server-to-client request, which 2026-07-28 replaces | not planned |

---

## Next patch

**3.8.1 — `title` and `icons`**, the next item of 3.8.x below.

Not planned, because the current spec removes or deprecates them: the held-open `GET`,
`resources/subscribe`, `logging/setLevel`, and new work on `Last-Event-ID` resumption. Nor workers
for stdio, unix and WebSocket: their buffers (`_ws_frame_buf`, `_ws_token_buf`, `_ws_q`, `_unix_buf`,
stdio's) are each one connection's, and a worker per connection would need a home for each.

## 3.8.x — The tool surface

- **`title` and `icons`** on tools, prompts and resources, and `instructions` from the server. Check their
  status against the 2026-07-28 schema first.
- **Multi round-trip requests.** `InputRequiredResult` is how a server now asks a client for input. It
  needs an additive handler entry beside the 2.0 ABI, which sees `arguments` and not `inputResponses`.
  It is also the first feature that needs a capability the client declares, so it adds `-32021` for a
  request that needs one the client did not: bote requires the client-capabilities object today and reads
  nothing from it. This is 10 of the 12 scenarios bote fails on the 2026-07-28 list.
- **Root keywords in `inputSchema`.** `ToolSchema` is type, properties and required, so a tool cannot say
  `$schema`, `$defs`, `allOf`, or the `additionalProperties: false` the spec recommends for a tool with no
  parameters. The suite's unscored `json-schema-2020-12` checks exactly that. An outputSchema is kept as its
  own JSON since 3.8.0, compiled from it and listed as written, which is the shape an inputSchema can take.
- **Output schemas on bote's own tools.** The libro tools already send `structuredContent`; give them, and
  `fs_read`, the outputSchema that says what it holds, so a client can check it.
- **JSON Schema coverage.** The 2026-07-28 revision allows any 2020-12 keyword. bote validates a subset and
  warns at registration about the keywords it advertises and does not enforce: `anyOf`, `oneOf`, `allOf`,
  `not`, `$ref`, `pattern`, the length and item bounds, `const`, type arrays and non-string enums. Enforce
  them, with the resource bounds the spec asks for on `$ref` and composition. And judge an object with one pass of the
  scanner for all of its names: each property and each required name is a scan of its own today, 2.6 µs for an object
  of three, which an outputSchema now pays on every result as an inputSchema does on every call.
- **What a valid tool call still leaves on the heap.** A request costs bote nothing that outlives it, except
  `args` and whatever a handler allocates, because the 2.0 ABI lets a handler keep both. bote's own
  libro, fs and web handlers need not: give them the request arena, and bound the binary. `fs_read` of a
  70 KB file keeps about 140 KB, its escaped result, for good, and `web_fetch`'s results are the same shape:
  build the result after the read buffer on the arena, move it down to the mark, and release past it, once
  nothing that keeps a result (a resumption buffer) is in the way. The ABI change that would bound every
  handler is a 4.0 matter.

## 3.9.x — Bote's own rules

- **`version` selects a schema but not a handler.** Handlers are keyed by name, so `tools/call` with an
  older `version` is validated and served as the latest. Either dispatch per version or refuse a
  version that is not the latest.
- **Namespace the bare public names.** `content_*`, `ann_*`, `prop_*`, `validate_tool_name`,
  `parse_property` break the project's own rule. Add prefixed names and keep the old ones as aliases
  for one minor.
- **Typed argument getters.** `jsonx_get_str` returns raw escapes and there is no public integer getter,
  so a handler has to know that. `jsonx_get_str_dec_a` and `_sch_parse_int` are the pieces.
- **Reclaim sessions.** A session's bytes are not freed when it expires, so a server churning sessions
  still grows, slowly. The legacy path keeps sessions until a 4.0 retires it.
- **A completion handler reads `ref` itself.** bote picks the prompt or template whose scopes a
  `completion/complete` needs from the first `ref`, as its one key scanner reads every key, and hands the
  handler the params as sent. A handler that reads a repeated `ref` last-wins answers for a target other
  than the one judged. Refuse a repeated `ref`, or hand the handler the one bote read. It is also judged in
  one look and served in another: a scoped prompt registered between the two gets completions for a token
  without the scope, the race 3.7.2 closed for `tools/call`, `prompts/get` and `resources/read`. Handing the
  handler what bote judged closes both.

## Open decisions (no version)

- **The default revision.** It stays 2025-11-25: a client that wants 2026-07-28 already gets it, so the
  default decides nothing on the wire, and bote should not name a revision whose required list it does not
  pass. Decide again when `InputRequiredResult` lands.
- **The tool set streamable and ws expose.** stdio, http, unix and bridge register the echo references
  plus the libro, fs and web tools; streamable and ws register the echo references only, kept so that
  folding the binaries into one did not widen what two network transports expose. Either every mode gets
  the full set, or the fs and web tools become stdio-and-unix-only. Nothing technical holds it any more:
  bote's own tools are safe on streamable's worker threads since 3.7.1. `scripts/transport-smoke.sh` pins
  the current shape.
- **How long a listen stream may be held, and by whom.** Over TCP a client that stops reading is not seen
  until its receive window is full, about a megabyte, which at one keepalive every 15 s is days; and a client
  that reads its keepalives is never dropped. So `BOTE_LISTEN_MAX` bounds memory, not whether another client
  can listen: one client opening 256 streams takes every slot. A stream's age (the server ends it with its
  result, and the client listens again), a bound per client or per token, or both. Either changes what a
  client that never listens again sees, and the TypeScript SDK's server has neither.
- **A listen stream's token.** A stream keeps no token, so its token is never judged again after the
  listen (a WebSocket's is, by the connection), and a stream made with claims is told of an update to a
  URI that requires a scope only when it holds that URI exactly and the scope held when it listened:
  not through a prefix, and not for a scope added since. Keeping each stream's claims in its slot, and
  judging them per update and per keepalive, would tell a token that has the scope everything it may
  read, and end a stream whose token expired. The claims are opaque to bote, so this needs the
  validator to say how long they hold.
- **A listen the server ends.** The cancellation page says a server MUST send `notifications/cancelled`
  for a listen it tears down; the subscriptions page says it SHOULD answer the listen with its result.
  bote ends a listen of its own accord only when nothing is honored, with the result, and when a hub
  thread cannot start, by closing the stream. The TypeScript SDK never answers a listen: it closes
  the stream on HTTP and sends `notifications/cancelled` on stdio.
- **`libro_retention`'s archive.** `chain_apply_retention` returns what it removed, and the tool drops
  it, so "archived" means deleted. It also ignores `claims`. It reaches nothing in the binary, whose
  chain is empty, but an embedder with an audit sink gets a destructive tool any client can call. Hand
  the archive to a sink, gate the tool on claims, or stop registering it by default.
- **WebSocket subprotocol and compression.** WebSocket is not an MCP transport, so nothing requires
  these. Both would also need a seam in `lib/ws_server.cyr`.

## Housekeeping — rides any patch

- **zugot recipe** (`zugot/marketplace/bote.cyml`) reads `version = "2.7.6"`. CLAUDE.md's dev loop lists it
  as a version-check item: either re-sync it per release or drop it from the checklist.
- **README benchmark table** is one `history.log` row. Refresh it on a toolchain bump, with the interleaved
  A/B in the CHANGELOG as the measurement.

## Watch list

Not tasks. Things that have bitten once and are checked, not assumed:

- **A stdlib upgrade can change what a parser accepts.** bayan 1.11.1 refused repeated JSON keys and made
  its base64 decoders stricter, and bote's tests stayed green over both. At a pin move, read the fold's
  own changelog for what its parsers now refuse.
- **A name a deprecation suggests may be newer than the bundles' minimum.** Before adopting one, or any
  stdlib private, check which fold first carries it. `scripts/bundle-min-cyrius.sh` measures the result.
- **Thin-sigil vs fold version.** Since cyrius 6.6.11 libro's `deps.sigil` selection is not vendored here,
  so the CI step reports "no thin sigil vendored". If it ever finds one, the resolver rule has changed.
- **`qemu-aarch64` `getrandom`.** If the sweep regresses to files exiting 90, suspect the emulator or the
  toolchain's `ESYSXLAT` rows before bote.
- **Benchmark rows that make a syscall move with the host kernel, and every row moves with host load.**
  Compare only within one boot, with an interleaved A/B, on a quiet host.
- **A null key to a stdlib map crashes on some runs only.** The hash is seeded per process and a null key looks in
  slot 0, so a missing guard passes most test runs. CLAUDE.md has the rule.
- **libro-growth heisenbug (v1.2.1 era).** Not reproduced on any 2.x or 3.x tree, and
  `bote_core_only_smoke.tcyr` still exits inline because of it.

## 4.0 — what would force a major

None planned. Any one of these would:

- a change to the handler ABI `fn h(args, claims) → result_cstr`, including making `args` request-scoped;
- removing or reordering a field in a frozen 2.0 shape (`JsonRpcRequest` / `Response` / `Error`, `ToolDef`,
  `ToolSchema` / `ToolAnnotations`, `BoteError`, the transport configs, `McpSession` / `SessionStore`,
  `CompiledSchema` / `PropertyDef`); appending at the tail is allowed in 3.x;
- dropping a transport, a bundle profile, or a supported MCP protocol version, which is what retiring the
  legacy path would be.

## Non-goals

- **Tool implementation** — bote dispatches to handlers, it doesn't implement business logic.
- **LLM integration** — that's hoosh.
- **Workflow orchestration** — that's szal.
- **Agent lifecycle** — that's daimon.
- **Storage** — that's patra (libro for audit, patra for general).
- **Authorization server** — bote is the resource server. OAuth 2.1 AS flow belongs alongside bote.
- **An MCP client.** bote is a server.

## Upstream

No open blockers. Filed, each in its project's `docs/development/issues/`:

- **sandhi keeps every TLS connection on the global heap** (sandhi, `2026-11-02-bote-tls-connect-keeps-each-connection-on-the-heap.md`):
  about 175 KB per `https://` request, whatever allocator the caller gave. bote makes its outbound requests in a child, which
  keeps it off bote's heap. It reaches bote when a cyrius release folds the sandhi that fixes it.
- **`ws_server_handshake` keeps the client's headers on the heap** (cyrius, `2026-11-02-bote-ws-server-handshake-keeps-client-headers-on-heap.md`):
  about 1.6 KB per completed WebSocket upgrade. bote bounds the headers it copies, so a refused upgrade keeps nothing; an arena
  variant would make a connection keep nothing either.

To file, none blocking:

- **sandhi — accepted sockets are not close-on-exec.** A WebSocket connection, and since 3.7.3 its listens, lives as long
  as its client, and a program an embedder's runner executes inherits it. bote's own Unix transport accepts with
  `sys_accept4(sfd, 0, 0, 0)` and could pass `SOCK_CLOEXEC`; the HTTP family's sockets are sandhi's to mark.

- **sandhi — DNS has no deadline.** `_SANDHI_RESOLVE_TIMEOUT_MS` is reserved and unused, and the request
  deadline starts after the resolve, so a lost DNS reply hangs the request. bote's outbound requests are made
  in a child the parent ends at `BOTE_WEB_TIMEOUT_MS`, so it no longer holds bote; any other sandhi client is
  exposed. The stdlib's `net_resolve_ipv4` has a deadline, two tries of 2 s.
- **sandhi — `Transfer-Encoding : chunked`** (a space before the colon) is not recognised, where RFC 9112
  §5.1 says 400. `sandhi_server_content_length` reads a malformed value as 0, the same as absent; bote
  works around that one with `bote_http_clen_bad`. `sandhi_url_parse` has no userinfo handling.
- **sandhi — obsolete line folding.** A header continued on the next line is read as ending at the first, where
  RFC 9112 §5.2 says refuse it or join it. A proxy that joins it would see another value from the one bote compared.
- **`lib/ws_server.cyr`** ignores FIN and RSV, accepts unmasked client frames (RFC 6455 §5.1 says close), does
  not reassemble fragments, and `ws_server_recv` allocates a frame's claimed length before its payload
  arrives. bote reads frames through `ws_server_recv_frame` into one buffer.
- **`lib/bayan.cyr` `bayan_base64_decode`** has no arena form, so a decode of a client's bytes stays on the
  heap for good. `src/http_era.cyr` carries a small decoder of its own for that reason.
- **`lib/str.cyr` `str_to_int`** skips non-digit bytes and wraps, though it documents "0 on invalid".
- **`lib/hashmap.cyr` `map_get(m, 0)`** crashes when slot 0 is occupied and returns 0 when it is not, which the
  seeded hash turns into a crash on some runs. It should answer the same on every run.
- **`lib/sakshi.cyr`** drops the trailing newline when a line fills its 256-byte buffer, gluing two records.
  bote caps its text below that.
- **`memcpy` is a byte loop**, about 20 instructions a byte. A word loop halves a `tools/list`.
- **A toolchain requirement a bundle can carry.** bote states its minimum on a comment line, because
  `cyrius distlib` writes no such header and an old compiler meeting a new construct gives a parser
  error. Either a `# Requires cyrius:` header or a version symbol for `#if` would let a bundle refuse an
  old toolchain by name.

Toolchain and stdlib issues are filed in the cyrius repo, and sandhi's in the sandhi repo, whose source cyrius
folds into `lib/sandhi.cyr`. bote's own go under `docs/development/issues/`; its resolved toolchain filings are
indexed in [resolved-lang-issues.md](../resolved-lang-issues.md).
