//! Async I/O support for `smol-epub`.
//!
//! Provides non-blocking versions of the core extraction and
//! stream-strip pipelines, suitable for cooperative multitasking
//! runtimes like Embassy.
//!
//! Enable with the `async` feature flag in `Cargo.toml`:
//!
//! ```toml
//! smol-epub = { version = "0.1", features = ["async"] }
//! ```
//!
//! # Traits
//!
//! | Trait | Purpose |
//! |-------|---------|
//! | [`AsyncReadAt`] | Random-access byte reads (SD card, flash, etc.) |
//! | [`AsyncWriteChunk`] | Incremental output (cache file append, etc.) |
//!
//! # Yield behaviour
//!
//! All async functions yield back to the executor between I/O
//! operations **and** periodically during CPU-bound work
//! (DEFLATE decompression, HTML stripping).  This ensures the
//! scheduler can service input, timers, and other Embassy tasks
//! even during large chapter extractions.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::html_strip::HtmlStripStream;
use crate::zip::{METHOD_DEFLATE, METHOD_STORED, ZipEntry, ZipIndex};

// ── yield primitive ─────────────────────────────────────────────────

/// Yield one timeslice back to the async executor.
///
/// Equivalent to `embassy_futures::yield_now()` but with no external
/// dependency.  Returns [`Poll::Pending`] exactly once, waking the
/// task immediately so it is re-polled on the next executor tick.
///
/// # Example
///
/// ```rust,ignore
/// for chunk in chunks {
///     do_work(chunk);
///     smol_epub::async_io::yield_now().await;
/// }
/// ```
pub fn yield_now() -> YieldNow {
    YieldNow(false)
}

/// Future returned by [`yield_now`].
pub struct YieldNow(bool);

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

// ── async I/O traits ────────────────────────────────────────────────

/// Random-access async byte reader.
///
/// Implementations read up to `buf.len()` bytes starting at the given
/// absolute `offset` and return the number of bytes actually read.
///
/// # Example
///
/// ```rust,ignore
/// struct SdReader<'a> { sd: &'a SdStorage, name: &'a str }
///
/// impl AsyncReadAt for SdReader<'_> {
///     async fn read_at(&mut self, offset: u32, buf: &mut [u8])
///         -> Result<usize, &'static str>
///     {
///         let n = storage::read_file_chunk(self.sd, self.name, offset, buf)?;
///         embassy_futures::yield_now().await;
///         Ok(n)
///     }
/// }
/// ```
#[allow(async_fn_in_trait)]
pub trait AsyncReadAt {
    /// Read bytes from `offset` into `buf`.  Returns the number of
    /// bytes actually read (0 signals EOF).
    async fn read_at(&mut self, offset: u32, buf: &mut [u8]) -> Result<usize, &'static str>;
}

/// Incremental async byte writer for cache output.
///
/// Each call to `write_chunk` appends `data` to the output stream.
#[allow(async_fn_in_trait)]
pub trait AsyncWriteChunk {
    /// Append `data` to the output.
    async fn write_chunk(&mut self, data: &[u8]) -> Result<(), &'static str>;
}

// ── async extract_entry ─────────────────────────────────────────────

/// Maximum uncompressed entry size (same as the sync version).
const MAX_ENTRY_SIZE: u32 = 192 * 1024;
const DEFLATE_READ_BUF: usize = 4096;

/// Async version of [`zip::extract_entry`](crate::zip::extract_entry).
///
/// Extracts a complete ZIP entry into a heap-allocated `Vec<u8>`,
/// yielding to the executor between I/O operations and decompression
/// iterations.
///
/// Supports both stored and DEFLATE-compressed entries.
pub async fn extract_entry_async<R: AsyncReadAt>(
    entry: &ZipEntry,
    local_offset: u32,
    reader: &mut R,
) -> Result<Vec<u8>, &'static str> {
    // read local file header
    let mut header = [0u8; 30];
    read_all_async(reader, local_offset, &mut header).await?;
    let skip = ZipIndex::local_header_data_skip(&header)?;
    let data_offset = local_offset + skip;

    if entry.uncomp_size > MAX_ENTRY_SIZE {
        return Err("zip: entry too large");
    }

    match entry.method {
        METHOD_STORED => extract_stored_async(entry, data_offset, reader).await,
        METHOD_DEFLATE => extract_deflate_async(entry, data_offset, reader).await,
        _ => Err("zip: unsupported compression method"),
    }
}

