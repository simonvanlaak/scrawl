# Agent video workflow

Use this with [Make It Apparent](../skills/make-it-apparent/SKILL.md) when an agent needs real desktop proof. It describes the existing GUI capture controls, not a new recorder or a portable VM harness. Read the host's capture safety rules first. Keep recordings, private paths, logs and QA outside git and public assets.

## Prepare and claim only your own take

1. Record the exact build, OS, running executable and installed CLI capabilities in a private evidence directory. Check installed help before choosing a command. Experimental walkthrough tooling in another branch is not a shipped prerequisite.
2. Choose one visible claim and rehearse it on a safe capture surface. Hide unrelated material before recording; post-production cropping cannot remove secrets from the retained raw movie.
3. Set a startup deadline and a separate finalization deadline before running the driver. Poll with a bounded interval, recording the last response and elapsed time. On timeout, stop the driver and report the phase; do not toggle recording or cancel an unknown capture to recover.
4. Require a successful `apparent capture state` response with `state.phase == "idle"` and `state.capture_id == null` before starting. A socket pathname is not readiness. A completed prior take is not an idle preflight; let the GUI return to idle or report the blocked precondition without issuing a shortcut that could affect someone else's work.
5. Start using the configured GUI shortcut (`apparent shortcut record` on Linux). Wait for a successful state response with `state.phase == "recording"` and a non-null ID, then retain that ID as the take you initiated. `starting` is not recording: a portal picker may still need selection. Keep JSON stdout separate from stderr and reject failed, malformed or incomplete responses.

The state ID is not proof of ownership. Use an isolated app/session for unattended trials. On a shared GUI, idle followed by recording cannot distinguish your start from someone else's simultaneous start; if ownership is uncertain, do not stop or cancel it.

## Drive a settled UI

Wait for the target window's bounds to remain unchanged across several observations before deriving gesture coordinates. Measure against the actual window and display, not a fixed desktop size. Recheck after layout changes, scrolling or reopening. Confirm that a drawn mark or clicked result is visible before moving to the next beat. Input acceptance alone does not establish that the intended control was hit.

Keep the result visible for a planned readable hold. For persistence proof, show the original mark, close/reopen, and verify the same mark. Keep an independent state or artifact assertion when the movie alone cannot establish the result.

## Finish once, then verify

For ordinary agent-authored videos, use `apparent capture stop <owned_id>` instead of whole-screen OCR for Finish. A visible Finish click is necessary only when that control is the subject of the test.

Immediately before stopping, require a fresh successful state read with phase `recording` and the same owned ID. The stop command checks the ID at the action, so a newer capture cannot be stopped by a stale ID. Do not use a shortcut as an automatic retry: a toggle could start or stop a different take.

A successful `outcome == "stopping"` reply means finalization began, **not** that a valid bundle exists. Poll read-only until both conditions hold:

- State is `completed` or `idle` with a null active ID.
- The exact bundle from this take has a valid manifest, summary, expected frames and a decodable nonempty recording. Inspect the saved content against the claim.

Use an exact returned path when available. Otherwise, use a private per-trial captures directory and compare its before/after contents; require exactly one new valid bundle. Do not guess `latest` in a shared directory or accept an unrelated old bundle as success.

If Finish already ended the take or a stop returns `already_ended`, do not send another stop. Check the owned take's saved bundle and ended state. A wrong ID, malformed reply, missing bundle or finalization timeout is not success; retain the diagnostic evidence without acting on another take.

A self-recorded movie ends at the actual stop. Verify saved-state aftermath separately. Do not append a completion still and claim the source movie showed it.

## Inspect the actual deliverable

Read `summary.md` and `apparent.json`, inspect `contact-sheet.png` and relevant full frames, and inspect the complete source movie. Render the share copy separately. Probe rate/duration and inspect the entire final movie, including every caption boundary, readable holds and the ending disclosure.

Record the final movie's SHA-256 alongside its QA grids and cue-boundary frames. Regenerate QA after every edit and require its recorded hash to match the delivered file. If an old grid contradicts the movie, inspect decoded frames from the final file before diagnosing a defect. Hash agreement identifies the bytes; it does not certify the claims or privacy.

Disclose CLI stop rather than a visible Finish click, injected input where used, exact build/platform, and audio limits. A caption-only take is not microphone or generated-speech proof. Verify playable delivery in the intended conversation; a remote filesystem path alone is not delivery. Public footage needs separate authorization.

## Compare trials without inventing a benchmark

Keep a private ledger of attempted action, observed result, elapsed time, retry and evidence path. Separate setup/readiness failures, missed gestures and false-red controller exits from confirmed product defects. For workflow-only retests, retain app and runner hashes and keep those bytes fixed.

Measure meaningful-action-to-stop request, planned readable hold, stop acceptance and saved/ended verification separately. Replacing OCR with CLI control is a deliberate workflow change, not a like-for-like Finish-button benchmark. Different cache state, reused speech, capture lengths or environment setup prevent a controlled total-time claim.

These practices came from a private Linux whiteboard persistence trial on Apparent 0.3.13, commit `e80e2141c989625c9080f4c9fed974e7715c0148`. The final take was caption-only; it did not test Mac capture or microphone audio. No app defect was confirmed. This guide adds no runtime behavior, narration dependency, experiment or platform-parity claim.
