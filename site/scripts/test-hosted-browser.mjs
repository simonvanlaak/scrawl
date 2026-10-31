// Controlled synthetic API + exact production artifact. No real Google or hosted service.
import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { readFile, mkdir } from 'node:fs/promises'
import { resolve, extname } from 'node:path'
import { chromium } from 'playwright-core'
const root = resolve(process.env.SITE_ARTIFACT || 'dist-apparent')
const evidence = resolve(process.env.SITE_EVIDENCE || `${process.env.TMPDIR}/hosted-web-evidence`)
await mkdir(evidence, { recursive: true })
for (const route of ['hosted', 'hosted/device']) {
  const html = await readFile(`${root}/${route}/index.html`, 'utf8')
  assert.match(html, /noindex, nofollow/); assert.match(html, /Checking your account/)
  assert(!html.includes('verified@example.com'))
}
assert(!(await readFile(`${root}/index.html`, 'utf8')).includes('href="/hosted'))
assert(!(await readFile(`${root}/sitemap.xml`, 'utf8')).includes('/hosted'))
let scenario = {}, writes = [], approved = false, signedOut = false, interest = false
const server = createServer(async (req, res) => {
  const url = new URL(req.url, 'http://localhost')
  if (url.pathname.startsWith('/v1/') || url.pathname === '/auth/logout') {
    res.setHeader('Content-Type', 'application/json'); res.setHeader('Cache-Control', 'no-store')
    if (scenario.loading && url.pathname === '/v1/session') await new Promise(r => setTimeout(r, 650))
    if (scenario.unavailable) return res.writeHead(503).end('{}')
    if (req.method === 'POST') {
      const chunks = []; for await (const chunk of req) chunks.push(chunk)
      const body = JSON.parse(Buffer.concat(chunks).toString())
      writes.push({ path: url.pathname, body, csrf: req.headers['x-csrf-token'], cookie: req.headers.cookie, origin: req.headers.origin })
      if (req.headers['x-csrf-token'] !== 'synthetic-csrf' || !req.headers.cookie?.includes('fixture=1') || req.headers.origin !== `http://127.0.0.1:${server.address().port}` || scenario.forbidden) return res.writeHead(403).end('{}')
      await new Promise(r => setTimeout(r, 180))
      if (url.pathname === '/v1/device/approve') approved = true
      if (url.pathname === '/v1/waitlist') interest = body.interestedAtFiveDollars
      if (url.pathname === '/auth/logout') signedOut = true
      return res.end('{"ok":true}')
    }
    if (url.pathname === '/v1/device') return res.end(JSON.stringify({ scope: scenario.scope || 'read', status: approved ? 'approved' : scenario.status || 'pending', ...(scenario.withExpiry ? { expiresAt: new Date(Date.now() + 600000).toISOString() } : {}) }))
    if (scenario.anonymous || signedOut) return res.end('{"authenticated":false,"csrfToken":null,"account":null,"slots":{"used":0,"limit":20,"remaining":20}}')
    return res.end(JSON.stringify({ authenticated: true, csrfToken: 'synthetic-csrf', account: { email: 'verified@example.com', beta: scenario.waitlist ? 'waitlist' : 'active', quotaBytes: 500000000, usedBytes: 0, waitlist: { interestedAtFiveDollars: interest } }, slots: { used: scenario.full ? 20 : 0, limit: scenario.invalid ? 50 : 20, remaining: scenario.full ? 0 : 20 }, accessToken: 'must-not-render' }))
  }
  let path = url.pathname
  if (path === '/hosted' || path === '/hosted/') path = '/hosted/index.html'
  if (path === '/hosted/device' || path === '/hosted/device/') path = '/hosted/device/index.html'
  if (path === '/') path = '/index.html'
  const file = resolve(root, '.' + path)
  if (!file.startsWith(root + '/')) return res.writeHead(403).end()
  try { res.setHeader('Content-Type', ({ '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.ttf': 'font/ttf' })[extname(file)] || 'application/octet-stream'); res.end(await readFile(file)) }
  catch { res.writeHead(404).end() }
})
await new Promise(r => server.listen(Number(process.env.HOSTED_FIXTURE_PORT || 0), '127.0.0.1', r))
console.log(`Synthetic hosted fixture at http://127.0.0.1:${server.address().port}/hosted/`)
if (process.env.HOSTED_FIXTURE_ONLY) await new Promise(() => {})
const browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || '/home/simon/.cache/ms-playwright/chromium-1228/chrome-linux64/chrome', headless: true, args: ['--no-sandbox'] })
const results = []
try {
  for (const test of [
    { name: 'anonymous', anonymous: true }, { name: 'anonymous-device', anonymous: true, device: true }, { name: 'loading', loading: true }, { name: 'empty-account', width: 320 }, { name: 'full-waitlist', waitlist: true, full: true, width: 390 },
    { name: 'read-approval', device: true }, { name: 'upload-approval', device: true, scope: 'upload', width: 390, withExpiry: true },
    { name: 'expired', device: true, status: 'expired' }, { name: 'rejected', device: true, status: 'rejected' },
    { name: 'invalid-scope', device: true, scope: 'admin' }, { name: 'no-code', device: true, noCode: true },
    { name: 'csrf-denied', device: true, forbidden: true }, { name: 'unavailable', unavailable: true }, { name: 'invalid-count', invalid: true },
  ]) {
    scenario = test; writes = []; approved = false; signedOut = false; interest = false
    const page = await browser.newPage({ viewport: { width: test.width || 1100, height: 900 } })
    const errors = []; page.on('pageerror', e => errors.push(String(e)))
    await page.context().addCookies([{ name: 'fixture', value: '1', url: `http://127.0.0.1:${server.address().port}` }])
    await page.goto(`http://127.0.0.1:${server.address().port}/hosted/${test.device ? `device${test.noCode ? '' : '?code=SYNTHETIC'}` : '?returnTo=https://evil.example'}`)
    if (test.loading) { await page.getByRole('status').filter({ hasText: 'Checking your account' }).waitFor(); await page.screenshot({ path: `${evidence}/loading-pending.png` }) }
    await page.waitForFunction(() => document.querySelector('.hosted-panel')?.getAttribute('aria-busy') === 'false')
    assert.equal(writes.length, 0, 'never auto approve or mutate on load')
    if (test.anonymous) { const href = await page.getByRole('link', { name: 'Continue with Google' }).getAttribute('href'); assert.equal(href, test.device ? '/auth/google/start?returnTo=%2Fhosted%2Fdevice%3Fcode%3DSYNTHETIC' : '/auth/google/start?returnTo=%2Fhosted') }
    if (test.waitlist) {
      assert.match(await page.locator('main').innerText(), /beta is full/)
      await page.getByRole('checkbox').check()
      await page.locator('form').evaluate(e => { e.requestSubmit(); e.requestSubmit() })
      await page.waitForFunction(() => document.querySelector('.hosted-panel')?.getAttribute('aria-busy') === 'false')
      assert.equal(writes.length, 1); assert.deepEqual(writes[0].body, { interestedAtFiveDollars: true })
    }
    const allow = page.getByRole('button', { name: /^Allow / })
    if (test.device && !test.anonymous && !test.status && !test.noCode && test.scope !== 'admin') {
      assert.equal(await allow.count(), 1)
      await allow.evaluate(e => { e.click(); e.click() })
      await page.waitForFunction(() => document.querySelector('.hosted-panel')?.getAttribute('aria-busy') === 'false')
      assert.equal(writes.length, 1); assert.equal(writes[0].csrf, 'synthetic-csrf'); assert.deepEqual(writes[0].body, { userCode: 'SYNTHETIC' })
      assert.match(await page.locator('main').innerText(), test.forbidden ? /not authorized/ : /Permission approved/)
    } else if (test.device) assert.equal(await allow.count(), 0)
    if (test.unavailable || test.invalid || test.scope === 'admin') assert.equal(await page.getByRole('alert').count(), 1)
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false)
    assert.equal(await page.evaluate(() => localStorage.length), 0)
    assert(!(await page.content()).includes('must-not-render'))
    assert(!(await page.content()).includes('synthetic-csrf'))
    assert.deepEqual(errors, [])
    await page.screenshot({ path: `${evidence}/${test.name}.png`, fullPage: true })
    if (test.name === 'empty-account') {
      await page.getByRole('button', { name: 'Log out' }).click()
      await page.getByRole('link', { name: 'Continue with Google' }).waitFor()
      assert.equal(writes[0].path, '/auth/logout'); assert.equal(writes[0].csrf, 'synthetic-csrf')
    }
    results.push(test.name); await page.close()
  }
  console.log(JSON.stringify({ synthetic: true, passed: results, screenshots: evidence }, null, 2))
} finally { await browser.close(); await new Promise(r => server.close(r)) }
