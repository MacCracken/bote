# Bote Roadmap

> **Forward-facing only.** Nothing here records what shipped — that is
> [CHANGELOG.md](../../CHANGELOG.md), per release. The "Shipped" table that used to live
> here was retired at the 3.3.12 documentation sweep because every row of it paraphrased
> an entry there. The current version is in [`VERSION`](../../VERSION); pins are in
> [`cyrius.cyml`](../../cyrius.cyml); test / benchmark counts are in
> [`CLAUDE.md`](../../CLAUDE.md)'s testing table.
>
> **Spec**: MCP 2025-11-25 default · [spec-compliance.md](../spec-compliance.md) ·
> [SECURITY.md](../../SECURITY.md)

## Where bote stands

The 2.0 handler ABI (`fn h(args, claims) → result_cstr`) and the six transports are
stable across the 2.x → 3.x line; patch releases add capabilities, not shape changes. The
protocol surface is `initialize`, `tools/*`, `prompts/*`, `resources/list` + `read`,
`completion/complete`, `ping`, and a polled push of `tools` / `prompts` `list_changed`
(buffered per session, drained on the client's next Streamable HTTP `GET` or piggybacked on
a `POST`). Four MCP protocol revisions are accepted, from one list. Since 3.4.5 every one of
those paths, error paths included, is asserted by spec clause in `tests/conformance.tcyr`.
Operational logging goes through sakshi (`src/log.cyr`, `BOTE_LOG`) — distinct from the MCP
`logging` capability, which still waits on 3.5.x. Two consumer bundles (`dist/bote.cyr`, `dist/bote-core.cyr`), one binary with every transport selected by argv,
x86_64 + aarch64 + agnos targets, all CI-gated.

What is **not** there, and where each item sits below: real-time *held-open* streaming (→ 3.5.x),
which is what `resources/subscribe`, `logging` and `$/cancelRequest` wait on; and WebSocket
subprotocol / compression negotiation (→ 3.6.x, needs an upstream seam).

---

## Next patch — 3.4.7

| Item | Notes | Effort |
|---|---|---|
| **SSRF classifier gaps.** | Found while wiring the 3.4.4 resolved-address guard. IPv4 (`_ssrf_classify_v4_ab`): no rule for CGNAT 100.64.0.0/10 (Tailscale and carrier-internal ranges — the 3.3.7 sandhi filing described it as covered, it is not), 255.255.255.255 broadcast, 240.0.0.0/4 reserved, 192.0.0.0/24, 198.18.0.0/15 benchmarking. IPv6 string matcher (`_ssrf_classify_ipv6`): textual, so `0:0:0:0:0:0:0:1`, `0::1` and `fe9f::1` are not recognised — the byte classifier (`ssrf_classify_ipv6_bytes`) already decides correctly, so the fix is to parse the literal to bytes and reuse it. Both are judgement calls about what "internal" means; add each with a test naming the range. | Small |

## 3.4.x — the rest of the minor, one bite per patch

Each item lands as its own patch, verified before the next starts. The patch numbers are the
intended order, not a promise.

