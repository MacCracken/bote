# `registry_new` symbol collision — bote-core × ai-hwaccel

**Filed:** 2026-06-11 (discovered during the szal Rust→Cyrius port, M2 engine arc)
**Resolved:** 2026-06-11 in bote `2.7.4` — bote renamed its constructor
`registry_new` → `tool_registry_new`.
**Severity:** Medium — blocked any Cyrius consumer that includes BOTH
`dist/bote-core.cyr` (or `dist/bote.cyr`) and `dist/ai-hwaccel.cyr` in one
compile unit. szal was the first such consumer (its `engine_hardware` module
was blocked).
**bote's role:** bote-core was one of the two colliding exporters and is the
one that took the rename. `registry_new` was bote's long-standing ToolRegistry
constructor; bote moved to the lib-descriptive `tool_registry_new` (parallel to
the existing `host_registry_new`), which unblocks the consumers without
depending on an ai-hwaccel release.
**Repos:** bote `2.7.4` · ai-hwaccel `2.3.9` (mirror filed in ai-hwaccel and szal).

## Summary

Two ecosystem libraries exported a **public function with the same name but
different identity**:

| Library | Symbol | Shape | Source |
|---|---|---|---|
| bote-core ≤2.7.3 | `fn registry_new()` | **24-byte** tool registry `{entries map@0, versions map@8, names vec@16}` | `src/registry.cyr:148` (→ `dist/bote-core.cyr:554`, `dist/bote.cyr:553`) |
| ai-hwaccel 2.3.9 | `fn registry_new()` | **32-byte** profile registry (`REGISTRY_SIZE=32`: `{profiles, warnings, system_io, schema}`) | `src/registry.cyr:20` (→ `dist/ai-hwaccel.cyr:3549`) |

Cyrius include semantics are textual paste + **last-definition-wins (with a
warning)**. A consumer that includes both bundles got exactly ONE `registry_new`
— whichever was included last — and every caller of the other one silently
allocated/interpreted the wrong struct layout (24 vs 32 bytes), corrupting memory.

## Why include order couldn't fix it

ai-hwaccel's own detection path calls `registry_new()` **internally**
(`registry_detect*`, `lazy`, `async_detect`), so even a consumer that only used
bote's `registry_new` directly still tripped the collision the moment it also
called ai-hwaccel detection. There was no include order that gave both libraries
a correct `registry_new`.

## Resolution — bote 2.7.4

**bote renamed `registry_new` → `tool_registry_new`** (`src/registry.cyr`,
both dist bundles regenerated). The new name is lib-descriptive — it constructs
bote's `ToolRegistry` and parallels the existing `host_registry_new`
(HostRegistry) constructor in `src/host.cyr`. Only the constructor changed;
every other `registry_*` accessor (`registry_register`, `registry_get`,
`registry_list`, `registry_validate_params`, …) keeps its name — none of those
collide with ai-hwaccel.

Taking the rename on bote's side (rather than waiting on ai-hwaccel's
`hw_registry_new` rename) unblocks the paired consumers immediately and removes
the colliding symbol from the more widely-included bundle.

**Consumer migration:** replace `registry_new()` with `tool_registry_new()`.
Struct layout, accessors, and dispatcher wire-up are unchanged. Consumers that
pair bote + ai-hwaccel (szal, mihi, hoosh) can now include both bundles in one
compile unit.

**ai-hwaccel:** the parallel `registry_new` → `hw_registry_new` rename is still
worthwhile for ai-hwaccel's own namespace hygiene, but is no longer a blocker
for bote consumers.

## Cross-references

- szal `docs/development/issues/2026-06-11-registry-new-collision.md` (the blocker record).
- ai-hwaccel `docs/development/issues/2026-06-11-registry-new-collision.md`.
- szal port-plan §3.3 (flagged pre-port as "Open Q9").
