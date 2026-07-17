# Bote Roadmap

> **Current**: `3.1.4` (cyrius 6.4.66, libro 2.8.2, majra 2.5.1, sigil 3.12.0, sakshi 2.4.6).
> 12 active test files, **786 unit assertions** + 1 drift-guard
> smoke, **14 criterion benchmarks**, **dual** consumer bundles
> (`dist/bote.cyr` full, 28 modules + `dist/bote-core.cyr` opt-in core via
> `[lib.core]` profile, 11 modules), per-transport binary trio
> (`bote` / `bote-streamable` / `bote-ws` — retained from the
> 5.10.x cap workaround; reconsolidation unblocked on 6.1.x), CI capacity + dual
> dist-freshness gates, full MCP capability suite (tools / prompts /
> resources / completion + polled `list_changed` push), fs / web / libro
> tool families, annotations-preserving `wrap_tool_result`,
> HostRegistry hot-reload, 4 fuzz harnesses, 6 transports,
> handler-claims ABI plumbed end-to-end, JWT HS256 + RFC 7636
> PKCE, bearer + allowlist + JWT validators, pluggable sandbox
> runner (kavach 3.0 compatible), typed MCP content blocks
> with annotations, HostRegistry + IPv4/IPv6 SSRF guard.
>
> **Spec**: MCP 2025-11-25 | **Compliance**: [spec-compliance.md](../spec-compliance.md)
>
> **Bench history**: [benchmarks-rust-v-cyrius.md](../benchmarks-rust-v-cyrius.md)
>
> **Full release history**: [CHANGELOG.md](../../CHANGELOG.md).
> Rust archive preserved at git tag `0.92.0` (retired in v1.0.1).

3.0 shipped. The 2.0 handler ABI (fn `(args, claims) → result_cstr`) and the six transports are stable across the 2.x→3.x line; patch releases add capabilities, not shape changes.

**2.6.x was the modernization arc.** Forward feature work that was
on the 2.6.x slate shifted to 2.7.x. The 2.6.x line was reserved
for catching bote up to the first-party Cyrius floor: the dist-bundle
dep contract, the cyrius.cyml + `${file:VERSION}` layout, the
versioned-toolchain CI installer, the `cyrius deps --verify` /
`cyrius.lock` gates, and the residual libro 2.6.x / majra 2.4.x /
sandhi-HTTP-server porting that the 5.10.34 toolchain bump
surfaced. See the **2.6.x modernization arc** section below.

---

## Shipped

