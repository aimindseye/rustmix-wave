#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-.}"
cd "$ROOT"
QM="target-xteink-x4/src/rustmix_x4/x4_apps/apps/widgets/quick_menu.rs"
BF="target-xteink-x4/src/rustmix_x4/x4_apps/apps/widgets/button_feedback.rs"
MG="target-xteink-x4/src/rustmix_x4/apps/manager.rs"

grep -Fq 'BitmapTextWeight::SemiBold' "$QM"
grep -Fq 'OK: activate  Back: close' "$QM"
grep -Fq 'OK: toggle  Back: close' "$QM"
if grep -Fq 'Sel: activate  Menu: close' "$QM"; then
  echo "legacy quick menu help text still present" >&2
  exit 1
fi
if grep -Fq 'Sel: cycle  Menu: close' "$QM"; then
  echo "legacy quick menu cycle help text still present" >&2
  exit 1
fi

grep -Fq 'draw_with_reader_quick_menu' "$BF"
grep -Fq 'return "OK";' "$BF"
grep -Fq 'reader_quick_menu_open' "$BF"
grep -Fq 'Action::Select => "Menu"' "$BF"

grep -Fq 'draw_with_reader_quick_menu(strip, active == AppId::Reader && self.quick_menu.open)' "$MG"
grep -Fq 'self.quick_menu.set_chrome_font(fonts::ui_list_section_font_fixed())' "$MG"

echo "reader-menu-font-weight=ok"
echo "reader-menu-help-labels=ok"
echo "reader-footer-ok-while-menu-open=ok"
echo "reader-menu-font-source=ok"
echo "rustmix-reader-menu-footer-fix-v1=ok"
