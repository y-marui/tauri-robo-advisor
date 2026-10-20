#!/usr/bin/env bash
# Print the code-signing identity for the app, or nothing when there is none (the caller then falls back
# to the ad-hoc signature). Like the swift-* repositories: a `Developer ID Application` certificate of the
# team found in the keychain; an explicit SIGN_IDENTITY wins. The key stays in the keychain. macOS only.
#
# env: SIGN_IDENTITY (optional), TEAM_ID, DEVELOPER_NAME (optional filters)
set -euo pipefail

if [[ -n "${SIGN_IDENTITY:-}" ]]; then
    echo "$SIGN_IDENTITY"
    exit 0
fi
command -v security >/dev/null 2>&1 || exit 0

want="Developer ID Application: ${DEVELOPER_NAME:-}"
suffix=""
[[ -n "${TEAM_ID:-}" ]] && suffix="(${TEAM_ID})"

security find-identity -v -p codesigning 2>/dev/null \
    | sed -n 's/.*"\(Developer ID Application: .*\)".*/\1/p' \
    | grep -F -- "$want" | grep -F -- "$suffix" | head -1 || true
