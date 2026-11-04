# bote

**MCP core service in Cyrius.**

JSON-RPC 2.0 protocol, tool registry, schema validation, dispatch, six
transports, bearer-token auth, libro audit tools, filesystem + web
tools, prompts / resources / completion capabilities, typed content
blocks, host registry with SSRF guard — in a self-hosting Cyrius project.
Eliminates per-app MCP implementations across the AGNOS ecosystem.

> **Name**: Bote (German) — messenger. The messenger between agents and tools.

[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)

> bote was originally a Rust crate. Ported to Cyrius for v1.0.0; the Rust
> archive was retired in v1.0.1 (the snapshot lives at git tag `0.92.0`
> for reference). See
> [docs/cyrius-feedback.md](docs/cyrius-feedback.md) for language issues
> discovered along the way and
> [docs/benchmarks-rust-v-cyrius.md](docs/benchmarks-rust-v-cyrius.md)
> for the side-by-side performance comparison.

---

## What it does

bote is the **MCP protocol layer** — handles JSON-RPC 2.0 wire format,
tool registration, schema validation, and call dispatch so individual
apps don't each reimplement the same protocol.

| Capability | Status |
|---|---|
| **JSON-RPC 2.0** — requests, responses, notifications, batch arrays | ✅ |
| **Tool registry** — register / discover / version / deprecate, insertion-order list | ✅ |
| **Compiled schema validation** — type, enum, bounds, nested objects, multi-violation | ✅ |
| **Output schemas** — `outputSchema` listed, and every result's `structuredContent` judged by it, in the shape each revision allows | ✅ (3.8.0) |
| **Titles, icons and instructions** — on tools, prompts, resources, templates and serverInfo, each checked once and sent as checked | ✅ (3.8.1) |
| **Dispatch** — `initialize`, `tools/list`, `tools/call`; `project_tool` name validation | ✅ |
| **Prompts / resources / completion** — `prompts/*`, `resources/*`, `completion/complete`; polled `list_changed` push | ✅ |
| **Stdio transport** — line-oriented JSON-RPC | ✅ |
| **HTTP/1.1 transport** — own server, Origin allow-list, MCP-Protocol-Version, MCP-Session-Id | ✅ |
| **Unix domain socket transport** | ✅ |
| **Bridge transport** — TS clients, CORS, MCP envelope wrap | ✅ |
| **Streamable HTTP transport (MCP 2025-11-25)** — POST + GET SSE, Last-Event-ID resumption | ✅ |
| **WebSocket transport (RFC 6455)** — full handshake, masked client / unmasked server frames | ✅ |
| **Bearer-token middleware (RFC 6750)** — opt-in per transport, fn-pointer + ctx validator | ✅ |
| **Sessions** — create / validate / prune; auto-create on `initialize` | ✅ |
| **`libro_tools`** — 5 built-in MCP tools (query / verify / export / proof / retention) over a libro audit chain | ✅ |
| **`fs_tools`** — 3 built-in MCP tools (`fs_write` / `fs_read` / `fs_mkdir`), root-confined via `BOTE_FS_ROOT` | ✅ |
| **`web_tools`** — `web_fetch` (HTML→text, size-capped) + `web_search` (SearXNG via `BOTE_SEARXNG_URL`) | ✅ |
| **Typed MCP content blocks** — text / image / audio / resource / resource_link / blob | ✅ |
| **`HostRegistry` + SSRF guard** — IPv4 + IPv6 blocklists for loopback, private, link-local, cloud-metadata | ✅ |
| **Audit / events sinks** — fn-pointer + ctx adapters, libro + majra wired | ✅ |
| **Streaming primitives** — `ProgressUpdate`, `CancellationToken`, progress notifications | data layer ✅ / streamable on worker threads ✅ (3.7.0) / `subscriptions/listen` ✅ (3.7.2–3.7.4) / cancellation ✅ (3.7.5) / progress on a request's stream ⏳ (roadmap 3.7.6) |
| **OAuth 2.1 substrate** — bearer (RFC 6750), Protected Resource Metadata (RFC 9728), JWT access tokens, HS256, RS256 and ES256, with audience, issuer, `exp` and `nbf` checks (RFC 8707, RFC 7519), PKCE-S256 helpers | ✅ |
| **Sandbox runner** — fn-pointer + ctx adapter (kavach-shaped), noop default | ✅ in `dist/bote.cyr` (3.3.13); not in the core bundle |

---

## Quick start

### Build

```sh
cyrius deps            # resolve [deps.*] → lib/ (gitignored)
./scripts/build-all.sh # build/bote (one binary; bote-streamable / bote-ws link to it)
```

Static ELF binary, no libc dependency.

### Run

```sh
./build/bote                    # stdio transport (default; also `bote stdio`)
./build/bote http [port]        # HTTP on 127.0.0.1:port (default 8390)
./build/bote unix <path>        # Unix domain socket
./build/bote bridge [port]      # TS bridge with CORS (default 8391)
./build/bote streamable [port]  # Streamable HTTP / SSE (default 8392)
./build/bote ws [port]          # WebSocket (default 8393)
```

One binary since 3.4.2. Through 3.4.1 streamable and WebSocket were separate
binaries; `bote-streamable [port]` and `bote-ws [port]` still work through the
links `build-all.sh` makes. stdio, http, unix and bridge register `bote_echo`,
the five `libro_*` tools, the three `fs_*` tools and the two `web_*` tools;
streamable and ws register the echo / greeting / info references only, as their
separate binaries did. `./scripts/transport-smoke.sh` starts every mode and
checks each one answers.

### Bearer auth

```sh
BOTE_BEARER_TOKENS="tok-a,tok-b" ./build/bote http 8390
# Now every POST /mcp requires Authorization: Bearer tok-a (or tok-b);
# missing or wrong → 401 with WWW-Authenticate: Bearer realm="mcp"
```

