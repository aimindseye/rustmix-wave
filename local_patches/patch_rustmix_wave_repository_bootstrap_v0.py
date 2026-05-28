#!/usr/bin/env python3
from pathlib import Path
import subprocess
import sys

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()

if not (ROOT / ".git").exists():
    raise SystemExit(f"not a git repository: {ROOT}")


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def replace_marked_block(src: str, begin: str, end: str, block: str) -> str:
    if begin in src and end in src:
        a = src.find(begin)
        b = src.find(end, a)
        line_end = src.find("\n", b)
        if line_end < 0:
            line_end = len(src)
        else:
            line_end += 1
        return src[:a] + block + "\n" + src[line_end:]
    return src.rstrip() + "\n\n" + block + "\n"


def git_remote_url(name: str) -> str:
    try:
        return subprocess.check_output(
            ["git", "remote", "get-url", name],
            cwd=ROOT,
            text=True,
            stderr=subprocess.STDOUT,
        ).strip()
    except subprocess.CalledProcessError:
        return ""


origin_url = git_remote_url("origin")
upstream_url = git_remote_url("rustmix-x4-upstream")

if "rustmix-wave" not in origin_url:
    raise SystemExit(f"origin does not point to rustmix-wave: {origin_url!r}")

if "rustmix-x4-firmware" not in upstream_url:
    raise SystemExit(f"rustmix-x4-upstream does not point to rustmix-x4-firmware: {upstream_url!r}")


# ---------------------------------------------------------------------------
# Skeleton folders.
# ---------------------------------------------------------------------------
for path in [
    ROOT / "hal-waveshare-epd397",
    ROOT / "hal-waveshare-epd397" / "src",
    ROOT / "target-waveshare-epd397",
    ROOT / "target-waveshare-epd397" / "src",
    ROOT / "docs" / "rustmix-wave",
    ROOT / "scripts",
]:
    path.mkdir(parents=True, exist_ok=True)


# ---------------------------------------------------------------------------
# HAL skeleton. No display backend is ported yet in this bootstrap.
# ---------------------------------------------------------------------------
write_text(
    ROOT / "hal-waveshare-epd397" / "README.md",
    """# hal-waveshare-epd397

Hardware abstraction skeleton for the Waveshare ESP32-S3 e-Paper 3.97 target.

This crate/folder is intentionally a skeleton in Repository Bootstrap v0.

Planned responsibilities:

- Display backend for Waveshare 3.97 e-paper.
- Rotary dial and safe button input mapping.
- Audio codec / microphone / speaker bring-up.
- Power, RTC, sensors, storage, Wi-Fi, and BLE board services.
- Board-specific pin ownership and hardware initialization.

Display backend source of truth for the next slice:

- Accepted Focus Hub free-function display backend.
- Accepted DisplayBackendAdapter.
- Accepted ShellDisplayBridge portrait mapping.
- Accepted Waveshare display pin map:
  - EPD_SCLK GPIO11
  - EPD_MOSI GPIO12
  - EPD_CS GPIO10
  - EPD_DC GPIO9
  - EPD_RST GPIO46
  - EPD_BUSY GPIO3

Do not port display code in this bootstrap slice.
""",
)

write_text(
    ROOT / "hal-waveshare-epd397" / "src" / "lib.rs",
    """//! Waveshare ESP32-S3 e-Paper 3.97 HAL skeleton.
//!
//! Repository Bootstrap v0 only establishes the target boundary.
//! Display, input, audio, storage, and power implementations are intentionally
//! added in later slices.
//!
//! Accepted future display source:
//! - Focus Hub free display backend.
//! - DisplayBackendAdapter.
//! - ShellDisplayBridge portrait mapping.
//!
//! Important pin ownership note:
//! - GPIO3 is EPD_BUSY and must not be reused for rotary/input.

pub mod board {
    pub const TARGET_NAME: &str = "waveshare-esp32-s3-epaper-3.97";
    pub const DISPLAY_WIDTH_NATIVE: usize = 800;
    pub const DISPLAY_HEIGHT_NATIVE: usize = 480;
    pub const DISPLAY_WIDTH_PORTRAIT: usize = 480;
    pub const DISPLAY_HEIGHT_PORTRAIT: usize = 800;
}

pub mod display {
    //! Placeholder for the accepted Focus Hub display backend import.
}

pub mod input {
    //! Placeholder for safe rotary input mapping.
    //! GPIO3 is reserved for EPD_BUSY and must not be used as input.
}

pub mod audio {
    //! Placeholder for future voice/audio codec bring-up.
}

pub mod storage {
    //! Placeholder for TF/SD storage adaptation.
}

pub mod power {
    //! Placeholder for PMU/battery services.
}

pub mod rtc {
    //! Placeholder for RTC/time services.
}

pub mod sensors {
    //! Placeholder for environment and IMU sensors.
}

pub mod wifi {
    //! Placeholder for Wi-Fi transfer and assistant connectivity.
}
""",
)


