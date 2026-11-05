# BOTE IS A CYRIUS DEP — READ THIS BEFORE TOUCHING BUILD / RELEASE

**This file is non-negotiable. Do not invent an alternative
distribution mechanism. Do not ignore it because "it seems to
work without it". libro / patra / majra are the references.
Copy them.**

---

## Who consumes bote

bote is an **upstream Cyrius library** — the MCP core service.
Downstream Cyrius projects pull bote into their `cyrius.cyml`
as a git-tagged dep. Known-intended consumers per bote's own
`CLAUDE.md` include **phylax, t-ron, sutra, jalwa, rasa, mneme,
daimon**. Any of them (or any future project) wires bote in
like this:

```toml
[deps.bote]
git = "https://github.com/MacCracken/bote.git"
tag = "<bote version>"
modules = ["dist/bote.cyr"]      # full bundle (default)
```

`cyrius deps` clones bote at the tag and copies
`dist/bote.cyr` into the consumer's `lib/`. That's the entry
point they `include` from.

## The contract

bote ships **two** distribution artifacts:

| Artifact              | Profile  | Modules | Use when                                     |
|-----------------------|----------|---------|----------------------------------------------|
| `dist/bote.cyr`       | default  | 40      | Consumer needs bote's full transport surface |
| `dist/bote-core.cyr`  | `core`   | 14      | Consumer wraps Dispatcher / Registry / Prompts / Resources / Content / Audit but supplies its own transport |

- Every tagged release must commit **both** artifacts.
- Each bundle is a single `.cyr` file. Every public function /
  struct / global the profile exports lives in that one file.
  Since cyrius 6.6.18 (bote 3.3.17) it opens with a
  compile-verified `# Requires` block, one
  `include "lib/<leaf>.cyr"` per leaf of its `.deps` sidecar, so
  `include "dist/bote.cyr"` alone compiles. It carries no other
  `include`. For a `cyrius deps` consumer the block changes
  nothing: include-once is keyed on the literal path, and these are
  the same lines it auto-prepends.
- Each bundle ships with its sidecar (`dist/bote.deps`,
  `dist/bote-core.deps`). CI gates both files of each pair
  byte-for-byte against `cyrius distlib`.
- The file paths and names are fixed: `dist/bote.cyr` and
  `dist/bote-core.cyr`. Not `dist/bote-2.7.2.cyr`. Not
  `build/bote.cyr`. Not `bote.cyr` at the repo root.

If either bundle is missing at the tag, downstream consumers
that selected that profile break at `cyrius deps` time.

### Stdlib the consumer must supply

Since cyrius 6.6.18 a sidecar names exactly the stdlib leaves
`distlib`'s compile-verify proves the bundle needs, and nothing
from the producer's own `[deps] stdlib`. Everything those leaves
need arrives through their own requires blocks. At 3.4.11:

| Sidecar | Leaves |
|---|---|
| `dist/bote.deps` | `hashmap bayan ws_server syscalls` (4; was 32 through 3.3.16) |
| `dist/bote-core.deps` | `hashmap` (1; was 2 through 3.4.12) |

`cyrius deps` reads the sidecar and vendors those leaves. **A
consumer that calls a stdlib family from its OWN code declares it
in its own `[deps] stdlib`.** Through 3.3.16 bote's 32-leaf
sidecar happened to supply most of the stdlib, which let a consumer
lean on it. Now it does not, and the cyrius 6.6.18+ CLI prints a
`hint:` line naming the leaf to add. A consumer that builds the
bundle by hand, without `cyrius deps`, still mirrors bote's own
`[deps] stdlib` list in `cyrius.cyml`, including its
**ordering**, which is load-bearing under single-pass
compilation.

**New at 3.2.0: `random`.** `src/session.cyr`'s
`_gen_session_id` now draws entropy through `random_bytes()`
(getrandom(2)) instead of opening `/dev/urandom` by raw
syscall. A consumer whose stdlib list omits `random` gets
`undefined function 'random_bytes'`. Add it before the modules
that reference it — bote lists it alongside `ct` / `keccak`,
ahead of `sigil`.

## Resolving the bundle: `cyrius lib sync` BEFORE `cyrius deps`

⚠ **Read this before filing a resolver bug.** The most common
first-time failure consuming bote (or the bote → libro → majra graph)
looks like a broken resolver and is not one:

