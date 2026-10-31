# Extraction verification record

Candidate starts at `3b95d12430d4113394ea85155163a5f8173bf8d8`.
All authored changes are inside `connect/web/**`. No site, service, native,
deployment or running-app changes. This is migration staging, not deployment.

## Baseline and development order

- Initial site build and hosted service test attempts exited 127 because this
  isolated checkout had no dependencies (`tsc: not found`). After existing locked
  dependency installation, both baselines passed without source edits.
- Site baseline: `APPARENT_PUBLIC_SITE=1 npm --prefix site run build`, then
  `cd site && node scripts/package-apparent.mjs && npm run test:hosted`:
  29 synthetic browser scenarios passed. The public-site flag follows the owning
  design doc; no claim about the ordinary private feature-map build is made.
- Service baseline: `npm --prefix hosted test`: 52 passed, 0 failed/skipped.
- Genuine first artifact RED: `cd connect/web && node --test test/artifact.test.mjs`
  exited 1 before implementation: independent Connect artifact was absent. The
  other new-file checks also failed on absent files; those are not credited as
  meaningful behavior RED. Existing auth behavior is inherited regression coverage,
  not a new authorization implementation or a separate auth TDD claim.
- First implementation commit: `a5134f5e824723452a2c038915422a85b21bfb98`.
  Artifact checks and all 32 extracted browser scenarios passed before commit.
  Pre-push ran the unmodified repo hook: offline cargo build and full tests;
  664 passed, 3 ignored. Push succeeded without bypasses.
- Visual comparison then found the inherited `.pond-page` foreground was missing.
  Added browser assertion for `rgb(2, 1, 2)`; RED exited 1 on the wrong colour.
  Restored the existing pond foreground locally, leaving the original hosted
  component, API boundary and hosted stylesheet byte-identical to the baseline.

## Final checks

- `cd connect/web && npx --yes bun@1.4.4 install --frozen-lockfile && npm test`:
  typecheck, production client/SSR/prerender, 3 artifact tests and 32 synthetic
  browser scenarios passed. Test build is isolated in `dist-test`; production
  `dist` retains `https://hosted.tryapparent.com` as the API default.
- Copied only this package plus the five approved `site/public` files to a fresh
  scratch tree with **no site implementation, manifest or node_modules**. Frozen
  Bun install, independent build and all artifact tests passed there.
- Final baseline comparison: 1440px computer, 1440px agent and 390px computer
  screenshots are pixel-identical. Computed styles matched for the page, header,
  heading, paragraph, primary action and summary. Dynamic expiry text was not
  treated as pixel-stable evidence.
- T3 collaborative preview opened the packaged synthetic device route, retained
  `?code=SYNTHETIC`, rendered the immutable read scope, loaded assets and fetched
  session/device successfully. Environment-port navigation initially selected
  `localhost`, which the fixture correctly rejected by CORS; explicit `127.0.0.1`
  matched the fixture allowlist. This is not a production CORS/OAuth claim.
- Auth/readback, keyboard, viewport and privacy checks use a labelled synthetic
  API, not fabricated live results. No real Google, Connect account, deployed
  origin, native device or capture was used. No capture-bundle directory exists
  at this Linux checkout's `~/Library/Application Support/apparent/captures`.
- Generated Bun lock is 165 lines. Every authored file is below 2,000 lines.

Evidence remains outside git at
`/srv/clanker-hermes/cache/scratch/connect-extraction-evidence/`:
`baseline-*-installed.log`, `red-artifact.log`, `red-color.log`, `final-full.log`,
`final-browser/receipt.json`, `standalone.log`, `final-compare-styles.log`,
`push.log`. The first commit push log is Linux compile/unit proof only, not GUI
or Mac proof. Any later push gets its own log and runs the hook again.

## Coordinator gates

Review the committed privacy/auth slice independently (no nested delegation in
this worker). Parent branch advanced only by a migration-plan document when
checked; no shared runtime delta needed reconciliation. Use the README's thin
artifact contract for composition and route mapping. Remove legacy wiring only
after the exact composed artifact passes. Deployment, feature-map changes,
live OAuth/callback/cookie/CORS verification and release remain integration work.
