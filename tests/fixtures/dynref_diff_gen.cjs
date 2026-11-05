// Random schemas built around references, static and dynamic (`$ref`, `$dynamicRef`,
// `$recursiveRef`, their anchors and `$id`s), and random values, each schema compiled and each
// value judged by the official MCP client's own validator (AjvJsonSchemaValidator of
// @modelcontextprotocol/client, the Ajv it bundles, its engine picked as the client picks it).
// For a schema whose root is no object the client is shown, for 2025-11-25, the form the SDK's
// own wrapOutputSchemaForLegacy makes, taken from the client's bundle and not written again
// here; that form is compiled and judged too. So bote's registration and its judgement of a
// result can be held to the client's: scripts/dynref-diff.sh.
// Usage: node dynref_diff_gen.cjs <the client's validators/ajv.cjs> <seed> <count> <values each> > cases.txt
// Lines: S\t<schema>\t<1 the client compiles it, else 0>\t<the same of its 2025-11-25 form, or ->\t<that form, or ->
// then V\t<value>\t<1 the client takes it, 0 not, 2 it throws>\t<the same against the 2025-11-25 form, or ->
// for each value. Schemas, forms and values are each a JSON string holding the JSON.
const fs = require('fs');
const path = require('path');
const mod = process.argv[2];
let seed = Number(process.argv[3] || 1) >>> 0;
const count = Number(process.argv[4] || 1000);
const per = Number(process.argv[5] || 8);
const { AjvJsonSchemaValidator } = require(mod);

// The SDK's wrapOutputSchemaForLegacy, from the bundle the client ships.
const dist = path.dirname(path.dirname(mod));
let wrapSrc = null;
let dialects = null;
for (const f of fs.readdirSync(dist)) {
  if (!f.endsWith('.cjs')) continue;
  const t = fs.readFileSync(path.join(dist, f), 'utf8');
  const a = t.indexOf('function isNonObjectJsonSchemaRoot(');
  const b = t.indexOf('function wrapOutputSchemaForLegacy(');
  if (a < 0 || b < 0) continue;
  const e = t.indexOf('//#endregion', b);
  wrapSrc = t.slice(a, e);
  const m = t.match(/const require_dialects = require\('\.\/([^']+)'\);/);
  dialects = require(path.join(dist, m[1]));
}
if (wrapSrc === null) { process.stderr.write('dynref_diff_gen: no wrapOutputSchemaForLegacy in the client\n'); process.exit(1); }
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
function leaf() {
  return pick([{ type: 'string' }, { type: 'integer' }, {}, { type: 'array' }, { type: 'object' }, { minItems: 1 },
    { required: ['a'] }, true, false, { type: ['integer', 'null'] }]);
}
// A schema of up to `depth` levels. `tame` 1 draws only the references the one reading both
// take (a `$dynamicRef: "#s"` and a `$recursiveRef: "#"`, to anchors on the root) and keeps
// out `$id`s and other anchors, so that most such schemas are taken and their values judged.
function node(depth, tame) {
  if (depth <= 0 || r(5) === 0) return leaf();
  const o = {};
  const nk = 1 + r(3);
  for (let i = 0; i < nk; i++) {
    const k = r(17);
    if (k === 0) o.type = pick(['object', 'array', 'integer', 'string', ['array', 'integer'], ['object', 'integer'], ['object', 'array']]);
    else if (k === 1) { o.properties = { a: node(depth - 1, tame) }; if (r(2)) o.properties.b = node(depth - 1, tame); }
    else if (k === 2) o.required = pick([['a'], ['b'], ['a', 'b']]);
    else if (k === 3) o.items = node(depth - 1, tame);
    else if (k === 4) o.minItems = 1 + r(2);
    else if (k === 5) o.anyOf = [node(depth - 1, tame), node(depth - 1, tame)];
    else if (k === 6) o.allOf = [node(depth - 1, tame)];
    else if (k === 7) o.not = node(depth - 1, tame);
    else if (k === 8 || k === 9) o.$dynamicRef = tame ? '#s' : pick(['#s', '#s', '#s', '#t', '#', '#/$defs/d', 'x#s', 5, '#a%62']);
    else if (k === 10) o.$recursiveRef = tame ? '#' : pick(['#', '#', '#s', 7]);
    else if (k === 11) o.$ref = tame ? pick(['#/$defs/d', '#/$defs/e']) : pick(['#/$defs/d', '#/$defs/e', '#t', '#', '#/properties/a', 'https://ex.com/r#/$defs/d']);
    else if (k === 12 && !tame) o.$dynamicAnchor = pick(['s', 't', 't', 5]);
    else if (k === 13) o.$recursiveAnchor = tame ? true : pick([true, false, 1]);
    else if (k === 14 && !tame) o.$anchor = pick(['t', 'u', 's']);
    else if (k === 15 && !tame && r(3) === 0) o.$id = pick(['https://ex.com/n', 'n2', '#frag', '']);
    else if (k === 16) o.additionalProperties = node(depth - 1, tame);
  }
  return o;
}
function val(depth) {
  const k = r(depth > 0 ? 8 : 4);
  if (k === 0) return r(3);
  if (k === 1) return pick(['x', '']);
  if (k === 2) return pick([true, null, 1.5]);
  if (k === 3) return pick([0, 'y']);
  if (k === 4 || k === 5) { const a = []; const n = r(4); for (let i = 0; i < n; i++) a.push(val(depth - 1)); return a; }
  const o = {};
  for (const key of ['a', 'b', 'c']) if (r(2)) o[key] = val(depth - 1);
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
  const tame = r(3) === 0 ? 1 : 0;
  let root = node(3, tame);
  if (typeof root !== 'object') root = {};
  const d = pick([2020, 2020, 2019, 2019, 7, 0]);
  if (d !== 0) root.$schema = S[d];
  if (r(3) !== 0) root.$dynamicAnchor = tame ? 's' : pick(['s', 's', 't']);
  if (r(2) === 0) root.$recursiveAnchor = true;
  if (r(3) !== 0) root.$defs = { d: node(2, tame), e: node(2, tame) };
  if (!tame && r(6) === 0) root.$id = pick(['https://ex.com/r', '', null, 'a', 0]);
  if (r(2) === 0) root.type = pick(['array', 'integer', ['array', 'integer'], ['object', 'array'], 'object']);
  const v0 = compiles(root);
  const non = isNonObjectJsonSchemaRoot(root);
  let w = null, v1 = null;
  if (non) { w = wrapOutputSchemaForLegacy(root); v1 = compiles(w); }
  out.push('S\t' + JSON.stringify(JSON.stringify(root)) + '\t' + (v0 ? 1 : 0) + '\t' + (non ? (v1 ? 1 : 0) : '-') + '\t' + (non ? JSON.stringify(JSON.stringify(w)) : '-'));
  if (!v0) continue;
  for (let i = 0; i < per; i++) {
    const x = val(4);
    out.push('V\t' + JSON.stringify(JSON.stringify(x)) + '\t' + verdict(v0, x) + '\t' + (v1 ? verdict(v1, { result: x }) : '-'));
  }
}
process.stdout.write(out.join('\n') + '\n');
