#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="1.2.0"
WIFI_TAG="v1.2.0-wifi"
BLE_TAG="v1.2.0-ble"

mkdir -p docs/releases docs/rustmix-remote scripts dist/releases

python3 - <<'PY'
from pathlib import Path
import re

cargo = Path("Cargo.toml")
if cargo.exists():
    s = cargo.read_text()
    s2 = re.sub(
        r'(?m)^version\s*=\s*"[^"]+"',
        'version = "1.2.0"',
        s,
        count=1,
    )
    cargo.write_text(s2)
PY

python3 - <<'PY'
from pathlib import Path

p = Path("README.md")
existing = p.read_text() if p.exists() else "# Rustmix Wave\n"

start = "<!-- RUSTMIX_WAVE_DUAL_RELEASE_V1_2_START -->"
end = "<!-- RUSTMIX_WAVE_DUAL_RELEASE_V1_2_END -->"

section = f"""{start}

## Rustmix Wave release variants

Rustmix Wave now publishes two separate firmware release variants for the Waveshare ESP32-S3 3.97-inch e-paper device:

| Release | Tag | Best for | Wi-Fi | BLE Remote |
|---|---|---|---|---|
| Wi-Fi build | `v1.2.0-wifi` | Normal daily use, Wi-Fi transfer, weather, NTP/time sync, network features | Enabled | Disabled |
| BLE Remote build | `v1.2.0-ble` | Samsung Wear OS Rustmix Remote page turning for TXT and EPUB readers | Disabled intentionally | Enabled |

### Why two releases?

The Waveshare 3.97-inch e-paper board uses an ESP32-S3. ESP32-S3 provides both Wi-Fi and Bluetooth LE, but they share the same 2.4 GHz RF subsystem. ESP-IDF supports coexistence, but this firmware has several heavy users of memory, radio, storage, and display refresh:

- Wi-Fi transfer web portal
- weather fetch
- NTP/time sync
- SD card access
- audio/voice notes
- TXT/EPUB reader cache workers
- e-paper refresh coordination
- BLE GATT command service for Rustmix Remote

For the accepted Rustmix Remote validation, the BLE feature build gives BLE ownership of the modem and intentionally skips Wi-Fi/network services. This keeps page turning reliable and keeps the BLE callback safe: BLE parses and enqueues commands only, while the main loop owns reader state mutation.

### Waveshare 3.97-inch e-paper limitations

The Waveshare ESP32-S3 3.97-inch e-paper device is powerful for an embedded reader, but the firmware must respect these practical limits:

- E-paper refresh is slow compared with LCD/OLED displays.
- Full refreshes are expensive; partial refreshes must be coordinated.
- Internal RAM is limited; PSRAM helps but does not remove all task-stack and allocation constraints.
- Wi-Fi and BLE share the ESP32-S3 radio subsystem.
- SD card, reader cache, network tasks, and display refresh can compete for time and memory.
- BLE callbacks must not mutate UI or reader state directly.

### Wi-Fi release

Use the Wi-Fi release for normal Rustmix-Wave behavior:

- TXT reader
- EPUB reader
- Indic EPUB support
- voice notes
- calendar
- dictionary
- games
- Wi-Fi transfer
- weather
- time/NTP/network features

Flash the Wi-Fi release ELF with:

```
espflash flash --chip esp32s3 --monitor rustmix-wave-v1.2.0-wifi.elf
```

### BLE Remote release

Use the BLE release when using the Samsung Wear OS Rustmix Remote app as a page-turning remote.

Accepted BLE Remote behavior:

- Rustmix Remote connects through BLE GATT.
- Saved/direct BLE MAC fallback works.
- TXT reader Previous/Next page turning works.
- EPUB reader Previous/Next page turning works.
- Wi-Fi is intentionally skipped in this build.

Flash the BLE release ELF with:

```
espflash flash --chip esp32s3 --monitor rustmix-wave-v1.2.0-ble.elf
```

Expected BLE logs:

```
rustmix-wave=rustmix-remote-gap event=AdvertisingStarted(Success)
rustmix-wave=rustmix-remote-gatts event=PeerConnected
rustmix-wave=rustmix-remote-gatts event=Write
rustmix-wave=rustmix-remote-command status=enqueued
rustmix-wave=rustmix-remote-event event=page-next route=reader-page
rustmix-wave=rustmix-remote-event event=page-previous route=reader-page
```

{end}
"""

if start in existing and end in existing:
    before = existing.split(start)[0].rstrip()
    after = existing.split(end, 1)[1].lstrip()
    new_text = before + "\n\n" + section + "\n" + after
else:
    new_text = existing.rstrip() + "\n\n" + section + "\n"

p.write_text(new_text)
PY

cat > docs/architecture.md <<'EOF'
# Rustmix Wave Architecture

Rustmix Wave is an embedded Rust application for the Waveshare ESP32-S3 3.97-inch e-paper device.

## Hardware target

