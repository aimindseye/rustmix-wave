//! Native RTC-localized Calendar with X4-compatible U.S. and personal events.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
};

use crate::{
    app::{
        state::AppState,
        typography::{Text, UiTextRole, UiTextStyle},
        widgets::{
            footer::draw_footer,
            header::draw_header,
            status_row::{draw_status_row, StatusRow},
        },
    },
    calendar::{
        compact_text, days_in_month, weekday, CalendarDate, CalendarEvent, CalendarEventKind,
        CALENDAR_EDITOR_KEY_ROWS,
    },
    orientation::OrientedFrameBuffer,
};

const GRID_LEFT: i32 = 26;
const GRID_TOP: i32 = 264;
const CELL_WIDTH: i32 = 61;
const CELL_HEIGHT: i32 = 48;
const AGENDA_SUMMARY_TOP: i32 = 190;
const AGENDA_SUMMARY_HEIGHT: u32 = 94;
const AGENDA_STATUS_BASELINE: i32 = 218;
const AGENDA_NOTICE_BASELINE: i32 = 242;
const AGENDA_RANGE_BASELINE: i32 = 266;
const AGENDA_FIRST_ROW_TOP: i32 = 300;
const AGENDA_ROW_STEP: i32 = 60;
const AGENDA_ROW_HEIGHT: u32 = 54;
const AGENDA_FOOTER_HINT: &str = "MOVE  SELECT OPEN  BOOT ADD  HOLD BACK";
const CALENDAR_EDITOR_FOOTER_HINT: &str = "MOVE  BOOT H/V  SELECT KEY  HOLD BACK";
const WEEKDAY_LABELS: [&str; 7] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
const MONTH_LABELS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Render the RTC-localized monthly Calendar page with compact event markers.
pub fn render_calendar(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let cursor = state.calendar.cursor;
    let month = month_label(cursor.month);
    let month_year = format!("{month} {}", cursor.year);
    let selected = selected_date_label(cursor);
    let today = state
        .board
        .rtc
        .map(|rtc| CalendarDate::from_rtc(state.regional.localize_rtc(rtc)));

    draw_header(
        display,
        state.display,
        "CALENDAR",
        "US EVENTS + DAILY AGENDA",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: state.calendar.mode.label(),
            middle: &month_year,
            right: "LOCAL RTC",
        },
    )?;

    Text::new(
        &month_year,
        Point::new(24, 172),
        state.display.heading_style(),
    )
    .draw(display)?;
    Text::new(
        "SELECT changes DAY / MONTH navigation.",
        Point::new(24, 204),
        state.display.body_style(),
    )
    .draw(display)?;
    Text::new(
        "BOOT short opens selected-day agenda.",
        Point::new(24, 230),
        state.display.detail_style(),
    )
    .draw(display)?;

    for (column, label) in WEEKDAY_LABELS.iter().enumerate() {
        Text::new(
            label,
            Point::new(GRID_LEFT + column as i32 * CELL_WIDTH + 8, 252),
            state.display.detail_style(),
        )
        .draw(display)?;
    }

    draw_month_grid(display, state, cursor, today)?;

    Rectangle::new(Point::new(22, 584), Size::new(436, 112))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;
    Text::new(
        "Selected day",
        Point::new(40, 614),
        state.display.body_style(),
    )
    .draw(display)?;
    Text::new(
        &selected,
        Point::new(40, 648),
        state.display.heading_style(),
    )
    .draw(display)?;
    Text::new(
        &state.calendar.selected_day_summary(),
        Point::new(40, 680),
        state.display.body_style(),
    )
    .draw(display)?;
    Text::new(
        &state.calendar.catalog.status_label(),
        Point::new(24, 716),
        state.display.detail_style(),
    )
    .draw(display)?;

    draw_footer(
        display,
        state.display,
        "UP/DOWN MOVE  SELECT MODE  BOOT AGENDA  HOLD BOOT BACK",
    )?;
    Ok(())
}

