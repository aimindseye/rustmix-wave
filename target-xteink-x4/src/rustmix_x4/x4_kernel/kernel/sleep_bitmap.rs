use core::cell::Cell;

use crate::rustmix_x4::x4_kernel::drivers::sdcard::SdStorage;
use crate::rustmix_x4::x4_kernel::drivers::storage;
use crate::rustmix_x4::x4_kernel::drivers::strip::{PHYS_BYTES_PER_ROW, STRIP_ROWS, StripBuffer};
use alloc::vec::Vec;

const SCREEN_W: i32 = 800;
const SCREEN_H: i32 = 480;
const BMP_HEADER_BUF_LEN: usize = 96;
const PREFETCH_BYTES: usize = PHYS_BYTES_PER_ROW * SCREEN_H as usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SleepBitmapLocation {
    Root,
    Subdir {
        dir: &'static str,
        subdir: &'static str,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SleepBitmapCandidate {
    pub location: SleepBitmapLocation,
    pub name: &'static str,
}

impl SleepBitmapCandidate {
    pub const fn root(name: &'static str) -> Self {
        Self {
            location: SleepBitmapLocation::Root,
            name,
        }
    }

    pub const fn nested(dir: &'static str, subdir: &'static str, name: &'static str) -> Self {
        Self {
            location: SleepBitmapLocation::Subdir { dir, subdir },
            name,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SleepBitmapInfo {
    candidate: SleepBitmapCandidate,
    data_offset: u32,
    row_stride: usize,
    top_down: bool,
    invert_bits: bool,
}

impl SleepBitmapInfo {
    pub const fn candidate(self) -> SleepBitmapCandidate {
        self.candidate
    }

    pub const fn data_len(self) -> usize {
        self.row_stride * SCREEN_H as usize
    }
}

#[derive(Debug)]
pub struct PrefetchedSleepBitmap {
    info: SleepBitmapInfo,
    data: Vec<u8>,
}

impl PrefetchedSleepBitmap {
    pub fn info(&self) -> SleepBitmapInfo {
        self.info
    }
}

pub const RUSTMIX_SLEEP_DIR: &str = "RUSTMIX/SLEEP";

// Images in /RUSTMIX/SLEEP are intentionally restricted to 8.3-safe BMP
// names so they work reliably with the embedded FAT/SFN storage layer.  Users
// can upload any subset of these files; the resolver randomly chooses from
// the valid 800x480 1bpp BMP files that exist.
pub const RUSTMIX_SLEEP_BITMAP_CANDIDATES: &[SleepBitmapCandidate] = &[
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP00.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP01.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP02.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP03.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP04.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP05.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP06.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP07.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP08.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP09.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP10.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP11.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP12.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP13.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP14.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP15.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP16.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP17.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP18.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP19.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP20.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP21.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP22.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP23.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP24.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP25.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP26.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP27.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP28.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP29.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP30.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP31.BMP"),
    SleepBitmapCandidate::nested("RUSTMIX", "SLEEP", "SLEEP32.BMP"),
];
pub const SLEEP_IMAGE_CACHE_HINT_FILE: SleepBitmapCandidate =
    SleepBitmapCandidate::root("SLPCACHE.TXT");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SleepImageMode {
    /// Randomly choose a valid 800x480 1bpp BMP from /RUSTMIX/SLEEP.
    RandomFolder,
}

impl SleepImageMode {
    pub const fn name(self) -> &'static str {
        match self {
            Self::RandomFolder => "random",
        }
    }

    pub const fn renders_bitmap(self) -> bool {
        matches!(self, Self::RandomFolder)
    }
}

/// Rustmix release policy: sleep always uses random images from /RUSTMIX/SLEEP.
/// Legacy sleep-mode settings are ignored so old configuration cannot leave
/// the device in text/no-redraw/static modes after an app-only firmware update.
pub fn read_sleep_image_mode(_sd: &SdStorage) -> SleepImageMode {
    SleepImageMode::RandomFolder
}

pub fn resolve_sleep_bitmap(sd: &SdStorage) -> Option<SleepBitmapInfo> {
    resolve_sleep_bitmap_at_uptime(sd, 0)
}

pub fn resolve_sleep_bitmap_at_uptime(sd: &SdStorage, uptime_secs: u32) -> Option<SleepBitmapInfo> {
    resolve_random_sleep_bitmap(sd, uptime_secs)
}

pub fn resolve_sleep_bitmap_for_mode(
    sd: &SdStorage,
    mode: SleepImageMode,
) -> Option<SleepBitmapInfo> {
    resolve_sleep_bitmap_for_mode_at_uptime(sd, mode, 0)
}

pub fn resolve_sleep_bitmap_for_mode_at_uptime(
    sd: &SdStorage,
    mode: SleepImageMode,
    uptime_secs: u32,
) -> Option<SleepBitmapInfo> {
    match mode {
        SleepImageMode::RandomFolder => resolve_random_sleep_bitmap(sd, uptime_secs),
    }
}

pub fn resolve_random_sleep_bitmap(sd: &SdStorage, uptime_secs: u32) -> Option<SleepBitmapInfo> {
    let mut valid_count = 0usize;
    for candidate in RUSTMIX_SLEEP_BITMAP_CANDIDATES.iter().copied() {
        if probe_sleep_bitmap(sd, candidate).is_some() {
            valid_count += 1;
        }
    }

    if valid_count == 0 {
        return None;
    }

    let target = random_sleep_index(uptime_secs, valid_count);
    let mut seen = 0usize;
    for candidate in RUSTMIX_SLEEP_BITMAP_CANDIDATES.iter().copied() {
        if let Some(info) = probe_sleep_bitmap(sd, candidate) {
            if seen == target {
                return Some(info);
            }
            seen += 1;
        }
    }

    None
}

fn random_sleep_index(uptime_secs: u32, valid_count: usize) -> usize {
    if valid_count <= 1 {
        return 0;
    }
    let mixed = uptime_secs
        .wrapping_mul(1_664_525)
        .wrapping_add(1_013_904_223)
        .rotate_left((uptime_secs & 15) + 1);
    (mixed as usize) % valid_count
}

pub fn resolve_sleep_bitmap_for_mode_timed(
    sd: &SdStorage,
    mode: SleepImageMode,
    uptime_secs: u32,
    bmp_decode_ms: &Cell<u64>,
) -> Option<SleepBitmapInfo> {
    let start = embassy_time::Instant::now();
    let resolved = resolve_sleep_bitmap_for_mode_at_uptime(sd, mode, uptime_secs);
    bmp_decode_ms.set(bmp_decode_ms.get() + start.elapsed().as_millis());
    resolved
}

pub fn probe_sleep_bitmap(
    sd: &SdStorage,
    candidate: SleepBitmapCandidate,
) -> Option<SleepBitmapInfo> {
    let mut header = [0u8; BMP_HEADER_BUF_LEN];
    let (_size, n) = read_start(sd, candidate, &mut header).ok()?;
    parse_sleep_bitmap_header(candidate, &header[..n])
}

pub fn prefetch_sleep_bitmap(
    sd: &SdStorage,
    info: &SleepBitmapInfo,
) -> Option<PrefetchedSleepBitmap> {
    if info.row_stride != PHYS_BYTES_PER_ROW || info.data_len() != PREFETCH_BYTES {
        return None;
    }

    let mut data = Vec::new();
    if data.try_reserve_exact(info.data_len()).is_err() {
        return None;
    }
    data.resize(info.data_len(), 0xFF);

    let n = read_chunk(sd, info.candidate, info.data_offset, &mut data).ok()?;
    if n != data.len() {
        return None;
    }

    if info.invert_bits {
        for byte in data.iter_mut() {
            *byte = !*byte;
        }
    }

    Some(PrefetchedSleepBitmap { info: *info, data })
}

pub fn prefetch_sleep_bitmap_timed(
    sd: &SdStorage,
    info: &SleepBitmapInfo,
    bmp_prefetch_ms: &Cell<u64>,
) -> Option<PrefetchedSleepBitmap> {
    let start = embassy_time::Instant::now();
    let prefetched = prefetch_sleep_bitmap(sd, info);
    bmp_prefetch_ms.set(bmp_prefetch_ms.get() + start.elapsed().as_millis());
    prefetched
}

pub fn draw_sleep_bitmap_strip(
    sd: &SdStorage,
    info: &SleepBitmapInfo,
    strip: &mut StripBuffer,
) -> bool {
    let (_x, y, w, h) = strip.window();
    if w as usize != PHYS_BYTES_PER_ROW * 8 || h == 0 || h > STRIP_ROWS {
        return false;
    }
    if info.row_stride != PHYS_BYTES_PER_ROW {
        return false;
    }

    let data = strip.data_mut();
    let rows = h as usize;
    for local_y in 0..rows {
        let screen_y = y as i32 + local_y as i32;
        if !(0..SCREEN_H).contains(&screen_y) {
            return false;
        }

        let src_y = if info.top_down {
            screen_y as u32
        } else {
            (SCREEN_H - 1 - screen_y) as u32
        };
        let file_offset = info.data_offset + src_y * info.row_stride as u32;
        let start = local_y * PHYS_BYTES_PER_ROW;
        let end = start + PHYS_BYTES_PER_ROW;
        let row = &mut data[start..end];

        match read_chunk(sd, info.candidate, file_offset, row) {
            Ok(n) if n == PHYS_BYTES_PER_ROW => {
                if info.invert_bits {
                    for b in row.iter_mut() {
                        *b = !*b;
                    }
                }
            }
            _ => return false,
        }
    }

    true
}

pub fn draw_sleep_bitmap_strip_timed(
    sd: &SdStorage,
    info: &SleepBitmapInfo,
    strip: &mut StripBuffer,
    bmp_draw_ms: &Cell<u64>,
) -> bool {
    let start = embassy_time::Instant::now();
    let ok = draw_sleep_bitmap_strip(sd, info, strip);
    bmp_draw_ms.set(bmp_draw_ms.get() + start.elapsed().as_millis());
    ok
}

pub fn draw_prefetched_sleep_bitmap_strip(
    bitmap: &PrefetchedSleepBitmap,
    strip: &mut StripBuffer,
) -> bool {
    let info = bitmap.info();
    let (_x, y, w, h) = strip.window();
    if w as usize != PHYS_BYTES_PER_ROW * 8 || h == 0 || h > STRIP_ROWS {
        return false;
    }
    if info.row_stride != PHYS_BYTES_PER_ROW || bitmap.data.len() != PREFETCH_BYTES {
        return false;
    }

    let data = strip.data_mut();
    let rows = h as usize;
    for local_y in 0..rows {
        let screen_y = y as i32 + local_y as i32;
        if !(0..SCREEN_H).contains(&screen_y) {
            return false;
        }

        let src_y = if info.top_down {
            screen_y as usize
        } else {
            (SCREEN_H - 1 - screen_y) as usize
        };
        let src_start = src_y * PHYS_BYTES_PER_ROW;
        let src_end = src_start + PHYS_BYTES_PER_ROW;
        let dst_start = local_y * PHYS_BYTES_PER_ROW;
        let dst_end = dst_start + PHYS_BYTES_PER_ROW;
        data[dst_start..dst_end].copy_from_slice(&bitmap.data[src_start..src_end]);
    }

    true
}

pub fn draw_prefetched_sleep_bitmap_strip_timed(
    bitmap: &PrefetchedSleepBitmap,
    strip: &mut StripBuffer,
    bmp_draw_ms: &Cell<u64>,
) -> bool {
    let start = embassy_time::Instant::now();
    let ok = draw_prefetched_sleep_bitmap_strip(bitmap, strip);
    bmp_draw_ms.set(bmp_draw_ms.get() + start.elapsed().as_millis());
    ok
}

pub fn sleep_bitmap_cache_hint_matches(sd: &SdStorage, info: &SleepBitmapInfo) -> bool {
    let expected = candidate_cache_key(info.candidate());
    let mut buf = [0u8; 96];
    let Ok((_size, n)) = read_start(sd, SLEEP_IMAGE_CACHE_HINT_FILE, &mut buf) else {
        return false;
    };
    trim_ascii(&buf[..n]) == expected.as_bytes()
}

pub fn sleep_bitmap_cache_hint_for_info(info: &SleepBitmapInfo) -> &'static str {
    candidate_cache_key(info.candidate())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeekdayKey {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

pub fn parse_weekday_key(data: &[u8]) -> Option<WeekdayKey> {
    let mut lower = [0u8; 32];
    let n = data.len().min(lower.len());
    for i in 0..n {
        lower[i] = data[i].to_ascii_lowercase();
    }
    let s = trim_ascii(&lower[..n]);

    if contains(s, b"monday") || contains(s, b"mon") {
        Some(WeekdayKey::Monday)
    } else if contains(s, b"tuesday") || contains(s, b"tue") {
        Some(WeekdayKey::Tuesday)
    } else if contains(s, b"wednesday") || contains(s, b"wed") {
        Some(WeekdayKey::Wednesday)
    } else if contains(s, b"thursday") || contains(s, b"thu") {
        Some(WeekdayKey::Thursday)
    } else if contains(s, b"friday") || contains(s, b"fri") {
        Some(WeekdayKey::Friday)
    } else if contains(s, b"saturday") || contains(s, b"sat") {
        Some(WeekdayKey::Saturday)
    } else if contains(s, b"sunday") || contains(s, b"sun") {
        Some(WeekdayKey::Sunday)
    } else {
        None
    }
}

pub fn parse_sleep_image_mode(data: &[u8]) -> Option<SleepImageMode> {
    let mut lower = [0u8; 32];
    let n = data.len().min(lower.len());
    for i in 0..n {
        lower[i] = data[i].to_ascii_lowercase();
    }
    let s = trim_ascii(&lower[..n]);

    if eq_ascii(s, b"random")
        || eq_ascii(s, b"rand")
        || eq_ascii(s, b"folder")
        || eq_ascii(s, b"random-folder")
        || eq_ascii(s, b"random_folder")
        || eq_ascii(s, b"static")
        || eq_ascii(s, b"daily")
        || eq_ascii(s, b"fast-daily")
        || eq_ascii(s, b"cached")
        || eq_ascii(s, b"text")
        || eq_ascii(s, b"off")
        || eq_ascii(s, b"none")
        || eq_ascii(s, b"no-redraw")
        || eq_ascii(s, b"no_redraw")
    {
        Some(SleepImageMode::RandomFolder)
    } else {
        None
    }
}

fn candidate_cache_key(candidate: SleepBitmapCandidate) -> &'static str {
    match candidate.location {
        SleepBitmapLocation::Root => candidate.name,
        SleepBitmapLocation::Subdir { dir, subdir } => match (dir, subdir, candidate.name) {
            ("RUSTMIX", "SLEEP", "SLEEP.BMP") => "/RUSTMIX/SLEEP/SLEEP.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP00.BMP") => "/RUSTMIX/SLEEP/SLEEP00.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP01.BMP") => "/RUSTMIX/SLEEP/SLEEP01.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP02.BMP") => "/RUSTMIX/SLEEP/SLEEP02.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP03.BMP") => "/RUSTMIX/SLEEP/SLEEP03.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP04.BMP") => "/RUSTMIX/SLEEP/SLEEP04.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP05.BMP") => "/RUSTMIX/SLEEP/SLEEP05.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP06.BMP") => "/RUSTMIX/SLEEP/SLEEP06.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP07.BMP") => "/RUSTMIX/SLEEP/SLEEP07.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP08.BMP") => "/RUSTMIX/SLEEP/SLEEP08.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP09.BMP") => "/RUSTMIX/SLEEP/SLEEP09.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP10.BMP") => "/RUSTMIX/SLEEP/SLEEP10.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP11.BMP") => "/RUSTMIX/SLEEP/SLEEP11.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP12.BMP") => "/RUSTMIX/SLEEP/SLEEP12.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP13.BMP") => "/RUSTMIX/SLEEP/SLEEP13.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP14.BMP") => "/RUSTMIX/SLEEP/SLEEP14.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP15.BMP") => "/RUSTMIX/SLEEP/SLEEP15.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP16.BMP") => "/RUSTMIX/SLEEP/SLEEP16.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP17.BMP") => "/RUSTMIX/SLEEP/SLEEP17.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP18.BMP") => "/RUSTMIX/SLEEP/SLEEP18.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP19.BMP") => "/RUSTMIX/SLEEP/SLEEP19.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP20.BMP") => "/RUSTMIX/SLEEP/SLEEP20.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP21.BMP") => "/RUSTMIX/SLEEP/SLEEP21.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP22.BMP") => "/RUSTMIX/SLEEP/SLEEP22.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP23.BMP") => "/RUSTMIX/SLEEP/SLEEP23.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP24.BMP") => "/RUSTMIX/SLEEP/SLEEP24.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP25.BMP") => "/RUSTMIX/SLEEP/SLEEP25.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP26.BMP") => "/RUSTMIX/SLEEP/SLEEP26.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP27.BMP") => "/RUSTMIX/SLEEP/SLEEP27.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP28.BMP") => "/RUSTMIX/SLEEP/SLEEP28.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP29.BMP") => "/RUSTMIX/SLEEP/SLEEP29.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP30.BMP") => "/RUSTMIX/SLEEP/SLEEP30.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP31.BMP") => "/RUSTMIX/SLEEP/SLEEP31.BMP",
            ("RUSTMIX", "SLEEP", "SLEEP32.BMP") => "/RUSTMIX/SLEEP/SLEEP32.BMP",
            _ => candidate.name,
        },
    }
}

