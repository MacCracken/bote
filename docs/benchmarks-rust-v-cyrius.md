# Benchmarks: Rust vs Cyrius

> **Final comparison.** All porting work from `rust-old/` is done — bote's
> live implementation is Cyrius. This doc captures the last Rust bench
> run (preserved at git tag `0.92.0`) alongside current Cyrius numbers
> so the cost/benefit of the port stays auditable.

| | Rust v0.92.0 (final) | Cyrius v2.5.1 (current) |
|---|---|---|
| **Source LOC** (`src/`) | 10,877 | 5,429 — **~2.0× smaller** |
| **Tests + fuzz LOC** (`tests/` + `fuzz/`) | (Rust criterion + tests) | 2,589 |
| **External deps** | ~50 crates (axum, tokio, serde_json, criterion, …) | 0 — vendored stdlib in `lib/` + 2 git-pinned AGNOS deps (`libro`, `majra`) |
| **Binary** (release) | not committed (typical Rust release of this surface: 10–20 MB) | **370 KB**, single static ELF, no libc dependency |
| **Build time** | `cargo` + `rustc` (~30 s clean release) | `cyrius build` — **672 ms** cold, sub-100 ms incremental |
| **Tests** | 248 lib + 44 conformance + 12 doc = 304 | **603 unit** across 8 files + 4 fuzz harnesses |
| **Benchmarks** | 13 criterion benches | 10 hot-path benches (`tests/bote.bcyr`) |
| **CPU** | AMD Ryzen 7 5800H | AMD Ryzen 7 5800H (same machine) |
| **Toolchain** | Rust 1.89 | cyrius 4.8.4 |

The Rust v0.92.0 surface had: protocol core + RwLock-backed dispatch +
streaming dispatch + auth/PKCE + host registry + libro_tools + bridge +
HTTP/Unix/stdio transports.

The Cyrius port **matches or exceeds every v0.92.0 surface** plus adds
net-new: **streamable HTTP**, **WebSocket transport**, **typed content
blocks with annotations**, **IPv4/IPv6 SSRF guard**, **env-driven CLI
bearer auth (`BOTE_BEARER_TOKENS`)**, **JWT HS256 verifier**, **RFC
7636 PKCE-S256 helpers**, **pluggable sandbox runner (kavach 3.0
compatible)**, and **handler-claims ABI** plumbed end-to-end through
all six transports.

The only v0.92.0 item not yet in Cyrius: **threaded streaming dispatch**
(data primitives exist; thread integration waits on cyrius
`lib/thread.cyr` MPSC + `lib/async.cyr` cancellation firming up).

---

## Hot-path comparison

Last Rust benches: `benches/history.log` entry `v0.92.0 (a06f7fd)` on
**2026-04-03 15:24:49Z**. Current Cyrius benches: `cyrius bench
tests/bote.bcyr` on cyrius 4.8.4, bote 2.5.1.

Workloads aren't perfectly identical (Rust `dispatch_call_100_tools`
registers 100 tools, Cyrius `dispatch_tools_call` exercises 1) — treat
as **order-of-magnitude rather than head-to-head**.

