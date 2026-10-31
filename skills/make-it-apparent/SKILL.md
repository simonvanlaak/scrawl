---
name: make-it-apparent
description: Use when told "make it apparent" or asked for video proof.
---

# Make It Apparent: make the situation understandable

Keep raw captures unchanged and private and deliver a separate polished share copy. Inspect the entire take before delivery; never commit generated media.

## Required spoken-style explanation and reusable CLI

Every delivered proof video needs timed natural sentences explaining the action, evidence and result, with matching burned subtitles and synchronized TTS by default. Retain editable cues, SRT and script. This is postproduction explanation, not a transcription or replacement of recorded evidence. Disclose synthetic narration; never play it through speakers during capture. Simon can request a silent captioned copy instead. Suitable live narration need not be duplicated.

Use the repository helper, not a new per-video script. Read `docs/proof-video.md` for setup, cue format and privacy. From the Apparent checkout, write a private `cues.json` array of `[start_seconds, end_seconds, text]` against the existing video's clock, then run:

```sh
uv run scripts/proof-video.py /absolute/private/walkthrough.mp4 \
  --cues /absolute/private/cues.json \
  --out-dir /absolute/private/proof-take-1 \
  --source-audio drop --allow-online-tts \
  --font /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf
```

The output directory must be new. The font example is Linux-specific; select a real regular sans-serif TTF on other hosts. `--allow-online-tts` consents to sending cue text only to Microsoft Edge TTS; never include secrets. Default voice is en-GB-RyanNeural at +10%, not Nova. Use `--silent` instead of that consent flag for silent opt-out. Explicit `--source-audio keep` mixes reviewed original audio; default practice is `drop` to avoid leaking source conversation. Neither choice changes the original. Do not publish media without destination approval.

Use measured `--crop x,y,width,height` for useful close-ups, and put `--caption-box x,y,width,height` in empty output space, away from results. Default output is 1440×900 with white regular 32 px sans-serif, padded rounded dark translucent boxes. The top-left disclosure also needs clear space. Keep captions to two short lines and natural spoken sentences. Reject overlapping/out-of-range cues and speech that exceeds its real window: shorten text rather than rush speech, retime real actions or invent claims. The helper emits H264/AAC/YUV420p faststart MP4 at 30 fps, fully decodes it and retains provenance/timing/QA reports; it does not certify truth, privacy or semantic caption placement. Inspect the complete `share.mp4`, caption boundaries, audio and final caveat before delivering it. Tooling has been exercised on Linux; macOS execution is not yet verified.

When Simon says "make it apparent," produce a short video that makes the current topic understandable. A finished implementation is not a prerequisite. Choose the mode from the viewer's question and available evidence:

- **Change demo — What works now?** Record the exact working build, meaningful action and visible result. Verify claims that pixels alone cannot prove.
- **Problem demonstration — What goes wrong?** Record a safely reproducible current failure or limitation. Unresolved behavior is a valid subject. Distinguish an observed failure from a suspected cause; do not invent a fix or successful ending.
- **Technical explainer — Why does this happen?** Use source-checked diagrams, illustrations, code excerpts or inspected evidence with timed explanation. Label conceptual examples, hypotheses and proposed changes inside the video. Created explanatory visuals are valid primary material, not failed proof.

For a generic request when implementation is unfinished or live capture is blocked, make the useful problem demonstration or technical explainer now. State the selected mode and limit briefly, then act. Do not stop at "nothing to show," require implementation first or ask permission merely because the explanation is not live footage. If the user explicitly asks to watch the real app, reproduce an exact bug, record with Apparent or prove an exact build, fulfill that evidence request or report its precise blocker. A conceptual supplement cannot satisfy an explicit live-proof request. Never present a hypothetical sequence as an observed reproduction.

The capture instructions below apply to scenes claiming live behavior, not as prerequisites for conceptual explanation. Keep raw evidence unchanged and private, inspect the entire final video and never commit generated media. A recording is not a substitute for tests. A screen-share of the coding session is not a demonstration of a finished result.

## Explain the cause, not the tool log

Before drafting, ask: **What does this viewer already know, and where must I pick them up?** Use the conversation and available context. Restore the immediate project context one local step before the unfamiliar mechanism; do not reteach familiar technical basics. If the evidence is thin, add one relevant bridge instead of asking the viewer to choose a level.

Open with one sentence stating the question, actual status and any required human action. Use **background → problem → mechanism/change → now → open → action**. These are story beats, not six mandatory slides. Give each beat's whole point first, then explain only the terms and causal links needed to follow it. Define unfamiliar things by a familiar class and relevant distinguishing feature. Use short active sentences and consistent names.

Show one concrete example moving through the mechanism: parts doing something to each other, not a component inventory or tool log. For example, a hypothetical timeline can save an earlier volume, lower it temporarily, accept a person's new choice, then show a naive restore overwriting that choice. Label this a **hypothetical example**, not a current app defect. Check any source before applying it to a product; show proposed protections separately from verified behavior.

For unfinished work, **now** explains the actual remaining gap, not a fictional success. State risks and untested claims before the ending. Finish with the real next action and owner, or "No human action is needed." Work still assigned to the agent stays with the agent.

