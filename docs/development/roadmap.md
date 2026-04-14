# Bote Roadmap

> **v0.1.0 (Cyrius port baseline)** — 12 modules, 3 transports, 251 tests, 10 benchmarks, 4 fuzz harnesses. Real MCP server in Cyrius.
>
> **Spec**: MCP 2025-11-25 | **Compliance**: [spec-compliance.md](../spec-compliance.md)

For shipped detail: [CHANGELOG.md](../../CHANGELOG.md). For Rust history: `rust-old/`.

---

## v0.2.0 — Bridge, Audit, Events

The first three modules from `rust-old/` to bring forward.

| Module | Effort | Notes |
|---|---|---|
| `src/bridge.cyr` | Medium | TypeScript-bridge MCP envelope + CORS preflight. Pure data layer + a small set of HTTP response shaping helpers; reuses `transport_http`. |
| `src/audit.cyr` | Medium | `AuditSink` trait equivalent (cyrius has no traits — use a function-pointer + context struct). `LibroAudit` adapter waits for a cyrius `lib/libro.cyr` (the libro AGNOS port). |
| `src/events.cyr` | Medium | `EventSink` analogue + topic constants (`bote/tool/completed`, `bote/tool/failed`, `bote/tool/registered`, `bote/tool/announce`). Adapter to `lib/majra.cyr` once that lands. |

---

## v0.3.0 — Discovery, Sandbox, Host

| Module | Effort | Notes |
|---|---|---|
| `src/discovery.cyr` | Low | `ToolAnnouncement`, `DiscoveryService`, subscribe-receiver. Trivial once `events` lands. |
| `src/sandbox.cyr` | High | Kavach integration. Needs cyrius `lib/kavach.cyr`. |
| `src/host.cyr` | High | MCP content blocks (text/image/audio/resource), host registry, SSRF check. |

---

## v0.4.0 — Streamable HTTP + Auth

| Module | Effort | Notes |
|---|---|---|
| `src/transport_streamable.cyr` | High | Single endpoint POST+GET, SSE event IDs, `Last-Event-ID` resumption, `retry:` hint. Needs cyrius SSE primitives. |
| `src/auth.cyr` | High | OAuth 2.1 / PKCE-S256 / bearer-token claims + middleware. Token validator function pointer on `HttpConfig`. |

---

## v0.5.0 — WebSocket, libro_tools

| Module | Effort | Notes |
|---|---|---|
| `src/transport_ws.cyr` | High | Server-side WebSocket. Cyrius `lib/ws.cyr` is client-side only — needs server handshake (Sec-WebSocket-Accept hash) and incoming-frame unmask. |
| `src/libro_tools.cyr` | Medium | 5 built-in MCP tools (`libro_query`, `libro_verify`, `libro_export`, `libro_proof`, `libro_retention`). Depends on audit + libro. |

---

## v0.6.0 — Streaming dispatch

The `stream` module currently has the data primitives (ProgressUpdate,
CancellationToken, ProgressSender). Threaded streaming dispatch via
`lib/thread.cyr` MPSC channels. Reach into `lib/async.cyr` for cancellation polling.

| Feature | Effort |
|---|---|
| `dispatcher_dispatch_streaming` | High |
| Streaming over stdio (progress notifications interleaved with final result) | Medium |
| Streaming over HTTP via SSE (waits on streamable transport) | Medium |
| `$/cancelRequest` handling | Low |

---

## v1.0.0 Criteria

- [ ] All Rust modules ported (bridge, audit, events, discovery, sandbox, host, auth, libro_tools)
- [ ] Streamable HTTP transport
- [ ] WebSocket transport (server-side)
- [ ] Streaming dispatch end-to-end
- [ ] Tests ≥ 400 unit + ≥ 50 conformance
- [ ] Benchmarks regression-tracked in CI
- [ ] At least 5 downstream cyrius consumers integrated (jalwa, shruti, tazama, rasa, daimon equivalents — once those are also ported)
- [ ] All cyrius-language pain points either fixed in cyrius or worked around with explicit comments

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
