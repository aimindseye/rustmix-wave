# Rustmix-Wave Reader Display Surface Boundary v0

## Scope

This slice adds the first reader-facing display boundary.

It does **not** port the reader, does **not** add `ReaderStorage`, does **not**
enable real rotary input, and does **not** reuse GPIO3 for input.

## Runtime path

```text
ReaderDisplaySurface
  -> ShellDisplayBridge
  -> DisplayBackendAdapter
  -> free Waveshare display backend
```

Reader code must not directly own:

- SPI
- DC/RST/BUSY pins
- `DisplayBackendAdapter`
- native display RAM orientation

## Added trait

```rust
pub trait ReaderDisplaySurface {
    fn logical_width(&self) -> u32;
    fn logical_height(&self) -> u32;
    fn clear(&mut self);
    fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, black: bool);
    fn draw_mono_bitmap(&mut self, x: u32, y: u32, w: u32, h: u32, data: &[u8]);
    fn flush(&mut self) -> anyhow::Result<()>;
}
```

## First implementation

`ReaderDisplaySurface` is implemented for `ShellDisplayBridge`.

The logical reader surface is portrait `480x800`. Native `800x480` RAM mapping
stays inside `ShellDisplayBridge`.

## Placeholder smoke

The target renders a static reader placeholder page after the Rustmix-Wave home
UI navigation smoke.

Expected markers:

- `RAW-RUSTMIX-WAVE-READER-BOUNDARY-DEMO-START`
- `RAW-RUSTMIX-WAVE-READER-BOUNDARY-V0-START`
- `RAW-RUSTMIX-WAVE-READER-DISPLAY-PLACEHOLDER-OK`
- `RAW-RUSTMIX-WAVE-READER-BOUNDARY-V0-OK`
- `RAW-RUSTMIX-WAVE-READER-BOUNDARY-DEMO-OK`

## Non-goals

- No reader content parsing.
- No EPUB.
- No TXT loading.
- No SD/storage adapter.
- No bookmarks/progress.
- No real rotary input.
- No GPIO3 input usage.
