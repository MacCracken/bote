# Bote — Claude Code Instructions

## Project Identity

**Bote** (German: messenger) — MCP core service — JSON-RPC 2.0, tool registry, schema validation, dispatch

- **Language**: Cyrius (ported from Rust at v1.0.1; Rust archive preserved at tag `0.92.0`)
- **License**: GPL-3.0-only
- **Cyrius pin**: 6.7.18 (`cyrius.cyml`; the pin selects both the compiler and the stdlib snapshot — every hop is a CHANGELOG entry)
- **Version**: SemVer; `VERSION` is the source of truth and `scripts/version-bump.sh` propagates it. The 2.0 handler ABI (`fn h(args, claims) → result`) is stable across the 2.x→3.x line; patch releases add capabilities, not shape changes. What each release did is in CHANGELOG; what is next is in `docs/development/roadmap.md` (forward-facing only)
- **Genesis repo**: [agnosticos](https://github.com/MacCracken/agnosticos)
- **Philosophy**: [AGNOS Philosophy & Intention](https://github.com/MacCracken/agnosticos/blob/main/docs/philosophy.md)
- **Standards**: [First-Party Standards](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-standards.md)
- **Recipes**: [zugot](https://github.com/MacCracken/zugot) — takumi build recipes

## Stack

| Dep | Role |
|-------|------|
| libro 2.11.3 | Hash-linked audit chain (`[deps.libro]` git pin). **patra 1.16.6** (`lib/patra.cyr`, the audit store) arrives via libro's own `deps.patra` manifest block — *not* via `dist/libro.deps`, which lists only stdlib leaves (just `hashmap` since libro 2.10.7). libro's pin equals the 6.7.18 fold's patra, and `cyrius deps` keeps the snapshot's copy; from 6.6.20 it says nothing, while earlier toolchains warned `refusing to overwrite stdlib leaf 'patra'`, which was the expected shape, not a defect. ⚠ **libro also declares a thin `deps.sigil` selection** (`dist/sigil-mldsa.cyr` + `src/sha256.cyr` / `sha_ni.cyr` / `hex.cyr`). From cyrius 6.6.11 the resolver keeps the stdlib fold over a thin profile of a package listed in `[deps] stdlib`, so none of it lands in bote's `lib/` (`cyrius deps` prints one "not vendored" note per module) and only the fold's `lib/sigil.cyr` is compiled. libro's sigil tag now decides only the sigil commit bote's lock pins, so keep it equal to the fold where libro allows; libro 2.11.3's 3.13.15 is the 6.7.18 fold's. Since cyrius 6.7.13 `distlib` credits that kept fold to libro, so `dist/bote-core.deps` no longer names `sigil` + `ct`, which no core module uses (3.3.17 recorded the cost, 3.4.11 what dropping them saved). Ship the sidecar as `distlib` writes it (CI gates it byte-for-byte). Through cyrius 6.6.10 the thin copies were vendored and won over the fold for ~236 functions under last-definition-wins: they diverged unnoticed at 3.3.8–3.3.9, and libro 2.10.3's 3.12.18 against the 3.13.4 fold was 5 hard arity errors (3.3.14). The CI step "libro's thin sigil matches the sigil fold" still fails if a thin copy is ever vendored again at a different version. ⚠ **Struct-layout contract, and it has now bitten once.** 2.8.4 APPENDED a 5th field to `struct chain` (`streaming`, 32 → 40 B) — appended, so `src/libro_tools.cyr`'s raw +0 offset read was unaffected. 2.8.11 **PREPENDED** `magic` to `struct error` (48 → 56 B), shifting every field +8, and the raw accessors that read it silently returned the wrong fields → SIGSEGV on the tamper-report path (fixed 3.3.4). The rule: a field appended at the END is safe to ignore; a field inserted at the FRONT moves everything. `struct error` is now read through libro's `#derive(accessors)` getters (`error_code` / `error_msg` / `error_index`) rather than offsets; `struct chain` / `struct entry` keep raw offsets, where libro exposes no getters. ⚠ 2.8.11+ also changed the entry-hash preimage and the Merkle construction — see the 3.3.4 CHANGELOG for who must re-anchor (bote itself: nobody) |
| majra 2.13.0 | Pub/sub event publishing (`[deps.majra]` git pin). `pubsub_new` / `pubsub_subscribe` / `pubsub_publish` signatures and the 32-byte hub struct are unchanged since 2.7.0 and `PUBSUB_LAG_BLOCK` stays the default, so wire behaviour is identical. 2.13.0's PostgreSQL SCRAM + TLS is in `-backends`, and 2.9.0's two **wire breaks** (encrypted-IPC handshake, signed-envelope domain separation) are in the `-backends` / `-signed` profiles bote does not consume; 2.8.0's `ERR_*` → `MAJRA_ERR_*` rename keeps same-value aliases; 2.8.1 fixed borrowed topic keys on the hub bote publishes into; 2.7.1 renamed `_sub_new` → `_majra_sub_new`, the libro collision cyrius 6.5.37 turned into a hard error (3.3.8). Since 2.7.0: no per-publish `map_keys()` allocation, and `pubsub_unsubscribe` + `PUBSUB_LAG_UNSUBSCRIBE` mean an abandoned subscriber cannot wedge a bote publish forever |
| sigil 3.13.15 | Crypto (sha256 / hmac_sha256 / ed25519). ⚠ **No `[deps.sigil]` block — the pin was REMOVED at 3.3.1** and sigil now arrives via the cyrius stdlib fold. The pin had gone stale and was holding bote at 3.12.1, behind two authentication bypasses (3.12.5 PKCS#1 v1.5, 3.12.6 RSA-PSS); re-adding it would also reclassify sigil out of the stdlib leaves in `dist/bote.deps` and break clean-room consumers. **Ordering in `[deps] stdlib` is still load-bearing:** `ct` / `keccak` / `random` must precede `sigil` (single-pass forward refs), and **`thread_local` must precede `sigil`** — without TLS storage the binaries link clean and then SIGILL at first crypto use (exit 132) |
| sakshi 2.5.10 | **bote's logger since 3.4.0** — `src/log.cyr` builds every operational line and hands it to sakshi (see the logging rule under Key Principles). ⚠ **No `[deps.sakshi]` block — removed at 3.3.1**; it arrives via the stdlib fold. libro logs through it too (61 calls), so `BOTE_LOG` governs libro's lines as well. Before 3.4.0 bote called **zero** sakshi symbols and its only diagnostic was one `eprint`. Do not remove the `"sakshi"` entry from `[deps] stdlib` |
| kavach 3.13.2 | Tool sandboxing (pluggable runner via fn-pointer + ctx adapter). **Not a declared dep** — bote ships the adapter shape only and the consumer wires the backend, so there is no `[deps.kavach]` block. kavach's `ExecResult` still matches what `src/sandbox.cyr` documents, but its `sandbox_exec(sandbox, command)` is 2-arg with no timeout, so a consumer adapter must drop bote's `timeout_ms`. Do not add it as a dep: it pulls ai-hwaccel + samay. (The old "pins sigil 3.11.1, behind bote's" rationale is retired — kavach 3.13.2 pins sigil 3.13.7; bote gets 3.13.15 from the 6.7.18 fold.) |

Only **libro** and **majra** have `[deps.<name>]` blocks, pinned with `git` + `tag`. No `path` line is committed. Since cyrius 6.7.6 a `path` beside `git` / `tag` is a development override: a default resolve ignores it, builds the tag and prints a "local checkout not in use" hint; `CYRIUS_LOCAL=<dep>` builds the sibling's working tree into `build/local-deps/lib/` and leaves `lib/` and `cyrius.lock` alone (both verified on bote at 3.4.0). Everything else arrives via the cyrius stdlib fold. `lib/` is gitignored — `cyrius lib sync --full` provisions the snapshot and `cyrius deps` OVERLAYS the declared deps on top. The contract is the pin, not the bytes on disk.

⚠ **A `[deps.<name>]` block overriding a folded stdlib module is a hazard, not a tool of first resort.** It was used historically for sigil and sakshi when the registry lagged; both went stale and were removed at 3.3.1 (sigil's held bote behind two auth bypasses). On a bundle-publishing library it also reclassifies the module OUT of the stdlib leaves in the `.deps` sidecars, breaking clean-room consumers.

⚠ **Through cyrius 6.7.5, `path =` beat `tag =`**, so a local checkout vendored the WORKING TREE and could mask a wrong tag; on the 6.7.18 pin a default build always builds the tag. Still verify transitive versions **after `cyrius build`, not after `cyrius deps`** — build does an implicit resolve that has silently reverted a vendored file before (3.3.1).

## Distribution

Two consumer bundles (see `DEPS-PATTERN.md` for the contract):

| Artifact | Profile | Modules | Use when |
|----------|---------|---------|----------|
| `dist/bote.cyr` | default `[lib]` | 35 | Consumer needs the full transport surface |
| `dist/bote-core.cyr` | `[lib.core]` | 13 | Consumer wraps Dispatcher / Registry / Prompts / Resources / Audit but supplies its own transport (e.g. t-ron's SecurityGate) |

Regenerate with `cyrius distlib` (default) and `cyrius distlib core`; `cyrius distlib --check` checks both. Both bundles state **cyrius ≥ 6.7.13**, on the `# Minimum cyrius:` line that opens `src/log.cyr` (their first module), measured by `scripts/bundle-min-cyrius.sh`; `bote-core.cyr` alone, with no transport, still builds from 6.7.2; a repo that copies the core bundle instead of pinning it must also declare `sakshi` (see DEPS-PATTERN.md). CI gates each bundle **and its `.deps` sidecar** for freshness. Since cyrius 6.6.18 a sidecar names only the leaves `distlib`'s compile-verify proves the bundle needs (`hashmap bayan ws_server syscalls` / `hashmap`), and each bundle opens with a matching `# Requires` include block.

## Binaries

One binary since 3.4.2, `build/bote` from `src/main.cyr`, the transport chosen by argv:

| Command | Transport | Default port | Tool set |
|---------|-----------|--------------|----------|
| `bote` / `bote stdio` | stdio | — | full |
| `bote http [port]` | HTTP | 8390 | full |
| `bote unix <path>` | Unix socket | — | full |
| `bote bridge [port]` | TS bridge with CORS | 8391 | full |
| `bote streamable [port]` | Streamable HTTP / SSE (+ session store, notification drain) | 8392 | echo references only |
| `bote ws [port]` | WebSocket | 8393 | echo references only |

"Full" is the echo / greeting / info references plus the libro, fs and web tools. streamable and ws keep the smaller set they had as separate binaries (2.7.2–3.4.1, a cyrius 5.10.x compile-source-cap workaround); changing that is an open roadmap decision, not a side effect of a refactor, and `scripts/transport-smoke.sh` fails if streamable gains `fs_write`. `bote-streamable` / `bote-ws` still work as argv[0] aliases (`bote_main_mode` matches the basename). `./scripts/build-all.sh` builds `build/bote` and links those two names to it; `./scripts/transport-smoke.sh` then exercises every mode (CI's "Transport smoke" step), including what a unit test cannot: a hostile Origin on each mode, and symlinks and FIFOs under `BOTE_FS_ROOT`.

**Targets.** Native x86_64 Linux; **aarch64** Linux (`cyrius build --aarch64`, the full suite runs under `qemu-aarch64`); **agnos** (`cyrius build --agnos` — a static x86-64 `ET_EXEC` over the agnos syscall ABI; the Unix transport fail-closes there because agnos has no `AF_UNIX`, the other five carry traffic). CI cross-builds the entry for both non-native targets and, since 3.3.12, also compiles every test / bench / fuzz unit for agnos with a zero-`undefined function` bar — cycc lowers an unreachable undefined fn to a `ud2` stub and still exits 0, so "OK" alone is not the bar. Runtime on agnos is proven on the kernel's QEMU image (`BOTE_SELFTEST` + `bote-mcp-smoke.sh` in the agnos repo), not in bote's CI.

## Consumers

**Consumers are not this project's problem unless they report a bug.** Do not scan sibling repos to judge a change, name a consumer as the reason for work, or size a change by who it reaches. Decide on the MCP spec and on bote's own correctness, security and performance. Keep the contract honest (`DEPS-PATTERN.md`, the stated minimum cyrius, the sidecars): that is bote's job, and who is on it is not. A bug a consumer reports is ordinary work: reproduce it in bote and fix it.

## Modules (src/)

**Core 13** — included in both `dist/bote.cyr` and `dist/bote-core.cyr`:

| Module | Purpose |
|--------|---------|
| `log.cyr` | Operational logging through sakshi (3.4.0) — stack-buffer lines, logfmt values with request-derived bytes escaped, `BOTE_LOG` / `BOTE_LOG_FILE` via `bote_log_init` (binaries only). FIRST in both profiles so every module can log |
| `error.cyr` | `BoteError` tagged enum + JSON-RPC code mapping |
| `protocol.cyr` | `JsonRpcRequest` / `Response` / `Error` types; (3.4.15) the request arena, `bote_req_alloc` / `bote_req_begin`; (3.6.3) `bote_req_note_token` / `bote_req_claims_kept`: a token that is the claims stays on the arena until a handler is handed it, once a message; (3.6.4) `bote_req_token`, and `bote_scope_token_ok`, the RFC 6749 scope-token check, moved here so `auth.cyr` can use it |
| `jsonx.cyr` | JSON helpers (flat / nested / array accessors) + `jsonx_valid`, the RFC 8259 syntax check the codec runs before anything reads a message (3.4.5) + `bote_json_escape`, the escaper every serializer uses, public and complete since 3.4.6 (`_json_emit_escaped` is its alias) + (3.5.2) `_jx_members`, for a schema's `properties`, where the keys are not known |
| `registry.cyr` | `ToolRegistry` — registration, discovery, versioning, deprecation, profile tags; (3.4.8) `registry_check_tool_def` refuses a malformed definition and registration compiles the input schema; (3.5.2) registration reads each `x-mcp-header` into `tool_def_param_headers` and refuses one that breaks the spec's rules; (3.6.3) `tool_def_with_required_scopes`, each an RFC 6749 scope-token (`bote_scope_token_ok`, in `protocol.cyr`) |
| `prompts.cyr` | `PromptRegistry` — MCP prompts capability (`prompts/list` + `prompts/get`) |
| `resources.cyr` | `ResourceRegistry` — MCP resources capability (`resources/list` + `resources/read`); (3.5.3) resource templates, RFC 6570 level 1: `resource_template_check`, `_rt_match`, `resource_template_var_a` |
| `events.cyr` | `EventSink` — pub/sub trait |
| `audit.cyr` | `AuditLogger` / `AuditSink` — tool-call event trail |
| `dispatch.cyr` | `Dispatcher` (2.0 handler ABI: `fn h(args, claims) → result_cstr`); (3.4.8) every handler return is coerced to a CallToolResult (`_dsp_coerce_result`); (3.5.0) `dispatcher_dispatch_a` is the era router: `_meta` selects MCP 2026-07-28 per request, an `initialize` selects legacy, and `_dsp_serve_a` holds the methods for both; (3.6.3) `dispatcher_set_scope_check` / `dispatcher_scopes_missing`: a `tools/call` lacking a scope is refused with `-31403` after its name and ahead of its version and arguments, so a batch and a WebSocket frame are covered; claims of 0 compare nothing; scopes with no checker fail closed |
| `codec.cyr` | JSON-RPC encode / decode, batch processing |
| `schema.cyr` | JSON Schema compile + validate; numbers per 2020-12 (`_sch_num_parse`, 3.4.8) |
| `content.cyr` | Typed MCP content blocks + annotations (joined the core profile at 3.3.6 — content blocks are the tool-result format every handler emits, and hand-rolling them per consumer duplicates JSON escaping) |

**Full bundle only** — included in `dist/bote.cyr`:

| Module | Purpose |
|--------|---------|
| `audit_libro.cyr` | libro chain audit-sink adapter |
| `events_majra.cyr` | majra pubsub event-sink adapter |
| `stream.cyr` | Streaming primitives (progress, cancellation) |
| `session.cyr` | MCP session store (validate_protocol_version, origin checks, lifecycle); (3.5.2) `bote_origin_self`: with no list set, the server's own loopback address passes |
| `discovery.cyr` | Cross-node tool discovery + announcement |
| `auth.cyr` | Bearer + allowlist + JWT HS256 + PKCE validators; (3.6.3) `auth_send_insufficient_scope`, the 403's challenge, which leaves out what is no scope-token (3.6.4); (3.6.4) `bote_http_refuse_hdr` / `auth_send_challenge_hdr` / `auth_bearer_check_hdr`, the senders with extra headers |
| `http_era.cyr` | (3.5.1) Which era a POST belongs to, on `bote http`, `bote streamable` and (3.5.2) `bote bridge`: `bote_http_era` reads `MCP-Protocol-Version` and `_meta` once the body is in hand and refuses, takes the legacy path, or takes `bote_http_serve_modern`. It holds 2026-07-28's `Mcp-Method` / `Mcp-Name` / (3.5.2) `Mcp-Param-{Name}` checks, the Base64 sentinel, the refusal of a field on two lines, and the status code each error goes out with. Every refusal carries a JSON-RPC error, which is how a client tells a modern server from a legacy one. A context from `bote_http_ctx` carries the socket, the request and any extra response headers |
| `oauth_metadata.cyr` | (3.6.0) OAuth 2.0 Protected Resource Metadata, RFC 9728, on `http`, `streamable` and `bridge`: `bote_prm_check` / `bote_prm_new` build the document and the 401's challenge once at startup, from what the operator configures and never from `Host`; `bote_prm_route` serves it ahead of the bearer check and behind Origin. bote is the resource server and names the authorization servers; it issues nothing. (3.6.3) `bote_http_scope_gate`, after the era check, turns a lone `tools/call` the token may not make into a `403` naming every scope and the metadata; the check itself is the dispatcher's |
| `transport_stdio.cyr` | stdio transport |
| `transport_http.cyr` | HTTP transport |
| `transport_unix.cyr` | Unix domain socket transport (accept via `sys_accept4`; capped-backoff accept-error policy since 3.2.1) |
| `bridge.cyr` | HTTP↔stdio TypeScript bridge with CORS; (3.6.3) exposes `WWW-Authenticate`; (3.6.4) every refusal past the Origin check carries the CORS headers, the 401 first among them, without which a page sees a network error |
| `transport_streamable.cyr` | Streamable HTTP / SSE transport |
| `transport_ws.cyr` | WebSocket transport (manually includes `lib/ws_server.cyr`); (3.6.4) `_ws_upgrade_ok` judges the four headers `ws_server_handshake` copies to the heap, on the arena first, and the connection's token lives in `_ws_token_buf`, so a refused upgrade keeps nothing and a connection keeps only the handshake's ~1.6 KB (a cyrius issue) |
| `host.cyr` | HostRegistry + IPv4/IPv6 SSRF guard + JSON config hot-reload; (3.4.7) IPv6 literals parsed to bytes (`_ssrf_parse_ipv6`) and judged by the same classifier as resolved AAAA answers |
| `libro_tools.cyr` | libro audit-tool dispatch (5 tools; in default binary + bundle since 2.7.5, not in core) |
| `fs_tools.cyr` | Filesystem MCP tools (`fs_write` / `fs_read` / `fs_mkdir`) — root-confined (`BOTE_FS_ROOT`); since 2.8.0 |
| `web_tools.cyr` | Web MCP tools (`web_fetch` / `web_search` via SearXNG, sandhi client); since 3.1.0; (3.4.4) `web_fetch` classifies the resolved address inside sandhi's client resolve hook, installed only around its request |
| `jwt.cyr` | JWT access tokens, HS256, RS256 and ES256 (RFC 7519 / 7515 / 7518) — exact `alg` field read, `crit` refused, `exp` and `nbf` enforcement; `auth_validator_jwt` plugs into the bearer middleware, and with an audience on the JwtSecret requires `aud` to name it, `exp`, and `iss` among the issuers, which is MCP's "issued for them", from `BOTE_JWT_SECRET` or `BOTE_JWKS_FILE` in the binary. (3.6.1) `_jwt_b64u_a` decodes on the request arena and takes one spelling, so a forged token keeps nothing. (3.6.2) `jwt_secret_from_jwks` loads public keys: a handle holds a secret or keys and never both, and each key has one algorithm, which is what rules out algorithm confusion. (3.6.3) `jwt_scope_check` reads `scope`, or `scp` as a string or an array, exactly; `jwt_scope_rule` says what a broader scope implies, each rule followed once. (3.6.4) tokens already verified: `jwt_secret_with_cache`, keyed by the SHA-256 of the token, `exp` and `nbf` judged again by `_jwt_window_ok` on every hit, never a failure stored, forgotten on a setter; `BOTE_JWT_CACHE` in the binary; `_bote_jwt_clock` for tests. **In the bundle since 3.2.0** (orphaned before that) |
| `conn.cyr` | Per-connection deadlines every transport applies (3.4.3): `BOTE_SEND_TIMEOUT_MS` (default 30000, `SO_SNDTIMEO` on every connection; a failed write drops the client) and `BOTE_UNIX_IDLE_MS` (default 0 — opt-in, because the Unix transport is persistent, the 3.3.7 WebSocket idle-drop lesson). (3.4.12) `bote_conn_serve`, the serve entry the four sandhi-served transports share: a failed start logs its step and errno. Ahead of the transports in `[lib]`; not in core |
| `pkce.cyr` | RFC 7636 PKCE — `pkce_code_verifier` (getrandom) + `pkce_code_challenge_s256`. **In the bundle since 3.2.0** |
| `sandbox.cyr` | Pluggable sandbox runner — kavach-shaped fn-pointer + ctx adapter, noop default, error envelope on a null runner. **In the bundle since 3.3.13**; it had been in `src/` and in no profile since 2.1.0, so no consumer could reach a `sandbox_*` symbol across thirteen minor lines — the same orphan shape `jwt.cyr` / `pkce.cyr` had until 3.2.0. Deliberately NOT in `[lib.core]`: eight repos vendor the core bundle and none references a `sandbox_*` symbol (rationale recorded in `cyrius.cyml`). ⭐ CI now fails on any `src/` module that is in no `[lib*]` profile and is not a declared entry point — the gate that would have caught this and the 3.2.0 pair |

**Binary entry** — `src/main.cyr` + `src/main_common.cyr` (entry-side helpers: reference dispatcher, mode selection, the binary's log lines). Neither is in a `[lib]` profile.

## Development Process

### P(-1): Scaffold Hardening (before any new features)

0. Read roadmap, CHANGELOG, and open issues — know what was intended before auditing what was built
1. Test + benchmark sweep of existing code
2. Cleanliness check: `for f in src/*.cyr; do cyrius fmt "$f" --check; done`, `cyrius lint src/main.cyr`, `cyrius vet src/main.cyr`, `cyrius deny src/main.cyr` — `fmt` is per-file with the flag after the path; `cyrius vet` is the include-dependency audit (`cyrius audit` is the toolchain's own self-host gate, not ours); plus the raw-`syscall(` grep CI runs over `src/` + `tests/` + `fuzz/`
3. Get baseline benchmarks (`./scripts/bench-log.sh`)
4. Initial refactor + audit (performance, memory, security, edge cases)
5. Cleanliness check — must be clean after audit
6. Additional tests/benchmarks from observations
7. Post-audit benchmarks — prove the wins
8. Repeat audit if heavy
9. Documentation audit — ADRs, source citations, guides, examples (see Documentation Standards in first-party-standards.md)

### Development Loop (continuous)

1. Work phase — new features, roadmap items, bug fixes
2. Cleanliness check: `for f in src/*.cyr; do cyrius fmt "$f" --check; done`, `cyrius lint src/main.cyr`, `cyrius vet src/main.cyr`, `cyrius deny src/main.cyr` — `fmt` is per-file with the flag after the path; `cyrius vet` is the include-dependency audit (`cyrius audit` is the toolchain's own self-host gate, not ours); plus the raw-`syscall(` grep CI runs over `src/` + `tests/` + `fuzz/`
3. Test + benchmark additions for new code
4. Run benchmarks (`./scripts/bench-log.sh`)
5. Audit phase — review performance, memory, security, throughput, correctness
6. Cleanliness check — must be clean after audit
7. Deeper tests/benchmarks from audit observations
8. Run benchmarks again — prove the wins
9. If audit heavy → return to step 5
10. Documentation — update CHANGELOG, roadmap, docs, ADRs for design decisions, source citations for algorithms/formulas, guides and examples for new API surface, verify recipe version in zugot
11. Version check — VERSION, `cyrius.cyml` cyrius pin, recipe (in zugot) all in sync; at a pin move, or when bote starts using a newer language feature, re-run `./scripts/bundle-min-cyrius.sh` and keep the `# Minimum cyrius:` line that opens `src/log.cyr` equal to what it measures (CI checks only that both bundles carry the same line and that it is not newer than the pin)
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
- **Namespace fn names too, not just constants.** The flat "last definition wins" namespace covers **functions** as well as enum constants, and the winner is decided by include order — which a consumer controls, not bote. Worked example: `src/stream.cyr`'s `cancel_token_new()` collided with `lib/async.cyr:1058`'s from 3.3.0 until **3.3.5**, when the family was renamed `bote_cancel_token_*`. bote's own binaries were fine (`src/` is included after `lib/`, so bote's won) — but a consumer vendoring `dist/bote.cyr` and including `async.cyr` after it silently got async's body, with no diagnostic on their side. ⭐ The rename moved `fn_table` +1: under last-definition-wins the shadowed definition occupied no slot, so the collision was literally hiding a function. **`src/` is warning-clean as of 3.3.5 — keep it that way**, so a new `duplicate fn` warning is signal instead of noise in a known-warning baseline. (Two deps can still collide with each other and bote cannot fix that from here. `_sub_new` — libro vs majra — was one until majra 2.7.1 renamed it; libro's thin sigil selection vs the fold's `lib/sigil.cyr` was one, 232 names, until cyrius 6.6.11 stopped vendoring the thin copy; see the libro row in the stack table.)
- **Namespace global constants.** Enum constants share one flat global namespace with "last definition wins" — a bare tag silently takes a dep's value if both define it. `BoteErrTag` tags are prefixed `BOTE_ERR_*` (namespaced at 3.1.3) after bare `ERR_IO` / `ERR_JSON` collided with libro's own `ERR_IO=3` / `ERR_JSON=4` in any libro-linked binary. Prefix every new enum/const with `BOTE_` (or a module tag like `SCH_`) so it can't alias a stdlib/dep symbol. **The rule cuts the other way too:** a bare stdlib name bote *reads* can be supplied by a dep on a target where the stdlib doesn't define it. Worked example (3.3.12): `transport_unix.cyr` used bare `EINTR` / `EAGAIN` / `EINVAL` / `EMFILE`; the agnos syscall peer defines no errno names, and the four resolved anyway — from `lib/sigil.cyr:44`'s own bare errno enum in the injected prelude, correct only because sigil spells Linux's numbers. `ENFILE`, absent from sigil's enum, is what finally failed the build. The module now spells all twelve `BOTE_ERRNO_*` it classifies. When a bare stdlib constant compiles on a target whose peer you know lacks it, find out who is really defining it.
- **Compile-source budget.** Watch `cyrius build` for `expanded source exceeds`. It has not fired since the 6.1.24 cap raise, but three response paths are on record if it does: an upstream cap raise, a per-transport binary split (the shape bote carried from 2.7.2 to 3.4.1 for the 5.10.x cap, folded back at 3.4.2), and an opt-in module split for consumers (`dist/bote-core.cyr`).
- **Function-table cap.** CI gates fn_table + identifier-buffer utilisation at < 95% (`CYRIUS_STATS=1`), parsing the cap out of the `cyrius stats:` block rather than hardcoding it. At 3.6.4 (cyrius 6.7.18) `src/main.cyr` sits at `fn_table 7192/131072`, `identifiers 225481/8388608`, `var_table 3300/1048576` — **5.5% / 2.7% / 0.3%**. Two rules: compare percentages only within a toolchain line (6.4.75 / 6.4.76 and the 6.6.x line each raised the ceilings, so older percentages are not comparable), and ⚠ **re-measure both sides on freshly-resolved trees before quoting a delta** — a figure taken against a stale `lib/` is not a baseline. Growth between releases is usually a dep's unreachable set (bayan's, at 3.3.4), not bote.
- **A pin move resolves from scratch: `rm -rf lib cyrius.lock && cyrius lib sync --full && cyrius deps`, in that order.** `lib sync` goes before any `deps` / `build`: from cyrius 6.6.17 a `deps` or `build` run first stamps the new pin over the lock rows of files `deps` does not vendor, and `lib sync` then refuses (it names `--relock`). The lock is deleted because from 6.6.20 it keeps a commit pin once recorded: re-resolving over the old lock at 3.3.17 left the previous libro / majra / sigil / patra rows beside the new ones. Then check `cyrius deps --verify` and the four `commit` rows.
- **A toolchain codegen A/B IS obtainable — run it on risky bumps.** Per-version toolchains live at `~/.cyrius/versions/<V>/` and **the `cyrius` wrapper re-execs the pinned one** (since 6.6.4: `cyrius which` inside the repo resolves to `~/.cyrius/versions/<pin>/bin/cycc`, not `~/.cyrius/bin/cycc`), so editing the `cyrius =` pin selects both the compiler and the stdlib snapshot. Recipe: build once at the old pin (`git show HEAD:cyrius.cyml > cyrius.cyml`, then `rm -rf lib && cyrius lib sync --full && cyrius deps`), keep that binary, restore the new pin and `lib/`, rebuild, run the two interleaved. ⚠ `path =` siblings vendor their WORKING TREE on both sides, so this is a toolchain control, not a previous-release reproduction, unless the siblings are checked out at the old tags. Done at 3.3.10 for 6.6.3 → 6.6.6 (14 benches, a wash: dispatch −3…−7%, schema +1…+3%), and — by stdin-prelude reconstruction, which the pre-6.6.4 wrapper needed because it ignored the pin for the compiler — at 3.3.4 for the 6.5.35 regalloc rewrite: 867/867 assertions agreed compiler-for-compiler.
- **No `unwrap()` / `panic!()` analog.** Library code returns 0 / -1 / error tags; consumer decides.
- **Log through `bote_log_*`, never `eprint` / `println` / a raw write to fd 2** (3.4.0). Every operational line goes through `src/log.cyr` to sakshi. The rules: (1) **library code never configures sakshi** — only a binary's `main` calls `bote_log_init` (`BOTE_LOG`, `BOTE_LOG_FILE`); an embedder's sakshi settings win; (2) **never stdout** — on the stdio transport it is the JSON-RPC wire; CI's "Logging smoke" step fails if a line lands there; (3) **never a secret** — no token, JWT, body, tool argument or full URL (log `ssrf_host(url)`, not the URL); (4) **client-supplied values go through `bote_log_str`**, which logfmt-escapes them — bote's JSON parser accepts a raw LF inside a string, so an unescaped method name forges a log line; (5) levels: ERROR = an operator must act, WARN = a refusal or anomaly survived, INFO = lifecycle, DEBUG = one line per request, TRACE = under a request; (6) a new refusal path in an HTTP-family transport uses `bote_http_refuse(cfd, level, transport, status, text, reason)`, which logs and sends in one call; (7) a failure that returns a status code says why on the way out — a silent `return 1` is the defect 3.4.0 removed. Build lines on the stack (`var ln[BOTE_LOG_LINE_BYTES];`): a filtered level costs one compare and allocates nothing.
- **What a request allocates goes on the request arena** (3.4.15). `alloc` never frees, so a byte left on the heap by a request is kept for the life of the process, and a client that can reach that path can kill the server. `bote_req_begin()` rewinds the arena at the top of every message, in every transport; everything between there and the sent response uses it, through `_a` functions and `bote_req_alloc()`. Only what outlives the message goes on the heap: what a handler or a sink is handed (`args`, claims, event data), sessions, the registry. Copy to the heap at the last moment, past every refusal: `args` used to be copied before validation, so 60 KB to a tool that does not exist still retained 60 KB. `tests/bote.tcyr` asserts 0 heap bytes per message for the paths that must stay bounded, and a new refusal path adds a row there, **with a long value in every field a client controls, headers included**: the 3.4.15 rows used short ones and missed the id, method and names (3.5.0), and the version header (3.5.1). A reply built from a list gets a row with a long list: `resources/list` and `prompts/list` kept up to 128 KB per request past 4 KB of list, and short rows passed (3.5.3). ⚠ An arena builder is not arena-only: `str_builder_new_a` gives it an arena buffer, but a plain `str_builder_add_cstr` or `_json_emit_escaped` past its room grows on the heap, silently. Size it first with `_sb_grow_a`, using `strlen * 6` for anything escaped, or use the `_a` add. Never rewind a global heap pointer: it cannot see lazy stdlib state.
- **One key scanner.** `_jx_scan` is the only code that walks a JSON object's keys, and `_jx_find_value_pos` is its one-key form, so the codec, the dispatcher and the validator cannot read a message differently. A parser differential is a security bug: a value validated one way and read another. Do not add a second walker; `str_to_int` on a client's number was the same bug by another route (3.4.15). `_jx_members` (3.5.2) walks an object whose keys are unknown, at registration only, and decodes them the same way.
- **Never look up a null key in a stdlib map.** A map's hash is seeded per process and `hash_str(0)` is 0, so `map_get(m, 0)` looks in slot 0 and crashes only if a key sits there: about one run in four with a handful of tools. A missing guard therefore passes most test runs, and a mutant of one survives (3.5.2). Guard a client's value before the lookup, and a test of the guard must fill slot 0 first, as `tests/bote_http_era.tcyr`'s `_mk_d` does.
- **No raw `syscall(`.** Every kernel touch goes through a stdlib wrapper — `sys_*` from `lib/syscalls*.cyr`, `random_bytes`, `eprint`, `sock_*`. A raw number is one target's ABI (60 is `exit` on x86_64 only; 8 is `lseek` on Linux and `dup` on agnos), and `SYS_ACCEPT` showed a bare constant can be x86-only even when it resolves everywhere. CI bans the form across `src/`, `tests/` and `fuzz/` (comment-stripped) since 3.3.11; `src/` has been clean since 3.2.1. Literals passed *to* a wrapper — mode bits, a backlog, fds 0/1 — are POSIX, not ABI, and are fine.
- **Feature-shape via `[lib.<profile>]`.** Don't invent feature flags in Cyrius — produce a separate dist bundle if a consumer subset is worth supporting.
- **`tracing` analog via libro / majra.** Audit goes to libro chain; events to majra pubsub. Wire via `dispatcher_set_audit` / `dispatcher_set_events`.

## Testing

| Test file | Assertions | Surface |
|-----------|-----------:|---------|
| `tests/bote.tcyr` | 876 | error / protocol / jsonx / registry / (3.4.11) the required-only fallback, a repeated key included; mutation-proven / (3.4.12) `bote_conn_serve`: step names, what each failure logs, http and bridge on a held port, and a new options struct reading as none; mutation-proven / (3.4.15; 3.5.0) the request arena: 0 heap bytes per message on every refusal, with a long value in every client-controlled field, kept args, claims, prompt args and URIs surviving the next message; Origin; UTF-8; escaped keys and methods; number bounds; sessions; every handler's result on one line / tool annotations + profiles / prompts / resources / completion / listChanged flag / dispatch / codec / schema / stream + notification builders / session (+ outbound slot) / HTTP helpers / discovery / bridge (+ 3.4.5: the codec's parse / id checks and a single, unwrapped `isError` pass-through; a handler returning nothing audited as a failure) / (3.4.6) `bote_json_escape`: every escape form, UTF-8 pass-through, and all 255 non-NUL bytes round-tripping through bayan / (3.4.8) numbers per JSON Schema 2020-12, every registration refusal (incl. t-ron's and phylax's `Str` shapes), `isError` and invalid-JSON results audited as failures / events / audit / audit_libro / events_majra wire-up / (3.4.0) the per-request DEBUG line (absent at INFO; method, id, tool, status; never tool arguments; raw-LF and JSON-escaped newlines in a method name stay one line) / (3.4.1) `resources/templates/list` and per-sink gating of the `tools/call` duration + event (audit-only, events-only, null-callback, no sinks) / (3.6.3) required scopes: registration's refusals, `dispatcher_scopes_missing` (AND, fail closed, only a 1 is held), `-31403` ahead of version and arguments, the audit record, and a token copied once a message for each of the four kinds of handler. Mutation-proven |
| `tests/conformance.tcyr` | 218 | **Protocol conformance by spec clause** (3.4.5), wire bytes in and out through `codec_process_message`: (3.5.0) MCP 2026-07-28's stateless path, M1–M92 (M90–M92, 3.6.3: `-31403` on both eras, and no token in play): per-request `_meta`, `-32022`, `server/discover`, the result shape, both eras in one process, and no heap left by a modern message; mutation-proven;  JSON-RPC 2.0 §4 / §5 / §4.1 / §5.1 / §6 including the spec's own examples; MCP initialize + all four revisions, ping, cancellation, capability gating, pagination cursors, tools/list, the tools/call protocol-vs-execution error split (SEP-1303), prompts, resources (`-32002`), completion, (3.4.6) a tool result holding control bytes and an escaped `serverInfo`, (3.4.8) every handler-result shape coerced to a CallToolResult and typed arguments on a schema compiled at registration; plus a standing check that every response is valid JSON and a §5 Response object. Ports the Rust archive's 44 scenarios (tag `0.92.0`), and the spec wins where they disagree. Failed 29 of 117 on 3.4.4; mutation-proven together with `fuzz/jsonx_valid.fcyr` |
| `tests/bote_auth.tcyr` | 73 | Bearer + allowlist + JWT HS256 + PKCE validators; (3.4.0) a 401 logs WARN with its reason and never the token; (3.6.3) the 403's exact bytes, and a token that stays on the arena through `auth_bearer_check_c`; (3.6.4) the `_hdr` senders, and a 403 that leaves out what is no scope-token |
| `tests/bote_content.tcyr` | 25 | Typed MCP content blocks + annotations + (3.4.5) `content_text_error_response` |
| `tests/bote_fs_tools.tcyr` | 56 | Filesystem tools — path safety (`..` / absolute refusal), JSON unescape, root confinement, (3.4.6) `fs_read` of `tests/fixtures/control-bytes.txt` comes back as valid JSON, (3.4.10) the input schemas (path / content required) and annotations |
| `tests/bote_host.tcyr` | 203 | HostRegistry + IPv4/IPv6 SSRF + JSON config hot-reload + (3.4.4) the resolved-address classifiers (packed IPv4, IPv6 bytes, literal-vs-resolved parity) + (3.4.0) `ssrf_reason` / `ssrf_host` + (3.4.7) a test naming each new range (and the two left open on purpose), IPv6 spellings / zone ids / malformed literals, and `_ssrf_parse_ipv6` checked against cyrius's `_tn_parse_ipv6` on 37 inputs. Mutation-proven |
| `tests/bote_http_era.tcyr` | 1661 | HTTP's side of MCP 2026-07-28 (3.5.1), each clause through the real `_http_handle`, `_strm_handle` and (3.5.2) `_bridge_handle` over a socketpair, asserting the status, the JSON-RPC error and exactly one reply: (3.6.0) Protected Resource Metadata and the 401's challenge on all three, with the spec's valid and invalid URIs; `Mcp-Method` / `Mcp-Name` missing, invalid, disagreeing; (3.5.2) `Mcp-Param-{Name}` by the spec's table, its Base64 examples and the conformance suite's cases, a nested property, a field on two lines, the bridge's CORS on every reply, the server's own Origin; the Base64 sentinel; the version header against `_meta`; one message per POST; `202`; `404` / `400`; `405` with `Allow`; sessions ignored on this path and kept on the legacy one; `require_protocol`; claims reach the handler; 0 heap bytes per refused message with 60 KB in each header; (3.6.3) the 403 on all three transports and both eras, a batch's `-31403`, a notification, repeated and escaped keys, the audit record, no checker, the bridge's CORS, and 0 heap bytes with a 60 KB token, 60 KB of arguments or a tool naming 64 long scopes; (3.6.4) CORS on each of the bridge's refusals. Handler tests are `#ifndef CYRIUS_TARGET_AGNOS`. Mutation-proven |
| `tests/bote_jwt.tcyr` | 376 | JWT HS256 verify — b64u decode, signature, exact `alg` field read (incl. the algorithm-confusion header), `exp` enforcement + malformed-claim rejection. Both gates mutation-proven. (3.4.14) a signature re-spelled in its spare bits or padding is refused on every toolchain. (3.6.1) `aud` as a string, an array and escaped, `iss`, `nbf`, a NUL in a value, the decoder's refusals, and 0 heap bytes for a forged token. (3.6.2) RS256 and ES256 against RFC 7515 A.2 and A.3 and `tests/fixtures/jwt_vectors.cyr`, tokens Node signed: `kid`, `crit`, algorithm confusion, CVE-2022-21449's zero signature, every key a set may not load. (3.6.3) `jwt_scope_check`: exact matching, `scope` and `scp`, every shape a claim can take, rules through a chain, a cycle and a dense set, 0 heap bytes. (3.6.4) the cache against an uncached handle on every side of `exp` and `nbf`, through `_bote_jwt_clock`; every byte of a remembered token changed; settings and resizes forgetting; a stored digest corrupted; 0 heap bytes. Mutation-proven |
| `tests/bote_libro_tools.tcyr` | 73 | libro audit-tool dispatch surface + (3.3.4) the `struct error` decode path on a TAMPERED chain and `libro_proof` on a POPULATED one — both were unreachable from any assertion before, which is why the 2.8.11 layout shift shipped a SIGSEGV. Mutation-proven. (3.4.10) every result a CallToolResult with `structuredContent`, `isError` on each failure, the `libro_retention` value guard (chain proven untouched), RFC 3339 timestamps (they were heap addresses), schemas and annotations |
| `tests/bote_log.tcyr` | 50 | `src/log.cyr` (3.4.0) — level names, `bote_log_enabled`, exact line bytes, logfmt escaping (incl. the forged-line case), value / line truncation, dead lines that emit nothing; captured through sakshi's emit hook. Mutation-proven |
| `tests/bote_pkce.tcyr` | 17 | RFC 7636 PKCE-S256 |
| `tests/bote_sandbox.tcyr` | 13 | kavach-shaped pluggable runner adapter |
| `tests/bote_transport_unix.tcyr` | 76 | Unix transport — (3.4.3) the send and idle deadlines over a `socketpair`, and `_unix_handle_client` dropping a peer that floods and never reads (it used to block for good; mutation-proven — a regression hangs until the runner's timeout kills it) — `_unix_sockaddr` (incl. the 107-byte truncation clamp), accept-error policy `_unix_accept_action` + capped backoff, `sleep_ms` really blocks, `sys_accept4` arch guard, and (3.3.17) a write to a closed peer returns `-EPIPE` instead of killing the process with SIGPIPE — mutation-proven: on the 6.6.14 toolchain the file dies with 141. Both socket probes are `#ifndef CYRIUS_TARGET_AGNOS` — the target has no `AF_UNIX`, and the loop they exercise is guarded the same way. New at 3.2.1; module had no coverage before |
| `tests/bote_streamable.tcyr` | 53 | Streamable HTTP — EventIdGenerator / StreamEvent / ResumptionBuffer / SessionOutbound (per-session buffer + id gen) / GET drain selection / client-notification sink (tools + prompts list_changed) / POST-piggyback SSE / StreamableConfig |
| `tests/bote_web_tools.tcyr` | 66 | Web tools — (3.4.10) the input schemas and open-world annotations, (3.4.4) the resolved-address guard through a fake embedder resolver (rebind to 127.0.0.1 / metadata / ULA refused, hook restored, no self-chaining; mutation-proven), (3.4.0) the SSRF guard on `web_fetch` (loopback v4/v6, metadata, private; refused before any socket; userinfo never logged), scheme guard, HTML→text stripper (incl. control-byte/NUL drop), url-encode, entity decode |
| `tests/bote_ws.tcyr` | 50 | WebSocket — WsConfig + handler wire-up + (3.4.3) the real `_bote_ws_handler` dropping a peer that floods frames and never reads (mutation-proven) + (3.5.1) the upgrade's version check, which leaves no heap + (3.6.3) `-31403` in a frame, and the upgrade's claims holding across three frames + (3.6.4) 0 heap bytes for a refused upgrade and none of bote's for a connection, each long handshake header refused, a handler's kept claims surviving the next connection. Mutation-proven |
| `tests/bote_core_only_smoke.tcyr` | drift guard | Includes only `dist/bote-core.cyr` — catches core/transport entanglement |
| **Total** | **3886** | + 1 drift smoke, + CI's "Logging smoke" step (the built binary at nine `BOTE_LOG` values: stdout must stay one JSON-RPC line). Green on **x86_64** and, since 3.3.11, **all of it under `qemu-aarch64` 11.1.3** (`cyrius test --aarch64 <file>`; the cross-build is gated in CI, the emulated sweep is local). Through 3.3.10 that sweep was PARTIAL — qemu 11.1.0 did not pass `getrandom` through, so `bote.tcyr`, `bote_pkce.tcyr` and `bote_streamable.tcyr` exited 90 on the fail-closed session-ID contract (373 of 883 executing). If it regresses to that shape, suspect the emulator or the toolchain's aarch64 `ESYSXLAT` rows before bote |

Criterion benchmarks: **30** in `tests/bote.bcyr` (`jwt_cached_hs256`, `jwt_cached_es256` and `jwt_forged_hs256_cache_on` / `_off`, 3.6.4, beside the verify rows, which run with the cache off; `scope_gate_open`, `scope_check_jwt` and `dispatch_tools_call_scoped`, 3.6.3; `jwt_validate_audience`, 3.6.1; `jwt_rs256` and `jwt_es256`, 3.6.2; `resource_template_match`, 3.5.3; `http_era` × 3, legacy, modern and (3.5.2) with `Mcp-Param` headers on a dispatcher of its own, so that no row's input moves; dispatch × 4 — incl. `dispatch_tools_call_result`, a handler returning a CallToolResult (3.4.8) beside `dispatch_tools_call`'s bare one — jsonx × 3 — incl. `jsonx_valid_request`, the 3.4.5 syntax pass — codec × 3, schema × 4, auth_bearer × 2). Fuzz harnesses: **10** in `fuzz/` (CI step "Fuzz").

**Third-party conformance** (3.5.3): `./scripts/conformance.sh` runs the official `@modelcontextprotocol/conformance` and `@modelcontextprotocol/client`, at versions pinned in the script, against `tests/fixtures/conformance_server.cyr`, which adds the names the suite calls and nothing else. Its numbers are 25 of 37 required scenarios with no failed check on 2026-07-28 (2 of the 25 carry a warning) and 21 of 30 on 2025-11-25, and the client passes 410 checks, the whole OAuth flow included, for HS256, RS256 and ES256: a 401, the metadata, a mock authorization server, a real JWT bound to the resource, the retry, ten tokens that must fail, (3.6.3) eight scope cases, and a step-up the client makes on its own from a 403. It fails on a failure not listed in `tests/fixtures/conformance-expected-<revision>.yaml` and on a listed one that now passes, so a new feature must remove its entries. Not a CI step, because it installs from npm; CI only builds the fixture. Run it by hand at a release that touches the protocol. A name in the fixture must have a real feature behind it: registering `test://template/123/data` as a plain resource would pass that scenario and prove nothing.

```bash
cyrius deps                            # Resolve [deps.*] → lib/ (gitignored)
./scripts/build-all.sh                 # Build build/bote (+ bote-streamable / bote-ws alias links)
./scripts/transport-smoke.sh           # Every mode of the built binary answers (needs curl, python3)
./scripts/conformance.sh               # The official suite and client, pinned (needs node, npm; not in CI)
node tests/fixtures/jwt_vectors.mjs > tests/fixtures/jwt_vectors.cyr  # Regenerate the signed test tokens (public keys and tokens only)
cyrius test tests/bote.tcyr            # Run a single test file
for f in tests/*.tcyr; do cyrius test "$f"; done  # All tests
cyrius bench tests/bote.bcyr           # Run benchmarks
./scripts/bench-log.sh                 # Benchmarks + append to benches/history.log
./scripts/bundle-min-cyrius.sh         # Which installed toolchains build this checkout (sets "Minimum cyrius")
cyrius distlib                         # Regenerate dist/bote.cyr
cyrius distlib core                    # Regenerate dist/bote-core.cyr
```

## Documentation Structure

```
Root files (required):
  README.md, CHANGELOG.md, CLAUDE.md, CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, LICENSE, DEPS-PATTERN.md (distribution contract)

docs/ (required):
  architecture/overview.md — module map, data flow, consumers
  development/roadmap.md — forward-facing only: next patch, upcoming minors, open decisions, 4.0 criteria (what shipped is the CHANGELOG)

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
- **Do not issue a raw `syscall(`** in `src/`, `tests/` or `fuzz/` — call the stdlib wrapper (`sys_exit`, `sys_open`, `random_bytes`, …). CI fails on the form itself, not on a constant list
- Do not auto-inject heavyweight stdlib (sandhi / tls / sigil / ws_server) when only one module needs it — manual `include "lib/..."` in that one src file (see `transport_ws.cyr` for the pattern)
- **Do not write history into `cyrius.cyml` — it is a manifest, not a ledger.** Present-tense rules only: why an order matters, what must not be added, what a contract is. Incidents and version-by-version narrative go in CHANGELOG; standing rules go in this file; distribution reasoning goes in DEPS-PATTERN.md. ⚠ **The failure mode is adding one line of context while making a real change** — a version number, a "joined at X", the measurement behind a decision. If an edit to this manifest mentions a release, delete the sentence and put it where it belongs. (It has accreted twice: rewritten 300 → 174 lines at 3.3.10, trimmed again after 3.3.13 added a profile-decision narrative to it.)
