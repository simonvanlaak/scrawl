---
name: make-it-apparent
description: Use when told "make it apparent" or asked for video proof.
---

# Make It Apparent: make the situation understandable

Keep raw captures unchanged and private and deliver a separate polished share copy. Inspect the entire take before delivery; never commit generated media.

## Scope: match the evidence to the viewer's question

This skill owns mode selection, real screen recording where needed, source inspection, timed narration/subtitles and playable delivery. Follow the causal story below, including unresolved problems and conceptual explanations. State material limits before the ending; the story must be understandable without the chat.

**When explanatory visuals are needed, use [apparent-visual-explanations](../apparent-visual-explanations/SKILL.md).** It owns slides, diagrams, illustrations, title/end cards and callouts, Apparent brand and visual QA. For live demonstrations, stay with real footage unless visuals help; do not add slides by default. For conceptual explainers, created visuals can be primary material. Never restyle recorded UI, replace explicitly requested live proof with slides, or redesign the existing rounded dark captions. A scripted slide window recorded through Apparent is still a slideshow, not app-behavior proof. A generic request can proceed as an honestly labelled explainer when capture is blocked; an explicit live-proof request remains blocked and needs that exact limit reported.

## Required spoken-style explanation and reusable CLI

Choose change demo, problem demonstration or technical explainer as described below. The bundle commands in this section apply only to captured footage; conceptual explanation does not require a capture. Rendering is an explicit off-by-default `agentWalkthrough` experiment, not ordinary export behavior.

Every delivered proof video needs timed natural sentences explaining the action, evidence and result, with matching burned subtitles and synchronized TTS by default. Retain editable cues, SRT and script. This is postproduction explanation, not a transcription or replacement of recorded evidence. Disclose synthetic narration; never play it through speakers during capture. Simon can request a silent captioned copy instead. Suitable live narration need not be duplicated.

Use the installed `apparent walkthrough` commands, not a new per-video script. Read `docs/proof-video.md` for dependency setup, cue format and privacy. First make a real Apparent capture. Then inspect its saved evidence and write a private `cues.json` array of `[start_seconds, end_seconds, text]` against the movie clock:

```sh
apparent walkthrough inspect /absolute/private/walkthrough.apparent --template /absolute/private/cues.json
# Inspect the whole movie and images, then fill the blank template.
apparent walkthrough export /absolute/private/walkthrough.apparent \
  --experiment agentWalkthrough \
  --cues /absolute/private/cues.json \
  --out-dir /absolute/private/proof-take-1 \
  --source-audio drop --allow-online-tts \
  --font /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf
```

The output directory must be new and outside the source bundle. The installed binary embeds the renderer; it needs Python 3.11+, Pillow==11.3.0, edge-tts==7.2.3 (voice), FFmpeg/ffprobe and a regular TTF prepared separately (`--python` and `--font` select paths). It never installs packages. The font example is Linux-specific. `--allow-online-tts` consents to sending cue text only to Microsoft Edge TTS; never include secrets. Default voice is en-GB-RyanNeural at +10%, not Nova. `--no-voice` and `--no-captions` are independent; `--silent` guarantees no audio with `--source-audio drop`. Explicit `--source-audio keep` mixes reviewed original audio; default practice is `drop` to avoid leaking source conversation. Neither choice changes the original. Do not publish media without destination approval.

`inspect` media readiness is not atomic capture readiness or a finalizer receipt. Confirm completion through the owned GUI capture ID and exact completed bundle, then inspect the saved media; never use a successful probe as permission to stop or claim another capture.

The workflow reuses Apparent's crop and activity-camera export (`--crop full|auto|x,y,width,height`, zoom on, `--no-zoom` only when intentional). Default output is 1440×900 with white regular 32 px sans-serif, padded rounded dark translucent boxes. Captions sit at the bottom; the top-left disclosure also needs clear space. Keep captions to two short lines and natural spoken sentences. Reject overlapping/out-of-range cues and speech that exceeds its real window: shorten text rather than rush speech or invent claims. The CLI emits H264/AAC/YUV420p faststart MP4 at 30 fps, fully decodes it and retains source hashes and provenance/timing/QA reports; it does not certify truth, privacy or semantic caption placement. Edit cues and render to a new directory without re-recording. Inspect the complete `share.mp4`, caption boundaries, audio and final caveat before delivering it. Tooling has been exercised on Linux; macOS execution is not yet verified. The repository helper remains available for older or non-bundle movies, not as a substitute for the native bundle workflow.

