# Proposal: `lib/http_server.cyr` for cyrius stdlib

> **Status**: design + reference implementation, ready for cyrius lang-agent
> to integrate into stdlib for the **4.5.0** release.
>
> **Why**: today both bote (MCP server) and vidya (HTTP API for the corpus)
> hand-roll their own minimal HTTP/1.1 server inline. The shapes are nearly
> identical (`make_crlf`, parse path, send response, accept loop). Lifting
> them into stdlib eliminates the duplication and gives every cyrius project
> a baseline server that handles the real-world problems
> (Content-Length-aware reads, percent-decoding, RESTful path matching,
> chunked responses for SSE).
>
> **Sibling**: existing `lib/http.cyr` is HTTP/1.0 **client-only** — keep it.
> `lib/http_server.cyr` is the server side and should be includable
> independently.

---

## Why each consumer hand-rolled it

| Project | LOC | Functions |
|---|---|---|
| `vidya/src/main.cyr` | ~150 (around `cmd_serve`) | `make_crlf`, `http_parse_path`, `http_get_param`, `http_path_segment`, `http_respond`, `http_ok`, `http_not_found`, `http_bad_request`, `http_route`, accept loop |
| `bote/src/transport_http.cyr` | ~400 | `_http_find`, `_http_to_lower`, `_http_iceq`, `_http_next_nl`, `http_find_header`, `http_get_method`, `http_get_path`, `http_body_offset`, `http_content_length`, `_http_send_status`, `_http_send_json_200`, `_http_send_204`, `_http_check_origin`, `_http_check_protocol`, `_http_check_session`, `_http_handle_connection`, `transport_http_run` |
| `bote/src/bridge.cyr` | ~280 | `wrap_tool_result`, `wrap_error_result`, `_bridge_cors_origin`, `_bridge_cors_headers`, `_bridge_send_health`, `_bridge_handle_connection`, `transport_bridge_run` |

Roughly **600 LOC of HTTP plumbing** that all does the same thing. Plus
both bote transports duplicate the bind+listen+accept pattern.

---

## Surface

### Status code constants

```cyr
var HTTP_OK                  = 200;
var HTTP_NO_CONTENT          = 204;
var HTTP_MOVED_PERMANENTLY   = 301;
var HTTP_FOUND               = 302;
var HTTP_NOT_MODIFIED        = 304;
var HTTP_BAD_REQUEST         = 400;
var HTTP_UNAUTHORIZED        = 401;
var HTTP_FORBIDDEN           = 403;
var HTTP_NOT_FOUND           = 404;
var HTTP_METHOD_NOT_ALLOWED  = 405;
var HTTP_REQUEST_TIMEOUT     = 408;
var HTTP_PAYLOAD_TOO_LARGE   = 413;
var HTTP_INTERNAL            = 500;
var HTTP_NOT_IMPLEMENTED     = 501;
var HTTP_SERVICE_UNAVAILABLE = 503;
```

### Server lifecycle

```cyr
# Bind to addr:port, listen, accept connections in a loop.
# For each connection, calls handler_fp(ctx, cfd, req_buf, req_bytes).
# Handler is responsible for parsing, dispatching, and sending the response.
# After the handler returns, http_server_run closes cfd.
#
# `addr` is a network-order IPv4 (use INADDR_ANY() / INADDR_LOOPBACK()).
# Returns 1 on bind/listen failure, never returns on success (loops forever).
fn http_server_run(addr, port, handler_fp, ctx) → exit_code
```

The handler signature:

```cyr
# Called once per accepted connection. Returns 0 (ignored).
# req_buf contains the raw request bytes; req_len is how many.
fn my_handler(ctx, cfd, req_buf, req_len) → 0
```

### Request reading (use inside handler if you need a larger buffer)

```cyr
# Read up to max bytes from cfd into buf. Reads until either:
#   - the request body is fully received (per Content-Length)
#   - cfd is closed by the peer
#   - max bytes are read
# Returns the total bytes read (or -1 on socket error).
# Adds a NUL terminator at buf[returned_len] so the buffer is a valid cstr.
fn http_recv_request(cfd, buf, max) → bytes
```

