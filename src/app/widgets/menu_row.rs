//! Reusable category and settings menu row.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{display::DisplayPreferences, menu::MenuEntry, typography::Text},
    orientation::OrientedFrameBuffer,
};

pub fn draw_menu_row(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    entry: MenuEntry,
    selected: bool,
    preferences: DisplayPreferences,
) -> Result<(), Infallible> {
    let border = if selected {
        PrimitiveStyle::with_stroke(BinaryColor::On, 4)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    let heading = preferences.navigation_style();
    let body = preferences.body_style();

    Rectangle::new(Point::new(22, top), Size::new(436, 76))
        .into_styled(border)
        .draw(display)?;
    Text::new(
        if selected { ">" } else { " " },
        Point::new(34, top + 35),
        body,
    )
    .draw(display)?;
    Text::new(entry.label, Point::new(56, top + 31), heading).draw(display)?;
    Text::new(entry.subtitle, Point::new(56, top + 62), body).draw(display)?;
    Text::new(entry.badge, Point::new(398, top + 62), body).draw(display)?;
    Ok(())
}
