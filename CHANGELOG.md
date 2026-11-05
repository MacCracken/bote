# Changelog

All notable changes to bote are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### Conventions

Adopted in **2.7.0** — the **`## [Unreleased]`** section below
accumulates entries during the patch cycle. When a release ships,
the **`## [Unreleased]`** header is renamed to `## [VERSION] —
DATE — headline` and a fresh, empty `## [Unreleased]` is seeded
back at the top. Avoids the "TODO" placeholder churn we used to
have per release.

## [Unreleased]

_(empty)_

## [3.8.13] — 2026-11-04 · Formats

**`format` is judged**, as the official client's own validator judges it. The client validates with Ajv and
ajv-formats 3.0.1 in its full mode, so a format ajv-formats knows is enforced and one it does not is ignored, and
bote does the same. Each of its 26 formats is judged as ajv-formats defines it, but `regex`. A string format judges
strings and a number format numbers, whatever `type` says, as Ajv applies one. Through 3.8.12 `format` was warned
of and judged nothing.

How each format is judged:
- **By the client's own expression**, compiled once by 3.8.11's engine: `uri`, `uri-reference`, `uri-template`,
  `email`, `hostname`, `ipv4`, `ipv6`, `uuid`, `json-pointer`, `json-pointer-uri-fragment` and
  `relative-json-pointer`. Each was written for JavaScript's `RegExp` with the `i` flag and without `u`. Its letters
  are folded here (`a-z` written `a-zA-Z`), and its classes are ASCII or name no letter, so code points and UTF-16
  units give one verdict.
- **By hand**, where ajv-formats' definition is code, a lookahead or a multiline expression:
  - `date`, a day the month has, February's 29th in a leap year;
  - `time` and `iso-time`, a leap second read in UTC. A second is read as a double, as the client reads it, so
    `12:00:59.9999999999999999Z` is 60 seconds and refused;
  - `date-time` and `iso-date-time`, split at `t`, `T` or any space JavaScript's `\s` takes, U+3000 among them;
  - `duration`, its parts in order, `T` followed by one at least, or weeks alone;
  - `url`: its `iu` flags fold `ſ` to `s` (`httpſ://a.bc` is a url) and `K` to `k`. Its lookaheads refuse the
    private dotted quads (10, 127, 169.254, 192.168, 172.16 to 172.31). Each `@` is tried as the userinfo's end,
    each try stopping at the next one, so the whole is linear;
  - `byte`, with the `m` flag ajv-formats gives it, so a string passes when any line does: `""` and `"!!!\n"` pass,
    `"!!!"` does not;
  - `hostname`'s length, its expression's lookahead: 253 characters, or 254 ending in a dot.
- **Numbers**: `int32` and `int64` exactly, `float` and `double` any number. Where the client reads a double,
  bote is the stricter. `int32` refuses `5.0000000000000001`, `2147483647.0000001` and `1e-400`, each of whose
  doubles is an integer to the client. `int64` refuses `9007199254740993.5`, and `1.7976931348623158e308`, which
  is past the largest double though the client rounds it down to it.
- **`password` and `binary`** take every string.

Each format's reading is charged to the judgement's budget, step by step or a step a byte, and is linear in the
string. Twice the string takes about twice the time for each format measured, and for `url` with 60,000 `@`s.

**A third differential holds it to the client** (`scripts/format-diff.sh`, which needs the client that
`./scripts/conformance.sh` installs; not CI). Strings near each format's edges are drawn: the client's own
examples mutated, and strings over each format's alphabet. Numbers are drawn at every bound. Each is judged by the
client's `AjvJsonSchemaValidator` and by bote. Of 504,138 values:
- every string is judged alike;
- no number bote takes is one the client refuses;
- 40 numbers bote refuses exactly are ones the client's double takes, the five kinds above.

**Registration refuses what the client cannot compile.** The client's validator throws on each of these, and
compiles nothing of the schema:
- a `format` that is no string;
- a `formatMinimum`, `formatMaximum`, `formatExclusiveMinimum` or `formatExclusiveMaximum` that is no string;
- one of those with no `format` beside it;
- one of those beside a format the client knows that has no order. Only the dates and times have one, and
  `password` and `binary` are let pass.

It warns of a `format` it does not judge: `regex`, which the client checks with JavaScript's own syntax, and a name
the client does not know (`idn-email`, `iri`), which neither judges. The four `format*` limit keywords, which the
client compares through JavaScript's `Date`, are warned of as not enforced.

**Twelve defects in 3.8.12 land beside it**, from an independent review of that release. The worst is a denial of
service in `patternProperties`: each pattern made its arrays ready for each key past the budget, uncharged, 7.3 s for
209 KB. See the correction notice in 3.8.12.

The stated minimum cyrius (6.7.13) is unchanged.

### Added

- **`src/format.cyr`** (core, after `src/regex.cyr` in both profiles: the core bundle has 15 modules, the full one
  41):
  - `bote_format_id(name)` and `bote_format_name(id)` (`BOTE_FMT_*`);
  - `bote_format_kind(id)`: 1 strings, 2 numbers, 0 nothing (`regex`, and an unknown name);
  - `bote_format_ordered(id)`;
  - `bote_format_test_a(a, id, buf, s, e, bud)` on a JSON string body: 1, 0, or -1 for the budget spent. What it
    decodes is on `a` and given back, on the calling thread's request arena when `a` is no bote arena;
  - `bote_format_number(id, v)`.

  The expressions are compiled once, published by compare-and-swap and readied at registration. Private names are
  `_bfm_*`, as lib/bench.cyr already has `_fmt_time`.
- **`format` in the schema** (`src/schema.cyr`): PropertyDef is 304 bytes, +296 the format. A violation reads
  `does not match the format email`.
- **`_sch_compiled_why`**, registration's refusals above, over every schema the dialect's engine compiles, as
  `_sch_refs_why` walks them. It checks an outputSchema, an inputSchema given as JSON, and a ToolSchema's listed
  form when a property may name a format.
- **`scripts/format-diff.sh`**, with `tests/fixtures/format_diff_gen.cjs` and `tests/fixtures/format_diff.cyr`. CI's
  "Differentials build" step builds its comparison program.
- **`tests/bote.bcyr`**: `validate_formats`, a call's arguments judged by an email, a date-time, a uri and an int32.

### Fixed

- **A pattern made its arrays ready past the budget** (HIGH). Each of 64 patterns `^p<i>.{0,2000}$` cleared its
  4,005 marks for each key, uncharged, 1.8 s for 49 KB of keys and 7.3 s for 209 KB.
  - A program past `BOTE_RX_SPARSE_FROM` (64) steps keeps the steps seen as a sparse set, which needs no clearing.
    A shorter one clears an array of at most 64 words, where a sparse set's extra reads would cost it a fifth more.
  - `bote_rx_test_a` returns -1 before anything is made ready when the budget is spent.
  - The patterns of `patternProperties` stop being tried once it is spent.

  The 64 patterns over 4,000 keys now take a few times what 64 patterns of four steps take.
- **`contains` counted a refusal by an integer as a match** (MEDIUM, from 3.8.12's new `_sv_matches` 3).
  `{"contains":{"not":{"type":"integer"}}}` passed `[1e30]` to a handler. A match is 1 or 2, as `anyOf` and `oneOf`
  read it, and `propertyNames` reads 3 as no match too (`_sch_no_match`).
- **Registration kept 382 KB of the heap for each large pattern, and compiled each twice** (MEDIUM). 100 patterns
  `^p<i>.{0,2000}$` kept 74 MB.
  - A pattern is compiled on the heap forms' scratch, under its lock, and only its program and its classes' ranges
    are kept: 96 KB for 4,005 steps.
  - Registration's WARN compiles with `bote_rx_why_of(src, names_out)`, which keeps nothing.

  The 100 patterns keep under 12 MB.
- **The clearing charge refused small inputs** (LOW–MEDIUM). Charged by the program and not by the input, it refused
  300 strings `"ab"` against `^.{0,2000}$`. No program clears more than 64 words now, and none of it is charged.
- **The duplicate-name check was quadratic** (LOW–MEDIUM): 32,000 groups took 13 s.
  - Each name keeps a count of the groups that can take part with one from where the parser reads.
  - A disjunction's alternative takes its names out when the next begins, and gives them all back when the
    disjunction ends.

  So a name costs one look, and 32,000 groups take about twice 16,000's time.
- **Registration took schemas the client cannot compile** (LOW), found by a wider grammar in `scripts/ref-diff.sh`:
  1,503 of 100,000. Over the schemas the dialect's engine compiles, it refuses (`_sch_compiled_why`, which holds
  3.8.13's format checks too):
  - an `$id` that is no string and that JavaScript reads as true (`5`, `true`, `{}`; the client calls `.replace`
    on it);
  - in 2019-09 and 2020-12, a `$dynamicAnchor` that is no string, and a `$recursiveAnchor` that is no boolean.

  `_sch_id_ok` refuses an `$id` with a `.` or `..` segment, written so or as `%2e`. fast-uri takes those out of a
  `ws:` URI, or one with no authority, otherwise than of the references beside it; refusing them everywhere is the
  stricter side, as the client takes them for `http:`. An empty `$id` is none, so a relative reference beside one
  resolves to nothing. Against the client: none of 100,000 bote takes is one it cannot compile, and 11,914 bote
  refuses it takes.
- **`formatMinimum` and its kin were missing from Ajv's rule list** (LOW), which decides whether an `if` beside a
  `then` of one is compiled.
- **Two groups of one name are warned of** (LOW): `pattern naming two groups alike, which Node before 23 refuses
  whole`. The engine takes them, as ECMA-262 does since 2025 (`bote_rx_names_twice`, Regex 40 bytes, +32).
- **The plain form kept the repeated-key check's table** (LOW, older: 3.8.6). `dispatcher_dispatch` with 2,000 keys
  kept 98 KB a call. Handed the heap, the walk is on the calling thread's request arena (`_dsp_keys_twice`), whose
  exact walker gives back what it takes. Not the heap forms' scratch: every plain dispatch would take its lock.
- **`__proto__`** (LOW, older). The client's Ajv leaves a property or pattern of that name out, so a member named
  `__proto__` is additional, and `required: ["__proto__"]` is always met to it, its `data.__proto__` the prototype.
  bote reads all three so now.
- **The `multipleOf` WARN's double bound** is where a double turns infinite (just past 1.7976931348623158e308), not
  the largest double. And an exponent is read to 16 digits, where the comment said 15.

Not fixed, and documented:
- A refusal by an integer (`_sv_matches` 3) means a refusal inside used the integer, not that the outcome depends on
  it. So `{"not":{"allOf":[{"not":{"type":"integer"}},{"type":"string"}]}}` refuses `1e30`, which it passes under
  either reading. That is the stricter side.
- Registration's WARNs cover a schema's root and its top-level properties, so a declined pattern nested deeper
  registers silently.

### Changed

- `BOTE_UNENFORCED` loses `format` and gains the four `format*` limit keywords.
- `tests/bote.tcyr` and `tests/bote_output.tcyr` take `formatMaximum` as their example of a keyword bote does not
  judge, and `format: email` as one it does.

### Tests

- **`tests/bote_format.tcyr`**, 350, new:
  - Each format by name and kind.
  - 280 rows whose expected verdicts are the client's own validator's. They cover:
    - leap seconds through offsets with minutes;
    - a second rounded as a double;
    - date-time's separators;
    - duration's order, and weeks alone;
    - url's userinfo, ports, private quads, ſ and K;
    - hostname's 253;
    - byte's lines;
    - ipv6's quirks (`1:2:3:4:5:6:7::`).
  - Five numbers bote judges exactly where the client's double takes them.
  - Through the walk: strings and numbers apart, under `anyOf` and `not`, a property, an item at its path.
  - Arguments through the dispatcher, and a result.
  - Registration's refusals of what the client cannot compile, through a `$ref` and in each kind of inputSchema,
    and six schemas it takes.
  - The WARNs.
  - The budget, linear in the string for six formats and for url with 60,000 `@`s, and 256 alternatives over 60 KB
    refused in time.
  - 0 heap bytes for fifty refused calls and for the heap handed in; the arena's first chunk.
  - Eight threads compiling one program at once.
- **`fuzz/schema_walk.fcyr`**: `format` of `date`, `ipv4` or `int32`, judged by hand in another form, with strings
  in the pool that meet each.
- **52 mutants, 52 killed.** One survived the first run, weeks followed by anything (`P1W1D`), now a row.
- **`tests/bote_regex.tcyr`**, 160, from 146, the review of 3.8.12:
  - an empty string against 4,001 steps costing its first step, and 2,000 of them a few times a one-step program's
    time;
  - a spent budget returning at once;
  - programs either side of the sparse set's bound;
  - a compile keeping only its program, and twenty why-only compiles keeping nothing;
  - names flagged when given twice, 32,000 named groups in about twice 16,000's time, and 1,001 alternatives of one
    name.
- **`tests/bote_schema_pattern.tcyr`**, 157, from 147, the review of 3.8.12:
  - 64 patterns of 4,005 steps over 4,000 keys in a few times 64 four-step patterns' time;
  - 8,000 keys each matched by 64 patterns whose first turn is 2,000 steps, refused in about 4,000's time;
  - the names WARN;
  - 100 large patterns registered under 12 MB;
  - the `multipleOf` bound.
- **`tests/bote_schema_ref.tcyr`**, 266, from 240, the review of 3.8.12:
  - `contains` of a refusal by the integer, three ways;
  - every `$id` and anchor the client refuses, checked against it, and seven it takes;
  - an empty `$id`;
  - `formatMinimum` in an `if`'s `then`, with a format and alone.
- **`tests/bote_format.tcyr`**, `__proto__` read as the client reads it.
- **`tests/bote_request_keys.tcyr`**, 94, from 92: the plain form's heap.
- **`tests/bote_schema_parts.tcyr`**, 79, from 78: `propertyNames` takes `format: regex` as its example of a keyword
  bote does not judge, and judges `format: email` on each key.
- **34 mutants of the review's fixes, 32 killed, one equivalent, and one showed code that did nothing.** The
  `patternProperties` loop's own budget check survives: `bote_rx_test_a` already returns -1 at once past the
  budget, so the check saves only the calls. The plain form's mark and release around its walk survived too: the
  exact walker gives back what it takes, so the pair was removed. Two survived the first run, each a gap now
  tested: the why-only compile keeping a class's ranges on the heap (its patterns had no class), and the rule
  list's `format*` keywords (its `then` held a `format` beside them). Four were of the matcher's split into an
  array and a sparse set: one never taking the sparse path, its bound moved, its set not reset, its steps
  uncharged. Two were of the plain form's walk: handed the heap, and the release above. One was of the first byte
  `required: ["__proto__"]` is looked for by (the Performance section).
- **`scripts/ref-diff.sh`**: the reviewer's wider grammar, and `$id`s and anchors that are no string. Its comparison
  takes registration's own verdict (`registry_check_tool_def`), where it took `_sch_refs_why` alone.

### Performance

3.8.12 against this release, 15 alternating rounds on one core, minimums (`ab`, on a quiet host; `validate_formats`
is new and runs here only). Every other row is within ±3.6%:

| Bench | 3.8.12 | 3.8.13 | Δ |
|-------|-------:|-------:|--:|
| `validate_formats` | — | 13,009 ns | new |
| `regex_slug` | 2,283 ns | 2,609 ns | +14.3% |
| `validate_compiled_text` | 2,804 ns | 3,111 ns | +10.9% |
| `validate_pattern_props` | 1,535 ns | 1,609 ns | +4.8% |
| `validate_compiled_nested` | 1,110 ns | 1,131 ns | +1.9% |
| `schema_compile_nested` | 36,199 ns | 37,301 ns | +3.0% |
| `dispatch_initialize` | 2,004 ns | 2,049 ns | +2.2% |
| `dispatch_tools_list` | 1,716 ns | 1,803 ns | +5.1% |
| `codec_serialize_response` | 178 ns | 190 ns | +6.7% |
| `fetch_in_child_empty` | 188.5 µs | 175.4 µs | −6.9% |

- **The engine's short match is about 330 ns slower, and nobody knows why yet** (`regex_slug`). The program and the
  functions on its path are 3.8.12's, step for step, and code position, the program's address, the arena, the stack,
  the names and dead-code elimination are each ruled out (the watch list). `validate_compiled_text`'s 307 ns is the
  same cost: its pattern runs on the engine. `validate_pattern_props` runs it on short keys. The sparse set, which
  the making-ready fix needs past 64 steps, is a separate matcher, so a short program never reads it.
- **Four formats on one call's arguments are 13.0 µs** (`validate_formats`: an email, a date-time, a uri and an
  int32). The email and the uri run the client's own expressions on the engine.
- **A required name costs a byte's look more** (`validate_compiled_nested`). `required: ["__proto__"]` is always met,
  as the client meets it. Through `streq`, which measures both strings, the check took this row to +5.9% in the first
  A/B. It now looks at the first byte first.
- **Registration compiles an object about a microsecond slower** (`schema_compile_nested`). It reads `format` and
  leaves a property named `__proto__` out. That is paid once per registration, never per call.
- **A plain dispatch with params walks for a key twice on the request arena** (`dispatch_initialize`, about 45 ns
  of its 2.2%). `dispatch_tools_call` carries params too and is +0.1%.
- **`dispatch_tools_list`'s and `codec_serialize_response`'s rows are where the code now sits, not work.** A list
  request has no params and reaches none of the new code, and the serializer did not change. With 3.8.12's tree and
  only this release's `src/dispatch.cyr`, unchanged rows moved by up to 13% (`session_validate`).
- **The first two A/Bs, on a loaded host, moved unchanged rows by up to 24%** (`jwt_cached_es256_sourced`,
  `jwt_cached_es256`, `session_validate`, each +19 to +24% in one and −10 to +1% in the other). On a quiet host
  they are within ±0.4%.

## [3.8.12] — 2026-11-04 · Pattern properties

> ⚠ **Corrected at 3.8.13.** These claims below were not true, found by an independent review of this release.
> - "Each key's match against each pattern is charged to the judgement's budget". The budget did not bound the
>   work. Each pattern made its arrays ready for each key past the budget, clearing as many words as its program has
>   steps, and none of that readying was charged. 64 patterns `^p<i>.{0,2000}$` over 49 KB of short keys took
>   1.8 s, and over 209 KB 7.3 s.
> - "100 short strings against `^.{0,2000}$`, a common bound, are judged". The clearing was charged an eighth of a
>   step a word for each match, a cost by the program and not by the input. So 300 strings `"ab"`, 1.5 KB, were
>   refused as too much to judge, and the client takes them.
> - "none bote takes is one the client cannot compile". Against a wider grammar, 1,503 of 100,000 schemas were
>   taken that the client cannot compile:
>   - an `ws:` or `wss:` `$id` with a `.` or `..` segment;
>   - an `http:` one with no authority and a dot path;
>   - an empty `$id` beside a relative reference;
>   - an `$id` that is no string;
>   - a `$dynamicAnchor` that is no string.
> - "`contains` still counts a match that took the integer as any match". It also counted a refusal by the integer,
>   `_sv_matches`' new 3, as a match. `{"contains":{"not":{"type":"integer"}}}` passed `[1e30]` to a handler, which
>   3.8.11 refused.
> - "`dispatcher_dispatch` ... keeps its reply". It also kept the repeated-key check's table, 98 KB for 2,000 keys
>   a call, older than this release (3.8.6).
> - "`(?<a>x)|(?<a>y)` is still taken", as ECMA-262 has taken it since 2025. But Node before 23 refuses such a
>   pattern whole, and the client runs on Node 20 and later, so there it can compile none of a schema holding one.
> - "`_sch_can_fail`, from Ajv's own `RULES.all`". The client's `addFormats` adds four keywords to that list,
>   `formatMinimum` and its kin, which were missing.
> - The duplicate-name check compared each named group with every one before it: 32,000 groups took 13 s.
> - Registration kept 382 KB of the heap for each `^.{0,2000}$` and compiled each pattern twice: 100 such patterns
>   kept 74 MB. The 3.8.11 claim "A pattern compiles once, at registration" did not hold either.
> - "`_sch_num_norm` reads 15 digits of them". It reads 16.
> - "past a double's range" was warned of for `1.7976931348623158e308`, which a double reads as its largest.
> - A property or pattern named `__proto__`, which the client's Ajv leaves out, was judged. Older than this release.

**`patternProperties` is judged**, by 3.8.11's engine: a member whose key matches a pattern, anywhere in the key and
by the code points it decodes to, is judged by that pattern's schema. Every pattern it matches judges it, and so
does `properties` when it names the key too. **`additionalProperties` beside it is judged**, covering only the
members neither names: `{"properties":{"id":{}},"patternProperties":{"^x-":{}},"additionalProperties":false}` takes
`{"id":1,"x-a":2}` and refuses `{"y":2}`. Through 3.8.11 both were warned of and judged nothing. Each key's match
against each pattern is charged to the judgement's budget, step by step: 100 keys of 500 bytes against 64 patterns
are refused as too much to judge, in under 64 times one pattern's time.

A pattern the engine does not take (lookaround, backreferences, `\p{...}`) judges nothing, with a WARN. Which members
it covers is then unknown, so `additionalProperties` beside it judges nothing either, and registration says so.

**Nine defects in 3.8.11 land beside it**, from an independent review of that release and from two differentials,
one repaired and one new. See the correction notice in 3.8.11. Four are denials of service in the engine:
- a registration that never ended;
- work a match did and never charged;
- a class read member by member;
- 12.8 MB kept a call through a dispatcher's plain form.

**A second differential holds registration to the official client's own validator** (`scripts/ref-diff.sh`, which
needs the client that `./scripts/conformance.sh` installs; not CI). 100,000 random root `$id`s, well formed and not,
and references through them are each compiled by the client's `AjvJsonSchemaValidator`, with the Ajv it bundles, and
checked by bote's registration:
- none bote takes is one the client cannot compile;
- 9,430 bote refuses are ones the client takes. Each is an odd spelling the client folds, or an `$id` the client
  takes that bote cannot be sure it reads one way. Registration says why.

The regex differential, its generator repaired, now draws 48,781 distinct patterns of 60,000, with 212,680 strings:
- every verdict is the same as ECMA-262's search;
- no pattern JavaScript refuses is taken;
- none it takes is refused as a syntax error;
- 1,580 are declined for a construct the engine does not take.

On four strings V8's own `test` departs from that search, and bote is held to the search. JavaScript's engine,
anchored at each code point in turn, gives the spec's answer:
- `[^a-z]$` refuses "😁" (which `^[^a-z]$` takes);
- `\B` matches between the two halves of a surrogate pair.

The stated minimum cyrius (6.7.13) is unchanged.

### Added

- **`patternProperties`** (core, `src/schema.cyr`): CompiledSchema is 128 bytes, +112 each pattern the engine takes,
  compiled, beside its schema, and +120 whether one was declined. `_sv_object` tests every member's key against
  every pattern, on the key as written in the document, each step charged.
- **`scripts/ref-diff.sh`**, with `tests/fixtures/ref_diff_gen.cjs` (it drives the client's own
  `AjvJsonSchemaValidator`, a fresh one for each schema, as the client caches one by its `$id`) and
  `tests/fixtures/ref_diff.cyr`. It fails on a schema bote takes that
  the client cannot compile, and writes each disagreement to `build/ref-diff/`. CI's "Differentials build" step
  (renamed from "Regex differential builds") builds both comparison programs.
- **Registration refuses an `$id` the client's resolver cannot read** (`_sch_id_ok`, from the new differential): the
  client's Ajv throws on one and compiles nothing of the schema, or reads one two ways. Every `$id` its walk reaches is
  checked, the root's and those below it, refs or not. It refuses any of these, a few the client takes among them:
  - a control, a space, DEL, or one of `` "<>\^`{|} ``;
  - a `%` before no two hex digits;
  - a colon before the first `/`, `?` or `#` that ends no scheme;
  - an authority whose host is no domain, dotted quad or IPv6 literal (as a website URL's), whose port is no number
    to 65535, or whose userinfo holds `[`, `]`, `@` or a byte past ASCII;
  - a path holding an empty segment;
  - a `urn:` whose name is no `nid:nss` as fast-uri's URN_REG reads one, or that holds a `%`, or a `.` or `..`
    segment.

  It reads `the outputSchema names in $id a URI the official client's resolver does not take`.
- **Registration warns of an integer `multipleOf` a double holds inexactly or not at all** (`_reg_multiple_size`):
  - past 2^53, `multipleOf past 2^53, judged exactly here and perhaps not by a client holding doubles`;
  - past a double's range, `multipleOf past a double's range, which a client holding doubles reads as infinite`.

### Fixed

- **A quantifier compiled every copy of what matches only the empty string** (MEDIUM). Registration of
  `(((a{0}){10000}){10000}){10000}` made 10^12 copies of nothing and never ended. What matches only the empty string
  is now one EMPTY node:
  - a concatenation leaves it out;
  - a quantifier on it, or one with a bound of 0, is it;
  - so is an alternation of nothing else.

  So every other node compiles to a step at least, and copies are bounded by the program's 4,096 steps. Three such
  levels compile in a few times `a{100}`'s time. `(?:){0,10000}` and `(?:|){5000}` compile, where they were past the
  bound.
- **A match's making ready went uncharged** (MEDIUM). It cleared an array as long as the program at every call, and
  the first turn's steps were charged only when a second followed. 20,000 empty strings against a 3,000-step
  alternation cleared 60 million words and took 40 million steps free.
  - The clearing is charged, one for every eight marks (`BOTE_RX_CLEAR_WORDS`): a cleared word is a store, about an
    eighth of a step.
  - The first turn is charged before it decides.

  Those 20,000 strings are refused as too much to judge in a few times a one-step pattern's time. 100 short strings
  against `^.{0,2000}$`, a common bound, are judged. Charging the clearing in full refused those 100. A sparse set,
  which needs no clearing, cost every short pattern a fifth more: the slug's match went from 2.5 to 3.1 µs.
- **A class read its members one by one** (MEDIUM). Its ranges are sorted and merged at compile, and a code point is
  looked for by halving. A 4,000-member class against 20,000 code points takes 1.5 ms, 1.5 times a 4-member class's
  1.0 ms. Through 3.8.11 it took 162 ms, 175 times the 4-member class's 0.92 ms.
- **Judged on the heap allocator, a judgement kept what it made** (MEDIUM). A dispatcher's plain form passes the
  heap allocator, where a release does nothing.
  - `bote_rx_test_a` handed an allocator that is no bote arena takes its arrays from the calling thread's request
    arena and gives them back.
  - `compiled_check_a` and `_sch_check_value_a`, so handed, judge on the heap forms' scratch and copy out only
    their answer (`_sch_strs_to_a`), as `compiled_validate` has since 3.8.8.

  `dispatcher_dispatch` of 100 strings through `^.{0,2000}$` keeps its reply, where it kept 12.8 MB. It also keeps
  none of the tables and quiet judgements 3.8.10's plain-form walks kept.
- **Two groups of one name that can both match were taken** (LOW). JavaScript refuses such a pattern
  (MightBothParticipate), so the client compiled none of the schema. The parser keeps the alternatives each named
  group is in, and refuses a name given twice outside two alternatives of one disjunction:
  `two groups named alike that can both match, which the u flag refuses`. `(?<a>x)|(?<a>y)` is still taken.
- **Registration resolved references otherwise than the client's resolver** (LOW). Each of these is read as fast-uri
  reads it now:
  - a last `..` leaves its slash (`a/b/..` against `http://x/a` is `/a/`, not `/a`);
  - a query's slashes are no segments (`/a?b/../../y` read as `/y`);
  - a userinfo's case counts.

  In each case bote found in the document a resource the client cannot, and the client compiled none of the schema.
- **Registration and the judgement followed an `if` Ajv ignores** (LOW). Ajv compiles `if` only beside a `then` or
  an `else` that can fail: one that is no `true`, no `{}`, and names a keyword Ajv compiles in its dialect
  (`_sch_can_fail`, from Ajv's own `RULES.all`).
  - `{"if":{"$ref":"#"}}` was refused as a cycle. It is taken, as the client takes it.
  - A loop through such an `if` no longer makes every value too deep to judge.
- **An exponent of seven digits or more was read as its first six** (LOW, older: 3.8.2). `_sch_num_norm` stopped
  reading one at 100,000, so `1e1000000` met `maximum: 1e100000`, and `1e100000` was a multiple of `1e1000000`. It
  reads 15 digits of them.
- **`not` and `if` typed nothing** (LOW). The budget block is 64 bytes: +56 is set when a quiet judgement refuses an
  argument's integer past a handler's range for being one. `_sv_matches` returns 3 for a failure by that.
  - `not` of a schema that took the integer refuses by it.
  - `not` of one that refused it takes the value by it, so `{"not":{"not":{"type":"integer"}}}` refuses `1e30` for
    a handler.
  - An `if` decided by the integer decides by it where its branches would decide otherwise.
  - `anyOf` and `oneOf` pass a refusal by it outward.

  `contains` still counts a match that took the integer as any match. A schema that says only that some item is an
  integer passes `[1e30]` to a handler.
- **The regex differential drew 307 patterns** (tooling): `Math.imul` in the generator's step. It also judges bote by
  ECMA-262's search where V8's unanchored `test` departs from it, and counts the departures.

### Changed

- `additionalProperties` beside `patternProperties` is judged, unless a pattern is one the engine declines.
- Registration warns of a `patternProperties` pattern the engine does not take, and why (`a patternProperties
  pattern bote's engine does not take, for a lookbehind`), of `additionalProperties` beside one, and of a
  `patternProperties` that is no object. It no longer warns of `patternProperties` as unenforced, nor of
  `additionalProperties` beside it.

### Tests

- **`tests/bote_regex.tcyr`**, 146, from 100, the review of 3.8.11:
  - Handed the heap, 100 matches of 4,001 steps keep nothing.
  - An empty string against 4,001 steps is charged an eighth of a step for each mark cleared, and its first step.
  - The first turn charged before it decides.
  - What matches only the empty string: `(?:){10000}`, `a{0}`, `(?:|){5000}` and `(?:){0,10000}` one step. Three
    levels of 10,000 copies, of `a{0}`, an empty group and two in a row, compile in a few times `a{100}`'s time.
    Matching unchanged around them, `^` still anchoring, and their quantifiers still read.
  - Names: six kinds of pattern refused, seven taken (V8's verdicts).
  - Classes: overlapping, out of order, complements, interleaved escapes, chains and adjacent ranges. A 4,000-member
    class at its ends, its middle and between members, timed against a 4-member one.
- **`tests/bote_schema_ref.tcyr`**, 240, from 169, the review of 3.8.11:
  - `if` beside nothing that can fail, in every dialect, as Ajv reads it, and judged.
  - References resolved as fast-uri resolves them.
  - Twenty-one `$id`s refused (fifteen the client's Ajv refuses, six on bote's stricter side) and thirteen taken,
    each checked against the client. One with no `$ref`, and one below the root.
  - `not` and `if` typing an argument's `1e30`: twelve schemas through the dispatcher, and a result's taken.
- **`tests/bote_schema_pattern.tcyr`**, 147, from 79:
  - Members by the pattern their key matches; two patterns on one key; a key judged by `properties` and a pattern.
  - `additionalProperties` false and a schema, each beside patterns.
  - Keys as they decode, by code points, unanchored and anchored; a 300-byte key past eight names.
  - A declined pattern judging nothing while the others judge.
  - Under `anyOf`; arguments through the dispatcher; a result.
  - 100 long keys against 64 patterns refused in time.
  - The four WARNs; 0 heap bytes for fifty refused calls; 2,000 keys against two patterns on the arena's first
    chunk.
  - (the review of 3.8.11) `_sch_check_value_a`, `compiled_check_a` and `dispatcher_dispatch` handed the heap keep
    only their answers, each violation copied out whole.
  - 20,000 empty strings against a 3,000-step alternation refused in time.
  - `multipleOf` past 2^53 and past a double's range warned of, 2^53 itself not, and thirteen sizes at and either
    side of each bound.
  - Exponents of seven digits and of 15 compared, judged by `maximum` and `multipleOf`.
  - `uniqueItems` over 1,000 integers through `compiled_check_a` and `_sch_check_value_a` handed the heap, keeping
    none of the walk's table.
- **`tests/bote_schema_walk.tcyr`**, 125, from 124: `additionalProperties` beside `patternProperties` judged, and
  beside a declined pattern not; no WARN for either.
- **`tests/bote.tcyr`**: its example of a keyword bote does not judge is `unevaluatedProperties`.
- **`tests/bote_schema_comp.tcyr`**: the budget's two-second bound, the host's figure, is a ratio to an `anyOf` of
  two over the same items, timed in the same process. `qemu-aarch64` met two seconds at 1.98 s for 3.8.11 and missed
  it at 2.05 s here. The ratio is 13.5 on x86 and 12 on aarch64, where judging all 12.9 million would be about 50.
- **`tests/bote.bcyr`**: `validate_pattern_props`, a call's arguments judged by `patternProperties` beside
  `properties` and `additionalProperties` false.
- **14 mutants of `patternProperties`, 14 killed.**
- **55 mutants of the review's fixes, 55 killed.** Two survived the first run, each a gap now tested:
  - `compiled_check_a` handed the heap without the scratch passed, as the engine's own fix left the schema under
    test nothing else to keep: a `uniqueItems` row, whose table it kept, 162 KB for five calls;
  - the exponent bound restored passed, which showed the release's own first claim wrong. Six digits were always read
    (`1e200000` was never `1e150000`); seven or more were read as six. The rows compare seven digits.
- **`fuzz/schema_walk.fcyr`**: `patternProperties` of `^a`, `b$` and `x`, with `additionalProperties` beside, the
  reference judging each matched member and leaving it out of the additional ones. It found a generator that drew
  one pattern twice, an object bayan refuses, before release.

### Performance

3.8.11 against this release, 15 alternating rounds on one core, minimums (`ab`; `validate_pattern_props` is new and
runs here only). Every other row is within ±3.4%:

| Bench | 3.8.11 | 3.8.12 | Δ |
|-------|-------:|-------:|--:|
| `regex_slug` | 2,473 ns | 2,327 ns | −5.9% |
| `validate_compiled_text` | 2,899 ns | 2,862 ns | −1.3% |
| `validate_pattern_props` | — | 1,557 ns | new |
| `schema_compile_simple` | 15,838 ns | 16,754 ns | +5.8% |
| `schema_compile_nested` | 35,235 ns | 36,703 ns | +4.2% |
| `fs_read_64k` | 3.77 ms | 3.98 ms | +5.4% |
| `jwt_cached_es256` | 2,022 ns | 2,589 ns | +28.0% |
| `dispatch_tools_call_structured` | 4,558 ns | 4,123 ns | −9.5% |
| `dispatch_tools_call_output` | 6,512 ns | 6,225 ns | −4.4% |

- **A 4,000-member class against 20,000 code points is 1.5 ms, from 162 ms**, now that classes are found by halving
  (the Fixed entry).
- **The engine's per-match cost is where it was.** A sparse set of steps seen, which needs no clearing, was tried
  for the making-ready fix. It took `regex_slug` from 2.44 to 2.96 µs in the first A/B, a fifth more, and was
  replaced by the cleared marks charged an eighth of a step each, which run it 5.9% faster than 3.8.11.
- **Registration compiles an object about a microsecond slower** (`schema_compile_*`). It looks for
  `patternProperties` in each object it compiles. That is paid once per registration, never per call.
- **`fs_read_64k`'s 5.4% is where the code now sits, not work.** Its path changed by one load a member. Alone, it is
  3.76 ms against 3.96 ms. 3.8.11 with only 3.8.12's `src/regex.cyr` swapped in is 3.91 ms, and that path never calls
  the engine. The structured rows' −9.5% and −4.4%, on the same path in both runs, read the same way.
- **`jwt_cached_es256`'s 28% is one fast baseline round** (2,022 ns, its range 2,022–2,856). Nothing on the JWT path
  changed. The first A/B of this release had it at −1.0% (2,467 against 2,442 ns).

## [3.8.11] — 2026-11-04 · Patterns and multiples

> ⚠ **Corrected at 3.8.12.** These claims below were not true, found by an independent review of this release and by
> the differentials 3.8.12 added or repaired.
> - "60,000 random patterns ... every one of 280,920 verdicts the same". The generator multiplied its seed as a double
>   past 2^53, and the sequence repeated: the 60,000 were 307 distinct patterns over 1,453 distinct strings. Drawn
>   properly, as 48,781 distinct patterns, they found eight that JavaScript refuses taken here: two groups of one
>   name that can both match. They also found four verdicts that differ, each V8 departing from ECMA-262, where bote
>   matches the spec's search (see 3.8.12).
> - "a string costs its length times the program's length at most" and "each step is charged". Not all of the work
>   was charged:
>   - each match cleared an array as long as the program, and charged none of it;
>   - the first turn's steps were charged only when a second turn followed;
>   - so 20,000 empty strings against a 3,000-step alternation cleared 60 million words and took 40 million steps,
>     none of it charged;
>   - a class read its members one by one for each code point, uncharged: all 4,000 of a 4,000-member class.
> - "A pattern compiles once, at registration". It compiled each copy a quantifier makes, of nothing too:
>   `(((a{0}){10000}){10000}){10000}` never finished registering.
> - "Its four arrays come from `a` and are given back, so nothing stays on the heap". On the heap allocator, which a
>   dispatcher's plain form (`dispatcher_dispatch`) judges on, a release does nothing: `^.{0,2000}$` kept 128 KB a
>   string, 12.8 MB for a call of 100. Every table and quiet judgement a walk made through that form was kept the same
>   way, since 3.8.8 moved only `compiled_validate` and `registry_validate_params` to the scratch.
> - "registration's reference check as Ajv 8 reads it". Its resolver read three spellings otherwise than the
>   client's (fast-uri) does:
>   - a last `..` dropped its slash;
>   - a query's slashes were read as path segments;
>   - a userinfo's case was folded.
>
>   It also followed an `if` beside no `then` or `else` that can fail, which Ajv compiles none of, and took an `$id`
>   the client's resolver cannot read: the client then compiles nothing of the schema.
> - "`not`, `if` and `contains` pass it by, as they type nothing". `not` of `not` an integer is an integer, and an
>   `if` can decide by one: `{"not":{"not":{"type":"integer"}}}` took `1e30` for a handler. (`contains` still counts
>   a match that takes it; see 3.8.12.)
> - Exactness past an exponent of 100,000, older than this release (3.8.2): `_sch_num_norm` stopped reading an
>   exponent there, so one of seven digits or more was read as its first six, and `1e1000000` met
>   `maximum: 1e100000`.

**`pattern` is judged**, by an engine of bote's own (`src/regex.cyr`, a core module). It reads a pattern as the
official client's Ajv compiles it, `new RegExp(pattern, "u")`. It matches code points, so `^.{3}$` takes
"été", and it takes the `u` flag's stricter syntax: an escape of a letter that means nothing, a lone `{`, `}` or
`]`, or a quantifier on an assertion is no pattern. A pattern compiles once, at registration, to a program for a
Thompson machine. The machine follows every way the pattern can match at once, one code point at a time, so a
string costs its length times the program's length at most. No pattern can be made to backtrack: `(a*)*b`
against 30,000 `a`s takes a few times what `z` does. Each step is charged to the judgement's budget.
Backreferences, lookahead and lookbehind, `\p{...}`, inline modifiers and programs past 4,096 steps are not taken.
Such a pattern judges nothing, and registration says why.

Against JavaScript itself (`scripts/regex-diff.sh`, which runs node; not CI), 60,000 random patterns over every
construct the engine parses or declines, each with ten strings:
- every one of 280,920 verdicts the same;
- no pattern JavaScript refuses taken;
- none it takes refused as a syntax error;
- 1,013 declined for a construct the engine does not take.

**Seven defects in 3.8.10 land beside it**, from an independent review of that release, which drew random schemas
with references against the official client's Ajv. The worst two are denials of service that `$ref` opened. One call
of 60 KB took 21 s, as a walk's bytes went uncharged. A 40 KB call kept 204 MB, as long keys were copied for every
judgement. See the correction notice in 3.8.10.

**`multipleOf` is judged, exactly**: a value is a multiple when dividing it by `multipleOf` leaves an integer, read
as decimals of any length. `19.99` is a multiple of `0.01` here. The official client divides as doubles, and finds
`19.99` no multiple of `0.01` (`1998.9999999999998`), so registration warns of a `multipleOf` that is no integer.
`multipleOf: 0` refuses every number, as the client's does; a negative one is judged by its size, as the client
judges it.

The stated minimum cyrius (6.7.13) is unchanged.

### Added

- **`src/regex.cyr`** (core, after `jsonx.cyr` in both profiles: the core bundle has 14 modules, the full one 40):
  - `bote_rx_compile(src)`, `bote_rx_why(rx)` (0, or why the engine does not take it), `bote_rx_count(rx)`.
  - `bote_rx_test_a(a, rx, buf, s, e, bud)`: 1 when the pattern matches somewhere in the JSON string body
    `[s, e)`, 0 when it does not, -1 when the budget `bud` ran out first. Its four arrays come from `a` and are
    given back, so nothing stays on the heap.
  - The string is read as `_sv_strlen` reads it: escapes decoded, a surrogate pair one code point, a lone
    surrogate itself, and bytes that are no UTF-8 a U+FFFD for each maximal part.
  - Supported: classes (ranges, `\d \w \s` and their complements, `[^]`), `.` (no line terminator), `^ $ \b \B`,
    groups (capturing, `(?:`, `(?<name>`), alternation, and every quantifier, lazy ones matching what greedy
    ones do.
  - lib/regex.cyr is not used: it matches bytes, keeps its compile and match state in globals that worker threads
    would share, and allocates on the heap for every match. Filed in cyrius.
- **`pattern` and `multipleOf`** (`src/schema.cyr`): PropertyDef is 296 bytes, +280 the compiled pattern and +288
  `multipleOf` (`_sch_multiple_of`: its significant digits as an integer, at most 17, and the power of ten they
  are times). `_sch_is_multiple` divides exactly: D × 10^p is a multiple of M × 10^q when M divides
  D × 10^(p−q), never when p < q, and a power of ten past 64 divides as 64 does. A violation reads `does not
  match the pattern` or `not a multiple of multipleOf`.
- **`scripts/regex-diff.sh`**, with `tests/fixtures/regex_diff_gen.mjs` and `tests/fixtures/regex_diff.cyr`: the
  differential against JavaScript. CI builds the comparison program.

### Fixed

- **CI's test step reports a file that exits non-zero.** It captured each file's output with `out=$(...); rc=$?`
  under `bash -e`, so a failing file ended the step at the assignment, before its output, its FAIL line or any
  later file: the log showed the file's name and `exit code 1`. It is `rc=0; out=$(...) || rc=$?`.

From the review of 3.8.10:

- **A walk's bytes were uncharged** (HIGH). `_sv_object` and `_sv_array` charge the budget one for every four bytes
  their walk reads (`BOTE_SCH_WALK_BYTES`), and stop once their slot is full. That is what a quiet judgement's
  first miss does. The 60 KB recursive union is refused as too much to judge, its time growing with its size, as
  is the same of arrays.
- **A long key's copy was kept for every judgement** (HIGH). It is given back once the key is looked up, and its
  path is written from the key as written. Twenty levels of 2,000-byte keys keep nothing on the heap, and take under
  a megabyte of the arena.
- **Registration read references otherwise than the official client** (MEDIUM). It reads them as Ajv 8 does:
  - anchors and `$id`s from what json-schema-traverse reaches (`_sch_ajv_nodes`): nothing on the root, no array
    but under `items` and the composition keywords, and any other key's object but those it skips;
  - one name twice refused, even on one schema;
  - only the references the dialect's engine compiles, reached from the root (`_sch_reach`, `_sch_kids`): 2020-12
    without `additionalItems`, draft-07 without `prefixItems`, `dependentSchemas` or the `unevaluated*` keywords,
    `then` and `else` only beside `if`, `dependencies` in each;
  - a reference resolved against the root's `$id` as a URI is resolved (relative, absolute-path, dot segments, the
    scheme and host in any case), and `""` as the root;
  - a reference to an anchor on the root refused with a word on how to write it: `#`.
- **An argument's integer past a handler's range was refused on sight** (LOW). A quiet judgement reports whether it
  took one as an integer (`_sv_matches` 2). `anyOf` refuses it only when every schema taking the value took it so,
  and `oneOf` when its one match did. `not`, `if` and `contains` pass it by, as they type nothing.
- **An inputSchema root's `const` and `enum` judged nothing** (LOW, older than 3.8.10). Each is judged
  (`_sch_fill_eq`), so `{"type":"object","const":{"a":1}}` refuses `{"a":2}`.
- **A batch's results drew on its arguments' base** (LOW). They share a base of their own
  (`BOTE_REQ_STATE_POOL_OUT`, the request state's +176, 184 bytes).
- **A violation's name** (LOW): the arguments' own is pushed bare, and nothing is stripped from another's.
  A member named `": x"` reads `: x: expected string`.

### Changed

- Registration no longer warns of `pattern` or `multipleOf` as unenforced. It warns of:
  - a pattern the engine does not take, and why (`pattern bote's engine does not take, for a lookbehind`);
  - a pattern that is no string;
  - a `multipleOf` that is no number, 0, below 0, of more than 17 significant digits, or no integer.

### Tests

- **`tests/bote_regex.tcyr`**, 100, new:
  - Each construct declined, and each the `u` flag refuses, by its reason.
  - Eleven kinds of pattern taken.
  - Matching as JavaScript matches: anywhere unless anchored; the empty string; code points, astral and lone
    surrogates; `.` and the four line terminators; `\s` as ECMA-262's white space; `\w` and `\d` as ASCII's;
    word boundaries at the ends; ranges past ASCII and astral; `[\b]`; `\u{...}`, pairs, `\x`, `\c`, `\/`, `\0`;
    JSON escapes in the value; every quantifier, lazy ones; loops that may match nothing; `^` in a group;
    bytes that are no UTF-8.
  - Four patterns that backtrack exponentially elsewhere, linear here.
  - The budget stopping a match, and charging each step.
  - 0 heap bytes for 1,000 matches; the arena given back, a 4,001-step program's arrays too.
- **`tests/bote_schema_pattern.tcyr`**, 79, new:
  - `pattern` on strings only, decoded as JSON, anywhere, by code points; one declined, one no client
    compiles and one that is no string, each judging nothing; beside the lengths; under `anyOf` and `not`; on
    `propertyNames`, decoded; through the dispatcher and a result.
  - `multipleOf` on 32 pairs, exactly, past 2^64, past a double's range, with exponents and fractions; zero,
    negative, 17 and 18 digits, no number; numbers only; beside the bounds; 19.99 for 0.01 through the dispatcher.
  - The budget: 256 patterns over 60 KB, and a 200,000-digit number under 256 `multipleOf`s, each refused, in
    under 64 times one alternative's time.
  - The WARNs; 0 heap bytes for refused calls; the arena's first chunk; the heap form.
- **`tests/bote_schema_ref.tcyr`**, 169, from 120, the review of 3.8.10:
  - Registration as the client's Ajv reads it: an anchor on the root (three kinds), under `prefixItems`, one name
    twice two ways, under an unknown keyword and `contentSchema` but not `default`; references relative to the
    root's `$id` four ways and in another case, another document refused, `""`; `then` without `if`, draft-07's
    `dependentSchemas`, 2020-12's `dependencies`; keywords a dialect's engine does not compile; a `$ref` that is no
    string, where it is compiled and where it is not.
  - A recursive union of objects and one of arrays over 60 KB, each refused, its time four times its quarter's at
    most.
  - Twenty levels of 2,000-byte keys keeping nothing on the heap, and under a megabyte of the arena.
  - `1e30` under `anyOf` of integer and number, `contains`, `not`, `if` and `oneOf` taken; under `anyOf` of integer
    and null, and a `oneOf` met only by the integer, refused.
  - A root's `const` and `enum`; a batch's valid result after a call whose arguments spent their base; violation
    names.
  - Five expectations changed with the reading: a definition nothing names compiles nothing.
- **`tests/bote_listen_line.tcyr`**, 193, from 192: the last of four threads' resource updates is posted once the
  stdio thread has taken the queue. 200 URIs overflow a 16-entry queue whenever that thread falls behind, and an
  update posted into an overflowed queue is told as the URI the subscription holds, `res://t/`, as designed: more
  than happened, never less. Posted at once, it failed on a two-core runner, in 2 of 6 runs on 3.8.10 too. It
  passes 10 of 10 on two cores and 5 of 5 on one.
- **`tests/bote.tcyr`**, **`tests/bote_output.tcyr`**, **`tests/bote_schema_parts.tcyr`** (78, from 77): their
  examples of a keyword bote does not judge are `patternProperties`, `format` and `unevaluatedProperties` now.
  `propertyNames` with a `pattern` is judged.
- **45 mutants, 45 killed**, in the engine (each op of the machine, each assertion, the closure's split, the loops a
  quantifier compiles to, the code point decoder, the class escapes, each refusal of the `u` flag's syntax, the
  bounds, the budget and the arena), the schema's two keywords (the exponent loop and the 17-digit bound among
  them, each found untested while the list was written and tested before the run) and the WARNs.
- **31 mutants of the review's fixes, 31 killed.** Two survived the first run, each a gap now tested:
  - Dropping the array walk's charge: the test's arrays met `contains` with `const`, which charges each item's bytes
    and bounded the work by itself. Its arrays now fail `maxItems` and `minItems`, which read nothing. Without the
    charge, 60 KB takes 36 s.
  - Dropping a long key's release: the walk's charge now bounds how often an object is judged, so 2,000-byte keys
    showed nothing. Ten levels of 6,000-byte keys and a 256 KB arena bound do: 128 KB with the release, 320 KB
    without.
- **`fuzz/regex.fcyr`**, new: random pattern trees written in every spelling the `u` flag takes, random strings
  raw and escaped, each judged by the tree by a reference that shares none of the engine. 77,065 matches and
  82,935 misses a run.
- **`fuzz/schema_walk.fcyr`**: `multipleOf` of 2, 3 or 0.5, and five patterns, each judged by hand.

### Performance

**A call costs what it did; a pattern costs its match.** Interleaved A/B against 3.8.10, 15 rounds pinned to one
core, the working tree's benches on both sides (the two new ones on this side only). Minimums, in ns:

| Bench | 3.8.10 | 3.8.11 | Delta |
|---|---:|---:|---:|
| `codec_process_message` | 4,344 | 4,385 | +0.9% |
| `dispatch_tools_call` | 2,661 | 2,700 | +1.5% |
| `dispatch_tools_call_result` | 2,380 | 2,401 | +0.9% |
| `dispatch_tools_call_structured` | 3,959 | 4,499 | +13.6% |
| `dispatch_tools_call_output` | 5,908 | 6,412 | +8.5% |
| `validate_compiled_simple` | 562 | 562 | +0.0% |
| `validate_compiled_nested` | 1,047 | 1,113 | +6.3% |
| `libro_export_100` | 1,839,000 | 1,846,000 | +0.4% |
| `fs_read_64k` | 3,724,000 | 3,731,000 | +0.2% |
| `schema_compile_simple` | 14,412 | 15,634 | +8.5% |
| `schema_compile_nested` | 33,248 | 34,713 | +4.4% |
| `regex_slug` (new) | | 2,431 | |
| `validate_compiled_text` (new) | | 2,858 | |

- **The structured rows' 13.6% and 8.5% are where dispatch's code now sits, not work.** Nothing on that path changed,
  and it allocates the 216 bytes a call it did. Code that is never called, appended to `registry.cyr` in a copy of
  this tree, takes the structured call from 4,420 ns to between 3,930 and 3,970 at each of five sizes, where 3.8.10
  is 3,990 to 4,030.
- **`validate_compiled_nested`'s 6.3% is position too.** 3.8.10 with `src/regex.cyr` included and never called runs
  it at 1,146 ns, against 1,057 without and 3.8.11's 1,129. `jsonx_get_str_flat` and `codec_serialize_response`,
  which nothing touches, moved −6.5% and −6.3% the same way, and at 3.8.7 +12% and +7%.
- **A slug's pattern on 27 characters is 2.4 µs** (`regex_slug`): the Thompson machine reading 27 code points
  against a 15-step program, its arrays taken from the arena and given back.
- **Registration compiles a pattern and reads `multipleOf`**, about 1.1 µs more a property, paid once a
  definition, never for a call.

## [3.8.10] — 2026-11-04 · References

> ⚠ **Corrected at 3.8.11.** These claims below were not true, found by an independent review of this release.
> - "`uniqueItems` over 64 KB under 256 alternatives is refused as too much to judge ... each well under a second"
>   holds, but the budget still did not bound what a judgement reads. Each object's and array's walk read its members
>   uncharged, and `$ref` lets a small schema judge one object many times. A recursive union shaped as zod writes one,
>   over 60 KB of arguments, took 21 s for one call.
> - Nothing kept on the heap: a key of 256 bytes or more, in an object whose schema names more than eight
>   properties, was copied for every judgement of it and never given back. With `allOf` inheritance through `$ref`,
>   a 40 KB call kept 204 MB.
> - "Each is resolved at registration, as the official client's Ajv resolves it". Not as Ajv reads them:
>   - Taken here and not compilable by the client, which then cannot call the tool: an anchor on the root (Ajv
>     registers nothing there), one under `prefixItems` (its walk does not reach it), and one name used twice on one
>     schema.
>   - Refused here and compiled by the client: an anchor under an unknown keyword; a reference relative to the root's
>     `$id`, or with its scheme and host in another case; `$ref: ""`; `then` without `if`, and draft-07's
>     `dependentSchemas`, counted as cycles; and a reference in a definition nothing names, or under a keyword its
>     dialect's engine does not compile.
> - "A quiet judgement still takes it as JSON Schema does, and ... the root refuses the arguments". It marked the
>   arguments on sight of the integer. `1e30` under `anyOf: [{"type":"integer","maximum":100},{"type":"number"}]`,
>   `contains`, `not`, `oneOf` and `if` was refused where 3.8.9 and the client take it.
> - "A batch's judgements share one" base. A batch's results shared its arguments' base, so a valid result was
>   refused after another call's arguments spent it.
> - "A violation of the arguments as a whole is named once". It was named by stripping `": "` from every violation,
>   so a member named `""` read as the arguments, and one named `": x"` read as `x`.
> - The stack the depth takes: `dependentSchemas` is the deepest shape, at 375 KB, not `anyOf`.

**`$ref` is judged, within its document**: `#`, a JSON Pointer into `$defs` or anywhere else (`#/$defs/node`,
`#/properties/a`, `#/allOf/0`), an anchor (`#node`, from `$anchor`, `$dynamicAnchor` or draft-07's `$id: "#node"`),
and any of these behind the root's own `$id`. Each is resolved at registration, as the official client's Ajv resolves
it, and compiled once, so a schema that refers to itself, a tree or a linked list, compiles to a cycle and judges a
value as deep as the value goes. A reference is judged beside the keywords next to it in every dialect. draft-07
ignores those keywords, and the official client's draft-07 engine judges them, as bote does; registration warns
when a draft-07 or draft-06 schema has any. The conformance suite's schema has a `"$ref": "#/$defs/address"`, and
its address is judged now. Through 3.8.9 every `$ref` was warned of and judged nothing.

**Registration refuses a reference it cannot judge**:
- one that names nothing in its document, or names no schema there;
- one into another document, which is the operator's to inline;
- a `$ref` that is no string;
- an anchor that is no anchor name, or one name used for two schemas;
- a schema below the root that names a resource of its own with `$id`, against which a reference would resolve
  otherwise;
- a reference that leads back to itself without judging any part of the value, such as `{"$ref":"#"}` or
  `{"anyOf":[{"$ref":"#"}]}`. That would judge a value forever, here and in the client.

A judgement also has a depth now, 512 steps (a value's nesting and the references between). A chain of 5,000
references, which registration takes, is refused as `nested too deep to judge`, where it would have overflowed a
worker's stack.

**Six defects in 3.8.9 land beside it**, from an independent review of that release, which drew random schemas and
values against the official client's validator. The worst: an inputSchema root's composition was never judged
unless the root held a property or another object keyword, so a discriminated union at the root took any member of
the wrong type. See the correction notice in 3.8.9.

The stated minimum cyrius (6.7.13) is unchanged.

### Added

- **`$ref`** (core, `src/schema.cyr`): PropertyDef is 280 bytes, +272 the schema a reference names, judged as an
  `allOf` of one, first. A compile carries a context (`_sch_cx_new`, 64 bytes) through every subschema: the dialect,
  the document's text, a map from each target's canonical JSON Pointer to its PropertyDef (`_sch_ref_ptr`: each part
  percent-decoded after the split, as Ajv splits, then `~1` and `~0`; `#` and `#/` the root), the anchors, read when
  first named (`_sch_cx_anchors`), and the targets still to compile. A target is compiled once the schema naming it
  is (`_sch_cx_drain`), not where it is named: a chain of 5,000 references recursed once a link, and overflowed a
  worker's stack at registration.
- **A ToolSchema's references** resolve in the inputSchema tools/list sends: `{"$ref":"#/properties/b"}` judges as
  `b` does, and `#` is the listed root. The listed document is written out only for a property that may hold a
  reference.
- **`_sch_refs_why(doc)`**, registration's check, for an outputSchema, an inputSchema given as JSON and a ToolSchema's
  listed form. It walks every schema in the document as json-schema-traverse walks one (`_sch_positions`), with a
  stack of its own. It searches the keywords that judge the value itself (`$ref`, `allOf`, `anyOf`, `oneOf`, `not`,
  `if`, `then`, `else`, `dependentSchemas`, draft-07's `dependencies`) from every reference's target, iteratively,
  for a cycle (`_sch_ref_cycle`). A reference to a broken one nothing names is refused too.
- **The depth** (`BOTE_SCH_MAX_DEPTH`, 512): the budget block is 48 bytes, +8 the depth and +16 whether it was
  passed. Passing it stops the walk, and the root refuses the value as `nested too deep to judge`. 512 is eight steps
  for each level of a value the codec admits. Judging that deep was measured at between 256 KB and 512 KB of stack
  for the deepest shape, a chain of references through `anyOf`, against a worker's 2 MB.

### Changed

- Registration no longer warns of `$ref` as unenforced. It warns of a keyword beside a `$ref` in draft-07 or draft-06,
  `keywords beside $ref, which this schema's dialect ignores and bote judges`.
- A violation of the arguments as a whole is named once: `for tool 't': matches no schema in oneOf`, where it read
  `': : '`.

### Fixed

From the review of 3.8.9:

- **An inputSchema root's composition was never judged when the root held nothing else for the object walk** (HIGH).
  `_sv_object` returned before it with no property, required name, count, `propertyNames` or `dependentSchemas` to
  judge. A root `oneOf` of two `required` lists ran its handler for `{}`, and `{"allOf":[false],"type":"object"}`
  took anything.
- **The budget counted values, not what a keyword reads of one** (MEDIUM). A keyword that reads a value's bytes
  (`uniqueItems`, `minLength` and `maxLength`, `enum` for each value it compares, `const`, an integer's type, a type
  array's, the numeric bounds) spends one more for each byte it reads (`_sv_charge`). `uniqueItems` over 64 KB under
  256 alternatives is refused as too much to judge, and so is a megabyte string under 256 `maxLength` alternatives,
  each well under a second.
- **Each request of a batch had a budget base of its own** (MEDIUM). A batch's judgements share one
  (`BOTE_REQ_STATE_POOL`, the request state's +168, 176 bytes; `_sch_pool_open` around a batch's requests), and each
  root keeps its own bytes' allowance. Of 80 calls that each spend most of a base, the first is met and the rest are
  refused as too much to judge, in well under 400 ms.
- **An argument's integer past what a handler holds reached it through `anyOf` or `oneOf`** (MEDIUM). A quiet
  judgement still takes it as JSON Schema does, and under a root that judges arguments it marks the block, and the
  root refuses the arguments as `integer out of range` (`_sv_int_ok`). A result's integer is a client's to read
  whole, as before.
- **`if`, `then` and `else` judged nothing in draft-06** (LOW). The official client judges a draft-06 schema with its
  draft-07 engine, which has them, and bote judges them as it does. Registration says so, `if, which draft-06 does
  not have and bote judges, as the official client does`.
- **`anyOf: []` and `oneOf: []` judged nothing** (LOW). Each refuses every value, as the client's does, and
  `allOf: []` takes every value. Registration still warns of each.

Not fixed, and stated here: **whether a value is refused as too much to judge can turn on the order of its
members**. A quiet judgement stops at its first violation, in member order, so one spelling may stop early and
another spend the budget. That follows from having a budget at all, and only a value refused for its work is
affected.

### Tests

- **`tests/bote_schema_ref.tcyr`**, 120, new:
  - Resolution and judging: pointers into `$defs` and `definitions`, one definition compiled once whatever its
    spelling, the root and `#/`, a tree, two definitions in turn, sixty levels; `~0`, `~1`, `%2F` within a name,
    `%25`, an array's item, another property, `allOf`'s first, a reference written with JSON escapes and under an
    escaped key, `false` and `true`, an index with a leading zero (not the second item); `$anchor`, `$dynamicAnchor`, draft-07's `$id`
    fragment, one schema by anchor and pointer; the root's `$id` with and without a pointer, and with an empty
    fragment; siblings in 2020-12 and draft-07; references under `anyOf`, `not` and `contains`; the conformance
    suite's address.
  - Registration: every refusal, a broken reference nothing names among them, a percent sign with one hex digit or cut
    short, an `$anchor` that is no string ignored as Ajv ignores it and one that is no text refused; twelve cycles (through each keyword
    that judges the value itself, an anchor, an inputSchema's root, the 3,000th of a chain), and five shapes that are
    no cycle.
  - A ToolSchema's references in the listed inputSchema, an escaped one refused, and `#`; a JSON root's reference
    judging the arguments; a result through a definition on both revisions.
  - The depth: a cycle compiled outside registration refused, a branching one in under two seconds, chains of 200,
    600 and 5,000 on a worker's stack, through `anyOf` too, a value as deep as allowed and one deeper, an inputSchema
    of 600 through the dispatcher and of 100 met.
  - The WARNs; 0 heap bytes for fifty refused calls sixty levels deep; the arena's first chunk for 5,000 quiet
    judgements through references.
- **`tests/bote_schema_comp.tcyr`**, 135, from 89, the review of 3.8.9:
  - A root's composition alone, the reviewer's shapes, through the dispatcher, `registry_validate_params` and
    `compiled_validate`.
  - What a keyword reads, under 256 alternatives each refused, and in under 64 times one alternative's time measured
    in the same process, so the bound holds under emulation as on the host: `uniqueItems` over 64 KB, a megabyte string
    for `maxLength`, `const` and `enum`, an array for an `enum` of arrays and objects, a 200,000-digit number for an
    integer, a type array, the bounds and an `enum` of numbers.
  - A batch of 80 sharing one base, in under 40 times one call's time; the next call with its own; the pool closed.
    Wall-clock bounds in the first draft failed under `qemu-aarch64` while every verdict held; each bound is a
    ratio now, and a mutant of each fix fails it.
  - `1e30`, twenty nines and `-1e30` through `anyOf` and `oneOf` refused as out of range, and a result's taken; empty
    `anyOf`, `oneOf` and `allOf`; draft-06's `if` judged.
- **`tests/bote_output.tcyr`**, 264, from 256: the 2025-11-25 form's conversion of eight schemas registration now
  refuses, each judged on the converter alone and each refusal asserted; the long list's schema resolves, with no
  cycle.
- **`tests/bote_tool_schema.tcyr`**: a property's `$ref` warned of no longer.
- **`fuzz/schema_walk.fcyr`**: up to three definitions and references to them, written plainly, percent-encoded and
  under an escaped key, the reference judging a `$ref` by its definition. It found a generator that wrote no comma
  after a reference, before release.
- **67 mutants, 67 killed.** Four survived the first run, each a gap now tested: a percent sign with one good hex
  digit; an index with a leading zero, which the test's one-item array could not tell from a missing one; and the
  charges for an integer's type and a type array's, which the bound beside them in the test charged anyway. The last
  two showed an `enum` of numbers reading a long number's digits once an entry, uncharged; it is charged.

### Performance

**A call costs about what it did; the heap form of validation pays a few nanoseconds for the depth and the
charges.** Interleaved A/B against 3.8.9, 15 rounds pinned to one core, the working tree's benches on both sides.
Minimums, in ns:

| Bench | 3.8.9 | 3.8.10 | Delta |
|---|---:|---:|---:|
| `codec_process_message` | 4,336 | 4,363 | +0.6% |
| `dispatch_tools_call` | 2,690 | 2,650 | −1.5% |
| `dispatch_tools_call_result` | 2,306 | 2,371 | +2.8% |
| `dispatch_tools_call_output` | 5,772 | 5,892 | +2.1% |
| `validate_compiled_simple` | 502 | 561 | +11.8% |
| `validate_compiled_nested` | 1,015 | 1,047 | +3.2% |
| `libro_export_100` | 1,820,000 | 1,836,000 | +0.9% |
| `fs_read_64k` | 3,710,000 | 3,709,000 | −0.0% |
| `schema_compile_simple` | 13,984 | 14,404 | +3.0% |
| `schema_compile_nested` | 32,214 | 33,184 | +3.0% |

- **Half of `validate_compiled_simple`'s 59 ns is where the code now sits.** A copy of this tree with the depth, the
  charges and the shared base taken out of the walk ran it at 538–545 ns. Against that, the work costs about 30 ns
  on this one-integer object through `compiled_validate`, and 0.1–0.2% on the dispatcher's result rows.
  `jsonx_get_str_flat`, which nothing here touches, moved +10.4% the same way, as it did at 3.8.7.
- **The first cut cost more:** a call to `bote_req_state()`, two to `_sv_charge`, one to `_sv_int_ok` for every
  integer, and four bound accessors read twice. With each made only where it is needed, an integer in range costs a
  load and a store. That brought the cost against the control from 33 ns to 30.
- **Registration reads `$ref` once a subschema and makes a compile context**, 3% of a compile: paid once a
  definition, never for a call.

## [3.8.9] — 2026-11-04 · Composition

> ⚠ **Corrected at 3.8.10.** These claims below were not true, found by an independent review of this release.
> - "An inputSchema root's composition ... judged after the members." Only when the root held something else for the
>   object walk to judge: a property, a required name, a count, `propertyNames` or `dependentSchemas`. Without one,
>   the walk returned before the composition, so `{"type":"object","oneOf":[{"required":["a"]},{"required":["b"]}]}`
>   ran its handler for `{}`, and a discriminated union at the root took a member of the wrong type. 1,071 of 8,000
>   random root schemas were judged otherwise than by the official client, every one of them taken.
> - "the budget bounds what a large schema can be made to cost by a client's many items." It counted values, not
>   what a keyword reads of one: `uniqueItems` over a 63 KB argument under 256 alternatives took 977 ms and was not
>   refused, and a megabyte string under 256 `maxLength` alternatives 751 ms. Each request of a batch had its own
>   base, so 80 calls in 69 KB took 521 ms.
> - "a keyword its dialect does not have judges nothing, as it is no keyword to a client, which picks its validator
>   by `$schema`". The official client judges a draft-06 schema with its draft-07 engine, which has `if`, `then` and
>   `else`, so a draft-06 result it refused passed.
> - "an `allOf`, `anyOf` or `oneOf` that is no non-empty array of schemas" judges nothing. The client refuses every
>   value under `anyOf: []` and `oneOf: []`, and such a result passed.
> - "where a schema judges a value directly, an integer past what a handler holds is still refused". True, and through
>   `anyOf: [{"type":"integer"},{"type":"null"}]`, the commonest optional integer, `1e30` reached the handler as one.
> - A violation of the arguments as a whole read `for tool 't': : matches no schema in oneOf`.

**`allOf`, `anyOf`, `oneOf`, `not` and `if` / `then` / `else` are judged**, on a value and on an inputSchema's root,
where alternatives for a tool's arguments are written. `allOf` judges a value as the schema itself does, its
violations kept; `anyOf`, `oneOf`, `not` and `if` ask quietly, as 3.8.8's `contains` does, so an alternative that
fails costs no message. The conformance suite's own 2020-12 schema, an `allOf` of an `anyOf` beside an `if`, is now
judged whole: `{"name":"x"}` is refused for naming neither a phone nor an email. Each was warned of at registration
through 3.8.8.

**A judgement has a budget**: 100,000 values and 32 for each byte of the document, every quiet judgement inside it
counted. A value judged once costs one; a schema whose alternatives each judge every item a client sends, 257 of them
for 50,000 items, stops at the budget and is refused as `more work to judge than its size allows`, in under a second,
where it would have judged 12.9 million values.

**Six defects in 3.8.8 land beside it**, from an independent review of that release. The worst: the keywords 3.8.8
judged were judged whatever `$schema` named, so a draft-07 result a client takes could be refused, and one it refuses
passed. See the correction notice in 3.8.8.

The stated minimum cyrius (6.7.13) is unchanged.

### Added

- **`allOf`, `anyOf`, `oneOf`, `not`, `if`, `then`, `else`** (core, `src/schema.cyr`, `_sv_comp`): a violation reads
  `matches no schema in anyOf`, `matches no schema in oneOf`, `matches more than one schema in oneOf`, `matches the
  schema in not`; `allOf` and the branch `if` chooses report their own. `then` and `else` with no `if` judge nothing.
  PropertyDef is 272 bytes (+208 to +256 the keywords, +264 whether it composes, so a value with none pays one load).
- **An inputSchema root's composition** (CompiledSchema +104, 112 bytes): compiled by `compiled_compile` from the
  JSON, judged after the members.
- **The budget** (`BOTE_SCH_BUDGET_BASE`, `BOTE_SCH_BUDGET_PER_BYTE`, `_sch_budget`): the violation slot is 48 bytes,
  +40 pointing at the root's budget, which every quiet judgement inside it shares. Once spent, no value is judged
  further, and the root records the refusal past the cap of 32 (`_violation_force`), so a value already full of
  violations says why it stopped too. Without `$ref` a schema is a tree, so the work is bounded by the schema's size
  times the value's; the budget bounds what a large schema can be made to cost by a client's many items.

### Changed

- Registration no longer warns of composition as unenforced. It warns of an `allOf`, `anyOf` or `oneOf` that is no
  non-empty array of schemas, a `not`, `if`, `then` or `else` that is no schema, and a `then` or `else` with no `if`.

### Fixed

From the review of 3.8.8:

- **A keyword was judged whatever its schema's dialect** (MEDIUM). The compile reads the dialect from a root's
  `$schema` (`_sch_dialect`: 2020-12 by default, 2019-09, draft-07, draft-06) and passes it to every subschema; a
  keyword its dialect does not have judges nothing, as it is no keyword to a client, which picks its validator by
  `$schema`: `prefixItems` is 2020-12's, `minContains`, `maxContains` and the `dependent*` keywords 2019-09's on,
  `if` / `then` / `else` draft-07's on. Registration warns of each, `prefixItems, which this schema's dialect does not
  have`.
- **A `dependentRequired` name that is no text was dropped**; it is kept, as one no member meets, so its member's
  presence is refused, as a client refuses it.
- **A quiet judgement judged an argument as an argument**, its integers held to what a handler can read, so a value
  past it counted as no match, and under `maxContains`, `not` or `oneOf` a value JSON Schema refuses passed. A quiet
  judgement now judges as JSON Schema does; where a schema judges a value directly, an integer past what a handler
  holds is still refused.
- **The taken flag counted a notification, and was cleared only by `bote_req_begin`.** It is set by a request with an
  id, and cleared by every codec call, so a legacy batch of a notification and a refused response is a 400, and
  `codec_unanswerable` through the heap form answers for the message just processed. It reads the state of the last
  message the codec processed on the calling thread, where at 3.8.7 it read its argument; the 3.8.8 entry did not say
  so.
- **The scratch lock was released with a plain store**, which an aarch64 core may let pass the critical section's
  rewind and copy. It is released by an atomic add, as bote's other locks are: a release on aarch64, and on x86 a
  locked add, where a fence before the store cost the heap forms about 110 ns a call (measured, and not shipped).

### Tests

- **`tests/bote_schema_comp.tcyr`**, 89, new: `allOf` met, missed, at its path; `anyOf` of types and of object
  schemas, under `items`; `oneOf` by each, by both, by neither; `not`, `{}` and `false`, of `not`; `if` with `then`
  and `else` each met and missed, without `if`, with no branch, `then` only; the conformance suite's schema through
  the dispatcher; an inputSchema root's `oneOf`; a result's `anyOf`; the budget refusing 50,000 items through 257
  alternatives in under two seconds, refusing an argument through the dispatcher, and staying far from three
  alternatives over 50,000 items or one type over 100,000, and, its globals set low, stopping the walk with the
  refusal last, alone, after the walk's own and past a full cap; from the review of 3.8.8, each keyword by its
  dialect (draft-07, 2019-09, draft-06, a property by its root's), a draft-07 result and arguments taken as a client
  takes them, a `dependentRequired` name that is no text, quiet judgements as JSON Schema judges (`[5,1e30]` under
  `maxContains`, 1e30 under `not`), and the taken flag through the heap form and a batch; the WARNs, the dialect's
  among them; 0 heap bytes; the arena's first chunk for 5,000 quiet judgements; the heap form keeping its answer.
- **`tests/bote_http_era.tcyr`**, 2000, from 1988: a legacy batch of a notification and a response, or of a
  notification and no object, a 400.
- **`tests/bote_threads.tcyr`**, 82, from 81: the heap forms' scratch on six threads, 1,800 judgements through
  `compiled_validate`, each answer its own and whole.
- **`tests/bote_tool_schema.tcyr`**: composition at the root warned of no longer.
- **`fuzz/schema_walk.fcyr`**: composition drawn, the reference judging it on bayan's trees.
- **31 mutants, 31 killed.** One survived the first run, a gap now tested: the budget's refusal held to the cap,
  which no test had filled before the budget was spent (`contains` asks of every item after the cap fills). The
  ordering of the scratch lock's release is no mutant a test on x86 can kill.

### Performance

**A call costs what it did; registration reads more keywords.** Interleaved A/B against 3.8.8, 15 rounds pinned to
one core, the working tree's benches on both sides. Minimums, in ns:

| Bench | 3.8.8 | 3.8.9 | Delta |
|---|---:|---:|---:|
| `codec_process_message` | 4,360 | 4,332 | −0.6% |
| `dispatch_tools_call` | 2,706 | 2,678 | −1.0% |
| `validate_compiled_simple` | 493 | 502 | +1.8% |
| `validate_compiled_nested` | 1,009 | 1,013 | +0.4% |
| `libro_export_100` | 1,827,000 | 1,819,000 | −0.4% |
| `fs_read_64k` | 3,733,000 | 3,706,000 | −0.7% |
| `schema_compile_simple` | 11,838 | 13,994 | +18.2% |
| `schema_compile_nested` | 27,549 | 32,175 | +16.8% |

- **The budget costs a value one decrement, and composition a schema without it one load.** Taken out of the walk
  in a copy of this tree, they moved the validate rows by less than a nanosecond.
- **A fence before the scratch lock's release cost the heap forms about 110 ns a call** (`validate_compiled_simple`,
  which calls `compiled_validate`, at 605 ns against 492 without it): an `mfence` on x86. The lock is released by an
  atomic add instead, ordered on aarch64 as bote's other locks are, and the rows are where 3.8.8 left them.
- **Registration reads seven keywords more and the dialect**, about 350 ns each: paid once a definition, never for a
  call.

## [3.8.8] — 2026-11-04 · The parts of a value

> ⚠ **Corrected at 3.8.9.** These claims below were not true, found by an independent review of this release.
> - "`prefixItems`, `contains` with `minContains` and `maxContains`, ... `dependentRequired` and `dependentSchemas` are
>   judged." Whatever `$schema` named: a draft-07 or 2019-09 schema has no `prefixItems`, and draft-07 none of the
>   others but `contains`, so a draft-07 result a client takes was a tool error, and `contains` with `minContains: 0`
>   passed an empty array the client refuses.
> - "`dependentRequired`: a member present requires the names it lists." A name that is no text was dropped, and its
>   requirement never asked.
> - "The slot judges as `slot` does". Inside `contains`, an argument's integer past what a handler holds counted as no
>   match, so under `maxContains: 1` the array `[5,1e30]` ran the handler.
> - "The codec now says as it goes whether it took a request or a notification". A notification counted as taken, so
>   a legacy batch of a notification and a response it refused was a 200; and only `bote_req_begin` cleared it, so
>   through the heap form `codec_process_message` every message after one request read as taken.
> - "taken under a lock of its own". The lock was released with a plain store, which on aarch64 may pass the arena's
>   rewind and the answer's copy.

**`prefixItems`, `contains` with `minContains` and `maxContains`, `propertyNames`, `dependentRequired` and
`dependentSchemas` are judged.** Each judges an array's items or an object's keys and members with no pattern and no
reference, and each was warned of at registration through 3.8.7. `contains` and `propertyNames` ask whether a value
meets a schema without keeping why it does not: a quiet judgement, which stops at its first violation and builds no
path and no message, so 5,000 items that do not meet `contains` cost no message each and leave nothing, even through
the heap forms.

**Four defects in 3.8.7 land beside them**, from an independent review of that release. The worst: a type array
holding a string that is no text crashed registration. See the correction notice in 3.8.7.

The stated minimum cyrius (6.7.13) is unchanged.

### Added

- **`prefixItems`** (core, `src/schema.cyr`): item i judged by the i-th schema, and `items` judging the items past
  them, as 2020-12 has it; `items: false` beside it allows no more. Draft-07's array `items` is still warned of and
  not judged.
- **`contains`**, with **`minContains`** (one by default; 0 takes an array with none) and **`maxContains`**. A
  violation reads `no item matches contains`, `fewer items than minContains match contains` or `more items than
  maxContains match contains`, and none of the items' own violations is kept.
- **`propertyNames`**: every key, each time it is written, as the string it decodes to. A violation reads
  `v.abcd: property name does not meet propertyNames`, at the member's path.
- **`dependentRequired`**: a member present requires the names it lists, `missing required field: billing_address,
  which credit_card requires`. A name it requires is no property, so `additionalProperties` judges it.
- **`dependentSchemas`**: a member present holds the whole object to its schema.
- **A quiet judgement** (`_sv_matches`): a violation slot is 40 bytes, +32 quiet; a quiet slot counts its first
  violation and makes nothing. PropertyDef is 208 bytes (+176 `prefixItems`, +184 `contains`, +192 and +200 the
  counts), CompiledSchema 104 (+80 `propertyNames`, +88 `dependentRequired`, +96 `dependentSchemas`), and each name
  the object keywords key on or require is watched, so its presence is its slot's bit.
- `prop_prefix_items`, `prop_contains`, `prop_min_contains`, `prop_max_contains`.

### Changed

- Registration no longer warns of the five as unenforced. It warns of a `prefixItems` that is no array, a `contains`
  or `propertyNames` that is no schema, a `minContains` or `maxContains` that is no non-negative integer, a
  `dependentRequired` that is no object of arrays of names and a `dependentSchemas` that is no object of schemas.

### Fixed

From the review of 3.8.7:

- **A type array holding a string that is no text crashed registration** (MEDIUM): `{"type":["\ud800"]}`, in an
  outputSchema, an inputSchema given as JSON or a ToolSchema property, decoded to 0 and went to `streq`. It is warned
  of as naming an unknown type, and judged by the names it knows.
- **The heap forms kept every table and copy `uniqueItems`, `const` and `enum` make** (MEDIUM), since a mark and a
  release do nothing on the heap: 32 KB a call for 1,000 integers judged unique, 64 KB for 1,000 escaped strings,
  1.5 KB for a `const` object, each for good, the item count the client's. `compiled_validate` and
  `registry_validate_params` now judge on a scratch arena of their own, taken under a lock of its own, and copy out
  only their answer; bote's own path judges on the request arena and never takes it.
- **The legacy 400 rule matched an answer's prefix** (LOW): the errors of a request whose id is `null`, which the
  codec takes, were 400s, and a batch of refused notifications, `[1]`, and a JSON-RPC response a client sends were
  200s. The codec now says as it goes whether it took a request or a notification from the message (the request
  state's +160, 168 bytes), and `codec_unanswerable` reads that: what it took gets 200 for its errors, and a message
  it took nothing from gets 400, a request refused at its admission among them, as on the modern path.
- The limit on numbers is worded to cover a value past a double's range, `1e400` or `1e-400`, as well as its
  precision.

### Tests

- **`tests/bote_schema_parts.tcyr`**, 77, new: `prefixItems` alone, with `items`, with `false` and `items: false`, one
  that is no array; `contains` met, missed, empty, `minContains` met and missed and 0, `maxContains`, both, neither
  without `contains`, object schemas, `false` and `true`, nested, beside `items`; `propertyNames` by a length and an
  enum, a key as it decodes, `false`, a keyword bote does not judge, nested, a 300-byte key; `dependentRequired` met,
  missed, decoded, several, nested, beside `additionalProperties: false`, twenty-four names past eight;
  `dependentSchemas` by `required`, a property's schema and `false`; a quiet judgement of 5,000 items keeping nothing
  on the arena or, through `compiled_validate`, the heap, a 70 KB string judged quietly within the arena's first chunk,
  5,000 judgements against 600 properties each giving back its table, and the heap forms' scratch rewound after 200
  calls of 32 violations; the dispatcher on arguments and a result; the WARNs; and 0 heap bytes for refused calls.
- **`tests/bote_schema_eq.tcyr`**, 92, from 83: three type arrays naming a string that is no text registering, warned
  of and judged by the names they know; `compiled_validate` and `registry_validate_params` keeping only their answer,
  and an answer that outlives the scratch's next use.
- **`tests/bote_http_era.tcyr`**, 1988, from 1938: the legacy status by what the codec took, on `http` and
  `streamable`: an id of `null` taken (its error and its result 200s), a batch of refused notifications, `[1]` and a
  JSON-RPC response 400s, a request refused at its admission a 400 with its id, a batch with a request taken a 200.
- **`fuzz/schema_walk.fcyr`**: the five drawn, the reference judging them on bayan's trees.
- **33 mutants, 32 killed, one equivalent**: the crash's guard beside `_reg_type_known`'s own. Four survived the first
  run, each a gap now tested: a quiet judgement that builds its messages or keeps what it took, and a scratch that is
  not rewound, which nothing had seen because what they built was given back or fitted in the first chunk.

### Performance

**A call costs what it did; registration reads seven keywords more.** Interleaved A/B against 3.8.7, 15 rounds pinned
to one core, the working tree's benches on both sides, with the machine's load at 3 to 4 throughout, so the rows are
wider than usual. Minimums, in ns:

| Bench | 3.8.7 | 3.8.8 | Delta |
|---|---:|---:|---:|
| `codec_process_message` | 4,499 | 4,402 | −2.2% |
| `dispatch_tools_call` | 2,752 | 2,732 | −0.7% |
| `dispatch_tools_call_output` | 5,874 | 5,887 | +0.2% |
| `libro_export_100` | 1,845,000 | 1,850,000 | +0.3% |
| `validate_compiled_simple` | 462 | 500 | +8.2% |
| `validate_compiled_nested` | 951 | 1,021 | +7.4% |
| `schema_compile_simple` | 9,486 | 12,093 | +27.5% |

- **The two validate rows are code position, not the new work.** Timed again interleaved they held at about +7%; with
  the new object checks taken out of the walk (`propertyNames`, both `dependent*`) in a copy of this tree, they held
  where they were, 502 against 499 ns. A bench this short moves 5 to 12% from where its code sits, and so do
  `jwt_cached_hs256` −13.8% and `session_validate` +6.3% here, which run nothing this release changes.
- **Registration looks for seven keywords more** (`prefixItems`, `contains`, `minContains`, `maxContains`,
  `propertyNames`, `dependentRequired`, `dependentSchemas`), about 350 ns each: paid once a definition, never for a
  call. Reading a property's keywords in one walk of its members, on the roadmap, would take this back.
- **A quiet judgement makes nothing**: 5,000 items that do not meet `contains` build no message and no path, and a
  70 KB string judged quietly stays within the arena's first chunk.

## [3.8.7] — 2026-11-04 · Values by value

> ⚠ **Corrected at 3.8.8.** These claims below were not true, found by an independent review of this release.
> - "It warns of a type array naming a type it does not know". A type array holding a string that is no text (a lone
>   surrogate, bytes that are no UTF-8) crashed registration with SIGSEGV: the name decoded to 0 and went to `streq`.
> - "so those forms keep only their answer up to 512 names". Through `compiled_validate` and `registry_validate_params`
>   every table, copy and decoding `uniqueItems`, `const` and `enum` make stayed on the heap: 32 KB a call for 1,000
>   integers judged unique, 64 KB for 1,000 escaped strings.
> - "now answer any input the codec could not take as a request, the answer an error with a null id, with 400 ... An
>   error for a request with an id stays a 200." The rule matched the answer's prefix: the errors of a request whose
>   id is `null`, which the codec takes, were 400s, and a batch of refused notifications, `[1]`, and a JSON-RPC
>   response a client sends were 200s.
> - "A client that holds numbers as doubles may find two equal that bote does not, past 15 significant digits or
>   2^53". Also a number past a double's range: `[1e400,2e400]` and `[1e-400,0]` are unique here and not to a client,
>   which reads them as Infinity and 0.

**`const`, `enum` of any value, `uniqueItems` and type arrays are judged, by JSON Schema's equality.** Two values are
equal when both are null, the same boolean, numbers of the same value (`1`, `1.0`, `1e0`), strings of the same
characters however they are escaped, arrays of equal items in order, or objects with the same keys and equal values,
in any order. `const` and `enum` judge every value whatever `type` says, where `enum` was judged only for a string
property and `const` not at all; `uniqueItems` finds two equal items through a seeded hash table, so an array costs a
look per item; a type array takes a value of any type it names. Each was warned of at registration through 3.8.6.

**Tool names are the spec's.** `dispatcher_register_tool` holds a name to 1 to 128 characters from `A-Z a-z 0-9 _ - .`,
the rule registration already held every definition to, where it also required an underscore and refused `-` and `.`
(bote's `project_tool` convention), so `t`, `get-weather` and `tool.v2` registered through the registry and were
refused, silently, through the dispatcher.

**Six defects in 3.8.6 land beside them**, from an independent review of that release. The worst: any client could
choose keys that made one message's key check quadratic, 1.02 s for a 121 KB message. See the correction notice in
3.8.6.

The stated minimum cyrius (6.7.13) is unchanged.

### Added

- **`const`** (core, `src/schema.cyr`): the value equals the one named. A violation reads `not the const value`.
- **`enum` of any value**: a string is compared with the enum's strings as it decodes, as before, and any other value
  with each value that is no string. A violation reads `value 'x' not in enum` for a string and `value not in enum`
  for another value.
- **`uniqueItems`**: each item hashed by its value (a number by its sign, exponent and significant digits, a string by
  its characters, an object by the sum of its members' hashes, so in any order) into a table on the arena, and an item
  compared only with one of the same hash. A violation names the first two found: `items 0 and 2 are equal, against
  uniqueItems`. The table and what the hashes read are given back before it.
- **Type arrays**: `"type": ["string", "null"]` takes a value of any type it names, and a violation names them in one
  order, `expected string or null`. An integer named without `number` is held to the range an argument may be, as
  `type: "integer"` is. One name is that type; none bote knows judges nothing.
- `prop_type_mask`, `prop_const_raw`, `prop_enum_raw`, `prop_unique_items`. A PropertyDef is 176 bytes.

### Changed

- **Strings compared as a client reads them, where finding two equal is the refusal.** For `uniqueItems` a string's
  bytes that are no UTF-8 are read as a client's decoder reads them, one U+FFFD for each maximal part, and a lone
  surrogate as U+FFFD, so two items a client finds equal are never two here. For `const` and `enum` a string that is no
  text equals nothing, as `enum` has had it since 3.8.3.
- **Numbers are compared exactly**, as decimals. A client that holds numbers as doubles may find two equal that bote
  does not, past 15 significant digits or 2^53; in a result's array judged by `uniqueItems` it would refuse what bote
  passed.
- **`validate_tool_name` is the spec's rule**, and `dispatcher_register_tool` leaves the check to registration, which
  logs why it refuses a name at ERROR; through 3.8.6 a name it refused got no line.
- Registration no longer warns of `const`, `uniqueItems`, a type array or an `enum` on a type other than string. It
  warns of a type array naming a type it does not know, an empty one, a `type` that is neither a string nor an array,
  an `enum` that is no array and a `uniqueItems` that is no boolean.

### Security

- **Keys a client chose could make the exact walker's key check quadratic** (from the review of 3.8.6, MEDIUM). Its
  table hashed keys with FNV-1a's own basis and took the index from the hash's low bits, which depend only on the
  basis's low bits: keys sharing them, found once offline, landed every key of an object in one bucket, 209 ms for a
  55 KB message against 1.3 ms for as many other keys, and 1.02 s for 121 KB, under every transport's cap and with no
  token. Every table bote keys by a client's bytes (`_jx_rep_keys`, `uniqueItems`, the object equality) now hashes with
  a seed made once per process from the clock (`_jx_hash_seed`, published by compare-and-swap) and a 64-bit finalizer
  that moves the low bits with every bit (`_jx_mix`). The reviewer's 5,000 keys now take what any 5,000 take.

### Fixed

From the review of 3.8.6:

- **A notification bote refused was a 200 on the legacy path.** `http` and `streamable` now answer any input the codec
  could not take as a request, the answer an error with a null id (`codec_unanswerable`), with 400, as Streamable HTTP
  requires, the error its body: a notification naming a key twice, one the codec refuses otherwise, invalid JSON, an
  id named twice. An error for a request with an id stays a 200. The bridge answers under its own contract, 200.
- **An id named twice was echoed when the message was malformed besides**: `{"jsonrpc":"2.0","id":7,"id":8}` was
  answered with id 7. It is answered with null.
- **The exact walker kept a vec for every object, empty ones too, to the end of its walk**, so one escaped key in
  54 KB of `[{},{},...]` grew the arena by 2.68 MB for the life of the thread. Each object's walk gives back what it
  took.
- **The table of seen members stayed on the heap through the heap forms**, `compiled_validate` and
  `registry_validate_params`, where a mark and a release do nothing: 18,000 objects against 80 properties kept
  1.58 MB a call. It is a bitset, 512 slots on the stack, so those forms keep only their answer up to 512 names, and
  an eighth of the table past them.
- **`BOTE_KEYS_INLINE` was a global a consumer could raise** past the sixteen keys the stack buffer it sizes holds; it
  is a constant.

### Tests

- **`tests/bote_schema_eq.tcyr`**, 83, new: `const` for numbers in every spelling, past 2^53 and past a double's
  exponent, strings raw and escaped, a surrogate pair, the empty string, strings that are no text, null, booleans,
  arrays in order and objects in any order, forty members through the table, nested; `enum` of each kind of value,
  untyped and on an integer; type arrays, each type, the message, integers and fractions, an object's and an array's
  keywords under one, one name, none, an unknown one; `uniqueItems` by value, by decoding, in any order, literals, two
  strings a client reads as one U+FFFD; 20,000 integers, 20,000 strings, 5,000 objects, 20,000 equal arrays and two
  objects of 2,000 members in under two seconds; the dispatcher on arguments and on a result; the WARNs; the seed and
  the finalizer; and 0 heap bytes for refused calls, 4,000 tags among them, and the arena given back.
- **`tests/bote_request_keys.tcyr`**, 92, from 83: the reviewer's 5,000 colliding keys
  (`tests/fixtures/fnv_low16_collisions.txt`) taking no longer than 5,000 others, both within a few syntax checks,
  and their repeat found; 18,000 empty objects behind an escaped key on less than 256 KB of arena; an id named twice
  in a malformed message.
- **`tests/bote_http_era.tcyr`**, 1938, from 1912: the legacy path's 400s on `http` and `streamable`, and a request's
  error kept a 200.
- **`tests/bote_schema_walk.tcyr`**, 124, from 121: `compiled_validate` keeping only its answer for 18,000 objects
  against 80 properties, and the bits naming the last slot.
- **`tests/bote.tcyr`**, 991, from 973: every kind of name the spec allows, 128 and 129 characters, what it refuses,
  three names registered through the dispatcher, listed and called, and the ERROR line for one it refuses.
- **`tests/bote_output.tcyr`**, 256, from 255: the WARNs of what is now judged.
- **`fuzz/schema_walk.fcyr`**: type arrays, `const`, `enum` of any values and `uniqueItems` drawn, the reference's
  equality its own on bayan's trees.
- **33 mutants, 32 killed, and one showed code that did nothing**: the number hash's trim of trailing zeros, which
  `_sch_num_norm` had already left out; it is gone. One survived the first run and showed a gap, now tested: a key
  hash of the first byte alone, which the flood test missed because its 5,000 other keys all began with one letter.
  That a seed is unpredictable is not something a test can show.

### Performance

**The new keywords cost a value a few loads where it has none of them, and registration about a microsecond more.**
Interleaved A/B against 3.8.6, 15 rounds pinned to one core, the working tree's benches on both sides. Minimums, in ns:

| Bench | 3.8.6 | 3.8.7 | Delta |
|---|---:|---:|---:|
| `validate_compiled_simple` | 429 | 455 | +6.1% |
| `validate_compiled_nested` | 907 | 939 | +3.5% |
| `codec_process_message` | 4,378 | 4,377 | −0.0% |
| `dispatch_tools_call` | 2,719 | 2,688 | −1.1% |
| `dispatch_tools_call_output` | 5,751 | 5,789 | +0.7% |
| `libro_export_100` | 1,805,000 | 1,818,000 | +0.7% |
| `schema_compile_simple` | 8,296 | 9,266 | +11.7% |
| `schema_compile_nested` | 19,161 | 21,873 | +14.2% |

- **A value judged pays for the keywords it does not have**: every value now asks whether its schema has a type
  array, a `const` or an `enum`, and a member's slot is a bit, where it was a byte. About 26 ns on a call with one
  property. Writing the bit arithmetic as shifts, where a division may be, moved it by less than two runs of one
  binary differ, and it was left as it was.
- **Registration reads three keywords more** (`const`, `uniqueItems`, the raw `type`), about 350 ns each: paid once a
  definition, never for a call.
- **`uniqueItems` is linear**: 20,000 distinct integers, 20,000 distinct strings, 5,000 objects, 20,000 equal arrays
  and two objects of 2,000 members are judged together in under two seconds in the test, where comparing every pair
  is 200 million comparisons for 20,000 items.
- **The flood is gone**: the reviewer's 5,000 colliding keys cost what any 5,000 keys cost, 1.29 ms against 1.28 ms
  for the check alone on 55 KB, and 4.5 ms for 121 KB through the codec, where 3.8.6 took 209 ms and 1.02 s.
- **The rest is code position.** `jsonx_get_str_flat` +12.3% and `codec_serialize_response` +7.3% run no code this
  release changes.

## [3.8.6] — 2026-11-04 · A key named twice

> ⚠ **Corrected at 3.8.7.** These claims below were not true, found by an independent review of this release.
> - "The check rides the syntax check the codec already made, so a message is still walked once", and its cost of
>   130 to 180 ns. An object of more than sixteen keys, or a key with an escape, sends a message to the exact walker,
>   whose table hashed keys with FNV's own basis and took the index from the low bits: keys chosen offline to share
>   them made the check quadratic, 209 ms for a 55 KB message and 1.02 s for 121 KB, from any client.
> - "a notification is a 400 where it was a 202". Only on the 2026-07-28 path: a legacy notification naming a key
>   twice got a 200 with the error on `http` and `streamable`, and the bridge answers 200 under its own contract.
> - "Its id is echoed unless the id is the key named twice." Not when the message is malformed besides:
>   `{"jsonrpc":"2.0","id":7,"id":8}` was answered with id 7.
> - "that judgement now takes less than a megabyte". Only on a bote arena: through `compiled_validate` and
>   `registry_validate_params`, which run on the heap, a mark and a release do nothing, and 18,000 objects against
>   80 properties kept 1.58 MB a call.

**A request that names a key twice is refused before anything reads it.** bote's one key scanner reads the first of a
repeated key, the inputSchema check with it, and a handler is handed the arguments as written: `{"count":3,"count":"x"}`
passed a schema that wants an integer, and a handler whose parser takes the last of a repeated key acted on `"x"`, a
value nothing judged. A message that names a key twice in any object (the envelope, `params`, the arguments and every
object below them, keys compared as they decode), or names a key that is no text, which a client decodes to U+FFFD and
another key may spell, is now an invalid request, `-32600`. Its id is echoed unless the id is the key named twice. The
check rides the syntax check the codec already made, so a message is still walked once. Reported by a consumer and
reproduced in bote first (`docs/development/issues/archive/2026-11-03-request-duplicate-keys-read-first-wins.md`, closed
and archived).

**Four defects in 3.8.5 land beside it**, from an independent review of that release. The worst: `maxLength` counted
no character for a byte from 0x80 to 0xBF that starts no sequence, so 60,000 of them met `maxLength: 4` and reached the
handler. See the correction notice in 3.8.5.

The stated minimum cyrius (6.7.13) is unchanged.

### Security

- **A request that names a key twice, or a key that is no text, is refused** (core: `src/jsonx.cyr`, `src/codec.cyr`,
  `src/dispatch.cyr`). Medium: the schema check and a handler could read different values for one argument.
  - **Where:** in the codec's admission (`_cdc_admit_l_a`), which every transport, the bridge, the modern HTTP path
    and each cancellation reader go through, ahead of every field it reads; and in `dispatcher_dispatch` /
    `dispatcher_dispatch_a`, which check the `params` of a request a consumer built.
  - **What:** a key repeated in one object as it decodes (`"count"` and `"co\u0075nt"` are one key, a surrogate pair
    and its UTF-8 are one key), past every place the scanner stops early; and a key holding bytes that are no UTF-8 or
    a surrogate escape that is not one of a pair. A value that is no text is no key, and is not refused here.
  - **The answer:** `-32600 invalid request: a key named twice, or one that is no text`. Not `-32700`: RFC 8259 asks
    only that names be unique (§4, I-JSON requires it), and a parse error goes out with a null id, which leaves a
    client waiting for an answer it cannot match. A batch's element is refused alone. On the HTTP-family transports a
    2026-07-28 message is a 400 and a legacy one a 200, as their other errors are; a notification is a 400 where it was
    a 202 (MCP: a server that does not accept the input answers with an HTTP error status).
  - **The readers around it:** a `notifications/cancelled` that names a key twice cancels nothing, on a line, a
    WebSocket frame or a POST of its own; the scope gate (`bote_http_scope_gate`) leaves such a message to the codec,
    where it answered a 403 for the first of two tool names.

### Fixed

From the review of 3.8.5:

- **`minLength` and `maxLength` counted no character for a stray UTF-8 continuation byte.** `_sv_strlen` counted a
  byte from 0x80 to 0xBF as part of the character before it whether or not one had begun, so 60,000 of them met
  `maxLength: 4` and were handed to the handler, `minLength: 1` refused one of them, and a result a client reads as six
  U+FFFD passed `maxLength: 2`, which the client then refused. Overlong forms and encoded surrogates were undercounted
  too. The count is now a client's: it decodes the message before JSON reads it, and a decoder (WHATWG's, the
  TypeScript SDK's TextDecoder) gives one U+FFFD for each maximal part of a sequence cut short or broken and one for
  each byte no sequence starts with (Unicode 3.9). Each test vector's count is Node's.
- **A `required` name pushed after `compiled_set_required` was never met**, so a CompiledSchema built by hand whose
  vec gained a name refused every call, where 3.8.4 passed it. Each name is kept beside its Watch, and a name that is
  not the one kept is looked for in the object, which writes nothing a thread shares.
- **A keyword bote judges, with a value it cannot judge by, judged nothing in silence**: `maxLength: "3"`,
  `minLength: 1.5`, `minItems: -1`, draft-04's `exclusiveMinimum: true`, `maximum: "9"`, `additionalProperties: "no"`.
  3.8.4 warned of each as unenforced; 3.8.5 took them off that list. Registration warns again, `maxLength that is no
  non-negative integer`, `exclusiveMinimum that is no number`, `additionalProperties that is no boolean or object`.
  `2.0` and `1e1` are integers, as JSON Schema has them.
- **A schema of more than 64 names kept its table of seen members on the arena for every object it walked**: a
  57 KB array of 19,000 empty objects judged by an item schema of 1,000 properties grew the arena by 19 MB. The table
  is given back when its object's walk adds no violation, and that judgement now takes less than a megabyte.

### Changed

- **The exact repeat walker reads each byte once** (`_jx_rep_value` returns where each value ends): a nested value was
  walked again by each level above it, so 60 KB nested 62 deep was read 63 times, and since a request with one escaped
  key now goes through it, a client could have made every message cost that.
- `dispatcher_dispatch_a` is now the checked entry; `_dsp_dispatch_admitted_a`, with no check, is what the codec, the
  bridge and the modern HTTP path call after admission has checked the whole message.
- 3.5.2's test row "a repeated key: the first, as everything here reads it" is now a refusal.

### Tests

- **`tests/bote_request_keys.tcyr`**, 83, new: the report's three rows (on 3.8.5 the first answered `tool not found`,
  the second a schema violation, and the third ran the handler); a repeat nested, in an array's item, deep, spelled with
  `\u` and `\/`, a surrogate pair against its UTF-8, past the scanner's early return; the envelope's method, id
  (answered with null), jsonrpc and params; keys that are no text; what is no repeat; objects past sixteen keys;
  300-byte keys; the exact walker past an array or an object; every method; a batch; the three cancellation readers; the
  public dispatcher on a request a consumer built; the DEBUG line, which never holds the key; and 0 heap bytes with 60
  KB in every field a client controls, through both checks, and for a ramp of 100 rising sizes.
- **`tests/bote_schema_walk.tcyr`**, 121, from 79: eighteen byte strings counted as Node's TextDecoder counts them,
  each at and either side of its count; 60,000 stray bytes refused through the dispatcher; a result a client reads as
  six characters refused for `maxLength: 2`; a `required` name pushed or replaced; the new WARNs and none for valid
  values; a large schema's table on less than a megabyte of arena, with three objects' violations each kept as written.
- **`tests/bote_http_era.tcyr`**, 1912, from 1863: the refusal on all three HTTP transports, both eras, a notification,
  an id named twice; the scope gate leaving a repeated name or argument to the codec.
- **`tests/bote.tcyr`**, 973, from 969: two arena rows, a repeat beside 60 KB and beside a 60 KB escaped key.
- **`fuzz/request_keys.fcyr`**, new: 20,000 tools/calls from a grammar that knows whether a key repeats or is no text,
  checked through both repeat checks and the codec, the handler running exactly when the message is served, no heap.
- **`fuzz/schema_walk.fcyr`**: strings that are no UTF-8, with the reference counting maximal parts in a form of its
  own; half the schemas padded past eight names, so keys are decoded and looked up; a 300-byte name spelled three ways.
- **35 mutants, 35 killed.** Two survived the first run and showed gaps, now tested: a table given back with the
  violation in it (no later object wrote over its bytes), and a repeat after an array on the exact walker's path, which
  only the new fuzz harness had caught. Each harness kills, alone, the mutants of its side it was run against.

### Performance

**A message costs about 130 to 180 ns more through the codec, and a request a consumer hands the dispatcher the walk
of its params.** Interleaved A/B against 3.8.5, 15 rounds pinned to one core, the working tree's benches on both sides.
Minimums, in ns:

| Bench | 3.8.5 | 3.8.6 | Delta |
|---|---:|---:|---:|
| `codec_process_message` | 4,251 | 4,383 | +3.1% |
| `codec_process_message_arena` | 4,165 | 4,314 | +3.6% |
| `codec_process_message_threaded` | 4,268 | 4,445 | +4.1% |
| `http_era_modern` | 11,133 | 11,443 | +2.8% |
| `jsonx_valid_request` | 564 | 575 | +2.0% |
| `cancel_lines_scan` | 2,898 | 3,046 | +5.1% |
| `dispatch_initialize` | 1,798 | 1,955 | +8.7% |
| `dispatch_tools_call` | 2,369 | 2,717 | +14.7% |
| `schema_compile_simple` | 8,342 | 8,322 | −0.2% |

- **Through the codec, the check is the keys compared while the syntax is read.** A plain walker over the message
  first cost a small call 1.3 µs, +30%; folded into the syntax walk, each key of plain ASCII is compared in place as
  its bytes are checked, and a message holding none other walks once. `jsonx_valid_request`, the syntax check alone,
  moves 2% with the argument it now passes.
- **The `dispatch_*` rows call `dispatcher_dispatch`, which checks the params of a request a consumer built.** Measured
  in one binary against the body with no check: 75 ns with no params, 170 ns for `initialize`'s 33 bytes, about
  340 ns for the call's 43. The transports call the body after admission, so a message served by bote pays only the
  codec's cost; the rest of these rows (`dispatch_tools_list` +8.7%, of which 75 ns is the check) is code position.
- **`cancel_lines_scan`**: each line that may be a cancellation is admitted with its keys checked.
- **The rest is code position.** `jsonx_get_str_flat` −8.8%, `validate_compiled_simple` −5.3% and `libro_export_100`
  −4.7% run no code this release changes in a way that would move them.

## [3.8.5] — 2026-11-03 · One walk of a value

> ⚠ **Corrected at 3.8.6.** These claims below were not true, found by a consumer's report and an independent review
> of this release.
> - "A member a schema names, written twice, is judged by its first occurrence, which is what bote hands a handler."
>   bote hands a handler the arguments as written, both members in them, so a handler whose parser takes the last of a
>   repeated key acted on a value the schema never judged: `{"count":3,"count":"x"}` passed an integer schema and the
>   handler read `"x"`. Since 3.8.6 a request that names a key twice is refused.
> - "`minLength` and `maxLength`, counted in characters as JSON Schema counts them." A byte from 0x80 to 0xBF that
>   starts no sequence counted none, so 60,000 of them met `maxLength: 4` and reached the handler, and a result a client
>   decodes to six U+FFFD passed `maxLength: 2`. Overlong forms and encoded surrogates were undercounted too.
> - DEPS-PATTERN's "one made with `compiled_new` / `compiled_add_property` / `compiled_set_required` and `prop_new`
>   works as before." A `required` name pushed into the vec after `compiled_set_required`, or into the one
>   `compiled_new` makes, was never met, so every call was refused.

**bote judges a value in one walk, and judges the counting keywords that walk makes cheap.** Each object's members are
looked up once, where each property and each required name was a scan of its own; a value is judged where it lies,
where every property's value was copied out first; and a violation's path is written out only for a violation, where
one was built for every element and every property below the root. `minLength` and `maxLength` (in characters, as
JSON Schema counts them), `minItems` and `maxItems`, `minProperties` and `maxProperties`, `exclusiveMinimum` and
`exclusiveMaximum`, and `additionalProperties` (false, or a schema the other members must meet) are judged now, where
registration warned of each, so a tool with no parameters that says `additionalProperties: false`, as the spec
recommends, refuses an argument. Each keyword judges the values of its kind whatever the schema's `type`, as JSON
Schema has it.

**Three defects in 3.8.4 land ahead of it**, from an independent review of that release. The worst: a libro tool's
error result carried `structuredContent` its own outputSchema refuses, and the v1 TypeScript SDK's client, which judges
an error result's too, threw on every one. See the correction notice in 3.8.4.

The stated minimum cyrius (6.7.13) is unchanged.

### Added

- **The counting keywords, judged** (core, `src/schema.cyr`): `minLength` and `maxLength`, counted in characters as
  JSON Schema counts them (code points: a surrogate pair is one, an escape is one); `minItems` and `maxItems`;
  `minProperties` and `maxProperties`; `exclusiveMinimum` and `exclusiveMaximum`, compared exactly as `minimum` and
  `maximum` are; and `additionalProperties`, `false` refusing any member no property names and a schema judging them,
  at the root of an inputSchema given as JSON and at every object below. Beside `patternProperties` it is not judged,
  since which members it covers turns on patterns bote does not match, and registration warns of it there
  (`additionalProperties beside patternProperties`). Each comes off `BOTE_UNENFORCED`, so registration no longer warns
  of it. A violation reads `v.b: additional property not allowed`, `shorter than minLength`, `longer than maxLength`,
  `fewer items than minItems`, `more items than maxItems`, `fewer properties than minProperties`,
  `more properties than maxProperties`, `not greater than exclusiveMinimum`, `not less than exclusiveMaximum`.
- `prop_min_length`, `prop_max_length`, `prop_min_items`, `prop_max_items`, `prop_exclusive_min_raw`,
  `prop_exclusive_max_raw`, `compiled_additional`, `compiled_min_properties`, `compiled_max_properties`.

### Changed

- **One walk of a value** (`_sv_value`, `_sv_object`, `_sv_array`, in place of `_validate_value` and
  `_validate_against`). An object's members are walked once: a key is compared as written with each name where a schema
  has eight or fewer (`BOTE_SV_LINEAR`), else decoded and looked up in a map, either way giving its property and a slot
  that each `required` name is met by. A value is judged as the span of the document it is, never copied. A violation's
  path is a chain of frames on the stack, written out only for a violation. A schema that judges no member does not walk
  at all. CompiledSchema is 80 B and PropertyDef 144 B.
- **Each keyword judges the values of its kind, whatever `type` says**, as JSON Schema has it: `{"maxLength":2}`
  judges a string and passes a number, and `minimum`, `items` and `properties` with no `type` judge the numbers, arrays
  and objects they meet. Through 3.8.4 they judged nothing without the matching `type`. `enum` is judged on strings
  only, as before.
- **A bound that is no number judges nothing.** `{"type":"number","minimum":"x"}` was read as `minimum` 0.
- **Violations come in the order of the members**, and the `required` names an object misses after them. Through
  3.8.4 the missing names came first. The messages are the same.
- A member a schema names, written twice, is judged by its first occurrence, which is what bote hands a handler; one
  it does not name is judged by `additionalProperties` at every occurrence.

### Fixed

From the review of 3.8.4:

- **A libro tool's error result carried `structuredContent` its own outputSchema refuses.** `{"ok":false,"error":…}`
  lacks what each success requires, and the v1 TypeScript SDK's client, which judges an error result's
  `structuredContent`, threw on every libro error. An error result now holds its JSON in its text block and no
  `structuredContent`, but for `libro_verify`'s on a broken chain, whose `ok`, `code`, `index` and `message` its schema
  takes. The schemas lose the `error` property.
- **The listed outputSchema lived in the word the public setter writes.** `tool_def_with_output_schema` called after
  registration sent its bytes unchecked until the next registration. Registration's copy has a slot of its own now,
  `tool_def_output_json` (ToolDef 152 B, +144), and the setter's value is read only by the next registration.
- **`fs_read`'s cut went back without bound** over bytes that are no UTF-8: 65,600 bytes of 0x80 came back as nothing,
  `bytes` 0. It goes back three bytes at most, the longest a character needs, and cuts at the cap past that.

### Tests

- **`tests/bote_schema_walk.tcyr`**, 79, new: each new keyword at and either side of its bound, characters counted in
  escapes, UTF-8 and surrogate pairs, a lone surrogate as one, the exclusive bounds past 2^62 and against fractions, a
  bound that is no number; `additionalProperties` false, true, a schema, beside `patternProperties`, a required name no
  property names; each keyword by kind with no `type`; paths through keys, indexes and keys again, on both ways of
  matching a key; keys compared and looked up, escaped, 300 bytes long, holding a NUL, a name's prefix, named twice;
  the cap of 32; an inputSchema's root and a result through the dispatcher; the WARNs; and the heap: a hundred
  judgements with thirteen violations each keeping nothing, and a 60 KB string judged without an arena chunk.
- **`tests/bote_output.tcyr`**, 255, from 249: a setter called after registration changing nothing listed or judged,
  and no JSON never listed.
- **`tests/bote_libro_tools.tcyr`**, 103, from 101: seven error results, each with no `structuredContent` or one its
  schema takes.
- **`tests/bote_fs_tools.tcyr`**, 78, from 76: a cut through bytes that are no UTF-8, and a four-byte character across
  the cap.
- **`fuzz/schema_walk.fcyr`**, new: 20,000 schemas drawn over the keywords bote judges and six values for each, about
  110,000 judged as a reference on bayan's parse judges them (29,167 passed, 80,324 refused), with no heap. A value
  naming a key twice is skipped, since bayan refuses one.
- **32 mutants, 32 killed.** Three survived the first run and showed gaps, now tested: a path of two keys on the
  looked-up way, a count exactly at `minProperties`, and an escape counted as one character against a tight bound. The
  fuzz harness kills the three of four it was run against that it can reach; the fourth, a repeated key, it skips.

### Performance

**Judging a result costs a quarter less, a nested value half, and registration's compile three times as much.**
Interleaved A/B against 3.8.4, 15 rounds pinned to one core, the working tree's benches on both sides. Minimums, in ns:

| Bench | 3.8.4 | 3.8.5 | Delta |
|---|---:|---:|---:|
| `validate_compiled_simple` | 478 | 451 | −5.6% |
| `validate_compiled_nested` | 1,841 | 925 | −49.8% |
| `dispatch_tools_call_output` | 7,319 | 5,508 | −24.7% |
| `libro_export_100` | 2,667,000 | 1,903,000 | −28.6% |
| `fs_read_64k` | 4,443,000 | 3,734,000 | −16.0% |
| `dispatch_tools_call` | 2,360 | 2,362 | +0.1% |
| `codec_process_message` | 4,278 | 4,238 | −0.9% |
| `schema_compile_simple` | 2,842 | 8,358 | +194.1% |
| `schema_compile_nested` | 6,508 | 19,200 | +195.0% |

- **`libro_export_100` loses 7.6 µs an entry** of the 13 µs 3.8.4 measured: each entry's six properties and six
  required names are one walk of its members, and no path is built. What is left of judging it is mostly the
  repeated-key check over the result, a walk of its own (the roadmap's coverage item).
- **`fs_read_64k` loses 0.7 ms**: its 64 KB string is judged where it lies, where it was copied out and walked seven
  times. Its text still goes out twice, in its block and its `structuredContent`.
- **`validate_compiled_nested`** halves: a nested object's members were a scan per property, with a path built for
  each level.
- **A schema compiles in about three times what it did**, because `parse_property` now looks for each keyword it
  judges, about twenty, in a property's JSON: 350 ns a keyword on a short one, about 6.3 of `schema_compile_simple`'s
  8.4 µs. It runs at registration only, never for a call. Reading a property's keywords in one walk of its members is
  on the roadmap, for the bite that adds more of them.
- **The rest is code position.** `scope_check_jwt` +3.8% and `codec_serialize_response` −5.2% run no code this release
  changes.

## [3.8.4] — 2026-11-03 · Output schemas on bote's own tools

> ⚠ **Corrected at 3.8.5.** These claims below were not true, found by an independent review of this release.
> - "A failed call is an error result, `{"ok":false,"error":…}`, which neither bote nor a client judges." The v1
>   TypeScript SDK's client judges an error result's `structuredContent` too, and that object lacks what each success
>   schema requires, so every libro error (a proof out of range, a retention policy missing its value, libro not set up)
>   threw in that client instead of coming back. The official client 547 passes were the v2 client's, which skips them.
> - "The list wrote the consumer's buffer… Registration keeps its own copy." It kept its copy of the outputSchema in the
>   word `tool_def_with_output_schema` sets, so a setter called after registration sent its bytes unchecked until the next
>   registration: none listed while calls were still judged, a raw LF, or a schema that is no JSON at all.

**The libro tools and `fs_read` say what their results hold.** Each lists an outputSchema, the dispatcher judges every
result that is no error by it, and a client can check it: the official client, on the real binary over stdio, checks
every one on both revisions and finds them as listed. The five libro tools have sent `structuredContent` since 3.4.10,
and their schemas describe it. `fs_read` now sends `structuredContent` too, `{text, bytes, truncated}`, beside the text
block it has always sent, so a client that reads either has the whole file, and one that reads the structured result
learns whether it was cut without reading the note at the end of the text.

The text goes twice now, and a call keeps no more than before: `fs_read`'s result is escaped once on the request
arena and made on the heap at its size, 131 KB for a 64 KB file, where 3.8.3's text block alone, built through a
growing builder, kept about 139 KB.

**Six defects in 3.8.3 land ahead of it**, from an independent review of that release. The worst: a definition
registered again was listed with its new inputSchema and judged by its first compile, so arguments the list forbade
reached the handler. See the correction notice in 3.8.3.

The stated minimum cyrius (6.7.13) is unchanged.

### Added

- **An outputSchema on each libro tool** (full bundle, `src/libro_tools.cyr`): `libro_query` and `libro_export` an `ok`, a
  count (`total`, `count`) and `entries`, each entry its `timestamp` (RFC 3339, or null), `severity`, `source`, `action`,
  `agent_id` and `hash`; `libro_verify` `ok`, and on a broken chain's error result `code`, `index` and `message`;
  `libro_proof` `ok`, `index`, `leaf_count` and `root`; `libro_retention` `ok`, `archived` and `retained`. A failed call
  is an error result, `{"ok":false,"error":…}`, which neither bote nor a client judges.
- **`fs_read`'s `structuredContent`** (full bundle, `src/fs_tools.cyr`): `text`, the file's contents (at most 65,536
  bytes, never cut inside a character), `bytes`, how many bytes of the file it holds, and `truncated`, and an
  outputSchema saying so. The text block is the file's text as before, with the note `[fs_read: truncated]` at its end
  when the file was cut; the structured text has no note. A refusal or a file that cannot be read is an error result
  with no `structuredContent`, as before.
- Every keyword the six schemas use at their root and on its properties is one bote enforces, so registration warns
  of none. An entry's `timestamp` may be null, which only a type array can say; it sits below the root, where bote
  neither warns nor judges, and the client judges it.

### Changed

- **A tool's input schema is compiled at every registration**, unless the consumer attached its own compiled schema
  with `tool_def_with_compiled`, which is kept (ToolDef 144 B, +136: 1 when registration compiled it). A compiled
  schema keeps its own copy of the `required` names. What registration makes is staged under the registries' lock, the
  compile and the `x-mcp-header`s among it (`st` 72 B).
- **A call reads its tool's definition from the look that served it**: `_dsp_tool_serving` copies it onto the request
  arena under the read lock, 144 bytes, and the whole call is judged and shaped by that copy.
- `fs_read`'s result is built at its size, with the text escaped once on the request arena (`_fs_read_result`).
- `scripts/conformance.sh` builds `build/bote` and the client drives its own tools over stdio on both revisions
  (`tests/fixtures/conformance_client.mjs`, `ownTools`).

### Fixed

From the review of 3.8.3:

- **A definition registered again was judged by its first compile.** Registration compiled the input schema only when
  none was attached, and the first registration attached one, so the second kept it: `tools/list` showed a new
  property `y` of type integer, and `{"y":"not an integer"}` reached the handler. The compile is made again at each
  registration now. This predates 3.8.3, which made the list follow the new schema.
- **An `x-mcp-header` taken away stayed required.** Registration stored the headers only when there were some, so a
  definition registered again without its annotation kept the old one, and HTTP refused every call that followed the
  list. Stored whether there are any or not.
- **A `required` name pushed after registration was judged at once**, while the list waited for the next
  registration: the compiled schema shared the ToolSchema's vec. It has its own copy.
- **The list wrote the consumer's buffer.** An inputSchema or an outputSchema given as JSON with no CR or LF was kept
  as the consumer's pointer, and a byte written into it after registration went out, a raw LF among them. Registration
  keeps its own copy (`_reg_own_line`).
- **A `required` name that is no text was dropped at the root of a JSON inputSchema**, so the tool needed nothing. It is
  refused at registration, and so are a ToolSchema property name and a `required` name that are no UTF-8, which a client
  reads as U+FFFD.
- **`jwt_state_principal` named one principal for different subjects**: `\ud800`, `\udbff` and U+FFFD each decoded to
  U+FFFD. An issuer or subject that is no text now names no one, and the state is bound to the token.
- **`inputResponses` keyed with a NUL passed the check that it is an object of objects**, the twin of 3.8.3's fix for
  input requests: the member walk left the member out. Each is read by its key as written, and one keyed with a NUL is
  `-32602`.
- **A call could read two forms of one outputSchema**: the look that served it was released, then the dispatcher read
  the compiled schema and its 2025-11-25 form from the definition, which a registration beside it stages in place. A
  handler that registered its own tool again with the schema taken away had its result lifted by one and judged by
  neither. The call reads its own copy now.

### Security

- Arguments are judged by the schema a tool is listed with, after any registration.
- A requestState sealed for one JWT subject no longer opens for another subject that decodes the same.

### Tests

- **5,928 assertions, from 5,864**, on x86_64 and under `qemu-aarch64` 11.1.3.
- **`tests/bote_tool_schema.tcyr`**, 103, from 85, from the review of 3.8.3: a definition registered again judged by
  its new schema, a consumer's compiled schema kept registered again and attached after a compile, a header taken away,
  a `required` name pushed after registration waiting for the next, the listed schemas registration's copies, names
  that are no text or no UTF-8 refused, and a call served by the definition it was looked up with while its handler
  registered it again.
- **`tests/bote_input.tcyr`**, 148, from 146: `inputResponses` keyed with a NUL.
- **`tests/bote_jwt.tcyr`**, 510, from 507: a subject and an issuer holding a lone surrogate, and a surrogate pair.
- **`tests/bote_libro_tools.tcyr`**, 101, from 73: all five schemas listed on both revisions and the list valid JSON,
  their required members, every tool's success on a populated chain passing its schema on both revisions, a failure
  and a broken chain's verify going as their error results, an entry with no time as null, six results libro's layout
  drifting would send refused by the compiled schema, and registration warning of nothing.
- **`tests/bote_fs_tools.tcyr`**, 76, from 63: the exact bytes, a cut file's note in its block and not its structured
  text, `bytes` and `truncated`, passing on both revisions, control bytes, a refusal judged by nothing, the list,
  registration warning of nothing, and the heap: a 64 KB read keeping its result and nothing more.
- **27 mutants, 26 killed**: 15 on the output schemas and fs_read's result, and 12 on the review's fixes. The survivor
  was a mark and release inside `_fs_read_result` that `fs_read_handler`'s own repeats; it is gone.
- **The official client** passes 547 checks, from 525: on the binary over stdio, on both revisions, the tools that
  list an outputSchema, each libro tool's result checked against its schema, `fs_read`'s text in both places, a cut
  file, and error results. With a schema bote does not enforce in full (a `maxLength` on `fs_read`'s text) the client
  refuses the result, so these passes are its own checks and not bote's.

### Performance

**A result that is judged costs what judging it costs, and `fs_read`'s carries its text twice.** Interleaved A/B
against 3.8.3, 15 rounds pinned to one core, the working tree's benches on both sides. Minimums, in ns:

| Bench | 3.8.3 | 3.8.4 | Delta |
|---|---:|---:|---:|
| `fs_read_64k` (new) | 1,857,000 | 4,514,000 | +143% |
| `libro_export_100` (new) | 1,369,000 | 2,659,000 | +94% |
| `dispatch_tools_call` | 2,335 | 2,361 | +1.1% |
| `codec_process_message` | 4,196 | 4,252 | +1.3% |
| `codec_process_message_arena` | 4,132 | 4,164 | +0.8% |

- **`libro_export_100`, about 13 µs an entry, is judging.** Each entry's six properties and six required names are a
  scan of their own, and a path is built for every element whether a violation is found or not: the cost the
  roadmap's next item, JSON Schema coverage, names, and its first bite in 3.8.5 takes on.
- **`fs_read_64k` pays twice.** Its result carries 64 KB of text in two places, and every pass the dispatcher makes over
  a result (the shape walk, the repeated-key check, the response) goes over twice the bytes: about 1.5 ms. Judging the
  text adds about 1.25 ms more, seven passes over a 64 KB string and a copy of it, which the same work cuts. Measured
  apart, with fs_read's new result on a tool with no outputSchema: 3.4 ms, judged 4.6 ms.
- **The snapshot each call now takes of its definition** first cost every `tools/call` 5 to 19%: 144 bytes through the
  stdlib's `memcpy`, which goes a byte at a time, 243 ns against 31 ns a word at a time. It copies words now, and so
  does `fs_read` for its text. The stdlib copy is filed in cyrius
  (`docs/development/issues/2026-11-02-memcpy-memset-byte-loops-dominate-frame-copies.md`).
- **The rest is code position.** `dispatch_tools_call_result` +8.4% and `dispatch_tools_call_structured` +4.1% run no
  code this release changes, and neither do `jsonx_get_str_flat` +9.6% and `codec_serialize_response` +8.5%: a short
  bench moves 5 to 12% from where its code sits.

## [3.8.3] — 2026-11-03 · Input schemas as JSON

> ⚠ **Corrected at 3.8.4.** These claims below were not true, found by an independent review of this release.
> - "A definition changed after its registration is listed as it was registered until it is registered again." Registered
>   again, it was listed with its new inputSchema and judged by its first compile; an `x-mcp-header` taken away was still
>   required of every HTTP call; and a `required` name pushed after registration was judged at once, while the list left
>   it out.
> - "listed as written, on one line" and "every inputSchema is now listed as registration kept it." An inputSchema or
>   an outputSchema given as JSON holding no CR or LF was listed from the consumer's own buffer, which a byte written into
>   it after registration changed, a raw LF among them.
> - A string that is no text "meets no `required` name." At the root of an inputSchema given as JSON such a name was
>   left out, and the tool registered as needing nothing.
> - `jwt_state_principal`'s parts are "each its length and then its bytes", so a subject unique within its issuer
>   names one principal. The decoder writes U+FFFD for every lone surrogate, so `\ud800`, `\udbff` and U+FFFD named the
>   same one.
> - An `x-mcp-header` is refused "under `$defs`, `allOf` or any keyword that holds a subschema." The keywords walked are
>   the ones the official client walks, which leaves out `additionalItems`, `dependencies` and `contentSchema`: bote and
>   the client agree, and the claim was too broad.
> - Registration "warns of each keyword it lists and does not enforce, the root's among them." It warns at the root and
>   at each property's top level, not below them, as before this release.

**A tool's inputSchema can be its own JSON, every keyword at its root kept.** `tool_schema_from_json(json)` takes a
whole JSON Schema, as `tool_def_with_output_schema` has taken an outputSchema since 3.8.0: `$schema`, `$defs`,
`allOf`, `if` / `then` / `else`, the `additionalProperties: false` the spec recommends for a tool with no parameters,
and the rest are listed as written, on both revisions. A ToolSchema could say only a type, properties and required
names. bote judges arguments by the root's `properties` and `required`, read from the JSON as they decode, and warns
at registration of each keyword it lists and does not enforce, the root's among them. The official suite's
`json-schema-2020-12` scenario, which it does not score, now passes on both revisions.

**Every inputSchema is now listed as registration kept it, on one line.** Through 3.8.2 the list wrote a ToolSchema
out on every request, each property's schema as the consumer gave it, so one written on several lines put a raw line
feed in `tools/list`, which ends the message on stdio and the Unix transport.

**Seven defects in 3.8.2 land ahead of it**, from an independent review of that release. The worst two were in what a
multi round-trip request's `requestState` is bound to: a state was not bound to the request's arguments, and where
the request's claims were not its token it was bound to no one, so it opened for any user who held it. bote's own
binary was not open to the second, since every validator it ships returns the token. A state is now bound to the
request, arguments included, and to its principal, which a new principal function names; the binary names a JWT's
issuer and subject, which also settles the open decision 3.8.2 left: a client that refreshes its token between rounds
keeps its state. See the correction notice in 3.8.2.

The stated minimum cyrius (6.7.13) is unchanged. A state sealed by 3.8.2 does not open under 3.8.3, whose binding is
spelled differently: a client mid-request when a server is upgraded starts the request again.

### Added

- **`tool_schema_from_json(json)`** (core, `src/registry.cyr`): a ToolSchema made from an inputSchema's JSON, with
  `tool_schema_json(s)` to read it back. Its type, its properties' names and schemas, and its required names are
  read from the JSON as they decode, so everything that reads a ToolSchema reads this one. Registration refuses one a
  client's parser would not take, which the official SDK drops with the whole list: an object whose `type` is
  `"object"`, whose `properties`, if any, is an object of schemas (an object or a boolean) with names, and whose
  `required`, if any, is an array of names. And what it refuses in an outputSchema: a key named twice in one object,
  a NUL in a key or a string, a key that is not text, and a `$schema` naming a dialect bote does not know.
- `tool_def_input_json(d)`: the inputSchema as `tools/list` sends it, made at registration.
- **`dispatcher_set_state_principal(d, fp, ctx)`** (core): who a request's claims name, for the binding of its
  `requestState`. `fp(ctx, a, claims)` gives a string naming them, or 0. Dispatcher 216 B, +200 and +208.
- **`jwt_state_principal(secret, a, claims)`** (full bundle, `src/jwt.cyr`): a JWT's issuer and subject, as they
  decode, for `dispatcher_set_state_principal`. It reads the token the bearer validator accepted for the request, as
  `jwt_scope_check` reads its scopes, and does not verify it again, which for ES256 would add a signature check of
  about 10 ms to every round. It names no one for what is no JWT, a token with no subject, or an issuer or subject
  that is no string, is empty or holds a NUL. The binary sets it on
  every dispatcher when its bearer validator is the JWT one, and so does the conformance fixture.

### Changed

- **`tools/list` and a discovery announcement write each inputSchema as registration kept it** (`_emit_input_schema`),
  where they built a ToolSchema's JSON every time. A definition changed after its registration is listed as it was
  registered until it is registered again, as its title and icons have been since 3.8.1.
- **An `x-mcp-header` is found anywhere in an inputSchema given as JSON**, and refused where the spec does not allow
  one: at the root, or under `$defs`, `allOf` or any keyword that holds a subschema. One on a property is read into
  its `Mcp-Param` header as in a ToolSchema.
- **An `x-mcp-header`'s property type is read as it decodes**: `"string"` is a string, as `parse_property` has
  read a type since 3.8.1. It was refused.
- **A `requestState` is bound to the request and who made it** (`_dsp_state_binding_a`), each part its length and
  then its bytes, so no part can borrow from the next: the method, the tool's or prompt's name or the resource's URI,
  the arguments with the spacing between their tokens left out (`_dsp_compact_a`), so a client that sends them
  again compact or pretty sends the same, and the principal the principal function names, else the token the request
  presented, else no one for a request with no claims. A request with claims that name no one cannot be bound: its
  handler's state is `-32603` and a state it brings `-32602`, each with a WARN. Through 3.8.2 the binding was the
  method, the target and the token, one to a line.
- ToolSchema is 32 B: +24 the JSON, or 0. One made by `schema_new` has it zeroed, and one made by hand must. ToolDef
  is 136 B, +128 the inputSchema as listed.

### Fixed

From the review of 3.8.2:

- **A `requestState` was bound to no one where the request's claims were not its token.** Only a validator that
  returns the token itself notes it, so with any other, or a transport that hands the codec its own claims, the
  binding held no identity: alice's state, brought by mallory, opened and its handler read alice's words. It is bound
  to the principal now, and refused both ways where none can be named.
- **A `requestState` was not bound to the arguments.** A state made for `t_a {"env":"staging"}` opened for
  `t_a {"env":"prod"}`, and a handler that keeps its conclusion in the state, an approval or a quote, was given it
  for a request it never judged.
- **Three ways past the InputRequiredResult check**, each sending a handler's state unsealed and its requests
  unjudged: a tool's `resultType` written in 64 bytes of escapes or more, which the shape walk took for none
  (`_dsp_str_dec_is` compares as it decodes now, at any length); a prompt's, a resource's or a completion's
  `resultType` named twice, where bote read the first and a client the last (`_dsp_input_result` reads every one,
  and one named twice is refused); and an input request keyed with a NUL, which the member walk left out (each is
  read by its key as written now, and one keyed with a NUL is refused).
- **Shapes the official client refuses passed the check**: `"inputRequests":{}` with no state, an elicitation or a
  sampling request with no `params`, `params` that are no object, and an elicitation `mode` that is no string,
  which was taken for form. Each is `-32603` now.
- **A string holding a lone surrogate met an enum of U+FFFD**, on arguments and results: bote writes U+FFFD for it,
  and a client's validator keeps the surrogate. Such a string, and one whose bytes are no UTF-8, now matches no enum
  and meets no `required` name, as such a key has been refused since 3.8.2 (`_sch_str_dec_a`).
- **An outputSchema taken away kept judging.** A definition registered again with `tool_def_with_output_schema(t, 0)`
  kept its compiled schema and its 2025-11-25 form: `tools/list` showed none, and a call was refused by it. This
  predates 3.8.2.
- A content block's `type` written in 64 bytes of escapes or more, `resource_link` among them at 78, was taken for no
  type, and the block went as a text block holding its JSON.
- The sealed state's room in its result was six times its length for every sealer. It is its length unless it holds
  a byte the escaper rewrites.

Also:

- **A ToolSchema property written on several lines split `tools/list` in two.** Its raw LF went into the list as it
  was, and stdio and the Unix transport frame a message by its newline, so a client read half a response and then a
  line that was no JSON. The list is now made once, at registration, with every CR and LF between tokens left out.

### Security

- A `requestState` is bound to its principal and its request, as the spec asks: through 3.8.2 a request whose claims
  were not its token bound it to no one, and no request bound its arguments.
- Every InputRequiredResult a handler gives is judged and its state sealed, whatever `resultType` spelling or
  repetition it uses and whatever its requests are keyed with.
- A string that is no text a client reads as bote does, a lone surrogate or bytes that are no UTF-8, meets no enum.

### Tests

- **5,864 assertions, from 5,680**, on x86_64 and under `qemu-aarch64` 11.1.3.
- **`tests/bote_input.tcyr`**, 146, from 102, from the review of 3.8.2: claims that are no token with no principal
  function, the handler's state refused and one brought back refused, and a state with no claims at all still
  working; a principal function's states, for tools, prompts and resources, opening for their principal and no
  other, winning over the token, and told apart from a token of the same bytes; arguments bound, compact, spaced and
  on two lines the same, a space inside a string not spacing, none the same as `{}`, a prompt's too; the binding's
  exact bytes, a target and a token holding a line feed among them; the compactor; an 84-byte escaped `resultType`
  judged and its state sealed; `resultType` twice from a prompt, a resource and a completion, either order; eight
  request shapes a client takes another way or refuses; and a JWT end to end, a refreshed token of the same subject
  keeping the state, another subject's and the same subject's from another issuer refused.
- **`tests/bote_output.tcyr`**, 249, from 240: an outputSchema taken away judging nothing, kept nowhere; a lone
  surrogate meeting no U+FFFD enum on a result and an argument, U+FFFD itself meeting it, a pair the wrong way round;
  a 78-byte escaped `resource_link` lifted.
- **`tests/bote_jwt.tcyr`**, 507, from 491: `jwt_state_principal`'s exact bytes, decoded spellings, every claim that
  names no one, what is no JWT, a token read and not verified again (an expired one, one another key signed), the
  hook's context not read, and no heap.
- **`tests/bote_tool_schema.tcyr`**, 85, new: what `tool_schema_from_json` reads, escaped spellings included;
  twenty-five refusals, each with why and nothing registered; the suite's schema, the spec's shape for no parameters,
  draft-07 and an escaped `x-mcp-header` type taken; the list's exact bytes on both revisions, a schema on several
  lines sent on one, a ToolSchema's property on several lines sent on one (it was split), names escaped; a definition
  changed after registration listed as registered, until registered again; discovery; arguments judged by the
  enum, a type and a required name read as they decode, and `additionalProperties` not judged; the root's WARNs and
  none for what sits below a root keyword; and the heap: forty 4 KB schemas listed ten times on both revisions, and a
  ramp of forty lists each a kilobyte larger, then the same again, keeping nothing.
- **`tests/bote_http_era.tcyr`**, 1863, from 1833: an `x-mcp-header` in an inputSchema given as JSON judged on all
  three HTTP transports.
- **`fuzz/input_schema.fcyr`**, new: 30,000 random schemas, refused or taken exactly as bayan's parser says a client
  would take them, those taken listed byte for byte with their CR and LF out, the list valid JSON, what was read from
  each the same as bayan reads, and no heap.
- **55 mutants, 54 killed**: 26 on inputSchema as JSON and 29 on the review's fixes. The survivor that was a gap,
  what is read from a `required` that is no array, has a test now. One is equivalent: a ToolDef field zeroed where
  fresh memory already is. The fuzz harness kills the four it was run against. One check was taken out before the
  run, a `type` check the ToolSchema's own check makes on the same decoded value. Two pieces have no mutant: the
  binary's wiring of `jwt_state_principal`, which no unit test reaches, and the sealed state's room, which only a
  sealer that writes bytes the escaper rewrites would show.
- **The official suite**: `json-schema-2020-12` passes on 2025-11-25 and 2026-07-28, all eight of its checks, and
  leaves both expected lists. 164 checks pass on 2026-07-28, from 157, and 62 on 2025-11-25, from 55; every scenario
  the suite scores for 2026-07-28 still passes, 39 of 39. **The official client** passes 525 of 525.

### Performance

**A list is cheaper, and a round that asks costs a little more for what its state is bound to.** Interleaved A/B
against 3.8.2, 15 rounds pinned to one core, the working tree's benches on both sides. Minimums, in ns:

| Bench | 3.8.2 | 3.8.3 | Delta |
|---|---:|---:|---:|
| `tools_list_props` (one tool, three properties) | 5,322 | 4,374 | −17.8% |
| `dispatch_tools_list` (one tool, no properties) | 1,811 | 1,638 | −9.6% |
| `input_required_sealed` | 36,051 | 37,323 | +3.5% |
| `input_retry_open` | 22,451 | 23,648 | +5.3% |
| `tools_list_json` (the suite's schema, as JSON) | — | 5,926 | new |

- **A list writes each inputSchema as registration kept it**, where it escaped every property's name and wrote the
  schema out on every request: −17.8% for a tool with three properties, −9.6% for one with none.
- **A round that asks, and its retry, bind more**: the arguments, compacted, and each part's length, HMACed with the
  rest. That is about 1.2 µs on each, and only a request that carries or gives a state pays it. A JWT principal adds
  a Base64url decode of the token's payload, which these rows, run with no claims, do not include. It reads the
  token the validator accepted rather than verifying it again: a first version did, and that would have added an
  ES256 signature check, 10.4 ms on this host (`jwt_es256`), to every round.
- `tools_list_json` has no 3.8.2 row to compare: 3.8.2 cannot list that schema, and its side of the A/B lists an
  empty one.
- **The rest is not the change.** `jsonx_get_str_flat` −10.1%, `cancel_lines_scan` −5.1%, `scope_check_jwt` +4.9%,
  `schema_compile_simple` +4.1%, `http_head_nul` +4.0% and `session_validate` +3.3% run code this release does not
  touch, or touches only for strings it never meets: code position, which moves a short bench by 5 to 12% on its own.

## [3.8.2] — 2026-11-03 · Multi round-trip requests

> ⚠ **Corrected at 3.8.3.** These claims below were not true, found by an independent review of this release.
> - "bound to the method, the tool, prompt or URI, and the token the request presented." Where the request's claims
>   were not the token, from a validator that returns a record or a transport that passes its own claims to the
>   codec, no token was noted, and a state was bound to no one: made for one user, it opened for another. And the
>   arguments were never bound, so a state made for `{"env":"staging"}` opened for `{"env":"prod"}`.
> - "Every state a handler gives" is sealed, and "anything else is the handler's mistake, `-32603`" or `-32021`. A
>   tool's `resultType` written in escapes of 64 bytes or more, `input_required` among them at 84, a prompt's, a
>   resource's or a completion's `resultType` named twice, and an input request keyed with a NUL each went out
>   unjudged, the handler's state unsealed. The client refused that state when it came back.
> - An InputRequiredResult "must hold requests or a state or both." `"inputRequests":{}` with no state passed, and so
>   did an elicitation or a sampling request with no `params` and a `mode` that is no string, which the official
>   client refuses.
> - "`completion/complete` may not ask." One naming `resultType` twice did.
> - "A schema can no longer pass a value a client reads another way." A string holding a lone surrogate met an enum of
>   U+FFFD, which bote writes for it and a client does not.

**A tool, a prompt or a resource can ask the client for input, on MCP 2026-07-28.** A handler returns an
`InputRequiredResult`: elicitation (a form, or a URL to send the user to), sampling, or the client's roots, under keys
of its own, and state of its own. The client gets what is asked and calls again with the answers, which the handler
reads from the request with the state it gave. The handler ABI is unchanged. Each request is judged before it goes:
one needing a capability the client did not declare is answered `-32021`, naming it, as the spec says. The spec makes
the state attacker-controlled, so bote seals every state a handler gives and opens every one a client brings back,
before any handler runs. The binary's sealer is HMAC-SHA256, bound to the method, the tool, prompt or URI, and the
token the request presented, and good for 600 seconds. With it, **every scenario the official suite scores for
2026-07-28 passes, 39 of 39**, the 14 `input-required-*` among them.

**Seven defects in 3.8.1 land ahead of it**, from an independent review of that release. The worst let a handler's
own thread, on a server with one loop, give back bytes its handler was using: the key scanner decoded a long escaped
key on the calling thread's request arena. See the correction notice in 3.8.1.

The stated minimum cyrius (6.7.13) is unchanged.

### Added

- **What a handler reads from the request** (core, `src/protocol.cyr` and the end of `src/jsonx.cyr`):
  - `bote_input_possible()`: 1 on a 2026-07-28 `tools/call`, `prompts/get` or `resources/read`, the three that may
    ask. On an older revision there is no way to ask, and a handler answers as it can.
  - `bote_client_has(cap)`: the request's `clientCapabilities`, by `"elicitation"`, `"elicitation.form"`,
    `"elicitation.url"`, `"sampling"`, `"sampling.tools"`, `"roots"`, or any other name. A bare `elicitation: {}`
    declares form mode, as the spec has it. `bote_caps_have(caps, cap)` is the same for any object. Neither
    allocates, so a handler's own thread may ask.
  - `bote_input_responses_a(a)` and `bote_input_response_a(a, key)`: the client's answers, all or by key, as the
    dispatcher judged them, whether the message came through the codec or `dispatcher_dispatch`.
    `bote_input_accepted_a(a, key)`: the content of an elicitation the client accepted, or 0 when it declined or
    cancelled.
  - `bote_request_state()`: the state the handler gave last round, opened.
- **What a handler returns** (core, `src/dispatch.cyr`): `bote_input_required_new_a(a)`, then
  `bote_input_elicit(ir, key, message, requested_schema)`, `bote_input_elicit_url(ir, key, message, url)`,
  `bote_input_sample(ir, key, params)`, `bote_input_roots(ir, key)` and `bote_input_state(ir, payload)`, and
  `bote_input_required_json(ir)`. Every string is escaped once.
- **The dispatcher judges what goes and what comes back** (core):
  - A handler's InputRequiredResult may ask for `elicitation/create` in form or URL mode, `sampling/createMessage`
    or `roots/list`, and must hold requests or a state or both, no key twice. Anything else is the handler's mistake,
    `-32603`. One needing a capability the request did not declare is `-32021`, with
    `data.requiredCapabilities` in the ClientCapabilities shape, every missing one merged.
  - On 2025-11-25, a tool that asks gets a tool error and a prompt or resource `-32603`. `completion/complete` may not
    ask. An InputRequiredResult carries no cache hints.
  - A retry's `inputResponses` must be an object of objects and its `requestState` a string, else `-32602`. Both
    mean nothing on any other method.
  - `dispatcher_set_state_sealer(d, seal_fp, open_fp, ctx)`: every state a handler gives is sealed by it, and every
    state a client brings is opened by it for the method, its target and the request's token, before any handler
    runs. One that does not open is `-32602`. With no sealer a state is refused both ways. The Dispatcher is 200 B.
- **`src/state_seal.cyr`** (full bundle, now 39 modules): `bote_state_hmac_new(key, klen, ttl_s)` (a key of 32 bytes
  or more), `bote_state_hmac_random(ttl_s)` and `dispatcher_with_state_hmac(d, h)`. A state is the expiry and the
  payload, Base64url, and an HMAC-SHA256 over a domain tag, the binding, the expiry and the payload. It opens under
  its key, for its binding, until it expires (`BOTE_STATE_TTL_S`, 600 s).
- **`BOTE_STATE_KEY`** in the binary: 64 hex digits or more, for servers that share states. Without it each process
  keys itself at random. A key that is not hex, or is too short, is an error at startup.
- The conformance fixture serves the suite's input-required tools and prompt, `test_missing_capability` and
  `test_streaming_elicitation`.

### Changed

- **A title may be empty**, as the spec's `title?: string` allows. 3.8.1 refused one.
- **An icon's `mimeType` may carry parameters** (RFC 9110 8.3.1), such as `image/svg+xml; charset=utf-8`.
- **An inputSchema property's schema is refused** when it names a key twice in one object, holds a NUL, or names a
  key that is not text, as an outputSchema has been since 3.8.1.
- **Registration warns of `format`, `additionalItems`, `dependencies` and an array `items`.** The official client
  enforces `format`, and draft-07's forms are not judged.
- A compiled PropertyDef is 96 B, keeping its bounds as written. The request state is 160 B.

### Fixed

From the review of 3.8.1:

- **A long escaped key was decoded on the calling thread's request arena.** On a server with one loop, a handler's own
  thread shares the request state. Its lookup marked and released the arena its handler was using, and corrupted
  about 4,000 of 80,000 blocks in the probe. On a pooled server each such thread kept a 64 KB arena. Registering
  from another thread went the same way. `_jx_scan` allocates nothing again: a key of 256 bytes or more is compared as
  it decodes (`_jx_dec_eq`), through the one decoder (`_jx_unescape_one`).
- **A `required` name or a string `enum` value that holds a NUL was compared as written.** `"x\u0000y"` as written is
  what `"x\\u0000y"` decodes to, so an enum holding one took a value holding the other, on arguments and on results.
  Such a value now matches nothing and such a name is never met (`_sch_str_dec_a`), and registration refuses a schema
  holding one.
- **A result's integer past 2^62 met bounds it breaks.** 3.8.1 let such a value through, but compared it with its
  bound after holding both at 2^62, so 6e18 met a `maximum` of 5e18. Bounds are now compared exactly, as decimals,
  from what each is written as (`_sch_num_cmp`). That also ends the limit documented since 3.4.8: a `minimum` of 4.5
  took 4.1, and an integer `minimum` of 2.5 took 2.
- **`format` and draft-07's `additionalItems`, `dependencies` and array `items` got no WARN.** They do now.
- **`bote_web_url_ok` looked at the first byte after `//` only.** `http://:80/x`, `http://@/`, `https://a:b:c/`, a port
  of 99999 and `http://%00/` passed, which a client's URL parser refuses, and so did `https://user:pass@host/`, a
  password sent in every list. The authority is now a host and an optional port: a domain, a dotted-quad IPv4 address
  or an IPv6 literal, a port up to 65535, and no userinfo.
- **A key that is not UTF-8 passed the repeated-key check.** A client reads `a<0xFF>` as `a\ufffd`, so the two were one
  key to a client and two to bote. A key that is not UTF-8, or holds a lone surrogate, is now refused where a repeat
  is.
- **The repeated-key check copied every level's subtree.** One 60 KB result nested 62 deep took 4 MB of arena, kept as
  the thread's peak. It now walks the value in place, keeps only the keys, and looks each up in a table past eight of
  them, where it compared every pair.

Also from the review:

- A list's display fragments were stored with nothing held, between a list's sizing pass and its writing pass. They
  are stored under the write lock now, with what registration makes for a tool's outputSchema.

### Security

- `requestState` is sealed and opened by the dispatcher, so a handler never sees a state it did not make for that
  method, target and token, or one past its expiry. The state is signed, not encrypted: the client can read it. It is
  bound to the token, where the spec asks for the principal: a client that refreshes its token between rounds starts
  again.
- The scanner no longer touches any arena, so a handler's thread cannot corrupt its handler's.
- An icon URL or website carrying a password is refused, as is one a client's parser refuses.
- A schema can no longer pass a value a client reads another way: a NUL in a `required` name or `enum` value, a
  bound past 2^62, a key that is not UTF-8.

### Tests

- **5,680 assertions, from 5,510**, on x86_64 and under `qemu-aarch64`.
  - **`tests/bote_input.tcyr`**, 102, new:
    - What a handler reads: whether it may ask, each capability (a bare elicitation is form, url only is not, a
      capability that is no object is not declared, names as they decode, a sub-capability in its own object only),
      the answers by key, accepted or not, read through `dispatcher_dispatch` as through the codec, none on an older revision, nothing kept from the last message, and asking
      allocating nothing.
    - The builder's exact bytes, escaped, and the result as it goes, serverInfo added, no cache hints.
    - `-32021` for each capability and four at once, merged, and each served where it is declared.
    - Each handler mistake, `-32603`, and why. An escaped `resultType` read as it decodes.
    - The state: sealed and not in plain sight; the retry served and its handler reading it as given; refused when
      tampered, a byte changed, spent on another tool, another method, another token or none, expired, not a string,
      not a state, or with no sealer; a prompt's and a resource's bound to their own name and URI.
    - Each method and revision; the audit; an outputSchema not judging an InputRequiredResult.
    - The sealer: a short key, a round trip, another binding, another key, every byte changed, seven shapes that are
      no state, a body too short and a payload holding a NUL even with a good MAC, expiry, and its Base64url against
      bayan's for every length to 70.
    - The heap: a round asking and its retry with 60 KB in each field keep only `args`, and each refusal nothing.
  - **`tests/bote_output.tcyr`**, 240, from 179: each review finding above, and 2,000 random keys, short and long,
    found by what they decode to.
  - **`tests/bote_display.tcyr`**, 107, from 100: forty URLs a parser takes or refuses, checked against Node's,
    mimeType parameters, empty titles.
- **107 mutants, 101 killed**: 56 on multi round-trip requests and the sealer, and 51 on the review's fixes. The
  first pass stopped partway, on a line of test output that was not UTF-8, and the runner now reads any bytes.
  Eleven survivors were gaps in the tests, now closed: among them a state's method left out of its binding, which
  only a prompt named as a tool tells, a short body or a NUL payload under a good MAC, a `required` name compiled
  without registration's check, and three URLs and a mimeType the parsers let through. One mutant was written wrong
  and rewritten. Six are equivalent: two sealer checks a later one always makes, an early exit the final length
  check repeats, the pairwise walk used for every object (slower, not wrong), a Base64 bit that is always zero, and a
  zero check whose answer is multiplied by zero.
- **`scripts/transport-smoke.sh`** checks `BOTE_STATE_KEY`: three keys that are none end the start with rc 2 and a
  line saying why, a good one serves, and the key never reaches the log.
- **The official suite**: 157 checks pass on 2026-07-28, from 134, every scored scenario among them. The fixture
  serves what the suite's input-required scenarios call, each through a real handler. 2025-11-25 is unchanged.
  **The official client** passes 525 of 525.

### Performance

**Asking costs what judging and sealing cost; every other path moves by a few percent either way.** Interleaved A/B
against 3.8.1, 15 rounds pinned to one core. Minimums, in ns:

| Bench | 3.8.1 | 3.8.2 | Delta |
|---|---|---|---|
| `input_required_sealed` | — | 35,968 | new |
| `input_retry_open` | — | 22,506 | new |
| `dispatch_tools_call` | 2,242 | 2,338 | +4.3% |
| `dispatch_tools_call_output` | 7,392 | 7,241 | −2.0% |
| `codec_process_message` | 4,087 | 4,189 | +2.5% |
| `validate_compiled_simple` | 470 | 484 | +3.0% |
| `schema_compile_simple` | 2,713 | 2,902 | +7.0% |
| `cancel_lines_scan` | 2,948 | 3,104 | +5.3% |
| `jsonx_get_str_flat` | 161 | 168 | +4.3% |
| `session_validate` (untouched) | 1,694 | 1,641 | −3.1% |

- **A round that asks, about 36 µs**, measured in pieces: a plain 2026-07-28 call is 10.8 µs; judging the
  InputRequiredResult adds 11.6 (the repeated-key walk, each request's method, mode and capability); the builder
  1.4; the state 11.7, of which the HMAC seal is 4.4 and the rest is writing the result again around the sealed
  state. **The retry, about 22.5 µs**, opens its state in 4.5. Neither is on the path of a call that does not ask,
  and each round waits on a person or a model.
- **`validate_compiled_simple` was +41.6% in the first build** (667 ns), because every bounded number went through the
  exact decimal compare. An integer bound inside 2^62 compares exactly as a value, so only a fractional or a larger
  bound keeps its text now (`_sch_bound_text`), and the row is +3%. Registration parses each bound once more for that,
  which is most of `schema_compile_simple`'s +7%.
- **The rest is not the change.** A first run had `jsonx_get_str_flat` at −1.2% and `cancel_lines_scan` at +4.8% on
  the same scanner code; `cancel_lines_scan`'s inputs hold no escape, so the new decoder never runs for them.
  Restoring 3.8.1's shape of the scanner's inner loop moved `cancel_lines_scan` −2.2% and
  `dispatch_tools_call_structured`, which it does not touch, +14.4%: code position, which moves a short bench by
  5 to 12% on its own. The loop stays as measured. `jwt_cached_hs256` and `_es256` read −30% on their minimums and
  their ranges reach 3.8.1's, so they are noise.

## [3.8.1] — 2026-11-03 · Titles, icons and instructions

> ⚠ **Corrected at 3.8.2.** These claims below were not true, found by an independent review of this release.
> - "It decodes any key now, on the request arena, and gives the bytes back once compared." The arena was the calling
>   thread's. On a server with one loop a handler's own thread shares the request state, and its lookup of a long
>   escaped key gave back bytes its handler was using; on a pooled server each such thread kept a 64 KB arena.
> - "`required` names and string `enum` values … are decoded on both sides now." One holding a NUL was kept as
>   written, `x\u0000y`, which is what `"x\\u0000y"` decodes to, so an enum took a value it does not hold.
> - "An integer of 2^62 or more in a result … the bound does not apply there." The value was let through but still
>   compared with its bound after both were held at 2^62, so a result's 6e18 met a `maximum` of 5e18, which 3.8.0
>   refused.
> - "Keywords bote does not enforce … now each get a WARN." `format`, which the official client enforces, and
>   draft-07's `additionalItems`, `dependencies` and array `items` got none, and bote does not judge them.
> - "`src` is an `http://` or `https://` URL with a host." Only the first byte after `//` was checked, so a URL with no
>   host, a bad port or a password in it was registered.
> - "`_jx_repeats_a` now finds one in any object." Keys were compared as bytes, and a client reads a byte that is not
>   UTF-8 as U+FFFD, so a key it reads twice was not found. The walk also copied every level's subtree.
> - "Refused when the spec would not allow it." An empty title and a mimeType with parameters were refused, and the
>   spec allows both.

**Tools, prompts, resources, templates and the server carry a `title` and `icons`, and the server its `instructions`.**
A title is for people, where `name` is for programs. Prompt arguments carry a title too. serverInfo also gets its
`description` and `websiteUrl`. Each is checked where it is registered or set, and refused when the spec would not
allow it, since the official client refuses a whole list over one icon it cannot read. A list then sends exactly what
passed. `instructions` goes in `initialize`'s result on 2025-11-25 and in `server/discover`'s on 2026-07-28.

**Ten defects in 3.8.0 land ahead of it**, from an independent review of that release. The worst let a result whose
`structuredContent` names a key twice pass its outputSchema: bote judged the first value, and a client reads the last.
See the correction notice in 3.8.0.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

### Added

- **Icons** (core, `src/jsonx.cyr`, beside the escaper their JSON is built with, as every registry uses them): `bote_icon_new(src)`, `bote_icon_with_mime_type`, `bote_icon_with_sizes`,
  `bote_icon_with_theme`, and their getters. An Icon is 32 B.
  - `bote_icons_check(icons)` says why a list cannot be sent, or 0.
    - `src` is an `http://` or `https://` URL with a host, or a `data:` URI of Base64 data. Every byte must be one a URI
      may hold.
    - `mimeType` is type/subtype.
    - Each size is WxH or `any`.
    - `theme` is `light` or `dark`.
  - `bote_display_check(title, icons)` also refuses an empty title. `bote_web_url_ok(url)` is the URL test.
- **Titles and icons on each definition** (core):
  - `tool_def_with_title` / `_with_icons` (ToolDef 128 B);
  - `prompt_def_with_title` / `_with_icons` (PromptDef 56 B), and `prompt_arg_with_title` (PromptArgument 40 B);
  - `resource_def_with_title` / `_with_icons` (ResourceDef 64 B);
  - `resource_template_with_title` / `_with_icons` (ResourceTemplate 72 B).

  Each is checked at registration and kept as the JSON fragment that passed, which its list sends after the name. A
  definition changed after it is registered sends what it was registered with until it is registered again.
- **`dispatcher_set_server_details(d, title, description, website_url, icons)`** (core), 0 or a BoteError. serverInfo is
  built in two forms when it is set:
  - every field, for `initialize` and `server/discover`;
  - the name, title and version, for the `_meta` of every other 2026-07-28 result and a listen's, so that a `data:` icon
    of some kilobytes is not in every reply.
- **`dispatcher_set_instructions(d, text)`** (core): in `initialize`'s and `server/discover`'s results, escaped once
  when set. The Dispatcher grows to 176 B.
- The conformance fixture has a titled tool, prompt, resource and template, and the server's details and instructions,
  which the official client reads back.

### Fixed

From the review of 3.8.0:

- **A key named twice inside `structuredContent` passed the outputSchema.** bote judged its first value, and a client
  reads the last. The check refused a repeat at the result's top level only. `_jx_repeats_a` now finds one in any object
  of the value, and the result is refused.
- **A key of 256 bytes or more, written with an escape, was compared as written.** The scanner (`_jx_scan`) decoded
  shorter keys only, so such a member was never judged by the property it names. It decodes any key now, on the request
  arena, and gives the bytes back once compared.
- **The validator read `type` without decoding it.** `"object"` judged nothing, and it was still listed as an
  object root. `"null"` judged nothing either. Both are judged now (`SCH_NULL`).
- **`required` names and string `enum` values were compared with their escapes left in**, so `"required":["é"]`
  refused every result. They are decoded on both sides now (`_sch_str_dec_a`), for arguments as for results.
- **A tool without an outputSchema let a result name `content`, `structuredContent` or `isError` twice.** Such a result
  was shaped and audited by its first member while a client reads its last, so the audit recorded a failure the client
  saw as a success. It is refused now, from every tool.
- **A schema property named with a NUL still matched**, as written, a member spelled that way. The member walker now
  leaves such a property out, so it matches nothing. An outputSchema holding one is refused at registration, since a
  client would judge it and bote cannot.
- **The 2025-11-25 form left a 2019-09 `$recursiveRef: "#"` as it was.** It then named the wrapper. It now becomes a
  `$ref` to `#/properties/result`, or an `allOf` entry beside a `$ref`, under the SDK's own test: a 2019-09 `$schema` and
  no `$recursiveAnchor: true` at the root.
- **An integer of 2^62 or more in a result was refused**, by the bound arguments have so that a handler reads what was
  sent. A client reads a result whole, so the bound does not apply there.
- **Keywords bote does not enforce went unnamed.** `additionalProperties`, the exclusive bounds, `multipleOf`,
  `uniqueItems`, the property counts, `prefixItems`, `contains`, the conditionals, `propertyNames`,
  `patternProperties`, the dependents, the `unevaluated` pair and `$dynamicRef` / `$recursiveRef` now each get a WARN at
  registration. So do an unknown type and an `enum` on a type other than string. This covers inputSchema and
  outputSchema alike (`BOTE_UNENFORCED`, `_reg_warn_schema`).
- **A bare content block whose `type` is written with an escape** went out as a text block holding it. It is lifted now
  (`_dsp_type_is_content`).

Found while fixing these:

- **An outputSchema naming a dialect bote does not know was taken.** It is refused now. The spec says "Clients and
  servers MUST handle unsupported dialects gracefully", and bote knows the SDK's four: 2020-12, 2019-09, draft-07 and
  draft-06. An outputSchema that repeats a key in one object, or holds a key with a NUL, is refused too.

### Security

- The first two fixes: a handler's result could pass its schema with a value a client reads differently.
- The third and fourth: a schema could judge less than it says, or refuse what it allows.
- An icon `src` that is `javascript:`, `file:` or anything else that is no web URL or Base64 `data:` URI is refused.
  bote fetches no icon.

### Tests

- **5,510 assertions, from 5,353**, on x86_64 and under `qemu-aarch64`.
  - **`tests/bote_display.tcyr`**, 100, new:
    - Every `src` an icon may and may not have: http and https, capitals, percent-encoding, an IPv6 host, `data:` with
      and without a type or a parameter. Refused: empty, none, other schemes, `javascript:`, `file:`, a relative
      reference, no host, a space, a quote, a backslash, a line feed, bad percent signs, and `data:` that is not Base64
      or has a space, a percent sign or a type a URI cannot hold.
    - `mimeType`, sizes, themes, a missing icon, an empty title, the website.
    - Registration refusing each kind of definition, with why, and registering nothing.
    - Each list's exact bytes, escaped, on both revisions. A definition changed after its registration, and then
      registered again.
    - serverInfo: by default; refused details setting nothing; every field in `initialize` and `server/discover`, and
      the name, title and version elsewhere; renamed with its details kept. `instructions`, set and cleared.
    - The heap: ten of each list, `initialize`, `server/discover` and a 2026-07-28 call, with every field long and a
      60 KB `data:` icon. Nothing kept.
  - **`tests/bote_output.tcyr`**, 179, from 122: each review finding above, through the codec on both revisions.
- **87 mutants, 87 killed**: 57 on titles, icons, serverInfo and instructions, and 30 on the review's fixes. Fifteen
  first died by a crash rather than a failed assertion: a refusal let through handed `assert_streq` a 0 in place of a
  reason. The two files now report that as a failure.
- **The official suite** is where 3.8.0 left it. **The official client** passes 525 checks, from 483. It reads
  back a titled tool, prompt, resource and template, serverInfo's title, description, website and icons, and the
  instructions, on each of its transports and revisions. It does not read back a prompt argument's title. bote sends
  it, as the spec's PromptArgument has it, but the client's own schema for an argument has no `title`, so its parse
  drops it. `tests/bote_display.tcyr` checks the bytes instead.

### Performance

**Correctness costs a little on every lookup, and about a microsecond on a result judged by its schema.** Interleaved
A/B against 3.8.0, 15 rounds pinned to one core. Minimums, in ns:

| Bench | 3.8.0 | 3.8.1 | Delta |
|---|---|---|---|
| `dispatch_tools_call_output` | 6,522 | 7,474 | +14.6% |
| `dispatch_tools_call_structured` | 3,547 | 3,587 | +1.1% |
| `dispatch_tools_call` | 2,233 | 2,256 | +1.0% |
| `dispatch_tools_list` | 1,730 | 1,849 | +6.9% |
| `dispatch_initialize` | 1,732 | 1,785 | +3.1% |
| `codec_process_message` | 4,174 | 4,128 | −1.1% |
| `jsonx_get_str_flat` | 156 | 163 | +4.5% |
| `validate_compiled_simple` | 446 | 476 | +6.7% |
| `schema_compile_nested` | 6,244 | 6,509 | +4.2% |
| `session_validate` (untouched) | 1,649 | 1,686 | +2.2% |

- **`dispatch_tools_call_output`, +950 ns:** the check for a key repeated inside `structuredContent`. It walks the
  value with the member walker, which copies each member. That is the price of judging what a client reads, since
  bote cannot judge a value a client may read differently. The first build cost 1.1 µs; passing over scalars, which
  hold no key, took off about 150 ns.
- **The scanner, about +7 ns a lookup** (`jsonx_get_str_flat`): `_jx_scan` is now a call around its walk, so that the
  arena a long key is decoded on is given back once. The first build did that bookkeeping for every key and cost
  more.
- **`validate_compiled_simple`, +30 ns:** that scanner call, and the validator's word that says it is judging a result.
- **`schema_compile_nested`:** registration now decodes `type`, `required` and `enum`.
- **`dispatch_tools_list` (+119 ns) and `dispatch_initialize` (+53 ns):** neither is explained by the code each adds,
  which is a few loads: the kept fragment and the cached serverInfo. `session_validate`, which this release does not
  touch, moved +2.2% in the same run. No layout control was run.

## [3.8.0] — 2026-11-03 · Output schemas

**A tool can declare an `outputSchema`, and every result is judged by it.** Build one with
`tool_def_with_output_schema(t, json)`, where `json` is a JSON Schema in its own JSON. `tools/list` sends it as written.
Every result that is no error must carry `structuredContent` that matches it, because the tools spec says "Servers
MUST provide structured results that conform to this schema". A result that does not match goes out as a tool error
naming each violation, never as it was. A handler can return a bare value, which goes as `structuredContent` with its
JSON in a text block. Each revision gets the shape it allows: 2026-07-28 allows any root and any value, and 2025-11-25
is shown an object, wrapped as `{"result": …}`, the way the official SDK does it.

**Three defects in 3.7.7 land ahead of it**, from an independent review of that release. The worst made a WebSocket
miss a cancellation queued for the next request. See the correction notice in 3.7.7.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

> ⚠ **Corrected at 3.8.1.** These claims below were not true, found by an independent review of this release.
> - "A handler's bug cannot hand a client data its schema says it will not get" and "whose structuredContent does not
>   match, is a tool error". A key named twice inside `structuredContent` was judged by its first value, and a client
>   reads the last, so `{"temperature":1,…,"temperature":"hot"}` passed a `number` property. So did a member of 256
>   bytes or more written with an escape: the scanner compared such a key as written, so the property it names never
>   judged it.
> - "Any root type is judged". A `type` written with an escape (`"obj\u0065ct"`) was read as written, judged nothing,
>   and was still listed as an object. `"null"`, and any type bote does not know, judged nothing too, with no warning.
> - `required` names and string `enum` values were compared with their escapes left in, so a schema's
>   `"required":["\u00e9"]` refused every result.
> - "A repeated content, structuredContent or isError is refused" held for a tool with an outputSchema only. From any
>   other tool such a result went out, shaped by its first `structuredContent`, while a client reads its last.
> - "Kept as it was written, and matches nothing". A property named with a NUL was kept as written, `n\u0000`, and
>   judged a message's member written that way.
> - "These are the official SDK's rules". The SDK also turns a 2019-09 `$recursiveRef: "#"` into a reference under
>   `properties/result`. bote left it, and it then named the wrapper.
> - An integer of 2^62 or more in a result was refused, by a bound that is the input side's.
> - Not every keyword bote does not enforce was warned of: `additionalProperties`, `enum` on a type other than a
>   string, and a dozen others were neither enforced nor named.
> - "A path is named only for a violation" held for the root's own properties. A path is still built for each array
>   element and each property below the root.
> - A bare content block whose `type` is written with an escape went out as a text block holding it.
>
> Each is fixed in 3.8.1, except the paths below the root, which are named as before.

### Added

- **`tool_def_with_output_schema(t, json)`** and `tool_def_output_schema(t)` (core, `src/registry.cyr`).
  - Registration refuses a schema that is not valid JSON or not an object.
  - Registration keeps the schema on one line and compiles it with the property parser the input side uses
    (`parse_property`), so any root type is judged.
  - It warns of the keywords bote lists and does not enforce, on the root and its properties, as for `inputSchema`.
  - For a root whose `type` is not `"object"`, it builds the form 2025-11-25 is shown (`_reg_output_legacy`):
    - the schema becomes the property `result` of an object;
    - `$schema` moves to the wrapper;
    - every `$ref` and `$dynamicRef` into the document moves under `#/properties/result`;
    - names in `properties`, `$defs` and the other maps, data in `const`, `enum`, `default` and `examples`, anchors,
      other documents, and a subschema with a base of its own are left as they were. These are the official SDK's rules.
  - A `ToolDef` grows from 80 to 104 B.
- **`tools/list`** sends a tool's outputSchema, in the form its revision allows. `_dsp_serve_a` takes the revision.
- **What a call's result is judged by, and sent as** (`_dsp_tool_result_out_a`):
  - **A tool with an outputSchema.** A result that is no error and has no `structuredContent`, or whose
    `structuredContent` does not match, is a tool error, as an argument failing the `inputSchema` is. Each violation
    is named from `structuredContent`, for example `structuredContent.temperature: expected number` or
    `structuredContent[0].id: expected string`.
    - So is a result that names `content`, `structuredContent` or `isError` twice. bote reads the first of each, and a
      client may read the last, so a second `isError` could have passed an unjudged result.
    - The call is audited and published as a failure. A WARN names the tool and why, never what the result held.
    - An `isError` result is not judged. That is the SDK's rule; the spec does not say.
  - **A bare value** a handler returns for such a tool is its `structuredContent`, with its JSON in a text block, as
    the spec says a tool SHOULD send it (`_dsp_tool_structured_result_a`).
  - **`structuredContent` that is no object**, from any tool, gets a text block holding its JSON when the result has
    none (SEP-2106 §4.3, as the SDK does). Before 2026-07-28 it goes as `{"result": …}`, and so does all
    `structuredContent` of a tool listed there with a wrapped schema.
- **`_sch_check_value_a(a, prop, root, value)`** (core): any JSON value against a whole schema, its violations on `a`,
  each named from `root`. It judges from the empty path, as arguments are judged, and names a violation only once it
  has one: a path built for every property cost a judgement a fifth more.
- **`_jx_members_a(a, obj)`** and **`_jx_members_raw_a(a, obj)`** (core): the member walker on an arena, and one that
  also gives each key as it was written, for code that writes an object again. The 2025-11-25 form of a schema and a
  rebuilt result write each key as it was, so it means what it did.
- **What coercion learns of a result is kept for the check after it** (`_dsp_result_shape(buf, info)`, a 24-byte
  DspResult): where its first `content` and `structuredContent` are, and whether it names `content`,
  `structuredContent` or `isError` twice, from the one pass over its keys coercion already made. A tool without an
  outputSchema whose result has no `structuredContent` pays nothing more for 3.8.0.
- **`bote_http_send_unanswered(hc)`**, for the fix below.
- The conformance fixture has three tools with an outputSchema, which the official client checks.

### Changed

- **What a call's result is built into goes on the request arena** (`_dsp_coerce_result`, which now takes the arena,
  the tool, whether a sink is listening, and the DspResult it fills). This covers the text block of a bare value, the lifted content block,
  the one-line copy of a result on several lines, and a failure's message. That message is copied to the heap only for
  an audit or event sink, which may keep it. Through 3.7.7 it was built on the heap, and part of it stayed for good:
  measured beyond `args`, 160 B a call for a handler returning a 7-byte bare record and 1.2 KB for a 60 KB one. Now 0.

### Fixed

From the review of 3.7.7:

- **A WebSocket missed a cancellation queued while the request before it ran.** The check judged a request by its id's
  address, which the next message mostly shares, since the arena is rewound for it. So it went on from where the last
  request's look had stopped, past the cancellation. The next request ran to its end and was answered. Each message's
  check now looks from the queue's head.
- **A cancelled legacy request's POST got no HTTP response.** The connection closed with no status line, which a
  client reads as a failed request. 2025-11-25 says a request's POST MUST get an SSE stream or a JSON object, and a
  cancelled request SHOULD get no JSON-RPC response. It now gets an SSE stream ended with no event: the shape a
  request whose progress had begun already got. A client that takes no SSE gets 202, as there is no JSON object to
  send it.
- **Streamable's piggyback read `Accept` from its first field and took `q=0` as yes**, while progress read every
  field. It reads `Accept` as progress does now (`bote_http_accepts_sse`).
- `docs/architecture/overview.md` named `bote_conn_tid`, which 3.7.7 removed.

Found while building this:

- **A handler's result was read by its keys as written.** The scanner reads a key as what it decodes to, and so does a
  client. Coercion now does too: `"is\u0045rror":true` was a success to the audit and an error to the client, and a
  result whose `"cont\u0065nt"` is an array went out as a text block holding the whole result.
- **A nested schema property whose name holds an escaped NUL judged a message's member cut at the NUL**: `"a\u0000b"`
  judged `"a"`. The member walker cut the decoded name at the NUL. A name whose decoding holds one is now kept as it was
  written, and matches nothing, as the scanner has it.

### Security

- An outputSchema is now enforced on the server: a handler's bug cannot hand a client data its schema says it will not
  get. A repeated `content`, `structuredContent` or `isError` is refused, so that what bote judged is what a client
  reads.
- The arena change above: a client calling a tool whose handler returns bare JSON no longer grows the heap with each
  call.

### Tests

- **5,353 assertions, from 5,224**, on x86_64 and under `qemu-aarch64`.
  - **`tests/bote_output.tcyr`**, 122, new:
    - registration's refusals, the schema kept on one line and compiled, and a definition registered again with
      another root;
    - the 2025-11-25 form's exact bytes for array, string and untyped roots, `$schema` moved, every kind of reference,
      names, data, anchors, other documents, a base kept, and escaped keys and references;
    - the WARNs, and `tools/list` on both revisions;
    - on both revisions, what a call's result goes as: matching, missing, mismatched with each violation named, an
      error result, `isError` false, a bare record and array lifted, a member twice (spelled two ways), a text block
      added after others or not, an untyped root wrapped, and a tool without a schema shaped and never judged;
    - keys read as they decode: an escaped `isError`, `content` and `structuredContent`, a property named with a NUL
      judging nothing, and every key of a rebuilt result written as it was;
    - the log never holding the result, the audit's messages kept past the next messages, and a batch;
    - the heap: results refused with 60 KB on two lines, a 60 KB array lifted on both revisions, a bare record as
      text, a ramp of rising sizes, and lists of forty 4 KB schemas with array and with object roots. Each keeps
      nothing beyond `args`.
  - **`tests/bote_cancel.tcyr`**, 141, from 137: a cancellation queued on a WebSocket while the request before it
    ran, and a cancelled legacy POST answered with an SSE stream with no event, or 202.
  - **`tests/bote_streamable.tcyr`**, 64, from 61: the piggyback's `Accept` with `q=0`, a second field, and a line in
    the body.
- **`fuzz/output_shape.fcyr`**, new: 20,000 random schemas through the 2025-11-25 form and 30,000 random results through
  their shaping. Each must come out valid JSON in the shape its revision allows, a schema with no reference must be
  carried byte for byte, and nothing may stay on the heap.
- **65 mutants, 65 killed.** One first survived: removing the room a `tools/list` keeps for each outputSchema. The long
  list's schemas all had array roots, and the room for their 2025-11-25 form covered the schema too. A list of
  object roots kills it.
- **The official suite** is where 3.7.7 left it. **The official client** passes 483 checks, from 448: a record, an
  array (wrapped on 2025-11-25) and a refused result, on each of its transports and revisions.

### Performance

**A tool call costs a little more, and judging a result by its outputSchema costs what judging the same object as
arguments does.** Interleaved A/B against 3.7.7, 15 rounds pinned to one core. Minimums, in ns:

| Bench | 3.7.7 | 3.8.0 | Delta |
|---|---|---|---|
| `dispatch_tools_call` | 2,057 | 2,213 | +7.6% |
| `dispatch_tools_call_result` | 1,871 | 1,886 | +0.8% |
| `dispatch_tools_call_scoped` | 4,990 | 5,148 | +3.2% |
| `codec_process_message` | 3,882 | 4,128 | +6.3% |
| `codec_process_message_arena` | 3,889 | 4,047 | +4.1% |
| `codec_serialize_response` (untouched) | 177 | 189 | +6.8% |
| `dispatch_tools_call_structured` | — | 3,504 | new |
| `dispatch_tools_call_output` | — | 6,437 | new |

- About 75 ns of `dispatch_tools_call`'s 156 is measured in isolation:
  - +40 ns: the text block of a bare value is sized before it is written, which keeps it on the arena;
  - +20 ns: the key walk decodes keys as the scanner does;
  - +15 ns: the check after coercion.

  `codec_serialize_response`, code this release does not touch, moved +6.8% in the same run. The rest is likely in
  the range that code position alone moves these rows, though no layout control was run to prove it.
- **`dispatch_tools_call_output` against `_structured`**: +2.9 µs. These are the same result, from a tool with an
  outputSchema and from one without. The cost is the validator judging three required fields, which is what an
  `inputSchema` of the same shape costs a call.
  - The first build cost 4.6 µs. A second walk copied every member to find a repeated key (1.2 µs), and a path was
    built for every property judged (0.7 µs).
  - Both are gone. The repeat is found in the walk coercion already makes, and a path is named only for a violation.
  - Judging an object's names in one pass of the scanner is on the roadmap.
- **`dispatch_tools_call_result`** paid +9.8% in the first build, from a second scan of the result's keys. With the
  walk shared it is +0.8%.

## [3.7.7] — 2026-11-03 · Cancellation everywhere

**Cancellation on a WebSocket, and on the legacy streamable path.** On a WebSocket a `notifications/cancelled` frame, a
close frame, or a client gone cancels a request. The check reads only frames that have arrived whole, and keeps each one
it reads for the loop, in its turn. For 2025-11-25 on `bote streamable`, a `notifications/cancelled` POST of its own
reaches the request in flight on another worker, within the same session. With 3.7.5 and 3.7.6, every transport now
carries cancellation and progress.

**Five defects in 3.7.6 land ahead of it**, from an independent review of that release. The worst corrupted a handler's
request arena: on a one-loop server its helper thread asked or reported through the shared request state. See the
correction notice in 3.7.6.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

> ⚠ **Corrected at 3.8.0.** Three claims below were not true, found by an independent review of this release.
> - "A `notifications/cancelled` naming the request cancels it" missed one queued while the request before it ran. The
>   check judged a request by its id's address, which the next message, on a rewound arena, mostly shares, so it went on
>   from where the last request's look had stopped. The review's probe over the real handler: the next request ran to its
>   end and was answered, with every id tried, numbers and strings.
> - "A cancelled request's POST ends with no answer, as 2025-11-25 says it SHOULD." The SHOULD is about the JSON-RPC
>   response. The POST got no HTTP response at all, where the spec says a request's POST MUST get an SSE stream or a
>   JSON object, and a client reads a closed connection as a failed request.
> - "Every field is read now, and a weight of 0 refuses" held for progress and not for streamable's piggyback, which
>   still read one `Accept` field and took `q=0` as yes, so one POST could be judged both ways.
>
> Each is fixed in 3.8.0.

### Added

- **On a WebSocket** (`src/transport_ws.cyr`), `_ws_cancelled`:
  - It peeks at what the socket holds (`MSG_PEEK | MSG_DONTWAIT`) and reads a frame only once it is whole
    (`_ws_frame_len`), never waiting for the rest of one.
  - Each frame it reads is queued, `BOTE_WS_QUEUE` (16) in `BOTE_WS_QUEUE_BYTES` (64 KB). `_ws_next_frame` serves them in
    order before the loop waits on the socket again, so a request or a ping read while looking is answered in its turn.
  - A close frame, a client gone, or a `notifications/cancelled` naming the request cancels it. A batch's next request
    looks again.
- **On the legacy streamable path** (`src/session.cyr`, `src/transport_streamable.cyr`):
  - A request with a session is entered in flight while it runs, by its session and its id as a value, with the word of
    its worker's request state that `bote_cancelled` reads (`session_inflight_begin` / `_end` / `_cancel`).
  - A `notifications/cancelled` POST on that session sets that word. A cancelled request's POST ends with no answer, as
    2025-11-25 says it SHOULD.
  - At most `BOTE_INFLIGHT_SLOTS` (64) are in flight a store, and a request that cannot be entered cannot be cancelled,
    which the spec allows. The session store grows from 32 to 40 B.
- **`codec_cancelled_id_a(a, p, n, params_out)`** (core): the `requestId` of a cancellation the codec would admit, for any
  transport. **`codec_message_cancels_a(a, p, n, id)`** (core): one frame's.
- **`bote_req_owner_here()`** (core), and the request state's +120, for the fix below. The state grows from 120 to 128 B.

### Changed

- The cancelled flag stays after the codec returns, for a transport that must close rather than answer. It is cleared by
  the next message's begin and between a batch's requests, and no longer before a dispatch, where a cancellation from
  another connection may already have set it.
- `bote http`, which serves one request at a time, gives a cancellation POST no request in flight to reach: it is served
  after the request it names, which has been answered. On `bote streamable` with workers it is matched.
- `bote_conn_tid` is gone, its work done once in the core.

### Fixed

From the review of 3.7.6:

- **A handler's helper thread still raced the handler on its request arena.** The guard was in each transport's check and
  writer, but `bote_cancelled()` and `bote_progress()` mark, allocate on and release the arena before they reach either.
  On a one-loop server every thread reaches the one request state, so a helper's ask rewound the arena over what the
  handler had just made. The review's probe over the real stdio loop corrupted 191,296 of the handler's blocks with
  `bote_cancelled`, and some in every run with `bote_progress`.
  - The thread that sets a check or a writer is recorded (`bote_req_set_cancel`, `bote_req_set_progress`).
  - Both functions answer any other thread with nothing, before they touch the arena.
- **A listen stream could keep a scope list it was never judged by.** It judged a URI by one look at the registry and kept
  the list from a second look, so a registration between the two left it holding the new list, and it was told. It now
  judges by the list it keeps, from one look.
- **A progress report cut short on a Unix socket or a WebSocket left the connection served.** The answer went straight
  after the cut bytes, glued to a broken line or inside a frame's payload. The writer marks the connection broken, writes
  nothing more, and the loop drops the client before it answers, as a failed listen write does.
- **`Accept` was read from its first field only, and `text/event-stream;q=0` counted as yes.** Every field is read now,
  and a weight of 0 refuses (RFC 9110 12.4.2, 12.5.1).
- **The line scanner logged a DEBUG line for each malformed line it looked at**, before the codec logged it in its turn.

### Security

- The arena fix above: a handler that started a thread on stdio, a Unix socket, a WebSocket or a one-loop HTTP server
  could corrupt its own request's memory through it.
- The scope fix above.
- A WebSocket's cancellation check reads only frames already whole, so a client cannot hold a handler on a frame it never
  finishes. A legacy cancellation reaches only a request of the session it came on.

### Tests

- **5,224 assertions, from 5,177**, on x86_64 and under `qemu-aarch64`.
  - **`tests/bote_cancel.tcyr`**, 137, from 109:
    - A WebSocket with a listen line on it, so its loop waits on the socket and the line between messages:
      - a cancellation frame read while the handler runs, and one naming another request;
      - a request read past while looking, served in its turn;
      - a batch's later request;
      - a frame half sent, never waited for and served once whole;
      - a helper thread answered with nothing;
      - a ping queued and answered, a close frame, and a half-close;
      - twelve cancellations on one connection, the queue emptied and used again.
    - The legacy streamable path on two threads:
      - another session's cancellation, and the same session's of another id, reaching nothing;
      - the session's own cancellation ending the request's POST with no answer;
      - an uncancelled request answered and gone from the table after.
    - A malformed line looked at, logging nothing.
  - **`tests/bote_progress.tcyr`**, 110, from 93:
    - A helper thread asking and reporting the whole time its handler makes and checks arena blocks, 5,000 rounds, every
      block as written. 3.7.6's tests joined the helper before the handler did anything, so the two never ran together.
    - A report cut short at the send deadline, the client draining before the answer would go: unix and WebSocket drop
      it, with no answer after the cut.
    - `text/event-stream` on a second `Accept` field, `q=0` refused, and `q=0.5` in another case accepted.
  - **`tests/bote_listen_line.tcyr`**, 192, from 190: a registration between the judging and the keeping, the stream
    left with the list it was judged by, and not told.
- **27 mutants, 27 killed.** Four first survived the tests, and the tests added for them kill them:
  - a queue never emptied, which only a connection that queues more than sixteen frames shows;
  - a cancellation matching any id of the session;
  - and a progress write cut short with the connection kept, on unix and on WebSocket, which needed a client that drains
    before the answer. That test also found 3.7.7's own first fix checking for the cut after the unix answer was written.
- **The official suite** is where 3.7.6 left it. **The official client** passes 448 checks.

### Performance

**The request path costs what it did.** Interleaved A/B against 3.7.6, 15 rounds pinned to one core. Minimums, in ns:

| Bench | 3.7.6 | 3.7.7 | Delta |
|---|---|---|---|
| `codec_process_message` | 3,937 | 3,920 | −0.4% |
| `codec_process_message_threaded` | 4,229 | 4,235 | +0.1% |
| `dispatch_tools_call` | 2,058 | 2,061 | +0.1% |
| `progress_report` | 1,804 | 2,178 | +20.7% |
| `cancel_lines_scan` | 2,907 | 3,015 | +3.7% |

`progress_report` pays for the fix above: one `gettid` a report, before the arena, and so does each `bote_cancelled()`. The
setters record the thread once a message, so every message on a transport that sets a check pays one `gettid` too:
3.7.6's "a request that reports nothing pays nothing" was not quite true either, at one `gettid` a POST. `cancel_lines_scan` reads each line with the codec's admission minus its log.
`session_validate` (+7.0%) runs none of this patch's code and its 3.7.6 range spans the new minimum, which is layout.

## [3.7.6] — 2026-11-02 · Progress

**A handler can report its progress, on the request's own stream.** A request that carries `_meta.progressToken` hears
`notifications/progress` as its handler reports with `bote_progress(progress, total, message)`:
- On HTTP, the response to its POST becomes an SSE stream at the first report: the reports, then the response, which ends
  it.
- On stdio, a Unix socket and a WebSocket, each report is a line or a frame, before the answer.

bote refuses a `progress` no greater than the last, which the spec says MUST increase. It sends nothing for a request
with no token, or one already cancelled or answered. The official suite's `tools-call-with-progress` now passes on both
revisions, and the official client hears the reports on its four transports.

**Four defects in 3.7.5 land ahead of it**, from an independent review of that release. The worst kept telling a token of
changes to a resource it could no longer read, once the resource was registered again with a scope the token lacks. See
the correction notice in 3.7.5.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

> ⚠ **Corrected at 3.7.7.** Five claims below were not true, found by an independent review of this release.
> - "No longer races the connection's own reads and writes" and "answers another with nothing" held for the transports'
>   checks and writers, and not for the request arena. `bote_cancelled()` and `bote_progress()` mark, allocate on and
>   release it before they reach either, so a helper thread still corrupted the handler's memory on a one-loop server.
> - "Told … only while that list still serves it" missed a registration between the look that judged a URI and the look
>   that kept its list: the stream kept a list it was never judged by.
> - On a Unix socket and a WebSocket a report cut short at the send deadline left the connection served, the answer
>   written after the cut.
> - "A client whose `Accept` names no `text/event-stream` hears no progress" read only the first `Accept` field, and took
>   `q=0` as yes.
> - On agnos, where a thread runs inline on its starter, the thread guard told nothing apart.
>
> Each is fixed in 3.7.7, the last made moot.

### Added

- **`bote_progress(progress, total, message)`** (core, `src/codec.cyr`). `total` is -1 when none is known, and `message`
  is 0 when there is none. It returns:
  - 1 when the report went;
  - 0 when there is nothing to send it to: no token, no stream for it, or a request cancelled;
  - -1 when it was refused: a `progress` no greater than the last, or a connection that would not take it.

  The notification is built on the request arena and gone once sent.
  - A transport sets its writer with `bote_req_set_progress(fp, ctx)`, set after `bote_req_begin`. `fp(ctx, a, json)`
    returns 0 when it sent, below 0 when it failed, and 1 when it has no stream for the report.
  - The codec names the request's params (`bote_req_set_request_params`); `bote_req_progress_sent()` counts.
  - The request state grows from 80 to 120 B: the writer, its context, the request's params, the last `progress` sent,
    and how many.
- **On HTTP** (`src/http_era.cyr`), `bote_http_progress_begin(hc)` and `bote_http_progress_end(ps, json)`, around the
  dispatch. They are used on the modern path and on the legacy one on `http` and `streamable`.
  - The first report sends the SSE head: `Cache-Control: no-cache`, `X-Accel-Buffering: no` (the spec's SHOULD), and the
    transport's own extra headers, the bridge's CORS among them.
  - Each report goes as an `event: message` chunk, and the response as the last, before the end.
  - A client whose `Accept` names no `text/event-stream` hears no progress. Its `Accept` is read only at the first report.
  - `bote_http_accepts_sse(hc)`.
- **On stdio, a Unix socket and a WebSocket**: the connection's own writer, a line or a text frame.
- **`bote_conn_tid()`** (`src/conn.cyr`): the calling thread, 0 off Linux, for the fix below.
- `test_tool_with_progress` in the conformance fixture reports 0, 50 and 100 of 100, 50 ms apart, as the suite asks, where
  it had answered and reported nothing.

### Changed

- On the legacy `streamable` path a request that reported answers on its SSE stream. Its session's pending notifications
  then wait for the next POST or GET, where they had followed the response.
- The bridge's legacy path stays one JSON reply: its contract with its callers is not MCP's.

### Fixed

From the review of 3.7.5:

- **A resource registered again with a scope the token lacks went on being told to it.** A stream records, for each URI it
  keeps, the scope list it was judged by: the registration's own. A stream made with claims is told of an update to a URI
  that requires a scope only while that list still serves it. The same list registered again keeps it. Another list is
  withheld, even one the token holds, since a stream keeps no token. The overflow path checks the same.
- **On stdio and a Unix socket, a batch lost the cancellation of a later request.** The first request's check read past a
  `notifications/cancelled` naming the second, and the second's look started after it. A check now looks again from the
  top when the request it is asked about changes.
- **A handler's helper thread could reach the transport's check and writer.** On a one-loop server every thread reaches the
  process's one request state. So a helper's `bote_cancelled()` read stdin or a Unix socket into the loop's buffer beside
  the loop, and its report could have gone on the response beside the handler's. Each transport records its serving
  thread (`bote_conn_tid`) and answers another with nothing, on stdio, unix, WebSocket and HTTP. A worker on `bote
  streamable` has a state of its own, so its helpers never reached one.
- **The line scanner honored a cancellation the codec refuses**, one whose `jsonrpc` is not `"2.0"`. It now reads a line as
  the codec admits it (`_cdc_admit_a`).

### Security

- The scope fix above. Through 3.7.5, a token judged by a resource's old registration heard of each change to it after
  the resource came to require a scope the token lacks.
- A handler that starts a thread on stdio, a Unix socket or a one-loop HTTP server no longer races the connection's own
  reads and writes through it.

### Tests

- **5,177 assertions, from 5,067**, on x86_64 and under `qemu-aarch64`.
  - **`tests/bote_progress.tcyr`**, 93, new:
    - `bote_progress` and the writer: no writer, no request, no `_meta`, no token, and a token that is no string or number.
      The exact bytes with and without a total and a message. MUST increase. A failed write, and a writer with no
      stream, not counted. Number and escaped tokens. Nothing for a cancelled request. A begun message clearing the
      writer. A thousand reports of a kilobyte leaving the arena where it was.
    - The codec: three reports stamped with the request's token, nothing once it is answered, nothing with no token,
      legacy, and a batch whose second request counts from 0 again.
    - stdio, a Unix socket and a WebSocket, a line or a frame each before the answer, and a handler's helper thread
      answered with nothing.
    - The three HTTP handlers:
      - the SSE head, one head, three events, then the response and the end, and CORS on the bridge;
      - JSON for a client that takes no event stream, and for a call with no token;
      - both eras on `http` and `streamable`, and the bridge's legacy contract one JSON reply.
    - A client that leaves mid-stream, cancelled and sent nothing more; the heap; two workers apart.
  - **`tests/bote_cancel.tcyr`**, 109, from 96:
    - A batch's later request cancelled through the real stdio and unix scanners, where 3.7.5's test used a stand-in
      check.
    - A handler's helper thread answered with nothing on stdio, unix and HTTP.
    - A `jsonrpc` the codec refuses.
  - **`tests/bote_listen_line.tcyr`**, 190, from 186: a resource registered again with its scopes still told, with
    another not, nor by an overflow; the scope list each URI keeps.
- **37 mutants, 36 killed.** The one left showed the check that `_meta` is an object to be redundant, since an array holds
  no token, and it is gone. Four first survived the tests:
  - The writer a begun message clears: the test's params were cleared too.
  - The thread guards on unix and HTTP: there was no helper test there.
- **The official suite** passes `tools-call-with-progress` on both revisions: three reports and a schema-valid wire.
- **The official client** passes 448 checks, from 434, hearing the reports on all four of its transports.
  - On stdio the client handles a response at once and a notification a tick later. So the last report, which arrives in
    the same read as the answer, finds its handler already removed and is dropped.
  - bote sends all three, in order, which a raw stdio run shows. The client's check there asks for two.

### Performance

**The request path costs what it did.** Interleaved A/B against 3.7.5, 15 rounds pinned to one core. Minimums, in ns:

| Bench | 3.7.5 | 3.7.6 | Delta |
|---|---|---|---|
| `codec_process_message` | 3,970 | 3,978 | +0.2% |
| `codec_process_message_threaded` | 4,258 | 4,263 | +0.1% |
| `dispatch_tools_call` | 2,071 | 2,076 | +0.2% |
| `http_era_modern` | 11,180 | 11,095 | −0.8% |

The codec now names each request's params with two stores beside the id's, from the state it already holds. Rows that run
none of this patch's code moved further (`jsonx_get_str_flat` −7.6%, `jwt_cached_hs256` +6.2%, `jsonx_valid_request`
+4.6%), which is layout. `progress_report`, new, is 1,822 ns for one report: the token found in the request's params, the
notification built, and a writer that drops it. A request that reports nothing pays nothing for it. On HTTP its `Accept`
is read only at the first report.

## [3.7.5] — 2026-11-02 · Cancellation

**A client can cancel a request in flight.** On `http`, `streamable` and `bridge`, for MCP 2026-07-28, a client that
closes its connection has cancelled its request, which the spec says the server MUST treat as one. On stdio and a Unix
socket the client sends `notifications/cancelled` naming the request. A handler runs to its end on the thread that called
it, so cancelling one is cooperative. A handler that runs long asks `bote_cancelled()` as it goes. Once that says 1, bote
sends nothing more for the request, its response included, and audits a `tools/call` as cancelled. bote's own
`web_fetch` and `web_search` ask while they wait on their child, and end it.

**Five defects in 3.7.4 land ahead of it**, from an independent review of that release. The worst let a token without a
resource's scope subscribe to its URI, or to a prefix above it, and hear every change, which also gave away the URIs
behind a scoped template. 3.7.4's fix for a change made while a listen opened held on stdio, unix and WebSocket, and not
on HTTP. See the correction notice in 3.7.4.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

> ⚠ **Corrected at 3.7.6.** Five claims below were not true, found by an independent review of this release.
> - "Never more than it may read" failed for a resource registered again with a scope the token lacks: the stream had been
>   judged by the old registration and went on being told.
> - "On stdio and a Unix socket the client sends `notifications/cancelled` naming the request" missed a batch's later
>   request, once an earlier request's check had read past its cancellation.
> - `bote_cancelled()` was not "0 off a handler's own thread" on a one-loop server, where every thread reaches the
>   process's one request state. A handler's helper therefore ran the transport's check beside the loop.
> - The line scanner did not read a line as the codec does: it honored a cancellation whose `jsonrpc` the codec refuses.
> - On a Unix socket an ask is two polls, a read and a look, not "one poll".
>
> Each is fixed in 3.7.6.

### Added

- **`bote_cancelled()`** (core, `src/protocol.cyr`): 1 when the client has cancelled the request the calling thread is
  serving. Each call asks the connection once, a poll or a look at what has arrived, and a 1 stays 1.
  - The transport answers through `bote_req_set_cancel(fp, ctx)`, set after `bote_req_begin`.
  - The codec names the request (`bote_req_set_request_id`).
  - `bote_req_cancel_live()` and `bote_req_was_cancelled()` read the state.
  - The request state grows from 40 to 80 B: the check, its context, the request's id, whether it was cancelled, and
    the claims a listen was judged with.
- **`bote_arena_mark(a, m)` / `bote_arena_release(a, m)`** (core): `bote_req_mark` for any bote arena, and nothing on an
  allocator that is not one.
- **`codec_lines_cancel_a(a, p, n, id, done)`** (core): for a transport whose messages are lines, 1 when a complete line
  of what it has read cancels the request `id`.
  - A line is copied and read only when it spells the method plainly or holds an escape. Each line's reading is given
    back once it is read.
  - The lines stay where they are, to be served in their turn.
  - `bote_id_key_a`, the id as a value, moves here from `src/listen.cyr`.
- **`bote_conn_gone(fd, half)`** and **`bote_conn_readable(fd)`** (`src/conn.cyr`): poll(2) with no wait. With `half`, a
  client that closed its sending side counts as gone, which is all TCP shows of a close. 0 on agnos, which has no poll.
- **On the transports:**
  - `bote_http_serve_modern` asks `bote_conn_gone(cfd, 1)`.
  - stdio reads in what stdin has ready, without waiting, while there is room, and looks through the lines past the
    message, each once. An end of stdin it reads is no cancellation: a client may send its last request and close
    stdin before it reads the answer. The loop remembers that end, since a terminal's is read once.
  - A Unix socket does the same, and a client that closed both ways has cancelled. One that only shut its sending
    side still gets its answers. A line its check read is served in the same pass.
- **`BOTE_FETCH_CANCEL_MS`** (100, `src/fetch.cyr`): a request whose client can cancel it asks that often while it waits
  on a child, and a cancelled one ends the child (`bote_fetch_why()` is `"cancelled"`).
- `fuzz/cancel_lines.fcyr`: the line scanner against a reference drawn from the same grammar.

### Changed

- A request a handler found cancelled is answered with nothing on every transport. A cancelled `tools/call` is audited
  and published as a failure with the error `"cancelled"`, and its DEBUG line says `status=cancelled`. A request whose
  handler never asks is answered: a cancellation that arrives after the work is done may be ignored.
- The legacy path on HTTP never asks: 2025-11-25 says a disconnect is no cancellation, and its `notifications/cancelled`
  is a POST of its own, which bote does not match to the request (roadmap).
- `web_fetch` and `web_search` log a cancelled request at TRACE, not WARN: the request's own line reports it.
- A listen stream's slot grows from 312 to 328 B: whether its listen was made with claims, and the dispatcher that judged
  it. Its URI room takes a flag byte per URI.

### Fixed

From the review of 3.7.4:

- **A token without a resource's scope heard of every change to it.** `resourceSubscriptions` was not judged by the scopes
  `resources/read` enforces. A token without them was honored the URI, or a prefix above it such as `sp:/`, and told of
  each update, which also named the URIs a scoped template serves and no list shows. Now a listen's URIs are judged with
  its claims by what would serve them, and judged again as the stream keeps them. A stream made with claims is told of an
  update to a URI that requires a scope only when it holds that URI exactly and the scope held when it listened. A
  stream keeps no token, so it is not told of one through a prefix, nor of a URI whose scope came after it listened:
  less than happened for a token that has the scope, never more than it may read (roadmap).
- **On HTTP a change made as a stream parked could still reach nobody.** The park counted the stream open with a plain
  store and then read the count of changes. A change counted itself and then read the open streams. x86 let each read pass
  the other side's write, so neither saw the other. The review's probe lost 112 to 266 changes in 400,000. The park counts
  with an atomic add, and both sides fence between the write and the read. A ListenLine was sound: its acknowledgement
  is written by the thread that counted the listen open.
- **The thread that writes the streams kept every frame of a pass on its arena**, for the life of the process: 38.3 MB
  for 256 streams told of 16 updates of a kilobyte URI, and 13.8 MB for one overflow of 256 streams holding 32 URIs. Each
  frame's bytes now go once it is sent (`_bl_send_one`).
- **A connection's queued updates outlived it.** A ListenLine's queue and its overflow survived a detach, so the next
  connection on a Unix socket or a WebSocket was told of updates made before it listened.
- **`"\u0000a"` and `"\\u0000a"` were one id**: a value holding a NUL fell back to its spelling, which was another id's
  value. Such a value is now keyed by its spelling, apart from both.

### Security

- The scope fix above. Through 3.7.4, on a server whose resources require scopes, any token that could open a listen could
  learn when each protected resource changed, and the URIs behind a scoped template.
- The arena fix above, a memory bound a client with `BOTE_LISTEN_MAX` streams could push to gigabytes at the bound of 65536.
- Cancellation lets a client end its own request early, and no other: by closing its own connection, or with a
  notification on it. The check reads only what has already arrived, within the transport's buffer, each line once.

### Tests

- **5,067 assertions, from 4,938**, on x86_64 and under `qemu-aarch64`.
  - **`tests/bote_cancel.tcyr`**, 96, new:
    - `bote_cancelled` and the check: none set, no request served, a yes that stays yes without asking again, the
      context and the id, a begin clearing them, and a thousand asks that allocate 4 KB each leaving the arena where it was.
    - The codec: no answer for a request found cancelled, the id it names, the call audited as cancelled, a handler that
      never asks answered, the legacy era, one element of a batch, and the DEBUG line.
    - `codec_lines_cancel_a`: ids by value through `\u` and `\/` escapes, a number that is not the string, the words
      inside another message's arguments, a request by that name, invalid JSON, a batch, a line not yet whole, the
      method spelled with escapes, 0 heap bytes for a hundred reads of 64 long lines, and one arena chunk for them.
    - The real loops on threads: stdio and a Unix connection with a cancellation sent after the handler began and with the
      request, one naming another, ids by value, an end of stdin and a half-close answered, a close cancelling; the three
      HTTP handlers with a client that waits, half-closes and closes, the legacy path never asking; a fetch's child ended
      at the third ask and reaped; two streamable workers, one cancelled and the other answered in full.
  - **`tests/bote_listen.tcyr`**, 224, from 217:
    - A park raced against a change and against an update, 300,000 rounds each, the change's delay swept across the
      park's write and read. 3.7.4's plain store lost 8 to 69 a run, every run; none now.
    - A scoped resource's update on HTTP. 16 streams told of 16 kilobyte URIs on one chunk of the arena, where the pass
      took 2.4 MB.
  - **`tests/bote_listen_line.tcyr`**, 186, from 160: a resource's scopes for its updates, with a token without the
    scope, one with it (exact URIs only: not through a prefix, not for a scope added since, not a deeper URI that needs
    another scope) and none; an overflow; the judgement again as a stream keeps its URIs. A connection's queue and
    overflow gone at detach. NUL ids by their spelling.
- **`fuzz/cancel_lines.fcyr`**, the fourteenth harness: 30,000 rounds of up to six lines from a grammar against a reference.
  Before release it found the scanner's first look skipping `notifications\/cancelled`, a spelling the codec reads as the
  method. `fuzz/http_era.fcyr` and `fuzz/oauth_metadata.fcyr` include `src/conn.cyr`, which `http_era.cyr` now calls.
- **52 mutants, 46 killed.** Of the rest, two found code that did nothing, which is gone. One forgets an end of stdin a
  check read, which only a terminal shows, since a pipe's end is read again. Three remove a fence after an atomic add,
  which on x86 is already a full barrier; they are there for aarch64. Three first survived, and tests added for them
  kill them:
  - 3.7.4's plain store in the park, until the race test swept the change's delay across the window. Lockstep alone never
    lost one.
  - A NUL id keyed as the empty string.
  - A scoped URI held as a prefix of a deeper one that needs another scope.
- **The official client** passes 434 checks. The suite is where 3.7.4 left it.

### Performance

**The request path costs what it did.** Interleaved A/B against 3.7.4, 15 rounds pinned to one core. Minimums, in ns:

| Bench | 3.7.4 | 3.7.5 | Delta |
|---|---|---|---|
| `codec_process_message` | 3,916 | 3,951 | +0.9% |
| `codec_process_message_threaded` | 4,215 | 4,228 | +0.3% |
| `dispatch_tools_call` | 2,046 | 2,062 | +0.8% |
| `http_era_modern` | 10,946 | 11,075 | +1.2% |

Rows that run none of this patch's code moved further (`jsonx_get_str_flat` +4.9%, `dispatch_tools_list` +2.9%,
`validate_compiled_nested` −3.5%), which is layout. To tell the two apart, the same tree with the codec's and the
dispatcher's cancellation lines taken out was interleaved against it, 11 rounds, layout held:

| Bench | Without | 3.7.5 | Delta |
|---|---|---|---|
| `codec_process_message` | 3,925 | 3,948 | +0.6% |
| `codec_process_message_arena` | 3,917 | 3,909 | −0.2% |
| `codec_process_message_threaded` | 4,243 | 4,229 | −0.3% |
| `dispatch_tools_call` | 2,072 | 2,064 | −0.4% |

The first build looked the request state up three times a message, and cost 1.2% to 1.7% against that control, the most
on a worker, where the state is a thread-local lookup. The codec now looks it up once and reads the stream word from it,
which saves the lookup `bote_req_stream()` made. The dispatcher reads the cancelled flag only for a sink.

Two new rows: `cancel_lines_scan`, 2,915 ns, for four lines past the message, one a cancellation of another request read
in full; and `cancel_check_socket`, 541 ns, one poll, which is what a handler pays for each ask on HTTP and a Unix socket.
A handler asks between steps of its work, not within them.

## [3.7.4] — 2026-11-02 · Resource updates

**`subscriptions/listen` is whole: `resourceSubscriptions` and `notifications/resources/updated`.** A client names the URIs
it wants to hear of, and the acknowledgement lists those bote honors. An embedder says a resource changed with
`dispatcher_resource_updated(d, uri)`, from any thread, and every listen subscribed to that URI, or to one it falls under,
is told, on every transport. `server/discover` advertises `resources.subscribe` where listen is carried. The official
client hears an update on all four of its transports.

**Seven defects in 3.7.3 land ahead of it**, from an independent review of that release. The worst stopped a WebSocket's
changes once its client pinged. See the correction notice in 3.7.3. **And a change made while a listen was opened, which
could reach nobody**, on every carrier since it was added: the aarch64 sweep lost one in 3 runs of 3.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

> ⚠ **Corrected at 3.7.5.** Four claims below were not true, found by an independent review of this release. "A change
> made while a listen was opened could reach nobody" was fixed on a ListenLine and not on HTTP: the park counted its stream
> open with a plain store and then read the count of changes, and x86 let the read pass the store, so a change made at
> that instant was raised for no stream and owed to none (112 to 266 in 400,000 in the review's probe). "An update
> allocates nothing" held for the heap and not for the writing thread's arena, which kept every frame of a pass: 38 MB for
> 256 streams told of 16 kilobyte URIs. "The acknowledgement lists those bote honors" honored URIs a token could not read:
> no resource's or template's required scopes were judged, so a token without them heard of every change, by name or
> through a prefix, and learned the URIs behind a scoped template. And listen ids "compared by value" took `"\u0000a"` and
> `"\\u0000a"` for one. The review also found a connection's queued updates outliving it. Each is fixed in 3.7.5.

### Added

- **`dispatcher_resource_updated(d, uri)`** (core): the resource at `uri` changed. Nothing keeps `uri`.
- **What a listen may ask for** (core):
  - `BOTE_LISTEN_UPDATES`, honored where the dispatcher has resources. A URI is honored when it is not empty, holds no
    NUL and is at most `BOTE_LISTEN_URI_MAX` (1024) bytes decoded, once each, while there is room: `BOTE_LISTEN_URIS`
    (32) of them in `BOTE_LISTEN_URI_ROOM` (4096) bytes.
  - `bote_listen_ack_uris_a`, the acknowledgement that lists them, decoded and escaped again.
- **The update hook** (`src/events.cyr`, core): EventSink grows to 32 B, and `event_sink_with_updated` /
  `event_sink_updated` carry a URI with no payload, so nothing is built or kept for an update. `bote_conn_serial_sinks`
  keeps it. A sink with no hook, majra's and the legacy drain's among them, hears nothing.
- **Updates in a hub and a ListenLine** (`src/listen.cyr`). A slot (312 B) keeps the URIs its acknowledgement lists, in a
  room made the first time the slot needs one and kept for the next. Each hub and line (160 B) queues an update's URI,
  `BOTE_LISTEN_QUEUE` (16) of them, each once, under a lock. The thread that writes its streams tells each subscription of
  every URI it covers: the same URI, one followed by `/`, `?` or `#`, or anything after one that ends in `/`. It builds
  each frame with `bote_json_escape` on the carrier's own arena, rewound each pass. A URI too long to queue, or one
  arriving with the queue full, overflows it, and each subscription is then told of every URI it holds: more than
  happened, never less. With none open an update is kept for nobody.
- `bote_listen_line_broken(c)`: 1 once a write to the connection failed, which each transport now drops on.
- `bote_update_resource` in the conformance fixture: a real `dispatcher_resource_updated`, which the official client calls.

### Changed

- `server/discover` says `resources.subscribe` where listen is carried (`_build_capabilities_sub`). The legacy `initialize`
  never does, having no `resources/subscribe`.
- A listen with a null id is `-32600`: MCP's ids are never null, and a subscription's id is how it is cancelled.
- A WebSocket closed at its idle deadline logs a DEBUG line, as the Unix socket's does.

### Fixed

From the review of 3.7.3:

- **A WebSocket's changes stopped once its client pinged.** After answering a ping the loop read on for the next frame,
  where it should have waited on the listen too, so a client that keeps its socket alive with pings heard nothing until it
  sent a request. `_bote_ws_recv` returns after a control frame (`BOTE_WS_CONTROL`).
- **A listen whose acknowledgement could not be written left the connection served**, and the next message's handler ran
  for a client already gone, or after a WebSocket frame sent in part. The line is marked broken, and the transport drops it.
- **A listen dispatched after a connection ended, on the same thread and with no new begin, crashed** on the line's writer
  of 0. Detaching clears the stream word, and an open with no writer is refused.
- **On stdio, a client that closed stdout killed the process with SIGPIPE.** stdio ignores it now, as sandhi's serve loops do,
  and a write that fails ends the loop with a WARN.
- **Listen ids were compared byte for byte**, so `"a/b"` and `"a\/b"` were two subscriptions, and a cancel spelled another way
  ended nothing. They are compared by value, a string decoded and kept apart from a number.
- **Judging a WebSocket's token while changes went out grew the request arena**, which nothing rewinds between frames: about
  87 bytes a change with the cache off. The judgement is in a mark and its release.
- **A hub-thread start that failed could, on its second close, end a stream that a later start served.** A start under way is
  waited for now, and there is one close, by the start that failed.

Found here:

- **A change made while a listen was opened could reach nobody.** A client lists after its acknowledgement, so a change
  before it is in what the client lists, and one after must be told. One in between found no subscription open to raise it
  for, and was kept for nobody. Under `qemu-aarch64` a change the instant a Unix client read its acknowledgement was lost in
  3 runs of 3; the 3.7.3 roadmap's 400 clean rounds were on x86. A ListenLine now counts the subscription open before it
  writes the acknowledgement, which only its own loop follows with changes, and takes it back if the write fails. On HTTP
  the worker writes the acknowledgement and the hub thread the changes, so the hub thread may not write first: each hub
  counts every change and update (`seen`, +152), whether or not a stream is open, the worker reads the count before it
  sends anything, and a stream parked with the count moved since is owed every kind it honors (`BOTE_LISTEN_OWED`, a slot's
  +304). The hub thread then tells it, of every URI it holds for an owed update. More than happened, never less.

### Security

- **What a listen keeps of its URIs is bounded**, at most 32 in 4 KB in a room per slot, at most `BOTE_LISTEN_MAX` rooms in
  all, and an update allocates nothing. A queue that overflows tells too much rather than too little.
- The 3.7.3 fixes above. The worst a client could do with them was crash a thread that dispatched without beginning, which
  bote's own transports never do.

### Tests

- **4,938 assertions, from 4,842**, on x86_64 and under `qemu-aarch64`.
  - **`tests/bote_listen_line.tcyr`**, 160, from 97:
    - The acknowledgement's honored URIs: decoded and escaped again, once each, never empty or with a NUL. The bounds:
      32 of 40, what fits the room, none past the longest.
    - What an update reaches: the same URI, a path, a query and a fragment under it, anything under a trailing `/`, never a
      URI that only begins the same. An escaped URI escaped where it goes out.
    - The queue: a repeat told once, overflow by count and by a URI too long, kept for nobody with none open, passed on
      to the next sink, kept through the workers' serial sinks.
    - `server/discover`'s `subscribe` on the line and not off it, and never in the legacy `initialize`, its `listChanged`
      on or not.
    - 0 heap bytes for a listen with URIs and its cancel once its slot has a room, and for a hundred updates of a
      kilobyte URI.
    - Four threads updating beside the real stdio loop, every line whole.
    - From the review: a WebSocket that pings and then hears a change with no frame of its own, and its arena unmoved by
      fifty judgements of 64 KB between frames. A failed acknowledgement dropping each transport before its next message.
      The stream word gone at detach, and an open with no writer refused. stdio surviving a closed stdout. Ids by value,
      and a null id refused.
    - A change made by the writer while it writes the acknowledgement, told after it, and a failed acknowledgement
      holding no slot for its id.
  - **`tests/bote_listen.tcyr`**, 217, from 184:
    - An HTTP stream and a streamable one told each of its own URIs, a sub-resource, overflow by a URI too long.
    - A hundred updates of a kilobyte URI delivered with 0 heap bytes, and four threads updating while a stream reads.
    - A hub start under way waited for, then started or taken over.
    - The window: a change and an update counted with none open; a stream parked with the count moved told what it honors
      and every URI it holds, once; one parked with the count unmoved owing nothing, nor inheriting what a stream before
      it on the slot was owed; and through `bote_listen_http`, a
      client that changes the tools while a head longer than its socket takes keeps the worker writing, then hears it.
- **`fuzz/listen_filter.fcyr`**'s reference counts the updates bit.
- **70 mutants, 69 killed.** The one left was an empty-URI check the decoded-length check already makes, and it is gone.
  Four first survived on the tests: an arena check that spent too little to outgrow the chunks earlier cases made, a
  mutant I had specified wrong, and two of the window's 16, an owed mark the next stream on a slot inherited and a failed
  acknowledgement that kept its slot.
- **The official client** passes 434 checks, from 422, hearing an update on all four of its transports. The suite is where
  3.7.3 left it.

### Performance

**The request path costs what it did.** Interleaved A/B against 3.7.3, 15 rounds pinned to one core, run after the
window's fix. Minimums, in ns:

| Bench | 3.7.3 | 3.7.4 | Delta |
|---|---|---|---|
| `codec_process_message` | 3,956 | 3,927 | −0.7% |
| `codec_process_message_threaded` | 4,265 | 4,224 | −1.0% |
| `http_era_modern` | 11,047 | 11,015 | −0.3% |
| `listen_filter` | 3,697 | 3,751 | +1.5% |

`listen_filter`'s dispatcher has no resources, so its URIs are not read. Rows that run none of this patch's code moved
further (`dispatch_tools_list` −5.5%, `jsonx_get_raw_nested` −2.9%, `codec_parse_request` +2.5%), which is layout. The
window's fix adds one atomic add to a change or an update and one atomic load to opening an HTTP stream, neither on a
benched path.

## [3.7.3] — 2026-11-02 · Listen everywhere

**`subscriptions/listen` on stdio, unix and WebSocket**, where the connection is itself the stream. A listen's
acknowledgement is its first message on the connection, each change it asked for goes out between the replies to other
requests, from whatever thread made it, and `notifications/cancelled` naming the listen's id ends it. Every transport now
carries listen, and `listChanged` is advertised where, and only where, it is delivered.

**The official suite's listen checks all pass**, on `http` and `streamable`: the acknowledgement first, the id stamped, the
filter honored, and a tool change and a prompt change delivered. They had probed 3.7.2's streams all along, but needed the
two hooks the suite calls to make a change, which the fixture now has. The official client opens a listen on all four of
its transports and hears a change on each.

**Six defects in 3.7.2 land ahead of it**, from an independent review of that release, none of them severe. See the
correction notice in 3.7.2, which also corrects a claim this release does not fix: over TCP a client that stops reading
keeps its stream for days.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

> ⚠ **Corrected at 3.7.4.** Two claims below were not true, found by an independent review of this release. "Each change
> it asked for goes out between the replies to other requests, from whatever thread made it" failed on a WebSocket whose
> client had pinged: after the pong the loop read on and waited on nothing else, so a client that keeps its socket alive
> with pings heard no change until it sent a request. And "every parked stream ends, before and after the next caller may
> try again" overreached: the second close could end a stream that a later, successful start was serving. "A cancel matches
> the listen's id byte for byte" was true and wrong, since JSON-RPC compares ids by value. The review also found a failed
> acknowledgement leaving its connection served, a crash for a listen dispatched after a connection ended, SIGPIPE killing
> stdio, and the request arena growing while a WebSocket's token was judged between frames. Each is fixed in 3.7.4. The
> first claim also failed for a change made while a listen was opened, which found no subscription open and reached nobody:
> the aarch64 sweep lost one in 3 runs of 3, where the roadmap below had 400 clean rounds on x86. Fixed in 3.7.4 too.

### Added

- **A ListenLine** (`src/listen.cyr`), for a connection that is the stream:
  - `bote_listen_line_new(max)`; `bote_listen_line_attach(c, d, write_fp, ctx)` and `_detach`, around each connection.
  - `bote_listen_line_begin(c)`, which a transport calls in place of `bote_req_begin()`.
  - `bote_listen_line_wait` / `_await(c, fd, deadline)`, which poll the connection and the line's wake pipe together,
    and write the changes meanwhile; `bote_listen_line_deliver`.
  - Its middle is laid out as a hub's, so `bote_listen_sink` serves both. An id already open is `-32600`; past
    `BOTE_LISTEN_MAX` a connection's listens get `-32603`; a cancel matches the listen's id byte for byte.
- **A BoteStream** (`src/protocol.cyr`, core): `bote_stream_new(open_fp, cancel_fp, ctx)`, `bote_stream_open_a`,
  `bote_stream_cancel_a`. The request state's stream is 0, 1 for an HTTP transport, or a BoteStream, and the codec hands a
  single message's accepted listen and any `notifications/cancelled` to it, so the core knows the shape and not what is
  behind it. A batch carries none.
- `transport_stdio_with_listen(line)`, `transport_unix_with_listen(line)` and `ws_config_with_listen(c, line)` (WsConfig grows
  to 80 B). The binary gives each of the three a line, `BOTE_LISTEN_MAX` subscriptions a connection, logged at INFO.
- stdio's loop is `_stdio_serve(d, in_fd, out_fd)`, which a test drives over pipes.
- `test_trigger_tool_change` and `test_trigger_prompt_change` in the conformance fixture: the suite's hooks, each a real
  registration. The fixture wires listen on every mode.

### Changed

- **`subscriptions/listen` and `server/discover`'s `listChanged` follow the message's transport** (`_dsp_listen_here`): an
  HTTP transport with a hub on the dispatcher, or a connection carrying a BoteStream.
- **With a line, the Unix and WebSocket idle deadlines count from what the client last sent**, not from the changes bote sent
  it; the WebSocket's token is judged again before a change goes out, as before a frame is served (`_ws_token_holds`).
- The WebSocket frame loop is `_ws_frames`.
- **`BOTE_LISTEN_MAX` is held to 65,536** (`BOTE_LISTEN_MAX_BOUND`) with a WARN: every slot is made at startup.
- `tests/bote_transport_unix.tcyr` and `tests/bote_ws.tcyr` include `http_era.cyr` and `listen.cyr`, which the transports now
  reach.

### Fixed

From the review of 3.7.2:

- **A listen stream was acknowledged before its socket was held**, and cut off, with no word in the log, when the process had
  no descriptor to spare. The socket is held first, with `F_DUPFD_CLOEXEC`, and one that cannot be is a `-32603` and a WARN.
- **A stream judged its filter again after its acknowledgement had**, so a prompt or resource registered between the two made
  the acknowledgement say nothing honored and the stream stay open, then hear a change it had been told it would not. A
  stream honors what its acknowledgement said (`_bl_ack_mask`).
- **`server/discover` advertised `listChanged` on a transport that answers listen with `-32601`**, when one dispatcher is served
  on HTTP and another transport.
- **A hub thread that could not start ended its own stream and not one another worker parked meanwhile**, which was left with
  nobody to serve it. Every parked stream ends, before and after the next caller may try again.
- **The parked sockets and the wake pipes were inherited by any program an embedder's runner executes.** They are close-on-exec.
  A child forked without an exec, as bote's own fetch is, still holds them until it exits.
- **A hub that could not be made, or a `BOTE_LISTEN_MAX` past what memory allows, was silent**: listen answered `-32601` with
  no line. It logs an ERROR now.

### Security

- **The listen fixes above**, none of which a client could turn into more than a stream cut off or held open.
- **Over TCP a client that stops reading is not seen for days**, and one that reads its keepalives never is, so one client can
  hold every stream slot. `BOTE_LISTEN_MAX` bounds memory and sockets, not who holds them. A stream's age or a bound per
  client is an open decision on the roadmap; the TypeScript SDK's server has neither.

### Tests

- **4,842 assertions, from 4,730**, on x86_64 and under `qemu-aarch64`.
  - **`tests/bote_listen_line.tcyr`**, 97, new:
    - A line through the codec: the acknowledgement exactly, a number id, an id already open, the capacity, nothing
      honored, the id bound, a filter that is none, legacy, a batch, a message not begun on the line.
    - `server/discover` on the line and off it.
    - Changes to each subscription by what it asked for, in slot order. A cancel by a string id, a number id and a whole
      id, never by its start.
    - A change waiting when the connection goes, a failed write, detach and attach.
    - 0 heap bytes for an open and its cancel, and for each refusal with 60 KB in it.
    - Waiting on two at once: the deadline, input, a change, a closed peer.
    - The real stdio loop on a thread over pipes, with a change from another thread written while the client is quiet,
      a cancel, a listen still open at EOF, and four threads changing the tools beside fifty replies, every line one whole
      message.
    - A Unix connection, its idle deadline from the client's last line, changes or not.
    - A WebSocket: frames, a token that stops holding closing the connection with 1008 before a change goes out, and the
      idle deadline from the last frame.
  - **`tests/bote_listen.tcyr`**, 184, from 169: a socket that cannot be held, close-on-exec on a stream and the wake pipe,
    an acknowledgement of nothing ending the stream whatever the filter, a hub thread that does not start ending another
    worker's stream, and `listChanged` following the transport.
- **38 mutants, all killed.** Four first survived, each on a missing case: a change waiting at detach, a number id matched by
  its start, a stdio listen open at EOF, and a test that restored the duplicate's command by hand.
- **`scripts/transport-smoke.sh`**: a listen over stdio, unix and ws, its cancel, and `BOTE_LISTEN_MAX` held to its bound.
- **The official suite** passes 133 checks on 2026-07-28, from 128, every listen check among them, with the same scenarios
  failing; the official client passes 422 checks, from 410.

### Performance

**The request path costs what it did.** Interleaved A/B against 3.7.2, 15 rounds pinned to an idle core. Minimums, in ns:

| Bench | 3.7.2 | 3.7.3 | Delta |
|---|---|---|---|
| `codec_process_message` | 3,940 | 3,945 | +0.1% |
| `codec_process_message_arena` | 3,886 | 3,917 | +0.8% |
| `codec_process_message_threaded` | 4,225 | 4,264 | +0.9% |
| `http_era_modern` | 11,038 | 11,024 | −0.1% |
| `listen_filter` | 3,715 | 3,698 | −0.5% |

Every message through the codec now reads the request's stream word once. Rows that run none of this patch's code moved
further (`validate_compiled_nested` +4.9%, `jsonx_get_raw_nested` +4.5%, `codec_parse_request` −3.1%), which is layout.

## [3.7.2] — 2026-11-02 · Listen streams

**`subscriptions/listen` on `http`, `bridge` and `streamable`.** It is MCP 2026-07-28's way to hear of changes, in place
of the held-open GET. A client names the changes it wants, each opt-in, and its response stays open as an event stream:
first `notifications/subscriptions/acknowledged`, naming what bote honors, then one notification for each change it asked
for, and a `: keepalive` comment every 15 s. Every message carries the request's id under
`io.modelcontextprotocol/subscriptionId`. bote honors `toolsListChanged`, `promptsListChanged` and
`resourcesListChanged`, each only where it has that list. One thread holds every stream, so a stream holds no worker, nor
a one-loop server. A client that closes, sends anything, or stops reading loses its stream. stdio, unix and WebSocket answer
`-32601` until 3.7.3.

**Eight defects in 3.7.1 land ahead of it**, from an independent review of that release. The worst ran a tool that requires a
scope for a token without it, when the tool was registered while the call was in flight. See the correction notice in
3.7.1. And CI had never run three of the test files: `bote_fetch` (3.6.5), `bote_jwks` (3.6.6) and `bote_threads` (3.7.0).

The handler ABI and the stated minimum cyrius are unchanged.

> ⚠ **Corrected at 3.7.3.** Three claims below were not true, found by an independent review of this release. "A client that
> stops reading loses its stream" holds on a socketpair and not over TCP: the client's receive window takes about a megabyte
> first, which at one keepalive every 15 s is days, and a client that reads its keepalives is never dropped, so one client can
> hold every slot. That is an open decision on the roadmap, not fixed in 3.7.3. "`server/discover` advertises `listChanged`
> once a carrier is set" advertised it on a transport that shares the dispatcher and answers listen with `-32601`. And "with
> nothing honored the stream ends at once" missed a prompt or resource registered between the acknowledgement and the stream's
> own look at the filter, when the acknowledgement said nothing and the stream stayed. A stream acknowledged before its socket
> was held was also cut off, with no word in the log, when the process had no descriptor to spare. All but the first are fixed
> in 3.7.3.

> ⚠ **Corrected at 3.7.4.** "Then one notification for each change it asked for" missed a change made after the
> acknowledgement was built and before the stream was parked, which reached the streams already open and not that one.
> 3.7.3 listed it as an open decision; 3.7.4 fixes it.

### Added

- **`src/listen.cyr`**, in the full bundle after `oauth_metadata.cyr`:
  - `bote_listen_hub_new(max)` and `bote_listen_hub_with_keepalive(h, ms)`.
  - `bote_listen_sink(h, next)`: the EventSink that carries the dispatcher's changes to the streams and passes every event
    on to `next`.
  - `bote_listen_open(h)` and `bote_listen_http(...)`.
  - The thread that answers a listen writes the head and the acknowledgement, parks a duplicate of the socket in one of
    the hub's slots, and returns. One hub thread, started by the first stream, polls every parked socket and a wake pipe.
  - A change is a bit, so changes of one kind coalesce. A change with no stream open is kept for nobody. Nothing per stream
    is on the heap, and the id, at most 256 bytes, is copied into its slot.
  - On agnos there is no hub, and the method is not found.
- **In the dispatcher** (core):
  - `dispatcher_set_listen` / `dispatcher_listen`; the Dispatcher grows to 112 B.
  - `dispatcher_listen_mask_a`: the honored subset, or -1 for a filter that is none. `resourceSubscriptions` is checked as
    a list of strings and never honored, since `resources.subscribe` is not advertised.
  - `bote_listen_ack_params_a`.
  - `BOTE_LISTEN_TOOLS`, `_PROMPTS` and `_RESOURCES`.
  - In the modern router, `subscriptions/listen` is `-32601` with no carrier or on a transport that cannot hold the stream,
    and `-32602` for a filter that is not one.
  - `server/discover` advertises `listChanged` for tools, prompts and resources once a carrier is set.
- `TOPIC_RESOURCE_REGISTERED`, published when a resource or a template is registered. streamable's drain path sends it as
  `notifications/resources/list_changed`, and a legacy `initialize` with notifications on now advertises `resources`
  `listChanged` beside tools and prompts.
- **In the request state** (core, 40 B):
  - `bote_req_set_stream` / `bote_req_stream`, set by a transport that can hold a stream open around its dispatch and
    cleared by `bote_req_begin`. A dispatcher one embedder serves on HTTP and on stdio answers listen on stdio as a method
    it does not have.
  - `bote_req_mark` / `bote_req_release`: give back what a function put on the arena once it has copied what it keeps.
- `registry_validate_def_a`, `prompt_registry_serving` and `resource_registry_serving`: what a call is judged by and what
  serves it, from one look.
- **`BOTE_LISTEN_MAX`** (256; 0 turns listen off) and **`BOTE_LISTEN_KEEPALIVE_MS`** (15000) in the binary, for `http`,
  `bridge` and `streamable`, logged at INFO.

### Changed

- **A `tools/call`, `prompts/get` and `resources/read` are judged and served by one look at the registries**: the tool, its
  handler and the version asked for (`_dsp_tool_serving`); the prompt and its handler; the handler that serves a URI and the
  scopes it requires.
- **A thread waiting on the registries' lock yields every 64 turns** (`BOTE_RW_SPINS`).
- **sigil's P-256 tables are built once for the process** (`_jwt_p256_warm`), not for every EC key of every set that changes.
- **CI runs every test file in `tests/`, by glob.** The list it kept had missed three since each was added.
- **`scripts/version-bump.sh` rewrites `docs/spec-compliance.md`'s `Bote Version` header** and fails if it does not name the
  new release. CI checks that header, and it was missed by hand at 3.3.14 and again at this release.
- SECURITY.md said scopes gate `tools/call` only and that a WebSocket checks its token once. Both stopped being true at
  3.6.6, and the section above them already said so.

### Fixed

From the review of 3.7.1:

- **A call could run a scoped tool for a token without the scope**, while the tool was registered: the check found no tool
  and so no scopes, and validation and the handler then found it. A probe ran 3,198 calls of 69,361 that way. A prompt or a
  resource replaced meanwhile was judged by one registration and served by another.
- **Loading an EC key kept the loading thread one of sigil's 64 crypto lanes for life** (since 3.6.2). bote's curve check
  called `_ecv_init`, the verify's tables, which outside a scope of sigil's takes a lane for good. At startup that thread is
  the main one, so 64 workers shared 63, and a worker that refreshed the keys and exited kept its lane. sigil's
  `pt_is_on_curve` makes what it needs itself, so the call is gone; the verify's tables are the warm's, whose sign and
  verify open their own scopes.
- **P-256's tables were built for every EC key of every set that changed** (3.7.1): a sign and a verify each, 13 ms. 256
  keys took 5.6 s, inside the worker answering a request.
- **Two threads registering a dispatcher's first prompt or resource each made a registry**, and the one stored first was lost
  with what was registered in it. A probe of 3,000 dispatchers lost a prompt on 2,927. It is made by compare-and-swap.
- **`fs_read`'s buffer stayed on the arena for the rest of its message** (3.7.1): 64 KB a batch element, up to 6.5 MB for a
  batch of a hundred on each thread's arena, and 128 KB a call where nothing rewinds the arena. It is released once the
  result is made.
- **A count from the environment counted leading zeros toward saturating**: `BOTE_WORKERS=0000000000` started 64 workers,
  not the one loop.
- **`registry_deprecate` wrote a registered tool with no lock**, so a list built beside it could size the reply without the
  message and then write it in.
- **The registries' lock spun without yielding.** With more workers than cores, readers spun through whole scheduler slices
  while the writer they waited for could have run.

### Security

- **The scope bypass above.** It needs a scoped tool, prompt or resource registered while a client with too few scopes calls
  it, which bote's binary never does; an embedder that registers while serving was exposed.
- **Listen streams are bounded**: at most `BOTE_LISTEN_MAX` at once, `-32603` "Subscription limit reached" past it, the
  TypeScript SDK's words. Each holds one socket and keeps nothing on the heap, and the hub thread never waits on a client: a
  frame the socket cannot take whole ends that stream. A stream's token is checked when it opens and not again; it carries
  only that a list changed, never what is in it.
- With no thread of bote's holding a crypto lane, `BOTE_WORKERS`' bound of 64 again means that good ES256 tokens are never
  turned away.

### Tests

- **4,730 assertions, from 4,536**, on x86_64 and under `qemu-aarch64`.
  - **`tests/bote_listen.tcyr`**, 169, new:
    - The filter's every shape, and the honored subset with and without each list.
    - The router: no carrier, a transport that cannot hold a stream, a legacy request.
    - Real streams through `_http_handle`, `_strm_handle` and `_bridge_handle` over socketpairs, read with a deadline: the
      head, the acknowledgement for a string id and a number id, and CORS on the bridge's.
    - Each stream told only the kinds it asked for; a closed client and one that writes, gone, and their slot reused.
    - The capacity, nothing honored ending at once with its result, a 256-byte id taken and a 257-byte one refused, a
      notification's `202`.
    - Keepalives, and a client that stops reading losing its stream.
    - 0 heap bytes per stream on all three transports and for each refusal carrying 60 KB.
    - Four threads opening 200 streams beside a writer, while one stream stays open throughout.
  - **`tests/bote_threads.tcyr`**, 81, from 74:
    - Writers changing what serves five names, beside a caller for each, whose token lacks the scope: no guarded handler runs,
      and no strict handler sees arguments another registration's schema passed.
    - Four threads registering the first prompt, resource and template on each of 1,000 dispatchers.
  - **`tests/bote_jwt.tcyr`**, 491, from 483: no lane kept after a load, a check, or a thread that loads and exits; one
    build of P-256's tables.
  - **`tests/bote_fs_tools.tcyr`**, 63, from 56: reads never rewound add no chunk, and a batch of fifty holds one buffer.
  - **`tests/bote_streamable.tcyr`**, 61, from 58: a resource and a template registered reach a session's buffer.
- **`fuzz/listen_filter.fcyr`**, new: 20,000 filters from a grammar, each judged against a reference made from the choices
  that drew it, on a dispatcher with every list and one with none, with nothing left on the heap.
- **51 mutants, all killed.**
  - Listen: 28 of 28. Two first survived, and each gave a test: an empty string as `resourceSubscriptions`, and the stream
    flag being off once a message is served.
  - The fuzz harness: 8 of 8. Its first survivor showed the same missing `""` case.
  - The one-look fix: 4 of 4. The validation mutant was first killed by one call in thousands, so its writer now replaces
    the tool with no gap, which kills it about 350 times a run.
  - The first registry, the read buffer's release, the curve check and the warm: 11 of 11.
  - **The first fix for the lanes was wrong twice.** A lane scope around the warm survived its mutant, because sigil's sign
    and verify open their own scopes, so the review was wrong about that half. And a lane scope around the curve check used
    `crypto_lane_enter`, which only the 6.7.18 fold has: `scripts/bundle-min-cyrius.sh` refused 6.7.13, the stated
    minimum. A probe then found the lane was taken by `_ecv_init`, a call the curve check never needed.
- **`scripts/transport-smoke.sh`**: a listen on each of the three transports, keepalives at 200 ms, a client gone in the
  log, `-32601` over stdio, and `BOTE_WORKERS` with leading zeros.
- **The official suite** is where 3.7.1 left it, and has no listen scenario yet; the official client passes 410 of 410.

### Performance

**The request path costs what it did, or less.** Interleaved A/B against 3.7.1, 15 rounds pinned to an idle core.
Minimums, in ns:

| Bench | 3.7.1 | 3.7.2 | Delta |
|---|---|---|---|
| `dispatch_tools_call` | 2,215 | 2,064 | −6.8% |
| `dispatch_tools_call_result` | 1,964 | 1,872 | −4.7% |
| `dispatch_tools_call_scoped` | 5,296 | 5,018 | −5.2% |
| `codec_process_message` | 4,069 | 3,942 | −3.1% |
| `http_era_modern` | 11,295 | 11,003 | −2.6% |
| `codec_process_message_threaded` | 4,358 | 4,232 | −2.9% |

Rows that run none of this patch's code moved too (`jsonx_get_str_flat` −5.4%, `validate_compiled_simple` −4.8%), which
is layout. So against a control that keeps this patch's layout and puts `tools/call` back on four lookups, 9 rounds: one
look saves 88 ns on `dispatch_tools_call` (−4.1%), 109 ns with a CallToolResult (−5.5%) and 258 ns with a scope (−4.9%),
where the rows that do not reach it moved within ±2.5%.

**`listen_filter`**, the mask and the acknowledgement's params for a filter naming every kind, is 3.7 µs.

**A registration under load no longer waits on spinning readers.** 100 registrations beside 16 threads serving
`tools/list`, on two cores: 138 to 161 ms in all on 3.7.1, the slowest 34 to 54 ms; 4 to 7 ms on 3.7.2, the slowest 0.5 to
2 ms. A thread yields only while it waits, so an uncontended lock costs what it did.

## [3.7.1] — 2026-11-02 · Registering while serving

**Everything a request reaches in bote is safe on worker threads.** 3.7.0 made streamable's own state safe between its
workers. This makes the rest safe: bote's own tools, and registering a tool, prompt or resource while a threaded server
serves, which is how a `listChanged` notification comes about. The binary's tool sets are unchanged: what streamable
exposes is an open decision, and nothing technical holds it any more.

**Five defects in 3.7.0 land ahead of it**, from an independent review of that release; the worst hung a threaded server
that had no entropy. See the correction notice in 3.7.0.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

> ⚠ **Corrected at 3.7.2.** Five claims below were not true, found by an independent review of this release. "Everything a
> request reaches in bote is safe on worker threads", registering included, missed three things. A `tools/call` checked a
> tool's scopes under one look at the registry and found its handler under another, so a tool registered beside the call
> ran for a token without its scope. A prompt or resource replaced meanwhile was judged by one registration and served by
> another. And two threads registering a dispatcher's first prompt or resource each made a registry, one lost with what was
> in it. `registry_deprecate` also wrote a registered tool with no lock. "Nothing else in bote holds a lane, so the binary
> cannot run out" was false once an EC key was loaded: the curve check kept the loading thread a lane for life, the main
> thread's at startup, so 64 workers shared 63. "A count past nine digits reads as 999999999, which each one's bound then
> holds" counted leading zeros, so `BOTE_WORKERS=0000000000` started 64 workers, and `BOTE_JWKS_REFRESH` has no bound to
> hold. "P-256's when it takes an EC key" built them for every EC key of every set that changed, 13 ms each. And `fs_read`'s
> buffer, once on the arena, stayed there for the rest of its message, 64 KB a batch element. Each is fixed in 3.7.2.

### Added

- **The registries' lock** (`src/protocol.cyr`, in core): `bote_reg_read` / `bote_reg_read_end` and `bote_reg_write` /
  `bote_reg_write_end`. One word: readers share it, a writer has it alone, and a writer that is waiting holds new readers
  off. It is taken only once a thread exists, so a single-threaded server pays a compare for each. It is not re-entrant.
- `bote_libro_lock` / `bote_libro_unlock` (`src/audit_libro.cyr`): the one lock on libro's chains.

### Changed

- **The tool, prompt and resource registries and the dispatcher's handler maps are under the registries' lock.** Each
  public function takes it for its own body and calls only the `_u` forms inside it (`_registry_get_u`, `_registry_add_u`,
  `_registry_deregister_u`, `_prompt_registry_get_u`, `_resource_registry_get_u`, `_resource_registry_template_for_u`).
  Registration's checks and schema compile run before it (`_registry_prepare`). The list builders hold it across both of
  their passes, the one that sizes the reply and the one that writes it. `dispatcher_register_tool` and
  `dispatcher_deregister_tool` put the tool and its handler in, or take them out, in one write, and publish their event
  after it, with nothing held.
- **`fs_read`'s buffer is on the request arena**, the calling thread's, where it was one buffer for the process.
- **The libro audit sink and the five libro tools take the chain's lock** around all of their work: libro locks nothing.
- **A JwtSecret builds the sigil tables its verifies need when it is made**: SHA-256's in `jwt_secret_new`, P-256's when it
  takes an EC key. An ES256 server pays the 0.4 s that costs at startup rather than on its first token.
- The session store reads the clock before it takes its lock, not under it.
- A count from the environment past nine digits reads as 999999999 (`BOTE_WORKERS`, `BOTE_JWT_CACHE`,
  `BOTE_JWKS_REFRESH`), which each one's bound then holds.
- `_bote_ses_random`, a test's stand-in for the session id's entropy, beside `_bote_jwt_clock`.

### Fixed

- **On worker threads, two `fs_read` calls could read into the same buffer**, and hand one client another's file.
- **The libro audit sink could append to a chain while a libro tool walked it**, from two workers.
- **Registering while a threaded server ran could crash it**, or make a tool that was there all along answer as not found:
  a map that grows rebuilds its table under a reader. Between registering a tool and its handler, a call could also find the
  one without the other.
- **A worker that found no entropy hung the server** (3.7.0). `_gen_session_id` exited 90 with `sys_exit`, which ends the
  calling thread alone, and it did so holding the session store's lock. It exits the process now, with `sys_exit_group`,
  and so does the binary on its way out, where a worker may still be running.
- **Two workers could each fetch the keys** (3.7.0), when one read the clock a second before the other took the ask: it saw a
  clock gone back, which opens the gate. A clock that went back by less than the cooldown no longer does.
- **A fetch forked from one worker while another built one of sigil's tables waited on it** until its deadline (3.7.0): the
  child inherits the build's claim and nobody to finish it. The tables a JWT verify builds lazily, SHA-256's and P-256's,
  which a fetch's TLS handshake also needs, are built before any worker is (above).
- **`BOTE_WORKERS` past nine digits kept 4** (3.7.0) rather than being held to its bound.

### Security

- **More than 64 workers can turn good ES256 tokens away.** A verify holds one of sigil's 64 crypto lanes while it runs, and
  refuses the token when none is free. `BOTE_WORKERS` stops at 64, and nothing else in bote holds a lane, so the binary
  cannot run out; `streamable_config_with_workers` now says so for an embedder, who can configure more.

### Tests

- 4,536 assertions, from 4,499, on x86_64 and under `qemu-aarch64`.
  - **`tests/bote_threads.tcyr`** has 74, from 49:
    - `fs_read` of four files on eight threads, none handed another's.
    - libro appends from three threads beside two walking the chain with its tools, then again with retention rewriting
      the chain.
    - Children forked from six threads at once, each answering its own.
    - Registering while serving: two writers grow every map, one through `dispatcher_register_tool` and one through
      `registry_register` and `dispatcher_handle`. Beside them run three threads dispatching, a thread for each public
      lookup and for each list calling only it, and a tool and its handler checked together under one hold of the lock.
    - Four threads each registering templates, and setting handlers, losing none.
    - No entropy on a worker ending the process with 90, in a child the test forks.
  - **`tests/bote_jwt.tcyr`** has 483, from 471:
    - One ask whichever second a worker read, and a clock gone back past the cooldown still asking.
    - The tables a handle builds when it is made and when it takes an EC key.
- **30 mutants, 27 killed.**
  - **The first run left 12 standing**, and they showed how a race hides from a test:
    - A lookup with an unlocked mutant was sheltered by a locked call beside it on the same path, so every lookup and
      every list got a thread of its own, calling nothing else.
    - A missing lock between writers needs two writers.
    - A tool and its handler must be checked while the name is being added, not once it has been.
  - **Three are equivalent on x86**, where a vec that never frees, and stores seen in order, give an unlocked reader whole,
    stale data: libro's read-only tools beside appends and retention, and the templates list builder. The locks stay, for
    aarch64's weaker ordering, which QEMU on an x86 host cannot show either.
  - **Two of the review's mutants first survived on a test of mine that was wrong:** an EC key's own checks already build the
    flag it read.
- **`scripts/transport-smoke.sh`**: a twelve-digit `BOTE_WORKERS` is held at 64.
- **The official suite** is where 3.7.0 left it, and the official client passes 410 of 410.
- stdio is byte-identical to 3.7.0 over 200,000 messages, and HTTP over 50 of 50.

### Performance

**Unthreaded, the request path costs what it did.** Interleaved A/B against 3.7.0, 15 rounds pinned to an idle core.
Minimums, in ns:

| Bench | 3.7.0 | 3.7.1 | Delta |
|---|---|---|---|
| `dispatch_tools_call` | 2,162 | 2,222 | +2.8% |
| `dispatch_initialize` | 1,752 | 1,734 | −1.0% |
| `codec_process_message` | 4,038 | 4,063 | +0.6% |
| `http_era_modern` | 11,288 | 11,319 | +0.3% |
| `scope_gate_open` | 2,294 | 2,353 | +2.6% |

Rows that run none of this patch's code moved as far (`validate_compiled_simple` +3.3%, `jsonx_get_str_flat` −6.2%),
which is layout. Until a thread exists, each registry function's lock is one compare.

**On a worker the registries' lock costs a message 52 ns.** `codec_process_message_threaded` now starts and joins a thread
first, so it runs as a worker does, with every lock that waits for threads taken; in 3.7.0 it ran without one. It takes
4,364 ns against `codec_process_message_arena`'s 4,048 ns for the same message unthreaded. Against a control with the
registries' lock emptied, its calls kept, 7 rounds: 4,311 ns, so 52 ns (1.2%) of that is this lock's atomics. Most of the
rest is lib/alloc.cyr's lock, which every heap allocation takes once a thread exists, and the 47 ns per-thread lookup.

## [3.7.0] — 2026-11-02 · Streamable on worker threads

**`bote streamable` serves on worker threads**, 4 by default (`BOTE_WORKERS`). Its loop only accepts, and each connection
goes to the next free worker, so a client that is slow to send or to read, or a token that is slow to verify, holds one
worker and not the server. On the smoke test a connection held open half-sent stops every other client of a
`BOTE_WORKERS=0` server, the one loop it was, and none of a threaded one. The first bite of 3.7.x: everything a streamable
request reaches in bote is made safe between workers, the request's own state per thread and the rest guarded.

**Five defects land ahead of it**, from an independent review of 3.6.6. The worst is older: since 3.4.15 the request arena
kept a chunk for every message a little larger than the one before, 105 MB for 300 refused `resources/read` calls, with no
auth. See the correction notices in 3.6.6 and 3.4.15.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

> ⚠ **Corrected at 3.7.1.** Four claims below were not true, found by an independent review of this release. "Everything a
> streamable request reaches in bote is made safe between workers" missed the session store's entropy check: with no
> entropy it exited with `sys_exit`, which ends only the calling thread, so on a worker the server stayed up with the
> store's lock held by a thread that was gone, and the next request waited for good. "Workers that find the keys due
> together make one fetch between them" failed across a second boundary: a worker that read the clock a second before
> another took the ask saw a clock gone back, and asked again. "One past the bound is held there" failed for
> `BOTE_WORKERS` past nine digits, which read as no count and kept 4. "The per-thread lookup costs a message 47 ns" holds
> armed from the main thread; armed from another, each lookup also asks sigil whether the thread has a block, which on
> an x86 kernel before Linux 5.9 is two system calls. And a fetch forked from a worker could still inherit sigil's claim on
> a table another worker was building, which the child then waited on until its deadline. Each is fixed in 3.7.1.

### Added

- **`streamable_config_with_workers(c, n)`** and `streamable_config_workers`: serve on `n` threads, or on the calling thread
  with 0, the default, so an embedder's server is as it was until it asks. StreamableConfig grows to 96 B.
- **`BOTE_WORKERS`** in the binary, for `bote streamable`: 4 unless set, at most `BOTE_WORKERS_MAX` (64), held there with a
  WARN; a value that is not a count keeps 4 with a WARN. Logged at INFO.
- **`src/conn.cyr`**: `bote_conn_serve_pooled(transport, addr, port, fp, ctx, workers)` on sandhi's pool;
  `bote_conn_arm()`, which gives every thread a request state of its own; `bote_conn_serial_sinks(d)`, which makes a
  dispatcher's audit and event sinks take turns, since libro's chain and majra's hub were written for one caller at a time.
- **`src/protocol.cyr`**, in core: `bote_req_state()`, the calling thread's request state (arena, token, its kept copy, a
  fetch's reason); `bote_req_arena_new()`, bote's own request arena, and `bote_req_arena_bytes(a)`.
- `session_store_lock` / `session_store_unlock`, `stream_event_to_wire_a`, `resumption_buffer_events_after_a`.

### Changed

- **A threaded server's shared state is guarded.** The session store takes a mutex in each of its functions (SessionStore
  32 B), which also covers each session's outbound buffer: publishing a notification and draining one hold it, and a drain is
  copied onto the arena under it, so what a GET sends is the buffer at one moment. A session's outbound bundle is installed
  with a compare-and-swap, so two workers asking at once share one. A JwtSecret (136 B) gives each refresh to one worker by
  a compare-and-swap on when it was last asked, so the others go on with the keys held and no verify waits on a fetch; its
  cache is read and written under its mutex once threads run, and each clear moves a generation on, so a token verified with
  keys a refresh has since replaced is not remembered. `bote_fetch_why()` is the calling thread's.
- **A fork from a worker holds the allocators' locks across it** (`_bf_fork`), so the child cannot inherit one another
  worker held: lib/alloc.cyr's and lib/freelist.cyr's, through their own functions, for want of a stdlib hook (filed in
  cyrius's repository as 2026-11-02-bote-fork-needs-the-allocator-locks.md).
- **Two JWK Sets with the same keys are the same set**, in any order.
- **On agnos a token naming a key the set does not hold asks nothing**; the timer still refreshes there.
- **A GET's drain, and a POST carrying the pending events, render them on the request arena.**

### Fixed

- **The request arena grew without bound for messages of rising size** (since 3.4.15). lib/alloc.cyr's growable arena, when
  the chunk after the one in use was too small, linked a new chunk in front of it and kept both, so each message a little
  larger than any before added a chunk for good: 105,138,568 B for 300 refused `resources/read` calls, the URI rising from
  12 KB to 42 KB. bote has its own arena: the first chunk ahead that fits, else one appended at the end, of the chunk size
  doubled until the request fits, so a ramp keeps a chunk for each size its largest message needs. Alone, 200 rounds of a
  rising size kept 24,121,984 B on the stdlib's and keep 262,160 B now; end to end, 150 messages rising to a 92 KB URI keep
  2.9 MB, where the stdlib's kept about 2 MB a message. Filed in cyrius's repository as
  2026-11-02-bote-growable-arena-keeps-a-chunk-per-larger-request.md.
- **A resource registered with no handler was judged by itself, and served by its template**, so the template's scopes did
  not apply to it, on `resources/read`, the HTTP 403 gate and a completion. It is judged by what serves it now.
- **A JWK Set served in another order on each fetch was a change each time**: a heap copy of the keys, kept, and the
  verified-token cache emptied, at up to one fetch a minute that a client could trigger.
- **On agnos a client could start a fetch** by naming an unknown `kid`, and there a fetch runs in this process and keeps what
  sandhi's client allocates.
- **`jwt_secret_from_jwks` kept a handle for each set it refused.** It checks the set first.
- **A GET's drain rendered every pending event on the heap**, each time: twenty GETs of forty events kept 275,200 B. A
  POST carrying them did the same.
- `ssrf_host` and `_ssrf_lower` said they return a fresh heap string; since 3.6.6 it is on the arena, until the next message.

### Security

- **One client no longer holds a streamable server.** A connection sent half a request held the one loop until sandhi's
  60 s budget for it, and a client reading a little at a time held it longer; each now holds a worker, and it takes
  `BOTE_WORKERS` of them to hold the server. A forged token's verify, about 10.5 ms for ES256, holds one worker too.
- **A client could exhaust the server's memory** with refused requests of rising size, needing no credentials, on every
  transport. Fixed above.
- **A template's scopes could be bypassed** through a URI registered as a resource with no handler. Fixed above.

### Tests

- 4,499 assertions, from 4,404, in 20 files, on x86_64 and under `qemu-aarch64`.
  - **`tests/bote_threads.tcyr`**, new, has 49, each driven from several threads at once: each thread's own request state,
    armed from a worker so that the main thread, with no thread-local block, gets one at its first look; 1,600 sessions made
    on four threads, none lost; a session's outbound bundle asked for by eight threads at once; 1,500 notifications published
    while three threads drain and another makes sessions, every drain whole and in order; sinks that take turns; eight
    workers finding the keys due, or meeting an unknown `kid`, making one fetch; sixteen tokens in four cache slots on eight
    threads; a fork while another thread holds lib/alloc.cyr's lock, and lib/freelist.cyr's; the streamable handler on six
    threads at once over socketpairs, 720 sessions and calls; and 0 heap bytes for a GET's drain and a POST's.
  - **`tests/bote.tcyr`** has 969, from 947: the arena, its chunks, alignment, bounds, realloc and order; 200 rounds of a
    rising size keeping one chunk; the review's ramp end to end, started past the largest size the rows before it reached,
    and run twice; and a resource with no handler judged by its template.
  - **`tests/bote_jwt.tcyr`** has 471, from 454: the same keys in another order, a key twice, and the generation: a token
    verified across a change of keys is not remembered, and the next verify is.
  - **`tests/bote_streamable.tcyr`** has 58, from 53: a full store keeps nothing for a refusal, and arming on a main thread
    with no thread-local block. **`tests/bote_fetch.tcyr`** has 62.
- **39 mutants, 39 killed.** The first run left seven standing. Six were test gaps, now covered: arming on a main thread
  with no block, a full store's refusal, publishing and draining unlocked (a vec that never frees hands a racing copy valid
  pointers to stale data, so the test checks order, not only validity), a shrinking realloc, and a ramp that started inside
  sizes the arena had already reached. The seventh was a mutant that left the lock's release in place, which let the child
  go free; written properly, it is killed.
- **`fuzz/req_arena.fcyr`**, new, the twelfth harness: random cycles of allocations, reallocations and resets, every block
  intact at the end of its cycle, aligned, and the heap bounded. lib/alloc.cyr's growable arena fails it on the bound.
- **`scripts/transport-smoke.sh`**: streamable answers beside a held connection, which stops a `BOTE_WORKERS=0` server;
  `BOTE_WORKERS` is held to its bound; and sixteen clients send 12,639 authenticated requests in 3.5 s while the keys are
  fetched from a `jwks_uri` every second, each fetch forked from a worker, none failing.
- **The official suite** is where 3.6.6 left it, and the official client passes 410 of 410 on every transport and token kind.
- stdio is byte-identical to 3.6.6 over 200,000 messages, and HTTP over 50 of 50.

### Performance

**On worker threads**, `bote streamable` with 8 client processes on 8 cores and the server on 8 others, 3 s per row, the
client part of what is measured. Requests a second:

| Load | `BOTE_WORKERS=0` | 4 workers |
|---|---|---|
| `server/discover`, no auth | 12,789 | 22,179 |
| the same with an ES256 token, remembered | 8,824 | 19,066 |
| a forged ES256 token, a full verify each, refused | 96 | 223 |

**Per message the request path costs what it did.** Interleaved A/B against 3.6.6, 15 rounds pinned to an idle core:
most rows are 1 to 3% slower, rows that run none of the changed code among them (`jsonx_get_str_flat` +14.6%,
`jsonx_get_raw_nested` +3.8%, `schema_compile_nested` +2.3%), which is layout: the bench binary grew 12.6 KB. So a
control was built with the same layout, this tree with 3.6.6's request path (the stdlib's arena and its fast path) and no
session lock, and run against it, 11 rounds. Minimums, in ns:

| Bench | control | 3.7.0 | Delta |
|---|---|---|---|
| `dispatch_tools_call` | 2,193 | 2,182 | −0.5% |
| `dispatch_initialize` | 1,776 | 1,778 | +0.1% |
| `codec_process_message` | 4,108 | 4,077 | −0.8% |
| `http_era_legacy` | 2,451 | 2,479 | +1.1% |
| `http_era_modern` | 11,595 | 11,413 | −1.6% |
| `session_validate`, new: under the store's lock | 1,685 | 1,708 | +1.4%, 23 ns |

**The per-thread lookup costs a message 47 ns**, armed: `codec_process_message_arena` 4,044 ns against
`codec_process_message_threaded` 4,091 ns, the same message before and after `bote_conn_arm`. A process that never arms
pays one compare. Most of `session_validate`'s 1.7 µs is the `clock_gettime` system call (3.4.1).

## [3.6.6] — 2026-11-02 · Keys from `jwks_uri`

**The rest of 3.6.x, authorization on HTTP.** Three items, each built and mutation-tested before the next:

- **Keys from the authorization server's `jwks_uri`.** `BOTE_JWKS_URL` fetches its JWK Set at startup, again every
  `BOTE_JWKS_REFRESH` seconds, and as soon as a token names a key the set does not hold, never more often than a 60 s cooldown. While the
  server is down the last good set serves and the asks back off. A set that changes forgets every remembered token, so a key the
  server removes stops passing tokens at the next refresh. Each fetch is made in a child (3.6.5), with a 5 s deadline that covers DNS.
- **Scopes on prompts, resources and resource templates**, enforced on `prompts/get`, `resources/read`, and a `completion/complete` for
  them, with the same 403 and `-31403` as tools.
- **A WebSocket connection's token is judged again**, at most once a second, and one that has expired ends the connection with 1008.

**Two defects in 3.6.5 land ahead of them**, from an independent review of that release: `web_fetch` kept its URL on the heap twice per
call, and on agnos a web call kept its record. See the correction notice in 3.6.5.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

> ⚠ **Corrected at 3.7.0.** Four claims in this entry were not true, found by an independent review of this release.
> `dispatcher_resource_scopes_missing` did not judge a URI "by what would serve it" when a resource was registered there with
> no handler: `resources/read` serves that URI by the template, and the check judged the resource, so a template's scopes
> were bypassed for it. "Each fetch is made in a child" holds on Linux; on agnos it runs in this process, and a client
> could start one by naming an unknown `kid`. "A set that has not changed... keeps nothing" failed for the same keys in
> another order, which read as a change: a heap copy of the keys a fetch, kept, and an emptied cache. And a JWK Set that
> `jwt_secret_from_jwks` refused kept the handle it had made. Each is fixed in 3.7.0.

### Added

- **`src/jwks.cyr`**, in the full bundle, now 37 modules: `bote_jwks_url_check(url, ca)`, `bote_jwks_source_new(url, ca)`, and
  `bote_jwks_fetch`, a key source. The URL is the operator's, so the SSRF guard does not apply; it must be `https://`, against the
  system's roots or `BOTE_JWKS_CA`'s, or `http://` to 127.0.0.0/8 or `localhost`, with no userinfo and no IPv6 literal. No redirect is
  followed, the body is at most `BOTE_JWKS_MAX` (1 MiB), and `BOTE_JWKS_TIMEOUT_MS` (5000) covers all of it. The child resolves the
  name with the stdlib's `net_resolve_ipv4`, which gives up on DNS after two tries of 2 s, through sandhi's resolve hook, unless an
  embedder's hook is installed. A failure says why: no connection, TLS, a timeout, no IPv4 address, the status, a NUL in the body.
- **A key source on a JwtSecret** (`src/jwt.cyr`): `jwt_secret_with_key_source(s, fp, ctx, every)`, `jwt_secret_refresh(s)`,
  `jwt_secret_set_jwks(s, json)`, and accessors for the refresh interval, the last fetch and the failures. A source is
  `fn (ctx, why_out) → set or 0`. The cooldown is `BOTE_JWKS_COOLDOWN` (60 s), or the refresh interval if that is shorter, and after a
  failure the wait doubles, up to the interval. A refresh logs INFO when the set changed, DEBUG when it did not, WARN with the reason
  when it failed; a token that names an unknown key and causes an ask, DEBUG. No key, set or token is logged.
- **`BOTE_JWKS_URL`, `BOTE_JWKS_CA` and `BOTE_JWKS_REFRESH`** in the binary. One of `BOTE_JWT_SECRET`, `BOTE_JWKS_FILE` and
  `BOTE_JWKS_URL`. The first fetch must succeed, or the server does not start: one that started with no keys would refuse every
  token and say less about why. The startup line names the host, the key count and the interval.
- **`prompt_def_with_required_scopes`, `resource_def_with_required_scopes` and `resource_template_with_required_scopes`**, with their
  getters. `dispatcher_prompt_scopes_missing`, `dispatcher_resource_scopes_missing`, which judges a URI by what would serve it, the
  resource at it, else the first template it matches, and `dispatcher_completion_scopes_missing_a`, which judges a completion by the
  prompt its `ref/prompt` names, or for `ref/resource` the template written that way, else the resource. `bote_scope_list_ok`.
  `resource_registry_template_for` and `resource_registry_template_named`.
- **`BOTE_WS_RECHECK_MS`** (1000): how often a WebSocket connection's token is judged again, by the validator that took it.

### Changed

- **`bote_http_scope_gate`** answers `prompts/get`, `resources/read` and `completion/complete` with the 403 too, from the dispatcher's own
  functions, so the gate and the dispatcher cannot judge one request differently.
- Registering a prompt, a resource or a template whose required scope is no scope-token is refused, as a tool's is.
- The WebSocket transport keeps the token a connection presented, in `_ws_token_buf`, whatever claims the validator returned, so the
  recheck judges the token and a handler is still handed the claims. The 64 KB bound applies to it either way.
- **A JWK Set is read on the request arena** (`jwt_jwks_check`, `jwt_secret_from_jwks`), and only the keys it holds are kept. A set
  that holds the same keys in the same order changes nothing, and keeps nothing.
- JwtSecret grows to 120 B, PromptDef to 32 B, ResourceDef to 40 B and ResourceTemplate to 48 B, each by appended fields.
- **The roadmap holds open work only.** The 3.5.x section, finished, is gone: the default revision is an open decision, the
  `-32021` check rides with `InputRequiredResult` in 3.8.x, and the official suite's remaining failures head "Where bote stands".
  Items that were waits on upstream moved to Upstream, and every "roadmap 3.5.x" pointer in the docs and two source comments now
  names 3.7.x, where threaded dispatch is. SECURITY.md supports 3.6.x and 3.5.x.

### Fixed

- **`web_fetch` kept its URL on the heap twice a call, and `web_search` its query once.** The argument was read with `jsonx_get_str`,
  and the SSRF guard lowercased the URL and copied its host there too (`_ssrf_lower`, `ssrf_check`, `ssrf_host`). A 60 KB URL kept
  120 KB a call with no network involved, refused or not. They read onto the request arena now.
- **On agnos a web call kept its record and its text**, 4 KB to 200 KB, because the record and `web_fetch`'s text buffer came from the
  heap; 3.6.4 had the buffer on the arena. Both are on the arena now, which a forked child has its own copy of.
- **A child could outlive a parent that died before it set its death signal**, holding the listening socket. It checks its parent
  after the `prctl` and goes if the parent has.
- **A child that had closed the pipe was still signalled**, which with SIGCHLD ignored could reach a reused pid. A child that ended
  its reply is reaped, not killed.

### Security

- **Keys rotate without a restart, and a removed key stops working.** Before, keys came from a file read at startup, so a key the
  authorization server withdrew kept passing tokens until bote was restarted. With `BOTE_JWKS_URL` it stops at the next refresh,
  remembered tokens included.
- **A client can make bote ask the authorization server for its keys**, by sending a token with a `kid` it does not hold, at most once
  a cooldown: one GET a minute by default. The set is the authorization server's; a client cannot choose where bote fetches it from.
- **A WebSocket connection lasted as long as the connection**, its token judged once at the upgrade. It now ends within a second of the
  token's `exp`, or of its key leaving the set.
- **Scopes reached tools only**, so a prompt or a resource an operator meant to restrict had no way to say so. A completion for a
  restricted prompt or template is refused too: it would offer that prompt's arguments or that template's values.

### Tests

- 4,404 assertions, from 4,014, on x86_64 and under `qemu-aarch64`.
  - **`tests/bote_jwks.tcyr`**, new, has 49: what a `jwks_uri` may be, a source's own copy of its URL, and real fetches from a child to a
    loopback server the test forks. A set, a 404, a NUL, a server that never answers, nobody listening, and a token verified end to end on
    the fetched keys, with 0 heap bytes for ten fetches and ten refreshes.
  - **`tests/bote_jwt.tcyr`** has 454, from 376, on the clock seam against a fake source. A set is taken only when it differs (in its
    kid, its material, its length or its order), and a key that left the set no longer passes a remembered token. The timer, a first
    fetch at 30 s, the cooldown, a `kid` the set does not hold, an HS256 token that asks nothing, and backoff of 60, 120, 240 and 480 s
    and then 600 s, none a second early. A refused set, a clock that went back, an unknown clock, the log lines with neither key nor
    token, one clock read for a remembered token, and 0 heap bytes for a refresh of the same set or a failed one.
  - **`tests/bote_ws.tcyr`** has 94, from 73. A validator that stops taking a token ends the connection with 1008 at the next frame, and
    the presented token is what is judged. Once a second, measured from the last judgement. A real JWT expiring under the clock seam, and
    no heap.
  - **`tests/bote.tcyr`** has 947, from 876: scopes on prompts, resources and templates, each lookup with slot 0 filled, a completion by
    its `ref`, and 0 heap bytes for each refusal with a 60 KB value.
  - **`tests/bote_http_era.tcyr`** has 1,833, from 1,671: the 403 for each on all three transports and both eras, and a batch's `-31403`.
  - **`bote_host`, `bote_web_tools` and `bote_fetch`** add the heap rows for the 3.6.5 fixes: a 60 KB URL keeps nothing of itself.
- **100 mutants, 97 killed.**
  - **The WebSocket recheck:** 12 of 12.
  - **The scopes:** 31 of 31. The first run left four standing, all test gaps, among them a null name reaching a map, which needs slot 0
    filled to crash.
  - **The key source, the fetcher and the 3.6.5 fixes:** 54 of 57. Six more were test gaps, now covered. The other three sat on checks
    that cannot change an outcome, now gone: `alg` beside a length that already decides it, a no-source check every caller has made,
    and an empty host that sandhi's parser refuses first.
  - **Not tested, and why:** the child's parent check and the no-kill on a closed pipe, which only a race or a reused pid shows; the
    resolve hook, which only a DNS lookup that hangs shows; and `web_fetch`'s text buffer on agnos.
- **`scripts/transport-smoke.sh`**:
  - **A `jwks_uri` over loopback http:** keys at startup, a key coming in, one leaving (its remembered token then refused), and the key
    server going down (the last set still serving).
  - **Refusals:** a non-loopback http URL, a server down at startup, and a URL beside a file.
  - **Over https:** keys fetched with `BOTE_JWKS_CA` from a CA the script makes, and refused without it.
- **The official client** runs its RS256 OAuth flows on keys bote fetched from the mock authorization server's `jwks_uri`: 410 of 410.
  The suite is where 3.6.5 left it.
- stdio is byte-identical to 3.6.5 over 200,000 messages, and HTTP over 50 of 50.

### Performance

Interleaved A/B, 15 alternating rounds, pinned to an idle core, 3.6.5 against this tree. Minimums, in ns:

| Bench | 3.6.5 | 3.6.6 | Delta |
|---|---|---|---|
| `jwt_validate_audience`, an HS256 verify | 8,927 | 8,399 | −5.9% |
| `jwt_cached_es256`, a remembered token | 2,599 | 2,657 | +2.2%, ranges overlapping |
| `jwt_cached_es256_sourced`, new: the same, on a handle with a key source | | 2,733 | |
| `jwt_es256` | 10,636,000 | 10,596,000 | −0.4% |
| `http_era_legacy` | 2,510 | 2,497 | −0.5% |
| `http_era_modern` | 11,665 | 11,277 | −3.3% |
| `dispatch_tools_call` | 2,233 | 2,178 | −2.5% |
| `dispatch_tools_call_scoped` | 5,334 | 5,290 | −0.8% |

**A key source costs a remembered token 76 ns**, against the same handle without one: the refresh's check reads the clock the token's
window reads, once, so a hit makes no second `clock_gettime`. A refresh itself is a fork, about 150 µs, and the GET, once every
`BOTE_JWKS_REFRESH` seconds. A set that has not changed is parsed on the arena and compared, and keeps nothing.

The rows that moved most are on code this patch did not touch: `jsonx_get_str_flat` −8.2%, `codec_serialize_response` −7.1% and
`codec_process_message` −6.8% are three of the rows 3.6.5's dead-code control showed move by layout alone, and they have moved back.

## [3.6.5] — 2026-11-02 · Outbound requests in a child

**The web tools make their requests in a child process.** sandhi's HTTP client gives the TLS stack the global heap, not the arena it
was handed, so every `https://` request kept about 175 KB for the life of the process: 179,000 B measured, against 544 B for plain
`http://`. `web_fetch` and `web_search` are tools a client calls, registered on `http`, `bridge`, `unix` and `stdio`, so a client could
grow the server 175 KB at a time. A child takes what it allocated with it when it exits, and the parent kills it at
`BOTE_WEB_TIMEOUT_MS`, which now bounds the DNS lookup sandhi gives no deadline. The parent reads back a record of what the tool needs,
on the request arena, and keeps nothing of the request. Filed against sandhi, in its own repository, as
`docs/development/issues/2026-11-02-bote-tls-connect-keeps-each-connection-on-the-heap.md`.

**Four defects in 3.6.4 land ahead of it**, from an independent review of that release: a NUL in a header got past the WebSocket
upgrade's bound; every WebSocket frame that called a handler copied the token again; a key outside base64 still kept heap; and the
metadata document's 405 and Transfer-Encoding 400 carried no CORS on the bridge. A NUL anywhere in a request head is now refused on every
HTTP-family transport, as RFC 9110 §5.5 allows.

**Fetching `jwks_uri` moves to 3.6.6.** It was this patch's roadmap item, and it needs the same child: a refresh repeats its request for
the life of the process, and an unknown `kid` lets a client ask for one. The child is a large item of its own, and the project's rule is
never to batch two.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

### Added

- **`src/fetch.cyr`**, in the full bundle, now 36 modules. `bote_fetch_in_child(fp, ctx, cap, deadline_ms, cur)` forks; the child runs
  `fp(ctx, record)` and writes the record to a pipe; the parent reads at most `cap` bytes of it onto the request arena, kills a child
  still running at the deadline (0 waits as long as it takes), and reaps every one. `bote_fetch_put_int`, `bote_fetch_put_str` and
  `bote_fetch_put_bytes` write a record; `bote_fetch_int` and `bote_fetch_str` read it back in order, a string in place, and read 0 past its
  end. `bote_fetch_why()` names a failure: `pipe`, `fork`, `poll`, `deadline`, `too large`, `memory` or `short reply`. On agnos the function runs
  in this process. A fork copies only the calling thread, so this is for a single-threaded caller, which every transport is until 3.7.x.
- **`bote_http_head_nul(buf, n)`**, eight bytes at a time, never past the head or `n`.
- `bote_req_note_token_kept(token, kept)` and `bote_req_kept()`: a connection's later messages hand over the copy of the token its first
  one made. `bote_http_refuse_method_hdr(cfd, transport, allow, extra)`, a 405 with extra headers.
- A WARN, `outbound request did not finish`, naming the tool and why.
- Benchmarks `http_head_nul` and `fetch_in_child_empty`.

### Changed

- **`web_fetch` and `web_search` run their request, and what they make of the reply, in a child.** `web_fetch`'s child does the guarded
  GET and the HTML-to-text, `web_search`'s builds the URL, parses SearXNG's JSON and writes the results, and each sends back what the
  parent needs: the guard's refusal, the status, the text. The parent logs and builds the result as before (`_web_fetch_result`,
  `_web_search_result`), so every log line is where it was. `BOTE_WEB_TIMEOUT_MS` bounds the whole request now; it used to start after
  the DNS lookup.
- `_ws_upgrade_ok` checks the key as the handshake does: 22 bytes of base64 and `==` (`_ws_key_ok`).
- `_bridge_with` is gone; its one use is `bote_http_refuse_method_hdr`.

### Fixed

- **A NUL in a header got past the WebSocket upgrade's bound.** `_ws_upgrade_ok` judged each value by `strlen`, which stops at a NUL,
  and `ws_server_handshake` copies the line to its end, so `Connection: Upgrade` and a NUL and 60 KB kept 60,072 B refused and
  61,600 B completed, and the review grew `bote ws` from 5.2 to 15.5 MB with 200 such requests. Every HTTP-family transport now refuses a
  request whose head holds a NUL, with a 400 and a WARN, `nul_in_head`, before it reads a header.
- **Every WebSocket frame that called a handler copied the token.** Each frame began a message, which forgets the copy the last one
  made, so 100 calls of a 20 KB token kept 2,003,200 B; 3.6.3 kept 21,616 B. The first frame to hand a handler the token makes the one
  copy, and every later frame hands over that one.
- **A key outside base64 kept heap.** `_ws_upgrade_ok` checked only its length, and the handshake checks the alphabet after copying all
  four headers: 64 B a request, or 320 B with a 256-byte `Connection`. It keeps nothing now.
- **The metadata document's 405 and Transfer-Encoding 400 carried no CORS on the bridge**, so a page could not read them.
- `_jwt_cache_put` stored whatever `alloc` returned. With nothing to store into, a token is now verified again next time.
- Two comments: a client can aim at a cache slot with tokens the authorization server issued it, which costs another token one verify
  more, and `jwt_secret_with_cache` does not reclaim a table it replaces. `_bote_env_count` sat between `_bote_env_ms` and its comment.

### Security

> ⚠ **Corrected at 3.6.6.** "What a call still keeps is what any handler call keeps, its `args` and its result" was not true. `web_fetch`
> read its URL onto the heap and the SSRF guard lowercased a second copy there, and `web_search` read its query onto the heap, so a 60 KB
> URL kept 120 KB a call beside its `args`, with no network involved. And "on agnos the function runs in this process, as it did before"
> was not as before: the record and `web_fetch`'s text buffer came from the heap, which 3.6.4 had taken from the request arena, so a web
> call there kept 4 KB to 200 KB. Both are fixed in 3.6.6, with two small races in the child, from an independent review of this release.

- **`web_fetch` and `web_search` over `https://` grew the server for good**, 175 KB a call, for any client allowed to call them. In a
  child, the parent keeps none of it: 0 B per request measured, plain and TLS. What a call still keeps is what any handler call keeps,
  its `args` and its result (roadmap 3.8.x).
- **A DNS lookup no longer holds the server past `BOTE_WEB_TIMEOUT_MS`.** The child is killed at the deadline, whatever it is doing.
- **A NUL in a request head is refused** on `http`, `streamable`, `bridge` and `ws` (RFC 9110 §5.5), because bote and the stdlib read a
  value differently past one.
- What is left: about 1.6 KB per completed WebSocket upgrade, all of it `ws_server_handshake`'s (filed at 3.6.4); a fork per outbound
  request; and a forked request is not open to a threaded caller, which 3.7.x has to answer.

### Tests

- 4,014 assertions, from 3,886, on x86_64 and under `qemu-aarch64`. **`tests/bote_fetch.tcyr`**, new, has 59: the record and its bounds,
  a NUL written over memory that was not zero, a child's globals its own, ten children of a megabyte each leaving nothing here, the cap
  exactly at it and one byte over, a 5 s child killed at a 200 ms deadline, a child that exits first, a deadline already past, and a real
  sandhi GET from a child to a loopback server the test forks: ten GETs, 0 heap bytes, every child reaped. `tests/bote_ws.tcyr` has 73,
  from 50: 60 KB behind a NUL in `Connection`, `Upgrade` and the key, each refused with 0 heap bytes, all 255 bytes judged at either end of
  the key, and one copy of a 20 KB token for 1, 10 or 100 handler calls, the 8 B each call's `args` keeps aside.
  `tests/bote_http_era.tcyr` has 1,671, from 1,661: the NUL on `http`, `streamable` and `bridge`, and CORS on the metadata's 405 and 400.
  `tests/bote_auth.tcyr` has 91, from 73: the head check at every offset of a head, aligned and not, through the word step and the bytes
  after the last word. `tests/bote_web_tools.tcyr` has 84, from 66: what each tool makes of every record its child can write.
- **`fuzz/http_head.fcyr`**, the eleventh harness: the head check against a byte loop over 20,000 random heads, with and without an end, a
  NUL or none, cut anywhere and at eight alignments, and a NUL just past `n` never seen.
- 77 mutants, 72 killed. The first round's 66 left four standing, and one test gap among the head check's, the bytes after the last word,
  which now has its test. Of the four, one was code nothing read, a NUL after the record, now gone. One is the loop that resumes a partial
  pipe write, which only a signal causes. One is `web_search`'s deadline: on a server that never answers, sandhi's own deadline inside the
  child ends the request first, and only a DNS lookup that hangs, which a local test cannot make, tells the two apart; the helper's
  deadline is tested on its own. The one-pass head check's 11 left two that change nothing a caller sees: a step of 7 after a clean word,
  and a word read past `n` that can hide nothing inside it.
- `scripts/transport-smoke.sh` sends a NUL in a header to each HTTP-family mode and gets a 400, and calls `web_search` against a loopback
  SearXNG: its results, a 404, nothing written to the stdio wire, and a server that never answers given up at the 500 ms deadline.
- stdio is byte-identical to 3.6.4 over 200,000 messages, and HTTP over 50 of 50. The official suite and client are where 3.6.4 left them:
  128 passed and 45 failed checks on 2026-07-28, 54 and 10 on 2025-11-25, and 410 of 410 client checks.

### Performance

Interleaved A/B, 15 alternating rounds, pinned to one core, 3.6.4 against this tree, load about 3.7. Minimums, in ns:

| Bench | 3.6.4 | 3.6.5 | Delta |
|---|---|---|---|
| `http_era_legacy` | 2,466 | 2,453 | −0.5% |
| `http_era_modern` | 11,151 | 11,392 | +2.2% |
| `dispatch_tools_call` | 2,147 | 2,189 | +2.0% |
| `auth_bearer_check_set` | 766 | 752 | −1.8% |
| `jwt_validate_audience`, an HS256 verify | 8,683 | 8,666 | −0.2% |
| `jwt_cached_es256` | 2,475 | 2,520 | +1.8% |
| `jwt_es256` | 10,421,000 | 10,429,000 | +0.1% |
| `http_head_nul`, new | | 252 | |
| `fetch_in_child_empty`, new | | 145,233 | |

Three rows on code this patch did not touch moved with ranges that do not overlap: `codec_process_message` +5.0%, `validate_compiled_simple`
+5.1%, `jsonx_get_str_flat` +4.4%. **A control says layout:** 3.6.4 with never-called copies of 3.6.5's new functions, in the same places,
moves them +4.4%, +4.6% and +4.9%, and 3.6.5 against that control is within ±1.3% on every row (12 rotating rounds, three binaries).

**The head check every HTTP-family request now pays** is 252 ns for a 140-byte head. The first version found the end of the head with
`sandhi_server_body_offset` and then scanned to it: 844 ns, 794 of them the search. The one pass stops at a NUL or at the first CRLF CRLF,
whichever comes first, and costs 265 ns on the same head, a third.

**A request made in a child** costs about 145 µs more than one made here, for the fork, the pipe and the reap of a child that sends nothing,
and a whole GET to a loopback server made that way took about 0.6 ms, in a probe outside the repository. A TLS GET to a loopback server
with a private CA took 17 ms in the same probe, so for `https://` the fork is under 1%. What it saves is the 179,000 B each such request kept.

## [3.6.4] — 2026-11-02 · Tokens already verified

A token that passed is remembered, so a client that sends it again pays about 2.6 µs instead of the verify: 10.3 ms for ES256,
1.26 ms for RS256, 8.7 µs for HS256. **The cache can only answer what a verify would.** It is keyed by the SHA-256 of the token's whole
text, which is all the signature covered, and holds the token's `exp` and `nbf`, which are judged again on the clock each time, by the
same function a verify uses. Everything else the verify decided depends on that text and the handle's settings, and changing the
audience or the issuers forgets everything. A token that failed is never stored, so a forged one costs what it did, plus its hash.

**Four defects in 3.6.3 land ahead of it**, from an independent review of that release: the bridge's 401 carried no CORS headers, a tool
named twice in `BOTE_TOOL_SCOPES` lost its first scopes, a WebSocket upgrade kept its token on the heap, and a scope set after registration
could end the 403's header. **The review led to a fifth, older one:** cyrius's WebSocket handshake copies the client's headers to the heap
before it judges them, so a refused upgrade with a 60 KB header kept 60 KB. See Security.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

### Added

- **`jwt_secret_with_cache(s, slots)`**, on by default at 1024 tokens: a power of two up to 65,536 (`BOTE_JWT_CACHE_MAX`), or 0 for none.
  `jwt_secret_cache_slots`, `jwt_secret_cache_hits` and `jwt_secret_cache_misses` say what it did, and a token judged from it is a TRACE
  line, `jwt verified from the cache`, with nothing of the token in it. Direct-mapped by the digest, which a client cannot aim without a
  preimage. The table, 48 B a slot, is allocated when the first token is stored and never grows.
- **`BOTE_JWT_CACHE`** in the binary, logged at INFO when it is not the default. A value that is not a count keeps the default, with a WARN.
- `bote_http_refuse_hdr`, `auth_send_challenge_hdr` and `auth_bearer_check_hdr`, the senders with extra response headers, which the
  bridge uses for its CORS. `bote_req_token()`.
- `_bote_jwt_clock`, the clock the time checks read, so a test can put a token on either side of its `exp` and `nbf`. 0 is the wall clock.
- Benchmarks `jwt_cached_hs256`, `jwt_cached_es256`, `jwt_forged_hs256_cache_on` and `jwt_forged_hs256_cache_off`. `jwt_validate_audience`,
  `jwt_rs256` and `jwt_es256` run with the cache off, so they still measure a verify.

### Changed

- **One window check for a verify and for a remembered token**: `_jwt_time_claim` reads `exp` and `nbf`, and `_jwt_window_ok` judges both
  against one clock read, with none for a token that has neither. `_jwt_exp_ok` and `_jwt_nbf_ok` are gone.
- JwtSecret grows to 72 B by appended fields, and `jwt_secret_new` allocates it.
- `bote_scope_token_ok` moves to `src/protocol.cyr`, ahead of everything that uses it, the 403's sender included.
- The WebSocket transport keeps a connection's token in `_ws_token_buf`, which the next connection reuses, and marks it as each frame's
  token, so a handler is handed its own copy as on HTTP. A token of 64 KB or more is refused with a 400, before the upgrade.

### Fixed

> ⚠ **Corrected at 3.6.5.** Four claims in this entry were not true. "A refused upgrade now keeps nothing", here and under Security, failed
> for a header holding a NUL: bote read each value up to it and the handshake copied the line to its end, so 60 KB behind a NUL still kept
> 60,072 B refused and 61,600 B completed, and a key of 24 bytes outside base64 kept 64 B. "A connection keeps no byte of its token" held,
> but every frame that called a handler made a copy of it: 100 calls of a 20 KB token kept 2 MB, where 3.6.3 kept one. "Every refusal past
> the Origin check now carries the bridge's CORS" missed the metadata document's 405 and its Transfer-Encoding 400. And a client can aim
> at a cache slot without a preimage, with tokens the authorization server issued it, at the cost of another token's verify. All are fixed
> in 3.6.5, from an independent review of this release.

- **The bridge's refusals reached no page.** Its 401, with no token or a refused one, and its 404, 405, 413 and four kinds of 400 went out
  without CORS headers, so a browser saw each as a network error: OAuth discovery could not start from the bridge. Every refusal past
  the Origin check now carries the bridge's CORS, `Access-Control-Expose-Headers: WWW-Authenticate` included.
- **A tool named twice in `BOTE_TOOL_SCOPES` kept only its last entry.** `fs_write=files:write admin,fs_write=x` left `fs_write` to a token
  holding `x`, and the review wrote a file that way. A second entry for a tool now stops the server.
- **A WebSocket upgrade kept its token on the heap**, through `auth_bearer_check`, before its version check and handshake ran, and every
  connection kept its copy for good. A refused upgrade now keeps nothing, and a connection keeps no byte of its token.
- **A scope that is no scope-token could end the 403's header.** Registration refuses one, but one set on a tool after registration was
  written as it stood. The 403 now leaves out what is not a scope-token, and `scope` with it when none is.

### Security

- **cyrius's WebSocket handshake kept client headers on the heap.** `ws_server_handshake` copies `Upgrade`, `Connection`,
  `Sec-WebSocket-Version` and `Sec-WebSocket-Key` to the heap before it judges them, so on `bote ws`, which takes no token by default,
  any client could grow the heap by about 60 KB a request: a refused upgrade with a 60 KB `Upgrade` header kept 60,064 B, and a completed
  one with a 60 KB `Connection` header 61,608 B. bote now reads the four on its request arena first, refuses unless the handshake's own
  rules hold, and bounds `Connection` to 256 bytes, so a refused upgrade keeps nothing. A completed one still keeps about 1.6 KB, all of it
  the handshake's, which bote cannot free from its side. Filed against cyrius as
  `docs/development/issues/2026-11-02-bote-ws-server-handshake-keeps-client-headers-on-heap.md`, with an arena variant proposed.
- **The cache revokes nothing, and neither did bote before it.** A token stays good until its `exp`, so keep access tokens short-lived.
- **The cache is single-threaded state**, like the request arena. Threaded dispatch (3.7.x) has to lock it or keep one per worker.
- A forged token still costs a full verify on the single loop: about 10.3 ms for ES256. Put a rate limit in front of a server on the internet.

### Tests

- 3,886 assertions, from 3,755, on x86_64 and under `qemu-aarch64`. `tests/bote_jwt.tcyr` has 376, from 290: the cache against an uncached
  handle at each side of `exp` and `nbf`, a clock set back, an unknown clock, every byte of a remembered token changed, a refusal never
  remembered, tokens for another audience or issuer, changed settings and resizes forgetting, one slot shared, a stored digest corrupted at
  either end, the clock read once, the TRACE line, and 0 heap bytes for a hit and a miss of a 60 KB token. `tests/bote_ws.tcyr` has 50, from
  29: 0 heap bytes for a refused upgrade and for a connection beyond what the handshake keeps, each long handshake header refused, the
  `Connection` bound at 256 and 257, and a handler's kept claims surviving the next connection. `tests/bote_http_era.tcyr` has 1,661, from
  1,646, with CORS on each of the bridge's refusals; `tests/bote_auth.tcyr` 73, from 64.
- 67 mutants, all killed. The first run of the cache's 41 left 9 standing. Four were test gaps, among them an unknown clock with `nbf`, a
  malformed `exp` with no clock, and a resize that kept the table; one was the early return that saves a clock read, now a counted test. The
  other four could not be observed: a slot's in-use flag, the table's zeroing, and two helpers only tests still called. Those are gone.
  The defect fixes' 31 left 2, a long version header and a long key, which only a heap row tells apart.
- `fuzz/jwt.fcyr` checks a cache of 4 slots against none on every byte change of a token, each asked twice, across a clock that crosses
  `exp` and `nbf`, goes back and reads 0, and fails unless the cache was used.
- `scripts/transport-smoke.sh` sends one token three times and finds two TRACE lines and neither the token nor the key, none with
  `BOTE_JWT_CACHE=0`, the size line for 8 and the WARN for `lots`, and a tool named twice stopping the server.

### Performance

Interleaved A/B, 15 alternating rounds, pinned to one core, 3.6.3 against this tree, load about 2.5. Minimums, in ns:

| Bench | 3.6.3 | 3.6.4 | Delta |
|---|---|---|---|
| `codec_process_message` | 3,970 | 3,990 | +0.5% |
| `dispatch_tools_call` | 2,111 | 2,111 | 0.0% |
| `http_era_legacy` | 2,411 | 2,433 | +0.9% |
| `http_era_modern` | 11,106 | 11,001 | −0.9% |
| `auth_bearer_check_set` | 748 | 755 | +0.9% |
| `jwt_validate_audience`, an HS256 verify | 8,622 | 8,615 | −0.1% |
| `jwt_rs256` | 1,260,000 | 1,259,000 | −0.1% |
| `jwt_es256` | 10,265,000 | 10,255,000 | −0.1% |

The short benches moved by layout as 3.6.3 showed they do, −8.8% to +3.3% on code this patch did not touch.

**What a token costs now**, from the same run:

| Token | Verified | Remembered | |
|---|---|---|---|
| ES256 | 10.26 ms | 2.57 µs | 4,000 times less |
| RS256 | 1.26 ms | the same lookup, not timed on its own; its token is longer, so a little more to hash | |
| HS256 | 8.6 µs | 2.56 µs | 3.4 times less |
| HS256, forged | 3.26 µs | 4.40 µs | +1.13 µs: the hash and the lookup |

A hit is the token's SHA-256 and one clock read, the second a `clock_gettime` syscall of about 1.35 µs on this host. A server taking
ES256 tokens went from about 97 requests a second on one thread to as many as the rest of a request allows, for clients it has seen
before. The A/B caught one regression on the way: the refactor read the clock for `nbf` even when a token had none, which cost every
verify 1.4 µs (8.7 → 10.1 µs) until a verify read the clock once.

## [3.6.3] — 2026-11-01 · `403` with `insufficient_scope`

A tool can now require scopes, and a token that lacks one gets the `403` MCP describes, naming every scope the tool needs and
where to read the metadata. That is what lets a client ask for more: **the official TypeScript client steps up on it with no help**,
requesting the union of its scopes and the server's and retrying, on HS256, RS256 and ES256 in both eras. This is the last
item of 3.6.x's first list.

**The check is the dispatcher's, and HTTP's 403 runs ahead of it.** A check in the transport alone would be walked around by a
JSON-RPC batch, which has no status of its own, and would miss WebSocket. So a batch element and a frame are refused by the same
code with `-31403`, and fuzzing asserts that no substitution of a request reaches a scoped tool without its scope.

**A defect fixed ahead of it:** every request a validator accepted kept its token on the heap. See Fixed.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

### Added

- **`tool_def_with_required_scopes(def, scopes)`**, a vec of scope names, and `tool_def_required_scopes`. A token must hold all of them.
  Registration refuses one that is not an RFC 6749 scope-token: `bote_scope_token_ok`, which the metadata check now shares.
- **`dispatcher_set_scope_check(d, fp, ctx)`**, with `fp(claims, scope, ctx)` returning 1 for a scope held, and
  **`dispatcher_scopes_missing(d, name, claims)`**. Claims are opaque to bote, so what reads them is supplied. A refused call is
  answered ahead of its version and arguments, so it learns nothing about them, and is audited and published as a failed call.
- **`bote_http_scope_gate`** on `http`, `streamable` and `bridge`, both eras: a lone `tools/call` the token may not make gets a `403`
  with `WWW-Authenticate: Bearer error="insufficient_scope", scope="…", resource_metadata="…"` (`auth_send_insufficient_scope`), logged
  at WARN with `reason=insufficient_scope`. `resource_metadata` is left out when none is configured, and `error_description`, which is
  optional, is not sent. A notification gets no reply, so it gets no 403 either.
- **`jwt_scope_check(claims, scope, rules)`** reads a verified JWT's scopes: `scope`, space-separated (RFC 9068 §2.2.3), or `scp` as a
  string or an array when `scope` is absent. Matching is exact and case-sensitive, so `files:read` is not `files:read-all`. What is not
  a JWT grants nothing. **`jwt_scope_rules_new()`** and **`jwt_scope_rule(rules, broad, narrow)`** say what a broader scope implies,
  which MCP says a server must account for. Rules chain, and each is followed once, so a cycle ends and a dense set stays quadratic.
- **`BOTE_TOOL_SCOPES`** (`tool=scope scope,tool=scope`) and **`BOTE_SCOPE_IMPLIES`** (`broad=narrow narrow,…`) in the binary. A name
  no tool has stops the server, because a typo must not leave a tool open, and so do scopes with no JWT verifier, a malformed entry
  and a scope that is no token. stdio and the Unix socket take no token, so there they change nothing, and bote logs a WARN saying so.
- `bote_req_note_token` and `bote_req_claims_kept`, which hold a token on the request arena until a handler is handed it. `bote_prm_metadata`.
- Benchmarks `scope_gate_open`, `scope_check_jwt` and `dispatch_tools_call_scoped`.

### Changed

- **`auth_bearer_check_c` leaves a token that is the claims on the request arena**, for the message. The dispatcher copies it when
  a handler is about to be handed it, once a message. `auth_bearer_check` still returns claims that outlive the message, which the
  WebSocket transport relies on. An embedder that keeps claims from `auth_bearer_check_c` past the message calls `bote_req_claims_kept`.
- **`-31403` (`BOTE_RPC_INSUFFICIENT_SCOPE`)** is where HTTP's 403 cannot reach. MCP reserves `-32020` to `-32099` for codes it defines
  and asks that other new codes sit outside JSON-RPC's reserved `-32768` to `-32000`, so it is outside both.
- ToolDef grows to 80 B, the Dispatcher to 104 B and ProtectedResource to 40 B, each by appended fields. Their constructors allocate them.
- Scopes gate `tools/call` only. `prompts/get` and `resources/read` are on the roadmap.

### Fixed

> ⚠ **Corrected at 3.6.4.** Two lines below were not true. The bridge's 401 carried no CORS headers at all, so exposing
> `WWW-Authenticate` reached the 403 alone, and a page still could not start OAuth from a 401. And "a token now reaches the heap only
> when a handler is handed it" held for HTTP, not WebSocket: an upgrade copied its token before its own checks, and every connection
> kept its copy. Both are fixed in 3.6.4, with two more an independent review found.

- **Every request a validator accepted kept its token on the heap**, since the request arena arrived at 3.4.15. A token was copied for
  a handler before anything knew one would run, so a refused call, a `tools/list` and a 403 each kept a kilobyte or so for a JWT, and a
  60 KB token kept 60 KB a request. It took a valid token to reach. A token now reaches the heap only when a handler is handed it, and
  `tests/bote_http_era.tcyr` asserts 0 B per message for those paths on all three transports, with a 60 KB token.
- **A page on another origin could not read `WWW-Authenticate` from the bridge.** CORS hides a response header unless it is exposed, so a
  browser client could see neither the 401's challenge, since 3.6.0, nor the 403's. The bridge now sends `Access-Control-Expose-Headers: WWW-Authenticate`.

### Security

- **A tool that requires a scope with nothing configured to read one is refused, not waved through.** With no claims, as on stdio,
  there is nothing to compare and the tool runs.
- **The gate and the dispatcher read a message one way**, through the one key scanner: `fuzz/oauth_metadata.fcyr` substitutes every byte
  of single, repeated-key, escaped, modern and batch calls, runs the gate and then the codec as a transport does, and fails if the
  scoped tool ever runs. A repeated `name` or `method` is tested on all three transports.
- **A WebSocket checks its token once, at the upgrade**, so a token that expires keeps its connection, and its scopes with it. On the roadmap.

### Tests

- 3,755 assertions, from 3,345, on x86_64 and under `qemu-aarch64`. `tests/bote_http_era.tcyr` has 1,646, from 1,372: every status on
  all three transports, a batch, a notification, a repeated key, an escaped name and method, the audit record, fail-closed with no
  checker, CORS on the bridge's 403, and 0 heap bytes per message with a 60 KB token, 60 KB of arguments or caller id, and a tool
  naming 64 scopes of 200 bytes. `tests/bote_jwt.tcyr` has 290, from 228; `tests/bote.tcyr` 876, from 819; `tests/bote_auth.tcyr` 64,
  from 55; `tests/bote_ws.tcyr` 29, from 24, with frames on the real handler; `tests/conformance.tcyr` 218, from 215.
- 86 mutants: 84 killed and 2 left out as equivalent: the gate's early return with no token, which only saves time, and a guard on a byte
  before a payload, which no test can place. Six survived the first run. Two were test gaps, a second token in one message and a body
  with no `method`, which now crashed the mutant, and two were checks a later one already made, which are gone.
- `fuzz/jwt.fcyr` checks scope matching against a plain split, and rules against a transitive closure on every directed graph of four scopes.
- The official client: 410 checks, from 266. The mock authorization server issues the scope the client asks for, a token is refused
  for no scope, another scope, a longer one and another case, and accepted for the scope, a broader one and `scp` either way.
- `scripts/transport-smoke.sh` runs the binary with `BOTE_TOOL_SCOPES`: the 403 and its challenge, a held and an implied scope, a batch
  element, the log line with no token in it, seven ways of refusing to start, and stdio serving with its WARN.
- stdio is byte-identical to 3.6.2 over 200,000 differential-fuzz messages, and HTTP on 50 of 50.

### Performance

Interleaved A/B, 15 alternating rounds, pinned to one core, 3.6.2 against this tree, on a quiet machine (load about 2). Minimums, in
ns:

| Bench | 3.6.2 | 3.6.3 | Delta |
|---|---|---|---|
| `codec_process_message` | 3,899 | 3,973 | +1.9% |
| `dispatch_tools_call` | 2,113 | 2,114 | 0.0% |
| `http_era_legacy` | 2,421 | 2,407 | −0.6% |
| `http_era_modern` | 11,173 | 11,065 | −1.0% |
| `auth_bearer_check_set` | 804 | 748 | −7.0% |
| `jwt_validate_audience`, HS256 | 8,726 | 8,627 | −1.1% |
| `dispatch_tools_list` | 1,625 | 1,826 | +12.4% |
| `codec_serialize_response` | 174 | 188 | +8.0% |
| `dispatch_tools_call_result` | 1,860 | 1,965 | +5.6% |
| `dispatch_initialize` | 1,674 | 1,754 | +4.8% |

**The last four are where code sits in the binary, not what it does.** None runs a line this patch added, and their ranges did not
overlap, so it was worth finding out. Bisected by file, 3.6.3's `registry.cyr` alone moved them. 3.6.2 with **never-called copies**
of the new registry functions, the same bytes in the same place and no other change, reproduced it: `dispatch_tools_list` +8.9%,
`codec_serialize_response` +6.8%, `dispatch_tools_call_result` +6.5%, `dispatch_initialize` +4.0%. Larger structs alone, the new bench
setup alone and moving the `protocol.cyr` additions each changed nothing, and dead functions padded into `protocol.cyr` swing
`jsonx_get_str_flat` and `codec_parse_request` by about 7.6%. These sub-2 µs benches carry a few percent of layout either way, and a delta
under 10% on one of them is no finding without a control like this. `auth_bearer_check_set` is the one real change: it no longer copies the token.

**What scopes cost**, from the same run:

| Path | Per request |
|---|---|
| `scope_gate_open`: the gate on a token's `tools/call` of a tool that asks for none | 2.2 µs |
| `scope_check_jwt`: one scope read from a JWT, reached through a rule | 1.5 µs |
| `dispatch_tools_call_scoped`: a call holding both of a tool's two scopes | 5.4 µs, against 2.1 µs with none |

A request with no token pays one compare. Against a real HTTP request of about 230 µs, the gate is 1%. An allowed call to a tool that
names two scopes reads the token four times on HTTP, twice in the gate and twice in the dispatcher, about 8 µs together.

## [3.6.2] — 2026-11-01 · RS256 and ES256, from a JWK Set

Most authorization servers sign with a private key, so 3.6.1's audience check reached few of them. bote now verifies RS256 and
ES256 against public keys the operator supplies, as a JWK Set, which is the document an authorization server publishes.

**The official client ran its OAuth flow on each algorithm.** The mock authorization server signs with real keys through Node's
crypto, and bote verifies them, in both eras: 266 checks, from 182. The verifiers are also checked against RFC 7515's published
A.2 and A.3 examples, and against tokens from a signer bote shares nothing with.

**A signature check is now the slowest thing a request does, and it is the thing to plan around.** See Performance. It is reached
with no credentials, and a forged token costs what a good one does.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

### Added

- **`jwt_secret_from_jwks(json)`** loads a JWK Set, and **`BOTE_JWKS_FILE`** points the binary at one. `jwt_jwks_check` says why a set
  cannot load. RS256 keys of 2048 to 4096 bits with e = 65537, and ES256 keys on P-256. A set holding anything else is not an
  error, because real ones hold encryption keys and other algorithms: those are left out with a WARN. A set with nothing usable,
  two keys with one `kid`, or a private key stops the server.
- **`auth_validator_jwt`**, for every algorithm. `auth_validator_jwt_hs256` is the same function under its old name.
- `_jx_array_next`, the one step along a JSON array, now that three places needed it.
- `tests/fixtures/jwt_vectors.mjs` writes `jwt_vectors.cyr`: public keys and tokens signed by Node. No private key is kept.
- Benchmarks `jwt_rs256` and `jwt_es256`.

### Security

- **Algorithm confusion cannot happen here by construction.** A key has one algorithm from the moment it loads (RFC 8725 §3.1). A
  token's `alg` only selects among keys of that algorithm, and a handle holds a shared secret or public keys and never both, so a public
  key never reaches the HMAC. An HS256 token keyed with the RSA modulus or with its PEM is refused by test, as is `alg: none`.
- **Nothing in a token can supply a key.** `kid` picks among the operator's keys, and `jwk`, `jku`, `x5u` and `x5c` are not read. With no
  `kid` there must be exactly one key of that `alg`, because trying each would hand a forged token one expensive verify per key.
- **`crit` is refused**, for HS256 as well, as RFC 7515 §4.1.11 asks of a verifier that does not understand it. It was ignored.
- **CVE-2022-21449's all-zero ECDSA signature is refused by test**, along with `r` or `s` equal to the curve order.
- **Keys are checked at load:** a modulus under 2048 bits, even, or not minimal, a small `e`, a point off the curve, a coordinate of the
  wrong length.
- **ES256 has two spellings of every signature.** `(r, n - s)` is valid, and JWS has no rule against it, so a token's text cannot key a
  revocation list under ES256. A test pins this. HS256 and RS256 keep one spelling. Key on `jti` instead.

### Tests

- 3,345 assertions, from 3,243, on x86_64 and under `qemu-aarch64`. `tests/bote_jwt.tcyr` has 228, from 126.
- 46 mutants, all killed, and one left out as equivalent. Ten survived the first run, nearly all because a second check caught the same
  input, and two of those would have been forgeries with the check gone: a correct RS256 signature with a byte after it, and an
  HS256 token signed with an empty key against a handle with no key. Each has a test of its own now.
- `fuzz/jwt.fcyr` flips every byte of an RS256 and an ES256 token, and sweeps JWK Sets through the loader.
- `scripts/transport-smoke.sh` runs Node-signed tokens through the binary, and proves each bad set stops it.

### Performance

No regression on a path that existed. Interleaved A/B, 15 alternating rounds, pinned to one core, 3.6.1 against this tree. The machine
was busy with other work, so this compares minimums, in ns, which load cannot lower:

| Bench | 3.6.1 | 3.6.2 | Delta |
|---|---|---|---|
| `codec_process_message` | 4,004 | 3,994 | −0.2% |
| `dispatch_tools_call` | 2,112 | 2,212 | +4.7% |
| `http_era_legacy` | 2,470 | 2,483 | +0.5% |
| `http_era_modern` | 11,416 | 11,504 | +0.8% |
| `jwt_validate_audience`, HS256 | 8,842 | 8,892 | +0.6% |

`dispatch_tools_call` is unchanged code, so its +4.7% is this binary's layout.

**What a token costs**, from the same run, a good token or a forged one alike:

| Algorithm | Per token | Against a request of about 230 µs |
|---|---|---|
| HS256 | 8.9 µs | 4% |
| RS256 | 1.3 ms | 6 times the request |
| ES256 | 10.5 ms | 46 times the request |

Most of that is sigil's arithmetic, and none of it is on the heap: a verify keeps 0 B. On one thread it caps a server at about 95
ES256 or 770 RS256 requests a second, and a forged token needs no credentials to spend that time, so about 95 forged tokens a
second occupy a server that takes ES256. `kid` and the signature's length are checked first, so a token that names no key costs
nothing, but one that names a real `kid` does. Put a rate limit in front of a server that takes tokens from the internet. The
roadmap has a cache of verified tokens for the legitimate half of this, and threaded dispatch for the rest.

Stdio is byte-identical to 3.6.1 over 200,000 messages, and HTTP with no auth configured on 50 of 50 cases, with only the version
string aligned.

## [3.6.1] — 2026-11-01 · A token must have been issued for this server

MCP says a server MUST validate that a token was issued for it, and until now bote checked a signature and an expiry. A good
token meant for another server got in, which is the confused deputy that rule exists to stop. `auth_validator_jwt_hs256` now
takes an audience, and the binary can use it at last: through 3.6.0 nothing called this verifier.

**The official client ran its whole OAuth flow on real tokens.** The mock authorization server signs HS256 JWTs with `aud` set
from the `resource` the client asked for, and bote verified them, in both eras. Ten more tokens went in by hand, each of which a
signature check alone would pass or that must fail for its own reason: another audience, a prefix of this one, no audience,
another issuer, no issuer, no `exp`, expired, not yet valid, the wrong key, and `alg: none`. All ten got a 401 with
`error="invalid_token"`, and an array of audiences and an escaped audience were let in.

Only HS256 is verified, so this helps where an authorization server shares a key. Most sign asymmetrically, which is next.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

### Security

- **A token for another server is refused.** `jwt_secret_with_audience` sets this server's identity. `aud` must name it, as a
  string or in an array, compared as what the JSON decodes to, so `https:\/\/` matches and a `\u0000` cannot end the comparison.
  With an audience set, `exp` is required, since a token with none lives forever, and `jwt_secret_with_issuers` limits `iss`.
- **A token that names an audience is refused by a verifier that has none**, as RFC 7519 §4.1.3 says. This reaches a caller of
  `auth_validator_jwt_hs256` with no audience set: a token with no `aud` still passes, as it did.
- **A forged token kept 45,024 B on the heap per request**, with a 60 KB header and no credentials, because the verifier decoded
  a client's bytes with bayan's heap-only decoder ahead of the signature check. Nothing called it, so this was dormant, and wiring
  it in would have made it remote. `_jwt_b64u_a` decodes on the request arena, the HMAC goes on the stack, and refusing any
  token keeps 0 B.
- **`nbf` is enforced**, by the same rules as `exp`. It was read by nothing.

### Added

- **`BOTE_JWT_SECRET`**: an HS256 key, checked against `BOTE_RESOURCE_URL` and `BOTE_AUTH_SERVERS`. A key under 32 bytes, which RFC 7518
  §3.2 does not allow, a key with no resource, and a key beside `BOTE_BEARER_TOKENS` each stop the server with exit 2 and the
  reason. The key is never logged.
- `jwt_secret_with_audience`, `jwt_secret_with_issuers`, `BOTE_JWT_MIN_KEY`. `JwtSecret` grows from 16 to 32 B.
- `fuzz/jwt.fcyr`: any byte of a good token, changed to anything, gets it refused. The decoder agrees with bayan's on what it takes
  and re-encodes to the same text.
- Benchmark `jwt_validate_audience`.

### Changed

- **`_jwt_b64u_a` takes one spelling** of a segment: no padding, no spare bits. That makes the re-encode-and-compare step 3.4.14 added
  after the HMAC unnecessary on every toolchain, so it is gone, along with the 48 B it kept for each valid token.
  `jwt_b64u_decode` is the same decoder, and refuses the padding it used to take.
- `src/jwt.cyr` no longer includes `lib/bayan.cyr`, and `dist/bote.deps` lists the same leaves in another order.
- `tests/bote_jwt.tcyr` needs `src/protocol.cyr` ahead of `src/jwt.cyr`, for the request arena, as a consumer of the module does.

### Tests

- 3,243 assertions, from 3,176, on x86_64 and under `qemu-aarch64`. `tests/bote_jwt.tcyr` has 126, from 59.
- The official client passes 182 checks, from 158.
- 45 mutants, all killed. One showed that nothing proved a signature with bytes after a correct MAC is refused: remove the length
  check and that token is accepted. It is refused, and a test now says so.
- `scripts/transport-smoke.sh` runs a real token, one for another server, and one from another issuer through the binary, and
  proves each bad configuration stops it, with the key in no log.

### Performance

No regression. No path that existed changed. Interleaved A/B, 15 alternating rounds, pinned to one core, 3.6.0 against this tree. The
machine was busy with other work, so this compares minimums, in ns, which load cannot lower:

| Bench | 3.6.0 | 3.6.1 | Delta |
|---|---|---|---|
| `codec_process_message` | 4,828 | 4,775 | −1.1% |
| `dispatch_tools_call` | 2,620 | 2,556 | −2.4% |
| `http_era_legacy` | 2,907 | 2,919 | +0.4% |
| `http_era_modern` | 13,393 | 13,473 | +0.6% |
| `auth_bearer_check_set` | 978 | 969 | −0.9% |
| `jwt_validate_audience`, new | | 10,317 | |

Checking a token costs about 10 µs, nearly all of it the HMAC, and only on a server that asks for one.

Stdio is byte-identical to 3.6.0 over 200,000 messages, and HTTP with no auth configured on 50 of 50 cases, with only the version
string aligned.

## [3.6.0] — 2026-11-01 · A client can find out where to get a token

MCP requires a protected server to say which authorization servers issue its tokens, and until now bote could not. A client
that hit a 401 had nowhere to go. With OAuth 2.0 Protected Resource Metadata (RFC 9728) configured, a 401 names a document, the
document names the authorization server, and the client gets a token and comes back.

**The official client has done exactly that.** `@modelcontextprotocol/client` 2.4.0's `ClientCredentialsProvider` was pointed at a
protected bote and a small mock authorization server: it took the 401, followed `resource_metadata`, read the document, found
the server, asked for a token bound to this resource with `resource=`, and retried, in both eras, with nothing else to go on.
The conformance suite has no resource-server scenario, so that is the third-party evidence here. It ran against an allowlist
validator: a real JWT has not been through it.

The handler ABI and the stated minimum cyrius (6.7.13) are unchanged, and with nothing configured every byte on the wire is
what it was, apart from the fix below.

### Added

- **`src/oauth_metadata.cyr`**, on `bote http`, `bote streamable` and `bote bridge`. `bote_prm_new(resource, servers, scopes, name)`
  builds the document and the 401's challenge once, at startup, and `http_config_with_resource_metadata`, and its two siblings,
  turn it on. The document is at `/.well-known/oauth-protected-resource` plus the resource's path, as RFC 9728 §3.1 has it:

  ```json
  {"resource":"https://mcp.example.com/mcp","authorization_servers":["https://auth.example.com"],
   "scopes_supported":["files:read"],"bearer_methods_supported":["header"],"resource_name":"bote"}
  ```

  It is served ahead of the bearer check, because a client reads it before it has a token, and after the Origin check, as
  everything is. `GET` alone, with `405` and `Allow` for the rest.
- **`BOTE_RESOURCE_URL`, `BOTE_AUTH_SERVERS`, `BOTE_AUTH_SCOPES`** set it up in the binary. What cannot be published stops the
  server with exit 2 and the reason, as `BOTE_BEARER_TOKENS` with no token does, and so does either of the first two without the
  other. Metadata with no bearer validator logs a WARN: it says where to get a token and requires none.
- **`bote_prm_check`** says why something cannot be published. A resource or an authorization server must be https, or http on
  loopback, with no query, fragment or userinfo. There must be at least one authorization server. A scope must be an RFC 6749
  token. `offline_access` is refused, because MCP says a resource does not require it.
- **`resource_metadata` and `scope` on a 401**, through `auth_bearer_check_c(..., challenge)` and `bote_prm_challenge`.
  `auth_send_challenge` builds the header. `auth_bearer_check` keeps its shape.
- **`conformance-server auth`** and the OAuth flow in `tests/fixtures/conformance_client.mjs`, 18 checks across both eras.
- `fuzz/oauth_metadata.fcyr`: whatever `bote_prm_check` accepts stays inside its header parameter, on one line, and the document
  stays valid JSON that reads back as what was configured.

### Changed

- **A 401 for a refused token carries `error="invalid_token"`**, as RFC 6750 §3.1 asks, with metadata or without. A request with
  no token gets no error code. This is the one change to the wire with nothing configured.
- `http_config_new`, `streamable_config_new` and `bridge_config_new` each gain a tail field, `resource_metadata`.
- `dist/bote.cyr` has 35 modules.

### Security

- **`resource` comes from the operator and never from a request.** A client compares it with the URL it used, which is its
  defence against impersonation (RFC 9728 §7.3), so bote does not read `Host` to build it, and a test sends a hostile `Host` and
  `X-Forwarded-Host` and gets the same bytes back.
- Every configured value is checked at startup, so none can end a quoted header parameter or split a line.

### Tests

- 3,176 assertions, from 3,016, on x86_64 and under `qemu-aarch64`. `tests/bote_http_era.tcyr` has 1,372, from 1,212: every rule on
  all three transports, the spec's own valid and invalid URIs, and 0 heap bytes for the document and for a 401, with 60 KB in the
  path, the Origin and the token.
- `scripts/transport-smoke.sh` runs it on the real binary, and proves a bad configuration stops the server.
- 67 mutants, all killed. One survivor was a test that could not fail: the CRLF injection case held a space, so the space rule
  refused it and the control-byte rule was never reached. Another showed nothing proved that an empty scope list is left out of
  the document, which RFC 9728 §3.2 requires, on the path the binary takes.

### Performance

No regression. Interleaved A/B, 15 alternating rounds, pinned to one core, 3.5.3 against this tree. The machine was busy with
other work, so this compares minimums, in ns, which load cannot lower:

| Bench | 3.5.3 | 3.6.0 | Delta |
|---|---|---|---|
| `codec_process_message` | 4,116 | 4,163 | +1.1% |
| `dispatch_tools_call` | 2,191 | 2,223 | +1.5% |
| `http_era_legacy` | 2,538 | 2,526 | −0.5% |
| `http_era_modern` | 11,729 | 11,701 | −0.2% |
| `auth_bearer_check_unset` | 8 | 8 | 0.0% |
| `auth_bearer_check_set` | 845 | 841 | −0.5% |

A first build took `auth_bearer_check_unset` from 8 to 16 ns, a wrapper's extra call before it could return. It returns first now.

Stdio is byte-identical to 3.5.3 over 200,000 messages, and HTTP with no auth configured on 50 of 50 cases, with only the version
string aligned.

## [3.5.3] — 2026-11-01 · The official MCP client and conformance suite have run against bote

Until this release, every claim about MCP conformance here was bote's own reading of the spec. Now the official
`@modelcontextprotocol/client` 2.4.0 has spoken to bote, and `@modelcontextprotocol/conformance` 0.2.0-alpha.12 has measured
it. The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

**The official client works.** It connects, negotiates and uses every capability bote has, in 140 checks, on `bote http`,
`bote streamable` and stdio in both eras, and on `bote bridge` in the modern one. In auto mode it settles on `2026-07-28` by
itself. It mirrors `Mcp-Param` headers for each encoding the spec has, Base64 included, and bote accepts them all.

**The suite, by its frozen requirement lists:**

| Revision | Required scenarios passing | What fails |
|---|---|---|
| 2026-07-28 | 25 of 37 | `InputRequiredResult` (10), progress (1), `server-stateless` (1: three of its 30 checks need tools bote cannot back) |
| 2025-11-25 | 21 of 30 | Sampling and elicitation (4), logging (2), resource subscriptions (2), progress (1) |

Passing means no failed check. Two of the 25 carry a warning, both for `InputRequiredResult`, and the baseline lists them. The
results match on `http` and `streamable`. Nothing that fails is a defect in what bote does: each is a feature bote lacks,
and `tests/fixtures/conformance-expected-<revision>.yaml` says which. Two scenarios the list leaves unscored also pass in
full: `http-header-validation`, 14 of 14, and `http-custom-header-server-validation`, 10 of 10, which is the suite's own proof of
3.5.1's headers and 3.5.2's `Mcp-Param` work. The plain binary passes 105 checks before any fixture, and its other failures
were tools the suite calls by name.

### Security

- **`resources/list` and `prompts/list` grew the heap on every request**, once a list was past 4 KB. Each built its reply with a
  fixed 4 KB of arena room, and a builder past its room grows on the heap, which never frees: with 200 entries, 128,792 B per
  `resources/list` and 62,232 B per `prompts/list`, for any client, so calling either in a loop runs the server out of memory.
  Both also built a vector of the entries on the heap, per request, at any length. They now size the reply first and read the
  registry directly: 0 B. 3.4.15's rows missed it because their lists were short; they now hold 200 long entries. `_dsp_room`
  goes from `2 × strlen` to `6 × strlen`, the most escaping can reach, and a list that escapes to its full six times has a row.
  Found by reading the code while adding the template list, not by the suite.

### Added

- **Resource templates**, the one gap on both required lists that was a small feature and not a large one.
  `dispatcher_register_resource_template(d, resource_template_new(uri_template, name, description, mime_type), &handler)`, with
  the resource handler ABI. `resources/templates/list` lists them, and `resources/read` tries an exact resource first and then
  each template in order. The handler reads a value with `resource_template_var_a(a, template, uri, name)`, as sent, and owns its
  percent-decoding. A template is RFC 6570 level 1: a `{name}` is one non-empty value with no `/`, `?` or `#`.
  `resource_template_check` refuses anything else rather than reading an operator as text, and refuses two expressions in a
  segment, which would make a match quadratic in a URI a client writes, and a name used twice, where a match would have to
  compare the values. A template with an unknown value falls through to the usual not-found error.
- **`scripts/conformance.sh`** runs the pinned suite and client against `tests/fixtures/conformance_server.cyr`, a dispatcher that
  adds the names the suite calls and nothing else. It is not a CI step, because it installs third-party code from npm, and
  CI now builds the fixture so that it cannot rot. It fails on a failure the baseline does not list and on a listed one that
  now passes.
- `fuzz/resource_template.fcyr`: what a template matches re-expands to the same URI, byte for byte. It found the repeated name.
- Benchmark `resource_template_match`.

### Changed

- The default stays 2025-11-25. bote passes 25 of the 37 scenarios 2026-07-28 requires, and the rest are real features. A client
  that wants that revision already gets it, so the default decides nothing on the wire.

### Tests

- 3,016 assertions, from 2,928. `tests/bote.tcyr` has 819, from 740, and `tests/conformance.tcyr` 215, from 206. All of them also
  pass under `qemu-aarch64`.
- 40 mutants, all killed, on `src/resources.cyr` and `src/dispatch.cyr`. Putting the 4 KB back fails at 124,800 B per message.

### Performance

No regression. Interleaved A/B, 15 alternating rounds, pinned to one core, 3.5.2 against this tree, medians in ns:

| Bench | 3.5.2 | 3.5.3 | Delta |
|---|---|---|---|
| `codec_process_message` | 4,048 | 3,943 | −2.6% |
| `dispatch_tools_call` | 2,163 | 2,094 | −3.2% |
| `dispatch_tools_list` | 1,800 | 1,643 | −8.7% |
| `http_era_legacy` | 2,466 | 2,433 | −1.3% |
| `http_era_modern` | 11,230 | 11,276 | +0.4% |
| `resource_template_match`, new | | 166 | |

These rows' code did not change, apart from `_dsp_room`, and other unchanged rows moved up to +6.3%. That is this binary's layout,
not a win.

## [3.5.2] — 2026-11-01 · `Mcp-Param` headers, and what the official SDK showed bote had wrong

`Mcp-Param-{Name}` closes the last transport gap in MCP 2026-07-28, and `bote bridge` now holds a modern request to the same
rules as the other two HTTP servers. The handler ABI and the stated minimum cyrius (6.7.13) are unchanged.

**No third-party client has run against bote yet.** The official packages, `@modelcontextprotocol/client` and `server` 2.4.0 and
`conformance` 0.2.0-alpha.12, were read as text and not executed. The suite's server scenarios were replayed in 109 checks by
a script of bote's own, over real sockets on `http`, `streamable`, `bridge` and stdio. That found two things no test of bote's
own had, both under Fixed. It is evidence about those clauses, not a run of the suite.

### Fixed

- **A JSON-RPC error on a `500` was unreadable to the official client.** 3.5.1 sent `-32603` as `500` and every `-32602` as `400`,
  reasoning that the spec gives them no status and `400` meets "a request missing any required field" on any reading. That
  clause is about `_meta`, in `basic/index`, and the official client takes a JSON-RPC error out of a `200` or a `400` and
  turns anything else into a generic HTTP failure, so a handler's internal error reached it with the error gone. A `5xx` is
  also what a proxy retries. Now `-32602` is `400` only for `_meta`, which is checked before dispatch, and every other
  `-32602` and `-32603` is in-band on a `200`, as the reference server does.
- **bote refused its own loopback Origin**, which the conformance suite's required `dns-rebinding-protection` scenario sends and
  expects to pass. With no list set, an Origin that is `http://` plus the request's `Host`, on `localhost`, `127.0.0.1` or
  `[::1]`, now passes. Nothing on bote's port serves a page, and a rebinding page's Origin carries the attacker's name. Another
  local port is still refused, and a list, once set, is the whole policy.
- **`bote bridge` served a modern request with no header checks.** It runs `bote_http_era` now, with CORS on every reply, modern
  or refused. Its legacy contract is its own and stays: a failed call is an `isError` envelope and a notification is `204`.
  One legacy change comes with it: an `MCP-Protocol-Version` bote does not speak is now `400` + `-32022` here, as on the
  other two. The bridge used to ignore that header.
- **A property name written with an escape was not enforced in a nested schema.** `_compile_object_body` kept
  `"c\u006et"` as those bytes, while the validator matches keys as they decode, so the rule on `cnt` was never applied.

### Security

- **A header field on two lines is refused** (`-32020`), for `MCP-Protocol-Version`, `Mcp-Method`, `Mcp-Name` and `Mcp-Param-*`.
  bote read the first line, and a proxy that reads the last would act on a value bote never compared. RFC 9110 §5.3 combines the
  lines into one value, which no body value matches.
- `Mcp-Param` refusals name the header and not its value, and neither value is logged.

### Added

- **`Mcp-Param-{Name}`.** A tool marks a property with `x-mcp-header` in its schema, and a modern `tools/call` is held to it:

  | Body | Header | Answer |
  |---|---|---|
  | A value | Equal, once decoded | Served |
  | A value | Missing | `400`, `-32020` |
  | A value | Different, malformed, on two lines, or Base64 that does not decode | `400`, `-32020` |
  | `null` or absent | Absent | Served |
  | `null` or absent | Present | `400`, `-32020` |
  | An object or an array | Either | `400`, `-32020` |
  | An integer past ±(2^53 − 1) | Either | `400`, `-32020` |

  Strings are compared decoded, and a `\u0000` matches nothing, since no header can say it. Integers are compared by value, so
  `42.0` is `42`, and a header that is not a JSON number is a mismatch. Booleans are `true` and `false`. A string where an
  integer belongs, or a fraction, passes this check when the header mirrors it and is then the tool's input error, which the
  model can correct. A header the tool does not declare is ignored, and the legacy path asks for none.

  Two rows go past the reference server, which ignores a header with no body value and lets an object through to validation.
  The spec says a header must match the body and that a value in the body needs its header, and no conforming client meets
  either refusal.
- **`registry_check_tool_def` refuses a bad `x-mcp-header`**, because a client must drop such a tool: an empty or non-token name,
  a type other than string, integer or boolean (`number` included), a repeat ignoring case, or anywhere but a chain of
  `properties` keys. The walk reads only keywords that hold a subschema, so a property named `x-mcp-header`, or a `default`
  holding the word, is not an annotation. `tool_def_param_headers`, `param_header_field` and `param_header_path` read the result.
- `ToolDef` gains a tail field, `param_headers` at `+64`, 64 → 72 B, set at registration. Appended, as the 2.0 shape allows.
- `_jx_members`, the one walk over an object's unknown keys, which `_compile_object_body` now uses in place of its own.
- `bote_origin_self`, `bote_http_origin_ok`, and `bote_http_ctx`.

### Breaking

These reach only code that calls the HTTP functions 3.5.1 added, or a tool with a bad annotation. The handler ABI is untouched.

- `bote_http_era(hc, body, require, dispatcher, req_out)`, `bote_http_check_version(hc, hv, require, body)` and
  `bote_http_serve_modern(hc, req, dispatcher, claims)` take a context in place of the socket, the transport's name and the
  request, and `bote_http_send_accepted(cfd, extra)` takes response headers. To migrate, build the context once in the handler:

  ```
  var hc[40];
  bote_http_ctx(&hc, cfd, "http", buf, n, 0);
  ```
- A tool that declares a bad `x-mcp-header` no longer registers, and `registry_register` says why. Fix the annotation or remove
  it. No conforming HTTP client could have listed that tool.

### Changed

- `_he_name_value_a` is `_he_field_value_a`, and `_he_header_ok` is `_he_field_ok`.
- The default stays 2025-11-25. A default that no real client has exercised would be a claim bote cannot back.

### Tests

- 2,928 assertions, from 2,158. `tests/bote_http_era.tcyr` has 1,212, from 517: every clause of the spec's value table, its
  Base64 examples and the conformance suite's Base64 cases, on all three HTTP transports, with the heap rows. `tests/bote.tcyr`
  has 740, from 668, and `tests/conformance.tcyr` 206, from 203.
- `scripts/transport-smoke.sh` runs a modern request over every transport, `unix` and a real WebSocket frame included.
- `fuzz/http_era.fcyr` sweeps a request with a header of each kind.
- 71 mutants, all killed, on `src/http_era.cyr`, `registry.cyr`, `session.cyr`, `bridge.cyr` and `jsonx.cyr`. One survivor was a
  real find: a tool lookup with a null name crashes only when a tool has landed in slot 0 of the map, because the hash is
  seeded per process, so the bug that crashed this patch's first test run passed three runs of four. The test now fills that slot.

### Performance

No regression on a path that existed. Interleaved A/B, 15 alternating rounds, pinned to one core, 3.5.1 against this tree,
medians in ns:

| Bench | 3.5.1 | 3.5.2 | Delta |
|---|---|---|---|
| `codec_process_message` | 4,050 | 4,095 | +1.1% |
| `dispatch_tools_call` | 2,149 | 2,202 | +2.5% |
| `dispatch_tools_list` | 1,784 | 1,820 | +2.0% |
| `dispatch_initialize` | 1,804 | 1,790 | −0.8% |
| `http_era_legacy` | 2,501 | 2,501 | 0.0% |
| `http_era_modern` | 9,952 | 11,413 | +14.7% |
| `http_era_params`, new, two `Mcp-Param` headers | | 19,461 | |

The `+14.7%`, 1.5 µs, is the three scans for a repeated field, on the modern path only. A first version cost `+33%`, and two
pieces of that were waste: a capabilities check at the edge that the dispatcher repeats, and a tool name read twice. Rows whose code did not change moved
between −6.2% and +4.3%, in both directions, which is this binary's layout and not a cost.

Stdio is byte-identical to 3.5.1 over 200,000 messages, and legacy HTTP on 50 of 50 cases, with only the version string
aligned.

## [3.5.1] — 2026-11-01 · MCP 2026-07-28 on HTTP, and `bote streamable` no longer turns away a client's first request

`bote http` and `bote streamable` now speak the current revision. A POST that names `2026-07-28` has its headers checked
against its body and is answered with the status each outcome calls for, and nothing else on the endpoint changes for it:
a legacy client beside it keeps its sessions and its `GET` stream. The handler ABI and the stated minimum cyrius (6.7.13)
are unchanged. `Mcp-Param-{Name}` headers are the last transport gap, roadmap 3.5.2.

> **Corrected at 3.5.2:** the `-32602` and `-32603` rows below were wrong. Only a `_meta` failure is `400`; the rest are
> in-band on a `200`, because the official client cannot read a JSON-RPC error out of a `500`. See that entry.

Working on that path showed a legacy defect that matters more than the new feature: see Fixed.

### Fixed

- **`bote streamable` refused a client's first request.** It demanded `MCP-Protocol-Version` on every POST and answered `400`
  without it, but 2025-11-25 asks for that header only on requests after `initialize`, because a client has no version to
  send until the reply gives it one, and revisions before 2025-06-18 do not define the header at all. A client of 2025-03-26
  could not use this transport, whatever bote advertised, and a later one could only by sending the header early, which is
  what `scripts/transport-smoke.sh` happened to do, so nothing failed. A POST with no header is now treated as `2025-03-26`,
  as the spec allows. `streamable_config_with_require_protocol` used to have no effect and now does, with `initialize`
  exempt. To keep the old strictness, set it.
- **A notification is `202` with no body**, on both HTTP transports. It was `204`, which neither revision allows.
- **A `405` names what is allowed**, as RFC 9110 §15.5.6 requires, on all four HTTP-family transports.
- **`prompts/get` reads its name as what it decodes to**, like `tools/call` and `resources/read`. `"code_review"` was a
  prompt that did not exist. A value validated one way and read another is the bug the one-scanner rule exists for.
- An unsupported `MCP-Protocol-Version` is `400` with `-32022` and `data.supported` in the body, in either era. It was an
  empty `400`, which a modern client would read as a legacy server.

### Security

- **A long `MCP-Protocol-Version` grew the heap by 83,886 B per request** on http and streamable: after the bearer check
  where auth is on, and with no credentials where it is not. `validate_protocol_version` built its refusal message on the heap for a value the client
  wrote. The transports now ask `bote_protocol_known`, which allocates nothing, and WebSocket's upgrade does too. Measured
  on real binaries with 40 KB headers: 83,886 B → 0.
- On the modern path a header that disagrees with the body is refused, so what a gateway routed on is what runs.

### Added

- **`src/http_era.cyr`**, shared by `bote http` and `bote streamable`. `bote_http_era` reads `MCP-Protocol-Version` and `_meta`
  once the body is in hand, because a refusal has to carry the request's id, and refuses, takes the legacy path, or takes
  `bote_http_serve_modern`:

  | Request | Answer |
  |---|---|
  | Unsupported version | `400`, `-32022`, with `data.supported` and `data.requested` |
  | Modern `_meta` and no version header, or the header and `_meta` disagree | `400`, `-32020` |
  | Modern header and `_meta` with no version | `400`, `-32602` |
  | `Mcp-Method` missing, invalid or not the body's method | `400`, `-32020` |
  | `Mcp-Name` missing, invalid, undecodable or not the body's name or URI, on `tools/call`, `resources/read`, `prompts/get` | `400`, `-32020` |
  | Batch, JSON-RPC response or non-object | `400`, `-32600` |
  | Invalid JSON | `400`, `-32700` |
  | Unknown method, `ping`, `initialize` | `404`, `-32601` |
  | Any other `-32602`: unknown tool, missing resource, invalid cursor | `400` |
  | `-32603` | `500` |
  | Notification | `202`, no body |
  | `GET` or `DELETE` naming this revision | `405`, with `Allow` |

  The spec gives no status for the last `-32602` row or for `-32603`. `400` for every `-32602` meets "a request missing any
  required field" on any reading of it, and a client takes a `400` with a recognized error as a modern server's answer.
  Precedence is not specified either: an unsupported version comes before a mismatch, since it says what bote does speak.
- **The Base64 sentinel**, `Mcp-Name: =?base64?…?=`, decoded before the comparison: lower-case markers, canonical Base64. The
  decoder is bote's own, because `bayan_base64_decode` keeps its output on the heap and this is a client's header on a path
  that may refuse it. Checked against bayan's by the new fuzz harness.
- `Mcp-Session-Id` and `Last-Event-ID` are ignored on the modern path, and no session is minted. Replies are
  `application/json`, which the spec lets a server choose per request.
- `bote_protocol_known`, `bote_http_log_refusal`, `bote_http_refuse_method`.

### Changed

- `validate_protocol_version` accepts `2026-07-28`, so a WebSocket upgrade naming it is no longer refused.
  `_mcp_is_supported` does not: `initialize` still never negotiates it.
- `dist/bote.cyr` has 34 modules.

### Performance

No regression on the paths that existed. Interleaved A/B, 15 alternating rounds, pinned to one core, 3.5.0 against 3.5.1,
medians in ns:

| Bench | 3.5.0 | 3.5.1 | Delta |
|---|---|---|---|
| `codec_process_message` | 3,844 | 3,863 | +0.5% |
| `dispatch_tools_call` | 2,050 | 2,055 | +0.2% |
| `dispatch_tools_list` | 1,699 | 1,705 | +0.4% |
| `dispatch_initialize` | 1,726 | 1,721 | −0.3% |
| `http_era_legacy` | new | 2,384 | |
| `http_era_modern` | new | 9,516 | |

Over real loopback HTTP, a legacy `tools/list`, 12 rounds of 1,500: 229.4 µs → 228.2 µs, inside the noise (220.5–233.4 and
210.5–235.0). A modern POST's checks are about 4% of such a round trip.

### Not done

- `Mcp-Param-{Name}`, for a tool that declares `x-mcp-header`. No built-in tool does, but an embedder can write the annotation
  in a raw property schema, and bote would advertise it and not check it. Roadmap 3.5.2.
- `-32021`, and SSE on the modern path, which waits for `subscriptions/listen`.
- **No third-party MCP client has been run against this.** Every check is against the spec's text. Roadmap 3.5.2.

### Proof

- **Legacy stdio:** 200,000 fuzzed messages byte-identical to 3.5.0 with the version string and the `prompts/get` line put
  back, so that line is the only change there.
- **Legacy HTTP:** 50 requests on each binary, across http and streamable, 36 identical. Of the 14 that differ, 3 are the
  version string in `initialize` and 11 are the fixes above.
- **The fuzzer caught a crash of this patch's own, before it shipped.** With `require_protocol` set, a body with no `method`
  dereferenced null. `fuzz/http_era.fcyr` found it, the real handler died with SIGSEGV under a unit test, and that test now
  pins it.
- **Mutation:** 52 mutants, all killed. Five survived at first. Two showed that no test could see a handler sending a second
  reply, so every clause now asserts exactly one.
- **Arena:** a build that overwrites the arena on every rewind matched a normal one over 100,000 messages and passes
  `tests/bote_http_era.tcyr`, `tests/conformance.tcyr` and `tests/bote.tcyr`.
- **Real binaries:** `scripts/transport-smoke.sh` drives a modern client and a legacy one on the same `bote http` and
  `bote streamable`.
- Tests: 1,637 → 2,158, on x86_64 and under `qemu-aarch64`, 517 of them in the new `tests/bote_http_era.tcyr`, which runs each
  clause through the real `_http_handle` and `_strm_handle` over a socketpair. agnos compiles with 0 undefined and does not
  run here, and that file's handler tests are compiled out there.

## [3.5.0] — 2026-11-01 · MCP 2026-07-28's stateless core beside the legacy revisions in one process, and the heap hole 3.4.15 left open

The first patch of the dual-era minor. A request that names `2026-07-28` in `params._meta` gets that revision, and every
other request gets what it got before: 200,000 fuzzed legacy messages came back byte-identical to 3.4.15's. A conforming
client reaches it over stdio and Unix. **Streamable HTTP does not speak it yet:** it answers `400` to
`MCP-Protocol-Version: 2026-07-28`, the header such a client sends on every POST. That is 3.5.1. The handler ABI and the
stated minimum cyrius (6.7.13) are unchanged.

The bench A/B for the router led to a separate finding: **3.4.15's claim that a client cannot grow the heap was wrong for
long values.** See Security.

> ⚠ **Corrected at 3.5.1.** The fix below missed one more long value: a client's `MCP-Protocol-Version` still grew the heap,
> by 83,886 B per request. See 3.5.1's Security section. This entry's "Streamable HTTP does not speak it yet" also understated
> what was wrong there: that transport was refusing legacy clients too. See 3.5.1's Fixed section.

### Security

- **A refused request with a long value still grew the heap, with no auth.** 3.4.15 moved request handling onto an arena
  and measured it, but its measurements put the long value in the padding, a token or the arguments. A long `id`, method,
  name, URI or cursor went a different way. The error serializer sized its builder at 256 bytes and then wrote those
  values through the plain builder, which grows on the heap even when its first buffer is on the arena. `prompts/get`,
  `resources/read` and the cursor check read their values onto the heap before refusing them, and the bridge built its
  `tools/call` errors there. The binary registers both a prompt and a resource, so all of this was reachable on a stock
  server. At
  the 1 GiB the closeout audit saw kill the process, 577 KB per request is about 1,900 requests.

  Measured on real binaries, RSS growth per request over 800 to 1,200 requests, on http, bridge, streamable and stdio:

  | Request | 3.4.15 | 3.5.0 |
  |---|---|---|
  | 60 KB unknown method | 60,293–61,167 B | 0 |
  | 60 KB string `id` on an unknown method | 60,293–61,167 B | 0 |
  | 60 KB unknown tool name | 60,293–120,586 B | 0 |
  | 60 KB unknown prompt | 120,586 B | 0 |
  | 60 KB unknown resource | 298,844–300,592 B | 0 |
  | Resource URI of 10,000 `\u0001` escapes | 576,717–579,338 B | 0 |
  | 60 KB cursor | 60,293–61,167 B | 0 |

  Unix and WebSocket share this code and were not measured. The same fix sizes the `-32022` reply, a tool error result
  (a validator message quotes the client's value: `value '…' not in enum`) and the deprecation event. `tests/bote.tcyr`
  adds 17 assertions, including that what a prompt or resource handler keeps survives the next message.
- ⚠ **Still grows, as before:** a valid tool, prompt, resource or completion call leaves what its handler is handed and
  whatever the handler allocates, because the 2.0 ABI lets a handler keep them. Roadmap 3.8.x.

### Added

- **Per-request protocol selection.** `dispatcher_dispatch_a` is now a router, and `_dsp_serve_a` holds the methods for
  both eras, so handlers, validation, audit and events are the same on each. `_meta`
  `io.modelcontextprotocol/protocolVersion` selects: `2026-07-28` is the stateless path, a supported older revision and a
  request with no version are the legacy path, and anything else is `-32022` with `data.supported` and `data.requested`.
  An `initialize` is always legacy, so it never lands on a revision that has no `initialize`. Nothing carries between
  messages. Required `_meta` fields missing or of the wrong type are `-32602`.
- **`server/discover`**, which servers MUST implement: `resultType`, `supportedVersions`, `capabilities`, `ttlMs`,
  `cacheScope`. Without a modern `_meta` it is `-32602`.
- **The modern result shape.** `resultType: "complete"` on every result, `serverInfo` in
  `_meta["io.modelcontextprotocol/serverInfo"]`, and `ttlMs` + `cacheScope` on discover, the three lists,
  `resources/templates/list` and `resources/read`. What a handler already set is left alone, since a repeated key would
  make the object ambiguous. The hints are `0` and `"private"`, and no `listChanged` is advertised, because this revision
  has no way to deliver a change until `subscriptions/listen` (roadmap 3.7.x).
- **Era-specific errors.** A missing resource is `-32602` on the modern path and stays `-32002` on the legacy one. A
  modern `ping` is `-32601`. A handler result that is not an object is `-32603`, since it cannot carry the fields.
- `resp_error_data_a`, `wrap_tool_result_a`, `wrap_error_result_a`. The plain forms are unchanged.

### Changed

- The router makes one pass over the params for `_meta` and the fields `tools/call` reads, through the one scanner. An
  escaped `_meta` key selects the same way.
- `tests/conformance.tcyr` M1–M75, `scripts/transport-smoke.sh` runs both eras in one stdio process, and `SECURITY.md`'s
  supported-versions table, which still named 3.3.x, is current.

### Performance

The first router cost `dispatch_tools_call` **+18.9%**. Two `streq` calls on every legacy request were about a third of
that, and a second walk of the params was the rest. Reordering removed the first and sharing `tools/call`'s pass removed the
second. Interleaved A/B, 15 alternating rounds, pinned to one core, 3.4.15 against 3.5.0, medians in ns:

| Bench | 3.4.15 | 3.5.0 | Delta |
|---|---|---|---|
| `codec_process_message` | 3,755 | 3,808 | +1.4% |
| `dispatch_tools_call` | 1,995 | 2,030 | +1.8% |
| `dispatch_tools_call_result` | 1,828 | 1,843 | +0.8% |
| `dispatch_tools_list` | 1,533 | 1,674 | **+9.2%** |
| `dispatch_initialize` | 1,394 | 1,712 | **+22.8%** |

`dispatch_tools_call` read +0.3% and +2.8% in two other runs, so take it as 0 to 3%. The two slower rows are real and
are not the path that matters: a client makes those calls once. `initialize` pays for a scan it used to skip, about 140 ns
measured with the scan removed. `tools/list` spends about 15 ns on the scan, and shifting the code by 24 bytes changed
nothing, so the rest is most likely the router's frame and key table.

### Not done

- Streamable HTTP's modern path, roadmap 3.5.1.
- `-32021`: no request bote serves needs a client capability yet, so capabilities are required and not otherwise read.
- `subscriptions/listen`, cancellation and progress on the modern path, roadmap 3.7.x.

### Proof

- **Legacy unchanged:** 200,000 fuzzed messages, 2 seeds, byte-identical to 3.4.15 with the version string put back.
- **Arena:** a build that overwrites the arena on every rewind matched a normal one over 100,000 messages and passes
  `tests/bote.tcyr` and `tests/conformance.tcyr`.
- **Mutation:** 40 mutants of the router and the heap fix, all killed. Three survived at first, on behaviour with no test,
  and those tests were then added.
- **Real transports:** a modern `server/discover` answered on stdio, Unix, WebSocket, HTTP and the bridge on the built
  binary, by hand. Only stdio is in `scripts/transport-smoke.sh`.
- Tests: 1,545 → 1,637, on x86_64 and under `qemu-aarch64`. agnos compiles with 0 undefined and does not run here.

## [3.4.15] — 2026-11-01 · closeout: a website can no longer reach bote, a request leaves nothing on the heap, and the roadmap follows the current spec

The P(-1) pass that closes 3.4.x. Four independent reviews found no memory-safety defect and a good deal else: any
website could drive `bote bridge`'s file tools, the other HTTP transports did not validate `Origin`, and an
unauthenticated client could kill the server in about 12 seconds. All of that is fixed. The full findings, and what
was deferred and why, are in [docs/audit/2026-11-01-closeout.md](docs/audit/2026-11-01-closeout.md). The handler ABI
and the stated minimum cyrius (6.7.13) are unchanged. MCP's current revision is 2026-07-28, which bote does not speak
yet; that now leads the roadmap.

> ⚠ **Corrected at 3.5.0.** "A request leaves nothing on the heap" held for the scenarios measured here, not for every
> request: a long value in a refused request's `id`, method, name, URI or cursor still grew the heap, with no auth. See
> 3.5.0's Security section.

> ⚠ **Corrected at 3.7.0.** The request arena did not stay bounded. lib/alloc.cyr's growable arena, when the chunk after
> the one in use was too small, linked a new chunk in front of it and kept both, so messages that each needed a little
> more than the one before kept a chunk apiece: 300 refused `resources/read` calls, the URI growing from 12 KB to 42 KB,
> kept 105 MB, with no auth. Every heap row here repeated one size, which keeps nothing. bote has its own arena since 3.7.0.

### Security

- **A website can no longer reach bote.** A present `Origin` not on `BOTE_ALLOWED_ORIGINS` gets 403 on every
  HTTP-family transport, before anything else runs, and with no list set every present `Origin` is refused. Through
  3.4.14 no list meant no check, the binary had no way to set one, and `bote bridge` answered
  `Access-Control-Allow-Origin: *` with the fs tools registered, so any page the operator opened could read and write
  under `BOTE_FS_ROOT`. A client that sends no `Origin` is unaffected. A refused bridge request no longer reaches a
  tool: a miss used to dispatch and only hide the reply, and `Origin: null` matched a sandboxed iframe.
- **An unauthenticated client can no longer exhaust memory.** See Memory below.
- **A schema-checked integer is the integer the handler reads.** `libro_retention {"value":1e3}` validated as 1000 and
  the handler read 13, because `str_to_int` skips non-digits; 2^64 wrapped to 0. Python's `json.dumps(1000.0)` writes
  `1000.0`, so ordinary clients reach this. Handlers now read through the validator's own parser, and an integer past
  what can be held is refused.
- **`web_fetch` and `web_search` have deadlines,** `BOTE_WEB_TIMEOUT_MS`, default 15 s. One silent URL used to stop every
  client for good. DNS is not covered: sandhi's resolver has no timeout yet (roadmap, Upstream).
- **The fs tools stay in their root through a symlink.** Confinement was a string check, so any link already in the root
  escaped it, for read, write and mkdir. A path may no longer pass through one, `fs_write` replaces a link rather than
  writing through it, and a FIFO no longer hangs the server. The check is by name, because the stdlib has no `openat`.
- **The audit trail names the configured identity.** A client's `caller_id` chose the `agent_id` in the hash-linked chain;
  it now stays in the details as a claim. A refused `tools/call` also reaches the audit and event sinks now, where it
  left no trace.
- **`BOTE_BEARER_TOKENS` set with no token refuses to start.** It logged a WARN and served with auth off.
- **`ssrf_check` on its own** now reads an authority as a client does: `http://169.254.169.254?@example.com/`,
  `http://a@b@127.0.0.1/` and `http://0x7f.0.0.1/` passed it. `web_fetch` was already safe, at the resolved address.
- **Sessions expire.** Nothing checked a session's age and nothing called the pruner, and `initialize` minted one before
  validation. A session is now minted after `initialize` succeeds, checked for age, and the store holds at most
  `BOTE_SESSION_MAX` (4096), answering 503 when full of live sessions.

### Fixed

- **One key scanner, and it decodes.** `_jx_scan` is now the only walker of a JSON object's keys, and `_jx_find_value_pos`
  is its one-key form. A key matches as what it decodes to, so `"c\u006et"` cannot skip a type check that `cnt` would
  meet, and `method`, tool names, versions and URIs are decoded: `"method":"tools\/list"` was `-32601`. A decoded NUL
  matches nothing. The decoder moved from `fs_tools` into `jsonx`.
- **Every response string is valid UTF-8** (RFC 8259 §8.1). `bote_json_escape` replaces an invalid sequence with U+FFFD, at
  the one point every string passes through. Binary files, Latin-1 pages and `&#233;` made strict parsers refuse the
  whole message.
- **Every handler's result is one line of valid JSON.** `prompts/get`, `resources/read` and `completion/complete` sent
  theirs unchecked (now `-32603`), and a pretty-printing handler split a line on stdio and Unix.
- **Numbers:** `-0.9` passed `minimum: 0` and `10.9` passed `maximum: 10`. **Schemas:** the schema `false` allowed
  everything; it now allows nothing, and registration warns about each keyword it advertises and does not enforce.
- **Line framing:** an overlong line lost its head silently and ran its tail as a new message. It now gets one `-32700`
  and is skipped to its newline, on stdio and Unix.
- **HTTP:** a `Content-Length` that is no number is 400, a NUL hiding trailing bytes is 400, and the POST SSE piggyback stops
  at the first failed write, as the GET path does.
- **Unix socket:** a path over 107 bytes is refused rather than bound truncated, a failed chmod is an error, and only a
  socket is replaced at the path, not somebody's file.
- **fs tools:** `fs_mkdir` reports failure, `fs_read` says when it truncated and refuses a binary file rather than cutting it at
  the first NUL, surrogate pairs and escaped paths are decoded, and a failed write leaves the old file whole.
- **JSON-RPC:** a request with a readable id and no method is answered with that id. `completion/complete` refuses non-object
  params. A null tool or nameless prompt is refused at registration, where it crashed the list that followed.
- **`libro_query`:** `offset` + `limit` could wrap and return nothing with `ok:true`.
- **`web_fetch`:** a page over 256 KiB failed with "transport error"; `WEB_BODY_CAP` (2 MiB) was dead code and now applies.
- **`bote ws`** gains `BOTE_WS_IDLE_MS`. SECURITY.md promised an opt-in the binary could not reach.
- **Logging:** a long line overflowed sakshi's buffer and glued two records together; bote's cap now sits below it.

### Memory

`alloc` never frees, and nothing was rewound between requests, so a server's memory was bytes-per-request times requests.
Every byte bote allocates to answer a message now goes on a request arena (`bote_req_begin`, in `protocol.cyr`), rewound when the
next message starts, on every transport. New `_a` forms carry it through the codec, dispatch, validation and the responses; the old
entry points are wrappers and behave as they did. Measured on the real binaries, RSS growth:

| Scenario | 3.4.14 | B / request | 3.4.15 | B / request |
|---|---|---|---|---|
| 60 KB ping, no auth, 3,000 requests | 174 MiB | 60,817 | 0 | 0 |
| 60 KB rejected token, 2,000 | 230 MiB | 120,586 | 0 | 0 |
| 60 KB arguments to an unknown tool, 2,000 | 346 MiB | 181,404 | 0 | 0 |
| `tools/list` over http, 3,000 | 96 MiB | 33,554 | 0 | 0 |
| `tools/list` over stdio, 20,000 | 630 MiB | 33,030 | 0 | 0 |
| WebSocket header claiming 1 MiB, 300 connections | 300 MiB | 1,048,576 | 0 | 0 |

- ⚠ **What still grows:** a tool call that passes validation leaves its `args` and whatever its handler allocates, because
  the 2.0 ABI lets a handler keep them. `args` reaches the heap only past the last refusal. Roadmap 3.8.x.
- WebSocket frames are read through the public `ws_server_recv_frame` into one buffer, so a header claiming 1 MiB stores
  nothing. Unix uses one read buffer instead of 128 KiB per connection. Validation stops at `BOTE_SCH_MAX_VIOLATIONS` (32).

### Changed — behaviour an embedder can see

- `transport_http_run`, `transport_streamable_run` and `transport_ws_run` refuse a present `Origin` when no list is configured.
  Pass a list, or `*`, with `*_config_with_allowed_origins`. `bridge_config_with_origins` with an empty list refuses too.
- `session_store_create` can return 0, and `session_store_validate` refuses an expired session.
- The libro audit sink no longer takes `agent_id` from `caller_id`.
- An audit or event sink now sees refused calls.
- `fs_write` is atomic. `fs_read` of a file holding a NUL is an error.

### Performance

WebSocket replies waited on the peer's delayed ACK, because a frame's header and payload are separate writes. `TCP_NODELAY`
takes a round trip from **41,000 µs to 50 µs**, median.

Interleaved A/B: 15 alternating rounds, pinned to one core, load about 9, 3.4.14 against 3.4.15, medians:

| Bench | 3.4.14 | 3.4.15 | Delta |
|---|---|---|---|
| `codec_process_message` | 6,158 ns | 4,292 ns | −30.3% |
| `dispatch_tools_call` | 3,172 ns | 2,290 ns | −27.8% |
| `dispatch_tools_call_result` | 2,871 ns | 2,105 ns | −26.7% |
| `codec_parse_request` | 1,528 ns | 969 ns | −36.6% |
| `codec_serialize_response` | 491 ns | 215 ns | −56.2% |
| `validate_compiled_nested` | 2,372 ns | 2,037 ns | −14.1% |
| `jsonx_get_str_flat` | 150 ns | 187 ns | **+24.7%** |
| `jsonx_get_raw_nested` | 423 ns | 462 ns | **+9.2%** |
| `schema_compile_nested` | 6,167 ns | 6,652 ns | **+7.9%** |
| `auth_bearer_check_set` | 830 ns | 872 ns | **+5.1%** |

The four slower rows are real: a single-key lookup now goes through the one scanner, and decodes escaped keys. The pipeline
those lookups sit inside is 30% faster. The other wins: one pass over a message, a success response sized once, no violations
vec on success, and streamable no longer building an event its sink discards.

### Added

- `BOTE_ALLOWED_ORIGINS`, `BOTE_WEB_TIMEOUT_MS`, `BOTE_WS_IDLE_MS`.
- Tests: 1,420 → 1,545. `tests/bote.tcyr` asserts 0 heap bytes per message on every refusal, and that what a handler or sink
  keeps survives the next message. `scripts/transport-smoke.sh` gains what a unit test cannot set up: hostile Origins on every
  mode, real bearer tokens, a refused `initialize`, and symlinks and FIFOs under `BOTE_FS_ROOT`.
- **Mutation-proven.** 33 mutants of these fixes, each killed by an assertion, a smoke check or a hang.
- **Poisoned arena.** A build that overwrites the arena on every rewind gives byte-identical output to a normal build over
  100,000 fuzzed messages, and passes the suite, so no request reads another's bytes.
- [docs/audit/2026-11-01-closeout.md](docs/audit/2026-11-01-closeout.md).

### Changed

- **The roadmap is rewritten** around open 3.x work, decided on bote's own merits: the consumer adoption track is gone. MCP
  2026-07-28 is the new 3.5.x, as a dual-era server, because a client that speaks only that revision cannot talk to bote today.
  3.6.x is HTTP authorization, 3.7.x threaded dispatch rebased on `subscriptions/listen`, and the old plan's held-open `GET`,
  `logging/setLevel` and `resources/subscribe`, which the spec removed, are dropped. [spec-compliance.md](docs/spec-compliance.md)
  says so at the top, and lists the gaps it did not: `outputSchema`, `title`, `icons`, `instructions`, RFC 9728.
- CLAUDE.md: consumers are not this project's problem unless they report a bug; the request-arena and one-scanner rules.
- Binaries: `bote` 3,243,328 B (3.4.14: 3,213,704); aarch64 3,812,384 B; agnos 3,207,672 B with 0 undefined. `fn_table` 7016 / 131072.
  Every test file is green on x86_64 and under `qemu-aarch64` 11.1.3. The minimum measures the same: the full tree needs 6.7.13.
  `dist/bote.deps` and `dist/bote-core.deps` are unchanged.

### Deferred, with reasons in the audit report

The per-write send deadline against a slow reader (3.7.x), `libro_retention`'s dropped archive (open decision), `version`
selecting a schema but not a handler (3.9.x), and tying a session to its token.

## [3.4.14] — 2026-11-01 · a JWT signature has one spelling on every toolchain, and corrections to 3.4.13

`jwt_verify_hs256` now requires the signature segment to be exactly what the encoder writes, so one
signature is one token string whatever the toolchain's decoder accepts. Building this showed that what
3.4.13 said about the subject was wrong in three ways, corrected below. The handler ABI, the wire and
the stated minimum cyrius (6.7.13) are unchanged.

### Security

- **A signature has one token string, on every supported toolchain.** After the HMAC passes, the
  decoded signature is re-encoded and compared with the segment. What verifies, for one valid token:

  | Signature spelled | 3.4.13 on 6.7.13 – 6.7.17 | 3.4.13 on 6.7.18 | 3.4.14 on any |
  |---|---|---|---|
  | canonically | verifies | verifies | verifies |
  | with non-zero spare bits (3 forms) | verifies | refused | refused |
  | with one `=` | verifies | **verifies** | refused |
  | with any longer run of `=` | verifies | refused | refused |

  Nobody could forge a token this way: every spelling carries the same MAC. The risk was to anything
  keyed on a token's text, such as a revocation list or replay cache.
- **Reach: nothing, today.** bote's binary does not call the verifier (its auth is `BOTE_BEARER_TOKENS`),
  no sibling repo calls `jwt_verify_hs256` or `auth_validator_jwt_hs256`, and bote keeps no list keyed
  on a token. This is for the embedder that wires it up.

### Corrections to 3.4.13

- **"A JWT signature has one spelling" on 6.7.18 was false.** One `=` still verified there: bayan 1.11.1
  takes the padding that completes a group. The same claim was in `src/jwt.cyr`'s header.
- **"Four spellings" below 6.7.18 was an undercount.** It was unbounded: any run of `=` on each of the
  four spare-bit forms.
- **"bote's own binary is pinned to 6.7.18" implied a reach it never had.** See above.

### Changed — the guard is on the signature only, after the HMAC

- The header and payload need no such check, because the HMAC covers them as text and only a key holder
  can re-spell them. `jwt_b64u_decode` is unchanged, so a padded header with a good MAC still verifies.
- A first version put the check in `jwt_b64u_decode`, for all three segments. It was measured and
  dropped: it cost 5,040 ns and 376 B per valid verify, and it re-encoded the header before the HMAC, on
  unverified bytes, on a heap that never frees. Measured here, 9 alternating rounds, pinned to one core:

  | Per verify | 3.4.13 | 3.4.14 |
  |---|---|---|
  | valid token | 4,217 ns, 208 B | 4,466 ns, 256 B |
  | forged token | 3,158 ns, 136 B | 3,173 ns, 136 B |

### Changed — tests

- `tests/bote_jwt.tcyr` 60 → 59, 1,421 → 1,420 in all. Three padded-signature assertions are added. The
  four direct decoder assertions from 3.4.13 are removed: they pinned what bayan does on one toolchain,
  and the property now lives in the verifier.
- **Proven by running the file on real toolchains and mutating `src/`:**

  | `tests/bote_jwt.tcyr` | 6.7.13 | 6.7.17 | 6.7.18 |
  |---|---|---|---|
  | with the guard | 59 pass | 59 pass | 59 pass |
  | guard removed | 6 fail | 6 fail | 1 fail |
  | length checked, bytes not compared | 3 fail | 3 fail | **passes** |

  ⚠ That last cell is a limit, not a gap: on 6.7.18 the decoder already refuses what the compare would, so
  CI, which runs only the pin, cannot see the compare removed. Dropping the length check changes no
  answer on any toolchain, because a mismatch also differs at the terminator; it stays to keep `memeq`
  inside the allocation. Inverting either check is killed by a crash in the test, not by an assertion.

### Changed

- Binaries: `bote` 3,213,704 B, aarch64 3,745,904 B, agnos 3,182,184 B with 0 undefined, the same file sizes
  as 3.4.13. `fn_table` 6959 / 131072. Both `.deps` sidecars are unchanged. The full tree still needs
  6.7.13, and 6.7.12 stops on `undefined variable 'SANDHI_SERVER_FAIL_CONFIG'`.
- Every test file is green on x86_64 and under `qemu-aarch64` 11.1.3. Every unit compiles for agnos with
  0 undefined. CI's step scripts and `fmt`, `lint`, `vet` and `deny` are clean.
- Roadmap: canonical JWT signatures below 6.7.18 is closed, and nothing is queued.

### Performance

- `tests/bote.bcyr` links no JWT code. The logged row (load 3.7) is within noise of 3.4.12's:
  `dispatch_tools_call` 2.982 µs against 2.944, `codec_process_message` 5.818 µs against 5.749. 3.4.13's
  high row was the host, as its A/B said.

## [3.4.13] — 2026-11-01 · the core bundle stops requiring bayan, and a JWT change 3.4.11 left unrecorded

Since 3.4.11 no core module calls bayan, so `dist/bote-core.cyr` stops requiring it. Checking who does
call it turned up something 3.4.11 should have recorded: the same bayan changed which JWTs bote
accepts. That is now written down and tested. The handler ABI, the wire and the stated minimum cyrius
(6.7.13) are unchanged.

### Security

> ⚠ **Corrected at 3.4.14.** This section is wrong in three places: one `=` still verified on 6.7.18,
> spellings below 6.7.18 were unbounded rather than four, and bote's binary never called the verifier.
> See 3.4.14's corrections.

- **Recorded late: since 3.4.11 a JWT signature has one spelling, on cyrius 6.7.18 and later.** bayan
  1.11.1's base64 decoders refuse what is not canonical RFC 4648, and `jwt_b64u_decode` goes through
  them. The 3.4.11 entry missed this.

  | Input | 3.4.10 (6.7.9) | 3.4.11 and later (6.7.18) |
  |---|---|---|
  | `QR`, `QUF`: non-zero spare bits | accepted, the same bytes as `QQ`, `QUE` | refused |
  | `Q`: a one-character group | accepted, length 0 | refused |
  | a signature re-spelled in its spare bits | **verified** | refused |

  A 32-byte HMAC leaves 2 spare bits in its last character, so one signature had four valid token
  strings. A revocation list or replay cache keyed on the token's text could be walked around by
  re-spelling it. Forging a token was never possible: all four carry the same MAC.
- ⚠ **Only the 6.7.18 fold has this.** The bundles state 6.7.13, and 6.7.13 – 6.7.17 (bayan 1.9.0 – 1.11.0)
  still accept all four spellings. bote's own binary is pinned to 6.7.18, and no sibling repo sits in that
  range today. `src/jwt.cyr`'s header says so, and a guard that holds on every toolchain is an open
  decision on the roadmap.

### Changed — the core bundle

- **`dist/bote-core.deps` is `hashmap`** (was `hashmap bayan`), and its `# Requires` block matches.
  `include "lib/bayan.cyr"` leaves `src/registry.cyr`. The two modules that use bayan, `src/jwt.cyr`
  (`base64url_decode`) and `src/web_tools.cyr`, now include it themselves, so `dist/bote.deps` is
  unchanged. The 3.4.11 roadmap called `web_tools.cyr` the one user. That was wrong, from a grep that
  only knew bayan's prefixed names; this time every function and variable bayan defines was checked
  against each module.
- **What it saves depends on how a repo takes core.** One-call consumer on cyrius 6.7.18:

  | Consumer | 3.4.12 | 3.4.13 |
  |---|---|---|
  | copied core bundle, no `bayan` in its own `[deps] stdlib` | 1,239,776 B | 276,192 B (−77.7%) |
  | pinned core bundle | 1,797,872 B | 1,797,856 B |
  | pinned full bundle | 3,200,504 B | 3,200,504 B |

  A pinned consumer keeps bayan because bote's `deps.libro` brings libro's thin sigil, which includes
  it. That is the roadmap's G14 filing, and this measurement is now part of it.

  **Who this reaches:** no sibling binary shrinks today. hoosh, szal and thoth copy the bundle but call
  bayan themselves, and the rest pin it. Nothing breaks: all 12 repos that consume core declare `bayan`
  themselves. A copied consumer builds and runs on 6.7.2, 6.7.9, 6.7.12, 6.7.13 and 6.7.18.

### Added

- `tests/bote_jwt.tcyr` +7, 1,414 → 1,421 in all: non-zero spare bits and a one-character group refused
  by the decoder, and each of a signature's three other spellings refused by `jwt_verify_hs256`. The
  behaviour is bayan's, and a mutation under `lib/` never reaches the compiler, so the proof is the real
  toolchain: on cyrius 6.7.17 six of the seven fail, and the three token cases verify.

### Changed

- Binaries are the same size as 3.4.12's: `bote` 3,213,704 B, aarch64 3,745,904 B, agnos 3,182,184 B with 0
  undefined. Capacity: `fn_table` 6959 / 131072. The minimum measures the same: the full tree needs
  6.7.13, and 6.7.12 stops on `undefined variable 'SANDHI_SERVER_FAIL_CONFIG'`.
- Every test file is green on x86_64 and under `qemu-aarch64` 11.1.3. Every unit compiles for agnos with
  0 undefined. CI's step scripts and `fmt`, `lint`, `vet` and `deny` are clean.
- Roadmap: bayan in the core bundle is closed, canonical JWT signatures below 6.7.18 is a new open
  decision, and nothing is queued.

### Performance

- A wash. The logged row reads high (`dispatch_tools_call` 3.272 µs against 3.4.12's 2.944), and that is
  the host: load was 5.4 and both builds are equally slow on it. Interleaved A/B against the 3.4.12 tag,
  11 alternating rounds each, pinned to one core: `dispatch_tools_call` −0.1%, `codec_process_message`
  +0.0%, every row within ±1%, and `jsonx_get_str_flat` +1.9%. The two bench binaries are the same size.

## [3.4.12] — 2026-11-01 · a transport that cannot serve says which step failed

The roadmap's next item, and the last open thread from bote's sandhi filing. A failed start on http,
bridge, streamable or ws could only be logged as "socket, bind, listen or accept failed". Those four
now name the step and the errno, as the Unix transport has since 3.4.0. This raises the stated minimum
cyrius from 6.7.2 to 6.7.13.

### Changed — what a failed start logs

Started on a port that is already held:

| | Line |
|---|---|
| 3.4.11 | `[ERROR] transport failed transport=http rc=1 cause="socket, bind, listen or accept failed (is the port in use?)"` |
| 3.4.12 | `[ERROR] serve failed transport=http step=bind port=8390 errno=98 cause="port in use"` |
| | `[ERROR] transport failed transport=http rc=1` |

- **`step`** is one of `socket`, `bind`, `listen`, `accept`, `alloc`, `config` or `thread`, from sandhi's
  own record. `unknown` means the options struct itself could not be allocated.
- **`errno`** is left out for a step that has none.
- **`cause`** is on the new line only, and only for `EADDRINUSE` at bind. A monitor that matched the old
  `cause=` text on `transport failed` needs the new line. The exit code is still 1.
- The line comes from the transport, not the binary, so an embedder calling `transport_http_run` and
  the like gets it too.

### Added

- `bote_conn_serve(transport, addr, port, handler_fp, ctx)` in `src/conn.cyr`, the serve entry all four
  transports now share, with `bote_conn_log_serve_failure` and `bote_conn_serve_step_name`.
- **`tests/bote.tcyr` +31**, 1,383 → 1,414 in all: every step name, what each kind of failure logs, http
  and bridge on a port the test holds, and a new options struct reading as none.
- **`scripts/transport-smoke.sh`** starts each of the four modes again on a port already held, and
  requires exit 1, the step line and the exit line. The in-process tests reach only http and bridge, so
  this is what covers streamable and ws, and CI's "Transport smoke" step runs it.
- **Mutation-proven.** 17 mutants, each killed by an assertion or the smoke:
  - the serve never logging, or passing no options struct;
  - `cause` on any step, or on any bind errno;
  - `errno` logged when 0, the wrong step name, WARN for ERROR, the port dropped;
  - each of the four transports back on bare `sandhi_server_run`, and ws carrying http's label;
  - a new options struct differing in its idle budget, request budget, request cap or stop flag.

  The first form of that last group edited `lib/sandhi.cyr` and survived, because `cyrius test` restores
  `lib/` before compiling, even with `--no-deps`. Driving it with sandhi's setters from the test side
  kills all four.

### Changed — behaviour that did not

- **Serving is unchanged.** `sandhi_server_run` is `sandhi_server_run_opts` with `opts = 0`, and of the
  options that loop reads, a new struct gives the same idle budget (30 s), request budget (60 s), request
  cap and no stop flag. `tests/bote.tcyr` pins all four, so a sandhi default that drifts fails here and
  not in a client. WebSocket still replaces the inherited receive deadline as it did.

### Changed — the minimum cyrius

- **Both bundles now state 6.7.13.** `sandhi_server_last_failure` first folds at 6.7.13 (sandhi 1.13.1;
  6.7.12 still has 1.10.9), and there is no way for a bundle to test its toolchain. Measured with
  `scripts/bundle-min-cyrius.sh`:

  | cyrius | build | core smoke | ping | first failure |
  |---|---|---|---|---|
  | 6.7.1 | fail | — | — | `lib/patra.cyr:3765:24: expected identifier, got '-'` |
  | 6.7.2, 6.7.9, 6.7.10, 6.7.11, 6.7.12 | fail | — | — | `src/conn.cyr: undefined variable 'SANDHI_SERVER_FAIL_CONFIG'` |
  | 6.7.13, 6.7.14, 6.7.18 | ok | ok | ok | |

- **`dist/bote-core.cyr` itself needs no more than it did.** It has no transport, and a copied core bundle
  builds and runs on 6.7.2, 6.7.9, 6.7.12, 6.7.13 and 6.7.18. CI requires both bundles to state one
  minimum, so the line says 6.7.13 and `src/log.cyr` and DEPS-PATTERN.md give the detail.

  **Who this reaches:** nobody today. The two sibling repos that select the full bundle, daimon and
  itihas, are on cyrius 6.6.x and bote 3.3.x, already below the old minimum. A repo on 6.7.2–6.7.12 can
  take the full bundle at 3.4.11 at the newest.

### Changed

- Binaries: `bote` 3,213,704 B (3.4.11: 3,213,728); aarch64 3,745,904 B (3,745,912); agnos 3,182,184 B with
  0 undefined. Capacity: `fn_table` 6959 / 131072. Both `.deps` sidecars are unchanged.
- Every test file is green on x86_64 and under `qemu-aarch64` 11.1.3. Every unit compiles for agnos with
  0 undefined. CI's step scripts and `fmt`, `lint`, `vet` and `deny` are clean.
- Only the bind failure runs for real, on x86_64 and aarch64. The other steps are logged from a record
  set by the test, because they cannot be provoked on demand, and agnos's 98 for `EADDRINUSE` is read
  from its syscall peer's comment, not run.
- Roadmap: nothing is queued. bayan in the core bundle stays an open decision.

### Performance

- `tests/bote.bcyr` links neither `src/conn.cyr` nor any transport. The logged row (load 5.3) is within
  noise of 3.4.11's: `dispatch_tools_call` 2.944 µs against 2.988, `codec_process_message` 5.749 µs
  against 5.823.

## [3.4.11] — 2026-11-01 · cyrius 6.7.18, libro 2.11.3, majra 2.13.0, and one fallback off bayan's parser

The toolchain and both git deps move to their latest tags, and the fold brings sigil, patra, sakshi,
sandhi and bayan with them. The handler ABI, the wire and the minimum cyrius (6.7.2) are unchanged.
The move took one source change: bayan 1.11.1 refuses a JSON object with a repeated key, and
`registry_validate_params`'s fallback read arguments with that parser.

### Changed — the pins

| Dep | 3.4.10 | 3.4.11 | How it arrives |
|---|---|---|---|
| cyrius | 6.7.9 | 6.7.18 | the `cyrius =` pin |
| libro | 2.11.0 | 2.11.3 | `[deps.libro]` |
| majra | 2.12.0 | 2.13.0 | `[deps.majra]` |
| sigil | 3.13.11 | 3.13.15 | the fold; libro's pin now equals it |
| patra | 1.16.0 | 1.16.6 | the fold; libro's pin equals it |
| sakshi | 2.5.8 | 2.5.10 | the fold |
| bayan | 1.5.13 | 1.11.1 | the fold |
| sandhi | 1.10.9 | 1.13.7 | the fold |

Resolved from scratch in the CLAUDE.md order: 115 locked, four `commit` rows at the new tags,
`cyrius deps --verify` clean, and the versions re-read after `cyrius build`. libro's sigil pin is no
longer one patch past the fold, which closes that watch-list item.

What reached bote, and what did not:

- **libro.** `struct chain`, `struct entry` and `struct error` are identical to 2.11.0's, so the raw
  offsets in `src/libro_tools.cyr` hold. Thirteen function bodies changed. bote calls none of them
  directly. The two it reaches, `_proof_verify_common` and `_sb_proof_entry`, differ only in a log
  literal's length. 2.11.3's stricter JSONL readers (`chain_import`, `filestore_verify_streamed`,
  `filestore_load_and_verify`) are not on any bote path.
- **majra.** The bodies of `pubsub_new`, `pubsub_subscribe` and `pubsub_publish` are identical to
  2.12.0's. 2.13.0's PostgreSQL SCRAM and TLS are in `-backends`, which bote does not consume.
- **sandhi.** The serve entry points' 0 / 1 return is unchanged, so `bote_main_log_exit` still reads
  it correctly. 1.10.10's fix arrives with this fold: a failed bind or listen no longer leaks the
  listen socket.
- **kavach** (not a dep). Checked at 3.13.2: `ExecResult` still matches `src/sandbox.cyr`, and
  `sandbox_exec` is still 2-arg.

### Fixed

- **`registry_validate_params`'s required-fields fallback answers a repeated key correctly.** bayan
  1.11.1 refuses a JSON object with a repeated key, and `bayan_json_parse` returns 0 for one. The
  fallback read arguments with it, so on this pin `{"path":"/a","path":"/b"}` came back `missing
  required field: path` for a field that is there. RFC 8259 allows a repeated key and `jsonx_valid`
  accepts one, so this input can arrive from the wire. The fallback now asks `jsonx_has`, the same
  check `compiled_validate` makes for `required`, so the two paths agree.

  **How far this reached:** nowhere that shipped. Registration always compiles a schema, so only a
  `ToolDef` placed in the registry by other means gets to this path, and the wrong answer existed
  only between this pin move and this fix.

  Measured with a probe on both toolchains: on valid objects with no repeated key (17 cases) the old
  lookup and `jsonx_has` agree. They differ on five malformed shapes such as `{"a":}` and `{,"a":1}`,
  which `jsonx_valid` stops before dispatch. A library caller that skips the codec can see that
  difference.
- **The `json_get` deprecation warning is gone.** It was the one new warning this pin raised in
  `src/`, which stays warning-clean. Its suggested replacements were deliberately not taken:
  `bayan_json_get_by_str` and `_by_cstr` exist only from cyrius 6.7.17, in a module both bundles
  carry, so renaming the call would have raised their minimum from 6.7.2 to 6.7.17.

### Security

Three upstream fixes reach bote through the fold, with no bote change:

- **sigil 3.13.13–3.13.15:** X.509 nameConstraints (CYRIUS-2026-0042), the DER over-reads and a
  ChaCha20-Poly1305 forgery. `web_fetch`'s HTTPS client reaches these through sandhi.
- **`ws_server_recv` (CYRIUS-2026-0043):** every connection read into one process-global 1 MiB
  buffer, so two connections on a pooled server could read each other's bytes. bote's own WebSocket
  transport serves on the single-threaded `sandhi_server_run`, so the binary was not exposed. A
  consumer running `_bote_ws_handler` on a pool was. The buffer is gone, and per-frame allocation is
  what it was.
- **The WebSocket handshake and control frames:** `ws_server_handshake` refuses a key that is not 22
  base64 characters and `==`, and a control frame over 125 bytes fails the read.

### Changed — the bundles

- **`dist/bote-core.deps` drops `sigil` and `ct`,** now `hashmap bayan`. No core module uses either.
  They were there because `distlib` recorded libro's needs against the bundle, which cyrius 6.7.13
  fixed (bote's filing, open since 3.3.17). The bundle's `# Requires` block shrinks to match.
  `dist/bote.deps` is unchanged, and neither sidecar needs bayan's new `slice` / `hashseed`, which it
  includes itself.

  **Who this reaches:** a consumer that called sigil or ct from its own code and leaned on this
  sidecar. Of the 12 sibling repos that consume the core bundle, every one that makes such a call
  declares them, so none breaks.

  | Copied core bundle, one-call consumer | Binary | `string_data` |
  |---|---|---|
  | 3.4.10 on 6.7.9 | 1,709,968 B | 39,568 |
  | 3.4.11 on 6.7.9 | 718,640 B | 27,142 |
  | 3.4.11 on 6.7.18 | 1,239,776 B | 369,131 |

  Dropping sigil and ct saves 991,328 B. bayan's Unicode tables give 521,136 B of that back on
  6.7.18. The net is −470,192 B.
- **The minimum cyrius stays 6.7.2.** Re-measured with `scripts/bundle-min-cyrius.sh`:

  | cyrius | build | core smoke | ping | first failure |
  |---|---|---|---|---|
  | 6.6.20, 6.7.0, 6.7.1 | fail | — | — | `lib/patra.cyr:3765:24: expected identifier, got '-'` |
  | 6.7.2, 6.7.3, 6.7.9, 6.7.10, 6.7.13, 6.7.16, 6.7.17, 6.7.18 | ok | ok | ok | |
- **Consumers, built for real.** A copied core bundle with DEPS-PATTERN's list plus `sakshi` builds and
  runs on 6.7.2, 6.7.9 and 6.7.18. `[deps.bote]` consumers declaring only `string fmt alloc vec str
  syscalls io freelist`, resolved against a scratch clone tagged 3.4.11, build with 0 undefined and
  answer a ping: core 1,797,872 B, full 3,200,464 B.

### Added

- `tests/bote.tcyr` +9, 1,374 → 1,383 in all. The fallback had no test, which is why the regression
  above passed all 1,374 assertions. It now has coverage for a missing field and its name, a second
  required field, a present field with any value, a nested key, a key spelled inside a string, and a
  repeated key on both the fallback and the compiled path.
- **Mutation-proven.** Four mutants, each killed by an assertion:
  - back to bayan's `json_get` (fails exactly the repeated-key assertion);
  - required fields never enforced;
  - the check inverted;
  - only the first required field checked.

### Changed

- Binaries: `bote` 3,213,728 B (3.4.10: 2,857,944, +355,784 B or +12.4%). `string_data` went 58,390 →
  401,865, so 343,475 B of that is bayan's read-only Unicode data, which dead-code elimination cannot
  remove (libro 2.11.1 measured the same). aarch64 3,745,912 B (3,328,680); agnos 3,182,184 B
  (2,822,288) with 0 undefined. Capacity: `fn_table` 6956 / 131072, `identifiers` 220251 / 8388608,
  `var_table` 3260 / 1048576.
- Every test file is green on x86_64 and under `qemu-aarch64` 11.1.3. Every unit compiles for agnos
  with 0 undefined. CI's step scripts for logging smoke, transport smoke, fuzz and the capacity gate
  pass. `fmt`, `lint`, `vet` and `deny` are clean.
- Roadmap: `sandhi_server_last_failure` is queued as the next patch, and bayan in the core bundle is
  a new open decision. Both of bote's open upstream filings are closed.

### Performance

Interleaved A/B: 15 alternating rounds each, pinned to one core, load about 3.9, the 3.4.10 tree on
6.7.9 against this tree on 6.7.18. A wash on the pipeline: `codec_process_message` is +0.8% and 12 of
the 16 rows are within ±2%. The other four:

| Bench | 6.7.9 (median) | 6.7.18 (median) | Delta |
|---|---|---|---|
| `jsonx_get_str_flat` | 162 ns | 142 ns | −12.3% |
| `validate_compiled_simple` | 528 ns | 507 ns | −4.0% |
| `dispatch_tools_call_result` | 2,622 ns | 2,692 ns | +2.7% |
| `codec_parse_request` | 1,369 ns | 1,437 ns | +5.0% |

`codec_parse_request`'s ranges do not overlap, so that row is a real shift for these two binaries.
`src/codec.cyr` did not change and the cause was not chased. `jsonx_get_str_flat` is the row 3.3.17
measured moving on code alignment alone. bayan's costlier JSON parsing does not reach these rows:
bote's request path does not use bayan.

## [3.4.10] — 2026-10-15 · bote's own tools: real schemas, real tool results, and two defects they hid

The roadmap's 3.4.10 item, audit B9: bote's own tools (libro, fs, web) get real input schemas,
annotations and real tool results. Making them found two defects: `libro_retention` could empty
the audit chain on a missing argument, and every libro entry's timestamp went out as a heap
address.

### Security

- **`libro_retention` no longer empties the chain when `value` is missing.** `keep_count`,
  `keep_duration` and `keep_after` read `value` as 0 when it was absent. `{"policy":"keep_count"}`
  therefore kept zero entries and archived the whole audit chain. A missing or negative `value`
  for those three policies is now an `isError` result, and the chain is left alone (tested on a
  populated chain). The schema now says `value` is a non-negative integer. The 2026-10-15 consumer
  audit found daimon exposing this tool on an unauthenticated endpoint.
- **libro entries no longer leak heap addresses.** libro stores an entry's timestamp as RFC 3339
  text (a Str), and `_lt_emit_entry` printed it with `str_builder_add_int`. Every `libro_query` and
  `libro_export` entry carried the Str's address as `"timestamp":140185473909144`. It is now the
  string, `"timestamp":"2026-10-16T05:46:12Z"`. This is the struct-layout drift CLAUDE.md warns
  about: here a field whose type changed, not one that moved.

### Changed — bote's own tools, on the wire

| Tool | 3.4.9 | 3.4.10 |
|---|---|---|
| all five libro tools | bare JSON (`{"ok":true}`), which the dispatcher wrapped as text since 3.4.8; a failure was not flagged | a CallToolResult: the JSON as text, the same object as `structuredContent`, and `isError: true` when `ok` is false |
| `libro_query` / `libro_export` | `"timestamp"` a heap address | `"timestamp"` the RFC 3339 string |
| `libro_retention` without `value` for a keep_* policy | ran with 0 | `isError` |
| `fs_write` without `content` | wrote an empty file | `isError` (schema violation); pass `""` to write an empty file |
| libro, fs and web arguments of the wrong type or range | reached the handler | `isError` from the dispatcher's validation (since 3.4.8, now that the schemas say something) |

**Who this reaches:**

- **daimon** calls `libro_tool_*` directly and passes a result that already has `content`
  through, so its clients see the same text plus `structuredContent`. Its own wrapper also derived
  `isError` from `ok`, so the failure flag is unchanged. `libro_query` / `libro_export` timestamps
  become strings; they were addresses before.
- **No other consumer** calls these handlers. itihas only mirrors the registration pattern.

### Added

- **Input schemas** where there were none:
  - `libro_query`: nine typed filters, severity 0–5, `offset` and `limit` ≥ 0;
  - `libro_proof`: `index` required, ≥ 0;
  - `libro_retention`: `policy` required, an enum of the six policies; `value` ≥ 0, with what it
    means for each policy;
  - `fs_write`: `path` and `content` required;
  - `fs_read` / `fs_mkdir`: `path` required;
  - `web_fetch`: `url` required, the SSRF refusals described;
  - `web_search`: `query` required, `count` described as clamped to 1–10, which is what the handler
    does.

  Each property has a description, so a client no longer learns the arguments from the tool's
  description text.
- **Annotations** on all ten:
  - the four libro readers, `fs_read`, `web_fetch` and `web_search` are read-only;
  - the web tools are open-world;
  - `fs_write` is destructive and idempotent;
  - `fs_mkdir` is idempotent;
  - `libro_retention` is destructive, not idempotent (`keep_duration` depends on the clock) and not
    open-world. The `ann_destructive()` preset says open-world, so that hint is overridden.
- Tests:
  - `tests/bote_libro_tools.tcyr` +26, and its old assertions read fields through
    `structuredContent`:
    - each result's shape, and `isError` on each failure path;
    - the retention guard, with the chain proven untouched;
    - RFC 3339 timestamps from export and query;
    - schema enforcement, and the published properties and annotations.
  - `tests/bote_fs_tools.tcyr` +8 and `tests/bote_web_tools.tcyr` +7: required and typed arguments,
    and the annotations in `tools/list`.

  1,333 → 1,374 in all.
- **Mutation-proven.** Seven mutants, each killed:
  - the retention guard removed;
  - failures without `isError`;
  - the timestamp printed as a number;
  - `libro_proof`'s `index` not required;
  - retention back to the open-world preset;
  - `fs_write`'s `content` not required;
  - `web_fetch` not open-world.

  The last one first survived, because the assertion found the hint on `web_search`. It now counts
  the hint once per tool.

### Changed

- Roadmap: nothing is queued for 3.4.x. Its planned work is done. The next work is either an item
  from the consumer adoption track, taken when a named consumer is ready, or the 3.5.x
  threaded-dispatch minor.
- Binaries: `bote` 2,857,944 B (3.4.9: 2,851,512); aarch64 3,328,680 B; agnos 2,822,288 B with 0
  undefined. Capacity: `fn_table` 6368 / 131072. Every test file is also green under
  `qemu-aarch64`, and both `.deps` sidecars are unchanged.
- The minimum cyrius stays 6.7.2. This patch uses no newer construct, and
  `scripts/bundle-min-cyrius.sh 6.7.2` still builds, passes the core smoke test and answers a ping.

### Performance

- `tests/bote.bcyr` links none of the changed modules. The logged row (pinned, load 2.5) is within
  noise of 3.4.9's: `dispatch_tools_call` 2.782 µs against 2.743, `dispatch_tools_list` 1.698 µs
  against 1.617.

## [3.4.9] — 2026-10-15 · the bundles state their minimum cyrius (6.7.2, measured) and what a copied core bundle needs

The roadmap's 3.4.9 item, findings B6 and B7 of the 2026-10-15 consumer audit. The bundles now say
what toolchain they need, and what a repo that copies one must declare, in the file it copies.

### Added — the bundles state their minimum cyrius

- **Measured first.** `scripts/bundle-min-cyrius.sh` (new) copies the working tree, pins it to
  each installed toolchain, provisions it from scratch, and requires three things: `src/main.cyr`
  builds with no undefined function, the core-only smoke test passes, and the binary answers a
  ping.

  | cyrius | build | core smoke | ping | first failure |
  |---|---|---|---|---|
  | 6.6.20, 6.7.0, 6.7.1 | fail | — | — | `src/log.cyr:54:7: expected '=', got identifier 'BOTE_LOG_LINE_BYTES'` |
  | 6.7.2 – 6.7.9, 6.7.13 | ok | ok | ok | |

  So the 3.4.0 CHANGELOG's "≥ 6.7.2" holds, and nothing since has raised it. Until now it was
  written only there and in DEPS-PATTERN. The audit found four repos on cyrius 6.6.6 (hoosh, szal,
  thoth, t-ron), and only a re-vendor attempt would have told them.
- **`# Minimum cyrius: 6.7.2`** now opens `src/log.cyr`, the first module of both bundles, so it
  is among the first lines a repo sees in a copied `dist/*.cyr`. It sits under a note saying how
  it was measured and when to re-measure. A second comment sits directly above the `const` an
  older compiler stops at, and says what the error means. cyrius has no `#error` and no version
  symbol, so a bundle cannot refuse an old toolchain by name. That is recorded as an upstream
  request.
- **What a copied core bundle needs.** Measured on 6.7.2 and 6.7.9: a consumer that copies
  `dist/bote-core.cyr` and declares `string fmt alloc vec str syscalls io freelist` stops on six
  undefined `sakshi_*` functions, and the same list plus `sakshi` builds and answers a ping. A
  pinned consumer gets sakshi through bote's `deps.libro` (patra includes it); a copied file does
  not. The same note at the top of `log.cyr` says so, and DEPS-PATTERN gains the vendoring
  paragraph it lacked.
- **CI step "Bundles state their minimum cyrius":** both bundles carry the line, it is the same
  in both, and it is not newer than the pin. The measurement itself stays local, like the
  `qemu-aarch64` sweep, because it needs every toolchain installed. CLAUDE.md's version-check step
  says to re-run the script at a pin move or when bote starts using a newer language feature.

### Changed

- DEPS-PATTERN states the minimum up front, with how it was measured, and says a 6.6.x repo can
  take bote 3.3.17 at the newest. README and CLAUDE.md's distribution section say the same in a
  line. README's module count for `dist/bote.cyr` was 32 and is 33 (`conn.cyr`, 3.4.3).
- Roadmap: the next patch is 3.4.10 (bote's own tools). Two upstream requests are recorded to
  file: a toolchain requirement a bundle can carry, and a vendoring-friendly pin.
- Re-measured on the final tree: 6.7.1 now stops at `src/log.cyr:74`, the `const` directly under
  the comment that explains the error, and 6.7.2 passes.
- Binaries are unchanged in size: `bote` 2,851,512 B, aarch64 3,326,344 B, agnos 2,815,848 B with
  0 undefined. Every test file is also green under `qemu-aarch64`.

### Performance

- Comments and docs only; no compiled code changed. The logged row (pinned, load 1.0) is within
  noise of 3.4.8's: `dispatch_tools_call` 2.743 µs against 2.817, `dispatch_tools_list` 1.617 µs
  against 1.677.

## [3.4.8] — 2026-10-15 · schemas compiled at registration, every tool result a CallToolResult

The roadmap's 3.4.8 item, findings B2–B4 of the 2026-10-15 consumer audit. bote's schema validator
had never run in production, because nothing compiled a schema. Whatever a tool handler returned
went out as it was. And a handler that reported its own failure was audited as a success.

### Breaking — what a client and an embedder see

| Situation | 3.4.7 | 3.4.8 |
|---|---|---|
| `tools/call` with an argument of the wrong type, outside its bounds or its enum | reached the handler (only `required` was checked) | a result with `isError: true` naming the argument (SEP-1303) |
| a handler returns bare JSON (`{"ok":true}`, an array, a number) | sent as the `result`, which no client can read as a CallToolResult | a text block holding that JSON |
| a handler returns one content block (`{"type":"image",…}`) | sent bare | lifted into a one-element `content` array |
| a handler returns bytes that are not JSON | sent, breaking the reply | a failed call (`isError`); the bytes are not sent |
| a handler returns a result with `isError: true` | audited and published as a success | audited as a failure, on `bote/tool/failed` |
| `registry_register` / `dispatcher_register_tool` given a malformed definition | registered; `tools/list` then emitted garbage or crashed | refused with a BoteError and an ERROR log line; nothing registered |

**Who this reaches**, checked against each consumer's source:

- **t-ron**: its four tools pass `str_from("object")` as the schema type, so they are now refused
  at registration. They used to be listed with garbage bytes in `tools/list`, and every call to
  them failed validation on a garbage `required` key. Fix: pass cstrs.
- **phylax**: its five tools pass `Str` names. They are now refused. On 3.3.8–3.4.7 `tools/list`
  crashed (measured). Fix: cstrs, `jsonx_get_str`, CallToolResult returns.
- **szal**: its 73 literal property schemas are all valid, so registration accepts every tool.
  Types, enums and bounds are now enforced on calls where only `required` was before. A client
  sending a string for an integer gets an `isError` result naming the argument.
- **itihas**: its integer `year` arguments are now type-checked, and its bare-record results arrive
  as text blocks.
- **bote's own binary**: `libro_query` / `_verify` / `_export` / `_proof` / `_retention` returned
  bare JSON (`{"ok":true}`) and now arrive as text blocks. On the built 3.4.7 binary, `libro_verify`
  over stdio answered `"result":{"ok":true}`.
- **hoosh, stiva, daimon, thoth, agnosai**: unaffected. They return CallToolResults already, or do
  not register tools through bote.

Migration:

- **Call `registry_check_tool_def(def)`** to see why a definition would be refused. It returns the
  reason, or 0. Refusals are also logged at ERROR, with the reason.
- **Return a CallToolResult** (`content_text_response`, `content_text_error_response`, the
  `content_*` builders) if the text-block wrapping of a bare result is not what a client should
  see. Report a tool-level failure with `isError: true`, and the audit will record it as a failure.
- **A schema with a fractional bound** (`"minimum": 4.5`) is compared on its integer part, so a
  value slightly outside it can pass. Integral bounds are exact.

### Fixed

- **Schemas are compiled at registration** (`registry_register`, and so `dispatcher_register_tool`).
  Through 3.4.7 nothing in `src/` or in any consumer called `compiled_compile`. Every tool took
  `registry_validate_params`' fallback, which checks only that `required` keys are present, so
  `compiled_validate` and 3.4.5's SEP-1303 path never ran in production. A compiled schema the
  caller attached with `tool_def_with_compiled` is kept.
- **Registration checks the definition** (`registry_check_tool_def`, new and public):
  - the name follows the MCP tools spec's guidance: 1–128 characters from `A-Z a-z 0-9 _ - .`.
    Every consumer's names already do, and a `Str` passed as a name fails;
  - inputSchema exists and has type `"object"` (the spec: "MUST be a valid JSON Schema object (not
    `null`)");
  - each property schema is valid JSON that is an object or a boolean;
  - property names and `required` entries are non-empty.
- **Numbers are read as JSON Schema 2020-12 reads them** (`_sch_num_parse`). Compiling every schema
  made two old validator bugs reachable:
  - **Exponents were ignored:** a number was read up to its first non-digit, so `5e2` (500) failed a
    minimum of 100 as 5.
  - **`integer` rejected any `.`:** 2020-12, MCP's default dialect, counts `5.0` and `1e2` as
    integers.

  Bounds written with exponents (`"maximum": 1e3`) are read too. A value is truncated toward zero
  for its bounds. Truncation is monotonic, so a valid value can never fail a bound; only a
  fractional bound can let a value slightly outside it through.
- **Every tool result is a CallToolResult** (`_dsp_coerce_result`; table above).
  - Only the five MCP content types lift as a block. A domain object with a `type` member
    (`{"type":"person"}`) stays data, in a text block.
  - Invalid JSON is checked with `jsonx_valid`, so a handler can no longer break the reply that
    carries its result.
- **A result carrying `isError: true` is a failure for the audit and the event**
  (`TOPIC_TOOL_FAILED`, `success` 0), like a handler that returns nothing or invalid JSON.

### Added

- `registry_check_tool_def(tool_def)`: the reason a definition would be refused, or 0.
- Tests:
  - `tests/bote.tcyr` +40:
    - numbers per 2020-12: `3.0`, `4e0`, `5e1`, `5e2`, `1.5e2`, `1e999`, `-12.75` and malformed
      forms;
    - registration accepted, compiled, and a caller's compiled schema kept;
    - each refusal (space in a name, 129 characters, empty, no definition, no schema, type string,
      t-ron's and phylax's `Str` shapes, a non-JSON and a non-schema property, an empty required
      entry) and the ERROR line;
    - 128 characters and a boolean subschema accepted;
    - `dispatcher_register_tool` returning the refusal with no handler added;
    - `isError` and invalid-JSON results audited as failures.
  - `tests/conformance.tcyr` +8:
    - J16–J20, each result shape on the wire (bare JSON, one block, invalid JSON, a domain object
      with `type`, the handler's own `isError` result);
    - J21–J23, typed arguments on a schema compiled at registration (`5.0` and `5e2` accepted, a
      fraction for an integer refused, a value below its minimum refused).
- **Mutation-proven.** Eleven mutants, each killed by at least one suite:
  - no compile;
  - the name check, the type check or the subschema check skipped;
  - the refusal ignored by `dispatcher_register_tool`;
  - no coercion, any `type` lifted, or invalid JSON passed through;
  - `isError` audited as success;
  - the integer rule reverted, or exponents ignored.

### Changed

- `registry_register` returns a BoteError on refusal (it always returned 0). Callers that ignore the
  return keep compiling, and a refused tool is simply absent.
- The registry header no longer says CompiledSchema integration is deferred.
- Roadmap: the next patch is 3.4.9. bote's own tools (libro's bare-JSON results, `ok:false` as
  `isError`, and the empty schemas, audit B9) are recorded as a follow-up.
- Binaries: `bote` 2,851,512 B (3.4.7: 2,842,848); aarch64 3,326,344 B; agnos 2,815,848 B with 0
  undefined. Capacity: `fn_table` 6365 / 131072. Every test file is also green under
  `qemu-aarch64`, and both `.deps` sidecars are unchanged.

### Performance

- **Interleaved A/B against 3.4.7**, both trees pinned to one core, three rounds each, on a quiet
  host (load 1.9–2.4):

  | Bench | 3.4.7 avgs (ns) | 3.4.8 avgs (ns) | Δ median |
  |---|---|---|---|
  | `dispatch_tools_call` (handler returns bare JSON) | 2066, 2068, 2011 | 2780, 2874, 2842 | +0.78 µs (+37.6%) |
  | `dispatch_tools_call_result` (new; returns a CallToolResult) | — | 2596, 2670, 2647 | — |
  | `codec_process_message` | 4567, 4578, 4483 | 5487, 5656, 5608 | +22.8% |
  | `validate_compiled_simple` | 498, 519, 484 | 521, 528, 521 | +4.6% |
  | `dispatch_tools_list` | 1709, 1710, 1700 | 1647, 1708, 1690 | −1.1% |

  - **The cost is checking the result.** With coercion turned off, `dispatch_tools_call` went back
    to 2.10 µs. With compile-at-registration turned off, it stayed at 3.10 µs, so compiling costs
    nothing measurable here (the bench schema has no properties).
  - **A CallToolResult**, what real handlers return, costs about 0.58 µs more than 3.4.7's
    unchecked passthrough: a syntax pass over the result, and one walk over its top-level members.
  - **A bare result** also gets wrapped in a text block: +0.78 µs on the bench's 12-byte one.
  - **A first version did worse:** two key lookups plus an allocation to read `type`, +1.0 µs.
    The single, allocation-free walk (`_dsp_result_shape`) replaced it.
  - **Numbers were briefly parsed twice:** `validate_compiled_simple` was +14% until the number
    branch parsed each value once.
  - **`jsonx_get_str_flat` reads +12%** (145 → 160 ns) on code this patch did not touch. That is a
    layout shift.
- The logged row (`benches/history.log`, pinned, load 2.5) agrees: `dispatch_tools_call` 2.817 µs,
  `dispatch_tools_call_result` 2.625 µs, `dispatch_tools_list` 1.677 µs.
- `dispatch_tools_call_result` is the sixteenth benchmark. It runs on its own dispatcher, so
  `dispatch_tools_list` still lists one tool and stays comparable with earlier rows (a first
  attempt that shared the registry read +86% on `tools/list`).

## [3.4.7] — 2026-10-15 · SSRF classifier gaps: shared, reserved and IETF space refused, IPv6 literals parsed to bytes

The roadmap's 3.4.7 item: the SSRF classifier gaps found while wiring the 3.4.4 resolved-address
guard. A few URLs that reached the network on 3.4.6 are refused now, each with the range named.

### Security — more internal address space is refused, and IPv6 literals are parsed

- **Reproduced on 3.4.6 first.** `web_fetch` over stdio on the built binaries:

  | URL | 3.4.6 | 3.4.7 |
  |---|---|---|
  | `http://[0:0:0:0:0:0:0:1]:18555/x` | passed the guard (transport error) | refused (loopback) |
  | `http://[fe9f::1]/x` | passed the guard (transport error) | refused (link_local) |
  | `http://[zzz]/x` | passed the guard (transport error) | refused (parse) |
  | `http://100.100.100.100/x` | passed the guard; the connect hung past 40 s | refused (shared) |
  | `http://255.255.255.255/x` | passed the guard; no reply within 40 s | refused (reserved) |

  The IPv6 rows failed at the transport only because sandhi's stock client cannot dial a literal.
  The guard itself had let them through.
- **IPv4** (`_ssrf_classify_v4`, which now reads three octets):
  - **100.64.0.0/10**, RFC 6598 shared address space (carrier-grade NAT, and the tailnet
    addresses Tailscale hands out), is refused with the new code `SSRF_SHARED` ("shared").
  - **192.0.0.0/24** (IETF protocol assignments) and **240.0.0.0/4** (reserved, which contains the
    255.255.255.255 broadcast address) are refused with the new code `SSRF_RESERVED`
    ("reserved").
  - **Two ranges the roadmap considered stay open, on purpose.** 198.18.0.0/15 (benchmarking):
    fake-IP DNS proxies (Clash, mihomo, sing-box) answer every name with an address there, so
    refusing it would refuse every fetch on such a host, and nothing an attacker wants lives in
    it. The documentation ranges (192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24): nothing routes
    them. Both are pinned by tests that say why.
- **IPv6 literals are parsed, not pattern-matched.**
  - `_ssrf_parse_ipv6` turns a URL's literal into its 16 bytes per RFC 4291 §2.2: one `::`, groups
    of 1–4 hex digits, an optional dotted-quad tail under the canonical-octet rule, and a zone id
    set aside. `ssrf_classify_ipv6_bytes` then judges it, the same classifier web_fetch's resolve
    hook uses for AAAA answers, so no spelling of an address is judged differently from the
    address.
  - The text matcher it replaces missed `0:0:0:0:0:0:0:1`, `0::1` and most of `fe80::/10`, and
    returned OK for anything it did not recognise. A literal that does not parse is now
    `SSRF_PARSE`, which refuses it.
  - A bracketed host now always goes to the IPv6 classifier. It used to be chosen by "has a
    colon", which sent `[zzz]` and `[]` down the hostname path, where they passed.
- **IPv6 ranges added to the byte classifier:**
  - `fec0::/10`: deprecated site-local, still internal wherever a stack routes it.
  - `64:ff9b:1::/48`: local-use NAT64, as the well-known prefix already was.
  - `2002::/16` (6to4): the IPv4 address in bytes 2–5 is judged as one, so `2002:7f00:1::` is
    loopback and 6to4 of a public address passes.
  - IPv4-mapped and NAT64 literals stay refused outright, public tail or not.

### Added

- `SSRF_SHARED` (9) and `SSRF_RESERVED` (10), named by `ssrf_reason`.
- `fuzz/ssrf_ipv6.fcyr`, the sixth fuzz harness. It runs host.cyr's IPv6 parser against the cyrius
  TLS stack's own (`lib/tls_hostid.cyr`, `_tn_parse_ipv6`) on eight seeds, every byte
  substitution at every offset, every prefix and every deletion: both must refuse, or both must
  accept with the same bytes. It also checks that `ssrf_check` returns a defined code for every
  bracketed URL built from those inputs.
- Tests: `tests/bote_host.tcyr` +45.
  - A test per new range, at both edges where there is one: 100.63.255.255 and 100.128.0.1 pass,
    100.64.0.1 and 100.127.255.255 are refused, and 239.255.255.255 is still multicast.
  - The two ranges left open on purpose.
  - Packed-address parity, including a 192.0.1.1 case proving the third octet is read.
  - IPv6 spellings, zone ids, 6to4, site-local, local-use NAT64 and eight malformed literals.
  - The two parsers agreeing on 37 inputs.
- **Mutation-checked.** Ten mutants; nine are killed:
  - each new IPv4 range removed, or widened;
  - the packed classifier dropping the third octet;
  - bracket routing removed;
  - five-digit groups allowed (the fuzz harness catches this one too);
  - the 6to4 rule, the fec0 rule, and the empty-zone check each removed.

  The tenth, dropping the explicit second-`::` check, is equivalent: the side holding the earlier
  `::` always contains an empty group and fails anyway. The check stays for clarity.

### Changed

- `_ssrf_classify_v4_ab(a, b)` is now `_ssrf_classify_v4(a, b, c)`, since 192.0.0.0/24 needs the
  third octet. The dotted-quad parse is split out as `_ssrf_parse_dotted`, which the IPv4 classifier
  and the IPv6 parser's dotted tail share. All of these are private.
- Roadmap: the next patch is 3.4.8, schemas compiled at registration and handler results checked.
- `docs/spec-compliance.md`'s SSRF rows describe the byte classifier and the ranges, and SECURITY.md
  drops the gaps this patch closes.
- Binaries: `bote` 2,842,848 B (3.4.6: 2,842,816); aarch64 3,325,872 B; agnos 2,807,192 B with 0
  undefined. Capacity: `fn_table` 6356 / 131072, +4 for the parser's functions. Every test file is
  also green under `qemu-aarch64`.

### Performance

- `tests/bote.bcyr` links neither `host.cyr` nor `web_tools.cyr`, and none of its modules changed.
  The logged row (pinned, load 2.7) sits 3–6% above the 3.4.6 row on every bench alike:
  `dispatch_tools_list` 1.708 µs against 1.613, `codec_serialize_response` 458 ns against 439.
  A uniform shift across code that did not change is host state, not this patch.

## [3.4.6] — 2026-10-15 · every string bote writes into JSON is escaped: control bytes no longer break replies

The roadmap's 3.4.6 item, the first finding of the 2026-10-15 consumer audit: bote's JSON escaper
left most control bytes raw, so a tool result containing one was not JSON.

### Fixed — every string bote writes into JSON is escaped (RFC 8259 §7)

- **Reproduced on 3.4.5 first.** `fs_read` of a file holding two ANSI colour sequences and a BEL,
  over stdio on the built binary: Python's `json.loads` rejects the reply ("Invalid control
  character"). Any client reading a coloured log, a terminal transcript or a file with a stray
  0x01 got a broken stream.
- **The escaper.** `_json_emit_escaped` escaped `"`, `\`, LF, CR and TAB, and copied every other
  byte below 0x20 through raw. It is now `bote_json_escape` (`src/jsonx.cyr`). Control bytes
  become `\b \f \n \r \t` or `\u00XX`, the same table as the stdlib's
  `str_builder_add_json_str`, which bote cannot call directly because it takes a Str and writes the
  quotes itself. Bytes from 0x20 up pass through, so UTF-8 is untouched. `_json_emit_escaped`
  stays as an alias: every bote module calls it, and so do six call sites in hoosh.
- **`serverInfo`** (`dispatcher_set_server_info`) went into the `initialize` reply with no escaping
  at all, so a quote in an embedder's server name broke the reply. Both fields are escaped now.
- **A sweep of every other place `src/` writes a string into JSON** found nothing else unescaped:
  each is a literal, a value from a fixed set (`"2.0"`, a negotiated protocol version, a content
  kind), or already went through the escaper.

### Added

- `bote_json_escape(sb, cstr)`: the escaper, public, for a consumer's own JSON. The audit found it
  re-implemented in itihas, varna, mneme, jalwa and daimon, two of them wrongly. `content.cyr`'s
  builders already go through it.
- Tests:
  - `tests/bote.tcyr` +13: each escape form, plain runs on either side of an escape, UTF-8 and DEL
    passing through, and every byte 1..255 in one string, which must come back as valid JSON that
    bayan decodes to the same 255 bytes.
  - `tests/conformance.tcyr` +2: J15, a tool result holding ESC, BEL, 0x1f, TAB and a quote; F12,
    an embedder's `serverInfo` with a quote and a backslash. Both also pass through the suite's
    standing valid-JSON check.
  - `tests/bote_fs_tools.tcyr` +3: `fs_read` of `tests/fixtures/control-bytes.txt` (new) is valid
    JSON, and the bytes come back as `\u001b` / `\u0007` / `\u001f` / `\t`.
  - `fuzz/jsonx_valid.fcyr` property 50, with `jsonx_valid` as the oracle: every byte 1..255 and
    every byte-substituted seed must come out of the escaper, quoted, as one valid JSON string.
  - **Mutation-proven.** Four mutants, each killed:
    - control bytes raw again: unit tests, the conformance standing check, the `fs_read` test and
      the fuzz property all fail;
    - `serverInfo` raw again: F12 fails;
    - the trailing run dropped, or the run copy one byte short: unit and conformance tests fail.

    The fuzz property misses the last two by design. It checks validity, not fidelity, and the
    unit round-trip covers fidelity.

- `docs/audit/2026-10-15-consumers.md`: a read-only audit of every AGNOS repo that consumes bote or
  runs its own MCP server or client (agnosai, daimon, hoosh, itihas, phylax, stiva, szal, thoth,
  t-ron, varna; jalwa, mneme, sandhi, agnostik). Every high-severity claim was re-read at its
  `file:line`.
  - No consumer depends on any behaviour 3.4.5 changed. Every bump is gated by the toolchain
    instead: the 3.4.x core bundle does not compile on the cyrius 6.6.x pins most consumers carry.
  - It found two bote defects, both now on the roadmap. The JSON escaper passes most control bytes
    raw (measured: the 3.4.5 binary's `fs_read` of an ANSI-coloured file returns invalid JSON). No
    input schema is ever compiled outside the tests, so the type / enum / bounds validator never
    runs in production.
  - It also found security defects in consumers, independent of bote: path traversal and an open
    bind in mneme, heap overflows in t-ron, an SSRF bypass and curl file exfiltration in szal, and
    unauthenticated MCP endpoints in daimon, agnosai and hoosh.

### Changed

- `_json_emit_escaped` moves from `dispatch.cyr` to `jsonx.cyr`, the JSON-helpers module that
  precedes every caller in both profiles. Neither profile's module list or sidecar changes.
- `jsonx.cyr`'s header no longer calls `jsonx_get_str`'s value "decoded". It returns the
  string's raw JSON text, as the function's own comment says.
- Roadmap: the next patch is 3.4.7, the SSRF classifier gaps.
- Binaries: `bote` 2,842,816 B (unchanged from 3.4.5), aarch64 3,325,840 B, agnos 2,807,160 B with
  0 undefined. Capacity: `fn_table` 6352 / 131072, +3 for the escaper's three functions. Every test
  file is also green under `qemu-aarch64`.
- Roadmap re-planned from the audit:
  - **3.4.6:** the escaper fix (control bytes as `\u00XX`, `serverInfo` escaped, a public name).
  - **3.4.7:** the SSRF classifier gaps, moved back from 3.4.6.
  - **3.4.8:** schemas compiled at registration and handler results checked.
  - **3.4.9:** the bundles state the toolchain they need.
  - A consumer adoption track holds envelope-free entry points, a pre-dispatch guard hook, a
    handler context pointer, decoding getters and namespaced public names.
  - Core-reachable HTTP guards and the SSRF classifier are an open profile decision.
  - A vendoring-friendly pin is an upstream filing still to be made.
  - `docs/spec-compliance.md` points the classifier gaps at 3.4.7.

### Performance

- **Interleaved A/B against 3.4.5,** both trees pinned to one core, three rounds each, on a quiet
  host (load falling from 3.8 to 3.4):

  | Bench | 3.4.5 avgs (ns) | 3.4.6 avgs (ns) | Δ median |
  |---|---|---|---|
  | `dispatch_tools_list` | 2066, 2070, 2043 | 1747, 1742, 1711 | **−15.7%** |
  | `dispatch_initialize` | 1436, 1443, 1430 | 1505, 1514, 1483 | +4.8% (+69 ns) |
  | `dispatch_tools_call` | 2084, 2079, 2053 | 2099, 2100, 2060 | +1.0% |
  | `codec_process_message` | 4634, 4617, 4527 | 4813, 4588, 4560 | −0.6% |
  | `codec_serialize_response` | 461, 461, 457 | 465, 457, 456 | −0.9% |

  - **`tools/list` is faster** because the escaper now copies a run of plain bytes in one go. The
    old one paid a `strlen`, a grow check and a `memcpy` for every byte, through a two-byte stack
    buffer.
  - **`initialize` is slower by about 70 ns**, every round, because `serverInfo` goes through the
    escaper now: two short strings that used to be copied raw. That is the price of the B5 fix.
  - **Everything else is within ±3%.**
- The logged row (`benches/history.log`, pinned, load 2.9) has `dispatch_tools_list` at 1.613 µs.
  The 3.4.5 row has 2.084 µs, but it was logged under load 6.2.
- An earlier attempt at the A/B under load 25 (other sessions compiling on every core) was
  discarded: untouched rows moved by up to 68%.

## [3.4.5] — 2026-10-15 · the conformance suite, by spec clause, and the nine violations it found

The roadmap's 3.4.5 item: port the conformance suite. The Rust archive's 44 protocol scenarios
are now `tests/conformance.tcyr`, organised by spec clause and driven through
`codec_process_message`, with the JSON-RPC 2.0 and MCP 2025-11-25 requirements the Rust suite never
had. Run against the 3.4.4 sources, it fails 29 of its 117 assertions. Each failure is a spec
violation, and each is fixed here.

### Breaking — wire behaviour changed to match the spec

Each of these changes what a client sees on the wire, so each gets a migration note. They ship in
a patch because each was a spec violation on the released binary, the same footing as 3.3.13's
`ping` fix. `dispatcher_dispatch`'s signature and the 2.0 handler ABI are unchanged.

| Request | 3.4.4 | 3.4.5 | Spec |
|---|---|---|---|
| `tools/call` naming an unknown tool | error `-32601` | error `-32602` | MCP tools spec, "Error Handling": the example is `{"code":-32602,"message":"Unknown tool: …"}` (also in 2025-06-18) |
| `tools/call` whose arguments fail the tool's `inputSchema` | error `-32602` | **result** `{"content":[{"type":"text","text":"schema violation for tool '…': …"}],"isError":true}` | MCP 2025-11-25, SEP-1303: input validation errors are tool execution errors, "to enable model self-correction" |
| `tools/call` whose handler returns nothing | `"result":null` | result with `isError:true`, text `tool execution failed: <name> - the handler returned no result` | a CallToolResult must carry `content`; `null` is not one |
| `resources/read` of a URI nothing serves | error `-32602`, "invalid params for tool '<uri>': resource not found" | error `-32002`, "resource not found: <uri>", `"data":{"uri":"<uri>"}` | MCP resources spec, "Error Handling" |
| `prompts/get` missing a required argument | the handler was called | error `-32602` | MCP prompts spec: missing required arguments are `-32602` |
| Invalid JSON (`not valid json`, a truncated object, a trailing comma, `"id":01`) | `-32600`, or dispatched, or silently dropped | error `-32700`, id null | JSON-RPC 2.0 §5.1 |
| An id that is an object, array or boolean | echoed back | error `-32600`, id null | JSON-RPC 2.0 §4: an id is a string, a number or null |
| `params` that is a string or a number | ignored | error `-32600` with the request's id | JSON-RPC 2.0 §4.2: params is structured, if present |
| A `cursor` on `tools/list`, `prompts/list`, `resources/list` or `resources/templates/list` | ignored, first page sent again | error `-32602` (a null cursor is still read as absent) | MCP pagination: "Invalid cursors SHOULD result in an error with code -32602". bote issues no cursors, so every cursor is invalid |

Migration:

- **A client matching `-32601` for an unknown tool** should match `-32602`. `-32601` now means only
  what JSON-RPC says it means: the *method* does not exist.
- **A client that treated a `-32602` from `tools/call` as "bad arguments"** now gets a result with
  `isError: true`, whose text names the offending argument. A missing or empty tool name,
  `arguments` that are not an object, and an unknown tool `version` are still `-32602`: those break
  the CallToolRequest itself.
- **An embedder that re-wraps `dispatcher_dispatch` results** should pass a result that already has
  a top-level `content` array through unchanged, as bote's own bridge does (`wrap_tool_result`).
  Otherwise a validation failure is wrapped twice.
- **An embedder that read `bote_err_rpc_code(BOTE_ERR_TOOL_NOT_FOUND)`** gets `-32602`.
- **A prompt that declares an argument `required` but tolerated its absence** must either receive it
  or stop declaring it required.
- **A client that sends a `cursor` bote never gave it** should stop: bote returns every item in one
  page and never sets `nextCursor`, so a conforming client never has a cursor to send.

### Fixed — the nine violations, in the code

- **`jsonx_valid` (new, `src/jsonx.cyr`)** checks RFC 8259 syntax in one pass, allocating nothing,
  with nesting capped at `JSONX_MAX_DEPTH`, the cap the extractors already walk under. The
  extractors never checked syntax: they find a key by scanning. `codec_process_message` now
  validates the whole message first. 3.4.4 had answered `not valid json` with `-32600`. The
  spec's own truncated-object example (`{"jsonrpc":"2.0","method":"foo","params":"bar","baz]`) has
  no id, so 3.4.4 dispatched it as a notification and answered nothing. `{"id":01,…}` went back as
  `"id":01`, which is not JSON. The spec's invalid-JSON batch example got a two-element array where
  one `-32700` belongs.
- **Request admission is one function.** `_cdc_admit` (`src/codec.cyr`) checks the jsonrpc
  version, the id type and the params type. `bridge.cyr`'s single-message path calls it, and the
  bridge runs `jsonx_valid` too. The bridge used to carry its own copy of the version check, and
  that copy would have missed every check added here. An explicit `"params":null` is read as absent: clients send
  it, and it carries nothing a method could misread.
- **`tools/call`** (`src/dispatch.cyr`):
  - an unknown tool, or a listed tool with no handler, is `-32602`;
  - non-object `arguments` is `-32602`, checked before validation;
  - a schema violation is a result built by `_dsp_tool_error_result`;
  - a handler that returns 0 is a failed call, audited and published as `TOPIC_TOOL_FAILED` where
    3.4.4 recorded a success.
- **`resources/read`** answers an unserved URI with `-32002` (`BOTE_RPC_RESOURCE_NOT_FOUND`) and
  the URI in `error.data`, through the new `resp_error_data` in `src/protocol.cyr`.
- **The list methods** (`tools/list`, `prompts/list`, `resources/list`,
  `resources/templates/list`) answer a non-null `cursor` with `-32602`, through
  `_dsp_cursor_given` / `_dsp_cursor_refusal`. 3.4.1 had documented ignoring a cursor as the honest
  reply, but the pagination spec says otherwise, and a first page sent again reads to a client
  joining pages as duplicates.
- **`prompts/get`** refuses non-object `arguments` and the first missing required argument with
  `-32602`, before the handler runs. An unknown prompt's message is now "prompt not found: <name>".
  It used to say "invalid params for tool '<name>'".

### Added

- **`tests/conformance.tcyr`**, 117 assertions in fifteen sections:
  - JSON-RPC 2.0 §4 request, §5 response, §4.1 notifications, §5.1 error codes and §6 batch,
    including the spec's own examples (`[1,2,3]`, the truncated object, the invalid batch);
  - MCP initialize and version negotiation for all four revisions;
  - ping and cancellation;
  - capability gating and pagination cursors;
  - tools/list, tools/call, prompts, resources and completion;
  - the registry's compiled-schema cleanup, and the whole initialize → initialized → list → call
    flow.

  Every response is also checked on its own: it must be valid JSON and a JSON-RPC 2.0 Response
  object (an id that is a string, number or null; exactly one of result and error; an error with an
  integer code and a string message). One assertion requires the nonconforming count to be zero;
  against 3.4.4 it was 4. The Rust suite's 44 scenarios are all here except the bridge's error
  envelope, which stays in `tests/bote.tcyr` because the bridge links the HTTP stack. Where the
  Rust suite and the spec disagree (unknown tool, schema violation), the scenario says which it
  follows: the spec.
- **`fuzz/jsonx_valid.fcyr`**, the fifth fuzz harness:
  - a valid and an invalid corpus;
  - depth at and past the cap;
  - every proper prefix of an object or array text is refused;
  - every single-byte substitution at every offset of three seeds returns 0 or 1, and anything
    accepted is walked by the extractors.
- `fuzz/codec_process.fcyr` now requires every reply to pass `jsonx_valid`. It used to check only
  the first byte, "we can't validate JSON here without a parser", which is how `"id":01` went
  unnoticed. Ten corpus entries cover the new paths.
- `content_text_error_response(text)`, the failed-execution twin of `content_text_response`.
- `resp_error_data(id, code, message, data)`.
- The per-request DEBUG line says `status=tool_error` for a `tools/call` result carrying
  `isError: true`. Validation failures are results now, so they would otherwise log as `ok`.
- Tests: conformance +117; `tests/bote.tcyr` +10 (the bridge's new parse, id and pass-through
  checks; a handler returning nothing, audited and published as a failure; the raw-LF log case
  split in two, since the codec now refuses that byte before the logger sees it; the
  templates-list cursor assertion now expects `-32602`); `tests/bote_content.tcyr` +1. 1,094 →
  1,222 in all, every file also green under `qemu-aarch64`.
- **Mutation-proven.** Fifteen mutants, each killed:
  - skipping the parse check;
  - letting control bytes, trailing bytes, trailing commas or bad escapes through the validator;
  - an off-by-one depth cap;
  - unchecked id or params types;
  - schema violations back to `-32602`;
  - a handler's 0 back to `null`, and that empty result audited as a success;
  - dropping the prompt check;
  - `-32002` back to `-32602`;
  - a cursor check that never fires, and one that fires on a null cursor.

  Three (control bytes, depth, escapes) are caught only by the fuzz harness, which CI runs.

### Changed

- `docs/spec-compliance.md` is re-audited against the suite. It gains rows for the parse check,
  request admission, pagination and the tools/call error split, and its coverage table moves from
  3.3.12's counts to today's. Its DNS-resolution row still said "⏳ roadmap 3.6.x" although 3.4.4
  shipped the guard; it now says what shipped. CLAUDE.md, README, SECURITY.md and
  `docs/architecture/overview.md` carry the new counts and the corrected error-code tables. The
  old tables said `-32601` meant "tool not found" and `-32602` "schema violation".
- CI's "Run tests" step runs `tests/conformance.tcyr`. The "Fuzz" step picks up
  `fuzz/jsonx_valid.fcyr` through its glob, and so do the aarch64 and agnos compile gates.
- The roadmap's next patch is 3.4.6, the SSRF classifier gaps. Nothing else is queued for 3.4.x.
- Binaries: `bote` 2,842,816 B (3.4.4: 2,830,176); aarch64 3,325,840 B; agnos 2,807,160 B with 0
  undefined. Capacity: `fn_table` 6349 / 131072 (4%).

### Performance

- **The syntax pass is the one new cost on the request path**, measured in an interleaved A/B
  against a 3.4.4 build. Both trees were pinned to one core (`taskset`), three rounds each,
  because another session was compiling on the host (load 7–14):

  | Bench | 3.4.4 avgs (ns) | 3.4.5 avgs (ns) | Δ median |
  |---|---|---|---|
  | `codec_process_message` | 4541, 4798, 4614 | 5326, 5332, 5360 | +0.72 µs (+15.6%) |
  | `jsonx_valid_request` (new) | — | 665, 665, 675 | — |
  | `dispatch_tools_call` | 2253, 2277, 2339 | 2421, 2442, 2489 | +7.2% |
  | `codec_parse_request` | 1685, 1685, 1729 | 1640, 1659, 1692 | −1.5% |
  | `validate_compiled_nested` | 2652, 2679, 2593 | 2609, 2611, 2567 | −1.6% |

  The `codec_process_message` delta is the new pass (`jsonx_valid_request`, a new benchmark, 15
  now, on the same 106-byte `tools/call` line) plus the admission checks. The `dispatch_tools_call`
  delta is mostly drift. In a direct with/without A/B of its only new work, the `arguments`
  object check, the difference was at most about 50 ns, and the untouched
  `dispatch_tools_list` moved as much between runs. Benches sharing no changed code stay within
  ±6%.
- The logged row (`benches/history.log`, pinned, load 6.2) has `codec_process_message` 4.633 µs
  and `jsonx_valid_request` 577 ns. Its untouched rows sit about 10% above the 3.4.4 row
  (`dispatch_initialize` 1.459 against 1.323 µs), which is the host, not bote.
- Letting plain string bytes through `_jx_valid_string` on one compare gave no win
  distinguishable from noise, and was not kept.

## [3.4.4] — 2026-10-15 · DNS-aware SSRF: web_fetch classifies the address a name resolves to

The roadmap's 3.4.4 item: DNS-aware SSRF for `web_fetch`. The guard 3.4.0 applied judged the URL's
host *string*, so a hostname that resolved inside the box sailed through it. Since cyrius 6.5.37
sandhi has carried the seam bote filed at 3.3.7; this release uses it.

### Security — a hostname resolving to an internal address is refused

- **Reproduced first, through real DNS.** A local HTTP server on `127.0.0.1:18555` served a file,
  and `web_fetch` was called over stdio:

  | URL | 3.4.3 | 3.4.4 |
  |---|---|---|
  | `http://127.0.0.1:18555/secret.txt` | refused (SSRF guard: loopback) | refused (SSRF guard: loopback) |
  | `http://127.0.0.1.nip.io:18555/secret.txt` | **returned the file's contents** | refused (SSRF guard: loopback, resolved address) |

  `127.0.0.1.nip.io` is a public name whose A record is `127.0.0.1`. Any attacker-controlled zone
  does the same, and it reaches the cloud metadata address just as easily.
- **The address is classified where sandhi connects.** `web_fetch` installs a resolve hook
  (`sandhi_client_set_resolver`, sandhi 1.9.14) for the duration of its request.
  - sandhi asks that hook for the address it will connect to, so the address that gets classified
    is the one that gets dialled. There is no second lookup to rebind between.
  - IPv4 answers are judged by `ssrf_classify_ipv4_addr`, IPv6 answers by
    `ssrf_classify_ipv6_bytes`.
  - Once a name is refused on IPv4 it is refused on IPv6 too, so one bad answer cannot fall through
    to the AAAA record.
  - The hook also fires for IP literals, and would fire on every redirect hop. `web_fetch` does not
    follow redirects (sandhi's default), so there is no redirect bypass today either.
  - A refusal is an `isError` result naming the reason, plus a WARN line with the host and the
    address it resolved to.

  The URL pre-check (`ssrf_check`) still runs first and fails a literal or a blocklisted name
  before any DNS query.
- **Scoped to `web_fetch`, and composes with an embedder's hook.**
  - The hook is installed around the request, and whatever was installed before is restored after
    (or cleared, if nothing was).
  - `web_search` keeps stock resolution. Its endpoint is the operator's `BOTE_SEARXNG_URL`, often on
    localhost, and a live check against a SearXNG stand-in on `127.0.0.1` still answered.
  - If an embedder had installed its own hook, the guard resolves through it and classifies what it
    returns.
  - The guard never chains to itself. A mutant that skipped the restore made the next fetch chain
    to its own hook and die with SIGSEGV, so one line now prevents that.
  - The embedder's hook is read from sandhi's private `_sandhi_client_resolver_fn` / `_ctx`; sandhi
    exposes only an installed 0/1. A rename there fails bote's build, not the guard.
  - The hook is process-wide while installed. That is right for today's single-threaded dispatch;
    the 3.5.x worker threads will need it per thread, which the code says.
- **One IPv4 decision for literals and answers.** The two-octet decision is extracted from
  `_ssrf_classify_ipv4` into `_ssrf_classify_v4_ab`, which the literal and resolved paths share. A
  test checks that `ssrf_check("http://a.b.c.d/")` and `ssrf_classify_ipv4_addr` agree.
- **IPv6 is classified on bytes.** `ssrf_classify_ipv6_bytes` covers the string classifier's
  categories (`::`, `::1`, IPv4-mapped and -compatible, NAT64, fe80::/10, fc00::/7, ff00::/8)
  decided on the 16 address bytes. It therefore covers the whole fe80::/10, including `febf::1`,
  which the text matcher misses. An IPv6 *literal* never resolves through the hook: sandhi's stock
  resolver cannot dial one either, and the URL pre-check has already classified it.
- New public helpers in `src/host.cyr`: `ssrf_classify_ipv4_addr`, `ssrf_classify_ipv6_bytes`,
  `ssrf_ipv4_addr_str`, `ssrf_ipv6_bytes_str`.
- Tests (+49 assertions):
  - `tests/bote_host.tcyr` +30: the packed IPv4 classifier, literal-versus-resolved parity, the
    byte IPv6 classifier, and both formatters.
  - `tests/bote_web_tools.tcyr` +19, through a fake embedder resolver, so nothing leaves the box:
    - a name resolving to 127.0.0.1, one resolving to 169.254.169.254, and one whose only answer is
      fd00::1 are each refused, with the host and address in the WARN line;
    - public IPv4 and IPv6 answers pass, IPv6 is refused after an IPv4 refusal, and a name with no
      answer is not a refusal;
    - the embedder's hook is restored, no hook is left behind when none was installed, and a stale
      guard hook is not chained to.
  - Mutations, each run under `--timeout 60`:
    - with IPv4 classification removed, the refusal assertions fail, and the run then hangs trying
      to connect to 169.254.169.254;
    - with the restore removed, the restore assertion fails;
    - with IPv6 asked after an IPv4 refusal, that assertion fails.
  - Both units pass under `qemu-aarch64`.

### Changed

- The roadmap's next patch is 3.4.5, the conformance suite. The classifier gaps found along the way
  are recorded as their own small item. The IPv4 side has no CGNAT (100.64.0.0/10), broadcast or
  240.0.0.0/4 rule, although the 3.3.7 sandhi filing described CGNAT as covered. The IPv6 string
  matcher misses spellings such as `0:0:0:0:0:0:0:1`.
- SECURITY.md's outbound-request paragraph says what the guard now covers and what it still does
  not. CLAUDE.md, README and `docs/architecture/overview.md` carry the new counts.
- Binaries: `bote` 2,825,968 → 2,830,176 B; aarch64 3,325,488 B; agnos 2,794,512 B with 0 undefined.
- Benchmarks: `tests/bote.bcyr` links neither `host.cyr` nor `web_tools.cyr`, and none of its
  modules changed. The logged 3.4.4 row matches 3.4.3 (`dispatch_tools_call` 1.786 µs against
  1.802 µs, load 1.75).

## [3.4.3] — 2026-10-15 · a client that stops reading no longer wedges the server: send deadlines on every transport

The roadmap's 3.4.3 pair: the Unix transport's accept-loop deadline and send-side timeouts.
Both turned out to be the same defect. Every bote transport serves one connection at a time,
and nothing bounded a write, so a client that stopped reading could stop the server for
everyone.

### Fixed — a client that stops reading no longer wedges the server

- **Reproduced on 3.4.2 first.** A probe script runs one bad client and then a healthy client
  that sends one `ping` and waits up to 4 s:

  | Bad client | 3.4.2 | 3.4.3, `BOTE_SEND_TIMEOUT_MS=500 BOTE_UNIX_IDLE_MS=500` |
  |---|---|---|
  | Unix: floods `tools/list`, never reads | healthy client unanswered (wedged) | answered |
  | WebSocket: floods frames, never reads | healthy client unanswered (wedged) | answered |
  | Unix: connects and sends nothing | healthy client unanswered | answered in 0.20 s |

  The two flood cases were permanent: the server sat in write(2) with no deadline. With the
  defaults (send 30 s, Unix idle off), a Unix flood released the healthy client 28.6 s after it
  started waiting, about 30 s after the stall. The log said
  `client dropped: a write to it failed (stopped reading?) transport=unix errno=11 send_timeout_ms=30000`.
- **A send deadline on every connection, on by default.** The new `src/conn.cyr`
  (`bote_conn_send_deadline`) applies `BOTE_SEND_TIMEOUT_MS`, default 30000, as `SO_SNDTIMEO`:
  - the Unix transport applies it to each accepted client;
  - HTTP, bridge, Streamable HTTP and WebSocket apply it at the top of their handlers, since
    sandhi's server sets only read deadlines on the connections it accepts.

  A write parks only when the peer has stopped reading entirely, so the deadline only ever drops
  a peer that is already broken.
- **A failed write now drops the client.** Before, every write result was ignored:
  - the Unix transport writes each reply line through `_unix_reply` (`sock_send_all`, which also
    stops a partial write from leaving half a line on the wire) and drops the client on failure;
  - the WebSocket loop checks `ws_server_send_text` and drops the connection, whose stream is
    desynchronised mid-frame;
  - the Streamable HTTP SSE drain stops at its first failed chunk, where carrying on would make
    each later chunk wait out the full deadline again.

  Each drop is one WARN line with the transport and errno.
- **The Unix receive deadline is opt-in** (`BOTE_UNIX_IDLE_MS`, default 0). The Unix transport is
  persistent: an MCP client may keep its connection open and quiet between requests, as a
  WebSocket client may. A default receive deadline would drop those healthy clients, which is the
  defect 3.3.7 fixed for WebSocket by making `ws_config_with_idle_ms` default to off. So a quiet
  client still holds the single-threaded Unix server unless an operator opts in, and the probe's
  "sends nothing" row stays wedged at the defaults by design. A read that ends on the idle
  deadline is a DEBUG line.
- **The binaries read both from the environment.** `bote_main_conn_from_env` parses
  `BOTE_SEND_TIMEOUT_MS` / `BOTE_UNIX_IDLE_MS` strictly (digits only; `0` disables), keeps the
  default with a WARN on anything else, and logs the values at INFO when either differs from its
  default. An embedder assigns the two globals before starting a transport.
- Tests (+26 assertions):
  - `tests/bote_transport_unix.tcyr` +19, over a `socketpair` with a shrunk `SO_SNDBUF`. AF_UNIX
    charges in-flight bytes to the sender, so a small send buffer on the server end is what backs
    its replies up. Covered: the send deadline gives `EAGAIN` within 150 ms – 5 s; 0 disables it;
    the idle deadline is off by default and gives `EAGAIN` when set; and `_unix_handle_client`,
    fed 2,000 pings by a peer that reads nothing, returns, logs the WARN, and wrote replies before
    the drop.
  - `tests/bote_ws.tcyr` +7: the real `_bote_ws_handler`, driven through the handshake and fed
    1,500 masked `tools/list` frames by a peer that never reads, returns and logs the drop.
  - Mutations, each run under `cyrius test --timeout 20`: removing the WebSocket send deadline,
    ignoring the WebSocket write failure, and ignoring the Unix write failure each make the unit
    hang until it is killed and reported FAIL. A regression therefore fails CI at the runner's
    default 300 s per-unit timeout instead of passing.
  - All four affected units pass under `qemu-aarch64`.

### Changed

- New module `src/conn.cyr`, in `[lib]` (33 modules) ahead of the transports. Not in `[lib.core]`,
  which carries no transport. `src/main.cyr` and the four test units that include a transport
  include it.
- Binaries: `bote` 2,821,576 → 2,825,968 B; aarch64 3,325,376 B; agnos 2,790,312 B with 0
  undefined.
- Benchmarks: `tests/bote.bcyr` links neither the transports nor `conn.cyr`, and none of its modules
  changed. The logged 3.4.3 row matches 3.4.2 (`dispatch_tools_call` 1.802 µs against 1.770 µs,
  `codec_process_message` 3.770 µs against 3.682 µs, at load 1.9).
- CLAUDE.md, README (a connection-deadlines note beside logging), `DEPS-PATTERN.md`,
  `docs/architecture/overview.md`, SECURITY.md and the roadmap updated. The roadmap's next patch is
  3.4.4, DNS-aware SSRF.

## [3.4.2] — 2026-10-15 · one bote binary again: every transport by argv, the old names as aliases

The roadmap's 3.4.2 item. The three per-transport binaries are one `bote` again, with the transport
chosen by argv. The split dated from 2.7.2, a workaround for the cyrius 5.10.x 2 MB
compile-source cap, which 6.1.24 raised. bote carried it for twenty-odd releases after the reason
was gone.

### Changed — one binary, every transport by argv

- **`src/main.cyr` serves all six transports.** `src/main_streamable.cyr` and `src/main_ws.cyr` are
  deleted:

  | Command | Transport | Default |
  |---|---|---|
  | `bote` / `bote stdio` | stdio | — |
  | `bote http [port]` | HTTP | 8390 |
  | `bote unix <path>` | Unix socket | — |
  | `bote bridge [port]` | TS bridge with CORS | 8391 |
  | `bote streamable [port]` | Streamable HTTP / SSE | 8392 |
  | `bote ws [port]` | WebSocket | 8393 |

  `bote stdio` is new as an explicit spelling of the default. An unrecognised mode still serves
  stdio, as it always has, and its warning now lists the valid modes.
- **The old binary names still work.** `bote_main_mode` matches argv[0]'s basename, so
  `bote-streamable [port]` and `bote-ws [port]` select their transports when invoked through a
  symlink. `scripts/build-all.sh` builds `build/bote` and links `build/bote-streamable` and
  `build/bote-ws` to it, which also replaces a stale pre-3.4.2 binary of either name in `build/`.
- **Each transport keeps the tool set it had.** As separate binaries, streamable and ws registered
  only the echo / greeting / info references, while `bote` also registered the libro, filesystem
  and web tools. Building one dispatcher before choosing the mode would have exposed `fs_write` and
  `web_fetch` over two more network transports, and a consolidation should not decide that. So
  streamable and ws build their own echo-only dispatcher, exactly as before. Whether they should
  expose the full set is an open roadmap decision. The session store and the notification event
  sink are still wired for streamable alone, so every other mode keeps 3.4.1's syscall-free
  `tools/call`.
- **Nothing that was released changes.** `release.yml` has only ever built and shipped `build/bote`,
  zugot's recipe installs only `bote`, and the agnos repo's MCP smoke builds `src/main.cyr`. The one
  released binary simply gains two modes. The agnos binary does too.
- Binaries: one `bote` of 2,821,576 B (+25,080 B over 3.4.1's) replaces three totalling
  8,313,872 B. On aarch64 it is 3,325,080 B and on agnos 2,785,920 B, with 0 undefined.
  Capacity for `src/main.cyr`: `fn_table` 6,222 → 6,313 / 131,072 (4.8%), `identifiers` 198,201 →
  200,628 / 8,388,608 (2.4%). The compile-source cap that forced the split has not been approached.
- CI:
  - The three places that listed three entries now list one: the manifest gate's `ENTRIES` and the
    aarch64 and agnos cross-build loops.
  - **New "Transport smoke" step**, `scripts/transport-smoke.sh`, which developers can run too. It
    starts http, bridge, streamable, ws and unix from the built binary and checks:
    - each answers `initialize` (ws: the upgrade answers 101);
    - both argv[0] aliases select their transports;
    - http lists `fs_write` and streamable does not.

    Run verbatim it passes. With `fs_tools_register` planted on the streamable dispatcher it fails
    and names the parity break.
- Checked beyond CI:
  - `scripts/ws-idle-probe.py` against `bote ws` held a framed connection through a 3 s idle (two
    `tools/list` round trips, each with its DEBUG line). Its usage line now reads
    `./build/bote ws 8399`.
  - The aarch64 binary under `qemu-aarch64` answered `initialize` over stdio and over streamable on
    a port.
- `src/session.cyr`'s comment on why the outbound slot stays type-agnostic no longer refers to "the
  bote-streamable binary".
- Benchmarks: `tests/bote.bcyr` links no entry code and none of its modules changed. The logged
  3.4.2 row matches 3.4.1 (`dispatch_tools_call` 1.770 µs against 1.794 µs, load 3.6).
- CLAUDE.md (Binaries, entries, commands, the compile-source rule), README (build, run, try-it,
  modules), `docs/architecture/overview.md` and the roadmap now describe one binary. The roadmap's
  next patch is 3.4.3, and it gains the tool-set question as an open decision.

## [3.4.1] — 2026-10-15 · resources/templates/list; the "+58%" was the host's syscalls, and tools/call stops paying them

The two items that sat under "next patch": `resources/templates/list`, the last ❌ in
`docs/spec-compliance.md`, and the `dispatch_tools_call` regression that had been open since
3.3.16. The regression turned out not to be bote's, and bote no longer pays for it. The roadmap
is reorganised to say what each remaining 3.4.x patch carries.

### Added — `resources/templates/list`

- **It answers `{"resourceTemplates":[]}` whenever resources are advertised.** Until now it
  answered `-32601`, and some clients send it whenever `resources` is in the capabilities and read
  that error as a server failure rather than "no templates". bote has no URI-template registry
  (RFC 6570), so the empty list is the honest reply. A pagination `cursor` is ignored, because
  there is never a next page. The method is gated like `resources/list`: with no
  `ResourceRegistry` it is still `-32601`, matching the capability that is not advertised.
  Verified on the built binary over stdio.
- `tests/bote.tcyr` +6 assertions: `-32601` without a registry, the empty list with one, a `cursor`
  ignored, and the serialized wire line. Removing the dispatch branch fails 4 of them. Each check
  treats a null result as `""`, so a missing method reports a failure instead of crashing
  `streq`, which an earlier draft of the test did.

### Performance — `tools/call` reads the clock only when something consumes the duration

- **Where the "+58%" went.** Since 3.3.16 the roadmap carried `dispatch_tools_call` +58% and
  `codec_process_message` +41% against the 3.3.13 row of `benches/history.log`, blamed on 3.3.14
  (cyrius 6.6.6 → 6.6.10, libro 2.10.4). The roadmap's planned bisect could not build: 3.3.13
  does not compile on 6.6.10 (libro 2.10.3's thin sigil against the 3.13.4 fold, the five arity
  errors 3.3.14 fixed). The control that could build:
  - **3.3.13 rebuilt from its tag, on cyrius 6.6.6 with libro 2.10.3, today** runs
    `dispatch_tools_call` within 1.7% of 3.4.0 and `codec_process_message` within 2%
    (interleaved). The 3.358 µs logged on 2026-09-22 does not reproduce with the same code on
    today's host.
  - Those are the only two benchmarks that make a syscall. Every `tools/call` read
    `clock_now_ns()` twice (a raw `clock_gettime`, unchanged in `lib/chrono.cyr` across 6.6.6 →
    6.6.10), and one costs **1,344–1,350 ns** on this host now (1M-call loop, five runs). Two of
    them account for the whole +1.94 µs.
  - The box took a kernel update on 2026-10-02 (7.2.7 → 7.2.8, the reboot that started its
    current uptime). Every slow measurement, 3.3.15 onward, postdates it; 3.3.13's fast row
    predates it. The CPU runs `spec_rstack_overflow: Mitigation: Safe RET`. The kernel step
    itself is not proven, since that needs a reboot into the old kernel. What is proven is that
    bote, its toolchain and libro did not regress, and that the time is two syscalls.
- **The fix.** The duration and the `ToolCallEvent` exist only for an audit sink or an event
  sink. With neither installed, `audit_sink_log` and `event_sink_publish` return without calling
  anything, so `tools/call` now skips the clock reads, the event and its JSON. Each sink is gated
  on its own (`_dsp_sink_live`: installed with a non-null callback), so an audit-only dispatcher
  no longer builds the JSON only an event sink reads. Behaviour for every installed sink is
  unchanged.
- **Measured.** 3.4.0 against this tree, interleaved, 12 rounds, at a 1-minute load of 4–7 with
  other sessions busy:

  | Bench | 3.4.0 | 3.4.1 | Change |
  |---|---|---|---|
  | `dispatch_tools_call` | 5.636 µs | 1.853 µs | **−67.1%** (ranges 5,578–5,776 vs 1,804–1,913 ns) |
  | `codec_process_message` | 7.542 µs | 3.838 µs | **−49.1%** |

  The other twelve rows are within −1.9% … +3.4%, and they run none of the changed code. Removing
  only the two clock reads, in a scratch copy, gives 2.506 µs. The remaining 0.65 µs is the event
  object and its JSON, now also skipped. `dispatch_tools_call` is now below 3.3.13's logged
  3.358 µs.
- Where the binaries run: `bote` (stdio / http / unix / bridge) installs no audit or event sink,
  so every `tools/call` there skips both syscalls. `bote-streamable` installs the notification
  event sink, so it keeps the timing, as it should.
- `tests/bote.tcyr` +15 assertions cover each sink on its own:
  - audit-only: still audits, publishes nothing;
  - events-only: publishes completed and failed, audits nothing;
  - null-callback sinks: count as absent;
  - no sinks: results and error codes unchanged.

  Gating the audit call on the events flag fails 2 of them.

### Changed

- **Roadmap.** Every item was checked against the code:
  - "Next patch" is now 3.4.2, the binary reconsolidation.
  - The 3.4.x table assigns each remaining item to a patch: 3.4.3 the Unix accept deadline and
    send-side timeouts, 3.4.4 DNS-aware SSRF, 3.4.5 the conformance suite.
  - **DNS-aware SSRF moves from 3.6.x to 3.4.4.** Its "upstream seam not yet filed" was stale:
    bote filed it at 3.3.7 and sandhi 1.9.14 shipped `sandhi_client_set_resolver`, which is in
    `lib/` today.
  - The sandhi errno filing moves from Housekeeping to Upstream, as filed.
  - The watch list gains the syscall-cost note above.
- `docs/spec-compliance.md`: `resources/templates/list` is ✅. No ❌ row remains.
- README and `docs/architecture/overview.md` list the method.
- `dist/bote.cyr` and `dist/bote-core.cyr` regenerated. `benches/history.log` gains the 3.4.1
  block (load 2.1); the README table is refreshed from it.

## [3.4.0] — 2026-10-15 · bote logs through sakshi; cyrius 6.7.9, libro 2.11.0, majra 2.12.0

bote had no logging. sakshi was in `[deps] stdlib` only because libro links it, and bote's own
diagnostic output was a single `eprint`, on a CSPRNG failure. A bind that failed, an accept loop that
gave up, an auth rejection, a refused path: each returned a status code and said nothing. This
release routes bote's operational output through sakshi. It also moves the toolchain to the 6.7
language minor and takes the libro and majra tags that pin it. Wiring the WARN line for SSRF blocks
showed that `web_fetch` never applied the SSRF guard SECURITY.md promises, so it does now.

### Added — operational logging through sakshi (`src/log.cyr`)

- **One module, first in both profiles.** `bote_log_line` / `bote_log_str` / `bote_log_int` /
  `bote_log_emit` build a line in a buffer on the caller's stack (`var ln[BOTE_LOG_LINE_BYTES];`) and
  hand it to sakshi. A level that is filtered out marks the line dead at `bote_log_line`, and every
  later call returns at once, so a suppressed line costs one compare and nothing is allocated.
  `bote_log_warn(msg)` and its siblings cover a plain message. `bote_log_level_from_name`,
  `bote_log_level_name` and `bote_log_enabled` round it out.
- **Library code never configures sakshi.** Only a binary's `main` calls `bote_log_init`. An embedder
  that vendors a bundle keeps its own `sakshi_set_level` / `sakshi_output_*`, and bote's lines go
  wherever it sends sakshi's.
- **The binaries read two variables.**

  | Variable | Values | Default |
  |---|---|---|
  | `BOTE_LOG` | `off` · `fatal` · `error` · `warn` · `info` · `debug` · `trace` (case-insensitive; `none`, `warning` accepted) | `info` |
  | `BOTE_LOG_FILE` | a path; lines are appended there instead of stderr | stderr |

  An unrecognised `BOTE_LOG` keeps `info` and says so. A `BOTE_LOG_FILE` that cannot be opened keeps
  stderr and says so, with the errno. At `trace` the binaries also turn on sandhi's request spans,
  which sandhi keeps off unless asked. libro already logs through sakshi (61 calls), so `BOTE_LOG`
  now governs its lines too.
- **Lines never reach stdout.** On the stdio transport stdout is the JSON-RPC wire. sakshi writes to
  fd 2, and the new CI step "Logging smoke" runs the built binary at nine `BOTE_LOG` values. Each run
  must give exactly one JSON-RPC line on stdout and the stderr that level promises. Planting
  `sakshi_set_output_fd(1)` in `bote_log_init` fails the step (log lines on stdout, a stdout that is
  no longer one JSON-RPC line, no starting line on stderr).
- **What is logged, and at what level:**

  | Level | Event |
  |---|---|
  | INFO | `bote starting` (version, transport, address and port or socket path, auth mode, log level) · `bote stopped` · `listening` (Unix) · `bearer auth enabled tokens=N` · `host config reloaded` · `unix accept recovered` |
  | ERROR | `transport failed` with the rc (the four sandhi-served transports cannot say which of socket, bind, listen or accept failed, because sandhi returns a bare 1, so the line names all four) · `unix transport failed step=socket\|bind\|listen\|accept errno=N` · `unix transport gave up accepting` · `usage: bote unix <socket-path>` |
  | WARN | `request refused` for 401 (`missing_token` / `invalid_token`; no `transport` field, since the shared `auth_bearer_check` is not told which transport called it), 403 origin, 413, and a rejected `Transfer-Encoding` · `web_fetch refused by the SSRF guard` (reason, host) · `fs tool refused an unsafe path` · `BOTE_BEARER_TOKENS is set but holds no token; bearer auth is OFF` · `unknown mode; serving stdio` · `port argument not usable` · `unix accept failing; backing off` (once per episode, not once per retry) · `host config reload failed` |
  | DEBUG | one `request` line per JSON-RPC request (method, id, tool for `tools/call`, `status=ok\|error\|notification`, error code, elapsed µs) · `request refused` for 404, 405, an unknown session, a protocol-version mismatch, a malformed request |
  | FATAL | the session-ID CSPRNG failure (was the raw `eprint`; the fail-closed `exit 90` is unchanged) |

  The 34 refusals in the four HTTP-family transports now go through one helper,
  `bote_http_refuse(cfd, level, transport, status, text, reason)`, which logs and sends. None of them
  logged before, and the helper keeps each log line next to the status it describes.
- **No secret goes into a line.** No bearer token, JWT, request body, tool argument or URL. A tool
  name, a method, a status, a length, a host or an errno does. `web_fetch` logs `ssrf_host(url)`, the
  host alone, because a URL's userinfo or query string can carry a credential. Tests check that
  `Bearer sk-live-SECRETVALUE`, a `hunter2` in URL userinfo, and a tool argument never appear in the
  line.
- **Values are logfmt, and client-supplied ones cannot forge a line.** A value with a space, `"`, `=`,
  `\`, a control byte or a byte ≥ 0x80 is quoted. `"` and `\` are escaped, `\n` `\r` `\t` are spelled
  out, other control bytes and DEL are `\xHH`, UTF-8 passes through, and a value over 96 bytes is cut
  and ends `...`. This is not theoretical: bote's JSON parser accepts a raw LF inside a string, so
  `{"method":"y<LF>[0] [ERROR] forged"}` puts a newline byte in the method name. It is logged as
  `method="y\n[0] [ERROR] forged"`, one line. Removing the escape fails 2 assertions in `bote.tcyr`
  and 2 in `bote_log.tcyr`.
- New public helpers in `src/host.cyr`: `ssrf_reason(code)` (the lowercase name of an `SSRF_*` code)
  and `ssrf_host(url)` (the host only, with userinfo, port and IPv6 brackets stripped).
- Tests:
  - New `tests/bote_log.tcyr`, 50 assertions. It captures sakshi's output through its emit hook, so
    it checks the exact bytes and level sakshi receives.
  - Additions to existing files: `bote.tcyr` +11 (the per-request line: absent at INFO, its fields, a
    tool name but never its arguments, both newline forms), `bote_auth.tcyr` +7, `bote_host.tcyr` +9,
    `bote_web_tools.tcyr` +10, `bote_fs_tools.tcyr` +5.
  - Mutations: three of four fail as they should. Removing the newline escape fails 2 assertions in
    `bote_log.tcyr` and 2 in `bote.tcyr`; removing the dead-line check fails 1; the logging smoke's
    stdout plant fails the step. The fourth, removing `bote_log`'s early level check, survives by
    design: sakshi's per-level calls filter again, so that check saves a `strlen` and changes no
    output. The code now says so.

### Security

- **`web_fetch` now applies the SSRF guard.** SECURITY.md has said since 2.x that "the `host` module's
  SSRF guard rejects URLs targeting loopback / link-local / private / cloud-metadata endpoints … before
  any network call". `web_fetch`, bote's one fetch of a client-supplied URL, never called
  `ssrf_check`. So any client of the default `bote` binary could read loopback services and
  `169.254.169.254` through it. It is refused now, with an `isError` result naming the reason and a
  WARN line. `bote_web_tools.tcyr` checks loopback (IPv4 and IPv6), the metadata address and a
  private address; each is refused before any socket opens. ⚠ The guard classifies IP literals and a
  hostname blocklist, and does not resolve DNS. A name that resolves to a private address still gets
  through, which is the roadmap's 3.6.x DNS-aware SSRF item. `web_search` is not guarded, by design:
  its endpoint is the operator's `BOTE_SEARXNG_URL`, often on localhost.
- **An empty `BOTE_BEARER_TOKENS` no longer turns auth off silently.** Set to `,` or `,,`, it parsed
  to zero tokens and the HTTP transports ran with no auth while the operator believed it was on. Auth
  is still off in that case, as before, but a WARN line now says so at startup.

### Changed

- **cyrius 6.6.20 → 6.7.9**, the 6.7 language minor. The re-provisioned folds move sigil 3.13.10 →
  3.13.11, sakshi 2.5.7 → 2.5.8, patra 1.15.2 → 1.16.0, bayan 1.5.12 → 1.5.13, sandhi 1.10.8 →
  1.10.9, ganita 1.2.14 → 1.2.15 and sankoch 2.8.1 → 2.8.3. A from-scratch resolve
  (`rm -rf lib cyrius.lock`, `lib sync --full`, `deps`) locks the same 115 files; `deps --verify`
  115/0. Every entry and unit built with no source change and no new warning.
- **The 6.7.x silent-semantics changes do not reach bote.** Three 6.7.x changes alter what
  already-compiling code computes, so a green suite proves nothing about them:
  - before 6.7.3 the lexer dropped a lone `!`, so `if (!ok)` meant `if (ok)`;
  - before 6.7.9 a unary `-` / `~` on a constant followed by an operator and a literal computed the
    wrong value;
  - before 6.7.9 a `*T` pointer's step leaked through operators.

  A comment- and string-stripped scan of `src/`, `tests/` and `fuzz/` (54 files) for all three, and
  for the newly reserved `trait` / `const` / `true` / `false` / `do` / `dyn`, found nothing. The
  scanner was proven first on a planted file: it caught all five shapes and ignored `!` in comments,
  in strings and in `!=`.
- **libro 2.10.8 → 2.11.0 and majra 2.9.3 → 2.12.0**, the latest tags, both pinning 6.7.9.
  - libro's bundle differs from 2.10.8's only in its version header, so `struct chain` /
    `struct entry` / `struct error` are unchanged.
  - ⚠ libro 2.11.0 pins sigil 3.13.12, one patch past the 6.7.9 fold (3.13.11), by libro's choice.
    bote's lock therefore commit-pins sigil 3.13.12 while `lib/sigil.cyr` is 3.13.11: the thin
    profile is still not vendored here, so the fold is what compiles. libro reports that its thin
    set differs between the two only in the version header, and that cyrius 6.7.10 folds 3.13.12.
    Patra's pin, 1.16.0, equals the fold.
  - majra 2.10.0 – 2.12.0 are additive and change no signature or wire format; bote calls only
    `pubsub_publish`.
- **Both bundles carry `src/log.cyr`.** `[lib]` grows to 32 modules and `[lib.core]` to 13. The
  sidecars are unchanged (`hashmap bayan ws_server syscalls` / `hashmap bayan sigil ct`). sakshi
  reaches the core bundle's verify through `lib/patra.cyr`'s own include (patra is libro's named
  dep), which is how a core consumer gets it too. Measured: scratch consumers of each bundle on
  6.7.9, declaring only `string fmt alloc vec str syscalls io freelist`, build with zero undefined
  names and answer a `ping`. That is independent of the sigil over-report filed
  against cyrius at 3.3.17. ⚠ `log.cyr` sizes its line buffer with a `const`, so the bundles need
  cyrius ≥ 6.7.2 to compile.
- **A port argument that does not parse, or is outside 1–65535, warns and uses the default.** It
  used to fall back in silence (a non-number) or be passed through (65536 and up). An unrecognised
  mode (`bote frobnicate`) still serves stdio, as before, and now says so.
- **The manifest's `path` rule is restated for cyrius 6.7.6**, verified on this tree. A `path` beside
  `git` / `tag` is a development override: a default resolve built libro's tag, printed `hint: 1 dep
  has a local checkout not in use`, and left `cyrius.lock` alone. `CYRIUS_LOCAL=libro` resolved into
  `build/local-deps/lib/` and left `lib/libro.cyr` and the lock untouched. The comment used to say
  `path` beats `tag`, which stopped being true at 6.7.6.
- `cyrius.cyml`'s sakshi line said "sakshi is listed because libro links it, not because bote calls
  it". Now it is bote's logger.
- CI's "Run tests" list gains `tests/bote_log.tcyr`, and now matches `tests/*.tcyr` exactly.
- `dist/bote.cyr` and `dist/bote-core.cyr` regenerated.
- Binaries, from 3.3.17 on 6.6.20 to 3.4.0 on 6.7.9: `bote` 2,768,920 → 2,796,448 B,
  `bote-streamable` 2,733,400 → 2,756,544 B, `bote-ws` 2,737,672 → 2,760,736 B. The toolchain alone
  accounts for +14,032 B on `bote` (2,782,952 B before any logging code), and logging for +13,496 B.
  The only build warning is `lib/sigil.cyr`'s static-array note, which was already there. Capacity
  for `src/main.cyr`: `fn_table` 6,160 → 6,222 / 131,072 (4.7%), `identifiers` 196,472 → 198,201 /
  8,388,608 (2.4%).

### Performance

Three bench binaries ran in one interleaved rotation, 15 rounds each with the order rotating, at a
1-minute load of 1.6–2.1: 3.3.17 on 6.6.20, this tree on 6.7.9 before any logging code, and this
tree as released. Medians:

| Bench | 6.6.20 | 6.7.9 | Toolchain | 6.7.9 + logging | Logging |
|---|---|---|---|---|---|
| `dispatch_tools_call` | 5.866 µs | 5.888 µs | +0.4% | 5.744 µs | −2.4% |
| `codec_process_message` | 7.630 µs | 7.676 µs | +0.6% | 7.696 µs | **+0.3%** |
| `jsonx_get_str_flat` | 157 ns | 149 ns | −5.1% | 142 ns | −4.7% |
| `validate_compiled_simple` | 529 ns | 492 ns | −7.0% | 501 ns | +1.8% |
| `codec_parse_request` | 1.373 µs | 1.363 µs | −0.7% | 1.424 µs | +4.5% |
| `validate_compiled_nested` | 2.136 µs | 2.129 µs | −0.3% | 2.224 µs | +4.5% |

- **The toolchain move is a wash.** 12 of 14 rows are within ±0.8%; the two alignment-sensitive rows
  are faster.
- **Logging costs nothing measurable where it runs.** `codec_process_message` is the one benchmark
  that passes through `_cdc_process_single`'s new level check, and it moved +0.3% with overlapping
  ranges. The bench runs at sakshi's default INFO, the binaries' default, so this is the
  suppressed-line cost: one compare per request.
- **The ±4.5% rows run no new code.** Symbol maps (`CYRIUS_SYMS`) of the pre-logging and released
  bench binaries show `jsonx_get_raw`, `_jx_find_value_pos`, `codec_parse_request` and
  `compiled_validate` at identical sizes, differing in 4–8 bytes (call displacements). Each sits at
  a new offset in its 64-byte line (`codec_parse_request` 43 → 31). The one function that grew is
  `_cdc_process_single`, 486 → 650 bytes. Both reconstructed binaries are byte-identical to the
  ones benchmarked.
- `benches/history.log` gains the 3.4.0 block from a single run at load 1.9; the README table is
  refreshed from it.

## [3.3.17] — 2026-10-15 · cyrius 6.6.20 + libro 2.10.8 + majra 2.9.3: the Unix transport survives a reset peer, and the sidecars are compile-verified

This release moves the toolchain from 6.6.14 to 6.6.20 and takes the libro and majra tags that pin 6.6.20. Nothing
broke on the way: all three entries, every test / bench / fuzz unit and both bundles built clean at the new pins with
no source change. bote's own changes are one regression assertion, the dist gate learning to compare the `.deps`
sidecars, and two `cyrius.cyml` rules retired because the toolchain no longer needs them.

### Security — a Unix-socket client could kill bote by resetting the connection (inherited fix)

`_unix_handle_client` writes each reply with `sock_send`. Through cyrius 6.6.15 that was a flagless `write(2)`, so a
client that reset the connection before reading its reply raised SIGPIPE, and SIGPIPE's default action ended the
process. The four sandhi-served transports (HTTP, bridge, Streamable HTTP, WebSocket) were not exposed: sandhi's
server has installed a process-wide `SIG_IGN` for SIGPIPE since sandhi 1.6.6. The Unix transport runs bote's own
accept loop and never installed one. Reproduced against the binaries: `bote unix <path>`, then a client that sends
2,000 `tools/list` lines and closes with `SO_LINGER {1, 0}` (an RST):

| Build | Runs | Outcome |
|---|---|---|
| 3.3.16 (cyrius 6.6.14) | 3 | died, exit 141, all three runs |
| 3.3.17 (cyrius 6.6.20) | 3 | alive, all three runs; the next client's `ping` is answered |

cyrius 6.6.16 (its CVE-74) routes every socket write through `_fd_write_nosigpipe` (`send` with `MSG_NOSIGNAL` on
Linux), so a reset peer is now a `-EPIPE` return. Severity is limited by the socket itself: `AF_UNIX`, mode 0600, so
the client has to be the same user or root. `tests/bote_transport_unix.tcyr` gains two assertions (47 → 49). They write
to the closed half of a `socketpair` with `sock_send` and `sock_send_all` and expect `-EPIPE`. Getting past those
writes is the real assertion: on the 6.6.14 toolchain the file is killed by signal 13 before it prints a verdict
(rc 141, measured).

### Changed

- **cyrius 6.6.14 → 6.6.20.** The re-provisioned folds move sigil 3.13.7 → 3.13.10, sakshi 2.5.6 → 2.5.7, patra
  1.15.1 → 1.15.2, bayan 1.5.11 → 1.5.12, sandhi 1.10.4 → 1.10.8, ganita 1.2.11 → 1.2.14 and sankoch 2.8.0 → 2.8.1.
  The snapshot gains one leaf, `poison` (6.6.18's `freelist` includes it), so a clean resolve locks 115 entries, up
  from 114. The lock was regenerated from scratch (`rm cyrius.lock`, `lib sync --full`, `deps`) because 6.6.20 keeps
  a commit pin once it is recorded: a resolve over 3.3.16's lock left the 2.10.6 / 2.9.2 / 3.13.7 / 1.15.1 rows
  beside the new ones. `deps --verify`: 115 verified, 0 failed.
- **sandhi 1.10.7's whole-request budget now applies to the four sandhi-served transports.** bote calls
  `sandhi_server_run` with null options. Reading sandhi's source, that takes the new default
  `SANDHI_SERVER_DEFAULT_REQUEST_MS = 60000`: a connection must deliver a complete request within 60 s of accept, or
  it gets `408 Request Timeout` (plaintext) and is closed. This is the Slowloris guard `idle_ms` alone could not
  give. It was not exercised here. A client that streams a large POST body over a slow link for more than a minute
  is the case it would cut.
- **libro 2.10.6 → 2.10.8 and majra 2.9.2 → 2.9.3**, the releases that pin cyrius 6.6.20.
  - libro 2.10.7 declares sigil 3.13.10 and patra 1.15.2, equal to the 6.6.20 folds, so the lock's four commit pins
    (libro `53401e7`, majra `ddf26f1`, patra `2bc1925`, sigil `6d065da`) name the code `lib/` holds. 2.10.8 makes
    `patrastore_*` fail cleanly before `patra_init()` and on a null handle. bote calls neither.
  - `struct chain`, `struct entry` and `struct error` are byte-identical to 2.10.6's, so the struct-layout contract
    holds for `src/libro_tools.cyr`.
  - majra 2.9.3 adds a `# Requires` block to its bundle and changes comments. The pubsub surface bote uses is
    unchanged.
  - libro's thin sigil is still not vendored here (the four "not vendored" notes), and the build prints no
    `duplicate fn` warning.
- **The bundles open with a `# Requires` block, and the sidecars are what the compile-verify proves** (cyrius 6.6.18's
  `distlib`). `dist/bote.deps` goes from 32 leaves to 4 (`hashmap bayan ws_server syscalls`) and
  `dist/bote-core.deps` from 8 to 4 (`hashmap bayan sigil ct`). The rest arrive through those leaves' own requires
  blocks. **Consumer impact:** a consumer that calls a stdlib family from its own code, and got that leaf only through
  bote's old sidecar, now lists it in its own `[deps] stdlib` (cyrius `docs/development/ecosystem-migration-6.6.18.md`).
  The cyrius 6.6.18 CLI prints a `hint:` line naming the leaf.
- ⚠ **`dist/bote-core.deps` names `sigil` and `ct`, which no core module uses.** distlib's verify compiles the bundle
  with bote's named deps in scope. libro calls `ct_eq_bytes_lens` and `SIG_ALG_ED25519`, and libro 2.10.7's sidecar
  shrank to `hashmap`, leaving those needs to its thin `sigil-mldsa` requires block. In bote that file is never
  vendored, because `sigil` is a stdlib leaf here. So the verify records the two leaves against the core bundle. The
  cost was measured with a scratch consumer that declares `[deps.bote] modules = ["dist/bote-core.cyr"]` and answers
  one `ping`:

  | Core bundle | Sigil in the consumer's unit | Binary |
  |---|---|---|
  | 3.3.16, consumer on 6.6.14 | the whole fold (through libro 2.10.6's 27-leaf sidecar) | 2,628,808 B |
  | 3.3.17 as shipped, consumer on 6.6.20 | the whole fold | 2,079,184 B |
  | 3.3.17 with the two leaves removed by hand | libro's thin `sigil-mldsa` 3.13.10 | 1,209,096 B |

  So core consumers pay nothing they did not pay at 3.3.16, but they miss the drop the thin profile would give. All
  three builds print zero `undefined` lines and answer the `ping` correctly. The bundle ships as `distlib` writes it,
  because CI gates `dist/` byte-for-byte. The cause is in cyrius (`cbt/deps.cyr`'s keep-the-fold branch never records
  the fold as standing in for the dep's module) and is filed there as
  `docs/development/issues/2026-10-15-bote-distlib-kept-fold-over-thin-profile-records-named-deps-need.md`.
- **distlib reports six names undefined on x86_64-windows**: majra's `SYS_FUTEX` / `FUTEX_WAIT` / `FUTEX_WAKE`,
  `sys_lstat`, `sys_fchmod` and bote's `sys_accept4`. This is a warning, the exit status is 0, and Windows is not a
  bote target.
- **Two `cyrius.cyml` rules retired.** Their reasons are gone at this pin. Both were verified on this tree with
  6.6.20: the probes below left the lock entries, all four `dist/` files and the `bote` binary byte-identical.
  - "No comments inside the array": 6.6.17 reads the manifest with a single reader that skips comments. Probe: a
    comment containing `]`, placed mid-`stdlib`.
  - "Write `deps.NAME` in comments, never the bracketed section header": 6.6.20 anchors distlib's header match.
    Probe: `[deps.hashmap]`, `[deps.patra]` and `[deps.sigil]` in comment prose.

  The `path` rule stays.
- Binaries, from 3.3.16 on 6.6.14 to 3.3.17 on 6.6.20: `bote` 2,710,608 → 2,768,920 B, `bote-streamable` 2,670,984 →
  2,733,400 B, `bote-ws` 2,675,256 → 2,737,672 B. The only build warning is `lib/sigil.cyr`'s static-array note,
  which was already there. Capacity for `src/main.cyr`: `fn_table` 6,060 → 6,160 / 131,072 (4.7%), `identifiers`
  193,298 → 196,472 / 8,388,608 (2.3%), `var_table` 2,737 → 2,794 / 1,048,576 (0.3%).
- `dist/bote.cyr` and `dist/bote-core.cyr` regenerated (the requires block, the version header and the
  `_bote_server_version()` literal).
- CLAUDE.md, `docs/architecture/overview.md`, `DEPS-PATTERN.md`, the README and the compliance header name the new pins,
  folds, sidecars and test counts. The README and overview test counts had stayed at 887 since 3.3.13; they now
  read 906. The roadmap drops its done follow-up (the `lib sync` before `deps` order) and lists the cyrius filing.

### Fixed — CI did not compare the `.deps` sidecars

The "dist bundles are up-to-date" step in `ci.yml`, and "Verify dist bundles match committed copies" in
`release.yml`, diffed only `dist/bote.cyr` and `dist/bote-core.cyr`. A stale or hand-edited sidecar passed both. From
cyrius 6.6.18 the sidecar is a compile-verified consumer contract, and the bundle's requires block is generated from
it. Both steps now diff each bundle together with its sidecar. Run verbatim under `bash -eo pipefail` on a committed
copy of this tree, the CI step returns rc 0. With `assert` appended to a committed `dist/bote-core.deps` it returns
rc 1 and names the file. The old step passes that plant.

### Performance

- **The toolchain move is a wash on 12 of 14 rows.** `tests/bote.bcyr` was built by 3.3.16 on 6.6.14 and by 3.3.17 on
  6.6.20. The two binaries ran interleaved for 10 rounds, order alternating, at a 1-minute load of 1.5. Medians:

  | Bench | 6.6.14 | 6.6.20 | Change |
  |---|---|---|---|
  | `dispatch_initialize` | 1.272 µs | 1.309 µs | +2.9% |
  | `dispatch_tools_call` | 5.300 µs | 5.454 µs | +2.9% |
  | `codec_process_message` | 7.185 µs | 7.107 µs | −1.1% |
  | `codec_serialize_response` | 441 ns | 420 ns | −4.9% |
  | `jsonx_get_str_flat` | 125 ns | 145 ns | **+16.0%** |
  | `validate_compiled_simple` | 452 ns | 484 ns | **+7.1%** |

  The other eight rows are within −4.3% … +3.2%. An earlier interleaved run at load 4.4 gave the same shape:
  `jsonx_get_str_flat` +15.8%, `validate_compiled_simple` +8.9%, the rest within ±5.5%.
- **The `jsonx_get_str_flat` move is placement, not codegen.** Symbol maps (`CYRIUS_SYMS`) of the two bench binaries
  show `_jx_skip_ws`, `_jx_skip_string` and `_jx_find_value_pos` with identical sizes. Their bytes differ in 0, 2 and
  15 places, consistent with call and global displacements. Each function sits 4 bytes later in its 64-byte line
  (`_jx_skip_ws` at offset 38 → 42). This is the alignment sensitivity the README's benchmark note describes; 3.3.16
  measured the same row −9.5% on its own toolchain move. `compiled_validate` shows the same +4 shift.
- `benches/history.log` gains the 3.3.17 block from a separate single run at load 0.78. The README table is refreshed
  from it.

## [3.3.16] — 2026-10-03 · cyrius 6.6.14 + libro 2.10.6: the lock names the folds, and no path lines

cyrius 6.6.14, libro 2.10.6 and majra 2.9.2, with no `path` line left in `cyrius.cyml`. The
only bote source change is that `src/web_tools.cyr`'s two `bayan_json_v_obj_get` calls take
the getter's non-deprecated name.

### Fixed — a required field after a nested object was reported missing

`registry_validate_params` falls back to bayan's flat `json_parse` + `json_get` when a tool
has no compiled schema. Before bayan 1.5.10, a nested object threw the parser's key/value
pairing off for the rest of the object, so `{"cfg":{"x":1},"path":"/tmp"}` against
`required: ["path"]` came back `BOTE_ERR_INVALID_PARAMS` ("missing required field: path")
for params that were valid. bayan 1.5.10, folded from cyrius 6.6.13, returns a nested
object as one value, and 6.6.14 folds 1.5.11. bote inherits the fix through the pin; its
source is unchanged. `tests/bote.tcyr` gains two assertions. The first, a field after a
nested object, fails on 3.3.15 (cyrius 6.6.10, bayan 1.5.8; measured on a clean 3.3.15
tree). The second, a key inside the nested object not satisfying a top-level requirement,
holds on both.

### Changed

- **cyrius 6.6.10 → 6.6.14.** The re-provisioned folds move sigil 3.13.4 → 3.13.7, bayan
  1.5.8 → 1.5.11, sandhi 1.10.2 → 1.10.4, sakshi 2.5.5 → 2.5.6 and ganita 1.2.8 → 1.2.11.
  patra (1.15.1) and sankoch (2.8.0) are unchanged. The TLS fixes of 6.6.13 / 6.6.14
  (CVE-59 … CVE-67) come with the snapshot, and `lib/math.cyr` no longer defines the
  `f64_le` / `f64_ge` / `f64_trunc` builtins 6.6.13 reserved.
- **libro 2.10.4 → 2.10.6 and majra 2.9.1 → 2.9.2.** libro 2.10.5 and 2.10.6 and majra 2.9.2 are pin-only releases:
  `dist/libro.cyr` and `dist/majra.cyr` differ from what 3.3.15 vendored only in their
  version headers. So `struct chain` / `struct entry` / `struct error` are unchanged for
  `src/libro_tools.cyr`. libro now declares patra 1.15.1 and sigil 3.13.7, the 6.6.14
  folds, so the lock's patra and sigil commit pins name the code `lib/` holds; at 3.3.15
  they named 1.14.3 and 3.13.4.
- **libro's thin sigil no longer lands in `lib/`.** From cyrius 6.6.11 the resolver keeps
  the stdlib fold over a thin profile of a package listed in `[deps] stdlib`. libro's
  `deps.sigil` selection (`sigil-mldsa` + `sha_ni` / `sha256` / `hex`) is therefore noted
  "not vendored", and only `lib/sigil.cyr` is compiled. The 236 `duplicate fn` warnings
  every 3.3.15 binary printed (708 across the three) are gone. The CI step "libro's thin
  sigil matches the sigil fold" passes as "no thin sigil vendored".
- **No `path` line in `cyrius.cyml`** (the roadmap follow-up recorded by cyrius 6.6.13).
  `path = "../libro"` and `path = "../majra"` made a local resolve vendor the sibling
  working trees while CI vendored the tags. Rule 2 above the git deps now says a `path`
  line is never committed.
- Binaries, from 3.3.15 on 6.6.10 to 3.3.16 on 6.6.14: `bote` 2,743,112 → 2,710,608 B,
  `bote-streamable` 2,707,600 → 2,670,984 B, `bote-ws` 2,711,856 → 2,675,256 B. All
  three builds print no warning from bote's own source.
- `dist/bote.cyr` and `dist/bote-core.cyr` regenerated (version header, the
  `_bote_server_version()` literal, and in the full bundle the two `web_tools` calls).
- CLAUDE.md's stack table, `docs/architecture/overview.md` and the compliance header name
  the new pins and folds. The roadmap drops the done follow-up, and its thin-sigil watch item
  now says what the CI step reports.

### Performance

- **The toolchain move is a wash.** `tests/bote.bcyr` was built by 3.3.15 on 6.6.10 and by
  3.3.16 on 6.6.14, and the two binaries were run interleaved for 6 rounds (order
  alternating) on a box other sessions held at a 1-minute load of 3.6–3.7. Medians: 13 of
  the 14 benchmarks are within ±2.2% (`dispatch_initialize` 1.345 → 1.315 µs, −2.2%;
  `codec_process_message` 7.428 → 7.461 µs, +0.5%; `validate_compiled_nested` 2.079 →
  2.105 µs, +1.3%). `jsonx_get_str_flat` is 142 → 128 ns, −9.5%. The load makes these
  medians fit only for comparing the two binaries. The README table is refreshed from a
  separate quiet run (load 0.99), logged as the 3.3.16 block of `benches/history.log`.
- ⚠ **Two rows moved before this release, not in it.** Against the last logged row (3.3.13
  on cyrius 6.6.6), `dispatch_tools_call` is 3.358 → 5.295 µs (+58%) and
  `codec_process_message` 5.033 → 7.104 µs (+41%), while the other twelve are flat or
  faster. The A/B above puts 3.3.15 at the same level (5.452 / 7.428 µs under load, where
  `dispatch_initialize` stayed at its quiet value). So the change came with 3.3.14 (cyrius
  6.6.10, libro 2.10.4), which logged no benchmark row. Not investigated here.

## [3.3.15] — 2026-09-30 · the aarch64 gate's raw-syscall denylist holds under pipefail

CI-only. No bote source change beyond the `initialize` version literal; the libro 2.10.4
and cyrius 6.6.10 pins are unchanged.

### Fixed — the raw `syscall(...)` denylist failed under `pipefail`

The second denylist in the "aarch64 portability gate" step builds
`raw=$(for f in …; do sed | grep | sed; done)` with no `|| true` on the per-file
pipeline. A clean file makes grep exit 1; under `pipefail` the last file's status becomes
the loop's, the assignment fails, and `set -e` ends the step with rc 1 right after the
`--- denylist: raw syscall(...)` banner, without an error line. The step is green on GitHub
only because a step with no `shell:` runs as `bash -e`, without pipefail. The first
denylist in the same step already carried `|| true`; the second now does too, per file.
Run verbatim (both denylists, up to the cross-build): rc 0 and "clean" under `bash -e` and
under `bash -eo pipefail`; with a planted `syscall(60, 0)` in `tests/`, rc 1 and the site
named under both. The pre-fix step gives rc 1 with no message under `bash -eo pipefail`.

### Changed

- `dist/bote.cyr` and `dist/bote-core.cyr` regenerated (version header and the
  `_bote_server_version()` literal).
- `docs/spec-compliance.md` header names Bote Version 3.3.15.

## [3.3.14] — 2026-09-29 · libro 2.10.4 + cyrius 6.6.10: the thin sigil matches the fold again

No bote source change beyond the `initialize` version literal.

### Fixed — 5 arity errors at cyrius 6.6.10

bote declares the `sigil` stdlib fold (`lib/sigil.cyr`) and also receives libro's thin sigil
selection (`lib/sigil-mldsa.cyr` + `lib/sigil_{sha_ni,sha256,hex}.cyr`) through libro's
`deps.sigil` block. Both land in one unit, which is benign only while the two sigil versions
are equal (the CLAUDE.md stack-table rule). libro 2.10.3 pins sigil 3.12.18; cyrius 6.6.10
folds 3.13.4, and 3.13.x changed the arity of five ML-DSA sponge helpers. So a clean resolve
of 3.3.13 with the pin moved to 6.6.10 did not build:

```
lib/sigil-mldsa.cyr:3780-4194 against lib/sigil.cyr
  _mldsa_shake_squeeze_blocks 3 vs 4, _mldsa_sv_init 0 vs 1, _mldsa_sv_absorb 2 vs 3,
  _mldsa_sv_finalize 0 vs 1, _mldsa_sv_squeeze 2 vs 3
```

libro 2.10.4 moves its sigil pins to 3.13.4. With `deps.libro` at 2.10.4 and the toolchain at
the released 6.6.10, the build is OK (2,743,112 B) with no arity error. The duplicate-fn
baseline is now **236**: all of them are the thin 3.13.4 set against the fold, which is a strict
subset of it with no arity mismatch (232 at 3.12.18).

### Changed

- **Toolchain `6.6.6` → `6.6.10`**, **libro `2.10.3` → `2.10.4`** (a pin-only libro release;
  `struct chain`, `struct entry` and `struct error` are unchanged, per the struct-layout
  contract). majra stays at 2.9.1.
- patra now arrives as the 6.6.10 fold's **1.15.1**. libro still pins 1.14.3, and
  `cyrius deps` keeps the snapshot's copy (`refusing to overwrite stdlib leaf 'patra'`).
- `dist/bote.cyr` and `dist/bote-core.cyr` regenerated. `dist/bote-core.deps` drops
  `tagged`, `fmt` and `io` (8 leaves), which is cyrius 6.6.10's `distlib`;
  `dist/bote.deps` is unchanged (32 leaves).
- `cyrius.lock` is the tag-mode lock a clean resolve writes: libro, majra, sigil and patra
  are commit-pinned. All four were resolved from their tags with the `path = "../X"` lines
  commented out, since the majra checkout is at 2.9.2, one past the pin.

### Added — CI checks libro's thin sigil against the fold

A new `ci.yml` step compares the `# Version:` headers of `lib/sigil-mldsa.cyr` and
`lib/sigil.cyr` and fails when they differ. A version drift with no arity change would compile
silently; this one only surfaced because 3.13.x happened to change arities. The step passes
when no thin copy is vendored. Verified to fail on a copy pinned to libro 2.10.3.

### Added — CI checks the docs' fold versions against `lib/`

The first cut of this release bumped the libro pin in the docs but left
`docs/architecture/overview.md` naming sigil 3.12.18, sakshi 2.5.2, patra 1.14.3 and bayan
1.5.6 as the fold versions, the CLAUDE.md sakshi row at 2.5.2, and the
`docs/spec-compliance.md` header at 3.3.13 (cyrius 6.6.6). All now name what `lib/` holds:
sigil 3.13.4, sakshi 2.5.5, patra 1.15.1, bayan 1.5.8; the compliance header reads 3.3.14
(cyrius 6.6.10) and keeps its 2026-09-22 audit date, marked as taken at 3.3.13. Two CI checks
keep them there: a Build & Test step compares every `<leaf> X.Y.Z` in the overview's fold
sentence and in CLAUDE.md's sigil / sakshi rows with that leaf's `lib/<leaf>.cyr` header,
and the Documentation job's version check now requires the compliance header to name
`VERSION`. Both were verified to fail on the uncorrected docs.

## [3.3.13] — 2026-09-22 · three conformance repairs the documentation sweep turned up

`ping` answers, `2025-06-18` is accepted, and `src/sandbox.cyr` reaches consumers for the
first time since it was written. All three were found by the 3.3.12 documentation sweep
(which ships in this release, below) — none by a test, which is the part worth noting: two
of them have new CI gates so the class cannot recur. **902** assertions, natively and under
`qemu-aarch64`.

### Fixed — `ping` answered `-32601 method not found`

Every MCP revision bote supports requires the receiver of a `ping` to respond promptly with
an empty result; bote had no route for it, so it fell through to the unknown-method tail.
SDK clients send `ping` as a keepalive and read an error as an unhealthy server — a client
may tear the session down on it. Now `{"result":{}}`, verified over stdio and over HTTP.

Placed ahead of every capability gate and taking no params, because a ping has to work on a
bare dispatcher with no registries wired — the state a client probes at startup; the suite
asserts exactly that case separately. It emits **no** audit or event record: a keepalive
every few seconds is liveness, not activity, and would swamp the libro chain. bote answers
pings; it does not originate them (server→client pings need the push path, roadmap 3.5.x).

⚠ The test for this is **guarded rather than chained**: on a `-32601` response
`resp_result` is 0 and `streq(0, "{}")` SIGSEGVs, which took the ~500 assertions *below* it
down with it and reported a crash instead of a diagnosis. Measured on the mutant with the
route deleted — before the guard it was one FAIL and a signal 11; after, three named
failures and a complete run.

### Fixed — protocol version `2025-06-18` was rejected, and the version list was in two places

`2025-06-18` is a published MCP revision. bote listed three versions and not that one, and
the effect was **asymmetric in a way neither side could see**: over stdio an `initialize`
naming it was silently negotiated up to `2025-11-25`, while over the HTTP family its
`MCP-Protocol-Version` header was a hard **400** — a client pinned to that revision could
not connect at all.

The root cause is why this is more than a one-line fix: the supported-version list existed
**twice** — `_mcp_is_supported` (`src/dispatch.cyr`, the negotiator) and
`validate_protocol_version` (`src/session.cyr`, what the HTTP-family transports run the
header through). Both happened to name the same three versions, so the duplication was
invisible until a fourth was needed. `validate_protocol_version` now defers to
`_mcp_is_supported`; there is one list, and adding a version is one line. dispatch.cyr
precedes session.cyr in every compile unit that has both (all three binaries, the test
files, `dist/bote.cyr`), and `[lib.core]` carries dispatch without session, so the
single-pass forward reference always resolves.

Four new assertions pin the two readers against each other version by version, so they
cannot drift again. Mutation-proven both ways: dropping `2025-06-18` from the one list
fails 3, and restoring session's private copy fails 2 — including the agreement row, which
is the one that would have caught the original split.

`2025-06-18` removed JSON-RPC batching from the spec. bote keeps accepting batch arrays on
every version: a superset is harmless to a client that never sends one, and refusing them
per-version would break the 2024/2025-03 clients that legitimately batch.

### Fixed — `src/sandbox.cyr` shipped in no bundle and no binary

The kavach-shaped runner adapter has been in `src/`, tested (13 assertions) and advertised
in the README and `[package].description` since 2.1.0 — and in neither `[lib]` profile, so
**no consumer could reach a single `sandbox_*` symbol**. The same orphan shape `jwt.cyr` and
`pkce.cyr` had until 3.2.0, which makes this the second occurrence of the class, not the
first.

It joins **`dist/bote.cyr`** (30 → 31 modules), placed after `dispatch.cyr` because its noop
runner calls `_json_emit_escaped`. The sidecars are byte-identical: it adds no stdlib leaf.

**Not in `[lib.core]`, decided rather than defaulted.** It would fit mechanically — no
sigil, no transport, no new leaf — and the profile's audience (consumers that wrap the
Dispatcher and run handlers) is plausibly who wants a sandbox slot. Against that: the core
profile exists to *bound* a consumer's compile set, **eight** repos vendor
`dist/bote-core.cyr`, and **zero** repos in the workspace reference any `sandbox_*` symbol.
`content.cyr` earned its core place at 3.3.6 on evidence — core consumers were hand-rolling
content envelopes and re-implementing JSON escaping each time — and there is no equivalent
here. Adding it on the argument that it *might* be wanted is the speculative direction.
Revisit when a core-profile consumer actually wires a backend. The reasoning lives in
`DEPS-PATTERN.md`, which is the distribution contract; `cyrius.cyml` carries only the
standing membership rule, because a manifest is not a ledger.

Its header comment also drops the stale `kavach 3.12.2` pin (kavach is not a declared dep;
CLAUDE.md's stack table tracks what it is checked against) and gains the one real mismatch a
consumer adapter hits: kavach's `sandbox_exec(sandbox, command)` is 2-arg with no timeout,
so an adapter drops bote's `timeout_ms`.

### Added — a CI gate for the orphan class

The manifest-completeness gate only proved that main.cyr's includes are in `[lib]`; it was
blind to a module in **neither**, which is exactly how `sandbox.cyr` sat unreachable from
2.1.0 to 3.3.12 — thirteen minor lines, and it was never in the manifest at any point in
that span (checked with `git log -S` over `cyrius.cyml`, not assumed). CI now also asserts that every `src/*.cyr` is in some `[lib*]` profile or is
one of four declared entry points, and — the reverse — that no profile names a file that
does not exist. Mutation-checked against the 3.3.12 manifest: it reports `src/sandbox.cyr`
and exits 1.

### Verified

- **902 passed, 0 failed** natively and under `qemu-aarch64` (887 → 902: +15 in
  `tests/bote.tcyr`); zero `undefined function` on any unit, either target.
- agnos: three entries + nineteen units, 0 undefined, `ET_EXEC` x86-64.
- **Clean-room consumer**, native and `--agnos`: a project declaring only `[deps.bote]` →
  `dist/bote.cyr` now builds a probe that calls `sandbox_runner_noop_new` / `sandbox_run`
  (including the null-runner error envelope), answers a `ping` through the bundle's
  dispatcher, and checks `2025-06-18` through both version readers — 0 undefined, exit 0.
  Every one of those calls was unresolvable against the 3.3.12 bundle.
- HTTP header path re-probed per version: `2024-11-05` / `2025-03-26` / `2025-06-18` /
  `2025-11-25` → 200, an unknown version → 400.
- Fuzz ×4 clean; `fmt` / `lint` / `vet` / `deny` clean; `src/` warning-free; raw-`syscall(`
  gate clean; `cyrius distlib --check` current on both profiles.
- Six-transport `tools/call` round trip; `initialize` reports **3.3.13**.

### Performance

None claimed. The logged row moves within the noise band established by the 3.3.10 ↔ 3.3.11
interleave (±5% on the sub-microsecond rows): six rows down, seven up, largest +2.6%
(`schema_compile_nested`), none touching changed code. `ping` adds one `streq` to the
dispatch chain ahead of `tools/call`, whose +0.9% is inside that band.

### Changed — documentation sweep

Stale information cleaned out of every living document, and the roadmap rebuilt as a
forward-facing plan. Done as its own pass before the fixes above — the three defects are
what it found. The CHANGELOG and `docs/development/issues/archive/` are records and were not
touched beyond this entry.

- **`docs/development/roadmap.md` is forward-facing only.** The 53-row "Shipped" table,
  the 2.6.x modernization-arc table, the closed "Blocked on cyrius" audit and the
  all-✅ cyrius-issue status table are gone — every row paraphrased a CHANGELOG entry.
  What remains is organised by where it lands: **3.3.13** (three conformance repairs),
  **3.4.x** (binary reconsolidation, the conformance-suite port, the two Low timeout
  items), **3.5.x** (threaded dispatch in seven bites, with what each unlocks),
  **3.6.x** (WebSocket subprotocol / deflate and DNS-aware SSRF, each with the upstream
  seam that has to be *filed*, not waited for), open decisions, housekeeping, a watch
  list, and the 4.0 criteria. Every library-surface claim in it was verified against the
  vendored `lib/` (`sandhi_resolve_ipv4`, sankoch's `deflate_*`, `arena_new_growable` /
  `arena_reset`, `thread_local_alloc`, the unused `resources_list_changed_notification`).
- **`docs/spec-compliance.md` re-audited against the built binary**, not the previous
  table — it had been anchored at bote 1.9.2 / cyrius 4.7.0 (519 assertions, SID from
  `/dev/urandom`, annotations "planned for 2.0", threaded dispatch "waits on cyrius").
  Now 3.3.12: JWT / PKCE rows added, the polled push described, the coverage table at
  887, and the gaps below recorded as ❌ rather than omitted.
- **README**: the versioning section no longer names a version (it pointed at 3.2.1);
  the benchmark table is the 3.3.12 `history.log` row with a note that a row is an
  illustration and the interleaved A/Bs in this file are the measurement; the docs table
  describes what each document is now; the sandbox row and `ping` / `2025-06-18` are
  stated honestly.
- **CLAUDE.md**: the pin line and the version line had become ledgers (every hop since
  2.7.3, every release headline since 3.3.4) and are condensed to the current state with
  pointers; the duplicated 6.1.x/6.2.x toolchain-syntax parentheticals in the dev loop,
  the 5.10.x compile-cap history and the 3.3.3 → 3.3.4 capacity-delta narrative are
  reduced to their rules; `sandbox.cyr` is listed as what it is.
- **`docs/architecture/overview.md`**: the HTTP section described a hand-rolled
  accept-loop server that 2.6.0 replaced with sandhi (`sandhi_server_run`, 30 s default
  idle timeout, WS replaces it after the handshake); design principle 4 no longer waits
  on a cyrius surface that shipped; counts, kavach pin and the docs tree re-anchored.
- **`DEPS-PATTERN.md`**: the sidecar paragraph described the 3.2.0 shape ("three names,
  partial hint"); since cyrius 6.6.6 it names exactly the declared 31 + `ws_server` and
  `cyrius deps` pulls the include-closure itself (the 3.3.10 clean-room measurement).
- **`SECURITY.md`** support table 3.1.x/3.0.x → 3.3.x/3.2.x. **`CONTRIBUTING.md`**: no
  pinned toolchain number, siblings are `libro` + `majra` only, the sakshi-logging advice
  replaced (bote references no sakshi symbol; audit and events go through the sinks), the
  raw-`syscall(` rule added. **`docs/benchmarks-rust-v-cyrius.md`** and
  **`docs/cyrius-feedback.md`** re-labelled as dated historical snapshots (they read as
  current). `scripts/build-all.sh` no longer promises a 5.11.x that never existed.

### Found by the sweep

Probing the released 3.3.12 binary while re-auditing the compliance matrix turned up four
things no test covered. Three are **fixed above**: `ping`, `2025-06-18`, and the orphaned
`src/sandbox.cyr`. The fourth is carried:

- `resources/templates/list` answers `-32601`. Optional in the spec, but some clients call
  it whenever `resources` is advertised and read the error as a failure rather than "no
  templates"; an empty `{"resourceTemplates":[]}` is the honest reply. Left for a patch
  that can weigh it against a real template registry — roadmap.

⚠ **None of the three was found by a test**, which is the finding behind the finding. The
44-scenario conformance suite in the Rust archive (tag `0.92.0`) was never ported; `ping`
would not have survived it. That port is now a 3.4.x roadmap item, and two of the three
fixes above ship with a gate (the cross-reader version agreement, the orphan-module check)
rather than only an assertion.

## [3.3.12] — 2026-09-22 · the agnos target is gated — and building for it found two things Linux could not show

`cyrius build --agnos` has compiled bote since 2.7.8, and 2.9.0 proved a live MCP flow on the
real agnos kernel under QEMU — but no gate ever built for the target, so nothing could see it
rot. Now CI does, and the sweep that established the baseline turned up an include gap five
test files had carried since the codec / schema split, and a dependency's errno constants
silently standing in for the stdlib's on agnos. **887** assertions natively and under
`qemu-aarch64`; every entry and every unit clean for `CYRIUS_TARGET_AGNOS`.

### Added — agnos portability gate in CI

Two compile-time gates, modelled on the toolchain's own `scripts/agnos-crossbuild-gate.sh`.
An agnos binary cannot run on the runner; behaviour is validated on the agnos QEMU image by
the kernel-side `BOTE_SELFTEST` hook + `bote-mcp-smoke.sh` in the agnos repo, not here.

1. **All three entries** cross-build to a valid agnos ring-3 binary — a static x86-64
   `ET_EXEC` ELF (magic `7f454c46`, `e_type` 0x0002, `e_machine` 0x003e) — with **zero**
   `undefined function` lines, reachable or not. "OK" alone is not the bar: cycc lowers an
   unreachable undefined function to a `ud2` stub and still exits 0, and the day a call site
   becomes reachable that is a SIGILL. Measured at this release: `bote-agnos` 2,664,888 B,
   `bote-streamable-agnos` 2,625,264 B, `bote-ws-agnos` 2,629,544 B, 0 undefined each.
2. **Every test, bench and fuzz unit** compiles for the target with the same zero-undefined
   bar — 14 `.tcyr` + `bote.bcyr` + 4 `.fcyr`, compile-only. Before this release one of the
   nineteen did not build for agnos at all (below).

Also proven once, outside CI: the **clean-room consumer** probe from 3.3.10 (a project
declaring only `[deps.bote]` → `dist/bote.cyr`) builds for `--agnos` from the sidecar alone —
`ET_EXEC` x86-64, 0 undefined, reaching the registry, dispatcher, majra hub, libro chain and
content blocks.

### Fixed — five test files compiled with an undefined function, and passed

`bote_content.tcyr` and `bote_sandbox.tcyr` included `src/dispatch.cyr` without `codec.cyr`
or `schema.cyr`, so `dispatch`'s call to `compiled_validate` was undefined;
`bote_fs_tools.tcyr`, `bote_libro_tools.tcyr` and `bote_web_tools.tcyr` included `schema.cyr`
without `codec.cyr`, so `schema`'s `_cdc_skip_ws` was. All five linked with a `warning:` and
passed every assertion, because nothing in those files reaches the gap — the exact shape
3.3.8 found in the fuzz harnesses ("compiled with 18 undefined functions and passed only
because every call site was unreachable"). The fuzz step has gated it since; the test step
never did. Not agnos-specific — the native build carries the same five warnings — but the
agnos sweep was the first thing to read every unit's build log. Each file now includes
`dispatch → codec → schema` in `src/main.cyr` order.

### Fixed — the CI test step could not fail on a test that failed to run

`out=$(cyrius test "$tcyr" 2>&1)` discarded the exit code, and the only check was a grep for
an explicit `N passed, M failed` with `M ≠ 0`. A file that **failed to compile**, or
**SIGSEGV'd before printing its summary**, produced no such line and passed the step with an
empty `tail -3`. Same class as cyrius 6.6.6's own runner repair ("a binary that ran nothing
scored a PASS"). The step now fails on a non-zero exit, requires a `[1-9]… passed, 0 failed`
line from every file that calls `assert_summary` (the core-only smoke is exit-code driven by
design and exempt by that same test), and fails on any `undefined function`, as the fuzz step
does. **Mutation-checked** with three scratch units: a syntax error → `FAIL (exit 1)` + no
summary; a `sys_exit(0)` before the summary → `FAIL (no passing summary line)`; a copy of
`bote_content.tcyr` with the two includes removed → `FAIL (undefined: compiled_validate)`. The
pre-3.3.12 logic, run on the second mutant: PASS.

### Fixed — on agnos, `transport_unix.cyr`'s errno names bound to sigil's, not the stdlib's

`_unix_accept_action` classified accept(2) errnos with five bare stdlib names — `EINTR`,
`EAGAIN`, `EINVAL`, `EMFILE`, `ENFILE` — and seven of its own `BOTE_ERRNO_*` for the ones the
Linux peer's `Errno` enum stops short of. The agnos syscall peer defines **no** errno names at
all. The build still resolved four of the five, because the `[deps] stdlib` prelude that
precedes every unit carries `lib/sigil.cyr`, whose file-scope enum declares bare `EINTR = 4`
… `EMFILE = 24` (`lib/sigil.cyr:44-58`) — so on agnos bote's accept policy was reading a
**dependency's** constants, correct only because sigil happens to spell Linux's numbers.
`ENFILE` is not in sigil's enum, which is how `bote_transport_unix.tcyr` failed to build for
`--agnos` (`undefined variable 'ENFILE'`) and how the leak was found. This is the flat
global namespace `CLAUDE.md` already warns about, from the other direction: not bote's
constant aliasing a dep's, but a dep's constant standing in where the target's own is absent.

The file now spells all twelve names it classifies — `BOTE_ERRNO_EINTR` / `EAGAIN` / `EINVAL`
/ `ENFILE` / `EMFILE` join the existing seven — and the comment says why. The values are the
Linux generic errno numbers, identical on x86_64 and aarch64, which is the only place the
accept loop runs: `transport_unix_run` fail-closes on agnos before any socket call (2.7.8).
The test uses the same names and guards its `sys_accept4` probe with `#ifndef
CYRIUS_TARGET_AGNOS` — the agnos peer has no `accept4` wrapper because the target has no
`AF_UNIX` — exactly as the loop it exercises is guarded. Everything else in the file
(`_unix_sockaddr`, the policy table, the backoff curve, `sleep_ms`) compiles and is asserted
on every target. Native and aarch64: 47 / 47, unchanged.

Sweep result: those three were the **only** bare errno names in `src/`; no other module is
exposed.

### Verified

- **887 passed, 0 failed** natively and **887 / 887 under `qemu-aarch64`**; zero
  `undefined function` on any unit, either target.
- agnos: three entries + nineteen units, 0 undefined, `ET_EXEC` x86-64; consumer probe likewise.
- All four fuzz harnesses clean; `fmt` / `lint` / `vet` / `deny` clean; `src/` warning-free;
  `cyrius distlib --check` current on both profiles.
- Six-transport `tools/call` round trip on the rebuilt binaries; `initialize` reports **3.3.12**.

### Performance

None claimed; no benchmark reaches the changed code. Single-run row logged: every figure
within the 3.3.10 ↔ 3.3.11 interleaved band, with `dispatch_initialize` back at 1.32 µs after
3.3.11's 1.44 µs single-run outlier.

### Docs

`CLAUDE.md` gains the agnos gate in the CI/testing notes and the sigil-errno case as the
worked example for the "dependency constants standing in" direction of the namespace rule;
`README.md` states the target support; roadmap re-anchored.

_(empty)_

## [3.3.11] — 2026-09-22 · no raw syscalls left, and the two resolved issues finally archived

No `src/` change beyond a comment path and the version literal. **887** assertions across 14
suites, natively **and under `qemu-aarch64` — the first complete emulated sweep** (every
earlier one was partial); all four fuzz harnesses; a six-transport round trip.

### Changed — every kernel touch in bote's own sources goes through a stdlib wrapper

`src/` was already clean — 3.2.1 routed its last eight sites — so the 26 that remained were
all the same shape, `syscall(SYS_EXIT, n)`, in the test files (15), the benchmark (1) and the
fuzz harnesses (10). Each is now `sys_exit(n)`. The same class libro cleared at 2.10.3.

Why it is worth a release line when `SYS_EXIT` is defined on every syscall peer: the constant
was never the hazard, the **form** is. A numeric `syscall(60, rc)` — 60 is `exit` on x86_64
only, and it is the spelling the toolchain's own cross-OS tests carry — compiles on every
target and exits through whatever 60 means there. `sys_exit` spells the call per target
(`lib/syscalls_linux_common.cyr`, the agnos and Windows peers), and a wrapper call is what a
reader can grep for the meaning of.

- **A failing suite still fails.** Proven rather than assumed, because a wrong exit path would
  have turned every test file green at once: a scratch `.tcyr` with `assert_eq(1, 2)` ending in
  `sys_exit(assert_summary())` exits **1** as a binary and is reported `FAIL … (exit 1)` by
  `cyrius test`.
- **The CI gate now bans the form everywhere.** 3.2.1's gate matched `syscall(SYS_*` in `src/`
  only, so it could not see a numeric syscall and did not look at tests, the bench or fuzz at
  all. It now fails on any `syscall(` across `src/*.cyr`, `tests/*.tcyr`, `tests/*.bcyr` and
  `fuzz/*.fcyr`, comments stripped first. Mutation-checked: a planted `syscall(60, 0)` in a test
  file is reported with `file:line`; the same text inside a comment stays clean. The x86-only
  `SYS_*` constant denylist over `src/` + `dist/` is kept as the second, independent axis.
- **The adjacent class was inspected and deliberately left alone.** Wrapper calls that pass
  literals — `sys_chmod(path, 0x180)`, `sys_mkdir(full, 0x1ED)`, `sys_listen(sfd, 16)`,
  `sys_read(0, …)` / `sys_write(1, …)` — are POSIX mode bits, a listen backlog and the standard
  fds, identical on every target; `AF_UNIX` / `SOCK_STREAM` are already the named constants.
  Nothing to rename.

### Changed — both open issues re-verified on the current tree and archived

`docs/development/issues/` had carried two files marked ✅ RESOLVED since 3.2.0 / 3.2.1 that
nobody moved. Each was re-verified against 3.3.11 (cyrius 6.6.6, libro 2.10.3, majra 2.9.1)
before archiving, and each gained a dated closure banner recording what was re-checked. Every
reference — `ci.yml`, `src/jwt.cyr`, the roadmap, the 3.2.0 CHANGELOG history — now points at
`issues/archive/`, the convention the 2026-06 issues already used.

- **`2026-07-17-aarch64-sys-open-urandom`** — zero x86-only `SYS_*` in `src/` or `dist/`
  outside comments; both entropy sites on `random_bytes()`, the accept loop on `sys_accept4`;
  all three entries cross-build to `EM_AARCH64`. ⭐ And the runtime sweep is **complete**:
  `cyrius test --aarch64` runs all 14 files under `qemu-aarch64` 11.1.1 — **887 / 887** — where
  `CLAUDE.md` recorded it as PARTIAL (three files exiting 90 on the emulator's missing
  `getrandom` passthrough, 373 of 883 executing). Between that note and this one the toolchain
  moved 6.5.35 → 6.6.6 (6.6.5 added sixteen aarch64 `ESYSXLAT` rows) and the emulator 11.1.0 →
  11.1.1; which of the two closed it is not established here. Measured, not attributed.
- **`2026-07-30-jwt-module-is-orphaned…`** — `_jwt_exp_ok` still runs only after `_jwt_ct_eq`
  short-circuits; `_jwt_str_field_eq` still reads `alg` as an exact field; `jwt_verify_hs256`
  and `pkce_code_verifier` are in `dist/bote.cyr` and absent from `dist/bote-core.cyr`; the
  alg-confusion and `exp` assertions pass on both arches. The symbol-collision sweep was
  **re-run, not reused**: 3.2.0 checked 71 `lib/` files, the graph has since gained libro
  2.10.3, majra 2.9.1, sigil 3.12.18's thin modules and the 6.6.6 fold, and `jwt_*` / `pkce_*`
  / `_jwt*` / `_pkce*` have zero definitions across all **117** `lib/**/*.cyr`. Finding (3) was
  a corrected premise, not a defect; its open **decision** stays in the roadmap, where the row
  gains a second expired premise — sigil 3.12.18 also exposes `ecdsa_p256_verify` /
  `ecdsa_p256_verify_der`, so ES256 is unblocked exactly as RS256 is. Still not built, for the
  reasons the row already gives: the one consumer that asked implements it locally because it
  needs `iss` / `aud` validation bote has no concept of.

`docs/development/issues/` holds only `archive/` for the first time since 2026-07-17.

### Verified

- **887 passed, 0 failed** natively and **887 / 887 under `qemu-aarch64`**, per file.
- All four `fuzz/*.fcyr` clean, no undefined functions; the benchmark builds and runs.
- `cyrius fmt --check` over `src/`: 0; `lint`: 0 warnings; `vet`: 29 deps, 0 untrusted; `deny`:
  0 violations; `src/` warning-free; `cyrius distlib --check`: both profiles current.
- One `tools/call` → `bote_echo` round trip over stdio, HTTP, Unix socket, bridge, Streamable
  HTTP and WebSocket on the rebuilt binaries; `initialize` reports **3.3.11** on all three.
- Raw-syscall gate: clean on the tree, red on the mutant.

### Performance

None claimed. The 3.3.10 and 3.3.11 bench binaries interleaved, three trials each, medians:
twelve of fourteen rows within noise; `dispatch_tools_call` **+5.4%** and
`codec_serialize_response` **+4.4%** with non-overlapping ranges, on sources that are
byte-identical apart from the version literal. Isolated rather than waved off: a third build of
the 3.3.11 tree with *only* the bench file's exit call reverted matches 3.3.11 to −0.2% / 0.0%,
so the `sys_exit` change is not the cause. What does differ is the version literal's length —
the 3.3.10 bench binary was built minutes before its own bump and carries `3.3.9`, five
characters against six — a one-byte string-data shift that moves every following data address
(2,539 differing bytes, all +1 relocations). Alignment, not code. The single-run `history.log`
row also shows `dispatch_initialize` +10%, which the interleaved medians put at −1.2%; that is
why the log is a history and the interleave is the measurement.

_(empty)_

## [3.3.10] — 2026-09-21 · cyrius 6.6.6 + libro 2.10.3 / majra 2.9.1 — and the manifest stops being a changelog

Toolchain **6.6.3 → 6.6.6** and both dependency pins to their latest tags. No source
change beyond the version literal. **887** assertions across 14 suites, all four fuzz
harnesses, the benchmark, and a clean-room consumer probe against the regenerated bundle.

### Changed — toolchain and dependency pins

| dep | was | now | arrives via |
|---|---|---|---|
| cyrius | 6.6.3 | **6.6.6** | `cyrius =` pin |
| libro | 2.10.0 | **2.10.3** | `[deps.libro]` |
| majra | 2.7.2 | **2.9.1** | `[deps.majra]` |
| sigil | 3.12.17 | **3.12.18** | stdlib fold |
| patra | 1.14.2 | **1.14.3** | stdlib fold (libro's `deps.patra` pins the same 1.14.3; `cyrius deps` keeps the snapshot's copy and says so) |
| sakshi / bayan / sandhi | 2.5.2 / 1.5.6 / 1.9.17 | unchanged | stdlib fold |

**The toolchain and libro pins move together or not at all.** libro 2.10.2 needs the
`O_NOFOLLOW` / `O_DIRECTORY` symbols cyrius 6.6.4 introduced (libro's own CHANGELOG measured
six `undefined variable` errors on a 2.10.1 manifest beside a 1.14.3 patra), so 2.10.3 on the
old 6.6.3 pin is not a valid intermediate state.

What the pre-flight checked against 6.6.4–6.6.6's consumer-visible changes, all empty in
`src/`: no `O_APPEND` / `O_TRUNC` outside vendored `lib/` (the single write is
`file_write_all` at `src/fs_tools.cyr`) and no Windows target, so the PE open-flag repair does
not reach here; no `async fn`, `operator` fn, `ret2` / `rethi`, SIMD intrinsics, struct
declarations, struct- or vector-typed parameters or `var`s (6.6.5's struct-local operator
rule), top-level bare `{` blocks (6.6.6's block-scope rule), duplicated global declarations,
`regression_*` call sites, or `vec_*` names colliding with `lib/vec.cyr`'s exports; no reach
across a `private` boundary (6.6.4's visibility gates — the build would have refused). Arity
and `: cstring` scans clean. Both bump notes that demand action were followed: `cyrius deps`
re-run after the pin move (6.6.5 — the aarch64 `SYS_UNLINKAT` renumbering), and `lib/` rebuilt
from the snapshot first (`rm -rf lib && cyrius lib sync --full && cyrius deps`).

What the dependency bumps were checked against, unchanged across both:

- **libro** `struct chain` / `struct entry` / `struct error` are byte-identical between 2.10.0
  and 2.10.3, so `src/libro_tools.cyr`'s raw offsets (entry 16 / 24 / 32 / 40 / 56 / 72, chain
  `entries` at +0) and the `#derive(accessors)` reads of `struct error` still hold. libro
  2.10.1–2.10.3 are the TPM-arm `Result` fix, the syscall-wrapper sweep and the pin bump — "no
  format change: every digest, proof and signature verifies exactly as under 2.10.1".
- **majra** `pubsub_new()` / `pubsub_subscribe(ps, topic)` / `pubsub_publish(ps, topic,
  payload)` keep their signatures and the 32-byte hub. 2.9.0's two **wire breaks** (encrypted
  IPC handshake, signed-envelope domain separation) live in the `-backends` / `-signed`
  profiles bote does not consume; 2.8.0's `ERR_*` → `MAJRA_ERR_*` rename keeps same-value
  aliases and could not collide with bote's `BOTE_ERR_*` anyway; 2.8.1's borrowed-key fix is
  on the hub bote publishes into and is the one behavioural change worth having.

### Changed — `cyrius.cyml` is a manifest again, 300 → 174 lines

The comment blocks had become a changelog: the libro_tools parking story through 2.6.x, the
sakshi and sigil pin-removal write-ups from 3.3.1 (with their four-step `cyrius build`
reversion transcript), the 3.2.0 tag↔lock realignment with its dist hash, the 2.8.4 / 2.8.11
struct incidents, the bayan / bigint retirement notes, a kavach version in the package
description, and "stable 2.x" in the same line two majors on. Every one of those facts is
already recorded in this file (3.3.1, 3.3.4, 3.2.0, 2.7.x) or in `CLAUDE.md`'s stack table —
grep-verified before cutting. What remains states the present rules only:

- why the `[lib]` / `[lib.core]` order matters and why `content.cyr` is last in core;
- the three load-bearing orderings in `[deps].stdlib` (`dynlib` → `fdlopen`, `thread` →
  `thread_local` → `sigil`, `ct` / `keccak` / `random` → `sigil`), why `sakshi` is listed and
  why `ws_server` is not;
- the rule against a `deps.sigil` / `deps.sakshi` block over a folded module;
- `path` beats `tag`, and transitive versions are verified after `cyrius build`;
- the libro struct-layout contract (appended fields are safe for a raw reader, prepended ones
  are not) as a rule, not a re-telling.

Two hygiene rules come over from libro 2.10.2's own cleanup: no comments inside an array,
and `deps.NAME` in prose rather than the bracketed header (distlib's sidecar scan is
unanchored). The comments that sat inside the `stdlib = [...]` array moved above it.
Resolution is proven unchanged: `cyrius lib sync --dry-run` before and after the rewrite
selects the same 59 files, plus `alloc_cx.cyr`, the one file 6.6.6 added to the snapshot.

### Changed — `dist/bote.deps` names 32 leaves, not 41

Nine transitive leaves came out of the full-profile sidecar: `hashseed`, `sha1`,
`syscalls_linux_common`, `result`, `boxed`, `mmap`, `process`, `fs`, `tls_native`.
`dist/bote-core.deps` is unchanged at 12.

This is 6.6.6's `distlib`, not the manifest rewrite — isolated by regenerating from the *old*
manifest text with only the pins edited, under 6.6.6: byte-identical sidecar. The dropped
names are exactly the include-closure of the declared modules, which the 6.6.x stdlib now
pulls itself ("`lib/string.cyr` and `lib/fmt.cyr` are self-sufficient now — they include what
they call", per 6.6.6; `lib/io.cyr` likewise).

Proven safe the only way that counts — a **clean-room consumer**: a scratch project declaring
`stdlib = ["assert"]` and `[deps.bote]` → `dist/bote.cyr`, nothing else. `cyrius deps` copied
**81** files, all nine dropped leaves among them; a probe reaching `tool_registry_new`,
`dispatcher_new`, `pubsub_new`, `chain_new` and `content_text_response` built with zero
undefined functions and exited 0.

### Fixed — the thin-sigil duplicates were not the same sigil for two releases

`cyrius build` on `src/main.cyr` reports **232** `duplicate fn` warnings from libro's thin
sigil selection landing beside the fold: `lib/sigil-mldsa.cyr` (201), `lib/sigil_sha256.cyr`
(15), `lib/sigil_sha_ni.cyr` (8) and `lib/sigil_hex.cyr` (8), each "first defined in
`lib/sigil.cyr`", so the **thin copy wins**. 3.3.5 recorded this class as benign on the
premise *"both files are the same sigil version and the bodies are byte-identical"*.

⚠ **That premise was false at 3.3.8 and 3.3.9, and nothing re-checked it.** 3.3.8 moved libro
to 2.10.0, whose `deps.sigil` pinned the thin surface at **3.12.9**, while the 6.6.2 / 6.6.3
folds carried **3.12.16 / 3.12.17** — so for those 232 functions (ed25519, ML-DSA, hybrid,
sha512, crypto_scratch, sha256, SHA-NI dispatch, hex) bote linked the older body and the
diagnostic said nothing new.

Benign in effect, and diff-proven rather than assumed: `git diff 3.12.9 3.12.18` in sigil
over `dist/sigil-mldsa.cyr` is a reindent plus the version banner (`-w` leaves one line: the
banner); `src/sha256.cyr`, `src/sha_ni.cyr` and `src/hex.cyr` are byte-identical across the
range. This release aligns both sides at **3.12.18** (libro 2.10.3's pin = the 6.6.6 fold),
verified by extracting every one of the 232 duplicated bodies from the thin file and from
`lib/sigil.cyr`: 232 of 232 identical.

**The rule that replaces the premise:** on every libro or toolchain bump, compare libro's
`deps.sigil` tag with `# Version:` in the fold's `lib/sigil.cyr`. They agree today; the day
they do not, the thin copy is what bote's crypto runs.

Also inventoried, unchanged and upstream: `duplicate fn 'uname_release'` (`lib/sys.cyr` vs
`lib/sigil.cyr`, majra tracks it) and sigil's `array local over the per-fn frame budget gets
STATIC storage` line, which 6.6.5 promoted from `note:` to `warning:` for the same storage.
`_sub_new` is gone (majra 2.7.1 renamed it) and `src/` is still warning-clean.

### Verified

- **887 passed, 0 failed** across 14 suites, per-file (`bote` 424 · `auth` 38 · `content` 24 ·
  `fs_tools` 26 · `host` 113 · `jwt` 53 · `libro_tools` 38 · `pkce` 17 · `sandbox` 13 ·
  `streamable` 53 · `transport_unix` 47 · `web_tools` 27 · `ws` 14 · core-only drift smoke).
- All four `fuzz/*.fcyr` clean, no undefined functions.
- **One `tools/call` → `bote_echo` round trip over each of the six transports** on the rebuilt
  binaries — stdio, HTTP, Unix socket, bridge (with CORS), Streamable HTTP (`initialize` →
  `MCP-Session-Id` → call) and WebSocket (RFC 6455 handshake + one masked text frame) — each
  answering `id: 7` with the echoed payload; `initialize` reports `serverInfo.version` **3.3.10**
  on all three binaries. ⚠ The first pass of that check ran against binaries built *before*
  `version-bump.sh` rewrote the literal and reported 3.3.9 — the same drift 3.3.1 shipped. Rebuild
  after the bump, then check the handshake; the script cannot do the rebuild for you.
- `cyrius fmt --check` over `src/`: 0; `cyrius lint src/main.cyr`: 0 warnings, 0 untracked
  deferrals; `cyrius vet`: 29 deps, 0 untrusted; `cyrius deny`: 0 violations.
- Capacity on `src/main.cyr` at 6.6.6: `fn_table 5708 / 131072`, `identifiers 184274 /
  8388608`, `var_table 2763 / 1048576` — 4.4% / 2.2% / 0.3% (the 6.6.x ceilings; not
  comparable with the 6.5.x percentages in `CLAUDE.md`'s history).
- `cyrius.lock`: **117** hashes + the `cyrius<TAB>6.6.6` trailer (was 116, no trailer).
  Reproducible: two independent `rm -rf lib && lib sync --full && deps` cycles wrote
  byte-identical locks. Vendored `lib/libro.cyr` and `lib/majra.cyr` are byte-identical to
  their tags, checked against `git diff <tag> -- dist/<pkg>.cyr` in each sibling — the
  `path =` override was vendoring the same bytes the tag would.
- Both bundles differ from 3.3.9's only in the two version lines; `cyrius distlib --check`
  passes both profiles.

### Performance

None claimed. Toolchain-only A/B on this host — the same source and the same vendored deps
built once under 6.6.3 and once under 6.6.6, run interleaved, three trials each, medians
(load < 0.7):

| bench | 6.6.3 | 6.6.6 | delta |
|---|---|---|---|
| dispatch_initialize | 1.410 µs | 1.305 µs | −7.4% |
| dispatch_tools_list | 2.095 µs | 2.023 µs | −3.4% |
| dispatch_tools_call | 3.330 µs | 3.175 µs | −4.7% |
| jsonx_get_str_flat | 146 ns | 141 ns | −3.4% |
| jsonx_get_raw_nested | 347 ns | 339 ns | −2.3% |
| codec_parse_request | 1.246 µs | 1.254 µs | +0.6% |
| codec_serialize_response | 445 ns | 425 ns | −4.5% |
| codec_process_message | 5.005 µs | 4.982 µs | −0.5% |
| validate_compiled_simple | 462 ns | 474 ns | +2.6% |
| validate_compiled_nested | 2.041 µs | 2.057 µs | +0.8% |
| schema_compile_simple | 2.216 µs | 2.278 µs | +2.8% |
| schema_compile_nested | 5.213 µs | 5.284 µs | +1.4% |
| auth_bearer_check_unset | 7 ns | 7 ns | 0.0% |
| auth_bearer_check_set | 685 ns | 710 ns | +3.6% |

The dispatch family moves down and the schema family up by a few percent, several rows with
non-overlapping trial ranges — the 6.6.5 / 6.6.6 emitter work, the same shape majra recorded
at 2.9.1. A wash. ⚠ The "6.6.3 side" vendored libro 2.10.3 / majra 2.9.1, not 3.3.9's
2.10.0 / 2.7.2: `path = "../libro"` beat `tag = "2.10.0"` because the sibling checkouts sit at
their latest tags — the exact hazard the manifest now states in one line. It makes this a
cleaner *toolchain* control (none of the 14 benches reach libro or majra), not a 3.3.9
reproduction. `benches/history.log` gains its first row since 3.3.7 — 3.3.8 and 3.3.9 never
logged one.

### Docs

- `docs/development/roadmap.md`: the "Moving the cyrius pin to 6.6.6" pre-flight shipped and
  is deleted; its measured claims are recorded above, as majra did at 2.9.1.
- `CLAUDE.md` re-anchored to 3.3.10 — it had read "3.3.4 current" with libro 2.8.12 / majra
  2.7.0 through two pin bumps. The toolchain-A/B principle is corrected: the `cyrius` wrapper
  now re-execs the *pinned* compiler (`cyrius which` inside the repo resolves to
  `~/.cyrius/versions/<pin>/bin/cycc`), so a pin edit alone selects both compiler and stdlib,
  and the stdin-prelude reconstruction is no longer needed for an A/B.
- `docs/architecture/overview.md` and `README.md` version / pin / assertion-count lines
  re-anchored.

## [3.3.9] — cyrius 6.6.3 · the distlib memory cap comes out

`cyrius` 6.6.2 → **6.6.3**. No source change; this release retires a CI workaround.

### Removed — the `ulimit -v` cap on the dist gate (3.3.8 → here)

3.3.8 wrapped both `cyrius distlib` invocations in `( ulimit -v 2097152; ... )` because
distlib's leaf-validation pass allocated ~30 GB on bote's full profile — measured peak RSS
**31,214 MB** — and uncapped it OOM-killed the *runner*, which Actions reported as
`Error: The operation was canceled`, two steps before any assertion ran. The cap converted a
runner-killing OOM into a contained non-zero exit.

cyrius 6.6.3 fixed the cause: the pass re-read every stdlib leaf for every leaf, and cyrius's
arena allocator never frees, so an O(n×m) read pattern became O(n×m) *retained*. It now caches
the snapshot. Re-measured here on the full profile, **uncapped**, at the 6.6.3 pin:

| | 6.6.2 | 6.6.3 |
|---|---|---|
| peak RSS, full profile | 31,214 MB | **120 MB** |
| exit code | OOM / cancelled | **0** |

A 260× reduction, comfortably inside a standard runner's ~7 GB. Both bundles come out
byte-identical to the committed copies, which also confirms 3.3.8's finding that the bundle was
emitted *before* the failing pass.

⭐ **The `|| echo "::warning::"` and `|| true` came out with it.** They existed only because a
capped run was *expected* to fail; keeping them would have left the dist gate fail-open against a
defect that no longer has an excuse. A `cyrius distlib` failure in CI is now a real failure and
exits non-zero.

## [3.3.8] — 2026-09-10 · cyrius 6.6.2 + libro 2.10.0 / majra 2.7.2 — and four fuzz harnesses that had not compiled since 6.1.25

Toolchain **6.5.35 → 6.6.2** (the `Result` / `Option` / `Either` value form) plus both
dependency pins. **887** assertions across 14 suites, the benchmark, and — for the first
time in CI — all four fuzz harnesses.

### Changed — toolchain and dependency pins

| dep | was | now |
|---|---|---|
| cyrius | 6.5.35 | **6.6.2** |
| libro | 2.8.12 | **2.10.0** |
| majra | 2.7.0 | **2.7.2** |

⛔ **The majra bump is not housekeeping — it was a build-stopper, and bote was the
source of it for the whole ecosystem.** majra ≤ 2.7.0 defines
`_sub_new(chan, filter_fn)`; libro defines `_sub_new(pattern)`. bote declares both
bundles, so both land in one compile unit. cyrius promoted that
same-name/different-arity pair from a silent "last definition wins" warning to a
**hard error at 6.5.37**, and bote pinned 6.5.35 — one release below it — so the
defect sat latent and invisible here while every downstream consumer that pulled
majra *through* bote inherited it. It surfaced in t-ron 2.1.10, whose CI could not
produce a binary at all; t-ron and nein are only immune now because each carries an
explicit `[deps.majra] ≥ 2.7.1` declared ahead of bote's. Those overrides can be
retired against this release. majra 2.7.1 renamed the symbol `_majra_sub_new`.

### Fixed — a retired accessor in `src/transport_unix.cyr`

`_unix_handle_client` called `payload(n_r)`, the accessor cyrius **deleted at 6.6.0**.
Unlike the collision above this one was never masked by anything: it fails locally and
in CI alike the moment the toolchain is 6.6.x. Now binds both halves of the
`sock_recv` result. No other site in `src/` used it, and a sweep for the *silent*
variant — a `Result` compared against `< 0`, which can never fire once `Err`'s tag is
`1` — found none.

### Fixed — all four fuzz harnesses, broken since cyrius 6.1.25

⛔ **`fuzz/*.fcyr` was never run by CI.** There was no fuzz step. Every harness
included `lib/json.cyr`, which was carved into `lib/bayan.cyr` at cyrius **6.1.25
(2026-06-10)** — so they stopped compiling roughly four minors ago and nothing could
see it. `src/jsonx.cyr:3` has recorded the carve-out in a comment the whole time.

Two further defects the repair exposed:

- `fuzz/codec_process.fcyr` called `registry_new()`, renamed `tool_registry_new()`
  when the prompt and resource registries arrived and the bare name became ambiguous.
- Three harnesses include `src/dispatch.cyr`, which routes `prompts/*` and
  `resources/*`, without including `src/prompts.cyr` or `src/resources.cyr` — so they
  compiled with **18 undefined functions** and "passed" only because every call site
  was unreachable. A bare exit-code gate would not have caught that half.

### Fixed — CI died mid-run: `cyrius distlib` OOM-kills the runner

⛔ **The 3.3.8 push failed with `Error: The operation was canceled` — no assertion,
no diagnosis, the job simply stopped.** That is what GitHub reports when the kernel
OOM-killer takes the runner down. `cyrius distlib` on bote's **full** profile
allocates ~30 GB (measured peak RSS **31,214 MB**) against a standard runner's ~7 GB.
The step already had `cyrius distlib || true`; that never helped, because the
process was not exiting non-zero — the *runner* was being killed.

Measured, not assumed:

| probe | result |
|---|---|
| `ulimit -v 7GB`, pins 6.6.0 / 6.6.1 / 6.6.2 | dies at all three — the regression predates 6.6.0 |
| libro reverted 2.10.0 → 2.8.12 | still dies — not caused by the dependency refresh |
| caps of 2 / 3 / 4 / 6 GB | `dist/bote.cyr` emitted **byte-identical every time** |
| `distlib core` @ 2 GB | **rc=0** — completes leaf validation |
| kavach (44 modules) / sankhya (36) / hisab (35) @ 7 GB | all **complete** |

That last row is the important one: it is **not** driven by module count — three repos
with larger `[lib]` profiles finish on the same toolchain. What bote has and they do
not is **`tls_native`** in its leaf set (with `ws_server` / `sigil` / `sha1` / `sync`),
pulled in by the transport and bridge modules. bote's own `[lib.core]`, which excludes
them, finishes inside a 2 GB cap. The trigger is a specific leaf graph, not size.

Since the bundle is emitted *before* the validation pass, both workflows now run
distlib under `ulimit -v 2` GB. The OOM becomes a contained non-zero exit instead of a
dead runner, the freshness diff — the only thing the gate actually asserts — still
runs, and a `::warning::` makes the skipped validation visible rather than silent.
`release.yml` regenerates the same bundles and is capped identically. Remove both caps
once the upstream defect is fixed.

### Added — a Fuzz step in CI

Runs every `fuzz/*.fcyr` and fails on a non-zero failure count **or** on any
`undefined function` in the output. The second assertion is the load-bearing one: it
is what distinguishes a harness that passes from one that merely does not reach its
own broken code.


_(empty)_

## [3.3.7] — 2026-08-23 — a healthy WebSocket client was being dropped after 30 seconds

**887** assertions green across 14 suites, 0 failed (883 → 887).

### Fixed — WebSocket connections dropped at 30 s idle, with no close frame

bote does not own the accept loop its HTTP-family transports run on:
`sandhi_server_run` sets a **30 s `SO_RCVTIMEO`** on every accepted connection
as a slowloris guard. That is exactly right for HTTP — and it demonstrably
works: an unterminated `POST` to bote's HTTP transport is dropped at **30.3 s**,
measured.

It is wrong once the connection has been **upgraded**. A WebSocket is
long-lived and legitimately idle between messages, but the deadline follows
`cfd` into the frame loop, where `ws_server_recv_frame` maps *any* `sock_recv`
error — EAGAIN from that timeout included — to `-1`, which `ws_server_recv`
reports as "peer closed". bote only *answers* PINGs and never initiates one, so
nothing keeps the socket warm. A quiet but perfectly healthy client was
disconnected at 30 s, and not even told: no close frame.

`_bote_ws_handler` now replaces the inherited deadline immediately after a
successful handshake. The upgrade request itself still runs under sandhi's
guard, so slowloris protection on the HTTP phase is unchanged.

⭐ **This defect was found by auditing a *documentation* row.** It surfaced
while re-deriving the "Blocked on cyrius / external" table for 3.3.6 — the
slowloris entry claimed bote was still waiting on `sock_set_recv_timeout`. It
was not; the primitive had shipped and was already in force. Chasing *why* it
was in force is what exposed the WebSocket side effect.

### Added — `ws_config_with_idle_ms` / `ws_config_idle_ms`

The frame-loop receive deadline is now explicit and configurable, defaulting to
**0 = no deadline** (correct for WebSocket). An operator who wants idle peers
reaped can set one.

⚠ `WsConfig` grows **64 → 72 bytes**. The field is **APPENDED**, never
inserted, so every existing offset is unchanged and a reader at the old offsets
stays correct. That is the rule bote codified at 3.3.4 after libro 2.8.11
PREPENDED a field to `struct error` and silently broke every raw-offset read of
it. A test asserts the append did not disturb its neighbours.

⚠ sandhi's server is single-threaded, so a connected peer occupies the server
for as long as it is connected. A deadline bounds how long an *idle* peer can
do that; it does not make the server concurrent.

### ⚠ How this was verified, and how it was nearly missed

Reproduced end to end against a real client, not inferred:
`scripts/ws-idle-probe.py` — 3 s idle survives, 33 s got EOF before the fix,
33 s survives after.

⛔ **The first probe "disproved" the bug, and the probe was wrong.** It read
`recv(4096)` once after the pre-idle request and got 4 of 138 bytes. Its
post-idle read then returned the ~134 **stale buffered bytes** still sitting in
the local socket buffer, so a dropped connection looked alive. Two runs (32 s
and 65 s) reported ALIVE against a server that was in fact closing the
connection. The fix was only found because the mechanism had been traced
through the source first and the "it doesn't reproduce" result was treated as
suspect rather than conclusive.

The probe is committed with that hazard documented at the top, because the
drain is load-bearing: **any rewrite that does not drain to a timeout before
idling will pass while the bug is present.** The `.tcyr` assertions pin the
*contract* (default 0, setter works, append did not clobber) — they cannot
observe a wall-clock socket effect, and say so at the call site.

### Fixed — `release.yml` never gated or shipped `dist/bote-core.cyr`

`ci.yml` has gated both bundles since 2.7.2, but the release workflow
regenerated, freshness-checked, archived and attached only `dist/bote.cyr`. A
core-profile consumer (t-ron's SecurityGate, nein's `mcp` module) could be
handed a tag whose core bundle was never verified and was not attached to the
release at all. Both workflows now cover the same set, and
`bote-<tag>-core.cyr` is a release asset with a checksum.

### Changed — staleness sweep across docs, comments and CI

A three-lens sweep with adversarial verification. Corrected, each verified
against the live tree:

- `cyrius.cyml` and `tests/bote_core_only_smoke.tcyr` still described the core
  profile as **11** and **9** modules; it is 12.
- Three transports claimed they were built on `lib/http_server.cyr`, which was
  folded into `lib/sandhi.cyr` at 5.10.x.
- `src/jsonx.cyr`, `src/registry.cyr` (×2) and `src/jwt.cyr` cited `lib/json.cyr`
  and `lib/base64.cyr`, both subsumed by `lib/bayan.cyr` at 6.1.x.
- The aarch64 denylist comment in `ci.yml` said the syscall peers define 95 and
  88 constants; live counts are **96** and **91**. ⚠ The `17` x86-only figure
  and the regex it feeds were re-derived and are **correct** — untouched.
- `src/sandbox.cyr` and `README.md` carried a stale kavach version and a stale
  `bote_libro_tools` assertion count (22 → 38).

⭐ **Four "stale" findings were REFUTED by the verification pass and
deliberately not applied** — applying them would have damaged correct
documents. `README.md`'s `CancellationToken` is a *type* name still used in
`src/stream.cyr`; 3.3.5 renamed functions, not the documented type.
`CLAUDE.md`'s auto-inject rule is correctly stated (its conditional is "when
only **one** module needs it" — `ws_server` is that case). `CLAUDE.md`'s
`fn_table 5585` sits under an explicit "At 3.3.4" label and is right for that
release. And a `cyrius.cyml` block flagged as a stale figure is a preserved
diagnostic transcript — rewriting it would falsify a record.

### Known, not fixed

⚠ **`src/sandbox.cyr` ships in neither bundle.** It is absent from `[lib]` and
`[lib.core]` and no `src/` file includes it, yet its own header tells consumers
to wire a backend into it — they cannot reach it. Same orphan class as
`jwt.cyr` / `pkce.cyr` before 3.2.0. ⚠ CI cannot see this: the
manifest-completeness gate only walks `src/main.cyr` against `[lib]`, so a
module included by no entry point is invisible by construction. Both need
fixing together, and adding a module to a published bundle is not a patch-sized
change.

### Performance

No change expected or claimed. Benchmarks re-run on a quiet box (load 0.50) and
land within noise. Capacity unchanged: `fn_table` **5586 / 32768** (17%),
`identifiers` **180563 / 524288** (34%).

## [3.3.6] — 2026-08-23 — content blocks reach the core profile; six of seven "external blockers" had already expired

**883** assertions green across 14 suites, 0 failed, plus a materially
stronger core drift guard.

### Added — `content.cyr` joins `[lib.core]` (core profile 11 → 12 modules)

Typed MCP content blocks — `content_text`, `content_text_response`,
`content_array`, `content_array_error`, `content_image`, `content_resource`,
`content_with_annotations` and the rest — now ship in `dist/bote-core.cyr`.

Content blocks are the tool-result format **every** handler emits, transport
or not, but the module was in the full bundle only. Core-profile consumers
(nein's `mcp` module, t-ron) were therefore hand-rolling
`{"content":[…],"isError":…}` with a raw `str_builder` — which means each of
them re-implemented JSON string escaping. ⭐ **That is the point**: duplicated
escaping is duplicated injection surface, and preventing exactly that is what
a shared core profile is for.

- **No new stdlib leaves.** `dist/bote-core.deps` is byte-identical — the
  module's only external references are `_json_emit_escaped` (already in core
  via `dispatch.cyr`), `str_builder_*` / `str_data`, `vec_*` and `alloc`.
  Verified, not assumed.
- Listed **last** in `[lib.core]`: cyrius is single-pass and it calls
  `_json_emit_escaped`.
- Core bundle 95,256 → **104,581 B**.

⚠ Consumers on `dist/bote.cyr` see no change — `content.cyr` was already in
the full bundle. This is additive for the core profile: nothing was removed
and no signature changed.

### Changed — the core drift guard now checks bytes, not just linkage

`tests/bote_core_only_smoke.tcyr` previously answered "did it link". For
content blocks that is not enough, because the reason they belong in core is
the escaping. It now asserts that a quote and a backslash come back **escaped**,
that `content_array` omits `isError` entirely, and that `content_array_error`
emits `"isError":true`.

Both new assertions are **mutation-proven**: expecting the raw unescaped
string exits 7, and asserting `isError` on the success array exits 9. The
substring helper is hand-rolled on purpose — pulling in an assert library
would weaken the very thing this file guards, that the core bundle stands up
with nothing else linked.

### Added — `DEPS-PATTERN.md`: the `cyrius lib sync` step it never documented

New section, **"Resolving the bundle: `cyrius lib sync` BEFORE `cyrius deps`"**.

A first-time consumer of the bote → libro → majra graph hits:

```
dep libro requires 'ct' ... is not in the cyrius stdlib
```

`ct` **is** in the stdlib. The message is misleading (filed upstream as
`2026-08-12-agnosai-deps-misleading-stdlib-error`); what it means is that
`./lib/` has not been provisioned yet. Cyrius deliberately does not
auto-resolve stdlib — a supply-chain choice — so the consumer must declare the
transitive modules **and** run `cyrius lib sync --full` *before* `cyrius deps`,
which only overlays git deps on top. `DEPS-PATTERN.md` documented
`git + tag + modules` and omitted both halves.

⭐ **nein 1.6.0 vendored `bote-core.cyr` outright over this misread**; 1.6.1
retired the vendoring and consumes it as an ordinary git dep. A documentation
gap cost a downstream project a vendoring cycle. The section also records the
two traps that have cost real time here: `cyrius build` does an implicit
resolve (so verify vendored versions *after a build*), and a local `path =`
beats `tag =` and can mask an unpushed tag.

### Changed — the "Blocked on cyrius / external" table, re-derived item by item

Every row was checked against the live pinned stdlib rather than its own
prose. **Six of the seven premises had expired.** Only `$/cancelRequest`
was still true.

| Row | Was | Actually |
|---|---|---|
| Slowloris recv timeout (H5) | waiting on `sock_set_recv_timeout` | Shipped 5.11.13 (`lib/net.cyr:239`) — **and already in force**: bote owns no HTTP accept loop, `sandhi_server_run` applies a 30 s `SO_RCVTIMEO` by default |
| WS `Sec-WebSocket-Key` validation (M4) | waiting on a `ws_server.cyr` fix | Fixed 5.11.16 — `lib/ws_server.cyr:97-98`, guard precedes the `alloc` it protects |
| WS arena-per-frame | waiting on `fl_free` | `fl_free` shipped a month *before* the issue was filed; real primitive is `arena_reset`. Usable since 6.5.9 |
| WS subprotocol negotiation | "header is read" | **Never read** — zero occurrences of `Sec-WebSocket-Protocol` anywhere. And never filed |
| WS per-message deflate | waiting on LZ77+Huffman / a zlib binding | **sankoch** ships native DEFLATE, 16 entry points. Never filed |
| DNS hostname SSRF | waiting on `getaddrinfo` | sandhi has a native RFC 1035 resolver. The old ask was the **wrong shape** |

⚠ **Two things genuinely need filing upstream**, and neither ever was: a
`ws_server` handshake / response-header seam (its exported surface offers no
hook, which blocks subprotocol negotiation *and* deflate), and a sandhi client
that accepts a pre-resolved address (without it, resolve-then-fetch means two
resolutions with an attacker-controlled gap — a real DNS-rebinding window, so
re-filing `getaddrinfo_hosts` would file the wrong thing).

⚠ **A mechanical cause worth recording:** all four of bote's filings in
cyrius's `issues/archived/` still read `**Status:** open.` in their bodies.
Closure lives only in directory placement and cyrius's CHANGELOG — so checking
a premise by opening the issue file, the obvious move, re-confirms a stale
blocker. That is upstream's to fix, but knowing it explains why this class
keeps recurring here.

Also corrected: **"Per-thread request buffers — tracked upstream"** was false
in three places. `thread_local_alloc` and the arena family all shipped, and
nothing upstream tracks it.

### Fixed — documentation

- A historical CHANGELOG line claiming the WS subprotocol header was "read but
  not enforced" is annotated with the correction rather than rewritten — it is
  a record of what was believed, and the belief was wrong.

### Not changed, deliberately

⚠ **The WS 30 s idle-drop is NOT fixed here.** Auditing the slowloris row
surfaced it: bote inherits sandhi's 30 s `SO_RCVTIMEO` — an HTTP-shaped
default — onto the WebSocket socket, and it follows `cfd` into the long-lived
frame loop, where `ws_server_recv_frame` maps *any* `sock_recv` error
(EAGAIN included) to close. bote only answers PINGs, never initiates one.
**Established by inspection; not reproduced against a live client**, and
fixing it means moving the WS transport to `sandhi_server_run_opts` with an
explicit idle value — a functional transport change that deserves its own
release rather than a ride-along. Tracked in the roadmap.

### Performance

No change expected or claimed; this release adds a module to a bundle and
edits docs. Benchmarks re-run on a quiet box (load 0.66) land within noise.
Capacity unchanged from 3.3.5: `fn_table` **5586 / 32768** (17%),
`identifiers` **180563 / 524288** (34%). Gate is 95%.

## [3.3.5] — 2026-08-23 — `cancel_token_new` no longer collides with the stdlib's

**883** assertions green across 14 suites, 0 failed. `src/` is now **entirely
free of duplicate-fn warnings** — previously one.

### Breaking — bote's cancellation-token family is `bote_`-prefixed

| removed | replacement |
|---|---|
| `cancel_token_new()` | `bote_cancel_token_new()` |
| `cancel_token_cancel(t)` | `bote_cancel_token_cancel(t)` |
| `cancel_token_is_cancelled(t)` | `bote_cancel_token_is_cancelled(t)` |

**Migration:** rename the call sites. Signatures, semantics and the 8-byte
layout are unchanged — this is a pure rename, no behavioural difference.

Only `cancel_token_new` actually collided. The other two are renamed anyway
because a token minted by `bote_cancel_token_new` and cancelled by a bare
`cancel_token_cancel` is an incoherent surface: half a namespace is worse than
either whole.

⚠ Consumers of **`dist/bote-core.cyr` are unaffected** — `src/stream.cyr` is
not in the `[lib.core]` profile, so the core bundle never carried these
symbols.

### Why it mattered, and why it was not a live bug

`src/stream.cyr`'s `cancel_token_new` collided with stdlib
`lib/async.cyr:1058`'s. Cyrius has **one flat function namespace with "last
definition wins"**, and the winner is decided by include order — which the
*consumer* controls, not bote.

In bote's own binaries `src/` is included after `lib/`, so bote's definition
won and nothing ever misbehaved. But a consumer vendoring `dist/bote.cyr` and
including `async.cyr` **after** it silently got async's implementation instead,
with no error and no warning on their side. Carried as a known-benign
diagnostic since 3.3.0 — the two bodies are semantically identical (an 8-byte
heap flag initialised to 0), which is exactly why it survived: it was a latent
trap, not a visible failure.

⭐ **The collision was hiding a function.** `fn_table` goes 5585 → **5586**
(+1) and `identifiers` 180531 → **180563**. Nothing was added — under "last
definition wins" async's `cancel_token_new` was being *overwritten* and
occupied no slot of its own. Both now coexist. That +1 is the most concrete
evidence available that a definition really was being replaced rather than
merely shadowed.

### Fixed

- The `warning:src/stream.cyr:11:1: duplicate fn 'cancel_token_new'` emitted on
  every build since at least cyrius 6.4.34 is gone. `src/` now produces **zero**
  duplicate-fn warnings, so any future one is signal rather than noise buried in
  a known-warning baseline.

### Not fixed, and deliberately so

⚠ `duplicate fn '_sub_new'` remains. It is a collision **between two deps**
(`lib/libro.cyr` and `lib/majra.cyr`), not one bote can resolve from here — the
fix belongs upstream in whichever library is willing to prefix. The
`lib/sigil-mldsa.cyr` vs `lib/sigil.cyr` duplicates likewise remain and are
benign: both files are the same sigil version and the bodies are
byte-identical.

### Performance

No change expected and none claimed — this release is a rename. Benchmarks were
re-run on a quiet box (load 0.61) and land within noise of the 3.3.4 block; the
small differences in `benches/history.log` are host variance, not a code
effect. Binary sizes are unchanged at 2,971,160 / 2,935,688 / 2,935,848 B.

Capacity: `fn_table` **5586 / 32768** (17%), `identifiers` **180563 / 524288**
(34%), `var_table` **2716 / 8192**. Gate is 95%.

### Provenance

⚠ This work was completed once before, in a background session on branch
`claude/eloquent-taussig-ee16ed`, and **lost**: the branch carried zero commits
(it pointed at an ancestor of `main`) and all of its work sat uncommitted in a
worktree that was then removed. Nothing was recoverable — no stash, no dangling
commit. The only surviving trace was that session's build logs, which did
confirm the approach worked (zero `cancel_token_new` warnings, build `OK`). Redone
here from scratch. The lesson is the ordinary one: work that is not committed
does not exist.

## [3.3.4] — 2026-08-22 — libro 2.8.12 moved `struct error`, and bote's tamper-report path segfaulted on it

### Changed — cyrius 6.5.31 → 6.5.35, libro 2.8.10 → 2.8.12, majra 2.6.6 → 2.7.0

**883** assertions green across 14 suites, 0 failed — up from 867; the 16 new
ones are the regression cover for the defect below.

### Security — `libro_verify` crashed the server on a tampered audit chain

libro **2.8.11 prepended `magic` to `struct error`** (`fl_alloc(48) → (56)`),
shifting every field by **+8**. `src/libro_tools.cyr` read that struct by raw
byte offset and was not moved with it, so the three accessors returned
magic-as-code, code-as-msg and field_name-as-index. `msg` is handed to
`_json_emit_escaped`, which does `load8(cstr + i)` — so decoding a violation
dereferenced the small integer error **code** as a pointer: **SIGSEGV, exit
139**, reproduced by mutation.

⭐ **It fires only when the audit chain is tampered with.** `libro_tool_verify`
touches the error struct only when `chain_verify` returns non-zero, so the
crash sat exactly on the path whose job is to report that the audit trail was
altered — fail-crash on the security-critical branch, while every happy-path
assertion stayed green.

⚠ **No test caught it, and the reason is structural, not an oversight.** The
suite installed `libro_tools_init(chain_new())` on an **empty** chain;
`chain_verify` short-circuits at `if (n == 0)`, so bote's only code touching a
libro `error` struct was unreachable from any assertion. The same was true of
`libro_tool_proof`: it always bailed at the range check, so `merkle_build` /
`merkle_inclusion_proof` / `merkle_tree_root` were never invoked by a test
either.

Both are now covered end to end — a chain is built, entry 0's `source` is
corrupted, and the violation is decoded through the real handler; a
three-entry chain exercises the proof path including the odd trailing leaf.
`tests/bote_libro_tools.tcyr` 22 → **38** assertions.

The fix does not restore the offsets. `struct error` carries
`#derive(accessors)` (`lib/libro.cyr:45`), so bote now reads it through
libro's generated `error_code` / `error_msg` / `error_index`, which track the
layout automatically — **this failure mode cannot recur.** The raw-offset
convention is retained for `struct chain` and `struct entry`, where it remains
correct and where libro exposes no getters.

⭐ **The generalisable rule, now recorded beside the code:** a field APPENDED
to a dep's struct is safe to ignore — that is why libro 2.8.4's `chain` growth
(32 → 40 B) never mattered. A field INSERTED AT THE FRONT moves everything,
and a raw-offset reader has no way to notice.

### Breaking — inherited from libro, with nothing for bote itself to migrate

libro 2.8.11 and 2.8.12 each changed the entry-hash preimage, and 2.8.12 also
changed the Merkle construction (RFC 9162 leaf and internal-node domain tags;
an odd trailing node is now **promoted** rather than paired with itself) and
the tree-head signature, which now covers `tree_size`, `timestamp` and
`algorithm` instead of the root alone.

**bote has nothing to re-anchor.** `src/main.cyr` stands up a purely in-memory
`chain_new()`; the tree has no `filestore_*` / `patrastore_*` / `memstore_*` /
`anchor_*` / `sign_tree_head` call sites, so no chain outlives the process and
there is no stored preimage or signature to invalidate. The observable
consequence is confined to **values**: the `hash` strings emitted by
`libro_export` / `libro_query` and the `root` emitted by `libro_proof` differ
from 3.3.3 for identical input.

⚠ **A consumer that persists a libro chain of its own must re-export and
re-anchor before upgrading** — a chain written by libro ≤ 2.8.10 will not
verify under 2.8.12, and neither will proofs or anchors over it. That reaches
downstream through **agnosai → bote → libro**: anyone serving a durable audit
trail behind bote's `libro_*` tools takes libro's migration, not bote's.

### Fixed — the MCP `initialize` handshake had reported the wrong version since 3.3.1

`_bote_server_version()` read **`"3.3.2"`** while `VERSION` said `3.3.3`, so
3.3.3 shipped a handshake — and two `dist/` bundles — announcing 3.3.2. This
is the exact drift `scripts/version-bump.sh` grew a guard for at 3.3.2; the
guard is sound and fired correctly here, which is how it was found. 3.3.3 was
cut without running the script.

### Changed — the dependency chain, and what actually moved

Resolved tag chain: **bote 3.3.4 → libro 2.8.12 → patra 1.13.10**, majra 2.7.0.

`cyrius.lock` moves **exactly four hashes** — `libro`, `majra`, `bayan`
(1.4.2 → 1.5.2) and `vani` (1.1.3 → 1.2.2). The entire 6.5.31 → 6.5.35 stdlib
snapshot differs in only three files, and the third, `patra` (1.13.9 →
1.13.10), is **net-zero for bote**: libro's `[deps.patra]` overlay already
pinned 1.13.10, so the fold merely caught up to it. After 3.3.1 and 3.3.2
spent two releases unpicking transitive patra drift, the pin and the fold are
now level.

⚠ Verified **after `cyrius build`, not after `cyrius deps`** — 3.3.1 recorded
that build does an implicit resolve which has silently reverted a vendored
file before. `lib/patra.cyr` holds at 1.13.10, `lib/sigil.cyr` at 3.12.9.

⚠ `majra` and `sigil`'s sibling checkouts sit one commit past their tags, and
`path =` vendors the working tree rather than the tag. Both post-tag commits
are docs-only and leave `dist/` untouched, so the vendored bytes are
byte-identical to the tags — checked rather than assumed, because this is the
drift class that broke a CI lock-verify before.

majra 2.7.0 is additive for bote: `pubsub_publish`'s signature and the 32-byte
hub struct are unchanged, and `PUBSUB_LAG_BLOCK` remains the default, so the
wire behaviour is identical. Its internal win is on bote's exact path — the
per-publish `map_keys()` allocation (a `vec_new` + `vec_push` per key on
**every** publish, bump-allocated and never freed) is gone.

### Performance

Capacity on `src/main.cyr`, both measurements freshly resolved rather than
compared against a stale `lib/`: `fn_table` 5502 → **5585 / 32768** (17%),
`identifiers` 178561 → **180531 / 524288** (34%), `var_table` 2702 → **2716 /
8192**. Gate is 95%. The growth is almost entirely bayan 1.5.2, whose 368 new
fns land in the unreachable set.

⚠ **Benchmarks are FLAT and the first measurement saying otherwise was wrong.**
An initial run showed a uniform 7–17% regression across all 14 benchmarks. It
was taken while seven audit agents saturated the box (load 4.5). Re-measured
as a controlled interleaved A/B — two binaries built from the two dep trees,
run alternately, five rounds each, best-of-5, load 0.8 — twelve of fourteen
land inside their own arm's spread, and the two that do not
(`jsonx_get_str_flat` −11.1%, `codec_parse_request` +6.2%) move in **opposite**
directions on sub-microsecond work. The honest result is: no change. Do not
cite the first number.

⚠ The `benches/history.log` block for 3.3.4 is ~2× better than the 3.2.1 block
across every row. **That is host state, not this release.** Every 3.2.1 row
carries a ~129 µs max outlier; the 3.3.4 maxima are under 16 µs.

### The cyrius 6.5.35 codegen differential — which upstream reported as unobtainable

6.5.35 is a register-allocator rewrite (linear-scan interval expiry plus
loop-aware liveness), the first release whose picker can time-share a
register; its own notes say reverting the prior behaviour fails 69 of 282
corpus tests with **wrong answers rather than crashes**. That makes it the
highest-risk bump class for a consumer, and libro 2.8.12 recorded that it
could not run a differential because prepending a version's `bin` to `PATH`
changes what `cyrius --version` reports but not which `cycc` `cyrius build`
invokes.

⭐ **On this host it is obtainable, and it was run.** Per-version compilers
live at `~/.cyrius/versions/<V>/bin/cycc` and can be invoked directly on
stdin; `~/.cyrius/bin/cycc` is byte-identical (sha256 `1f14ebff…`) to
`6.5.35/bin/cycc`, confirming the wrapper always uses the installed one. The
stdlib prelude the wrapper injects was reconstructed from `[deps] stdlib`
order, so both compilers received **identical source**. Includes are deduped,
so a full prelude composes with the test files' own includes.

Result across all 13 assertion-bearing suites — **867 / 867 assertions agree,
compiler for compiler**, with 6.5.35 emitting a smaller binary every time:

| suite | 6.5.31 | 6.5.35 | delta | result |
|---|---:|---:|---:|---|
| `bote` | 2,888,080 | 2,863,504 | −24,576 | 424 = 424 |
| `bote_streamable` | 2,836,304 | 2,811,728 | −24,576 | 53 = 53 |
| `bote_host` | 2,542,896 | 2,522,416 | −20,480 | 113 = 113 |
| `bote_jwt` | 2,529,256 | 2,512,872 | −16,384 | 53 = 53 |
| `bote_libro_tools` | 2,734,952 | 2,714,472 | −20,480 | 22 = 22 |
| _(9 more, all AGREE)_ | | | −4,096 … −20,480 | |

⚠ **This is behavioural evidence, not proof against miscompilation.** It shows
the paths the suite reaches are unaffected; the build reports 4,038 unreachable
fns, `src/transport_http.cyr` has no dedicated test file, and
`tests/bote_ws.tcyr` is 10 assertions of config wire-up. bote makes **no speed
claim** from the regalloc rewrite — the toolchain's own notes say no runtime
win is demonstrable on their corpus, and bote's A/B agrees.

⚠ x86_64 only. aarch64 takes the `RA_SCAN_LOOPS` stub and keeps pre-.35
behaviour — neither the win nor the risk.

### Platform

Cross-build green for all three binaries on **aarch64** (`EM_AARCH64`
verified). ⚠ The aarch64 *runtime* sweep under `qemu-aarch64` 11.1.0 is
**partial, and this is an emulator limit, not a regression**: a three-line
probe shows `random_bytes(16)` returning a negative errno under qemu while
returning 16 natively, so `getrandom` is not passed through. bote's
fail-closed contract then refuses to mint a guessable session ID and exits 90,
exactly as designed. 373 of 883 assertions execute; `bote.tcyr`,
`bote_pkce.tcyr` and `bote_streamable.tcyr` cannot run there. CLAUDE.md's
"green on x86_64 **and** aarch64" is corrected to say so.

### Fixed — CHANGELOG figures that were wrong when written

3.3.3 opened with "**116** assertions green across 14 suites" and 3.3.2 with
"**881**". Both are **867**, freshly measured. Corrected in place rather than
left as a record, because they are claims about a measurement, not narrative.

## [3.3.3] — 2026-08-21 — libro 2.8.10: a PatraStore read from another thread no longer crashes

### Changed — libro 2.8.8 → 2.8.10

**867** assertions green across 14 suites, 0 failed.

libro 2.8.9 removes `PatraStore`'s cross-thread prepared-statement cache, and
2.8.10 declares the patra it is actually built against.
`patrastore_open` prepared its `SELECT` and `COUNT` once and stored the handles;
patra's SQL parse scratch is **per-thread**, so executing one from another thread
dereferenced TLS that was not there and **killed the process** — no diagnostic,
no error return.

bote does not read a PatraStore off-thread today, so nothing here changes
behaviour. It is carried because bote is the middle link in
**agnosai → bote → libro**, and Agnostic — which does serve a durable audit trail
from a worker pool — cannot get the fix any other way.

⚠ **`[deps.patra]` moves 1.13.9 → 1.13.10 with it**, because libro 2.8.10
declares it. 1.13.10 removes an unconditional `sakshi_set_level(SK_WARN)` from
`patra_init` that clobbered the host's log level. That transitive tag is exactly
the thing 3.3.2 had to unpick, so it is stated rather than left to be discovered:
resolved from git, this tag chain is
`bote 3.3.3 → libro 2.8.10 → patra 1.13.10`.

⚠ **Tag ordering.** patra 1.13.10 is tagged; libro 2.8.10 must be tagged before
this release's lock resolves for anyone without sibling checkouts. (libro 2.8.9
shipped the crash fix but still declared patra 1.13.9 — 2.8.10 corrects that,
which is why this depends on .10 and not .9.)

## [3.3.2] — 2026-08-19 — closing the transitive patra downgrade, and a version literal that had lied since 3.3.1

### Changed — cyrius 6.5.20 → 6.5.31, libro 2.8.5 → 2.8.8, majra 2.6.3 → 2.6.6

**867** assertions green across 14 suites, 0 failed.

The toolchain pin was eleven patches behind and pulled the 6.5.20 stdlib
snapshot; 6.5.31 brings the folds shipped since (sakshi 2.4.11, patra 1.13.9,
yukti 2.3.8, niyama 1.0.7, mabda 4.1.0, ganita 1.1.4, yantra 1.0.3).

### Fixed — the transitive patra downgrade that reached agnosai

bote is the middle link in **agnosai → bote → libro → patra**. libro 2.8.5
declared `[deps.patra] tag = "1.13.8"` while the toolchain folds 1.13.9, and
`cyrius deps` applies a declared dep's copy *on top of* the `lib sync --full`
snapshot on every resolve — so the stale tag rewrote `lib/patra.cyr` for
everything downstream. Measured before the bump: `lib sync --full` landed patra
1.13.9, then `cyrius deps` rewrote it to **1.13.0**.

It only bites where there are no sibling checkouts, because `path` beats `tag`
locally — which is why it passed every developer machine and broke agnosai's CI.
`deps --verify` does not catch it either: the lock is regenerated *from* the
downgraded file.

libro 2.8.8 moves its pin to 1.13.9; taking that here closes the chain. Verified
after this bump: `lib/patra.cyr` stays **1.13.9** across
`lib sync --full` → `cyrius deps`, and `lib/libro.cyr` reports 2.8.8.

Once released, agnosai can drop the defensive `[deps.patra]` shim it carries in
2.0.2 and move to 6.5.31 itself.

⚠ **Release order matters**: libro 2.8.8 must be tagged and pushed before this
release's `[deps.libro] tag = "2.8.8"` will resolve in CI, which has no
`../libro` to fall back on.

### Fixed — `_bote_server_version()` reported 3.3.0, and the bump script's guard could not see it

`src/dispatch.cyr:22` read `"3.3.0"` while VERSION said 3.3.1, so bote 3.3.1's
MCP `initialize` handshake under-reported its own version to every client.

The reason it survived a release is the guard in `scripts/version-bump.sh`: it
asserted the **OLD** value was absent rather than that the **NEW** value was
present. Once the literal had drifted off OLD, the sed pattern (built from OLD)
matched nothing *and* the absent-OLD check passed — so the script rewrote
nothing and reported success. A guard that can only fail when the thing it
guards is already correct is vacuous.

Both fixed: the literal now reads 3.3.2, and the guard asserts the new value is
present, printing the offending line when it is not.

### Changed — 13 `src/` files reformatted for 6.5.31's canonical form

**Caused by the pin bump, unlike libro's sweep.** bote was 0/35 unformatted
under 6.5.20 and 13/35 under 6.5.31 — the formatter's canonical continuation
indent changed between them. (libro's 6 files fail identically under 6.5.27 and
6.5.31, i.e. genuinely pre-existing; the two sweeps are not the same finding.)

bote's CI has no format gate, so this would not have failed the build — it would
have ambushed the next person to run `cyrius fmt`. Swept now.
`git diff -w -- src/` is empty apart from the `_bote_server_version` literal
above, so nothing but indentation moved. Both bundles regenerated and verified
idempotent across a second `cyrius distlib` / `cyrius distlib core`.


## [3.3.1] — 2026-08-12 — the dependency set was years behind, and one pin hid two auth bypasses

### Security

- **`[deps.sigil]` is REMOVED, which upgrades sigil 3.12.1 → 3.12.7 and closes
  two authentication bypasses bote was pinned behind.**

  `sigil` is already listed in `[deps].stdlib` and sigil is **folded into the
  cyrius stdlib snapshot**, so this block was a pure override — it held bote, and
  every consumer resolving through bote's manifest, on a sigil from before:

  - **3.12.5 — PKCS#1 v1.5 verify accepted forged signatures.** Both operands of
    the final compare lived in `cbank()`-banked file-scope globals, so a lane
    collision let one thread's valid pair satisfy another thread's check. The
    reporting consumer measured **888 forged signatures accepted out of 400,000**,
    alongside **70% of valid signatures wrongly rejected**.
  - **3.12.6 — the same class in RSA-PSS**, which is the path TLS 1.3
    CertificateVerify runs through. bote terminates TLS in `transport_http`,
    `transport_streamable` and `transport_ws`, so this is on bote's own peer
    authentication, not a hypothetical. Root cause was a cyrius tail-call
    frame-release bug fixed in **6.5.14** — which this pin also predated.

  bote's own JWT is HS256 and PKCE is SHA-256, neither of which touches the RSA
  paths; the exposure is the TLS transports, plus every downstream consumer that
  inherited the downgrade.

  The pin's original justification is also gone: it existed because the pre-fold
  stdlib *registry* lagged the self-contained bundle (the 6.1.x registry resolved
  `sigil` → 3.7.10, whose `dist/sigil.cyr` dangled guarded `include` lines that
  broke single-file consumers). 3.7.12 inlined those banks and the fold has been
  self-contained since.

### Changed

- **Toolchain pinned to cyrius 6.5.20** (was 6.5.10 — ten releases behind).
  Full three-step, `lib/` diffed against the 6.5.20 snapshot **after the sync and
  again after a build**: zero drift, `cyrius deps --verify` 113 verified / 0 failed.

  6.5.20 is taken rather than .19 for two reasons. It **re-folds patra 1.13.0**,
  which is what finally retires the defensive sakshi pin below; and it fixes a
  **P1 miscompile** in `switch` / `match` — a case body could only be left safely
  by `return`, so a body that fell through after storing to a local produced a
  wrong answer with no diagnostic, or a segfault, on every target. The cause was
  the regalloc NOP-harvest compactor shifting jump-table entries by +4 per
  preceding case body. bote's own `src/` contains no statement-position
  `switch` / `match`, so the exposure was in the vendored surface only.

- **`[deps.majra]` 2.5.3 → 2.6.3**, seven releases. Picks up an O(n²) queue drain
  fix (a `pq_dequeue` that shifted every survivor per pop — 15.6 µs → 89 ns at
  depth 16,000), an unguarded negative priority that indexed **before** the queue
  struct, a concurrent-`relay_receive` allocator race, and majra dropping its own
  `[deps.sakshi]`.

- **`[deps.libro]` 2.8.4 → 2.8.5.** This is the release that terminates the
  stale-sakshi chain: libro bumps its own `[deps.patra]` 1.12.12 → **1.13.0**
  (zero `[deps.*]` blocks) and its `[deps.sigil]` / `[deps.sigil_tpm]` 3.12.1 →
  **3.12.7**. ⚠ libro fed the overlay through **both** pins — sigil 3.12.1's
  manifest declared `[deps.sakshi]` at **2.4.3** alongside patra's 2.4.2 — which
  no earlier analysis had identified.

  ⚠ libro 2.8.5's `dist/libro.deps` no longer declares `sakshi` as a leaf. bote
  is unaffected because it declares `"sakshi"` in its own `[deps].stdlib`; do
  not remove that declaration on the assumption libro supplies it.

- **`[deps.sakshi]` (2.4.10) is DELETED — the workaround's two preconditions
  both landed.** It was a forward pin defending against a chain three levels
  down, not against anything bote wants: bote references zero sakshi symbols
  directly.

  ```
  bote -> [deps.libro] 2.8.4 -> [deps.patra] 1.12.12 -> [deps.sakshi] 2.4.2
  ```

  ⚠ **Deleting it EARLIER in this cycle was tried and made things worse**, and
  the mechanism is kept because it is what makes the verification rule
  non-negotiable: with the block removed the three-step ended correct (2.4.10)
  and **the very next `cyrius build` silently reverted `lib/sakshi.cyr` to
  2.4.2** through the implicit resolve. `deps --verify` cannot catch that — the
  lock is written *from disk*, so it records the downgraded file's hash.

  Both gates are now cleared: cyrius **6.5.20 re-folds patra 1.13.0**, and
  **libro 2.8.5** bumps the link bote actually resolves through. Re-verified
  **after a build**, not after the sync: `lib/sakshi.cyr` holds at the
  snapshot's **2.4.10**, `lib/patra.cyr` at 1.13.0, `lib/sigil.cyr` at 3.12.7,
  and no stdlib file differs from the snapshot.

- **`[deps.majra]` 2.5.3 → 2.6.3 also picks up majra's own 6.5.20 pin**, which
  retires the last piece of the `fl_alloc` workaround: `relay_receive_ex`
  allocates its result struct outside the lock again, shortening a critical
  section held across a subscriber-list walk on a hot pub/sub path.

### Known issues

- ✅ **The whole four-level chain is CLOSED as of this release.** All three
  upstream repos landed, in order: **patra 1.13.0** (zero `[deps.*]`, sakshi from
  the fold), **cyrius 6.5.20** (re-folds patra 1.13.0), **libro 2.8.5**
  (`[deps.patra]` → 1.13.0, `[deps.sigil]`/`[deps.sigil_tpm]` → 3.12.7). bote's
  defensive `[deps.sakshi]` is deleted above and re-verified after a build.

  ⚠ **One correction to the earlier plan, which had libro deleting all three of
  its git deps in favour of `[deps].stdlib` entries.** Two of those three would
  have been wrong:

  - **`[deps.patra]` must STAY** (bumped, not deleted). bote consumes
    `dist/libro.cyr`, which calls `patra_*` 54 times and defines none of them,
    and bote declares no patra of its own — so bote can only obtain patra by
    resolving libro's block transitively. Deleting it strands bote.
  - **`[deps.sigil]` must stay a THIN sub-bundle selection** rather than becoming
    a `[deps].stdlib` entry. The fold is the monolith: `lib/sigil.cyr` is 27,550
    lines carrying the x509/RSA bignum banks. Measured — a consumer taking
    `dist/sigil.cyr` builds with `warning: large static data (10794336 bytes)`,
    while libro's thin build is `.bss` **80,224 bytes**. Only the *tag* needed to
    move.

- ⚠ **A latent hole in libro's sidecar was unmasked, not created, by 2.8.5.**
  `dist/libro.deps` declared `sakshi` only because patra 1.12.12's
  `[deps.sakshi]` block put it there — the very defect being fixed. Measured at
  27 leaves on 1.12.12, **26** on 1.13.0, and 26 with the block removed
  entirely. bote is unaffected (it declares `"sakshi"` itself), but a clean-room
  consumer relying only on libro's sidecar would now fail. `cyrius distlib`'s
  self-check cannot catch this class: undefined *functions* are downgraded to
  warnings, so only an undefined *variable* fails a bundle.

- **13 repos in this workspace still pin `[deps.sakshi]` at stale tags**
  (2.3.0 → 2.4.7): argonaut, anuenue, shakti, garjan, ghurni, daimon, phylax,
  prani, vidya, stiva, yukti, owl, sit. None is in bote's dependency path.

- **`duplicate fn 'sha256_hex'` / `'sha512_hex'`** (`lib/sigil_hex.cyr` vs
  `lib/sigil.cyr`) are inherited from libro's thin sigil selection and are
  **benign**: both files are sigil 3.12.7 and the function bodies are
  byte-identical, verified by diff. Alongside the pre-existing `_sub_new` and
  `cancel_token_new` diagnostics.

## [3.3.0] — 2026-07-31 — `serverInfo` is configurable; a dispatcher can carry the host's identity

### Added

- **`dispatcher_set_server_info(d, name, version)`** plus the
  `dispatcher_server_name` / `dispatcher_server_version` accessors.

  `_build_initialize_result` hardcoded `"serverInfo":{"name":"bote"}`
  (`src/dispatch.cyr:402`), so **every consumer embedding bote's dispatcher
  introduced itself to MCP clients as bote**, at bote's version, rather than as
  the host product. That is wrong for any downstream, and for a host whose own
  wire says something else it was unfixable short of hand-rolling the whole
  JSON-RPC envelope — which is exactly what agnosai did (`server/routes/mcp.cyr`
  builds its own, citing this line as the reason it cannot delegate).

  Both arguments are cstrs and each falls back independently: `0` means "keep
  bote's value for this field", so `dispatcher_set_server_info(d, "agnosai", 0)`
  renames the server and keeps bote's version.

  **Additive — an unconfigured dispatcher emits the pre-3.3.0 wire byte for
  byte**, and a test pins that default alongside the overrides so the
  compatibility claim is checked rather than asserted. Mutation-verified:
  forcing the name back to a constant fails two assertions.

  `dispatcher_new` grows 72 → 88 bytes. ABI-safe: `dispatcher_new` is the only
  constructor (verified — the other `alloc(72)` in `transport_http.cyr` is an
  unrelated struct), the size is not documented to callers, and all 22 call
  sites take the struct from the function.

  Ships in **both** profiles, including `[lib.core]` — `dist/bote-core.cyr` is
  what agnosai pins.

### Changed

- **Cyrius pin `6.5.3` → `6.5.4`**, with `cyrius lib sync --full`. `lib/` matches
  the pinned snapshot exactly — 0 of 99 files differ — and the build emits no
  drift or shadow warning. The two files outside the snapshot (`libro`, `majra`)
  are declared git deps, not staleness.
- `_bote_server_version()` returns `3.3.0`.

## [3.2.1] — 2026-07-30 — `sys_accept4` closes the last x86-numbered syscall; accept loop no longer busy-spins

**Closes the item 3.2.0 deliberately left open.** `src/transport_unix.cyr`'s accept call was the
last place bote reached past the stdlib to a raw syscall number, and fixing it surfaced a second,
worse defect in the same six lines: the loop had no error branch at all.

### Fixed
- **`syscall(SYS_ACCEPT, sfd, 0, 0)` → `sys_accept4(sfd, 0, 0, 0)`.** `SYS_ACCEPT` is defined in
  **neither** syscall peer — its only definition anywhere in `lib/` is `lib/net.cyr:12`, a bare
  unguarded `var SYS_ACCEPT = 43`, the **x86_64** number. It therefore resolved on every arch and
  worked on aarch64 only because the cyrius backend emits a runtime renumber chain (`ESYSXLAT`,
  43 → 202) ahead of every `svc`. That is an implementation detail rather than a documented
  guarantee, and 43 on aarch64 is `statfs` — an unlisted number does not fail loudly, it calls the
  wrong syscall. `sys_accept4` issues `SYS_ACCEPT4`, which the peers define natively per arch
  (x86_64 288, aarch64 242); neither is in the renumber table because neither needs to be.
  `accept4(fd, NULL, NULL, 0)` is defined to be exactly `accept(fd, NULL, NULL)`, so behaviour is
  identical. `flags` stays `0` — `SOCK_CLOEXEC` is the usual reason to reach for accept4 and would
  be a behaviour change, so it belongs in its own bite.
  - **`sock_accept()` was considered and rejected.** It is the obvious "own the stack" answer and
    the one 3.2.0's issue note suggested, but its own non-agnos branch (`lib/net.cyr:350`) issues
    the **same bare 43** — it would have relocated the dependence into the stdlib, not removed it.
  - The stdlib makes this argument itself. `sys_getpeername` in `lib/syscalls_linux_common.cyr`
    documents sandhi 1.9.0 hardcoding a raw `syscall(52, …)` for getpeername, which on aarch64 is
    **fchmod** and *succeeds* — "the caller believes it obtained a peer address while its buffer is
    untouched garbage. Verified on real pi." Its conclusion is explicit: do not solve that class
    with an ESYSXLAT entry, take the number from the per-arch peer. This does that.
  - Verified, not assumed: a new assertion calls `sys_accept4` on a non-socket fd and requires
    `-ENOTSOCK` (−88), which under `cyrius test --aarch64` proves 242 genuinely lands on accept4
    there. A live three-connection round-trip over a real AF_UNIX socket confirms
    accept → handle → close → re-accept, with the socket at mode 0600 and the idle process at
    00:00:00 CPU.
- **The accept loop busy-spun forever on any persistent error.** It was
  `if (cfd >= 0) { … }` with **no else**, so a failing `accept(2)` was re-issued immediately,
  without bound, sleep or diagnostic — 100% of one core indefinitely. Under `EMFILE` the pending
  connection is not dequeued either, so it retried the same connection at full speed. This is an
  availability defect, not a privilege one: the socket is `chmod 0600`, so no other UID can
  connect; the realistic trigger is bote's own fd exhaustion (libro/patra chain fds, `fs_tools`,
  `web_tools`' sandhi client sockets) or a low `RLIMIT_NOFILE`.
  - Replaced with a three-way policy in a **pure, unit-testable** `_unix_accept_action(errno)`:
    `EINTR` / `ECONNABORTED` / `EPROTO` / `EAGAIN` retry immediately; `EBADF` / `EINVAL` /
    `ENOTSOCK` / `EOPNOTSUPP` are fatal and hand the listener back to the caller (per the no-panic
    rule — the consumer decides); everything else backs off. Backoff is capped exponential
    (1 ms doubling to 250 ms, reset on success) with a 200-consecutive-failure give-up (~50 s).
  - **Unknown errnos back off rather than being treated as fatal**, deliberately: misclassifying a
    recoverable condition as fatal would take a working server down, whereas backoff merely wastes
    time before the give-up counter ends the loop. Sleeping is never as bad as spinning.
  - `EAGAIN` is currently unreachable — the listen fd is never made non-blocking and carries no
    `SO_RCVTIMEO` — and is handled anyway so the loop stays correct if that changes.
  - **Note for the other four transports:** `sandhi_server_run` and its pooled/TLS variants carry
    the *identical* unguarded spin, so http / streamable / ws / bridge still inherit it. bote's
    policy is deliberately stricter than the house pattern rather than copying it; the sandhi side
    wants an upstream fix.
- **Stale line citations** in `docs/development/issues/archive/2026-07-17-aarch64-sys-open-urandom.md` and
  the 3.2.0 CHANGELOG entry (`transport_unix.cyr:113` → `:123`, `lib/net.cyr:331` → `:320`),
  caught while verifying this change.

### Added
- **`tests/bote_transport_unix.tcyr` — 47 assertions, the first coverage this module has ever
  had.** No test file included `src/transport_unix.cyr` and no assertion touched
  `transport_unix_run`, `_unix_sockaddr`, `_unix_process_lines` or `_unix_handle_client`.
  `transport_unix_run` itself cannot be unit-tested (it blocks in an accept loop forever), which is
  precisely why the error classification was factored out as a pure function. Covers: the full
  `_unix_accept_action` errno matrix including the unknown-errno default; `_unix_backoff_next`
  including the clamp at the ceiling and a negative input never yielding a negative sleep;
  `_unix_sockaddr` including the **107-byte truncation clamp and its exact boundary**, previously
  untested; that `sleep_ms` really blocks (this is bote's first `sleep_ms` call site, and on
  aarch64 it works only because the backend rewrites `poll(7)` into `ppoll(73)` — before that fix
  it returned `-EFAULT` instantly and the "sleep" was a spin); and the `sys_accept4` arch guard.
- **CI: a bare `syscall(SYS_*)` in `src/` is now a hard failure.** The 3.2.0 denylist enumerates
  constants that are x86-only *today*, so it could not have caught `SYS_ACCEPT`, which is x86-only
  by virtue of being defined nowhere except `net.cyr`. Banning the *form* is the stronger guard:
  every syscall bote needs has an arch-portable stdlib wrapper. Verified to fire on the pre-fix
  `transport_unix.cyr` and pass on the fixed tree.

### Changed
- **The remaining eight raw `syscall(SYS_*)` sites in `src/` now use their stdlib wrappers** —
  `sys_exit` in the three `main*.cyr` entries, and `sys_write` / `sys_read` in
  `src/transport_stdio.cyr`, alongside `sys_socket` / `sys_bind` / `sys_listen` / `sys_close` in
  `src/transport_unix.cyr`. All were already arch-correct (the peers define these per arch), so
  this is style — but it is what lets the CI gate above be an exception-free ban rather than a
  list of carve-outs. All four wrappers exist on the agnos peer too, so no target regresses.
  Verified with a live stdio round-trip (`initialize` + `tools/list`).

## [3.2.0] — 2026-07-30 — toolchain 6.5.3 + full dep refresh; aarch64 portability; JWT `exp` enforced and JWT/PKCE finally shipped

**Minor, not a patch — `dist/bote.cyr` grows two modules.** `src/jwt.cyr` and `src/pkce.cyr`, the
"JWT HS256 + RFC 7636 PKCE" that `[package].description` has advertised since **2.2.0**, were in
neither `[lib]` nor `[lib.core]` and reachable by no consumer. They ship now — with the `exp` check
their own header has documented and **not performed** for the same eight releases actually
implemented, and with the `alg` substring scan replaced by an exact field read.

Alongside that: every dependency pin moves to its current tag, the toolchain goes onto the 6.5.x
line, the three x86_64-only syscall constants that made an aarch64 cross-build impossible are gone,
and a `[deps.sigil]` tag/lock drift that had CI red at HEAD is realigned.

### Security
- **`jwt_verify_hs256` enforced no expiry.** Its header comment claimed *"`exp` claim, if present
  in the payload, is in the future (uses chrono's wall-clock seconds)"* since 2.2.0. No such check
  existed — `exp` appeared exactly once in the file, in that comment, and the module referenced no
  clock function at all. A structurally valid, correctly signed, **arbitrarily expired** token
  verified. Severity was bounded only by the packaging defect below: no consumer could reach the
  module. Fixing the packaging without fixing this would have shipped the gap to every consumer at
  once, so it is fixed first. Resolves finding (2) of
  `docs/development/issues/archive/2026-07-30-jwt-module-is-orphaned-and-documents-an-exp-check-it-does-not-perform.md`.
  - Runs **only after the HMAC verifies**. The constant-time compare was previously the function's
    terminal statement; it now short-circuits, and the payload is decoded below that line, so claim
    parsing never touches unauthenticated bytes.
  - Absent `exp` accepts (RFC 7519 §4.1.4 makes it optional). Past `exp` rejects. **`exp == now`
    rejects** — §4.1.4 requires *now* to be strictly before *exp*. **No leeway**: the RFC permits
    "some small leeway" and bote declines it, because a server that silently honours a token past
    its stated expiry does something the operator cannot see in the logs.
  - A present-but-malformed `exp` — string-typed `"1785461260"`, `null`, `true`, `1e9`, or 25
    digits — **rejects rather than reading as absent**. Folding "unparseable" into "no expiry" is
    the fail-open direction. `_jwt_parse_uint` returns `-1` rather than `0` so the sentinel can
    never be confused with a real zero, and caps at 18 digits so i64 overflow is impossible rather
    than merely unlikely.
  - `clock_epoch_secs() == 0` **skips** the window. `lib/chrono.cyr:50` documents 0 as "clock
    unknown", not the epoch; treating it as epoch would expire every token and lock every client
    out. Matches sigil's `_x509_in_window` precedent.
- **The `alg` check was a substring scan, and it was defeatable.** It searched the decoded header
  for the five bytes `HS256` *anywhere*, so `{"alg":"none","kid":"HS256-2024"}` satisfied it — the
  literal sits inside the `kid` value. Replaced with an exact JSON field read
  (`_jwt_str_field_eq`), byte-exact and case-sensitive per RFC 7515 §4.1.1. The issue classed this
  as "harmless while HS256 is the only algorithm"; it is fixed now anyway, because that caution is
  only sound for as long as nobody forgets it, and because it is a precondition for RS256 ever
  landing here.
- **Claim readers use jsonx's `(buf, len)` internals, not its public cstr API.** Every public jsonx
  entry point begins with `strlen()`. The decoded header and payload are attacker-controlled, and
  an embedded NUL would truncate that view — hiding `exp` entirely, which reads as "no expiry".
  bayan's base64url decoder *does* NUL-terminate its output, so the cstr path would have compiled
  and appeared to work. That is what made it worth avoiding rather than merely noting.

### Added
- **`src/jwt.cyr` + `src/pkce.cyr` ship in `dist/bote.cyr`** (28 → 30 module folds), exporting
  `jwt_verify_hs256`, `auth_validator_jwt_hs256`, `jwt_secret_*`, `pkce_code_verifier` and
  `pkce_code_challenge_s256`. Resolves finding (1) of the same issue. **Deliberately not in
  `[lib.core]`**: that profile's documented stdlib footprint excludes sigil and both modules need
  it (`hmac_sha256` / `sha256`), so folding them in would defeat the reason the core profile
  exists. That is a decision, not an oversight — the issue explicitly left it open. Capacity cost:
  `fn_table` 4961 → 4974, identifiers 163966 → 164307 (15% / 31% of ceiling).
- **`tests/bote_jwt.tcyr` 28 → 53 assertions**, with a new `_mint_hs256(hdr, payload, key)` helper.
  Every token in the file was previously a hardcoded literal, which is why the `exp` cases (whose
  value must track the wall clock) and the alg-confusion case (which needs a *valid* signature over
  a hostile header) could not be expressed at all. **Both gates are mutation-proven**: restoring
  the substring `alg` scan fails 2 assertions, removing the `exp` gate fails 8. Also noted and
  relabelled — the pre-existing `alg=none` literal never reached the alg gate (its empty third
  segment is rejected structurally several checks earlier), so it had never tested what its name
  claimed.
- **aarch64 portability gate in CI** (`.github/workflows/ci.yml`). bote had no aarch64 lane at
  all, which is exactly why the regression below went unnoticed until a consumer hit it. Two
  gates, both deterministic and emulator-free: a comment-stripped **denylist grep** over
  `src/*.cyr` + `dist/*.cyr` for all 17 `SYS_*` constants that exist on x86_64 but not aarch64,
  and a **blocking cross-build** of all three entries asserting `e_machine == 0xB7`
  (`EM_AARCH64`) on each output. The denylist was verified to fire on the pre-fix
  `src/session.cyr` and pass on the fixed tree.
- **`tests/bote_fs_tools.tcyr` and `tests/bote_web_tools.tcyr` added to the CI test list.** Both
  existed and passed locally but were absent from the CI loop — 53 assertions that were never
  gated.

### Fixed
- **`bayan_json_v_parse_str` → `bayan_json_v_parse_buf`** (`src/web_tools.cyr`). bayan 1.3.0, folded
  into the stdlib at cyrius 6.5.1, renamed the buf+len typed JSON parser and removed the old name.
  Identical `(buf, len)` signature and semantics, but building `src/main.cyr` against the 6.5.x
  stdlib died with `error: refusing to emit binary with 1 reachable undefined function(s)`. This was
  the only stdlib removal reaching bote — a mechanical diff of all 666 distinct stdlib call targets
  in `src/` across the 6.4.66 and 6.5.3 snapshots found no other removal and **zero** arity changes.
- **17 wrong-arity call sites in the test + bench suites.** cyrius 6.5.1 escalated a wrong argument
  count from a warning that still emitted a binary to a hard error that emits nothing. The affected
  calls — `codec_process_message` 2-of-3 (×7) and `bridge_process_message` 2-of-3 (×4) in
  `tests/bote.tcyr`, `auth_bearer_check` 5-of-6 (×4) in `tests/bote_auth.tcyr` and (×2) in
  `tests/bote.bcyr` — had been running with the trailing parameter bound to whatever occupied the
  register. All now pass the `claims` / `claims_out` argument explicitly as `0`, matching every
  `src/` call site. **`src/` itself was already correct**; this was test-suite rot only, and it means
  the pre-3.2.0 assertions on those paths were passing against an unbound parameter.
- **`[deps.sigil]` tag realigned with `cyrius.lock`.** 3.1.4 shipped `tag = "3.12.0"` while
  `cyrius.lock` already recorded 3.12.1's content hash (`292e0a2d…`); the local `path = "../sigil"`
  override vendored the 3.12.1 body and masked the drift. A clean `git`+`tag` CI checkout would have
  vendored 3.12.0 (`c36d26a0…`) and **failed `cyrius deps --verify`** — i.e. CI was red at HEAD, for
  the same reason and in the same shape as the libro drift fixed at 3.1.4. Tag, lock and vendored
  body now agree for all four AGNOS deps.
- **aarch64 cross-build (`undefined variable 'SYS_OPEN'`).** Resolves
  `docs/development/issues/archive/2026-07-17-aarch64-sys-open-urandom.md`, filed by **daimon**. The
  fix is **wider than the report**: the issue identified `SYS_OPEN` as "the *only* blocker", but
  set-differencing the two syscall peers (95 constants on x86_64, 88 on aarch64) shows 17 x86-only
  constants, of which bote referenced **three** — `SYS_OPEN` (`src/session.cyr`, `src/pkce.cyr`)
  plus `SYS_UNLINK` and `SYS_CHMOD` (`src/transport_unix.cyr:99` / `:109`, both missed by the
  report, both in the shipped bundle). An `undefined variable` is fatal, so fixing only `SYS_OPEN`
  would have moved the failure two lines down, not to green.
  - Both entropy sites now use **`random_bytes()`** (getrandom(2)) rather than the report's
    suggested `sys_open`. `lib/random.cyr:3` recommends it over `/dev/urandom` + open/read/close
    outright, `lib/io.cyr:68` warns that raw `sys_open` is ABI-wrong on agnos (a silent
    miscompile, not a build error), and `SYS_GETRANDOM` is defined on both arches. One syscall
    instead of three, internal short-read looping, and it works where `/dev/urandom` is not
    reachable (early boot, chroot, landlocked process). The 1.9.4 audit M2 fail-closed contract —
    refuse to mint a session ID rather than ship a guessable one — is preserved exactly.
  - `src/transport_unix.cyr` routes through `sys_unlink` / `sys_chmod`, which expand to
    `unlinkat(AT_FDCWD, …)` / `fchmodat(AT_FDCWD, …)` on aarch64.
  - Two hardcoded numeric syscalls in `_gen_session_id` (`syscall(1, 2, "fatal: …", 47)` — the
    **x86** write number) became `eprint(msg, len)` / `sys_exit(90)`. They worked on aarch64 only
    because the cyrius backend renumbers `x8=1 → 64`; correctness should not rest on a
    compiler-internal translation table.
  - **Verified end to end**, not merely linked: all three binaries cross-build to real
    `EM_AARCH64` ELF, and the **full suite — 13 files, 786 assertions — passes under
    `cyrius test --aarch64`**.
  - Still open by choice: `src/transport_unix.cyr:123` uses `SYS_ACCEPT` from `lib/net.cyr`'s
    unguarded `var SYS_ACCEPT = 43` (the x86 number), correct on aarch64 only via the same
    backend renumbering. Not a build blocker, so out of scope here; `sock_accept()` is the
    consistent replacement and arguably an upstream `net.cyr` fix.

### Changed
- **cyrius pin `6.4.66` → `6.5.3`.** Clears the "manifest-pin: 6.4.66 (drift — wrapper is 6.5.3)"
  warning and satisfies every dep's declared toolchain floor (sakshi 2.4.7 declares the highest at
  6.5.0). Two capacity ceilings moved inside the window: 6.4.75 raised `fn_table` 8192 → 32768 and
  6.4.76 raised the identifier pool 256 KB → 512 KB in place, so `src/main.cyr` now reads
  **4961/32768 fn_table (15%) and 163966/524288 identifiers (31%)**, against 4879/8192 and
  161809/262144 (60% / 62%) at 3.1.4. The CI capacity gate needs no change — it parses the
  denominator out of the `cyrius stats:` block rather than hardcoding it.
- **libro `2.8.2` → `2.8.4`**, **majra `2.5.1` → `2.5.3`**, **sigil `3.12.0` → `3.12.1`**,
  **sakshi `2.4.6` → `2.4.7`**. Zero API change across bote's adapter surface: libro adds three
  functions (`chain_new_streaming` / `entry_free` / `_chain_retain`) and removes none, majra's
  signature set is byte-identical, sigil adds only `_sigil_ensure_slot`, sakshi renames two private
  internals. patra does not move (1.12.12); libro 2.8.4 pins the same sigil 3.12.1 / patra 1.12.12
  as 2.8.2.
  - libro 2.8.4 appends a 5th field to `struct chain` (`streaming`, 32 → 40 bytes). `src/libro_tools.cyr`
    reads libro structs by raw offset; the field is **appended**, so `_lt_chain_entries(c) = load64(c)`
    at +0 is unaffected. Comment updated; the four stale `lib/libro.cyr:NNN` line citations that had
    already rotted at 2.8.2 are removed in favour of struct names, which don't drift on a bump.
  - majra 2.5.3 changes pattern matching (`#`/`+` honoured only at whole-level positions). bote is
    unaffected — all 11 topics in `src/events.cyr` are fully-qualified 3-level strings and the only
    pattern in-tree is a whole-level trailing `#`. Its head-of-line-blocking fix is the one
    behaviourally visible win for consumers with a slow subscriber.
- **`_web_get_int` → `_web_arg_number`** (`src/web_tools.cyr`). cyrius treats a trailing
  `_int`/`_str`/`_cstr` as an overload-dispatch suffix, so `_web_get_int(args, key)` occupied the
  `_int` slot of `_web_get(url)` at a different arity — the exact landmine sakshi 2.4.7 defused in
  its own `_sk_write_int`/`_sk_write_str` pair. Dormant here (both `_web_get` call sites pass a cstr
  as arg 1), but the arity mismatch is what arms it. Module-private; no consumer surface.

### Performance
- **Bump is performance-neutral — proven, because the raw numbers say otherwise.** The v3.2.0
  `benches/history.log` block reads 1.5–3.7× slower than v3.1.4 across all 14 benchmarks. A
  controlled A/B/C on the same host in the same hour — identical bench source, `cyrius build
  --no-deps` + direct run — isolates it: **A** (cycc 6.4.66 + stdlib 6.4.66, the exact 3.1.4 world)
  **2.924 µs** `dispatch_initialize` / **1.347 µs** `auth_bearer_check_unset`; **B** (cycc 6.5.3 +
  stdlib 6.5.3) **2.864 / 1.333 µs**; **C** (cycc 6.5.3 + stdlib 6.4.66) **2.901 / 1.347 µs**. A ≡ B ≡ C
  within run-to-run noise, and the 6.4.66 control reproduces the *slow* numbers — so the level shift
  is the host, not the bump. `lib/bench.cyr` and `lib/chrono.cyr` are byte-identical between the two
  snapshots (`diff -q`, rc=0), ruling out a measurement-harness change. Full control table is
  recorded inline in `benches/history.log`. Compare v3.2.0 forward against v3.2.0 until the host
  baseline is re-established.
- **6.4.80 constant-fold defect does not reach bote.** cyrius 6.4.80 fixed a CRITICAL silent
  wrong-value bug (a literal subtraction with a negative intermediate discarding its left operand,
  e.g. `1 - 2 + 3` folding to 5) that was live at bote's previous 6.4.66 pin — and CI builds at the
  manifest pin. A comment- and string-stripped scan for three-term literal `+ - & | ^` expressions
  found **0 matches** in `src/*.cyr`, `tests/*.tcyr` and `tests/*.bcyr`, and 37 in the vendored
  `lib/*.cyr` of which **0** have a negative literal intermediate. Recorded because "our tests
  passed" is not evidence against a defect whose failure mode is a silently wrong value.

## [3.1.4] — 2026-07-17 — libro 2.8.2 (`LIBRO_ERR_*`) + `[deps.libro]` pin/lock realign

**libro dep bump.** `[deps.libro]` `2.8.1 → 2.8.2`. libro 2.8.2 namespaces its own `LibroErr`
enum — the upstream reciprocal of bote's `BOTE_ERR_*` rename at 3.1.3 — so the bare `ERR_IO` /
`ERR_JSON` collision is now resolved at the source on **both** sides of a libro-linked bote binary,
not merely avoided by bote's prefix. No bote logic change; wire contract identical.

### Changed
- **libro `2.8.1` → `2.8.2`** (`[deps.libro]` tag). Upstream, libro 2.8.2 renamed its `LibroErr`
  enum `ERR_* → LIBRO_ERR_*` (13 constants, e.g. `LIBRO_ERR_IO=3`) and moved its own internal deps
  to sigil 3.12.1 / patra 1.12.12 under a cyrius 6.4.66 pin. For bote this is transparent: the
  `audit_libro` / `libro_tools` adapters call libro's **function** API, not its error constants, so
  no bote source references the renamed tags. bote's vendored `lib/patra.cyr` is already 1.12.12 and
  bote keeps its own `[deps.sigil]` 3.12.0 pin, so nothing else in bote's dep set shifts. Rebuilt +
  retested against 2.8.2; `dist/bote.cyr` + `dist/bote-core.cyr` regenerated at v3.1.4.

### Fixed
- **`[deps.libro]` tag realigned with `cyrius.lock`.** 3.1.3 shipped with the tag reading `2.8.1`
  while `cyrius.lock` already recorded 2.8.2's content hash (`0fd0ff08…`) — the local
  `path = "../libro"` override had vendored the 2.8.2 body, masking the drift locally. A clean
  `git`+`tag` CI checkout would have vendored 2.8.1 (`7187f5b6…`) and **failed lock verification**.
  Bumping the tag to `2.8.2` makes tag ↔ lock ↔ vendored body agree (all three hashes verified
  equal); `cyrius.lock` itself is unchanged because it was already correct.

## [3.1.3] — 2026-07-17 — toolchain 6.4.66 + `BoteErrTag` namespacing (`BOTE_ERR_*`)

**Toolchain bump + error-tag namespacing.** The cyrius wrapper had already rolled to 6.4.66,
leaving the manifest pin (6.4.64) drifting and the local `lib/` snapshot stale enough that the
server binary no longer linked. This release re-pins to 6.4.66, re-syncs `lib/`, and namespaces
bote's `BoteErrTag` constants to escape a bare-token collision with libro. No transport, dispatch,
or wire-format behavior change; numeric error values and JSON-RPC code mapping are identical.

### Changed
- **cyrius pin `6.4.64` → `6.4.66`** — clears the "manifest-pin: 6.4.64 (drift — wrapper is
  6.4.66)" warning. Re-syncing the declared stdlib subset (`cyrius deps` + `cyrius lib sync`)
  refreshes `lib/thread_local.cyr` to the 6.4.65 **slot allocator** (`thread_local_alloc`) that
  sigil 3.12.0's `crypto_scratch` and patra 1.12.10's TLS slots now call — without it every binary
  failed to link with `undefined function 'thread_local_alloc'`. `cyrius.lock` picks up the
  refreshed `bayan 1.1.0 → 1.2.0` and `patra` snapshot hashes from the same sync; also clears the
  `./lib/` shadow warning. 6.4.66 itself is a Win64-only ≥10-arg call fix (per the 6.4.64 span) —
  inert on this target.

### Fixed
- **`BoteErrTag` constants namespaced `ERR_*` → `BOTE_ERR_*`** (all 13 tags,
  `BOTE_ERR_TOOL_NOT_FOUND` … `BOTE_ERR_TAG_COUNT`). The bare tags collided with `lib/libro.cyr`'s
  own bare `ERR_IO` (=3) / `ERR_JSON` (=4) — bote's are `=11` / `=10`. In the default binary and
  full bundle (which pull libro in via `libro_tools`), cyrius is single-pass and "last definition
  wins", so the shared token resolved to whichever module was included last — leaving
  `bote_err_rpc_code` / `bote_err_format` able to map an IO/JSON error against the wrong tag value.
  Namespacing removes the shared-symbol hazard entirely. Numeric tag values (0–12) and the
  JSON-RPC code mapping are unchanged, so the wire contract is identical. These are internal
  handler/dispatch/registry tags — no first-party consumer references the bare names; any
  out-of-tree consumer that referenced them must prefix `BOTE_ERR_`. `dist/bote.cyr` +
  `dist/bote-core.cyr` regenerated (53 tags each, 0 bare `ERR_`).

## [3.1.2] — 2026-07-16 — toolchain 6.4.64 + dependency refresh (libro 2.8.1, majra 2.5.1, sigil 3.12.0, sakshi pin)

**Toolchain + full dependency refresh.** No bote logic change — the release moves every pin to
its current tag, clears two per-build toolchain warnings (manifest-pin drift, sakshi lib-freshness
shadow), and banks upstream correctness fixes bote inherits: libro's audit-row quoting fix and
sigil's crypto-bank thread-local slot fix.

### Changed
- **cyrius pin `6.4.34` → `6.4.64`** — clears the "manifest-pin: 6.4.34 (drift — wrapper is
  6.4.64)" warning. Span highlights relevant to bote: `net.cyr` `sock_accept` per-poll alloc-leak
  fix (6.4.61 — all four socket transports sit on `net`), sandhi `signal_ignore`/SIGPIPE +
  `output_buf` 16 MiB → 1 GiB (6.4.51/.52), per-profile distlib `.deps` subsetting (6.4.48),
  DX diagnostics — column numbers, source carets, multi-error reporting (6.4.60/.62), and the
  lib-freshness shadow check (6.4.63) that surfaced the sakshi lag below. 6.4.64 itself is a
  Win64-only ≥10-arg call fix, inert on this target. `cyrius.lock` now records the
  declared-subset resolution (the 6.4.x `cyrius deps` behavior).
- **libro `2.7.10` → `2.8.1`.** 2.8.0 is a toolchain/dep refresh in which libro thinned its own
  sigil surface (its binary, not the `dist/libro.cyr` body bote consumes — that is unchanged);
  2.8.1 fixes `patrastore_append` to use a bound (prepared-statement) INSERT, so a `'` in any
  audit field no longer silently drops the row (integrity fix on the audit chain bote fronts via
  `libro_tools`). libro's `.deps` sidecar now also pulls **patra 1.12.10** — a new transitive
  `lib/patra.cyr` vendored by `cyrius deps` (the tests that include `lib/libro.cyr` already
  include it).
- **majra `2.5.0` → `2.5.1`** — toolchain/dep refresh; the dist bundle body is byte-identical
  (banner restamp only).
- **sigil `3.9.8` → `3.12.0`.** Banks 3.9.9's **crypto-bank thread-local slot fix (slot 0 → 8)**
  — slot 0 collided with patra's SQL scratch, corrupting banked crypto state in any process
  linking both, which bote now does (patra arrives via libro 2.8.x above). 3.10.x–3.12.0 add
  Authenticode/UEFI signing, per-primitive distlib profiles, and BLAKE2b + Argon2id — additive;
  bote's consumed surface (sha256 / hmac_sha256 / ed25519) is unchanged. The full
  `dist/sigil.cyr` bundle is retained: bote consumes two overlapping primitive families
  (SHA-2/HMAC + Ed25519), and majra 2.5.1's footprint review showed the thin per-primitive
  profiles overlap on ~121 fns for that shape (noisier than the full bundle's deduplicated
  closure).
- **6.4.x cyrfmt reflow** of `src/fs_tools.cyr` + `src/web_tools.cyr` (whitespace-only
  continuation-line indentation — same reflow class as the 2.7.6 formatter churn).
- `_bote_server_version()` → `"3.1.2"` (`src/dispatch.cyr`); `dist/bote.cyr` (6538 lines) +
  `dist/bote-core.cyr` (2569 lines) regenerated at v3.1.2.

### Added
- **Explicit `[deps.sakshi]` pin at `2.4.6`** (`dist/sakshi.cyr`, self-contained — links only
  stdlib `fnptr` + `atomic`). The stdlib registry still resolves `sakshi` → 2.4.3, which tripped
  6.4.63's lib-freshness warning on every build. Same registry-lag class as the sigil pin;
  brings sakshi 2.4.4's custom headers + W3C 128-bit trace-id along the way.

### Notes
- The pre-existing benign linker diagnostics are unchanged: `duplicate fn '_sub_new'`
  (lib/majra.cyr) and `duplicate fn 'cancel_token_new'` — the latter now root-caused as stdlib
  `async.cyr`'s own cancel-token (present since at least 6.4.34, semantically identical to
  `src/stream.cyr`'s: an 8-byte heap flag initialised to 0; bote's later definition wins).
- Verified: **786/786 assertions** across 12 test files + the core-only drift smoke; **14
  benchmarks** flat vs the 2.7.6 baseline (`schema_compile_nested` 9 → 7.5 µs; first entry
  logged at the 6.4.x sub-µs bench precision); capacity **59% / 61%** (`fn_table 4841/8192`,
  `identifiers 160696/262144`) — well under the 95% CI gate; `fmt` / `lint` / `vet` / `deny`
  clean; stdio `initialize` + `tools/call` round-trip smoke on the shipped binary
  (`serverInfo.version` = 3.1.2).

## [3.1.1] — 2026-07-09 — native HTTPS large responses (toolchain TLS fix), pin `6.4.34`

**`web_fetch` / `web_search` now work over the sovereign native TLS backend for
real-world (large) HTTPS responses.** 3.1.0 shipped the web tools calling the
sandhi client, which uses the default **native** TLS backend — but the stdlib
native TLS record layer had a size-dependent bug: any response whose body arrived
in a full 16 KB TLS record failed (`example.com` slipped through; anthropic.com /
cyriusb.com / secureyeoman.ai / robertmaccracken.com did not). No bote source
change was needed — the fix is entirely in the toolchain's stdlib TLS module,
picked up by the pin bump.

### Changed
- **cyrius pin `6.4.20` → `6.4.34`**, which carries the native TLS record-layer
  fix: (1) an off-by-one in the record-decrypt output buffer (a max-size TLS 1.3
  record's inner plaintext is `content(2^14) + type-byte` = 16385, above the old
  16384-byte scratch → `TLS_ERR_BUFFER_FULL` after the AEAD sequence advanced, a
  connection wedge), and (2) `tls_native_read` now delivers **partial records**
  (holding any remainder) instead of "whole record or error", so a caller reading
  in sub-record chunks no longer loses data. `web_fetch` verified over the native
  backend against all four hosts above (200, byte-lengths identical to libssl).

_No `web_tools.cyr` change_ — the earlier local libssl-backend fallback WIP was a
workaround for this now-fixed root cause and was dropped (never released).

## [3.1.0] — 2026-07-09 — web tools (web_fetch / web_search)

**Web MCP tools.** A new tool family, `src/web_tools.cyr`, alongside `fs_tools` / `libro_tools`, so an MCP
client (thoth, via daimon) can fetch pages and search the web through the spine — no reimplementation in the
consumer. Additive; registered by `web_tools_register()` and shipped in the default `dist/bote.cyr` bundle.
Outbound HTTP uses the sandhi client (the same transport the rest of AGNOS speaks). 25 assertions.

### Added
- **`web_fetch {url}`** — GET an `http://` / `https://` URL and return its **readable text**: HTML tags are
  stripped, `<script>` / `<style>` blocks dropped, the common named + numeric entities decoded, and runs of
  whitespace collapsed. Output is size-capped (64 KiB); non-http schemes are refused; a transport or non-2xx
  failure is reported, not faked.
- **`web_search {query, count?}`** — query a **SearXNG** instance (`BOTE_SEARXNG_URL`, configurable,
  self-hostable, no third-party key) via its JSON API and return the top results (title · url · snippet),
  parsed with bayan. An unset endpoint is reported honestly — no silent default to a third party.
- Registered in `src/main.cyr` (after `fs_tools_register`) and the `[lib]` distlib manifest. Tests in
  `tests/bote_web_tools.tcyr` cover the scheme guard, the HTML→text stripper, url-encoding, and entity decode.

### Security
- The HTML→text stripper **drops C0 control bytes / DEL / raw NUL** (both literal and `&#<n>;`-decoded) from
  the untrusted page — readable text has none, and it keeps `web_fetch`'s result valid JSON downstream (the
  shared escaper does not `\u`-escape control bytes, and a raw NUL would silently truncate the text).
  Adversarial-review-caught (untrusted-input lens); 27 assertions incl. the control-byte/NUL case.

## [3.0.1] — 2026-07-07 — bote_echo MCP conformance + toolchain 6.4.20

### Fixed
- **`bote_echo` now returns an MCP-conformant result.** The reference sample tool returned its
  arguments verbatim (a bare JSON object), which is not a valid `tools/call` result — a strict MCP
  client that reads `content[0].text` (e.g. thoth's `/call`, and the agentic tool loop) found none and
  rendered "no text content could be parsed". `bote_echo_handler` now wraps the echoed arguments in a
  text content block via the existing `content_text_response` helper —
  `{"content":[{"type":"text","text":"<args>"}]}`. The real `fs_*` tools were already conformant; only
  the echo sample was not. (`src/main_common.cyr`.)

### Changed
- **Toolchain pin `6.3.42 → 6.4.20`** (`cyrius.cyml` + `cyrius lib sync`, 56 floor modules). Clears the
  drift warning; all three binaries (bote / bote-streamable / bote-ws) build and the full test suite
  passes on the new toolchain.

## [3.0.0] — 2026-07-03 — MCP capability suite + honest polled-push notifications

The **3.0.0** milestone rounds out bote's MCP surface. bote went from three
methods (`initialize` / `tools/list` / `tools/call`) to a server advertising
four derived capabilities — **tools, prompts, resources, completion** — plus a
working **polled server→client notification path** (`notifications/tools/
list_changed` + `notifications/prompts/list_changed`), delivered on the client's
next streamable `GET` or piggybacked on a POST. Every capability is advertised
only when it can actually be honored: `listChanged` appears solely on the
streamable transport (which has a drain path), never on stdio/http/ws. All
growth is additive on the 2.0 handler ABI (`fn h(args, claims) → result`),
needs no new dependencies, and each internal struct grew by appended slots only.
Test surface: **733** assertions (was 653 at 2.9.0) + the core-only drift smoke.
The `[lib.core]` bundle grew 9 → 11 modules (`prompts.cyr`, `resources.cyr`).

### Breaking
- **`bote-streamable` now enforces MCP session lifecycle.** The shipped
  streamable binary configures a `SessionStore` (to back the per-session
  notification buffers), so `initialize` mints an `MCP-Session-Id` and every
  subsequent request — including the `GET` SSE stream — must present it; a
  session-less non-`initialize` request returns `404`. **Migration:** clients
  must perform the `initialize` handshake and echo the returned `MCP-Session-Id`
  header on all following requests (standard MCP 2025-11-25 session management).
  Consumers embedding the transport *library* are unaffected — the session store
  stays opt-in via `streamable_config_with_session_store`.

### Changed
- **Toolchain pin `6.3.38` → `6.3.42`** (`cyrius.cyml`; `cyrius lib sync --full`
  re-vendored the gitignored `lib/` and `cyrius deps` re-locked — only 3 stdlib
  hashes moved: `sigil` / `protobuf` / `sankoch`, the rest byte-identical). Full
  suite (733) green, all 13 benchmarks run clean (dispatch ~3–7 µs, jsonx
  sub-µs — no regression), and the SIGILL-sensitive crypto path (jwt/pkce/auth +
  the startup `chain_new` → sha256) was re-verified on the new toolchain via a
  runtime binary smoke.

### Fixed
- **`tests/bote.bcyr` compile** — the benchmark unit was missing the
  `prompts.cyr` / `resources.cyr` includes ever since those modules landed (it
  compiles `dispatch.cyr`, which now references them, so it failed with 13
  undefined functions). Surfaced by running the benchmark suite for the 3.0.0
  release; fixed by adding the includes in dependency order.

### Added
- **POST-piggyback SSE** — bite 6 (optional) of the server→client push path. When
  a `POST` response is built and the requesting session has pending notifications
  **and** the client's `Accept` allows `text/event-stream`, the transport answers
  that POST with an SSE stream (the JSON-RPC response as an id-less `message`
  frame, then the drained notifications) instead of `application/json` — so a
  client that never opens a GET stream still receives server notifications.
  Spec-allowed (MCP Streamable HTTP) and gated three ways (`Accept`, a resolved
  session, non-empty buffer), so it's a pure add-on: with no pending events the
  POST returns `application/json` exactly as before (regression-verified on the
  wire). The response frame is deliberately id-less (a synthetic id would
  pollute the client's `Last-Event-ID`; only the drained notifications carry
  resumable ids). New helpers `_strm_accepts_sse` / `_strm_post_session_outbound`
  / `_strm_response_frame`; +4 assertions in `tests/bote_streamable.tcyr`
  (49 → 53). Full-bundle-only (`dist/bote-core.cyr` unchanged).
- **`tools`/`prompts` `listChanged` — advertised, honestly** (bite 5, the payoff
  of the push path). `_build_capabilities` now emits `listChanged:true` on the
  `tools` and `prompts` capabilities **only when `dispatcher_notifications(d)`
  is set** — a new `+64` dispatcher flag (`dispatcher_set_notifications`) that a
  transport turns on **only if it actually has a client-drain path**.
  `main_streamable` sets it; `bote`/`bote-ws` do not. Result, verified on the
  wire: `bote-streamable`'s `initialize` advertises
  `{"tools":{"listChanged":true},"prompts":{"listChanged":true},…}` while the
  default `bote` advertises `{"tools":{},"prompts":{},…}` — same core
  `_build_capabilities`, different truth per binary, so no transport ever
  promises notifications it can't deliver. Also wires the **prompts** producer:
  a new `TOPIC_PROMPT_REGISTERED` event (`dispatcher_register_prompt` now emits
  it; `strm_notify_sink` maps it to `notifications/prompts/list_changed`). The
  stale `_server_emits_tool_list_changed` stub is gone, closing the
  `dispatch.cyr` capability-honesty note for tools + prompts. `resources`
  `listChanged`/`subscribe` and `logging` stay off (no producer). Dispatcher
  grew one slot (64 → 72 bytes). +1 assertion in `tests/bote.tcyr` (414 → 415),
  +3 in `tests/bote_streamable.tcyr` (46 → 49). **This completes the honest
  polled-push MVP** (delivery on the client's next GET); real-time held-open
  push remains the separate, threaded, phase-2 item.
- **`tools/list_changed` producer** — bite 4 of the server→client push path (the
  producer that makes the notification actually flow). A client-notification
  `EventSink` (`strm_notify_sink(store)` in `src/transport_streamable.cyr`) wraps
  the `SessionStore`; on a `TOPIC_TOOL_REGISTERED`/`_DEREGISTERED` event it
  **broadcasts** `notifications/tools/list_changed` into every active session's
  outbound buffer (each with that session's own event id), for delivery on the
  next SSE GET. Non-tool-set topics (completed/failed/deprecated — audit events)
  are ignored. No `dispatch.cyr` change and no second sink slot needed: register/
  dereg already publish to the dispatcher's `EventSink`, and `main_streamable`
  wasn't using that slot. `main_streamable.cyr` now wires a `SessionStore` +
  `dispatcher_set_events(strm_notify_sink(store))` and configures the store on
  the transport — which also **activates MCP session enforcement** (`initialize`
  mints an `MCP-Session-Id`; other requests, incl. the GET stream, must present
  it — verified on the wire: init returns a session id, a session GET streams,
  a session-less GET → 404). The `list_changed` path is proven by a full
  `dispatcher_register_tool` → sink → buffer test; it stays dormant in the
  reference binary only because nothing dynamically (de)registers there. The
  capability is **not advertised yet** — that is bite 5, gated on this working.
  +5 assertions in `tests/bote_streamable.tcyr` (41 → 46).
- **Unconditional GET drain** — bite 3 of the server→client push path.
  `_strm_handle_get` now resolves the client's per-session `SessionOutbound`
  buffer (from a valid `MCP-Session-Id` when a session store is configured; else
  the process-global buffer for back-compat) and drains it over the SSE stream:
  strictly after a non-empty `Last-Event-ID` cursor, else **everything currently
  buffered** — the "deliver on the client's next GET" model. The drain selection
  is a pure `_strm_drain_events(rbuf, last_id)` (unit-testable without a socket;
  empty cursor treated as absent → drain-all, not `events_after("")`). **The
  primer is now an SSE comment** (`: bote stream open`) instead of an
  id-carrying event: a synthetic primer id is in no buffer, so a client that
  adopted it as its `Last-Event-ID` would `events_after`-miss and silently resume
  nothing — the comment carries no `id:`, so per the SSE spec it never becomes
  the client's cursor. Ships inert in the shipped `bote-streamable` (no session
  store configured yet ⇒ storeless fallback, empty drain) and until a producer
  feeds a buffer. Verified on the wire (GET emits the comment primer + retry, no
  crash). +5 assertions in `tests/bote_streamable.tcyr` (36 → 41).
- **Per-session outbound notification buffer** — bite 2 of the server→client
  push path. `McpSession` gains an **opaque** `+32 outbound` slot (32 → 40
  bytes; `mcp_session_outbound` / `mcp_session_set_outbound`) that `session.cyr`
  never dereferences — the buffer types live in `transport_streamable.cyr`
  (streamable binary only) while `session.cyr` is in all three binaries, so the
  slot stays type-agnostic. The streamable transport adds a `SessionOutbound`
  bundle (a `ResumptionBuffer` + its **own** `EventIdGenerator`) and a lazy
  `_strm_session_outbound(session)` accessor that stashes it in the slot. The
  per-session id generator is the fix for a real resumption bug: event IDs must
  not straddle sessions, since `resumption_buffer_events_after` matches an exact
  id and returns empty on a miss — a shared global counter would let one
  session's `Last-Event-ID` land in another's gap and silently resume nothing.
  No free path needed (`session.cyr` never frees `McpSession`; the bundle drops
  with the session). Ships unused — the buffer stays empty until the producer
  bite. Full-bundle-only (neither module is in `[lib.core]`), so
  `dist/bote-core.cyr` is unchanged. +4 assertions in `tests/bote.tcyr`
  (410 → 414), +11 in `tests/bote_streamable.tcyr` (25 → 36) covering per-session
  independence + id continuity.
- **Notification wire builders** (`src/stream.cyr`) — the first bite of the
  server→client push path. A generic `notification_wire(method, params)` emits
  the JSON-RPC notification envelope `{"jsonrpc":"2.0","method":…,"params":…}`
  (method JSON-escaped; raw params, `0` ⇒ `{}`), with thin wrappers
  `tools_list_changed_notification` / `prompts_list_changed_notification` /
  `resources_list_changed_notification` / `resource_updated_notification(uri)`.
  `progress_notification` now delegates to it (output byte-identical — the two
  existing assertions are unchanged). These produce wire bytes only; delivery is
  the transport's job (the streamable `ResumptionBuffer`, wired in a later bite).
  Dep-free, additive, full-bundle-only (`stream.cyr` is not in `[lib.core]`), so
  `dist/bote-core.cyr` is unchanged. +6 assertions in `tests/bote.tcyr`
  (404 → 410). Groundwork for the honest polled-push MVP (`*/list_changed`);
  no capability is advertised yet — that waits until a drain path exists.

### Changed
- **Roadmap: dropped the stale "blocked on cyrius `lib/thread.cyr` MPSC"
  framing** for streaming dispatch. `lib/thread.cyr` (MPSC + mutex) and
  `lib/async.cyr` are complete and pinned — server→client notifications are
  unwired-**by-choice**, not gated on cyrius. A dep-free polled-push MVP (buffer
  at produce time, drain on the client's next streamable `GET`) needs no threads;
  only real-time held-open streaming does (the single-threaded sandhi accept loop
  would otherwise deadlock).

  single completion handler on the dispatcher (`+56 completion_handler`, 56 → 64
  bytes) set via `dispatcher_set_completion`; the `completions` capability is
  **derived** from its presence (`{"tools":{},…,"completions":{}}`). Handler ABI
  `fn h(params_cstr, claims) → result_cstr` — it receives the raw
  `{ref, argument, …}` params and returns the full `{"completion":{"values":[…]}}`
  body, so bote does no ref/argument interpretation (consumer completes against
  its own prompts/resources). Unknown/unset → `-32601`. No new module (unlike
  prompts/resources) — dispatcher slot only. A reference completion handler is
  wired in the binary family, proven on the wire. +6 assertions in
  `tests/bote.tcyr` (398 → 404), including the four-capability canonical-order
  check (`{"tools":{},"prompts":{},"resources":{},"completions":{}}`). Sixth bite
  of the secureyeoman → bote MCP bring-over.

  _Logging (`logging/setLevel` + `notifications/message`) is intentionally NOT
  in this bite: it exists only to gate server→client log push, which is not
  wired — advertising a `logging` capability would promise messages bote can't
  send. It lands with the notification/push work._
- **MCP resources capability** — `resources/list` + `resources/read` (new module
  `src/resources.cyr`). Same shape as the prompts capability: a
  `ResourceRegistry` (metadata for `resources/list` + a uri→read-handler map),
  a lazily-created dispatcher slot (`+48 resource_registry`, 48 → 56 bytes) via
  `dispatcher_register_resource`, and the `resources` capability **derived** from
  registry presence in `_build_capabilities` (`{"tools":{},"resources":{}}`; with
  prompts too → `{"tools":{},"prompts":{},"resources":{}}`). `resources/list`
  serializes each resource's `uri`/`name` (+ optional `description`/`mimeType`);
  read handlers use the ABI `fn h(uri_cstr, claims) → result_cstr` and return the
  full `{"contents":[…]}` body (bare MCP `ResourceContents`, not a content
  block). `subscribe`/`listChanged` are intentionally omitted — no server→client
  push path is wired. Unknown/absent capability → `-32601`. A reference
  `bote://info` resource is registered in the binary family, proven on the wire
  (`resources/list` → `resources/read`). `src/resources.cyr` is in both `[lib]`
  and `[lib.core]` (**core bundle 10 → 11 modules**). +11 assertions in
  `tests/bote.tcyr` (387 → 398). Fifth bite of the secureyeoman → bote MCP
  bring-over.
- **MCP prompts capability** — `prompts/list` + `prompts/get` (new module
  `src/prompts.cyr`). A `PromptRegistry` mirrors the tool registry (metadata for
  `prompts/list` + a name→generator-handler map for `prompts/get`), and the
  dispatcher gained one slot (`+40 prompt_registry`, 40 → 48 bytes) plus
  `dispatcher_register_prompt`, which **lazily** stands up the registry — so
  registering the first prompt is also what flips the `prompts` capability on at
  initialize. This is the first payoff of the 0b `_build_capabilities` seam: the
  `prompts` key is **derived** from actual registry presence (`{"tools":{}}`
  with no prompts; `{"tools":{},"prompts":{}}` once one is registered), never
  advertised blind. Prompt generator handlers use the 2.0 ABI
  (`fn h(arguments_cstr, claims) → result_cstr`) and return the full
  `{description?, messages:[…]}` body. `prompts/list` serializes each prompt's
  `name` / optional `description` / optional `arguments` (each arg's `required`
  emitted only when true). When no prompt registry exists, `prompts/*` fall
  through to `-32601` (matching the un-advertised capability). A reference
  `bote_greeting` prompt is registered in the binary family alongside
  `bote_echo`, proven on the wire (`initialize` → `prompts/list` → `prompts/get
  name=Ada` ⇒ a `content_text` "Hello, Ada!" message). `src/prompts.cyr` is in
  both `[lib]` and `[lib.core]` (**core bundle 9 → 10 modules**). +10 assertions
  in `tests/bote.tcyr` (377 → 387). Fourth bite of the secureyeoman → bote MCP
  bring-over.
- **Tool annotations now serialized in `tools/list`** (MCP 2025-11-25
  `ToolAnnotations`). The `ToolAnnotations` model + `tool_def_with_annotations`
  have existed on the registry since the annotations work landed, but the
  `tools/list` builder never emitted them — the hints were stored and never seen
  by clients. `_build_tools_list_result` (`src/dispatch.cyr`) now appends an
  `"annotations":{...}` object per tool via the new `_emit_annotations` /
  `_emit_hint_field` helpers, honoring the tri-state contract documented at
  `src/registry.cyr:16-17`: `readOnlyHint` / `destructiveHint` / `idempotentHint`
  / `openWorldHint` are each emitted as `true`/`false` only when set, and the
  whole object is omitted when the annotations ptr is null **or** every hint is
  unset. Purely additive — no annotations → byte-identical output to before, so
  the handler ABI and existing clients are unaffected. +4 assertions in
  `tests/bote.tcyr` (363 → 367) covering the read-only preset (all four hints),
  a single-hint tool, and both absence cases. First bite of the secureyeoman →
  bote MCP capability bring-over (annotations were modeled but unwired).

### Added
- **Tool profiles + `tools/list` profile filter** (`src/registry.cyr`,
  `src/dispatch.cyr`). Tools can carry opaque consumer-defined profile tags
  (e.g. `security` / `web` / `full`) via `tool_def_with_profiles(d, vec)`; a
  client may pass `{"profile":"<tag>"}` in `tools/list` params to receive only
  the tools tagged with that profile (exact-match membership via the new
  `tool_def_has_profile`). This is bote's `[lib.profile]` idea applied at
  runtime — the reusable primitive under secureyeoman's AGNOS-bridge profiles /
  smart-schema-delivery, with **no interpretation in bote** (an untagged tool is
  in no profile; `full` etc. are pure consumer convention). Fully **additive and
  opt-in**: `tools/list` with no `profile` param returns the complete list,
  byte-identical to before. `ToolDef` grew one appended slot (56 → 64 bytes,
  `+56 profiles`) — same additive pattern as `annotations`/`compiled`, so all
  existing field offsets and the builder/accessor API are unchanged. The
  `tools/list` builder now tracks an emitted-count for comma placement so
  filtered gaps can't corrupt the JSON array. +8 assertions in `tests/bote.tcyr`
  (369 → 377). Third bite of the secureyeoman → bote MCP bring-over.

### Changed
- **`initialize` capabilities extracted into a `_build_capabilities` seam**
  (`src/dispatch.cyr`). The `capabilities` object was a hardcoded string literal
  inside `_build_initialize_result`; it's now built by a dedicated helper whose
  advertised keys are **derived from what bote actually implements**, so future
  `prompts` / `resources` / `logging` / `completions` capabilities drop in gated
  on their own handler surfaces without touching the initialize body. Output is
  **byte-identical** (`"capabilities":{"tools":{}}`) — purely a refactor + a
  recorded decision. The **`tools.listChanged` decision is now explicit and
  pinned by a test**: it is deliberately **not** advertised because bote has no
  server→client push path (the `TOPIC_TOOL_REGISTERED/_DEREGISTERED` EventSink
  topics are bote's internal bus, not MCP `notifications/tools/list_changed`), so
  advertising `listChanged:true` would leave clients waiting for notifications
  that never arrive. A single predicate `_server_emits_tool_list_changed` flips
  it when a persistent push channel lands. +2 assertions in `tests/bote.tcyr`
  (367 → 369). Second bite of the secureyeoman → bote MCP bring-over (the
  capabilities enabler for prompts/resources/completion).

## [2.9.0] — 2026-07-03 — runs + serves MCP on agnos (cyrius 6.3.38)

### Changed
- **Toolchain migrated `6.3.15` → `6.3.38`** (`cyrius.cyml` pin; `cyrius lib sync
  --full` re-vendored the gitignored `lib/` from the current stdlib snapshot).
  Host suite green (25/22/… all files, 0 failed).

### Fixed
- **bote now runs on agnos** (was: SIGSEGV at startup on the `--agnos` build).
  Root cause was **not** bote or sigil — it was a **stale vendored stdlib
  `freelist.cyr`** carried by the old 6.3.15 pin. Pre-fix `freelist.cyr` used the
  Linux 6-arg `syscall(SYS_MMAP, 0, size, …)` on every target; on agnos `mmap#27`
  takes the length in arg1, so the leading Linux addr-hint `0` was read as the
  size → `mmap#27(0)` → `MAP_FAILED` (0) → the next `store64` SIGSEGV'd. That
  killed **every `fl_alloc` consumer** — all of sigil's crypto (sha256/hmac/
  ed25519/aes-gcm) — so bote died in `main()` at `chain_new()` → `sha256_init()`
  → `fl_alloc(144)`. cyrius already fixed `freelist.cyr` (a `_fl_mmap()` that
  dispatches `#ifdef CYRIUS_TARGET_AGNOS → syscall(SYS_MMAP, length)`); the
  6.3.38 migration picks it up. **Proven on agnos under mirshi** (AGNOS→Linux
  syscall translation): `bote-agnos` serves the full MCP flow — `initialize`
  (serverInfo bote 2.8.0), `tools/list` (9 tools), and `tools/call bote_echo`
  (executes, exercising the crypto path). **Also proven on the REAL agnos kernel
  under QEMU** (real `mmap#27`, not mirshi's host-kernel emulation): the new
  `BOTE_SELFTEST` kernel hook + `scripts/bote-mcp-smoke.sh` (in the agnos repo)
  pipe an MCP `initialize` + `tools/call bote_echo` into bote's fd0 via two kernel
  pipes and capture fd1 — serial shows bote's `serverInfo` reply and the echoed
  `agnos-kernel` argument, `exit 0`, no fault/panic.

## [2.8.0] — 2026-07-02 — filesystem tools (`fs_write` / `fs_read` / `fs_mkdir`)

A new `src/fs_tools.cyr` module adds three filesystem tools so an MCP client
(e.g. thoth, via daimon) can create a small file-based project end to end.
Registered on the `bote` binary alongside `bote_echo` and the five `libro_*`
audit tools and — like `libro_tools` — folded into the full `dist/bote.cyr`
bundle (`[lib] modules`), so downstream consumers can opt in by calling
`fs_tools_register()`. NOT in the transport-free `dist/bote-core.cyr`, and no
existing tool or ABI moved (minor: additive surface only).

### Added

- **`fs_write` / `fs_read` / `fs_mkdir`** (`src/fs_tools.cyr`) — MCP tools
  for writing, reading, and creating directories. `fs_write` creates missing
  parent directories and returns the bytes written; all three return proper
  MCP content blocks (`{"content":[{"type":"text",…}],"isError":…}`) so a
  client renders the result directly. `fs_write` JSON-unescapes its `content`
  argument (`\n`/`\t`/`\"`/`\\`/`\uXXXX`→UTF-8) so multi-line source files
  land byte-correct on disk.
- **`src/fs_tools.cyr` added to `[lib] modules`** — the tool module ships in
  the full `dist/bote.cyr` bundle (25 modules), mirroring `libro_tools`.
- **`tests/bote_fs_tools.tcyr`** — 26 assertions covering the path-safety
  guard, the JSON unescaper (incl. `\u` → UTF-8), tool registration, and the
  handler's refusal path.

### Security

- Filesystem access is **confined to a root** (`BOTE_FS_ROOT`, default `.`):
  an argument path that is absolute or contains a `..` segment is **refused**
  (`isError:true`, no I/O) — defense-in-depth beneath t-ron's per-tool
  authorization. The root confinement is a floor, not a full sandbox;
  registration is **opt-in** (`fs_tools_register`), so embedding the bundle
  does not expose filesystem writes until a consumer wires the tools in.

## [2.7.8] — 2026-07-01 — AF_UNIX transport fail-closes on agnos

AGNOS cross-build readiness. bote-core was already `--agnos`-clean; this
closes the full-server transport surface so the `bote` binary itself
compiles under `cyrius build --agnos`.

### Fixed

- **`--agnos` build**: `transport_unix_run` (`src/transport_unix.cyr`) is
  guarded with `#ifdef CYRIUS_TARGET_AGNOS` and fail-closes on agnos, which
  has no AF_UNIX domain sockets. Guarding the entry drops the whole AF_UNIX
  subtree (`SYS_SOCKET` / `BIND` / `CHMOD` / `LISTEN` / `ACCEPT` — unrelated
  syscall numbers on agnos) off the agnos target; bote's TCP / streamable / ws
  transports carry traffic there. Mirrors majra's ipc AF_UNIX guard.

## [2.7.7] — 2026-06-30 — cyrius 6.3.15 base-stack migration + 6.3.x stdlib rename reconciliation

Tier-3 step of the coordinated base-security-stack migration to cyrius
**6.3.15** (sakshi 2.4.3 → sigil 3.9.8 → majra 2.5.0 → libro 2.7.9 →
**bote 2.7.7** → the five consumers). Toolchain pin + dependency refresh
plus the stdlib boundary/rename reconciliation the 6.3.x line requires.
No bote runtime *logic* changed. All 653 assertions pass across all 11
test files on the new stack; `dist/bote.cyr` / `dist/bote-core.cyr`
regenerated at v2.7.7.

### Changed

- **Toolchain**: pinned to cyrius **6.3.15** (was 6.2.11).
- **Dependencies**: libro **2.7.9** (was 2.7.4), majra **2.5.0** (was
  2.4.7), sigil **3.9.8** (was 3.7.14) — the migrated tiers below bote.
- **`[deps] stdlib`**: added `atomic` + `sync` (patra's transitive
  `lib/sync.cyr` requirement surfaces through the libro chain on 6.3.x)
  and `dynlib` (6.3.x's `fdlopen.cyr` references `dynlib_auxv_get` /
  `dynlib_read_auxv`; `dynlib` must precede `fdlopen` — single-pass).
- **`http_send_204` → `sandhi_server_send_204`** (transport_http /
  bridge / transport_streamable): the retired bote-local `http_*` shim
  alias was still referenced at three 204-response sites; renamed to the
  canonical sandhi export (identical `(cfd, extra_headers)` signature).
- **`_bote_server_version()`**: corrected stale literal `2.7.1` → `2.7.7`
  (the MCP `initialize` handshake now reports the true release again).

### Fixed

- **`auth_bearer_check` undefined** in `tests/bote.tcyr`,
  `bote_streamable.tcyr`, `bote_ws.tcyr`: these files include the
  transports that call it but omitted `src/auth.cyr`, which compiled at
  6.2.x only because DCE treated the call site as unreachable. 6.3.x's
  reachability marks it live; added the `src/auth.cyr` include before the
  transport include in each (mirrors `src/main.cyr`).
- **`http_find_header` undefined** (WS handshake, via stdlib
  `lib/ws_server.cyr`): the 6.3.x stdlib renamed its `http_*` header
  helpers to `sandhi_server_*` but missed `ws_server.cyr`, leaving the
  symbol dangling. Added a bote-local compat shim in `src/transport_ws.cyr`
  forwarding `http_find_header(buf, blen, name)` →
  `sandhi_server_find_header(buf, blen, name)` (identical contract).
  Filed upstream against cyrius; remove the shim once `ws_server.cyr` is
  fixed.

## [2.7.6] — 2026-06-15 — cyrius 6.2.11 (first move onto the 6.2.x line) + dependency refresh

Toolchain + dependency refresh, ecosystem-wide 6.2.x sweep. No bote
source *logic* changed — pins, a required `thread_local` include guard,
and the 6.2.11 formatter reflow. All 653 assertions (+ 1 drift smoke)
pass on the new stack; all 14 benchmarks run clean; `dist/bote.cyr` /
`dist/bote-core.cyr` regenerated at v2.7.6.

### Changed

- **Cyrius pin `6.1.41` → `6.2.11`** (`cyrius.cyml [package].cyrius`).
  First step onto the 6.2.x maintenance line. The installed toolchain
  was already 6.2.11; this aligns the manifest pin and clears the drift
  warning.
- **Dependencies bumped to current tags:**
  - **libro `2.7.2` → `2.7.4`** (`[deps.libro]`) — toolchain refresh;
    `dist/libro.cyr` body byte-identical apart from the version header.
  - **majra `2.4.5` → `2.4.7`** (`[deps.majra]`) — toolchain refresh;
    bundle bodies byte-identical to 2.4.5.
  - **sigil `3.7.12` → `3.7.14`** (`[deps.sigil]`) — self-contained
    `dist/sigil.cyr` retained; transitive agnosys `1.3.2` → `1.4.3`.
- **`dist/bote.cyr` / `dist/bote-core.cyr` regenerated at v2.7.6.** The
  only body change is the 6.2.11 formatter's continuation-line reflow
  (whitespace) — no module moved, no symbol changed.
- **fn_table / identifier utilisation on `src/main.cyr`: 58% / 60%**
  (`fn_table 4770/8192`, `identifiers 157633/262144`) — the small bump
  over 2.7.5's `4764/157278` is the `thread_local` pull-through. Well
  under the 95% CI gate.

### Fixed

- **sigil 3.7.14 TLS-path SIGILL guard (latent CI landmine).** sigil
  3.7.14's `crypto_scratch` exercises the thread-local-storage path that
  3.7.12 never hit. Without `lib/thread_local.cyr` ahead of
  `lib/sigil.cyr`, the binaries and crypto tests link clean but **SIGILL
  at first crypto use (exit 132, `Illegal instruction`)** — a build-only
  check misses it; the harness must be *run*. Added `thread_local` to
  `[deps] stdlib` (immediately after `thread`, before `sigil`) so the
  three binaries pick it up via auto-injection, and an explicit
  `include "lib/thread_local.cyr"` after `lib/thread.cyr` in every
  sigil-using test file (`bote.tcyr`, `bote_jwt.tcyr`, `bote_pkce.tcyr`,
  `bote_libro_tools.tcyr`, `bote_streamable.tcyr`, `bote_ws.tcyr`).
  `bote_jwt` / `bote_pkce` also gained the `thread` include they
  previously did without. All 653 assertions pass exit-0; jwt (28) and
  pkce (17) confirm the crypto path runs clean. This is the exact
  failure mode libro 2.7.4's CHANGELOG documents.

### Notes

- **New benign 6.2.11 linker diagnostics.** The 6.2.11 linker now warns
  on duplicate global symbols — `duplicate symbol 'ERR_IO' / 'ERR_UNKNOWN'`
  (`lib/libro.cyr` / `lib/agnosys.cyr`) and `duplicate fn '_sub_new'`
  (`lib/majra.cyr`). Pre-existing name collisions between the bundled
  deps' error enums; harmless (last-definition-wins, all tests pass),
  simply silent before 6.2.11 began diagnosing them. Not a bote
  regression.
- **`cyrius fmt` CLI changed in 6.2.x.** The flag now follows the file:
  `cyrius fmt <file> --check` (was `cyrius fmt --check <file>` on 6.1.x).
  The 6.2.11 formatter also reflows deep continuation-line indentation to
  a fixed indent — hence the whitespace-only churn across `src/` and
  `tests/` in this release.

## [2.7.5] — 2026-06-11 — libro_tools folded back into the default binary + full bundle

### Added

- **libro audit tools in the default binary + full bundle.** The five
  `libro_*` MCP tools (`libro_query` / `libro_verify` / `libro_export` /
  `libro_proof` / `libro_retention`) are now registered on `build/bote`
  by default — `main()` stands up an in-memory libro chain via
  `chain_new()` and calls `libro_tools_init` + `libro_tools_register`.
  `src/libro_tools.cyr` also joins the `[lib]` profile, so it ships in
  `dist/bote.cyr` (now 24 modules). It is deliberately **not** in
  `dist/bote-core.cyr`: like `audit_libro` it depends on a live libro
  chain, which the transport-free core profile excludes.

### Changed

- Reverted the 1.9.4 cap-headroom decision that held `libro_tools` out
  of the default build. The 6.1.x function-table cap raise dropped
  `src/main.cyr` util to 58% / 60% (`fn_table 4764/8192`,
  `identifiers 157278/262144`), well under the 95% CI gate, so the
  headroom argument no longer applies. All 653 assertions (+ 1 drift
  smoke) pass; `tests/bote_libro_tools.tcyr` (22) exercises the
  re-included surface.

## [2.7.4] — 2026-06-11 — cyrius 6.1.41; tool-registry constructor renamed to resolve the ai-hwaccel `registry_new` collision

Toolchain patch refresh + a targeted breaking rename to unblock
multi-library consumers (szal, mihi, hoosh) that include both a bote
bundle and `dist/ai-hwaccel.cyr` in one compile unit. All 653
assertions (+ 1 drift smoke) that exercise the registry surface pass
on the renamed constructor.

### Breaking

- **`registry_new()` → `tool_registry_new()`.** bote's `ToolRegistry`
  constructor collided with ai-hwaccel's 32-byte profile-registry
  constructor of the same name (both export `fn registry_new()` with
  incompatible struct layouts — 24 vs 32 bytes). Cyrius include
  semantics are textual paste + last-definition-wins, so any consumer
  including both bundles silently got one `registry_new` and corrupted
  memory on the other's call sites. There is no include order that
  fixes it because ai-hwaccel's detection path calls its own
  `registry_new` internally. bote takes the rename to a lib-descriptive
  name — `tool_registry_new` parallels the existing `host_registry_new`
  (HostRegistry) constructor. Only the constructor changed; every other
  `registry_*` accessor keeps its name (none of them collide).

  **Migration:** replace `registry_new()` with `tool_registry_new()`.
  The struct layout, all accessors (`registry_register`, `registry_get`,
  `registry_list`, …), and the dispatcher wire-up are unchanged.

  See `docs/development/issues/archive/2026-06-11-registry-new-collision.md`.

### Changed

- **Cyrius toolchain pin: 6.1.24 → 6.1.41.** Patch-series bump within
  the 6.1.x line. No MCP wire-format, handler-ABI, or compile-cap
  change.

- **Stdlib dep migration for the 6.1.x consolidation
  (`[deps] stdlib`).** The 6.1.x toolchain folded the standalone
  `json` and `base64` stdlib modules into a single consolidated
  **`bayan`** module (JSON via `json_parse` / `json_get` back-compat
  shims, plus `base64` / `base64url`, csv, u128) and retired the
  standalone `bigint` module (sigil 3.x now bundles its own `u256` /
  `u384` inline; bote has no direct big-integer use). bote's
  `[deps] stdlib` and the test-file include headers moved
  `json` + `base64` → `bayan`, dropped `bigint`, and `src/registry.cyr`
  now `include`s `lib/bayan.cyr`. **This was the binding correctness
  fix:** the previous pin bump left a stale `lib/` (pre-6.1 `json.cyr` /
  `base64.cyr` / `bigint.cyr` snapshots) shadowing the version-matched
  toolchain libs — `cyrius deps` against 6.1.41 errors out on the
  removed module names until the manifest is migrated. (`ganita` in
  6.1.x is an unrelated linear-algebra module, **not** the bigint
  successor.)

- **Function-table + identifier-buffer utilisation: 52% / 52% →
  **58% / 60%** (`CYRIUS_STATS=1`: `fn_table 4740/8192`,
  `identifiers 156646/262144`) on `src/main.cyr`.** The rise is the
  `bayan` consolidation pulling in more surface than the old split
  `json` + `base64` modules; still well under the 95% CI gate.

- **sigil pinned explicitly via `[deps.sigil]` at tag 3.7.12** (git +
  `../sigil` path, single-file `dist/sigil.cyr` — same pattern as
  libro / majra), and `.cyrius-toolchain` marker corrected `4.8.4` →
  `6.1.41`.

### Fixed

- **CI build broke with `cannot open include file: src/sha_ni.cyr`.**
  The 6.1.x stdlib registry resolves the bare `sigil` stdlib dep to
  **3.7.10**, whose `dist/sigil.cyr` is *not* self-contained — it
  carries guarded `include "src/sha_ni.cyr"` / `include "src/aes_ni.cyr"`
  directives that dangle for single-file consumers (the toolchain
  *snapshot* ships the fixed 3.7.12, but `cyrius deps` pulls the
  registry version and ignores the lock for version selection).
  Pinning sigil to **3.7.12** via `[deps.sigil]` (which overrides the
  stdlib registry resolution) restores a self-contained bundle.
  Verified: `cyrius build src/main.cyr build/bote` → ELF, all 653
  assertions + drift smoke pass from a clean `cyrius deps`.

- **CI Build step masked compile failures.**
  `cyrius build … | tee build/build.log` returned `tee`'s exit 0 even
  when the compile failed, so a broken build passed the Build step and
  only surfaced two steps later as a confusing `build/bote: No such
  file`. Added `set -o pipefail` so the compile's status fails the step
  at its source.

- **`dist/bote.cyr` + `dist/bote-core.cyr` regenerated** for the
  constructor rename (`cyrius distlib` / `cyrius distlib core`).

## [2.7.3] — 2026-06-10 — cyrius 6.1.24 + libro 2.7.2 + majra 2.4.5; major-toolchain jump relieves the 5.10.x compile cap

Toolchain + dep refresh. The headline is the **cyrius major-version
jump (5.10.44 → 6.1.24)** — the 5.11.x migration that the 2.7.2
notes flagged as "planned" landed as 6.1.x instead. No MCP
wire-format change, no handler-ABI change, no `src/*.cyr` change.
All 653 assertions (+ 1 drift smoke) pass unchanged on the new
toolchain; the 14 criterion benchmarks show no regression
(`benches/history.log`, v2.7.3).

### Changed

- **Cyrius toolchain pin: 5.10.44 → 6.1.24.** Major-version jump.
  The expanded-source compile cap that forced the 2.7.2
  per-transport binary split is no longer the binding constraint —
  `src/main.cyr` builds clean. Function-table + identifier-buffer
  utilisation dropped from 93% / 92% (5.10.x) to **52% / 52%**
  (`CYRIUS_STATS=1`: `fn_table 4250/8192`, `identifiers
  137635/262144`) on the raised 6.1.x caps. The per-transport
  binary split (`bote` / `bote-streamable` / `bote-ws`) is retained
  for this release — reconsolidation to a single binary is now
  unblocked and tracked for a follow-up.

- **First-party dep pins, all bumped to latest released:**
  - **libro 2.6.3 → 2.7.2** — `dist/libro.cyr` refresh; audit-chain
    + `sha256_hex` surface unchanged at bote's `audit_libro` /
    `libro_tools` call sites.
  - **majra 2.4.4 → 2.4.5** — minor refresh; pubsub / counter
    surface unchanged at bote's `events_majra` adapter call sites.

- **`cyrius.lock` now carries full stdlib hashes (6 → 40 entries).**
  6.1.24's `cyrius deps` locks every resolved `lib/*.cyr`, not just
  the first-party dist bundles. CI's byte-clean lock gate covers the
  whole resolved set now.

- **CI / release toolchain installer rewritten for 6.x.** The
  hand-rolled two-tarball extraction in `.github/workflows/{ci,release}.yml`
  probed for `bin/cc5` — renamed to `cycc` in the 6.x toolchain, so the
  install gate failed outright. Both workflows now delegate to the
  upstream `scripts/install.sh` keyed on the `cyrius.cyml` pin (the
  pattern patra / agnosys already run on 6.x), which lays out
  `$HOME/.cyrius/{bin,lib}` including the stdlib snapshot. Toolchain
  verify step `cc5 --version` → `cyrius --version`.

### Removed

- **`cyrius audit` dropped from the cleanliness sequence.** Under
  6.1.x `cyrius audit` is the *toolchain* self-host gate
  (self-host + test + fmt + lint over the compiler), not a project
  dependency audit, and it fails outside the cyrius repo. The
  project-policy + dependency checks now run as `cyrius deny
  src/main.cyr` (policy) + the new `cyrius vet src/main.cyr`
  (include-dependency audit); `cyrius fmt --check` + `cyrius lint`
  are unchanged.

## [2.7.2] — 2026-05-11 — cyrius 5.10.44 + libro 2.6.3 + majra 2.4.4; per-transport binary split for 5.10.x cap

Toolchain + dep refresh, plus a structural workaround for the
cyrius 5.10.x 2 MB compile-source cap that the upgraded deps push
bote past. No MCP wire-format change, no handler-ABI change. The
default `bote` binary's CLI surface shrinks (streamable / ws moved
to siblings); the API surface in `src/*.cyr` is unchanged.

### Changed

- **Cyrius toolchain pin: 5.10.34 → 5.10.44.** Picks up the
  5.10.x stdlib + frontend deltas. Notable for bote: pulls
  `lib/slice.cyr` (now required transitively by libro 2.6.3 →
  agnosys for slice subscripts) and `lib/assert.cyr` (used by
  libro / majra).

- **First-party dep pins, all bumped to latest released:**
  - **libro 2.6.2 → 2.6.3** — `dist/libro.cyr` fixes the bare
    `ct_eq` call at the old call site (links against
    `ct_eq_bytes_lens` now). Closes the libro-side blocker
    phylax 1.1.1 documented from the consumer side.
  - **majra 2.4.3 → 2.4.4** — minor refresh; pubsub / counter
    surface unchanged at bote's `events_majra` adapter call sites.

- **Stdlib additions: `slice`, `assert`, `ct`, `keccak`, `random`.**
  Required by libro 2.6.3 + sigil 3.1.1 (PQ / AES-GCM surfaces).
  bote itself only consumes `sha256_hex`; the linker needs the
  symbols declared even if DCE prunes most call sites. See phylax
  1.1.1 CHANGELOG for the original pull-through write-up.

- **`ws_server` removed from `[deps] stdlib` auto-inject.** It
  lives in `src/transport_ws.cyr` + `tests/bote_ws.tcyr` and gets
  pulled in via manual `include "lib/ws_server.cyr"` from those
  files only. Keeps every other test / binary from carrying the
  11 KB ws-frame machinery they don't use.

### Added

- **`dist/bote-core.cyr` — opt-in core bundle.** Closes the
  consumer-side blocker tracked at
  `docs/development/issues/2026-05-10-opt-in-transport-profile.md`.
  Nine transport-free modules (`error`, `protocol`, `jsonx`,
  `registry`, `events`, `audit`, `dispatch`, `codec`, `schema`)
  packaged via `cyrius distlib core` (new `[lib.core]` profile
  in `cyrius.cyml`). 1989 lines / 70 KB — matches the issue's
  projected shape. Consumers wrap bote's dispatch surface but
  supply their own transport stack; the bundle excludes
  `transport_*` / `bridge` / `auth` / `session` / `discovery` /
  `content` / `host` / `audit_libro` / `events_majra`. **t-ron
  2.1.x is the trigger consumer** — its `cyrius.cyml` will flip
  to `modules = ["dist/bote-core.cyr"]` on the 2.7.2 bump,
  retiring the per-module pull workaround it shipped at 2.1.0.
  Drift guard: `tests/bote_core_only_smoke.tcyr` includes the
  bundle in isolation and runs a dispatcher + registry round-trip.
  See `DEPS-PATTERN.md` for the profile-selection contract.

- **`DEPS-PATTERN.md`** — distribution contract doc, modelled on
  libro's. Documents the dual-bundle shape (`dist/bote.cyr` +
  `dist/bote-core.cyr`), the `cyrius distlib` invocations, the
  profile-selection rule for downstream consumers, and the
  core-9 module list.

- **Per-transport binary split** (cyrius 5.10.x cap workaround):
  - `build/bote` — default. stdio + http + unix + bridge.
  - `build/bote-streamable` (new) — Streamable HTTP / SSE on
    port 8392 default. Built from `src/main_streamable.cyr`.
  - `build/bote-ws` (new) — WebSocket MCP on port 8393 default.
    Built from `src/main_ws.cyr`.
  Shared helpers (echo handler, env-driven bearer wiring, CSV
  token split, dispatcher constructor) live in
  `src/main_common.cyr`. `scripts/build-all.sh` builds the trio.
  Folds back into a single `build/bote` binary when bote migrates
  to cyrius 5.11.x (cap raised to 4 MB; see companion proposal in
  `cyrius/docs/development/proposals/2026-05-10-raise-compile-source-cap.md`).

- **Per-module test split** (Streamable HTTP / WS): the
  `tests/bote.tcyr` catch-all was hitting the 2 MB cap once the
  cyrius / libro / majra refresh landed. Streamable + WebSocket
  test sections extracted into:
  - `tests/bote_streamable.tcyr` — 25 assertions (EventIdGenerator,
    StreamEvent wire format, ResumptionBuffer eviction, events_after
    edge cases, StreamableConfig retry_ms, sandhi path-strip sanity).
  - `tests/bote_ws.tcyr` — 10 assertions (WsConfig path / addr /
    port / allowed_origins / require_protocol / dispatcher wire-up,
    `_bote_ws_handler` fp addressable).
  `tests/bote.tcyr` keeps 363 assertions; full test count across
  all 10 `tests/*.tcyr` files is **653**. Pattern mirrors phylax
  1.1.1's per-module test split.

### Fixed

- **`scripts/bench-log.sh` ported from `cargo bench` to `cyrius bench`.**
  The script was a Rust-era stale and had never run successfully
  against the Cyrius port. Output captured from `cyrius bench
  tests/bote.bcyr` (14 criterion benches: dispatch_initialize /
  dispatch_tools_list / dispatch_tools_call / jsonx_get_str_flat
  / jsonx_get_raw_nested / codec_parse_request /
  codec_serialize_response / codec_process_message /
  validate_compiled_simple / validate_compiled_nested /
  schema_compile_simple / schema_compile_nested /
  auth_bearer_check_unset / auth_bearer_check_set).

### Performance

Benchmark snapshot at v2.7.2 (full block in
`benches/history.log`). Per-iteration averages:

| Bench | avg | iters |
|-------|-----|-------|
| dispatch_initialize       | 2 µs  | 10 000  |
| dispatch_tools_list       | 3 µs  | 10 000  |
| dispatch_tools_call       | 6 µs  | 10 000  |
| jsonx_get_str_flat        | 1 µs  | 100 000 |
| jsonx_get_raw_nested      | 1 µs  | 100 000 |
| codec_parse_request       | 2 µs  | 10 000  |
| codec_serialize_response  | 1 µs  | 10 000  |
| codec_process_message     | 8 µs  | 10 000  |
| validate_compiled_simple  | 1 µs  | 10 000  |
| validate_compiled_nested  | 3 µs  | 10 000  |
| schema_compile_simple     | 4 µs  | 10 000  |
| schema_compile_nested     | 7 µs  | 10 000  |
| auth_bearer_check_unset   | 1 µs  | 100 000 |
| auth_bearer_check_set     | 2 µs  | 100 000 |

No regressions vs. 2.7.1 (within measurement noise — all benches
were already at the sub-10-µs floor).

### Breaking

- **`bote streamable [port]`** and **`bote ws [port]`** CLI modes
  removed from the default `bote` binary. Callers must invoke
  `bote-streamable [port]` / `bote-ws [port]` instead. The
  invocation matrix:

  | Transport      | 2.7.1                    | 2.7.2                          |
  |----------------|--------------------------|--------------------------------|
  | stdio (default)| `bote`                   | `bote` (unchanged)             |
  | HTTP           | `bote http [port]`       | `bote http [port]` (unchanged) |
  | Unix socket    | `bote unix <path>`       | `bote unix <path>` (unchanged) |
  | TS bridge      | `bote bridge [port]`     | `bote bridge [port]` (unchanged) |
  | Streamable     | `bote streamable [port]` | `bote-streamable [port]`       |
  | WebSocket      | `bote ws [port]`         | `bote-ws [port]`               |

  This is purely a CLI surface change — the underlying transport
  modules (`src/transport_streamable.cyr` / `src/transport_ws.cyr`)
  and their public API are unchanged. Consolidates back into a
  single binary on cyrius 5.11.x migration.

## [2.7.1] — 2026-05-10 — HostRegistry hot-reload + CONTRIBUTING.md Cyrius-era cleanup

Second 2.7.x patch. Lands the **HostRegistry hot-reload** feature
the 2.6.x deferred slate promised, plus a long-overdue
`CONTRIBUTING.md` rewrite that drops the Rust-era references the
Cyrius port left behind.

No MCP wire-format change, no handler-ABI change. New API surface
on `HostRegistry`; existing in-memory wire-up unchanged.

### Added

- **HostRegistry JSON config + hot-reload** (`src/host.cyr`).
  Five new public functions:
  - `host_entry_from_json(obj_cstr)` — parse a single host-entry
    object. Required fields: `name`, `url`. Optional:
    `capabilities` (array of cstrs). Returns 0 on missing
    required fields or malformed `capabilities`.
  - `host_registry_load_json(json_cstr)` — parse a JSON array
    of entry objects into a fresh registry. Empty array is legal
    (returns empty registry, not an error).
  - `host_registry_load_from_file(path)` — read `path` into a
    64 KB buffer (`HOST_CONFIG_BUF_MAX`) and parse. Files larger
    than the buffer truncate at read time and fail-closed at
    parse.
  - `host_registry_reload(r, path)` — read fresh config and swap
    the entries map in-place. Existing pointers stay valid; the
    old map is left to the bump allocator. **Fail-safe**:
    returns `-1` and leaves the registry unchanged on
    open/read/parse failure, so a malformed live edit cannot
    drop the running configuration.
  - `host_registry_clear(r)` — empty the registry in-place;
    used by `reload`, also exposed for explicit caller use.
  Plus one internal helper, `_host_parse_str_array`, that walks
  a JSON array of strings into a `vec<cstr>` (used by the
  `capabilities` branch of `host_entry_from_json`).

### Config format

```json
[
  {"name":"upstream-1","url":"https://api.example.com"},
  {"name":"upstream-2","url":"https://other.example.com",
   "capabilities":["fetch","tools_call"]}
]
```

`headers` parsing is deferred — header values may carry secrets
and need a redaction / audit story before they can come through
file config. The in-memory `host_entry_with_headers` setter is
still available for callers that need headers in code.

### Changed

- **`CONTRIBUTING.md` rewritten for the Cyrius era.** The
  pre-port doc referenced `make check` / `cargo fmt` /
  `cargo-deny` / `src/lib.rs` / `Cargo.toml` / MSRV 1.89.
  Replaced with the actual cyrius commands the codebase uses
  today: `cyrius deps` / `cyrius build` / `cyrius test` /
  `cyrius distlib` / `CYRIUS_STATS=1` / `CYRIUS_DCE=1`. Adds
  a Common Commands table, an Adding a New Module section
  that matches the 2.6.3 dist-bundle contract, and a Testing
  section that documents the 8-file split + the parser quirk
  workaround (stage 2-arg inner calls into a var when
  `assert(streq(call(...), "lit") == 1, "msg")` trips
  `expected ')', got string` on cyrius 5.10.x).
- **`tests/bote_host.tcyr`** now includes `lib/io.cyr` and
  `src/jsonx.cyr`. The hot-reload parser piggybacks on jsonx
  for whitespace / string / struct skipping (same surface bote
  uses everywhere else for JSON-ish handling); the file
  roundtrip tests need `file_write_all` from `lib/io.cyr`.

### Verified (cyrius 5.10.x, local)

- **653 unit assertions across 8 test files** — **+46 vs 2.7.0**:
  - `bote_host.tcyr` **113** (+46) from the new hot-reload
    coverage: 4 `host_entry_from_json` shape paths,
    `_host_parse_str_array` empty / two-element / malformed
    cases, `host_registry_load_json` happy / empty-array /
    null / non-array / malformed-entry / unterminated paths,
    `host_registry_load_from_file` roundtrip via `/tmp`,
    `host_registry_reload` swap + fail-safe + missing-file +
    null-arg paths, `host_registry_clear`.
  - Per-file: `bote.tcyr` 398 / `bote_auth.tcyr` 38 /
    `bote_content.tcyr` 24 / `bote_host.tcyr` **113** /
    `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 /
    `bote_sandbox.tcyr` 13 / `bote_libro_tools.tcyr` 22.
- Default binary capacity: fn_table **89%** (3658/4096) —
  +6 vs 2.7.0 from the 5 new public functions + helper. Well
  under the 95% CI gate from 2.6.4.
- `bote_host.tcyr` compile unit absorbs `lib/io.cyr` +
  `src/jsonx.cyr` without strain — under the capacity gate
  comfortably.
- `dist/bote.cyr` regenerated; header reflects 2.7.1.

### Forward roadmap

Remaining 2.7.x candidates after this patch ship:

| Item | Effort |
|---|---|
| OAuth 2.1 AS flow | High — explicitly out of scope, kept on the list as a marker |

The 2.7.x feature slate from the deferred 2.6.x carry-forward
list is otherwise empty. 2.7.x continues opportunistically; the
next 2.7.x patch lands when new in-tree needs emerge. The next
*planned* arc opens at 2.8.x — likely the long-deferred
threaded streaming dispatch, gated on cyrius
`lib/thread.cyr` MPSC + `lib/async.cyr` cancellation firming up.

## [2.7.0] — 2026-05-10 — Annotations through `wrap_tool_result` + bench coverage + `[Unreleased]` flow

Opens the 2.7.x feature line after the 2.6.x modernization arc
closed at 2.6.4. Three Low-effort items from the deferred 2.6.x
candidates list bundled into one carry-forward release; no MCP
wire-format change, no handler-ABI change. Closes the last loose
ends from the 1.9.6 (annotations) and bench-coverage eras.

### Added

- **Annotations propagation through `wrap_tool_result`**
  (`src/bridge.cyr`). A handler returning a single MCP content
  block (object with a `type` field, possibly carrying
  `annotations` from `content_with_annotations`) is now lifted
  into a 1-element `content` array verbatim, preserving the
  annotations. Previously the block got JSON-escaped into a
  synthesised `text` payload — annotations survived as
  characters in a string but became semantically useless to MCP
  clients. Three input shapes are now distinguished:
  1. Object with a `content` array → passthrough (unchanged).
  2. Object with a `type` field → lift into envelope (**new**).
  3. Anything else → wrap as synthesised text block (unchanged).
  Four new assertions in `tests/bote.tcyr` cover the lift path
  for text + image blocks, with and without annotations, plus
  the non-block-object fallthrough.
- **`schema_compile_simple` + `schema_compile_nested`
  benchmarks** in `tests/bote.bcyr`. The existing validation
  benches measure the compiled artifact's runtime cost; these
  new ones measure pure compilation cost. Local numbers (cyrius
  5.10.x, x86_64): `schema_compile_simple` 4µs avg,
  `schema_compile_nested` 7µs avg.
- **`auth_bearer_check_unset` + `auth_bearer_check_set`
  benchmarks** in `tests/bote.bcyr`. Quantifies the no-overhead
  claim from 1.9.0 — the bearer-middleware fast path
  (validator fp == 0) is expected to be essentially free.
  Confirmed: `auth_bearer_check_unset` runs at **1µs avg**
  versus `auth_bearer_check_set` at 2µs (the full path: parse
  Authorization header, call allowlist validator, accept/reject).
- **`## [Unreleased]` section pattern** at the top of
  `CHANGELOG.md`. From 2.7.0 onward, in-flight entries
  accumulate under `[Unreleased]` and the header is renamed
  at release time. Conventions block documents the flow.
- **Bench compile-unit additions**: `tests/bote.bcyr` now
  includes `lib/tagged.cyr`, `lib/net.cyr`, `lib/tls.cyr`,
  `lib/sandhi.cyr`, and `src/auth.cyr` — required by the new
  `auth_bearer_check_*` benches. Compile unit at fn_table
  **85%** (3478/4096), identifier buffer **85%**
  (111806/131072) — under the 95% CI gate from 2.6.4.

### Verified (cyrius 5.10.x, local)

- **607 unit assertions across 8 test files** — **+4 vs 2.6.4**
  from the four new `wrap_tool_result` lift-path assertions.
  Per-file: `bote.tcyr` **398** (+4) / `bote_auth.tcyr` 38 /
  `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_jwt.tcyr`
  28 / `bote_pkce.tcyr` 17 / `bote_sandbox.tcyr` 13 /
  `bote_libro_tools.tcyr` 22.
- **14 criterion benchmarks** — was 10; +4 from
  `schema_compile_{simple,nested}` and
  `auth_bearer_check_{unset,set}`.
- Default binary: fn_table **89%** (3652/4096), identifier
  buffer **88%** (116378/131072) — unchanged vs 2.6.4 (the
  bridge.cyr branch addition adds <5 fns and doesn't move
  the meter at this resolution). Well under the capacity gate.
- Bench compile unit: fn_table **85%** (3478/4096), identifier
  buffer **85%** (111806/131072) — comfortable headroom for
  the four new bench callbacks and their auth + sandhi
  includes.
- `dist/bote.cyr` regenerated; header reflects 2.7.0.

### Forward roadmap

The 2.7.x feature candidates remaining (from the deferred
2.6.x slate):
- **HostRegistry hot-reload from config file** (Medium) —
  useful for deployments that rotate allowed upstreams without
  a restart.
- **CONTRIBUTING.md Cyrius-era cleanup** — the current doc
  still references Rust-era commands (`make check`,
  `cargo-deny`, `src/lib.rs`). Stale since the Cyrius port.
- **OAuth 2.1 authorization-code flow** (High) — out of scope
  for MCP core; bote is the resource server. Explicitly
  deferred — consumers compose bote with their own AS layer.

## [2.6.4] — 2026-05-10 — Capacity gate (closes 2.6.x modernization arc)

Last patch in the 2.6.x modernization arc. The 2.6.0 toolchain
bump pushed the default-binary compile-time fn_table to 89%
(3652/4096) and the identifier buffer to 88% (116370/131072) —
both above cyrius's own 85% in-compiler warning threshold but
neither blocking. 2.6.4 lands a CI gate so the situation can't
quietly drift past the cap during 2.7.x feature work without
forcing the split / cap-raise conversation.

No source-side architectural change; the split / feature-gate
options are documented as response paths if the gate fires.
The 2.7.x feature backlog (deferred from the original 2.6.x
slate by 2.6.0) is now unblocked.

### Added

- **CI capacity gate** (`.github/workflows/ci.yml`). The
  `cyrius build` step now runs under `CYRIUS_STATS=1`, which
  emits a machine-parseable `cyrius stats:` block at the
  tail of stdout:
  ```
  cyrius stats:
    fn_table:    3652 / 4096
    identifiers: 116370 / 131072
    var_table:   1890 / 8192
    fixup_table: 12331 / 262144
    string_data: 30735 / 2097152
    code_size:   1220568 / 1048576
  ```
  A follow-up step parses `fn_table` and `identifiers`,
  computes utilisation, and fails the build if either crosses
  **95%**. Threshold rationale: cyrius itself warns at 85%
  (advisory only); 95% leaves ~5% headroom (~205 fn slots,
  ~7350 ident bytes) for a mid-PR feature add before the
  gate trips. Current util is 89% / 88% — well under the
  gate.
- **Three documented response paths** for when the gate
  ever fires (inline in `.github/workflows/ci.yml` and
  echoed in `docs/development/roadmap.md`):
  1. **Land the cyrius cap raise upstream.** Preferred —
     the cap has moved before (fn_table 2048→4096 in
     cyrius 4.7.1, identifier buffer raised to 131072 in
     4.6.2). File against `cyrius-feedback.md`.
  2. **Split a transport into an opt-in compile unit.**
     Keep stdio + http in the default binary; ws /
     streamable / bridge / unix pulled in via a separate
     `include "src/transport_ws.cyr"` at the consumer's
     `main.cyr`. Mirrors the `libro_tools.cyr` opt-in
     pattern from 2.6.2.
  3. **Feature-gate the unused config-setter surface**
     behind `#ifdef BOTE_FULL_CONFIG`. The lean profile
     would shed ~30 setters that no in-tree consumer calls
     (`bridge_config_with_*`, `streamable_config_with_*`,
     `ws_config_with_*` etc.).

### Verified (cyrius 5.10.x, local)

- **Capacity gate simulation** — local run with the same
  parser CI uses reports `fn_table: 3652 / 4096 (89%)`,
  `identifiers: 116370 / 131072 (88%)`. `GATE: pass`.
- **`CYRIUS_DCE=1` impact measured**: zero. DCE only
  affects emitted code bytes; the fn_table / identifier
  counters are compile-time resource consumption, eaten
  by every `fn` declaration regardless of whether it gets
  emitted. (Captured in the gate's inline rationale so the
  next person doesn't re-test this.)
- **603 unit assertions across 8 test files** — unchanged.
- Build: `src/main.cyr → build/bote` at fn_table 89%
  (3652/4096), identifier buffer 88% (116370/131072) —
  no change vs 2.6.3.
- `dist/bote.cyr` regenerated; header reflects 2.6.4.

### Modernization arc — done

The 2.6.x arc opened at 2.6.0 with the cyrius 5.10.34 /
libro 2.6.2 / majra 2.4.3 floor bump and closes at 2.6.4
with the capacity gate. Five patches, each a contained
bite, no behaviour drift:

| Patch | Bite |
|---|---|
| 2.6.0 | cyrius / libro / majra floor bump + cyrius.cyml + dist-bundle deps + CI installer modernization + sandhi compat shim |
| 2.6.1 | Retire `_sandhi_compat.cyr` — 108 call sites flipped to `sandhi_server_*` |
| 2.6.2 | Port `libro_tools.cyr` to libro 2.6.x API; re-enable `bote_libro_tools.tcyr`; back to 603-assertion baseline |
| 2.6.3 | Ship `dist/bote.cyr` consumer bundle via `cyrius distlib`; CI freshness gate; release asset |
| 2.6.4 | CI capacity gate (95% fn_table / identifier threshold); modernization arc closed |

2.7.x picks up the deferred feature backlog — see
`docs/development/roadmap.md`.

## [2.6.3] — 2026-05-10 — Ship `dist/bote.cyr` for downstream consumers

Fourth patch in the 2.6.x modernization arc. Brings bote onto
the same distribution contract as libro and majra: a
single-file `dist/bote.cyr` bundle that downstream MCP
consumers (phylax, t-ron, sutra, jalwa, rasa, mneme) can pull
via `[deps.bote] modules = ["dist/bote.cyr"]` in their own
`cyrius.cyml`. No source edits to existing consumers; new
consumers get a clean one-file entry point.

No MCP wire-format change, no handler-ABI change, no source
behaviour change.

### Added

- **`dist/bote.cyr` (4615 lines, committed)** — single-file
  distribution bundle generated by `cyrius distlib` from
  `cyrius.cyml [lib] modules`. Self-contained at the
  source level (no `include` directives in-bundle); stdlib +
  libro + majra are supplied by the consumer's own
  `[deps] stdlib` / `[deps.libro]` / `[deps.majra]` set,
  the same way bote pulls libro 2.6.2 and majra 2.4.3.
  `libro_tools.cyr` is **not** in the bundle — it stays
  opt-in (see 2.6.2 notes). Header carries
  `# Version: 2.6.3` for runtime introspection by consumers
  that want to detect the bundle version.
- **CI dist-freshness gate** (`.github/workflows/ci.yml`).
  Every CI run regenerates `dist/bote.cyr` and asserts a
  byte-clean diff against the committed file. Catches the
  failure mode where a `src/*.cyr` change lands without a
  follow-up `cyrius distlib` — downstream `cyrius deps` at
  our tag would otherwise pull stale source. Mirrors the
  libro / majra freshness gate.
- **Release workflow regenerates + ships `dist/bote.cyr`**
  (`.github/workflows/release.yml`). The bundle joins the
  release asset set as `bote-<ver>.cyr` alongside the
  source tarball, the x86_64 binary, `cyrius.lock`, and
  `SHA256SUMS`. A tagged release without the bundle would
  break every consumer's `cyrius deps` at the tag — the
  release gate fails fast on a stale or missing bundle.

### Consumer wire-up (canonical form)

```toml
# In a consumer's cyrius.cyml:
[deps.bote]
git = "https://github.com/MacCracken/bote"
tag = "2.6.3"
modules = ["dist/bote.cyr"]
```

`cyrius deps` clones bote at the tag and copies
`dist/bote.cyr` into the consumer's `lib/bote.cyr`. The
consumer then `include "lib/bote.cyr"` from their own
`src/main.cyr`, alongside their stdlib + libro + majra dep
includes. The bundle's `registry_new` / `dispatcher_new` /
`transport_*_run` / `auth_*` / `content_*` / `host_*` /
`audit_*` / `events_*` surfaces are all available.

### Verified (cyrius 5.10.x, local)

- **`cyrius distlib`** emits `dist/bote.cyr` deterministically;
  bundle header captures version from `cyrius.cyml`.
- **Consumer-perspective compile** — synthesised a
  consumer-style entry that includes `dist/bote.cyr` plus
  the stdlib + libro + majra dep set; compiles cleanly,
  binary runs and exits 0.
- **603 unit assertions across 8 test files** — unchanged
  from 2.6.2; the distlib step is a pure post-processing
  step that doesn't touch the build path.
- Build: `src/main.cyr → build/bote` still at fn_table 89%
  (3652/4096); no change in default-binary footprint.

### Forward roadmap

- 2.6.4: capacity / split prep — last patch in the
  modernization arc. Decide between splitting the streamable
  / WS transports into an opt-in compile unit, feature-gating
  the unused-config setters, or holding for the next cyrius
  cap raise.

## [2.6.2] — 2026-05-10 — Port libro_tools.cyr to libro 2.6.x

Third patch in the 2.6.x modernization arc. The 2.6.0 release
deferred `src/libro_tools.cyr` (the five built-in MCP tools that
expose the libro audit chain) because libro 2.x retired the
public `entry_*` / `error_*` / `merkle_*` accessors that the
2.5.1 source assumed. 2.6.2 lands the port and re-enables
`tests/bote_libro_tools.tcyr` — bote is now back to **603
passing assertions** across 8 active test files, matching the
2.5.1 baseline.

Still no MCP wire-format change, no handler-ABI change; the
five tool handlers emit byte-identical JSON for byte-identical
input. The port is internal-only.

### Changed

- **`src/libro_tools.cyr` ported to the libro 2.6.x API**:
  - Replaced six retired entry accessors (`entry_timestamp` /
    `entry_severity` / `entry_source` / `entry_action` /
    `entry_agent_id` / `entry_hash`) with raw struct-offset
    reads via local helpers `_lt_entry_timestamp` etc. The
    libro 2.x entry struct layout is documented inline at
    `src/libro_tools.cyr:46-58` and tracks
    `lib/libro.cyr:200-205`.
  - Replaced the retired `chain_entries(c)` getter with a
    one-line `_lt_chain_entries(c) { return load64(c); }` —
    the libro chain struct keeps the entries vec at +0 and
    is unlikely to drift.
  - Replaced the retired `error_code` / `error_index` /
    `error_msg` getters with raw-offset helpers (`_lt_err_code`
    / `_lt_err_index` / `_lt_err_msg`). The libro 2.x error
    struct is 6 fields wide (`code`, `msg`, `field_name`,
    `index`, `expected`, `actual`) and the message field is
    now a cstr rather than a Str — the wrapper drops the
    `str_data` unwrap.
  - Renamed `merkle_proof(tree, idx)` →
    `merkle_inclusion_proof(tree, idx)`.
  - Replaced the retired `merkle_tree_leaf_count(tree)` with
    `_lt_merkle_leaf_count(t) { return load64(t + 8); }` —
    the libro 2.x merkle_tree struct is `{nodes, leaf_count}`
    so leaf_count sits at +8.
- **`tests/bote_libro_tools.tcyr` re-enabled and updated for
  the 2.6.0 dist-bundle layout**:
  - Nine individual `lib/libro_*.cyr` includes collapsed to
    a single `include "lib/libro.cyr"` (the dist bundle).
  - Added `lib/fs.cyr` / `lib/ct.cyr` / `lib/keccak.cyr` /
    `lib/process.cyr` / `lib/random.cyr` to the include set
    — libro 2.6.x consumes these transitively at parse time
    (`file_open`, `ct_eq`, `random_bytes` etc.).
  - Updated all 9 handler invocations to pass the
    2-arg `(args, claims)` ABI (was 1-arg in the 2.5.1
    source; the second slot landed in 2.0.0 but the test
    file never caught up).
- **`.github/workflows/ci.yml`**: `bote_libro_tools.tcyr`
  added to the test matrix. 8th test file is now mandatory.
- **`src/main.cyr` header comment**: the `DEFERRED:` marker
  on the `include "src/libro_tools.cyr"` line is now an
  `OPT-IN:` marker — the module compiles cleanly, and
  consumers that want libro audit-tool dispatch can include
  + wire it locally without touching the bote default binary.
  Kept out of the default build because the main binary is
  at 89% fn_table utilisation; the libro_tools handlers
  would tip the build over.

### Verified (cyrius 5.10.x, local)

- **603 unit assertions across 8 test files** — back to the
  2.5.1 baseline:
  - `bote.tcyr` **394** / `bote_auth.tcyr` 38 /
    `bote_content.tcyr` 24 / `bote_host.tcyr` 67 /
    `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 /
    `bote_sandbox.tcyr` 13 / `bote_libro_tools.tcyr` **22**.
- Default binary build: `src/main.cyr → build/bote` still at
  fn_table 89% (3652/4096) — no change vs 2.6.1 (libro_tools
  stays out of main).
- `bote_libro_tools.tcyr` compile unit: fn_table 85%
  (3485/4096), identifier buffer 85% (111665/131072) —
  comfortable headroom.

### Forward roadmap

- 2.6.3: emit `dist/bote.cyr` via `cyrius distlib`.
- 2.6.4: capacity / split prep.

## [2.6.1] — 2026-05-10 — Retire the sandhi compat shim

Second patch in the 2.6.x modernization arc. The 2.6.0 release
bridged `lib/http_server.cyr` (retired in cyrius 5.10.x) to the
new `lib/sandhi.cyr` HTTP-server surface via a thin compat shim
(`src/_sandhi_compat.cyr`) so the version bump didn't churn
every transport file. 2.6.1 retires the shim with a mechanical
rename pass — bote now calls into sandhi directly.

No wire-format change, no handler-ABI change, no behaviour
drift; this is a name-only refactor.

### Changed

- **108 HTTP-server call sites renamed.** Across
  `src/auth.cyr`, `src/bridge.cyr`, `src/transport_http.cyr`,
  `src/transport_streamable.cyr`, `src/transport_ws.cyr`, plus
  `tests/bote.tcyr` + `tests/bote_auth.tcyr`. Twelve symbols
  flipped from the pre-5.10 `http_*` family to the
  `sandhi_server_*` names that landed in cyrius 5.10's
  `lib/sandhi.cyr`:
  - `http_send_status`         → `sandhi_server_send_status`
  - `http_send_response`       → `sandhi_server_send_response`
  - `http_send_chunked_start`  → `sandhi_server_send_chunked_start`
  - `http_send_chunk`          → `sandhi_server_send_chunk`
  - `http_send_chunked_end`    → `sandhi_server_send_chunked_end`
  - `http_get_method`          → `sandhi_server_get_method`
  - `http_get_path`            → `sandhi_server_get_path`
  - `http_find_header`         → `sandhi_server_find_header`
  - `http_content_length`      → `sandhi_server_content_length`
  - `http_body_offset`         → `sandhi_server_body_offset`
  - `http_path_only`           → `sandhi_server_path_only`
  - `http_server_run`          → `sandhi_server_run`
  `HTTP_*` status-code constants (`HTTP_OK`, `HTTP_BAD_REQUEST`,
  `HTTP_UNAUTHORIZED`, etc.) are unchanged — sandhi exports them
  under the same names.

### Removed

- **`src/_sandhi_compat.cyr` deleted.** The 2.6.0 shim
  (~50 LoC, 12 tail-call wrappers) was a transition aid only;
  with all call sites renamed the file has no callers left.
- **The CI manifest-completeness gate's `EXCLUDES` list** (.github/workflows/ci.yml)
  is gone — `[lib]` modules in `cyrius.cyml` now exactly equal
  the set of `src/<file>.cyr` includes in `main.cyr`. No
  exceptions to track.

### Verified (cyrius 5.10.x, local)

- **581 unit assertions** across 7 active test files — same
  counts as 2.6.0; no regression.
  - `bote.tcyr` 394 / `bote_auth.tcyr` 38 /
    `bote_content.tcyr` 24 / `bote_host.tcyr` 67 /
    `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 /
    `bote_sandbox.tcyr` 13.
- Production build: `src/main.cyr → build/bote` succeeds.
  Function-table util 89% (3652/4096, -11 vs 2.6.0 from
  removing the 12 shim wrappers); identifier buffer 88%
  (116370/131072, -196 bytes).
- Smoke test: `initialize` round-trips clean,
  `serverInfo.version == "2.6.1"`.

### Forward roadmap

- 2.6.2: port `src/libro_tools.cyr` to the libro 2.6.x API
  and re-enable `tests/bote_libro_tools.tcyr` (22 assertions).
- 2.6.3: emit `dist/bote.cyr` via `cyrius distlib`.
- 2.6.4: capacity / split prep.

## [2.6.0] — 2026-05-10 — Modernization platform: cyrius 5.10.34, libro 2.6.2, majra 2.4.3

2.6.0 catches bote up to the first-party Cyrius floor and opens
the **2.6.x modernization arc** (see `docs/development/roadmap.md`).
No MCP wire-format change, no handler-ABI change; the live
audit + events integration is byte-identical to 2.5.1 at the
JSON-RPC boundary. Forward feature work that was slotted for
2.6.x slides one minor to 2.7.x; 2.6.x is reserved for the
modernization sequence.

### Changed

- **Cyrius toolchain pin: 4.8.4 → 5.10.34.** Matches the
  current first-party floor (agnosys 1.2.4, agnostik 1.2.1,
  libro 2.6.2, majra 2.4.3). Notable upstream changes spanning
  this range: arch-peer include resolution now expects
  `~/.cyrius/versions/<V>/lib` (5.10.9+) — CI installer
  updated accordingly; richer fmt/lint/vet/capacity surfaces;
  `CYRIUS_DCE=1` available for release binaries; raised fixup
  cap; stdlib `ct_eq_bytes` family; `secret` promoted to a
  storage-class keyword (forced a rename in `src/jwt.cyr`);
  `lib/http_server.cyr` retired from stdlib and folded into
  the new `lib/sandhi.cyr` HTTP-server surface.
- **libro 1.0.3 → 2.6.2 via single-file dist bundle.** The
  consumer contract switched in libro 2.x from `[deps.libro]
  modules = ["src/error.cyr", "src/hasher.cyr", …]` to
  `modules = ["dist/libro.cyr"]`. `cyrius deps` copies the
  upstream `dist/libro.cyr` into `lib/libro.cyr`. All
  symbol-level call sites bote uses (`chain_new`,
  `chain_append`, `chain_append_with_agent`, `chain_verify`,
  `pubsub_*`) are unchanged in the new dist surface — the
  live audit_libro / events_majra adapters compile clean
  without source edits.
- **majra 2.2.0 → 2.4.3 via single-file dist bundle.** Same
  pattern — `[deps.majra] modules = ["dist/majra.cyr"]`,
  resolves to `lib/majra.cyr`. The default profile (core
  pubsub engine without backends) is the bote-appropriate
  bundle; future transport work can upgrade to
  `dist/majra-signed.cyr` or `-backends.cyr` if signed
  envelopes or network backends become bote-side concerns.
- **`cyrius.toml` → `cyrius.cyml`.** Adopts the first-party
  manifest layout: `version = "${file:VERSION}"` placeholder
  (the version is owned by `VERSION`, the manifest pulls it),
  `[lib]` section enumerating bote's own modules for future
  `cyrius distlib dist/bote.cyr` (2.6.3), `cyrius = "5.10.34"`
  toolchain pin in the manifest (no separate
  `.cyrius-toolchain` file). The release workflow enforces
  the `${file:VERSION}` placeholder so the version can't drift
  out of sync at tag time.
- **`/lib/` is no longer committed.** `.gitignore` covers
  `/lib/` — `cyrius deps` repopulates it from the
  version-pinned stdlib snapshot plus the tagged libro / majra
  dist bundles. Matches agnosys / majra / libro / yukti /
  patra convention. Prevents stale stubs from prior cyrius
  versions sitting in tree.
- **HTTP server surface bridged from `http_server` to `sandhi`
  via `src/_sandhi_compat.cyr`.** The cyrius 5.10.x stdlib
  retired `lib/http_server.cyr` and folded its surface into
  `lib/sandhi.cyr` under a `sandhi_server_` prefix. 2.6.0
  introduces a thin compat shim re-exporting the ~12 symbols
  bote uses (`http_send_status`, `http_send_response`,
  `http_send_chunked_*`, `http_get_method`, `http_get_path`,
  `http_find_header`, `http_content_length`, `http_body_offset`,
  `http_path_only`, `http_server_run`) as tail-calls into
  the sandhi names. Zero call-site churn in 2.6.0; the rename
  pass and shim retirement land in 2.6.1.
- **`tls` added to `[deps] stdlib`.** sandhi references
  `TLS_EARLY_DATA_ACCEPTED` at parse time — without `tls.cyr`
  in the dep set, cyrius's deps-aware build can't validate
  the dep graph. Mirrors majra 2.4.2's same addition.
- **`src/jwt.cyr` — `secret` parameter renamed to `key`.**
  `secret` is a storage-class keyword in cyrius 5.10
  (sigil's HMAC ipad/opad buffers use it). Renamed inside
  `jwt_verify_hs256` and `jwt_secret_new`; the exported
  function names (`jwt_secret_new`, `jwt_secret_data`,
  `jwt_secret_len`, `auth_validator_jwt_hs256`) are unchanged
  — `secret` is reserved only as a bare identifier, not
  inside compound names. `tests/bote_jwt.tcyr` renamed its
  local `var secret = "..."` to `var jwt_key = "..."` for the
  same reason.

### CI / release modernization (matches majra / agnosys)

- **Versioned toolchain installer.** The installer now lays
  out `~/.cyrius/versions/<V>/{bin,lib}` and symlinks
  `~/.cyrius/{bin,lib}` to the version-pinned snapshot.
  Required by cc5 5.10.9+: arch-peer includes
  (`syscalls_x86_64_linux.cyr` etc.) resolve against the
  version-pinned `lib/`, so the flat `~/.cyrius/lib` layout
  the 4.8.x installer used no longer works.
- **Source-archive fetch for `lib/`.** 5.10.x release
  tarballs ship `bin/` + `deps/` only — the `lib/` stdlib
  snapshot is NOT in the tarball. CI now pulls the GitHub
  source archive at the version tag and copies `lib/` from
  there. Sanity-checked against `syscalls_x86_64_linux.cyr`
  + `bin/cc5` to fail fast if either extraction is broken.
- **`cyrius deps --verify` lockfile gate.** `cyrius.lock`
  (committed) records SHA-256 of every resolved dep. CI
  enforces hash match; the gate self-skips on the first push
  that introduces a new dep before the lockfile lands.
- **Manifest-completeness gate.** Every
  `include "src/<file>.cyr"` in `src/main.cyr` must be listed
  under `[lib]` modules in `cyrius.cyml` (or be in the known
  excludes — currently `src/_sandhi_compat.cyr`). Prevents
  `cyrius distlib` from silently shipping a bundle missing a
  module once 2.6.3 lands.
- **`CYRIUS_NO_WARN_SHADOW_LIB=1` + `CYRIUS_DCE=1`.** Standard
  env across CI / release steps — silences the
  cwd-shadows-version-snapshot informational note and turns
  on whole-program dead-code elimination. Matches majra /
  libro / agnosys / agnostik.
- **Release workflow accepts both `v1.2.3` and `1.2.3` tag
  styles.** The version-verify step enforces semver shape +
  exact match against the `VERSION` file + the
  `${file:VERSION}` placeholder in `cyrius.cyml`. Release
  notes auto-extracted from the matching `## [VERSION]`
  section of `CHANGELOG.md`.
- **Release artifacts.** Source tarball
  (`bote-<ver>-src.tar.gz`), x86_64-linux binary
  (`bote-<ver>-x86_64-linux`), `cyrius.lock`, and
  `SHA256SUMS` over all artifacts. Matches majra's release
  asset set.

### Deferred

- **`tests/bote_libro_tools.tcyr`** (22 assertions) is parked
  for 2.6.2 alongside the `src/libro_tools.cyr` port. The
  libro 2.x dist bundle dropped six entry accessors
  (`entry_action` / `entry_severity` / `entry_hash` /
  `entry_source` / `entry_agent_id` / `entry_timestamp`),
  replaced `merkle_proof` with `merkle_inclusion_proof`,
  retired `merkle_tree_leaf_count` (now `merkle_canonical_root(tree, size)`
  with explicit size), and folded
  `libro_export` into `export_jsonl` over a fd. The
  libro-tool dispatcher rewrites cleanly against the new
  surface but is its own focused bite; targeting 2.6.2.
- **The `_sandhi_compat.cyr` shim retires in 2.6.1.** ~56
  call sites across `transport_http.cyr` /
  `transport_streamable.cyr` / `bridge.cyr` /
  `transport_ws.cyr` / `auth.cyr` flip from `http_*` to
  `sandhi_server_*`; mechanical pass.

### Verified (cyrius 5.10.x, local)

- **581 unit assertions across 7 test files** —
  `bote.tcyr` **394** / `bote_auth.tcyr` 38 /
  `bote_content.tcyr` 24 / `bote_host.tcyr` 67 /
  `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 /
  `bote_sandbox.tcyr` 13. (Original 2.5.1 = 603 over 8
  files; the 22-assertion gap is `bote_libro_tools.tcyr`
  parked for 2.6.2 — no regression on the live integration.)
- Production build: `src/main.cyr → build/bote` succeeds
  cleanly. Function-table utilisation 89% (3663/4096) and
  identifier buffer 88% (116566/131072) — tracked for 2.6.4
  if the libro_tools restore pushes us over.
- Benchmarks: `tests/bote.bcyr → build/bote_bench` builds
  clean.
- `cyrius deps` resolves 6 deps and writes `cyrius.lock`;
  `cyrius deps --verify` round-trips clean.

### Forward roadmap

`docs/development/roadmap.md` rewritten: 2.6.x is now the
modernization arc (2.6.0–2.6.4 bounded above); the
previously-2.6.x feature backlog moved to **2.7.x candidates**.

## [2.5.1] — 2026-04-14 — Restore audit_libro + events_majra tests (cyrius 4.8.4 retag)

The cyrius lang-agent retagged 4.8.4 with the alpha2-that-actually-works
binary (fix for the 2.5.0-era local-vs-CI binary skew reported in
`docs/bugs/cyrius-4.8.4-ci-binary-skew.md`). Clean-installing the
updated toolchain lets bote carry the full libro + majra + sigil
dependency graph in `tests/bote.tcyr` again — no more workaround.

### Restored
- **`tests/bote.tcyr`** — re-added the full dep-graph includes dropped during the 2.5.0 CI workaround:
  - `lib/sakshi.cyr`, `lib/bigint.cyr`, `lib/sigil.cyr`
  - `lib/libro_*.cyr` (error / hasher / entry / verify / query / retention / chain)
  - `lib/majra_*.cyr` (error / counter / envelope / namespace / queue / pubsub)
  - `src/audit_libro.cyr`, `src/events_majra.cyr` — now usable again because `SEV_INFO` + friends resolve
- **8 shape-only test assertions recovered**:
  - `audit_libro` struct + accessors (7 asserts: chain handle, default source, custom source, agent_id)
  - `audit_libro` AuditSink wire-up fp-addressability
  - `events_majra` EventSink wire-up (fp addressable + ctx=0 call)

### Verified (fresh cyrius 4.8.4 install)
- All 8 test files green: `bote.tcyr` **394** (was 386 in 2.5.0) / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_auth.tcyr` 38 / `bote_sandbox.tcyr` 13 / `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 = **603 total** (back to pre-trim count).
- Production build: `src/main.cyr -> bote 370480 bytes 674ms [x86]`.
- Local `cc3` rebuilt from the retagged cyrius repo.

### Resolved
- `docs/bugs/cyrius-4.8.4-ci-binary-skew.md` — closed by the retag. Roadmap "Open bugs" entry flipped to ✅.

### Carried forward
- v1.2.1 libro-growth heisenbug: unchanged.
- Slowloris recv timeout (audit H5): still needs `sock_set_recv_timeout` stdlib helper.
- WS handshake key-length validation (audit M4): upstream stdlib.
- DNS for SSRF hostname classification: still needs `getaddrinfo` stub.
- JWT RS256 / ES256: still waits on sigil RSA / ECDSA primitives.

## [2.5.0] — 2026-04-14 — Claims propagation + cyrius 4.8.4 pin

Lands the claims-propagation refactor that 2.0's handler-ABI break
was designed for. Validators' return values now flow transport →
codec → dispatcher → `fncall2(handler, args, claims)`. Handlers that
want per-tool authorization can inspect `claims` directly (opaque
cstr, JWT payload ptr, whatever the backend validator produced).
Handlers that ignore it see no change — the 2nd arg is just unused.

Pairs with **cyrius 4.8.4**, which closed the three 4.8.3 regressions
that temporarily blocked the refactor (see `docs/bugs/cyrius-4.8.3-
regressions.md` — all three fixes landed in 4.8.4 as advertised).

### Changed
- **cyrius pin**: 4.8.1 → 4.8.4 (needed for the path-traversal fix,
  include-once cap 64→256, and nested-include `PP_IFDEF_PASS` fixpoint).
- **`src/auth.cyr`** — `auth_bearer_check` gains a `claims_out` ptr param;
  writes the validator's return to `*claims_out` on success. Passing 0
  means "don't care" (e.g. streamable GET which doesn't dispatch).
- **`src/dispatch.cyr`** — `dispatcher_dispatch(d, request)` → `(d, request, claims)`; `fncall2(fp, args, 0)` → `fncall2(fp, args, claims)`.
- **`src/codec.cyr`** — `codec_process_message` + `_cdc_process_single` both gain `claims` arg; thread through to `dispatcher_dispatch`.
- **`src/bridge.cyr`** — `bridge_process_message` + `_bridge_process_single` both gain `claims`; handler captures into `claims_slot[8]`, passes through.
- **Per-transport handlers** — `transport_http` / `transport_ws` / `bridge` declare `var claims_slot[8]; store64(&claims_slot, 0);`, pass `&claims_slot` to `auth_bearer_check`, then `load64(&claims_slot)` into the dispatch call. `transport_streamable` does the same for its POST path; its GET path passes 0 (SSE stream, no dispatch). `transport_stdio` and `transport_unix` (local-only, no auth) pass 0 verbatim.
- **`tests/bote.tcyr`** — `lib/http_server.cyr` added explicitly (the dep resolver wasn't pulling it transitively for this compile unit). libro+majra+sigil+sakshi+bigint `include`s tried locally (compiled fine on `cc3 4.8.4-alpha2`) but reverted after the 4.8.4 release binary on CI hit the misleading `lib/assert.cyr:3` parse error at `fn=908/4096` — well under cap but still tripping. Left for a later round once the CI binary skew is resolved.

### Validator contract (handler perspective)

Handlers now look like:

```cyr
fn my_tool(args, claims) {
    # claims is 0 if auth was disabled at the transport config, or the
    # validator's non-zero return otherwise. For the bundled validators:
    #   auth_validator_allow_all      → returns the token cstr
    #   auth_validator_allowlist      → returns the token cstr on match
    #   auth_validator_jwt_hs256      → returns the token cstr on valid JWT
    # A consumer-supplied validator may return a parsed claims struct.
    # Treat as opaque unless you know which validator is configured.
    if (claims == 0) { return _err("auth required"); }
    # ... use `args` + `claims` ...
    return some_result;
}
```

### Verified (cyrius 4.8.4)
- All 8 test files green: `bote.tcyr` 386 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_auth.tcyr` 38 / `bote_sandbox.tcyr` 13 / `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 = **595 total**.
- The audit_libro + events_majra shape-only tests that tried to fit into `bote.tcyr` on 2.4.0-era were dropped here too — they'd need `lib/libro_*` + `lib/majra_*` in `bote.tcyr`'s compile unit, which locally passes cyrius 4.8.4-alpha2 but fails on the 4.8.4 release binary that CI installs. Keeping `bote.tcyr` lean and not pulling those in dodges the version-skew issue while we work out what's different across 4.8.4 builds.
- Production build: `src/main.cyr -> bote 370480 bytes 684ms [x86]` with 15 undefined-fn warnings (same as 2.4.0 — libro heisenbug-avoidance stubs).
- `cyrlint src/auth.cyr src/dispatch.cyr src/codec.cyr src/bridge.cyr` → 0 warnings.
- Live HTTP: `./bote http 8390` + `curl -X POST -H 'Authorization: Bearer tok' ...` with `BOTE_BEARER_TOKENS=tok` — handler receives `args` + `claims=tok` (bearer middleware unchanged; just the plumbing downstream).
- 4.8.4 capacity meter on `tests/bote.tcyr`: `fn_table 1233/4096`, `identifiers 32312/131072` (25% used) — comfortable headroom for future 2.x features.

### Resolved from prior reports
- `docs/bugs/cyrius-4.8.3-regressions.md` — all three blockers closed in 4.8.4.
- `docs/development/roadmap.md` — the "waiting on 4.8.3 capacity meter" note can now be cleared; next refactor cycle has hard numbers.

### Carried forward
- v1.2.1 libro-growth heisenbug: unchanged.
- Slowloris recv timeout (audit H5): still needs `sock_set_recv_timeout` stdlib helper.
- WS handshake key-length validation (audit M4): upstream stdlib.
- DNS for SSRF hostname classification: still needs `getaddrinfo` stub.
- JWT RS256 / ES256: still waits on sigil RSA / ECDSA primitives.

## [2.4.0] — 2026-04-14 — Bump cyrius 4.8.1 + base64url adoption + compile-unit trim

Toolchain bump and structural cleanup. The cyrius 4.8.1 stdlib added
`base64url_encode` / `base64url_decode` (the proposal we shipped in
2.2.0 / cleaned up in 2.3.1 — your cyrius agent landed it). This
release adopts those primitives and trims compile-unit headroom enough
that future ABI work (claims propagation in 2.5) fits cleanly.

### Changed
- **cyrius pin** `4.7.1` → `4.8.1` (`cyrius.toml` + `.cyrius-toolchain`).
- **`src/jwt.cyr` lifts to stdlib base64url.** The inline `_jwt_b64u_val` table-builder + `jwt_b64u_decode` byte-loop (~55 LOC, 2 fns) is now a 7-line wrapper that calls `base64url_decode` from `lib/base64.cyr`. Test signatures unchanged — the wrapper translates the stdlib's `{ptr, len}` 16-byte pair to our existing `(ptr, *out_len)` shape so `tests/bote_jwt.tcyr` passes unchanged.
- **`src/main.cyr` trim.** Removed `lib/assert.cyr`, `lib/sakshi.cyr`, `lib/bigint.cyr` from the production main.cyr include list. None were actually referenced by main — they were transitively pulled in once-upon-a-time and never cleaned. Frees ~50 fns from production compile unit (was 1001 → 943 unreachable).
- **`tests/bote.tcyr` trim.** Removed `lib/sakshi.cyr`, `lib/bigint.cyr`, `lib/sigil.cyr`, all `lib/libro_*.cyr`, all `lib/majra_*.cyr` from the **core test** compile unit. Those modules were only needed by integration tests that already live in their own per-module test files (`tests/bote_libro_tools.tcyr`, etc.). Frees enough headroom (was 830 → 457 unreachable) that the larger 4.8.1 `lib/base64.cyr` (67 LOC → 177 LOC for the new base64url variants) doesn't tip the cap. The `&libro_audit_log` reference in `bote.tcyr` becomes an undefined-fn warning at compile — harmless because that test only checks fn-pointer addressability, never invokes.

### Verified (cyrius 4.8.1)
- All 8 test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_auth.tcyr` 38 / `bote_sandbox.tcyr` 13 / `bote_jwt.tcyr` 28 / `bote_pkce.tcyr` 17 = **603 total** (unchanged).
- `cyrius build src/main.cyr bote` → OK; `./bote` reports `"version":"2.4.0"`.
- `cyrlint src/jwt.cyr src/main.cyr` → 0 warnings.

### What this unlocks (deferred to 2.5.0)
- **Claims propagation through transports.** The plumbing change (auth_bearer_check → codec_process_message → dispatcher_dispatch all gain a `claims` arg, transports capture and thread the validator's return) was attempted in 2.3.0 and reverted because it tipped the test cap. With the trim landed here, **the freed headroom is enough that the same attempt should succeed** — left for 2.5.0 because the patch touches every transport handler and warrants its own focused release.

### Carried forward
- v1.2.1 libro-growth heisenbug: unchanged.
- Slowloris recv timeout (audit H5): still needs a `sock_set_recv_timeout` stdlib helper.
- WS handshake key-length validation (audit M4): still upstream stdlib.
- DNS for SSRF hostname classification: still needs cyrius `getaddrinfo` stub.
- JWT RS256 / ES256: still waits on sigil RSA / ECDSA primitives.

## [2.3.1] — 2026-04-14 — Cleanup: remove proposal docs that landed upstream

All three of bote's proposal documents have been folded into cyrius
stdlib. **`base64url_encode` / `base64url_decode` shipped in cyrius
4.8.1** (the ask in `cyrius-stdlib-base64url.md` from this release
cycle), joining the earlier two:

| Proposal | Landed in cyrius |
|---|---|
| `cyrius-stdlib-http-server.md` (with `lib_http_server.cyr` + example) | 4.5.0 (bote adopted in 1.3.0) |
| `cyrius-stdlib-ws-server.md` (with `lib_ws_server.cyr` + example) | 4.5.1 (bote adopted in 1.5.0) |
| `cyrius-stdlib-base64url.md` | **4.8.1** (bote will adopt in 2.4.x) |

### Removed
- `docs/proposals/` directory contents (all 7 files): the three `*.md` proposals + their companion `lib_*.cyr` reference impls / runnable examples. Workflow is preserved — future proposals will land back under the same path.

### Adoption note (deferred to 2.4.x)
- `src/jwt.cyr`'s inline `jwt_b64u_decode` (~50 LOC + a few fns) can now be replaced with a call to stdlib `lib/base64.cyr`'s `base64url_decode`. Lifting alongside the cyrius pin bump (4.7.1 → 4.8.x) will free meaningful identifier-table headroom — enough that the 2.3.0-era claims-propagation work (which tipped the test cap and was reverted) can land in 2.4.0.

### Verified
- All 8 test files green (603 total, unchanged from 2.3.0).
- `./bote` reports `"version":"2.3.1"`.

## [2.3.0] — 2026-04-14 — RFC 7636 PKCE-S256 helpers

OAuth 2.1 mandates PKCE on every authorization-code flow; this release
ships the verifier + S256 challenge primitives so MCP clients running
inside bote-hosted handlers can initiate the auth dance without
reimplementing the (small but easy-to-mess-up) crypto. Pairs with the
JWT verifier (2.2.0) and bearer middleware (1.9.0).

### Added
- **`src/pkce.cyr`** (~80 LOC, opt-in module):
  - `pkce_code_verifier(out_buf, len) → 0|err` — writes `len` URL-safe random bytes from `/dev/urandom`. RFC 7636 §4.1 length [43..128]; `out_buf` must be `len + 1` bytes (NUL-terminated). Returns non-zero on out-of-range len or entropy failure.
  - `pkce_code_challenge_s256(verifier) → cstr` — `base64url(sha256(verifier))`, no padding. Uses sigil's `sha256` one-shot. Returns a NUL-terminated 43-char cstr (32 bytes → 43 chars).

### Notes
- **S256 only.** OAuth 2.1 explicitly removes the `plain` method; we don't ship it.
- **Mod-bias.** Verifier byte mapping is `urandom_byte % 66` over the unreserved-character set. Worst-case bias per char is ~0.4% (256/66 = 3 remainder); for 43+-char tokens this is negligible to any guessing attack.
- **No SHA-256 reimplementation.** Calls sigil's existing `sha256(data, len, out)` one-shot — same approach as 2.2.0's HMAC use.

### Tests
- **`tests/bote_pkce.tcyr`** — 17 assertions:
  - Verifier length 43 (RFC minimum) + 128 (RFC maximum) succeed
  - Out-of-range (42, 129) rejected
  - Verifier bytes are all in the RFC unreserved set (`[A-Za-z0-9._~-]`)
  - Two consecutive verifiers differ (entropy sanity)
  - **RFC 7636 Appendix B reference vector**: verifier `dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk` → challenge `E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM`
  - Challenge is deterministic
  - Out-of-range / null verifier rejected
  - Fn-pointer addressability for plugging into auth flows
- **603 total** (was 586). Breakdown: `bote.tcyr` 394, `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 24, `bote_host.tcyr` 67, `bote_auth.tcyr` 38, `bote_sandbox.tcyr` 13, `bote_jwt.tcyr` 28, `bote_pkce.tcyr` 17.

### Verified (cyrius 4.7.1)
- All eight test files green.
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 2.2.
- `cyrlint src/pkce.cyr tests/bote_pkce.tcyr` → 0 warnings.
- `./bote` reports `"version":"2.3.0"`.

### Deferred from 2.3.0 to 2.4.0 — claims propagation through transports
Started this release; reverted before tagging. The plumbing change (auth_bearer_check → codec_process_message → dispatcher_dispatch all gain a `claims` arg, transports capture and thread the validator's return) all compiles and `src/main.cyr` builds cleanly — but it tipped `tests/bote.tcyr`'s compile unit past the cyrius 4.7.1 identifier-buffer cap (the misleading `lib/assert.cyr:3` cascade). Production runtime is unaffected; the test file is the cap-pressured one. Will land cleanly when cyrius 4.8.0 raises the cap.

### Roadmap
- **Claims propagation** → 2.4.0 once 4.8.0 lands.
- **OAuth 2.1 authorization-code endpoints** (Bote-as-AS rather than Bote-as-RS) — out of scope; bote is a resource server. Consumers wanting a full AS layer compose bote with their own authorization server.
- **JWT RS256 / ES256** — waits on sigil's RSA / ECDSA primitives.

## [2.2.0] — 2026-04-14 — JWT HS256 verifier (auth roadmap)

Lands the JWT bearer-token verifier from the 2.0 deferred list. Pairs
with the bearer middleware (1.9.0) — plug `auth_validator_jwt_hs256`
into `auth_bearer_check` and your bearer endpoint validates RFC 7519
JWTs against an HMAC-SHA256 secret.

### Added
- **`src/jwt.cyr`** (~170 LOC). Opt-in module; not wired into the default `main.cyr`. Consumers `include` it explicitly.
  - **`jwt_verify_hs256(token, secret, secret_len) → 0|1`** — full RFC 7515 §3.5 verification: 3-segment split, header decode + `alg=HS256` check (rejects `alg=none` downgrade), HMAC-SHA256 over `header.payload`, constant-time signature compare.
  - **`jwt_secret_new(secret) → handle`** + accessors. 16-byte handle, opaque to bote.
  - **`auth_validator_jwt_hs256(token, secret_handle) → claims | 0`** — drop-in for the bearer middleware fn-pointer slot. Returns the original token cstr as opaque "claims" on success.
  - **`jwt_b64u_decode(enc, elen, *out_len) → ptr | 0`** — RFC 4648 §5 base64url decoder. Inlined for now; will lift to stdlib `lib/base64.cyr` once `cyrius-stdlib-base64url.md` lands (proposal in `docs/proposals/`).
- **HMAC-SHA256 via sigil.** Calls sigil's existing `hmac_sha256` (already in the bundled `lib/sigil.cyr`) — bote doesn't reimplement. Saves ~30 LOC of inline crypto.

### Security properties
- **`alg=none` downgrade rejected** — the header is decoded and scanned for the literal `HS256`. A token with `{"alg":"none"}` cannot pass even with an empty signature.
- **Constant-time signature compare** — `_jwt_ct_eq` XOR-accumulates all 32 bytes with no early exit. Defeats the per-byte timing oracle.
- **Caller-supplied secret only** — bote doesn't store, log, or expose the secret. The validator handle holds a pointer + length; rotation is the caller's responsibility.

### Tests
- **`tests/bote_jwt.tcyr`** — 28 assertions:
  - base64url decode of RFC 4648 §10 vectors (`"Zm9v"` → `"foo"`, `"Zm9vYmFy"` → `"foobar"`)
  - JWT decode of the RFC 7515 §3.5 example header
  - URL-safe alphabet (`-` and `_` accepted)
  - **Canonical jwt.io HS256 token** (`SflKxwRJ...` with secret `your-256-bit-secret`) verifies — the standard reference vector for HS256 implementations
  - Wrong secret rejected; tampered signature rejected; tampered payload rejected
  - 2-segment / no-dot / null-token / null-secret / zero-length-secret all rejected
  - `alg=none` downgrade rejected
  - `JwtSecret` handle accessors, validator adapter returns claims on success / 0 on bad secret / 0 on null handle
  - Fn-pointer addressability for plugging into `auth_bearer_check`
- **586 total** (was 558). Breakdown: `bote.tcyr` 394, `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 24, `bote_host.tcyr` 67, `bote_auth.tcyr` 38, `bote_sandbox.tcyr` 13, `bote_jwt.tcyr` 28.

### Verified (cyrius 4.7.1)
- All seven test files green.
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 2.1.
- `cyrlint src/jwt.cyr tests/bote_jwt.tcyr` → 0 warnings.
- `./bote` reports `"version":"2.2.0"`.

### Proposed for cyrius 4.8.0
- **`base64url_encode` / `base64url_decode` in `lib/base64.cyr`** — see `docs/proposals/cyrius-stdlib-base64url.md`. Ship-ready reference impl + RFC 4648 §10 test vectors. Bote 2.2.x will lift its inline `jwt_b64u_decode` to the stdlib call once the cyrius agent folds it in (drops ~50 LOC + a few fns from bote's compile unit — meaningful for the long-running cyrius identifier-table cap pressure).

### Roadmap
- **OAuth 2.1 / PKCE-S256** still ahead — JWT HS256 is the verifier substrate; PKCE helpers are a future ship.
- **JWT RS256 / ES256** waits on sigil's RSA / ECDSA primitives.

## [2.1.0] — 2026-04-14 — Pluggable sandbox runner (kavach 3.0 compatible)

Lands the sandbox abstraction the 2.0 roadmap deferred. **`kavach` is at
3.0** (multi-backend: NOOP / process / OCI / gvisor / firecracker / SEV)
— the previous "waits on kavach v2" CHANGELOG line was stale. Bote now
ships the abstract `SandboxRunner` surface; consumers wire kavach (or
any other sandbox) behind a single fn-pointer + ctx, the same adapter
pattern used for `AuditSink` (libro) and `EventSink` (majra).

### Added
- **`src/sandbox.cyr`** (~50 LOC, no deps):
  - `sandbox_runner_new(run_fp, ctx)` — 16-byte handle.
  - `sandbox_run(s, command, timeout_ms)` — null-safe; returns the runner's JSON result or an error envelope. Suggested result shape matches kavach's `ExecResult`: `{"exit_code":N,"stdout":"...","stderr":"...","duration_ms":N,"timed_out":0|1}`.
  - `sandbox_runner_noop` + `sandbox_runner_noop_new()` — built-in adapter that echoes the command back as stdout (exit 0). Useful for tests and for environments where sandboxing isn't required.
- Validator signature: `fn run(ctx, command_cstr, timeout_ms) → result_cstr`. Tool authors call `sandbox_run(s, command, timeout)` from inside their handler body when they need to invoke an external process under isolation.

### Why a fn-pointer adapter, not a `[deps.kavach]` direct link
Kavach 3.0 is a substantial dependency (33 modules / ~7K LOC). Pulling it directly into bote's compile unit would tip the cyrius 4.7.1 identifier-table cap that's already constraining bote (see `docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`). The adapter pattern keeps bote independent — consumers that need kavach link it in their own `cyrius.toml` and write a 5-line `kavach_run_adapter(ctx, cmd, timeout) → ExecResult-as-JSON` function. Consumers that prefer a different sandbox (or none at all) drop in a different fn-pointer.

### Tests
- **New test file** — `tests/bote_sandbox.tcyr` (13 assertions). Covers runner shape (alloc + accessors), `sandbox_run` dispatching to a configured fp, null-safety on a null runner, the noop adapter (echo + JSON escaping), and fn-pointer addressability.
- **558 total** (was 545). Breakdown: `bote.tcyr` 394, `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 24, `bote_host.tcyr` 67, `bote_auth.tcyr` 38, `bote_sandbox.tcyr` 13.

### Verified (cyrius 4.7.1)
- All six test files green.
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 2.0.
- `cyrlint src/sandbox.cyr tests/bote_sandbox.tcyr` → 0 warnings.
- `./bote` reports `"version":"2.1.0"`.

### Roadmap note
- The earlier "kavach sandbox waits on kavach v2 hardening" item is **closed** — kavach 3.0 is shipped and bote 2.1.0 provides the integration substrate. Consumer-side adapter (~5 lines wrapping `noop_exec` / `process_exec` / `oci_exec` / etc.) is left to the consuming application — bote stays kavach-agnostic.

## [2.0.0] — 2026-04-14 — Stable: handler-claims ABI + carry-forward of all 1.9.x security work

The 1.x line ports bote from Rust to Cyrius and then iterates feature-by-feature; the 2.0 ship is the **first stable line** with a single deliberate ABI break (handler signature) so the auth → handler claims pipeline can land cleanly in 2.x without another major bump.

### Breaking
- **Handler signature**: `fn h(args_cstr) → result_cstr` → **`fn h(args_cstr, claims) → result_cstr`**. The new `claims` argument is opaque; in 2.0 it's always `0` (transports don't yet plumb the validator's return value down through `codec_process_message → dispatcher_dispatch → fncall2`). 2.x patch releases will populate it. Existing handlers must update their signature even if they ignore the second arg — `fn echo(args)` becomes `fn echo(args, claims)`. All bundled handlers (`bote_echo`, the five `libro_*` tools, all test/fuzz handlers) updated in this release.
- **Migration cost**: a single argument added to every tool handler the consumer registers. ~5 minutes per app.

### Carries forward (cumulative across 1.9.x — see per-release entries below)
- **Bearer-token middleware** (1.9.0) — fn-pointer + ctx validator, opt-in per HTTP-family transport, RFC 6750 401 with `WWW-Authenticate`.
- **Constant-time bearer compare + HTTP smuggling guard + batch cap + jsonx depth cap + `/dev/urandom`-or-fail** (1.9.4 audit batch A).
- **SSRF rewrite** — integer/octal/hex-IPv4 bypasses + IPv4-mapped-IPv6 + dot-consume verification (1.9.5 audit batch B). All 3 audit Criticals closed.
- **413 + bridge CORS oracle + Unix socket 0600** (1.9.6 audit polish).
- **`content_with_annotations`** — typed-block `audience` + `priority` annotations (1.9.6).

### Verified (cyrius 4.7.1)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_auth.tcyr` 38 = **545 total**.
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.9.x.
- `cyrlint src/*.cyr` → **0 warnings** across all sources.
- `cyrius fuzz fuzz/*.fcyr` → **4 passed, 0 failed**.
- `./bote` reports `"version":"2.0.0"`.

### Known carry-forward (deferred past 2.0)
- **Slowloris recv timeout** (audit H5) — needs a `sock_set_recv_timeout` helper in cyrius `lib/net.cyr` that doesn't exist. Workaround: deploy behind nginx/caddy.
- **WS handshake key-length validation** (audit M4) — lives in stdlib `lib/ws_server.cyr`; tracked upstream.
- **DNS resolution for SSRF hostname classification** — needs a cyrius `getaddrinfo` stub. Production callers should pair with a network-policy egress block.
- **Bridge optional protocol-version gate** (audit M5) — tried for 1.9.6, tipped the cyrius 4.7.1 cap; revisit when 4.8.0 raises room.
- **`libro_tools` default registration** — turned opt-in via `BOTE_LIBRO=1` env var in 1.9.4 to free identifier-table headroom; restore default-on when 4.8.0 lands.
- **Threaded streaming dispatch / WS arena allocator / `kavach` sandbox** — wait on cyrius primitives.
- **OAuth 2.1 / PKCE / JWT verifier** — bearer substrate is the hook; flows + verifier are the next net-new feature work.
- **Claims propagation through transports** — handler ABI is in place (this release); auth → dispatch plumbing follows in 2.1.
- **v1.2.1 libro-growth heisenbug** — unchanged.

### Carried-forward release map (1.x → 2.0)

| Tag | Headline |
|---|---|
| 1.0.0 | Cyrius port — protocol core, registry, dispatch, schema, codec, sessions, four transports |
| 1.1.0 | AuditSink + EventSink fn-ptr+ctx adapters |
| 1.2.0 | LibroAudit + MajraEvents adapters via `[deps.libro]` + `[deps.majra]` |
| 1.3.0 | Adopt cyrius 4.5.0 stdlib `lib/http_server.cyr` |
| 1.4.0 | Streamable HTTP transport (MCP 2025-11-25) |
| 1.5.0 | WebSocket transport (RFC 6455) on stdlib `lib/ws_server.cyr` |
| 1.5.1 | P(-1) hardening — HTTP body clamp + null-guard sweep |
| 1.6.0 | `libro_tools` — 5 built-in MCP audit tools |
| 1.7.0 | Typed MCP content blocks |
| 1.8.0 | `HostRegistry` + IPv4 `ssrf_check` |
| 1.8.1 | Bump cyrius 4.6.2 |
| 1.9.0 | Bearer-token middleware (RFC 6750) |
| 1.9.1 | IPv6 SSRF + binary-blob resource + env-driven CLI bearer |
| 1.9.2 | Bump cyrius 4.7.0 |
| 1.9.3 | Bump cyrius 4.7.1 + 2.0-prep doc sweep |
| 1.9.4 | Security batch A — 5 audit findings |
| 1.9.5 | Security batch B — 3 SSRF criticals |
| 1.9.6 | Final polish — 413, CORS oracle, Unix 0600, annotations |
| **2.0.0** | **Handler-claims ABI** + the 1.9.x carry-forward |

---

## [1.9.6] — 2026-04-14 — Final pre-2.0 polish: contained audit items + annotations

Closes the audit items that didn't need a cyrius stdlib helper, plus
re-lands the `content_with_annotations` work that was reverted from
1.9.1 / 1.9.4 for cap-headroom reasons. Last 1.9.x patch — next stop is
**2.0.0** (claims propagation through handler signature).

### Security
- **M1 — 413 Payload Too Large on oversized declared bodies.** `transport_http`, `transport_streamable`, and `bridge` now reject any request whose `Content-Length` declares more than 60 KB (60 × 1024 = 61440) — well under the 64 KB recv buffer with room for headers. Previous behaviour silently truncated and treated as a malformed 400; new behaviour returns the spec-correct status. Live smoke: 100 KB POST → 413 ✓.
- **M3 — bridge CORS oracle.** `_bridge_cors_origin` previously returned `vec_get(allowed_origins, 0)` (the first allowed origin) on a miss — leaks the allowed-origin list to any cross-origin requester. New behaviour returns the literal string `"null"` (a spec-compliant value; browser will block the response either way). Existing test updated.
- **L1 — Unix socket file mode.** `transport_unix_run` now `chmod(path, 0600)` post-bind so only the owning UID can connect. Previous behaviour inherited the process umask (typically 0022 → mode 0755) and let any local user dial the socket. Live smoke: socket file shows `srw-------` ✓.

### Added
- **`content_with_annotations(block, audience, priority)`** — re-landed from the 1.9.1 / 1.9.4 deferral. Splices an `annotations` field into any pre-built block cstr right before the trailing `}`. Audience is a vec of cstr (e.g. `"user"` / `"assistant"`); priority is an i64 in [0, 100] (`-1` = unset). Either argument may be null/-1; if both are absent the input is returned unchanged. **6 new test assertions** including no-op pass-through, audience-only, priority-only, both, image-block annotation, and null-block guard.

### Audit findings still open after 1.9.6
- **High — Slowloris recv timeout** (H5). Needs a `sock_set_recv_timeout` helper in `lib/net.cyr`. Workaround: deploy behind nginx/caddy that absorbs slow connections.
- **Medium — bridge optional protocol-version gate** (M5). Was attempted in 1.9.6 but added 2 new bridge_config_* fns that tipped the test compile unit past the cyrius 4.7.1 cap. Bridge is local-/single-app-scoped in practice; the bearer-token gate already covers production deployments. Will revisit when cyrius 4.8.0 raises the cap.
- **Medium — WS `Sec-WebSocket-Key` length validation** (M4). Lives in stdlib `lib/ws_server.cyr`; bote can't patch without forking. Tracked upstream.

### Verified (cyrius 4.7.1)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 24 / `bote_host.tcyr` 67 / `bote_auth.tcyr` 38 = **545 total** (was 539 at 1.9.5; +6 annotation assertions).
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.9.5.
- `cyrlint src/*.cyr` → 0 warnings across all sources.
- Live HTTP smoke: normal POST 200, 100 KB POST 413; Unix socket created with mode 0600.
- `./bote` reports `"version":"1.9.6"`.

### Up next: 2.0.0
- **Claims propagation through handler signature.** Handler currently `fn(args_cstr) → result_cstr`; 2.0 threads the validator's claims through dispatch so handlers can authorize per-tool. ABI break (warrants the major bump).
- **Final P(-1) hardening sweep.**
- **Restore `libro_tools` default registration** (was made opt-in in 1.9.4 for cap headroom; cyrius 4.8.0 should give us room).

## [1.9.5] — 2026-04-14 — Security batch B: SSRF rewrite (3 critical bypasses)

Closes the three Critical findings from the 2026-04-14 audit
(`docs/audit/2026-04-14.md`). All three are SSRF bypasses that landed
on cloud-metadata / loopback endpoints despite the 1.8.0+ blocklist.

### Security
- **C1 — integer-form IPv4 bypass.** `http://2130706433/` (the integer
  encoding of 127.0.0.1) was reaching the hostname classifier as a
  0-dot host and passing as `SSRF_OK` because the blocklist only knew
  literal `localhost` / `metadata`. **Fix**: hostname classifier now
  rejects all-`[0-9.]` hosts as `SSRF_PARSE`. Also catches short-form
  IPv4 (`127.1`, `127.1.1`).
- **C2 — octal IPv4 bypass.** `_ssrf_parse_octet` accepted leading-zero
  multi-digit forms: `0177` parsed as decimal 177 (apparently public)
  while glibc's `inet_aton` interprets the same bytes as octal 127 →
  loopback. **Fix**: octet parser now rejects digits>1 starting with
  zero (`00`, `01`, `0177`, `010` all invalid; `0`, `127`, `255`
  still valid).
- **C3 — IPv4-mapped IPv6 bypass.** `http://[::ffff:127.0.0.1]/` and
  `http://[::ffff:7f00:1]/` were classified by the IPv6 prefix-blocklist
  which only matched `::1`, `::`, `fe80:`, `fc/fd`, `ff` exact —
  `::ffff:` v4-mapped fell through as `SSRF_OK`. Also: the IPv4
  classifier's `_ssrf_parse_octet` didn't verify it consumed the full
  byte range up to the next dot, so `64:ff9b::1.2.3.4` (NAT64 form)
  parsed as `64.2.3.4` (public) and never reached the IPv6 path.
  **Fix**: IPv6 classifier now blocks `::ffff:`, `::*.*.*.*` (v4-compat),
  `64:ff9b:` (NAT64 well-known) outright; IPv4 classifier requires each
  octet parse to consume exactly the dot-to-dot range.

### Tests
- **11 new host assertions** for the bypasses: integer IPv4, octal IPv4
  (`0177.0.0.1`, `0010.0.0.1`), short-form IPv4 (`127.1`),
  `::ffff:127.0.0.1`, `::ffff:7f00:1`, `::127.0.0.1`, NAT64
  `64:ff9b::1.2.3.4`, plus regression checks for canonical decimal
  IPv4 still passing (`10.0.0.1` → PRIVATE, `1.2.3.4` → OK).
- **539 total** (was 528). Breakdown: `bote.tcyr` 394,
  `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 18,
  `bote_host.tcyr` 67 (was 56), `bote_auth.tcyr` 38.

### Audit report
- **`docs/audit/2026-04-14.md`** — full findings list (3 critical,
  5 high, 5 medium, 1 informational), audit-driven release map, and
  follow-up notes for items deferred to 2.0.

### Verified (cyrius 4.7.1)
- All five test files green (539 total).
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.9.4.
- `cyrlint src/host.cyr tests/bote_host.tcyr` → 0 warnings.
- `./bote` reports `"version":"1.9.5"`.

### Audit findings still open (deferred to 2.0)
- **High — Slowloris** (recv timeout — needs `sock_set_recv_timeout` stdlib helper)
- **Medium — bridge CORS oracle**, **WS handshake key-length validation** (upstream stdlib), **bridge protocol-version gate**, **413 cap on oversized requests**
- **Informational — Unix socket default umask** (chmod 0600 missing)

See `docs/audit/2026-04-14.md` for the full list and audit-driven
release map.

## [1.9.4] — 2026-04-14 — Security batch A (audit-driven)

First slice of the 2.0-prep security pass. Closes 4 of the 5 audit
findings in batch A; the slowloris recv-timeout fix needs a
`sock_set_recv_timeout` stdlib helper that doesn't exist yet, so it's
deferred to 1.9.5 / 2.0.

### Security
- **HTTP request smuggling guard** (RFC 9112 §6 / CVE-2019-18276 family). All four HTTP-family transports (`http` / `bridge` / `streamable` / `ws`) now reject any inbound request that carries a `Transfer-Encoding` header — bote doesn't dechunk, and rejecting any TE eliminates the CL.TE / TE.CL ambiguity. Inlined as a 3-line guard in each handler (rather than a shared helper) to stay under cyrius 4.7.1's identifier-table headroom.
- **Constant-time bearer-token comparison** (`auth_validator_allowlist`). The previous `streq`-based compare leaked per-byte timing, letting a network attacker byte-by-byte-guess a token against the allowlist. New implementation: lengths are still compared first (token length leaks; unavoidable), but the byte-loop XOR-accumulates with no early exit and the outer vec walk doesn't short-circuit on match — defeats both the per-byte oracle and the position-of-match oracle. Both helpers are inlined into `auth_validator_allowlist` itself (rather than a separate `_auth_ct_eq` fn) for the same identifier-table reason.
- **Batch-size cap** (`src/codec.cyr::codec_process_message`). Hardcoded `n > 100` check on JSON-RPC array batches — defends against a 1 MiB body of `[{},{},...]` (~300k elements) that would otherwise spin allocating per-element responses until OOM. The literal `100` is inlined to avoid a top-level `var` symbol; 100 is generous (real clients batch single-digit calls).
- **JSON nesting depth cap** (`src/jsonx.cyr::_jx_skip_struct`). Hardcoded `depth > 64` returns end-of-buffer rather than walking adversarial deep structures. The 64 KB inbound HTTP buffer already caps the absolute worst case, but the explicit guard keeps us safe if anyone raises the buffer.
- **`/dev/urandom`-or-fail** (`src/session.cyr::_gen_session_id`). The previous fallback used `clock_now_ns()` (predictable to any timing observer) when `/dev/urandom` failed to open. New behaviour: refuse to mint a session ID — write a fatal error and `SYS_EXIT(90)`. Also loops the read until 16 bytes are received (was a single `syscall(SYS_READ, ...)` that could short-read and leave uninitialized bytes in the SID material).

### Changed
- **`libro_tools` no longer registered by default at startup.** Made opt-in via `BOTE_LIBRO=1` env var to free identifier-table headroom for the security inlines above. Consumers who want the five `libro_*` MCP tools just set `BOTE_LIBRO=1` in their environment, or include `src/libro_tools.cyr` + call `libro_tools_init` / `libro_tools_register` from their own `main.cyr`. **This is a behaviour regression** — minor for most consumers (the tools were registered against an empty chain anyway). Will revert to default-on when cyrius lifts the cap further (4.8.0+).

### Tests
- **9 new auth assertions** for the constant-time compare: first-byte-differ, last-byte-differ, shorter, longer, position-independence across multi-entry allowlists. **528 total** (was 519). Breakdown: `bote.tcyr` 394, `bote_libro_tools.tcyr` 22, `bote_content.tcyr` 18, `bote_host.tcyr` 56, `bote_auth.tcyr` 38.
- Live HTTP smoke: normal POST → 200 ✓; POST with `Transfer-Encoding: chunked` → 400 ✓; 150-element batch → `-32600` "batch too large" ✓.

### Verified (cyrius 4.7.1)
- All five test files green (528 total).
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.9.3.
- `cyrlint src/*.cyr` → 0 warnings.
- `./bote` reports `"version":"1.9.4"`.

### Audit findings still open (deferred to 1.9.5 / 2.0)
- **Critical (3) — SSRF bypasses**: integer/decimal IPv4 (`http://2130706433/`), octal/hex IPv4 (`http://0177.0.0.1/`), IPv4-mapped IPv6 (`http://[::ffff:127.0.0.1]/`). Need a coherent rewrite of the host parser; queued for 1.9.5.
- **High — Slowloris** (single-byte-then-pause holds the accept loop). Needs a `sock_set_recv_timeout` stdlib helper.
- **Medium — bridge CORS oracle** (echoes `allowed_origins[0]` on miss), **WS handshake doesn't validate `Sec-WebSocket-Key` length**, **bridge skips protocol/session checks**.
- **Informational — Unix socket created with default umask perms** (chmod 0600 not set).

See `docs/development/roadmap.md` for the 1.9.5 / 2.0 plan.

## [1.9.3] — 2026-04-14 — Bump pin to cyrius 4.7.1 + 2.0-prep doc sweep

Toolchain bump + a comprehensive 2.0-prep documentation pass. No
behavioral source changes; per-module test-file layout stays because
even cyrius 4.7.1's `BUILD_METHOD_NAME` scratch-corruption fix doesn't
cover bote's specific overflow path.

### Changed
- **cyrius pin** `4.7.0` → `4.7.1` (`cyrius.toml` + `.cyrius-toolchain`).
- `src/dispatch.cyr` — `_bote_server_version` → `"1.9.3"`.

### Documentation
- **`README.md`** rewritten for current state — six transports, bearer auth, all built-in tools, full module list, 519 tests, env-driven CLI auth quickstart.
- **`docs/architecture/overview.md`** rewritten — ASCII diagram updated for six transports + auth/content/host outbound utilities + sinks + libro_tools surface, full module listing, adapter pattern documented.
- **`docs/development/roadmap.md`** rewritten — shipped-per-release table for 1.0.0 → 1.9.2, explicit "must-have for 2.0" / "nice-to-have for 2.0" / "deferred past 2.0" sections, cyrius-language-dependency status all marked ✅ for items resolved.
- **`docs/spec-compliance.md`** rewritten — every compliance category extended for 1.4.0+ work (streamable HTTP, WS, content blocks, SSRF, bearer middleware, host registry, env-driven auth, `libro_tools`). Adds a content-block subtable and a host/SSRF subtable.
- **`docs/benchmarks-rust-v-cyrius.md`** rewritten — Rust v0.92.0 final history-log entry (2026-04-03) vs Cyrius 1.9.2 / cyrius 4.7.0 numbers side-by-side. New "Net call" + "Where Cyrius wins decisively" framing.
- **`docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`** updated — 4.7.1 status header documents that bote's case is still uncovered (~1339 fns under the 4096 cap, so the bug is elsewhere — possibly identifier-bytes or a different scratch path).

### What we got from 4.7.1
- Function-table cap raised 2048 → 4096.
- `BUILD_METHOD_NAME` scratch corruption fix — directly addresses the misleading-error class we reported.

### What 4.7.1 still doesn't cover (for bote)
- `tests/bote.tcyr` + `lib/ws_server.cyr` still trips `lib/assert.cyr:3: expected '=', got string`. Re-verified with freshly-bootstrapped `cc3 4.7.1`. Per-module test layout stays.

### Verified (cyrius 4.7.1)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 18 / `bote_host.tcyr` 56 / `bote_auth.tcyr` 29 = **519 total** (unchanged).
- `cyrius build src/main.cyr bote` → OK; `./bote` reports `"version":"1.9.3"`.
- `cyrlint src/*.cyr` → 0 warnings.

### Up next (2.0-prep, per the audit)
1. **1.9.4** — security batch A: HTTP Transfer-Encoding rejection + recv timeouts (smuggling + slowloris), constant-time bearer compare, batch-size cap, jsonx depth cap, `/dev/urandom`-or-fail.
2. **1.9.5** — SSRF rewrite: canonical-IPv4-only parser (rejects octal/hex/integer/short-form), full IPv6 16-byte classifier with `::ffff:` v4-mapped, optional getaddrinfo single-shot pre-classification.
3. **2.0.0** — `content_with_annotations`, claims propagation through handler signature, final P(-1) sweep, tag.

### Carried forward
- Identifier-buffer / scratch-corruption — bote-specific case still uncovered in 4.7.1; tracked in `docs/bugs/`.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.9.2] — 2026-04-14 — Bump pin to cyrius 4.7.0

Toolchain bump only. No source changes — bote doesn't use 4.7.0's
headline feature (`shared;` / `.so` output) and the identifier-buffer
ceiling that's been pinning the test split + the deferred annotations
work isn't lifted until 4.7.1 (in flight upstream).

### Changed
- **cyrius pin** `4.6.2` → `4.7.0` (`cyrius.toml` + `.cyrius-toolchain`).
- `src/dispatch.cyr` — `_bote_server_version` → `"1.9.2"`.

### What we got from 4.7.0
- Real `dlopen`-able `.so` end-to-end (`shared;` mode) — not used by bote today, will matter when we factor a tool out as a loadable module.
- `DT_INIT` runs top-level initializers on dlopen, PIC-safe addressing in shared mode, full `.dynamic`/`.dynsym`/`.hash` emission.
- Shared-mode DCE fix (parallel to the 4.6.0-beta2 object-mode fix).

### What we're waiting for in 4.7.1
- Identifier-buffer ceiling raise. Bote currently sits one feature past it: every attempt to add `content_with_annotations` (deferred from 1.9.1) trips the same misleading `lib/assert.cyr:3: expected '=', got string` cascade. Will land + collapse the per-module test-file split when 4.7.1 ships.

### Verified (cyrius 4.7.0)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 18 / `bote_host.tcyr` 56 / `bote_auth.tcyr` 29 = **519 total** (unchanged).
- `cyrius build src/main.cyr bote` → OK; `./bote` reports `"version":"1.9.2"`.
- `cyrlint src/*.cyr` → 0 warnings.

### Carried forward
- cyrius identifier-buffer ceiling — fix in flight for 4.7.1.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.9.1] — 2026-04-14 — IPv6 SSRF + binary-blob resource + env-driven bearer auth

Closes three deferred items from earlier releases. A fourth (block annotations) was tried and reverted — it tipped the cyrius 4.6.2 identifier-buffer ceiling. Will land in 2.0 / when 4.7.0 frees up headroom.

### Added
- **IPv6 SSRF blocklist** (`src/host.cyr`):
  - `_ssrf_classify_ipv6` covers `::1` (loopback), `::` (unspec), `fe80::/10` (link-local), `fc00::/7` ULA private (`fc` and `fd` prefixes), `ff00::/8` (multicast)
  - `_ssrf_extract_host` now parses bracket form (`http://[::1]:8080/...`); malformed bracket → `SSRF_PARSE`
  - `ssrf_check` routes hosts containing `:` (after IPv4 fallback) to the IPv6 classifier
  - 9 new test assertions (`tests/bote_host.tcyr` 47 → 56)
- **Binary resource (`blob`) variant** (`src/content.cyr`):
  - `content_resource_blob(uri, mime, b64_data)` emits `{"type":"resource","resource":{"uri":"...","mimeType":"...","blob":"..."}}` — the spec-prescribed shape for non-UTF-8 resource bodies. Optional fields omitted from output when null, same as `content_resource`.
  - 3 new test assertions (`tests/bote_content.tcyr` 15 → 18)
- **CLI bearer-auth via `BOTE_BEARER_TOKENS` env var** (`src/main.cyr`):
  - `_split_csv_tokens(csv)` parses comma-separated values into a vec of cstrs
  - `_bote_bearer_from_env` reads `BOTE_BEARER_TOKENS`, builds the allowlist + validator, returns fp + ctx via out-params
  - All four HTTP-family transports (`http` / `bridge` / `streamable` / `ws`) auto-wire `auth_validator_allowlist` when the env var is set
  - Stdio + Unix-socket transports skip auth (they're local-only)
  - Backward compatible: env var unset → no behaviour change

### Verified (cyrius 4.6.2)
- All five test files green: `bote.tcyr` 394 / `bote_libro_tools.tcyr` 22 / `bote_content.tcyr` 18 / `bote_host.tcyr` 56 / `bote_auth.tcyr` 29 = **519 total** (was 507).
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.9.0 baseline.
- `cyrlint src/*.cyr` → **0 warnings** across all sources.
- Live HTTP smoke with `BOTE_BEARER_TOKENS=tok-a,tok-b`: `POST /mcp` with no header → 401, with `Authorization: Bearer wrong` → 401, with `Authorization: Bearer tok-a` → 200 + serverInfo `"version":"1.9.1"`. Empty/unset env var → unchanged 200 with no auth.

### Reverted from 1.9.1 scope
- **`content_with_annotations`** (audience + priority MCP annotations on any block). The pre-built-block-splice approach added enough symbols to push `src/main.cyr`'s compile unit past the cyrius 4.6.2 identifier-buffer ceiling. Reverted to keep the build green; will revisit when cyrius 4.7.0 lands.

### Carried forward
- cyrius 4.6.2 identifier-buffer ceiling — bote's full compile unit sits ~one feature away from the new cap. 4.7.0 expected to provide more room.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.9.0] — 2026-04-14 — Bearer-token middleware (RFC 6750)

First slice of the roadmap `auth` item. OAuth 2.1 + PKCE follow in
later releases; this one delivers the substrate they all sit on:
extract `Authorization: Bearer <token>` from a request, hand the token
to a caller-supplied validator function pointer, and emit a spec-
compliant 401 if the token is missing or rejected. **Opt-in** — a
transport with no validator configured behaves exactly as before.

### Added
- **`src/auth.cyr`** (~140 LOC, no AGNOS deps):
  - `auth_bearer_extract(buf, blen)` — case-insensitive `Bearer ` scheme parse, leading/trailing OWS handling, returns the alloc'd token cstr or 0.
  - `auth_bearer_check(cfd, buf, blen, validator_fp, validator_ctx)` — middleware entry. Returns 0 on pass / `HTTP_UNAUTHORIZED` on reject (response already on the wire). No-op when `validator_fp == 0`.
  - `auth_send_unauthorized(cfd, realm)` — 401 with `WWW-Authenticate: Bearer realm="..."`.
  - `auth_validator_allow_all(token, ctx)` — pass-anything validator (testing / dev only).
  - `auth_validator_allowlist(token, vec)` — vec membership check; rejects null/empty.
- Validator signature: `fn validator(token_cstr, ctx) → claims_ptr | 0`. Non-zero return is "valid"; the value is opaque to bote today and will be threaded through to handlers when request-scoped context lands. Returning 0 means "reject".

### Changed
- All four HTTP-family transport configs now have `bearer_validator` + `bearer_ctx` slots **at the end** of the struct (existing offsets preserved):
  - **`HttpConfig`** 56 → 72 bytes (`+56` validator, `+64` ctx)
  - **`BridgeConfig`** 32 → 48 bytes (`+32`, `+40`)
  - **`StreamableConfig`** 64 → 80 bytes (`+64`, `+72`)
  - **`WsConfig`** 48 → 64 bytes (`+48`, `+56` — applied to the upgrade HTTP request only, not per-frame)
- Each gets a corresponding `X_config_with_bearer_validator(c, fp, ctx)` setter and `X_config_bearer_validator(c)` / `X_config_bearer_ctx(c)` accessors.
- Each transport's per-request handler now calls `auth_bearer_check` right after the Origin check and before the protocol-version check.
- `src/main.cyr` includes `src/auth.cyr` ahead of the transports that use it.

### Tests
- **New test file** — `tests/bote_auth.tcyr` (29 assertions). Covers:
  - Header parsing — exact match, three case variants of the scheme, leading/trailing whitespace handling
  - Rejections — no header / Basic scheme / `"Bearer"` with no space / empty token / whitespace-only token
  - `auth_validator_allow_all` accepts any non-empty token, rejects null/empty
  - `auth_validator_allowlist` accepts on match, rejects misses + null inputs
  - Middleware: no-validator → pass, valid token → pass, missing/wrong token → 401
  - Fn-pointer addressability (sanity)
- **Total assertions: 507** (was 478). Breakdown: `tests/bote.tcyr` 394, `tests/bote_libro_tools.tcyr` 22, `tests/bote_content.tcyr` 15, `tests/bote_host.tcyr` 47, `tests/bote_auth.tcyr` 29.

### Verified (cyrius 4.6.2)
- All five test files green (394 / 22 / 15 / 47 / 29).
- `cyrius bench tests/bote.bcyr` → 10 hot paths within noise of 1.8.1 (dispatch_* 1–3µs, jsonx_* 584–877ns, codec_* 789ns–6µs, validate_* 982ns–2µs).
- `cyrlint src/auth.cyr tests/bote_auth.tcyr src/transport_http.cyr src/transport_streamable.cyr src/transport_ws.cyr src/bridge.cyr` → **0 warnings**.
- Live HTTP smoke (`./bote http 18900` with no validator): `POST /mcp` returns 200 with serverInfo as before; auth machinery has zero overhead when not configured.
- `./bote` reports `"version":"1.9.0"`.

### Deferred to v1.9.x / v2.0
- **OAuth 2.1 + PKCE-S256.** Token *acquisition*, not just *validation*. Will reuse the same validator fn-ptr surface for verification.
- **Claims propagation to handlers.** Today a successful validate just returns 1; richer claims (subject, scopes) need request-scoped context plumbing into the handler signature, which is a bigger ABI change.
- **JWT verification helper.** Worth shipping when the first consumer needs it; keeps the validator surface lean for now.
- **CLI flag to enable bearer auth from the command line.** Programmatic callers can use `X_config_with_bearer_validator` today; CLI wiring follows once we pick a config-file format.

### Carried forward
- cyrius 4.6.2 function-table cap — fifth split test file added; addressed structurally.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.8.1] — 2026-04-14 — Bump to cyrius 4.6.2

Toolchain bump. No source changes beyond the pin + the test-file comment
that describes why the per-module test split is now the permanent
layout rather than a workaround.

### Changed
- **cyrius pin** `4.5.1` → `4.6.2` (`cyrius.toml` + `.cyrius-toolchain`).
- **`src/dispatch.cyr`** — `_bote_server_version` → `"1.8.1"`.
- **`tests/bote.tcyr`** comment updated to explain that per-module test files are the permanent layout. `lib/ws_server.cyr` stays out of the shared compile unit because bote's dep graph (`[deps.libro]` + `[deps.majra]` + `lib/sigil.cyr` alone at 354 fns + 15 stdlib modules + 15 bote sources) already lands near 4.6.2's 2048-fn function-table ceiling — ws_server's 16 fns tip it over.
- **`docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`** — added a 4.6.2 status header: identifier buffer *was* raised, the original repro now trips the function-table cap with a clean diagnostic, but the 4.6.1 diagnostic fix doesn't cover the specific overflow path bote hits.

### What we got from 4.6.1 / 4.6.2
- ✅ Identifier buffer raised (~60 KB headroom) — resolves the 1.5.0-era class of error.
- ✅ Clean `function table full (2048/2048)` diagnostic on the original repro.
- 🟡 Diagnostic fix doesn't cover every overflow path — bote still sees the misleading `lib/assert.cyr:3: expected '=', got string` when its full test unit + ws_server is compiled. Documented for the cyrius agent to take another pass.

### Verified (cyrius 4.6.2)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius test tests/bote_libro_tools.tcyr` → **22 passed, 0 failed**
- `cyrius test tests/bote_content.tcyr` → **15 passed, 0 failed**
- `cyrius test tests/bote_host.tcyr` → **47 passed, 0 failed** (478 total, unchanged)
- `cyrius build src/main.cyr bote` → OK; `./bote` reports `"version":"1.8.1"`.
- `cyrlint src/*.cyr` → 0 warnings.

### Carried forward
- cyrius 4.6.2 function-table cap limits the shared test compile unit — addressed structurally via per-module test files; waiting on either another cyrius-side raise or a compile-unit-level DCE that prunes unreferenced fns from counted totals.
- v1.2.1 libro-growth heisenbug: unchanged.

## [1.8.0] — 2026-04-14 — HostRegistry + SSRF guard

Closes out the `host` module started in 1.7.0. The registry gives
handlers a named list of external hosts bote is permitted to reach;
the SSRF guard rejects URLs targeting loopback / private / link-local
/ cloud-metadata endpoints *before* any network call goes out.

### Added
- **`src/host.cyr`** (~260 LOC, no AGNOS deps):
  - **`HostEntry`** (32 bytes): `name` / `url` / optional `headers` vec (alternating key/value cstrs) / optional `capabilities` vec. `host_entry_allows(entry, cap)` is an allowlist check — no-caps means "anything allowed" (fail-open for convenience); an explicit vec enforces the allowlist.
  - **`HostRegistry`** (16 bytes): `name → HostEntry` map backed by stdlib `hashmap`, plus a cached `count` that stays stable on replacement so callers trust it as an O(1) size hint.
  - `host_registry_new/add/get/has/count/names`.
- **`ssrf_check(url)`** — returns `SSRF_OK` (0) on pass, or a non-zero reason code:

  | Code | Meaning |
  |------|---------|
  | `SSRF_PARSE` | Malformed URL / empty host |
  | `SSRF_SCHEME` | Scheme isn't `http://` or `https://` |
  | `SSRF_LOOPBACK` | `127.0.0.0/8`, `localhost` |
  | `SSRF_LINK_LOCAL` | `169.254.0.0/16` (reserved for link-local generally) |
  | `SSRF_PRIVATE` | RFC 1918 (`10/8`, `172.16/12`, `192.168/16`) |
  | `SSRF_METADATA` | `169.254.169.254` (AWS/GCP/Azure), hostname `metadata.google.internal`, bare `metadata` |
  | `SSRF_UNSPEC` | `0.0.0.0/8` |
  | `SSRF_MULTICAST` | `224.0.0.0/4` |

  Case-insensitive on scheme + hostname. Strips `user:pass@` userinfo before classifying. Parses dotted-decimal IPv4 literals directly (no DNS). Non-IP hostnames hit a conservative string blocklist — this is defense-in-depth, not the last line; production callers should pair with DNS-level controls.
- Convenience: `ssrf_is_safe(url)` returns `1`/`0` for call sites that only need a boolean.

### Tests
- **New test file** — `tests/bote_host.tcyr` (47 assertions). Covers:
  - `HostEntry` / `HostRegistry` shape, replace-doesn't-double-count semantics, capability allowlist behaviour including the no-caps fail-open default
  - SSRF pass cases (`api.github.com`, `1.1.1.1`, `8.8.8.8`, port + query, `user:pass@` userinfo)
  - Every blocklist code path with a representative IPv4 (inc. edge cases: `172.15` & `172.32` public, `172.16` & `172.31` private)
  - Hostname blocklist (`localhost`, case-insensitive, `metadata.google.internal`, bare `metadata`)
  - Non-http schemes rejected (`file://`, `gopher://`, `ftp://`)
  - Null / malformed input
- **Total assertions: 478** (was 431). Breakdown: `tests/bote.tcyr` 394, `tests/bote_libro_tools.tcyr` 22, `tests/bote_content.tcyr` 15, `tests/bote_host.tcyr` 47.

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius test tests/bote_libro_tools.tcyr` → **22 passed, 0 failed**
- `cyrius test tests/bote_content.tcyr` → **15 passed, 0 failed**
- `cyrius test tests/bote_host.tcyr` → **47 passed, 0 failed**
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.7.0 baseline.
- `cyrlint src/host.cyr tests/bote_host.tcyr` → **0 warnings**.
- `./bote` — `initialize` → `{"serverInfo":{"name":"bote","version":"1.8.0"}}`.

### Deferred
- **DNS resolution for hostname classification.** A hostname that resolves to a blocked IP isn't caught today — caller can feed `127.0.0.1.nip.io` and pass. Requires a DNS stub on cyrius that doesn't yet exist; queued for a later release. In the meantime, pair with a network policy that blocks egress to RFC 1918.
- **IPv6 literal classification** (`::1`, `fe80::/10`, `fc00::/7`). Skipped for 1.8.0 — IPv4 covers today's deployments; IPv6 blocklist is next slice.
- **Registry persistence / hot-reload.** Registry is built in-process from config; no file watch.

### Known (unchanged)
- cyrius 4.5.1 identifier-buffer cap — this release needed a *fourth* split test file (`bote_host.tcyr`). All four collapse back into `bote.tcyr` when cyrius 4.6.1 lifts the cap. See `docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`.
- v1.2.1 libro-growth heisenbug — unrelated to this release.

## [1.7.0] — 2026-04-14 — Typed content blocks (MCP 2025-11-25)

Handlers can now return **typed content** — `text`, `image`, `audio`,
`resource` (embedded), and `resource_link` (reference) — instead of
only plain-text tool results. First piece of the `src/host.cyr`
roadmap item; host registry + SSRF follow in a later release.

### Added
- **`src/content.cyr`** (~135 LOC). Constructors for every MCP block type:
  - `content_text(text)` — `{"type":"text","text":"..."}`
  - `content_image(b64_data, mime)` / `content_audio(b64_data, mime)` — binary payloads, base64 in-band
  - `content_resource(uri, mime, text)` — embedded resource; `mime` and `text` are optional and omitted from the emitted object when null
  - `content_resource_link(uri, name, mime)` — reference only; client fetches by URI
  - `content_array(blocks)` — `{"content":[...]}` envelope over a vec of pre-built block cstrs
  - `content_array_error(blocks)` — same, with `"isError":true` (MCP distinguishes tool-execution errors from protocol errors by this flag)
  - `content_single(block)` and `content_text_response(text)` — shorthand for the single-block case (no vec alloc)
- Every string argument is a cstr; JSON escaping happens at the boundary via `_json_emit_escaped` (reused from `src/dispatch.cyr`).

### Interop
- `src/bridge.cyr`'s existing `wrap_tool_result` already detects a ready-made `{"content":[...]}` envelope and passes it through untouched — verified by a new test (`wrap_tool_result passes through a content envelope` in `bote_content.tcyr`). Handlers can opt into typed blocks without any transport-layer changes.

### Tests
- **New test file** — `tests/bote_content.tcyr` (15 assertions). Split out of `tests/bote.tcyr` for the cyrius 4.5.1 parser identifier-buffer cap — same pattern as `bote_libro_tools.tcyr`. Both will collapse back into the main test file when cyrius 4.6.1 lifts the cap.
- Coverage: every constructor's exact JSON output (including optional-field omission), JSON escaping of quotes, null-text → empty-string fallback, mixed-type arrays, empty-array case, `isError` flag, and the pass-through interop with `wrap_tool_result`.
- **Total assertions: 431** (was 416). Breakdown: `tests/bote.tcyr` 394, `tests/bote_libro_tools.tcyr` 22, `tests/bote_content.tcyr` 15.

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius test tests/bote_libro_tools.tcyr` → **22 passed, 0 failed**
- `cyrius test tests/bote_content.tcyr` → **15 passed, 0 failed**
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.6.0 baseline (dispatch_* 1–3µs, jsonx_* 612–963ns, codec_* 785ns–6µs, validate_* 1–3µs).
- `cyrlint src/content.cyr tests/bote_content.tcyr` → **0 warnings**.
- `./bote` — `initialize` → `{"serverInfo":{"name":"bote","version":"1.7.0"}}`.

### Deferred to v1.8+
- **Annotations** (`audience`, `priority`) on content blocks — MCP spec optional; skipped to keep 1.7.0 focused.
- **Binary resource contents** (`blob`) — the current `content_resource` handles text; the blob variant needs a base64 decoder surface (or passes through pre-encoded input). Punt until a consumer needs it.

### Known (unchanged)
- cyrius 4.5.1 identifier-buffer cap — forced the third test file. Tracked for 4.6.1 per `docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`.
- v1.2.1 libro-growth heisenbug — unrelated to this release.

## [1.6.0] — 2026-04-14 — libro_tools (5 built-in MCP audit tools)

Lands the `libro_tools` module from the 1.3 roadmap: five MCP tools that
expose a libro audit chain through the normal `tools/call` JSON-RPC
surface. Any MCP client can now search, verify, export, prove, and
retain-manage a bote-hosted chain without learning libro's native API.

### Added
- **`src/libro_tools.cyr`** (~310 LOC) — five handlers + registration:
  - `libro_query` — filter by `source` / `agent_id` / `severity` / `min_severity` / `action` / `after` / `before`; paginate with `offset` + `limit`. Returns `{"ok":true,"total":N,"entries":[...]}`.
  - `libro_verify` — hash-link integrity check. Returns `{"ok":true}` or `{"ok":false,"code":N,"index":i,"message":"..."}`.
  - `libro_export` — every entry as a JSON array. Returns `{"ok":true,"count":N,"entries":[...]}`.
  - `libro_proof` — Merkle inclusion proof for the entry at `index`. Returns `{"ok":true,"index":i,"leaf_count":N,"root":"<hex>"}`. Path hashes are not yet emitted — a follow-up will pin the wire format and include them.
  - `libro_retention` — apply `keep_count` / `keep_duration` / `keep_after` / `pci_dss` / `hipaa` / `sox` policies. Returns `{"ok":true,"archived":N,"retained":M}`.
- `libro_tools_init(chain)` + `libro_tools_register(dispatcher)` wire-up. Every cstr→Str crossing at the libro boundary goes through `str_from()` (the v1.2.1 cstr/Str fix pattern).
- **`src/main.cyr`** — the built-in dispatcher now creates an empty chain at startup and registers the five tools by default, so MCP clients discover them via `tools/list` without any flags.

### Tests
- **New test file** — `tests/bote_libro_tools.tcyr` (22 assertions). Lives separately from `tests/bote.tcyr` because pulling libro_tools into the main test file trips the cyrius 4.5.1 parser identifier-buffer cap. Split-test is tracked to collapse back in 4.6.1 when the cap is lifted.
- Coverage: registration of all five tools in the dispatcher map, handler-fn-pointer addressability, empty-chain shape of each tool's response (`{`-prefixed JSON), required-arg validation (`libro_proof` without `index`, `libro_retention` without `policy`), policy name whitelist (unknown policy → `ok:false`), each preset policy (`pci_dss`, `keep_count`) returns successfully.
- **Total assertions: 416** (was 394). Breakdown: `tests/bote.tcyr` 394, `tests/bote_libro_tools.tcyr` 22.

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius test tests/bote_libro_tools.tcyr` → **22 passed, 0 failed**
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.5.1 baseline (dispatch_* 1–3µs, jsonx_* 588–925ns, codec_* 773ns–6µs, validate_* 970ns–2µs).
- `cyrlint src/libro_tools.cyr tests/bote_libro_tools.tcyr` → **0 warnings**.
- `./bote` (stdio transport) — `initialize` → `{"serverInfo":{"name":"bote","version":"1.6.0"}}`, `tools/list` returns `bote_echo` + 5 `libro_*` entries, all 5 `tools/call` paths return clean JSON on an empty chain (verified by hand).

### Known (unchanged)
- **v1.2.1 libro-growth heisenbug**: creating an empty chain at startup (`chain_new()`) is safe; growing via `chain_append` is where the heap-sensitivity shows up. `libro_tools` itself is correct — only reads chain state, doesn't append. Writes still go through `src/audit_libro.cyr`, which is where the heisenbug lives.
- **cyrius 4.5.1 identifier-buffer cap** (`docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`) — forced the second test file. Tracked for 4.6.1.

## [1.5.1] — 2026-04-14 — P(-1) scaffold hardening

First hardening pass since 1.5.0. No new features; audit-driven fixes to
defensive guards, a line-length lint cleanup across `src/`, and two new
test assertions. All 394 tests / 10 benches / 4 fuzz harnesses green.

### Security
- **HTTP body-length clamp** — `src/transport_http.cyr`, `src/transport_streamable.cyr`, and `src/bridge.cyr` each copy the request body with `memcpy(body, buf + bo, clen)` after reading `Content-Length`. If a lying `Content-Length` header declared more bytes than actually arrived on the wire, `memcpy` would read past the request buffer into adjacent memory. All three paths now clamp `clen = min(clen, n - bo)` before the copy. Tested by manual audit; integration coverage by the existing transport tests still passes.

### Fixed
- **`resumption_buffer_events_after` null/empty guard** — Accepting `last_event_id == 0` previously would have segfaulted on the first `streq`; accepting `""` would have silently scanned the whole buffer for no matches. The caller in `_strm_handle_get` already guards null, but the helper is now defensive in its own right (returns empty vec in both cases). **Two new test assertions** cover these paths (394 total, was 392).

### Changed
- **Line-length cleanup** (`cyrlint`-clean): `src/bridge.cyr:172` (CORS header), `src/dispatch.cyr:57` (tool-name validation error message), `src/stream.cyr:93` (progress notification JSON). No behavior change — just wrapped the offending literals across two `str_builder_add_cstr` calls so lines stay under 120 chars. All `src/*.cyr` files now report `0 warnings` from `cyrlint`.

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **394 passed, 0 failed**
- `cyrius bench tests/bote.bcyr` → all 10 hot paths within noise of the 1.5.0 baseline (dispatch_* 1–3µs, jsonx_* 580–864ns, codec_* 763ns–6µs, validate_* 976ns–2µs)
- `cyrius fuzz fuzz/*.fcyr` → **4 passed, 0 failed**
- `cyrlint src/*.cyr` → all **0 warnings**

### Audit findings deferred
Captured during this pass but not actioned in 1.5.1:
- **Bump-allocator leak on long-lived WS connections** — every inbound frame allocs a fresh payload buffer, and `codec_process_message` returns fresh alloc'd JSON. Short-lived HTTP requests don't notice; WebSocket connections that stay open for hours will accumulate. Proper fix needs either an arena-per-message lifetime or stdlib `fl_free` support. Tracked for v1.6.
- **Global state in `transport_streamable.cyr` (`_strm_event_ids`, `_strm_resumption`) and `transport_stdio.cyr` (`_stdio_buf`, `_stdio_buf_len`)** — safe today because all transports are single-connection-at-a-time per the v1.0 design, but will need mutex-guarding when streaming dispatch (v1.5+ per roadmap) lets a server handle concurrent sessions.

### Carried forward
- v1.2.1 libro live-integration heisenbug: unchanged.
- cyrius 4.5.1 identifier-buffer cap: unchanged (`docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`).

---

## [1.5.0] — 2026-04-14 — WebSocket transport (RFC 6455)

Adds a sixth MCP transport: **WebSocket**. Each TEXT frame is one JSON-RPC
2.0 message. Built on `lib/ws_server.cyr` which landed in **cyrius 4.5.1**
(see `docs/proposals/cyrius-stdlib-ws-server.md` for the design rationale)
— and on the existing `lib/http_server.cyr` for the HTTP/1.1 Upgrade
handshake. **~110 LOC** of MCP-specific wire-up on top of the stdlib,
versus ~400 LOC if hand-rolled.

### Added
- **`src/transport_ws.cyr`** (~110 LOC):
  - **`WsConfig`** (48 bytes) — path, addr, port, allowed_origins, require_protocol, dispatcher
  - **`_bote_ws_handler`** — invoked per connection by `http_server_run`. Enforces Origin + `MCP-Protocol-Version` middleware (same shape as `transport_http`), calls `ws_server_handshake` to upgrade in place, then loops reading TEXT frames and feeding each to `codec_process_message` (control frames — ping/pong/close — handled by stdlib transparently).
  - **`transport_ws_run(dispatcher, config)`** — defers to stdlib `http_server_run`.
- **CLI** — `./build/bote ws [port]` (default `8393`).
- **Proposal artifacts** under `docs/proposals/` (same workflow as the http_server proposal that became cyrius 4.5.0):
  - `cyrius-stdlib-ws-server.md` — design doc + RFC 6455 spec coverage table
  - `lib_ws_server.cyr` — reference implementation with inlined SHA-1
  - `lib_ws_server_example.cyr` — runnable echo server

### Changed
- **cyrius pin bumped to 4.5.1** (required for `lib/ws_server.cyr`).
- `src/main.cyr` dispatches on `ws` argv (default port 8393).

### Spec compliance (RFC 6455, delegated to stdlib)
- ✅ HTTP/1.1 Upgrade handshake, `Sec-WebSocket-Accept = base64(sha1(key + magic))`
- ✅ `Sec-WebSocket-Version: 13` enforced
- ✅ Server reads MASKED client frames, writes UNMASKED server frames
- ✅ Small / medium (16-bit) / large (64-bit) payload length encodings
- ✅ Text + Binary data frames
- ✅ Ping / Pong control frames (handled transparently by `ws_server_recv`)
- ✅ Close handshake with status code + optional reason
- 🟡 Per-message deflate (RFC 7692) — deferred to stdlib
- 🟡 Subprotocol negotiation (`Sec-WebSocket-Protocol`) — header read but not enforced
  <br>⚠ **Correction (3.3.6): "header read" was never true.** `Sec-WebSocket-Protocol` has zero
  occurrences in `src/` or `lib/ws_server.cyr`; the handshake reads only Upgrade / Connection /
  Version / Key. See the roadmap's blocked-items table.

### Tests
- 10 new unit assertions (**392 total**, was 382):
  - `WsConfig` defaults + setters (path, addr, port, origins, require_protocol, dispatcher)
  - `_bote_ws_handler` fn-pointer addressability
  - Dispatcher wire-up on `transport_ws_run`
- Live handshake + frame round-trip stays with the stdlib `ws_server` conformance tests (avoids duplicating the protocol suite, and dodges the 4.5.1 parser input-buffer cap we hit when pulling the full `ws_server.cyr` into this file).

### Verified (cyrius 4.5.1)
- `cyrius test tests/bote.tcyr` → **392 passed, 0 failed**
- `cyrius build` → `./bote` (with `ws` subcommand binds `127.0.0.1:8393` and returns 101 on `GET /mcp` with a valid `Sec-WebSocket-Key` — verified via local `wscat` / `curl` probe)
- `cyrius bench` → 10 hot paths unchanged
- `./build/bote` initialize handshake reports `"version":"1.5.0"`

### Carried forward
- v1.2.1 libro live-integration heisenbug: still present, still tracked.

### Known cyrius 4.5.1 artifact
- The parser's input-buffer cap is reached when `tests/bote.tcyr` also includes `lib/ws_server.cyr` directly. Worked around by keeping ws_server out of the test file (the handler's frame I/O is covered by the stdlib conformance tests anyway). Tracked upstream; a follow-up cyrius patch will lift the cap.

---

## [1.4.0] — 2026-04-14 — Streamable HTTP transport (MCP 2025-11-25)

Closes the **streamable HTTP** spec item from MCP 2025-11-25. Single endpoint
serves both `POST` (JSON-RPC request → response) and `GET` (open SSE stream
for server-initiated messages). Built on the stdlib `lib/http_server.cyr`
chunked primitives that shipped in cyrius 4.5.0.

### Added
- **`src/transport_streamable.cyr`** (~290 LOC). Modules:
  - **`EventIdGenerator`** — monotonic counter, emits `"evt-N"` strings
  - **`StreamEvent`** — `{id, event="message", data}` with SSE wire-format renderer (`stream_event_to_wire` → `id: ...\nevent: ...\ndata: ...\n\n`)
  - **`ResumptionBuffer`** — bounded ring of recent events; `events_after(last_id)` for `Last-Event-ID` replay
  - **`StreamableConfig`** (64 bytes) — path, addr, port, allowed_origins, require_protocol, session_store, retry_ms, dispatcher
  - **`transport_streamable_run(d, cfg)`** — defers to stdlib `http_server_run` with a single dispatch handler that routes POST → JSON-RPC, GET → SSE stream
- **CLI** — `./build/bote streamable [port]` (default `8392`).

### Spec compliance
- ✅ `POST <endpoint>` JSON-RPC dispatch (same shape as plain HTTP)
- ✅ `GET <endpoint>` SSE stream open with priming event
- ✅ `MCP-Protocol-Version` header **required** on every request (400 if absent — stricter than plain HTTP transport which makes it optional by default)
- ✅ `MCP-Session-Id` header validated when SessionStore is configured
- ✅ `Origin` allow-list (DNS rebinding protection)
- ✅ `Last-Event-ID` request header → replay buffered events on GET
- ✅ Server emits `id:`-tagged SSE events for resumption tracking
- ✅ `retry: <ms>\n\n` hint sent before stream close (default 5000ms, configurable)
- 🟡 Server-initiated event push on the GET stream: deferred — waits on streaming dispatch (v1.5+) to populate the resumption buffer with real events. The transport correctly opens the SSE stream and replays anything in the buffer; the buffer is just empty until something publishes to it.

### Tests
- 23 new unit assertions (382 total, was 359):
  - `EventIdGenerator` produces monotonic `evt-0`/`evt-1`/`evt-2`
  - `StreamEvent` accessors + SSE wire format (with and without data)
  - `ResumptionBuffer` push, eviction (oldest first when over capacity), `events_after` lookup (present and absent IDs)
  - `StreamableConfig` defaults + setters
  - `http_path_only` correctly strips query string for path matching

### Performance
Bench numbers unchanged. `cyrius bench` still shows 10 hot paths sub-10µs.

### Verified end-to-end
- `POST /mcp` with `MCP-Protocol-Version: 2025-11-25` → returns serverInfo
- `POST /mcp` without protocol header → `400 Bad Request`
- `POST /mcp tools/call` → standard JSON-RPC response
- `GET /mcp` with `Accept: text/event-stream` → opens SSE stream, sends primer (`id: evt-0\nevent: message\ndata: \n\n`), sends retry hint (`retry: 5000\n\n`), closes
- `Last-Event-ID: evt-7` (when buffer has events past evt-7) → replays them in order

### Carried forward
- v1.2.1 libro live-integration heisenbug: still present, still tracked.

### Verified (cyrius 4.5.0)
- `cyrius test` → **382 passed, 0 failed**
- `cyrius fuzz` → 4 passed, 0 failed
- `cyrius bench` → 10 hot paths unchanged
- `./build/bote` initialize handshake reports `"version":"1.4.0"`

---

## [1.3.0] — 2026-04-14 — Adopt stdlib `lib/http_server.cyr` (cyrius 4.5.0)

Cyrius **4.5.0** shipped `lib/http_server.cyr` — verbatim from the proposal
in `docs/proposals/cyrius-stdlib-http-server.md`. Bote drops 236 lines of
hand-rolled HTTP plumbing in favour of the shared stdlib.

### Changed
- **`cyrius.toml`** — added `"http_server"` to `[deps]` stdlib list. Cyrius pin → `4.5.0`.
- **`src/transport_http.cyr`** — was 404 LOC, now **150 LOC** (-63%). Dropped: `_http_find`, `_http_to_lower`, `_http_iceq`, `_http_next_nl`, `http_find_header`, `http_get_method`, `http_get_path`, `http_body_offset`, `http_content_length`, `_http_send_status`, `_http_send_json_200`, `_http_send_204`, plus the bind/listen/accept ceremony. All come from stdlib now. Kept: `HttpConfig` struct + accessors, `_http_check_origin / _protocol / _session` middleware, `_http_handle` request handler, `transport_http_run` (now a 5-line wrapper around `http_server_run`).
- **`src/bridge.cyr`** — was 280 LOC, now **170 LOC** (-39%). Dropped: `_bridge_handle_connection` (rewritten to handler-style), bind/listen/accept loop, response builders. Kept: `BridgeConfig` (added `dispatcher` slot), `wrap_tool_result` / `wrap_error_result` (MCP envelope contract, bote-specific), `_bridge_cors_*` headers, `bridge_process_message` (bridge-specific routing).
- **`HttpConfig`** gained a `+48 dispatcher` slot (was 48 → now 56 bytes) so it can carry the dispatcher into the stdlib `http_server_run` ctx pointer.
- **`BridgeConfig`** gained a `+24 dispatcher` slot (was 24 → now 32 bytes) for the same reason. New setter: `bridge_config_with_dispatcher(c, d)`.
- **Status codes** now use `HTTP_OK`, `HTTP_NOT_FOUND`, etc. constants from stdlib (was hardcoded integers).
- **Path matching** uses `http_path_only(path)` from stdlib so `/mcp?something` matches `/mcp` correctly. Same for bridge `/health` and `/`.

### Performance / size
- **bote ELF binary**: was 130 KB (1.2.1) → **127 KB** (1.3.0). The stdlib HTTP code is shared with any future cyrius project.
- Function count freed up from bote's compilation unit: ~28 fns (the entire HTTP plumbing layer) now lives in stdlib and counts once across all consumers.
- Hot-path benchmarks unchanged.

### Spec impact
- **Content-Length-aware request reading** now correct (stdlib `http_recv_request` reads until body is fully received). Previous bote behaviour did a single `sock_recv` and silently truncated requests larger than one TCP packet — fixed for free.
- **Unblocks v1.4.0 streamable HTTP** — stdlib provides `http_send_chunked_start` / `http_send_chunk` / `http_send_chunked_end` for SSE.

### Carried forward
- v1.2.1 known issue (live libro chain integration heisenbug in tests/bote.tcyr) **persists** despite freed function-count budget. Adapter remains correct in isolated probes; in-test live integration test is still shape-only. Suggests the heisenbug is heap-layout / global-init related, not function-count related — needs deeper cyrius investigation.

### Verified (cyrius 4.5.0)
- `cyrius test` → **359 passed, 0 failed**
- `cyrius fuzz` → 4 passed, 0 failed
- `cyrius bench` → 10 hot paths unchanged
- `./build/bote` initialize handshake reports `"version":"1.3.0"`
- HTTP and bridge transports both verified end-to-end with `curl`

### `docs/proposals/`
The proposal docs (`cyrius-stdlib-http-server.md`, `lib_http_server.cyr`,
`lib_http_server_example.cyr`) remain in the repo as the spec the lang-agent
implemented from. Useful reference for any future stdlib proposals coming
out of bote work.

---

## [1.2.1] — 2026-04-13 — Adapter init-dance docs + v1.2.0 patch hardening

### Fixed
- **`src/audit_libro.cyr::libro_audit_log`** wraps every cstr boundary value through `str_from` before passing to `chain_append` / `chain_append_with_agent`. libro expects `Str` (fat strings) for source / action / details / agent_id, not raw cstrs — the previous version passed cstrs directly which produced garbage `str_len` reads inside libro's hash function. Verified by isolated probe: `chain_append(c, SEV_INFO, str_from("bote"), str_from("tool.completed"), str_from("{}"))` correctly grows the chain to length 1.

### Documented
- **Init dance** for any binary that uses LibroAudit:
  ```
  alloc_init();   # bump allocator
  fl_init();      # freelist — libro entries
  ed25519_init(); # sigil signing constants
  ```
  Codified in the `audit_libro.cyr` header comment.

### Cyrius pin
- Bumped to **4.4.6** (`cyrius.toml`).

### Known issue (tracked for v1.3.0)
- Linking libro + majra + the full bote test corpus into one binary triggers a heap heisenbug: `libro_audit_log` enters an infinite loop (apparently re-entering `main`). The adapter itself is correct — verified by an isolated probe that calls `chain_append` directly with `str_from`-wrapped strings. Suspect: cumulative globals from the cross-product exceed an internal cyrius compilation-unit boundary and corrupt the bump-allocator's prologue. Workarounds explored: dropping `lib/patra.cyr` (saved 244 KB but didn't help), splitting includes (no effect), running the demo as a standalone binary (still loops). Filed as a tracking item; resolution likely needs the multi-file linker on cyrius's v4.5 roadmap.

### Verified (cyrius 4.4.6)
- `cyrius test` → 359 passed, 0 failed
- `cyrius fuzz` → 4 passed, 0 failed
- `cyrius bench` → 10 hot paths unchanged
- `./build/bote` initialize handshake reports `"version":"1.2.1"`

---

## [1.2.0] — 2026-04-13 — LibroAudit + MajraEvents adapters

Wires bote's AuditSink and EventSink (introduced in 1.1.0) to the **libro**
audit chain and **majra** pub/sub. First release with `[deps.<crate>]` git+tag
pinned dependencies.

### Added
- **`[deps.libro]`** — pinned to git tag `1.0.3` (`https://github.com/MacCracken/libro`, falls back to local `../libro`). 9 modules pulled in: `error / hasher / entry / verify / query / retention / chain / export / merkle`.
- **`[deps.majra]`** — pinned to git tag `2.2.0` (`https://github.com/MacCracken/majra`, falls back to local `../majra`). 6 modules pulled in: `error / counter / envelope / namespace / queue / pubsub`. Trimmed to only the modules `pubsub` actually exercises (skipped `metrics / ratelimit / heartbeat / fleet / dag / etc`).
- **`src/audit_libro.cyr`** — `LibroAudit` adapter (24 bytes: chain ptr, source cstr, agent_id cstr). `libro_audit_new(chain)`, `libro_audit_with_source(la, src)`, `libro_audit_with_agent_id(la, id)`, `libro_audit_log(ctx, event)`. Maps bote's ToolCallEvent to libro's `chain_append_with_agent` (or `chain_append` when no agent): `SEV_INFO` + `"tool.completed"` on success, `SEV_ERROR` + `"tool.failed"` on failure. **caller_id wins over the configured agent_id.**
- **`src/events_majra.cyr`** — thin `majra_events_publish(ctx, topic, payload)` that calls `pubsub_publish(ps, topic, payload)`. Wire-up:
  ```
  var ps = pubsub_new();
  var sink = event_sink_new(&majra_events_publish, ps);
  dispatcher_set_events(d, sink);
  ```
- 8 new unit assertions (359 total) covering adapter struct shapes, accessors, AuditSink fp wiring, EventSink fp wiring (incl. `ctx=0` no-op safety).

### Performance
Bench numbers unchanged from 1.1.0 — adapters are pass-through over already-measured sink-publish overhead.

### Deferred
Live `chain_append_with_agent` integration tests (and full pubsub deliver paths) require running libro/majra's full init dance (`alloc_init`, `fl_init`, `ed25519_init`, `patra_init`, etc). Those are exercised by libro's and majra's own test suites; bote's tests currently verify the **adapter shape** (struct + fp wiring). Live integration tests will land in **v1.2.1** once the init-call documentation is finalized.

`src/libro_tools.cyr` (5 built-in MCP audit tools) deferred to **v1.3.0**. Reason: the cyrius compiler hits an internal token-table boundary when the full bote + libro + majra + tool-handler-fns set is included in one compilation unit. v1.3.0 will either (a) split bote into multiple compilation units (multi-file linker, on cyrius's v4.5 roadmap) or (b) ship libro_tools as a separate `[deps.bote-libro-tools]` package.

### Verified (cyrius 4.4.4)
- 359 tests passed, 0 failed
- 4 fuzz harnesses passed
- 10 benchmarks unchanged
- `./build/bote` initialize handshake reports `"version":"1.2.0"`

---

## [1.1.0] — 2026-04-13 — AuditSink + EventSink + dispatcher wire-up

First minor bump on the cyrius lineage. Adds the audit and event-publishing
abstractions and wires them into the dispatcher. Adapters for libro / majra /
discovery-via-pubsub land in **v1.2.0**.

### Added
- **`src/events.cyr`** — `EventSink` (function-pointer + opaque ctx struct, the cyrius equivalent of the Rust trait), `event_sink_new` / `event_sink_publish` / `event_sink_noop`. Topic constants exported as functions: `TOPIC_TOOL_COMPLETED`, `TOPIC_TOOL_FAILED`, `TOPIC_TOOL_REGISTERED`, `TOPIC_TOOL_DEREGISTERED`, `TOPIC_TOOL_DEPRECATED`, `TOPIC_TOOL_ANNOUNCE`, `TOPIC_TOOL_DISCOVERED`, plus 3 sandbox topics for the v1.3 sandbox port.
- **`src/audit.cyr`** — `ToolCallEvent` (40 bytes: tool_name, duration_ms, success, error, caller_id), `tool_call_event_to_json` (matches Rust `serde_json` output, skips `error` / `caller_id` when 0), `AuditSink` struct + `audit_sink_log` / `audit_sink_noop`.
- **`Dispatcher` extended** to 40 bytes — new slots for `audit_sink` and `event_sink`. Setters: `dispatcher_set_audit(d, sink)` and `dispatcher_set_events(d, sink)`. Sinks default to 0 (no-op); pre-1.1 callers see no behavior change.
- **Dispatcher emits per-call audit + event hooks**:
  - `tools/call` success → `audit_sink_log` + publish to `bote/tool/completed`
  - `tools/call` failure (handler not in map) → `audit_sink_log` + publish to `bote/tool/failed`
  - `tools/call` on a deprecated tool → publish to `bote/tool/deprecated` *before* the call
  - `dispatcher_register_tool` → publish to `bote/tool/registered`
  - `dispatcher_deregister_tool` → publish to `bote/tool/deregistered`
  - All include a `{"tool_name":"..."}` payload (deprecated also includes `message`).
- **`caller_id`** now extracted from `tools/call` params (`jsonx_get_str(params, "caller_id")`) and threaded through to the audit event.
- **`src/discovery.cyr` migrated to `EventSink`** — `discovery_new(node_id, event_sink)` replaces the bare `publish_fp`. Uses `discovery_event_sink(d)` accessor; same callers, cleaner integration with the rest of the event surface.
- **50 new unit assertions** (351 total, was 301): topic constants, sink no-op safety, sink invocation, ToolCallEvent JSON round-trips (success / failure / minimal), full dispatcher wire-up (success+failure+initialize+list+register+dereg+deprecated paths), discovery via EventSink, "validate-stage error doesn't audit" parity check vs. Rust.

### Performance
Audit + event hooks add ~2µs to `dispatch_tools_call` (1µs → 3µs) and ~2µs to `codec_process_message` (4µs → 6µs) when sinks are wired. With `audit_sink_noop()` / `event_sink_noop()` (or unset, the default), the overhead is a single null-pointer check per emission site. Other benchmarks unchanged.

### Changed
- `discovery_new` signature: was `(node_id, publish_fp)` taking a bare `fn(topic, json)` pointer; now `(node_id, event_sink)` taking an EventSink. **Source-breaking** — but the only known caller was `tests/bote.tcyr`, and the new shape is what real callers (MajraEvents in v1.2) need anyway. v1.0 callers building a discovery service should switch to `event_sink_new(&publish_fn, ctx)`.

### Verification (cyrius 4.4.4)
- `cyrius test` → **351 passed, 0 failed**
- `cyrius fuzz` → 4 passed, 0 failed
- `cyrius bench` → 10 hot paths, sinks-noop overhead is 1 conditional branch per emission
- `./build/bote` initialize handshake reports `"version":"1.1.0"`

### Deferred to v1.2.0
- `src/audit_libro.cyr` — LibroAudit adapter (calls `chain_append_with_agent` on libro's hash chain). Needs `[deps.libro] path = "../libro"` in `cyrius.toml`.
- `src/events_majra.cyr` — MajraEvents adapter (calls `pubsub_publish`). Needs `[deps.majra] path = "../majra"` in `cyrius.toml`.
- `src/libro_tools.cyr` — 5 built-in MCP tools (`libro_query`, `libro_verify`, `libro_export`, `libro_proof`, `libro_retention`). Depends on audit + libro.

---

## [1.0.1] — 2026-04-13 — Retire rust-old/, trim spec-compliance, bench comparison

### Removed
- **`rust-old/`** — the Rust source archive that came in via `cyrius port` is gone. Its purpose (porting reference) is fulfilled. The last Rust state remains accessible at git tag `0.92.0`.
- `.gitignore` rules for `rust-old/target/` and `rust-old/**/target/` (no longer needed).

### Added
- **`docs/benchmarks-rust-v-cyrius.md`** — side-by-side performance comparison. Source / binary / dep counts, per-op timings (Rust v0.92.0 vs Cyrius v1.0.1), the structural reasons Cyrius is 3-10× slower per op, and where each side wins. Preserves the 5-entry Rust bench history before the archive went away.
- Cyrius pin bumped to **4.4.4** (`cyrius.toml`). All correctness pain points from the original port are now fixed in cyrius itself; only the documented `var buf[N]` size limit remains as a design choice.

### Changed
- **`docs/spec-compliance.md`** — rewritten. Was carrying the Rust-era matrix that listed modules like `host::McpContentBlock` and `auth::TokenClaims` as "Complete" when they aren't ported yet. New version lists **only what cyrius v1.0.x covers today** (with explicit `✅` checkmarks per module + accessor function name) plus a single "Gaps that need coverage" section that points at the roadmap rather than duplicating it.
- README, `docs/architecture/overview.md`, `docs/development/roadmap.md` — `rust-old/` references rewritten to point at git tag `0.92.0`.
- Server `initialize` response — `serverInfo.version` now reports `"1.0.1"`.

### Verified
- 301 tests pass on cyrius 4.4.4
- 4 fuzz harnesses pass (no regressions)
- All 4 transports (stdio / HTTP / Unix / bridge) confirmed end-to-end

---

## [1.0.0] — 2026-04-13 — Stable cyrius MCP core

Bote's cyrius implementation is **stable**. The MCP protocol surface, registry,
dispatcher, schema validation, sessions, discovery, and four transports
(stdio, HTTP, Unix socket, TS bridge) are feature-complete and verified:

- **298 unit assertions** all passing
- **10 hot-path benchmarks**, all sub-10µs on x86_64
- **4 fuzz harnesses**, ~330 calls across malformed and edge-case inputs, no crashes
- **End-to-end smoke tests** for stdio (pipe), HTTP (curl), Unix socket (Python AF_UNIX), bridge (curl + CORS)

The data shapes for `JsonRpcRequest`, `JsonRpcResponse`, `ToolDef`, `ToolSchema`,
`ToolAnnotations`, `CompiledSchema`, `BoteError`, `McpSession`, and the four
`HttpConfig` / `BridgeConfig` flavours are **frozen** — additive changes only
within the 1.x series.

### Critical bug fix included in 1.0.0

- **`src/jsonx.cyr::jsonx_get_str`**: on truncated input (opening `"` with no closing — e.g. `{"k":"`), `_jx_skip_string` returned `end == len`, making `inner_len = end - pos - 2 == -1`. The subsequent `memcpy(out, src, -1)` was interpreted as a huge unsigned size → segfault. Surfaced by the `jsonx_extract.fcyr` fuzz harness on cyrius 4.4.x. **Fix**: clamp `inner_len` to `>= 0` (returns empty string for truncated input). Regression covered in `tests/bote.tcyr`.

### Workaround cleanup (cyrius 4.4.3 unblocked it)

Now that cyrius 4.4.3 ships `\r` escape correctness, `&&`/`||` short-circuit,
and per-block `var` shadowing, the defensive workarounds in bote can collapse:

- **`src/transport_http.cyr` + `src/bridge.cyr`**: removed `_crlf` / `_crlfcrlf` global pointers and `_http_init_crlf()` setup function. All HTTP / CORS response builders now use embedded `"\r\n"` and `"\r\n\r\n"` literals directly. ~50 lines removed.
- **`src/jsonx.cyr`**: collapsed three nested `if (i >= len) { ... } if (load8 != X) { ... }` patterns into single `if (i >= len || load8 != X)` checks. Same for `if (i < len) { if (load8 == 44) { ... } }` → `&&`.
- **`src/jsonx.cyr`**: `if (key_len_actual == klen) { if (memeq(...)) { ... } }` → `key_len_actual == klen && memeq(...) == 1` (was the explicit fix for the non-short-circuit `memeq`-on-truncated-input bug; now safe to write naturally).
- **`src/registry.cyr`**: `if (v != 0) { if (streq(v, version) == 1) { return t; } }` → `&&`.
- **`src/dispatch.cyr`**: `_extract_tool_name` ditched the `var bad = 0;` flag; now `if (name == 0 || strlen(name) == 0)`. `if (ver != 0) { if (registry_get_versioned(...) == 0) { ... } }` → `&&`. Schema-emit `if (props != 0) { if (vec_len(props) > 0) { ... } }` → `&&`.

Net diff: **60 lines removed across 6 files**. No behavior change, all tests / fuzz / e2e smokes still green.

### What's in 1.0.0

| Area | Status |
|---|---|
| JSON-RPC 2.0 (request, response, notification, batch) | ✅ |
| MCP `initialize` / `tools/list` / `tools/call` | ✅ |
| Tool registry with versioning + deprecation + annotations | ✅ |
| Compiled schema (type/enum/bounds/nested object/array items, multi-violation) | ✅ |
| `JsonRpcError` codes — full spec mapping | ✅ |
| Session management (create/validate/prune, MCP-Session-Id header) | ✅ |
| Origin allow-list + protocol-version header validation | ✅ |
| stdio transport | ✅ |
| HTTP/1.1 transport with middleware | ✅ |
| Unix domain socket transport | ✅ |
| TypeScript bridge (CORS + MCP envelope wrap) | ✅ |
| Discovery (data layer + pluggable publish_fp) | ✅ |
| Streaming primitives (ProgressUpdate, CancellationToken) | ✅ data layer |

### Post-1.0 extensions (1.x minor bumps)

These are additive — none change existing API shapes.

| Module | Status |
|---|---|
| `src/audit.cyr` + `LibroAudit` adapter | **Ready to port** — libro v1.0.3 available via `[deps.libro] path = "../libro"` |
| `src/events.cyr` + `MajraEvents` adapter | **Ready to port** — majra v2.2.0 available via `[deps.majra] path = "../majra"` |
| `src/discovery.cyr` wire-up to majra pubsub | **Ready to port** — depends on events |
| `src/libro_tools.cyr` (5 built-in audit tools) | **Ready to port** — depends on audit + libro |
| `src/sandbox.cyr` + kavach integration | Wait — kavach v2-arch hardening in flight |
| `src/host.cyr` (content blocks, host registry) | Ready (no AGNOS dep) |
| `src/auth.cyr` (OAuth 2.1 / PKCE / bearer) | Ready (no AGNOS dep) |
| `src/transport_streamable.cyr` (POST + SSE single endpoint) | Ready (rolls SSE on top of `transport_http`) |
| `src/transport_ws.cyr` (server-side WebSocket) | Cyrius `lib/ws.cyr` is client-only; needs server handshake + frame unmasking written |
| Threaded streaming dispatch | Needs `lib/thread.cyr` MPSC wired into `dispatcher_dispatch_streaming` |

### Versioning policy from here

Pre-1.0 used `0.D.M` (day.month). From 1.0.0 forward, **standard SemVer**:
- **Major** — break a frozen data shape or remove a public function.
- **Minor** — add a module / function / config option.
- **Patch** — fix bugs, refactor internals, improve diagnostics.

### Cyrius toolchain pin

Built and tested against cyrius **4.4.0** (`cyriusly use 4.4.0`).

---

## [0.1.1] — 2026-04-13 — Bridge + cyrius 4.4.0 + review punch list

### Added
- **`src/bridge.cyr`** — TypeScript-bridge HTTP transport: CORS preflight (`OPTIONS /`), `GET /health`, `POST /` JSON-RPC dispatch with MCP-envelope wrapping for `tools/call` results. `wrap_tool_result` (passthrough if already shaped, else wraps text), `wrap_error_result` (adds `isError: true`).
- **CLI**: `./build/bote bridge [port]` (default 8391).
- 29 new unit assertions: bridge wrappers, CORS origin selection, `bridge_process_message` round-trips, schema bounds at exact `min` / `max`, codec pure-notification batch.

### Fixed (review punch list)
- **`src/jsonx.cyr`**: `key_len_actual == klen && memeq(...)` was unsafe because cyrius `&&` doesn't short-circuit — `memeq` was called on truncated input. Now nested as separate `if`s.
- **`src/transport_http.cyr`**: when `Content-Length` was absent and `body_off > n` (malformed request), `clen = n - body_off` could be negative → `memcpy` UB. Now guarded.
- **`src/schema.cyr`**: `_sch_parse_int` replaced `i = i + 999999` marker-hack with proper `break` (per-block scoping now works in cyrius 4.4.0).
- **`src/transport_http.cyr`**: `http_find_header` similarly cleaned — replaced `vs = vs - 0; line_start = headers_end; vs = vs - 0;` marker hack with structured loops + `break`.

### Verified against cyrius 4.4.0 (`cyriusly install 4.4.0 && cyriusly use 4.4.0`)
- ✅ `\r` escape now emits CR (13) — fixed upstream
- ✅ Per-block `var` shadowing now works — fixed upstream
- ❌ `&&` / `||` short-circuit still missing — workarounds retained
- ➕ DCE now available via `CYRIUS_DCE=1` at build time

`docs/cyrius-feedback.md` updated with v4.4.0 verification status against each repro.

### Performance
Bench numbers unchanged from 0.1.0 — bridge adds a thin envelope-wrap layer with no measurable overhead on the hot dispatch path.

---

## [0.1.0] — 2026-04-13 — Cyrius port baseline

### Breaking
- **Language switch**: bote moved from Rust to Cyrius. The Rust source is preserved under `rust-old/` for reference and recovery. Version reset to `0.1.0` to mark the new lineage.
- **API change**: idiomatic Cyrius — module-prefixed function APIs (`registry_register`, `dispatcher_dispatch`, `codec_process_message`) over offset-addressed structs (`store64`/`load64`). No traits, generics, async, or borrow checking. Handler functions are i64 function pointers (`fn h(args_cstr) → result_cstr`).

### Added
- **`src/error.cyr`** — `BoteErrTag` enum (12 variants), `bote_err_rpc_code`, `bote_err_format`, schema-violation list support.
- **`src/protocol.cyr`** — `JsonRpcRequest` / `JsonRpcResponse` / `JsonRpcError` with raw-JSON-literal id/params/result/data slots.
- **`src/jsonx.cyr`** — Nested-aware JSON value extractor (`jsonx_get_raw`, `jsonx_get_str`, `jsonx_has`, `jsonx_is_object`). Handles nested objects, arrays, escaped strings; needed because `lib/json.cyr` is flat-only.
- **`src/registry.cyr`** — `ToolDef` (with `version`, `deprecated`, `annotations`, `compiled` slots), `ToolSchema`, `ToolAnnotations` (presets `read_only` / `destructive`), `ToolRegistry` (insertion-ordered, hashmap-indexed). Versioned tools, deprecation, validate-by-required-fields fallback.
- **`src/dispatch.cyr`** — `Dispatcher`, sync handler dispatch, `initialize` / `tools/list` / `tools/call` routing, MCP protocol-version negotiation, `validate_tool_name` (project_tool format, 256 char max), dynamic register/deregister.
- **`src/codec.cyr`** — `codec_parse_request`, `codec_serialize_response`, `codec_process_message` (single + batch + notification + error responses), JSON-message escaping reused from dispatch.
- **`src/schema.cyr`** — `CompiledSchema` with full type-checking (`string`, `number`, `integer`, `boolean`, `array`, `object`, `Any`), enum constraints, numeric bounds, recursive nested objects + array items, multi-violation reporting. `tool_def_with_compiled` slot wires it into `registry_validate_params`.
- **`src/stream.cyr`** — `CancellationToken`, `ProgressUpdate`, `ProgressSender`, `StreamContext`, `progress_notification` JSON builder. (Thread integration deferred.)
- **`src/session.cyr`** — `SessionStore` (hex-encoded 16-byte SIDs from `/dev/urandom`), `validate_protocol_version`, `validate_origin` (wildcard `*`, exact match, strict mode).
- **`src/transport_stdio.cyr`** — Line-oriented JSON-RPC over stdin/stdout, 128KB heap-allocated buffer, partial-line shifting.
- **`src/transport_http.cyr`** — HTTP/1.1 server (`POST /mcp` → JSON-RPC). Origin/MCP-Protocol-Version/MCP-Session-Id middleware. Auto-creates a session on `initialize` and emits the new `MCP-Session-Id` response header. Case-insensitive header lookup. 64KB request buffer.
- **`src/transport_unix.cyr`** — `AF_UNIX` line-oriented transport (own socket-creation code since `lib/net.cyr` is `AF_INET`-only). 128KB per-connection buffer.
- **CLI** — `./build/bote [stdio|http <port>|unix <path>]` selects transport.
- **Tests** — `tests/bote.tcyr` with **251 unit assertions** covering all modules.
- **Benchmarks** — `tests/bote.bcyr` with 10 hot-path benchmarks (all sub-10µs on x86_64).
- **Fuzz** — `fuzz/codec_parse.fcyr`, `fuzz/codec_process.fcyr`, `fuzz/jsonx_extract.fcyr`, `fuzz/schema_validate.fcyr` (~330 fuzzed calls; no crashes).
- **`docs/cyrius-feedback.md`** — language-level issues found during the port.
- `.gitignore` rules for `rust-old/target/` and `/build/`.

### Performance
- `dispatch_initialize` ~2µs avg
- `dispatch_tools_list` ~2µs avg
- `dispatch_tools_call` ~1µs avg
- `jsonx_get_str_flat` 600ns avg
- `jsonx_get_raw_nested` ~1µs avg
- `codec_parse_request` ~2µs avg
- `codec_serialize_response` ~1µs avg
- `codec_process_message` (full pipeline) ~5µs avg
- `validate_compiled_simple` ~1µs avg
- `validate_compiled_nested` ~3µs avg

### Deferred to future cyrius releases
- `bridge` — TypeScript bridge with CORS / MCP envelope wrapping.
- `audit` — libro hash-linked audit chain integration.
- `events` — majra pub/sub event publishing.
- `discovery` — cross-node tool announcements (depends on `events`).
- `sandbox` — kavach tool isolation.
- `host` — MCP hosting layer (content blocks, host registry).
- `libro_tools` — 5 built-in libro audit MCP tools.
- `auth` — OAuth 2.1 / PKCE / bearer-token middleware.
- `transport_ws` — server-side WebSocket (cyrius `lib/ws.cyr` is client-only).
- `transport_streamable` — streamable HTTP (POST + SSE single endpoint).
- Streaming dispatch (needs thread + channel integration).

### Known cyrius-language workarounds applied
- `\r` string escape emits byte `r` (114) instead of CR (13) — built CRLF via `store8`.
- `&&` / `||` operators do not short-circuit — guarded null derefs nested as `if (p != 0) { if (...) { ... } }`.
- No per-block local scoping — distinct names per `fn` body (`req_one`, `rcompiled`, `prog_notif`, etc.).
- Static `var buf[N] >~ 16KB` exhausts the output buffer — large buffers heap-allocated (`var ptr = 0;` global + `ptr = alloc(N);` at startup).

See [docs/cyrius-feedback.md](docs/cyrius-feedback.md) for full reproductions.

---

## Historical (Rust) — preserved under `rust-old/`

## [0.91.0] — 2026-04-02

### Added
- `libro_tools` module (feature: `audit`) — 5 built-in MCP tools for libro audit chain operations:
  - `libro_query` — query audit entries by source, severity, action, agent, min_severity, with limit
  - `libro_verify` — verify chain integrity and return structured `ChainReview` JSON with integrity status, entry count, time range, source/severity/agent distributions (was text-only)
  - `libro_export` — export chain as JSON Lines or CSV
  - `libro_proof` — generate Merkle inclusion proof for an entry by index, returns structured proof JSON with verification status
  - `libro_retention` — apply retention policies (PCI-DSS, HIPAA, SOX, keep_count) and report archived entries (destructive, not read-only)
- `libro_tools::register()` — convenience function to register all 5 libro tools on a dispatcher
- Read-only tools annotated with `ToolAnnotations::read_only()` (MCP 2025-11-25); `libro_retention` is destructive (no annotation)
- `LibroAudit::with_source()` — custom source tag for audit entries (default: `"bote"`)
- `LibroAudit::with_agent_id()` — server agent identity on all entries; `caller_id` from events takes precedence
- `LibroAudit` now uses `append_with_agent()` when caller_id or agent_id is present, populating libro's agent tracking
- 17 libro_tools tests + 8 audit tests (was 8 + 3)
- **HTTP transport middleware**: Origin validation (403), `MCP-Protocol-Version` enforcement (400), `MCP-Session-Id` session lifecycle (404), bearer token extraction with 401/403 responses (feature `auth`)
- **Streamable HTTP transport router**: axum router with POST (JSON-RPC) and GET (SSE stream) on configurable endpoint path, same middleware stack as HTTP, SSE event IDs via `EventIdGenerator`, `Last-Event-ID` resumption via `ResumptionBuffer` replay, `retry:` hint before close, priming event on connect
- `HttpConfig` builder: `with_allowed_origins()`, `with_session_timeout()`, `with_token_validator()` (feature `auth`)
- `StreamableConfig` builder: `with_session_timeout()`, `without_sessions()`
- `TokenValidator` trait (feature `auth`) — consumers implement to validate bearer tokens
- Shared `transport::middleware` module — `check_origin`, `check_protocol_version`, `check_protocol_version_required`, `check_session`, `check_bearer` reused by both transports
- Periodic session pruning via tokio interval in both `http::serve()` and `streamable::serve()`
- `streamable::streamable_router()` — build router without binding a port (for testing)
- 35 new transport middleware tests (origin, protocol version, session enforcement in both transports)
- `cargo vet` supply chain auditing: 156 crates fully audited via trusted imports (mozilla, google, bytecode-alliance, isrg, zcash, ariel-os, embark-studios) and 27 trusted publishers (dtolnay, seanmonstar, Manishearth, epage, fitzgen, kennykerr, Amanieu, BurntSushi, Thomasdezeeuw, cuviper, alexcrichton, carllerche, Darksonn, rust-lang-owner), 66 exempted, CI integration

### Changed
- Upgraded libro dependency from 0.25 to 0.91 (BLAKE3 hashing, serde on all types, key rotation support)
- `HttpConfig` expanded with `allowed_origins`, `session_timeout`, `token_validator` fields
- `StreamableConfig` expanded with `session_timeout` field
- Streamable transport `MCP-Protocol-Version` header is **required** (per MCP 2025-11-25), unlike plain HTTP where it is optional

## [0.90.0] — 2026-04-01

### Fixed
- **JSON-RPC 2.0 spec compliance**: Unknown methods now return `-32601` (Method not found) instead of `-32600` (Invalid Request)
- **Bridge spec compliance**: Error wrapping no longer sets both `result` and `error` on the response (JSON-RPC 2.0 violation)
- `scripts/bench-log.sh`: Added missing `--features bridge` flag

### Performance
- **Notification dispatch 17x faster** (170ns → 10ns): Early-return before lock acquisition when request is a notification
- **Parameter validation 26% faster** (47ns → 35ns): Merged `tools` + `compiled` HashMaps into single `entries` map, eliminating key duplication
- **Schema validation 8% faster** (107ns → 99ns): Same registry merge reduces lookup overhead

### Changed
- `ToolRegistry` internal structure: merged separate `tools` and `compiled` maps into unified `entries` map
- CLAUDE.md: Added task sizing, refactoring guidelines, testing section, documentation structure, CHANGELOG format, module table, stack table

### Added
- 3 new conformance tests: `error_codes_comply_with_spec`, `bridge_error_response_is_spec_compliant`, `registry_deregister_cleans_up_compiled_schema`
- 18 downstream consumers integrated (daimon, agnoshi, t-ron, jalwa, nein, stiva, itihas, varna, selah, hoosh, vidya, rasayan, szal, tarang, vidhana, nazar, mneme, tazama)

## [0.50.0] — 2026-03-26

### Added
- Protocol conformance test suite (41 tests in `tests/conformance.rs`)
- Streaming audit logging — all transports now call `log_tool_call()` after streaming handler completion with timing and success/error status
- `BoteError::SandboxError` variant for sandbox execution failures

### Fixed
- Streaming tool calls in HTTP/SSE, WebSocket, Unix, and stdio transports now correctly produce audit events via `log_tool_call()`
- Added missing doc comment on `Dispatcher::new()`

## [0.25.3] — 2026-03-26

### Added
- Tool sandboxing via kavach (feature `sandbox`)
- `ToolSandboxConfig` with presets: `basic()`, `strict()`, `noop()`
- `SandboxExecutor` for running commands in kavach sandboxes
- `wrap_command()` and `wrap_streaming_command()` handler wrappers
- `Dispatcher::register_sandboxed_tool()` and `register_sandboxed_streaming_tool()` convenience methods
- `BoteError::SandboxError` variant for sandbox execution failures
- Sandbox lifecycle event topics: `bote/sandbox/created`, `bote/sandbox/destroyed`, `bote/sandbox/error`
- Async-sync bridge with `OnceLock<Runtime>` fallback for non-tokio contexts

## [0.24.3] — 2026-03-26

### Added
- Full JSON Schema validation: type checking (string, number, integer, boolean, array, object), enum constraints, numeric bounds, nested object/array validation
- `CompiledSchema` — compile `ToolSchema` into typed representation for fast validation
- Default value injection via `CompiledSchema::apply_defaults()`
- `SchemaType`, `PropertyDef` types in new `schema` module
- `BoteError::SchemaViolation` variant with multiple violation reporting
- Tool versioning: `version` and `deprecated` fields on `ToolDef`
- `ToolDef::with_version()` and `ToolDef::with_deprecated()` builder methods
- `ToolRegistry::get_versioned()`, `list_versions()`, `deprecate()`, `deregister()`
- Version negotiation in `tools/call` dispatch
- Deprecation warnings via tracing + event publishing
- Dynamic tool registration/deregistration via `Dispatcher::register_tool()`, `deregister_tool()`
- Hot-reload: re-registering a tool atomically replaces its handler
- Tool namespacing: `project_tool` format enforcement on dynamic registration
- `TOPIC_TOOL_DEPRECATED` and `TOPIC_TOOL_DEREGISTERED` event topics
- Schema validation, versioning, and dynamic registration benchmarks

### Changed
- `Dispatcher` internals migrated to `RwLock` for thread-safe dynamic registration
- `ToolRegistry::validate_params()` now uses compiled schema for full type validation
- `tools/list` response includes `version` and `deprecated` fields when present

## [0.23.3] — 2026-03-26

### Added
- TypeScript bridge module with CORS and MCP result formatting (feature `bridge`)
- `wrap_tool_result` adapter — converts raw results to SY's `{ content: [{ type, text }] }` envelope
- Bridge CORS preflight handling for cross-origin TypeScript clients
- Cross-node tool discovery via majra pub/sub (feature `discovery`)
- `DiscoveryService` for announcing and subscribing to tool announcements
- `ToolAnnouncement` type for cross-node tool broadcast
- New event topics: `bote/tool/announce`, `bote/tool/discovered`
- Bridge benchmark (`wrap_tool_result` overhead)

### Changed
- `full` feature now includes `bridge` and `discovery`
- Transport codec module visibility changed to `pub(crate)` for bridge reuse

## [0.22.3] — 2026-03-22

### Added
- HTTP transport (axum-based, feature `http`)
- WebSocket transport (bidirectional, feature `ws`)
- Unix domain socket transport (newline-delimited JSON, feature `unix`)
- Graceful shutdown on all network transports via shutdown future
- SSE streaming for long-running tool calls (HTTP)
- Progress notifications during execution (StreamContext, ProgressSender, CancellationToken)
- Streaming handler type (StreamingToolHandler, dispatch_streaming, DispatchOutcome)
- Cancellation support ($/cancelRequest, CancellationToken)
- Batch requests (JSON-RPC 2.0 batch array)
- Notification support (no id, no response expected)
- Protocol version negotiation in initialize handshake (2024-11-05, 2025-03-26)
- process_message() codec function for batch/notification/single dispatch
- Audit logging via libro (AuditSink trait, LibroAudit adapter, feature `audit`)
- Event publishing via majra (EventSink trait, MajraEvents adapter, feature `events`)
- Tool call timing with automatic audit + event logging in dispatch
- Event topic constants (TOPIC_TOOL_COMPLETED, TOPIC_TOOL_FAILED, TOPIC_TOOL_REGISTERED)
- Feature flags: http, ws, unix, all-transports, audit, events, full
- progress_notification() helper for consistent JSON-RPC notification format
- extract_tool_name() helper for validated tool name extraction
- Send + Sync compile-time assertions on all public types
- Benchmark suite: 8 benchmarks (dispatch, process_message, batch, streaming, validation)
- Benchmark history logging via scripts/bench-log.sh
- CODE_OF_CONDUCT.md, CONTRIBUTING.md, SECURITY.md
- codecov.yml with 80% project / 75% patch targets
- 129 tests across all modules

### Changed
- Transport module restructured: transport.rs -> transport/ directory (codec, stdio, http, ws, unix)
- Dispatcher.dispatch() returns Option<JsonRpcResponse> (None for notifications)
- JsonRpcRequest.id is now Option<serde_json::Value> (supports notifications)
- All transports use process_message() for unified dispatch
- WS/Unix transports use outgoing message channel pattern for streaming
- Mutex locks are poison-safe across all transports (unwrap_or_else)
- Handler panics caught and returned as -32603 error responses
- Cancelled tasks return -32800 (distinguished from panics in async transports)
- BoteError is now #[non_exhaustive]
- Serialization calls use explicit BUG labels instead of bare unwrap()
- deny.toml: tightened license list, added version 2 advisories, explicit allow-registry
- Makefile: added coverage, bench, --no-default-features clippy, RUSTDOCFLAGS for doc
- Cargo.toml: added documentation, exclude, full feature

### Fixed
- HTTP returns proper JSON-RPC error for malformed JSON (was returning 422)
- validate_params rejects non-object params (was silently passing)
- Dispatch uses BoteError consistently (was hardcoding error codes)
- transport::parse_request uses Json error variant (was converting to string)
- Empty tool name now returns -32602 instead of falling through to not-found
- jsonrpc version validated (rejects non-"2.0")
- Progress notification JSON deduplicated via stream::progress_notification()
- Tool name extraction deduplicated via Dispatcher::extract_tool_name()

### Removed
- Unused dependencies: anyhow, tokio, uuid (from core; tokio re-added as optional for transports)
