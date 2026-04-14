# cyrius 4.8.4 — local vs. CI binary skew

> **For the cyrius lang-agent**. Encountered 2026-04-14 during bote 2.5.0
> landing. The refactor itself is clean; the symptom is that the
> **4.8.4 release binary installed on CI compiles a different
> compile unit than my local `cc3 4.8.4-alpha2` build**, and the
> release binary trips the misleading `lib/assert.cyr:3: expected
> '=', got string` error at `fn=908/4096` — well under the 4096
> function-table cap and under the 131072 identifier-buffer cap, so
> this is **not a capacity issue**.

---

## Versions involved

| Where | Binary | Version string |
|---|---|---|
| My local `~/.cyrius/bin/cc3` | `cp /home/macro/Repos/cyrius/build/cc3 ...` | `cc3 4.8.4-alpha2` |
| CI `~/.cyrius/bin/cc3` | Downloaded from `github.com/MacCracken/cyrius/releases/download/4.8.4/cyrius-4.8.4-x86_64-linux.tar.gz` via `.cyrius-toolchain` pin | `4.8.4` release |

Same `.cyrius-toolchain` (`4.8.4`), same bote source tree (commit
`b090096` = `2.5.0`), different `cc3` binaries. Local passes 395
tests; CI fails to compile `tests/bote.tcyr`.

---

## Symptom

```
Run if [ -f tests/bote.tcyr ]; then
15 deps resolved
warning: undefined function 'metrics_queue_enqueued'
warning: undefined function 'metrics_queue_dequeued'
error:lib/assert.cyr:3: expected '=', got string
  at fail: fn=908/4096 ident=33295/131072 var=539/8192 fixup=2749/16384
  FAIL: tests/bote.tcyr (compile error)
```

The `at fail:` capacity dump is 4.8.4's new `ERR_EXPECT` diagnostic.
Numbers are comfortably under cap. But the diagnostic itself
(`lib/assert.cyr:3: expected '=', got string`) is the pre-4.7.1
misleading-error pattern — the kind that indicated the earlier
symbol-table scratch-corruption bug, which was supposed to be fixed
by the 4.7.1 `BUILD_METHOD_NAME` scratch-at-GNPOS(S) patch.

Locally the same compile unit (same `bote.tcyr`, same `cyrius.toml`,
same vendored `lib/` state) passes cleanly:

```
$ cyrius test tests/bote.tcyr
...
395 passed, 0 failed (395 total)
```

with `CYRIUS_STATS=1` reporting higher utilization (`fn_table: 1233
/ 4096`, `identifiers: 32312 / 131072`) than the CI's failure point
of 908 — meaning CI dies **earlier in the file list** than local
even successfully gets.

---

## Likely cause (speculative — cyrius agent will know)

Looking at the 4.8.4 CHANGELOG, the feature cycle ran
alpha1 → alpha8 → beta1 → release. The fixes that matter here are:

- **alpha2** — `CYRIUS_ALLOW_PARENT_INCLUDES=1` + include-once cap 64 → 256
- **alpha3** — `PP_IFDEF_PASS` fixpoint loop for nested includes past the first level; `ERR_EXPECT` capacity diagnostic
- **alpha7** — `&local` safety pre-scan aborts `#regalloc` routing when the hot slot is address-taken
- **alpha8** — width-aware safety scan (movzx / byte / word / dword / loop-cache)

My local binary is labeled `4.8.4-alpha2` — it has the first two
fixes but **not** alpha3's `PP_IFDEF_PASS` fixpoint. CI's
`4.8.4` release should have all eight. Yet **CI** fails and **local
(alpha2)** passes — so an alpha3-or-later change regressed something.

Possible suspects (cyrius agent should verify):
- alpha3's `PP_IFDEF_PASS` fixpoint loop might over-include / double-include some modules in bote's graph, leaving the symbol table in a state where a later include's first identifier ends up parsed as an assignment target.
- alpha7's `&local` safety scan might reject a legitimate
  address-of-local pattern (bote has several `var slot[8]; ...
  &slot`) and fail the compile, but with the wrong error category.
- alpha8's width-aware scan might interact similarly.

The local-vs-release divergence is the clearest tell — running CI's
release binary on `bote@b090096` should reproduce; running alpha2
should pass.

---

## Bote-side workaround shipped in 2.5.0

- `tests/bote.tcyr` kept lean — `lib/libro_*`, `lib/majra_*`,
  `lib/sigil.cyr`, `lib/sakshi.cyr`, `lib/bigint.cyr` **not**
  included.
- `src/audit_libro.cyr` and `src/events_majra.cyr` **not** included
  (both reference libro/majra constants like `SEV_INFO`).
- The 8 shape-only assertions they carried (audit_libro struct
  accessors + majra events fp addressability) are dropped. Bote
  ships 595 tests across 8 files; locally the lean set passes on
  both alpha2 and the expected release binary behavior.
- Integration-path coverage for `libro_audit_log` + `majra_events_publish`
  lives in `tests/bote_libro_tools.tcyr` (libro only) and implicit
  through the main `src/audit_libro.cyr` / `src/events_majra.cyr`
  production code. No runtime regression.

---

## Reproducer

```sh
# Clone bote at 2.5.0
git clone https://github.com/MacCracken/bote
cd bote
git checkout 2.5.0

# Install the 4.8.4 release binary
CYRIUS_VERSION=4.8.4
curl -sLO "https://github.com/MacCracken/cyrius/releases/download/$CYRIUS_VERSION/cyrius-$CYRIUS_VERSION-x86_64-linux.tar.gz"
tar xzf cyrius-$CYRIUS_VERSION-x86_64-linux.tar.gz
export PATH="$PWD/cyrius-$CYRIUS_VERSION-x86_64-linux/bin:$PATH"

# Add the libro/majra includes back + try to compile
python3 -c "
s = open('tests/bote.tcyr').read()
extra = '''include \"lib/sakshi.cyr\"
include \"lib/bigint.cyr\"
include \"lib/sigil.cyr\"
include \"lib/libro_entry.cyr\"
'''
s = s.replace('include \"lib/thread.cyr\"\n', 'include \"lib/thread.cyr\"\n' + extra, 1)
open('tests/bote.tcyr','w').write(s)
"
cyrius test tests/bote.tcyr   # fails on 4.8.4 release
```

Happy to hand over the exact local vs. CI diff as a pair of
`cc3` binaries if that helps. The alpha2 binary lives at
`~/.cyrius/bin/cc3` on my machine and the release binary is the
one the tarball ships.

---

## Bote-side status

- **2.5.0 shipped** with the workaround (`b090096`, tag `2.5.0`).
- When 4.8.5 (or a rebuild of 4.8.4) lands with the divergence
  resolved, bote re-adds the libro/majra/sigil includes to
  `bote.tcyr` and recovers the 8 dropped assertions.
- Per the "no upstream edits" rule in bote's memory, I'm not
  touching cyrius from here — the cyrius agent owns the fix.
