//! EPUB chapter cache: streaming decompress + HTML strip pipeline.
//!
//! No persistent heap; ≈ 51 KB temporary per chapter.
//!
//! v3 unified cache format: single file per book (`_XXXXXXX.BIN`)
//! containing a fixed 128-byte header, chapter offset table,
//! concatenated chapter text, image index, and image data.
//!
//! [`strip_html_buf`] provides an in-memory variant that strips
//! already-decompressed XHTML, suitable for offloading to a
//! background task when the caller has pre-extracted the ZIP entry.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::html_strip::HtmlStripStream;
use crate::zip::{METHOD_DEFLATE, METHOD_STORED, ZipEntry, ZipIndex};

const CACHE_MAGIC: u32 = 0x504C_5043; // "PLPC"
const CACHE_VERSION: u8 = 2;
const META_HEADER: usize = 16;

/// Maximum number of chapters that can be tracked in a single cache.
pub const MAX_CACHE_CHAPTERS: usize = 256;
/// Maximum byte size of a `META.BIN` file (header + one `u32` per chapter).
pub const META_MAX_SIZE: usize = META_HEADER + 4 * MAX_CACHE_CHAPTERS;

const WINDOW_SIZE: usize = 32768; // DEFLATE sliding window
const READ_BUF_SIZE: usize = 4096; // compressed read chunk
const STRIP_BUF_SIZE: usize = 4096; // strip output accumulator
const FLUSH_THRESHOLD: usize = STRIP_BUF_SIZE - 128;

