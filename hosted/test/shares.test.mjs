import {test} from 'node:test';
import assert from 'node:assert/strict';
import {fixture,id,file,digest} from './helpers.mjs';
import {readFile} from 'node:fs/promises';

async function setup(){
 const f=await fixture({SHARING_ENABLED:'true'});
 // Baseline remains valid before this additive migration exists.
 try{await f.db.exec((await readFile('migrations/0003.sql','utf8')).replaceAll('\n',' '));}catch(e){if(e.code!=='ENOENT')throw e;}
 return f;
}
async function reserve(f,a,n=171){const p='/v1/captures/'+id(n);assert.equal((await f.call(p,{token:a.token,method:'POST',body:{files:[file()]}})).status,200);return p;}
async function ready(f,a,p){assert.equal((await f.call(p+'/files/summary.md',{token:a.token,method:'PUT',raw:'hello'})).status,200);assert.equal((await f.call(p+'/complete',{token:a.token,method:'POST'})).status,200);}

test('explicit capability reservation is pending, stable, single-capture and read-only',async()=>{
 const f=await setup();try{
  const a=await f.identity('alice'),b=await f.identity('bob'),p=await reserve(f,a);
  assert.equal((await f.call(p+'/share',{token:a.read,method:'POST'})).status,403);
  assert.equal((await f.call(p+'/share',{token:b.token,method:'POST'})).status,404);
  const r=await f.call(p+'/share',{token:a.token,method:'POST'});assert.equal(r.status,200,'explicit owner sharing must reserve a capability before upload completes');
  const s=await r.json();assert.equal(s.status,'pending');assert.match(s.url,/^https:\/\/api\.example\/s\/[a-f0-9]{64}$/);
  const path=new URL(s.url).pathname,secret=path.split('/').at(-1);
  assert.notEqual(secret,a.token);assert.ok(!secret.includes(id(171)));
  const repeats=await Promise.all(Array.from({length:5},()=>f.call(p+'/share',{token:a.token,method:'POST'}).then(r=>r.json())));assert.ok(repeats.every(x=>x.url===s.url));
  assert.equal((await f.db.prepare('SELECT count(*) AS n FROM shares WHERE revoked_at IS NULL').first()).n,1);
  const stored=await f.db.prepare('SELECT * FROM shares').first();assert.equal(stored.hash,digest(secret));assert.ok(!JSON.stringify(stored).includes(secret),'plaintext capability must not be stored');
  assert.equal((await (await f.call(path+'/manifest.json')).json()).status,'pending');
  assert.equal((await f.call(path+'/files/summary.md')).status,409);
  for(const suffix of ['', '/manifest.json','/files/summary.md'])for(const method of ['POST','PUT','DELETE'])assert.equal((await f.call(path+suffix,{method})).status,405);
  for(const suffix of ['/v1/account','/captures','/files/../secret','/files/secret.txt'])assert.equal((await f.call(path+suffix)).status,404);
  assert.equal((await f.call('/v1/account',{token:secret})).status,401);
  await ready(f,a,p);
  assert.equal(await (await f.call(path+'/files/summary.md')).text(),'hello');
  const m=await (await f.call(path+'/manifest.json')).json();assert.equal(m.status,'ready');assert.equal(m.files.length,1);assert.equal(m.files[0].url,s.url+'/files/summary.md');assert.ok(!JSON.stringify(m).includes('alice'));
  const html=await (await f.call(path)).text();assert.ok(html.includes('hello'));assert.ok(html.includes('manifest.json'));
  const list=await (await f.call('/v1/shares',{token:a.read})).json();assert.equal(list.shares.length,1);assert.equal(list.shares[0].url,s.url);
  assert.deepEqual((await (await f.call('/v1/shares',{token:b.read})).json()).shares,[]);
 }finally{await f.close();}
});
