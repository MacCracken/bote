# Bote Roadmap

> **Current**: `2.7.0` (cyrius 5.10.34, libro 2.6.2, majra 2.4.3).
> 8 active test files, **607 unit assertions**, **14 criterion
> benchmarks**, single-file `dist/bote.cyr` consumer bundle, CI
> capacity gate, annotations-preserving `wrap_tool_result`,
> 4 fuzz harnesses, 6 transports,
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

2.0 shipped. 2.x is feature-stable on the handler ABI (fn `(args, claims) → result_cstr`) and the six transports; patch releases add capabilities, not shape changes.

**2.6.x is the modernization arc.** Forward feature work that was
on the 2.6.x slate has shifted to 2.7.x. The 2.6.x line is reserved
for catching bote up to the first-party Cyrius floor: the dist-bundle
dep contract, the cyrius.cyml + `${file:VERSION}` layout, the
versioned-toolchain CI installer, the `cyrius deps --verify` /
`cyrius.lock` gates, and the residual libro 2.6.x / majra 2.4.x /
sandhi-HTTP-server porting that the 5.10.34 toolchain bump
surfaces. See the **2.6.x modernization arc** section below.

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

### Next candidates (no blockers)

| Item | Effort | Notes |
|---|---|---|
| **`HostRegistry` hot-reload from config file** | Medium | Useful for deployments that rotate allowed upstreams without a restart. Natural fit for 2.7.1. |
| **CONTRIBUTING.md Cyrius-era cleanup** | Low | Doc still references Rust-era commands (`make check`, `cargo-deny`, MSRV, `src/lib.rs`). Stale since the Cyrius port. Replace with `cyrius build` / `cyrius test` / `cyrius distlib` / `cyrius deps --verify` flow. |
| **OAuth 2.1 authorization-code flow** (bote-as-AS) | High | Out of scope for MCP core; bote is the resource server. Flagged as explicitly deferred — consumers compose bote with their own AS layer. |

### Blocked on cyrius / external

| Item | Waiting on |
|---|---|
| **Threaded streaming dispatch** (`dispatcher_dispatch_streaming`) | cyrius `lib/thread.cyr` MPSC + `lib/async.cyr` cancellation polling firming up. Data primitives (`ProgressUpdate`, `CancellationToken`) already in place. |
| **`$/cancelRequest` mid-stream handling** | Streaming dispatch first. |
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

Status against current cyrius (5.10.34):

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
| `lib/http_server.cyr` folded into `lib/sandhi.cyr` (5.10.x) | ✅ Bridged in 2.6.0 via `src/_sandhi_compat.cyr` shim; retire in 2.6.1 |
| `lib/tls.cyr` required by sandhi for `TLS_EARLY_DATA_ACCEPTED` | ✅ Added to `[deps] stdlib` in 2.6.0 |
| `secret` is a storage-class keyword in 5.10.x | ✅ jwt.cyr parameter rename in 2.6.0 |
| Per-thread request buffers (process-global today) | 🟡 Tracked upstream; affects future threaded dispatch |
| Bump allocator without `fl_free` for general use | 🟡 Tracked; affects WS arena work |
| fn_table / identifier-buffer headroom at 88-89% with full integration | 🟡 Tracked for 2.6.4 — split / feature-gate decision |

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
