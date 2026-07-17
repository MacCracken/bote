# Changelog

All notable changes to bote are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### Conventions

Adopted in **2.7.0** — the **`## [Unreleased]`** section below
accumulates entries during the patch cycle. When a release ships,
the **`## [Unreleased]`** header is renamed to `## [VERSION] —
DATE — headline` and a fresh, empty `## [Unreleased]` is seeded
back at the top. Avoids the "TODO" placeholder churn we used to
have per release.

## [Unreleased]

_(empty)_

## [3.1.4] — 2026-07-17 — libro 2.8.2 (`LIBRO_ERR_*`) + `[deps.libro]` pin/lock realign

**libro dep bump.** `[deps.libro]` `2.8.1 → 2.8.2`. libro 2.8.2 namespaces its own `LibroErr`
enum — the upstream reciprocal of bote's `BOTE_ERR_*` rename at 3.1.3 — so the bare `ERR_IO` /
`ERR_JSON` collision is now resolved at the source on **both** sides of a libro-linked bote binary,
not merely avoided by bote's prefix. No bote logic change; wire contract identical.

### Changed
- **libro `2.8.1` → `2.8.2`** (`[deps.libro]` tag). Upstream, libro 2.8.2 renamed its `LibroErr`
  enum `ERR_* → LIBRO_ERR_*` (13 constants, e.g. `LIBRO_ERR_IO=3`) and moved its own internal deps
  to sigil 3.12.1 / patra 1.12.12 under a cyrius 6.4.66 pin. For bote this is transparent: the
  `audit_libro` / `libro_tools` adapters call libro's **function** API, not its error constants, so
  no bote source references the renamed tags. bote's vendored `lib/patra.cyr` is already 1.12.12 and
  bote keeps its own `[deps.sigil]` 3.12.0 pin, so nothing else in bote's dep set shifts. Rebuilt +
  retested against 2.8.2; `dist/bote.cyr` + `dist/bote-core.cyr` regenerated at v3.1.4.

### Fixed
- **`[deps.libro]` tag realigned with `cyrius.lock`.** 3.1.3 shipped with the tag reading `2.8.1`
  while `cyrius.lock` already recorded 2.8.2's content hash (`0fd0ff08…`) — the local
  `path = "../libro"` override had vendored the 2.8.2 body, masking the drift locally. A clean
  `git`+`tag` CI checkout would have vendored 2.8.1 (`7187f5b6…`) and **failed lock verification**.
  Bumping the tag to `2.8.2` makes tag ↔ lock ↔ vendored body agree (all three hashes verified
  equal); `cyrius.lock` itself is unchanged because it was already correct.

## [3.1.3] — 2026-07-17 — toolchain 6.4.66 + `BoteErrTag` namespacing (`BOTE_ERR_*`)

**Toolchain bump + error-tag namespacing.** The cyrius wrapper had already rolled to 6.4.66,
leaving the manifest pin (6.4.64) drifting and the local `lib/` snapshot stale enough that the
server binary no longer linked. This release re-pins to 6.4.66, re-syncs `lib/`, and namespaces
bote's `BoteErrTag` constants to escape a bare-token collision with libro. No transport, dispatch,
or wire-format behavior change; numeric error values and JSON-RPC code mapping are identical.

### Changed
- **cyrius pin `6.4.64` → `6.4.66`** — clears the "manifest-pin: 6.4.64 (drift — wrapper is
  6.4.66)" warning. Re-syncing the declared stdlib subset (`cyrius deps` + `cyrius lib sync`)
  refreshes `lib/thread_local.cyr` to the 6.4.65 **slot allocator** (`thread_local_alloc`) that
  sigil 3.12.0's `crypto_scratch` and patra 1.12.10's TLS slots now call — without it every binary
  failed to link with `undefined function 'thread_local_alloc'`. `cyrius.lock` picks up the
  refreshed `bayan 1.1.0 → 1.2.0` and `patra` snapshot hashes from the same sync; also clears the
  `./lib/` shadow warning. 6.4.66 itself is a Win64-only ≥10-arg call fix (per the 6.4.64 span) —
  inert on this target.

### Fixed
- **`BoteErrTag` constants namespaced `ERR_*` → `BOTE_ERR_*`** (all 13 tags,
  `BOTE_ERR_TOOL_NOT_FOUND` … `BOTE_ERR_TAG_COUNT`). The bare tags collided with `lib/libro.cyr`'s
  own bare `ERR_IO` (=3) / `ERR_JSON` (=4) — bote's are `=11` / `=10`. In the default binary and
  full bundle (which pull libro in via `libro_tools`), cyrius is single-pass and "last definition
  wins", so the shared token resolved to whichever module was included last — leaving
  `bote_err_rpc_code` / `bote_err_format` able to map an IO/JSON error against the wrong tag value.
  Namespacing removes the shared-symbol hazard entirely. Numeric tag values (0–12) and the
  JSON-RPC code mapping are unchanged, so the wire contract is identical. These are internal
  handler/dispatch/registry tags — no first-party consumer references the bare names; any
  out-of-tree consumer that referenced them must prefix `BOTE_ERR_`. `dist/bote.cyr` +
  `dist/bote-core.cyr` regenerated (53 tags each, 0 bare `ERR_`).

## [3.1.2] — 2026-07-16 — toolchain 6.4.64 + dependency refresh (libro 2.8.1, majra 2.5.1, sigil 3.12.0, sakshi pin)

**Toolchain + full dependency refresh.** No bote logic change — the release moves every pin to
its current tag, clears two per-build toolchain warnings (manifest-pin drift, sakshi lib-freshness
shadow), and banks upstream correctness fixes bote inherits: libro's audit-row quoting fix and
sigil's crypto-bank thread-local slot fix.

### Changed
- **cyrius pin `6.4.34` → `6.4.64`** — clears the "manifest-pin: 6.4.34 (drift — wrapper is
  6.4.64)" warning. Span highlights relevant to bote: `net.cyr` `sock_accept` per-poll alloc-leak
  fix (6.4.61 — all four socket transports sit on `net`), sandhi `signal_ignore`/SIGPIPE +
  `output_buf` 16 MiB → 1 GiB (6.4.51/.52), per-profile distlib `.deps` subsetting (6.4.48),
  DX diagnostics — column numbers, source carets, multi-error reporting (6.4.60/.62), and the
  lib-freshness shadow check (6.4.63) that surfaced the sakshi lag below. 6.4.64 itself is a
  Win64-only ≥10-arg call fix, inert on this target. `cyrius.lock` now records the
  declared-subset resolution (the 6.4.x `cyrius deps` behavior).
- **libro `2.7.10` → `2.8.1`.** 2.8.0 is a toolchain/dep refresh in which libro thinned its own
  sigil surface (its binary, not the `dist/libro.cyr` body bote consumes — that is unchanged);
  2.8.1 fixes `patrastore_append` to use a bound (prepared-statement) INSERT, so a `'` in any
  audit field no longer silently drops the row (integrity fix on the audit chain bote fronts via
  `libro_tools`). libro's `.deps` sidecar now also pulls **patra 1.12.10** — a new transitive
  `lib/patra.cyr` vendored by `cyrius deps` (the tests that include `lib/libro.cyr` already
  include it).
- **majra `2.5.0` → `2.5.1`** — toolchain/dep refresh; the dist bundle body is byte-identical
  (banner restamp only).
- **sigil `3.9.8` → `3.12.0`.** Banks 3.9.9's **crypto-bank thread-local slot fix (slot 0 → 8)**
  — slot 0 collided with patra's SQL scratch, corrupting banked crypto state in any process
  linking both, which bote now does (patra arrives via libro 2.8.x above). 3.10.x–3.12.0 add
  Authenticode/UEFI signing, per-primitive distlib profiles, and BLAKE2b + Argon2id — additive;
  bote's consumed surface (sha256 / hmac_sha256 / ed25519) is unchanged. The full
  `dist/sigil.cyr` bundle is retained: bote consumes two overlapping primitive families
  (SHA-2/HMAC + Ed25519), and majra 2.5.1's footprint review showed the thin per-primitive
  profiles overlap on ~121 fns for that shape (noisier than the full bundle's deduplicated
  closure).
- **6.4.x cyrfmt reflow** of `src/fs_tools.cyr` + `src/web_tools.cyr` (whitespace-only
  continuation-line indentation — same reflow class as the 2.7.6 formatter churn).
- `_bote_server_version()` → `"3.1.2"` (`src/dispatch.cyr`); `dist/bote.cyr` (6538 lines) +
  `dist/bote-core.cyr` (2569 lines) regenerated at v3.1.2.

### Added
- **Explicit `[deps.sakshi]` pin at `2.4.6`** (`dist/sakshi.cyr`, self-contained — links only
  stdlib `fnptr` + `atomic`). The stdlib registry still resolves `sakshi` → 2.4.3, which tripped
  6.4.63's lib-freshness warning on every build. Same registry-lag class as the sigil pin;
  brings sakshi 2.4.4's custom headers + W3C 128-bit trace-id along the way.

### Notes
- The pre-existing benign linker diagnostics are unchanged: `duplicate fn '_sub_new'`
  (lib/majra.cyr) and `duplicate fn 'cancel_token_new'` — the latter now root-caused as stdlib
  `async.cyr`'s own cancel-token (present since at least 6.4.34, semantically identical to
  `src/stream.cyr`'s: an 8-byte heap flag initialised to 0; bote's later definition wins).
- Verified: **786/786 assertions** across 12 test files + the core-only drift smoke; **14
  benchmarks** flat vs the 2.7.6 baseline (`schema_compile_nested` 9 → 7.5 µs; first entry
  logged at the 6.4.x sub-µs bench precision); capacity **59% / 61%** (`fn_table 4841/8192`,
  `identifiers 160696/262144`) — well under the 95% CI gate; `fmt` / `lint` / `vet` / `deny`
  clean; stdio `initialize` + `tools/call` round-trip smoke on the shipped binary
  (`serverInfo.version` = 3.1.2).

## [3.1.1] — 2026-07-09 — native HTTPS large responses (toolchain TLS fix), pin `6.4.34`

**`web_fetch` / `web_search` now work over the sovereign native TLS backend for
real-world (large) HTTPS responses.** 3.1.0 shipped the web tools calling the
sandhi client, which uses the default **native** TLS backend — but the stdlib
native TLS record layer had a size-dependent bug: any response whose body arrived
in a full 16 KB TLS record failed (`example.com` slipped through; anthropic.com /
cyriusb.com / secureyeoman.ai / robertmaccracken.com did not). No bote source
change was needed — the fix is entirely in the toolchain's stdlib TLS module,
picked up by the pin bump.

### Changed
- **cyrius pin `6.4.20` → `6.4.34`**, which carries the native TLS record-layer
  fix: (1) an off-by-one in the record-decrypt output buffer (a max-size TLS 1.3
  record's inner plaintext is `content(2^14) + type-byte` = 16385, above the old
  16384-byte scratch → `TLS_ERR_BUFFER_FULL` after the AEAD sequence advanced, a
  connection wedge), and (2) `tls_native_read` now delivers **partial records**
  (holding any remainder) instead of "whole record or error", so a caller reading
  in sub-record chunks no longer loses data. `web_fetch` verified over the native
  backend against all four hosts above (200, byte-lengths identical to libssl).

_No `web_tools.cyr` change_ — the earlier local libssl-backend fallback WIP was a
workaround for this now-fixed root cause and was dropped (never released).

## [3.1.0] — 2026-07-09 — web tools (web_fetch / web_search)

**Web MCP tools.** A new tool family, `src/web_tools.cyr`, alongside `fs_tools` / `libro_tools`, so an MCP
client (thoth, via daimon) can fetch pages and search the web through the spine — no reimplementation in the
consumer. Additive; registered by `web_tools_register()` and shipped in the default `dist/bote.cyr` bundle.
Outbound HTTP uses the sandhi client (the same transport the rest of AGNOS speaks). 25 assertions.

### Added
- **`web_fetch {url}`** — GET an `http://` / `https://` URL and return its **readable text**: HTML tags are
  stripped, `<script>` / `<style>` blocks dropped, the common named + numeric entities decoded, and runs of
  whitespace collapsed. Output is size-capped (64 KiB); non-http schemes are refused; a transport or non-2xx
  failure is reported, not faked.
- **`web_search {query, count?}`** — query a **SearXNG** instance (`BOTE_SEARXNG_URL`, configurable,
  self-hostable, no third-party key) via its JSON API and return the top results (title · url · snippet),
  parsed with bayan. An unset endpoint is reported honestly — no silent default to a third party.
- Registered in `src/main.cyr` (after `fs_tools_register`) and the `[lib]` distlib manifest. Tests in
  `tests/bote_web_tools.tcyr` cover the scheme guard, the HTML→text stripper, url-encoding, and entity decode.

### Security
- The HTML→text stripper **drops C0 control bytes / DEL / raw NUL** (both literal and `&#<n>;`-decoded) from
  the untrusted page — readable text has none, and it keeps `web_fetch`'s result valid JSON downstream (the
  shared escaper does not `\u`-escape control bytes, and a raw NUL would silently truncate the text).
  Adversarial-review-caught (untrusted-input lens); 27 assertions incl. the control-byte/NUL case.

## [3.0.1] — 2026-07-07 — bote_echo MCP conformance + toolchain 6.4.20

