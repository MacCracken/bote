# Cyrius Language Feedback (from porting bote)

> **Tested against**: the cyrius binary installed at `~/.cyrius/bin/cc3` at the
> time of the bote port (2026-04-13). `cyrius version` reports `0.1.0`;
> `cyrius --help` reports `0.92.0` — same toolchain, inconsistent self-report.
> Cyrius main is at v4.3.2 per its roadmap, so **some or all of these may
> already be fixed upstream** — the reproductions below are the easiest way to
> verify against the live tree.

This doc collects language-level issues encountered while porting ~10K lines of
Rust to ~3K lines of Cyrius (12 modules + tests + benches + fuzz). Each item
has:

1. **Symptom** — what bote saw.
2. **Repro** — the smallest standalone `.cyr` that exhibits it.
3. **Expected** — what the docs say should happen.
4. **Workaround** — what bote does today.

Severity guide:
- **🔴 Correctness bug** — code silently does the wrong thing
- **🟡 Ergonomics gap** — code is harder to write/read than necessary
- **🟢 Diagnostics** — error messages could be friendlier

---

## 🔴 1. `\r` escape in string literals emits byte `r` (114), not CR (13)

### Symptom
HTTP responses contain literal `r` instead of carriage return:
```
HTTP/1.1 200 OKr     ← that's a literal 'r', not CR
Content-Type: ...r
```
Browsers and `curl` tolerate LF-only line endings, so headers parse, but
**request parsing fails** because incoming HTTP requests have real CRLF and
our search for `\r\n\r\n` is actually searching for `r\nr\n`.

### Repro
```cyr
include "lib/string.cyr"
include "lib/syscalls.cyr"
fn main() {
    var s = "\r\n\r\n";
    var i = 0;
    while (i < 4) {
        var c = load8(s + i);
        # Print c as decimal — should be 13, 10, 13, 10
        # Got: 114, 10, 114, 10
        i = i + 1;
    }
    return 0;
}
```

### Expected
Per `docs/cyrius-guide.md`: *"Escape sequences: `\n \r \t \0 \\ \"`"*. So `\r`
should be byte 13.

### Actual
`\r` becomes byte 114 (literal `r`). The other escapes (`\n`, `\t`, `\\`, `\"`,
`\0`) work correctly.

### Workaround in bote
`src/transport_http.cyr` builds CRLF manually:
```cyr
var _crlf = 0;        # global, init at startup
var _crlfcrlf = 0;
fn _http_init_crlf() {
    _crlf = alloc(3);
    store8(_crlf, 13); store8(_crlf + 1, 10); store8(_crlf + 2, 0);
    _crlfcrlf = alloc(5);
    store8(_crlfcrlf, 13); store8(_crlfcrlf + 1, 10);
    store8(_crlfcrlf + 2, 13); store8(_crlfcrlf + 3, 10);
    store8(_crlfcrlf + 4, 0);
    return 0;
}
```
Then every `"...\r\n..."` literal must be split:
```cyr
str_builder_add_cstr(sb, "Content-Length: ");
str_builder_add_int(sb, len);
str_builder_add_cstr(sb, _crlf);
```

### Severity
🔴 — silent miscompile of documented escape, breaks HTTP wire protocol.

---

## 🔴 2. `&&` and `||` do not short-circuit

### Symptom
Code like `if (p != 0 && vec_len(p) > 0) { ... }` segfaults when `p == 0`,
because `vec_len(0)` is still called and dereferences the null pointer.

This bug manifested as **silent termination with exit 0 (or 139)** in our test
suite — the test runner's `assert_summary` never printed because the crash
happened mid-suite, and stdout buffering swallowed any prior output before the
faulting instruction.

### Repro
```cyr
include "lib/string.cyr"
include "lib/syscalls.cyr"

fn fail_side(p) {
    syscall(1, 1, "FAIL_CALLED\n", 12);
    return load64(p);
}

fn main() {
    var p = 0;
    if (p != 0 && fail_side(p) == 0) {
        syscall(1, 1, "INSIDE\n", 7);
    }
    syscall(1, 1, "AFTER\n", 6);
    return 0;
}
var r = main();
syscall(60, r);
```

