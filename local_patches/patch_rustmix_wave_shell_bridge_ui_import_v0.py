#!/usr/bin/env python3
from pathlib import Path
import sys

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()

HAL = ROOT / "hal-waveshare-epd397" / "src" / "lib.rs"
TARGET_MAIN = ROOT / "target-waveshare-epd397" / "src" / "main.rs"
DOCS = ROOT / "docs" / "rustmix-wave"
README = ROOT / "README.md"
SCRIPTS = ROOT / "scripts"

for path in [HAL, TARGET_MAIN, DOCS, README, SCRIPTS]:
    if not path.exists():
        raise SystemExit(f"missing path: {path}")


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


def find_matching_brace(src: str, brace_pos: int) -> int:
    depth = 0
    for i in range(brace_pos, len(src)):
        if src[i] == "{":
            depth += 1
        elif src[i] == "}":
            depth -= 1
            if depth == 0:
                return i + 1
    raise SystemExit("could not find matching brace")


def replace_from_marker_through_loop(src: str, marker: str, replacement: str) -> str:
    start = src.find(marker)
    if start < 0:
        raise SystemExit(f"could not find marker in target main: {marker}")

    loop_idx = src.find("    loop {", start)
    if loop_idx < 0:
        raise SystemExit("could not find park loop after display smoke marker")

    brace = src.find("{", loop_idx)
    end = find_matching_brace(src, brace)

    while end < len(src) and src[end] in "\r\n":
        end += 1

    return src[:start] + replacement + src[end:]


# ---------------------------------------------------------------------------
# HAL: ensure ShellDisplayBridge has stroke_rect and add Rustmix-Wave UI module.
# ---------------------------------------------------------------------------
hal = HAL.read_text()

required_hal = [
    "pub struct DisplayBackendAdapter",
    "pub struct ShellDisplayBridge",
    "fn shell_logical_to_native",
    "SHELL_LOGICAL_WIDTH: usize = 480",
    "SHELL_LOGICAL_HEIGHT: usize = 800",
    "GPIO3 is EPD_BUSY",
]

missing = [item for item in required_hal if item not in hal]
if missing:
    raise SystemExit("missing accepted display bridge pieces: " + ", ".join(missing))

if "pub fn stroke_rect(" not in hal:
    needle = "        pub fn write_frame(&mut self, frame: &[u8]) -> Result<()> {"
    if needle not in hal:
        raise SystemExit("could not find ShellDisplayBridge::write_frame insertion point")

    stroke_rect = r'''        pub fn stroke_rect(
            &mut self,
            x: u32,
            y: u32,
            w: u32,
            h: u32,
            stroke: u32,
            color: BinaryColor,
        ) {
            if w == 0 || h == 0 || stroke == 0 {
                return;
            }

            self.fill_rect(x, y, w, stroke, color);
            self.fill_rect(x, y.saturating_add(h.saturating_sub(stroke)), w, stroke, color);
            self.fill_rect(x, y, stroke, h, color);
            self.fill_rect(x.saturating_add(w.saturating_sub(stroke)), y, stroke, h, color);
        }

'''
    hal = hal.replace(needle, stroke_rect + needle, 1)

