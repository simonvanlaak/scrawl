# Hosted API Worker

Private Cloudflare Worker, D1 metadata and R2 bundle bytes. Public UI is owned by the site. See [`../docs/hosted-api.md`](../docs/hosted-api.md) for the shared browser/native contract.

## Local proof (no Cloudflare account or credentials)

```sh
cd hosted
npm ci
npm test
```

Tests bundle the actual Worker and run it in Miniflare/workerd with real local D1 and R2 bindings. OAuth tests use a controlled HTTP provider with RSA-signed ID tokens, including invalid signatures and claims. This is **not live Google verification**. Other API security tests seed synthetic accounts/tokens directly to isolate authorization behavior. The suite includes a streamed 90,000,000-byte upload and deletion while a real upload is in flight.

Production code counts streamed chunks through `FixedLengthStream` and asks R2 to verify SHA256 natively using `R2PutOptions.sha256`, not JavaScript per-byte hashing or whole-movie `arrayBuffer()`. Completion checks R2's stored checksum, not custom metadata. Retries validate the supplied bytes in a separately registered temporary object and delete it without replacing the original. D1 batches and conditional updates preserve atomic manifests, admission, token approval, publication and tombstones. Failed/stale upload keys are registered before writing; cleanup revisits tombstones to remove late uploads. Cleanup rotates through at most 1,000 attempt keys per run, rather than starving later pages. Tombstone/attempt metadata is deliberately retained; monitor D1 growth before broadening this limited beta.

`npm run build` only bundles. `npm run dev` uses local Wrangler; local browser OAuth needs secure cookie-compatible localhost routing and a controlled provider or authorized Google test client. `wrangler.toml` contains intentionally unusable resource/origin placeholders. No test provisions remote resources.

## Deployment checklist (parent authorization required)

- Discover the actual Cloudflare account/routes and existing resources first; do not deploy this template blindly.
- **Local SQL budgets pass; remote Free verification is still blocked.** See the measured operation counts below. The template omits the Paid-only `limits.cpu_ms` override. Local native-checksum tests and statement counts do not prove Free's 10ms CPU limit. Do not activate Paid or deploy without the parent's separate decision.
- Configure an authorized D1 database and private R2 bucket. Disable all public R2 access. Keep bucket lifecycle deletion rules off unless they preserve the service's live seven-day retention contract.
- Configure same-site API/site origins; preferably route API/auth on the site's own origin. Cross-site cookies are not supported. Static `/hosted*` routes stay with the site.
- Apply `migrations/0001.sql` using Wrangler's D1 migration tooling, after review. The schema is a fresh-service migration, not an alteration of existing production tables.
- Configure `GOOGLE_CLIENT_ID`, secret `GOOGLE_CLIENT_SECRET`, secret `TOKEN_ENCRYPTION_KEY` (base64 encoding of 32 cryptographically random bytes). Never put secrets in TOML or source. Key rotation must retain old-key decryption until pending device flows expire, or explicitly invalidate those flows; existing bearer hashes remain valid.
- Register exact `API_ORIGIN/auth/google/callback` in the approved Google OAuth client. Google defaults are fixed in code. `OAUTH_ISSUER`, `OAUTH_AUTHORIZATION_URL`, `OAUTH_TOKEN_URL`, `OAUTH_JWKS_URL` exist only for controlled integration environments; do not override them for production Google sign-in.
- Confirm Google consent-screen beta/test-user rules with the owning account. No production sign-in has been run here.
- Enable the 15-minute cleanup cron. Quota and authorization expire immediately even when cron fails. Monitor cleanup failures/backlog and D1 metadata growth. A staging object can outlive its quota reservation briefly, but can never be downloaded through the API after tombstone/expiry.
- Keep request/event logs disabled or explicitly redact URLs, Authorization, Cookie, OAuth query codes/state and response tokens in any upstream logging system. The Worker never logs requests, exceptions or provider bodies. Wrangler observability is off in this template.
- Smoke-test actual CORS/cookie routing, live Google claims, 90MB request handling and cron in an authorized staging environment before production. Local workerd proof does not establish remote account configuration.

