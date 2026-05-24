# Repository Working Notes

## Current priority

Keep the Xteink X4 reader path stable. Prefer small, device-testable changes that preserve reader pagination, SD-card behavior, Wi-Fi Transfer, Date & Time, and the accepted partition table.

## Active runtime

The active firmware target is `target-xteink-x4`. New functionality should be added under Rustmix-owned source paths, especially `target-xteink-x4/src/rustmix_x4/**`, `core/**`, `hal-xteink-x4/**`, `support/**`, `tools/**`, and `examples/sd-card/**`.

`vendor/pulp-os` is reference/compatibility material only. Do not add new product behavior there.

## UI direction

Home remains a Biscuit-style category launcher. Internal pages use CrossInk-style chrome with fixed Inter UI typography. Reader/book fonts remain separate from OS chrome.

## Naming and artifacts

Use semantic names. Do not commit generated archives, temporary helper folders, root build output, OS metadata, or one-time script artifacts.

## Validation

For repository checks, run:

```bash
scripts/check_repo_hygiene.sh
scripts/audit_remaining_pulp_runtime_dependencies.sh
scripts/validate_x4_standard_partition_table_compatibility.sh
scripts/deploy/check_deploy_ready.sh
```

For firmware checks, run:

```bash
cargo fmt --all
cargo check -p target-xteink-x4 --target riscv32imc-unknown-none-elf
cargo build -p target-xteink-x4 --release --target riscv32imc-unknown-none-elf
```
