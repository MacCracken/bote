# Opt-in transport profile — `dist/bote-core.cyr` without sandhi / tls / ws_server

**Filed:** 2026-05-10 during `t-ron` 2.1.x modernization arc
**Severity:** Medium — blocks downstream consumers that only use
bote's Dispatcher / Registry / Audit surface (not the transports)
from adopting the `dist/bote.cyr` single-bundle pattern bote /
libro / patra / agnosys / majra all standardized on
**Affects:** `dist/bote.cyr` distribution shape;
`cyrius.cyml [lib] modules`; no source change to the transports
themselves

## Summary

`dist/bote.cyr` (4 857 lines / 175 KB at 2.7.1) is bote's
single-file consumer bundle, produced by `cyrius distlib` from
the 23 modules listed in `cyrius.cyml [lib] modules`. It is
authoritative for downstream consumers per `DEPS-PATTERN.md`
("one `modules = ["dist/bote.cyr"]` line per dep").

**The bundle today is monolithic** — it includes every transport
(`transport_stdio`, `transport_http`, `transport_unix`,
`transport_streamable`, `transport_ws`, `bridge`) plus the
session / discovery / auth surface. Consuming `dist/bote.cyr`
pulls them all in, even when the consumer only uses bote's
Dispatcher + ToolRegistry + Audit primitives.

The transport stack is what drives `dist/bote.cyr`'s transitive
stdlib footprint up: pulling `lib/bote.cyr` forces
`lib/sandhi.cyr` (11 729 lines — HTTP server), `lib/tls.cyr`
(694 lines), and `lib/ws_server.cyr` (374 lines) into the
consumer's expanded-source budget.

**t-ron 2.1.0** is the first consumer to feel this in production.
t-ron wraps bote's Dispatcher via `SecurityGate` and registers
four introspection tools (`tron_status` / `tron_risk` /
`tron_audit` / `tron_policy`) — it does **not** use any
transport. Yet the only available bote-dist contract forces the
full transport stack into t-ron's compile unit, where it then
collides with cyrius's 2 MB compile-source-size cap once paired
with `libro 2.6.2`'s dist bundle. See `t-ron` CHANGELOG 2.1.0:

> **Full dist-bundle dep adoption (bote)** — blocked on either a
> cyrius compile-source-size cap raise (the 2 MB ceiling forces
> per-module bote pull today) or a **bote opt-in profile that
> excludes the transport stack**.

This issue is the bote-side path: ship an alternate dist profile
**without** the transport stack, so consumers that only need the
dispatch surface can adopt the single-bundle pattern.

## Reproduction

Counted from t-ron's perspective (the trigger consumer):

```
$ cd ~/Repos/t-ron && cyrius deps && cyrius build src/main.cyr build/t-ron
# under the dist-bundle pattern (both libro and bote dist):
compile src/main.cyr -> build/t-ron [x86_64]
error: expanded source exceeds 2MB (2097606 bytes)
FAIL
```

The actual workaround t-ron applied (per-module bote pull):

```toml
# t-ron/cyrius.cyml
[deps.bote]
git = "https://github.com/MacCracken/bote"
tag = "2.7.1"
modules = [
    "src/error.cyr",
    "src/protocol.cyr",
    "src/jsonx.cyr",
    "src/codec.cyr",
    "src/registry.cyr",
    "src/events.cyr",
    "src/audit.cyr",
    "src/dispatch.cyr",
    "src/schema.cyr",
]
```

Nine cherry-picked modules. The 23-module bundle is **9 modules
of "core" + 14 modules of "transport / session / discovery /
auth"**. The split is clean: the four introspection tools t-ron
needs all live in the core nine; nothing in t-ron touches the
other fourteen.

## What the split looks like

`cyrius.cyml [lib] modules` already orders the bundle so the
core sits at the top. Counting from the current 2.7.1 manifest:

**Core (9 modules — what t-ron pulls today):**

