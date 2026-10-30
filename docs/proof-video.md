# Agent proof-video share copies

This is repository agent tooling, **not** an `apparent` app subcommand or an automatic capture change. It never adds narration to a bundle, microphone WAV, contact sheet or source movie. First record and inspect real evidence with `skills/make-it-apparent/SKILL.md`. If useful, run the installed app's `apparent export` for its activity camera first; this helper accepts that existing movie too. No Rust build or app restart is needed.

For a normal product walkthrough, start the verified running GUI through its configured recording shortcut, retain the exact owned `capture_id` from `apparent capture state`, wait for `recording`, then finish only that capture with `apparent capture stop <capture_id>`. Inspect the exact completed bundle before rendering. Check the actual source and final crop retain the real recording pill whenever it is promised; never add a simulated pill. Timed `record --duration` is a separate recorder fallback with no normal GUI pill or click/Shift-ink telemetry, not equivalent product proof. Disclose it as such. See `docs/capture.md` for safe capture ownership and permissions.

## Setup and run

Requires Python 3.11+, `uv`, FFmpeg/ffprobe with libx264 and AAC, and a regular sans-serif TTF. `uv` installs the helper's declared Pillow and edge-tts dependencies in an isolated environment. Linux is exercised; macOS commands and fonts are not yet exercised. No Mac allocation is required for Linux postproduction. Select an actual installed font path on your host (the Linux example below is not a Mac font path).

Write a private UTF-8 JSON array of `[start_seconds, end_seconds, spoken_text]` against the **input movie clock**, for example:

```json
[
  [0, 3.5, "I'm opening the saved recording."],
  [4, 8, "The result is visible now.\nThis checks the local build, not a release."]
]
```

Use natural short sentences describing the real action, evidence and result. Write the build/platform and caveat into the script when they matter. The tool does not invent claims, transcribe source speech, decide that a fix works, or certify footage privacy. Cues must be ordered, non-overlapping, finite, inside the input duration and at most two lines. Gaps are silent and caption-free. The same wording goes to TTS and captions. Long lines fail rather than shrink. Synthesized speech must fit its window with 100 ms spare; it is **never** time-stretched. Shorten the explanation if it fails; do not speed up footage or shift claims ahead of visible results. Default synthesis uses en-GB-RyanNeural at +10%, matching the reference voice; this is not Nova.

From the Apparent checkout, with an existing private parent directory:

```sh
uv run scripts/proof-video.py /absolute/private/walkthrough.mp4 \
  --cues /absolute/private/cues.json \
  --out-dir /absolute/private/proof-take-1 \
  --source-audio drop --allow-online-tts \
  --font /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf
```

`--out-dir` must not exist. It is created with mode 0700; all artifacts stay there, including diagnostic files after a failed take. Never point it at public/static assets, a capture bundle, or git. Rerun into a new directory after editing cues. The original is opened read-only and SHA-256 checked before publication. `share.mp4` appears only after codec/duration/audio checks and a complete decode. Existing destinations are refused, never overwritten. The helper does not defend against a hostile same-user process mutating its private directory.

### Framing and safe areas

Output is 1440×900, H264/YUV420p at 30 fps, AAC when narrated, with faststart. Real action timing and duration remain unchanged. Other aspect ratios are letterboxed, not stretched. A measured static close-up is available with `--crop x,y,width,height` in **source pixels**; it must fit entirely inside the source. For the approved 1440×900 meter footage the close-up is `--crop 350,0,760,475`. This is not an automatic activity camera. Check the full source before choosing a crop; don't hide context or failure.

Captions use white regular 32 px text, 15 px padding, radius 8 and dark alpha 220 boxes. `--caption-box 40,730,1360,140` is the default allowed region in **output pixels**. Move it to a measured empty region so neither controls nor results are covered. The helper validates bounds and text fit, not semantic occlusion. A small disclosure occupies the top-left output area (12,12 to 620,48); keep that area clear too. Inspect every caption boundary, including gaps and final caveat; FFmpeg image timestamps are frame-quantized. Keep sentences readable at the intended player size.

### Audio privacy and silence

`--source-audio` is required. Choose `drop` for a clean synthetic explanation without exposing source conversation; the raw track remains untouched in the source. Choose `keep` only after reviewing it for privacy and relevance: it mixes original audio at its recorded timing with synthetic speech, without ducking, and can mask narration or clip if loud. Inspect the mix before delivery. This is postproduction, **not evidentiary source speech**. A persistent visible badge and MP4 metadata disclose synthetic voice-over and the source-audio choice.

`--allow-online-tts` explicitly permits sending **cue text only** to Microsoft's Edge TTS service; audio/video is not uploaded. Don't put secrets or private personal text in cues. This approval does not authorize publishing footage. The service needs network access and has no availability guarantee; a service error must not be replaced with invented narration. No credentials are required or fetched. `--voice` and `--rate` select a legitimate available Edge voice/rate, not another provider's identity. No speaker playback occurs.

For Simon's silent opt-out, replace `--allow-online-tts` with `--silent` and retain `--source-audio drop`. This makes captioned H264 MP4 with **no audio stream** and no TTS request. Suitable existing live narration can use the ordinary app export instead; do not duplicate narration, and still supply explanatory subtitles through a separately reviewed workflow if needed.

## Inspect and deliver

Retained sidecars: `cues.json`, `spoken-script.txt`, `explanation.srt`, `provenance.json` (source hash, policy, voice and framing), `timing-report.json` (measured speech intervals), `qa.json` (ffprobe streams), caption PNGs and generated speech. These are editable/reproducible inputs and private diagnostics, not a public bundle. Do not commit them. Verify the complete raw take and share copy visually, full-decode the output (the CLI does this), check speech/caption synchronization and safe areas, and independently verify the claims. Never call an exported clip a passing test, release, Mac GUI proof of this helper, or a fresh capture. Deliver the playable file in the current conversation, naming source build/platform and limitations. Do not publish it to a PR or another destination without approval.

## Tests

```sh
uv run --with Pillow python -m unittest discover -s scripts/tests -p test_proof_video.py
```

Tests use temporary synthetic footage, real FFmpeg encode/decode, gap pixels, duration/stream QA, source preservation and refusal checks. Fixture speech replaces the online provider for narrated keep/drop assembly, verifying audible onset against a delayed source track and against the cue clock. Source metadata and chapters are stripped from share copies. Tests do not send text online. Real TTS/export integration must also be exercised on private existing footage before changing the synthesis/render path. Keep that media outside git. No Cargo manifests or app settings are involved.
