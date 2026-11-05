// Random root `$id`s and `$ref`s that name a definition through it, each compiled by the
// official MCP client's own validator (AjvJsonSchemaValidator of @modelcontextprotocol/client,
// the Ajv it bundles, its engine for 2020-12 picked as the client picks it), so that bote's
// registration (src/schema.cyr, _sch_refs_why) can be held to the client's reading of which
// references are in the document: scripts/ref-diff.sh.
// (3.8.14) For a root that is no object, the form 2025-11-25 lists it in is compiled too, made by
// the SDK's own wrapOutputSchemaForLegacy from the client's bundle, as registration checks both.
// Usage: node ref_diff_gen.cjs <the client's validators/ajv.cjs> <seed> <count> > cases.txt
// Line format: C\t<schema as JSON>\t<1 the client compiles it, else 0>\t<the same of its
// 2025-11-25 form, or ->.
const fs = require('fs');
const npath = require('path');
const mod = process.argv[2];
let seed = Number(process.argv[3] || 1) >>> 0;
const count = Number(process.argv[4] || 1000);
const { AjvJsonSchemaValidator } = require(mod);
const dist = npath.dirname(npath.dirname(mod));
let wrapSrc = null;
let dialects = null;
for (const f of fs.readdirSync(dist)) {
  if (!f.endsWith('.cjs')) continue;
  const t = fs.readFileSync(npath.join(dist, f), 'utf8');
  const a = t.indexOf('function isNonObjectJsonSchemaRoot(');
  const b = t.indexOf('function wrapOutputSchemaForLegacy(');
  if (a < 0 || b < 0) continue;
  wrapSrc = t.slice(a, t.indexOf('//#endregion', b));
  dialects = require(npath.join(dist, t.match(/const require_dialects = require\('\.\/([^']+)'\);/)[1]));
}
if (wrapSrc === null) { process.stderr.write('ref_diff_gen: no wrapOutputSchemaForLegacy in the client\n'); process.exit(1); }
const { isNonObjectJsonSchemaRoot, wrapOutputSchemaForLegacy } =
  new Function('require_dialects', wrapSrc + '\nreturn { isNonObjectJsonSchemaRoot, wrapOutputSchemaForLegacy };')(dialects);
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
// (3.8.13, from the 3.8.12 review) A wider grammar, the reviewer's: ws and wss, a scheme with no
// authority and a dot path, relative bases, percent-encoded dots, ports and hosts of every edge.
const wschemes = ['http', 'https', 'HTTP', 'x-y', 'file', 'ws', 'wss', 'urn', 'tag', 'mailto', 'a+b', 'a.b'];
const wusers = ['', '', '', 'u@', 'U@', 'u:p@', 'a:@', ':@', 'u%40@', 'u%3a@'];
const whosts = ['x', 'X', 'x.', 'x.y', 'X.Y', 'a-b', 'xn--d', '127.0.0.1', '127.0.0.01', '0.0.0.0', '[::1]', '[::A]', '[::a]', '[FE80::1]', '[fe80::1]',
  '[::ffff:1.2.3.4]', '[::FFFF:1.2.3.4]', '[fe80::1%25eth0]', '[0:0:0:0:0:0:0:1]', '', 'localhost', 'LOCALHOST', '1.2.3', '1', 'x..y', '-x', 'x-', '_x'];
const wports = ['', '', '', ':80', ':443', ':0', ':65535', ':65536', ':00080', ':'];
const wsegs = ['a', 'A', '.', '..', '%2e', '%2E', '%2e%2e', '.%2e', '%2E.', '', 'b.json', '%41', '%7e', '%7E', '~', '...', '.a', 'a.', ';p', 'a;b', '=', '@', ':'];
const wqueries = ['', '', '', '?', '?q', '?Q', '?a/..', '?/../x', '?%41', '?#'];
function wpath(n) { let s = ''; for (let i = 0; i < n; i++) s += '/' + pick(wsegs); return s; }
function wrel(n) { let s = pick(wsegs); for (let i = 1; i < n; i++) s += '/' + pick(wsegs); return s; }
function wbase() {
  const sc = pick(wschemes);
  const k = r(10);
  if (sc === 'urn' || sc === 'tag' || sc === 'mailto') return sc + ':' + pick(['a:b', 'A:B', 'ab:c/d', 'ab:c/../d', 'x,2020:a/b', 'a@b', 'ex:a/./b', 'ex:%41']) + pick(wqueries);
  if (k === 0) return pick(['', '/', '//x', '/a/b', 'a/b', 'a', './a', '../a']) + pick(wqueries);
  if (k === 1) return sc + ':' + wrel(1 + r(3)) + pick(wqueries);
  if (k === 2) return sc + ':' + wpath(1 + r(3)) + pick(wqueries);
  return sc + '://' + pick(wusers) + pick(whosts) + pick(wports) + wpath(r(4)) + pick(wqueries);
}
function wrespell(b) {
  let s = b;
  const k = r(10);
  if (k === 0) s = s.replace(/^[a-z+.-]+/i, (m) => m.toUpperCase());
  else if (k === 1) s = s.replace(/\/\/([^/?#]*)/, (m, a) => '//' + a.toUpperCase());
  else if (k === 2) s = s.replace(/\/\/([^/?#]*)/, (m, a) => '//' + a.toLowerCase());
  else if (k === 3) s = s.replace(/(\/[^/?#]*)(\?|$)/, (m, a, t) => a + pick(['/.', '/..', '/./', '/x/..', '/%2e', '/%2e%2e', '/x/%2e%2e']) + t);
  else if (k === 4) s = s.replace(/%([0-9a-f]{2})/i, (m, h) => '%' + (r(2) ? h.toLowerCase() : h.toUpperCase()));
  else if (k === 5) s = s + pick(['/', '?', '?q', '#']);
  else if (k === 6) s = s.replace(/[?].*$/, '');
  else if (k === 7) s = s.replace(/:(80|443)(\/|\?|$)/, (m, p, t) => t);
  return s;
}
function wref(b) {
  const k = r(10);
  if (k === 0) return '';
  if (k === 1) return wrel(1 + r(3));
  if (k === 2) return wpath(1 + r(3));
  if (k === 3) return pick(['?', '?q', '?Q', '.', './', '..', '../', '../..', '/', '//x', '//X/']);
  if (k === 4) return wbase();
  return wrespell(b);
}
// An `$id` or an anchor that is no string, at the root or below it.
const odd = [5, true, null, {}, [], 'x'];
const out = [];
for (let c = 0; c < count; c++) {
  const wide = r(2) === 0;
  const b = wide ? wbase() : base();
  const f = wide ? wref(b) : ref(b);
  const sch = { $id: b, $defs: { x: { type: 'string' } }, properties: { p: { $ref: f + '#/$defs/x' } } };
  const k = r(20);
  if (k === 0) sch.$id = pick(odd);
  else if (k === 1) sch.$defs.x.$id = pick(odd);
  else if (k === 2) sch.$defs.x.$anchor = pick(odd);
  else if (k === 3) sch.$defs.x.$dynamicAnchor = pick(odd);
  else if (k === 4) sch.$anchor = pick(odd);
  let ok = 0;
  // A validator of its own for each: the client caches a schema by its $id.
  try { new AjvJsonSchemaValidator().getValidator(sch); ok = 1; } catch (e) { ok = 0; }
  let wok = '-';
  if (isNonObjectJsonSchemaRoot(sch)) {
    try { new AjvJsonSchemaValidator().getValidator(wrapOutputSchemaForLegacy(sch)); wok = 1; } catch (e) { wok = 0; }
  }
  out.push('C\t' + JSON.stringify(JSON.stringify(sch)) + '\t' + ok + '\t' + wok);
}
process.stdout.write(out.join('\n') + '\n');
