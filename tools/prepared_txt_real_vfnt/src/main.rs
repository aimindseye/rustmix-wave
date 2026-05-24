use fontdue::{Font, FontSettings, Metrics};
use rustybuzz::{script, Direction, Face, UnicodeBuffer};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const VFNT_MAGIC: &[u8; 4] = b"VFNT";
const VFNT_VERSION: u16 = 1;
const VFNT_HEADER_LEN: usize = 44;
const VFNT_METRICS_LEN: usize = 16;
const VFNT_BITMAP_LEN: usize = 16;
const VFNT_ONE_BPP: u16 = 1;
const SCRIPT_LATIN: u16 = 1;
const SCRIPT_DEVANAGARI: u16 = 2;

const VRUN_MAGIC: &[u8; 4] = b"VRUN";
const VRUN_VERSION: u16 = 1;
const VRUN_HEADER_LEN: usize = 20;
const VRUN_RECORD_LEN: usize = 20;

const FONT_LATIN: u32 = 1;
const FONT_DEVANAGARI: u32 = 2;
const LATIN_FILE: &str = "LAT18.VFN";
const DEVANAGARI_FILE: &str = "DEV22.VFN";

#[derive(Clone, Debug)]
struct Args {
    book: PathBuf,
    device_path: String,
    latin_font: PathBuf,
    devanagari_font: PathBuf,
    out: PathBuf,
    title: String,
    latin_size: f32,
    devanagari_size: f32,
    line_height: Option<i16>,
    page_width: i16,
    page_height: i16,
    margin_x: i16,
    margin_y: i16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScriptKind {
    Latin,
    Devanagari,
}

#[derive(Clone, Debug)]
struct TextRun<'a> {
    script: ScriptKind,
    text: &'a str,
}

#[derive(Clone, Copy, Debug)]
struct ShapedGlyph {
    font_id: u32,
    glyph_id: u32,
    x_offset: i16,
    y_offset: i16,
    advance_x: i16,
    advance_y: i16,
    cluster: u32,
}

#[derive(Clone, Copy, Debug)]
struct PositionedGlyph {
    font_id: u32,
    glyph_id: u32,
    x: i16,
    y: i16,
    advance_x: i16,
    advance_y: i16,
    cluster: u32,
}

#[derive(Clone, Debug)]
struct PreparedPage {
    glyphs: Vec<PositionedGlyph>,
}

#[derive(Clone, Debug)]
struct GlyphAsset {
    glyph_id: u32,
    advance_x: i16,
    bearing_x: i16,
    bearing_y: i16,
    width: u16,
    height: u16,
    row_stride: u16,
    bitmap: Vec<u8>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let text =
        fs::read_to_string(&args.book).map_err(|err| format!("read book as UTF-8: {err}"))?;
    let latin_data = read_required_file(&args.latin_font, "Latin font")?;
    let devanagari_data = read_required_file(&args.devanagari_font, "Devanagari font")?;

    let mut latin_face =
        Face::from_slice(&latin_data, 0).ok_or("parse Latin font with rustybuzz")?;
    latin_face.set_points_per_em(Some(args.latin_size));
    let mut devanagari_face =
        Face::from_slice(&devanagari_data, 0).ok_or("parse Devanagari font with rustybuzz")?;
    devanagari_face.set_points_per_em(Some(args.devanagari_size));

    let latin_font = Font::from_bytes(latin_data.clone(), FontSettings::default())
        .map_err(|err| format!("parse Latin font for rasterization: {err}"))?;
    let devanagari_font = Font::from_bytes(devanagari_data.clone(), FontSettings::default())
        .map_err(|err| format!("parse Devanagari font for rasterization: {err}"))?;

    let layout = layout_text(
        &args,
        &text,
        &latin_face,
        &devanagari_face,
        &latin_font,
        &devanagari_font,
    )?;
    if layout.pages.iter().all(|page| page.glyphs.is_empty()) {
        return Err("prepared page has no glyphs".to_string());
    }

