// Publish only this section on an inventoried, approved live homepage.
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { resolve, join, relative } from 'node:path'
const [baselineArg, candidateArg, manifestArg] = process.argv.slice(2)
if (!baselineArg || !candidateArg || !manifestArg) throw Error('Usage: node scripts/package-feature-grid.mjs BASELINE NEW_CANDIDATE PRIVATE_MANIFEST')
const baseline = resolve(baselineArg), candidate = resolve(candidateArg), manifest = resolve(manifestArg)
if (existsSync(candidate) || manifest.startsWith(candidate + '/')) throw Error('Candidate must be new; manifest must stay outside the public release')
const sha = bytes => createHash('sha256').update(bytes).digest('hex')
function inventory(root) {
  const files = {}
  function walk(dir) { for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name)
    if (entry.isSymbolicLink()) throw Error('Release must contain regular files, not symlinks')
    if (entry.isDirectory()) walk(path)
    else files[relative(root, path)] = sha(readFileSync(path))
  }}
  walk(root); return files
}
const original = readFileSync(join(baseline, 'index.html'), 'utf8')
if (!original.includes('pond-demo') || !original.includes('app.tryapparent.com') || original.includes('feature-grid-overlay')) throw Error('Unexpected or already-overlaid baseline')
const before = inventory(baseline)
cpSync(baseline, candidate, { recursive: true })
let html = original
for (const extension of ['js', 'css']) {
  const bytes = readFileSync(resolve(`dist-feature-grid/feature-grid.${extension}`))
  const name = `assets/feature-grid-overlay-${sha(bytes).slice(0, 16)}.${extension}`
  if (existsSync(join(candidate, name))) throw Error('Overlay asset collision')
  writeFileSync(join(candidate, name), bytes)
  const tag = extension === 'js' ? `<script type="module" src="/${name}"></script>` : `<link rel="stylesheet" href="/${name}">`
  html = html.replace('</head>', `${tag}</head>`)
}
const demos = join(candidate, 'feature-demos')
if (existsSync(demos)) throw Error('Existing feature-demos directory needs an explicit rebase')
mkdirSync(demos)
cpSync(resolve('public/feature-demos'), demos, { recursive: true })
writeFileSync(join(candidate, 'index.html'), html)
const after = inventory(candidate)
for (const [file, hash] of Object.entries(before)) {
  if (file !== 'index.html' && after[file] !== hash) throw Error(`Baseline asset changed: ${file}`)
}
writeFileSync(manifest, JSON.stringify({ baseline, candidate, before, after }, null, 2) + '\n')
console.log(`Feature-only overlay: ${Object.keys(after).length - Object.keys(before).length} added files; existing assets preserved`)