### Expected
Per `docs/cyrius-guide.md`: *"Logical (short-circuit, chainable): `&&` `||`"*.
With `p != 0` false, `fail_side(p)` should be skipped. Output should be just
`AFTER`.

### Actual
Output:
```
FAIL_CALLED
```
then segfault (`fail_side` dereferences 0). `&&` evaluates both operands
unconditionally; same for `||`.

### Workaround in bote
Always nest:
```cyr
if (p != 0) {
    if (vec_len(p) > 0) { ... }
}
```
Found and fixed across `src/dispatch.cyr`, `src/registry.cyr`, `src/jsonx.cyr`,
`src/schema.cyr`. Saved hours by eventually realizing the pattern; bisected
with `println("MK:foo")` markers.

### Severity
🔴 — silent miscompile of documented short-circuit semantics; very easy to
introduce when porting code from any language with proper short-circuit.

---

## 🔴 3. No per-block local variable scoping

### Symptom
A `var` declared anywhere inside a `fn` body conflicts with any other `var` of
the same name, even across distinct `if` / `while` / inner-block scopes:

```cyr
fn main() {
    if (cond1) { var i = 0; ... }
    if (cond2) { var i = 99; ... }   # → "duplicate variable"
    var i = 5;                        # → "duplicate variable"
}
```

### Repro
```cyr
include "lib/syscalls.cyr"
fn main() {
    var x = 1;
    if (1 == 1) { var x = 2; }   # error:NNNN: duplicate variable
    return 0;
}
var r = main(); syscall(60, r);
```

### Expected
Per general programming convention, locals declared inside a block scope only
live until the end of that block. Most languages allow shadowing of an outer
name within an inner scope.

### Actual
All locals share a single flat scope per `fn` body. Compile error
`duplicate variable`.

### Workaround in bote
`tests/bote.tcyr` is one big `fn main()` with hundreds of locals — every name
must be globally unique within the function. We ended up with `req_one`,
`rcompiled`, `prog_notif`, `ps_send`, `c_str`, `c_num`, `c_int`, `c_bool`,
`c_arr`, `c_obj`, `c_en`, `c_nb`, `c_ib`, `c_req`, `c_multi`, `c_any`, …

Beyond rename ergonomics, this also makes refactoring painful — moving a block
of test code into an `if (running_test == "x")` wrapper requires renaming any
locals that happened to share a name with the surrounding function.

### Severity
🔴 — significant ergonomics tax, especially in long test files. Suspect that
adding per-block scoping requires the basic-block analysis pass that's already
on the cyrius v4.2.0 roadmap.

---

## 🟡 4. Static `var buf[N]` size limit (~16KB)

### Symptom
`var _stdio_buf[131072];` fails at compile time:
```
code:67576 data: 1050776 strings: 1250
  tip: heap-allocate large var buf[N] arrays
```