```
dep libro requires 'ct' ... is not in the cyrius stdlib
```

`ct` **is** in the cyrius stdlib. The message is misleading (filed
upstream as `2026-08-12-agnosai-deps-misleading-stdlib-error`). What it
actually means is that `./lib/` does not yet contain that module.

**Minimum cyrius: 6.7.13**, stated for both bundles. Each bundle states it on
the `# Minimum cyrius:` line that opens `src/log.cyr`, its first module, so a
repo that copies a bundle reads it in the file it copied. The number is
measured, not inherited: `scripts/bundle-min-cyrius.sh` builds a checkout on
every installed toolchain. 6.7.13 and later build both bundles, pass the
core-only smoke test and answer a ping. 6.7.2 through 6.7.12 still build
`dist/bote-core.cyr`, which has no transport, but stop on `dist/bote.cyr` with
`undefined variable 'SANDHI_SERVER_FAIL_CONFIG'`, because its transports need
sandhi 1.10.10. 6.7.1 and earlier stop at `log.cyr`'s first `const`. CI checks
that both bundles state the same minimum and that it is not newer than the pin.
A repo on 6.7.2 through 6.7.12 can take the full bundle at 3.4.11 at the
newest, and a repo on 6.6.x bote 3.3.17.

Cyrius deliberately does **not** auto-resolve stdlib — that is a
supply-chain choice, not an oversight. Two things follow, and the
**order matters**:

1. **Below cyrius 6.6.19**, declare every transitive stdlib module your
   graph reaches in your own `[deps] stdlib`: your snapshot's folds carry
   no requires blocks, so nothing else brings them. For the full bote
   bundle plus libro/majra that means adding, beyond the obvious ones,
   `ct`, `keccak`, `random`, `slice`, `thread`, `thread_local`, `sync`,
   `atomic`, `result`, `sigil` and `ws_server`. **From 6.6.19** the
   sidecars and the folds' own requires blocks carry the closure. A consumer
   that declares only `string fmt alloc vec str syscalls io freelist` builds
   with zero undefined names and runs: of `dist/bote.cyr` on 6.6.20 at
   3.3.17, and of both `dist/bote.cyr` and `dist/bote-core.cyr` on 6.7.9 at
   3.4.0 and on 6.7.18 at 3.4.11 (the core one gets sakshi through libro →
   patra). Declare what your own code calls.
2. Run **`cyrius lib sync --full`** to provision `./lib/` from the pinned
   toolchain snapshot, and only *then* `cyrius deps`, which **overlays**
   your declared `[deps.<name>]` git deps on top of that snapshot.

```sh
cyrius lib sync --full   # provision ./lib/ from the pinned snapshot
cyrius deps              # overlay [deps.*] git deps on top
cyrius deps --verify     # confirm against cyrius.lock
```

Running `cyrius deps` against an empty `./lib/` fails on the stdlib
leaves each dep's `dist/<pkg>.deps` sidecar names. This is exactly what
bote's own CI does, in this order — see `.github/workflows/ci.yml`.

⚠ Two traps worth stating, both of which have cost real time:

- **`cyrius build` does an implicit resolve.** So verify a vendored
  dependency's version *after a build*, not after `cyrius deps` — bote
  had a case (3.3.1) where the three-step ended correct and the very
  next build silently reverted a file.
- **Below cyrius 6.7.6 a local `path =` beats `tag =`** and vendors your
  working tree, which can mask a wrong or unpushed tag. Your machine
  passes; a clean CI checkout resolving from `git + tag` gets different
  bytes and fails `cyrius deps --verify`. From 6.7.6 a default resolve
  builds the tag and only prints a hint; `CYRIUS_LOCAL=<dep>` opts into
  the checkout, in `build/local-deps/lib/`.

**nein 1.6.0 vendored `bote-core.cyr` outright over this misread; 1.6.1
retired the vendoring** and consumes bote-core + sigil as ordinary git
deps, the same way daimon does. Vendoring is not the fix — the sync step
is.

If you copy `dist/bote-core.cyr` into your tree anyway (hoosh, szal and
thoth do), **declare `sakshi` in your own `[deps] stdlib`**. `log.cyr` calls
sakshi, and the bundle's requires block does not include it. A pinned
consumer gets it through bote's `deps.libro` (patra includes it); a copied
file brings neither. Measured on 6.7.2, 6.7.9 and 6.7.18: a copied core bundle with
`string fmt alloc vec str syscalls io freelist` stops on undefined
`sakshi_*` functions, and the same list plus `sakshi` builds and answers a
ping.

