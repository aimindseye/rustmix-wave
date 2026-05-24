# Rustmix Reader Menu/Footer Fix v1

This patch improves the Reader quick menu readability and footer labeling.

Changes:
- Draws Reader quick-menu labels/values/help text with semibold bitmap UI labels.
- Uses the stronger Inter list-section font for the Reader quick menu.
- Replaces old quick-menu help text (`Sel`/`Menu`) with `OK`/`Back` hints.
- Keeps the normal Reader footer action as `Menu` while reading.
- Changes the Reader footer Select label to `OK` while the quick menu is open.

Apply from repo root:

```bash
unzip -o rustmix_reader_menu_footer_fix_v1.zip
./rustmix_reader_menu_footer_fix_v1/scripts/apply_rustmix_reader_menu_footer_fix_v1.sh .
./rustmix_reader_menu_footer_fix_v1/scripts/validate_rustmix_reader_menu_footer_fix_v1.sh .
cargo fmt --all
cargo build -p target-xteink-x4 --release --target riscv32imc-unknown-none-elf
scripts/flash_x4_rustmix_app0.sh /dev/ttyACM0
```