| # | File | Role |
|---|---|---|
| 1 | `src/error.cyr` | `BoteError` tagged enum |
| 2 | `src/protocol.cyr` | `JsonRpcRequest` / `Response` / `Error` |
| 3 | `src/jsonx.cyr` | JSON helpers |
| 4 | `src/registry.cyr` | `ToolRegistry` |
| 5 | `src/events.cyr` | `EventSink` |
| 6 | `src/audit.cyr` | `AuditLogger` / `AuditSink` |
| 7 | `src/dispatch.cyr` | `Dispatcher` (2.0 handler ABI) |
| 8 | `src/codec.cyr` | Encoder / decoder |
| 9 | `src/schema.cyr` | Schema compile |

**Audit-sink integrations (2 modules):**

| # | File | Role |
|---|---|---|
| 10 | `src/audit_libro.cyr` | libro chain sink |
| 11 | `src/events_majra.cyr` | majra pubsub sink |

**Transports + auth + session + discovery + content + host
(12 modules):**

| # | File | Role |
|---|---|---|
| 12 | `src/stream.cyr` | Streaming dispatch primitives |
| 13 | `src/session.cyr` | MCP session store |
| 14 | `src/discovery.cyr` | Tool discovery |
| 15 | `src/auth.cyr` | Bearer + JWT + PKCE |
| 16 | `src/transport_stdio.cyr` | stdio transport |
| 17 | `src/transport_http.cyr` | HTTP transport |
| 18 | `src/transport_unix.cyr` | Unix-socket transport |
| 19 | `src/bridge.cyr` | HTTP↔stdio bridge |
| 20 | `src/transport_streamable.cyr` | Streamable HTTP transport |
| 21 | `src/transport_ws.cyr` | WebSocket transport |
| 22 | `src/content.cyr` | Typed content blocks + annotations |
| 23 | `src/host.cyr` | HostRegistry + SSRF guard |

The 12 transport-and-above modules are what drag in sandhi / tls
/ ws_server. The core 9 only depend on stdlib (`string`, `fmt`,
`alloc`, `vec`, `str`, `tagged`, `assert`, `fnptr`, `hashmap`,
`json`) — no transport-stack stdlib at all.

## Proposed shape

Ship **two** dist bundles from a single `cyrius distlib` run,
selectable by the consumer:

```toml
# cyrius.cyml — bote-side

[lib]
# default bundle — current shape, all 23 modules
modules = [
    "src/error.cyr",
    # ... 23 total
]

[lib.core]
# opt-in bundle — core 9 only, no transport / auth / session
output = "dist/bote-core.cyr"
modules = [
    "src/error.cyr",
    "src/protocol.cyr",
    "src/jsonx.cyr",
    "src/registry.cyr",
    "src/events.cyr",
    "src/audit.cyr",
    "src/dispatch.cyr",
    "src/codec.cyr",
    "src/schema.cyr",
]
```

The syntax here is illustrative — the exact mechanism
(`[lib.<profile>]`, repeated `[[lib]]` arrays, a separate
`cyrius distlib --profile core` invocation, …) is a `cyrius
distlib` design call. The shape the bote release needs is:

- **`dist/bote.cyr`** — current monolithic bundle, no change.
  Existing consumers continue to use it verbatim.
- **`dist/bote-core.cyr`** — new opt-in bundle, 9 modules,
  ~70 KB / ~2 000 lines. Transport-free; consumer supplies its
  own stdlib without `sandhi` / `tls` / `ws_server`.

The `DEPS-PATTERN.md` contract gets a "Profile selection" section:

```toml
# Default (recommended unless you have a reason)
[deps.bote]
git = "..."
tag = "2.x.x"
modules = ["dist/bote.cyr"]

# Core-only (when your consumer wraps bote's Dispatcher /
# Registry / Audit surface but supplies its own transport — e.g.
# t-ron's SecurityGate middleware)
[deps.bote]
git = "..."
tag = "2.x.x"
modules = ["dist/bote-core.cyr"]
```