### Expected
*Documented* — `docs/development/roadmap.md` lists this as gotcha #4: *"Large
`var buf[N]` exhausts output buffer — Use `alloc(N)` for >4KB"*. So this is
known, not a bug. Mentioning here because the threshold (~16KB? 64KB?) isn't
precisely documented and the failure mode is opaque (the `code:NNN` numbers
don't tell you "your var is too big").

### Workaround in bote
```cyr
var STDIO_BUF_SIZE = 131072;
var _stdio_buf = 0;          # global pointer

fn _stdio_init() {
    if (_stdio_buf == 0) { _stdio_buf = alloc(STDIO_BUF_SIZE); }
    return 0;
}
```
Then access via `_stdio_buf + offset` (no `&`). Same pattern in
`transport_http.cyr` for the request buffer.

### Severity
🟡 — documented; but the error message could name the offending `var`.

---

## 🟡 5. `is_err` (syscalls.cyr) vs `is_err_result` (tagged.cyr) — naming clash

### Symptom
`lib/syscalls.cyr::is_err(ret)` checks `ret < 0` (raw syscall return).
`lib/tagged.cyr::is_err_result(res)` checks the tag of a `Result` heap struct.

Vidya's `cmd_serve()` does:
```cyr
var sfd = tcp_socket();           # returns Ok(fd) heap pointer (always > 0)
if (is_err(sfd) == 1) { ... }     # ← always false! is_err on a heap ptr
sfd = payload(sfd);
```
This **silently never catches socket errors** — `is_err` from syscalls.cyr
checks `< 0`, but `tcp_socket()` returns a tagged `Result` (heap pointer,
always positive). Real errors get unwrapped via `payload(Err_ptr)` which
returns the error code as if it were a valid fd, then bind/listen fail later.

### Expected
Either name should resolve to the right semantics, or the docs / lint should
warn when `is_err` is applied to a Result type.

### Workaround in bote
Use `is_err_result` from `lib/tagged.cyr` everywhere we check Result returns.

### Severity
🟡 — easy to miss when copy-pasting from vidya patterns. The vidya bug (probably
inherited) is a good candidate for `cyrius lint` to catch.

---

## 🟢 6. Cascading parse errors from undefined symbols

### Symptom
Forgetting `include "lib/io.cyr"` while using `lib/json.cyr` (which references
`file_read_all`) produces:
```
error:1729: expected '=', got string
```
Line 1729 is in the *preprocessed* source, not the user's file. There's no
`error:src/foo.cyr:42:` form for this case.

The actual root cause was an undefined symbol several thousand preprocessed
lines earlier; the parser then tried to interpret an unrelated literal as a
syntax token.

### Expected
Per docs, v4.0.0+ provides `error:lib/foo.cyr:42:` style file:line. Apparently
the cascade-from-undefined path doesn't go through that mapping.

### Workaround in bote
- Bisect by removing `include` lines until the error disappears.
- Defensive include — every `src/*.cyr` `include`s its full transitive deps
  even though the entry point also does (cyrius handles duplicates fine).

### Severity
🟢 — bote always produces buildable code now, but new contributors will hit
this. Better diagnostic would save a lot of bisecting.

---

## 🟢 7. fmt_int writes to stdout regardless of caller intent

### Symptom
Diagnostic prints to stderr that include `fmt_int(value)` cause output
interleaving:
```
syscall(SYS_WRITE, 2, "[http] dprobe=", 14);  # → stderr
fmt_int(dprobe);                              # → stdout (different stream!)
syscall(SYS_WRITE, 2, "\n", 1);               # → stderr
```
Output is non-atomic across streams and can appear out of order in
test/diagnostic logs.

### Expected
Either a `fmt_int_fd(fd, n)` variant, or a `fmt_int_buf(n, buf) → len` (which
exists per `lib/fmt.cyr`, but isn't always obvious).

### Workaround in bote
Use `fmt_int_buf` + explicit `syscall(SYS_WRITE, fd, buf, len)`, or
`str_builder_add_int(sb, n)` when building a response.

### Severity
🟢 — existing API has the building blocks; might benefit from a sibling
`fmt_eprintf` / `efmt_int`.

---

## What worked surprisingly well

To balance the bug list:

- **`cyrius port` itself** — the move-to-rust-old + scaffold + cyrius.toml +
  test.sh setup was painless. Took 30 seconds and produced a usable starting
  point.
- **`cyrius build`** auto-resolving `cyrius.toml` deps + auto-including. No
  need to hand-curate include order at the command line.
- **`cyrius test` / `cyrius bench` / `cyrius fuzz`** — three discoverable
  conventions (`.tcyr`, `.bcyr`, `.fcyr`) that "just work". We added test/bench
  /fuzz files and they showed up immediately.
- **Function pointer ergonomics** (`&fn_name` + `fncall1(fp, arg)`) made
  handler registration straightforward. No closure capture meant we never
  fought with environments — handlers are pure functions of their arguments.
- **i64-only data model** — initially intimidating, ended up reducing decisions.
  Every value is a heap pointer or an i64; no boxing, no Option<Box<T>>, no
  generic monomorphization explosion.
- **Vendored stdlib (`lib/`)** — the `cyrius port` scaffold copies 47 stdlib
  modules into the project. Self-contained, no path issues.

---

## How to verify against current cyrius

For each repro above, save it to `/tmp/probe.cyr`, then:
```sh
cyrius build /tmp/probe.cyr /tmp/probe && /tmp/probe; echo "exit=$?"
```

If a repro now produces the expected behavior, please bump that section to
✅ and link the cyrius commit. PRs welcome.
