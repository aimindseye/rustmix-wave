//! Clean placeholder for future modular applications.

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
};

pub fn render_placeholder(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let route = state.active_route();
    let parent = route.parent().map_or("Home", |value| value.label());
    let title = route.label().to_ascii_uppercase();
    let heading = state.display.heading_style();
    let body = state.display.body_style();

    draw_header(display, state.display, &title, "COMING SOON")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "PLACEHOLDER",
            middle: parent,
            right: "SOON",
        },
    )?;
    Text::new(
        route.label(),
        Point::new(22, 170),
        state.display.navigation_style(),
    )
    .draw(display)?;
    Rectangle::new(Point::new(22, 222), Size::new(436, 238))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;
    Text::new("Reserved for a later", Point::new(48, 300), heading).draw(display)?;
    Text::new("isolated feature milestone.", Point::new(48, 340), heading).draw(display)?;
    Text::new("Navigation is ready now.", Point::new(48, 402), body).draw(display)?;
    Text::new(&format!("Parent: {parent}"), Point::new(22, 530), body).draw(display)?;
    draw_footer(display, state.display, "HOLD BOOT BACK")?;
    Ok(())
}
