#!/usr/bin/env bash
# Verify the signature of a built .app and say whether it is certificate-signed or ad-hoc.
# usage: verify-signature.sh <app>  (exit 1 when the signature is invalid; ad-hoc is only reported)
set -euo pipefail

app="${1:?usage: $0 <app>}"
codesign --verify --deep --strict "$app"
info="$(codesign -dvv "$app" 2>&1)"
if grep -q 'Signature=adhoc' <<<"$info"; then
    echo "Signature: ad-hoc (permissions will be asked again after every build)."
else
    echo "Signature: $(sed -n 's/^Authority=//p' <<<"$info" | head -1)"
    echo "Requirement: $(codesign -d -r- "$app" 2>&1 | sed -n 's/^designated => //p')"
fi