fn parse_sleep_bitmap_header(
    candidate: SleepBitmapCandidate,
    header: &[u8],
) -> Option<SleepBitmapInfo> {
    if header.len() < 62 || header.get(0..2)? != b"BM" {
        return None;
    }

    let data_offset = le_u32(header, 10)?;
    let dib_size = le_u32(header, 14)?;
    if dib_size < 40 {
        return None;
    }

    let width = le_i32(header, 18)?;
    let raw_height = le_i32(header, 22)?;
    let planes = le_u16(header, 26)?;
    let bits_per_pixel = le_u16(header, 28)?;
    let compression = le_u32(header, 30)?;

    if width != SCREEN_W || raw_height.unsigned_abs() as i32 != SCREEN_H {
        return None;
    }
    if planes != 1 || bits_per_pixel != 1 || compression != 0 {
        return None;
    }

    let row_stride = (width as usize * bits_per_pixel as usize).div_ceil(32) * 4;
    if row_stride != PHYS_BYTES_PER_ROW {
        return None;
    }

    let palette_offset = 14usize + dib_size as usize;
    let invert_bits = if header.len() >= palette_offset + 8 {
        let p0 = &header[palette_offset..palette_offset + 4];
        let p1 = &header[palette_offset + 4..palette_offset + 8];
        luminance(p0) > luminance(p1)
    } else {
        false
    };

    Some(SleepBitmapInfo {
        candidate,
        data_offset,
        row_stride,
        top_down: raw_height < 0,
        invert_bits,
    })
}