| Patch | Item | Notes | Effort |
|---|---|---|---|
| 3.4.8 | **Schemas compiled at registration; handler results checked** (audit B2–B4, G2 + G3). | Nothing compiles an input schema outside the tests: `registry_register` and `dispatcher_register_tool` never call `compiled_compile`, and neither does any `src/` module or consumer. Every tool in production takes `registry_validate_params`' fallback, which checks only that `required` keys are present, so the type / enum / bounds validator and 3.4.5's SEP-1303 path never run. Compile on registration (keeping `tool_def_with_compiled` as the explicit override), and refuse a schema whose `type` is not `"object"` or whose property value fails `jsonx_valid` — that check would have caught t-ron's and phylax's `Str`-for-cstr registrations. Also check what a handler returns: valid JSON and an object with `content`, else wrap it as a text block or answer an `isError` result (itihas returns bare records today; phylax returns a `Str` header). Audit and publish a result carrying `isError: true` as a failure, not `TOPIC_TOOL_COMPLETED`. ⚠ Wire-visible for consumers whose tools have typed schemas and loose callers: a wrongly typed argument that passes today becomes an `isError` result. Give it a migration note and say which consumers it reaches. | Medium |
| 3.4.9 | **The bundles state the toolchain they need** (audit B6 + B7, G13). | The 3.4.x core bundle fails to compile on cyrius 6.6.6 at `const BOTE_LOG_LINE_BYTES` (measured during the audit), the pin hoosh, szal, thoth and t-ron carry. The minimum (≥ 6.7.2) is written only in the 3.4.0 CHANGELOG and DEPS-PATTERN, not in the file a vendoring repo actually opens. The bundle header is written by `cyrius distlib`, so either a comment at the top of `src/log.cyr` (the first module in both profiles) or an upstream request for a `# Requires cyrius:` header line taken from the pin. In the same place, say that a repo that *vendors* the core bundle must declare `sakshi` itself, because the sidecar does not list it and the libro → patra path that supplies it to pinned consumers does not exist for a copied file. | Small |

## Consumer adoption track

From the [2026-10-15 consumer audit](../audit/2026-10-15-consumers.md), section 4. Additive API
that would let the MCP servers built without bote (jalwa, mneme), the consumers with
non-JSON-RPC front doors (daimon, itihas, stiva) and t-ron's security gate move onto it. None
changes the 2.0 handler ABI. Each is its own patch after the 3.4.x queue. Take them in the order a
consumer is ready to adopt them, not as listed, and name that consumer in the patch.

| Item | What | For |
|---|---|---|
| **Envelope-free entry points** (G4) | `dispatcher_call_tool(d, name, args, claims)` returning a CallToolResult (admission, validation, audit and events as `tools/call` does them), and a public tools-list builder (`_build_tools_list_result` is private). | daimon's builtins (matched by hand today), itihas (misusing `HostRegistry` to publish descriptions), stiva |
| **A pre-dispatch guard hook** (G5) | e.g. `dispatcher_set_call_guard(d, fp, ctx)`, run inside dispatch for every `tools/call`, so a gate sees batches and notifications and sits behind 3.4.5's admission checks instead of in front of them. | t-ron (and thoth through it), whose gate wraps `dispatcher_dispatch` from outside |
| **A handler context pointer** (G8) | An additive registration variant carrying a `ctx` the handler receives, beside the 2-arg ABI, which stays as it is. | jalwa (`plib`, `engine`), mneme (`manager`, `engines`), which would otherwise need globals |
| **Decoding and typed argument getters** (G9) | `jsonx_get_str` returns a string's raw escapes, and there is no int / bool / string-array getter. `_fs_unescape` exists but is private and full-bundle only. | jalwa (`max`), mneme (`limit`, `tags`), every handler that reads arguments |
| **Namespaced public names** (G12) | `content_*`, `ann_*`, `prop_*`, `validate_tool_name`, `parse_property` are bare. `content_text` / `content_audio` collide silently with agnostik and `content_image` by arity; `ann_new` already collided with sit's (thoth renames sit's copy with sed). Add prefixed names and keep the old ones as aliases for one minor. | any repo that vendors bote beside agnostik or sit |

Two larger items need a minor of their own and a design pass first:

- **An MCP client layer** (G10): initialize / initialized, session-id capture, the
  `MCP-Protocol-Version` header, and reading both error shapes. sandhi's ADR 0001 leaves
  protocol semantics to bote, but bote has no client side, so daimon hand-rolls a forwarder that
  cannot reach `bote streamable`. The transport defects stay sandhi's (dual `Accept`,
  notifications, response headers, depth-aware error detection).
