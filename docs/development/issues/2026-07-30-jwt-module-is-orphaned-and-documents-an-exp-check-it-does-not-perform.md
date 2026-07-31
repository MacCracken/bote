# `src/jwt.cyr` ships in no bundle, and documents an `exp` check it does not perform

> ## ✅ RESOLVED in 3.2.0 — 2026-07-30
>
> Findings (1) and (2) are fixed, in the order the report insisted on. Finding (3) is
> accepted as a correction and left as an open **decision** rather than a defect. Both
> "cautions if RS256 is ever added" were acted on now rather than deferred. See
> "Resolution" at the bottom.

**Status (as filed):** 🟡 **OPEN** — filed 2026-07-30 against bote 3.1.4. Three findings, verified by reading:
`src/jwt.cyr`'s only mention in `cyrius.cyml` is inside a **comment** (line 200), so it is in neither
`[lib] modules` nor `[lib.core] modules`; `grep -c '^fn jwt_verify_hs256' dist/bote.cyr
dist/bote-core.cyr` returns **0 and 0**; and `grep -cE '\bexp\b' src/jwt.cyr` returns **1** — the doc
line itself — with **zero** references to any clock or epoch function.
**Placement:** unpinned. (1) and (2) are independent; (2) is the one with a security consequence.
**Discovered:** 2026-07-30 while diagnosing whether agnosai's `server/auth.rs` port was blocked on a
bote `jwt_verify_rs256`. It is not — but the investigation surfaced these.
**Severity:** Medium. Nothing is broken for an existing consumer, because no consumer can reach the
module. The `exp` documentation is the sharp edge: it is a false security claim.
**Affects:** bote 3.1.4 and earlier, back to whenever `src/jwt.cyr` left the bundle lists (2.2.0
shipped HS256, so this may have been true from the start).

## 1. The module is not in either bundle

`src/jwt.cyr` defines `jwt_verify_hs256`, `jwt_b64u_decode`, `jwt_secret_new`, `jwt_secret_data`,
`jwt_secret_len`, `auth_validator_jwt_hs256`, `_jwt_dot` and `_jwt_ct_eq`. None of them reach a
consumer:

```sh
grep -n 'src/jwt.cyr' cyrius.cyml
# 200: # hmac_sha256 / ed25519_init (src/jwt.cyr, src/pkce.cyr); ct / keccak /   <- a COMMENT

grep -c '^fn jwt_verify_hs256' dist/bote.cyr dist/bote-core.cyr
# dist/bote.cyr:0
# dist/bote-core.cyr:0
```

`[package].description` advertises "JWT HS256 + RFC 7636 PKCE". Neither bundle ships either — `pkce.cyr`
is in the same position.

**Fix:** one line in `[lib] modules` plus `cyrius distlib`. Whether it also belongs in `[lib.core]` is
a real decision rather than an oversight: that profile exists to bound the consumer compile set (see
`archive/2026-05-10-opt-in-transport-profile.md`), and JWT verification is arguably transport-adjacent. A
consumer wanting it from the core bundle would have to argue for it.

## 2. The `exp` check is documented but absent

`src/jwt.cyr`'s header states, under "Validation performed":

```
#   - `exp` claim, if present in the payload, is in the future
#     (uses chrono's wall-clock seconds)
```

There is no such check. `exp` appears exactly once in the file — in that line — and the module
references no clock function at all:

```sh
grep -cE '\bexp\b' src/jwt.cyr          # 1  (the doc line)
grep -cE 'clock_|epoch' src/jwt.cyr     # 0
```

So `jwt_verify_hs256` accepts a **structurally valid, correctly signed, arbitrarily expired token**.

This is the finding worth acting on independently of everything else. A reader of that comment
reasonably concludes expiry is enforced, and a consumer that builds an auth path on it inherits a
gap it was told did not exist. It is currently unreachable — which is the only reason this is Medium
rather than High — but fixing (1) without fixing (2) would ship the gap to every consumer at once.

**Fix:** either implement it (~5 lines; `chrono` is already in `[deps] stdlib`, verified) or correct
the comment (one line). Implementing is better, but **do not fix (1) before one of the two.**

## 3. The RS256 gate premise is stale

`src/jwt.cyr` says:

```
# Scope: HS256 only. RS256 / ES256 need an asymmetric primitive sigil
# doesn't yet expose; tracked for a later release.
```

and `docs/development/roadmap.md:148` repeats it:

