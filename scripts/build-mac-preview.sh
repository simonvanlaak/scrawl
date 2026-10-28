#!/usr/bin/env bash
# Build an isolated, ad-hoc-signed Mac preview. Never call the production release script here.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${APPARENT_PREVIEW_VERSION:?set APPARENT_PREVIEW_VERSION (e.g. 0.123.1)}"
BUILD_NUMBER="${APPARENT_PREVIEW_BUILD_NUMBER:?set APPARENT_PREVIEW_BUILD_NUMBER (e.g. 123.1)}"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "Invalid preview version" >&2; exit 1; }
[[ "$BUILD_NUMBER" =~ ^[0-9]+\.[0-9]+$ ]] || { echo "Invalid preview build number" >&2; exit 1; }

cd "$ROOT"
APPARENT_PREVIEW_BUILD=1 APPARENT_HELPER_BUNDLE_ID=dev.simonvanlaak.apparent.preview.helper \
  cargo build --release --locked -j 3

dist="$ROOT/dist"
app="$dist/Apparent Preview.app"
staging="$dist/preview-staging"
name="Apparent-Preview-${VERSION}-arm64.dmg"
rm -rf "$app" "$staging"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$staging"
cp target/release/apparent "$app/Contents/MacOS/apparent"
cp target/release/apparent-helper "$app/Contents/MacOS/"
chmod +x "$app/Contents/MacOS/apparent" "$app/Contents/MacOS/apparent-helper"
if [[ -f assets/Apparent.icns ]]; then
  cp assets/Apparent.icns "$app/Contents/Resources/Apparent.icns"
fi

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>apparent</string>
  <key>CFBundleIconFile</key><string>Apparent</string>
  <key>CFBundleIdentifier</key><string>dev.simonvanlaak.apparent.preview</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>Apparent Preview</string>
  <key>CFBundleDisplayName</key><string>Apparent Preview</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${BUILD_NUMBER}</string>
  <key>LSMinimumSystemVersion</key><string>15.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSScreenCaptureUsageDescription</key><string>Apparent captures your screen to build annotated agent bundles.</string>
  <key>NSMicrophoneUsageDescription</key><string>Apparent records your voice to align transcripts with keyframes.</string>
  <key>NSCameraUsageDescription</key><string>Apparent mirrors and records a cabled iPhone's screen.</string>
  <key>NSSpeechRecognitionUsageDescription</key><string>Apparent transcribes your narration on this Mac.</string>
  <key>NSAppleEventsUsageDescription</key><string>Apparent reads the address of the browser tab you captured, to send a capture of a dev site to its project's session.</string>
</dict>
</plist>
PLIST

plutil -lint "$app/Contents/Info.plist"
# There is deliberately no Sparkle framework or production update feed in this app.
codesign --force --sign - --identifier dev.simonvanlaak.apparent.preview.helper "$app/Contents/MacOS/apparent-helper"
codesign --force --sign - --identifier dev.simonvanlaak.apparent.preview "$app"
codesign --verify --deep --strict --verbose=2 "$app"

cp -R "$app" "$staging/Apparent Preview.app"
ln -s /Applications "$staging/Applications"
rm -f "$dist/$name" "$dist/$name.sha256"
hdiutil create -quiet -fs HFS+ -format UDZO -volname "Apparent Preview" -srcfolder "$staging" "$dist/$name"
hdiutil verify "$dist/$name"
(cd "$dist" && shasum -a 256 "$name" > "$name.sha256")
printf 'Built %s\n' "$dist/$name"
