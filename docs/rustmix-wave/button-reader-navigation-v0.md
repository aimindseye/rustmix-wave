# Rustmix-Wave Button Reader Navigation v0

## Scope

This slice adds real button polling for reader page navigation.

It keeps:

- direct TXT boot flow
- SD TXT reader path
- DisplayBackendAdapter
- ShellDisplayBridge
- ReaderDisplaySurface
- GPIO3 reserved for EPD_BUSY

It adds polling for vendor app buttons:

- GPIO4 Button_Up
- GPIO5 Button_Function
- GPIO6 Button_Down

Buttons use pull-up inputs and are active-low.

## Mapping

- GPIO6 Button_Down = next page
- GPIO4 Button_Up = previous page
- GPIO5 Button_Function = refresh/select current page

GPIO0 Boot is documented only and is not used in this slice.

## Runtime path

boot
  -> SD TXT first page
  -> polling loop
  -> Button_Down renders next page
  -> Button_Up renders previous page
  -> Button_Function refreshes current page

## Markers

- RAW-RUSTMIX-WAVE-BUTTON-NAV-V0-START
- RAW-RUSTMIX-WAVE-BUTTON-NAV-PINS-OK
- RAW-RUSTMIX-WAVE-BUTTON-NAV-READY
- RAW-RUSTMIX-WAVE-BUTTON-DOWN-NEXT
- RAW-RUSTMIX-WAVE-BUTTON-DOWN-NEXT-OK
- RAW-RUSTMIX-WAVE-BUTTON-UP-PREV
- RAW-RUSTMIX-WAVE-BUTTON-UP-PREV-OK
- RAW-RUSTMIX-WAVE-BUTTON-FUNCTION-REFRESH
- RAW-RUSTMIX-WAVE-BUTTON-FUNCTION-REFRESH-OK

## Non-goals

- No interrupts.
- No GPIO0 Boot use.
- No GPIO3 input use.
- No EPUB.
- No bookmark persistence.
- No progress persistence.
