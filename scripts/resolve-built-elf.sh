#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PACKAGE_NAME="waveshare-epd397-rust-app"
TARGET_TRIPLE="${RUSTMIX_WAVE_TARGET:-xtensa-esp32s3-espidf}"
BUILD_STD="std,panic_abort"

if ! command -v cargo >/dev/null 2>&1; then
  echo 'resolve-built-elf=failed error=cargo-not-found' >&2
  exit 1
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo 'resolve-built-elf=failed error=python3-not-found' >&2
  exit 1
fi

JSON_LOG="$(mktemp)"
trap 'rm -f "$JSON_LOG"' EXIT

# Xtensa ESP-IDF does not ship a precompiled Rust standard library. Build it
# from rust-src explicitly while requesting Cargo's authoritative JSON artifact.
cargo +esp build \
  -Z "build-std=${BUILD_STD}" \
  --release \
  --target "$TARGET_TRIPLE" \
  --bin "$PACKAGE_NAME" \
  --message-format=json-render-diagnostics \
  > "$JSON_LOG"

python3 - "$JSON_LOG" "$PACKAGE_NAME" "$TARGET_TRIPLE" <<'PY_RESOLVE'
import json
import os
import sys

log_path, package_name, target_triple = sys.argv[1:]
candidates = []
rejected_host = []
needle = os.sep + target_triple + os.sep
with open(log_path, "r", encoding="utf-8") as handle:
    for raw in handle:
        raw = raw.strip()
        if not raw:
            continue
        try:
            item = json.loads(raw)
        except json.JSONDecodeError:
            print(raw, file=sys.stderr)
            continue
        if item.get("reason") == "compiler-message":
            rendered = item.get("message", {}).get("rendered")
            if rendered:
                print(rendered, end="", file=sys.stderr)
            continue
        if item.get("reason") != "compiler-artifact":
            continue
        target = item.get("target", {})
        kinds = target.get("kind", [])
        executable = item.get("executable")
        if target.get("name") != package_name or "bin" not in kinds:
            continue
        if not isinstance(executable, str) or not executable:
            continue
        if needle not in os.path.normpath(executable):
            rejected_host.append(executable)
            continue
        candidates.append(executable)

for candidate in reversed(candidates):
    if os.path.isfile(candidate):
        print(candidate)
        raise SystemExit(0)

if candidates:
    print(
        "resolve-built-elf=failed error=reported-embedded-executable-missing "
        f"target={target_triple} paths={candidates!r}",
        file=sys.stderr,
    )
elif rejected_host:
    print(
        "resolve-built-elf=failed error=host-artifact-rejected "
        f"target={target_triple} rejected={rejected_host!r}",
        file=sys.stderr,
    )
else:
    print(
        "resolve-built-elf=failed error=cargo-did-not-report-embedded-bin-executable "
        f"package={package_name} target={target_triple}",
        file=sys.stderr,
    )
raise SystemExit(1)
PY_RESOLVE