- **Dynamic resources and prompts** (G11): a provider hook or real RFC 6570 templates in place of
  the exact-key registries, for a consumer whose set is a live map (agnosai).

## 3.5.x — Threaded dispatch (real-time push)

The one architectural item. Everything below is gated on it, and nothing upstream is:
cyrius's `lib/thread.cyr` (MPSC + mutex), `lib/async.cyr`, `thread_local_alloc` and the
`arena_*` family are complete and pinned. A held-open `GET` on the single-threaded sandhi
accept loop would starve the `POST`s that feed it, which is why the push path is polled
today. **Large — small bites only**, each verified before the next:

1. Worker-thread dispatch behind the streamable transport: a request queue (MPSC) and a
   per-session outbound channel, with the accept loop never blocking on a handler.
2. Held-open `GET` streams draining the outbound channel live, replacing the poll-on-next-
   request drain. `ResumptionBuffer` and `Last-Event-ID` semantics unchanged.
3. `logging` capability + `notifications/message` — advertise only once there is a producer.
4. `resources/subscribe` / `unsubscribe` + `notifications/resources/updated`, and
   `resources` `listChanged` (the builder already exists in `stream.cyr`; nothing calls it).
5. `$/cancelRequest` mid-stream — `bote_cancel_token_*` already exists as the data layer.
6. Per-thread request buffers — the process-global request buffers become per-worker
   (`thread_local_alloc` + arenas). Bote-side work; nothing upstream tracks it.
7. WebSocket arena-per-frame allocation (`arena_new_growable` + `arena_reset`, usable since
   cyrius 6.5.9; the fixed-capacity arena crashed on exhaustion before that). Independent of
   the thread work and can go earlier if a WS memory profile motivates it.

## 3.6.x — WebSocket extensions

Both need a seam upstream that bote should **file, not wait for** — not yet filed:

| Item | Upstream seam needed | Then, in bote |
|---|---|---|
| **WS subprotocol negotiation** (`Sec-WebSocket-Protocol`) | `lib/ws_server.cyr`'s handshake reads only Upgrade / Connection / Version / Key and exposes no hook to read a request header or add a response header. | Read the offered list, echo one — `mcp` — in the 101. |
| **WS per-message deflate** (RFC 7692) | The same handshake / response-header seam. The codec is not a blocker: **sankoch** (`lib/sankoch.cyr`) ships a native DEFLATE with `deflate_compress` / `deflate_decompress`. | Negotiate `permessage-deflate`, wrap frames. |

## Open decisions (no version)

- **The tool set streamable and ws expose.** In the one binary, stdio / http / unix / bridge
  register the echo references plus the libro, fs and web tools, while streamable and ws register
  the echo references only, the set they had as separate binaries through 3.4.1. 3.4.2 kept that
  deliberately: a consolidation should not widen what two network transports expose
  (`fs_write`, `web_fetch`). Unifying it is a product call: either every mode gets the full set,
  or the network modes all get the smaller one and the fs / web tools become stdio-and-unix-only.
  `scripts/transport-smoke.sh` pins the current shape and needs changing with it.

- **JWT RS256 / ES256.** Not a dependency: sigil exposes `rsa_pubkey_from_der` +
  `rsa_pkcs1v15_verify_sha256` (verified end to end at 3.2.0 from a real SPKI PEM and an
  openssl-signed token) and `ecdsa_p256_verify` / `ecdsa_p256_verify_der` (checked at
  3.3.11). Not built because the one consumer that asked (agnosai) implements it locally —
  it needs `iss` / `aud` claim validation bote has no concept of, so routing through bote
  would be indirection over no shared code. Re-decide on a second consumer, not on a
  premise. Precondition already met: `_jwt_str_field_eq` reads `alg` as an exact field.