begin = "// BEGIN RUSTMIX_WAVE_SHELL_BRIDGE_UI_IMPORT_V0"
end = "// END RUSTMIX_WAVE_SHELL_BRIDGE_UI_IMPORT_V0"
hal = replace_marked_block(hal, begin, end, r'''// BEGIN RUSTMIX_WAVE_SHELL_BRIDGE_UI_IMPORT_V0
pub mod ui {
    use anyhow::Result;
    use embedded_graphics::pixelcolor::BinaryColor;
    use esp_idf_hal::delay::FreeRtos;

    use crate::{display::ShellDisplayBridge, raw_marker};

    pub trait RustmixWaveHomeDisplaySurface {
        fn clear(&mut self, color: BinaryColor);
        fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: BinaryColor);
        fn stroke_rect(
            &mut self,
            x: u32,
            y: u32,
            w: u32,
            h: u32,
            stroke: u32,
            color: BinaryColor,
        );
        fn flush(&mut self) -> Result<()>;
    }

    impl<'d> RustmixWaveHomeDisplaySurface for ShellDisplayBridge<'d> {
        fn clear(&mut self, color: BinaryColor) {
            self.clear_fb(color);
        }

        fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: BinaryColor) {
            self.fill_rect(x, y, w, h, color);
        }

        fn stroke_rect(
            &mut self,
            x: u32,
            y: u32,
            w: u32,
            h: u32,
            stroke: u32,
            color: BinaryColor,
        ) {
            self.stroke_rect(x, y, w, h, stroke, color);
        }

        fn flush(&mut self) -> Result<()> {
            self.flush()
        }
    }

    pub struct RustmixWaveHomeItem {
        pub label: &'static str,
        pub status: &'static str,
        pub detail_title: &'static str,
        pub detail_text: &'static str,
    }

    pub struct RustmixWaveHomeState {
        pub selected_index: usize,
        pub items: &'static [RustmixWaveHomeItem],
        pub footer_hint: &'static str,
        pub voice_status: &'static str,
    }

    const RUSTMIX_WAVE_HOME_ITEMS: [RustmixWaveHomeItem; 6] = [
        RustmixWaveHomeItem {
            label: "READER",
            status: "BOOKS",
            detail_title: "READER",
            detail_text: "OPEN BOOKS AND RECENT READS",
        },
        RustmixWaveHomeItem {
            label: "NETWORK",
            status: "WIFI",
            detail_title: "NETWORK",
            detail_text: "WIFI TRANSFER AND SYNC",
        },
        RustmixWaveHomeItem {
            label: "PRODUCT",
            status: "TOOLS",
            detail_title: "PRODUCTIVITY",
            detail_text: "CALENDAR NOTES AND TASKS",
        },
        RustmixWaveHomeItem {
            label: "VOICE",
            status: "HOLD",
            detail_title: "VOICE",
            detail_text: "HOLD DIAL TO TALK LATER",
        },
        RustmixWaveHomeItem {
            label: "TOOLS",
            status: "APPS",
            detail_title: "TOOLS",
            detail_text: "FLASHCARDS DICTIONARY APPS",
        },
        RustmixWaveHomeItem {
            label: "SYSTEM",
            status: "SETUP",
            detail_title: "SYSTEM",
            detail_text: "SETTINGS POWER AND STATUS",
        },
    ];

    impl RustmixWaveHomeState {
        pub fn new(selected_index: usize) -> Self {
            let max_index = RUSTMIX_WAVE_HOME_ITEMS.len().saturating_sub(1);

            Self {
                selected_index: core::cmp::min(selected_index, max_index),
                items: &RUSTMIX_WAVE_HOME_ITEMS,
                footer_hint: "ROTATE SELECT  PRESS OPEN  HOLD TALK",
                voice_status: "VOICE IDLE",
            }
        }

        pub fn selected_item(&self) -> &'static RustmixWaveHomeItem {
            &self.items[self.selected_index]
        }
    }

    fn marker_for_selection(index: usize) {
        match index {
            0 => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-READER\n\0"),
            1 => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-NETWORK\n\0"),
            2 => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-PRODUCT\n\0"),
            3 => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-VOICE\n\0"),
            4 => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-TOOLS\n\0"),
            5 => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-SYSTEM\n\0"),
            _ => raw_marker(b"RAW-RUSTMIX-WAVE-UI-SELECT-UNKNOWN\n\0"),
        }
    }

    fn glyph_5x7(ch: char) -> [u8; 7] {
        match ch {
            'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
            'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
            'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
            'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
            'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
            'G' => [0b01111, 0b10000, 0b10000, 0b10011, 0b10001, 0b10001, 0b01111],
            'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
            'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100],
            'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
            'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
            'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
            'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
            'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
            'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
            'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
            'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
            'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
            'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
            'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
            'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
            'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
            'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
            '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
            '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
            '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
            '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
            '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
            '5' => [0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110],
            '6' => [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
            '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
            '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
            '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
            ':' => [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000],
            '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
            '/' => [0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000],
            ' ' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
            _ => [0b11111, 0b10001, 0b00110, 0b00100, 0b00110, 0b10001, 0b11111],
        }
    }

    fn draw_char<D>(
        display: &mut D,
        x: u32,
        y: u32,
        scale: u32,
        ch: char,
        color: BinaryColor,
    )
    where
        D: RustmixWaveHomeDisplaySurface,
    {
        let glyph = glyph_5x7(ch);

        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..5u32 {
                if (*bits & (1u8 << (4 - col))) != 0 {
                    display.fill_rect(
                        x + col * scale,
                        y + row as u32 * scale,
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
    }

    fn draw_text<D>(
        display: &mut D,
        mut x: u32,
        y: u32,
        scale: u32,
        text: &str,
        color: BinaryColor,
    )
    where
        D: RustmixWaveHomeDisplaySurface,
    {
        for ch in text.chars() {
            draw_char(display, x, y, scale, ch, color);
            x = x.saturating_add(6 * scale);
        }
    }

    fn draw_menu_row<D>(
        display: &mut D,
        index: usize,
        item: &RustmixWaveHomeItem,
        selected: bool,
    )
    where
        D: RustmixWaveHomeDisplaySurface,
    {
        let y = 96 + index as u32 * 74;

        if selected {
            display.stroke_rect(76, y - 8, 368, 64, 4, BinaryColor::On);
            display.fill_rect(90, y + 5, 12, 38, BinaryColor::On);
            display.fill_rect(112, y + 4, 124, 32, BinaryColor::On);
            draw_text(display, 126, y + 10, 3, item.label, BinaryColor::Off);
        } else {
            display.stroke_rect(88, y - 4, 344, 58, 2, BinaryColor::On);
            draw_text(display, 116, y + 10, 3, item.label, BinaryColor::On);
        }

        draw_text(display, 304, y + 18, 2, item.status, BinaryColor::On);
    }

    pub fn render_rustmix_wave_home_v0<D>(
        display: &mut D,
        state: &RustmixWaveHomeState,
    ) -> Result<()>
    where
        D: RustmixWaveHomeDisplaySurface,
    {
        raw_marker(b"RAW-RUSTMIX-WAVE-UI-RENDER-START\n\0");

        display.clear(BinaryColor::Off);

        display.fill_rect(0, 0, 480, 64, BinaryColor::On);
        draw_text(display, 20, 18, 3, "RUSTMIX WAVE", BinaryColor::Off);
        draw_text(display, 366, 18, 3, "14:15", BinaryColor::Off);

        draw_text(display, 80, 74, 2, "ROTARY HOME", BinaryColor::On);

        for (index, item) in state.items.iter().enumerate() {
            draw_menu_row(display, index, item, index == state.selected_index);
        }

        let selected = state.selected_item();

        display.stroke_rect(76, 558, 368, 126, 3, BinaryColor::On);
        draw_text(display, 102, 584, 3, selected.detail_title, BinaryColor::On);
        draw_text(display, 102, 626, 2, selected.detail_text, BinaryColor::On);
        draw_text(display, 102, 656, 2, state.voice_status, BinaryColor::On);

        display.fill_rect(76, 728, 368, 6, BinaryColor::On);
        draw_text(display, 34, 752, 2, state.footer_hint, BinaryColor::On);

        display.flush()?;

        raw_marker(b"RAW-RUSTMIX-WAVE-UI-RENDER-OK\n\0");

        Ok(())
    }

    pub fn render_rustmix_wave_home_navigation_smoke<D>(display: &mut D) -> Result<()>
    where
        D: RustmixWaveHomeDisplaySurface,
    {
        raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-UI-V0-START\n\0");

        for selected_index in 0..RUSTMIX_WAVE_HOME_ITEMS.len() {
            marker_for_selection(selected_index);

            let state = RustmixWaveHomeState::new(selected_index);
            render_rustmix_wave_home_v0(display, &state)?;

            FreeRtos::delay_ms(1200);
        }

        raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-UI-V0-OK\n\0");

        Ok(())
    }
}
// END RUSTMIX_WAVE_SHELL_BRIDGE_UI_IMPORT_V0''')

