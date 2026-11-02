// Writes tests/fixtures/jwt_vectors.cyr: public keys and tokens signed by Node's crypto, so
// bote's verifier is tested against a signer it shares nothing with. The private keys live
// for one run of this script and are not kept. RFC 7515's own examples go in beside them.
//
//   node tests/fixtures/jwt_vectors.mjs > tests/fixtures/jwt_vectors.cyr
import { createHmac, createPublicKey, generateKeyPairSync, sign } from 'node:crypto';
import { readFileSync } from 'node:fs';

const rfc = JSON.parse(readFileSync(new URL('./jwt_vectors_rfc.json', import.meta.url)));
const b64u = b => Buffer.from(b).toString('base64url');
const out = [];
const put = (name, value) => out.push(`var ${name} = "${value.replaceAll('\\', '\\\\').replaceAll('"', '\\"')}";`);

const rsa = bits => generateKeyPairSync('rsa', { modulusLength: bits, publicExponent: 65537 });
const r1 = rsa(2048);
const r2 = rsa(2048);
const r3 = rsa(3072);
const r4 = rsa(4096);
const e1 = generateKeyPairSync('ec', { namedCurve: 'P-256' });
const e2 = generateKeyPairSync('ec', { namedCurve: 'P-256' });
const jwk = (pair, extra) => ({ ...pair.publicKey.export({ format: 'jwk' }), ...extra });

const AUD = 'https://mcp.example.com/mcp';
const ISS = 'https://as.example.com';
const YEAR_2100 = 4102444800;
const good = { aud: AUD, iss: ISS, exp: YEAR_2100 };

const rs = key => data => sign('sha256', Buffer.from(data), key.privateKey);
const es = key => data => sign('sha256', Buffer.from(data), { key: key.privateKey, dsaEncoding: 'ieee-p1363' });
const mint = (header, claims, signer) => {
  const body = `${b64u(JSON.stringify(header))}.${b64u(JSON.stringify(claims))}`;
  return `${body}.${b64u(signer(body))}`;
};

put('JV_AUD', AUD);
put('JV_ISS', ISS);
put('JV_JWKS', JSON.stringify({ keys: [jwk(r1, { kid: 'r1', use: 'sig', alg: 'RS256' }), jwk(r2, { kid: 'r2' }), jwk(r3, { kid: 'r3' }),
  jwk(r4, { kid: 'r4' }), jwk(e1, { kid: 'e1', use: 'sig', alg: 'ES256' }), jwk(e2, { kid: 'e2' })] }));
put('JV_JWKS_ONE_RSA', JSON.stringify({ keys: [jwk(r1, {})] }));
put('JV_JWKS_ONE_EC', JSON.stringify({ keys: [jwk(e1, {})] }));
put('JV_JWK_RSA', JSON.stringify(jwk(r1, {})));
put('JV_JWK_EC', JSON.stringify(jwk(e1, {})));

for (const [p, alg, kid, signer, other] of [['JV_RS', 'RS256', 'r1', rs(r1), rs(r2)], ['JV_ES', 'ES256', 'e1', es(e1), es(e2)]]) {
  const h = { alg, kid };
  put(`${p}_GOOD`, mint(h, good, signer));
  put(`${p}_NO_KID`, mint({ alg }, good, signer));
  put(`${p}_UNKNOWN_KID`, mint({ alg, kid: 'zz' }, good, signer));
  put(`${p}_NUMBER_KID`, mint({ alg, kid: 5 }, good, signer));
  put(`${p}_OTHER_KEY`, mint(h, good, other));
  put(`${p}_OTHER_AUD`, mint(h, { ...good, aud: 'https://other.example.com/mcp' }, signer));
  put(`${p}_OTHER_ISS`, mint(h, { ...good, iss: 'https://evil.example.com' }, signer));
  put(`${p}_EXPIRED`, mint(h, { ...good, exp: 1000 }, signer));
  put(`${p}_FUTURE`, mint(h, { ...good, nbf: YEAR_2100 }, signer));
  put(`${p}_CRIT`, mint({ ...h, crit: ['exp'] }, good, signer));
  put(`${p}_LOWER_ALG`, mint({ alg: alg.toLowerCase(), kid }, good, signer));
}
// No kid, signed by the last key of its alg: what a verifier that tried each key would take.
put('JV_RS_NO_KID_LAST', mint({ alg: 'RS256' }, good, rs(r4)));
put('JV_ES_NO_KID_LAST', mint({ alg: 'ES256' }, good, es(e2)));
// A correct signature with a byte after it, and one cut short.
const rsBody = `${b64u(JSON.stringify({ alg: 'RS256', kid: 'r1' }))}.${b64u(JSON.stringify(good))}`;
const rsSig = rs(r1)(rsBody);
put('JV_RS_LONG_SIG', `${rsBody}.${b64u(Buffer.concat([rsSig, Buffer.from([0])]))}`);
put('JV_RS_SHORT_SIG', `${rsBody}.${b64u(rsSig.subarray(0, 255))}`);
put('JV_RS3072', mint({ alg: 'RS256', kid: 'r3' }, good, rs(r3)));
put('JV_RS4096', mint({ alg: 'RS256', kid: 'r4' }, good, rs(r4)));

// An RSA signature under a header that claims ES256, and the reverse.
put('JV_RS_AS_ES', mint({ alg: 'ES256', kid: 'r1' }, good, rs(r1)));
put('JV_ES_AS_RS', mint({ alg: 'RS256', kid: 'e1' }, good, es(e1)));

