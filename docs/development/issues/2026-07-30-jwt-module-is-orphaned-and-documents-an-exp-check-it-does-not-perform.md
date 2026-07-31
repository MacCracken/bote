# `src/jwt.cyr` ships in no bundle, and documents an `exp` check it does not perform

**Status:** 🟡 **OPEN** — filed 2026-07-30 against bote 3.1.4. Three findings, verified by reading:
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
`2026-05-10-opt-in-transport-profile.md`), and JWT verification is arguably transport-adjacent. A
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
