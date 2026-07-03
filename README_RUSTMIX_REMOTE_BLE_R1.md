# Rustmix Remote BLE GATT r1 Overlay

This overlay promotes the previous RRBP parser scaffold into a feature-gated
Rustmix-Wave BLE GATT r1 service.

## What changes

- Adds live `src/rustmix_remote` module.
- Adds `rustmix-remote-ble` Cargo feature.
- Adds ESP-IDF Bluedroid GATT server module behind that feature.
- Adds a `RemoteEventQueue` callback boundary.
- Wires main loop to consume only `PageNext` and `PagePrevious` while on the
  Reader page.
- Leaves default firmware builds unchanged.

## r1 limitation

When built with `--features rustmix-remote-ble`, BLE owns `peripherals.modem`, so
Wi-Fi/network services are skipped in that feature build. Default builds are not
changed.