### Request parsing (read-only; no allocations except returned cstrs)

```cyr
fn http_get_method(buf, blen) → cstr        # "GET" / "POST" / etc
fn http_get_path(buf, blen) → cstr          # "/foo?bar=baz"
fn http_find_header(buf, blen, name) → cstr # case-insensitive; or 0 if absent
fn http_body_offset(buf, blen) → i64        # offset of body (after \r\n\r\n) or -1
fn http_content_length(buf, blen) → i64     # 0 if header absent or malformed
```

### Path / query helpers

```cyr
fn http_path_only(path) → cstr           # strip "?..." from path
fn http_get_param(path, name) → cstr     # query param value, percent-decoded; 0 if absent
fn http_path_segment(path, n) → cstr     # nth segment of "/a/b/c"; 0 = "a"
fn http_url_decode(s) → cstr             # percent-decode (%20 → ' ', + → ' ')
```

### Response builders

```cyr
# Status response with Content-Length: 0 (e.g. 404, 405).
fn http_send_status(cfd, code, msg)

# Full response. extra_headers can be 0 or a cstr that already ends with CRLF
# for each line (e.g. "X-Custom: 1\r\nX-Other: 2\r\n").
fn http_send_response(cfd, code, msg, content_type, body, body_len, extra_headers)

# 204 No Content with optional extra headers.
fn http_send_204(cfd, extra_headers)
```

### Chunked / streaming response (for SSE, large bodies)

```cyr
# Start a Transfer-Encoding: chunked response. After this, call
# http_send_chunk repeatedly, then http_send_chunked_end to finish.
fn http_send_chunked_start(cfd, code, content_type, extra_headers)
fn http_send_chunk(cfd, data, len)
fn http_send_chunked_end(cfd)
```

This is what enables MCP's **streamable HTTP** transport (POST + GET-with-SSE
on the same endpoint) and any future use of long-running responses.

---

## Reference implementation

Cyrius lang-agent: this is ready to drop into `lib/http_server.cyr`. Has been
cross-validated against bote's existing inline server (which it replaces) and
covers vidya's needs too.