| Release | Headline |
|---|---|
| **1.0.0 – 1.8.1** | Cyrius port + incremental ships: AuditSink / EventSink, libro + majra adapters, streamable HTTP, WebSocket, libro_tools, content blocks, HostRegistry + IPv4 SSRF |
| **1.9.0** | Bearer-token middleware (RFC 6750) — opt-in, all four HTTP-family transports |
| **1.9.1** | IPv6 SSRF + `content_resource_blob` + `BOTE_BEARER_TOKENS` env-wired auth |
| **1.9.2 – 1.9.3** | Toolchain bumps (cyrius 4.7.0 → 4.7.1); 2.0-prep doc sweep |
| **1.9.4** | Security batch A — HTTP smuggling guard, constant-time bearer compare, batch-size cap, jsonx depth cap, urandom-or-fail |
| **1.9.5** | Security batch B — SSRF rewrite (integer-form / octal / IPv4-mapped IPv6 bypasses, 3 criticals closed) |
| **1.9.6** | Final pre-2.0 polish — 413 cap, bridge CORS oracle fix, Unix socket mode 0600, `content_with_annotations` |
| **2.0.0** | Stable release — handler-claims ABI (`fn h(args, claims)`), carry-forward of all 1.9.x hardening |
| **2.1.0** | Pluggable sandbox runner — kavach 3.0 compatible via fn-pointer + ctx adapter |
| **2.2.0** | JWT HS256 verifier + validator adapter |
| **2.3.0** | RFC 7636 PKCE-S256 helpers (verifier gen + S256 challenge) |
| **2.3.1** | Cleanup — remove proposal docs that landed upstream |
| **2.4.0** | Bump cyrius 4.8.1 + base64url adoption + compile-unit trim |
| **2.5.0** | Claims propagation through transports (validator's return threads to handler) |
| **2.5.1** | Restore audit_libro + events_majra tests after cyrius 4.8.4 retag |
| **2.6.0** | Modernization platform — cyrius 5.10.34, libro 2.6.2 / majra 2.4.3 via dist bundles, cyrius.cyml + `${file:VERSION}` layout, versioned-toolchain CI installer, sandhi compat shim |
| **2.6.1** | Retire `_sandhi_compat.cyr` — 108 call sites flipped to `sandhi_server_*` names; mechanical rename, no behaviour change |
| **2.6.2** | Port `src/libro_tools.cyr` to libro 2.6.x API (raw struct offsets replace retired `entry_*`/`error_*`/`merkle_*` getters); `bote_libro_tools.tcyr` re-enabled (22 assertions); back to 603-assertion baseline |
| **2.6.3** | Ship `dist/bote.cyr` — single-file consumer bundle via `cyrius distlib`. CI freshness gate + release asset. libro/majra-style downstream distribution contract |
| **2.6.4** | CI capacity gate (`CYRIUS_STATS=1` + 95% fn_table / identifier-buffer threshold). Modernization arc closes. Three documented response paths (upstream cap raise, transport split, `BOTE_FULL_CONFIG` gate) if the gate ever fires |
| **2.7.0** | Carry-forward cleanup. Annotations propagation through `wrap_tool_result` (single content block lifts into envelope, preserves block-level annotations from 1.9.6). `schema_compile` + `auth_bearer_check` benchmarks (closes the bench-coverage list in `docs/benchmarks-rust-v-cyrius.md`). `## [Unreleased]` CHANGELOG flow adopted |
| **2.7.1** | HostRegistry hot-reload — `host_entry_from_json` / `host_registry_load_json` / `host_registry_load_from_file` / `host_registry_reload` / `host_registry_clear`. Fail-safe semantics on bad config (registry unchanged on parse error). +46 assertions, total 653. CONTRIBUTING.md rewritten for the Cyrius era |
| **2.7.2** | Toolchain + dep refresh (cyrius 5.10.34 → 5.10.44, libro 2.6.2 → 2.6.3, majra 2.4.3 → 2.4.4); stdlib + `slice` / `assert` / `ct` / `keccak` / `random` for sigil 3.x transitives. **`dist/bote-core.cyr` opt-in profile** (9 modules, 70 KB, `cyrius distlib core`) closes the t-ron consumer blocker and lands `DEPS-PATTERN.md` + `tests/bote_core_only_smoke.tcyr` drift guard + dual dist-freshness CI. **Per-transport binary split** (`bote` / `bote-streamable` / `bote-ws`) — interim 5.10.x cap workaround; reconsolidates on 5.11.x. Per-module test split: `bote_streamable.tcyr` (25) + `bote_ws.tcyr` (10) extracted from the monolithic `bote.tcyr`. `scripts/bench-log.sh` ported from `cargo bench` to `cyrius bench` |
| **2.7.3** | **Cyrius major-version jump (5.10.44 → 6.1.24)** — the planned 5.11.x migration landed as 6.1.x. libro 2.6.3 → 2.7.2, majra 2.4.4 → 2.4.5. No `src/*.cyr` change; all 653 assertions + drift smoke pass, 14 benchmarks no-regression. Compile cap relieved: fn_table / identifier utilisation 93% / 92% → **52% / 52%** on raised 6.1.x caps — per-transport split reconsolidation now unblocked. `cyrius.lock` full-hash format (6 → 40 entries). Cleanliness sequence drops repurposed `cyrius audit`; adds `cyrius vet` (include-dependency audit) alongside `cyrius deny` |
| **2.7.4** | Toolchain patch refresh (cyrius 6.1.41); **breaking** tool-registry constructor rename to resolve the ai-hwaccel `registry_new` collision for multi-library consumers (szal, mihi, hoosh). All 653 assertions (+ drift smoke) pass on the renamed constructor |
| **2.7.5** | **`libro_tools` folded back into the default binary + `dist/bote.cyr`** (now 24 modules) — reverts the 1.9.4 cap-headroom decision now that the 6.1.x cap raise puts `src/main.cyr` at 58% / 60% (`fn_table 4764/8192`). `main()` stands up an in-memory libro chain and registers the five `libro_*` audit tools by default. Stays out of `dist/bote-core.cyr` (depends on a live libro chain, like `audit_libro`). All 653 assertions (+ drift smoke) pass |
| **2.7.6** | **Cyrius 6.1.41 → 6.2.11** (first move onto the 6.2.x line) + dep refresh (libro 2.7.2 → 2.7.4, majra 2.4.5 → 2.4.7, sigil 3.7.12 → 3.7.14). **sigil 3.7.14 TLS-path SIGILL guard**: `thread_local` added to `[deps] stdlib` (before `sigil`) and to all six sigil-using test files — without it the crypto path links clean but SIGILLs at runtime (exit 132). 6.2.11 formatter reflow (whitespace) across `src/` + `tests/`; `dist/*` regenerated at v2.7.6. `fn_table 4770/8192` (58% / 60%). All 653 assertions (+ drift smoke) pass, 14 benchmarks no-regression |
| **2.7.7** | **Cyrius 6.2.11 → 6.3.15 base-stack migration** — tier-3 step of the coordinated base-security-stack migration (sakshi 2.4.3 → sigil 3.9.8 → majra 2.5.0 → libro 2.7.9 → bote → the five consumers). 6.3.x stdlib rename reconciliation (`http_send_204` → `sandhi_server_send_204`; bote-local `http_find_header` compat shim in `transport_ws.cyr`); `atomic` / `sync` / `dynlib` added to `[deps] stdlib`; stale `_bote_server_version` literal fixed (`2.7.1` → `2.7.7`). No runtime logic change; all 653 assertions pass |
| **2.7.8** | **AF_UNIX transport fail-closes on agnos** — `transport_unix_run` guarded with `#ifdef CYRIUS_TARGET_AGNOS` (agnos has no AF_UNIX domain sockets); the full `bote` binary now compiles under `cyrius build --agnos` (bote-core was already agnos-clean). Mirrors majra's ipc AF_UNIX guard |
| **2.8.0** | **Filesystem tools** — `fs_write` / `fs_read` / `fs_mkdir` in new `src/fs_tools.cyr`, root-confined (`BOTE_FS_ROOT`; absolute / `..` paths refused), opt-in via `fs_tools_register()`. In the full bundle (25 modules), not core. +26 assertions (`tests/bote_fs_tools.tcyr`) |
| **2.9.0** | **Runs + serves MCP on agnos** — cyrius 6.3.15 → 6.3.38 picks up the stdlib `freelist.cyr` agnos `mmap#27` fix (the stale vendored copy SIGSEGV'd every `fl_alloc` consumer, killing sigil's crypto in `main()` at `chain_new()`). Full MCP flow proven under mirshi and on the real agnos kernel under QEMU (`BOTE_SELFTEST` + `bote-mcp-smoke.sh`) |
| **3.0.0** | **MCP capability suite + honest polled-push notifications** — prompts / resources / completion capabilities plus `notifications/tools/list_changed` + `notifications/prompts/list_changed`, delivered on the client's next streamable `GET` or POST-piggyback SSE; `listChanged` advertised only where a drain path exists (streamable, never stdio/http/ws). **Breaking**: `bote-streamable` enforces MCP session lifecycle (`MCP-Session-Id`). `[lib.core]` 9 → 11 modules (`prompts.cyr`, `resources.cyr`). cyrius 6.3.38 → 6.3.42. 733 assertions (was 653) |
| **3.0.1** | **`bote_echo` MCP conformance** — the reference sample tool now wraps its echoed args in a text content block via `content_text_response` (was a bare JSON object, invalid as a `tools/call` result). Toolchain 6.3.42 → 6.4.20 |
| **3.1.0** | **Web tools** — `web_fetch` (HTML→readable-text stripper, 64 KiB cap, scheme guard) + `web_search` (SearXNG via `BOTE_SEARXNG_URL` — self-hostable, no third-party key) in new `src/web_tools.cyr`; outbound HTTP via the sandhi client. Stripper drops C0 control bytes / DEL / raw NUL from the untrusted page. +27 assertions (`tests/bote_web_tools.tcyr`) |
| **3.1.1** | **Native HTTPS large responses** — cyrius 6.4.20 → 6.4.34 carries the stdlib native-TLS record-layer fix (max-size 16 KB record off-by-one + partial-record delivery); `web_fetch` / `web_search` now work against real-world hosts over the sovereign native backend. No bote source change |
| **3.1.2** | **Toolchain 6.4.64 + full dependency refresh** — libro 2.8.1 (audit-row quoting integrity fix; pulls patra 1.12.10 as a new transitive), majra 2.5.1, sigil 3.12.0 (crypto-bank thread-local slot fix), new explicit **sakshi 2.4.6 pin** (registry lag, same class as the sigil pin). No bote logic change. 786/786 assertions across 12 test files + drift smoke, 14 benchmarks flat, capacity 59% / 61% (`fn_table 4841/8192`) |
| **3.1.3** | **Toolchain 6.4.66 + `BoteErrTag` namespacing** — cyrius 6.4.64 → 6.4.66 (clears pin drift; `lib/` re-sync pulls the `thread_local_alloc` slot allocator that sigil 3.12.0 / patra 1.12.10 now require — the stale snapshot no longer linked). `BoteErrTag` constants `ERR_*` → `BOTE_ERR_*` to escape a flat-namespace collision with libro's own `ERR_IO=3` / `ERR_JSON=4` (bote's `=11` / `=10`; "last definition wins" in the libro-linked binary). Wire contract unchanged. 786/786 assertions, 14 benchmarks flat, capacity 60% / 62% (`fn_table 4879/8192`) |
| **3.1.4** | **libro 2.8.2 (`LIBRO_ERR_*`) + pin/lock realign** — `[deps.libro]` `2.8.1 → 2.8.2`. libro 2.8.2 namespaces its own `LibroErr` enum `ERR_* → LIBRO_ERR_*` — the upstream reciprocal of 3.1.3's `BOTE_ERR_*`; the bare `ERR_IO`/`ERR_JSON` clash is now resolved at the source on both sides. Also realigns the `[deps.libro]` tag with the lockfile (3.1.3 shipped tag `2.8.1` while the lock already held 2.8.2's content hash via the local `path` override — a clean `git+tag` CI checkout would fail hash verification). libro 2.8.2's own deps (sigil 3.12.1 / patra 1.12.12) sit inside its dist; bote keeps sigil 3.12.0 and its already-1.12.12 patra. No bote source change beyond the version string. 786/786 assertions, 14 benchmarks flat, capacity flat 60% / 62% |

