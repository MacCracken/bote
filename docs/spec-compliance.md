# MCP Spec Compliance

> **Spec Version**: 2025-11-25 (default), 2026-07-28 (stateless core) | **Bote Version**: 3.8.11 (cyrius 6.7.18) | **Last Audited**: 2026-11-01 ([closeout audit](audit/2026-11-01-closeout.md))
>
> ⚠ **bote speaks the current revision, 2026-07-28, beside the older ones.** [That section](#mcp-2026-07-28-stateless-path) lists what is in and what is not. Every other table here is against 2025-11-25.

This file lists what the shipped Cyrius implementation **covers today**, including the
gaps — a ❌ row is a defect on the released binary, a ⏳ row is planned work. Both are
scheduled in [`docs/development/roadmap.md`](development/roadmap.md). Re-audited at the
3.3.12 documentation sweep by probing the built binary, not by reading the previous table.
Since 3.4.5 the protocol rows below are also asserted, by spec clause, in
`tests/conformance.tcyr`, which CI runs. On the 3.4.4 sources that suite failed 29 of its 117
assertions, which is how the rows marked (3.4.5) were found.

---

## Protocol Versions

| Version | Status |
|---|---|
| `2024-11-05` | ✅ |
| `2025-03-26` | ✅ |
| `2025-06-18` | ✅ (3.3.13) |
| `2025-11-25` | ✅ **default** |
| `2026-07-28` | ✅ stateless, on every transport (3.5.0; HTTP 3.5.1). Gaps listed below |

The first four are negotiated via `initialize`. The server picks the highest
mutually supported version, so `initialize` never lands on `2026-07-28`, which
has no `initialize`. A request selects that revision itself, in `_meta`, and one
process serves both eras. The legacy list lives in exactly one place —
`_mcp_is_supported` (`src/dispatch.cyr`); `session.cyr`'s header validator defers to it. Through
3.3.12 there were two copies, and the drift they allowed was asymmetric: a
version missing from the header validator 400s on every HTTP-family transport
while stdio negotiates it happily. Four assertions now pin the two readers
against each other per version.

JSON-RPC batching was removed from the spec at `2025-06-18`; bote keeps accepting
batch arrays on every version (a superset is harmless to a client that never
sends one, and refusing them per-version would break the earlier clients that
legitimately batch).

---

## MCP 2026-07-28, stateless path

Selected per request by `params._meta["io.modelcontextprotocol/protocolVersion"]`. A request without
that key, and every `initialize`, takes the legacy path unchanged. Handlers are the same on both. On HTTP
the header and `_meta` must agree first; a POST with neither is treated as `2025-03-26`, as the spec allows.
Asserted in `tests/conformance.tcyr`, M1 to M75, through `codec_process_message`, and in
`tests/bote_http_era.tcyr` on the real HTTP handlers.

| Spec Requirement | Status |
|---|---|
| Every request carries its version and client capabilities in `_meta`; a required field missing or of the wrong type is `-32602` | ✅ (3.5.0) |
| An unsupported version is `-32022`, with `data.supported` and `data.requested` | ✅ (3.5.0) — the supported list is the same on both eras. `requested` is escaped, since a client wrote it |
| An older revision named in `_meta` is served on the legacy path | ✅ (3.5.0) |
| `server/discover`, which servers MUST implement: `resultType`, `supportedVersions`, `capabilities`, `ttlMs`, `cacheScope`. A discover request without `_meta` is `-32602` | ✅ (3.5.0) |
| `resultType: "complete"` on every result | ✅ (3.5.0) — added to what a handler returned, and left alone if the handler set it |
| `ttlMs` + `cacheScope` on `server/discover`, the `tools`, `prompts` and `resources` lists, `resources/templates/list` and `resources/read` | ✅ (3.5.0) — `0` and `"private"`. A longer lifetime needs change notification to invalidate it, and that is `subscriptions/listen`, roadmap 3.7.x |
| `serverInfo` in `result._meta["io.modelcontextprotocol/serverInfo"]` | ✅ (3.5.0) |
| A missing resource is `-32602`, never `-32002` | ✅ (3.5.0) — `-32002` stays on the legacy path |
| `ping` and `initialize` are gone | ✅ (3.5.0) — a modern `ping` is `-32601`. An `initialize` selects the legacy path whatever `_meta` it carries, so it never lands on a revision that has no `initialize` |
| `listChanged` advertised only where it is delivered | ✅ (3.5.0) none; (3.7.2) `server/discover` advertises it for tools, prompts and resources on `http`, `bridge` and `streamable`, which carry `subscriptions/listen`; (3.7.3) and on stdio, unix and WebSocket. It follows the message's transport, not the dispatcher, so a dispatcher served on two transports advertises it only on the one that delivers |
| `tools/list` in a deterministic order | ✅ registration order, in both eras |
| `outputSchema` of any JSON Schema 2020-12, and `structuredContent` any JSON value (SEP-2106) | ✅ (3.8.0) a root of any type, listed as written on this revision and as an object's `result` before it; `structuredContent` judged by it, sent as it is here and wrapped before; one that is no object gets a text block with its JSON when the result has none. bote judges the `inputSchema` subset and warns at registration of the keywords it does not enforce. See Core Protocol below |
| `instructions` in `server/discover`'s result; serverInfo in every result's `_meta` | ✅ (3.8.1) instructions when set; discover's serverInfo every field, another result's the name, version and title |
| Every POST: `MCP-Protocol-Version` and `Mcp-Method`, and `Mcp-Name` on `tools/call`, `resources/read` and `prompts/get`. Missing, invalid or disagreeing with the body is `400` + `-32020` | ✅ (3.5.1) `src/http_era.cyr`, on `bote http`, `bote streamable` and (3.5.2) `bote bridge`. Names are case-insensitive, values case-sensitive. A field on two lines combines into a value that matches nothing, so it is refused (3.5.2) |
| `Mcp-Name` in the `=?base64?…?=` sentinel, decoded before the comparison | ✅ (3.5.1) lower-case markers only, canonical Base64 only |
| An unsupported version is `400` + `-32022`; a method bote does not implement is `404` + `-32601`; a `_meta` missing a required field is `400` + `-32602` | ✅ (3.5.1, corrected at 3.5.2) that clause is about `_meta`. Every other `-32602` and `-32603` is in-band on a `200`: the spec gives them no status, and the official client reads a JSON-RPC error out of a `200` or a `400`, not out of a `500` |
| A notification is `202` with no body | ✅ (3.5.1), in both eras. It was `204` through 3.5.0, which 2025-11-25 does not allow either |
| One message per POST | ✅ (3.5.1) a batch, a response or a non-object is `400`, a batch that names this revision included |
| Responses in `application/json` | ✅ the spec lets the server choose per request; (3.7.2) `subscriptions/listen` answers with `text/event-stream` |
| `GET` and `DELETE` are `405` to a request naming this revision, with `Allow` | ✅ (3.5.1). A legacy `GET` keeps its stream |
| `Mcp-Session-Id` and `Last-Event-ID` are ignored, and no session is minted | ✅ (3.5.1) |
| `Mcp-Param-{Name}`, for a tool that declares `x-mcp-header` | ✅ (3.5.2) a value in the body needs its header, decoded and equal, integers compared by value and booleans as `true` / `false`. Missing, malformed, mismatched or undecodable is `400` + `-32020`. A header with no value in the body is refused too, which the spec's table does not ask for and no conforming client meets. A header the tool does not declare is ignored |
| An `x-mcp-header` that breaks the rules makes the tool invalid | ✅ (3.5.2) registration refuses it, since a client would drop the tool: an empty or non-token name, a type other than string, integer or boolean, a repeat ignoring case, or anywhere but a chain of `properties` keys |
| Integers mirrored in a header stay in the safe range | ✅ (3.5.2) a value past ±(2^53 − 1) is refused with `-32020`, even with its header |
| `-32021` for a missing client capability | ✅ (3.8.2) a handler's InputRequiredResult that needs a capability the request's `clientCapabilities` does not declare is answered `-32021`, with `data.requiredCapabilities` in the ClientCapabilities shape, every missing one merged: `elicitation.form` (a bare `elicitation: {}` declares it), `elicitation.url`, `sampling`, `sampling.tools`, `roots`. HTTP `400` |
| Multi round-trip requests: `InputRequiredResult` on `tools/call`, `prompts/get` and `resources/read` | ✅ (3.8.2) `resultType: "input_required"`, `inputRequests` keyed by the server, `requestState`, or both, the retry a new request with `inputResponses` and `requestState` beside the same params. Each request is `elicitation/create` (form or url), `sampling/createMessage` or `roots/list`, and anything else, an unknown mode, none of either, a key twice, is `-32603`. Never a cache hint on one. On an older revision there is no way to ask: a tool asking is a tool error, a prompt or resource `-32603`. `completion/complete` may not ask (`-32603`). `inputResponses` and `requestState` mean nothing on any other method |
| `requestState` is attacker-controlled: integrity-protected, refused when it fails, bound to the principal, the request and a short expiry | ✅ (3.8.2) every state a handler gives is sealed and every one a client brings is opened, by the dispatcher's sealer, before any handler runs; one that does not open is `-32602`. The full bundle's sealer (`src/state_seal.cyr`) is HMAC-SHA256 over a domain tag, the binding, an expiry (600 s by default) and the payload. (3.8.3) The binding is the request and who made it, each part its length and its bytes: the method, its target (the tool's or prompt's name, the resource's URI), the arguments, compact, and the principal `dispatcher_set_state_principal` names for the claims (a JWT's issuer and subject in the binary), else the token the request presented; a request with claims that name no one cannot be bound, and its state is refused both ways. ⚠ Corrected at 3.8.3: through 3.8.2 this row's ✅ claimed the principal and the request, but the arguments were not bound, and a request whose claims were not its token was bound to no one. With no sealer a state is refused both ways. A state is not single-use, as the spec allows: a handler that must spend one once keeps that record |
| `inputResponses` the client sends | ✅ (3.8.2) an object of objects, else `-32602` before any handler; a handler reads them with `bote_input_response_a` / `bote_input_accepted_a`, by the keys it asked under |
| A third-party client | ✅ (3.5.3) `@modelcontextprotocol/client` 2.4.0 connects and uses every capability bote has, 410 checks on `http`, `streamable` and stdio in both eras and `bridge` in the modern one, and (3.6.0 to 3.6.3) its whole OAuth flow, with HS256, RS256 and ES256 tokens from a mock authorization server, and a step-up from a 403 it makes on its own. `./scripts/conformance.sh` repeats it |
| `@modelcontextprotocol/conformance`, by its frozen requirement list | ✅ on 2026-07-28: (3.8.2) **all 39 scenarios it scores**, the 14 `input-required-*` and every check of `server-stateless` among them; the list was given here as 26 of 37 from 3.5.3, and holds 39. Unscored, and failing: the optional tasks extension and `json-schema-2020-12`. ⏳ **22 of 30** on 2025-11-25 (`tools-call-with-progress` since 3.7.6), the same on `http` and `streamable`. What fails there is a feature bote lacks: sampling and elicitation as server-to-client requests, logging, subscriptions. `tests/fixtures/conformance-expected-<revision>.yaml` lists each, and why |
| `subscriptions/listen`: each kind opt-in, never one not asked for, the honored subset acknowledged first in `notifications/subscriptions/acknowledged`, every message stamped with `io.modelcontextprotocol/subscriptionId`, keepalives | ✅ (3.7.2) on `http`, `bridge` and `streamable`, for `toolsListChanged`, `promptsListChanged` and `resourcesListChanged`, each honored only where that list exists. A filter that is not one is `-32602`; past `BOTE_LISTEN_MAX` streams, `-32603` "Subscription limit reached", the TypeScript SDK's words. With nothing honored the stream ends at once with its result, as the SDK does. A client that closes, sends anything, or stops reading loses its stream. `: keepalive` every `BOTE_LISTEN_KEEPALIVE_MS` (15 s, the SDK's) |
| `resourceSubscriptions` and `notifications/resources/updated` | ✅ (3.7.4) on every transport, where the dispatcher has resources: the acknowledgement lists the URIs honored, decoded and escaped again, once each, at most 32 in 4 KB, none empty or holding a NUL or past 1 KB. An embedder says a resource changed with `dispatcher_resource_updated(d, uri)`, from any thread, and every subscription to that URI, or to one it falls under (followed by `/`, `?` or `#`, or anything after a trailing `/`), is told, naming the URI that changed, as the spec allows. `server/discover` advertises `resources.subscribe` where listen is carried. (3.7.5) A resource's required scopes hold, as the Resources page asks ("Resource permissions SHOULD be checked before operations"): a listen made with a token is honored no URI the token may not read, judged by what would serve it as `resources/read` judges it, and told of an update to a URI that requires a scope only when it holds that URI exactly; a stream keeps no token, so not through a prefix |
| `subscriptions/listen` on stdio, unix and WebSocket | ✅ (3.7.3) the connection is the stream: the acknowledgement is the listen's first message on it, the changes go out between the replies to other requests, from whatever thread made them, and `notifications/cancelled` naming the listen's id ends it, matched byte for byte. An id already open is `-32600`; past `BOTE_LISTEN_MAX` a connection's listens get `-32603`. With nothing honored, the acknowledgement and then the result. A listen in a batch is `-32601`: 2026-07-28 has no batches. On WebSocket the token is judged again before a change goes out, as before a frame is served |
| The suite's listen checks (`sep-2575-*` in `server-stateless`) | ✅ (3.7.3) all six on `http` and `streamable`: the acknowledgement first, the id stamped, the filter honored, and a tool and a prompt change delivered through the suite's `test_trigger_tool_change` and `test_trigger_prompt_change`, which the fixture now has |