    let latin_assets = rasterize_used_glyphs(
        &latin_font,
        args.latin_size,
        layout.used_latin.iter().copied(),
    )?;
    let devanagari_assets = rasterize_used_glyphs(
        &devanagari_font,
        args.devanagari_size,
        layout.used_devanagari.iter().copied(),
    )?;

    let latin_vfnt = build_vfnt(SCRIPT_LATIN, args.latin_size, &latin_assets)?;
    let devanagari_vfnt = build_vfnt(SCRIPT_DEVANAGARI, args.devanagari_size, &devanagari_assets)?;
    validate_vfnt(&latin_vfnt)?;
    validate_vfnt(&devanagari_vfnt)?;

    let book_id = book_folder_for_path(&args.device_path);
    let cache_dir = args.out.join(&book_id);
    fs::create_dir_all(&cache_dir).map_err(|err| format!("create cache directory: {err}"))?;
    fs::write(cache_dir.join(LATIN_FILE), &latin_vfnt)
        .map_err(|err| format!("write {LATIN_FILE}: {err}"))?;
    fs::write(cache_dir.join(DEVANAGARI_FILE), &devanagari_vfnt)
        .map_err(|err| format!("write {DEVANAGARI_FILE}: {err}"))?;

    let latin_ids = vfnt_glyph_ids(&latin_vfnt)?;
    let devanagari_ids = vfnt_glyph_ids(&devanagari_vfnt)?;
    let mut page_names = Vec::new();
    for (index, page) in layout.pages.iter().enumerate() {
        validate_page_references(page, &latin_ids, &devanagari_ids)?;
        let name = format!("P{index:03}.VRN");
        let data = build_vrun(page, args.page_width as u16, args.page_height as u16)?;
        fs::write(cache_dir.join(&name), data).map_err(|err| format!("write {name}: {err}"))?;
        page_names.push(name);
    }

    fs::write(
        cache_dir.join("FONTS.IDX"),
        format!("Latin={LATIN_FILE}\nDevanagari={DEVANAGARI_FILE}\n"),
    )
    .map_err(|err| format!("write FONTS.IDX: {err}"))?;
    fs::write(cache_dir.join("PAGES.IDX"), page_names.join("\n") + "\n")
        .map_err(|err| format!("write PAGES.IDX: {err}"))?;
    fs::write(
        cache_dir.join("META.TXT"),
        format!(
            "book_id={book_id}\nsource=/{}\ntitle={}\npage_count={}\nlatin_font={LATIN_FILE}\ndevanagari_font={DEVANAGARI_FILE}\npages=PAGES.IDX\n",
            args.device_path.trim_start_matches('/'),
            args.title,
            page_names.len()
        ),
    )
    .map_err(|err| format!("write META.TXT: {err}"))?;

    println!("book_id={book_id}");
    println!("prepared cache: {}", cache_dir.display());
    println!("pages={}", page_names.len());
    println!("latin_glyphs={}", latin_ids.len());
    println!("devanagari_glyphs={}", devanagari_ids.len());
    Ok(())
}

struct LayoutResult {
    pages: Vec<PreparedPage>,
    used_latin: BTreeSet<u32>,
    used_devanagari: BTreeSet<u32>,
}

