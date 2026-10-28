# iPhone sources, ScrawlKit ingest and dev-site routing

Read this before changing `src/phone.rs`, `src/delivery.rs`, `src/routing.rs`, `src/web.rs`, `src/sdk.js`, `Sources/ScrawlKit/` or the helper's phone capture. The delivery recovery rules are in `AGENTS.md` because they apply even when you aren't editing this code.

## iPhone sources (`src/phone.rs`, `helper/`)

- `scrawl-helper` (Swift, built by `build.rs` into `target/<profile>/`, copied and signed into `Scrawl.app/Contents/MacOS` by `install-app.sh`) does SpeechAnalyzer, CoreMediaIO/AVFoundation phone capture with the mirror window, and movie decoding. Swift was chosen over objc2 bindings because SpeechAnalyzer has no Objective-C surface and `objc2-av-foundation` is not in the lockfile.
- Picker discovery (`scrawl-helper devices`) reads the USB registry only: it is instant and never touches AVFoundation, so opening the picker can't raise a camera prompt or hang. The first time iPhone is chosen without Camera, Scrawl closes the picker and opens the onboarding window's Camera row instead of starting the mirror (whose prompt would land under the picker).
- Live capture: the mirror window's rect (global top-left points, streamed by the helper) is the telemetry target, so ink/laser/clicks normalize into phone coordinates; inputs are re-based onto the movie's clock (`recording.startedAt`) at finish. Keyframes are selected on 320 px candidates and re-decoded from the native video at a 1600 px long edge.
- ScrawlKit ingest (`ingest_recording`): the Mac saves the bundle and answers `200 {"ok":true,"delivered":false,"state":"saved","sessionTitle":"Mac"}`, or `state:"processing"` after the phone's `X-Scrawl-Wait` (default 25 s, max 45 s). A failed build answers 500 and forgets the recording so it can be sent again. Copies of one recording (LAN + Tailscale, HTTP + USB, keyed by `recordedAt`) wait for the first and get its outcome. No upload is automatically forwarded to an agent session, even if the original timeline names one. Over USB the Mac writes `<stamp>.delivery` (the same JSON) before `<stamp>.sent` ("Mac" for older ScrawlKit readers). Tests swap bundle building through `IngestHooks`.
- The GUI's listener is loopback-only by default. General › Allow phone uploads from other devices (`remotePhoneIngest`) enables the all-IPv4 bind and rebinds live when switched off; off does not close the local port. Switching off rejects pending remote requests and closes accepted sockets; an upload already admitted for bundle building may finish. The explicit headless `scrawl serve` command still accepts remote connections regardless of this GUI setting. The token does not replace firewall and reachability checks.
- Audio for transcription is always the movie's audio exported on the asset timeline (`scrawl-helper audio`). Do not feed a `.mov` straight to SpeechAnalyzer: `AVAudioFile` ignores the movie timeline and its word times drift (≈1.5 s late at 41 s on a phone recording).

## Dev-site routing (`src/routing.rs`, `src/web.rs`, `src/sdk.js`)

Screen and Window captures are saved and copied, without looking up an OpenCode session or sending anywhere. Old routing code and `/sdk.js` remain for compatibility, but nothing calls the capture-routing entry point.

The old `smartRouting` experiment has no effect on capture delivery.
