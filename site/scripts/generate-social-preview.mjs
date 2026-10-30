// Rebuild the checked-in static card with the site's own font and artwork.
// CHROME_PATH=/path/to/chrome node scripts/generate-social-preview.mjs
import { chromium } from "playwright-core"
import { readFileSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const site = join(dirname(fileURLToPath(import.meta.url)), "..")
const asset = (name, mime) => `data:${mime};base64,${readFileSync(join(site, "public", name)).toString("base64")}`
const browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || undefined })
try {
  const page = await browser.newPage({ viewport: { width: 1200, height: 630 }, deviceScaleFactor: 1 })
  await page.setContent(`<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Apparent social preview</title><style>
    @font-face { font-family: Host; src: url('${asset("HostGrotesk-Variable.ttf", "font/ttf")}'); }
    * { box-sizing: border-box; }
    body { margin: 0; width: 1200px; height: 630px; overflow: hidden; background: #f5f0e8; color: #020102; font-family: Host, sans-serif; }
    .wordmark { position: absolute; left: 64px; top: 54px; width: 236px; }
    h1 { position: absolute; left: 64px; top: 182px; margin: 0; font-size: 82px; line-height: 1.04; font-weight: 550; letter-spacing: -4px; }
    .meaning { position: relative; z-index: 1; }
    .meaning::after { content: ''; position: absolute; left: -5px; right: -7px; bottom: 4px; height: 30px; background: #b7a5d8; z-index: -1; transform: rotate(-2deg); }
    .frog { position: absolute; width: 410px; right: 32px; top: 152px; image-rendering: pixelated; }
    .ring { position: absolute; right: 63px; top: 207px; width: 345px; height: 233px; }
    footer { position: absolute; left: 64px; right: 64px; bottom: 48px; border-top: 1px solid #020102; padding-top: 22px; display: flex; justify-content: space-between; font-size: 25px; }
    .url { font-size: 21px; padding-top: 3px; }
  </style></head><body>
    <img class="wordmark" src="${asset("apparent-wordmark.svg", "image/svg+xml")}" alt="apparent">
    <h1>Show your agent<br>what you <span class="meaning">mean.</span></h1>
    <svg class="ring" viewBox="0 0 345 233" fill="none" aria-hidden="true"><path d="M298 48C252 8 128 9 61 43C-6 77 4 158 69 192C135 228 274 218 315 159C346 113 312 57 257 36" stroke="#b7a5d8" stroke-width="7" stroke-linecap="round"/></svg>
    <img class="frog" src="${asset("pond/frog-wave-cohesive.png", "image/png")}" alt="A waving frog">
    <footer><span>Screen, voice and marks.</span><span class="url">app.tryapparent.com</span></footer>
  </body></html>`)
  await page.evaluate(async () => {
    await document.fonts.ready
    await Promise.all([...document.images].map((image) => image.decode()))
  })
  await page.screenshot({ path: join(site, "public/social-preview-v1.png") })
  console.log("Wrote public/social-preview-v1.png (1200 × 630)")
} finally {
  await browser.close()
}
