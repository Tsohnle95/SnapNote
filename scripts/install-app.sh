#!/bin/zsh
# Replace the installed SnapNote with a freshly built bundle and relaunch it.
#
# Quits the running resident first, refuses to touch an open capture (which may
# hold an unsaved draft), preserves the stable code signature, and launches the
# exact installed path so macOS does not resolve the dist copy instead.
set -eu
cd "${0:A:h:h}"
source_bundle="${1:-dist/SnapNote.app}"
install_root="${SNAPNOTE_INSTALL_DIR:-/Applications}"
destination="$install_root/SnapNote.app"

[[ -d "$source_bundle" ]] || {
    print -u2 "Missing $source_bundle. Run ./scripts/build-app.sh first."
    exit 1
}
codesign --verify --deep --strict "$source_bundle"

if pgrep -f "SnapNote Editor.app/Contents/MacOS/SnapNote Editor" >/dev/null; then
    print -u2 "Close the SnapNote capture window before installing; it may hold an unsaved draft."
    exit 1
fi

# Stop every copy (dist or Applications) so no stale resident keeps running.
pkill -TERM -f "/SnapNote.app/Contents/MacOS/SnapNote" 2>/dev/null || true
for _ in {1..30}; do
    pgrep -f "/SnapNote.app/Contents/MacOS/SnapNote" >/dev/null || break
    sleep 0.1
done
if pgrep -f "/SnapNote.app/Contents/MacOS/SnapNote" >/dev/null; then
    print -u2 "SnapNote did not quit. Quit it from the menu bar and try again."
    exit 1
fi

rm -rf "$destination"
ditto "$source_bundle" "$destination"
codesign --verify --deep --strict "$destination"

# Refresh LaunchServices so open-by-name prefers the installed bundle.
lsregister="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
[[ -x "$lsregister" ]] && "$lsregister" -f "$destination" >/dev/null 2>&1 || true
open -n "$destination"
print "Installed $destination"