fn layout_text(
    args: &Args,
    text: &str,
    latin_face: &Face<'_>,
    devanagari_face: &Face<'_>,
    latin_font: &Font,
    devanagari_font: &Font,
) -> Result<LayoutResult, String> {
    let line_height = args.line_height.unwrap_or_else(|| {
        (args.latin_size.max(args.devanagari_size).ceil() as i16).saturating_add(8)
    });
    let baseline_start = args.margin_y + line_height;
    let mut pages = vec![PreparedPage { glyphs: Vec::new() }];
    let mut page_index = 0usize;
    let mut x = args.margin_x;
    let mut baseline = baseline_start;
    let mut used_latin = BTreeSet::new();
    let mut used_devanagari = BTreeSet::new();

    for line in text.split('\n') {
        for run in split_script_runs(line) {
            let shaped = match run.script {
                ScriptKind::Latin => shape_run(
                    run.text,
                    ScriptKind::Latin,
                    latin_face,
                    args.latin_size,
                    FONT_LATIN,
                )?,
                ScriptKind::Devanagari => shape_run(
                    run.text,
                    ScriptKind::Devanagari,
                    devanagari_face,
                    args.devanagari_size,
                    FONT_DEVANAGARI,
                )?,
            };
            for glyph in shaped {
                if x + glyph.advance_x > args.page_width && x > args.margin_x {
                    next_line(
                        &mut pages,
                        &mut page_index,
                        &mut x,
                        &mut baseline,
                        args,
                        line_height,
                        baseline_start,
                    );
                }
                let (metrics, _) = font_metrics_for(
                    glyph.font_id,
                    glyph.glyph_id,
                    args,
                    latin_font,
                    devanagari_font,
                )?;
                let draw_x = x
                    .saturating_add(glyph.x_offset)
                    .saturating_add(metrics.xmin as i16);
                let draw_y = baseline
                    .saturating_sub(metrics.height as i16)
                    .saturating_sub(metrics.ymin as i16)
                    .saturating_add(glyph.y_offset);
                pages[page_index].glyphs.push(PositionedGlyph {
                    font_id: glyph.font_id,
                    glyph_id: glyph.glyph_id,
                    x: draw_x,
                    y: draw_y,
                    advance_x: glyph.advance_x,
                    advance_y: glyph.advance_y,
                    cluster: glyph.cluster,
                });
                match glyph.font_id {
                    FONT_LATIN => {
                        used_latin.insert(glyph.glyph_id);
                    }
                    FONT_DEVANAGARI => {
                        used_devanagari.insert(glyph.glyph_id);
                    }
                    _ => {}
                }
                x = x.saturating_add(glyph.advance_x);
            }
        }
        next_line(
            &mut pages,
            &mut page_index,
            &mut x,
            &mut baseline,
            args,
            line_height,
            baseline_start,
        );
    }

    Ok(LayoutResult {
        pages,
        used_latin,
        used_devanagari,
    })
}

fn font_metrics_for(
    font_id: u32,
    glyph_id: u32,
    args: &Args,
    latin_font: &Font,
    devanagari_font: &Font,
) -> Result<(Metrics, Vec<u8>), String> {
    let font = if font_id == FONT_LATIN {
        latin_font
    } else {
        devanagari_font
    };
    let size = if font_id == FONT_LATIN {
        args.latin_size
    } else {
        args.devanagari_size
    };
    rasterize_one(font, size, glyph_id)
}

fn next_line(
    pages: &mut Vec<PreparedPage>,
    page_index: &mut usize,
    x: &mut i16,
    baseline: &mut i16,
    args: &Args,
    line_height: i16,
    baseline_start: i16,
) {
    *x = args.margin_x;
    *baseline = baseline.saturating_add(line_height);
    if *baseline + line_height > args.page_height {
        pages.push(PreparedPage { glyphs: Vec::new() });
        *page_index += 1;
        *baseline = baseline_start;
    }
}

