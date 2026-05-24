# Rustmix Roadmap

## Principles

1. Protect the reading path.
2. Keep the Xteink X4 / CrossPoint partition table stable.
3. Keep Home simple and keep internal pages visually consistent.
4. Keep reader/book fonts separate from fixed OS UI typography.
5. Keep optional Lua apps bounded to SD-loaded app data.
6. Freeze local reader state before adding sync behavior.
7. Treat XTC as compatibility/import support.
8. Treat `.vchk` as the Rustmix-native package format.

## Near-term work

### Reader Home and Library polish

- Improve Continue Reading.
- Stabilize recent-book ordering.
- Improve title/metadata display.
- Improve broken-file and missing-cache handling.
- Continue validating long-book open/resume responsiveness on device.

### Reader data model freeze

Freeze and document:

- book identity
- reading progress
- bookmarks
- highlights
- per-book settings
- library entries
- content format and content location anchors

### XTC compatibility

- Detect XTC files.
- Open XTC packages.
- Render through the existing reader path.
- Persist progress and bookmarks against stable package identity.

### Rustmix `.vchk` package

- Finalize package header and manifest.
- Define content payload rules.
- Define navigation metadata.
- Define embedded reader-state section.
- Define sync metadata section.

### `.vchk` read/open support

- Validate package.
- Load manifest.
- Load content and navigation metadata.
- Render through the existing reader path.

### Mutable `.vchk` state

- Update progress.
- Add and remove bookmarks.
- Add and remove highlights.
- Persist revisions safely.

### Sync alignment

- Define sync object IDs.
- Define revision semantics.
- Define offline-first local mutation queue.
- Serialize progress, bookmark, and highlight state.

## Later work

- Stronger OTA and release-channel workflow.
- Additional app data packs.
- Expanded font packs.
- Additional hardware targets after X4 remains stable.