/// Compute the FNV-1a hash of `data`.
#[inline]
pub fn fnv1a(data: &[u8]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for &b in data {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// Compute the FNV-1a hash of `data` with ASCII case folding.
///
/// Identical to [`fnv1a`] except each byte is lowercased before
/// hashing, so `b"FOO.EPUB"` and `b"foo.epub"` produce the same
/// result.  Used by the bookmark cache for case-insensitive filename
/// matching on FAT filesystems.
#[inline]
pub fn fnv1a_icase(data: &[u8]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for &b in data {
        h ^= b.to_ascii_lowercase() as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

// -- v3 unified cache format --
//
// single file per book: _XXXXXXX.BIN under _PULP/
//
// layout:
//   [0..128)     fixed header (see HEADER_SIZE)
//   [128..128+N*8) chapter offset table (N = chapter_count)
//   [...]        concatenated chapter text data
//   [...]        image index: u16 count + 12 bytes per entry
//   [...]        concatenated image data (4-byte w/h header + 1bpp pixels)

#[allow(missing_docs)]
pub const HEADER_SIZE: usize = 128;
#[allow(missing_docs)]
pub const CACHE_V3: u8 = 3;
#[allow(missing_docs)]
pub const TITLE_CAP: usize = 48;
#[allow(missing_docs)]
pub const NAME_CAP: usize = 13;

// header flag bits
#[allow(missing_docs)]
pub const FLAG_CHAPTERS_COMPLETE: u8 = 1 << 0;
#[allow(missing_docs)]
pub const FLAG_IMAGES_COMPLETE: u8 = 1 << 1;

// per-chapter table entry: offset(u32) + size(u32) = 8 bytes
#[allow(missing_docs)]
pub const CHAPTER_ENTRY_SIZE: usize = 8;
// per-image index entry: path_hash(u32) + offset(u32) + size(u32) = 12 bytes
#[allow(missing_docs)]
pub const IMAGE_ENTRY_SIZE: usize = 12;
// image data header: width(u16) + height(u16) = 4 bytes
#[allow(missing_docs)]
pub const IMAGE_DATA_HEADER: usize = 4;

// parsed v3 header
#[allow(missing_docs)]
#[derive(Clone, Copy)]
pub struct CacheHeader {
    pub version: u8,
    pub chapter_count: u16,
    pub flags: u8,
    pub epub_size: u32,
    pub name_hash: u32,
    pub title_len: u8,
    pub title: [u8; TITLE_CAP],
    pub name_len: u8,
    pub name: [u8; NAME_CAP],
}

#[allow(missing_docs)]
impl CacheHeader {
    pub const fn empty() -> Self {
        Self {
            version: 0,
            chapter_count: 0,
            flags: 0,
            epub_size: 0,
            name_hash: 0,
            title_len: 0,
            title: [0u8; TITLE_CAP],
            name_len: 0,
            name: [0u8; NAME_CAP],
        }
    }

    #[inline]
    pub fn title_str(&self) -> &str {
        core::str::from_utf8(&self.title[..self.title_len as usize]).unwrap_or("")
    }

    #[inline]
    pub fn chapters_complete(&self) -> bool {
        self.flags & FLAG_CHAPTERS_COMPLETE != 0
    }

    // byte offset where the chapter offset table starts
    #[inline]
    pub fn table_offset(&self) -> u32 {
        HEADER_SIZE as u32
    }

    // byte offset where chapter data starts (after the offset table)
    #[inline]
    pub fn data_offset(&self) -> u32 {
        HEADER_SIZE as u32 + self.chapter_count as u32 * CHAPTER_ENTRY_SIZE as u32
    }
}

// encode v3 header into a 128-byte buffer
#[allow(missing_docs)]
pub fn encode_v3_header(h: &CacheHeader, buf: &mut [u8; HEADER_SIZE]) {
    *buf = [0u8; HEADER_SIZE];
    buf[0..4].copy_from_slice(&CACHE_MAGIC.to_le_bytes());
    buf[4] = CACHE_V3;
    buf[5..7].copy_from_slice(&h.chapter_count.to_le_bytes());
    buf[7] = h.flags;
    buf[8..12].copy_from_slice(&h.epub_size.to_le_bytes());
    buf[12..16].copy_from_slice(&h.name_hash.to_le_bytes());
    buf[16] = h.title_len;
    let tlen = h.title_len as usize;
    buf[17..17 + tlen].copy_from_slice(&h.title[..tlen]);
    buf[65] = h.name_len;
    let nlen = h.name_len as usize;
    buf[66..66 + nlen].copy_from_slice(&h.name[..nlen]);
}

// parse v3 header from a 128-byte buffer
#[allow(missing_docs)]
pub fn parse_v3_header(buf: &[u8; HEADER_SIZE]) -> Result<CacheHeader, &'static str> {
    let magic = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    if magic != CACHE_MAGIC {
        return Err("cache: bad magic");
    }
    let version = buf[4];
    if version != CACHE_V3 {
        return Err("cache: version mismatch (expected v3)");
    }
    let chapter_count = u16::from_le_bytes([buf[5], buf[6]]);
    let flags = buf[7];
    let epub_size = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
    let name_hash = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
    let title_len = buf[16].min(TITLE_CAP as u8);
    let mut title = [0u8; TITLE_CAP];
    title[..title_len as usize].copy_from_slice(&buf[17..17 + title_len as usize]);
    let name_len = buf[65].min(NAME_CAP as u8);
    let mut name = [0u8; NAME_CAP];
    name[..name_len as usize].copy_from_slice(&buf[66..66 + name_len as usize]);

    Ok(CacheHeader {
        version,
        chapter_count,
        flags,
        epub_size,
        name_hash,
        title_len,
        title,
        name_len,
        name,
    })
}

// validate a parsed header against the current epub
#[allow(missing_docs)]
pub fn validate_v3_header(
    h: &CacheHeader,
    epub_size: u32,
    name_hash: u32,
    expected_chapters: usize,
) -> Result<(), &'static str> {
    if h.epub_size != epub_size {
        return Err("cache: epub size changed");
    }
    if h.name_hash != name_hash {
        return Err("cache: epub hash changed");
    }
    if h.chapter_count as usize != expected_chapters {
        return Err("cache: chapter count mismatch");
    }
    Ok(())
}

// encode chapter offset table: [(offset, size); count]
#[allow(missing_docs)]
pub fn encode_chapter_table(entries: &[(u32, u32)], buf: &mut [u8]) {
    for (i, &(offset, size)) in entries.iter().enumerate() {
        let base = i * CHAPTER_ENTRY_SIZE;
        buf[base..base + 4].copy_from_slice(&offset.to_le_bytes());
        buf[base + 4..base + 8].copy_from_slice(&size.to_le_bytes());
    }
}

// parse chapter offset table from buffer
#[allow(missing_docs)]
pub fn parse_chapter_table(
    buf: &[u8],
    count: usize,
    out: &mut [(u32, u32)],
) -> Result<(), &'static str> {
    let needed = count * CHAPTER_ENTRY_SIZE;
    if buf.len() < needed {
        return Err("cache: chapter table truncated");
    }
    for i in 0..count {
        let base = i * CHAPTER_ENTRY_SIZE;
        let offset = u32::from_le_bytes([buf[base], buf[base + 1], buf[base + 2], buf[base + 3]]);
        let size = u32::from_le_bytes([buf[base + 4], buf[base + 5], buf[base + 6], buf[base + 7]]);
        out[i] = (offset, size);
    }
    Ok(())
}

// image index entry
#[allow(missing_docs)]
#[derive(Clone, Copy, Default)]
pub struct ImageIndexEntry {
    pub path_hash: u32,
    pub offset: u32,
    pub size: u32,
}

// encode image index: u16 count + entries
#[allow(missing_docs)]
pub fn encode_image_index(entries: &[ImageIndexEntry], buf: &mut [u8]) -> usize {
    let count = entries.len() as u16;
    buf[0..2].copy_from_slice(&count.to_le_bytes());
    let mut pos = 2;
    for e in entries {
        buf[pos..pos + 4].copy_from_slice(&e.path_hash.to_le_bytes());
        buf[pos + 4..pos + 8].copy_from_slice(&e.offset.to_le_bytes());
        buf[pos + 8..pos + 12].copy_from_slice(&e.size.to_le_bytes());
        pos += IMAGE_ENTRY_SIZE;
    }
    pos
}

// parse image index, returns count
#[allow(missing_docs)]
pub fn parse_image_index(buf: &[u8], out: &mut [ImageIndexEntry]) -> Result<usize, &'static str> {
    if buf.len() < 2 {
        return Err("cache: image index too short");
    }
    let count = u16::from_le_bytes([buf[0], buf[1]]) as usize;
    let needed = 2 + count * IMAGE_ENTRY_SIZE;
    if buf.len() < needed || count > out.len() {
        return Err("cache: image index truncated or too large");
    }
    for i in 0..count {
        let base = 2 + i * IMAGE_ENTRY_SIZE;
        out[i] = ImageIndexEntry {
            path_hash: u32::from_le_bytes([buf[base], buf[base + 1], buf[base + 2], buf[base + 3]]),
            offset: u32::from_le_bytes([
                buf[base + 4],
                buf[base + 5],
                buf[base + 6],
                buf[base + 7],
            ]),
            size: u32::from_le_bytes([
                buf[base + 8],
                buf[base + 9],
                buf[base + 10],
                buf[base + 11],
            ]),
        };
    }
    Ok(count)
}

