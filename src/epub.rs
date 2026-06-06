// rustmix-wave=v0.17.0-parser-doc-repair-v2
// rustmix-wave=epub-xml-attribute-tokenizer-repair-ready
// rustmix-wave=epub-parser-stack-isolation-ready
// rustmix-wave=epub-chapter-aware-presentation-ready
// rustmix-wave=epub-watchdog-memory-pressure-repair-ready
//! Bounded reflowable EPUB reader foundation.
//!
//! The embedded target keeps EPUB processing deliberately small and explicit:
//! ZIP central-directory parsing is bounded, `META-INF/container.xml` selects
//! one OPF package, the manifest and spine are parsed without a general XML DOM,
//! XHTML is flattened into reflowable UTF-8 text, and EPUB3 navigation or EPUB2
//! NCX records become a compact table of contents. Images, CSS layout and
//! interactive links remain deferred.

use std::{collections::BTreeMap, fs, path::Path};

use miniz_oxide::inflate::decompress_to_vec;

/// Maximum EPUB archive bytes accepted from removable storage.
pub const EPUB_ARCHIVE_BYTES_LIMIT: usize = 16 * 1024 * 1024;
/// Maximum central-directory records accepted from one EPUB.
pub const EPUB_ARCHIVE_ENTRY_LIMIT: usize = 512;
/// Maximum compressed bytes extracted for one EPUB member.
pub const EPUB_MEMBER_COMPRESSED_LIMIT: usize = 2 * 1024 * 1024;
/// Maximum decompressed bytes extracted for one EPUB member.
pub const EPUB_MEMBER_UNCOMPRESSED_LIMIT: usize = 4 * 1024 * 1024;
/// Maximum flattened reflowable text retained in RAM for one EPUB.
pub const EPUB_REFLOW_TEXT_LIMIT: usize = 2 * 1024 * 1024;
/// Maximum manifest records retained from one OPF package.
pub const EPUB_MANIFEST_LIMIT: usize = 256;
/// Maximum spine records retained from one OPF package.
pub const EPUB_SPINE_LIMIT: usize = 128;
/// Maximum TOC records rendered by the Reader UI.
pub const EPUB_TOC_LIMIT: usize = 128;
/// Dedicated parser-worker stack budget. Real EPUB DEFLATE and XHTML work
/// must not run on the 16 KB firmware main task.
pub const EPUB_PARSER_WORKER_STACK_BYTES: usize = 64 * 1024;
/// Lightweight OPF-title worker stack budget. Library scans only read bounded
/// ZIP metadata and must not reserve the full parser stack for each title.
pub const EPUB_TITLE_WORKER_STACK_BYTES: usize = 32 * 1024;

/// One reflowable EPUB TOC destination. `text_offset` is an offset into the
/// flattened UTF-8 text buffer retained by [`EpubDocument`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpubTocEntry {
    pub label: String,
    pub text_offset: u64,
    pub spine_index: usize,
}

/// One readable spine chapter retained alongside flattened EPUB text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpubChapter {
    /// Sequential readable chapter number exposed by the Reader UI.
    pub number: usize,
    pub label: String,
    pub text_offset: u64,
    pub text_end_offset: u64,
    pub spine_index: usize,
}

/// One bounded, reflowable EPUB book retained while the Reader session is open.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpubDocument {
    pub title: String,
    pub text: String,
    pub toc: Vec<EpubTocEntry>,
    pub chapters: Vec<EpubChapter>,
    pub spine_count: usize,
}

impl EpubDocument {
    #[must_use]
    pub fn text_size_bytes(&self) -> u64 {
        self.text.len() as u64
    }

