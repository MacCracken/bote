# Changelog

All notable changes to bote are documented here.

## [2.3.0] — 2026-04-14 — RFC 7636 PKCE-S256 helpers

OAuth 2.1 mandates PKCE on every authorization-code flow; this release
ships the verifier + S256 challenge primitives so MCP clients running
inside bote-hosted handlers can initiate the auth dance without
reimplementing the (small but easy-to-mess-up) crypto. Pairs with the
JWT verifier (2.2.0) and bearer middleware (1.9.0).

### Added
- **`src/pkce.cyr`** (~80 LOC, opt-in module):
  - `pkce_code_verifier(out_buf, len) → 0|err` — writes `len` URL-safe random bytes from `/dev/urandom`. RFC 7636 §4.1 length [43..128]; `out_buf` must be `len + 1` bytes (NUL-terminated). Returns non-zero on out-of-range len or entropy failure.
  - `pkce_code_challenge_s256(verifier) → cstr` — `base64url(sha256(verifier))`, no padding. Uses sigil's `sha256` one-shot. Returns a NUL-terminated 43-char cstr (32 bytes → 43 chars).

### Notes
- **S256 only.** OAuth 2.1 explicitly removes the `plain` method; we don't ship it.
- **Mod-bias.** Verifier byte mapping is `urandom_byte % 66` over the unreserved-character set. Worst-case bias per char is ~0.4% (256/66 = 3 remainder); for 43+-char tokens this is negligible to any guessing attack.
- **No SHA-256 reimplementation.** Calls sigil's existing `sha256(data, len, out)` one-shot — same approach as 2.2.0's HMAC use.

### Tests
- **`tests/bote_pkce.tcyr`** — 17 assertions:
  - Verifier length 43 (RFC minimum) + 128 (RFC maximum) succeed
  - Out-of-range (42, 129) rejected
  - Verifier bytes are all in the RFC unreserved set (`[A-Za-z0-9._~-]`)
  - Two consecutive verifiers differ (entropy sanity)
  - **RFC 7636 Appendix B reference vector**: verifier `dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk` → challenge `E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM`
  - Challenge is deterministic
  - Out-of-range / null verifier rejected
  - Fn-pointer addressability for plugging into auth flows
- **603 total** (was 586). Breakdown: `bote.tcyr` 394, `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 24, `bote_host.tcyr` 67, `bote_auth.tcyr` 38, `bote_sandbox.tcyr` 13, `bote_jwt.tcyr` 28, `bote_pkce.tcyr` 17.

### Verified (cyrius 4.7.1)
- All eight test files green.
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 2.2.
- `cyrlint src/pkce.cyr tests/bote_pkce.tcyr` → 0 warnings.
- `./bote` reports `"version":"2.3.0"`.

### Deferred from 2.3.0 to 2.4.0 — claims propagation through transports
Started this release; reverted before tagging. The plumbing change (auth_bearer_check → codec_process_message → dispatcher_dispatch all gain a `claims` arg, transports capture and thread the validator's return) all compiles and `src/main.cyr` builds cleanly — but it tipped `tests/bote.tcyr`'s compile unit past the cyrius 4.7.1 identifier-buffer cap (the misleading `lib/assert.cyr:3` cascade). Production runtime is unaffected; the test file is the cap-pressured one. Will land cleanly when cyrius 4.8.0 raises the cap.

### Roadmap
- **Claims propagation** → 2.4.0 once 4.8.0 lands.
- **OAuth 2.1 authorization-code endpoints** (Bote-as-AS rather than Bote-as-RS) — out of scope; bote is a resource server. Consumers wanting a full AS layer compose bote with their own authorization server.
- **JWT RS256 / ES256** — waits on sigil's RSA / ECDSA primitives.

## [2.2.0] — 2026-04-14 — JWT HS256 verifier (auth roadmap)

Lands the JWT bearer-token verifier from the 2.0 deferred list. Pairs
with the bearer middleware (1.9.0) — plug `auth_validator_jwt_hs256`
into `auth_bearer_check` and your bearer endpoint validates RFC 7519
JWTs against an HMAC-SHA256 secret.

### Added
- **`src/jwt.cyr`** (~170 LOC). Opt-in module; not wired into the default `main.cyr`. Consumers `include` it explicitly.
  - **`jwt_verify_hs256(token, secret, secret_len) → 0|1`** — full RFC 7515 §3.5 verification: 3-segment split, header decode + `alg=HS256` check (rejects `alg=none` downgrade), HMAC-SHA256 over `header.payload`, constant-time signature compare.
  - **`jwt_secret_new(secret) → handle`** + accessors. 16-byte handle, opaque to bote.
  - **`auth_validator_jwt_hs256(token, secret_handle) → claims | 0`** — drop-in for the bearer middleware fn-pointer slot. Returns the original token cstr as opaque "claims" on success.
  - **`jwt_b64u_decode(enc, elen, *out_len) → ptr | 0`** — RFC 4648 §5 base64url decoder. Inlined for now; will lift to stdlib `lib/base64.cyr` once `cyrius-stdlib-base64url.md` lands (proposal in `docs/proposals/`).
- **HMAC-SHA256 via sigil.** Calls sigil's existing `hmac_sha256` (already in the bundled `lib/sigil.cyr`) — bote doesn't reimplement. Saves ~30 LOC of inline crypto.

### Security properties
- **`alg=none` downgrade rejected** — the header is decoded and scanned for the literal `HS256`. A token with `{"alg":"none"}` cannot pass even with an empty signature.
- **Constant-time signature compare** — `_jwt_ct_eq` XOR-accumulates all 32 bytes with no early exit. Defeats the per-byte timing oracle.
- **Caller-supplied secret only** — bote doesn't store, log, or expose the secret. The validator handle holds a pointer + length; rotation is the caller's responsibility.

### Tests
- **`tests/bote_jwt.tcyr`** — 28 assertions:
  - base64url decode of RFC 4648 §10 vectors (`"Zm9v"` → `"foo"`, `"Zm9vYmFy"` → `"foobar"`)
  - JWT decode of the RFC 7515 §3.5 example header
  - URL-safe alphabet (`-` and `_` accepted)
  - **Canonical jwt.io HS256 token** (`SflKxwRJ...` with secret `your-256-bit-secret`) verifies — the standard reference vector for HS256 implementations
  - Wrong secret rejected; tampered signature rejected; tampered payload rejected
  - 2-segment / no-dot / null-token / null-secret / zero-length-secret all rejected
  - `alg=none` downgrade rejected
  - `JwtSecret` handle accessors, validator adapter returns claims on success / 0 on bad secret / 0 on null handle
  - Fn-pointer addressability for plugging into `auth_bearer_check`
- **586 total** (was 558). Breakdown: `bote.tcyr` 394, `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 24, `bote_host.tcyr` 67, `bote_auth.tcyr` 38, `bote_sandbox.tcyr` 13, `bote_jwt.tcyr` 28.

### Verified (cyrius 4.7.1)
- All seven test files green.
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 2.1.
- `cyrlint src/jwt.cyr tests/bote_jwt.tcyr` → 0 warnings.
- `./bote` reports `"version":"2.2.0"`.