## Profile selection

Most consumers want the default:

```toml
# Default — recommended unless you have a reason
[deps.bote]
git = "https://github.com/MacCracken/bote.git"
tag = "2.x.x"
modules = ["dist/bote.cyr"]
```

Consumers that only use bote's dispatch surface (Dispatcher /
ToolRegistry / Audit / Codec / Schema) and supply their own
transport stack should use the core-only bundle:

```toml
# Core-only — when your consumer wraps bote's Dispatcher /
# Registry / Audit surface but supplies its own transport
# (e.g. t-ron's SecurityGate middleware over a custom socket).
[deps.bote]
git = "https://github.com/MacCracken/bote.git"
tag = "2.x.x"
modules = ["dist/bote-core.cyr"]
```

The core-only bundle excludes the transport stack
(`transport_stdio`, `transport_http`, `transport_unix`,
`bridge`, `transport_streamable`, `transport_ws`), the
session / discovery / auth / content / host modules, and the
audit_libro / events_majra adapters. The consumer supplies its
own stdlib without `sandhi` / `tls` / `ws_server` /
`sigil 3.x` — a much smaller compile-source budget that fits
under the cyrius 5.10.x 2 MB cap with room for the consumer's
own modules. See
`docs/development/issues/archive/2026-05-10-opt-in-transport-profile.md`
for the rationale and module-split derivation.

### A consumer that serves messages itself

bote allocates what a message needs on a request arena (`bote_req_alloc()`), and a
transport rewinds it at the top of every message with `bote_req_begin()`. bote's own
transports do. A consumer that hands messages to `codec_process_message` or
`dispatcher_dispatch` from its own transport, as a core-bundle consumer does, must call
`bote_req_begin()` before each one, on the thread that serves it. Without it the arena
is never rewound, and what each message put there stays for the life of the process.
Use the `_a` forms (`codec_process_message_a(bote_req_begin(), ...)`) to put the reply
there too; the plain forms put it on the heap. What a plain form judges with, a schema's
walk and its regular expressions, is not kept there (3.8.12): it is judged on a scratch
arena of bote's own, and only the answer is copied out. A threaded server calls
`bote_conn_arm()` once, before its first thread, so each thread has an arena of its own.

To carry `subscriptions/listen` over a connection of its own (3.7.3), such a consumer, on
the full bundle, makes a ListenLine (`bote_listen_line_new(max)`), chains its sink into
the dispatcher's events (`dispatcher_set_events(d, bote_listen_sink(line, dispatcher_event_sink(d)))`),
attaches each connection with a writer (`bote_listen_line_attach(line, d, write_fp, ctx)`,
`write_fp(ctx, json)` sending one message), starts each message with
`bote_listen_line_begin(line)` in place of `bote_req_begin()`, waits for input with
`bote_listen_line_await(line, fd, deadline)`, which writes the changes meanwhile, and
detaches when the connection ends. The core bundle has no listen: a message dispatched
without a stream answers `subscriptions/listen` as a method not found. A resource that
changed is announced with `dispatcher_resource_updated(d, uri)` (3.7.4), which reaches
the listens through the dispatcher's event sinks: a sink of the consumer's own hears it
through `event_sink_with_updated`, and one with no such hook does not.

To let a client cancel a request (3.7.5), such a consumer sets the check its connection
answers after `bote_req_begin()`: `bote_req_set_cancel(fp, ctx)`, `fp(ctx, a, id)` saying 1
when the request whose raw id is `id` has been cancelled. The codec names the request, and a
handler asks through `bote_cancelled()`; once that has said 1 the codec answers the request
with nothing. Both are in the core bundle. A transport whose messages are lines finds a
`notifications/cancelled` among what it has read with `codec_lines_cancel_a`, one whose
messages are frames with `codec_message_cancels_a` (3.7.7), and any reads the `requestId` of
a cancellation the codec would admit with `codec_cancelled_id_a`, and one on a
socket can tell a client gone with `bote_conn_gone` (full bundle). Without a check nothing
is ever cancelled, and every request is answered as before.

