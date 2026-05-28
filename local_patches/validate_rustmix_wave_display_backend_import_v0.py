#!/usr/bin/env python3
from pathlib import Path
import subprocess
import sys

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path.cwd().resolve()
VALIDATOR = ROOT / "scripts" / "validate_rustmix_wave_display_backend_import_v0.py"

if not VALIDATOR.exists():
    raise SystemExit("repo validator missing: scripts/validate_rustmix_wave_display_backend_import_v0.py")

subprocess.check_call(["python3", str(VALIDATOR), str(ROOT)])
print("local-rustmix-wave-display-backend-import-v0-validate=ok")