### Fixed
- **`bote_echo` now returns an MCP-conformant result.** The reference sample tool returned its
  arguments verbatim (a bare JSON object), which is not a valid `tools/call` result — a strict MCP
  client that reads `content[0].text` (e.g. thoth's `/call`, and the agentic tool loop) found none and
  rendered "no text content could be parsed". `bote_echo_handler` now wraps the echoed arguments in a
  text content block via the existing `content_text_response` helper —
  `{"content":[{"type":"text","text":"<args>"}]}`. The real `fs_*` tools were already conformant; only
  the echo sample was not. (`src/main_common.cyr`.)

### Changed
- **Toolchain pin `6.3.42 → 6.4.20`** (`cyrius.cyml` + `cyrius lib sync`, 56 floor modules). Clears the
  drift warning; all three binaries (bote / bote-streamable / bote-ws) build and the full test suite
  passes on the new toolchain.

## [3.0.0] — 2026-07-03 — MCP capability suite + honest polled-push notifications

The **3.0.0** milestone rounds out bote's MCP surface. bote went from three
methods (`initialize` / `tools/list` / `tools/call`) to a server advertising
four derived capabilities — **tools, prompts, resources, completion** — plus a
working **polled server→client notification path** (`notifications/tools/
list_changed` + `notifications/prompts/list_changed`), delivered on the client's
next streamable `GET` or piggybacked on a POST. Every capability is advertised
only when it can actually be honored: `listChanged` appears solely on the
streamable transport (which has a drain path), never on stdio/http/ws. All
growth is additive on the 2.0 handler ABI (`fn h(args, claims) → result`),
needs no new dependencies, and each internal struct grew by appended slots only.
Test surface: **733** assertions (was 653 at 2.9.0) + the core-only drift smoke.
The `[lib.core]` bundle grew 9 → 11 modules (`prompts.cyr`, `resources.cyr`).

### Breaking
- **`bote-streamable` now enforces MCP session lifecycle.** The shipped
  streamable binary configures a `SessionStore` (to back the per-session
  notification buffers), so `initialize` mints an `MCP-Session-Id` and every
  subsequent request — including the `GET` SSE stream — must present it; a
  session-less non-`initialize` request returns `404`. **Migration:** clients
  must perform the `initialize` handshake and echo the returned `MCP-Session-Id`
  header on all following requests (standard MCP 2025-11-25 session management).
  Consumers embedding the transport *library* are unaffected — the session store
  stays opt-in via `streamable_config_with_session_store`.

### Changed
- **Toolchain pin `6.3.38` → `6.3.42`** (`cyrius.cyml`; `cyrius lib sync --full`
  re-vendored the gitignored `lib/` and `cyrius deps` re-locked — only 3 stdlib
  hashes moved: `sigil` / `protobuf` / `sankoch`, the rest byte-identical). Full
  suite (733) green, all 13 benchmarks run clean (dispatch ~3–7 µs, jsonx
  sub-µs — no regression), and the SIGILL-sensitive crypto path (jwt/pkce/auth +
  the startup `chain_new` → sha256) was re-verified on the new toolchain via a
  runtime binary smoke.

### Fixed
- **`tests/bote.bcyr` compile** — the benchmark unit was missing the
  `prompts.cyr` / `resources.cyr` includes ever since those modules landed (it
  compiles `dispatch.cyr`, which now references them, so it failed with 13
  undefined functions). Surfaced by running the benchmark suite for the 3.0.0
  release; fixed by adding the includes in dependency order.

### Added
- **POST-piggyback SSE** — bite 6 (optional) of the server→client push path. When
  a `POST` response is built and the requesting session has pending notifications
  **and** the client's `Accept` allows `text/event-stream`, the transport answers
  that POST with an SSE stream (the JSON-RPC response as an id-less `message`
  frame, then the drained notifications) instead of `application/json` — so a
  client that never opens a GET stream still receives server notifications.
  Spec-allowed (MCP Streamable HTTP) and gated three ways (`Accept`, a resolved
  session, non-empty buffer), so it's a pure add-on: with no pending events the
  POST returns `application/json` exactly as before (regression-verified on the
  wire). The response frame is deliberately id-less (a synthetic id would
  pollute the client's `Last-Event-ID`; only the drained notifications carry
  resumable ids). New helpers `_strm_accepts_sse` / `_strm_post_session_outbound`
  / `_strm_response_frame`; +4 assertions in `tests/bote_streamable.tcyr`
  (49 → 53). Full-bundle-only (`dist/bote-core.cyr` unchanged).
- **`tools`/`prompts` `listChanged` — advertised, honestly** (bite 5, the payoff
  of the push path). `_build_capabilities` now emits `listChanged:true` on the
  `tools` and `prompts` capabilities **only when `dispatcher_notifications(d)`
  is set** — a new `+64` dispatcher flag (`dispatcher_set_notifications`) that a
  transport turns on **only if it actually has a client-drain path**.
  `main_streamable` sets it; `bote`/`bote-ws` do not. Result, verified on the
  wire: `bote-streamable`'s `initialize` advertises
  `{"tools":{"listChanged":true},"prompts":{"listChanged":true},…}` while the
  default `bote` advertises `{"tools":{},"prompts":{},…}` — same core
  `_build_capabilities`, different truth per binary, so no transport ever
  promises notifications it can't deliver. Also wires the **prompts** producer:
  a new `TOPIC_PROMPT_REGISTERED` event (`dispatcher_register_prompt` now emits
  it; `strm_notify_sink` maps it to `notifications/prompts/list_changed`). The
  stale `_server_emits_tool_list_changed` stub is gone, closing the
  `dispatch.cyr` capability-honesty note for tools + prompts. `resources`
  `listChanged`/`subscribe` and `logging` stay off (no producer). Dispatcher
  grew one slot (64 → 72 bytes). +1 assertion in `tests/bote.tcyr` (414 → 415),
  +3 in `tests/bote_streamable.tcyr` (46 → 49). **This completes the honest
  polled-push MVP** (delivery on the client's next GET); real-time held-open
  push remains the separate, threaded, phase-2 item.
- **`tools/list_changed` producer** — bite 4 of the server→client push path (the
  producer that makes the notification actually flow). A client-notification
  `EventSink` (`strm_notify_sink(store)` in `src/transport_streamable.cyr`) wraps
  the `SessionStore`; on a `TOPIC_TOOL_REGISTERED`/`_DEREGISTERED` event it
  **broadcasts** `notifications/tools/list_changed` into every active session's
  outbound buffer (each with that session's own event id), for delivery on the
  next SSE GET. Non-tool-set topics (completed/failed/deprecated — audit events)
  are ignored. No `dispatch.cyr` change and no second sink slot needed: register/
  dereg already publish to the dispatcher's `EventSink`, and `main_streamable`
  wasn't using that slot. `main_streamable.cyr` now wires a `SessionStore` +
  `dispatcher_set_events(strm_notify_sink(store))` and configures the store on
  the transport — which also **activates MCP session enforcement** (`initialize`
  mints an `MCP-Session-Id`; other requests, incl. the GET stream, must present
  it — verified on the wire: init returns a session id, a session GET streams,
  a session-less GET → 404). The `list_changed` path is proven by a full
  `dispatcher_register_tool` → sink → buffer test; it stays dormant in the
  reference binary only because nothing dynamically (de)registers there. The
  capability is **not advertised yet** — that is bite 5, gated on this working.
  +5 assertions in `tests/bote_streamable.tcyr` (41 → 46).
- **Unconditional GET drain** — bite 3 of the server→client push path.
  `_strm_handle_get` now resolves the client's per-session `SessionOutbound`
  buffer (from a valid `MCP-Session-Id` when a session store is configured; else
  the process-global buffer for back-compat) and drains it over the SSE stream:
  strictly after a non-empty `Last-Event-ID` cursor, else **everything currently
  buffered** — the "deliver on the client's next GET" model. The drain selection
  is a pure `_strm_drain_events(rbuf, last_id)` (unit-testable without a socket;
  empty cursor treated as absent → drain-all, not `events_after("")`). **The
  primer is now an SSE comment** (`: bote stream open`) instead of an
  id-carrying event: a synthetic primer id is in no buffer, so a client that
  adopted it as its `Last-Event-ID` would `events_after`-miss and silently resume
  nothing — the comment carries no `id:`, so per the SSE spec it never becomes
  the client's cursor. Ships inert in the shipped `bote-streamable` (no session
  store configured yet ⇒ storeless fallback, empty drain) and until a producer
  feeds a buffer. Verified on the wire (GET emits the comment primer + retry, no
  crash). +5 assertions in `tests/bote_streamable.tcyr` (36 → 41).
- **Per-session outbound notification buffer** — bite 2 of the server→client
  push path. `McpSession` gains an **opaque** `+32 outbound` slot (32 → 40
  bytes; `mcp_session_outbound` / `mcp_session_set_outbound`) that `session.cyr`
  never dereferences — the buffer types live in `transport_streamable.cyr`
  (streamable binary only) while `session.cyr` is in all three binaries, so the
  slot stays type-agnostic. The streamable transport adds a `SessionOutbound`
  bundle (a `ResumptionBuffer` + its **own** `EventIdGenerator`) and a lazy
  `_strm_session_outbound(session)` accessor that stashes it in the slot. The
  per-session id generator is the fix for a real resumption bug: event IDs must
  not straddle sessions, since `resumption_buffer_events_after` matches an exact
  id and returns empty on a miss — a shared global counter would let one
  session's `Last-Event-ID` land in another's gap and silently resume nothing.
  No free path needed (`session.cyr` never frees `McpSession`; the bundle drops
  with the session). Ships unused — the buffer stays empty until the producer
  bite. Full-bundle-only (neither module is in `[lib.core]`), so
  `dist/bote-core.cyr` is unchanged. +4 assertions in `tests/bote.tcyr`
  (410 → 414), +11 in `tests/bote_streamable.tcyr` (25 → 36) covering per-session
  independence + id continuity.
- **Notification wire builders** (`src/stream.cyr`) — the first bite of the
  server→client push path. A generic `notification_wire(method, params)` emits
  the JSON-RPC notification envelope `{"jsonrpc":"2.0","method":…,"params":…}`
  (method JSON-escaped; raw params, `0` ⇒ `{}`), with thin wrappers
  `tools_list_changed_notification` / `prompts_list_changed_notification` /
  `resources_list_changed_notification` / `resource_updated_notification(uri)`.
  `progress_notification` now delegates to it (output byte-identical — the two
  existing assertions are unchanged). These produce wire bytes only; delivery is
  the transport's job (the streamable `ResumptionBuffer`, wired in a later bite).
  Dep-free, additive, full-bundle-only (`stream.cyr` is not in `[lib.core]`), so
  `dist/bote-core.cyr` is unchanged. +6 assertions in `tests/bote.tcyr`
  (404 → 410). Groundwork for the honest polled-push MVP (`*/list_changed`);
  no capability is advertised yet — that waits until a drain path exists.

### Changed
- **Roadmap: dropped the stale "blocked on cyrius `lib/thread.cyr` MPSC"
  framing** for streaming dispatch. `lib/thread.cyr` (MPSC + mutex) and
  `lib/async.cyr` are complete and pinned — server→client notifications are
  unwired-**by-choice**, not gated on cyrius. A dep-free polled-push MVP (buffer
  at produce time, drain on the client's next streamable `GET`) needs no threads;
  only real-time held-open streaming does (the single-threaded sandhi accept loop
  would otherwise deadlock).

  single completion handler on the dispatcher (`+56 completion_handler`, 56 → 64
  bytes) set via `dispatcher_set_completion`; the `completions` capability is
  **derived** from its presence (`{"tools":{},…,"completions":{}}`). Handler ABI
  `fn h(params_cstr, claims) → result_cstr` — it receives the raw
  `{ref, argument, …}` params and returns the full `{"completion":{"values":[…]}}`
  body, so bote does no ref/argument interpretation (consumer completes against
  its own prompts/resources). Unknown/unset → `-32601`. No new module (unlike
  prompts/resources) — dispatcher slot only. A reference completion handler is
  wired in the binary family, proven on the wire. +6 assertions in
  `tests/bote.tcyr` (398 → 404), including the four-capability canonical-order
  check (`{"tools":{},"prompts":{},"resources":{},"completions":{}}`). Sixth bite
  of the secureyeoman → bote MCP bring-over.

  _Logging (`logging/setLevel` + `notifications/message`) is intentionally NOT
  in this bite: it exists only to gate server→client log push, which is not
  wired — advertising a `logging` capability would promise messages bote can't
  send. It lands with the notification/push work._
- **MCP resources capability** — `resources/list` + `resources/read` (new module
  `src/resources.cyr`). Same shape as the prompts capability: a
  `ResourceRegistry` (metadata for `resources/list` + a uri→read-handler map),
  a lazily-created dispatcher slot (`+48 resource_registry`, 48 → 56 bytes) via
  `dispatcher_register_resource`, and the `resources` capability **derived** from
  registry presence in `_build_capabilities` (`{"tools":{},"resources":{}}`; with
  prompts too → `{"tools":{},"prompts":{},"resources":{}}`). `resources/list`
  serializes each resource's `uri`/`name` (+ optional `description`/`mimeType`);
  read handlers use the ABI `fn h(uri_cstr, claims) → result_cstr` and return the
  full `{"contents":[…]}` body (bare MCP `ResourceContents`, not a content
  block). `subscribe`/`listChanged` are intentionally omitted — no server→client
  push path is wired. Unknown/absent capability → `-32601`. A reference
  `bote://info` resource is registered in the binary family, proven on the wire
  (`resources/list` → `resources/read`). `src/resources.cyr` is in both `[lib]`
  and `[lib.core]` (**core bundle 10 → 11 modules**). +11 assertions in
  `tests/bote.tcyr` (387 → 398). Fifth bite of the secureyeoman → bote MCP
  bring-over.
- **MCP prompts capability** — `prompts/list` + `prompts/get` (new module
  `src/prompts.cyr`). A `PromptRegistry` mirrors the tool registry (metadata for
  `prompts/list` + a name→generator-handler map for `prompts/get`), and the
  dispatcher gained one slot (`+40 prompt_registry`, 40 → 48 bytes) plus
  `dispatcher_register_prompt`, which **lazily** stands up the registry — so
  registering the first prompt is also what flips the `prompts` capability on at
  initialize. This is the first payoff of the 0b `_build_capabilities` seam: the
  `prompts` key is **derived** from actual registry presence (`{"tools":{}}`
  with no prompts; `{"tools":{},"prompts":{}}` once one is registered), never
  advertised blind. Prompt generator handlers use the 2.0 ABI
  (`fn h(arguments_cstr, claims) → result_cstr`) and return the full
  `{description?, messages:[…]}` body. `prompts/list` serializes each prompt's
  `name` / optional `description` / optional `arguments` (each arg's `required`
  emitted only when true). When no prompt registry exists, `prompts/*` fall
  through to `-32601` (matching the un-advertised capability). A reference
  `bote_greeting` prompt is registered in the binary family alongside
  `bote_echo`, proven on the wire (`initialize` → `prompts/list` → `prompts/get
  name=Ada` ⇒ a `content_text` "Hello, Ada!" message). `src/prompts.cyr` is in
  both `[lib]` and `[lib.core]` (**core bundle 9 → 10 modules**). +10 assertions
  in `tests/bote.tcyr` (377 → 387). Fourth bite of the secureyeoman → bote MCP
  bring-over.
- **Tool annotations now serialized in `tools/list`** (MCP 2025-11-25
  `ToolAnnotations`). The `ToolAnnotations` model + `tool_def_with_annotations`
  have existed on the registry since the annotations work landed, but the
  `tools/list` builder never emitted them — the hints were stored and never seen
  by clients. `_build_tools_list_result` (`src/dispatch.cyr`) now appends an
  `"annotations":{...}` object per tool via the new `_emit_annotations` /
  `_emit_hint_field` helpers, honoring the tri-state contract documented at
  `src/registry.cyr:16-17`: `readOnlyHint` / `destructiveHint` / `idempotentHint`
  / `openWorldHint` are each emitted as `true`/`false` only when set, and the
  whole object is omitted when the annotations ptr is null **or** every hint is
  unset. Purely additive — no annotations → byte-identical output to before, so
  the handler ABI and existing clients are unaffected. +4 assertions in
  `tests/bote.tcyr` (363 → 367) covering the read-only preset (all four hints),
  a single-hint tool, and both absence cases. First bite of the secureyeoman →
  bote MCP capability bring-over (annotations were modeled but unwired).

### Added
- **Tool profiles + `tools/list` profile filter** (`src/registry.cyr`,
  `src/dispatch.cyr`). Tools can carry opaque consumer-defined profile tags
  (e.g. `security` / `web` / `full`) via `tool_def_with_profiles(d, vec)`; a
  client may pass `{"profile":"<tag>"}` in `tools/list` params to receive only
  the tools tagged with that profile (exact-match membership via the new
  `tool_def_has_profile`). This is bote's `[lib.profile]` idea applied at
  runtime — the reusable primitive under secureyeoman's AGNOS-bridge profiles /
  smart-schema-delivery, with **no interpretation in bote** (an untagged tool is
  in no profile; `full` etc. are pure consumer convention). Fully **additive and
  opt-in**: `tools/list` with no `profile` param returns the complete list,
  byte-identical to before. `ToolDef` grew one appended slot (56 → 64 bytes,
  `+56 profiles`) — same additive pattern as `annotations`/`compiled`, so all
  existing field offsets and the builder/accessor API are unchanged. The
  `tools/list` builder now tracks an emitted-count for comma placement so
  filtered gaps can't corrupt the JSON array. +8 assertions in `tests/bote.tcyr`
  (369 → 377). Third bite of the secureyeoman → bote MCP bring-over.

### Changed
- **`initialize` capabilities extracted into a `_build_capabilities` seam**
  (`src/dispatch.cyr`). The `capabilities` object was a hardcoded string literal
  inside `_build_initialize_result`; it's now built by a dedicated helper whose
  advertised keys are **derived from what bote actually implements**, so future
  `prompts` / `resources` / `logging` / `completions` capabilities drop in gated
  on their own handler surfaces without touching the initialize body. Output is
  **byte-identical** (`"capabilities":{"tools":{}}`) — purely a refactor + a
  recorded decision. The **`tools.listChanged` decision is now explicit and
  pinned by a test**: it is deliberately **not** advertised because bote has no
  server→client push path (the `TOPIC_TOOL_REGISTERED/_DEREGISTERED` EventSink
  topics are bote's internal bus, not MCP `notifications/tools/list_changed`), so
  advertising `listChanged:true` would leave clients waiting for notifications
  that never arrive. A single predicate `_server_emits_tool_list_changed` flips
  it when a persistent push channel lands. +2 assertions in `tests/bote.tcyr`
  (367 → 369). Second bite of the secureyeoman → bote MCP bring-over (the
  capabilities enabler for prompts/resources/completion).

## [2.9.0] — 2026-07-03 — runs + serves MCP on agnos (cyrius 6.3.38)

### Changed
- **Toolchain migrated `6.3.15` → `6.3.38`** (`cyrius.cyml` pin; `cyrius lib sync
  --full` re-vendored the gitignored `lib/` from the current stdlib snapshot).
  Host suite green (25/22/… all files, 0 failed).

### Fixed
- **bote now runs on agnos** (was: SIGSEGV at startup on the `--agnos` build).
  Root cause was **not** bote or sigil — it was a **stale vendored stdlib
  `freelist.cyr`** carried by the old 6.3.15 pin. Pre-fix `freelist.cyr` used the
  Linux 6-arg `syscall(SYS_MMAP, 0, size, …)` on every target; on agnos `mmap#27`
  takes the length in arg1, so the leading Linux addr-hint `0` was read as the
  size → `mmap#27(0)` → `MAP_FAILED` (0) → the next `store64` SIGSEGV'd. That
  killed **every `fl_alloc` consumer** — all of sigil's crypto (sha256/hmac/
  ed25519/aes-gcm) — so bote died in `main()` at `chain_new()` → `sha256_init()`
  → `fl_alloc(144)`. cyrius already fixed `freelist.cyr` (a `_fl_mmap()` that
  dispatches `#ifdef CYRIUS_TARGET_AGNOS → syscall(SYS_MMAP, length)`); the
  6.3.38 migration picks it up. **Proven on agnos under mirshi** (AGNOS→Linux
  syscall translation): `bote-agnos` serves the full MCP flow — `initialize`
  (serverInfo bote 2.8.0), `tools/list` (9 tools), and `tools/call bote_echo`
  (executes, exercising the crypto path). **Also proven on the REAL agnos kernel
  under QEMU** (real `mmap#27`, not mirshi's host-kernel emulation): the new
  `BOTE_SELFTEST` kernel hook + `scripts/bote-mcp-smoke.sh` (in the agnos repo)
  pipe an MCP `initialize` + `tools/call bote_echo` into bote's fd0 via two kernel
  pipes and capture fd1 — serial shows bote's `serverInfo` reply and the echoed
  `agnos-kernel` argument, `exit 0`, no fault/panic.

## [2.8.0] — 2026-07-02 — filesystem tools (`fs_write` / `fs_read` / `fs_mkdir`)

A new `src/fs_tools.cyr` module adds three filesystem tools so an MCP client
(e.g. thoth, via daimon) can create a small file-based project end to end.
Registered on the `bote` binary alongside `bote_echo` and the five `libro_*`
audit tools and — like `libro_tools` — folded into the full `dist/bote.cyr`
bundle (`[lib] modules`), so downstream consumers can opt in by calling
`fs_tools_register()`. NOT in the transport-free `dist/bote-core.cyr`, and no
existing tool or ABI moved (minor: additive surface only).

### Added

- **`fs_write` / `fs_read` / `fs_mkdir`** (`src/fs_tools.cyr`) — MCP tools
  for writing, reading, and creating directories. `fs_write` creates missing
  parent directories and returns the bytes written; all three return proper
  MCP content blocks (`{"content":[{"type":"text",…}],"isError":…}`) so a
  client renders the result directly. `fs_write` JSON-unescapes its `content`
  argument (`\n`/`\t`/`\"`/`\\`/`\uXXXX`→UTF-8) so multi-line source files
  land byte-correct on disk.
- **`src/fs_tools.cyr` added to `[lib] modules`** — the tool module ships in
  the full `dist/bote.cyr` bundle (25 modules), mirroring `libro_tools`.
- **`tests/bote_fs_tools.tcyr`** — 26 assertions covering the path-safety
  guard, the JSON unescaper (incl. `\u` → UTF-8), tool registration, and the
  handler's refusal path.

### Security

- Filesystem access is **confined to a root** (`BOTE_FS_ROOT`, default `.`):
  an argument path that is absolute or contains a `..` segment is **refused**
  (`isError:true`, no I/O) — defense-in-depth beneath t-ron's per-tool
  authorization. The root confinement is a floor, not a full sandbox;
  registration is **opt-in** (`fs_tools_register`), so embedding the bundle
  does not expose filesystem writes until a consumer wires the tools in.

## [2.7.8] — 2026-07-01 — AF_UNIX transport fail-closes on agnos

AGNOS cross-build readiness. bote-core was already `--agnos`-clean; this
closes the full-server transport surface so the `bote` binary itself
compiles under `cyrius build --agnos`.

### Fixed

- **`--agnos` build**: `transport_unix_run` (`src/transport_unix.cyr`) is
  guarded with `#ifdef CYRIUS_TARGET_AGNOS` and fail-closes on agnos, which
  has no AF_UNIX domain sockets. Guarding the entry drops the whole AF_UNIX
  subtree (`SYS_SOCKET` / `BIND` / `CHMOD` / `LISTEN` / `ACCEPT` — unrelated
  syscall numbers on agnos) off the agnos target; bote's TCP / streamable / ws
  transports carry traffic there. Mirrors majra's ipc AF_UNIX guard.

## [2.7.7] — 2026-06-30 — cyrius 6.3.15 base-stack migration + 6.3.x stdlib rename reconciliation

Tier-3 step of the coordinated base-security-stack migration to cyrius
**6.3.15** (sakshi 2.4.3 → sigil 3.9.8 → majra 2.5.0 → libro 2.7.9 →
**bote 2.7.7** → the five consumers). Toolchain pin + dependency refresh
plus the stdlib boundary/rename reconciliation the 6.3.x line requires.
No bote runtime *logic* changed. All 653 assertions pass across all 11
test files on the new stack; `dist/bote.cyr` / `dist/bote-core.cyr`
regenerated at v2.7.7.

### Changed

- **Toolchain**: pinned to cyrius **6.3.15** (was 6.2.11).
- **Dependencies**: libro **2.7.9** (was 2.7.4), majra **2.5.0** (was
  2.4.7), sigil **3.9.8** (was 3.7.14) — the migrated tiers below bote.
- **`[deps] stdlib`**: added `atomic` + `sync` (patra's transitive
  `lib/sync.cyr` requirement surfaces through the libro chain on 6.3.x)
  and `dynlib` (6.3.x's `fdlopen.cyr` references `dynlib_auxv_get` /
  `dynlib_read_auxv`; `dynlib` must precede `fdlopen` — single-pass).
- **`http_send_204` → `sandhi_server_send_204`** (transport_http /
  bridge / transport_streamable): the retired bote-local `http_*` shim
  alias was still referenced at three 204-response sites; renamed to the
  canonical sandhi export (identical `(cfd, extra_headers)` signature).
- **`_bote_server_version()`**: corrected stale literal `2.7.1` → `2.7.7`
  (the MCP `initialize` handshake now reports the true release again).

### Fixed

- **`auth_bearer_check` undefined** in `tests/bote.tcyr`,
  `bote_streamable.tcyr`, `bote_ws.tcyr`: these files include the
  transports that call it but omitted `src/auth.cyr`, which compiled at
  6.2.x only because DCE treated the call site as unreachable. 6.3.x's
  reachability marks it live; added the `src/auth.cyr` include before the
  transport include in each (mirrors `src/main.cyr`).
- **`http_find_header` undefined** (WS handshake, via stdlib
  `lib/ws_server.cyr`): the 6.3.x stdlib renamed its `http_*` header
  helpers to `sandhi_server_*` but missed `ws_server.cyr`, leaving the
  symbol dangling. Added a bote-local compat shim in `src/transport_ws.cyr`
  forwarding `http_find_header(buf, blen, name)` →
  `sandhi_server_find_header(buf, blen, name)` (identical contract).
  Filed upstream against cyrius; remove the shim once `ws_server.cyr` is
  fixed.

## [2.7.6] — 2026-06-15 — cyrius 6.2.11 (first move onto the 6.2.x line) + dependency refresh

Toolchain + dependency refresh, ecosystem-wide 6.2.x sweep. No bote
source *logic* changed — pins, a required `thread_local` include guard,
and the 6.2.11 formatter reflow. All 653 assertions (+ 1 drift smoke)
pass on the new stack; all 14 benchmarks run clean; `dist/bote.cyr` /
`dist/bote-core.cyr` regenerated at v2.7.6.

### Changed

- **Cyrius pin `6.1.41` → `6.2.11`** (`cyrius.cyml [package].cyrius`).
  First step onto the 6.2.x maintenance line. The installed toolchain
  was already 6.2.11; this aligns the manifest pin and clears the drift
  warning.
- **Dependencies bumped to current tags:**
  - **libro `2.7.2` → `2.7.4`** (`[deps.libro]`) — toolchain refresh;
    `dist/libro.cyr` body byte-identical apart from the version header.
  - **majra `2.4.5` → `2.4.7`** (`[deps.majra]`) — toolchain refresh;
    bundle bodies byte-identical to 2.4.5.
  - **sigil `3.7.12` → `3.7.14`** (`[deps.sigil]`) — self-contained
    `dist/sigil.cyr` retained; transitive agnosys `1.3.2` → `1.4.3`.
- **`dist/bote.cyr` / `dist/bote-core.cyr` regenerated at v2.7.6.** The
  only body change is the 6.2.11 formatter's continuation-line reflow
  (whitespace) — no module moved, no symbol changed.
- **fn_table / identifier utilisation on `src/main.cyr`: 58% / 60%**
  (`fn_table 4770/8192`, `identifiers 157633/262144`) — the small bump
  over 2.7.5's `4764/157278` is the `thread_local` pull-through. Well
  under the 95% CI gate.

### Fixed

- **sigil 3.7.14 TLS-path SIGILL guard (latent CI landmine).** sigil
  3.7.14's `crypto_scratch` exercises the thread-local-storage path that
  3.7.12 never hit. Without `lib/thread_local.cyr` ahead of
  `lib/sigil.cyr`, the binaries and crypto tests link clean but **SIGILL
  at first crypto use (exit 132, `Illegal instruction`)** — a build-only
  check misses it; the harness must be *run*. Added `thread_local` to
  `[deps] stdlib` (immediately after `thread`, before `sigil`) so the
  three binaries pick it up via auto-injection, and an explicit
  `include "lib/thread_local.cyr"` after `lib/thread.cyr` in every
  sigil-using test file (`bote.tcyr`, `bote_jwt.tcyr`, `bote_pkce.tcyr`,
  `bote_libro_tools.tcyr`, `bote_streamable.tcyr`, `bote_ws.tcyr`).
  `bote_jwt` / `bote_pkce` also gained the `thread` include they
  previously did without. All 653 assertions pass exit-0; jwt (28) and
  pkce (17) confirm the crypto path runs clean. This is the exact
  failure mode libro 2.7.4's CHANGELOG documents.

### Notes

- **New benign 6.2.11 linker diagnostics.** The 6.2.11 linker now warns
  on duplicate global symbols — `duplicate symbol 'ERR_IO' / 'ERR_UNKNOWN'`
  (`lib/libro.cyr` / `lib/agnosys.cyr`) and `duplicate fn '_sub_new'`
  (`lib/majra.cyr`). Pre-existing name collisions between the bundled
  deps' error enums; harmless (last-definition-wins, all tests pass),
  simply silent before 6.2.11 began diagnosing them. Not a bote
  regression.
- **`cyrius fmt` CLI changed in 6.2.x.** The flag now follows the file:
  `cyrius fmt <file> --check` (was `cyrius fmt --check <file>` on 6.1.x).
  The 6.2.11 formatter also reflows deep continuation-line indentation to
  a fixed indent — hence the whitespace-only churn across `src/` and
  `tests/` in this release.

## [2.7.5] — 2026-06-11 — libro_tools folded back into the default binary + full bundle

### Added

- **libro audit tools in the default binary + full bundle.** The five
  `libro_*` MCP tools (`libro_query` / `libro_verify` / `libro_export` /
  `libro_proof` / `libro_retention`) are now registered on `build/bote`
  by default — `main()` stands up an in-memory libro chain via
  `chain_new()` and calls `libro_tools_init` + `libro_tools_register`.
  `src/libro_tools.cyr` also joins the `[lib]` profile, so it ships in
  `dist/bote.cyr` (now 24 modules). It is deliberately **not** in
  `dist/bote-core.cyr`: like `audit_libro` it depends on a live libro
  chain, which the transport-free core profile excludes.

### Changed

- Reverted the 1.9.4 cap-headroom decision that held `libro_tools` out
  of the default build. The 6.1.x function-table cap raise dropped
  `src/main.cyr` util to 58% / 60% (`fn_table 4764/8192`,
  `identifiers 157278/262144`), well under the 95% CI gate, so the
  headroom argument no longer applies. All 653 assertions (+ 1 drift
  smoke) pass; `tests/bote_libro_tools.tcyr` (22) exercises the
  re-included surface.

## [2.7.4] — 2026-06-11 — cyrius 6.1.41; tool-registry constructor renamed to resolve the ai-hwaccel `registry_new` collision

Toolchain patch refresh + a targeted breaking rename to unblock
multi-library consumers (szal, mihi, hoosh) that include both a bote
bundle and `dist/ai-hwaccel.cyr` in one compile unit. All 653
assertions (+ 1 drift smoke) that exercise the registry surface pass
on the renamed constructor.

### Breaking

- **`registry_new()` → `tool_registry_new()`.** bote's `ToolRegistry`
  constructor collided with ai-hwaccel's 32-byte profile-registry
  constructor of the same name (both export `fn registry_new()` with
  incompatible struct layouts — 24 vs 32 bytes). Cyrius include
  semantics are textual paste + last-definition-wins, so any consumer
  including both bundles silently got one `registry_new` and corrupted
  memory on the other's call sites. There is no include order that
  fixes it because ai-hwaccel's detection path calls its own
  `registry_new` internally. bote takes the rename to a lib-descriptive
  name — `tool_registry_new` parallels the existing `host_registry_new`
  (HostRegistry) constructor. Only the constructor changed; every other
  `registry_*` accessor keeps its name (none of them collide).

  **Migration:** replace `registry_new()` with `tool_registry_new()`.
  The struct layout, all accessors (`registry_register`, `registry_get`,
  `registry_list`, …), and the dispatcher wire-up are unchanged.

  See `docs/development/issues/archive/2026-06-11-registry-new-collision.md`.

### Changed

- **Cyrius toolchain pin: 6.1.24 → 6.1.41.** Patch-series bump within
  the 6.1.x line. No MCP wire-format, handler-ABI, or compile-cap
  change.

- **Stdlib dep migration for the 6.1.x consolidation
  (`[deps] stdlib`).** The 6.1.x toolchain folded the standalone
  `json` and `base64` stdlib modules into a single consolidated
  **`bayan`** module (JSON via `json_parse` / `json_get` back-compat
  shims, plus `base64` / `base64url`, csv, u128) and retired the
  standalone `bigint` module (sigil 3.x now bundles its own `u256` /
  `u384` inline; bote has no direct big-integer use). bote's
  `[deps] stdlib` and the test-file include headers moved
  `json` + `base64` → `bayan`, dropped `bigint`, and `src/registry.cyr`
  now `include`s `lib/bayan.cyr`. **This was the binding correctness
  fix:** the previous pin bump left a stale `lib/` (pre-6.1 `json.cyr` /
  `base64.cyr` / `bigint.cyr` snapshots) shadowing the version-matched
  toolchain libs — `cyrius deps` against 6.1.41 errors out on the
  removed module names until the manifest is migrated. (`ganita` in
  6.1.x is an unrelated linear-algebra module, **not** the bigint
  successor.)

- **Function-table + identifier-buffer utilisation: 52% / 52% →
  **58% / 60%** (`CYRIUS_STATS=1`: `fn_table 4740/8192`,
  `identifiers 156646/262144`) on `src/main.cyr`.** The rise is the
  `bayan` consolidation pulling in more surface than the old split
  `json` + `base64` modules; still well under the 95% CI gate.

- **sigil pinned explicitly via `[deps.sigil]` at tag 3.7.12** (git +
  `../sigil` path, single-file `dist/sigil.cyr` — same pattern as
  libro / majra), and `.cyrius-toolchain` marker corrected `4.8.4` →
  `6.1.41`.

### Fixed

- **CI build broke with `cannot open include file: src/sha_ni.cyr`.**
  The 6.1.x stdlib registry resolves the bare `sigil` stdlib dep to
  **3.7.10**, whose `dist/sigil.cyr` is *not* self-contained — it
  carries guarded `include "src/sha_ni.cyr"` / `include "src/aes_ni.cyr"`
  directives that dangle for single-file consumers (the toolchain
  *snapshot* ships the fixed 3.7.12, but `cyrius deps` pulls the
  registry version and ignores the lock for version selection).
  Pinning sigil to **3.7.12** via `[deps.sigil]` (which overrides the
  stdlib registry resolution) restores a self-contained bundle.
  Verified: `cyrius build src/main.cyr build/bote` → ELF, all 653
  assertions + drift smoke pass from a clean `cyrius deps`.

- **CI Build step masked compile failures.**
  `cyrius build … | tee build/build.log` returned `tee`'s exit 0 even
  when the compile failed, so a broken build passed the Build step and
  only surfaced two steps later as a confusing `build/bote: No such
  file`. Added `set -o pipefail` so the compile's status fails the step
  at its source.

- **`dist/bote.cyr` + `dist/bote-core.cyr` regenerated** for the
  constructor rename (`cyrius distlib` / `cyrius distlib core`).

## [2.7.3] — 2026-06-10 — cyrius 6.1.24 + libro 2.7.2 + majra 2.4.5; major-toolchain jump relieves the 5.10.x compile cap

Toolchain + dep refresh. The headline is the **cyrius major-version
jump (5.10.44 → 6.1.24)** — the 5.11.x migration that the 2.7.2
notes flagged as "planned" landed as 6.1.x instead. No MCP
wire-format change, no handler-ABI change, no `src/*.cyr` change.
All 653 assertions (+ 1 drift smoke) pass unchanged on the new
toolchain; the 14 criterion benchmarks show no regression
(`benches/history.log`, v2.7.3).

### Changed

- **Cyrius toolchain pin: 5.10.44 → 6.1.24.** Major-version jump.
  The expanded-source compile cap that forced the 2.7.2
  per-transport binary split is no longer the binding constraint —
  `src/main.cyr` builds clean. Function-table + identifier-buffer
  utilisation dropped from 93% / 92% (5.10.x) to **52% / 52%**
  (`CYRIUS_STATS=1`: `fn_table 4250/8192`, `identifiers
  137635/262144`) on the raised 6.1.x caps. The per-transport
  binary split (`bote` / `bote-streamable` / `bote-ws`) is retained
  for this release — reconsolidation to a single binary is now
  unblocked and tracked for a follow-up.

- **First-party dep pins, all bumped to latest released:**
  - **libro 2.6.3 → 2.7.2** — `dist/libro.cyr` refresh; audit-chain
    + `sha256_hex` surface unchanged at bote's `audit_libro` /
    `libro_tools` call sites.
  - **majra 2.4.4 → 2.4.5** — minor refresh; pubsub / counter
    surface unchanged at bote's `events_majra` adapter call sites.

- **`cyrius.lock` now carries full stdlib hashes (6 → 40 entries).**
  6.1.24's `cyrius deps` locks every resolved `lib/*.cyr`, not just
  the first-party dist bundles. CI's byte-clean lock gate covers the
  whole resolved set now.

- **CI / release toolchain installer rewritten for 6.x.** The
  hand-rolled two-tarball extraction in `.github/workflows/{ci,release}.yml`
  probed for `bin/cc5` — renamed to `cycc` in the 6.x toolchain, so the
  install gate failed outright. Both workflows now delegate to the
  upstream `scripts/install.sh` keyed on the `cyrius.cyml` pin (the
  pattern patra / agnosys already run on 6.x), which lays out
  `$HOME/.cyrius/{bin,lib}` including the stdlib snapshot. Toolchain
  verify step `cc5 --version` → `cyrius --version`.

### Removed

- **`cyrius audit` dropped from the cleanliness sequence.** Under
  6.1.x `cyrius audit` is the *toolchain* self-host gate
  (self-host + test + fmt + lint over the compiler), not a project
  dependency audit, and it fails outside the cyrius repo. The
  project-policy + dependency checks now run as `cyrius deny
  src/main.cyr` (policy) + the new `cyrius vet src/main.cyr`
  (include-dependency audit); `cyrius fmt --check` + `cyrius lint`
  are unchanged.

## [2.7.2] — 2026-05-11 — cyrius 5.10.44 + libro 2.6.3 + majra 2.4.4; per-transport binary split for 5.10.x cap

Toolchain + dep refresh, plus a structural workaround for the
cyrius 5.10.x 2 MB compile-source cap that the upgraded deps push
bote past. No MCP wire-format change, no handler-ABI change. The
default `bote` binary's CLI surface shrinks (streamable / ws moved
to siblings); the API surface in `src/*.cyr` is unchanged.

### Changed

- **Cyrius toolchain pin: 5.10.34 → 5.10.44.** Picks up the
  5.10.x stdlib + frontend deltas. Notable for bote: pulls
  `lib/slice.cyr` (now required transitively by libro 2.6.3 →
  agnosys for slice subscripts) and `lib/assert.cyr` (used by
  libro / majra).

- **First-party dep pins, all bumped to latest released:**
  - **libro 2.6.2 → 2.6.3** — `dist/libro.cyr` fixes the bare
    `ct_eq` call at the old call site (links against
    `ct_eq_bytes_lens` now). Closes the libro-side blocker
    phylax 1.1.1 documented from the consumer side.
  - **majra 2.4.3 → 2.4.4** — minor refresh; pubsub / counter
    surface unchanged at bote's `events_majra` adapter call sites.

- **Stdlib additions: `slice`, `assert`, `ct`, `keccak`, `random`.**
  Required by libro 2.6.3 + sigil 3.1.1 (PQ / AES-GCM surfaces).
  bote itself only consumes `sha256_hex`; the linker needs the
  symbols declared even if DCE prunes most call sites. See phylax
  1.1.1 CHANGELOG for the original pull-through write-up.

- **`ws_server` removed from `[deps] stdlib` auto-inject.** It
  lives in `src/transport_ws.cyr` + `tests/bote_ws.tcyr` and gets
  pulled in via manual `include "lib/ws_server.cyr"` from those
  files only. Keeps every other test / binary from carrying the
  11 KB ws-frame machinery they don't use.

### Added

- **`dist/bote-core.cyr` — opt-in core bundle.** Closes the
  consumer-side blocker tracked at
  `docs/development/issues/2026-05-10-opt-in-transport-profile.md`.
  Nine transport-free modules (`error`, `protocol`, `jsonx`,
  `registry`, `events`, `audit`, `dispatch`, `codec`, `schema`)
  packaged via `cyrius distlib core` (new `[lib.core]` profile
  in `cyrius.cyml`). 1989 lines / 70 KB — matches the issue's
  projected shape. Consumers wrap bote's dispatch surface but
  supply their own transport stack; the bundle excludes
  `transport_*` / `bridge` / `auth` / `session` / `discovery` /
  `content` / `host` / `audit_libro` / `events_majra`. **t-ron
  2.1.x is the trigger consumer** — its `cyrius.cyml` will flip
  to `modules = ["dist/bote-core.cyr"]` on the 2.7.2 bump,
  retiring the per-module pull workaround it shipped at 2.1.0.
  Drift guard: `tests/bote_core_only_smoke.tcyr` includes the
  bundle in isolation and runs a dispatcher + registry round-trip.
  See `DEPS-PATTERN.md` for the profile-selection contract.

- **`DEPS-PATTERN.md`** — distribution contract doc, modelled on
  libro's. Documents the dual-bundle shape (`dist/bote.cyr` +
  `dist/bote-core.cyr`), the `cyrius distlib` invocations, the
  profile-selection rule for downstream consumers, and the
  core-9 module list.

- **Per-transport binary split** (cyrius 5.10.x cap workaround):
  - `build/bote` — default. stdio + http + unix + bridge.
  - `build/bote-streamable` (new) — Streamable HTTP / SSE on
    port 8392 default. Built from `src/main_streamable.cyr`.
  - `build/bote-ws` (new) — WebSocket MCP on port 8393 default.
    Built from `src/main_ws.cyr`.
  Shared helpers (echo handler, env-driven bearer wiring, CSV
  token split, dispatcher constructor) live in
  `src/main_common.cyr`. `scripts/build-all.sh` builds the trio.
  Folds back into a single `build/bote` binary when bote migrates
  to cyrius 5.11.x (cap raised to 4 MB; see companion proposal in
  `cyrius/docs/development/proposals/2026-05-10-raise-compile-source-cap.md`).

- **Per-module test split** (Streamable HTTP / WS): the
  `tests/bote.tcyr` catch-all was hitting the 2 MB cap once the
  cyrius / libro / majra refresh landed. Streamable + WebSocket
  test sections extracted into:
  - `tests/bote_streamable.tcyr` — 25 assertions (EventIdGenerator,
    StreamEvent wire format, ResumptionBuffer eviction, events_after
    edge cases, StreamableConfig retry_ms, sandhi path-strip sanity).
  - `tests/bote_ws.tcyr` — 10 assertions (WsConfig path / addr /
    port / allowed_origins / require_protocol / dispatcher wire-up,
    `_bote_ws_handler` fp addressable).
  `tests/bote.tcyr` keeps 363 assertions; full test count across
  all 10 `tests/*.tcyr` files is **653**. Pattern mirrors phylax
  1.1.1's per-module test split.

### Fixed

- **`scripts/bench-log.sh` ported from `cargo bench` to `cyrius bench`.**
  The script was a Rust-era stale and had never run successfully
  against the Cyrius port. Output captured from `cyrius bench
  tests/bote.bcyr` (14 criterion benches: dispatch_initialize /
  dispatch_tools_list / dispatch_tools_call / jsonx_get_str_flat
  / jsonx_get_raw_nested / codec_parse_request /
  codec_serialize_response / codec_process_message /
  validate_compiled_simple / validate_compiled_nested /
  schema_compile_simple / schema_compile_nested /
  auth_bearer_check_unset / auth_bearer_check_set).

### Performance

Benchmark snapshot at v2.7.2 (full block in
`benches/history.log`). Per-iteration averages:

| Bench | avg | iters |
|-------|-----|-------|
| dispatch_initialize       | 2 µs  | 10 000  |
| dispatch_tools_list       | 3 µs  | 10 000  |
| dispatch_tools_call       | 6 µs  | 10 000  |
| jsonx_get_str_flat        | 1 µs  | 100 000 |
| jsonx_get_raw_nested      | 1 µs  | 100 000 |
| codec_parse_request       | 2 µs  | 10 000  |
| codec_serialize_response  | 1 µs  | 10 000  |
| codec_process_message     | 8 µs  | 10 000  |
| validate_compiled_simple  | 1 µs  | 10 000  |
| validate_compiled_nested  | 3 µs  | 10 000  |
| schema_compile_simple     | 4 µs  | 10 000  |
| schema_compile_nested     | 7 µs  | 10 000  |
| auth_bearer_check_unset   | 1 µs  | 100 000 |
| auth_bearer_check_set     | 2 µs  | 100 000 |

No regressions vs. 2.7.1 (within measurement noise — all benches
were already at the sub-10-µs floor).

### Breaking

- **`bote streamable [port]`** and **`bote ws [port]`** CLI modes
  removed from the default `bote` binary. Callers must invoke
  `bote-streamable [port]` / `bote-ws [port]` instead. The
  invocation matrix:

  | Transport      | 2.7.1                    | 2.7.2                          |
  |----------------|--------------------------|--------------------------------|
  | stdio (default)| `bote`                   | `bote` (unchanged)             |
  | HTTP           | `bote http [port]`       | `bote http [port]` (unchanged) |
  | Unix socket    | `bote unix <path>`       | `bote unix <path>` (unchanged) |
  | TS bridge      | `bote bridge [port]`     | `bote bridge [port]` (unchanged) |
  | Streamable     | `bote streamable [port]` | `bote-streamable [port]`       |
  | WebSocket      | `bote ws [port]`         | `bote-ws [port]`               |

  This is purely a CLI surface change — the underlying transport
  modules (`src/transport_streamable.cyr` / `src/transport_ws.cyr`)
  and their public API are unchanged. Consolidates back into a
  single binary on cyrius 5.11.x migration.

## [2.7.1] — 2026-05-10 — HostRegistry hot-reload + CONTRIBUTING.md Cyrius-era cleanup

Second 2.7.x patch. Lands the **HostRegistry hot-reload** feature
the 2.6.x deferred slate promised, plus a long-overdue
`CONTRIBUTING.md` rewrite that drops the Rust-era references the
Cyrius port left behind.

No MCP wire-format change, no handler-ABI change. New API surface
on `HostRegistry`; existing in-memory wire-up unchanged.

### Added

- **HostRegistry JSON config + hot-reload** (`src/host.cyr`).
  Five new public functions:
  - `host_entry_from_json(obj_cstr)` — parse a single host-entry
    object. Required fields: `name`, `url`. Optional:
    `capabilities` (array of cstrs). Returns 0 on missing
    required fields or malformed `capabilities`.
  - `host_registry_load_json(json_cstr)` — parse a JSON array
    of entry objects into a fresh registry. Empty array is legal
    (returns empty registry, not an error).
  - `host_registry_load_from_file(path)` — read `path` into a
    64 KB buffer (`HOST_CONFIG_BUF_MAX`) and parse. Files larger
    than the buffer truncate at read time and fail-closed at
    parse.
  - `host_registry_reload(r, path)` — read fresh config and swap
    the entries map in-place. Existing pointers stay valid; the
    old map is left to the bump allocator. **Fail-safe**:
    returns `-1` and leaves the registry unchanged on
    open/read/parse failure, so a malformed live edit cannot
    drop the running configuration.
  - `host_registry_clear(r)` — empty the registry in-place;
    used by `reload`, also exposed for explicit caller use.
  Plus one internal helper, `_host_parse_str_array`, that walks
  a JSON array of strings into a `vec<cstr>` (used by the
  `capabilities` branch of `host_entry_from_json`).

### Config format

```json
[
  {"name":"upstream-1","url":"https://api.example.com"},
  {"name":"upstream-2","url":"https://other.example.com",
   "capabilities":["fetch","tools_call"]}
]
```

`headers` parsing is deferred — header values may carry secrets
and need a redaction / audit story before they can come through
file config. The in-memory `host_entry_with_headers` setter is
still available for callers that need headers in code.

### Changed

- **`CONTRIBUTING.md` rewritten for the Cyrius era.** The
  pre-port doc referenced `make check` / `cargo fmt` /
  `cargo-deny` / `src/lib.rs` / `Cargo.toml` / MSRV 1.89.
  Replaced with the actual cyrius commands the codebase uses
  today: `cyrius deps` / `cyrius build` / `cyrius test` /
  `cyrius distlib` / `CYRIUS_STATS=1` / `CYRIUS_DCE=1`. Adds
  a Common Commands table, an Adding a New Module section
  that matches the 2.6.3 dist-bundle contract, and a Testing
  section that documents the 8-file split + the parser quirk
  workaround (stage 2-arg inner calls into a var when
  `assert(streq(call(...), "lit") == 1, "msg")` trips
  `expected ')', got string` on cyrius 5.10.x).
- **`tests/bote_host.tcyr`** now includes `lib/io.cyr` and
  `src/jsonx.cyr`. The hot-reload parser piggybacks on jsonx
  for whitespace / string / struct skipping (same surface bote
  uses everywhere else for JSON-ish handling); the file
  roundtrip tests need `file_write_all` from `lib/io.cyr`.

### Verified (cyrius 5.10.x, local)

- **653 unit assertions across 8 test files** — **+46 vs 2.7.0**:
  - `bote_host.tcyr` **113** (+46) from the new hot-reload
    coverage: 4 `host_entry_from_json` shape paths,
    `_host_parse_str_array` empty / two-element / malformed
    cases, `host_registry_load_json` happy / empty-array /
    null / non-array / malformed-entry / unterminated paths,
    `host_registry_load_from_file` roundtrip via `/tmp`,
    `host_registry_reload` swap + fail-safe + missing-file +
    null-arg paths, `host_registry_clear`.
  - Per-file: `bote.tcyr` 398 / `bote_auth.tcyr` 38 /
    `bote_content.tcyr` 24 / `bote_host.tcyr` **113** /
    `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 /
    `bote_sandbox.tcyr` 13 / `bote_libro_tools.tcyr` 22.
- Default binary capacity: fn_table **89%** (3658/4096) —
  +6 vs 2.7.0 from the 5 new public functions + helper. Well
  under the 95% CI gate from 2.6.4.
- `bote_host.tcyr` compile unit absorbs `lib/io.cyr` +
  `src/jsonx.cyr` without strain — under the capacity gate
  comfortably.
- `dist/bote.cyr` regenerated; header reflects 2.7.1.

### Forward roadmap

Remaining 2.7.x candidates after this patch ship:

| Item | Effort |
|---|---|
| OAuth 2.1 AS flow | High — explicitly out of scope, kept on the list as a marker |

The 2.7.x feature slate from the deferred 2.6.x carry-forward
list is otherwise empty. 2.7.x continues opportunistically; the
next 2.7.x patch lands when new in-tree needs emerge. The next
*planned* arc opens at 2.8.x — likely the long-deferred
threaded streaming dispatch, gated on cyrius
`lib/thread.cyr` MPSC + `lib/async.cyr` cancellation firming up.

## [2.7.0] — 2026-05-10 — Annotations through `wrap_tool_result` + bench coverage + `[Unreleased]` flow

Opens the 2.7.x feature line after the 2.6.x modernization arc
closed at 2.6.4. Three Low-effort items from the deferred 2.6.x
candidates list bundled into one carry-forward release; no MCP
wire-format change, no handler-ABI change. Closes the last loose
ends from the 1.9.6 (annotations) and bench-coverage eras.

### Added

- **Annotations propagation through `wrap_tool_result`**
  (`src/bridge.cyr`). A handler returning a single MCP content
  block (object with a `type` field, possibly carrying
  `annotations` from `content_with_annotations`) is now lifted
  into a 1-element `content` array verbatim, preserving the
  annotations. Previously the block got JSON-escaped into a
  synthesised `text` payload — annotations survived as
  characters in a string but became semantically useless to MCP
  clients. Three input shapes are now distinguished:
  1. Object with a `content` array → passthrough (unchanged).
  2. Object with a `type` field → lift into envelope (**new**).
  3. Anything else → wrap as synthesised text block (unchanged).
  Four new assertions in `tests/bote.tcyr` cover the lift path
  for text + image blocks, with and without annotations, plus
  the non-block-object fallthrough.
- **`schema_compile_simple` + `schema_compile_nested`
  benchmarks** in `tests/bote.bcyr`. The existing validation
  benches measure the compiled artifact's runtime cost; these
  new ones measure pure compilation cost. Local numbers (cyrius
  5.10.x, x86_64): `schema_compile_simple` 4µs avg,
  `schema_compile_nested` 7µs avg.
- **`auth_bearer_check_unset` + `auth_bearer_check_set`
  benchmarks** in `tests/bote.bcyr`. Quantifies the no-overhead
  claim from 1.9.0 — the bearer-middleware fast path
  (validator fp == 0) is expected to be essentially free.
  Confirmed: `auth_bearer_check_unset` runs at **1µs avg**
  versus `auth_bearer_check_set` at 2µs (the full path: parse
  Authorization header, call allowlist validator, accept/reject).
- **`## [Unreleased]` section pattern** at the top of
  `CHANGELOG.md`. From 2.7.0 onward, in-flight entries
  accumulate under `[Unreleased]` and the header is renamed
  at release time. Conventions block documents the flow.
- **Bench compile-unit additions**: `tests/bote.bcyr` now
  includes `lib/tagged.cyr`, `lib/net.cyr`, `lib/tls.cyr`,
  `lib/sandhi.cyr`, and `src/auth.cyr` — required by the new
  `auth_bearer_check_*` benches. Compile unit at fn_table
  **85%** (3478/4096), identifier buffer **85%**
  (111806/131072) — under the 95% CI gate from 2.6.4.

### Verified (cyrius 5.10.x, local)

- **607 unit assertions across 8 test files** — **+4 vs 2.6.4**
  from the four new `wrap_tool_result` lift-path assertions.
  Per-file: `bote.tcyr` **398** (+4) / `bote_auth.tcyr` 38 /
  `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_jwt.tcyr`
  28 / `bote_pkce.tcyr` 17 / `bote_sandbox.tcyr` 13 /
  `bote_libro_tools.tcyr` 22.
- **14 criterion benchmarks** — was 10; +4 from
  `schema_compile_{simple,nested}` and
  `auth_bearer_check_{unset,set}`.
- Default binary: fn_table **89%** (3652/4096), identifier
  buffer **88%** (116378/131072) — unchanged vs 2.6.4 (the
  bridge.cyr branch addition adds <5 fns and doesn't move
  the meter at this resolution). Well under the capacity gate.
- Bench compile unit: fn_table **85%** (3478/4096), identifier
  buffer **85%** (111806/131072) — comfortable headroom for
  the four new bench callbacks and their auth + sandhi
  includes.
- `dist/bote.cyr` regenerated; header reflects 2.7.0.

### Forward roadmap

The 2.7.x feature candidates remaining (from the deferred
2.6.x slate):
- **HostRegistry hot-reload from config file** (Medium) —
  useful for deployments that rotate allowed upstreams without
  a restart.
- **CONTRIBUTING.md Cyrius-era cleanup** — the current doc
  still references Rust-era commands (`make check`,
  `cargo-deny`, `src/lib.rs`). Stale since the Cyrius port.
- **OAuth 2.1 authorization-code flow** (High) — out of scope
  for MCP core; bote is the resource server. Explicitly
  deferred — consumers compose bote with their own AS layer.

## [2.6.4] — 2026-05-10 — Capacity gate (closes 2.6.x modernization arc)

Last patch in the 2.6.x modernization arc. The 2.6.0 toolchain
bump pushed the default-binary compile-time fn_table to 89%
(3652/4096) and the identifier buffer to 88% (116370/131072) —
both above cyrius's own 85% in-compiler warning threshold but
neither blocking. 2.6.4 lands a CI gate so the situation can't
quietly drift past the cap during 2.7.x feature work without
forcing the split / cap-raise conversation.

No source-side architectural change; the split / feature-gate
options are documented as response paths if the gate fires.
The 2.7.x feature backlog (deferred from the original 2.6.x
slate by 2.6.0) is now unblocked.

### Added

- **CI capacity gate** (`.github/workflows/ci.yml`). The
  `cyrius build` step now runs under `CYRIUS_STATS=1`, which
  emits a machine-parseable `cyrius stats:` block at the
  tail of stdout:
  ```
  cyrius stats:
    fn_table:    3652 / 4096
    identifiers: 116370 / 131072
    var_table:   1890 / 8192
    fixup_table: 12331 / 262144
    string_data: 30735 / 2097152
    code_size:   1220568 / 1048576
  ```
  A follow-up step parses `fn_table` and `identifiers`,
  computes utilisation, and fails the build if either crosses
  **95%**. Threshold rationale: cyrius itself warns at 85%
  (advisory only); 95% leaves ~5% headroom (~205 fn slots,
  ~7350 ident bytes) for a mid-PR feature add before the
  gate trips. Current util is 89% / 88% — well under the
  gate.
- **Three documented response paths** for when the gate
  ever fires (inline in `.github/workflows/ci.yml` and
  echoed in `docs/development/roadmap.md`):
  1. **Land the cyrius cap raise upstream.** Preferred —
     the cap has moved before (fn_table 2048→4096 in
     cyrius 4.7.1, identifier buffer raised to 131072 in
     4.6.2). File against `cyrius-feedback.md`.
  2. **Split a transport into an opt-in compile unit.**
     Keep stdio + http in the default binary; ws /
     streamable / bridge / unix pulled in via a separate
     `include "src/transport_ws.cyr"` at the consumer's
     `main.cyr`. Mirrors the `libro_tools.cyr` opt-in
     pattern from 2.6.2.
  3. **Feature-gate the unused config-setter surface**
     behind `#ifdef BOTE_FULL_CONFIG`. The lean profile
     would shed ~30 setters that no in-tree consumer calls
     (`bridge_config_with_*`, `streamable_config_with_*`,
     `ws_config_with_*` etc.).

### Verified (cyrius 5.10.x, local)

- **Capacity gate simulation** — local run with the same
  parser CI uses reports `fn_table: 3652 / 4096 (89%)`,
  `identifiers: 116370 / 131072 (88%)`. `GATE: pass`.
- **`CYRIUS_DCE=1` impact measured**: zero. DCE only
  affects emitted code bytes; the fn_table / identifier
  counters are compile-time resource consumption, eaten
  by every `fn` declaration regardless of whether it gets
  emitted. (Captured in the gate's inline rationale so the
  next person doesn't re-test this.)
- **603 unit assertions across 8 test files** — unchanged.
- Build: `src/main.cyr → build/bote` at fn_table 89%
  (3652/4096), identifier buffer 88% (116370/131072) —
  no change vs 2.6.3.
- `dist/bote.cyr` regenerated; header reflects 2.6.4.

### Modernization arc — done

The 2.6.x arc opened at 2.6.0 with the cyrius 5.10.34 /
libro 2.6.2 / majra 2.4.3 floor bump and closes at 2.6.4
with the capacity gate. Five patches, each a contained
bite, no behaviour drift:

| Patch | Bite |
|---|---|
| 2.6.0 | cyrius / libro / majra floor bump + cyrius.cyml + dist-bundle deps + CI installer modernization + sandhi compat shim |
| 2.6.1 | Retire `_sandhi_compat.cyr` — 108 call sites flipped to `sandhi_server_*` |
| 2.6.2 | Port `libro_tools.cyr` to libro 2.6.x API; re-enable `bote_libro_tools.tcyr`; back to 603-assertion baseline |
| 2.6.3 | Ship `dist/bote.cyr` consumer bundle via `cyrius distlib`; CI freshness gate; release asset |
| 2.6.4 | CI capacity gate (95% fn_table / identifier threshold); modernization arc closed |

2.7.x picks up the deferred feature backlog — see
`docs/development/roadmap.md`.

## [2.6.3] — 2026-05-10 — Ship `dist/bote.cyr` for downstream consumers

Fourth patch in the 2.6.x modernization arc. Brings bote onto
the same distribution contract as libro and majra: a
single-file `dist/bote.cyr` bundle that downstream MCP
consumers (phylax, t-ron, sutra, jalwa, rasa, mneme) can pull
via `[deps.bote] modules = ["dist/bote.cyr"]` in their own
`cyrius.cyml`. No source edits to existing consumers; new
consumers get a clean one-file entry point.

No MCP wire-format change, no handler-ABI change, no source
behaviour change.

### Added

- **`dist/bote.cyr` (4615 lines, committed)** — single-file
  distribution bundle generated by `cyrius distlib` from
  `cyrius.cyml [lib] modules`. Self-contained at the
  source level (no `include` directives in-bundle); stdlib +
  libro + majra are supplied by the consumer's own
  `[deps] stdlib` / `[deps.libro]` / `[deps.majra]` set,
  the same way bote pulls libro 2.6.2 and majra 2.4.3.
  `libro_tools.cyr` is **not** in the bundle — it stays
  opt-in (see 2.6.2 notes). Header carries
  `# Version: 2.6.3` for runtime introspection by consumers
  that want to detect the bundle version.
- **CI dist-freshness gate** (`.github/workflows/ci.yml`).
  Every CI run regenerates `dist/bote.cyr` and asserts a
  byte-clean diff against the committed file. Catches the
  failure mode where a `src/*.cyr` change lands without a
  follow-up `cyrius distlib` — downstream `cyrius deps` at
  our tag would otherwise pull stale source. Mirrors the
  libro / majra freshness gate.
- **Release workflow regenerates + ships `dist/bote.cyr`**
  (`.github/workflows/release.yml`). The bundle joins the
  release asset set as `bote-<ver>.cyr` alongside the
  source tarball, the x86_64 binary, `cyrius.lock`, and
  `SHA256SUMS`. A tagged release without the bundle would
  break every consumer's `cyrius deps` at the tag — the
  release gate fails fast on a stale or missing bundle.

### Consumer wire-up (canonical form)

```toml
# In a consumer's cyrius.cyml:
[deps.bote]
git = "https://github.com/MacCracken/bote"
tag = "2.6.3"
modules = ["dist/bote.cyr"]
```

`cyrius deps` clones bote at the tag and copies
`dist/bote.cyr` into the consumer's `lib/bote.cyr`. The
consumer then `include "lib/bote.cyr"` from their own
`src/main.cyr`, alongside their stdlib + libro + majra dep
includes. The bundle's `registry_new` / `dispatcher_new` /
`transport_*_run` / `auth_*` / `content_*` / `host_*` /
`audit_*` / `events_*` surfaces are all available.

### Verified (cyrius 5.10.x, local)

- **`cyrius distlib`** emits `dist/bote.cyr` deterministically;
  bundle header captures version from `cyrius.cyml`.
- **Consumer-perspective compile** — synthesised a
  consumer-style entry that includes `dist/bote.cyr` plus
  the stdlib + libro + majra dep set; compiles cleanly,
  binary runs and exits 0.
- **603 unit assertions across 8 test files** — unchanged
  from 2.6.2; the distlib step is a pure post-processing
  step that doesn't touch the build path.
- Build: `src/main.cyr → build/bote` still at fn_table 89%
  (3652/4096); no change in default-binary footprint.

### Forward roadmap

- 2.6.4: capacity / split prep — last patch in the
  modernization arc. Decide between splitting the streamable
  / WS transports into an opt-in compile unit, feature-gating
  the unused-config setters, or holding for the next cyrius
  cap raise.

## [2.6.2] — 2026-05-10 — Port libro_tools.cyr to libro 2.6.x

Third patch in the 2.6.x modernization arc. The 2.6.0 release
deferred `src/libro_tools.cyr` (the five built-in MCP tools that
expose the libro audit chain) because libro 2.x retired the
public `entry_*` / `error_*` / `merkle_*` accessors that the
2.5.1 source assumed. 2.6.2 lands the port and re-enables
`tests/bote_libro_tools.tcyr` — bote is now back to **603
passing assertions** across 8 active test files, matching the
2.5.1 baseline.

Still no MCP wire-format change, no handler-ABI change; the
five tool handlers emit byte-identical JSON for byte-identical
input. The port is internal-only.

### Changed

- **`src/libro_tools.cyr` ported to the libro 2.6.x API**:
  - Replaced six retired entry accessors (`entry_timestamp` /
    `entry_severity` / `entry_source` / `entry_action` /
    `entry_agent_id` / `entry_hash`) with raw struct-offset
    reads via local helpers `_lt_entry_timestamp` etc. The
    libro 2.x entry struct layout is documented inline at
    `src/libro_tools.cyr:46-58` and tracks
    `lib/libro.cyr:200-205`.
  - Replaced the retired `chain_entries(c)` getter with a
    one-line `_lt_chain_entries(c) { return load64(c); }` —
    the libro chain struct keeps the entries vec at +0 and
    is unlikely to drift.
  - Replaced the retired `error_code` / `error_index` /
    `error_msg` getters with raw-offset helpers (`_lt_err_code`
    / `_lt_err_index` / `_lt_err_msg`). The libro 2.x error
    struct is 6 fields wide (`code`, `msg`, `field_name`,
    `index`, `expected`, `actual`) and the message field is
    now a cstr rather than a Str — the wrapper drops the
    `str_data` unwrap.
  - Renamed `merkle_proof(tree, idx)` →
    `merkle_inclusion_proof(tree, idx)`.
  - Replaced the retired `merkle_tree_leaf_count(tree)` with
    `_lt_merkle_leaf_count(t) { return load64(t + 8); }` —
    the libro 2.x merkle_tree struct is `{nodes, leaf_count}`
    so leaf_count sits at +8.
- **`tests/bote_libro_tools.tcyr` re-enabled and updated for
  the 2.6.0 dist-bundle layout**:
  - Nine individual `lib/libro_*.cyr` includes collapsed to
    a single `include "lib/libro.cyr"` (the dist bundle).
  - Added `lib/fs.cyr` / `lib/ct.cyr` / `lib/keccak.cyr` /
    `lib/process.cyr` / `lib/random.cyr` to the include set
    — libro 2.6.x consumes these transitively at parse time
    (`file_open`, `ct_eq`, `random_bytes` etc.).
  - Updated all 9 handler invocations to pass the
    2-arg `(args, claims)` ABI (was 1-arg in the 2.5.1
    source; the second slot landed in 2.0.0 but the test
    file never caught up).
- **`.github/workflows/ci.yml`**: `bote_libro_tools.tcyr`
  added to the test matrix. 8th test file is now mandatory.
- **`src/main.cyr` header comment**: the `DEFERRED:` marker
  on the `include "src/libro_tools.cyr"` line is now an
  `OPT-IN:` marker — the module compiles cleanly, and
  consumers that want libro audit-tool dispatch can include
  + wire it locally without touching the bote default binary.
  Kept out of the default build because the main binary is
  at 89% fn_table utilisation; the libro_tools handlers
  would tip the build over.

### Verified (cyrius 5.10.x, local)

- **603 unit assertions across 8 test files** — back to the
  2.5.1 baseline:
  - `bote.tcyr` **394** / `bote_auth.tcyr` 38 /
    `bote_content.tcyr` 24 / `bote_host.tcyr` 67 /
    `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 /
    `bote_sandbox.tcyr` 13 / `bote_libro_tools.tcyr` **22**.
- Default binary build: `src/main.cyr → build/bote` still at
  fn_table 89% (3652/4096) — no change vs 2.6.1 (libro_tools
  stays out of main).
- `bote_libro_tools.tcyr` compile unit: fn_table 85%
  (3485/4096), identifier buffer 85% (111665/131072) —
  comfortable headroom.

### Forward roadmap

- 2.6.3: emit `dist/bote.cyr` via `cyrius distlib`.
- 2.6.4: capacity / split prep.

## [2.6.1] — 2026-05-10 — Retire the sandhi compat shim

Second patch in the 2.6.x modernization arc. The 2.6.0 release
bridged `lib/http_server.cyr` (retired in cyrius 5.10.x) to the
new `lib/sandhi.cyr` HTTP-server surface via a thin compat shim
(`src/_sandhi_compat.cyr`) so the version bump didn't churn
every transport file. 2.6.1 retires the shim with a mechanical
rename pass — bote now calls into sandhi directly.

No wire-format change, no handler-ABI change, no behaviour
drift; this is a name-only refactor.

### Changed

- **108 HTTP-server call sites renamed.** Across
  `src/auth.cyr`, `src/bridge.cyr`, `src/transport_http.cyr`,
  `src/transport_streamable.cyr`, `src/transport_ws.cyr`, plus
  `tests/bote.tcyr` + `tests/bote_auth.tcyr`. Twelve symbols
  flipped from the pre-5.10 `http_*` family to the
  `sandhi_server_*` names that landed in cyrius 5.10's
  `lib/sandhi.cyr`:
  - `http_send_status`         → `sandhi_server_send_status`
  - `http_send_response`       → `sandhi_server_send_response`
  - `http_send_chunked_start`  → `sandhi_server_send_chunked_start`
  - `http_send_chunk`          → `sandhi_server_send_chunk`
  - `http_send_chunked_end`    → `sandhi_server_send_chunked_end`
  - `http_get_method`          → `sandhi_server_get_method`
  - `http_get_path`            → `sandhi_server_get_path`
  - `http_find_header`         → `sandhi_server_find_header`
  - `http_content_length`      → `sandhi_server_content_length`
  - `http_body_offset`         → `sandhi_server_body_offset`
  - `http_path_only`           → `sandhi_server_path_only`
  - `http_server_run`          → `sandhi_server_run`
  `HTTP_*` status-code constants (`HTTP_OK`, `HTTP_BAD_REQUEST`,
  `HTTP_UNAUTHORIZED`, etc.) are unchanged — sandhi exports them
  under the same names.

### Removed

- **`src/_sandhi_compat.cyr` deleted.** The 2.6.0 shim
  (~50 LoC, 12 tail-call wrappers) was a transition aid only;
  with all call sites renamed the file has no callers left.
- **The CI manifest-completeness gate's `EXCLUDES` list** (.github/workflows/ci.yml)
  is gone — `[lib]` modules in `cyrius.cyml` now exactly equal
  the set of `src/<file>.cyr` includes in `main.cyr`. No
  exceptions to track.

### Verified (cyrius 5.10.x, local)

- **581 unit assertions** across 7 active test files — same
  counts as 2.6.0; no regression.
  - `bote.tcyr` 394 / `bote_auth.tcyr` 38 /
    `bote_content.tcyr` 24 / `bote_host.tcyr` 67 /
    `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 /
    `bote_sandbox.tcyr` 13.
- Production build: `src/main.cyr → build/bote` succeeds.
  Function-table util 89% (3652/4096, -11 vs 2.6.0 from
  removing the 12 shim wrappers); identifier buffer 88%
  (116370/131072, -196 bytes).
- Smoke test: `initialize` round-trips clean,
  `serverInfo.version == "2.6.1"`.

### Forward roadmap

- 2.6.2: port `src/libro_tools.cyr` to the libro 2.6.x API
  and re-enable `tests/bote_libro_tools.tcyr` (22 assertions).
- 2.6.3: emit `dist/bote.cyr` via `cyrius distlib`.
- 2.6.4: capacity / split prep.

## [2.6.0] — 2026-05-10 — Modernization platform: cyrius 5.10.34, libro 2.6.2, majra 2.4.3

2.6.0 catches bote up to the first-party Cyrius floor and opens
the **2.6.x modernization arc** (see `docs/development/roadmap.md`).
No MCP wire-format change, no handler-ABI change; the live
audit + events integration is byte-identical to 2.5.1 at the
JSON-RPC boundary. Forward feature work that was slotted for
2.6.x slides one minor to 2.7.x; 2.6.x is reserved for the
modernization sequence.

### Changed

- **Cyrius toolchain pin: 4.8.4 → 5.10.34.** Matches the
  current first-party floor (agnosys 1.2.4, agnostik 1.2.1,
  libro 2.6.2, majra 2.4.3). Notable upstream changes spanning
  this range: arch-peer include resolution now expects
  `~/.cyrius/versions/<V>/lib` (5.10.9+) — CI installer
  updated accordingly; richer fmt/lint/vet/capacity surfaces;
  `CYRIUS_DCE=1` available for release binaries; raised fixup
  cap; stdlib `ct_eq_bytes` family; `secret` promoted to a
  storage-class keyword (forced a rename in `src/jwt.cyr`);
  `lib/http_server.cyr` retired from stdlib and folded into
  the new `lib/sandhi.cyr` HTTP-server surface.
- **libro 1.0.3 → 2.6.2 via single-file dist bundle.** The
  consumer contract switched in libro 2.x from `[deps.libro]
  modules = ["src/error.cyr", "src/hasher.cyr", …]` to
  `modules = ["dist/libro.cyr"]`. `cyrius deps` copies the
  upstream `dist/libro.cyr` into `lib/libro.cyr`. All
  symbol-level call sites bote uses (`chain_new`,
  `chain_append`, `chain_append_with_agent`, `chain_verify`,
  `pubsub_*`) are unchanged in the new dist surface — the
  live audit_libro / events_majra adapters compile clean
  without source edits.
- **majra 2.2.0 → 2.4.3 via single-file dist bundle.** Same
  pattern — `[deps.majra] modules = ["dist/majra.cyr"]`,
  resolves to `lib/majra.cyr`. The default profile (core
  pubsub engine without backends) is the bote-appropriate
  bundle; future transport work can upgrade to
  `dist/majra-signed.cyr` or `-backends.cyr` if signed
  envelopes or network backends become bote-side concerns.
- **`cyrius.toml` → `cyrius.cyml`.** Adopts the first-party
  manifest layout: `version = "${file:VERSION}"` placeholder
  (the version is owned by `VERSION`, the manifest pulls it),
  `[lib]` section enumerating bote's own modules for future
  `cyrius distlib dist/bote.cyr` (2.6.3), `cyrius = "5.10.34"`
  toolchain pin in the manifest (no separate
  `.cyrius-toolchain` file). The release workflow enforces
  the `${file:VERSION}` placeholder so the version can't drift
  out of sync at tag time.
- **`/lib/` is no longer committed.** `.gitignore` covers
  `/lib/` — `cyrius deps` repopulates it from the
  version-pinned stdlib snapshot plus the tagged libro / majra
  dist bundles. Matches agnosys / majra / libro / yukti /
  patra convention. Prevents stale stubs from prior cyrius
  versions sitting in tree.
- **HTTP server surface bridged from `http_server` to `sandhi`
  via `src/_sandhi_compat.cyr`.** The cyrius 5.10.x stdlib
  retired `lib/http_server.cyr` and folded its surface into
  `lib/sandhi.cyr` under a `sandhi_server_` prefix. 2.6.0
  introduces a thin compat shim re-exporting the ~12 symbols
  bote uses (`http_send_status`, `http_send_response`,
  `http_send_chunked_*`, `http_get_method`, `http_get_path`,
  `http_find_header`, `http_content_length`, `http_body_offset`,
  `http_path_only`, `http_server_run`) as tail-calls into
  the sandhi names. Zero call-site churn in 2.6.0; the rename
  pass and shim retirement land in 2.6.1.
- **`tls` added to `[deps] stdlib`.** sandhi references
  `TLS_EARLY_DATA_ACCEPTED` at parse time — without `tls.cyr`
  in the dep set, cyrius's deps-aware build can't validate
  the dep graph. Mirrors majra 2.4.2's same addition.
- **`src/jwt.cyr` — `secret` parameter renamed to `key`.**
  `secret` is a storage-class keyword in cyrius 5.10
  (sigil's HMAC ipad/opad buffers use it). Renamed inside
  `jwt_verify_hs256` and `jwt_secret_new`; the exported
  function names (`jwt_secret_new`, `jwt_secret_data`,
  `jwt_secret_len`, `auth_validator_jwt_hs256`) are unchanged
  — `secret` is reserved only as a bare identifier, not
  inside compound names. `tests/bote_jwt.tcyr` renamed its
  local `var secret = "..."` to `var jwt_key = "..."` for the
  same reason.

### CI / release modernization (matches majra / agnosys)

- **Versioned toolchain installer.** The installer now lays
  out `~/.cyrius/versions/<V>/{bin,lib}` and symlinks
  `~/.cyrius/{bin,lib}` to the version-pinned snapshot.
  Required by cc5 5.10.9+: arch-peer includes
  (`syscalls_x86_64_linux.cyr` etc.) resolve against the
  version-pinned `lib/`, so the flat `~/.cyrius/lib` layout
  the 4.8.x installer used no longer works.
- **Source-archive fetch for `lib/`.** 5.10.x release
  tarballs ship `bin/` + `deps/` only — the `lib/` stdlib
  snapshot is NOT in the tarball. CI now pulls the GitHub
  source archive at the version tag and copies `lib/` from
  there. Sanity-checked against `syscalls_x86_64_linux.cyr`
  + `bin/cc5` to fail fast if either extraction is broken.
- **`cyrius deps --verify` lockfile gate.** `cyrius.lock`
  (committed) records SHA-256 of every resolved dep. CI
  enforces hash match; the gate self-skips on the first push
  that introduces a new dep before the lockfile lands.
- **Manifest-completeness gate.** Every
  `include "src/<file>.cyr"` in `src/main.cyr` must be listed
  under `[lib]` modules in `cyrius.cyml` (or be in the known
  excludes — currently `src/_sandhi_compat.cyr`). Prevents
  `cyrius distlib` from silently shipping a bundle missing a
  module once 2.6.3 lands.
- **`CYRIUS_NO_WARN_SHADOW_LIB=1` + `CYRIUS_DCE=1`.** Standard
  env across CI / release steps — silences the
  cwd-shadows-version-snapshot informational note and turns
  on whole-program dead-code elimination. Matches majra /
  libro / agnosys / agnostik.
- **Release workflow accepts both `v1.2.3` and `1.2.3` tag
  styles.** The version-verify step enforces semver shape +
  exact match against the `VERSION` file + the
  `${file:VERSION}` placeholder in `cyrius.cyml`. Release
  notes auto-extracted from the matching `## [VERSION]`
  section of `CHANGELOG.md`.
- **Release artifacts.** Source tarball
  (`bote-<ver>-src.tar.gz`), x86_64-linux binary
  (`bote-<ver>-x86_64-linux`), `cyrius.lock`, and
  `SHA256SUMS` over all artifacts. Matches majra's release
  asset set.

### Deferred

- **`tests/bote_libro_tools.tcyr`** (22 assertions) is parked
  for 2.6.2 alongside the `src/libro_tools.cyr` port. The
  libro 2.x dist bundle dropped six entry accessors
  (`entry_action` / `entry_severity` / `entry_hash` /
  `entry_source` / `entry_agent_id` / `entry_timestamp`),
  replaced `merkle_proof` with `merkle_inclusion_proof`,
  retired `merkle_tree_leaf_count` (now `merkle_canonical_root(tree, size)`
  with explicit size), and folded
  `libro_export` into `export_jsonl` over a fd. The
  libro-tool dispatcher rewrites cleanly against the new
  surface but is its own focused bite; targeting 2.6.2.
- **The `_sandhi_compat.cyr` shim retires in 2.6.1.** ~56
  call sites across `transport_http.cyr` /
  `transport_streamable.cyr` / `bridge.cyr` /
  `transport_ws.cyr` / `auth.cyr` flip from `http_*` to
  `sandhi_server_*`; mechanical pass.

### Verified (cyrius 5.10.x, local)

- **581 unit assertions across 7 test files** —
  `bote.tcyr` **394** / `bote_auth.tcyr` 38 /
  `bote_content.tcyr` 24 / `bote_host.tcyr` 67 /
  `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 /
  `bote_sandbox.tcyr` 13. (Original 2.5.1 = 603 over 8
  files; the 22-assertion gap is `bote_libro_tools.tcyr`
  parked for 2.6.2 — no regression on the live integration.)
- Production build: `src/main.cyr → build/bote` succeeds
  cleanly. Function-table utilisation 89% (3663/4096) and
  identifier buffer 88% (116566/131072) — tracked for 2.6.4
  if the libro_tools restore pushes us over.
- Benchmarks: `tests/bote.bcyr → build/bote_bench` builds
  clean.
- `cyrius deps` resolves 6 deps and writes `cyrius.lock`;
  `cyrius deps --verify` round-trips clean.

### Forward roadmap

`docs/development/roadmap.md` rewritten: 2.6.x is now the
modernization arc (2.6.0–2.6.4 bounded above); the
previously-2.6.x feature backlog moved to **2.7.x candidates**.

## [2.5.1] — 2026-04-14 — Restore audit_libro + events_majra tests (cyrius 4.8.4 retag)

The cyrius lang-agent retagged 4.8.4 with the alpha2-that-actually-works
binary (fix for the 2.5.0-era local-vs-CI binary skew reported in
`docs/bugs/cyrius-4.8.4-ci-binary-skew.md`). Clean-installing the
updated toolchain lets bote carry the full libro + majra + sigil
dependency graph in `tests/bote.tcyr` again — no more workaround.

### Restored
- **`tests/bote.tcyr`** — re-added the full dep-graph includes dropped during the 2.5.0 CI workaround:
  - `lib/sakshi.cyr`, `lib/bigint.cyr`, `lib/sigil.cyr`
  - `lib/libro_*.cyr` (error / hasher / entry / verify / query / retention / chain)
  - `lib/majra_*.cyr` (error / counter / envelope / namespace / queue / pubsub)
  - `src/audit_libro.cyr`, `src/events_majra.cyr` — now usable again because `SEV_INFO` + friends resolve
- **8 shape-only test assertions recovered**:
  - `audit_libro` struct + accessors (7 asserts: chain handle, default source, custom source, agent_id)
  - `audit_libro` AuditSink wire-up fp-addressability
  - `events_majra` EventSink wire-up (fp addressable + ctx=0 call)

### Verified (fresh cyrius 4.8.4 install)
- All 8 test files green: `bote.tcyr` **394** (was 386 in 2.5.0) / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_auth.tcyr` 38 / `bote_sandbox.tcyr` 13 / `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 = **603 total** (back to pre-trim count).
- Production build: `src/main.cyr -> bote 370480 bytes 674ms [x86]`.
- Local `cc3` rebuilt from the retagged cyrius repo.

### Resolved
- `docs/bugs/cyrius-4.8.4-ci-binary-skew.md` — closed by the retag. Roadmap "Open bugs" entry flipped to ✅.

### Carried forward
- v1.2.1 libro-growth heisenbug: unchanged.
- Slowloris recv timeout (audit H5): still needs `sock_set_recv_timeout` stdlib helper.
- WS handshake key-length validation (audit M4): upstream stdlib.
- DNS for SSRF hostname classification: still needs `getaddrinfo` stub.
- JWT RS256 / ES256: still waits on sigil RSA / ECDSA primitives.

## [2.5.0] — 2026-04-14 — Claims propagation + cyrius 4.8.4 pin

Lands the claims-propagation refactor that 2.0's handler-ABI break
was designed for. Validators' return values now flow transport →
codec → dispatcher → `fncall2(handler, args, claims)`. Handlers that
want per-tool authorization can inspect `claims` directly (opaque
cstr, JWT payload ptr, whatever the backend validator produced).
Handlers that ignore it see no change — the 2nd arg is just unused.

Pairs with **cyrius 4.8.4**, which closed the three 4.8.3 regressions
that temporarily blocked the refactor (see `docs/bugs/cyrius-4.8.3-
regressions.md` — all three fixes landed in 4.8.4 as advertised).

### Changed
- **cyrius pin**: 4.8.1 → 4.8.4 (needed for the path-traversal fix,
  include-once cap 64→256, and nested-include `PP_IFDEF_PASS` fixpoint).
- **`src/auth.cyr`** — `auth_bearer_check` gains a `claims_out` ptr param;
  writes the validator's return to `*claims_out` on success. Passing 0
  means "don't care" (e.g. streamable GET which doesn't dispatch).
- **`src/dispatch.cyr`** — `dispatcher_dispatch(d, request)` → `(d, request, claims)`; `fncall2(fp, args, 0)` → `fncall2(fp, args, claims)`.
- **`src/codec.cyr`** — `codec_process_message` + `_cdc_process_single` both gain `claims` arg; thread through to `dispatcher_dispatch`.
- **`src/bridge.cyr`** — `bridge_process_message` + `_bridge_process_single` both gain `claims`; handler captures into `claims_slot[8]`, passes through.
- **Per-transport handlers** — `transport_http` / `transport_ws` / `bridge` declare `var claims_slot[8]; store64(&claims_slot, 0);`, pass `&claims_slot` to `auth_bearer_check`, then `load64(&claims_slot)` into the dispatch call. `transport_streamable` does the same for its POST path; its GET path passes 0 (SSE stream, no dispatch). `transport_stdio` and `transport_unix` (local-only, no auth) pass 0 verbatim.
- **`tests/bote.tcyr`** — `lib/http_server.cyr` added explicitly (the dep resolver wasn't pulling it transitively for this compile unit). libro+majra+sigil+sakshi+bigint `include`s tried locally (compiled fine on `cc3 4.8.4-alpha2`) but reverted after the 4.8.4 release binary on CI hit the misleading `lib/assert.cyr:3` parse error at `fn=908/4096` — well under cap but still tripping. Left for a later round once the CI binary skew is resolved.

### Validator contract (handler perspective)

Handlers now look like:

```cyr
fn my_tool(args, claims) {
    # claims is 0 if auth was disabled at the transport config, or the
    # validator's non-zero return otherwise. For the bundled validators:
    #   auth_validator_allow_all      → returns the token cstr
    #   auth_validator_allowlist      → returns the token cstr on match
    #   auth_validator_jwt_hs256      → returns the token cstr on valid JWT
    # A consumer-supplied validator may return a parsed claims struct.
    # Treat as opaque unless you know which validator is configured.
    if (claims == 0) { return _err("auth required"); }
    # ... use `args` + `claims` ...
    return some_result;
}
```

### Verified (cyrius 4.8.4)
- All 8 test files green: `bote.tcyr` 386 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_auth.tcyr` 38 / `bote_sandbox.tcyr` 13 / `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 = **595 total**.
- The audit_libro + events_majra shape-only tests that tried to fit into `bote.tcyr` on 2.4.0-era were dropped here too — they'd need `lib/libro_*` + `lib/majra_*` in `bote.tcyr`'s compile unit, which locally passes cyrius 4.8.4-alpha2 but fails on the 4.8.4 release binary that CI installs. Keeping `bote.tcyr` lean and not pulling those in dodges the version-skew issue while we work out what's different across 4.8.4 builds.
- Production build: `src/main.cyr -> bote 370480 bytes 684ms [x86]` with 15 undefined-fn warnings (same as 2.4.0 — libro heisenbug-avoidance stubs).
- `cyrlint src/auth.cyr src/dispatch.cyr src/codec.cyr src/bridge.cyr` → 0 warnings.
- Live HTTP: `./bote http 8390` + `curl -X POST -H 'Authorization: Bearer tok' ...` with `BOTE_BEARER_TOKENS=tok` — handler receives `args` + `claims=tok` (bearer middleware unchanged; just the plumbing downstream).
- 4.8.4 capacity meter on `tests/bote.tcyr`: `fn_table 1233/4096`, `identifiers 32312/131072` (25% used) — comfortable headroom for future 2.x features.

### Resolved from prior reports
- `docs/bugs/cyrius-4.8.3-regressions.md` — all three blockers closed in 4.8.4.
- `docs/development/roadmap.md` — the "waiting on 4.8.3 capacity meter" note can now be cleared; next refactor cycle has hard numbers.

### Carried forward
- v1.2.1 libro-growth heisenbug: unchanged.
- Slowloris recv timeout (audit H5): still needs `sock_set_recv_timeout` stdlib helper.
- WS handshake key-length validation (audit M4): upstream stdlib.
- DNS for SSRF hostname classification: still needs `getaddrinfo` stub.
- JWT RS256 / ES256: still waits on sigil RSA / ECDSA primitives.

## [2.4.0] — 2026-04-14 — Bump cyrius 4.8.1 + base64url adoption + compile-unit trim

Toolchain bump and structural cleanup. The cyrius 4.8.1 stdlib added
`base64url_encode` / `base64url_decode` (the proposal we shipped in
2.2.0 / cleaned up in 2.3.1 — your cyrius agent landed it). This
release adopts those primitives and trims compile-unit headroom enough
that future ABI work (claims propagation in 2.5) fits cleanly.

### Changed
- **cyrius pin** `4.7.1` → `4.8.1` (`cyrius.toml` + `.cyrius-toolchain`).
- **`src/jwt.cyr` lifts to stdlib base64url.** The inline `_jwt_b64u_val` table-builder + `jwt_b64u_decode` byte-loop (~55 LOC, 2 fns) is now a 7-line wrapper that calls `base64url_decode` from `lib/base64.cyr`. Test signatures unchanged — the wrapper translates the stdlib's `{ptr, len}` 16-byte pair to our existing `(ptr, *out_len)` shape so `tests/bote_jwt.tcyr` passes unchanged.
- **`src/main.cyr` trim.** Removed `lib/assert.cyr`, `lib/sakshi.cyr`, `lib/bigint.cyr` from the production main.cyr include list. None were actually referenced by main — they were transitively pulled in once-upon-a-time and never cleaned. Frees ~50 fns from production compile unit (was 1001 → 943 unreachable).
- **`tests/bote.tcyr` trim.** Removed `lib/sakshi.cyr`, `lib/bigint.cyr`, `lib/sigil.cyr`, all `lib/libro_*.cyr`, all `lib/majra_*.cyr` from the **core test** compile unit. Those modules were only needed by integration tests that already live in their own per-module test files (`tests/bote_libro_tools.tcyr`, etc.). Frees enough headroom (was 830 → 457 unreachable) that the larger 4.8.1 `lib/base64.cyr` (67 LOC → 177 LOC for the new base64url variants) doesn't tip the cap. The `&libro_audit_log` reference in `bote.tcyr` becomes an undefined-fn warning at compile — harmless because that test only checks fn-pointer addressability, never invokes.

### Verified (cyrius 4.8.1)
- All 8 test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_auth.tcyr` 38 / `bote_sandbox.tcyr` 13 / `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 = **603 total** (unchanged).
- `cyrius build src/main.cyr bote` → OK; `./bote` reports `"version":"2.4.0"`.
- `cyrlint src/jwt.cyr src/main.cyr` → 0 warnings.

### What this unlocks (deferred to 2.5.0)
- **Claims propagation through transports.** The plumbing change (auth_bearer_check → codec_process_message → dispatcher_dispatch all gain a `claims` arg, transports capture and thread the validator's return) was attempted in 2.3.0 and reverted because it tipped the test cap. With the trim landed here, **the freed headroom is enough that the same attempt should succeed** — left for 2.5.0 because the patch touches every transport handler and warrants its own focused release.

### Carried forward
- v1.2.1 libro-growth heisenbug: unchanged.
- Slowloris recv timeout (audit H5): still needs a `sock_set_recv_timeout` stdlib helper.
- WS handshake key-length validation (audit M4): still upstream stdlib.
- DNS for SSRF hostname classification: still needs cyrius `getaddrinfo` stub.
- JWT RS256 / ES256: still waits on sigil RSA / ECDSA primitives.

## [2.3.1] — 2026-04-14 — Cleanup: remove proposal docs that landed upstream

All three of bote's proposal documents have been folded into cyrius
stdlib. **`base64url_encode` / `base64url_decode` shipped in cyrius
4.8.1** (the ask in `cyrius-stdlib-base64url.md` from this release
cycle), joining the earlier two:

| Proposal | Landed in cyrius |
|---|---|
| `cyrius-stdlib-http-server.md` (with `lib_http_server.cyr` + example) | 4.5.0 (bote adopted in 1.3.0) |
| `cyrius-stdlib-ws-server.md` (with `lib_ws_server.cyr` + example) | 4.5.1 (bote adopted in 1.5.0) |
| `cyrius-stdlib-base64url.md` | **4.8.1** (bote will adopt in 2.4.x) |

### Removed
- `docs/proposals/` directory contents (all 7 files): the three `*.md` proposals + their companion `lib_*.cyr` reference impls / runnable examples. Workflow is preserved — future proposals will land back under the same path.

### Adoption note (deferred to 2.4.x)
- `src/jwt.cyr`'s inline `jwt_b64u_decode` (~50 LOC + a few fns) can now be replaced with a call to stdlib `lib/base64.cyr`'s `base64url_decode`. Lifting alongside the cyrius pin bump (4.7.1 → 4.8.x) will free meaningful identifier-table headroom — enough that the 2.3.0-era claims-propagation work (which tipped the test cap and was reverted) can land in 2.4.0.

### Verified
- All 8 test files green (603 total, unchanged from 2.3.0).
- `./bote` reports `"version":"2.3.1"`.

## [2.3.0] — 2026-04-14 — RFC 7636 PKCE-S256 helpers

OAuth 2.1 mandates PKCE on every authorization-code flow; this release
ships the verifier + S256 challenge primitives so MCP clients running
inside bote-hosted handlers can initiate the auth dance without
reimplementing the (small but easy-to-mess-up) crypto. Pairs with the
JWT verifier (2.2.0) and bearer middleware (1.9.0).

### Added
- **`src/pkce.cyr`** (~80 LOC, opt-in module):
  - `pkce_code_verifier(out_buf, len) → 0|err` — writes `len` URL-safe random bytes from `/dev/urandom`. RFC 7636 §4.1 length [43..128]; `out_buf` must be `len + 1` bytes (NUL-terminated). Returns non-zero on out-of-range len or entropy failure.
  - `pkce_code_challenge_s256(verifier) → cstr` — `base64url(sha256(verifier))`, no padding. Uses sigil's `sha256` one-shot. Returns a NUL-terminated 43-char cstr (32 bytes → 43 chars).

### Notes
- **S256 only.** OAuth 2.1 explicitly removes the `plain` method; we don't ship it.
- **Mod-bias.** Verifier byte mapping is `urandom_byte % 66` over the unreserved-character set. Worst-case bias per char is ~0.4% (256/66 = 3 remainder); for 43+-char tokens this is negligible to any guessing attack.
- **No SHA-256 reimplementation.** Calls sigil's existing `sha256(data, len, out)` one-shot — same approach as 2.2.0's HMAC use.

### Tests
- **`tests/bote_pkce.tcyr`** — 17 assertions:
  - Verifier length 43 (RFC minimum) + 128 (RFC maximum) succeed
  - Out-of-range (42, 129) rejected
  - Verifier bytes are all in the RFC unreserved set (`[A-Za-z0-9._~-]`)
  - Two consecutive verifiers differ (entropy sanity)
  - **RFC 7636 Appendix B reference vector**: verifier `dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk` → challenge `E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM`
  - Challenge is deterministic
  - Out-of-range / null verifier rejected
  - Fn-pointer addressability for plugging into auth flows
- **603 total** (was 586). Breakdown: `bote.tcyr` 394, `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 24, `bote_host.tcyr` 67, `bote_auth.tcyr` 38, `bote_sandbox.tcyr` 13, `bote_jwt.tcyr` 28, `bote_pkce.tcyr` 17.

### Verified (cyrius 4.7.1)
- All eight test files green.
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 2.2.
- `cyrlint src/pkce.cyr tests/bote_pkce.tcyr` → 0 warnings.
- `./bote` reports `"version":"2.3.0"`.

### Deferred from 2.3.0 to 2.4.0 — claims propagation through transports
Started this release; reverted before tagging. The plumbing change (auth_bearer_check → codec_process_message → dispatcher_dispatch all gain a `claims` arg, transports capture and thread the validator's return) all compiles and `src/main.cyr` builds cleanly — but it tipped `tests/bote.tcyr`'s compile unit past the cyrius 4.7.1 identifier-buffer cap (the misleading `lib/assert.cyr:3` cascade). Production runtime is unaffected; the test file is the cap-pressured one. Will land cleanly when cyrius 4.8.0 raises the cap.

### Roadmap
- **Claims propagation** → 2.4.0 once 4.8.0 lands.
- **OAuth 2.1 authorization-code endpoints** (Bote-as-AS rather than Bote-as-RS) — out of scope; bote is a resource server. Consumers wanting a full AS layer compose bote with their own authorization server.
- **JWT RS256 / ES256** — waits on sigil's RSA / ECDSA primitives.

## [2.2.0] — 2026-04-14 — JWT HS256 verifier (auth roadmap)

Lands the JWT bearer-token verifier from the 2.0 deferred list. Pairs
with the bearer middleware (1.9.0) — plug `auth_validator_jwt_hs256`
into `auth_bearer_check` and your bearer endpoint validates RFC 7519
JWTs against an HMAC-SHA256 secret.

### Added
- **`src/jwt.cyr`** (~170 LOC). Opt-in module; not wired into the default `main.cyr`. Consumers `include` it explicitly.
  - **`jwt_verify_hs256(token, secret, secret_len) → 0|1`** — full RFC 7515 §3.5 verification: 3-segment split, header decode + `alg=HS256` check (rejects `alg=none` downgrade), HMAC-SHA256 over `header.payload`, constant-time signature compare.
  - **`jwt_secret_new(secret) → handle`** + accessors. 16-byte handle, opaque to bote.
  - **`auth_validator_jwt_hs256(token, secret_handle) → claims | 0`** — drop-in for the bearer middleware fn-pointer slot. Returns the original token cstr as opaque "claims" on success.
  - **`jwt_b64u_decode(enc, elen, *out_len) → ptr | 0`** — RFC 4648 §5 base64url decoder. Inlined for now; will lift to stdlib `lib/base64.cyr` once `cyrius-stdlib-base64url.md` lands (proposal in `docs/proposals/`).
- **HMAC-SHA256 via sigil.** Calls sigil's existing `hmac_sha256` (already in the bundled `lib/sigil.cyr`) — bote doesn't reimplement. Saves ~30 LOC of inline crypto.

### Security properties
- **`alg=none` downgrade rejected** — the header is decoded and scanned for the literal `HS256`. A token with `{"alg":"none"}` cannot pass even with an empty signature.
- **Constant-time signature compare** — `_jwt_ct_eq` XOR-accumulates all 32 bytes with no early exit. Defeats the per-byte timing oracle.
- **Caller-supplied secret only** — bote doesn't store, log, or expose the secret. The validator handle holds a pointer + length; rotation is the caller's responsibility.

### Tests
- **`tests/bote_jwt.tcyr`** — 28 assertions:
  - base64url decode of RFC 4648 §10 vectors (`"Zm9v"` → `"foo"`, `"Zm9vYmFy"` → `"foobar"`)
  - JWT decode of the RFC 7515 §3.5 example header
  - URL-safe alphabet (`-` and `_` accepted)
  - **Canonical jwt.io HS256 token** (`SflKxwRJ...` with secret `your-256-bit-secret`) verifies — the standard reference vector for HS256 implementations
  - Wrong secret rejected; tampered signature rejected; tampered payload rejected
  - 2-segment / no-dot / null-token / null-secret / zero-length-secret all rejected
  - `alg=none` downgrade rejected
  - `JwtSecret` handle accessors, validator adapter returns claims on success / 0 on bad secret / 0 on null handle
  - Fn-pointer addressability for plugging into `auth_bearer_check`
- **586 total** (was 558). Breakdown: `bote.tcyr` 394, `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 24, `bote_host.tcyr` 67, `bote_auth.tcyr` 38, `bote_sandbox.tcyr` 13, `bote_jwt.tcyr` 28.

### Verified (cyrius 4.7.1)
- All seven test files green.
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 2.1.
- `cyrlint src/jwt.cyr tests/bote_jwt.tcyr` → 0 warnings.
- `./bote` reports `"version":"2.2.0"`.

### Proposed for cyrius 4.8.0
- **`base64url_encode` / `base64url_decode` in `lib/base64.cyr`** — see `docs/proposals/cyrius-stdlib-base64url.md`. Ship-ready reference impl + RFC 4648 §10 test vectors. Bote 2.2.x will lift its inline `jwt_b64u_decode` to the stdlib call once the cyrius agent folds it in (drops ~50 LOC + a few fns from bote's compile unit — meaningful for the long-running cyrius identifier-table cap pressure).

### Roadmap
- **OAuth 2.1 / PKCE-S256** still ahead — JWT HS256 is the verifier substrate; PKCE helpers are a future ship.
- **JWT RS256 / ES256** waits on sigil's RSA / ECDSA primitives.

## [2.1.0] — 2026-04-14 — Pluggable sandbox runner (kavach 3.0 compatible)

Lands the sandbox abstraction the 2.0 roadmap deferred. **`kavach` is at
3.0** (multi-backend: NOOP / process / OCI / gvisor / firecracker / SEV)
— the previous "waits on kavach v2" CHANGELOG line was stale. Bote now
ships the abstract `SandboxRunner` surface; consumers wire kavach (or
any other sandbox) behind a single fn-pointer + ctx, the same adapter
pattern used for `AuditSink` (libro) and `EventSink` (majra).

### Added
- **`src/sandbox.cyr`** (~50 LOC, no deps):
  - `sandbox_runner_new(run_fp, ctx)` — 16-byte handle.
  - `sandbox_run(s, command, timeout_ms)` — null-safe; returns the runner's JSON result or an error envelope. Suggested result shape matches kavach's `ExecResult`: `{"exit_code":N,"stdout":"...","stderr":"...","duration_ms":N,"timed_out":0|1}`.
  - `sandbox_runner_noop` + `sandbox_runner_noop_new()` — built-in adapter that echoes the command back as stdout (exit 0). Useful for tests and for environments where sandboxing isn't required.
- Validator signature: `fn run(ctx, command_cstr, timeout_ms) → result_cstr`. Tool authors call `sandbox_run(s, command, timeout)` from inside their handler body when they need to invoke an external process under isolation.

### Why a fn-pointer adapter, not a `[deps.kavach]` direct link
Kavach 3.0 is a substantial dependency (33 modules / ~7K LOC). Pulling it directly into bote's compile unit would tip the cyrius 4.7.1 identifier-table cap that's already constraining bote (see `docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`). The adapter pattern keeps bote independent — consumers that need kavach link it in their own `cyrius.toml` and write a 5-line `kavach_run_adapter(ctx, cmd, timeout) → ExecResult-as-JSON` function. Consumers that prefer a different sandbox (or none at all) drop in a different fn-pointer.

### Tests
- **New test file** — `tests/bote_sandbox.tcyr` (13 assertions). Covers runner shape (alloc + accessors), `sandbox_run` dispatching to a configured fp, null-safety on a null runner, the noop adapter (echo + JSON escaping), and fn-pointer addressability.
- **558 total** (was 545). Breakdown: `bote.tcyr` 394, `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 24, `bote_host.tcyr` 67, `bote_auth.tcyr` 38, `bote_sandbox.tcyr` 13.

### Verified (cyrius 4.7.1)
- All six test files green.
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 2.0.
- `cyrlint src/sandbox.cyr tests/bote_sandbox.tcyr` → 0 warnings.
- `./bote` reports `"version":"2.1.0"`.

### Roadmap note
- The earlier "kavach sandbox waits on kavach v2 hardening" item is **closed** — kavach 3.0 is shipped and bote 2.1.0 provides the integration substrate. Consumer-side adapter (~5 lines wrapping `noop_exec` / `process_exec` / `oci_exec` / etc.) is left to the consuming application — bote stays kavach-agnostic.

## [2.0.0] — 2026-04-14 — Stable: handler-claims ABI + carry-forward of all 1.9.x security work

The 1.x line ports bote from Rust to Cyrius and then iterates feature-by-feature; the 2.0 ship is the **first stable line** with a single deliberate ABI break (handler signature) so the auth → handler claims pipeline can land cleanly in 2.x without another major bump.

### Breaking
- **Handler signature**: `fn h(args_cstr) → result_cstr` → **`fn h(args_cstr, claims) → result_cstr`**. The new `claims` argument is opaque; in 2.0 it's always `0` (transports don't yet plumb the validator's return value down through `codec_process_message → dispatcher_dispatch → fncall2`). 2.x patch releases will populate it. Existing handlers must update their signature even if they ignore the second arg — `fn echo(args)` becomes `fn echo(args, claims)`. All bundled handlers (`bote_echo`, the five `libro_*` tools, all test/fuzz handlers) updated in this release.
- **Migration cost**: a single argument added to every tool handler the consumer registers. ~5 minutes per app.

### Carries forward (cumulative across 1.9.x — see per-release entries below)
- **Bearer-token middleware** (1.9.0) — fn-pointer + ctx validator, opt-in per HTTP-family transport, RFC 6750 401 with `WWW-Authenticate`.
- **Constant-time bearer compare + HTTP smuggling guard + batch cap + jsonx depth cap + `/dev/urandom`-or-fail** (1.9.4 audit batch A).
- **SSRF rewrite** — integer/octal/hex-IPv4 bypasses + IPv4-mapped-IPv6 + dot-consume verification (1.9.5 audit batch B). All 3 audit Criticals closed.
- **413 + bridge CORS oracle + Unix socket 0600** (1.9.6 audit polish).
- **`content_with_annotations`** — typed-block `audience` + `priority` annotations (1.9.6).

### Verified (cyrius 4.7.1)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_auth.tcyr` 38 = **545 total**.
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.9.x.
- `cyrlint src/*.cyr` → **0 warnings** across all sources.
- `cyrius fuzz fuzz/*.fcyr` → **4 passed, 0 failed**.
- `./bote` reports `"version":"2.0.0"`.

### Known carry-forward (deferred past 2.0)
- **Slowloris recv timeout** (audit H5) — needs a `sock_set_recv_timeout` helper in cyrius `lib/net.cyr` that doesn't exist. Workaround: deploy behind nginx/caddy.
- **WS handshake key-length validation** (audit M4) — lives in stdlib `lib/ws_server.cyr`; tracked upstream.
- **DNS resolution for SSRF hostname classification** — needs a cyrius `getaddrinfo` stub. Production callers should pair with a network-policy egress block.
- **Bridge optional protocol-version gate** (audit M5) — tried for 1.9.6, tipped the cyrius 4.7.1 cap; revisit when 4.8.0 raises room.
- **`libro_tools` default registration** — turned opt-in via `BOTE_LIBRO=1` env var in 1.9.4 to free identifier-table headroom; restore default-on when 4.8.0 lands.
- **Threaded streaming dispatch / WS arena allocator / `kavach` sandbox** — wait on cyrius primitives.
- **OAuth 2.1 / PKCE / JWT verifier** — bearer substrate is the hook; flows + verifier are the next net-new feature work.
- **Claims propagation through transports** — handler ABI is in place (this release); auth → dispatch plumbing follows in 2.1.
- **v1.2.1 libro-growth heisenbug** — unchanged.

### Carried-forward release map (1.x → 2.0)

| Tag | Headline |
|---|---|
| 1.0.0 | Cyrius port — protocol core, registry, dispatch, schema, codec, sessions, four transports |
| 1.1.0 | AuditSink + EventSink fn-ptr+ctx adapters |
| 1.2.0 | LibroAudit + MajraEvents adapters via `[deps.libro]` + `[deps.majra]` |
| 1.3.0 | Adopt cyrius 4.5.0 stdlib `lib/http_server.cyr` |
| 1.4.0 | Streamable HTTP transport (MCP 2025-11-25) |
| 1.5.0 | WebSocket transport (RFC 6455) on stdlib `lib/ws_server.cyr` |
| 1.5.1 | P(-1) hardening — HTTP body clamp + null-guard sweep |
| 1.6.0 | `libro_tools` — 5 built-in MCP audit tools |
| 1.7.0 | Typed MCP content blocks |
| 1.8.0 | `HostRegistry` + IPv4 `ssrf_check` |
| 1.8.1 | Bump cyrius 4.6.2 |
| 1.9.0 | Bearer-token middleware (RFC 6750) |
| 1.9.1 | IPv6 SSRF + binary-blob resource + env-driven CLI bearer |
| 1.9.2 | Bump cyrius 4.7.0 |
| 1.9.3 | Bump cyrius 4.7.1 + 2.0-prep doc sweep |
| 1.9.4 | Security batch A — 5 audit findings |
| 1.9.5 | Security batch B — 3 SSRF criticals |
| 1.9.6 | Final polish — 413, CORS oracle, Unix 0600, annotations |
| **2.0.0** | **Handler-claims ABI** + the 1.9.x carry-forward |

---

## [1.9.6] — 2026-04-14 — Final pre-2.0 polish: contained audit items + annotations

Closes the audit items that didn't need a cyrius stdlib helper, plus
re-lands the `content_with_annotations` work that was reverted from
1.9.1 / 1.9.4 for cap-headroom reasons. Last 1.9.x patch — next stop is
**2.0.0** (claims propagation through handler signature).

### Security
- **M1 — 413 Payload Too Large on oversized declared bodies.** `transport_http`, `transport_streamable`, and `bridge` now reject any request whose `Content-Length` declares more than 60 KB (60 × 1024 = 61440) — well under the 64 KB recv buffer with room for headers. Previous behaviour silently truncated and treated as a malformed 400; new behaviour returns the spec-correct status. Live smoke: 100 KB POST → 413 ✓.
- **M3 — bridge CORS oracle.** `_bridge_cors_origin` previously returned `vec_get(allowed_origins, 0)` (the first allowed origin) on a miss — leaks the allowed-origin list to any cross-origin requester. New behaviour returns the literal string `"null"` (a spec-compliant value; browser will block the response either way). Existing test updated.
- **L1 — Unix socket file mode.** `transport_unix_run` now `chmod(path, 0600)` post-bind so only the owning UID can connect. Previous behaviour inherited the process umask (typically 0022 → mode 0755) and let any local user dial the socket. Live smoke: socket file shows `srw-------` ✓.

### Added
- **`content_with_annotations(block, audience, priority)`** — re-landed from the 1.9.1 / 1.9.4 deferral. Splices an `annotations` field into any pre-built block cstr right before the trailing `}`. Audience is a vec of cstr (e.g. `"user"` / `"assistant"`); priority is an i64 in [0, 100] (`-1` = unset). Either argument may be null/-1; if both are absent the input is returned unchanged. **6 new test assertions** including no-op pass-through, audience-only, priority-only, both, image-block annotation, and null-block guard.

### Audit findings still open after 1.9.6
- **High — Slowloris recv timeout** (H5). Needs a `sock_set_recv_timeout` helper in `lib/net.cyr`. Workaround: deploy behind nginx/caddy that absorbs slow connections.
- **Medium — bridge optional protocol-version gate** (M5). Was attempted in 1.9.6 but added 2 new bridge_config_* fns that tipped the test compile unit past the cyrius 4.7.1 cap. Bridge is local-/single-app-scoped in practice; the bearer-token gate already covers production deployments. Will revisit when cyrius 4.8.0 raises the cap.
- **Medium — WS `Sec-WebSocket-Key` length validation** (M4). Lives in stdlib `lib/ws_server.cyr`; bote can't patch without forking. Tracked upstream.

### Verified (cyrius 4.7.1)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_auth.tcyr` 38 = **545 total** (was 539 at 1.9.5; +6 annotation assertions).
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.9.5.
- `cyrlint src/*.cyr` → 0 warnings across all sources.
- Live HTTP smoke: normal POST 200, 100 KB POST 413; Unix socket created with mode 0600.
- `./bote` reports `"version":"1.9.6"`.

### Up next: 2.0.0
- **Claims propagation through handler signature.** Handler currently `fn(args_cstr) → result_cstr`; 2.0 threads the validator's claims through dispatch so handlers can authorize per-tool. ABI break (warrants the major bump).
- **Final P(-1) hardening sweep.**
- **Restore `libro_tools` default registration** (was made opt-in in 1.9.4 for cap headroom; cyrius 4.8.0 should give us room).

## [1.9.5] — 2026-04-14 — Security batch B: SSRF rewrite (3 critical bypasses)

Closes the three Critical findings from the 2026-04-14 audit
(`docs/audit/2026-04-14.md`). All three are SSRF bypasses that landed
on cloud-metadata / loopback endpoints despite the 1.8.0+ blocklist.

### Security
- **C1 — integer-form IPv4 bypass.** `http://2130706433/` (the integer
  encoding of 127.0.0.1) was reaching the hostname classifier as a
  0-dot host and passing as `SSRF_OK` because the blocklist only knew
  literal `localhost` / `metadata`. **Fix**: hostname classifier now
  rejects all-`[0-9.]` hosts as `SSRF_PARSE`. Also catches short-form
  IPv4 (`127.1`, `127.1.1`).
- **C2 — octal IPv4 bypass.** `_ssrf_parse_octet` accepted leading-zero
  multi-digit forms: `0177` parsed as decimal 177 (apparently public)
  while glibc's `inet_aton` interprets the same bytes as octal 127 →
  loopback. **Fix**: octet parser now rejects digits>1 starting with
  zero (`00`, `01`, `0177`, `010` all invalid; `0`, `127`, `255`
  still valid).
- **C3 — IPv4-mapped IPv6 bypass.** `http://[::ffff:127.0.0.1]/` and
  `http://[::ffff:7f00:1]/` were classified by the IPv6 prefix-blocklist
  which only matched `::1`, `::`, `fe80:`, `fc/fd`, `ff` exact —
  `::ffff:` v4-mapped fell through as `SSRF_OK`. Also: the IPv4
  classifier's `_ssrf_parse_octet` didn't verify it consumed the full
  byte range up to the next dot, so `64:ff9b::1.2.3.4` (NAT64 form)
  parsed as `64.2.3.4` (public) and never reached the IPv6 path.
  **Fix**: IPv6 classifier now blocks `::ffff:`, `::*.*.*.*` (v4-compat),
  `64:ff9b:` (NAT64 well-known) outright; IPv4 classifier requires each
  octet parse to consume exactly the dot-to-dot range.

### Tests
- **11 new host assertions** for the bypasses: integer IPv4, octal IPv4
  (`0177.0.0.1`, `0010.0.0.1`), short-form IPv4 (`127.1`),
  `::ffff:127.0.0.1`, `::ffff:7f00:1`, `::127.0.0.1`, NAT64
  `64:ff9b::1.2.3.4`, plus regression checks for canonical decimal
  IPv4 still passing (`10.0.0.1` → PRIVATE, `1.2.3.4` → OK).
- **539 total** (was 528). Breakdown: `bote.tcyr` 394,
  `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 18,
  `bote_host.tcyr` 67 (was 56), `bote_auth.tcyr` 38.

### Audit report
- **`docs/audit/2026-04-14.md`** — full findings list (3 critical,
  5 high, 5 medium, 1 informational), audit-driven release map, and
  follow-up notes for items deferred to 2.0.

### Verified (cyrius 4.7.1)
- All five test files green (539 total).
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.9.4.
- `cyrlint src/host.cyr tests/bote_host.tcyr` → 0 warnings.
- `./bote` reports `"version":"1.9.5"`.

### Audit findings still open (deferred to 2.0)
- **High — Slowloris** (recv timeout — needs `sock_set_recv_timeout` stdlib helper)
- **Medium — bridge CORS oracle**, **WS handshake key-length validation** (upstream stdlib), **bridge protocol-version gate**, **413 cap on oversized requests**
- **Informational — Unix socket default umask** (chmod 0600 missing)

See `docs/audit/2026-04-14.md` for the full list and audit-driven
release map.

## [1.9.4] — 2026-04-14 — Security batch A (audit-driven)

First slice of the 2.0-prep security pass. Closes 4 of the 5 audit
findings in batch A; the slowloris recv-timeout fix needs a
`sock_set_recv_timeout` stdlib helper that doesn't exist yet, so it's
deferred to 1.9.5 / 2.0.

### Security
- **HTTP request smuggling guard** (RFC 9112 §6 / CVE-2019-18276 family). All four HTTP-family transports (`http` / `bridge` / `streamable` / `ws`) now reject any inbound request that carries a `Transfer-Encoding` header — bote doesn't dechunk, and rejecting any TE eliminates the CL.TE / TE.CL ambiguity. Inlined as a 3-line guard in each handler (rather than a shared helper) to stay under cyrius 4.7.1's identifier-table headroom.
- **Constant-time bearer-token comparison** (`auth_validator_allowlist`). The previous `streq`-based compare leaked per-byte timing, letting a network attacker byte-by-byte-guess a token against the allowlist. New implementation: lengths are still compared first (token length leaks; unavoidable), but the byte-loop XOR-accumulates with no early exit and the outer vec walk doesn't short-circuit on match — defeats both the per-byte oracle and the position-of-match oracle. Both helpers are inlined into `auth_validator_allowlist` itself (rather than a separate `_auth_ct_eq` fn) for the same identifier-table reason.
- **Batch-size cap** (`src/codec.cyr::codec_process_message`). Hardcoded `n > 100` check on JSON-RPC array batches — defends against a 1 MiB body of `[{},{},...]` (~300k elements) that would otherwise spin allocating per-element responses until OOM. The literal `100` is inlined to avoid a top-level `var` symbol; 100 is generous (real clients batch single-digit calls).
- **JSON nesting depth cap** (`src/jsonx.cyr::_jx_skip_struct`). Hardcoded `depth > 64` returns end-of-buffer rather than walking adversarial deep structures. The 64 KB inbound HTTP buffer already caps the absolute worst case, but the explicit guard keeps us safe if anyone raises the buffer.
- **`/dev/urandom`-or-fail** (`src/session.cyr::_gen_session_id`). The previous fallback used `clock_now_ns()` (predictable to any timing observer) when `/dev/urandom` failed to open. New behaviour: refuse to mint a session ID — write a fatal error and `SYS_EXIT(90)`. Also loops the read until 16 bytes are received (was a single `syscall(SYS_READ, ...)` that could short-read and leave uninitialized bytes in the SID material).

### Changed
- **`libro_tools` no longer registered by default at startup.** Made opt-in via `BOTE_LIBRO=1` env var to free identifier-table headroom for the security inlines above. Consumers who want the five `libro_*` MCP tools just set `BOTE_LIBRO=1` in their environment, or include `src/libro_tools.cyr` + call `libro_tools_init` / `libro_tools_register` from their own `main.cyr`. **This is a behaviour regression** — minor for most consumers (the tools were registered against an empty chain anyway). Will revert to default-on when cyrius lifts the cap further (4.8.0+).

### Tests
- **9 new auth assertions** for the constant-time compare: first-byte-differ, last-byte-differ, shorter, longer, position-independence across multi-entry allowlists. **528 total** (was 519). Breakdown: `bote.tcyr` 394, `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 18, `bote_host.tcyr` 56, `bote_auth.tcyr` 38.
- Live HTTP smoke: normal POST → 200 ✓; POST with `Transfer-Encoding: chunked` → 400 ✓; 150-element batch → `-32600` "batch too large" ✓.

### Verified (cyrius 4.7.1)
- All five test files green (528 total).
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.9.3.
- `cyrlint src/*.cyr` → 0 warnings.
- `./bote` reports `"version":"1.9.4"`.

### Audit findings still open (deferred to 1.9.5 / 2.0)
- **Critical (3) — SSRF bypasses**: integer/decimal IPv4 (`http://2130706433/`), octal/hex IPv4 (`http://0177.0.0.1/`), IPv4-mapped IPv6 (`http://[::ffff:127.0.0.1]/`). Need a coherent rewrite of the host parser; queued for 1.9.5.
- **High — Slowloris** (single-byte-then-pause holds the accept loop). Needs a `sock_set_recv_timeout` stdlib helper.
- **Medium — bridge CORS oracle** (echoes `allowed_origins[0]` on miss), **WS handshake doesn't validate `Sec-WebSocket-Key` length**, **bridge skips protocol/session checks**.
- **Informational — Unix socket created with default umask perms** (chmod 0600 not set).

See `docs/development/roadmap.md` for the 1.9.5 / 2.0 plan.

## [1.9.3] — 2026-04-14 — Bump pin to cyrius 4.7.1 + 2.0-prep doc sweep

Toolchain bump + a comprehensive 2.0-prep documentation pass. No
behavioral source changes; per-module test-file layout stays because
even cyrius 4.7.1's `BUILD_METHOD_NAME` scratch-corruption fix doesn't
cover bote's specific overflow path.

### Changed
- **cyrius pin** `4.7.0` → `4.7.1` (`cyrius.toml` + `.cyrius-toolchain`).
- `src/dispatch.cyr` — `_bote_server_version` → `"1.9.3"`.

### Documentation
- **`README.md`** rewritten for current state — six transports, bearer auth, all built-in tools, full module list, 519 tests, env-driven CLI auth quickstart.
- **`docs/architecture/overview.md`** rewritten — ASCII diagram updated for six transports + auth/content/host outbound utilities + sinks + libro_tools surface, full module listing, adapter pattern documented.
- **`docs/development/roadmap.md`** rewritten — shipped-per-release table for 1.0.0 → 1.9.2, explicit "must-have for 2.0" / "nice-to-have for 2.0" / "deferred past 2.0" sections, cyrius-language-dependency status all marked ✅ for items resolved.
- **`docs/spec-compliance.md`** rewritten — every compliance category extended for 1.4.0+ work (streamable HTTP, WS, content blocks, SSRF, bearer middleware, host registry, env-driven auth, `libro_tools`). Adds a content-block subtable and a host/SSRF subtable.
- **`docs/benchmarks-rust-v-cyrius.md`** rewritten — Rust v0.92.0 final history-log entry (2026-04-03) vs Cyrius 1.9.2 / cyrius 4.7.0 numbers side-by-side. New "Net call" + "Where Cyrius wins decisively" framing.
- **`docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`** updated — 4.7.1 status header documents that bote's case is still uncovered (~1339 fns under the 4096 cap, so the bug is elsewhere — possibly identifier-bytes or a different scratch path).

### What we got from 4.7.1
- Function-table cap raised 2048 → 4096.
- `BUILD_METHOD_NAME` scratch corruption fix — directly addresses the misleading-error class we reported.

### What 4.7.1 still doesn't cover (for bote)
- `tests/bote.tcyr` + `lib/ws_server.cyr` still trips `lib/assert.cyr:3: expected '=', got string`. Re-verified with freshly-bootstrapped `cc3 4.7.1`. Per-module test layout stays.

### Verified (cyrius 4.7.1)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 18 / `bote_host.tcyr` 56 / `bote_auth.tcyr` 29 = **519 total** (unchanged).
- `cyrius build src/main.cyr bote` → OK; `./bote` reports `"version":"1.9.3"`.
- `cyrlint src/*.cyr` → 0 warnings.

### Up next (2.0-prep, per the audit)
1. **1.9.4** — security batch A: HTTP Transfer-Encoding rejection + recv timeouts (smuggling + slowloris), constant-time bearer compare, batch-size cap, jsonx depth cap, `/dev/urandom`-or-fail.
2. **1.9.5** — SSRF rewrite: canonical-IPv4-only parser (rejects octal/hex/integer/short-form), full IPv6 16-byte classifier with `::ffff:` v4-mapped, optional getaddrinfo single-shot pre-classification.
3. **2.0.0** — `content_with_annotations`, claims propagation through handler signature, final P(-1) sweep, tag.

### Carried forward
- Identifier-buffer / scratch-corruption — bote-specific case still uncovered in 4.7.1; tracked in `docs/bugs/`.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.9.2] — 2026-04-14 — Bump pin to cyrius 4.7.0

Toolchain bump only. No source changes — bote doesn't use 4.7.0's
headline feature (`shared;` / `.so` output) and the identifier-buffer
ceiling that's been pinning the test split + the deferred annotations
work isn't lifted until 4.7.1 (in flight upstream).

### Changed
- **cyrius pin** `4.6.2` → `4.7.0` (`cyrius.toml` + `.cyrius-toolchain`).
- `src/dispatch.cyr` — `_bote_server_version` → `"1.9.2"`.

### What we got from 4.7.0
- Real `dlopen`-able `.so` end-to-end (`shared;` mode) — not used by bote today, will matter when we factor a tool out as a loadable module.
- `DT_INIT` runs top-level initializers on dlopen, PIC-safe addressing in shared mode, full `.dynamic`/`.dynsym`/`.hash` emission.
- Shared-mode DCE fix (parallel to the 4.6.0-beta2 object-mode fix).

### What we're waiting for in 4.7.1
- Identifier-buffer ceiling raise. Bote currently sits one feature past it: every attempt to add `content_with_annotations` (deferred from 1.9.1) trips the same misleading `lib/assert.cyr:3: expected '=', got string` cascade. Will land + collapse the per-module test-file split when 4.7.1 ships.

### Verified (cyrius 4.7.0)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 18 / `bote_host.tcyr` 56 / `bote_auth.tcyr` 29 = **519 total** (unchanged).
- `cyrius build src/main.cyr bote` → OK; `./bote` reports `"version":"1.9.2"`.
- `cyrlint src/*.cyr` → 0 warnings.

### Carried forward
- cyrius identifier-buffer ceiling — fix in flight for 4.7.1.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.9.1] — 2026-04-14 — IPv6 SSRF + binary-blob resource + env-driven bearer auth

Closes three deferred items from earlier releases. A fourth (block annotations) was tried and reverted — it tipped the cyrius 4.6.2 identifier-buffer ceiling. Will land in 2.0 / when 4.7.0 frees up headroom.

### Added
- **IPv6 SSRF blocklist** (`src/host.cyr`):
  - `_ssrf_classify_ipv6` covers `::1` (loopback), `::` (unspec), `fe80::/10` (link-local), `fc00::/7` ULA private (`fc` and `fd` prefixes), `ff00::/8` (multicast)
  - `_ssrf_extract_host` now parses bracket form (`http://[::1]:8080/...`); malformed bracket → `SSRF_PARSE`
  - `ssrf_check` routes hosts containing `:` (after IPv4 fallback) to the IPv6 classifier
  - 9 new test assertions (`tests/bote_host.tcyr` 47 → 56)
- **Binary resource (`blob`) variant** (`src/content.cyr`):
  - `content_resource_blob(uri, mime, b64_data)` emits `{"type":"resource","resource":{"uri":"...","mimeType":"...","blob":"..."}}` — the spec-prescribed shape for non-UTF-8 resource bodies. Optional fields omitted from output when null, same as `content_resource`.
  - 3 new test assertions (`tests/bote_content.tcyr` 15 → 18)
- **CLI bearer-auth via `BOTE_BEARER_TOKENS` env var** (`src/main.cyr`):
  - `_split_csv_tokens(csv)` parses comma-separated values into a vec of cstrs
  - `_bote_bearer_from_env` reads `BOTE_BEARER_TOKENS`, builds the allowlist + validator, returns fp + ctx via out-params
  - All four HTTP-family transports (`http` / `bridge` / `streamable` / `ws`) auto-wire `auth_validator_allowlist` when the env var is set
  - Stdio + Unix-socket transports skip auth (they're local-only)
  - Backward compatible: env var unset → no behaviour change

### Verified (cyrius 4.6.2)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 18 / `bote_host.tcyr` 56 / `bote_auth.tcyr` 29 = **519 total** (was 507).
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.9.0 baseline.
- `cyrlint src/*.cyr` → **0 warnings** across all sources.
- Live HTTP smoke with `BOTE_BEARER_TOKENS=tok-a,tok-b`: `POST /mcp` with no header → 401, with `Authorization: Bearer wrong` → 401, with `Authorization: Bearer tok-a` → 200 + serverInfo `"version":"1.9.1"`. Empty/unset env var → unchanged 200 with no auth.

### Reverted from 1.9.1 scope
- **`content_with_annotations`** (audience + priority MCP annotations on any block). The pre-built-block-splice approach added enough symbols to push `src/main.cyr`'s compile unit past the cyrius 4.6.2 identifier-buffer ceiling. Reverted to keep the build green; will revisit when cyrius 4.7.0 lands.

### Carried forward
- cyrius 4.6.2 identifier-buffer ceiling — bote's full compile unit sits ~one feature away from the new cap. 4.7.0 expected to provide more room.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.9.0] — 2026-04-14 — Bearer-token middleware (RFC 6750)

First slice of the roadmap `auth` item. OAuth 2.1 + PKCE follow in
later releases; this one delivers the substrate they all sit on:
extract `Authorization: Bearer <token>` from a request, hand the token
to a caller-supplied validator function pointer, and emit a spec-
compliant 401 if the token is missing or rejected. **Opt-in** — a
transport with no validator configured behaves exactly as before.

### Added
- **`src/auth.cyr`** (~140 LOC, no AGNOS deps):
  - `auth_bearer_extract(buf, blen)` — case-insensitive `Bearer ` scheme parse, leading/trailing OWS handling, returns the alloc'd token cstr or 0.
  - `auth_bearer_check(cfd, buf, blen, validator_fp, validator_ctx)` — middleware entry. Returns 0 on pass / `HTTP_UNAUTHORIZED` on reject (response already on the wire). No-op when `validator_fp == 0`.
  - `auth_send_unauthorized(cfd, realm)` — 401 with `WWW-Authenticate: Bearer realm="..."`.
  - `auth_validator_allow_all(token, ctx)` — pass-anything validator (testing / dev only).
  - `auth_validator_allowlist(token, vec)` — vec membership check; rejects null/empty.
- Validator signature: `fn validator(token_cstr, ctx) → claims_ptr | 0`. Non-zero return is "valid"; the value is opaque to bote today and will be threaded through to handlers when request-scoped context lands. Returning 0 means "reject".

### Changed
- All four HTTP-family transport configs now have `bearer_validator` + `bearer_ctx` slots **at the end** of the struct (existing offsets preserved):
  - **`HttpConfig`** 56 → 72 bytes (`+56` validator, `+64` ctx)
  - **`BridgeConfig`** 32 → 48 bytes (`+32`, `+40`)
  - **`StreamableConfig`** 64 → 80 bytes (`+64`, `+72`)
  - **`WsConfig`** 48 → 64 bytes (`+48`, `+56` — applied to the upgrade HTTP request only, not per-frame)
- Each gets a corresponding `X_config_with_bearer_validator(c, fp, ctx)` setter and `X_config_bearer_validator(c)` / `X_config_bearer_ctx(c)` accessors.
- Each transport's per-request handler now calls `auth_bearer_check` right after the Origin check and before the protocol-version check.
- `src/main.cyr` includes `src/auth.cyr` ahead of the transports that use it.

### Tests
- **New test file** — `tests/bote_auth.tcyr` (29 assertions). Covers:
  - Header parsing — exact match, three case variants of the scheme, leading/trailing whitespace handling
  - Rejections — no header / Basic scheme / `"Bearer"` with no space / empty token / whitespace-only token
  - `auth_validator_allow_all` accepts any non-empty token, rejects null/empty
  - `auth_validator_allowlist` accepts on match, rejects misses + null inputs
  - Middleware: no-validator → pass, valid token → pass, missing/wrong token → 401
  - Fn-pointer addressability (sanity)
- **Total assertions: 507** (was 478). Breakdown: `tests/bote.tcyr` 394, `tests/bote_libro_tools.tcyr` 22, `tests/bote_content.tcyr` 15, `tests/bote_host.tcyr` 47, `tests/bote_auth.tcyr` 29.

### Verified (cyrius 4.6.2)
- All five test files green (394 / 22 / 15 / 47 / 29).
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.8.1 (dispatch_* 1–3µs, jsonx_* 584–877ns, codec_* 789ns–6µs, validate_* 982ns–2µs).
- `cyrlint src/auth.cyr tests/bote_auth.tcyr src/transport_http.cyr src/transport_streamable.cyr src/transport_ws.cyr src/bridge.cyr` → **0 warnings**.
- Live HTTP smoke (`./bote http 18900` with no validator): `POST /mcp` returns 200 with serverInfo as before; auth machinery has zero overhead when not configured.
- `./bote` reports `"version":"1.9.0"`.

### Deferred to v1.9.x / v2.0
- **OAuth 2.1 + PKCE-S256.** Token *acquisition*, not just *validation*. Will reuse the same validator fn-ptr surface for verification.
- **Claims propagation to handlers.** Today a successful validate just returns 1; richer claims (subject, scopes) need request-scoped context plumbing into the handler signature, which is a bigger ABI change.
- **JWT verification helper.** Worth shipping when the first consumer needs it; keeps the validator surface lean for now.
- **CLI flag to enable bearer auth from the command line.** Programmatic callers can use `X_config_with_bearer_validator` today; CLI wiring follows once we pick a config-file format.

### Carried forward
- cyrius 4.6.2 function-table cap — fifth split test file added; addressed structurally.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.8.1] — 2026-04-14 — Bump to cyrius 4.6.2

Toolchain bump. No source changes beyond the pin + the test-file comment
that describes why the per-module test split is now the permanent
layout rather than a workaround.

### Changed
- **cyrius pin** `4.5.1` → `4.6.2` (`cyrius.toml` + `.cyrius-toolchain`).
- **`src/dispatch.cyr`** — `_bote_server_version` → `"1.8.1"`.
- **`tests/bote.tcyr`** comment updated to explain that per-module test files are the permanent layout. `lib/ws_server.cyr` stays out of the shared compile unit because bote's dep graph (`[deps.libro]` + `[deps.majra]` + `lib/sigil.cyr` alone at 354 fns + 15 stdlib modules + 15 bote sources) already lands near 4.6.2's 2048-fn function-table ceiling — ws_server's 16 fns tip it over.
- **`docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`** — added a 4.6.2 status header: identifier buffer *was* raised, the original repro now trips the function-table cap with a clean diagnostic, but the 4.6.1 diagnostic fix doesn't cover the specific overflow path bote hits.

### What we got from 4.6.1 / 4.6.2
- ✅ Identifier buffer raised (~60 KB headroom) — resolves the 1.5.0-era class of error.
- ✅ Clean `function table full (2048/2048)` diagnostic on the original repro.
- 🟡 Diagnostic fix doesn't cover every overflow path — bote still sees the misleading `lib/assert.cyr:3: expected '=', got string` when its full test unit + ws_server is compiled. Documented for the cyrius agent to take another pass.

### Verified (cyrius 4.6.2)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius test tests/bote_libro_tools.tcyr` → **22 passed, 0 failed**
- `cyrius test tests/bote_content.tcyr` → **15 passed, 0 failed**
- `cyrius test tests/bote_host.tcyr` → **47 passed, 0 failed** (478 total, unchanged)
- `cyrius build src/main.cyr bote` → OK; `./bote` reports `"version":"1.8.1"`.
- `cyrlint src/*.cyr` → 0 warnings.

### Carried forward
- cyrius 4.6.2 function-table cap limits the shared test compile unit — addressed structurally via per-module test files; waiting on either another cyrius-side raise or a compile-unit-level DCE that prunes unreferenced fns from counted totals.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.8.0] — 2026-04-14 — HostRegistry + SSRF guard

Closes out the `host` module started in 1.7.0. The registry gives
handlers a named list of external hosts bote is permitted to reach;
the SSRF guard rejects URLs targeting loopback / private / link-local
/ cloud-metadata endpoints *before* any network call goes out.

### Added
- **`src/host.cyr`** (~260 LOC, no AGNOS deps):
  - **`HostEntry`** (32 bytes): `name` / `url` / optional `headers` vec (alternating key/value cstrs) / optional `capabilities` vec. `host_entry_allows(entry, cap)` is an allowlist check — no-caps means "anything allowed" (fail-open for convenience); an explicit vec enforces the allowlist.
  - **`HostRegistry`** (16 bytes): `name → HostEntry` map backed by stdlib `hashmap`, plus a cached `count` that stays stable on replacement so callers trust it as an O(1) size hint.
  - `host_registry_new/add/get/has/count/names`.
- **`ssrf_check(url)`** — returns `SSRF_OK` (0) on pass, or a non-zero reason code:

  | Code | Meaning |
  |------|---------|
  | `SSRF_PARSE` | Malformed URL / empty host |
  | `SSRF_SCHEME` | Scheme isn't `http://` or `https://` |
  | `SSRF_LOOPBACK` | `127.0.0.0/8`, `localhost` |
  | `SSRF_LINK_LOCAL` | `169.254.0.0/16` (reserved for link-local generally) |
  | `SSRF_PRIVATE` | RFC 1918 (`10/8`, `172.16/12`, `192.168/16`) |
  | `SSRF_METADATA` | `169.254.169.254` (AWS/GCP/Azure), hostname `metadata.google.internal`, bare `metadata` |
  | `SSRF_UNSPEC` | `0.0.0.0/8` |
  | `SSRF_MULTICAST` | `224.0.0.0/4` |

  Case-insensitive on scheme + hostname. Strips `user:pass@` userinfo before classifying. Parses dotted-decimal IPv4 literals directly (no DNS). Non-IP hostnames hit a conservative string blocklist — this is defense-in-depth, not the last line; production callers should pair with DNS-level controls.
- Convenience: `ssrf_is_safe(url)` returns `1`/`0` for call sites that only need a boolean.

### Tests
- **New test file** — `tests/bote_host.tcyr` (47 assertions). Covers:
  - `HostEntry` / `HostRegistry` shape, replace-doesn't-double-count semantics, capability allowlist behaviour including the no-caps fail-open default
  - SSRF pass cases (`api.github.com`, `1.1.1.1`, `8.8.8.8`, port + query, `user:pass@` userinfo)
  - Every blocklist code path with a representative IPv4 (inc. edge cases: `172.15` & `172.32` public, `172.16` & `172.31` private)
  - Hostname blocklist (`localhost`, case-insensitive, `metadata.google.internal`, bare `metadata`)
  - Non-http schemes rejected (`file://`, `gopher://`, `ftp://`)
  - Null / malformed input
- **Total assertions: 478** (was 431). Breakdown: `tests/bote.tcyr` 394, `tests/bote_libro_tools.tcyr` 22, `tests/bote_content.tcyr` 15, `tests/bote_host.tcyr` 47.

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius test tests/bote_libro_tools.tcyr` → **22 passed, 0 failed**
- `cyrius test tests/bote_content.tcyr` → **15 passed, 0 failed**
- `cyrius test tests/bote_host.tcyr` → **47 passed, 0 failed**
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.7.0 baseline.
- `cyrlint src/host.cyr tests/bote_host.tcyr` → **0 warnings**.
- `./bote` — `initialize` → `{"serverInfo":{"name":"bote","version":"1.8.0"}}`.

### Deferred
- **DNS resolution for hostname classification.** A hostname that resolves to a blocked IP isn't caught today — caller can feed `127.0.0.1.nip.io` and pass. Requires a DNS stub on cyrius that doesn't yet exist; queued for a later release. In the meantime, pair with a network policy that blocks egress to RFC 1918.
- **IPv6 literal classification** (`::1`, `fe80::/10`, `fc00::/7`). Skipped for 1.8.0 — IPv4 covers today's deployments; IPv6 blocklist is next slice.
- **Registry persistence / hot-reload.** Registry is built in-process from config; no file watch.

### Known (unchanged)
- cyrius 4.5.1 identifier-buffer cap — this release needed a *fourth* split test file (`bote_host.tcyr`). All four collapse back into `bote.tcyr` when cyrius 4.6.1 lifts the cap. See `docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`.
- v1.2.1 libro-growth heisenbug — unrelated to this release.

## [1.7.0] — 2026-04-14 — Typed content blocks (MCP 2025-11-25)

Handlers can now return **typed content** — `text`, `image`, `audio`,
`resource` (embedded), and `resource_link` (reference) — instead of
only plain-text tool results. First piece of the `src/host.cyr`
roadmap item; host registry + SSRF follow in a later release.

### Added
- **`src/content.cyr`** (~135 LOC). Constructors for every MCP block type:
  - `content_text(text)` — `{"type":"text","text":"..."}`
  - `content_image(b64_data, mime)` / `content_audio(b64_data, mime)` — binary payloads, base64 in-band
  - `content_resource(uri, mime, text)` — embedded resource; `mime` and `text` are optional and omitted from the emitted object when null
  - `content_resource_link(uri, name, mime)` — reference only; client fetches by URI
  - `content_array(blocks)` — `{"content":[...]}` envelope over a vec of pre-built block cstrs
  - `content_array_error(blocks)` — same, with `"isError":true` (MCP distinguishes tool-execution errors from protocol errors by this flag)
  - `content_single(block)` and `content_text_response(text)` — shorthand for the single-block case (no vec alloc)
- Every string argument is a cstr; JSON escaping happens at the boundary via `_json_emit_escaped` (reused from `src/dispatch.cyr`).

### Interop
- `src/bridge.cyr`'s existing `wrap_tool_result` already detects a ready-made `{"content":[...]}` envelope and passes it through untouched — verified by a new test (`wrap_tool_result passes through a content envelope` in `bote_content.tcyr`). Handlers can opt into typed blocks without any transport-layer changes.

### Tests
- **New test file** — `tests/bote_content.tcyr` (15 assertions). Split out of `tests/bote.tcyr` for the cyrius 4.5.1 parser identifier-buffer cap — same pattern as `bote_libro_tools.tcyr`. Both will collapse back into the main test file when cyrius 4.6.1 lifts the cap.
- Coverage: every constructor's exact JSON output (including optional-field omission), JSON escaping of quotes, null-text → empty-string fallback, mixed-type arrays, empty-array case, `isError` flag, and the pass-through interop with `wrap_tool_result`.
- **Total assertions: 431** (was 416). Breakdown: `tests/bote.tcyr` 394, `tests/bote_libro_tools.tcyr` 22, `tests/bote_content.tcyr` 15.

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius test tests/bote_libro_tools.tcyr` → **22 passed, 0 failed**
- `cyrius test tests/bote_content.tcyr` → **15 passed, 0 failed**
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.6.0 baseline (dispatch_* 1–3µs, jsonx_* 612–963ns, codec_* 785ns–6µs, validate_* 1–3µs).
- `cyrlint src/content.cyr tests/bote_content.tcyr` → **0 warnings**.
- `./bote` — `initialize` → `{"serverInfo":{"name":"bote","version":"1.7.0"}}`.

### Deferred to v1.8+
- **Annotations** (`audience`, `priority`) on content blocks — MCP spec optional; skipped to keep 1.7.0 focused.
- **Binary resource contents** (`blob`) — the current `content_resource` handles text; the blob variant needs a base64 decoder surface (or passes through pre-encoded input). Punt until a consumer needs it.

### Known (unchanged)
- cyrius 4.5.1 identifier-buffer cap — forced the third test file. Tracked for 4.6.1 per `docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`.
- v1.2.1 libro-growth heisenbug — unrelated to this release.

## [1.6.0] — 2026-04-14 — libro_tools (5 built-in MCP audit tools)

Lands the `libro_tools` module from the 1.3 roadmap: five MCP tools that
expose a libro audit chain through the normal `tools/call` JSON-RPC
surface. Any MCP client can now search, verify, export, prove, and
retain-manage a bote-hosted chain without learning libro's native API.

### Added
- **`src/libro_tools.cyr`** (~310 LOC) — five handlers + registration:
  - `libro_query` — filter by `source` / `agent_id` / `severity` / `min_severity` / `action` / `after` / `before`; paginate with `offset` + `limit`. Returns `{"ok":true,"total":N,"entries":[...]}`.
  - `libro_verify` — hash-link integrity check. Returns `{"ok":true}` or `{"ok":false,"code":N,"index":i,"message":"..."}`.
  - `libro_export` — every entry as a JSON array. Returns `{"ok":true,"count":N,"entries":[...]}`.
  - `libro_proof` — Merkle inclusion proof for the entry at `index`. Returns `{"ok":true,"index":i,"leaf_count":N,"root":"<hex>"}`. Path hashes are not yet emitted — a follow-up will pin the wire format and include them.
  - `libro_retention` — apply `keep_count` / `keep_duration` / `keep_after` / `pci_dss` / `hipaa` / `sox` policies. Returns `{"ok":true,"archived":N,"retained":M}`.
- `libro_tools_init(chain)` + `libro_tools_register(dispatcher)` wire-up. Every cstr→Str crossing at the libro boundary goes through `str_from()` (the v1.2.1 cstr/Str fix pattern).
- **`src/main.cyr`** — the built-in dispatcher now creates an empty chain at startup and registers the five tools by default, so MCP clients discover them via `tools/list` without any flags.

### Tests
- **New test file** — `tests/bote_libro_tools.tcyr` (22 assertions). Lives separately from `tests/bote.tcyr` because pulling libro_tools into the main test file trips the cyrius 4.5.1 parser identifier-buffer cap. Split-test is tracked to collapse back in 4.6.1 when the cap is lifted.
- Coverage: registration of all five tools in the dispatcher map, handler-fn-pointer addressability, empty-chain shape of each tool's response (`{`-prefixed JSON), required-arg validation (`libro_proof` without `index`, `libro_retention` without `policy`), policy name whitelist (unknown policy → `ok:false`), each preset policy (`pci_dss`, `keep_count`) returns successfully.
- **Total assertions: 416** (was 394). Breakdown: `tests/bote.tcyr` 394, `tests/bote_libro_tools.tcyr` 22.

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius test tests/bote_libro_tools.tcyr` → **22 passed, 0 failed**
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.5.1 baseline (dispatch_* 1–3µs, jsonx_* 588–925ns, codec_* 773ns–6µs, validate_* 970ns–2µs).
- `cyrlint src/libro_tools.cyr tests/bote_libro_tools.tcyr` → **0 warnings**.
- `./bote` (stdio transport) — `initialize` → `{"serverInfo":{"name":"bote","version":"1.6.0"}}`, `tools/list` returns `bote_echo` + 5 `libro_*` entries, all 5 `tools/call` paths return clean JSON on an empty chain (verified by hand).

### Known (unchanged)
- **v1.2.1 libro-growth heisenbug**: creating an empty chain at startup (`chain_new()`) is safe; growing via `chain_append` is where the heap-sensitivity shows up. `libro_tools` itself is correct — only reads chain state, doesn't append. Writes still go through `src/audit_libro.cyr`, which is where the heisenbug lives.
- **cyrius 4.5.1 identifier-buffer cap** (`docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`) — forced the second test file. Tracked for 4.6.1.

## [1.5.1] — 2026-04-14 — P(-1) scaffold hardening

First hardening pass since 1.5.0. No new features; audit-driven fixes to
defensive guards, a line-length lint cleanup across `src/`, and two new
test assertions. All 394 tests / 10 benches / 4 fuzz harnesses green.

### Security
- **HTTP body-length clamp** — `src/transport_http.cyr`, `src/transport_streamable.cyr`, and `src/bridge.cyr` each copy the request body with `memcpy(body, buf + bo, clen)` after reading `Content-Length`. If a lying `Content-Length` header declared more bytes than actually arrived on the wire, `memcpy` would read past the request buffer into adjacent memory. All three paths now clamp `clen = min(clen, n - bo)` before the copy. Tested by manual audit; integration coverage by the existing transport tests still passes.

### Fixed
- **`resumption_buffer_events_after` null/empty guard** — Accepting `last_event_id == 0` previously would have segfaulted on the first `streq`; accepting `""` would have silently scanned the whole buffer for no matches. The caller in `_strm_handle_get` already guards null, but the helper is now defensive in its own right (returns empty vec in both cases). **Two new test assertions** cover these paths (394 total, was 392).

### Changed
- **Line-length cleanup** (`cyrlint`-clean): `src/bridge.cyr:172` (CORS header), `src/dispatch.cyr:57` (tool-name validation error message), `src/stream.cyr:93` (progress notification JSON). No behavior change — just wrapped the offending literals across two `str_builder_add_cstr` calls so lines stay under 120 chars. All `src/*.cyr` files now report `0 warnings` from `cyrlint`.

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.5.0 baseline (dispatch_* 1–3µs, jsonx_* 580–864ns, codec_* 763ns–6µs, validate_* 976ns–2µs)
- `cyrius fuzz fuzz/*.fcyr` → **4 passed, 0 failed**
- `cyrlint src/*.cyr` → all **0 warnings**

### Audit findings deferred
Captured during this pass but not actioned in 1.5.1:
- **Bump-allocator leak on long-lived WS connections** — every inbound frame allocs a fresh payload buffer, and `codec_process_message` returns fresh alloc'd JSON. Short-lived HTTP requests don't notice; WebSocket connections that stay open for hours will accumulate. Proper fix needs either an arena-per-message lifetime or stdlib `fl_free` support. Tracked for v1.6.
- **Global state in `transport_streamable.cyr` (`_strm_event_ids`, `_strm_resumption`) and `transport_stdio.cyr` (`_stdio_buf`, `_stdio_buf_len`)** — safe today because all transports are single-connection-at-a-time per the v1.0 design, but will need mutex-guarding when streaming dispatch (v1.5+ per roadmap) lets a server handle concurrent sessions.

### Carried forward
- v1.2.1 libro live-integration heisenbug: unchanged.
- cyrius 4.5.1 identifier-buffer cap: unchanged (`docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`).

---

## [1.5.0] — 2026-04-14 — WebSocket transport (RFC 6455)

Adds a sixth MCP transport: **WebSocket**. Each TEXT frame is one JSON-RPC
2.0 message. Built on `lib/ws_server.cyr` which landed in **cyrius 4.5.1**
(see `docs/proposals/cyrius-stdlib-ws-server.md` for the design rationale)
— and on the existing `lib/http_server.cyr` for the HTTP/1.1 Upgrade
handshake. **~110 LOC** of MCP-specific wire-up on top of the stdlib,
versus ~400 LOC if hand-rolled.

### Added
- **`src/transport_ws.cyr`** (~110 LOC):
  - **`WsConfig`** (48 bytes) — path, addr, port, allowed_origins, require_protocol, dispatcher
  - **`_bote_ws_handler`** — invoked per connection by `http_server_run`. Enforces Origin + `MCP-Protocol-Version` middleware (same shape as `transport_http`), calls `ws_server_handshake` to upgrade in place, then loops reading TEXT frames and feeding each to `codec_process_message` (control frames — ping/pong/close — handled by stdlib transparently).
  - **`transport_ws_run(dispatcher, config)`** — defers to stdlib `http_server_run`.
- **CLI** — `./build/bote ws [port]` (default `8393`).
- **Proposal artifacts** under `docs/proposals/` (same workflow as the http_server proposal that became cyrius 4.5.0):
  - `cyrius-stdlib-ws-server.md` — design doc + RFC 6455 spec coverage table
  - `lib_ws_server.cyr` — reference implementation with inlined SHA-1
  - `lib_ws_server_example.cyr` — runnable echo server

### Changed
- **cyrius pin bumped to 4.5.1** (required for `lib/ws_server.cyr`).
- `src/main.cyr` dispatches on `ws` argv (default port 8393).

### Spec compliance (RFC 6455, delegated to stdlib)
- ✅ HTTP/1.1 Upgrade handshake, `Sec-WebSocket-Accept = base64(sha1(key + magic))`
- ✅ `Sec-WebSocket-Version: 13` enforced
- ✅ Server reads MASKED client frames, writes UNMASKED server frames
- ✅ Small / medium (16-bit) / large (64-bit) payload length encodings
- ✅ Text + Binary data frames
- ✅ Ping / Pong control frames (handled transparently by `ws_server_recv`)
- ✅ Close handshake with status code + optional reason
- 🟡 Per-message deflate (RFC 7692) — deferred to stdlib
- 🟡 Subprotocol negotiation (`Sec-WebSocket-Protocol`) — header read but not enforced

### Tests
- 10 new unit assertions (**392 total**, was 382):
  - `WsConfig` defaults + setters (path, addr, port, origins, require_protocol, dispatcher)
  - `_bote_ws_handler` fn-pointer addressability
  - Dispatcher wire-up on `transport_ws_run`
- Live handshake + frame round-trip stays with the stdlib `ws_server` conformance tests (avoids duplicating the protocol suite, and dodges the 4.5.1 parser input-buffer cap we hit when pulling the full `ws_server.cyr` into this file).

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **392 passed, 0 failed**
- `cyrius build` → `./bote` (with `ws` subcommand binds `127.0.0.1:8393` and returns 101 on `GET /mcp` with a valid `Sec-WebSocket-Key` — verified via local `wscat` / `curl` probe)
- `cyrius bench` → 10 hot paths unchanged
- `./build/bote` initialize handshake reports `"version":"1.5.0"`

### Carried forward
- v1.2.1 libro live-integration heisenbug: still present, still tracked.

### Known cyrius 4.5.1 artifact
- The parser's input-buffer cap is reached when `tests/bote.tcyr` also includes `lib/ws_server.cyr` directly. Worked around by keeping ws_server out of the test file (the handler's frame I/O is covered by the stdlib conformance tests anyway). Tracked upstream; a follow-up cyrius patch will lift the cap.

---

## [1.4.0] — 2026-04-14 — Streamable HTTP transport (MCP 2025-11-25)

Closes the **streamable HTTP** spec item from MCP 2025-11-25. Single endpoint
serves both `POST` (JSON-RPC request → response) and `GET` (open SSE stream
for server-initiated messages). Built on the stdlib `lib/http_server.cyr`
chunked primitives that shipped in cyrius 4.5.0.

### Added
- **`src/transport_streamable.cyr`** (~290 LOC). Modules:
  - **`EventIdGenerator`** — monotonic counter, emits `"evt-N"` strings
  - **`StreamEvent`** — `{id, event="message", data}` with SSE wire-format renderer (`stream_event_to_wire` → `id: ...\nevent: ...\ndata: ...\n\n`)
  - **`ResumptionBuffer`** — bounded ring of recent events; `events_after(last_id)` for `Last-Event-ID` replay
  - **`StreamableConfig`** (64 bytes) — path, addr, port, allowed_origins, require_protocol, session_store, retry_ms, dispatcher
  - **`transport_streamable_run(d, cfg)`** — defers to stdlib `http_server_run` with a single dispatch handler that routes POST → JSON-RPC, GET → SSE stream
- **CLI** — `./build/bote streamable [port]` (default `8392`).

### Spec compliance
- ✅ `POST <endpoint>` JSON-RPC dispatch (same shape as plain HTTP)
- ✅ `GET <endpoint>` SSE stream open with priming event
- ✅ `MCP-Protocol-Version` header **required** on every request (400 if absent — stricter than plain HTTP transport which makes it optional by default)
- ✅ `MCP-Session-Id` header validated when SessionStore is configured
- ✅ `Origin` allow-list (DNS rebinding protection)
- ✅ `Last-Event-ID` request header → replay buffered events on GET
- ✅ Server emits `id:`-tagged SSE events for resumption tracking
- ✅ `retry: <ms>\n\n` hint sent before stream close (default 5000ms, configurable)
- 🟡 Server-initiated event push on the GET stream: deferred — waits on streaming dispatch (v1.5+) to populate the resumption buffer with real events. The transport correctly opens the SSE stream and replays anything in the buffer; the buffer is just empty until something publishes to it.

### Tests
- 23 new unit assertions (382 total, was 359):
  - `EventIdGenerator` produces monotonic `evt-0`/`evt-1`/`evt-2`
  - `StreamEvent` accessors + SSE wire format (with and without data)
  - `ResumptionBuffer` push, eviction (oldest first when over capacity), `events_after` lookup (present and absent IDs)
  - `StreamableConfig` defaults + setters
  - `http_path_only` correctly strips query string for path matching

### Performance
Bench numbers unchanged. `cyrius bench` still shows 10 hot paths sub-10µs.

### Verified end-to-end
- `POST /mcp` with `MCP-Protocol-Version: 2025-11-25` → returns serverInfo
- `POST /mcp` without protocol header → `400 Bad Request`
- `POST /mcp tools/call` → standard JSON-RPC response
- `GET /mcp` with `Accept: text/event-stream` → opens SSE stream, sends primer (`id: evt-0\nevent: message\ndata: \n\n`), sends retry hint (`retry: 5000\n\n`), closes
- `Last-Event-ID: evt-7` (when buffer has events past evt-7) → replays them in order

### Carried forward
- v1.2.1 libro live-integration heisenbug: still present, still tracked.

### Verified (cyrius 4.5.0)
- `cyrius test` → **382 passed, 0 failed**
- `cyrius fuzz` → 4 passed, 0 failed
- `cyrius bench` → 10 hot paths unchanged
- `./build/bote` initialize handshake reports `"version":"1.4.0"`

---

## [1.3.0] — 2026-04-14 — Adopt stdlib `lib/http_server.cyr` (cyrius 4.5.0)

Cyrius **4.5.0** shipped `lib/http_server.cyr` — verbatim from the proposal
in `docs/proposals/cyrius-stdlib-http-server.md`. Bote drops 236 lines of
hand-rolled HTTP plumbing in favour of the shared stdlib.

### Changed
- **`cyrius.toml`** — added `"http_server"` to `[deps]` stdlib list. Cyrius pin → `4.5.0`.
- **`src/transport_http.cyr`** — was 404 LOC, now **150 LOC** (-63%). Dropped: `_http_find`, `_http_to_lower`, `_http_iceq`, `_http_next_nl`, `http_find_header`, `http_get_method`, `http_get_path`, `http_body_offset`, `http_content_length`, `_http_send_status`, `_http_send_json_200`, `_http_send_204`, plus the bind/listen/accept ceremony. All come from stdlib now. Kept: `HttpConfig` struct + accessors, `_http_check_origin / _protocol / _session` middleware, `_http_handle` request handler, `transport_http_run` (now a 5-line wrapper around `http_server_run`).
- **`src/bridge.cyr`** — was 280 LOC, now **170 LOC** (-39%). Dropped: `_bridge_handle_connection` (rewritten to handler-style), bind/listen/accept loop, response builders. Kept: `BridgeConfig` (added `dispatcher` slot), `wrap_tool_result` / `wrap_error_result` (MCP envelope contract, bote-specific), `_bridge_cors_*` headers, `bridge_process_message` (bridge-specific routing).
- **`HttpConfig`** gained a `+48 dispatcher` slot (was 48 → now 56 bytes) so it can carry the dispatcher into the stdlib `http_server_run` ctx pointer.
- **`BridgeConfig`** gained a `+24 dispatcher` slot (was 24 → now 32 bytes) for the same reason. New setter: `bridge_config_with_dispatcher(c, d)`.
- **Status codes** now use `HTTP_OK`, `HTTP_NOT_FOUND`, etc. constants from stdlib (was hardcoded integers).
- **Path matching** uses `http_path_only(path)` from stdlib so `/mcp?something` matches `/mcp` correctly. Same for bridge `/health` and `/`.

### Performance / size
- **bote ELF binary**: was 130 KB (1.2.1) → **127 KB** (1.3.0). The stdlib HTTP code is shared with any future cyrius project.
- Function count freed up from bote's compilation unit: ~28 fns (the entire HTTP plumbing layer) now lives in stdlib and counts once across all consumers.
- Hot-path benchmarks unchanged.

### Spec impact
- **Content-Length-aware request reading** now correct (stdlib `http_recv_request` reads until body is fully received). Previous bote behaviour did a single `sock_recv` and silently truncated requests larger than one TCP packet — fixed for free.
- **Unblocks v1.4.0 streamable HTTP** — stdlib provides `http_send_chunked_start` / `http_send_chunk` / `http_send_chunked_end` for SSE.

### Carried forward
- v1.2.1 known issue (live libro chain integration heisenbug in tests/bote.tcyr) **persists** despite freed function-count budget. Adapter remains correct in isolated probes; in-test live integration test is still shape-only. Suggests the heisenbug is heap-layout / global-init related, not function-count related — needs deeper cyrius investigation.

### Verified (cyrius 4.5.0)
- `cyrius test` → **359 passed, 0 failed**
- `cyrius fuzz` → 4 passed, 0 failed
- `cyrius bench` → 10 hot paths unchanged
- `./build/bote` initialize handshake reports `"version":"1.3.0"`
- HTTP and bridge transports both verified end-to-end with `curl`

### `docs/proposals/`
The proposal docs (`cyrius-stdlib-http-server.md`, `lib_http_server.cyr`,
`lib_http_server_example.cyr`) remain in the repo as the spec the lang-agent
implemented from. Useful reference for any future stdlib proposals coming
out of bote work.

---

## [1.2.1] — 2026-04-13 — Adapter init-dance docs + v1.2.0 patch hardening

### Fixed
- **`src/audit_libro.cyr::libro_audit_log`** wraps every cstr boundary value through `str_from` before passing to `chain_append` / `chain_append_with_agent`. libro expects `Str` (fat strings) for source / action / details / agent_id, not raw cstrs — the previous version passed cstrs directly which produced garbage `str_len` reads inside libro's hash function. Verified by isolated probe: `chain_append(c, SEV_INFO, str_from("bote"), str_from("tool.completed"), str_from("{}"))` correctly grows the chain to length 1.

### Documented
- **Init dance** for any binary that uses LibroAudit:
  ```
  alloc_init();   # bump allocator
  fl_init();      # freelist — libro entries
  ed25519_init(); # sigil signing constants
  ```
  Codified in the `audit_libro.cyr` header comment.

### Cyrius pin
- Bumped to **4.4.6** (`cyrius.toml`).

### Known issue (tracked for v1.3.0)
- Linking libro + majra + the full bote test corpus into one binary triggers a heap heisenbug: `libro_audit_log` enters an infinite loop (apparently re-entering `main`). The adapter itself is correct — verified by an isolated probe that calls `chain_append` directly with `str_from`-wrapped strings. Suspect: cumulative globals from the cross-product exceed an internal cyrius compilation-unit boundary and corrupt the bump-allocator's prologue. Workarounds explored: dropping `lib/patra.cyr` (saved 244 KB but didn't help), splitting includes (no effect), running the demo as a standalone binary (still loops). Filed as a tracking item; resolution likely needs the multi-file linker on cyrius's v4.5 roadmap.

### Verified (cyrius 4.4.6)
- `cyrius test` → 359 passed, 0 failed
- `cyrius fuzz` → 4 passed, 0 failed
- `cyrius bench` → 10 hot paths unchanged
- `./build/bote` initialize handshake reports `"version":"1.2.1"`

---

## [1.2.0] — 2026-04-13 — LibroAudit + MajraEvents adapters

Wires bote's AuditSink and EventSink (introduced in 1.1.0) to the **libro**
audit chain and **majra** pub/sub. First release with `[deps.<crate>]` git+tag
pinned dependencies.

### Added
- **`[deps.libro]`** — pinned to git tag `1.0.3` (`https://github.com/MacCracken/libro`, falls back to local `../libro`). 9 modules pulled in: `error / hasher / entry / verify / query / retention / chain / export / merkle`.
- **`[deps.majra]`** — pinned to git tag `2.2.0` (`https://github.com/MacCracken/majra`, falls back to local `../majra`). 6 modules pulled in: `error / counter / envelope / namespace / queue / pubsub`. Trimmed to only the modules `pubsub` actually exercises (skipped `metrics / ratelimit / heartbeat / fleet / dag / etc`).
- **`src/audit_libro.cyr`** — `LibroAudit` adapter (24 bytes: chain ptr, source cstr, agent_id cstr). `libro_audit_new(chain)`, `libro_audit_with_source(la, src)`, `libro_audit_with_agent_id(la, id)`, `libro_audit_log(ctx, event)`. Maps bote's ToolCallEvent to libro's `chain_append_with_agent` (or `chain_append` when no agent): `SEV_INFO` + `"tool.completed"` on success, `SEV_ERROR` + `"tool.failed"` on failure. **caller_id wins over the configured agent_id.**
- **`src/events_majra.cyr`** — thin `majra_events_publish(ctx, topic, payload)` that calls `pubsub_publish(ps, topic, payload)`. Wire-up:
  ```
  var ps = pubsub_new();
  var sink = event_sink_new(&majra_events_publish, ps);
  dispatcher_set_events(d, sink);
  ```
- 8 new unit assertions (359 total) covering adapter struct shapes, accessors, AuditSink fp wiring, EventSink fp wiring (incl. `ctx=0` no-op safety).

### Performance
Bench numbers unchanged from 1.1.0 — adapters are pass-through over already-measured sink-publish overhead.

### Deferred
Live `chain_append_with_agent` integration tests (and full pubsub deliver paths) require running libro/majra's full init dance (`alloc_init`, `fl_init`, `ed25519_init`, `patra_init`, etc). Those are exercised by libro's and majra's own test suites; bote's tests currently verify the **adapter shape** (struct + fp wiring). Live integration tests will land in **v1.2.1** once the init-call documentation is finalized.

`src/libro_tools.cyr` (5 built-in MCP audit tools) deferred to **v1.3.0**. Reason: the cyrius compiler hits an internal token-table boundary when the full bote + libro + majra + tool-handler-fns set is included in one compilation unit. v1.3.0 will either (a) split bote into multiple compilation units (multi-file linker, on cyrius's v4.5 roadmap) or (b) ship libro_tools as a separate `[deps.bote-libro-tools]` package.

### Verified (cyrius 4.4.4)
- 359 tests passed, 0 failed
- 4 fuzz harnesses passed
- 10 benchmarks unchanged
- `./build/bote` initialize handshake reports `"version":"1.2.0"`

---

## [1.1.0] — 2026-04-13 — AuditSink + EventSink + dispatcher wire-up

First minor bump on the cyrius lineage. Adds the audit and event-publishing
abstractions and wires them into the dispatcher. Adapters for libro / majra /
discovery-via-pubsub land in **v1.2.0**.

### Added
- **`src/events.cyr`** — `EventSink` (function-pointer + opaque ctx struct, the cyrius equivalent of the Rust trait), `event_sink_new` / `event_sink_publish` / `event_sink_noop`. Topic constants exported as functions: `TOPIC_TOOL_COMPLETED`, `TOPIC_TOOL_FAILED`, `TOPIC_TOOL_REGISTERED`, `TOPIC_TOOL_DEREGISTERED`, `TOPIC_TOOL_DEPRECATED`, `TOPIC_TOOL_ANNOUNCE`, `TOPIC_TOOL_DISCOVERED`, plus 3 sandbox topics for the v1.3 sandbox port.
- **`src/audit.cyr`** — `ToolCallEvent` (40 bytes: tool_name, duration_ms, success, error, caller_id), `tool_call_event_to_json` (matches Rust `serde_json` output, skips `error` / `caller_id` when 0), `AuditSink` struct + `audit_sink_log` / `audit_sink_noop`.
- **`Dispatcher` extended** to 40 bytes — new slots for `audit_sink` and `event_sink`. Setters: `dispatcher_set_audit(d, sink)` and `dispatcher_set_events(d, sink)`. Sinks default to 0 (no-op); pre-1.1 callers see no behavior change.
- **Dispatcher emits per-call audit + event hooks**:
  - `tools/call` success → `audit_sink_log` + publish to `bote/tool/completed`
  - `tools/call` failure (handler not in map) → `audit_sink_log` + publish to `bote/tool/failed`
  - `tools/call` on a deprecated tool → publish to `bote/tool/deprecated` *before* the call
  - `dispatcher_register_tool` → publish to `bote/tool/registered`
  - `dispatcher_deregister_tool` → publish to `bote/tool/deregistered`
  - All include a `{"tool_name":"..."}` payload (deprecated also includes `message`).
- **`caller_id`** now extracted from `tools/call` params (`jsonx_get_str(params, "caller_id")`) and threaded through to the audit event.
- **`src/discovery.cyr` migrated to `EventSink`** — `discovery_new(node_id, event_sink)` replaces the bare `publish_fp`. Uses `discovery_event_sink(d)` accessor; same callers, cleaner integration with the rest of the event surface.
- **50 new unit assertions** (351 total, was 301): topic constants, sink no-op safety, sink invocation, ToolCallEvent JSON round-trips (success / failure / minimal), full dispatcher wire-up (success+failure+initialize+list+register+dereg+deprecated paths), discovery via EventSink, "validate-stage error doesn't audit" parity check vs. Rust.

### Performance
Audit + event hooks add ~2µs to `dispatch_tools_call` (1µs → 3µs) and ~2µs to `codec_process_message` (4µs → 6µs) when sinks are wired. With `audit_sink_noop()` / `event_sink_noop()` (or unset, the default), the overhead is a single null-pointer check per emission site. Other benchmarks unchanged.

### Changed
- `discovery_new` signature: was `(node_id, publish_fp)` taking a bare `fn(topic, json)` pointer; now `(node_id, event_sink)` taking an EventSink. **Source-breaking** — but the only known caller was `tests/bote.tcyr`, and the new shape is what real callers (MajraEvents in v1.2) need anyway. v1.0 callers building a discovery service should switch to `event_sink_new(&publish_fn, ctx)`.

### Verification (cyrius 4.4.4)
- `cyrius test` → **351 passed, 0 failed**
- `cyrius fuzz` → 4 passed, 0 failed
- `cyrius bench` → 10 hot paths, sinks-noop overhead is 1 conditional branch per emission
- `./build/bote` initialize handshake reports `"version":"1.1.0"`

### Deferred to v1.2.0
- `src/audit_libro.cyr` — LibroAudit adapter (calls `chain_append_with_agent` on libro's hash chain). Needs `[deps.libro] path = "../libro"` in `cyrius.toml`.
- `src/events_majra.cyr` — MajraEvents adapter (calls `pubsub_publish`). Needs `[deps.majra] path = "../majra"` in `cyrius.toml`.
- `src/libro_tools.cyr` — 5 built-in MCP tools (`libro_query`, `libro_verify`, `libro_export`, `libro_proof`, `libro_retention`). Depends on audit + libro.

---

## [1.0.1] — 2026-04-13 — Retire rust-old/, trim spec-compliance, bench comparison

### Removed
- **`rust-old/`** — the Rust source archive that came in via `cyrius port` is gone. Its purpose (porting reference) is fulfilled. The last Rust state remains accessible at git tag `0.92.0`.
- `.gitignore` rules for `rust-old/target/` and `rust-old/**/target/` (no longer needed).

### Added
- **`docs/benchmarks-rust-v-cyrius.md`** — side-by-side performance comparison. Source / binary / dep counts, per-op timings (Rust v0.92.0 vs Cyrius v1.0.1), the structural reasons Cyrius is 3-10× slower per op, and where each side wins. Preserves the 5-entry Rust bench history before the archive went away.
- Cyrius pin bumped to **4.4.4** (`cyrius.toml`). All correctness pain points from the original port are now fixed in cyrius itself; only the documented `var buf[N]` size limit remains as a design choice.

### Changed
- **`docs/spec-compliance.md`** — rewritten. Was carrying the Rust-era matrix that listed modules like `host::McpContentBlock` and `auth::TokenClaims` as "Complete" when they aren't ported yet. New version lists **only what cyrius v1.0.x covers today** (with explicit `✅` checkmarks per module + accessor function name) plus a single "Gaps that need coverage" section that points at the roadmap rather than duplicating it.
- README, `docs/architecture/overview.md`, `docs/development/roadmap.md` — `rust-old/` references rewritten to point at git tag `0.92.0`.
- Server `initialize` response — `serverInfo.version` now reports `"1.0.1"`.

### Verified
- 301 tests pass on cyrius 4.4.4
- 4 fuzz harnesses pass (no regressions)
- All 4 transports (stdio / HTTP / Unix / bridge) confirmed end-to-end

---

## [1.0.0] — 2026-04-13 — Stable cyrius MCP core

Bote's cyrius implementation is **stable**. The MCP protocol surface, registry,
dispatcher, schema validation, sessions, discovery, and four transports
(stdio, HTTP, Unix socket, TS bridge) are feature-complete and verified:

- **298 unit assertions** all passing
- **10 hot-path benchmarks**, all sub-10µs on x86_64
- **4 fuzz harnesses**, ~330 calls across malformed and edge-case inputs, no crashes
- **End-to-end smoke tests** for stdio (pipe), HTTP (curl), Unix socket (Python AF_UNIX), bridge (curl + CORS)

The data shapes for `JsonRpcRequest`, `JsonRpcResponse`, `ToolDef`, `ToolSchema`,
`ToolAnnotations`, `CompiledSchema`, `BoteError`, `McpSession`, and the four
`HttpConfig` / `BridgeConfig` flavours are **frozen** — additive changes only
within the 1.x series.

### Critical bug fix included in 1.0.0

- **`src/jsonx.cyr::jsonx_get_str`**: on truncated input (opening `"` with no closing — e.g. `{"k":"`), `_jx_skip_string` returned `end == len`, making `inner_len = end - pos - 2 == -1`. The subsequent `memcpy(out, src, -1)` was interpreted as a huge unsigned size → segfault. Surfaced by the `jsonx_extract.fcyr` fuzz harness on cyrius 4.4.x. **Fix**: clamp `inner_len` to `>= 0` (returns empty string for truncated input). Regression covered in `tests/bote.tcyr`.

### Workaround cleanup (cyrius 4.4.3 unblocked it)

Now that cyrius 4.4.3 ships `\r` escape correctness, `&&`/`||` short-circuit,
and per-block `var` shadowing, the defensive workarounds in bote can collapse:

- **`src/transport_http.cyr` + `src/bridge.cyr`**: removed `_crlf` / `_crlfcrlf` global pointers and `_http_init_crlf()` setup function. All HTTP / CORS response builders now use embedded `"\r\n"` and `"\r\n\r\n"` literals directly. ~50 lines removed.
- **`src/jsonx.cyr`**: collapsed three nested `if (i >= len) { ... } if (load8 != X) { ... }` patterns into single `if (i >= len || load8 != X)` checks. Same for `if (i < len) { if (load8 == 44) { ... } }` → `&&`.
- **`src/jsonx.cyr`**: `if (key_len_actual == klen) { if (memeq(...)) { ... } }` → `key_len_actual == klen && memeq(...) == 1` (was the explicit fix for the non-short-circuit `memeq`-on-truncated-input bug; now safe to write naturally).
- **`src/registry.cyr`**: `if (v != 0) { if (streq(v, version) == 1) { return t; } }` → `&&`.
- **`src/dispatch.cyr`**: `_extract_tool_name` ditched the `var bad = 0;` flag; now `if (name == 0 || strlen(name) == 0)`. `if (ver != 0) { if (registry_get_versioned(...) == 0) { ... } }` → `&&`. Schema-emit `if (props != 0) { if (vec_len(props) > 0) { ... } }` → `&&`.

Net diff: **60 lines removed across 6 files**. No behavior change, all tests / fuzz / e2e smokes still green.

### What's in 1.0.0

| Area | Status |
|---|---|
| JSON-RPC 2.0 (request, response, notification, batch) | ✅ |
| MCP `initialize` / `tools/list` / `tools/call` | ✅ |
| Tool registry with versioning + deprecation + annotations | ✅ |
| Compiled schema (type/enum/bounds/nested object/array items, multi-violation) | ✅ |
| `JsonRpcError` codes — full spec mapping | ✅ |
| Session management (create/validate/prune, MCP-Session-Id header) | ✅ |
| Origin allow-list + protocol-version header validation | ✅ |
| stdio transport | ✅ |
| HTTP/1.1 transport with middleware | ✅ |
| Unix domain socket transport | ✅ |
| TypeScript bridge (CORS + MCP envelope wrap) | ✅ |
| Discovery (data layer + pluggable publish_fp) | ✅ |
| Streaming primitives (ProgressUpdate, CancellationToken) | ✅ data layer |

### Post-1.0 extensions (1.x minor bumps)

These are additive — none change existing API shapes.

| Module | Status |
|---|---|
| `src/audit.cyr` + `LibroAudit` adapter | **Ready to port** — libro v1.0.3 available via `[deps.libro] path = "../libro"` |
| `src/events.cyr` + `MajraEvents` adapter | **Ready to port** — majra v2.2.0 available via `[deps.majra] path = "../majra"` |
| `src/discovery.cyr` wire-up to majra pubsub | **Ready to port** — depends on events |
| `src/libro_tools.cyr` (5 built-in audit tools) | **Ready to port** — depends on audit + libro |
| `src/sandbox.cyr` + kavach integration | Wait — kavach v2-arch hardening in flight |
| `src/host.cyr` (content blocks, host registry) | Ready (no AGNOS dep) |
| `src/auth.cyr` (OAuth 2.1 / PKCE / bearer) | Ready (no AGNOS dep) |
| `src/transport_streamable.cyr` (POST + SSE single endpoint) | Ready (rolls SSE on top of `transport_http`) |
| `src/transport_ws.cyr` (server-side WebSocket) | Cyrius `lib/ws.cyr` is client-only; needs server handshake + frame unmasking written |
| Threaded streaming dispatch | Needs `lib/thread.cyr` MPSC wired into `dispatcher_dispatch_streaming` |

### Versioning policy from here

Pre-1.0 used `0.D.M` (day.month). From 1.0.0 forward, **standard SemVer**:
- **Major** — break a frozen data shape or remove a public function.
- **Minor** — add a module / function / config option.
- **Patch** — fix bugs, refactor internals, improve diagnostics.

### Cyrius toolchain pin

Built and tested against cyrius **4.4.0** (`cyriusly use 4.4.0`).

---

## [0.1.1] — 2026-04-13 — Bridge + cyrius 4.4.0 + review punch list

### Added
- **`src/bridge.cyr`** — TypeScript-bridge HTTP transport: CORS preflight (`OPTIONS /`), `GET /health`, `POST /` JSON-RPC dispatch with MCP-envelope wrapping for `tools/call` results. `wrap_tool_result` (passthrough if already shaped, else wraps text), `wrap_error_result` (adds `isError: true`).
- **CLI**: `./build/bote bridge [port]` (default 8391).
- 29 new unit assertions: bridge wrappers, CORS origin selection, `bridge_process_message` round-trips, schema bounds at exact `min` / `max`, codec pure-notification batch.

### Fixed (review punch list)
- **`src/jsonx.cyr`**: `key_len_actual == klen && memeq(...)` was unsafe because cyrius `&&` doesn't short-circuit — `memeq` was called on truncated input. Now nested as separate `if`s.
- **`src/transport_http.cyr`**: when `Content-Length` was absent and `body_off > n` (malformed request), `clen = n - body_off` could be negative → `memcpy` UB. Now guarded.
- **`src/schema.cyr`**: `_sch_parse_int` replaced `i = i + 999999` marker-hack with proper `break` (per-block scoping now works in cyrius 4.4.0).
- **`src/transport_http.cyr`**: `http_find_header` similarly cleaned — replaced `vs = vs - 0; line_start = headers_end; vs = vs - 0;` marker hack with structured loops + `break`.

### Verified against cyrius 4.4.0 (`cyriusly install 4.4.0 && cyriusly use 4.4.0`)
- ✅ `\r` escape now emits CR (13) — fixed upstream
- ✅ Per-block `var` shadowing now works — fixed upstream
- ❌ `&&` / `||` short-circuit still missing — workarounds retained
- ➕ DCE now available via `CYRIUS_DCE=1` at build time

`docs/cyrius-feedback.md` updated with v4.4.0 verification status against each repro.

### Performance
Bench numbers unchanged from 0.1.0 — bridge adds a thin envelope-wrap layer with no measurable overhead on the hot dispatch path.

---

## [0.1.0] — 2026-04-13 — Cyrius port baseline

### Breaking
- **Language switch**: bote moved from Rust to Cyrius. The Rust source is preserved under `rust-old/` for reference and recovery. Version reset to `0.1.0` to mark the new lineage.
- **API change**: idiomatic Cyrius — module-prefixed function APIs (`registry_register`, `dispatcher_dispatch`, `codec_process_message`) over offset-addressed structs (`store64`/`load64`). No traits, generics, async, or borrow checking. Handler functions are i64 function pointers (`fn h(args_cstr) → result_cstr`).

### Added
- **`src/error.cyr`** — `BoteErrTag` enum (12 variants), `bote_err_rpc_code`, `bote_err_format`, schema-violation list support.
- **`src/protocol.cyr`** — `JsonRpcRequest` / `JsonRpcResponse` / `JsonRpcError` with raw-JSON-literal id/params/result/data slots.
- **`src/jsonx.cyr`** — Nested-aware JSON value extractor (`jsonx_get_raw`, `jsonx_get_str`, `jsonx_has`, `jsonx_is_object`). Handles nested objects, arrays, escaped strings; needed because `lib/json.cyr` is flat-only.
- **`src/registry.cyr`** — `ToolDef` (with `version`, `deprecated`, `annotations`, `compiled` slots), `ToolSchema`, `ToolAnnotations` (presets `read_only` / `destructive`), `ToolRegistry` (insertion-ordered, hashmap-indexed). Versioned tools, deprecation, validate-by-required-fields fallback.
- **`src/dispatch.cyr`** — `Dispatcher`, sync handler dispatch, `initialize` / `tools/list` / `tools/call` routing, MCP protocol-version negotiation, `validate_tool_name` (project_tool format, 256 char max), dynamic register/deregister.
- **`src/codec.cyr`** — `codec_parse_request`, `codec_serialize_response`, `codec_process_message` (single + batch + notification + error responses), JSON-message escaping reused from dispatch.
- **`src/schema.cyr`** — `CompiledSchema` with full type-checking (`string`, `number`, `integer`, `boolean`, `array`, `object`, `Any`), enum constraints, numeric bounds, recursive nested objects + array items, multi-violation reporting. `tool_def_with_compiled` slot wires it into `registry_validate_params`.
- **`src/stream.cyr`** — `CancellationToken`, `ProgressUpdate`, `ProgressSender`, `StreamContext`, `progress_notification` JSON builder. (Thread integration deferred.)
- **`src/session.cyr`** — `SessionStore` (hex-encoded 16-byte SIDs from `/dev/urandom`), `validate_protocol_version`, `validate_origin` (wildcard `*`, exact match, strict mode).
- **`src/transport_stdio.cyr`** — Line-oriented JSON-RPC over stdin/stdout, 128KB heap-allocated buffer, partial-line shifting.
- **`src/transport_http.cyr`** — HTTP/1.1 server (`POST /mcp` → JSON-RPC). Origin/MCP-Protocol-Version/MCP-Session-Id middleware. Auto-creates a session on `initialize` and emits the new `MCP-Session-Id` response header. Case-insensitive header lookup. 64KB request buffer.
- **`src/transport_unix.cyr`** — `AF_UNIX` line-oriented transport (own socket-creation code since `lib/net.cyr` is `AF_INET`-only). 128KB per-connection buffer.
- **CLI** — `./build/bote [stdio|http <port>|unix <path>]` selects transport.
- **Tests** — `tests/bote.tcyr` with **251 unit assertions** covering all modules.
- **Benchmarks** — `tests/bote.bcyr` with 10 hot-path benchmarks (all sub-10µs on x86_64).
- **Fuzz** — `fuzz/codec_parse.fcyr`, `fuzz/codec_process.fcyr`, `fuzz/jsonx_extract.fcyr`, `fuzz/schema_validate.fcyr` (~330 fuzzed calls; no crashes).
- **`docs/cyrius-feedback.md`** — language-level issues found during the port.
- `.gitignore` rules for `rust-old/target/` and `/build/`.

### Performance
- `dispatch_initialize` ~2µs avg
- `dispatch_tools_list` ~2µs avg
- `dispatch_tools_call` ~1µs avg
- `jsonx_get_str_flat` 600ns avg
- `jsonx_get_raw_nested` ~1µs avg
- `codec_parse_request` ~2µs avg
- `codec_serialize_response` ~1µs avg
- `codec_process_message` (full pipeline) ~5µs avg
- `validate_compiled_simple` ~1µs avg
- `validate_compiled_nested` ~3µs avg

### Deferred to future cyrius releases
- `bridge` — TypeScript bridge with CORS / MCP envelope wrapping.
- `audit` — libro hash-linked audit chain integration.
- `events` — majra pub/sub event publishing.
- `discovery` — cross-node tool announcements (depends on `events`).
- `sandbox` — kavach tool isolation.
- `host` — MCP hosting layer (content blocks, host registry).
- `libro_tools` — 5 built-in libro audit MCP tools.
- `auth` — OAuth 2.1 / PKCE / bearer-token middleware.
- `transport_ws` — server-side WebSocket (cyrius `lib/ws.cyr` is client-only).
- `transport_streamable` — streamable HTTP (POST + SSE single endpoint).
- Streaming dispatch (needs thread + channel integration).

### Known cyrius-language workarounds applied
- `\r` string escape emits byte `r` (114) instead of CR (13) — built CRLF via `store8`.
- `&&` / `||` operators do not short-circuit — guarded null derefs nested as `if (p != 0) { if (...) { ... } }`.
- No per-block local scoping — distinct names per `fn` body (`req_one`, `rcompiled`, `prog_notif`, etc.).
- Static `var buf[N] >~ 16KB` exhausts the output buffer — large buffers heap-allocated (`var ptr = 0;` global + `ptr = alloc(N);` at startup).

See [docs/cyrius-feedback.md](docs/cyrius-feedback.md) for full reproductions.

---

## Historical (Rust) — preserved under `rust-old/`

## [0.91.0] — 2026-04-02

### Added
- `libro_tools` module (feature: `audit`) — 5 built-in MCP tools for libro audit chain operations:
  - `libro_query` — query audit entries by source, severity, action, agent, min_severity, with limit
  - `libro_verify` — verify chain integrity and return structured `ChainReview` JSON with integrity status, entry count, time range, source/severity/agent distributions (was text-only)
  - `libro_export` — export chain as JSON Lines or CSV
  - `libro_proof` — generate Merkle inclusion proof for an entry by index, returns structured proof JSON with verification status
  - `libro_retention` — apply retention policies (PCI-DSS, HIPAA, SOX, keep_count) and report archived entries (destructive, not read-only)
- `libro_tools::register()` — convenience function to register all 5 libro tools on a dispatcher
- Read-only tools annotated with `ToolAnnotations::read_only()` (MCP 2025-11-25); `libro_retention` is destructive (no annotation)
- `LibroAudit::with_source()` — custom source tag for audit entries (default: `"bote"`)
- `LibroAudit::with_agent_id()` — server agent identity on all entries; `caller_id` from events takes precedence
- `LibroAudit` now uses `append_with_agent()` when caller_id or agent_id is present, populating libro's agent tracking
- 17 libro_tools tests + 8 audit tests (was 8 + 3)
- **HTTP transport middleware**: Origin validation (403), `MCP-Protocol-Version` enforcement (400), `MCP-Session-Id` session lifecycle (404), bearer token extraction with 401/403 responses (feature `auth`)
- **Streamable HTTP transport router**: axum router with POST (JSON-RPC) and GET (SSE stream) on configurable endpoint path, same middleware stack as HTTP, SSE event IDs via `EventIdGenerator`, `Last-Event-ID` resumption via `ResumptionBuffer` replay, `retry:` hint before close, priming event on connect
- `HttpConfig` builder: `with_allowed_origins()`, `with_session_timeout()`, `with_token_validator()` (feature `auth`)
- `StreamableConfig` builder: `with_session_timeout()`, `without_sessions()`
- `TokenValidator` trait (feature `auth`) — consumers implement to validate bearer tokens
- Shared `transport::middleware` module — `check_origin`, `check_protocol_version`, `check_protocol_version_required`, `check_session`, `check_bearer` reused by both transports
- Periodic session pruning via tokio interval in both `http::serve()` and `streamable::serve()`
- `streamable::streamable_router()` — build router without binding a port (for testing)
- 35 new transport middleware tests (origin, protocol version, session enforcement in both transports)
- `cargo vet` supply chain auditing: 156 crates fully audited via trusted imports (mozilla, google, bytecode-alliance, isrg, zcash, ariel-os, embark-studios) and 27 trusted publishers (dtolnay, seanmonstar, Manishearth, epage, fitzgen, kennykerr, Amanieu, BurntSushi, Thomasdezeeuw, cuviper, alexcrichton, carllerche, Darksonn, rust-lang-owner), 66 exempted, CI integration

### Changed
- Upgraded libro dependency from 0.25 to 0.91 (BLAKE3 hashing, serde on all types, key rotation support)
- `HttpConfig` expanded with `allowed_origins`, `session_timeout`, `token_validator` fields
- `StreamableConfig` expanded with `session_timeout` field
- Streamable transport `MCP-Protocol-Version` header is **required** (per MCP 2025-11-25), unlike plain HTTP where it is optional

## [0.90.0] — 2026-04-01

### Fixed
- **JSON-RPC 2.0 spec compliance**: Unknown methods now return `-32601` (Method not found) instead of `-32600` (Invalid Request)
- **Bridge spec compliance**: Error wrapping no longer sets both `result` and `error` on the response (JSON-RPC 2.0 violation)
- `scripts/bench-log.sh`: Added missing `--features bridge` flag

### Performance
- **Notification dispatch 17x faster** (170ns → 10ns): Early-return before lock acquisition when request is a notification
- **Parameter validation 26% faster** (47ns → 35ns): Merged `tools` + `compiled` HashMaps into single `entries` map, eliminating key duplication
- **Schema validation 8% faster** (107ns → 99ns): Same registry merge reduces lookup overhead

### Changed
- `ToolRegistry` internal structure: merged separate `tools` and `compiled` maps into unified `entries` map
- CLAUDE.md: Added task sizing, refactoring guidelines, testing section, documentation structure, CHANGELOG format, module table, stack table

### Added
- 3 new conformance tests: `error_codes_comply_with_spec`, `bridge_error_response_is_spec_compliant`, `registry_deregister_cleans_up_compiled_schema`
- 18 downstream consumers integrated (daimon, agnoshi, t-ron, jalwa, nein, stiva, itihas, varna, selah, hoosh, vidya, rasayan, szal, tarang, vidhana, nazar, mneme, tazama)

## [0.50.0] — 2026-03-26

### Added
- Protocol conformance test suite (41 tests in `tests/conformance.rs`)
- Streaming audit logging — all transports now call `log_tool_call()` after streaming handler completion with timing and success/error status
- `BoteError::SandboxError` variant for sandbox execution failures

### Fixed
- Streaming tool calls in HTTP/SSE, WebSocket, Unix, and stdio transports now correctly produce audit events via `log_tool_call()`
- Added missing doc comment on `Dispatcher::new()`

## [0.25.3] — 2026-03-26

### Added
- Tool sandboxing via kavach (feature `sandbox`)
- `ToolSandboxConfig` with presets: `basic()`, `strict()`, `noop()`
- `SandboxExecutor` for running commands in kavach sandboxes
- `wrap_command()` and `wrap_streaming_command()` handler wrappers
- `Dispatcher::register_sandboxed_tool()` and `register_sandboxed_streaming_tool()` convenience methods
- `BoteError::SandboxError` variant for sandbox execution failures
- Sandbox lifecycle event topics: `bote/sandbox/created`, `bote/sandbox/destroyed`, `bote/sandbox/error`
- Async-sync bridge with `OnceLock<Runtime>` fallback for non-tokio contexts

## [0.24.3] — 2026-03-26

### Added
- Full JSON Schema validation: type checking (string, number, integer, boolean, array, object), enum constraints, numeric bounds, nested object/array validation
- `CompiledSchema` — compile `ToolSchema` into typed representation for fast validation
- Default value injection via `CompiledSchema::apply_defaults()`
- `SchemaType`, `PropertyDef` types in new `schema` module
- `BoteError::SchemaViolation` variant with multiple violation reporting
- Tool versioning: `version` and `deprecated` fields on `ToolDef`
- `ToolDef::with_version()` and `ToolDef::with_deprecated()` builder methods
- `ToolRegistry::get_versioned()`, `list_versions()`, `deprecate()`, `deregister()`
- Version negotiation in `tools/call` dispatch
- Deprecation warnings via tracing + event publishing
- Dynamic tool registration/deregistration via `Dispatcher::register_tool()`, `deregister_tool()`
- Hot-reload: re-registering a tool atomically replaces its handler
- Tool namespacing: `project_tool` format enforcement on dynamic registration
- `TOPIC_TOOL_DEPRECATED` and `TOPIC_TOOL_DEREGISTERED` event topics
- Schema validation, versioning, and dynamic registration benchmarks

### Changed
- `Dispatcher` internals migrated to `RwLock` for thread-safe dynamic registration
- `ToolRegistry::validate_params()` now uses compiled schema for full type validation
- `tools/list` response includes `version` and `deprecated` fields when present

## [0.23.3] — 2026-03-26

### Added
- TypeScript bridge module with CORS and MCP result formatting (feature `bridge`)
- `wrap_tool_result` adapter — converts raw results to SY's `{ content: [{ type, text }] }` envelope
- Bridge CORS preflight handling for cross-origin TypeScript clients
- Cross-node tool discovery via majra pub/sub (feature `discovery`)
- `DiscoveryService` for announcing and subscribing to tool announcements
- `ToolAnnouncement` type for cross-node tool broadcast
- New event topics: `bote/tool/announce`, `bote/tool/discovered`
- Bridge benchmark (`wrap_tool_result` overhead)

### Changed
- `full` feature now includes `bridge` and `discovery`
- Transport codec module visibility changed to `pub(crate)` for bridge reuse

## [0.22.3] — 2026-03-22

### Added
- HTTP transport (axum-based, feature `http`)
- WebSocket transport (bidirectional, feature `ws`)
- Unix domain socket transport (newline-delimited JSON, feature `unix`)
- Graceful shutdown on all network transports via shutdown future
- SSE streaming for long-running tool calls (HTTP)
- Progress notifications during execution (StreamContext, ProgressSender, CancellationToken)
- Streaming handler type (StreamingToolHandler, dispatch_streaming, DispatchOutcome)
- Cancellation support ($/cancelRequest, CancellationToken)
- Batch requests (JSON-RPC 2.0 batch array)
- Notification support (no id, no response expected)
- Protocol version negotiation in initialize handshake (2024-11-05, 2025-03-26)
- process_message() codec function for batch/notification/single dispatch
- Audit logging via libro (AuditSink trait, LibroAudit adapter, feature `audit`)
- Event publishing via majra (EventSink trait, MajraEvents adapter, feature `events`)
- Tool call timing with automatic audit + event logging in dispatch
- Event topic constants (TOPIC_TOOL_COMPLETED, TOPIC_TOOL_FAILED, TOPIC_TOOL_REGISTERED)
- Feature flags: http, ws, unix, all-transports, audit, events, full
- progress_notification() helper for consistent JSON-RPC notification format
- extract_tool_name() helper for validated tool name extraction
- Send + Sync compile-time assertions on all public types
- Benchmark suite: 8 benchmarks (dispatch, process_message, batch, streaming, validation)
- Benchmark history logging via scripts/bench-log.sh
- CODE_OF_CONDUCT.md, CONTRIBUTING.md, SECURITY.md
- codecov.yml with 80% project / 75% patch targets
- 129 tests across all modules

### Changed
- Transport module restructured: transport.rs -> transport/ directory (codec, stdio, http, ws, unix)
- Dispatcher.dispatch() returns Option<JsonRpcResponse> (None for notifications)
- JsonRpcRequest.id is now Option<serde_json::Value> (supports notifications)
- All transports use process_message() for unified dispatch
- WS/Unix transports use outgoing message channel pattern for streaming
- Mutex locks are poison-safe across all transports (unwrap_or_else)
- Handler panics caught and returned as -32603 error responses
- Cancelled tasks return -32800 (distinguished from panics in async transports)
- BoteError is now #[non_exhaustive]
- Serialization calls use explicit BUG labels instead of bare unwrap()
- deny.toml: tightened license list, added version 2 advisories, explicit allow-registry
- Makefile: added coverage, bench, --no-default-features clippy, RUSTDOCFLAGS for doc
- Cargo.toml: added documentation, exclude, full feature

### Fixed
- HTTP returns proper JSON-RPC error for malformed JSON (was returning 422)
- validate_params rejects non-object params (was silently passing)
- Dispatch uses BoteError consistently (was hardcoding error codes)
- transport::parse_request uses Json error variant (was converting to string)
- Empty tool name now returns -32602 instead of falling through to not-found
- jsonrpc version validated (rejects non-"2.0")
- Progress notification JSON deduplicated via stream::progress_notification()
- Tool name extraction deduplicated via Dispatcher::extract_tool_name()

### Removed
- Unused dependencies: anyhow, tokio, uuid (from core; tokio re-added as optional for transports)