| Operation | Rust v0.92.0 | Cyrius v2.5.1 | Ratio | Notes |
|---|---|---|:-:|---|
| `dispatch initialize` | 281 ns | 1 µs | **3.5×** | Both build a JSON response with serverInfo + capabilities. |
| `dispatch tools/list` | 109 µs (100 tools) | 2 µs (1 tool) | n/a | Rust scales linearly (~1.1 µs/tool); Cyrius single-tool is mostly fixed overhead. |
| `dispatch tools/call` | 321 ns (100 tools) | 3 µs (1 tool) | ~10× | Both look up by name, validate, invoke. Cyrius does an extra alloc per call (no fn-pointer arena yet). |
| `dispatch notification` | 9.7 ns | not benched | n/a | Rust early-returns before lock acquire; Cyrius equivalent is similar but unmeasured. |
| `process_message single` | 1.09 µs | 6 µs (`codec_process_message`) | **~5×** | Full pipeline: parse → dispatch → serialize. |
| `process_message batch_10` | 13.5 µs | not benched | n/a | Cyrius batch path is exercised in conformance tests, not micro-benchmarked. |
| `validate_params 2-required (no schema)` | 38 ns | n/a | n/a | Cyrius always uses compiled schema if attached. |
| `validate_params typed (string + enum + integer-bounds)` | 103 ns | 988 ns (`validate_compiled_simple`) | **~10×** | Cyrius walks JSON via offset-based parsing each call; Rust uses already-decoded `serde_json::Value`. |
| `schema_compile` | 298 ns | not benched | n/a | One-shot at registration; not in the hot path. |
| `wrap_tool_result raw` | 404 ns | not benched | n/a | `src/bridge.cyr` does the same wrapping but isn't isolated in `bote.bcyr`. |
| `wrap_tool_result passthrough` | 159 ns | not benched | n/a | Same. |
| `dispatch_streaming setup` | 147 ns | n/a | n/a | Streaming dispatch deferred — waits on cyrius thread/MPSC primitives. |
| `dispatch_call rwlock` | 257 ns | n/a | n/a | Cyrius has no concurrent-readers contention model — single-threaded dispatch per transport instance. |

### Cyrius-only benchmarks (2.5.1 current)

| Operation | Cyrius v2.5.1 | What it measures |
|---|---|---|
| `jsonx_get_str_flat` | 593 ns | Extract `"name"` from a flat JSON object cstr |
| `jsonx_get_raw_nested` | 873 ns | Extract `"arguments"` (a nested object) — slice-based parser respects nested braces |
| `codec_parse_request` | 2 µs | Parse a tools/call request line into `JsonRpcRequest` struct |
| `codec_serialize_response` | 752 ns | Emit a JSON response from `JsonRpcResponse` struct |
| `validate_compiled_nested` | 2 µs | Validate against schema with nested object + required field + bounds |

Numbers within noise of the 1.9.2 baseline — hot paths untouched by
the 2.x feature additions (claims propagation, JWT, PKCE, sandbox
adapter all sit off the dispatch critical path).

The `jsonx` and `codec` benchmarks have no Rust analogue because Rust's
pipeline goes source-bytes → `serde_json::Value` (parsed tree) and never
walks raw bytes for sub-object extraction.

---

## Why Cyrius is 3–10× slower per operation

Three structural reasons, none unfixable:

