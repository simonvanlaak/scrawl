import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {generateKeyPair,exportJWK,SignJWT} from 'jose';
import {fixture} from './helpers.mjs';

async function provider() {
 const {privateKey,publicKey}=await generateKeyPair('RS256'),other=await generateKeyPair('RS256');
 const jwk={...await exportJWK(publicKey),kid:'test',alg:'RS256',use:'sig'},codes=new Map();let issuer;
 const server=createServer(async(req,res)=>{
  if(req.url==='/jwks'){res.setHeader('Content-Type','application/json');res.end(JSON.stringify({keys:[jwk]}));return;}
  if(req.url==='/token'){
   let raw='';for await(const c of req)raw+=c;
   const params=new URLSearchParams(raw),c=codes.get(params.get('code'));codes.delete(params.get('code'));
   if(!c||params.get('client_secret')!=='controlled-secret'||!params.get('code_verifier')){res.writeHead(400);res.end('{}');return;}
   const claims={nonce:c.nonce,email:c.email||c.sub+'@example.com',email_verified:true,...c.claims};
   const token=await new SignJWT(claims).setProtectedHeader({alg:'RS256',kid:'test'}).setIssuer(c.issuer||issuer).setSubject(c.sub).setAudience(c.aud||'test-client').setIssuedAt().setExpirationTime(c.exp||'5m').sign(c.badKey?other.privateKey:privateKey);
   res.setHeader('Content-Type','application/json');res.end(JSON.stringify({id_token:token}));return;
  }
  res.writeHead(404);res.end();
 });
 await new Promise(r=>server.listen(0,'127.0.0.1',r));issuer='http://127.0.0.1:'+server.address().port;
 return {bindings:{GOOGLE_CLIENT_ID:'test-client',GOOGLE_CLIENT_SECRET:'controlled-secret',OAUTH_ISSUER:issuer,OAUTH_AUTHORIZATION_URL:issuer+'/authorize',OAUTH_TOKEN_URL:issuer+'/token',OAUTH_JWKS_URL:issuer+'/jwks'},codes,close:()=>new Promise(r=>server.close(r))};
}
async function login(f,provider,sub,options={}) {
 const start=await f.call('/auth/google/start?returnTo='+encodeURIComponent('/hosted/device?code=ABCDEF123456'));
 assert.equal(start.status,302);const u=new URL(start.headers.get('Location'));
 assert.equal(u.searchParams.get('code_challenge_method'),'S256');
 const code='code-'+sub;provider.codes.set(code,{sub,nonce:u.searchParams.get('nonce'),...options});
 const cookie=start.headers.get('Set-Cookie').split(';')[0];
 const callback='/auth/google/callback?state='+u.searchParams.get('state')+'&code='+code;
 const response=await f.call(callback,{cookie});
 return {response,callback,cookie};
}
const sessionCookie=r=>r.headers.get('Set-Cookie').match(/__Host-apparent_session=[a-f0-9]+/)[0];

test('controlled OAuth signed provider: concurrent admission max20, overflow waitlist, cookie session and no auto-approval',async()=>{
 const p=await provider(),f=await fixture(p.bindings);try{
 const pending=await(await f.call('/v1/device/start',{method:'POST',body:{scope:'upload'}})).json();
 const results=await Promise.all(Array.from({length:22},(_,i)=>login(f,p,'person'+i)));
 for(const {response} of results)assert.equal(response.status,302,await response.clone().text());
 const states=await Promise.all(results.map(({response})=>f.call('/v1/session',{cookie:sessionCookie(response)}).then(r=>r.json())));
 assert.equal(states.filter(s=>s.account.beta==='active').length,20);assert.equal(states.filter(s=>s.account.beta==='waitlist').length,2);
 assert.deepEqual(states[0].slots,{used:20,limit:20,remaining:0});
 assert.equal((await f.db.prepare('SELECT status FROM devices').first()).status,'pending');
 const index=states.findIndex(s=>s.account.beta==='waitlist'),cookie=sessionCookie(results[index].response),csrf=states[index].csrfToken;
 assert.equal((await f.call('/v1/waitlist',{cookie,csrf,origin:'https://site.example',method:'POST',body:{interestedAtFiveDollars:true}})).status,200);
 assert.equal((await(await f.call('/v1/session',{cookie})).json()).account.waitlist.interestedAtFiveDollars,true);
 assert.equal((await f.call('/v1/device/approve',{cookie,csrf,origin:'https://site.example',method:'POST',body:{userCode:pending.userCode}})).status,403);
 const replay=await f.call(results[0].callback,{cookie:results[0].cookie});assert.equal(replay.status,401);
 assert.match(results[0].response.headers.get('Set-Cookie'),/Secure; HttpOnly; SameSite=Lax/);
 assert.equal(results[0].response.headers.get('Location'),'https://site.example/hosted/device?code=ABCDEF123456');
 }finally{await f.close();await p.close();}
});

test('controlled OAuth rejects unverified email, wrong nonce/issuer/audience/expiry/signature and state',async()=>{
 const p=await provider(),f=await fixture(p.bindings);try{
 let n=0;
 for(const options of [{claims:{email_verified:false}},{claims:{nonce:'wrong'}},{issuer:'https://evil.example'},{aud:'other-client'},{exp:'-1h'},{badKey:true}]) {
  const {response}=await login(f,p,'invalid'+n++,options);assert.equal(response.status,401);assert.deepEqual(await response.json(),{error:{code:'oauth_failed',message:'oauth failed'}});
 }
 assert.equal((await f.call('/auth/google/callback?state=wrong&code=stolen',{cookie:'__Host-apparent_oauth=other'})).status,401);
 assert.equal((await f.call('/auth/google/start?returnTo=https://evil.example/hosted')).status,400);
 assert.equal((await f.call('/auth/google/start?returnTo=/hosted?token=oops')).status,400);
 assert.equal((await f.db.prepare('SELECT count(*) AS n FROM accounts').first()).n,0);
 const anonymous=await(await f.call('/v1/session')).json();assert.deepEqual(anonymous,{authenticated:false,csrfToken:null,account:null,slots:{used:0,limit:20,remaining:20}});
 }finally{await f.close();await p.close();}
});
