#!/usr/bin/env python3
from pathlib import Path
import sys

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()
HAL = ROOT / "hal-waveshare-epd397" / "src" / "lib.rs"
VALIDATOR = ROOT / "scripts" / "validate_rustmix_wave_shell_bridge_ui_import_v0.py"

if not HAL.exists():
    raise SystemExit(f"missing {HAL}")
if not VALIDATOR.exists():
    raise SystemExit(f"missing {VALIDATOR}")

src = HAL.read_text()


def find_matching_brace(text: str, brace_pos: int) -> int:
    depth = 0
    for i in range(brace_pos, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return i + 1
    raise SystemExit("could not find matching brace")


def impl_range(text: str, needle: str) -> tuple[int, int]:
    start = text.find(needle)
    if start < 0:
        raise SystemExit(f"could not find impl block: {needle}")

    brace = text.find("{", start)
    if brace < 0:
        raise SystemExit(f"could not find opening brace for impl: {needle}")

    end = find_matching_brace(text, brace)
    return start, end


def remove_fn_in_range(text: str, range_start: int, range_end: int, signature: str) -> tuple[str, bool]:
    start = text.find(signature, range_start, range_end)
    if start < 0:
        return text, False

    brace = text.find("{", start, range_end)
    if brace < 0:
        raise SystemExit(f"could not find function opening brace: {signature}")

    end = find_matching_brace(text, brace)

    while end < len(text) and text[end] in "\r\n":
        end += 1

    return text[:start] + text[end:], True


stroke_rect_fn = r'''        pub fn stroke_rect(
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

# 1. Remove accidentally inserted DisplayBackendAdapter::stroke_rect.
adapter_start, adapter_end = impl_range(src, "impl<'d> DisplayBackendAdapter<'d>")
src, removed_adapter_stroke = remove_fn_in_range(
    src,
    adapter_start,
    adapter_end,
    "        pub fn stroke_rect(",
)

# Recompute ranges after removal.
shell_start, shell_end = impl_range(src, "impl<'d> ShellDisplayBridge<'d>")
shell_impl = src[shell_start:shell_end]

# 2. Add ShellDisplayBridge::stroke_rect if missing.
if "        pub fn stroke_rect(" not in shell_impl:
    insert_needle = "        pub fn write_frame(&mut self, frame: &[u8]) -> Result<()> {"
    insert_at = src.find(insert_needle, shell_start, shell_end)
    if insert_at < 0:
        raise SystemExit("could not find ShellDisplayBridge::write_frame insertion point")
    src = src[:insert_at] + stroke_rect_fn + src[insert_at:]

# 3. Fix trait impl recursion by explicitly calling the inherent ShellDisplayBridge method.
old_recursive = "            self.stroke_rect(x, y, w, h, stroke, color);"
new_disambiguated = "            ShellDisplayBridge::stroke_rect(self, x, y, w, h, stroke, color);"

if old_recursive in src:
    src = src.replace(old_recursive, new_disambiguated, 1)

if "ShellDisplayBridge::stroke_rect(self, x, y, w, h, stroke, color);" not in src:
    raise SystemExit("failed to disambiguate trait stroke_rect call")

HAL.write_text(src)

# 4. Strengthen validator so this regression is caught later.
v = VALIDATOR.read_text()

if "ShellDisplayBridge::stroke_rect(self, x, y, w, h, stroke, color)" not in v:
    marker = '''    "pub fn stroke_rect",
    "pub mod ui",
'''
    replacement = '''    "pub fn stroke_rect",
    "ShellDisplayBridge::stroke_rect(self, x, y, w, h, stroke, color)",
    "pub mod ui",
'''
    v = v.replace(marker, replacement)

    extra_check = '''
# Ensure stroke_rect is not accidentally implemented on DisplayBackendAdapter.
adapter_impl_start = hal.find("impl<'d> DisplayBackendAdapter<'d>")
shell_struct_start = hal.find("pub struct ShellDisplayBridge")
if adapter_impl_start >= 0 and shell_struct_start >= 0:
    adapter_impl_text = hal[adapter_impl_start:shell_struct_start]
    if "pub fn stroke_rect(" in adapter_impl_text:
        errors.append("DisplayBackendAdapter must not own stroke_rect; it belongs on ShellDisplayBridge")
'''
    if "DisplayBackendAdapter must not own stroke_rect" not in v:
        v = v.replace(
            '''if "GPIO3 is EPD_BUSY" not in hal:
    errors.append("HAL must preserve GPIO3 as EPD_BUSY")
''',
            '''if "GPIO3 is EPD_BUSY" not in hal:
    errors.append("HAL must preserve GPIO3 as EPD_BUSY")
''' + extra_check,
        )

VALIDATOR.write_text(v)

print("rustmix-wave-shell-bridge-stroke-rect-repair-v0=ok")