See [CHANGELOG.md](../../CHANGELOG.md) for the full detail per release.

---

## 2.6.x modernization arc

The 2.6.x line catches bote up to the first-party Cyrius floor.
Each patch is a small, well-bounded bite — nothing in this arc
ships new MCP surface; behaviour is preserved at the wire level.

| Patch | Bite | Notes |
|---|---|---|
| **2.6.0** | Toolchain floor + dist-bundle deps | ✅ Shipped. cyrius 5.10.34, libro 2.6.2 / majra 2.4.3 via `dist/<crate>.cyr`, cyrius.cyml + `${file:VERSION}`, lib/ untracked, CI installer matches majra/agnosys, sandhi compat shim. `bote_libro_tools.tcyr` parked. |
| **2.6.1** | Retire `_sandhi_compat.cyr` | ✅ Shipped. 108 call sites across `auth.cyr` / `bridge.cyr` / `transport_http.cyr` / `transport_streamable.cyr` / `transport_ws.cyr` + tests flipped to `sandhi_server_*` names. Shim deleted; CI manifest-completeness gate's `EXCLUDES` allowlist gone. |
| **2.6.2** | Port `libro_tools.cyr` to libro 2.6.x API | ✅ Shipped. Raw struct-offset accessors (`_lt_entry_*`, `_lt_err_*`, `_lt_chain_entries`, `_lt_merkle_leaf_count`) replace the retired `entry_*`/`error_*`/`chain_entries`/`merkle_tree_leaf_count` getters; `merkle_proof` → `merkle_inclusion_proof`. `bote_libro_tools.tcyr` re-enabled (22 assertions); 8-file matrix in CI. libro_tools is still opt-in for the default binary (fn_table headroom). |
| **2.6.3** | `cyrius distlib` bundle for bote | ✅ Shipped. `dist/bote.cyr` (4615 lines, committed) generated from `cyrius.cyml [lib] modules`; CI freshness gate enforces byte-clean diff vs the committed bundle; release ships it as `bote-<ver>.cyr` next to source tarball + binary + lockfile + SHA256SUMS. `libro_tools.cyr` stays out of the default bundle (opt-in). |
| **2.6.4** | Capacity / split prep | ✅ Shipped. CI capacity gate enforces fn_table + identifier-buffer utilisation < 95% via `CYRIUS_STATS=1` + a parser step in `.github/workflows/ci.yml`. Current util 89% / 88% (no source-side split needed yet). `CYRIUS_DCE=1` measured to be a no-op for the cap counters (compile-time vs emitted bytes). Three documented response paths if the gate fires: upstream cap raise (preferred — has happened before), opt-in transport split (mirrors libro_tools), or `#ifdef BOTE_FULL_CONFIG` feature gate on the ~30 unused config setters. |