---

## Core Protocol

| Spec Requirement | Module | Status |
|---|---|---|
| JSON-RPC 2.0 — request, response, notification, batch | `protocol` + `codec` | ✅ |
| Invalid JSON is `-32700`; valid JSON that is not a request is `-32600` (§5.1) | `jsonx_valid` + `codec` | ✅ (3.4.5) — the whole message is syntax-checked (RFC 8259, nesting ≤ 64) before anything reads it. Through 3.4.4 `not valid json` was `-32600`, the spec's truncated-object example was dispatched as a notification, and `"id":01` was echoed back as invalid JSON |
| A name is unique within an object (RFC 8259 §4 SHOULD; I-JSON, RFC 7493 §2.3, MUST): a request that names a key twice in any object, keys compared as they decode, or names one that is no text, is `-32600` | `jsonx::_jx_valid_keys_a`, `codec::_cdc_admit_l_a`, `dispatch::dispatcher_dispatch_a` | ✅ (3.8.6) — checked with the syntax, before anything reads the message, on every transport and in the public dispatcher. Through 3.8.5 bote read the first of a repeated key and handed a handler both, so `{"count":3,"count":"x"}` passed an integer schema and a handler whose parser takes the last acted on `"x"`. The id is echoed unless it is the key named twice: `-32700` would carry a null id, which leaves a client waiting |
| `id` is a string, a number or null; `params`, if present, is an object or an array (§4) | `codec::_cdc_admit` | ✅ (3.4.5) — otherwise `-32600`. An explicit `"params":null` is read as absent. A null id is answered, though MCP forbids a client to send one |
| Spec error codes (-32700, -32600, -32601, -32602, -32002, -32000, -32003, -32603, -32800) | `error` + `dispatch` | ✅ (`-32002` since 3.4.5) |
| Every response is valid JSON: strings escaped per RFC 8259 §7, control bytes included, and valid UTF-8 (§8.1), invalid bytes as U+FFFD (3.4.15); every handler's result on one line of valid JSON (3.4.15) | `jsonx::bote_json_escape`, `dispatch::_dsp_checked_result` | ✅ (3.4.6) — through 3.4.5 the escaper left control bytes other than LF / CR / TAB raw, so a tool result holding an ESC or a BEL (an ANSI-coloured log through `fs_read`) was not JSON, and an embedder's `serverInfo` was not escaped at all. Asserted by the conformance suite's standing valid-JSON check (J15, F12) and a fuzz property |
| `initialize` handshake (serverInfo + capabilities + version negotiation; `serverInfo` configurable via `dispatcher_set_server_info`) | `dispatch` | ✅ |
| `ping` → empty `{}` result (spec: the receiver *MUST* respond promptly) | `dispatch` | ✅ (3.3.13) — answered ahead of every capability gate, so a bare dispatcher replies; emits no audit or event record (liveness, not activity). bote answers pings, it does not originate them |
| `notifications/initialized` / `notifications/cancelled` — accepted, never answered | `codec` (no response to any notification) | ✅ |
| Cancellation, 2026-07-28: on Streamable HTTP "the server MUST treat a client disconnect as cancellation of that request"; on stdio the client MUST send `notifications/cancelled`; the server SHOULD stop and MUST NOT send any further messages for the request | `protocol::bote_cancelled`, `http_era`, `transport_stdio`, `transport_unix`, `fetch` | ✅ (3.7.5) cooperatively: a handler asks `bote_cancelled()` as it works. On `http`, `streamable` and `bridge` a client that closed its connection has cancelled; on stdio and a Unix socket a `notifications/cancelled` naming the request, among the lines that have arrived, compared by value, and on a Unix socket a client that closed both ways. Once a handler has found its request cancelled nothing more goes out for it, its response included. bote's `web_fetch` and `web_search` ask while they wait. A handler that never asks runs to its end and is answered, which the spec allows ("MAY ignore … processing has already completed"). (3.7.7) On a WebSocket, a `notifications/cancelled` frame read while the handler runs, from what has arrived whole and never waiting for the rest of a frame, each frame read kept for the loop in its turn; a close frame or a client gone cancels too. 2025-11-25 on `bote streamable`: a `notifications/cancelled` POST of its own, on the request's session, which another worker serves, reaches the request in flight there, and gets no JSON-RPC response ("SHOULD NOT send a response"); (3.8.0) its POST gets an SSE stream ended with no event, since a request's POST MUST get an SSE stream or a JSON object, or 202 for a client that takes no SSE. Through 3.7.7 it got no HTTP response at all. On `bote http`, which serves one request at a time, a cancellation POST waits behind the request it names, which is answered first, as the spec allows. 2025-11-25's HTTP disconnect is no cancellation, and bote does not treat it as one |
| `tools/list` with full `inputSchema`, any JSON Schema 2020-12 keyword at its root (SEP-1613, SEP-2106) | `registry::tool_schema_from_json`, `dispatch::_emit_input_schema` | ✅ (3.8.3) an inputSchema given as its own JSON is listed as written on both revisions, on one line, `$schema`, `$defs`, `allOf`, `if` / `then` / `else` and `additionalProperties` among what it keeps; refused at registration unless it is an object whose `type` is `"object"`, `properties` an object of schemas and `required` an array of names, with no key twice and no NUL, as a client's parser requires. Every inputSchema is listed as registration kept it: through 3.8.2 a ToolSchema's property written on several lines put a raw LF in the list, which ended the message on stdio and Unix |
| `outputSchema`: "Servers MUST provide structured results that conform to this schema" | `registry::tool_def_with_output_schema`, `dispatch::_dsp_tool_result_out_a` | ✅ (3.8.4) bote's own libro tools and `fs_read` list one, and the official client checks their results against it on the binary. (3.8.0) a tool's outputSchema, its own JSON, refused at registration unless it is a JSON object, compiled there, and listed as written. Every result that is no error must carry `structuredContent` that matches it: one that does not, or has none, or names `content`, `structuredContent` or `isError` twice, is sent as a tool error naming what is wrong, never as it was, and audited as a failure. A handler's bare value is the `structuredContent`, its JSON in a text block, as the spec says a tool SHOULD send it. Validation is the `inputSchema` subset below. 2025-11-25 allows only an object at the root and only an object as `structuredContent`: a tool whose root is anything else is listed there with the schema as the property `result` of an object, references into it moved to match, and its results' `structuredContent` goes as `{"result": …}`, as the official SDK does it; `structuredContent` that is no object is wrapped the same way for every tool. On every revision `structuredContent` that is no object gets a text block holding its JSON when the result has none (SEP-2106 §4.3) |
| `title` on Tool, Prompt, PromptArgument, Resource, ResourceTemplate and Implementation; `icons` on all of them but PromptArgument; Implementation's `description` and `websiteUrl` | `registry::bote_icon_*`, `bote_icons_check`; `dispatch::dispatcher_set_server_details` | ✅ (3.8.1) checked where each is registered or set, and refused when the spec would not allow it: a title that is empty, an icon `src` that is no `http://` / `https://` URL and no `data:` URI of Base64, a `mimeType` that is no type/subtype, a size that is no WxH or `any`, a theme other than `light` or `dark` (the official client refuses a whole list over one), a `websiteUrl` that is no web URL. Each list sends the JSON built from what passed. `initialize` and `server/discover` send every serverInfo field; another 2026-07-28 result's `_meta` serverInfo carries the name, the version and the title. bote fetches no icon; the spec's same-domain SHOULD and SVG caution are the consumer's. The official TypeScript client (2.4.0) drops a prompt argument's `title` when it parses `prompts/list`: its schema for PromptArgument has none, where the spec's does |
| `instructions`: in `InitializeResult` (2025-11-25), in `DiscoverResult` (2026-07-28) | `dispatch::dispatcher_set_instructions` | ✅ (3.8.1) in each, escaped once when set |
| Pagination: `nextCursor` omitted when there is no next page; an invalid `cursor` is `-32602` | `dispatch` | ✅ (3.4.5) — bote returns every item in one page and never issues a cursor, so any `cursor` a client sends is invalid and gets `-32602`, the spec's code for one, on `tools/list`, `prompts/list`, `resources/list` and `resources/templates/list`. A null `cursor` is read as absent. Through 3.4.4 a cursor was ignored and the first page sent again, which a client concatenating pages would take as duplicates |
| `tools/call` with arguments + version selection | `dispatch` + `schema` | ✅ |
| `tools/call` error split: an unknown tool or a malformed CallToolRequest (missing / empty / non-string `name`, non-object `arguments`) is a protocol error `-32602`; arguments failing the tool's `inputSchema` are a tool execution error, a result with `isError: true` | `dispatch` | ✅ (3.4.5) — per the tools spec's "Error Handling" and SEP-1303. Through 3.4.4 an unknown tool was `-32601` and a schema violation `-32602`. The dispatcher has no per-session negotiated version, so this split applies to every revision. 2025-06-18 already listed "invalid input data" as a tool execution error |
| `tools/call` result is always a CallToolResult | `dispatch::_dsp_coerce_result` | ✅ (3.4.5; 3.4.8) — a handler that returns nothing or invalid JSON is a failed call (`isError: true`); bare JSON arrives as a text block, one content block is lifted into `content`, and a CallToolResult passes through. A result carrying `isError` is audited and published as a failure. Through 3.4.7 bare JSON went out as the result, so bote's own libro tools answered `{"ok":true}`. (3.8.0) A result's keys are read as they decode, as a client reads them: an escaped `isError` or `content` was missed, and the result built from what bote builds is on the request arena |
| `prompts/list` + `prompts/get` (capability advertised iff a `PromptRegistry` is present) | `dispatch` + `prompts` | ✅ — an unknown name, a missing required argument and non-object `arguments` are `-32602` (the last two since 3.4.5) |
| `resources/list` + `resources/read` (capability advertised iff a `ResourceRegistry` is present) | `dispatch` + `resources` | ✅ — an unserved URI is `-32002` with `data.uri`, as in the resources spec's example (3.4.5; was `-32602`) |
| `resources/templates/list` and reading through a template (optional in the spec) | `dispatch` + `resources` | ✅ (3.5.3) `dispatcher_register_resource_template`. RFC 6570 level 1: `{name}` is one non-empty value with no `/`, `?` or `#`, one to a segment. An exact resource wins, and the handler owns percent-decoding. Gated like `resources/*`, and an empty list with none registered |
| `resources/subscribe` / `unsubscribe` + `notifications/resources/updated` | `listen` (3.7.4) | 🟡 `notifications/resources/updated` arrives with `subscriptions/listen`'s `resourceSubscriptions`, on 2026-07-28 only. `resources/subscribe` is not planned, because 2026-07-28 removes it, and the legacy `initialize` advertises no `subscribe` |
| `resources` `listChanged`, `notifications/resources/list_changed` | `dispatch` (`TOPIC_RESOURCE_REGISTERED`) | ✅ (3.7.2) a resource or a template registered is a change: streamable's drain path carries it, and `listChanged` is advertised beside tools and prompts |
| `completion/complete` (capability advertised iff a completion handler is set) | `dispatch` (`dispatcher_set_completion`) | ✅ |
| `notifications/tools/list_changed` + `notifications/prompts/list_changed` + (3.7.2) `notifications/resources/list_changed` (buffered per session, drained on the client's next streamable `GET`; `listChanged` advertised only by a transport with a drain path) | `dispatch` (`dispatcher_set_notifications`) + `transport_streamable` (`strm_notify_sink`) | ✅ streamable (polled); live on a `subscriptions/listen` stream (3.7.2) |
| `logging/setLevel` + `notifications/message` | — | ⏳ not advertised — advertising it would promise messages bote cannot deliver; not planned, because 2026-07-28 removes it |
| Notifications produce no response | `dispatch` + `codec` | ✅ |
| Batch arrays — mixed req + notif return only req responses (spec ≤ 2025-03-26; still accepted for later versions) | `codec` | ✅ |

## Tool Definitions

| Spec Requirement | Module | Status |
|---|---|---|
| Tool name, description, inputSchema | `registry::ToolDef` | ✅ |
| Tool versioning + version negotiation in `tools/call` | `tool_def_with_version` + `registry_get_versioned` | ✅ |
| Tool deprecation message | `tool_def_with_deprecated` | ✅ |
| Tool annotations: `readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint` — serialized in `tools/list` | `ToolAnnotations` (+ `ann_read_only` / `ann_destructive` presets); `_emit_annotations` | ✅ |
| Tool profile tags + optional `{"profile":"<tag>"}` filter on `tools/list` (bote extension) | `tool_def_with_profiles` + `_build_tools_list_result` | ✅ |
| Tool names follow the spec's guidance (1–128 characters from `A-Z a-z 0-9 _ - .`) and inputSchema is an object schema ("MUST be a valid JSON Schema object") | `registry::registry_check_tool_def` | ✅ (3.4.8) — checked at every registration; a refused definition is not registered, an ERROR line says why, and the call returns a BoteError. It refuses the `Str`-for-cstr registrations the consumer audit found in t-ron and phylax. (3.8.7) `dispatcher_register_tool` too: through 3.8.6 it ran bote's own `project_tool` rule first, which required an underscore and refused `-` and `.`, so `t`, `get-weather` and `tool.v2` registered through the registry and were refused, silently, through the dispatcher. `validate_tool_name` is the spec's rule |

## Schema Validation

| Spec Requirement | Module | Status |
|---|---|---|
| Input schemas compiled at registration, so every tool's arguments are validated | `registry::registry_register` → `compiled_compile` | ✅ (3.4.8) — through 3.4.7 nothing compiled a schema, so only `required` was ever checked in production |
| JSON Schema subset — string, number, integer, boolean, array, object, ANY | `schema::SchemaTypeTag` | ✅ |
| `enum` of any value, and `const` | `schema::_sv_enum`, `_sv_eq` | ✅ (3.8.7) every value, whatever `type` says, by JSON Schema's equality (§4.2.2): numbers by value (`1`, `1.0`, `1e0`), strings by their characters however escaped, arrays in order, objects in any order. A string that is no text equals nothing. Through 3.8.6 `enum` was judged only for a string property and `const` not at all |
| `uniqueItems` | `schema::_sv_unique`, `_sv_hash` | ✅ (3.8.7) each item hashed by its value into a seeded table, so an array costs a look per item; a string's bytes that are no UTF-8 read as a client reads them, so two items a client finds equal are equal here. Numbers are compared exactly: a client holding them as doubles may find two equal past 15 significant digits or 2^53, or past a double's range (`1e400`, `1e-400`), that bote does not |
| Type arrays (`"type": ["string", "null"]`) | `schema::_sv_types_ok` | ✅ (3.8.7) a value of any type named; an integer named without `number` held to an argument's range, as `type: "integer"` is |
| `prefixItems`, and `items` beside it | `schema::_sv_array` | ✅ (3.8.8) item i by the i-th schema, `items` for the rest; draft-07's array `items` is warned of and not judged |
| `contains`, `minContains`, `maxContains` | `schema::_sv_array`, `_sv_matches` | ✅ (3.8.8) each item asked quietly whether it meets the schema, no message made for one that does not; one by default, 0 taking an array with none |
| `propertyNames` | `schema::_sv_object`, `_sv_matches` | ✅ (3.8.8) every key, each time it is written, as the string it decodes to |
| `dependentRequired`, `dependentSchemas` | `schema::_sv_object` | ✅ (3.8.8) a member present requires its names, or holds the object to its schema; draft-07's `dependencies` is warned of and not judged |
| `allOf`, `anyOf`, `oneOf`, `not`, `if` / `then` / `else` | `schema::_sv_comp`, `_sv_matches` | ✅ (3.8.9) on any value and on an inputSchema's root; `allOf` and the branch `if` chooses keep their violations, the rest ask quietly. The conformance suite's 2020-12 schema is judged whole |
| A bound on the work of judging | `schema::_sch_budget`, `_sv_charge` | ✅ (3.8.9) 100,000 values and 32 a byte of the document, quiet judgements counted; past it the value is refused as `more work to judge than its size allows`. (3.8.10) Each byte a keyword reads costs one more, and a batch's requests share one base |
| `$ref` within its document | `schema::_sch_ref_target`, `_sch_refs_why` | ✅ (3.8.10) `#`, a JSON Pointer anywhere, an anchor (`$anchor`, `$dynamicAnchor`, draft-07's `$id` fragment), behind the root's `$id` too, resolved at registration as Ajv resolves them; one into another document, to nothing, or around a cycle judging no part of the value refused there. A judgement deeper than 512 steps is refused as `nested too deep to judge` |
| Numeric bounds (`minimum`, `maximum`) for number + integer | `prop_min` / `prop_max` | ✅ — exponents read (3.4.8); a fractional value is compared as itself (3.4.15), and a fractional bound is truncated |
| `integer` per JSON Schema 2020-12 (a zero fractional part counts: `5.0`, `1e2`) | `schema::_sch_num_parse` | ✅ (3.4.8; any `.` was refused before) |
| Required field check | `compiled_required` | ✅ |
| Recursive nested object validation | `_sv_object` | ✅ (3.8.5) in one walk of its members, each looked up once, and judged where it lies |
| Array `items` typing | `prop_items` | ✅ |
| `type` read as it decodes, and `null` | `schema::parse_property` | ✅ (3.8.1) `"obj\u0065ct"` is an object; `null` is judged. Through 3.8.0 both judged nothing |
| `required` names and string `enum` values compared as they decode | `schema::_sch_str_dec_a` | ✅ (3.8.1) on the schema's side and the value's. Through 3.8.0 `"required":["\u00e9"]` was never met |
| Multi-violation reporting (collect all, don't fail-fast) | `compiled_validate` returns vec | ✅ |
| Permissive on extra (unknown) fields, unless `additionalProperties` says otherwise | `_sv_object` | ✅ (3.8.5) `additionalProperties` false refuses a member no property names, and a schema judges it; beside `patternProperties` it is not judged, and registration warns |
| `minLength` / `maxLength` (code points), `minItems` / `maxItems`, `minProperties` / `maxProperties`, `exclusiveMinimum` / `exclusiveMaximum` | `schema::_sv_string`, `_sv_array`, `_sv_object`, `_sv_number` | ✅ (3.8.5); each keyword judges the values of its kind whatever `type` says, as JSON Schema has it. (3.8.6) Bytes that are no UTF-8 are counted as a client's decoder reads them, one U+FFFD for each maximal part (WHATWG Encoding, Unicode §3.9): 3.8.5 counted a byte from 0x80 to 0xBF that starts no sequence as none, so 60,000 of them met `maxLength: 4`. A value bote cannot judge by (`maxLength: "3"`, `exclusiveMinimum: true`, `additionalProperties: "no"`) judges nothing, and registration WARNs of it |
| Schema `false` allows nothing | `SCH_NEVER` | ✅ (3.4.15; it allowed anything) |
| `pattern` | `regex::bote_rx_test_a`, `schema::_sv_string` | ✅ (3.8.11) ECMA-262 under the `u` flag, as the official client's Ajv compiles it: code points, anywhere in the string, in time linear in it, each step charged to the budget. Lookaround, backreferences, `\p{...}` and modifiers are declined (the pattern judges nothing, with a WARN). Held to JavaScript's `RegExp` by `scripts/regex-diff.sh` |
| `multipleOf` | `schema::_sch_is_multiple` | ✅ (3.8.11) exactly, as decimals of any length: 19.99 is a multiple of 0.01. The official client divides as doubles and does not find it one, so registration warns of a fractional `multipleOf`. 0 refuses every number and a negative one is judged by its size, as the client judges them |
| Keywords outside the subset (`$dynamicRef`, `$recursiveRef`, `format`, `patternProperties`, `unevaluated*`, and the rest of `BOTE_UNENFORCED`) | `registry::_reg_warn_unenforced`, (3.8.0) `_reg_warn_output` | 🟡 advertised in `tools/list`, not enforced; a WARN at registration names each (3.4.15), on an outputSchema's root and its properties too (3.8.0). Roadmap 3.8.x |
| An outputSchema judges a result's `structuredContent`, any JSON value | `schema::_sch_check_value_a` | ✅ (3.8.0) the same subset, from the schema's root: each violation named from `structuredContent` (`structuredContent.temperature: expected number`, `structuredContent[0].id: expected string`). (3.8.1) A key named twice in any object of it is refused, as bote would judge the first and a client reads the last; a key of any length is read as it decodes; an integer is not bounded as an argument is. An outputSchema that repeats a key, has one holding a NUL, or names a dialect other than 2020-12, 2019-09, draft-07 or draft-06 is refused at registration |

## Session Management

| Spec Requirement | Module | Status |
|---|---|---|
| `MCP-Protocol-Version` header parsing + validation | `session::bote_protocol_known` + `http_era::bote_http_check_version` | ✅ — absent is taken as `2025-03-26`, as the spec allows, unless `require_protocol` is set. Through 3.5.0 `bote streamable` refused a POST without it, `initialize` included (3.5.1) |
| `MCP-Session-Id` — server-issued on `initialize`, validated on subsequent | `session::SessionStore` + per-transport `_check_session` | ✅ HTTP / streamable / bridge |
| Session creation on initialize (auto, returned in response header) | `session_store_create` + handler hook | ✅ |
| Session timeout + pruning (configurable; default 1h) | `session_store_validate` + `session_store_prune_expired` | ✅ (3.4.15) — through 3.4.14 nothing checked a session's age and nothing called the pruner, so a session never expired. The store now holds at most `BOTE_SESSION_MAX` (4096) and answers 503 when full of live sessions; a session is minted only after `initialize` succeeds |
| Random 128-bit SID, 32-hex encoded, from the kernel CSPRNG via `random_bytes()` (`getrandom(2)`); the process **refuses to mint a session ID** if that fails rather than fall back to anything guessable | `_gen_session_id` | ✅ (3.2.0; was `/dev/urandom` by raw syscall) |

## Security

| Spec Requirement | Module | Status |
|---|---|---|
| Origin validated on every HTTP-family transport (DNS rebinding protection — 403 on rejection); `BOTE_ALLOWED_ORIGINS` sets the list | `session::bote_origin_ok` + per-transport `_check_origin` | ✅ (3.5.2) with no list the server's own loopback address passes, an Origin of `http://` + the request's `Host` on `localhost`, `127.0.0.1` or `[::1]`, which the conformance suite requires and which no rebinding page can send. (3.4.15) — a present Origin must be listed, and with no list every present one is refused. Through 3.4.14 no list meant no check, the binary had no way to set one, and `bote bridge` allowed `*` |
| Wildcard `*` and exact-match origins | same | ✅ |
| Strict mode (empty allow-list rejects all) | same | ✅ |
| CORS preflight (OPTIONS / + 3 ACA-* headers) | `bridge::_bridge_cors_headers` | ✅ |
| **Bearer-token middleware (RFC 6750)** — opt-in fn-pointer + ctx validator on every HTTP-family transport | `auth::auth_bearer_check` + per-transport wiring | ✅ (1.9.0) |
| **`WWW-Authenticate: Bearer realm="mcp"` on 401**, with no metadata configured | `auth::auth_send_challenge` | ✅ |
| **Built-in validators**: `auth_validator_allow_all` (dev), `auth_validator_allowlist` (vec membership) | `auth` | ✅ |
| **`BOTE_BEARER_TOKENS` env var** wires an allowlist validator across all four HTTP-family transports at startup | `main::_bote_bearer_from_env` | ✅ (1.9.1) |
| **HTTP body-length clamp** — `clen = min(clen, n - bo)` so a lying Content-Length can't make `memcpy` read past the request buffer | `transport_http`, `transport_streamable`, `bridge` | ✅ (1.5.1) |
| A field value holding a NUL is refused or replaced (RFC 9110 §5.5) | `auth::bote_http_head_nul`, first in every HTTP-family handler | ✅ (3.6.5) a 400 on `http`, `streamable`, `bridge` and `ws`, before a header is read |
| **SSRF guard for outbound URL fetches** — IPv4 + IPv6 blocklists for loopback / private / link-local / cloud-metadata | `host::ssrf_check` | ✅ (1.8.0 / 1.9.1) |
| **JWT verifier** (RFC 7519 / 7515) — `alg` read as an exact JSON field (not a substring scan), constant-time HMAC compare, each segment strict base64url on the request arena, `exp` and `nbf` enforced *after* the signature verifies with no leeway, a malformed one rejects, an absent one accepts unless an audience is set, which requires `exp`; `auth_validator_jwt_hs256` plugs into the bearer middleware, and the binary uses it with `BOTE_JWT_SECRET` | `jwt::jwt_verify_hs256` | ✅ (2.2.0; `exp` + exact `alg` at 3.2.0; `nbf`, `aud` and `iss` at 3.6.1) |
| **RFC 7636 PKCE-S256** — `pkce_code_verifier` (CSPRNG) + `pkce_code_challenge_s256` | `pkce` | ✅ (2.3.0; ships in `dist/bote.cyr` since 3.2.0) |
| **Pluggable sandbox runner** — fn-pointer + ctx adapter for tool handlers (kavach-shaped), noop default, error envelope on a null runner | `sandbox::sandbox_run` | ✅ (2.1.0; ships in `dist/bote.cyr` since **3.3.13** — before that it was in `src/` only and reached no consumer). Not in `dist/bote-core.cyr` |
| Asymmetric JWT (RS256, ES256), keys from a JWK Set | `jwt::jwt_secret_from_jwks` | ✅ (3.6.2) `BOTE_JWKS_FILE`. A key has one algorithm (RFC 8725 §3.1), `kid` picks among the operator's keys, `crit` is refused, and `jwk`, `jku`, `x5u` and `x5c` are never read. Checked against RFC 7515 A.2 and A.3 and tokens Node signed. (3.6.6) Fetched from the authorization server's `jwks_uri` with `BOTE_JWKS_URL`: at startup, every `BOTE_JWKS_REFRESH` seconds, and for a `kid` the set does not hold, never more often than a 60 s cooldown, backing off while the server is down and keeping the last good set; a set that changes forgets every verified token. `https://` (system roots or `BOTE_JWKS_CA`) or `http://` to loopback, no redirect, in a child with a 5 s deadline. The official client's RS256 flows run on keys fetched this way |
| OAuth 2.0 Protected Resource Metadata (RFC 9728): a document at `/.well-known/oauth-protected-resource` + the resource's path, with `resource` and at least one authorization server, readable with no token | `oauth_metadata` | ✅ (3.6.0) on `http`, `streamable` and `bridge`, from `bote_prm_new` or `BOTE_RESOURCE_URL` + `BOTE_AUTH_SERVERS` + `BOTE_AUTH_SCOPES`. `resource` is the operator's and never comes from a request (RFC 9728 §7.3). `scopes_supported`, `bearer_methods_supported` and `resource_name` go with it; `offline_access` is refused |
| `resource_metadata` and `scope` in the `WWW-Authenticate` of a 401, and `error="invalid_token"` when a token was refused | `auth::auth_bearer_check_c` | ✅ (3.6.0), which meets both of MCP's discovery mechanisms at once. The official TypeScript client completes its OAuth flow on this, in both eras |
| A server validates that a token was issued for it (RFC 8707, MCP Token Handling) | `jwt::auth_validator_jwt_hs256` | ✅ (3.6.1) with an audience set on the JwtSecret, `aud` must name it, as a string or in an array, compared after JSON decoding; `exp` must be present; `iss` must be one of the issuers set. With none set, a token that names an audience is refused, as RFC 7519 §4.1.3 asks. `BOTE_JWT_SECRET` sets this from `BOTE_RESOURCE_URL` and `BOTE_AUTH_SERVERS`. Another validator must do this itself: bote cannot see into one |
| `403` with `error="insufficient_scope"`, `scope` naming every scope the operation needs, and `resource_metadata` (RFC 6750 §3.1, MCP Scope Challenge Handling) | `oauth_metadata::bote_http_scope_gate`, `auth::auth_send_insufficient_scope` | ✅ (3.6.3) for a lone `tools/call` on `http`, `streamable` and `bridge`, on both eras, and (3.6.6) for `prompts/get`, `resources/read` and a `completion/complete` naming a prompt or a template. The official TypeScript client steps up on it and retries, on HS256, RS256 and ES256. `error_description` is optional and not sent |
| A scope a tool requires is enforced wherever the tool can be reached | `dispatch::dispatcher_scopes_missing` | ✅ (3.6.3) in the dispatcher, so a batch element and a WebSocket frame are refused too, with `-31403`, a code outside the range JSON-RPC reserves. (3.6.6) A prompt, a resource and a template declare scopes too (`*_with_required_scopes`), judged by what would serve the request, and a completion by the prompt or template its `ref` names |
| A server accounts for scope hierarchies | `jwt::jwt_scope_rule`, `BOTE_SCOPE_IMPLIES` | ✅ (3.6.3) rules chain, each followed once, so a cycle ends. A validator an embedder supplies accounts for its own |

## Transports

| Transport | Module | Status |
|---|---|---|
| stdio (line-oriented) | `transport_stdio` | ✅ |
| HTTP/1.1 (`POST <endpoint>`) with full middleware | `transport_http` | ✅ |
| Unix domain socket (line-oriented) | `transport_unix` | ✅ |
| TypeScript bridge (POST `/`, GET `/health`, OPTIONS, MCP envelope wrap) | `bridge` | ✅ |
| **Streamable HTTP (MCP 2025-11-25)** — single endpoint POST + GET SSE, `Last-Event-ID` resumption, bounded resumption buffer (default 1000), `retry:` hint | `transport_streamable` | ✅ (1.4.0) |
| **WebSocket (RFC 6455)** — handshake (`Sec-WebSocket-Accept = base64(sha1(key + magic))`), masked-client / unmasked-server frames, ping/pong/close handled transparently | `transport_ws` + `lib/ws_server.cyr` | ✅ (1.5.0). (3.6.6) A connection's token is judged again, at most once a second (`BOTE_WS_RECHECK_MS`), and one that no longer passes ends it with 1008 |

## Content Blocks (MCP 2025-11-25)

| Block type | Constructor | Status |
|---|---|---|
| `text` | `content_text(text)` | ✅ (1.7.0) |
| `image` (base64 inline) | `content_image(b64, mime)` | ✅ (1.7.0) |
| `audio` (base64 inline) | `content_audio(b64, mime)` | ✅ (1.7.0) |
| `resource` (embedded text body) | `content_resource(uri, mime, text)` | ✅ (1.7.0) |
| `resource` (embedded binary `blob`) | `content_resource_blob(uri, mime, b64)` | ✅ (1.9.1) |
| `resource_link` (reference) | `content_resource_link(uri, name, mime)` | ✅ (1.7.0) |
| Envelope: `{"content":[...]}` | `content_array(blocks)` / `content_single(block)` / `content_text_response(text)` | ✅ |
| Tool-error envelope: `{"content":[...],"isError":true}` | `content_array_error(blocks)` | ✅ |
| Block-level annotations (`audience`, `priority`) | `content_with_annotations`; preserved through `bridge::wrap_tool_result` | ✅ (1.9.6; propagation 2.7.0) |

## Built-in tools

### `libro_*` (1.6.0)

| Tool | Purpose | Module |
|---|---|---|
| `libro_query` | Filter / paginate chain entries by source / agent / severity / time | `libro_tools::libro_tool_query` |
| `libro_verify` | Hash-link integrity check | `libro_tool_verify` |
| `libro_export` | Every entry as a JSON array | `libro_tool_export` |
| `libro_proof` | Merkle inclusion proof for entry at index | `libro_tool_proof` |
| `libro_retention` | Apply policy (`keep_count` / `keep_duration` / `keep_after` / `pci_dss` / `hipaa` / `sox`) | `libro_tool_retention` |

Registered by default in `main.cyr` against an empty chain at startup;
clients see them in `tools/list` immediately.

### `fs_*` (2.8.0) and `web_*` (3.1.0)

| Tool | Purpose | Guard |
|---|---|---|
| `fs_write` / `fs_read` / `fs_mkdir` | File operations | Root-confined to `BOTE_FS_ROOT` (default `.`); absolute and `..` paths refused |
| `web_fetch` | GET a URL and strip HTML to readable text | `http` / `https` only, the SSRF guard on the URL and the resolved address, 64 KiB cap, control bytes dropped; in a child killed at `BOTE_WEB_TIMEOUT_MS` (3.6.5) |
| `web_search` | Query a SearXNG JSON endpoint | `BOTE_SEARXNG_URL` — self-hostable, no third-party key; in a child killed at `BOTE_WEB_TIMEOUT_MS` (3.6.5) |

All three families are opt-in via `*_tools_register()` for library consumers and
registered by default in the binaries.

## Bridge (MCP envelope contract)

| Spec Requirement | Module | Status |
|---|---|---|
| `tools/call` success → wrap result in `{"content":[{"type":"text","text":...}]}` | `wrap_tool_result` | ✅ (passthrough if already shaped) |
| `tools/call` error → `result: {"content":[...],"isError":true}` | `wrap_error_result` + `_bridge_process_single` | ✅ |
| Other methods (`initialize`, `tools/list`) — pass through unchanged | `bridge_process_message` | ✅ |
| `GET /health` returns `200 ok` | `_bridge_send_health` | ✅ |
| Pre-built `content` envelope from typed-block constructors → passed through | verified in `bote_content.tcyr` | ✅ |

## Discovery (data layer)

| Spec Requirement | Module | Status |
|---|---|---|
| `ToolAnnouncement` JSON envelope with `node_id` + `tools[]` | `announcement_to_json` | ✅ |
| `DiscoveryService` with pluggable publish function pointer | `discovery_new` + `discovery_publish_fp` | ✅ |
| `DiscoveryReceiver` queue with `try_recv` | `discovery_receiver_*` | ✅ |
| Wired to majra pubsub | `events_majra_publish` adapter | ✅ (1.2.0) |

## Streaming primitives (data layer)

| Spec Requirement | Module | Status |
|---|---|---|
| `ProgressUpdate` (progress / total / message) | `progress_update_*` | ✅ |
| `CancellationToken` (clone-shared flag) | `bote_cancel_token_*` | ✅ the data type. A request's cancellation is `bote_cancelled()` (3.7.5), which asks the connection: a flag nothing sets would never change |
| `notifications/progress` JSON builder | `progress_notification` | ✅ |
| Progress: a request's `_meta.progressToken`; `notifications/progress` with the token, `progress`, `total` and `message`; "The `progress` value MUST increase"; "Progress notifications MUST stop after completion"; on Streamable HTTP only on the request's own SSE response, before the response, which SHOULD end the stream, with `X-Accel-Buffering: no` | `codec::bote_progress`, `http_era` | ✅ (3.7.6) a handler reports with `bote_progress`; a `progress` no greater than the last is refused, nothing goes for a request with no token, one cancelled, or one already answered. On HTTP the POST's response turns into `text/event-stream` at the first report, on both eras on `http` and `streamable` and on 2026-07-28 on the bridge; a client whose `Accept` takes no `text/event-stream` hears none. On stdio, unix and WebSocket a line or a frame each. Never on a listen stream. `progress` and `total` are integers here; the spec allows numbers |
| Threaded streaming dispatch | — | ⏳ bote-side work (roadmap 3.7.x). The cyrius primitives this once waited on — `lib/thread.cyr` MPSC, `lib/async.cyr`, `thread_local_alloc`, `arena_*` — are complete and pinned |
| Cancelling a request in flight | `bote_cancelled` | ✅ (3.7.5), see Core Protocol |
| Server → client push (`tools` / `prompts` / `resources` `list_changed`) | `transport_streamable` (`SessionOutbound`); `listen` | ✅ polled — buffered per session at produce time, drained on the next `GET` or piggybacked as SSE on a `POST`; advertised only by a transport with a drain path (3.0.0). ✅ live on a `subscriptions/listen` stream (3.7.2) |
| Held-open `GET` stream with live push | `transport_streamable` | 🟡 the stream opens and replays the resumption buffer; *live* delivery waits on threaded dispatch |

## Audit / Events Sinks

| Spec Requirement | Module | Status |
|---|---|---|
| `AuditSink` (fn-pointer + ctx) — sinks-noop default = zero overhead | `audit::AuditSink` + `dispatcher_set_audit` | ✅ (1.1.0) |
| `EventSink` (fn-pointer + ctx) | `events::EventSink` + `dispatcher_set_events` | ✅ (1.1.0) |
| Topic constants (10 well-known) | `events::TOPIC_*` | ✅ |
| Dispatcher emits audit + events on tools/call, register, dereg, deprecate | `dispatch` | ✅ |
| `LibroAudit` adapter | `audit_libro::libro_audit_log` | ✅ (1.2.0) |
| `MajraEvents` adapter | `events_majra::majra_events_publish` | ✅ (1.2.0) |

## Host / SSRF

| Spec Requirement | Module | Status |
|---|---|---|
| `HostRegistry` — name → `{url, headers, capabilities}` map | `host::HostRegistry` | ✅ (1.8.0) |
| Capability allowlist (`host_entry_allows`); fail-open when unset | same | ✅ |
| `ssrf_check(url)` — IPv4 (loopback / private / link-local / metadata / unspec / multicast; since 3.4.7 also 100.64.0.0/10 shared, 192.0.0.0/24 and 240.0.0.0/4 reserved) | `host::_ssrf_classify_v4` | ✅ (1.8.0; 3.4.7) — 198.18.0.0/15 and the documentation ranges are allowed on purpose (fake-IP DNS proxies answer inside 198.18/15; nothing routes the documentation ranges) |
| IPv6 literals parsed to bytes (RFC 4291 §2.2, zone id set aside) and classified on them: `::`, `::1`, IPv4-mapped / -compatible, NAT64 (`64:ff9b::/96`, `64:ff9b:1::/48`), 6to4 carrying an internal IPv4, `fe80::/10`, `fec0::/10`, `fc00::/7`, `ff00::/8` | `host::_ssrf_parse_ipv6` + `ssrf_classify_ipv6_bytes` | ✅ (1.9.1; bytes since 3.4.7) — a literal that does not parse is refused. Through 3.4.6 a text matcher missed spellings such as `0:0:0:0:0:0:0:1` and `fe9f::1` and passed anything it did not recognise. The parser is checked against cyrius's own (`lib/tls_hostid.cyr`) in `tests/bote_host.tcyr` and `fuzz/ssrf_ipv6.fcyr` |
| Hostname blocklist (`localhost`, `metadata.google.internal`, `metadata`) — case-insensitive | `host::_ssrf_classify_hostname` | ✅ |
| `user:pass@` userinfo stripping before classification | `host::_ssrf_extract_host` | ✅ |
| Scheme gate — only `http://` / `https://` | same | ✅ |
| DNS resolution (catch `127.0.0.1.nip.io` style bypasses) — the address a name resolves to is classified where sandhi connects | `web_tools::_web_ssrf_resolve` via `sandhi_client_set_resolver` | ✅ (3.4.4) — `web_fetch` only; `web_search` keeps stock resolution for the operator's own `BOTE_SEARXNG_URL`. The hook uses the same classifiers as the URL check, so 3.4.7's ranges apply to resolved addresses too |

---

## Test Coverage (bote 3.8.2)

| Scope | Count | Source |
|---|---|---|
| **Protocol conformance by spec clause** — JSON-RPC 2.0 §4 / §5 / §4.1 / §5.1 / §6 (incl. the spec's own examples), MCP initialize + all four revisions, ping, cancellation, capability gating, pagination cursors, tools/list, the tools/call protocol-vs-execution error split, prompts, resources, completion, control bytes in a tool result, every handler-result shape, typed arguments; the 2026-07-28 stateless path; `-31403` on both eras; every response checked to be valid JSON and a §5 Response object | **218** | `tests/conformance.tcyr` (3.4.5; 3.5.0) |
| Core — error / protocol / jsonx / registry / prompts / resources / completion / dispatch / codec / schema / stream / session / HTTP helpers / discovery / bridge / events / audit / the per-request DEBUG line, the string escaper, registration checks, numbers per 2020-12, required scopes, the request arena and its bound | **969** | `tests/bote.tcyr` |
| Bearer + allowlist + JWT + PKCE validators, the 403's challenge, the head check | **91** | `tests/bote_auth.tcyr` |
| Cancelling a request in flight: the check, the codec, stdio, unix, a WebSocket, HTTP on both eras, a fetch's child | **141** | `tests/bote_cancel.tcyr` (3.7.5) |
| Content blocks + annotations | **25** | `tests/bote_content.tcyr` |
| An outbound request made in a child: the record, the cap, the deadline, a real GET | **62** | `tests/bote_fetch.tcyr` |
| `fs_tools` path safety + root confinement + control bytes as valid JSON + schemas and annotations; decoded names, truncation, fs_mkdir | **63** | `tests/bote_fs_tools.tcyr` |
| HostRegistry + IPv4 / IPv6 SSRF (URL and resolved address; each 3.4.7 range named) + the IPv6 parser against cyrius's + hot-reload | **207** | `tests/bote_host.tcyr` |
| MCP 2026-07-28 on HTTP, each clause on the real handlers over a socketpair; Protected Resource Metadata; the 403 for a missing scope | **1833** | `tests/bote_http_era.tcyr` |
| A `jwks_uri`: what it may be, and real fetches from a child | **49** | `tests/bote_jwks.tcyr` |
| JWT — HS256, RS256, ES256, JWK Sets, exact `alg`, `crit`, `exp`, `nbf`, `aud`, `iss`, algorithm confusion, scopes, tokens already verified, keys from a source, no heap (mutation-proven) | **491** | `tests/bote_jwt.tcyr` |
| `libro_tools` incl. tampered-chain decode + populated-chain proof + CallToolResults, the retention guard, RFC 3339 timestamps, schemas, integers as validated, audit identity | **73** | `tests/bote_libro_tools.tcyr` |
| `subscriptions/listen`: the filter, and real streams on the three HTTP handlers | **224** | `tests/bote_listen.tcyr` (3.7.2) |
| `subscriptions/listen` where the connection is the stream: stdio, unix, WebSocket | **192** | `tests/bote_listen_line.tcyr` (3.7.3) |
| Titles, icons, serverInfo's details and instructions: the checks, every list's bytes, the heap | **107** | `tests/bote_display.tcyr` (3.8.1) |
| Operational logging over sakshi | **50** | `tests/bote_log.tcyr` |
| A tool's outputSchema: registration, the 2025-11-25 form, `tools/list`, each result judged and shaped on both revisions, the audit, the heap | **240** | `tests/bote_output.tcyr` (3.8.0) |
| Multi round-trip requests: what a handler reads and returns, `-32021`, each refusal, the sealed state and its sealer, every method and revision, the heap | **102** | `tests/bote_input.tcyr` (3.8.2) |
| RFC 7636 PKCE-S256 | **17** | `tests/bote_pkce.tcyr` |
| Progress on a request's own stream: the writer, stdio, unix, WebSocket, HTTP's SSE response | **110** | `tests/bote_progress.tcyr` (3.7.6) |
| Sandbox runner adapter | **13** | `tests/bote_sandbox.tcyr` |
| Streamable HTTP — event ids, resumption, per-session outbound, drain selection, a full store's refusal, the piggyback's `Accept` | **64** | `tests/bote_streamable.tcyr` |
| bote on worker threads — each case driven from several threads at once (3.7.0); bote's own tools and registering while serving (3.7.1) | **81** | `tests/bote_threads.tcyr` |
| Unix transport — sockaddr, accept-error policy, backoff, deadlines, closed-peer write, overlong lines, socket-path refusals | **76** | `tests/bote_transport_unix.tcyr` |
| `web_tools` — scheme guard, SSRF guard (URL and resolved address), HTML stripper, entities, schemas and annotations, outbound deadline, a request in a child | **88** | `tests/bote_web_tools.tcyr` |
| WebSocket config + wire-up + the send-deadline drop + the upgrade's version check + scopes per frame + the token judged again | **94** | `tests/bote_ws.tcyr` |
| **Total assertions** | **5680** | + `bote_core_only_smoke.tcyr` (drift guard over `dist/bote-core.cyr`, exit-code driven); all of them also run under `qemu-aarch64` |
| Hot-path benchmarks | **44** | `tests/bote.bcyr` |
| Fuzz harnesses | **15** | `fuzz/*.fcyr` (in CI since 3.3.8) |
| End-to-end transport round trips | 6 (stdio, HTTP, Unix, bridge, streamable, WS) | CI's "Transport smoke" step plus a per-release stdio probe on the built binary |

The Rust archive's conformance suite (`tests/conformance.rs`, tag `0.92.0`, 44 scenarios) is
ported as `tests/conformance.tcyr` (3.4.5). Its scenarios are there, with the JSON-RPC and MCP
requirements it never covered, and where the Rust suite and the spec disagree (an unknown tool's
code, a schema violation's shape) the spec wins. The bridge's error-envelope scenario lives in
`tests/bote.tcyr`, which links the bridge.

---

*Audit method: manual comparison against [MCP spec 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25), with every method row probed against the built binary over stdio (the 3.3.12 sweep is how the `ping`, `2025-06-18` and `resources/templates/list` rows were found). For the explicit security-property table see [SECURITY.md](../SECURITY.md).*