```
| **JWT RS256 / ES256** | sigil RSA / ECDSA primitives. HS256 already shipped in 2.2.0. |
```

sigil exposes those primitives now, and bote already depends on it — `[deps.sigil]` at tag `3.12.0`
(`cyrius.cyml:204-207`). `rsa_pkcs1v15_verify_sha256` and `rsa_pubkey_from_der` are both available;
I verified them end to end from a real SPKI PEM and an openssl-signed RS256 token while investigating
this, and the verification returns 1 for a good signature and 0 for both a tampered input and a
tampered signature.

So RS256 is unblocked whenever bote wants it. This is not a request that it be built — agnosai is
implementing RS256 locally, since it needs claim validation (`iss`/`aud`/`exp`) that bote has no
concept of, and going through bote would be indirection with no shared code. It is only a note that
the stated reason for deferring is no longer true, so the decision should be re-taken on its merits.

**Two cautions if RS256 is ever added here**, both from reading the HS256 implementation:

- The `alg` check is a **substring scan** of the decoded header, not a JSON field read. So
  `{"alg":"none","kid":"HS256-2024"}` satisfies it. Harmless while HS256 is the only algorithm and
  the secret still has to match — but the moment a module verifies more than one algorithm, a
  substring `alg` check is the classic algorithm-confusion vector. agnosai's port reads `alg` as a
  parsed JSON field and requires exact equality for this reason.
- The symbols are unprefixed (`jwt_verify_hs256`, `jwt_b64u_decode`, `_jwt_dot`). Cyrius has one flat
  namespace with last-definition-wins, so if this module ever enters `bote-core.cyr`, a consumer that
  has rolled its own `jwt_*` helpers gets a silent collision.

## Consumer-side workaround

None needed. agnosai reaches none of this — it pins `modules = ["dist/bote-core.cyr"]`, and calls no
bote symbol at all today. Filed because the `exp` documentation is wrong in a way that would mislead
the next reader, not because anything is blocked.

---

## ✅ Resolution — bote 3.2.0, 2026-07-30

The report was accurate on all three findings and correct about the ordering constraint. Fixed in
the order it demanded: **(2) before (1)**, so the gap was never shipped to consumers even
transiently.

### (2) `exp` is now enforced — FIXED

`_jwt_exp_ok(payload, plen)` in `src/jwt.cyr`. Four decisions the original comment never had to
make, made explicitly:

| Case | Behaviour | Why |
|---|---|---|
| `exp` absent | **accept** | RFC 7519 §4.1.4 makes the claim optional |
| `exp` in the past, or `exp == now` | **reject** | §4.1.4 requires now to be *strictly before* exp |
| `exp` present but not a JSON number (`"1785461260"`, `null`, `true`, `1e9`, 25 digits) | **reject** | §2 defines NumericDate as a JSON number. Folding "malformed" into "absent" is the fail-open direction — it would buy a crafted claim unlimited lifetime |
| `clock_epoch_secs() == 0` | **skip the window** | `lib/chrono.cyr:50` documents 0 as *"clock unknown"*, not the epoch. Treating it as 0 would expire every token and lock every client out. Matches sigil's `_x509_in_window`, which does exactly this |

**No leeway.** RFC 7519 §4.1.4 permits "some small leeway" for clock skew; bote declines it. A
resource server that silently honours a token past its stated expiry is doing something the
operator cannot see in the logs, and the size of that grace would be an invisible security
parameter.

**Ordering is load-bearing.** The report noted `_jwt_ct_eq` was the function's terminal statement.
It now short-circuits (`if (... == 0) { return 0; }`) and the payload is decoded and inspected only
below that line — so claim parsing never runs on unauthenticated bytes. The `alg` check stays
*above* the HMAC on purpose: it is the downgrade guard, so it has to gate what the signature step
will even attempt.

The claim readers deliberately use jsonx's `(buf, len)` internals (`_jx_find_value_pos`,
`_jx_skip_string`, `_jx_skip_value`) rather than `jsonx_get_str`. Every public jsonx entry point
starts with `strlen()`, and an attacker-embedded NUL in the decoded payload would truncate that
view — hiding `exp` entirely, which reads as "no expiry". bayan's base64url decoder *does*
NUL-terminate its output, so the cstr path would have compiled and appeared to work. That is
precisely what made it worth avoiding.

