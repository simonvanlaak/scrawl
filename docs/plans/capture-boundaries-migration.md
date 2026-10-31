# Capture boundaries migration implementation plan

> **For Hermes:** Use subagent-driven-development for later slices; this foundation slice has no nested agents.

**Goal:** Deliver one recording engine used by independently installable Agent and Desktop clients, separate Connect and Public products, and thin native/UI adapters without changing existing identity or consent contracts.

**Architecture:** `capture/engine` owns local recording, processing, bundles, recovery and retention. `capture/{desktop,agent}` own their workflows; `connect/` owns service/browser/client transfer; `public/` owns marketing/brand; `platform/` owns only OS mechanisms; `ui/` owns only shared primitives/tokens. Move working behavior in tested increments, not empty directory scaffolds.

**Tech stack verified:** Root is one Rust 2024 binary crate (not a Cargo workspace), GPUI 0.2.2; macOS also uses Swift/AppKit/Metal. Linux uses X11/XWayland GPUI, GStreamer, portals and compositor adapters. `site/` is React 19.3/TypeScript/Vite 8, with no shadcn/Radix dependency today. `hosted/` is strict TypeScript, esbuild, Cloudflare Worker/D1/R2, Miniflare tests. React/TypeScript with shadcn/Radix is the proposed Desktop UI, not an existing implementation or chosen native host.

## 0. Frozen foundation and ownership

