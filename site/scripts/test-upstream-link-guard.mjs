import assert from "node:assert/strict"
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { dirname, join } from "node:path"
import { spawnSync } from "node:child_process"
import { test } from "node:test"
import { fileURLToPath } from "node:url"

const site = fileURLToPath(new URL("../", import.meta.url))
const rejected = [
  '<link rel="canonical" href="https://apparent.kitlangton.com/">',
  '<a href="HTTP://APPARENT.KITLANGTON.COM/download">download</a>',
  '<a href="//apparent.kitlangton.com">download</a>',
  '<a href="https://apparent.kitlangton.com.:443">download</a>',
  '<a href=https://apparent.kitlangton.com>download</a>',
  '<a href="https://apparent&#46;kitlangton.com">download</a>',
  '<base href="https://apparent.kitlangton.com/"><a href="release.dmg">download</a>',
  '<a href="https://github.com/kitlangton/tap">tap</a>',
  '<a href="https://github.com/KITLANGTON/homebrew-tap/blob/main/Casks/apparent.rb">tap</a>',
  '<a href="https://github.com/%6bitlangton/%74ap.git">tap</a>',
  '<meta property="og:url" content="https://apparent.kitlangton.com">',
  '<code>brew install --cask kitlangton/tap/apparent</code>',
  '<code>brew install --cask kitlangton&#47;tap/apparent</code>',
]
const accepted = [
  '<a href="https://example.com/?next=https://apparent.kitlangton.com/">other host</a>',
  '<a href="https://example.com/kitlangton/tap">other repo host</a>',
  '<a href="https://apparent.kitlangton.com.example.org/">lookalike</a>',
  '<a href="https://apparent.kitlangton.com@example.org/">userinfo</a>',
  '<a href="https://github.com/other/kitlangton/tap">other owner</a>',
  '<a href="https://github.com/kitlangton/tapestry">other repo</a>',
  '<!-- https://apparent.kitlangton.com/ kitlangton/tap -->',
  '<script>const oldSite = "https://apparent.kitlangton.com/"</script>',
  '<p>Historical site: https://apparent.kitlangton.com/</p>',
  '<code>brew install otherkitlangton/tap/apparent</code>',
]

// Run the real packager in a disposable site, never mutate the built artifact.
for (const [reject, cases] of [[true, rejected], [false, accepted]]) {
  for (const fragment of cases) {
    test(`${reject ? "rejects" : "accepts"} ${fragment}`, () => {
      const root = mkdtempSync(join(tmpdir(), "apparent-upstream-guard-"))
      try {
        cpSync(join(site, "scripts"), join(root, "scripts"), { recursive: true })
        writeFileSync(join(root, "package.json"), '{"type":"module"}')
        symlinkSync(join(site, "node_modules"), join(root, "node_modules"), "dir")
        const html = `<!doctype html><html><head><link rel="canonical" href="https://app.tryapparent.com/"></head><body><h1>Visual feedback gets lost</h1><p>We believe work should be enjoyable</p>${fragment}</body></html>`
        for (const name of ["index.html", "connect", "hosted", "assets", "pond", "team", "feature-demos", "apparent-wordmark.svg", "icon.svg", "HostGrotesk-Variable.ttf", "social-preview-v1.png", "Apparent-ghost-demo-v4-lilac.mp4", "Apparent-ghost-demo-v4-lilac-poster.jpg"]) {
          const path = join(root, "dist", name)
          mkdirSync(dirname(path), { recursive: true })
          writeFileSync(path, name === "index.html" ? html : "synthetic asset")
        }
        mkdirSync(join(root, "dist-apparent"))
        writeFileSync(join(root, "dist-apparent", "sentinel"), "previous artifact")
        const result = spawnSync(process.execPath, [join(root, "scripts", "package-apparent.mjs")], { encoding: "utf8" })
        if (reject) {
          assert.notEqual(result.status, 0)
          assert.match(result.stderr, /Refusing to publish upstream download or canonical links/)
          assert.equal(readFileSync(join(root, "dist-apparent", "sentinel"), "utf8"), "previous artifact")
        } else {
          assert.equal(result.status, 0, result.stderr)
          assert.equal(readFileSync(join(root, "dist-apparent", "index.html"), "utf8"), html)
        }
      } finally {
        rmSync(root, { recursive: true, force: true })
      }
    })
  }
}
