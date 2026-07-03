# Rustmix-Wave BLE GATT Service r1

## Goal

Expose a feature-gated BLE GATT service named `Rustmix Remote` on Rustmix-Wave.
The Wear OS app writes 6-byte RRBP v1 packets to the command characteristic.
Firmware parses the packet, enqueues a `RemoteEvent`, and the main loop drains
that queue safely.

## Safety policy

BLE callbacks must not mutate reader/UI state. The callback only:

1. checks the target characteristic handle;
2. parses the 6-byte RRBP packet;
3. enqueues a `RemoteEvent`; and
4. returns.

The main firmware loop owns all UI transitions and display refreshes.

## r1 scope

Implemented r1 commands:

- `PageNext` maps to the existing `ButtonEvent::Down` reader-page action.
- `PagePrevious` maps to the existing `ButtonEvent::Up` reader-page action.

Ignored in r1:

- Select
- Back
- Menu
- Sleep
- Wake
- Scroll
- Bookmark
- Refresh

Unsupported commands are logged and ignored.

## Feature flag

The BLE code is compiled only with:

```bash
cargo build --release --features rustmix-remote-ble
```

Default builds do not compile or start the BLE GATT service.

## Modem ownership in r1

Rustmix-Wave currently uses `peripherals.modem` for Wi-Fi through
`NetworkRuntime::connect(...)`. The ESP-IDF BLE wrapper also needs the modem.
For this r1 hardware validation build, `--features rustmix-remote-ble` gives BLE
ownership of the modem and skips Wi-Fi startup.

This is intentional for r1 so default firmware behavior remains unchanged.
A later r2 can revisit Wi-Fi/BLE coexistence or a runtime mode switch.

## GATT layout

Service UUID:

```text
8f7a0000-6b8f-4a91-9e2c-727573740001
```

Command characteristic UUID:

```text
8f7a0001-6b8f-4a91-9e2c-727573740001
```

Command payload length:

```text
6 bytes
```

## RRBP command payload

```text
Byte 0: version, always 0x01 for RRBP v1
Byte 1: sequence
Byte 2: command
Byte 3: flags
Byte 4: parameter
Byte 5: reserved
```

## Test commands from watch/client

```text
01 00 01 00 00 00  # page next
01 01 02 00 00 00  # page previous
```

## Expected logs

```text
rustmix-wave=rustmix-remote-ble status=ready feature=rustmix-remote-ble
rustmix-wave=rustmix-remote-ble status=advertising name=Rustmix Remote
rustmix-wave=rustmix-remote-command status=enqueued ... bytes=[01, 00, 01, 00, 00, 00]
rustmix-wave=rustmix-remote-event event=page-next route=reader-page
```
