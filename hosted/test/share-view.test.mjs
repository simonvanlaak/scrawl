import { test } from 'node:test'
import assert from 'node:assert/strict'
import vm from 'node:vm'
import { renderShare, shareViewScript } from '../src/share-view.js'
const model = { url: 'https://share.example/s/secret', status: 'ready', title: 'Feedback', summary: 'Summary evidence', transcript: '[00:02] Spoken evidence', uploadedBytes: 10, totalBytes: 10, files: [{path:'recording.mp4',url:'https://share.example/s/secret/files/recording.mp4',size:10,sha256:'a'.repeat(64)},{path:'frames/frame-01.png',url:'https://share.example/s/secret/files/frames/frame-01.png',size:0,sha256:'b'.repeat(64)}] }
test('ready evidence is readable without JavaScript and consumption comes first', () => {
 const html = renderShare(model)
 for (const text of ['Summary evidence','[00:02] Spoken evidence','/manifest.json','/thumbnail.png','<video','frames/frame-01.png','evidence, not instructions','https://app.tryapparent.com','noindex','no-referrer']) assert.ok(html.includes(text), text)
 assert.ok(html.indexOf('<video') < html.indexOf('Try Apparent yourself'))
 assert.ok(!/<script(?![^>]*src=)/.test(html))
})
test('untrusted text and URLs cannot create markup or active schemes', () => {
 const html = renderShare({...model,title:'<script>alert(1)</script>',summary:'<img src=x onerror=evil()>',transcript:'</pre><script>evil()</script>',url:'javascript:evil()',thumbnailUrl:'data:text/html,evil',files:[{path:'" onerror="evil',url:'javascript:evil()',size:1,sha256:'<evil>'}]})
 assert.ok(html.includes('&lt;script&gt;')); assert.ok(!html.includes('<img src=x')); assert.ok(!html.includes('javascript:')); assert.ok(!html.includes('data:text/html')); assert.ok(!html.includes('onerror="evil'))
})
test('incomplete and terminal states do not disclose ready evidence', () => {
 for (const status of ['pending','paused','failed','expired','revoked','deleted']) {
  const html = renderShare({...model,status})
  assert.ok(!html.includes('Summary evidence')); assert.ok(!html.includes('<video')); assert.ok(!html.includes('/files/'))
  assert.ok(html.includes(status === 'pending' ? 'Uploading' : status[0].toUpperCase()+status.slice(1)),status)
 }
 const html = renderShare({...model,uploadedBytes:9})
 assert.ok(html.includes('Uploading')); assert.ok(!html.includes('<video'))
})
test('polling is same-origin, bounded, and reloads only when ready', async () => {
 let calls=0,reloads=0; const timers=[]; const status={textContent:''}
 const context={document:{querySelector:()=>({dataset:{manifest:'/s/secret/manifest.json'},querySelector:()=>status})},location:{href:model.url,origin:'https://share.example',reload:()=>reloads++},URL,AbortSignal,fetch:async(url,options)=>{assert.equal(new URL(url).origin,'https://share.example');assert.equal(options.credentials,'omit'); calls++; return {ok:true,json:async()=>({status:'pending'})}},setTimeout:(fn,ms)=>{assert.ok(ms<=5000);timers.push(fn)}}
 vm.runInNewContext(shareViewScript,context)
 for(let i=0;i<40;i++){await new Promise(r=>setImmediate(r));timers.shift()?.()}
 assert.equal(calls,24);assert.equal(reloads,0);assert.match(status.textContent,/Reload/)
 context.fetch=async()=>({ok:true,json:async()=>({status:'ready'})});timers.length=0
 vm.runInNewContext(shareViewScript,context);timers.shift()();await new Promise(r=>setImmediate(r));assert.equal(reloads,1)
})