fn shape_run(
    text: &str,
    script_kind: ScriptKind,
    face: &Face<'_>,
    px: f32,
    font_id: u32,
) -> Result<Vec<ShapedGlyph>, String> {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(Direction::LeftToRight);
    buffer.set_script(match script_kind {
        ScriptKind::Latin => script::LATIN,
        ScriptKind::Devanagari => script::DEVANAGARI,
    });
    let shaped = rustybuzz::shape(face, &[], buffer);
    let scale = px / face.units_per_em() as f32;
    let infos = shaped.glyph_infos();
    let positions = shaped.glyph_positions();
    let mut out = Vec::with_capacity(infos.len());
    for (info, pos) in infos.iter().zip(positions.iter()) {
        out.push(ShapedGlyph {
            font_id,
            glyph_id: info.glyph_id,
            x_offset: scale_i32(pos.x_offset, scale),
            y_offset: -scale_i32(pos.y_offset, scale),
            advance_x: scale_i32(pos.x_advance, scale).max(0),
            advance_y: scale_i32(pos.y_advance, scale),
            cluster: info.cluster,
        });
    }
    Ok(out)
}

fn split_script_runs(input: &str) -> Vec<TextRun<'_>> {
    let mut runs = Vec::new();
    let mut start = 0usize;
    let mut current = None;
    for (index, ch) in input.char_indices() {
        let script = if is_devanagari(ch) {
            ScriptKind::Devanagari
        } else {
            ScriptKind::Latin
        };
        if let Some(active) = current {
            if active != script {
                runs.push(TextRun {
                    script: active,
                    text: &input[start..index],
                });
                start = index;
                current = Some(script);
            }
        } else {
            current = Some(script);
            start = index;
        }
    }
    if let Some(script) = current {
        runs.push(TextRun {
            script,
            text: &input[start..],
        });
    }
    runs
}

fn is_devanagari(ch: char) -> bool {
    ('\u{0900}'..='\u{097F}').contains(&ch)
}

fn rasterize_used_glyphs<I>(font: &Font, px: f32, glyph_ids: I) -> Result<Vec<GlyphAsset>, String>
where
    I: Iterator<Item = u32>,
{
    let mut assets = Vec::new();
    for glyph_id in glyph_ids {
        let (metrics, grayscale) = rasterize_one(font, px, glyph_id)?;
        let (bitmap, row_stride) = coverage_to_one_bpp(&grayscale, metrics.width, metrics.height);
        assets.push(GlyphAsset {
            glyph_id,
            advance_x: metrics.advance_width.round() as i16,
            bearing_x: metrics.xmin as i16,
            bearing_y: metrics.ymin as i16,
            width: metrics.width as u16,
            height: metrics.height as u16,
            row_stride,
            bitmap,
        });
    }
    Ok(assets)
}

fn rasterize_one(font: &Font, px: f32, glyph_id: u32) -> Result<(Metrics, Vec<u8>), String> {
    let glyph_index =
        u16::try_from(glyph_id).map_err(|_| format!("glyph id too large: {glyph_id}"))?;
    Ok(font.rasterize_indexed(glyph_index, px))
}

fn coverage_to_one_bpp(grayscale: &[u8], width: usize, height: usize) -> (Vec<u8>, u16) {
    let row_stride = width.div_ceil(8);
    let mut out = vec![0u8; row_stride * height];
    for y in 0..height {
        for x in 0..width {
            if grayscale[y * width + x] >= 96 {
                out[y * row_stride + x / 8] |= 1 << (7 - (x & 7));
            }
        }
    }
    (out, row_stride as u16)
}