HAL.write_text(hal)


# ---------------------------------------------------------------------------
# Target main: replace black/white smoke with shell bridge UI smoke.
# ---------------------------------------------------------------------------
main = TARGET_MAIN.read_text()

main = main.replace(
    "use hal_waveshare_epd397::{board, display::DisplayBackendAdapter, raw_marker};",
    "use hal_waveshare_epd397::{\n    board,\n    display::{DisplayBackendAdapter, ShellDisplayBridge},\n    raw_marker,\n    ui::render_rustmix_wave_home_navigation_smoke,\n};",
)

ui_smoke = r'''raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-UI-SMOKE-START\n\0");

    let backend = DisplayBackendAdapter::new(spi, dc, rst, busy);
    let mut shell_display = ShellDisplayBridge::new(backend);

    shell_display
        .init()
        .context("Rustmix-Wave shell display init failed")?;
    raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-UI-INIT-OK\n\0");

    render_rustmix_wave_home_navigation_smoke(&mut shell_display)
        .context("Rustmix-Wave shell UI navigation smoke failed")?;

    raw_marker(b"RAW-RUSTMIX-WAVE-SHELL-UI-SMOKE-OK\n\0");

    loop {
        esp_idf_hal::delay::FreeRtos::delay_ms(1000);
    }
'''

