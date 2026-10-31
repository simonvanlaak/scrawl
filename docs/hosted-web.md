# Hosted web (unlinked implementation candidate)

Routes: `/hosted/` and `/hosted/device?code=USERCODE`, including slash/no-slash variants. Both are prerendered with a loading state, no account data, and `noindex,nofollow`. They are absent from the homepage and sitemap. `package-apparent.mjs` explicitly includes their built `hosted/` directory. No deployment or real sign-in was performed.

## Backend contract and boundaries

The frontend is aligned to `docs/hosted-api.md`; `site/src/hosted/api.ts` isolates the wire contract:

- `GET /v1/session`: anonymous `{authenticated:false,...}` or authenticated `{authenticated:true,csrfToken,account:{email,beta:'active'|'waitlist',waitlist:{interestedAtFiveDollars:boolean},...},slots:{used,limit:20,remaining}}`. Invalid or missing counts do not render invented availability. An account marked `waitlist` is already on the authenticated waitlist; the form updates its optional interest.
- `GET /auth/google/start?returnTo=...`: only fixed `/hosted` or `/hosted/device?code=...` destinations are generated. Arbitrary incoming `returnTo` is ignored. Callback processing belongs entirely to the Worker.
- `POST /auth/logout`, `POST /v1/waitlist {interestedAtFiveDollars:boolean}`, and `POST /v1/device/approve {userCode}`: credentials included, `X-CSRF-Token` from session, JSON body. The browser supplies Origin. After mutations the UI reads the exact session/device state back before displaying success.
- `GET /v1/device?userCode=...`: authenticated preview `{scope:'read'|'upload',status:'pending'|'approved'|'rejected'|'expired'}`. **Requested API improvement: include ISO `expiresAt`** so the screen can show actual expiry and reject a newly expired code locally. Until available, the UI states the documented ten-minute lifetime, handles server `expired` status, and leaves enforcement to the API. Optional `expiresAt` is supported and validated. Preview never approves anything; only an explicit confirmation click does.
- Upload permission includes add/read/delete of account captures, per backend contract. Agent `read` is a separate read-only credential. Unknown scopes fail closed. No browser endpoint should return device secrets or bearer tokens.

Same-origin is preferred. Optional `VITE_HOSTED_API_ORIGIN` is a deployment-controlled HTTPS origin only (loopback HTTP for local tests), without credentials, path, query, or fragment. It is public build configuration, never an OAuth secret. Cross-origin deployment requires an explicitly trusted credentialed CORS site origin. The backend supports **same-site** cookies, not arbitrary cross-site hosting. Preserve HttpOnly/Secure/SameSite cookie protections and enforce Origin plus CSRF server-side.

Session/CSRF data stays in memory, never localStorage or DOM attributes. The human code is only passed to fixed approval endpoints and the fixed sign-in return route, never treated as a capture permission. No token input, capture listing, web recorder, billing, or public capture sharing exists. Capture authorization must be enforced by the API; knowing an ID is not access.

## Verification

From `site/`:

```sh
APPARENT_PUBLIC_SITE=1 npm run build
node scripts/package-apparent.mjs
npm run test:hosted
node --test scripts/test-brand-favicon.mjs scripts/test-github-stars.mjs scripts/test-latest-download.mjs scripts/test-platform-os.mjs scripts/test-public-homepage.mjs scripts/test-video-demo.mjs
```

`test:hosted` serves the exact production package with a **controlled synthetic API**, not a live hosted service. It checks built route files/noindex, absent homepage/sitemap promotion, anonymous sign-in destinations, loading, zero/full count states, authenticated waitlist interest, read/upload explicit approvals, expired/rejected/unknown scope/no-code states, denied CSRF, unavailable/malformed API, mutation duplicate guards, cookie/Origin/CSRF request paths, mutation readback, logout, no token/CSRF DOM exposure or localStorage, hydration errors, and 320/390/1100px overflow. Screenshots go to `$SITE_EVIDENCE` or `$TMPDIR/hosted-web-evidence`; inspect them before accepting visual changes. Set `CHROME_PATH` if the default existing Playwright Chrome path differs. `SITE_ARTIFACT` can select another package.

For private T3 preview inspection, `HOSTED_FIXTURE_ONLY=1 HOSTED_FIXTURE_PORT=5173 node scripts/test-hosted-browser.mjs` runs the synthetic server only. Do not confuse it with backend integration or use it in production.

Observed: public build/typecheck/package passed, hosted browser fixtures passed, and the existing six-file public site test command passed (18 tests). T3 preview and fixture phone/desktop screenshots were inspected. The general `npm run build` typechecked and built both bundles but failed its existing feature audit: missing `added` for `site-belief-pond`, `agent-capture-control`, `export-cli`, and `onboarding`. `site/src/features.json` belongs to the Rust worker and was not changed here.

## Release prerequisites

Parent must integrate/review the Worker, configure Google OAuth and private D1/R2 resources, route auth/API safely at the same site, and verify a real signed-in round trip, slot admission, quota, expiry, approval, and logout on the intended deployment. Update the live reverse-proxy allowlist for both hosted routes and API/auth routes as needed; packaged files alone do not prove the host serves them. Keep routes unlinked until beta deployment is verified. No credentials, commits, pushes, PRs, deploys, or public personal artifacts were created by this workstream.
