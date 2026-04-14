# cyrius 4.5.1 — identifier buffer cap hit at ~64 KB

> **Status on cyrius 4.7.1** (2026-04-14):
> - ✅ **Function table cap raised 2048 → 4096** in 4.7.1.
> - ✅ **`BUILD_METHOD_NAME` scratch corruption fix** in 4.7.1 (per cyrius CHANGELOG: scratch now starts at `GNPOS(S)` past live identifiers, with `NPOS_GUARD(S, 256)`, lookup-only) — directly addresses the misleading `lib/assert.cyr:3: expected '=', got string` cascade we reported.
> - ✅ Original `docs/bugs/cyrius-4.5.1-repro.cyr` now trips the function-table cap with a clean diagnostic.
> - 🟡 **Bote's specific case still trips the misleading error on 4.7.1.** Re-tested after pinning to 4.7.1 with a freshly-bootstrapped `cc3 4.7.1` — including `lib/ws_server.cyr` in `tests/bote.tcyr` still reports `error:lib/assert.cyr:3: expected '=', got string`. Symbol-count math says we're well under the new 4096 cap (bote src 404 + stdlib 728 + libro 131 + majra 76 = ~1339), so something else (identifier-bytes? token table? a different scratch corruption path the 4.7.1 fix didn't cover) is the trigger.
> - **Bote workaround**: per-module test files (`tests/bote_<module>.tcyr`) — the permanent layout. `lib/ws_server.cyr` stays out of the shared `bote.tcyr` compile unit.

> **Status on cyrius 4.6.2** (2026-04-14, historical):
> - Identifier buffer raised; original repro tripped the new function-table cap. Bote-specific diagnostic miss persisted.
>
> **Original 4.5.1 context (preserved below)**: found while landing bote 1.5.0 (WebSocket transport, which pulls in `lib/ws_server.cyr`). The bug was a hard compile-time ceiling on the identifier/symbol table; real mid-size projects hit it without anything unusual. A companion minimal reproduction lives at `docs/bugs/cyrius-4.5.1-repro.cyr` in this repo.

---

## TL;DR

The cyrius compiler has a **64 KB identifier buffer** (hard cap, 65536
bytes). When a compilation unit's combined identifier table exceeds that
ceiling, cyrius emits one of two diagnostics:

1. **Clean** (when overflow lands at a top-level definition):
   ```
   error: identifier buffer full (65514/65536 bytes) — reduce included
   modules or split into separate unit
   ```

2. **Misleading** (when overflow lands mid-way through parsing an
   included file, specifically inside an `include` directive):
   ```
   error:lib/assert.cyr:3: expected '=', got string
   ```
   Line 3 of `assert.cyr` is itself `include "lib/string.cyr"` — the
   parser has lost the association between the token `include` and its
   keyword role, so it tries to parse the line as an assignment and
   stumbles on the string literal. That diagnostic sent us looking in
   the wrong place for ~45 minutes before a line-count bisect (see
   below) pinned the real cause.

Bote hits the cap in `tests/bote.tcyr` because the test unit imports
every stdlib module the transports need (16) plus 7 libro modules plus
6 majra modules plus 15 bote modules — and every function name, global
name, and local identifier in that whole forest contributes to the
shared table.

---

## Reproduction

### Clean diagnostic

```sh
cyrius test docs/bugs/cyrius-4.5.1-repro.cyr
```

The repro file just `include`s three small stdlib modules and defines
2000 functions named `some_long_function_name_number_1` …
`some_long_function_name_number_2000`. The clean diagnostic fires at
~1700 fns with that name length — so the ceiling is effectively
**identifier bytes stored, not identifier count**.

### Misleading diagnostic (the one bote actually saw)

Harder to produce in an isolated probe because it depends on overflow
happening while the parser is mid-way through an `include` token in
**another** included file. In bote, the trigger was:

- 16 stdlib modules, incl. `lib/http_server.cyr` + `lib/ws_server.cyr`
- 7 libro modules (via `[deps.libro]`)
- 6 majra modules (via `[deps.majra]`)
- 15 bote source modules
- A 1162-line test file with ~380 assertions (lots of local identifiers)

At 1162 lines we got the misleading error. At 1156 lines we passed. In
neither case did the cyrius diagnostic itself point at the identifier
buffer — it pointed at `lib/assert.cyr:3`.

---

## Bisect log (from the bote session)

| Change | Result |
|---|---|
| HEAD test file (1162 lines), cyrius 4.5.0 pin, `ws_server` NOT in stdlib list | **pass** — 382/382 |
| HEAD test file, cyrius 4.5.1 pin, `ws_server` added to `[deps] stdlib` | `lib/assert.cyr:3: expected '=', got string` |
| Same, but `ws_server` removed from stdlib list (still `include`d in `src/main.cyr` only) | pass |
| Same, but also `include "lib/ws_server.cyr"` in the test file | `lib/assert.cyr:3: expected '=', got string` |
| Same, test file trimmed to 1156 lines | pass |
| Same, test file at 1157 lines | fail |

The crossover line was insensitive to **what** the extra line contained
(a no-op comment was enough to flip it) — confirming it's a byte-count
issue in the identifier buffer, not a syntactic one. The extra parse
input of `ws_server.cyr`'s ~430 LOC was enough to eat most of the
buffer; the last ~6 lines of the test file were the straw.

---

## Suggested fixes

1. **Raise the cap.** 64 KB is tight for a language with aggressive
   stdlib inclusion and no per-module symbol scoping. A jump to 256 KB
   or 1 MB would absorb bote-scale projects comfortably. If the buffer
   is a fixed-size static array in the compiler, moving to a heap-
   allocated growable table would lift the ceiling to RAM bounds.

2. **Fix the misleading diagnostic.** When the parser detects an
   identifier-buffer overflow mid-parse, emit the buffer-full message
   from wherever it's actually detected — not the downstream
   `expected '=', got string` that follows once the symbol table is
   corrupted. A guard right before the overflow-producing intern would
   give users an accurate pointer to the real problem.

3. **Show the running total in the file header.** A `--stats` flag that
   prints "identifier bytes: X/65536" after compilation (even on
   success) would let projects see how close they are to the ceiling.
   Bote was at ~95 % before this release landed — the jump from 4.5.0
   → 4.5.1 (adding `ws_server.cyr` to the stdlib list) pushed it over.

4. **Per-module identifier namespacing (long-term).** If names from
   `lib/foo.cyr` don't need to be interned into the same global table
   as names from `lib/bar.cyr`, only the exported surface of each
   module would contribute to the shared table. That would roughly
   halve bote's footprint today and remove the class-of-bug entirely.

---

## Bote-side workaround (already shipped in 1.5.0)

`tests/bote.tcyr` keeps these lines at the top of the transport
includes block:

```cyr
include "src/transport_streamable.cyr"
# NB: lib/ws_server.cyr intentionally NOT included here — its frame I/O
# isn't exercised in these shape-only tests (live round-trip is covered
# by the stdlib ws_server conformance tests). Keeping it out also avoids
# pushing the cyrius 4.5.1 parser past its input-buffer cap.
include "src/transport_ws.cyr"
```

The test file references `ws_server_*` symbols only indirectly (via the
ws transport's handler fn). Those show up as `warning: undefined
function` at compile time — consistent with how unbound symbols were
handled for `metrics_queue_enqueued` / `metrics_queue_dequeued` (which
live in a majra module that bote doesn't link). The test suite passes
at 392/392 with those warnings present.

The full integration happens at `cyrius build src/main.cyr` time —
main.cyr *does* include `lib/ws_server.cyr` and produces a working
`./bote ws` binary with all the frame I/O wired up. Local verification:
`./bote ws 18393` binds `127.0.0.1:18393` and returns 101 Switching
Protocols on a valid WebSocket upgrade request.

---

## Verification after a cyrius fix

Once the buffer is raised (or removed), this repo has two simple checks:

```sh
# 1. The repro should either pass or produce a clearer error.
cyrius test docs/bugs/cyrius-4.5.1-repro.cyr

# 2. Bote's test suite with ws_server.cyr re-included should pass.
#    Revert the NB comment in tests/bote.tcyr and uncomment:
#      include "lib/ws_server.cyr"
#    above the src/transport_ws.cyr include.
cyrius test tests/bote.tcyr    # should still report 392 passed
```

If check 2 passes, bote's next release can drop the NB workaround.
