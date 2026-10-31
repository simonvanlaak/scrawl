# Capture boundaries migration implementation plan

> **For Hermes:** Use subagent-driven-development for later slices; this foundation slice has no nested agents.

**Goal:** Deliver one recording engine used by independently installable Agent and Desktop clients, separate Connect and Public products, and thin native/UI adapters without changing existing identity or consent contracts.

**Architecture:** `capture/engine` owns local recording, processing, bundles, recovery and retention. `capture/{desktop,agent}` own their workflows; `connect/` owns service/web/client transfer; `public/` is the later marketing/brand destination (marketing remains in `site/` now). `platform/ui` owns only shared UI primitives/tokens and their thin mechanisms. Capture OS adapters stay inside the relevant capture capability; a global platform directory must not absorb capture policy. Move working behavior in tested increments, not empty directory scaffolds.

**Tech stack verified:** Root is one Rust 2024 binary crate (not a Cargo workspace), GPUI 0.2.2; macOS also uses Swift/AppKit/Metal. Linux uses X11/XWayland GPUI, GStreamer, portals and compositor adapters. `site/` is React 19.3/TypeScript/Vite 8, with no shadcn/Radix dependency today. `hosted/` is strict TypeScript, esbuild, Cloudflare Worker/D1/R2, Miniflare tests. React/TypeScript with shadcn/Radix is the proposed Desktop UI, not an existing implementation or chosen native host.

## 0. Frozen foundation and ownership

