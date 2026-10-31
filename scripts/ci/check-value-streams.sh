#!/usr/bin/env bash
# Thin package entrypoints; GitHub's declarative matrix owns scheduling.
set -euo pipefail
plan=false
if [[ ${1:-} == --plan ]]; then plan=true; shift; fi
stream=${1:-}
if [[ $# != 1 ]] || [[ ! $stream =~ ^(capture-engine|connect-web|hosted-service|public-site)$ ]]; then
  echo 'usage: bash scripts/ci/check-value-streams.sh [--plan] {capture-engine|connect-web|hosted-service|public-site}' >&2
  exit 2
fi
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
run() {
  local directory=$1; shift
  printf '(%s) ' "${directory#"$root"/}"
  printf '%q ' "$@"
  printf '\n'
  if ! $plan; then (cd "$directory" && "$@"); fi
}
case "$stream" in
  capture-engine)
    # Uses only this package's lock and target, never the GUI/root package.
    run "$root" cargo fmt --manifest-path capture/engine/Cargo.toml -- --check
    run "$root" cargo build --manifest-path capture/engine/Cargo.toml --locked -j 3
    run "$root" cargo test --manifest-path capture/engine/Cargo.toml --locked -j 3 -- --test-threads=2
    ;;
  connect-web)
    # npm test builds production, checks its artifact, then builds a separate
    # loopback artifact and exercises the controlled browser fixture.
    run "$root/connect/web" env -u CONNECT_BUILD_DIR -u CONNECT_ARTIFACT VITE_HOSTED_API_ORIGIN=https://hosted.tryapparent.com npm test
    run "$root/connect/web" test -s dist/connect/index.html
    ;;
  hosted-service)
    # No Wrangler, credentials, remote bindings or real OAuth provider.
    run "$root/hosted" rm -f dist/worker.js
    run "$root/hosted" npm test
    run "$root/hosted" test -s dist/worker.js
    ;;
  public-site)
    # The existing composition contract is intentional. Never upload raw dist.
    run "$root/site" env -u CONNECT_BUILD_DIR APPARENT_PUBLIC_SITE=1 VITE_HOSTED_API_ORIGIN=https://hosted.tryapparent.com node scripts/build-public.mjs
    run "$root/site" env -u CONNECT_BUILD_DIR node --test scripts/test-brand-favicon.mjs scripts/test-github-stars.mjs scripts/test-latest-download.mjs scripts/test-platform-os.mjs scripts/test-public-homepage.mjs scripts/test-video-demo.mjs scripts/test-composed-package.mjs scripts/test-product-boundaries.mjs
    run "$root/connect/web" env -u CONNECT_ARTIFACT npm run test:artifact
    run "$root/site" test -s dist-apparent/index.html
    ;;
esac