Stdio + Unix sockets are local-only and skip the bearer check. `BOTE_BEARER_TOKENS` set with no
token in it refuses to start.

To have clients find their way to a token, name this server and the servers that issue its tokens:

```sh
BOTE_RESOURCE_URL="https://mcp.example.com/mcp" \
BOTE_AUTH_SERVERS="https://auth.example.com" \
BOTE_AUTH_SCOPES="files:read,files:write" \
BOTE_BEARER_TOKENS="tok-a" ./build/bote http 8390
# GET /.well-known/oauth-protected-resource/mcp → the RFC 9728 document, with no token
# no token → 401 with WWW-Authenticate: Bearer resource_metadata="https://mcp.example.com/
#   .well-known/oauth-protected-resource/mcp", scope="files:read files:write"
```

`BOTE_RESOURCE_URL` is the URL a client uses, exactly: a client refuses a document that names any
other, so bote never builds it from a request. It must be https, or http on loopback. Works on
`http`, `streamable` and `bridge`. Anything that cannot be published stops the server at startup.
bote is the resource server: it names the authorization servers and issues nothing, and the
validator is still what decides whether a token is good.

For tokens from an authorization server that signs with HS256, give bote the key instead of a
token list:

```sh
BOTE_JWT_SECRET="at-least-32-bytes-of-shared-secret" \
BOTE_RESOURCE_URL="https://mcp.example.com/mcp" \
BOTE_AUTH_SERVERS="https://auth.example.com" ./build/bote http 8390
# A token must be signed with that key, name BOTE_RESOURCE_URL in `aud`, name one of
# BOTE_AUTH_SERVERS in `iss`, and hold an `exp` still in the future; `nbf` is honoured.
# Anything else → 401 with error="invalid_token"
```

A token issued for another server is refused even when its signature is good, which is what MCP
asks of a resource server. A key under 32 bytes, a key with no `BOTE_RESOURCE_URL`, or a key beside
`BOTE_BEARER_TOKENS` stops the server at startup.

Most authorization servers sign with a private key. Save their JWK Set, the document at their
`jwks_uri`, and point bote at it:

```sh
BOTE_JWKS_FILE="/etc/bote/jwks.json" \
BOTE_RESOURCE_URL="https://mcp.example.com/mcp" \
BOTE_AUTH_SERVERS="https://auth.example.com" ./build/bote http 8390
# RS256 (2048 to 4096 bits, e = 65537) and ES256 (P-256), with the same claim checks
```

A token's `kid` picks the key, and with no `kid` there must be exactly one key of its `alg`. Keys
bote cannot use are skipped with a WARN. A set with a private key in it, or with nothing usable, stops
the server, and so does giving it `BOTE_JWKS_FILE` and `BOTE_JWT_SECRET` together. A file is read at
startup, so rotating it means a restart.

Or let bote fetch the set from the authorization server's `jwks_uri`, and follow its rotations:

```sh
BOTE_JWKS_URL="https://auth.example.com/.well-known/jwks.json" BOTE_RESOURCE_URL="https://mcp.example.com/mcp" BOTE_AUTH_SERVERS="https://auth.example.com" ./build/bote http 8390
# BOTE_JWKS_REFRESH=600   seconds between refreshes
# BOTE_JWKS_CA=/etc/bote/as-ca.pem   trust roots for a private CA, in place of the system's
```

The set is fetched at startup, and bote does not start if that fails. After that it is fetched again
every `BOTE_JWKS_REFRESH` seconds, and as soon as a token names a key the set does not hold, but never more
often than once a minute. If the authorization server is down, the last set keeps serving, and bote asks
again less often until it answers. A key that leaves the set stops passing tokens at the next refresh,
remembered ones included. The URL must be `https://`, or `http://` to a loopback address.

A signature check is the slowest thing a request does. ES256 costs about 10.5 ms and RS256 about 1.3 ms,
for a forged token as much as a good one, on one thread. On `bote streamable` it holds one worker, not the
server; on the other transports it holds the server. Put a rate limit ahead of a server that takes tokens
from the internet.

A token that passed is remembered, so a client that sends the same token again pays about 2.6 µs, on
any algorithm, instead of the verify. It is judged again on its `exp` and `nbf` each time, and a token
that failed is never remembered, so a forged one still costs the full verify. The server remembers 1024
tokens by default:

```sh
BOTE_JWT_CACHE=4096   # tokens remembered, a power of two up to 65536; 0 turns it off
```

To ask for a scope before a tool runs, name it. Only a JWT says what it grants, so this needs one of
the verifiers above:

```sh
BOTE_TOOL_SCOPES="fs_write=files:write,web_fetch=net:read net:write" \
BOTE_SCOPE_IMPLIES="admin=files:write net:read,files:write=files:read" \
BOTE_JWKS_FILE="/etc/bote/jwks.json" \
BOTE_RESOURCE_URL="https://mcp.example.com/mcp" \
BOTE_AUTH_SERVERS="https://auth.example.com" ./build/bote http 8390
# A token that lacks a tool's scopes → 403 with error="insufficient_scope", naming all of them
```

A token needs every scope a tool names. Its scopes come from the `scope` claim, or from `scp`, as a string
or an array, and they match exactly. `BOTE_SCOPE_IMPLIES` says what a broader scope covers, and chains. The
403 is what lets a client ask for more: the official TypeScript client requests the union and retries on its
own. A call inside a batch, or on a WebSocket, has no status of its own and gets JSON-RPC error `-31403`.

A name no tool has stops the server, because a typo must not leave a tool open, and so do scopes with no JWT
verifier. stdio and the Unix socket take no token, so scopes change nothing there, and bote logs a WARN.
`BOTE_TOOL_SCOPES` covers tools. An embedder declares scopes on a prompt, a resource or a resource template
with `prompt_def_with_required_scopes` and its siblings, and `prompts/get`, `resources/read` and a completion
for them are held to the same rules.