- Historical foundation ownership was **only this file**, plus branch setup. Current integration ownership is web composition/CI/docs and the agreed seams below; no root AGENTS, merge, release, installation, issue comments, user config edits or native recording.
- Starting branch `t3/pull-latest-main`, SHA `416320bdefcb004dc91f8bea64b445162eb48e6d`, clean. Draft #198 owns its hook work; do not push to that branch or edit that PR.
- Fetched `origin/main`: `dd801e420e2a810b221ca0de8571a8a5ded4ead0`. Common ancestor `1bd17ae0979da0c1394f9a991c0727ab97dd3157`; start is 1 unique commit, main 5 unique commits. Main adds Connect naming/CLI compatibility (#196/#197).
- Dedicated branch: `integration/capture-boundaries-plan`, created here, not in another worktree. Merge checkpoint `3b95d12430d4113394ea85155163a5f8173bf8d8` has both start and fetched main as ancestors. No conflict resolutions or manual product edits.
- Proposed PR base is `main`; its deliberate inherited diff includes #198's three hook files until #198 lands. Do not cherry-pick/duplicate the hook or retarget/overwrite #198. Recheck live ancestry/diff before publication. Coordinator owns later trunk integration and branch coordination.
- Every implementation lane gets its own short-lived worktree from a committed checkpoint. One writer per listed path; only integration owner changes shared manifests, module declarations, entrypoints, feature map, CI and packaging. Commit a coherent slice before long verification; keep PRs draft until their gates pass. No force pushes or hook bypasses.

## 1. Evidence and seams to preserve

| Current source | Verified responsibility / migration pressure |
|---|---|
| `src/main.rs` (3110 lines at merged checkpoint), `src/ui.rs` (7372) | CLI and GUI both reach `finalize_capture_bundle` (`main.rs:920`); GUI coordinates recording plus onboarding, completion receipt, clipboard and Connect effects (`ui.rs:3301-3462`). Extract common runtime, then caller effects; do not simply move the coordinator into Engine. |
| `src/capture.rs`, `src/capture/{macos,linux}.rs`, `src/audio.rs`, `src/model.rs` | Shared recorder/session, native mechanisms and saved model. Platform module is selected by cfg; preserve capture target resolution and no-frame failure behavior. |
| `src/agent_capture.rs:1-164,251-382` | Agent commands currently control the running GUI over a same-user socket with listener ownership lock. Receipts are explicitly process-lifetime, not durable. This is not standalone Agent readiness. |
| `src/retention.rs:138-187` | Bundle claims and recording counters are process-local. They cannot protect readers/writers after splitting the executable. Preserve existing pin/pending markers while introducing cross-process leases. |
| `src/handoff.rs:1135+` | Existing locked read/modify/atomic-publication primitive already preserves unrelated JSON. Extract it intact; keep caller policy out. `src/hosted.rs` uses queue → config lock order and consent/grant generations. Never introduce a second settings store. |
| `src/{recover,inbox,phone,export/*,sketch,render,keyframes}.rs` | Local archive recovery/read/export/format behavior stays Capture-owned, including saved historical device bundles. Live iPhone retirement has separate draft #185; do not silently implement or reverse it here. Windows is unsupported. |
| `hosted/src/*.ts`, `src/hosted.rs`, `site/src/hosted/{Hosted.tsx,api.ts,hosted.css}` | Connect service, native client and browser UI are split across three roots. Separate them without changing device scopes, quota/retention, auth, upload revocation or complete-file transfer. |
| `site/src/{main,entry-server}.tsx`, `site/scripts/{prerender.ts,package-apparent.mjs}`, `scripts/cd/*` | Browser routes, SSR and public packaging are coupled. Public package intentionally excludes feeds, agent files and unlinked feature map; preserve allowlisting and private-artifact exclusion. |
| `build.rs`, `.cargo/config.toml`, `scripts/{install-app.sh,release-app.sh}`, `scripts/linux/*`, `.github/workflows/*` | Generated helper/resources, signed identities, CPU baselines, release artifacts and web delivery depend on current paths. Root wrappers may remain thin; migrate their real dependencies, not just imports. |

Read before assigned implementation: `AGENTS.md`, `docs/{capture,keyframes-and-text,onboarding-and-settings,linux,export-and-retention,hosted-api,hosted-client,hosted-web,native-launch,release}.md` for the relevant seam. Some docs still describe retained phone acquisition; live code and explicit scope decisions must be reconciled, not guessed.

## 2. Non-negotiable contracts

- Engine returns completed bundle identity/path plus structured diagnostics; no clipboard, paste, upload, browser-open, onboarding or login effects. Effects remain explicit Desktop/Agent caller choices; Connect completion receives only caller-authorized work.
- One recorder, telemetry clock and finalizer for both callers. Agent installation must run without Desktop, GPUI, webview or GUI event loop. A graphical session/portal may still be an honest OS capability prerequisite; report unavailable versus permission-required versus ready separately. Never claim GUI-free recording means bypassing OS consent.
- Common session admission across processes: one owner, opaque operation ID, request deadline/idempotency, wrong-owner stop/cancel refusal, terminal durable receipt, crash/restart reconciliation, stale-lock handling and cleanup. First actual frame establishes recording, not PID or socket existence.
- Retention leases protect recording, finalization, export, reprocess, zoom, fetch/import and transfer reads across processes. Kernel-held ownership must distinguish dead owners from PID reuse; lock path safety and stale state recovery need adversarial subprocess tests. No TTL-only deletion of a live reader.
- One inspectable `~/.config/apparent/config.json` with existing `APPARENT_CONFIG` override; unknown nested keys/secrets, mode, malformed-key behavior and existing read timing survive. Policy-free transaction owns locking/durability; Connect policy owns consent and queue → config lock ordering. No network or nested lock acquisition inside a config transaction.
- Preserve bundle JSON/filenames/timelines, capture roots/overrides, historical metadata, URLs and aliases (`/connect`, `/connect/device`, `/hosted*`), API/OAuth origin, `apparent` CLI plus `connect`/`cloud` compatibility, secret paths, signing/install/update identities and permission grants.
- Changed output/UX stays off by default behind an experiment, with `site/src/features.json` (later owning path after migration) updated in the same behavior slice. File moves alone must produce equivalent output; do not invent a new experiment for unchanged behavior.
- Shared UI exports only primitives/tokens. No Capture/Connect workflow, session, auth, consent or settings state may move there. Linux adapters compose X11, portal, Hyprland and input mechanisms; shared Rust policy chooses behavior from explicit capabilities.

## 3. Executable slice protocol

For every row below: write the named failing contract regression; run it and retain genuine failure; implement only that behavior; rerun focused then owning suite/build; inspect exact diff and an allowed synthetic/native bundle when relevant; commit; push; return exact SHA, commands/exits and remaining gates. Before dispatch, coordinator confirms every listed current path exists and owns all shared-file wiring. New paths below are **planned outputs**, not claimed APIs.

Root Linux verification command (no macOS `taskpolicy` installed here):

```sh
cargo build -j 3 --offline && cargo test -j 3 --offline -- --test-threads=2
python3 -m unittest discover -s tests
python3 -m unittest discover -s scripts/cd -p 'test_*.py'
(cd hosted && npm ci && npm test)
(cd site && bun install --frozen-lockfile)
(cd connect/web && npx --yes bun@1.4.4 install --frozen-lockfile)
(cd site && VITE_HOSTED_API_ORIGIN=https://hosted.tryapparent.com node scripts/build-public.mjs)
git diff --check
```

Use marketing Bun 1.3.10, Connect Bun 1.4.4 and Node 22 (workflow pins 22.23.0); never substitute a different dependency lock. Connect's generated v2 lock cannot be read by 1.3.10. Retain complete command/exit/head/log records outside git. Browser acceptance uses `site/scripts/test-hosted-browser.mjs` against synthetic local accounts and cross-origin API, not live Google or private captures. See web workflow for the exact package build environment.

### Phase A — source mapping already established; implement measured slices

The source table in section 1 is the mapping evidence, not a proposal for four
new investigation-only prerequisites. Use Team Topologies outcome ownership:
keep each product's workflow together, reduce the context an owner must hold,
share thin contracts rather than internal policy, and ship small reversible
slices. Routine technical choices belong to the implementer; consent, publication
and destructive-operation gates still require their actual authority.

Session/retention mapping is already in `agent_capture.rs`/`retention.rs`; config
mapping is the existing locked transaction in `handoff.rs`; bundle consumers are
listed above; Connect mapping is `hosted/`, `src/hosted.rs` and the now-extracted
`connect/web/`. Add executable regressions at each changed boundary while moving
working behavior. Native host selection still needs measured native evidence,
but does not block the independent Connect/config/main seam extractions.

### Phase B — behavior-preserving extraction (sequential integration of parallel-owned leaves)

| Lane | Exclusive existing files / destination | Acceptance and dependency |
|---|---|---|
| B1 config transaction | Isolated config owner splits `src/handoff.rs` into ≤2000-line siblings under `src/handoff/**`; integration adopts the committed seam | Relocate existing primitive, not policy. Multiprocess lost-update/refusal/atomic failure tests preserve full unknown nested object and secret sentinel; paused Settings write cannot resurrect Connect consent. Root facade remains compatible until all callers migrate. Use the mapped queue → config lock order. |
| B2 session ownership and leases | `src/agent_capture.rs`, `src/retention.rs`, `capture/engine/src/session/*`, `capture/engine/src/storage/*`, dedicated subprocess tests | Replace process-local ownership/claims with common admission and lease implementation; two real processes, owner kill, PID reuse surrogate, concurrent cleanup, restart receipt recovery and wrong-ID requests. Keep recorder unchanged in this increment; wire root call sites only through integrator. Use the existing session/archive mapping and B1 transaction seam. |
| B3 artifact processing | `src/{model,audio,keyframes,transcribe,recover,inbox}.rs`, `src/export/*`, `capture/engine/src/{bundle,processing,export}/*` | Move pure model/processing/archive code with test and fixture paths; exact semantic summary/manifest/image equivalence on synthetic bundles, failed finalization cannot publish success. Integrator pre-splits oversized `src/{autocrop,render,sketch}.rs` before assigning those separate files. Use the mapped archive consumers, adding fixture regressions with each moved behavior. |
| B4 Connect service/browser | `connect/web/**`, `site/src/{main,entry-server}.tsx`, `site/scripts/**`, `scripts/cd/**`, web workflow/docs; service remains `hosted/**` | Browser extraction integrated from #200 with attribution, then composed and legacy implementation removed after same-path SSR/browser/pixel parity. One production browser owner; no service move yet. Strict TS/service and synthetic canonical/legacy auth/consent checks remain required. See current integration evidence below. |
| B5 Public | Later migration of remaining `site/src/**` to `public/**`, plus `assets/brand/**` only after assignment | Deferred: marketing stays in `site/` in this slice. Preserve UI/downloads, allowlisting and private-artifact exclusion. Integrator owns feature map, entrypoints and scripts; do not race B4 build wiring. Existing route map is in section 1 and web docs. |

B2/B3 are different file sets but shared runtime integration is serialized. Do not let two workers edit `handoff`, `main`, `ui`, manifests or the same tests under different lane labels.

### Phase C — one real runtime, then Desktop host selection

1. Integrator owns `src/{main,ui,capture,handoff,settings,onboarding,settings_window}.rs`, `src/native/pill/model.rs`, all root manifests/module wiring, `build.rs`, feature map, `site/scripts/**`, `.github/**`, `scripts/**`, packaging and docs references. First extract each touched oversized file into cohesive ≤2000-line siblings with unchanged behavior and owning tests; publish these as small preparatory commits. Preserve inherited hook bytes. No single giant move commit.
2. Engine owner migrates recorder orchestration into `capture/engine/src/{runtime,recording}/*`; integration owner routes existing GUI and timed record commands through it. Keep native implementation shared; compare interaction coverage rather than silently inheriting timed recorder's telemetry gap. Side-effect spies prove Engine never touches clipboard/network/onboarding; terminal bundle receipt survives a newer UI session.
3. Native mechanics owner owns `src/capture/{macos,linux}.rs`, `src/linux/**` and relevant `helper/**` only after each exact file is allocated. Capture adapters move within the relevant capture capability, not a global platform-policy owner. Separate recording/permission probes from UI/login/paste. Shared policy consumes capabilities. Preserve all supported Linux compositor differences; never add Windows work.
4. Agent owner creates `capture/agent/**` only after common runtime/leases land; integrator creates package targets and dependency gates. Install Agent in an isolated home with Desktop absent, check dynamic dependency closure excludes GPUI/webview, and exercise capabilities/readiness/start/status/stop/cancel/receipt plus contention with Desktop. No automatic permission or model download, no fake ready response. CLI compatibility wrapper can remain at root.
5. Host proof owner executes the native host matrix on macOS and disposable Linux environments. Require transparent overlay, selective click-through, ink hit-testing, no focus theft, shortcut parity, multi-display/fractional coordinates, startup/first-frame/input latency, recording inclusion/exclusion matching current controls and privacy policy, accessibility and compositor teardown. Measure p50/p95 against current build and freeze explicit limits before comparing candidates. A host fails if any hard behavior cannot be proven; retain native overlay surface if webview cannot meet it. Do not choose Tauri/Electron/etc. from a feature list.
6. Only after host acceptance, Desktop owner implements `capture/desktop/**` over narrow versioned native commands/events, keeping workflow there and tokens/primitives under `platform/ui/**`. Integrator allocates remaining native UI files and removes superseded production paths only when same-path tests and native acceptance pass. Validate bounded events/backpressure, stale command rejection, reconnect and teardown. Experiments remain off by default.
7. Integrator consolidates Connect native transfer into `connect/client/**`; Capture calls explicit authorized adapter, never owns transfer consent. Complete old→new dependency/import/CI/release/docs-reference audit, clean fresh-checkout builds, root wrapper tests and install/update compatibility. Remove dead duplicate recorders and old imports; directory shape alone is not completion.

## 4. Acceptance ledger (unchecked means NOT proved)

| Gate | Required evidence | State |
|---|---|---|
| F0 | Ancestry, isolated branch, hook unchanged, owned plan, committed/pushed/read-back PR | Plan checkpoint published as draft #199; final evidence commit readback belongs to handoff |
| E1 | One recorder/finalizer used by GUI and Agent; no engine caller effects | Unrun |
| E2 | Real multiprocess contention, restart receipts, crash recovery, retention leases | Unrun |
| E3 | One policy-free config transaction; nested/secret preservation and consent races | In-crate policy-free storage seam integrated; donor storage/subprocess and inherited consent tests pass on combined Linux candidate. Config donor `43bececa` and combined extraction `68b64d86` independently source-approved without blockers; macOS proof pending. Not a separate package |
| E4 | Synthetic old bundles, cleaned media, device timelines, export/reprocess/read/zoom parity | Shared in-crate bundle production seam integrated with root aliases preserved; 10 focused pipeline tests and synthetic demo pass. Combined extraction `68b64d86` independently source-approved: exact 12 donor blobs and pipeline functions preserved. Donor artifact equivalence retained; complete archive/export/native matrix unrun |
| A1 | Fresh standalone Agent installation, no Desktop GUI runtime dependency; real lifecycle | Unrun |
| D1 | Host discriminator native matrix and measured latency on both supported OSes | Unrun; no host chosen |
| D2 | Complete Desktop UX, keyboard/accessibility, error/cancel/reconnect/focus/overlay behavior | Unrun |
| C1 | Connect strict TS/service tests + browser routes/auth/CORS/CSRF/complete transfer/revocation | Browser extraction/composition passed synthetic gates; service unchanged and full 52 pass. Independent auth/privacy source review approved `cd5aa9ec` without blockers, with exact source/artifact preservation; live OAuth/native transfer unrun |
| P1 | Public build/package/browser visual parity, URL/download compatibility, privacy-clean output | Independent marketing/Connect production builds and composed public tests passed; 65 screenshot pairs compared (64 exact, one expiry timestamp). No public directory move or deployment |
| X1 | Linux x86_64/aarch64 build/package + CPU/decoder/encoder checks; native X11 and Wayland (Hyprland/GNOME/KDE) capability/limitations accurate | Unrun platform matrix |
| X2 | macOS supported architectures/minimum OS; Swift helper/Metal, signed identity/TCC persistence, virtual-display native flows and export | Unrun; Linux build cannot prove this |
| R1 | Thin root wrappers, fresh dependency installs, feature map, moved CI paths, signed/install/update assets and docs references | Unrun; no merge/release authorized |

No physical desk capture/audio; use synthetic media, virtual displays or owned disposable VMs. Inspect full resulting movies plus summary/manifest/contact-sheet/frames after behavior changes; a screenshot, `--help`, schema fixture, host build or mock shell is not native capture proof. Existing local capture root was absent during foundation inventory, so no private bundle was read or manufactured as a substitute.

## 5. Large-file policy and baseline record

The effective `.githooks` path is active. `pre-commit` reads staged blobs, rejects added/changed text over 2000 physical lines (including rename destinations, comments and final unterminated line), and leaves unchanged oversized files alone. No extension exemptions; ordinary Cargo lock updates are also subject to the cap. `pre-push` independently requires clean exact HEAD, builds and runs full Rust tests. Do not bypass/disable either hook.

At merged checkpoint, oversized migration inputs include `src/ui.rs` 7372, `src/handoff.rs` 3388, `src/settings.rs` 3185, `src/main.rs` 3110, `src/render.rs` 2522, `src/autocrop.rs` 2469, `src/capture.rs` 2446, `src/sketch.rs` 2448, `src/settings_window.rs` 2330, `src/native/pill/model.rs` 2308, `src/onboarding.rs` 2201, `src/capture/macos.rs` 2070, `site/src/styles.css` 4096 and `Cargo.lock` 8226. A fixture is also oversized. A pure rename of one fails; split it coherently in the same commit before changing its path. Do not churn Cargo.lock; any necessary dependency change needs a policy-compatible lock strategy explicitly resolved with coordinator, never an exemption smuggled into this migration.

Foundation test logs live outside git. Initial source checkpoint is `3b95d12430d4113394ea85155163a5f8173bf8d8`; only this non-embedded plan changes afterward. This file will record exact observed results before handoff; no native GUI, live OAuth, deployment or release proof is implied by baseline checks.

- All baseline checks below exited **0**. Evidence root: `/srv/clanker-hermes/cache/scratch/capture-boundaries-baseline/`. Each initial check has `<name>.json` (command/cwd/exit/duration) and `<name>.log` (including starting HEAD). Sources were `3b95d124` or first plan checkpoint `415c43fe`; all product/build inputs are identical between these and the final docs-only commit. No claim of fresh native proof.
- `cargo build -j 3 --offline` (`rust-build.log`) and `cargo test -j 3 --offline -- --test-threads=2` (`rust-test.log`): **664 passed, 3 ignored**, no failures. Do not sum nested subprocess summaries. Ignored: `handoff::sync_live_check::sync_to_pro`, `pause_transcribe::tests::compare_real_audio_with_batch`, `speech::tests::downloads_and_runs_parakeet` (external Mac/real-audio/download probes). Linux uses this actual checkout, with platform modules present; `taskpolicy` is absent.
- `python3 -m unittest discover -s tests`: **22 passed**, `hooks.log`. `python3 -m unittest discover -s scripts/cd -p 'test_*.py'`: **13 passed**, `delivery.log`.
- `npm ci --prefix hosted` and `(cd hosted && npm test)`: **52 passed**, no skips (`hosted-install.log`, `hosted-test.log`); test script includes strict TypeScript build and Miniflare Worker/D1/R2 integration. No production service requests.
- Bun **1.3.10** was installed under the evidence root because Bun was absent. Node **22.23.4**, npm **10.9.9**; CI Node pin is **22.23.0**, so this is local major-version-matched proof, not the exact CI environment. `(cd site && bun install --frozen-lockfile)` passed without lock edits.
- With `APPARENT_PUBLIC_SITE=1 VITE_HOSTED_API_ORIGIN=https://hosted.tryapparent.com`: `(cd site && npm run build && node scripts/package-apparent.mjs)` passed (`site-build.log`, `site-package.log`), including TS, client, SSR and prerender. Public package generated locally only, never deployed. `node --test scripts/test-brand-favicon.mjs scripts/test-github-stars.mjs scripts/test-latest-download.mjs scripts/test-platform-os.mjs scripts/test-public-homepage.mjs scripts/test-video-demo.mjs`: **18 passed** (`site-tests.log`).
- Rebuilt/repackaged with `APPARENT_PUBLIC_SITE=1 VITE_HOSTED_API_ORIGIN=http://127.0.0.1:18809`, then `(cd site && HOSTED_TEST_API_ORIGIN=http://127.0.0.1:18809 SITE_EVIDENCE=<evidence-root>/browser-evidence npm run test:hosted)`: **29 synthetic cross-origin scenarios passed**, count independently checked against receipt list (`site-cross-build.log`, `site-cross-browser.log`, `browser-evidence/receipt.json`). Canonical/legacy URLs, approval scope/readback, CSRF refusal, expiry, errors and responsive layouts exercised. Desktop computer setup and narrow upload-approval screenshots inspected: command, checkbox, scope and action readable without clipping. Controlled synthetic API only; not live OAuth, deployed service, native or actual browser-zoom proof.
- Initial and final publication use the unmodified pre-push hook (fresh Linux build/full Rust suite); logs `push-initial.log` and `push-final.log`. Foundation self-review requires only this plan plus the three inherited #198 files versus pinned main; no independent review spawned because this slice changes no runtime behavior, and nested agents are explicitly excluded.
- VM launcher and platform skills inspected; no guest started or worker app replaced. Native baseline is explicitly unrun, not unavailable. This docs-only slice does not require new GUI proof.
- Draft plan PR: https://github.com/tryapparent/apparent/pull/199, base `main`. Existing #198 remains untouched; main stayed at `dd801e42` through foundation fetch/readback. No issue comments, merge, release, app install, user preference writes or outbound messages to people.

### Connect integration evidence (2026-10-31)

- Source commits `a5134f5e` and `6123548ed5616683725a6723337fa147ab3418d9`
  adopted with attribution as `303f5ccd` and `b5f97635`; composition checkpoint
  `0af67331`. PRs #199 (integration) and #200 (source) remain separate review
  records; neither was merged or deployed by this worker.
- `connect/web` is the sole production browser implementation. Marketing no
  longer imports or renders it. Existing canonical/legacy URLs and query strings
  are retained by composing independent SSR and referenced hashed assets into
  the existing public allowlist. Different bytes at the same asset path fail
  before replacing the previous output; equal parent bytes must also have equal
  transitive dependencies. Private pitch/capture/docs/feature-map output is not
  admitted wholesale.
- Independent Connect 3 artifact/32 browser cases; composed original 29 and
  independent 32 browser cases; service 52; Python hook 22/CD 15; initial public
  tests 23 passed, with the later dependency collision regression now GREEN.
  65 visual pairs compared: 64 identical, one dynamic expiry timestamp only.
  Full evidence and exact scope limits are in `connect/web/VERIFICATION.md`.
- Both published checkpoints ran unmodified pre-push build/full Rust suites
  (664 passed, 3 ignored). Auth/API code, Cargo/locks, native sources and helper
  files are unchanged. Independent auth/privacy source review approved frozen
  `cd5aa9ec315b10d3ec02e65b39c750d9d6b916e9` without blockers, including exact
  source/artifact preservation. This bounded review is not native/macOS proof
  or merge acceptance. Live OAuth, native transfer and external
  deployment remain unrun, not fabricated prerequisites for byte equivalence.

### Config and bundle pipeline integration evidence (2026-10-31)

- Starting identity: clean `integration/capture-boundaries-plan` at
  `cd5aa9ec315b10d3ec02e65b39c750d9d6b916e9`. Adopted config commits
  `8b623318`, `cc345856`, `552e9e87`, `43bececa`, then pipeline commits
  `3239708b`, `fd4681d9`, `8262e72e`, all serially with `cherry-pick -x`.
  The older foundation document commit `415c43fe` was not imported.
- Combined code checkpoint: `7ccf30ed6fbc0cbc46123e62c037dbc6d0ad79ce`, tree
  `030289ffa93725ef7261853cb07599094582e9d1`. All eight `handoff` files match
  config donor `43bececabc8d31551a72df7ca8cdb7de0d7f7189` exactly; `main.rs`
  and all three pipeline files match donor `8262e72e09100a9be14ac348efa05c0ab22088f9`
  exactly. No semantic reconciliation was needed. Every other baseline tracked
  path was byte-identical before this ledger update, including all Connect/web,
  hook, manifest/lock, build, helper and CI artifacts. This document is the sole
  intentional exception afterward. Changed text files remain below 2000 lines.
- Private combined evidence: `/srv/clanker-hermes/cache/scratch/capture-seams-integration/`.
  `01-checkpoint-push.log` records the unmodified pre-push hook's fresh
  `cargo build -j 3 --offline` and full
  `cargo test -j 3 --offline -- --test-threads=2`: **671 passed, 3 ignored**,
  zero failures, plus 10 passing hook-guard tests. Linux has no `taskpolicy`.
  `02-focused.log` records nonempty filters: `handoff::` **40/1 ignored**,
  `config` **36**, `hosted::` **31**, `feature_map` **2**, `bundle_pipeline::`
  **10** passed. Nested child summaries are not added to the parent totals.
  Receipts retain commands, exits, source tree identities and log SHA-256 hashes.
- Source-based feature audit reviewed: it still finds `handoff::Config` in the
  facade and literal experiment constants; both compiled scanner tests pass.
  `git diff --check` passes. Scoped file-based rustfmt checks pass on all eight
  config files and pipeline demo; `main.rs`, pipeline `mod.rs` and `tests.rs`
  retain donor formatting drift. Exact donor/candidate formatter diagnostics
  match for each file (`format-baseline-file-checks.json`). The failing scoped
  invocation remains recorded, not presented as green. No formatting repair was
  mixed into the reviewed bytes. An initial stdin-only rustfmt probe did not
  diagnose file drift and was superseded by the actual file-based comparison.
- Fresh synthetic `apparent demo --out <private-evidence>/synthetic.apparent`
  succeeds with the rebuilt binary; summary, manifest, contact sheet and both
  frames were inspected. Number chips, click mark and red ink are present. This
  fixture contains authored transcript and no recorded audio/movie, so its
  no-microphone warning is expected; it is not real capture or native proof.
  Existing config donor evidence (full 671/3, standalone storage harness 8) and
  pipeline donor artifact comparisons retain their original identities and are
  not claimed as freshly rerun independent/native acceptance here.
- Config donor `43bececabc8d31551a72df7ca8cdb7de0d7f7189` received an independent
  APPROVED source review without blockers. Its
  nonblocking follow-up is to bound the child `.status()` wait in
  `config_path_override_is_process_local`; reviewed bytes are unchanged here.
  No test edit is included in this docs integration. Independent combined Capture
  extraction source review approved `68b64d86e317ee1eb85f194e4ecf28a1bf5ad30b`
  without blockers: all 12 donor blobs and pipeline function bodies are preserved,
  and the Linux **671 passed, 3 ignored** evidence above retains its original
  source identities. These bounded source reviews are not native/macOS proof
  or merge acceptance. No nested reviewer was spawned. Coordinator still owns
  macOS/native proof, standalone Engine/Agent wiring, cross-process
  admission/receipts/leases and web Desktop host selection/implementation.
  These in-crate seams do not complete
  the refactor. Draft #199 stays draft; no merge, deployment, installation,
  live config change, real-screen capture or outbound delivery was performed.

### Docs-only integration and outstanding Engine repair (2026-10-31)

- Docs integration starts from clean `68b64d86e317ee1eb85f194e4ecf28a1bf5ad30b`,
  tree `c9cd3941c0f019a6546e80b40235976a8a27b4e2`. Repair
  `fef48b0bebc5f6baa35821c6e3633c2c57d5fef6`, verified on remote
  `t3/repair-hosted-doc-recipes`, is adopted with `cherry-pick -x` as `284f98b6`.
  Only `site/docs/clanker-deploy.md`, `site/docs/hosted-design.md` and this ledger
  change. The recipes match `site/scripts/build-public.mjs`, the package composer
  and `docs/hosted-web.md#verification`: production builds both products before
  composition; synthetic browser checks use matching loopback build/test origins
  and `CONNECT_BUILD_DIR=dist-test`. No build, browser run or deployment is implied
  by checking these commands against their owners.
- All Rust, Connect/browser, script, hook, manifest/lock, helper, build and CI bytes
  stay identical to `68b64d86`; the two adopted docs match the donor exactly.
  Compile-time `include_str!`/`include_bytes!` and build-script input inspection
  found none of these three docs embedded. Existing source, artifact, browser and
  Linux test evidence is reused at its recorded SHA, not relabelled as new proof.
  Only the unmodified publication hooks rerun their required checks. Exact scope
  checks, command exits and hook logs are retained outside git under
  `/srv/clanker-hermes/cache/scratch/capture-docs-integration-20261031/`;
  final head/tree and remote/draft readback belong to the handoff.
- Standalone Engine draft #202 at `46922056d6b9381a359a3408329cd1d6ea4c6b1a`
  has a bounded source-review verdict of **REQUEST_CHANGES** for unsafe trust of
  the final bundle directory and incompatible legacy IDs. Repair is actively
  running in a separate worktree; this records no repaired or approved candidate.
  That package is not production-wired. Standalone Agent/Engine wiring and web
  Desktop remain unfinished. #199 remains draft; this is docs integration only,
  not refactor completion, native acceptance or permission to merge.

### Coordinator handoff / next dispatch

Implement the mapped seams, not A1–A4 investigation-only jobs. The sole integration
writer owns this shared branch. Isolated main seam ownership is `src/main.rs`,
`src/bundle_pipeline/**`, `src/cli_dispatch/**`; isolated config seam ownership is
`src/handoff.rs`, `src/handoff/**`. Do not edit across those boundaries. Adopt
their committed, tested slices serially; no Cargo/build/helper churn. Outstanding
acceptance dependencies remain: unchosen native host, no standalone Agent runtime,
process-local ownership/leases, native platform matrix and oversized source/lock
constraints. Other open PRs are not permission to fold unrelated features here.