/// Render the selected-day scrollable agenda.
pub fn render_calendar_agenda(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let date = selected_date_label(state.calendar.cursor);
    let events = state.calendar.selected_day_events();
    let count = agenda_event_count_label(events.len());
    draw_header(display, state.display, "CALENDAR", "DAILY AGENDA")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "AGENDA",
            middle: &count,
            right: "US + PERS",
        },
    )?;
    Text::new(&date, Point::new(22, 172), state.display.heading_style()).draw(display)?;
    Rectangle::new(
        Point::new(22, AGENDA_SUMMARY_TOP),
        Size::new(436, AGENDA_SUMMARY_HEIGHT),
    )
    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
    .draw(display)?;
    Text::new(
        &compact_text(&state.calendar.catalog.status_label(), 54),
        Point::new(34, AGENDA_STATUS_BASELINE),
        state.display.detail_style(),
    )
    .draw(display)?;
    Text::new(
        &compact_text(&state.calendar.notice, 54),
        Point::new(34, AGENDA_NOTICE_BASELINE),
        state.display.detail_style(),
    )
    .draw(display)?;
    let visible = state.calendar.agenda_visible_range();
    Text::new(
        &agenda_visible_range_label(events.len(), &visible),
        Point::new(34, AGENDA_RANGE_BASELINE),
        state.display.detail_style(),
    )
    .draw(display)?;

    if events.is_empty() {
        Rectangle::new(Point::new(22, AGENDA_FIRST_ROW_TOP), Size::new(436, 118))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display)?;
        Text::new(
            "No events for selected day.",
            Point::new(44, AGENDA_FIRST_ROW_TOP + 58),
            state.display.body_style(),
        )
        .draw(display)?;
    } else {
        for (visible_index, event_index) in visible.enumerate() {
            let event = events[event_index];
            let selected = event_index == state.calendar.agenda_selected;
            draw_agenda_row(
                display,
                state,
                AGENDA_FIRST_ROW_TOP + visible_index as i32 * AGENDA_ROW_STEP,
                event,
                selected,
            )?;
        }
    }

    draw_footer(display, state.display, AGENDA_FOOTER_HINT)?;
    Ok(())
}

/// Render one personal or read-only U.S. calendar event.
pub fn render_calendar_event_details(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let Some(event) = state.calendar.selected_agenda_event() else {
        draw_header(display, state.display, "CALENDAR EVENT", "SAFE DETAILS")?;
        draw_status_row(
            display,
            state.display,
            StatusRow {
                left: "AGENDA",
                middle: "NO EVENT",
                right: "SAFE",
            },
        )?;
        Text::new(
            "No event is selected.",
            Point::new(22, 230),
            state.display.body_style(),
        )
        .draw(display)?;
        draw_footer(display, state.display, "HOLD BOOT BACK")?;
        return Ok(());
    };

    let personal = event.kind == CalendarEventKind::Personal;
    draw_header(
        display,
        state.display,
        "CALENDAR EVENT",
        if personal {
            "PERSONAL EVENT"
        } else {
            "READ-ONLY US HOLIDAY"
        },
    )?;
    let date = selected_date_label(event.date);
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: event.kind.label(),
            middle: &date,
            right: if personal { "EDITABLE" } else { "READ ONLY" },
        },
    )?;
    Text::new(
        &compact_text(&event.title, 34),
        Point::new(22, 176),
        state.display.heading_style(),
    )
    .draw(display)?;
    detail_line(
        display,
        230,
        "Source",
        event.kind.source_file(),
        state.display.body_style(),
    )?;
    detail_line(
        display,
        274,
        "Category",
        event.kind.label(),
        state.display.body_style(),
    )?;
    detail_line(display, 318, "Date", &date, state.display.body_style())?;
    Rectangle::new(Point::new(22, 350), Size::new(436, 148))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;
    Text::new("Detail", Point::new(40, 386), state.display.body_style()).draw(display)?;
    draw_wrapped_detail(display, state, &event.detail, 40, 424)?;

    if personal {
        for (index, label) in [
            "Edit personal event",
            "Delete personal event",
            "Return to agenda",
        ]
        .iter()
        .enumerate()
        {
            draw_action_row(
                display,
                state,
                526 + index as i32 * 54,
                label,
                index == state.calendar.details_action_selected,
            )?;
        }
        Text::new(
            "Only personal EVENTS.TXT rows can change.",
            Point::new(22, 714),
            state.display.detail_style(),
        )
        .draw(display)?;
        draw_footer(
            display,
            state.display,
            "UP/DOWN MOVE  SELECT ACTION  HOLD BOOT BACK",
        )?;
    } else {
        Text::new(
            "U.S. pack entries remain read-only.",
            Point::new(22, 572),
            state.display.body_style(),
        )
        .draw(display)?;
        draw_footer(display, state.display, "HOLD BOOT BACK")?;
    }
    Ok(())
}

