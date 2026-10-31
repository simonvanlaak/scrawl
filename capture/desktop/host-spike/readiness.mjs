import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
const require = createRequire(import.meta.url);
let binary;
try { binary = require('electron'); }
catch { console.error('Electron not installed. Run npm ci inside this isolated directory.'); process.exit(1); }
// Node mode executes the real binary without opening a native GUI or profile.
const result = spawnSync(binary, ['-p', 'JSON.stringify(process.versions)'], {
  env: { ...process.env, ELECTRON_RUN_AS_NODE: '1' }, encoding: 'utf8', timeout: 10000,
});
if (result.status !== 0) {
  console.error(result.error?.message ?? result.stderr); process.exit(1);
}
console.log(JSON.stringify({
  binary, versions: JSON.parse(result.stdout), binaryRunnable: true,
  nativeWindowTested: false, overlayAccepted: false, recordingTested: false,
}, null, 2));
