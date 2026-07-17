# `ERR_IO` enum constant collides ecosystem-wide — namespace `BoteErrTag` as `BOTE_ERR_*`

**Filed:** 2026-06-23 (by a hoosh consumer — hoosh 2.4.7 toolchain bump to cyrius 6.2.37)
**Resolved:** 2026-07-17 in bote `3.1.3` — the entire `BoteErrTag` enum was
prefixed `ERR_* → BOTE_ERR_*` (all 13 tags, `BOTE_ERR_TOOL_NOT_FOUND` …
`BOTE_ERR_TAG_COUNT`); the `bote_err_*` constructors, the `BoteErrTag → JSON-RPC
code` mapping, and every `src/` + test reference were updated, and
`dist/bote-core.cyr` + `dist/bote.cyr` regenerated (0 bare `ERR_`, 53 tags
each). No bare aliases were kept — a compatibility `ERR_IO` alias would
reintroduce the exact libro `ERR_IO=3` collision this fixes. The real in-tree
collision was against `lib/libro.cyr`'s `ERR_IO=3` / `ERR_JSON=4` in the
libro-linked default binary (bote's are `=11` / `=10`). Landed as a patch
(3.1.3), not the originally-suggested 2.8.0 — bote is well past that line.
**Severity:** Medium — `last-definition-wins` build warning today; latent
value-dependent-logic hazard when bote-core is compiled alongside another lib
that also defines a bare `ERR_IO`.
**Component:** `src/error.cyr:16` (`enum BoteErrTag { … ERR_IO = 11; … }`) →
`dist/bote-core.cyr:24`, `dist/bote.cyr:23`.
**bote's role: FIX OWNER for its own error/tag enum.** Part of a coordinated
ecosystem-wide error-enum namespacing effort (see Cross-references).
**Repos:** bote `2.7.6` (mirrors filed in sigil, yukti, sakshi, ai-hwaccel).

## Summary

Cyrius enum members are **global constants** — `BoteErrTag` does *not* namespace
them. `ERR_IO` (and siblings `ERR_PARSE = 4`, `ERR_TOOL_NOT_FOUND = 0`) collide
by name across the ecosystem:

| Library | Enum | `ERR_IO` | Source |
|---|---|---|---|
| **bote 2.7.6** | `BoteErrTag` | **11** | `src/error.cyr:16` → `dist/bote-core.cyr:24` |
| sigil 3.9.2 | `SigilError` | 6 | `src/error.cyr:15` |
| yukti 2.2.6 | `YuktiErrorKind` | 14 | `src/error.cyr:25` |

This is the exact warning hoosh sees when it pairs bote-core (MCP
`/v1/tools/*`) with sigil (audit chain):

```
warning:src/vendor/bote-core.cyr:24: duplicate symbol 'ERR_IO' redefined with conflicting value (last definition wins)
```

Cyrius include semantics are textual paste + **last-definition-wins (with a
warning)** — one global `ERR_IO` survives per binary, whichever bundle is last.

## Why this is more than a warning

Intra-module comparisons stay self-consistent after last-wins, but bote maps tags
to JSON-RPC codes (`bote_err_*` / the `BoteErrTag → code` mapping). Any path that
**serializes** the tag value, or that a consumer interprets as bote's documented
`11`, silently uses another lib's integer if bote's definition loses the link.

## The precedent already exists in-tree

`TLS_ERR_IO`, `PATRA_ERR_IO`, `SANDHI_ERR_TIMEOUT` already namespace exactly this
clash. bote should follow suit.

## Recommended fix

Prefix the **entire `BoteErrTag` enum** `ERR_* → BOTE_ERR_*` (e.g.
`BOTE_ERR_IO`, `BOTE_ERR_TOOL_NOT_FOUND`, `BOTE_ERR_TAG_COUNT`) and update the
`bote_err_*` constructors / JSON-RPC code mapping and every `ERR_*` reference
under `src/`. Regenerate `dist/bote-core.cyr` + `dist/bote.cyr`. Breaking change
to the exported tag surface → suggest **bote 2.8.0**, optionally keeping bare
aliases for one minor (note bote already namespaced its registry constructor in
the `registry_new` → `tool_registry_new` fix; this completes that hygiene).

## Interim (consumer-side)

hoosh tolerates the warning (last-wins benign for its reachable MCP paths). The
upstream rename retires it for all bote + sigil/yukti consumers (szal, mihi, hoosh).

## Cross-references

- sigil `…2026-06-23-err-io-enum-collision-namespace.md`.
- yukti `…2026-06-23-err-enum-collision-namespace.md`.
- sakshi / ai-hwaccel `…2026-06-23-err-timeout-enum-collision-namespace.md`.
- Prior bote namespacing: `2026-06-11-registry-new-collision.md` (registry_new → tool_registry_new).