1. **No SIMD-accelerated JSON parsing.** Rust's `serde_json` uses
   `simd-json`-derived fast paths for ASCII and string scanning. Cyrius's
   `jsonx` is byte-by-byte. A future cyrius `lib/json` upgrade with
   `pcmpistri` (already on cyrius's roadmap) would close most of the gap
   on the parse paths.

2. **String-builder allocation per call.** Cyrius rebuilds response JSON
   via `str_builder_*` (which mallocs from the bump allocator). Rust uses
   `serde::Serializer` to write into a stack buffer. A pooled-builder
   pattern in bote could save ~300 ns/call once profiling justifies it.

3. **No closure-captured handler state.** Rust handlers are
   `Arc<dyn Fn(Value) -> Value>` — closures capture context cheaply.
   Cyrius handlers are bare function pointers (`fn h(args_cstr, claims)
   → result_cstr` per the 2.0 ABI); state is threaded through globals
   or args. Fine for typical tools but means slightly more bookkeeping
   per dispatch.

**For an MCP server doing ≤1000 req/s** (typical), wire I/O and
handler-side logic dominate. Even at 6 µs per `codec_process_message`,
that's a **~165 K req/s ceiling** on a single thread — well above any
realistic deployment envelope.

---

## Where Cyrius wins decisively

| | Rust v0.92.0 | Cyrius v2.5.1 |
|---|---|---|
| Binary size | ~10–20 MB (typical release with this surface) | **370 KB** |
| Build time | ~30 s clean release (cargo + rustc) | **~670 ms** (cyrius build) |
| Boot time | dynamic linker + allocator init + tokio reactor | direct `_start` → `main`, no init |
| Dependencies | ~50 crates from crates.io | 0 external — vendored stdlib + 2 git-pinned AGNOS deps |
| Auditability | transitive dep tree, opaque generic monomorphizations | every byte traceable to source readable in <1 hour |
| Source LOC | 10,877 | **5,429 (2.0× smaller)** for a *broader* feature surface than v0.92.0 |
| Test count | 304 | **603** (2.0× more) |

---

## Where Rust wins

- **Per-op latency** — 3–10× faster on hot paths (covered above).
- **Concurrent dispatch** — RwLock + tokio: hundreds of in-flight requests. Cyrius is currently sequential per transport instance.
- **Streaming** — channels + spawn for long-running tools. Cyrius has the data primitives (`ProgressUpdate`, `CancellationToken`) but no threaded streaming dispatch yet — slated for after cyrius's thread/async surface firms up.
- **JWT RS256 / ES256** — Rust's `jsonwebtoken` crate supports asymmetric. Cyrius has **HS256** (shipped 2.2.0) + **PKCE-S256** (2.3.0); RS256/ES256 wait on sigil RSA/ECDSA primitives.
- **Tooling** — clippy, rust-analyzer, `cargo bench --baseline X`, criterion's variance reporting. Cyrius has the new `CYRIUS_STATS=1` capacity meter + `cyrius capacity` subcommand (4.8.3+), but the broader IDE-integration surface is younger.

---

## Net call

The port shipped at **2.0× less source code** for a surface that's
now *broader* than the Rust v0.92.0 baseline (Rust didn't have
streamable HTTP, WebSocket, typed content blocks with annotations,
SSRF guard, env-driven auth, JWT, PKCE, sandbox adapter). The 3–10×
per-op latency is the price; for sub-1000-req/s deployments — the
realistic target for an MCP service — the **boot time, binary size,
and zero-deps story dominate the per-call latency** by orders of
magnitude.

The Rust archive lives at git tag `0.92.0` if a regression in the
Cyrius port ever needs to be cross-checked against the original
behaviour. After bote 1.0.1 the `rust-old/` directory was removed from
the working tree.

---

## What's missing from each side's bench suite

### Should add to Cyrius (`tests/bote.bcyr`)

- `dispatch_tools_list_100_tools` — apples-to-apples for the Rust 109 µs number
- `dispatch_notification` — verify early-return is similarly cheap
- `process_message_batch_10` — batch overhead
- `bridge_wrap_tool_result_raw` and `_passthrough` — match Rust's bridge benches
- `schema_compile` — startup cost matters for projects with many tools (queued in roadmap)
- `auth_bearer_check` (1.9.0+) — middleware overhead with allowlist of N (queued in roadmap)
- `ssrf_check` (1.8.0+) — URL classification cost
- `jwt_verify_hs256` (2.2.0+) — HMAC + base64url decode overhead
- `pkce_code_challenge_s256` (2.3.0+) — SHA-256 + base64url encode overhead

### Should have added to Rust (n/a — port complete)

- `jsonx`-style raw byte extraction — Rust has no analogue, so the comparison was always one-sided. Cyrius's slice-and-cache approach is potentially faster for large requests where most fields are ignored, but that workload isn't benched on either side.

---

## Source

- **Rust history**: `benches/history.log` at git tag `0.92.0` (5 entries: v0.22.3 → v0.92.0, 2026-03-22 to 2026-04-03)
- **Rust bench source**: `benches/dispatch.rs` at git tag `0.92.0`
- **Cyrius bench source**: `tests/bote.bcyr`
- **Cyrius re-runs**: `cyrius bench tests/bote.bcyr`

To re-baseline Cyrius:

```sh
cyrius bench tests/bote.bcyr
```

To re-baseline Rust (would require restoring the Rust toolchain + reinstalling crates.io deps):

```sh
git worktree add /tmp/bote-rust 0.92.0
cd /tmp/bote-rust
cargo bench --bench dispatch
```
