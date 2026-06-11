#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

REPO="$TMP/repo"
FAKEBIN="$TMP/fakebin"
CUSTOM_TARGET="$TMP/custom-target"
STALE_LOCAL="$REPO/target/xtensa-esp32s3-espidf/release/waveshare-epd397-rust-app"
HOST_ELF="$REPO/target/release/waveshare-epd397-rust-app"
ACTUAL_ELF="$CUSTOM_TARGET/xtensa-esp32s3-espidf/release/deps/waveshare_epd397_rust_app-authoritative"
mkdir -p "$REPO/scripts" "$REPO/.cargo" "$FAKEBIN" "$(dirname "$STALE_LOCAL")" "$(dirname "$HOST_ELF")" "$(dirname "$ACTUAL_ELF")"

cp "$ROOT/scripts/resolve-built-elf.sh" "$ROOT/scripts/flash.sh" "$ROOT/scripts/build-release-firmware.sh" "$ROOT/scripts/flash-release.sh" "$REPO/scripts/"
cp "$ROOT/.cargo/config.toml" "$REPO/.cargo/config.toml"
chmod +x "$REPO/scripts/"*.sh
cat > "$REPO/Cargo.toml" <<'TOML'
[package]
name = "waveshare-epd397-rust-app"
version = "1.1.0"
edition = "2021"
TOML
cat > "$REPO/scripts/validate.sh" <<'EOF_VALIDATE'
#!/usr/bin/env bash
set -euo pipefail
echo 'fake-validation=ok'
EOF_VALIDATE
chmod +x "$REPO/scripts/validate.sh"
printf 'STALE-LOCAL-ELF\n' > "$STALE_LOCAL"
printf 'HOST-NON-ESP-IDF-ELF\n' > "$HOST_ELF"
printf 'ACTUAL-XTENSA-ESP-IDF-ELF\n' > "$ACTUAL_ELF"

cat > "$FAKEBIN/cargo" <<EOF_CARGO
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" == "+esp" ]]; then shift; fi
case "\${1:-}" in
  build)
    shift
    args=" \$* "
    if [[ "\$args" != *" -Z build-std=std,panic_abort "* ]]; then
      echo 'fake-cargo-build=failed error=missing-build-std' >&2
      exit 3
    fi
    if [[ "\$args" != *" --target xtensa-esp32s3-espidf "* ]]; then
      echo 'fake-cargo-build=failed error=missing-explicit-xtensa-target' >&2
      exit 3
    fi
    echo 'fake-cargo-build=ok target=xtensa-esp32s3-espidf build-std=std,panic_abort' >&2
    cat <<'JSON'
{"reason":"compiler-artifact","package_id":"path+file:///fixture#waveshare-epd397-rust-app@1.1.0","manifest_path":"/fixture/Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"waveshare-epd397-rust-app","src_path":"/fixture/src/main.rs","edition":"2021","doc":true,"doctest":false,"test":true},"profile":{"opt_level":"s","debuginfo":0,"debug_assertions":false,"overflow_checks":false,"test":false},"features":[],"filenames":["$HOST_ELF"],"executable":"$HOST_ELF","fresh":true}
{"reason":"compiler-artifact","package_id":"path+file:///fixture#waveshare-epd397-rust-app@1.1.0","manifest_path":"/fixture/Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"waveshare-epd397-rust-app","src_path":"/fixture/src/main.rs","edition":"2021","doc":true,"doctest":false,"test":true},"profile":{"opt_level":"s","debuginfo":0,"debug_assertions":false,"overflow_checks":false,"test":false},"features":[],"filenames":["$ACTUAL_ELF"],"executable":"$ACTUAL_ELF","fresh":true}
{"reason":"build-finished","success":true}
JSON
    ;;
  *)
    echo "unexpected fake cargo args: \$*" >&2
    exit 2
    ;;
esac
EOF_CARGO
chmod +x "$FAKEBIN/cargo"

cat > "$FAKEBIN/espflash" <<EOF_FLASH
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >> '$TMP/espflash.args'
EOF_FLASH
chmod +x "$FAKEBIN/espflash"

(
  cd "$REPO"
  PATH="$FAKEBIN:$PATH" ./scripts/flash.sh monitor
)
grep -F -- "flash --chip esp32s3 --monitor $ACTUAL_ELF" "$TMP/espflash.args" >/dev/null
if grep -F -- "$STALE_LOCAL" "$TMP/espflash.args" >/dev/null; then
  echo 'flash-target-resolution-selftest=failed error=stale-local-target-selected' >&2
  exit 1
fi
if grep -F -- "$HOST_ELF" "$TMP/espflash.args" >/dev/null; then
  echo 'flash-target-resolution-selftest=failed error=host-target-selected' >&2
  exit 1
fi

(
  cd "$REPO"
  PATH="$FAKEBIN:$PATH" ./scripts/build-release-firmware.sh --skip-validate
)
cmp -s "$ACTUAL_ELF" "$REPO/dist/waveshare-epd397-rust-app-v1.1.0.elf"
if find "$REPO/dist" -name '*-flash.bin' -print -quit | grep -q .; then
  echo 'flash-target-resolution-selftest=failed error=unsafe-raw-bin-generated' >&2
  exit 1
fi

grep -F 'target = "xtensa-esp32s3-espidf"' "$REPO/.cargo/config.toml" >/dev/null
grep -F 'build-std = ["std", "panic_abort"]' "$REPO/.cargo/config.toml" >/dev/null

echo 'flash-target-resolution-selftest=ok explicit-target=xtensa-esp32s3-espidf build-std=std,panic_abort host-artifact=rejected'
