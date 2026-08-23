# Bote — Claude Code Instructions

## Project Identity

**Bote** (German: messenger) — MCP core service — JSON-RPC 2.0, tool registry, schema validation, dispatch

- **Language**: Cyrius (ported from Rust at v1.0.1; Rust archive preserved at tag `0.92.0`)
- **License**: GPL-3.0-only
- **Cyrius pin**: 6.5.35 (see `cyrius.cyml`; `6.5.31 → 6.5.35` at 3.3.4; `6.5.20 → 6.5.31` at 3.3.2; onto the 6.5.x line at 3.2.0; `6.4.64 → 6.4.66` at 3.1.3; `6.4.34 → 6.4.64` at 3.1.2; onto the 6.4.x line at 3.0.1; `6.3.38 → 6.3.42` at 3.0.0; onto the 6.3.x line at 2.9.0; first 6.2.x at 2.7.6; major jump from 5.10.x at 2.7.3)
- **Version**: SemVer; 2.0 handler ABI (`fn h(args, claims) → result`) stable across the 2.x→3.x line; **3.3.4 current** (toolchain 6.5.35 + libro 2.8.12 / majra 2.7.0; fixes a SIGSEGV in `libro_verify` — libro 2.8.11 PREPENDED `magic` to `struct error`, shifting every field +8, and bote's raw-offset accessors read the old layout, so decoding a TAMPERED chain dereferenced an integer error code as a pointer; also repairs `_bote_server_version()`, which had reported 3.3.2 since 3.3.1. libro 2.8.10 at 3.3.3; cyrius 6.5.31 + the transitive patra downgrade at 3.3.2; `sys_accept4` + accept-loop error policy at 3.2.1; libro `LIBRO_ERR_*` namespacing at 3.1.4; `BOTE_ERR_*` error-tag namespacing at 3.1.3; web tools at 3.1.0; full MCP capability suite — prompts / resources / completion + polled list_changed push — at 3.0.0; see CHANGELOG)
- **Genesis repo**: [agnosticos](https://github.com/MacCracken/agnosticos)
- **Philosophy**: [AGNOS Philosophy & Intention](https://github.com/MacCracken/agnosticos/blob/main/docs/philosophy.md)
- **Standards**: [First-Party Standards](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-standards.md)
- **Recipes**: [zugot](https://github.com/MacCracken/zugot) — takumi build recipes

## Stack

| Dep | Role |
|-------|------|
| libro 2.8.12 | Hash-linked audit chain (`[deps.libro]` git pin). **patra 1.13.10** (`lib/patra.cyr`, the audit store) arrives via libro's own `[deps.patra]` manifest block — *not* via `dist/libro.deps`, which lists only stdlib leaves. As of 3.3.4 that pin is exactly level with the toolchain fold, closing the transitive-downgrade class 3.3.1/3.3.2 spent two releases unpicking. ⚠ **Struct-layout contract, and it has now bitten once.** 2.8.4 APPENDED a 5th field to `struct chain` (`streaming`, 32 → 40 B) — appended, so `src/libro_tools.cyr`'s raw +0 offset read was unaffected. 2.8.11 **PREPENDED** `magic` to `struct error` (48 → 56 B), shifting every field +8, and the raw accessors that read it silently returned the wrong fields → SIGSEGV on the tamper-report path (fixed 3.3.4). The rule: a field appended at the END is safe to ignore; a field inserted at the FRONT moves everything. `struct error` is now read through libro's `#derive(accessors)` getters (`error_code` / `error_msg` / `error_index`) rather than offsets; `struct chain` / `struct entry` keep raw offsets, where libro exposes no getters. ⚠ 2.8.11+ also changed the entry-hash preimage and the Merkle construction — see the 3.3.4 CHANGELOG for who must re-anchor (bote itself: nobody) |
| majra 2.7.0 | Pub/sub event publishing (`[deps.majra]` git pin). 2.7.0 is additive for bote: `pubsub_publish`'s signature and the 32-byte hub struct are unchanged and `PUBSUB_LAG_BLOCK` stays the default, so wire behaviour is identical. It removes the per-publish `map_keys()` allocation that ran on **every** publish (bump-allocated, never freed) — bote's exact path. New downstream capability: `pubsub_unsubscribe` + `PUBSUB_LAG_UNSUBSCRIBE` mean an abandoned subscriber can no longer wedge a bote publish forever |
| sigil 3.12.9 | Crypto (sha256 / hmac_sha256 / ed25519). ⚠ **No `[deps.sigil]` block — the pin was REMOVED at 3.3.1** and sigil now arrives via the cyrius stdlib fold. The pin had gone stale and was holding bote at 3.12.1, behind two authentication bypasses (3.12.5 PKCS#1 v1.5, 3.12.6 RSA-PSS); re-adding it would also reclassify sigil out of the stdlib leaves in `dist/bote.deps` and break clean-room consumers. **Ordering in `[deps] stdlib` is still load-bearing:** `ct` / `keccak` / `random` must precede `sigil` (single-pass forward refs), and **`thread_local` must precede `sigil`** — without TLS storage the binaries link clean and then SIGILL at first crypto use (exit 132) |
| sakshi 2.4.11 | Tracing / structured logging. ⚠ **No `[deps.sakshi]` block — removed at 3.3.1**; it arrives via the stdlib fold. bote references **zero** sakshi symbols directly; it is in `[deps] stdlib` only because libro links it. Do not remove that `"sakshi"` entry on the assumption libro's sidecar supplies it |
| kavach 3.12.2 | Tool sandboxing (pluggable runner via fn-pointer + ctx adapter). **Not a declared dep** — bote ships the adapter shape only and the consumer wires the backend, so there is no `[deps.kavach]` block. kavach's `ExecResult` still matches what `src/sandbox.cyr` documents, but its `sandbox_exec(sandbox, command)` is 2-arg with no timeout, so a consumer adapter must drop bote's `timeout_ms`. Do not add it as a dep: it pulls ai-hwaccel + samay. (The old "pins sigil 3.11.1, behind bote's" rationale is retired — kavach pins 3.12.9, the same sigil bote gets.) |

Only **libro** and **majra** have `[deps.<name>]` blocks, pinned with `git` + `tag` (+ `path` for local dev). Everything else arrives via the cyrius stdlib fold. `lib/` is gitignored — `cyrius lib sync --full` provisions the snapshot and `cyrius deps` OVERLAYS the declared deps on top. The contract is the pin, not the bytes on disk.

⚠ **A `[deps.<name>]` block overriding a folded stdlib module is a hazard, not a tool of first resort.** It was used historically for sigil and sakshi when the registry lagged; both went stale and were removed at 3.3.1 (sigil's held bote behind two auth bypasses). On a bundle-publishing library it also reclassifies the module OUT of the stdlib leaves in the `.deps` sidecars, breaking clean-room consumers.

⚠ **`path =` beats `tag =`, so a local checkout vendors the WORKING TREE and can mask a wrong tag.** Verify the vendored bytes against the tag before releasing, and verify transitive versions **after `cyrius build`, not after `cyrius deps`** — build does an implicit resolve that has silently reverted a vendored file before (3.3.1).

## Distribution

Two consumer bundles (see `DEPS-PATTERN.md` for the contract):

| Artifact | Profile | Modules | Use when |
|----------|---------|---------|----------|
| `dist/bote.cyr` | default `[lib]` | 30 | Consumer needs the full transport surface |
| `dist/bote-core.cyr` | `[lib.core]` | 12 | Consumer wraps Dispatcher / Registry / Prompts / Resources / Audit but supplies its own transport (e.g. t-ron's SecurityGate) |

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

**Core 12** — included in both `dist/bote.cyr` and `dist/bote-core.cyr`:

| Module | Purpose |
|--------|---------|
| `error.cyr` | `BoteError` tagged enum + JSON-RPC code mapping |
| `protocol.cyr` | `JsonRpcRequest` / `Response` / `Error` types |
| `jsonx.cyr` | JSON helpers (flat / nested / array accessors) |
| `registry.cyr` | `ToolRegistry` — registration, discovery, versioning, deprecation, profile tags |
| `prompts.cyr` | `PromptRegistry` — MCP prompts capability (`prompts/list` + `prompts/get`) |
| `resources.cyr` | `ResourceRegistry` — MCP resources capability (`resources/list` + `resources/read`) |
| `events.cyr` | `EventSink` — pub/sub trait |
| `audit.cyr` | `AuditLogger` / `AuditSink` — tool-call event trail |
| `dispatch.cyr` | `Dispatcher` (2.0 handler ABI: `fn h(args, claims) → result_cstr`) |
| `codec.cyr` | JSON-RPC encode / decode, batch processing |
| `schema.cyr` | JSON Schema compile + validate |
| `content.cyr` | Typed MCP content blocks + annotations (joined the core profile at 3.3.6 — content blocks are the tool-result format every handler emits, and hand-rolling them per consumer duplicates JSON escaping) |

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
| `transport_unix.cyr` | Unix domain socket transport (accept via `sys_accept4`; capped-backoff accept-error policy since 3.2.1) |
| `bridge.cyr` | HTTP↔stdio TypeScript bridge with CORS |
| `transport_streamable.cyr` | Streamable HTTP / SSE transport |
| `transport_ws.cyr` | WebSocket transport (manually includes `lib/ws_server.cyr`) |
| `host.cyr` | HostRegistry + IPv4/IPv6 SSRF guard + JSON config hot-reload |
| `libro_tools.cyr` | libro audit-tool dispatch (5 tools; in default binary + bundle since 2.7.5, not in core) |
| `fs_tools.cyr` | Filesystem MCP tools (`fs_write` / `fs_read` / `fs_mkdir`) — root-confined (`BOTE_FS_ROOT`); since 2.8.0 |
| `web_tools.cyr` | Web MCP tools (`web_fetch` / `web_search` via SearXNG, sandhi client); since 3.1.0 |
| `jwt.cyr` | JWT HS256 verifier (RFC 7519 / 7515) — exact `alg` field read + `exp` enforcement; `auth_validator_jwt_hs256` plugs into the bearer middleware. **In the bundle since 3.2.0** (orphaned before that) |
| `pkce.cyr` | RFC 7636 PKCE — `pkce_code_verifier` (getrandom) + `pkce_code_challenge_s256`. **In the bundle since 3.2.0** |

**Binary entries** — `src/main.cyr` + `src/main_streamable.cyr` + `src/main_ws.cyr` + `src/main_common.cyr` (shared helpers).

## Development Process

### P(-1): Scaffold Hardening (before any new features)

0. Read roadmap, CHANGELOG, and open issues — know what was intended before auditing what was built
1. Test + benchmark sweep of existing code
2. Cleanliness check: `for f in src/*.cyr; do cyrius fmt "$f" --check; done`, `cyrius lint src/main.cyr`, `cyrius vet src/main.cyr`, `cyrius deny src/main.cyr` (6.2.x: `fmt` takes one file at a time AND the flag now follows the file — `cyrius fmt <file> --check`, was `fmt --check <file>` on 6.1.x; `cyrius audit` was repurposed to the toolchain self-host gate — use `cyrius vet` for the include-dependency audit)
3. Get baseline benchmarks (`./scripts/bench-log.sh`)
4. Initial refactor + audit (performance, memory, security, edge cases)
5. Cleanliness check — must be clean after audit
6. Additional tests/benchmarks from observations
7. Post-audit benchmarks — prove the wins
8. Repeat audit if heavy
9. Documentation audit — ADRs, source citations, guides, examples (see Documentation Standards in first-party-standards.md)

### Development Loop (continuous)

1. Work phase — new features, roadmap items, bug fixes
2. Cleanliness check: `for f in src/*.cyr; do cyrius fmt "$f" --check; done`, `cyrius lint src/main.cyr`, `cyrius vet src/main.cyr`, `cyrius deny src/main.cyr` (6.2.x: `fmt` takes one file at a time AND the flag now follows the file — `cyrius fmt <file> --check`, was `fmt --check <file>` on 6.1.x; `cyrius audit` was repurposed to the toolchain self-host gate — use `cyrius vet` for the include-dependency audit)
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
- **Cyrius is single-pass.** Include order matters. New stdlib transitive deps go BEFORE the modules that reference them in `cyrius.cyml [deps] stdlib` (see the `ct` / `keccak` / `random` → `sigil` ordering for the worked example, and `thread` → `thread_local` → `sigil` for the 3.7.14 TLS-storage SIGILL guard added at 2.7.6).
- **Namespace fn names too, not just constants.** The flat "last definition wins" namespace covers **functions** as well as enum constants, and the winner is decided by include order — which a consumer controls, not bote. Worked example: `src/stream.cyr`'s `cancel_token_new()` collided with `lib/async.cyr:1058`'s from 3.3.0 until **3.3.5**, when the family was renamed `bote_cancel_token_*`. bote's own binaries were fine (`src/` is included after `lib/`, so bote's won) — but a consumer vendoring `dist/bote.cyr` and including `async.cyr` after it silently got async's body, with no diagnostic on their side. ⭐ The rename moved `fn_table` +1: under last-definition-wins the shadowed definition occupied no slot, so the collision was literally hiding a function. **`src/` is warning-clean as of 3.3.5 — keep it that way**, so a new `duplicate fn` warning is signal instead of noise in a known-warning baseline. (Two deps can still collide with each other and bote cannot fix that from here: `_sub_new` is defined by both `lib/libro.cyr` and `lib/majra.cyr`.)
- **Namespace global constants.** Enum constants share one flat global namespace with "last definition wins" — a bare tag silently takes a dep's value if both define it. `BoteErrTag` tags are prefixed `BOTE_ERR_*` (namespaced at 3.1.3) after bare `ERR_IO` / `ERR_JSON` collided with libro's own `ERR_IO=3` / `ERR_JSON=4` in any libro-linked binary. Prefix every new enum/const with `BOTE_` (or a module tag like `SCH_`) so it can't alias a stdlib/dep symbol.
- **Compile-source budget.** The cyrius 5.10.x parser had a 2 MB cap on expanded source that forced the per-transport binary split. **6.1.24 (2.7.3) raised it** — `src/main.cyr` builds clean and the cap is no longer the binding constraint. Still watch the `cyrius build` output for `expanded source exceeds`. Three response paths remain on record if it ever fires again: upstream cap raise (the 6.1.x path), per-transport binary split (the 2.7.2 path, still in place), opt-in module split for consumers (`dist/bote-core.cyr`).
- **Function-table cap.** CI gates fn_table + identifier-buffer utilisation at < 95% (`CYRIUS_STATS=1`). At 3.3.4 (cyrius 6.5.35) we're at **17% / 34%** on `src/main.cyr` (`fn_table 5585/32768`, `identifiers 180531/524288`, `var_table 2716/8192`). Against 3.3.3's freshly-resolved `5502 / 178561 / 2702` the growth is +83 fns, almost entirely **bayan 1.4.2 → 1.5.2**, whose 368 new fns land in the unreachable set. ⚠ **Re-measure both sides on freshly-resolved trees before quoting a delta** — a figure taken against a stale `lib/` is not a baseline (the pre-bump `lib/` read 5217 and could not be reproduced). The CI gate itself needs no edit — it parses the cap out of the `cyrius stats:` block rather than hardcoding it. Compare percentages only within a toolchain line: 6.4.75 raised the fn_table ceiling 8192 → 32768 and 6.4.76 the identifier pool 256 KB → 512 KB, so pre-6.4.75 percentages are not comparable.
- **A toolchain codegen A/B IS obtainable — run it on risky bumps.** Per-version compilers live at `~/.cyrius/versions/<V>/bin/cycc` and take source on **stdin**. `~/.cyrius/bin/cycc` is the installed one, so `cyrius build` always uses it regardless of the manifest pin (the pin selects the *stdlib snapshot*, not the compiler) — which means a "pre-bump baseline" built through the wrapper is NOT a pre-bump codegen baseline. To diff: reconstruct the wrapper's stdlib prelude from `[deps] stdlib` order, prepend it, and pipe the same bytes to each `cycc`. Includes are deduped, so a full prelude composes with a test file's own includes. Done at 3.3.4 for the 6.5.35 regalloc rewrite: 867/867 assertions agreed compiler-for-compiler, with 6.5.35 emitting a smaller binary every time.
- **No `unwrap()` / `panic!()` analog.** Library code returns 0 / -1 / error tags; consumer decides.
- **Feature-shape via `[lib.<profile>]`.** Don't invent feature flags in Cyrius — produce a separate dist bundle if a consumer subset is worth supporting.
- **`tracing` analog via libro / majra.** Audit goes to libro chain; events to majra pubsub. Wire via `dispatcher_set_audit` / `dispatcher_set_events`.

## Testing

| Test file | Assertions | Surface |
|-----------|-----------:|---------|
| `tests/bote.tcyr` | 424 | error / protocol / jsonx / registry / tool annotations + profiles / prompts / resources / completion / listChanged flag / dispatch / codec / schema / stream + notification builders / session (+ outbound slot) / HTTP helpers / discovery / bridge / events / audit / audit_libro / events_majra wire-up |
| `tests/bote_auth.tcyr` | 38 | Bearer + allowlist + JWT HS256 + PKCE validators |
| `tests/bote_content.tcyr` | 24 | Typed MCP content blocks + annotations |
| `tests/bote_fs_tools.tcyr` | 26 | Filesystem tools — path safety (`..` / absolute refusal), JSON unescape, root confinement |
| `tests/bote_host.tcyr` | 113 | HostRegistry + IPv4/IPv6 SSRF + JSON config hot-reload |
| `tests/bote_jwt.tcyr` | 53 | JWT HS256 verify — b64u decode, signature, exact `alg` field read (incl. the algorithm-confusion header), `exp` enforcement + malformed-claim rejection. Both gates mutation-proven |
| `tests/bote_libro_tools.tcyr` | 38 | libro audit-tool dispatch surface + (3.3.4) the `struct error` decode path on a TAMPERED chain and `libro_proof` on a POPULATED one — both were unreachable from any assertion before, which is why the 2.8.11 layout shift shipped a SIGSEGV. Mutation-proven |
| `tests/bote_pkce.tcyr` | 17 | RFC 7636 PKCE-S256 |
| `tests/bote_sandbox.tcyr` | 13 | kavach-shaped pluggable runner adapter |
| `tests/bote_transport_unix.tcyr` | 47 | Unix transport — `_unix_sockaddr` (incl. the 107-byte truncation clamp), accept-error policy `_unix_accept_action` + capped backoff, `sleep_ms` really blocks, `sys_accept4` arch guard. New at 3.2.1; module had no coverage before |
| `tests/bote_streamable.tcyr` | 53 | Streamable HTTP — EventIdGenerator / StreamEvent / ResumptionBuffer / SessionOutbound (per-session buffer + id gen) / GET drain selection / client-notification sink (tools + prompts list_changed) / POST-piggyback SSE / StreamableConfig |
| `tests/bote_web_tools.tcyr` | 27 | Web tools — scheme guard, HTML→text stripper (incl. control-byte/NUL drop), url-encode, entity decode |
| `tests/bote_ws.tcyr` | 10 | WebSocket — WsConfig + handler wire-up |
| `tests/bote_core_only_smoke.tcyr` | drift guard | Includes only `dist/bote-core.cyr` — catches core/transport entanglement |
| **Total** | **883** | + 1 drift smoke. Green on **x86_64**. ⚠ On **aarch64** the CROSS-BUILD is gated in CI and green, but the runtime sweep under `qemu-aarch64` 11.1.0 is PARTIAL — qemu does not pass `getrandom` through (`random_bytes(16)` returns a negative errno there, 16 natively), so `bote.tcyr`, `bote_pkce.tcyr` and `bote_streamable.tcyr` exit 90 on bote's fail-closed session-ID contract. 373 of 883 execute, 0 failed. This is an emulator limit, not a bote defect |

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
