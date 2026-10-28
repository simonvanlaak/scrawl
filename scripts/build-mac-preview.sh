#!/usr/bin/env bash
# Build an isolated, ad-hoc-signed Mac preview. Never call the production release script here.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${SCRAWL_PREVIEW_VERSION:?set SCRAWL_PREVIEW_VERSION (e.g. 0.123.1)}"
BUILD_NUMBER="${SCRAWL_PREVIEW_BUILD_NUMBER:?set SCRAWL_PREVIEW_BUILD_NUMBER (e.g. 123.1)}"
PUBLIC_KEY="${SCRAWL_PREVIEW_PUBLIC_KEY:?set the preview Sparkle EdDSA public key}"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "Invalid preview version" >&2; exit 1; }
[[ "$BUILD_NUMBER" =~ ^[0-9]+\.[0-9]+$ ]] || { echo "Invalid preview build number" >&2; exit 1; }
python3 - "$PUBLIC_KEY" <<'PY'
import base64, sys
try:
    key = base64.b64decode(sys.argv[1], validate=True)
    if len(key) != 32:
        raise ValueError('wrong public key length')
except ValueError:
    sys.exit('Preview Sparkle public key must be 32 base64-encoded bytes')
PY

cd "$ROOT"
SPARKLE_DIR="$(scripts/setup-sparkle.sh)"
SCRAWL_PREVIEW_BUILD=1 SCRAWL_HELPER_BUNDLE_ID=dev.simonvanlaak.scrawl.preview.helper \
  cargo build --release --locked -j 3

dist="$ROOT/dist"
app="$dist/Scrawl Preview.app"
staging="$dist/preview-staging"
name="Scrawl-Preview-${VERSION}-arm64.dmg"
zip="Scrawl-Preview-${VERSION}-arm64.zip"
rm -rf "$app" "$staging"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$app/Contents/Frameworks" "$staging"
cp target/release/scrawl target/release/scrawl-helper "$app/Contents/MacOS/"
chmod +x "$app/Contents/MacOS/scrawl" "$app/Contents/MacOS/scrawl-helper"
if [[ -f assets/Scrawl.icns ]]; then
  cp assets/Scrawl.icns "$app/Contents/Resources/Scrawl.icns"
fi
ditto "$SPARKLE_DIR/Sparkle.framework" "$app/Contents/Frameworks/Sparkle.framework"

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
  <key>SUEnableAutomaticChecks</key><true/>
  <key>SUAutomaticallyUpdate</key><false/>
  <key>SUAllowsAutomaticUpdates</key><false/>
  <key>SUFeedURL</key><string>https://raw.githubusercontent.com/tryapparent/apparent/preview-mac-feed/appcast.xml</string>
  <key>SUPublicEDKey</key><string>${PUBLIC_KEY}</string>
  <key>SUVerifyUpdateBeforeExtraction</key><true/>
</dict>
</plist>
PLIST

plutil -lint "$app/Contents/Info.plist"
sparkle="$app/Contents/Frameworks/Sparkle.framework/Versions/B"
codesign --force --sign - "$sparkle/XPCServices/Installer.xpc"
codesign --force --sign - --preserve-metadata=entitlements "$sparkle/XPCServices/Downloader.xpc"
codesign --force --sign - "$sparkle/Autoupdate"
codesign --force --sign - "$sparkle/Updater.app"
codesign --force --sign - "$app/Contents/Frameworks/Sparkle.framework"
codesign --force --sign - --identifier dev.simonvanlaak.scrawl.preview.helper "$app/Contents/MacOS/scrawl-helper"
codesign --force --sign - --identifier dev.simonvanlaak.scrawl.preview "$app"
codesign --verify --deep --strict --verbose=2 "$app"

cp -R "$app" "$staging/Scrawl Preview.app"
ln -s /Applications "$staging/Applications"
rm -f "$dist/$name" "$dist/$name.sha256"
hdiutil create -quiet -fs HFS+ -format UDZO -volname "Scrawl Preview" -srcfolder "$staging" "$dist/$name"
hdiutil verify "$dist/$name"
(cd "$dist" && shasum -a 256 "$name" > "$name.sha256")
ditto -c -k --sequesterRsrc --keepParent "$app" "$dist/$zip"
printf 'Built %s and %s\n' "$dist/$name" "$dist/$zip"