async fn extract_stored_async<R: AsyncReadAt>(
    entry: &ZipEntry,
    data_offset: u32,
    reader: &mut R,
) -> Result<Vec<u8>, &'static str> {
    let size = entry.uncomp_size as usize;
    log::info!("zip-async: stored entry ({} bytes)", size);

    let mut out = Vec::new();
    out.try_reserve_exact(size)
        .map_err(|_| "zip: chapter too large for memory")?;
    out.resize(size, 0);
    read_all_async(reader, data_offset, &mut out).await?;
    Ok(out)
}

async fn extract_deflate_async<R: AsyncReadAt>(
    entry: &ZipEntry,
    data_offset: u32,
    reader: &mut R,
) -> Result<Vec<u8>, &'static str> {
    use miniz_oxide::inflate::TINFLStatus;
    use miniz_oxide::inflate::core::DecompressorOxide;
    use miniz_oxide::inflate::core::decompress;
    use miniz_oxide::inflate::core::inflate_flags;

    let comp_size = entry.comp_size as usize;
    let uncomp_size = entry.uncomp_size as usize;

    log::info!(
        "zip-async: deflate stream {} -> {} bytes",
        comp_size,
        uncomp_size
    );

    let mut output = Vec::new();
    output
        .try_reserve_exact(uncomp_size)
        .map_err(|_| "zip: chapter too large for memory")?;
    output.resize(uncomp_size, 0);

    // ~11 KB DecompressorOxide — heap-allocate to avoid stack overflow
    let decomp_ptr =
        unsafe { alloc::alloc::alloc_zeroed(core::alloc::Layout::new::<DecompressorOxide>()) };
    if decomp_ptr.is_null() {
        return Err("zip: out of memory for decompressor");
    }
    let mut decomp = unsafe { Box::from_raw(decomp_ptr as *mut DecompressorOxide) };
    let mut out_pos: usize = 0;

    let mut rbuf = vec![0u8; DEFLATE_READ_BUF];
    let mut in_avail: usize = 0;
    let mut file_pos = data_offset;
    let mut comp_left = comp_size;
    let mut iterations: u32 = 0;

    loop {
        // top up compressed read buffer (async I/O)
        if in_avail < DEFLATE_READ_BUF && comp_left > 0 {
            let space = DEFLATE_READ_BUF - in_avail;
            let want = space.min(comp_left);
            match reader
                .read_at(file_pos, &mut rbuf[in_avail..in_avail + want])
                .await
            {
                Ok(n) if n > 0 => {
                    file_pos += n as u32;
                    comp_left -= n;
                    in_avail += n;
                }
                Ok(_) => {
                    comp_left = 0;
                }
                Err(e) => return Err(e),
            }
        }

        if in_avail == 0 && out_pos == 0 {
            return Err("zip: empty deflate stream");
        }

        let flags = inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF
            | if comp_left > 0 {
                inflate_flags::TINFL_FLAG_HAS_MORE_INPUT
            } else {
                0
            };

        let (status, consumed, produced) =
            decompress(&mut *decomp, &rbuf[..in_avail], &mut output, out_pos, flags);

        out_pos += produced;

        if consumed > 0 && consumed < in_avail {
            rbuf.copy_within(consumed..in_avail, 0);
        }
        in_avail -= consumed;

        // yield every 8 iterations to let other tasks run
        iterations += 1;
        if iterations % 8 == 0 {
            yield_now().await;
        }

        match status {
            TINFLStatus::Done => break,
            TINFLStatus::NeedsMoreInput => {
                if comp_left == 0 && in_avail == 0 {
                    return Err("zip: truncated deflate stream");
                }
                if consumed == 0 && produced == 0 && in_avail >= DEFLATE_READ_BUF {
                    return Err("zip: deflate stream stuck");
                }
            }
            TINFLStatus::HasMoreOutput => {
                return Err("zip: deflate output exceeds declared size");
            }
            _ => return Err("zip: deflate decompression error"),
        }
    }

    output.truncate(out_pos);
    Ok(output)
}

// ── async stream_strip_entry ────────────────────────────────────────

