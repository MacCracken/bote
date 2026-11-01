# Bote Roadmap

> **Forward-facing only.** What shipped is in [CHANGELOG.md](../../CHANGELOG.md), per release. The
> current version is in [`VERSION`](../../VERSION), pins are in [`cyrius.cyml`](../../cyrius.cyml),
> and test and benchmark counts are in [`CLAUDE.md`](../../CLAUDE.md)'s testing table.
>
> **Spec**: bote's default is MCP 2025-11-25. The stateless core of **2026-07-28** is in, on stdio, Unix
> and WebSocket; Streamable HTTP is next.
> [spec-compliance.md](../spec-compliance.md) · [SECURITY.md](../../SECURITY.md) ·
> [2026-11-01 closeout audit](../audit/2026-11-01-closeout.md)

Work here is decided on bote's own merits: the MCP specification, bote's correctness and security,
and its performance. A consumer's missing feature is not a roadmap item. A bug a consumer reports is
ordinary work: reproduce it in bote and fix it.

## Where bote stands

The 2.0 handler ABI (`fn h(args, claims) → result_cstr`) and the six transports are stable across the
2.x → 3.x line, and patch releases add capabilities, not shape changes. The protocol surface is
`initialize`, `tools/*`, `prompts/*`, `resources/list` + `read`, `completion/complete`, `ping`, and a
polled push of `tools` / `prompts` `list_changed`. Four revisions are accepted through `initialize`,
2024-11-05 through 2025-11-25, and every path is asserted by spec clause in `tests/conformance.tcyr`.
One binary carries every transport, on x86_64, aarch64 and agnos, all CI-gated.

**The gap that orders everything below:** MCP 2026-07-28 is a breaking revision, and bote speaks it only
in part. Since 3.5.0 a request carrying `_meta` `io.modelcontextprotocol/protocolVersion` takes the
stateless path, with `server/discover` and the modern result shape, and an `initialize` takes today's
path, in one process. Every transport that reads `MCP-Protocol-Version` still refuses the new version
in that header, and a conforming client sends it on every POST, so a modern client reaches bote over
stdio and Unix, and over WebSocket when it sends no such header, but not yet over Streamable HTTP.

---

## Next patch

**3.5.1 — Streamable HTTP, modern path.** See below. A defect found in the 3.5.0 core lands ahead of it.

## 3.5.x — MCP 2026-07-28, dual-era

The largest open item, and the first. The revision removes `initialize`, `Mcp-Session-Id`, `ping`,
`logging/setLevel`, `resources/subscribe`, the Streamable HTTP `GET` stream and `Last-Event-ID`. The
protocol becomes stateless: each request carries its own version and capabilities. **Large: small bites,
each verified before the next.**

1. **Streamable HTTP, modern path (3.5.1).** `validate_protocol_version` accepts `2026-07-28`.
   `MCP-Protocol-Version` on every POST, checked against `_meta`. `Mcp-Method` and `Mcp-Name`, with
   mismatches refused as `400` + `-32020`. One message per POST. `202` for a notification. `404` +
   `-32601` for an unknown method. `405` for a modern `GET` or `DELETE`. `Mcp-Session-Id` and
   `Last-Event-ID` ignored on the modern path. Read `basic/transports/streamable-http` first.
2. **Client capabilities (3.5.1 or later).** `-32021` belongs to a request that needs a capability the
   client did not declare. No request bote serves needs one today, so 3.5.0 requires the object and reads
   nothing from it. The first feature that needs one adds the check.
3. **Conformance and the default (3.5.2).** Run the 2026-07-28 clauses over every transport, keep the
   legacy clauses passing on the same binary, and then decide whether the modern revision becomes the
   documented default.

Nothing here touches the handler ABI. The legacy path stays until a 4.0 removes it.

## 3.6.x — Authorization on HTTP

bote is a resource server with bearer auth, and the 2026-07-28 authorization spec has requirements it
does not meet. Each is a `MUST` or `SHOULD` in the spec, and authorization as a whole is optional, so
this applies when an operator turns auth on.

1. **OAuth 2.0 Protected Resource Metadata (RFC 9728):** the well-known endpoint, which servers MUST
   implement. Read `basic/authorization/authorization-server-discovery` for the path and fields.
