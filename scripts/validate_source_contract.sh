#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

failed=0
check() {
  local label="$1"
  shift
  if "$@"; then
    printf '%s=ok\n' "$label"
  else
    printf '%s=failed\n' "$label" >&2
    failed=1
  fi
}
contains() {
  local path="$1"
  local pattern="$2"
  grep -Fq -- "$pattern" "$path"
}
not_contains() {
  local path="$1"
  local pattern="$2"
  ! grep -Fq -- "$pattern" "$path"
}

clean_repository_contract() {
  python3 - <<'PY'
from pathlib import Path
import re

root = Path('.')

# Extracted patch overlays, generated archives, and cache files are not source.
for child in root.iterdir():
    if child.is_dir() and child.name.startswith('waveshare-epd397-rust-'):
        raise AssertionError(f'extracted overlay directory present: {child}')
ignored_generated_roots = {'.git', 'target', '.embuild', 'dist'}
for path in root.rglob('*'):
    if any(part in ignored_generated_roots for part in path.parts):
        continue
    if path.is_file() and (path.suffix in {'.zip', '.sha256', '.pyc', '.orig', '.rej'} or path.name == '.DS_Store'):
        raise AssertionError(f'local artifact present: {path}')
    if path.is_dir() and path.name == '__pycache__':
        raise AssertionError(f'python cache directory present: {path}')

# Durable documentation is intentionally small and consolidated.
expected = {
    'ARCHITECTURE.md',
    'BOARD_CONTRACT.md',
    'KNOWN_ISSUES.md',
    'PHYSICAL_SMOKE_TEST.md',
    'RELEASE.md',
    'SD_CARD_SETUP.md',
    'USER_GUIDE.md',
}
actual = {p.name for p in Path('docs').iterdir() if p.is_file()}
assert actual == expected, f'durable docs mismatch: actual={sorted(actual)} expected={sorted(expected)}'
assert not list(Path('docs').glob('V0.*')), 'historical milestone docs must be removed'
assert not list(Path('docs').glob('*REPAIR*')), 'repair docs must be removed'

# README and architecture are the durable entry points.
readme = Path('README.md').read_text()
arch = Path('docs/ARCHITECTURE.md').read_text()
for fragment in (
    'Current release: **v1.0.0**',
    'scripts/build-release-firmware.sh',
    'scripts/flash-release.sh',
    'scripts/test-release-flash-workflow.sh',
    'Power short',
    'Power long',
    'Dictionary',
    'Calendar',
    'Voice Notes',
    'docs/USER_GUIDE.md',
    'screenshots/',
    'Sensor-driven utilities and motion games',
    'Main-task safety and worker isolation',
):
    assert fragment in readme, f'README missing: {fragment}'
for fragment in (
    'Design rules',
    'Display and refresh ownership',
    'Voice Notes boundary',
    'Dictionary boundary',
    'Calendar boundary',
    'Wi-Fi transfer boundary',
    'Validation boundary',
    'Board-service and sensor ownership',
    'Native IMU event pipeline',
    'Main-task safety and worker boundary',
    'Screenshot-driven user documentation',
):
    assert fragment in arch, f'architecture missing: {fragment}'
PY
}


