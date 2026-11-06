//! Reader landing, library, AO3 blurb cards, bookmarks, loading, TXT / EPUB page, TOC and options screens.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        reader_typography::reader_body_style,
        state::AppState,
        typography::{Text, TextBounds, UiTextRole},
        widgets::{
            footer::draw_footer,
            header::draw_header,
            status_row::{draw_status_row, StatusRow},
        },
    },
    epub::EpubAo3Meta,
    orientation::OrientedFrameBuffer,
    reader::{
        Ao3ScrollLine, BookFormat, ParagraphAlignment, ReaderBook, ReaderLibraryTab,
        ReaderLoadingStage, ReaderOption, ReadingPreference, ReadingTheme, FANFIC_VIEWPORT_LINES,
    },
};

pub fn render_continue_reading(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(
        display,
        state.display,
        "CONTINUE READING",
        "PERSISTENT BOOK RESUME",
    )?;
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    if let Some(session) = state.reader.session.as_ref() {
        Text::new(
            &truncate(&session.book.title, 38),
            Point::new(24, 190),
            heading,
        )
        .draw(display)?;
        Text::new(
            &format!(
                "Runtime page {} is ready.",
                session.current_absolute_page() + 1
            ),
            Point::new(24, 240),
            body,
        )
        .draw(display)?;
        Text::new("SELECT resumes the open page.", Point::new(24, 284), body).draw(display)?;
    } else if let Some(resume) = state.reader.resume.as_ref() {
        Text::new(&truncate(&resume.title, 38), Point::new(24, 190), heading).draw(display)?;
        Text::new(
            &format!("Saved page {} is ready to restore.", resume.page_index + 1),
            Point::new(24, 240),
            body,
        )
        .draw(display)?;
        Text::new(
            "SELECT loads the saved position.",
            Point::new(24, 284),
            body,
        )
        .draw(display)?;
    } else {
        Text::new("No saved book", Point::new(24, 190), heading).draw(display)?;
        Text::new(
            "Open Library and choose a TXT or EPUB work.",
            Point::new(24, 240),
            body,
        )
        .draw(display)?;
        Text::new(
            "The last-read page is stored on the SD card.",
            Point::new(24, 284),
            body,
        )
        .draw(display)?;
    }
    draw_footer(display, state.display, "SELECT RESUME  HOLD BOOT BACK")
}

pub fn render_library(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let reader = &state.reader;
    let body = state.display.body_style();
    let detail = state.display.detail_style();

    if reader.library_tab == ReaderLibraryTab::Fanfic {
        draw_header(
            display,
            state.display,
            "AO3 // ARCHIVE OF OUR OWN",
            "RUSTMIX/BOOKS/FANFIC",
        )?;
    } else {
        draw_header(display, state.display, "LIBRARY", "TXT / REFLOWABLE EPUB")?;
    }

    let status = library_status(reader.library_tab, reader.visible_entries().len());
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: status.left,
            middle: &status.middle,
            right: status.right,
        },
    )?;
    draw_tabs(display, state, reader.library_tab)?;

    let control_selected = reader.library_selected == 0;
    draw_row(
        display,
        state,
        182,
        control_selected,
        "Change tab (Books / Fanfic / Marks)",
        "SELECT",
        "TAB",
    )?;

    let visible = reader.visible_entries();
    if visible.is_empty() {
        let message = reader
            .library_error
            .as_deref()
            .unwrap_or(match reader.library_tab {
                ReaderLibraryTab::Recent => "No recent books yet.",
                ReaderLibraryTab::Fanfic => "Put .EPU/.TXT works in /RUSTMIX/BOOKS/FANFIC.",
                ReaderLibraryTab::Bookmarks => "No saved bookmarks yet.",
                _ => "Copy TXT or EPUB books into /RUSTMIX/BOOKS.",
            });
        Text::new(&truncate(message, 52), Point::new(24, 302), body).draw(display)?;
        return draw_footer(
            display,
            state.display,
            "MOVE  SELECT OPEN/TAB  HOLD BOOT BACK",
        );
    }

    if reader.library_tab == ReaderLibraryTab::Fanfic {
        let active_idx = reader.library_selected.saturating_sub(1);
        let page_start = (active_idx / 2) * 2;
        for (slot, index) in (page_start..(page_start + 2).min(visible.len())).enumerate() {
            let selected = reader.library_selected == index + 1;
            let card_top = 240 + (slot as i32) * 246;
            draw_ao3_blurb_card(
                display,
                state,
                card_top,
                &visible[index].book,
                selected,
                None,
            )?;
        }
        return draw_footer(
            display,
            state.display,
            "UP/DOWN BROWSE AO3  SELECT READ  BOOT BACK",
        );
    }

    for (index, entry) in visible.iter().take(7).enumerate() {
        let selected = reader.library_selected == index + 1;
        let columns = library_entry_columns(reader, entry);
        draw_row(
            display,
            state,
            248 + index as i32 * 58,
            selected,
            &truncate(&entry.book.title, 25),
            columns.badge.as_str(),
            columns.suffix.as_str(),
        )?;
    }
    if reader.library_tab != ReaderLibraryTab::Bookmarks {
        Text::new(
            "TXT and EPUB open with staged first-page loading.",
            Point::new(24, 716),
            detail,
        )
        .draw(display)?;
    }
    draw_footer(
        display,
        state.display,
        "MOVE  SELECT OPEN/TAB  HOLD BOOT BACK",
    )
}