2. **`WWW-Authenticate`** on 401 with `resource_metadata` and `scope`, and `403` with
   `error="insufficient_scope"` when a token lacks a scope.
3. **Audience validation (RFC 8707).** Servers MUST validate that a token was issued for them, and today
   `jwt.cyr` reads neither `aud` nor `iss`.
4. **RS256 and ES256.** An OAuth 2.1 authorization server does not share an HS256 secret with its
   resource servers, so HS256 alone cannot meet item 3 in practice. sigil has the primitives
   (`rsa_pkcs1v15_verify_sha256`, `ecdsa_p256_verify`), and `_jwt_str_field_eq` already reads `alg` as an
   exact field, which is the precondition for accepting more than one algorithm.

## 3.7.x — Threaded dispatch and `subscriptions/listen`

Rebased on the current spec. A held-open stream on the single-threaded sandhi loop starves the POSTs
that feed it, and `subscriptions/listen` is such a stream. **Large: small bites.**

1. Worker-thread dispatch behind the streamable transport, with the accept loop never blocking on a
   handler. This also ends one slow client holding every other client, which the send deadline
   (per write, not per response) cannot do on its own.
2. Per-worker state. Today's process-wide buffers become per-worker: the request arena
   (`bote_req_alloc`), `_fs_read_buf`, `_ws_frame_buf`, `_unix_buf`. sandhi's worker arena is fixed-size
   and fails when full, so it needs its grow option or a large reply crashes.
3. `subscriptions/listen`, with `toolsListChanged`, `promptsListChanged`, `resourcesListChanged` and
   `resourceSubscriptions`, `notifications/subscriptions/acknowledged`, and
   `io.modelcontextprotocol/subscriptionId` tagging. Then `notifications/resources/updated`.
4. Cancellation: on Streamable HTTP closing the response stream cancels the request, and on stdio
   `notifications/cancelled`. `bote_cancel_token_*` is the data layer.
5. `notifications/progress` on a request's own response stream.

Not planned, because the current spec removes or deprecates them: the held-open `GET`,
`resources/subscribe`, `logging/setLevel`, and new work on `Last-Event-ID` resumption.

## 3.8.x — The tool surface

- **`outputSchema`**, in the spec since 2025-06-18 and absent here. `structuredContent` already goes out
  on bote's own libro tools.
- **`title` and `icons`** on tools, prompts and resources, and `instructions` from the server. Check their
  status against the 2026-07-28 schema first.
- **Multi round-trip requests.** `InputRequiredResult` is how a server now asks a client for input. It
  needs an additive handler entry beside the 2.0 ABI.
- **JSON Schema coverage.** The 2026-07-28 revision allows any 2020-12 keyword. bote validates a subset and,
  since 3.4.15, warns at registration about the keywords it advertises and does not enforce: `anyOf`,
  `oneOf`, `allOf`, `not`, `$ref`, `pattern`, the length and item bounds, `const`, type arrays and
  non-string enums. Enforce them, with the resource bounds the spec asks for on `$ref` and composition.
- **`x-mcp-header`**, optional for a server.
- **What a valid tool call still leaves on the heap.** Since 3.4.15 a request costs bote nothing that
  outlives it, except `args` and whatever a handler allocates, because the 2.0 ABI lets a handler keep
  both. bote's own handlers need not: give them the request arena, and bound the binary. The ABI change
  that would bound every handler is a 4.0 matter.

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
  still grows, slowly. The legacy path keeps sessions, so this outlives 3.5.x.

## Open decisions (no version)

- **The tool set streamable and ws expose.** stdio, http, unix and bridge register the echo references
  plus the libro, fs and web tools; streamable and ws register the echo references only, which 3.4.2 kept
  so that a consolidation would not widen what two network transports expose. Either every mode gets
  the full set, or the fs and web tools become stdio-and-unix-only. `scripts/transport-smoke.sh` pins
  the current shape.
- **`libro_retention`'s archive.** `chain_apply_retention` returns what it removed, and the tool drops
  it, so "archived" means deleted. It also ignores `claims`. It reaches nothing in the binary, whose
  chain is empty, but an embedder with an audit sink gets a destructive tool any client can call. Hand
  the archive to a sink, gate the tool on claims, or stop registering it by default.
