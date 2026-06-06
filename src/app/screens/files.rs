//! SDMMC read-only file-browser screen.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::app::typography::Text;

use crate::{
    app::{
        state::AppState,
        widgets::{
            footer::draw_footer,
            header::draw_header,
            status_row::{draw_status_row, StatusRow},
        },
    },
    orientation::OrientedFrameBuffer,
    storage::{FilePreview, StorageSnapshot},
};

/// Draw the read-only SDMMC browser or the bounded text-preview panel.
pub fn render_files(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let storage = &state.storage;
    if let Some(preview) = &storage.preview {
        return render_preview(display, state, storage, preview);
    }

    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let detail = state.display.detail_style();
    let path = truncate_label(&storage.current_path, 18);
    let page = storage.page_label();

    draw_header(display, state.display, "FILES", "SDMMC READ-ONLY BROWSER")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: storage.status_label(),
            middle: &path,
            right: &page,
        },
    )?;

    Text::new("Directory", Point::new(22, 152), heading).draw(display)?;
    if let Some(error) = &storage.error {
        Text::new(&truncate_label(error, 68), Point::new(22, 180), body).draw(display)?;
    } else if storage.scan.retained_entries == 0 {
        Text::new(
            "No files or directories found on this SD card.",
            Point::new(22, 180),
            body,
        )
        .draw(display)?;
    } else {
        Text::new(
            "Directories first, then files. No write operations.",
            Point::new(22, 180),
            body,
        )
        .draw(display)?;
    }

    let selected_on_page = storage.selected_on_page();
    for (index, entry) in storage.visible_entries().iter().enumerate() {
        let top = 210 + (index as i32 * 66);
        let selected = index == selected_on_page;
        let outline = if selected {
            PrimitiveStyle::with_stroke(BinaryColor::On, 3)
        } else {
            PrimitiveStyle::with_stroke(BinaryColor::On, 1)
        };
        Rectangle::new(Point::new(22, top), Size::new(436, 54))
            .into_styled(outline)
            .draw(display)?;
        Text::new(
            if selected { ">" } else { " " },
            Point::new(36, top + 32),
            heading,
        )
        .draw(display)?;
        Text::new(
            &truncate_label(&entry.name, 29),
            Point::new(62, top + 23),
            heading,
        )
        .draw(display)?;
        Text::new(entry.kind.badge(), Point::new(62, top + 43), detail).draw(display)?;
        Text::new(&entry.size_label(), Point::new(382, top + 32), detail).draw(display)?;
    }

    draw_footer(display, state.display, "MOVE  SELECT OPEN  HOLD BOOT BACK")?;
    Ok(())
}

fn render_preview(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    storage: &StorageSnapshot,
    preview: &FilePreview,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let detail = state.display.detail_style();
    let path = truncate_label(&storage.current_path, 18);
    let mode = if preview.binary { "BINARY" } else { "TEXT" };

    draw_header(
        display,
        state.display,
        "FILE PREVIEW",
        "BOUNDED READ-ONLY VIEW",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: mode,
            middle: &path,
            right: if preview.truncated { "TRUNC" } else { "FULL" },
        },
    )?;
    Text::new(
        &truncate_label(&preview.name, 52),
        Point::new(22, 152),
        heading,
    )
    .draw(display)?;
    Text::new(
        if preview.binary {
            "Binary content is intentionally not rendered."
        } else if preview.truncated {
            "Preview capped at 384 bytes. File remains unchanged."
        } else {
            "Text preview. File remains unchanged."
        },
        Point::new(22, 182),
        body,
    )
    .draw(display)?;

    Rectangle::new(Point::new(22, 210), Size::new(436, 498))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;
    for (index, line) in preview.display_lines(18, 52).iter().enumerate() {
        Text::new(line, Point::new(34, 238 + (index as i32 * 24)), detail).draw(display)?;
    }

    draw_footer(display, state.display, "SELECT CLOSE  HOLD BOOT BACK")?;
    Ok(())
}

fn truncate_label(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.into();
    }
    let mut output: String = value.chars().take(max_chars.saturating_sub(3)).collect();
    output.push_str("...");
    output
}
