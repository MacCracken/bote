# Resolved cyrius-language issues

Historical index of upstream cyrius issues bote discovered and reported,
all now resolved. Each entry is a short summary; the full live-
investigation write-ups lived at `docs/bugs/<issue>.md` during active
triage and were cleared once the upstream fix landed + bote adopted
the new pin.

For the rolling cyrius-language feedback loop (language-level
friction, not specific bugs) see
[docs/cyrius-feedback.md](cyrius-feedback.md).

---

## 2026-04 — `cyrius-4.5.1-identifier-buffer-cap.md`

**Symptom:** Real-world projects (bote's 16-stdlib + 9-libro + 6-majra
+ 15-src-module compile unit) hit a hard 64 KB identifier-intern
table cap. When the buffer overflowed mid-parse inside an `include`
directive, the symbol-table for the `include` keyword itself got
corrupted, and the parser emitted a misleading `lib/assert.cyr:3:
expected '=', got string` instead of the real capacity error. 45
minutes of bisecting wasted on the decoy diagnostic before we
pinned the real cause.

**Reported:** bote 1.5.0 era (2026-04-14), with a 2000-fn standalone
reproducer that cleanly triggered the buffer-full diagnostic on the
release binary.

**Resolved:**
- **cyrius 4.6.2** — identifier buffer raised to 131072 bytes (~60 KB
  headroom past the old 64 KB cap); clean diagnostic on overflow.
- **cyrius 4.7.1** — `BUILD_METHOD_NAME` scratch-corruption fix (the
  real root cause of the misleading `assert.cyr:3` error). Scratch
  now starts at `GNPOS(S)` past live identifiers with `NPOS_GUARD(S,
  256)`, lookup-only. Directly addresses the specific overflow path
  bote hit.
- **cyrius 4.7.1** — function-table cap raised 2048 → 4096.

**Bote adoption:** pin bump 4.5.1 → 4.7.1 in bote 1.8.1.

---

## 2026-04 — `cyrius-4.8.3-regressions.md`

**Symptom:** Three simultaneous 4.8.3 regressions blocked bote's
build:
1. **Path-traversal rejection** on `[deps.X] path = "../X"` in
   `cyrius.toml` — broke the standard AGNOS sibling-repo layout
   (bote + libro + majra all as siblings under one parent dir).
2. **Stricter undefined-var detection** — 4.7.1/4.8.1 made undefined
   fns runtime warnings; 4.8.3 hard-errored undefined vars. Caught
   bote's `SEV_INFO` references in `src/audit_libro.cyr` when the
   test compile unit had been trimmed (2.4.0 cap squeeze).
3. **Include-once table capped at 64 files** — bote's main.cyr
   compile unit sits at ~52 top-level + ~8 transitive → right at
   the ceiling. Any consumer adding a dep tripped it.

**Reported:** bote 2.4.0 era (2026-04-14), during the 4.8.3 capacity-
meter adoption attempt. Bote stayed pinned to 4.8.1 with a note.

**Resolved in cyrius 4.8.4:**
- Path traversal allowed by default for config-declared `[deps.X]
  path = ...` entries; `CYRIUS_ALLOW_PARENT_INCLUDES=1` env override
  additionally available.
- Include-once cap raised 64 → 256.
- `PP_IFDEF_PASS` fixpoint loop lets nested includes past the first
  level expand cleanly.
- New `ERR_EXPECT` capacity diagnostic self-reports cap utilization
  at the fail point — future near-cap errors surface the real cause
  instead of the misleading cascade.

**Bote adoption:** pin bump 4.8.1 → 4.8.4 in bote 2.5.0 (landed with
claims propagation through transports).

---

## 2026-04 — `cyrius-4.8.4-ci-binary-skew.md`

**Symptom:** The 4.8.4 release binary installed on CI (from
`github.com/MacCracken/cyrius/releases/download/4.8.4/...`) compiled
bote's full libro + majra + sigil test unit differently than my
local `cc3 4.8.4-alpha2` build. Local passed 394 assertions; CI died
at `fn=908/4096` (well under cap) with the pre-4.7.1-era misleading
`lib/assert.cyr:3: expected '=', got string` error. Suggested an
alpha3+ change (PP_IFDEF_PASS fixpoint, `&local` safety scan, or
width-aware scan) that made it into the release build but regressed
behavior vs the alpha2 that worked.

**Reported:** bote 2.5.0 era (2026-04-14). Bote shipped 2.5.0 with a
lean `tests/bote.tcyr` workaround that dropped 8 shape-only
assertions to dodge the CI failure.

**Resolved:** cyrius lang-agent retagged 4.8.4 with the
alpha2-that-actually-works binary. Clean-install from the retagged
release, and the full libro+majra+sigil compile unit passes as
expected.

**Bote adoption:** bote 2.5.1 restored the 8 dropped assertions
(`bote.tcyr` 386 → 394; total 603 tests across 8 files, matching
pre-trim count).

---

## Pattern

Every one of these followed the same workflow:

1. bote hits a real-world upstream limitation during a feature ship
2. I write up the symptom + reproducer in a `docs/bugs/<issue>.md`
3. The cyrius agent picks it up on their own schedule
4. Upstream fix ships in a cyrius release
5. bote bumps its pin + adopts
6. bote 2.x patch release closes the loop

Total time from report → resolution → bote adoption for each:
same-day or next-day in practice. The pattern's held across four
cyrius minor versions (4.5 → 4.6 → 4.7 → 4.8).
