# Linux

Read this before changing `src/capture/linux.rs`, `src/ui/linux.rs`, `src/linux/`, `src/ink_view.rs`, `src/speech.rs`, or anything behind a `platform::` call in `src/capture.rs` and `src/ui.rs`. `docs/windows.md` explains the shared shape (one GPUI UI, per-OS shells); Linux is the third shell beside macOS and Windows.

## X11 everywhere, Wayland around it

GPUI is built with its `x11` feature only, and `linux::prepare_process` (the first thing `main` does) hides `WAYLAND_DISPLAY` from it, so on Wayland every Scrawl window is an XWayland window (`SCRAWL_NATIVE_WAYLAND=1` opts out, for experiments). That keeps one window model on both: Scrawl's floating windows (pill, picker, ink overlay) are override-redirect X windows it places itself in global pixels (`linux::x11::float`, `place`), with an empty input shape to let clicks through (`set_click_through`). `float` waits for the window manager to finish mapping and then unmanaging the window before it takes it over; otherwise GNOME on Wayland keeps drawing it at its old frame's offset.

GPUI 0.2.2's X11 window panics on `HasWindowHandle`, so `x11::window_id` finds a GPUI window's X id itself (the newest `_NET_WM_PID` window it hasn't seen, cached per `WindowId`).

Global coordinates are root-window pixels; X11 has one scale for the screen (`x11::ui_scale`), so `MAX_CAPTURE_SCALE` is 1 as on Windows.

## What each session can do

Verified in an Ubuntu 24.04 arm64 VM with GNOME 46 (Wayland and X11) and Plasma 5.27.12 (Wayland). Plasma 6 and GNOME 48+ portal shortcuts still need desktop testing.

| | X11 | GNOME on Wayland | KDE Plasma on Wayland |
|---|---|---|---|
| Shortcuts | `XGrabKey` | GNOME custom keybindings running `scrawl shortcut record\|paste` (GNOME 48+: the GlobalShortcuts portal) | Plasma 5: native KGlobalAccel; Plasma 6: GlobalShortcuts portal, with KGlobalAccel fallback |
| Recording a display or area | `ximagesrc` | `org.gnome.Mutter.ScreenCast.RecordArea`, no dialog | ScreenCast portal, asks once (restore token in `~/.local/share/scrawl/portal.json`) |
| Recording a window | `ximagesrc xid=` | X11 apps by id; native Wayland windows are not listed | X11 apps by id; native Wayland windows are not listed |
| Clicks, keys, ⇧ ink | the X server (pointer and key state, polled) | the input devices (`/dev/input`, needs the `input` group) | same |
| Pointer position | the X server | XWayland while it's over X11 windows; an absolute pointing device (tablet, touchscreen, VM or remote desktop) from the input devices; the overlay takes the pointer while ⇧ is held | same |
| App in front | `_NET_ACTIVE_WINDOW` | AT-SPI when the app exposes it (`linux::atspi`; X11 apps use XWayland) | AT-SPI when available; Kate did not expose context in the test VM |
| Paste | XTEST `Ctrl+V` | RemoteDesktop portal, asks once | same |

### Clicks and ink on Wayland

Wayland tells no app where the pointer is over another app's window. Scrawl reads buttons and keys from the input devices. In onboarding or Settings › Permissions, **Copy command** copies `sudo usermod -aG input "$USER"`. Run it in a terminal, then log out and back in. Scrawl checks whether it can open actual keyboard/mouse devices, rather than trusting group membership; it retries devices when access changes. This is the **Input devices** row (the Accessibility row on macOS).

While ⇧ is held, the ink overlay takes the pointer (`linux::overlay::take_pointer`), so XWayland reports where it is and the ink follows it; a click while ⇧ is held lands on the overlay, not the app. On KDE Wayland, Scrawl hides the recording overlay between strokes because KWin 5 otherwise blocks clicks into native apps even with an empty X input shape. Ink remains in the bundle, but disappears from the desktop when ⇧ is released.

A click's position is right over X11 apps and with an absolute pointing device. With a mouse or touchpad over a Wayland app, XWayland still has the pointer where it last saw it, so the click's position is stale: the keyframe is cut at the right moment, but its ripple can be in the wrong place. A GNOME Shell extension that reports the pointer would close this gap (see the PR's open questions).

Without the `input` group, a capture on Wayland has no clicks, keys or ⇧ ink outside Scrawl's own windows, and records only the screen and voice.

### Picker and portal limits

Screen and Area work on the tested desktops. The Window picker lists X11/XWayland windows only; use Screen or Area for native Wayland apps. A portal window source exists internally, but the UI does not currently offer it. A press of the record shortcut records the display under the pointer once its keys come up, however long they're held; the tray menu's **Choose What to Record…** opens the picker, which stays open until you choose a target or press Esc. The Mac-only iPhone mode is hidden. Screen sharing can ask again if a desktop does not honor the saved restore token. Finishing while the dialog is open cancels the pending session; a failed or empty capture reports an error rather than producing demo imagery, and `latest` skips its unfinished directory.

Plasma 5 can report successful GlobalShortcuts binding with an empty result. Scrawl validates both returned bindings and uses KGlobalAccel directly on Plasma 5, so the shortcut does not also type into the focused app.

### Hyprland

Tried on Omarchy 4 (Hyprland 0.56, Lua config), Wayland. `linux::hyprland` binds the shortcuts at runtime with `hyprctl eval hl.bind(…)` (a hyprlang config takes `keyword bindd` instead, untested), each running `scrawl shortcut record|paste`. Hyprland consumes the chord, so it doesn't also reach the app in front, and nothing needs the `input` group. The binds are described `Scrawl: record` and `Scrawl: paste`. A chord some other bind holds is left alone and reported as taken; a bind with Scrawl's description (a permanent one in the user's config, which a config reload doesn't drop) is left in place:

```lua
hl.bind("ALT + SHIFT + S", hl.dsp.exec_cmd("scrawl shortcut record"), { description = "Scrawl: record" })
```

A config reload drops runtime binds, so Scrawl binds them again on the event socket's `configreloaded`. The GlobalShortcuts portal can't stand in: `xdg-desktop-portal` refuses an app without an app id, and `xdg-desktop-portal-hyprland` only fires shortcuts the user bound by hand with `bind = …, global, <app id>:<name>`.

Clicks on the pill and the picker need four things, found in Hyprland's source (`ViewHitTester.cpp`, `XSurface.cpp`) and Omarchy's `windows.lua`, because the symptom is always the same: the click lands on the app below.

- Omarchy gives `no_focus` to floating XWayland windows with an empty class and title, which Hyprland's hit test skips. GPUI sets no class on these windows, so `x11::accept_pointer` gives them one.
- For an override-redirect window of `_NET_WM_WINDOW_TYPE_NOTIFICATION` (GPUI's pop-up type), tooltip, menu, splash or utility, the hit test answers "the window in focus" instead of the window. `accept_pointer` makes them toolbar windows, which get the pointer and aren't focused when they map.
- Hyprland takes an override-redirect window's position when it maps it and ignores later moves, so `x11::place` maps the window again when it moves on Hyprland.
- The full-screen ink overlay is a notification window above the pill, so it shares the KDE treatment: mapped only while ⇧ is held (`hide_between_strokes`).

XWayland sees the pointer only over X11 windows and keeps the last place it saw it over the rest, which picked the wrong display for a recording and started an area selection in the wrong place. On Hyprland `linux::cursor_position` asks Hyprland (`j/cursorpos`) and maps the point to the same place on the same output in XWayland's layout (`hyprland::cursor_in_x_space`): XWayland can put a monitor at y 0 where Hyprland has it at y 1080, so outputs are matched by name and the point is moved by the difference.

The clipboard goes through `wl-copy`, as on KDE Wayland.

On Linux, the recording controls offer **Whiteboard** by default.
It opens a separate native window over the ongoing display capture: draw without holding ⇧,
use Undo or Clear, and close/reopen it without losing the current session's strokes. Resizing
reveals more of the fixed paper instead of scaling marks. On Hyprland the window is floated
after it maps so it can move and resize without tiling the browser; the recording remains
one continuous screen-and-voice session. The board appears in the captured video like any
other window.
At finish, a nonempty board also gets its own last contact-sheet tile: the final strokes
on white paper, cropped to their bounds for readability even if the window was closed or
small when recording stopped. The numbered tile and stroke count appear in `summary.md`;
the screen keyframes still show where the board was beside the browser. Clear removes
the drawing from this final tile. An ordinary clipboard copy is text only; use Paste
to attach the contact sheet and text to an agent composer.

With `"experiments": {"audioEqualizer": true}`, the recording capsule also shows five
smooth level bars fed by the captured microphone PCM (off by default). They indicate
input activity only, not who spoke or whether transcription can understand the voice.
The recording cue and Finish remain separate; microphone and intelligibility warnings
retain their own status text rather than borrowing the bars.

`autoPasteDraft` is an off-by-default shortcut experiment. On a Hyprland Wayland recording started by the record shortcut, it remembers the exact focused window before opening Scrawl's UI. It preflights receiver support before changing the completion path: a missing original window or unsupported composer keeps normal routing and Copy. With a supported receiver, after a complete bundle exists it tries the guarded image-first handoff only if that same window is still focused; it never raises another window or presses Send. It consumes the attempt once and, only after successful preflight, skips dev-site/agent routing and the normal completion-time clipboard overwrite. The completed pill and a notification report the manual `contact-sheet.png` + `summary.md` fallback when it skips. The current `draftHandoff` Linux receiver has no verified T3 editor/attachment observer, so **the experiment currently refuses before any paste**; do not enable it as a working chat integration. Completing #3's real unsent composer check and editor-level identity is required before claiming #6's X1 acceptance.

Not done: a click's position still comes from XWayland (see above; `j/cursorpos` would answer it), and a Region capture of the second monitor failed in testing with a PipeWire `not-negotiated` error while Display on it and Region on the first monitor worked.

## Scrawl's own windows are in the recording

Neither X11 nor GNOME's or KDE's screencasts let an app leave its windows out, so the pill and the ink are in `recording.mp4` and the keyframes. The ink is drawn again at the same place on the keyframes, so it doesn't double. The candidate frames used to pick keyframes paint over the pill (`linux::media::own_window_masks`), so its ticking clock never counts as activity.

## Movies, transcription, the rest

- **Movies:** GStreamer (`linux::media`): the source above, `videorate` to 30 fps, the first encoder installed of `x264enc`, `openh264enc` and VA-API's, `mp4mux`. Candidate frames come off a `tee` through an `appsink`; `Recorder::finish` drains it, but a stalled candidate branch must not make Scrawl delete an MP4 whose file sink has already reported EOS. An unfinished encoder or mux still fails after the eight-second deadline. Compositors send frames only when the screen changes, so the last frame is reused until a new one comes. Without an encoder Scrawl records no movie and keeps sharper candidates instead.
- **H.264 decoder:** Scrawl checks its own GStreamer registry at launch and in Settings › Recording. If no usable H.264 decoder factory exists, it warns with the host distribution's installation command (Arch/Omarchy: `sudo pacman -S gst-libav`; Debian/Ubuntu: `sudo apt install gstreamer1.0-libav`; Fedora: `sudo dnf install gstreamer1-plugin-libav`). Scrawl never runs the command. A capture whose MP4 decode fails keeps candidate frames and writes an `Image quality:` explanation in `summary.md`; the Linux GUI also notifies. A decoder factory does not guarantee that a particular stream can be decoded, so the packaging check actually decodes a small H.264 MP4 inside the AppImage.
- **Transcription:** Parakeet through `transcribe.cpp` (`speech.rs`, shared with Windows). As on Windows, Scrawl downloads the model once at launch through `speech_model::ensure` (731 MB with `curl` or `wget`, pinned Hugging Face revision, checked against its SHA-256, into `~/.local/share/scrawl/models`) unless Hex already has one; Settings › Recording shows its progress. A capture made before it lands says so in `summary.md` and is marked `.transcript-pending`, and is transcribed again in place when the download finishes.
- **Tray:** a StatusNotifierItem (`ksni`); left click starts a recording, the menu is the macOS menu bar menu. GNOME shows it with the AppIndicator extension (on by default on Ubuntu). Without a tray, Scrawl says so in a notification, and launching it again opens Settings (`linux::ipc`, a socket at `$XDG_RUNTIME_DIR/scrawl.sock`).
- **Clipboard:** text and PNG through the X clipboard (`linux::clipboard`) on X11 and GNOME; KDE Wayland uses `wl-copy` (bundled in the AppImage) because KWin does not reliably bridge background XWayland selections to native apps.
- **Launch at login:** `~/.config/autostart/scrawl.desktop`; an explicit `launchAtLogin: true` in config refreshes a stale `Exec=` path when the AppImage moves, but respects a desktop-disabled entry. **Trash:** the freedesktop trash. **Captures:** `~/.local/share/scrawl/captures`; **log:** `~/.local/share/scrawl/Logs/scrawl.log`.
- **Command-line tool:** Settings › Agents links `~/.local/bin/scrawl` to the AppImage (`$APPIMAGE`).

## Not on Linux

The cabled iPhone, video export, `agentContext` lookups (`ax_other.rs`), Sparkle updates (**Check for Updates…** opens the releases page) and the dev-site tab URL (`/sdk.js` registrations still route).

## Packaging

`scripts/linux/appimage.sh [binary]` builds `target/appimage/Scrawl-<arch>.AppImage` with linuxdeploy and its GStreamer plugin, so GStreamer, `pipewiresrc`, `ximagesrc` and the H.264 encoders come with it. The binary baseline is **Ubuntu 24.04+**. Install `libfuse2t64` to run the AppImage, or run `./Scrawl-<arch>.AppImage --appimage-extract-and-run app` without FUSE.
The build installs `gstreamer1.0-libav` and `scripts/linux/check-appimage-decoder.sh` creates a tiny H.264 MP4 and asks the packaged executable to decode a full-resolution frame. This checks the packaged runtime rather than the runner's host plugins.

`.github/workflows/release-linux.yml` builds x86_64 and aarch64 on Ubuntu 24.04 runners. Pull requests and manual dispatches produce artifacts and run packaged `--help` checks; `v*` tags upload versioned AppImages to the matching GitHub release. The site links directly to those versioned assets. Build/deploy the site only after the appcast version is updated and both release assets exist. Flatpak is out of scope.

CPU portability (#82): `transcribe-cpp` uses `dynamic-backends`, which forces `GGML_NATIVE=OFF`. On x86_64 it builds separate `libggml-cpu-<tier>.so` modules (including separate Alder Lake and Zen 4 tiers); on aarch64 it builds a portable CPU module. `.cargo/config.toml` sets the Rust Linux baselines to `x86-64` and `generic`, respectively. `build.rs` stages the runtime and module directories next to binaries/tests with `$ORIGIN` RPATH. AppImage packaging explicitly includes the modules that dependency scanning cannot discover, all beside `libtranscribe` in `usr/lib`; `init_backends_default` resolves that library with `dladdr` and selects a CPU tier there once per process.

Both Linux workflows run `scripts/linux/check-cpu-tiers.py` before stripping symbols. It checks the core libraries and all x86 tiers for EVEX/AVX-512, VEX AVX-VNNI and AMX against their declared features, including baseline-only backend scoring. Release CI checks the actual CMake cache has native compilation off, then runs Parakeet on two seconds of audio with only the extracted AppImage's libraries available. The x86 job repeats it under Intel SDE `-icl`: AVX512-VNNI present, AVX-VNNI absent, as on the Ryzen 8700G in #82. It also generates a demo bundle with the shipped executable under SDE, covering Rust post-processing without a desktop or model installation.

Linux previews are published manually from the checked AppImage artifacts as `linux-preview-N`, with `--prerelease --latest=false` and a combined `SHA256SUMS`. These tags do not trigger the `v*` release jobs, and neither the site nor the Sparkle appcast selects them.

## Building and testing

Build dependencies (Ubuntu/Debian; CI installs the same):

```sh
./scripts/linux/dependencies.sh
cargo build -j 2 --offline
./target/debug/scrawl app
```

GPUI needs Vulkan (`mesa-vulkan-drivers`; llvmpipe works in a VM). Tests: `cargo test`. A desktop test runs in a Tart Ubuntu VM on the Mac mini (`macvm`, see `~/code/home/tools/macvm`): GNOME on Wayland by default, X11 with `WaylandEnable=false` in `/etc/gdm3/custom.conf`. Never test on Kit's desktop session.
