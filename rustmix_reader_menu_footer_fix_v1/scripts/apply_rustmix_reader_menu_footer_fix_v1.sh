#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-.}"
PATCH_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cp "$PATCH_DIR/files/target-xteink-x4/src/rustmix_x4/x4_apps/apps/widgets/quick_menu.rs" \
  "target-xteink-x4/src/rustmix_x4/x4_apps/apps/widgets/quick_menu.rs"
cp "$PATCH_DIR/files/target-xteink-x4/src/rustmix_x4/x4_apps/apps/widgets/button_feedback.rs" \
  "target-xteink-x4/src/rustmix_x4/x4_apps/apps/widgets/button_feedback.rs"
cp "$PATCH_DIR/files/target-xteink-x4/src/rustmix_x4/apps/manager.rs" \
  "target-xteink-x4/src/rustmix_x4/apps/manager.rs"
echo "rustmix-reader-menu-footer-fix-v1 applied"