### Browsers

An HTTP-family transport refuses a present `Origin` with 403 unless it is on the list, and the list is
empty until you set one. A client that sends no `Origin`, such as `curl` or an MCP client library, is
not affected.

```sh
BOTE_ALLOWED_ORIGINS="https://app.example,http://localhost:3000" ./build/bote bridge 8391
```

### Worker threads

`bote streamable` serves on worker threads, 4 by default: its loop only accepts, and each connection goes
to the next free worker, so a client that is slow to send or to read, or a token that is slow to verify,
holds one worker and not the server. `BOTE_WORKERS` sets how many, up to 64, and `0` serves every request
on the one loop, one at a time, as before 3.7.0. The other transports serve one client at a time.

```sh
BOTE_WORKERS=8 ./build/bote streamable 8392
```

### Listen streams

A client of MCP 2026-07-28 can send `subscriptions/listen` naming the list changes it wants,
`toolsListChanged`, `promptsListChanged` and `resourcesListChanged`, and the resources whose updates it
wants, `resourceSubscriptions`, and hear of each as it happens:
first an acknowledgement of what bote honors, then a notification for each change it asked for, until it
sends `notifications/cancelled` naming the listen. On `http`, `bridge` and `streamable` the response stays
open as an event stream, and one thread holds every stream, so a stream holds no worker; a client that
closes or sends anything loses its stream, and so does one that stops reading, once its socket will take no
more, and `BOTE_LISTEN_KEEPALIVE_MS` (default 15000) is how often each gets a keepalive comment. On stdio, unix and `ws` the connection is the stream, and the
changes go out between the replies. `BOTE_LISTEN_MAX` (default 256, at most 65536) bounds the streams open at
once over HTTP, and the listens one connection holds elsewhere; `0` turns the method off. An embedder says a
resource changed with `dispatcher_resource_updated(d, uri)`, and each listen subscribed to that URI, or to one
it falls under, is told.

```sh
BOTE_LISTEN_MAX=64 BOTE_LISTEN_KEEPALIVE_MS=30000 ./build/bote http 8390
```

A listen made with a token is honored only the URIs the token may read, and is told of an update to a resource
that requires a scope only when it named that resource's URI itself.

### Cancellation

A client can cancel a request it sent. On `http`, `streamable` and `bridge`, for MCP 2026-07-28, it closes its
connection. On stdio, a Unix socket and a WebSocket it sends `notifications/cancelled` naming the request. For
2025-11-25 on `streamable` it sends that in a POST of its own, on the request's session. A handler runs to its
end on the thread that called it, so a handler that runs long asks as it goes:

```cyrius
fn h_crawl(args, claims) {
    var i = 0;
    while (i < 1000) {
        if (bote_cancelled() == 1) { return content_text_response("stopped"); }
        # ... one step of the work ...
        i = i + 1;
    }
    return content_text_response("done");
}
```

Once `bote_cancelled()` has said 1, nothing more goes out for the request, its response included, and a `tools/call`
is audited as cancelled. Ask from the handler's own thread: a thread it starts is answered with 0. Each call asks the connection once: a poll, and on stdio, a Unix socket and a
WebSocket a look at what has arrived. A handler that never asks is answered as before. `web_fetch` and `web_search` ask
every 100 ms while they wait.

### Progress

A request that carries `_meta.progressToken` hears how it is going. A handler reports with
`bote_progress(progress, total, message)`, `total` -1 when none is known and `message` 0 for none:

```cyrius
fn h_index(args, claims) {
    var i = 0;
    while (i < 100) {
        # ... index one file ...
        bote_progress(i + 1, 100, "indexing");
        i = i + 1;
    }
    return content_text_response("indexed");
}
```

It goes as `notifications/progress`, stamped with the request's token, on the request's own stream:

- **HTTP:** the response to its POST becomes an SSE stream at the first report, the reports and then the response,
  which ends it. This is on `http`, `streamable` and `bridge` for 2026-07-28, and on `http` and `streamable` for
  2025-11-25.
- **stdio, a Unix socket and a WebSocket:** a line or a frame each, before the answer.

It returns 1 when it went, 0 when there is nothing to send it to, and -1 when it was refused:

- **0, nothing to send it to:** no token, a client whose `Accept` takes no event stream, a request already cancelled, or
  a call from another thread than the handler's.
- **-1, refused:** a `progress` no greater than the last, which the spec says MUST increase, or a client gone.

A request that reports nothing is answered exactly as before.

### Structured output

A tool can say what its result holds, as the tools spec's `outputSchema`: a JSON Schema, in its own JSON.

```cyrius
var WEATHER = "{\"type\":\"object\",\"properties\":{\"temperature\":{\"type\":\"number\"},\"conditions\":{\"type\":\"string\"}},\"required\":[\"temperature\",\"conditions\"]}";

fn h_weather(args, claims) { return "{\"temperature\":22.5,\"conditions\":\"Partly cloudy\"}"; }

dispatcher_register_tool(d, tool_def_with_output_schema(tool_def_new("svc_weather", "The weather now", schema_new("object", 0, vec_new())), WEATHER), &h_weather);
```

- **Listed as written.** On 2026-07-28 the root may be any type. 2025-11-25 allows only an object there, so a tool
  whose root is anything else is listed there with the schema as the property `result` of an object, its references
  moved to match, as the official SDK does it.
- **A bare value is the structured result.** The handler above returns a record, which goes as `structuredContent`,
  with its JSON in a text block, as the spec says a tool SHOULD send it. A handler can return a whole CallToolResult
  instead.