- **HTTP guards and the SSRF classifier for core consumers** (audit G6 + G7).
  `validate_origin`, `validate_protocol_version`, the bearer / allowlist validators
  (`session.cyr`, `auth.cyr`) and the SSRF classifier (`host.cyr`) are full-bundle only.
  agnosai, mneme and hoosh run their own sandhi servers, and none checks Origin. szal, stiva and
  agnosai make outbound requests with hand-rolled or literal-only guards. Options: a third
  profile (`[lib.guards]`: core plus those three modules, no transports), or moving the pure
  helpers into core. Weigh it the way `content.cyr` and `sandbox.cyr` were weighed below: on
  which consumers would include it, not on fit.
- **`[lib.core]` membership.** The profile exists to bound a transport-free consumer's
  compile set (t-ron's SecurityGate). `content.cyr` joined at 3.3.6 on evidence — core
  consumers were hand-rolling content envelopes and re-implementing JSON escaping.
  `sandbox.cyr` was weighed at 3.3.13 and **kept out**: it fits mechanically (no sigil, no
  transport, no new stdlib leaf) but eight repos vendor the core bundle and none references
  a `sandbox_*` symbol. Revisit when a core-profile consumer wires a backend. JWT / PKCE
  stay out — both need sigil, which the core footprint deliberately excludes.

## Housekeeping — rides any patch

- **zugot recipe** (`zugot/marketplace/bote.cyml`) reads `version = "2.7.6"` and has not
  tracked a patch since. CLAUDE.md's dev loop lists it as a version-check item; either
  re-sync it per release or drop it from the checklist.
- **README benchmark table** is a copy of one `history.log` row and rots per toolchain;
  the 3.3.12 sweep re-anchored it. Refresh it only on a toolchain bump, with the
  interleaved A/B in the CHANGELOG as the measurement and the table as the illustration.
- **Upstream filing** — the `lib/ws_server.cyr` handshake seam the two 3.6.x items share.

## Watch list

Not tasks. Things that have bitten once and are checked, not assumed:

- **libro's sigil pin is one patch past the fold.** libro 2.11.0 pins sigil 3.13.12; cyrius 6.7.9
  folds 3.13.11, so `cyrius.lock` commit-pins 3.13.12 while `lib/sigil.cyr` is 3.13.11. libro reports
  the thin set differs only in its header; cyrius 6.7.10 folds 3.13.12, so the next pin move should
  close it. Check the sigil `commit` row against `lib/sigil.cyr`'s header then.
- **Thin-sigil vs fold version.** Since cyrius 6.6.11 libro's `deps.sigil` selection is
  not vendored here (the fold wins over a thin profile of a stdlib leaf), so the CI step
  reports "no thin sigil vendored". If it ever finds a thin copy again, the resolver rule has
  changed: the two must then be the same sigil (they diverged unnoticed at 3.3.8–3.3.9).
- **libro-growth heisenbug (v1.2.1 era).** Heap-layout sensitivity when the chain grows
  while libro + majra + bote are all loaded. Not reproduced on any 2.x / 3.x tree;
  `bote_core_only_smoke.tcyr` still exits inline because of it. Retire the note if the
  3.5.x allocator work makes it unreachable.
- **`qemu-aarch64` `getrandom`.** The full suite runs under qemu since 3.3.11; through
  3.3.10 three files exited 90 there on a missing passthrough. If it regresses to that
  shape, suspect the emulator or the toolchain's `ESYSXLAT` rows before bote.
- **`dist/bote-core.deps` should drop `sigil` and `ct`** once cyrius fixes the filing under
  Upstream. At each pin move, regenerate and read the sidecar: if it is still
  `hashmap bayan sigil ct`, the filing is open. If it is `hashmap bayan`, the fix has landed;
  say so in the CHANGELOG, since core consumers then fall back to libro's thin sigil (about
  −870 KB for a one-call consumer, measured at 3.3.17).
