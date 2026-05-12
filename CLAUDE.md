# Bote — Claude Code Instructions

## Project Identity

**Bote** (German: messenger) — MCP core service — JSON-RPC 2.0, tool registry, schema validation, dispatch

- **Language**: Cyrius (ported from Rust at v1.0.1; Rust archive preserved at tag `0.92.0`)
- **License**: GPL-3.0-only
- **Cyrius pin**: 5.10.44 (see `cyrius.cyml`; migration to 5.11.x planned)
- **Version**: SemVer 2.x stable on the handler ABI; 2.7.2 current
- **Genesis repo**: [agnosticos](https://github.com/MacCracken/agnosticos)
- **Philosophy**: [AGNOS Philosophy & Intention](https://github.com/MacCracken/agnosticos/blob/main/docs/philosophy.md)
- **Standards**: [First-Party Standards](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-standards.md)
- **Recipes**: [zugot](https://github.com/MacCracken/zugot) — takumi build recipes

## Stack

| Dep | Role |
|-------|------|
| libro 2.6.3 | Hash-linked audit chain (`[deps.libro]` git pin) |
| majra 2.4.4 | Pub/sub event publishing (`[deps.majra]` git pin) |
| kavach 3.0  | Tool sandboxing (pluggable runner via fn-pointer + ctx adapter) |

All AGNOS deps pinned in `cyrius.cyml [deps.<name>]` with `git` + `tag` (+ `path` for local dev). `lib/` is gitignored — `cyrius deps` rehydrates from the pinned tags. The contract is the pin, not the bytes on disk.

## Distribution

Two consumer bundles (see `DEPS-PATTERN.md` for the contract):

| Artifact | Profile | Modules | Use when |
|----------|---------|---------|----------|
| `dist/bote.cyr` | default `[lib]` | 23 | Consumer needs the full transport surface |
| `dist/bote-core.cyr` | `[lib.core]` | 9 | Consumer wraps Dispatcher / Registry / Audit but supplies its own transport (e.g. t-ron's SecurityGate) |

Regenerate with `cyrius distlib` (default) and `cyrius distlib core`. CI gates both bundles for freshness.

## Binaries

5.10.x cap workaround — per-transport binary split (reconsolidates on 5.11.x):

| Binary | Entry | Transports | Default port |
|--------|-------|------------|--------------|
| `build/bote` | `src/main.cyr` | stdio + http + unix + bridge | — / 8390 / — / 8391 |
| `build/bote-streamable` | `src/main_streamable.cyr` | Streamable HTTP / SSE | 8392 |
| `build/bote-ws` | `src/main_ws.cyr` | WebSocket | 8393 |

Build all three: `./scripts/build-all.sh`.

## Consumers

All consumer apps with MCP tools (phylax, t-ron, sutra, jalwa, rasa, mneme, etc.)

## Modules (src/)

**Core 9** — included in both `dist/bote.cyr` and `dist/bote-core.cyr`:

| Module | Purpose |
|--------|---------|
| `error.cyr` | `BoteError` tagged enum + JSON-RPC code mapping |
| `protocol.cyr` | `JsonRpcRequest` / `Response` / `Error` types |
| `jsonx.cyr` | JSON helpers (flat / nested / array accessors) |
| `registry.cyr` | `ToolRegistry` — registration, discovery, versioning, deprecation |
| `events.cyr` | `EventSink` — pub/sub trait |
| `audit.cyr` | `AuditLogger` / `AuditSink` — tool-call event trail |
| `dispatch.cyr` | `Dispatcher` (2.0 handler ABI: `fn h(args, claims) → result_cstr`) |
| `codec.cyr` | JSON-RPC encode / decode, batch processing |
| `schema.cyr` | JSON Schema compile + validate |

**Full bundle only** — included in `dist/bote.cyr`:

| Module | Purpose |
|--------|---------|
| `audit_libro.cyr` | libro chain audit-sink adapter |
| `events_majra.cyr` | majra pubsub event-sink adapter |
| `stream.cyr` | Streaming primitives (progress, cancellation) |
| `session.cyr` | MCP session store (validate_protocol_version, origin checks, lifecycle) |
| `discovery.cyr` | Cross-node tool discovery + announcement |
| `auth.cyr` | Bearer + allowlist + JWT HS256 + PKCE validators |
| `transport_stdio.cyr` | stdio transport |
| `transport_http.cyr` | HTTP transport |
| `transport_unix.cyr` | Unix domain socket transport |
| `bridge.cyr` | HTTP↔stdio TypeScript bridge with CORS |
| `transport_streamable.cyr` | Streamable HTTP / SSE transport |
| `transport_ws.cyr` | WebSocket transport (manually includes `lib/ws_server.cyr`) |
| `content.cyr` | Typed MCP content blocks + annotations |
| `host.cyr` | HostRegistry + IPv4/IPv6 SSRF guard + JSON config hot-reload |
| `libro_tools.cyr` | libro audit-tool dispatch (opt-in; not in default binary or bundle) |

**Binary entries** — `src/main.cyr` + `src/main_streamable.cyr` + `src/main_ws.cyr` + `src/main_common.cyr` (shared helpers).

## Development Process

### P(-1): Scaffold Hardening (before any new features)

0. Read roadmap, CHANGELOG, and open issues — know what was intended before auditing what was built
1. Test + benchmark sweep of existing code
2. Cleanliness check: `cyrius fmt --check src/*.cyr`, `cyrius lint src/main.cyr`, `cyrius audit`, `cyrius deny src/main.cyr`
3. Get baseline benchmarks (`./scripts/bench-log.sh`)
4. Initial refactor + audit (performance, memory, security, edge cases)
5. Cleanliness check — must be clean after audit
6. Additional tests/benchmarks from observations
7. Post-audit benchmarks — prove the wins
8. Repeat audit if heavy
9. Documentation audit — ADRs, source citations, guides, examples (see Documentation Standards in first-party-standards.md)

### Development Loop (continuous)

1. Work phase — new features, roadmap items, bug fixes
2. Cleanliness check: `cyrius fmt --check src/*.cyr`, `cyrius lint src/main.cyr`, `cyrius audit`, `cyrius deny src/main.cyr`
3. Test + benchmark additions for new code
4. Run benchmarks (`./scripts/bench-log.sh`)
5. Audit phase — review performance, memory, security, throughput, correctness
6. Cleanliness check — must be clean after audit
7. Deeper tests/benchmarks from audit observations
8. Run benchmarks again — prove the wins
9. If audit heavy → return to step 5
10. Documentation — update CHANGELOG, roadmap, docs, ADRs for design decisions, source citations for algorithms/formulas, guides and examples for new API surface, verify recipe version in zugot
11. Version check — VERSION, `cyrius.cyml` cyrius pin, recipe (in zugot) all in sync
12. Regenerate `dist/bote.cyr` + `dist/bote-core.cyr` if `src/` or `[lib]` / `[lib.core]` changed
13. Return to step 1

### Task Sizing

- **Low/Medium effort**: Batch freely — multiple items per work loop cycle
- **Large effort**: Small bites only — break into sub-tasks, verify each before moving to the next. Never batch large items together
- **If unsure**: Treat it as large. Smaller bites are always safer than overcommitting

### Refactoring

- Refactor when the code tells you to — duplication, unclear boundaries, performance bottlenecks
- Never refactor speculatively. Wait for the third instance before extracting an abstraction
- Refactoring is part of the work loop, not a separate phase. If a review reveals structural issues, refactor before moving on
- Every refactor must pass the same cleanliness + benchmark gates as new code

### Key Principles

- **Never skip benchmarks.** Numbers don't lie. The history log is the proof.
- **Tests + benchmarks are the way.** Aim to keep every public function exercised by `tests/bote_<module>.tcyr` or `tests/bote.tcyr`.
- **Own the stack.** If an AGNOS dep wraps an external lib, depend on the AGNOS dep — don't reach around it.
- **No magic.** Every operation is measurable, auditable, traceable.
- **Cyrius is single-pass.** Include order matters. New stdlib transitive deps go BEFORE the modules that reference them in `cyrius.cyml [deps] stdlib` (see the `ct` / `keccak` / `random` → `sigil` ordering for the worked example).
- **Compile-source budget.** The cyrius 5.10.x parser has a 2 MB cap on expanded source. Watch the `cyrius build` output for `expanded source exceeds 2MB`. Three response paths (mirroring 2.6.4 / 2.7.2): upstream cap raise (preferred — landed before), per-transport binary split (current 2.7.2 path), opt-in module split for consumers (`dist/bote-core.cyr`).
- **Function-table cap.** CI gates fn_table + identifier-buffer utilisation at < 95% (`CYRIUS_STATS=1`). At 2.7.2 we're at 93% / 92% on `src/main.cyr` — comfortable but bears watching.
- **No `unwrap()` / `panic!()` analog.** Library code returns 0 / -1 / error tags; consumer decides.
- **Feature-shape via `[lib.<profile>]`.** Don't invent feature flags in Cyrius — produce a separate dist bundle if a consumer subset is worth supporting.
- **`tracing` analog via libro / majra.** Audit goes to libro chain; events to majra pubsub. Wire via `dispatcher_set_audit` / `dispatcher_set_events`.

## Testing

| Test file | Assertions | Surface |
|-----------|-----------:|---------|
| `tests/bote.tcyr` | 363 | error / protocol / jsonx / registry / dispatch / codec / schema / stream / session / HTTP helpers / discovery / bridge / events / audit / audit_libro / events_majra wire-up |
| `tests/bote_auth.tcyr` | 38 | Bearer + allowlist + JWT HS256 + PKCE validators |
| `tests/bote_content.tcyr` | 24 | Typed MCP content blocks + annotations |
| `tests/bote_host.tcyr` | 113 | HostRegistry + IPv4/IPv6 SSRF + JSON config hot-reload |
| `tests/bote_jwt.tcyr` | 28 | JWT HS256 verify (header / payload / sig parsing) |
| `tests/bote_libro_tools.tcyr` | 22 | libro audit-tool dispatch surface (opt-in) |
| `tests/bote_pkce.tcyr` | 17 | RFC 7636 PKCE-S256 |
| `tests/bote_sandbox.tcyr` | 13 | kavach 3.0 pluggable runner adapter |
| `tests/bote_streamable.tcyr` | 25 | Streamable HTTP — EventIdGenerator / StreamEvent / ResumptionBuffer / StreamableConfig |
| `tests/bote_ws.tcyr` | 10 | WebSocket — WsConfig + handler wire-up |
| `tests/bote_core_only_smoke.tcyr` | drift guard | Includes only `dist/bote-core.cyr` — catches core/transport entanglement |
| **Total** | **653** | + 1 drift smoke |

Criterion benchmarks: **14** in `tests/bote.bcyr` (dispatch × 3, jsonx × 2, codec × 3, schema × 4, auth_bearer × 2).

```bash
cyrius deps                            # Resolve [deps.*] → lib/ (gitignored)
./scripts/build-all.sh                 # Build bote / bote-streamable / bote-ws
cyrius test tests/bote.tcyr            # Run a single test file
for f in tests/*.tcyr; do cyrius test "$f"; done  # All tests
cyrius bench tests/bote.bcyr           # Run benchmarks
./scripts/bench-log.sh                 # Benchmarks + append to benches/history.log
cyrius distlib                         # Regenerate dist/bote.cyr
cyrius distlib core                    # Regenerate dist/bote-core.cyr
```

## Documentation Structure

```
Root files (required):
  README.md, CHANGELOG.md, CLAUDE.md, CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, LICENSE, DEPS-PATTERN.md (distribution contract)

docs/ (required):
  architecture/overview.md — module map, data flow, consumers
  development/roadmap.md — completed, backlog, future, v1.0 criteria

docs/ (when earned):
  adr/ — architectural decision records
  guides/ — usage guides, integration patterns
  examples/ — worked examples
  standards/ — external spec conformance
  compliance/ — regulatory, audit, security compliance
  sources.md — source citations for algorithms/formulas (required for science/math crates)
```

## CHANGELOG Format

Follow [Keep a Changelog](https://keepachangelog.com/). Sections: Added, Changed, Fixed, Removed, Security, Performance.

- Every PR gets a CHANGELOG entry
- Performance claims MUST include benchmark numbers
- Breaking changes get a **Breaking** section with migration guide

## DO NOT

- **Do not commit or push** — the user handles all git operations (commit, push, tag)
- **NEVER use `gh` CLI** — use `curl` to GitHub API only
- Do not add unnecessary dependencies — keep it lean
- Do not panic/abort in library code — return 0 / -1 / error tags and let the consumer decide
- Do not skip benchmarks before claiming performance improvements
- Do not commit `build/` or `lib/` (both gitignored; `cyrius deps` rehydrates)
- Do not regenerate `dist/bote.cyr` or `dist/bote-core.cyr` without re-running `cyrius distlib` / `cyrius distlib core` — CI gates byte-clean diff vs the committed bundle
- Do not auto-inject heavyweight stdlib (sandhi / tls / sigil / ws_server) when only one module needs it — manual `include "lib/..."` in that one src file (see `transport_ws.cyr` for the pattern)