- **Every result is judged before it goes.** "Servers MUST provide structured results that conform to this schema."
  A result that is no error and has no `structuredContent`, one whose `structuredContent` does not match, and one that
  names `content`, `structuredContent` or `isError` twice are sent as a tool error naming what is wrong, never as
  they were. The call is audited as a failure, and a WARN names the tool and why, never what the result held.
- **The shape each revision allows.** `structuredContent` that is no object gets a text block holding its JSON when
  the result has none, and on 2025-11-25 it goes as `{"result": …}`, from any tool.

Registration refuses a schema that is not a JSON object. bote judges the subset it judges an `inputSchema` by, and
warns at registration of the keywords it lists and does not enforce.

### Input schemas as JSON

An `inputSchema` can be its own JSON too, which carries every keyword at its root: `$schema`, `$defs`, `allOf`, `if`,
the `additionalProperties: false` the spec recommends for a tool with no parameters, and the rest.

```cyrius
var CONTACT = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"type\":\"object\",\"$defs\":{\"address\":{\"type\":\"object\",\"properties\":{\"city\":{\"type\":\"string\"}}}},\"properties\":{\"name\":{\"type\":\"string\"},\"address\":{\"$ref\":\"#/$defs/address\"}},\"required\":[\"name\"],\"additionalProperties\":false}";

dispatcher_register_tool(d, tool_def_new("svc_contact", "Adds a contact", tool_schema_from_json(CONTACT)), &h_contact);
```

- **Listed as written**, on both revisions, on one line: a CR or LF between its tokens is left out.
- **Judged by what bote enforces of it**: the root's `properties` and `required`, read from it as they decode, as a
  `schema_new` schema's are. A WARN names each keyword at the root, and on each property, that bote lists and does
  not enforce.
- **Refused at registration** unless a client's parser would take it: an object whose `type` is `"object"`, whose
  `properties` is an object of schemas and whose `required` is an array of names, with no key twice in one object, no
  NUL in a key or a string, and a `$schema` naming a dialect bote knows.

A schema made with `schema_new` is listed as it was, now built once at registration.

### Titles, icons and instructions

What a client shows people: a `title` beside the `name`, which is for programs, and `icons`. Tools, prompts, prompt
arguments (a title only), resources, resource templates and the server itself carry them, on both revisions:

```cyrius
var icon = bote_icon_with_theme(bote_icon_with_mime_type(bote_icon_new("https://example.com/weather.png"), "image/png"), "light");
var icons = vec_new();
vec_push(icons, icon);
dispatcher_register_tool(d, tool_def_with_icons(tool_def_with_title(tool_def_new("svc_weather", "The weather now", schema), "Weather"), icons), &h_weather);

dispatcher_set_server_details(d, "Acme MCP", "Weather and maps for Acme", "https://acme.example/mcp", icons);
dispatcher_set_instructions(d, "Prefer svc_weather for forecasts.");
```

- **An icon** is `bote_icon_new(src)`, with `_with_mime_type`, `_with_sizes` (a vec of `"48x48"` or `"any"`) and
  `_with_theme` (`"light"` or `"dark"`). `src` is an `http://` or `https://` URL or a `data:` URI of Base64 data.
  bote checks each field where it is registered and refuses one the spec does not allow: the official client refuses a
  whole list over one icon it cannot read. `bote_icons_check` gives the reason first.
- **Checked once, sent as checked:** each list sends what passed the check at registration. A definition changed after
  that sends what it was registered with until it is registered again.
- **serverInfo:** `dispatcher_set_server_details(d, title, description, website_url, icons)` returns 0 or a BoteError.
  `initialize` and `server/discover` send every field. Every other 2026-07-28 result names the server in its `_meta`
  with the name, the version and the title only, so a `data:` icon of some kilobytes is not in every reply.
- **Instructions** go in `initialize`'s result on 2025-11-25 and in `server/discover`'s on 2026-07-28, where the spec
  moved them.

### Multi round-trip requests

On 2026-07-28 a handler that needs something from the client asks for it in its answer. It returns an
`InputRequiredResult`; the client gets what is asked, from its user, its model or its roots, and calls again with the
answers. `tools/call`, `prompts/get` and `resources/read` may ask. The handler ABI is unchanged: a handler reads the
answers and its own state from the request.

```cyrius
fn h_deploy(args, claims) {
    var a = bote_req_alloc();
    if (bote_input_response_a(a, "confirm") != 0) {
        # The client answered: accepted, or declined or cancelled.
        if (bote_input_accepted_a(a, "confirm") != 0) { return "{\"content\":[{\"type\":\"text\",\"text\":\"deployed\"}]}"; }
        return "{\"content\":[{\"type\":\"text\",\"text\":\"not deployed\"}]}";
    }
    if (bote_input_possible() == 0) { return "{\"content\":[{\"type\":\"text\",\"text\":\"confirmation needed\"}],\"isError\":true}"; }
    var ir = bote_input_required_new_a(a);
    bote_input_elicit(ir, "confirm", "Deploy to production?", "{\"type\":\"object\",\"properties\":{\"ok\":{\"type\":\"boolean\"}}}");
    bote_input_state(ir, "step=confirm");
    return bote_input_required_json(ir);
}
```

- **What a handler may ask:** `bote_input_elicit` (a form), `bote_input_elicit_url` (send the user to a URL, the spec's
  way to ask for anything sensitive), `bote_input_sample` and `bote_input_roots`, each under a key the answer comes
  back under. `bote_input_state` adds state of its own, which `bote_request_state()` gives back on the retry.
- **What the client declared:** `bote_client_has("elicitation.form")`, `"elicitation.url"`, `"sampling"`,
  `"sampling.tools"`, `"roots"` read the request's `clientCapabilities`. A result asking for one the client did not
  declare is answered `-32021` with the capabilities it needs, as the spec says.
