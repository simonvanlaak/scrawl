---
name: make-it-apparent
description: Use when a visible change needs a short video proof.
---

# Make It Apparent: show the change

Use this developer skill for a short proof video of a visible change. Keep the raw capture as evidence and make a separate share copy only when it improves clarity. Inspect the entire take before delivery; never commit generated media. A recording is not a substitute for tests, nor is a screen-share of the coding session a demonstration of the finished result.

## Point out the claim

1. Write one viewer question: “Can I now ___?” Choose a single real scenario that answers it. If motion or interaction adds nothing, use before/after screenshots or a written result instead of forcing a video. For a bug fix, show the old failure if a safe capture exists, then the same task succeeding; never stage an old failure on the new build.
2. Identify the **exact build and environment**: commit or installed version, local/deployed URL where relevant, OS/device, and target window. Verify the app process or server actually serves that build. A local preview does not prove the released app, another platform, or the backend. Run the flow once before recording and confirm the expected result, not merely a successful click.
3. Outline three beats: starting state → meaningful action → visible result. Include a human decision or exception only when it is central to the claim. Aim for a short watchable take, not a fixed duration: keep the result on screen long enough to read it and cut idle waiting, not evidence. Record the relevant assertion or test result separately if pixels alone cannot prove it.

Point at the claim, not every click. For a visual comparison, hold the same viewport and crop before and after. Use a sparse callout, outline or zoom only when the viewer might otherwise miss the changed region. Measure positions from the settled UI rather than guessing coordinates; recheck marks after scrolling or layout changes. Keep captions in empty space, and never cover the result they describe. Do not add arrows, measurements or captions merely to make the video look produced.

Make walkthrough videos at **30 fps by default**, including browser and generated review videos; use **60 fps only when the requester specifically asks for it**. A browser's live rendering rate is not the video's frame rate. Preserve real-time duration when encoding and verify the playable file's rate and duration before sharing it.

## Choose and prepare the capture surface

Prefer the installed **Apparent** on the same machine showing the result for native windows, screen motion, or iPhone. Follow the host repo's `AGENTS.md` and capture safety rules first. On Apparent's development Macs, read `docs/capture.md`; never wake, flash, or record Kit's real displays to make a demo. On another host, check its own permissions and capture capabilities; do not run a binary from a different machine and call it that host's proof.

Prepare a clean window before capture: hide notifications, unrelated windows and tabs, personal or customer records, tokens, credentials and auth prompts. Use synthetic **data** if necessary, but label a mocked or simulated **behavior** as such. Authenticate off camera. A crop or blur made after recording does not remove private material from the retained raw file; if a secret appears, discard that take and capture again. Keep the source bundle private, outside public/static asset paths and git.

For a browser-only change, an existing browser recorder may be more precise **if it is already available and the user has not asked specifically for Apparent footage**. Rehearse locators, then record the actual flow at a fixed viewport; wait for the semantic result, not just a timer or `networkidle`. Close the recording context to flush the video. Preserve the original and say “browser recording,” not “Apparent capture.” Do not install a recorder or add a project dependency just for a one-off proof without checking the project's workflow. A browser recording cannot establish native app or OS behavior; repo-local recording helpers from other projects are not portable commands.

## Record the proof with Apparent

Check `apparent record --help` on the **installed** app. For a fixed-duration desktop take, choose the known window and perform the action *while* recording:

```sh
apparent record --window 'Window title or app' --duration 15 --out /absolute/private/path/walkthrough.apparent
```

`--window` is a title/app substring; check the actual target in the bundle. For a supported cabled phone: `apparent record --iphone --duration 15 --out /absolute/private/path/walkthrough.apparent`. This command waits for the duration; it does not perform UI actions for you. Arrange the interaction separately or use the normal interactive recording shortcut and stop after the result. Start before the meaningful action and finish after the result settles. Use the returned exact bundle path, not `latest` when other captures could finish concurrently. If the CLI is unavailable, inspect the installed app path and permissions; don't pretend a transcript, synthetic `apparent demo` bundle, or a command that never recorded the action is a walkthrough.

## Inspect and render the share copy

Read `summary.md` and `apparent.json`; open `contact-sheet.png` and the relevant `frames/*.png`. Play the **whole raw** `recording.mp4` or `recording.mov`, including first paint, transitions and ending. Check that it shows the correct build/window, all three beats, readable result, correct audio if narrated, no private frames, no error toast or misleading splice. The contact sheet indexes moments but cannot certify the intervening video. If an automatic crop hides context, inspect full frames/source before approving it. Verify the result independently with the relevant test, app state or log where appropriate; the recording proves only what it visibly shows.

If export works on this host, use `--fps 30` (or `--fps 60` on explicit request) and add zoom only when it helps. Do not hand back a 60 fps original by accident:

```sh
apparent export /absolute/private/path/walkthrough.apparent --out /absolute/private/path/walkthrough.mp4 --fps 30
```

Do not invent a frame-rate flag for `apparent record`; set it during export or browser encoding without changing real-time duration. Add `--zoom` only when it makes the action easier to follow; compare the exported video to the source and watch it end-to-end too. Apparent export supplies crop/zoom and the recorded audio, **not** timed caption or speed cues. If a separate editor is available and captions genuinely help, burn short captions into a separate MP4 for players that ignore subtitle tracks; check every caption's timing against the visible state and keep the original. Never promise captioning or fast-forward as built-in Apparent flags. Do not silently speed through a state transition or overlay a success claim before the result exists. Check the final file opens in the intended player; use `ffprobe` to verify frame rate and duration, but not truth or privacy. A browser WebM may need an approved MP4 conversion for its destination; do not re-encode reflexively. If the source media is missing or Apparent is unavailable, say so and provide a screenshot or written trace, never a claimed video.

## Hand it back

Embed or link the **verified playable file** in a channel the viewer can reach. Give one sentence for the change, the exact build/environment, and the principal limit (“local Linux build,” “mocked backend,” “browser only,” or “does not prove persistence”). Include a before/after image or timestamp when that makes inspection faster. A path on an inaccessible machine is not delivery. Do not upload, post to a PR, send to a person, or publish the raw or edited footage without authorization for that destination; attachment visibility may inherit repository visibility. If recording or delivery is blocked, say precisely which step failed and provide the best truthful screenshot or written evidence instead. Never claim a playable video exists until you have opened it.

## Failure traps

- A staged click after the result was already present proves nothing; reset to the starting state and capture the action.
- A smooth mock or browser replay is not evidence for a live native app; name the provenance visibly and in the handoff.
- Automatic zoom, captions and fast-forward are optional explanation, not permission to conceal failures or invent causality.
- A complete source bundle is not a shareable artifact until its entire media and delivery destination have been checked.