// generate 8.3 cache filename from hash: _XXXXXXX.BIN
#[allow(missing_docs)]
pub fn cache_filename(name_hash: u32) -> [u8; 12] {
    let h = name_hash & 0x0FFF_FFFF;
    let mut buf = [0u8; 12];
    buf[0] = b'_';
    for i in 0..7 {
        let nibble = ((h >> (24 - i * 4)) & 0xF) as u8;
        buf[1 + i] = if nibble < 10 {
            b'0' + nibble
        } else {
            b'A' + nibble - 10
        };
    }
    buf[8] = b'.';
    buf[9] = b'B';
    buf[10] = b'I';
    buf[11] = b'N';
    buf
}

#[allow(missing_docs)]
#[inline]
pub fn cache_filename_str(buf: &[u8; 12]) -> &str {
    core::str::from_utf8(buf).unwrap_or("_0000000.BIN")
}

/// Generate an 8.3-safe cache directory name from a hash.
///
/// Format: `_` followed by 7 uppercase hex digits of the lower 28 bits.
pub fn dir_name_for_hash(name_hash: u32) -> [u8; 8] {
    let h = name_hash & 0x0FFF_FFFF;
    let mut buf = [0u8; 8];
    buf[0] = b'_';
    for i in 0..7 {
        let nibble = ((h >> (24 - i * 4)) & 0xF) as u8;
        buf[1 + i] = if nibble < 10 {
            b'0' + nibble
        } else {
            b'A' + nibble - 10
        };
    }
    buf
}