Progress (3.7.6) is the same the other way: the consumer sets the writer its connection
sends a notification through, `bote_req_set_progress(fp, ctx)`, `fp(ctx, a, json)` returning
0 when it sent, below 0 when it failed and 1 when it has no stream for it, and a handler
reports with `bote_progress(progress, total, message)`. Both are in the core bundle; the
codec names the request's params, where its `progressToken` is. Without a writer a report
is answered with 0 and nothing goes. The thread that sets the check or the writer is the one
whose asks and reports are answered (3.7.7): set them on the thread that serves the message.

A tool's outputSchema (3.8.0, `tool_def_with_output_schema`) is the core's alone: the
dispatcher lists it and judges each result by it, in the shape the request's revision
allows, so a transport has nothing to wire. A `ToolDef` is 104 bytes from 3.8.0, which
`tool_def_new` allocates; code that reads one through the `tool_def_*` getters is
unaffected.

Titles and icons (3.8.1) are the core's too: `bote_icon_*`, `tool_def_with_title` /
`_with_icons`, `prompt_def_with_title` / `_with_icons`, `prompt_arg_with_title`,
`resource_def_with_title` / `_with_icons`, `resource_template_with_title` / `_with_icons`,
`dispatcher_set_server_details` and `dispatcher_set_instructions`. The structs they extend
grow, each appended at its end: a `ToolDef` is 128 bytes, a `PromptDef` 56, a
`PromptArgument` 40, a `ResourceDef` 64, a `ResourceTemplate` 72 and a `Dispatcher` 176. Code
that makes them with their `_new` functions and reads them through their getters is
unaffected. Set a definition's title and icons before registering it: what is sent is what
passed the check then.

