# Generated bitmap-font notices

RustMix Wave embeds generated 1-bpp printable-ASCII bitmap atlases derived from:

- Inter Medium and Inter SemiBold
- Atkinson Hyperlegible Regular and Atkinson Hyperlegible Bold

Compact, Standard and Large raster strikes are generated for the physical e-paper panel. The repository contains only generated Rust arrays in `src/app/typography/assets.rs`; raw font files are intentionally not distributed.

The source families are distributed under the SIL Open Font License 1.1. The license text is retained in `docs/licenses/OFL-1.1.txt`.


RustMix Wave v0.16.2 also embeds generated printable-ASCII Reader-page raster strikes derived from DejaVu Serif. The repository contains only generated Rust arrays in `src/app/reader_serif_assets.rs`; the raw DejaVu font file is intentionally not distributed. DejaVu changes are public domain and the base Bitstream Vera permission notice is retained in `docs/licenses/DEJAVU-SERIF-NOTICE.txt`.


RustMix Wave v0.17.2 adds Reader-only generated printable-ASCII raster strikes derived from Atkinson Hyperlegible Next Medium and Literata Medium. The generated Rust arrays live in `src/app/reader_atkinson_next_assets.rs` and `src/app/reader_literata_assets.rs`. The existing persisted Reader preference keys `atkinson-hyperlegible` and `serif` remain unchanged; `literata` is a new explicit key. Raw font files are intentionally not distributed. Both new source families are used under the SIL Open Font License 1.1 retained in `docs/licenses/OFL-1.1.txt`.

## Optional local Noto Sans Indic inputs

Rustmix Wave v1.1 can generate SD Reader packs from locally supplied **Noto Sans Devanagari** and **Noto Sans Gujarati** files. Raw Noto font files are not distributed in this repository or copied to the device. Users obtain the fonts separately and run the local browser builder. Noto fonts are distributed under the SIL Open Font License 1.1; retain the applicable font license notice with any redistributed generated assets.
