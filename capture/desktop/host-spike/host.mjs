import { randomUUID } from 'node:crypto';

// Diagnostic state only. No application imports, sockets, preferences or media.
export function createHost() {
  const epoch = randomUUID();
  let revision = 0;
  let last = null;
  const replies = new Map();
  const snapshot = () => ({
    version: 1, epoch, revision, last,
    kind: 'diagnostic-only', overlayOwner: 'existing-native',
    capabilities: { recording: false, settings: false, connect: false },
  });
  function command(input) {
    if (!input || Array.isArray(input) || typeof input !== 'object' ||
        Object.keys(input).sort().join(',') !== 'epoch,id,revision,type,version' ||
        input.version !== 1 || input.type !== 'probe.echo' ||
        !/^[a-zA-Z0-9-]{1,64}$/.test(input.id) ||
        typeof input.id !== 'string' || !Number.isSafeInteger(input.revision) ||
        input.revision < 0 || input.epoch !== epoch) {
      return { ok: false, error: 'invalid_command', snapshot: snapshot() };
    }
    const fingerprint = JSON.stringify([input.epoch, input.revision, input.type]);
    const prior = replies.get(input.id);
    if (prior) return prior.fingerprint === fingerprint ? prior.reply :
      { ok: false, error: 'id_conflict', snapshot: snapshot() };
    if (input.revision !== revision) {
      return { ok: false, error: 'stale_revision', snapshot: snapshot() };
    }
    revision += 1;
    last = { version: 1, type: 'probe.echoed', id: input.id, epoch, revision };
    const reply = { ok: true, event: last, snapshot: snapshot() };
    replies.set(input.id, { fingerprint, reply });
    // Evicted IDs are still refused by their old revision; never re-applied.
    if (replies.size > 32) replies.delete(replies.keys().next().value);
    return reply;
  }
  return { snapshot, command };
}
