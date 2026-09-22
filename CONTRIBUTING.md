# Contributing to bote

Thank you for your interest in contributing to bote. This document covers the
development workflow, code standards, and project conventions.

## Development Workflow

1. **Fork** the repository on GitHub.
2. **Create a branch** from `main` for your work.
3. **Make your changes**, ensuring all checks pass.
4. **Open a pull request** against `main`.

## Prerequisites

- **Cyrius toolchain** — the version is pinned in `cyrius.cyml` (`cyrius = "…"`); the `cyrius` wrapper re-execs that pinned toolchain, so install per the [cyrius README](https://github.com/MacCracken/cyrius) and let the pin pick. `cyrius lib sync --full && cyrius deps` provisions `lib/`.
- Optionally sibling-checkout the two git-pinned AGNOS deps (`libro`, `majra`) under `../` so `cyrius deps` resolves via `path =` instead of fetching tags. ⚠ A sibling vendors its **working tree**, so keep it on the tag before you release (see `DEPS-PATTERN.md`). Everything else — sigil, sakshi, patra, sandhi — arrives through the toolchain's stdlib fold; do not add `[deps.<name>]` blocks for them.

## Common Commands

| Command                                              | Description                                                         |
| ---------------------------------------------------- | ------------------------------------------------------------------- |
| `cyrius deps`                                        | Populate `./lib/` from the version-pinned stdlib + tagged libro/majra dist bundles. Run before any build/test/check. |
| `cyrius deps --verify`                               | Enforce `cyrius.lock` hash match on every resolved dep.             |
| `cyrius build src/main.cyr build/bote`               | Build the production binary.                                        |
| `cyrius test tests/bote.tcyr`                        | Run one test compile unit. Repeat per `tests/bote_<module>.tcyr`.   |
| `cyrius build tests/bote.bcyr build/bote_bench && ./build/bote_bench` | Build and run the benchmark harness.               |
| `cyrius distlib`                                     | Regenerate `dist/bote.cyr` for downstream consumers.                |
| `CYRIUS_STATS=1 cyrius build src/main.cyr build/bote` | Print the capacity meter (fn_table / identifiers / etc.).         |
| `CYRIUS_DCE=1 ...`                                   | Whole-program dead-code elimination on the emitted binary.          |
| `CYRIUS_NO_WARN_SHADOW_LIB=1 ...`                    | Silence the cwd-shadows-version-snapshot informational note (set by default in CI). |

Before opening a PR, run:

```sh
cyrius deps --verify       # dep hash match
for t in tests/*.tcyr; do cyrius test "$t"; done   # full test matrix
cyrius distlib             # regen dist bundle if you touched src/
git diff --exit-code dist/bote.cyr   # bundle must match the source
```

CI runs the same gates plus a capacity gate (fail if `fn_table` or `identifiers` cross 95%) and a manifest-completeness gate (`[lib]` modules ⊇ `main.cyr` `src/` includes). See `.github/workflows/ci.yml`.

## Adding a New Module

1. Create `src/module.cyr` with your implementation.
2. Add `include "src/module.cyr"` to `src/main.cyr` at the right position (Cyrius is single-pass: includes must appear before any forward references).
3. Add `"src/module.cyr"` to the `[lib] modules` list in `cyrius.cyml` — otherwise the manifest-completeness CI gate fails. If the module is opt-in (consumers wire it themselves; not in the default binary), leave it out of `[lib]` and document the opt-in path in the file header (mirrors `src/libro_tools.cyr`).
4. Add a per-module test file `tests/bote_<module>.tcyr`. Splitting per-module keeps each compile unit under the cyrius fn_table cap.
5. Run `cyrius distlib` and commit the updated `dist/bote.cyr`.

## Code Style

- **No `panic!` or `unwrap()` in library code.** Cyrius doesn't have those, but the analogue is an unguarded `sys_exit(...)` or an implicit out-of-bounds read — guard at the boundary and return 0 / -1 / an error tag.
- **No raw `syscall(`** anywhere in `src/`, `tests/` or `fuzz/` — call the stdlib wrapper (`sys_exit`, `sys_open`, `random_bytes`, …). CI fails on the form itself.
- **Constant-time comparisons** for any token / signature / secret material. The codebase has the pattern; mirror it.
- **No logging of bote's own.** Audit records go to the `AuditSink` (libro adapter) and events to the `EventSink` (majra adapter), both wired by the consumer; bote references no sakshi symbol and should not start to. Diagnostics that must reach a human go through `eprint` on a fail-closed path, as `session.cyr` does.
- **`#[non_exhaustive]`-equivalent**: cyrius enums always allow tail-extension; rely on default-case handling in any `if (tag == BOTE_ERR_X)` chain. (Enum constants are global — prefix them `BOTE_`; see the `BOTE_ERR_*` namespacing note in CLAUDE.md.)
- **Keep functions focused and testable** — `tests/bote_<module>.tcyr` is the contract.
- **Stage nested calls into a `var` before asserting on them.** A 5.10.x-era parser limitation (`expected ')', got string` on a 2-arg call nested inside `assert(streq(...))` with JSON literals) is the origin; the staged form is also what the test files read like, so keep to it.

## Testing

- Unit tests go in `tests/bote_<module>.tcyr` and run via `cyrius test tests/bote_<module>.tcyr`.
- The default test runner is the harness in `lib/assert.cyr` (`assert`, `assert_eq`, `assert_summary`).
- Transport tests must be deterministic — retry-connect, not sleep.
- New features require tests before merge.
- Run the full 8-file matrix locally before pushing; CI runs the same matrix plus benchmarks and the dist-freshness gate.

## Documentation

- Every release gets a CHANGELOG entry under the `[Unreleased]` section (adopted in 2.7.0). Entries follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) shape.
- Performance claims must include benchmark numbers (`tests/bote.bcyr`).
- Roadmap state changes go in `docs/development/roadmap.md`.

## License

bote is licensed under **GPL-3.0-only**. All contributions must be compatible with this license. By submitting a pull request, you agree that your contribution is licensed under the same terms.
