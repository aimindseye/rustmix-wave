//! Product-facing Home dashboard for the RustMix Wave shell.

use crate::{
    app::{
        menu::home_entries,
        state::AppState,
        widgets::{
            card::{draw_card, CardSpec},
            home_dashboard::{
                draw_home_dashboard_strip, draw_home_footer, draw_home_header, HomeDashboardStrip,
            },
        },
    },
    orientation::OrientedFrameBuffer,
    rtc::RtcDateTime,
};
use core::convert::Infallible;

pub fn render_home(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let date = home_date_label(state);
    let time = state.board.time_label(state.regional);
    let weather = state.weather.current_summary();
    let battery = home_battery_label(state);
    let wifi = format!("Wi-Fi {}", state.network.wifi_state.label());

    draw_home_header(display, state.display)?;
    draw_home_dashboard_strip(
        display,
        state.display,
        HomeDashboardStrip {
            date: &date,
            time: &time,
            weather: &weather,
            battery: &battery,
            wifi: &wifi,
        },
    )?;

    for (index, entry) in home_entries().iter().copied().enumerate() {
        draw_card(
            display,
            state.display,
            CardSpec {
                top: 204 + index as i32 * 102,
                title: entry.label,
                subtitle: entry.subtitle,
                badge: entry.badge,
                selected: state.home_selected == index,
            },
        )?;
    }

    draw_home_footer(display, state.display)?;
    Ok(())
}

fn home_date_label(state: &AppState) -> String {
    state.board.rtc.map_or_else(
        || "Date unavailable".into(),
        |rtc| compact_local_date(state.regional.localize_rtc(rtc)),
    )
}

fn compact_local_date(local: RtcDateTime) -> String {
    const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let weekday = WEEKDAYS
        .get(usize::from(local.weekday))
        .copied()
        .unwrap_or("---");
    let month = local
        .month
        .checked_sub(1)
        .and_then(|index| MONTHS.get(usize::from(index)))
        .copied()
        .unwrap_or("---");
    format!("{weekday}, {month} {}", local.day)
}

fn home_battery_label(state: &AppState) -> String {
    state
        .board
        .power
        .and_then(|snapshot| snapshot.battery_percent)
        .map_or_else(
            || "Battery --".into(),
            |percent| format!("Battery {percent}%"),
        )
}

#[cfg(test)]
mod tests {
    use super::compact_local_date;
    use crate::rtc::RtcDateTime;

    #[test]
    fn renders_compact_dashboard_date() {
        assert_eq!(
            compact_local_date(RtcDateTime {
                year: 2026,
                month: 6,
                day: 4,
                weekday: 4,
                hour: 8,
                minute: 13,
                second: 0,
            }),
            "Thu, Jun 4"
        );
    }
}
