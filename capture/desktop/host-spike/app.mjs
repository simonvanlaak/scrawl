const $ = id => document.getElementById(id);
const native = Boolean(window.hostSpike);
async function request(path, input) {
  const response = await fetch(path, input ? {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(input), signal: AbortSignal.timeout(3000),
  } : { signal: AbortSignal.timeout(3000) });
  const result = await response.json();
  if (!response.ok && !result.snapshot) throw new Error(result.error ?? 'transport_failed');
  return result;
}
const bridge = window.hostSpike ?? {
  snapshot: () => request('/snapshot'),
  command: input => request('/command', input),
};
let snapshot;
let lastCommand;
let busy = false;
function available(value) {
  for (const id of ['echo', 'refresh', 'stale']) $(id).disabled = !value;
  $('stale').disabled ||= !lastCommand;
}
function apply(next) {
  if (next.version !== 1 || next.kind !== 'diagnostic-only') throw new Error('unsupported_host');
  snapshot = next;
  $('revision').textContent = String(next.revision);
  $('transport').textContent = native ? 'Electron IPC' : 'Browser → loopback HTTP';
}
async function run(action) {
  if (busy) return;
  busy = true; available(false);
  try { await action(); }
  catch (error) {
    $('status').textContent = `Host unavailable: ${error.message}. Reconnect before retrying.`;
    $('transport').textContent = 'Disconnected';
    snapshot = undefined;
  } finally {
    busy = false; available(Boolean(snapshot)); $('refresh').disabled = false;
  }
}
$('refresh').onclick = () => run(async () => {
  apply(await bridge.snapshot());
  $('status').textContent = 'Host state read. No recording or preference changed.';
});
$('echo').onclick = () => run(async () => {
  const command = { version: 1, type: 'probe.echo', id: crypto.randomUUID(),
    epoch: snapshot.epoch, revision: snapshot.revision };
  const reply = await bridge.command(command);
  lastCommand = command;
  apply(reply.snapshot);
  $('event').textContent = JSON.stringify(reply, null, 2);
  $('status').textContent = reply.ok ? `Host acknowledged pulse at revision ${reply.event.revision}.` : `Refused: ${reply.error}`;
});
$('stale').onclick = () => run(async () => {
  const reply = await bridge.command({ ...lastCommand, id: crypto.randomUUID() });
  apply(reply.snapshot);
  $('event').textContent = JSON.stringify(reply, null, 2);
  $('status').textContent = reply.ok ? 'Unexpected acceptance — inspect the host.' : `Refused: ${reply.error}. Host revision unchanged.`;
});
await run(async () => {
  apply(await bridge.snapshot());
  $('status').textContent = 'Diagnostic host ready. Send a pulse to test the round trip.';
});