```cyr
# lib/http_server.cyr — minimal HTTP/1.1 server primitives
#
# Companion to lib/http.cyr (which is HTTP/1.0 client only).
# Requires: alloc.cyr, string.cyr, fmt.cyr, str.cyr, net.cyr, tagged.cyr, syscalls.cyr

# ================================================================
# Status codes
# ================================================================
var HTTP_OK                  = 200;
var HTTP_NO_CONTENT          = 204;
var HTTP_MOVED_PERMANENTLY   = 301;
var HTTP_FOUND               = 302;
var HTTP_NOT_MODIFIED        = 304;
var HTTP_BAD_REQUEST         = 400;
var HTTP_UNAUTHORIZED        = 401;
var HTTP_FORBIDDEN           = 403;
var HTTP_NOT_FOUND           = 404;
var HTTP_METHOD_NOT_ALLOWED  = 405;
var HTTP_REQUEST_TIMEOUT     = 408;
var HTTP_PAYLOAD_TOO_LARGE   = 413;
var HTTP_INTERNAL            = 500;
var HTTP_NOT_IMPLEMENTED     = 501;
var HTTP_SERVICE_UNAVAILABLE = 503;

# ================================================================
# Internal byte-search + ASCII helpers
# ================================================================

# Find the first occurrence of `needle[0..nlen]` in `haystack[0..hlen]`.
# Returns the offset, or 0 - 1 if not found.
fn _hsv_find(haystack, hlen, needle, nlen) {
    if (nlen == 0) { return 0; }
    if (nlen > hlen) { return 0 - 1; }
    var i = 0;
    while (i <= hlen - nlen) {
        if (memeq(haystack + i, needle, nlen) == 1) { return i; }
        i = i + 1;
    }
    return 0 - 1;
}

fn _hsv_to_lower(c) {
    if (c >= 65) { if (c <= 90) { return c + 32; } }
    return c;
}

fn _hsv_iceq(a, b, n) {
    var i = 0;
    while (i < n) {
        if (_hsv_to_lower(load8(a + i)) != _hsv_to_lower(load8(b + i))) { return 0; }
        i = i + 1;
    }
    return 1;
}

fn _hsv_next_nl(buf, start, end) {
    var i = start;
    while (i < end) {
        if (load8(buf + i) == 10) { return i; }
        i = i + 1;
    }
    return end;
}

# ================================================================
# Request parsing
# ================================================================

fn http_get_method(buf, blen) {
    var sp = _hsv_find(buf, blen, " ", 1);
    if (sp < 0) { return ""; }
    var out = alloc(sp + 1);
    memcpy(out, buf, sp);
    store8(out + sp, 0);
    return out;
}

fn http_get_path(buf, blen) {
    var sp1 = _hsv_find(buf, blen, " ", 1);
    if (sp1 < 0) { return ""; }
    var sp2 = _hsv_find(buf + sp1 + 1, blen - sp1 - 1, " ", 1);
    if (sp2 < 0) { return ""; }
    var out = alloc(sp2 + 1);
    memcpy(out, buf + sp1 + 1, sp2);
    store8(out + sp2, 0);
    return out;
}

fn http_body_offset(buf, blen) {
    var off = _hsv_find(buf, blen, "\r\n\r\n", 4);
    if (off < 0) { return 0 - 1; }
    return off + 4;
}

fn http_find_header(buf, blen, name) {
    var nlen = strlen(name);
    var headers_end = _hsv_find(buf, blen, "\r\n\r\n", 4);
    if (headers_end < 0) { headers_end = blen; }
    var line_start = _hsv_next_nl(buf, 0, headers_end);
    if (line_start >= headers_end) { return 0; }
    line_start = line_start + 1;
    while (line_start < headers_end) {
        var line_end = _hsv_next_nl(buf, line_start, headers_end);
        if (line_start + nlen + 1 <= line_end) {
            if (_hsv_iceq(buf + line_start, name, nlen) == 1
                && load8(buf + line_start + nlen) == 58) {
                var vs = line_start + nlen + 1;
                while (vs < line_end && load8(buf + vs) == 32) { vs = vs + 1; }
                var ve = line_end;
                if (ve > vs && load8(buf + ve - 1) == 13) { ve = ve - 1; }
                var vlen = ve - vs;
                var out = alloc(vlen + 1);
                memcpy(out, buf + vs, vlen);
                store8(out + vlen, 0);
                return out;
            }
        }
        line_start = line_end + 1;
    }
    return 0;
}

fn http_content_length(buf, blen) {
    var v = http_find_header(buf, blen, "Content-Length");
    if (v == 0) { return 0; }
    var n = 0;
    var i = 0;
    while (load8(v + i) != 0) {
        var c = load8(v + i);
        if (c >= 48 && c <= 57) { n = n * 10 + (c - 48); }
        i = i + 1;
    }
    return n;
}

# ================================================================
# Path + query helpers
# ================================================================

# "/foo?bar=baz" → "/foo".  No allocation if no '?'.
fn http_path_only(path) {
    if (path == 0) { return path; }
    var i = 0;
    while (load8(path + i) != 0) {
        if (load8(path + i) == 63) {
            var out = alloc(i + 1);
            memcpy(out, path, i);
            store8(out + i, 0);
            return out;
        }
        i = i + 1;
    }
    return path;
}

# Percent-decode (%20 → ' ', + → ' '). Returns alloc'd cstr.
fn http_url_decode(s) {
    if (s == 0) { return ""; }
    var slen = strlen(s);
    var out = alloc(slen + 1);
    var si = 0;
    var oi = 0;
    while (si < slen) {
        var c = load8(s + si);
        if (c == 37 && si + 2 < slen) {
            var h1 = load8(s + si + 1);
            var h2 = load8(s + si + 2);
            var v1 = 0;
            var v2 = 0;
            if (h1 >= 48 && h1 <= 57) { v1 = h1 - 48; }
            elif (h1 >= 65 && h1 <= 70) { v1 = h1 - 55; }
            elif (h1 >= 97 && h1 <= 102) { v1 = h1 - 87; }
            if (h2 >= 48 && h2 <= 57) { v2 = h2 - 48; }
            elif (h2 >= 65 && h2 <= 70) { v2 = h2 - 55; }
            elif (h2 >= 97 && h2 <= 102) { v2 = h2 - 87; }
            store8(out + oi, v1 * 16 + v2);
            si = si + 3;
        } elif (c == 43) {
            store8(out + oi, 32);
            si = si + 1;
        } else {
            store8(out + oi, c);
            si = si + 1;
        }
        oi = oi + 1;
    }
    store8(out + oi, 0);
    return out;
}

# Query parameter: "/a?foo=1&bar=hello%20world" name="bar" → "hello world".
# Returns 0 if not found.
fn http_get_param(path, name) {
    if (path == 0) { return 0; }
    var qmark = 0;
    var i = 0;
    while (load8(path + i) != 0) {
        if (load8(path + i) == 63) { qmark = i + 1; i = i + 999999; }
        i = i + 1;
    }
    if (qmark == 0) { return 0; }
    var nlen = strlen(name);
    var pi = qmark - 1;   # roll back from the +999999 break
    if (pi < qmark) { pi = qmark; }
    pi = qmark;
    var plen = strlen(path);
    while (pi < plen) {
        # Match name=
        if (pi + nlen + 1 <= plen
            && memeq(path + pi, name, nlen) == 1
            && load8(path + pi + nlen) == 61) {
            var vs = pi + nlen + 1;
            var ve = vs;
            while (ve < plen && load8(path + ve) != 38) { ve = ve + 1; }
            var raw = alloc(ve - vs + 1);
            memcpy(raw, path + vs, ve - vs);
            store8(raw + ve - vs, 0);
            return http_url_decode(raw);
        }
        # Skip to next '&'
        while (pi < plen && load8(path + pi) != 38) { pi = pi + 1; }
        pi = pi + 1;
    }
    return 0;
}

# Nth segment of a path. http_path_segment("/a/b/c", 0) → "a", 1 → "b".
# Returns 0 if n is out of range.
fn http_path_segment(path, n) {
    if (path == 0) { return 0; }
    var path_only = http_path_only(path);
    var plen = strlen(path_only);
    var seg = 0;
    var ss = 0;
    var pi = 0;
    if (pi < plen && load8(path_only + pi) == 47) { pi = pi + 1; ss = pi; }
    while (pi <= plen) {
        var c = 0;
        if (pi < plen) { c = load8(path_only + pi); }
        if (c == 47 || c == 0) {
            if (seg == n) {
                var slen = pi - ss;
                var out = alloc(slen + 1);
                memcpy(out, path_only + ss, slen);
                store8(out + slen, 0);
                return out;
            }
            seg = seg + 1;
            ss = pi + 1;
        }
        pi = pi + 1;
    }
    return 0;
}

# ================================================================
# Response builders
# ================================================================

# Minimal status response: HTTP/1.1 <code> <msg>\r\nContent-Length: 0\r\nConnection: close\r\n\r\n
fn http_send_status(cfd, code, msg) {
    var sb = str_builder_new();
    str_builder_add_cstr(sb, "HTTP/1.1 ");
    str_builder_add_int(sb, code);
    str_builder_add_cstr(sb, " ");
    str_builder_add_cstr(sb, msg);
    str_builder_add_cstr(sb, "\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    var s = str_data(str_builder_build(sb));
    sock_send(cfd, s, strlen(s));
    return 0;
}

# Full response. extra_headers is a cstr (each line CRLF-terminated) or 0.
fn http_send_response(cfd, code, msg, content_type, body, body_len, extra_headers) {
    var sb = str_builder_new();
    str_builder_add_cstr(sb, "HTTP/1.1 ");
    str_builder_add_int(sb, code);
    str_builder_add_cstr(sb, " ");
    str_builder_add_cstr(sb, msg);
    str_builder_add_cstr(sb, "\r\nContent-Type: ");
    str_builder_add_cstr(sb, content_type);
    str_builder_add_cstr(sb, "\r\nContent-Length: ");
    str_builder_add_int(sb, body_len);
    str_builder_add_cstr(sb, "\r\n");
    if (extra_headers != 0) { str_builder_add_cstr(sb, extra_headers); }
    str_builder_add_cstr(sb, "Connection: close\r\n\r\n");
    var head = str_data(str_builder_build(sb));
    sock_send(cfd, head, strlen(head));
    if (body_len > 0) { sock_send(cfd, body, body_len); }
    return 0;
}

fn http_send_204(cfd, extra_headers) {
    var sb = str_builder_new();
    str_builder_add_cstr(sb, "HTTP/1.1 204 No Content\r\n");
    if (extra_headers != 0) { str_builder_add_cstr(sb, extra_headers); }
    str_builder_add_cstr(sb, "Content-Length: 0\r\nConnection: close\r\n\r\n");
    var s = str_data(str_builder_build(sb));
    sock_send(cfd, s, strlen(s));
    return 0;
}

# ================================================================
# Chunked / streaming responses (for SSE, long-running)
# ================================================================

fn http_send_chunked_start(cfd, code, content_type, extra_headers) {
    var sb = str_builder_new();
    str_builder_add_cstr(sb, "HTTP/1.1 ");
    str_builder_add_int(sb, code);
    str_builder_add_cstr(sb, " OK\r\nContent-Type: ");
    str_builder_add_cstr(sb, content_type);
    str_builder_add_cstr(sb, "\r\nTransfer-Encoding: chunked\r\n");
    if (extra_headers != 0) { str_builder_add_cstr(sb, extra_headers); }
    str_builder_add_cstr(sb, "Connection: close\r\n\r\n");
    var head = str_data(str_builder_build(sb));
    sock_send(cfd, head, strlen(head));
    return 0;
}

fn http_send_chunk(cfd, data, len) {
    var sb = str_builder_new();
    # Hex length + CRLF + data + CRLF
    # str_builder doesn't have add_hex; emit via small helper
    var hexbuf[32];
    var hi = 0;
    var n = len;
    if (n == 0) { store8(&hexbuf, 48); hi = 1; }
    else {
        var tmp[32];
        var ti = 0;
        while (n > 0) {
            var d = n & 15;
            if (d < 10) { store8(&tmp + ti, 48 + d); }
            else { store8(&tmp + ti, 87 + d); }
            n = n >> 4;
            ti = ti + 1;
        }
        var ri = ti;
        while (ri > 0) {
            ri = ri - 1;
            store8(&hexbuf + hi, load8(&tmp + ri));
            hi = hi + 1;
        }
    }
    store8(&hexbuf + hi, 13);
    store8(&hexbuf + hi + 1, 10);
    sock_send(cfd, &hexbuf, hi + 2);
    if (len > 0) { sock_send(cfd, data, len); }
    sock_send(cfd, "\r\n", 2);
    return 0;
}

fn http_send_chunked_end(cfd) {
    sock_send(cfd, "0\r\n\r\n", 5);
    return 0;
}

# ================================================================
# Request reading — Content-Length aware
# ================================================================
#
# Read until either Content-Length is satisfied or peer closes. Single
# sock_recv only reads what's currently buffered; we may need multiple
# recvs if the request spans TCP packets. Returns the total bytes read,
# or 0 - 1 on socket error.
fn http_recv_request(cfd, buf, max) {
    var have = 0;
    var need_body = 0 - 1;   # discovered after we have full headers

    while (have < max) {
        var r = sock_recv(cfd, buf + have, max - have);
        if (is_err_result(r) == 1) { return 0 - 1; }
        var n = payload(r);
        if (n <= 0) {
            # Peer closed; return what we have
            store8(buf + have, 0);
            return have;
        }
        have = have + n;

        # Once we have full headers, read the body to completion.
        if (need_body == 0 - 1) {
            store8(buf + have, 0);
            var bo = http_body_offset(buf, have);
            if (bo >= 0) {
                var clen = http_content_length(buf, have);
                need_body = bo + clen;
            }
        }
        if (need_body != 0 - 1 && have >= need_body) {
            store8(buf + have, 0);
            return have;
        }
    }
    store8(buf + have, 0);
    return have;
}

# ================================================================
# Server lifecycle
# ================================================================

# Bind addr:port, listen, accept-loop. For each connection:
#   read request → call handler_fp(ctx, cfd, buf, bytes) → close cfd.
# Returns 1 on bind/listen failure; never returns on success.
#
# `addr` is a network-order IPv4 (use INADDR_ANY() for 0.0.0.0,
# INADDR_LOOPBACK() for 127.0.0.1).
#
# A 64KB request buffer is allocated lazily on first call and reused.
var _hsv_req_buf = 0;
var HSV_REQ_BUF_SIZE = 65536;

fn http_server_run(addr, port, handler_fp, ctx) {
    if (_hsv_req_buf == 0) { _hsv_req_buf = alloc(HSV_REQ_BUF_SIZE); }

    var sfd_r = tcp_socket();
    if (is_err_result(sfd_r) == 1) { return 1; }
    var sfd = payload(sfd_r);
    sock_reuse(sfd);

    var br = sock_bind(sfd, addr, port);
    if (is_err_result(br) == 1) { return 1; }

    var lr = sock_listen(sfd, 16);
    if (is_err_result(lr) == 1) { return 1; }

    while (1 == 1) {
        var cr = sock_accept(sfd);
        if (is_err_result(cr) == 0) {
            var cfd = payload(cr);
            var n = http_recv_request(cfd, _hsv_req_buf, HSV_REQ_BUF_SIZE - 1);
            if (n > 0) {
                fncall4(handler_fp, ctx, cfd, _hsv_req_buf, n);
            }
            sock_close(cfd);
        }
    }
    return 0;
}
```