/// Interpret an 8-byte directory name buffer as a UTF-8 `&str`.
#[inline]
pub fn dir_name_str(buf: &[u8; 8]) -> &str {
    core::str::from_utf8(buf).unwrap_or("_0000000")
}

/// Generate an 8.3-safe chapter filename: `CH000.TXT` through `CH255.TXT`.
pub fn chapter_file_name(idx: u16) -> [u8; 9] {
    debug_assert!(idx < 1000, "chapter index out of 3-digit range");
    let mut n = *b"CH000.TXT";
    n[2] = b'0' + ((idx / 100) % 10) as u8;
    n[3] = b'0' + ((idx / 10) % 10) as u8;
    n[4] = b'0' + (idx % 10) as u8;
    n
}

/// Interpret a 9-byte chapter filename buffer as a UTF-8 `&str`.
#[inline]
pub fn chapter_file_str(buf: &[u8; 9]) -> &str {
    core::str::from_utf8(buf).unwrap_or("CH000.TXT")
}

/// Filename used for the cache metadata file.
pub const META_FILE: &str = "META.BIN";

/// Encode cache metadata into `buf`; returns the number of bytes written.
///
/// The metadata header stores a magic value, version, the EPUB file size,
/// a name hash, and a `u32` size for each cached chapter.
pub fn encode_cache_meta(
    epub_size: u32,
    name_hash: u32,
    chapter_sizes: &[u32],
    buf: &mut [u8],
) -> usize {
    let count = chapter_sizes.len().min(MAX_CACHE_CHAPTERS);
    let total = META_HEADER + count * 4;
    debug_assert!(
        buf.len() >= total,
        "meta buffer too small: {} < {}",
        buf.len(),
        total
    );

    buf[0..4].copy_from_slice(&CACHE_MAGIC.to_le_bytes());
    buf[4] = CACHE_VERSION;
    // count stored as u16 LE in bytes [5..7); supports up to 65535 chapters
    buf[5..7].copy_from_slice(&(count as u16).to_le_bytes());
    buf[7] = 0;
    buf[8..12].copy_from_slice(&epub_size.to_le_bytes());
    buf[12..16].copy_from_slice(&name_hash.to_le_bytes());

    for (i, &size) in chapter_sizes.iter().enumerate().take(count) {
        let off = META_HEADER + i * 4;
        buf[off..off + 4].copy_from_slice(&size.to_le_bytes());
    }

    total
}

/// Parse and validate a `META.BIN` blob.
///
/// On success, writes individual chapter sizes into `chapter_sizes_out`
/// and returns the number of chapters. Returns an error if the magic,
/// version, EPUB size, name hash, or chapter count do not match.
pub fn parse_cache_meta(
    data: &[u8],
    epub_size: u32,
    name_hash: u32,
    expected_chapters: usize,
    chapter_sizes_out: &mut [u32],
) -> Result<usize, &'static str> {
    if data.len() < META_HEADER {
        return Err("cache: meta too short");
    }

    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != CACHE_MAGIC {
        return Err("cache: bad magic");
    }

    let version = data[4];
    if version != CACHE_VERSION && version != 1 {
        return Err("cache: version mismatch");
    }

    let stored_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    let stored_hash = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);

    if stored_size != epub_size {
        return Err("cache: epub size changed");
    }
    if stored_hash != name_hash {
        return Err("cache: epub hash changed");
    }

    // v1 stored count as u8 in byte [5]; v2 stores as u16 LE in [5..7)
    let count = if version >= 2 {
        u16::from_le_bytes([data[5], data[6]]) as usize
    } else {
        data[5] as usize
    };
    if count != expected_chapters {
        return Err("cache: chapter count mismatch");
    }

    let needed = META_HEADER + count * 4;
    if data.len() < needed {
        return Err("cache: meta truncated");
    }

    if chapter_sizes_out.len() < count {
        return Err("cache: output slice too small");
    }

    for i in 0..count {
        let off = META_HEADER + i * 4;
        chapter_sizes_out[i] =
            u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
    }

    Ok(count)
}