/// Render create/edit personal-event keyboard screen.
pub fn render_calendar_event_editor(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let Some(editor) = state.calendar.editor.as_ref() else {
        draw_header(display, state.display, "CALENDAR EDITOR", "SAFE FALLBACK")?;
        Text::new(
            "No personal event editor is active.",
            Point::new(22, 220),
            state.display.body_style(),
        )
        .draw(display)?;
        draw_footer(display, state.display, "HOLD BOOT BACK")?;
        return Ok(());
    };
    let date = calendar_editor_status_date_label(editor.date);
    draw_header(
        display,
        state.display,
        "CALENDAR EDITOR",
        editor.mode.label(),
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: editor.active_field.label(),
            middle: &date,
            right: editor.navigation_mode_label(),
        },
    )?;
    Text::new("Title", Point::new(22, 170), state.display.body_style()).draw(display)?;
    Rectangle::new(Point::new(22, 184), Size::new(436, 46))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;
    Text::new(
        &truncate_or_placeholder(&editor.title, "_", 44),
        Point::new(34, 214),
        state.display.body_style(),
    )
    .draw(display)?;
    Text::new("Detail", Point::new(22, 260), state.display.body_style()).draw(display)?;
    Rectangle::new(Point::new(22, 274), Size::new(436, 58))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;
    Text::new(
        &truncate_or_placeholder(&editor.detail, "_", 50),
        Point::new(34, 310),
        state.display.detail_style(),
    )
    .draw(display)?;
    Text::new(
        &compact_text(&editor.message, 56),
        Point::new(22, 360),
        state.display.detail_style(),
    )
    .draw(display)?;
    draw_editor_keyboard(display, state)?;
    draw_footer(display, state.display, CALENDAR_EDITOR_FOOTER_HINT)?;
    Ok(())
}

/// Render explicit delete confirmation for one personal row.
pub fn render_calendar_delete_confirmation(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(
        display,
        state.display,
        "DELETE CALENDAR EVENT?",
        "PERSONAL EVENTS.TXT ROW",
    )?;
    let title = state
        .calendar
        .selected_agenda_event()
        .map_or("No event selected", |event| event.title.as_str());
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "CONFIRM",
            middle: "PERSONAL ONLY",
            right: "ATOMIC WRITE",
        },
    )?;
    Text::new(
        &compact_text(title, 36),
        Point::new(22, 206),
        state.display.heading_style(),
    )
    .draw(display)?;
    Text::new(
        "The U.S. holiday pack is never modified.",
        Point::new(22, 254),
        state.display.body_style(),
    )
    .draw(display)?;
    for (index, label) in ["Cancel", "Delete permanently"].iter().enumerate() {
        draw_action_row(
            display,
            state,
            332 + index as i32 * 64,
            label,
            index == state.calendar.delete_confirmation_selected,
        )?;
    }
    draw_footer(
        display,
        state.display,
        "UP/DOWN MOVE  SELECT  HOLD BOOT BACK",
    )?;
    Ok(())
}

fn draw_month_grid(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    cursor: CalendarDate,
    today: Option<CalendarDate>,
) -> Result<(), Infallible> {
    let first_weekday = usize::from(weekday(cursor.year, cursor.month, 1));
    let month_days = days_in_month(cursor.year, cursor.month);

    for day in 1..=month_days {
        let index = first_weekday + usize::from(day - 1);
        let column = (index % 7) as i32;
        let row = (index / 7) as i32;
        let left = GRID_LEFT + column * CELL_WIDTH;
        let top = GRID_TOP + row * CELL_HEIGHT;
        let cell_date = CalendarDate {
            year: cursor.year,
            month: cursor.month,
            day,
        };
        let is_selected = cell_date == cursor;
        let is_today = today.is_some_and(|value| value == cell_date);
        let has_event = state.calendar.event_count_for_date(cell_date) > 0;
        let border = PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(if is_selected { 3 } else { 1 })
            .fill_color(if is_selected {
                BinaryColor::On
            } else {
                BinaryColor::Off
            })
            .build();
        let ink = if is_selected {
            BinaryColor::Off
        } else {
            BinaryColor::On
        };

        Rectangle::new(Point::new(left, top), Size::new(54, 40))
            .into_styled(border)
            .draw(display)?;
        Text::new(
            &format!("{day:>2}"),
            Point::new(left + 14, top + 28),
            state.display.text_style(UiTextRole::Body, ink),
        )
        .draw(display)?;
        if is_today {
            Rectangle::new(Point::new(left + 43, top + 6), Size::new(5, 5))
                .into_styled(PrimitiveStyle::with_fill(ink))
                .draw(display)?;
        }
        if has_event {
            Rectangle::new(Point::new(left + 6, top + 31), Size::new(10, 3))
                .into_styled(PrimitiveStyle::with_fill(ink))
                .draw(display)?;
        }
    }
    Ok(())
}