# ---------------------------------------------------------------------------
# Target skeleton. Not wired into workspace yet.
# ---------------------------------------------------------------------------
write_text(
    ROOT / "target-waveshare-epd397" / "README.md",
    """# target-waveshare-epd397

Target skeleton for Rustmix-Wave on the Waveshare ESP32-S3 e-Paper 3.97 board.

Repository Bootstrap v0 does not port display code yet and does not delete the
existing Rustmix X4 target/code.

Planned target flow:

1. Import accepted Focus Hub display backend into `hal-waveshare-epd397`.
2. Create a minimal Waveshare dashboard binary.
3. Port Rustmix reader/product model behind display/storage/input abstractions.
4. Add rotary-first navigation.
5. Add voice assistant states and audio bring-up.
""",
)

write_text(
    ROOT / "target-waveshare-epd397" / "src" / "main.rs",
    """//! Rustmix-Wave target skeleton.
//!
//! This file is intentionally not wired into the workspace in Repository
//! Bootstrap v0. The next slice will add a real Waveshare ESP32-S3 target binary
//! using the accepted Focus Hub display backend.

fn main() {
    println!("rustmix-wave target skeleton: Waveshare 3.97 display backend not ported yet");
}
""",
)


# ---------------------------------------------------------------------------
# Docs.
# ---------------------------------------------------------------------------
write_text(
    ROOT / "docs" / "rustmix-wave" / "bootstrap-v0.md",
    """# Rustmix-Wave Repository Bootstrap v0

## Status

This repository is now the clean product home for Rustmix-Wave.

Rustmix-Wave means:

- Rustmix product model.
- Waveshare ESP32-S3 e-Paper 3.97 hardware target.
- Rotary-first non-touch UI.
- Future voice assistant layer.
- Reuse of working Rustmix X4 reader/app code where possible.
- Reuse of accepted Focus Hub Waveshare display backend where appropriate.

## What this slice does

- Keeps Rustmix X4 code as the upstream reference.
- Adds `hal-waveshare-epd397/` skeleton.
- Adds `target-waveshare-epd397/` skeleton.
- Adds Rustmix-Wave docs.
- Adds validation script.

## What this slice intentionally does not do

- Does not port the display backend yet.
- Does not delete X4 code.
- Does not wire a new target into the workspace.
- Does not enable rotary input.
- Does not add voice/audio code.
""",
)

write_text(
    ROOT / "docs" / "rustmix-wave" / "architecture.md",
    """# Rustmix-Wave Architecture Direction

## Product base

Rustmix-Wave should reuse the Rustmix product model from the Xteink X4 firmware:

- Reader.
- Library/recent books.
- Progress/bookmarks/settings.
- Wi-Fi transfer model.
- Dictionary shards.
- Flashcards.
- Lua app structure.
- Custom fonts and prepared assets where portable.

## Hardware target

The new board target is Waveshare ESP32-S3 e-Paper 3.97.

This target should be isolated in:

- `hal-waveshare-epd397/`
- `target-waveshare-epd397/`

## Display/backend source

The display backend source for the first real Waveshare slice is the accepted
Focus Hub bring-up path:

- Free-function display backend.
- DisplayBackendAdapter.
- ShellDisplayBridge.
- Portrait 480x800 logical mapping over native 800x480 RAM.

The old Focus Hub `EpaperDisplay::new` wrapper should not be reused because it
had a constructor/return-path hang during bring-up.

## Rotary-first UI

The UI should be non-touch and focus-first:

- Rotary turn changes selected row.
- Press opens selected item.
- Hold-to-talk activates voice.
- Selected row should be visually obvious with a focus bar, border, or inverse pill.
- Avoid touch-style grids as the primary home navigation.

## Future voice layer

Voice should be a system layer:

- UI-only voice states first.
- Audio codec record/playback second.
- Network assistant request third.
- Assistant workflows later.
""",
)

