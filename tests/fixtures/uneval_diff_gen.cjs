// Random schemas built around `unevaluatedProperties` and `unevaluatedItems` and every keyword that
// evaluates a member or an item beside them (properties, patternProperties, additionalProperties,
// dependentSchemas, prefixItems, items, contains, and allOf, anyOf, oneOf, not, if / then / else
// and $ref around them), and random objects and arrays, each schema compiled and each value
// judged by the official MCP client's own validator (AjvJsonSchemaValidator of
// @modelcontextprotocol/client, its engine picked as the client picks it). For a root that is no
// object the client is shown, for 2025-11-25, the form the SDK's own wrapOutputSchemaForLegacy
// makes, taken from the client's bundle; that form is compiled and judged too. So bote's
// registration and its judgement of a result can be held to the client's: scripts/uneval-diff.sh.
// Usage: node uneval_diff_gen.cjs <the client's validators/ajv.cjs> <seed> <count> <values each> > cases.txt
// Lines as tests/fixtures/dynref_diff_gen.cjs writes them, which tests/fixtures/dynref_diff.cyr reads.
const fs = require('fs');
const path = require('path');
const mod = process.argv[2];
let seed = Number(process.argv[3] || 1) >>> 0;
const count = Number(process.argv[4] || 1000);
const per = Number(process.argv[5] || 8);
const { AjvJsonSchemaValidator } = require(mod);

const dist = path.dirname(path.dirname(mod));
let wrapSrc = null;
let dialects = null;
for (const f of fs.readdirSync(dist)) {
  if (!f.endsWith('.cjs')) continue;
  const t = fs.readFileSync(path.join(dist, f), 'utf8');
  const a = t.indexOf('function isNonObjectJsonSchemaRoot(');
  const b = t.indexOf('function wrapOutputSchemaForLegacy(');
  if (a < 0 || b < 0) continue;
  wrapSrc = t.slice(a, t.indexOf('//#endregion', b));
  dialects = require(path.join(dist, t.match(/const require_dialects = require\('\.\/([^']+)'\);/)[1]));
}
if (wrapSrc === null) { process.stderr.write('uneval_diff_gen: no wrapOutputSchemaForLegacy in the client\n'); process.exit(1); }
const { isNonObjectJsonSchemaRoot, wrapOutputSchemaForLegacy } =
  new Function('require_dialects', wrapSrc + '\nreturn { isNonObjectJsonSchemaRoot, wrapOutputSchemaForLegacy };')(dialects);

function rnd() { seed = (Math.imul(seed, 1103515245) + 12345) >>> 0; return seed / 4294967296; }
function r(n) { return Math.floor(rnd() * n); }
function pick(a) { return a[r(a.length)]; }
const S = {
  2020: 'https://json-schema.org/draft/2020-12/schema',
  2019: 'https://json-schema.org/draft/2019-09/schema',
  7: 'http://json-schema.org/draft-07/schema#',
};
let D = 2020;
function leaf() {
  return pick([{ type: 'string' }, { type: 'integer' }, {}, { type: 'object' }, { type: 'array' }, true, false, { required: ['a'] },
    { minItems: 1 }, { unevaluatedProperties: false }, { unevaluatedItems: false }, { properties: { a: {} } }]);
}
function uval(depth) { return pick([false, false, true, { type: 'integer' }, node(depth - 1)]); }
function node(depth) {
  if (depth <= 0 || r(6) === 0) return leaf();
  const o = {};
  const nk = 1 + r(4);
  for (let i = 0; i < nk; i++) {
    const k = r(22);
    if (k === 0) o.type = pick(['object', 'array', 'integer', ['object', 'array'], ['object', 'integer']]);
    else if (k === 1 || k === 2) { o.properties = {}; for (const key of ['a', 'b']) if (r(2)) o.properties[key] = node(depth - 1); }
    else if (k === 3) o.patternProperties = pick([{ '^a': node(depth - 1) }, { '^[bc]': node(depth - 1) }, { 'x': {} }]);
    else if (k === 4) o.additionalProperties = pick([true, false, {}, node(depth - 1)]);
    else if (k === 5) o.required = pick([['a'], ['b'], ['a', 'c']]);
    else if (k === 6) o.dependentSchemas = { [pick(['a', 'b'])]: node(depth - 1) };
    else if (k === 7 || k === 8) o.unevaluatedProperties = uval(depth);
    // 2019-09's and draft-07's array `items` and `additionalItems` bote does not judge yet.
    else if (k === 9) { if (D === 2020) o.prefixItems = [node(depth - 1), node(depth - 1)].slice(0, 1 + r(2)); }
    else if (k === 10) o.items = node(depth - 1);
    else if (k === 11 && r(3) === 0) o.contains = node(depth - 1);
    else if (k === 12 || k === 13) o.unevaluatedItems = uval(depth);
    else if (k === 14) o.allOf = [node(depth - 1), node(depth - 1)].slice(0, 1 + r(2));
    else if (k === 15) o.anyOf = [node(depth - 1), node(depth - 1)];
    else if (k === 16) o.oneOf = [node(depth - 1), node(depth - 1)];
    else if (k === 17) o.not = node(depth - 1);
    else if (k === 18) { o.if = node(depth - 1); if (r(3)) o.then = node(depth - 1); if (r(3)) o.else = node(depth - 1); }
    else if (k === 19) o.$ref = pick(['#/$defs/d', '#/$defs/e']);
    else if (k === 21) o.minProperties = 1;
  }
  return o;
}
function val(depth) {
  const k = r(depth > 0 ? 9 : 4);
  if (k === 0) return r(3);
  if (k === 1) return pick(['x', '']);
  if (k === 2) return pick([true, null]);
  if (k === 3) return 1.5;
  if (k === 4 || k === 5) { const a = []; const n = r(4); for (let i = 0; i < n; i++) a.push(val(depth - 1)); return a; }
  const o = {};
  for (const key of ['a', 'b', 'c', 'xa']) if (r(2)) o[key] = val(depth - 1);
  return o;
}
function compiles(s) {
  try { return new AjvJsonSchemaValidator().getValidator(s); } catch (e) { return null; }
}
function verdict(v, x) {
  try { return v(x).valid ? 1 : 0; } catch (e) { return 2; }
}
const out = [];
for (let c = 0; c < count; c++) {
  D = pick([2020, 2020, 2020, 2019, 7]);
  let root = node(3);
  if (typeof root !== 'object') root = {};
  if (D !== 2020 || r(4) === 0) root.$schema = S[D];
  if (r(2) === 0) root.$defs = { d: node(2), e: node(2) };
  if (r(3) === 0) root.unevaluatedProperties = uval(2);
  if (r(4) === 0) root.unevaluatedItems = uval(2);
  if (r(3) === 0) root.type = pick(['object', 'array', ['object', 'array']]);
  const v0 = compiles(root);
  const non = isNonObjectJsonSchemaRoot(root);
  let w = null, v1 = null;
  if (non) { w = wrapOutputSchemaForLegacy(root); v1 = compiles(w); }
  out.push('S\t' + JSON.stringify(JSON.stringify(root)) + '\t' + (v0 ? 1 : 0) + '\t' + (non ? (v1 ? 1 : 0) : '-') + '\t' + (non ? JSON.stringify(JSON.stringify(w)) : '-'));
  if (!v0) continue;
  for (let i = 0; i < per; i++) {
    const x = val(3);
    out.push('V\t' + JSON.stringify(JSON.stringify(x)) + '\t' + verdict(v0, x) + '\t' + (v1 ? verdict(v1, { result: x }) : '-'));
  }
}
process.stdout.write(out.join('\n') + '\n');