---

## Adoption — what changes in bote

Once `lib/http_server.cyr` ships in cyrius 4.5.0:

### `bote/src/transport_http.cyr` shrinks dramatically

```cyr
include "lib/http_server.cyr"

# HttpConfig stays the same (path, addr, port, allowed_origins, etc).

fn _http_handle(config_ctx, cfd, buf, n) {
    var config = config_ctx;     # we passed the HttpConfig in as ctx

    var method = http_get_method(buf, n);
    var path   = http_get_path(buf, n);

    if (streq(method, "POST") == 0) {
        http_send_status(cfd, HTTP_METHOD_NOT_ALLOWED, "Method Not Allowed");
        return 0;
    }
    if (streq(http_path_only(path), http_config_path(config)) == 0) {
        http_send_status(cfd, HTTP_NOT_FOUND, "Not Found");
        return 0;
    }

    # ... origin / protocol / session middleware (unchanged) ...

    var bo = http_body_offset(buf, n);
    if (bo < 0) { http_send_status(cfd, HTTP_BAD_REQUEST, "Bad Request"); return 0; }
    var clen = http_content_length(buf, n);
    if (clen <= 0) { clen = n - bo; }
    var body = alloc(clen + 1);
    memcpy(body, buf + bo, clen);
    store8(body + clen, 0);

    var resp = codec_process_message(body, http_config_dispatcher(config));
    if (resp == 0) { http_send_204(cfd, 0); return 0; }
    http_send_response(cfd, HTTP_OK, "OK", "application/json",
                       resp, strlen(resp), 0);
    return 0;
}

fn transport_http_run(dispatcher, config) {
    return http_server_run(http_config_addr(config),
                           http_config_port(config),
                           &_http_handle,
                           config);    # config carries the dispatcher
}
```