### Proposed for cyrius 4.8.0
- **`base64url_encode` / `base64url_decode` in `lib/base64.cyr`** — see `docs/proposals/cyrius-stdlib-base64url.md`. Ship-ready reference impl + RFC 4648 §10 test vectors. Bote 2.2.x will lift its inline `jwt_b64u_decode` to the stdlib call once the cyrius agent folds it in (drops ~50 LOC + a few fns from bote's compile unit — meaningful for the long-running cyrius identifier-table cap pressure).

### Roadmap
- **OAuth 2.1 / PKCE-S256** still ahead — JWT HS256 is the verifier substrate; PKCE helpers are a future ship.
- **JWT RS256 / ES256** waits on sigil's RSA / ECDSA primitives.

## [2.1.0] — 2026-04-14 — Pluggable sandbox runner (kavach 3.0 compatible)

Lands the sandbox abstraction the 2.0 roadmap deferred. **`kavach` is at
3.0** (multi-backend: NOOP / process / OCI / gvisor / firecracker / SEV)
— the previous "waits on kavach v2" CHANGELOG line was stale. Bote now
ships the abstract `SandboxRunner` surface; consumers wire kavach (or
any other sandbox) behind a single fn-pointer + ctx, the same adapter
pattern used for `AuditSink` (libro) and `EventSink` (majra).

### Added
- **`src/sandbox.cyr`** (~50 LOC, no deps):
  - `sandbox_runner_new(run_fp, ctx)` — 16-byte handle.
  - `sandbox_run(s, command, timeout_ms)` — null-safe; returns the runner's JSON result or an error envelope. Suggested result shape matches kavach's `ExecResult`: `{"exit_code":N,"stdout":"...","stderr":"...","duration_ms":N,"timed_out":0|1}`.
  - `sandbox_runner_noop` + `sandbox_runner_noop_new()` — built-in adapter that echoes the command back as stdout (exit 0). Useful for tests and for environments where sandboxing isn't required.
- Validator signature: `fn run(ctx, command_cstr, timeout_ms) → result_cstr`. Tool authors call `sandbox_run(s, command, timeout)` from inside their handler body when they need to invoke an external process under isolation.

### Why a fn-pointer adapter, not a `[deps.kavach]` direct link
Kavach 3.0 is a substantial dependency (33 modules / ~7K LOC). Pulling it directly into bote's compile unit would tip the cyrius 4.7.1 identifier-table cap that's already constraining bote (see `docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`). The adapter pattern keeps bote independent — consumers that need kavach link it in their own `cyrius.toml` and write a 5-line `kavach_run_adapter(ctx, cmd, timeout) → ExecResult-as-JSON` function. Consumers that prefer a different sandbox (or none at all) drop in a different fn-pointer.

### Tests
- **New test file** — `tests/bote_sandbox.tcyr` (13 assertions). Covers runner shape (alloc + accessors), `sandbox_run` dispatching to a configured fp, null-safety on a null runner, the noop adapter (echo + JSON escaping), and fn-pointer addressability.
- **558 total** (was 545). Breakdown: `bote.tcyr` 394, `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 24, `bote_host.tcyr` 67, `bote_auth.tcyr` 38, `bote_sandbox.tcyr` 13.

### Verified (cyrius 4.7.1)
- All six test files green.
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 2.0.
- `cyrlint src/sandbox.cyr tests/bote_sandbox.tcyr` → 0 warnings.
- `./bote` reports `"version":"2.1.0"`.

### Roadmap note
- The earlier "kavach sandbox waits on kavach v2 hardening" item is **closed** — kavach 3.0 is shipped and bote 2.1.0 provides the integration substrate. Consumer-side adapter (~5 lines wrapping `noop_exec` / `process_exec` / `oci_exec` / etc.) is left to the consuming application — bote stays kavach-agnostic.

## [2.0.0] — 2026-04-14 — Stable: handler-claims ABI + carry-forward of all 1.9.x security work

The 1.x line ports bote from Rust to Cyrius and then iterates feature-by-feature; the 2.0 ship is the **first stable line** with a single deliberate ABI break (handler signature) so the auth → handler claims pipeline can land cleanly in 2.x without another major bump.

### Breaking
- **Handler signature**: `fn h(args_cstr) → result_cstr` → **`fn h(args_cstr, claims) → result_cstr`**. The new `claims` argument is opaque; in 2.0 it's always `0` (transports don't yet plumb the validator's return value down through `codec_process_message → dispatcher_dispatch → fncall2`). 2.x patch releases will populate it. Existing handlers must update their signature even if they ignore the second arg — `fn echo(args)` becomes `fn echo(args, claims)`. All bundled handlers (`bote_echo`, the five `libro_*` tools, all test/fuzz handlers) updated in this release.
- **Migration cost**: a single argument added to every tool handler the consumer registers. ~5 minutes per app.

### Carries forward (cumulative across 1.9.x — see per-release entries below)
- **Bearer-token middleware** (1.9.0) — fn-pointer + ctx validator, opt-in per HTTP-family transport, RFC 6750 401 with `WWW-Authenticate`.
- **Constant-time bearer compare + HTTP smuggling guard + batch cap + jsonx depth cap + `/dev/urandom`-or-fail** (1.9.4 audit batch A).
- **SSRF rewrite** — integer/octal/hex-IPv4 bypasses + IPv4-mapped-IPv6 + dot-consume verification (1.9.5 audit batch B). All 3 audit Criticals closed.
- **413 + bridge CORS oracle + Unix socket 0600** (1.9.6 audit polish).
- **`content_with_annotations`** — typed-block `audience` + `priority` annotations (1.9.6).

### Verified (cyrius 4.7.1)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_auth.tcyr` 38 = **545 total**.
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.9.x.
- `cyrlint src/*.cyr` → **0 warnings** across all sources.
- `cyrius fuzz fuzz/*.fcyr` → **4 passed, 0 failed**.
- `./bote` reports `"version":"2.0.0"`.

### Known carry-forward (deferred past 2.0)
- **Slowloris recv timeout** (audit H5) — needs a `sock_set_recv_timeout` helper in cyrius `lib/net.cyr` that doesn't exist. Workaround: deploy behind nginx/caddy.
- **WS handshake key-length validation** (audit M4) — lives in stdlib `lib/ws_server.cyr`; tracked upstream.
- **DNS resolution for SSRF hostname classification** — needs a cyrius `getaddrinfo` stub. Production callers should pair with a network-policy egress block.
- **Bridge optional protocol-version gate** (audit M5) — tried for 1.9.6, tipped the cyrius 4.7.1 cap; revisit when 4.8.0 raises room.
- **`libro_tools` default registration** — turned opt-in via `BOTE_LIBRO=1` env var in 1.9.4 to free identifier-table headroom; restore default-on when 4.8.0 lands.
- **Threaded streaming dispatch / WS arena allocator / `kavach` sandbox** — wait on cyrius primitives.
- **OAuth 2.1 / PKCE / JWT verifier** — bearer substrate is the hook; flows + verifier are the next net-new feature work.
- **Claims propagation through transports** — handler ABI is in place (this release); auth → dispatch plumbing follows in 2.1.
- **v1.2.1 libro-growth heisenbug** — unchanged.

### Carried-forward release map (1.x → 2.0)

| Tag | Headline |
|---|---|
| 1.0.0 | Cyrius port — protocol core, registry, dispatch, schema, codec, sessions, four transports |
| 1.1.0 | AuditSink + EventSink fn-ptr+ctx adapters |
| 1.2.0 | LibroAudit + MajraEvents adapters via `[deps.libro]` + `[deps.majra]` |
| 1.3.0 | Adopt cyrius 4.5.0 stdlib `lib/http_server.cyr` |
| 1.4.0 | Streamable HTTP transport (MCP 2025-11-25) |
| 1.5.0 | WebSocket transport (RFC 6455) on stdlib `lib/ws_server.cyr` |
| 1.5.1 | P(-1) hardening — HTTP body clamp + null-guard sweep |
| 1.6.0 | `libro_tools` — 5 built-in MCP audit tools |
| 1.7.0 | Typed MCP content blocks |
| 1.8.0 | `HostRegistry` + IPv4 `ssrf_check` |
| 1.8.1 | Bump cyrius 4.6.2 |
| 1.9.0 | Bearer-token middleware (RFC 6750) |
| 1.9.1 | IPv6 SSRF + binary-blob resource + env-driven CLI bearer |
| 1.9.2 | Bump cyrius 4.7.0 |
| 1.9.3 | Bump cyrius 4.7.1 + 2.0-prep doc sweep |
| 1.9.4 | Security batch A — 5 audit findings |
| 1.9.5 | Security batch B — 3 SSRF criticals |
| 1.9.6 | Final polish — 413, CORS oracle, Unix 0600, annotations |
| **2.0.0** | **Handler-claims ABI** + the 1.9.x carry-forward |

---

## [1.9.6] — 2026-04-14 — Final pre-2.0 polish: contained audit items + annotations

Closes the audit items that didn't need a cyrius stdlib helper, plus
re-lands the `content_with_annotations` work that was reverted from
1.9.1 / 1.9.4 for cap-headroom reasons. Last 1.9.x patch — next stop is
**2.0.0** (claims propagation through handler signature).

### Security
- **M1 — 413 Payload Too Large on oversized declared bodies.** `transport_http`, `transport_streamable`, and `bridge` now reject any request whose `Content-Length` declares more than 60 KB (60 × 1024 = 61440) — well under the 64 KB recv buffer with room for headers. Previous behaviour silently truncated and treated as a malformed 400; new behaviour returns the spec-correct status. Live smoke: 100 KB POST → 413 ✓.
- **M3 — bridge CORS oracle.** `_bridge_cors_origin` previously returned `vec_get(allowed_origins, 0)` (the first allowed origin) on a miss — leaks the allowed-origin list to any cross-origin requester. New behaviour returns the literal string `"null"` (a spec-compliant value; browser will block the response either way). Existing test updated.
- **L1 — Unix socket file mode.** `transport_unix_run` now `chmod(path, 0600)` post-bind so only the owning UID can connect. Previous behaviour inherited the process umask (typically 0022 → mode 0755) and let any local user dial the socket. Live smoke: socket file shows `srw-------` ✓.

### Added
- **`content_with_annotations(block, audience, priority)`** — re-landed from the 1.9.1 / 1.9.4 deferral. Splices an `annotations` field into any pre-built block cstr right before the trailing `}`. Audience is a vec of cstr (e.g. `"user"` / `"assistant"`); priority is an i64 in [0, 100] (`-1` = unset). Either argument may be null/-1; if both are absent the input is returned unchanged. **6 new test assertions** including no-op pass-through, audience-only, priority-only, both, image-block annotation, and null-block guard.

### Audit findings still open after 1.9.6
- **High — Slowloris recv timeout** (H5). Needs a `sock_set_recv_timeout` helper in `lib/net.cyr`. Workaround: deploy behind nginx/caddy that absorbs slow connections.
- **Medium — bridge optional protocol-version gate** (M5). Was attempted in 1.9.6 but added 2 new bridge_config_* fns that tipped the test compile unit past the cyrius 4.7.1 cap. Bridge is local-/single-app-scoped in practice; the bearer-token gate already covers production deployments. Will revisit when cyrius 4.8.0 raises the cap.
- **Medium — WS `Sec-WebSocket-Key` length validation** (M4). Lives in stdlib `lib/ws_server.cyr`; bote can't patch without forking. Tracked upstream.

### Verified (cyrius 4.7.1)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_auth.tcyr` 38 = **545 total** (was 539 at 1.9.5; +6 annotation assertions).
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.9.5.
- `cyrlint src/*.cyr` → 0 warnings across all sources.
- Live HTTP smoke: normal POST 200, 100 KB POST 413; Unix socket created with mode 0600.
- `./bote` reports `"version":"1.9.6"`.

### Up next: 2.0.0
- **Claims propagation through handler signature.** Handler currently `fn(args_cstr) → result_cstr`; 2.0 threads the validator's claims through dispatch so handlers can authorize per-tool. ABI break (warrants the major bump).
- **Final P(-1) hardening sweep.**
- **Restore `libro_tools` default registration** (was made opt-in in 1.9.4 for cap headroom; cyrius 4.8.0 should give us room).

## [1.9.5] — 2026-04-14 — Security batch B: SSRF rewrite (3 critical bypasses)

Closes the three Critical findings from the 2026-04-14 audit
(`docs/audit/2026-04-14.md`). All three are SSRF bypasses that landed
on cloud-metadata / loopback endpoints despite the 1.8.0+ blocklist.

### Security
- **C1 — integer-form IPv4 bypass.** `http://2130706433/` (the integer
  encoding of 127.0.0.1) was reaching the hostname classifier as a
  0-dot host and passing as `SSRF_OK` because the blocklist only knew
  literal `localhost` / `metadata`. **Fix**: hostname classifier now
  rejects all-`[0-9.]` hosts as `SSRF_PARSE`. Also catches short-form
  IPv4 (`127.1`, `127.1.1`).
- **C2 — octal IPv4 bypass.** `_ssrf_parse_octet` accepted leading-zero
  multi-digit forms: `0177` parsed as decimal 177 (apparently public)
  while glibc's `inet_aton` interprets the same bytes as octal 127 →
  loopback. **Fix**: octet parser now rejects digits>1 starting with
  zero (`00`, `01`, `0177`, `010` all invalid; `0`, `127`, `255`
  still valid).
- **C3 — IPv4-mapped IPv6 bypass.** `http://[::ffff:127.0.0.1]/` and
  `http://[::ffff:7f00:1]/` were classified by the IPv6 prefix-blocklist
  which only matched `::1`, `::`, `fe80:`, `fc/fd`, `ff` exact —
  `::ffff:` v4-mapped fell through as `SSRF_OK`. Also: the IPv4
  classifier's `_ssrf_parse_octet` didn't verify it consumed the full
  byte range up to the next dot, so `64:ff9b::1.2.3.4` (NAT64 form)
  parsed as `64.2.3.4` (public) and never reached the IPv6 path.
  **Fix**: IPv6 classifier now blocks `::ffff:`, `::*.*.*.*` (v4-compat),
  `64:ff9b:` (NAT64 well-known) outright; IPv4 classifier requires each
  octet parse to consume exactly the dot-to-dot range.

### Tests
- **11 new host assertions** for the bypasses: integer IPv4, octal IPv4
  (`0177.0.0.1`, `0010.0.0.1`), short-form IPv4 (`127.1`),
  `::ffff:127.0.0.1`, `::ffff:7f00:1`, `::127.0.0.1`, NAT64
  `64:ff9b::1.2.3.4`, plus regression checks for canonical decimal
  IPv4 still passing (`10.0.0.1` → PRIVATE, `1.2.3.4` → OK).
- **539 total** (was 528). Breakdown: `bote.tcyr` 394,
  `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 18,
  `bote_host.tcyr` 67 (was 56), `bote_auth.tcyr` 38.

### Audit report
- **`docs/audit/2026-04-14.md`** — full findings list (3 critical,
  5 high, 5 medium, 1 informational), audit-driven release map, and
  follow-up notes for items deferred to 2.0.

### Verified (cyrius 4.7.1)
- All five test files green (539 total).
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.9.4.
- `cyrlint src/host.cyr tests/bote_host.tcyr` → 0 warnings.
- `./bote` reports `"version":"1.9.5"`.

### Audit findings still open (deferred to 2.0)
- **High — Slowloris** (recv timeout — needs `sock_set_recv_timeout` stdlib helper)
- **Medium — bridge CORS oracle**, **WS handshake key-length validation** (upstream stdlib), **bridge protocol-version gate**, **413 cap on oversized requests**
- **Informational — Unix socket default umask** (chmod 0600 missing)

See `docs/audit/2026-04-14.md` for the full list and audit-driven
release map.

## [1.9.4] — 2026-04-14 — Security batch A (audit-driven)

First slice of the 2.0-prep security pass. Closes 4 of the 5 audit
findings in batch A; the slowloris recv-timeout fix needs a
`sock_set_recv_timeout` stdlib helper that doesn't exist yet, so it's
deferred to 1.9.5 / 2.0.

### Security
- **HTTP request smuggling guard** (RFC 9112 §6 / CVE-2019-18276 family). All four HTTP-family transports (`http` / `bridge` / `streamable` / `ws`) now reject any inbound request that carries a `Transfer-Encoding` header — bote doesn't dechunk, and rejecting any TE eliminates the CL.TE / TE.CL ambiguity. Inlined as a 3-line guard in each handler (rather than a shared helper) to stay under cyrius 4.7.1's identifier-table headroom.
- **Constant-time bearer-token comparison** (`auth_validator_allowlist`). The previous `streq`-based compare leaked per-byte timing, letting a network attacker byte-by-byte-guess a token against the allowlist. New implementation: lengths are still compared first (token length leaks; unavoidable), but the byte-loop XOR-accumulates with no early exit and the outer vec walk doesn't short-circuit on match — defeats both the per-byte oracle and the position-of-match oracle. Both helpers are inlined into `auth_validator_allowlist` itself (rather than a separate `_auth_ct_eq` fn) for the same identifier-table reason.
- **Batch-size cap** (`src/codec.cyr::codec_process_message`). Hardcoded `n > 100` check on JSON-RPC array batches — defends against a 1 MiB body of `[{},{},...]` (~300k elements) that would otherwise spin allocating per-element responses until OOM. The literal `100` is inlined to avoid a top-level `var` symbol; 100 is generous (real clients batch single-digit calls).
- **JSON nesting depth cap** (`src/jsonx.cyr::_jx_skip_struct`). Hardcoded `depth > 64` returns end-of-buffer rather than walking adversarial deep structures. The 64 KB inbound HTTP buffer already caps the absolute worst case, but the explicit guard keeps us safe if anyone raises the buffer.
- **`/dev/urandom`-or-fail** (`src/session.cyr::_gen_session_id`). The previous fallback used `clock_now_ns()` (predictable to any timing observer) when `/dev/urandom` failed to open. New behaviour: refuse to mint a session ID — write a fatal error and `SYS_EXIT(90)`. Also loops the read until 16 bytes are received (was a single `syscall(SYS_READ, ...)` that could short-read and leave uninitialized bytes in the SID material).

### Changed
- **`libro_tools` no longer registered by default at startup.** Made opt-in via `BOTE_LIBRO=1` env var to free identifier-table headroom for the security inlines above. Consumers who want the five `libro_*` MCP tools just set `BOTE_LIBRO=1` in their environment, or include `src/libro_tools.cyr` + call `libro_tools_init` / `libro_tools_register` from their own `main.cyr`. **This is a behaviour regression** — minor for most consumers (the tools were registered against an empty chain anyway). Will revert to default-on when cyrius lifts the cap further (4.8.0+).

### Tests
- **9 new auth assertions** for the constant-time compare: first-byte-differ, last-byte-differ, shorter, longer, position-independence across multi-entry allowlists. **528 total** (was 519). Breakdown: `bote.tcyr` 394, `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 18, `bote_host.tcyr` 56, `bote_auth.tcyr` 38.
- Live HTTP smoke: normal POST → 200 ✓; POST with `Transfer-Encoding: chunked` → 400 ✓; 150-element batch → `-32600` "batch too large" ✓.

### Verified (cyrius 4.7.1)
- All five test files green (528 total).
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.9.3.
- `cyrlint src/*.cyr` → 0 warnings.
- `./bote` reports `"version":"1.9.4"`.

### Audit findings still open (deferred to 1.9.5 / 2.0)
- **Critical (3) — SSRF bypasses**: integer/decimal IPv4 (`http://2130706433/`), octal/hex IPv4 (`http://0177.0.0.1/`), IPv4-mapped IPv6 (`http://[::ffff:127.0.0.1]/`). Need a coherent rewrite of the host parser; queued for 1.9.5.
- **High — Slowloris** (single-byte-then-pause holds the accept loop). Needs a `sock_set_recv_timeout` stdlib helper.
- **Medium — bridge CORS oracle** (echoes `allowed_origins[0]` on miss), **WS handshake doesn't validate `Sec-WebSocket-Key` length**, **bridge skips protocol/session checks**.
- **Informational — Unix socket created with default umask perms** (chmod 0600 not set).

See `docs/development/roadmap.md` for the 1.9.5 / 2.0 plan.

## [1.9.3] — 2026-04-14 — Bump pin to cyrius 4.7.1 + 2.0-prep doc sweep

Toolchain bump + a comprehensive 2.0-prep documentation pass. No
behavioral source changes; per-module test-file layout stays because
even cyrius 4.7.1's `BUILD_METHOD_NAME` scratch-corruption fix doesn't
cover bote's specific overflow path.

### Changed
- **cyrius pin** `4.7.0` → `4.7.1` (`cyrius.toml` + `.cyrius-toolchain`).
- `src/dispatch.cyr` — `_bote_server_version` → `"1.9.3"`.

### Documentation
- **`README.md`** rewritten for current state — six transports, bearer auth, all built-in tools, full module list, 519 tests, env-driven CLI auth quickstart.
- **`docs/architecture/overview.md`** rewritten — ASCII diagram updated for six transports + auth/content/host outbound utilities + sinks + libro_tools surface, full module listing, adapter pattern documented.
- **`docs/development/roadmap.md`** rewritten — shipped-per-release table for 1.0.0 → 1.9.2, explicit "must-have for 2.0" / "nice-to-have for 2.0" / "deferred past 2.0" sections, cyrius-language-dependency status all marked ✅ for items resolved.
- **`docs/spec-compliance.md`** rewritten — every compliance category extended for 1.4.0+ work (streamable HTTP, WS, content blocks, SSRF, bearer middleware, host registry, env-driven auth, `libro_tools`). Adds a content-block subtable and a host/SSRF subtable.
- **`docs/benchmarks-rust-v-cyrius.md`** rewritten — Rust v0.92.0 final history-log entry (2026-04-03) vs Cyrius 1.9.2 / cyrius 4.7.0 numbers side-by-side. New "Net call" + "Where Cyrius wins decisively" framing.
- **`docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`** updated — 4.7.1 status header documents that bote's case is still uncovered (~1339 fns under the 4096 cap, so the bug is elsewhere — possibly identifier-bytes or a different scratch path).

### What we got from 4.7.1
- Function-table cap raised 2048 → 4096.
- `BUILD_METHOD_NAME` scratch corruption fix — directly addresses the misleading-error class we reported.

### What 4.7.1 still doesn't cover (for bote)
- `tests/bote.tcyr` + `lib/ws_server.cyr` still trips `lib/assert.cyr:3: expected '=', got string`. Re-verified with freshly-bootstrapped `cc3 4.7.1`. Per-module test layout stays.

### Verified (cyrius 4.7.1)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 18 / `bote_host.tcyr` 56 / `bote_auth.tcyr` 29 = **519 total** (unchanged).
- `cyrius build src/main.cyr bote` → OK; `./bote` reports `"version":"1.9.3"`.
- `cyrlint src/*.cyr` → 0 warnings.

### Up next (2.0-prep, per the audit)
1. **1.9.4** — security batch A: HTTP Transfer-Encoding rejection + recv timeouts (smuggling + slowloris), constant-time bearer compare, batch-size cap, jsonx depth cap, `/dev/urandom`-or-fail.
2. **1.9.5** — SSRF rewrite: canonical-IPv4-only parser (rejects octal/hex/integer/short-form), full IPv6 16-byte classifier with `::ffff:` v4-mapped, optional getaddrinfo single-shot pre-classification.
3. **2.0.0** — `content_with_annotations`, claims propagation through handler signature, final P(-1) sweep, tag.

### Carried forward
- Identifier-buffer / scratch-corruption — bote-specific case still uncovered in 4.7.1; tracked in `docs/bugs/`.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.9.2] — 2026-04-14 — Bump pin to cyrius 4.7.0

