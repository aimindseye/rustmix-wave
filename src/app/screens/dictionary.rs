//! Native rotary-first Dictionary screen backed by the Rustmix X4 prefix pack.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        state::AppState,
        typography::Text,
        widgets::{
            footer::draw_footer,
            header::draw_header,
            status_row::{draw_status_row, StatusRow},
        },
    },
    dictionary::{DictionaryUiState, DICTIONARY_KEY_ROWS, DICTIONARY_SHARD_MAX_BYTES},
    orientation::OrientedFrameBuffer,
};

pub fn render_dictionary(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let dictionary = &state.dictionary;
    let body = state.display.body_style();
    let heading = state.display.heading_style();
    let detail = state.display.detail_style();
    let query = if dictionary.query.is_empty() {
        "_".into()
    } else if dictionary.wildcard {
        format!("{}*", dictionary.query)
    } else {
        dictionary.query.clone()
    };
    let shard_label = dictionary
        .current_match()
        .map_or_else(|| "INDEX.TXT".into(), |item| item.shard.clone());
    let mode = if dictionary.wildcard {
        "PREFIX"
    } else {
        "EXACT"
    };

    draw_header(display, state.display, "DICTIONARY", "X4 PREFIX-SHARD PACK")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: if dictionary.pack_ready {
                "READY"
            } else {
                "SD PACK"
            },
            middle: &shard_label,
            right: dictionary.navigation_mode_label(),
        },
    )?;

    Text::new("Search", Point::new(22, 158), heading).draw(display)?;
    Rectangle::new(Point::new(22, 176), Size::new(436, 52))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
        .draw(display)?;
    Text::new(&query, Point::new(38, 210), body).draw(display)?;
    Text::new(
        &truncate(&dictionary.message, 48),
        Point::new(22, 258),
        body,
    )
    .draw(display)?;
    Text::new(&format!("LOOKUP {mode}"), Point::new(350, 258), detail).draw(display)?;

    Rectangle::new(Point::new(22, 278), Size::new(436, 190))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;
    if let Some(result) = dictionary.current_match() {
        Text::new(&truncate(&result.word, 34), Point::new(38, 316), heading).draw(display)?;
        Text::new(&dictionary.match_label(), Point::new(342, 316), body).draw(display)?;
        for (index, line) in wrap_lines(&result.definition, 51, 5).iter().enumerate() {
            Text::new(line, Point::new(38, 352 + index as i32 * 24), detail).draw(display)?;
        }
    } else {
        Text::new("Offline native lookup", Point::new(38, 322), heading).draw(display)?;
        Text::new("Reuses /RUSTMIX/APPS/DICT", Point::new(38, 362), body).draw(display)?;
        Text::new(
            &format!(
                "Bounded shard read: {} KiB",
                DICTIONARY_SHARD_MAX_BYTES / 1024
            ),
            Point::new(38, 402),
            body,
        )
        .draw(display)?;
    }

    draw_keyboard(display, dictionary, body, heading)?;
    draw_footer(
        display,
        state.display,
        "UP/DOWN MOVE  BOOT H/V  SELECT  HOLD BOOT BACK",
    )?;
    Ok(())
}

fn draw_keyboard(
    display: &mut OrientedFrameBuffer<'_>,
    dictionary: &DictionaryUiState,
    body: crate::app::typography::UiTextStyle,
    heading: crate::app::typography::UiTextStyle,
) -> Result<(), Infallible> {
    for (row_index, row) in DICTIONARY_KEY_ROWS.iter().enumerate() {
        for (column_index, label) in row.iter().enumerate() {
            let index = row_index * 6 + column_index;
            let left = 22 + column_index as i32 * 73;
            let top = 486 + row_index as i32 * 48;
            let selected = dictionary.selected_key_index() == index;
            Rectangle::new(Point::new(left, top), Size::new(68, 40))
                .into_styled(PrimitiveStyle::with_stroke(
                    BinaryColor::On,
                    if selected { 3 } else { 1 },
                ))
                .draw(display)?;
            Text::new(
                label,
                Point::new(left + if label.len() > 1 { 8 } else { 24 }, top + 27),
                if selected { heading } else { body },
            )
            .draw(display)?;
        }
    }
    Ok(())
}

fn wrap_lines(value: &str, max_chars: usize, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in value.split_whitespace() {
        if !current.is_empty() && current.len() + 1 + word.len() > max_chars {
            lines.push(current);
            current = String::new();
            if lines.len() >= max_lines {
                break;
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if lines.len() < max_lines && !current.is_empty() {
        lines.push(current);
    }
    lines
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
    use super::render_dictionary;
    use crate::{app::AppState, framebuffer::FrameBuffer, orientation::OrientedFrameBuffer};

    #[test]
    fn dictionary_screen_renders_without_sd_pack() {
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        render_dictionary(&mut display, &AppState::default()).unwrap();
    }
}