write_text(
    ROOT / "docs" / "rustmix-wave" / "ui-direction.md",
    """# Rustmix-Wave UI Direction

Rustmix-Wave should feel like a Waveshare-native Rustmix device, not a direct
copy of the X4 UI.

## Home model

Use a vertical rotary-first menu:

- Reader
- Network
- Productivity
- Voice
- Tools
- System

Each selected row should update a detail panel with:

- Detail title.
- Short description.
- Current action hint.
- Voice/status strip.

## Footer

The footer should communicate physical controls:

- Rotate: Select
- Press: Open
- Hold: Talk

## E-paper rules

- Prefer strong black/white contrast.
- Avoid excessive full-screen redraws.
- Use partial redraw only after the full display pipeline is stable.
- Design around focus state, not touch targets.
""",
)

write_text(
    ROOT / "docs" / "rustmix-wave" / "voice-layer.md",
    """# Rustmix-Wave Voice Layer Direction

The Waveshare ESP32-S3 e-Paper 3.97 board is a better target for voice features
than the Xteink X4 because the product direction includes microphone/speaker and
assistant-style interaction.

## Voice interaction model

- Rotate: select menu item.
- Press: open menu item.
- Hold: push-to-talk voice capture.
- Release/press: stop capture and process.
- Display assistant response text on e-paper.
- Optionally play spoken response later.

## Recommended phases

1. Voice UI states only:
   - Idle
   - Listening
   - Processing
   - Reply ready
   - Offline

2. Audio codec bring-up:
   - speaker test
   - microphone capture
   - WAV record/play from SD

3. Network assistant:
   - send captured audio or text request over Wi-Fi
   - display response

4. Device actions:
   - open reader
   - show weather
   - set timer
   - summarize today
""",
)

write_text(
    ROOT / "docs" / "rustmix-wave" / "migration-plan.md",
    """# Rustmix-Wave Migration Plan

## Source repositories

- Product source base: Rustmix X4 firmware.
- Hardware bring-up source: Focus Hub Waveshare firmware experiments.

## Keep from Rustmix X4

- Reader/product model.
- Books/library state.
- Progress and bookmarks.
- Wi-Fi transfer.
- Dictionary shards.
- Flashcards.
- Lua app model.
- Fonts and asset conventions where portable.

## Replace for Waveshare

- X4 ESP32-C3 target setup.
- X4 e-paper driver and pin map.
- X4 input mapping.
- X4 power assumptions.
- X4 orientation assumptions.

## Import from Focus Hub bring-up later

- Accepted Waveshare display pin map.
- Free display backend.
- DisplayBackendAdapter.
- ShellDisplayBridge.
- Portrait mapping.
- Rotary-first UI experiments.

## Bootstrap rule

Do not delete the X4 code until the Rustmix-Wave target has its own display,
storage, reader, and UI path.
""",
)


# ---------------------------------------------------------------------------
# README section.
# ---------------------------------------------------------------------------
readme = ROOT / "README.md"
readme_src = readme.read_text() if readme.exists() else "# Rustmix-Wave\n"

readme_block = """<!-- BEGIN RUSTMIX_WAVE_REPOSITORY_BOOTSTRAP_V0 -->
## Rustmix-Wave

Rustmix-Wave is the Waveshare ESP32-S3 e-Paper 3.97 version of Rustmix.

This repository keeps the Rustmix X4 code as the upstream reference while adding
a new Waveshare target direction.

Repository Bootstrap v0 adds:

- `hal-waveshare-epd397/` skeleton.
- `target-waveshare-epd397/` skeleton.
- Rustmix-Wave architecture docs.
- Validation script for repository bootstrap.

Product direction:

- Reuse the Rustmix product model and reader/app logic where possible.
- Use the accepted Focus Hub Waveshare display/backend work as the hardware display source.
- Build a rotary-first UI for a non-touch e-paper device.
- Add a future voice assistant layer inspired by the Focus Hub / Durobo direction.

The first real hardware slice after this bootstrap should import the accepted
Waveshare 3.97 display backend. This bootstrap intentionally does not port
display code and does not delete the existing X4 code.
<!-- END RUSTMIX_WAVE_REPOSITORY_BOOTSTRAP_V0 -->"""