fn draw_agenda_row(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    top: i32,
    event: &CalendarEvent,
    selected: bool,
) -> Result<(), Infallible> {
    let box_style = if selected {
        PrimitiveStyle::with_fill(BinaryColor::On)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    let ink = if selected {
        BinaryColor::Off
    } else {
        BinaryColor::On
    };
    Rectangle::new(Point::new(22, top), Size::new(436, AGENDA_ROW_HEIGHT))
        .into_styled(box_style)
        .draw(display)?;
    Text::new(
        event.kind.label(),
        Point::new(36, top + 22),
        state.display.text_style(UiTextRole::Detail, ink),
    )
    .draw(display)?;
    Text::new(
        &compact_text(&event.title, 31),
        Point::new(112, top + 32),
        state.display.text_style(UiTextRole::Body, ink),
    )
    .draw(display)?;
    Ok(())
}

fn agenda_event_count_label(event_count: usize) -> String {
    if event_count == 1 {
        "1 EVENT".into()
    } else {
        format!("{event_count} EVENTS")
    }
}

fn agenda_visible_range_label(event_count: usize, visible: &core::ops::Range<usize>) -> String {
    if event_count == 0 {
        "Showing 0 of 0".into()
    } else {
        format!(
            "Showing {}-{} of {event_count}",
            visible.start + 1,
            visible.end
        )
    }
}

fn detail_line(
    display: &mut OrientedFrameBuffer<'_>,
    y: i32,
    label: &str,
    value: &str,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    Text::new(label, Point::new(22, y), style).draw(display)?;
    Text::new(&compact_text(value, 37), Point::new(154, y), style).draw(display)?;
    Ok(())
}

fn draw_wrapped_detail(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    detail: &str,
    left: i32,
    first_baseline: i32,
) -> Result<(), Infallible> {
    let words = detail.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        Text::new(
            "No additional detail.",
            Point::new(left, first_baseline),
            state.display.body_style(),
        )
        .draw(display)?;
        return Ok(());
    }
    let mut line = String::new();
    let mut lines = Vec::new();
    for word in words {
        let proposed = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if proposed.chars().count() > 36 && !line.is_empty() {
            lines.push(line);
            line = word.to_string();
        } else {
            line = proposed;
        }
        if lines.len() == 3 {
            break;
        }
    }
    if !line.is_empty() && lines.len() < 3 {
        lines.push(line);
    }
    for (index, line) in lines.iter().enumerate() {
        Text::new(
            &compact_text(line, 36),
            Point::new(left, first_baseline + index as i32 * 34),
            state.display.body_style(),
        )
        .draw(display)?;
    }
    Ok(())
}

fn draw_action_row(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    top: i32,
    label: &str,
    selected: bool,
) -> Result<(), Infallible> {
    let style = if selected {
        PrimitiveStyle::with_fill(BinaryColor::On)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    let ink = if selected {
        BinaryColor::Off
    } else {
        BinaryColor::On
    };
    Rectangle::new(Point::new(22, top), Size::new(436, 42))
        .into_styled(style)
        .draw(display)?;
    Text::new(
        label,
        Point::new(40, top + 29),
        state.display.text_style(UiTextRole::Body, ink),
    )
    .draw(display)?;
    Ok(())
}

fn draw_editor_keyboard(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let editor = state
        .calendar
        .editor
        .as_ref()
        .expect("editor checked before keyboard");
    for (row_index, row) in CALENDAR_EDITOR_KEY_ROWS.iter().enumerate() {
        for (column_index, label) in row.iter().enumerate() {
            let index = row_index * 7 + column_index;
            let left = 22 + column_index as i32 * 62;
            let top = 390 + row_index as i32 * 54;
            let selected = editor.selected_key_index() == index;
            Rectangle::new(Point::new(left, top), Size::new(58, 46))
                .into_styled(PrimitiveStyle::with_stroke(
                    BinaryColor::On,
                    if selected { 3 } else { 1 },
                ))
                .draw(display)?;
            Text::new(
                label,
                Point::new(left + if label.len() > 3 { 4 } else { 15 }, top + 29),
                state.display.detail_style(),
            )
            .draw(display)?;
        }
    }
    Ok(())
}

fn truncate_or_placeholder(value: &str, placeholder: &str, max_chars: usize) -> String {
    if value.is_empty() {
        placeholder.into()
    } else {
        compact_text(value, max_chars)
    }
}

fn month_label(month: u8) -> &'static str {
    month
        .checked_sub(1)
        .and_then(|index| MONTH_LABELS.get(usize::from(index)))
        .copied()
        .unwrap_or("Unknown")
}