main = replace_from_marker_through_loop(
    main,
    'raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-SMOKE-START\\n\\0");',
    ui_smoke,
)

target_no_comments = "\n".join(line.split("//", 1)[0] for line in main.splitlines())

for forbidden in ["InputRouter", "spawn_input_task", "rot_sw", "gpio3.degrade_input"]:
    if forbidden in target_no_comments:
        raise SystemExit(f"target main appears to enable real input: {forbidden}")

TARGET_MAIN.write_text(main)


# ---------------------------------------------------------------------------
# Docs and README update.
# ---------------------------------------------------------------------------
write_text(
    DOCS / "shell-bridge-ui-import-v0.md",
    """# Rustmix-Wave Shell Bridge UI Import v0

## Scope

This slice imports the shell-facing UI layer onto the accepted Waveshare display
backend.

It keeps:

- `DisplayBackendAdapter` as the display path.
- `ShellDisplayBridge` portrait 480x800 mapping.
- GPIO3 reserved for EPD_BUSY.

It adds:

- Rustmix-Wave rotary-first home dashboard.
- Vertical focus-first menu.
- Selected row highlight.
- Detail panel.
- Voice/status line.
- Simulated navigation only.

## Menu model

- Reader
- Network
- Productivity
- Voice
- Tools
- System

## What this slice does not do

- Does not enable real rotary input.
- Does not use GPIO3 for input.
- Does not port the reader.
- Does not enable audio/voice capture.

## Smoke markers

- `RAW-RUSTMIX-WAVE-SHELL-UI-V0-START`
- `RAW-RUSTMIX-WAVE-UI-SELECT-READER`
- `RAW-RUSTMIX-WAVE-UI-SELECT-NETWORK`
- `RAW-RUSTMIX-WAVE-UI-SELECT-PRODUCT`
- `RAW-RUSTMIX-WAVE-UI-SELECT-VOICE`
- `RAW-RUSTMIX-WAVE-UI-SELECT-TOOLS`
- `RAW-RUSTMIX-WAVE-UI-SELECT-SYSTEM`
- `RAW-RUSTMIX-WAVE-SHELL-UI-V0-OK`
""",
)

readme = README.read_text()
readme_block = """<!-- BEGIN RUSTMIX_WAVE_SHELL_BRIDGE_UI_IMPORT_V0 -->
## Rustmix-Wave Shell Bridge UI Import v0

Rustmix-Wave now renders a rotary-first home dashboard through the accepted
Waveshare display path:

`DisplayBackendAdapter -> ShellDisplayBridge -> Rustmix-Wave home UI`

This slice adds:

- Portrait 480x800 shell UI rendering.
- Rotary-first vertical home menu.
- Selected row highlight.
- Detail panel and voice/status line.
- Simulated navigation only.

This slice intentionally does not enable real rotary input and does not port the
reader yet. GPIO3 remains reserved for EPD_BUSY.
<!-- END RUSTMIX_WAVE_SHELL_BRIDGE_UI_IMPORT_V0 -->"""

