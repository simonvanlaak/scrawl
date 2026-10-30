---
name: make-it-apparent
description: Use when told "make it apparent" or asked for video proof.
---

# Make It Apparent: show the change

When Simon says "make it apparent," treat it as a request to run this skill and show the result, not a request to edit the product name or copy. Use this developer skill for a short proof video of a visible change. Keep the raw capture as evidence and deliver a separate polished share copy by default. Inspect the entire take before delivery; never commit generated media. A recording is not a substitute for tests, nor is a screen-share of the coding session a demonstration of the finished result.

## Required spoken-style explanation and reusable CLI

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

The workflow reuses Apparent's crop and activity-camera export (`--crop full|auto|x,y,width,height`, zoom on, `--no-zoom` only when intentional). Default output is 1440×900 with white regular 32 px sans-serif, padded rounded dark translucent boxes. Captions sit at the bottom; the top-left disclosure also needs clear space. Keep captions to two short lines and natural spoken sentences. Reject overlapping/out-of-range cues and speech that exceeds its real window: shorten text rather than rush speech or invent claims. The CLI emits H264/AAC/YUV420p faststart MP4 at 30 fps, fully decodes it and retains source hashes and provenance/timing/QA reports; it does not certify truth, privacy or semantic caption placement. Edit cues and render to a new directory without re-recording. Inspect the complete `share.mp4`, caption boundaries, audio and final caveat before delivering it. Tooling has been exercised on Linux; macOS execution is not yet verified. The repository helper remains available for older or non-bundle movies, not as a substitute for the native bundle workflow.

Original time is the default; `--no-compression` explicitly preserves it. When idle waits harm the explanation, use `--compress-gaps --source-audio drop` inside the existing experiment (defaults: 4× gaps, 250 ms padding). Speech is measured first and never time-stretched. Readable captions and repeatable `--protect-range START,END` source-time spans stay real time; protect latency/animation/proof intervals before shortening. The speed indicator must remain visible on accelerated spans. Inspect the source/output map in provenance and mapped speech/SRT, not source timestamps alone. Caption-only mode preserves whole cue windows; both captions and voice off protects only explicit ranges. Retained source audio cannot be compressed. Reuse reviewed speech with `--speech-dir` only for exactly matching cues. See `docs/proof-video.md` for validation and rounding limits. Never mistake a sped-up clock or animation for real-time proof.


## Point out the claim

1. Write one viewer question: “Can I now ___?” Choose a single real scenario that answers it. If motion or interaction adds nothing, use before/after screenshots or a written result instead of forcing a video. For a bug fix, show the old failure if a safe capture exists, then the same task succeeding; never stage an old failure on the new build.
2. Identify the **exact build and environment**: commit or installed version, local/deployed URL where relevant, OS/device, and target window. Verify the app process or server actually serves that build. A local preview does not prove the released app, another platform, or the backend. Run the flow once before recording and confirm the expected result, not merely a successful click.
3. Order the explanation: background → observed problem → fix explanation → demonstrated new state → concrete human next action. If safe before footage is unavailable, say so; never stage or invent an old failure. End with a real action the human can take only when one exists, not a fabricated request to click or approve. Aim for a short watchable take, not a fixed duration: keep the result readable and shorten idle waiting, not evidence. Record the relevant assertion or test result separately if pixels alone cannot prove it.

Point at the claim, not every click. For a visual comparison, hold the same viewport and crop before and after. Use a sparse callout, outline or zoom only when the viewer might otherwise miss the changed region. Measure positions from the settled UI rather than guessing coordinates; recheck marks after scrolling or layout changes. Keep captions in empty space, and never cover the result they describe. Do not add arrows, measurements or captions merely to make the video look produced.

Make walkthrough videos at **30 fps by default**, including browser and generated review videos; use **60 fps only when the requester specifically asks for it**. A browser's live rendering rate is not the video's frame rate. Preserve real-time duration unless gap compression is explicitly selected, then verify the mapped duration, protected ranges and speed disclosure before sharing it.

## Choose and prepare the capture surface

Prefer the installed **Apparent** on the same machine showing the result for native windows, screen motion, or iPhone. Follow the host repo's `AGENTS.md` and capture safety rules first. On Apparent's development Macs, read `docs/capture.md`; never wake, flash, or record Kit's real displays to make a demo. On another host, check its own permissions and capture capabilities; do not run a binary from a different machine and call it that host's proof.

Prepare a clean window before capture: hide notifications, unrelated windows and tabs, personal or customer records, tokens, credentials and auth prompts. Use synthetic **data** if necessary, but label a mocked or simulated **behavior** as such. Authenticate off camera. A crop or blur made after recording does not remove private material from the retained raw file; if a secret appears, discard that take and capture again. Keep the source bundle private, outside public/static asset paths and git.

