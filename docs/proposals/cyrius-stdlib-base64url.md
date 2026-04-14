# Proposal: `base64url_encode` / `base64url_decode` for `lib/base64.cyr`

> **Status**: design + reference implementation, ready for cyrius
> lang-agent to fold into 4.8.0.
>
> **Why**: bote 2.2.x needs JWT (HS256) verification for the
> `auth_validator_jwt_hs256` validator. JWT tokens are
> `<base64url(header)>.<base64url(payload)>.<base64url(sig)>` per
> RFC 7515 §3.5 — base64url, not standard base64. Today bote
> would have to ship its own ~50 LOC URL-variant decoder; with this
> in stdlib, every consumer that needs JWT, OAuth 2.0 PKCE / state,
> capability URLs, or any URL-safe binary payload gets it for free.
>
> **Sister APIs already in lib/base64.cyr**: `base64_encode`,
> `base64_decode`. This proposal extends with the URL-safe variant
> per RFC 4648 §5.
>
> **Why not bote-local**: every AGNOS auth path will need this
> (vidya, jalwa, shruti for OAuth 2.0; bote for JWT). It's ~70 LOC
> total. Belongs alongside its sibling.

---

## Surface

```cyr
# RFC 4648 §5 URL-safe base64 ("base64url"). Differences vs standard:
#   - alphabet:  `+` → `-`, `/` → `_`
#   - padding `=` typically omitted; encoder omits it, decoder
#     accepts both forms

# Encode buf[0..len] to base64url (no padding). Returns null-
# terminated cstr. Output length is exactly ceil(len*4/3).
fn base64url_encode(buf, len) → cstr

# Decode base64url cstr (with or without `=` padding) of length
# enc_len. Returns {data_ptr, decoded_len} as a 16-byte alloc'd
# pair, same shape as base64_decode. Returns 0 on invalid input.
fn base64url_decode(encoded, enc_len) → {ptr, len} | 0
```

Mirrors `base64_encode` / `base64_decode` exactly so consumers can
drop one in for the other based on transport context.

---

## Reference implementation

Drop into `lib/base64.cyr` after the existing `base64_decode`
function. Adds one shared `_b64u_enc` constant near the top.

```cyr
# Add near the existing _b64_enc constant:
var _b64u_enc = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

# Encode buf[0..len] to base64url (no padding). Returns null-terminated
# cstr. Output length is exactly ceil(len*4/3) bytes plus the NUL.
fn base64url_encode(buf, len) {
    var n_full = (len / 3) * 4;
    var rem = len % 3;
    var out_len = n_full;
    if (rem == 1) { out_len = out_len + 2; }
    if (rem == 2) { out_len = out_len + 3; }
    var out = alloc(out_len + 1);
    var si = 0;
    var di = 0;
    while (si + 3 <= len) {
        var b0 = load8(buf + si);
        var b1 = load8(buf + si + 1);
        var b2 = load8(buf + si + 2);
        store8(out + di,     load8(_b64u_enc + (b0 >> 2)));
        store8(out + di + 1, load8(_b64u_enc + ((b0 & 3) << 4 | (b1 >> 4))));
        store8(out + di + 2, load8(_b64u_enc + ((b1 & 15) << 2 | (b2 >> 6))));
        store8(out + di + 3, load8(_b64u_enc + (b2 & 63)));
        si = si + 3;
        di = di + 4;
    }
    if (rem == 1) {
        var t0 = load8(buf + si);
        store8(out + di,     load8(_b64u_enc + (t0 >> 2)));
        store8(out + di + 1, load8(_b64u_enc + ((t0 & 3) << 4)));
    }
    if (rem == 2) {
        var u0 = load8(buf + si);
        var u1 = load8(buf + si + 1);
        store8(out + di,     load8(_b64u_enc + (u0 >> 2)));
        store8(out + di + 1, load8(_b64u_enc + ((u0 & 3) << 4 | (u1 >> 4))));
        store8(out + di + 2, load8(_b64u_enc + ((u1 & 15) << 2)));
    }
    store8(out + out_len, 0);
    return out;
}

# Decode base64url cstr (with or without `=` padding). Returns
# {data_ptr, decoded_len} as a 16-byte alloc'd pair, same shape as
# base64_decode. Returns 0 on invalid input character.
fn base64url_decode(encoded, enc_len) {
    var dtbl = alloc(256);
    memset(dtbl, 255, 256);
    var i = 0;
    while (i < 64) {
        store8(dtbl + load8(_b64u_enc + i), i);
        i = i + 1;
    }
    # Strip trailing padding if present (compatibility with
    # implementations that include it despite the spec).
    var elen = enc_len;
    while (elen > 0 && load8(encoded + elen - 1) == 61) { elen = elen - 1; }

    var quads = elen / 4;
    var rem = elen - (quads * 4);
    var out_len = quads * 3;
    if (rem == 2) { out_len = out_len + 1; }
    if (rem == 3) { out_len = out_len + 2; }
    var out = alloc(out_len + 1);
    var si = 0;
    var di = 0;
    while (si + 4 <= elen) {
        var a = load8(dtbl + load8(encoded + si));
        var b = load8(dtbl + load8(encoded + si + 1));
        var c = load8(dtbl + load8(encoded + si + 2));
        var d = load8(dtbl + load8(encoded + si + 3));
        if (a == 255 || b == 255 || c == 255 || d == 255) { return 0; }
        store8(out + di,     ((a << 2) | (b >> 4)) & 0xFF);
        store8(out + di + 1, (((b & 15) << 4) | (c >> 2)) & 0xFF);
        store8(out + di + 2, (((c & 3) << 6) | d) & 0xFF);
        si = si + 4;
        di = di + 3;
    }
    if (rem == 2) {
        var ta = load8(dtbl + load8(encoded + si));
        var tb = load8(dtbl + load8(encoded + si + 1));
        if (ta == 255 || tb == 255) { return 0; }
        store8(out + di, ((ta << 2) | (tb >> 4)) & 0xFF);
    }
    if (rem == 3) {
        var ua = load8(dtbl + load8(encoded + si));
        var ub = load8(dtbl + load8(encoded + si + 1));
        var uc = load8(dtbl + load8(encoded + si + 2));
        if (ua == 255 || ub == 255 || uc == 255) { return 0; }
        store8(out + di,     ((ua << 2) | (ub >> 4)) & 0xFF);
        store8(out + di + 1, (((ub & 15) << 4) | (uc >> 2)) & 0xFF);
    }
    store8(out + out_len, 0);
    var result = alloc(16);
    store64(result, out);
    store64(result + 8, out_len);
    return result;
}
```

