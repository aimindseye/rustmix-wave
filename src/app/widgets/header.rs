//! Reusable black product header.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{display::DisplayPreferences, typography::Text},
    orientation::OrientedFrameBuffer,
};

/// Draw the product header shared by every portrait screen.
pub fn draw_header(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    title: &str,
    subtitle: &str,
) -> Result<(), Infallible> {
    let black = PrimitiveStyle::with_fill(BinaryColor::On);
    let title_style = preferences.header_title_style();
    let subtitle_style = preferences.header_subtitle_style();

    Rectangle::new(Point::new(0, 0), Size::new(480, 70))
        .into_styled(black)
        .draw(display)?;
    Text::new(title, Point::new(18, 32), title_style).draw(display)?;
    Text::new(subtitle, Point::new(18, 60), subtitle_style).draw(display)?;
    Ok(())
}