Original time is the default; `--no-compression` explicitly preserves it. When idle waits harm the explanation, use `--compress-gaps --source-audio drop` inside the existing experiment (defaults: 4× gaps, 250 ms padding). Speech is measured first and never time-stretched. Readable captions and repeatable `--protect-range START,END` source-time spans stay real time; protect latency/animation/proof intervals before shortening. The speed indicator must remain visible on accelerated spans. Inspect the source/output map in provenance and mapped speech/SRT, not source timestamps alone. Caption-only mode preserves whole cue windows; both captions and voice off protects only explicit ranges. Retained source audio cannot be compressed. Reuse reviewed speech with `--speech-dir` only for exactly matching cues. See `docs/proof-video.md` for validation and rounding limits. Never mistake a sped-up clock or animation for real-time proof.


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

Make walkthrough videos at **30 fps by default**, including browser and generated review videos; use **60 fps only when the requester specifically asks for it**. A browser's live rendering rate is not the video's frame rate. Preserve real-time duration unless gap compression is explicitly selected, then verify the mapped duration, protected ranges and speed disclosure before sharing it.

## Choose and prepare the capture surface

Prefer the installed **Apparent** on the same machine showing the result for native windows, screen motion, or iPhone. Follow the host repo's `AGENTS.md` and capture safety rules first. On Apparent's development Macs, read `docs/capture.md`; never wake, flash, or record Kit's real displays to make a demo. On another host, check its own permissions and capture capabilities; do not run a binary from a different machine and call it that host's proof.

Prepare a clean window before capture: hide notifications, unrelated windows and tabs, personal or customer records, tokens, credentials and auth prompts. Use synthetic **data** if necessary, but label a mocked or simulated **behavior** as such. Authenticate off camera. A crop or blur made after recording does not remove private material from the retained raw file; if a secret appears, discard that take and capture again. Keep the source bundle private, outside public/static asset paths and git.

For a browser-only change, an existing browser recorder may be more precise **if it is already available and the user has not asked specifically for Apparent footage**. Rehearse locators, then record the actual flow at a fixed viewport; wait for the semantic result, not just a timer or `networkidle`. Close the recording context to flush the video. Preserve the original and say “browser recording,” not “Apparent capture.” Do not install a recorder or add a project dependency just for a one-off proof without checking the project's workflow. A browser recording cannot establish native app or OS behavior; repo-local recording helpers from other projects are not portable commands.

## Record the proof with Apparent

For polished desktop demos, use `apparent capture start --request-id <id> --wait --timeout 30` through the already-running GUI's normal interaction-aware path: it collects the clicks, marks and cursor path that drive automatic zoom. Generate and privately retain a 32-hex-character request ID before sending. Retain the reply's top-level `capture_id`, including on failure or timeout, and require `status == "recording"` before acting: a real frame has arrived. Occupied sessions are refused, not toggled; pending portal consent is not recording. Check installed help and input permissions first; older builds lack these controls, and the CLI does not launch the app or bypass consent.

Finish agent-authored videos with `apparent capture stop <owned_id> --wait --timeout 60`, rather than hunting for Finish with OCR; require the visible button only when that button is the claim being tested. Success requires `status == "saved"` and that capture's exact `bundle_path`; a non-waited `stopping` reply is only acceptance. After timeout or disconnect, use `apparent capture inspect <id>` or retry the same start request ID or owned-ID Stop, not a shortcut toggle, `latest` or a directory scan. An admitted capture remains intact after a readiness timeout; explicitly cancel only your own ID if discarding it. Rejected request IDs remain rejected, and receipts last only for this GUI process, not an app restart. Follow [the agent video workflow](../../docs/agent-video-workflow.md) for bounded recovery and saved-bundle checks. Shared tests do not prove native Mac capture; verify the exact candidate on each claimed platform before release.

The GUI path supplies the normal recording pill and, where input permissions allow, clicks, marks and cursor telemetry for activity zoom. Inspect decoded source frames and the final crop: when the pill is promised, both must retain the **real pill** without a caption covering it. A selected-window source may omit surrounding controls. Reframe or record the appropriate display; never composite a fake pill.

Timed `record --duration` is a **separate recorder fallback**: it has no normal GUI recording pill and no click/Shift-ink telemetry. Use it only when the GUI route is unavailable or that separate recorder is deliberately being tested. Disclose those limits; a native timed clock take is not normal product-walkthrough proof. For that fallback, check `apparent record --help`, choose the known window and perform the action *while* recording:

```sh
apparent record --window 'Window title or app' --duration 15 --out /absolute/private/path/walkthrough.apparent
```