/// Stream-decompress a ZIP entry, strip HTML, and emit plain-text chunks.
///
/// `read_fn(offset, buf)` reads raw bytes from the underlying store.
/// `output_fn(chunk)` receives stripped plain-text output incrementally.
///
/// Returns the total number of bytes written through `output_fn`.
/// Peak temporary memory ≈ 47 KB (decompressor + sliding window + strip
/// buffers).
pub fn stream_strip_entry<E>(
    entry: &ZipEntry,
    local_offset: u32,
    mut read_fn: impl FnMut(u32, &mut [u8]) -> Result<usize, E>,
    mut output_fn: impl FnMut(&[u8]) -> Result<(), &'static str>,
) -> Result<u32, &'static str> {
    // skip local file header to reach entry data
    let mut header = [0u8; 30];
    read_fn(local_offset, &mut header).map_err(|_| "cache: read local header failed")?;
    let skip = ZipIndex::local_header_data_skip(&header)?;
    let data_offset = local_offset + skip;

    match entry.method {
        METHOD_STORED => stream_stored(entry, data_offset, &mut read_fn, &mut output_fn),
        METHOD_DEFLATE => stream_deflate(entry, data_offset, &mut read_fn, &mut output_fn),
        _ => Err("cache: unsupported compression method"),
    }
}

// stored entry: read raw, strip HTML, write via callback; stack-only
fn stream_stored<E>(
    entry: &ZipEntry,
    data_offset: u32,
    read_fn: &mut impl FnMut(u32, &mut [u8]) -> Result<usize, E>,
    output_fn: &mut impl FnMut(&[u8]) -> Result<(), &'static str>,
) -> Result<u32, &'static str> {
    let mut stripper = HtmlStripStream::new();
    let mut read_buf = [0u8; READ_BUF_SIZE];
    let mut strip_buf = [0u8; STRIP_BUF_SIZE];
    let mut strip_pos: usize = 0;
    let mut total_written: u32 = 0;

    let size = entry.uncomp_size;
    let mut file_pos = data_offset;
    let mut remaining = size;

    log::info!("cache: streaming stored entry ({} bytes)", size);

    while remaining > 0 {
        let want = (remaining as usize).min(READ_BUF_SIZE);
        let n =
            read_fn(file_pos, &mut read_buf[..want]).map_err(|_| "cache: read failed (stored)")?;
        if n == 0 {
            return Err("cache: unexpected EOF in stored entry");
        }
        file_pos += n as u32;
        remaining -= n as u32;

        feed_and_flush(
            &mut stripper,
            &read_buf[..n],
            &mut strip_buf,
            &mut strip_pos,
            &mut total_written,
            output_fn,
        )?;
    }

    // flush trailing stripper state (deferred newlines, etc.)
    let trailing = stripper.finish(&mut strip_buf[strip_pos..]);
    strip_pos += trailing;
    if strip_pos > 0 {
        output_fn(&strip_buf[..strip_pos])?;
        total_written += strip_pos as u32;
    }

    Ok(total_written)
}

