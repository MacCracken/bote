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
| `dist/bote.cyr`       | default  | 30      | Consumer needs bote's full transport surface |
| `dist/bote-core.cyr`  | `core`   | 12      | Consumer wraps Dispatcher / Registry / Prompts / Resources / Content / Audit but supplies its own transport |

- Every tagged release must commit **both** artifacts.
- Each bundle is a self-contained, include-free single `.cyr`
  file. Every public function / struct / global the profile
  exports lives in that one file.
- The file paths and names are fixed: `dist/bote.cyr` and
  `dist/bote-core.cyr`. Not `dist/bote-2.7.2.cyr`. Not
  `build/bote.cyr`. Not `bote.cyr` at the repo root.

If either bundle is missing at the tag, downstream consumers
that selected that profile break at `cyrius deps` time.

### Stdlib the consumer must supply

The bundles are include-free by design — they carry **no**
`include "lib/…"` lines, so the consumer's own `[deps] stdlib`
has to cover every stdlib leaf the fold calls. Since cyrius
6.6.6 the auto-generated `dist/bote.deps` sidecar names exactly
that: the 31 modules in bote's `[deps] stdlib` plus `ws_server`
(32 leaves; `dist/bote-core.deps` names 11). The include-closure
behind them — `hashseed`, `sha1`, `tls_native`, `result`, … —
is no longer listed because `cyrius deps` pulls it itself (a
clean-room consumer resolving from the sidecar alone gets 81
files and builds; measured at 3.3.10). Mirror bote's own
`[deps] stdlib` list in `cyrius.cyml` — including its
**ordering**, which is load-bearing under single-pass
compilation.

**New at 3.2.0: `random`.** `src/session.cyr`'s
`_gen_session_id` now draws entropy through `random_bytes()`
(getrandom(2)) instead of opening `/dev/urandom` by raw
syscall. A consumer whose stdlib list omits `random` gets
`undefined function 'random_bytes'`. Add it before the modules
that reference it — bote lists it alongside `ct` / `keccak`,
ahead of `sigil`.

## Resolving the bundle: `cyrius lib sync` BEFORE `cyrius deps`

⚠ **Read this before filing a resolver bug.** The most common
first-time failure consuming bote (or the bote → libro → majra graph)
looks like a broken resolver and is not one:

```
dep libro requires 'ct' ... is not in the cyrius stdlib
```

`ct` **is** in the cyrius stdlib. The message is misleading (filed
upstream as `2026-08-12-agnosai-deps-misleading-stdlib-error`). What it
actually means is that `./lib/` does not yet contain that module.

Cyrius deliberately does **not** auto-resolve stdlib — that is a
supply-chain choice, not an oversight. Two things follow, and the
**order matters**:

1. Declare every transitive stdlib module your graph reaches in your own
   `[deps] stdlib`. For the full bote bundle plus libro/majra that means
   adding, beyond the obvious ones, `ct`, `keccak`, `random`, `slice`,
   `thread`, `thread_local`, `sync`, `atomic`, `result`, `sigil` — and
   `ws_server` only if you use the WebSocket transport.
2. Run **`cyrius lib sync --full`** to provision `./lib/` from the pinned
   toolchain snapshot, and only *then* `cyrius deps`, which **overlays**
   your declared `[deps.<name>]` git deps on top of that snapshot.

```sh
cyrius lib sync --full   # provision ./lib/ from the pinned snapshot
cyrius deps              # overlay [deps.*] git deps on top
cyrius deps --verify     # confirm against cyrius.lock
```

Running `cyrius deps` against an empty `./lib/` fails on the stdlib
leaves each dep's `dist/<pkg>.deps` sidecar names. This is exactly what
bote's own CI does, in this order — see `.github/workflows/ci.yml`.

⚠ Two traps worth stating, both of which have cost real time:

- **`cyrius build` does an implicit resolve.** So verify a vendored
  dependency's version *after a build*, not after `cyrius deps` — bote
  had a case (3.3.1) where the three-step ended correct and the very
  next build silently reverted a file.
- **A local `path =` beats `tag =`** and vendors your working tree, which
  can mask a wrong or unpushed tag. Your machine passes; a clean CI
  checkout resolving from `git + tag` gets different bytes and fails
  `cyrius deps --verify`.

