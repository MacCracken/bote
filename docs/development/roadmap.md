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
`completion/complete`, and a polled push of `tools` / `prompts` `list_changed` (buffered per
session, drained on the client's next Streamable HTTP `GET` or piggybacked on a `POST`).
Two consumer bundles (`dist/bote.cyr`, `dist/bote-core.cyr`), three per-transport binaries,
x86_64 + aarch64 + agnos targets, all CI-gated.

What is **not** there, and where each item sits below: real-time *held-open* streaming
(→ 3.5.x), which is what `resources/subscribe`, `logging` and `$/cancelRequest` wait on;
WebSocket subprotocol / compression negotiation and DNS-aware SSRF (→ 3.6.x, both need an
upstream seam); and a handful of conformance gaps the 3.3.12 documentation sweep found by
probing the released binary (→ next patch).

---

## Next patch — 3.3.13

Small, independent, each a bite. All three were found by the 3.3.12 documentation sweep
and are recorded as ❌ in [spec-compliance.md](../spec-compliance.md) until they ship.

| Item | Why now | Effort |
|---|---|---|
| **`ping` answers `{}`.** The dispatcher routes no `ping`, so a `{"method":"ping"}` request gets `-32601 method not found`. Every MCP revision bote supports says the receiver *MUST* respond with an empty result, and SDK clients use it as a keepalive — an error reply reads as an unhealthy server. | Conformance defect on the released binary; one route in `dispatch.cyr` + an assertion. | Small |
| **Accept protocol version `2025-06-18`.** `validate_protocol_version` lists `2024-11-05`, `2025-03-26`, `2025-11-25`. Over stdio a `2025-06-18` client is negotiated up to the default; over the HTTP family the `MCP-Protocol-Version: 2025-06-18` header is a hard **400**, so a client pinned to that published revision cannot talk to bote at all. `2025-06-18` removed JSON-RPC batching from the spec — bote may keep accepting batches (a superset is harmless), but it must not reject the version. | One line in `session.cyr`; the compliance doc's version table gains a row. | Small |
| **Ship `src/sandbox.cyr`.** The kavach-shaped runner adapter is tested (`bote_sandbox.tcyr`, 13) and advertised in the README and the package description, but it is in neither `[lib]` profile, neither bundle and neither binary — the same orphan shape `jwt.cyr` / `pkce.cyr` had until 3.2.0. Decide the profile when it lands: it needs no sigil and no transport, so `[lib.core]` is admissible; whether a transport-free consumer wants a sandbox slot is the question. Update its header comment's kavach pin (3.12.2 → the current 3.12.x) in the same change. | Manifest line + `cyrius distlib`; the docs already say "not in either bundle". | Small |

Also worth taking in the same patch if it stays small: **`resources/templates/list`**
answers `-32601`. The method is optional in the spec, but some clients call it whenever
`resources` is advertised and treat an error as a failure rather than "no templates". An
empty `{"resourceTemplates":[]}` is the honest answer until a template registry exists.

## 3.4.x — Consolidation and timeouts

| Item | Notes | Effort |
|---|---|---|
| **Reconsolidate the per-transport binaries** — fold `bote-streamable` + `bote-ws` back into one `bote`, transport selected by argv. | The split was a cyrius 5.10.x compile-source cap workaround; the cap was raised at 6.1.24 (bote 2.7.3) and the trio has been carried since. `build-all.sh`, `release.yml`, the README run table and the six-transport round trip all simplify. The `[lib]` bundle is unaffected. | Small |
| **Port the conformance suite.** The Rust archive (tag `0.92.0`) carried 44 protocol-level scenarios; none was ported. `ping` would not have survived a conformance suite — that is the argument for doing this before, not after, the 3.5.x work adds more surface. Lands as `tests/conformance.tcyr`, driven through `codec_process_message` so it needs no live transport. | Medium |
| **`transport_unix` accept-loop deadline.** The one accept loop bote owns; its listen fd is never made non-blocking and carries no `SO_RCVTIMEO` (the file's own comment says so). Low severity — `AF_UNIX`, local-only, mode 0600. | Small |
| **Send-side timeouts.** `sandhi_server_run_opts` applies only `SO_RCVTIMEO`; `sock_set_send_timeout` is never reached on any path, so a stalled *send* is unguarded — the case that primitive's own docstring warns about. Low severity. | Small |

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

## 3.6.x — WebSocket extensions and DNS-aware SSRF

Both need a seam upstream that bote should **file, not wait for** — neither has been filed
with anyone:

| Item | Upstream seam needed | Then, in bote |
|---|---|---|
| **WS subprotocol negotiation** (`Sec-WebSocket-Protocol`) | `lib/ws_server.cyr`'s handshake reads only Upgrade / Connection / Version / Key and exposes no hook to read a request header or add a response header. | Read the offered list, echo one — `mcp` — in the 101. |
| **WS per-message deflate** (RFC 7692) | The same handshake / response-header seam. The codec is not a blocker: **sankoch** (`lib/sankoch.cyr`) ships a native DEFLATE with `deflate_compress` / `deflate_decompress`. | Negotiate `permessage-deflate`, wrap frames. |
| **DNS-aware SSRF** — catch `127.0.0.1.nip.io`-style bypasses | A resolve hook on the sandhi HTTP *client*, so the address it connects to is the one bote classified (no rebinding window). sandhi already carries an RFC 1035 resolver (`sandhi_resolve_ipv4`); what is missing is the client calling back before connect. | `ssrf_check` resolves and classifies the address, not only the literal. |

## Open decisions (no version)

- **JWT RS256 / ES256.** Not a dependency: sigil exposes `rsa_pubkey_from_der` +
  `rsa_pkcs1v15_verify_sha256` (verified end to end at 3.2.0 from a real SPKI PEM and an
  openssl-signed token) and `ecdsa_p256_verify` / `ecdsa_p256_verify_der` (checked at
  3.3.11). Not built because the one consumer that asked (agnosai) implements it locally —
  it needs `iss` / `aud` claim validation bote has no concept of, so routing through bote
  would be indirection over no shared code. Re-decide on a second consumer, not on a
  premise. Precondition already met: `_jwt_str_field_eq` reads `alg` as an exact field.
- **`[lib.core]` membership.** The profile exists to bound a transport-free consumer's
  compile set (t-ron's SecurityGate). `content.cyr` joined at 3.3.6 because every handler
  emits content blocks; `sandbox.cyr` is the next candidate (above). JWT / PKCE stay out —
  both need sigil, which the core footprint deliberately excludes.

## Housekeeping — rides any patch

- **zugot recipe** (`zugot/marketplace/bote.cyml`) reads `version = "2.7.6"` and has not
  tracked a patch since. CLAUDE.md's dev loop lists it as a version-check item; either
  re-sync it per release or drop it from the checklist.
- **README benchmark table** is a copy of one `history.log` row and rots per toolchain;
  the 3.3.12 sweep re-anchored it. Refresh it only on a toolchain bump, with the
  interleaved A/B in the CHANGELOG as the measurement and the table as the illustration.
- **Upstream filings** — the two seams in 3.6.x above.

## Watch list

Not tasks. Things that have bitten once and are checked, not assumed:

- **Thin-sigil vs fold version.** libro's `deps.sigil` selection wins over the fold's
  `lib/sigil.cyr` for 232 functions under last-definition-wins; benign only while the two
  are the same sigil (they diverged unnoticed at 3.3.8–3.3.9). Rule and check in CLAUDE.md.
- **libro-growth heisenbug (v1.2.1 era).** Heap-layout sensitivity when the chain grows
  while libro + majra + bote are all loaded. Not reproduced on any 2.x / 3.x tree;
  `bote_core_only_smoke.tcyr` still exits inline because of it. Retire the note if the
  3.5.x allocator work makes it unreachable.
- **`qemu-aarch64` `getrandom`.** The full suite runs under qemu since 3.3.11; through
  3.3.10 three files exited 90 there on a missing passthrough. If it regresses to that
  shape, suspect the emulator or the toolchain's `ESYSXLAT` rows before bote.
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

No open cyrius blockers. bote's five historical toolchain filings are archived in the cyrius
repo (`docs/development/issues/archived/`) and indexed with their fixes in
[resolved-lang-issues.md](../resolved-lang-issues.md);
[cyrius-feedback.md](../cyrius-feedback.md) is the port-era language-issue log, all of it
resolved by cyrius 4.4.3 and kept as a record. New bote-side issues go under
`docs/development/issues/` and move to `issues/archive/` with a closure banner when fixed;
toolchain issues are filed in the cyrius repo.