`_jwt_parse_uint` returns **-1**, not 0, on any parse failure — a NumericDate is never negative, so
the sentinel is unambiguous and 0 cannot double as "absent". Its 18-digit cap makes i64 overflow
impossible rather than merely unlikely; an overflowing `exp` would wrap negative and read as
long-expired, which is fail-closed *by accident*, and we would rather reject on purpose.

### (1) Both modules ship — FIXED

`src/jwt.cyr` and `src/pkce.cyr` are in `src/main.cyr` and in `cyrius.cyml [lib]`.
`dist/bote.cyr` goes 28 → 30 module folds and now exports `jwt_verify_hs256`,
`auth_validator_jwt_hs256`, `jwt_secret_*`, `pkce_code_verifier` and
`pkce_code_challenge_s256`. `[package].description`'s "JWT HS256 + RFC 7636 PKCE" is true for the
first time since 2.2.0.

**`[lib.core]`: excluded, deliberately** — answering the open question at lines 34-37 of this
report. `DEPS-PATTERN.md` documents the core profile's stdlib footprint as explicitly excluding
sigil, and both modules need it (`hmac_sha256` for jwt, `sha256` for pkce). Pulling the sigil
bundle into the transport-free profile would defeat the reason that profile exists. A consumer who
wants JWT from the core bundle has to argue for it; the answer is not automatic.

Capacity cost of the fold: `fn_table` 4961 → 4974, identifiers 163966 → 164307. Against a 32768 /
524288 ceiling, i.e. 15% / 31%. Not a factor.

### (3) The RS256 premise — CORRECTED, decision left open

Accepted as a correction. `src/jwt.cyr`'s "sigil doesn't yet expose" note and the matching
`roadmap.md` row are rewritten: RS256 is unblocked and therefore a **decision**, not a dependency.
Not built — agnosai implements it locally because it also needs `iss`/`aud` validation bote has no
concept of, so routing through bote would be indirection over no shared code.

### Both "cautions if RS256 is ever added" — acted on now, not deferred

- **The substring `alg` scan.** The report called this "harmless while HS256 is the only
  algorithm". True, and it was still replaced with an exact JSON field read
  (`_jwt_str_field_eq`), because the caution is only sound as long as nobody forgets it. The old
  scan accepted `{"alg":"none","kid":"HS256-2024"}` — the literal sits inside the `kid` *value*.
  The new regression test mints that exact header with a **correct** HMAC so nothing else can
  reject it, and is **mutation-proven**: restoring the substring scan makes it fail with
  `got 1, expected 0`.
- **Unprefixed symbols.** Re-verified rather than assumed: `jwt_*` / `pkce_*` / `_jwt*` / `_pkce*`
  have **zero** hits across all 71 `lib/*.cyr` (stdlib snapshot + libro + majra + sigil + sakshi +
  patra) and both dist bundles. No rename is forced today. The concern was scoped to
  `bote-core.cyr`, which these modules do not enter.

### Coverage

`tests/bote_jwt.tcyr` goes **28 → 53 assertions**, and gains an `_mint_hs256(hdr, payload, key)`
helper — every token in the file used to be a hardcoded literal, which is why the `exp` cases
(whose value must move with the wall clock) and the alg-confusion case (which needs a *valid*
signature over a hostile header) could not be expressed at all.

**Mutation-proven, both gates:**

| Mutant | Assertions killed |
|---|---|
| `alg` reverted to the pre-3.2.0 substring scan | 2 — incl. `alg confusion: HS256 inside kid does not satisfy alg` |
| `exp` gate removed (pre-3.2.0 behaviour) | 8 — every expiry and malformed-claim case |

The report also noted the pre-existing `alg=none` literal never reached the alg gate: its third
segment is empty, so `dot2 >= tlen - 1` rejects it structurally first. Confirmed. That assertion is
kept as a malformed-input case and **relabelled** so it no longer claims coverage it never had.

Full suite: **811 assertions, 0 failures, on x86_64 *and* aarch64.**

### One thing this fix made urgent

Folding `src/pkce.cyr` into the bundle would have shipped a second aarch64-undefined `SYS_OPEN`
call site to every consumer — `src/session.cyr` already had one. Both were fixed first, in the
same release, under
[`2026-07-17-aarch64-sys-open-urandom.md`](2026-07-17-aarch64-sys-open-urandom.md). Consumers must
add `random` to their `[deps] stdlib`; see `DEPS-PATTERN.md`.