Multi round-trip requests (3.8.2) are the core's: the builders (`bote_input_required_new_a`,
`bote_input_elicit`, `_elicit_url`, `_sample`, `_roots`, `_state`, `bote_input_required_json`),
what a handler reads (`bote_input_possible`, `bote_client_has`, `bote_caps_have`,
`bote_input_responses_a`, `bote_input_response_a`, `bote_input_accepted_a`,
`bote_request_state`) and `dispatcher_set_state_sealer(d, seal_fp, open_fp, ctx)`. The
dispatcher reads a request's client capabilities and opens its state itself, so a transport
has nothing to wire. A consumer whose handlers return state must set a sealer: with none, a
state is refused both ways. The HMAC sealer, `bote_state_hmac_new` /
`dispatcher_with_state_hmac`, is in the full bundle (`src/state_seal.cyr`, sigil's HMAC);
a core consumer can wire it from the full bundle's source or bring its own, `seal_fp(ctx, a,
binding, payload)` and `open_fp(ctx, a, binding, state)` each a cstr on `a` or 0. The
request state grows to 160 bytes and the Dispatcher to 200, each appended at its end, and a
compiled PropertyDef to 96. Code that makes them with their `_new` functions is unaffected.

(3.8.3) A state is bound to who made it: the principal `dispatcher_set_state_principal(d, fp,
ctx)` names for the request's claims (`fp(ctx, a, claims)`, a cstr or 0), else the token the
request presented. ⚠ **A consumer whose transport hands the codec or the dispatcher claims that
are not the presented token, or whose validator returns a record, must set a principal
function** to use states: with none, such a request can be bound to no one, and its handler's
state is `-32603` and a state it brings `-32602`. Through 3.8.2 such a state was bound to no one
and opened for anyone. The full bundle's `jwt_state_principal` names a JWT's issuer and subject.
The arguments are bound too, so a retry sends the request's arguments again. The Dispatcher is
216 bytes, a ToolSchema 32 (its JSON at +24, from `tool_schema_from_json`, which a ToolSchema
made by hand rather than by `schema_new` must zero), and a ToolDef 136.

(3.8.4) A ToolDef is 144 bytes. Registration compiles a tool's input schema each time it is
registered, unless the consumer attached its own with `tool_def_with_compiled`, which it keeps:
a consumer that relied on the first compile sticking, and changed the schema after, now gets
the schema it registered. bote's own libro tools and `fs_read` list an outputSchema, and
`fs_read`'s result carries `structuredContent` beside its text block.

(3.8.5) bote judges `additionalProperties`, the length, item and member counts and the
exclusive bounds, so a schema that says `additionalProperties: false` now refuses an argument
or a result with a member it does not name, where through 3.8.4 it was listed and not judged;
and each keyword judges the values of its kind whatever `type` says. Violations come in the
order of the members. A ToolDef is 152 bytes: the outputSchema `tools/list` sends is
registration's own (`tool_def_output_json`, +144), and `tool_def_output_schema` gives back what
the consumer set. A CompiledSchema is 80 bytes and a PropertyDef 144: one made with
`compiled_new` / `compiled_add_property` / `compiled_set_required` and `prop_new` works as
before.

(3.8.6) A request that names a key twice in any object, keys compared as they decode, or names a
key that is no text, is refused with `-32600` before anything reads it: by the codec, every
transport, and `dispatcher_dispatch` / `dispatcher_dispatch_a`, which check a request a consumer
built by its `params`. Its id is echoed unless the id is what is named twice. ⚠ **Such a request
used to be served**, the schema judging the first of the repeated key and the handler handed
both, so a handler whose parser takes the last acted on a value never judged; a consumer that
worked around that can drop the workaround. `minLength` and `maxLength` count bytes that are no
UTF-8 as a client's decoder reads them, one U+FFFD for each maximal part, where 3.8.5 counted a
stray continuation byte as none. A `required` name pushed into a CompiledSchema's vec after
`compiled_set_required` is met by its member, as through 3.8.4; 3.8.5 refused every call.

(3.8.7) `const`, `enum` of any value, `uniqueItems` and type arrays are judged, so a schema that
names them refuses what it was listed with and passed through 3.8.6: an untyped `enum`, an `enum`
on an integer, a `const`, a repeat in a `uniqueItems` array, a value of a type a type array does
not name. A PropertyDef is 176 bytes; one made by `prop_new` has the new words zeroed, and one a
consumer allocates itself must be that size. `dispatcher_register_tool` takes every name the spec
allows (1 to 128 of `A-Z a-z 0-9 _ - .`), where it also required an underscore and refused `-`
and `.`; `validate_tool_name` is that rule. On `http` and `streamable`, legacy input the codec
cannot take as a request (its answer an error with a null id) is a 400, where it was a 200.

(3.8.8) `prefixItems`, `contains` with `minContains` and `maxContains`, `propertyNames`,
`dependentRequired` and `dependentSchemas` are judged, so a schema that names them refuses what
it was listed with and passed through 3.8.7. A PropertyDef is 208 bytes and a CompiledSchema 104;
`prop_new` and `compiled_new` set the new words, and one a consumer allocates itself must be that
size, with them zeroed (and +192, +200 of a PropertyDef -1).

(3.8.9) `allOf`, `anyOf`, `oneOf`, `not` and `if` / `then` / `else` are judged, on a value and on
an inputSchema's root, so a schema that names them refuses what it passed through 3.8.8. A
judgement has a budget, 100,000 values and 32 for each byte, past which the value is refused as
`more work to judge than its size allows`. A PropertyDef is 272 bytes and a CompiledSchema 112;
`prop_new` and `compiled_new` set the new words, zeroed.

(3.8.10) `$ref` is judged within its document, so a schema that names one refuses what it passed
through 3.8.9, and registration refuses a reference that names nothing there, points into another
document, or leads back to itself with no part of the value judged in between: inline what another
document holds. A judgement is refused as `nested too deep to judge` past 512 steps. An inputSchema
root's composition is judged with no property beside it, `anyOf: []` and `oneOf: []` refuse every
value, draft-06's `if` / `then` / `else` are judged, and an argument's integer past what a handler
holds is refused through `anyOf` and `oneOf` too: each refuses what 3.8.9 passed. A PropertyDef is
280 bytes; `prop_new` sets the new word, zeroed. The request state is 176 bytes.

(3.8.11) `pattern` and `multipleOf` are judged, so a schema that names them refuses what it
passed through 3.8.10. A pattern is read as the official client reads it, under the `u` flag;
one bote's engine declines (lookaround, backreferences, `\p{...}`, modifiers) judges nothing, and
registration warns. `multipleOf` is exact, where the client divides as doubles: a fractional one
is warned of. The core bundle holds 14 modules, `src/regex.cyr` after `jsonx.cyr`, and a
consumer that copies it gets the engine with it. A PropertyDef is 296 bytes. From the review of
3.8.10: registration reads a `$ref` as the official client's Ajv does, so it refuses a reference
to an anchor on the root (write it `#`) and takes a reference relative to the root's `$id`, and
the request state is 184 bytes.

