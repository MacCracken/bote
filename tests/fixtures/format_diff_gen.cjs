// Strings and numbers near each format's edges, each judged by the official MCP client's own
// validator (AjvJsonSchemaValidator of @modelcontextprotocol/client: the Ajv and ajv-formats it
// bundles, as it makes them), so that src/format.cyr can be held to the client's reading of
// every format it judges: scripts/format-diff.sh.
// Usage: node format_diff_gen.cjs <the client's validators/ajv.cjs> <seed> <per format> > cases.txt
// Line format: F\t<format>\t<value as JSON>\t<1 the client takes it, else 0>.
const mod = process.argv[2];
let seed = Number(process.argv[3] || 1) >>> 0;
const per = Number(process.argv[4] || 1000);
const { AjvJsonSchemaValidator } = require(mod);
function rnd() { seed = (Math.imul(seed, 1103515245) + 12345) >>> 0; return seed / 4294967296; }
function r(n) { return Math.floor(rnd() * n); }
function pick(a) { return a[r(a.length)]; }
const seeds = {
  date: ['2020-01-31', '2020-02-29', '2019-02-28', '0000-01-01', '1999-12-31', '2000-02-29', '1900-02-28', '2021-04-30'],
  time: ['12:34:56Z', '23:59:60Z', '00:00:00+01:00', '12:34:56.789-05:30', '12:00:00+0530', '12:00:00+05', '23:59:60.5+00:00',
    '00:59:60+01:00', '23:59:59.999999999999996447286321199499070644378662109375z', '12:00:59.9999999999999999Z', '22:59:60-01:00'],
  'date-time': ['2020-01-01T12:00:00Z', '2020-01-01t12:00:00z', '2020-01-01 12:00:00+01:00', '2016-12-31T23:59:60Z', '2020-02-29T00:00:00.5-12:00'],
  'iso-time': ['12:34:56', '12:34:56Z', '23:59:60', '01:02:03.4+0530'],
  'iso-date-time': ['2020-01-01T12:00:00', '2020-01-01 12:00:00Z', '2020-01-01t00:00:00.1'],
  duration: ['P1Y2M3DT4H5M6S', 'P1W', 'PT1H', 'P1D', 'PT1M30S', 'P12M', 'PT0S', 'P1Y'],
  uri: ['http://example.com/a?b#c', 'urn:isbn:123', 'mailto:a@b.c', 'https://[::1]:8080/x', 'file:///etc/passwd', 'a:b',
    'http://[v1.x]/', 'http://1.2.3.4:/', 'x:/y', 'http://u:p@h/%41?q=1#f/?', 'h:/'],
  'uri-reference': ['//example.com', '/a/b', 'a/b', '?q', '#f', '', 'http://x', '../a', 'a:b/c', '//[::1]'],
  'uri-template': ['http://example.com/{id}', '/a{/b,c}{?q*}', '{+x:3}', '%20', '{#a,b:12}', 'x', '{a_b}'],
  url: ['http://example.com', 'https://a.b.co/x?y', 'ftp://1.2.3.4:21/f', 'http://user:pass@host.com:8080/p', 'http://xn--d1abbgf6aiiy.xn--p1ai',
    'http://例子.测试', 'HTTPS://A.BC', 'http://a@b@c.de/', 'http://10.0.0.1', 'http://172.16.0.1', 'http://172.15.0.1', 'http://100.1.2.3', 'http://a.b-c.de'],
  email: ['a@b.co', 'a.b+c@d-e.f.gh', "!#$%&'*+/=?^_`{|}~-@x.y", 'A@B.C', 'a@b'],
  hostname: ['example.com', 'a', 'a-b.c', 'a.', 'x'.repeat(63) + '.y', ('a'.repeat(63) + '.').repeat(3) + 'a'.repeat(61), 'A1.b2', '1.2.3.4'],
  ipv4: ['1.2.3.4', '255.255.255.255', '0.0.0.0', '10.1.1.1'],
  ipv6: ['::1', '1:2:3:4:5:6:7:8', '::ffff:1.2.3.4', 'fe80::1', '1::', '1:2:3:4:5:6:7::', '::', 'ABCD::ef'],
  uuid: ['123e4567-e89b-12d3-a456-426614174000', 'urn:uuid:123e4567-e89b-12d3-a456-426614174000', 'URN:UUID:123E4567-E89B-12D3-A456-426614174000'],
  'json-pointer': ['', '/a/b', '/a~0b~1c', '/', '/é/😀'],
  'json-pointer-uri-fragment': ['#', '#/a/%20', '#/~0', '#/A/b'],
  'relative-json-pointer': ['0', '1/a', '2#', '0/~1', '10'],
  byte: ['', 'QUJD', 'QQ==', 'QUI=', '!!!\n', 'QUJDRA==', 'ab\r\ncd'],
  password: ['x', ''],
  binary: ['x', ''],
};
const pool = ['0', '1', '2', '5', '9', 'a', 'b', 'f', 'g', 'z', 'Z', 't', 'T', 'P', 'W', 'Y', 'M', 'D', 'H', 'S', 'e', 'x', 'X', 'v', 'V',
  '-', ':', '.', '/', '?', '#', '[', ']', '@', '%', '~', '+', '=', '_', '{', '}', ',', '*', '!', '$', '&', "'", '(', ')', ';', '|', '^', '`',
  '"', '<', '>', '\\', ' ', 'é', 'ſ', 'K', '😀', '\u00a0', '\u3000', '\u2028', '\ufeff', '\u0000', '\n', '\r', '\t', '\ud800', '\udc00', '\ufffd', '%2', '%zz', '::', '..', '//'];
