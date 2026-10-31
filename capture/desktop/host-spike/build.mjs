import { mkdir, copyFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';

for (const file of ['app.mjs', 'host.mjs', 'server.mjs', 'electron.cjs', 'preload.cjs', 'readiness.mjs']) {
  const check = spawnSync(process.execPath, ['--check', new URL(file, import.meta.url).pathname], { stdio: 'inherit' });
  if (check.status !== 0) process.exit(check.status ?? 1);
}
await mkdir(new URL('./dist/', import.meta.url), { recursive: true });
for (const name of ['index.html', 'app.mjs', 'style.css']) {
  await copyFile(new URL(name, import.meta.url), new URL(`dist/${name}`, import.meta.url));
}
console.log('Built three local renderer assets; no framework or root build inputs.');