fn build_vfnt(script_code: u16, pixel_size: f32, glyphs: &[GlyphAsset]) -> Result<Vec<u8>, String> {
    if glyphs.is_empty() {
        return Err("font has no used glyphs".to_string());
    }
    let mut sorted = glyphs.to_vec();
    sorted.sort_by_key(|glyph| glyph.glyph_id);

    let metrics_offset = VFNT_HEADER_LEN;
    let bitmap_index_offset = metrics_offset + sorted.len() * VFNT_METRICS_LEN;
    let bitmap_data_offset = bitmap_index_offset + sorted.len() * VFNT_BITMAP_LEN;
    let mut metrics = Vec::new();
    let mut bitmap_index = Vec::new();
    let mut bitmap_data = Vec::new();
    for glyph in &sorted {
        let offset = bitmap_data.len() as u32;
        bitmap_data.extend_from_slice(&glyph.bitmap);
        put_u32(&mut metrics, glyph.glyph_id);
        put_i16(&mut metrics, glyph.advance_x);
        put_i16(&mut metrics, 0);
        put_i16(&mut metrics, glyph.bearing_x);
        put_i16(&mut metrics, glyph.bearing_y);
        put_u16(&mut metrics, glyph.width);
        put_u16(&mut metrics, glyph.height);

        put_u32(&mut bitmap_index, glyph.glyph_id);
        put_u32(&mut bitmap_index, offset);
        put_u32(&mut bitmap_index, glyph.bitmap.len() as u32);
        put_u16(&mut bitmap_index, glyph.row_stride);
        put_u16(&mut bitmap_index, 0);
    }

    let line_height = (pixel_size.ceil() as u16).saturating_add(8);
    let ascent = pixel_size.ceil() as i16;
    let descent = -((line_height as i16).saturating_sub(ascent));
    let mut out = Vec::new();
    out.extend_from_slice(VFNT_MAGIC);
    put_u16(&mut out, VFNT_VERSION);
    put_u16(&mut out, VFNT_HEADER_LEN as u16);
    put_u32(&mut out, 0);
    put_u16(&mut out, pixel_size.round() as u16);
    put_u16(&mut out, line_height);
    put_i16(&mut out, ascent);
    put_i16(&mut out, descent);
    put_u32(&mut out, sorted.len() as u32);
    put_u32(&mut out, metrics_offset as u32);
    put_u32(&mut out, bitmap_index_offset as u32);
    put_u32(&mut out, bitmap_data_offset as u32);
    put_u32(&mut out, bitmap_data.len() as u32);
    put_u16(&mut out, script_code);
    put_u16(&mut out, VFNT_ONE_BPP);
    out.extend(metrics);
    out.extend(bitmap_index);
    out.extend(bitmap_data);
    Ok(out)
}

fn build_vrun(page: &PreparedPage, width: u16, height: u16) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(VRUN_HEADER_LEN + page.glyphs.len() * VRUN_RECORD_LEN);
    out.extend_from_slice(VRUN_MAGIC);
    put_u16(&mut out, VRUN_VERSION);
    put_u16(&mut out, VRUN_HEADER_LEN as u16);
    put_u32(&mut out, page.glyphs.len() as u32);
    put_u16(&mut out, width);
    put_u16(&mut out, height);
    put_u32(&mut out, 0);
    for glyph in &page.glyphs {
        put_u32(&mut out, glyph.font_id);
        put_u32(&mut out, glyph.glyph_id);
        put_i16(&mut out, glyph.x);
        put_i16(&mut out, glyph.y);
        put_i16(&mut out, glyph.advance_x);
        put_i16(&mut out, glyph.advance_y);
        put_u32(&mut out, glyph.cluster);
    }
    Ok(out)
}

fn validate_page_references(
    page: &PreparedPage,
    latin_ids: &BTreeSet<u32>,
    devanagari_ids: &BTreeSet<u32>,
) -> Result<(), String> {
    for glyph in &page.glyphs {
        match glyph.font_id {
            FONT_LATIN if latin_ids.contains(&glyph.glyph_id) => {}
            FONT_DEVANAGARI if devanagari_ids.contains(&glyph.glyph_id) => {}
            FONT_LATIN | FONT_DEVANAGARI => {
                return Err(format!("VRN references missing glyph {}", glyph.glyph_id));
            }
            _ => {
                return Err(format!(
                    "VRN references unknown font slot {}",
                    glyph.font_id
                ))
            }
        }
    }
    Ok(())
}

