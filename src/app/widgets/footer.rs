//! Reusable footer hints for button-driven screens.

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

/// Draw a footer separator and one-line button hint.
pub fn draw_footer(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    hint: &str,
) -> Result<(), Infallible> {
    let line = PrimitiveStyle::with_fill(BinaryColor::On);
    let body = preferences.footer_style();

    Rectangle::new(Point::new(14, 746), Size::new(452, 1))
        .into_styled(line)
        .draw(display)?;
    Text::new(hint, Point::new(18, 782), body).draw(display)?;
    Ok(())
}
