# Rustmix Scope

## In scope

Rustmix is scoped as an Xteink X4 reader-first firmware.

Current in-scope areas:

- X4 boot, display, input, SD storage, reader, settings, network, and sleep-image behavior.
- X4/CrossPoint-compatible partition table preservation.
- TXT and EPUB reading with visible loading/cache progress, bookmarks, title cache, prepared cache metadata, reader settings, and footer-safe pagination.
- Bionic Reading, Guide Dots, sunlight-fading mitigation, and reader font support.
- Biscuit-style Home dashboard with CrossInk-style internal pages.
- Wi-Fi setup, Wi-Fi Transfer, network time, and cached/live Date & Time behavior.
- Optional Lua apps under `/RUSTMIX/APPS`.
- Dictionary prefix-shard SD data model.
- Calendar with native month grid and SD-loaded event files.
- Games catalog and game shells with consistent footer-safe UI.
- Firmware artifact creation for GitHub Actions and new-device installation.

## Repository scope

Keep source control focused on the product/runtime baseline:

- root Rust workspace
- active X4 target
- SD-card examples
- host data-generation tools
- install and validation scripts
- consolidated root documentation
- GitHub workflows for checks and firmware artifacts

Do not commit generated archives, extracted temporary folders, local build output, OS metadata, or one-time helper scripts.

## Vendor scope

`vendor/pulp-os` may remain for reference and compatibility comparison. New Rustmix functionality belongs under Rustmix-owned paths.

`vendor/smol-epub` remains as the EPUB dependency source.

## Deferred

- Broad app store or plugin ecosystem.
- OPDS and cloud-only flows.
- OTA hardening beyond the current app0/new-device artifact path.
- Waveshare/S3 implementation.
- Palm/Tern compatibility host.
- Sync transport until local reader state is stable.
- Mutable `.vchk` state until package read/open semantics are stable.


## Lua Text App Safe Display Scope

Marker: `lua-text-app-safe-display-scope`

Lua text app display is in scope for shared header/footer-safe wrapping and Inter UI typography.  SD app data formats and runtime loading paths remain unchanged.