const WINDOW_SIZE: usize = 32768;
const READ_BUF_SIZE: usize = 4096;
const STRIP_BUF_SIZE: usize = 4096;
const FLUSH_THRESHOLD: usize = STRIP_BUF_SIZE - 128;

/// Async version of
/// [`cache::stream_strip_entry`](crate::cache::stream_strip_entry).
///
/// Stream-decompresses a ZIP entry, strips HTML, and emits plain-text
/// chunks through the `writer`, yielding to the executor between I/O
/// and decompression iterations.
///
/// Returns the total number of bytes written.
pub async fn stream_strip_entry_async<R: AsyncReadAt, W: AsyncWriteChunk>(
    entry: &ZipEntry,
    local_offset: u32,
    reader: &mut R,
    writer: &mut W,
) -> Result<u32, &'static str> {
    // skip local file header
    let mut header = [0u8; 30];
    read_all_async(reader, local_offset, &mut header).await?;
    let skip = ZipIndex::local_header_data_skip(&header)?;
    let data_offset = local_offset + skip;

    match entry.method {
        METHOD_STORED => stream_stored_async(entry, data_offset, reader, writer).await,
        METHOD_DEFLATE => stream_deflate_async(entry, data_offset, reader, writer).await,
        _ => Err("cache: unsupported compression method"),
    }
}

async fn stream_stored_async<R: AsyncReadAt, W: AsyncWriteChunk>(
    entry: &ZipEntry,
    data_offset: u32,
    reader: &mut R,
    writer: &mut W,
) -> Result<u32, &'static str> {
    let mut stripper = HtmlStripStream::new();
    let mut read_buf = [0u8; READ_BUF_SIZE];
    let mut strip_buf = [0u8; STRIP_BUF_SIZE];
    let mut strip_pos: usize = 0;
    let mut total_written: u32 = 0;

    let size = entry.uncomp_size;
    let mut file_pos = data_offset;
    let mut remaining = size;

    log::info!("cache-async: streaming stored entry ({} bytes)", size);

    while remaining > 0 {
        let want = (remaining as usize).min(READ_BUF_SIZE);
        let n = reader
            .read_at(file_pos, &mut read_buf[..want])
            .await
            .map_err(|_| "cache: read failed (stored)")?;
        if n == 0 {
            return Err("cache: unexpected EOF in stored entry");
        }
        file_pos += n as u32;
        remaining -= n as u32;

        feed_and_flush_async(
            &mut stripper,
            &read_buf[..n],
            &mut strip_buf,
            &mut strip_pos,
            &mut total_written,
            writer,
        )
        .await?;
    }

    // flush trailing stripper state
    let trailing = stripper.finish(&mut strip_buf[strip_pos..]);
    strip_pos += trailing;
    if strip_pos > 0 {
        writer.write_chunk(&strip_buf[..strip_pos]).await?;
        total_written += strip_pos as u32;
    }

    Ok(total_written)
}

