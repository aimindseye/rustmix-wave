#!/usr/bin/env python3
from pathlib import Path
import sys

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()

cargo_dir = ROOT / ".cargo"
cargo_dir.mkdir(exist_ok=True)

config = cargo_dir / "config.toml"
root_cargo = ROOT / "Cargo.toml"
validator = ROOT / "scripts" / "validate_rustmix_wave_display_backend_import_v0.py"

if not root_cargo.exists():
    raise SystemExit(f"missing {root_cargo}")
if not validator.exists():
    raise SystemExit(f"missing {validator}")

config.write_text("""# Rustmix-Wave ESP-IDF build configuration.
#
# xtensa-esp32s3-espidf does not use prebuilt core/std from rustup.
# The ESP toolchain builds std/core from source for this target.
[build]
target = "xtensa-esp32s3-espidf"

[target.xtensa-esp32s3-espidf]
linker = "ldproxy"
rustflags = ["--cfg", "espidf_time64"]

[unstable]
build-std = ["std", "panic_abort"]

[env]
MCU = "esp32s3"
""")

# Move profile settings to workspace root because profiles in target crate are ignored.
cargo = root_cargo.read_text()

if "[profile.release]" not in cargo:
    cargo = cargo.rstrip() + """

[profile.release]
opt-level = "s"
lto = true

[profile.dev]
debug = true
opt-level = "z"
"""
elif "opt-level = \"s\"" not in cargo and "opt-level = 's'" not in cargo:
    cargo = cargo.rstrip() + """

# Rustmix-Wave target package profiles live here because this is the workspace root.
# Existing profile sections above may override these values.
"""

root_cargo.write_text(cargo)

v = validator.read_text()

if 'read(ROOT / ".cargo/config.toml")' not in v:
    needle = 'readme = read(ROOT / "README.md")\n'
    v = v.replace(
        needle,
        needle + 'cargo_config = read(ROOT / ".cargo/config.toml")\nroot_cargo = read(ROOT / "Cargo.toml")\n',
    )

check = '''
required_xtensa_config = [
    'target = "xtensa-esp32s3-espidf"',
    'linker = "ldproxy"',
    'build-std = ["std", "panic_abort"]',
    'MCU = "esp32s3"',
]

for item in required_xtensa_config:
    if item not in cargo_config:
        errors.append(f".cargo/config.toml missing ESP-IDF Xtensa build config: {item}")

for item in [
    "[profile.release]",
    'opt-level = "s"',
]:
    if item not in root_cargo:
        errors.append(f"workspace Cargo.toml missing root profile setting: {item}")
'''

if "required_xtensa_config" not in v:
    marker = 'combined_docs = docs + "\\n" + readme\n'
    v = v.replace(marker, check + "\n" + marker)

validator.write_text(v)

print("rustmix-wave-xtensa-build-std-config-v0=ok")