- **WebSocket subprotocol and compression.** WebSocket is not an MCP transport, so nothing requires
  these. Both would also need a seam in `lib/ws_server.cyr`.
- **The default protocol version.** Item 3 of 3.5.x changes it. Whether it changes in that minor or
  after a release of soak is a call for then.

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
- **The canonical-signature compare is invisible to CI.** On cyrius 6.7.18 the decoder refuses what the
  `memeq` in `jwt_verify_hs256` would, so only a run on 6.7.13 – 6.7.17 can see it removed.
- **`qemu-aarch64` `getrandom`.** If the sweep regresses to files exiting 90, suspect the emulator or the
  toolchain's `ESYSXLAT` rows before bote.
- **Benchmark rows that make a syscall move with the host kernel, and every row moves with host load.**
  Compare only within one boot, with an interleaved A/B.
- **libro-growth heisenbug (v1.2.1 era).** Not reproduced on any 2.x or 3.x tree, and
  `bote_core_only_smoke.tcyr` still exits inline because of it.

## 4.0 — what would force a major

None planned. Any one of these would:

- a change to the handler ABI `fn h(args, claims) → result_cstr`, including making `args` request-scoped;
- removing or reordering a field in a frozen 2.0 shape (`JsonRpcRequest` / `Response` / `Error`, `ToolDef`,
  `ToolSchema` / `ToolAnnotations`, `BoteError`, the transport configs, `McpSession` / `SessionStore`,
  `CompiledSchema` / `PropertyDef`); appending at the tail is allowed in 3.x;
- dropping a transport, a bundle profile, or a supported MCP protocol version, which is what retiring the
  legacy path after 3.5.x would be.

## Non-goals

- **Tool implementation** — bote dispatches to handlers, it doesn't implement business logic.
- **LLM integration** — that's hoosh.
- **Workflow orchestration** — that's szal.
- **Agent lifecycle** — that's daimon.
- **Storage** — that's patra (libro for audit, patra for general).
- **Authorization server** — bote is the resource server. OAuth 2.1 AS flow belongs alongside bote.
- **An MCP client.** bote is a server.

## Upstream

No open blockers. To file, all found by the 2026-11-01 audit or earlier, and none blocking:

- **sandhi — DNS has no deadline.** `_SANDHI_RESOLVE_TIMEOUT_MS` is reserved and unused, and the request
  deadline starts after the resolve, so a lost DNS reply can still hang `web_fetch` and every client
  behind it. bote's deadlines cannot reach it.
- **sandhi — `Transfer-Encoding : chunked`** (a space before the colon) is not recognised, where RFC 9112
  §5.1 says 400. `sandhi_server_content_length` reads a malformed value as 0, the same as absent; bote
  works around that one with `bote_http_clen_bad`. `sandhi_url_parse` has no userinfo handling.
- **`lib/ws_server.cyr`** ignores FIN and RSV, accepts unmasked client frames (RFC 6455 §5.1 says close), does
  not reassemble fragments, and `ws_server_recv` allocates a frame's claimed length before its payload
  arrives. bote reads frames through `ws_server_recv_frame` into one buffer.
- **`lib/str.cyr` `str_to_int`** skips non-digit bytes and wraps, though it documents "0 on invalid".
- **`lib/sakshi.cyr`** drops the trailing newline when a line fills its 256-byte buffer, gluing two records.
  bote caps its text below that.
- **`memcpy` is a byte loop**, about 20 instructions a byte. A word loop halves a `tools/list`.
- **A toolchain requirement a bundle can carry.** bote states its minimum on a comment line, because
  `cyrius distlib` writes no such header and an old compiler meeting a new construct gives a parser
  error. Either a `# Requires cyrius:` header or a version symbol for `#if` would let a bundle refuse an
  old toolchain by name.

bote's six toolchain filings are archived in the cyrius repo (`docs/development/issues/archived/`) and
indexed in [resolved-lang-issues.md](../resolved-lang-issues.md). New bote-side issues go under
`docs/development/issues/`, and toolchain issues are filed in the cyrius repo.
