//! RTC-backed Clock overview and readable details pages.

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

/// Draw the user-facing RTC overview.
pub fn render_clock(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let large = state.display.large_style();
    let outline = PrimitiveStyle::with_stroke(BinaryColor::On, 2);
    let time = state.board.time_label(state.regional);
    let date_time = state.board.date_time_label(state.regional);
    let battery = state.board.battery_label();
    let temperature = state
        .board
        .temperature_label(state.regional.temperature_unit);
    let humidity = state.board.humidity_label();

    draw_header(display, state.display, "CLOCK", "RTC AND BOARD STATUS")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: &time,
            middle: &battery,
            right: "LIVE",
        },
    )?;

    Rectangle::new(Point::new(22, 146), Size::new(436, 150))
        .into_styled(outline)
        .draw(display)?;
    Text::new("Current RTC time", Point::new(42, 180), heading).draw(display)?;
    Text::new(&time, Point::new(42, 238), large).draw(display)?;
    Text::new(&date_time, Point::new(42, 274), body).draw(display)?;

    Text::new("Onboard status", Point::new(22, 356), heading).draw(display)?;
    line(display, 404, "Temperature", &temperature, body)?;
    line(display, 442, "Humidity", &humidity, body)?;
    line(display, 480, "Battery", &battery, body)?;

    if let Some(power) = state.board.power {
        let usb = if power.vbus_present {
            "Connected"
        } else {
            "Not detected"
        };
        let charge = if power.charging {
            "Charging"
        } else {
            "Not charging"
        };
        line(display, 518, "USB", usb, body)?;
        line(display, 556, "Charge state", charge, body)?;
    }

    draw_action(display, 642, "RTC details", body)?;
    draw_footer(display, state.display, "SELECT DETAILS  HOLD BOOT BACK")?;
    Ok(())
}

/// Draw timezone, storage-basis and power details without crowding the overview.
pub fn render_clock_details(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let timezone = state.regional.timezone_label_for_rtc(state.board.rtc);
    let rtc_storage = state.regional.rtc_storage_label();
    let rtc_health = if state.board.rtc_clock_integrity_was_lost {
        "Cleared during startup"
    } else {
        "Clear"
    };
    let battery_voltage = state.board.power.map_or_else(
        || "Unavailable".into(),
        |power| {
            power
                .battery_voltage_mv
                .map_or_else(|| "Unavailable".into(), |mv| format!("{mv} mV"))
        },
    );

    draw_header(display, state.display, "RTC DETAILS", "CLOCK CONFIGURATION")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: &state.board.time_label(state.regional),
            middle: "RTC",
            right: "DETAILS",
        },
    )?;

    Text::new("Time basis", Point::new(22, 164), heading).draw(display)?;
    line(display, 210, "Display zone", &timezone, body)?;
    line(display, 250, "RTC storage", &rtc_storage, body)?;
    line(display, 290, "Integrity", rtc_health, body)?;

    Text::new("Power", Point::new(22, 364), heading).draw(display)?;
    line(display, 410, "Battery voltage", &battery_voltage, body)?;
    if let Some(power) = state.board.power {
        line(
            display,
            450,
            "USB VBUS",
            if power.vbus_present {
                "Connected"
            } else {
                "Not detected"
            },
            body,
        )?;
        line(
            display,
            490,
            "Charge state",
            if power.charging {
                "Charging"
            } else {
                "Not charging"
            },
            body,
        )?;
    }

    Text::new("Refresh policy", Point::new(22, 568), heading).draw(display)?;
    line(display, 614, "Live refresh", "30 seconds", body)?;
    line(display, 654, "Idle sleep", "60 seconds", body)?;
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
    Text::new(value, Point::new(196, y), style).draw(display)?;
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
    use super::{render_clock, render_clock_details};
    use crate::{app::AppState, framebuffer::FrameBuffer, orientation::OrientedFrameBuffer};

    #[test]
    fn clock_overview_and_details_render_without_optional_services() {
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        let state = AppState::default();
        render_clock(&mut display, &state).unwrap();
        render_clock_details(&mut display, &state).unwrap();
    }
}
