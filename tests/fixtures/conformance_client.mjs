// The official TypeScript client against bote's conformance fixture, over every
// transport the client has, in each era. The suite speaks HTTP alone, so this is what
// puts a third-party client on stdio. Run it through scripts/conformance.sh.
import { Client, ClientCredentialsProvider, StreamableHTTPClientTransport, extractWWWAuthenticateParams, discoverOAuthProtectedResourceMetadata } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';
import { spawn } from 'node:child_process';
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

// The whole of OAuth against a protected bote, through the client's own provider: bote
// says no, names its metadata, the metadata names an authorization server, and a token
// from there is let in. The authorization server is the smallest that can answer.
async function oauth(mode) {
  const label = `oauth/${mode}`;
  const before = fails;
  const asPort = 19530;
  const port = mode === 'legacy' ? 19531 : 19532;
  const issuer = `http://127.0.0.1:${asPort}`;
  const resource = `http://127.0.0.1:${port}/mcp`;
  const seen = [];
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
        res.end(JSON.stringify({ access_token: 'fixture-token', token_type: 'Bearer', expires_in: 3600 }));
      } else {
        res.statusCode = 404;
        res.end('{}');
      }
    });
  });
  await new Promise(r => as.listen(asPort, '127.0.0.1', r));
  const server = spawn(BIN, ['auth', String(port), String(asPort)], { stdio: 'ignore' });
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

  const authProvider = new ClientCredentialsProvider({ clientId: 'fixture', clientSecret: 'secret', scope: 'mcp:read', expectedIssuer: issuer });
  const c = new Client({ name: 'bote-conformance', version: '1.0.0' }, { capabilities: {}, versionNegotiation: { mode } });
  c.onerror = () => {};
  await attempt(`${label} connect`, () => c.connect(new StreamableHTTPClientTransport(new URL(resource), { authProvider })));
  const r = await attempt(`${label} a call with the token`, () => c.callTool({ name: 'test_simple_text', arguments: {} }));
  ok(`${label} the call went through`, r && r.content[0].text === 'This is a simple text response for testing.', r);
  const token = seen.find(s => s.url === '/token');
  ok(`${label} the client asked its token for this resource`, token && new URLSearchParams(token.body).get('resource') === resource, token);
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
await oauth('legacy');
await oauth('auto');
console.log(`client: ${passes} passed, ${fails} failed`);
process.exit(fails ? 1 : 0);