    /// Resolve the readable chapter containing one flattened UTF-8 byte offset.
    #[must_use]
    pub fn chapter_for_offset(&self, offset: u64) -> Option<&EpubChapter> {
        self.chapters.iter().find(|chapter| {
            offset >= chapter.text_offset
                && (offset < chapter.text_end_offset
                    || (offset == chapter.text_end_offset
                        && chapter.text_end_offset == self.text_size_bytes()))
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ZipEntry {
    name: String,
    flags: u16,
    method: u16,
    compressed_size: usize,
    uncompressed_size: usize,
    local_header_offset: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ZipArchive {
    bytes: Vec<u8>,
    entries: Vec<ZipEntry>,
}

impl ZipArchive {
    fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes =
            fs::read(path.as_ref()).map_err(|error| format!("EPUB open failed: {error}"))?;
        if bytes.len() > EPUB_ARCHIVE_BYTES_LIMIT {
            return Err(format!(
                "EPUB archive exceeds {} byte limit",
                EPUB_ARCHIVE_BYTES_LIMIT
            ));
        }
        let eocd = find_eocd(&bytes).ok_or_else(|| "EPUB ZIP end record missing".to_string())?;
        let entry_count = read_u16(&bytes, eocd + 10)? as usize;
        let central_size = read_u32(&bytes, eocd + 12)? as usize;
        let central_offset = read_u32(&bytes, eocd + 16)? as usize;
        if entry_count > EPUB_ARCHIVE_ENTRY_LIMIT {
            return Err(format!("EPUB ZIP has too many entries: {entry_count}"));
        }
        let central_end = central_offset
            .checked_add(central_size)
            .ok_or_else(|| "EPUB ZIP directory overflow".to_string())?;
        if central_end > bytes.len() {
            return Err("EPUB ZIP directory exceeds archive".into());
        }
        let mut entries = Vec::new();
        let mut cursor = central_offset;
        for _ in 0..entry_count {
            if read_u32(&bytes, cursor)? != 0x0201_4B50 {
                return Err("EPUB ZIP central record signature mismatch".into());
            }
            let flags = read_u16(&bytes, cursor + 8)?;
            let method = read_u16(&bytes, cursor + 10)?;
            let compressed_size = read_u32(&bytes, cursor + 20)? as usize;
            let uncompressed_size = read_u32(&bytes, cursor + 24)? as usize;
            let name_len = read_u16(&bytes, cursor + 28)? as usize;
            let extra_len = read_u16(&bytes, cursor + 30)? as usize;
            let comment_len = read_u16(&bytes, cursor + 32)? as usize;
            let local_header_offset = read_u32(&bytes, cursor + 42)? as usize;
            let name_start = cursor + 46;
            let name_end = name_start
                .checked_add(name_len)
                .ok_or_else(|| "EPUB ZIP filename overflow".to_string())?;
            if name_end > central_end {
                return Err("EPUB ZIP filename exceeds directory".into());
            }
            let name = String::from_utf8_lossy(&bytes[name_start..name_end]).replace('\\', "/");
            entries.push(ZipEntry {
                name,
                flags,
                method,
                compressed_size,
                uncompressed_size,
                local_header_offset,
            });
            cursor = name_end
                .checked_add(extra_len)
                .and_then(|value| value.checked_add(comment_len))
                .ok_or_else(|| "EPUB ZIP central record overflow".to_string())?;
            if cursor > central_end {
                return Err("EPUB ZIP central record exceeds directory".into());
            }
        }
        Ok(Self { bytes, entries })
    }

    fn entry(&self, name: &str) -> Option<&ZipEntry> {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|entry| entry.name.eq_ignore_ascii_case(name))
            })
    }

    fn extract(&self, name: &str) -> Result<Vec<u8>, String> {
        let entry = self
            .entry(name)
            .ok_or_else(|| format!("EPUB member missing: {name}"))?;
        if entry.flags & 0x0001 != 0 {
            return Err(format!(
                "Encrypted EPUB member is unsupported: {}",
                entry.name
            ));
        }
        if entry.compressed_size > EPUB_MEMBER_COMPRESSED_LIMIT {
            return Err(format!(
                "EPUB member compressed size is too large: {}",
                entry.name
            ));
        }
        if entry.uncompressed_size > EPUB_MEMBER_UNCOMPRESSED_LIMIT {
            return Err(format!(
                "EPUB member expanded size is too large: {}",
                entry.name
            ));
        }
        let offset = entry.local_header_offset;
        if read_u32(&self.bytes, offset)? != 0x0403_4B50 {
            return Err(format!("EPUB local ZIP header mismatch: {}", entry.name));
        }
        let name_len = read_u16(&self.bytes, offset + 26)? as usize;
        let extra_len = read_u16(&self.bytes, offset + 28)? as usize;
        let data_start = offset
            .checked_add(30)
            .and_then(|value| value.checked_add(name_len))
            .and_then(|value| value.checked_add(extra_len))
            .ok_or_else(|| "EPUB ZIP data offset overflow".to_string())?;
        let data_end = data_start
            .checked_add(entry.compressed_size)
            .ok_or_else(|| "EPUB ZIP member overflow".to_string())?;
        if data_end > self.bytes.len() {
            return Err(format!("EPUB ZIP member exceeds archive: {}", entry.name));
        }
        let compressed = &self.bytes[data_start..data_end];
        let output = match entry.method {
            0 => compressed.to_vec(),
            8 => decompress_to_vec(compressed)
                .map_err(|error| format!("EPUB deflate failed for {}: {error:?}", entry.name))?,
            method => {
                return Err(format!(
                    "Unsupported EPUB compression method {method} for {}",
                    entry.name
                ))
            }
        };
        if output.len() > EPUB_MEMBER_UNCOMPRESSED_LIMIT {
            return Err(format!("EPUB member expanded beyond limit: {}", entry.name));
        }
        if entry.uncompressed_size != 0 && output.len() != entry.uncompressed_size {
            return Err(format!("EPUB member size mismatch: {}", entry.name));
        }
        Ok(output)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ManifestItem {
    id: String,
    href: String,
    media_type: String,
    properties: String,
}

/// Parse one EPUB on a short-lived dedicated worker stack. The Reader keeps
/// its existing synchronous staged-loading contract, while archive parsing,
/// DEFLATE expansion and XHTML flattening no longer consume the firmware main
/// task's 16 KB stack budget.
pub fn open_epub_on_worker(path: impl AsRef<Path>) -> Result<EpubDocument, String> {
    let path = path.as_ref().to_path_buf();
    log::info!(
        "rustmix-wave=epub-parser-worker status=starting stack-bytes={}",
        EPUB_PARSER_WORKER_STACK_BYTES
    );
    let worker = std::thread::Builder::new()
        .name("epub-parser".into())
        .stack_size(EPUB_PARSER_WORKER_STACK_BYTES)
        .spawn(move || open_epub(path))
        .map_err(|error| {
            let message = format!("EPUB parser worker start failed: {error}");
            log::warn!("rustmix-wave=epub-parser-worker status=start-failed error={message}");
            message
        })?;
    let result = worker.join().map_err(|_| {
        let message = "EPUB parser worker panicked".to_string();
        log::warn!("rustmix-wave=epub-parser-worker status=panicked");
        message
    })?;
    match &result {
        Ok(document) => log::info!(
            "rustmix-wave=epub-parser-worker status=completed spine-items={} toc-entries={} text-bytes={}",
            document.spine_count,
            document.toc.len(),
            document.text_size_bytes()
        ),
        Err(error) => log::warn!("rustmix-wave=epub-parser-worker status=failed error={error}"),
    }
    result
}

/// Read only the OPF title on a lightweight bounded worker stack. Library scans
/// remain safe on the firmware main task and fall back to the FAT filename when
/// metadata cannot be read.
pub fn read_epub_title_on_worker(path: impl AsRef<Path>) -> Result<String, String> {
    let path = path.as_ref().to_path_buf();
    let worker = std::thread::Builder::new()
        .name("epub-title".into())
        .stack_size(EPUB_TITLE_WORKER_STACK_BYTES)
        .spawn(move || read_epub_title(path))
        .map_err(|error| format!("EPUB title worker start failed: {error}"))?;
    worker
        .join()
        .map_err(|_| "EPUB title worker panicked".to_string())?
}

/// Read one OPF metadata title without flattening the spine.
#[inline(never)]
pub fn read_epub_title(path: impl AsRef<Path>) -> Result<String, String> {
    let archive = ZipArchive::open(path)?;
    let (_, package, _) = epub_package(&archive)?;
    Ok(package_title(&package))
}

fn epub_package(archive: &ZipArchive) -> Result<(String, String, String), String> {
    let container = utf8_member(archive, "META-INF/container.xml")?;
    let rootfile = first_open_tag(&container, "rootfile")
        .and_then(|tag| attribute(tag, "full-path"))
        .ok_or_else(|| "EPUB container rootfile missing".to_string())?;
    let package_path = normalize_archive_path("", &rootfile);
    let package = utf8_member(archive, &package_path)?;
    let package_dir = archive_parent(&package_path);
    Ok((package_path, package, package_dir))
}

fn package_title(package: &str) -> String {
    first_element_text(package, "title")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Untitled EPUB".into())
}

/// Open one EPUB archive and produce a bounded reflowable document.
#[inline(never)]
pub fn open_epub(path: impl AsRef<Path>) -> Result<EpubDocument, String> {
    let archive = ZipArchive::open(path)?;
    let (_, package, package_dir) = epub_package(&archive)?;
    let title = package_title(&package);

    let manifest = parse_manifest(&package)?;
    let spine_ids = parse_spine_ids(&package)?;
    if spine_ids.is_empty() {
        return Err("EPUB spine is empty".into());
    }

    let mut text = String::new();
    let mut chapter_offsets = BTreeMap::new();
    let mut chapter_labels = Vec::new();
    let mut chapters = Vec::new();
    for (spine_index, idref) in spine_ids.iter().enumerate() {
        let item = manifest
            .get(idref)
            .ok_or_else(|| format!("EPUB spine item missing from manifest: {idref}"))?;
        let member = normalize_archive_path(&package_dir, &item.href);
        let xhtml = utf8_member(&archive, &member)?;
        let chapter = html_to_text(&xhtml);
        if chapter.trim().is_empty() {
            continue;
        }
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        let offset = text.len() as u64;
        let label = fallback_chapter_label(&xhtml, spine_index);
        chapter_offsets.insert(member.clone(), (spine_index, offset));
        chapter_labels.push((spine_index, offset, label.clone()));
        text.push_str(chapter.trim());
        let text_end_offset = text.len() as u64;
        chapters.push(EpubChapter {
            number: chapters.len() + 1,
            label,
            text_offset: offset,
            text_end_offset,
            spine_index,
        });
        if text.len() > EPUB_REFLOW_TEXT_LIMIT {
            return Err(format!(
                "EPUB reflow text exceeds {} byte limit",
                EPUB_REFLOW_TEXT_LIMIT
            ));
        }
    }
    if text.trim().is_empty() {
        return Err("EPUB spine did not contain readable text".into());
    }

    let mut toc = parse_navigation_toc(
        &archive,
        &package,
        &package_dir,
        &manifest,
        &chapter_offsets,
    )?;
    if toc.is_empty() {
        toc = chapter_labels
            .into_iter()
            .take(EPUB_TOC_LIMIT)
            .map(|(spine_index, text_offset, label)| EpubTocEntry {
                label,
                text_offset,
                spine_index,
            })
            .collect();
    }
    dedupe_toc(&mut toc);
    toc.truncate(EPUB_TOC_LIMIT);
    Ok(EpubDocument {
        title,
        text,
        toc,
        chapters,
        spine_count: spine_ids.len(),
    })
}

fn parse_manifest(package: &str) -> Result<BTreeMap<String, ManifestItem>, String> {
    let mut manifest = BTreeMap::new();
    for tag in open_tags(package, "item")
        .into_iter()
        .take(EPUB_MANIFEST_LIMIT)
    {
        let Some(id) = attribute(tag, "id") else {
            continue;
        };
        let Some(href) = attribute(tag, "href") else {
            continue;
        };
        let media_type = attribute(tag, "media-type").unwrap_or_default();
        let properties = attribute(tag, "properties").unwrap_or_default();
        manifest.insert(
            id.clone(),
            ManifestItem {
                id,
                href,
                media_type,
                properties,
            },
        );
    }
    if manifest.is_empty() {
        return Err("EPUB manifest is empty".into());
    }
    Ok(manifest)
}

fn parse_spine_ids(package: &str) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    for tag in open_tags(package, "itemref")
        .into_iter()
        .take(EPUB_SPINE_LIMIT)
    {
        if let Some(idref) = attribute(tag, "idref") {
            ids.push(idref);
        }
    }
    Ok(ids)
}

fn parse_navigation_toc(
    archive: &ZipArchive,
    package: &str,
    package_dir: &str,
    manifest: &BTreeMap<String, ManifestItem>,
    chapter_offsets: &BTreeMap<String, (usize, u64)>,
) -> Result<Vec<EpubTocEntry>, String> {
    if let Some(nav) = manifest.values().find(|item| {
        item.properties
            .split_whitespace()
            .any(|value| value == "nav")
    }) {
        let member = normalize_archive_path(package_dir, &nav.href);
        let nav_text = utf8_member(archive, &member)?;
        let base = archive_parent(&member);
        let toc = links_to_toc(&nav_text, &base, chapter_offsets);
        if !toc.is_empty() {
            return Ok(toc);
        }
    }

    let spine_toc = first_open_tag(package, "spine").and_then(|tag| attribute(tag, "toc"));
    let ncx = spine_toc
        .as_ref()
        .and_then(|id| manifest.get(id))
        .or_else(|| {
            manifest
                .values()
                .find(|item| item.media_type == "application/x-dtbncx+xml")
        });
    if let Some(ncx) = ncx {
        let member = normalize_archive_path(package_dir, &ncx.href);
        let ncx_text = utf8_member(archive, &member)?;
        let base = archive_parent(&member);
        return Ok(ncx_to_toc(&ncx_text, &base, chapter_offsets));
    }
    Ok(Vec::new())
}

fn links_to_toc(
    html: &str,
    base: &str,
    chapter_offsets: &BTreeMap<String, (usize, u64)>,
) -> Vec<EpubTocEntry> {
    let mut toc = Vec::new();
    let mut cursor = 0;
    while toc.len() < EPUB_TOC_LIMIT {
        let Some(start_rel) = html[cursor..].find("<a") else {
            break;
        };
        let start = cursor + start_rel;
        let Some(open_end_rel) = html[start..].find('>') else {
            break;
        };
        let open_end = start + open_end_rel;
        let tag = &html[start + 1..open_end];
        let Some(href) = attribute(tag, "href") else {
            cursor = open_end + 1;
            continue;
        };
        let Some(close_rel) = html[open_end + 1..].find("</a>") else {
            break;
        };
        let close = open_end + 1 + close_rel;
        let label = html_to_text(&html[open_end + 1..close]);
        if let Some(entry) = toc_for_href(base, &href, label.trim(), chapter_offsets) {
            toc.push(entry);
        }
        cursor = close + 4;
    }
    toc
}

fn ncx_to_toc(
    ncx: &str,
    base: &str,
    chapter_offsets: &BTreeMap<String, (usize, u64)>,
) -> Vec<EpubTocEntry> {
    let mut toc = Vec::new();
    let mut cursor = 0;
    while toc.len() < EPUB_TOC_LIMIT {
        let Some(start_rel) = ncx[cursor..].find("<navPoint") else {
            break;
        };
        let start = cursor + start_rel;
        let end = ncx[start..]
            .find("</navPoint>")
            .map_or(ncx.len(), |value| start + value + "</navPoint>".len());
        let block = &ncx[start..end];
        let href = first_open_tag(block, "content").and_then(|tag| attribute(tag, "src"));
        let label = first_element_text(block, "text").unwrap_or_else(|| "Chapter".into());
        if let Some(href) = href {
            if let Some(entry) = toc_for_href(base, &href, label.trim(), chapter_offsets) {
                toc.push(entry);
            }
        }
        cursor = end;
    }
    toc
}

fn toc_for_href(
    base: &str,
    href: &str,
    label: &str,
    chapter_offsets: &BTreeMap<String, (usize, u64)>,
) -> Option<EpubTocEntry> {
    let member = normalize_archive_path(base, href);
    let (spine_index, text_offset) = chapter_offsets.get(&member).copied()?;
    Some(EpubTocEntry {
        label: if label.is_empty() {
            format!("Chapter {}", spine_index + 1)
        } else {
            label.to_string()
        },
        text_offset,
        spine_index,
    })
}

fn dedupe_toc(toc: &mut Vec<EpubTocEntry>) {
    let mut unique = Vec::new();
    for entry in toc.drain(..) {
        if unique.iter().any(|existing: &EpubTocEntry| {
            existing.text_offset == entry.text_offset && existing.label == entry.label
        }) {
            continue;
        }
        unique.push(entry);
    }
    *toc = unique;
}

fn fallback_chapter_label(xhtml: &str, spine_index: usize) -> String {
    for tag in ["h1", "h2", "h3", "title"] {
        if let Some(label) = first_element_text(xhtml, tag).filter(|value| !value.is_empty()) {
            return label;
        }
    }
    format!("Chapter {}", spine_index + 1)
}

fn utf8_member(archive: &ZipArchive, name: &str) -> Result<String, String> {
    let bytes = archive.extract(name)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn find_eocd(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 22 {
        return None;
    }
    let start = bytes.len().saturating_sub(65_557);
    (start..=bytes.len() - 22)
        .rev()
        .find(|offset| bytes.get(*offset..*offset + 4) == Some(&[0x50_u8, 0x4B, 0x05, 0x06][..]))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "EPUB ZIP truncated u16".to_string())?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "EPUB ZIP truncated u32".to_string())?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn archive_parent(path: &str) -> String {
    path.rsplit_once('/')
        .map_or("".into(), |(parent, _)| parent.into())
}

fn normalize_archive_path(base: &str, href: &str) -> String {
    let href = href.split('#').next().unwrap_or("");
    let decoded = percent_decode(href);
    let raw = if decoded.starts_with('/') || base.is_empty() {
        decoded.trim_start_matches('/').to_string()
    } else {
        format!("{base}/{decoded}")
    };
    let mut parts = Vec::new();
    for part in raw.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            value => parts.push(value),
        }
    }
    parts.join("/")
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let high = hex_value(bytes[index + 1]);
            let low = hex_value(bytes[index + 2]);
            if let (Some(high), Some(low)) = (high, low) {
                output.push((high << 4) | low);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&output).into_owned()
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn first_open_tag<'a>(xml: &'a str, local_name: &str) -> Option<&'a str> {
    open_tags(xml, local_name).into_iter().next()
}