function mutate(s) {
  const k = 1 + r(3);
  for (let i = 0; i < k; i++) {
    const cps = [...s];
    const at = r(cps.length + 1);
    const m = r(6);
    if (m === 0) cps.splice(at, 0, pick(pool));
    else if (m === 1 && cps.length) cps.splice(Math.min(at, cps.length - 1), 1);
    else if (m === 2 && cps.length) cps[Math.min(at, cps.length - 1)] = pick(pool);
    else if (m === 3 && cps.length) { const j = Math.min(at, cps.length - 1); cps.splice(j, 0, cps[j]); }
    else if (m === 4) { const t = cps.join(''); return r(2) ? t.toUpperCase() : t.toLowerCase(); }
    else if (m === 5 && cps.length > 1) { const j = r(cps.length - 1); [cps[j], cps[j + 1]] = [cps[j + 1], cps[j]]; }
    s = cps.join('');
  }
  return s;
}
// Each format's own alphabet, for strings drawn from it whole.
const alpha = {
  date: '0123456789-', time: '0123456789:.+-zZ', 'date-time': '0123456789:.+-zZtT \u3000', 'iso-time': '0123456789:.+-zZ',
  'iso-date-time': '0123456789:.+-zZtT ', duration: 'PYMWDTHS0123', uri: 'ab:/?#[]@%1.v-', 'uri-reference': 'ab:/?#[]@%1.v-',
  'uri-template': 'a{}+#,:*%1 ', url: 'htpsf:/@.a1-é\u3000 ', email: 'a1.-@+_', hostname: 'a0-.Z', ipv4: '0125.', ipv6: '0af:.F1',
  uuid: '0aF-urn:id', 'json-pointer': '/~01a', 'json-pointer-uri-fragment': '#/~01a%', 'relative-json-pointer': '0123#/~a',
  byte: 'QUJD=+/!\n\r', password: 'x', binary: 'x',
};
// A hostname of `n` characters, in labels of at most `lab`, perhaps ending in a dot.
function host(n, lab, dot) {
  let s = '';
  while (s.length < n) { const k = Math.min(lab, n - s.length); s += 'a'.repeat(k); if (s.length < n) s += '.'; }
  return dot ? s + '.' : s;
}
const nums = ['0', '-0', '1', '-1', '2147483647', '2147483648', '-2147483648', '-2147483649', '2147483647.0000001', '5.0000000000000001',
  '1e-400', '-1e-400', '1e400', '1.7976931348623157e308', '1.7976931348623158e308', '1.5', '1E2', '21474836.47e2', '9007199254740993',
  '9007199254740993.5', '1e9', '0.0', '1.0e1', '2.147483647e9', '-2.147483648e9', '2.1474836475e9', '4294967296', '123456789012345678901234567890'];
const out = [];
for (const f of Object.keys(seeds)) {
  const v = new AjvJsonSchemaValidator().getValidator({ format: f });
  const seen = new Set();
  const take = (s) => { if (seen.has(s)) return; seen.add(s); out.push('F\t' + f + '\t' + JSON.stringify(s) + '\t' + (v(s).valid ? 1 : 0)); };
  for (const s of seeds[f]) take(s);
  for (let i = 0; i < per; i++) take(mutate(pick(seeds[f])));
  for (let i = 0; i < per / 10; i++) { let s = ''; const n = r(12); for (let j = 0; j < n; j++) s += pick(pool); take(s); }
  const al = [...alpha[f]];
  for (let i = 0; i < per; i++) { let s = ''; const n = r(30); for (let j = 0; j < n; j++) s += pick(al); take(s); }
  if (f === 'hostname') {
    for (const n of [1, 62, 63, 64, 252, 253, 254, 255]) for (const lab of [1, 62, 63, 64]) { take(host(n, lab, false)); take(host(n, lab, true)); }
  }
}
for (const f of ['int32', 'int64', 'float', 'double']) {
  const v = new AjvJsonSchemaValidator().getValidator({ format: f });
  for (const t of nums) out.push('N\t' + f + '\t' + t + '\t' + (v(JSON.parse(t)).valid ? 1 : 0));
  for (let i = 0; i < per / 4; i++) {
    let t = (r(3) === 0 ? '-' : '') + String(r(2) ? r(10) : r(4294967296) * (r(2) ? 1 : 2));
    if (r(3) === 0) t += '.' + String(r(1000)).padStart(r(4) + 1, '0');
    if (r(4) === 0) t += 'e' + (r(2) ? '-' : '') + r(30);
    out.push('N\t' + f + '\t' + t + '\t' + (v(JSON.parse(t)).valid ? 1 : 0));
  }
}
process.stdout.write(out.join('\n') + '\n');