✅ **Modernization arc closed at 2.6.4.** The 2.7.x feature
backlog below is now unblocked.

---

## Forward roadmap — 2.7.x candidates

After 2.7.0's carry-forward cleanup, the remaining 2.7.x slate
narrows to one functional feature, one doc cleanup, and one
out-of-scope marker. 2.7.x is where MCP-spec-aligned capability
work belongs.

### Next candidates

| Item | Priority | Effort | Notes |
|---|---|---|---|
| **Add `content.cyr` to the `[lib.core]` profile** — ship the typed content-block constructors (`content_text`, `content_text_response`, `content_array`, `content_array_error`, `content_image`, `content_resource`, …) in `dist/bote-core.cyr` | **P1** | Small | Content blocks are the tool-result format *every* handler emits — transport or not — but `content.cyr` currently ships only in the full `[lib]` bundle. Core-profile consumers (nein 1.6.0 `mcp` module; t-ron) are therefore forced to hand-roll `{"content":[…],"isError":…}` with a raw `str_builder` + `_json_emit_escaped`, duplicating logic content.cyr already provides — and re-implementing JSON escaping per consumer is exactly the injection-surface duplication the core profile should prevent. `content.cyr` (232 lines, 13 fns) references only `_json_emit_escaped` (already in core via `dispatch.cyr`), `str_builder_*`, and `vec_*` — no transport/host/session deps — so it drops into `[lib.core]` after `dispatch.cyr` cleanly (single-pass ordering). Fix: add the module to `[lib.core]`, extend the core-only drift guard (`tests/bote_core_only_smoke.tcyr`), regen both dist bundles. Surfaced building nein's MCP tool handlers against `dist/bote-core.cyr`. |
| **DEPS-PATTERN.md doesn't mention `cyrius lib sync`** — a core consumer following the doc hits `dep libro requires 'ct' … not in the cyrius stdlib` and reasonably (but wrongly) concludes it's a resolver bug | **P2 — docs** | Small | **NOT a resolver bug** (earlier diagnosis was wrong). Cyrius deliberately does not auto-resolve stdlib (supply-chain safety); a consumer of the bote/libro/majra graph must (a) declare every transitive stdlib module in `[deps] stdlib` — `ct, keccak, random, slice, thread, thread_local, sync, atomic, ws_server, result` (+ `sigil`) — and (b) run **`cyrius lib sync`** to copy that declared subset into `./lib/` **before** `cyrius deps`. DEPS-PATTERN.md documents `git + tag + modules` but omits the `lib sync` step and the transitive-stdlib requirement, so a first-time core consumer dead-ends on the `ct` error and thinks it's broken. nein 1.6.0 vendored bote-core over this misread; nein 1.6.1 retired the vendoring and consumes bote-core + sigil as git deps the same way daimon does (works cleanly). Fix: add a "Consuming the core bundle from a project without the crypto stack" section to DEPS-PATTERN.md showing the `[deps] stdlib` list + the `cyrius lib sync → cyrius deps` order. Surfaced building nein's `mcp` + `sign` modules. |
| **Opt-in transport profile** — `dist/bote-core.cyr` alongside `dist/bote.cyr` | ✅ **Shipped 2.7.2** | Medium | Resolved per [`issues/2026-05-10-opt-in-transport-profile.md`](issues/2026-05-10-opt-in-transport-profile.md). `cyrius.cyml [lib.core]` profile, 9-module 70 KB bundle, `DEPS-PATTERN.md`, `tests/bote_core_only_smoke.tcyr` drift guard, dual dist-freshness CI. t-ron 2.1.x flips its [deps.bote] to `dist/bote-core.cyr` in next patch. |
| **Reconsolidate per-transport binaries** — fold `bote-streamable` + `bote-ws` back into single `bote` binary | **P2 — unblocked** | Small | Unblocked by the 6.1.x cap raise at 2.7.3 (the planned 5.11.x migration per the companion proposal at `cyrius/docs/development/proposals/2026-05-10-raise-compile-source-cap.md` landed as 6.1.x). When taken up, retire `src/main_streamable.cyr` / `src/main_ws.cyr` / `src/main_common.cyr` and restore the streamable / ws CLI branches in `src/main.cyr`. The `dist/bote-core.cyr` profile stays — still useful for transport-less consumers. |
| **OAuth 2.1 authorization-code flow** (bote-as-AS) | Deferred | High | Out of scope for MCP core; bote is the resource server. Flagged as explicitly deferred — consumers compose bote with their own AS layer. |