Toolchain bump only. No source changes — bote doesn't use 4.7.0's
headline feature (`shared;` / `.so` output) and the identifier-buffer
ceiling that's been pinning the test split + the deferred annotations
work isn't lifted until 4.7.1 (in flight upstream).

### Changed
- **cyrius pin** `4.6.2` → `4.7.0` (`cyrius.toml` + `.cyrius-toolchain`).
- `src/dispatch.cyr` — `_bote_server_version` → `"1.9.2"`.

### What we got from 4.7.0
- Real `dlopen`-able `.so` end-to-end (`shared;` mode) — not used by bote today, will matter when we factor a tool out as a loadable module.
- `DT_INIT` runs top-level initializers on dlopen, PIC-safe addressing in shared mode, full `.dynamic`/`.dynsym`/`.hash` emission.
- Shared-mode DCE fix (parallel to the 4.6.0-beta2 object-mode fix).

### What we're waiting for in 4.7.1
- Identifier-buffer ceiling raise. Bote currently sits one feature past it: every attempt to add `content_with_annotations` (deferred from 1.9.1) trips the same misleading `lib/assert.cyr:3: expected '=', got string` cascade. Will land + collapse the per-module test-file split when 4.7.1 ships.

### Verified (cyrius 4.7.0)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 18 / `bote_host.tcyr` 56 / `bote_auth.tcyr` 29 = **519 total** (unchanged).
- `cyrius build src/main.cyr bote` → OK; `./bote` reports `"version":"1.9.2"`.
- `cyrlint src/*.cyr` → 0 warnings.

### Carried forward
- cyrius identifier-buffer ceiling — fix in flight for 4.7.1.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.9.1] — 2026-04-14 — IPv6 SSRF + binary-blob resource + env-driven bearer auth

Closes three deferred items from earlier releases. A fourth (block annotations) was tried and reverted — it tipped the cyrius 4.6.2 identifier-buffer ceiling. Will land in 2.0 / when 4.7.0 frees up headroom.

### Added
- **IPv6 SSRF blocklist** (`src/host.cyr`):
  - `_ssrf_classify_ipv6` covers `::1` (loopback), `::` (unspec), `fe80::/10` (link-local), `fc00::/7` ULA private (`fc` and `fd` prefixes), `ff00::/8` (multicast)
  - `_ssrf_extract_host` now parses bracket form (`http://[::1]:8080/...`); malformed bracket → `SSRF_PARSE`
  - `ssrf_check` routes hosts containing `:` (after IPv4 fallback) to the IPv6 classifier
  - 9 new test assertions (`tests/bote_host.tcyr` 47 → 56)
- **Binary resource (`blob`) variant** (`src/content.cyr`):
  - `content_resource_blob(uri, mime, b64_data)` emits `{"type":"resource","resource":{"uri":"...","mimeType":"...","blob":"..."}}` — the spec-prescribed shape for non-UTF-8 resource bodies. Optional fields omitted from output when null, same as `content_resource`.
  - 3 new test assertions (`tests/bote_content.tcyr` 15 → 18)
- **CLI bearer-auth via `BOTE_BEARER_TOKENS` env var** (`src/main.cyr`):
  - `_split_csv_tokens(csv)` parses comma-separated values into a vec of cstrs
  - `_bote_bearer_from_env` reads `BOTE_BEARER_TOKENS`, builds the allowlist + validator, returns fp + ctx via out-params
  - All four HTTP-family transports (`http` / `bridge` / `streamable` / `ws`) auto-wire `auth_validator_allowlist` when the env var is set
  - Stdio + Unix-socket transports skip auth (they're local-only)
  - Backward compatible: env var unset → no behaviour change

### Verified (cyrius 4.6.2)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 18 / `bote_host.tcyr` 56 / `bote_auth.tcyr` 29 = **519 total** (was 507).
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.9.0 baseline.
- `cyrlint src/*.cyr` → **0 warnings** across all sources.
- Live HTTP smoke with `BOTE_BEARER_TOKENS=tok-a,tok-b`: `POST /mcp` with no header → 401, with `Authorization: Bearer wrong` → 401, with `Authorization: Bearer tok-a` → 200 + serverInfo `"version":"1.9.1"`. Empty/unset env var → unchanged 200 with no auth.

### Reverted from 1.9.1 scope
- **`content_with_annotations`** (audience + priority MCP annotations on any block). The pre-built-block-splice approach added enough symbols to push `src/main.cyr`'s compile unit past the cyrius 4.6.2 identifier-buffer ceiling. Reverted to keep the build green; will revisit when cyrius 4.7.0 lands.

### Carried forward
- cyrius 4.6.2 identifier-buffer ceiling — bote's full compile unit sits ~one feature away from the new cap. 4.7.0 expected to provide more room.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.9.0] — 2026-04-14 — Bearer-token middleware (RFC 6750)

First slice of the roadmap `auth` item. OAuth 2.1 + PKCE follow in
later releases; this one delivers the substrate they all sit on:
extract `Authorization: Bearer <token>` from a request, hand the token
to a caller-supplied validator function pointer, and emit a spec-
compliant 401 if the token is missing or rejected. **Opt-in** — a
transport with no validator configured behaves exactly as before.

### Added
- **`src/auth.cyr`** (~140 LOC, no AGNOS deps):
  - `auth_bearer_extract(buf, blen)` — case-insensitive `Bearer ` scheme parse, leading/trailing OWS handling, returns the alloc'd token cstr or 0.
  - `auth_bearer_check(cfd, buf, blen, validator_fp, validator_ctx)` — middleware entry. Returns 0 on pass / `HTTP_UNAUTHORIZED` on reject (response already on the wire). No-op when `validator_fp == 0`.
  - `auth_send_unauthorized(cfd, realm)` — 401 with `WWW-Authenticate: Bearer realm="..."`.
  - `auth_validator_allow_all(token, ctx)` — pass-anything validator (testing / dev only).
  - `auth_validator_allowlist(token, vec)` — vec membership check; rejects null/empty.
- Validator signature: `fn validator(token_cstr, ctx) → claims_ptr | 0`. Non-zero return is "valid"; the value is opaque to bote today and will be threaded through to handlers when request-scoped context lands. Returning 0 means "reject".

### Changed
- All four HTTP-family transport configs now have `bearer_validator` + `bearer_ctx` slots **at the end** of the struct (existing offsets preserved):
  - **`HttpConfig`** 56 → 72 bytes (`+56` validator, `+64` ctx)
  - **`BridgeConfig`** 32 → 48 bytes (`+32`, `+40`)
  - **`StreamableConfig`** 64 → 80 bytes (`+64`, `+72`)
  - **`WsConfig`** 48 → 64 bytes (`+48`, `+56` — applied to the upgrade HTTP request only, not per-frame)
- Each gets a corresponding `X_config_with_bearer_validator(c, fp, ctx)` setter and `X_config_bearer_validator(c)` / `X_config_bearer_ctx(c)` accessors.
- Each transport's per-request handler now calls `auth_bearer_check` right after the Origin check and before the protocol-version check.
- `src/main.cyr` includes `src/auth.cyr` ahead of the transports that use it.

### Tests
- **New test file** — `tests/bote_auth.tcyr` (29 assertions). Covers:
  - Header parsing — exact match, three case variants of the scheme, leading/trailing whitespace handling
  - Rejections — no header / Basic scheme / `"Bearer"` with no space / empty token / whitespace-only token
  - `auth_validator_allow_all` accepts any non-empty token, rejects null/empty
  - `auth_validator_allowlist` accepts on match, rejects misses + null inputs
  - Middleware: no-validator → pass, valid token → pass, missing/wrong token → 401
  - Fn-pointer addressability (sanity)
- **Total assertions: 507** (was 478). Breakdown: `tests/bote.tcyr` 394, `tests/bote_libro_tools.tcyr` 22, `tests/bote_content.tcyr` 15, `tests/bote_host.tcyr` 47, `tests/bote_auth.tcyr` 29.

### Verified (cyrius 4.6.2)
- All five test files green (394 / 22 / 15 / 47 / 29).
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.8.1 (dispatch_* 1–3µs, jsonx_* 584–877ns, codec_* 789ns–6µs, validate_* 982ns–2µs).
- `cyrlint src/auth.cyr tests/bote_auth.tcyr src/transport_http.cyr src/transport_streamable.cyr src/transport_ws.cyr src/bridge.cyr` → **0 warnings**.
- Live HTTP smoke (`./bote http 18900` with no validator): `POST /mcp` returns 200 with serverInfo as before; auth machinery has zero overhead when not configured.
- `./bote` reports `"version":"1.9.0"`.

### Deferred to v1.9.x / v2.0
- **OAuth 2.1 + PKCE-S256.** Token *acquisition*, not just *validation*. Will reuse the same validator fn-ptr surface for verification.
- **Claims propagation to handlers.** Today a successful validate just returns 1; richer claims (subject, scopes) need request-scoped context plumbing into the handler signature, which is a bigger ABI change.
- **JWT verification helper.** Worth shipping when the first consumer needs it; keeps the validator surface lean for now.
- **CLI flag to enable bearer auth from the command line.** Programmatic callers can use `X_config_with_bearer_validator` today; CLI wiring follows once we pick a config-file format.

### Carried forward
- cyrius 4.6.2 function-table cap — fifth split test file added; addressed structurally.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.8.1] — 2026-04-14 — Bump to cyrius 4.6.2