## Cost analysis

- **Source change:** zero. The 23 modules stay in `src/`. Only
  `cyrius.cyml [lib]` grows a profile mechanism and the release
  flow emits a second `.cyr` file.
- **CI surface:** the dist-freshness gate in `ci.yml` runs once
  per profile (`cyrius distlib` produces both files; the gate
  diffs both against committed copies).
- **Release asset surface:** `bote-<tag>-core.cyr` joins
  `bote-<tag>.cyr` in the release asset list. SHA256SUMS picks
  it up automatically.
- **Maintenance:** zero ongoing. The profile split is mechanical
  (the 9-module core list is stable; transports get added below
  the line, never inside the core).
- **Risk:** profile drift — a future bote change that wires a
  core-module's symbol against a transport-module's helper would
  silently break the core-only consumer's build. **Mitigation:**
  CI builds a tiny `tests/bote_core_only_smoke.tcyr` that
  `include`s only `dist/bote-core.cyr` (after copying it into
  `lib/`) plus stdlib, and runs one `dispatcher_new()` +
  `registry_register()` + `dispatcher_dispatch()` round-trip.
  If the core bundle ever silently regresses, this smoke test
  catches it.

## Recommendation

### Option A: Ship `dist/bote-core.cyr` alongside `dist/bote.cyr` (preferred)

The 9-module split is mechanically clean and there is a real
consumer (t-ron) waiting on it. Lands as a 2.8.x or
2.7.x-patch — depending on whether bote treats new dist profiles
as feature or maintenance work.

### Option B: Wait on the cyrius compile-source-size cap raise

Companion cyrius proposal at
`~/Repos/cyrius/docs/development/proposals/2026-05-10-raise-compile-source-cap.md`
proposes raising the 2 MB cap to 4 MB. If that lands first, the
opt-in profile becomes a nice-to-have rather than a blocker.

The two paths are **not mutually exclusive** — landing both
gives consumers the choice. Some will prefer the monolithic
bundle (one less knob); some will prefer the trim profile
(smaller compile-unit footprint, faster CI, better cap headroom
for future composition).

### Option C: Decline; keep the monolithic bundle

t-ron and projected daimon / phylax continue to use per-module
pulls. Each documents its `DEPS-PATTERN.md` deviation. The
ecosystem-uniform single-bundle contract erodes by one consumer
per quarter as more downstreams hit the cap. Not recommended
but listed for completeness.

## Severity rationale

MEDIUM. Higher than a docs-only or convenience issue because:

- It directly forces a deviation from the
  `DEPS-PATTERN.md`-stated contract, which bote authored.
- It bottlenecks a real consumer (t-ron 2.1.x) and projected
  consumers (daimon, phylax) from adopting the same pattern.
- The split is mechanically simple — the cost is in policy /
  release-flow design, not in source.

Lower than HIGH because the workaround (per-module pull) ships
and works today; bote 2.7.1 consumers are not stuck.

## What t-ron is doing

t-ron 2.1.0 shipped with `[deps.bote]` cherry-picking nine
modules. The split is documented in t-ron CHANGELOG 2.1.0 and
in t-ron `cyrius.cyml` with an inline comment referencing this
issue:

> bote MCP core — per-module pull. […] Dist-bundle adoption is
> parked as a later 2.1.x candidate gated on either a cyrius
> compile-source-size cap raise (the 2 MB ceiling forces
> per-module bote pull today) or a bote opt-in profile that
> excludes the transport stack.

When `dist/bote-core.cyr` lands, t-ron will flip
`[deps.bote] modules` to `["dist/bote-core.cyr"]` in the next
release-worthy patch, closing the "Future / Blocked" row on the
t-ron roadmap.

## Companion proposal

cyrius cap raise: `~/Repos/cyrius/docs/development/proposals/2026-05-10-raise-compile-source-cap.md`
