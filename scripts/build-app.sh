#!/bin/zsh
set -eu
cd "${0:A:h:h}"
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
renderer="${1:-glow}"
version="$(awk -F '\"' '/^version = / { print $2; exit }' Cargo.toml)"
case "$renderer" in
  glow) target_dir="target"; bundle="dist/SnapNote.app" ;;
  wgpu) target_dir="target/renderer-wgpu"; bundle="dist/comparison/wgpu/SnapNote.app" ;;
  *) print -u2 'usage: build-app.sh [glow|wgpu]'; exit 2 ;;
esac
# Keep one stable signing identity so macOS keeps Screen Recording permission
# across rebuilds. SNAPNOTE_ADHOC_SIGNING=1 is an explicit opt-out for throwaway
# builds and release packaging.
if [[ "${SNAPNOTE_ADHOC_SIGNING:-0}" == "1" ]]; then
  signing_identity="-"
  print -u2 "Warning: ad-hoc signing will reset Screen Recording permission on every rebuild."
else
  signing_identity="${SNAPNOTE_SIGNING_IDENTITY:-SnapNote Local Signing}"
  ./scripts/dev-signing-identity.sh "$signing_identity"
fi
cargo build --release --locked --no-default-features --features "$renderer" --target-dir "$target_dir"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp "$target_dir/release/snapnote" "$bundle/Contents/MacOS/SnapNote"
helper="$bundle/Contents/Helpers/SnapNote Editor.app"
mkdir -p "$helper/Contents/MacOS" "$helper/Contents/Resources"
cp "$target_dir/release/snapnote-editor" "$helper/Contents/MacOS/SnapNote Editor"
cp assets/SnapNote.icns "$helper/Contents/Resources/SnapNote.icns"
cat > "$helper/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>SnapNote Editor</string>
<key>CFBundleIdentifier</key><string>local.snapnote.editor</string>
<key>CFBundleName</key><string>SnapNote Editor</string>
<key>CFBundleIconFile</key><string>SnapNote.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>LSUIElement</key><true/>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign --force --sign "$signing_identity" "$helper"
cp assets/SnapNote.icns "$bundle/Contents/Resources/SnapNote.icns"
cat > "$bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>SnapNote</string>
<key>CFBundleIdentifier</key><string>local.snapnote.app</string>
<key>CFBundleName</key><string>SnapNote</string>
<key>CFBundleIconFile</key><string>SnapNote.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>LSMinimumSystemVersion</key><string>12.0</string>
<key>LSUIElement</key><true/>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign --force --sign "$signing_identity" "$bundle"
print "Built $bundle ($renderer, signed by $signing_identity)"