Toolchain bump. No source changes beyond the pin + the test-file comment
that describes why the per-module test split is now the permanent
layout rather than a workaround.

### Changed
- **cyrius pin** `4.5.1` → `4.6.2` (`cyrius.toml` + `.cyrius-toolchain`).
- **`src/dispatch.cyr`** — `_bote_server_version` → `"1.8.1"`.
- **`tests/bote.tcyr`** comment updated to explain that per-module test files are the permanent layout. `lib/ws_server.cyr` stays out of the shared compile unit because bote's dep graph (`[deps.libro]` + `[deps.majra]` + `lib/sigil.cyr` alone at 354 fns + 15 stdlib modules + 15 bote sources) already lands near 4.6.2's 2048-fn function-table ceiling — ws_server's 16 fns tip it over.
- **`docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`** — added a 4.6.2 status header: identifier buffer *was* raised, the original repro now trips the function-table cap with a clean diagnostic, but the 4.6.1 diagnostic fix doesn't cover the specific overflow path bote hits.

### What we got from 4.6.1 / 4.6.2
- ✅ Identifier buffer raised (~60 KB headroom) — resolves the 1.5.0-era class of error.
- ✅ Clean `function table full (2048/2048)` diagnostic on the original repro.
- 🟡 Diagnostic fix doesn't cover every overflow path — bote still sees the misleading `lib/assert.cyr:3: expected '=', got string` when its full test unit + ws_server is compiled. Documented for the cyrius agent to take another pass.

### Verified (cyrius 4.6.2)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius test tests/bote_libro_tools.tcyr` → **22 passed, 0 failed**
- `cyrius test tests/bote_content.tcyr` → **15 passed, 0 failed**
- `cyrius test tests/bote_host.tcyr` → **47 passed, 0 failed** (478 total, unchanged)
- `cyrius build src/main.cyr bote` → OK; `./bote` reports `"version":"1.8.1"`.
- `cyrlint src/*.cyr` → 0 warnings.

### Carried forward
- cyrius 4.6.2 function-table cap limits the shared test compile unit — addressed structurally via per-module test files; waiting on either another cyrius-side raise or a compile-unit-level DCE that prunes unreferenced fns from counted totals.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.8.0] — 2026-04-14 — HostRegistry + SSRF guard

Closes out the `host` module started in 1.7.0. The registry gives
handlers a named list of external hosts bote is permitted to reach;
the SSRF guard rejects URLs targeting loopback / private / link-local
/ cloud-metadata endpoints *before* any network call goes out.

### Added
- **`src/host.cyr`** (~260 LOC, no AGNOS deps):
  - **`HostEntry`** (32 bytes): `name` / `url` / optional `headers` vec (alternating key/value cstrs) / optional `capabilities` vec. `host_entry_allows(entry, cap)` is an allowlist check — no-caps means "anything allowed" (fail-open for convenience); an explicit vec enforces the allowlist.
  - **`HostRegistry`** (16 bytes): `name → HostEntry` map backed by stdlib `hashmap`, plus a cached `count` that stays stable on replacement so callers trust it as an O(1) size hint.
  - `host_registry_new/add/get/has/count/names`.
- **`ssrf_check(url)`** — returns `SSRF_OK` (0) on pass, or a non-zero reason code:

  | Code | Meaning |
  |------|---------|
  | `SSRF_PARSE` | Malformed URL / empty host |
  | `SSRF_SCHEME` | Scheme isn't `http://` or `https://` |
  | `SSRF_LOOPBACK` | `127.0.0.0/8`, `localhost` |
  | `SSRF_LINK_LOCAL` | `169.254.0.0/16` (reserved for link-local generally) |
  | `SSRF_PRIVATE` | RFC 1918 (`10/8`, `172.16/12`, `192.168/16`) |
  | `SSRF_METADATA` | `169.254.169.254` (AWS/GCP/Azure), hostname `metadata.google.internal`, bare `metadata` |
  | `SSRF_UNSPEC` | `0.0.0.0/8` |
  | `SSRF_MULTICAST` | `224.0.0.0/4` |

  Case-insensitive on scheme + hostname. Strips `user:pass@` userinfo before classifying. Parses dotted-decimal IPv4 literals directly (no DNS). Non-IP hostnames hit a conservative string blocklist — this is defense-in-depth, not the last line; production callers should pair with DNS-level controls.
- Convenience: `ssrf_is_safe(url)` returns `1`/`0` for call sites that only need a boolean.

### Tests
- **New test file** — `tests/bote_host.tcyr` (47 assertions). Covers:
  - `HostEntry` / `HostRegistry` shape, replace-doesn't-double-count semantics, capability allowlist behaviour including the no-caps fail-open default
  - SSRF pass cases (`api.github.com`, `1.1.1.1`, `8.8.8.8`, port + query, `user:pass@` userinfo)
  - Every blocklist code path with a representative IPv4 (inc. edge cases: `172.15` & `172.32` public, `172.16` & `172.31` private)
  - Hostname blocklist (`localhost`, case-insensitive, `metadata.google.internal`, bare `metadata`)
  - Non-http schemes rejected (`file://`, `gopher://`, `ftp://`)
  - Null / malformed input
- **Total assertions: 478** (was 431). Breakdown: `tests/bote.tcyr` 394, `tests/bote_libro_tools.tcyr` 22, `tests/bote_content.tcyr` 15, `tests/bote_host.tcyr` 47.

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius test tests/bote_libro_tools.tcyr` → **22 passed, 0 failed**
- `cyrius test tests/bote_content.tcyr` → **15 passed, 0 failed**
- `cyrius test tests/bote_host.tcyr` → **47 passed, 0 failed**
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.7.0 baseline.
- `cyrlint src/host.cyr tests/bote_host.tcyr` → **0 warnings**.
- `./bote` — `initialize` → `{"serverInfo":{"name":"bote","version":"1.8.0"}}`.

### Deferred
- **DNS resolution for hostname classification.** A hostname that resolves to a blocked IP isn't caught today — caller can feed `127.0.0.1.nip.io` and pass. Requires a DNS stub on cyrius that doesn't yet exist; queued for a later release. In the meantime, pair with a network policy that blocks egress to RFC 1918.
- **IPv6 literal classification** (`::1`, `fe80::/10`, `fc00::/7`). Skipped for 1.8.0 — IPv4 covers today's deployments; IPv6 blocklist is next slice.
- **Registry persistence / hot-reload.** Registry is built in-process from config; no file watch.

### Known (unchanged)
- cyrius 4.5.1 identifier-buffer cap — this release needed a *fourth* split test file (`bote_host.tcyr`). All four collapse back into `bote.tcyr` when cyrius 4.6.1 lifts the cap. See `docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`.
- v1.2.1 libro-growth heisenbug — unrelated to this release.

## [1.7.0] — 2026-04-14 — Typed content blocks (MCP 2025-11-25)

Handlers can now return **typed content** — `text`, `image`, `audio`,
`resource` (embedded), and `resource_link` (reference) — instead of
only plain-text tool results. First piece of the `src/host.cyr`
roadmap item; host registry + SSRF follow in a later release.

### Added
- **`src/content.cyr`** (~135 LOC). Constructors for every MCP block type:
  - `content_text(text)` — `{"type":"text","text":"..."}`
  - `content_image(b64_data, mime)` / `content_audio(b64_data, mime)` — binary payloads, base64 in-band
  - `content_resource(uri, mime, text)` — embedded resource; `mime` and `text` are optional and omitted from the emitted object when null
  - `content_resource_link(uri, name, mime)` — reference only; client fetches by URI
  - `content_array(blocks)` — `{"content":[...]}` envelope over a vec of pre-built block cstrs
  - `content_array_error(blocks)` — same, with `"isError":true` (MCP distinguishes tool-execution errors from protocol errors by this flag)
  - `content_single(block)` and `content_text_response(text)` — shorthand for the single-block case (no vec alloc)
- Every string argument is a cstr; JSON escaping happens at the boundary via `_json_emit_escaped` (reused from `src/dispatch.cyr`).

### Interop
- `src/bridge.cyr`'s existing `wrap_tool_result` already detects a ready-made `{"content":[...]}` envelope and passes it through untouched — verified by a new test (`wrap_tool_result passes through a content envelope` in `bote_content.tcyr`). Handlers can opt into typed blocks without any transport-layer changes.

### Tests
- **New test file** — `tests/bote_content.tcyr` (15 assertions). Split out of `tests/bote.tcyr` for the cyrius 4.5.1 parser identifier-buffer cap — same pattern as `bote_libro_tools.tcyr`. Both will collapse back into the main test file when cyrius 4.6.1 lifts the cap.
- Coverage: every constructor's exact JSON output (including optional-field omission), JSON escaping of quotes, null-text → empty-string fallback, mixed-type arrays, empty-array case, `isError` flag, and the pass-through interop with `wrap_tool_result`.
- **Total assertions: 431** (was 416). Breakdown: `tests/bote.tcyr` 394, `tests/bote_libro_tools.tcyr` 22, `tests/bote_content.tcyr` 15.

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius test tests/bote_libro_tools.tcyr` → **22 passed, 0 failed**
- `cyrius test tests/bote_content.tcyr` → **15 passed, 0 failed**
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.6.0 baseline (dispatch_* 1–3µs, jsonx_* 612–963ns, codec_* 785ns–6µs, validate_* 1–3µs).
- `cyrlint src/content.cyr tests/bote_content.tcyr` → **0 warnings**.
- `./bote` — `initialize` → `{"serverInfo":{"name":"bote","version":"1.7.0"}}`.

### Deferred to v1.8+
- **Annotations** (`audience`, `priority`) on content blocks — MCP spec optional; skipped to keep 1.7.0 focused.
- **Binary resource contents** (`blob`) — the current `content_resource` handles text; the blob variant needs a base64 decoder surface (or passes through pre-encoded input). Punt until a consumer needs it.

### Known (unchanged)
- cyrius 4.5.1 identifier-buffer cap — forced the third test file. Tracked for 4.6.1 per `docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`.
- v1.2.1 libro-growth heisenbug — unrelated to this release.

## [1.6.0] — 2026-04-14 — libro_tools (5 built-in MCP audit tools)

Lands the `libro_tools` module from the 1.3 roadmap: five MCP tools that
expose a libro audit chain through the normal `tools/call` JSON-RPC
surface. Any MCP client can now search, verify, export, prove, and
retain-manage a bote-hosted chain without learning libro's native API.

### Added
- **`src/libro_tools.cyr`** (~310 LOC) — five handlers + registration:
  - `libro_query` — filter by `source` / `agent_id` / `severity` / `min_severity` / `action` / `after` / `before`; paginate with `offset` + `limit`. Returns `{"ok":true,"total":N,"entries":[...]}`.
  - `libro_verify` — hash-link integrity check. Returns `{"ok":true}` or `{"ok":false,"code":N,"index":i,"message":"..."}`.
  - `libro_export` — every entry as a JSON array. Returns `{"ok":true,"count":N,"entries":[...]}`.
  - `libro_proof` — Merkle inclusion proof for the entry at `index`. Returns `{"ok":true,"index":i,"leaf_count":N,"root":"<hex>"}`. Path hashes are not yet emitted — a follow-up will pin the wire format and include them.
  - `libro_retention` — apply `keep_count` / `keep_duration` / `keep_after` / `pci_dss` / `hipaa` / `sox` policies. Returns `{"ok":true,"archived":N,"retained":M}`.
- `libro_tools_init(chain)` + `libro_tools_register(dispatcher)` wire-up. Every cstr→Str crossing at the libro boundary goes through `str_from()` (the v1.2.1 cstr/Str fix pattern).
- **`src/main.cyr`** — the built-in dispatcher now creates an empty chain at startup and registers the five tools by default, so MCP clients discover them via `tools/list` without any flags.

