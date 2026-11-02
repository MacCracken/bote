// The official TypeScript client against bote's conformance fixture, over every
// transport the client has, in each era. The suite speaks HTTP alone, so this is what
// puts a third-party client on stdio. Run it through scripts/conformance.sh.
import { Client, ClientCredentialsProvider, StreamableHTTPClientTransport, extractWWWAuthenticateParams, discoverOAuthProtectedResourceMetadata } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';
import { spawn } from 'node:child_process';
import { createHmac, generateKeyPairSync, sign } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import { createServer } from 'node:http';

const BIN = process.argv[2];
let fails = 0;
let passes = 0;
const ok = (name, cond, got) => {
  if (cond) { passes++; return; }
  fails++;
  console.log('  FAIL', name, got === undefined ? '' : JSON.stringify(got).slice(0, 220));
};
const attempt = async (name, f) => {
  try { return await f(); } catch (e) { ok(`${name} threw`, false, String((e && e.message) || e)); return undefined; }
};
const thrown = async f => { try { await f(); return null; } catch (e) { return e; } };
const listening = async port => {
  for (let i = 0; i < 50; i++) {
    try { await fetch(`http://127.0.0.1:${port}/none`); return; } catch { await new Promise(r => setTimeout(r, 50)); }
  }
};

async function exercise(label, transport, mode, want) {
  const c = new Client({ name: 'bote-conformance', version: '1.0.0' }, { capabilities: {}, versionNegotiation: { mode } });
  const before = fails;
  c.onerror = () => {};
  await attempt(`${label} connect`, () => c.connect(transport()));
  const version = c.getNegotiatedProtocolVersion();
  ok(`${label} negotiated`, version === want, version);

  const tools = await attempt(`${label} listTools`, () => c.listTools());
  ok(`${label} every tool kept`, tools && tools.tools.some(t => t.name === 'test_custom_headers'), tools && tools.tools.map(t => t.name));
  const text = await attempt(`${label} text`, () => c.callTool({ name: 'test_simple_text', arguments: {} }));
  ok(`${label} text`, text && text.content[0].text === 'This is a simple text response for testing.', text);

  const values = [['plain', 'us-west1'], ['non-ASCII', 'Hello, 世界'], ['spaces', ' padded '], ['newline', 'a\nb'],
    ['sentinel', '=?base64?literal?='], ['empty', '']];
  for (const [name, region] of values) {
    const r = await attempt(`${label} ${name}`, () => c.callTool({ name: 'test_custom_headers', arguments: { region } }));
    ok(`${label} header, ${name}`, r && !r.isError, r);
  }
  const absent = await attempt(`${label} absent`, () => c.callTool({ name: 'test_custom_headers', arguments: {} }));
  ok(`${label} header, absent`, absent && !absent.isError, absent);
  const nul = await attempt(`${label} null`, () => c.callTool({ name: 'test_custom_headers', arguments: { region: null } }));
  ok(`${label} null fails the schema, not the header`, nul && nul.isError && nul.content[0].text.includes('expected string'), nul);

  const failed = await attempt(`${label} isError`, () => c.callTool({ name: 'test_error_handling', arguments: {} }));
  ok(`${label} isError`, failed && failed.isError === true, failed);
  const unknown = await thrown(() => c.callTool({ name: 'no_such_tool', arguments: {} }));
  ok(`${label} an unknown tool is a protocol error`, unknown && unknown.code === -32602, unknown && unknown.code);

  const prompt = await attempt(`${label} getPrompt`, () => c.getPrompt({ name: 'test_prompt_with_arguments', arguments: { arg1: 'hello', arg2: 'wörld' } }));
  ok(`${label} prompt`, prompt && prompt.messages[0].content.text.includes("arg2='wörld'"), prompt);

  const res = await attempt(`${label} listResources`, () => c.listResources());
  ok(`${label} resources`, res && res.resources.length === 2, res);
  const tpl = await attempt(`${label} listResourceTemplates`, () => c.listResourceTemplates());
  ok(`${label} templates`, tpl && tpl.resourceTemplates[0].uriTemplate === 'test://template/{id}/data', tpl);
  const read = await attempt(`${label} read`, () => c.readResource({ uri: 'test://static-text' }));
  ok(`${label} read`, read && read.contents[0].text.startsWith('This is the content'), read);
  const via = await attempt(`${label} read a template`, () => c.readResource({ uri: 'test://template/42/data' }));
  ok(`${label} read a template`, via && via.contents[0].text.includes('Data for ID: 42'), via);
  const gone = await thrown(() => c.readResource({ uri: 'test://nope' }));
  // bote sends -32002 on the legacy path, and this client reports both codes as -32602.
  ok(`${label} a missing resource`, gone && gone.code === -32602, gone && gone.code);

  const done = await attempt(`${label} complete`, () => c.complete({ ref: { type: 'ref/prompt', name: 'test_prompt_with_arguments' }, argument: { name: 'arg1', value: 'par' } }));
  ok(`${label} completion`, done && done.completion.values.length === 3, done);
  await attempt(`${label} close`, () => c.close());
  console.log(`${fails === before ? 'ok  ' : 'FAIL'} ${label.padEnd(20)} ${version}`);
}

