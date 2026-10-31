import assert from 'node:assert/strict'
import { readFileSync, existsSync } from 'node:fs'
import { join, dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'
const artifact = fileURLToPath(new URL('../dist-apparent/', import.meta.url))
const root = readFileSync(join(artifact, 'index.html'), 'utf8')
test('packaged homepage places four explanations between the video and belief sections', () => {
  assert.ok(root.indexOf('id="demo"') < root.indexOf('id="feature-grid-title"'))
  assert.ok(root.indexOf('id="feature-grid-title"') < root.indexOf('id="belief"'))
  assert.equal((root.match(/class="feature-grid-card"/g) ?? []).length, 4)
  assert.match(root, /Pause animations/)
})
test('every local animation asset is packaged, including the cropped whiteboard movie', () => {
  for (const page of ['how-it-works', 'timeline', 'handoff', 'anywhere']) {
    const file = join(artifact, 'feature-demos', page, 'index.html')
    const html = readFileSync(file, 'utf8')
    assert.doesNotMatch(html, /127\.0\.0\.1|\/Users\/liv\//)
    for (const [, reference] of html.matchAll(/(?:src|poster)="([^"]+)"/g)) {
      if (/^(?:https?:|data:)/.test(reference)) continue
      const target = reference.startsWith('/') ? join(artifact, reference) : resolve(dirname(file), reference)
      assert.ok(existsSync(target), `${page}: missing ${reference}`)
    }
    for (const [, script] of html.matchAll(/<script(?:\s[^>]*)?>([\s\S]*?)<\/script>/g)) assert.doesNotThrow(() => new Function(script), `${page}: invalid animation script`)
  }
})