(3.8.12) `patternProperties` is judged, and `additionalProperties` beside it covers the members
neither `properties` nor a pattern names: a schema with both refuses members it passed through
3.8.11. Beside a pattern bote's engine declines, `additionalProperties` still judges nothing. A
CompiledSchema is 128 bytes; `compiled_new` sets the new words, zeroed.

(3.8.13) `format` is judged, each of the formats the official client's ajv-formats knows as it
defines them but `regex`, so a schema that names one refuses what it passed through 3.8.12. A
string format judges strings and a number format numbers, whatever `type` says. Registration
refuses a `format` that is no string, and a formatMinimum, formatMaximum, formatExclusiveMinimum or
formatExclusiveMaximum that is no string, has no `format` beside it, or sits beside a format with
no order: the client's validator refuses each such schema whole. From the review of 3.8.12 it
also refuses, on a schema the client compiles, an `$id` that is no string (and is not false, null
or 0), a `$dynamicAnchor` that is no string and a `$recursiveAnchor` that is no boolean; and an
`$id` with a `.` or `..` segment anywhere, which the client takes for `http:`. An empty `$id` is
none. A property, a pattern or a `required` name `__proto__` is read as the client's Ajv reads it:
no property, no pattern, always met. A pattern is compiled on the heap forms' scratch, so a
compile takes its lock, and a Regex is 40 bytes. The core bundle holds 15 modules,
`src/format.cyr` after `src/regex.cyr`. A PropertyDef is 304 bytes; `prop_new` sets the new word,
zeroed.

(3.8.14) `$dynamicRef` and `$recursiveRef` are judged in 2019-09 and 2020-12, where the official
client's Ajv and JSON Schema read them alike: a `$dynamicRef: "#name"` to the root's
`$dynamicAnchor`, a `$recursiveRef: "#"` beside a root `$recursiveAnchor: true`, each naming the
root. Registration refuses every other reachable one, each a schema the two read apart, and one
beside other keywords under `not` or `if`. From the review of 3.8.13, registration refuses what the
client's validator cannot compile: a keyword whose value is of a kind its engine refuses
(`minLength: "5"`, `if: 5`), a `type` that is no JSON type, `nullable` with no type, `id`, a
`required` or `dependentRequired` entry that is no string, a pattern JavaScript refuses, a format
named for one of Object.prototype, an `$id` twice, a root anchor named twice. A schema that
registered with a WARN through 3.8.13 for one of these is refused now. For an outputSchema whose
root is no object, the form 2025-11-25 lists it in is checked too, and keeps the root's `$schema`
inside `result`, as the SDK does. A result reads a name of Object.prototype (`toString`) as the
client finds it on every object; arguments, by member. A count that is no non-negative integer is
compared as JavaScript compares it (`maxLength: -1` refuses every string), where it judged nothing.
A PropertyDef is 312 bytes, a CompiledSchema 136, a Regex 48; `prop_new` and `compiled_new` set the
new words, zeroed.

(3.8.15) `unevaluatedProperties` and `unevaluatedItems` are judged in 2019-09 and 2020-12, as JSON
Schema collects what each keyword evaluated. Registration refuses where the official client's Ajv
counts what is evaluated otherwise, and where its code throws or skips a keyword whatever is beside
it, none of which needs an unevaluated keyword: `patternProperties` after a count left unset (write it
in an `allOf` of its own), a tuple beside `contains` under `not` or `if`, `contains` judged for each
item or member without `minItems: 1` or a `maxContains`, a `not` of `{}` beside composition under `not`
or `if`. A schema that registered through 3.8.14 in one of those shapes is refused now, and the
message says which. From the review of 3.8.14: `$async`, `enum: []`, two `$id`s Ajv resolves alike,
a `urn:uuid:` `$id` in capitals beside a reference, and a capture group named past ASCII are refused;
a `minContains` below 0 asks for one match; a result's `const`, `enum` and `uniqueItems` compare as
the client's fast-deep-equal does, and a result whose comparison would make it throw is refused; in
arguments a property named `__proto__` judges that member. A PropertyDef is 336 bytes, a
CompiledSchema 152; `prop_new` and `compiled_new` set the new words, zeroed.

