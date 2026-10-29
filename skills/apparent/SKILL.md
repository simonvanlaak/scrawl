---
name: apparent
description: Read and dig into Apparent screen captures, where someone recorded their screen while talking, clicking and drawing red marks on it. Use whenever a message contains an Apparent capture (a line like `[apparent 0:12 · App “Window” · /path/to/….apparent]`, a `.apparent` bundle path, or a contact-sheet.png from one), when the user says "look at my apparent" or refers to something they just recorded or drew, or when you need the `apparent` CLI to zoom into a moment, pull frames, copy a capture, or export one as video.
---

# Apparent captures

Apparent records someone's screen, voice, clicks and red ink, then packs the result into a `.apparent` folder (a bundle). They send it to show you a problem instead of describing it, so the capture is the bug report. Read it closely before you answer, and don't ask them to explain again what the capture already shows. If something is unclear, look closer with `apparent zoom` first.

## What a paste looks like

```text
[apparent 0:33 · OpenCode “Settings” · /Users/me/Library/Application Support/apparent/captures/apparent-1792432139176.apparent]
Open contact-sheet.png and frames/ in that folder before changing anything: the marks and words only make sense on the images, which can show more than the text does.
Cropped to: the activity in OpenCode (full frames in frames-full/)
- #1 (0:04) [clicked “New tab”] “Command T makes a new session, but here it should make a tab.”
- #2 (0:07) “No, it doesn't.”
- #3 (0:10) ⚑ “It says command shift B makes a new tab.”
- #4 (0:18) clicked (x 0.51, y 0.93)
```

- The first line has the length, the app and window recorded, and the bundle path.
- The next line asks you to open the images first. Do: the text is only what was said and done, and a problem can show in a frame without being marked or mentioned.
- Each `#N (M:SS)` line is a keyframe: what they did at that moment in brackets, then what they said while it was on screen. A line with no quote has the action alone.
- `⚑` means they flagged the moment on purpose. `⟨Name⟩` means the app reported a new screen. Lines starting `·` are events or values the app logged.
- Optional lines near the top: `Screen:` with app context, `Note:`, and `Cropped to:`. Older bundles may include `For session:` as recorded metadata; that does not authorize delivery.
- Coordinates are fractions of the captured area, from the top left (x 0.51 is just right of centre).

When you answer, refer to moments the same way, as in "#3 (0:10)", so the person can find them on the contact sheet.

## Reading a bundle

