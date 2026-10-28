#!/usr/bin/env bash
# Build an isolated, ad-hoc-signed Mac preview. Never call the production release script here.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${SCRAWL_PREVIEW_VERSION:?set SCRAWL_PREVIEW_VERSION (e.g. 0.123.1)}"
BUILD_NUMBER="${SCRAWL_PREVIEW_BUILD_NUMBER:?set SCRAWL_PREVIEW_BUILD_NUMBER (e.g. 123.1)}"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "Invalid preview version" >&2; exit 1; }
[[ "$BUILD_NUMBER" =~ ^[0-9]+\.[0-9]+$ ]] || { echo "Invalid preview build number" >&2; exit 1; }

cd "$ROOT"
SCRAWL_PREVIEW_BUILD=1 SCRAWL_HELPER_BUNDLE_ID=dev.simonvanlaak.scrawl.preview.helper \
  cargo build --release --locked -j 3

dist="$ROOT/dist"
app="$dist/Scrawl Preview.app"
staging="$dist/preview-staging"
name="Scrawl-Preview-${VERSION}-arm64.dmg"
rm -rf "$app" "$staging"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$staging"
cp target/release/scrawl target/release/scrawl-helper "$app/Contents/MacOS/"
chmod +x "$app/Contents/MacOS/scrawl" "$app/Contents/MacOS/scrawl-helper"
if [[ -f assets/Scrawl.icns ]]; then
  cp assets/Scrawl.icns "$app/Contents/Resources/Scrawl.icns"
fi

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>scrawl</string>
  <key>CFBundleIconFile</key><string>Scrawl</string>
  <key>CFBundleIdentifier</key><string>dev.simonvanlaak.scrawl.preview</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>Scrawl Preview</string>
  <key>CFBundleDisplayName</key><string>Scrawl Preview</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${BUILD_NUMBER}</string>
  <key>LSMinimumSystemVersion</key><string>15.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSScreenCaptureUsageDescription</key><string>Scrawl captures your screen to build annotated agent bundles.</string>
  <key>NSMicrophoneUsageDescription</key><string>Scrawl records your voice to align transcripts with keyframes.</string>
  <key>NSCameraUsageDescription</key><string>Scrawl mirrors and records a cabled iPhone's screen.</string>
  <key>NSSpeechRecognitionUsageDescription</key><string>Scrawl transcribes your narration on this Mac.</string>
  <key>NSAppleEventsUsageDescription</key><string>Scrawl reads the address of the browser tab you captured, to send a capture of a dev site to its project's session.</string>
</dict>
</plist>
PLIST

plutil -lint "$app/Contents/Info.plist"
# There is deliberately no Sparkle framework or production update feed in this app.
codesign --force --sign - --identifier dev.simonvanlaak.scrawl.preview.helper "$app/Contents/MacOS/scrawl-helper"
codesign --force --sign - --identifier dev.simonvanlaak.scrawl.preview "$app"
codesign --verify --deep --strict --verbose=2 "$app"

cp -R "$app" "$staging/Scrawl Preview.app"
ln -s /Applications "$staging/Applications"
rm -f "$dist/$name" "$dist/$name.sha256"
hdiutil create -quiet -fs HFS+ -format UDZO -volname "Scrawl Preview" -srcfolder "$staging" "$dist/$name"
hdiutil verify "$dist/$name"
(cd "$dist" && shasum -a 256 "$name" > "$name.sha256")
printf 'Built %s\n' "$dist/$name"