Every mode needs timed, plain-spoken explanation, readable burned captions and editable timed text so the story works muted. Add synchronized generated voice by default unless the user requests silence or suitable narration already exists; disclose generated narration. Never play it through speakers during capture. Use only authorized speech services: do not send private text online or incur charges without permission. If narration is unavailable, report that production blocker; do not silently replace it with silence. An explicitly silent scenario can use captions alone, disclosed in the video and handoff.

### Produce a conceptual video without a capture

Use an existing local renderer/editor and encoder; no guest or desktop recorder is needed. Do not install a renderer, add a product dependency or use a paid service just for the video without authorization. Resolve Apparent's current visual assets from `assets/brand/`, site styles and supplied fonts. Use Paper `#F5F0E8`, Carbon `#020102`, restrained Signal `#7A58C7` and Lilac `#B7A5D8` if no newer palette exists. Prefer supplied Host Grotesk, otherwise a readable system sans. Keep labels large and beside their objects, use flat fills and clear arrow direction, and do not recolor real footage. Keep the established readable rounded dark subtitle style. Do not fabricate a logo or imply a conceptual diagram is an app screenshot.

Render a timed visual sequence, synthesize permitted narration when required, and encode a private playable MP4 at 30 fps with burned captions. Keep editable visual sources, narration, timed text and timing map outside git and public asset directories. A storyboard, script or HTML page alone is not delivery. Inspect every scene, transition and caption boundary at the intended player size and review the whole final export; verify timing, labels, privacy and playback. Check frame rate, duration and decoding with available media tools. No generated video can establish live app behavior. If rendering or delivery fails, report that exact production blocker rather than claiming there was no subject to explain.

## Point out the claim

1. Write one viewer question: “Can I now ___?”, “What goes wrong when ___?” or “Why does ___ cause ___?” Choose one checked scenario or labelled conceptual worked example. For a requested video, choose an explanatory visual sequence if live interaction adds nothing; do not silently downgrade to prose. For a verified bug fix, show the old failure if a safe capture exists, then the same task succeeding; never stage an old failure on the new build.
2. For live-proof scenes, identify the **exact build and environment**: commit or installed version, local/deployed URL where relevant, OS/device, and target window. Verify the app process or server actually serves that build. A local preview does not prove the released app, another platform, or the backend. Rehearse and confirm the result, not merely a successful click. For conceptual scenes, identify the source and separate observed facts, assumptions and proposed behavior instead.
3. Within a live-proof scene, capture starting state → meaningful action → visible result. Place it within the story above. Keep results readable and cut idle waiting, not evidence. Record the relevant assertion or test result separately if pixels alone cannot prove it.

Point at the claim, not every click. For a visual comparison, hold the same viewport and crop before and after. Use a sparse callout, outline or zoom only when the viewer might otherwise miss the changed region. Measure positions from the settled UI rather than guessing coordinates; recheck marks after scrolling or layout changes. Keep captions in empty space, and never cover the result they describe. Do not add arrows, measurements or captions merely to make the video look produced.

Make walkthrough videos at **30 fps by default**, including browser and generated review videos; use **60 fps only when the requester specifically asks for it**. A browser's live rendering rate is not the video's frame rate. Preserve real-time duration when encoding and verify the playable file's rate and duration before sharing it.

## Choose and prepare the capture surface

Prefer the installed **Apparent** on the same machine showing the result for native windows, screen motion, or iPhone. Follow the host repo's `AGENTS.md` and capture safety rules first. On Apparent's development Macs, read `docs/capture.md`; never wake, flash, or record Kit's real displays to make a demo. On another host, check its own permissions and capture capabilities; do not run a binary from a different machine and call it that host's proof.

Prepare a clean window before capture: hide notifications, unrelated windows and tabs, personal or customer records, tokens, credentials and auth prompts. Use synthetic **data** if necessary, but label a mocked or simulated **behavior** as such. Authenticate off camera. A crop or blur made after recording does not remove private material from the retained raw file; if a secret appears, discard that take and capture again. Keep the source bundle private, outside public/static asset paths and git.

For a browser-only change, an existing browser recorder may be more precise **if it is already available and the user has not asked specifically for Apparent footage**. Rehearse locators, then record the actual flow at a fixed viewport; wait for the semantic result, not just a timer or `networkidle`. Close the recording context to flush the video. Preserve the original and say “browser recording,” not “Apparent capture.” Do not install a recorder or add a project dependency just for a one-off proof without checking the project's workflow. A browser recording cannot establish native app or OS behavior; repo-local recording helpers from other projects are not portable commands.

## Record the proof with Apparent

For a normal product walkthrough, use the actual running GUI's recording shortcut. Verify its executable/build and `apparent capture state` first; do not start if another capture is selecting, starting, recording or processing. Start with the configured shortcut (`apparent shortcut record` on Linux), then read `apparent capture state` and retain the exact owned `capture_id`. Wait for `recording`, inspect the real recording pill, perform the action, and stop only that ID with `apparent capture stop <capture_id>`. Never toggle the shortcut to stop an unknown recording. Retain the exact completed bundle path and inspect/export that bundle; see `docs/capture.md` for the control protocol and permissions.

