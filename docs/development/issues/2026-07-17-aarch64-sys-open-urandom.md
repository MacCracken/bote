# `/dev/urandom` opened via raw `syscall(SYS_OPEN, ...)` — breaks aarch64 builds (`open(2)` doesn't exist on aarch64 Linux)

> ## ✅ RESOLVED in 3.2.0 — 2026-07-30
>
> Fixed, but **wider than filed**: the report identified one of three build blockers,
> and the recommended fix was not the best one available. See the
> "Resolution" section at the bottom for what actually landed, what the report
> got wrong, and what is deliberately left open.

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

---

## ✅ Resolution — bote 3.2.0, 2026-07-30

**Status: FIXED and verified end to end.** All three binaries cross-build to real
`EM_AARCH64` ELF, and the **entire test suite — 13 files, 786 assertions — passes
under `cyrius test --aarch64`**, not merely links. The verification the issue asked
for (an ELF check) was the floor; this is the ceiling.

### The report was right about the defect and wrong about its extent

Two corrections, both material:

**1. `SYS_OPEN` was one of three blockers, not the only one.** The issue states
"`SYS_OPEN` is the *only* blocker". It is not. Set-differencing the two syscall
peers — 95 `SYS_*` constants in `lib/syscalls_x86_64_linux.cyr` against 88 in
`lib/syscalls_aarch64_linux.cyr` — gives **17 constants that exist on x86_64 only**:

```
SYS_ACCESS SYS_CHMOD SYS_DUP2 SYS_EPOLL_WAIT SYS_FORK SYS_LINK SYS_LSTAT
SYS_MKDIR SYS_OPEN SYS_PAUSE SYS_PIPE SYS_READLINK SYS_RENAME SYS_RMDIR
SYS_STAT SYS_SYMLINK SYS_UNLINK
```

bote referenced **three** of them, and `src/transport_unix.cyr` held the two the
report missed:

| Site | Constant | Portable wrapper used |
|---|---|---|
| `src/session.cyr` `_gen_session_id` | `SYS_OPEN` | `random_bytes()` — see below |
| `src/pkce.cyr` `pkce_code_verifier` | `SYS_OPEN` | `random_bytes()` |
| `src/transport_unix.cyr:99` | `SYS_UNLINK` | `sys_unlink(path)` → `unlinkat(AT_FDCWD, path, 0)` |
| `src/transport_unix.cyr:109` | `SYS_CHMOD` | `sys_chmod(path, mode)` → `fchmodat(AT_FDCWD, path, mode, 0)` |

An `undefined variable` is fatal, not a warning, so fixing only `SYS_OPEN` would
have moved the failure down to `transport_unix.cyr:99` rather than to green. Both
constants are in the `#ifndef CYRIUS_TARGET_AGNOS` arm, which does compile on
aarch64-Linux, and both reached the shipped `dist/bote.cyr`.

**2. "`pkce_code_verifier` is the same gap immediately behind it" — no.**
`src/pkce.cyr` was in **no** bundle at 3.1.4 (`grep -c pkce dist/bote.cyr
dist/bote-core.cyr` → 0 and 0) and in no `main*.cyr` include list. It was reachable
only from `tests/bote_pkce.tcyr`. So the *consumer-facing* exposure was
`src/session.cyr` alone. Fixing pkce still mattered — its own test fails an
`--aarch64` run — and it matters more now, because bote 3.2.0 folds `src/pkce.cyr`
into `dist/bote.cyr` (see the JWT/PKCE issue). Had the fold landed without this fix,
it would have shipped a *second* aarch64-undefined site to every consumer.

### What landed instead of `sys_open`

The issue proposed `sys_open("/dev/urandom", 0, 0)`. That is arity-correct — the
wrapper has an identical 3-arg `i64` signature on both peers — but two stdlib
comments argue against it:

- `lib/random.cyr:3-6` says verbatim: *"Use this rather than `/dev/urandom` +
  open/read/close — getrandom is the one-syscall path"*. `random_bytes(buf, len)`
  wraps `sys_getrandom`, which **is** defined on both arches (x86 318, aarch64 278),
  blocks until the kernel CSPRNG is seeded, loops internally on short reads, and
  works where `/dev/urandom` is not reachable — early boot, a chroot, a landlocked
  process.
- `lib/io.cyr:68-74` warns that raw `sys_open` is **ABI-wrong on agnos**, whose
  signature is `(name, namelen, ao_flags)` rather than Linux's
  `(path, O_flags, mode)` — a silent miscompile, not a build error.

So both entropy sites now call `random_bytes()`. This deletes the open / read-loop /
close trio at each site and is portable to aarch64, macOS and agnos in one move. The
fail-closed contract from the 1.9.4 audit (M2: *refuse to mint a session ID rather
than ship a guessable one*) is preserved exactly — only the source of the bytes
changed.

The two hardcoded numeric syscalls in `_gen_session_id`
(`syscall(1, 2, "fatal: …", 47)`, an x86 write number) went with them, replaced by
`eprint(msg, len)` and `sys_exit(90)`. They happened to work on aarch64 only because
the cyrius backend's `ESYSXLAT` table renumbers `x8=1 → 64`; correctness should not
rest on a compiler-internal table.

### Regression gate

`.github/workflows/ci.yml` gains an **aarch64 portability gate** — bote had no
aarch64 lane at all, which is why this regressed silently in the first place:

1. A **denylist grep** for all 17 x86-only constants across `src/*.cyr` and
   `dist/*.cyr`, comment-stripped first (the fix commit deliberately names those
   constants in explanatory comments). Verified to fire on the pre-fix
   `src/session.cyr` and to pass on the fixed tree.
2. A **blocking cross-build** of all three entries with an `e_machine == 0xB7`
   (`EM_AARCH64`) assertion on each output — the backstop for a future upstream
   symbol the denylist can't know about.

The full aarch64 *test* sweep is not in CI (it needs `qemu-user`); it was run
locally at 3.2.0 and `cyrius test --aarch64 <file>` reproduces it wherever
`qemu-aarch64` is on `PATH`.

### Deliberately left open

`src/transport_unix.cyr:113` calls `syscall(SYS_ACCEPT, sfd, 0, 0)`, and
`SYS_ACCEPT` comes from `lib/net.cyr:12` — a bare, unguarded `var SYS_ACCEPT = 43`,
which is the **x86** number. It compiles on aarch64 and is runtime-correct only
because the backend renumbers `43 → 202`, the same compiler-internal dependence the
numeric syscalls above were removed for. It is **not** a build blocker, so it is out
of scope for this fix; `sock_accept(sfd)` (`lib/net.cyr:331`, Result-returning) is
the consistent replacement, and `src/transport_unix.cyr` already uses `sock_send` /
`sock_recv` ten lines away. Worth its own bite, and arguably an upstream `net.cyr`
fix rather than a bote one.

### Downstream

**daimon 1.4.0+** — re-run `cyrius deps` at bote 3.2.0 and the aarch64 lane should
go green; the CI allowlist entry classifying `SYS_OPEN` as a known upstream gap can
retire. Note the fix is broader than daimon's report, so if daimon's allowlist also
suppressed `SYS_UNLINK` / `SYS_CHMOD`, those entries can go too. Consumers must add
`random` to their `[deps] stdlib` — see the new "Stdlib the consumer must supply"
section in `DEPS-PATTERN.md`.
