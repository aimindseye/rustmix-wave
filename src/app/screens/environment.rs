//! SHTC3-backed environment overview and readable sensor details.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        state::AppState,
        typography::{Text, UiTextStyle},
        widgets::{
            footer::draw_footer,
            header::draw_header,
            status_row::{draw_status_row, StatusRow},
        },
    },
    orientation::OrientedFrameBuffer,
};

/// Draw temperature and humidity from the onboard SHTC3.
pub fn render_environment(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let large = state.display.large_style();
    let outline = PrimitiveStyle::with_stroke(BinaryColor::On, 2);
    let temperature = state
        .board
        .temperature_label(state.regional.temperature_unit);
    let humidity = state.board.humidity_label();
    let battery = state.board.battery_label();

    draw_header(
        display,
        state.display,
        "ENVIRONMENT",
        "TEMPERATURE AND HUMIDITY",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: &temperature,
            middle: &humidity,
            right: &battery,
        },
    )?;

    Rectangle::new(Point::new(22, 156), Size::new(436, 160))
        .into_styled(outline)
        .draw(display)?;
    Text::new("Temperature", Point::new(42, 198), heading).draw(display)?;
    Text::new(&temperature, Point::new(42, 270), large).draw(display)?;

    Rectangle::new(Point::new(22, 352), Size::new(436, 160))
        .into_styled(outline)
        .draw(display)?;
    Text::new("Relative humidity", Point::new(42, 394), heading).draw(display)?;
    Text::new(&humidity, Point::new(42, 466), large).draw(display)?;

    draw_action(display, 640, "Sensor details", body)?;
    draw_footer(display, state.display, "SELECT DETAILS  HOLD BOOT BACK")?;
    Ok(())
}

pub fn render_environment_details(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let id = state
        .board
        .environment_sensor_id
        .map_or_else(|| "Unavailable".into(), |id| format!("0x{id:04X}"));
    let status = if state.board.environment.is_some() {
        "Ready"
    } else {
        "Unavailable"
    };

    draw_header(
        display,
        state.display,
        "SENSOR DETAILS",
        "SHTC3 ENVIRONMENT SENSOR",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "SHTC3",
            middle: status,
            right: "DETAILS",
        },
    )?;

    Text::new("Sensor", Point::new(22, 166), heading).draw(display)?;
    line(display, 214, "Status", status, body)?;
    line(display, 254, "Device ID", &id, body)?;
    line(display, 294, "Command", "Wake / measure / sleep", body)?;
    line(display, 334, "Validation", "Sensirion CRC-8", body)?;

    Text::new("Calibration", Point::new(22, 410), heading).draw(display)?;
    line(display, 458, "Compensation", "-1.5 C / -2.7 F", body)?;
    line(display, 498, "Live refresh", "Every 30 seconds", body)?;

    Text::new(
        "Hold BOOT to return to Environment.",
        Point::new(22, 666),
        body,
    )
    .draw(display)?;
    draw_footer(display, state.display, "HOLD BOOT BACK")?;
    Ok(())
}

fn line(
    display: &mut OrientedFrameBuffer<'_>,
    y: i32,
    label: &str,
    value: &str,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    Text::new(label, Point::new(22, y), style).draw(display)?;
    Text::new(value, Point::new(188, y), style).draw(display)?;
    Ok(())
}

fn draw_action(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    label: &str,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    Rectangle::new(Point::new(22, top), Size::new(436, 52))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 4))
        .draw(display)?;
    Text::new(">", Point::new(38, top + 34), style).draw(display)?;
    Text::new(label, Point::new(68, top + 34), style).draw(display)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{render_environment, render_environment_details};
    use crate::{app::AppState, framebuffer::FrameBuffer, orientation::OrientedFrameBuffer};

    #[test]
    fn environment_overview_and_details_render_without_sensor() {
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        let state = AppState::default();
        render_environment(&mut display, &state).unwrap();
        render_environment_details(&mut display, &state).unwrap();
    }
}