- **Benchmark rows that make a syscall move with the host kernel.** On the reference host
  (Ryzen 7 5800H, kernel 7.2.8, `spec_rstack_overflow: Safe RET`) one `clock_gettime` costs
  1.35 µs. That is what made `dispatch_tools_call` look +58% between the 3.3.13 row (logged on
  an older kernel) and every row after the 2026-10-02 kernel update: 3.3.13 rebuilt from its
  tag runs at the same speed as 3.4.0 on today's host. Compare such rows only within one boot,
  with an interleaved A/B; since 3.4.1 an unobserved `tools/call` makes no syscall at all.
- **Dependency pins move together.** libro ≥ 2.10.2 needs `O_NOFOLLOW` / `O_DIRECTORY`
  from cyrius ≥ 6.6.4; `cyrius deps` keeps the snapshot's `patra` over libro's transitive
  one and warns, which is the expected shape.

## 4.0 — what would force a major

None planned. Any one of these would:

- a change to the handler ABI `fn h(args, claims) → result_cstr`;
- removing or reordering a field in a frozen 2.0 shape (`JsonRpcRequest` / `Response` /
  `Error`, `ToolDef`, `ToolSchema` / `ToolAnnotations`, `BoteError`, the transport configs,
  `McpSession` / `SessionStore`, `CompiledSchema` / `PropertyDef`) — appending at the tail
  is allowed in 3.x;
- dropping a transport, a bundle profile, or a supported MCP protocol version.

## Non-goals

- **Tool implementation** — bote dispatches to handlers, doesn't implement business logic.
- **LLM integration** — that's hoosh.
- **Workflow orchestration** — that's szal.
- **Agent lifecycle** — that's daimon.
- **Storage** — that's patra (libro for audit, patra for general).
- **Authorization server** — bote is the resource server. OAuth 2.1 AS flow belongs
  alongside bote, not inside; bote supplies the substrate (bearer, JWT HS256, PKCE-S256).

## Upstream

No open blockers. Two open filings, neither blocking, and one to file:

- **To file with cyrius — a vendoring-friendly pin** (audit G14). A `[deps.bote]` limited to
  `modules = ["dist/bote-core.cyr"]` still pulls bote's own `deps.libro` / `deps.majra` into the
  consumer (t-ron shows it), which is the reason hoosh, szal and thoth vendor the bundle by hand
  instead, and why all three sit 13 releases behind. A resolver that honours the selected
  module's `.deps` sidecar, or deps scoped to a profile, would remove the reason.

- **cyrius** — `docs/development/issues/2026-10-15-bote-distlib-kept-fold-over-thin-profile-records-named-deps-need.md`.
  `distlib` records libro's sigil needs (`sigil`, `ct`) on `dist/bote-core.deps` because the
  resolver keeps the sigil fold over libro's thin profile and never credits the fold to libro.
  Measured cost and consumer numbers are in the 3.3.17 CHANGELOG entry. cyrius placed it in
  its 6.7.13 tools lane (2026-10-15): at the pin move that reaches 6.7.13, check the watch-list
  item on `dist/bote-core.deps`.
- **sandhi** — `docs/development/issues/2026-10-15-bote-server-run-failures-return-bare-1.md`.
  The five serve entry points return a bare `1` for every failure (the bind's `EADDRINUSE` is
  in a local and discarded), so bote's `transport failed` line can only name socket, bind,
  listen and accept together; `_run_opts` — bote's — and `_run_async` also leak the listen
  socket on each failed start. When sandhi ships a step + errno, `bote_main_log_exit` names
  it, the way the Unix transport already does.

bote's five historical toolchain filings are archived in the cyrius
repo (`docs/development/issues/archived/`) and indexed with their fixes in
[resolved-lang-issues.md](../resolved-lang-issues.md);
[cyrius-feedback.md](../cyrius-feedback.md) is the port-era language-issue log, all of it
resolved by cyrius 4.4.3 and kept as a record. New bote-side issues go under
`docs/development/issues/` and move to `issues/archive/` with a closure banner when fixed;
toolchain issues are filed in the cyrius repo.
