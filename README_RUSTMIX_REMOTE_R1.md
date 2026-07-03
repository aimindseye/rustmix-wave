# Rustmix Remote r1 Firmware Scaffold Overlay

This overlay adds the RRBP parser scaffold for Rustmix-Wave.

Apply from the Rustmix-Wave repo root:

```bash
cd /home/mindseye73/Documents/projects/rustmix-wave
unzip -o ~/Downloads/rustmix-wave-ble-remote-r1-scaffold-overlay.zip -d .
chmod +x scripts/validate_rustmix_remote_rrbp.sh
./scripts/validate_rustmix_remote_rrbp.sh
```

Commit:

```bash
git status
git add firmware/assistant-rs/src/rustmix_remote docs/rustmix-remote scripts/validate_rustmix_remote_rrbp.sh patches/rustmix-wave-main-loop-hook-example.diff README_RUSTMIX_REMOTE_R1.md
git commit -m "feat: add Rustmix Remote RRBP parser scaffold"
git push
```

This scaffold is intentionally non-invasive. The next step is to wire the parser into the BLE write callback and existing main-loop input route.