- Foundation owner may edit **only this file**, plus branch setup. No product source migration, root AGENTS changes, merge, release, installation, issue comments, user config edits or native recording in this slice.
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
(cd site && bun install --frozen-lockfile && APPARENT_PUBLIC_SITE=1 npm run build && node scripts/package-apparent.mjs)
git diff --check
```

Use Bun 1.3.10 and Node 22 (workflow pins 22.23.0); never substitute a different dependency lock. Retain complete command/exit/head/log records outside git. Browser acceptance uses `site/scripts/test-hosted-browser.mjs` against synthetic local accounts and cross-origin API, not live Google or private captures. See web workflow for the exact package build environment.

### Phase A — next independent jobs (parallel; no shared-file writes)

| Lane | Exclusive files it may create | Concrete first deliverable / acceptance |
|---|---|---|
| A1 session contract | `capture/engine/contracts/session.md`, `capture/engine/contracts/session.schema.json`, `capture/engine/tests/session_contract.py` | Specify admission/deadline/ownership/readiness/terminal receipt envelopes from actual `agent_capture` behavior; executable schema vectors for start contention, wrong ID, expired retry, restart and crash. Run `python3 capture/engine/tests/session_contract.py`; vectors alone are contract proof, not runtime acceptance. No standalone worker scaffold. |
| A2 native host discriminator | `capture/desktop/proofs/host-matrix.md`, `capture/desktop/proofs/verify-host-proof.py` | Inventory existing native/GPUI overlay semantics and candidate hosts; verifier must reject missing native evidence for either supported OS. Define instrumented safe fixtures and thresholds relative to measured current UI before choosing host. Run verifier on intentionally incomplete evidence (must fail). No framework install or shell scaffold yet. |
| A3 Connect separation inventory | `connect/migration/transfer-boundary.md`, `connect/migration/check-transfer-inventory.py` | Trace all service/browser/native imports, routes, storage and consent callers; executable manifest requires every current production/test/build input, including outside-package consumers. Run inventory checker against committed source; propose exact old→new paths and preserved public URLs. No edits to `hosted/`, `site/` or `src/hosted.rs`. |
| A4 archive compatibility inventory | `capture/engine/contracts/bundle-compatibility.md`, `capture/engine/tests/bundle_inventory.py` | Enumerate actual model/recovery/export/retention consumers and synthetic historical-format cases; script validates fixture inventory and required artifacts. Include cleaned media and saved device timeline cases. Do not copy personal bundles into fixtures or logs. |

These four jobs can be assigned to separate Luna implementers without module/manifest collisions. Their tools must run against this candidate, not another checkout. No nested agents needed for this foundation slice. Coordinator integrates each committed artifact before authorizing Phase B.

### Phase B — behavior-preserving extraction (sequential integration of parallel-owned leaves)

| Lane | Exclusive existing files / destination | Acceptance and dependency |
|---|---|---|
| B1 config transaction | Integrator alone splits `src/handoff.rs` into ≤2000-line siblings, then assigns `capture/engine/src/config_store.rs` and its dedicated test file to one owner | Relocate existing primitive, not policy. Multiprocess lost-update/refusal/atomic failure tests preserve full unknown nested object and secret sentinel; paused Settings write cannot resurrect Connect consent. Root facade remains compatible until all callers migrate. Requires A3 lock inventory. |
| B2 session ownership and leases | `src/agent_capture.rs`, `src/retention.rs`, `capture/engine/src/session/*`, `capture/engine/src/storage/*`, dedicated subprocess tests | Replace process-local ownership/claims with common admission and lease implementation; two real processes, owner kill, PID reuse surrogate, concurrent cleanup, restart receipt recovery and wrong-ID requests. Keep recorder unchanged in this increment; wire root call sites only through integrator. Requires A1/A4 and B1. |
| B3 artifact processing | `src/{model,audio,keyframes,transcribe,recover,inbox}.rs`, `src/export/*`, `capture/engine/src/{bundle,processing,export}/*` | Move pure model/processing/archive code with test and fixture paths; exact semantic summary/manifest/image equivalence on synthetic bundles, failed finalization cannot publish success. Integrator pre-splits oversized `src/{autocrop,render,sketch}.rs` before assigning those separate files. Requires A4. |
| B4 Connect service/browser | `hosted/**`, `site/src/hosted/**`, `connect/{service,browser}/**` | One production Worker and browser implementation, strict TS negative-type regression, Miniflare full suite and canonical+legacy route/SSR cross-origin browser tests. Preserve auth, scopes, quotas, budgets, grants, tombstones and public assets. Requires A3 and integrator's manifest/route wiring checkpoint. |
| B5 Public | Remaining `site/src/**` excluding `site/src/hosted/**`, `public/**`, `assets/brand/**` | Same marketing UI/downloads, allowlisted publish package, no private pitch or capture assets. No ownership of `site/src/features.json`, `site/src/{main,entry-server}.tsx`, `site/scripts/**` during parallel work: integrator wires these. Requires A3 route map; cannot race B4's build wiring. |

B2/B3 are different file sets but shared runtime integration is serialized. Do not let two workers edit `handoff`, `main`, `ui`, manifests or the same tests under different lane labels.

### Phase C — one real runtime, then Desktop host selection

1. Integrator owns `src/{main,ui,capture,handoff,settings,onboarding,settings_window}.rs`, `src/native/pill/model.rs`, all root manifests/module wiring, `build.rs`, feature map, `site/scripts/**`, `.github/**`, `scripts/**`, packaging and docs references. First extract each touched oversized file into cohesive ≤2000-line siblings with unchanged behavior and owning tests; publish these as small preparatory commits. Preserve inherited hook bytes. No single giant move commit.
2. Engine owner migrates recorder orchestration into `capture/engine/src/{runtime,recording}/*`; integration owner routes existing GUI and timed record commands through it. Keep native implementation shared; compare interaction coverage rather than silently inheriting timed recorder's telemetry gap. Side-effect spies prove Engine never touches clipboard/network/onboarding; terminal bundle receipt survives a newer UI session.
3. Native mechanics owner owns `src/capture/{macos,linux}.rs`, `src/linux/**`, relevant `helper/**` and `platform/**` only after each exact helper file is allocated. Separate recording/permission probes from UI/login/paste. Shared policy consumes capabilities. Preserve all supported Linux compositor differences; never add Windows work.
4. Agent owner creates `capture/agent/**` only after common runtime/leases land; integrator creates package targets and dependency gates. Install Agent in an isolated home with Desktop absent, check dynamic dependency closure excludes GPUI/webview, and exercise capabilities/readiness/start/status/stop/cancel/receipt plus contention with Desktop. No automatic permission or model download, no fake ready response. CLI compatibility wrapper can remain at root.
5. Host proof owner executes A2 matrix on native macOS and disposable Linux environments. Require transparent overlay, selective click-through, ink hit-testing, no focus theft, shortcut parity, multi-display/fractional coordinates, startup/first-frame/input latency, recording inclusion/exclusion matching current controls and privacy policy, accessibility and compositor teardown. Measure p50/p95 against current build and freeze explicit limits before comparing candidates. A host fails if any hard behavior cannot be proven; retain native overlay surface if webview cannot meet it. Do not choose Tauri/Electron/etc. from a feature list.
6. Only after host acceptance, Desktop owner implements `capture/desktop/**` over narrow versioned native commands/events, keeping workflow there and tokens/primitives under `ui/**`. Integrator allocates remaining native UI files and removes superseded production paths only when same-path tests and native acceptance pass. Validate bounded events/backpressure, stale command rejection, reconnect and teardown. Experiments remain off by default.
7. Integrator consolidates Connect native transfer into `connect/client/**`; Capture calls explicit authorized adapter, never owns transfer consent. Complete old→new dependency/import/CI/release/docs-reference audit, clean fresh-checkout builds, root wrapper tests and install/update compatibility. Remove dead duplicate recorders and old imports; directory shape alone is not completion.

## 4. Acceptance ledger (unchecked means NOT proved)

| Gate | Required evidence | State |
|---|---|---|
| F0 | Ancestry, isolated branch, hook unchanged, owned plan, committed/pushed/read-back PR | In progress; see foundation result below |
| E1 | One recorder/finalizer used by GUI and Agent; no engine caller effects | Unrun |
| E2 | Real multiprocess contention, restart receipts, crash recovery, retention leases | Unrun |
| E3 | One policy-free config transaction; nested/secret preservation and consent races | Existing primitive found; extraction unrun |
| E4 | Synthetic old bundles, cleaned media, device timelines, export/reprocess/read/zoom parity | Unrun |
| A1 | Fresh standalone Agent installation, no Desktop GUI runtime dependency; real lifecycle | Unrun |
| D1 | Host discriminator native matrix and measured latency on both supported OSes | Unrun; no host chosen |
| D2 | Complete Desktop UX, keyboard/accessibility, error/cancel/reconnect/focus/overlay behavior | Unrun |
| C1 | Connect strict TS/service tests + browser routes/auth/CORS/CSRF/complete transfer/revocation | Baseline pending; migration unrun |
| P1 | Public build/package/browser visual parity, URL/download compatibility, privacy-clean output | Baseline pending; migration unrun |
| X1 | Linux x86_64/aarch64 build/package + CPU/decoder/encoder checks; native X11 and Wayland (Hyprland/GNOME/KDE) capability/limitations accurate | Unrun platform matrix |
| X2 | macOS supported architectures/minimum OS; Swift helper/Metal, signed identity/TCC persistence, virtual-display native flows and export | Unrun; Linux build cannot prove this |
| R1 | Thin root wrappers, fresh dependency installs, feature map, moved CI paths, signed/install/update assets and docs references | Unrun; no merge/release authorized |

No physical desk capture/audio; use synthetic media, virtual displays or owned disposable VMs. Inspect full resulting movies plus summary/manifest/contact-sheet/frames after behavior changes; a screenshot, `--help`, schema fixture, host build or mock shell is not native capture proof. Existing local capture root was absent during foundation inventory, so no private bundle was read or manufactured as a substitute.

## 5. Large-file policy and baseline record

The effective `.githooks` path is active. `pre-commit` reads staged blobs, rejects added/changed text over 2000 physical lines (including rename destinations, comments and final unterminated line), and leaves unchanged oversized files alone. No extension exemptions; ordinary Cargo lock updates are also subject to the cap. `pre-push` independently requires clean exact HEAD, builds and runs full Rust tests. Do not bypass/disable either hook.

At merged checkpoint, oversized migration inputs include `src/ui.rs` 7372, `src/handoff.rs` 3388, `src/settings.rs` 3185, `src/main.rs` 3110, `src/render.rs` 2522, `src/autocrop.rs` 2469, `src/capture.rs` 2446, `src/sketch.rs` 2448, `src/settings_window.rs` 2330, `src/native/pill/model.rs` 2308, `src/onboarding.rs` 2201, `src/capture/macos.rs` 2070, `site/src/styles.css` 4096 and `Cargo.lock` 8226. A fixture is also oversized. A pure rename of one fails; split it coherently in the same commit before changing its path. Do not churn Cargo.lock; any necessary dependency change needs a policy-compatible lock strategy explicitly resolved with coordinator, never an exemption smuggled into this migration.

Foundation test logs live outside git. Initial source checkpoint is `3b95d12430d4113394ea85155163a5f8173bf8d8`; only this non-embedded plan changes afterward. This file will record exact observed results before handoff; no native GUI, live OAuth, deployment or release proof is implied by baseline checks.

- Baseline run: pending. Linux uses `cargo ... -j 3 --offline`; `taskpolicy` is absent. Runner records command, cwd, source SHA, exit and duration. Rust build uses this actual Linux checkout (platform modules are present).
- `python3 -m unittest discover -s tests`: passed (hook regressions); exact count to be recorded from log.
- `python3 -m unittest discover -s scripts/cd -p 'test_*.py'`: passed; exact count to be recorded from log.
- Backend/site dependency setup and owning checks: pending; private Bun 1.3.10 provisioned outside checkout because no Bun command was installed.
- VM launcher and platform skills inspected; no guest started or worker app replaced. Native baseline is explicitly unrun, not unavailable. This docs-only slice does not require new GUI proof.
