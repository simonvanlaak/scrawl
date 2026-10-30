# Homepage link preview

The homepage declares `https://app.tryapparent.com/social-preview-v1.png` in its
static Open Graph and Twitter metadata. The 1200 × 630 PNG uses the homepage's
Host Grotesk font, canonical wordmark, Paper/Carbon/Lilac colors and waving frog.
It is a brand illustration, not a screenshot of a running app. No JavaScript,
video or external font request is needed for a crawler to display it.

Rebuild the checked-in image from `site/`:

```sh
CHROME_PATH=/path/to/chrome node scripts/generate-social-preview.mjs
APPARENT_PUBLIC_SITE=1 npm run build
node scripts/package-apparent.mjs
node --test scripts/test-social-preview.mjs
```

The generator uses the existing `playwright-core` dependency and an installed
Chrome (or Playwright's existing Chromium when `CHROME_PATH` is absent). It does
not install a browser. Inspect the generated PNG after changing its design.
The public packager must include the exact image bytes; the tests check both
source and packaged head metadata, dimensions and byte preservation.

## Publication and receiver check

Follow `clanker-deploy.md`, preserving the currently deployed homepage and all
unrelated assets. The live homepage may contain approved overlays absent from
this checkout; do not replace it wholesale with an older build just to change
metadata. This candidate does not deploy anything.

Before activation, add the exact `/social-preview-v1.png` path to Caddy's
`@publicApparent` allowlist if missing. Validate/reload Caddy according to the
existing deployment procedure. Verify a crawler-style GET of the homepage
returns the image metadata and a GET of its exact image URL returns HTTP 200,
`image/png`, and bytes matching the packaged PNG. A local build or merged PR
does not establish this public-server behavior. When changing the artwork
later, use a new versioned filename and update metadata, tests and the allowlist
together so caches cannot retain the old image at the new URL.

Slack and other receivers cache unfurls independently. Test a fresh unfurl after
publication; an existing message is not guaranteed to update. Any test message
must be reviewed and approved before sending. Local metadata/image checks do
not establish that Slack has fetched or displayed the new card.
