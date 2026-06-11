#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

./scripts/validate.sh

cargo +esp build \
  -Z build-std=std,panic_abort \
  --release \
  --target xtensa-esp32s3-espidf
