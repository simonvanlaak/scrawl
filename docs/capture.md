# Capture: shortcuts, Mac recording, displays

## Give Feedback (experiment)

Set `"experiments": {"feedbackIssue": true}` in `~/.config/apparent/config.json` and restart Apparent. The menu's **Give Feedback…** starts a screen capture on the pointer's display. Finish it normally; Apparent opens the local capture review. **Open GitHub issue…** opens the public feedback form in your browser. It does not create or submit an issue. Describe the problem there; optionally attach only screenshots or video that you have reviewed for private information. No transcript, recording, or local path goes into the browser URL or gets uploaded by Apparent. The regular **Copy** button copies the local handoff to your clipboard; review its contents before pasting them into a public issue.

Read this before changing `src/shortcut.rs`, `src/capture.rs`, `src/screens.rs` or the picker and capsule placement in `src/ui.rs`.

## Shortcuts (`src/shortcut.rs`, `is_capture_shortcut` in `src/capture.rs`)

A press of `⌥⇧S` (or the shortcut set in Settings), however long it's held, records the display under the pointer as soon as its modifiers come up, auto-cropped, with a brief red edge glow from the overlay as the start cue, and the capsule springs open onto switch chips (Area · Window · Screen · iPhone if cabled; `shortcut::Reveal` and its `REVEAL_*` constants) until 3.5 s pass or the user clicks, scrolls or draws (never while hovered). A chip (`switch_recording`) restarts rather than retargets: Window/Area discard the few seconds and morph the capsule into the picker, iPhone discards and starts the phone.

Recording starts on the release rather than the press so nothing is captured while ⌥⇧ is still down (⇧ is the ink key), and Esc before the release cancels. The picker opens only from the menu's **Choose What to Record…** (or a chip), on the pointer's display and on the mode last used; there the shortcut records what it's on, and repeats of a held S never record (`handle_selection_key`'s `repeat`). While recording, a press finishes, ignoring one within 400 ms of the start.

During a screen recording, the capsule shows **Mic unavailable** if the input cannot open, **Mic stopped** if samples stop after a startup grace period, or **Mic very quiet** after sustained near-silent captured samples. Quiet input does not prove broken hardware. The warning is inline beside the timer; the red screen-recording cue and the recording controls remain in place, and the screen capture continues. Finish stays visible when the normal controls are shown; during the existing 3.5-second switch-chip reveal, Cancel is shown instead. The warning clears after sustained input recovery or when recording ends. Phone and simulated captures have no microphone warning. On macOS, five bars beside the timer reflect recent captured PCM and settle on silence or stale samples. They do not indicate speech recognition or determine input health.

The recording controls also open a floating **Whiteboard**. It uses Apparent's Carbon toolbar, lilac mark and controls, Host Grotesk type, and warm Paper drawing surface on both desktops. The drawing stays dark on light paper regardless of system appearance; the final bundle tile remains plain white, without toolbar or branding. Its drawing survives closing and reopening the window during the same capture. Finish closes the window and adds the final drawing as a separate paper frame in the bundle; the next capture starts with an empty board.

With the off-by-default `movableRecordingControls` experiment enabled, drag the recording capsule by its clock/status cluster to uncover the screen beneath it. The whole panel stays on its original display, holds its dropped position for that capture and resets to the top centre for the next capture. The capsule remains in the Mac ScreenCaptureKit movie and in decoded full-quality keyframes; Linux already records it and only masks the small keyframe-selection candidates. The picker/ink overlay and macOS system prompts keep their separate privacy exclusions. See [interaction rationale and test matrix](recording-capsule-drag.md).

Carbon hotkeys and the key event tap report presses through `emit_capture_shortcut` (deduplicated within 90 ms); while a press waits for its release the UI loop polls the modifiers and takes Esc from the key tap (`tick_shortcut`, `take_escape`), so the release is seen whichever window has focus. Esc from the key tap also closes an open picker when another app has the keys.

## Mac captures (`src/capture.rs`)

### Timed CLI recording versus the app shortcut

`apparent record --duration N` starts a separate, timed screen-and-microphone recorder. It does **not** start the app's global input tap or ink overlay. A click may be visible in `recording.mp4`, but no click or Shift mark is collected for `events`, `marks`, click keyframes or ripples. New timed screen bundles say `"interactionTelemetry": "videoOnly"` in `apparent.json` and explain this in `summary.md` (including after reprocessing or journal-backed recovery). An absent field on an older bundle or orphaned movie says nothing about its collection mode. Window/cursor samples and scene-change keyframes are still possible; “video only” describes *interaction telemetry*, not a lack of audio or all other context.

For interaction-aware agent captures, use the already-running app's recording shortcut (`apparent shortcut record` on Linux, or the configured shortcut on Mac) to start and finish. This follows the same permission rules as an ordinary app recording: macOS Accessibility for click/ink collection; Linux input-device access on supported Wayland sessions, or the X server on X11. It must not silently install a global input hook from the CLI, bypass consent, or forward a bundle to a chat. CLI `--iphone` is a separate native video/mirror path, not the timed screen path.

