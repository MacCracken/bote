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
  ok(`${label} resources`, res && res.resources.length === 3, res);
  const tpl = await attempt(`${label} listResourceTemplates`, () => c.listResourceTemplates());
  ok(`${label} templates`, tpl && tpl.resourceTemplates[0].uriTemplate === 'test://template/{id}/data', tpl);
  const read = await attempt(`${label} read`, () => c.readResource({ uri: 'test://static-text' }));
  ok(`${label} read`, read && read.contents[0].text.startsWith('This is the content'), read);
  const via = await attempt(`${label} read a template`, () => c.readResource({ uri: 'test://template/42/data' }));
  ok(`${label} read a template`, via && via.contents[0].text.includes('Data for ID: 42'), via);
  const gone = await thrown(() => c.readResource({ uri: 'test://nope' }));
  // bote sends -32002 on the legacy path, and this client reports both codes as -32602.
  ok(`${label} a missing resource`, gone && gone.code === -32602, gone && gone.code);

  // (3.7.6) Progress on the request's own stream: the client asks with onprogress, and hears
  // each report, in order, before its answer.
  const seen = [];
  const prog = await attempt(`${label} progress`, () => c.callTool({ name: 'test_tool_with_progress', arguments: {} }, { onprogress: p => seen.push(p.progress) }));
  ok(`${label} progress answered`, prog && prog.content[0].text === 'done', prog);
  // bote sends all three, then the answer. The client handles a response at once and a
  // notification a tick later, so on stdio, where the last report and the answer arrive in one
  // read, its _onresponse removes the progress handler first and that report is dropped.
  const need = label.startsWith('stdio') ? 2 : 3;
  ok(`${label} progress heard`, seen.length >= need && seen.every((v, i) => i === 0 || seen[i - 1] < v), seen);

  // (3.8.0) outputSchema: the client checks each structured result against what tools/list
  // said, and 2025-11-25 is shown an array root wrapped as {result}, on both sides.
  const sw = tools && tools.tools.find(t => t.name === 'bote_structured');
  ok(`${label} outputSchema listed`, sw && sw.outputSchema && sw.outputSchema.required.includes('humidity'), sw);
  const st = await attempt(`${label} structured`, () => c.callTool({ name: 'bote_structured', arguments: {} }));
  ok(`${label} structured`, st && !st.isError && st.structuredContent.temperature === 22.5 && JSON.parse(st.content[0].text).humidity === 65, st);
  const su = tools && tools.tools.find(t => t.name === 'bote_structured_list');
  const arrayRoot = want === NEW ? su && su.outputSchema.type === 'array' : su && su.outputSchema.type === 'object' && su.outputSchema.properties.result.type === 'array';
  ok(`${label} an array root listed`, arrayRoot, su && su.outputSchema);
  const sl = await attempt(`${label} structured list`, () => c.callTool({ name: 'bote_structured_list', arguments: {} }));
  const users = sl && (want === NEW ? sl.structuredContent : sl.structuredContent.result);
  ok(`${label} structured list`, sl && !sl.isError && Array.isArray(users) && users[1].name === 'Bob', sl);
  const sb = await attempt(`${label} structured, refused`, () => c.callTool({ name: 'bote_structured_bad', arguments: {} }));
  ok(`${label} a result its schema refuses is a tool error`, sb && sb.isError === true && sb.content[0].text.includes('structuredContent.temperature: expected number'), sb);

  // (3.8.1) Titles, icons, serverInfo's other fields and instructions, read back as sent.
  const iconOk = icons => Array.isArray(icons) && icons.length === 2 && icons[0].src === 'https://example.com/bote.png'
    && icons[0].theme === 'dark' && icons[0].sizes[0] === '48x48' && icons[1].src.startsWith('data:image/svg+xml;base64,');
  ok(`${label} a tool's title and icons`, sw && sw.title === 'Weather now' && iconOk(sw.icons), sw);
  const lp = await attempt(`${label} listPrompts`, () => c.listPrompts());
  const tp = lp && lp.prompts.find(p => p.name === 'bote_titled_prompt');
  // An argument's title is in the spec's PromptArgument, and bote sends it (tests/bote_display.tcyr checks the
  // bytes), but this client's schema for PromptArgument has no `title`, so its parse drops it.
  ok(`${label} a prompt's title and icons`, tp && tp.title === 'Titled prompt' && iconOk(tp.icons) && tp.arguments[0].name === 'topic', tp);
  const tr = res && res.resources.find(r => r.uri === 'test://bote-titled');
  ok(`${label} a resource's title and icons`, tr && tr.title === 'Titled resource' && iconOk(tr.icons), tr);
  const tt = tpl && tpl.resourceTemplates.find(t => t.uriTemplate === 'test://bote-titled/{id}');
  ok(`${label} a template's title and icons`, tt && tt.title === 'Titled template' && iconOk(tt.icons), tt);
  const si = c.getServerVersion();
  ok(`${label} serverInfo's title, description, website and icons`, si && si.title === 'bote conformance fixture' && si.websiteUrl === 'https://github.com/MacCracken/bote'
    && si.description.startsWith('The servers') && iconOk(si.icons), si);
  ok(`${label} instructions`, c.getInstructions() === 'Call bote_structured for the weather.\nIt has an outputSchema.', c.getInstructions());

  const done = await attempt(`${label} complete`, () => c.complete({ ref: { type: 'ref/prompt', name: 'test_prompt_with_arguments' }, argument: { name: 'arg1', value: 'par' } }));
  ok(`${label} completion`, done && done.completion.values.length === 3, done);

  // subscriptions/listen (3.7.3), which only 2026-07-28 has: the acknowledgement names what is
  // honored, a tool registered meanwhile is a change the client hears, and close() cancels.
  if (want === NEW) {
    let heard = 0;
    c.setNotificationHandler('notifications/tools/list_changed', () => { heard++; });
    const sub = await attempt(`${label} listen`, () => c.listen({ toolsListChanged: true, promptsListChanged: true }));
    ok(`${label} listen honored`, sub && sub.honoredFilter.toolsListChanged === true && sub.honoredFilter.promptsListChanged === true, sub && sub.honoredFilter);
    await attempt(`${label} change the tools`, () => c.callTool({ name: 'test_trigger_tool_change', arguments: {} }));
    for (let i = 0; i < 60 && heard === 0; i++) { await new Promise(r => setTimeout(r, 50)); }
    ok(`${label} a change heard`, heard >= 1, heard);
    if (sub) { await attempt(`${label} listen close`, () => sub.close()); }
    // (3.7.4) resourceSubscriptions: the URIs honored come back, and an update under one of them
    // is heard, and one under none is not.
    const updated = [];
    c.setNotificationHandler('notifications/resources/updated', n => { updated.push(n.params.uri); });
    const want = ['test://static-text', 'test://template/'];
    const rs = await attempt(`${label} listen for updates`, () => c.listen({ resourceSubscriptions: want }));
    ok(`${label} updates honored`, rs && JSON.stringify(rs.honoredFilter.resourceSubscriptions) === JSON.stringify(want), rs && rs.honoredFilter);
    await attempt(`${label} update an unasked one`, () => c.callTool({ name: 'bote_update_resource', arguments: { uri: 'test://static-binary' } }));
    await attempt(`${label} update one under a subscription`, () => c.callTool({ name: 'bote_update_resource', arguments: { uri: 'test://template/42/data' } }));
    for (let i = 0; i < 60 && !updated.includes('test://template/42/data'); i++) { await new Promise(r => setTimeout(r, 50)); }
    ok(`${label} an update heard`, updated.includes('test://template/42/data'), updated);
    ok(`${label} and none unasked for`, !updated.includes('test://static-binary'), updated);
    if (rs) { await attempt(`${label} updates close`, () => rs.close()); }
    const after = await attempt(`${label} after the listen`, () => c.callTool({ name: 'test_simple_text', arguments: {} }));
    ok(`${label} still served after the listen`, after && after.content[0].text.startsWith('This is a simple'), after);
  }
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
      } else if (req.url === '/jwks') {
        // Its jwks_uri, which bote fetches for RS256 (3.6.6).
        res.setHeader('Content-Type', 'application/jwk-set+json');
        res.end(JSON.stringify({ keys: [signers[alg].jwk] }));
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
  // RS256's keys come from the authorization server's jwks_uri, ES256's from a file.
  if (alg === 'RS256') {
    args.push(`${issuer}/jwks`);
  } else if (alg !== 'HS256') {
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