fn open_tags<'a>(xml: &'a str, local_name: &str) -> Vec<&'a str> {
    let mut tags = Vec::new();
    let mut cursor = 0;
    while cursor < xml.len() {
        let Some(start_rel) = xml[cursor..].find('<') else {
            break;
        };
        let start = cursor + start_rel;
        let Some(end_rel) = xml[start + 1..].find('>') else {
            break;
        };
        let end = start + 1 + end_rel;
        let tag = &xml[start + 1..end];
        let trimmed = tag.trim_start();
        if !trimmed.starts_with('/') && !trimmed.starts_with('!') && !trimmed.starts_with('?') {
            let name = trimmed
                .split(|character: char| character.is_whitespace() || character == '/')
                .next()
                .unwrap_or("");
            if name.rsplit(':').next() == Some(local_name) {
                tags.push(tag);
            }
        }
        cursor = end + 1;
    }
    tags
}

fn attribute(tag: &str, key: &str) -> Option<String> {
    let bytes = tag.as_bytes();
    let mut cursor = 0;

    // Skip the opening element name before scanning attribute tokens. Without
    // this boundary, `<rootfile full-path='...'>` consumes `full-path` while
    // recovering from the non-attribute `rootfile` token.
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() && bytes[cursor] != b'/' {
        cursor += 1;
    }

    while cursor < bytes.len() {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] == b'/' {
            break;
        }

        let start = cursor;
        while cursor < bytes.len()
            && !bytes[cursor].is_ascii_whitespace()
            && bytes[cursor] != b'='
            && bytes[cursor] != b'/'
        {
            cursor += 1;
        }
        let name = &tag[start..cursor];
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'=' {
            while cursor < bytes.len()
                && !bytes[cursor].is_ascii_whitespace()
                && bytes[cursor] != b'/'
            {
                cursor += 1;
            }
            continue;
        }

        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || (bytes[cursor] != b'\'' && bytes[cursor] != b'"') {
            while cursor < bytes.len()
                && !bytes[cursor].is_ascii_whitespace()
                && bytes[cursor] != b'/'
            {
                cursor += 1;
            }
            continue;
        }

        let quote = bytes[cursor];
        cursor += 1;
        let value_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != quote {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            break;
        }
        let value = &tag[value_start..cursor];
        cursor += 1;
        if name.rsplit(':').next() == Some(key) {
            return Some(decode_entities(value));
        }
    }
    None
}

