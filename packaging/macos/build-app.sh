#!/bin/sh
# Build Kowalski.app and Kowalski.dmg from the kowalski and kowalski-cli binaries (universal or
# single-arch). Signing and notarization run only when their settings are present, so a build
# without them (a pull request, a local try) still produces an unsigned app for testing.
#
#   packaging/macos/build-app.sh <kowalski> <kowalski-cli> <version> <out-dir>
#
# Signing:       MACOS_SIGN_IDENTITY   "Developer ID Application: <name> (<team>)" in a keychain
# Notarization:  APPLE_ID, APPLE_TEAM_ID, APPLE_APP_PASSWORD (an app-specific password)
set -eu
[ $# -eq 4 ] || { sed -n '5,9p' "$0" | sed 's/^# \{0,1\}//'; exit 1; }
server="$1"; cli="$2"; version="$3"; out="$4"
here="$(cd "$(dirname "$0")" && pwd)"
app="$out/Kowalski.app"
dmg="$out/Kowalski.dmg"

rm -rf "$app" "$dmg"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
sed "s/@VERSION@/$version/g" "$here/Info.plist" > "$app/Contents/Info.plist"
# The launcher needs its own name: APFS is case-insensitive, so "Kowalski" would be "kowalski".
# Universal: one slice per Mac architecture (kowalski itself may be single-arch for local tries).
build="$(mktemp -d)"
for arch in arm64 x86_64; do
  swiftc -O -target "$arch-apple-macos12" -o "$build/kowalski-app-$arch" "$here/Launcher.swift"
done
lipo -create -output "$app/Contents/MacOS/kowalski-app" "$build/kowalski-app-arm64" "$build/kowalski-app-x86_64"
rm -rf "$build"
cp "$server" "$app/Contents/MacOS/kowalski"
cp "$cli" "$app/Contents/MacOS/kowalski-cli"
chmod +x "$app/Contents/MacOS/"*
cp "$here/Kowalski.icns" "$app/Contents/Resources/Kowalski.icns"

if [ -n "${MACOS_SIGN_IDENTITY:-}" ]; then
  echo "==> signing with $MACOS_SIGN_IDENTITY"
  for bin in kowalski kowalski-cli kowalski-app; do
    codesign --force --options runtime --timestamp --sign "$MACOS_SIGN_IDENTITY" "$app/Contents/MacOS/$bin"
  done
  codesign --force --options runtime --timestamp --sign "$MACOS_SIGN_IDENTITY" "$app"
  codesign --verify --deep --strict --verbose=2 "$app"
else
  echo "==> no MACOS_SIGN_IDENTITY: unsigned app (macOS will ask the user to allow it)"
fi

staging="$(mktemp -d)"
cp -R "$app" "$staging/"
ln -s /Applications "$staging/Applications"
hdiutil create -quiet -volname "Kowalski $version" -srcfolder "$staging" -ov -format UDZO "$dmg"
rm -rf "$staging"

if [ -n "${MACOS_SIGN_IDENTITY:-}" ]; then
  codesign --force --timestamp --sign "$MACOS_SIGN_IDENTITY" "$dmg"
  if [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ] && [ -n "${APPLE_APP_PASSWORD:-}" ]; then
    echo "==> notarizing (Apple usually answers within minutes)"
    xcrun notarytool submit "$dmg" --apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" \
      --password "$APPLE_APP_PASSWORD" --wait
    xcrun stapler staple "$dmg"
    spctl --assess --type open --context context:primary-signature --verbose=2 "$dmg"
  else
    echo "==> no Apple ID settings: signed but not notarized"
  fi
fi
shasum -a 256 "$dmg" | sed "s|$out/||" > "$dmg.sha256"
echo "==> $dmg"