const KEY = 'conformance-fixture-secret-32-bytes!';

const b64u = s => Buffer.from(s).toString('base64url');

const signers = {
  HS256: { sign: (body, key = KEY) => createHmac('sha256', key).update(body).digest() },
  RS256: (() => {
    const pair = generateKeyPairSync('rsa', { modulusLength: 2048 });
    const other = generateKeyPairSync('rsa', { modulusLength: 2048 });
    return { jwk: { ...pair.publicKey.export({ format: 'jwk' }), kid: 'k1', use: 'sig', alg: 'RS256' },
      sign: (body, key = pair.privateKey) => sign('sha256', Buffer.from(body), key), otherKey: other.privateKey };
  })(),
  ES256: (() => {
    const pair = generateKeyPairSync('ec', { namedCurve: 'P-256' });
    const other = generateKeyPairSync('ec', { namedCurve: 'P-256' });
    const p1363 = key => ({ key, dsaEncoding: 'ieee-p1363' });
    return { jwk: { ...pair.publicKey.export({ format: 'jwk' }), kid: 'k1', use: 'sig', alg: 'ES256' },
      sign: (body, key = pair.privateKey) => sign('sha256', Buffer.from(body), p1363(key)), otherKey: other.privateKey };
  })(),
};

// A real access token, signed as the authorization server signs it.
function jwt(alg, claims, key, raw = null) {
  const body = `${b64u(JSON.stringify({ alg, typ: 'at+jwt', kid: 'k1' }))}.${b64u(raw || JSON.stringify(claims))}`;
  return `${body}.${b64u(signers[alg].sign(body, key))}`;
}

