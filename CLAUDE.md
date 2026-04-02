# Bote — Claude Code Instructions

## Project Identity

**Bote** (German: messenger) — MCP core service — JSON-RPC 2.0, tool registry, schema validation, dispatch

- **Type**: Flat library crate
- **License**: GPL-3.0-only
- **MSRV**: 1.89
- **Version**: SemVer 0.D.M pre-1.0

## Stack

| Crate | Role |
|-------|------|
| libro | Hash-linked audit chain (optional, `audit` feature) |
| majra | Pub/sub event publishing (optional, `events` feature) |
| kavach | Tool sandboxing (optional, `sandbox` feature) |

All AGNOS crates are patched to local paths in `[patch.crates-io]` when developing locally.

## Consumers

All consumer apps with MCP tools (phylax, t-ron, sutra, jalwa, rasa, mneme, etc.)

## Modules

| Module | Purpose |
|--------|---------|
| `protocol` | JSON-RPC 2.0 types (Request/Response/Error) |
| `registry` | Tool registration, discovery, versioning |
| `schema` | JSON Schema compilation and validation |
| `dispatch` | Tool call routing to handlers (RwLock interior mutability) |
| `stream` | Streaming primitives (progress, cancellation) |
| `error` | Error types with JSON-RPC code mapping |
| `audit` | Audit logging trait + libro integration |
| `events` | Event publishing trait + majra integration |
| `host` | MCP hosting layer (feature: `host`) |
| `bridge` | TypeScript bridge with CORS (feature: `bridge`) |
| `discovery` | Cross-node tool discovery (feature: `discovery`) |
| `sandbox` | Tool sandboxing via kavach (feature: `sandbox`) |
| `transport/codec` | JSON-RPC serialization/deserialization |
| `transport/stdio` | Standard I/O transport |
| `transport/http` | HTTP transport (feature: `http`) |
| `transport/ws` | WebSocket transport (feature: `ws`) |
| `transport/unix` | Unix domain socket transport (feature: `unix`) |

## Development Process

### P(-1): Scaffold Hardening (before any new features)

0. Read roadmap, CHANGELOG, and open issues — know what was intended before auditing what was built
1. Test + benchmark sweep of existing code
2. Cleanliness check: `cargo fmt --check`, `cargo clippy --all-features --all-targets -- -D warnings`, `cargo audit`, `cargo deny check`
3. Get baseline benchmarks (`./scripts/bench-log.sh`)
4. Initial refactor + audit (performance, memory, security, edge cases)
5. Cleanliness check — must be clean after audit
6. Additional tests/benchmarks from observations
7. Post-audit benchmarks — prove the wins
8. Repeat audit if heavy

### Development Loop (continuous)

1. Work phase — new features, roadmap items, bug fixes
2. Cleanliness check: `cargo fmt --check`, `cargo clippy --all-features --all-targets -- -D warnings`, `cargo audit`, `cargo deny check`
3. Test + benchmark additions for new code
4. Run benchmarks (`./scripts/bench-log.sh`)
5. Audit phase — review performance, memory, security, throughput, correctness
6. Cleanliness check — must be clean after audit
7. Deeper tests/benchmarks from audit observations
8. Run benchmarks again — prove the wins
9. If audit heavy → return to step 5
10. Documentation — update CHANGELOG, roadmap, docs
11. Return to step 1

### Task Sizing

- **Low/Medium effort**: Batch freely — multiple items per work loop cycle
- **Large effort**: Small bites only — break into sub-tasks, verify each before moving to the next. Never batch large items together
- **If unsure**: Treat it as large. Smaller bites are always safer than overcommitting

### Refactoring

- Refactor when the code tells you to — duplication, unclear boundaries, performance bottlenecks
- Never refactor speculatively. Wait for the third instance before extracting an abstraction
- Refactoring is part of the work loop, not a separate phase. If a review reveals structural issues, refactor before moving on
- Every refactor must pass the same cleanliness + benchmark gates as new code

### Key Principles

- **Never skip benchmarks.** Numbers don't lie. The history log is the proof.
- **Tests + benchmarks are the way.** Minimum 80%+ coverage target.
- **Own the stack.** If an AGNOS crate wraps an external lib, depend on the AGNOS crate.
- **No magic.** Every operation is measurable, auditable, traceable.
- **`#[non_exhaustive]`** on all public enums.
- **`#[must_use]`** on all pure functions.
- **`#[inline]`** on hot-path functions.
- **`write!` over `format!`** — avoid temporary allocations.
- **Cow over clone** — borrow when you can, allocate only when you must.
- **Vec arena over HashMap** — when indices are known, direct access beats hashing.
- **Feature-gate optional deps** — consumers pull only what they need.
- **tracing on all operations** — structured logging for audit trail.

## Testing

| Category | Count |
|----------|-------|
| Library unit tests | 274 |
| Conformance tests | 44 |
| Doc-tests | 1 |
| Criterion benchmarks | 13 |

```bash
cargo test --all-features                    # All tests
cargo test --all-features --test conformance # Conformance only
cargo bench --bench dispatch --features bridge  # Criterion benchmarks
./scripts/bench-log.sh                       # Benchmarks + history log
make test-all                                # Full feature matrix
make coverage                                # cargo llvm-cov --all-features --html
```

## Documentation Structure

```
Root files (required):
  README.md, CHANGELOG.md, CLAUDE.md, LICENSE

docs/ (required):
  architecture/overview.md — system design
  development/roadmap.md — open items only (completed work → CHANGELOG)

docs/ (when earned):
  adr/ — architecture decision records
  guides/ — usage patterns and examples
```

## CHANGELOG Format

Follow [Keep a Changelog](https://keepachangelog.com/). Sections: Added, Changed, Fixed, Removed, Security, Performance.

- Every PR gets a CHANGELOG entry
- Performance claims MUST include benchmark numbers
- Breaking changes get a **Breaking** section with migration guide

## DO NOT

- **Do not commit or push** — the user handles all git operations (commit, push, tag)
- **NEVER use `gh` CLI** — use `curl` to GitHub API only
- Do not add unnecessary dependencies — keep it lean
- Do not `unwrap()` or `panic!()` in library code
- Do not skip benchmarks before claiming performance improvements
- Do not commit `target/` or `Cargo.lock` (library crates only)