async fn stream_deflate_async<R: AsyncReadAt, W: AsyncWriteChunk>(
    entry: &ZipEntry,
    data_offset: u32,
    reader: &mut R,
    writer: &mut W,
) -> Result<u32, &'static str> {
    use miniz_oxide::inflate::TINFLStatus;
    use miniz_oxide::inflate::core::{DecompressorOxide, decompress, inflate_flags};

    let comp_size = entry.comp_size as usize;
    let uncomp_size = entry.uncomp_size;

    log::info!(
        "cache-async: streaming deflate {} -> {} bytes",
        comp_size,
        uncomp_size
    );

    // ~11 KB DecompressorOxide
    let decomp_ptr =
        unsafe { alloc::alloc::alloc_zeroed(core::alloc::Layout::new::<DecompressorOxide>()) };
    if decomp_ptr.is_null() {
        return Err("cache: OOM for decompressor");
    }
    let mut decomp = unsafe { Box::from_raw(decomp_ptr as *mut DecompressorOxide) };

    // 32 KB circular dictionary
    let mut window = Vec::new();
    window
        .try_reserve_exact(WINDOW_SIZE)
        .map_err(|_| "cache: OOM for window")?;
    window.resize(WINDOW_SIZE, 0);

    // 4 KB read buffer
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
    let mut out_pos: usize = 0;
    let mut iterations: u32 = 0;

    loop {
        // top up read buffer (async I/O)
        if in_avail < READ_BUF_SIZE && comp_left > 0 {
            let space = READ_BUF_SIZE - in_avail;
            let want = space.min(comp_left);
            match reader
                .read_at(file_pos, &mut rbuf[in_avail..in_avail + want])
                .await
            {
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

        // circular-buffer mode
        let flags = if comp_left > 0 {
            inflate_flags::TINFL_FLAG_HAS_MORE_INPUT
        } else {
            0
        };

        let old_out_pos = out_pos;
        let (status, consumed, produced) =
            decompress(&mut decomp, &rbuf[..in_avail], &mut window, out_pos, flags);

        // feed new output to HTML stripper
        if produced > 0 {
            let end = old_out_pos + produced;
            feed_and_flush_async(
                &mut stripper,
                &window[old_out_pos..end],
                &mut strip_buf,
                &mut strip_pos,
                &mut total_written,
                writer,
            )
            .await?;
        }

        out_pos += produced;

        if consumed > 0 && consumed < in_avail {
            rbuf.copy_within(consumed..in_avail, 0);
        }
        in_avail -= consumed;

        // yield every 8 iterations
        iterations += 1;
        if iterations % 8 == 0 {
            yield_now().await;
        }

        match status {
            TINFLStatus::Done => break,

            TINFLStatus::HasMoreOutput => {
                // window full; reset
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
        writer.write_chunk(&strip_buf[..strip_pos]).await?;
        total_written += strip_pos as u32;
    }

    Ok(total_written)
}

// ── async strip_html_buf ────────────────────────────────────────────

/// Async version of [`cache::strip_html_buf`](crate::cache::strip_html_buf).
///
/// Strips HTML from already-decompressed XHTML, yielding periodically
/// during processing.  Returns the styled plain text.
pub async fn strip_html_buf_async(xhtml: &[u8]) -> Result<Vec<u8>, &'static str> {
    let mut out = Vec::new();
    out.try_reserve_exact(xhtml.len())
        .map_err(|_| "cache: OOM for strip buffer")?;

    let mut stripper = HtmlStripStream::new();
    let mut tmp = [0u8; STRIP_BUF_SIZE];
    let mut ip: usize = 0;
    let mut iterations: u32 = 0;

    while ip < xhtml.len() {
        let (consumed, written) = stripper.feed(&xhtml[ip..], &mut tmp);
        if written > 0 {
            out.extend_from_slice(&tmp[..written]);
        }
        if consumed == 0 && written == 0 {
            ip += 1;
        } else {
            ip += consumed;
        }

        iterations += 1;
        if iterations % 64 == 0 {
            yield_now().await;
        }
    }

    let trailing = stripper.finish(&mut tmp);
    if trailing > 0 {
        out.extend_from_slice(&tmp[..trailing]);
    }

    Ok(out)
}

// ── helpers ─────────────────────────────────────────────────────────

async fn read_all_async<R: AsyncReadAt>(
    reader: &mut R,
    offset: u32,
    buf: &mut [u8],
) -> Result<(), &'static str> {
    let mut total = 0usize;
    while total < buf.len() {
        let n = reader
            .read_at(offset + total as u32, &mut buf[total..])
            .await
            .map_err(|_| "zip: read failed")?;
        if n == 0 {
            return Err("zip: unexpected EOF");
        }
        total += n;
    }
    Ok(())
}

async fn feed_and_flush_async<W: AsyncWriteChunk>(
    stripper: &mut HtmlStripStream,
    input: &[u8],
    strip_buf: &mut [u8; STRIP_BUF_SIZE],
    strip_pos: &mut usize,
    total_written: &mut u32,
    writer: &mut W,
) -> Result<(), &'static str> {
    let mut ip: usize = 0;

    while ip < input.len() {
        let avail_out = STRIP_BUF_SIZE - *strip_pos;
        if avail_out == 0 {
            writer.write_chunk(&strip_buf[..*strip_pos]).await?;
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
            if *strip_pos > 0 {
                writer.write_chunk(&strip_buf[..*strip_pos]).await?;
                *total_written += *strip_pos as u32;
                *strip_pos = 0;
            } else {
                ip += 1;
            }
            continue;
        }

        if *strip_pos >= FLUSH_THRESHOLD {
            writer.write_chunk(&strip_buf[..*strip_pos]).await?;
            *total_written += *strip_pos as u32;
            *strip_pos = 0;
        }
    }

    Ok(())
}
