// Random ECMA-262 patterns and strings, with JavaScript's verdicts under the u flag, which is
// how the official MCP client's Ajv compiles a JSON Schema `pattern` (scripts/regex-diff.sh).
// Usage: [VALID=1] node regex_diff_gen.mjs <seed> <count> > cases.txt
// VALID=1 draws fewer pieces the u flag refuses, longer patterns and longer strings.
// Line format: P\t<pattern as JSON>\t<1 compiles under u, else 0>, then per string
// S\t<string as JSON>\t<RegExp.prototype.test, 0/1>\t<ECMA-262's search, 0/1>. The second is
// JavaScript's own engine run sticky at each code point boundary in turn, as the spec's
// RegExpBuiltinExec tries them under the u flag: V8's unanchored search departs from it on a
// few patterns (`[^a-z]$` refuses "😁", which `^[^a-z]$` takes; `\B` matches between the two
// halves of a surrogate pair), and bote is held to the spec's reading.
let seed = Number(process.argv[2] || 1) >>> 0;
const count = Number(process.argv[3] || 1000);
const VALID = process.env.VALID === '1';
function rnd() { seed = (Math.imul(seed, 1103515245) + 12345) >>> 0; return seed / 4294967296; }
function r(n) { return Math.floor(rnd() * n); }
function pick(a) { return a[r(a.length)]; }
const lits = ['a', 'b', 'c', 'x', 'é', 'ü', '😀', '1', '9', '_', ' ', '-', ',', '/', ':', '!', '=', 'Z'];
const escs = ['\\d', '\\D', '\\w', '\\W', '\\s', '\\S', '\\.', '\\\\', '\\/', '\\-', '\\u0061', '\\u00e9', '\\u{1F600}', '\\x41', '\\cA', '\\0', '\\t', '\\n', '\\r', '\\f', '\\v',
  '\\1', '\\k<n>', '\\p{L}', '\\P{Lu}',
  // (3.8.14) Backreferences that name a group and that do not, and \p in each shape.
  '\\k<n0>', '\\k<n1>', '\\2', '\\10', '\\k', '\\k<>', '\\k<n0', '\\pL', '\\p{L', '\\p{=x}', '\\p{Script=Latin}', '\\p{L_x}', '\\p{}', '\\q', '\\a', '\\ud83d\\ude00', '\\ud800', '\\u{110000}', '\\c1', '\\x4', '\\*', '\\?', '\\(', '\\)', '\\[', '\\]', '\\{', '\\}', '\\|', '\\^', '\\$', '\\+', '\\01'];
const classes = ['[abc]', '[^a-z]', '[\\d\\s]', '[a-\\d]', '[é-ü]', '[😀-😂]', '[\\b]', '[]', '[^]', '[-a]', '[a-]', '[z-a]', '[\\w-]', '[\\-x]', '[a\\]b]', '[[]', '[\\u0061-\\u0063]', '[^\\s\\S]', '[.]', '[\\p{L}]', '[\\1]', '[\\0]', '[\\cZ]', '[a-c-e]', '[\\D]', '[^\\W]'];
function atom(d) {
  const k = r(100);
  if (k < 34) return pick(lits);
  if (k < 52) return pick(escs);
  if (k < 66) return pick(classes);
  if (k < 72) return '.';
  if (k < 74 && !VALID) return pick(['{', '}', ']', '*', '+', '?', ')', '{1}', '(']);
  if (d > 3) return pick(lits);
  const inner = disj(d + 1);
  const g = r(10);
  if (g < 5) return '(' + inner + ')';
  if (g < 7) return '(?:' + inner + ')';
  if (g < 8) return '(?<n' + r(3) + '>' + inner + ')';
  // (3.8.14) Group names and modifiers, as the u flag takes and refuses them.
  if (g < 9 && r(2) == 0) return pick(['(?<1a>', '(?<>', '(?<$x>', '(?<_é>', '(?<\\u0061b>', '(?<a-b>', '(?<a', '(?-:', '(?ii:', '(?x:', '(?m-s:', '(?i-i:', '(?s)', '(?-i:', '(?ims:']) + inner + ')';
  return pick(['(?=', '(?!', '(?<=', '(?<!', '(?i:']) + inner + ')';
}
function quant() {
  const k = r(100);
  if (k < 55) return '';
  const q = pick(['*', '+', '?', '{2}', '{1,3}', '{0,}', '{2,1}', '{,2}', '{3', '{1,2', '{0}', '{0,1}']);
  return q + (r(4) == 0 ? '?' : '') + (r(30) == 0 && !VALID ? '*' : '');
}
function term(d) {
  // (3.8.14) Bounds past bote's, on one literal only: on a group V8's search backtracks through
  // each of 20,000 copies, and one pattern ran for an hour.
  if (r(200) == 0) return 'a' + pick(['{20000,10001}', '{10001,20000}', '{20000}', '{0020001,20000}']);
  const k = r(100);
  if (k < 6) return '^' + (r(10) == 0 ? '*' : '');
  if (k < 12) return '$';
  if (k < 15) return pick(['\\b', '\\B']) + (r(10) == 0 ? '+' : '');
  return atom(d) + quant();
}
function alt(d) { let s = ''; const n = VALID ? 1 + r(6) : r(4); for (let i = 0; i < n; i++) s += term(d); return s; }
function disj(d) { let s = alt(d); while (r(5) == 0) s += '|' + alt(d); return s; }
const alpha = ['a', 'b', 'c', 'x', 'é', 'ü', '😀', '😁', '1', '9', '_', ' ', '-', '\n', '\r', '\t', ' ', 'A', 'Z', '\u0001', ' ', '﻿', '\ud800', '\udc00', '"', '\\', '/', ',', '.', '[', '\u0000'];
function str() { let s = ''; const n = VALID ? r(14) : r(7); for (let i = 0; i < n; i++) s += pick(alpha); return s; }
function spec(re, s) {
  let i = 0;
  for (;;) {
    re.lastIndex = i;
    if (re.test(s)) return 1;
    if (i >= s.length) return 0;
    const c = s.codePointAt(i);
    i += c > 0xffff ? 2 : 1;
  }
}
const out = [];
for (let c = 0; c < count; c++) {
  const p = disj(0);
  let re = null;
  try { re = new RegExp(p, 'u'); } catch (e) { re = null; }
  out.push('P\t' + JSON.stringify(p) + '\t' + (re ? 1 : 0));
  if (re) {
    const sticky = new RegExp(p, 'uy');
    for (let i = 0; i < 10; i++) {
      const s = str();
      out.push('S\t' + JSON.stringify(s) + '\t' + (re.test(s) ? 1 : 0) + '\t' + spec(sticky, s));
    }
  }
}
process.stdout.write(out.join('\n') + '\n');