readme = replace_marked_block(
    readme,
    "<!-- BEGIN RUSTMIX_WAVE_SHELL_BRIDGE_UI_IMPORT_V0 -->",
    "<!-- END RUSTMIX_WAVE_SHELL_BRIDGE_UI_IMPORT_V0 -->",
    readme_block,
)
README.write_text(readme)


# ---------------------------------------------------------------------------
# Validator.
# ---------------------------------------------------------------------------
write_text(
    SCRIPTS / "validate_rustmix_wave_shell_bridge_ui_import_v0.py",
    r'''#!/usr/bin/env python3
from pathlib import Path
import sys

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()

errors = []

def read(path: Path) -> str:
    try:
        return path.read_text()
    except FileNotFoundError:
        errors.append(f"missing file: {path.relative_to(ROOT)}")
        return ""

hal = read(ROOT / "hal-waveshare-epd397/src/lib.rs")
target = read(ROOT / "target-waveshare-epd397/src/main.rs")
docs = read(ROOT / "docs/rustmix-wave/shell-bridge-ui-import-v0.md")
readme = read(ROOT / "README.md")

required_hal = [
    "pub struct DisplayBackendAdapter",
    "pub struct ShellDisplayBridge",
    "pub fn stroke_rect",
    "pub mod ui",
    "pub trait RustmixWaveHomeDisplaySurface",
    "pub struct RustmixWaveHomeItem",
    "pub struct RustmixWaveHomeState",
    "pub selected_index: usize",
    "pub footer_hint: &'static str",
    "pub voice_status: &'static str",
    "render_rustmix_wave_home_v0",
    "render_rustmix_wave_home_navigation_smoke",
    "RAW-RUSTMIX-WAVE-SHELL-UI-V0-START",
    "RAW-RUSTMIX-WAVE-UI-SELECT-READER",
    "RAW-RUSTMIX-WAVE-UI-SELECT-NETWORK",
    "RAW-RUSTMIX-WAVE-UI-SELECT-PRODUCT",
    "RAW-RUSTMIX-WAVE-UI-SELECT-VOICE",
    "RAW-RUSTMIX-WAVE-UI-SELECT-TOOLS",
    "RAW-RUSTMIX-WAVE-UI-SELECT-SYSTEM",
    "RAW-RUSTMIX-WAVE-SHELL-UI-V0-OK",
    "RUSTMIX WAVE",
    "ROTARY HOME",
    "VOICE IDLE",
]

for item in required_hal:
    if item not in hal:
        errors.append(f"HAL missing {item}")

required_target = [
    "DisplayBackendAdapter",
    "ShellDisplayBridge",
    "render_rustmix_wave_home_navigation_smoke",
    "RAW-RUSTMIX-WAVE-SHELL-UI-SMOKE-START",
    "RAW-RUSTMIX-WAVE-SHELL-UI-INIT-OK",
    "RAW-RUSTMIX-WAVE-SHELL-UI-SMOKE-OK",
    "pins.gpio3",
]

for item in required_target:
    if item not in target:
        errors.append(f"target missing {item}")

target_no_comments = "\n".join(line.split("//", 1)[0] for line in target.splitlines())

for forbidden in [
    "InputRouter",
    "spawn_input_task",
    "rot_sw",
    "gpio3.degrade_input",
    "reader port complete",
]:
    if forbidden in target_no_comments:
        errors.append(f"target appears to include forbidden input/reader functionality: {forbidden}")

for item in [
    "Does not enable real rotary input",
    "Does not use GPIO3 for input",
    "Does not port the reader",
    "DisplayBackendAdapter",
    "ShellDisplayBridge",
]:
    combined = docs + "\n" + readme
    if item.lower() not in combined.lower():
        errors.append(f"docs/README missing {item}")

if "GPIO3 is EPD_BUSY" not in hal:
    errors.append("HAL must preserve GPIO3 as EPD_BUSY")

if errors:
    for error in errors:
        print("ERROR:", error)
    raise SystemExit(1)

print("rustmix-wave-shell-bridge-ui-import-v0=ok")
''',
)

(SCRIPTS / "validate_rustmix_wave_shell_bridge_ui_import_v0.py").chmod(0o755)

print("rustmix-wave-shell-bridge-ui-import-v0-applied=ok")
