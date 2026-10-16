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
| `dist/bote.cyr`       | default  | 33      | Consumer needs bote's full transport surface |
| `dist/bote-core.cyr`  | `core`   | 13      | Consumer wraps Dispatcher / Registry / Prompts / Resources / Content / Audit but supplies its own transport |

- Every tagged release must commit **both** artifacts.
- Each bundle is a single `.cyr` file. Every public function /
  struct / global the profile exports lives in that one file.
  Since cyrius 6.6.18 (bote 3.3.17) it opens with a
  compile-verified `# Requires` block, one
  `include "lib/<leaf>.cyr"` per leaf of its `.deps` sidecar, so
  `include "dist/bote.cyr"` alone compiles. It carries no other
  `include`. For a `cyrius deps` consumer the block changes
  nothing: include-once is keyed on the literal path, and these are
  the same lines it auto-prepends.
- Each bundle ships with its sidecar (`dist/bote.deps`,
  `dist/bote-core.deps`). CI gates both files of each pair
  byte-for-byte against `cyrius distlib`.
- The file paths and names are fixed: `dist/bote.cyr` and
  `dist/bote-core.cyr`. Not `dist/bote-2.7.2.cyr`. Not
  `build/bote.cyr`. Not `bote.cyr` at the repo root.

If either bundle is missing at the tag, downstream consumers
that selected that profile break at `cyrius deps` time.

### Stdlib the consumer must supply

Since cyrius 6.6.18 a sidecar names exactly the stdlib leaves
`distlib`'s compile-verify proves the bundle needs, and nothing
from the producer's own `[deps] stdlib`. Everything those leaves
need arrives through their own requires blocks. At 3.3.17:

| Sidecar | Leaves |
|---|---|
| `dist/bote.deps` | `hashmap bayan ws_server syscalls` (4; was 32 through 3.3.16) |
| `dist/bote-core.deps` | `hashmap bayan sigil ct` (4; was 8) |

`cyrius deps` reads the sidecar and vendors those leaves. **A
consumer that calls a stdlib family from its OWN code declares it
in its own `[deps] stdlib`.** Through 3.3.16 bote's 32-leaf
sidecar happened to supply most of the stdlib, which let a consumer
lean on it. Now it does not, and the cyrius 6.6.18+ CLI prints a
`hint:` line naming the leaf to add. A consumer that builds the
bundle by hand, without `cyrius deps`, still mirrors bote's own
`[deps] stdlib` list in `cyrius.cyml`, including its
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

1. **Below cyrius 6.6.19**, declare every transitive stdlib module your
   graph reaches in your own `[deps] stdlib`: your snapshot's folds carry
   no requires blocks, so nothing else brings them. For the full bote
   bundle plus libro/majra that means adding, beyond the obvious ones,
   `ct`, `keccak`, `random`, `slice`, `thread`, `thread_local`, `sync`,
   `atomic`, `result`, `sigil` and `ws_server`. **From 6.6.19** the
   sidecars and the folds' own requires blocks carry the closure. A consumer
   that declares only `string fmt alloc vec str syscalls io freelist` builds
   with zero undefined names and runs: of `dist/bote.cyr` on 6.6.20 at
   3.3.17, and of both `dist/bote.cyr` and `dist/bote-core.cyr` on 6.7.9 at
   3.4.0 (the core one gets sakshi through libro → patra). Declare what your own code calls.
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
- **Below cyrius 6.7.6 a local `path =` beats `tag =`** and vendors your
  working tree, which can mask a wrong or unpushed tag. Your machine
  passes; a clean CI checkout resolving from `git + tag` gets different
  bytes and fails `cyrius deps --verify`. From 6.7.6 a default resolve
  builds the tag and only prints a hint; `CYRIUS_LOCAL=<dep>` opts into
  the checkout, in `build/local-deps/lib/`.

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

## What lives in the core 13?

| #  | File                  | Role                                          |
|----|-----------------------|-----------------------------------------------|
| 0  | `src/log.cyr`         | Operational logging through sakshi (3.4.0)    |
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
resources capabilities landed, 11 → 12 at **3.3.6** with
`content.cyr`, and 12 → 13 at **3.4.0** with `log.cyr`, listed first:
`codec.cyr` emits a DEBUG line per request through it, so a core
consumer that sets sakshi to DEBUG sees its dispatch traffic. bote
never configures sakshi from library code — the consumer's
`sakshi_set_level` / `sakshi_output_*` decide where, and whether, those
lines go. `src/sandbox.cyr` was weighed at 3.3.13 and kept
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
stdlib leaves. Its only external references are `_json_emit_escaped` (in
core through `jsonx.cyr`; since 3.4.6 an alias of the public
`bote_json_escape`), `_dsp_tool_error_result` (`dispatch.cyr`),
`str_builder_*` / `str_data`, `vec_*` and `alloc`. It is listed **last** in
`[lib.core]` because cyrius is single-pass and it calls into both.
`bote_json_escape(sb, cstr)` is the escaper a consumer's own JSON should use:
it appends a cstr as a JSON string body, escaping every control byte as
RFC 8259 §7 requires.

Stdlib footprint of the 13 modules: `string`, `fmt`, `alloc`,
`vec`, `str`, `tagged`, `assert`, `fnptr`, `hashmap`, `bayan`,
`chrono`, `freelist`, `sakshi`. `sakshi` is not on the sidecar: distlib's
verify finds it already in scope through `lib/patra.cyr`'s own include
(patra is libro's named dep), which is how a core consumer gets it too.
⚠ `log.cyr` sizes its stack line buffer with a `const`, so the bundles
need cyrius ≥ 6.7.2. No core module calls into `tls` / `sandhi` /
`sigil` / `ws_server` / `slice`.

⚠ **`dist/bote-core.deps` names `sigil` and `ct` anyway (3.3.17).**
They are libro's needs, not core's. A core consumer gets libro
transitively through bote's `deps.libro`, whatever profile it
selects. libro 2.10.7+ leaves its sigil needs to its thin
`sigil-mldsa` requires block, which bote never vendors because
`sigil` is a stdlib leaf here. So `distlib`'s verify records the two
leaves against the core bundle. Measured with a one-`ping` core
consumer: 2,079,184 B on 3.3.17, against 2,628,808 B on 3.3.16,
which also compiled the whole sigil fold (through libro's old
27-leaf sidecar). With the two leaves removed, libro's thin sigil
would serve instead at 1,209,096 B. The bundle ships as `distlib`
writes it. The defect is cyrius's and is filed there
(`2026-10-15-bote-distlib-kept-fold-over-thin-profile-records-named-deps-need.md`).

That "no core module calls sigil" is why `src/jwt.cyr` and `src/pkce.cyr` are in
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