The GUI path supplies the normal recording pill and, where input permissions allow, clicks, marks and cursor telemetry for activity zoom. Inspect decoded source frames and the final crop: when the pill is promised, both must retain the **real pill** without a caption covering it. A selected-window source may omit surrounding controls. Reframe or record the appropriate display; never composite a fake pill.

Timed `record --duration` is a **separate recorder fallback**: it has no normal GUI recording pill and no click/Shift-ink telemetry. Use it only when the GUI route is unavailable or that separate recorder is deliberately being tested. Disclose those limits; a native timed clock take is not normal product-walkthrough proof. For that fallback, check `apparent record --help`, choose the known window and perform the action *while* recording:

```sh
apparent record --window 'Window title or app' --duration 15 --out /absolute/private/path/walkthrough.apparent
```

`--window` is a title/app substring; check the actual target in the bundle. For a supported cabled phone: `apparent record --iphone --duration 15 --out /absolute/private/path/walkthrough.apparent`. This command waits for the duration; it does not perform UI actions for you. Arrange the interaction separately or use the normal interactive recording shortcut and stop after the result. Start before the meaningful action and finish after the result settles. Use the returned exact bundle path, not `latest` when other captures could finish concurrently. If the CLI is unavailable, inspect the installed app path and permissions; don't pretend a transcript, synthetic `apparent demo` bundle, or a command that never recorded the action is a walkthrough.

## Inspect and render the live share copy

Read `summary.md` and `apparent.json`; open `contact-sheet.png` and the relevant `frames/*.png`. Play the **whole raw** `recording.mp4` or `recording.mov`, including first paint, transitions and ending. Check that it shows the correct build/window, all three beats, readable result, correct audio if narrated, no private frames, no error toast or misleading splice. The contact sheet indexes moments but cannot certify the intervening video. If an automatic crop hides context, inspect full frames/source before approving it. Verify the result independently with the relevant test, app state or log where appropriate; the recording proves only what it visibly shows.

For appearance changes, distinguish offscreen renders from a live window: render both schemes to inspect every state, then capture the exact running build opening while the OS is already dark and switching light→dark→light in the same process. Read back the OS appearance alongside screenshots; restore its original setting. A settled screenshot checks that frame, not every transition frame or a live permission grant.

If export works on this host, use `--fps 30` (or `--fps 60` on explicit request). Smooth activity zoom is on by default; keep it on for demos. Use `--no-zoom` only for an intentionally static share copy (for example, a motion-sensitive viewer or a fixed-frame comparison). Do not hand back the unprocessed original or a 60 fps original by accident:

```sh
apparent export /absolute/private/path/walkthrough.apparent --out /absolute/private/path/walkthrough.mp4 --fps 30
```

Do not invent a frame-rate flag for `apparent record`; set it during export or browser encoding without changing real-time duration. Check installed `export --help` before using the flags: older builds require `--zoom` to opt in, whereas current builds use `--no-zoom` to opt out. Compare the exported video to the source and watch it end-to-end too. If no camera motion appears, inspect the interaction telemetry rather than claiming the default flag guarantees motion. Browser-only recordings bypass Apparent's camera planner; prefer an Apparent capture of the browser when polished interaction zoom is required, and disclose an unpolished fallback rather than calling it equivalent. Ordinary Apparent export supplies crop/zoom and recorded audio, not timed captions. Run the required proof-video helper above for the narrated share copy; it does not add caption or speed flags to ordinary export. Never overlay a success claim before the result exists. Check the final file opens in the intended player; `ffprobe` verifies format, not truth or privacy. If source media or Apparent is unavailable, report the blocker and provide a screenshot or written trace, never a claimed video.

Missing source media blocks live proof, not a generic conceptual explanation. For an explicit live-proof request, disclose the missing media and give truthful partial evidence without calling it the requested video. Do not silently speed through a state transition.

## Hand it back

Embed or link the **verified playable file** in a channel the viewer can reach. Name the mode, exact build/environment for live proof (or source and hypothetical status for an explainer), and principal limit. State whether narration is generated or explicitly silent. Include a useful timestamp when it makes inspection faster. A path on an inaccessible machine is not delivery. Do not upload, post to a PR, send to a person, or publish raw or edited media without authorization for that destination; attachment visibility may inherit repository visibility. If production or delivery is blocked, name the exact failed step and best truthful partial evidence without calling the video done. Never claim a playable video exists until you have opened it.

## Failure traps

- A staged click after the result was already present proves nothing; reset to the starting state and capture the action.
- A smooth mock or browser replay is not evidence for a live native app; name the provenance visibly and in the handoff.
- Timed spoken-style explanation, matching captions and default TTS are delivery requirements; silent opt-out keeps captions. A measured crop or zoom helps readability, never conceals failures or invents causality.
- Automatic zoom is the default presentation for live footage. Burned explanatory captions are required; fast-forward is optional and must be disclosed. None permits concealing failures or inventing causality.
- A complete source bundle is not a shareable artifact until its entire media and delivery destination have been checked.