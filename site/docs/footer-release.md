# Frog theatre footer

The accepted footer is concept 04: paper above, theatre below. The generated artwork is the original owner-selected frog theatre, encoded as `public/pond/footer-theatre.webp` (1672×941, quality 90). It contains no capture screenshots or personal data. Private studies and feedback recordings must never be packaged.

`src/Footer.tsx` and `src/footer/` are the canonical implementation. Desktop keeps the 688px scene but combines the wordmark, rotating credit and GitHub link in one 84px base row, removing the separate credit row. Narrow layouts keep a compact two-row base without squeezing or clipping the full phrases. The four phrases include their prepositions. Hover/focus pause rotation, clicking toggles pause, hidden documents stop timers and reduced motion disables rotation. The custom element removes listeners and pending animation work when detached.

The primary button explains installation first and links to the actual Mac/Linux preview releases. It does not launch capture, upload or send anything. The LinkedIn chooser uses Olivia and Simon's existing Team profile destinations. Native dialogs support Escape, focus containment and focus return. No placeholder destinations ship.

## Verification and publication

Build with `APPARENT_PUBLIC_SITE=1 npm run build`, package with `node scripts/package-apparent.mjs`, and run the `node:test` scripts (not standalone browser programs without their environment). `SITE_ARTIFACT=/absolute/artifact SITE_EVIDENCE=/private/evidence node scripts/test-footer-browser.mjs` exercises desktop/mobile, all phrases, pause/resume, reduced motion, dialog focus and detach cleanup. `SITE_URL=https://app.tryapparent.com/` selects live acceptance instead. Evidence stays outside public assets.

Deploy only a scoped overlay: `node scripts/package-footer-release.mjs BASELINE NEW_CANDIDATE EXTERNAL_MANIFEST`. It uses the same markup, CSS and behavior as the source build, replaces the SSR and React footer, and asserts every unrelated HTML byte and pre-existing asset is preserved. It reuses byte-identical immutable files and refuses conflicting ones. `SITE_BASELINE=/absolute/active-release node --test scripts/test-footer-packaging.mjs` covers initial/repeated publication and conflict refusal.

The existing `activate-belief-release.py` provides the common locked activation/guarded rollback protocol; despite its historical filename it validates any same-shape scoped manifest, preserves all old assets, and refuses a changed baseline or candidate. Use it under the required filesystem authority. It holds `/opt/presentations/releases/.apparent-site-publish.lock` across the baseline check and symlink switch. Re-read the active target and verify the public homepage plus every new asset against the manifest after publication. `/pond/*` is already allowed; no Caddy, appcast, installer or unrelated route change is needed. Retain the previous release for guarded rollback.
