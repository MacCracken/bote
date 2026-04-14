# Bote Roadmap

> **v1.0.0 (Stable)** — 13 modules, 4 transports (stdio + HTTP + Unix + bridge), 298 tests, 10 benchmarks, 4 fuzz harnesses. Built and tested against cyrius 4.4.0. Data shapes frozen for the 1.x line.
>
> **Spec**: MCP 2025-11-25 | **Compliance**: [spec-compliance.md](../spec-compliance.md)

For shipped detail: [CHANGELOG.md](../../CHANGELOG.md). For Rust history: `rust-old/`.

---

## v1.1.0 — Audit + Events + Discovery wire-up + libro_tools

All four are now unblocked: **libro v1.0.3** and **majra v2.2.0** are sibling
cyrius projects. Wire them into `cyrius.toml` via `[deps.libro] path = "../libro"`
and `[deps.majra] path = "../majra"`.

| Module | Effort | Notes |
|---|---|---|
| `src/audit.cyr` + `LibroAudit` adapter | Medium | `AuditSink` analogue (function-pointer-based since cyrius has no traits). Wraps libro's `memstore_append` / hash chain. |
| `src/events.cyr` + `MajraEvents` adapter | Medium | Topic constants + sink fn-pointer; calls majra's `pubsub_publish` with serialized `ToolCallEvent`. |
| `src/discovery.cyr` wire-up | Low | Replace placeholder `publish_fp` with majra `pubsub_publish`; replace `DiscoveryReceiver` queue with majra `pubsub_subscribe`. |
| `src/libro_tools.cyr` | Medium | 5 built-in MCP tools (`libro_query`, `libro_verify`, `libro_export`, `libro_proof`, `libro_retention`). Direct calls to libro's `memstore_*` / `verify_*` functions. |

---

## v1.2.0 — Host, Auth, Streamable HTTP

| Module | Effort | Notes |
|---|---|---|
| `src/host.cyr` | High | MCP content blocks (text/image/audio/resource), host registry, SSRF check. No AGNOS deps. |
| `src/auth.cyr` | High | OAuth 2.1 / PKCE-S256 / bearer-token claims + middleware. Token-validator fn pointer on `HttpConfig`. No AGNOS deps. |
| `src/transport_streamable.cyr` | High | Single endpoint POST+GET, SSE event IDs, `Last-Event-ID` resumption, `retry:` hint. Builds on `transport_http`. |

---

## v1.3.0 — WebSocket + Sandbox + streaming dispatch

| Module | Effort | Notes |
|---|---|---|
| `src/transport_ws.cyr` | High | Server-side WebSocket. Cyrius `lib/ws.cyr` is client-side only — needs server handshake (Sec-WebSocket-Accept hash) + incoming-frame unmask. |
| `src/sandbox.cyr` + kavach integration | High | Waits for kavach v2-arch hardening to land at a stable release. |
| `dispatcher_dispatch_streaming` | High | `lib/thread.cyr` MPSC + `lib/async.cyr` cancellation polling. |
| Streaming over HTTP via SSE | Medium | Depends on streamable transport (1.2.0). |
| `$/cancelRequest` handling | Low | Wires into `CancellationToken`. |

---

## Feature freeze

The 1.x line preserves the data shapes frozen in 1.0.0:
- `JsonRpcRequest` / `Response` / `Error`
- `ToolDef` (with `compiled` slot — additions allowed at the tail)
- `ToolSchema` / `ToolAnnotations`
- `BoteError` (12 tag variants — additions allowed at the tail)
- `HttpConfig` / `BridgeConfig` / `McpSession` / `SessionStore`
- `CompiledSchema` / `PropertyDef`

Any change that requires removing a field, repurposing an offset, or changing
a function signature triggers a 2.0 major bump.

---

## Cyrius-language dependencies

Some bote work is gated on cyrius itself improving. See [docs/cyrius-feedback.md](../cyrius-feedback.md) for the full list with reproductions. Highlights:

- `\r` escape correctness (currently emits `r` byte 114 instead of CR byte 13)
- `&&` / `||` short-circuit (currently both sides always evaluated)
- Per-block local variable scoping (currently a single flat scope per `fn`)

These are not blockers — workarounds are in place — but bote becomes cleaner as cyrius fixes them.

---

## Non-goals

- **Tool implementation** — bote dispatches to handlers, doesn't implement business logic.
- **LLM integration** — that's hoosh.
- **Workflow orchestration** — that's szal.
- **Agent lifecycle** — that's daimon.