`--window` is a title/app substring; check the actual target in the bundle. For a supported cabled phone: `apparent record --iphone --duration 15 --out /absolute/private/path/walkthrough.apparent`. This command waits for the duration; it does not perform UI actions for you. Arrange the interaction separately; the GUI `capture` controls do not control a timed recorder. Start before the meaningful action and finish after the result settles. Use the returned exact bundle path, not `latest` when other captures could finish concurrently. If the CLI is unavailable, inspect the installed app path and permissions; don't pretend a transcript, synthetic `apparent demo` bundle, or a command that never recorded the action is a walkthrough.

## Inspect and render the live share copy

Read `summary.md` and `apparent.json`; open `contact-sheet.png` and the relevant `frames/*.png`. Play the **whole raw** `recording.mp4` or `recording.mov`, including first paint, transitions and ending. Check that it shows the correct build/window, all three beats, readable result, correct audio if narrated, no private frames, no error toast or misleading splice. The contact sheet indexes moments but cannot certify the intervening video. If an automatic crop hides context, inspect full frames/source before approving it. Verify the result independently with the relevant test, app state or log where appropriate; the recording proves only what it visibly shows.

For appearance changes, distinguish offscreen renders from a live window: render both schemes to inspect every state, then capture the exact running build opening while the OS is already dark and switching light→dark→light in the same process. Read back the OS appearance alongside screenshots; restore its original setting. A settled screenshot checks that frame, not every transition frame or a live permission grant.

If export works on this host, use `--fps 30` (or `--fps 60` on explicit request). Smooth activity zoom is on by default; keep it on for demos. Use `--no-zoom` only for an intentionally static share copy (for example, a motion-sensitive viewer or a fixed-frame comparison). Do not hand back the unprocessed original or a 60 fps original by accident:

```sh
apparent export /absolute/private/path/walkthrough.apparent --out /absolute/private/path/walkthrough.mp4 --fps 30
```

Do not invent a frame-rate flag for `apparent record`; the explicit walkthrough exports at 30 fps and keeps original time by default. Check installed `walkthrough --help` first. Ordinary `export` still supplies no explanation captions or gap compression; `walkthrough export` is the opt-in explanatory path. Compare both source and final movie end-to-end. No camera motion without telemetry is not a zoom-flag failure. Never overlay a success claim before the result exists or present browser-only footage as Apparent capture proof. If source media or the installed workflow is unavailable, report the exact blocker and provide a truthful screenshot or written trace, not a claimed walkthrough.

Missing source media blocks live proof, not a generic conceptual explanation. For an explicit live-proof request, disclose the missing media and give truthful partial evidence without calling it the requested video. Do not silently speed through a state transition.

## Hand it back

Embed or link the **verified playable file** in a channel the viewer can reach. Name the mode, exact build/environment for live proof (or source and hypothetical status for an explainer), and principal limit. State whether narration is generated or explicitly silent. Include a useful timestamp when it makes inspection faster. A path on an inaccessible machine is not delivery. Do not upload, post to a PR, send to a person, or publish raw or edited media without authorization for that destination; attachment visibility may inherit repository visibility. If production or delivery is blocked, name the exact failed step and best truthful partial evidence without calling the video done. Never claim a playable video exists until you have opened it.

## Agent recording trial lessons

- Read `capture state` for context, not ownership; a socket pathname is not readiness. Start with a retained request ID and require the request's own `recording` receipt before acting. Atomic admission refuses occupied sessions. Never adopt or stop a capture that was already active.
- Derive gestures from settled window bounds and verify the visible mark before continuing. A successful input command proves only that input was accepted.
- Use owned-ID `stop --wait` and its exact saved `bundle_path`, not a global state check. The command checks identity at the action; retrying it cannot stop a newer capture. If Finish already ended the take, inspect that ID or repeat its waited Stop to recover the same result. Unknown, cancelled, failed, wrong-ID and timeout replies are not saved success.
- Tie QA grids and cue-boundary frames to the final movie's hash; regenerate them after every edit. Inspect the actual deliverable before diagnosing a defect from an old grid.
- Keep a private attempt ledger with build/runner identity, elapsed times, retries and evidence paths. Separate harness failures from product defects. When comparing workflow-only retests, keep app and runner bytes fixed; disclose a different stop route and measure readable holds separately. Do not call different routes a Finish-button benchmark or infer total-time improvements from unequal setup.

## Failure traps

- A staged click after the result was already present proves nothing; reset to the starting state and capture the action.
- A smooth mock or browser replay is not evidence for a live native app; name the provenance visibly and in the handoff.
- Timed spoken-style explanation, matching captions and default TTS are delivery requirements; silent opt-out keeps captions. A measured crop or zoom helps readability, never conceals failures or invents causality.
- Automatic zoom is the default presentation for live footage. Burned explanatory captions are required; fast-forward is optional and must be disclosed. None permits concealing failures or inventing causality.
- A complete source bundle is not a shareable artifact until its entire media and delivery destination have been checked.