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
  assert.match(body, /class="pond-wordmark"[^>]*src="\/apparent-wordmark\.svg"[^>]*alt="Apparent"[^>]*\/> lets you show an agent/)
  assert.equal(existsSync(join(publicDir, "apparent-wordmark.svg")), true)
  assert.doesNotMatch(body, />\s*(?:Apparent|apparent)\b/)
})

test("the headline highlights enjoyable without changing its words or break", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  assert.match(html, /We believe work should be <span class="word-highlight">enjoyable(?:<svg[^>]*class="brush-highlight"[^>]*aria-hidden="true"[^>]*><path[^>]*fill="#b7a5d8"[^>]*><\/path><\/svg>)?<\/span><br\/> and leave us free for late nights with friends\./)
})

test("the first screen has a full-size reversible frog and one wave button", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  const body = html.split("<body>")[1]
  assert.match(body, /Visual feedback gets lost<br\/>in .*text\./)
  assert.equal(body.match(/class="pond-cover-frog"/g)?.length, 1)
  assert.equal(body.match(/class="pond-lounging-frog"/g)?.length, 1)
  assert.match(body, /lily-clean\.png\?v=1/, "lily bypasses a cached pre-deploy 404")
  for (const asset of ["lily-clean", "frog-rest-cohesive", "frog-air-cohesive", "frog-lounge-cohesive", "frog-wave-cohesive"]) {
    assert.equal(existsSync(join(publicDir, "pond", `${asset}.png`)), true)
  }
  const source = readFileSync(join(site, "src", "FrogPond.tsx"), "utf8")
  assert.match(source, /atEnd\.current = !returning/)
  assert.match(source, /if \(waving\.current\) return/)
  assert.match(source, /prefers-reduced-motion: reduce/)
  assert.ok(body.indexOf('id="demo"') < body.indexOf('class="pond-purpose"'), "purpose headline follows the product demo")
  assert.doesNotMatch(body, /Animation study only|Or send this to your agent/)
})

test("public homepage carries the source site's complete team section", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  const body = html.split("<body>")[1]
  assert.match(body, /id="team"/)
  assert.ok(body.indexOf('class="pond-purpose"') < body.indexOf('id="team"'))
  assert.ok(body.indexOf('id="team"') < body.indexOf('class="foot"'))
  for (const name of ["Olivia", "Simon", "START Berlin", "Plug and Play", "IMAGO", "OpenClaw", "HPI Engine", "CODE"]) {
    assert.ok(body.includes(name), `missing team content: ${name}`)
  }
  for (const name of [
    "olivia.png", "simon.png", "start-berlin.png", "plug-and-play.png",
    "imago.png", "openclaw.svg", "hpi-engine.svg", "code-wordmark.svg",
  ]) {
    assert.equal(existsSync(join(publicDir, "team", name)), true, `missing team asset: ${name}`)
  }
})

test("download actions fall back safely until the visitor's architecture is known", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  const bundle = readdirSync(join(publicDir, "assets")).find((name) => /^index-.*\.js$/.test(name))
  assert.ok(bundle)
  assert.equal(html.match(/Choose Mac download/g)?.length, 1)
  assert.match(readFileSync(join(publicDir, "assets", bundle), "utf8"), /api\.github\.com\/repos\/tryapparent\/apparent\/releases/)
  assert.doesNotMatch(html, /class="release-note"/)
})