fn draw_ao3_blurb_card(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    top: i32,
    book: &ReaderBook,
    selected: bool,
    page_info: Option<&str>,
) -> Result<(), Infallible> {
    let default_meta = EpubAo3Meta::default();
    let meta = book.ao3.as_ref().unwrap_or(&default_meta);
    let theme = state.reader.preferences.theme;
    let georgia_md = reader_body_style(crate::reader::BookFont::Serif, crate::reader::BookFontSize::Medium, theme);
    let lucida_body = reader_body_style(crate::reader::BookFont::Inter, crate::reader::BookFontSize::Small, theme);
    let detail = state.display.detail_style();

    Rectangle::new(Point::new(14, top), Size::new(452, 236))
        .into_styled(PrimitiveStyle::with_stroke(
            BinaryColor::On,
            if selected { 3 } else { 1 },
        ))
        .draw(display)?;

    let gx = 24;
    let gy = top + 12;
    for (dx, dy, sym, invert) in [
        (0, 0, meta.rating_text.chars().next().unwrap_or('T'), selected),
        (24, 0, meta.category_text.chars().next().unwrap_or('/'), false),
        (0, 24, meta.warning_text.chars().next().unwrap_or('!'), false),
        (24, 24, '=', false),
    ] {
        let cell_pt = Point::new(gx + dx, gy + dy);
        if invert {
            Rectangle::new(cell_pt, Size::new(22, 22))
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display)?;
            Text::new(
                &sym.to_string(),
                Point::new(gx + dx + 6, gy + dy + 16),
                state.display.text_style(UiTextRole::Detail, BinaryColor::Off),
            )
            .draw(display)?;
        } else {
            Rectangle::new(cell_pt, Size::new(22, 22))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)?;
            Text::new(&sym.to_string(), Point::new(gx + dx + 6, gy + dy + 16), detail).draw(display)?;
        }
    }

    let prefix = if selected { "> " } else { "" };
    Text::new(
        &truncate(&format!("{prefix}{}", book.title), 26),
        Point::new(80, top + 28),
        georgia_md,
    )
    .draw(display)?;
    Text::new(
        &truncate(&format!("by {}", meta.author), 34),
        Point::new(80, top + 48),
        lucida_body,
    )
    .draw(display)?;
    Text::new(&truncate(&meta.fandoms, 38), Point::new(80, top + 64), detail).draw(display)?;

    Rectangle::new(Point::new(24, top + 72), Size::new(432, 1))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;

    let mut tx = 24;
    let ty = top + 78;
    for tag in meta.additional_tags.split(',') {
        let clean = tag.trim().to_string();
        if clean.is_empty() {
            continue;
        }
        let pill_w = (clean.len() as i32 * 7 + 12).min(140);
        if tx + pill_w > 454 {
            break;
        }
        Rectangle::new(Point::new(tx, ty), Size::new(pill_w as u32, 22))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display)?;
        Text::new(&truncate(&clean, 16), Point::new(tx + 6, ty + 16), detail).draw(display)?;
        tx += pill_w + 6;
    }

    Rectangle::new(Point::new(28, top + 108), Size::new(3, 82))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    let sum_text = if meta.summary.is_empty() {
        "Archive of Our Own reflowable work."
    } else {
        meta.summary.as_str()
    };
    let s_line1: String = sum_text.chars().take(44).collect();
    let s_line2: String = sum_text.chars().skip(44).take(44).collect();
    Text::new(&s_line1, Point::new(40, top + 126), lucida_body).draw(display)?;
    if !s_line2.is_empty() {
        Text::new(&s_line2, Point::new(40, top + 152), lucida_body).draw(display)?;
    }

    Rectangle::new(Point::new(24, top + 198), Size::new(432, 1))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    let words = if meta.words.is_empty() {
        format!("{} KB", book.size_bytes / 1024)
    } else {
        meta.words.clone()
    };
    let right_stat = page_info.unwrap_or("Format: EPUB");
    Text::new(
        &format!("Words: {words}   |   {right_stat}"),
        Point::new(24, top + 220),
        detail,
    )
    .draw(display)?;
    Ok(())
}

