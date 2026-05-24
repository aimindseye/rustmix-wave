// directory listing cache: sorted entries with title resolution
// loaded lazily from SD, held in RAM, invalidated on demand

use crate::rustmix_x4::x4_kernel::drivers::sdcard::SdStorage;
use crate::rustmix_x4::x4_kernel::drivers::storage::{
    DirEntry, DirPage, TITLES_FILE, X4_DIR, list_root_files, read_file_start_in_dir,
};
use crate::rustmix_x4::x4_kernel::error::Result;

const MAX_DIR_ENTRIES: usize = 128;
const TITLE_MAP_FILE: &str = "TITLEMAP.TSV";
pub const TITLE_KIND_EPUB: u8 = 1;
pub const TITLE_KIND_TEXT: u8 = 2;

pub struct DirCache {
    entries: [DirEntry; MAX_DIR_ENTRIES],
    count: usize,
    valid: bool,
}

impl Default for DirCache {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
fn is_epub_or_epu_name(name: &[u8]) -> bool {
    if name.len() >= 5
        && name[name.len() - 5] == b'.'
        && name[name.len() - 4..].eq_ignore_ascii_case(b"EPUB")
    {
        return true;
    }

    name.len() >= 4
        && name[name.len() - 4] == b'.'
        && name[name.len() - 3..].eq_ignore_ascii_case(b"EPU")
}

fn is_text_title_name(name: &[u8]) -> bool {
    if name.len() >= 4
        && name[name.len() - 4] == b'.'
        && name[name.len() - 3..].eq_ignore_ascii_case(b"TXT")
    {
        return true;
    }

    name.len() >= 3
        && name[name.len() - 3] == b'.'
        && name[name.len() - 2..].eq_ignore_ascii_case(b"MD")
}

fn trim_title_line_end(mut line: &[u8]) -> &[u8] {
    while matches!(line.last(), Some(b'\n' | b'\r')) {
        line = &line[..line.len() - 1];
    }
    line
}

fn trim_ascii_title_part(mut part: &[u8]) -> &[u8] {
    while matches!(part.first(), Some(b' ' | b'\t')) {
        part = &part[1..];
    }
    while matches!(part.last(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
        part = &part[..part.len() - 1];
    }
    part
}

fn title_cache_title_is_usable(file_part: &[u8], title_part: &[u8]) -> bool {
    if title_part.is_empty()
        || title_part.len() > crate::rustmix_x4::x4_kernel::drivers::storage::TITLE_CAP
    {
        return false;
    }

    let title_visible_bytes = title_part
        .iter()
        .filter(|&&b| b > b' ' && b != 0x7F)
        .count();
    if title_visible_bytes >= 3 {
        return true;
    }

    // Ignore stale/corrupt mappings such as ALICES~1.EPU<TAB>A so the
    // soft SFN fallback and EPUB scanner can recover the real long title.
    let stem_letters = file_part
        .iter()
        .take_while(|&&b| b != b'.')
        .filter(|&&b| b.is_ascii_alphanumeric())
        .count();
    title_visible_bytes > 0 && stem_letters <= 2
}

impl DirCache {
    pub const fn new() -> Self {
        Self {
            entries: [DirEntry::EMPTY; MAX_DIR_ENTRIES],
            count: 0,
            valid: false,
        }
    }

    pub fn ensure_loaded(&mut self, sd: &SdStorage) -> Result<()> {
        if self.valid {
            return Ok(());
        }

        let count = list_root_files(sd, &mut self.entries)?;
        self.count = count;
        sort_entries(&mut self.entries, self.count);
        self.load_host_title_map(sd);
        self.load_titles(sd);
        for i in 0..self.count {
            self.entries[i].humanize_sfn();
        }
        self.valid = true;
        Ok(())
    }

    fn load_host_title_map(&mut self, sd: &SdStorage) {
        let mut buf = [0u8; 4096];
        let n = match read_file_start_in_dir(sd, X4_DIR, TITLE_MAP_FILE, &mut buf) {
            Ok((_, n)) => n,
            Err(_) => return,
        };

        let data = &buf[..n];
        let mut start = 0usize;
        while start < data.len() {
            let end = data[start..]
                .iter()
                .position(|&b| b == b'\n')
                .map(|p| start + p)
                .unwrap_or(data.len());
            let mut line = &data[start..end];
            if line.ends_with(b"\r") {
                line = &line[..line.len() - 1];
            }
            if !line.is_empty() {
                self.apply_title_line(line);
            }
            start = end.saturating_add(1);
        }
    }

    fn load_titles(&mut self, sd: &SdStorage) {
        let mut buf = [0u8; 4096];
        let n = match read_file_start_in_dir(sd, X4_DIR, TITLES_FILE, &mut buf) {
            Ok((_, n)) => n,
            Err(_) => return,
        };

        let data = &buf[..n];
        let mut start = 0;
        while start < data.len() {
            let end = data[start..]
                .iter()
                .position(|&b| b == b'\n')
                .map(|p| start + p)
                .unwrap_or(data.len());
            let line = &data[start..end];
            if !line.is_empty() {
                self.apply_title_line(line);
            }
            start = end + 1;
        }
    }

    fn apply_title_line(&mut self, line: &[u8]) {
        let line = trim_title_line_end(line);
        let tab_pos = match line.iter().position(|&b| b == b'\t') {
            Some(p) => p,
            None => return,
        };
        let file_part = trim_ascii_title_part(&line[..tab_pos]);
        let title_part = trim_ascii_title_part(&line[tab_pos + 1..]);
        if !title_cache_title_is_usable(file_part, title_part) {
            return;
        }

        let file_str = match core::str::from_utf8(file_part) {
            Ok(s) => s,
            Err(_) => return,
        };

        for i in 0..self.count {
            if self.entries[i].name_str().eq_ignore_ascii_case(file_str) {
                self.entries[i].set_title(title_part);
                break;
            }
        }
    }

    pub fn page(&self, offset: usize, buf: &mut [DirEntry]) -> DirPage {
        let total = self.count;
        let start = offset.min(total);
        let end = (start + buf.len()).min(total);
        let count = end - start;
        buf[..count].clone_from_slice(&self.entries[start..end]);
        DirPage { total, count }
    }

    pub fn invalidate(&mut self) {
        self.valid = false;
    }

    pub fn next_untitled_epub(&self, from: usize) -> Option<(usize, [u8; 13], u8)> {
        for i in from..self.count {
            let e = &self.entries[i];
            if e.has_real_title() || e.is_dir {
                continue;
            }
            let name = e.name_str().as_bytes();
            if is_epub_or_epu_name(name) {
                return Some((i, e.name, e.name_len));
            }
        }
        None
    }

    pub fn next_untitled_reader_title(&self, from: usize) -> Option<(usize, [u8; 13], u8, u8)> {
        for i in from..self.count {
            let e = &self.entries[i];
            if e.has_real_title() || e.is_dir {
                continue;
            }

            let name = e.name_str().as_bytes();
            if is_epub_or_epu_name(name) {
                return Some((i, e.name, e.name_len, TITLE_KIND_EPUB));
            }

            // Title extraction repair:
            // TXT/MD body-title scanning is disabled. It was unsafe because
            // license/body lines can be cached as display titles. A future
            // FAT LFN/title-map lane should provide proper TXT display names.
            if is_text_title_name(name) {
                continue;
            }
        }

        None
    }

    // look up the display title for a filename (case-insensitive);
    // returns (title_bytes, title_len) including humanized SFN
    pub fn find_title(&self, filename: &[u8]) -> Option<(&[u8], u8)> {
        let name = match core::str::from_utf8(filename) {
            Ok(s) => s,
            Err(_) => return None,
        };
        for i in 0..self.count {
            let e = &self.entries[i];
            if e.name_str().eq_ignore_ascii_case(name) {
                let len = (e.title_len & 0x7F) as usize;
                if len > 0 {
                    return Some((&e.title[..len], len as u8));
                }
                return None;
            }
        }
        None
    }

    pub fn set_entry_title(&mut self, index: usize, title: &[u8]) {
        if index < self.count {
            self.entries[index].set_title(title);
        }
    }
}

// insertion sort; count <= 128
fn sort_entries(entries: &mut [DirEntry], count: usize) {
    for i in 1..count {
        let key = entries[i];
        let mut j = i;
        while j > 0 && entry_gt(&entries[j - 1], &key) {
            entries[j] = entries[j - 1];
            j -= 1;
        }
        entries[j] = key;
    }
}

// directories before files, then case-insensitive name order
fn entry_gt(a: &DirEntry, b: &DirEntry) -> bool {
    if a.is_dir != b.is_dir {
        return !a.is_dir;
    }
    let an = a.name_str().as_bytes();
    let bn = b.name_str().as_bytes();
    for (ab, bb) in an.iter().zip(bn.iter()) {
        let ac = ab.to_ascii_lowercase();
        let bc = bb.to_ascii_lowercase();
        if ac != bc {
            return ac > bc;
        }
    }
    an.len() > bn.len()
}
