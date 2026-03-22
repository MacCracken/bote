# Contributing to bote

Thank you for your interest in contributing to bote. This document covers the
development workflow, code standards, and project conventions.

## Development Workflow

1. **Fork** the repository on GitHub.
2. **Create a branch** from `main` for your work.
3. **Make your changes**, ensuring all checks pass.
4. **Open a pull request** against `main`.

## Prerequisites

- Rust toolchain (MSRV: **1.89**)
- `cargo-deny` — supply chain checks
- `cargo-llvm-cov` — code coverage

## Makefile Targets

| Target           | Description                                             |
| ---------------- | ------------------------------------------------------- |
| `make check`     | Run fmt + clippy + test + audit (the full suite)        |
| `make fmt`       | Format check with `cargo fmt`                           |
| `make clippy`    | Lint (default, no-default, and all-features)            |
| `make test`      | Run the test suite                                      |
| `make test-all`  | Test every feature flag individually and combined        |
| `make deny`      | Audit dependencies with `cargo deny`                    |
| `make coverage`  | Generate HTML code coverage report                      |
| `make bench`     | Run benchmarks and append results to history log        |
| `make doc`       | Build documentation with strict warnings                |

Before opening a PR, run `make check` to verify everything passes.

## Adding a New Module

1. Create `src/module.rs` with your implementation.
2. Add the module to `src/lib.rs` (feature-gated if appropriate).
3. Add unit tests in the module file under `#[cfg(test)]`.
4. Add the feature flag to `Cargo.toml` if applicable.
5. Update `README.md` with the new feature table entry.

## Code Style

- Run `cargo fmt` before committing. All code must be formatted.
- `cargo clippy -D warnings` must pass with no warnings.
- All public items (functions, structs, enums, traits, type aliases) must have
  doc comments.
- Keep functions focused and testable.
- Use `#[non_exhaustive]` on public enums for forward compatibility.

## Testing

- Unit tests go in the module file under `#[cfg(test)]`.
- Transport tests must be deterministic (retry-connect, not sleep).
- All new features require tests before merge.
- Run `make test-all` to verify all feature combinations.

## License

bote is licensed under **AGPL-3.0-only**. All contributions must be compatible
with this license. By submitting a pull request, you agree that your
contribution is licensed under the same terms.
