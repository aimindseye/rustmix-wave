//! Persistent global user-interface typography settings.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::app::typography::{Text, UiTextStyle};

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
};

pub fn render_display(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let prefs = state.display;

    draw_header(display, state.display, "DISPLAY", "FONT AND SIZE")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: prefs.font_family.compact_label(),
            middle: prefs.font_size.label(),
            right: prefs.persistence_label(),
        },
    )?;
    Text::new("Display preferences", Point::new(22, 160), heading).draw(display)?;

    draw_setting_row(
        display,
        202,
        "UI font",
        prefs.font_family.compact_label(),
        state.display_action_selected == 0,
        body,
    )?;
    draw_setting_row(
        display,
        292,
        "UI size",
        prefs.font_size.label(),
        state.display_action_selected == 1,
        body,
    )?;

    Text::new("Live preview", Point::new(22, 410), heading).draw(display)?;
    Rectangle::new(Point::new(22, 438), Size::new(436, 160))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;
    Text::new("Reader", Point::new(44, 500), prefs.navigation_style()).draw(display)?;
    Text::new("Books, progress and bookmarks", Point::new(44, 548), body).draw(display)?;
    Text::new(
        "Hold BOOT to return to Settings.",
        Point::new(22, 666),
        body,
    )
    .draw(display)?;

    draw_footer(
        display,
        state.display,
        "MOVE  SELECT CHANGE  HOLD BOOT BACK",
    )?;
    Ok(())
}

fn draw_setting_row(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    label: &str,
    value: &str,
    selected: bool,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    let border = if selected {
        PrimitiveStyle::with_stroke(BinaryColor::On, 4)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    Rectangle::new(Point::new(22, top), Size::new(436, 70))
        .into_styled(border)
        .draw(display)?;
    Text::new(
        if selected { ">" } else { " " },
        Point::new(38, top + 43),
        style,
    )
    .draw(display)?;
    Text::new(label, Point::new(68, top + 43), style).draw(display)?;
    Text::new(value, Point::new(258, top + 43), style).draw(display)?;
    Ok(())
}
