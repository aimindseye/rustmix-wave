//! Home-only dashboard chrome for a compact product-facing landing page.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
};

use crate::{
    app::{
        display::DisplayPreferences,
        typography::{Text, UiTextRole},
    },
    orientation::OrientedFrameBuffer,
};

/// Product-facing values shown above the five main category cards.
pub struct HomeDashboardStrip<'a> {
    pub date: &'a str,
    pub time: &'a str,
    pub weather: &'a str,
    pub battery: &'a str,
    pub wifi: &'a str,
}

/// Draw the simplified Home header without milestone or developer copy.
pub fn draw_home_header(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
) -> Result<(), Infallible> {
    let black = PrimitiveStyle::with_fill(BinaryColor::On);
    let title = preferences.text_style(UiTextRole::Large, BinaryColor::Off);

    Rectangle::new(Point::new(0, 0), Size::new(480, 66))
        .into_styled(black)
        .draw(display)?;
    Text::new("RUSTMIX WAVE", Point::new(18, 44), title).draw(display)?;
    Ok(())
}

/// Draw date/time and the user-relevant weather, battery and Wi-Fi summary.
pub fn draw_home_dashboard_strip(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    strip: HomeDashboardStrip<'_>,
) -> Result<(), Infallible> {
    let outline = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
    let divider = PrimitiveStyle::with_fill(BinaryColor::On);
    let body = preferences.body_style();
    let detail = preferences.detail_style();

    Rectangle::new(Point::new(14, 80), Size::new(452, 110))
        .into_styled(outline)
        .draw(display)?;
    Rectangle::new(Point::new(14, 125), Size::new(452, 1))
        .into_styled(divider)
        .draw(display)?;
    Rectangle::new(Point::new(238, 126), Size::new(1, 64))
        .into_styled(divider)
        .draw(display)?;

    Text::new(strip.date, Point::new(24, 110), body).draw(display)?;
    Text::new(strip.time, Point::new(356, 110), body).draw(display)?;

    Text::new("WEATHER", Point::new(24, 151), detail).draw(display)?;
    Text::new(strip.weather, Point::new(24, 179), body).draw(display)?;

    Text::new(strip.battery, Point::new(254, 151), body).draw(display)?;
    Text::new(strip.wifi, Point::new(254, 179), body).draw(display)?;
    Ok(())
}

/// Draw the fixed dark Home action bar. Home intentionally omits Back because
/// it is the hierarchy root.
pub fn draw_home_footer(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
) -> Result<(), Infallible> {
    let black = PrimitiveStyleBuilder::new()
        .fill_color(BinaryColor::On)
        .build();
    let body = preferences.text_style(UiTextRole::Body, BinaryColor::Off);

    Rectangle::new(Point::new(0, 744), Size::new(480, 56))
        .into_styled(black)
        .draw(display)?;
    Text::new("UP/DOWN MOVE    SELECT OPEN", Point::new(18, 781), body).draw(display)?;
    Ok(())
}
