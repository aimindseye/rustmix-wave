#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

TAG="v1.2.0-wifi"
TITLE="Rustmix Wave v1.2.0 Wi-Fi"
ELF="dist/releases/rustmix-wave-v1.2.0-wifi.elf"
SUM="$ELF.sha256"

./scripts/build_release_wifi_v1_2_0.sh

git diff --check

if ! git rev-parse "$TAG" >/dev/null 2>&1; then
  git tag -a "$TAG" -m "$TITLE"
fi

git push origin main
git push origin "$TAG"

if gh release view "$TAG" >/dev/null 2>&1; then
  gh release upload "$TAG" "$ELF" "$SUM" --clobber
else
  gh release create "$TAG" \
    "$ELF" \
    "$SUM" \
    --title "$TITLE" \
    --notes-file docs/releases/v1.2.0-wifi.md
fi
