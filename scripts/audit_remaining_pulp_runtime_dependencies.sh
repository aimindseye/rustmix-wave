#!/usr/bin/env bash
set -euo pipefail

# vendor/pulp-os may remain as reference/compatibility material. This audit only
# rejects active package/Cargo dependencies on the old Pulp runtime.

if rg -n 'pulp_os::|package = "x4-os"|x4-kernel =' Cargo.toml target-xteink-x4/Cargo.toml target-xteink-x4/src; then
  echo "ERROR: active old Pulp runtime package reference remains" >&2
  exit 1
fi

if rg -n 'vendor/pulp-os' Cargo.toml target-xteink-x4/Cargo.toml target-xteink-x4/src; then
  echo "ERROR: active source/Cargo path references vendor/pulp-os" >&2
  exit 1
fi

echo "rustmix_active_pulp_runtime_dependency_audit=ok"
