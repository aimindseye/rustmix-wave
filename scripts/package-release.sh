#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

./scripts/validate.sh

VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)"
if [[ -z "$VERSION" ]]; then
  echo 'Unable to determine Cargo package version.' >&2
  exit 1
fi

mkdir -p dist
OUT="dist/waveshare-epd397-rust-app-v${VERSION}-github-ready.zip"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/waveshare-epd397-rust-app"
rsync -a ./ "$TMP/waveshare-epd397-rust-app/" \
  --exclude '.git/' \
  --exclude 'target/' \
  --exclude '.embuild/' \
  --exclude 'dist/' \
  --exclude '.DS_Store' \
  --exclude '__pycache__/' \
  --exclude '*.pyc' \
  --exclude '*.bak' \
  --exclude '*.orig' \
  --exclude '*.rej' \
  --exclude '*.zip' \
  --exclude '*.sha256' \
  --exclude 'waveshare-epd397-rust-*-repair-*/' \
  --exclude 'waveshare-epd397-rust-*-v*/'

rm -f "$OUT" "$OUT.sha256"
(
  cd "$TMP"
  zip -qr "$ROOT/$OUT" waveshare-epd397-rust-app
)
(
  cd dist
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$(basename "$OUT")" > "$(basename "$OUT").sha256"
  else
    sha256sum "$(basename "$OUT")" > "$(basename "$OUT").sha256"
  fi
)

echo "release-source-zip=$OUT"
echo "release-source-sha256=$OUT.sha256"