1. Look at `contact-sheet.png` in the bundle first (it's often attached already). It shows every keyframe in time order, left to right and top to bottom, each with its number `N` in a small white chip in a top corner (the `#N` in `summary.md`).
2. Read the transcript lines next to it. Speech recognition mishears names and code terms, so trust the screen when the two disagree.
3. Open `frames/frame-NN.png` for any keyframe you need at full size (`frame-03.png` is #3). When the summary says `Cropped to:`, the uncropped screens are in `frames-full/`.

If the contact sheet is not attached, try opening it at the bundle path in the first line. A plain `apparent copy` is text-only by default; the Paste control offers the image first and the text second, but a paste keystroke does not prove the composer accepted either. If you cannot open the local path (for example, the capture is on another machine), ask for `contact-sheet.png` as an attachment and check that you can actually view it. Do not claim to know what a click or mark targeted from “element unavailable” or coordinates alone; wait for the image when that target matters to the request. Never send a draft on the user's behalf.

What the drawings mean:

- Red strokes, circles, underlines and arrows are the person's marks, and they point at the problem. Marks drawn on the same screen all show on one keyframe. When there are several, the text numbers them in drawing order (`✎① 0:06 …`, `✎② 0:09 …`); the image shows only the ink, so use `apparent zoom --mark` to see which ink is which.
- A ripple around the cursor shows a click or tap, and a finger trail shows a swipe.
- A glowing red trail that tapers off is the laser pointer, used to point without leaving a mark.

Other files: `apparent.json` is the full manifest (keyframes, `marks` with shape and box, `events`, `transcript.words` with times, app `context`, and `autocrop`). `recording.mp4` or `recording.mov` and `audio.wav` are the source media. A `.media-pending` file means the video is still being copied from another Mac.

## The apparent CLI

### Configuring Apparent

The Settings controls for recording, shortcuts and retention use `~/.config/apparent/config.json` (`APPARENT_CONFIG` overrides the path). The feature map at `site/src/features.json` in the Apparent repository lists config keys, accepted values and platforms. Change only the needed key, preserve other keys, and keep the file private (`0600` on Unix): old configs may contain private values. Never paste or print the whole config. A programmatic edit must preserve valid JSON, rather than replacing the file with a partial object. The UI writes changes there; an explicit `"launchAtLogin": true` or `false` also updates OS registration on app launch or within two seconds while it runs. A disabled OS login item can still need the user's approval. Some values (such as captures folder) are read on demand; shortcuts are registered at app startup or when changed through Settings, so restart Apparent after editing `shortcuts` by hand. Permissions are OS grants, not JSON settings; do not claim to enable them by editing this file. Installing the CLI or the agent skill is an installation action, not an app preference.

`apparent` is the installed Apparent app's binary on macOS or Linux. If `command -v apparent` finds nothing on a Mac, use `/Applications/Apparent.app/Contents/MacOS/apparent` (or the same path under `~/Applications`); on Linux, use the installed `~/.local/bin/apparent` link or the AppImage's path. Settings › Agents can install the CLI link without replacing another program. Run the command on the computer with Apparent and access to the bundle; elsewhere, read `contact-sheet.png`, `frames/` and `apparent.json` directly. Most commands take a bundle path or `latest`, the newest capture. Quote paths containing spaces, such as macOS `Application Support` paths.

### Look closer at a moment

Use `zoom` when the problem is motion (an animation, a jump, a flicker) or happens between keyframes, or when you can't tell what a mark points at:

```sh
apparent zoom <bundle> --at 0:21                 # a time, M:SS or seconds
apparent zoom <bundle> --at "the width changes"  # or a phrase they said
apparent zoom <bundle> --at 0:21 --span 1 --frames 16 --full
apparent zoom <bundle> --mark 2                  # the mark shown as ✎② in the text
apparent zoom <bundle> --mark 1 --frame 4        # ✎① of keyframe #4, when several have a ✎①
```

It prints the path of a PNG sheet of frames around that moment, the time of each cell, and what was said. Open the PNG and look at it before you act on the feedback. `--span` is the seconds covered (default 2, a quarter of it before the moment), `--frames` the number of cells (default 12), and `--full` shows the whole screen instead of the crop. The cells are numbered from #1 within the sheet, so those numbers aren't keyframe numbers.

`--mark` takes a mark's number from the text or its id (`m2`) and centres the sheet on the time it was drawn, with only that mark's ink, drawn whole from the moment it starts. `apparent inspect <bundle> --marks` lists every mark (`#2 ✎② 0:09 marked (2 strokes) · m2`: keyframe, number, time, shape, what it was drawn on, id), and `--marks --json` gives the same as JSON.

### Other commands

```sh
apparent inspect <bundle>               # summary.md, the text above
apparent inspect <bundle> --json        # apparent.json
apparent inspect <bundle> --marks       # the marks, numbered as in the text [--json]
apparent frames <bundle> --from 0:04 --to 0:06 [--fps 4]   # plain PNGs of a range; prints their paths
apparent copy <bundle>                  # copy the capture to the clipboard
apparent export <bundle> [--out demo.mp4] [--zoom] [--crop auto|full|x,y,w,h] [--size 1920]
```

- `zoom` and `frames` write into `<bundle>/zoom/`. Leave the rest of the bundle as it is, because Apparent manages those files and cleans them up.
- Apparent does not send captures to agent sessions. Use `copy` to put a finished capture on the clipboard.
- `export` is macOS only; Linux does not support video export. On Mac it writes one `.mp4` with the ink and clicks burned in and the audio in sync (default `~/Movies/Apparent/<bundle>.mp4`). `--zoom` eases in toward the activity. `--no-ink`, `--no-clicks`, `--hevc` and `--fps` are also available.
- On Linux, `apparent update status` prints JSON with the selected channel, the **live process** version (`running.version`, or `null` if none), the installed AppImage identity, and whether a restart is pending. The `running` object in `~/.config/apparent/config.json` is a launch record, not proof the process is still alive after a crash; use `status` to validate its PID and start time. `apparent update channel stable|main` chooses the feed; `apparent update check` checks and stages an update for the next launch (installed AppImages only).
- Video and audio go to the Trash 7 days after a capture unless the person changed that in Settings. After that `zoom`, `frames` and `export` can't read the video, but `frames/`, the contact sheet and `apparent.json` remain.
- `apparent <command> --help` lists every flag, and `apparent skill` prints this guide as the installed version knows it.

The optional `make-it-apparent` workflow for showing a finished change on video is a **separate** skill. The default capture-reading skill install does not guarantee it is present; install it explicitly in the agent's environment with `bunx skills add tryapparent/apparent --skill make-it-apparent`. Installing a skill does not install the desktop app or grant recording permissions.
