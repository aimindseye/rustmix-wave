#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

python3 - <<'PY'
from pathlib import Path

p = Path("README.md")
text = p.read_text() if p.exists() else "# Rustmix Wave\n"

start = "<!-- RUSTMIX_WAVE_BLE_RELEASE_NOTICE_START -->"
end = "<!-- RUSTMIX_WAVE_BLE_RELEASE_NOTICE_END -->"

section = f"""{start}

## Rustmix Remote BLE release

Rustmix Wave now has a dedicated BLE Remote firmware release for using a Samsung Wear OS watch as a page-turning remote.

Use the BLE release when you want:

- Rustmix Remote Wear OS app support
- BLE GATT page-turning
- TXT reader Previous / Next page control
- EPUB reader Previous / Next page control
- Saved/direct BLE MAC fallback connection from the watch app
- A dedicated low-distraction remote-control workflow

The BLE release is intended for the Rustmix Remote watch app:

    Wear OS watch
        -> BLE GATT
        -> Rustmix-Wave
        -> TXT / EPUB reader page turn

### Which release should I install?

| Need | Install this release |
|---|---|
| Normal reader use with Wi-Fi transfer, weather, NTP/time sync, and network features | `v1.2.0-wifi` |
| Samsung Wear OS Rustmix Remote page turning for TXT and EPUB | `v1.2.0-ble` |

### BLE release capabilities

The `v1.2.0-ble` firmware supports:

- BLE advertising for Rustmix Remote
- Rustmix Remote BLE GATT service
- RRBP command packets
- Previous page command
- Next page command
- TXT reader page turning
- EPUB reader page turning

Accepted Rustmix Remote app behavior:

- Swipeable Remote / Device UI
- Saved BLE device address
- Connect Saved
- Scan / Fallback
- Disconnect
- Status text for connection and write activity

### Important BLE release limitation

The BLE release intentionally disables Wi-Fi/network features.

Disabled in `v1.2.0-ble`:

- Wi-Fi transfer portal
- NTP/time sync
- weather/network fetch
- Wi-Fi connection workflow

Use `v1.2.0-wifi` if you need those features.

### Why are Wi-Fi and BLE separate releases?

The Waveshare ESP32-S3 3.97-inch e-paper device uses one ESP32-S3 radio subsystem for Wi-Fi and Bluetooth LE. Rustmix Wave also has several memory-sensitive and timing-sensitive subsystems:

- e-paper refresh coordination
- SD card access
- TXT/EPUB reader cache workers
- audio and voice notes
- weather/network tasks
- Wi-Fi transfer web portal
- BLE GATT command service

For the accepted Rustmix Remote path, the BLE build gives BLE ownership of the modem and skips Wi-Fi/network services. This keeps watch page turning reliable while preserving a separate Wi-Fi build for normal daily use.

### BLE firmware build

    cd /home/mindseye73/Documents/projects/rustmix-wave

    export PATH="$HOME/.cargo/bin:$PATH"
    export RUSTFLAGS="${{RUSTFLAGS:-}} --cfg esp_idf_version_least_5_5_0"

    cargo +esp build \\
      --release \\
      --target xtensa-esp32s3-espidf \\
      --features rustmix-remote-ble

### BLE firmware flash

    espflash flash --chip esp32s3 --monitor \\
      target/xtensa-esp32s3-espidf/release/waveshare-epd397-rust-app

Expected BLE logs:

    rustmix-wave=rustmix-remote-gap event=AdvertisingStarted(Success)
    rustmix-wave=rustmix-remote-gatts event=PeerConnected
    rustmix-wave=rustmix-remote-gatts event=Write
    rustmix-wave=rustmix-remote-command status=enqueued
    rustmix-wave=rustmix-remote-event event=page-next route=reader-page
    rustmix-wave=rustmix-remote-event event=page-previous route=reader-page

### Rustmix Remote watch app

Rustmix Remote v1.0.0 is the validated Wear OS companion app:

    https://github.com/aimindseye/rustmix-watch-remote/releases/tag/v1.0.0

{end}
"""

if start in text and end in text:
    before = text.split(start)[0].rstrip()
    after = text.split(end, 1)[1].lstrip()
    new_text = before + "\n\n" + section + "\n" + after
else:
    lines = text.splitlines()
    insert_at = None

    for i, line in enumerate(lines):
        if line.startswith("## "):
            insert_at = i
            break

    if insert_at is None:
        new_text = text.rstrip() + "\n\n" + section + "\n"
    else:
        before = "\n".join(lines[:insert_at]).rstrip()
        after = "\n".join(lines[insert_at:]).lstrip()
        new_text = before + "\n\n" + section + "\n" + after + "\n"

p.write_text(new_text)
PY

echo "Updated README.md BLE release notice."