- **Judged before it goes:** only the three methods the spec allows, each request in the shape a client parses
  (`params`, an object, on an elicitation and a sampling request; a mode of `form` or `url`), at least one request or
  a state, and no key twice. A handler's mistake is `-32603`. On an older revision there is no way to ask: a tool
  that does gets a tool error, and `bote_input_possible()` is 0, so a handler can answer as it can.
- **The state is sealed.** The spec makes it attacker-controlled. bote seals every state a handler gives and opens
  every one a client brings back, through the dispatcher's sealer, before any handler runs: a state that does not
  open is `-32602`. The binary's sealer is HMAC-SHA256 and good for 600 seconds; set `BOTE_STATE_KEY` (64 hex digits
  or more) for servers that share states, or each process keys itself. A library user wires it with
  `dispatcher_with_state_hmac(d, bote_state_hmac_new(key, len, ttl))`, or their own with
  `dispatcher_set_state_sealer`. With none, a state is refused both ways. The client can read the state: put nothing
  in it the client may not know.
- **Bound to the request and who made it.** A state opens only for the same method, the same tool, prompt or URI,
  the same arguments (compact, so spacing does not count), and the same principal: the one
  `dispatcher_set_state_principal(d, fp, ctx)` names for the request's claims, else the token the request presented.
  With JWT access tokens the binary names the token's issuer and subject (`jwt_state_principal`), so a client that
  refreshes its token between rounds keeps its state. A request whose claims are not its token, from a validator that
  returns a record or a transport that passes its own, can be bound to no one without a principal function, so its
  state is refused both ways. The answers are not bound: keep in the state what the handler judged.
- **On a server with worker threads**, a thread a handler starts has a request state of its own, empty:
  `bote_client_has`, `bote_input_possible` and `bote_request_state` answer 0 there. Ask on the handler's thread.

### Connection deadlines

Every transport but streamable serves one client at a time, so each connection gets a **send deadline**:
a client that stops reading for `BOTE_SEND_TIMEOUT_MS` (default 30000) is dropped instead of
wedging the server. The Unix transport's receive deadline, `BOTE_UNIX_IDLE_MS`, is **off** by
default, because a Unix client may legitimately keep its connection quiet between requests; set it
to reap idle peers. `BOTE_WS_IDLE_MS` is the same for WebSocket, where a connection's token is also judged again at
most once a second, and one that has expired ends the connection. `BOTE_WEB_TIMEOUT_MS` (default 15000)
bounds an outbound `web_fetch` or `web_search`, since every client waits on it; each is made in a child
process, which is killed at that deadline, DNS lookup included. All take milliseconds, and `0` disables.

```sh
BOTE_SEND_TIMEOUT_MS=10000 BOTE_UNIX_IDLE_MS=60000 ./build/bote unix /run/bote.sock
```

### Logging