**nein 1.6.0 vendored `bote-core.cyr` outright over this misread; 1.6.1
retired the vendoring** and consumes bote-core + sigil as ordinary git
deps, the same way daimon does. Vendoring is not the fix — the sync step
is.

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
`docs/development/issues/archive/2026-05-10-opt-in-transport-profile.md`
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

The cyrius major jump to 6.1.24 (bote 2.7.3) raised the cap and
the core profile **stays** — it's still a smaller compile-source
footprint and a faster CI for consumers that don't need the
transports. The same jump unblocked folding bote's own three
per-transport binaries back into one; that is still in place and
is a 3.4.x roadmap item. The bundles are unaffected either way.

## What lives in the core 12?

| #  | File                  | Role                                          |
|----|-----------------------|-----------------------------------------------|
| 1  | `src/error.cyr`       | `BoteError` tagged enum                       |
| 2  | `src/protocol.cyr`    | `JsonRpcRequest` / `Response` / `Error`       |
| 3  | `src/jsonx.cyr`       | JSON helpers                                  |
| 4  | `src/registry.cyr`    | `ToolRegistry`                                |
| 5  | `src/prompts.cyr`     | `PromptRegistry` (MCP prompts capability)     |
| 6  | `src/resources.cyr`   | `ResourceRegistry` (MCP resources capability) |
| 7  | `src/events.cyr`      | `EventSink`                                   |
| 8  | `src/audit.cyr`       | `AuditLogger` / `AuditSink`                   |
| 9  | `src/dispatch.cyr`    | `Dispatcher` (2.0 handler ABI)                |
| 10 | `src/codec.cyr`       | Encoder / decoder                             |
| 11 | `src/schema.cyr`      | Schema compile                                |
| 12 | `src/content.cyr`     | Typed MCP content blocks + annotations        |

(The core profile grew 9 → 11 at 3.0.0, when the MCP prompts and
resources capabilities landed, and 11 → 12 at **3.3.6** with
`content.cyr`. `src/sandbox.cyr` was weighed at 3.3.13 and kept
OUT: it fits mechanically — no sigil, no transport, no new stdlib
leaf — but this profile exists to bound a consumer's compile set,
and while eight repos vendor `bote-core.cyr`, none references a
`sandbox_*` symbol. It joined the FULL bundle in the same release.
Revisit if a core-profile consumer wires a sandbox backend.)

`content.cyr` is in core because content blocks are the tool-result
format **every** handler emits, transport or not. Before 3.3.6,
core-profile consumers (nein's `mcp` module, t-ron) hand-rolled
`{"content":[…],"isError":…}` with a raw `str_builder` — which means
each of them re-implemented JSON string escaping, the exact duplicated
injection surface this profile exists to prevent. It adds **no** new
stdlib leaves: its only external references are `_json_emit_escaped`
(already in core via `dispatch.cyr`), `str_builder_*` / `str_data`,
`vec_*` and `alloc`. It is listed **last** in `[lib.core]` because
cyrius is single-pass and it calls `_json_emit_escaped`.

Stdlib footprint: `string`, `fmt`, `alloc`, `vec`, `str`,
`tagged`, `assert`, `fnptr`, `hashmap`, `bayan`, `chrono`,
`freelist`. No `tls` / `sandhi` / `sigil` / `ws_server` / `slice`.

That "no `sigil`" is why `src/jwt.cyr` and `src/pkce.cyr` are in
`[lib]` only and **not** in `[lib.core]`, even though both ship
in the full bundle from 3.2.0: jwt needs `hmac_sha256` and pkce
needs `sha256`. Pulling the sigil bundle into the transport-free
profile would defeat the reason that profile exists. A consumer
who wants JWT from the core bundle has to argue for it.

Drift guard: `tests/bote_core_only_smoke.tcyr` includes only
`dist/bote-core.cyr` plus the minimal stdlib and runs a
`dispatcher_new + registry_register + dispatcher_handle`
round-trip. If a future bote change wires a core-module symbol
against a transport-module helper, the smoke fails at CI time.

Since 3.3.6 it also asserts the content-block **bytes**, not just that
they link: that a quote and a backslash come back escaped, and that
`content_array` omits `isError` while `content_array_error` emits
`"isError":true`. A link-only check would not notice an escaping
regression, and escaping is the whole reason `content.cyr` belongs in
this profile. Both assertions are mutation-proven — expecting the raw
unescaped string exits 7, and asserting `isError` on the success array
exits 9.
