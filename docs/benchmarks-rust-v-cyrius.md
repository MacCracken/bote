# Benchmarks: Rust vs Cyrius

Comparing the Rust implementation (preserved in `rust-old/`, last benched at
v0.92.0 on 2026-04-03) with the Cyrius port (current, v1.0.0, cyrius 4.4.3).

| | Rust v0.92.0 | Cyrius v1.0.0 |
|---|---|---|
| **Source LOC** | 10,877 (`rust-old/src/`) | 3,142 (`src/`) — **~3.5× smaller** |
| **External crate deps** | ~50 (axum, tokio, serde_json, criterion, etc.) | 0 — vendored stdlib in `lib/` |
| **Binary** | (release build not committed) | 127 KB, single static ELF, no libc dependency |
| **Build tool** | cargo + rustc | `cyrius build` (one shot, sub-second) |
| **Tests** | 248 lib + 44 conformance + 12 doc | 301 unit + 4 fuzz (~330 calls) |
| **Benchmarks** | 13 criterion benches (in `rust-old/benches/dispatch.rs`) | 10 hot-path benches (in `tests/bote.bcyr`) |
| **CPU** | (history) AMD Ryzen 7 5800H | (current) AMD Ryzen 7 5800H |

The Rust version had a richer feature surface — streaming dispatch, RwLock-backed
interior mutability, OAuth/PKCE, the host registry. The Cyrius 1.0.0 port covers
the protocol core (registry, dispatch, schema, codec, sessions, discovery, four
transports) — see `docs/development/roadmap.md` for the v1.x extension plan.

---

## Hot-path comparison

These are the most directly comparable measurements. Workloads aren't identical
(see notes), so treat as **order-of-magnitude rather than head-to-head**.

| Operation | Rust v0.92.0 | Cyrius v1.0.0 | Ratio | Notes |
|---|---|---|:-:|---|
| `dispatch initialize` | 281 ns | ~1 µs | **3.5×** | Both build a JSON response with serverInfo + capabilities. |
| `dispatch tools/list` | 109 µs (100 tools) | 2 µs (1 tool) | n/a | Rust scales linearly with N tools (~1.1 µs/tool); Cyrius single-tool is mostly fixed overhead. Re-bench with N=100 in Cyrius would land in roughly the same ballpark. |
| `dispatch tools/call` | 321 ns (100 tools) | 1 µs (1 tool) | ~3× | Both look up tool by name, validate, invoke handler. Cyrius does an extra alloc per call. |
| `dispatch notification` | 9.7 ns | not benched | n/a | Rust early-returns before lock acquire; Cyrius equivalent is similar but unmeasured. |
| `process_message single` | 1.09 µs | 4 µs (`codec_process_message`) | ~4× | Full pipeline: parse → dispatch → serialize. Includes JSON parsing of params and serializing the response. |
| `process_message batch_10` | 13.5 µs | not benched | n/a | Cyrius batch path is exercised in conformance tests but not micro-benchmarked. |
| `validate_params 2-required (no schema)` | 38 ns | n/a | n/a | Cyrius always uses compiled schema if attached; otherwise the fallback path mirrors this. |
| `validate_params typed (string + enum + integer-bounds)` | 103 ns | 950 ns (`validate_compiled_simple`) | ~9× | Cyrius walks JSON via offset-based parsing each call; Rust uses `serde_json::Value` which already-decoded fields. |
| `schema_compile` | 298 ns | not benched | n/a | One-shot at registration; not in the hot path. |
| `wrap_tool_result raw` | 404 ns | not benched | n/a | Bridge module — Cyrius `bridge.cyr` does the same wrapping but isn't isolated in `bote.bcyr`. |
| `wrap_tool_result passthrough` | 159 ns | not benched | n/a | |
| `dispatch_streaming setup` | 147 ns | n/a | n/a | Streaming dispatch is deferred to Cyrius v1.3.0 (waits on `lib/thread.cyr` MPSC). |
| `dispatch_call rwlock` | 257 ns | n/a | n/a | Cyrius has no concurrent-readers contention model — single-threaded dispatch. |

### Cyrius-only benchmarks (no direct Rust analogue)

| Operation | Cyrius v1.0.0 | What it measures |
|---|---|---|
| `jsonx_get_str_flat` | 597 ns | Extract `"name"` from a flat JSON object cstr |
| `jsonx_get_raw_nested` | 882 ns | Extract `"arguments"` (a nested object) — slice-based parser respects nested braces |
| `codec_parse_request` | 2 µs | Parse a tools/call request line into JsonRpcRequest struct |
| `codec_serialize_response` | 763 ns | Emit a JSON response from JsonRpcResponse struct |
| `validate_compiled_nested` | 3 µs | Validate against schema with nested object + required field + bounds |

