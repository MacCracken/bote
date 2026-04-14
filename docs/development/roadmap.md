# Bote Roadmap

> **Current**: `1.9.2` (cyrius 4.7.0). 4,888 LOC `src/`, 519 unit
> assertions across 5 test files, 10 benchmarks, 4 fuzz harnesses,
> 6 transports, all major Rust v0.92.0 features ported plus net-new
> (streamable HTTP, WebSocket, content blocks, SSRF guard, env-driven
> bearer auth).
>
> **Spec**: MCP 2025-11-25 | **Compliance**: [spec-compliance.md](../spec-compliance.md)
>
> **Bench history**: [benchmarks-rust-v-cyrius.md](../benchmarks-rust-v-cyrius.md)

For shipped detail per release: [CHANGELOG.md](../../CHANGELOG.md).
For Rust history: git tag `0.92.0` (Rust archive retired in v1.0.1).

---

## Shipped

| Release | Headline |
|---|---|
| **1.0.0** | Cyrius port — protocol core, registry, dispatch, schema, codec, sessions, discovery, four transports |
| **1.0.1** | Retire `rust-old/` directory; preserve at git tag `0.92.0` |
| **1.1.0** | AuditSink + EventSink + dispatcher wire-up (sinks-noop default keeps zero overhead) |
| **1.2.0** | LibroAudit + MajraEvents adapters via `[deps.libro]` + `[deps.majra]` |
| **1.2.1** | Adapter init-dance docs + cstr→Str boundary fix |
| **1.3.0** | Adopt cyrius 4.5.0's stdlib `lib/http_server.cyr` (saved ~28 fns) |
| **1.4.0** | Streamable HTTP transport (MCP 2025-11-25): POST + GET SSE, Last-Event-ID resumption |
| **1.5.0** | WebSocket transport (RFC 6455) on stdlib `lib/ws_server.cyr` |
| **1.5.1** | P(-1) hardening: HTTP body-length clamp, `events_after` null guard, lint cleanup |
| **1.6.0** | `libro_tools` — 5 built-in MCP audit tools |
| **1.7.0** | Typed MCP content blocks (text / image / audio / resource / resource_link) |
| **1.8.0** | `HostRegistry` + `ssrf_check` (IPv4 blocklist) |
| **1.8.1** | Bump cyrius pin to 4.6.2 |
| **1.9.0** | Bearer-token middleware (RFC 6750) — opt-in, all four HTTP-family transports |
| **1.9.1** | IPv6 SSRF blocklist + `content_resource_blob` + `BOTE_BEARER_TOKENS` env-wired auth |
| **1.9.2** | Bump cyrius pin to 4.7.0 |

---

## Headed for v2.0

The 1.x line is feature-stable; v2.0 is the cleanup + completion ship.

### Must-have for 2.0

| Item | Effort | Status |
|---|---|---|
| **Security audit + repair** — 0-day / CVE-class external research and fixes (header injection, timing-safe token compare, JSON depth caps, request-size limits, error-message disclosure, etc.) | Medium | In progress |
| **`content_with_annotations`** (audience + priority MCP optional metadata on any block) | Low | Reverted from 1.9.1 — tipped the cyrius 4.5.1 / 4.6.2 identifier-buffer ceiling. Lands when 4.7.1 frees room. |
| **Claims propagation to handlers** — handler signature carries the validator's claims so handlers can authorize per-tool | Medium-High | Handler-ABI change; want one shot to land cleanly with the right shape. |
| **OAuth 2.1 / PKCE-S256** — token acquisition flow on top of the existing bearer substrate | High | Bearer middleware (1.9.0) already provides the validator surface. |
| **JWT verifier helper** (`auth_validator_jwt_hs256` / `_rs256`) — common case for production deployments | Medium | Will reuse `lib/sigil.cyr` for SHA-256; RS256 needs RSA. |
| **Final hardening sweep** (P(-1) audit) before tagging 2.0 | Medium | Standing process; runs once 4.7.1 + above land. |

### Nice-to-have for 2.0