fn calendar_editor_status_date_label(date: CalendarDate) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

fn selected_date_label(date: CalendarDate) -> String {
    const WEEKDAYS: [&str; 7] = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
    let weekday = WEEKDAYS
        .get(usize::from(date.weekday()))
        .copied()
        .unwrap_or("Unknown");
    format!(
        "{weekday}, {} {}, {}",
        month_label(date.month),
        date.day,
        date.year
    )
}

#[cfg(test)]
mod tests {
    use super::{
        agenda_event_count_label, agenda_visible_range_label, calendar_editor_status_date_label,
        month_label, selected_date_label, AGENDA_FIRST_ROW_TOP, AGENDA_FOOTER_HINT,
        AGENDA_RANGE_BASELINE, AGENDA_ROW_HEIGHT, AGENDA_ROW_STEP, CALENDAR_EDITOR_FOOTER_HINT,
    };
    use crate::{
        app::AppState,
        calendar::{CalendarDate, CALENDAR_AGENDA_VISIBLE_ROWS},
        framebuffer::FrameBuffer,
        orientation::OrientedFrameBuffer,
    };

    #[test]
    fn renders_readable_selected_date() {
        let date = CalendarDate::new(2026, 6, 4).unwrap();
        assert_eq!(month_label(6), "June");
        assert_eq!(selected_date_label(date), "Thursday, June 4, 2026");
    }

    #[test]
    fn editor_and_delete_confirmation_render_without_sd_card() {
        let mut state = AppState::default();
        state.calendar.begin_create_personal();
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        super::render_calendar_event_editor(&mut display, &state).unwrap();
        super::render_calendar_delete_confirmation(&mut display, &state).unwrap();
    }

    #[test]
    fn editor_status_date_and_footer_fit_the_shared_status_strip() {
        let date = CalendarDate::new(2026, 6, 6).unwrap();
        assert_eq!(calendar_editor_status_date_label(date), "2026-06-06");
        assert!(calendar_editor_status_date_label(date).chars().count() <= 10);
        assert_eq!(
            CALENDAR_EDITOR_FOOTER_HINT,
            "MOVE  BOOT H/V  SELECT KEY  HOLD BACK"
        );
        assert!(CALENDAR_EDITOR_FOOTER_HINT.chars().count() <= 40);
    }

    #[test]
    fn agenda_labels_use_singular_plural_and_safe_vertical_bounds() {
        assert_eq!(agenda_event_count_label(0), "0 EVENTS");
        assert_eq!(agenda_event_count_label(1), "1 EVENT");
        assert_eq!(agenda_event_count_label(2), "2 EVENTS");
        assert_eq!(agenda_visible_range_label(0, &(0..0)), "Showing 0 of 0");
        assert_eq!(agenda_visible_range_label(1, &(0..1)), "Showing 1-1 of 1");
        assert!(AGENDA_RANGE_BASELINE < AGENDA_FIRST_ROW_TOP);
        let last_row_bottom = AGENDA_FIRST_ROW_TOP
            + (CALENDAR_AGENDA_VISIBLE_ROWS as i32 - 1) * AGENDA_ROW_STEP
            + AGENDA_ROW_HEIGHT as i32;
        assert!(last_row_bottom < 746);
    }

    #[test]
    fn agenda_footer_hint_is_compact_for_the_e_paper_width() {
        assert_eq!(AGENDA_FOOTER_HINT, "MOVE  SELECT OPEN  BOOT ADD  HOLD BACK");
        assert!(AGENDA_FOOTER_HINT.chars().count() <= 40);
    }
}
