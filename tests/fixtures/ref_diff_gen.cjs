// Random root `$id`s and `$ref`s that name a definition through it, each compiled by the
// official MCP client's own validator (AjvJsonSchemaValidator of @modelcontextprotocol/client,
// the Ajv it bundles, its engine for 2020-12 picked as the client picks it), so that bote's
// registration (src/schema.cyr, _sch_refs_why) can be held to the client's reading of which
// references are in the document: scripts/ref-diff.sh.
// Usage: node ref_diff_gen.cjs <the client's validators/ajv.cjs> <seed> <count> > cases.txt
// Line format: C\t<schema as JSON>\t<1 the client compiles it, else 0>.
const mod = process.argv[2];
let seed = Number(process.argv[3] || 1) >>> 0;
const count = Number(process.argv[4] || 1000);
const { AjvJsonSchemaValidator } = require(mod);
function rnd() { seed = (Math.imul(seed, 1103515245) + 12345) >>> 0; return seed / 4294967296; }
function r(n) { return Math.floor(rnd() * n); }
function pick(a) { return a[r(a.length)]; }
const schemes = ['http', 'HTTP', 'https', 'Https', 'urn', 'tag', 'x-y', 'file'];
const users = ['', '', '', 'u@', 'U@', 'u:p@', 'U:P@', 'a%41@'];
const hosts = ['x', 'X', 'x.y', 'X.y', '127.0.0.1', '[::1]', '[::A]', 'x%41', '', 'x y', '[::g]', '[1:2:3:4:5:6:7:8:9]', 'é', 'x_y', '999.1.1.1', '[v1.x]'];
const ports = ['', '', '', ':80', ':443', ':8080', ':', ':080', ':99999', ':-1', ':a'];
const segs = ['a', 'b', 'A', '.', '..', '%7E', '~', '%41', '%61', '', 'c.json', '%2f', '%2F', 'a%2Fb', 'é', '%zz', 'a b', '[x]', 'a:b', '%', '|'];
const queries = ['', '', '', '?q', '?Q', '?a/../b', '?', '?%41', '?a/./b', '?a b', '?%z', '?[x]'];
function path(n) { let s = ''; for (let i = 0; i < n; i++) s += '/' + pick(segs); return s; }
function rel(n) { let s = pick(segs); for (let i = 1; i < n; i++) s += '/' + pick(segs); return s; }
function auth() { return '//' + pick(users) + pick(hosts) + pick(ports); }
function base() {
  const sc = pick(schemes);
  if (sc === 'urn' || sc === 'tag') return sc + ':' + pick(['a:b', 'A:b', 'a:b/c', 'x', 'a:b/../c', 'ab:c', 'Ab:C', 'ab:c/.', 'ab:c/./d', 'ab:c/..', 'ab:c%41', 'uuid:f81d4fae-7dec-11d0-a765-00a0c91e6bf6', 'uuid:12', '-a:b', 'a-:b', 'ab:', 'ab:é', 'abcdefghijklmnopqrstuvwxyz0123456:x', 'abcdefghijklmnopqrstuvwxyz012345:x']) + pick(queries);
  return sc + ':' + auth() + path(r(4)) + pick(queries);
}
// A spelling of `b` the client may or may not read as the same resource.
function respell(b) {
  let s = b;
  const k = r(8);
  if (k === 0) s = s.replace(/^[a-z]+/i, (m) => m.toUpperCase());
  else if (k === 1) s = s.replace(/\/\/([^/?]*)/, (m, a) => '//' + a.toUpperCase());
  else if (k === 2) s = s.replace(/\/\/([^/?:@]*)(\/|\?|$)/, (m, a, t) => '//' + a + pick([':80', ':443', ':']) + t);
  else if (k === 3) s = s.replace(/(\/[^/?]*)(\?|$)/, (m, a, t) => a + pick(['/.', '/..', '/./', '/x/..']) + t);
  else if (k === 4) s = s.replace(/a/, '%61');
  else if (k === 5) s = s.replace(/%([0-9a-f]{2})/i, (m, h) => '%' + h.toLowerCase());
  else if (k === 6) s = s + pick(['/', '?', '?q']);
  return s;
}
function ref(b) {
  const k = r(9);
  if (k === 0) return '';
  if (k === 1) return rel(1 + r(3));
  if (k === 2) return path(1 + r(3));
  if (k === 3) return pick(queries.filter((q) => q !== '')) ;
  if (k === 4) return auth() + path(r(3));
  if (k === 5) return base();
  return respell(b);
}
const out = [];
for (let c = 0; c < count; c++) {
  const b = base();
  const f = ref(b);
  const sch = { $id: b, $defs: { x: { type: 'string' } }, properties: { p: { $ref: f + '#/$defs/x' } } };
  let ok = 0;
  // A validator of its own for each: the client caches a schema by its $id.
  try { new AjvJsonSchemaValidator().getValidator(sch); ok = 1; } catch (e) { ok = 0; }
  out.push('C\t' + JSON.stringify(JSON.stringify(sch)) + '\t' + ok);
}
process.stdout.write(out.join('\n') + '\n');