fn validate_vfnt(data: &[u8]) -> Result<(), String> {
    if data.len() < VFNT_HEADER_LEN {
        return Err("VFNT header is truncated".to_string());
    }
    if &data[0..4] != VFNT_MAGIC {
        return Err("VFNT magic is invalid".to_string());
    }
    if read_u16(data, 4)? != VFNT_VERSION {
        return Err("VFNT version is unsupported".to_string());
    }
    if read_u16(data, 42)? != VFNT_ONE_BPP {
        return Err("VFNT bitmap format is unsupported".to_string());
    }
    let count = read_u32(data, 20)? as usize;
    let metrics_offset = read_u32(data, 24)? as usize;
    let bitmap_index_offset = read_u32(data, 28)? as usize;
    let bitmap_data_offset = read_u32(data, 32)? as usize;
    let bitmap_data_len = read_u32(data, 36)? as usize;
    checked_range(data.len(), metrics_offset, count * VFNT_METRICS_LEN)?;
    checked_range(data.len(), bitmap_index_offset, count * VFNT_BITMAP_LEN)?;
    checked_range(data.len(), bitmap_data_offset, bitmap_data_len)?;
    for index in 0..count {
        let off = bitmap_index_offset + index * VFNT_BITMAP_LEN;
        let bitmap_offset = read_u32(data, off + 4)? as usize;
        let bitmap_len = read_u32(data, off + 8)? as usize;
        checked_range(bitmap_data_len, bitmap_offset, bitmap_len)?;
    }
    Ok(())
}

fn vfnt_glyph_ids(data: &[u8]) -> Result<BTreeSet<u32>, String> {
    validate_vfnt(data)?;
    let count = read_u32(data, 20)? as usize;
    let metrics_offset = read_u32(data, 24)? as usize;
    let mut ids = BTreeSet::new();
    for index in 0..count {
        ids.insert(read_u32(data, metrics_offset + index * VFNT_METRICS_LEN)?);
    }
    Ok(ids)
}

fn checked_range(total: usize, start: usize, len: usize) -> Result<(), String> {
    let end = start
        .checked_add(len)
        .ok_or_else(|| "range overflows".to_string())?;
    if start <= total && end <= total {
        Ok(())
    } else {
        Err("range is outside buffer".to_string())
    }
}

fn book_folder_for_path(path: &str) -> String {
    let mut value: u32 = 0x811C9DC5;
    for byte in normalized_path_key(path).bytes() {
        value ^= u32::from(byte);
        value = value.wrapping_mul(0x01000193);
    }
    format!("{value:08X}")
}

fn normalized_path_key(path: &str) -> String {
    path.chars()
        .map(|ch| if ch == '\\' { '/' } else { ch })
        .flat_map(char::to_lowercase)
        .collect()
}

fn read_required_file(path: &Path, label: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|err| format!("{label} missing or unreadable: {err}"))
}

fn parse_args() -> Result<Args, String> {
    let mut values = BTreeMap::new();
    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--help" || arg == "-h" {
            return Err(usage());
        }
        if !arg.starts_with("--") {
            return Err(format!("unexpected argument: {arg}\n{}", usage()));
        }
        let Some(value) = iter.next() else {
            return Err(format!("missing value for {arg}"));
        };
        values.insert(arg, value);
    }
    let book = required_path(&values, "--book")?;
    let title = values
        .get("--title")
        .cloned()
        .unwrap_or_else(|| "Prepared TXT Smoke".to_string());
    Ok(Args {
        book,
        device_path: required_string(&values, "--device-path")?,
        latin_font: required_path(&values, "--latin-font")?,
        devanagari_font: required_path(&values, "--devanagari-font")?,
        out: required_path(&values, "--out")?,
        title,
        latin_size: optional_f32(&values, "--latin-size", 18.0)?,
        devanagari_size: optional_f32(&values, "--devanagari-size", 22.0)?,
        line_height: optional_i16(&values, "--line-height")?,
        page_width: optional_i16(&values, "--page-width")?.unwrap_or(464),
        page_height: optional_i16(&values, "--page-height")?.unwrap_or(730),
        margin_x: optional_i16(&values, "--margin-x")?.unwrap_or(0),
        margin_y: optional_i16(&values, "--margin-y")?.unwrap_or(4),
    })
}

