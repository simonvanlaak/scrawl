import assert from "node:assert/strict"
import { existsSync, readFileSync, readdirSync } from "node:fs"
import { dirname, join } from "node:path"
import { test } from "node:test"
import { fileURLToPath } from "node:url"

const site = join(dirname(fileURLToPath(import.meta.url)), "..")
const publicDir = join(site, "dist-apparent")

test("public homepage has one interactive demonstration", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  assert.match(html, /id="demo"/)
  assert.equal(html.match(/aria-label="Apparent demo"/g)?.length, 1)
  assert.match(html, /Play with sound/)
  assert.equal(/id="demo-film"|apparent-demo-pi\.mp4|id="how"|href="#how"/.test(html), false, "duplicate demos must not appear in the public page")
  assert.equal(existsSync(join(publicDir, "apparent-demo-pi.mp4")), false)
  assert.equal(existsSync(join(publicDir, "apparent-demo-pi-poster.jpg")), false)
})

test("visible product names use the wordmark", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  const body = html.split("<body>")[1]
  assert.match(html, /class="inline-wordmark"[^>]*src="\/apparent-wordmark\.svg"[^>]*alt="Apparent"[^>]*> lets you show an agent/)
  assert.equal(existsSync(join(publicDir, "apparent-wordmark.svg")), true)
  assert.doesNotMatch(body, />\s*(?:Apparent|apparent)\b/)
})

test("the headline highlights enjoyable without changing its words or break", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  assert.match(html, /We believe work should be <span class="word-highlight">enjoyable(?:<svg[^>]*class="brush-highlight"[^>]*aria-hidden="true"[^>]*><path[^>]*fill="#b7a5d8"[^>]*><\/path><\/svg>)?<\/span><br\/> and leave us free for late nights with friends\./)
})

test("the first screen uses a lilac ink stroke rather than a flat rectangle", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  assert.match(html, /enjoyable<svg[^>]*class="brush-highlight"[^>]*aria-hidden="true"[^>]*><path[^>]*d="M[^\"]+"[^>]*fill="#b7a5d8"[^>]*><\/path><\/svg>/)
})

test("download actions fall back safely until the visitor's architecture is known", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  const bundle = readdirSync(join(publicDir, "assets")).find((name) => /^index-.*\.js$/.test(name))
  assert.ok(bundle)
  assert.equal(html.match(/Choose Mac download/g)?.length, 2)
  assert.match(readFileSync(join(publicDir, "assets", bundle), "utf8"), /api\.github\.com\/repos\/tryapparent\/apparent\/releases/)
  assert.doesNotMatch(html, /class="release-note"/)
})