**Net delta in transport_http.cyr**: ~250 LOC removed (everything from
`_http_find` through `_http_send_204` and the bind/listen ceremony is now
in stdlib). Same for bridge.cyr.

### Function-count budget

Currently bote+libro+majra exceeds the cyrius compilation-unit budget
(see v1.2.1 known-issue note). Removing 28 internal HTTP fns from bote
(now in stdlib, shared, counted once) buys back significant headroom and
**should resolve the libro live-integration heisenbug** because the
allocator state at the boundary is no longer cumulatively perturbed.

### What vidya gets

```cyr
include "lib/http_server.cyr"

fn _vidya_handle(ctx, cfd, buf, n) {
    var path = http_get_path(buf, n);
    if (memeq(path, "/info/", 6) == 1) {
        var topic = http_path_segment(path, 1);    # was http_path_segment helper
        # ...
    }
    if (memeq(path, "/search", 7) == 1) {
        var q = http_get_param(path, "q");          # url-decoded for free
        # ...
    }
    # ...
}

fn cmd_serve(port) {
    return http_server_run(INADDR_ANY(), port, &_vidya_handle, 0);
}
```

vidya's `make_crlf`, `http_parse_path`, `http_get_param`, `http_path_segment`,
`http_respond`, `http_ok`, `http_not_found`, `http_bad_request`, and the
whole `cmd_serve` body all collapse to stdlib calls. **~150 LOC saved.**