The functional 2.7.x slate from the 2.6.x carry-forward list is
empty after 2.7.2 (HostRegistry hot-reload + CONTRIBUTING.md + the
opt-in core profile all shipped). ✅ **The notifications arc shipped
at 3.0.0** — the dep-free polled-push MVP (buffer at produce time,
drain on the client's next streamable `GET`, built on the
`ResumptionBuffer` scaffold, plus POST-piggyback SSE) landed with
`tools` / `prompts` `list_changed`; `resources/subscribe` and
`logging` stay intentionally unadvertised (no producer — advertising
them would promise messages bote can't deliver). Only real-time
*held-open* streaming remains open; cyrius `lib/thread.cyr` (MPSC +
mutex) and `lib/async.cyr` are **complete and pinned**, so it's a
bote-side threading task, not a cyrius gate — needed because the
single-threaded sandhi accept loop would otherwise deadlock (a held
GET starves the POSTs that feed it).

### Blocked on cyrius / external

| Item | Waiting on |
|---|---|
| **`$/cancelRequest` mid-stream handling** | Real-time (held-open) streaming dispatch first — itself a bote-side threading task, not a cyrius gate (see the notifications note above). |
| **Slowloris recv timeout** (audit H5) | `sock_set_recv_timeout` helper in stdlib `lib/net.cyr`. |
| **WebSocket `Sec-WebSocket-Key` length validation** (audit M4) | stdlib `lib/ws_server.cyr` fix. |
| **WebSocket arena-per-frame allocator** | stdlib `fl_free` support for long-lived connections. |
| **WS subprotocol negotiation** (`Sec-WebSocket-Protocol`) | Header is read; enforcement needs a registry design. |
| **WS per-message deflate** (RFC 7692) | LZ77 + Huffman in stdlib; likely via a future `lib/dynlib.cyr` zlib binding. |
| **DNS resolution for hostname SSRF** | cyrius `getaddrinfo` stub. Production callers pair with a network-policy egress block. |
| **JWT RS256 / ES256** | sigil RSA / ECDSA primitives. HS256 already shipped in 2.2.0. |

### Carried forward (not release-blocking)

| Item | Notes |
|---|---|
| **v1.2.1 libro-growth heisenbug** | Heap-layout sensitivity when the chain grows while libro+majra+bote are all loaded. Does not affect 1.6.0+ `libro_tools` (read-only). Isolated probes prove the adapter is correct. |
| **Per-thread request buffers** | cyrius-side; affects future threaded dispatch. |

---

## Feature freeze

Data shapes frozen in 2.0.0:
- `JsonRpcRequest` / `Response` / `Error`
- `ToolDef` (with `compiled` slot — additions allowed at the tail)
- `ToolSchema` / `ToolAnnotations`
- `BoteError` (12 tag variants — additions allowed at the tail)
- `HttpConfig` / `BridgeConfig` / `StreamableConfig` / `WsConfig` / `McpSession` / `SessionStore`
- `CompiledSchema` / `PropertyDef`
- **Handler ABI**: `fn h(args_cstr, claims) → result_cstr` (the breaking change 2.0 made)

2.x may append fields at the tail of any struct. Any shape change
that removes / reorders fields or changes a fn signature triggers
3.0.

---

## Cyrius-language dependencies

Some bote work is gated on cyrius. Live language-level friction
(idioms, missing stdlib surface, cyrius patterns bote needs):
[docs/cyrius-feedback.md](../cyrius-feedback.md). Historical index of
resolved upstream issues bote reported + each fix landed:
[docs/resolved-lang-issues.md](../resolved-lang-issues.md).

Status against current cyrius (6.4.66):

| Issue | Status |
|---|---|
| `\r` escape correctness | ✅ Fixed in 4.4.0 |
| `&&` / `||` short-circuit | ✅ Fixed in 4.4.3 |
| Per-block local variable scoping | ✅ Fixed in 4.4.0 |
| Cascading parse errors from missing include | ✅ Fixed in 4.4.3 |
| `fmt_int` to stdout-only | ✅ Fixed in 4.4.3 (`fmt_int_fd` shipped) |
| `lib/http_server.cyr` stdlib primitive | ✅ Shipped in 4.5.0 (bote adopted in 1.3.0) |
| `lib/ws_server.cyr` stdlib primitive | ✅ Shipped in 4.5.1 (bote adopted in 1.5.0) |
| Identifier-buffer cap | ✅ Raised to 131072 bytes (4.6.2) |
| Function-table cap | ✅ Raised 2048 → 4096 (4.7.1) |
| `BUILD_METHOD_NAME` scratch corruption (misleading `lib/assert.cyr:3` error) | ✅ Fixed in 4.7.1 |
| `lib/base64.cyr` URL-safe variant | ✅ Shipped in 4.8.1 (bote adopted in 2.4.0) |
| Capacity meter (`CYRIUS_STATS=1` + `cyrius capacity` + `ERR_EXPECT` diagnostic) | ✅ Shipped in 4.8.3 |
| Path-traversal rejection on `../sibling` dep paths | ✅ Fixed in 4.8.4 |
| Include-once cap 64 → 256 | ✅ Raised in 4.8.4 |
| `PP_IFDEF_PASS` nested-include fixpoint | ✅ Shipped in 4.8.4 |
| 4.8.4 release-binary vs alpha2 skew | ✅ Closed by 2026-04-14 retag; bote 2.5.1 restored full dep-graph tests |
| `lib/http_server.cyr` folded into `lib/sandhi.cyr` (5.10.x) | ✅ Bridged in 2.6.0 via `src/_sandhi_compat.cyr` shim; retired in 2.6.1 |
| `lib/tls.cyr` required by sandhi for `TLS_EARLY_DATA_ACCEPTED` | ✅ Added to `[deps] stdlib` in 2.6.0 |
| `secret` is a storage-class keyword in 5.10.x | ✅ jwt.cyr parameter rename in 2.6.0 |
| Per-thread request buffers (process-global today) | 🟡 Tracked upstream; affects future threaded dispatch |
| Bump allocator without `fl_free` for general use | 🟡 Tracked; affects WS arena work |
| fn_table / identifier-buffer headroom at 88-89% with full integration | ✅ Relieved by the 6.1.x cap raise (2.7.3); 59% / 61% at 3.1.2 under the 2.6.4 CI capacity gate |

No current open bugs. Future reports land under `docs/bugs/` during
active triage and move to `docs/resolved-lang-issues.md` when closed.

---

## Non-goals (won't ship in any 1.x or 2.x)

- **Tool implementation** — bote dispatches to handlers, doesn't implement business logic.
- **LLM integration** — that's hoosh.
- **Workflow orchestration** — that's szal.
- **Agent lifecycle** — that's daimon.
- **Storage** — that's patra (libro for audit, patra for general).
- **Authorization server** — bote is the resource server. OAuth 2.1 AS flow belongs alongside bote, not inside.
