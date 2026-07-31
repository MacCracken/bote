# `/dev/urandom` opened via raw `syscall(SYS_OPEN, ...)` — breaks aarch64 builds (`open(2)` doesn't exist on aarch64 Linux)

**Filed:** 2026-07-17 by daimon (consumer), bote 3.1.4
**Severity:** Low — portability-only. No runtime security impact; the read
itself is correct on x86_64. This blocks *building* bote (and any consumer that
vendors bote's `dist/bote.cyr`) for aarch64 Linux — an undefined symbol at
compile/link time, never a runtime fault.
**Affects:** `src/session.cyr` (`_gen_session_id`) and `src/pkce.cyr`
(`pkce_code_verifier`). Byte-identical across bote 3.1.0 / 3.1.1 / 3.1.4.
**Fix cost:** two one-line call-site edits; no new symbols, no ABI change.

## Summary

Both entropy-reading paths open `/dev/urandom` with the **raw x86_64 syscall
number** rather than the stdlib's arch-portable wrapper:

```cyrius
# src/session.cyr:72  (_gen_session_id)
var fd = syscall(SYS_OPEN, "/dev/urandom", 0, 0);

# src/pkce.cyr:39     (pkce_code_verifier)
var fd = syscall(SYS_OPEN, "/dev/urandom", 0, 0);
```

`SYS_OPEN` is defined only in `lib/syscalls_x86_64_linux.cyr` (`SYS_OPEN = 2`).
**aarch64 Linux has no `open(2)` syscall** — the generic syscall table provides
only `openat(2)`. `lib/syscalls_aarch64_linux.cyr` therefore never defines
`SYS_OPEN`, so an aarch64 build fails at the reference:

```
cyrius build --aarch64 src/main.cyr build/bote-aarch64
  -> undefined variable 'SYS_OPEN'
```

(In a consumer's link, `_gen_session_id` surfaces first because it's earlier in
the bundle; `pkce_code_verifier` is the same gap immediately behind it.)

The other syscalls in both functions — `SYS_READ` (0), `SYS_CLOSE`, `SYS_EXIT`
— are defined identically on both arches, so `SYS_OPEN` is the *only* blocker.

## Root cause

The stdlib already ships a portable `sys_open` wrapper that does exactly the
right per-arch thing — bote's two call sites just don't use it:

```cyrius
# lib/syscalls_x86_64_linux.cyr
fn sys_open(path, flags, mode): i64 {
    return syscall(SYS_OPEN, path, flags, mode);          # bare open(2)
}

# lib/syscalls_aarch64_linux.cyr
fn sys_open(path, flags, mode): i64 {
    return syscall(SYS_OPENAT, AT_FDCWD, path, flags, mode);   # openat(AT_FDCWD, ...)
}
```

`openat(AT_FDCWD, path, ...)` is semantically identical to `open(path, ...)`
for an absolute path like `/dev/urandom`, so behavior is unchanged on every
arch. bote currently calls `sys_open` nowhere (0 hits in `src/`); it reaches
past the wrapper to the raw constant.

## Fix

Replace the raw syscall with the portable wrapper at both sites:

```cyrius
# src/session.cyr:72
var fd = sys_open("/dev/urandom", 0, 0);

# src/pkce.cyr:39
var fd = sys_open("/dev/urandom", 0, 0);
```

`sys_open` is provided by whichever `lib/syscalls_<arch>_linux.cyr` the build
selects, so this is a straight substitution — no `#if aarch64` fork, no new
constant. Confirm `session.cyr` / `pkce.cyr` already pull the syscall module
that exports `sys_open` (they include it transitively today for `SYS_READ` /
`SYS_CLOSE`); if not, add the include.

### Alternative (if the wrapper is undesirable at these sites)

Call `openat` directly, gated per-arch — more verbose, not recommended when the
portable wrapper already exists:

```cyrius
# aarch64: AT_FDCWD = -100
var fd = syscall(SYS_OPENAT, -100, "/dev/urandom", 0, 0);
```

## Verification

After the fix, an aarch64 cross-build of a bote consumer should link cleanly:

```sh
cyrius build --aarch64 src/main.cyr build/bote-aarch64
file build/bote-aarch64   # -> ELF 64-bit ... ARM aarch64
```

A `dispatcher`/session round-trip smoke test on x86_64 must still pass
(behavior is unchanged there).

## Downstream impact

- **daimon 1.4.0+** vendors `dist/bote.cyr` and hosts bote's session/libro
  tooling. Its aarch64 cross-build (a best-effort CI/release lane) fails on
  this exact `undefined variable 'SYS_OPEN'`. daimon has shipped a CI-side
  mitigation (classify `SYS_OPEN` as a known upstream gap → warning, keep the
  x86_64 build authoritative) and tracks it at
  `daimon/docs/development/issues/2026-07-17-bote-aarch64-sys-open-urandom.md`.
  When this fix lands and daimon re-runs `cyrius deps`, its aarch64 lane goes
  green and the CI allowlist entry can retire.
- Any other consumer that vendors `dist/bote.cyr` and targets aarch64 hits the
  same wall.

## References

- aarch64 Linux syscall table: no `__NR_open`; `__NR_openat` (56) only. See
  `include/uapi/asm-generic/unistd.h` (the generic table aarch64 uses).
- `lib/syscalls_aarch64_linux.cyr` — `sys_open` → `openat(AT_FDCWD, ...)`; the
  portable wrapper this issue routes through.
