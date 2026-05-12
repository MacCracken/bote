# BOTE IS A CYRIUS DEP — READ THIS BEFORE TOUCHING BUILD / RELEASE

**This file is non-negotiable. Do not invent an alternative
distribution mechanism. Do not ignore it because "it seems to
work without it". libro / patra / majra are the references.
Copy them.**

---

## Who consumes bote

bote is an **upstream Cyrius library** — the MCP core service.
Downstream Cyrius projects pull bote into their `cyrius.cyml`
as a git-tagged dep. Known-intended consumers per bote's own
`CLAUDE.md` include **phylax, t-ron, sutra, jalwa, rasa, mneme,
daimon**. Any of them (or any future project) wires bote in
like this:

```toml
[deps.bote]
git = "https://github.com/MacCracken/bote.git"
tag = "<bote version>"
modules = ["dist/bote.cyr"]      # full bundle (default)
```

`cyrius deps` clones bote at the tag and copies
`dist/bote.cyr` into the consumer's `lib/`. That's the entry
point they `include` from.

## The contract

bote ships **two** distribution artifacts:

| Artifact              | Profile  | Modules | Use when                                     |
|-----------------------|----------|---------|----------------------------------------------|
| `dist/bote.cyr`       | default  | 23      | Consumer needs bote's full transport surface |
| `dist/bote-core.cyr`  | `core`   | 9       | Consumer wraps Dispatcher / Registry / Audit but supplies its own transport |

- Every tagged release must commit **both** artifacts.
- Each bundle is a self-contained, include-free single `.cyr`
  file. Every public function / struct / global the profile
  exports lives in that one file.
- The file paths and names are fixed: `dist/bote.cyr` and
  `dist/bote-core.cyr`. Not `dist/bote-2.7.2.cyr`. Not
  `build/bote.cyr`. Not `bote.cyr` at the repo root.

If either bundle is missing at the tag, downstream consumers
that selected that profile break at `cyrius deps` time.

## Profile selection

Most consumers want the default:

```toml
# Default — recommended unless you have a reason
[deps.bote]
git = "https://github.com/MacCracken/bote.git"
tag = "2.x.x"
modules = ["dist/bote.cyr"]
```

Consumers that only use bote's dispatch surface (Dispatcher /
ToolRegistry / Audit / Codec / Schema) and supply their own
transport stack should use the core-only bundle:

```toml
# Core-only — when your consumer wraps bote's Dispatcher /
# Registry / Audit surface but supplies its own transport
# (e.g. t-ron's SecurityGate middleware over a custom socket).
[deps.bote]
git = "https://github.com/MacCracken/bote.git"
tag = "2.x.x"
modules = ["dist/bote-core.cyr"]
```

The core-only bundle excludes the transport stack
(`transport_stdio`, `transport_http`, `transport_unix`,
`bridge`, `transport_streamable`, `transport_ws`), the
session / discovery / auth / content / host modules, and the
audit_libro / events_majra adapters. The consumer supplies its
own stdlib without `sandhi` / `tls` / `ws_server` /
`sigil 3.x` — a much smaller compile-source budget that fits
under the cyrius 5.10.x 2 MB cap with room for the consumer's
own modules. See
`docs/development/issues/2026-05-10-opt-in-transport-profile.md`
for the rationale and module-split derivation.

## How to produce both bundles

`cyrius distlib` reads `[lib]` (default) or `[lib.<profile>]`
(named profile) from `cyrius.cyml` and emits the matching
`dist/<name>[-<profile>].cyr` deterministically:

```sh
cyrius distlib            # → dist/bote.cyr
cyrius distlib core       # → dist/bote-core.cyr
```

Run **both** commands:

1. **Locally** whenever `src/*.cyr` changes — verify both
   bundles are up to date, then commit them.
2. **In the release workflow** (`.github/workflows/release.yml`)
   before any `git archive` / asset-upload step.

CI gates the dist-freshness for both bundles. See `.github/workflows/ci.yml`.

## Why two profiles?

The transport stack is what drives bote's transitive stdlib
footprint up: pulling `lib/bote.cyr` forces `lib/sandhi.cyr`
(466 KB — HTTP server), `lib/tls.cyr` (31 KB), `lib/sigil.cyr`
(318 KB), and `lib/ws_server.cyr` (11 KB) into the consumer's
expanded-source budget. The 12 transport-and-above modules add
roughly 700 KB of transitive stdlib that pure dispatch consumers
never call.

The core-only bundle was triggered by `t-ron 2.1.x` hitting the
cyrius 5.10.x 2 MB cap when adopting the dist-bundle pattern.
The split is mechanically clean (9 modules, no transitive
transport deps) and consumer-side documentation in t-ron's own
`cyrius.cyml` flips to `modules = ["dist/bote-core.cyr"]` on
the 2.7.2 bump.

When bote migrates to cyrius 5.11.x (cap raised to 4 MB per the
companion proposal), the core profile **stays** — it's still a
smaller compile-source footprint and a faster CI for consumers
that don't need the transports. The migration just removes the
per-transport binary split inside bote itself (see CHANGELOG
2.7.2).

## What lives in the core 9?

| # | File                  | Role                                          |
|---|-----------------------|-----------------------------------------------|
| 1 | `src/error.cyr`       | `BoteError` tagged enum                       |
| 2 | `src/protocol.cyr`    | `JsonRpcRequest` / `Response` / `Error`       |
| 3 | `src/jsonx.cyr`       | JSON helpers                                  |
| 4 | `src/registry.cyr`    | `ToolRegistry`                                |
| 5 | `src/events.cyr`      | `EventSink`                                   |
| 6 | `src/audit.cyr`       | `AuditLogger` / `AuditSink`                   |
| 7 | `src/dispatch.cyr`    | `Dispatcher` (2.0 handler ABI)                |
| 8 | `src/codec.cyr`       | Encoder / decoder                             |
| 9 | `src/schema.cyr`      | Schema compile                                |

Stdlib footprint: `string`, `fmt`, `alloc`, `vec`, `str`,
`tagged`, `assert`, `fnptr`, `hashmap`, `json`, `chrono`,
`freelist`. No `tls` / `sandhi` / `sigil` / `ws_server` / `slice`.

Drift guard: `tests/bote_core_only_smoke.tcyr` includes only
`dist/bote-core.cyr` plus the minimal stdlib and runs a
`dispatcher_new + registry_register + dispatcher_handle`
round-trip. If a future bote change wires a core-module symbol
against a transport-module helper, the smoke fails at CI time.
