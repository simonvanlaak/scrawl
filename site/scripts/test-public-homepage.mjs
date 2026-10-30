import assert from "node:assert/strict"
import { existsSync, readFileSync, readdirSync } from "node:fs"
import { dirname, join } from "node:path"
import { test } from "node:test"
import { fileURLToPath } from "node:url"

const site = join(dirname(fileURLToPath(import.meta.url)), "..")
const publicDir = join(site, "dist-apparent")

test("packaged homepage ships the brand favicon and its cache-busting URL", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  assert.match(html, /<link rel="icon" href="\/icon\.svg\?v=brand-a-1" type="image\/svg\+xml"/)
  assert.equal(readFileSync(join(publicDir, "icon.svg"), "utf8"),
    readFileSync(join(site, "../assets/brand/apparent-app-icon.svg"), "utf8"))
})

test("public homepage has one interactive demonstration", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  assert.match(html, /id="demo"/)
  assert.equal(html.match(/aria-label="Apparent demo"/g)?.length, 1)
  assert.match(html, /Play with sound/)
  assert.equal(/id="demo-film"|apparent-demo-pi\.mp4|id="how"|href="#how"/.test(html), false, "duplicate demos must not appear in the public page")
  assert.equal(existsSync(join(publicDir, "apparent-demo-pi.mp4")), false)
  assert.equal(existsSync(join(publicDir, "apparent-demo-pi-poster.jpg")), false)
})

test("product names use the wordmark except the lowercase demo heading", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  const body = html.split("<body>")[1]
  assert.match(body, /class="pond-wordmark"[^>]*src="\/apparent-wordmark\.svg"[^>]*alt="Apparent"[^>]*\/> lets you show an agent/)
  assert.equal(existsSync(join(publicDir, "apparent-wordmark.svg")), true)
  assert.match(body, /<h2>See what <span>apparent<\/span> captures<\/h2>/)
  assert.doesNotMatch(body.replace('<h2>See what <span>apparent</span> captures</h2>', ""), />\s*(?:Apparent|apparent)\b/)
})

test("the belief headline highlights enjoyable without changing its words", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  const belief = html.match(/<h2 id="belief">(.*?)<\/h2>/s)?.[1]
  assert.ok(belief)
  assert.equal(belief.replace(/<[^>]*>/g, ""), "We believe work should be enjoyable and leave us free for late nights with friends.")
  assert.match(belief, /class="enjoy-highlight"/)
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
  assert.match(body, /href="#team">Team<\/a>/)
  assert.ok(body.indexOf('class="pond-purpose"') < body.indexOf('id="team"'), "team follows the interactive belief")
  assert.ok(body.indexOf('id="team"') < body.indexOf('class="frog-footer"'), "team precedes the current footer")
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

test("team portraits fit without cropping and every affiliation logo links to its official site", () => {
  const html = readFileSync(join(publicDir, "index.html"), "utf8")
  const css = readFileSync(join(site, "src", "pond-hero.css"), "utf8")
  assert.match(css, /\.pond-team-profile > img \{[^}]*object-fit: contain;/)
  for (const [domain, image] of [
    ["start-berlin.com", "start-berlin.png"],
    ["plugandplaytechcenter.com", "plug-and-play.png"],
    ["imago-images.com", "imago.png"],
    ["openclaw.ai", "openclaw.svg"],
    ["engine.hpi.de", "hpi-engine.svg"],
    ["code.berlin", "code-wordmark.svg"],
  ]) {
    const url = `https://${domain}`
    assert.match(html, new RegExp(`<a[^>]*href="${url}"[^>]*><img[^>]*src="/team/${image.replaceAll(".", "\\.")}"`), `missing official link for ${image}`)
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