For a browser-only change, an existing browser recorder may be more precise **if it is already available and the user has not asked specifically for Apparent footage**. Rehearse locators, then record the actual flow at a fixed viewport; wait for the semantic result, not just a timer or `networkidle`. Close the recording context to flush the video. Preserve the original and say “browser recording,” not “Apparent capture.” Do not install a recorder or add a project dependency just for a one-off proof without checking the project's workflow. A browser recording cannot establish native app or OS behavior; repo-local recording helpers from other projects are not portable commands.

## Record the proof with Apparent

For polished desktop demos, use the app's interactive recording shortcut to start and finish: it collects the clicks, marks and cursor path that drive automatic zoom. Check the installed app's capabilities and input permissions first. Timed `record --duration` captures do **not** collect that interaction telemetry; do not choose them silently and expect camera motion. Use a timed take only when interaction-aware recording is unavailable or a static camera is intentional, and disclose that limit. For that fallback, check `apparent record --help`, choose the known window and perform the action *while* recording:

```sh
apparent record --window 'Window title or app' --duration 15 --out /absolute/private/path/walkthrough.apparent
```

`--window` is a title/app substring; check the actual target in the bundle. For a supported cabled phone: `apparent record --iphone --duration 15 --out /absolute/private/path/walkthrough.apparent`. This command waits for the duration; it does not perform UI actions for you. Arrange the interaction separately or use the normal interactive recording shortcut and stop after the result. Start before the meaningful action and finish after the result settles. Use the returned exact bundle path, not `latest` when other captures could finish concurrently. If the CLI is unavailable, inspect the installed app path and permissions; don't pretend a transcript, synthetic `apparent demo` bundle, or a command that never recorded the action is a walkthrough.

## Inspect and render the share copy

Read `summary.md` and `apparent.json`; open `contact-sheet.png` and the relevant `frames/*.png`. Play the **whole raw** `recording.mp4` or `recording.mov`, including first paint, transitions and ending. Check that it shows the correct build/window, all three beats, readable result, correct audio if narrated, no private frames, no error toast or misleading splice. The contact sheet indexes moments but cannot certify the intervening video. If an automatic crop hides context, inspect full frames/source before approving it. Verify the result independently with the relevant test, app state or log where appropriate; the recording proves only what it visibly shows.

For appearance changes, distinguish offscreen renders from a live window: render both schemes to inspect every state, then capture the exact running build opening while the OS is already dark and switching light→dark→light in the same process. Read back the OS appearance alongside screenshots; restore its original setting. A settled screenshot checks that frame, not every transition frame or a live permission grant.

If export works on this host, use `--fps 30` (or `--fps 60` on explicit request). Smooth activity zoom is on by default; keep it on for demos. Use `--no-zoom` only for an intentionally static share copy (for example, a motion-sensitive viewer or a fixed-frame comparison). Do not hand back the unprocessed original or a 60 fps original by accident:

```sh
apparent export /absolute/private/path/walkthrough.apparent --out /absolute/private/path/walkthrough.mp4 --fps 30
```

Do not invent a frame-rate flag for `apparent record`; the explicit walkthrough exports at 30 fps and keeps original time by default. Check installed `walkthrough --help` first. Ordinary `export` still supplies no explanation captions or gap compression; `walkthrough export` is the opt-in explanatory path. Compare both source and final movie end-to-end. No camera motion without telemetry is not a zoom-flag failure. Never overlay a success claim before the result exists or present browser-only footage as Apparent capture proof. If source media or the installed workflow is unavailable, report the exact blocker and provide a truthful screenshot or written trace, not a claimed walkthrough.

## Hand it back

Embed or link the **verified playable file** in a channel the viewer can reach. Give one sentence for the change, the exact build/environment, and the principal limit (“local Linux build,” “mocked backend,” “browser only,” or “does not prove persistence”). Include a before/after image or timestamp when that makes inspection faster. A path on an inaccessible machine is not delivery. Do not upload, post to a PR, send to a person, or publish the raw or edited footage without authorization for that destination; attachment visibility may inherit repository visibility. If recording or delivery is blocked, say precisely which step failed and provide the best truthful screenshot or written evidence instead. Never claim a playable video exists until you have opened it.

## Failure traps

- A staged click after the result was already present proves nothing; reset to the starting state and capture the action.
- A smooth mock or browser replay is not evidence for a live native app; name the provenance visibly and in the handoff.
- Timed spoken-style explanation, matching captions and default TTS are delivery requirements; silent opt-out keeps captions. A measured crop or zoom helps readability, never conceals failures or invents causality.
- A complete source bundle is not a shareable artifact until its entire media and delivery destination have been checked.