// deflate entry: decompress into 32KB circular window, strip HTML; ~47KB temp
fn stream_deflate<E>(
    entry: &ZipEntry,
    data_offset: u32,
    read_fn: &mut impl FnMut(u32, &mut [u8]) -> Result<usize, E>,
    output_fn: &mut impl FnMut(&[u8]) -> Result<(), &'static str>,
) -> Result<u32, &'static str> {
    use miniz_oxide::inflate::TINFLStatus;
    use miniz_oxide::inflate::core::{DecompressorOxide, decompress, inflate_flags};

    let comp_size = entry.comp_size as usize;
    let uncomp_size = entry.uncomp_size;

    log::info!(
        "cache: streaming deflate {} -> {} bytes",
        comp_size,
        uncomp_size
    );

    // ~11KB DecompressorOxide; alloc zeroed directly (Box::new overflows stack)

    let decomp_ptr =
        unsafe { alloc::alloc::alloc_zeroed(core::alloc::Layout::new::<DecompressorOxide>()) };
    if decomp_ptr.is_null() {
        return Err("cache: OOM for decompressor");
    }
    let mut decomp = unsafe { Box::from_raw(decomp_ptr as *mut DecompressorOxide) };

    // 32KB circular dictionary
    let mut window = Vec::new();
    window
        .try_reserve_exact(WINDOW_SIZE)
        .map_err(|_| "cache: OOM for window")?;
    window.resize(WINDOW_SIZE, 0);

    // 4KB read buffer
    let mut rbuf = Vec::new();
    rbuf.try_reserve_exact(READ_BUF_SIZE)
        .map_err(|_| "cache: OOM for read buffer")?;
    rbuf.resize(READ_BUF_SIZE, 0);

    let mut stripper = HtmlStripStream::new();
    let mut strip_buf = [0u8; STRIP_BUF_SIZE];
    let mut strip_pos: usize = 0;
    let mut total_written: u32 = 0;

    let mut in_avail: usize = 0;
    let mut file_pos = data_offset;
    let mut comp_left = comp_size;
    let mut out_pos: usize = 0; // write position in circular window

    loop {
        // top up read buffer
        if in_avail < READ_BUF_SIZE && comp_left > 0 {
            let space = READ_BUF_SIZE - in_avail;
            let want = space.min(comp_left);
            match read_fn(file_pos, &mut rbuf[in_avail..in_avail + want]) {
                Ok(n) if n > 0 => {
                    file_pos += n as u32;
                    comp_left -= n;
                    in_avail += n;
                }
                Ok(_) => {
                    comp_left = 0;
                }
                Err(_) => return Err("cache: read failed during deflate"),
            }
        }

        if in_avail == 0 && out_pos == 0 {
            return Err("cache: empty deflate stream");
        }

        // circular-buffer mode: do not set TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF
        let flags = if comp_left > 0 {
            inflate_flags::TINFL_FLAG_HAS_MORE_INPUT
        } else {
            0
        };

        let old_out_pos = out_pos;
        let (status, consumed, produced) =
            decompress(&mut decomp, &rbuf[..in_avail], &mut window, out_pos, flags);

        // feed new output to HTML stripper; always contiguous within window
        if produced > 0 {
            let end = old_out_pos + produced;
            debug_assert!(
                end <= WINDOW_SIZE,
                "deflate produced past window boundary: {} > {}",
                end,
                WINDOW_SIZE
            );

            feed_and_flush(
                &mut stripper,
                &window[old_out_pos..end],
                &mut strip_buf,
                &mut strip_pos,
                &mut total_written,
                output_fn,
            )?;
        }

        out_pos += produced;

        if consumed > 0 && consumed < in_avail {
            rbuf.copy_within(consumed..in_avail, 0);
        }
        in_avail -= consumed;

        match status {
            TINFLStatus::Done => break,

            TINFLStatus::HasMoreOutput => {
                // window full; reset write pos, data stays for back-references
                out_pos = 0;
            }

            TINFLStatus::NeedsMoreInput => {
                if comp_left == 0 && in_avail == 0 {
                    return Err("cache: truncated deflate stream");
                }
                if consumed == 0 && produced == 0 && in_avail >= READ_BUF_SIZE {
                    return Err("cache: deflate stream stuck");
                }
            }

            _ => return Err("cache: deflate decompression error"),
        }
    }

    let trailing = stripper.finish(&mut strip_buf[strip_pos..]);
    strip_pos += trailing;
    if strip_pos > 0 {
        output_fn(&strip_buf[..strip_pos])?;
        total_written += strip_pos as u32;
    }

    Ok(total_written)
}

