// Rebuild the compact thumbnail from the canonical app mark. No poster copy.
// CHROME_PATH=/path/to/chrome node scripts/generate-social-preview.mjs
import { chromium } from "playwright-core"
import { readFileSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const site = join(dirname(fileURLToPath(import.meta.url)), "..")
const asset = (name, mime) => `data:${mime};base64,${readFileSync(join(site, "public", name)).toString("base64")}`
const browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || undefined })
try {
  const page = await browser.newPage({ viewport: { width: 512, height: 512 }, deviceScaleFactor: 1 })
  await page.setContent(`<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Apparent social preview</title><style>
    body { margin: 0; background: #020102; }
    img { display: block; width: 512px; height: 512px; }
  </style></head><body>
    <img src="${asset("icon.svg", "image/svg+xml")}" alt="Apparent logo">
  </body></html>`)
  await page.evaluate(async () => {
    await Promise.all([...document.images].map((image) => image.decode()))
  })
  await page.screenshot({ path: join(site, "public/social-preview-v1.png") })
  console.log("Wrote public/social-preview-v1.png (512 × 512)")
} finally {
  await browser.close()
}