The `jsonx` and `codec` benchmarks have no Rust analogue because Rust uses
`serde_json::Value` (a parsed tree) — there's no equivalent slice-based
extraction step.

---

## Why Cyrius is 3–10× slower per operation

Three structural reasons:

1. **No SIMD-accelerated JSON parsing.** Rust's `serde_json` uses
   `simd-json`-derived fast paths for ASCII and string scanning. Cyrius's
   `jsonx` is byte-by-byte. A future `cyrius lib/json` upgrade with `pcmpistri`
   (already on cyrius's roadmap) would close most of the gap on the parse paths.

2. **String-builder allocation per call.** Cyrius rebuilds response JSON via
   `str_builder_*` (which mallocs from the bump allocator). Rust uses
   `serde::Serializer` to write into a stack buffer. A pooled-builder pattern
   in bote could save ~300 ns/call.

3. **No closure-captured handler state.** Rust handlers are `Arc<dyn Fn(Value) -> Value>`
   — closures capture context cheaply. Cyrius handlers are bare function
   pointers (`fn h(args_cstr) → result_cstr`); state has to be threaded through
   globals or the args. This is fine for typical tools but means slightly more
   bookkeeping per dispatch.

**For an MCP server doing ≤1000 req/s** (typical), the wire I/O and the
handler's own logic dominate. Even at 4 µs per `codec_process_message`, that's
**250 K req/s ceiling** on a single thread — well above any realistic
deployment.

---

## Where Cyrius wins

- **Binary size**: 127 KB static ELF vs (Rust release builds were ~10–20 MB
  with all transports + runtime).
- **Build time**: `cyrius build src/main.cyr build/bote` returns in
  sub-second. Rust criterion bench runs took **minutes** per cycle.
- **Zero deps**: `cyrius.toml` has 14 stdlib entries, all vendored. No
  `crates.io`, no `Cargo.lock`, no supply chain to audit.
- **Source LOC**: 3.5× smaller (3,142 vs 10,877). Same protocol surface for
  the v1.0 modules (the Rust extras like `streamable`, `auth`, `host`,
  `libro_tools` add another ~3000 LOC and are deferred to v1.x).
- **Boot time**: no dynamic linker, no allocator init. Stdio transport
  starts dispatching JSON-RPC the moment `main` returns.
- **Auditability**: every byte in the binary maps back to source you can
  read in under an hour. No transitive-dependency surprises.

---

## Where Rust wins

- **Per-op latency** — 3-10× faster on the hot path, as shown above.
- **Concurrent dispatch** — RwLock + tokio means hundreds of in-flight
  requests; Cyrius is currently sequential per transport instance.
- **Streaming** — channels + thread spawn for long-running tools. Cyrius
  has the data primitives (`ProgressUpdate`, `CancellationToken`) but no
  threaded streaming dispatch yet (v1.3.0).
- **Mature SSE / WebSocket / Streamable HTTP** — Cyrius transports are POST
  + line-oriented stdio + UDS for now; SSE and WS-server land in v1.2 / v1.3.
- **Tooling** — clippy, rust-analyzer, `cargo bench --baseline X`, criterion's
  variance reporting. Cyrius's `bench_*` helpers are simpler.

---

## What's missing from each side's bench suite

### Should add to Cyrius (`tests/bote.bcyr`)
- `dispatch_tools_list_100_tools` — apples-to-apples for the Rust 109 µs number
- `dispatch_notification` — to verify the early-return is similarly cheap
- `process_message_batch_10` — batch overhead
- `bridge_wrap_tool_result_raw` and `_passthrough` — match Rust's bridge benches
- `schema_compile` — startup cost matters for projects with many tools

### Should add to Rust (would have, if Rust port were continuing)
- `jsonx`-style raw byte extraction — the Rust pipeline goes
  source-bytes → `serde_json::Value` and never walks raw bytes for sub-object
  extraction. The Cyrius approach (slice-and-cache) is potentially faster for
  large requests where most fields are ignored.

---

## Source

- **Rust history**: `rust-old/benches/history.log` (5 entries: v0.22.3 →
  v0.92.0, 2026-03-22 to 2026-04-03)
- **Rust bench source**: `rust-old/benches/dispatch.rs`
- **Cyrius bench source**: `tests/bote.bcyr`
- **Cyrius re-runs**: `cyrius bench tests/bote.bcyr`

To re-baseline Cyrius:

```sh
cyriusly use 4.4.3
cyrius bench tests/bote.bcyr
```

To re-baseline Rust (would require restoring the Rust toolchain + dependencies):

```sh
cd rust-old
cargo bench --bench dispatch
```