| Item | Notes |
|---|---|
| **DNS resolution for hostname classification** in `ssrf_check` | A name resolving to `127.0.0.1` (e.g. `127.0.0.1.nip.io`) currently passes the conservative blocklist. Production deployments pair with a network policy; a cyrius DNS stub would let us catch it in-process. |
| **Block-level annotations propagation through `wrap_tool_result`** | Once `content_with_annotations` lands. |
| **`schema_compile` benchmark** | Startup cost matters for projects with many tools. |
| **`auth_bearer_check` benchmark** | Validate the no-overhead claim when validator unset. |
| **CHANGELOG migration to "[Unreleased]" section** | Conventional Keep-a-Changelog flow. |

### Deferred past 2.0 (waiting on cyrius / external)

| Item | Blocked by |
|---|---|
| **Threaded streaming dispatch** (`dispatcher_dispatch_streaming`) | cyrius `lib/thread.cyr` MPSC + `lib/async.cyr` cancellation polling firming up; data primitives (`ProgressUpdate`, `CancellationToken`) already in place. |
| **`$/cancelRequest` mid-stream handling** | Streaming dispatch first. |
| **WebSocket per-connection arena allocator** | Long-lived WS connections accumulate per-frame allocs in the bump allocator; needs `fl_free` support or arena-per-message lifetime. |
| **`kavach` v2 sandbox integration** | Kavach v2 hardening at a stable release. |
| **Live libro integration heisenbug** (v1.2.1 carried) | Heap-layout sensitivity; investigation pending. Does not affect the 1.6.0+ libro_tools (read-only). |
| **WS subprotocol negotiation** (`Sec-WebSocket-Protocol`) | Header is read but not enforced; consumers can inspect. |
| **WS per-message deflate** (RFC 7692) | Significant code (LZ77 + Huffman); will hang off a future `lib/dynlib.cyr` zlib binding. |
| **HostRegistry persistence / hot-reload** | Registry built in-process from config; no file watch yet. |

---

## Feature freeze

The 1.x line preserves the data shapes frozen in 1.0.0:
- `JsonRpcRequest` / `Response` / `Error`
- `ToolDef` (with `compiled` slot — additions allowed at the tail)
- `ToolSchema` / `ToolAnnotations`
- `BoteError` (12 tag variants — additions allowed at the tail)
- `HttpConfig` / `BridgeConfig` / `StreamableConfig` / `WsConfig` / `McpSession` / `SessionStore`
- `CompiledSchema` / `PropertyDef`

Config structs grew in 1.9.0 (bearer slots **appended** at the end of
each transport config — existing offsets preserved). 2.0 may make
breaking changes if claims-propagation requires a handler-signature
ABI change; that's the primary 2.0 reason.

---

## Cyrius-language dependencies

Some bote work is gated on cyrius. See
[docs/cyrius-feedback.md](../cyrius-feedback.md) for the full list with
reproductions, and [docs/bugs/](../bugs/) for active bug reports.

Status against current cyrius (4.7.0):

| Issue | Status |
|---|---|
| `\r` escape correctness | ✅ Fixed in 4.4.0 |
| `&&` / `||` short-circuit | ✅ Fixed in 4.4.3 |
| Per-block local variable scoping | ✅ Fixed in 4.4.0 |
| Cascading parse errors from missing include | ✅ Fixed in 4.4.3 |
| `fmt_int` to stdout-only | ✅ Fixed in 4.4.3 (`fmt_int_fd` shipped) |
| `lib/http_server.cyr` stdlib primitive | ✅ Shipped in 4.5.0 (bote adopted in 1.3.0) |
| `lib/ws_server.cyr` stdlib primitive | ✅ Shipped in 4.5.1 (bote adopted in 1.5.0) |
| Identifier-buffer cap (real projects hit it) | 🟡 Raised in 4.6.2; misleading-diagnostic case still hits bote occasionally — fix in flight for 4.7.1 |
| Per-thread request buffers (process-global today) | 🟡 Tracked upstream; affects future threaded dispatch |
| Bump allocator without `fl_free` for general use | 🟡 Tracked; affects WS arena work |

---

## Non-goals (won't ship in any 1.x or 2.x)

- **Tool implementation** — bote dispatches to handlers, doesn't implement business logic.
- **LLM integration** — that's hoosh.
- **Workflow orchestration** — that's szal.
- **Agent lifecycle** — that's daimon.
- **Storage** — that's patra (libro for audit, patra for general).
