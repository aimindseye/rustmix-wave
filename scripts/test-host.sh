#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

HOST_TRIPLE="$(rustc +stable -vV | sed -n 's/^host: //p')"
if [[ -z "$HOST_TRIPLE" ]]; then
  echo 'host-test-native-target-isolation=failed error=unable-to-determine-stable-rust-host-target' >&2
  exit 1
fi

printf 'host-test-native-target=%s\n' "$HOST_TRIPLE"
cargo +stable test --target "$HOST_TRIPLE" --lib
echo 'host-test-native-target-isolation=ok'