fn first_element_text(xml: &str, local_name: &str) -> Option<String> {
    let mut cursor = 0;
    while cursor < xml.len() {
        let Some(start_rel) = xml[cursor..].find('<') else {
            return None;
        };
        let start = cursor + start_rel;
        let Some(end_rel) = xml[start + 1..].find('>') else {
            return None;
        };
        let end = start + 1 + end_rel;
        let tag = &xml[start + 1..end];
        let trimmed = tag.trim_start();
        let name = trimmed
            .split(|character: char| character.is_whitespace() || character == '/')
            .next()
            .unwrap_or("");
        if !trimmed.starts_with('/') && name.rsplit(':').next() == Some(local_name) {
            let close = format!("</{name}>");
            if let Some(close_rel) = xml[end + 1..].find(&close) {
                return Some(html_to_text(&xml[end + 1..end + 1 + close_rel]));
            }
        }
        cursor = end + 1;
    }
    None
}

/// Convert XHTML into bounded, paragraph-aware reflowable UTF-8 text.
pub fn html_to_text(html: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < html.len() {
        let rest = &html[cursor..];
        if rest.starts_with('<') {
            let Some(end_rel) = rest.find('>') else { break };
            let tag = rest[1..end_rel].trim();
            let closing = tag.starts_with('/');
            let name = tag
                .trim_start_matches('/')
                .split(|character: char| character.is_whitespace() || character == '/')
                .next()
                .unwrap_or("")
                .rsplit(':')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if matches!(
                name.as_str(),
                "p" | "div"
                    | "section"
                    | "article"
                    | "blockquote"
                    | "li"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "br"
            ) {
                push_newline(&mut output);
                if closing || name == "br" {
                    push_newline(&mut output);
                }
            }
            cursor += end_rel + 1;
            continue;
        }
        if rest.starts_with('&') {
            if let Some(end_rel) = rest.find(';').filter(|value| *value <= 12) {
                let decoded = decode_entity(&rest[1..end_rel]);
                for character in decoded.chars() {
                    push_text_character(&mut output, character);
                }
                cursor += end_rel + 1;
                continue;
            }
        }
        let character = rest.chars().next().unwrap_or(' ');
        push_text_character(&mut output, character);
        cursor += character.len_utf8();
    }
    output.trim().to_string()
}

