# A request that repeats a JSON key is read first-wins; a handler can act on a value the schema check never saw

**Status:** 🟡 **OPEN** — filed 2026-11-03 from szal 2.3.1, against bote 3.8.5 (HEAD `3.8.5`). Measured on
the 3.4.10, 3.8.4 and 3.8.5 `dist/bote-core.cyr`, driven through `codec_process_message` on a
`dispatcher_new` dispatcher with szal's 54 tools registered.
**Severity:** medium — the dispatcher's inputSchema check and the tool handler can read different values
for one argument. No bote-owned tool is affected (they read arguments with `jsonx`, the same scanner);
any consumer whose handler parses `arguments` with another reader is.

## What happens

`_jx_scan` (`src/jsonx.cyr:237`) is bote's one key scanner — the codec, the dispatcher and the
validator share it so they "cannot read a message differently". It records the offset of a key's
**first** occurrence (`if (load64(pos + k * 8) < 0)`, `:268`) and returns as soon as every wanted key
is found (`:276`), so a later duplicate is never seen at all. Measured:

| request | bote 3.4.10, 3.8.4, 3.8.5 |
|---|---|
| `tools/call`, `params` `{"name":"nope","name":"szal_json_validate","arguments":{…}}` | dispatches the first: `-32602 tool not found: nope` |
| `szal_uuid`, `arguments` `{"count":"x","count":3}` | inputSchema check reads the first: `schema violation … count: expected integer` |
| `szal_uuid`, `arguments` `{"count":3,"count":"x"}` | inputSchema check passes on `3`; the handler is handed the raw `arguments` text (`src/dispatch.cyr:2541`), both pairs intact |

The third row is the problem. Every reader downstream of the dispatcher decides for itself what
`{"count":3,"count":"x"}` means:

- **bayan 1.11.1** (cyrius 6.7.18's stdlib) **refuses** it: "duplicate key" at the byte of the second.
  bayan's own reason (its "Duplicate object keys (1.11.1)" note) is this exact hazard —
  `{"role":"user","role":"admin"}` read one way by a validator and another by an actor.
- **serde_json** and most other readers keep the **last** value — `"x"`, which the schema check never
  judged.
- **jsonx** (bote) keeps the **first**.

So the guarantee the shared scanner gives inside bote stops at the handler boundary.

## What it did to a consumer

szal's handlers parse `arguments` with bayan. Through szal 2.3.0 they read a refused parse as `{}`, so
on cyrius 6.7.18 `szal_uuid {count:3,count:3}` answered ONE uuid and
`szal_random_token {length:4,length:4}` a 32-byte token, both `isError: false` — the tool ran on its
defaults after bote had validated the caller's values. szal 2.3.1 makes a refused parse a tool error
(`invalid arguments: duplicate key at byte 11`; szal `docs/adr/0003-refuse-json-the-stdlib-refuses.md`),
so szal now fails closed. A consumer whose handler reads last-wins would instead act on `"x"`.

## The ask

Refuse a request whose JSON repeats a key, before dispatch, so the envelope, the inputSchema check and
every handler see one document:

- **Where:** in the request read, before `tools/call` (and the other methods) look anything up — the
  envelope and `params` as well as `arguments`, nested objects included, since a handler reads those too.
- **Answer:** `-32700 Parse error` (under the stdlib's rules the request is not one JSON value), or
  `-32602` naming the key. Either way it is a protocol error, as a non-object `arguments` already is.
- **How:** `_jx_scan`'s early return means today's single pass never visits the rest of the object, so
  detection is a full walk with a key set. bayan 1.11.1's `_jk_*` set is a measured O(n) one (inline
  for ≤ 32 keys, no allocation; open addressing past that), comparing keys decoded — as `_jx_scan`
  already does (`"cnt"` is `cnt`).
- **Tests:** the three rows above, a duplicate inside a nested object, a duplicate that differs only
  by escaping (`"count"` / `"count"`), and one past `_jx_scan`'s early-return point.

## For consumers when it lands

- szal: re-vendor, and move `tests/szal_consumer_bundle.tcyr` `t_refused_args` to expect the protocol
  error; szal keeps its handler guard (`szal_args_parse` / `szal_args_error`), which fails closed on any
  argument text bayan refuses whatever the dispatcher does.
- Any consumer that worked around first-wins can drop the workaround.

## Closes when

bote refuses duplicate keys in requests, or decides not to and records why here.
