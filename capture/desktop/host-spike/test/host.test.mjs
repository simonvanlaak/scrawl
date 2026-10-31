import test from 'node:test';
import assert from 'node:assert/strict';
import { createHost } from '../host.mjs';
import { createPreview } from '../server.mjs';
import { request } from 'node:http';

function pulse(host, extra = {}) {
  const { epoch, revision } = host.snapshot();
  return { version: 1, type: 'probe.echo', id: 'pulse-1', epoch, revision, ...extra };
}

test('real host event, retry, conflicting ID and stale command', () => {
  const host = createHost();
  const command = pulse(host);
  const reply = host.command(command);
  assert.equal(reply.ok, true);
  assert.deepEqual(reply.event, { version: 1, type: 'probe.echoed', id: 'pulse-1', epoch: command.epoch, revision: 1 });
  assert.deepEqual(host.command(command), reply);
  assert.equal(host.command({ ...command, revision: 1 }).error, 'id_conflict');
  assert.equal(host.command({ ...command, id: 'old' }).error, 'stale_revision');
  assert.equal(host.snapshot().revision, 1);
});

test('invalid versions, extra capabilities and prior-process epoch do not mutate state', () => {
  const host = createHost();
  for (const input of [null, [], {}, pulse(host, { version: 2 }), pulse(host, { type: 'capture.start' }),
    pulse(host, { epoch: createHost().snapshot().epoch }), pulse(host, { revision: -1 }),
    pulse(host, { id: '' }), { ...pulse(host), path: '/private' }]) {
    assert.equal(host.command(input).ok, false);
    assert.equal(host.snapshot().revision, 0);
  }
  assert.deepEqual(host.snapshot().capabilities, { recording: false, settings: false, connect: false });
});

test('bounded reply cache cannot reapply evicted old command; reconnect reads latest state', () => {
  const host = createHost();
  const first = pulse(host);
  for (let i = 0; i < 40; i++) assert.equal(host.command(pulse(host, { id: `pulse-${i}` })).ok, true);
  assert.equal(host.command(first).error, 'stale_revision');
  assert.equal(host.snapshot().revision, 40);
  assert.equal(host.snapshot().last.id, 'pulse-39');
});

test('HTTP adapter serves built assets and round trip, refuses cross-origin and oversized requests', async t => {
  const server = createPreview();
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => { server.closeAllConnections(); server.close(); });
  const origin = `http://127.0.0.1:${server.address().port}`;
  const page = await fetch(origin);
  assert.equal(page.status, 200);
  assert.match(await page.text(), /Native ink stays native/);
  assert.match(page.headers.get('content-security-policy'), /default-src 'none'/);
  const snapshot = await (await fetch(`${origin}/snapshot`)).json();
  const command = { version: 1, type: 'probe.echo', id: 'http-pulse', epoch: snapshot.epoch, revision: 0 };
  const post = (body, headers = {}) => fetch(`${origin}/command`, {
    method: 'POST', headers: { Origin: origin, 'Content-Type': 'application/json', ...headers }, body,
  });
  const reply = await post(JSON.stringify(command));
  assert.equal(reply.status, 200);
  assert.equal((await reply.json()).event.id, 'http-pulse');
  assert.equal((await (await fetch(`${origin}/snapshot`)).json()).revision, 1);
  assert.equal((await post(JSON.stringify(command), { Origin: 'https://example.com' })).status, 403);
  const wrongHostStatus = await new Promise((resolve, reject) => {
    const req = request(`${origin}/snapshot`, { headers: { Host: 'evil.example' } }, res => {
      res.resume(); resolve(res.statusCode);
    });
    req.on('error', reject); req.end();
  });
  assert.equal(wrongHostStatus, 403);
  assert.equal((await post('x'.repeat(2049))).status, 413);
  assert.equal((await post('{')).status, 400);
  assert.equal((await fetch(`${origin}/../../Cargo.toml`)).status, 404);
  assert.equal((await (await fetch(`${origin}/snapshot`)).json()).revision, 1);
});
