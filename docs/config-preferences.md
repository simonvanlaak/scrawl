# Config preference coverage

The canonical file is `~/.config/apparent/config.json`; `APPARENT_CONFIG` replaces
its path. It contains requested preferences, not proof that the OS applied them.
Keep unknown keys and secrets when editing it. Application writers must reject
malformed, non-object or unreadable existing files rather than replacing them.

This table records source contracts for the config-preference candidate, based on
the committed shared writer from PR #180 (`191f1df857e356a0bccdf2b27c63631f904ee89e`).
Shortcut and experiment maps mutate inside its locked transaction. This is not
a claim that every row has been exercised on both operating systems.

## User controls

| Preference | JSON path; default and validation | Consumer and file-edit timing |
| --- | --- | --- |
| Launch at login | `launchAtLogin`: boolean; absent leaves OS state alone | Shared Settings writes intent, then reconciles with macOS login registration / Linux autostart. Launch and periodic reconciliation. Inspect actual OS state separately. |
| Record shortcut | `shortcuts.record`: `{keyCode,modifiers,label}`; platform default when absent | Settings applies immediately; direct file edits require restart. Record cannot be disabled. Hyprland-managed shortcuts can take precedence. |
| Paste shortcut | `shortcuts.paste`: same object, or `null` to disable; platform default when absent | Same as Record. OS reservation/registration is independent of JSON. |
| Microphone | `microphone`: `{id,name}`; absent follows system default | Next recording / input stream. A saved unavailable device stays saved while input falls back. Both OSes; Linux device enumeration requires a suitable audio backend. |
| Lower playback | `lowerPlayback`: boolean, false; `playbackLevel`: integer, 20, capped at 100 after u8 parsing | Next recording guard. Linux mechanics and UI are owned by #35, not this change. |
| Transcription engine | `transcriber`: `apple`, `hex`, `command`; invalid names select platform-local engine | Next engine selection. Local means Apple Speech on Mac and Parakeet on Linux. |
| Warm local recognizer | `warmRecognizer`: boolean, true | Linux Parakeet lifecycle; no Mac counterpart. |
| Hex model and language | `hexModel`: model ID or automatic; `hexLanguage`: supported code, `en` by default | Next Hex selection. Unavailable models and unsupported languages are handled by the engine; language falls back to English. |
| Apple language | `appleLocale`: supported BCP-47 string; absent means automatic | Mac only. Settings restarts the model check; restart after direct file edits to refresh its cached status. |
| Command transcriber | `transcribeCommand`: nonempty string array with absolute executable; `transcribeTimeoutSeconds`: positive bounded number | Loaded when command transcription is selected. Invalid command configuration fails explicitly; no shell evaluation. See keyframes-and-text.md. |
| Export menu | `videoExport`: boolean, false | Current menu construction; CLI export is independent. |
| Capture location | `capturesDir`: absolute path or home-expanded path; absent uses platform capture directory | Read at destination lookup. Environment override wins. Existing captures are not moved. Linux Settings currently displays location without the Mac folder chooser. |
| Retention | `retention.mediaDays`: u32 or null, 7; `retention.bundleDays`: u32 or null, 30 | Each cleanup pass; Settings requests a pass. Null means forever, zero means immediately eligible. Invalid field falls back independently, preserving a valid sibling. Per-bundle `.pinned` is a separate object-level choice. |
| Last capture mode | `lastCaptureMode`: `area`, `window`, `screen`, `iphone`; default `area` | Loaded at app startup; restart after direct edits. iPhone remains Mac-only. Saved when recording starts. Legacy `last-mode` is read only when the key is absent, and removed only after a successful canonical write. |
| Update channel | `updateChannel`: `stable` or `main`, default stable | Linux subsequent check and pre-install channel guard. Mac Sparkle has no channel control here. |
| Automatic update checks | `automaticUpdateChecks`: boolean; absent retains existing updater defaults | Linux reads before each launch/hourly background check. Mac applies an explicit value before starting Sparkle; restart after direct file edits. Manual checks remain available. |
| Automatic update downloads | `automaticUpdateDownloads`: boolean; absent retains existing updater defaults | Linux false performs discovery only; manual Check still downloads and installs. Rechecked before background download and after download before swap. Mac applies explicit value before Sparkle starts; current packages prohibit automatic downloads and installation (`SUAllowsAutomaticUpdates=false`), even when this preference is true. |
| Native capsule | `nativePill`: boolean, true on Mac; `pillStyle`: black/glass aliases, black by default | Mac only. Environment/debug tune overrides take precedence. Restart to replace an existing renderer. |
| Screen auto-crop | `autocrop`: boolean or settings object; absent/true uses defaults, false disables | Next processing operation. See keyframes-and-text.md for individual crop fields and command overrides. |
| Experiments | `experiments.<name>`: boolean, supported boolean string or number; false except `agentDemoVideo` | Read at each feature call site. Invalid entries are skipped individually. Settings edits agentDemoVideo; CLI `--experiment` is process-local. Current names and limitations live in keyframes-and-text.md and the feature map. |
| Legacy routing helpers | `routing.apps`, `routing.servers`: string maps; `agentHost`, `agentOpencode`: restricted command/host strings | Retained utilities, not automatic capture delivery. `autoSend` is compatibility data, not an active delivery preference. |

Settings and the microphone CLI use the shared top-level config writer. Experiment
and shortcut maps require a transaction that reads and mutates the nested value
under the same writer lock; a pre-lock snapshot is not safe. No queue or network
lock may be acquired inside that transaction.

## Other stores and non-preferences

- Legacy Application Support shortcut config is migration input, not a second
  active preference authority. Delete it only after canonical persistence succeeds.
- `tune.json` is a documented debug-only motion laboratory. Release builds ignore
  it. Its CLI and reload behavior are documented in motion.md; it is not a hidden
  release-app preference store.
- Hex's own settings and Hyprland bindings are external application/compositor
  preferences. Apparent must not rewrite them to make its JSON appear effective.
- Onboarding progress, inbox history, active recording, running PID, installed
  update markers, backup metadata and check results are runtime/history data.
- Permissions, system appearance, installed CLI links and URL-handler registration
  are observed OS state. JSON does not grant access or prove registration.

## Remaining acceptance gaps

Do not claim whole-app coverage until these source findings are resolved or
explicitly scoped out with evidence:

- Hand-edited shortcuts still need the same semantic validation as Settings.
- Capture-directory fallback currently checks permission bits, not successful
  creation; ACLs, foreign-owned directories and read-only mounts need proof.
- Auto-crop object deserialization can still discard valid sibling fields when
  one field is malformed; retention now isolates those failures.
- Explicit Mac automation keys override Sparkle, but absent keys still inherit
  its UserDefaults/package policy. Check interval and system-profile preference
  remain Sparkle-owned. This is not yet a single-authority preference contract.
- Exact-source Linux GUI and Mac runtime proof, current-main integration and
  independent review are required before calling the candidate merge-ready.
