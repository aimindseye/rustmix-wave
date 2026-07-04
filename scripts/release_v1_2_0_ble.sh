#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

TAG="v1.2.0-ble"
TITLE="Rustmix Wave v1.2.0 BLE Remote"
ELF="dist/releases/rustmix-wave-v1.2.0-ble.elf"
SUM="$ELF.sha256"

./scripts/build_release_ble_v1_2_0.sh

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
    --notes-file docs/releases/v1.2.0-ble.md
fi
