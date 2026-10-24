#!/bin/zsh
set -eu
cd "${0:A:h:h}"
[[ "$(uname -m)" == arm64 ]] || { print -u2 'Build this release on an Apple silicon Mac.'; exit 1; }
# Releases are not notarized. Sign with an explicit Developer ID if provided;
# otherwise keep the historical ad-hoc signature instead of using the local
# development certificate, which only exists on this Mac.
if [[ -n "${SNAPNOTE_SIGNING_IDENTITY:-}" ]]; then
  ./scripts/build-app.sh
else
  SNAPNOTE_ADHOC_SIGNING=1 ./scripts/build-app.sh
fi
bundle="dist/SnapNote.app"
output="${PWD}/dist/release"
mkdir -p "$output"
stage="$(mktemp -d "${TMPDIR:-/tmp}/snapnote-release.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
ditto "$bundle" "$stage/SnapNote.app"
ln -s /Applications "$stage/Applications"
cat > "$stage/Read Me.txt" <<'TEXT'
SnapNote — A screenshot. With context.

1. Drag SnapNote into Applications.
2. Open SnapNote from Applications. It lives in your menu bar.
3. Allow Screen Recording in System Settings when prompted.
4. Press Command+B, select a region, annotate, and add your context.

This first release is ad-hoc signed, not Apple-notarized. If macOS
blocks the app, open System Settings > Privacy & Security after
attempting to launch, then choose Open Anyway for SnapNote.
Only approve the app if you downloaded it from the official release:
https://github.com/Tsohnle95/SnapNote/releases

Requires an Apple silicon Mac (M1 or later) running macOS 12+.
Your captures stay on your Mac. No account or subscription.
TEXT
codesign --verify --deep --strict "$stage/SnapNote.app"
hdiutil create -volname SnapNote -srcfolder "$stage" -ov -format UDZO \
  -o "$output/SnapNote-macOS-arm64.dmg"
ditto -c -k --sequesterRsrc --keepParent "$bundle" "$output/SnapNote-macOS-arm64.zip"
(
  cd "$output"
  shasum -a 256 SnapNote-macOS-arm64.dmg SnapNote-macOS-arm64.zip > SHA256SUMS.txt
)
print "Release files: $output"