---

## Open questions for cyrius lang-agent

1. **Naming**: `lib/http_server.cyr` reads cleanly alongside the existing
   `lib/http.cyr` (client). Alternative: rename existing to `http_client.cyr`
   and add `http_server.cyr`. I'd avoid the rename — backward incompat for no
   real win.

2. **`http_recv_request` and `_hsv_req_buf`** — the global request buffer is
   per-process. If a future cyrius adds threading we'll want per-thread
   buffers. For 4.5.0 single-thread is fine.

3. **Keep-alive**: this proposal does `Connection: close` on every response.
   Real keep-alive would let one TCP connection serve many requests. Worth
   doing in 4.6.0 once we have a story for non-blocking accept.

4. **Method enum vs cstr**: kept as cstrs to match the rest of cyrius style.
   Could expose `HTTP_METHOD_GET`, `HTTP_METHOD_POST` as integer constants
   if there's appetite — comparison would be `if (m == HTTP_METHOD_POST)`.

5. **Error propagation**: socket errors silently close the connection. A
   future revision could log via `sakshi_warn` if `lib/sakshi.cyr` is in scope.

---

## Test plan

Once integrated:

1. `bote/src/transport_http.cyr` builds against `lib/http_server.cyr` with
   no behavior change (existing 359 unit tests + 4-transport e2e all pass).
2. `bote/src/bridge.cyr` same.
3. New stdlib tests `lib/http_server.tcyr` cover: status codes, header
   parsing (case-insensitive, missing header, multiple headers), body offset
   (with/without body), content-length parsing, query param (single, multi,
   percent-encoded, missing), path segment (root, deep, out-of-range), URL
   decode (alphanumeric, %XX, +, mixed), chunked response shape.
4. `vidya/src/main.cyr` replaces `cmd_serve` body — confirms the API
   matches both consumers' shapes.

---

## Summary

| | Before | After |
|---|---|---|
| Lines of HTTP plumbing | 600 in bote + 150 in vidya = **750** | **~600** in stdlib (shared) |
| Hand-rolled CRLF helpers | 2 (vidya `make_crlf`, bote pre-4.4.0 `_crlf`) | 0 |
| Per-project request buffer | 2 (bote, bridge) | 1 (stdlib, reused) |
| `http_get_param` available | only vidya | every cyrius project |
| `http_path_segment` available | only vidya | every cyrius project |
| `http_url_decode` available | nowhere | every cyrius project |
| Chunked / SSE responses | nowhere | every cyrius project |
| Content-Length-aware request read | **nowhere** (single recv assumed) | stdlib |
