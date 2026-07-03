#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RRBP="$ROOT/firmware/assistant-rs/src/rustmix_remote/rrbp.rs"

if [ ! -f "$RRBP" ]; then
  echo "Missing $RRBP"
  exit 1
fi

if ! command -v rustc >/dev/null 2>&1; then
  echo "ERROR: rustc not found. Run this inside the Rustmix-Wave Rust toolchain shell."
  exit 1
fi

TMP_BIN="/tmp/rustmix_remote_rrbp_tests"
rustc --edition=2021 --test "$RRBP" -o "$TMP_BIN"
"$TMP_BIN"

echo "Rustmix Remote RRBP parser scaffold validation: OK"
