# Desktop host ruling — independent acceptance spike

## Ruling

**Use an Electron sidecar as the next ordinary-window candidate. Keep the existing
GPUI/native overlay as the production default. Do not approve a full Desktop
host migration.** The sidecar can eventually host settings/library/workflow
views; it must not own ink, the picker, capsule, shortcut/input capture, recording,
permission handling or retention merely because those controls have web markup.

This is an executable discriminator, not a new app architecture. The real browser
control round trip and Electron binary readiness pass. Native Electron window,
preload IPC and overlay acceptance remain unrun. Phase C line 100 / D1 is NOT met;
Phase C line 101 / D2 and any production Desktop migration remain blocked.
No claims about Connect/Public migration completion follow from this spike.

## Repository mechanics, not a framework vote

Inspected against parent `9a678faf` in `docs/plans/capture-boundaries-migration.md`:

- Root `Cargo.toml` already uses GPUI 0.2.2, AppKit/ScreenCaptureKit on macOS,
  and X11/XFixes/Shape, ashpd, zbus and GStreamer on Linux. It has no Wry,
  Tao, Tauri or Electron host dependency. This package changes none of that.
- `site/package.json` and `connect/web/package.json` use React 19.3 and Vite 8.3.4.
  An ordinary web renderer can carry that UI later without sharing their auth,
  SSR, deployment or build policy. This diagnostic uses DOM controls to avoid
  importing either product or choosing their production wiring for the integrator.
- `src/ui/window_host.rs` already coordinates transparent picker/pill windows,
  focus and restacking, with window IDs in the capture exclusion list.
  `src/ui/macos.rs::configure_floating_window` sets clear background, mouse
  ignoring and status-window level. `src/capture/macos.rs` explicitly updates
  ScreenCaptureKit filters when excluded window IDs change. A new host cannot
  replace these with CSS transparency or a content-protection boolean.
- `src/linux/overlay.rs` already has the difficult compositor behavior: native
  X11 input shape, Shift-time pointer ownership, geometry/scale transforms and
  hiding between strokes on KDE/Hyprland Wayland so controls remain usable.
  `Cargo.toml` explicitly routes GPUI windows through XWayland on Wayland.
  This is working product-specific machinery, not disposable UI boilerplate.

| Candidate | Actual mechanical consequence | Decision |
|---|---|---|
| GPUI + current adapters | Existing input/capture/focus/geometry policy; no new renderer process. Does not itself host the existing web UI. | Keep overlays/default. |
| Wry + Tao, directly or via Tauri 2 | Native system webview; attractive Rust boundary. Wry requires GTK/WebKitGTK on Linux and GTK event-loop integration; child webview raw-handle path is X11-only, with GTK recommended for X11+Wayland. Neither cached host crates nor `gtk+-3.0`/`webkit2gtk-4.1` pkg-config modules were present here. Adding it now requires a new native build/event-loop seam and cannot be exercised under this lane's no-system-install constraint. Tauri adds application/capability configuration but does not remove those overlay issues. | Defer; not ruled out on performance or quality. |
| Electron 45.2.1, separate process | Local pinned npm dependency; same Chromium renderer for supported systems; sandboxed preload can expose exactly two diagnostic operations without entangling GPUI's event loop. Real binary ran in Node mode here. Cost: a separate Chromium runtime/process tree; unpacked binary tree measured 286,782,319 bytes here, not a startup or RSS measurement. | Choose for this bounded ordinary-window candidate, NOT overlay acceptance. |

Do not embed a webview inside the working GPUI overlay merely to avoid a sidecar.
Wry's GTK-loop requirement and existing GPUI XWayland behavior make that a native
integration experiment of its own. Do not introduce a second recorder or hidden
preference store. Real integration must use the integrator-owned versioned
Engine/Desktop boundary, existing config transaction and consent rules.

## Primary API evidence and why it does not prove acceptance

