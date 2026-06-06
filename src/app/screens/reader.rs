//! Reader landing, library, bookmarks, loading, TXT / EPUB page, TOC and options screens.

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
    orientation::OrientedFrameBuffer,
    reader::{
        BookFormat, ParagraphAlignment, ReaderLibraryTab, ReaderLoadingStage, ReaderOption,
        ReadingPreference, ReadingTheme,
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
            "Open Library and choose a TXT book.",
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
    draw_header(display, state.display, "LIBRARY", "TXT / REFLOWABLE EPUB")?;
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
        188,
        control_selected,
        "Change tab",
        "SELECT",
        "TABS",
    )?;
    let visible = reader.visible_entries();
    if visible.is_empty() {
        let message = reader
            .library_error
            .as_deref()
            .unwrap_or(match reader.library_tab {
                ReaderLibraryTab::Recent => "No recent books yet.",
                ReaderLibraryTab::Bookmarks => "No saved bookmarks yet.",
                _ => "Copy TXT or EPUB books into /RUSTMIX/BOOKS.",
            });
        Text::new(&truncate(message, 54), Point::new(26, 302), body).draw(display)?;
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
        "OPENING BOOK",
        "RESPONSIVE FIRST-PAGE-FIRST CACHE",
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
    Text::new(
        "The current page opens before full indexing.",
        Point::new(24, 410),
        body,
    )
    .draw(display)?;
    draw_footer(display, state.display, "HOLD BOOT CANCEL")
}

pub fn render_page(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let Some(session) = state.reader.session.as_ref() else {
        return render_continue_reading(display, state);
    };
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
    /// Shared Reader body rectangle used by Classic and High Contrast. The
    /// stronger High Contrast frame stays outside this viewport, so switching
    /// themes never changes TXT pagination or cache fingerprints.
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
    let body = state.display.body_style();
    let tabs = [
        ReaderLibraryTab::Recent,
        ReaderLibraryTab::Books,
        ReaderLibraryTab::Files,
        ReaderLibraryTab::Bookmarks,
    ];
    for (index, tab) in tabs.iter().copied().enumerate() {
        let left = 10 + index as i32 * 117;
        if tab == active {
            Rectangle::new(Point::new(left, 132), Size::new(112, 42))
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display)?;
            Text::new(
                tab.label(),
                Point::new(left + 8, 161),
                state.display.text_style(UiTextRole::Body, BinaryColor::Off),
            )
            .draw(display)?;
        } else {
            Text::new(tab.label(), Point::new(left + 8, 161), body).draw(display)?;
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
