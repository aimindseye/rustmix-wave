#!/usr/bin/env bash
set -euo pipefail

OUT="${OUT:-/tmp/x4-deploy-ready-check.txt}"
status="ACCEPTED"
reason="DeployReady"

fail() {
  status="REJECTED"
  reason="$1"
}

[ -f Cargo.toml ] || fail "MissingCargoToml"
[ -f rust-toolchain.toml ] || fail "MissingRustToolchain"
[ -f espflash.toml ] || fail "MissingEspflashConfig"
[ -f partitions/xteink_x4_standard.bin ] || fail "MissingPartitionBin"
[ -f target-xteink-x4/Cargo.toml ] || fail "MissingTargetCargoToml"
[ -f target-xteink-x4/src/rustmix_x4/apps/home.rs ] || fail "MissingHomeApp"
[ -f target-xteink-x4/src/rustmix_x4/x4_apps/apps/reader/mod.rs ] || fail "MissingReaderApp"

if ! ./scripts/check_repo_hygiene.sh >/tmp/rustmix-hygiene.log 2>&1; then
  fail "RepoHygieneFailed"
fi

if ! ./scripts/validate_x4_standard_partition_table_compatibility.sh >/tmp/rustmix-partition.log 2>&1; then
  fail "PartitionValidationFailed"
fi

{
  echo "# X4 Deploy Ready Check"
  echo "status=$status"
  echo "reason=$reason"
  echo "target=target-xteink-x4"
  echo "chip=esp32c3"
  echo "flash_size=16MB"
  echo "partition_table=partitions/xteink_x4_standard.bin"
  echo "artifact_script=scripts/build_x4_firmware_artifacts.sh"
  echo "marker=x4-deploy-ready-current-state-ok"
} | tee "$OUT"

echo "Wrote: $OUT"

if [ "$status" != "ACCEPTED" ]; then
  exit 5
fi
