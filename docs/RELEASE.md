# Release generation

Rustmix Wave has three release helpers:

```text
scripts/build-release-firmware.sh  Build an ELF-only firmware release bundle
scripts/flash-release.sh           Flash an existing release ELF safely
scripts/package-release.sh         Package the cleaned GitHub-ready source tree
```

## Supported release artifact

The supported firmware artifact is the ESP-IDF release ELF:

```text
dist/waveshare-epd397-rust-app-v<VERSION>.elf
```

Flash it with the ELF-aware `espflash flash` command:

```bash
./scripts/flash-release.sh \
  dist/waveshare-epd397-rust-app-v<VERSION>.elf
```

Equivalent direct command:

```bash
espflash flash --chip esp32s3 --monitor \
  dist/waveshare-epd397-rust-app-v<VERSION>.elf
```

Ordinary development flashing remains unchanged:

```bash
./scripts/flash.sh monitor
```


## Explicit Xtensa target safety

The development flasher and release builder force:

```bash
cargo +esp build -Z build-std=std,panic_abort --release --target xtensa-esp32s3-espidf
```

They then select Cargo's `compiler-artifact.executable` only when its path is rooted under the requested `xtensa-esp32s3-espidf` target directory. A host artifact such as `target/release/waveshare-epd397-rust-app` is rejected because it is not an ESP-IDF firmware ELF and does not contain the ESP-IDF application descriptor required by `espflash`.

## Safety warning: do not use raw-address flashing

Do **not** use `espflash write-bin` for the release ELF or for any artifact from
this repository. `write-bin` is a raw-address operation. A raw write requires an
explicitly validated flash layout and correct bootloader, partition-table, and
application offsets.

The earlier unverified `*-flash.bin` artifact and the `write-bin ... 0x0`
workflow have been removed.

## Future merged factory image

A merged factory-image workflow remains deferred. It may be added only after all
of the following have been validated on physical hardware:

- Bootloader offset and image
- Partition-table offset and image
- Factory application partition offset
- Flash mode, frequency, and size
- Recovery procedure from ROM download mode

Until then, the ELF-aware `espflash flash` path is the only supported release
installation method.

## Build a firmware release

```bash
./scripts/build-release-firmware.sh
```

The script:

1. Runs `./scripts/validate.sh` unless `--skip-validate` is provided.
2. Builds the embedded release ELF with an explicit `cargo +esp build -Z build-std=std,panic_abort --release --target xtensa-esp32s3-espidf` target through `scripts/resolve-built-elf.sh`.
3. Copies the ELF into `dist/`.
4. Copies the safe `flash-release.sh` helper into `dist/`.
5. Writes SHA-256 checksums.
6. Generates a release ZIP containing the ELF, flashing helper, checksum
   manifest, and flashing instructions.

Output naming:

```text
dist/waveshare-epd397-rust-app-v<VERSION>.elf
dist/waveshare-epd397-rust-app-v<VERSION>-flash-release.sh
dist/waveshare-epd397-rust-app-v<VERSION>-firmware-release.sha256
dist/waveshare-epd397-rust-app-v<VERSION>-FLASHING.txt
dist/waveshare-epd397-rust-app-v<VERSION>-firmware-release.zip
```

No `*-flash.bin` artifact is generated.

## Skip validation during a repeated local build

```bash
./scripts/build-release-firmware.sh --skip-validate
```

Use this only after the exact source tree has already passed
`./scripts/validate.sh`.

## Optional Indic Reader font packs

The ELF release does not embed or redistribute raw Noto font files. Generate Devanagari and Gujarati `.RWF` packs locally with `tools/font-builder/index.html`, then install them under `/RUSTMIX/FONTS`. See [`UNICODE_FONTS.md`](UNICODE_FONTS.md).

## Package cleaned source

```bash
./scripts/package-release.sh
```

This produces:

```text
dist/waveshare-epd397-rust-app-v<VERSION>-github-ready.zip
dist/waveshare-epd397-rust-app-v<VERSION>-github-ready.zip.sha256
```

The source package excludes Git metadata, build outputs, generated release
artifacts, local caches, patch scratch files, and extracted overlay directories.
## Cargo target-directory safety

The development flash script and release builder resolve the compiled ESP32-S3 ELF from Cargo's authoritative `compiler-artifact.executable` JSON field. They do not reconstruct an assumed path from `target_directory`. This remains correct when Cargo uses a redirected target directory, a workspace setting, a nonstandard artifact layout, or a hashed executable path. Flashing a hard-coded stale ELF can make a successful source build appear to have no effect on hardware.

After `./scripts/flash.sh monitor`, verify the reported `development-flash-elf=<path>` and the boot marker version.

