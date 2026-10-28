---
name: make-it-apparent
description: Use when a change needs a short Apparent video walkthrough.
---

# Make It Apparent: show the change

Use this when someone asks to *see* a finished change, especially a UI flow that prose or a screenshot cannot explain. This is an agent procedure, not a new recording service. Do not make a video merely because a change is complex: if there is no useful visible behavior, explain it in text instead.

## Choose the proof

Name one question the viewer should be able to answer after watching. Show the starting state, the action and the resulting state in that order. Keep it short; do not tour unrelated settings or repeat the implementation story. For a comparison, show both states with the same task. A local preview demonstrates only that build and environment, not a released app or another OS.

Before recording, run the actual app and check the flow yourself. Decide which window or device will be shown. Close or mask personal tabs, notifications, credentials, customer data and unrelated windows *before* capture; cropping afterwards cannot undo their presence in a source recording. Do not upload, publish or send private footage without the user's approval. Do not record on a host where the project's safety rules forbid screen capture or waking a display. On Apparent's development Macs, follow `AGENTS.md` and `docs/capture.md` first.

## Record and review

If Apparent is installed on the machine showing the result, use its installed CLI (`apparent` on PATH, or the binary inside Apparent.app), not a different host's binary. `apparent record --help` gives the current flags. For an unattended, fixed-duration desktop take, target the known window and arrange the UI action while recording:

```sh
apparent record --window 'Window title or app' --duration 15 --out /absolute/path/walkthrough.apparent
```

`--window` matches a title or app substring; confirm the captured window in the result. For a cabled phone use `apparent record --iphone --duration 15 --out /absolute/path/walkthrough.apparent`, when supported. If the action requires interactive control, use Apparent's normal recording shortcut and stop it after the flow, then locate the actual bundle; don't claim that a command alone performed the interaction. Do not assume `latest` is your take when other captures can finish concurrently.

Read the take's `summary.md` and inspect `contact-sheet.png` plus the relevant `frames/` images. Play the actual `recording.mp4` (or `.mov`) from start to finish to catch blank windows, private material, missing steps, silent audio, and bad timing. The sheet is an index, not a substitute for video playback. If an edited, cropped or zoomed share copy helps, `apparent export <bundle> --out /absolute/path/walkthrough.mp4 --zoom` is available where export is supported; inspect that file too. Keep the unedited source private. If the source media is missing or Apparent is unavailable, say so and provide a truthful alternative such as a screenshot or a written trace—never label one a video.

## Hand it back

Link or embed the verified video using the delivery channel's supported attachment/path format. Say in one sentence what it shows and in one sentence what it does *not* establish (for example, only a local build, one display size, or one platform). A filesystem path is useful only when the viewer can access that machine; otherwise arrange an approved delivery route. Never substitute the capture's transcript or a promise to record later for the actual playable artifact.