fn draw_dotted_underline(
    display: &mut OrientedFrameBuffer<'_>,
    x: i32,
    y: i32,
    width: i32,
) -> Result<(), Infallible> {
    let mut cx = x;
    let end = (x + width).min(452);
    while cx < end {
        Rectangle::new(Point::new(cx, y), Size::new(2, 1))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(display)?;
        cx += 4;
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LibraryStatus {
    left: &'static str,
    middle: String,
    right: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LibraryEntryColumns {
    badge: String,
    suffix: String,
}

fn library_status(tab: ReaderLibraryTab, entry_count: usize) -> LibraryStatus {
    if tab == ReaderLibraryTab::Bookmarks {
        LibraryStatus {
            left: "Bookmarks",
            middle: format!("{entry_count} saved"),
            right: "MARKS.TXT",
        }
    } else if tab == ReaderLibraryTab::Fanfic {
        LibraryStatus {
            left: "AO3 Works",
            middle: format!("{entry_count} fics"),
            right: "BOOKS/FANFIC",
        }
    } else {
        LibraryStatus {
            left: tab.label(),
            middle: format!("{entry_count} books"),
            right: "SD BOOKS",
        }
    }
}

fn library_entry_columns(
    reader: &crate::reader::ReaderUiState,
    entry: &crate::reader::ReaderLibraryEntry,
) -> LibraryEntryColumns {
    if reader.library_tab == ReaderLibraryTab::Bookmarks {
        let page = entry
            .location
            .as_ref()
            .map_or(1, |bookmark| reader.bookmark_display_page(bookmark));
        if let Some(chapter) = entry
            .location
            .as_ref()
            .and_then(|bookmark| reader.bookmark_display_chapter_page(bookmark))
        {
            LibraryEntryColumns {
                badge: format!("CH {}", chapter.chapter_number),
                suffix: format!("P {}", chapter.page_text()),
            }
        } else {
            LibraryEntryColumns {
                badge: "PAGE".into(),
                suffix: page.to_string(),
            }
        }
    } else {
        LibraryEntryColumns {
            badge: entry.book.format.badge().into(),
            suffix: "OPEN".into(),
        }
    }
}

fn bookmark_entry_columns(
    reader: &crate::reader::ReaderUiState,
    bookmark: &crate::reader::ReaderLocation,
) -> LibraryEntryColumns {
    if let Some(chapter) = reader.bookmark_display_chapter_page(bookmark) {
        LibraryEntryColumns {
            badge: format!("CH {}", chapter.chapter_number),
            suffix: format!("P {}", chapter.page_text()),
        }
    } else {
        LibraryEntryColumns {
            badge: "PAGE".into(),
            suffix: reader.bookmark_display_page(bookmark).to_string(),
        }
    }
}

pub fn render_bookmarks(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(
        display,
        state.display,
        "BOOKMARKS",
        "PERSISTENT READER MARKS",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "MARKS.TXT",
            middle: &format!("{} saved", state.reader.bookmarks.len()),
            right: "SD FILE",
        },
    )?;
    let body = state.display.body_style();
    if state.reader.bookmarks.is_empty() {
        Text::new(
            "No saved bookmarks",
            Point::new(24, 210),
            state.display.heading_style(),
        )
        .draw(display)?;
        Text::new(
            "Open a Reader page, choose Reader Options,",
            Point::new(24, 264),
            body,
        )
        .draw(display)?;
        Text::new(
            "then select Add / Remove Bookmark.",
            Point::new(24, 306),
            body,
        )
        .draw(display)?;
    } else {
        for (index, bookmark) in state.reader.bookmarks.iter().take(8).enumerate() {
            let top = 164 + index as i32 * 64;
            let columns = bookmark_entry_columns(&state.reader, bookmark);
            draw_row(
                display,
                state,
                top,
                state.reader.bookmarks_selected == index,
                &truncate(&bookmark.title, 23),
                columns.badge.as_str(),
                columns.suffix.as_str(),
            )?;
        }
    }
    draw_footer(display, state.display, "MOVE  SELECT OPEN  HOLD BOOT BACK")
}

pub fn render_loading(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(
        display,
        state.display,
        "OPENING WORK",
        "RESPONSIVE CACHE",
    )?;
    let body = state.display.body_style();
    let heading = state.display.heading_style();
    let loading = state.reader.loading.as_ref();
    let title = loading.map_or("Book", |value| value.book.title.as_str());
    let stage = loading.map_or(ReaderLoadingStage::OpeningFile, |value| value.stage);
    Text::new(&truncate(title, 36), Point::new(24, 176), heading).draw(display)?;
    Text::new(stage.label(), Point::new(24, 238), body).draw(display)?;
    draw_progress(display, stage.progress())?;
    let message = loading.map_or("Preparing reader...", |value| value.message.as_str());
    Text::new(&truncate(message, 52), Point::new(24, 356), body).draw(display)?;
    draw_footer(display, state.display, "HOLD BOOT CANCEL")
}

pub fn render_page(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let Some(session) = state.reader.session.as_ref() else {
        return render_continue_reading(display, state);
    };
    if session.is_fanfic {
        return render_ao3_continuous_scroll_page(display, state, session);
    }

    let size = display.orientation().logical_size();
    let width = size.width as i32;
    let height = size.height as i32;
    let landscape = width > height;
    let header_height = if landscape { 52 } else { 70 };
    let status_top = header_height + 10;
    let status_height = if landscape { 34 } else { 42 };
    let footer_line = height - 54;
    let body = ReaderBodyGeometry::new(width, status_top, status_height, footer_line);
    let body_style = reader_body_style(
        state.reader.preferences.book_font,
        state.reader.preferences.font_size,
        state.reader.preferences.theme,
    );
    let ui_body = state.display.body_style();
    let ui_detail = state.display.detail_style();

    Rectangle::new(
        Point::new(0, 0),
        Size::new(size.width, header_height as u32),
    )
    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
    .draw(display)?;
    Text::new(
        &truncate(&session.book.title, if landscape { 52 } else { 27 }),
        Point::new(18, if landscape { 28 } else { 32 }),
        state.display.header_title_style(),
    )
    .draw(display)?;
    Text::new(
        if session.book.format == BookFormat::Text {
            "TXT READER"
        } else {
            "EPUB REFLOWABLE"
        },
        Point::new(18, if landscape { 48 } else { 60 }),
        state.display.header_subtitle_style(),
    )
    .draw(display)?;

    Rectangle::new(
        Point::new(14, status_top),
        Size::new((width - 28) as u32, status_height as u32),
    )
    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
    .draw(display)?;
    let status_baseline = status_top + status_height - 10;
    let marked = state.reader.current_page_is_bookmarked();
    if state.reader.preferences.show_progress {
        Text::new(
            if session.book.format == BookFormat::Text {
                session.encoding.label()
            } else {
                "EPUB"
            },
            Point::new(24, status_baseline),
            ui_body,
        )
        .draw(display)?;
        Text::new(
            &session.display_page_label(),
            Point::new(if landscape { 274 } else { 176 }, status_baseline),
            ui_body,
        )
        .draw(display)?;
        let cache_label = format!("CACHE {}%", session.progress_percent());
        Text::new(
            if marked {
                "MARKED"
            } else {
                cache_label.as_str()
            },
            Point::new(if landscape { 590 } else { 358 }, status_baseline),
            ui_body,
        )
        .draw(display)?;
    } else {
        Text::new(
            state.reader.preferences.book_font.label(),
            Point::new(24, status_baseline),
            ui_body,
        )
        .draw(display)?;
        Text::new(
            session.content_badge(),
            Point::new(if landscape { 370 } else { 210 }, status_baseline),
            ui_body,
        )
        .draw(display)?;
        if marked {
            Text::new(
                "MARKED",
                Point::new(if landscape { 590 } else { 358 }, status_baseline),
                ui_body,
            )
            .draw(display)?;
        }
    }

    if state.reader.preferences.theme == ReadingTheme::HighContrast {
        Rectangle::new(
            Point::new(body.frame.left, body.frame.top),
            Size::new(body.frame.width() as u32, body.frame.height() as u32),
        )
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
        .draw(display)?;
    }

    if let Some(page) = session.current_cached_page() {
        let line_step = i32::from(body_style.line_height()) + 2;
        let first_baseline = body.text.top + i32::from(body_style.line_height());
        for (index, line) in page
            .lines
            .iter()
            .take(session.layout.lines_per_page)
            .enumerate()
        {
            let baseline = first_baseline + index as i32 * line_step;
            if baseline >= body.text.bottom {
                break;
            }
            let (rendered, left) = aligned_reader_line(
                line.text.as_str(),
                line.paragraph_end,
                session.layout.paragraph_alignment,
                body_style,
                body.text,
            );
            Text::new(rendered.as_str(), Point::new(left, baseline), body_style)
                .draw_clipped(display, body.text)?;
        }
    } else {
        let baseline = body.text.top + i32::from(body_style.line_height());
        Text::new(
            "Preparing page...",
            Point::new(body.text.left, baseline),
            body_style,
        )
        .draw_clipped(display, body.text)?;
    }

    Rectangle::new(
        Point::new(14, footer_line),
        Size::new((width - 28) as u32, 1),
    )
    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
    .draw(display)?;
    Text::new(
        if landscape {
            "UP PREV  DOWN NEXT  SELECT OPTIONS"
        } else {
            "UP previous   DOWN next   SELECT options"
        },
        Point::new(18, height - 18),
        if landscape { ui_detail } else { ui_body },
    )
    .draw(display)?;
    Ok(())
}

fn render_ao3_continuous_scroll_page(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    session: &crate::reader::ReaderSession,
) -> Result<(), Infallible> {
    let theme = state.reader.preferences.theme;
    let georgia_xl = reader_body_style(crate::reader::BookFont::Serif, crate::reader::BookFontSize::XLarge, theme);
    let georgia_lg = reader_body_style(crate::reader::BookFont::Serif, crate::reader::BookFontSize::Large, theme);
    let georgia_md = reader_body_style(crate::reader::BookFont::Serif, crate::reader::BookFontSize::Medium, theme);

    let lucida_body = reader_body_style(
        crate::reader::BookFont::Inter,
        state.reader.preferences.font_size,
        theme,
    );
    let lucida_sm = reader_body_style(crate::reader::BookFont::Inter, crate::reader::BookFontSize::Small, theme);
    let detail = state.display.detail_style();
    let inv_detail = state.display.text_style(UiTextRole::Detail, BinaryColor::Off);

    let total_lines = session.ao3_scroll_lines.len().max(1);
    let start = session.scroll_line.min(total_lines.saturating_sub(1));
    let mut y: i32 = 18;
    let bottom_limit: i32 = 744;

    for line in session.ao3_scroll_lines.iter().skip(start) {
        if y >= bottom_limit {
            break;
        }
        match line {
            Ao3ScrollLine::TopBanner => {
                Text::new("AO3", Point::new(16, y + 10), state.display.heading_style()).draw(display)?;
                Text::new("Archive of Our Own", Point::new(62, y + 10), georgia_lg).draw(display)?;
                Text::new("beta", Point::new(310, y + 2), detail).draw(display)?;
                y += 26;
            }
            Ao3ScrollLine::NavBar => {
                Rectangle::new(Point::new(0, y - 6), Size::new(480, 26))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
                Text::new(
                    "Fandoms    Browse    Search    About",
                    Point::new(16, y + 12),
                    inv_detail,
                )
                .draw(display)?;
                y += 28;
            }
            Ao3ScrollLine::ActionButtons => {
                for (bx, bw, lbl) in [
                    (16, 88, "Comments"),
                    (110, 160, "Hide Creator's Style"),
                    (276, 62, "Share"),
                    (344, 108, "Download v"),
                ] {
                    Rectangle::new(Point::new(bx, y - 6), Size::new(bw, 24))
                        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                        .draw(display)?;
                    Text::new(lbl, Point::new(bx + 6, y + 11), detail).draw(display)?;
                }
                y += 28;
            }
            Ao3ScrollLine::MetaBoxTop => {
                Rectangle::new(Point::new(14, y), Size::new(452, 2))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
                Rectangle::new(Point::new(12, y + 2), Size::new(2, 24))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
                Rectangle::new(Point::new(464, y + 2), Size::new(2, 24))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
                y += 14;
            }
            Ao3ScrollLine::MetaLabel(label, bold) => {
                Rectangle::new(Point::new(12, y - 10), Size::new(2, 24))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
                Rectangle::new(Point::new(464, y - 10), Size::new(2, 24))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
                Text::new(label, Point::new(24, y), detail).draw(display)?;
                if *bold {
                    Text::new(label, Point::new(25, y), detail).draw(display)?;
                }
                y += 20;
            }
            Ao3ScrollLine::MetaValue(val, bold) => {
                Rectangle::new(Point::new(12, y - 10), Size::new(2, 24))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
                Rectangle::new(Point::new(464, y - 10), Size::new(2, 24))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
                Text::new(val, Point::new(36, y), lucida_sm).draw(display)?;
                if *bold {
                    Text::new(val, Point::new(37, y), lucida_sm).draw(display)?;
                }
                let w = lucida_sm.text_width(val).max(20);
                draw_dotted_underline(display, 36, y + 3, w)?;
                y += 24;
            }
            Ao3ScrollLine::MetaStats(stats) => {
                Rectangle::new(Point::new(12, y - 10), Size::new(2, 24))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
                Rectangle::new(Point::new(464, y - 10), Size::new(2, 24))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
                Text::new(stats, Point::new(24, y), detail).draw(display)?;
                y += 26;
            }
            Ao3ScrollLine::MetaBoxBottom => {
                Rectangle::new(Point::new(14, y), Size::new(452, 2))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
                y += 20;
            }
            Ao3ScrollLine::Blank => {
                y += 14;
            }
            Ao3ScrollLine::WorkTitle(t) => {
                let w = georgia_xl.text_width(t);
                let cx = ((480 - w) / 2).max(20);
                Text::new(t, Point::new(cx, y + 10), georgia_xl).draw(display)?;
                y += 36;
            }
            Ao3ScrollLine::WorkAuthor(author) => {
                let w = georgia_md.text_width(author);
                let cx = ((480 - w) / 2).max(20);
                Text::new(author, Point::new(cx, y + 4), georgia_md).draw(display)?;
                draw_dotted_underline(display, cx, y + 8, w)?;
                y += 30;
            }
            Ao3ScrollLine::SectionHeader(hdr) => {
                Text::new(hdr, Point::new(24, y + 6), georgia_lg).draw(display)?;
                Rectangle::new(Point::new(24, y + 12), Size::new(432, 1))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
                y += 30;
            }
            Ao3ScrollLine::IndentedProse(text) => {
                Text::new(text, Point::new(36, y + 2), lucida_body).draw(display)?;
                y += i32::from(lucida_body.line_height()) + 2;
            }
            Ao3ScrollLine::ChapterHeading(ch) => {
                let w = georgia_lg.text_width(ch);
                let cx = ((480 - w) / 2).max(24);
                Text::new(ch, Point::new(cx, y + 6), georgia_lg).draw(display)?;
                y += 32;
            }
            Ao3ScrollLine::BodyProse { text, .. } => {
                let bounds = TextBounds::new(24, y - 20, 456, bottom_limit);
                Text::new(text, Point::new(24, y + 2), lucida_body)
                    .draw_clipped(display, bounds)?;
                y += i32::from(lucida_body.line_height()) + 2;
            }
        }
    }

    // Scrollbar Track & Thumb on right edge
    let track_top = 16;
    let track_h = 724;
    Rectangle::new(Point::new(472, track_top), Size::new(4, track_h as u32))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;
    let thumb_h = ((FANFIC_VIEWPORT_LINES as i32 * track_h) / total_lines as i32).clamp(18, track_h);
    let max_start = total_lines.saturating_sub(FANFIC_VIEWPORT_LINES).max(1) as i32;
    let thumb_y = track_top + ((start as i32 * (track_h - thumb_h)) / max_start).clamp(0, track_h - thumb_h);
    Rectangle::new(Point::new(472, thumb_y), Size::new(4, thumb_h as u32))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;

    Rectangle::new(Point::new(14, 748), Size::new(452, 1))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    let pct = ((start.saturating_add(FANFIC_VIEWPORT_LINES).min(total_lines) * 100) / total_lines).min(100);
    Text::new(
        &format!("UP/DOWN scroll ({pct}%)   SELECT options   BOOT back"),
        Point::new(18, 782),
        detail,
    )
    .draw(display)?;
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReaderBodyGeometry {
    text: TextBounds,
    frame: ReaderFrameBounds,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReaderFrameBounds {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl ReaderFrameBounds {
    #[must_use]
    const fn width(self) -> i32 {
        self.right - self.left
    }

    #[must_use]
    const fn height(self) -> i32 {
        self.bottom - self.top
    }
}

impl ReaderBodyGeometry {
    #[must_use]
    const fn new(width: i32, status_top: i32, status_height: i32, footer_line: i32) -> Self {
        let text = TextBounds::new(
            24,
            status_top + status_height + 18,
            width - 24,
            footer_line - 12,
        );
        let frame = ReaderFrameBounds {
            left: text.left - 8,
            top: text.top - 8,
            right: text.right + 8,
            bottom: text.bottom + 8,
        };
        Self { text, frame }
    }
}

pub fn render_options(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(display, state.display, "READER OPTIONS", "READER ACTIONS")?;
    for (index, option) in ReaderOption::ALL.iter().copied().enumerate() {
        let badge = match option {
            ReaderOption::Bookmark if state.reader.current_page_is_bookmarked() => "REMOVE",
            ReaderOption::Bookmark => "ADD",
            ReaderOption::Bookmarks => "LIST",
            ReaderOption::TableOfContents if state.reader.has_structured_toc() => "LIST",
            _ => option.badge(),
        };
        draw_row(
            display,
            state,
            146 + index as i32 * 66,
            state.reader.options_selected == index,
            option.label(),
            badge,
            "",
        )?;
    }
    draw_footer(
        display,
        state.display,
        "MOVE  SELECT ACTIVATE  HOLD BOOT BACK",
    )
}

pub fn render_preferences(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(
        display,
        state.display,
        "READING PREFERENCES",
        "SETTINGS-STYLE ROW EDITOR",
    )?;
    for (index, preference) in ReadingPreference::ALL.iter().copied().enumerate() {
        let badge = match preference {
            ReadingPreference::ReadingTheme => state.reader.preferences.theme.label(),
            ReadingPreference::Orientation => state.reader.preferences.orientation.label(),
            ReadingPreference::BookFontSize => state.reader.preferences.font_size.label(),
            ReadingPreference::BookFont => state.reader.preferences.book_font.label(),
            ReadingPreference::ParagraphAlignment => {
                state.reader.preferences.paragraph_alignment.label()
            }
            ReadingPreference::ShowProgress if state.reader.preferences.show_progress => "On",
            ReadingPreference::ShowProgress => "Off",
        };
        draw_row(
            display,
            state,
            156 + index as i32 * 78,
            state.reader.preferences_selected == index,
            preference.label(),
            badge,
            "",
        )?;
    }
    draw_footer(
        display,
        state.display,
        "UP/DOWN MOVE  SELECT CHANGE  HOLD BOOT BACK",
    )
}

pub fn render_toc(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(
        display,
        state.display,
        "TABLE OF CONTENTS",
        if state.reader.has_structured_toc() {
            "EPUB NAVIGATION"
        } else {
            "TXT FOUNDATION"
        },
    )?;
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let toc = state.reader.toc_entries();
    if toc.is_empty() {
        Text::new("No structured TOC", Point::new(24, 200), heading).draw(display)?;
        Text::new(
            "Ordinary TXT files do not provide a formal",
            Point::new(24, 258),
            body,
        )
        .draw(display)?;
        Text::new(
            "table of contents. EPUB books expose their",
            Point::new(24, 300),
            body,
        )
        .draw(display)?;
        Text::new(
            "navigation entries on this screen.",
            Point::new(24, 342),
            body,
        )
        .draw(display)?;
        return draw_footer(display, state.display, "HOLD BOOT BACK");
    }

    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "EPUB TOC",
            middle: &format!("{} entries", toc.len()),
            right: "SELECT OPEN",
        },
    )?;
    let first = state.reader.toc_selected.saturating_sub(7);
    for (row, entry) in toc.iter().skip(first).take(8).enumerate() {
        let index = first + row;
        draw_row(
            display,
            state,
            166 + row as i32 * 64,
            state.reader.toc_selected == index,
            &truncate(&entry.label, 27),
            "CH",
            &(entry.spine_index + 1).to_string(),
        )?;
    }
    draw_footer(display, state.display, "MOVE  SELECT OPEN  HOLD BOOT BACK")
}

fn aligned_reader_line(
    line: &str,
    paragraph_end: bool,
    alignment: ParagraphAlignment,
    style: crate::app::typography::UiTextStyle,
    bounds: TextBounds,
) -> (String, i32) {
    let width = style.text_width(line);
    let available = bounds.width().max(0);
    match alignment {
        ParagraphAlignment::Left => (line.into(), bounds.left),
        ParagraphAlignment::Center => (line.into(), bounds.left + (available - width).max(0) / 2),
        ParagraphAlignment::Right => (line.into(), bounds.left + (available - width).max(0)),
        ParagraphAlignment::Justified if !paragraph_end => {
            (justify_reader_line(line, style, available), bounds.left)
        }
        ParagraphAlignment::Justified => (line.into(), bounds.left),
    }
}

fn justify_reader_line(
    line: &str,
    style: crate::app::typography::UiTextStyle,
    available: i32,
) -> String {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() < 2 {
        return line.into();
    }
    let base = words.join(" ");
    let space = style.text_width(" ").max(1);
    let extra_spaces = ((available - style.text_width(base.as_str())).max(0) / space) as usize;
    let gaps = words.len() - 1;
    let mut output = String::new();
    for (index, word) in words.iter().enumerate() {
        output.push_str(word);
        if index < gaps {
            let remainder = if index < extra_spaces % gaps { 1 } else { 0 };
            let count = 1 + extra_spaces / gaps + remainder;
            output.extend(core::iter::repeat(' ').take(count));
        }
    }
    output
}

fn draw_tabs(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    active: ReaderLibraryTab,
) -> Result<(), Infallible> {
    let detail = state.display.detail_style();
    let tabs = [
        ReaderLibraryTab::Recent,
        ReaderLibraryTab::Books,
        ReaderLibraryTab::Fanfic,
        ReaderLibraryTab::Files,
        ReaderLibraryTab::Bookmarks,
    ];
    for (index, tab) in tabs.iter().copied().enumerate() {
        let left = 10 + index as i32 * 93;
        if tab == active {
            Rectangle::new(Point::new(left, 132), Size::new(89, 40))
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display)?;
            Text::new(
                tab.label(),
                Point::new(left + 8, 158),
                state.display.text_style(UiTextRole::Detail, BinaryColor::Off),
            )
            .draw(display)?;
        } else {
            Rectangle::new(Point::new(left, 132), Size::new(89, 40))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)?;
            Text::new(tab.label(), Point::new(left + 8, 158), detail).draw(display)?;
        }
    }
    Ok(())
}

fn draw_row(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    top: i32,
    selected: bool,
    label: &str,
    badge: &str,
    suffix: &str,
) -> Result<(), Infallible> {
    let body = if selected {
        state.display.text_style(UiTextRole::Body, BinaryColor::Off)
    } else {
        state.display.body_style()
    };
    let style = if selected {
        PrimitiveStyle::with_fill(BinaryColor::On)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    Rectangle::new(Point::new(20, top), Size::new(440, 50))
        .into_styled(style)
        .draw(display)?;
    Text::new(
        if selected { ">" } else { " " },
        Point::new(32, top + 32),
        body,
    )
    .draw(display)?;
    Text::new(label, Point::new(58, top + 32), body).draw(display)?;
    Text::new(badge, Point::new(338, top + 32), body).draw(display)?;
    Text::new(suffix, Point::new(402, top + 32), body).draw(display)?;
    Ok(())
}

fn draw_progress(display: &mut OrientedFrameBuffer<'_>, percent: u8) -> Result<(), Infallible> {
    Rectangle::new(Point::new(24, 282), Size::new(432, 38))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
        .draw(display)?;
    let width = 4 * percent as u32;
    Rectangle::new(Point::new(30, 288), Size::new(width.min(420), 26))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    Ok(())
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.into();
    }
    let mut output: String = value.chars().take(max_chars.saturating_sub(3)).collect();
    output.push_str("...");
    output
}

#[cfg(test)]
mod tests {
    use super::{
        aligned_reader_line, bookmark_entry_columns, library_entry_columns, library_status,
        render_bookmarks, render_continue_reading, render_library, render_loading, render_options,
        render_preferences, render_toc, ReaderBodyGeometry,
    };
    use crate::{
        app::AppState,
        framebuffer::FrameBuffer,
        orientation::OrientedFrameBuffer,
        reader::{
            BookFormat, ParagraphAlignment, PendingReaderOpen, ReaderBook, ReaderChapterPageLabel,
            ReaderLibraryEntry, ReaderLibraryTab, ReaderLoadingStage, ReaderLocation,
        },
    };

    #[test]
    fn high_contrast_frame_stays_outside_shared_text_viewport() {
        let body = ReaderBodyGeometry::new(480, 80, 42, 746);
        assert!(body.frame.left < body.text.left);
        assert!(body.frame.top < body.text.top);
        assert!(body.frame.right > body.text.right);
        assert!(body.frame.bottom > body.text.bottom);
        assert_eq!(body.text.left, 24);
        assert_eq!(body.text.right, 456);
    }

    #[test]
    fn library_bookmarks_tab_uses_saved_status_and_page_columns() {
        let bookmark = ReaderLocation {
            path: "POIROT~1.TXT".into(),
            title: "POIROT~1".into(),
            format: BookFormat::Text,
            size_bytes: 123,
            modified_seconds: 456,
            byte_offset: 789,
            page_index: 11,
            epub_chapter: None,
        };
        let mut reader = crate::reader::ReaderUiState::default();
        reader.library_tab = ReaderLibraryTab::Bookmarks;
        let entry = ReaderLibraryEntry {
            book: bookmark.as_book(),
            location: Some(bookmark),
        };
        assert_eq!(
            library_status(ReaderLibraryTab::Bookmarks, 9),
            super::LibraryStatus {
                left: "Bookmarks",
                middle: "9 saved".into(),
                right: "MARKS.TXT",
            }
        );
        assert_eq!(
            library_entry_columns(&reader, &entry),
            super::LibraryEntryColumns {
                badge: "PAGE".into(),
                suffix: "12".into(),
            }
        );
    }
    #[test]
    fn epub_bookmark_columns_show_chapter_and_chapter_page_total() {
        let bookmark = ReaderLocation {
            path: "NOVEL.EPU".into(),
            title: "Novel".into(),
            format: BookFormat::Epub,
            size_bytes: 123,
            modified_seconds: 456,
            byte_offset: 789,
            page_index: 11,
            epub_chapter: Some(ReaderChapterPageLabel {
                chapter_number: 4,
                page_number: 3,
                page_count: 12,
            }),
        };
        let reader = crate::reader::ReaderUiState::default();
        assert_eq!(
            bookmark_entry_columns(&reader, &bookmark),
            super::LibraryEntryColumns {
                badge: "CH 4".into(),
                suffix: "P 3/12".into(),
            }
        );
    }

    #[test]
    fn library_books_and_files_tabs_keep_format_and_open_columns() {
        let entry = ReaderLibraryEntry {
            book: ReaderBook {
                path: "POIROT~1.TXT".into(),
                title: "POIROT~1".into(),
                format: BookFormat::Text,
                size_bytes: 123,
                modified_seconds: 456,
            },
            location: None,
        };
        for tab in [ReaderLibraryTab::Books, ReaderLibraryTab::Files] {
            let mut reader = crate::reader::ReaderUiState::default();
            reader.library_tab = tab;
            assert_eq!(
                library_entry_columns(&reader, &entry),
                super::LibraryEntryColumns {
                    badge: "TXT".into(),
                    suffix: "OPEN".into(),
                }
            );
        }
    }

    #[test]
    fn reader_screens_render_without_sd_card() {
        let mut state = AppState::default();
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        render_continue_reading(&mut display, &state).unwrap();
        render_library(&mut display, &state).unwrap();
        render_bookmarks(&mut display, &state).unwrap();
        render_options(&mut display, &state).unwrap();
        render_preferences(&mut display, &state).unwrap();
        render_toc(&mut display, &state).unwrap();
        state.reader.loading = Some(PendingReaderOpen {
            book: ReaderBook {
                path: "a.txt".into(),
                title: "A".into(),
                format: BookFormat::Text,
                size_bytes: 1,
                modified_seconds: 0,
            },
            stage: ReaderLoadingStage::OpeningFile,
            encoding: None,
            epub_document: None,
            resume: None,
            message: "Preparing".into(),
        });
        render_loading(&mut display, &state).unwrap();
    }
    #[test]
    fn paragraph_alignment_moves_or_justifies_reader_lines_inside_bounds() {
        let style = AppState::default().display.body_style();
        let bounds = crate::app::typography::TextBounds::new(20, 0, 220, 100);
        let (_, left) =
            aligned_reader_line("short line", true, ParagraphAlignment::Left, style, bounds);
        let (_, center) = aligned_reader_line(
            "short line",
            true,
            ParagraphAlignment::Center,
            style,
            bounds,
        );
        let (_, right) =
            aligned_reader_line("short line", true, ParagraphAlignment::Right, style, bounds);
        assert!(left < center);
        assert!(center < right);
        let (justified, _) = aligned_reader_line(
            "one two three",
            false,
            ParagraphAlignment::Justified,
            style,
            bounds,
        );
        assert!(justified.len() > "one two three".len());
    }
}