// The classic confusion: HS256, keyed with what the verifier holds as a public key.
const n = Buffer.from(r1.publicKey.export({ format: 'jwk' }).n, 'base64url');
const pem = r1.publicKey.export({ format: 'pem', type: 'spki' });
const hs = key => data => createHmac('sha256', key).update(data).digest();
put('JV_HS_WITH_N', mint({ alg: 'HS256', kid: 'r1' }, good, hs(n)));
put('JV_HS_WITH_PEM', mint({ alg: 'HS256', kid: 'r1' }, good, hs(pem)));

// ES256's second spelling: (r, n - s) signs the same bytes.
const P256_N = 0xffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551n;
const body = `${b64u(JSON.stringify({ alg: 'ES256', kid: 'e1' }))}.${b64u(JSON.stringify(good))}`;
const sig = es(e1)(body);
const s = BigInt(`0x${Buffer.from(sig.subarray(32)).toString('hex')}`);
const flipped = Buffer.concat([sig.subarray(0, 32), Buffer.from((P256_N - s).toString(16).padStart(64, '0'), 'hex')]);
put('JV_ES_OTHER_S', `${body}.${b64u(flipped)}`);
put('JV_ES_BODY', body);
put('JV_ES_ZERO_SIG', `${body}.${b64u(Buffer.alloc(64))}`);
const nBytes = Buffer.from(P256_N.toString(16), 'hex');
put('JV_ES_R_IS_N', `${body}.${b64u(Buffer.concat([nBytes, sig.subarray(32)]))}`);
put('JV_ES_S_IS_N', `${body}.${b64u(Buffer.concat([sig.subarray(0, 32), nBytes]))}`);
put('JV_ES_SHORT_SIG', `${body}.${b64u(sig.subarray(0, 63))}`);
put('JV_ES_LONG_SIG', `${body}.${b64u(Buffer.concat([sig, Buffer.from([0])]))}`);
put('JV_ES_DER_SIG', `${body}.${b64u(sign('sha256', Buffer.from(body), e1.privateKey))}`);

// Keys bote must not load, or must leave out.
const weak = rsa(1024);
put('JV_JWK_RSA_1024', JSON.stringify(jwk(weak, {})));
const n2047 = generateKeyPairSync('rsa', { modulusLength: 2047, publicExponent: 65537 });
put('JV_JWK_RSA_2047', JSON.stringify(jwk(n2047, {})));
const big = Buffer.concat([Buffer.from([0x80]), Buffer.alloc(1022, 0xab), Buffer.from([0x01])]);
put('JV_JWK_RSA_HUGE', JSON.stringify({ kty: 'RSA', n: b64u(big), e: 'AQAB' }));
const rj = r1.publicKey.export({ format: 'jwk' });
put('JV_JWK_RSA_E3', JSON.stringify({ ...rj, e: 'Aw' }));
put('JV_JWK_RSA_EVEN', JSON.stringify({ ...rj, n: b64u(Buffer.concat([n.subarray(0, 255), Buffer.from([0])])) }));
put('JV_JWK_RSA_LEADING_ZERO', JSON.stringify({ ...rj, n: b64u(Buffer.concat([Buffer.from([0]), n])) }));
const ej = e1.publicKey.export({ format: 'jwk' });
put('JV_JWK_EC_OFF_CURVE', JSON.stringify({ ...ej, y: ej.x }));
put('JV_JWK_EC_SHORT', JSON.stringify({ ...ej, x: b64u(Buffer.from(ej.x, 'base64url').subarray(1)) }));
put('JV_JWK_EC_LONG', JSON.stringify({ ...ej, x: b64u(Buffer.concat([Buffer.from(ej.x, 'base64url'), Buffer.from([0])])) }));
put('JV_JWK_EC_CURVE_NAME', JSON.stringify({ ...ej, crv: 'secp256k1' }));
put('JV_JWK_RSA_PS256', JSON.stringify({ ...rj, alg: 'PS256' }));
const p384 = generateKeyPairSync('ec', { namedCurve: 'P-384' });
put('JV_JWK_EC_P384', JSON.stringify(jwk(p384, {})));
put('JV_JWK_ED25519', JSON.stringify(generateKeyPairSync('ed25519').publicKey.export({ format: 'jwk' })));
// A member a private key has: the value is filler, because holding it at all is the fault.
const filler = b64u(Buffer.alloc(32, 0x11));
put('JV_JWK_PRIVATE_RSA', JSON.stringify({ ...rj, d: filler }));
put('JV_JWK_PRIVATE_EC', JSON.stringify({ ...ej, d: filler }));
put('JV_JWK_PRIVATE_P', JSON.stringify({ ...rj, p: filler }));
put('JV_JWK_OCT', JSON.stringify({ kty: 'oct', k: filler }));

// RFC 7515 A.2 and A.3, as published.
put('JV_RFC_RS_JWKS', JSON.stringify({ keys: [{ kty: 'RSA', n: rfc.rs.n, e: 'AQAB' }] }));
put('JV_RFC_RS_JWS', rfc.rs.jws);
put('JV_RFC_ES_JWKS', JSON.stringify({ keys: [{ kty: 'EC', crv: 'P-256', x: rfc.es.x, y: rfc.es.y }] }));
put('JV_RFC_ES_JWS', rfc.es.jws);

console.log(`# Generated by tests/fixtures/jwt_vectors.mjs: public keys and tokens only.\n${out.join('\n')}`);
