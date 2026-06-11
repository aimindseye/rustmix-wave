#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

FIXTURE="$TMP/fixture"
FAKEBIN="$TMP/fakebin"
mkdir -p "$FIXTURE/scripts" "$FAKEBIN"
cp "$ROOT/Cargo.toml" "$FIXTURE/Cargo.toml"
cp "$ROOT/scripts/build-release-firmware.sh" "$FIXTURE/scripts/build-release-firmware.sh"
cp "$ROOT/scripts/flash-release.sh" "$FIXTURE/scripts/flash-release.sh"
cp "$ROOT/scripts/resolve-built-elf.sh" "$FIXTURE/scripts/resolve-built-elf.sh"
chmod +x "$FIXTURE/scripts/"*.sh

cat > "$FAKEBIN/cargo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == '+esp' ]]; then shift; fi
case "${1:-}" in
  build)
    shift
    args=" $* "
    if [[ "$args" != *" --target xtensa-esp32s3-espidf "* ]]; then
      echo 'fake cargo missing explicit embedded target' >&2
      exit 3
    fi
    mkdir -p target/xtensa-esp32s3-espidf/release
    ELF="$PWD/target/xtensa-esp32s3-espidf/release/waveshare-epd397-rust-app"
    printf 'dummy-release-elf\n' > "$ELF"
    printf '{"reason":"compiler-artifact","target":{"kind":["bin"],"name":"waveshare-epd397-rust-app"},"executable":"%s"}\n' "$ELF"
    printf '{"reason":"build-finished","success":true}\n'
    ;;
  *)
    echo "fake cargo unexpected arguments: $*" >&2
    exit 1
    ;;
esac
SH
chmod +x "$FAKEBIN/cargo"

cat > "$FAKEBIN/espflash" <<SH
#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "\$*" > "$TMP/espflash-args.txt"
SH
chmod +x "$FAKEBIN/espflash"

(
  cd "$FIXTURE"
  PATH="$FAKEBIN:$PATH" ./scripts/build-release-firmware.sh --skip-validate
)

VERSION="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$FIXTURE/Cargo.toml" | head -n 1)"
[[ -n "$VERSION" ]] || {
  echo 'release-flash-workflow-selftest=failed reason=version-unavailable' >&2
  exit 1
}
PREFIX="$FIXTURE/dist/waveshare-epd397-rust-app-v${VERSION}"
for required in \
  "$PREFIX.elf" \
  "$PREFIX-flash-release.sh" \
  "$PREFIX-firmware-release.sha256" \
  "$PREFIX-FLASHING.txt" \
  "$PREFIX-firmware-release.zip"
do
  [[ -f "$required" ]] || {
    echo "release-flash-workflow-selftest=failed missing=$required" >&2
    exit 1
  }
done

if find "$FIXTURE/dist" -maxdepth 1 -type f -name '*-flash.bin' | grep -q .; then
  echo 'release-flash-workflow-selftest=failed reason=legacy-bin-generated' >&2
  exit 1
fi
if unzip -Z1 "$PREFIX-firmware-release.zip" | grep -q -- '-flash.bin$'; then
  echo 'release-flash-workflow-selftest=failed reason=legacy-bin-packaged' >&2
  exit 1
fi

grep -Fq "espflash flash --chip esp32s3 --monitor waveshare-epd397-rust-app-v${VERSION}.elf" "$PREFIX-FLASHING.txt"
grep -Fq 'Do not flash this release with espflash write-bin.' "$PREFIX-FLASHING.txt"

PATH="$FAKEBIN:$PATH" "$FIXTURE/scripts/flash-release.sh" "$PREFIX.elf"
[[ "$(cat "$TMP/espflash-args.txt")" == "flash --chip esp32s3 --monitor $PREFIX.elf" ]]

PATH="$FAKEBIN:$PATH" "$FIXTURE/scripts/flash-release.sh" --port /dev/cu.TEST "$PREFIX.elf"
[[ "$(cat "$TMP/espflash-args.txt")" == "flash --chip esp32s3 --port /dev/cu.TEST --monitor $PREFIX.elf" ]]

echo 'release-flash-workflow-selftest=ok'
