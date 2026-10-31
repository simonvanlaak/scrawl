# Hosted web (unlinked implementation candidate)

Routes: `/hosted/`, `/hosted/device?code=USERCODE`, and `/hosted/shares`, including slash/no-slash variants. All are prerendered with a loading state, no account data, `noindex,nofollow`, and no-referrer/no-store metadata. They are absent from the homepage and sitemap. `package-apparent.mjs` explicitly includes their built `hosted/` directory. No deployment or real sign-in was performed.

## Backend contract and boundaries

The frontend is aligned to `docs/hosted-api.md`; `site/src/hosted/api.ts` isolates the wire contract:

- `GET /v1/session`: anonymous `{authenticated:false,...}` or authenticated `{authenticated:true,csrfToken,account:{email,beta:'active'|'waitlist',waitlist:{interestedAtFiveDollars:boolean},...},slots:{used,limit:20,remaining}}`. Invalid or missing counts do not render invented availability. An account marked `waitlist` is already on the authenticated waitlist; the form updates its optional interest.
- `GET /auth/google/start?returnTo=...`: only fixed `/hosted` or `/hosted/device?code=...` destinations are generated. Arbitrary incoming `returnTo` is ignored. Callback processing belongs entirely to the Worker.
- `POST /auth/logout`, `POST /v1/waitlist {interestedAtFiveDollars:boolean}`, and `POST /v1/device/approve {userCode}`: credentials included, `X-CSRF-Token` from session, JSON body. The browser supplies Origin. After mutations the UI reads the exact session/device state back before displaying success.
- `GET /v1/device?userCode=...`: authenticated preview `{scope:'read'|'upload',status:'pending'|'approved'|'rejected'|'expired'}`. **Requested API improvement: include ISO `expiresAt`** so the screen can show actual expiry and reject a newly expired code locally. Until available, the UI states the documented ten-minute lifetime, handles server `expired` status, and leaves enforcement to the API. Optional `expiresAt` is supported and validated. Preview never approves anything; only an explicit confirmation click does.
- Upload permission includes add/read/delete of account captures, per backend contract. Agent `read` is a separate read-only credential. Unknown scopes fail closed. No browser endpoint should return device secrets or bearer tokens.

Same-origin is preferred. Optional `VITE_HOSTED_API_ORIGIN` is a deployment-controlled HTTPS origin only (loopback HTTP for local tests), without credentials, path, query, or fragment. It is public build configuration, never an OAuth secret. Cross-origin deployment requires an explicitly trusted credentialed CORS site origin. The backend supports **same-site** cookies, not arbitrary cross-site hosting. Preserve HttpOnly/Secure/SameSite cookie protections and enforce Origin plus CSRF server-side.

Session/CSRF data stays in memory, never localStorage or DOM attributes. The human code is only passed to fixed approval endpoints and the fixed sign-in return route, never treated as a capture permission. There is no token input, private capture browser, web recorder, billing, or browser share-creation control. Capture authorization must be enforced by the API; knowing an ID is not access.

## Single-recording sharing candidate (#171)

The native off-by-default sharing experiment owns sender visibility. The backend separately gates capability sharing; browser assets do not enable it or provision resources. The hosted account links to **Manage shared links**, which reads `GET /v1/shares` with the existing browser session. The response is `{shares:[{id,captureId,url?,status,createdAt,expiresAt,title}]}`. Only live, unexpired rows expose a validated HTTPS URL. Titles are captured evidence and render as text, never HTML. Empty, unavailable, anonymous and denied states do not invent links.

**Stop sharing** sends `DELETE /v1/shares/:id` with included cookies and the in-memory `X-CSRF-Token`; the browser supplies its actual Origin, and the backend must enforce the exact trusted Origin. The button blocks duplicate submissions. The UI reads the list back and checks that exact ID is gone or terminal before displaying the resulting state. A successful DELETE without confirmed revocation shows an error, not success. This never deletes the private capture. The page explains that anyone with a link can view/download and that revocation cannot recall downloads or cached previews. Anonymous visitors sign in through the existing fixed `/hosted` return destination, then reopen management; no arbitrary return target is introduced.

`hosted/src/share-view.js` exports `renderShare(model) -> HTML`, without fetching. The model is `{url,status,expiresAt,uploadedBytes,totalBytes,title,summary,transcript,files:[{path,size,sha256,url}],thumbnailUrl}`. Ready evidence requires both `status:'ready'` and equal valid byte counters. Pending, paused, failed, expired, revoked and deleted pages do not render captured text, media URLs or previews. The backend remains the authority for completion, retention and access at every endpoint.