screenshot_user_guide_contract() {
  python3 - <<'PY'
from pathlib import Path

screenshots = Path('screenshots')
guide = Path('docs/USER_GUIDE.md').read_text()
readme = Path('README.md').read_text()
architecture = Path('docs/ARCHITECTURE.md').read_text()

assert screenshots.is_dir(), 'screenshots directory missing'
assert (screenshots / 'README.md').is_file(), 'screenshots/README.md missing'

expected = {
    'alarm-details.jpg', 'alarms.jpg', 'audio-details.jpg', 'audio.jpg',
    'calendar-create-note.jpg', 'calendar-current-day.jpg', 'calendar-date-details.jpg',
    'calendar-us-events.jpg', 'clock.jpg', 'continue-reading1.jpg', 'device-info.jpg',
    'device-info1.jpg', 'device-info2.jpg', 'dictionary-result.jpg', 'dictionary.jpg',
    'directory-listing.jpg', 'display.jpg', 'environment.jpg', 'environment1.jpg',
    'epub-reader-options.jpg', 'epub-reader.jpg', 'files-listing.jpg', 'games-listing.jpg',
    'games.jpg', 'hello-grid.jpg', 'homepage.jpg', 'library-bookmarks.jpg',
    'library-books.jpg', 'library-files.jpg', 'library-recent.jpg', 'minesweeper.jpg',
    'motion-events.jpg', 'motion.jpg', 'motion20248.jpg', 'network-details.jpg',
    'network.jpg', 'opening_book.jpg', 'productivity.jpg', 'reader-bookmarks-list.jpg',
    'reader-bookmarks.jpg', 'reader-main.jpg', 'reader-reading-prefs.jpg', 'reader-toc.jpg',
    'rtc-details.jpg', 'settings.jpg', 'settings1.jpg', 'sleep.jpg', 'sobokan-tilt.jpg',
    'sudoku.jpg', 'tilt-maze.jpg', 'tools.jpg', 'txt-reader-options.jpg', 'txt-reader.jpg',
    'unit-converter.jpg', 'unit-converter1.jpg', 'voice_note_detail.jpg',
    'voice_note_edit.jpg', 'voice_notes.jpg', 'voice_notes_record.jpg', 'weather-1.jpg',
    'weather.png', 'wifi-transfer.jpg',
}
actual = {path.name for path in screenshots.iterdir() if path.is_file() and path.name != 'README.md'}
assert actual == expected, f'screenshot set mismatch: missing={sorted(expected-actual)} extra={sorted(actual-expected)}'
for filename in sorted(expected):
    assert f'../screenshots/{filename}' in guide, f'user guide does not reference screenshot: {filename}'
for fragment in (
    '# Rustmix Wave user guide',
    '## Physical controls',
    '## 2. Reader',
    '## 3. Productivity',
    '## 4. Games',
    '## 5. Tools',
    '## 6. Settings',
    '## 7. Power-key maintenance and sleep',
    '## 8. Screenshot index',
    'NAV H', 'NAV V', 'Tilt Maze', 'Motion 2048', 'Sokoban Tilt',
):
    assert fragment in guide, f'user guide missing: {fragment}'
for content, label in ((readme, 'README'), (architecture, 'architecture')):
    assert 'USER_GUIDE.md' in content, f'{label} missing user guide link'
    assert 'screenshots/' in content or '/screenshots' in content, f'{label} missing screenshots link'
PY
}

ci_workflow_contract() {
  python3 - <<'PY'
from pathlib import Path
workflow = Path('.github/workflows/ci.yml').read_text()
assert not Path('.github/workflows/source-contract.yml').exists()
for fragment in (
    'actions/checkout@v4',
    'dtolnay/rust-toolchain@stable',
    'components: rustfmt',
    'bash -n scripts/*.sh',
    'cargo +stable fmt --all -- --check',
    './scripts/validate_source_contract.sh',
    './scripts/test-host.sh',
    './scripts/test-release-flash-workflow.sh',
    'workflow_dispatch:',
):
    assert fragment in workflow, f'workflow missing: {fragment}'
PY
}

release_binary_builder_contract() {
  python3 - <<'PY'
from pathlib import Path
builder = Path('scripts/build-release-firmware.sh').read_text()
flasher = Path('scripts/flash-release.sh').read_text()
release_doc = Path('docs/RELEASE.md').read_text()
readme = Path('README.md').read_text()
known = Path('docs/KNOWN_ISSUES.md').read_text()

for fragment in (
    './scripts/validate.sh',
    'cargo +esp build --release',
    'target/xtensa-esp32s3-espidf/release/waveshare-epd397-rust-app',
    '-flash-release.sh',
    '-firmware-release.sha256',
    '-firmware-release.zip',
    "echo 'release-firmware-format=elf-only'",
    "echo 'release-firmware-build=ok'",
):
    assert fragment in builder, f'ELF release builder missing: {fragment}'
for unsafe in (
    'espflash save-image',
    'espflash write-bin --chip esp32s3 0x0',
    'BIN_OUT=',
    'release-firmware-bin=',
):
    assert unsafe not in builder, f'unsafe release builder fragment present: {unsafe}'
assert 'rm -f dist/waveshare-epd397-rust-app-v*-flash.bin' in builder

for fragment in (
    'espflash flash --chip esp32s3',
    '--monitor "$ELF"',
    '--port "$PORT"',
    'release-flash=failed error=missing-release-elf',
):
    assert fragment in flasher, f'release flash helper missing: {fragment}'
assert 'write-bin' not in flasher, 'release flash helper must not use raw writes'

for content, label in ((release_doc, 'release doc'), (readme, 'README'), (known, 'known issues')):
    assert 'espflash write-bin' in content, f'{label} missing raw-address warning'
    assert 'raw-address' in content.lower(), f'{label} missing raw-address explanation'
    assert 'factory' in content.lower(), f'{label} missing deferred factory-image note'
assert 'espflash write-bin --chip esp32s3 0x0' not in release_doc
assert '*-flash.bin' in release_doc and 'No `*-flash.bin` artifact is generated.' in release_doc

# The cleaned source must not carry an unverified legacy raw-address artifact.
assert not list(Path('dist').glob('*-flash.bin')), 'legacy dist/*-flash.bin artifact present'
PY
}

