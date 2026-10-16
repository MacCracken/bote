# Consumer Audit — 2026-10-15

> **Scope**: every AGNOS repo that consumes bote, or runs an MCP server or client of its own:
> agnosai, daimon, hoosh, itihas, phylax, stiva, szal, thoth, t-ron, varna (all on bote), plus
> jalwa, mneme, sandhi and agnostik (MCP without bote). sutra and rasa do not exist under
> `~/Repos`. **Trigger**: bote 3.4.5 changed several wire behaviours to match the spec. The
> request was to find what breaks on upgrade, what is wrong on the wire today, and where bote
> could close gaps.
>
> **Method**: four read-only sub-agent passes (one per consumer group). Then every high-severity
> claim, and a sample of the rest, was re-read at its `file:line` before it went in here. Marks:
> - **verified**: re-read at the source by the auditor;
> - **measured**: shown by a probe or a live run (the auditor's, or a sub-agent's scratch probe,
>   as noted);
> - **reported**: from a sub-agent's code reading, not re-checked.
>
> No consumer repo was modified. Probes ran in scratch copies.
>
> **Result**:
> - **Not one consumer depends on the behaviour 3.4.5 changed.** Nobody matches `-32601`, branches
>   on a `-32602` from `tools/call`, re-wraps dispatcher results, or relies on `"result":null`.
> - **Every bump is gated by the toolchain, not by bote's wire.** The 3.4.x core bundle does not
>   compile on the cyrius 6.6.x pins most consumers carry (measured on 6.6.6).
> - **Two bote defects** surfaced: the JSON escaper passes most control bytes raw (measured on
>   bote 3.4.5's own `fs_read`), and no tool anywhere gets its input schema compiled, so bote's
>   type/enum/bounds validator never runs in production.
> - **The worst consumer findings are security findings independent of bote**: path traversal and
>   an open network bind in mneme, heap overflows in t-ron, an SSRF bypass and file exfiltration
>   in szal, and unauthenticated MCP endpoints in daimon, agnosai and hoosh.

---

## 1. Who uses bote, and how

| Consumer | bote | How it reaches bote | 3.4.5 wire impact |
|---|---|---|---|
| agnosai 2.1.7 | 3.3.16 `bote-core` pin | **Calls nothing.** Its `/mcp` route is hand-built on bayan; bote appears only in comments (verified: `src/server/routes/mcp.cyr:16,45,49`) | none |
| daimon 2.4.3 | 3.3.13 full pin, **plus** a second bote-core 3.3.13 through nein's `[deps.bote-core]` | Uses bote as a tool library (`libro_tool_*`, `web_fetch_handler`, `web_search_handler`, `jsonx_get_raw`); serves its own REST facade, forwards JSON-RPC through sandhi | builtins: none. Relayed upstream errors change code (reported) |
| hoosh 2.8.1 | 3.3.13 core, **vendored** | `codec_process_message` behind POST `/v1/tools/call`; calls the private `_json_emit_escaped` in 6 places | unknown tool `-32602`, invalid JSON `-32700` |
| itihas 2.5.1 | 3.3.8 full pin (opt-in feature) | Registers tools onto an embedder's dispatcher; misuses `HostRegistry` for descriptions | none |
| phylax 1.2.7 | 3.3.8 core pin, `path = "../bote"` committed | `tool_def_new` / `registry_register` / `dispatcher_handle` — with `Str` arguments (see §3) | none (already broken) |
| stiva 3.0.22 | 3.3.7 core pin | Tool definitions only; its own dispatch and result types | none |
| szal 2.2.0 | 3.3.13 core, **vendored** | 54 tools registered on a bote dispatcher; `codec_process_message` in tests | missing-argument text changes (reported) |
| thoth 0.52.8 | 3.3.13 core, **vendored** (for t-ron only) | Calls nothing; talks REST to daimon | none |
| t-ron 2.1.11 | 3.3.13 `bote-core` pin, `path = "../bote"` committed | `security_gate_dispatch` → `dispatcher_dispatch`; never inspects results | none beyond §3 |
| varna | 3.3.7 core, commented out (`-D MCP`) | Nothing linked; own ToolDef | n/a |

Not on bote: **jalwa** (stdio MCP server), **mneme** (tools/call-only HTTP endpoint behind
daimon), **sandhi** (`src/rpc/mcp.cyr`, MCP *client* transport used by daimon), **agnostik**
(LLM vocabulary, not MCP).

Of the 428 public functions in `dist/bote.cyr`, consumers call 50. **No consumer calls any
`content_*` helper.** Every consumer that returns tool results builds its envelope by hand, and
several get it wrong (§3).

---

## 2. Findings in bote itself

| ID | Sev | Finding | Where | Status |
|---|---|---|---|---|
| **B1** | **High** | **The JSON escaper emits control bytes raw.** `_json_emit_escaped` escapes `"` `\` LF CR TAB and copies every other byte below 0x20 through. It backs every bote serializer: messages, schemas, `content_text*`, `_dsp_tool_error_result`, `fs_tools`' and `libro_tools`' results. **Measured on the 3.4.5 binary:** `fs_read` of a file containing `ESC[31m` and BEL returns a reply that Python's `json.loads` rejects ("Invalid control character"). Any ANSI-coloured log a client reads breaks its stream. The conformance suite missed it because no test handler returns a control byte. hoosh calls the private name directly and inherits it. | `src/dispatch.cyr:261-281` | verified, measured |
| **B2** | **Medium** | **No input schema is ever compiled outside the tests.** `registry_register` and `dispatcher_register_tool` do not call `compiled_compile`. No `src/` module does either, nor does any consumer. Every tool therefore takes `registry_validate_params`' fallback, which checks only that `required` keys are present, through bayan's flat `json_parse`. Type, enum, bounds and nested checks (`compiled_validate`, ~500 lines, 3.4.5's SEP-1303 path) never run in production. | `src/registry.cyr:184-205,309-350`; `src/dispatch.cyr:194-203` | verified |
| **B3** | Medium | **A handler's result goes on the wire unchecked.** A bare object or array (itihas), a `Str` header (phylax) or invalid JSON reaches the client as `"result"`. A CallToolResult must carry `content`. | `src/dispatch.cyr` tools/call tail | verified (itihas); measured (phylax, sub-agent probe) |
| **B4** | Low | **A handler result carrying `isError: true` is audited and published as a success** (`TOPIC_TOOL_COMPLETED`). Only a 0 return counts as a failure. The DEBUG request line does say `status=tool_error`. | `src/dispatch.cyr` audit block after the handler call | verified |
| **B5** | Low | **`serverInfo` name and version are emitted unescaped.** An embedder's name containing `"` produces invalid JSON at `initialize`. | `src/dispatch.cyr:447,449` | verified |
| **B6** | Medium | **The bundles do not say which toolchain they need.** The 3.4.x core bundle fails to compile on cyrius 6.6.6 at `const BOTE_LOG_LINE_BYTES` (measured by sub-agent). That is the pin hoosh, szal, thoth and t-ron carry, and itihas, phylax, varna and stiva are on 6.6.2–6.6.10. 3.3.17 is the newest core bundle a 6.6.x consumer can take. The minimum (≥ 6.7.2) is in the 3.4.0 CHANGELOG and DEPS-PATTERN, not in the bundle header a vendoring repo actually reads. | `dist/*.cyr` header | measured (sub-agent) |
| **B7** | Medium | **Vendoring consumers lose sakshi.** The core sidecar omits `sakshi`, which `log.cyr` calls. A pinned consumer gets it through bote's `deps.libro` → patra's include (measured at 3.4.0). hoosh, szal and thoth copy the file instead, so they get no libro, and must declare `sakshi` themselves when they re-vendor 3.4.x. | `dist/bote-core.deps`; DEPS-PATTERN.md | verified |
| **B8** | Low | **Unprefixed public names collide.** `content_text` / `content_audio` (silent) and `content_image` (arity: build error) collide with agnostik's `llm.cyr`. `ann_new` already collided with sit's: thoth sed-renames sit's copy. `prop_*`, `validate_tool_name` and `parse_property` are also bare. Latent today; live in the first repo that vendors both. | `src/content.cyr`, `src/registry.cyr`, `src/schema.cyr` | reported (agnostik, thoth) |
| **B9** | Low | **bote's own tools publish empty schemas.** libro, web and fs tools register `{"type":"object"}` with no properties, so clients learn their arguments only from description text. Combined with B2, nothing validates them either. | `src/libro_tools.cyr:366`, `src/web_tools.cyr:481-487` | reported |

szal's include-order hazard does not hold: the sub-agent flagged `include "lib/sigil.cyr"` ahead
of `ct` / `random` / `thread_local` in the bundle header. `lib/sigil.cyr` includes all of those
(and `keccak`) ahead of its own body (lines 18–23), and includes are once-only, so their order
is kept. Checked; not a finding.

---

## 3. Consumer findings

### 3.1 Security — fix regardless of bote

| Sev | Consumer | Finding | Where | Status |
|---|---|---|---|---|
| **High** | mneme | **Path traversal → arbitrary file creation.** `mneme_create_note`'s `path` goes to `path_join(notes_dir, rel)` with no `..` or absolute-path check, then `mkdir -p` and a write. A probe wrote attacker text two levels above the vault. Existing files are refused, so a missing `~/.ssh/authorized_keys` is reachable. | `src/mcp_tools.cyr:148` → `src/store_files.cyr:48-52` | verified (code), measured (sub-agent probe) |
| **High** | mneme | **Open network endpoint.** `sandhi_server_run_opts(INADDR_ANY(), …)` with no auth, no Origin, no Content-Type or method check. Any peer can create notes or switch the vault; with the traversal above, any peer can create files. | `src/mcp_server.cyr:274` | verified |
| **High** | t-ron | **Heap overflows reachable from a `tools/call`.** `tron_risk_handler` copies the client's `agent_id` into `alloc(256)` with no bound. `_audit_details` allocates `tool + 256` but appends the reason and other fields. `_tron_events_to_json` and `audit_export_json` use fixed per-entry budgets. thoth works around two of these (`GATE_TRON_NAME_ROOM = 163`); t-ron's own docs do not mention them. | `src/tools.cyr:151-154`, `src/audit.cyr:14-31`, `src/tools.cyr:194-208`, `src/audit.cyr:216` | verified (first two) |
| **High** | szal | **SSRF bypass.** `_net_host` cuts at the first `/` or `:` with no userinfo or IPv6 handling, so `http://x@127.0.0.1/` yields host `x@127.0.0.1` and passes. `_net_is_safe` compares literal strings: `127.0.0.2`, `2130706433`, CGNAT and names that resolve inside all pass. bote's `ssrf_check` and its 3.4.4 resolve hook cover exactly this. | `src/mcp_tools_net.cyr:48-66,69-109` | verified (`_net_host`) |
| **High** | szal | **Local file exfiltration through curl.** The request body goes to curl as `-d <body>`, and a body beginning `@` makes curl read and send a local file, bypassing `szal_validate_path`. Fix: `--data-raw`. | `src/mcp_tools_net.cyr:190-193` | verified |
| High (by design) | szal | `szal_exec` runs any PATH command with any arguments (`sh -c` works). No `destructiveHint` on any of the 54 tools. hoosh plans to expose this set over HTTP (below). | `src/mcp_tools_process.cyr:186-224` | reported |
| **High** | daimon | **`web_fetch` with no SSRF guard, behind an unauthenticated endpoint.** The 3.3.13 bundle's `web_fetch` checks the scheme only, and daimon calls neither `ssrf_check` nor `ssrf_is_safe`. Mitigation: 127.0.0.1 bind by default, Host and Origin checks. Still reachable by any local process (including supervised agents), and by the whole network under `--listen 0.0.0.0`. Upgrading bote fixes it (3.4.0 literal guard, 3.4.4 resolved-address guard). | `lib/bote.cyr:7354-7358` (vendored), `src/mcp_builtin.cyr:72` | verified |
| Medium | daimon | **`libro_retention` and `libro_export` are callable from the same unauthenticated endpoint**, against the chain every `daimon_audit()` writes. Builtin calls write no audit entry. | `src/mcp_builtin.cyr:37,71`; `src/api_mcp.cyr:98-113` | verified (registration) |
| Medium | daimon | **Registration callback SSRF that relays the response.** `validate_callback_url` checks only the `http(s)://` prefix. | `src/mcp.cyr:193-204` | reported |
| **High** | agnosai | **`/mcp` has no auth by default, binds `INADDR_ANY`, and never reads Origin.** `tools/call` reaches `load_testing` (up to 500 users × 300 s of outbound GETs). Its SSRF guard checks literals only ("name resolution is deliberately not attempted"). | `src/main.cyr:160-164,432-434`; `src/router.cyr:609`; `ssrf.cyr:342-345` | verified (bind/notification paths), reported (rest) |
| Medium | stiva | **Unconfined build context** (`context_dir` straight from params), plus `stiva_push` to a client-chosen registry: host file exfiltration. No claims reach handlers. | `src/stiva_core.cyr:594-596,702-716` | verified (`context_dir`) |
| Medium | hoosh | `Access-Control-Allow-Origin: *` on every response, no Origin check, auth open when no tokens are set. Today it serves only `bote_echo`; it plans to register szal's tools, including `szal_exec`. | `src/lib/http_server.cyr:43`, `src/lib/auth.cyr:36`, `src/lib/mcp.cyr:9-14` | reported |
| Medium | jalwa | **Writes to stdout from the playback thread** (`syscall(1, 1, …)`) while stdout is the stdio MCP wire. The line can land inside a response. Raw syscalls also break `jalwa mcp` off x86_64 (agnos: syscall 0 is exit). | `src/playback/audio.cyr:98,107`; `src/mcp_serve.cyr:289,302` | verified (audio) |
| Medium (latent) | phylax | `phylax_scan` hashes any host path the client names (up to 100 MB), with no confinement: a file-existence and hash oracle, once its MCP registration works. | `src/integration.cyr:330-345` | reported |
| Low | stiva | Client strings written raw to fd 2 can forge log lines. | `src/stiva_core.cyr:816`, `src/error.cyr:86-96` | reported |

### 3.2 Wire and spec violations

| Sev | Consumer(s) | Finding | Where | Status |
|---|---|---|---|---|
| **High** | phylax, t-ron | **`Str` passed where bote takes cstrs.** Tool names, descriptions, schema types and property keys are `str_from(...)` objects, and bote reads the 16-byte header as text. phylax: `tools/call` reports "tool not found" and `tools/list` SIGSEGVs (measured on 3.4.5). t-ron: `tools/list` emits garbage bytes as invalid JSON, and every `tron_risk` / `tron_policy` call fails validation on a garbage `required` key (measured). phylax's handlers also return `Str` and call bayan's flat `json_get` on bote's cstr args, which crashes (measured). Nothing in either repo's tests drives the wire. | phylax `src/integration.cyr:329-433`; t-ron `src/tools.cyr:44-84` | verified (code), measured (sub-agent probes) |
| **High** | mneme | **Invalid JSON on every multi-line result.** `_mcp_escape` escapes only `"` and `\`, and results contain their own `\n\n`. 5 of 15 probe replies failed `json.loads`. daimon relays them verbatim. | `src/mcp_protocol.cyr:7-18` | verified |
| **High** | agnosai, jalwa, mneme | **Notifications are answered.** agnosai turns a missing id into `null` and replies; jalwa writes a reply for every line; mneme returns `-32601` with HTTP 200 for `notifications/initialized`. JSON-RPC §4.1; Streamable HTTP wants 202 with no body. t-ron's gate also answers a *denied* `tools/call` notification. | agnosai `routes/mcp.cyr:565-566`; jalwa `src/mcp_serve.cyr:314-322`; mneme `src/mcp_server.cyr:113-115` | verified (agnosai, jalwa) |
| Medium | itihas | **Tool results are not CallToolResults**: bare records and arrays (`{"ok":false,…}` for failures, never `isError`). SDK clients reject them. Fix: `content_text_response` / `content_text_error_response`. | `src/mcp.cyr:50-73,115-197` | verified |
| Medium | agnosai, jalwa | **No `ping`** (falls to `-32601`; jalwa answers it as a *result* `{"error":"unknown method: ping"}`). The receiver MUST answer `{}`. | agnosai `routes/mcp.cyr:594`; jalwa `src/mcp_serve.cyr:271-276` | verified (agnosai) |
| Medium | agnosai, jalwa, mneme, stiva | **Unknown tool returned as an `isError` result**, not the spec's `-32602`. mneme's also leaks heap-pointer bytes (a `Str` added with `str_builder_add_cstr`). agnosai's is deliberate (Rust oracle) and pinned by a test. | agnosai `routes/mcp.cyr:257-261`; mneme `src/mcp_tools.cyr:403`; jalwa `src/mcp.cyr:294`; stiva `src/stiva_core.cyr:850` | verified (agnosai) |
| Medium | agnosai, jalwa | **Parse errors and invalid requests are not JSON-RPC errors.** agnosai answers HTTP 400/422 with `{"error":…}`. jalwa checks only the first byte, cannot handle whitespace, finds keys anywhere in the line (a nested `"id"` was echoed), and echoed an object id as invalid JSON. | agnosai `routes/mcp.cyr:545-562`; jalwa `src/mcp_serve.cyr:67-78,249-259` | reported (measured on jalwa's binary by sub-agent) |
| Medium | agnosai | Refuses batches while advertising `2025-03-26`, which requires them; ignores the requested `protocolVersion`. | `routes/mcp.cyr:63,546` | reported |
| Medium | agnosai | `resources/list` builds URIs from the agent *name*, `resources/read` looks up the agent *key*. Listed resources that 404. | `routes/mcp.cyr:448-451,488-493` | reported |
| Medium | daimon | The relayed-upstream-error envelope uses `"type":"text/plain"` (must be `"text"`) and a non-standard `code` member. The branch is also dead: `sandhi_rpc_ok` is 0 for a JSON-RPC error on HTTP 200, so every upstream error becomes HTTP 502. | `src/api_mcp.cyr:166,183-193` | verified (envelope), reported (dead branch) |
| Medium | daimon (+ sandhi) | The forwarder sends one bare request: no `initialize`, no `MCP-Protocol-Version`, no session id, `Accept: application/json` only. It cannot reach a tool hosted on `bote streamable` (400 / 404). sandhi's client cannot send a notification, drops response headers, and detects errors by a key anywhere in the body. | daimon `src/trace.cyr:220-227`; sandhi `src/rpc/mcp.cyr:34-49,141-166`, `src/rpc/dispatch.cyr:77-83` | reported |
| Low | daimon, mneme, varna | `inputSchema` without `"type":"object"`, or with only some properties; clients cannot see optional arguments. | daimon `src/mcp.cyr:35-56`; mneme `src/mcp_server.cyr:179-209`; varna `src/mcp.cyr:39-45` | reported |
| Low | jalwa | 64 KiB line cap splits a long request into two replies; protocol version pinned to `2024-11-05`. | `src/mcp_serve.cyr:166,286-296` | reported |
| Low | varna | transliterate under-budgets escapes (4× for a 6× `\u00XX`) and returned truncated JSON in a probe. | `src/mcp.cyr:268` | reported (measured by sub-agent) |
| Low | phylax | Three listed tools have no handler. | `src/integration.cyr:392-430` | reported |

### 3.3 What a bump to 3.4.5 costs each consumer

| Consumer | Required | Notes |
|---|---|---|
| all on 6.6.x | cyrius ≥ 6.7.2 (6.7.9 in practice) and the 6.7.x silent-semantics scan of their own code (3.4.0 CHANGELOG) | B6. A repo that cannot move yet can take 3.3.17 |
| daimon | toolchain 6.6.6 → 6.7.9; move nein's bote-core pin in lockstep (or stop nein pushing it); fix `tests/smoke.sh:644`, which fetches `http://127.0.0.1:18094/page` and will now be refused by the SSRF guard; retire the `deps.sigil` / `deps.sakshi` / `deps.bayan` overrides and committed `path` lines | verified (smoke) |
| hoosh, szal, thoth | re-vendor, add `sakshi` to `[deps] stdlib` (B7); thoth's `tests/cases/vendor.cyr:60` pins the `# Version` header; hoosh's `sync-bote.sh` comment is stale | sync scripts check only the header, not a hash |
| t-ron, phylax | delete the committed `path = "../bote"`: on 6.6.x a path beats the tag, so the next local `cyrius deps` would vendor bote's 3.4.5 working tree, which does not compile there | |
| itihas | re-resolve: `cyrius.lock` still records bote 3.3.7 / libro 2.8.12 under a 3.3.8 tag | reported |
| varna | its enable recipe in `cyrius.cyml` describes the 3.3.7 bundle (12 modules, `chrono` only); rename `tool_def_name` / `tool_def_description`, which collide with bote's | verified (collision) |
| agnosai | nothing on the wire; either delegate to bote or drop the pin | |

---

## 4. Gaps bote could close

Ordered by how many consumers each would help and how much it would fix.

| # | Gap | Helps | Fixes |
|---|---|---|---|
| G1 | **Complete, public JSON string escaper** (`\u00XX` for C0; NUL-safe length form). Fixes B1 and B5. | everyone; hoosh calls the private one; itihas, varna, mneme, jalwa and daimon each re-implement it (mneme's and varna's wrongly) | B1, B5, mneme invalid JSON |
| G2 | **Compile the input schema at registration**, and refuse a schema whose `type` is not `"object"` or whose property value is not valid JSON. | every tool in production (B2); the `type` check would have caught t-ron's and phylax's `Str` bug at registration | B2, the `Str` footgun |
| G3 | **Check a handler's result**: valid JSON (`jsonx_valid`), an object with `content`; otherwise wrap it as text or turn it into an `isError` result. Audit `isError` results as failures. | itihas, phylax; every future handler | B3, B4 |
| G4 | **Envelope-free entry points**: `dispatcher_call_tool(d, name, args, claims)` returning a CallToolResult, and a public tools-list builder. | daimon, itihas and stiva, which have non-JSON-RPC front doors and today match names by hand or misuse `HostRegistry` | daimon's hand-matched builtins, itihas's HostRegistry misuse |
| G5 | **A pre-dispatch guard hook** for `tools/call` (e.g. `dispatcher_set_call_guard`), so a security gate runs inside the codec. | t-ron (and thoth through it): today the gate sits outside the codec and misses batches, notifications and 3.4.5's admission checks | t-ron answering denied notifications |
| G6 | **HTTP guards in a core-reachable profile**: `validate_origin`, `validate_protocol_version` and bearer/allowlist auth live in `session.cyr` / `auth.cyr`, full bundle only. | agnosai, mneme, hoosh, which run their own sandhi servers and all lack Origin checks | three Origin findings |
| G7 | **SSRF classifier in a core-reachable profile** (`host.cyr` is full-bundle only), plus the resolve-hook pattern from `web_fetch`. | szal, stiva, agnosai, daimon's callback URLs | szal bypass, agnosai literal-only guard |
| G8 | **A handler context pointer** (`fn h(args, claims)` has nowhere to put state). | jalwa, mneme — both would otherwise need globals to adopt bote | adoption blocker |
| G9 | **Decoding and typed getters**: `jsonx_get_str` returns raw escapes; no int/bool/array getters. | jalwa (`max`), mneme (`limit`, `tags`), and every handler reading arguments | adoption blocker |
| G10 | **An MCP client layer**: initialize / initialized, session-id capture, protocol-version header, both error shapes. sandhi's ADR defers protocol semantics to bote, but bote has no client side. | daimon (via sandhi), thoth | daimon's forwarder |
| G11 | **Dynamic resources and prompts**: a provider hook or real URI templates instead of exact-key registries. | agnosai, whose set is a live map | agnosai's own MCP route |
| G12 | **Namespace the bare public names** (`content_*`, `ann_*`, `prop_*`, `validate_tool_name`, `parse_property`), keeping aliases for one minor. | agnostik / sit co-vendors | B8 |
| G13 | **State the minimum cyrius in the bundle header**, next to `# Version:`. | every vendoring repo | B6 |
| G14 | **A vendoring-friendly pin** (cyrius side): a `[deps.bote]` limited to `dist/bote-core.cyr` still drags in bote's libro and majra blocks, which is why three repos vendor by hand and are 13 releases behind. | hoosh, szal, thoth, t-ron | stale copies |

Moving onto bote would remove most of jalwa's and mneme's wire findings in one step
(notifications, ping, error codes, invalid JSON, id/params types, version negotiation), and its
`codec_process_message` returning 0 maps directly to "write nothing" on stdio and 202 over HTTP.
Both need G1, G8 and G9 first. sandhi sits below bote and should fix its client transport itself
(dual `Accept`, notifications, response headers, depth-aware envelope detection). agnostik is not
MCP; it needs only G12.

---

## 5. Recommended order

> **Disposition (2026-10-15).** Accepted for bote's roadmap: G1 (B1 + B5) is the next patch,
> 3.4.6, ahead of the SSRF classifier gaps, which move to 3.4.7. G2 + G3 (B2–B4) are 3.4.8 and
> G13 (B6 + B7) is 3.4.9. G4, G5, G8, G9 and G12 are the roadmap's consumer adoption track, and
> G10 / G11 are noted there as minor-sized. G6 + G7 are an open profile decision, and G14 is an
> upstream filing still to be made. The consumer-side items below belong to each repo; none has
> been filed or fixed from here.

**In bote** (each a patch, one at a time):
1. G1: the escaper (B1, B5). It is a defect on the shipped binary, measured, and it touches every
   serializer. A conformance scenario with a control byte in a tool result belongs with it.
2. G2 + G3: compile schemas at registration, check handler results, audit `isError` as failure.
   These make the validator and the SEP-1303 path real in production.
3. G13: the toolchain line in the bundle header, which is cheap and unblocks vendoring repos'
   planning.
4. The 3.4.6 SSRF classifier gaps already on the roadmap, then G6 / G7 (core-reachable guards) as
   a profile decision, then G4 / G5 / G8 / G9 as the adoption track for jalwa, mneme, daimon and
   t-ron. G10 and G11 are larger and belong in a minor.

**In consumers** (security first, independent of bote):
- mneme: confine `create_note` paths; bind 127.0.0.1, require a token from daimon, check Origin.
- t-ron: bound every copy in `tools.cyr` / `audit.cyr`; pass cstrs to bote.
- szal: parse the host properly (or use bote's `ssrf_check`); `--data-raw` instead of `-d`.
- daimon: upgrade bote (SSRF guard); deny `libro_retention` by default; audit builtin calls; fix
  the relayed-error envelope; validate callback URLs.
- agnosai: auth on by default or a localhost bind; Origin check; answer notifications with 202;
  add `ping`.
- phylax: cstrs, `jsonx_get_str`, CallToolResult returns, handlers for all listed tools.
- itihas, jalwa, mneme, stiva: CallToolResult envelopes (`content_text_response` /
  `content_text_error_response`) once G1 lands.
- jalwa: move the playback messages to fd 2; stop answering notifications; answer `ping`.