- Board class: Waveshare ESP32-S3 3.97-inch e-paper device
- Display: 800 x 480 e-paper panel
- MCU: ESP32-S3
- Storage: SD card
- Memory: internal RAM plus PSRAM
- Wireless: Wi-Fi and Bluetooth LE through ESP32-S3 radio subsystem

## Main firmware architecture

```text
Boot / board init
        |
        v
SD card mount and config load
        |
        v
Services
  - display
  - buttons / power key
  - RTC / alarms
  - audio / voice notes
  - storage browser
  - reader state
  - optional Wi-Fi services
  - optional BLE Remote service
        |
        v
Main UI router
        |
        v
Apps and reader
  - TXT reader
  - EPUB reader
  - calendar
  - voice notes
  - dictionary
  - games
  - settings
```

## Release variants

Rustmix Wave publishes two firmware variants for the same hardware.

### Wi-Fi build

The Wi-Fi build is the normal daily-use firmware.

```text
cargo +esp build --release --target xtensa-esp32s3-espidf
```

Enabled behavior:

- Wi-Fi config
- Wi-Fi transfer
- weather/network fetch
- NTP/time sync
- normal reader/apps/settings behavior

BLE Remote is not enabled in this build.

### BLE Remote build

The BLE Remote build enables the Rustmix Remote BLE GATT service.

```text
cargo +esp build --release --target xtensa-esp32s3-espidf --features rustmix-remote-ble
```

Enabled behavior:

- BLE GATT advertising
- Rustmix Remote service UUID
- command characteristic
- RRBP packet parsing
- command queue
- TXT reader page turning
- EPUB reader page turning

Wi-Fi/network services are intentionally skipped in this build.

## Why Wi-Fi and BLE are separate

The ESP32-S3 supports Wi-Fi and Bluetooth LE, but both share the same 2.4 GHz RF subsystem. ESP-IDF uses coexistence/time-sharing to coordinate radio access. Rustmix Wave also has substantial memory and timing pressure from e-paper refresh, SD card access, EPUB/TXT pagination, audio, weather/network workers, and UI routing.

For the accepted Rustmix Remote BLE path, the BLE feature build gives BLE ownership of the modem and disables Wi-Fi/network features. This produces a reliable page-turning firmware for the Wear OS remote while preserving the normal Wi-Fi firmware as a separate release.

## Rustmix Remote BLE path

```text
Samsung Wear OS watch
        |
        | BLE GATT write
        v
Rustmix-Wave BLE GATT service
        |
        | RRBP parse/enqueue
        v
Main loop drains command queue
        |
        | reader event
        v
TXT / EPUB page navigation
```

## RRBP command packet

RRBP packets are 6 bytes:

```text
byte 0: version
byte 1: sequence
byte 2: command
byte 3: flags
byte 4: parameter
byte 5: reserved
```

Accepted command examples:

```text
01 00 01 00 00 00 = page next
01 02 02 00 00 00 = page previous
```

## BLE callback safety

The BLE callback must not mutate reader state or UI state directly.

Accepted boundary:

```text
BLE callback: parse and enqueue only
main loop: drain queue and mutate reader/UI state
```

This is required to keep page turning safe and deterministic.

## Documentation references

Espressif documents ESP32-S3 RF coexistence as a shared 2.4 GHz RF subsystem managed with time-division multiplexing. See:

- https://docs.espressif.com/projects/esp-idf/en/stable/esp32s3/api-guides/coexist.html
- https://docs.espressif.com/projects/esp-faq/en/latest/software-framework/coexistence.html
EOF

cat > docs/rustmix-remote/ble-feature-build-boundary.md <<'EOF'
# Rustmix Remote BLE Feature Build Boundary

The `rustmix-remote-ble` build is intentionally separate from the normal Wi-Fi build.

## Accepted BLE behavior

Validated on hardware:

- Samsung Wear OS Rustmix Remote app connects to Rustmix-Wave over BLE GATT.
- TXT reader Previous/Next page turning works.
- EPUB reader Previous/Next page turning works.
- RRBP packets are written to the command characteristic.
- Commands are queued and drained by the main loop.

## Why Wi-Fi is disabled in this build

The Waveshare 3.97-inch e-paper device uses ESP32-S3. Wi-Fi and Bluetooth LE share the ESP32-S3 RF subsystem. Rustmix-Wave also has e-paper refresh timing, SD card access, reader cache workers, and optional network/audio tasks.

The accepted BLE build gives BLE ownership of the modem to keep watch page turning reliable. Therefore, these features are intentionally skipped in the BLE build:

- Wi-Fi connection
- Wi-Fi transfer portal
- NTP/time sync
- weather/network fetch

Use the Wi-Fi release for those features.

## Build command

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export RUSTFLAGS="${RUSTFLAGS:-} --cfg esp_idf_version_least_5_5_0"

cargo +esp build \
  --release \
  --target xtensa-esp32s3-espidf \
  --features rustmix-remote-ble