package_release_contract() {
  python3 - <<'PY'
from pathlib import Path
script = Path('scripts/package-release.sh').read_text()
for fragment in (
    './scripts/validate.sh',
    "--exclude 'dist/'",
    "--exclude '*.zip'",
    "--exclude '*.sha256'",
    "--exclude 'waveshare-epd397-rust-*-repair-*/'",
    "--exclude 'waveshare-epd397-rust-*-v*/'",
    'release-source-zip=',
):
    assert fragment in script, f'source packager missing: {fragment}'
PY
}

host_test_native_target_contract() {
  python3 - <<'PY'
from pathlib import Path
script = Path('scripts/test-host.sh').read_text()
for fragment in (
    'HOST_TRIPLE="$(rustc +stable -vV',
    "sed -n 's/^host: //p'",
    'cargo +stable test --target "$HOST_TRIPLE" --lib',
    "echo 'host-test-native-target-isolation=ok'",
):
    assert fragment in script, f'host test helper missing: {fragment}'
PY
}

runtime_contract() {
  python3 - <<'PY'
from pathlib import Path

lib = Path('src/lib.rs').read_text()
main = Path('src/main.rs').read_text()
state = Path('src/app/state.rs').read_text()
calendar = Path('src/calendar.rs').read_text()
dictionary = Path('src/dictionary.rs').read_text()
keyboard = Path('src/keyboard_navigation.rs').read_text()
voice = Path('src/voice_notes.rs').read_text()
wifi = Path('src/wifi_transfer.rs').read_text()
power = Path('src/power_key.rs').read_text()

for module in (
    'calendar', 'dictionary', 'keyboard_navigation', 'power_key', 'power_key_menu',
    'reader', 'epub', 'voice_notes', 'voice_note_metadata', 'wifi_transfer',
    'alarm', 'sleep_mode', 'sleep_images', 'sleep_network', 'lua_runtime', 'games',
):
    assert f'pub mod {module};' in lib, f'library module missing: {module}'

for marker in (
    'rustmix-wave=release-flash-workflow-safety-ready',
    'rustmix-wave=power-key-short-menu-long-sleep-ready',
    'rustmix-wave=calendar-personal-event-editor-ready',
    'rustmix-wave=calendar-us-events-daily-agenda-ready',
    'rustmix-wave=offline-dictionary-x4-pack-native-foundation-ready',
    'rustmix-wave=voice-notes-organizer-controls-export-ready',
    'rustmix-wave=wifi-transfer-web-portal-ready',
):
    assert marker in main, f'runtime readiness marker missing: {marker}'

# Power-key behavior: short menu, long sleep, manual global refresh, non-const helper.
assert 'pub fn power_key_sleep_restore_route(&self) -> ScreenRoute {' in state
assert 'pub const fn power_key_sleep_restore_route(&self) -> ScreenRoute {' not in state
assert 'state.open_power_key_menu();' in main
assert 'event == PowerKeyEvent::ShortPress' in main
assert 'power_key_event_from_irq_status' in power
assert 'power_key_clear_ghost' in main
assert 'POWER_KEY_LONG_PRESS_MASK' in power
assert 'POWER_KEY_SHORT_PRESS_MASK' in power

# Shared keyboard H/V behavior is the default for keyboard-like screens.
for fragment in ('KeyboardGridNavigation', 'toggle_axis', 'Horizontal', 'Vertical'):
    assert fragment in keyboard, f'keyboard helper missing: {fragment}'
assert 'keyboard_navigation: KeyboardGridNavigation' in dictionary
assert 'toggle_navigation_axis' in dictionary
assert 'NAV H' in keyboard and 'NAV V' in keyboard
assert 'KeyboardGridNavigation' in calendar
assert 'title_edit_navigation: KeyboardGridNavigation' in voice
assert 'toggle_title_editor_navigation_axis' in voice

# Dictionary reuses the bounded X4 pack.
for fragment in (
    'DICTIONARY_ROOT: &str = "/sdcard/RUSTMIX/APPS/DICT"',
    'DICTIONARY_INDEX_FILE: &str = "INDEX.TXT"',
    'DICTIONARY_SHARD_MAX_BYTES: usize = 16 * 1024',
    'DATA/', '.JSN', 'lookup_dictionary_with_index',
):
    assert fragment in dictionary, f'dictionary contract missing: {fragment}'

# Calendar personal rows are writable, U.S. rows read-only, Hindu pack excluded.
for fragment in (
    'CALENDAR_ROOT: &str = "/sdcard/RUSTMIX/APPS/CALENDAR"',
    'CALENDAR_EVENTS_FILE: &str = "EVENTS.TXT"',
    'CALENDAR_US_EVENTS_FILE: &str = "US2026.TXT"',
    'CALENDAR_EVENTS_TEMP_FILE: &str = "EVENTS.TMP"',
    'CALENDAR_EVENTS_BACKUP_FILE: &str = "EVENTS.BAK"',
    'HINDU26.TXT',
):
    assert fragment in calendar, f'calendar contract missing: {fragment}'

# Voice Notes keep FAT-safe WAV files, bounded stream finalization, and native telemetry.
for fragment in (
    'VOICE_NOTES_ROOT', 'VOICE_PCM_MONO_CHUNK_BYTES', 'VOICE001.WAV',
    'META.TXT', 'SETTINGS.TXT', 'VoicePlaybackSession', 'cleanup_stale_voice_tmp',
):
    assert fragment in voice or fragment in Path('src/voice_note_metadata.rs').read_text(), f'voice notes contract missing: {fragment}'
assert 'sys::esp_vfs_fat_info' in main
assert 'sys::statvfs' not in main
voice_screen = Path('src/app/screens/voice_notes.rs').read_text()
calendar_screen = Path('src/app/screens/calendar.rs').read_text()
for fragment in (
    'VOICE NOTE TITLE', 'EDIT FRIENDLY TITLE', 'VOICE_TITLE_EDITOR_KEY_ROWS',
    'MOVE  BOOT H/V  SELECT KEY  HOLD BACK',
):
    assert fragment in voice_screen, f'voice title editor layout missing: {fragment}'
for fragment in (
    'calendar_editor_status_date_label', 'CALENDAR_EDITOR_FOOTER_HINT',
    'MOVE  BOOT H/V  SELECT KEY  HOLD BACK',
):
    assert fragment in calendar_screen, f'calendar editor compact layout missing: {fragment}'

# Wi-Fi portal protects configuration and internal sidecars.
for protected in (
    'WIFI.TXT', 'ALARMS.TXT', 'DISPLAY.TXT', 'WEATHER.TXT',
    'VOICE/META.TXT', 'VOICE/SETTINGS.TXT',
    'APPS/CALENDAR/EVENTS.TMP', 'APPS/CALENDAR/EVENTS.BAK',
):
    assert f'"{protected}"' in wifi, f'protected portal path missing: {protected}'
PY
}

