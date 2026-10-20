#!/usr/bin/env bash
# Build a DMG from a built .app, the way the swift-* repositories do: `hdiutil` directly, with an
# Applications link, and no Finder scripting (Tauri's own DMG bundler asks Finder to lay out the
# window, which times out without the Automation permission). macOS only.
#
# usage: build-dmg.sh <built .app> <output .dmg>
set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "usage: $0 <built .app> <output .dmg>" >&2
    exit 64
fi

app="$1"
dmg="$2"
[[ -d "$app" ]] || { echo "Error: app not found at '$app'" >&2; exit 1; }

name="$(basename "$app" .app)"
staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT

cp -R "$app" "$staging/"
ln -s /Applications "$staging/Applications"

mkdir -p "$(dirname "$dmg")"
rm -f "$dmg"
hdiutil create -volname "$name" -srcfolder "$staging" -ov -format UDZO "$dmg" >/dev/null
# Sign with SIGN_IDENTITY (a Developer ID Application certificate) when the Makefile found one, else ad-hoc.
codesign --force --sign "${SIGN_IDENTITY:--}" "$dmg"
echo "Built: $dmg"
