# cyrius 4.8.3 — three regressions blocking bote's build

> **For the cyrius lang-agent**. Found 2026-04-14 while landing bote's
> 2.5.0 claims-propagation refactor (the refactor the capacity meter
> was designed to unblock). The meter works beautifully — measured
> bote's `tests/bote.tcyr` at 651 / 4096 fns, 17507 / 131072
> identifiers (13% used). Refactor would easily fit. But three other
> 4.8.3 changes (none advertised in the 4.8.3 CHANGELOG as breaking
> changes) now block both the production build and the tests. Bote
> stays pinned at 4.8.1 in `cyrius.toml` until these are resolved.

---

## Regression 1 — `path traversal rejected` for `../sibling` dep paths

### Symptom
```
$ cyrius build src/main.cyr bote
error: path traversal rejected:../libro/src/error.cyr
error: path traversal rejected:../libro/src/hasher.cyr
...
error: path traversal rejected:../majra/src/pubsub.cyr
```

### Trigger
`cyrius.toml` declares sibling-repo deps with relative paths:

```toml
[deps.libro]
git = "https://github.com/MacCracken/libro"
path = "../libro"
tag = "1.0.3"
modules = ["src/error.cyr", "src/hasher.cyr", ...]

[deps.majra]
path = "../majra"
```

This has been the standard AGNOS cross-repo layout since bote 1.2.0
(2026-04 landings) and is documented as the recommended pattern for
local iteration across sibling projects (bote + libro + majra + kavach
typically all checked out as siblings under one parent dir).

### Expected
`path = "../X"` where X is a trusted neighboring repo should resolve
via the dep manifest just like `path = "/abs/X"`. Path-traversal
rejection makes sense for *string concatenation into arbitrary file
system access*, not for *declared dep manifest paths a human put in
a config file*.

### Suggested fix
- If the path comes from a `[deps.X] path = ...` declaration in
  `cyrius.toml`, allow `..` segments. Canonicalize to absolute path
  at config-parse time so everything downstream sees an absolute
  path anyway.
- Alternative: require absolute paths in `cyrius.toml` and document
  a migration (all AGNOS consumers would need a one-line config
  update per dep). Less friendly but explicit.

---

## Regression 2 — Stricter undefined-var detection catches legitimate lazy-include patterns

### Symptom
```
$ cyrius test tests/bote.tcyr
error:src/audit_libro.cyr:65: undefined variable 'SEV_INFO'
  (missing include or enum?)
FAIL: compile error
```

### Trigger
bote 2.4.0 `tests/bote.tcyr` does not `include "lib/libro_entry.cyr"`
(intentional trim to fit the pre-meter cap). `src/audit_libro.cyr`
references `SEV_INFO` — a libro enum — but that reference is only
reached at runtime if the caller invokes the libro adapter (which
the bote.tcyr tests never do; they only check fn-pointer
addressability via `&libro_audit_log`, no invocation).

On cyrius 4.7.1 and 4.8.1, undefined *functions* became runtime-crash
warnings (`note: undefined function 'X' (will crash at runtime)`),
compilation proceeded, and bote's test harness worked because the
dead paths were never executed. Undefined *variables* like `SEV_INFO`
now produce a **hard compile error** instead.

### Expected
Consistency with the fn-handling: an undefined variable referenced
from a code path that gets compiled but never executed should be a
warning, not a hard error. Alternatively: always hard-error, but
then 4.7.1's "undefined fn becomes runtime warning" behavior should
also flip to hard-error — pick one.

### Suggested fix
- Cheapest: downgrade undefined-var to a warning with the same
  "(will crash at runtime)" framing that undefined-fn uses. Consistent
  with the existing sibling behavior.
- Higher bar: introduce `--strict` or `CYRIUS_STRICT=1` that turns
  warnings into errors, and have the capacity meter's `--check` mode
  include strictness. That way CI can opt into strict; local iteration
  stays permissive.

### Bote workaround (forced)
Move the `SEV_INFO`-using adapter tests out of `bote.tcyr` and into
`bote_libro_tools.tcyr` (which does include `lib/libro_entry.cyr`).
Shape-only coverage for the `audit_sink_new(&libro_audit_log, ctx)`
fp-addressability check moves cleanly. Happy to do this bote-side
regardless of the upstream call.

---

## Regression 3 — `include-once table full (64 files)`

### Symptom
```
$ cyrius build src/main.cyr bote
...
error: include-once table full (64 files) — split compilation un[it]
```

### Trigger
bote's `src/main.cyr` includes:

```
16 stdlib modules (string, fmt, alloc, vec, str, syscalls, io, args,
                   hashmap, json, fnptr, chrono, tagged, net, base64,
                   freelist, thread, sigil, http_server, ws_server)
 9 libro modules via [deps.libro]
 6 majra modules via [deps.majra]
21 bote src modules (error, protocol, jsonx, registry, dispatch,
                     codec, schema, stream, session, discovery,
                     audit, audit_libro, events, events_majra, auth,
                     content, host, libro_tools, bridge,
                     transport_{stdio,http,unix,streamable,ws})
```

= ~52 unique files in the include-once table before nested stdlib
includes (sakshi, bigint, thread, etc.). That overhead pushes past 64.

### Expected
64 is tight for any real project. bote's 2.4.0 graph already sits
at ~52 top-level + ~8 transitive = right at the ceiling. Any
consumer adding a dep would trip it.

### Suggested fix
Raise to 256 (or remove — include-once dedup is a bitmap/hashset
operation, O(1) per lookup regardless of entry count on a modern
hashmap). Same headroom pattern as the fn-table raise in 4.7.1
(2048 → 4096). 256 files gives meaningful room for real projects
without being unbounded.

---

## Bote-side status

- **2.4.0 stays the last shipped release.** Pin at cyrius 4.8.1.
- **2.5.0 (claims propagation)** is held until 4.8.x resolves the
  above. The refactor itself is ready and the capacity meter on 4.8.3
  confirmed it fits (651/4096 fns used, so ~3445 fn headroom and
  ~113 KB identifier headroom — the refactor costs <10 fns and
  <1 KB identifiers).
- Local `~/.cyrius/bin/cc3` is on 4.8.3 for my own development so I
  can use the capacity meter; production CI should stay on 4.8.1
  until these are fixed.

Happy to adopt each fix bote-side as soon as 4.8.4 ships. Also happy
to test any proposed patch against bote's `cyrius.toml` before
release — bote is a realistic cross-repo-dep / 52-file exemplar.