```

## Flash command

```bash
espflash flash --chip esp32s3 --monitor \
  target/xtensa-esp32s3-espidf/release/waveshare-epd397-rust-app
```
EOF

cat > docs/releases/v1.2.0-wifi.md <<'EOF'
# Rustmix Wave v1.2.0 Wi-Fi Release

This is the normal daily-use Rustmix-Wave firmware release for the Waveshare ESP32-S3 3.97-inch e-paper device.

## Use this release for

- TXT reader
- EPUB reader
- Indic EPUB support
- voice notes
- calendar
- dictionary
- games
- settings
- Wi-Fi transfer
- weather/network fetch
- NTP/time sync

## Not included

Rustmix Remote BLE page turning is not enabled in this release.

Use `v1.2.0-ble` for the Samsung Wear OS Rustmix Remote page-turning build.

## Flash

```bash
espflash flash --chip esp32s3 --monitor rustmix-wave-v1.2.0-wifi.elf
```
EOF

cat > docs/releases/v1.2.0-ble.md <<'EOF'
# Rustmix Wave v1.2.0 BLE Remote Release

This is the Rustmix Remote BLE firmware release for the Waveshare ESP32-S3 3.97-inch e-paper device.

## Use this release for

- Samsung Wear OS Rustmix Remote page turning
- TXT reader Previous/Next
- EPUB reader Previous/Next

## Accepted behavior

- BLE advertising starts successfully.
- Rustmix Remote connects over BLE GATT.
- RRBP command writes are received.
- Commands are queued and handled by the main loop.
- TXT reader page turning works.
- EPUB reader page turning works.

## Important limitation

Wi-Fi is intentionally disabled in this release because the BLE feature build owns the ESP32-S3 modem.

Disabled in this release:

- Wi-Fi transfer
- NTP/time sync
- weather/network fetch

Use `v1.2.0-wifi` for normal Wi-Fi-enabled firmware.

## Flash

```bash
espflash flash --chip esp32s3 --monitor rustmix-wave-v1.2.0-ble.elf
```
EOF

cat > scripts/build_release_wifi_v1_2_0.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

mkdir -p dist/releases

export PATH="$HOME/.cargo/bin:$PATH"

if [ -f sdkconfig.defaults.before-rustmix-remote-ble-r1 ]; then
  cp sdkconfig.defaults.before-rustmix-remote-ble-r1 sdkconfig.defaults
fi

rm -rf target/xtensa-esp32s3-espidf

cargo +esp build \
  --release \
  --target xtensa-esp32s3-espidf

ELF="target/xtensa-esp32s3-espidf/release/waveshare-epd397-rust-app"
OUT="dist/releases/rustmix-wave-v1.2.0-wifi.elf"

test -f "$ELF"
cp "$ELF" "$OUT"

sha256sum "$OUT" > "$OUT.sha256"

echo "Created $OUT"
cat "$OUT.sha256"
EOF

cat > scripts/build_release_ble_v1_2_0.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

mkdir -p dist/releases

export PATH="$HOME/.cargo/bin:$PATH"
export RUSTFLAGS="${RUSTFLAGS:-} --cfg esp_idf_version_least_5_5_0"

SDK_BACKUP="$(mktemp)"
cp sdkconfig.defaults "$SDK_BACKUP"

restore_sdkconfig() {
  cp "$SDK_BACKUP" sdkconfig.defaults
  rm -f "$SDK_BACKUP"
}
trap restore_sdkconfig EXIT

if [ -f sdkconfig.defaults.before-rustmix-remote-ble-r1 ]; then
  cp sdkconfig.defaults.before-rustmix-remote-ble-r1 sdkconfig.defaults
fi

if [ -f sdkconfig.defaults.rustmix-remote-ble ]; then
  cat sdkconfig.defaults.rustmix-remote-ble >> sdkconfig.defaults
fi

rm -rf target/xtensa-esp32s3-espidf

cargo +esp build \
  --release \
  --target xtensa-esp32s3-espidf \
  --features rustmix-remote-ble

ELF="target/xtensa-esp32s3-espidf/release/waveshare-epd397-rust-app"
OUT="dist/releases/rustmix-wave-v1.2.0-ble.elf"

test -f "$ELF"
cp "$ELF" "$OUT"

sha256sum "$OUT" > "$OUT.sha256"

echo "Created $OUT"
cat "$OUT.sha256"
EOF

cat > scripts/release_v1_2_0_wifi.sh <<'EOF'
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
EOF

cat > scripts/release_v1_2_0_ble.sh <<'EOF'
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
EOF

chmod +x \
  scripts/build_release_wifi_v1_2_0.sh \
  scripts/build_release_ble_v1_2_0.sh \
  scripts/release_v1_2_0_wifi.sh \
  scripts/release_v1_2_0_ble.sh

echo "Prepared Rustmix Wave v1.2.0 dual-release docs and scripts."