fn push_text_character(output: &mut String, character: char) {
    if character == '\r' {
        return;
    }
    if character == '\n' || character == '\t' || character.is_whitespace() {
        if !output.ends_with(' ') && !output.ends_with('\n') && !output.is_empty() {
            output.push(' ');
        }
    } else {
        output.push(character);
    }
}

fn push_newline(output: &mut String) {
    while output.ends_with(' ') {
        output.pop();
    }
    if !output.ends_with("\n\n") && !output.is_empty() {
        output.push('\n');
    }
}

fn decode_entities(value: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < value.len() {
        let rest = &value[cursor..];
        if rest.starts_with('&') {
            if let Some(end_rel) = rest.find(';').filter(|value| *value <= 12) {
                output.push_str(&decode_entity(&rest[1..end_rel]));
                cursor += end_rel + 1;
                continue;
            }
        }
        let character = rest.chars().next().unwrap_or(' ');
        output.push(character);
        cursor += character.len_utf8();
    }
    output
}

fn decode_entity(entity: &str) -> String {
    match entity {
        "amp" => "&".into(),
        "lt" => "<".into(),
        "gt" => ">".into(),
        "quot" => "\"".into(),
        "apos" => "'".into(),
        "nbsp" => " ".into(),
        value if value.starts_with("#x") || value.starts_with("#X") => {
            u32::from_str_radix(&value[2..], 16)
                .ok()
                .and_then(char::from_u32)
                .map_or_else(|| "?".into(), |character| character.to_string())
        }
        value if value.starts_with('#') => value[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map_or_else(|| "?".into(), |character| character.to_string()),
        _ => "?".into(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        attribute, first_open_tag, html_to_text, open_epub, open_epub_on_worker,
        read_epub_title_on_worker, EPUB_PARSER_WORKER_STACK_BYTES, EPUB_TITLE_WORKER_STACK_BYTES,
    };

    fn temp_epub(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rustmix-{name}-{nonce}.epu"))
    }

    fn push_u16(output: &mut Vec<u8>, value: u16) {
        output.extend(value.to_le_bytes());
    }
    fn push_u32(output: &mut Vec<u8>, value: u32) {
        output.extend(value.to_le_bytes());
    }

    fn stored_zip(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut output = Vec::new();
        let mut central = Vec::new();
        for (name, body) in entries {
            let offset = output.len() as u32;
            push_u32(&mut output, 0x0403_4B50);
            push_u16(&mut output, 20);
            push_u16(&mut output, 0);
            push_u16(&mut output, 0);
            push_u16(&mut output, 0);
            push_u16(&mut output, 0);
            push_u32(&mut output, 0);
            push_u32(&mut output, body.len() as u32);
            push_u32(&mut output, body.len() as u32);
            push_u16(&mut output, name.len() as u16);
            push_u16(&mut output, 0);
            output.extend(name.as_bytes());
            output.extend(body.as_bytes());

            push_u32(&mut central, 0x0201_4B50);
            push_u16(&mut central, 20);
            push_u16(&mut central, 20);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u32(&mut central, 0);
            push_u32(&mut central, body.len() as u32);
            push_u32(&mut central, body.len() as u32);
            push_u16(&mut central, name.len() as u16);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u32(&mut central, 0);
            push_u32(&mut central, offset);
            central.extend(name.as_bytes());
        }
        let central_offset = output.len() as u32;
        let central_size = central.len() as u32;
        output.extend(central);
        push_u32(&mut output, 0x0605_4B50);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, entries.len() as u16);
        push_u16(&mut output, entries.len() as u16);
        push_u32(&mut output, central_size);
        push_u32(&mut output, central_offset);
        push_u16(&mut output, 0);
        output
    }

    #[test]
    fn xml_attribute_tokenizer_reads_attributes_after_element_name() {
        let tag =
            "rootfile full-path='OEBPS/book.opf' media-type=\"application/oebps-package+xml\"/";
        assert_eq!(
            attribute(tag, "full-path").as_deref(),
            Some("OEBPS/book.opf")
        );
        assert_eq!(
            attribute(tag, "media-type").as_deref(),
            Some("application/oebps-package+xml")
        );
    }

    #[test]
    fn container_rootfile_lookup_ignores_plural_wrapper() {
        let container =
            "<container><rootfiles><rootfile full-path='OEBPS/book.opf'/></rootfiles></container>";
        let rootfile =
            first_open_tag(container, "rootfile").and_then(|tag| attribute(tag, "full-path"));
        assert_eq!(rootfile.as_deref(), Some("OEBPS/book.opf"));
    }

    #[test]
    fn parser_worker_stack_budget_is_explicit() {
        assert_eq!(EPUB_PARSER_WORKER_STACK_BYTES, 64 * 1024);
    }

    #[test]
    fn title_worker_uses_a_smaller_bounded_stack() {
        assert_eq!(EPUB_TITLE_WORKER_STACK_BYTES, 32 * 1024);
        assert!(EPUB_TITLE_WORKER_STACK_BYTES < EPUB_PARSER_WORKER_STACK_BYTES);
    }

    #[test]
    fn flattens_xhtml_into_reflowable_paragraphs() {
        assert_eq!(
            html_to_text("<h1>Title</h1><p>Hello &amp; goodbye.</p>"),
            "Title\n\nHello & goodbye."
        );
    }

    #[test]
    fn opens_stored_epub_manifest_spine_and_nav_toc() {
        let path = temp_epub("stored");
        let bytes = stored_zip(&[
            ("META-INF/container.xml", "<container><rootfiles><rootfile full-path='OEBPS/book.opf'/></rootfiles></container>"),
            ("OEBPS/book.opf", "<package><metadata><dc:title>Sample EPUB</dc:title></metadata><manifest><item id='nav' href='nav.xhtml' media-type='application/xhtml+xml' properties='nav'/><item id='c1' href='c1.xhtml' media-type='application/xhtml+xml'/><item id='c2' href='c2.xhtml' media-type='application/xhtml+xml'/></manifest><spine><itemref idref='c1'/><itemref idref='c2'/></spine></package>"),
            ("OEBPS/nav.xhtml", "<nav><ol><li><a href='c1.xhtml'>Start</a></li><li><a href='c2.xhtml'>Second</a></li></ol></nav>"),
            ("OEBPS/c1.xhtml", "<html><body><h1>Start</h1><p>First chapter.</p></body></html>"),
            ("OEBPS/c2.xhtml", "<html><body><h1>Second</h1><p>Second chapter.</p></body></html>"),
        ]);
        fs::write(&path, bytes).unwrap();
        let epub = open_epub(&path).unwrap();
        let worker_epub = open_epub_on_worker(&path).unwrap();
        assert_eq!(worker_epub, epub);
        assert_eq!(epub.title, "Sample EPUB");
        assert_eq!(epub.spine_count, 2);
        assert_eq!(epub.chapters.len(), 2);
        assert_eq!(epub.chapters[0].number, 1);
        assert_eq!(epub.chapters[1].number, 2);
        assert_eq!(
            epub.chapter_for_offset(epub.chapters[1].text_offset)
                .unwrap()
                .number,
            2
        );
        assert_eq!(read_epub_title_on_worker(&path).unwrap(), "Sample EPUB");
        assert!(epub.text.contains("First chapter."));
        assert!(epub.text.contains("Second chapter."));
        assert_eq!(epub.toc.len(), 2);
        assert_eq!(epub.toc[0].label, "Start");
        assert!(epub.toc[1].text_offset > epub.toc[0].text_offset);
        let _ = fs::remove_file(path);
    }
}