`apparent record --window <name>` resolves against the same selectable windows as the picker. A name must match exactly one on-screen window by title or app (case-insensitive substring); an empty, missing, or ambiguous name fails before a bundle or recorder is created. Without `--window`, the CLI records the selected display. If a selected window disappears before ScreenCaptureKit starts, its filter does not fall back to a display.

The sampler keeps only small candidates (480 px long edge, in a `CandidateBuffer` capped at 384 MB that thins evenly and halves its sampling rate when full, never dropping the start or click/stroke frames). At finish the chosen keyframes are decoded from `recording.mp4` at native size (2560 px Screen, 1280 px Window/Region, never above the display's own scale, so a 1x 1920 pt display records 1920 px), shifted by `videoOffsetMs` (when the movie started on the capture clock); if decoding fails the candidates are used. macOS prompts over a Screen or Area capture (`windows::system_prompts_over`: UserNotificationCenter, SecurityAgent; ScreenCaptureKit's "bypass the system private window picker" alert comes back every 30 days) are left out of the recording through the content filter when ScreenCaptureKit lists them, frames that may show one are dropped from the candidates (`CandidateBuffer::drop_since`), and clicks on them aren't recorded. `apparent capture-sim` drives this headless for memory and timing checks.

Before a screen movie starts, `.unsafe-video` marks its empty output directory as unsafe to recover. A directory with existing capture files is refused rather than reused while an old recovery could be running. A successfully built bundle removes the marker; an interrupted build remains unrecoverable from the raw movie. If every safe screenshot fails, finalization leaves the raw movie in place but creates no bundle. The marker prevents both automatic orphan recovery and `apparent recover` from converting that movie into keyframes: the movie may contain a system prompt excluded by the screenshot sampler. Treat the retained movie as private raw media, not as safe material to paste or reprocess.

On a disposable Mac, a debug build launched with `APPARENT_TEST_DROP_SCK_FRAMES=1` can exercise this failure path without changing the movie recorder or revoking TCC permissions. The flag affects only `capture_sck_frame`; remove it from the launch environment and restart the installed app before a healthy-capture check. Do not use it on Kit's Mac.

## More than one display (`src/screens.rs`)

Every rect and point is global top-left points (`CGEvent`, `CGDisplayBounds`, `SCWindow` frames, `TargetFrame`); displays left of or above the primary one are negative. `screens::flip` converts AppKit frames, `screens::current()` lists displays with menu bar, notch and `backingScaleFactor` (main thread), `screens::displays()` the CoreGraphics view (any thread).

A capture belongs to one display: a tap takes the one under the pointer, Window the one holding the window's centre, Area the one holding its centre (clamped to it; `sourceRect` is display-local via `screens::source_rect`); `TargetFrame.scale` is that display's.

The picker opens on the pointer's display and follows the pointer between displays (window, Metal overlay and capsule: `follow_pointer_display`) except while an area is dragged or iPhone is shown; the overlay `move_to`s the target's display when recording starts and follows the iPhone mirror if it's dragged. The capsule (`NativePill::place_on`) sits on the display in use, `PANEL_Y` below its top or lower for a taller menu bar or the notch (`screens::pill_origin`). Test with a `CGVirtualDisplay` (see `scripts/virtual-display.swift`), never on Kit's desk Mac.

On Linux/Hyprland, the picker and capsule use XWayland windows positioned through the Linux UI adapter. The selected output's RandR rectangle anchors a Region crop within its PipeWire stream; the Hyprland portal mapping ID checks the output name before a multi-output capture can begin. See `docs/linux.md` for the mixed-scale two-output Omarchy VM check and its limits. This is not a claim about other Wayland portal backends or mirrored displays.

## Agent control of the running GUI

`apparent capture state` reads the running GUI over a same-user Unix socket and prints JSON with
`phase` (`idle`, `selecting`, `starting`, `recording`, `processing`, or `completed`) and a
`capture_id` only while a capture is pending or active. `starting` means no real screen frame has
arrived yet (on Linux this includes a pending portal chooser); it is **not** proof of recording.
The ID is random per capture, including a switch to a new target. It is not a bundle path or a
claim of who started the capture. Never cancel a capture merely because one is present: obtain
the ID from a start you initiated and compare it with a fresh state read.

`apparent capture stop <capture_id>` finishes only that capture after a frame arrives;
`apparent capture cancel <capture_id>` discards it even while pending. A mismatched ID or a stop
while still starting fails without acting. An already ended ID returns `already_ended`, so retries
do not toggle, start, or alter a newer capture. These commands control the GUI, not a separate
blocking `record --duration` process. They require a running app and return errors if its control
socket cannot answer. Neither sends a capture to a chat; finishing uses the ordinary local bundle
and clipboard path. A `stopping` reply means finalization has begun, not that the bundle has
already been saved; read state again to observe processing or completion.

## Capture history

In **Open Latest Capture**, the review window shows Newer and Older buttons and a position in the capture history. It reads completed `.apparent` bundles from the configured captures directory in reverse creation order, skips cancelled or invalid manifests, and loads images for the selected capture only. The first and last entries have no button past the end. The review window's Copy and Show in Files act on the capture currently open, not the newest one.
