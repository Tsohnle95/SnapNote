#!/bin/zsh
# One command for the local loop: build, install into /Applications, relaunch.
set -eu
cd "${0:A:h:h}"
renderer="${1:-glow}"
./scripts/build-app.sh "$renderer"
case "$renderer" in
  glow) bundle="dist/SnapNote.app" ;;
  wgpu) bundle="dist/comparison/wgpu/SnapNote.app" ;;
  *) print -u2 'usage: build-and-install.sh [glow|wgpu]'; exit 2 ;;
esac
./scripts/install-app.sh "$bundle"