## How to produce both bundles

`cyrius distlib` reads `[lib]` (default) or `[lib.<profile>]`
(named profile) from `cyrius.cyml` and emits the matching
`dist/<name>[-<profile>].cyr` deterministically:

```sh
cyrius distlib            # → dist/bote.cyr
cyrius distlib core       # → dist/bote-core.cyr
```

Run **both** commands:

1. **Locally** whenever `src/*.cyr` changes — verify both
   bundles are up to date, then commit them.
2. **In the release workflow** (`.github/workflows/release.yml`)
   before any `git archive` / asset-upload step.

CI gates the dist-freshness for both bundles. See `.github/workflows/ci.yml`.

## Why two profiles?

The transport stack is what drives bote's transitive stdlib
footprint up: pulling `lib/bote.cyr` forces `lib/sandhi.cyr`
(466 KB — HTTP server), `lib/tls.cyr` (31 KB), `lib/sigil.cyr`
(318 KB), and `lib/ws_server.cyr` (11 KB) into the consumer's
expanded-source budget. The 12 transport-and-above modules add
roughly 700 KB of transitive stdlib that pure dispatch consumers
never call.

The core-only bundle was triggered by `t-ron 2.1.x` hitting the
cyrius 5.10.x 2 MB cap when adopting the dist-bundle pattern.
The split is mechanically clean (9 modules, no transitive
transport deps) and consumer-side documentation in t-ron's own
`cyrius.cyml` flips to `modules = ["dist/bote-core.cyr"]` on
the 2.7.2 bump.

The cyrius major jump to 6.1.24 (bote 2.7.3) raised the cap and
the core profile **stays** — it's still a smaller compile-source
footprint and a faster CI for consumers that don't need the
transports. The same jump unblocked folding bote's own three
per-transport binaries back into one; that is still in place and
is a 3.4.x roadmap item. The bundles are unaffected either way.

## What lives in the core 15?

| #  | File                  | Role                                          |
|----|-----------------------|-----------------------------------------------|
| 0  | `src/log.cyr`         | Operational logging through sakshi (3.4.0)    |
| 1  | `src/error.cyr`       | `BoteError` tagged enum                       |
| 2  | `src/protocol.cyr`    | `JsonRpcRequest` / `Response` / `Error`       |
| 3  | `src/jsonx.cyr`       | JSON helpers                                  |
| 4  | `src/regex.cyr`       | `pattern`'s regular expressions (3.8.11)      |
| 5  | `src/format.cyr`      | `format`, as the client's ajv-formats (3.8.13)|
| 6  | `src/registry.cyr`    | `ToolRegistry`                                |
| 7  | `src/prompts.cyr`     | `PromptRegistry` (MCP prompts capability)     |
| 8  | `src/resources.cyr`   | `ResourceRegistry` (MCP resources capability) |
| 9  | `src/events.cyr`      | `EventSink`                                   |
| 10 | `src/audit.cyr`       | `AuditLogger` / `AuditSink`                   |
| 11 | `src/dispatch.cyr`    | `Dispatcher` (2.0 handler ABI)                |
| 12 | `src/codec.cyr`       | Encoder / decoder                             |
| 13 | `src/schema.cyr`      | Schema compile                                |
| 14 | `src/content.cyr`     | Typed MCP content blocks + annotations        |

(The core profile grew 9 → 11 at 3.0.0, when the MCP prompts and
resources capabilities landed, 11 → 12 at **3.3.6** with
`content.cyr`, 12 → 13 at **3.4.0** with `log.cyr`, listed first, 13 → 14 at **3.8.11**
with `regex.cyr`, after `jsonx.cyr`, which `schema.cyr`'s `pattern` needs and which adds no stdlib leaf,
and 14 → 15 at **3.8.13** with `format.cyr`, after `regex.cyr`, whose engine it uses, adding none either.
`log.cyr` matters most of them here:
`codec.cyr` emits a DEBUG line per request through it, so a core
consumer that sets sakshi to DEBUG sees its dispatch traffic. bote
never configures sakshi from library code — the consumer's
`sakshi_set_level` / `sakshi_output_*` decide where, and whether, those
lines go. `src/sandbox.cyr` was weighed at 3.3.13 and kept
OUT: it fits mechanically — no sigil, no transport, no new stdlib
leaf — but this profile exists to bound a consumer's compile set,
and while eight repos vendor `bote-core.cyr`, none references a
`sandbox_*` symbol. It joined the FULL bundle in the same release.
Revisit if a core-profile consumer wires a sandbox backend.)

