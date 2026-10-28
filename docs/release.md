# Releasing Apparent

## Separate Mac preview for friends

The fork's **Mac preview DMG** workflow (`.github/workflows/release-mac-preview.yml`) is a
manual, Apple-silicon-only prototype path; it does **not** run the production steps below.
After merging it into `tryapparent/apparent`'s `main`, choose **Actions → Mac preview DMG → Run
workflow → main**. The job builds on `macos-26`, assigns version `0.<workflow run>.<attempt>` and
tag `preview-mac-<run>-<attempt>`, and creates a GitHub **prerelease** with the DMG and SHA-256
file. A re-run uses a new attempt number; an existing tag is never overwritten. Download it
from the release assets, not from the Actions log.

The app is named `Apparent Preview` with bundle ID `dev.simonvanlaak.apparent.preview`, so it
does not overwrite `Apparent.app` or follow Kit's Sparkle feed. It is ad-hoc signed, **not
notarized**: testers may have to use System Settings → Privacy & Security → Open Anyway.
Updates are **manual downloads**, not in-app automatic updates. It still uses Apparent's normal
capture/config paths, so it is not a separate data profile: don't run it alongside production
Apparent or assume their settings and hotkeys are independent. The preview cannot register itself
at login or install the `apparent` command, to avoid replacing production integrations. Do not use
this workflow for a production release; that still requires the Developer ID, notarization,
Sparkle signing and the checks below.

Everything lands on `main` through pull requests, the version bump and the appcast included. A release
is one Mac build (notarized DMG + Sparkle zip), one Windows zip and two Linux AppImages,
built from the same tagged commit.

1. **Version:** bump `version` in `Cargo.toml` and `Cargo.lock` in a `release/<version>` PR, and in the
   same PR change every `"status": "main"` in `site/src/features.json` to `"stable@<version>"`.
   No push or PR runs CI automatically. Test the change locally before merging.
2. **Mac build** (Studio, from that commit): `APPARENT_BUILD_NUMBER=<n> APPARENT_SPARKLE_DIR=~/.local/share/apparent-release/sparkle-2.9.4 ./scripts/release-app.sh`.
   The build number goes up by one every release (Sparkle compares it, not the version). The script
   builds, signs, notarizes and staples `dist/Apparent-<version>-arm64.dmg` and `.zip`. Its own
   `dist/updates/appcast.xml` is throwaway: the published appcast comes from step 4.
3. **Tag and release:** merge the PR, tag the merge commit `v<version>`, and create the GitHub release
   with the DMG. Manually dispatch the Windows and Linux release workflows on `main`, with
   `tag=v<version>`; each checks out the tag and attaches its assets. Check both runs and assets
   before deploying. Pushing the tag alone no longer starts either workflow.
4. **Appcast** (the Pro, over ssh): copy the Sparkle zip into `~/.local/share/apparent-release/updates/`
   and run `launchctl kickstart gui/$(id -u)/dev.kit.apparent-appcast`. The job
   (`~/Library/LaunchAgents/dev.kit.apparent-appcast.plist` → `~/.local/share/apparent-release/appcast.sh`)
   reads the ed25519 key from the login keychain, which only a GUI-session job can, and runs
   `generate_appcast` over that folder, which holds every earlier zip so it can make deltas. Its log is
   `~/.local/share/apparent-release/appcast.log` and ends with `exit=0`. Copy `updates/appcast.xml` into
   `site/public/appcast.xml` in a PR and check the new item's `edSignature` against the app's
   `SUPublicEDKey`. For a dry run, run `appcast.sh <folder> <log>` as a one-off launchd job on a copy
   of the folder.
5. **Site** (the Pro's checkout, where `wrangler` is signed in): fast-forward to `main`, copy the DMG,
   Sparkle zip, Windows zip and the new `Apparent<n>-*.delta` files into `site/public/releases/` (also as
   `Apparent-latest-*`), then `bun run build && bunx wrangler@latest deploy`.
6. **Homebrew:** a PR to `kitlangton/homebrew-tap` bumping `version` and `sha256` in `Casks/apparent.rb`.

Keep the Sparkle tools and the updates folder under `~/.local/share/apparent-release/`, never in `target/`
or `/tmp`: a clean or a reboot would take the tools and every earlier zip with them.
`scripts/setup-sparkle.sh` fetches Sparkle (checksummed) into `$APPARENT_SPARKLE_DIR`, or `target/` when
it's unset.

Check after deploying: the live DMG, zips and newest delta match the release's sha256s, the live
appcast's signature verifies, `spctl` accepts the downloaded DMG and the app in it, the site shows
the new version for both the Mac and Windows download buttons, and the Windows zip has `apparent.exe`,
`transcribe.dll`, `ggml.dll`, `ggml-base.dll`, nine `ggml-cpu-<tier>.dll` (x64, sse42, sandybridge,
haswell, alderlake, skylakex, cannonlake, cascadelake, icelake; see [windows.md](windows.md)), the three
Visual C++ runtime DLLs (`VCRUNTIME140.dll`, `VCRUNTIME140_1.dll`, `MSVCP140.dll`), README and LICENSE.
Check both Linux AppImages and their `.sha256` assets on the tagged release before advertising them.
