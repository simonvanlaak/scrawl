# Agent walkthroughs: record first, explain second

`apparent walkthrough` connects the real recorder, evidence inspection and a separate polished share copy. Ordinary captures and `apparent export` defaults are unchanged. Rendering is an off-by-default `agentWalkthrough` experiment: opt in per export with `--experiment agentWalkthrough`, or set `"experiments": {"agentWalkthrough": true}` in `~/.config/apparent/config.json` (read at invocation, no restart).

## Two passes

```sh
apparent walkthrough record --duration 20 --out /private/take.apparent \
  --note 'A factual task note, not a test verdict'
apparent walkthrough inspect /private/take.apparent --template /private/cues.json
# Inspect the entire movie, summary, contact sheet and frames. Edit cues.json.
apparent walkthrough export /private/take.apparent \
  --experiment agentWalkthrough --cues /private/cues.json \
  --out-dir /private/share-1 --source-audio drop --allow-online-tts
```

All three commands print machine-readable JSON on stdout; recorder progress goes to stderr. `record` delegates to the existing timed recorder, supports its unique `--window` selector, refuses an existing bundle, and waits through finalization. It records screen and microphone but **no click/Shift-ink telemetry**; a changing UI can still be visible in the movie. It does not perform your task for you. Arrange actions concurrently with recording. No wall clock is used for cues.

For interaction-driven camera motion, use the running app's normal recording shortcut instead, retain its `capture_id` from `apparent capture state`, and finish only your capture with `apparent capture stop <capture_id>`. Do not toggle a shortcut to stop an unknown recording. See [capture control](capture.md#agent-control-of-the-running-gui). Then use `walkthrough inspect` on the exact completed bundle. The workflow does not introduce another recording engine or change input permissions.

`inspect` returns `ready` only after loading a manifest and probing a real movie. A directory without a manifest returns `finalizing_or_incomplete` with instructions; this is not a guarantee an abandoned capture will finish. Invalid manifests or missing media fail. JSON includes absolute evidence paths, raw events/marks, telemetry limits, frame paths and movie times. Bundle milliseconds become movie seconds by `(t - videoOffsetMs) / 1000`; negative values precede the movie. The editable template is a JSON array of `[start_seconds, end_seconds, text]`, initially `[[0, duration, ""]]`. Empty text is intentionally rejected: the agent must supply meaning after inspection. Existing templates are never overwritten.

```json
[
  [0, 3.5, "I'm opening the saved recording."],
  [4, 8, "The result is visible now.\nThis checks the local build, not a release."]
]
```

Write natural short sentences tied to actual visible evidence. Notes are factual context, not inferred pass/fail claims. The app never calls a model to invent explanation. Cues must be finite, ordered, non-overlapping, within movie duration, nonempty and at most two lines. Gaps are caption-free and silent unless reviewed source audio is kept. Synthesized speech must fit its window with 100 ms spare. If it does not fit, shorten the text or choose a longer evidenced window: never rush speech or retime real actions. Default voice is **en-GB-RyanNeural at +10%**, not Nova. New text needs a new render directory, not a new recording.

## Privacy and independent switches

Captions and TTS are ON in this explicitly selected workflow only. `--no-captions` omits burned explanation while retaining voice; `--no-voice` omits TTS while retaining captions and the chosen source-audio policy. Both can be used together. `--silent` implies no voice and requires `--source-audio drop`, so it produces no audio stream. A visible provenance badge remains even without captions.

`--source-audio keep|drop` is required. `keep` mixes reviewed movie/microphone audio at its original time, without ducking; loud input can mask narration or clip. `drop` removes it from **share.mp4**, not from the original or private working files. Always review microphone privacy before choosing `keep`.

`--allow-online-tts` explicitly permits sending **cue text only** to Microsoft's Edge TTS service. It does not upload footage or source audio and does not authorize publishing the result. Do not put secrets or private personal text in cues. Without consent, choose `--no-voice` or `--silent`; there is no silent network fallback. No credentials are read and no speaker playback occurs.

## Installed dependencies and rendering

The Python renderer is embedded in the Apparent binary at build time. An installed/staged executable works without a checkout or `scripts/` directory. It invokes the same executable's ordinary `export` command for shared crop, ink and camera planning, then calls the **same** renderer as `scripts/proof-video.py`; there is no duplicate pipeline.

External prerequisites, on both macOS and Linux:

- Python 3.11+ with **Pillow==11.3.0** and, for voice, **edge-tts==7.2.3**. Prepare an environment separately and use `--python /absolute/venv/bin/python` if needed. The CLI never downloads or installs packages, runs `uv`, or installs system tools. The repository helper's PEP 723 metadata pins those same versions for explicit `uv run` use.
- `ffmpeg`/`ffprobe` on PATH, with libx264 and AAC support.
- A regular sans-serif TTF. Linux DejaVu Sans and macOS supplemental Arial locations are checked; use `--font /absolute/path/font.ttf` if absent. Font appearance on macOS is not yet verified.
- The ordinary installed app/export prerequisites, including the signed Mac helper on macOS.

`--crop full|auto|x,y,w,h` uses the existing bundle exporter (rectangle coordinates are source pixels). Activity camera zoom is on by default; `--no-zoom` fixes the view. With no interaction telemetry it stays still. Output is H264/YUV420p, 1440×900 at 30 fps, AAC when audio is present, faststart. Other aspect ratios are letterboxed. Real duration and chronological action order remain unchanged.

Caption style: regular 32 px white text, 15 px padding, radius 8, dark alpha 220 box, bottom safe area. Keep explanatory lines short. A disclosure occupies the upper left. Inspect actual output for semantic occlusion: bounds validation cannot know which result your caption hides. The standalone helper additionally has measured `--crop` and `--caption-box` controls for existing non-bundle movies; that is not fresh Apparent capture proof.

## Preservation, reproducibility and delivery

Output must be a NEW private directory outside the bundle. Originals are read-only; all source bundle files are SHA-256 pinned before export and checked again before publishing. Symlinked bundle files are refused. A failure leaves diagnostics in the private output directory, not a completed share copy. `share.mp4` is exclusively published only after full decode, duration/codec/audio QA and source-hash comparison. Existing paths are never replaced. This does not defend against a hostile same-user process swapping private directories.

JSON points to `share.mp4`, `provenance.json`, `timing-report.json`, `qa.json` and the copied `cues.json`. Sidecars also retain editable SRT, script, caption images, generated speech and **camera.mp4**, an intermediate that can contain source audio even when the final share drops it. Keep the entire work directory private. Provenance names original paths/hashes and chosen policies; do not publish these private sidecars unreviewed.

Inspect the whole original and final movie, every caption boundary/gap, audio synchronization, result readability and private content before delivery. Full decode is not a truth or privacy certificate. Keep test results separate. Never commit media, host it or upload it to a PR without destination approval. Linux execution does not verify Mac capture, Swift rendering, or packaged fonts; macOS remains a separate platform gate.

## Tests

```sh
python3 -m unittest discover -s scripts/tests -p test_proof_video.py
APPARENT_TEST_BINARY=/absolute/staged/apparent \
  python3 -m unittest discover -s scripts/tests -p test_walkthrough_cli.py
cargo test -j 3 --offline -- --test-threads=2
cargo build -j 3 --offline
```

Helper tests use synthetic footage and fixture speech rather than network TTS. CLI tests exercise installed dispatch, offset mapping, missing evidence/template instructions, actual bundle export, independent policy and source preservation. Real TTS and a fresh Apparent GUI/portal or supported window capture still require private end-to-end proof.