`content.cyr` is in core because content blocks are the tool-result
format **every** handler emits, transport or not. Before 3.3.6,
core-profile consumers (nein's `mcp` module, t-ron) hand-rolled
`{"content":[…],"isError":…}` with a raw `str_builder` — which means
each of them re-implemented JSON string escaping, the exact duplicated
injection surface this profile exists to prevent. It adds **no** new
stdlib leaves. Its only external references are `_json_emit_escaped` (in
core through `jsonx.cyr`; since 3.4.6 an alias of the public
`bote_json_escape`), `_dsp_tool_error_result` (`dispatch.cyr`),
`str_builder_*` / `str_data`, `vec_*` and `alloc`. It is listed **last** in
`[lib.core]` because cyrius is single-pass and it calls into both.
`bote_json_escape(sb, cstr)` is the escaper a consumer's own JSON should use:
it appends a cstr as a JSON string body, escaping every control byte as
RFC 8259 §7 requires.

Stdlib footprint of the 15 modules: `string`, `fmt`, `alloc`,
`vec`, `str`, `tagged`, `assert`, `fnptr`, `hashmap`, `bayan`,
`chrono`, `freelist`, `sakshi`. `sakshi` is not on the sidecar: distlib's
verify finds it already in scope through `lib/patra.cyr`'s own include
(patra is libro's named dep), which is how a pinned core consumer gets it
too; a copied bundle must declare it (above). `log.cyr` sizes its stack line
buffer with a `const`, which is what sets the 6.7.2 minimum. No core module
calls into `tls` / `sandhi` /
`sigil` / `ws_server` / `slice`.

**`dist/bote-core.deps` names neither `sigil` nor `ct` (3.4.11).** A
core consumer gets libro transitively through bote's `deps.libro`,
whatever profile it selects, and libro's thin `sigil-mldsa` selection
serves it. Through 3.4.10 `distlib`'s verify recorded those two leaves
against the core bundle, a cyrius defect fixed at 6.7.13. Measured with
a one-`ping` core consumer on 3.4.11 and cyrius 6.7.18: 1,797,872 B
pinned. The copied-bundle numbers are in the 3.4.11 CHANGELOG. A
consumer that calls sigil or ct from its own code declares them.

**Nor does it name `bayan` (3.4.13).** No core module calls it.
`jwt.cyr` and `web_tools.cyr` do, and each includes it itself, so
`dist/bote.deps` is unchanged. What this saves depends on how you take
core. Measured with a one-`ping` consumer on 3.4.13 and cyrius 6.7.18:

| How | Binary | Why |
|---|---|---|
| copied, no `bayan` in your own `[deps] stdlib` | 276,192 B | nothing brings it (1,239,776 B through 3.4.12) |
| pinned | 1,797,856 B | bote's `deps.libro` brings libro's thin sigil, which includes bayan |

Declare `bayan` if your own code calls `json_*`, `base64_*` or anything
else of its.

That "no core module calls sigil" is why `src/jwt.cyr` and `src/pkce.cyr` are in
`[lib]` only and **not** in `[lib.core]`, even though both ship
in the full bundle from 3.2.0: jwt needs `hmac_sha256` and pkce
needs `sha256`. Pulling the sigil bundle into the transport-free
profile would defeat the reason that profile exists. A consumer
who wants JWT from the core bundle has to argue for it.

Drift guard: `tests/bote_core_only_smoke.tcyr` includes only
`dist/bote-core.cyr` plus the minimal stdlib and runs a
`dispatcher_new + registry_register + dispatcher_handle`
round-trip. If a future bote change wires a core-module symbol
against a transport-module helper, the smoke fails at CI time.

Since 3.3.6 it also asserts the content-block **bytes**, not just that
they link: that a quote and a backslash come back escaped, and that
`content_array` omits `isError` while `content_array_error` emits
`"isError":true`. A link-only check would not notice an escaping
regression, and escaping is the whole reason `content.cyr` belongs in
this profile. Both assertions are mutation-proven — expecting the raw
unescaped string exits 7, and asserting `isError` on the success array
exits 9.