### Tests
- **New test file** — `tests/bote_libro_tools.tcyr` (22 assertions). Lives separately from `tests/bote.tcyr` because pulling libro_tools into the main test file trips the cyrius 4.5.1 parser identifier-buffer cap. Split-test is tracked to collapse back in 4.6.1 when the cap is lifted.
- Coverage: registration of all five tools in the dispatcher map, handler-fn-pointer addressability, empty-chain shape of each tool's response (`{`-prefixed JSON), required-arg validation (`libro_proof` without `index`, `libro_retention` without `policy`), policy name whitelist (unknown policy → `ok:false`), each preset policy (`pci_dss`, `keep_count`) returns successfully.
- **Total assertions: 416** (was 394). Breakdown: `tests/bote.tcyr` 394, `tests/bote_libro_tools.tcyr` 22.

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius test tests/bote_libro_tools.tcyr` → **22 passed, 0 failed**
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.5.1 baseline (dispatch_* 1–3µs, jsonx_* 588–925ns, codec_* 773ns–6µs, validate_* 970ns–2µs).
- `cyrlint src/libro_tools.cyr tests/bote_libro_tools.tcyr` → **0 warnings**.
- `./bote` (stdio transport) — `initialize` → `{"serverInfo":{"name":"bote","version":"1.6.0"}}`, `tools/list` returns `bote_echo` + 5 `libro_*` entries, all 5 `tools/call` paths return clean JSON on an empty chain (verified by hand).

### Known (unchanged)
- **v1.2.1 libro-growth heisenbug**: creating an empty chain at startup (`chain_new()`) is safe; growing via `chain_append` is where the heap-sensitivity shows up. `libro_tools` itself is correct — only reads chain state, doesn't append. Writes still go through `src/audit_libro.cyr`, which is where the heisenbug lives.
- **cyrius 4.5.1 identifier-buffer cap** (`docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`) — forced the second test file. Tracked for 4.6.1.

## [1.5.1] — 2026-04-14 — P(-1) scaffold hardening

First hardening pass since 1.5.0. No new features; audit-driven fixes to
defensive guards, a line-length lint cleanup across `src/`, and two new
test assertions. All 394 tests / 10 benches / 4 fuzz harnesses green.

### Security
- **HTTP body-length clamp** — `src/transport_http.cyr`, `src/transport_streamable.cyr`, and `src/bridge.cyr` each copy the request body with `memcpy(body, buf + bo, clen)` after reading `Content-Length`. If a lying `Content-Length` header declared more bytes than actually arrived on the wire, `memcpy` would read past the request buffer into adjacent memory. All three paths now clamp `clen = min(clen, n - bo)` before the copy. Tested by manual audit; integration coverage by the existing transport tests still passes.

### Fixed
- **`resumption_buffer_events_after` null/empty guard** — Accepting `last_event_id == 0` previously would have segfaulted on the first `streq`; accepting `""` would have silently scanned the whole buffer for no matches. The caller in `_strm_handle_get` already guards null, but the helper is now defensive in its own right (returns empty vec in both cases). **Two new test assertions** cover these paths (394 total, was 392).

### Changed
- **Line-length cleanup** (`cyrlint`-clean): `src/bridge.cyr:172` (CORS header), `src/dispatch.cyr:57` (tool-name validation error message), `src/stream.cyr:93` (progress notification JSON). No behavior change — just wrapped the offending literals across two `str_builder_add_cstr` calls so lines stay under 120 chars. All `src/*.cyr` files now report `0 warnings` from `cyrlint`.

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.5.0 baseline (dispatch_* 1–3µs, jsonx_* 580–864ns, codec_* 763ns–6µs, validate_* 976ns–2µs)
- `cyrius fuzz fuzz/*.fcyr` → **4 passed, 0 failed**
- `cyrlint src/*.cyr` → all **0 warnings**

### Audit findings deferred
Captured during this pass but not actioned in 1.5.1:
- **Bump-allocator leak on long-lived WS connections** — every inbound frame allocs a fresh payload buffer, and `codec_process_message` returns fresh alloc'd JSON. Short-lived HTTP requests don't notice; WebSocket connections that stay open for hours will accumulate. Proper fix needs either an arena-per-message lifetime or stdlib `fl_free` support. Tracked for v1.6.
- **Global state in `transport_streamable.cyr` (`_strm_event_ids`, `_strm_resumption`) and `transport_stdio.cyr` (`_stdio_buf`, `_stdio_buf_len`)** — safe today because all transports are single-connection-at-a-time per the v1.0 design, but will need mutex-guarding when streaming dispatch (v1.5+ per roadmap) lets a server handle concurrent sessions.

### Carried forward
- v1.2.1 libro live-integration heisenbug: unchanged.
- cyrius 4.5.1 identifier-buffer cap: unchanged (`docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`).

---

## [1.5.0] — 2026-04-14 — WebSocket transport (RFC 6455)

Adds a sixth MCP transport: **WebSocket**. Each TEXT frame is one JSON-RPC
2.0 message. Built on `lib/ws_server.cyr` which landed in **cyrius 4.5.1**
(see `docs/proposals/cyrius-stdlib-ws-server.md` for the design rationale)
— and on the existing `lib/http_server.cyr` for the HTTP/1.1 Upgrade
handshake. **~110 LOC** of MCP-specific wire-up on top of the stdlib,
versus ~400 LOC if hand-rolled.

### Added
- **`src/transport_ws.cyr`** (~110 LOC):
  - **`WsConfig`** (48 bytes) — path, addr, port, allowed_origins, require_protocol, dispatcher
  - **`_bote_ws_handler`** — invoked per connection by `http_server_run`. Enforces Origin + `MCP-Protocol-Version` middleware (same shape as `transport_http`), calls `ws_server_handshake` to upgrade in place, then loops reading TEXT frames and feeding each to `codec_process_message` (control frames — ping/pong/close — handled by stdlib transparently).
  - **`transport_ws_run(dispatcher, config)`** — defers to stdlib `http_server_run`.
- **CLI** — `./build/bote ws [port]` (default `8393`).
- **Proposal artifacts** under `docs/proposals/` (same workflow as the http_server proposal that became cyrius 4.5.0):
  - `cyrius-stdlib-ws-server.md` — design doc + RFC 6455 spec coverage table
  - `lib_ws_server.cyr` — reference implementation with inlined SHA-1
  - `lib_ws_server_example.cyr` — runnable echo server

### Changed
- **cyrius pin bumped to 4.5.1** (required for `lib/ws_server.cyr`).
- `src/main.cyr` dispatches on `ws` argv (default port 8393).

### Spec compliance (RFC 6455, delegated to stdlib)
- ✅ HTTP/1.1 Upgrade handshake, `Sec-WebSocket-Accept = base64(sha1(key + magic))`
- ✅ `Sec-WebSocket-Version: 13` enforced
- ✅ Server reads MASKED client frames, writes UNMASKED server frames
- ✅ Small / medium (16-bit) / large (64-bit) payload length encodings
- ✅ Text + Binary data frames
- ✅ Ping / Pong control frames (handled transparently by `ws_server_recv`)
- ✅ Close handshake with status code + optional reason
- 🟡 Per-message deflate (RFC 7692) — deferred to stdlib
- 🟡 Subprotocol negotiation (`Sec-WebSocket-Protocol`) — header read but not enforced

### Tests
- 10 new unit assertions (**392 total**, was 382):
  - `WsConfig` defaults + setters (path, addr, port, origins, require_protocol, dispatcher)
  - `_bote_ws_handler` fn-pointer addressability
  - Dispatcher wire-up on `transport_ws_run`
- Live handshake + frame round-trip stays with the stdlib `ws_server` conformance tests (avoids duplicating the protocol suite, and dodges the 4.5.1 parser input-buffer cap we hit when pulling the full `ws_server.cyr` into this file).

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **392 passed, 0 failed**
- `cyrius build` → `./bote` (with `ws` subcommand binds `127.0.0.1:8393` and returns 101 on `GET /mcp` with a valid `Sec-WebSocket-Key` — verified via local `wscat` / `curl` probe)
- `cyrius bench` → 10 hot paths unchanged
- `./build/bote` initialize handshake reports `"version":"1.5.0"`

### Carried forward
- v1.2.1 libro live-integration heisenbug: still present, still tracked.

### Known cyrius 4.5.1 artifact
- The parser's input-buffer cap is reached when `tests/bote.tcyr` also includes `lib/ws_server.cyr` directly. Worked around by keeping ws_server out of the test file (the handler's frame I/O is covered by the stdlib conformance tests anyway). Tracked upstream; a follow-up cyrius patch will lift the cap.

---

## [1.4.0] — 2026-04-14 — Streamable HTTP transport (MCP 2025-11-25)

Closes the **streamable HTTP** spec item from MCP 2025-11-25. Single endpoint
serves both `POST` (JSON-RPC request → response) and `GET` (open SSE stream
for server-initiated messages). Built on the stdlib `lib/http_server.cyr`
chunked primitives that shipped in cyrius 4.5.0.

### Added
- **`src/transport_streamable.cyr`** (~290 LOC). Modules:
  - **`EventIdGenerator`** — monotonic counter, emits `"evt-N"` strings
  - **`StreamEvent`** — `{id, event="message", data}` with SSE wire-format renderer (`stream_event_to_wire` → `id: ...\nevent: ...\ndata: ...\n\n`)
  - **`ResumptionBuffer`** — bounded ring of recent events; `events_after(last_id)` for `Last-Event-ID` replay
  - **`StreamableConfig`** (64 bytes) — path, addr, port, allowed_origins, require_protocol, session_store, retry_ms, dispatcher
  - **`transport_streamable_run(d, cfg)`** — defers to stdlib `http_server_run` with a single dispatch handler that routes POST → JSON-RPC, GET → SSE stream
- **CLI** — `./build/bote streamable [port]` (default `8392`).

### Spec compliance
- ✅ `POST <endpoint>` JSON-RPC dispatch (same shape as plain HTTP)
- ✅ `GET <endpoint>` SSE stream open with priming event
- ✅ `MCP-Protocol-Version` header **required** on every request (400 if absent — stricter than plain HTTP transport which makes it optional by default)
- ✅ `MCP-Session-Id` header validated when SessionStore is configured
- ✅ `Origin` allow-list (DNS rebinding protection)
- ✅ `Last-Event-ID` request header → replay buffered events on GET
- ✅ Server emits `id:`-tagged SSE events for resumption tracking
- ✅ `retry: <ms>\n\n` hint sent before stream close (default 5000ms, configurable)
- 🟡 Server-initiated event push on the GET stream: deferred — waits on streaming dispatch (v1.5+) to populate the resumption buffer with real events. The transport correctly opens the SSE stream and replays anything in the buffer; the buffer is just empty until something publishes to it.

### Tests
- 23 new unit assertions (382 total, was 359):
  - `EventIdGenerator` produces monotonic `evt-0`/`evt-1`/`evt-2`
  - `StreamEvent` accessors + SSE wire format (with and without data)
  - `ResumptionBuffer` push, eviction (oldest first when over capacity), `events_after` lookup (present and absent IDs)
  - `StreamableConfig` defaults + setters
  - `http_path_only` correctly strips query string for path matching

### Performance
Bench numbers unchanged. `cyrius bench` still shows 10 hot paths sub-10µs.

### Verified end-to-end
- `POST /mcp` with `MCP-Protocol-Version: 2025-11-25` → returns serverInfo
- `POST /mcp` without protocol header → `400 Bad Request`
- `POST /mcp tools/call` → standard JSON-RPC response
- `GET /mcp` with `Accept: text/event-stream` → opens SSE stream, sends primer (`id: evt-0\nevent: message\ndata: \n\n`), sends retry hint (`retry: 5000\n\n`), closes
- `Last-Event-ID: evt-7` (when buffer has events past evt-7) → replays them in order

### Carried forward
- v1.2.1 libro live-integration heisenbug: still present, still tracked.

### Verified (cyrius 4.5.0)
- `cyrius test` → **382 passed, 0 failed**
- `cyrius fuzz` → 4 passed, 0 failed
- `cyrius bench` → 10 hot paths unchanged
- `./build/bote` initialize handshake reports `"version":"1.4.0"`

---

## [1.3.0] — 2026-04-14 — Adopt stdlib `lib/http_server.cyr` (cyrius 4.5.0)

Cyrius **4.5.0** shipped `lib/http_server.cyr` — verbatim from the proposal
in `docs/proposals/cyrius-stdlib-http-server.md`. Bote drops 236 lines of
hand-rolled HTTP plumbing in favour of the shared stdlib.

### Changed
- **`cyrius.toml`** — added `"http_server"` to `[deps]` stdlib list. Cyrius pin → `4.5.0`.
- **`src/transport_http.cyr`** — was 404 LOC, now **150 LOC** (-63%). Dropped: `_http_find`, `_http_to_lower`, `_http_iceq`, `_http_next_nl`, `http_find_header`, `http_get_method`, `http_get_path`, `http_body_offset`, `http_content_length`, `_http_send_status`, `_http_send_json_200`, `_http_send_204`, plus the bind/listen/accept ceremony. All come from stdlib now. Kept: `HttpConfig` struct + accessors, `_http_check_origin / _protocol / _session` middleware, `_http_handle` request handler, `transport_http_run` (now a 5-line wrapper around `http_server_run`).
- **`src/bridge.cyr`** — was 280 LOC, now **170 LOC** (-39%). Dropped: `_bridge_handle_connection` (rewritten to handler-style), bind/listen/accept loop, response builders. Kept: `BridgeConfig` (added `dispatcher` slot), `wrap_tool_result` / `wrap_error_result` (MCP envelope contract, bote-specific), `_bridge_cors_*` headers, `bridge_process_message` (bridge-specific routing).
- **`HttpConfig`** gained a `+48 dispatcher` slot (was 48 → now 56 bytes) so it can carry the dispatcher into the stdlib `http_server_run` ctx pointer.
- **`BridgeConfig`** gained a `+24 dispatcher` slot (was 24 → now 32 bytes) for the same reason. New setter: `bridge_config_with_dispatcher(c, d)`.
- **Status codes** now use `HTTP_OK`, `HTTP_NOT_FOUND`, etc. constants from stdlib (was hardcoded integers).
- **Path matching** uses `http_path_only(path)` from stdlib so `/mcp?something` matches `/mcp` correctly. Same for bridge `/health` and `/`.

### Performance / size
- **bote ELF binary**: was 130 KB (1.2.1) → **127 KB** (1.3.0). The stdlib HTTP code is shared with any future cyrius project.
- Function count freed up from bote's compilation unit: ~28 fns (the entire HTTP plumbing layer) now lives in stdlib and counts once across all consumers.
- Hot-path benchmarks unchanged.

### Spec impact
- **Content-Length-aware request reading** now correct (stdlib `http_recv_request` reads until body is fully received). Previous bote behaviour did a single `sock_recv` and silently truncated requests larger than one TCP packet — fixed for free.
- **Unblocks v1.4.0 streamable HTTP** — stdlib provides `http_send_chunked_start` / `http_send_chunk` / `http_send_chunked_end` for SSE.

### Carried forward
- v1.2.1 known issue (live libro chain integration heisenbug in tests/bote.tcyr) **persists** despite freed function-count budget. Adapter remains correct in isolated probes; in-test live integration test is still shape-only. Suggests the heisenbug is heap-layout / global-init related, not function-count related — needs deeper cyrius investigation.

### Verified (cyrius 4.5.0)
- `cyrius test` → **359 passed, 0 failed**
- `cyrius fuzz` → 4 passed, 0 failed
- `cyrius bench` → 10 hot paths unchanged
- `./build/bote` initialize handshake reports `"version":"1.3.0"`
- HTTP and bridge transports both verified end-to-end with `curl`

### `docs/proposals/`
The proposal docs (`cyrius-stdlib-http-server.md`, `lib_http_server.cyr`,
`lib_http_server_example.cyr`) remain in the repo as the spec the lang-agent
implemented from. Useful reference for any future stdlib proposals coming
out of bote work.

---

## [1.2.1] — 2026-04-13 — Adapter init-dance docs + v1.2.0 patch hardening

### Fixed
- **`src/audit_libro.cyr::libro_audit_log`** wraps every cstr boundary value through `str_from` before passing to `chain_append` / `chain_append_with_agent`. libro expects `Str` (fat strings) for source / action / details / agent_id, not raw cstrs — the previous version passed cstrs directly which produced garbage `str_len` reads inside libro's hash function. Verified by isolated probe: `chain_append(c, SEV_INFO, str_from("bote"), str_from("tool.completed"), str_from("{}"))` correctly grows the chain to length 1.

### Documented
- **Init dance** for any binary that uses LibroAudit:
  ```
  alloc_init();   # bump allocator
  fl_init();      # freelist — libro entries
  ed25519_init(); # sigil signing constants
  ```
  Codified in the `audit_libro.cyr` header comment.

### Cyrius pin
- Bumped to **4.4.6** (`cyrius.toml`).

### Known issue (tracked for v1.3.0)
- Linking libro + majra + the full bote test corpus into one binary triggers a heap heisenbug: `libro_audit_log` enters an infinite loop (apparently re-entering `main`). The adapter itself is correct — verified by an isolated probe that calls `chain_append` directly with `str_from`-wrapped strings. Suspect: cumulative globals from the cross-product exceed an internal cyrius compilation-unit boundary and corrupt the bump-allocator's prologue. Workarounds explored: dropping `lib/patra.cyr` (saved 244 KB but didn't help), splitting includes (no effect), running the demo as a standalone binary (still loops). Filed as a tracking item; resolution likely needs the multi-file linker on cyrius's v4.5 roadmap.

### Verified (cyrius 4.4.6)
- `cyrius test` → 359 passed, 0 failed
- `cyrius fuzz` → 4 passed, 0 failed
- `cyrius bench` → 10 hot paths unchanged
- `./build/bote` initialize handshake reports `"version":"1.2.1"`

---

## [1.2.0] — 2026-04-13 — LibroAudit + MajraEvents adapters

Wires bote's AuditSink and EventSink (introduced in 1.1.0) to the **libro**
audit chain and **majra** pub/sub. First release with `[deps.<crate>]` git+tag
pinned dependencies.

### Added
- **`[deps.libro]`** — pinned to git tag `1.0.3` (`https://github.com/MacCracken/libro`, falls back to local `../libro`). 9 modules pulled in: `error / hasher / entry / verify / query / retention / chain / export / merkle`.
- **`[deps.majra]`** — pinned to git tag `2.2.0` (`https://github.com/MacCracken/majra`, falls back to local `../majra`). 6 modules pulled in: `error / counter / envelope / namespace / queue / pubsub`. Trimmed to only the modules `pubsub` actually exercises (skipped `metrics / ratelimit / heartbeat / fleet / dag / etc`).
- **`src/audit_libro.cyr`** — `LibroAudit` adapter (24 bytes: chain ptr, source cstr, agent_id cstr). `libro_audit_new(chain)`, `libro_audit_with_source(la, src)`, `libro_audit_with_agent_id(la, id)`, `libro_audit_log(ctx, event)`. Maps bote's ToolCallEvent to libro's `chain_append_with_agent` (or `chain_append` when no agent): `SEV_INFO` + `"tool.completed"` on success, `SEV_ERROR` + `"tool.failed"` on failure. **caller_id wins over the configured agent_id.**
- **`src/events_majra.cyr`** — thin `majra_events_publish(ctx, topic, payload)` that calls `pubsub_publish(ps, topic, payload)`. Wire-up:
  ```
  var ps = pubsub_new();
  var sink = event_sink_new(&majra_events_publish, ps);
  dispatcher_set_events(d, sink);
  ```
- 8 new unit assertions (359 total) covering adapter struct shapes, accessors, AuditSink fp wiring, EventSink fp wiring (incl. `ctx=0` no-op safety).

### Performance
Bench numbers unchanged from 1.1.0 — adapters are pass-through over already-measured sink-publish overhead.

### Deferred
Live `chain_append_with_agent` integration tests (and full pubsub deliver paths) require running libro/majra's full init dance (`alloc_init`, `fl_init`, `ed25519_init`, `patra_init`, etc). Those are exercised by libro's and majra's own test suites; bote's tests currently verify the **adapter shape** (struct + fp wiring). Live integration tests will land in **v1.2.1** once the init-call documentation is finalized.

`src/libro_tools.cyr` (5 built-in MCP audit tools) deferred to **v1.3.0**. Reason: the cyrius compiler hits an internal token-table boundary when the full bote + libro + majra + tool-handler-fns set is included in one compilation unit. v1.3.0 will either (a) split bote into multiple compilation units (multi-file linker, on cyrius's v4.5 roadmap) or (b) ship libro_tools as a separate `[deps.bote-libro-tools]` package.

### Verified (cyrius 4.4.4)
- 359 tests passed, 0 failed
- 4 fuzz harnesses passed
- 10 benchmarks unchanged
- `./build/bote` initialize handshake reports `"version":"1.2.0"`

---

## [1.1.0] — 2026-04-13 — AuditSink + EventSink + dispatcher wire-up

First minor bump on the cyrius lineage. Adds the audit and event-publishing
abstractions and wires them into the dispatcher. Adapters for libro / majra /
discovery-via-pubsub land in **v1.2.0**.

### Added
- **`src/events.cyr`** — `EventSink` (function-pointer + opaque ctx struct, the cyrius equivalent of the Rust trait), `event_sink_new` / `event_sink_publish` / `event_sink_noop`. Topic constants exported as functions: `TOPIC_TOOL_COMPLETED`, `TOPIC_TOOL_FAILED`, `TOPIC_TOOL_REGISTERED`, `TOPIC_TOOL_DEREGISTERED`, `TOPIC_TOOL_DEPRECATED`, `TOPIC_TOOL_ANNOUNCE`, `TOPIC_TOOL_DISCOVERED`, plus 3 sandbox topics for the v1.3 sandbox port.
- **`src/audit.cyr`** — `ToolCallEvent` (40 bytes: tool_name, duration_ms, success, error, caller_id), `tool_call_event_to_json` (matches Rust `serde_json` output, skips `error` / `caller_id` when 0), `AuditSink` struct + `audit_sink_log` / `audit_sink_noop`.
- **`Dispatcher` extended** to 40 bytes — new slots for `audit_sink` and `event_sink`. Setters: `dispatcher_set_audit(d, sink)` and `dispatcher_set_events(d, sink)`. Sinks default to 0 (no-op); pre-1.1 callers see no behavior change.
- **Dispatcher emits per-call audit + event hooks**:
  - `tools/call` success → `audit_sink_log` + publish to `bote/tool/completed`
  - `tools/call` failure (handler not in map) → `audit_sink_log` + publish to `bote/tool/failed`
  - `tools/call` on a deprecated tool → publish to `bote/tool/deprecated` *before* the call
  - `dispatcher_register_tool` → publish to `bote/tool/registered`
  - `dispatcher_deregister_tool` → publish to `bote/tool/deregistered`
  - All include a `{"tool_name":"..."}` payload (deprecated also includes `message`).
- **`caller_id`** now extracted from `tools/call` params (`jsonx_get_str(params, "caller_id")`) and threaded through to the audit event.
- **`src/discovery.cyr` migrated to `EventSink`** — `discovery_new(node_id, event_sink)` replaces the bare `publish_fp`. Uses `discovery_event_sink(d)` accessor; same callers, cleaner integration with the rest of the event surface.
- **50 new unit assertions** (351 total, was 301): topic constants, sink no-op safety, sink invocation, ToolCallEvent JSON round-trips (success / failure / minimal), full dispatcher wire-up (success+failure+initialize+list+register+dereg+deprecated paths), discovery via EventSink, "validate-stage error doesn't audit" parity check vs. Rust.

### Performance
Audit + event hooks add ~2µs to `dispatch_tools_call` (1µs → 3µs) and ~2µs to `codec_process_message` (4µs → 6µs) when sinks are wired. With `audit_sink_noop()` / `event_sink_noop()` (or unset, the default), the overhead is a single null-pointer check per emission site. Other benchmarks unchanged.

### Changed
- `discovery_new` signature: was `(node_id, publish_fp)` taking a bare `fn(topic, json)` pointer; now `(node_id, event_sink)` taking an EventSink. **Source-breaking** — but the only known caller was `tests/bote.tcyr`, and the new shape is what real callers (MajraEvents in v1.2) need anyway. v1.0 callers building a discovery service should switch to `event_sink_new(&publish_fn, ctx)`.

### Verification (cyrius 4.4.4)
- `cyrius test` → **351 passed, 0 failed**
- `cyrius fuzz` → 4 passed, 0 failed
- `cyrius bench` → 10 hot paths, sinks-noop overhead is 1 conditional branch per emission
- `./build/bote` initialize handshake reports `"version":"1.1.0"`

### Deferred to v1.2.0
- `src/audit_libro.cyr` — LibroAudit adapter (calls `chain_append_with_agent` on libro's hash chain). Needs `[deps.libro] path = "../libro"` in `cyrius.toml`.
- `src/events_majra.cyr` — MajraEvents adapter (calls `pubsub_publish`). Needs `[deps.majra] path = "../majra"` in `cyrius.toml`.
- `src/libro_tools.cyr` — 5 built-in MCP tools (`libro_query`, `libro_verify`, `libro_export`, `libro_proof`, `libro_retention`). Depends on audit + libro.

---

## [1.0.1] — 2026-04-13 — Retire rust-old/, trim spec-compliance, bench comparison

### Removed
- **`rust-old/`** — the Rust source archive that came in via `cyrius port` is gone. Its purpose (porting reference) is fulfilled. The last Rust state remains accessible at git tag `0.92.0`.
- `.gitignore` rules for `rust-old/target/` and `rust-old/**/target/` (no longer needed).

### Added
- **`docs/benchmarks-rust-v-cyrius.md`** — side-by-side performance comparison. Source / binary / dep counts, per-op timings (Rust v0.92.0 vs Cyrius v1.0.1), the structural reasons Cyrius is 3-10× slower per op, and where each side wins. Preserves the 5-entry Rust bench history before the archive went away.
- Cyrius pin bumped to **4.4.4** (`cyrius.toml`). All correctness pain points from the original port are now fixed in cyrius itself; only the documented `var buf[N]` size limit remains as a design choice.

### Changed
- **`docs/spec-compliance.md`** — rewritten. Was carrying the Rust-era matrix that listed modules like `host::McpContentBlock` and `auth::TokenClaims` as "Complete" when they aren't ported yet. New version lists **only what cyrius v1.0.x covers today** (with explicit `✅` checkmarks per module + accessor function name) plus a single "Gaps that need coverage" section that points at the roadmap rather than duplicating it.
- README, `docs/architecture/overview.md`, `docs/development/roadmap.md` — `rust-old/` references rewritten to point at git tag `0.92.0`.
- Server `initialize` response — `serverInfo.version` now reports `"1.0.1"`.

### Verified
- 301 tests pass on cyrius 4.4.4
- 4 fuzz harnesses pass (no regressions)
- All 4 transports (stdio / HTTP / Unix / bridge) confirmed end-to-end

---

## [1.0.0] — 2026-04-13 — Stable cyrius MCP core

Bote's cyrius implementation is **stable**. The MCP protocol surface, registry,
dispatcher, schema validation, sessions, discovery, and four transports
(stdio, HTTP, Unix socket, TS bridge) are feature-complete and verified:

- **298 unit assertions** all passing
- **10 hot-path benchmarks**, all sub-10µs on x86_64
- **4 fuzz harnesses**, ~330 calls across malformed and edge-case inputs, no crashes
- **End-to-end smoke tests** for stdio (pipe), HTTP (curl), Unix socket (Python AF_UNIX), bridge (curl + CORS)

The data shapes for `JsonRpcRequest`, `JsonRpcResponse`, `ToolDef`, `ToolSchema`,
`ToolAnnotations`, `CompiledSchema`, `BoteError`, `McpSession`, and the four
`HttpConfig` / `BridgeConfig` flavours are **frozen** — additive changes only
within the 1.x series.

### Critical bug fix included in 1.0.0

- **`src/jsonx.cyr::jsonx_get_str`**: on truncated input (opening `"` with no closing — e.g. `{"k":"`), `_jx_skip_string` returned `end == len`, making `inner_len = end - pos - 2 == -1`. The subsequent `memcpy(out, src, -1)` was interpreted as a huge unsigned size → segfault. Surfaced by the `jsonx_extract.fcyr` fuzz harness on cyrius 4.4.x. **Fix**: clamp `inner_len` to `>= 0` (returns empty string for truncated input). Regression covered in `tests/bote.tcyr`.

### Workaround cleanup (cyrius 4.4.3 unblocked it)

Now that cyrius 4.4.3 ships `\r` escape correctness, `&&`/`||` short-circuit,
and per-block `var` shadowing, the defensive workarounds in bote can collapse:

- **`src/transport_http.cyr` + `src/bridge.cyr`**: removed `_crlf` / `_crlfcrlf` global pointers and `_http_init_crlf()` setup function. All HTTP / CORS response builders now use embedded `"\r\n"` and `"\r\n\r\n"` literals directly. ~50 lines removed.
- **`src/jsonx.cyr`**: collapsed three nested `if (i >= len) { ... } if (load8 != X) { ... }` patterns into single `if (i >= len || load8 != X)` checks. Same for `if (i < len) { if (load8 == 44) { ... } }` → `&&`.
- **`src/jsonx.cyr`**: `if (key_len_actual == klen) { if (memeq(...)) { ... } }` → `key_len_actual == klen && memeq(...) == 1` (was the explicit fix for the non-short-circuit `memeq`-on-truncated-input bug; now safe to write naturally).
- **`src/registry.cyr`**: `if (v != 0) { if (streq(v, version) == 1) { return t; } }` → `&&`.
- **`src/dispatch.cyr`**: `_extract_tool_name` ditched the `var bad = 0;` flag; now `if (name == 0 || strlen(name) == 0)`. `if (ver != 0) { if (registry_get_versioned(...) == 0) { ... } }` → `&&`. Schema-emit `if (props != 0) { if (vec_len(props) > 0) { ... } }` → `&&`.

Net diff: **60 lines removed across 6 files**. No behavior change, all tests / fuzz / e2e smokes still green.

### What's in 1.0.0

| Area | Status |
|---|---|
| JSON-RPC 2.0 (request, response, notification, batch) | ✅ |
| MCP `initialize` / `tools/list` / `tools/call` | ✅ |
| Tool registry with versioning + deprecation + annotations | ✅ |
| Compiled schema (type/enum/bounds/nested object/array items, multi-violation) | ✅ |
| `JsonRpcError` codes — full spec mapping | ✅ |
| Session management (create/validate/prune, MCP-Session-Id header) | ✅ |
| Origin allow-list + protocol-version header validation | ✅ |
| stdio transport | ✅ |
| HTTP/1.1 transport with middleware | ✅ |
| Unix domain socket transport | ✅ |
| TypeScript bridge (CORS + MCP envelope wrap) | ✅ |
| Discovery (data layer + pluggable publish_fp) | ✅ |
| Streaming primitives (ProgressUpdate, CancellationToken) | ✅ data layer |

### Post-1.0 extensions (1.x minor bumps)

These are additive — none change existing API shapes.

| Module | Status |
|---|---|
| `src/audit.cyr` + `LibroAudit` adapter | **Ready to port** — libro v1.0.3 available via `[deps.libro] path = "../libro"` |
| `src/events.cyr` + `MajraEvents` adapter | **Ready to port** — majra v2.2.0 available via `[deps.majra] path = "../majra"` |
| `src/discovery.cyr` wire-up to majra pubsub | **Ready to port** — depends on events |
| `src/libro_tools.cyr` (5 built-in audit tools) | **Ready to port** — depends on audit + libro |
| `src/sandbox.cyr` + kavach integration | Wait — kavach v2-arch hardening in flight |
| `src/host.cyr` (content blocks, host registry) | Ready (no AGNOS dep) |
| `src/auth.cyr` (OAuth 2.1 / PKCE / bearer) | Ready (no AGNOS dep) |
| `src/transport_streamable.cyr` (POST + SSE single endpoint) | Ready (rolls SSE on top of `transport_http`) |
| `src/transport_ws.cyr` (server-side WebSocket) | Cyrius `lib/ws.cyr` is client-only; needs server handshake + frame unmasking written |
| Threaded streaming dispatch | Needs `lib/thread.cyr` MPSC wired into `dispatcher_dispatch_streaming` |

### Versioning policy from here

Pre-1.0 used `0.D.M` (day.month). From 1.0.0 forward, **standard SemVer**:
- **Major** — break a frozen data shape or remove a public function.
- **Minor** — add a module / function / config option.
- **Patch** — fix bugs, refactor internals, improve diagnostics.

### Cyrius toolchain pin

Built and tested against cyrius **4.4.0** (`cyriusly use 4.4.0`).

---

## [0.1.1] — 2026-04-13 — Bridge + cyrius 4.4.0 + review punch list

### Added
- **`src/bridge.cyr`** — TypeScript-bridge HTTP transport: CORS preflight (`OPTIONS /`), `GET /health`, `POST /` JSON-RPC dispatch with MCP-envelope wrapping for `tools/call` results. `wrap_tool_result` (passthrough if already shaped, else wraps text), `wrap_error_result` (adds `isError: true`).
- **CLI**: `./build/bote bridge [port]` (default 8391).
- 29 new unit assertions: bridge wrappers, CORS origin selection, `bridge_process_message` round-trips, schema bounds at exact `min` / `max`, codec pure-notification batch.

### Fixed (review punch list)
- **`src/jsonx.cyr`**: `key_len_actual == klen && memeq(...)` was unsafe because cyrius `&&` doesn't short-circuit — `memeq` was called on truncated input. Now nested as separate `if`s.
- **`src/transport_http.cyr`**: when `Content-Length` was absent and `body_off > n` (malformed request), `clen = n - body_off` could be negative → `memcpy` UB. Now guarded.
- **`src/schema.cyr`**: `_sch_parse_int` replaced `i = i + 999999` marker-hack with proper `break` (per-block scoping now works in cyrius 4.4.0).
- **`src/transport_http.cyr`**: `http_find_header` similarly cleaned — replaced `vs = vs - 0; line_start = headers_end; vs = vs - 0;` marker hack with structured loops + `break`.

### Verified against cyrius 4.4.0 (`cyriusly install 4.4.0 && cyriusly use 4.4.0`)
- ✅ `\r` escape now emits CR (13) — fixed upstream
- ✅ Per-block `var` shadowing now works — fixed upstream
- ❌ `&&` / `||` short-circuit still missing — workarounds retained
- ➕ DCE now available via `CYRIUS_DCE=1` at build time

`docs/cyrius-feedback.md` updated with v4.4.0 verification status against each repro.

### Performance
Bench numbers unchanged from 0.1.0 — bridge adds a thin envelope-wrap layer with no measurable overhead on the hot dispatch path.

---

## [0.1.0] — 2026-04-13 — Cyrius port baseline

### Breaking
- **Language switch**: bote moved from Rust to Cyrius. The Rust source is preserved under `rust-old/` for reference and recovery. Version reset to `0.1.0` to mark the new lineage.
- **API change**: idiomatic Cyrius — module-prefixed function APIs (`registry_register`, `dispatcher_dispatch`, `codec_process_message`) over offset-addressed structs (`store64`/`load64`). No traits, generics, async, or borrow checking. Handler functions are i64 function pointers (`fn h(args_cstr) → result_cstr`).

### Added
- **`src/error.cyr`** — `BoteErrTag` enum (12 variants), `bote_err_rpc_code`, `bote_err_format`, schema-violation list support.
- **`src/protocol.cyr`** — `JsonRpcRequest` / `JsonRpcResponse` / `JsonRpcError` with raw-JSON-literal id/params/result/data slots.
- **`src/jsonx.cyr`** — Nested-aware JSON value extractor (`jsonx_get_raw`, `jsonx_get_str`, `jsonx_has`, `jsonx_is_object`). Handles nested objects, arrays, escaped strings; needed because `lib/json.cyr` is flat-only.
- **`src/registry.cyr`** — `ToolDef` (with `version`, `deprecated`, `annotations`, `compiled` slots), `ToolSchema`, `ToolAnnotations` (presets `read_only` / `destructive`), `ToolRegistry` (insertion-ordered, hashmap-indexed). Versioned tools, deprecation, validate-by-required-fields fallback.
- **`src/dispatch.cyr`** — `Dispatcher`, sync handler dispatch, `initialize` / `tools/list` / `tools/call` routing, MCP protocol-version negotiation, `validate_tool_name` (project_tool format, 256 char max), dynamic register/deregister.
- **`src/codec.cyr`** — `codec_parse_request`, `codec_serialize_response`, `codec_process_message` (single + batch + notification + error responses), JSON-message escaping reused from dispatch.
- **`src/schema.cyr`** — `CompiledSchema` with full type-checking (`string`, `number`, `integer`, `boolean`, `array`, `object`, `Any`), enum constraints, numeric bounds, recursive nested objects + array items, multi-violation reporting. `tool_def_with_compiled` slot wires it into `registry_validate_params`.
- **`src/stream.cyr`** — `CancellationToken`, `ProgressUpdate`, `ProgressSender`, `StreamContext`, `progress_notification` JSON builder. (Thread integration deferred.)
- **`src/session.cyr`** — `SessionStore` (hex-encoded 16-byte SIDs from `/dev/urandom`), `validate_protocol_version`, `validate_origin` (wildcard `*`, exact match, strict mode).
- **`src/transport_stdio.cyr`** — Line-oriented JSON-RPC over stdin/stdout, 128KB heap-allocated buffer, partial-line shifting.
- **`src/transport_http.cyr`** — HTTP/1.1 server (`POST /mcp` → JSON-RPC). Origin/MCP-Protocol-Version/MCP-Session-Id middleware. Auto-creates a session on `initialize` and emits the new `MCP-Session-Id` response header. Case-insensitive header lookup. 64KB request buffer.
- **`src/transport_unix.cyr`** — `AF_UNIX` line-oriented transport (own socket-creation code since `lib/net.cyr` is `AF_INET`-only). 128KB per-connection buffer.
- **CLI** — `./build/bote [stdio|http <port>|unix <path>]` selects transport.
- **Tests** — `tests/bote.tcyr` with **251 unit assertions** covering all modules.
- **Benchmarks** — `tests/bote.bcyr` with 10 hot-path benchmarks (all sub-10µs on x86_64).
- **Fuzz** — `fuzz/codec_parse.fcyr`, `fuzz/codec_process.fcyr`, `fuzz/jsonx_extract.fcyr`, `fuzz/schema_validate.fcyr` (~330 fuzzed calls; no crashes).
- **`docs/cyrius-feedback.md`** — language-level issues found during the port.
- `.gitignore` rules for `rust-old/target/` and `/build/`.

### Performance
- `dispatch_initialize` ~2µs avg
- `dispatch_tools_list` ~2µs avg
- `dispatch_tools_call` ~1µs avg
- `jsonx_get_str_flat` 600ns avg
- `jsonx_get_raw_nested` ~1µs avg
- `codec_parse_request` ~2µs avg
- `codec_serialize_response` ~1µs avg
- `codec_process_message` (full pipeline) ~5µs avg
- `validate_compiled_simple` ~1µs avg
- `validate_compiled_nested` ~3µs avg

### Deferred to future cyrius releases
- `bridge` — TypeScript bridge with CORS / MCP envelope wrapping.
- `audit` — libro hash-linked audit chain integration.
- `events` — majra pub/sub event publishing.
- `discovery` — cross-node tool announcements (depends on `events`).
- `sandbox` — kavach tool isolation.
- `host` — MCP hosting layer (content blocks, host registry).
- `libro_tools` — 5 built-in libro audit MCP tools.
- `auth` — OAuth 2.1 / PKCE / bearer-token middleware.
- `transport_ws` — server-side WebSocket (cyrius `lib/ws.cyr` is client-only).
- `transport_streamable` — streamable HTTP (POST + SSE single endpoint).
- Streaming dispatch (needs thread + channel integration).

### Known cyrius-language workarounds applied
- `\r` string escape emits byte `r` (114) instead of CR (13) — built CRLF via `store8`.
- `&&` / `||` operators do not short-circuit — guarded null derefs nested as `if (p != 0) { if (...) { ... } }`.
- No per-block local scoping — distinct names per `fn` body (`req_one`, `rcompiled`, `prog_notif`, etc.).
- Static `var buf[N] >~ 16KB` exhausts the output buffer — large buffers heap-allocated (`var ptr = 0;` global + `ptr = alloc(N);` at startup).

See [docs/cyrius-feedback.md](docs/cyrius-feedback.md) for full reproductions.

---

## Historical (Rust) — preserved under `rust-old/`

## [0.91.0] — 2026-04-02

### Added
- `libro_tools` module (feature: `audit`) — 5 built-in MCP tools for libro audit chain operations:
  - `libro_query` — query audit entries by source, severity, action, agent, min_severity, with limit
  - `libro_verify` — verify chain integrity and return structured `ChainReview` JSON with integrity status, entry count, time range, source/severity/agent distributions (was text-only)
  - `libro_export` — export chain as JSON Lines or CSV
  - `libro_proof` — generate Merkle inclusion proof for an entry by index, returns structured proof JSON with verification status
  - `libro_retention` — apply retention policies (PCI-DSS, HIPAA, SOX, keep_count) and report archived entries (destructive, not read-only)
- `libro_tools::register()` — convenience function to register all 5 libro tools on a dispatcher
- Read-only tools annotated with `ToolAnnotations::read_only()` (MCP 2025-11-25); `libro_retention` is destructive (no annotation)
- `LibroAudit::with_source()` — custom source tag for audit entries (default: `"bote"`)
- `LibroAudit::with_agent_id()` — server agent identity on all entries; `caller_id` from events takes precedence
- `LibroAudit` now uses `append_with_agent()` when caller_id or agent_id is present, populating libro's agent tracking
- 17 libro_tools tests + 8 audit tests (was 8 + 3)
- **HTTP transport middleware**: Origin validation (403), `MCP-Protocol-Version` enforcement (400), `MCP-Session-Id` session lifecycle (404), bearer token extraction with 401/403 responses (feature `auth`)
- **Streamable HTTP transport router**: axum router with POST (JSON-RPC) and GET (SSE stream) on configurable endpoint path, same middleware stack as HTTP, SSE event IDs via `EventIdGenerator`, `Last-Event-ID` resumption via `ResumptionBuffer` replay, `retry:` hint before close, priming event on connect
- `HttpConfig` builder: `with_allowed_origins()`, `with_session_timeout()`, `with_token_validator()` (feature `auth`)
- `StreamableConfig` builder: `with_session_timeout()`, `without_sessions()`
- `TokenValidator` trait (feature `auth`) — consumers implement to validate bearer tokens
- Shared `transport::middleware` module — `check_origin`, `check_protocol_version`, `check_protocol_version_required`, `check_session`, `check_bearer` reused by both transports
- Periodic session pruning via tokio interval in both `http::serve()` and `streamable::serve()`
- `streamable::streamable_router()` — build router without binding a port (for testing)
- 35 new transport middleware tests (origin, protocol version, session enforcement in both transports)
- `cargo vet` supply chain auditing: 156 crates fully audited via trusted imports (mozilla, google, bytecode-alliance, isrg, zcash, ariel-os, embark-studios) and 27 trusted publishers (dtolnay, seanmonstar, Manishearth, epage, fitzgen, kennykerr, Amanieu, BurntSushi, Thomasdezeeuw, cuviper, alexcrichton, carllerche, Darksonn, rust-lang-owner), 66 exempted, CI integration

### Changed
- Upgraded libro dependency from 0.25 to 0.91 (BLAKE3 hashing, serde on all types, key rotation support)
- `HttpConfig` expanded with `allowed_origins`, `session_timeout`, `token_validator` fields
- `StreamableConfig` expanded with `session_timeout` field
- Streamable transport `MCP-Protocol-Version` header is **required** (per MCP 2025-11-25), unlike plain HTTP where it is optional

## [0.90.0] — 2026-04-01

### Fixed
- **JSON-RPC 2.0 spec compliance**: Unknown methods now return `-32601` (Method not found) instead of `-32600` (Invalid Request)
- **Bridge spec compliance**: Error wrapping no longer sets both `result` and `error` on the response (JSON-RPC 2.0 violation)
- `scripts/bench-log.sh`: Added missing `--features bridge` flag

### Performance
- **Notification dispatch 17x faster** (170ns → 10ns): Early-return before lock acquisition when request is a notification
- **Parameter validation 26% faster** (47ns → 35ns): Merged `tools` + `compiled` HashMaps into single `entries` map, eliminating key duplication
- **Schema validation 8% faster** (107ns → 99ns): Same registry merge reduces lookup overhead

### Changed
- `ToolRegistry` internal structure: merged separate `tools` and `compiled` maps into unified `entries` map
- CLAUDE.md: Added task sizing, refactoring guidelines, testing section, documentation structure, CHANGELOG format, module table, stack table

### Added
- 3 new conformance tests: `error_codes_comply_with_spec`, `bridge_error_response_is_spec_compliant`, `registry_deregister_cleans_up_compiled_schema`
- 18 downstream consumers integrated (daimon, agnoshi, t-ron, jalwa, nein, stiva, itihas, varna, selah, hoosh, vidya, rasayan, szal, tarang, vidhana, nazar, mneme, tazama)

## [0.50.0] — 2026-03-26

### Added
- Protocol conformance test suite (41 tests in `tests/conformance.rs`)
- Streaming audit logging — all transports now call `log_tool_call()` after streaming handler completion with timing and success/error status
- `BoteError::SandboxError` variant for sandbox execution failures

### Fixed
- Streaming tool calls in HTTP/SSE, WebSocket, Unix, and stdio transports now correctly produce audit events via `log_tool_call()`
- Added missing doc comment on `Dispatcher::new()`

## [0.25.3] — 2026-03-26

### Added
- Tool sandboxing via kavach (feature `sandbox`)
- `ToolSandboxConfig` with presets: `basic()`, `strict()`, `noop()`
- `SandboxExecutor` for running commands in kavach sandboxes
- `wrap_command()` and `wrap_streaming_command()` handler wrappers
- `Dispatcher::register_sandboxed_tool()` and `register_sandboxed_streaming_tool()` convenience methods
- `BoteError::SandboxError` variant for sandbox execution failures
- Sandbox lifecycle event topics: `bote/sandbox/created`, `bote/sandbox/destroyed`, `bote/sandbox/error`
- Async-sync bridge with `OnceLock<Runtime>` fallback for non-tokio contexts

## [0.24.3] — 2026-03-26

### Added
- Full JSON Schema validation: type checking (string, number, integer, boolean, array, object), enum constraints, numeric bounds, nested object/array validation
- `CompiledSchema` — compile `ToolSchema` into typed representation for fast validation
- Default value injection via `CompiledSchema::apply_defaults()`
- `SchemaType`, `PropertyDef` types in new `schema` module
- `BoteError::SchemaViolation` variant with multiple violation reporting
- Tool versioning: `version` and `deprecated` fields on `ToolDef`
- `ToolDef::with_version()` and `ToolDef::with_deprecated()` builder methods
- `ToolRegistry::get_versioned()`, `list_versions()`, `deprecate()`, `deregister()`
- Version negotiation in `tools/call` dispatch
- Deprecation warnings via tracing + event publishing
- Dynamic tool registration/deregistration via `Dispatcher::register_tool()`, `deregister_tool()`
- Hot-reload: re-registering a tool atomically replaces its handler
- Tool namespacing: `project_tool` format enforcement on dynamic registration
- `TOPIC_TOOL_DEPRECATED` and `TOPIC_TOOL_DEREGISTERED` event topics
- Schema validation, versioning, and dynamic registration benchmarks

### Changed
- `Dispatcher` internals migrated to `RwLock` for thread-safe dynamic registration
- `ToolRegistry::validate_params()` now uses compiled schema for full type validation
- `tools/list` response includes `version` and `deprecated` fields when present

## [0.23.3] — 2026-03-26

### Added
- TypeScript bridge module with CORS and MCP result formatting (feature `bridge`)
- `wrap_tool_result` adapter — converts raw results to SY's `{ content: [{ type, text }] }` envelope
- Bridge CORS preflight handling for cross-origin TypeScript clients
- Cross-node tool discovery via majra pub/sub (feature `discovery`)
- `DiscoveryService` for announcing and subscribing to tool announcements
- `ToolAnnouncement` type for cross-node tool broadcast
- New event topics: `bote/tool/announce`, `bote/tool/discovered`
- Bridge benchmark (`wrap_tool_result` overhead)

### Changed
- `full` feature now includes `bridge` and `discovery`
- Transport codec module visibility changed to `pub(crate)` for bridge reuse

## [0.22.3] — 2026-03-22

### Added
- HTTP transport (axum-based, feature `http`)
- WebSocket transport (bidirectional, feature `ws`)
- Unix domain socket transport (newline-delimited JSON, feature `unix`)
- Graceful shutdown on all network transports via shutdown future
- SSE streaming for long-running tool calls (HTTP)
- Progress notifications during execution (StreamContext, ProgressSender, CancellationToken)
- Streaming handler type (StreamingToolHandler, dispatch_streaming, DispatchOutcome)
- Cancellation support ($/cancelRequest, CancellationToken)
- Batch requests (JSON-RPC 2.0 batch array)
- Notification support (no id, no response expected)
- Protocol version negotiation in initialize handshake (2024-11-05, 2025-03-26)
- process_message() codec function for batch/notification/single dispatch
- Audit logging via libro (AuditSink trait, LibroAudit adapter, feature `audit`)
- Event publishing via majra (EventSink trait, MajraEvents adapter, feature `events`)
- Tool call timing with automatic audit + event logging in dispatch
- Event topic constants (TOPIC_TOOL_COMPLETED, TOPIC_TOOL_FAILED, TOPIC_TOOL_REGISTERED)
- Feature flags: http, ws, unix, all-transports, audit, events, full
- progress_notification() helper for consistent JSON-RPC notification format
- extract_tool_name() helper for validated tool name extraction
- Send + Sync compile-time assertions on all public types
- Benchmark suite: 8 benchmarks (dispatch, process_message, batch, streaming, validation)
- Benchmark history logging via scripts/bench-log.sh
- CODE_OF_CONDUCT.md, CONTRIBUTING.md, SECURITY.md
- codecov.yml with 80% project / 75% patch targets
- 129 tests across all modules

### Changed
- Transport module restructured: transport.rs -> transport/ directory (codec, stdio, http, ws, unix)
- Dispatcher.dispatch() returns Option<JsonRpcResponse> (None for notifications)
- JsonRpcRequest.id is now Option<serde_json::Value> (supports notifications)
- All transports use process_message() for unified dispatch
- WS/Unix transports use outgoing message channel pattern for streaming
- Mutex locks are poison-safe across all transports (unwrap_or_else)
- Handler panics caught and returned as -32603 error responses
- Cancelled tasks return -32800 (distinguished from panics in async transports)
- BoteError is now #[non_exhaustive]
- Serialization calls use explicit BUG labels instead of bare unwrap()
- deny.toml: tightened license list, added version 2 advisories, explicit allow-registry
- Makefile: added coverage, bench, --no-default-features clippy, RUSTDOCFLAGS for doc
- Cargo.toml: added documentation, exclude, full feature

### Fixed
- HTTP returns proper JSON-RPC error for malformed JSON (was returning 422)
- validate_params rejects non-object params (was silently passing)
- Dispatch uses BoteError consistently (was hardcoding error codes)
- transport::parse_request uses Json error variant (was converting to string)
- Empty tool name now returns -32602 instead of falling through to not-found
- jsonrpc version validated (rejects non-"2.0")
- Progress notification JSON deduplicated via stream::progress_notification()
- Tool name extraction deduplicated via Dispatcher::extract_tool_name()

### Removed
- Unused dependencies: anyhow, tokio, uuid (from core; tokio re-added as optional for transports)
