#!/usr/bin/env bash
# Install a built .app into the deploy folder (macOS only).
#
# usage: install-app.sh <built .app> <deploy folder> <app name>
#
# `ditto` keeps the timestamp of the .app, which `install-my-apps` compares with the .app in the
# published DMG to tell whether the installed app is up to date. Install from the DMG's copy
# (not from a separate build output) so that the two match.
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: $0 <built .app> <deploy folder> <app name>" >&2
    exit 64
fi

src="$1"
deploy_dir="$2"
name="$3"

[[ -d "$src" ]] || { echo "Error: build output not found at '$src'" >&2; exit 1; }

mkdir -p "$deploy_dir"
rm -rf "${deploy_dir:?}/$name.app"
ditto "$src" "$deploy_dir/$name.app"
echo "Installed: $deploy_dir/$name.app"
