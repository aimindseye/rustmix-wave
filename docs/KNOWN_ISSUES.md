# Known issues and deferred work

## Weather provider reliability

Open-Meteo requests can fail transiently with transport, TLS, timeout, or HTTP service errors. The device already applies bounded retries, delayed backoff, and last-known-good in-memory retention. A cold boot with no successful fetch may still end in a readable `Weather unavailable` state.

## MCU deep sleep

Sleep-image mode suspends network services, sleeps the e-paper panel, and disables the panel rail, but the MCU event loop remains active. This preserves validated AXP2101 Power-key polling and GPIO45 RTC-alarm handling. Full MCU deep sleep remains deferred.

## EPUB scope

Reader supports bounded reflowable text extraction, TOC navigation, bookmarks, and resume. CSS layout, images, hyperlinks, footnotes, fixed-layout EPUB, DRM, ZIP64, and SD-backed EPUB anchor caches remain deferred.

## Calendar scope

Calendar personal events and U.S. holidays are active. U.S. holiday rows remain read-only. Calendar reminders do not automatically create RTC alarms. Non-U.S. calendar packs are intentionally excluded from the native Calendar route.

## Dictionary scope

Dictionary exact and prefix lookup is active through the complete X4 pack. Saved words, search history, and Reader word-selection lookup remain deferred.

## Merged factory-image release artifact

The supported release artifact is the ESP-IDF ELF flashed through `espflash flash`.
Raw-address flashing with `espflash write-bin` is intentionally unsupported. A
merged factory image remains deferred until the bootloader, partition-table, and
application offsets have been validated on physical hardware.
