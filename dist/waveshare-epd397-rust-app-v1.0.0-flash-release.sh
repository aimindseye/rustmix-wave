#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat >&2 <<TXT
usage: $0 [--port PORT] [RELEASE_ELF]

Flash an existing Rustmix Wave release ELF through the ELF-aware espflash path.
When RELEASE_ELF is omitted inside the source tree, the script selects the ELF
matching the Cargo package version under dist/.
TXT
}

PORT=""
ELF=""
while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --port)
      if [[ "$#" -lt 2 ]]; then
        usage
        exit 1
      fi
      PORT="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      if [[ -n "$ELF" ]]; then
        usage
        exit 1
      fi
      ELF="$1"
      shift
      ;;
  esac
done

if ! command -v espflash >/dev/null 2>&1; then
  echo 'release-flash=failed error=espflash-not-found' >&2
  echo 'Install espflash before flashing a release ELF.' >&2
  exit 1
fi

if [[ -z "$ELF" ]]; then
  if [[ -f "$ROOT/Cargo.toml" ]]; then
    VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/Cargo.toml" | head -n1)"
    ELF="$ROOT/dist/waveshare-epd397-rust-app-v${VERSION}.elf"
  else
    shopt -s nullglob
    candidates=("$SCRIPT_DIR"/*.elf)
    shopt -u nullglob
    if [[ "${#candidates[@]}" -eq 1 ]]; then
      ELF="${candidates[0]}"
    else
      echo 'release-flash=failed error=release-elf-required' >&2
      usage
      exit 1
    fi
  fi
fi

if [[ ! -f "$ELF" ]]; then
  echo "release-flash=failed error=missing-release-elf path=$ELF" >&2
  exit 1
fi

if [[ -n "$PORT" ]]; then
  exec espflash flash --chip esp32s3 --port "$PORT" --monitor "$ELF"
else
  exec espflash flash --chip esp32s3 --monitor "$ELF"
fi