fn usage() -> String {
    "usage: prepared-txt-real-vfnt --book <TXT> --device-path <PATH> --latin-font <TTF> --devanagari-font <TTF> --out <FCACHE>".to_string()
}

fn required_path(values: &BTreeMap<String, String>, key: &str) -> Result<PathBuf, String> {
    Ok(PathBuf::from(required_string(values, key)?))
}

fn required_string(values: &BTreeMap<String, String>, key: &str) -> Result<String, String> {
    values
        .get(key)
        .cloned()
        .ok_or_else(|| format!("missing required argument {key}"))
}

fn optional_f32(values: &BTreeMap<String, String>, key: &str, default: f32) -> Result<f32, String> {
    match values.get(key) {
        Some(value) => value.parse().map_err(|_| format!("invalid {key}: {value}")),
        None => Ok(default),
    }
}

fn optional_i16(values: &BTreeMap<String, String>, key: &str) -> Result<Option<i16>, String> {
    values
        .get(key)
        .map(|value| value.parse().map_err(|_| format!("invalid {key}: {value}")))
        .transpose()
}

fn scale_i32(value: i32, scale: f32) -> i16 {
    (value as f32 * scale)
        .round()
        .clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_i16(out: &mut Vec<u8>, value: i16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, String> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| "read past end".to_string())?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, String> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| "read past end".to_string())?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_existing_reader_book_id_policy() {
        assert_eq!(book_folder_for_path("MIXED.TXT"), "5E3BBD2E");
        assert_eq!(
            book_folder_for_path("Books\\MIXED.TXT"),
            book_folder_for_path("books/mixed.txt")
        );
    }

    #[test]
    fn splits_latin_and_devanagari_runs() {
        let runs = split_script_runs("Om ॐ नमः");
        assert_eq!(runs[0].script, ScriptKind::Latin);
        assert_eq!(runs[1].script, ScriptKind::Devanagari);
    }

    #[test]
    fn generated_vfnt_contains_non_empty_bitmap() {
        let glyph = GlyphAsset {
            glyph_id: 42,
            advance_x: 5,
            bearing_x: 0,
            bearing_y: 0,
            width: 3,
            height: 2,
            row_stride: 1,
            bitmap: vec![0b1110_0000, 0b1010_0000],
        };
        let data = build_vfnt(SCRIPT_LATIN, 18.0, &[glyph]).unwrap();
        validate_vfnt(&data).unwrap();
        assert!(vfnt_glyph_ids(&data).unwrap().contains(&42));
        assert!(data.ends_with(&[0b1110_0000, 0b1010_0000]));
    }

    #[test]
    fn generated_vrun_references_multiple_positioned_glyphs() {
        let page = PreparedPage {
            glyphs: vec![
                PositionedGlyph {
                    font_id: FONT_LATIN,
                    glyph_id: 1,
                    x: 0,
                    y: 0,
                    advance_x: 6,
                    advance_y: 0,
                    cluster: 0,
                },
                PositionedGlyph {
                    font_id: FONT_DEVANAGARI,
                    glyph_id: 2,
                    x: 8,
                    y: 1,
                    advance_x: 10,
                    advance_y: 0,
                    cluster: 1,
                },
            ],
        };
        let data = build_vrun(&page, 464, 730).unwrap();
        assert_eq!(&data[0..4], VRUN_MAGIC);
        assert_eq!(read_u32(&data, 8).unwrap(), 2);
        assert_eq!(data.len(), VRUN_HEADER_LEN + 2 * VRUN_RECORD_LEN);
    }

    #[test]
    fn rejects_missing_font_files() {
        let result = read_required_file(Path::new("/definitely/missing/font.ttf"), "font");
        assert!(result.is_err());
    }
}
