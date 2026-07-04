#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

test -f docs/videos/rustmix-wave-ble-remote-demo.mp4
test -f docs/images/rustmix-wave-ble-remote-demo.jpg

python3 - <<'PY'
from pathlib import Path

p = Path("README.md")
text = p.read_text()

start = "<!-- RUSTMIX_WAVE_BLE_DEMO_VIDEO_START -->"
end = "<!-- RUSTMIX_WAVE_BLE_DEMO_VIDEO_END -->"

section = f"""{start}

## Rustmix Remote BLE demo

The demo below shows the Rustmix Remote Wear OS app controlling Rustmix-Wave page turns on the Waveshare ESP32-S3 3.97-inch e-paper device over BLE GATT.

[![Rustmix Wave BLE Remote demo](docs/images/rustmix-wave-ble-remote-demo.jpg)](docs/videos/rustmix-wave-ble-remote-demo.mp4)

Demo coverage:

- Samsung Wear OS watch running Rustmix Remote
- Rustmix-Wave BLE firmware on Waveshare 3.97-inch e-paper
- Saved/direct BLE MAC connection
- Previous / Next page controls
- TXT reader page turning
- EPUB reader page turning

For this demo, use the BLE firmware release:

    v1.2.0-ble

Use the Wi-Fi firmware release for normal Wi-Fi transfer, weather, NTP/time sync, and network features:

    v1.2.0-wifi

{end}
"""

if start in text and end in text:
    before = text.split(start)[0].rstrip()
    after = text.split(end, 1)[1].lstrip()
    new_text = before + "\n\n" + section + "\n" + after
else:
    marker = "<!-- RUSTMIX_WAVE_BLE_RELEASE_NOTICE_END -->"
    if marker in text:
        before, after = text.split(marker, 1)
        new_text = before.rstrip() + "\n" + marker + "\n\n" + section + "\n" + after.lstrip()
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

echo "Updated README.md with BLE demo video section."