sd_examples_contract() {
  python3 - <<'PY'
from pathlib import Path
required = (
    'examples/sd-card/RUSTMIX/WIFI.TXT.example',
    'examples/sd-card/RUSTMIX/WEATHER.TXT.example',
    'examples/sd-card/RUSTMIX/ALARMS.TXT.example',
    'examples/sd-card/RUSTMIX/DISPLAY.TXT.example',
    'examples/sd-card/RUSTMIX/SLEEP/SLEEP.BMP',
    'examples/sd-card/RUSTMIX/APPS/DICT/INDEX.TXT',
    'examples/sd-card/RUSTMIX/APPS/DICT/DATA/AA.JSN',
    'examples/sd-card/RUSTMIX/APPS/CALENDAR/EVENTS.TXT',
    'examples/sd-card/RUSTMIX/APPS/CALENDAR/US2026.TXT',
    'examples/sd-card/RUSTMIX/APPS/SUDOKU/MAIN.LUA',
    'examples/sd-card/RUSTMIX/APPS/MINES/MAIN.LUA',
    'examples/sd-card/RUSTMIX/APPS/TILTMAZE/MAIN.LUA',
    'examples/sd-card/RUSTMIX/APPS/M2048/MAIN.LUA',
    'examples/sd-card/RUSTMIX/APPS/SOKOBAN/MAIN.LUA',
)
for path in required:
    assert Path(path).is_file(), f'SD example missing: {path}'
assert not Path('examples/sd-card/RUSTMIX/APPS/CALENDAR/HINDU26.TXT').exists()
PY
}

