# Verification receipt

## Scope

Run 2026-10-31 on Linux x86_64, Node 22.23.4. Runtime/control implementation
is checkpoint `13c0032ef6811f8de8835ea427ba9205c7ae7fe3`, based on canonical
`9a678fafd168bffe286c8bbcb8a9f2274f72c32d`. The next evidence slice adds only
this receipt, the API/acceptance ruling, README updates and the isolated npm lock;
no production code or root build input changes. Draft PR:
<https://github.com/tryapparent/apparent/pull/213>, targeting
`integration/capture-boundaries-plan`. The parent is the sole canonical
integrator. Nothing was merged or cherry-picked into canonical.

## Executed

From a clean dependency reinstall in this directory:

```sh
npm ci --no-audit --no-fund
npm run build
npm test
npm run readiness
```

- Installation added 13 packages using the isolated lock. No system installation.
- Build syntax-checked host/renderer/adapters and copied three local assets.
- Four Node tests passed, zero failures/skips: real diagnostic event + retry/
  conflicting ID/stale rejection; invalid/extra fields/prior epoch; bounded cache
  eviction and latest snapshot; real HTTP assets/round trip + CSP/Host/Origin/
  input size/invalid JSON/path refusal. The wrong-Host test uses Node HTTP rather
  than fetch, which does not reliably preserve an explicit Host override.
- The installed Electron executable ran with `ELECTRON_RUN_AS_NODE=1`, reporting
  Electron **45.2.1**, Chromium **156.0.8078.75**, Node **24.22.1** and
  `binaryRunnable:true`. It explicitly reports native-window, overlay and recording
  tests as false. This checks binary executability, not GUI/platform acceptance.
- The unchanged repository pre-push hook for `13c0032e` ran its ten Python hook
  tests, `cargo build -j 3 --offline`, and full Linux suite with two test threads:
  **674 passed, 0 failed, 3 ignored**. This is mandatory hook evidence, not native
  GUI proof. No extra root suite was invoked outside the hooks.

## Real browser, not an IPC mock

Used the T3 collaborative preview at `http://127.0.0.1:18841`:

1. Initial page loaded all three built assets and fetched host snapshot revision 0.
2. Clicked **Send diagnostic pulse**. The real loopback host returned HTTP 200,
   a `probe.echoed` event with matching command ID and snapshot revision 1.
   The page rendered “Host acknowledged pulse at revision 1.”
3. Clicked **Try stale command**. HTTP 409 returned `stale_revision`; host and
   rendered revision remained 1. The browser console's 409 resource entry is the
   expected refusal, not an uncaught renderer exception.
4. Clicked **Reconnect / read state**. A new snapshot request returned revision 1;
   the page rendered “Host state read. No recording or preference changed.”
5. Inspected screenshots and browser accessibility tree at 1280×800 and 390×844
   CSS pixels. No horizontal overflow; narrow controls stack at 44 CSS pixel
   height. Tree exposes named buttons and the status live region. Pressing Tab
   moved focus to **Try stale command**. This is browser keyboard/AX inspection,
   not VoiceOver/AT-SPI native acceptance or a complete accessibility audit.

Private logs and synthetic screenshots remain outside git under the owner's
private evidence directory. No capture bundle existed in this environment's
standard macOS capture path; no personal capture or live app was read or touched.

## Explicitly not executed / not inferred

- Native Electron window creation, actual preload IPC or native close/reopen.
- GPUI overlay, selective cross-application hit testing, global shortcuts or ink.
- Native startup/first frame/input timings or RSS; **no browser startup latency
  was measured either**. No timing percentile or speed comparison is claimed.
- macOS, Linux desktop/compositor matrix, fractional/multiple physical displays,
  screen readers, permission prompts, capture/recording filters or complete video.
- VM ownership, GUI capture, live app install/config, credential access, paid Mac,
  Connect auth/transfer, Public build/deployment, merge or release.

These are acceptance gates, not incidental TODOs. See [ACCEPTANCE.md](ACCEPTANCE.md).
The native overlay stays the explicit working default until the parent-owned
native proof passes with frozen limits.
