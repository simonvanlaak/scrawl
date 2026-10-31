# Independent package checks

`.github/workflows/value-stream-checks.yml` adds credential-free build/test jobs
for packages that already exist. It does not extract packages or change delivery.

## Why a separate workflow

The existing `web-delivery.yml` verification job already covers hosted tests,
public composition, artifact checks and synthetic cross-origin browser behavior.
It is useful whole-product proof, but couples all three web packages in one job:
a hosted failure prevents reaching the independent browser/site checks. Keep that
composition and delivery contract unchanged. The new matrix gives each existing
package an independently reported result, with no `needs` between rows:

| Stream | Runner | Existing entrypoint |
| --- | --- | --- |
| `capture-engine` | Ubuntu 24.04, macOS 15 | Package `cargo fmt`, `cargo build`, `cargo test` |
| `connect-web` | Ubuntu 24.04 | `connect/web`: `npm test` |
| `hosted-service` | Ubuntu 24.04 | `hosted`: `npm test` |
| `public-site` | Ubuntu 24.04 | `site/scripts/build-public.mjs` and existing public/composition tests |

The Linux release job builds/tests the entire native app, not the Engine package
independently, and is not a PR gate. The updater harness tests a different
contract. Neither replaces the Engine rows. No GUI/root package, native capture,
installer, deployment, live authentication, account or remote service is run by
this workflow. Synthetic browser and workerd fixtures use loopback/local storage.
Dependency and browser downloads during setup still require network access.

## Commands and dependencies

The thin selector is `scripts/ci/check-value-streams.sh`. Scheduling belongs to
the declarative Actions matrix, not the script. From the repository root:

```sh
bash scripts/ci/check-value-streams.sh --plan capture-engine
bash scripts/ci/check-value-streams.sh capture-engine
bash scripts/ci/check-value-streams.sh connect-web
bash scripts/ci/check-value-streams.sh hosted-service
bash scripts/ci/check-value-streams.sh public-site
python3 scripts/ci/check-value-streams.test.py
```

Unknown/missing streams fail with exit 2 before any package command. `--plan`
prints the same selected commands without executing them. The selector tests
check package isolation, nonempty explicit public test selection, invalid inputs,
error propagation and rejection of stale/missing Worker output. They do not
replace execution of the actual package tests.

- **Engine:** current stable Rust with rustfmt; the package declares Rust 1.89
  minimum. `--manifest-path capture/engine/Cargo.toml --locked` uses its own
  committed lockfile and target, not the root GUI dependency graph. Builds use
  three Cargo jobs; tests use two threads. Add `CARGO_NET_OFFLINE=true` locally
  when dependencies are already cached. Finalizer tests require trusted temp
  ancestors (see `capture/engine/README.md`). Actions creates a private home child
  and removes only that test-owned directory afterward. Never change real cache,
  runtime or capture permissions to satisfy a fixture.
- **Connect:** Node 22.23.0; `bun install --frozen-lockfile` with Bun 1.4.4.
  `npm test` builds the production route-only artifact, runs artifact tests, then
  uses `scripts/test-browser.mjs` to build separate `dist-test` output against a
  controlled loopback API. Set `CHROME_PATH` to a real browser locally and
  `CONNECT_EVIDENCE` to a private scratch directory. Actions installs Chromium
  through Connect's locked Playwright version and keeps evidence in runner temp.
  Connect needs only its five approved `site/public` assets, not site dependencies
  or a site build. Production output is read back after testing.
- **Hosted:** Node 22.23.0; `npm ci` with its existing npm lock/cache.
  `npm test` typechecks, bundles the Worker, then runs the complete serial
  `test/*.test.mjs` Miniflare/workerd/D1/R2 suite. Tests use synthetic accounts,
  local migrations and a controlled OAuth provider, not real Google/Cloudflare.
  The selector removes the old Worker output first and requires nonempty fresh
  `dist/worker.js` afterward. No Wrangler deploy/dev command is invoked.
- **Public:** install site with Bun 1.3.10 and Connect with Bun 1.4.4, preserving
  both existing locks. The existing `build-public.mjs` independently builds both
  inputs and invokes `package-apparent.mjs`; this is intentional composition,
  not an import of Connect implementation into marketing. The public flag is
  `APPARENT_PUBLIC_SITE=1` and the production build-time API origin is
  `https://hosted.tryapparent.com`; building does not contact that API. The
  selector clears ambient Connect output overrides. The eight existing public
  test files include composed-package and product-boundary checks, followed by
  Connect's artifact tests and a nonempty composed homepage readback.

The allowlisted composition contract remains authoritative. Never publish raw
`site/dist`, private pitch material, feature-map routes, synthetic loopback
artifacts or browser evidence. This workflow does not upload any artifact.

## Triggers, permissions and limits

PRs and pushes to `main` or `integration/capture-boundaries-plan` use one
conservative union of input paths. Every selected event runs all matrix rows;
there is no changed-file API, custom selector or potentially incomplete per-row
filter. It includes all package contents/locks, `.cargo/**`, Rust toolchain files,
`site/**` (including the assets Connect copies), canonical `assets/brand/**`, the
shared API contract, this document, the selector/tests and this workflow. Public
composition therefore reruns for Connect changes; Connect reruns for canonical
site-asset changes. Root GUI-only changes intentionally do not select these jobs.

The workflow uses read-only `contents` permission, checkout without persisted
credentials, no secrets/environments and no deployment/authentication steps.
Matrix failures do not cancel siblings. At most three rows run concurrently;
each has a 20-minute limit. Rust uses the existing Swatinem cache pattern scoped
to the Engine workspace; hosted uses the existing setup-node npm cache pattern.
Bun lock versions are not mixed or converted. New pushes can cancel obsolete
check runs; the separate delivery workflow's concurrency is unchanged.

A local pass is not an Actions run. macOS Engine compilation and filesystem/
process behavior require the macOS matrix row; Linux proof cannot stand in for
it. Engine proof is not native product/adoption proof. This workflow does not
change branch protection or waive the repository's unchanged local push hook,
which still builds/tests exact HEAD at the root. If that hook or a separately
held native gate blocks publication, stop and report it; do not bypass the hook.
