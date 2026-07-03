# Rustmix Remote Firmware Scaffold

This overlay adds a small, non-invasive RRBP parser scaffold for Rustmix-Wave.

It does not modify the accepted main loop, reader code, BLE stack setup, or routing files.

## Files

```text
firmware/assistant-rs/src/rustmix_remote/mod.rs
firmware/assistant-rs/src/rustmix_remote/rrbp.rs
firmware/assistant-rs/src/rustmix_remote/integration_stub.rs
scripts/validate_rustmix_remote_rrbp.sh
```

## Validate parser scaffold

```bash
./scripts/validate_rustmix_remote_rrbp.sh
```

## Next manual integration step

After the parser scaffold validates, wire it into the Rustmix-Wave BLE stack behind a feature gate:

```text
RUSTMIX_WAVE_ENABLE_BLE_REMOTE=1
```

The BLE write callback should call the RRBP parser and enqueue the resulting `RemoteEvent`.
The main UI loop should drain that queue and route page next/previous through the existing physical-button navigation path.