Evidence retrieved 2026-10-31. Electron links are pinned to the installed 45.2.1
source, not inferred from a browser. Wry/Tao/Tauri links describe upstream APIs;
no such runtime was installed or executed.

| Requirement | Documented mechanism / limitation | Consequence |
|---|---|---|
| Transparent, stacked overlay | [Electron custom window styles][styles] supports `transparent`, but explicitly says transparent areas are not click-through and transparent windows are not reliably resizable. [BaseWindow][window] says always-on-top is unsupported on Wayland. | Native overlay remains the only default; a transparent page proves nothing about desktop placement/stacking. |
| Selective click-through + ink hit-test | [BaseWindow][window] `setIgnoreMouseEvents` ignores the whole window; forwarding mouse motion is macOS-only among our supported platforms. On Wayland the input region takes effect on the next frame. Experimental Linux `setShape` clips both drawing and interaction; it is not a cross-platform independent hit region. | CSS `pointer-events` cannot route clicks to another application. Native split surfaces/input-shape adapter and transition/race proof would still be required. |
| Focus + shortcuts | [BaseWindow][window] `showInactive` does not focus, while `show` does; runtime `setFocusable` is macOS-only among our supported systems. [Constructor options][options] documents Linux `focusable:false` changes WM interaction. [globalShortcut][shortcuts] reports registration success/failure and defaults to the portal path on Wayland in this pinned version. | Shortcut registration is not Shift press/release observation, input-device telemetry or no-focus-theft proof. The spike deliberately registers none. |
| Fractional/multiple displays | [screen][screen] exposes displays/scale changes and DIP coordinates, but global cursor position is unsupported on Wayland and physical↔DIP point conversions are unsupported there. | Browser device scale/resizing is not native mixed-display proof. Keep the existing native global-pixel ↔ logical geometry contract. |
| Accessibility | [Electron accessibility][a11y] uses HTML semantics and Chromium's accessibility tree, with assistive-technology activation. | Browser AX names/status and keyboard controls can be checked now. VoiceOver and Linux AT-SPI, focus order across native/web surfaces and screen-reader announcements still need real native tests. |
| Recording privacy | [BaseWindow][window] `setContentProtection` is macOS-only among supported systems and sets `NSWindowSharingNone`; it does not specify Apparent's recording/filter policy. Apple's [ScreenCaptureKit exclusion initializer][sck] takes explicit windows. The [ScreenCast portal][portal] has session/source-selection/PipeWire semantics, not a portable per-window exclusion promise. | Keep native capture/filter ownership. Test movie, keyframes and bundle separately; preserve documented Linux limitations instead of promising exclusion from a window flag. |
| Startup/input latency | [Electron performance][perf] explicitly requires measurement/profiling; `ready-to-show` is only a show lifecycle hook. | A warm browser navigation or binary Node mode is not native cold startup, first visible frame, ink latency or memory proof. No native latency numbers are reported. |
| Renderer boundary | [Context isolation][isolation] and [IPC][ipc] support a narrow preload and sender validation. | Implemented here, but only browser HTTP transport and binary readiness executed. Native preload IPC, crash/teardown and permission denial need the parent-owned native proof lane. |

[window]: https://raw.githubusercontent.com/electron/electron/v45.2.1/docs/api/base-window.md
[options]: https://raw.githubusercontent.com/electron/electron/v45.2.1/docs/api/structures/base-window-options.md
[styles]: https://raw.githubusercontent.com/electron/electron/v45.2.1/docs/tutorial/custom-window-styles.md
[shortcuts]: https://raw.githubusercontent.com/electron/electron/v45.2.1/docs/api/global-shortcut.md
[screen]: https://raw.githubusercontent.com/electron/electron/v45.2.1/docs/api/screen.md
[a11y]: https://raw.githubusercontent.com/electron/electron/v45.2.1/docs/tutorial/accessibility.md
[perf]: https://raw.githubusercontent.com/electron/electron/v45.2.1/docs/tutorial/performance.md
[isolation]: https://www.electronjs.org/docs/latest/tutorial/context-isolation
[ipc]: https://www.electronjs.org/docs/latest/tutorial/ipc
[sck]: https://developer.apple.com/documentation/screencapturekit/sccontentfilter/init(display:excludingwindows:)
[portal]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.ScreenCast.html

