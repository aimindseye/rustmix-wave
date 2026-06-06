//! SD-backed RTC alarm scheduling and active-alarm actions.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    alarm::ALARMS_CONFIG_PATH,
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

pub fn render_alarms(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let alarms = &state.alarms;
    let current_time = state.board.time_label(state.regional);
    let next = alarms.next_label();

    draw_header(display, state.display, "ALARMS", "RTC SCHEDULES")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: &current_time,
            middle: alarms.home_badge(),
            right: if alarms.hardware_programmed {
                "RTC ARMED"
            } else {
                "RTC IDLE"
            },
        },
    )?;

    if let Some(active) = alarms.active.as_ref() {
        Text::new("Alarm active", Point::new(22, 158), heading).draw(display)?;
        Text::new(&active.label(), Point::new(22, 202), body).draw(display)?;
        Text::new(state.audio.alarm_label(), Point::new(22, 240), body).draw(display)?;
        draw_action(display, 318, "Snooze", alarms.selected == 0, body)?;
        draw_action(display, 386, "Dismiss", alarms.selected == 1, body)?;
        Text::new(
            &format!("Snooze interval: {} minutes", alarms.snooze_minutes),
            Point::new(22, 490),
            body,
        )
        .draw(display)?;
        draw_footer(
            display,
            state.display,
            "UP/DOWN  SELECT RUN  HOLD BOOT BACK",
        )?;
        return Ok(());
    }

    if let Some(editor) = alarms.editor.as_ref() {
        Text::new("Runtime editor", Point::new(22, 154), heading).draw(display)?;
        Text::new(
            &format!("Alarm: {}", editor.draft.name),
            Point::new(22, 194),
            body,
        )
        .draw(display)?;
        draw_editor_row(
            display,
            238,
            "Hour",
            &format!("{:02}", editor.draft.hour),
            editor.field_index == 0,
            body,
        )?;
        draw_editor_row(
            display,
            294,
            "Minute",
            &format!("{:02}", editor.draft.minute),
            editor.field_index == 1,
            body,
        )?;
        draw_editor_row(
            display,
            350,
            "Enabled",
            editor.draft.status_label(),
            editor.field_index == 2,
            body,
        )?;
        draw_editor_row(
            display,
            406,
            "Mode",
            match editor.draft.schedule {
                crate::alarm::AlarmScheduleKind::Recurring { .. } => "RECURRING",
                crate::alarm::AlarmScheduleKind::OneTime { .. } => "ONE TIME",
            },
            editor.field_index == 3,
            body,
        )?;
        draw_editor_row(
            display,
            462,
            "Weekdays / date",
            &editor.draft.schedule.compact_label(),
            editor.field_index == 4,
            body,
        )?;
        draw_action(
            display,
            548,
            "Save runtime edit",
            editor.field_index == 5,
            body,
        )?;
        Text::new(
            "Edit ALARMS.TXT for persistent changes.",
            Point::new(22, 632),
            body,
        )
        .draw(display)?;
        draw_footer(
            display,
            state.display,
            "UP/DOWN CHANGE  SELECT NEXT  BOOT BACK",
        )?;
        return Ok(());
    }

    Text::new("Configured schedules", Point::new(22, 154), heading).draw(display)?;
    Text::new(&format!("Next: {next}"), Point::new(22, 194), body).draw(display)?;
    Text::new(
        &format!("Config: {ALARMS_CONFIG_PATH}"),
        Point::new(22, 230),
        body,
    )
    .draw(display)?;

    if alarms.alarms.is_empty() {
        Text::new("No alarm schedules were loaded.", Point::new(22, 298), body).draw(display)?;
        Text::new("Add alarm rows to ALARMS.TXT.", Point::new(22, 338), body).draw(display)?;
    } else {
        for (index, alarm) in alarms.alarms.iter().take(6).enumerate() {
            draw_alarm_row(
                display,
                264 + index as i32 * 62,
                alarm,
                alarms.selected == index,
                body,
            )?;
        }
    }

    if let Some(error) = alarms.error.as_deref() {
        Text::new(&format!("Last error: {error}"), Point::new(22, 674), body).draw(display)?;
    }
    draw_footer(
        display,
        state.display,
        "UP/DOWN  SELECT EDIT  HOLD BOOT BACK",
    )?;
    Ok(())
}

fn draw_alarm_row(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    alarm: &crate::alarm::AlarmDefinition,
    selected: bool,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    let border = if selected {
        PrimitiveStyle::with_stroke(BinaryColor::On, 4)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    Rectangle::new(Point::new(22, top), Size::new(436, 52))
        .into_styled(border)
        .draw(display)?;
    Text::new(
        if selected { ">" } else { " " },
        Point::new(36, top + 34),
        style,
    )
    .draw(display)?;
    Text::new(&alarm.name, Point::new(58, top + 34), style).draw(display)?;
    Text::new(&alarm.time_label(), Point::new(206, top + 34), style).draw(display)?;
    Text::new(
        &alarm.schedule.compact_label(),
        Point::new(274, top + 34),
        style,
    )
    .draw(display)?;
    Text::new(alarm.status_label(), Point::new(420, top + 34), style).draw(display)?;
    Ok(())
}

fn draw_editor_row(
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
    Rectangle::new(Point::new(22, top), Size::new(436, 46))
        .into_styled(border)
        .draw(display)?;
    Text::new(
        if selected { ">" } else { " " },
        Point::new(38, top + 30),
        style,
    )
    .draw(display)?;
    Text::new(label, Point::new(68, top + 30), style).draw(display)?;
    Text::new(value, Point::new(248, top + 30), style).draw(display)?;
    Ok(())
}

fn draw_action(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    label: &str,
    selected: bool,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    let border = if selected {
        PrimitiveStyle::with_stroke(BinaryColor::On, 4)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    Rectangle::new(Point::new(22, top), Size::new(436, 52))
        .into_styled(border)
        .draw(display)?;
    Text::new(
        if selected { ">" } else { " " },
        Point::new(38, top + 34),
        style,
    )
    .draw(display)?;
    Text::new(label, Point::new(68, top + 34), style).draw(display)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::render_alarms;
    use crate::{app::AppState, framebuffer::FrameBuffer, orientation::OrientedFrameBuffer};

    #[test]
    fn alarms_screen_renders_without_loaded_configuration() {
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        render_alarms(&mut display, &AppState::default()).unwrap();
    }
}