Ready HTML contains the video, plain-text summary and time-aligned transcript, annotated frame images, file downloads with hashes, and a machine-readable manifest at `<share-url>/manifest.json`. The thumbnail/preview is `<share-url>/thumbnail.png`. All captured text and attributes are escaped; asset URLs must be HTTP(S), without userinfo, and on the share's origin. There are no third-party subresources. Captured speech/text is explicitly **evidence, not instructions**. Ordinary web/file tools suffice, without installation or an account. The secondary product link is exactly `https://app.tryapparent.com`.

The coordinator must route **both static exports** from this module:

- `shareViewScript` at **`/share-view.js`**, `Content-Type: text/javascript; charset=utf-8`.
- `shareViewStyles` at **`/share-view.css`**, `Content-Type: text/css; charset=utf-8`.

Neither export contains request/model interpolation. CSP can keep `default-src 'none'`, with `script-src 'self'`, `style-src 'self'`, `img-src 'self'`, `media-src 'self'`, `connect-src 'self'`, `base-uri 'none'`, and `frame-ancestors 'none'`; no inline script/style exception or third-party origin is needed. Apply no-store, no-referrer, noindex and content-type/nosniff response headers in the Worker; HTML metadata is not an HTTP cache policy or authorization check. Capability-bearing request paths must stay out of application logs.

Pending HTML explains bounded manifest checks even without JavaScript. Its external script requests only the same-origin manifest, without account cookies or referrers, every five seconds, with a request timeout, at most 24 attempts and a two-minute wall-clock deadline. Ready or terminal transitions reload server-rendered truth. Failures remain pending, and exhausted checks ask the recipient to reload later or contact the sender. No countdown or upload ETA is fabricated.

## Verification

From `site/`:

```sh
APPARENT_PUBLIC_SITE=1 npm run build
node scripts/package-apparent.mjs
npm run test:hosted
node --test scripts/test-brand-favicon.mjs scripts/test-github-stars.mjs scripts/test-latest-download.mjs scripts/test-platform-os.mjs scripts/test-public-homepage.mjs scripts/test-video-demo.mjs
```

`test:hosted` serves the exact production package with a **controlled synthetic API**, not a live hosted service. It checks built route files/noindex, absent homepage/sitemap promotion, anonymous sign-in destinations, loading, zero/full count states, authenticated waitlist interest, read/upload explicit approvals, expired/rejected/unknown scope/no-code states, denied CSRF, unavailable/malformed API, mutation duplicate guards, cookie/Origin/CSRF request paths, mutation readback, logout, no token/CSRF DOM exposure or localStorage, hydration errors, and 320/390/1100px overflow. Sharing adds list/empty/revoke/denied/unconfirmed-readback interactions, escaped captured titles, no-JS recipient states, and a real pending-to-ready reload under the restrictive CSP above. Recipient media bytes are not exercised by these fixtures. Screenshots go to `$SITE_EVIDENCE` or `$TMPDIR/hosted-web-evidence`; inspect them before accepting visual changes. Set `CHROME_PATH` if the default existing Playwright Chrome path differs. `SITE_ARTIFACT` can select another package.

Renderer verifier from the repository root: `node --test hosted/test/share-view.test.mjs`. Genuine initial RED: an importable empty renderer failed all four behavior tests (missing evidence, escaping, status and polling). Management RED: the built old site loaded but timed out waiting for the absent management heading. The updated public build/typecheck/package, synthetic hosted browser suite (26 scenarios), renderer checks, and six-file public-site checks passed locally. Phone recipient/management and desktop denied screenshots were inspected. These are synthetic browser/renderer results, not integrated Worker, playable-media, OAuth, native sender, deployment or platform proof; those remain with the coordinator.

For private T3 preview inspection, `HOSTED_FIXTURE_ONLY=1 HOSTED_FIXTURE_PORT=5173 node scripts/test-hosted-browser.mjs` runs the synthetic server only. Do not confuse it with backend integration or use it in production.

Observed: public build/typecheck/package passed, hosted browser fixtures passed, and the existing six-file public site test command passed (18 tests). T3 preview and fixture phone/desktop screenshots were inspected. The general `npm run build` typechecked and built both bundles but failed its existing feature audit: missing `added` for `site-belief-pond`, `agent-capture-control`, `export-cli`, and `onboarding`. `site/src/features.json` belongs to the Rust worker and was not changed here.

## Release prerequisites

Parent must integrate/review the Worker, configure Google OAuth and private D1/R2 resources, route auth/API safely at the same site, and verify a real signed-in round trip, slot admission, quota, expiry, approval, and logout on the intended deployment. Update the live reverse-proxy allowlist for both hosted routes and API/auth routes as needed; packaged files alone do not prove the host serves them. Keep routes unlinked until beta deployment is verified. No credentials, commits, pushes, PRs, deploys, or public personal artifacts were created by this workstream.