Other compared primary references:
[Wry platform/event-loop/child webviews](https://docs.rs/wry/latest/wry/),
[Tao window builder](https://docs.rs/tao/latest/tao/window/struct.WindowBuilder.html),
[Tauri window API](https://v2.tauri.app/reference/javascript/api/namespacewindow/).

## Exact open gates for the native proof owner

All are **UNRUN**. No VM, live application, native GUI capture, install, config,
credentials, paid macOS session or global shortcut was touched in this lane.
Use the existing native proof ownership; do not duplicate another worker's VM.

1. **Freeze the comparison before running it.** Pin candidate/current builds,
   supported macOS versions/architectures and X11 + Wayland GNOME/KDE/Hyprland
   environments. Record baseline sample count, p50/p95 and approved numeric
   budgets for cold launch→first visible frame, hotkey→overlay and input→ink,
   plus RSS/idle CPU. No baseline or numeric acceptance limits have been frozen
   by this spike; D1 cannot pass until the proof owner does so. Browser timings
   must be labeled separately, never substituted.
2. **Transparency/stacking/hit testing.** Checkerboard app under ink, picker,
   pill and ordinary web window. Prove desktop pixels, correct z-order, idle
   clicks reach the underlying app, controls remain clickable and Shift switches
   ownership correctly. Test compositor remaps, display moves, cancellation and
   teardown; no stranded input region/window. Failure retains native surfaces.
3. **Focus and shortcut parity.** Record focus before/after open, start, draw,
   cancel, stop and close. Prove current record/handoff shortcuts, Shift down/up,
   Escape/keyboard picker behavior, registration conflicts and release cleanup.
   Ordinary settings windows may intentionally take focus; overlays must not.
4. **Display geometry.** At least two displays with unequal/fractional scale,
   non-zero/negative origins, rotation and hotplug. Verify native pointer, ink,
   cropped keyframes, target rect and recorded pixels align across boundaries.
5. **Privacy/capture parity.** Use synthetic targets, not desk/personal media.
   Exercise current include/exclude control modes and permission deny/revoke.
   Inspect complete output video, `summary.md`, `apparent.json`, contact sheet and
   frames. Native window IDs/filter updates must reach the existing recorder;
   Linux inclusion limitations stay explicit. No browser permission prompt can
   establish native recorder readiness or capture exclusion.
6. **Accessibility/error/reconnect/teardown.** Real VoiceOver and Linux AT-SPI,
   keyboard-only flow, focus return and live announcements. Disconnect/crash and
   reopen renderer without replaying capture mutations; bound queues/backpressure,
   preserve terminal receipt across newer sessions, reject stale epoch/revision,
   close sidecar without stopping a live capture, and close capture without
   leaving sidecar/native surfaces or shortcut registrations behind.
7. **Packaging boundary.** Parent tests signed/update identity and TCC persistence
   on macOS, Linux package/runtime closure/compositor selection, supported minimum
   OS and fresh install. Agent dependency closure must remain free of Electron,
   GPUI/webview and desktop state. Do not move package/CI/feature-map wiring until
   accepted; no Windows work. Connect tokens/transfers remain in its authorized
   adapter, and Public assets remain independently built/privacy allowlisted.

## Smallest next integration step

Have the native proof owner open this **ordinary diagnostic window** alongside
an unchanged native build in its disposable environment and exercise the actual
preload IPC + close/reopen path. If that passes, the integrator can allocate one
off-by-default read-only ordinary-window experiment over its real snapshot/event
contract. Do not attach `capture.start`, settings writes or Connect transfer to
this diagnostic bridge; none has lifecycle, consent or receipt proof here.
