import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Miniflare } from 'miniflare';
import { readFile } from 'node:fs/promises';

test('real Worker device authorization starts pending with requested scope', async () => {
 const mf = new Miniflare({modules:true,scriptPath:'dist/worker.js',compatibilityDate:'2026-02-05',d1Databases:['DB'],r2Buckets:['BUCKET'],bindings:{SITE_ORIGIN:'https://site.example',API_ORIGIN:'https://api.example'}});
 try {
  const db=await mf.getD1Database('DB');
  await db.exec((await readFile('migrations/0001.sql','utf8')).replaceAll('\n',' '));
  const r = await mf.dispatchFetch('https://api.example/v1/device/start',{method:'POST',body:JSON.stringify({scope:'read'})});
  assert.equal(r.status,200);
  const d = await r.json(); assert.match(d.deviceCode,/^[a-f0-9]{64}$/); assert.equal(d.interval,5);
  const p = await mf.dispatchFetch('https://api.example/v1/device/poll',{method:'POST',body:JSON.stringify({deviceCode:d.deviceCode})});
  assert.deepEqual(await p.json(),{status:'pending'});
 } finally { await mf.dispose(); }
});