/// Strip HTML from an already-decompressed XHTML buffer, producing
/// styled plain text with inline `[MARKER, tag]` codes — the same
/// output format as [`stream_strip_entry`].
///
/// This is the in-memory counterpart of the streaming pipeline: the
/// caller extracts the full ZIP entry first (via [`crate::zip::extract_entry`]),
/// then hands the uncompressed bytes here for CPU-only processing.
///
/// Returns the stripped text as a new `Vec<u8>`, or an error if
/// allocation fails.
///
/// # Example
///
/// ```rust,ignore
/// let xhtml = smol_epub::zip::extract_entry(&entry, offset, read_fn)?;
/// let text  = smol_epub::cache::strip_html_buf(&xhtml)?;
/// // write `text` to the chapter cache file
/// ```
pub fn strip_html_buf(xhtml: &[u8]) -> Result<Vec<u8>, &'static str> {
    // Worst case: output ≈ input size (pure text, no tags). Over-
    // estimate slightly; the Vec will not reallocate beyond this.
    let mut out = Vec::new();
    out.try_reserve_exact(xhtml.len())
        .map_err(|_| "cache: OOM for strip buffer")?;

    let mut stripper = HtmlStripStream::new();
    let mut tmp = [0u8; STRIP_BUF_SIZE];
    let mut ip: usize = 0;

    while ip < xhtml.len() {
        let (consumed, written) = stripper.feed(&xhtml[ip..], &mut tmp);
        if written > 0 {
            out.extend_from_slice(&tmp[..written]);
        }
        if consumed == 0 && written == 0 {
            // no progress — skip one byte to break deadlock
            ip += 1;
        } else {
            ip += consumed;
        }
    }

    // flush any trailing state (deferred newlines, etc.)
    let trailing = stripper.finish(&mut tmp);
    if trailing > 0 {
        out.extend_from_slice(&tmp[..trailing]);
    }

    Ok(out)
}

// feed input through stripper; flush to output_fn when FLUSH_THRESHOLD reached
fn feed_and_flush(
    stripper: &mut HtmlStripStream,
    input: &[u8],
    strip_buf: &mut [u8; STRIP_BUF_SIZE],
    strip_pos: &mut usize,
    total_written: &mut u32,
    output_fn: &mut impl FnMut(&[u8]) -> Result<(), &'static str>,
) -> Result<(), &'static str> {
    let mut ip: usize = 0;

    while ip < input.len() {
        let avail_out = STRIP_BUF_SIZE - *strip_pos;
        if avail_out == 0 {
            // output buffer full; flush before continuing
            output_fn(&strip_buf[..*strip_pos])?;
            *total_written += *strip_pos as u32;
            *strip_pos = 0;
            continue;
        }

        let (consumed, written) = stripper.feed(
            &input[ip..],
            &mut strip_buf[*strip_pos..*strip_pos + avail_out],
        );
        ip += consumed;
        *strip_pos += written;

        if consumed == 0 && written == 0 {
            // no progress: flush pending data, or skip byte to break deadlock
            if *strip_pos > 0 {
                output_fn(&strip_buf[..*strip_pos])?;
                *total_written += *strip_pos as u32;
                *strip_pos = 0;
            } else {
                ip += 1;
            }
            continue;
        }

        // flush when buffer is sufficiently full
        if *strip_pos >= FLUSH_THRESHOLD {
            output_fn(&strip_buf[..*strip_pos])?;
            *total_written += *strip_pos as u32;
            *strip_pos = 0;
        }
    }

    Ok(())
}