bote logs through [sakshi](https://github.com/MacCracken/sakshi) to **stderr** — never stdout,
which is the JSON-RPC wire on the stdio transport.

```sh
BOTE_LOG=debug ./build/bote http 8390        # off | fatal | error | warn | info (default) | debug | trace
BOTE_LOG_FILE=/var/log/bote.log ./build/bote # append lines to a file instead of stderr
```

Captured from `BOTE_LOG=debug BOTE_BEARER_TOKENS=tok-a ./build/bote http 18390` and five `curl`
requests (two authorized, a wrong token, no token, a `GET`):

```
[1127760397684945] [INFO] bearer auth enabled tokens=1
[1127760397713810] [INFO] bote starting version=3.4.0 transport=http addr=127.0.0.1 port=18390 auth=bearer log=debug
[1127760893749315] [DEBUG] request method=initialize id=1 status=ok us=10
[1127760900255149] [DEBUG] request method=tools/call id=4 tool=bote_echo status=ok us=22
[1127760906066760] [WARN] request refused status=401 reason=invalid_token
[1127760911987918] [WARN] request refused status=401 reason=missing_token
[1127760917921870] [DEBUG] request refused transport=http status=405 reason=method
```

The bracketed number is sakshi's monotonic nanosecond clock, not wall time.

INFO is lifecycle (starting, listening, stopped), WARN a refusal the server survived (401 / 403 / 413,
an SSRF block, a path escape), ERROR something an operator must fix (a bind that failed), DEBUG one
line per request, and `trace` also turns on sandhi's request spans. No token, request body, tool
argument or full URL is ever logged; client-supplied values (a method or tool name) are logfmt-escaped
so they cannot forge a line. Library code never configures sakshi: an application that vendors
`dist/bote.cyr` keeps its own `sakshi_set_level` / `sakshi_output_*`.

### Try it

```sh
# stdio
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}' | ./build/bote

# HTTP
./build/bote http 8390 &
curl -X POST http://127.0.0.1:8390/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2025-11-25' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'

# WebSocket (with wscat)
./build/bote ws 8393 &
wscat -c ws://127.0.0.1:8393/mcp
> {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}
```

---

## MCP Protocol

bote implements [MCP](https://modelcontextprotocol.io/) over JSON-RPC 2.0.

| Method | Description | Response |
|---|---|---|
| `initialize` | Handshake | Server info, capabilities, negotiated protocol version |
| `tools/list` | Discovery | Array of tool definitions with schemas |
| `tools/call` | Execution | Tool result or JSON-RPC error |
| `prompts/list` / `prompts/get` | Prompts capability (when a `PromptRegistry` is wired) | Prompt metadata / generated messages |
| `resources/list` / `resources/read` / `resources/templates/list` | Resources capability (when a `ResourceRegistry` is wired) | Resource metadata / contents / `{"resourceTemplates":[]}` (no URI-template registry) |
| `completion/complete` | Completion capability | Argument completion values |
| `ping` | Keepalive | Empty result — answered even on a dispatcher with no registries wired |
| `server/discover` | 2026-07-28 only | Supported versions and capabilities, with the cache hints |
| `subscriptions/listen` | 2026-07-28 only | The acknowledgement, then a notification for each list change asked for: an event stream on HTTP, the connection itself on stdio, unix and `ws` |

Supported MCP protocol versions: `2024-11-05`, `2025-03-26`, `2025-06-18`,
`2025-11-25` (default) — the same list for `initialize` negotiation and for the
`MCP-Protocol-Version` header on every HTTP-family transport. A request with no such
header is taken as `2025-03-26`, as the spec allows, unless `require_protocol` is set.

`2026-07-28` is served beside them in one process, on every transport. A request selects it
in `params._meta`, and on HTTP in `MCP-Protocol-Version` too, and the two must agree. See
[docs/spec-compliance.md](docs/spec-compliance.md#mcp-2026-07-28-stateless-path). A tool can
have an argument mirrored in an `Mcp-Param-{Name}` header by putting `x-mcp-header` on its
property in the schema; registration checks the annotation, and HTTP checks the header against
the body. Do not mark a secret that way: every intermediary sees the header.

The official MCP client works against bote on `http`, `streamable`, `bridge` and stdio, and the
official conformance suite passes every one of the 39 scenarios it scores for 2026-07-28 and 22 of
the 30 2025-11-25 requires. What fails is features bote lacks, listed with the reason in
`tests/fixtures/conformance-expected-<revision>.yaml`. `./scripts/conformance.sh` reruns it all.

### Built-in tools (registered by default)

| Tool | Purpose |
|---|---|
| `bote_echo` | Echoes its arguments verbatim — smoke-test target |
| `libro_query` | Filter / paginate libro chain entries (source / agent / severity / time) |
| `libro_verify` | Hash-link integrity check |
| `libro_export` | Every entry as a JSON array |
| `libro_proof` | Merkle inclusion proof for an entry by index |
| `libro_retention` | Apply a policy (`keep_count` / `keep_duration` / `keep_after` / `pci_dss` / `hipaa` / `sox`) |
| `fs_write` / `fs_read` / `fs_mkdir` | Filesystem ops confined to `BOTE_FS_ROOT` (default `.`); `fs_read` sends the text as a block and as `structuredContent`, `{text, bytes, truncated}` |
| `web_fetch` | GET a URL (http/https only; loopback / private / link-local / metadata refused by the SSRF guard — on the URL and, since 3.4.4, on the address the name resolves to), strip HTML to readable text, size-capped |
| `web_search` | Query a SearXNG JSON endpoint (`BOTE_SEARXNG_URL`) |

The five libro tools and `fs_read` list an outputSchema (3.8.4), which the dispatcher judges each of their results by,
and a client can check.

### Error codes

| Code | Meaning |
|---|---|
| -32700 | Parse error (the message is not valid JSON) |
| -32600 | Invalid request (valid JSON, but not a JSON-RPC request) |
| -32601 | Method not found (including a method behind an un-advertised capability) |
| -32602 | Invalid params — including an unknown tool, a malformed `tools/call`, an invalid pagination cursor, a missing required prompt argument |
| -32002 | Resource not found (`resources/read`; the URI is in `error.data`) |
| -32000 | Tool execution / sandbox error — in `bote_err_rpc_code` for an embedder's own use; bote's dispatcher reports a failed tool as a result with `isError: true` instead |
| -32003 | Transport closed / bind failed |
| -32603 | Internal error, including a handler's InputRequiredResult bote cannot send |
| -32800 | Request cancelled |
| -32021 | Missing required client capability (2026-07-28): the request needs input the client did not declare it can give, named in `error.data.requiredCapabilities` |

---

## Modules

```
src/log.cyr                Operational logging through sakshi (BOTE_LOG, BOTE_LOG_FILE)
src/error.cyr              BoteErrTag enum, rpc_code mapping, format
src/protocol.cyr           JsonRpcRequest / Response / Error
src/jsonx.cyr              Nested-aware JSON value extractor + RFC 8259 syntax check + the string escaper
src/registry.cyr           ToolDef / ToolSchema / ToolRegistry / annotations
src/prompts.cyr            PromptRegistry (prompts/list + prompts/get)
src/resources.cyr          ResourceRegistry (resources/list + resources/read)
src/dispatch.cyr           Dispatcher + initialize / tools / prompts / resources / completion
src/codec.cyr              parse_request, serialize_response, process_message
src/schema.cyr             CompiledSchema (typed validation)
src/stream.cyr             ProgressUpdate, CancellationToken, progress notifications
src/session.cyr            SessionStore, validate_protocol_version, validate_origin
src/discovery.cyr          Cross-node tool discovery
src/audit.cyr              AuditSink (fn-ptr + ctx)
src/audit_libro.cyr        LibroAudit adapter
src/events.cyr             EventSink (fn-ptr + ctx) + topic constants
src/events_majra.cyr       MajraEvents adapter
src/auth.cyr               Bearer-token middleware (RFC 6750)
src/jwt.cyr                JWT HS256 verifier (RFC 7519)
src/pkce.cyr               RFC 7636 PKCE-S256 helpers
src/sandbox.cyr            Pluggable sandbox runner (kavach-shaped)
src/content.cyr            Typed MCP content blocks (text/image/audio/resource/blob)
src/host.cyr               HostRegistry + SSRF guard (IPv4 + IPv6)
src/libro_tools.cyr        Five built-in MCP tools over a libro chain
src/fs_tools.cyr           fs_write / fs_read / fs_mkdir (BOTE_FS_ROOT-confined)
src/web_tools.cyr          web_fetch / web_search (SearXNG)
src/fetch.cyr              An outbound request made in a child, with a deadline
src/jwks.cyr               A JWK Set fetched from a jwks_uri
src/conn.cyr               Per-connection deadlines, the serve entry, worker threads
src/http_era.cyr           Which MCP revision an HTTP POST belongs to, and the 2026-07-28 header checks
src/oauth_metadata.cyr     OAuth 2.0 Protected Resource Metadata (RFC 9728) and the scope gate
src/listen.cyr             subscriptions/listen streams, held by one thread off the serving ones
src/transport_stdio.cyr    Line-oriented JSON-RPC over stdin/stdout
src/transport_http.cyr     HTTP/1.1 server with middleware
src/transport_unix.cyr     AF_UNIX line-oriented transport
src/bridge.cyr             TS-client bridge with CORS
src/transport_streamable.cyr  Streamable HTTP / SSE (MCP 2025-11-25)
src/transport_ws.cyr       WebSocket (RFC 6455)
src/main.cyr               CLI entry: stdio / http / unix / bridge / streamable / ws
src/main_common.cyr        Entry helpers (dispatcher, mode selection, env bearer wiring, log lines)
```

Dependencies rehydrate into `lib/` (gitignored) via `cyrius deps`.
Cross-project deps **libro and majra** are git-pinned via
`[deps.<name>]` in `cyrius.cyml`. Two consumer bundles ship in `dist/`:
`bote.cyr` (full, 38 modules) and `bote-core.cyr` (core 13,
transport-free) — see [DEPS-PATTERN.md](DEPS-PATTERN.md). Both state
**cyrius ≥ 6.7.13** on the `# Minimum cyrius:` line near the top of each (`bote-core.cyr` alone still builds from 6.7.2);
a repo that copies the core bundle rather than pinning it must also declare
`sakshi`.

---

## Verification

### Tests — 5928 total across twenty-seven files (+ a core-only drift smoke)

Green on **x86_64 and aarch64** (`cyrius test --aarch64 <file>`; all 5928 run under `qemu-aarch64`). Every entry and every test / bench / fuzz unit also compiles for the **agnos** target (`cyrius build --agnos`) with zero undefined functions — CI-gated since 3.3.12; the Unix transport fail-closes there, agnos having no `AF_UNIX`.

```sh
cyrius test tests/bote.tcyr               # 969 — core protocol/dispatch/codec/schema/session/transports
cyrius test tests/conformance.tcyr        # 218 — JSON-RPC 2.0 + MCP 2025-11-25 + 2026-07-28 conformance, by spec clause
cyrius test tests/bote_auth.tcyr          # 91 — bearer + allowlist + JWT + PKCE validators, the head check
cyrius test tests/bote_cancel.tcyr        # 141 — cancelling a request in flight: the check, the codec, stdio, unix, HTTP, a fetch's child
cyrius test tests/bote_content.tcyr       # 25 — typed content blocks
cyrius test tests/bote_fetch.tcyr         # 62 — a request made in a child: the record, the cap, the deadline, a real GET
cyrius test tests/bote_fs_tools.tcyr      # 76 — fs_write / fs_read / fs_mkdir
cyrius test tests/bote_host.tcyr          # 207 — host registry + SSRF guard (IPv4 + IPv6)
cyrius test tests/bote_http_era.tcyr      # 1863 — MCP 2026-07-28 on HTTP, on the real handlers, and the 403 for a missing scope
cyrius test tests/bote_jwks.tcyr          # 49 — a jwks_uri: what it may be, and real fetches from a child
cyrius test tests/bote_jwt.tcyr           # 510 — JWT: HS256, RS256, ES256, JWK Sets, every claim enforcement, scopes, tokens already verified, keys from a source
cyrius test tests/bote_libro_tools.tcyr   # 101 — libro_tools wrappers
cyrius test tests/bote_listen.tcyr        # 224 — subscriptions/listen: the filter, and real streams on the three HTTP handlers
cyrius test tests/bote_listen_line.tcyr   # 192 — subscriptions/listen where the connection is the stream: stdio, unix, WebSocket
cyrius test tests/bote_log.tcyr           # 50 — sakshi logging: levels, line bytes, escaping, truncation
cyrius test tests/bote_display.tcyr       # 107 — titles, icons, serverInfo's details and instructions: the checks, each list, the heap
cyrius test tests/bote_output.tcyr        # 249 — outputSchema: registration, the 2025-11-25 form, tools/list, each result judged and shaped, the heap
cyrius test tests/bote_input.tcyr         # 148 — multi round-trip requests: what a handler reads and returns, -32021, the sealed state and its binding, the heap
cyrius test tests/bote_tool_schema.tcyr   # 103 — an inputSchema as its own JSON: what registration reads and refuses, the list, the heap
cyrius test tests/bote_progress.tcyr      # 110 — progress on a request's own stream: the writer, stdio, unix, WebSocket, HTTP's SSE response
cyrius test tests/bote_pkce.tcyr          # 17 — RFC 7636 PKCE-S256
cyrius test tests/bote_sandbox.tcyr       # 13 — kavach-shaped runner adapter
cyrius test tests/bote_streamable.tcyr    # 64 — Streamable HTTP / SSE internals
cyrius test tests/bote_threads.tcyr       # 81 — bote on worker threads: each case from several threads at once
cyrius test tests/bote_transport_unix.tcyr # 76 — Unix transport: sockaddr, accept-error policy, backoff, deadlines, closed-peer write
cyrius test tests/bote_web_tools.tcyr     # 88 — web_fetch / web_search + the SSRF guard (URL and resolved address) + their records
cyrius test tests/bote_ws.tcyr            # 94 — WebSocket config + wire-up + the send-deadline drop + scopes per frame + what an upgrade and a connection keep
cyrius test tests/bote_core_only_smoke.tcyr  # drift guard — includes only dist/bote-core.cyr
```

### Benchmarks

```sh
cyrius bench tests/bote.bcyr
```

A logged row — bote 3.4.15 on cyrius 6.7.18, single run (1-minute load 8.6), `AMD Ryzen 7 5800H`. The full history, with the host conditions
each block was taken under, is [`benches/history.log`](benches/history.log).

| Hot path | Avg |
|---|---|
| `dispatch_initialize` | ~1.55 µs |
| `dispatch_tools_list` | ~1.70 µs |
| `dispatch_tools_call` (no audit / event sink) | ~2.23 µs |
| `dispatch_tools_call_result` (a handler returning a CallToolResult) | ~2.06 µs |
| `jsonx_get_str_flat` | ~183 ns |
| `jsonx_get_raw_nested` | ~455 ns |
| `jsonx_valid_request` | ~585 ns |
| `codec_parse_request` | ~953 ns |
| `codec_serialize_response` | ~210 ns |
| `codec_process_message` (full pipeline) | ~4.19 µs |
| `validate_compiled_simple` | ~491 ns |
| `validate_compiled_nested` | ~1.99 µs |
| `schema_compile_simple` | ~2.83 µs |
| `schema_compile_nested` | ~6.43 µs |
| `auth_bearer_check_unset` | ~8 ns |
| `auth_bearer_check_set` | ~851 ns |

> A single row is an illustration, not a measurement: run-to-run swings of
> 5–10 % on the sub-microsecond rows are host state, and a one-byte shift in
> string data moves the alignment-sensitive ones. The measurements are the
> **interleaved A/Bs** recorded in the CHANGELOG at each toolchain bump
> (3.3.10 for 6.6.3 → 6.6.6: a wash; 3.4.0 for 6.6.20 → 6.7.9: a wash, and the
> logging added at no measurable cost; 3.4.1, where an unobserved `tools/call`
> stopped making two `clock_gettime` syscalls, 1.35 µs each on this host; 3.3.17 for 6.6.14 → 6.6.20, where
> `jsonx_get_str_flat`'s +16% is identical code sitting 4 bytes later in
> its cache line; 3.4.11 for 6.7.9 → 6.7.18: a wash on the full pipeline; 3.4.15, the closeout: the full pipeline 30% faster, and a single-key lookup slower for going through the one scanner). Numbers across toolchain lines are not comparable — the
> 6.6.x emitter moved `dispatch_tools_call` and `codec_process_message`
> well below their 6.5.x figures.

### Fuzz

```sh
cyrius fuzz fuzz/codec_parse.fcyr
cyrius fuzz fuzz/codec_process.fcyr
cyrius fuzz fuzz/jsonx_extract.fcyr
cyrius fuzz fuzz/jwt.fcyr
cyrius fuzz fuzz/http_era.fcyr
cyrius fuzz fuzz/jsonx_valid.fcyr
cyrius fuzz fuzz/oauth_metadata.fcyr
cyrius fuzz fuzz/resource_template.fcyr
cyrius fuzz fuzz/schema_validate.fcyr
cyrius fuzz fuzz/ssrf_ipv6.fcyr
cyrius fuzz fuzz/http_head.fcyr
cyrius fuzz fuzz/req_arena.fcyr
cyrius fuzz fuzz/listen_filter.fcyr
cyrius fuzz fuzz/cancel_lines.fcyr
cyrius fuzz fuzz/output_shape.fcyr
cyrius fuzz fuzz/input_schema.fcyr
# 1 passed, 0 failed each
```

---

## Why bote

Every AGNOS consumer used to implement its own MCP server. After bote
lands as a dep, those become a `Dispatcher` + a handful of tool
handlers — protocol code drops from ~150 LOC per app to zero.

The Cyrius port (v1.0.0) extended the same value into the cyrius
ecosystem, shipping a binary one-tenth the size of the Rust release for
the same surface.

---

## Documentation

| Doc | Topic |
|---|---|
| [docs/architecture/overview.md](docs/architecture/overview.md) | Module map, data flow, six-transport surface |
| [docs/development/roadmap.md](docs/development/roadmap.md) | Forward-facing only: next patch, 3.4.x → 3.6.x, open decisions, 4.0 criteria |
| [docs/spec-compliance.md](docs/spec-compliance.md) | MCP conformance matrix, 2025-11-25 and 2026-07-28 |
| [docs/benchmarks-rust-v-cyrius.md](docs/benchmarks-rust-v-cyrius.md) | Side-by-side performance: Rust v0.92.0 vs Cyrius |
| [docs/cyrius-feedback.md](docs/cyrius-feedback.md) | Port-era Cyrius language issues (historical — all resolved by cyrius 4.4.3) |
| [docs/development/issues/](docs/development/issues/) | Consumer-filed bote issues with their resolutions; every one is closed and lives under `archive/` |
| [docs/resolved-lang-issues.md](docs/resolved-lang-issues.md) | Historical index of bote's upstream cyrius filings and the fix each landed |
| [DEPS-PATTERN.md](DEPS-PATTERN.md) | Distribution contract (`dist/bote.cyr` / `dist/bote-core.cyr`) |
| [SECURITY.md](SECURITY.md) | Threat model, reporting policy |

---

## Versioning

**Current**: see [`VERSION`](VERSION) and the top of [CHANGELOG.md](CHANGELOG.md).
SemVer. The 3.x line carries the full MCP capability suite (prompts / resources /
completion + polled `list_changed` push), the fs / web / libro tool families, six
transports in one binary, and the bearer + JWT HS256 + PKCE auth substrate;
the 2.0 handler ABI is stable across the 2.x → 3.x line. Builds and tests clean on
x86_64 and aarch64; compiles for agnos.

**Roadmap** — see [roadmap](docs/development/roadmap.md).

---

## License

GPL-3.0-only. See [LICENSE](LICENSE).