rust_lexical_delimiter_scan() {
  python3 - <<'PY'
from pathlib import Path

pairs = {'(': ')', '[': ']', '{': '}'}
closing = {v: k for k, v in pairs.items()}
for path in sorted(Path('src').rglob('*.rs')):
    text = path.read_text()
    stack = []
    i = 0
    state = 'code'
    while i < len(text):
        ch = text[i]
        nxt = text[i + 1] if i + 1 < len(text) else ''
        if state == 'code':
            if ch == '/' and nxt == '/':
                state = 'line_comment'; i += 2; continue
            if ch == '/' and nxt == '*':
                state = 'block_comment'; i += 2; continue
            if ch == '"':
                state = 'string'; i += 1; continue
            if ch == "'":
                # Rust lifetimes are not character literals. Treat as a char only
                # when a closing quote is nearby.
                end = text.find("'", i + 1, min(len(text), i + 6))
                if end != -1:
                    i = end + 1; continue
            if ch in pairs:
                stack.append((ch, i))
            elif ch in closing:
                if not stack or stack[-1][0] != closing[ch]:
                    raise AssertionError(f'{path}: unmatched {ch} at byte {i}')
                stack.pop()
        elif state == 'line_comment':
            if ch == '\n': state = 'code'
        elif state == 'block_comment':
            if ch == '*' and nxt == '/': state = 'code'; i += 2; continue
        elif state == 'string':
            if ch == '\\': i += 2; continue
            if ch == '"': state = 'code'
        i += 1
    if stack:
        raise AssertionError(f'{path}: unclosed delimiters: {stack[-3:]}')
PY
}

check cargo-version-v1.0.0 grep -Eq '^version = "1\.0\.0"$' Cargo.toml
check cargo-lock-version-v1.0.0 bash -c "grep -A2 'name = \"waveshare-epd397-rust-app\"' Cargo.lock | grep -q 'version = \"1.0.0\"'"
check sdkconfig-version-v1.0.0 contains sdkconfig.defaults 'CONFIG_APP_PROJECT_VER="1.0.0"'
check build-info-milestone contains src/build_info.rs 'UI_SHELL_MILESTONE: &str = "text-editor-layout-alignment"'
check cleaned-repository-contract clean_repository_contract
check screenshot-user-guide-contract screenshot_user_guide_contract
check ci-workflow-contract ci_workflow_contract
check release-elf-builder release_binary_builder_contract
check release-flash-workflow-selftest-script contains scripts/test-release-flash-workflow.sh 'release-flash-workflow-selftest=ok'
check package-release-contract package_release_contract
check host-test-native-target-isolation host_test_native_target_contract
check runtime-contract runtime_contract
check sd-examples-contract sd_examples_contract
check font-notice-serif contains docs/licenses/FONT_NOTICES.md 'DejaVu Serif'
check font-notice-atkinson contains docs/licenses/FONT_NOTICES.md 'Atkinson Hyperlegible Next Medium'
check font-notice-literata contains docs/licenses/FONT_NOTICES.md 'Literata Medium'
check no-raw-font-files bash -c '! find . -type f \( -iname "*.ttf" -o -iname "*.otf" -o -iname "*.woff" -o -iname "*.woff2" \) -print -quit | grep -q .'
for script in scripts/*.sh; do
  check "bash-syntax-$(basename "$script")" bash -n "$script"
done
check rust-lexical-delimiter-scan rust_lexical_delimiter_scan

if [[ "$failed" -ne 0 ]]; then
  echo 'source-contract-validation=failed' >&2
  exit 1
fi

echo 'source-contract-validation=ok'
