# Rustmix-Wave r1 Wire-up Plan

## Objective

Wire the RRBP parser into Rustmix-Wave as a BLE command receiver with page-next and page-previous only.

## Required GATT pieces

```text
Service: Rustmix Remote Service
Command characteristic: Write Without Response
Status characteristic: Read + Notify, optional for r1
Capabilities characteristic: Read, optional for r1
```

## Safe callback shape

```rust
fn on_remote_command_write(bytes: &[u8]) {
    if let Ok(Some(event)) = parser.parse(bytes) {
        remote_queue.push(event);
    }
}
```

The callback must not call reader page-turn functions directly.

## Main-loop shape

```rust
while let Some(event) = remote_queue.pop() {
    match event {
        RemoteEvent::PageNext => route_existing_next_page_action(),
        RemoteEvent::PagePrevious => route_existing_previous_page_action(),
        _ => {}
    }
}
```

## r1 acceptance test

- Device boots normally.
- Existing buttons still work.
- Watch connects.
- Next sends exactly one page turn.
- Previous sends exactly one page turn.
- Unsupported commands are ignored.
- Duplicate sequence number is ignored.
- No sleep/wake regression.
- No Wi-Fi transfer regression.