Total: ~70 LOC, 2 public fns, 1 private constant.

---

## Test plan

In a future `tests/base64.tcyr` (or extending what already exists):

```cyr
# Round-trip
var encoded = base64url_encode("hello world", 11);
assert(streq(encoded, "aGVsbG8gd29ybGQ") == 1, "round-trip encode");
var decoded = base64url_decode(encoded, strlen(encoded));
assert(load64(decoded + 8) == 11, "round-trip decode len");

# RFC 4648 §10 test vectors (encoded values use URL alphabet):
#   ""     → ""
#   "f"    → "Zg"
#   "fo"   → "Zm8"
#   "foo"  → "Zm9v"
#   "foob" → "Zm9vYg"
#   "fooba"→ "Zm9vYmE"
#   "foobar"→"Zm9vYmFy"

# JWT real-world example (RFC 7515 §3.5):
#   header b64u: "eyJ0eXAiOiJKV1QiLA0KICJhbGciOiJIUzI1NiJ9"
#   decoded:     '{"typ":"JWT",\r\n "alg":"HS256"}'
var hdr = base64url_decode("eyJ0eXAiOiJKV1QiLA0KICJhbGciOiJIUzI1NiJ9", 40);
assert(load64(hdr + 8) == 30, "JWT header decoded length");

# URL-safety: encoded output contains no `+` or `/` characters
var with_special = base64url_encode("\xfa\xfb\xfc", 3);
# would-be standard base64: "+vv8" → URL form: "-vv8"
assert(load8(with_special) == 45, "first byte is - (URL-safe), not +");

# Padding tolerance on decode
var padded = base64url_decode("Zm9vYg==", 8);
assert(load64(padded + 8) == 4, "padded decode works");
var unpadded = base64url_decode("Zm9vYg", 6);
assert(load64(unpadded + 8) == 4, "unpadded decode works");

# Invalid char → 0
assert(base64url_decode("Zm9*Yg", 6) == 0, "invalid char rejected");
```

---

## Adoption — bote 2.2.x

Once 4.8.0 ships:

```cyr
# Drop ~50 LOC from src/jwt.cyr — replace
#   _jwt_b64u_val + jwt_b64u_decode
# with a single call to base64url_decode. Saves the inline byte-table
# build and the per-call decode loop.
var pair = base64url_decode(token, dot1);
var header = load64(pair);
var hlen   = load64(pair + 8);
```

About 7 fns / 80 LOC dropped from `src/jwt.cyr`'s compile-unit
footprint — meaningful given bote's known cap pressure
(`docs/bugs/cyrius-4.5.1-identifier-buffer-cap.md`).

---

## Summary

|  | Before | After |
|---|---|---|
| `lib/base64.cyr` size | 67 LOC, 2 fns | ~140 LOC, 4 fns |
| URL-safe encode/decode | every consumer ports their own | one stdlib path |
| bote `src/jwt.cyr` LOC | ~250 (with inline base64url) | ~170 (using stdlib) |
| AGNOS auth ergonomics | each project reinvents | RFC 4648 §5 just works |