// The whole of OAuth against a protected bote, through the client's own provider: bote
// says no, names its metadata, the metadata names an authorization server, and a real
// JWT from there, for this resource, is let in. The server is the smallest that can answer.
async function oauth(mode, alg) {
  const label = `oauth/${alg}/${mode}`;
  const before = fails;
  const asPort = 19530;
  const port = mode === 'legacy' ? 19531 : 19532;
  const issuer = `http://127.0.0.1:${asPort}`;
  const resource = `http://127.0.0.1:${port}/mcp`;
  const seen = [];
  const soon = Math.floor(Date.now() / 1000) + 3600;
  const as = createServer((req, res) => {
    let body = '';
    req.on('data', d => { body += d; });
    req.on('end', () => {
      seen.push({ url: req.url, body });
      res.setHeader('Content-Type', 'application/json');
      if (req.url === '/.well-known/oauth-authorization-server') {
        res.end(JSON.stringify({ issuer, authorization_endpoint: `${issuer}/authorize`, token_endpoint: `${issuer}/token`,
          response_types_supported: ['code'], grant_types_supported: ['client_credentials'],
          token_endpoint_auth_methods_supported: ['client_secret_basic', 'client_secret_post'] }));
      } else if (req.url === '/token') {
        // RFC 8707: the audience of the token is the resource the client asked for, and
        // it holds the scope the client asked for: a step-up has to ask.
        const form = new URLSearchParams(body);
        const aud = form.get('resource');
        const scope = form.get('scope') || '';
        res.end(JSON.stringify({ access_token: jwt(alg, { iss: issuer, aud, sub: 'fixture', exp: soon, scope }), token_type: 'Bearer', expires_in: 3600, scope }));
      } else {
        res.statusCode = 404;
        res.end('{}');
      }
    });
  });
  await new Promise(r => as.listen(asPort, '127.0.0.1', r));
  const args = ['auth', String(port), String(asPort)];
  if (alg !== 'HS256') {
    const file = `jwks-${alg}.json`;
    writeFileSync(file, JSON.stringify({ keys: [signers[alg].jwk] }));
    args.push(file);
  }
  const server = spawn(BIN, args, { stdio: 'ignore' });
  await listening(port);

  const bare = await fetch(resource, { method: 'POST', body: '{}' });
  ok(`${label} no token is a 401`, bare.status === 401, bare.status);
  const params = extractWWWAuthenticateParams(bare);
  ok(`${label} the 401 names the metadata`, params.resourceMetadataUrl && params.resourceMetadataUrl.href === `http://127.0.0.1:${port}/.well-known/oauth-protected-resource/mcp`, params);
  ok(`${label} the 401 names the scope`, params.scope === 'mcp:read', params);
  const wrong = await fetch(resource, { method: 'POST', body: '{}', headers: { Authorization: 'Bearer nope' } });
  ok(`${label} a bad token gets invalid_token`, extractWWWAuthenticateParams(wrong).error === 'invalid_token', extractWWWAuthenticateParams(wrong));
  const meta = await attempt(`${label} discover`, () => discoverOAuthProtectedResourceMetadata(resource));
  ok(`${label} the document names this resource`, meta && meta.resource === resource, meta);
  ok(`${label} the document names the server`, meta && meta.authorization_servers[0] === issuer, meta);
  ok(`${label} the document names the scope`, meta && meta.scopes_supported[0] === 'mcp:read', meta);

  // Tokens the signature alone would let through. The audience is what stops a token
  // meant for another server, which is what MCP calls a confused deputy.
  const good = { iss: issuer, aud: resource, exp: soon };
  const refused = [['another audience', jwt(alg, { ...good, aud: 'https://other.example.com/mcp' })],
    ['a prefix of this audience', jwt(alg, { ...good, aud: `http://127.0.0.1:${port}` })],
    ['no audience', jwt(alg, { iss: issuer, exp: soon })],
    ['another issuer', jwt(alg, { ...good, iss: 'https://evil.example.com' })],
    ['no issuer', jwt(alg, { aud: resource, exp: soon })],
    ['no expiry', jwt(alg, { iss: issuer, aud: resource })],
    ['an expired token', jwt(alg, { ...good, exp: 1000 })],
    ['not yet valid', jwt(alg, { ...good, nbf: soon })],
    ['the wrong key', jwt(alg, good, alg === 'HS256' ? 'another-key-another-key-another-key!' : signers[alg].otherKey)],
    ['alg none', `${b64u('{"alg":"none"}')}.${b64u(JSON.stringify(good))}.`]];
  for (const [name, token] of refused) {
    const r = await fetch(resource, { method: 'POST', body: '{}', headers: { Authorization: `Bearer ${token}` } });
    ok(`${label} refused: ${name}`, r.status === 401 && extractWWWAuthenticateParams(r).error === 'invalid_token', r.status);
  }
  const init = JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'x', version: '1' } } });
  const headers = token => ({ Authorization: `Bearer ${token}`, 'Content-Type': 'application/json', Accept: 'application/json, text/event-stream' });
  const accepted = [['an array of audiences', jwt(alg, { ...good, aud: ['https://other.example.com', resource] })],
    ['an escaped audience', jwt(alg, null, undefined, JSON.stringify(good).replaceAll('/', '\\/'))]];
  for (const [name, token] of accepted) {
    const r = await fetch(resource, { method: 'POST', body: init, headers: headers(token) });
    ok(`${label} accepted: ${name}`, r.status === 200, r.status);
  }

  // What a token holds, as bote reads it. `test_needs_scope` asks for mcp:write, and
  // mcp:admin implies that.
  const call = JSON.stringify({ jsonrpc: '2.0', id: 2, method: 'tools/call', params: { name: 'test_needs_scope' } });
  const scoped = [['no scope', 403, { ...good }], ['another scope', 403, { ...good, scope: 'mcp:read' }],
    ['a scope that starts the same', 403, { ...good, scope: 'mcp:write-all' }], ['another case', 403, { ...good, scope: 'MCP:WRITE' }],
    ['the scope', 200, { ...good, scope: 'mcp:read mcp:write' }], ['a broader scope', 200, { ...good, scope: 'mcp:admin' }],
    ['scp as an array', 200, { ...good, scp: ['mcp:read', 'mcp:write'] }], ['scp as a string', 200, { ...good, scp: 'mcp:write' }]];
  for (const [name, status, claims] of scoped) {
    const s = await fetch(resource, { method: 'POST', body: call, headers: headers(jwt(alg, claims)) });
    ok(`${label} ${name} is a ${status}`, s.status === status, s.status);
    if (status === 403) {
      const w = extractWWWAuthenticateParams(s);
      ok(`${label} ${name} says insufficient_scope`, w.error === 'insufficient_scope', w);
      ok(`${label} ${name} names what it needs`, w.scope === 'mcp:write', w);
      ok(`${label} ${name} names the metadata`, w.resourceMetadataUrl && w.resourceMetadataUrl.href === params.resourceMetadataUrl.href, w);
    }
  }

  const authProvider = new ClientCredentialsProvider({ clientId: 'fixture', clientSecret: 'secret', scope: 'mcp:read', expectedIssuer: issuer });
  const c = new Client({ name: 'bote-conformance', version: '1.0.0' }, { capabilities: {}, versionNegotiation: { mode } });
  c.onerror = () => {};
  await attempt(`${label} connect`, () => c.connect(new StreamableHTTPClientTransport(new URL(resource), { authProvider })));
  const r = await attempt(`${label} a call with the token`, () => c.callTool({ name: 'test_simple_text', arguments: {} }));
  ok(`${label} the call went through`, r && r.content[0].text === 'This is a simple text response for testing.', r);
  const token = seen.find(s => s.url === '/token');
  ok(`${label} the client asked its token for this resource`, token && new URLSearchParams(token.body).get('resource') === resource, token);

  // Step up: the client holds mcp:read, the call gets a 403 naming mcp:write, and the
  // client's own code asks for both and retries, with nothing here telling it to.
  const earlier = seen.filter(s => s.url === '/token').length;
  const up = await attempt(`${label} a call that needs more`, () => c.callTool({ name: 'test_needs_scope', arguments: {} }));
  ok(`${label} ran after the step-up`, up && up.content[0].text === 'This is a simple text response for testing.', up);
  const asked = seen.filter(s => s.url === '/token');
  ok(`${label} one more token`, asked.length === earlier + 1, asked.length - earlier);
  const wanted = asked.length ? new URLSearchParams(asked[asked.length - 1].body).get('scope') : null;
  ok(`${label} asked for both scopes at once`, wanted && wanted.split(' ').sort().join(' ') === 'mcp:read mcp:write', wanted);
  const again = await attempt(`${label} the same call again`, () => c.callTool({ name: 'test_needs_scope', arguments: {} }));
  ok(`${label} needs no further token`, again && seen.filter(s => s.url === '/token').length === asked.length, seen.length);
  const version = c.getNegotiatedProtocolVersion();
  await attempt(`${label} close`, () => c.close());
  server.kill();
  await new Promise(r => as.close(r));
  console.log(`${fails === before ? 'ok  ' : 'FAIL'} ${label.padEnd(20)} ${version}`);
}

const OLD = '2025-11-25';
const NEW = '2026-07-28';
for (const [mode, port, path] of [['http', 19520, '/mcp'], ['streamable', 19521, '/mcp'], ['bridge', 19522, '/']]) {
  const server = spawn(BIN, [mode, String(port)], { stdio: 'ignore' });
  await listening(port);
  const http = () => new StreamableHTTPClientTransport(new URL(`http://127.0.0.1:${port}${path}`));
  // The bridge's legacy contract is not MCP's: a failed call is a result there.
  if (mode !== 'bridge') { await exercise(`${mode}/legacy`, http, 'legacy', OLD); }
  await exercise(`${mode}/auto`, http, 'auto', NEW);
  server.kill();
}
const stdio = () => new StdioClientTransport({ command: BIN, args: [] });
await exercise('stdio/legacy', stdio, 'legacy', OLD);
await exercise('stdio/auto', stdio, 'auto', NEW);
for (const alg of ['HS256', 'RS256', 'ES256']) {
  await oauth('legacy', alg);
  await oauth('auto', alg);
}
console.log(`client: ${passes} passed, ${fails} failed`);
process.exit(fails ? 1 : 0);
