//! Reusable high-contrast Home category card primitive.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyleBuilder, Rectangle},
};

use crate::{
    app::{
        display::DisplayPreferences,
        typography::{Text, UiTextRole},
    },
    orientation::OrientedFrameBuffer,
};

/// Text and selection state for one product category card.
pub struct CardSpec<'a> {
    pub top: i32,
    pub title: &'a str,
    pub subtitle: &'a str,
    pub badge: &'a str,
    pub selected: bool,
}

/// Draw one Home-screen card. The selected card is inverted for clear
/// handheld-distance contrast on the monochrome e-paper panel.
pub fn draw_card(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    spec: CardSpec<'_>,
) -> Result<(), Infallible> {
    let ink = if spec.selected {
        BinaryColor::Off
    } else {
        BinaryColor::On
    };
    let border = PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::On)
        .stroke_width(if spec.selected { 2 } else { 1 })
        .fill_color(if spec.selected {
            BinaryColor::On
        } else {
            BinaryColor::Off
        })
        .build();
    let heading = preferences.text_style(UiTextRole::Heading, ink);
    let body = preferences.text_style(UiTextRole::Body, ink);

    Rectangle::new(Point::new(18, spec.top), Size::new(444, 82))
        .into_styled(border)
        .draw(display)?;
    Text::new(
        if spec.selected { ">" } else { " " },
        Point::new(30, spec.top + 39),
        body,
    )
    .draw(display)?;
    Text::new(spec.title, Point::new(54, spec.top + 34), heading).draw(display)?;
    Text::new(spec.subtitle, Point::new(54, spec.top + 68), body).draw(display)?;
    Text::new(spec.badge, Point::new(402, spec.top + 68), body).draw(display)?;
    Ok(())
}