readme_src = replace_marked_block(
    readme_src,
    "<!-- BEGIN RUSTMIX_WAVE_REPOSITORY_BOOTSTRAP_V0 -->",
    "<!-- END RUSTMIX_WAVE_REPOSITORY_BOOTSTRAP_V0 -->",
    readme_block,
)
readme.write_text(readme_src)


# ---------------------------------------------------------------------------
# Validation script committed into repo.
# ---------------------------------------------------------------------------
write_text(
    ROOT / "scripts" / "validate_rustmix_wave_repository_bootstrap_v0.py",
    r'''#!/usr/bin/env python3
from pathlib import Path
import subprocess
import sys

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()

errors = []

def read(path: Path) -> str:
    try:
        return path.read_text()
    except FileNotFoundError:
        errors.append(f"missing file: {path.relative_to(ROOT)}")
        return ""

def remote_url(name: str) -> str:
    try:
        return subprocess.check_output(
            ["git", "remote", "get-url", name],
            cwd=ROOT,
            text=True,
            stderr=subprocess.STDOUT,
        ).strip()
    except subprocess.CalledProcessError:
        errors.append(f"missing git remote: {name}")
        return ""

for folder in [
    "hal-waveshare-epd397",
    "hal-waveshare-epd397/src",
    "target-waveshare-epd397",
    "target-waveshare-epd397/src",
    "docs/rustmix-wave",
    "scripts",
]:
    if not (ROOT / folder).is_dir():
        errors.append(f"missing folder: {folder}")

for file in [
    "hal-waveshare-epd397/README.md",
    "hal-waveshare-epd397/src/lib.rs",
    "target-waveshare-epd397/README.md",
    "target-waveshare-epd397/src/main.rs",
    "docs/rustmix-wave/bootstrap-v0.md",
    "docs/rustmix-wave/architecture.md",
    "docs/rustmix-wave/ui-direction.md",
    "docs/rustmix-wave/voice-layer.md",
    "docs/rustmix-wave/migration-plan.md",
    "README.md",
]:
    if not (ROOT / file).is_file():
        errors.append(f"missing file: {file}")

origin = remote_url("origin")
upstream = remote_url("rustmix-x4-upstream")

if "rustmix-wave" not in origin:
    errors.append(f"origin does not point to rustmix-wave: {origin}")

if "rustmix-x4-firmware" not in upstream:
    errors.append(f"rustmix-x4-upstream does not point to rustmix-x4-firmware: {upstream}")

readme = read(ROOT / "README.md")
docs = "\n".join(
    read(path)
    for path in [
        ROOT / "docs/rustmix-wave/bootstrap-v0.md",
        ROOT / "docs/rustmix-wave/architecture.md",
        ROOT / "docs/rustmix-wave/ui-direction.md",
        ROOT / "docs/rustmix-wave/voice-layer.md",
        ROOT / "docs/rustmix-wave/migration-plan.md",
    ]
)

combined = readme + "\n" + docs

required_text = [
    "Waveshare ESP32-S3 e-Paper 3.97",
    "Rustmix product model",
    "Focus Hub",
    "rotary-first",
    "voice",
    "hal-waveshare-epd397",
    "target-waveshare-epd397",
]

for text in required_text:
    if text.lower() not in combined.lower():
        errors.append(f"README/docs missing required text: {text}")

hal_src = read(ROOT / "hal-waveshare-epd397/src/lib.rs")
if "GPIO3 is EPD_BUSY" not in hal_src:
    errors.append("HAL skeleton must document GPIO3 as EPD_BUSY")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-repository-bootstrap-v0=ok")
''',
)

(ROOT / "scripts" / "validate_rustmix_wave_repository_bootstrap_v0.py").chmod(0o755)

print("rustmix-wave-repository-bootstrap-v0-applied=ok")
