---
name: X4 HAL extraction task
about: Track one seam of Xteink X4 HAL extraction into Rustmix
title: "[X4 HAL] "
labels: ["x4", "hal", "extraction"]
assignees: []
---

## Scope

Which seam is this task for?

- [ ] Power
- [ ] Input
- [ ] Storage
- [ ] Display contract
- [ ] Display driver
- [ ] Target runtime

## Source in x4-reader-os-rs

List source files / modules:

```text

```

## Destination in rustmix-x4-firmware

List target files / modules:

```text

```

## What must be preserved

- [ ] documented hardware fact
- [ ] existing unit test or new unit test
- [ ] serial log evidence if hardware behavior is involved
- [ ] no app policy mixed into HAL

## Acceptance criteria

- [ ] `cargo fmt --all --check`
- [ ] `cargo check --workspace --all-targets`
- [ ] `cargo test --workspace --all-targets`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] source map updated if needed
- [ ] validation matrix updated if needed

## Notes / risks

```text

```