fn read_start(
    sd: &SdStorage,
    candidate: SleepBitmapCandidate,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<(u32, usize)> {
    match candidate.location {
        SleepBitmapLocation::Root => storage::read_file_start(sd, candidate.name, buf),
        SleepBitmapLocation::Subdir { dir, subdir } => {
            storage::read_file_start_in_subdir(sd, dir, subdir, candidate.name, buf)
        }
    }
}

fn read_chunk(
    sd: &SdStorage,
    candidate: SleepBitmapCandidate,
    offset: u32,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    match candidate.location {
        SleepBitmapLocation::Root => storage::read_file_chunk(sd, candidate.name, offset, buf),
        SleepBitmapLocation::Subdir { dir, subdir } => {
            storage::read_file_chunk_in_subdir(sd, dir, subdir, candidate.name, offset, buf)
        }
    }
}

fn trim_ascii(mut data: &[u8]) -> &[u8] {
    while let Some((first, rest)) = data.split_first() {
        if first.is_ascii_whitespace() {
            data = rest;
        } else {
            break;
        }
    }
    while let Some((last, rest)) = data.split_last() {
        if last.is_ascii_whitespace() {
            data = rest;
        } else {
            break;
        }
    }
    data
}

fn eq_ascii(left: &[u8], right: &[u8]) -> bool {
    left == right
}

fn le_u16(buf: &[u8], offset: usize) -> Option<u16> {
    let bytes = buf.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn le_u32(buf: &[u8], offset: usize) -> Option<u32> {
    let bytes = buf.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn le_i32(buf: &[u8], offset: usize) -> Option<i32> {
    let bytes = buf.get(offset..offset + 4)?;
    Some(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn luminance(bgra: &[u8]) -> u16 {
    bgra[0] as u16 + bgra[1] as u16 + bgra[2] as u16
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::{
        SleepBitmapCandidate, SleepImageMode, WeekdayKey, parse_sleep_bitmap_header,
        parse_sleep_image_mode, parse_weekday_key,
    };

    #[test]
    fn weekday_key_accepts_names_and_abbreviations() {
        assert_eq!(parse_weekday_key(b"weekday=tue"), Some(WeekdayKey::Tuesday));
        assert_eq!(parse_weekday_key(b"Sunday\n"), Some(WeekdayKey::Sunday));
        assert_eq!(parse_weekday_key(b"unknown"), None);
    }

    #[test]
    fn sleep_image_mode_parser_maps_legacy_values_to_random_folder() {
        for value in [
            b"daily
"
            .as_slice(),
            b"fast-daily".as_slice(),
            b"static".as_slice(),
            b"random".as_slice(),
            b"cached".as_slice(),
            b"text".as_slice(),
            b"off".as_slice(),
            b"no-redraw".as_slice(),
        ] {
            assert_eq!(
                parse_sleep_image_mode(value),
                Some(SleepImageMode::RandomFolder)
            );
        }
    }

    #[test]
    fn bmp_header_accepts_x4_1bpp_bitmap() {
        let mut header = [0u8; 96];
        header[0] = b'B';
        header[1] = b'M';
        header[10..14].copy_from_slice(&62u32.to_le_bytes());
        header[14..18].copy_from_slice(&40u32.to_le_bytes());
        header[18..22].copy_from_slice(&800i32.to_le_bytes());
        header[22..26].copy_from_slice(&480i32.to_le_bytes());
        header[26..28].copy_from_slice(&1u16.to_le_bytes());
        header[28..30].copy_from_slice(&1u16.to_le_bytes());
        header[30..34].copy_from_slice(&0u32.to_le_bytes());
        header[54..58].copy_from_slice(&[0, 0, 0, 0]);
        header[58..62].copy_from_slice(&[255, 255, 255, 0]);

        let info = parse_sleep_bitmap_header(SleepBitmapCandidate::root("sleep.bmp"), &header)
            .expect("valid sleep bitmap header");
        assert!(!info.top_down);
        assert!(!info.invert_bits);
        assert_eq!(info.data_len(), 48_000);
    }
}