## Deliberate v1 limits

500,000,000 bytes per account, at most 20 beta accounts, seven days from reservation, 24-hour pending-upload timeout, 512 files per capture, 90,000,000 bytes per file, 128KiB JSON, 128-character exact allowlisted paths. No billing, public object URLs, multipart protocol, account deletion UI, admin promotion route or background native sync is implemented here. Separate read/upload device grants expire after 30 days; browser sessions last seven days. Revocation endpoint revokes the presented token. Browser logout revokes only its own session, not independently approved devices.

## Bounded Free audit (not deployment approval)

Official limits: [Workers](https://developers.cloudflare.com/workers/platform/limits/), [D1](https://developers.cloudflare.com/d1/platform/limits/), [R2 checksums](https://developers.cloudflare.com/r2/api/workers/workers-api-reference/). Workers Free permits 10ms CPU per HTTP request and cron invocation. Its 50 external subrequests are distinct from 1,000 internal-service subrequests. D1 separately documents **50 SQL queries per Free invocation**, 1,000 on Paid, and 100 bound parameters per query. A batch does not make its individual SQL statements disappear.

`test/budgets.test.mjs` wraps real local D1/R2 bindings to count submitted statements and operations. It verifies the unchanged full 512-file contract, without claiming Miniflare enforces remote account limits:

- Admission: **5 SQL statements**, down from 516 (one bearer lookup, one existing-ID lookup, a two-statement transaction, one live-capture lookup). The transaction inserts the reservation and all 512 files using `json_each(?)`, preserving the eviction trigger and atomic rollback. It binds six parameters for the reservation and two for the files, not a giant parameter list. Existing-ID retries use three statements; a failed insert can add one recovery lookup.
- Completion: **512 R2 heads + 5 SQL statements**. This is below 1,000 internal operations; calling it a 50-fetch violation would be wrong. CPU remains unverified remotely.
- A full cleanup page: **17 SQL statements + 10 R2 deletes**, down from 1,007 SQL statements. Expiration and attempt selection use two statements; each of ten 100-key R2 delete pages gets one `json_each(?)` set update (two bound parameters), followed by five auth/rate cleanup statements. Failed deletes never advance `cleaned_at`; a failed stamp leaves that page eligible for repeat deletion. Tombstones remain for late PUTs. A batch's five auth cleanup statements count as five, not one.

Budget tests first failed against the prior implementation at 516 admission statements and 1,007 cleanup statements. They now enforce at most 50 SQL statements and 1,000 combined SQL/R2 operations, including scheduled outcomes (Miniflare returns `outcome: exception` rather than rejecting the scheduler call). Tests also cover 20 accounts with 60 captures each, 512 files on each of the first account's existing captures, eviction isolation, full-manifest retries, rollback after an injected file-insert failure, 2,001 attempts across accounts, repeated rotation, late arrivals and injected delete/stamp failures. Existing concurrency, ready-state and streaming tests remain in the full suite.

Only manifest insertion and cleanup stamping had per-row SQL loops. Account usage, slot admission, quota eviction, capture expiration and auth cleanup remain set queries; file transfer and device routes use a fixed number of statements. Completion still performs one R2 head per file (at most 512), not SQL per file. The byte quota does **not** bound capture/file/attempt counts: zero-byte files and repeated uploads are legal, and tombstone metadata is retained. Statement counts are bounded independently of those cardinalities, but SQL scan time, daily D1 rows read/written, database growth and remote CPU remain unproven. No claim of a finite worst-case data volume or remote Free viability follows from this local audit.

No contract limits were reduced and no multipart/queue protocol was added. Parent must enable R2 in the dashboard and separately authorize staging proof; no provisioning, plan change or live CPU measurement was performed here. A $5 Paid plan is a fallback only if real Free staging fails, not a conclusion from these local counts.
